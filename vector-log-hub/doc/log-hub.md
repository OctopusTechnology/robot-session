1. 执行摘要
本文档概述了在嵌入式环境中为用Rust、C++和Python编写的多个微服务实现集中式日志系统的架构决策。经过评估，选择了Vector作为日志聚合器，配合Grafana进行可视化，因为Vector本身不提供全面的仪表板解决方案。
2. 问题陈述
我们的嵌入式系统由多种语言（Rust、C++、Python）实现的微服务组成，需要统一的日志处理方法。关键需求包括：
- 跨语言支持所有服务
- 在资源受限环境中高效运行
- 具备处理间歇性连接的缓冲能力
- 支持结构化日志和上下文保存
- 对应用程序性能的影响最小化
- 为开发人员提供简单的集成
- 调试和监控的可视化功能
3. 评估的选项
3.1 日志收集解决方案
解决方案
优点
缺点
语言
Vector
原生Rust实现，更好的跨度支持，对Rust服务的较低开销，出色的缓冲功能
稍高的内存使用（约10-20MB），有限的内置可视化
Rust
Fluent Bit
极轻量级（约1MB），基于C可能带来性能优势
更基础的跨度处理，配置表达能力较弱
C
自定义解决方案
完全控制，可能更小的资源占用
开发开销，维护负担
多种
3.2 传输方法
方法
优点
缺点
TCP Socket
网络透明性，定义良好的协议
小开销
Unix Domain Socket
本地通信更快
限于单机
内存映射文件
本地服务极快
复杂实现
基于文件的日志
实现简单
I/O开销，可能导致存储磨损
3.3 可视化选项
选项
优点
缺点
Grafana
丰富的可视化，警报，仪表板
额外服务
自定义HTML/JS
资源使用最少，可定制
开发工作量大
Vector API
直接访问数据，无需中间存储
仅提供GraphQL操作界面，无实际仪表板
4. 架构决策
4.1 选择的架构：Vector + Grafana
我们将实现：
1. Vector作为主要日志收集器和处理器
2. TCP Socket传输用于从服务到Vector的日志传递
3. Vector的API用于数据访问
4. Grafana通过Vector API和文件日志进行可视化
4.2 理由
- 选择Vector而非Fluent Bit是因为：
  - 原生Rust实现与我们的主要服务语言一致
  - 对Rust的tracing库有更好的支持
  - 配置选项表达能力更强
  - 更好的缓冲能力
  - GraphQL API用于数据访问
- 选择TCP Socket传输是因为：
  - 跨语言兼容性
  - 网络透明性（适用于本地或分布式）
  - 未来可扩展性路径
  - 各种语言实现更简单
- Grafana是必要的，因为：
  - Vector只提供GraphQL操作界面，没有完整的仪表板
  - 监控需要丰富的可视化功能
  - 支持警报和通知
  - 可以直接查询Vector的API和日志文件
5. 实现细节
5.1 Vector配置
```TOML
[sources.tcp_logs]
type = "socket"
address = "0.0.0.0:9000"
mode = "tcp"
decoding.codec = "json"

[transforms.parse_json]
type = "remap"
inputs = ["tcp_logs"]

# Primary log storage with circular buffer behavior
[sinks.file_logs]
type = "file"
inputs = ["parse_json"]
path = "/var/log/vector/app.log"
encoding.codec = "json"
rotate.size_mb = 2         # Small 2MB files
rotate.max_files = 3       # Keep only 3 files (~6MB total)
buffer.type = "memory"
buffer.max_size = 1048576  # 1MB memory buffer
buffer.when_full = "drop_newest"  # Don't block on buffer full

[api]
enabled = true
address = "0.0.0.0:8686"```
5.2 标准JSON日志格式
```JSON
{
  "timestamp": "2023-09-28T15:04:05Z",
  "level": "info",
  "message": "操作完成",
  "service": "服务名称",
  "context": {
    "operation_id": "abc123",
    "duration_ms": 42
  }
}```
5.3 语言集成
5.3.1 Rust与Tracing的集成
```Rust
// 简单的Vector tracing层
struct VectorLayer {
    service_name: String,
    addr: String,
}

impl<S> Layer<S> for VectorLayer where S: Subscriber {
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        // 提取事件数据并发送到Vector
    }
}

// 初始化tracing
fn init_tracing() {
    let vector_layer = VectorLayer::new("rust-service", "localhost:9000");
    tracing_subscriber::registry().with(vector_layer).init();
}

// 使用示例
#[instrument]
fn process_request(user_id: u64) {
    info!("处理请求");
}```
5.3.2 Python与Logging的集成
```Python
class VectorHandler(logging.Handler):
    def __init__(self, host='localhost', port=9000, service_name='python-service'):
        super().__init__()
        self.host = host
        self.port = port
        self.service_name = service_name
    
    def emit(self, record):
        # 将记录格式化为JSON并发送到Vector
        pass

# 初始化logging
def setup_logging():
    handler = VectorHandler()
    logger = logging.getLogger()
    logger.addHandler(handler)
    return logger

# 使用示例
logger = setup_logging()
logger.info("应用程序已启动", extra={"version": "1.0"})```
5.3.3 C++与spdlog的集成
```C++
// Vector sink for spdlog
template<typename Mutex>
class vector_sink : public spdlog::sinks::base_sink<Mutex> {
public:
    vector_sink(const std::string& host, int port, const std::string& service_name)
        : host_(host), port_(port), service_name_(service_name) {}

protected:
    void sink_it_(const spdlog::details::log_msg& msg) override {
        // 格式化为JSON并发送到Vector
    }

private:
    std::string host_;
    int port_;
    std::string service_name_;
};

// 初始化logger
auto setup_logger() {
    auto sink = std::make_shared<vector_sink<std::mutex>>("localhost", 9000, "cpp-service");
    auto logger = std::make_shared<spdlog::logger>("vector_logger", sink);
    return logger;
}

// 使用示例
auto logger = setup_logger();
logger->info("启动应用程序");```
6. 部署架构
```Plain Text
┌─────────────────┐     ┌───────────┐     ┌─────────────┐
│  微服务         │     │           │     │             │
│  ┌───────────┐  │     │           │     │             │
│  │ Rust 服务 ├──┼────►│           │     │             │
│  └───────────┘  │     │           │     │             │
│  ┌───────────┐  │     │  Vector   ├────►│  Grafana    │
│  │Python服务 ├──┼────►│           │     │             │
│  └───────────┘  │     │           │     │             │
│  ┌───────────┐  │     │           │     │             │
│  │ C++ 服务  ├──┼────►│           │     │             │
│  └───────────┘  │     │           │     │             │
└─────────────────┘     └───────────┘     └─────────────┘
      TCP:9000            API:8686          HTTP:3000```
7. 开发环境
在开发过程中：
- 使用Vector的GraphQL操作界面进行数据检查
- 使用Grafana进行日志可视化
- 设置简单的文件监控日志
8. 生产环境考虑因素
- 实现离线操作的缓冲
- 配置适当的日志保留策略
- 为嵌入式存储限制实现日志轮转
- 考虑暴露端点的安全影响
9. 未来增强
1. 如果系统扩展，实现OpenTelemetry
2. 在日志收集的同时添加指标收集
3. 探索通过Grafana实现告警功能
4. 考虑日志压缩以提高存储效率
10. 结论
Vector + Grafana架构提供了一个强大、高效的日志解决方案，满足了我们对跨语言支持、最小资源使用和综合可视化功能的要求。Vector负责收集和处理，而Grafana提供了Vector的GraphQL操作界面无法单独提供的可视化功能。