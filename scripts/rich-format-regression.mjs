import { chromium } from '@playwright/test'

const appUrl = 'http://localhost:5173/?stress=rich-format'
const apiUrl = 'http://127.0.0.1:8080'
const bravePath = '/Applications/Brave Browser.app/Contents/MacOS/Brave Browser'
const title = `Rich Format ${Date.now()}`

const noteResponse = await fetch(`${apiUrl}/api/v1/notes`, {
  method: 'POST',
  headers: { 'content-type': 'application/json' },
  body: JSON.stringify({
    title,
    path: '/',
    tags: [],
    initialText: '#Heading One\n##Heading Two\nNormal paragraph',
  }),
})

if (!noteResponse.ok) {
  throw new Error(`Failed to create rich format note: ${noteResponse.status} ${await noteResponse.text()}`)
}

const browser = await chromium.launch({ executablePath: bravePath, headless: true })
const context = await browser.newContext({ viewport: { width: 1280, height: 860 } })
await context.addInitScript(() => localStorage.setItem('og-suite:notes:editor-render-mode', 'rich'))
const page = await context.newPage()

try {
  await page.goto(appUrl, { waitUntil: 'networkidle' })
  await page.getByRole('button', { name: title, exact: true }).click()
  const editor = page.locator('.rich-editor-content')
  await editor.waitFor()

  const headingOne = await editor.locator('h1').innerText()
  const headingTwo = await editor.locator('h2').innerText()
  if (headingOne !== 'Heading One' || headingTwo !== 'Heading Two') {
    throw new Error(JSON.stringify({ message: 'Markdown heading mapping failed', headingOne, headingTwo }, null, 2))
  }

  await editor.locator('p').filter({ hasText: 'Normal paragraph' }).click()
  await page.getByRole('button', { name: 'Indent', exact: true }).click()
  await page.waitForFunction(() => document.querySelector('.rich-editor-content p')?.getAttribute('data-indent') === '1')
  const indent = await editor.locator('p').filter({ hasText: 'Normal paragraph' }).getAttribute('data-indent')
  if (indent !== '1') {
    throw new Error(JSON.stringify({ message: 'Rich paragraph indent failed', indent }, null, 2))
  }

  const boldButton = page.getByRole('button', { name: 'Bold', exact: true })
  await boldButton.click()
  await page.waitForFunction(() => document.querySelector('button[aria-label="Bold"]')?.classList.contains('active-action'))
  const boldClass = await boldButton.getAttribute('class')
  if (!boldClass?.includes('active-action')) {
    throw new Error(JSON.stringify({ message: 'Bold active outline failed', boldClass }, null, 2))
  }

  console.log(JSON.stringify({ title, headingOne, headingTwo, indent, boldClass }, null, 2))
} finally {
  await browser.close()
}
