use crate::domain::SessionStatus;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// 微服务注册 API
#[derive(Debug, Deserialize)]
pub struct RegisterMicroserviceRequest {
    pub service_id: String,
    pub endpoint: String,
    pub metadata: Option<HashMap<String, String>>,
}

#[derive(Debug, Serialize)]
pub struct RegisterMicroserviceResponse {
    pub success: bool,
    pub service_id: String,
    pub message: String,
}

// 会话创建 API
#[derive(Debug, Deserialize)]
pub struct CreateSessionRequest {
    pub user_identity: String,
    pub user_name: Option<String>,
    pub room_name: Option<String>,
    pub metadata: Option<HashMap<String, String>>,
    pub required_services: Option<Vec<String>>,
}

#[derive(Debug, Serialize)]
pub struct CreateSessionResponse {
    pub session_id: String,
    pub room_name: String,
    pub access_token: String,
    pub livekit_url: String,
    pub status: SessionStatus,
}

// 会话状态查询 API
#[derive(Debug, Serialize)]
pub struct SessionStatusResponse {
    pub session_id: String,
    pub room_name: String,
    pub status: SessionStatus,
    pub ready_services: Vec<String>,
    pub pending_services: Vec<String>,
    pub created_at: DateTime<Utc>,
}

// 服务就绪通知 API
#[derive(Debug, Deserialize)]
pub struct ServiceReadyRequest {
    pub service_id: String,
}

#[derive(Debug, Serialize)]
pub struct ServiceReadyResponse {
    pub success: bool,
    pub message: String,
    pub all_services_ready: bool,
}

// 健康检查 API
#[derive(Debug, Serialize)]
pub struct HealthCheckResponse {
    pub status: String,
    pub timestamp: DateTime<Utc>,
    pub version: String,
}

// 错误响应
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub message: String,
    pub timestamp: DateTime<Utc>,
}
