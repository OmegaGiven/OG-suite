use crate::{
    error::{AppError, AppResult},
    models::*,
};
use async_trait::async_trait;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf, sync::Arc};
use tokio::sync::RwLock;
use uuid::Uuid;

const COMPACT_AFTER_UPDATES: usize = 20;

#[async_trait]
pub trait SuiteRepository: Send + Sync {
    async fn apps(&self) -> AppResult<Vec<AppRegistryEntry>>;
    async fn note_folders(&self) -> AppResult<Vec<NoteFolder>>;
    async fn notes(&self) -> AppResult<Vec<Note>>;
    async fn create_note(&self, request: CreateNoteRequest) -> AppResult<Note>;
    async fn upsert_note(&self, note: Note) -> AppResult<Note>;
    async fn update_note_metadata(
        &self,
        id: Uuid,
        request: UpdateNoteMetadataRequest,
    ) -> AppResult<Note>;
    async fn delete_note(&self, id: Uuid) -> AppResult<()>;
    async fn document(&self, id: Uuid) -> AppResult<CrdtDocumentState>;
    async fn append_document_updates(
        &self,
        id: Uuid,
        updates: Vec<IncomingCrdtUpdate>,
    ) -> AppResult<CrdtDocumentState>;
    async fn append_document_update(&self, update: CrdtUpdate) -> AppResult<CrdtDocumentState>;
    async fn envelope(&self) -> AppResult<SyncEnvelope>;
    async fn apply_sync_operation(&self, operation: SyncOperation) -> AppResult<()>;
    async fn feed_events(&self) -> AppResult<Vec<FeedActivityEvent>>;
    async fn append_feed_event(&self, event: FeedActivityEvent) -> AppResult<FeedActivityEvent>;
    async fn feed_favorites(&self) -> AppResult<Vec<FeedFavorite>>;
    async fn create_feed_favorite(
        &self,
        request: CreateFeedFavoriteRequest,
    ) -> AppResult<FeedFavorite>;
    async fn delete_feed_favorite(&self, id: Uuid) -> AppResult<()>;
    async fn audio_recordings(&self) -> AppResult<Vec<AudioRecording>>;
    async fn audio_folders(&self) -> AppResult<Vec<AudioFolder>>;
    async fn create_audio_folder(
        &self,
        request: CreateAudioFolderRequest,
    ) -> AppResult<AudioFolder>;
    async fn delete_audio_folder(&self, id: Uuid) -> AppResult<()>;
    async fn create_audio_recording(
        &self,
        request: CreateAudioRecordingRequest,
    ) -> AppResult<AudioRecording>;
    async fn audio_recording(&self, id: Uuid) -> AppResult<AudioRecording>;
    async fn update_audio_recording(
        &self,
        id: Uuid,
        request: UpdateAudioRecordingRequest,
    ) -> AppResult<AudioRecording>;
    async fn upload_audio(
        &self,
        id: Uuid,
        request: UploadAudioRequest,
    ) -> AppResult<AudioRecording>;
    async fn audio_asset(&self, id: Uuid) -> AppResult<String>;
    async fn delete_audio_recording(&self, id: Uuid) -> AppResult<()>;
    async fn audio_transcript(&self, id: Uuid) -> AppResult<AudioTranscript>;
    async fn upsert_audio_transcript(
        &self,
        transcript: AudioTranscript,
    ) -> AppResult<AudioTranscript>;
    async fn update_audio_recording_status(
        &self,
        id: Uuid,
        status: &str,
    ) -> AppResult<AudioRecording>;
    async fn drive_files(&self) -> AppResult<Vec<DriveFile>>;
    async fn drive_folders(&self) -> AppResult<Vec<DriveFolder>>;
    async fn create_drive_folder(&self, request: CreateDriveFolderRequest)
        -> AppResult<DriveFolder>;
    async fn update_drive_folder(
        &self,
        id: Uuid,
        request: UpdateDriveFolderRequest,
    ) -> AppResult<DriveFolder>;
    async fn delete_drive_folder(&self, id: Uuid) -> AppResult<()>;
    async fn create_drive_file(&self, request: CreateDriveFileRequest) -> AppResult<DriveFile>;
    async fn update_drive_file(
        &self,
        id: Uuid,
        request: UpdateDriveFileRequest,
    ) -> AppResult<DriveFile>;
    async fn drive_file(&self, id: Uuid) -> AppResult<DriveFile>;
    async fn drive_asset(&self, id: Uuid) -> AppResult<String>;
    async fn delete_drive_file(&self, id: Uuid) -> AppResult<()>;
    async fn appearance_themes(
        &self,
        owner_id: &str,
        workspace_id: &str,
    ) -> AppResult<Vec<AppearanceTheme>>;
    async fn create_appearance_theme(
        &self,
        owner_id: &str,
        workspace_id: &str,
        request: CreateAppearanceThemeRequest,
    ) -> AppResult<AppearanceTheme>;
    async fn update_appearance_theme(
        &self,
        id: Uuid,
        actor_id: &str,
        request: UpdateAppearanceThemeRequest,
    ) -> AppResult<AppearanceTheme>;
    async fn delete_appearance_theme(&self, id: Uuid, actor_id: &str) -> AppResult<()>;
    async fn appearance_settings(
        &self,
        user_id: &str,
        workspace_id: &str,
    ) -> AppResult<Option<AppearanceSettings>>;
    async fn update_appearance_settings(
        &self,
        user_id: &str,
        workspace_id: &str,
        request: UpdateAppearanceSettingsRequest,
    ) -> AppResult<AppearanceSettings>;
}

#[derive(Clone, Default)]
pub struct InMemoryRepository {
    inner: Arc<RwLock<RepositoryData>>,
    data_path: Option<Arc<PathBuf>>,
}

#[derive(Default)]
struct RepositoryData {
    notes: HashMap<Uuid, Note>,
    note_folders: HashMap<Uuid, NoteFolder>,
    documents: HashMap<Uuid, CrdtDocumentState>,
    tombstones: HashMap<(String, Uuid), SyncTombstone>,
    feed_events: Vec<FeedActivityEvent>,
    feed_favorites: HashMap<Uuid, FeedFavorite>,
    audio_recordings: HashMap<Uuid, AudioRecording>,
    audio_folders: HashMap<Uuid, AudioFolder>,
    audio_assets: HashMap<Uuid, String>,
    audio_transcripts: HashMap<Uuid, AudioTranscript>,
    drive_files: HashMap<Uuid, DriveFile>,
    drive_folders: HashMap<Uuid, DriveFolder>,
    drive_assets: HashMap<Uuid, String>,
    appearance_themes: HashMap<Uuid, AppearanceTheme>,
    appearance_settings: HashMap<(String, String), AppearanceSettings>,
}

#[derive(Default, Deserialize, Serialize)]
struct RepositorySnapshot {
    notes: Vec<Note>,
    note_folders: Vec<NoteFolder>,
    documents: Vec<CrdtDocumentState>,
    tombstones: Vec<SyncTombstone>,
    feed_events: Vec<FeedActivityEvent>,
    feed_favorites: Vec<FeedFavorite>,
    audio_recordings: Vec<AudioRecording>,
    audio_folders: Vec<AudioFolder>,
    audio_assets: Vec<(Uuid, String)>,
    audio_transcripts: Vec<AudioTranscript>,
    drive_files: Vec<DriveFile>,
    drive_folders: Vec<DriveFolder>,
    drive_assets: Vec<(Uuid, String)>,
    appearance_themes: Vec<AppearanceTheme>,
    appearance_settings: Vec<AppearanceSettings>,
}

