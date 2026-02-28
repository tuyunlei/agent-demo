# agent-demo 分层架构设计（D-ARCH-01）

**状态**：Draft（用于 MVP 实施）  
**更新时间**：2026-02-24  
**适用范围**：`server/` Rust 单体（多租户 AI Agent 平台）

---

## 1. 架构总览

agent-demo 采用四层分层架构：**接入层（Channel）→ 编排层（Orchestration）→ 能力层（Capability）← 基础设施层（Infrastructure）**。  
依赖方向遵循“**由外向内**”与“**依赖抽象（trait）而非实现**”：接入层只负责协议与鉴权，编排层负责一次 turn 的流程调度，能力层按领域定义稳定的业务抽象与默认实现，基础设施层仅负责外部系统适配并实现能力层 trait。整体保持 MVP 单体部署（ADR-001），但代码边界按可拆分服务的方式组织。

### 1.1 文本层次图（ASCII）

```text
┌───────────────────────────────────────────────────────────────┐
│ Layer 1: Channel Layer（接入层）                              │
│ - gRPC / HTTP / WebSocket 协议转换                            │
│ - AuthN/AuthZ                                                  │
│ - DTO ↔ Domain Input/Output                                    │
└───────────────────────────┬───────────────────────────────────┘
                            │ call use cases / turn orchestration
                            ▼
┌───────────────────────────────────────────────────────────────┐
│ Layer 2: Orchestration Layer（编排层）                         │
│ - TurnExecutor                                                  │
│ - SessionLifecycle                                              │
│ - 协调：context → llm → tools loop → persist → response        │
└───────────────────────────┬───────────────────────────────────┘
                            │ depend on traits only
                            ▼
┌───────────────────────────────────────────────────────────────┐
│ Layer 3: Capability Layer（能力层）                            │
│ - agent-context  : ContextBuilder / PromptSection              │
│ - agent-llm      : LlmProvider                                 │
│ - agent-tools    : ToolRuntime / Tool                          │
│ - agent-memory   : MemoryProvider                              │
│ - agent-domain   : Event / Session / User / Tool types         │
└───────────────────────────▲───────────────────────────────────┘
                            │ implements traits
┌───────────────────────────┴───────────────────────────────────┐
│ Layer 4: Infrastructure Layer（基础设施层）                    │
│ - PostgreSQL / Redis / File / 3rd-party API adapters          │
│ - 不含业务流程，仅技术对接                                      │
└───────────────────────────────────────────────────────────────┘

Allowed:    L1 -> L2 -> L3,   L4 -> L3
Forbidden:  L1 -> L3, L1 -> L4, L2 -> L4, L3 -> L1/L2/L4
```

---

## 2. 四层定义

> 目标：在保持单体部署的前提下，保证“可演进、可替换、可测试”的模块边界。

### 2.1 Layer 1：接入层（Channel Layer）

**职责**
- 接收外部请求并做协议转换（gRPC、HTTP、WebSocket 等）。
- 执行认证与鉴权（租户身份、用户身份、权限校验）。
- 将外部 DTO 转换为编排层可消费的输入模型；将编排结果映射为协议响应。

**明确不做**
- 不包含业务规则（例如“是否需要压缩会话”“是否继续工具循环”）。
- 不直接操作数据库或第三方 API。
- 不自行拼装 system prompt、不直接调用 LLM provider。

**与其他层交互**
- 只调用 Layer 2 的编排入口（例如 `TurnExecutor::run_turn`）。
- 作为 Composition Root 承担依赖注入（DI）职责：创建 trait 实现并注入 Layer 2。

**MVP 建议**
- 首先保留 gRPC 主入口；HTTP/WebSocket 作为并行扩展入口。
- 鉴权先做最小闭环（tenant_id + user_id + basic permission），接口留扩展点。

### 2.2 Layer 2：编排层（Orchestration Layer）

**职责**
- 协调一次完整 turn 的业务流程：
  1) 接收用户消息输入  
  2) 读取/创建会话  
  3) 调用 ContextBuilder 组装上下文  
  4) 调用 LLM 生成响应  
  5) 处理工具调用循环  
  6) 持久化 event/message/session state  
  7) 返回最终响应

**核心模块**
- `TurnExecutor`：
  - 单次 turn 的状态机与流程驱动器。
  - 控制最大工具迭代次数、错误回退、终止条件。
