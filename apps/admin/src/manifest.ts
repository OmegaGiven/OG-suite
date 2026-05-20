import type { AppManifest } from '@og-suite/runtime'

export const adminManifest: AppManifest = {
  id: 'admin',
  name: 'Admin',
  route: '/admin',
  standaloneRoute: '/admin',
  capabilities: ['remoteSave'],
  toolbar: [],
}
