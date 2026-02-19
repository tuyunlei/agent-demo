# 错误类型体系设计（D06）

## 1. 目标与边界

本设计定义 MVP 阶段的三层错误体系（Domain / Application / Infrastructure），确保：

1. 错误按层归属清晰，依赖方向不反转（Hexagonal）
2. 外部框架错误不泄漏到核心层
3. 错误可追踪（携带 request_id/user_id/session_id 等上下文）
4. 对外响应安全（不暴露内部实现细节）

---

## 2. 分层错误模型

## 2.1 Domain 层：`DomainError`

**职责**：表达业务语义上的失败原因；不包含框架类型（`sqlx::Error`/`tonic::Status`/`reqwest::Error` 等）。

```rust
// agent-domain
#[derive(Debug, thiserror::Error)]
pub enum DomainError {
    #[error("user not found: {user_id}")]
    UserNotFound { user_id: String },

    #[error("session not found: {session_id}")]
    SessionNotFound { session_id: String },

    #[error("agent not found: {agent_id}")]
    AgentNotFound { agent_id: String },

    #[error("authentication failed")]
    AuthFailed,

    #[error("permission denied")]
    PermissionDenied,

    #[error("idempotency conflict: request_id={request_id}")]
    IdempotencyConflict { request_id: String },

    #[error("tool execution failed: tool={tool}, reason={reason}")]
    ToolExecutionFailed { tool: String, reason: String },

    #[error("external dependency failed: {service}")]
    ExternalDependencyFailed { service: String },
}
```

> 说明：`ExternalDependencyFailed` 用于承接 LLM/第三方能力在领域视角下的“依赖失败”，避免把 provider-specific 细节引入领域。

---

## 2.2 Application 层：`AppError`

**职责**：编排流程中的统一错误出口，封装 `DomainError`，并承接基础设施/系统级异常。

```rust
// agent-app
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error(transparent)]
    Domain(#[from] DomainError),

    #[error("infrastructure error: kind={kind}, message={message}")]
    Infrastructure {
        kind: InfraErrorKind,
        message: String,
        // 给日志与 tracing 用，不直接对外透出
        context: ErrorContext,
    },

    #[error("request timeout")]
    Timeout { context: ErrorContext },

    #[error("internal error")]
    Internal {
        message: String,
        context: ErrorContext,
    },
}

#[derive(Debug, Clone)]
pub enum InfraErrorKind {
    Storage,
    LlmProvider,
    Network,
    Serialization,
    Unknown,
}

#[derive(Debug, Clone, Default)]
pub struct ErrorContext {
    pub request_id: Option<String>,
    pub user_id: Option<String>,
    pub session_id: Option<String>,
    pub agent_id: Option<String>,
}
```

---

## 2.3 Infrastructure 层：适配器私有错误

**职责**：描述实现细节错误；在适配器边界转换成 `AppError`（必要时先映射成 `DomainError`）。

```rust
// agent-storage (example)
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("db connection failed")]
    Connection,
    #[error("constraint violation: {0}")]
    Constraint(String),
    #[error("record not found")]
    NotFound,
    #[error(transparent)]
    Unexpected(#[from] sqlx::Error),
}

// agent-llm (example)
#[derive(Debug, thiserror::Error)]
pub enum LlmError {
    #[error("provider rate limited")]
    RateLimited,
    #[error("provider timeout")]
    Timeout,
    #[error("provider unauthorized")]
    Unauthorized,
    #[error(transparent)]
    Transport(#[from] reqwest::Error),
}
```

> 这些类型只存在于各自 infra crate 内，不向 domain 暴露。

---

## 3. 跨层转换规则

## 3.1 总规则

1. **Domain 只能产出 `DomainError`**
2. **Application 统一对外（对 adapter）产出 `AppError`**
3. **Infrastructure 先本地归一化，再映射到 `AppError` / `DomainError`**
4. **协议层（gRPC/HTTP）只做 `AppError -> transport status`**

