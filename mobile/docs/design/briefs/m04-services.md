# M04 Brief: 服务层设计

## 任务目标

设计客户端服务层（Service Layer）的模块划分、接口定义和协作关系。服务层是基础层能力到业务层需求之间的桥梁。

## 输出位置

`mobile/docs/design/services.md`（Markdown）

## 成功标准

1. 明确服务层包含哪些服务模块，每个模块的职责和边界
2. 每个服务的对外接口草案（Protocol 级别 + 关键方法签名）
3. 服务间的依赖关系图
4. 与基础层（M03）的协作方式：哪些基础层模块被使用
5. 与服务端 gRPC 接口的对齐说明
6. 错误处理策略：服务层如何包装基础层错误
7. 不确定的地方标注 [待确认]

## 必要上下文

### 服务端 gRPC 接口（来自 server/docs/design/protocols/grpc-services.md）

```
ChatService:
  - SendMessage(SendMessageRequest) → SendMessageResponse  // 发消息
  - Subscribe(SubscribeRequest) → stream AgentEvent         // 订阅事件流
  - SubmitToolResult(SubmitToolResultRequest) → ...         // 提交工具结果

AuthService:
  - Register / Login / RefreshToken / Logout
```

AgentEvent 类型（服务端 Subscribe 推送）：
- TextChunk：文本片段
- ToolRequest：工具调用请求
- TurnComplete：本轮回复结束
- Error：错误事件

### 基础层模块（来自 M03 foundation.md）

- `NetworkCore`：gRPC channel/stream/interceptor
- `StorageCore`：SQLite 连接/事务/迁移
- `SecureStoreCore`：Keychain 安全存储
- `LogCore`：日志
- `ConcurrencyCore`：任务管理/重试
- `PlatformCore`：网络可达性/生命周期

### 产品背景

MVP 核心流程：注册/登录 → 会话列表 → 进入聊天 → 发消息 → 实时收到 AI 回复

### 服务层定义（来自 M01 principles.md）

- 全局通用服务（网络、存储、认证、推送），对上暴露接口
- 接口三端对齐，实现可平台特定
- 不依赖业务层

### 服务层应包含的核心服务（参考方向）

1. **AuthService**：注册、登录、token 管理（存储、刷新、过期处理）
2. **NetworkService / APIClient**：封装 gRPC 调用，处理连接管理、重连
3. **MessageService / ChatService**：消息收发（对齐服务端 ChatService）
4. **StorageService**：本地数据持久化（会话、消息的本地缓存）
5. **ConnectivityService**：网络状态监听，供上层做离线/在线切换

### 关键设计问题

- gRPC Subscribe 是长连接流式，客户端需要管理连接生命周期（断线重连、App 前后台切换）
- AuthService 的 token 刷新需要和 NetworkCore 的 interceptor 配合
- 消息本地缓存 vs 服务端同步的策略

## 参考文档

- 架构原则：`mobile/docs/design/principles.md`
- 技术选型：`mobile/docs/design/tech-stack.md`
- 基础层设计：`mobile/docs/design/foundation.md`
- 服务端 gRPC 协议：`server/docs/design/protocols/grpc-services.md`
- 服务端 Channel 系统：`server/docs/design/channel-system/overview.md`

## 约束

- 不要展开业务模块设计（M05 的事）
- 不要展开应用集成层设计（M06 的事）
- 服务层对上暴露 Protocol，不暴露实现细节
- MVP 先做 iOS，但接口设计考虑三端对齐
- 网络不可靠是设计前提（principles.md 原则 6）
