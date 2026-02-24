# ADR-006: gRPC 接口定义（与分层与事件流对齐）

**状态**: Updated（原决策已扩展）
**日期**: 2026-02-24

## 上下文

原 ADR-006 定义了 Auth/Chat/Agent/Session 四类服务与 ChatEvent oneof 流式事件框架，方向正确，但接口语义仍停留在旧 runtime 与消息快照阶段。

在新架构下，gRPC 被明确为 Layer 1 接入层，需要做到：

- 只做协议转换、认证鉴权、错误映射
- 不承载业务编排策略
- 将聊天请求统一映射到 TurnExecutor
- 输出模型与事件流语义一致（含工具调用与系统事件）

因此 ADR-006 需升级为“接入层契约”而非“流程内嵌定义”。

## 决策

1. gRPC 继续作为主要外部协议（继承 ADR-003）。
2. 以 `SendMessage -> TurnExecutor::run_turn` 作为聊天主入口映射。
3. Channel 层职责固定为：
   - DTO 校验
   - JWT 认证与 scope 鉴权
   - DTO ↔ 编排输入输出映射
   - OrchestratorError/AuthError → gRPC Status 统一映射
4. ChatEvent 输出语义与事件流对齐，至少覆盖：
   - 文本增量/完成
   - 工具调用开始/结果
   - 完成/错误
5. handler 禁止直连数据库、LLM SDK 或工具执行实现。
6. 依赖注入在 `agent-server` 统一组装（composition root），`agent-channel` 仅持有 trait/usecase 依赖。

## 理由

- **边界清晰**：防止接入层变成“第二编排层”。
- **多协议扩展友好**：HTTP/WebSocket 可复用同一编排接口。
- **错误一致性**：统一状态码与业务错误码提升客户端可预测性。
- **事件流一致性**：流式事件可表达工具链路与执行状态，而非仅文本。

## 后果

### 正面影响
- 接口与内部架构对齐，职责不混乱。
- 客户端可消费更完整、稳定的事件语义。
- 服务端更易维护与测试（handler 薄、编排厚）。

### 负面影响
- 需要补齐 mapper/error/auth 中间件规范与测试。
- 旧 handler 中混杂逻辑需要迁移出接入层。
- Subscribe 等流式能力在 MVP 可能需先保留占位实现。

## 参考

- `docs/design/infrastructure/grpc-layer.md`
- `docs/design/architecture.md`
- `docs/design/core/event-model.md`
- `docs/design/orchestration/turn-executor.md`
- `docs/design/AUDIT.md`

## 历史背景（保留）

以下内容来自 2026-02-16 的原 ADR-006：

- 定义四个服务：AuthService、ChatService、AgentService、SessionService
- 定义 `SendMessage -> stream ChatEvent` 与 `GetHistory` 等核心接口
- ChatEvent 采用 oneof，包含 text/tool/client-tool/done/error
- 明确 MVP 可先“单条完整回复 + Done”，后续再增强流式

该基线保留为协议演进起点；本次更新将其纳入新分层与事件流语义。