impl InMemoryRepository {
    pub fn new() -> Self {
        let data_path = repository_data_path();
        let data = data_path
            .as_ref()
            .and_then(|path| match load_repository_data(path) {
                Ok(data) => Some(data),
                Err(error) => {
                    tracing::warn!(path = %path.display(), error = %error, "failed to load repository snapshot");
                    None
                }
            })
            .unwrap_or_default();
        Self {
            inner: Arc::new(RwLock::new(data)),
            data_path: data_path.map(Arc::new),
        }
    }

    fn persist_snapshot(&self, data: &RepositoryData) -> AppResult<()> {
        let Some(path) = &self.data_path else {
            return Ok(());
        };
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|error| AppError::Database(error.to_string()))?;
        }
        let snapshot = RepositorySnapshot::from(data);
        let json = serde_json::to_string_pretty(&snapshot)
            .map_err(|error| AppError::Database(error.to_string()))?;
        let tmp_path = path.with_extension("json.tmp");
        std::fs::write(&tmp_path, json).map_err(|error| AppError::Database(error.to_string()))?;
        std::fs::rename(&tmp_path, path.as_ref())
            .map_err(|error| AppError::Database(error.to_string()))?;
        Ok(())
    }

    pub async fn admin_database_tables(&self) -> Vec<AdminDatabaseTable> {
        let data = self.inner.read().await;
        let active_document_ids = data
            .notes
            .values()
            .filter(|note| note.deleted_at.is_none())
            .map(|note| note.document_id)
            .collect::<std::collections::HashSet<_>>();
        let active_documents = data
            .documents
            .values()
            .filter(|document| active_document_ids.contains(&document.id))
            .cloned()
            .collect::<Vec<_>>();
        let crdt_update_rows = active_documents
            .iter()
            .flat_map(|document| document.updates.iter())
            .map(|update| {
                serde_json::json!({
                    "id": update.id,
                    "document_id": update.document_id,
                    "client_id": update.client_id,
                    "sequence": update.sequence,
                    "payload_bytes": update.payload.len(),
                    "created_at": update.created_at,
                })
            })
            .collect::<Vec<_>>();
        let transcript_segment_rows = data
            .audio_transcripts
            .values()
            .flat_map(|transcript| transcript.segments.iter())
            .map(|segment| {
                serde_json::json!({
                    "id": segment.id,
                    "recording_id": segment.recording_id,
                    "channel": segment.channel,
                    "speaker_label": segment.speaker_label,
                    "start_ms": segment.start_ms,
                    "end_ms": segment.end_ms,
                    "text": segment.text,
                })
            })
            .collect::<Vec<_>>();
        let table = |key: &str,
                     label: &str,
                     row_count: usize,
                     columns: &[&str],
                     rows: Vec<serde_json::Value>|
         -> AdminDatabaseTable {
            AdminDatabaseTable {
                key: key.to_string(),
                label: label.to_string(),
                row_count,
                columns: columns.iter().map(|column| column.to_string()).collect(),
                rows,
            }
        };

        vec![
            table(
                "app_registry",
                "App Registry",
                4,
                &["id", "name", "route", "standalone_route", "capabilities"],
                vec![
                    feed_registry_entry(),
                    notes_registry_entry(),
                    files_registry_entry(),
                    audio_registry_entry(),
                ]
                .into_iter()
                .map(|app| serde_json::to_value(app).unwrap_or_else(|_| serde_json::json!({})))
                .collect(),
            ),
            table(
                "note_folders",
                "Note Folders",
                data.note_folders.len(),
                &[
                    "id",
                    "path",
                    "name",
                    "owner_id",
                    "workspace_id",
                    "created_at",
                    "updated_at",
                    "deleted_at",
                ],
                data.note_folders
                    .values()
                    .map(|folder| {
                        serde_json::to_value(folder).unwrap_or_else(|_| serde_json::json!({}))
                    })
                    .collect(),
            ),
            table(
                "notes",
                "Notes",
                data.notes.len(),
                &[
                    "id",
                    "document_id",
                    "title",
                    "path",
                    "tags",
                    "owner_id",
                    "workspace_id",
                    "created_at",
                    "updated_at",
                    "deleted_at",
                ],
                data.notes
                    .values()
                    .map(|note| {
                        serde_json::to_value(note).unwrap_or_else(|_| serde_json::json!({}))
                    })
                    .collect(),
            ),
            table(
                "crdt_documents",
                "CRDT Documents",
                data.documents.len(),
                &[
                    "id",
                    "kind",
                    "snapshot_bytes",
                    "update_count",
                    "version",
                    "compacted_at",
                ],
                data.documents
                    .values()
                    .map(|document| {
                        serde_json::json!({
                            "id": document.id,
                            "kind": document.kind,
                            "snapshot_bytes": document.snapshot.len(),
                            "update_count": document.updates.len(),
                            "version": document.version,
                            "compacted_at": document.compacted_at,
                        })
                    })
                    .collect(),
            ),
            table(
                "crdt_updates",
                "CRDT Updates",
                crdt_update_rows.len(),
                &[
                    "id",
                    "document_id",
                    "client_id",
                    "sequence",
                    "payload_bytes",
                    "created_at",
                ],
                crdt_update_rows,
            ),
            table(
                "sync_tombstones",
                "Sync Tombstones",
                data.tombstones.len(),
                &["entity", "id", "deleted_at"],
                data.tombstones
                    .values()
                    .map(|tombstone| {
                        serde_json::to_value(tombstone).unwrap_or_else(|_| serde_json::json!({}))
                    })
                    .collect(),
            ),
            table(
                "feed_activity_events",
                "Feed Activity Events",
                data.feed_events.len(),
                &[
                    "id",
                    "app_id",
                    "action",
                    "summary",
                    "target_kind",
                    "target_id",
                    "target_label",
                    "actor_id",
                    "actor_name",
                    "workspace_id",
                    "is_public",
                    "created_at",
                ],
                data.feed_events
                    .iter()
                    .map(|event| {
                        serde_json::to_value(event).unwrap_or_else(|_| serde_json::json!({}))
                    })
                    .collect(),
            ),
            table(
                "feed_favorites",
                "Feed Favorites",
                data.feed_favorites.len(),
                &[
                    "id",
                    "target_kind",
                    "target_id",
                    "label",
                    "app_id",
                    "actor_id",
                    "workspace_id",
                    "created_at",
                ],
                data.feed_favorites
                    .values()
                    .map(|favorite| {
                        serde_json::to_value(favorite).unwrap_or_else(|_| serde_json::json!({}))
                    })
                    .collect(),
            ),
            table(
                "audio_folders",
                "Audio Folders",
                data.audio_folders.len(),
                &[
                    "id",
                    "path",
                    "name",
                    "owner_id",
                    "workspace_id",
                    "created_at",
                    "updated_at",
                    "deleted_at",
                ],
                data.audio_folders
                    .values()
                    .map(|folder| {
                        serde_json::to_value(folder).unwrap_or_else(|_| serde_json::json!({}))
                    })
                    .collect(),
            ),
            table(
                "audio_recordings",
                "Audio Recordings",
                data.audio_recordings.len(),
                &[
                    "id",
                    "title",
                    "path",
                    "mime_type",
                    "duration_ms",
                    "size_bytes",
                    "status",
                    "asset_ref",
                    "owner_id",
                    "workspace_id",
                    "created_at",
                    "updated_at",
                    "deleted_at",
                ],
                data.audio_recordings
                    .values()
                    .map(|recording| {
                        serde_json::to_value(recording).unwrap_or_else(|_| serde_json::json!({}))
                    })
                    .collect(),
            ),
            table(
                "audio_assets",
                "Audio Assets",
                data.audio_assets.len(),
                &["recording_id", "data_url_bytes"],
                data.audio_assets
                    .iter()
                    .map(|(recording_id, data_url)| {
                        serde_json::json!({
                            "recording_id": recording_id,
                            "data_url_bytes": data_url.len(),
                        })
                    })
                    .collect(),
            ),
            table(
                "audio_transcripts",
                "Audio Transcripts",
                data.audio_transcripts.len(),
                &["recording_id", "status", "segment_count", "updated_at"],
                data.audio_transcripts
                    .values()
                    .map(|transcript| {
                        serde_json::json!({
                            "recording_id": transcript.recording_id,
                            "status": transcript.status,
                            "segment_count": transcript.segments.len(),
                            "updated_at": transcript.updated_at,
                        })
                    })
                    .collect(),
            ),
            table(
                "audio_transcript_segments",
                "Audio Transcript Segments",
                transcript_segment_rows.len(),
                &[
                    "id",
                    "recording_id",
                    "channel",
                    "speaker_label",
                    "start_ms",
                    "end_ms",
                    "text",
                ],
                transcript_segment_rows,
            ),
            table(
                "drive_folders",
                "Drive Folders",
                data.drive_folders.len(),
                &[
                    "id",
                    "path",
                    "name",
                    "owner_id",
                    "workspace_id",
                    "created_at",
                    "updated_at",
                    "deleted_at",
                ],
                data.drive_folders
                    .values()
                    .map(|folder| {
                        serde_json::to_value(folder).unwrap_or_else(|_| serde_json::json!({}))
                    })
                    .collect(),
            ),
            table(
                "drive_files",
                "Drive Files",
                data.drive_files.len(),
                &[
                    "id",
                    "name",
                    "path",
                    "mime_type",
                    "size_bytes",
                    "owner_id",
                    "workspace_id",
                    "created_at",
                    "updated_at",
                    "deleted_at",
                ],
                data.drive_files
                    .values()
                    .map(|file| {
                        serde_json::to_value(file).unwrap_or_else(|_| serde_json::json!({}))
                    })
                    .collect(),
            ),
            table(
                "drive_assets",
                "Drive Assets",
                data.drive_assets.len(),
                &["file_id", "data_url_bytes"],
                data.drive_assets
                    .iter()
                    .map(|(file_id, data_url)| {
                        serde_json::json!({
                            "file_id": file_id,
                            "data_url_bytes": data_url.len(),
                        })
                    })
                    .collect(),
            ),
            table(
                "appearance_themes",
                "Appearance Themes",
                data.appearance_themes.len(),
                &[
                    "id",
                    "name",
                    "owner_id",
                    "workspace_id",
                    "is_shared",
                    "created_at",
                    "updated_at",
                ],
                data.appearance_themes
                    .values()
                    .map(|theme| {
                        serde_json::json!({
                            "id": theme.id,
                            "name": theme.name,
                            "owner_id": theme.owner_id,
                            "workspace_id": theme.workspace_id,
                            "is_shared": theme.is_shared,
                            "created_at": theme.created_at,
                            "updated_at": theme.updated_at,
                        })
                    })
                    .collect(),
            ),
            table(
                "appearance_settings",
                "Appearance Settings",
                data.appearance_settings.len(),
                &["user_id", "workspace_id", "tokens_bytes", "updated_at"],
                data.appearance_settings
                    .values()
                    .map(|settings| {
                        serde_json::json!({
                            "user_id": settings.user_id,
                            "workspace_id": settings.workspace_id,
                            "tokens_bytes": settings.tokens.to_string().len(),
                            "updated_at": settings.updated_at,
                        })
                    })
                    .collect(),
            ),
        ]
    }
}

