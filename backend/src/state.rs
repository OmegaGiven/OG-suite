use crate::{
    error::{AppError, AppResult},
    models::{
        AdminAuditEntry, AdminAuthSettings, AdminDatabaseOverview, AdminDatabaseTable,
        AdminDeploymentSettings, AdminRolePolicy, AdminStorageOverview, AdminSummary,
        AdminUserSummary, AppToolScope, AuthSession, CompleteSetupRequest, CrdtUpdate,
        CreateAdminRoleRequest, CreateAdminUserRequest, CurrentSession, LoginRequest, PresencePeer,
        RefreshSessionRequest, RegisterProfileRequest, ResetAdminUserPasswordRequest,
        UpdateAdminUserAccessRequest, UserProfile, WorkspaceProfile,
    },
    repository::InMemoryRepository,
    transcription::LocalTranscriptionEngine,
};
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::HashMap, path::PathBuf, sync::Arc};
use tokio::sync::{RwLock, broadcast};
use uuid::Uuid;

#[derive(Clone)]
pub struct AppState {
    pub repo: InMemoryRepository,
    pub transcription: LocalTranscriptionEngine,
    presence: Arc<RwLock<HashMap<String, PresenceRoom>>>,
    document_updates: Arc<RwLock<HashMap<String, DocumentUpdateRoom>>>,
    auth: Arc<RwLock<AuthStore>>,
    auth_path: Option<Arc<PathBuf>>,
}

#[derive(Default)]
struct AuthStore {
    accounts: HashMap<String, AccountRecord>,
    sessions: HashMap<String, SessionRecord>,
    refresh_tokens: HashMap<String, String>,
    audits: Vec<AdminAuditEntry>,
    custom_role_policies: Vec<AdminRolePolicy>,
}

#[derive(Clone, Deserialize, Serialize)]
struct AccountRecord {
    user: UserProfile,
    workspace: WorkspaceProfile,
    password_hash: String,
    storage_limit_mb: u64,
    app_scopes: AppToolScope,
    created_at: chrono::DateTime<Utc>,
    updated_at: chrono::DateTime<Utc>,
}

#[derive(Default, Deserialize, Serialize)]
struct AuthSnapshot {
    accounts: Vec<AccountRecord>,
    audits: Vec<AdminAuditEntry>,
    custom_role_policies: Vec<AdminRolePolicy>,
}

#[derive(Clone)]
struct SessionRecord {
    user: UserProfile,
    workspace: WorkspaceProfile,
    refresh_token: String,
    expires_at: chrono::DateTime<Utc>,
}

struct PresenceRoom {
    peers: HashMap<String, PresencePeer>,
    tx: broadcast::Sender<Vec<PresencePeer>>,
}

struct DocumentUpdateRoom {
    tx: broadcast::Sender<CrdtUpdate>,
}

impl AppState {
    pub fn new() -> Self {
        Self::with_transcription(LocalTranscriptionEngine::from_env())
    }

    pub fn with_transcription(transcription: LocalTranscriptionEngine) -> Self {
        let auth_path = auth_data_path();
        let mut auth = auth_path
            .as_ref()
            .and_then(|path| match load_auth_store(path) {
                Ok(auth) => Some(auth),
                Err(error) => {
                    tracing::warn!(path = %path.display(), error = %error, "failed to load auth snapshot");
                    None
                }
            })
            .unwrap_or_default();
        if auth.accounts.is_empty() {
            seed_default_admin(&mut auth);
        }
        Self {
            repo: InMemoryRepository::new(),
            transcription,
            presence: Arc::new(RwLock::new(HashMap::new())),
            document_updates: Arc::new(RwLock::new(HashMap::new())),
            auth: Arc::new(RwLock::new(auth)),
            auth_path: auth_path.map(Arc::new),
        }
    }

