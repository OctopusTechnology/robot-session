use std::time::Duration;
use tokio::time::sleep;
use reqwest::Client;
use serde_json::json;
use session_manager::{
    config::AppConfig,
    server::Server,
};
use futures_util::StreamExt;
use livekit::prelude::*;
use reqwest_eventsource::{Event, EventSource};

// Test configuration for LiveKit
const LIVEKIT_URL: &str = "ws://localhost:7880";
const LIVEKIT_API_KEY: &str = "devkey";
const LIVEKIT_API_SECRET: &str = "secret";

#[tokio::test]
async fn test_session_creation_with_livekit_client_join() {
    // 初始化详细日志
    tracing_subscriber::fmt()
        .with_env_filter("session_manager=trace,livekit=trace,livekit_api=trace,tower_http=debug,reqwest=debug")
        .with_target(true)
        .with_line_number(true)
        .with_thread_ids(true)
        .with_level(true)
        .init();
    
    println!("🔍 开始 LiveKit 集成测试，启用详细日志记录");
    
    // 等待 LiveKit 服务启动
    wait_for_livekit().await;
    
    // 创建测试配置
    let config = create_test_config();
    
    // 启动会话管理器服务器
    let server = Server::new(config.clone()).await.expect("Failed to create server");
    
    // 在后台运行服务器
    let server_handle = tokio::spawn(async move {
        server.run().await.expect("Server failed to run");
    });
    
    // 等待服务器启动
    sleep(Duration::from_millis(1000)).await;
    
    let client = Client::new();
    let base_url = "http://127.0.0.1:8080";
    
    // 1. 测试健康检查
    println!("✓ 测试健康检查");
    let health_response = client
        .get(&format!("{}/health", base_url))
        .send()
        .await
        .expect("健康检查请求失败");
    
    assert!(health_response.status().is_success());
    println!("✓ 健康检查通过");
    
    // 2. 注册微服务
    println!("✓ 注册微服务");
    let microservices = vec![
        ("asr-service", "http://localhost:8001"),
        ("llm-service", "http://localhost:8002"),
        ("tts-service", "http://localhost:8003"),
    ];
    
    for (service_id, endpoint) in microservices {
        let register_request = json!({
            "service_id": service_id,
            "endpoint": endpoint,
            "metadata": {
                "type": service_id.split('-').next().unwrap().to_uppercase(),
                "version": "1.0.0"
            }
        });
        
        let register_response = client
            .post(&format!("{}/api/v1/microservices/register", base_url))
            .json(&register_request)
            .send()
            .await
            .expect("微服务注册请求失败");
        
        assert!(register_response.status().is_success());
        println!("✓ 注册微服务: {}", service_id);
    }
    
    // 3. 创建会话并监听 SSE 流
    println!("✓ 创建会话并监听 SSE 流");
    let session_request = json!({
        "user_identity": "test-user-livekit-123",
        "user_name": "LiveKit Test User",
        "room_name": "livekit-integration-test-room",
        "metadata": {
            "test": "livekit_integration",
            "client_type": "integration_test"
        }
    });
    
    // 创建 SSE 客户端
    let sse_url = format!("{}/api/v1/sessions", base_url);
    println!("🔗 创建 SSE 连接到: {}", sse_url);
    println!("📤 发送会话请求: {}", serde_json::to_string_pretty(&session_request).unwrap());
    
    let mut event_source = EventSource::new(
        client
            .post(&sse_url)
            .header("Accept", "text/event-stream")
            .json(&session_request)
    ).expect("Failed to create EventSource");
    
    println!("✓ SSE 连接已建立，开始监听事件流...");
    
    let mut session_id = None;
    let mut access_token = None;
    let mut room_name = None;
    let mut livekit_url = None;
    let mut session_ready = false;
    let mut client_joined = false;
    let mut event_count = 0;
    
    // 监听 SSE 事件
    tracing::info!("开始监听 SSE 事件流");
    while let Some(event) = event_source.next().await {
        tracing::trace!("收到 SSE 事件: {:?}", event);
        match event {
            Ok(Event::Open) => {
                tracing::info!("SSE 连接已打开");
                println!("✓ SSE 连接已打开");
            }
            Ok(Event::Message(message)) => {
                tracing::info!("收到 SSE 消息: event={:?}, data_len={}", message.event, message.data.len());
                tracing::debug!("SSE 消息数据: {}", message.data);
                println!("收到 SSE 消息: event={:?}, data={}", message.event, message.data);
                
                // 解析事件数据
                if let Ok(json_data) = serde_json::from_str::<serde_json::Value>(&message.data) {
                    tracing::trace!("解析后的事件数据: {:#}", json_data);
                    println!("事件数据: {:#}", json_data);
                    
                    // 提取会话信息
                    if let Some(sid) = json_data.get("session_id").and_then(|v| v.as_str()) {
                        session_id = Some(sid.to_string());
                    }
                    if let Some(token) = json_data.get("access_token").and_then(|v| v.as_str()) {
                        access_token = Some(token.to_string());
                    }
                    if let Some(room) = json_data.get("room_name").and_then(|v| v.as_str()) {
                        room_name = Some(room.to_string());
                    }
                    if let Some(url) = json_data.get("livekit_url").and_then(|v| v.as_str()) {
                        livekit_url = Some(url.to_string());
                    }
                    if json_data.get("all_participants_joined").and_then(|v| v.as_bool()).unwrap_or(false) {
                        session_ready = true;
                    }
                    
                    // 如果收到会话创建事件且还没有作为客户端加入，尝试加入 LiveKit
                    if let (Some(ref token), Some(ref url)) = (&access_token, &livekit_url) {
                        if !client_joined {
                            tracing::info!("获得会话凭据，尝试作为客户端加入 LiveKit");
                            tracing::debug!("LiveKit URL: {}", url);
                            tracing::trace!("Access Token: {}", token);
                            println!("✓ 获得会话凭据，尝试作为客户端加入 LiveKit...");
                            
                            match test_client_join_livekit(url, token).await {
                                Ok(_) => {
                                    tracing::info!("客户端成功加入 LiveKit 房间");
                                    println!("✓ 客户端成功加入 LiveKit 房间");
                                    client_joined = true;
                                    
                                    // 等待一段时间让事件传播
                                    tracing::debug!("等待事件传播...");
                                    sleep(Duration::from_millis(2000)).await;
                                }
                                Err(e) => {
                                    tracing::error!("客户端加入 LiveKit 失败: {}", e);
                                    println!("⚠ 客户端加入 LiveKit 失败: {}", e);
                                }
                            }
                        }
                    }
                }
                
                event_count += 1;
            }
            Err(err) => {
                println!("SSE 错误: {}", err);
                break;
            }
        }
        
        // 如果会话准备就绪或达到最大事件数，退出循环
        if session_ready || event_count < -12312 {
            println!("会话准备就绪或达到最大事件数，停止监听");
            break;
        }
    }
    
    // 关闭 SSE 连接
    event_source.close();
    
    // 验证会话创建结果
    assert!(session_id.is_some(), "应该提供会话 ID");
    assert!(access_token.is_some(), "应该提供访问令牌");
    assert!(room_name.is_some(), "应该提供房间名");
    assert!(livekit_url.is_some(), "应该提供 LiveKit URL");
    assert!(client_joined, "客户端应该成功加入 LiveKit");
    
    println!("✓ LiveKit 集成测试完成");
    println!("  会话 ID: {:?}", session_id);
    println!("  房间名: {:?}", room_name);
    println!("  LiveKit URL: {:?}", livekit_url);
    println!("  客户端已加入: {}", client_joined);
    
    // 停止服务器
    server_handle.abort();
}