impl From<&RepositoryData> for RepositorySnapshot {
    fn from(data: &RepositoryData) -> Self {
        Self {
            notes: data.notes.values().cloned().collect(),
            note_folders: data.note_folders.values().cloned().collect(),
            documents: data.documents.values().cloned().collect(),
            tombstones: data.tombstones.values().cloned().collect(),
            feed_events: data.feed_events.clone(),
            feed_favorites: data.feed_favorites.values().cloned().collect(),
            audio_recordings: data.audio_recordings.values().cloned().collect(),
            audio_folders: data.audio_folders.values().cloned().collect(),
            audio_assets: data
                .audio_assets
                .iter()
                .map(|(recording_id, data_url)| (*recording_id, data_url.clone()))
                .collect(),
            audio_transcripts: data.audio_transcripts.values().cloned().collect(),
            drive_files: data.drive_files.values().cloned().collect(),
            drive_folders: data.drive_folders.values().cloned().collect(),
            drive_assets: data
                .drive_assets
                .iter()
                .map(|(file_id, data_url)| (*file_id, data_url.clone()))
                .collect(),
            appearance_themes: data.appearance_themes.values().cloned().collect(),
            appearance_settings: data.appearance_settings.values().cloned().collect(),
        }
    }
}

impl From<RepositorySnapshot> for RepositoryData {
    fn from(snapshot: RepositorySnapshot) -> Self {
        Self {
            notes: snapshot
                .notes
                .into_iter()
                .map(|note| (note.id, note))
                .collect(),
            note_folders: snapshot
                .note_folders
                .into_iter()
                .map(|folder| (folder.id, folder))
                .collect(),
            documents: snapshot
                .documents
                .into_iter()
                .map(|document| (document.id, document))
                .collect(),
            tombstones: snapshot
                .tombstones
                .into_iter()
                .map(|tombstone| ((tombstone.entity.clone(), tombstone.id), tombstone))
                .collect(),
            feed_events: snapshot.feed_events,
            feed_favorites: snapshot
                .feed_favorites
                .into_iter()
                .map(|favorite| (favorite.id, favorite))
                .collect(),
            audio_recordings: snapshot
                .audio_recordings
                .into_iter()
                .map(|recording| (recording.id, recording))
                .collect(),
            audio_folders: snapshot
                .audio_folders
                .into_iter()
                .map(|folder| (folder.id, folder))
                .collect(),
            audio_assets: snapshot.audio_assets.into_iter().collect(),
            audio_transcripts: snapshot
                .audio_transcripts
                .into_iter()
                .map(|transcript| (transcript.recording_id, transcript))
                .collect(),
            drive_files: snapshot
                .drive_files
                .into_iter()
                .map(|file| (file.id, file))
                .collect(),
            drive_folders: snapshot
                .drive_folders
                .into_iter()
                .map(|folder| (folder.id, folder))
                .collect(),
            drive_assets: snapshot.drive_assets.into_iter().collect(),
            appearance_themes: snapshot
                .appearance_themes
                .into_iter()
                .map(|theme| (theme.id, theme))
                .collect(),
            appearance_settings: snapshot
                .appearance_settings
                .into_iter()
                .map(|settings| {
                    (
                        (settings.user_id.clone(), settings.workspace_id.clone()),
                        settings,
                    )
                })
                .collect(),
        }
    }
}

fn repository_data_path() -> Option<PathBuf> {
    if let Ok(path) = std::env::var("OG_SUITE_REPOSITORY_FILE") {
        let trimmed = path.trim();
        if !trimmed.is_empty() {
            return Some(PathBuf::from(trimmed));
        }
    }
    std::env::var("OG_SUITE_DATA_DIR")
        .ok()
        .map(|path| PathBuf::from(path).join("repository.json"))
}

