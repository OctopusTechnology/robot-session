use reqwest::Client;
use std::sync::Arc;
use std::time::Duration;
use tracing::{error, info};

use crate::{
    errors::{MicroserviceError, Result},
    models::*,
    traits::MicroserviceHandler,
};

/// Client for communicating with the Session Manager
#[derive(Debug, Clone)]
pub struct SessionManagerClient {
    config: MicroserviceConfig,
    http_client: Client,
}

impl SessionManagerClient {
    /// Create a new session manager client
    pub fn new(config: MicroserviceConfig) -> Result<Self> {
        let http_client = Client::builder()
            .timeout(Duration::from_secs(config.request_timeout_secs))
            .build()
            .map_err(MicroserviceError::HttpError)?;

        Ok(Self {
            config,
            http_client,
        })
    }

    /// Register this microservice with the session manager
    pub async fn register(&self) -> Result<RegisterMicroserviceResponse> {
        let url = format!(
            "{}/api/v1/microservices/register",
            self.config.session_manager_url
        );

        let request = RegisterMicroserviceRequest {
            service_id: self.config.service_id.clone(),
            endpoint: self.config.service_endpoint.clone(),
            metadata: if self.config.metadata.is_empty() {
                None
            } else {
                Some(self.config.metadata.clone())
            },
        };

        info!(
            "Registering microservice {} with session manager",
            self.config.service_id
        );

        let response = self.http_client.post(&url).json(&request).send().await?;

        if response.status().is_success() {
            let register_response: RegisterMicroserviceResponse = response.json().await?;
            info!(
                "Successfully registered microservice: {}",
                register_response.message
            );
            Ok(register_response)
        } else {
            let status = response.status().as_u16();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());

            // Try to parse as ErrorResponse
            if let Ok(error_response) = serde_json::from_str::<ErrorResponse>(&error_text) {
                Err(MicroserviceError::SessionManagerError {
                    status,
                    message: error_response.message,
                })
            } else {
                Err(MicroserviceError::SessionManagerError {
                    status,
                    message: error_text,
                })
            }
        }
    }

    /// Get the service configuration
    pub fn config(&self) -> &MicroserviceConfig {
        &self.config
    }
}

/// Microservice runner that handles HTTP server and session manager integration
pub struct MicroserviceRunner {
    client: SessionManagerClient,
    handler: Arc<dyn MicroserviceHandler>,
}

impl MicroserviceRunner {
    /// Create a new microservice runner
    pub fn new(config: MicroserviceConfig, handler: Arc<dyn MicroserviceHandler>) -> Result<Self> {
        let client = SessionManagerClient::new(config)?;

        Ok(Self { client, handler })
    }

    /// Start the microservice (register and start HTTP server)
    pub async fn start(&self) -> Result<()> {
        // Register with session manager
        match self.client.register().await {
            Ok(response) => {
                info!("Registration successful: {}", response.message);
            }
            Err(e) => {
                error!("Failed to register with session manager: {}", e);
                return Err(e);
            }
        }

        // Start HTTP server to handle join-room requests
        self.start_http_server().await
    }

    /// Start HTTP server to handle requests from session manager
    async fn start_http_server(&self) -> Result<()> {
        use axum::{extract::State, http::StatusCode, response::Json, routing::post, Router};

        #[derive(Clone)]
        struct AppState {
            handler: Arc<dyn MicroserviceHandler>,
        }

        let app_state = AppState {
            handler: self.handler.clone(),
        };

        // Handler for join-room requests
        async fn handle_join_room(
            State(state): State<AppState>,
            Json(request): Json<JoinRoomRequest>,
        ) -> std::result::Result<Json<JoinRoomResponse>, (StatusCode, String)> {
            info!(
                "Received join-room request for session {}",
                request.session_id
            );

            // Call the microservice handler
            match state.handler.handle_join_room(request.clone()).await {
                Ok(()) => {
                    info!(
                        "Successfully joined room for session {}",
                        request.session_id
                    );
                    let response = JoinRoomResponse {
                        success: true,
                        message: "Successfully joined room".to_string(),
                        session_id: request.session_id,
                        service_id: request.service_identity,
                    };
                    Ok(Json(response))
                }
                Err(e) => {
                    error!("Failed to join room: {}", e);
                    Err((
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("Failed to join room: {}", e),
                    ))
                }
            }
        }

        // Health check handler
        async fn handle_health_check(
            State(state): State<AppState>,
        ) -> std::result::Result<Json<serde_json::Value>, (StatusCode, String)> {
            match state.handler.health_check().await {
                Ok(()) => Ok(Json(serde_json::json!({"status": "healthy"}))),
                Err(e) => Err((
                    StatusCode::SERVICE_UNAVAILABLE,
                    format!("Health check failed: {}", e),
                )),
            }
        }

        let app = Router::new()
            .route("/join-room", post(handle_join_room))
            .route("/health", axum::routing::get(handle_health_check))
            .with_state(app_state);

        // Extract port from service endpoint
        let port = self.extract_port_from_endpoint()?;
        let addr = format!("0.0.0.0:{}", port);

        info!("Starting HTTP server on {}", addr);

        let listener = tokio::net::TcpListener::bind(&addr).await.map_err(|e| {
            MicroserviceError::ConfigurationError(format!("Failed to bind to {}: {}", addr, e))
        })?;

        axum::serve(listener, app)
            .await
            .map_err(|e| MicroserviceError::ConfigurationError(format!("Server error: {}", e)))?;

        Ok(())
    }

    /// Extract port number from service endpoint URL
    fn extract_port_from_endpoint(&self) -> Result<u16> {
        let endpoint = &self.client.config.service_endpoint;

        // Parse URL to extract port
        let url = url::Url::parse(endpoint).map_err(|e| {
            MicroserviceError::ConfigurationError(format!("Invalid endpoint URL: {}", e))
        })?;

        let port = url.port().unwrap_or(match url.scheme() {
            "http" => 80,
            "https" => 443,
            _ => {
                return Err(MicroserviceError::ConfigurationError(
                    "Unknown URL scheme".to_string(),
                ))
            }
        });

        Ok(port)
    }
}
