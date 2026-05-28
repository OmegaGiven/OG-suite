import { chromium } from '@playwright/test'
import { createStressBrowserLaunchOptions } from './lib/stress-browser.mjs'
const appUrl = process.env.OG_STRESS_APP_URL ?? 'http://localhost:5173/?stress=create-delete'
const apiUrl = process.env.OG_STRESS_API_URL ?? 'http://127.0.0.1:8080'
const cycles = Number.parseInt(process.env.OG_STRESS_CREATE_DELETE_CYCLES ?? '3', 10)
const session = await getStressSession()
const browser = await chromium.launch(createStressBrowserLaunchOptions())
const createdTitles = []

try {
  const desktop = await browser.newContext({ viewport: { width: 1280, height: 860 } })
  const mobile = await browser.newContext({
    viewport: { width: 390, height: 844 },
    isMobile: true,
    hasTouch: true,
    userAgent:
      'Mozilla/5.0 (iPhone; CPU iPhone OS 18_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.0 Mobile/15E148 Safari/604.1',
  })
  await Promise.all([prepareContext(desktop), prepareContext(mobile)])

  const desktopPage = await desktop.newPage()
  const mobilePage = await mobile.newPage()
  await Promise.all([openNotesApp(desktopPage), openNotesApp(mobilePage)])

  const results = []
  for (let index = 0; index < cycles; index += 1) {
    const desktopTitle = `Create Delete Desktop ${Date.now()} ${index}`
    await createNoteFromClient(desktopPage, desktopTitle)
    createdTitles.push(desktopTitle)
    await waitForNoteOnServer(desktopTitle)
    await selectNote(mobilePage, desktopTitle, true)
    await deleteSelectedNote(mobilePage, true)
    await waitForNoteDeletedOnServer(desktopTitle)
    results.push({ direction: 'desktop-to-mobile-delete', title: desktopTitle })

    const mobileTitle = `Create Delete Mobile ${Date.now()} ${index}`
    await createNoteFromClient(mobilePage, mobileTitle, true)
    createdTitles.push(mobileTitle)
    await waitForNoteOnServer(mobileTitle)
    await selectNote(desktopPage, mobileTitle)
    await deleteSelectedNote(desktopPage)
    await waitForNoteDeletedOnServer(mobileTitle)
    results.push({ direction: 'mobile-to-desktop-delete', title: mobileTitle })
  }

  console.log(JSON.stringify({ cycles, operations: results.length, results }, null, 2))
  await desktop.close()
  await mobile.close()
} finally {
  await browser.close()
}

async function prepareContext(context) {
  await context.addInitScript(
    ({ apiUrl, authSession }) => {
      localStorage.setItem('og-suite:server-url', apiUrl)
      localStorage.setItem('og-suite:auth:access-token', authSession.accessToken)
      localStorage.setItem('og-suite:auth:refresh-token', authSession.refreshToken)
      localStorage.setItem('og-suite:auth:expires-at', authSession.expiresAt)
      localStorage.setItem('og-suite:notes:editor-render-mode', 'text')
      localStorage.removeItem('og-suite:notes:local-only')
    },
    { apiUrl, authSession: session },
  )
}

async function openNotesApp(page) {
  await page.goto(appUrl, { waitUntil: 'networkidle' })
  await page
    .getByRole('button', { name: 'Notes', exact: true })
    .click({ timeout: 5000 })
    .catch(async () => {
      await page.evaluate(() => {
        const notesButton = Array.from(document.querySelectorAll('button')).find((button) => button.textContent?.trim() === 'Notes')
        notesButton?.dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true }))
      })
    })
  await page.locator('main.notes-app').waitFor({ timeout: 5000 })
}

async function createNoteFromClient(page, title, mobileLayout = false) {
  await closeFloatingOverlays(page)
  if (mobileLayout) await openMobileFiles(page)
  const clicked = await page.evaluate(() => {
    const button = Array.from(document.querySelectorAll('button')).find((item) => item.getAttribute('aria-label') === 'New note')
    if (!(button instanceof HTMLButtonElement) || button.disabled) return false
    button.click()
    return true
  })
  if (!clicked) await page.getByRole('button', { name: 'New note' }).click()
  const titleInput = page.locator('input[aria-label="Title"]')
  await titleInput.waitFor({ timeout: 5000 })
  await titleInput.fill(title)
  await page.getByRole('button', { name: 'Save' }).click().catch(async () => {
    await titleInput.press(process.platform === 'darwin' ? 'Meta+S' : 'Control+S')
  })
  await page.waitForTimeout(700)
}