async fn test_client_join_livekit(livekit_url: &str, access_token: &str) -> Result<(), Box<dyn std::error::Error>> {
    tracing::info!("尝试连接到 LiveKit: {}", livekit_url);
    tracing::trace!("使用访问令牌: {}", access_token);
    println!("连接到 LiveKit: {}", livekit_url);
    
    // 连接到 LiveKit 房间
    tracing::debug!("调用 Room::connect...");
    let (room, mut event_rx) = Room::connect(livekit_url, access_token, RoomOptions::default()).await?;
    
    tracing::info!("成功连接到 LiveKit 房间: {} ({})",
        room.name(),
        room.maybe_sid().map(|s| s.to_string()).unwrap_or_else(|| "unknown".to_string())
    );
    println!("✓ 连接到 LiveKit 房间: {} ({})",
        room.name(),
        room.maybe_sid().map(|s| s.to_string()).unwrap_or_else(|| "unknown".to_string())
    );
    
    // 等待初始连接建立
    let mut event_count = 0;
    let mut connected = false;
    let mut participants_seen = 0;
    let mut webrtc_connected = false;
    let mut data_channel_ready = false;
    
    // 第一阶段：等待基本连接
    let connection_timeout = tokio::time::timeout(Duration::from_secs(10), async {
        while let Some(event) = event_rx.recv().await {
            println!("LiveKit 事件: {:?}", event);
            event_count += 1;
            
            match event {
                RoomEvent::Connected { participants_with_tracks } => {
                    connected = true;
                    participants_seen = participants_with_tracks.len();
                    println!("✓ 客户端连接到 LiveKit 房间，发现 {} 个参与者", participants_seen);
                }
                RoomEvent::ConnectionStateChanged(state) => {
                    println!("🔗 连接状态变化: {:?}", state);
                    if matches!(state, ConnectionState::Connected) {
                        webrtc_connected = true;
                        println!("✓ WebRTC 连接已建立");
                    }
                }
                RoomEvent::ParticipantConnected(participant) => {
                    println!("✓ 参与者连接: {}", participant.identity());
                    participants_seen += 1;
                }
                RoomEvent::ParticipantDisconnected(participant) => {
                    println!("✓ 参与者断开: {}", participant.identity());
                }
                RoomEvent::DataReceived { payload: _, participant: _, kind: _, topic: _ } => {
                    println!("✓ 收到数据通道消息");
                    data_channel_ready = true;
                }
                _ => {}
            }
            
            // 收到足够事件或连接建立后退出
            if event_count >= 5 || (connected && webrtc_connected) {
                break;
            }
        }
    }).await;
    
    match connection_timeout {
        Ok(_) => {
            println!("✓ 成功接收 LiveKit 连接事件");
            println!("  基本连接状态: {}", connected);
            println!("  WebRTC 连接状态: {}", webrtc_connected);
            println!("  参与者数量: {}", participants_seen);
        }
        Err(_) => {
            println!("⚠ 等待 LiveKit 连接事件超时");
        }
    }
    
    // 第二阶段：测试 WebRTC 数据通道功能
    if connected && webrtc_connected {
        println!("📡 测试 WebRTC 数据通道功能...");
        
        // 测试数据通道
        match test_data_channel(&room).await {
            Ok(_) => {
                data_channel_ready = true;
                println!("✓ 数据通道测试成功");
            }
            Err(e) => {
                println!("⚠ 数据通道测试失败: {}", e);
            }
        }
        
        // 等待数据通道事件
        let data_timeout = tokio::time::timeout(Duration::from_secs(5), async {
            while let Some(event) = event_rx.recv().await {
                println!("数据通道事件: {:?}", event);
                
                match event {
                    RoomEvent::DataReceived { payload, participant: _, kind: _, topic } => {
                        println!("✓ 收到数据: topic={:?}, size={} bytes", topic, payload.len());
                        if let Ok(message) = String::from_utf8(payload.to_vec()) {
                            println!("  消息内容: {}", message);
                        }
                        data_channel_ready = true;
                        break; // 收到数据后退出
                    }
                    _ => {}
                }
            }
        }).await;
        
        match data_timeout {
            Ok(_) => println!("✓ 数据通道功能测试完成"),
            Err(_) => println!("⚠ 数据通道功能测试超时"),
        }
    }
    
    // 输出最终状态
    println!("📊 WebRTC 连接测试结果:");
    println!("  基本连接: {}", connected);
    println!("  WebRTC 连接: {}", webrtc_connected);
    println!("  数据通道: {}", data_channel_ready);
    
    // 关闭连接
    room.close().await?;
    println!("✓ 断开 LiveKit 房间连接");
    
    // 验证 WebRTC 功能
    if !webrtc_connected {
        return Err("WebRTC 连接未建立".into());
    }
    
    Ok(())
}