- `SessionLifecycle`：
  - 会话创建、读取、锁定、压缩、归档等生命周期管理。
  - 与 `TurnExecutor` 协作，确保并发与历史窗口策略一致。

**依赖约束**
- 只能依赖 Layer 3 暴露的 trait 与领域类型。
- 不能依赖 Layer 4 的具体实现（例如不能 import Postgres 实现类型）。

**MVP 建议**
- 首版实现稳定的同步 turn（request/response）；流式输出后续迭代。
- 压缩策略采用“阈值触发 + 简单摘要”即可，先保障可用性。

### 2.3 Layer 3：能力层（Capability Layer）

**职责**
- 提供跨业务流程可复用的能力抽象（trait）与默认实现。
- 以“按域分布”组织，降低耦合并提升可替换性。

#### 2.3.1 `agent-context`
- 关键抽象：
  - `ContextBuilder`：组装模型输入消息（system/history/user/tool）。
  - `PromptSection`：system prompt 的可插拔 section。
- 设计要点：
  - system prompt 采用 section pipeline，避免大字符串硬编码。
  - 首轮构建 system prompt，后续 turn 尽量复用（利于缓存与稳定性）。

#### 2.3.2 `agent-llm`
- 关键抽象：`LlmProvider` trait。
- 设计要点：
  - 统一 chat/stream/tool-call 协议映射。
  - 将 provider 特有参数封装为能力层统一模型，避免泄漏到编排层。

#### 2.3.3 `agent-tools`
- 关键抽象：
  - `Tool`：工具元数据、参数 schema、执行入口。
  - `ToolRuntime`：工具注册、发现、调度、执行与结果回填。
- 设计要点：
  - 支持内置工具（能力层实现）与外部工具（基础设施实现）并存。
  - 工具协议转换集中在 runtime，编排层只处理“调用/结果”语义。

#### 2.3.4 `agent-memory`
- 关键抽象：`MemoryProvider` trait。
- 设计要点：
  - MVP 可先实现最小 memory（按 session/user 检索），为后续语义记忆预留接口。
  - 仅定义业务语义，不绑定向量库或具体存储引擎。

#### 2.3.5 `agent-domain`
- 关键内容：
  - 核心领域类型：`Event`、`Session`、`User`、`Message`、`ToolCall` 等。
  - 领域错误与通用值对象（tenant/session/user 相关 ID 类型）。
- 设计要点：
  - 领域类型稳定优先，外部协议与存储模型不直接污染领域对象。

**trait 与实现关系**
- trait 在 Layer 3 定义。
- 实现可在 Layer 3（如默认 ContextBuilder、内置 Tool）或 Layer 4（如 PostgreSQL 适配器）。

### 2.4 Layer 4：基础设施层（Infrastructure Layer）

**职责**
- 对接外部系统：数据库、缓存、文件系统、三方 API、消息系统等。
- 实现 Layer 3 定义的 trait，并将技术细节隔离在边界内。

**明确不做**
- 不承载业务流程编排。
- 不定义业务语义（例如“何时压缩会话”）。

**典型实现示例**
- `PostgresEventStore` 实现事件存储相关 trait。
- `OpenAIProviderAdapter`/`AnthropicAdapter` 实现 `LlmProvider`。
- `FileSessionArchive` 实现会话归档接口。

**MVP 建议**
- 优先实现最关键适配器：PostgreSQL + 主 LLM Provider。
- 其他外部依赖统一通过 trait stub 接入，满足 Walking Skeleton。

---

## 3. 依赖方向

### 3.1 允许的依赖关系

1. `Layer 1 -> Layer 2`  
   接入层调用编排层用例。
2. `Layer 2 -> Layer 3`  
   编排层依赖能力层抽象与领域类型。
3. `Layer 4 -> Layer 3`  
   基础设施层实现能力层定义的 trait。
4. `Layer 3 (内部域之间)`  
   能力层各域可以在“抽象与领域语义”层面互相引用（例如 tools 依赖 domain 类型）。

### 3.2 禁止的依赖关系

1. `Layer 2 -> Layer 4`（禁止）  
   编排层不能直接依赖 Postgres/SDK 实现类型。
2. `Layer 1 -> Layer 3/4`（禁止）  
   接入层不能绕过编排层直接调能力或基础设施。
3. `Layer 3 -> Layer 1/2/4`（禁止）  
   能力层不能反向依赖上层，也不能直接依赖基础设施实现。
