import fs from 'node:fs'
import path from 'node:path'
import { spawn } from 'node:child_process'
import process from 'node:process'

const rootDir = process.cwd()
const runId = new Date().toISOString().replace(/[:.]/g, '-')
const artifactsDir = path.join(rootDir, 'artifacts', 'sync-regression', runId)
fs.mkdirSync(artifactsDir, { recursive: true })

const suitePort = Number.parseInt(process.env.OG_SUITE_TEST_PORT ?? '4173', 10)
const apiPort = Number.parseInt(process.env.OG_SUITE_TEST_API_PORT ?? '18081', 10)
const suiteUrl = process.env.OG_STRESS_APP_BASE_URL ?? `http://127.0.0.1:${suitePort}`
const apiUrl = process.env.OG_STRESS_API_URL ?? `http://127.0.0.1:${apiPort}`
const shouldStartServers = !process.argv.includes('--reuse-existing')

const scenarios = {
  concurrency: {
    script: 'scripts/concurrency-stress.mjs',
    query: '?stress=100wpm',
    description: 'Desktop/mobile concurrent typing convergence in text mode',
  },
  delete: {
    script: 'scripts/delete-concurrency-stress.mjs',
    query: '?stress=delete',
    description: 'Concurrent delete/backspace convergence in text mode',
  },
  createDelete: {
    script: 'scripts/create-delete-sync-stress.mjs',
    query: '?stress=create-delete',
    description: 'Cross-device create/delete propagation',
  },
  switching: {
    script: 'scripts/switching-concurrency-stress.mjs',
    query: '?stress=switching',
    description: 'Switching tabs/notes while concurrent edits continue',
  },
  rich: {
    script: 'scripts/rich-concurrency-stress.mjs',
    query: '?stress=rich',
    description: 'Rich mode convergence across desktop/mobile peers',
  },
  selection: {
    script: 'scripts/selection-stability-stress.mjs',
    query: '?stress=selection',
    description: 'Selection stability while remote updates arrive',
  },
}

if (process.argv.includes('--list')) {
  for (const [name, scenario] of Object.entries(scenarios)) {
    console.log(`${name}: ${scenario.description}`)
  }
  process.exit(0)
}

const selectedNames = parseSelectedScenarioNames(process.argv, Object.keys(scenarios))
const selectedScenarios = selectedNames.map((name) => ({ name, ...scenarios[name] }))
const activeChildren = new Set()

process.on('SIGINT', async () => {
  await shutdownAll()
  process.exit(130)
})

process.on('SIGTERM', async () => {
  await shutdownAll()
  process.exit(143)
})

try {
  if (shouldStartServers) {
    const dataDir = path.join(artifactsDir, 'server-data')
    fs.mkdirSync(dataDir, { recursive: true })
    const backend = startLoggedProcess({
      name: 'backend',
      command: 'cargo',
      args: ['run', '--manifest-path', 'backend/Cargo.toml'],
      env: {
        ...process.env,
        OG_SUITE_BIND: `127.0.0.1:${apiPort}`,
        OG_SUITE_DATA_DIR: dataDir,
      },
    })
    await waitForHttp(`${apiUrl}/api/v1/system/version`, 60_000)

    const suite = startLoggedProcess({
      name: 'suite',
      command: resolveLocalBin('vite'),
      args: ['--host', '127.0.0.1', '--port', String(suitePort), '--strictPort'],
      cwd: path.join(rootDir, 'apps', 'suite'),
      env: {
        ...process.env,
        VITE_OG_API_URL: apiUrl,
      },
    })
    await waitForHttp(suiteUrl, 30_000)
  }

  const results = []
  for (const scenario of selectedScenarios) {
    const result = await runScenario(scenario)
    results.push(result)
    renderProgress(results)
  }

  const failed = results.filter((result) => !result.ok)
  const summary = {
    runId,
    suiteUrl,
    apiUrl,
    artifactsDir,
    selectedScenarios: selectedNames,
    passed: results.length - failed.length,
    failed: failed.length,
    results,
  }
  fs.writeFileSync(path.join(artifactsDir, 'summary.json'), `${JSON.stringify(summary, null, 2)}\n`)
  console.log(`\nSummary written to ${path.join(artifactsDir, 'summary.json')}`)
  if (failed.length > 0) process.exit(1)
} finally {
  await shutdownAll()
}

