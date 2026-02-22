# agent-demo/server — ROADMAP

> Phase 0 架构设计已完成。进入 Phase 1 Walking Skeleton 实现。
> 详细设计文档见 `docs/design/`

---

## 架构认知基线

- **Channel 是核心抽象**：gRPC 只是一个 Channel Adapter 实现
- **六边形架构**：Domain 定义 Port，适配器实现，依赖只能由外向内
- **8 crate workspace**：agent-types → agent-domain → agent-app → [...] → agent-server

---

## Phase 1：Walking Skeleton

### Step 0：工程脚手架

| # | Task | 状态 | 内容 |
|---|------|------|------|
| T0.1 | Cargo workspace 初始化 | ✅ | 创建 8 个 crate，DAG 依赖，cargo check/test 通过 |
| T0.2 | GitHub Actions CI | ✅ | Linux runner，cargo check + test，path filter，push/PR 触发 |

### Step 1：Echo 闭环

| # | Task | 状态 | 内容 |
|---|------|------|------|
| T1.1 | Proto 编译 | ✅ | tonic-build，4 proto → Rust 代码生成 |
| T1.2 | 最小 gRPC 服务 | ✅ | ChatService.SendMessage echo，[::1]:50051 |
| T1.3 | grpcurl 端到端验证 | ✅ | Echo 通过，Step 1 完成 |

### Step 2：真实认证

| # | Task | 状态 | 内容 |
|---|------|------|------|
| T2.1 | JWT 认证 | ✅ | AuthPort → AuthService → AuthHandler，六边形四层贯通 |
| T2.2 | Auth 拦截器 | ✅ | tonic interceptor，ChatService 受保护 |
| T2.3 | grpcurl 验证 | ✅ | 完整认证链路验证，Step 2 完成 |

### Step 3：真实 AI 回复 + 部署

| # | Task | 状态 | 内容 |
|---|------|------|------|
| T3.1 | LlmProvider Port + 适配器 | ✅ | agent-domain LlmProvider trait + agent-llm OpenAI 兼容适配器 |
| T3.2 | Agent Runtime 最小路径 | ✅ | SendMessage → AgentRuntime → LLM(Kimi K2.5) → 整块返回 AI 回复 |
| T3.3 | Caddy + TLS 部署 | ✅ | REDACTED_HOST:8443，Caddy 反代 gRPC，Let's Encrypt DNS-01 |
| T3.4 | 公网 e2e 验证 | ✅ | grpcurl 通过公网 TLS：登录 + 发消息 + AI 回复 |

---

## Quality Gate：质量保障体系

> 独立于功能 Step，按顺序逐步推进。每项完成后即在 CI 中强制执行，不可绕过。

### 代码质量（自动门禁）

| # | Task | 状态 | 内容 |
|---|------|------|------|
| QG1 | fmt + clippy | ✅ | 修复现有问题 + CI 加 `cargo fmt --check` + `cargo clippy -D warnings` |
| QG2 | 架构依赖检查 | ✅ | 脚本自动验证 crate 依赖方向 + CI 硬门禁 |
| QG3 | 文件大小 & 复杂度 | ✅ | 单文件 ≤300 行，单函数 ≤50 行；脚本检查 + CI 硬门禁 |

### 功能质量（测试保障）

| # | Task | 状态 | 内容 |
|---|------|------|------|
| QG4 | 补齐现有测试缺口 | ✅ | 逐模块盘点 + 补充单测（13→25 个） |
| QG5 | 覆盖率工具 + CI 阈值 | 🔲 | cargo-tarpaulin/llvm-cov + CI 报告 + 最低阈值（棘轮：只升不降） |

### 长期防劣化

| # | Task | 状态 | 内容 |
|---|------|------|------|
| QG6 | 集成测试框架 | 🔲 | CI service container (PG) + #[ignore] 测试在 CI 中跑 |
| QG7 | 验收测试脚本化 | 🔲 | 关键业务流程 grpcurl 脚本，部署后自动验证 |

### 质量铁律（QG 全部就位后强制执行）

- CI 红 = 不能 merge，没有例外
- 新功能 PR 必须包含对应测试
- 覆盖率只升不降（棘轮机制）
- review sub-agent 对照验收标准逐项检查

---

## Phase 1（续）：功能开发

> QG1-3 完成后恢复功能开发，后续功能开发与 QG4-7 交替推进。

### Step 4：持久化

| # | Task | 状态 | 内容 |
|---|------|------|------|
| T4.1 | PostgreSQL 接入 | ✅ | PR #1，sqlx + bcrypt + users 表 + migration |
| T4.2 | 真实用户注册 | ✅ | PR #2，Register 端点 + AuthPort.create_user |
| T4.3 | 消息持久化 | ✅ | PR #3，sessions + messages 表，历史 context 送 LLM |

### Step 5：韧性

| # | Task | 状态 | 内容 |
|---|------|------|------|
| T5.1 | 错误处理 | 🔲 | 统一错误类型，gRPC status 映射 |
| T5.2 | LLM 重试/降级 | 🔲 | provider 不可用时的 fallback 策略 |

### Step 6：会话管理

| # | Task | 状态 | 内容 |
|---|------|------|------|
| T6.1 | SessionService | 🔲 | 会话 CRUD（创建、列表、归档） |
| T6.2 | 多会话隔离 | 🔲 | 不同会话独立上下文 |

---

## Phase 0：架构设计（已完成）

<details>
<summary>展开查看</summary>

### 轨道 A：可行性验证（Spikes）

| # | 任务 | 状态 | 位置 |
|---|------|------|------|
| S01 | async trait：dyn vs 泛型 | ✅ | `spikes/s01-async-trait/` |
| S02 | ChannelAdapter dyn 兼容 | ✅ | `spikes/s02-channel-adapter-dyn/` |
| S03 | Tokio actor/mailbox | ✅ | `spikes/s03-tokio-actor/` |

### 轨道 B：深化设计（Design Docs）

| # | 任务 | 状态 | 位置 |
|---|------|------|------|
| D01 | 架构原则文档更新 | ✅ | `docs/design/principles.md` |
| D02 | Crate 划分设计 | ✅ | `docs/design/crate-structure.md` |
| D03 | Channel Port trait 设计 | ✅ | `docs/design/channel-system/port.md` |
| D04 | Agent Runtime 树形拆分 | ✅ | `docs/design/core/agent-runtime-detail.md` |
| D05 | 核心 Port trait 精确定义 | ✅ | `docs/design/ports/` |
| D06 | 错误类型体系设计 | ✅ | `docs/design/error-types.md` |

</details>

---

## 未来建设（待讨论）

| # | Task | 状态 | 内容 |
|---|------|------|------|
| OBS-1 | 日志与观测体系 | 🔲 | 结构化日志、tracing、错误追踪。待涂涂讨论后细化 |

---

*最后更新：2026-02-22*