fn load_repository_data(path: &PathBuf) -> AppResult<RepositoryData> {
    if !path.exists() {
        return Ok(RepositoryData::default());
    }
    let json =
        std::fs::read_to_string(path).map_err(|error| AppError::Database(error.to_string()))?;
    let snapshot = serde_json::from_str::<RepositorySnapshot>(&json)
        .map_err(|error| AppError::Database(error.to_string()))?;
    Ok(snapshot.into())
}

#[async_trait]
impl SuiteRepository for InMemoryRepository {
    async fn apps(&self) -> AppResult<Vec<AppRegistryEntry>> {
        Ok(vec![
            feed_registry_entry(),
            notes_registry_entry(),
            files_registry_entry(),
            audio_registry_entry(),
        ])
    }

    async fn note_folders(&self) -> AppResult<Vec<NoteFolder>> {
        let data = self.inner.read().await;
        Ok(data
            .note_folders
            .values()
            .cloned()
            .filter(|folder| folder.deleted_at.is_none())
            .collect())
    }

    async fn notes(&self) -> AppResult<Vec<Note>> {
        let data = self.inner.read().await;
        Ok(data
            .notes
            .values()
            .cloned()
            .filter(|note| note.deleted_at.is_none())
            .collect())
    }

    async fn create_note(&self, request: CreateNoteRequest) -> AppResult<Note> {
        let now = Utc::now();
        let document_id = Uuid::new_v4();
        let note = Note {
            id: Uuid::new_v4(),
            document_id,
            title: request.title,
            path: request.path,
            tags: request.tags,
            owner_id: "local-user".to_string(),
            workspace_id: "default".to_string(),
            created_at: now,
            updated_at: now,
            deleted_at: None,
        };
        let document = CrdtDocumentState {
            id: document_id,
            kind: "note".to_string(),
            snapshot: request.initial_text,
            updates: Vec::new(),
            version: 0,
            compacted_at: None,
        };
        let mut data = self.inner.write().await;
        data.documents.insert(document_id, document);
        data.notes.insert(note.id, note.clone());
        self.persist_snapshot(&data)?;
        Ok(note)
    }

    async fn upsert_note(&self, note: Note) -> AppResult<Note> {
        let mut data = self.inner.write().await;
        data.tombstones.remove(&("notes".to_string(), note.id));
        data.notes.insert(note.id, note.clone());
        self.persist_snapshot(&data)?;
        Ok(note)
    }

    async fn update_note_metadata(
        &self,
        id: Uuid,
        request: UpdateNoteMetadataRequest,
    ) -> AppResult<Note> {
        let mut data = self.inner.write().await;
        let note = data
            .notes
            .get_mut(&id)
            .ok_or(crate::error::AppError::NotFound)?;
        if let Some(title) = request.title {
            note.title = title;
        }
        if let Some(path) = request.path {
            note.path = path;
        }
        if let Some(tags) = request.tags {
            note.tags = tags;
        }
        note.updated_at = Utc::now();
        let note = note.clone();
        self.persist_snapshot(&data)?;
        Ok(note)
    }

    async fn delete_note(&self, id: Uuid) -> AppResult<()> {
        let mut data = self.inner.write().await;
        let deleted_at = Utc::now();
        if let Some(note) = data.notes.get_mut(&id) {
            note.deleted_at = Some(deleted_at);
        }
        data.tombstones.insert(
            ("notes".to_string(), id),
            SyncTombstone {
                entity: "notes".to_string(),
                id,
                deleted_at,
            },
        );
        self.persist_snapshot(&data)?;
        Ok(())
    }

    async fn document(&self, id: Uuid) -> AppResult<CrdtDocumentState> {
        let data = self.inner.read().await;
        data.documents
            .get(&id)
            .cloned()
            .ok_or(crate::error::AppError::NotFound)
    }

    async fn append_document_updates(
        &self,
        id: Uuid,
        updates: Vec<IncomingCrdtUpdate>,
    ) -> AppResult<CrdtDocumentState> {
        let mut document = self.document(id).await?;
        for incoming in updates {
            if document.updates.iter().any(|update| {
                update.client_id == incoming.client_id
                    && update.sequence == incoming.sequence
                    && update.payload == incoming.payload
            }) {
                continue;
            }
            document.updates.push(CrdtUpdate {
                id: Uuid::new_v4(),
                document_id: id,
                client_id: incoming.client_id,
                sequence: incoming.sequence,
                payload: incoming.payload,
                created_at: Utc::now(),
            });
            document.version += 1;
        }
        compact_if_needed(&mut document);
        let mut data = self.inner.write().await;
        data.documents.insert(id, document.clone());
        self.persist_snapshot(&data)?;
        Ok(document)
    }

    async fn append_document_update(&self, update: CrdtUpdate) -> AppResult<CrdtDocumentState> {
        let mut data = self.inner.write().await;
        let document =
            data.documents
                .entry(update.document_id)
                .or_insert_with(|| CrdtDocumentState {
                    id: update.document_id,
                    kind: "note".to_string(),
                    snapshot: String::new(),
                    updates: Vec::new(),
                    version: 0,
                    compacted_at: None,
                });
        if document
            .updates
            .iter()
            .any(|existing| existing.id == update.id)
        {
            return Ok(document.clone());
        }
        document.updates.push(update);
        document.version += 1;
        compact_if_needed(document);
        let document = document.clone();
        self.persist_snapshot(&data)?;
        Ok(document)
    }

    async fn envelope(&self) -> AppResult<SyncEnvelope> {
        let data = self.inner.read().await;
        let active_document_ids = data
            .notes
            .values()
            .filter(|note| note.deleted_at.is_none())
            .map(|note| note.document_id)
            .collect::<std::collections::HashSet<_>>();
        let documents = data
            .documents
            .values()
            .filter(|document| active_document_ids.contains(&document.id))
            .cloned()
            .collect::<Vec<_>>();
        Ok(SyncEnvelope {
            cursors: SyncCursorSet {
                generated_at: Utc::now(),
            },
            apps: vec![
                feed_registry_entry(),
                notes_registry_entry(),
                audio_registry_entry(),
            ],
            note_folders: data
                .note_folders
                .values()
                .cloned()
                .filter(|folder| folder.deleted_at.is_none())
                .collect(),
            notes: data
                .notes
                .values()
                .cloned()
                .filter(|note| note.deleted_at.is_none())
                .collect(),
            document_updates: documents
                .iter()
                .flat_map(|document| document.updates.iter().cloned())
                .collect(),
            documents,
            tombstones: data.tombstones.values().cloned().collect(),
            conflicts: Vec::new(),
        })
    }

