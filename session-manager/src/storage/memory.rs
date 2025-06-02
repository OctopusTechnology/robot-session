use std::sync::Arc;
use async_trait::async_trait;
use dashmap::DashMap;
use crate::{
    domain::Session,
    storage::SessionStorage,
    utils::errors::Result,
};

#[derive(Debug)]
pub struct MemoryStorage {
    sessions: Arc<DashMap<String, Session>>,
}

impl MemoryStorage {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(DashMap::new()),
        }
    }
}

#[async_trait]
impl SessionStorage for MemoryStorage {
    async fn save_session(&self, session: &Session) -> Result<()> {
        self.sessions.insert(session.id.clone(), session.clone());
        Ok(())
    }

    async fn get_session(&self, session_id: &str) -> Result<Option<Session>> {
        Ok(self.sessions.get(session_id).map(|entry| entry.clone()))
    }

    async fn update_session(&self, session: &Session) -> Result<()> {
        self.sessions.insert(session.id.clone(), session.clone());
        Ok(())
    }

    async fn delete_session(&self, session_id: &str) -> Result<()> {
        self.sessions.remove(session_id);
        Ok(())
    }

    async fn list_sessions(&self) -> Result<Vec<Session>> {
        Ok(self.sessions.iter().map(|entry| entry.clone()).collect())
    }
}

impl Default for MemoryStorage {
    fn default() -> Self {
        Self::new()
    }
}