import { chromium } from '@playwright/test'
import { createStressBrowserLaunchOptions } from './lib/stress-browser.mjs'

const appUrl = process.env.OG_STRESS_APP_URL ?? 'http://localhost:5173/?stress=delete'
const apiUrl = process.env.OG_STRESS_API_URL ?? 'http://127.0.0.1:8080'
const title = `Delete Stress ${Date.now()}`
const initialText = [
  'alpha beta gamma delta epsilon zeta eta theta iota kappa lambda mu',
  'browser and mobile will both delete through this middle section',
  'the final text must converge after backspace and delete operations',
].join('\n')

const session = await getStressSession()
const note = await createStressNote()
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
  await Promise.all([prepareContext(desktop, 'txt'), prepareContext(mobile, 'txt')])

  const desktopPage = await desktop.newPage()
  const mobilePage = await mobile.newPage()
  const desktopTextarea = await openTextNote(desktopPage)
  const mobileTextarea = await openTextNote(mobilePage, true)

  await Promise.all([
    deleteWithKeyboard(desktopPage, desktopTextarea, 'middle section', ['Backspace', 'Backspace', 'Backspace', 'Backspace', 'Delete', 'Delete']),
    deleteWithKeyboard(mobilePage, mobileTextarea, 'middle section', ['Backspace', 'Backspace', 'Delete', 'Backspace', 'Delete', 'Delete']),
  ])
  const afterFirstDelete = await Promise.all([desktopTextarea.inputValue(), mobileTextarea.inputValue()])
  await Promise.all([
    insertMarker(desktopPage, desktopTextarea, 'browser'),
    insertMarker(mobilePage, mobileTextarea, 'mobile'),
  ])
  await Promise.all([
    deleteWithKeyboard(desktopPage, desktopTextarea, 'gamma', ['Delete', 'Delete', 'Backspace']),
    deleteWithKeyboard(mobilePage, mobileTextarea, 'epsilon', ['Backspace', 'Backspace', 'Delete']),
  ])

  const result = await waitForConvergence(desktopTextarea, mobileTextarea, note.documentId, { afterFirstDelete })
  console.log(JSON.stringify({ title, mode: 'txt', ...result }, null, 2))

  await desktop.close()
  await mobile.close()
} finally {
  await browser.close()
}

async function createStressNote() {
  const response = await fetch(`${apiUrl}/api/v1/notes`, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({ title, path: '/', tags: [], initialText }),
  })
  if (!response.ok) throw new Error(`Failed to create delete stress note: ${response.status} ${await response.text()}`)
  return response.json()
}

async function prepareContext(context, editorMode) {
  await context.addInitScript(
    ({ apiUrl, authSession, editorMode }) => {
      localStorage.setItem('og-suite:server-url', apiUrl)
      localStorage.setItem('og-suite:auth:access-token', authSession.accessToken)
      localStorage.setItem('og-suite:auth:refresh-token', authSession.refreshToken)
      localStorage.setItem('og-suite:auth:expires-at', authSession.expiresAt)
      localStorage.setItem('og-suite:notes:editor-render-mode', editorMode)
      localStorage.removeItem('og-suite:notes:local-only')
    },
    { apiUrl, authSession: session, editorMode },
  )
}

async function openTextNote(page, mobileLayout = false) {
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
  if (mobileLayout) await page.getByRole('button', { name: 'Open files' }).click({ timeout: 5000 }).catch(() => {})
  await page.locator('button').filter({ hasText: title }).first().click()
  const textarea = page.locator('textarea')
  await textarea.waitFor()
  await textarea.click()
  return textarea
}

async function deleteWithKeyboard(page, textarea, needle, keys) {
  await focusAfterNeedle(textarea, needle)
  for (const key of keys) {
    await page.keyboard.press(key)
    await page.waitForTimeout(35)
  }
  await page.waitForTimeout(260)
}

async function insertMarker(page, textarea, needle) {
  await focusAfterNeedle(textarea, needle)
  await page.keyboard.type(`[${needle}-kept]`, { delay: 4 })
  await page.waitForTimeout(260)
}

async function focusAfterNeedle(textarea, needle) {
  await textarea.click()
  await textarea.evaluate((node, needle) => {
    node.focus()
    const index = node.value.indexOf(needle)
    const position = index >= 0 ? index + needle.length : node.value.length
    node.setSelectionRange(position, position)
  }, needle)
}

async function waitForConvergence(desktopTextarea, mobileTextarea, documentId, debug = {}) {
  const deadline = Date.now() + 12000
  let desktopValue = ''
  let mobileValue = ''
  let serverValue = ''
  while (Date.now() < deadline) {
    ;[desktopValue, mobileValue, serverValue] = await Promise.all([
      desktopTextarea.inputValue(),
      mobileTextarea.inputValue(),
      readServerDocument(documentId),
    ])
    if (desktopValue === mobileValue && desktopValue === serverValue && desktopValue.includes('[browser-kept]') && desktopValue.includes('[mobile-kept]')) {
      return {
        length: desktopValue.length,
        desktopPreview: desktopValue.slice(0, 220),
        mobilePreview: mobileValue.slice(0, 220),
        serverPreview: serverValue.slice(0, 220),
      }
    }
    await new Promise((resolve) => setTimeout(resolve, 300))
  }
  throw new Error(JSON.stringify({ message: 'Delete convergence timed out', ...debug, desktopValue, mobileValue, serverValue }, null, 2))
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
