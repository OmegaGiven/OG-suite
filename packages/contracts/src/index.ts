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

export type FeedActivityAction =
  | 'note_created'
  | 'note_edited'
  | 'note_metadata_updated'
  | 'note_deleted'
  | 'folder_created'
  | 'folder_deleted'
  | 'document_edited'
  | 'favorite_added'
  | 'favorite_removed'
  | 'audio_recording_created'
  | 'audio_recording_renamed'
  | 'audio_uploaded'
  | 'audio_transcript_queued'
  | 'audio_transcript_generated'

export type FeedActivityEvent = {
  id: string
  appId: string
  action: FeedActivityAction
  summary: string
  targetKind: 'note' | 'folder' | 'document' | 'tool' | 'settings' | 'recording'
  targetId: string
  targetLabel: string
  actorId: string
  actorName: string
  workspaceId: string
  isPublic: boolean
  createdAt: IsoDateTime
}

export type FeedFavorite = {
  id: string
  targetKind: 'note' | 'folder' | 'document' | 'tool' | 'recording'
  targetId: string
  label: string
  appId: string
  actorId: string
  workspaceId: string
  createdAt: IsoDateTime
}

export type CreateFeedFavoriteRequest = {
  targetKind: FeedFavorite['targetKind']
  targetId: string
  label: string
  appId: string
}

export type AudioRecordingStatus = 'local' | 'uploading' | 'uploaded' | 'transcribing' | 'transcribed' | 'failed'

export type AudioRecording = {
  id: string
  title: string
  path: string
  mimeType: string
  durationMs: number
  sizeBytes: number
  status: AudioRecordingStatus
  assetRef?: string | null
  ownerId: string
  workspaceId: string
  createdAt: IsoDateTime
  updatedAt: IsoDateTime
  deletedAt?: IsoDateTime | null
}

export type AudioFolder = {
  id: string
  path: string
  name: string
  ownerId: string
  workspaceId: string
  createdAt: IsoDateTime
  updatedAt: IsoDateTime
  deletedAt?: IsoDateTime | null
}

export type AudioTranscriptSegment = {
  id: string
  recordingId: string
  channel?: number | null
  speakerLabel?: string | null
  startMs: number
  endMs: number
  text: string
}

export type AudioTranscript = {
  recordingId: string
  status: 'queued' | 'processing' | 'ready' | 'failed'
  segments: AudioTranscriptSegment[]
  updatedAt: IsoDateTime
}

export type AudioTranscriptionStatus = {
  recordingId: string
  status: AudioTranscript['status']
  engine: 'disabled' | 'command' | 'whisper_cpp' | string
  updatedAt: IsoDateTime
}

export type CreateAudioRecordingRequest = {
  title: string
  path?: string
  mimeType: string
  durationMs: number
  sizeBytes: number
}

export type UploadAudioRequest = {
  dataUrl: string
  mimeType: string
  sizeBytes: number
}

export type UpdateAudioRecordingRequest = {
  title?: string
  path?: string
}

export type CreateAudioFolderRequest = {
  path: string
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
  entity: 'notes' | 'documents' | 'noteFolders' | 'audioRecordings'
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
