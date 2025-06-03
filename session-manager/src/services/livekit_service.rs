use std::{collections::HashMap, sync::Arc, time::Duration};
use livekit_api::{
    access_token::{AccessToken, VideoGrants},
    services::room::{CreateRoomOptions, RoomClient},
};
use livekit::prelude::*;
use tokio::sync::{mpsc, RwLock};
use crate::{
    config::LiveKitConfig,
    events::EventBus,
    utils::errors::{Result, SessionManagerError},
};

#[derive(Debug)]
pub struct LiveKitService {
    room_client: RoomClient,
    config: LiveKitConfig,
    room_connections: Arc<RwLock<HashMap<String, RoomConnection>>>,
    event_bus: Arc<EventBus>,
}

#[derive(Debug)]
struct RoomConnection {
    room: Room,
    _event_handle: tokio::task::JoinHandle<()>,
}

impl LiveKitService {
    pub fn new(config: LiveKitConfig, event_bus: Arc<EventBus>) -> Self {
        // RoomClient expects HTTP URL for API calls - ensure we use HTTP scheme
        let api_url = if config.server_url.starts_with("ws://") {
            config.server_url.replace("ws://", "http://")
        } else if config.server_url.starts_with("wss://") {
            config.server_url.replace("wss://", "https://")
        } else {
            // Already HTTP/HTTPS, use as-is
            config.server_url.clone()
        };
        
        let room_client = RoomClient::with_api_key(
            &api_url,
            &config.api_key,
            &config.api_secret,
        );
        
        Self {
            room_client,
            config,
            room_connections: Arc::new(RwLock::new(HashMap::new())),
            event_bus,
        }
    }
    
    pub async fn create_room(&self, room_name: &str) -> Result<()> {
        let options = CreateRoomOptions {
            empty_timeout: 300, // 5 minutes
            max_participants: 50,
            ..Default::default()
        };
        
        self.room_client.create_room(room_name, options).await
            .map_err(|e| SessionManagerError::LiveKit(e))?;
        
        tracing::info!("Created LiveKit room: {}", room_name);
        Ok(())
    }
    
    pub async fn generate_access_token(
        &self,
        identity: &str,
        room_name: &str,
        grants: Option<VideoGrants>,
    ) -> Result<String> {
        let video_grants = grants.unwrap_or(VideoGrants {
            room_join: true,
            room: room_name.to_string(),
            can_publish: true,
            can_subscribe: true,
            can_publish_data: true,
            ..Default::default()
        });
        
        let token = AccessToken::with_api_key(&self.config.api_key, &self.config.api_secret)
            .with_identity(identity)
            .with_grants(video_grants)
            .with_ttl(Duration::from_secs(3600 * 6)) // 6 hours
            .to_jwt()
            .map_err(|e| SessionManagerError::Internal(e.into()))?;
        
        tracing::debug!("Generated access token for identity: {}", identity);
        Ok(token)
    }
    
    pub async fn delete_room(&self, room_name: &str) -> Result<()> {
        self.room_client.delete_room(room_name).await
            .map_err(|e| SessionManagerError::LiveKit(e))?;
        
        tracing::info!("Deleted LiveKit room: {}", room_name);
        Ok(())
    }
    
    pub async fn join_room_as_manager(&self, room_name: &str, session_id: &str) -> Result<()> {
        // 检查是否已经连接到这个房间
        if self.room_connections.read().await.contains_key(room_name) {
            tracing::debug!("Already connected to room: {}", room_name);
            return Ok(());
        }

        // 会话管理器作为特殊参与者加入房间来管理生命周期
        let manager_identity = format!("session-manager-{}", session_id);
        let video_grants = VideoGrants {
            room_join: true,
            room: room_name.to_string(),
            room_admin: true, // 管理员权限
            can_publish: true, // 允许发布以建立连接
            can_subscribe: true, // 允许订阅以建立连接
            hidden: true, // 对其他参与者隐藏
            ..Default::default()
        };
        
        let token = AccessToken::with_api_key(&self.config.api_key, &self.config.api_secret)
            .with_identity(&manager_identity)
            .with_grants(video_grants)
            .with_ttl(Duration::from_secs(3600 * 24)) // 24 hours
            .to_jwt()
            .map_err(|e| SessionManagerError::Internal(e.into()))?;
        
        // 实际连接到 LiveKit 房间 - Room::connect 需要 WebSocket URL
        let ws_url = if self.config.server_url.starts_with("http://") {
            self.config.server_url.replace("http://", "ws://")
        } else if self.config.server_url.starts_with("https://") {
            self.config.server_url.replace("https://", "wss://")
        } else {
            // 如果已经是 ws:// 或 wss://，直接使用
            self.config.server_url.clone()
        };
        
        let (room, mut event_rx) = Room::connect(&ws_url, &token, RoomOptions::default())
            .await
            .map_err(|e| SessionManagerError::Internal(e.into()))?;
        
        tracing::info!("Session manager connected to room: {} as {}", room_name, manager_identity);
        
        // 启动事件监听任务
        let event_bus = self.event_bus.clone();
        let room_name_clone = room_name.to_string();
        let session_id_clone = session_id.to_string();
        
        let event_handle = tokio::spawn(async move {
            while let Some(event) = event_rx.recv().await {
                Self::handle_room_event(event, &event_bus, &room_name_clone, &session_id_clone).await;
            }
            tracing::info!("Room event listener stopped for room: {}", room_name_clone);
        });
        
        // 存储房间连接
        let room_connection = RoomConnection {
            room,
            _event_handle: event_handle,
        };
        
        self.room_connections.write().await.insert(room_name.to_string(), room_connection);
        
        Ok(())
    }
    
    async fn handle_room_event(
        event: RoomEvent,
        event_bus: &Arc<EventBus>,
        room_name: &str,
        session_id: &str,
    ) {
        match event {
            RoomEvent::ParticipantConnected(participant) => {
                let identity = participant.identity().to_string();
                tracing::info!("Participant joined room {}: {}", room_name, identity);
                
                // 发布参与者加入事件
                if let Err(e) = event_bus.publish_participant_joined(session_id, &identity).await {
                    tracing::error!("Failed to publish participant joined event: {}", e);
                }
            }
            RoomEvent::ParticipantDisconnected(participant) => {
                let identity = participant.identity().to_string();
                tracing::info!("Participant left room {}: {}", room_name, identity);
                
                // 发布参与者离开事件
                if let Err(e) = event_bus.publish_participant_left(session_id, &identity).await {
                    tracing::error!("Failed to publish participant left event: {}", e);
                }
            }
            RoomEvent::Connected { .. } => {
                tracing::info!("Session manager successfully connected to room: {}", room_name);
            }
            RoomEvent::Disconnected { reason } => {
                tracing::warn!("Session manager disconnected from room {}: {:?}", room_name, reason);
            }
            _ => {
                // 忽略其他事件
            }
        }
    }
    
    pub async fn leave_room(&self, room_name: &str) -> Result<()> {
        if let Some(connection) = self.room_connections.write().await.remove(room_name) {
            if let Err(e) = connection.room.close().await {
                tracing::error!("Failed to close room connection for {}: {}", room_name, e);
            }
            tracing::info!("Session manager left room: {}", room_name);
        }
        Ok(())
    }
}