# Spike: Mock gRPC Server 可行性验证

## 目标

验证能否在测试 target 里启动一个 plaintext gRPC server，供 XCUITest 和集成测试使用。

## 验证内容

### 1. SPM server 依赖（只加到测试 target）

在 `Package.swift` 中新增一个测试 target（如 `GRPCTestSupport`），依赖 server 端：
- `GRPCNIOTransportHTTP2`（server transport，**非** TransportServices）
- `GRPCProtobuf`（复用 proto 生成代码）

**注意**：`GRPCNIOTransportHTTP2` 是 NIO posix transport，适合 server 端和 Linux；`GRPCNIOTransportHTTP2TransportServices` 是 Network.framework transport，适合 iOS client。Server 不能用 TransportServices。

**不要把 server 依赖加到 `GRPCClient` 或 App target 里。**

### 2. 最小 Mock Server

写一个最小的 mock server，实现 `AuthService.Login` 一个方法就够：

```swift
// 大致思路（伪代码）
import GRPCNIOTransportHTTP2

struct MockAuthService: AgentPlatform_Auth_AuthService.ServiceProtocol {
    func login(request: ...) async throws -> ... {
        // 返回硬编码的 token
    }
    // 其他方法 unimplemented
}

let server = GRPCServer(
    transport: .http2NIOPosix(address: .ipv4(host: "127.0.0.1", port: 0)),
    services: [MockAuthService()]
)
```

### 3. Plaintext Client 连接

验证 App 的 `APIClient` 能以 plaintext 模式连接 mock server。

当前 `APIClient.withClient` 硬编码了 `transportSecurity: .tls`。需要支持 plaintext：

```swift
// APIClient.swift — 加一个参数或环境变量控制
let usePlaintext = ProcessInfo.processInfo.environment["SERVER_PLAINTEXT"] == "true"
let transportSecurity: HTTP2ClientTransport.TransportServices.TransportSecurity = usePlaintext ? .plaintext : .tls
```

**这是唯一需要改 App 代码的地方**，而且只是传输层配置，不是测试逻辑。

### 4. 端到端通信验证

在测试中：
1. 启动 mock server（port 0，系统分配端口）
2. 拿到实际端口号
3. 创建 plaintext client 连接该端口
4. 调用 Login → 收到预期响应

## 验收标准

- [ ] `Package.swift` 新增测试辅助 target，依赖 server transport，编译通过
- [ ] Mock server 能启动并监听端口
- [ ] Plaintext client 能连上 mock server 并完成一次 RPC 调用
- [ ] App target 不包含任何 server 依赖
- [ ] 现有测试仍然通过

## 参考

- `Package.swift` 当前结构：只有 `GRPCClient` 一个 target
- `APIClient.swift`：`Sources/GRPCClient/APIClient.swift`（环境变量读 host/port）
- grpc-swift v2 server 文档：https://github.com/grpc/grpc-swift
- Proto 文件：`../../proto/`（auth.proto, chat.proto, session.proto）
- 服务端 e2e 测试（Rust 版同一思路）：`../../server/crates/agent-e2e/`

## 约束

- Server 依赖只加到测试 target，不污染 App target
- 不要改 proto 文件
- 不要改现有测试
- 如果遇到阻塞问题（如 grpc-swift v2 server API 不可用），记录问题并停止，不要绕路
