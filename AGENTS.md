# agent-demo — AI Agent Platform

多租户 AI 智能体平台（SaaS），面向普通用户的情感陪伴和生活助手。

## 项目结构

```
agent-demo/
├── proto/                    # gRPC Proto 定义（4 文件：auth, chat, common, session）
├── server/                   # Rust 服务端（Cargo workspace，8 crate）
│   ├── crates/               # 六边形架构：types → domain → app → channel/llm/storage → server
│   ├── docs/design/          # 架构设计文档
│   ├── crates/agent-e2e/tests/arch.rs  # Rust 架构/文件大小门禁测试
│   └── migrations/           # 在 crates/agent-storage/migrations/
├── mobile/                   # iOS 客户端（Swift + SwiftUI）
│   ├── AgentDemo/            # Xcode 项目
│   └── docs/design/          # 客户端架构设计
└── deploy/                   # 部署配置（Caddyfile + .env）
```

## 核心架构

- **服务端**：六边形架构（Ports & Adapters），Domain 定义 Port trait，适配器实现，依赖只能由外向内
- **客户端**：四层架构（应用集成 → 业务 → 服务 → 基础）
- **通信**：gRPC（tonic/grpc-swift v2），Proto package `ai.agent.platform.v1`
- **LLM**：provider-agnostic，当前用 Kimi K2.5（volcengine OpenAI-compatible API）
- **数据库**：PostgreSQL 16，sqlx

## Git 分支策略

- `feature/*` → `develop`（PR + CI，merge commit，不要 squash）→ `main`（需要人工确认）
- **禁止直接提交 develop 或 main**
- feature 分支创建后立即开 PR（触发 CI）
- PR merge 后删除 feature 分支

## 开发环境

```bash
git config core.hooksPath .githooks   # 启用 pre-push hook（fmt + clippy + test）
```

跳过 hook（紧急情况）：`git push --no-verify`

## 质量标准（不可妥协）

- **CI 红 = 不能 merge**，没有例外
- 新功能必须包含对应测试
- 代码必须符合架构约束（依赖方向只能由外向内）

## Proto 关键约定

- `ContentBlock` 的 oneof 字段叫 `kind`（不是 `block`）
- package: `ai.agent.platform.v1`
- Proto 文件不能随意修改，改动需要同步服务端和客户端

## 部署

- 服务地址见 `deploy/.env`（gitignored）
- Caddy 反代 gRPC（TLS）
- 所有凭证走环境变量，不入仓库

⚠️ **仓库是 public 的** — 禁止写入 IP 地址、密码、API key、内部域名等敏感信息。

## 当前进度

服务端和客户端的详细进度见各自目录的 `STATE.md` 和 `ROADMAP.md`。
