# Brief: D05 — 核心 Port Trait 精确定义

## 任务目标

为 Agent Runtime 依赖的 5 个核心 Port 写出精确的 Rust trait 设计文档：
方法签名、参数类型、返回类型、错误类型。这些 Port 定义在 `agent-domain` crate，
是核心层的稳定接口契约，适配器层实现它们。

## 输出位置

新建：`server/docs/design/ports/core-ports.md`

## 成功标准

以下 5 个 Port trait 都有精确定义（Rust 伪代码）：

1. `LlmProvider` — LLM 调用，返回流式 token
2. `MessageStore` — 消息历史 CRUD
3. `SessionStore` — 会话状态读写
4. `MemoryStore` — 记忆摘要读写
5. `PersonaStore` — 人格配置读取

每个 Port 包含：
- trait 方法签名（async，参数类型，返回类型）
- 返回错误类型（用统一的 `DomainError` 或各自错误类型）
- 一句话说明每个方法的语义

## 必要上下文

**哪些子模块依赖哪些 Port（来自 agent-runtime-detail.md）**：
- `ContextAssembler` → MessageStore、PersonaStore、MemoryStore
- `LlmCaller` → LlmProvider
- `ToolCoordinator` → ToolRuntime（本文不包含，单独处理）
- `PostProcessor` → MessageStore、MemoryStore（可能 SessionStore）
- `EventPublisher` → ChannelAdapter（已在 D03 定义）

**LlmProvider 关键设计要求**：
- 返回流式 token 流（`DynTokenStream` 或等价 boxed stream）
- 每个 token 可能是 TextDelta、ToolCallStart、ToolCallDelta、ToolCallEnd
- 支持请求取消（stream drop 即取消）
- 包含 usage 信息（在流结束时或单独方法）

**MessageStore 关键设计要求**：
- 所有方法都带 `user_id`（租户隔离，不能跨租户）
- 分页读取会话历史（MessagePage，含 cursor）
- 写入单条消息、批量写入
- 软删除（用 is_deleted 标记）

**SessionStore 关键设计要求**：
- get/create/update 操作
- 带 user_id 租户隔离
- session 有状态机（Idle/Active/Compacting/Archived）
- CAS（比较并交换）更新防止并发写冲突

**MemoryStore 关键设计要求**：
- 读取记忆摘要（按 session 或 agent）
- 写入/更新摘要（PostProcessor 在 turn 完成后触发）
- 未来预留向量检索接口（MVP 可以是空实现）

**PersonaStore 关键设计要求**：
- 只读（不修改人格）
- 按 agent_id 读取人格配置（名字、系统 prompt、风格参数等）
- 可能有多个版本（active 版本为准）

**S01 结论（适用于所有 Port）**：
- 所有 Port 使用 `#[async_trait]`
- 持有方式：`Arc<dyn PortTrait + Send + Sync>`
- 错误类型建议统一为 `DomainError`（domain 层定义）

**参考文件**：
- `docs/design/core/agent-runtime-detail.md`（Port 使用场景）
- `docs/design/core/data-model.md`（数据模型定义）

## 约束

- 不定义 ToolRuntime Port（工具系统复杂，单独文档）
- 不定义 ChannelAdapter Port（已在 D03 完成）
- 错误类型暂时统一用 `DomainError`，具体细化留给 D06
- 每个 Port 方法不超过 6-8 个（保持接口隔离）
- Rust 伪代码，不需要通过 cargo check
