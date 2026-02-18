# Brief: D06 — 错误类型体系设计

## 任务目标

设计整个系统的错误类型体系：各层的错误类型定义、跨层传播规则、
`thiserror` 的使用方式，以及适配器错误如何转换到领域/应用错误。

## 输出位置

新建：`server/docs/design/error-types.md`

## 成功标准

1. 定义三层错误类型（Domain / Application / Infrastructure），并给出 Rust 伪代码
2. 说明跨层转换规则（Infrastructure → Application → 对外响应）
3. 说明 `thiserror` vs `anyhow` 的选型策略（什么场景用哪个）
4. 给出 `DomainError` 的主要变体（不超过 8 个，覆盖 MVP 场景）
5. 说明如何把 `sqlx::Error`、`tonic::Status` 等转换为内部错误，且不泄漏到领域层

## 必要上下文

**分层架构（来自 crate-structure.md）**：
```
agent-types    跨层共享的纯数据类型，可以包含基础错误
agent-domain   领域层，定义 DomainError
agent-app      应用层，定义 AppError，封装 DomainError
Infrastructure 各自的错误类型，转换为 AppError 再往上传
```

**典型错误场景（需要覆盖）**：
- 用户不存在 / session 不存在 / agent 不存在
- LLM 调用失败（provider 错误、超时、限流）
- 存储读写失败（连接失败、约束违反）
- 工具执行失败（超时、工具返回错误）
- 认证失败（token 无效、权限不足）
- 幂等冲突（request_id 重复）
- 系统内部错误（预期外的错误）

**约束条件**：
- `sqlx::Error`、`tonic::Status`、`reqwest::Error` 等框架错误绝不能出现在领域层
- 适配器层做错误映射，转换为 `DomainError` 或 `AppError`
- 错误应该携带足够的上下文信息（request_id、user_id 等）用于日志追踪
- 面向用户的错误响应需要安全（不泄漏内部实现细节）

**参考**：
- `docs/design/principles.md`（4.2 错误处理分层）

## 约束

- 不需要定义所有可能的错误变体，只覆盖 MVP 主路径
- 不写实际 Rust 代码实现，只写设计文档
- 保持简洁，避免过度细化（MVP 阶段 DomainError 10 个变体以内）
