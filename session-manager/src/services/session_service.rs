use std::sync::Arc;
use async_trait::async_trait;
use uuid::Uuid;
use serde::{Deserialize, Serialize};
use crate::{
    config::MicroserviceConfig,
    domain::{JoinRoomRequest, Session, SessionStatus},
    services::{LiveKitService, MicroserviceRegistry},
    storage::SessionStorage,
    utils::errors::{Result, SessionManagerError}
};

#[async_trait]
pub trait SessionService: Send + Sync {
    async fn create_session(&self, request: JoinSessionRequest) -> Result<(Session, String)>;
    async fn get_session(&self, session_id: &str) -> Result<Option<Session>>;
    async fn update_session_status(&self, session_id: &str, status: SessionStatus) -> Result<()>;
    async fn terminate_session(&self, session_id: &str) -> Result<()>;
    async fn notify_service_ready(&self, session_id: &str, service_id: &str) -> Result<bool>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JoinSessionRequest {
    pub user_identity: String,
    pub user_name: Option<String>,
    pub room_name: Option<String>,
    pub metadata: Option<std::collections::HashMap<String, String>>,
    pub required_services: Option<Vec<String>>,
}

pub struct SessionServiceImpl {
    storage: Arc<dyn SessionStorage>,
    livekit_service: Arc<LiveKitService>,
    microservice_registry: Arc<MicroserviceRegistry>,
    config: MicroserviceConfig,
    participant_tracker: crate::events::SessionParticipantTracker,
}

impl SessionServiceImpl {
    pub fn new(
        storage: Arc<dyn SessionStorage>,
        livekit_service: Arc<LiveKitService>,
        microservice_registry: Arc<MicroserviceRegistry>,
        config: MicroserviceConfig,
        event_bus: crate::events::EventBus,
    ) -> Self {
        let participant_tracker = crate::events::SessionParticipantTracker::new(event_bus);
        Self {
            storage,
            livekit_service,
            microservice_registry,
            config,
            participant_tracker,
        }
    }

    async fn notify_microservices_to_join(&self, session: &Session) -> Result<()> {
        for service in &session.registered_microservices {
            // 为每个微服务生成访问令牌
            let access_token = self.livekit_service.generate_access_token(
                &service.service_id,
                &session.room_name,
                None
            ).await?;
            
            let join_request = JoinRoomRequest {
                room_name: session.room_name.clone(),
                session_id: session.id.clone(),
                service_identity: service.service_id.clone(),
                access_token,
            };
            
            // 异步通知微服务加入房间
            let service_endpoint = service.endpoint.clone();
            let session_id = session.id.clone();
            let service_id = service.service_id.clone();
            
            tokio::spawn(async move {
                if let Err(e) = Self::notify_service_join(service_endpoint, join_request).await {
                    tracing::error!("Failed to notify service {} to join room: {}", service_id, e);
                }
            });
        }
        Ok(())
    }

    async fn notify_service_join(endpoint: String, request: JoinRoomRequest) -> Result<()> {
        let client = reqwest::Client::new();
        let url = format!("{}/join-room", endpoint);
        
        let response = client
            .post(&url)
            .json(&request)
            .send()
            .await?;

        if response.status().is_success() {
            tracing::info!("Successfully notified service at {} to join room", endpoint);
        } else {
            tracing::error!("Failed to notify service at {}: {}", endpoint, response.status());
        }

        Ok(())
    }
}

#[async_trait]
impl SessionService for SessionServiceImpl {
    async fn create_session(&self, request: JoinSessionRequest) -> Result<(Session, String)> {
        // 1. 生成会话 ID 和房间名
        let session_id = Uuid::new_v4().to_string();
        let room_name = request.room_name.unwrap_or_else(|| format!("room-{}", session_id));
        
        tracing::info!("Creating session {} for room {}", session_id, room_name);
        
        // 2. 创建 LiveKit 房间
        self.livekit_service.create_room(&room_name).await?;
        
        // 3. 会话管理器作为参与者加入房间来管理会话生命周期
        self.livekit_service.join_room_as_manager(&room_name, &session_id).await?;
        
        // 4. 获取已注册的微服务
        let registered_services = if let Some(required_service_ids) = request.required_services {
            // 如果指定了特定的微服务 ID，则获取这些微服务
            self.microservice_registry
                .get_services_by_ids(&required_service_ids).await?
        } else {
            // 如果没有指定，则获取所有可用的微服务
            self.microservice_registry
                .get_all_available_services().await?
        };
        
        // 5. 创建会话对象
        let mut session = Session::new(
            session_id,
            room_name.clone(),
            request.metadata.unwrap_or_default(),
        );
        
        // 添加微服务到会话
        for service in registered_services {
            session.add_microservice(service);
        }
        
        // 6. 保存会话
        self.storage.save_session(&session).await?;
        
        // 7. 通知微服务加入房间
        self.notify_microservices_to_join(&session).await?;
        
        // 8. 生成用户访问令牌
        let access_token = self.livekit_service.generate_access_token(
            &request.user_identity,
            &room_name,
            None
        ).await?;
        
        // 9. 更新状态为等待微服务
        session.update_status(SessionStatus::WaitingForServices);
        self.storage.update_session(&session).await?;
        
        tracing::info!("Session {} created successfully", session.id);
        Ok((session, access_token))
    }

    async fn get_session(&self, session_id: &str) -> Result<Option<Session>> {
        self.storage.get_session(session_id).await
    }

    async fn update_session_status(&self, session_id: &str, status: SessionStatus) -> Result<()> {
        if let Some(mut session) = self.storage.get_session(session_id).await? {
            session.update_status(status);
            self.storage.update_session(&session).await?;
            tracing::info!("Updated session {} status to {:?}", session_id, session.status);
        }
        Ok(())
    }

    async fn terminate_session(&self, session_id: &str) -> Result<()> {
        if let Some(mut session) = self.storage.get_session(session_id).await? {
            session.update_status(SessionStatus::Terminating);
            self.storage.update_session(&session).await?;
            
            // 会话管理器离开房间
            self.livekit_service.leave_room(&session.room_name).await?;
            
            // 删除 LiveKit 房间
            self.livekit_service.delete_room(&session.room_name).await?;
            
            // 标记为已终止
            session.update_status(SessionStatus::Terminated);
            self.storage.update_session(&session).await?;
            
            tracing::info!("Session {} terminated", session_id);
        }
        Ok(())
    }

    async fn notify_service_ready(&self, session_id: &str, service_id: &str) -> Result<bool> {
        if let Some(mut session) = self.storage.get_session(session_id).await? {
            let was_ready = session.mark_service_ready(service_id);
            
            if was_ready {
                self.storage.update_session(&session).await?;
                tracing::info!("Service {} marked as ready for session {}", service_id, session_id);
                
                // 检查是否所有服务都已就绪
                if session.is_ready() {
                    tracing::info!("All services ready for session {}", session_id);
                }
            }
            
            Ok(was_ready)
        } else {
            Err(SessionManagerError::SessionNotFound { 
                session_id: session_id.to_string() 
            })
        }
    }
}
