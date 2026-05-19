import { chromium } from '@playwright/test'

const appUrl = 'http://localhost:5173/?stress=100wpm'
const apiUrl = 'http://127.0.0.1:8080'
const title = `Stress ${Date.now()}`
const seedText = 'Seed line before the remote flush starts.\n'
const desktopText =
  'Desktop burst one keeps typing while sync flushes in the background. Desktop burst two should not disappear after the server answers. Desktop burst three keeps caret position stable under heavy input.\n'
const mobileText =
  'Mobile burst one keeps typing while sync flushes in the background. Mobile burst two should not disappear after the server answers. Mobile burst three keeps caret position stable under heavy input.\n'
const bravePath = '/Applications/Brave Browser.app/Contents/MacOS/Brave Browser'

const noteResponse = await fetch(`${apiUrl}/api/v1/notes`, {
  method: 'POST',
  headers: { 'content-type': 'application/json' },
  body: JSON.stringify({ title, path: '/', tags: [], initialText: '' }),
})

if (!noteResponse.ok) {
  throw new Error(`Failed to create stress note: ${noteResponse.status} ${await noteResponse.text()}`)
}

const browser = await chromium.launch({ executablePath: bravePath, headless: true })
const desktop = await browser.newContext({ viewport: { width: 1280, height: 860 } })
const mobile = await browser.newContext({
  viewport: { width: 390, height: 844 },
  isMobile: true,
  hasTouch: true,
  userAgent:
    'Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.0 Mobile/15E148 Safari/604.1',
})

const desktopPage = await desktop.newPage()
const mobilePage = await mobile.newPage()

async function openNote(page, mobileLayout = false) {
  await page.goto(appUrl, { waitUntil: 'networkidle' })
  if (mobileLayout) {
    await page.getByRole('button', { name: 'Open files' }).click()
  }
  await page.getByRole('button', { name: title, exact: true }).click()
  const textarea = page.locator('textarea')
  await textarea.click()
  await page.keyboard.press(process.platform === 'darwin' ? 'Meta+A' : 'Control+A')
  return textarea
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
