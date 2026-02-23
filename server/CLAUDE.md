# server — Rust 服务端

## 技术栈

- Rust（stable）/ Tokio / Tonic（gRPC）
- PostgreSQL 16 + sqlx（compile-time checked queries）
- 六边形架构（Ports & Adapters）

## Crate 结构（严格 DAG）

```
agent-types      ← 共享类型（error, config）
agent-proto      ← tonic-build 生成的 gRPC 代码
agent-domain     ← 核心领域（Port trait 定义，业务逻辑）
agent-app        ← 应用服务（编排层）
agent-channel    ← gRPC 适配器（AuthHandler, ChatHandler, SessionHandler）
agent-llm        ← LLM 适配器（OpenAI 兼容）
agent-storage    ← PostgreSQL 适配器（sqlx）
agent-server     ← 组装入口（main.rs）
```

**依赖方向**：只能由外向内。`agent-channel` 可以依赖 `agent-domain`，反过来不行。

## 架构约束（不可妥协）

- Domain 定义 Port trait，适配器实现；依赖只能由外向内
- `async trait`：`dyn Trait + async-trait`，`Arc<dyn Port + Send + Sync>`
- 显式错误处理，显式依赖注入
- 单文件 ≤300 行（业务）/ ≤500 行（测试），单函数 ≤50 行

## CI 门禁（全部强制）

1. `cargo fmt --all -- --check`
2. `cargo clippy --workspace -- -D warnings`
3. `cargo test --workspace --test arch`（`crates/agent-e2e/tests/arch.rs`：架构依赖方向 + 文件大小检查）
4. `cargo check --workspace`
5. `cargo test --workspace`（需要 DATABASE_URL）
6. cargo-tarpaulin 覆盖率 ≥54%（棘轮，只升不降）
7. 集成测试：`sqlx::test` + CI PostgreSQL service container

## 关键设计决策

- **Port trait 注入**：`Arc<dyn PortTrait + Send + Sync>` 构造注入
- **消息排序**：用 `sequence_num BIGSERIAL`（不用 `created_at`，秒级精度不够）
- **JWT**：access_token + refresh_token（refresh 暂未实现）
- **LLM**：OpenAI-compatible API，当前接 Kimi K2.5
- **Migration 路径**：`crates/agent-storage/migrations/`，sqlx::test 用 `"./migrations"` 相对路径

## 数据库 Schema

```sql
-- users
id UUID PRIMARY KEY, email TEXT UNIQUE, password_hash TEXT, display_name TEXT, created_at/updated_at TIMESTAMPTZ

-- sessions
id UUID PRIMARY KEY, user_id UUID FK, agent_id TEXT, title TEXT, summary TEXT, created_at/updated_at/last_message_at TIMESTAMPTZ, archived BOOLEAN

-- messages
id UUID PRIMARY KEY, session_id UUID FK, role TEXT, content TEXT, created_at TIMESTAMPTZ, sequence_num BIGSERIAL
```

## 测试

- 38 个测试（33 单元 + 5 集成），覆盖率 54%
- 集成测试用 `#[sqlx::test]`，每个测试独立临时数据库
- e2e 测试：`crates/agent-e2e`（Rust 原生 e2e，6 个核心场景，已替代旧 grpcurl shell 验收脚本）

## 本地开发

```bash
# 环境变量
export PATH="$HOME/.cargo/bin:$PATH"
export DATABASE_URL="postgres://user:pass@127.0.0.1:5432/agentdemo"  # 实际凭证见 deploy/.env

# 编译运行
cd server && cargo build && cargo run

# 测试
cargo test --workspace

# 质量检查
cargo fmt --all -- --check
cargo clippy --workspace -- -D warnings
cargo test --workspace --test arch
```

## 设计文档

详细架构设计见 `docs/design/`：
- `principles.md` — 架构原则
- `crate-structure.md` — Crate 划分设计
- `ports/` — 核心 Port trait 精确定义
- `core/agent-runtime-detail.md` — Agent Runtime 设计
- `channel-system/port.md` — Channel Port trait
- `error-types.md` — 错误类型体系

## 进度追踪

- `ROADMAP.md` — 完整任务列表和状态
- `STATE.md` — 当前 phase 和活跃任务
