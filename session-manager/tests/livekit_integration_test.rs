use livekit::prelude::*;
use reqwest::Client;
use serde_json::json;
use session_manager::{config::AppConfig, server::Server};
use std::{sync::Arc, time::Duration};
use tokio::time::sleep;

mod pong_service;
use pong_service::{start_pong_service, PongService};

// Test configuration for LiveKit
const LIVEKIT_URL: &str = "ws://localhost:7880";
const LIVEKIT_API_KEY: &str = "devkey";
const LIVEKIT_API_SECRET: &str = "secret";

#[tokio::test]
async fn test_session_creation_with_microservice_integration() {
    // Initialize detailed logging
    tracing_subscriber::fmt()
        .with_env_filter("session_manager=debug,microservice_sdk=debug,livekit=info,livekit_api=info,tower_http=info")
        .with_target(true)
        .with_line_number(true)
        .with_thread_ids(true)
        .with_level(true)
        .init();

    tracing::info!("🔍 Starting LiveKit integration test with microservice support");

    // Wait for LiveKit service to be ready
    wait_for_livekit().await;

    // Create test configuration
    let config = create_test_config();

    // Start session manager server
    let server = Server::new(config.clone())
        .await
        .expect("Failed to create server");

    // Run server in background
    let server_handle = tokio::spawn(async move {
        server.run().await.expect("Server failed to run");
    });

    // Wait for server to start
    sleep(Duration::from_millis(1000)).await;

    let client = Client::new();
    let base_url = "http://127.0.0.1:8080";

    // 1. Test health check
    tracing::info!("✓ Testing health check");
    let health_response = client
        .get(format!("{}/health", base_url))
        .send()
        .await
        .expect("Health check request failed");

    assert!(health_response.status().is_success());
    tracing::info!("✓ Health check passed");

    // 2. Start pong microservice
    tracing::info!("✓ Starting pong microservice");
    let pong_service =
        start_pong_service("pong-service-test".to_string(), 3001, base_url.to_string())
            .await
            .expect("Failed to start pong service");

    // Wait for microservice to register
    sleep(Duration::from_millis(2000)).await;

    // 3. Create session WITH microservices
    tracing::info!("✓ Creating session with microservices");
    let session_request = json!({
        "user_identity": "test-user-microservice-123",
        "user_name": "Microservice Test User",
        "room_name": "microservice-integration-test-room",
        "required_services": ["pong-service-test"],
        "metadata": {
            "test": "microservice_integration",
            "client_type": "integration_test"
        }
    });

    // Call session creation API
    let session_url = format!("{}/api/v1/sessions", base_url);
    tracing::info!("🔗 Sending session creation request to: {}", session_url);
    tracing::debug!(
        "📤 Session request: {}",
        serde_json::to_string_pretty(&session_request).unwrap()
    );

    let session_response = client
        .post(&session_url)
        .json(&session_request)
        .send()
        .await
        .expect("Session creation request failed");

    assert!(
        session_response.status().is_success(),
        "Session creation should succeed"
    );

    let session_data: serde_json::Value = session_response
        .json()
        .await
        .expect("Failed to parse session response");

    tracing::info!("✓ Session created successfully");
    tracing::debug!("Session response: {:#}", session_data);

    // Extract session information
    let session_id = session_data
        .get("session_id")
        .and_then(|v| v.as_str())
        .expect("Should have session_id");
    let access_token = session_data
        .get("access_token")
        .and_then(|v| v.as_str())
        .expect("Should have access_token");
    let room_name = session_data
        .get("room_name")
        .and_then(|v| v.as_str())
        .expect("Should have room_name");
    let livekit_url = session_data
        .get("livekit_url")
        .and_then(|v| v.as_str())
        .expect("Should have livekit_url");

    tracing::info!("Session information:");
    tracing::info!("  Session ID: {}", session_id);
    tracing::info!("  Room name: {}", room_name);
    tracing::info!("  LiveKit URL: {}", livekit_url);

    // 4. Test client joining LiveKit room and data channel communication
    tracing::info!("✓ Testing client LiveKit connection with microservice communication");
    match test_client_with_microservice_communication(livekit_url, access_token, &pong_service)
        .await
    {
        Ok(_) => {
            tracing::info!(
                "✓ Client successfully communicated with microservice via data channels"
            );
        }
        Err(e) => {
            tracing::error!("⚠ Client-microservice communication failed: {}", e);
            panic!("Client should be able to communicate with microservice");
        }
    }

    // Stop server
    server_handle.abort();
}

