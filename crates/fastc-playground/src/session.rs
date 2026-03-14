use dashmap::DashMap;
use std::time::{Duration, Instant};
use tokio::sync::broadcast;
use uuid::Uuid;

use crate::executor::ExecutionMessage;

/// A user session
#[derive(Clone)]
pub struct Session {
    pub id: Uuid,
    pub code: String,
    pub created_at: Instant,
    pub last_active: Instant,
}

impl Session {
    pub fn new() -> Self {
        let now = Instant::now();
        Self {
            id: Uuid::new_v4(),
            code: String::new(),
            created_at: now,
            last_active: now,
        }
    }

    pub fn touch(&mut self) {
        self.last_active = Instant::now();
    }
}

impl Default for Session {
    fn default() -> Self {
        Self::new()
    }
}

/// In-memory session store with broadcast channels for output
pub struct SessionStore {
    sessions: DashMap<Uuid, Session>,
    /// Broadcast channels for each running session
    channels: DashMap<Uuid, broadcast::Sender<ExecutionMessage>>,
}

impl SessionStore {
    pub fn new() -> Self {
        Self {
            sessions: DashMap::new(),
            channels: DashMap::new(),
        }
    }

    /// Create a new session and return its ID
    pub fn create(&self) -> Uuid {
        let session = Session::new();
        let id = session.id;
        self.sessions.insert(id, session);
        id
    }

    /// Get a session by ID
    pub fn get(&self, id: Uuid) -> Option<Session> {
        self.sessions.get(&id).map(|s| s.clone())
    }

    /// Update the code for a session
    pub fn update_code(&self, id: Uuid, code: String) {
        if let Some(mut session) = self.sessions.get_mut(&id) {
            session.code = code;
            session.touch();
        }
    }

    /// Touch a session to update its last active time
    pub fn touch(&self, id: Uuid) {
        if let Some(mut session) = self.sessions.get_mut(&id) {
            session.touch();
        }
    }

    /// Remove expired sessions
    pub fn cleanup_expired(&self, max_age: Duration) {
        let now = Instant::now();
        self.sessions.retain(|_, session| {
            now.duration_since(session.last_active) < max_age
        });
        // Also clean up old channels
        self.channels.retain(|id, _| self.sessions.contains_key(id));
    }

    /// Get or create a session
    pub fn get_or_create(&self, id: Option<Uuid>) -> Session {
        if let Some(id) = id {
            if let Some(session) = self.get(id) {
                return session;
            }
        }
        let session = Session::new();
        let cloned = session.clone();
        self.sessions.insert(session.id, session);
        cloned
    }

    /// Create a broadcast channel for a session and return the sender
    pub fn create_channel(&self, id: Uuid) -> broadcast::Sender<ExecutionMessage> {
        let (tx, _) = broadcast::channel(100);
        self.channels.insert(id, tx.clone());
        tx
    }

    /// Subscribe to a session's broadcast channel
    pub fn subscribe(&self, id: Uuid) -> Option<broadcast::Receiver<ExecutionMessage>> {
        self.channels.get(&id).map(|tx| tx.subscribe())
    }

    /// Remove a session's channel
    pub fn remove_channel(&self, id: Uuid) {
        self.channels.remove(&id);
    }
}

impl Default for SessionStore {
    fn default() -> Self {
        Self::new()
    }
}
