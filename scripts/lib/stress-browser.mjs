import fs from 'node:fs'

const bravePath = '/Applications/Brave Browser.app/Contents/MacOS/Brave Browser'

export function createStressBrowserLaunchOptions() {
  if (process.env.OG_STRESS_BROWSER_EXECUTABLE) {
    return {
      executablePath: process.env.OG_STRESS_BROWSER_EXECUTABLE,
      headless: process.env.OG_STRESS_HEADLESS !== 'false',
    }
  }

  if (fs.existsSync(bravePath)) {
    return {
      executablePath: bravePath,
      headless: process.env.OG_STRESS_HEADLESS !== 'false',
    }
  }

  return { headless: process.env.OG_STRESS_HEADLESS !== 'false' }
}
