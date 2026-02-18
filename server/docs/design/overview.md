# 架构总览（Architecture Overview）

> 面向多用户托管式 AI Agent SaaS 的系统全景蓝图（MVP）

## 1. 系统定位

本系统是一个**面向普通用户**的多租户托管式 AI Agent 平台：用户通过 iOS App 获得"专属 AI 助手"，核心价值是**个性化陪伴 + 生活协助 + 低门槛持续调教**。MVP 以邀请制登录、Onboarding 聊天、流式对话、会话持久化和 Agent 人格为核心，后端采用 Rust 单体（模块化边界）+ gRPC + PostgreSQL，在保证快速交付的同时，为后续多渠道接入、多模型切换、工具生态扩展预留标准化扩展点。

---

## 2. C4 Level 1 - 系统上下文（System Context）

### 2.1 文字说明

系统对外处于"应用核心中台"位置：

- 对内服务对象：iOS 客户端（最终用户入口）
- 对外依赖：
  - LLM API（OpenAI / Anthropic / 国产模型等）
  - 工具服务（搜索、天气等）
  - APNs（消息推送）
  - MCP 服务（未来外部工具协议接入）

系统边界内统一承载认证、会话、Agent Runtime、工具编排与数据持久化。所有外部依赖均通过适配器接入，核心业务不直接依赖外部 SDK/协议细节。

### 2.2 ASCII 图

```text
                +------------------------+
                |         用户           |
                |   (iOS App 使用者)     |
                +-----------+------------+
                            |
                            | gRPC/HTTP2 + JWT
                            v
+-------------------------------------------------------------------+
|            AI Agent Platform (Rust Monolith, Hexagonal)           |
|                                                                   |
|  认证 / 会话 / 对话流 / Agent Runtime / 工具编排 / 持久化          |
+-------------------------------------------------------------------+
      |                    |                    |                 |
      | LLM 调用           | Tool API 调用      | Push            | MCP（未来）
      v                    v                    v                 v
+-------------+     +--------------+     +-------------+   +--------------+
|  LLM APIs   |     | Tool Services|     |    APNs     |   | MCP Services |
| (可切换)     |     |(搜索/天气等) |     |(iOS 推送)    |   |(外部工具协议) |
+-------------+     +--------------+     +-------------+   +--------------+
```

---

## 3. C4 Level 2 - 容器视图（Container / Module View）

> 说明：此处"容器"指单体内**逻辑容器（模块）**，不是部署容器。

### 3.1 模块职责

1. **API Gateway**
   - gRPC Server（tonic）
   - 统一鉴权（JWT）、限流、请求路由
   - 对外暴露 `Auth / Chat / Agent / Session` 四类 Service（ADR-006）

2. **Channel Adapter**
   - 抽象不同消息入口（MVP 为 App）
   - 将渠道请求转换为统一应用命令/事件

3. **Agent Service**
   - 业务核心：Agent Runtime、上下文组装、会话生命周期管理
   - 编排 LLM Gateway、Tool Runtime、Data Layer
   - 执行 ADR-005 主链路

4. **LLM Gateway**
   - 抽象多模型 Provider
   - 负责 provider 选择、重试、降级
   - 统一模型调用语义，隔离第三方 API 差异

5. **Tool Runtime**
   - 工具注册、发现、执行
   - 支持 server-side 工具；保留 client-side / MCP 扩展点
   - 工具失败返回 Runtime 交由 LLM 决策（ADR-005）

6. **Data Layer**
   - PostgreSQL 持久化访问抽象（Repository Ports）
   - 结构化数据 + JSONB（未来可扩展 pgvector）
   - 落实多租户隔离：所有查询强制带 `user_id`（ADR-004）

### 3.2 六边形依赖方向（外向内）

```text
[外部世界]
  iOS / LLM APIs / Tool APIs / APNs / MCP
        |
        v
+---------------------------- Adapters Layer -----------------------------+
|  API Gateway  | Channel Adapter | LLM Provider Adapter | Tool Adapter  |
+-------------------------------+-----------------------------------------+
                                |
                                v
+----------------------------- Application -------------------------------+
|                         Agent Service (Use Cases)                      |
+-------------------------------+-----------------------------------------+
                                |
                                v
+------------------------------- Domain ----------------------------------+
|     Ports (trait interfaces) / Domain Models / Domain Rules            |
+-------------------------------+-----------------------------------------+
                                |
                                v
+--------------------------- Infrastructure ------------------------------+
|                  Data Layer (PostgreSQL via sqlx)                      |
+------------------------------------------------------------------------+

依赖规则：Adapters -> Application -> Domain(Ports)
实现关系：Infrastructure/Adapters 实现 Domain 定义的 Ports
Domain 不反向依赖任何外部框架或协议
```

### 3.3 容器交互图（运行时）

