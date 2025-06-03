pub mod memory;

use crate::{domain::Session, utils::errors::Result};
use async_trait::async_trait;

#[async_trait]
pub trait SessionStorage: Send + Sync {
    async fn save_session(&self, session: &Session) -> Result<()>;
    async fn get_session(&self, session_id: &str) -> Result<Option<Session>>;
    async fn update_session(&self, session: &Session) -> Result<()>;
    async fn delete_session(&self, session_id: &str) -> Result<()>;
    async fn list_sessions(&self) -> Result<Vec<Session>>;
}
