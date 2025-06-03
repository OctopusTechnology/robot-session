use thiserror::Error;

#[derive(Debug, Error)]
pub enum SessionManagerError {
    #[error("Session not found: {session_id}")]
    SessionNotFound { session_id: String },

    #[error("LiveKit error: {0}")]
    LiveKit(#[from] livekit_api::services::ServiceError),

    #[error("Storage error: {0}")]
    Storage(String),

    #[error("Microservice communication error: {0}")]
    MicroserviceCommunication(#[from] reqwest::Error),

    #[error("Configuration error: {0}")]
    Configuration(String),

    #[error("Timeout waiting for microservices to join")]
    MicroserviceJoinTimeout,

    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    #[error("Internal error: {0}")]
    Internal(#[from] anyhow::Error),
}

pub type Result<T> = std::result::Result<T, SessionManagerError>;
