import { chromium } from '@playwright/test'

const appUrl = process.env.OG_STRESS_APP_URL ?? 'http://localhost:5173/?stress=rich'
const apiUrl = process.env.OG_STRESS_API_URL ?? 'http://127.0.0.1:8080'
const bravePath = '/Applications/Brave Browser.app/Contents/MacOS/Brave Browser'

const desktopText =
  'Desktop rich typing stays stable while the editor scrolls and collaborator updates arrive. Desktop keeps typing after the first remote merge. '
const peerText =
  'Peer rich typing stays stable while the editor scrolls and collaborator updates arrive. Peer keeps typing after the first remote merge. '
const mobileText =
  'Mobile rich typing stays stable while the editor scrolls and collaborator updates arrive. Mobile keeps typing after the first remote merge. '
const desktopParts = ['Desktop rich typing', 'collaborator updates arrive', 'Desktop keeps typing', 'first remote merge']
const peerParts = ['Peer rich typing', 'collaborator updates arrive', 'Peer keeps typing', 'first remote merge']
const mobileParts = ['Mobile rich typing', 'collaborator updates arrive', 'Mobile keeps typing', 'first remote merge']

const session = await getStressSession()
const browser = await chromium.launch({ executablePath: bravePath, headless: true })

try {
  const desktopDesktop = await runScenario({
    label: 'two-desktop',
    firstText: desktopText,
    secondText: peerText,
    expectedParts: [...desktopParts, ...peerParts],
    secondIsMobile: false,
    secondPostScrollText: 'Peer after scroll. ',
  })
  const desktopMobile = await runScenario({
    label: 'desktop-mobile',
    firstText: desktopText,
    secondText: mobileText,
    expectedParts: [...desktopParts, ...mobileParts],
    secondIsMobile: true,
    secondPostScrollText: 'Mobile after scroll. ',
  })

  console.log(JSON.stringify({ desktopDesktop, desktopMobile }, null, 2))
} finally {
  await browser.close()
}

async function runScenario({ label, firstText, secondText, expectedParts, secondIsMobile, secondPostScrollText }) {
  const title = `Rich Stress ${label} ${Date.now()}`
  const noteResponse = await fetch(`${apiUrl}/api/v1/notes`, {
    method: 'POST',
    headers: {
      'content-type': 'application/json',
      authorization: `Bearer ${session.accessToken}`,
    },
    body: JSON.stringify({ title, path: '/', tags: [], initialText: '' }),
  })

  if (!noteResponse.ok) {
    throw new Error(`Failed to create rich stress note: ${noteResponse.status} ${await noteResponse.text()}`)
  }

  const firstContext = await newRichContext({ width: 1280, height: 860 })
  const secondContext = secondIsMobile
    ? await newRichContext({
        width: 390,
        height: 844,
        isMobile: true,
        hasTouch: true,
        userAgent:
          'Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.0 Mobile/15E148 Safari/604.1',
      })
    : await newRichContext({ width: 1180, height: 820 })

  const firstPage = await firstContext.newPage()
  const secondPage = await secondContext.newPage()
  try {
    const firstEditor = await openRichNote(firstPage, title)
    const secondEditor = await openRichNote(secondPage, title, secondIsMobile)

    await Promise.all([
      firstPage.keyboard.type(firstText.repeat(5), { delay: 2 }),
      secondPage.keyboard.type(secondText.repeat(5), { delay: 2 }),
    ])

    await Promise.all([
      firstPage.keyboard.press('ArrowUp'),
      secondPage.keyboard.press('ArrowUp'),
      firstPage.mouse.wheel(0, -450),
      secondPage.mouse.wheel(0, -450),
    ])
    await Promise.all([
      firstPage.keyboard.type('Desktop after scroll. ', { delay: 2 }),
      secondPage.keyboard.type(secondPostScrollText, { delay: 2 }),
    ])

    await waitForRichConvergence(firstEditor, secondEditor, [...expectedParts, 'Desktop after scroll. ', secondPostScrollText])

    const reloadValue = await reloadAndReadRichNote(firstPage, title)
    assertIncludes('reload', reloadValue, [...expectedParts, 'Desktop after scroll. ', secondPostScrollText])

    const switchedValue = await switchModesAndReturnRich(firstPage)
    assertIncludes('mode-switch', switchedValue, [...expectedParts, 'Desktop after scroll. ', secondPostScrollText])

    const [firstValue, secondValue] = await Promise.all([firstEditor.innerText(), secondEditor.innerText()])
    return {
      title,
      firstLength: firstValue.length,
      secondLength: secondValue.length,
      reloadLength: reloadValue.length,
      switchedLength: switchedValue.length,
      firstPreview: firstValue.slice(0, 180),
      secondPreview: secondValue.slice(0, 180),
    }
  } finally {
    await firstContext.close()
    await secondContext.close()
  }
}

