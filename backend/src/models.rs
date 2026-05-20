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
pub struct UserProfile {
    pub id: String,
    pub display_name: String,
    pub username: Option<String>,
    pub roles: Vec<String>,
    pub must_change_password: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceProfile {
    pub id: String,
    pub name: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthSession {
    pub user: UserProfile,
    pub workspace: WorkspaceProfile,
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CurrentSession {
    pub user: UserProfile,
    pub workspace: WorkspaceProfile,
    pub expires_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisterProfileRequest {
    pub username: String,
    pub display_name: String,
    pub password: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RefreshSessionRequest {
    pub refresh_token: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompleteSetupRequest {
    pub username: String,
    pub display_name: String,
    pub password: String,
    pub confirm_password: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppToolScope {
    pub feed: bool,
    pub notes: bool,
    pub files: bool,
    pub audio: bool,
    pub admin: bool,
}

impl AppToolScope {
    pub fn admin() -> Self {
        Self {
            feed: true,
            notes: true,
            files: true,
            audio: true,
            admin: true,
        }
    }

    pub fn member() -> Self {
        Self {
            feed: true,
            notes: true,
            files: true,
            audio: true,
            admin: false,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminRolePolicy {
    pub name: String,
    pub app_scopes: AppToolScope,
    pub admin_panel: bool,
    pub manage_users: bool,
    pub manage_storage: bool,
    pub manage_auth: bool,
    pub manage_deployment: bool,
    pub manage_database: bool,
    pub view_audits: bool,
}

pub type CreateAdminRoleRequest = AdminRolePolicy;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminUserSummary {
    pub id: String,
    pub username: String,
    pub display_name: String,
    pub roles: Vec<String>,
    pub must_change_password: bool,
    pub storage_used_bytes: u64,
    pub storage_limit_mb: u64,
    pub app_scopes: AppToolScope,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateAdminUserRequest {
    pub username: String,
    pub display_name: String,
    pub password: String,
    pub roles: Vec<String>,
    pub storage_limit_mb: u64,
    pub app_scopes: AppToolScope,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateAdminUserAccessRequest {
    pub roles: Vec<String>,
    pub storage_limit_mb: u64,
    pub app_scopes: AppToolScope,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResetAdminUserPasswordRequest {
    pub password: String,
    pub confirm_password: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminStorageOverview {
    pub total_used_bytes: u64,
    pub total_limit_mb: u64,
    pub user_count: usize,
    pub notes_bytes: u64,
    pub audio_bytes: u64,
    pub files_bytes: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminDatabaseTable {
    pub key: String,
    pub label: String,
    pub row_count: usize,
    pub columns: Vec<String>,
    pub rows: Vec<serde_json::Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminDatabaseOverview {
    pub backend: String,
    pub generated_at: DateTime<Utc>,
    pub tables: Vec<AdminDatabaseTable>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminAuditEntry {
    pub id: String,
    pub occurred_at: DateTime<Utc>,
    pub actor_id: String,
    pub actor_label: String,
    pub action: String,
    pub target_kind: String,
    pub target_label: String,
    pub details: Option<serde_json::Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminAuthSettings {
    pub default_admin_enabled: bool,
    pub local_password_enabled: bool,
    pub require_setup_password_change: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminDeploymentSettings {
    pub server_version: String,
    pub build_date: String,
    pub api_compatibility_version: String,
    pub release_channel: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminSummary {
    pub generated_at: DateTime<Utc>,
    pub users: Vec<AdminUserSummary>,
    pub role_policies: Vec<AdminRolePolicy>,
    pub storage: AdminStorageOverview,
    pub authentication: AdminAuthSettings,
    pub deployment: AdminDeploymentSettings,
    pub database: AdminDatabaseOverview,
    pub audits: Vec<AdminAuditEntry>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemVersion {
    pub backend_version: String,
    pub api_compatibility_version: String,
    pub minimum_client_version: String,
    pub build_date: String,
    pub capabilities: Vec<String>,
    pub auth_required: bool,
    pub auth_modes: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FeedActivityAction {
    NoteCreated,
    NoteEdited,
    NoteMetadataUpdated,
    NoteDeleted,
    FolderCreated,
    FolderDeleted,
    DocumentEdited,
    FavoriteAdded,
    FavoriteRemoved,
    AudioRecordingCreated,
    AudioRecordingRenamed,
    AudioUploaded,
    AudioTranscriptQueued,
    AudioTranscriptGenerated,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeedActivityEvent {
    pub id: Uuid,
    pub app_id: String,
    pub action: FeedActivityAction,
    pub summary: String,
    pub target_kind: String,
    pub target_id: String,
    pub target_label: String,
    pub actor_id: String,
    pub actor_name: String,
    pub workspace_id: String,
    pub is_public: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeedFavorite {
    pub id: Uuid,
    pub target_kind: String,
    pub target_id: String,
    pub label: String,
    pub app_id: String,
    pub actor_id: String,
    pub workspace_id: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateFeedFavoriteRequest {
    pub target_kind: String,
    pub target_id: String,
    pub label: String,
    pub app_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioRecording {
    pub id: Uuid,
    pub title: String,
    pub path: String,
    pub mime_type: String,
    pub duration_ms: u64,
    pub size_bytes: u64,
    pub status: String,
    pub asset_ref: Option<String>,
    pub owner_id: String,
    pub workspace_id: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioFolder {
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
pub struct AudioTranscriptSegment {
    pub id: Uuid,
    pub recording_id: Uuid,
    pub channel: Option<u16>,
    pub speaker_label: Option<String>,
    pub start_ms: u64,
    pub end_ms: u64,
    pub text: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioTranscript {
    pub recording_id: Uuid,
    pub status: String,
    pub segments: Vec<AudioTranscriptSegment>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioTranscriptionStatus {
    pub recording_id: Uuid,
    pub status: String,
    pub engine: String,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateAudioRecordingRequest {
    pub title: String,
    #[serde(default = "default_path")]
    pub path: String,
    pub mime_type: String,
    pub duration_ms: u64,
    pub size_bytes: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UploadAudioRequest {
    pub data_url: String,
    pub mime_type: String,
    pub size_bytes: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct UpdateAudioRecordingRequest {
    pub title: Option<String>,
    pub path: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateAudioFolderRequest {
    pub path: String,
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
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
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