async fn test_data_channel(room: &Room) -> Result<(), Box<dyn std::error::Error>> {
    println!("📡 测试数据通道...");
    
    // 发送测试数据 - 使用正确的 DataPacket 结构
    let test_data = b"Hello from WebRTC data channel!";
    let data_packet = livekit::DataPacket {
        payload: test_data.to_vec(),
        topic: Some("test-topic".to_string()),
        reliable: true,
        destination_identities: vec![],
    };
    
    room.local_participant().publish_data(data_packet).await?;
    
    println!("✓ 数据通道消息已发送");
    
    // 等待一小段时间让数据传输
    sleep(Duration::from_millis(500)).await;
    
    Ok(())
}

async fn wait_for_livekit() {
    let client = Client::new();
    let mut attempts = 0;
    const MAX_ATTEMPTS: u32 = 30;
    
    println!("等待 LiveKit 服务启动...");
    
    while attempts < MAX_ATTEMPTS {
        match client.get("http://localhost:7880").send().await {
            Ok(response) if response.status().is_success() => {
                println!("✓ LiveKit 服务已启动");
                return;
            }
            _ => {
                attempts += 1;
                println!("等待 LiveKit 服务启动... 尝试 {}/{}", attempts, MAX_ATTEMPTS);
                sleep(Duration::from_secs(2)).await;
            }
        }
    }
    
    panic!("LiveKit 服务在 {} 秒内未启动", MAX_ATTEMPTS * 2);
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
