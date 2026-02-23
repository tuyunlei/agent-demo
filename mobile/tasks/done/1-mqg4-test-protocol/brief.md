# MQG4: 测试补全 + Protocol 化

## 目标

让 SessionServiceClient 可测试（抽 Protocol），补全现有测试覆盖。

## 验收标准

- [ ] `SessionServiceProtocol` 定义（参照已有的 `ChatServiceProtocol`）
- [ ] `SessionServiceClient` 实现该 Protocol
- [ ] Mock 实现用于测试
- [ ] `ChatViewModel` 中 `loadHistory` 相关逻辑有测试覆盖（至少 3 个 case）
- [ ] `AppState` 基础测试（至少 2 个 case）
- [ ] 所有现有测试仍然通过
- [ ] SwiftLint + SwiftFormat 通过

## 上下文

- 参考已有模式：`Sources/GRPCClient/ChatServiceProtocol.swift` + `ChatServiceClient.swift`
- 现有测试：`AgentDemoTests/ChatViewModelTests.swift`（5 个测试）
- `SessionServiceClient` 在：`Sources/GRPCClient/SessionServiceClient.swift`
- `AppState` 在：`AgentDemo/AppState.swift`

## 约束

- 不要改 ChatServiceProtocol 的现有接口
- 不要改 proto 文件
- 测试用 Swift Testing 框架（`@Test`），不用 XCTest
