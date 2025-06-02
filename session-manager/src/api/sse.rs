use std::convert::Infallible;
use std::time::Duration;
use axum::{
    extract::{Path, State},
    response::sse::{Event, KeepAlive, Sse},
};
use futures_util::stream::{self, Stream, StreamExt};
use tokio::sync::broadcast;
use crate::{
    api::handlers::AppState,
    events::SessionEvent,
};

/// SSE 端点用于监听会话事件
pub async fn session_events_stream(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    tracing::info!("客户端连接到会话事件流: {}", session_id);
    
    // 获取会话特定的事件流
    let mut receiver = match state.event_bus.get_session_stream(&session_id) {
        Some(receiver) => receiver,
        None => {
            // 如果会话不存在，创建一个空流
            tracing::warn!("会话 {} 不存在，创建空事件流", session_id);
            state.event_bus.create_session_stream(session_id.clone())
        }
    };
    
    // 创建一个异步流来处理广播接收器
    let event_stream = stream::unfold(receiver, |mut rx| async move {
        match rx.recv().await {
            Ok(event) => {
                let event_result = match serde_json::to_string(&event) {
                    Ok(json_data) => {
                        let event_type = match &event {
                            SessionEvent::SessionCreated { .. } => "session_created",
                            SessionEvent::MicroserviceJoined { .. } => "microservice_joined",
                            SessionEvent::ClientJoined { .. } => "client_joined",
                            SessionEvent::SessionReady { .. } => "session_ready",
                            SessionEvent::SessionStatusChanged { .. } => "status_changed",
                            SessionEvent::Error { .. } => "error",
                        };
                        
                        Ok(Event::default()
                            .event(event_type)
                            .data(json_data))
                    }
                    Err(e) => {
                        tracing::error!("序列化事件失败: {}", e);
                        Ok(Event::default()
                            .event("error")
                            .data("serialization_error"))
                    }
                };
                Some((event_result, rx))
            }
            Err(broadcast::error::RecvError::Lagged(_)) => {
                // 处理滞后错误，继续接收
                Some((Ok(Event::default()
                    .event("system")
                    .data("lagged")), rx))
            }
            Err(broadcast::error::RecvError::Closed) => {
                // 通道关闭，结束流
                None
            }
        }
    });
    
    Sse::new(event_stream).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("keep-alive")
    )
}

/// 全局事件流（用于监控所有会话）
pub async fn global_events_stream(
    State(state): State<AppState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    tracing::info!("客户端连接到全局事件流");
    
    let mut receiver = state.event_bus.subscribe_global();
    
    // 创建一个异步流来处理广播接收器
    let event_stream = stream::unfold(receiver, |mut rx| async move {
        match rx.recv().await {
            Ok(event) => {
                let event_result = match serde_json::to_string(&event) {
                    Ok(json_data) => {
                        let event_type = match &event {
                            SessionEvent::SessionCreated { .. } => "session_created",
                            SessionEvent::MicroserviceJoined { .. } => "microservice_joined",
                            SessionEvent::ClientJoined { .. } => "client_joined",
                            SessionEvent::SessionReady { .. } => "session_ready",
                            SessionEvent::SessionStatusChanged { .. } => "status_changed",
                            SessionEvent::Error { .. } => "error",
                        };
                        
                        Ok(Event::default()
                            .event(event_type)
                            .data(json_data))
                    }
                    Err(e) => {
                        tracing::error!("序列化全局事件失败: {}", e);
                        Ok(Event::default()
                            .event("error")
                            .data("serialization_error"))
                    }
                };
                Some((event_result, rx))
            }
            Err(broadcast::error::RecvError::Lagged(_)) => {
                // 处理滞后错误，继续接收
                Some((Ok(Event::default()
                    .event("system")
                    .data("lagged")), rx))
            }
            Err(broadcast::error::RecvError::Closed) => {
                // 通道关闭，结束流
                None
            }
        }
    });
    
    Sse::new(event_stream).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("keep-alive")
    )
}