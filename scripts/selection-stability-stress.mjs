import { chromium } from '@playwright/test'
import * as Y from 'yjs'

const appUrl = process.env.OG_STRESS_APP_URL ?? 'http://127.0.0.1:5173/?stress=selection'
const apiUrl = process.env.OG_STRESS_API_URL ?? 'http://127.0.0.1:8080'
const bravePath = '/Applications/Brave Browser.app/Contents/MOS/Brave Browser'.replace('/MOS/', '/MacOS/')
const title = `Selection Stress ${Date.now()}`
const baseText = 'Alpha beta gamma delta epsilon zeta eta theta iota kappa.\n'
const peerText = 'Peer inserted text while selection is active.\n'

const session = await getStressSession()
const noteResponse = await fetch(`${apiUrl}/api/v1/notes`, {
  method: 'POST',
  headers: { 'content-type': 'application/json' },
  body: JSON.stringify({ title, path: '/', tags: [], initialText: baseText }),
})

if (!noteResponse.ok) {
  throw new Error(`Failed to create selection stress note: ${noteResponse.status} ${await noteResponse.text()}`)
}

const browser = await chromium.launch({ executablePath: bravePath, headless: true })

try {
  const first = await browser.newContext({ viewport: { width: 1280, height: 860 } })
  await prepareContext(first, session)

  const firstPage = await first.newPage()
  const firstTextarea = await openNote(firstPage)

  await firstTextarea.evaluate((node) => node.setSelectionRange(6, 22, 'forward'))
  const before = await firstTextarea.evaluate((node) => ({
    start: node.selectionStart,
    end: node.selectionEnd,
    text: node.value.slice(node.selectionStart, node.selectionEnd),
  }))

  const note = await noteResponse.json()
  await appendPeerUpdate(note.documentId)
  await firstPage.waitForTimeout(3500)

  const during = await firstTextarea.evaluate((node) => ({
    start: node.selectionStart,
    end: node.selectionEnd,
    text: node.value.slice(node.selectionStart, node.selectionEnd),
    value: node.value,
  }))

  if (during.start !== before.start || during.end !== before.end || during.text !== before.text) {
    throw new Error(JSON.stringify({ message: 'selection changed while remote update arrived', before, during }, null, 2))
  }

  console.log(
    JSON.stringify(
      {
        title,
        before,
        during: { start: during.start, end: during.end, text: during.text },
        serverUpdateApplied: true,
      },
      null,
      2,
    ),
  )
} finally {
  await browser.close()
}

async function openNote(page, mobileLayout = false) {
  await page.goto(appUrl, { waitUntil: 'networkidle' })
  await openNotesApp(page)
  if (mobileLayout) await page.getByRole('button', { name: 'Open files' }).click()
  await page.locator('.notes-list button.note-row').filter({ hasText: title }).first().click()
  const textarea = page.locator('textarea')
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

async function prepareContext(context, authSession) {
  await context.addInitScript(
    ({ apiUrl, authSession }) => {
      localStorage.setItem('og-suite:server-url', apiUrl)
      localStorage.setItem('og-suite:auth:access-token', authSession.accessToken)
      localStorage.setItem('og-suite:auth:refresh-token', authSession.refreshToken)
      localStorage.setItem('og-suite:auth:expires-at', authSession.expiresAt)
      localStorage.removeItem('og-suite:notes:local-only')
      localStorage.setItem('og-suite:notes:editor-render-mode', 'text')
    },
    { apiUrl, authSession },
  )
}

async function appendPeerUpdate(documentId) {
  const documentResponse = await fetch(`${apiUrl}/api/v1/documents/${documentId}`)
  if (!documentResponse.ok) throw new Error(`Failed to load document: ${documentResponse.status}`)
  const document = await documentResponse.json()
  const doc = new Y.Doc()
  applyStoredUpdate(doc, document.snapshot)
  for (const update of document.updates) applyStoredUpdate(doc, update.payload)
  const stateVector = Y.encodeStateVector(doc)
  doc.getText('content').insert(doc.getText('content').length, peerText)
  const payload = Buffer.from(Y.encodeStateAsUpdate(doc, stateVector)).toString('base64')
  const response = await fetch(`${apiUrl}/api/v1/documents/${documentId}/updates`, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({
      updates: [
        {
          documentId,
          clientId: 'selection-stress-peer',
          sequence: Date.now(),
          payload,
        },
      ],
    }),
  })
  if (!response.ok) throw new Error(`Failed to append peer update: ${response.status} ${await response.text()}`)
}

function applyStoredUpdate(doc, payload) {
  if (!payload) return
  try {
    Y.applyUpdate(doc, new Uint8Array(Buffer.from(payload, 'base64')))
  } catch {
    const text = doc.getText('content')
    if (text.length === 0) text.insert(0, payload)
  }
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