4. `Layer 4 -> Layer 1/2`（禁止）  
   基础设施层不应感知上层流程与协议。

### 3.3 DI（依赖注入）责任

- **Layer 1 是 Composition Root**：
  - 实例化 Layer 4 适配器；
  - 组装 Layer 3 默认实现；
  - 将 trait 对象（或泛型实现）注入 Layer 2；
  - 暴露统一服务入口（gRPC/HTTP handler）。
- 这样可确保业务流程（Layer 2）只面向抽象，替换实现无需修改流程代码。

---

## 4. Crate 映射

> 本节给出“目标结构”与“当前结构”的映射关系，并标注差异。差异仅记录，不在本设计中解决。

### 4.1 当前 workspace crates（现状）

根据 `Cargo.toml`：

- `agent-types`
- `agent-domain`
- `agent-app`
- `agent-proto`
- `agent-storage`
- `agent-llm`
- `agent-channel`
- `agent-server`
- `agent-e2e`

### 4.2 目标 crate 结构（按四层归属）

| 目标 crate | 所属层 | 主要职责 |
|---|---|---|
| `agent-server` | Layer 1 Channel | 进程入口、配置加载、DI 组装 |
| `agent-channel` | Layer 1 Channel | gRPC/HTTP/WebSocket handler、鉴权、DTO 转换 |
| `agent-orchestrator` | Layer 2 Orchestration | `TurnExecutor`、`SessionLifecycle`、turn 流程编排 |
| `agent-context` | Layer 3 Capability | `ContextBuilder`、`PromptSection`、system prompt 组装 |
| `agent-llm` | Layer 3 Capability | `LlmProvider` 抽象与默认实现（若有） |
| `agent-tools` | Layer 3 Capability | `Tool`/`ToolRuntime` 抽象与内置工具能力 |
| `agent-memory` | Layer 3 Capability | `MemoryProvider` 抽象与默认 memory 策略 |
| `agent-domain` | Layer 3 Capability | 核心领域类型、领域错误、值对象 |
| `agent-storage` | Layer 4 Infrastructure | Postgres/File 等存储适配器，实现 Layer 3 trait |
| `agent-llm-adapters`（可选） | Layer 4 Infrastructure | 三方 LLM SDK 适配（当 provider 增多时拆分） |
| `agent-observability`（可选） | Layer 4 Infrastructure | tracing/metrics/exporter 适配 |
| `agent-proto` | Shared（边界协议） | gRPC protobuf 生成类型（供 Layer 1 使用） |
| `agent-e2e` | Test | 端到端集成测试 |

> 注：`agent-proto` 与 `agent-e2e` 不计入核心四层业务能力，但服务于边界协议与验证。

### 4.3 当前 vs 目标差异

| 当前 crate | 当前角色（推断） | 目标状态 | 差异说明 |
|---|---|---|---|
| `agent-app` | 应用服务/编排混合 | **建议迁移为 `agent-orchestrator`** | 命名与职责需聚焦到 turn 编排 |
| `agent-types` | 通用类型容器 | **建议并入 `agent-domain` 或按域下沉** | 避免“万能 types 包”弱化边界 |
| `agent-llm` | LLM 相关 | 保留 | 需明确 trait vs adapter 分层 |
| `agent-storage` | 存储 | 保留 | 明确只放基础设施适配器 |
| `agent-channel` | 接入层 | 保留 | 确保不承载业务逻辑 |
| `agent-server` | 进程入口 | 保留 | 明确作为 Composition Root |
| `agent-domain` | 领域类型 | 保留 | 成为能力层稳定核心 |
| `agent-proto` | 协议定义 | 保留 | 仅作为协议边界共享 |
| `agent-e2e` | 测试 | 保留 | 覆盖四层打通路径 |
| （缺失）`agent-context` | - | **新增** | 当前缺少上下文能力独立边界 |
| （缺失）`agent-tools` | - | **新增** | 当前缺少工具能力独立域 |
| （缺失）`agent-memory` | - | **新增** | 当前缺少记忆能力独立域 |

### 4.4 MVP 可执行落地顺序（仅架构建议）

1. 从 `agent-app` 中抽离 `TurnExecutor`/`SessionLifecycle` 到 `agent-orchestrator`。  
2. 新建 `agent-context`，先落 `ContextBuilder` + 最小 `PromptSection`。  
3. 新建 `agent-tools`，把工具抽象从编排逻辑中剥离。  
4. 新建 `agent-memory`，先提供最小 trait + stub/default impl。  
5. 逐步收敛 `agent-types` 到 `agent-domain` 与各能力域。

