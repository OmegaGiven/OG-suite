import { readFile, readdir } from 'node:fs/promises'
import path from 'node:path'

const root = process.cwd()
const searchRoots = ['apps', 'packages/ui']
const colorPattern = /#[0-9a-fA-F]{3,8}\b|rgba?\([^)]+\)|hsla?\([^)]+\)/g
const allowedFragments = [
  'var(',
  'color-mix(',
  'transparent',
  'currentColor',
  'data:image',
]

const findings = []

for (const searchRoot of searchRoots) {
  const directory = path.join(root, searchRoot)
  for (const file of await collectFiles(directory)) {
    if (!/\.(svelte|ts|css)$/.test(file)) continue
    if (file.endsWith(path.join('packages', 'ui', 'src', 'index.ts'))) continue
    const content = await readFile(file, 'utf8')
    const lines = content.split('\n')
    lines.forEach((line, index) => {
      if (allowedFragments.some((fragment) => line.includes(fragment))) return
      const matches = line.match(colorPattern)
      if (!matches) return
      findings.push({
        file: path.relative(root, file),
        line: index + 1,
        colors: [...new Set(matches)],
        source: line.trim(),
      })
    })
  }
}

if (findings.length === 0) {
  console.log('No literal color values found outside tokenized expressions.')
  process.exit(0)
}

const byFile = new Map()
for (const finding of findings) {
  const fileFindings = byFile.get(finding.file) ?? []
  fileFindings.push(finding)
  byFile.set(finding.file, fileFindings)
}
for (const [file, fileFindings] of byFile) {
  console.log(`\n${file}`)
  for (const finding of fileFindings) {
    console.log(`  ${finding.line}: ${finding.colors.join(', ')} | ${finding.source}`)
  }
}

console.log(`\nFound ${findings.length} literal color line${findings.length === 1 ? '' : 's'} to review.`)

async function collectFiles(directory) {
  const entries = await readdir(directory, { withFileTypes: true })
  const files = await Promise.all(
    entries.map((entry) => {
      const entryPath = path.join(directory, entry.name)
      if (['dist', 'node_modules', 'target', 'build', '.gradle'].includes(entry.name)) return []
      return entry.isDirectory() ? collectFiles(entryPath) : entryPath
    }),
  )
  return files.flat()
}