async fn test_client_with_microservice_communication(
    livekit_url: &str,
    access_token: &str,
    pong_service: &Arc<PongService>,
) -> Result<(), Box<dyn std::error::Error>> {
    tracing::info!("Attempting to connect to LiveKit: {}", livekit_url);
    tracing::trace!("Using access token: {}", access_token);

    // Connect to LiveKit room
    tracing::debug!("Calling Room::connect...");
    let (room, mut event_rx) =
        Room::connect(livekit_url, access_token, RoomOptions::default()).await?;

    tracing::info!(
        "Successfully connected to LiveKit room: {} ({})",
        room.name(),
        room.maybe_sid()
            .map(|s| s.to_string())
            .unwrap_or_else(|| "unknown".to_string())
    );

    // Shared state for event handling
    let connected = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let webrtc_connected = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let participants_seen = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let received_pong = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let received_messages = Arc::new(tokio::sync::Mutex::new(Vec::<String>::new()));

    // Clone for the event handler task
    let connected_clone = connected.clone();
    let webrtc_connected_clone = webrtc_connected.clone();
    let participants_seen_clone = participants_seen.clone();
    let received_pong_clone = received_pong.clone();
    let received_messages_clone = received_messages.clone();

    // Spawn event handler task
    let event_handler = tokio::spawn(async move {
        while let Some(event) = event_rx.recv().await {
            tracing::debug!("LiveKit event: {:?}", event);

            match event {
                RoomEvent::Connected {
                    participants_with_tracks,
                } => {
                    connected_clone.store(true, std::sync::atomic::Ordering::Relaxed);
                    participants_seen_clone.store(
                        participants_with_tracks.len(),
                        std::sync::atomic::Ordering::Relaxed,
                    );
                    tracing::info!(
                        "✓ Client connected to LiveKit room, found {} participants",
                        participants_with_tracks.len()
                    );
                }
                RoomEvent::ConnectionStateChanged(state) => {
                    tracing::info!("🔗 Connection state changed: {:?}", state);
                    if matches!(state, ConnectionState::Connected) {
                        webrtc_connected_clone.store(true, std::sync::atomic::Ordering::Relaxed);
                        tracing::info!("✓ WebRTC connection established");
                    }
                }
                RoomEvent::ParticipantConnected(participant) => {
                    tracing::info!("✓ Participant connected: {}", participant.identity());
                    participants_seen_clone.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                }
                RoomEvent::ParticipantDisconnected(participant) => {
                    tracing::info!("✓ Participant disconnected: {}", participant.identity());
                }
                RoomEvent::DataReceived {
                    payload,
                    participant,
                    topic,
                    ..
                } => {
                    let message = String::from_utf8(payload.to_vec())
                        .unwrap_or_else(|_| format!("Binary data ({} bytes)", payload.len()));

                    let participant_identity = participant
                        .as_ref()
                        .map(|p| p.identity().to_string())
                        .unwrap_or_else(|| "unknown".to_string());

                    tracing::info!(
                        "✓ Received data from {}: topic={:?}, message='{}'",
                        participant_identity,
                        topic,
                        message
                    );

                    // Store the received message
                    received_messages_clone.lock().await.push(message.clone());

                    // Check if it's a pong response
                    if message.to_lowercase().contains("pong") {
                        received_pong_clone.store(true, std::sync::atomic::Ordering::Relaxed);
                        tracing::info!("✓ Received pong response from microservice!");
                    }
                }
                _ => {}
            }
        }
    });

    // Phase 1: Wait for basic connection and microservice to join
    let connection_timeout = tokio::time::timeout(Duration::from_secs(15), async {
        loop {
            let is_connected = connected.load(std::sync::atomic::Ordering::Relaxed);
            let is_webrtc_connected = webrtc_connected.load(std::sync::atomic::Ordering::Relaxed);
            let participant_count = participants_seen.load(std::sync::atomic::Ordering::Relaxed);

            // We expect at least 1 participant (the microservice)
            if is_connected && is_webrtc_connected && participant_count >= 1 {
                break;
            }

            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    })
    .await;

    let final_connected = connected.load(std::sync::atomic::Ordering::Relaxed);
    let final_webrtc_connected = webrtc_connected.load(std::sync::atomic::Ordering::Relaxed);
    let final_participants = participants_seen.load(std::sync::atomic::Ordering::Relaxed);

    match connection_timeout {
        Ok(_) => {
            tracing::info!("✓ Successfully connected with microservice present");
            tracing::info!("  Basic connection status: {}", final_connected);
            tracing::info!("  WebRTC connection status: {}", final_webrtc_connected);
            tracing::info!("  Participants count: {}", final_participants);
        }
        Err(_) => {
            tracing::warn!("⚠ Timeout waiting for microservice to join room");
            return Err("Timeout waiting for microservice connection".into());
        }
    }

    // Phase 2: Test ping-pong communication
    if final_connected && final_webrtc_connected && final_participants >= 1 {
        tracing::info!("📡 Testing ping-pong communication with microservice...");

        // Send ping message
        let ping_data = DataPacket {
            payload: "ping".to_string().into_bytes(),
            topic: Some("test-ping".to_string()),
            reliable: true,
            destination_identities: vec![],
        };

        tracing::info!("Sending ping message to microservice...");
        room.local_participant().publish_data(ping_data).await?;
        tracing::info!("✓ Ping message sent");

        // Wait for pong response
        let pong_timeout = tokio::time::timeout(Duration::from_secs(10), async {
            loop {
                if received_pong.load(std::sync::atomic::Ordering::Relaxed) {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        })
        .await;

        match pong_timeout {
            Ok(_) => {
                tracing::info!("✓ Received pong response from microservice!");

                // Verify microservice received our ping
                let service_messages = pong_service.get_received_messages().await;
                tracing::info!(
                    "Microservice received {} messages: {:?}",
                    service_messages.len(),
                    service_messages
                );

                if service_messages.iter().any(|msg| msg.contains("ping")) {
                    tracing::info!("✓ Microservice confirmed receiving ping message");
                } else {
                    tracing::warn!("⚠ Microservice did not receive ping message");
                }
            }
            Err(_) => {
                tracing::error!("✗ Timeout waiting for pong response");
                return Err("No pong response received from microservice".into());
            }
        }
    }

    // Output final status
    tracing::info!("📊 Microservice communication test results:");
    tracing::info!("  Basic connection: {}", final_connected);
    tracing::info!("  WebRTC connection: {}", final_webrtc_connected);
    tracing::info!("  Microservice participants: {}", final_participants);
    tracing::info!(
        "  Ping-pong communication: {}",
        received_pong.load(std::sync::atomic::Ordering::Relaxed)
    );

    // Close connection and cleanup
    room.close().await?;
    event_handler.abort();
    tracing::info!("✓ Disconnected from LiveKit room");

    tracing::info!("✓ Microservice integration test completed successfully");
    tracing::info!("📋 Test Summary:");
    tracing::info!("  ✅ Health check API works");
    tracing::info!("  ✅ Session creation API works");
    tracing::info!("  ✅ Microservice registration works");
    tracing::info!("  ✅ LiveKit room creation works");
    tracing::info!("  ✅ Client can connect to LiveKit room");
    tracing::info!("  ✅ Microservice can join LiveKit room");
    tracing::info!("  ✅ WebRTC connection established");
    tracing::info!("  ✅ Data channel communication works");
    tracing::info!("  ✅ Ping-pong message exchange successful");

    Ok(())
}

async fn wait_for_livekit() {
    let client = Client::new();
    let mut attempts = 0;
    const MAX_ATTEMPTS: u32 = 30;

    tracing::info!("Waiting for LiveKit service to start...");

    while attempts < MAX_ATTEMPTS {
        match client.get("http://localhost:7880").send().await {
            Ok(response) if response.status().is_success() => {
                tracing::info!("✓ LiveKit service is ready");
                return;
            }
            _ => {
                attempts += 1;
                tracing::debug!(
                    "Waiting for LiveKit service... attempt {}/{}",
                    attempts,
                    MAX_ATTEMPTS
                );
                sleep(Duration::from_secs(2)).await;
            }
        }
    }

    panic!(
        "LiveKit service did not start within {} seconds",
        MAX_ATTEMPTS * 2
    );
}

fn create_test_config() -> AppConfig {
    AppConfig {
        server: session_manager::config::ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 8080,
            workers: Some(1),
        },
        livekit: session_manager::config::LiveKitConfig {
            server_url: LIVEKIT_URL.to_string(),
            api_key: LIVEKIT_API_KEY.to_string(),
            api_secret: LIVEKIT_API_SECRET.to_string(),
        },
        microservices: session_manager::config::MicroserviceConfig {
            registration_timeout: 30,
            join_timeout: 60,
        },
        logging: session_manager::config::LoggingConfig {
            level: "debug".to_string(),
            format: "json".to_string(),
        },
        vector_log: session_manager::config::VectorLogConfig {
            enabled: false,
            endpoint: "http://localhost:8686".to_string(),
            source_name: "session-manager-livekit-test".to_string(),
        },
    }
}
