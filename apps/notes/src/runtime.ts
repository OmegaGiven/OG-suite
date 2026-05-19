import {
  createBrowserLocalCache,
  createBrowserSyncQueue,
  createHttpApiClient,
  createWebSocketDocumentUpdates,
  createWebSocketPresence,
} from '@og-suite/runtime'
import { loadStoredTokens } from '@og-suite/ui'
import type { RuntimeServices } from '@og-suite/runtime'

export function createStandaloneRuntime(): RuntimeServices {
  const apiHost = typeof window === 'undefined' || window.location.hostname === 'localhost' ? '127.0.0.1' : window.location.hostname
  const defaultApiUrl =
    typeof window === 'undefined' ? 'http://127.0.0.1:8080' : `http://${apiHost}:8080`
  const baseUrl = import.meta.env.VITE_OG_API_URL ?? defaultApiUrl
  const clientId = localStorage.getItem('og-suite:client-id') ?? crypto.randomUUID()
  localStorage.setItem('og-suite:client-id', clientId)
  return {
    api: createHttpApiClient(baseUrl),
    cache: createBrowserLocalCache('og-suite:notes:workspace'),
    syncQueue: createBrowserSyncQueue('og-suite:notes:sync-queue'),
    presence: createWebSocketPresence(baseUrl, clientId),
    documentUpdates: createWebSocketDocumentUpdates(baseUrl, clientId),
    tokens: loadStoredTokens(),
    clientId,
  }
}
