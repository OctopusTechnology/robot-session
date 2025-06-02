use std::collections::{HashMap, HashSet};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use crate::domain::microservice::MicroserviceInfo;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub room_name: String,
    pub status: SessionStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub client_token: Option<String>,
    pub registered_microservices: Vec<MicroserviceInfo>,
    pub ready_microservices: HashSet<String>,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SessionStatus {
    Creating,           // 正在创建房间
    WaitingForServices, // 等待微服务加入
    Ready,              // 准备就绪，可以返回令牌
    Active,             // 客户端已连接
    Terminating,        // 正在终止
    Terminated,         // 已终止
}

impl Session {
    pub fn new(id: String, room_name: String, metadata: HashMap<String, String>) -> Self {
        let now = Utc::now();
        Self {
            id,
            room_name,
            status: SessionStatus::Creating,
            created_at: now,
            updated_at: now,
            client_token: None,
            registered_microservices: Vec::new(),
            ready_microservices: HashSet::new(),
            metadata,
        }
    }

    pub fn update_status(&mut self, status: SessionStatus) {
        self.status = status;
        self.updated_at = Utc::now();
    }

    pub fn add_microservice(&mut self, microservice: MicroserviceInfo) {
        self.registered_microservices.push(microservice);
        self.updated_at = Utc::now();
    }

    pub fn mark_service_ready(&mut self, service_id: &str) -> bool {
        let was_inserted = self.ready_microservices.insert(service_id.to_string());
        if was_inserted {
            self.updated_at = Utc::now();
            
            // 检查是否所有微服务都已就绪
            if self.ready_microservices.len() == self.registered_microservices.len() {
                self.status = SessionStatus::Ready;
            }
        }
        was_inserted
    }

    pub fn is_ready(&self) -> bool {
        self.status == SessionStatus::Ready
    }

    pub fn get_pending_services(&self) -> Vec<String> {
        self.registered_microservices
            .iter()
            .filter(|service| !self.ready_microservices.contains(&service.service_id))
            .map(|service| service.service_id.clone())
            .collect()
    }

    pub fn get_ready_services(&self) -> Vec<String> {
        self.ready_microservices.iter().cloned().collect()
    }
}