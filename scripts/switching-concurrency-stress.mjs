import { chromium } from '@playwright/test'
import { createStressBrowserLaunchOptions } from './lib/stress-browser.mjs'

const appUrl = process.env.OG_STRESS_APP_URL ?? 'http://localhost:5173/?stress=switching'
const apiUrl = process.env.OG_STRESS_API_URL ?? 'http://127.0.0.1:8080'
const titleA = `Switch Stress A ${Date.now()}`
const titleB = `Switch Stress B ${Date.now()}`
const session = await getStressSession()

const noteB = await createStressNote(titleB, 'Note B seed.\n')
const noteA = await createStressNote(titleA, 'Note A seed.\n')
const browser = await chromium.launch(createStressBrowserLaunchOptions())

try {
  const desktop = await browser.newContext({ viewport: { width: 1280, height: 860 } })
  const mobile = await browser.newContext({
    viewport: { width: 390, height: 844 },
    isMobile: true,
    hasTouch: true,
    userAgent:
      'Mozilla/5.0 (Linux; Android 14; Pixel 8) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/125 Mobile Safari/537.36',
  })
  await Promise.all([prepareContext(desktop), prepareContext(mobile)])

  const desktopPage = await desktop.newPage()
  const mobilePage = await mobile.newPage()
const desktopEditor = await openTextNote(desktopPage, titleA, false, 'Note A seed.')
const mobileEditor = await openTextNote(mobilePage, titleA, true, 'Note A seed.')

await Promise.all([
  appendText(desktopPage, desktopEditor, 'desktop edit A before tab switch\n'),
  appendText(mobilePage, mobileEditor, 'mobile edit A before note switch\n'),
])
const afterFirstA = await Promise.all([desktopEditor.inputValue(), mobileEditor.inputValue()])

  const desktopFeedTab = await desktop.newPage()
  await desktopFeedTab.goto(appUrl, { waitUntil: 'networkidle' })
  await desktopFeedTab.getByRole('button', { name: 'Feed', exact: true }).click({ timeout: 5000 }).catch(() => {})
  await desktopFeedTab.waitForTimeout(250)

  await selectNote(mobilePage, titleB, true, 'Note B seed.')
  await appendText(mobilePage, mobileEditor, 'mobile edit B after note switch\n')
  await selectNote(desktopPage, titleB, false, 'Note B seed.')
  await appendText(desktopPage, desktopEditor, 'desktop edit B after tab switch\n')

  await desktopFeedTab.close()
  await selectNote(desktopPage, titleA, false, 'Note A seed.')
  await selectNote(mobilePage, titleA, true, 'Note A seed.')
  await Promise.all([
    appendText(desktopPage, desktopEditor, 'desktop edit A after return\n'),
    appendText(mobilePage, mobileEditor, 'mobile edit A after return\n'),
  ])

  const result = await waitForConvergence([
    {
      title: titleA,
      documentId: noteA.documentId,
      expected: ['desktop edit A before tab switch', 'mobile edit A before note switch', 'desktop edit A after return', 'mobile edit A after return'],
    },
    {
      title: titleB,
      documentId: noteB.documentId,
      expected: ['desktop edit B after tab switch', 'mobile edit B after note switch'],
    },
  ], { afterFirstA })

  console.log(JSON.stringify(result, null, 2))
  await desktop.close()
  await mobile.close()
} finally {
  await browser.close()
}

async function createStressNote(title, initialText) {
  const response = await fetch(`${apiUrl}/api/v1/notes`, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({ title, path: '/', tags: [], initialText }),
  })
  if (!response.ok) throw new Error(`Failed to create switching stress note: ${response.status} ${await response.text()}`)
  return response.json()
}

async function prepareContext(context) {
  await context.addInitScript(
    ({ apiUrl, authSession }) => {
      localStorage.setItem('og-suite:server-url', apiUrl)
      localStorage.setItem('og-suite:auth:access-token', authSession.accessToken)
      localStorage.setItem('og-suite:auth:refresh-token', authSession.refreshToken)
      localStorage.setItem('og-suite:auth:expires-at', authSession.expiresAt)
      localStorage.setItem('og-suite:notes:editor-render-mode', 'txt')
      localStorage.removeItem('og-suite:notes:local-only')
    },
    { apiUrl, authSession: session },
  )
}

