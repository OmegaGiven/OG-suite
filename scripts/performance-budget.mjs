import { gzipSync } from 'node:zlib'
import { readFile, readdir } from 'node:fs/promises'
import { existsSync } from 'node:fs'
import path from 'node:path'

const root = process.cwd()
const configPath = path.join(root, 'performance-budgets.json')
const config = JSON.parse(await readFile(configPath, 'utf8'))
const failures = []
const reports = []

for (const [appName, budget] of Object.entries(config.apps ?? {})) {
  const distPath = path.join(root, budget.dist)
  if (!existsSync(distPath)) {
    const message = `${appName}: missing ${budget.dist}; run npm run build first`
    if (budget.required) failures.push(message)
    else reports.push({ appName, skipped: true, message })
    continue
  }

  const files = await collectFiles(distPath)
  const initialAssets = files.filter((file) => /\.(js|css)$/i.test(file))
  const js = await sizeGroup(initialAssets.filter((file) => file.endsWith('.js')))
  const css = await sizeGroup(initialAssets.filter((file) => file.endsWith('.css')))
  const total = {
    bytes: js.bytes + css.bytes,
    gzipBytes: js.gzipBytes + css.gzipBytes,
  }

  checkLimit(appName, 'initial JS', js.bytes, budget.maxInitialJsBytes)
  checkLimit(appName, 'initial JS gzip', js.gzipBytes, budget.maxInitialJsGzipBytes)
  checkLimit(appName, 'initial CSS', css.bytes, budget.maxInitialCssBytes)
  checkLimit(appName, 'initial CSS gzip', css.gzipBytes, budget.maxInitialCssGzipBytes)
  checkLimit(appName, 'initial assets', total.bytes, budget.maxInitialAssetBytes)
  checkLimit(appName, 'initial assets gzip', total.gzipBytes, budget.maxInitialAssetGzipBytes)

  reports.push({ appName, js, css, total })
}

for (const report of reports) {
  if (report.skipped) {
    console.log(report.message)
    continue
  }
  console.log(
    [
      `${report.appName}:`,
      `js ${formatBytes(report.js.bytes)} (${formatBytes(report.js.gzipBytes)} gzip)`,
      `css ${formatBytes(report.css.bytes)} (${formatBytes(report.css.gzipBytes)} gzip)`,
      `total ${formatBytes(report.total.bytes)} (${formatBytes(report.total.gzipBytes)} gzip)`,
    ].join(' '),
  )
}

if (failures.length > 0) {
  console.error('\nPerformance budget failures:')
  for (const failure of failures) console.error(`- ${failure}`)
  process.exit(1)
}

console.log('\nPerformance budgets passed.')

async function collectFiles(directory) {
  const entries = await readdir(directory, { withFileTypes: true })
  const files = await Promise.all(
    entries.map((entry) => {
      const entryPath = path.join(directory, entry.name)
      return entry.isDirectory() ? collectFiles(entryPath) : entryPath
    }),
  )
  return files.flat()
}

async function sizeGroup(files) {
  let bytes = 0
  let gzipBytes = 0
  for (const file of files) {
    const content = await readFile(file)
    bytes += content.byteLength
    gzipBytes += gzipSync(content).byteLength
  }
  return { bytes, gzipBytes, count: files.length }
}

function checkLimit(appName, label, actual, limit) {
  if (typeof limit !== 'number') return
  if (actual > limit) failures.push(`${appName}: ${label} ${formatBytes(actual)} exceeds ${formatBytes(limit)}`)
}

function formatBytes(bytes) {
  if (bytes < 1024) return `${bytes} B`
  return `${(bytes / 1024).toFixed(1)} KB`
}
