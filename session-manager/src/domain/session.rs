use crate::config::LiveKitConfig;
use crate::domain::microservice::MicroserviceInfo;
use crate::events::{EventBus, SessionEvent};
use crate::utils::errors::{Result, SessionManagerError};
use chrono::{DateTime, Utc};
use livekit::prelude::*;
use livekit_api::access_token::{AccessToken, VideoGrants};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;

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

    // Non-serialized fields for runtime state
    #[serde(skip)]
    pub room_connection: Option<Arc<RwLock<SessionRoomConnection>>>,
}

/// Runtime connection state for a session's LiveKit room
#[derive(Debug)]
pub struct SessionRoomConnection {
    pub room: Room,
    pub event_handle: Option<tokio::task::JoinHandle<()>>,
    pub livekit_config: LiveKitConfig,
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
            room_connection: None,
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

    /// Create a LiveKit room for this session
    pub async fn create_livekit_room(&self, config: &LiveKitConfig) -> Result<()> {
        use livekit_api::services::room::{CreateRoomOptions, RoomClient};

        tracing::debug!("Creating LiveKit room for session {}", self.id);
        tracing::debug!("  Room name: {}", self.room_name);
        tracing::debug!("  LiveKit server URL: {}", config.server_url);

        // Convert WebSocket URL to HTTP for API calls
        let api_url = if config.server_url.starts_with("ws://") {
            config.server_url.replace("ws://", "http://")
        } else if config.server_url.starts_with("wss://") {
            config.server_url.replace("wss://", "https://")
        } else {
            config.server_url.clone()
        };

        tracing::debug!("  Converted API URL: {}", api_url);

        let room_client = RoomClient::with_api_key(&api_url, &config.api_key, &config.api_secret);

        let options = CreateRoomOptions {
            empty_timeout: 300, // 5 minutes
            max_participants: 50,
            ..Default::default()
        };

        tracing::debug!(
            "  Room options: empty_timeout={}s, max_participants={}",
            options.empty_timeout,
            options.max_participants
        );

        match room_client.create_room(&self.room_name, options).await {
            Ok(_) => {
                tracing::info!("✓ Successfully created LiveKit room: {}", self.room_name);
                Ok(())
            }
            Err(e) => {
                tracing::error!(
                    "✗ Failed to create LiveKit room '{}': {}",
                    self.room_name,
                    e
                );
                Err(SessionManagerError::LiveKit(e))
            }
        }
    }

    /// Delete the LiveKit room for this session
    pub async fn delete_livekit_room(&self, config: &LiveKitConfig) -> Result<()> {
        use livekit_api::services::room::RoomClient;

        tracing::debug!("Deleting LiveKit room for session {}", self.id);
        tracing::debug!("  Room name: {}", self.room_name);

        // Convert WebSocket URL to HTTP for API calls
        let api_url = if config.server_url.starts_with("ws://") {
            config.server_url.replace("ws://", "http://")
        } else if config.server_url.starts_with("wss://") {
            config.server_url.replace("wss://", "https://")
        } else {
            config.server_url.clone()
        };

        tracing::debug!("  Using API URL: {}", api_url);

        let room_client = RoomClient::with_api_key(&api_url, &config.api_key, &config.api_secret);

        match room_client.delete_room(&self.room_name).await {
            Ok(_) => {
                tracing::info!("✓ Successfully deleted LiveKit room: {}", self.room_name);
                Ok(())
            }
            Err(e) => {
                tracing::error!(
                    "✗ Failed to delete LiveKit room '{}': {}",
                    self.room_name,
                    e
                );
                Err(SessionManagerError::LiveKit(e))
            }
        }
    }

