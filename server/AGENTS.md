# server — Rust 服务端

## 技术栈

- Rust 2024 edition（stable）/ Tokio / Tonic（gRPC）
- PostgreSQL 16 + sqlx（compile-time checked queries）
- 四层架构（Channel → Orchestration → Capability → Infrastructure）

## Crate 结构（四层 13 crate，严格 DAG）

```
Layer 1 (Channel):        agent-channel, agent-server
Layer 2 (Orchestration):  agent-orchestrator
Layer 3 (Capability):     agent-llm, agent-tools, agent-context, agent-memory
Layer 4 (Infrastructure): agent-storage
Cross-cutting:            agent-domain (所有层可依赖), agent-proto (L1 可依赖)
Testing:                  agent-e2e
```

**依赖方向**：只能上层依赖下层，不可反向。`arch.rs` 自动检测违规。

## 架构约束（不可妥协）

- Domain 定义 Port trait，适配器实现；依赖只能由上向下
- Trait 按域分布：每个 capability crate 定义自己的 trait
- `async trait`：`dyn Trait + async-trait`，`Arc<dyn Trait + Send + Sync>`
- 显式错误处理，显式依赖注入
- 单文件 ≤300 行（业务）/ ≤500 行（测试），单函数 ≤50 行
- agent-server 是 Composition Root：4 层 DI 组装

## CI 门禁（全部强制，红 = 不能 merge）

1. `cargo fmt --all -- --check`
2. `cargo clippy --workspace --exclude agent-e2e -- -D warnings`
3. `cargo test --workspace --test arch`（架构依赖方向 + 文件大小检查）
4. `cargo check --workspace`
5. `cargo test --workspace`（需要 DATABASE_URL，CI 用 PostgreSQL service container）
6. cargo-tarpaulin 覆盖率门槛（见下方测试策略）
7. 集成测试：`sqlx::test` + CI PostgreSQL service container

## 测试策略（不可妥协）

### 核心原则

- **所有预期内的生产功能 feature 都必须有测试**，没有例外
- 边界 case 优先级可以低一些，但核心链路 feature 全部要测到
- 新功能必须有对应测试
- 单元测试 + 集成测试（sqlx::test）共同计算覆盖率

### 覆盖率目标

- **长期目标：90%**（棘轮，只升不降）
- 当前门槛：见 `.github/workflows/server-ci.yml` 中 THRESHOLD 值

### 覆盖率白名单

确实无法单测的文件可以排除在覆盖率计算之外。**白名单变更需要涂涂审批。**

当前已审批白名单：

| 文件 | 行数 | 理由 | 审批时间 |
|------|------|------|---------|
| `agent-server/src/lib.rs` | 92 | Composition Root：DI 组装 + DB migration + admin seed，纯启动胶水代码 | 2026-02-25 |
| `*/src/main.rs` | ~8 | 入口文件 | 2026-02-25 |
| `*/build.rs` | - | 构建脚本 | 2026-02-25 |
| `*.pb.rs` | - | protobuf 生成代码 | 2026-02-25 |

**不允许加白名单的**：有业务逻辑的文件（provider、handler、store 等）。这些必须通过 mock/stub 方式测试。

### 测试分层

- **单元测试**：每个 crate 内 `*_tests.rs`，测试纯逻辑
- **集成测试**：`agent-storage` 内 `sqlx::test`，测试 DB 交互（本地 + CI 都要跑）
- **e2e 测试**：`agent-e2e`，测试完整 gRPC 链路（需要 DATABASE_URL）
- **架构测试**：`agent-e2e/tests/arch.rs`，自动检测依赖方向违规 + 文件大小

## 关键设计决策

- **ADR-001**: 单二进制 + 模块（trait 边界支持未来拆分）
- **ADR-002**: PostgreSQL only（结构化 + JSONB + 未来 pgvector）
- **ADR-003**: gRPC（tonic），在 Application Service 层抽象
- **ADR-004~008**: 数据模型、Agent Runtime、gRPC 接口、事件流、PromptSection
- **事件流模型**: append-only EventStore，Session State = Fold(Event Stream)
- **系统提示词**: session 创建时构建一次，ConfigChange 事件驱动更新
- **LLM Provider**: 支持 fallback chain，当前接 OpenAI-compatible API

## 数据库 Schema

```sql
-- users: id, email, password_hash, display_name, created_at, updated_at
-- sessions: id, user_id, tenant_id, agent_id, title, summary, status, version,
--           event_count, last_sequence, estimated_prompt_tokens,
--           compacted_until_sequence, last_message_at, archived, created_at, updated_at
-- messages: id, session_id, role, content, created_at, sequence_num
-- events: event_id, session_id, sequence_number, event_type, payload(JSONB),
--         tenant_id, user_id, created_at
```

## 本地开发

```bash
export PATH="$HOME/.cargo/bin:$PATH"
export DATABASE_URL="postgres://agentdemo:<password>@127.0.0.1:5432/agentdemo"  # 见 deploy/.env

# 编译 + 测试（含 storage 集成测试）
cargo build
cargo test --workspace

# 质量检查
cargo fmt --all -- --check
cargo clippy --workspace --exclude agent-e2e -- -D warnings
cargo test --workspace --test arch
```

## Merge 策略

- **Merge commit**，不用 squash
- feature/* → develop → main（develop→main 需涂涂确认）
- 创建 feature 分支后立即开 PR（触发 CI）
- merge 后删分支

## 设计文档

`docs/design/` 下 11 个设计文档（~9500 行），按层组织：
- `architecture.md` — 整体分层架构
- `core/event-model.md` — 事件流数据模型
- `capabilities/` — llm-provider, tool-system, context-builder, session-lifecycle
- `orchestration/turn-executor.md` — Turn 编排
- `infrastructure/` — grpc-layer, postgres-adapter
- `decisions/` — ADR 001-008
- `docs/research/` — 框架对比调研

## 进度追踪

- `ROADMAP.md` — 完整任务列表和状态
- `STATE.md` — 当前 phase 和活跃任务
