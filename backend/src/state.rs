use crate::{
    models::{CrdtUpdate, PresencePeer},
    repository::InMemoryRepository,
    transcription::LocalTranscriptionEngine,
};
use chrono::Utc;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::{RwLock, broadcast};

#[derive(Clone)]
pub struct AppState {
    pub repo: InMemoryRepository,
    pub transcription: LocalTranscriptionEngine,
    presence: Arc<RwLock<HashMap<String, PresenceRoom>>>,
    document_updates: Arc<RwLock<HashMap<String, DocumentUpdateRoom>>>,
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
        Self {
            repo: InMemoryRepository::new(),
            transcription,
            presence: Arc::new(RwLock::new(HashMap::new())),
            document_updates: Arc::new(RwLock::new(HashMap::new())),
        }
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

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