async function newRichContext(options) {
  const { width, height, ...contextOptions } = options
  const context = await browser.newContext({ viewport: { width, height }, ...contextOptions })
  await context.addInitScript(
    ({ apiUrl, authSession }) => {
      localStorage.setItem('og-suite:notes:editor-render-mode', 'rich')
      localStorage.setItem('og-suite:server-url', apiUrl)
      localStorage.setItem('og-suite:auth:access-token', authSession.accessToken)
      localStorage.setItem('og-suite:auth:refresh-token', authSession.refreshToken)
      localStorage.setItem('og-suite:auth:expires-at', authSession.expiresAt)
      localStorage.removeItem('og-suite:notes:local-only')
    },
    { apiUrl, authSession: session },
  )
  return context
}

async function openRichNote(page, title, mobileLayout = false) {
  await page.goto(appUrl, { waitUntil: 'networkidle' })
  await openNotesApp(page)
  await selectNoteRow(page, title, mobileLayout)
  const editor = page.locator('.rich-editor-content')
  await editor.click()
  return editor
}

async function selectNoteRow(page, title, mobileLayout = false) {
  for (let attempt = 0; attempt < 3; attempt += 1) {
    if (mobileLayout) {
      await openMobileFiles(page)
      await page.evaluate((title) => {
        const row = Array.from(document.querySelectorAll('.notes-list.mobile-open .note-row, .notes-list .note-row')).find((item) => item.textContent?.includes(title))
        if (row instanceof HTMLElement) row.click()
      }, title)
    } else {
      await page.locator('.notes-list button.note-row').filter({ hasText: title }).first().click()
    }
    await page.locator('.rich-editor-content').waitFor()
    const selectedTitle = await page.locator('input[aria-label="Title"]').inputValue().catch(() => '')
    if (selectedTitle === title) return
    await page.waitForTimeout(250)
  }
  throw new Error(`Could not select rich note ${title}`)
}

async function openMobileFiles(page) {
  await page.evaluate(() => {
    const button = Array.from(document.querySelectorAll('button')).find((item) => item.getAttribute('aria-label') === 'Open files')
    if (button instanceof HTMLElement) button.click()
  })
  await page.locator('.notes-list.mobile-open').waitFor({ state: 'visible', timeout: 5000 }).catch(() => {})
}

async function reloadAndReadRichNote(page, title) {
  await page.reload({ waitUntil: 'networkidle' })
  await openNotesApp(page)
  await page.locator('button').filter({ hasText: title }).first().click()
  const editor = page.locator('.rich-editor-content')
  await editor.waitFor()
  return editor.innerText()
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

async function switchModesAndReturnRich(page) {
  await page.getByRole('button', { name: 'MD', exact: true }).click()
  await page.locator('textarea').waitFor()
  await page.getByRole('button', { name: 'TXT', exact: true }).click()
  await page.locator('textarea').waitFor()
  await page.getByRole('button', { name: 'RICH', exact: true }).click()
  const editor = page.locator('.rich-editor-content')
  await editor.waitFor()
  return editor.innerText()
}

async function waitForRichConvergence(firstEditor, secondEditor, expectedParts) {
  const deadline = Date.now() + 12000
  let firstValue = ''
  let secondValue = ''
  while (Date.now() < deadline) {
    ;[firstValue, secondValue] = await Promise.all([firstEditor.innerText(), secondEditor.innerText()])
    const firstOk = includesAll(firstValue, expectedParts)
    const secondOk = includesAll(secondValue, expectedParts)
    if (firstOk && secondOk) return
    await new Promise((resolve) => setTimeout(resolve, 250))
  }
  throw new Error(
    JSON.stringify(
      {
        message: 'Rich convergence timed out',
        firstMissing: missingParts(firstValue, expectedParts),
        secondMissing: missingParts(secondValue, expectedParts),
        firstValue,
        secondValue,
      },
      null,
      2,
    ),
  )
}

function assertIncludes(label, value, expectedParts) {
  const missing = missingParts(value, expectedParts)
  if (missing.length) {
    throw new Error(JSON.stringify({ label, missing, value }, null, 2))
  }
}

function includesAll(value, expectedParts) {
  return missingParts(value, expectedParts).length === 0
}

function missingParts(value, expectedParts) {
  return expectedParts
    .map((part) => part.trim())
    .filter(Boolean)
    .filter((part) => !value.includes(part))
}

async function getStressSession() {
  const username = `rich-stress-${Date.now()}`
  const password = 'stress-password-123'
  const registered = await fetch(`${apiUrl}/api/v1/auth/register`, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({ username, displayName: 'Rich Stress User', password }),
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
