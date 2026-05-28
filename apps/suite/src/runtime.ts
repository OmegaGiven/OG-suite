import {
  createHttpApiClient,
  createMemoryLocalCache,
  createMemorySyncQueue,
  createRuntimeId,
  createWebSocketDocumentUpdates,
  createWebSocketPresence,
} from '@og-suite/runtime'
import { loadStoredTokens } from '@og-suite/ui'
import type { RuntimeServices } from '@og-suite/runtime'

export function createSuiteRuntime(): RuntimeServices {
  const apiHost = typeof window === 'undefined' || window.location.hostname === 'localhost' ? '127.0.0.1' : window.location.hostname
  const defaultApiUrl =
    typeof window === 'undefined' ? 'http://127.0.0.1:8080' : `http://${apiHost}:8080`
  const baseUrl = import.meta.env.VITE_OG_API_URL ?? defaultApiUrl
  const clientId = localStorage.getItem('og-suite:client-id') ?? createRuntimeId('client')
  localStorage.setItem('og-suite:client-id', clientId)
  return {
    api: createHttpApiClient(baseUrl, () => localStorage.getItem('og-suite:auth:access-token')),
    cache: createMemoryLocalCache(),
    syncQueue: createMemorySyncQueue(),
    presence: createWebSocketPresence(baseUrl, clientId),
    documentUpdates: createWebSocketDocumentUpdates(baseUrl, clientId),
    tokens: loadStoredTokens(),
    clientId,
    runtimeMode: 'remote',
    serverUrl: baseUrl,
  }
}