    async fn apply_sync_operation(&self, operation: SyncOperation) -> AppResult<()> {
        match operation {
            SyncOperation::CreateNote { note, document } => {
                let mut data = self.inner.write().await;
                data.tombstones.remove(&("notes".to_string(), note.id));
                data.tombstones
                    .remove(&("documents".to_string(), document.id));
                data.documents
                    .entry(document.id)
                    .and_modify(|existing| merge_document_state(existing, document.clone()))
                    .or_insert(document);
                data.notes
                    .entry(note.id)
                    .and_modify(|existing| {
                        if note.updated_at >= existing.updated_at {
                            *existing = note.clone();
                        }
                    })
                    .or_insert(note);
                self.persist_snapshot(&data)?;
                Ok(())
            }
            SyncOperation::UpdateNoteMetadata { note } => self.upsert_note(note).await.map(|_| ()),
            SyncOperation::DeleteNote { id, deleted_at } => {
                let mut data = self.inner.write().await;
                if let Some(note) = data.notes.get_mut(&id) {
                    note.deleted_at = Some(deleted_at);
                }
                data.tombstones.insert(
                    ("notes".to_string(), id),
                    SyncTombstone {
                        entity: "notes".to_string(),
                        id,
                        deleted_at,
                    },
                );
                self.persist_snapshot(&data)?;
                Ok(())
            }
            SyncOperation::CreateNoteFolder { folder } => {
                let mut data = self.inner.write().await;
                data.tombstones
                    .remove(&("noteFolders".to_string(), folder.id));
                data.note_folders.insert(folder.id, folder);
                self.persist_snapshot(&data)?;
                Ok(())
            }
            SyncOperation::DeleteNoteFolder { id, deleted_at } => {
                let mut data = self.inner.write().await;
                if let Some(folder) = data.note_folders.get_mut(&id) {
                    folder.deleted_at = Some(deleted_at);
                }
                data.tombstones.insert(
                    ("noteFolders".to_string(), id),
                    SyncTombstone {
                        entity: "noteFolders".to_string(),
                        id,
                        deleted_at,
                    },
                );
                self.persist_snapshot(&data)?;
                Ok(())
            }
            SyncOperation::AppendDocumentUpdate { update } => {
                self.append_document_update(update).await.map(|_| ())
            }
        }
    }

    async fn feed_events(&self) -> AppResult<Vec<FeedActivityEvent>> {
        let data = self.inner.read().await;
        let mut events = data
            .feed_events
            .iter()
            .filter(|event| event.is_public)
            .cloned()
            .collect::<Vec<_>>();
        events.sort_by(|left, right| right.created_at.cmp(&left.created_at));
        Ok(events)
    }

    async fn append_feed_event(&self, event: FeedActivityEvent) -> AppResult<FeedActivityEvent> {
        let mut data = self.inner.write().await;
        data.feed_events.push(event.clone());
        self.persist_snapshot(&data)?;
        Ok(event)
    }

    async fn feed_favorites(&self) -> AppResult<Vec<FeedFavorite>> {
        let data = self.inner.read().await;
        let mut favorites = data.feed_favorites.values().cloned().collect::<Vec<_>>();
        favorites.sort_by(|left, right| right.created_at.cmp(&left.created_at));
        Ok(favorites)
    }

    async fn create_feed_favorite(
        &self,
        request: CreateFeedFavoriteRequest,
    ) -> AppResult<FeedFavorite> {
        let mut data = self.inner.write().await;
        if let Some(existing) = data
            .feed_favorites
            .values()
            .find(|favorite| {
                favorite.target_kind == request.target_kind
                    && favorite.target_id == request.target_id
            })
            .cloned()
        {
            return Ok(existing);
        }
        let favorite = FeedFavorite {
            id: Uuid::new_v4(),
            target_kind: request.target_kind,
            target_id: request.target_id,
            label: request.label,
            app_id: request.app_id,
            actor_id: "local-user".to_string(),
            workspace_id: "default".to_string(),
            created_at: Utc::now(),
        };
        data.feed_favorites.insert(favorite.id, favorite.clone());
        self.persist_snapshot(&data)?;
        Ok(favorite)
    }

    async fn delete_feed_favorite(&self, id: Uuid) -> AppResult<()> {
        let mut data = self.inner.write().await;
        data.feed_favorites.remove(&id);
        self.persist_snapshot(&data)?;
        Ok(())
    }

    async fn audio_recordings(&self) -> AppResult<Vec<AudioRecording>> {
        let data = self.inner.read().await;
        let mut recordings = data
            .audio_recordings
            .values()
            .filter(|recording| recording.deleted_at.is_none())
            .cloned()
            .collect::<Vec<_>>();
        recordings.sort_by(|left, right| right.created_at.cmp(&left.created_at));
        Ok(recordings)
    }

    async fn audio_folders(&self) -> AppResult<Vec<AudioFolder>> {
        let data = self.inner.read().await;
        let mut folders = data
            .audio_folders
            .values()
            .filter(|folder| folder.deleted_at.is_none())
            .cloned()
            .collect::<Vec<_>>();
        folders.sort_by(|left, right| left.path.cmp(&right.path));
        Ok(folders)
    }

    async fn create_audio_folder(
        &self,
        request: CreateAudioFolderRequest,
    ) -> AppResult<AudioFolder> {
        let path = normalize_folder_path(&request.path);
        let mut data = self.inner.write().await;
        if let Some(existing) = data
            .audio_folders
            .values()
            .find(|folder| {
                normalize_folder_path(&folder.path) == path && folder.deleted_at.is_none()
            })
            .cloned()
        {
            return Ok(existing);
        }
        let now = Utc::now();
        let folder = AudioFolder {
            id: Uuid::new_v4(),
            name: folder_name(&path),
            path,
            owner_id: "local-user".to_string(),
            workspace_id: "default".to_string(),
            created_at: now,
            updated_at: now,
            deleted_at: None,
        };
        data.audio_folders.insert(folder.id, folder.clone());
        self.persist_snapshot(&data)?;
        Ok(folder)
    }

    async fn delete_audio_folder(&self, id: Uuid) -> AppResult<()> {
        let mut data = self.inner.write().await;
        let deleted_at = Utc::now();
        if let Some(folder) = data.audio_folders.get_mut(&id) {
            folder.deleted_at = Some(deleted_at);
            folder.updated_at = deleted_at;
        }
        self.persist_snapshot(&data)?;
        Ok(())
    }

    async fn create_audio_recording(
        &self,
        request: CreateAudioRecordingRequest,
    ) -> AppResult<AudioRecording> {
        let now = Utc::now();
        let recording = AudioRecording {
            id: Uuid::new_v4(),
            title: request.title,
            path: normalize_folder_path(&request.path),
            mime_type: request.mime_type,
            duration_ms: request.duration_ms,
            size_bytes: request.size_bytes,
            status: "local".to_string(),
            asset_ref: None,
            owner_id: "local-user".to_string(),
            workspace_id: "default".to_string(),
            created_at: now,
            updated_at: now,
            deleted_at: None,
        };
        let mut data = self.inner.write().await;
        data.audio_recordings
            .insert(recording.id, recording.clone());
        self.persist_snapshot(&data)?;
        Ok(recording)
    }

    async fn audio_recording(&self, id: Uuid) -> AppResult<AudioRecording> {
        let data = self.inner.read().await;
        data.audio_recordings
            .get(&id)
            .cloned()
            .filter(|recording| recording.deleted_at.is_none())
            .ok_or(crate::error::AppError::NotFound)
    }

    async fn update_audio_recording(
        &self,
        id: Uuid,
        request: UpdateAudioRecordingRequest,
    ) -> AppResult<AudioRecording> {
        let mut data = self.inner.write().await;
        let recording = data
            .audio_recordings
            .get_mut(&id)
            .filter(|recording| recording.deleted_at.is_none())
            .ok_or(crate::error::AppError::NotFound)?;
        if let Some(title) = request.title {
            recording.title = title;
        }
        if let Some(path) = request.path {
            recording.path = normalize_folder_path(&path);
        }
        recording.updated_at = Utc::now();
        let recording = recording.clone();
        self.persist_snapshot(&data)?;
        Ok(recording)
    }

