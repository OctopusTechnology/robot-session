use anyhow::Result;
use session_manager::{config::AppConfig, server::Server};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "session_manager=trace,tower_http=debug,livekit=trace,livekit_api=trace".into()),
        )
        .with(tracing_subscriber::fmt::layer().with_target(true).with_line_number(true))
        .init();

    // 加载配置
    let config = AppConfig::load()?;
    tracing::info!("Starting session manager with config: {:?}", config);

    // 启动服务器
    let server = Server::new(config).await?;
    server.run().await?;

    Ok(())
}