# Agent Platform — 架构设计 ROADMAP

全局路由表。每个方向链接到对应的设计文档，标注状态。

---

## 产品设计 → `product/`

| 方向 | 文档 | 状态 |
|------|------|------|
| 产品愿景与 SCQA | `product/vision.md` | ✅ 已完成 |
| 约束清单 | `product/constraints.md` | ✅ 已完成 |
| Onboarding 流程 | `product/onboarding.md` | 🔲 待设计 |
| 用户调教机制 | `product/customization.md` | 🔲 待设计 |
| 主动性设计 | `product/proactivity.md` | 🔲 待设计 |
| 功能引导/发现 | `product/feature-discovery.md` | 🔲 待设计 |
| 多 agent | `product/multi-agent.md` | 🔲 待设计 |
| 阻塞点（等涂涂决策） | `product/blocked.md` | 📋 持续更新 |

## 架构设计 → `design/`

### 基础决策（ADR）→ `design/decisions/`

| ADR | 内容 | 状态 |
|-----|------|------|
| 001 | 单体 + 模块化 | ✅ |
| 002 | PostgreSQL | ✅ |
| 003 | gRPC (tonic) | ✅ |
| 004 | 数据模型 | ✅ |
| 005 | Agent Runtime 流程 | ✅ |
| 006 | gRPC 接口定义 | ✅ |

### 架构总览 → `design/overview.md`

| 内容 | 状态 |
|------|------|
| C4 Level 1 Context | ✅ 已完成 |
| C4 Level 2 Container | ✅ 已完成 |
| 技术栈总结 | ✅ 已完成 |
| 核心数据流 | ✅ 已完成 |
| 关键架构特征 | ✅ 已完成 |
| 设计约束 | ✅ 已完成 |
| 模块索引 | ✅ 已完成 |

### 核心架构 → `design/core/`

| 方向 | 文档 | 状态 |
|------|------|------|
| 数据模型详细设计 | `design/core/data-model.md` | ✅ 已完成 |
| Agent Runtime 详细设计 | `design/core/agent-runtime.md` | ✅ 已完成 |
| 上下文编排策略 | `design/core/context-management.md` | ✅ 已完成 |
| 人格系统 | `design/core/persona-system.md` | ✅ 已完成 |
| 记忆系统 | `design/core/memory-system.md` | ✅ 已完成 |

### 工具系统 → `design/tool-system/`

| 方向 | 文档 | 状态 |
|------|------|------|
| 工具系统总览 | `design/tool-system/overview.md` | ✅ 已完成 |
| Tool 接口设计 | `design/tool-system/tool-interface.md` | 🔲 已调研，待整理 |
| 注册与发现 | `design/tool-system/registry.md` | 🔲 待设计 |
| 执行流程 | `design/tool-system/execution.md` | 🔲 待设计 |
| 客户端工具 | `design/tool-system/client-tools.md` | ⏳ 方向记录已有 |
| MCP 接入 | `design/tool-system/mcp.md` | 🔲 待设计 |

### LLM 网关 → `design/llm-gateway/`

| 方向 | 文档 | 状态 |
|------|------|------|
| LLM Gateway 总览 | `design/llm-gateway/overview.md` | ✅ 已完成 |
| 多 provider 切换 | `design/llm-gateway/provider-switching.md` | 🔲 待设计 |
| 重试/降级/fallback | `design/llm-gateway/resilience.md` | 🔲 待设计 |

### Channel 系统 → `design/channel-system/`

| 方向 | 文档 | 状态 |
|------|------|------|
| Channel 抽象设计 | `design/channel-system/overview.md` | ✅ 已完成 |

### 并发模型 → `design/concurrency/`

| 方向 | 文档 | 状态 |
|------|------|------|
| 并发模型设计 | `design/concurrency/overview.md` | ✅ 已完成 |

### 认证授权 → `design/auth/`

| 方向 | 文档 | 状态 |
|------|------|------|
| 认证方案设计 | `design/auth/overview.md` | ✅ 已完成 |

### 推送系统 → `design/push-system/`

| 方向 | 文档 | 状态 |
|------|------|------|
| 推送架构设计 | `design/push-system/overview.md` | 🔲 待设计 |

### 语音交互 → `design/voice/`

| 方向 | 文档 | 状态 |
|------|------|------|
| 语音交互方向 | `design/voice/overview.md` | ⏳ 方向记录已有 |

### 协议设计 → `design/protocols/`

| 方向 | 文档 | 状态 |
|------|------|------|
| gRPC Service 定义 | `design/protocols/grpc-services.md` | ✅ 已完成（含 Message 定义） |
| gRPC Message 定义 | （合并到 grpc-services.md） | ✅ 已完成 |

### 横切关注点

| 方向 | 文档 | 状态 |
|------|------|------|
| 可观测性 | `design/observability.md` | ✅ 已完成 |
| 成本控制 | `design/cost-control.md` | 🔲 待设计 |
| 部署方案 | `design/deployment.md` | ✅ 已完成 |

## 参考资料 → `design/`

| 文档 | 说明 |
|------|------|
| `design/principles.md` | 架构设计原则与品味约束 |
| `design/thinking-toolkit.md` | 思考框架工具箱 |

---

*状态说明：✅ 已完成 | ⏳ 进行中/部分完成 | 🔲 待设计 | 🚫 阻塞 | 📋 持续更新*
