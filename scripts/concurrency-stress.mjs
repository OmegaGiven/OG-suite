import { chromium } from '@playwright/test'
import { createStressBrowserLaunchOptions } from './lib/stress-browser.mjs'

const appUrl = process.env.OG_STRESS_APP_URL ?? 'http://localhost:5173/?stress=100wpm'
const apiUrl = process.env.OG_STRESS_API_URL ?? 'http://127.0.0.1:8080'
const title = `Stress ${Date.now()}`
const seedText = 'Seed line before the remote flush starts.\n'
const desktopText =
  'Desktop burst one keeps typing while sync flushes in the background. Desktop burst two should not disappear after the server answers. Desktop burst three keeps caret position stable under heavy input.\n'
const mobileText =
  'Mobile burst one keeps typing while sync flushes in the background. Mobile burst two should not disappear after the server answers. Mobile burst three keeps caret position stable under heavy input.\n'

const session = await getStressSession()
const noteResponse = await fetch(`${apiUrl}/api/v1/notes`, {
  method: 'POST',
  headers: { 'content-type': 'application/json' },
  body: JSON.stringify({ title, path: '/', tags: [], initialText: '' }),
})

if (!noteResponse.ok) {
  throw new Error(`Failed to create stress note: ${noteResponse.status} ${await noteResponse.text()}`)
}

const browser = await chromium.launch(createStressBrowserLaunchOptions())
const desktop = await browser.newContext({ viewport: { width: 1280, height: 860 } })
const mobile = await browser.newContext({
  viewport: { width: 390, height: 844 },
  isMobile: true,
  hasTouch: true,
  userAgent:
    'Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.0 Mobile/15E148 Safari/604.1',
})
await Promise.all([prepareContext(desktop, session), prepareContext(mobile, session)])

const desktopPage = await desktop.newPage()
const mobilePage = await mobile.newPage()

async function openNote(page, mobileLayout = false) {
  await page.goto(appUrl, { waitUntil: 'networkidle' })
  await openNotesApp(page)
  if (mobileLayout) {
    await page.getByRole('button', { name: 'Open files' }).click({ timeout: 5000 }).catch(() => {})
  }
  await page.locator('button').filter({ hasText: title }).first().click()
  const textarea = page.locator('textarea')
  await textarea.click()
  await page.keyboard.press(process.platform === 'darwin' ? 'Meta+A' : 'Control+A')
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

const desktopTextarea = await openNote(desktopPage)
const mobileTextarea = await openNote(mobilePage, true)

await desktopPage.keyboard.type(seedText, { delay: 4 })
await desktopPage.waitForTimeout(260)

await Promise.all([
  desktopPage.keyboard.type(desktopText, { delay: 4 }),
  mobilePage.keyboard.type(mobileText, { delay: 4 }),
])

await desktopPage.waitForTimeout(6500)

const [desktopValue, mobileValue, desktopSelection, mobileSelection] = await Promise.all([
  desktopTextarea.inputValue(),
  mobileTextarea.inputValue(),
  desktopTextarea.evaluate((node) => ({ start: node.selectionStart, end: node.selectionEnd, length: node.value.length })),
  mobileTextarea.evaluate((node) => ({ start: node.selectionStart, end: node.selectionEnd, length: node.value.length })),
])

const missing = []
for (const [label, value] of [
  ['desktop', desktopValue],
  ['mobile', mobileValue],
]) {
  if (!value.includes(seedText.trim())) missing.push(`${label} missing seed text`)
  for (const segment of ['Desktop burst one', 'Desktop burst two', 'Desktop burst three']) {
    if (!value.includes(segment)) missing.push(`${label} missing ${segment}`)
  }
  for (const segment of ['Mobile burst one', 'Mobile burst two', 'Mobile burst three']) {
    if (!value.includes(segment)) missing.push(`${label} missing ${segment}`)
  }
}

const cursorProblems = []
if (desktopSelection.start < desktopText.length - 8 && desktopSelection.start < mobileText.length - 8) {
  cursorProblems.push(`desktop cursor suspicious: ${JSON.stringify(desktopSelection)}`)
}
if (mobileSelection.start < desktopText.length - 8 && mobileSelection.start < mobileText.length - 8) {
  cursorProblems.push(`mobile cursor suspicious: ${JSON.stringify(mobileSelection)}`)
}

await browser.close()

if (missing.length || cursorProblems.length) {
  console.error(JSON.stringify({ title, missing, cursorProblems, desktopValue, mobileValue, desktopSelection, mobileSelection }, null, 2))
  process.exit(1)
}

console.log(
  JSON.stringify(
    {
      title,
      desktopSelection,
      mobileSelection,
      desktopLength: desktopValue.length,
      mobileLength: mobileValue.length,
      mergedText: desktopValue,
    },
    null,
    2,
  ),
)

async function prepareContext(context, authSession) {
  await context.addInitScript(
    ({ apiUrl, authSession }) => {
      localStorage.setItem('og-suite:server-url', apiUrl)
      localStorage.setItem('og-suite:auth:access-token', authSession.accessToken)
      localStorage.setItem('og-suite:auth:refresh-token', authSession.refreshToken)
      localStorage.setItem('og-suite:auth:expires-at', authSession.expiresAt)
      localStorage.removeItem('og-suite:notes:local-only')
    },
    { apiUrl, authSession },
  )
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
      await setup.json()
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
