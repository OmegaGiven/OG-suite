use crate::{error::AppResult, models::*, repository::SuiteRepository, state::AppState};
use axum::{
    Json, Router,
    body::Body,
    extract::{
        Path, Query, State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    http::{
        HeaderMap, StatusCode,
        header::{CONTENT_DISPOSITION, CONTENT_TYPE},
    },
    response::{IntoResponse, Response},
    routing::{delete, get, patch, post},
};
use chrono::Utc;
use futures::{SinkExt, StreamExt};
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/api/v1/system/version", get(system_version))
        .route("/api/v1/auth/register", post(register_profile))
        .route("/api/v1/auth/login", post(login))
        .route("/api/v1/auth/refresh", post(refresh_session))
        .route("/api/v1/auth/session", get(current_session))
        .route("/api/v1/auth/complete-setup", post(complete_setup))
        .route("/api/v1/auth/logout", post(logout))
        .route(
            "/api/v1/appearance/themes",
            get(list_appearance_themes).post(create_appearance_theme),
        )
        .route(
            "/api/v1/appearance/settings",
            get(get_appearance_settings).put(update_appearance_settings),
        )
        .route(
            "/api/v1/appearance/themes/{id}",
            patch(update_appearance_theme).delete(delete_appearance_theme),
        )
        .route("/api/v1/admin/summary", get(admin_summary))
        .route("/api/v1/admin/roles", post(create_admin_role))
        .route("/api/v1/admin/users", post(create_admin_user))
        .route(
            "/api/v1/admin/users/{id}/access",
            patch(update_admin_user_access),
        )
        .route(
            "/api/v1/admin/users/{id}/reset-password",
            post(reset_admin_user_password),
        )
        .route("/api/v1/apps", get(list_apps))
        .route("/api/v1/sync/bootstrap", get(sync_bootstrap))
        .route("/api/v1/sync/pull", post(sync_pull))
        .route("/api/v1/sync/push", post(sync_push))
        .route("/api/v1/feed", get(feed_events))
        .route(
            "/api/v1/feed/favorites",
            get(feed_favorites).post(create_feed_favorite),
        )
        .route("/api/v1/feed/favorites/{id}", delete(delete_feed_favorite))
        .route(
            "/api/v1/audio/recordings",
            get(list_audio_recordings).post(create_audio_recording),
        )
        .route(
            "/api/v1/audio/folders",
            get(list_audio_folders).post(create_audio_folder),
        )
        .route("/api/v1/audio/folders/{id}", delete(delete_audio_folder))
        .route(
            "/api/v1/audio/recordings/{id}",
            get(get_audio_recording)
                .patch(update_audio_recording)
                .delete(delete_audio_recording),
        )
        .route(
            "/api/v1/audio/recordings/{id}/audio",
            get(get_audio_asset).post(upload_audio),
        )
        .route(
            "/api/v1/audio/recordings/{id}/transcript",
            get(get_audio_transcript).post(retry_audio_transcription),
        )
        .route(
            "/api/v1/audio/recordings/{id}/transcript.vtt",
            get(download_audio_transcript_vtt),
        )
        .route(
            "/api/v1/audio/recordings/{id}/transcript.srt",
            get(download_audio_transcript_srt),
        )
        .route("/api/v1/note-folders", get(list_note_folders))
        .route("/api/v1/notes", get(list_notes).post(create_note))
        .route("/api/v1/notes/{id}/metadata", patch(update_note_metadata))
        .route("/api/v1/notes/{id}", delete(delete_note))
        .route("/api/v1/documents/{id}", get(get_document))
        .route(
            "/api/v1/documents/{id}/updates",
            post(append_document_updates),
        )
        .route("/ws/presence/{document_id}", get(presence_socket))
        .route("/ws/documents/{document_id}", get(document_socket))
        .with_state(state)
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        timestamp: Utc::now(),
    })
}

async fn system_version() -> Json<SystemVersion> {
    Json(SystemVersion {
        backend_version: env!("CARGO_PKG_VERSION").to_string(),
        api_compatibility_version: "1".to_string(),
        minimum_client_version: "0.1.0".to_string(),
        build_date: option_env!("OG_SUITE_BUILD_DATE")
            .unwrap_or("development")
            .to_string(),
        capabilities: vec![
            "profiles".to_string(),
            "local-auth".to_string(),
            "default-admin-setup".to_string(),
            "admin-dashboard".to_string(),
            "sync".to_string(),
            "audio".to_string(),
            "files".to_string(),
            "appearance-themes".to_string(),
            "appearance-settings".to_string(),
        ],
        auth_required: true,
        auth_modes: vec!["local-password".to_string()],
    })
}

async fn list_apps(State(state): State<AppState>) -> AppResult<Json<Vec<AppRegistryEntry>>> {
    Ok(Json(state.repo.apps().await?))
}