    /// Connect to LiveKit room and start monitoring for microservice joins
    pub async fn connect_to_livekit(
        &mut self,
        livekit_config: LiveKitConfig,
        event_bus: Arc<EventBus>,
    ) -> Result<()> {
        tracing::debug!("Connecting session {} to LiveKit room", self.id);
        tracing::debug!("  Room name: {}", self.room_name);
        tracing::debug!(
            "  Expected microservices: {}",
            self.registered_microservices.len()
        );

        for service in &self.registered_microservices {
            tracing::debug!("    - {} ({})", service.service_id, service.endpoint);
        }

        // Generate room token for session manager
        tracing::debug!("Generating room token for session manager");
        let room_token = self.generate_room_token(&livekit_config)?;
        tracing::debug!("✓ Room token generated successfully");

        // Convert server URL to WebSocket format for Room::connect
        let ws_url = if livekit_config.server_url.starts_with("http://") {
            livekit_config.server_url.replace("http://", "ws://")
        } else if livekit_config.server_url.starts_with("https://") {
            livekit_config.server_url.replace("https://", "wss://")
        } else {
            livekit_config.server_url.clone()
        };

        tracing::debug!("  WebSocket URL: {}", ws_url);

        // Create room connection - Room::connect returns (Room, UnboundedReceiver<RoomEvent>)
        tracing::debug!("Attempting to connect to LiveKit room...");
        let (room, event_rx) = Room::connect(&ws_url, &room_token, RoomOptions::default())
            .await
            .map_err(|e| {
                tracing::error!("✗ Failed to connect to LiveKit room: {}", e);
                SessionManagerError::Internal(anyhow::anyhow!(
                    "Failed to connect to LiveKit room: {}",
                    e
                ))
            })?;

        tracing::info!(
            "✓ Successfully connected to LiveKit room: {}",
            self.room_name
        );

        // Start monitoring participants
        let session_id = self.id.clone();
        let expected_services: HashSet<String> = self
            .registered_microservices
            .iter()
            .map(|s| s.service_id.clone())
            .collect();

        tracing::debug!(
            "Starting lifecycle monitoring for {} expected services",
            expected_services.len()
        );

        let event_bus_clone = event_bus.clone();
        let event_handle = tokio::spawn(async move {
            Self::monitor_session_lifecycle(
                session_id,
                event_rx,
                expected_services,
                event_bus_clone,
            )
            .await;
        });

        // Store connection
        let connection = SessionRoomConnection {
            room,
            event_handle: Some(event_handle),
            livekit_config,
        };

        self.room_connection = Some(Arc::new(RwLock::new(connection)));
        self.update_status(SessionStatus::WaitingForServices);

        tracing::info!(
            "✓ Session {} connected to LiveKit and monitoring started",
            self.id
        );
        Ok(())
    }

