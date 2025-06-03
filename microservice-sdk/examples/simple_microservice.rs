//! PongService - A simple microservice that responds to "ping" with "pong"
//!
//! This example shows how to create a microservice that:
//! 1. Registers itself with the session manager
//! 2. Actually connects to LiveKit rooms when requested
//! 3. Listens for data messages containing "ping"
//! 4. Responds with "pong" messages

use std::sync::Arc;
use std::collections::HashMap;
use microservice_sdk::{
    MicroserviceConfig, MicroserviceRunner, MicroserviceHandler,
    JoinRoomRequest, Result as SdkResult,
};
use async_trait::async_trait;
use tracing::{info, error, warn};
use livekit::prelude::*;

/// PongService that connects to LiveKit and responds to ping messages
struct PongService {
    service_name: String,
}

impl PongService {
    fn new(service_name: String) -> Self {
        Self {
            service_name,
        }
    }
}

#[async_trait]
impl MicroserviceHandler for PongService {
    async fn handle_join_room(&self, request: JoinRoomRequest) -> SdkResult<()> {
        info!("PongService {} joining room {} for session {}",
            self.service_name, request.room_name, request.session_id);

        // Use the LiveKit URL provided in the request
        let livekit_url = &request.livekit_url;

        // Connect to LiveKit room
        let room_options = RoomOptions::default();
        
        match Room::connect(livekit_url, &request.access_token, room_options).await {
            Ok((room, mut event_rx)) => {
                info!("PongService {} successfully connected to room {}",
                    self.service_name, request.room_name);

                // Spawn a task to handle room events
                let session_id = request.session_id.clone();
                let room_name = request.room_name.clone();
                let service_name = self.service_name.clone();

                tokio::spawn(async move {
                    info!("PongService {} starting event loop for room {}", service_name, room_name);

                    while let Some(event) = event_rx.recv().await {
                        match event {
                            RoomEvent::DataReceived { payload, participant, .. } => {
                                let message = String::from_utf8_lossy(&payload);
                                info!("PongService {} received message from {}: {}",
                                    service_name, participant.map(|p| p.identity()).unwrap_or_default(), message);

                                // Check if message contains "ping"
                                if message.to_lowercase().contains("ping") {
                                    info!("PongService {} detected ping, sending pong!", service_name);
                                    
                                    // Send pong response
                                    let pong_data = DataPacket {
                                        payload: "pong".to_string().into_bytes(),
                                        reliable: true,
                                        ..Default::default()
                                    };

                                    if let Err(e) = room.local_participant().publish_data(pong_data).await {
                                        error!("PongService {} failed to send pong: {}", service_name, e);
                                    } else {
                                        info!("PongService {} sent pong response!", service_name);
                                    }
                                }
                            }
                            RoomEvent::ParticipantConnected(participant) => {
                                info!("PongService {} - participant joined: {}",
                                    service_name, participant.identity());
                            }
                            RoomEvent::ParticipantDisconnected(participant) => {
                                info!("PongService {} - participant left: {}",
                                    service_name, participant.identity());
                            }
                            RoomEvent::Disconnected { reason } => {
                                warn!("PongService {} disconnected from room {}: {:?}",
                                    service_name, room_name, reason);
                                break;
                            }
                            _ => {
                                // Log other events for debugging
                                info!("PongService {} received event: {:?}", service_name, event);
                            }
                        }
                    }

                    info!("PongService {} event loop ended for room {}", service_name, room_name);
                });

                Ok(())
            }
            Err(e) => {
                error!("PongService {} failed to connect to room {}: {}",
                    self.service_name, request.room_name, e);
                Err(microservice_sdk::MicroserviceError::JoinRoomFailed(
                    format!("LiveKit connection failed: {}", e)
                ))
            }
        }
    }

    async fn health_check(&self) -> SdkResult<()> {
        info!("PongService {} health check - OK", self.service_name);
        Ok(())
    }
}

#[tokio::main]
async fn main() -> SdkResult<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Configuration
    let service_id = "pong-service-1".to_string();
    let service_endpoint = "http://localhost:3001".to_string();
    let session_manager_url = "http://localhost:8080".to_string();

    let mut metadata = HashMap::new();
    metadata.insert("type".to_string(), "pong-service".to_string());
    metadata.insert("capabilities".to_string(), "ping-pong-responder".to_string());

    let config = MicroserviceConfig::new(
        session_manager_url,
        service_id.clone(),
        service_endpoint,
    )
    .with_metadata(metadata)
    .with_timeout(30);

    // Create the microservice handler
    let handler = Arc::new(PongService::new(service_id));

    // Create and start the microservice runner
    let runner = MicroserviceRunner::new(config, handler)?;
    
    info!("Starting PongService microservice...");
    runner.start().await?;

    Ok(())
}