```text
iOS App
   |
   | gRPC (Auth/Chat/Agent/Session)
   v
[API Gateway] ---> [Channel Adapter] ---> [Agent Service]
                                          |      |      |
                                          |      |      +--> [Data Layer -> PostgreSQL]
                                          |      |
                                          |      +---------> [Tool Runtime] ---> Tool Services / MCP(未来)
                                          |
                                          +---------------> [LLM Gateway] ---> OpenAI/Anthropic/国产模型

[Agent Service] ---> ChatEvent Stream ---> [API Gateway] ---> iOS App
[Agent Service] ---> Push Trigger(异步) ---> APNs
```

---

## 4. 技术栈总结

| 维度 | 选型 | 当前结论 | 说明 |
|---|---|---|---|
| 后端语言 | Rust | 已定 | 单体 binary，模块化边界（ADR-001） |
| 架构风格 | 六边形架构（Ports & Adapters） | 已定 | Core 定义 trait，Adapter 实现，依赖外向内 |
| 通信协议 | gRPC（tonic） | 已定 | HTTP/2 + Protobuf，统一 Service 契约（ADR-003/006） |
| 异步运行时 | Tokio | 已定 | 支撑高并发 I/O 与流式响应 |
| 数据存储 | PostgreSQL（sqlx） | 已定 | 唯一主存储；结构化 + JSONB（ADR-002） |
| 向量扩展 | pgvector（未来） | 预留 | 当前不强依赖，作为记忆检索演进点 |
| 认证机制 | JWT + 邀请码 + 设备密钥 | MVP 已定 | 先满足可用与安全基线，后续可升级 |
| 客户端 | iOS（Swift + SwiftUI + grpc-swift） | MVP 已定 | 首端聚焦移动体验与快速验证 |
| LLM 接入 | 多 Provider 抽象 | 已定 | 通过 LLM Gateway 做切换/重试/降级 |
| 工具集成 | Tool Runtime（server-side，预留 client/MCP） | 已定 | 统一注册与执行模型，便于扩展 |
| 推送通道 | APNs | 已定 | 异步触达 iOS 用户 |

---

## 5. 核心数据流（用户消息完整链路）

以下描述对应 ADR-005 + ADR-006：

1. **用户发起消息**
   - iOS 通过 `ChatService` 发起 gRPC 请求（携带 JWT、会话信息、消息内容）。

2. **入口处理**
   - API Gateway 完成鉴权、限流、路由。
   - Channel Adapter 将请求转换为统一内部命令。

3. **上下文组装**
   - Agent Service 从 Data Layer 读取会话历史、Agent 人格、长期记忆摘要等上下文（按 `user_id` 约束查询）。

4. **首次 LLM 推理**
   - Agent Service 调用 LLM Gateway。
   - LLM Gateway 选择 provider 并发起请求，返回模型输出（含可能的工具调用意图）。

5. **工具调用循环（可多轮）**
   - 若模型请求工具：Agent Service 交给 Tool Runtime 执行。
   - Tool Runtime 调用外部工具服务（未来可接 MCP）。
   - 工具结果回填给 Agent Service，再送回 LLM Gateway 继续推理。
   - 重复直到模型产出最终回复。

6. **失败与降级策略**
   - **工具失败**：错误信息显式回传给 LLM，由 LLM 决定重试、替代回答或道歉说明。
   - **LLM 失败**：LLM Gateway 执行重试/切换/降级；若仍失败，返回统一降级响应（保证链路可结束）。

7. **持久化与流式返回**
   - Agent Service 将用户消息、助手消息、关键中间状态持久化到 PostgreSQL。
   - 通过 `ChatEvent` 统一事件流向客户端持续输出（token/chunk/状态事件等）。

8. **异步后处理**
   - 触发记忆沉淀、摘要更新、必要的推送调度（APNs）等后台任务，不阻塞主对话响应。

```text
User -> iOS -> API Gateway -> Agent Service
                     |             |
                     |             +-> Data Layer(读取上下文)
                     |             +-> LLM Gateway(推理)
                     |                         |
                     |                         +-> Tool Runtime(若需工具, 循环)
                     |                                      |
                     |                                      +-> External Tools/MCP
                     |
                     +<---- ChatEvent Stream <---- Agent Service(流式输出)
                                   |
                                   +-> Data Layer(持久化)
                                   +-> Async Post-Process / APNs
```

---

## 6. 关键架构特征（质量属性）

### 6.1 可扩展性（Scalability / Extensibility）

- 单体内模块化边界（ADR-001）降低早期分布式复杂度，同时支持后续按模块拆分。
- Tool Runtime、LLM Gateway 以 Port 抽象变化点：新增 provider/工具主要是"新增实现"。
- 数据层使用 PostgreSQL + JSONB，兼顾结构化稳定字段与快速迭代字段。

### 6.2 可替换性（Replaceability）

- 传输层与业务层解耦：协议变更（如未来 HTTP API）主要影响 Adapter，不侵入核心用例。
- LLM provider 切换在 Gateway 层完成，Agent Service 不感知具体厂商。
- 工具调用统一运行时接口，替换外部服务时只改工具适配器。