---

## 5. 与参考框架的对比与设计选择

### 5.1 借鉴点归纳

#### 来自 ZeroClaw（Rust）
- **trait-first + 按域组织**：provider/tool/memory/context 分域定义 trait。
- **Turn handler 位于编排层**：核心循环集中在单个流程驱动器。
- **PromptSection 可插拔**：system prompt 采用 section 组合，便于扩展。

**为何采用**：与 Rust 的 trait 模型天然契合，能在单体内实现清晰边界，后续拆分成本低。

#### 来自 PicoClaw（Go）
- **ContextBuilder 独立模块化**：上下文组装不混入主循环。
- **SessionManager 职责明确**：会话生命周期有清晰管理者。

**为何采用**：降低 TurnExecutor 复杂度，提升可读性与单测可控性，适合 1 人团队维护。

#### 来自 OpenClaw（TypeScript）
- **生产级会话治理思想**：会话持久化、压缩、归档、重试策略有明确位置。
- **接入与编排分离**：入口层只路由，核心逻辑在 runner/orchestrator。

**为何采用**：多租户场景必须优先保证会话治理；否则系统会在增长阶段过早失控。

### 5.2 关键设计选择与理由

1. **选择四层而非纯三层**  
   - 理由：将“业务能力抽象（Layer 3）”与“技术适配（Layer 4）”分离，更符合 trait + adapter 的 Rust 实践。

2. **编排层只依赖 trait，不依赖实现**  
   - 理由：降低替换成本（LLM provider、存储、工具运行时可独立演进），符合依赖反转原则。

3. **system prompt 采用 PromptSection 组合**  
   - 理由：比“大字符串模板”更可维护，可按租户/场景增删 section，减少改动范围。

4. **会话生命周期独立为 SessionLifecycle**  
   - 理由：多租户下会话治理是核心复杂度来源，必须从 turn 细节中解耦。

5. **MVP 仍保持单体部署（ADR-001）**  
   - 理由：1 人团队优先交付速度；通过 crate 与 trait 边界保留未来服务化可能。

6. **禁止跨层捷径调用**  
   - 理由：短期“快”会导致长期耦合失控。明确禁止 `L2 -> L4` 可避免业务逻辑被基础设施细节污染。

### 5.3 对 MVP 可行性的结论

本架构在不增加部署复杂度的前提下，给出了稳定边界：
- 运行时仍是单 Rust binary；
- 代码层面已具备“替换实现/增量扩展/低成本重构”的结构；
- 对 1 人团队，优先实现最小闭环（gRPC + TurnExecutor + 主 LLM + Postgres），其余能力域可按需求补齐。

---

## 附录 A：依赖规则速查

```text
[Allowed]
agent-server    -> agent-channel, agent-orchestrator, agent-storage, agent-llm(...adapters)
agent-channel   -> agent-orchestrator, agent-domain, agent-proto
agent-orchestrator -> agent-context, agent-llm, agent-tools, agent-memory, agent-domain
agent-storage   -> agent-domain, agent-memory(ports), agent-tools(ports), agent-context(ports)

[Forbidden]
agent-orchestrator -> agent-storage(concrete impl)
agent-channel      -> agent-storage / agent-llm adapters
agent-domain       -> any infra/channel/orchestrator crate
```

## 附录 B：术语

- **turn**：一次用户输入驱动的完整 agent 处理周期。  
- **port/trait**：能力层定义的抽象接口。  
- **adapter**：基础设施层对 port 的具体实现。  
- **composition root**：应用启动时统一装配依赖的位置（本设计中为 Layer 1 入口）。

## 附录 C：单次 Turn 参考时序（文本）

### C.1 正常路径

```text
[Client]
  -> [Channel Handler]
      1) decode request
      2) authn/authz
      3) map dto -> TurnInput
  -> [TurnExecutor]
      4) SessionLifecycle.load_or_create(session_id)
      5) ContextBuilder.build(...)
      6) LlmProvider.chat(messages, tool_specs)
      7) ToolRuntime.execute_if_needed(...)
      8) SessionLifecycle.persist(events/messages)
      9) assemble TurnOutput
  -> [Channel Handler]
      10) map TurnOutput -> dto
  -> [Client]
```

### C.2 工具循环路径

