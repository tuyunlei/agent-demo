# TM5.2: Token 自动刷新

## 目标

Access token 过期时自动用 refresh token 换取新 token pair，用户无感知。

## 验收标准

- [ ] Token 刷新逻辑实现（调用服务端 RefreshToken RPC）
- [ ] gRPC 拦截器或中间件：收到 UNAUTHENTICATED 错误时自动触发刷新
- [ ] 刷新成功 → 重试原始请求
- [ ] 刷新失败 → 跳转登录页
- [ ] Keychain 中的 token 同步更新
- [ ] 并发请求场景：多个请求同时触发刷新时只刷一次（避免竞态）
- [ ] 单元测试覆盖：刷新成功、刷新失败、并发刷新
- [ ] SwiftLint + SwiftFormat 通过

## 上下文

- 服务端已支持 RefreshToken RPC（PR #26 merged）
- Proto 定义：`../proto/auth.proto`（RefreshTokenRequest/Response）
- 现有 Auth 相关：`Sources/GRPCClient/AuthServiceClient.swift`
- Token 存储：`AgentDemo/Utilities/KeychainHelper.swift`
- 登录后存的是 access_token + refresh_token pair

## 约束

- 不要改 proto 文件
- refresh token 本身过期时必须回到登录页，不能死循环刷新
