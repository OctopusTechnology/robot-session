use crate::{
    config::LiveKitConfig,
    domain::{Session, SessionStatus},
    services::MicroserviceRegistry,
    storage::SessionStorage,
    utils::errors::Result,
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::*;
use uuid::Uuid;

#[async_trait]
pub trait SessionService: Send + Sync {
    async fn create_session(&self, request: CreateSessionRequest) -> Result<(Session, String)>;
    async fn get_session(&self, session_id: &str) -> Result<Option<Session>>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSessionRequest {
    pub user_identity: String,
    pub user_name: Option<String>,
    pub room_name: Option<String>,
    pub metadata: Option<std::collections::HashMap<String, String>>,
    pub required_services: Option<Vec<String>>,
}

pub struct SessionServiceImpl {
    storage: Arc<dyn SessionStorage>,
    microservice_registry: Arc<MicroserviceRegistry>,
    livekit_config: LiveKitConfig,
    livekit_url: String,
    event_bus: crate::events::EventBus,
}

impl SessionServiceImpl {
    pub fn new(
        storage: Arc<dyn SessionStorage>,
        microservice_registry: Arc<MicroserviceRegistry>,
        livekit_config: LiveKitConfig,
        livekit_url: String,
        event_bus: crate::events::EventBus,
    ) -> Self {
        Self {
            storage,
            microservice_registry,
            livekit_config,
            livekit_url,
            event_bus,
        }
    }
}

#[async_trait]
impl SessionService for SessionServiceImpl {
    #[instrument(
        name = "create_session",
        skip(self),
        fields(
            user_identity = %request.user_identity,
            user_name = ?request.user_name,
            room_name = ?request.room_name,
            required_services = ?request.required_services,
            session_id,
            microservices_count,
            session_status
        )
    )]
    async fn create_session(&self, request: CreateSessionRequest) -> Result<(Session, String)> {
        // 1. Generate session ID and room name
        let session_id = Uuid::new_v4().to_string();
        let room_name = request
            .room_name
            .unwrap_or_else(|| format!("room-{}", session_id));

        // Record session_id in the span
        tracing::Span::current().record("session_id", &session_id);

        tracing::info!("Creating session for room {}", room_name);

        // 2. Get registered microservices (optional)
        let registered_services = if let Some(required_service_ids) = request.required_services {
            // If specific microservice IDs are specified, get those services
            match self
                .microservice_registry
                .get_services_by_ids(&required_service_ids)
                .await
            {
                Ok(services) => {
                    tracing::debug!(
                        "Found {} of {} required microservices",
                        services.len(),
                        required_service_ids.len()
                    );
                    services
                }
                Err(e) => {
                    tracing::warn!("Failed to get some required microservices: {}", e);
                    Vec::new() // Continue creating session, but without microservices
                }
            }
        } else {
            // If none specified, get all available microservices
            match self
                .microservice_registry
                .get_all_available_services()
                .await
            {
                Ok(services) => {
                    tracing::debug!("Found {} available microservices", services.len());
                    services
                }
                Err(e) => {
                    tracing::warn!("Failed to get available microservices: {}", e);
                    Vec::new() // Continue creating session, but without microservices
                }
            }
        };

        // 3. Create session object
        let mut session = Session::new(
            session_id.clone(),
            room_name.clone(),
            request.metadata.unwrap_or_default(),
        );

        // Add microservices to session (if any)
        for service in registered_services {
            session.add_microservice(service);
        }

        // Record microservices count in the span
        tracing::Span::current().record(
            "microservices_count",
            session.registered_microservices.len(),
        );

        // 4. Session creates its own LiveKit room
        session.create_livekit_room(&self.livekit_config).await?;

        // 5. Set session status and connect to LiveKit
        if session.registered_microservices.is_empty() {
            // No microservices, session is immediately ready
            session.update_status(SessionStatus::Ready);
            tracing::info!("Session created without microservices - immediately ready");
        } else {
            // Has microservices, let Session connect to LiveKit and monitor participants
            let livekit_config = self.livekit_config.clone();
            let event_bus = Arc::new(self.event_bus.clone());

            session
                .connect_to_livekit(livekit_config.clone(), event_bus)
                .await?;
            tracing::info!(
                "Session connected to LiveKit and monitoring for {} microservices",
                session.registered_microservices.len()
            );
        }

        // Record final session status in the span
        tracing::Span::current().record("session_status", format!("{:?}", session.status).as_str());

        // 6. Save session
        self.storage.save_session(&session).await?;

        // 7. Generate user access token
        let access_token = session.generate_client_token(&self.livekit_config)?;

        // 8. Notify microservices to join room (don't wait)
        if !session.registered_microservices.is_empty() {
            // Session notifies microservices to join - monitors their joining via events
            session
                .notify_microservices_to_join(&self.livekit_config, &self.livekit_url)
                .await?;
        }

        // 9. Publish session creation event
        self.event_bus.publish_to_session(
            &session.id,
            crate::events::SessionEvent::SessionCreated {
                session_id: session.id.clone(),
                room_name: session.room_name.clone(),
                access_token: access_token.clone(),
                livekit_url: self.livekit_url.clone(),
            },
        );

        tracing::info!("Session created successfully");
        Ok((session, access_token))
    }

    #[instrument(
        name = "get_session",
        skip(self),
        fields(session_id = %session_id, found, status)
    )]
    async fn get_session(&self, session_id: &str) -> Result<Option<Session>> {
        match self.storage.get_session(session_id).await {
            Ok(Some(session)) => {
                tracing::Span::current().record("found", true);
                tracing::Span::current().record("status", format!("{:?}", session.status).as_str());
                tracing::debug!(
                    "Session found with {} microservices",
                    session.registered_microservices.len()
                );
                Ok(Some(session))
            }
            Ok(None) => {
                tracing::Span::current().record("found", false);
                tracing::debug!("Session not found");
                Ok(None)
            }
            Err(e) => {
                tracing::error!("Failed to retrieve session: {}", e);
                Err(e)
            }
        }
    }
}
