use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// Configuration for the microservice SDK
#[derive(Debug, Clone)]
pub struct MicroserviceConfig {
    /// Session manager base URL (e.g., "http://localhost:8080")
    pub session_manager_url: String,
    /// Unique identifier for this microservice
    pub service_id: String,
    /// HTTP endpoint where this microservice can be reached
    pub service_endpoint: String,
    /// Optional metadata about this microservice
    pub metadata: HashMap<String, String>,
    /// Timeout for HTTP requests (in seconds)
    pub request_timeout_secs: u64,
}

impl MicroserviceConfig {
    pub fn new(
        session_manager_url: String,
        service_id: String,
        service_endpoint: String,
    ) -> Self {
        Self {
            session_manager_url,
            service_id,
            service_endpoint,
            metadata: HashMap::new(),
            request_timeout_secs: 30,
        }
    }

    pub fn with_metadata(mut self, metadata: HashMap<String, String>) -> Self {
        self.metadata = metadata;
        self
    }

    pub fn with_timeout(mut self, timeout_secs: u64) -> Self {
        self.request_timeout_secs = timeout_secs;
        self
    }
}

/// Request to register a microservice with the session manager
#[derive(Debug, Serialize)]
pub struct RegisterMicroserviceRequest {
    pub service_id: String,
    pub endpoint: String,
    pub metadata: Option<HashMap<String, String>>,
}

/// Response from registering a microservice
#[derive(Debug, Deserialize)]
pub struct RegisterMicroserviceResponse {
    pub success: bool,
    pub service_id: String,
    pub message: String,
}

/// Request to join a LiveKit room (sent by session manager to microservice)
#[derive(Debug, Clone, Deserialize)]
pub struct JoinRoomRequest {
    pub room_name: String,
    pub session_id: String,
    pub service_identity: String,
    pub access_token: String,
    pub livekit_url: String,
}

/// Response when joining a room
#[derive(Debug, Serialize)]
pub struct JoinRoomResponse {
    pub success: bool,
    pub message: String,
    pub session_id: String,
    pub service_id: String,
}

/// Request to notify that the service is ready
#[derive(Debug, Serialize)]
pub struct ServiceReadyRequest {
    pub service_id: String,
}

/// Response from notifying service ready
#[derive(Debug, Deserialize)]
pub struct ServiceReadyResponse {
    pub success: bool,
    pub message: String,
    pub all_services_ready: bool,
}

/// Error response from session manager
#[derive(Debug, Deserialize)]
pub struct ErrorResponse {
    pub error: String,
    pub message: String,
    pub timestamp: String,
}