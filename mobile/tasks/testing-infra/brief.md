# 测试基础设施建设

## 背景

Spike（PR #2）已验证 Mock gRPC Server 方案可行。现在正式铺设测试基础设施。

### Spike 成果（需要合入本次 PR）

1. `grpc-swift-proto-generator-config.json` 加 `"servers": true` → 生成 `SimpleServiceProtocol`
2. `APIClient.swift` 加 `usePlaintext` 参数（读 `SERVER_PLAINTEXT` 环境变量）
   - 用 `TransportServices`（不是 Posix）做 plaintext，只需改 `transportSecurity` 参数
3. 参考 spike 分支 `feature/spike-mock-grpc-server` 的代码

### ⚠️ Spike 发现的坑

Xcode test target 不能直接添加独立 SPM package dependency（触发 Crypto framework 重复链接）。
**解决方案**：不要在 Package.swift 里加 `GRPCTestSupport` target。Mock 代码直接放测试文件里，通过 host app 继承 `GRPCNIOTransportHTTP2` 的链接。

## 目标

建立完整的 iOS 测试基础设施，覆盖三层：
1. **单元测试**（已有基础，本次补齐）
2. **集成测试**（In-process Mock gRPC Server）
3. **XCUITest**（Mock gRPC Server via launchEnvironment）

## 验收标准

### 集成测试基础设施
- [ ] Mock gRPC Server helper 代码（inline 在测试文件中）
- [ ] MockAuthService（登录成功 + 刷新成功 + 失败场景）
- [ ] MockChatService（发消息 + 收回复）
- [ ] MockSessionService（列会话 + 列消息）
- [ ] 至少 3 个集成测试场景：登录 → 发消息 → 拉历史（验证真实 gRPC 链路）

### XCUITest 基础设施
- [ ] XCUITest setUp 中启动 Mock gRPC Server（plaintext，port 0）
- [ ] 通过 `app.launchEnvironment` 传递 `SERVER_HOST`/`SERVER_PORT`/`SERVER_PLAINTEXT`
- [ ] 至少 2 个 UI 测试场景：登录流程 + 聊天发消息

### 质量
- [ ] 所有现有测试仍然通过
- [ ] SwiftLint + SwiftFormat 通过
- [ ] CI 绿

## 上下文

- Spike 分支：`feature/spike-mock-grpc-server`（参考实现）
- 现有单元测试：`AgentDemoTests/`（14 个，ChatViewModel + AppState + LoadHistory + SessionProtocol）
- 现有 UI 测试：`AgentDemoUITests/`（基本是空壳）
- Proto 定义：`../proto/`（auth.proto, chat.proto, session.proto）
- 3 个 Service Protocol：`ChatServiceProtocol`、`SessionServiceProtocol`、`AuthServiceClient`

## 约束

- 不要在 Package.swift 里加 `GRPCTestSupport` target（原因见上方 Spike 发现的坑）
- 不要改 proto 文件
- Mock 代码直接写在测试文件中（或测试目录下的 helper 文件，不是 SPM target）
- 服务端测试依赖通过 host app 的 linked frameworks 传递
