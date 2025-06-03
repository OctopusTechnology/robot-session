# Session Manager Web API 详细文档

## 概述

Session Manager 是一个基于 LiveKit 的智能机器人后端会话管理系统，提供 RESTful API 接口用于管理实时通信会话和微服务注册。

## 基础信息

- **基础 URL**: `http://localhost:8080`
- **API 版本**: v1
- **内容类型**: `application/json`
- **字符编码**: UTF-8

## 认证

当前版本不需要认证，但建议在生产环境中添加适当的认证机制。

## 错误处理

所有 API 错误响应都遵循统一格式：

```json
{
  "error": "错误类型",
  "message": "详细错误信息",
  "timestamp": "2024-01-01T12:00:00Z"
}
```

### 常见错误码

- `400 Bad Request`: 请求参数无效
- `404 Not Found`: 资源不存在
- `408 Request Timeout`: 请求超时
- `500 Internal Server Error`: 服务器内部错误

## API 接口详情

### 1. 系统健康检查

检查系统运行状态和版本信息。

**接口地址**: `GET /health`

**请求参数**: 无

**响应示例**:
```json
{
  "status": "healthy",
  "timestamp": "2024-01-01T12:00:00Z",
  "version": "0.1.0"
}
```

**响应字段说明**:
- `status`: 系统状态，固定为 "healthy"
- `timestamp`: 响应时间戳
- `version`: 系统版本号

---

### 2. 微服务注册

注册微服务到会话管理器，使其能够参与会话。

**接口地址**: `POST /api/v1/microservices/register`

**请求头**:
```
Content-Type: application/json
```

**请求参数**:
```json
{
  "service_id": "asr-service-1",
  "endpoint": "http://localhost:8001",
  "metadata": {
    "type": "ASR",
    "version": "1.0.0",
    "capabilities": "speech-recognition",
    "language": "zh-CN"
  }
}
```

**请求字段说明**:
- `service_id` (必填): 微服务唯一标识符
- `endpoint` (必填): 微服务 HTTP 端点地址
- `metadata` (可选): 微服务元数据信息
  - `type`: 服务类型 (如 ASR, TTS, LLM)
  - `version`: 服务版本
  - `capabilities`: 服务能力描述
  - `language`: 支持的语言

**响应示例**:
```json
{
  "success": true,
  "service_id": "asr-service-1",
  "message": "Microservice registered successfully"
}
```

**响应字段说明**:
- `success`: 注册是否成功
- `service_id`: 已注册的服务 ID
- `message`: 操作结果消息

**错误响应示例**:
```json
{
  "error": "InvalidRequest",
  "message": "Service ID already exists",
  "timestamp": "2024-01-01T12:00:00Z"
}
```

---

### 3. 创建会话

创建新的实时通信会话，可指定需要的微服务。

**接口地址**: `POST /api/v1/create-session`

**请求头**:
```
Content-Type: application/json
```

**请求参数**:
```json
{
  "user_identity": "user123",
  "user_name": "张三",
  "room_name": "meeting-room-001",
  "metadata": {
    "purpose": "语音助手对话",
    "language": "zh-CN",
    "duration_limit": "3600"
  },
  "required_services": ["asr-service-1", "llm-service-1", "tts-service-1"]
}
```

**请求字段说明**:
- `user_identity` (必填): 用户唯一标识符
- `user_name` (可选): 用户显示名称
- `room_name` (可选): 自定义房间名称，不提供则自动生成
- `metadata` (可选): 会话元数据
  - `purpose`: 会话目的
  - `language`: 会话语言
  - `duration_limit`: 时长限制（秒）
- `required_services` (可选): 需要的微服务列表，不提供则使用所有可用服务

**响应示例**:
```json
{
  "session_id": "550e8400-e29b-41d4-a716-446655440000",
  "room_name": "room-550e8400-e29b-41d4-a716-446655440000",
  "access_token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
  "livekit_url": "ws://localhost:7880",
  "status": "WaitingForServices"
}
```

**响应字段说明**:
- `session_id`: 会话唯一标识符
- `room_name`: LiveKit 房间名称
- `access_token`: 客户端访问令牌（JWT 格式）
- `livekit_url`: LiveKit 服务器 WebSocket 地址
- `status`: 会话状态
  - `Creating`: 正在创建
  - `WaitingForServices`: 等待微服务加入
  - `Ready`: 准备就绪
  - `Active`: 活跃状态
  - `Terminating`: 正在终止
  - `Terminated`: 已终止

