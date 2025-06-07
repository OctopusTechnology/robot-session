# Session Architecture Summary

## Project Overview

**robot-session** is a sophisticated Rust-based intelligent robot backend session management system built on top of LiveKit. It provides a distributed microservice architecture for managing real-time communication sessions between users and AI services, with comprehensive session lifecycle management, event-driven architecture, and robust error handling.

## Core Architecture Components

### 1. Session Manager (`session-manager/`)

The central orchestrator that manages LiveKit rooms, microservice registration, and complete session lifecycle.

#### Key Features:
- **Session Creation & Management**: Full lifecycle from creation to termination
- **Microservice Registration & Discovery**: Dynamic service registry with health monitoring
- **LiveKit Integration**: Room management, token generation, and real-time monitoring
- **Event-Driven Architecture**: Real-time event broadcasting with SSE support
- **In-Memory Storage**: High-performance concurrent storage using DashMap
- **Vector Logging**: Structured logging with external aggregation support
- **Timeout & Retry Logic**: Robust handling of service failures and reconnections

#### Module Structure:

**Configuration Layer** ([`config.rs`](session-manager/src/config.rs:5))
- [`AppConfig`](session-manager/src/config.rs:5): Main configuration structure
- [`ServerConfig`](session-manager/src/config.rs:14): HTTP server settings
- [`LiveKitConfig`](session-manager/src/config.rs:21): LiveKit connection parameters
- [`MicroserviceConfig`](session-manager/src/config.rs:28): Service timeout configurations
- [`LoggingConfig`](session-manager/src/config.rs:34): Logging level and format
- [`VectorLogConfig`](session-manager/src/config.rs:40): Vector logging integration

**HTTP Server Layer** ([`server.rs`](session-manager/src/server.rs:21))
- [`Server`](session-manager/src/server.rs:21): Main server struct with Axum integration
- CORS and tracing middleware
- Dependency injection for services and storage

**API Layer** ([`api/`](session-manager/src/api/))
- [`handlers.rs`](session-manager/src/api/handlers.rs:16): REST endpoint implementations
  - [`health_check()`](session-manager/src/api/handlers.rs:24): System health monitoring
  - [`register_microservice()`](session-manager/src/api/handlers.rs:33): Service registration
  - [`create_session()`](session-manager/src/api/handlers.rs:54): Session creation endpoint
- [`models.rs`](session-manager/src/api/models.rs:8): Request/response data structures
- [`AppState`](session-manager/src/api/handlers.rs:16): Shared application state

**Domain Layer** ([`domain/`](session-manager/src/domain/))
- [`session.rs`](session-manager/src/domain/session.rs:14): Core session business logic
  - [`Session`](session-manager/src/domain/session.rs:14): Main session entity with 618 lines of complex logic
  - [`SessionStatus`](session-manager/src/domain/session.rs:39): Session state machine (Creating, WaitingForServices, Ready, Active, Terminating, Terminated)
  - [`SessionRoomConnection`](session-manager/src/domain/session.rs:32): LiveKit connection management
- [`microservice.rs`](session-manager/src/domain/microservice.rs:6): Microservice domain models
  - [`MicroserviceInfo`](session-manager/src/domain/microservice.rs:6): Service metadata and status
  - [`ServiceStatus`](session-manager/src/domain/microservice.rs:15): Service state (Registered, Joining, Ready, Disconnected)
  - [`JoinRoomRequest`](session-manager/src/domain/microservice.rs:43)/[`JoinRoomResponse`](session-manager/src/domain/microservice.rs:52): Service communication protocols

**Services Layer** ([`services/`](session-manager/src/services/))
- [`session_service.rs`](session-manager/src/services/session_service.rs:29): Core session business logic
  - [`SessionService`](session-manager/src/services/session_service.rs:15) trait: Service interface
  - [`SessionServiceImpl`](session-manager/src/services/session_service.rs:29): Implementation with comprehensive session management
- [`livekit_service.rs`](session-manager/src/services/livekit_service.rs:15): LiveKit integration service
  - [`LiveKitService`](session-manager/src/services/livekit_service.rs:15): Room management and token generation
  - Real-time event monitoring and participant tracking
- [`microservice_registry.rs`](session-manager/src/services/microservice_registry.rs:9): Service discovery
  - [`MicroserviceRegistry`](session-manager/src/services/microservice_registry.rs:9): Thread-safe service registry

