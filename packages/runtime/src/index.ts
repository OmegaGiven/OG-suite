import type {
  AppCapability,
  AppRegistryEntry,
  CrdtUpdate,
  DesignTokens,
  PresencePeer,
  SyncEnvelope,
  SyncOperation,
} from '@og-suite/contracts'

export type ToolbarItem =
  | { kind: 'button'; id: string; label: string; command: string }
  | { kind: 'dropdown'; id: string; label: string; options: { label: string; command: string }[] }

export type AppManifest = {
  id: string
  name: string
  route: string
  standaloneRoute: string
  capabilities: AppCapability[]
  toolbar: ToolbarItem[]
}

export type ApiClient = {
  get<T>(path: string): Promise<T>
  post<T>(path: string, body: unknown): Promise<T>
  patch<T>(path: string, body: unknown): Promise<T>
  delete(path: string): Promise<void>
}

export type LocalCache = {
  loadEnvelope(): Promise<SyncEnvelope | null>
  saveEnvelope(envelope: SyncEnvelope): Promise<void>
}

export type SyncQueue = {
  list(): Promise<QueuedOperation[]>
  enqueue(operation: SyncOperation): Promise<QueuedOperation>
  remove(ids: string[]): Promise<void>
}

export type PresenceChannel = {
  connect(documentId: string, onPeers: (peers: PresencePeer[]) => void): () => void
  publishCursor(documentId: string, cursor: number | null): void
}

export type DocumentUpdateChannel = {
  connect(documentId: string, onUpdate: (update: CrdtUpdate) => void): () => void
  publishUpdate(documentId: string, update: CrdtUpdate): boolean
}

export type RuntimeServices = {
  api: ApiClient
  cache: LocalCache
  syncQueue: SyncQueue
  presence: PresenceChannel
  documentUpdates: DocumentUpdateChannel
  tokens: DesignTokens
  clientId: string
}

export type RegisteredApp = {
  manifest: AppManifest
  registryEntry: AppRegistryEntry
}

export type QueuedOperation = {
  id: string
  operation: SyncOperation
  createdAt: string
}

export function registerApp(manifest: AppManifest): RegisteredApp {
  return {
    manifest,
    registryEntry: {
      id: manifest.id,
      name: manifest.name,
      route: manifest.route,
      standaloneRoute: manifest.standaloneRoute,
      capabilities: manifest.capabilities,
    },
  }
}

export function createHttpApiClient(baseUrl: string): ApiClient {
  async function request<T>(method: string, path: string, body?: unknown): Promise<T> {
    const response = await fetch(`${baseUrl}${path}`, {
      method,
      headers: body === undefined ? undefined : { 'content-type': 'application/json' },
      body: body === undefined ? undefined : JSON.stringify(body),
    })
    if (!response.ok) {
      throw new Error(`${method} ${path} failed with ${response.status}`)
    }
    if (response.status === 204) return undefined as T
    return (await response.json()) as T
  }
  return {
    get: (path) => request('GET', path),
    post: (path, body) => request('POST', path, body),
    patch: (path, body) => request('PATCH', path, body),
    delete: (path) => request('DELETE', path),
  }
}

export function createBrowserLocalCache(key = 'og-suite:workspace'): LocalCache {
  return {
    async loadEnvelope() {
      const raw = localStorage.getItem(key)
      return raw ? (JSON.parse(raw) as SyncEnvelope) : null
    },
    async saveEnvelope(envelope) {
      localStorage.setItem(key, JSON.stringify(envelope))
    },
  }
}

export function createBrowserSyncQueue(key = 'og-suite:sync-queue'): SyncQueue {
  function load(): QueuedOperation[] {
    const raw = localStorage.getItem(key)
    return raw ? (JSON.parse(raw) as QueuedOperation[]) : []
  }
  function save(items: QueuedOperation[]) {
    localStorage.setItem(key, JSON.stringify(items))
  }
  return {
    async list() {
      return load()
    },
    async enqueue(operation) {
      const queued = {
        id: crypto.randomUUID(),
        operation,
        createdAt: new Date().toISOString(),
      }
      save([...load(), queued])
      return queued
    },
    async remove(ids) {
      const idSet = new Set(ids)
      save(load().filter((item) => !idSet.has(item.id)))
    },
  }
}

export function createWebSocketPresence(baseUrl: string, clientId: string): PresenceChannel {
  const sockets = new Map<string, WebSocket>()
  return {
    connect(documentId, onPeers) {
      const url = `${baseUrl.replace(/^http/, 'ws')}/ws/presence/${documentId}?client_id=${encodeURIComponent(clientId)}`
      const socket = new WebSocket(url)
      sockets.set(documentId, socket)
      socket.addEventListener('message', (event) => {
        const payload = JSON.parse(String(event.data)) as { peers?: PresencePeer[] }
        onPeers(payload.peers ?? [])
      })
      return () => {
        sockets.delete(documentId)
        socket.close()
      }
    },
    publishCursor(documentId, cursor) {
      const socket = sockets.get(documentId)
      if (socket?.readyState === WebSocket.OPEN) {
        socket.send(JSON.stringify({ kind: 'cursor', cursor }))
      }
    },
  }
}

export function createWebSocketDocumentUpdates(baseUrl: string, clientId: string): DocumentUpdateChannel {
  const sockets = new Map<string, WebSocket>()
  const pending = new Map<string, CrdtUpdate[]>()

  function flushPending(documentId: string) {
    const socket = sockets.get(documentId)
    if (socket?.readyState !== WebSocket.OPEN) return false
    const updates = pending.get(documentId) ?? []
    for (const update of updates) socket.send(JSON.stringify(update))
    pending.delete(documentId)
    return true
  }

  return {
    connect(documentId, onUpdate) {
      const url = `${baseUrl.replace(/^http/, 'ws')}/ws/documents/${documentId}?client_id=${encodeURIComponent(clientId)}`
      const socket = new WebSocket(url)
      sockets.set(documentId, socket)
      socket.addEventListener('open', () => {
        flushPending(documentId)
      })
      socket.addEventListener('message', (event) => {
        const payload = JSON.parse(String(event.data)) as { update?: CrdtUpdate }
        if (payload.update && payload.update.clientId !== clientId) onUpdate(payload.update)
      })
      return () => {
        sockets.delete(documentId)
        socket.close()
      }
    },
    publishUpdate(documentId, update) {
      const socket = sockets.get(documentId)
      if (socket?.readyState !== WebSocket.OPEN) {
        pending.set(documentId, [...(pending.get(documentId) ?? []), update])
        return false
      }
      socket.send(JSON.stringify(update))
      return true
    },
  }
}