    /// Monitor session lifecycle - handles microservices and client connections throughout session lifetime
    async fn monitor_session_lifecycle(
        session_id: String,
        mut event_rx: tokio::sync::mpsc::UnboundedReceiver<RoomEvent>,
        expected_services: HashSet<String>,
        event_bus: Arc<EventBus>,
    ) {
        let mut joined_services = HashSet::new();
        let mut client_connected = false;
        let mut client_last_seen = std::time::Instant::now();
        let mut service_last_seen: std::collections::HashMap<String, std::time::Instant> =
            std::collections::HashMap::new();

        const CLIENT_TIMEOUT_SECS: u64 = 300; // 5 minutes
        const SERVICE_TIMEOUT_SECS: u64 = 60; // 1 minute
        const SERVICE_RETRY_INTERVAL_SECS: u64 = 30; // 30 seconds

        let mut retry_timer =
            tokio::time::interval(std::time::Duration::from_secs(SERVICE_RETRY_INTERVAL_SECS));

        tracing::info!(
            "Starting session lifecycle monitoring for session {}",
            session_id
        );

        loop {
            tokio::select! {
                // Handle room events
                event = event_rx.recv() => {
                    match event {
                        Some(RoomEvent::ParticipantConnected(participant)) => {
                            let identity = participant.identity().to_string();

                            if expected_services.contains(&identity) {
                                // Microservice joined
                                if !joined_services.contains(&identity) {
                                    joined_services.insert(identity.clone());
                                    service_last_seen.insert(identity.clone(), std::time::Instant::now());

                                    tracing::info!("Microservice {} joined session {}", identity, session_id);

                                    // Publish microservice joined event
                                    event_bus.publish_to_session(&session_id, SessionEvent::MicroserviceJoined {
                                        session_id: session_id.clone(),
                                        service_id: identity,
                                    });

                                    // Check if all services have joined
                                    if joined_services.len() == expected_services.len() {
                                        event_bus.publish_to_session(&session_id, SessionEvent::SessionReady {
                                            session_id: session_id.clone(),
                                            all_participants_joined: true,
                                        });
                                    }
                                } else {
                                    // Service reconnected
                                    service_last_seen.insert(identity.clone(), std::time::Instant::now());
                                    tracing::info!("Microservice {} reconnected to session {}", identity, session_id);
                                }
                            } else if identity.starts_with("client-") || (!identity.starts_with("session-manager-") && !expected_services.contains(&identity)) {
                                // Client joined
                                client_connected = true;
                                client_last_seen = std::time::Instant::now();
                                tracing::info!("Client {} joined session {}", identity, session_id);

                                event_bus.publish_to_session(&session_id, SessionEvent::ClientJoined {
                                    session_id: session_id.clone(),
                                    user_identity: identity,
                                });
                            }
                        }

                        Some(RoomEvent::ParticipantDisconnected(participant)) => {
                            let identity = participant.identity().to_string();

                            if joined_services.contains(&identity) {
                                // Microservice disconnected
                                tracing::warn!("Microservice {} disconnected from session {}", identity, session_id);
                                // Don't remove from joined_services immediately - wait for timeout
                            } else if identity.starts_with("client-") || (!identity.starts_with("session-manager-") && !expected_services.contains(&identity)) {
                                // Client disconnected
                                client_connected = false;
                                tracing::info!("Client {} disconnected from session {}", identity, session_id);
                            }
                        }

                        Some(_) => {
                            // Other room events - update last seen times for all participants
                            client_last_seen = std::time::Instant::now();
                            let now = std::time::Instant::now();
                            for service in &joined_services {
                                service_last_seen.insert(service.clone(), now);
                            }
                        }

                        None => {
                            // Event stream closed - session should terminate
                            tracing::warn!("Room event stream closed for session {}", session_id);
                            break;
                        }
                    }
                }

                // Periodic checks for timeouts and retries
                _ = retry_timer.tick() => {
                    let now = std::time::Instant::now();

                    // Check client timeout
                    if client_connected && now.duration_since(client_last_seen).as_secs() > CLIENT_TIMEOUT_SECS {
                        tracing::warn!("Client timeout for session {} - terminating session", session_id);
                        event_bus.publish_to_session(&session_id, SessionEvent::SessionStatusChanged {
                            session_id: session_id.clone(),
                            status: SessionStatus::Terminating,
                        });
                        break;
                    }

                    // Check service timeouts and retry disconnected services
                    let mut services_to_retry = Vec::new();
                    for service in &expected_services {
                        if let Some(last_seen) = service_last_seen.get(service) {
                            if now.duration_since(*last_seen).as_secs() > SERVICE_TIMEOUT_SECS {
                                tracing::warn!("Service {} timeout in session {} - will retry", service, session_id);
                                joined_services.remove(service);
                                services_to_retry.push(service.clone());
                            }
                        } else if !joined_services.contains(service) {
                            // Service never joined - retry
                            services_to_retry.push(service.clone());
                        }
                    }

                    // Retry failed services
                    if !services_to_retry.is_empty() {
                        tracing::info!("Retrying {} services for session {}", services_to_retry.len(), session_id);
                        // TODO: Implement service retry logic here
                        // This would involve re-sending join notifications to failed services
                    }
                }
            }
        }

        tracing::info!(
            "Session lifecycle monitoring ended for session {}",
            session_id
        );
    }