    async fn upload_audio(
        &self,
        id: Uuid,
        request: UploadAudioRequest,
    ) -> AppResult<AudioRecording> {
        let mut data = self.inner.write().await;
        data.audio_assets.insert(id, request.data_url);
        let now = Utc::now();
        let recording = data
            .audio_recordings
            .get_mut(&id)
            .ok_or(crate::error::AppError::NotFound)?;
        recording.mime_type = request.mime_type;
        recording.size_bytes = request.size_bytes;
        recording.status = "transcribing".to_string();
        recording.asset_ref = Some(format!("memory://audio/{id}"));
        recording.updated_at = now;
        let recording = recording.clone();
        data.audio_transcripts.insert(
            id,
            AudioTranscript {
                recording_id: id,
                status: "queued".to_string(),
                segments: vec![AudioTranscriptSegment {
                    id: Uuid::new_v4(),
                    recording_id: id,
                    channel: Some(1),
                    speaker_label: Some("Speaker 1".to_string()),
                    start_ms: 0,
                    end_ms: recording.duration_ms,
                    text: "Transcript queued. Backend transcription provider is not connected yet."
                        .to_string(),
                }],
                updated_at: now,
            },
        );
        self.persist_snapshot(&data)?;
        Ok(recording)
    }

    async fn audio_asset(&self, id: Uuid) -> AppResult<String> {
        let data = self.inner.read().await;
        data.audio_assets
            .get(&id)
            .cloned()
            .ok_or(crate::error::AppError::NotFound)
    }

    async fn delete_audio_recording(&self, id: Uuid) -> AppResult<()> {
        let mut data = self.inner.write().await;
        let deleted_at = Utc::now();
        if let Some(recording) = data.audio_recordings.get_mut(&id) {
            recording.deleted_at = Some(deleted_at);
            recording.updated_at = deleted_at;
        }
        data.audio_assets.remove(&id);
        data.audio_transcripts.remove(&id);
        data.tombstones.insert(
            ("audioRecordings".to_string(), id),
            SyncTombstone {
                entity: "audioRecordings".to_string(),
                id,
                deleted_at,
            },
        );
        self.persist_snapshot(&data)?;
        Ok(())
    }

    async fn audio_transcript(&self, id: Uuid) -> AppResult<AudioTranscript> {
        let data = self.inner.read().await;
        if let Some(transcript) = data.audio_transcripts.get(&id) {
            return Ok(transcript.clone());
        }
        if data.audio_recordings.contains_key(&id) {
            return Ok(AudioTranscript {
                recording_id: id,
                status: "queued".to_string(),
                segments: Vec::new(),
                updated_at: Utc::now(),
            });
        }
        Err(crate::error::AppError::NotFound)
    }

    async fn upsert_audio_transcript(
        &self,
        transcript: AudioTranscript,
    ) -> AppResult<AudioTranscript> {
        let mut data = self.inner.write().await;
        data.audio_transcripts
            .insert(transcript.recording_id, transcript.clone());
        if let Some(recording) = data.audio_recordings.get_mut(&transcript.recording_id) {
            recording.status = match transcript.status.as_str() {
                "ready" => "transcribed".to_string(),
                "failed" => "failed".to_string(),
                status => status.to_string(),
            };
            recording.updated_at = Utc::now();
        }
        self.persist_snapshot(&data)?;
        Ok(transcript)
    }

    async fn update_audio_recording_status(
        &self,
        id: Uuid,
        status: &str,
    ) -> AppResult<AudioRecording> {
        let mut data = self.inner.write().await;
        let recording = data
            .audio_recordings
            .get_mut(&id)
            .ok_or(crate::error::AppError::NotFound)?;
        recording.status = status.to_string();
        recording.updated_at = Utc::now();
        let recording = recording.clone();
        self.persist_snapshot(&data)?;
        Ok(recording)
    }

    async fn drive_files(&self) -> AppResult<Vec<DriveFile>> {
        let data = self.inner.read().await;
        let mut files = data
            .drive_files
            .values()
            .filter(|file| file.deleted_at.is_none())
            .cloned()
            .collect::<Vec<_>>();
        files.sort_by(|left, right| right.updated_at.cmp(&left.updated_at));
        Ok(files)
    }

    async fn drive_folders(&self) -> AppResult<Vec<DriveFolder>> {
        let data = self.inner.read().await;
        let mut folders = data
            .drive_folders
            .values()
            .filter(|folder| folder.deleted_at.is_none())
            .cloned()
            .collect::<Vec<_>>();
        folders.sort_by(|left, right| left.path.cmp(&right.path));
        Ok(folders)
    }

    async fn create_drive_folder(
        &self,
        request: CreateDriveFolderRequest,
    ) -> AppResult<DriveFolder> {
        let path = normalize_folder_path(&request.path);
        let mut data = self.inner.write().await;
        if let Some(existing) = data
            .drive_folders
            .values()
            .find(|folder| normalize_folder_path(&folder.path) == path && folder.deleted_at.is_none())
            .cloned()
        {
            return Ok(existing);
        }
        let now = Utc::now();
        let folder = DriveFolder {
            id: Uuid::new_v4(),
            name: folder_name(&path),
            path,
            owner_id: "local-user".to_string(),
            workspace_id: "default".to_string(),
            created_at: now,
            updated_at: now,
            deleted_at: None,
        };
        data.drive_folders.insert(folder.id, folder.clone());
        self.persist_snapshot(&data)?;
        Ok(folder)
    }

    async fn update_drive_folder(
        &self,
        id: Uuid,
        request: UpdateDriveFolderRequest,
    ) -> AppResult<DriveFolder> {
        let mut data = self.inner.write().await;
        let folder = data
            .drive_folders
            .get_mut(&id)
            .filter(|folder| folder.deleted_at.is_none())
            .ok_or(crate::error::AppError::NotFound)?;
        if let Some(path) = request.path {
            let next_path = normalize_folder_path(&path);
            folder.path = next_path.clone();
            folder.name = folder_name(&next_path);
        }
        folder.updated_at = Utc::now();
        let folder = folder.clone();
        self.persist_snapshot(&data)?;
        Ok(folder)
    }

    async fn delete_drive_folder(&self, id: Uuid) -> AppResult<()> {
        let mut data = self.inner.write().await;
        let deleted_at = Utc::now();
        if let Some(folder) = data.drive_folders.get_mut(&id) {
            folder.deleted_at = Some(deleted_at);
            folder.updated_at = deleted_at;
        }
        self.persist_snapshot(&data)?;
        Ok(())
    }

    async fn create_drive_file(&self, request: CreateDriveFileRequest) -> AppResult<DriveFile> {
        let now = Utc::now();
        let file = DriveFile {
            id: Uuid::new_v4(),
            name: request.name,
            path: normalize_folder_path(&request.path),
            mime_type: request.mime_type,
            size_bytes: request.size_bytes,
            owner_id: "local-user".to_string(),
            workspace_id: "default".to_string(),
            created_at: now,
            updated_at: now,
            deleted_at: None,
        };
        let mut data = self.inner.write().await;
        data.drive_assets.insert(file.id, request.data_url);
        data.drive_files.insert(file.id, file.clone());
        self.persist_snapshot(&data)?;
        Ok(file)
    }