async function openTextNote(page, title, mobileLayout = false, expectedText = '') {
  await page.goto(appUrl, { waitUntil: 'networkidle' })
  await openNotesApp(page)
  await selectNote(page, title, mobileLayout, expectedText)
  const textarea = page.locator('main.notes-app textarea').first()
  await textarea.waitFor()
  await textarea.click()
  return textarea
}

async function openNotesApp(page) {
  await page
    .getByRole('button', { name: 'Notes', exact: true })
    .click({ timeout: 5000 })
    .catch(async () => {
      await page.evaluate(() => {
        const notesButton = Array.from(document.querySelectorAll('button')).find((button) => button.textContent?.trim() === 'Notes')
        notesButton?.dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true }))
      })
    })
}

async function selectNote(page, title, mobileLayout = false, expectedText = '') {
  for (let attempt = 0; attempt < 3; attempt += 1) {
    if (mobileLayout) {
      await openMobileFiles(page)
      const clicked = await page.evaluate((title) => {
        const rows = Array.from(document.querySelectorAll('.notes-list.mobile-open .note-row, .notes-list .note-row'))
        const row = rows.find((item) => item.textContent?.includes(title))
        if (!(row instanceof HTMLElement)) return false
        row.click()
        return true
      }, title)
      if (!clicked) throw new Error(`Could not find mobile note row for ${title}`)
    } else {
      await page.locator('.notes-list button.note-row').filter({ hasText: title }).first().click()
    }
    await page.locator('main.notes-app textarea').first().waitFor()
    const selectedTitle = await page.locator('main.notes-app input[aria-label="Title"]').inputValue().catch(() => '')
    if (selectedTitle === title && (!expectedText || (await waitForEditorText(page, expectedText)))) return
    await page.waitForTimeout(250)
  }
  throw new Error(`Could not select note ${title}`)
}

async function waitForEditorText(page, expectedText) {
  const deadline = Date.now() + 5000
  while (Date.now() < deadline) {
    const value = await page.locator('main.notes-app textarea').first().inputValue().catch(() => '')
    if (value.includes(expectedText)) return true
    await page.waitForTimeout(100)
  }
  return false
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

async function appendText(page, textarea, value) {
  await textarea.click()
  await page.keyboard.press(process.platform === 'darwin' ? 'Meta+End' : 'Control+End')
  await page.keyboard.type(value, { delay: 3 })
  await page.waitForTimeout(320)
}

async function waitForConvergence(notes, debug = {}) {
  const deadline = Date.now() + 15000
  const previews = {}
  while (Date.now() < deadline) {
    const serverValues = Object.fromEntries(await Promise.all(notes.map(async (note) => [note.title, await readServerDocument(note.documentId)])))
    const allOk = notes.every((note) => note.expected.every((part) => serverValues[note.title]?.includes(part)))
    if (allOk) {
      for (const note of notes) previews[note.title] = serverValues[note.title].slice(0, 220)
      return { notes: notes.map((note) => ({ title: note.title, documentId: note.documentId, length: serverValues[note.title].length, preview: previews[note.title] })) }
    }
    await new Promise((resolve) => setTimeout(resolve, 300))
  }
  const serverValues = Object.fromEntries(await Promise.all(notes.map(async (note) => [note.title, await readServerDocument(note.documentId)])))
  throw new Error(JSON.stringify({ message: 'Switching convergence timed out', ...debug, serverValues }, null, 2))
}

async function readServerDocument(documentId) {
  const response = await fetch(`${apiUrl}/api/v1/documents/${documentId}`)
  if (!response.ok) return ''
  const document = await response.json()
  const crdt = await import('../packages/crdt/src/index.ts')
  return crdt.applyUpdates(document).text
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