**Event System** ([`events.rs`](session-manager/src/events.rs:42))
- [`EventBus`](session-manager/src/events.rs:42): Centralized event broadcasting
- [`SessionEvent`](session-manager/src/events.rs:9): Comprehensive event types
  - [`SessionCreated`](session-manager/src/events.rs:10), [`MicroserviceJoined`](session-manager/src/events.rs:16), [`ClientJoined`](session-manager/src/events.rs:20)
  - [`SessionReady`](session-manager/src/events.rs:24), [`SessionStatusChanged`](session-manager/src/events.rs:28), [`Error`](session-manager/src/events.rs:32)
- [`SessionParticipantTracker`](session-manager/src/events.rs:135): Participant lifecycle monitoring

**Storage Layer** ([`storage/`](session-manager/src/storage/))
- [`SessionStorage`](session-manager/src/storage/mod.rs:7) trait: Storage abstraction
- [`MemoryStorage`](session-manager/src/storage/memory.rs:11): High-performance in-memory implementation using DashMap

**Error Handling** ([`utils/errors.rs`](session-manager/src/utils/errors.rs:4))
- [`SessionManagerError`](session-manager/src/utils/errors.rs:4): Comprehensive error types
- Structured error handling with proper HTTP status mapping

### 2. Microservice SDK (`microservice-sdk/`)

A comprehensive Rust SDK enabling microservices to integrate seamlessly with the session manager.

#### Key Features:
- **Automatic Registration**: Self-registration with session manager
- **LiveKit Integration**: Direct room joining with token management
- **HTTP Server**: Built-in server for receiving session commands
- **Health Monitoring**: Automated health check endpoints
- **Configurable Metadata**: Service capability advertisement
- **Timeout Management**: Configurable request timeouts

#### SDK Components:
- [`client.rs`](microservice-sdk/src/client.rs:14): Session manager communication client
- [`models.rs`](microservice-sdk/src/models.rs:6): Configuration and request models
- [`traits.rs`](microservice-sdk/src/traits.rs:6): [`MicroserviceHandler`](microservice-sdk/src/traits.rs:6) trait for service implementation
- [`examples/simple_microservice.rs`](microservice-sdk/examples/simple_microservice.rs:20): Complete ping-pong service example

## Detailed Architecture Flow

### 1. Microservice Registration Flow
```
Microservice Startup → SDK Registration → Session Manager Registry
1. Microservice starts with SDK configuration
2. SDK sends POST /api/v1/microservices/register
3. Session manager stores service in MicroserviceRegistry
4. Health monitoring initiated
5. Service marked as Available
```

### 2. Session Creation Flow
```
Client Request → Session Creation → LiveKit Setup → Service Notification → Monitoring
1. Client sends POST /api/v1/create-session with required_services
2. SessionService creates Session entity
3. Session creates LiveKit room via API
4. Session connects to LiveKit for monitoring
5. Session generates tokens for microservices
6. Session notifies microservices to join (async)
7. Session monitors participant joins via LiveKit events
8. Session status updates: Creating → WaitingForServices → Ready
9. Client receives access token and can join
```

### 3. Real-time Communication Flow
```
Client ↔ LiveKit Room ↔ Microservices ↔ Session Manager
- Audio/video streams flow through LiveKit
- Data channel messages for service communication
- Participant events monitored by Session Manager
- Session lifecycle managed with timeouts and retries
- Event broadcasting to interested parties
```

### 4. Session Lifecycle Management
```
Session States: Creating → WaitingForServices → Ready → Active → Terminating → Terminated

Monitoring Features:
- Participant join/leave detection
- Service timeout handling (60s)
- Client timeout handling (300s)
- Automatic retry for failed services (30s intervals)
- Graceful session termination
```

## Key Data Models & State Management

### Session Model ([`Session`](session-manager/src/domain/session.rs:14))
```rust
pub struct Session {
    pub id: String,                              // Unique session identifier
    pub room_name: String,                       // LiveKit room name
    pub status: SessionStatus,                   // Current session state
    pub created_at: DateTime<Utc>,              // Creation timestamp
    pub updated_at: DateTime<Utc>,              // Last update timestamp
    pub client_token: Option<String>,           // Client access token
    pub registered_microservices: Vec<MicroserviceInfo>, // Required services
    pub ready_microservices: HashSet<String>,   // Services that joined
    pub metadata: HashMap<String, String>,      // Custom session data
    pub room_connection: Option<Arc<RwLock<SessionRoomConnection>>>, // LiveKit connection
}
```

