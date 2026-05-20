import { svelte } from '@sveltejs/vite-plugin-svelte'
import { fileURLToPath, URL } from 'node:url'
import { defineConfig } from 'vite'

const alias = [
  { find: '@og-suite/ui/ActionBar', replacement: fileURLToPath(new URL('../../packages/ui/src/ActionBar.svelte', import.meta.url)) },
  { find: '@og-suite/ui/ActionButton', replacement: fileURLToPath(new URL('../../packages/ui/src/ActionButton.svelte', import.meta.url)) },
  { find: '@og-suite/ui/Icon', replacement: fileURLToPath(new URL('../../packages/ui/src/Icon.svelte', import.meta.url)) },
  { find: '@og-suite/contracts', replacement: fileURLToPath(new URL('../../packages/contracts/src/index.ts', import.meta.url)) },
  { find: '@og-suite/runtime', replacement: fileURLToPath(new URL('../../packages/runtime/src/index.ts', import.meta.url)) },
  { find: '@og-suite/ui/FileNavigator', replacement: fileURLToPath(new URL('../../packages/ui/src/FileNavigator.svelte', import.meta.url)) },
  { find: '@og-suite/ui/MobileSuiteMenu', replacement: fileURLToPath(new URL('../../packages/ui/src/MobileSuiteMenu.svelte', import.meta.url)) },
  { find: '@og-suite/ui/MobileSuiteTopBar', replacement: fileURLToPath(new URL('../../packages/ui/src/MobileSuiteTopBar.svelte', import.meta.url)) },
  { find: '@og-suite/ui', replacement: fileURLToPath(new URL('../../packages/ui/src/index.ts', import.meta.url)) },
]

export default defineConfig({
  plugins: [svelte()],
  resolve: { alias },
})
