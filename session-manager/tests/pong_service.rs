//! PongService - A test microservice that responds to "ping" with "pong"
//!
//! This service is used for integration testing to verify:
//! 1. Microservice registration with session manager
//! 2. LiveKit room joining functionality
//! 3. Data channel communication between participants

use async_trait::async_trait;
use livekit::prelude::*;
use microservice_sdk::{
    JoinRoomRequest, MicroserviceConfig, MicroserviceHandler, MicroserviceRunner,
    Result as SdkResult,
};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, error, info, warn};

/// PongService that connects to LiveKit and responds to ping messages
pub struct PongService {
    service_name: String,
    received_messages: Arc<tokio::sync::Mutex<Vec<String>>>,
}

impl PongService {
    pub fn new(service_name: String) -> Self {
        Self {
            service_name,
            received_messages: Arc::new(tokio::sync::Mutex::new(Vec::new())),
        }
    }

    pub async fn get_received_messages(&self) -> Vec<String> {
        self.received_messages.lock().await.clone()
    }
}

#[async_trait]
impl MicroserviceHandler for PongService {
    async fn handle_join_room(&self, request: JoinRoomRequest) -> SdkResult<()> {
        info!(
            "PongService {} joining room {} for session {}",
            self.service_name, request.room_name, request.session_id
        );

        // Use the LiveKit URL provided in the request
        let livekit_url = &request.livekit_url;

        // Connect to LiveKit room
        let room_options = RoomOptions::default();

        match Room::connect(livekit_url, &request.access_token, room_options).await {
            Ok((room, mut event_rx)) => {
                info!(
                    "PongService {} successfully connected to room {}",
                    self.service_name, request.room_name
                );

                // Spawn a task to handle room events
                let room_name = request.room_name.clone();
                let service_name = self.service_name.clone();
                let received_messages = self.received_messages.clone();

                tokio::spawn(async move {
                    info!(
                        "PongService {} starting event loop for room {}",
                        service_name, room_name
                    );

                    while let Some(event) = event_rx.recv().await {
                        debug!("PongService {} received event: {:?}", service_name, event);

                        match event {
                            RoomEvent::DataReceived {
                                payload,
                                participant,
                                topic,
                                ..
                            } => {
                                let message = String::from_utf8_lossy(&payload);
                                let participant_identity = participant
                                    .as_ref()
                                    .map(|p| p.identity().to_string())
                                    .unwrap_or_else(|| "unknown".to_string());

                                info!("PongService {} received message from {}: topic={:?}, message='{}'",
                                    service_name, participant_identity, topic, message);

                                // Store the received message for testing verification
                                received_messages.lock().await.push(message.to_string());

                                // Check if message contains "ping"
                                if message.to_lowercase().contains("ping") {
                                    info!(
                                        "PongService {} detected ping, sending pong!",
                                        service_name
                                    );

                                    // Send pong response
                                    let pong_data = DataPacket {
                                        payload: "pong".to_string().into_bytes(),
                                        topic: Some("test-response".to_string()),
                                        reliable: true,
                                        destination_identities: vec![], // Send to all participants
                                    };

                                    if let Err(e) =
                                        room.local_participant().publish_data(pong_data).await
                                    {
                                        error!(
                                            "PongService {} failed to send pong: {}",
                                            service_name, e
                                        );
                                    } else {
                                        info!("PongService {} sent pong response!", service_name);
                                    }
                                }
                            }
                            RoomEvent::ParticipantConnected(participant) => {
                                info!(
                                    "PongService {} - participant joined: {}",
                                    service_name,
                                    participant.identity()
                                );
                            }
                            RoomEvent::ParticipantDisconnected(participant) => {
                                info!(
                                    "PongService {} - participant left: {}",
                                    service_name,
                                    participant.identity()
                                );
                            }
                            RoomEvent::Connected {
                                participants_with_tracks,
                            } => {
                                info!("PongService {} connected to room with {} existing participants",
                                    service_name, participants_with_tracks.len());
                            }
                            RoomEvent::ConnectionStateChanged(state) => {
                                info!(
                                    "PongService {} connection state changed: {:?}",
                                    service_name, state
                                );
                            }
                            RoomEvent::Disconnected { reason } => {
                                warn!(
                                    "PongService {} disconnected from room {}: {:?}",
                                    service_name, room_name, reason
                                );
                                break;
                            }
                            _ => {
                                // Log other events for debugging
                                debug!(
                                    "PongService {} received other event: {:?}",
                                    service_name, event
                                );
                            }
                        }
                    }

                    info!(
                        "PongService {} event loop ended for room {}",
                        service_name, room_name
                    );
                });

                Ok(())
            }
            Err(e) => {
                error!(
                    "PongService {} failed to connect to room {}: {}",
                    self.service_name, request.room_name, e
                );
                Err(microservice_sdk::MicroserviceError::JoinRoomFailed(
                    format!("LiveKit connection failed: {}", e),
                ))
            }
        }
    }

    async fn health_check(&self) -> SdkResult<()> {
        info!("PongService {} health check - OK", self.service_name);
        Ok(())
    }
}

/// Helper function to create and start a pong service for testing
pub async fn start_pong_service(
    service_id: String,
    service_port: u16,
    session_manager_url: String,
) -> SdkResult<Arc<PongService>> {
    let service_endpoint = format!("http://localhost:{}", service_port);

    let mut metadata = HashMap::new();
    metadata.insert("type".to_string(), "pong-service".to_string());
    metadata.insert(
        "capabilities".to_string(),
        "ping-pong-responder".to_string(),
    );
    metadata.insert("test".to_string(), "integration".to_string());

    let config = MicroserviceConfig::new(session_manager_url, service_id.clone(), service_endpoint)
        .with_metadata(metadata)
        .with_timeout(30);

    // Create the microservice handler
    let handler = Arc::new(PongService::new(service_id));
    let handler_clone = handler.clone();

    // Create and start the microservice runner
    let runner = MicroserviceRunner::new(config, handler_clone)?;

    info!(
        "Starting PongService microservice on port {}...",
        service_port
    );

    // Start the runner in a background task
    tokio::spawn(async move {
        if let Err(e) = runner.start().await {
            error!("PongService runner failed: {}", e);
        }
    });

    // Give the service a moment to start
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    Ok(handler)
}
