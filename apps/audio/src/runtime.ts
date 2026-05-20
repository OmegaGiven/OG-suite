import {
  createBrowserLocalCache,
  createBrowserSyncQueue,
  createHttpApiClient,
  createRuntimeId,
  createWebSocketDocumentUpdates,
  createWebSocketPresence,
} from '@og-suite/runtime'
import type { RuntimeServices } from '@og-suite/runtime'
import { loadStoredTokens } from '@og-suite/ui'

export function createStandaloneRuntime(serverUrl?: string): RuntimeServices {
  const apiHost = typeof window === 'undefined' || window.location.hostname === 'localhost' ? '127.0.0.1' : window.location.hostname
  const defaultApiUrl =
    typeof window === 'undefined' ? 'http://127.0.0.1:8080' : `http://${apiHost}:8080`
  const storedServerUrl = typeof localStorage === 'undefined' ? null : localStorage.getItem('og-suite:server-url')
  const baseUrl = serverUrl ?? import.meta.env.VITE_OG_API_URL ?? storedServerUrl ?? defaultApiUrl
  const clientId = localStorage.getItem('og-suite:client-id') ?? createRuntimeId('client')
  localStorage.setItem('og-suite:client-id', clientId)
  return {
    api: createHttpApiClient(baseUrl, () => localStorage.getItem('og-suite:auth:access-token')),
    cache: createBrowserLocalCache('og-suite:audio:workspace'),
    syncQueue: createBrowserSyncQueue('og-suite:audio:sync-queue'),
    presence: createWebSocketPresence(baseUrl, clientId),
    documentUpdates: createWebSocketDocumentUpdates(baseUrl, clientId),
    tokens: loadStoredTokens(),
    clientId,
  }
}