    fn persist_auth(&self, auth: &AuthStore) -> AppResult<()> {
        let Some(path) = &self.auth_path else {
            return Ok(());
        };
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|error| AppError::Database(error.to_string()))?;
        }
        let snapshot = AuthSnapshot::from(auth);
        let json = serde_json::to_string_pretty(&snapshot)
            .map_err(|error| AppError::Database(error.to_string()))?;
        let tmp_path = path.with_extension("json.tmp");
        std::fs::write(&tmp_path, json).map_err(|error| AppError::Database(error.to_string()))?;
        std::fs::rename(&tmp_path, path.as_ref())
            .map_err(|error| AppError::Database(error.to_string()))?;
        Ok(())
    }

    pub async fn register_profile(
        &self,
        request: RegisterProfileRequest,
    ) -> AppResult<AuthSession> {
        let username = normalize_username(&request.username)?;
        if request.password.len() < 8 {
            return Err(AppError::BadRequest(
                "password must be at least 8 characters".to_string(),
            ));
        }
        let display_name = request.display_name.trim();
        if display_name.is_empty() {
            return Err(AppError::BadRequest("display name is required".to_string()));
        }

        let mut auth = self.auth.write().await;
        if auth.accounts.contains_key(&username) {
            return Err(AppError::BadRequest("profile already exists".to_string()));
        }

        let user = UserProfile {
            id: Uuid::new_v4().to_string(),
            display_name: display_name.to_string(),
            username: Some(username.clone()),
            roles: vec!["owner".to_string()],
            must_change_password: false,
        };
        let workspace = WorkspaceProfile {
            id: Uuid::new_v4().to_string(),
            name: format!("{}'s Workspace", display_name),
        };
        let account = AccountRecord {
            user: user.clone(),
            workspace: workspace.clone(),
            password_hash: password_hash(&username, &request.password),
            storage_limit_mb: 2048,
            app_scopes: AppToolScope::member(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        auth.accounts.insert(username, account);
        let session = create_session(&mut auth, user, workspace);
        self.persist_auth(&auth)?;
        Ok(session)
    }

    pub async fn login(&self, request: LoginRequest) -> AppResult<AuthSession> {
        let username = normalize_username(&request.username)?;
        let mut auth = self.auth.write().await;
        let Some(account) = auth.accounts.get(&username).cloned() else {
            return Err(AppError::Unauthorized);
        };
        if account.password_hash != password_hash(&username, &request.password) {
            return Err(AppError::Unauthorized);
        }
        Ok(create_session(&mut auth, account.user, account.workspace))
    }

    pub async fn complete_setup(
        &self,
        access_token: &str,
        request: CompleteSetupRequest,
    ) -> AppResult<CurrentSession> {
        let next_username = normalize_username(&request.username)?;
        if request.password != request.confirm_password {
            return Err(AppError::BadRequest("passwords do not match".to_string()));
        }
        if request.password.len() < 8 {
            return Err(AppError::BadRequest(
                "password must be at least 8 characters".to_string(),
            ));
        }
        let display_name = request.display_name.trim();
        if display_name.is_empty() {
            return Err(AppError::BadRequest("display name is required".to_string()));
        }

        let mut auth = self.auth.write().await;
        let Some(session) = auth.sessions.get(access_token).cloned() else {
            return Err(AppError::Unauthorized);
        };
        if session.expires_at <= Utc::now() {
            return Err(AppError::Unauthorized);
        }
        let current_username = session
            .user
            .username
            .clone()
            .ok_or(AppError::Unauthorized)?;
        if next_username != current_username && auth.accounts.contains_key(&next_username) {
            return Err(AppError::BadRequest("username already exists".to_string()));
        }

        let Some(mut account) = auth.accounts.remove(&current_username) else {
            return Err(AppError::Unauthorized);
        };
        account.user.username = Some(next_username.clone());
        account.user.display_name = display_name.to_string();
        account.user.must_change_password = false;
        account.password_hash = password_hash(&next_username, &request.password);
        account.updated_at = Utc::now();
        let updated_user = account.user.clone();
        let updated_workspace = account.workspace.clone();
        auth.accounts.insert(next_username, account);

        for record in auth.sessions.values_mut() {
            if record.user.id == updated_user.id {
                record.user = updated_user.clone();
                record.workspace = updated_workspace.clone();
            }
        }
        push_audit(
            &mut auth,
            &updated_user,
            "Completed default admin setup",
            "user",
            &updated_user.display_name,
        );
        self.persist_auth(&auth)?;

        Ok(CurrentSession {
            user: updated_user,
            workspace: updated_workspace,
            expires_at: session.expires_at,
        })
    }

    pub async fn refresh_session(&self, request: RefreshSessionRequest) -> AppResult<AuthSession> {
        let mut auth = self.auth.write().await;
        let Some(access_token) = auth.refresh_tokens.get(&request.refresh_token).cloned() else {
            return Err(AppError::Unauthorized);
        };
        let Some(record) = auth.sessions.remove(&access_token) else {
            return Err(AppError::Unauthorized);
        };
        auth.refresh_tokens.remove(&request.refresh_token);
        Ok(create_session(&mut auth, record.user, record.workspace))
    }

    pub async fn logout(&self, access_token: &str) -> AppResult<()> {
        let mut auth = self.auth.write().await;
        if let Some(record) = auth.sessions.remove(access_token) {
            auth.refresh_tokens.remove(&record.refresh_token);
        }
        Ok(())
    }

    pub async fn current_session(&self, access_token: &str) -> AppResult<CurrentSession> {
        let auth = self.auth.read().await;
        let Some(record) = auth.sessions.get(access_token) else {
            return Err(AppError::Unauthorized);
        };
        if record.expires_at <= Utc::now() {
            return Err(AppError::Unauthorized);
        }
        Ok(CurrentSession {
            user: record.user.clone(),
            workspace: record.workspace.clone(),
            expires_at: record.expires_at,
        })
    }

    pub async fn admin_summary(&self, access_token: &str) -> AppResult<AdminSummary> {
        let session = self.current_session(access_token).await?;
        if !session.user.roles.iter().any(|role| role == "admin") {
            return Err(AppError::Unauthorized);
        }

        let mut repository_tables = self.repo.admin_database_tables().await;
        let auth = self.auth.read().await;
        let users = auth
            .accounts
            .values()
            .map(admin_user_summary)
            .collect::<Vec<_>>();
        let total_limit_mb = users.iter().map(|user| user.storage_limit_mb).sum();
        let user_rows = auth
            .accounts
            .values()
            .map(|account| {
                serde_json::json!({
                    "id": account.user.id,
                    "username": account.user.username,
                    "display_name": account.user.display_name,
                    "roles": account.user.roles,
                    "must_change_password": account.user.must_change_password,
                    "storage_limit_mb": account.storage_limit_mb,
                    "created_at": account.created_at,
                    "updated_at": account.updated_at,
                })
            })
            .collect::<Vec<_>>();
        let workspace_rows = auth
            .accounts
            .values()
            .map(|account| {
                serde_json::json!({
                    "id": account.workspace.id,
                    "owner_id": account.user.id,
                    "name": account.workspace.name,
                })
            })
            .collect::<Vec<_>>();
        let session_rows = auth
            .sessions
            .values()
            .map(|session| {
                serde_json::json!({
                    "user_id": session.user.id,
                    "workspace_id": session.workspace.id,
                    "expires_at": session.expires_at,
                })
            })
            .collect::<Vec<_>>();
        let role_policy_rows = role_policies(&auth)
            .into_iter()
            .map(|role| serde_json::to_value(role).unwrap_or_else(|_| serde_json::json!({})))
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

        let mut tables = vec![
            table(
                "users",
                "Users",
                auth.accounts.len(),
                &[
                    "id",
                    "username",
                    "display_name",
                    "roles",
                    "must_change_password",
                    "storage_limit_mb",
                    "created_at",
                    "updated_at",
                ],
                user_rows,
            ),
            table(
                "workspaces",
                "Workspaces",
                auth.accounts.len(),
                &["id", "owner_id", "name"],
                workspace_rows,
            ),
            table(
                "auth_sessions",
                "Auth Sessions",
                auth.sessions.len(),
                &["user_id", "workspace_id", "expires_at"],
                session_rows,
            ),
            table(
                "trusted_devices",
                "Trusted Devices",
                0,
                &["user_id", "device_id", "platform"],
                Vec::new(),
            ),
            table(
                "role_policies",
                "Role Policies",
                role_policy_rows.len(),
                &[
                    "name",
                    "app_scopes",
                    "admin_panel",
                    "manage_users",
                    "manage_storage",
                    "manage_auth",
                    "manage_deployment",
                    "manage_database",
                    "view_audits",
                ],
                role_policy_rows,
            ),
        ];
        tables.append(&mut repository_tables);

        Ok(AdminSummary {
            generated_at: Utc::now(),
            users,
            role_policies: role_policies(&auth),
            storage: AdminStorageOverview {
                total_used_bytes: 0,
                total_limit_mb,
                user_count: auth.accounts.len(),
                notes_bytes: 0,
                audio_bytes: 0,
                files_bytes: 0,
            },
            authentication: AdminAuthSettings {
                default_admin_enabled: true,
                local_password_enabled: true,
                require_setup_password_change: true,
            },
            deployment: AdminDeploymentSettings {
                server_version: env!("CARGO_PKG_VERSION").to_string(),
                build_date: option_env!("OG_SUITE_BUILD_DATE")
                    .unwrap_or("development")
                    .to_string(),
                api_compatibility_version: "1".to_string(),
                release_channel: "local".to_string(),
            },
            database: AdminDatabaseOverview {
                backend: "durable JSON snapshots with postgres-compatible migration schema"
                    .to_string(),
                generated_at: Utc::now(),
                tables,
            },
            audits: auth.audits.clone(),
        })
    }

    pub async fn create_admin_role(
        &self,
        access_token: &str,
        request: CreateAdminRoleRequest,
    ) -> AppResult<AdminRolePolicy> {
        let actor = self.require_admin(access_token).await?;
        let mut role = request;
        role.name = role.name.trim().to_lowercase();
        if role.name.len() < 2 {
            return Err(AppError::BadRequest(
                "role name must be at least 2 characters".to_string(),
            ));
        }

        let mut auth = self.auth.write().await;
        if role_policies(&auth)
            .iter()
            .any(|existing| existing.name == role.name)
        {
            return Err(AppError::BadRequest("role already exists".to_string()));
        }
        auth.custom_role_policies.push(role.clone());
        push_audit(&mut auth, &actor, "Created role policy", "role", &role.name);
        self.persist_auth(&auth)?;
        Ok(role)
    }

    pub async fn create_admin_user(
        &self,
        access_token: &str,
        request: CreateAdminUserRequest,
    ) -> AppResult<AdminUserSummary> {
        let actor = self.require_admin(access_token).await?;
        let username = normalize_username(&request.username)?;
        let display_name = request.display_name.trim();
        if display_name.is_empty() {
            return Err(AppError::BadRequest("display name is required".to_string()));
        }
        if request.password.len() < 8 {
            return Err(AppError::BadRequest(
                "password must be at least 8 characters".to_string(),
            ));
        }
        let roles = normalize_roles(request.roles);
        let now = Utc::now();
        let mut auth = self.auth.write().await;
        if auth.accounts.contains_key(&username) {
            return Err(AppError::BadRequest("profile already exists".to_string()));
        }
        let user = UserProfile {
            id: Uuid::new_v4().to_string(),
            display_name: display_name.to_string(),
            username: Some(username.clone()),
            roles,
            must_change_password: true,
        };
        let workspace = WorkspaceProfile {
            id: Uuid::new_v4().to_string(),
            name: format!("{}'s Workspace", display_name),
        };
        let account = AccountRecord {
            user: user.clone(),
            workspace,
            password_hash: password_hash(&username, &request.password),
            storage_limit_mb: request.storage_limit_mb,
            app_scopes: request.app_scopes,
            created_at: now,
            updated_at: now,
        };
        let summary = admin_user_summary(&account);
        auth.accounts.insert(username, account);
        push_audit(
            &mut auth,
            &actor,
            "Created admin-managed user",
            "user",
            &user.display_name,
        );
        self.persist_auth(&auth)?;
        Ok(summary)
    }

    pub async fn update_admin_user_access(
        &self,
        access_token: &str,
        user_id: &str,
        request: UpdateAdminUserAccessRequest,
    ) -> AppResult<AdminUserSummary> {
        let actor = self.require_admin(access_token).await?;
        let mut auth = self.auth.write().await;
        let Some(username) = username_for_user_id(&auth, user_id) else {
            return Err(AppError::NotFound);
        };
        let (summary, target_user) = {
            let Some(account) = auth.accounts.get_mut(&username) else {
                return Err(AppError::NotFound);
            };
            account.user.roles = normalize_roles(request.roles);
            account.storage_limit_mb = request.storage_limit_mb;
            account.app_scopes = request.app_scopes;
            account.updated_at = Utc::now();
            (admin_user_summary(account), account.user.clone())
        };
        for record in auth.sessions.values_mut() {
            if record.user.id == user_id {
                record.user = target_user.clone();
            }
        }
        push_audit(
            &mut auth,
            &actor,
            "Updated user roles, app scopes, and storage limit",
            "user",
            &summary.display_name,
        );
        self.persist_auth(&auth)?;
        Ok(summary)
    }

    pub async fn reset_admin_user_password(
        &self,
        access_token: &str,
        user_id: &str,
        request: ResetAdminUserPasswordRequest,
    ) -> AppResult<AdminUserSummary> {
        let actor = self.require_admin(access_token).await?;
        if request.password != request.confirm_password {
            return Err(AppError::BadRequest("passwords do not match".to_string()));
        }
        if request.password.len() < 8 {
            return Err(AppError::BadRequest(
                "password must be at least 8 characters".to_string(),
            ));
        }
        let mut auth = self.auth.write().await;
        let Some(username) = username_for_user_id(&auth, user_id) else {
            return Err(AppError::NotFound);
        };
        let (summary, target_user) = {
            let Some(account) = auth.accounts.get_mut(&username) else {
                return Err(AppError::NotFound);
            };
            account.password_hash = password_hash(&username, &request.password);
            account.user.must_change_password = true;
            account.updated_at = Utc::now();
            (admin_user_summary(account), account.user.clone())
        };
        for record in auth.sessions.values_mut() {
            if record.user.id == user_id {
                record.user = target_user.clone();
            }
        }
        push_audit(
            &mut auth,
            &actor,
            "Reset user password",
            "user",
            &summary.display_name,
        );
        self.persist_auth(&auth)?;
        Ok(summary)
    }

    async fn require_admin(&self, access_token: &str) -> AppResult<UserProfile> {
        let session = self.current_session(access_token).await?;
        if !session.user.roles.iter().any(|role| role == "admin") {
            return Err(AppError::Unauthorized);
        }
        Ok(session.user)
    }

    pub async fn join_presence(
        &self,
        document_id: &str,
        client_id: String,
    ) -> broadcast::Receiver<Vec<PresencePeer>> {
        let mut rooms = self.presence.write().await;
        let room = rooms.entry(document_id.to_string()).or_insert_with(|| {
            let (tx, _) = broadcast::channel(64);
            PresenceRoom {
                peers: HashMap::new(),
                tx,
            }
        });
        room.peers.insert(
            client_id.clone(),
            PresencePeer {
                client_id,
                user_id: "local-user".to_string(),
                display_name: "Local user".to_string(),
                cursor: None,
                color: "#67c587".to_string(),
                last_seen_at: Utc::now(),
            },
        );
        let peers = room.peers.values().cloned().collect::<Vec<_>>();
        let _ = room.tx.send(peers);
        room.tx.subscribe()
    }

    pub async fn update_presence_cursor(
        &self,
        document_id: &str,
        client_id: &str,
        cursor: Option<usize>,
    ) {
        let mut rooms = self.presence.write().await;
        if let Some(room) = rooms.get_mut(document_id) {
            if let Some(peer) = room.peers.get_mut(client_id) {
                peer.cursor = cursor;
                peer.last_seen_at = Utc::now();
            }
            let peers = room.peers.values().cloned().collect::<Vec<_>>();
            let _ = room.tx.send(peers);
        }
    }

    pub async fn leave_presence(&self, document_id: &str, client_id: &str) {
        let mut rooms = self.presence.write().await;
        if let Some(room) = rooms.get_mut(document_id) {
            room.peers.remove(client_id);
            let peers = room.peers.values().cloned().collect::<Vec<_>>();
            let _ = room.tx.send(peers);
        }
    }

    pub async fn join_document_updates(
        &self,
        document_id: &str,
    ) -> broadcast::Receiver<CrdtUpdate> {
        let mut rooms = self.document_updates.write().await;
        let room = rooms.entry(document_id.to_string()).or_insert_with(|| {
            let (tx, _) = broadcast::channel(256);
            DocumentUpdateRoom { tx }
        });
        room.tx.subscribe()
    }

    pub async fn broadcast_document_update(&self, document_id: &str, update: CrdtUpdate) {
        let rooms = self.document_updates.read().await;
        if let Some(room) = rooms.get(document_id) {
            let _ = room.tx.send(update);
        }
    }
}

impl From<&AuthStore> for AuthSnapshot {
    fn from(auth: &AuthStore) -> Self {
        Self {
            accounts: auth.accounts.values().cloned().collect(),
            audits: auth.audits.clone(),
            custom_role_policies: auth.custom_role_policies.clone(),
        }
    }
}

impl From<AuthSnapshot> for AuthStore {
    fn from(snapshot: AuthSnapshot) -> Self {
        Self {
            accounts: snapshot
                .accounts
                .into_iter()
                .filter_map(|account| {
                    account
                        .user
                        .username
                        .clone()
                        .map(|username| (username, account))
                })
                .collect(),
            sessions: HashMap::new(),
            refresh_tokens: HashMap::new(),
            audits: snapshot.audits,
            custom_role_policies: snapshot.custom_role_policies,
        }
    }
}

fn auth_data_path() -> Option<PathBuf> {
    if let Ok(path) = std::env::var("OG_SUITE_AUTH_FILE") {
        let trimmed = path.trim();
        if !trimmed.is_empty() {
            return Some(PathBuf::from(trimmed));
        }
    }
    std::env::var("OG_SUITE_DATA_DIR")
        .ok()
        .map(|path| PathBuf::from(path).join("auth.json"))
}

fn load_auth_store(path: &PathBuf) -> AppResult<AuthStore> {
    if !path.exists() {
        return Ok(AuthStore::default());
    }
    let json =
        std::fs::read_to_string(path).map_err(|error| AppError::Database(error.to_string()))?;
    let snapshot = serde_json::from_str::<AuthSnapshot>(&json)
        .map_err(|error| AppError::Database(error.to_string()))?;
    Ok(snapshot.into())
}

fn create_session(
    auth: &mut AuthStore,
    user: UserProfile,
    workspace: WorkspaceProfile,
) -> AuthSession {
    let access_token = format!("oga_{}", Uuid::new_v4().simple());
    let refresh_token = format!("ogr_{}", Uuid::new_v4().simple());
    let expires_at = Utc::now() + Duration::hours(12);
    auth.sessions.insert(
        access_token.clone(),
        SessionRecord {
            user: user.clone(),
            workspace: workspace.clone(),
            refresh_token: refresh_token.clone(),
            expires_at,
        },
    );
    auth.refresh_tokens
        .insert(refresh_token.clone(), access_token.clone());
    AuthSession {
        user,
        workspace,
        access_token,
        refresh_token,
        expires_at,
    }
}

fn normalize_username(username: &str) -> AppResult<String> {
    let normalized = username.trim().to_lowercase();
    if normalized.len() < 3 {
        return Err(AppError::BadRequest(
            "username must be at least 3 characters".to_string(),
        ));
    }
    Ok(normalized)
}

fn password_hash(username: &str, password: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"og-suite-local-profile-v1:");
    hasher.update(username.as_bytes());
    hasher.update(b":");
    hasher.update(password.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn seed_default_admin(auth: &mut AuthStore) {
    let now = Utc::now();
    let username = "admin".to_string();
    let user = UserProfile {
        id: Uuid::new_v4().to_string(),
        display_name: "Default Admin".to_string(),
        username: Some(username.clone()),
        roles: vec!["admin".to_string(), "owner".to_string()],
        must_change_password: true,
    };
    let workspace = WorkspaceProfile {
        id: Uuid::new_v4().to_string(),
        name: "OG Suite Workspace".to_string(),
    };
    auth.accounts.insert(
        username.clone(),
        AccountRecord {
            user: user.clone(),
            workspace,
            password_hash: password_hash(&username, "password"),
            storage_limit_mb: 10240,
            app_scopes: AppToolScope::admin(),
            created_at: now,
            updated_at: now,
        },
    );
    push_audit(auth, &user, "Seeded default admin account", "user", "admin");
}

fn admin_user_summary(account: &AccountRecord) -> AdminUserSummary {
    AdminUserSummary {
        id: account.user.id.clone(),
        username: account.user.username.clone().unwrap_or_default(),
        display_name: account.user.display_name.clone(),
        roles: account.user.roles.clone(),
        must_change_password: account.user.must_change_password,
        storage_used_bytes: 0,
        storage_limit_mb: account.storage_limit_mb,
        app_scopes: account.app_scopes.clone(),
        created_at: account.created_at,
        updated_at: account.updated_at,
    }
}

fn username_for_user_id(auth: &AuthStore, user_id: &str) -> Option<String> {
    auth.accounts.iter().find_map(|(username, account)| {
        if account.user.id == user_id {
            Some(username.clone())
        } else {
            None
        }
    })
}

fn normalize_roles(roles: Vec<String>) -> Vec<String> {
    let mut normalized = roles
        .into_iter()
        .map(|role| role.trim().to_lowercase())
        .filter(|role| !role.is_empty())
        .collect::<Vec<_>>();
    normalized.sort();
    normalized.dedup();
    if normalized.is_empty() {
        normalized.push("owner".to_string());
    }
    normalized
}

fn role_policies(auth: &AuthStore) -> Vec<AdminRolePolicy> {
    let mut policies = vec![
        AdminRolePolicy {
            name: "admin".to_string(),
            app_scopes: AppToolScope::admin(),
            admin_panel: true,
            manage_users: true,
            manage_storage: true,
            manage_auth: true,
            manage_deployment: true,
            manage_database: true,
            view_audits: true,
        },
        AdminRolePolicy {
            name: "owner".to_string(),
            app_scopes: AppToolScope::member(),
            admin_panel: false,
            manage_users: false,
            manage_storage: false,
            manage_auth: false,
            manage_deployment: false,
            manage_database: false,
            view_audits: false,
        },
    ];
    policies.extend(auth.custom_role_policies.clone());
    policies
}

fn push_audit(
    auth: &mut AuthStore,
    actor: &UserProfile,
    action: &str,
    target_kind: &str,
    target_label: &str,
) {
    auth.audits.push(AdminAuditEntry {
        id: Uuid::new_v4().to_string(),
        occurred_at: Utc::now(),
        actor_id: actor.id.clone(),
        actor_label: actor.display_name.clone(),
        action: action.to_string(),
        target_kind: target_kind.to_string(),
        target_label: target_label.to_string(),
        details: None,
    });
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
