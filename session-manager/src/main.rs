use anyhow::Result;
use session_manager::{config::AppConfig, server::Server};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<()> {
    // 加载配置
    let config = AppConfig::load()?;

    // 初始化日志
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        "session_manager=trace,tower_http=debug,livekit=trace,livekit_api=trace".into()
    });

    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_target(true)
        .with_line_number(true);

    // 如果启用了 Vector 日志，添加 Vector 层
    if config.vector_log.enabled {
        // Extract host:port from endpoint URL
        let vector_addr = if config.vector_log.endpoint.starts_with("http://") {
            config.vector_log.endpoint.strip_prefix("http://").unwrap_or(&config.vector_log.endpoint)
        } else {
            &config.vector_log.endpoint
        };
        
        let vector_layer = tracing_vector::VectorLayer::new(
            &config.vector_log.source_name,
            vector_addr,
        );
        
        tracing_subscriber::registry()
            .with(env_filter)
            .with(fmt_layer)
            .with(vector_layer)
            .init();
            
        tracing::info!("Vector logging initialized successfully to: {}", vector_addr);
    } else {
        tracing_subscriber::registry()
            .with(env_filter)
            .with(fmt_layer)
            .init();
    }

    tracing::info!("Starting session manager with config: {:?}", config);

    // 启动服务器
    let server = Server::new(config).await?;
    server.run().await?;

    Ok(())
}
