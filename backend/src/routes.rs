use crate::{
    error::AppResult,
    models::*,
    repository::SuiteRepository,
    state::AppState,
};
use axum::{
    Json, Router,
    extract::{
        Path, Query, State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    http::StatusCode,
    response::IntoResponse,
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
        .route("/api/v1/apps", get(list_apps))
        .route("/api/v1/sync/bootstrap", get(sync_bootstrap))
        .route("/api/v1/sync/pull", post(sync_pull))
        .route("/api/v1/sync/push", post(sync_push))
        .route("/api/v1/note-folders", get(list_note_folders))
        .route("/api/v1/notes", get(list_notes).post(create_note))
        .route("/api/v1/notes/{id}/metadata", patch(update_note_metadata))
        .route("/api/v1/notes/{id}", delete(delete_note))
        .route("/api/v1/documents/{id}", get(get_document))
        .route("/api/v1/documents/{id}/updates", post(append_document_updates))
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

async fn list_apps(State(state): State<AppState>) -> AppResult<Json<Vec<AppRegistryEntry>>> {
    Ok(Json(state.repo.apps().await?))
}

async fn sync_bootstrap(State(state): State<AppState>) -> AppResult<Json<SyncEnvelope>> {
    Ok(Json(state.repo.envelope().await?))
}

async fn sync_pull(State(state): State<AppState>, Json(_payload): Json<SyncPullRequest>) -> AppResult<Json<SyncEnvelope>> {
    Ok(Json(state.repo.envelope().await?))
}

async fn sync_push(State(state): State<AppState>, Json(payload): Json<SyncPushRequest>) -> AppResult<Json<SyncPushResponse>> {
    let mut accepted = Vec::new();
    for operation in payload.operations {
        accepted.push(operation_id(&operation));
        let update_to_broadcast = match &operation {
            SyncOperation::AppendDocumentUpdate { update } => Some(update.clone()),
            _ => None,
        };
        state.repo.apply_sync_operation(operation).await?;
        if let Some(update) = update_to_broadcast {
            state.broadcast_document_update(&update.document_id.to_string(), update).await;
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

async fn create_note(State(state): State<AppState>, Json(payload): Json<CreateNoteRequest>) -> AppResult<Json<Note>> {
    Ok(Json(state.repo.create_note(payload).await?))
}

async fn update_note_metadata(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
    Json(payload): Json<UpdateNoteMetadataRequest>,
) -> AppResult<Json<Note>> {
    Ok(Json(state.repo.update_note_metadata(id, payload).await?))
}

async fn delete_note(Path(id): Path<Uuid>, State(state): State<AppState>) -> AppResult<StatusCode> {
    state.repo.delete_note(id).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn get_document(Path(id): Path<Uuid>, State(state): State<AppState>) -> AppResult<Json<CrdtDocumentState>> {
    Ok(Json(state.repo.document(id).await?))
}

async fn append_document_updates(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
    Json(payload): Json<AppendDocumentUpdatesRequest>,
) -> AppResult<Json<CrdtDocumentState>> {
    let incoming_count = payload.updates.len();
    let document = state.repo.append_document_updates(id, payload.updates).await?;
    for update in document.updates.iter().rev().take(incoming_count) {
        state.broadcast_document_update(&id.to_string(), update.clone()).await;
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
    let client_id = query.client_id.unwrap_or_else(|| Uuid::new_v4().to_string());
    upgrade.on_upgrade(move |socket| handle_presence_socket(socket, state, document_id, client_id))
}

async fn handle_presence_socket(socket: WebSocket, state: AppState, document_id: String, client_id: String) {
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
                let cursor = payload.get("cursor").and_then(|value| value.as_u64()).map(|value| value as usize);
                state.update_presence_cursor(&document_id, &client_id, cursor).await;
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
    let client_id = query.client_id.unwrap_or_else(|| Uuid::new_v4().to_string());
    upgrade.on_upgrade(move |socket| handle_document_socket(socket, state, document_id, client_id))
}

async fn handle_document_socket(socket: WebSocket, state: AppState, document_id: String, client_id: String) {
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
                if state.repo.append_document_update(update.clone()).await.is_ok() {
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
        SyncOperation::AppendDocumentUpdate { update } => format!("append-document-update:{}", update.id),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, http::{Request, StatusCode}};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    #[tokio::test]
    async fn health_route_works() {
        let app = router(AppState::new());
        let response = app
            .oneshot(Request::builder().uri("/health").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
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
                    .body(Body::from(serde_json::to_vec(&SyncPushRequest { operations: vec![operation] }).unwrap()))
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

    #[test]
    fn migration_schema_contains_core_tables() {
        let schema = crate::db::initial_schema();
        assert!(schema.contains("create table if not exists notes"));
        assert!(schema.contains("create table if not exists note_folders"));
        assert!(schema.contains("create table if not exists crdt_documents"));
        assert!(schema.contains("create table if not exists sync_tombstones"));
    }
}