### Microservice Model ([`MicroserviceInfo`](session-manager/src/domain/microservice.rs:6))
```rust
pub struct MicroserviceInfo {
    pub service_id: String,                     // Unique service identifier
    pub endpoint: String,                       // HTTP endpoint for communication
    pub status: ServiceStatus,                  // Current service state
    pub registered_at: DateTime<Utc>,          // Registration timestamp
    pub metadata: HashMap<String, String>,     // Service capabilities
}
```

### Event Model ([`SessionEvent`](session-manager/src/events.rs:9))
```rust
pub enum SessionEvent {
    SessionCreated { session_id, room_name, access_token, livekit_url },
    MicroserviceJoined { session_id, service_id },
    ClientJoined { session_id, user_identity },
    SessionReady { session_id, all_participants_joined },
    SessionStatusChanged { session_id, status },
    Error { session_id, message },
}
```

## API Endpoints

### Session Manager REST API
- `GET /health` - System health check with version info
- `POST /api/v1/microservices/register` - Register microservice with metadata
- `POST /api/v1/create-session` - Create new session with optional service requirements

### Microservice SDK API (Auto-generated)
- `POST /join-room` - Join LiveKit room (called by session manager)
- `GET /health` - Service health check

## Configuration Management

### Session Manager Configuration ([`AppConfig`](session-manager/src/config.rs:5))
```toml
[server]
host = "0.0.0.0"
port = 8080
workers = 4

[livekit]
server_url = "ws://localhost:7880"
api_key = "devkey"
api_secret = "secret"

[microservices]
registration_timeout = 30
join_timeout = 60

[logging]
level = "debug"
format = "json"

[vector_log]
enabled = true
endpoint = "http://localhost:8686"
source_name = "session-manager"
```

### Environment Variable Overrides
- `LIVEKIT_API_KEY` / `LIVEKIT_API_SECRET`: LiveKit authentication
- `LIVEKIT_SERVER_URL`: LiveKit server endpoint
- `SERVER_HOST` / `SERVER_PORT`: HTTP server binding
- `VECTOR_LOG_ENABLED` / `VECTOR_LOG_ENDPOINT`: Logging configuration

## Deployment Architecture

### Docker Compose Setup ([`docker-compose.yml`](docker-compose.yml:1))
```yaml
services:
  livekit:
    image: livekit/livekit-server:latest
    volumes:
      - ./config.yaml:/etc/livekit.yaml
    network_mode: host
    
  session-manager:
    build: ./session-manager
    network_mode: host
    depends_on: [livekit]
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8080/health"]
```

### LiveKit Configuration ([`config.yaml`](config.yaml:1))
- **Port Configuration**: Main TCP port 7880, WebRTC ports 52000-60000
- **Authentication**: API key/secret pairs for JWT token generation
- **Room Settings**: Auto-creation, timeouts, participant limits
- **Network**: Interface filtering, IP address management
- **Logging**: Configurable levels and JSON output

## Technology Stack & Dependencies

### Core Technologies
- **Rust 2021 Edition**: Systems programming with memory safety
- **Axum 0.8**: Modern async web framework with excellent performance
- **LiveKit SDK**: Real-time communication platform integration
- **Tokio**: Async runtime with full feature set
- **Serde**: Serialization with derive macros

### Key Dependencies ([`Cargo.toml`](session-manager/Cargo.toml:1))
- **livekit/livekit-api 0.4.3**: LiveKit Rust SDK with access tokens and services
- **dashmap 6.1.0**: Concurrent hash map for thread-safe storage
- **tracing/tracing-subscriber**: Structured logging with spans and events
- **reqwest 0.12.19**: HTTP client for microservice communication
- **uuid 1.0**: Unique identifier generation
- **chrono 0.4**: Date/time handling with UTC timestamps
- **anyhow/thiserror**: Error handling and propagation

## Event-Driven Architecture

### Event Bus Implementation ([`EventBus`](session-manager/src/events.rs:42))
- **Global Broadcasting**: System-wide event distribution
- **Session-Specific Streams**: Isolated event channels per session
- **Participant Tracking**: Automatic join/leave detection
- **Real-time Updates**: Server-Sent Events (SSE) support for live monitoring

### Event Flow Patterns
1. **Session Events**: Creation, status changes, completion
2. **Participant Events**: Microservice and client join/leave
3. **Error Events**: Timeout, communication failures, system errors
4. **Monitoring Events**: Health checks, service availability

## Storage Strategy & Performance

### In-Memory Storage ([`MemoryStorage`](session-manager/src/storage/memory.rs:11))
- **High Performance**: Zero database overhead with microsecond access times
- **Thread Safety**: DashMap provides lock-free concurrent access
- **Session Lifecycle**: Ephemeral storage matching session duration
- **Scalability**: Horizontal scaling through stateless design

