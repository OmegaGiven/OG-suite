import type { AppManifest } from '@og-suite/runtime'

export const audioManifest: AppManifest = {
  id: 'audio',
  name: 'Audio',
  route: '/audio',
  standaloneRoute: '/',
  capabilities: ['offline', 'remoteSave', 'media'],
  toolbar: [],
}
