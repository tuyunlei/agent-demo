# server/ — 架构约束

本文件是给所有在 `server/` 目录下工作的开发者（包括 sub-agent）的架构指南。
改代码前先读这个文件。子目录下如果有自己的 AGENTS.md，也要读。

---

## 项目概述

多租户 AI Agent 平台后端。Rust 单体，trait 边界保证未来可拆分。

## 四层架构

```
┌─ Channel（接入层）─────────────────────────────────┐
│  协议转换 · 认证鉴权 · DTO ↔ Domain 映射            │
│  crates: agent-server, agent-channel               │
└────────────────────────┬───────────────────────────┘
                         ▼
┌─ Orchestration（编排层）───────────────────────────┐
│  TurnExecutor · SessionLifecycle · turn 状态机       │
│  crates: agent-orchestrator                         │
└────────────────────────┬───────────────────────────┘
                         ▼
┌─ Capability（能力层）──────────────────────────────┐
│  业务抽象 trait + 默认实现                           │
│  crates: agent-domain, agent-context, agent-llm,    │
│          agent-tools, agent-memory                   │
└────────────────────────▲───────────────────────────┘
                         │ implements traits
┌─ Infrastructure（基础设施层）───────────────────────┐
│  外部系统适配器，实现 Capability trait                │
│  crates: agent-storage                              │
└────────────────────────────────────────────────────┘

Shared: agent-proto（协议生成代码）, agent-e2e（测试）
```

## 依赖规则（硬约束，CI 架构测试强制执行）

**允许：**
- Channel → Orchestration → Capability
- Infrastructure → Capability

**禁止：**
- Orchestration → Infrastructure（编排层不能依赖具体实现）
- Channel → Capability 或 Channel → Infrastructure（接入层不能绕过编排层）
- Capability → Channel / Orchestration / Infrastructure（能力层不能反向依赖）
- Infrastructure → Channel / Orchestration（基础设施不感知上层）

违反依赖方向 = CI 红 = 不能 merge。

## Crate 总表

| Crate | 层 | 一句话职责 |
|---|---|---|
| agent-server | Channel | 进程入口 + Composition Root（DI 在这里） |
| agent-channel | Channel | 协议 handler + 鉴权 + DTO 转换 |
| agent-orchestrator | Orchestration | TurnExecutor + AuthService + turn 流程编排 |
| agent-domain | Capability | 领域模型：Event, Session, User, ports(trait) |
| agent-context | Capability | ContextBuilder + PromptSection 组合 |
| agent-llm | Capability | LlmProvider trait + provider adapter |
| agent-tools | Capability | Tool trait + ToolRuntime + 内置工具 |
| agent-memory | Capability | CompactionService trait + 策略 |
| agent-storage | Infrastructure | PostgreSQL 适配器，实现 Capability port trait |
| agent-proto | 共享 | protobuf 生成代码 |
| agent-e2e | 测试 | 端到端 + 架构依赖测试 |

## 关键设计决策（ADR 摘要）

每条决策都有完整文档在 `docs/archive/design/decisions/`，这里只记结论。

1. **单体部署**（ADR-001）— 一个 binary，trait 边界保留拆分能力
2. **PostgreSQL 唯一存储**（ADR-002）— 结构化 + JSONB + 未来 pgvector；所有查询必须带 user_id
3. **协议是实现细节**（ADR-003）— 不抽象传输层，抽象业务层；加新协议就加一个 handler
4. **Append-only 事件流**（ADR-004/007）— 事件流是 source of truth，消息列表是投影
5. **TurnExecutor + ContextBuilder 分离**（ADR-005）— 编排归编排，上下文归上下文
6. **接入层只做协议转换**（ADR-006）— handler 禁止直连 DB/LLM/工具
7. **PromptSection 可插拔**（ADR-008）— system prompt = section 组合，不是大字符串模板
8. **Provider 能力统一接口**（ADR-009）— stateful/builtin tools/compaction 通过可选字段和元数据建模，不膨胀 trait

## 事件类型（8 种）

UserMessage · AssistantMessage · ToolCallRequest · ToolCallResult · SystemEvent · ConfigChange · CompactionMarker · Summary

事件追加写入，不可变。压缩通过 Summary + CompactionMarker 表达，不改写历史。

## 质量门禁

- `cargo fmt` + `cargo clippy -- -D warnings` + `cargo test`（pre-push hook）
- 覆盖率 ≥ 85%（CI tarpaulin）
- 函数 ≤ 30 行，认知复杂度 ≤ 10
- deny: `cast_possible_truncation`, `cast_sign_loss`, `unwrap_used`, `too_many_lines`, `cognitive_complexity`
- warn: `cast_lossless`, `must_use_candidate`
- 生产代码禁止 `unwrap()`/`expect()` 用于可能失败的操作
- 新功能必须有对应测试

## 安全约束

- JWT secret + 密码 = 环境变量，禁止硬编码
- 仓库是 public 的 — 禁止提交 IP、密码、API key、内部域名
- 多租户查询必须带 tenant/user 维度过滤

## AGENTS.md 维护

每个 crate 和关键子目录都有自己的 AGENTS.md。这是活的记忆，不是一次性文档。

**读**：改代码前先读对应目录的 AGENTS.md。

**写**：改完代码后，如果遇到以下情况，更新对应的 AGENTS.md：
- 新增、删除或重命名了公共接口
- 踩了坑或发现了不明显的约束
- 做了设计决策（为什么选 A 不选 B）
- 修了 bug 且根因涉及架构理解

不确定要不要记？记。宁可多记一条以后删掉，不要漏掉重要的东西。