    /// Generate a room token for connecting to LiveKit
    fn generate_room_token(&self, config: &LiveKitConfig) -> Result<String> {
        tracing::debug!("Generating room token for session manager");
        tracing::debug!("  Session ID: {}", self.id);
        tracing::debug!("  Room name: {}", self.room_name);
        tracing::debug!("  Identity: session-manager-{}", self.id);

        let grants = VideoGrants {
            room_join: true,
            room: self.room_name.clone(),
            can_publish: false,
            can_subscribe: true,
            ..Default::default()
        };

        tracing::debug!("  Grants: room_join=true, can_publish=false, can_subscribe=true");

        let token = AccessToken::with_api_key(&config.api_key, &config.api_secret)
            .with_identity(&format!("session-manager-{}", self.id))
            .with_grants(grants)
            .to_jwt()
            .map_err(|e| {
                tracing::error!("✗ Failed to generate room token: {}", e);
                SessionManagerError::Internal(anyhow::anyhow!(
                    "Failed to generate room token: {}",
                    e
                ))
            })?;

        tracing::debug!("✓ Room token generated successfully");
        Ok(token)
    }

    /// Generate a client token for the session
    pub fn generate_client_token(&self, config: &LiveKitConfig) -> Result<String> {
        tracing::debug!("Generating client token for session {}", self.id);
        tracing::debug!("  Room name: {}", self.room_name);
        tracing::debug!("  Client identity: client-{}", self.id);

        let grants = VideoGrants {
            room_join: true,
            room: self.room_name.clone(),
            can_publish: true,
            can_subscribe: true,
            ..Default::default()
        };

        tracing::debug!("  Grants: room_join=true, can_publish=true, can_subscribe=true");

        let token = AccessToken::with_api_key(&config.api_key, &config.api_secret)
            .with_identity(&format!("client-{}", self.id))
            .with_grants(grants)
            .to_jwt()
            .map_err(|e| {
                tracing::error!("✗ Failed to generate client token: {}", e);
                SessionManagerError::Internal(anyhow::anyhow!(
                    "Failed to generate client token: {}",
                    e
                ))
            })?;

        tracing::info!(
            "✓ Client token generated successfully for session {}",
            self.id
        );
        Ok(token)
    }

    /// Handle microservice joined event
    pub fn handle_microservice_joined(&mut self, service_id: &str) {
        if self.mark_service_ready(service_id) {
            tracing::info!("Microservice {} joined session {}", service_id, self.id);

            // Check if all services are ready
            if self.ready_microservices.len() == self.registered_microservices.len() {
                self.update_status(SessionStatus::Ready);
                tracing::info!("All microservices joined session {} - now ready", self.id);
            }
        }
    }

    /// Notify microservices to join this session's room
    /// This sends notifications but doesn't wait - actual join success is detected via RoomEvent
    pub async fn notify_microservices_to_join(
        &self,
        livekit_config: &LiveKitConfig,
        livekit_url: &str,
    ) -> Result<()> {
        if self.registered_microservices.is_empty() {
            tracing::debug!("No microservices to notify for session {}", self.id);
            return Ok(());
        }

        tracing::info!(
            "Notifying {} microservices to join session {}",
            self.registered_microservices.len(),
            self.id
        );
        tracing::debug!("  LiveKit URL: {}", livekit_url);

        // Send join requests to all microservices concurrently (fire and forget)
        for service in &self.registered_microservices {
            tracing::debug!("Preparing notification for service: {}", service.service_id);
            tracing::debug!("  Service endpoint: {}", service.endpoint);

            // Generate access token for each microservice
            let access_token =
                self.generate_microservice_token(&service.service_id, livekit_config)?;

            let join_request = crate::domain::JoinRoomRequest {
                room_name: self.room_name.clone(),
                session_id: self.id.clone(),
                service_identity: service.service_id.clone(),
                access_token,
                livekit_url: livekit_url.to_string(),
            };

            tracing::debug!("  Join request prepared for {}", service.service_id);

            let service_endpoint = service.endpoint.clone();
            let service_id = service.service_id.clone();

            // Fire and forget - actual join success will be detected via RoomEvent
            tokio::spawn(async move {
                tracing::debug!(
                    "Sending join notification to service {} at {}",
                    service_id,
                    service_endpoint
                );
                match Self::notify_service_join(service_endpoint, join_request).await {
                    Ok(()) => {
                        tracing::info!(
                            "✓ Successfully sent join notification to service {}",
                            service_id
                        );
                    }
                    Err(e) => {
                        tracing::error!("✗ Failed to notify service {} to join: {}", service_id, e);
                    }
                }
            });
        }

        tracing::info!(
            "✓ Sent join notifications to all {} microservices for session {}",
            self.registered_microservices.len(),
            self.id
        );

        Ok(())
    }

