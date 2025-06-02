use std::sync::Arc;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};
use chrono::Utc;
use crate::{
    api::models::*,
    domain::MicroserviceInfo,
    services::{SessionService, MicroserviceRegistry},
    utils::errors::SessionManagerError,
};

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

    match state.microservice_registry.register_service(microservice).await {
        Ok(_) => Ok(Json(RegisterMicroserviceResponse {
            success: true,
            service_id: request.service_id,
            message: "Microservice registered successfully".to_string(),
        })),
        Err(e) => Err(handle_error(e)),
    }
}

// 创建会话 - 返回 SSE 流直到会话准备就绪
pub async fn create_session(
    State(state): State<AppState>,
    Json(request): Json<JoinSessionRequest>,
) -> axum::response::sse::Sse<impl futures_util::stream::Stream<Item = Result<axum::response::sse::Event, std::convert::Infallible>>> {
    use futures_util::stream::{self, StreamExt};
    use axum::response::sse::{Event, KeepAlive};
    use std::time::Duration;
    
    // 转换请求类型
    let session_request = crate::services::session_service::JoinSessionRequest {
        user_identity: request.user_identity.clone(),
        user_name: request.user_name,
        room_name: request.room_name,
        metadata: request.metadata,
        required_services: request.required_services,
    };

    // 创建会话事件流
    let event_bus = state.event_bus.clone();
    let session_service = state.session_service.clone();
    let config = state.config.clone();
    
    // 定义状态结构
    #[derive(Clone)]
    struct SessionState {
        service: std::sync::Arc<dyn crate::services::SessionService>,
        request: crate::services::session_service::JoinSessionRequest,
        bus: crate::events::EventBus,
        config: crate::config::AppConfig,
        session_created: bool,
        session_id: Option<String>,
    }
    
    let initial_state = SessionState {
        service: session_service,
        request: session_request,
        bus: event_bus,
        config,
        session_created: false,
        session_id: None,
    };
    
    let session_stream = stream::unfold(initial_state, |mut state| async move {
        if !state.session_created {
            // 第一步：创建会话
            match state.service.create_session(state.request.clone()).await {
                Ok((session, access_token)) => {
                    let session_id = session.id.clone();
                    
                    // 创建会话事件流
                    let _receiver = state.bus.create_session_stream(session_id.clone());
                    
                    // 发送会话创建事件
                    state.bus.publish_to_session(&session_id, crate::events::SessionEvent::SessionCreated {
                        session_id: session_id.clone(),
                        room_name: session.room_name.clone(),
                        access_token: access_token.clone(),
                        livekit_url: state.config.livekit.server_url.clone(),
                    });
                    
                    // 返回会话创建事件
                    let event_data = serde_json::json!({
                        "session_id": session_id,
                        "room_name": session.room_name,
                        "access_token": access_token,
                        "livekit_url": state.config.livekit.server_url,
                        "status": session.status
                    });
                    
                    let event = Event::default()
                        .event("session_created")
                        .data(event_data.to_string());
                    
                    state.session_created = true;
                    state.session_id = Some(session_id);
                    
                    Some((Ok(event), state))
                }
                Err(e) => {
                    let error_event = Event::default()
                        .event("error")
                        .data(format!("{{\"error\": \"{}\"}}", e));
                    state.session_created = true;
                    Some((Ok(error_event), state))
                }
            }
        } else if let Some(ref sid) = state.session_id {
            // 检查会话状态
            match state.service.get_session(sid).await {
                Ok(Some(session)) => {
                    if session.is_ready() {
                        // 会话准备就绪，发送最终事件并结束流
                        let event = Event::default()
                            .event("session_ready")
                            .data(format!("{{\"session_id\": \"{}\", \"all_participants_joined\": true}}", sid));
                        state.session_id = None; // 标记结束
                        Some((Ok(event), state))
                    } else {
                        // 会话还未准备就绪，等待一段时间后再检查
                        tokio::time::sleep(Duration::from_millis(500)).await;
                        let event = Event::default()
                            .event("waiting")
                            .data("{\"status\": \"waiting_for_participants\"}");
                        Some((Ok(event), state))
                    }
                }
                Ok(None) => {
                    let error_event = Event::default()
                        .event("error")
                        .data("{\"error\": \"session_not_found\"}");
                    state.session_id = None;
                    Some((Ok(error_event), state))
                }
                Err(e) => {
                    let error_event = Event::default()
                        .event("error")
                        .data(format!("{{\"error\": \"{}\"}}", e));
                    state.session_id = None;
                    Some((Ok(error_event), state))
                }
            }
        } else {
            // 没有会话 ID，结束流
            None
        }
    });

    axum::response::sse::Sse::new(session_stream).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(10))
            .text("keep-alive")
    )
}


// 错误处理辅助函数
fn handle_error(error: SessionManagerError) -> (StatusCode, Json<ErrorResponse>) {
    let (status_code, error_type) = match &error {
        SessionManagerError::SessionNotFound { .. } => (StatusCode::NOT_FOUND, "SessionNotFound"),
        SessionManagerError::InvalidRequest(_) => (StatusCode::BAD_REQUEST, "InvalidRequest"),
        SessionManagerError::MicroserviceJoinTimeout => (StatusCode::REQUEST_TIMEOUT, "Timeout"),
        SessionManagerError::Configuration(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Configuration"),
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