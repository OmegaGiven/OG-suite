export type IsoDateTime = string

export type AppCapability = 'offline' | 'remoteSave' | 'collaboration' | 'files' | 'media'

export type UserProfile = {
  id: string
  displayName: string
  roles: string[]
}

export type AppRegistryEntry = {
  id: string
  name: string
  route: string
  standaloneRoute: string
  capabilities: AppCapability[]
}

export type DesignTokens = {
  colorBackground: string
  colorBackgroundGradient: string
  backgroundGradients: BackgroundGradient[]
  backgroundImage: string
  backgroundImageOpacity: number
  panelOpacity: number
  colorSurface: string
  colorSurfaceSubtle: string
  colorSurfaceStrong: string
  colorToolBackground: string
  colorActionBarBackground: string
  colorText: string
  colorMuted: string
  colorAccent: string
  colorAccentSoft: string
  colorAccentBorder: string
  colorAccentContrast: string
  colorBorder: string
  colorNav: string
  shadow: string
  margin: number
  radius: number
  density: 'compact' | 'comfortable'
  fontFamily: string
  confirmDelete: boolean
}

export type BackgroundGradient = {
  id: string
  name: string
  strength: number
  points: BackgroundGradientPoint[]
}

export type BackgroundGradientPoint = {
  id: string
  color: string
  strength: number
  x: number
  y: number
  stop: number
}

export type Note = {
  id: string
  documentId: string
  title: string
  path: string
  tags: string[]
  ownerId: string
  workspaceId: string
  createdAt: IsoDateTime
  updatedAt: IsoDateTime
  deletedAt?: IsoDateTime | null
}

export type NoteFolder = {
  id: string
  path: string
  name: string
  ownerId: string
  workspaceId: string
  createdAt: IsoDateTime
  updatedAt: IsoDateTime
  deletedAt?: IsoDateTime | null
}

export type CrdtDocumentState = {
  id: string
  kind: 'note'
  snapshot: string
  updates: CrdtUpdate[]
  version: number
  compactedAt?: IsoDateTime | null
}

export type CrdtUpdate = {
  id: string
  documentId: string
  clientId: string
  sequence: number
  payload: string
  createdAt: IsoDateTime
  clientSchemaVersion?: number
}

export type PresencePeer = {
  clientId: string
  userId: string
  displayName: string
  cursor?: number | null
  color: string
  lastSeenAt: IsoDateTime
}

export type SyncCursorSet = {
  generatedAt: IsoDateTime
}

export type SyncTombstone = {
  entity: 'notes' | 'documents' | 'noteFolders'
  id: string
  deletedAt: IsoDateTime
}

export type SyncConflict = {
  entity: 'notes' | 'documents'
  id: string
  reason: string
}

export type SyncOperation =
  | { kind: 'create_note'; note: Note; document: CrdtDocumentState }
  | { kind: 'update_note_metadata'; note: Note }
  | { kind: 'delete_note'; id: string; deletedAt: IsoDateTime }
  | { kind: 'create_note_folder'; folder: NoteFolder }
  | { kind: 'delete_note_folder'; id: string; deletedAt: IsoDateTime }
  | { kind: 'append_document_update'; update: CrdtUpdate }

export type SyncEnvelope = {
  cursors: SyncCursorSet
  apps: AppRegistryEntry[]
  noteFolders: NoteFolder[]
  notes: Note[]
  documents: CrdtDocumentState[]
  documentUpdates: CrdtUpdate[]
  tombstones: SyncTombstone[]
  conflicts: SyncConflict[]
}

export type SyncBootstrapResponse = SyncEnvelope
export type SyncPullRequest = { cursors: SyncCursorSet }
export type SyncPullResponse = SyncEnvelope
export type SyncPushRequest = { operations: SyncOperation[] }
export type SyncPushResponse = SyncEnvelope & { acceptedOperationIds: string[] }

export type CreateNoteRequest = {
  title: string
  path?: string
  tags?: string[]
  initialText?: string
}

export type UpdateNoteMetadataRequest = {
  title?: string
  path?: string
  tags?: string[]
}

export type AppendDocumentUpdatesRequest = {
  updates: Omit<CrdtUpdate, 'id' | 'createdAt'>[]
}