    /// Generate access token for a microservice
    fn generate_microservice_token(
        &self,
        service_id: &str,
        config: &LiveKitConfig,
    ) -> Result<String> {
        use std::time::Duration;

        tracing::debug!("Generating microservice token for service: {}", service_id);
        tracing::debug!("  Room name: {}", self.room_name);
        tracing::debug!("  TTL: 6 hours");

        let grants = VideoGrants {
            room_join: true,
            room: self.room_name.clone(),
            can_publish: true,
            can_subscribe: true,
            can_publish_data: true,
            ..Default::default()
        };

        tracing::debug!(
            "  Grants: room_join=true, can_publish=true, can_subscribe=true, can_publish_data=true"
        );

        let token = AccessToken::with_api_key(&config.api_key, &config.api_secret)
            .with_identity(service_id)
            .with_grants(grants)
            .with_ttl(Duration::from_secs(3600 * 6)) // 6 hours
            .to_jwt()
            .map_err(|e| {
                tracing::error!(
                    "✗ Failed to generate microservice token for {}: {}",
                    service_id,
                    e
                );
                SessionManagerError::Internal(anyhow::anyhow!(
                    "Failed to generate microservice token: {}",
                    e
                ))
            })?;

        tracing::debug!(
            "✓ Microservice token generated successfully for {}",
            service_id
        );
        Ok(token)
    }

    /// Notify a single service to join the room
    async fn notify_service_join(
        endpoint: String,
        request: crate::domain::JoinRoomRequest,
    ) -> Result<()> {
        let client = reqwest::Client::new();
        let url = format!("{}/join-room", endpoint);

        tracing::debug!("Sending join notification to service");
        tracing::debug!("  URL: {}", url);
        tracing::debug!("  Room: {}", request.room_name);
        tracing::debug!("  Session ID: {}", request.session_id);
        tracing::debug!("  Service Identity: {}", request.service_identity);
        tracing::debug!("  LiveKit URL: {}", request.livekit_url);

        let response = client
            .post(&url)
            .json(&request)
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await
            .map_err(|e| {
                tracing::error!("✗ HTTP request failed to {}: {}", endpoint, e);
                SessionManagerError::MicroserviceCommunication(e)
            })?;

        let status = response.status();
        tracing::debug!("  Response status: {}", status);

        if status.is_success() {
            tracing::info!(
                "✓ Successfully notified service at {} to join room",
                endpoint
            );
            Ok(())
        } else {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            tracing::error!(
                "✗ Service at {} returned error {}: {}",
                endpoint,
                status,
                error_text
            );
            Err(SessionManagerError::Internal(anyhow::anyhow!(
                "Service returned error {}: {}",
                status,
                error_text
            )))
        }
    }

    /// Disconnect from LiveKit room
    pub async fn disconnect_from_livekit(&mut self) -> Result<()> {
        tracing::debug!("Disconnecting session {} from LiveKit", self.id);

        if let Some(connection_arc) = self.room_connection.take() {
            tracing::debug!("  Found active room connection, closing...");
            let mut connection = connection_arc.write().await;

            // Cancel the monitoring task
            if let Some(handle) = connection.event_handle.take() {
                tracing::debug!("  Aborting monitoring task");
                handle.abort();
            }

            // Disconnect from room
            tracing::debug!("  Closing room connection");
            match connection.room.close().await {
                Ok(_) => tracing::debug!("  ✓ Room connection closed successfully"),
                Err(e) => tracing::warn!("  ⚠ Error closing room connection: {}", e),
            }
        } else {
            tracing::debug!("  No active room connection found");
        }

        self.update_status(SessionStatus::Terminated);
        tracing::info!(
            "✓ Session {} disconnected from LiveKit and terminated",
            self.id
        );
        Ok(())
    }
}