## 3.2 映射路径

```text
sqlx::Error / reqwest::Error / tonic::Status
            ↓ (adapter内部)
      StorageError / LlmError / ChannelError
            ↓ (map_err at adapter boundary)
      AppError::Infrastructure / AppError::Timeout
            ↓ (use case 可按语义再细化)
           AppError::Domain(DomainError::...)
            ↓
       gRPC/HTTP 安全错误响应
```

## 3.3 典型映射示例

### A) `sqlx::Error`（存储）

- `RowNotFound` → `DomainError::UserNotFound` / `SessionNotFound`（根据仓储语义）
- 唯一键冲突（如 request_id）→ `DomainError::IdempotencyConflict`
- 连接失败/池耗尽/协议错误 → `AppError::Infrastructure { kind: Storage, ... }`

### B) `reqwest::Error`（LLM HTTP）

- 超时 → `AppError::Timeout`
- 429/限流 → `AppError::Infrastructure { kind: LlmProvider, ... }`
- 401/403（provider 鉴权）→ `DomainError::ExternalDependencyFailed { service: "llm" }` 或 `AppError::Infrastructure`（根据是否进入领域决策）

### C) `tonic::Status`（仅通道层）

- 来自上游 channel 的未认证/无权限可映射到 `DomainError::AuthFailed` / `PermissionDenied`
- 其余传输错误（unavailable/internal/deadline_exceeded）映射 `AppError::Infrastructure/Timeout`
- **禁止**把 `tonic::Status` 传入 domain/app 核心逻辑

---

## 4. 对外错误响应策略（安全）

协议适配层维护稳定映射表：

- `DomainError::UserNotFound | SessionNotFound | AgentNotFound` → `NOT_FOUND`
- `AuthFailed` → `UNAUTHENTICATED`
- `PermissionDenied` → `PERMISSION_DENIED`
- `IdempotencyConflict` → `ALREADY_EXISTS`（或 HTTP 409）
- `ToolExecutionFailed | ExternalDependencyFailed` → `FAILED_PRECONDITION` / `UNAVAILABLE`
- `AppError::Timeout` → `DEADLINE_EXCEEDED` / HTTP 504
- `AppError::Infrastructure | Internal` → `INTERNAL`

返回给用户的 message 必须是通用、安全文案；详细错误（sql、provider body、stack）只进日志。

---

## 5. `thiserror` vs `anyhow` 策略

## 5.1 `thiserror`（默认）

用于**可预期、可分类、跨边界传播**的错误：

- DomainError / AppError / 各 adapter 的 typed error
- 需要稳定匹配（`match`）和协议映射的场景
- 需要明确 From 转换链的场景

## 5.2 `anyhow`（受限使用）

仅用于**应用入口或一次性胶水代码**：

- binary `main` 启动/装配阶段
- 临时脚本、测试辅助路径
- 不再继续跨层传播、只用于最终打印/记录的错误

**禁止**在 domain 公开 API、port trait、use case 输出中使用 `anyhow::Error`。

---

## 6. MVP 实施约束与检查清单

- [ ] `DomainError` 变体 <= 8（当前 8 个）
- [ ] domain/app crate 不依赖 sqlx/tonic/reqwest
- [ ] 所有适配器在边界显式 `map_err`
- [ ] 错误日志统一包含 `ErrorContext`（至少 request_id）
- [ ] 对外响应不泄漏内部细节

---

## 7. 推荐落地顺序

1. 在 `agent-domain` 定义 `DomainError`
2. 在 `agent-app` 定义 `AppError + ErrorContext + InfraErrorKind`
3. 各 infra crate 定义本地 typed error 并实现映射
4. 在 `agent-server` 集中维护 `AppError -> tonic::Status` 映射
5. 增加集成测试：验证关键错误路径不会泄漏框架错误类型
