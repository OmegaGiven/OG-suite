import type { AppManifest } from '@og-suite/runtime'

export const feedManifest: AppManifest = {
  id: 'feed',
  name: 'Feed',
  route: '/feed',
  standaloneRoute: '/feed',
  capabilities: ['remoteSave'],
  toolbar: [],
}
