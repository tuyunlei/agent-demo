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
| T0.1 | Cargo workspace 初始化 | ✅ | 创建 8 个 crate（空壳），`Cargo.toml` workspace 配置，`cargo check --workspace` 通过 |
| T0.2 | GitHub Actions CI | ⏸ blocked | Linux runner，`cargo check + cargo test`，push/PR 触发（需要先建 GitHub repo） |

### Step 1：Echo 闭环

| # | Task | 状态 | 内容 |
|---|------|------|------|
| T1.1 | Proto 编译 | ✅ | tonic-build 配置，.proto → Rust 代码生成，放入 agent-proto crate |
| T1.2 | 最小 gRPC 服务 | ✅ | ChatService.SendMessage echo 回传，agent-server 监听 [::1]:50051 |
| T1.3 | grpcurl 端到端验证 | ✅ | SendMessage echo 通过，Subscribe 返回 Unimplemented |

### Step 2：真实认证

| # | Task | 状态 | 内容 |
|---|------|------|------|
| T2.1 | JWT 认证 | 🔲 | AuthService.Login（硬编码用户，不接 DB），返回 JWT TokenPair |
| T2.2 | Auth 拦截器 | 🔲 | tonic interceptor 验证 JWT，ChatService 受保护 |
| T2.3 | grpcurl 验证 | 🔲 | login → 拿 token → 带 token 发消息，记录验证命令 |

### Step 3：真实 AI 回复

| # | Task | 状态 | 内容 |
|---|------|------|------|
| T3.1 | LlmProvider Port + 适配器 | 🔲 | agent-domain 定义 LlmProvider trait，agent-llm 实现一个 provider（OpenAI 兼容 API） |
| T3.2 | Agent Runtime 最小路径 | 🔲 | SendMessage → 构建上下文 → 调 LLM → 整块返回 AI 回复（unary，不做流式） |
| T3.3 | grpcurl 验证 | 🔲 | 发消息 → 收到 AI 生成的回复 |

### Step 4：持久化

| # | Task | 状态 | 内容 |
|---|------|------|------|
| T4.1 | PostgreSQL 接入 | 🔲 | agent-storage crate，sqlx，users + sessions + messages 表 |
| T4.2 | 真实用户注册/登录 | 🔲 | AuthService.Register，密码哈希，DB 存储 |
| T4.3 | 消息持久化 | 🔲 | 聊天记录入库，历史消息查询 |

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

*最后更新：2026-02-20*