```text
TurnExecutor
  ├─ Iteration #1
  │   ├─ call LlmProvider
  │   ├─ parse tool calls
  │   └─ call ToolRuntime.execute(calls)
  ├─ Iteration #2
  │   ├─ call LlmProvider(with tool results)
  │   └─ maybe more tools
  └─ Final iteration
      ├─ no tool calls
      └─ finalize assistant message
```

### C.3 超限路径

```text
if iteration_count > max_tool_iterations:
  - emit event: ToolLoopExceeded
  - persist turn failure snapshot
  - return controlled error to channel
```

### C.4 会话压缩路径

```text
if token_budget_exceeded || message_window_exceeded:
  SessionLifecycle.compact(session)
    -> MemoryProvider.summarize_or_retrieve(...)
    -> persist summary event
    -> trim old messages by policy
```

---

## 附录 D：Layer 级别边界检查清单（Code Review 用）

### D.1 Layer 1（Channel）检查项

- [ ] handler 中是否只做协议转换与鉴权？
- [ ] handler 是否避免直接访问数据库实现？
- [ ] handler 是否仅调用 orchestrator 公开入口？
- [ ] 输入 DTO 到 domain input 的转换是否单向、显式？
- [ ] 输出 domain output 到 DTO 的转换是否单向、显式？
- [ ] 鉴权失败是否在入口就短路返回？
- [ ] 租户上下文是否在入口阶段注入？
- [ ] 错误码映射是否稳定且与协议绑定？
- [ ] 是否避免在 handler 内拼装 prompt？
- [ ] 是否避免在 handler 内执行工具？
- [ ] 是否避免在 handler 内调用 LLM SDK？
- [ ] trace/span 是否在入口创建？
- [ ] 请求 ID 是否贯穿到 Layer 2？
- [ ] 时间戳/locale 这类 request metadata 是否显式下传？
- [ ] 对流式接口是否仅负责传输层 framing？
- [ ] Channel 层是否零业务策略分支？
- [ ] panic 是否被捕获并映射为受控错误？
- [ ] Channel 层是否不保留跨请求可变状态？
- [ ] 是否将 DI 放在 server 启动路径而不是 handler 内动态 new？
- [ ] 单测是否覆盖鉴权失败、参数非法、正常转发三类路径？

### D.2 Layer 2（Orchestration）检查项

- [ ] 是否由 TurnExecutor 单点驱动 turn？
- [ ] 是否通过 SessionLifecycle 管理会话而非散落函数？
- [ ] 是否仅读取 Layer 3 trait？
- [ ] 是否不 import Layer 4 具体实现？
- [ ] 工具循环上限是否可配置？
- [ ] 失败重试策略是否明确且可审计？
- [ ] 持久化顺序是否固定（先 event 再 session version 等）？
- [ ] 并发策略是否明确（session lock / optimistic version）？
- [ ] 是否把上下文构建委托给 ContextBuilder？
- [ ] 是否把工具协议处理委托给 ToolRuntime？
- [ ] 是否把记忆读取委托给 MemoryProvider？
- [ ] 是否在超限时返回可恢复错误？
- [ ] 是否在异常时记录完整诊断事件？
- [ ] 是否保证无工具调用时快速结束？
- [ ] 是否避免循环内重复构建不必要的静态数据？
- [ ] 是否在返回前统一生成 TurnResult？
- [ ] 代码中是否没有协议层 DTO 类型？
- [ ] 代码中是否没有 SQL/SDK 细节？
- [ ] 是否提供 mock-friendly trait 注入构造器？
- [ ] 单测是否覆盖 0 次/1 次/多次工具循环？

### D.3 Layer 3（Capability）检查项

- [ ] trait 是否命名清晰且单一职责？
- [ ] trait 方法粒度是否避免“万能接口”？
- [ ] domain 类型是否稳定、无外部协议耦合？
- [ ] ContextBuilder 是否可独立测试？
- [ ] PromptSection 是否支持插拔顺序控制？
- [ ] LlmProvider 输入输出模型是否统一？
- [ ] ToolRuntime 是否隔离 provider 差异？
- [ ] MemoryProvider 是否避免绑定具体存储技术？
- [ ] 默认实现是否可被 Layer 4 替换？
- [ ] capability crate 是否不依赖 infra crate？
- [ ] 错误类型是否表达业务语义而非 SDK 错误？
- [ ] 是否提供最小 stub 实现以支撑 walking skeleton？
- [ ] trait 对象化策略是否统一（dyn/generic）？
- [ ] async 边界是否只在 I/O 相关方法出现？
- [ ] 是否避免把配置读取逻辑放入能力层？
- [ ] 是否避免日志基础设施耦合？
- [ ] 工具 schema 是否与执行逻辑一致？
- [ ] memory 检索接口是否包含 tenant/session 维度？
- [ ] context 组装是否支持 token budget 参数？
- [ ] 单测是否覆盖能力层纯逻辑而不依赖外部系统？

