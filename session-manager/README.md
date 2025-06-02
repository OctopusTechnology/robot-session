# Session Manager

基于 LiveKit 的智能机器人后端会话管理器，使用 Rust 实现。

## 功能特性

- **会话管理**: 创建和管理 LiveKit 房间会话
- **微服务注册**: 支持动态微服务注册和管理
- **令牌生成**: 自动生成 LiveKit 访问令牌
- **状态跟踪**: 实时跟踪会话和微服务状态
- **内存存储**: 使用高性能内存存储（无需数据库）
- **Vector 日志**: 集成 Vector 日志聚合系统

## 快速开始

### 环境要求

- Rust 1.75+
- LiveKit 服务器运行在 `ws://localhost:7880`

### 安装和运行

```bash
# 克隆项目
cd session-manager

# 构建项目
cargo build

# 运行开发服务器
cargo run
```

### 环境变量

```bash
export LIVEKIT_API_KEY="your-api-key"
export LIVEKIT_API_SECRET="your-api-secret"
export LIVEKIT_SERVER_URL="ws://localhost:7880"
export VECTOR_LOG_ENABLED="true"
export VECTOR_LOG_ENDPOINT="http://localhost:8686"
```

## API 接口

### 健康检查

```bash
GET /health
```

### 注册微服务

```bash
POST /api/v1/microservices/register
Content-Type: application/json

{
  "service_id": "asr-service-1",
  "endpoint": "http://localhost:8001",
  "metadata": {
    "type": "ASR",
    "version": "1.0.0"
  }
}
```

### 创建会话

```bash
POST /api/v1/sessions
Content-Type: application/json

{
  "user_identity": "user123",
  "user_name": "John Doe",
  "room_name": "my-room",
  "metadata": {},
  "required_services": ["asr-service-1", "llm-service-1"]
}
```

### 查询会话状态

```bash
GET /api/v1/sessions/{session_id}
```

### 通知服务就绪

```bash
POST /api/v1/sessions/{session_id}/ready
Content-Type: application/json

{
  "service_id": "asr-service-1"
}
```

## 架构设计

### 核心组件

1. **会话管理器**: 协调房间生命周期和微服务
2. **LiveKit 集成**: 房间管理和令牌生成
3. **微服务注册表**: 管理可用的微服务
4. **内存存储**: 高性能会话状态存储

### 数据流

1. 微服务启动时注册到会话管理器
2. 用户请求创建会话
3. 会话管理器创建 LiveKit 房间
4. 通知相关微服务加入房间
5. 等待所有微服务就绪
6. 返回访问令牌给用户

## 开发

### 项目结构

```
src/
├── main.rs              # 应用入口
├── lib.rs               # 库根模块
├── config.rs            # 配置管理
├── server.rs            # HTTP 服务器
├── api/                 # API 层
│   ├── handlers.rs      # HTTP 处理器
│   └── models.rs        # API 数据模型
├── services/            # 业务服务层
│   ├── session_service.rs
│   ├── livekit_service.rs
│   └── microservice_registry.rs
├── domain/              # 领域模型
│   ├── session.rs
│   └── microservice.rs
├── storage/             # 存储层
│   └── memory.rs
└── utils/               # 工具模块
    └── errors.rs
```

### 运行测试

```bash
cargo test
```

### 构建发布版本

```bash
cargo build --release
```

## 部署

### Docker

```dockerfile
FROM rust:1.75 as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/session-manager /usr/local/bin/
EXPOSE 8080
CMD ["session-manager"]
```

### 环境变量

- `LIVEKIT_API_KEY`: LiveKit API 密钥
- `LIVEKIT_API_SECRET`: LiveKit API 秘密
- `LIVEKIT_SERVER_URL`: LiveKit 服务器地址
- `SERVER_HOST`: 服务器监听地址 (默认: 0.0.0.0)
- `SERVER_PORT`: 服务器端口 (默认: 8080)
- `RUST_LOG`: 日志级别 (默认: debug)

## 许可证

MIT License