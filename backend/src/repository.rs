use crate::{error::AppResult, models::*};
use async_trait::async_trait;
use chrono::Utc;
use std::{collections::HashMap, sync::Arc};
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
    async fn update_note_metadata(&self, id: Uuid, request: UpdateNoteMetadataRequest) -> AppResult<Note>;
    async fn delete_note(&self, id: Uuid) -> AppResult<()>;
    async fn document(&self, id: Uuid) -> AppResult<CrdtDocumentState>;
    async fn append_document_updates(&self, id: Uuid, updates: Vec<IncomingCrdtUpdate>) -> AppResult<CrdtDocumentState>;
    async fn append_document_update(&self, update: CrdtUpdate) -> AppResult<CrdtDocumentState>;
    async fn envelope(&self) -> AppResult<SyncEnvelope>;
    async fn apply_sync_operation(&self, operation: SyncOperation) -> AppResult<()>;
}

#[derive(Clone, Default)]
pub struct InMemoryRepository {
    inner: Arc<RwLock<RepositoryData>>,
}

#[derive(Default)]
struct RepositoryData {
    notes: HashMap<Uuid, Note>,
    note_folders: HashMap<Uuid, NoteFolder>,
    documents: HashMap<Uuid, CrdtDocumentState>,
    tombstones: HashMap<(String, Uuid), SyncTombstone>,
}

impl InMemoryRepository {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl SuiteRepository for InMemoryRepository {
    async fn apps(&self) -> AppResult<Vec<AppRegistryEntry>> {
        Ok(vec![notes_registry_entry()])
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
        Ok(data.notes.values().cloned().filter(|note| note.deleted_at.is_none()).collect())
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
        Ok(note)
    }

    async fn upsert_note(&self, note: Note) -> AppResult<Note> {
        let mut data = self.inner.write().await;
        data.tombstones.remove(&("notes".to_string(), note.id));
        data.notes.insert(note.id, note.clone());
        Ok(note)
    }

    async fn update_note_metadata(&self, id: Uuid, request: UpdateNoteMetadataRequest) -> AppResult<Note> {
        let mut data = self.inner.write().await;
        let note = data.notes.get_mut(&id).ok_or(crate::error::AppError::NotFound)?;
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
        Ok(note.clone())
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
        Ok(())
    }

    async fn document(&self, id: Uuid) -> AppResult<CrdtDocumentState> {
        let data = self.inner.read().await;
        data.documents.get(&id).cloned().ok_or(crate::error::AppError::NotFound)
    }

    async fn append_document_updates(&self, id: Uuid, updates: Vec<IncomingCrdtUpdate>) -> AppResult<CrdtDocumentState> {
        let mut document = self.document(id).await?;
        for incoming in updates {
            if document
                .updates
                .iter()
                .any(|update| update.client_id == incoming.client_id && update.sequence == incoming.sequence)
            {
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
        Ok(document)
    }

    async fn append_document_update(&self, update: CrdtUpdate) -> AppResult<CrdtDocumentState> {
        let mut data = self.inner.write().await;
        let document = data
            .documents
            .entry(update.document_id)
            .or_insert_with(|| CrdtDocumentState {
                id: update.document_id,
                kind: "note".to_string(),
                snapshot: String::new(),
                updates: Vec::new(),
                version: 0,
                compacted_at: None,
            });
        if document.updates.iter().any(|existing| existing.id == update.id) {
            return Ok(document.clone());
        }
        document.updates.push(update);
        document.version += 1;
        compact_if_needed(document);
        Ok(document.clone())
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
            cursors: SyncCursorSet { generated_at: Utc::now() },
            apps: vec![notes_registry_entry()],
            note_folders: data
                .note_folders
                .values()
                .cloned()
                .filter(|folder| folder.deleted_at.is_none())
                .collect(),
            notes: data.notes.values().cloned().filter(|note| note.deleted_at.is_none()).collect(),
            document_updates: documents.iter().flat_map(|document| document.updates.iter().cloned()).collect(),
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
                data.tombstones.remove(&("documents".to_string(), document.id));
                data.documents.insert(document.id, document);
                data.notes.insert(note.id, note);
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
                Ok(())
            }
            SyncOperation::CreateNoteFolder { folder } => {
                let mut data = self.inner.write().await;
                data.tombstones.remove(&("noteFolders".to_string(), folder.id));
                data.note_folders.insert(folder.id, folder);
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
                Ok(())
            }
            SyncOperation::AppendDocumentUpdate { update } => self.append_document_update(update).await.map(|_| ()),
        }
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

fn notes_registry_entry() -> AppRegistryEntry {
    AppRegistryEntry {
        id: "notes".to_string(),
        name: "Notes".to_string(),
        route: "/notes".to_string(),
        standalone_route: "/".to_string(),
        capabilities: vec![AppCapability::Offline, AppCapability::RemoteSave, AppCapability::Collaboration],
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
}
