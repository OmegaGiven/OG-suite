import type { AppManifest } from '@og-suite/runtime'

export const filesManifest: AppManifest = {
  id: 'files',
  name: 'Files',
  route: '/files',
  standaloneRoute: '/files',
  capabilities: ['offline', 'files'],
  toolbar: [],
}
