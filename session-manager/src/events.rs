use std::sync::Arc;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use dashmap::DashMap;
use crate::domain::SessionStatus;

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
        if let Some(sender) = self.session_senders.get(session_id) {
            Some(sender.subscribe())
        } else {
            None
        }
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
    pub async fn publish_participant_joined(&self, session_id: &str, participant_identity: &str) -> Result<(), crate::utils::errors::SessionManagerError> {
        let event = if participant_identity.starts_with("session-manager-") {
            // 忽略会话管理器自己的加入事件
            return Ok(());
        } else if participant_identity.contains("user") || !participant_identity.contains("service") {
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
    pub async fn publish_participant_left(&self, session_id: &str, participant_identity: &str) -> Result<(), crate::utils::errors::SessionManagerError> {
        // 可以根据需要添加参与者离开的事件类型
        tracing::info!("Participant {} left session {}", participant_identity, session_id);
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

/// 会话参与者跟踪器
#[derive(Clone)]
pub struct SessionParticipantTracker {
    // session_id -> (expected_participants, joined_participants)
    sessions: Arc<DashMap<String, (Vec<String>, Vec<String>)>>,
    event_bus: EventBus,
}

impl SessionParticipantTracker {
    pub fn new(event_bus: EventBus) -> Self {
        Self {
            sessions: Arc::new(DashMap::new()),
            event_bus,
        }
    }

    /// 初始化会话跟踪
    pub fn initialize_session(&self, session_id: String, expected_participants: Vec<String>) {
        self.sessions.insert(session_id.clone(), (expected_participants, Vec::new()));
        tracing::info!("初始化会话跟踪: {}", session_id);
    }

    /// 标记参与者已加入
    pub fn mark_participant_joined(&self, session_id: &str, participant_id: String) -> bool {
        if let Some(mut entry) = self.sessions.get_mut(session_id) {
            let (expected, joined) = entry.value_mut();
            
            if expected.contains(&participant_id) && !joined.contains(&participant_id) {
                joined.push(participant_id.clone());
                
                // 发布参与者加入事件
                if participant_id.starts_with("user-") {
                    self.event_bus.publish_to_session(session_id, SessionEvent::ClientJoined {
                        session_id: session_id.to_string(),
                        user_identity: participant_id,
                    });
                } else {
                    self.event_bus.publish_to_session(session_id, SessionEvent::MicroserviceJoined {
                        session_id: session_id.to_string(),
                        service_id: participant_id,
                    });
                }

                // 检查是否所有参与者都已加入
                if joined.len() == expected.len() {
                    self.event_bus.publish_to_session(session_id, SessionEvent::SessionReady {
                        session_id: session_id.to_string(),
                        all_participants_joined: true,
                    });
                    tracing::info!("会话 {} 所有参与者已加入", session_id);
                    return true;
                }
            }
        }
        false
    }

    /// 检查会话是否准备就绪
    pub fn is_session_ready(&self, session_id: &str) -> bool {
        if let Some(entry) = self.sessions.get(session_id) {
            let (expected, joined) = entry.value();
            return joined.len() == expected.len();
        }
        false
    }

    /// 获取会话状态
    pub fn get_session_status(&self, session_id: &str) -> Option<(usize, usize)> {
        self.sessions.get(session_id).map(|entry| {
            let (expected, joined) = entry.value();
            (expected.len(), joined.len())
        })
    }

    /// 清理会话跟踪
    pub fn cleanup_session(&self, session_id: &str) {
        self.sessions.remove(session_id);
        self.event_bus.cleanup_session(session_id);
    }
}