use crate::utils::errors::{Result, SessionManagerError};
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub livekit: LiveKitConfig,
    pub microservices: MicroserviceConfig,
    pub logging: LoggingConfig,
    pub vector_log: VectorLogConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub workers: Option<usize>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct LiveKitConfig {
    pub server_url: String,
    pub api_key: String,
    pub api_secret: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct MicroserviceConfig {
    pub registration_timeout: u64,
    pub join_timeout: u64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct LoggingConfig {
    pub level: String,
    pub format: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct VectorLogConfig {
    pub enabled: bool,
    pub endpoint: String,
    pub source_name: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            server: ServerConfig {
                host: "0.0.0.0".to_string(),
                port: 8080,
                workers: Some(4),
            },
            livekit: LiveKitConfig {
                server_url: "ws://localhost:7880".to_string(),
                api_key: std::env::var("LIVEKIT_API_KEY").unwrap_or_else(|_| "devkey".to_string()),
                api_secret: std::env::var("LIVEKIT_API_SECRET")
                    .unwrap_or_else(|_| "secret".to_string()),
            },
            microservices: MicroserviceConfig {
                registration_timeout: 30,
                join_timeout: 60,
            },
            logging: LoggingConfig {
                level: "debug".to_string(),
                format: "json".to_string(),
            },
            vector_log: VectorLogConfig {
                enabled: std::env::var("VECTOR_LOG_ENABLED").unwrap_or_else(|_| "true".to_string())
                    == "true",
                endpoint: std::env::var("VECTOR_LOG_ENDPOINT")
                    .unwrap_or_else(|_| "localhost:9000".to_string()),
                source_name: "session-manager".to_string(),
            },
        }
    }
}

impl AppConfig {
    pub fn load() -> Result<Self> {
        // 首先尝试从 TOML 配置文件加载
        let config_path = std::env::var("CONFIG_PATH").unwrap_or_else(|_| "/etc/session-manager/config.toml".to_string());
        
        let mut config = if std::path::Path::new(&config_path).exists() {
            // 从 TOML 文件加载配置
            let config_str = std::fs::read_to_string(&config_path)
                .map_err(|e| SessionManagerError::Configuration(format!("Failed to read config file {}: {}", config_path, e)))?;
            
            toml::from_str::<AppConfig>(&config_str)
                .map_err(|e| SessionManagerError::Configuration(format!("Failed to parse config file: {}", e)))?
        } else {
            // 如果配置文件不存在，使用默认配置
            AppConfig::default()
        };

        // 环境变量覆盖配置文件设置
        if let Ok(url) = std::env::var("LIVEKIT_SERVER_URL") {
            config.livekit.server_url = url;
        }
        if let Ok(key) = std::env::var("LIVEKIT_API_KEY") {
            config.livekit.api_key = key;
        }
        if let Ok(secret) = std::env::var("LIVEKIT_API_SECRET") {
            config.livekit.api_secret = secret;
        }

        // 覆盖服务器配置
        if let Ok(host) = std::env::var("SERVER_HOST") {
            config.server.host = host;
        }
        if let Ok(port) = std::env::var("SERVER_PORT") {
            config.server.port = port
                .parse()
                .map_err(|e| SessionManagerError::Configuration(format!("Invalid port: {}", e)))?;
        }

        // 覆盖 Vector 日志配置
        if let Ok(enabled) = std::env::var("VECTOR_LOG_ENABLED") {
            config.vector_log.enabled = enabled == "true";
        }
        if let Ok(endpoint) = std::env::var("VECTOR_LOG_ENDPOINT") {
            config.vector_log.endpoint = endpoint;
        }

        Ok(config)
    }
}