async fn register_profile(
    State(state): State<AppState>,
    Json(payload): Json<RegisterProfileRequest>,
) -> AppResult<Json<AuthSession>> {
    Ok(Json(state.register_profile(payload).await?))
}

async fn login(
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> AppResult<Json<AuthSession>> {
    Ok(Json(state.login(payload).await?))
}

async fn refresh_session(
    State(state): State<AppState>,
    Json(payload): Json<RefreshSessionRequest>,
) -> AppResult<Json<AuthSession>> {
    Ok(Json(state.refresh_session(payload).await?))
}

async fn current_session(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<CurrentSession>> {
    Ok(Json(state.current_session(&bearer_token(&headers)?).await?))
}

async fn complete_setup(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<CompleteSetupRequest>,
) -> AppResult<Json<CurrentSession>> {
    Ok(Json(
        state
            .complete_setup(&bearer_token(&headers)?, payload)
            .await?,
    ))
}

async fn logout(State(state): State<AppState>, headers: HeaderMap) -> AppResult<StatusCode> {
    state.logout(&bearer_token(&headers)?).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn list_appearance_themes(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<Vec<AppearanceTheme>>> {
    let session = state.current_session(&bearer_token(&headers)?).await?;
    Ok(Json(
        state
            .repo
            .appearance_themes(&session.user.id, &session.workspace.id)
            .await?,
    ))
}

async fn create_appearance_theme(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<CreateAppearanceThemeRequest>,
) -> AppResult<Json<AppearanceTheme>> {
    let session = state.current_session(&bearer_token(&headers)?).await?;
    Ok(Json(
        state
            .repo
            .create_appearance_theme(&session.user.id, &session.workspace.id, payload)
            .await?,
    ))
}

async fn update_appearance_theme(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<UpdateAppearanceThemeRequest>,
) -> AppResult<Json<AppearanceTheme>> {
    let session = state.current_session(&bearer_token(&headers)?).await?;
    Ok(Json(
        state
            .repo
            .update_appearance_theme(id, &session.user.id, payload)
            .await?,
    ))
}

async fn delete_appearance_theme(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<StatusCode> {
    let session = state.current_session(&bearer_token(&headers)?).await?;
    state
        .repo
        .delete_appearance_theme(id, &session.user.id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn get_appearance_settings(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<Option<AppearanceSettings>>> {
    let session = state.current_session(&bearer_token(&headers)?).await?;
    Ok(Json(
        state
            .repo
            .appearance_settings(&session.user.id, &session.workspace.id)
            .await?,
    ))
}

async fn update_appearance_settings(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<UpdateAppearanceSettingsRequest>,
) -> AppResult<Json<AppearanceSettings>> {
    let session = state.current_session(&bearer_token(&headers)?).await?;
    Ok(Json(
        state
            .repo
            .update_appearance_settings(&session.user.id, &session.workspace.id, payload)
            .await?,
    ))
}

async fn admin_summary(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<AdminSummary>> {
    Ok(Json(state.admin_summary(&bearer_token(&headers)?).await?))
}

async fn create_admin_user(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<CreateAdminUserRequest>,
) -> AppResult<Json<AdminUserSummary>> {
    Ok(Json(
        state
            .create_admin_user(&bearer_token(&headers)?, payload)
            .await?,
    ))
}

async fn create_admin_role(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<CreateAdminRoleRequest>,
) -> AppResult<Json<AdminRolePolicy>> {
    Ok(Json(
        state
            .create_admin_role(&bearer_token(&headers)?, payload)
            .await?,
    ))
}

async fn update_admin_user_access(
    Path(id): Path<String>,
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<UpdateAdminUserAccessRequest>,
) -> AppResult<Json<AdminUserSummary>> {
    Ok(Json(
        state
            .update_admin_user_access(&bearer_token(&headers)?, &id, payload)
            .await?,
    ))
}

async fn reset_admin_user_password(
    Path(id): Path<String>,
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<ResetAdminUserPasswordRequest>,
) -> AppResult<Json<AdminUserSummary>> {
    Ok(Json(
        state
            .reset_admin_user_password(&bearer_token(&headers)?, &id, payload)
            .await?,
    ))
}

async fn sync_bootstrap(State(state): State<AppState>) -> AppResult<Json<SyncEnvelope>> {
    Ok(Json(state.repo.envelope().await?))
}

async fn sync_pull(
    State(state): State<AppState>,
    Json(_payload): Json<SyncPullRequest>,
) -> AppResult<Json<SyncEnvelope>> {
    Ok(Json(state.repo.envelope().await?))
}

async fn sync_push(
    State(state): State<AppState>,
    Json(payload): Json<SyncPushRequest>,
) -> AppResult<Json<SyncPushResponse>> {
    let mut accepted = Vec::new();
    for operation in payload.operations {
        accepted.push(operation_id(&operation));
        let event = feed_event_from_operation(&operation);
        let update_to_broadcast = match &operation {
            SyncOperation::AppendDocumentUpdate { update } => Some(update.clone()),
            _ => None,
        };
        state.repo.apply_sync_operation(operation).await?;
        state.repo.append_feed_event(event).await?;
        if let Some(update) = update_to_broadcast {
            state
                .broadcast_document_update(&update.document_id.to_string(), update)
                .await;
        }
    }
    let envelope = state.repo.envelope().await?;
    Ok(Json(SyncPushResponse {
        cursors: envelope.cursors,
        apps: envelope.apps,
        note_folders: envelope.note_folders,
        notes: envelope.notes,
        documents: envelope.documents,
        document_updates: envelope.document_updates,
        tombstones: envelope.tombstones,
        conflicts: envelope.conflicts,
        accepted_operation_ids: accepted,
    }))
}

async fn list_notes(State(state): State<AppState>) -> AppResult<Json<Vec<Note>>> {
    Ok(Json(state.repo.notes().await?))
}

async fn list_note_folders(State(state): State<AppState>) -> AppResult<Json<Vec<NoteFolder>>> {
    Ok(Json(state.repo.note_folders().await?))
}

async fn create_note(
    State(state): State<AppState>,
    Json(payload): Json<CreateNoteRequest>,
) -> AppResult<Json<Note>> {
    let note = state.repo.create_note(payload).await?;
    state
        .repo
        .append_feed_event(note_event(
            FeedActivityAction::NoteCreated,
            &note,
            format!("Created note \"{}\"", note.title),
        ))
        .await?;
    Ok(Json(note))
}

async fn update_note_metadata(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
    Json(payload): Json<UpdateNoteMetadataRequest>,
) -> AppResult<Json<Note>> {
    let note = state.repo.update_note_metadata(id, payload).await?;
    state
        .repo
        .append_feed_event(note_event(
            FeedActivityAction::NoteMetadataUpdated,
            &note,
            format!("Updated note \"{}\"", note.title),
        ))
        .await?;
    Ok(Json(note))
}

async fn delete_note(Path(id): Path<Uuid>, State(state): State<AppState>) -> AppResult<StatusCode> {
    state.repo.delete_note(id).await?;
    state
        .repo
        .append_feed_event(feed_event(
            FeedActivityAction::NoteDeleted,
            "notes",
            "Deleted a note".to_string(),
            "note",
            id.to_string(),
            short_id(id),
            None,
        ))
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn feed_events(State(state): State<AppState>) -> AppResult<Json<Vec<FeedActivityEvent>>> {
    Ok(Json(state.repo.feed_events().await?))
}

async fn feed_favorites(State(state): State<AppState>) -> AppResult<Json<Vec<FeedFavorite>>> {
    Ok(Json(state.repo.feed_favorites().await?))
}

async fn create_feed_favorite(
    State(state): State<AppState>,
    Json(payload): Json<CreateFeedFavoriteRequest>,
) -> AppResult<Json<FeedFavorite>> {
    let favorite = state.repo.create_feed_favorite(payload).await?;
    state
        .repo
        .append_feed_event(feed_event(
            FeedActivityAction::FavoriteAdded,
            "feed",
            format!("Added \"{}\" to favorites", favorite.label),
            &favorite.target_kind,
            favorite.target_id.clone(),
            favorite.label.clone(),
            Some(favorite.workspace_id.clone()),
        ))
        .await?;
    Ok(Json(favorite))
}

async fn delete_feed_favorite(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
) -> AppResult<StatusCode> {
    state.repo.delete_feed_favorite(id).await?;
    state
        .repo
        .append_feed_event(feed_event(
            FeedActivityAction::FavoriteRemoved,
            "feed",
            "Removed a favorite".to_string(),
            "tool",
            id.to_string(),
            short_id(id),
            None,
        ))
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn list_audio_recordings(
    State(state): State<AppState>,
) -> AppResult<Json<Vec<AudioRecording>>> {
    Ok(Json(state.repo.audio_recordings().await?))
}

async fn list_audio_folders(State(state): State<AppState>) -> AppResult<Json<Vec<AudioFolder>>> {
    Ok(Json(state.repo.audio_folders().await?))
}

async fn create_audio_folder(
    State(state): State<AppState>,
    Json(payload): Json<CreateAudioFolderRequest>,
) -> AppResult<Json<AudioFolder>> {
    Ok(Json(state.repo.create_audio_folder(payload).await?))
}

async fn delete_audio_folder(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
) -> AppResult<StatusCode> {
    state.repo.delete_audio_folder(id).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn create_audio_recording(
    State(state): State<AppState>,
    Json(payload): Json<CreateAudioRecordingRequest>,
) -> AppResult<Json<AudioRecording>> {
    let recording = state.repo.create_audio_recording(payload).await?;
    state
        .repo
        .append_feed_event(feed_event(
            FeedActivityAction::AudioRecordingCreated,
            "audio",
            format!("Created audio recording \"{}\"", recording.title),
            "recording",
            recording.id.to_string(),
            recording.title.clone(),
            Some(recording.workspace_id.clone()),
        ))
        .await?;
    Ok(Json(recording))
}

async fn get_audio_recording(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
) -> AppResult<Json<AudioRecording>> {
    Ok(Json(state.repo.audio_recording(id).await?))
}

async fn update_audio_recording(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
    Json(payload): Json<UpdateAudioRecordingRequest>,
) -> AppResult<Json<AudioRecording>> {
    let recording = state.repo.update_audio_recording(id, payload).await?;
    state
        .repo
        .append_feed_event(feed_event(
            FeedActivityAction::AudioRecordingRenamed,
            "audio",
            format!("Renamed audio recording \"{}\"", recording.title),
            "recording",
            recording.id.to_string(),
            recording.title.clone(),
            Some(recording.workspace_id.clone()),
        ))
        .await?;
    Ok(Json(recording))
}

async fn upload_audio(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
    Json(payload): Json<UploadAudioRequest>,
) -> AppResult<Json<AudioRecording>> {
    let data_url = payload.data_url.clone();
    let mime_type = payload.mime_type.clone();
    let recording = state.repo.upload_audio(id, payload).await?;
    state
        .repo
        .append_feed_event(feed_event(
            FeedActivityAction::AudioUploaded,
            "audio",
            format!("Uploaded audio for \"{}\"", recording.title),
            "recording",
            recording.id.to_string(),
            recording.title.clone(),
            Some(recording.workspace_id.clone()),
        ))
        .await?;
    state
        .repo
        .append_feed_event(feed_event(
            FeedActivityAction::AudioTranscriptQueued,
            "audio",
            format!("Queued transcript for \"{}\"", recording.title),
            "recording",
            recording.id.to_string(),
            recording.title.clone(),
            Some(recording.workspace_id.clone()),
        ))
        .await?;
    run_local_transcription(&state, &recording, &data_url, &mime_type).await?;
    Ok(Json(recording))
}

async fn get_audio_asset(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
) -> AppResult<Response> {
    let data_url = state.repo.audio_asset(id).await?;
    let (mime_type, bytes) = decode_data_url(&data_url)?;
    Ok(([(CONTENT_TYPE, mime_type)], Body::from(bytes)).into_response())
}

async fn delete_audio_recording(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
) -> AppResult<StatusCode> {
    state.repo.delete_audio_recording(id).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn get_audio_transcript(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
) -> AppResult<Json<AudioTranscript>> {
    Ok(Json(state.repo.audio_transcript(id).await?))
}

async fn download_audio_transcript_vtt(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
) -> AppResult<Response> {
    let recording = state.repo.audio_recording(id).await?;
    let transcript = state.repo.audio_transcript(id).await?;
    let filename = caption_filename(&recording.title, "vtt");
    Ok((
        [
            (CONTENT_TYPE, "text/vtt; charset=utf-8".to_string()),
            (
                CONTENT_DISPOSITION,
                format!("attachment; filename=\"{filename}\""),
            ),
        ],
        Body::from(render_vtt(&transcript)),
    )
        .into_response())
}

async fn download_audio_transcript_srt(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
) -> AppResult<Response> {
    let recording = state.repo.audio_recording(id).await?;
    let transcript = state.repo.audio_transcript(id).await?;
    let filename = caption_filename(&recording.title, "srt");
    Ok((
        [
            (
                CONTENT_TYPE,
                "application/x-subrip; charset=utf-8".to_string(),
            ),
            (
                CONTENT_DISPOSITION,
                format!("attachment; filename=\"{filename}\""),
            ),
        ],
        Body::from(render_srt(&transcript)),
    )
        .into_response())
}

async fn retry_audio_transcription(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
) -> AppResult<Json<AudioTranscriptionStatus>> {
    let recording = state.repo.audio_recording(id).await?;
    state
        .repo
        .update_audio_recording_status(id, "transcribing")
        .await?;
    let status = AudioTranscriptionStatus {
        recording_id: id,
        status: "queued".to_string(),
        engine: state.transcription.label().to_string(),
        updated_at: Utc::now(),
    };
    state
        .repo
        .append_feed_event(feed_event(
            FeedActivityAction::AudioTranscriptQueued,
            "audio",
            format!("Queued transcript for \"{}\"", recording.title),
            "recording",
            recording.id.to_string(),
            recording.title,
            Some(recording.workspace_id),
        ))
        .await?;
    Ok(Json(status))
}

async fn run_local_transcription(
    state: &AppState,
    recording: &AudioRecording,
    data_url: &str,
    mime_type: &str,
) -> AppResult<()> {
    if let Some(transcript) = state
        .transcription
        .transcribe(recording.id, data_url, mime_type, recording.duration_ms)
        .await
    {
        let status = transcript.status.clone();
        state.repo.upsert_audio_transcript(transcript).await?;
        if status == "ready" {
            state
                .repo
                .append_feed_event(feed_event(
                    FeedActivityAction::AudioTranscriptGenerated,
                    "audio",
                    format!("Generated local transcript for \"{}\"", recording.title),
                    "recording",
                    recording.id.to_string(),
                    recording.title.clone(),
                    Some(recording.workspace_id.clone()),
                ))
                .await?;
        }
    }
    Ok(())
}

async fn get_document(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
) -> AppResult<Json<CrdtDocumentState>> {
    Ok(Json(state.repo.document(id).await?))
}

async fn append_document_updates(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
    Json(payload): Json<AppendDocumentUpdatesRequest>,
) -> AppResult<Json<CrdtDocumentState>> {
    let incoming_count = payload.updates.len();
    let document = state
        .repo
        .append_document_updates(id, payload.updates)
        .await?;
    for update in document.updates.iter().rev().take(incoming_count) {
        state
            .broadcast_document_update(&id.to_string(), update.clone())
            .await;
    }
    if incoming_count > 0 {
        state
            .repo
            .append_feed_event(document_event(id, "Edited document".to_string()))
            .await?;
    }
    Ok(Json(document))
}

#[derive(Deserialize)]
struct PresenceQuery {
    client_id: Option<String>,
}

async fn presence_socket(
    Path(document_id): Path<String>,
    Query(query): Query<PresenceQuery>,
    State(state): State<AppState>,
    upgrade: WebSocketUpgrade,
) -> impl IntoResponse {
    let client_id = query
        .client_id
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    upgrade.on_upgrade(move |socket| handle_presence_socket(socket, state, document_id, client_id))
}

async fn handle_presence_socket(
    socket: WebSocket,
    state: AppState,
    document_id: String,
    client_id: String,
) {
    let mut rx = state.join_presence(&document_id, client_id.clone()).await;
    let (mut sender, mut receiver) = socket.split();
    let send_task = tokio::spawn(async move {
        while let Ok(peers) = rx.recv().await {
            let message = json!({ "peers": peers }).to_string();
            if sender.send(Message::Text(message.into())).await.is_err() {
                break;
            }
        }
    });

    while let Some(Ok(message)) = receiver.next().await {
        if let Message::Text(text) = message {
            if let Ok(payload) = serde_json::from_str::<serde_json::Value>(&text) {
                let cursor = payload
                    .get("cursor")
                    .and_then(|value| value.as_u64())
                    .map(|value| value as usize);
                state
                    .update_presence_cursor(&document_id, &client_id, cursor)
                    .await;
            }
        }
    }

    state.leave_presence(&document_id, &client_id).await;
    send_task.abort();
}

#[derive(Deserialize)]
struct DocumentSocketQuery {
    client_id: Option<String>,
}

async fn document_socket(
    Path(document_id): Path<String>,
    Query(query): Query<DocumentSocketQuery>,
    State(state): State<AppState>,
    upgrade: WebSocketUpgrade,
) -> impl IntoResponse {
    let client_id = query
        .client_id
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    upgrade.on_upgrade(move |socket| handle_document_socket(socket, state, document_id, client_id))
}

async fn handle_document_socket(
    socket: WebSocket,
    state: AppState,
    document_id: String,
    client_id: String,
) {
    let mut rx = state.join_document_updates(&document_id).await;
    let (mut sender, mut receiver) = socket.split();
    let send_task = tokio::spawn(async move {
        while let Ok(update) = rx.recv().await {
            if update.client_id == client_id {
                continue;
            }
            let message = json!({ "update": update }).to_string();
            if sender.send(Message::Text(message.into())).await.is_err() {
                break;
            }
        }
    });

    while let Some(Ok(message)) = receiver.next().await {
        if let Message::Text(text) = message {
            if let Ok(update) = serde_json::from_str::<CrdtUpdate>(&text) {
                if state
                    .repo
                    .append_document_update(update.clone())
                    .await
                    .is_ok()
                {
                    let _ = state
                        .repo
                        .append_feed_event(document_event(
                            update.document_id,
                            "Edited document".to_string(),
                        ))
                        .await;
                    state.broadcast_document_update(&document_id, update).await;
                }
            }
        }
    }

    send_task.abort();
}

fn operation_id(operation: &SyncOperation) -> String {
    match operation {
        SyncOperation::CreateNote { note, .. } => format!("create-note:{}", note.id),
        SyncOperation::UpdateNoteMetadata { note } => format!("update-note:{}", note.id),
        SyncOperation::DeleteNote { id, .. } => format!("delete-note:{id}"),
        SyncOperation::CreateNoteFolder { folder } => format!("create-note-folder:{}", folder.id),
        SyncOperation::DeleteNoteFolder { id, .. } => format!("delete-note-folder:{id}"),
        SyncOperation::AppendDocumentUpdate { update } => {
            format!("append-document-update:{}", update.id)
        }
    }
}

fn feed_event_from_operation(operation: &SyncOperation) -> FeedActivityEvent {
    match operation {
        SyncOperation::CreateNote { note, .. } => note_event(
            FeedActivityAction::NoteCreated,
            note,
            format!("Created note \"{}\"", note.title),
        ),
        SyncOperation::UpdateNoteMetadata { note } => note_event(
            FeedActivityAction::NoteMetadataUpdated,
            note,
            format!("Updated note \"{}\"", note.title),
        ),
        SyncOperation::DeleteNote { id, .. } => feed_event(
            FeedActivityAction::NoteDeleted,
            "notes",
            "Deleted a note".to_string(),
            "note",
            id.to_string(),
            short_id(*id),
            None,
        ),
        SyncOperation::CreateNoteFolder { folder } => feed_event(
            FeedActivityAction::FolderCreated,
            "notes",
            format!("Created folder \"{}\"", folder.path),
            "folder",
            folder.id.to_string(),
            folder.path.clone(),
            Some(folder.workspace_id.clone()),
        ),
        SyncOperation::DeleteNoteFolder { id, .. } => feed_event(
            FeedActivityAction::FolderDeleted,
            "notes",
            "Deleted a folder".to_string(),
            "folder",
            id.to_string(),
            short_id(*id),
            None,
        ),
        SyncOperation::AppendDocumentUpdate { update } => {
            document_event(update.document_id, "Edited document".to_string())
        }
    }
}

fn note_event(action: FeedActivityAction, note: &Note, summary: String) -> FeedActivityEvent {
    feed_event(
        action,
        "notes",
        summary,
        "note",
        note.id.to_string(),
        note.title.clone(),
        Some(note.workspace_id.clone()),
    )
}

fn document_event(document_id: Uuid, summary: String) -> FeedActivityEvent {
    feed_event(
        FeedActivityAction::DocumentEdited,
        "notes",
        summary,
        "document",
        document_id.to_string(),
        short_id(document_id),
        None,
    )
}

fn feed_event(
    action: FeedActivityAction,
    app_id: &str,
    summary: String,
    target_kind: &str,
    target_id: String,
    target_label: String,
    workspace_id: Option<String>,
) -> FeedActivityEvent {
    FeedActivityEvent {
        id: Uuid::new_v4(),
        app_id: app_id.to_string(),
        action,
        summary,
        target_kind: target_kind.to_string(),
        target_id,
        target_label,
        actor_id: "local-user".to_string(),
        actor_name: "Local user".to_string(),
        workspace_id: workspace_id.unwrap_or_else(|| "default".to_string()),
        is_public: true,
        created_at: Utc::now(),
    }
}

fn short_id(id: Uuid) -> String {
    id.to_string().chars().take(8).collect()
}

fn decode_data_url(data_url: &str) -> AppResult<(String, Vec<u8>)> {
    use base64::{Engine as _, engine::general_purpose::STANDARD};
    let Some((metadata, encoded)) = data_url.split_once(',') else {
        return Err(crate::error::AppError::Database(
            "invalid audio data URL".to_string(),
        ));
    };
    let mime_type = metadata
        .strip_prefix("data:")
        .and_then(|value| value.split(';').next())
        .filter(|value| !value.is_empty())
        .unwrap_or("application/octet-stream")
        .to_string();
    let bytes = STANDARD
        .decode(encoded.as_bytes())
        .map_err(|error| crate::error::AppError::Database(error.to_string()))?;
    Ok((mime_type, bytes))
}

fn render_vtt(transcript: &AudioTranscript) -> String {
    let mut output = String::from("WEBVTT\n\n");
    for segment in &transcript.segments {
        output.push_str(&format!(
            "{} --> {}\n{}\n\n",
            format_vtt_time(segment.start_ms),
            format_vtt_time(segment.end_ms),
            caption_text(segment),
        ));
    }
    output
}

fn render_srt(transcript: &AudioTranscript) -> String {
    transcript
        .segments
        .iter()
        .enumerate()
        .map(|(index, segment)| {
            format!(
                "{}\n{} --> {}\n{}\n",
                index + 1,
                format_srt_time(segment.start_ms),
                format_srt_time(segment.end_ms),
                caption_text(segment),
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn caption_text(segment: &AudioTranscriptSegment) -> String {
    if let Some(speaker) = &segment.speaker_label {
        if !speaker.is_empty() && speaker != "Speaker 1" {
            return format!("{speaker}: {}", segment.text);
        }
    }
    segment.text.clone()
}

fn format_vtt_time(ms: u64) -> String {
    let hours = ms / 3_600_000;
    let minutes = (ms % 3_600_000) / 60_000;
    let seconds = (ms % 60_000) / 1_000;
    let millis = ms % 1_000;
    format!("{hours:02}:{minutes:02}:{seconds:02}.{millis:03}")
}

fn format_srt_time(ms: u64) -> String {
    format_vtt_time(ms).replace('.', ",")
}

fn caption_filename(title: &str, extension: &str) -> String {
    let stem = title
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_string();
    format!(
        "{}.{}",
        if stem.is_empty() { "transcript" } else { &stem },
        extension
    )
}

fn bearer_token(headers: &HeaderMap) -> AppResult<String> {
    let value = headers
        .get("authorization")
        .and_then(|header| header.to_str().ok())
        .ok_or(crate::error::AppError::Unauthorized)?;
    let Some(token) = value.strip_prefix("Bearer ") else {
        return Err(crate::error::AppError::Unauthorized);
    };
    Ok(token.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transcription::LocalTranscriptionEngine;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    #[tokio::test]
    async fn health_route_works() {
        let app = router(AppState::new());
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn auth_register_session_and_logout_work() {
        let app = router(AppState::new());
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/auth/register")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&RegisterProfileRequest {
                            username: "nathan".to_string(),
                            display_name: "Nathan".to_string(),
                            password: "password123".to_string(),
                        })
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let session: AuthSession = serde_json::from_slice(&body).unwrap();
        assert_eq!(session.user.display_name, "Nathan");
        assert!(!session.access_token.is_empty());

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/auth/session")
                    .header("authorization", format!("Bearer {}", session.access_token))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let current: CurrentSession = serde_json::from_slice(&body).unwrap();
        assert_eq!(current.user.id, session.user.id);

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/auth/logout")
                    .header("authorization", format!("Bearer {}", session.access_token))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NO_CONTENT);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/auth/session")
                    .header("authorization", format!("Bearer {}", session.access_token))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn default_admin_must_complete_setup() {
        let app = router(AppState::new());
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/auth/login")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&LoginRequest {
                            username: "admin".to_string(),
                            password: "password".to_string(),
                        })
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let session: AuthSession = serde_json::from_slice(&body).unwrap();
        assert!(session.user.must_change_password);
        assert!(session.user.roles.contains(&"admin".to_string()));

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/auth/complete-setup")
                    .header("authorization", format!("Bearer {}", session.access_token))
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&CompleteSetupRequest {
                            username: "nathan-admin".to_string(),
                            display_name: "Nathan Admin".to_string(),
                            password: "better-password".to_string(),
                            confirm_password: "better-password".to_string(),
                        })
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let current: CurrentSession = serde_json::from_slice(&body).unwrap();
        assert_eq!(current.user.username.as_deref(), Some("nathan-admin"));
        assert!(!current.user.must_change_password);

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/auth/login")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&LoginRequest {
                            username: "admin".to_string(),
                            password: "password".to_string(),
                        })
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn admin_summary_requires_admin_role() {
        let app = router(AppState::new());
        let admin_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/auth/login")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&LoginRequest {
                            username: "admin".to_string(),
                            password: "password".to_string(),
                        })
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        let body = admin_response
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes();
        let admin_session: AuthSession = serde_json::from_slice(&body).unwrap();

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/admin/summary")
                    .header(
                        "authorization",
                        format!("Bearer {}", admin_session.access_token),
                    )
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let summary: AdminSummary = serde_json::from_slice(&body).unwrap();
        assert_eq!(summary.users.len(), 1);
        assert!(
            summary
                .role_policies
                .iter()
                .any(|role| role.name == "admin")
        );

        let create_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/admin/users")
                    .header(
                        "authorization",
                        format!("Bearer {}", admin_session.access_token),
                    )
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&CreateAdminUserRequest {
                            username: "managed".to_string(),
                            display_name: "Managed User".to_string(),
                            password: "password123".to_string(),
                            roles: vec!["owner".to_string()],
                            storage_limit_mb: 512,
                            app_scopes: AppToolScope::member(),
                        })
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(create_response.status(), StatusCode::OK);
        let body = create_response
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes();
        let managed: AdminUserSummary = serde_json::from_slice(&body).unwrap();
        assert_eq!(managed.storage_limit_mb, 512);

        let update_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri(format!("/api/v1/admin/users/{}/access", managed.id))
                    .header(
                        "authorization",
                        format!("Bearer {}", admin_session.access_token),
                    )
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&UpdateAdminUserAccessRequest {
                            roles: vec!["admin".to_string(), "owner".to_string()],
                            storage_limit_mb: 1024,
                            app_scopes: AppToolScope::admin(),
                        })
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(update_response.status(), StatusCode::OK);
        let body = update_response
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes();
        let updated: AdminUserSummary = serde_json::from_slice(&body).unwrap();
        assert!(updated.roles.contains(&"admin".to_string()));
        assert_eq!(updated.storage_limit_mb, 1024);

        let reset_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/v1/admin/users/{}/reset-password", managed.id))
                    .header(
                        "authorization",
                        format!("Bearer {}", admin_session.access_token),
                    )
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&ResetAdminUserPasswordRequest {
                            password: "new-password".to_string(),
                            confirm_password: "new-password".to_string(),
                        })
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(reset_response.status(), StatusCode::OK);

        let user_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/auth/register")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&RegisterProfileRequest {
                            username: "member".to_string(),
                            display_name: "Member".to_string(),
                            password: "password123".to_string(),
                        })
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        let body = user_response
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes();
        let member_session: AuthSession = serde_json::from_slice(&body).unwrap();
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/admin/summary")
                    .header(
                        "authorization",
                        format!("Bearer {}", member_session.access_token),
                    )
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn system_version_reports_auth_capabilities() {
        let app = router(AppState::new());
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/system/version")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let payload: SystemVersion = serde_json::from_slice(&body).unwrap();
        assert_eq!(payload.backend_version, env!("CARGO_PKG_VERSION"));
        assert!(payload.auth_required);
        assert!(payload.auth_modes.contains(&"local-password".to_string()));
    }

    #[tokio::test]
    async fn sync_push_replays_note_create() {
        let state = AppState::new();
        let note_id = Uuid::new_v4();
        let document_id = Uuid::new_v4();
        let now = Utc::now();
        let operation = SyncOperation::CreateNote {
            note: Note {
                id: note_id,
                document_id,
                title: "Queued".to_string(),
                path: "/".to_string(),
                tags: vec![],
                owner_id: "local-user".to_string(),
                workspace_id: "default".to_string(),
                created_at: now,
                updated_at: now,
                deleted_at: None,
            },
            document: CrdtDocumentState {
                id: document_id,
                kind: "note".to_string(),
                snapshot: "draft".to_string(),
                updates: vec![],
                version: 0,
                compacted_at: None,
            },
        };
        let app = router(state);
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/sync/push")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&SyncPushRequest {
                            operations: vec![operation],
                        })
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let payload: SyncPushResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(payload.notes.len(), 1);
        assert_eq!(payload.documents.len(), 1);
    }

    #[tokio::test]
    async fn feed_returns_newest_public_activity() {
        let state = AppState::new();
        let mut older = feed_event(
            FeedActivityAction::NoteCreated,
            "notes",
            "Created note \"Older\"".to_string(),
            "note",
            Uuid::new_v4().to_string(),
            "Older".to_string(),
            None,
        );
        older.created_at = Utc::now() - chrono::Duration::minutes(1);
        let mut newer = feed_event(
            FeedActivityAction::NoteCreated,
            "notes",
            "Created note \"Newer\"".to_string(),
            "note",
            Uuid::new_v4().to_string(),
            "Newer".to_string(),
            None,
        );
        newer.created_at = Utc::now();
        state.repo.append_feed_event(older).await.unwrap();
        state.repo.append_feed_event(newer).await.unwrap();

        let app = router(state);
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/feed")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let payload: Vec<FeedActivityEvent> = serde_json::from_slice(&body).unwrap();
        assert_eq!(payload.len(), 2);
        assert_eq!(payload[0].target_label, "Newer");
        assert_eq!(payload[1].target_label, "Older");
    }

    #[tokio::test]
    async fn audio_recording_upload_queues_transcript() {
        let state = AppState::with_transcription(LocalTranscriptionEngine::Disabled);
        let app = router(state);
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/audio/recordings")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&CreateAudioRecordingRequest {
                            title: "Voice note".to_string(),
                            path: "/".to_string(),
                            mime_type: "audio/webm".to_string(),
                            duration_ms: 1200,
                            size_bytes: 64,
                        })
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let recording: AudioRecording = serde_json::from_slice(&body).unwrap();

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/v1/audio/recordings/{}/audio", recording.id))
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&UploadAudioRequest {
                            data_url: "data:audio/webm;base64,AA==".to_string(),
                            mime_type: "audio/webm".to_string(),
                            size_bytes: 64,
                        })
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let response = app
            .oneshot(
                Request::builder()
                    .uri(format!(
                        "/api/v1/audio/recordings/{}/transcript",
                        recording.id
                    ))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let transcript: AudioTranscript = serde_json::from_slice(&body).unwrap();
        assert_eq!(transcript.status, "queued");
        assert_eq!(transcript.segments.len(), 1);
    }

    #[test]
    fn migration_schema_contains_core_tables() {
        let schema = crate::db::initial_schema();
        assert!(schema.contains("create table if not exists notes"));
        assert!(schema.contains("create table if not exists note_folders"));
        assert!(schema.contains("create table if not exists crdt_documents"));
        assert!(schema.contains("create table if not exists sync_tombstones"));
        assert!(schema.contains("create table if not exists feed_activity_events"));
        assert!(schema.contains("create table if not exists feed_favorites"));
        assert!(schema.contains("create table if not exists audio_recordings"));
        assert!(schema.contains("create table if not exists audio_transcript_segments"));
        assert!(
            crate::db::profile_session_schema()
                .contains("create table if not exists auth_sessions")
        );
        assert!(
            crate::db::profile_session_schema()
                .contains("create table if not exists trusted_devices")
        );
    }
}
