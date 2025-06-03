use crate::{errors::Result, models::JoinRoomRequest};
use async_trait::async_trait;

/// Trait that microservices must implement to handle session manager requests
#[async_trait]
pub trait MicroserviceHandler: Send + Sync {
    /// Called when the session manager requests this microservice to join a room
    ///
    /// The microservice should:
    /// 1. Connect to the LiveKit room using the provided access token
    /// 2. Set up any necessary resources
    /// 3. Return Ok(()) when ready, or Err() if failed
    async fn handle_join_room(&self, request: JoinRoomRequest) -> Result<()>;

    /// Called when the microservice should clean up and leave the room
    ///
    /// This is optional - microservices can implement cleanup logic here
    async fn handle_leave_room(&self, session_id: &str, room_name: &str) -> Result<()> {
        tracing::info!("Leaving room {} for session {}", room_name, session_id);
        Ok(())
    }

    /// Called to check if the microservice is healthy
    ///
    /// This is optional - microservices can implement health check logic here
    async fn health_check(&self) -> Result<()> {
        Ok(())
    }
}
