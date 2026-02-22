# mobile — iOS 客户端

## 技术栈

- iOS 26.2 / Swift 5.0 / SwiftUI
- Xcode 26.2
- gRPC: grpc-swift v2（grpc-swift-protobuf + grpc-swift-nio-transport）
- 服务端地址: `REDACTED_HOST:8443`（gRPC over TLS）

## 项目结构

```
mobile/
├── AgentDemo/
│   ├── AgentDemo/           # 主 App target
│   │   ├── Views/           # SwiftUI 视图（LoginView, ChatView）
│   │   ├── ViewModels/      # ViewModel + Model（ChatViewModel, ChatMessage）
│   │   ├── Protocols/       # 协议定义（SessionServiceProtocol）
│   │   ├── Services/        # 服务适配器（SessionServiceAdapter）
│   │   ├── Utilities/       # 工具（KeychainHelper）
│   │   ├── AppState.swift   # 全局状态（token Keychain 持久化）
│   │   └── ContentView.swift
│   ├── Sources/GRPCClient/  # gRPC 客户端（独立 SPM module）
│   │   ├── APIClient.swift  # gRPC 连接管理
│   │   ├── AuthServiceClient.swift
│   │   ├── ChatServiceClient.swift
│   │   ├── ChatServiceProtocol.swift
│   │   └── SessionServiceClient.swift
│   └── AgentDemoTests/      # 单元测试（Swift Testing framework）
├── docs/design/             # 架构设计文档
├── scripts/
│   └── check-file-size.sh   # 文件大小检查
├── .swiftformat             # SwiftFormat 配置
└── .swiftlint.yml           # SwiftLint 配置
```

## 架构要点

- **GRPCClient 是独立 SPM module**：Proto 生成的代码和客户端封装在 `Sources/GRPCClient/`
- **协议注入**：ChatServiceProtocol、SessionServiceProtocol 用于依赖注入和测试
- **Proto 由 SPM Build Plugin 自动生成**：grpc-swift v2 plugin，config key `generatedSource.accessLevel: "Public"`
- **四层架构**：应用集成層 → 業務層 → サービス層 → 基礎層

## 当前功能

- ✅ 注册 / 登录（AuthService Login + Register）
- ✅ 聊天（ChatService SendMessage，AI 整块返回）
- ✅ Token 持久化（Keychain）+ Session ID 持久化（UserDefaults）
- ✅ 聊天历史加载（SessionService ListSessionMessages）
- ✅ 网络错误友好提示 + 失败回滚 + Sign Out

## CI 门禁（全部强制）

1. **SwiftLint** `--strict`（warnings = errors）
2. **SwiftFormat** `--lint`（配置见 `.swiftformat`）
3. **文件大小**：业务 ≤300 行 / 测试 ≤500 行 / 函数 ≤50 行
4. **xcodebuild test**（iPhone 16 Pro Simulator）

## 代码规范

- **SwiftFormat `andOperator` 规则**：`if/guard/while` 条件用逗号 `,` 不用 `&&`
- **`blankLinesBetweenScopes`**：不同 scope（class/struct/func/property）之间必须有空行
- **`redundantThrows` 已禁用**：`@objc` override 的 `throws` 不能删
- **Swift 并发**：`@MainActor` 用于 ViewModel 和 View 相关类型；protocol 不要加 `@MainActor`（会导致默认参数初始化问题）
- **import 不能删**：`import Combine` 是 `ObservableObject` + `@Published` 必须的

## 测试规范

- 使用 **Swift Testing** 框架（`import Testing`，`@Test func`，`#expect`）
- Mock 用 `@MainActor final class`（不要用 `actor`，避免跨 isolation 问题）
- 所有测试 init 中清理 `UserDefaults.standard.removeObject(forKey: "lastSessionID")`
- **测试中必须注入 mock service**，不要让默认参数创建真实的 APIClient（会在测试环境 crash）

## 已知问题 / 待修

- MQG4 PR #13 CI 修复中（mock actor → @MainActor class 改造）
- TM5.2 Token 自动刷新：被阻塞，服务端 RefreshToken 还没实现
- TM4.1 GRDB 本地缓存：待定

## 设计文档

详细架构设计见 `docs/design/`：
- `principles.md` — 架构原则 + 分层规范
- `tech-stack.md` — 技术选型 + 约束
- `foundation.md` — 基础层设计
- `services.md` — 服务层设计
- `business-modules.md` — 业务模块划分
- `app-integration.md` — 应用集成层设计

## 进度追踪

- `ROADMAP.md` — 完整任务列表和状态
- `STATE.md` — 当前 phase 和活跃任务