### D.4 Layer 4（Infrastructure）检查项

- [ ] 是否仅实现 Layer 3 trait，不新增业务流程？
- [ ] adapter 是否可替换（构造参数化，不写死全局）？
- [ ] 外部 SDK 错误是否转换为能力层错误？
- [ ] SQL/HTTP 请求是否有 timeout 与重试策略？
- [ ] 是否有连接池与资源释放策略？
- [ ] 是否有健康检查接口（若必要）？
- [ ] 配置项是否集中管理并可覆盖？
- [ ] 是否避免在 adapter 内读写会话业务状态机？
- [ ] 数据模型映射是否集中且可测试？
- [ ] 是否避免把 proto DTO 泄漏到 infra？
- [ ] 是否有最小集成测试覆盖真实依赖？
- [ ] 是否在日志中避免敏感信息？
- [ ] 是否支持租户隔离字段透传？
- [ ] 是否支持幂等写入（按事件 ID/版本）？
- [ ] 是否支持回放/审计所需字段？
- [ ] 是否在失败时保留足够诊断上下文？
- [ ] 是否确保 schema 变更可迁移？
- [ ] 是否确保 adapter 不依赖 channel/orchestrator crate？
- [ ] 是否将三方 API 限流封装在 adapter 内？
- [ ] 是否保证 shutdown 时优雅回收连接？

---

## 附录 E：核心接口建议（草案级）

> 以下是职责示意，不是最终代码。

### E.1 Orchestration

```rust
pub trait TurnExecutor {
    async fn run_turn(&self, input: TurnInput) -> Result<TurnOutput, OrchestratorError>;
}

pub trait SessionLifecycle {
    async fn load_or_create(&self, key: SessionKey) -> Result<SessionState, SessionError>;
    async fn persist_turn(&self, delta: SessionDelta) -> Result<(), SessionError>;
    async fn compact_if_needed(&self, key: SessionKey) -> Result<(), SessionError>;
    async fn archive_if_needed(&self, key: SessionKey) -> Result<(), SessionError>;
}
```

### E.2 Context

```rust
pub trait PromptSection: Send + Sync {
    fn name(&self) -> &'static str;
    fn build(&self, ctx: &PromptBuildContext) -> Result<String, ContextError>;
}

pub trait ContextBuilder: Send + Sync {
    fn build_system_prompt(&self, ctx: &PromptBuildContext) -> Result<String, ContextError>;
    fn build_messages(&self, input: ContextInput) -> Result<Vec<ModelMessage>, ContextError>;
}
```

### E.3 LLM

```rust
pub trait LlmProvider: Send + Sync {
    async fn complete(&self, req: LlmRequest) -> Result<LlmResponse, LlmError>;
    async fn stream(&self, req: LlmRequest) -> Result<LlmStream, LlmError>;
}
```

### E.4 Tools

```rust
pub trait Tool: Send + Sync {
    fn spec(&self) -> ToolSpec;
    async fn execute(&self, input: ToolInput) -> Result<ToolOutput, ToolError>;
}

pub trait ToolRuntime: Send + Sync {
    fn register(&mut self, tool: Box<dyn Tool>);
    fn list_specs(&self) -> Vec<ToolSpec>;
    async fn execute_calls(&self, calls: Vec<ToolCall>) -> Vec<ToolResult>;
}
```

### E.5 Memory

```rust
pub trait MemoryProvider: Send + Sync {
    async fn recall(&self, query: MemoryQuery) -> Result<Vec<MemoryItem>, MemoryError>;
    async fn store(&self, item: MemoryItem) -> Result<(), MemoryError>;
}
```

---

## 附录 F：目标 crate 详细职责清单

### F.1 `agent-server`（Layer 1）

- 启动应用（config、logger、runtime）。
- 创建连接池与外部客户端。
- 组装 adapter 实现。
- 构建 capability 默认实现。
- 构建 orchestrator 实例。
- 注入 channel handlers。
- 托管进程生命周期。
- 暴露 health/readiness 端点。
- 注入全局限流器（若有）。
- 管理优雅停机。

