use std::sync::Arc;
use axum::{
    routing::{get, post},
    Router,
};
use tower::ServiceBuilder;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tokio::net::TcpListener;

use crate::{
    api::handlers,
    config::AppConfig,
    services::{
        session_service::SessionServiceImpl,
        livekit_service::LiveKitService,
        microservice_registry::MicroserviceRegistry,
    },
    storage::memory::MemoryStorage,
    utils::errors::Result,
};

pub struct Server {
    config: AppConfig,
    app: Router,
}

impl Server {
    pub async fn new(config: AppConfig) -> Result<Self> {
        // 创建存储
        let storage = Arc::new(MemoryStorage::new());
        
        // 创建事件总线
        let event_bus = crate::events::EventBus::new();
        let event_bus_arc = Arc::new(event_bus.clone());
        
        // 创建 LiveKit 服务
        let livekit_service = Arc::new(LiveKitService::new(config.livekit.clone(), event_bus_arc));
        
        // 创建微服务注册表
        let microservice_registry = Arc::new(MicroserviceRegistry::new());
        
        // 创建会话服务
        let session_service = Arc::new(SessionServiceImpl::new(
            storage,
            livekit_service,
            microservice_registry.clone(),
            config.microservices.clone(),
            event_bus.clone(),
        ));
        
        // 创建应用状态
        let app_state = handlers::AppState {
            session_service,
            microservice_registry,
            config: config.clone(),
            event_bus,
        };

        // 构建路由
        let app = Router::new()
            .route("/health", get(handlers::health_check))
            .route("/api/v1/microservices/register", post(handlers::register_microservice))
            .route("/api/v1/sessions", post(handlers::create_session))
            .with_state(app_state)
            .layer(
                ServiceBuilder::new()
                    .layer(TraceLayer::new_for_http())
                    .layer(CorsLayer::permissive())
            );

        Ok(Self { config, app })
    }

    pub async fn run(self) -> Result<()> {
        let addr = format!("{}:{}", self.config.server.host, self.config.server.port);
        tracing::info!("Starting server on {}", addr);

        let listener = TcpListener::bind(&addr).await
            .map_err(|e| crate::utils::errors::SessionManagerError::Internal(e.into()))?;

        axum::serve(listener, self.app).await
            .map_err(|e| crate::utils::errors::SessionManagerError::Internal(e.into()))?;

        Ok(())
    }
}