**会话创建流程**:
1. 系统生成唯一的会话 ID 和房间名称
2. 创建 LiveKit 房间
3. 生成客户端访问令牌
4. 通知指定的微服务加入房间
5. 监控微服务加入状态
6. 返回会话信息给客户端

**错误响应示例**:
```json
{
  "error": "Timeout",
  "message": "Timeout waiting for microservices to join",
  "timestamp": "2024-01-01T12:00:00Z"
}
```

---

## 使用示例

### 完整会话创建流程

1. **微服务注册**:
```bash
curl -X POST http://localhost:8080/api/v1/microservices/register \
  -H "Content-Type: application/json" \
  -d '{
    "service_id": "asr-service-1",
    "endpoint": "http://localhost:8001",
    "metadata": {
      "type": "ASR",
      "language": "zh-CN"
    }
  }'
```

2. **创建会话**:
```bash
curl -X POST http://localhost:8080/api/v1/create-session \
  -H "Content-Type: application/json" \
  -d '{
    "user_identity": "user123",
    "user_name": "张三",
    "required_services": ["asr-service-1"]
  }'
```

3. **客户端连接**:
使用返回的 `access_token` 和 `livekit_url` 连接到 LiveKit 房间。

### JavaScript 客户端示例

```javascript
import { Room, RoomEvent, RemoteTrack } from 'livekit-client';

// 创建会话
const response = await fetch('http://localhost:8080/api/v1/create-session', {
  method: 'POST',
  headers: {
    'Content-Type': 'application/json',
  },
  body: JSON.stringify({
    user_identity: 'user123',
    user_name: '张三',
    required_services: ['asr-service-1']
  })
});

const sessionData = await response.json();

// 创建 LiveKit 房间实例
const room = new Room({
  // 自动管理订阅的视频质量
  adaptiveStream: true,
  // 优化发布带宽和 CPU
  dynacast: true,
});

// 设置事件监听器
room.on(RoomEvent.Connected, () => {
  console.log('已连接到房间:', room.name);
});

room.on(RoomEvent.DataReceived, (payload, participant) => {
  const message = new TextDecoder().decode(payload);
  console.log(`收到来自 ${participant?.identity || 'unknown'} 的消息: ${message}`);
});

room.on(RoomEvent.ParticipantConnected, (participant) => {
  console.log('参与者加入:', participant.identity);
});

room.on(RoomEvent.Disconnected, () => {
  console.log('已断开连接');
});

// 连接到房间
await room.connect(sessionData.livekit_url, sessionData.access_token);

// 发送数据消息
const encoder = new TextEncoder();
const data = encoder.encode('Hello, 微服务!');
await room.localParticipant.publishData(data);

// 启用摄像头和麦克风（可选）
await room.localParticipant.setCameraEnabled(true);
await room.localParticipant.setMicrophoneEnabled(true);
```

## 最佳实践

### 1. 错误处理
- 始终检查 HTTP 状态码
- 解析错误响应中的详细信息
- 实现重试机制处理临时性错误

### 2. 会话管理
- 及时清理不再使用的会话
- 监控会话状态变化
- 处理网络断线重连

### 3. 微服务集成
- 确保微服务健康检查正常
- 实现优雅的服务启动和关闭
- 处理服务超时和重连

### 4. 性能优化
- 复用 HTTP 连接
- 合理设置超时时间
- 监控系统资源使用

## 故障排除

### 常见问题

1. **微服务注册失败**
   - 检查服务 ID 是否重复
   - 验证端点地址是否可访问
   - 确认请求格式正确

2. **会话创建超时**
   - 检查微服务是否正常运行
   - 验证 LiveKit 服务器连接
   - 查看系统日志获取详细错误

3. **客户端连接失败**
   - 验证访问令牌有效性
   - 检查 LiveKit 服务器地址
   - 确认网络连接正常

### 日志级别

- `ERROR`: 系统错误和异常
- `WARN`: 警告信息和潜在问题
- `INFO`: 一般操作信息
- `DEBUG`: 详细调试信息
- `TRACE`: 最详细的跟踪信息

## 版本历史

- **v0.1.0**: 初始版本
  - 基础会话管理功能
  - 微服务注册和发现
  - LiveKit 集成
  - 事件系统

## 技术支持

如有问题或建议，请通过以下方式联系：

- 项目仓库: [GitHub Repository]
- 文档更新: 请查看最新版本文档
- 技术讨论: 欢迎提交 Issue 或 Pull Request