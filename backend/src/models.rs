use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthResponse {
    pub status: &'static str,
    pub timestamp: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum AppCapability {
    #[serde(rename = "offline")]
    Offline,
    #[serde(rename = "remoteSave")]
    RemoteSave,
    #[serde(rename = "collaboration")]
    Collaboration,
    #[serde(rename = "files")]
    Files,
    #[serde(rename = "media")]
    Media,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppRegistryEntry {
    pub id: String,
    pub name: String,
    pub route: String,
    pub standalone_route: String,
    pub capabilities: Vec<AppCapability>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Note {
    pub id: Uuid,
    pub document_id: Uuid,
    pub title: String,
    pub path: String,
    pub tags: Vec<String>,
    pub owner_id: String,
    pub workspace_id: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NoteFolder {
    pub id: Uuid,
    pub path: String,
    pub name: String,
    pub owner_id: String,
    pub workspace_id: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CrdtDocumentState {
    pub id: Uuid,
    pub kind: String,
    pub snapshot: String,
    pub updates: Vec<CrdtUpdate>,
    pub version: u64,
    pub compacted_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CrdtUpdate {
    pub id: Uuid,
    pub document_id: Uuid,
    pub client_id: String,
    pub sequence: u64,
    pub payload: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateNoteRequest {
    pub title: String,
    #[serde(default = "default_path")]
    pub path: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub initial_text: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct UpdateNoteMetadataRequest {
    pub title: Option<String>,
    pub path: Option<String>,
    pub tags: Option<Vec<String>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppendDocumentUpdatesRequest {
    pub updates: Vec<IncomingCrdtUpdate>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IncomingCrdtUpdate {
    pub document_id: Uuid,
    pub client_id: String,
    pub sequence: u64,
    pub payload: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncCursorSet {
    pub generated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncTombstone {
    pub entity: String,
    pub id: Uuid,
    pub deleted_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncConflict {
    pub entity: String,
    pub id: Uuid,
    pub reason: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", rename_all_fields = "camelCase")]
pub enum SyncOperation {
    CreateNote {
        note: Note,
        document: CrdtDocumentState,
    },
    UpdateNoteMetadata {
        note: Note,
    },
    DeleteNote {
        id: Uuid,
        deleted_at: DateTime<Utc>,
    },
    CreateNoteFolder {
        folder: NoteFolder,
    },
    DeleteNoteFolder {
        id: Uuid,
        deleted_at: DateTime<Utc>,
    },
    AppendDocumentUpdate {
        update: CrdtUpdate,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncEnvelope {
    pub cursors: SyncCursorSet,
    pub apps: Vec<AppRegistryEntry>,
    pub note_folders: Vec<NoteFolder>,
    pub notes: Vec<Note>,
    pub documents: Vec<CrdtDocumentState>,
    pub document_updates: Vec<CrdtUpdate>,
    pub tombstones: Vec<SyncTombstone>,
    pub conflicts: Vec<SyncConflict>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncPullRequest {
    pub cursors: SyncCursorSet,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncPushRequest {
    pub operations: Vec<SyncOperation>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncPushResponse {
    pub cursors: SyncCursorSet,
    pub apps: Vec<AppRegistryEntry>,
    pub note_folders: Vec<NoteFolder>,
    pub notes: Vec<Note>,
    pub documents: Vec<CrdtDocumentState>,
    pub document_updates: Vec<CrdtUpdate>,
    pub tombstones: Vec<SyncTombstone>,
    pub conflicts: Vec<SyncConflict>,
    pub accepted_operation_ids: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PresencePeer {
    pub client_id: String,
    pub user_id: String,
    pub display_name: String,
    pub cursor: Option<usize>,
    pub color: String,
    pub last_seen_at: DateTime<Utc>,
}

fn default_path() -> String {
    "/".to_string()
}