async function selectNote(page, title, mobileLayout = false) {
  await closeFloatingOverlays(page)
  const deadline = Date.now() + 10000
  while (Date.now() < deadline) {
    if (mobileLayout) {
      await openMobileFiles(page)
      const clicked = await page.evaluate((title) => {
        const rows = Array.from(document.querySelectorAll('.notes-list.mobile-open .note-row, .notes-list .note-row'))
        const row = rows.find((item) => item.textContent?.includes(title))
        if (!(row instanceof HTMLElement)) return false
        row.click()
        return true
      }, title)
      if (clicked) {
        await page.locator('input[aria-label="Title"]').waitFor({ timeout: 5000 })
        if ((await page.locator('input[aria-label="Title"]').inputValue()) === title) return
      }
    } else {
      const row = page.locator('.notes-list button.note-row').filter({ hasText: title }).first()
      if ((await row.count()) > 0) {
        await row.click()
        await page.locator('input[aria-label="Title"]').waitFor({ timeout: 5000 })
        if ((await page.locator('input[aria-label="Title"]').inputValue()) === title) return
      }
    }
    await page.reload({ waitUntil: 'networkidle' })
    await openNotesApp(page)
    await page.waitForTimeout(400)
  }
  throw new Error(`Could not select synced note "${title}"`)
}

async function deleteSelectedNote(page, mobileLayout = false) {
  await closeFloatingOverlays(page)
  if (mobileLayout) await openMobileFiles(page)
  page.once('dialog', async (dialog) => dialog.accept())
  const clicked = await page.evaluate(() => {
    const button = Array.from(document.querySelectorAll('button')).find((item) => item.getAttribute('aria-label') === 'Delete selected note')
    if (!(button instanceof HTMLButtonElement) || button.disabled) return false
    button.click()
    return true
  })
  if (!clicked) await page.getByRole('button', { name: 'Delete selected note' }).click()
  await page.waitForTimeout(700)
}

async function closeFloatingOverlays(page) {
  await page.evaluate(() => {
    const closeButtons = Array.from(document.querySelectorAll('button')).filter((item) => {
      const label = item.getAttribute('aria-label')
      return label === 'Close note status' || label === 'Close connected servers'
    })
    for (const button of closeButtons) button.click()
  })
  await page.waitForTimeout(100)
}

async function openMobileFiles(page) {
  const opened = await page.evaluate(() => {
    const button = Array.from(document.querySelectorAll('button')).find((item) => item.getAttribute('aria-label') === 'Open files')
    if (!button) return false
    button.click()
    return true
  })
  if (!opened) return
  await page.locator('.notes-list.mobile-open').waitFor({ state: 'visible', timeout: 5000 }).catch(() => {})
}

async function waitForNoteOnServer(title) {
  const note = await waitForServerNote(title, (item) => !item.deletedAt)
  return note
}

async function waitForNoteDeletedOnServer(title) {
  await waitForServerNote(title, (item) => Boolean(item.deletedAt), true)
}

async function waitForServerNote(title, predicate, allowMissing = false) {
  const deadline = Date.now() + 12000
  let latest = []
  while (Date.now() < deadline) {
    latest = await listServerNotes()
    const matches = latest.filter((item) => item.title === title)
    const matched = matches.find(predicate)
    if (matched) return matched
    if (allowMissing && matches.length === 0) return null
    await new Promise((resolve) => setTimeout(resolve, 300))
  }
  throw new Error(
    JSON.stringify(
      {
        message: `Timed out waiting for server note state: ${title}`,
        knownStressNotes: latest.filter((item) => createdTitles.includes(item.title)).map((item) => ({
          title: item.title,
          deletedAt: item.deletedAt ?? null,
        })),
      },
      null,
      2,
    ),
  )
}

async function listServerNotes() {
  const response = await fetch(`${apiUrl}/api/v1/notes`)
  if (!response.ok) throw new Error(`Failed to list server notes: ${response.status} ${await response.text()}`)
  return response.json()
}

async function getStressSession() {
  const username = 'stress-admin'
  const password = 'stress-password-123'
  const directLogin = await login(username, password)
  if (directLogin) return directLogin

  const adminLogin = await login('admin', 'password')
  if (adminLogin?.user?.mustChangePassword) {
    const setup = await fetch(`${apiUrl}/api/v1/auth/complete-setup`, {
      method: 'POST',
      headers: {
        'content-type': 'application/json',
        authorization: `Bearer ${adminLogin.accessToken}`,
      },
      body: JSON.stringify({
        username,
        displayName: 'Stress Admin',
        password,
        confirmPassword: password,
      }),
    })
    if (setup.ok) {
      const session = await login(username, password)
      if (session) return session
    }
  }

  const registered = await fetch(`${apiUrl}/api/v1/auth/register`, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({ username, displayName: 'Stress Admin', password }),
  })
  if (registered.ok) return registered.json()

  const fallbackLogin = await login(username, password)
  if (fallbackLogin) return fallbackLogin
  throw new Error(`Could not create stress auth session: ${registered.status} ${await registered.text()}`)
}

async function login(username, password) {
  const response = await fetch(`${apiUrl}/api/v1/auth/login`, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({ username, password }),
  })
  if (!response.ok) return null
  return response.json()
}