### 6.3 多租户隔离（Multi-tenancy Isolation）

- 核心实体模型明确租户边界：`User -> Agent -> Session -> Message`，`Agent -> Memory`（ADR-004）。
- 查询约束：所有读写必须显式带 `user_id`，从数据访问层制度化隔离。
- 认证（JWT + 设备密钥）与数据访问控制协同，避免跨用户数据串读。

### 6.4 可演进性（Evolvability）

- MVP 先聚焦 iOS 单渠道与核心对话闭环，减少前期设计负担。
- 通过 Channel Adapter 预留多渠道扩展（Telegram 等）。
- 通过 MCP 扩展位支持未来工具生态标准化接入。
- `pgvector` 作为未来能力增量，不阻塞当前主链路交付（YAGNI）。

### 6.5 可靠性与可恢复性（Reliability）

- LLM Gateway 内建重试与降级路径，避免单 provider 故障直接中断用户体验。
- 工具失败不直接终止会话，交由模型做语义层恢复。
- 流式协议 + 持久化确保"响应可见 + 状态可追踪"。

---

## 7. 设计约束

### 7.1 不可妥协约束（Must）

1. **严格六边形架构**：核心领域仅定义 Port（trait）与业务规则，不依赖框架/SDK。
2. **依赖方向固定**：只允许外层依赖内层，禁止 Domain 反向依赖 Adapter。
3. **关注点分离**：API、业务编排、模型网关、工具运行时、数据访问职责清晰。
4. **显式设计**：显式依赖注入、显式错误传播、显式边界，不使用隐式全局状态。
5. **多租户隔离硬约束**：所有数据查询必须带 `user_id`。
6. **与 ADR 一致**：本总览不得与 ADR-001~006 冲突。

### 7.2 有意简化（MVP, By Design）

1. **单体部署形态**：优先交付速度与可维护性，暂不引入微服务治理复杂度。
2. **单数据库策略**：仅 PostgreSQL，避免多存储一致性复杂度。
3. **单客户端优先**：先 iOS，验证核心价值后再扩展多端。
4. **认证先用 JWT 体系**：满足 MVP 安全与体验平衡，后续再演进更复杂机制。
5. **不在本文件展开部署拓扑与模块内部细节**：详见对应子文档。

---

## 8. 模块索引（详细设计文档入口）

### 架构决策记录（ADR）

| ADR | 文档 |
|-----|------|
| ADR-001 单体 + 模块化 | [decisions/001-monolith.md](decisions/001-monolith.md) |
| ADR-002 PostgreSQL | [decisions/002-postgresql.md](decisions/002-postgresql.md) |
| ADR-003 gRPC | [decisions/003-grpc.md](decisions/003-grpc.md) |
| ADR-004 数据模型 | [decisions/004-data-model.md](decisions/004-data-model.md) |
| ADR-005 Agent Runtime | [decisions/005-agent-runtime.md](decisions/005-agent-runtime.md) |
| ADR-006 gRPC 接口 | [decisions/006-grpc-interfaces.md](decisions/006-grpc-interfaces.md) |

### 核心架构

| 模块 | 文档 |
|------|------|
| 数据模型详细设计 | [core/data-model.md](core/data-model.md) |
| Agent Runtime 详细设计 | [core/agent-runtime.md](core/agent-runtime.md) |
| 上下文编排策略 | [core/context-management.md](core/context-management.md) |
| 人格系统 | [core/persona-system.md](core/persona-system.md) |
| 记忆系统 | [core/memory-system.md](core/memory-system.md) |

### 子系统

| 模块 | 文档 |
|------|------|
| 工具系统 | [tool-system/overview.md](tool-system/overview.md) |
| LLM Gateway | [llm-gateway/overview.md](llm-gateway/overview.md) |
| Channel 系统 | [channel-system/overview.md](channel-system/overview.md) |
| 认证授权 | [auth/overview.md](auth/overview.md) |
| 并发模型 | [concurrency/overview.md](concurrency/overview.md) |
| 推送系统 | [push-system/overview.md](push-system/overview.md) |
| 语音交互 | [voice/overview.md](voice/overview.md) |

### 协议与横切

| 模块 | 文档 |
|------|------|
| gRPC Service 定义 | [protocols/grpc-services.md](protocols/grpc-services.md) |
| gRPC Message 定义 | [protocols/grpc-messages.md](protocols/grpc-messages.md) |
| 可观测性 | [observability.md](observability.md) |
| 成本控制 | [cost-control.md](cost-control.md) |
| 部署方案 | [deployment.md](deployment.md) |

### 参考

| 文档 | 说明 |
|------|------|
| [principles.md](principles.md) | 架构设计原则与品味约束 |
| [thinking-toolkit.md](thinking-toolkit.md) | 思考框架工具箱 |