### F.2 `agent-channel`（Layer 1）

- gRPC 服务定义实现。
- HTTP handler（后续）。
- WebSocket handler（后续）。
- token 解析与校验。
- tenant/user 上下文建立。
- DTO <-> domain input/output 映射。
- 协议错误码映射。
- stream framing（若使用流式）。
- request tracing 注入。
- 入口级请求校验。

### F.3 `agent-orchestrator`（Layer 2）

- TurnExecutor 实现。
- SessionLifecycle 实现或门面。
- 工具循环状态机。
- token budget 决策。
- 重试与兜底策略。
- 回写事件顺序控制。
- 失败路径归一化。
- 统一 TurnResult 结构。
- 审计事件触发。
- 编排层单元测试。

### F.4 `agent-context`（Layer 3）

- PromptSection trait。
- 内置 sections（identity/safety/tools/runtime/date 等）。
- section pipeline 组装器。
- message window 选择策略。
- 历史消息裁剪策略。
- system prompt 构建缓存策略。
- context 输入模型定义。
- context 错误类型定义。
- context 默认实现。
- context 纯逻辑测试。

### F.5 `agent-llm`（Layer 3）

- LlmProvider trait。
- Provider-neutral 请求/响应模型。
- 工具调用统一协议模型。
- stream chunk 统一模型。
- provider 错误归类。
- provider 能力声明（supports_tools/stream）。
- fallback provider 抽象（可选）。
- 速率限制抽象（可选）。
- mock provider。
- trait 合约测试。

### F.6 `agent-tools`（Layer 3）

- Tool trait。
- ToolSpec 定义。
- ToolRuntime trait。
- 内置工具注册表。
- 工具发现机制。
- 工具调用编解码。
- 工具结果标准化。
- 工具超时/取消策略。
- 工具审计事件模型。
- 工具层合约测试。

### F.7 `agent-memory`（Layer 3）

- MemoryProvider trait。
- Memory item/query 模型。
- recall/store 生命周期定义。
- 租户/用户/会话维度键模型。
- memory 默认策略（MVP）。
- memory TTL/保留策略（可选）。
- memory 结果排序策略。
- memory 错误类型。
- memory mock/stub。
- memory 合约测试。

### F.8 `agent-domain`（Layer 3）

- Session 领域模型。
- Event 领域模型。
- Message 领域模型。
- ToolCall/ToolResult 领域模型。
- User/Tenant 值对象。
- 领域错误类型。
- ID/Version 类型封装。
- 时间与状态枚举。
- 领域不变量校验。
- 领域单元测试。

### F.9 `agent-storage`（Layer 4）

- Postgres adapter。
- 事件表读写实现。
- 会话状态持久化实现。
- 乐观锁/版本控制实现。
- 归档存储实现（文件或库表）。
- 分页/索引优化。
- migration 管理。
- 仓储映射层。
- SQL 错误转换。
- infra 集成测试。

### F.10 `agent-proto`（Shared）

- protobuf 定义输出。
- tonic 生成代码。
- 协议兼容性约束。
- DTO schema 版本管理。
- proto lint（若有）。

### F.11 `agent-e2e`（Test）

- 端到端 happy path。
- 鉴权失败路径。
- 工具循环路径。
- 会话压缩路径。
- provider 失败回退路径。
- 多租户隔离路径。
- 并发请求路径。
- 数据一致性校验。
- 升级兼容性校验。
- 回归测试入口。

---

## 附录 G：目标结构与当前结构差异清单（展开）

### G.1 组织层面差异

1. 现状存在 `agent-app`，目标将其职责明确为 `agent-orchestrator`。
2. 现状缺少 `agent-context` 独立域，目标要求拆出上下文能力。
3. 现状缺少 `agent-tools` 独立域，目标要求工具抽象独立。
4. 现状缺少 `agent-memory` 独立域，目标要求记忆抽象独立。
5. 现状有 `agent-types`，目标建议逐步去中心化，避免“杂项类型仓库”。

### G.2 依赖层面差异

1. 目标要求编排层仅依赖 trait；现状可能仍有实现耦合风险。
2. 目标要求 Layer 1 成为唯一 DI 根；现状可能在内部模块自行 new。
3. 目标要求 capability 与 infra 明确双向边界（定义/实现）；现状需进一步收敛。