function parseSelectedScenarioNames(argv, defaults) {
  const argument = argv.find((value) => value.startsWith('--cases='))
  if (!argument) return defaults
  const requested = argument
    .slice('--cases='.length)
    .split(',')
    .map((value) => value.trim())
    .filter(Boolean)
  const invalid = requested.filter((name) => !scenarios[name])
  if (invalid.length > 0) {
    throw new Error(`Unknown scenarios: ${invalid.join(', ')}`)
  }
  return requested
}

function resolveLocalBin(name) {
  return path.join(rootDir, 'node_modules', '.bin', name)
}

function startLoggedProcess({ name, command, args, cwd = rootDir, env = process.env }) {
  const logPath = path.join(artifactsDir, `${name}.log`)
  const logStream = fs.createWriteStream(logPath, { flags: 'a' })
  const child = spawn(command, args, {
    cwd,
    env,
    stdio: ['ignore', 'pipe', 'pipe'],
  })
  activeChildren.add(child)
  child.stdout.on('data', (chunk) => logStream.write(chunk))
  child.stderr.on('data', (chunk) => logStream.write(chunk))
  child.on('exit', () => {
    activeChildren.delete(child)
    logStream.end()
  })
  return { child, logPath, name }
}

async function runScenario(scenario) {
  const startedAt = Date.now()
  const env = {
    ...process.env,
    OG_STRESS_APP_URL: `${suiteUrl}/${scenario.query}`,
    OG_STRESS_API_URL: apiUrl,
  }
  const logPath = path.join(artifactsDir, `${scenario.name}.log`)
  const outputPath = path.join(artifactsDir, `${scenario.name}.json`)
  const child = spawn(process.execPath, [path.join(rootDir, scenario.script)], {
    cwd: rootDir,
    env,
    stdio: ['ignore', 'pipe', 'pipe'],
  })
  activeChildren.add(child)

  const stdout = []
  const stderr = []
  const logStream = fs.createWriteStream(logPath, { flags: 'a' })
  child.stdout.on('data', (chunk) => {
    stdout.push(chunk)
    logStream.write(chunk)
  })
  child.stderr.on('data', (chunk) => {
    stderr.push(chunk)
    logStream.write(chunk)
  })

  const exit = await new Promise((resolve) => {
    child.on('exit', (code, signal) => resolve({ code, signal }))
  })
  activeChildren.delete(child)
  logStream.end()

  const stdoutText = Buffer.concat(stdout).toString('utf8').trim()
  const stderrText = Buffer.concat(stderr).toString('utf8').trim()
  const payload = tryParseLastJson(stdoutText) ?? { stdout: stdoutText }
  fs.writeFileSync(outputPath, `${JSON.stringify(payload, null, 2)}\n`)

  return {
    name: scenario.name,
    description: scenario.description,
    ok: exit.code === 0,
    exitCode: exit.code,
    signal: exit.signal,
    durationMs: Date.now() - startedAt,
    logPath,
    outputPath,
    error: exit.code === 0 ? null : stderrText || stdoutText || 'Scenario failed without output',
  }
}

function tryParseLastJson(text) {
  if (!text) return null
  const candidates = text
    .split(/\n(?=\{)/g)
    .map((value) => value.trim())
    .filter(Boolean)
    .reverse()
  for (const candidate of candidates) {
    try {
      return JSON.parse(candidate)
    } catch {
      continue
    }
  }
  try {
    return JSON.parse(text)
  } catch {
    return null
  }
}

async function waitForHttp(url, timeoutMs) {
  const deadline = Date.now() + timeoutMs
  let lastError = 'unknown error'
  while (Date.now() < deadline) {
    try {
      const response = await fetch(url)
      if (response.ok || response.status < 500) return
      lastError = `HTTP ${response.status}`
    } catch (error) {
      lastError = error instanceof Error ? error.message : String(error)
    }
    await sleep(500)
  }
  throw new Error(`Timed out waiting for ${url}: ${lastError}`)
}

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms))
}

function renderProgress(results) {
  const latest = results.at(-1)
  if (!latest) return
  const status = latest.ok ? 'PASS' : 'FAIL'
  const duration = `${(latest.durationMs / 1000).toFixed(1)}s`
  console.log(`${status} ${latest.name} ${duration}`)
}

async function shutdownAll() {
  await Promise.all([...activeChildren].map((child) => terminateChild(child)))
}

async function terminateChild(child) {
  if (child.exitCode !== null || child.signalCode !== null) return
  child.kill('SIGTERM')
  await Promise.race([
    new Promise((resolve) => child.once('exit', resolve)),
    sleep(2_000),
  ])
  if (child.exitCode === null && child.signalCode === null) {
    child.kill('SIGKILL')
    await new Promise((resolve) => child.once('exit', resolve))
  }
}