    async fn update_drive_file(
        &self,
        id: Uuid,
        request: UpdateDriveFileRequest,
    ) -> AppResult<DriveFile> {
        let mut data = self.inner.write().await;
        let file = data
            .drive_files
            .get_mut(&id)
            .filter(|file| file.deleted_at.is_none())
            .ok_or(crate::error::AppError::NotFound)?;
        if let Some(name) = request.name {
            file.name = name;
        }
        if let Some(path) = request.path {
            file.path = normalize_folder_path(&path);
        }
        file.updated_at = Utc::now();
        let file = file.clone();
        self.persist_snapshot(&data)?;
        Ok(file)
    }

    async fn drive_file(&self, id: Uuid) -> AppResult<DriveFile> {
        let data = self.inner.read().await;
        data.drive_files
            .get(&id)
            .cloned()
            .filter(|file| file.deleted_at.is_none())
            .ok_or(crate::error::AppError::NotFound)
    }

    async fn drive_asset(&self, id: Uuid) -> AppResult<String> {
        let data = self.inner.read().await;
        data.drive_assets
            .get(&id)
            .cloned()
            .ok_or(crate::error::AppError::NotFound)
    }

    async fn delete_drive_file(&self, id: Uuid) -> AppResult<()> {
        let mut data = self.inner.write().await;
        let deleted_at = Utc::now();
        if let Some(file) = data.drive_files.get_mut(&id) {
            file.deleted_at = Some(deleted_at);
            file.updated_at = deleted_at;
        }
        data.drive_assets.remove(&id);
        self.persist_snapshot(&data)?;
        Ok(())
    }

    async fn appearance_themes(
        &self,
        owner_id: &str,
        workspace_id: &str,
    ) -> AppResult<Vec<AppearanceTheme>> {
        let data = self.inner.read().await;
        let mut themes = data
            .appearance_themes
            .values()
            .filter(|theme| {
                theme.workspace_id == workspace_id
                    && (theme.is_shared || theme.owner_id == owner_id)
            })
            .cloned()
            .collect::<Vec<_>>();
        themes.sort_by(|left, right| right.updated_at.cmp(&left.updated_at));
        Ok(themes)
    }

    async fn create_appearance_theme(
        &self,
        owner_id: &str,
        workspace_id: &str,
        request: CreateAppearanceThemeRequest,
    ) -> AppResult<AppearanceTheme> {
        let name = request.name.trim();
        if name.is_empty() {
            return Err(AppError::BadRequest("theme name is required".to_string()));
        }
        let now = Utc::now();
        let theme = AppearanceTheme {
            id: Uuid::new_v4(),
            name: name.to_string(),
            tokens: request.tokens,
            owner_id: owner_id.to_string(),
            workspace_id: workspace_id.to_string(),
            is_shared: request.is_shared,
            created_at: now,
            updated_at: now,
        };
        let mut data = self.inner.write().await;
        data.appearance_themes.insert(theme.id, theme.clone());
        self.persist_snapshot(&data)?;
        Ok(theme)
    }

    async fn update_appearance_theme(
        &self,
        id: Uuid,
        actor_id: &str,
        request: UpdateAppearanceThemeRequest,
    ) -> AppResult<AppearanceTheme> {
        let mut data = self.inner.write().await;
        let theme = data
            .appearance_themes
            .get_mut(&id)
            .ok_or(crate::error::AppError::NotFound)?;
        if theme.owner_id != actor_id {
            return Err(AppError::Unauthorized);
        }
        if let Some(name) = request.name {
            let trimmed = name.trim();
            if trimmed.is_empty() {
                return Err(AppError::BadRequest("theme name is required".to_string()));
            }
            theme.name = trimmed.to_string();
        }
        if let Some(tokens) = request.tokens {
            theme.tokens = tokens;
        }
        if let Some(is_shared) = request.is_shared {
            theme.is_shared = is_shared;
        }
        theme.updated_at = Utc::now();
        let theme = theme.clone();
        self.persist_snapshot(&data)?;
        Ok(theme)
    }

    async fn delete_appearance_theme(&self, id: Uuid, actor_id: &str) -> AppResult<()> {
        let mut data = self.inner.write().await;
        let Some(theme) = data.appearance_themes.get(&id) else {
            return Err(crate::error::AppError::NotFound);
        };
        if theme.owner_id != actor_id {
            return Err(AppError::Unauthorized);
        }
        data.appearance_themes.remove(&id);
        self.persist_snapshot(&data)?;
        Ok(())
    }

    async fn appearance_settings(
        &self,
        user_id: &str,
        workspace_id: &str,
    ) -> AppResult<Option<AppearanceSettings>> {
        let data = self.inner.read().await;
        Ok(data
            .appearance_settings
            .get(&(user_id.to_string(), workspace_id.to_string()))
            .cloned())
    }

    async fn update_appearance_settings(
        &self,
        user_id: &str,
        workspace_id: &str,
        request: UpdateAppearanceSettingsRequest,
    ) -> AppResult<AppearanceSettings> {
        let settings = AppearanceSettings {
            user_id: user_id.to_string(),
            workspace_id: workspace_id.to_string(),
            tokens: request.tokens,
            updated_at: Utc::now(),
        };
        let mut data = self.inner.write().await;
        data.appearance_settings.insert(
            (settings.user_id.clone(), settings.workspace_id.clone()),
            settings.clone(),
        );
        self.persist_snapshot(&data)?;
        Ok(settings)
    }
}

fn compact_if_needed(document: &mut CrdtDocumentState) {
    if document.updates.len() < COMPACT_AFTER_UPDATES {
        return;
    }
    document.updates.sort_by(|left, right| {
        left.sequence
            .cmp(&right.sequence)
            .then_with(|| left.client_id.cmp(&right.client_id))
            .then_with(|| left.created_at.cmp(&right.created_at))
    });
    // Yjs updates are not "last write wins" payloads. Until the server has a
    // Yjs-aware compactor, keep the full update log so concurrent edits remain
    // mergeable and no client contribution is discarded.
}

fn merge_document_state(existing: &mut CrdtDocumentState, incoming: CrdtDocumentState) {
    if existing.kind.is_empty() {
        existing.kind = incoming.kind;
    }
    let mut seen_update_ids = existing
        .updates
        .iter()
        .map(|update| update.id)
        .collect::<std::collections::HashSet<_>>();
    for update in incoming.updates {
        let duplicate_payload = existing.updates.iter().any(|existing_update| {
            existing_update.client_id == update.client_id
                && existing_update.sequence == update.sequence
                && existing_update.payload == update.payload
        });
        if seen_update_ids.contains(&update.id) || duplicate_payload {
            continue;
        }
        seen_update_ids.insert(update.id);
        existing.updates.push(update);
        existing.version += 1;
    }
    existing.version = existing.version.max(incoming.version);
    existing.compacted_at = existing.compacted_at.or(incoming.compacted_at);
    compact_if_needed(existing);
}

fn notes_registry_entry() -> AppRegistryEntry {
    AppRegistryEntry {
        id: "notes".to_string(),
        name: "Notes".to_string(),
        route: "/notes".to_string(),
        standalone_route: "/".to_string(),
        capabilities: vec![
            AppCapability::Offline,
            AppCapability::RemoteSave,
            AppCapability::Collaboration,
        ],
    }
}

fn files_registry_entry() -> AppRegistryEntry {
    AppRegistryEntry {
        id: "files".to_string(),
        name: "Files".to_string(),
        route: "/files".to_string(),
        standalone_route: "/files".to_string(),
        capabilities: vec![AppCapability::Files, AppCapability::RemoteSave],
    }
}