### Storage Interface ([`SessionStorage`](session-manager/src/storage/mod.rs:7))
```rust
#[async_trait]
pub trait SessionStorage: Send + Sync {
    async fn save_session(&self, session: &Session) -> Result<()>;
    async fn get_session(&self, session_id: &str) -> Result<Option<Session>>;
    async fn update_session(&self, session: &Session) -> Result<()>;
    async fn delete_session(&self, session_id: &str) -> Result<()>;
    async fn list_sessions(&self) -> Result<Vec<Session>>;
}
```

## Security & Authentication

### LiveKit Token Management
- **JWT-based Authentication**: Secure token generation with configurable TTL
- **Role-based Access**: Different permissions for clients, services, and managers
- **Token Scoping**: Room-specific access with granular permissions
- **Automatic Renewal**: Long-lived tokens for persistent services

### Network Security
- **CORS Configuration**: Permissive CORS for development, configurable for production
- **TLS Support**: HTTPS/WSS support for encrypted communication
- **API Key Authentication**: Secure service-to-service communication
- **Endpoint Validation**: Input validation and sanitization

## Error Handling & Resilience

### Comprehensive Error Types ([`SessionManagerError`](session-manager/src/utils/errors.rs:4))
```rust
pub enum SessionManagerError {
    SessionNotFound { session_id: String },
    LiveKit(livekit_api::services::ServiceError),
    Storage(String),
    MicroserviceCommunication(reqwest::Error),
    Configuration(String),
    MicroserviceJoinTimeout,
    InvalidRequest(String),
    Internal(anyhow::Error),
}
```

### Resilience Features
- **Timeout Management**: Configurable timeouts for all operations
- **Retry Logic**: Automatic retry for failed service communications
- **Circuit Breaking**: Service health monitoring with automatic recovery
- **Graceful Degradation**: Session continues even if some services fail
- **Resource Cleanup**: Automatic cleanup of failed or terminated sessions

## Monitoring & Observability

### Structured Logging
- **Tracing Integration**: Span-based logging with context propagation
- **Vector Support**: External log aggregation and analysis
- **Performance Metrics**: Request timing and resource usage
- **Error Tracking**: Comprehensive error logging with stack traces

### Health Monitoring
- **Service Health Checks**: Automatic health monitoring for all services
- **Session Metrics**: Real-time session status and participant counts
- **System Health**: Overall system health with version information
- **Resource Monitoring**: Memory usage and connection tracking

## Scalability & Performance

### Horizontal Scaling
- **Stateless Design**: Session manager instances can be replicated
- **Service Discovery**: Dynamic microservice registration and load balancing
- **Room Isolation**: Independent session management without cross-dependencies
- **Event Distribution**: Scalable event broadcasting architecture

### Performance Optimizations
- **Async Architecture**: Non-blocking I/O with Tokio runtime
- **Zero-Copy Operations**: Efficient data handling with minimal allocations
- **Connection Pooling**: Reused HTTP connections for service communication
- **Memory Efficiency**: Rust's ownership model prevents memory leaks

## Development Workflow

### Local Development Setup
1. **Start LiveKit**: `docker-compose up livekit`
2. **Run Session Manager**: `cd session-manager && cargo run`
3. **Implement Microservice**: Use SDK with [`MicroserviceHandler`](microservice-sdk/src/traits.rs:6) trait
4. **Test Integration**: Register service and create sessions

### Testing Strategy
- **Unit Tests**: Individual component testing with mocks
- **Integration Tests**: End-to-end session flows with real LiveKit
- **Example Services**: Reference implementations for common patterns
- **Health Monitoring**: Automated service health verification

## Future Architecture Considerations

### Potential Enhancements
- **Persistent Storage**: Database integration for session history and analytics
- **Service Mesh**: Advanced microservice communication with Istio/Linkerd
- **Auto-scaling**: Dynamic service instance management based on load
- **Advanced Monitoring**: Prometheus metrics and Grafana dashboards
- **Security Hardening**: Enhanced authentication, authorization, and audit logging

### Extension Points
- **Custom Event Handlers**: Pluggable event processing for specialized workflows
- **Storage Backends**: Alternative storage implementations (Redis, PostgreSQL)
- **Protocol Support**: Additional communication protocols (gRPC, WebSocket)
- **Service Types**: Specialized microservice categories with custom lifecycle management

This architecture provides a robust, scalable foundation for building intelligent robot applications with real-time communication capabilities, leveraging Rust's performance and safety guarantees while maintaining flexibility for diverse microservice implementations and deployment scenarios.