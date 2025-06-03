use crate::{
    api::models::*,
    domain::MicroserviceInfo,
    services::{MicroserviceRegistry, SessionService},
    utils::errors::SessionManagerError,
};
use axum::{extract::State, http::StatusCode, response::Json};
use chrono::Utc;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub session_service: Arc<dyn SessionService>,
    pub microservice_registry: Arc<MicroserviceRegistry>,
    pub config: crate::config::AppConfig,
    pub event_bus: crate::events::EventBus,
}

// 健康检查
pub async fn health_check() -> Json<HealthCheckResponse> {
    Json(HealthCheckResponse {
        status: "healthy".to_string(),
        timestamp: Utc::now(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

// 注册微服务
pub async fn register_microservice(
    State(state): State<AppState>,
    Json(request): Json<RegisterMicroserviceRequest>,
) -> Result<Json<RegisterMicroserviceResponse>, (StatusCode, Json<ErrorResponse>)> {
    let microservice = MicroserviceInfo::new(
        request.service_id.clone(),
        request.endpoint,
        request.metadata.unwrap_or_default(),
    );

    match state
        .microservice_registry
        .register_service(microservice)
        .await
    {
        Ok(_) => Ok(Json(RegisterMicroserviceResponse {
            success: true,
            service_id: request.service_id,
            message: "Microservice registered successfully".to_string(),
        })),
        Err(e) => Err(handle_error(e)),
    }
}

// 创建会话 - 简单同步创建，返回会话信息
pub async fn create_session(
    State(state): State<AppState>,
    Json(request): Json<JoinSessionRequest>,
) -> Result<Json<CreateSessionResponse>, (StatusCode, Json<ErrorResponse>)> {
    // 转换请求类型
    let session_request = crate::services::session_service::JoinSessionRequest {
        user_identity: request.user_identity.clone(),
        user_name: request.user_name,
        room_name: request.room_name,
        metadata: request.metadata,
        required_services: request.required_services,
    };

    // 创建会话
    match state.session_service.create_session(session_request).await {
        Ok((session, access_token)) => {
            let response = CreateSessionResponse {
                session_id: session.id.clone(),
                room_name: session.room_name.clone(),
                access_token,
                livekit_url: state.config.livekit.server_url.clone(),
                status: session.status,
            };

            tracing::info!("Session {} created successfully", session.id);
            Ok(Json(response))
        }
        Err(e) => {
            tracing::error!("Failed to create session: {}", e);
            Err(handle_error(e))
        }
    }
}

// 错误处理辅助函数
fn handle_error(error: SessionManagerError) -> (StatusCode, Json<ErrorResponse>) {
    let (status_code, error_type) = match &error {
        SessionManagerError::SessionNotFound { .. } => (StatusCode::NOT_FOUND, "SessionNotFound"),
        SessionManagerError::InvalidRequest(_) => (StatusCode::BAD_REQUEST, "InvalidRequest"),
        SessionManagerError::MicroserviceJoinTimeout => (StatusCode::REQUEST_TIMEOUT, "Timeout"),
        SessionManagerError::Configuration(_) => {
            (StatusCode::INTERNAL_SERVER_ERROR, "Configuration")
        }
        _ => (StatusCode::INTERNAL_SERVER_ERROR, "InternalError"),
    };

    (
        status_code,
        Json(ErrorResponse {
            error: error_type.to_string(),
            message: error.to_string(),
            timestamp: Utc::now(),
        }),
    )
}