### G.3 测试层面差异

1. 目标要求各 capability 域有合约测试；现状可能以集成测试为主。
2. 目标要求 orchestrator 可通过 mock 全量单测；现状需检查注入方式。
3. 目标要求 e2e 覆盖多租户隔离场景；现状需补充覆盖。

### G.4 命名层面差异

1. `agent-app` 命名过泛，不利于表达“编排职责”。
2. `agent-types` 命名过泛，不利于约束边界。
3. 新增域命名需与职责一一对应，避免再次出现大杂烩 crate。

---

## 附录 H：关键风险与缓解策略（MVP 视角）

### H.1 风险：过度抽象导致开发速度下降

- 现象：为了“完美分层”引入过多 trait 与 wrapper。
- 影响：1 人团队开发效率下降。
- 缓解：
  - 每个域先定义最小 trait（2~4 个核心方法）。
  - 先有一个默认实现跑通，再扩展可选能力。
  - 非必要能力（多 provider fallback、复杂策略）延后。

### H.2 风险：编排层偷穿到基础设施

- 现象：TurnExecutor 直接拿 SQL client 或 SDK 调用。
- 影响：后续替换成本高，测试困难。
- 缓解：
  - review 强制检查 `L2 -> L4` import。
  - 编排层构造函数只接受 trait。
  - 在 CI 中增加依赖边界检查（后续）。

### H.3 风险：会话治理不足

- 现象：会话只“能存”，但无压缩、归档、并发策略。
- 影响：成本与稳定性快速恶化。
- 缓解：
  - MVP 即落地 SessionLifecycle 最小版本。
  - 先做阈值压缩与版本并发控制。
  - 关键事件可追踪（create/load/compact/archive）。

### H.4 风险：工具循环不受控

- 现象：工具调用反复循环、无终止条件。
- 影响：成本飙升、响应超时。
- 缓解：
  - 强制 `max_tool_iterations`。
  - 每轮记录调用轨迹。
  - 超限返回受控错误并落审计事件。

### H.5 风险：上下文组装碎片化

- 现象：不同模块各自拼 prompt，行为不一致。
- 影响：模型输出不稳定、难以调优。
- 缓解：
  - 唯一 `ContextBuilder` 入口。
  - system prompt 统一由 PromptSection pipeline 生成。
  - 变更 section 顺序需要评审。

### H.6 风险：多租户边界泄漏

- 现象：部分查询/缓存键未带 tenant 维度。
- 影响：严重数据隔离问题。
- 缓解：
  - domain key 类型强制包含 tenant。
  - memory/store 接口必须携带 tenant 维度。
  - e2e 必须包含 cross-tenant 防串测例。

---

## 附录 I：实施阶段里程碑（文档级计划）

### I.1 M0（Walking Skeleton）

- 目标：
  - gRPC 入口 + 单次 turn + 主 LLM + 最小持久化跑通。
- 完成标准：
  - 请求可进入、可返回、可落库。
  - 无工具调用也能稳定输出。

### I.2 M1（工具与上下文能力成型）

- 目标：
  - `agent-context`、`agent-tools` 落地最小可用实现。
- 完成标准：
  - 至少 1 个工具可被模型调用并回填结果。
  - system prompt 可通过 section 扩展。

### I.3 M2（会话治理增强）

- 目标：
  - SessionLifecycle 支持压缩与归档基本策略。
- 完成标准：
  - 长会话可稳定处理，不出现无界增长。

### I.4 M3（多租户与可靠性）

- 目标：
  - 强化 tenant 隔离、并发控制、可观测性。
- 完成标准：
  - 关键路径有 trace/event；多租户 e2e 通过。

---

## 附录 J：决策对齐（与既有文档）

### J.1 与 ADR-001（单体）的一致性

- 本文坚持“单体部署 + 模块边界清晰”原则。
- 不要求服务拆分，不引入分布式复杂度。
- 通过 crate/trait 组织为未来演进预留空间。

### J.2 与 design/principles.md 的一致性

- 满足由外向内依赖。
- 满足依赖反转与接口隔离。
- 满足 YAGNI 与可扩展平衡。
- 满足 Walking Skeleton 先打通再演进。

### J.3 与 framework research 的一致性

- 借鉴 ZeroClaw：trait-first、PromptSection。
- 借鉴 PicoClaw：ContextBuilder/SessionManager 边界。
- 借鉴 OpenClaw：生产级 session 治理意识。

