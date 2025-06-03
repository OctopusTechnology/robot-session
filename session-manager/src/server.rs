use axum::{
    routing::{get, post},
    Router,
};
use std::sync::Arc;
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tower_http::{cors::CorsLayer, trace::TraceLayer};

use crate::{
    api::handlers,
    config::AppConfig,
    services::{microservice_registry::MicroserviceRegistry, session_service::SessionServiceImpl},
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

        // 创建微服务注册表
        let microservice_registry = Arc::new(MicroserviceRegistry::new());

        // 创建会话服务
        let session_service = Arc::new(SessionServiceImpl::new(
            storage,
            microservice_registry.clone(),
            config.livekit.clone(),
            config.livekit.server_url.clone(),
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
            .route(
                "/api/v1/microservices/register",
                post(handlers::register_microservice),
            )
            .route("/api/v1/create-session", post(handlers::create_session))
            .with_state(app_state)
            .layer(
                ServiceBuilder::new()
                    .layer(TraceLayer::new_for_http())
                    .layer(CorsLayer::permissive()),
            );

        Ok(Self { config, app })
    }

    pub async fn run(self) -> Result<()> {
        let addr = format!("{}:{}", self.config.server.host, self.config.server.port);
        tracing::info!("Starting server on {}", addr);

        let listener = TcpListener::bind(&addr)
            .await
            .map_err(|e| crate::utils::errors::SessionManagerError::Internal(e.into()))?;

        axum::serve(listener, self.app)
            .await
            .map_err(|e| crate::utils::errors::SessionManagerError::Internal(e.into()))?;

        Ok(())
    }
}
