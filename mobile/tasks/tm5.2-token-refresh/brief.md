# TM5.2: Token 自动刷新

## 前置清理（来自 spike PR #2）

Spike 验证了 mock gRPC server 方案可行，但 PR 未合入。本任务先完成以下清理：

1. **`grpc-swift-proto-generator-config.json`**：加 `"servers": true`（生成 `SimpleServiceProtocol`，mock 需要）
2. **`APIClient.swift`**：加 `usePlaintext` 参数（读 `SERVER_PLAINTEXT` 环境变量，默认 false）
   - 用 `TransportServices`（不是 Posix）做 plaintext，只需改 `transportSecurity` 参数
3. **`Package.swift`**：不要添加 `GRPCTestSupport` target（Xcode test target 链接 SPM 依赖会触发 Crypto framework 重复链接问题）
4. **Mock 代码放测试文件里**：直接 inline 在 test file 中，通过 host app 继承 `GRPCNIOTransportHTTP2` 的链接

参考 spike 分支 `feature/spike-mock-grpc-server` 的代码，但不要原样搬——需要按上述要求调整。

## 目标

Access token 过期时自动用 refresh token 换取新 token pair，用户无感知。

## 验收标准

- [ ] Proto config 加 `servers: true`
- [ ] APIClient 支持 plaintext 连接（`usePlaintext` 参数）
- [ ] Token 刷新逻辑实现（调用服务端 RefreshToken RPC）
- [ ] gRPC 拦截器或中间件：收到 UNAUTHENTICATED 错误时自动触发刷新
- [ ] 刷新成功 → 重试原始请求
- [ ] 刷新失败 → 跳转登录页
- [ ] Keychain 中的 token 同步更新
- [ ] 并发请求场景：多个请求同时触发刷新时只刷一次（避免竞态）
- [ ] 单元测试覆盖：刷新成功、刷新失败、并发刷新
- [ ] Mock gRPC server 集成测试（inline mock，验证真实 gRPC 链路）
- [ ] SwiftLint + SwiftFormat 通过

## 上下文

- 服务端已支持 RefreshToken RPC（PR #26 merged）
- Proto 定义：`../proto/auth.proto`（RefreshTokenRequest/Response）
- 现有 Auth 相关：`Sources/GRPCClient/AuthServiceClient.swift`
- Token 存储：`AgentDemo/Utilities/KeychainHelper.swift`
- 登录后存的是 access_token + refresh_token pair
- Spike 分支 `feature/spike-mock-grpc-server` 有参考实现（MockAuthService、MockGRPCServer、inline test）

## 约束

- 不要改 proto 文件
- 不要在 Package.swift 里加 `GRPCTestSupport` target（原因见前置清理第 3 点）
- refresh token 本身过期时必须回到登录页，不能死循环刷新
- 服务端测试依赖只通过 host app 的 linked frameworks 传递
