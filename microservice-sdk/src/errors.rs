use thiserror::Error;

/// Errors that can occur when using the microservice SDK
#[derive(Error, Debug)]
pub enum MicroserviceError {
    #[error("HTTP request failed: {0}")]
    HttpError(#[from] reqwest::Error),

    #[error("JSON serialization/deserialization failed: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("Session manager returned error: {status} - {message}")]
    SessionManagerError { status: u16, message: String },

    #[error("Registration failed: {0}")]
    RegistrationFailed(String),

    #[error("Join room failed: {0}")]
    JoinRoomFailed(String),

    #[error("Notify ready failed: {0}")]
    NotifyReadyFailed(String),

    #[error("Configuration error: {0}")]
    ConfigurationError(String),

    #[error("Timeout waiting for response")]
    Timeout,

    #[error("Invalid response format")]
    InvalidResponse,
}

/// Result type for microservice operations
pub type Result<T> = std::result::Result<T, MicroserviceError>;