fn feed_registry_entry() -> AppRegistryEntry {
    AppRegistryEntry {
        id: "feed".to_string(),
        name: "Feed".to_string(),
        route: "/feed".to_string(),
        standalone_route: "/feed".to_string(),
        capabilities: vec![AppCapability::RemoteSave],
    }
}

fn audio_registry_entry() -> AppRegistryEntry {
    AppRegistryEntry {
        id: "audio".to_string(),
        name: "Audio".to_string(),
        route: "/audio".to_string(),
        standalone_route: "/".to_string(),
        capabilities: vec![
            AppCapability::Offline,
            AppCapability::RemoteSave,
            AppCapability::Media,
        ],
    }
}

fn normalize_folder_path(path: &str) -> String {
    let trimmed = path.trim().trim_matches('/');
    if trimmed.is_empty() {
        "/".to_string()
    } else {
        format!("/{trimmed}")
    }
}

fn folder_name(path: &str) -> String {
    let normalized = normalize_folder_path(path);
    if normalized == "/" {
        "Root".to_string()
    } else {
        normalized
            .split('/')
            .filter(|part| !part.is_empty())
            .next_back()
            .unwrap_or("Folder")
            .to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn notes_crud_and_document_updates_work() {
        let repo = InMemoryRepository::new();
        let note = repo
            .create_note(CreateNoteRequest {
                title: "First".to_string(),
                path: "/".to_string(),
                tags: vec![],
                initial_text: "hello".to_string(),
            })
            .await
            .unwrap();
        repo.update_note_metadata(
            note.id,
            UpdateNoteMetadataRequest {
                title: Some("Renamed".to_string()),
                path: None,
                tags: None,
            },
        )
        .await
        .unwrap();
        let document = repo
            .append_document_updates(
                note.document_id,
                vec![IncomingCrdtUpdate {
                    document_id: note.document_id,
                    client_id: "a".to_string(),
                    sequence: 1,
                    payload: "hello world".to_string(),
                }],
            )
            .await
            .unwrap();
        assert_eq!(document.version, 1);
        assert_eq!(repo.notes().await.unwrap()[0].title, "Renamed");
    }

    #[tokio::test]
    async fn snapshot_keeps_crdt_update_log_after_threshold() {
        let repo = InMemoryRepository::new();
        let note = repo
            .create_note(CreateNoteRequest {
                title: "First".to_string(),
                path: "/".to_string(),
                tags: vec![],
                initial_text: String::new(),
            })
            .await
            .unwrap();
        for sequence in 1..=20 {
            repo.append_document_updates(
                note.document_id,
                vec![IncomingCrdtUpdate {
                    document_id: note.document_id,
                    client_id: "a".to_string(),
                    sequence,
                    payload: format!("v{sequence}"),
                }],
            )
            .await
            .unwrap();
        }
        let document = repo.document(note.document_id).await.unwrap();
        assert_eq!(document.snapshot, "");
        assert_eq!(document.updates.len(), 20);
        assert!(document.compacted_at.is_none());
    }

    #[tokio::test]
    async fn delayed_create_note_sync_does_not_overwrite_existing_document_updates() {
        let repo = InMemoryRepository::new();
        let note = repo
            .create_note(CreateNoteRequest {
                title: "Phone draft".to_string(),
                path: "/".to_string(),
                tags: vec![],
                initial_text: String::new(),
            })
            .await
            .unwrap();
        let stale_document = repo.document(note.document_id).await.unwrap();
        let server_update = CrdtUpdate {
            id: Uuid::new_v4(),
            document_id: note.document_id,
            client_id: "browser".to_string(),
            sequence: 1,
            payload: "server-update".to_string(),
            created_at: Utc::now(),
        };
        repo.append_document_update(server_update.clone())
            .await
            .unwrap();

        repo.apply_sync_operation(SyncOperation::CreateNote {
            note,
            document: stale_document,
        })
        .await
        .unwrap();

        let document = repo.document(server_update.document_id).await.unwrap();
        assert_eq!(document.updates.len(), 1);
        assert_eq!(document.updates[0].id, server_update.id);
    }

    #[tokio::test]
    async fn same_client_sequence_with_new_payload_is_not_dropped() {
        let repo = InMemoryRepository::new();
        let note = repo
            .create_note(CreateNoteRequest {
                title: "Sequence restart".to_string(),
                path: "/".to_string(),
                tags: vec![],
                initial_text: String::new(),
            })
            .await
            .unwrap();

        repo.append_document_updates(
            note.document_id,
            vec![IncomingCrdtUpdate {
                document_id: note.document_id,
                client_id: "phone".to_string(),
                sequence: 1,
                payload: "old-local-edit".to_string(),
            }],
        )
        .await
        .unwrap();
        repo.append_document_updates(
            note.document_id,
            vec![IncomingCrdtUpdate {
                document_id: note.document_id,
                client_id: "phone".to_string(),
                sequence: 1,
                payload: "new-local-edit-after-restart".to_string(),
            }],
        )
        .await
        .unwrap();

        let document = repo.document(note.document_id).await.unwrap();
        assert_eq!(document.updates.len(), 2);
        assert!(
            document
                .updates
                .iter()
                .any(|update| update.payload == "old-local-edit")
        );
        assert!(
            document
                .updates
                .iter()
                .any(|update| update.payload == "new-local-edit-after-restart")
        );
    }

    #[tokio::test]
    async fn appearance_settings_are_stored_per_user_workspace() {
        let repo = InMemoryRepository::new();
        let saved = repo
            .update_appearance_settings(
                "user-a",
                "workspace-a",
                UpdateAppearanceSettingsRequest {
                    tokens: serde_json::json!({ "colorBackground": "#111111" }),
                },
            )
            .await
            .unwrap();

        let loaded = repo
            .appearance_settings("user-a", "workspace-a")
            .await
            .unwrap()
            .unwrap();
        let missing = repo
            .appearance_settings("user-a", "workspace-b")
            .await
            .unwrap();

        assert_eq!(loaded.tokens, saved.tokens);
        assert!(missing.is_none());
    }

    #[tokio::test]
    async fn delayed_create_note_sync_keeps_same_sequence_new_payload() {
        let repo = InMemoryRepository::new();
        let note = repo
            .create_note(CreateNoteRequest {
                title: "Manual backup".to_string(),
                path: "/".to_string(),
                tags: vec![],
                initial_text: String::new(),
            })
            .await
            .unwrap();
        let mut local_document = repo.document(note.document_id).await.unwrap();
        let server_update = CrdtUpdate {
            id: Uuid::new_v4(),
            document_id: note.document_id,
            client_id: "phone".to_string(),
            sequence: 1,
            payload: "server-old".to_string(),
            created_at: Utc::now(),
        };
        let local_update = CrdtUpdate {
            id: Uuid::new_v4(),
            document_id: note.document_id,
            client_id: "phone".to_string(),
            sequence: 1,
            payload: "phone-new-after-restart".to_string(),
            created_at: Utc::now(),
        };
        local_document.updates.push(local_update.clone());

        repo.append_document_update(server_update.clone())
            .await
            .unwrap();
        repo.apply_sync_operation(SyncOperation::CreateNote {
            note,
            document: local_document,
        })
        .await
        .unwrap();

        let document = repo.document(server_update.document_id).await.unwrap();
        assert_eq!(document.updates.len(), 2);
        assert!(
            document
                .updates
                .iter()
                .any(|update| update.id == server_update.id)
        );
        assert!(
            document
                .updates
                .iter()
                .any(|update| update.id == local_update.id)
        );
    }
}
