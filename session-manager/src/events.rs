use crate::domain::SessionStatus;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::broadcast;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SessionEvent {
    SessionCreated {
        session_id: String,
        room_name: String,
        access_token: String,
        livekit_url: String,
    },
    MicroserviceJoined {
        session_id: String,
        service_id: String,
    },
    ClientJoined {
        session_id: String,
        user_identity: String,
    },
    SessionReady {
        session_id: String,
        all_participants_joined: bool,
    },
    SessionStatusChanged {
        session_id: String,
        status: SessionStatus,
    },
    Error {
        session_id: String,
        message: String,
    },
}

pub type EventSender = broadcast::Sender<SessionEvent>;
pub type EventReceiver = broadcast::Receiver<SessionEvent>;

#[derive(Clone, Debug)]
pub struct EventBus {
    // 全局事件广播
    global_sender: EventSender,
    // 每个会话的事件广播
    session_senders: Arc<DashMap<String, EventSender>>,
}

impl EventBus {
    pub fn new() -> Self {
        let (global_sender, _) = broadcast::channel(1000);
        Self {
            global_sender,
            session_senders: Arc::new(DashMap::new()),
        }
    }

    /// 创建会话特定的事件流
    pub fn create_session_stream(&self, session_id: String) -> EventReceiver {
        let (sender, receiver) = broadcast::channel(100);
        self.session_senders.insert(session_id, sender);
        receiver
    }

    /// 获取会话特定的事件流
    pub fn get_session_stream(&self, session_id: &str) -> Option<EventReceiver> {
        self.session_senders
            .get(session_id)
            .map(|sender| sender.subscribe())
    }

    /// 发布事件到特定会话
    pub fn publish_to_session(&self, session_id: &str, event: SessionEvent) {
        if let Some(sender) = self.session_senders.get(session_id) {
            let _ = sender.send(event.clone());
        }
        // 同时发布到全局流
        let _ = self.global_sender.send(event);
    }

    /// 发布全局事件
    pub fn publish_global(&self, event: SessionEvent) {
        let _ = self.global_sender.send(event);
    }

    /// 发布参与者加入事件
    pub async fn publish_participant_joined(
        &self,
        session_id: &str,
        participant_identity: &str,
    ) -> Result<(), crate::utils::errors::SessionManagerError> {
        let event = if participant_identity.starts_with("session-manager-") {
            // 忽略会话管理器自己的加入事件
            return Ok(());
        } else if participant_identity.contains("user") || !participant_identity.contains("service")
        {
            SessionEvent::ClientJoined {
                session_id: session_id.to_string(),
                user_identity: participant_identity.to_string(),
            }
        } else {
            SessionEvent::MicroserviceJoined {
                session_id: session_id.to_string(),
                service_id: participant_identity.to_string(),
            }
        };

        self.publish_to_session(session_id, event);
        Ok(())
    }

    /// 发布参与者离开事件
    pub async fn publish_participant_left(
        &self,
        session_id: &str,
        participant_identity: &str,
    ) -> Result<(), crate::utils::errors::SessionManagerError> {
        // 可以根据需要添加参与者离开的事件类型
        tracing::info!(
            "Participant {} left session {}",
            participant_identity,
            session_id
        );
        Ok(())
    }

    /// 清理会话事件流
    pub fn cleanup_session(&self, session_id: &str) {
        self.session_senders.remove(session_id);
    }

    /// 获取全局事件流
    pub fn subscribe_global(&self) -> EventReceiver {
        self.global_sender.subscribe()
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}
