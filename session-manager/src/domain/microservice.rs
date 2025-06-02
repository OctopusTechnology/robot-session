use std::collections::HashMap;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MicroserviceInfo {
    pub service_id: String,
    pub endpoint: String,
    pub status: ServiceStatus,
    pub registered_at: DateTime<Utc>,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ServiceStatus {
    Registered,  // 已注册
    Joining,     // 正在加入房间
    Ready,       // 已就绪
    Disconnected, // 已断开
}

impl MicroserviceInfo {
    pub fn new(service_id: String, endpoint: String, metadata: HashMap<String, String>) -> Self {
        Self {
            service_id,
            endpoint,
            status: ServiceStatus::Registered,
            registered_at: Utc::now(),
            metadata,
        }
    }

    pub fn update_status(&mut self, status: ServiceStatus) {
        self.status = status;
    }

    pub fn is_available(&self) -> bool {
        matches!(self.status, ServiceStatus::Registered | ServiceStatus::Ready)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JoinRoomRequest {
    pub room_name: String,
    pub session_id: String,
    pub service_identity: String,
    pub access_token: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JoinRoomResponse {
    pub success: bool,
    pub message: String,
}