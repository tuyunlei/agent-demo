# 工具系统总览设计（Tool System Overview）

## 1. 目标与边界

本文定义多用户托管式 AI Agent 平台中的工具系统总体设计，目标是让 Agent 在对话外具备可执行能力（搜索、天气、日程、文件等），并与既有 Agent Runtime 工具循环、消息模型、ChatEvent 机制一致。

**本文关注：** 工具生命周期、分类、接口、注册与发现、执行流程、安全权限、MCP 接入、MVP 演进。  
**本文不重复：** Runtime 核心循环细节、Prompt 注入策略、Rust 代码实现。

---

## 2. 架构定位（六边形架构）

### 2.1 分层职责

- **Domain 层（核心）**
  - `Tool` 抽象（trait/接口语义）
  - `ToolRegistry`（注册与发现）
  - `ToolExecutionPolicy`（超时/并行/重试策略语义）
  - `ToolAuthorizationPolicy`（可用性与权限决策）
- **Application 层（编排）**
  - `ToolRuntimePort` 的实现编排（接 Runtime）
  - 工具执行协调（server/client 路由、事件发射、结果归并）
- **Adapter 层（外部实现）**
  - 具体 server-side 工具（搜索、天气等）
  - client-side 执行通道（gRPC ClientToolRequest/Response）
  - MCP 适配器（MCP server/client transport）
  - 外部 API SDK、沙盒执行器、审计日志基础设施

### 2.2 与既有模块对接

- Runtime 通过 `ToolRuntimePort` 发起调用（已定）。
- 调用信息落库到 `messages.tool_calls` / `messages.tool_result`（已定）。
- 过程事件使用 `ToolCallStart` / `ToolCallResult` / `ClientToolRequest`（已定）。

---

## 3. 工具生命周期（注册 → 发现 → 声明 → 调用 → 执行 → 返回）

1. **注册（Register）**
   - 系统启动或租户配置变更时，将工具元数据与执行器绑定到 `ToolRegistry`。
   - 注册维度包括：作用域、执行目标、版本、启用状态。

2. **发现（Discover）**
   - Runtime 在一次会话/回合前，根据 `tenant/user/agent` 上下文查询可用工具集合。
   - 发现阶段只返回“候选且可声明”的工具（通过权限与状态过滤）。

3. **声明（Declare）**
   - 将发现结果转换为 LLM function/tool schema（名称、描述、参数）。
   - 声明内容与工具实体建立稳定映射（`tool_name + version`）。

4. **调用（Invoke）**
   - LLM 产出 `tool_call`（含 `call_id/tool_name/arguments`），Runtime 交给 `ToolRuntimePort`。

5. **执行（Execute）**
   - 执行器依据工具元数据路由到 Server/Client/Hybrid/MCP。
   - 发射 `ToolCallStart`，执行参数校验、鉴权、超时与并行控制。

6. **返回（Return）**
   - 统一封装 `tool_result`（成功/失败/超时/拒绝），写入 `messages.tool_result`。
   - 发射 `ToolCallResult`，结果回填 Runtime，进入下一轮 LLM 决策。

---

## 4. 工具分类与执行目标

> 统一枚举：`ExecutionTarget = Server | Client | Hybrid | MCP`

### 4.1 Server-side Tools

**定义：** 在平台后端执行。  
**特点：**
- 稳定、可观测、易统一治理
- 不依赖终端在线
- 适合标准化 API（搜索、天气、汇率、知识检索）

**适用场景：** 通用能力、弱个性化数据访问、需平台级审计。

### 4.2 Client-side Tools

**定义：** 在用户设备/客户端执行，通过事件通道回传结果。  
**特点：**
- 可访问设备私有能力（相册、联系人、日历、定位）
- 受设备在线状态与客户端版本影响
- 需要明确用户授权边界

**适用场景：** 强隐私本地数据、设备 API、系统级交互。

### 4.3 Hybrid Tools

**定义：** 同一工具语义可在两端执行，按策略择优路由。  
**特点：**
- 有回退能力（client 不在线可 server 执行，反之亦然）
- 需要一致的输入输出契约与冲突处理规则

**适用场景：** “读本地优先，云端兜底”或“低延迟优先”的能力。

### 4.4 MCP Tools

**定义：** 通过 Model Context Protocol 接入第三方工具服务。  
**特点：**
- 标准协议化接入，降低集成成本
- 支持外部工具生态扩展
- 需处理远端信任、稳定性与配额隔离

**适用场景：** 快速接入外部能力、组织内部工具平台互联。

---

## 5. Tool 接口设计（Domain 语义）

> 不给代码，仅定义语义契约。

### 5.1 核心方法（自然语言）

1. **`identity()`**：返回工具稳定标识（name/version/provider）。
2. **`metadata()`**：返回工具声明信息（描述、参数 schema、执行目标、并行安全等）。
3. **`validate(input)`**：执行 schema 与业务规则校验，返回结构化错误。
4. **`authorize(context, input)`**：基于租户/用户/Agent/权限策略判断是否允许执行。
5. **`execute(request, runtime_ctx)`**：实际执行工具调用，支持同步结果与流式进度。
6. **`cancel(call_id)`**（可选）：支持中止长任务。
7. **`health()`**（可选）：给注册中心和路由器用于健康探测。

### 5.2 工具元数据模型（关键字段）

- `name`（全局唯一，建议命名空间化，如 `core.search.web`）
- `version`（语义化版本）
- `description`（给模型与人类的行为说明）
- `input_schema`（JSON Schema）
- `output_schema`（JSON Schema，可选但建议）
- `execution_target`（Server/Client/Hybrid/MCP）
- `parallel_safe`（是否允许并行）
- `idempotent`（是否幂等，影响重试策略）
- `timeout_ms_default` / `timeout_ms_max`
- `risk_level`（low/medium/high）
- `confirmation_required`（是否需二次确认）
- `required_capabilities`（如 `calendar.read`, `files.write`）
- `scope`（Global/User/Agent）
- `enabled`（启用状态）

### 5.3 统一输入输出格式

**ToolCall（输入）**
- `call_id`：调用唯一 ID（与消息模型一致）
- `tool_name` / `tool_version`
- `arguments`：JSON object
- `invocation_context`：tenant_id/user_id/agent_id/session_id/trace_id
- `deadline`：绝对超时时间

**ToolResult（输出）**
- `call_id`
- `status`：`success | error | timeout | denied | canceled`
- `output`：结构化 JSON（成功时）
- `error`：`code/message/retriable/details`（失败时）
- `latency_ms`
- `artifacts`（可选，多模态结果引用）
- `progress_events`（可选，供流式 UI）

---

## 6. ToolRegistry（注册与发现）

### 6.1 注册机制

- **静态注册（MVP）**：服务启动时装配内置工具。
- **动态注册（演进）**：
  - 租户安装/卸载工具包
  - 用户授权第三方连接器后注册用户级工具实例
  - MCP endpoint 绑定后生成适配工具

### 6.2 发现机制

运行时通过以下过滤链获取“当前可用工具”：
1. 作用域匹配（Global/User/Agent）
2. 启用状态（系统开关、租户开关、用户开关）
3. 权限与授权（capabilities + policy）
4. 上下文约束（客户端是否在线、地域/合规限制、健康状态）

### 6.3 作用域模型

- **Global Tool**：平台级公共工具（如天气）。
- **User Tool**：用户绑定后的私有工具（如个人日历）。
- **Agent Tool**：仅某个 Agent persona/配置可见的工具集合。

**优先级建议：** Agent 覆盖 User 覆盖 Global（同名冲突时要求显式命名空间，避免隐式覆盖）。

### 6.4 启用/禁用

- 三层开关：`system_default`、`tenant_override`、`user_override`
- 支持“软禁用”（不对 LLM 声明）和“硬禁用”（拒绝执行）
- 禁用变更应审计并可回滚

---

## 7. 工具执行流程

## 7.1 Server-side 执行流程

1. 接收 Runtime 调用请求（含 call_id）。
2. 发射 `ToolCallStart`。
3. 参数校验 + 权限校验 + 风险策略检查。
4. 进入执行器（带超时控制与并行调度）。
5. 收集输出，规范化为 `ToolResult`。
6. 发射 `ToolCallResult`，写入 `messages.tool_result`，返回 Runtime。

## 7.2 Client-side 执行流程（gRPC）

1. 服务端完成校验后生成 `ClientToolRequest` 事件。
2. 通过 gRPC 流下发至目标客户端（按 user/session/device 路由）。
3. 客户端本地执行工具并回传 `ClientToolResponse`。
4. 服务端进行响应验签/关联（call_id 对齐）与 schema 校验。
5. 规范化为 `ToolResult`，发射 `ToolCallResult` 并回填 Runtime。

**离线处理：** 客户端不在线时根据策略返回 `timeout/unavailable`，或 Hybrid 回退到 server 执行。

## 7.3 超时、重试、隔离

- **超时**：默认 10s（与 Runtime 配置一致），工具可声明更小默认值；上限受平台策略约束。
- **重试**：仅对 `idempotent=true` 且 `retriable=true` 错误自动重试（指数退避 + 抖动，有限次数）。
- **并行**：仅 `parallel_safe=true` 工具进入并行批执行。
- **隔离**：
  - 外部调用在 adapter 隔离边界内执行（网络/凭证最小权限）
  - 高风险工具可要求沙盒执行（进程级/容器级）

## 7.4 与 Runtime 工具循环对接

- 完全复用已定循环：LLM tool_call → ToolRuntimePort → tool_result 回填。
- 错误不抛出到会话外层，统一作为 `tool_result.error` 供模型决策。
- 循环次数与单次超时沿用 Runtime 配置（默认 8 次、10s）。

---

## 8. 权限与安全

### 8.1 可用性与权限模型

- **RBAC + Capability** 组合：
  - RBAC 决定“谁可使用哪类工具”
  - Capability 决定“可执行到什么粒度”（read/write/delete）
- 权限决策输入：tenant、user、agent、tool、action、resource。

### 8.2 参数验证

- 双层校验：
  1. JSON Schema（结构、类型、枚举、范围）
  2. 业务规则（白名单域名、路径限制、最大数量等）
- 校验失败返回结构化错误码（如 `INVALID_ARGUMENT`），禁止隐式容错。

### 8.3 危险操作二次确认

对 `risk_level=high` 或 `confirmation_required=true` 的工具：
- 进入确认门（human-in-the-loop）
- 未确认前不执行；超时则返回 `denied/expired`
- 审计记录：请求参数摘要、确认人、确认时间、执行结果

### 8.4 审计与合规

- 记录最小必要审计日志：谁在何时调用了什么工具、结果状态、耗时、错误码。
- 对敏感输出做脱敏存储策略（token、PII、密钥）。

---

## 9. MCP 接入层设计

### 9.1 MCP 简介（面向本系统）

MCP 提供标准化的模型工具/资源交互协议，使平台可统一接入外部工具提供方，减少定制集成成本。

### 9.2 MCP 到 Tool trait 的适配

引入 **MCP Adapter**，将 MCP tool 描述映射到平台 `Tool` 语义：
- MCP `tool.name/description/inputSchema` → 平台 `metadata`
- MCP 调用请求 → `ToolCall.arguments`
- MCP 响应/错误 → `ToolResult.output/error`
- MCP 会话与连接状态 → `health()` 与可用性过滤

**关键点：**
- 保持 call_id 贯通，便于 trace 与审计
- 对远端 schema 做本地缓存与版本锁定
- 在授权层增加“第三方连接状态”检查

### 9.3 MCP 扩展方向

- 多 MCP provider 路由与优先级
- MCP 工具市场（租户可安装）
- 远端能力探测与自动降级
- 统一配额与成本治理（per tenant / per tool）

---

## 10. MVP 范围与演进路线

### 10.1 MVP（阶段 1）

**实现：**
- 仅 Server-side tools
- 工具：`web_search`、`weather_current`、`time_now`（示例）
- `Tool` 抽象 + `ToolRegistry` + `ToolRuntimePort` 对接
- 基础权限（RBAC + capability）
- 超时/并行/错误规范化
- ChatEvent：`ToolCallStart` / `ToolCallResult`

**简化：**
- 不做 Client-side 执行链路
- 不做 MCP 接入
- 不做复杂流式进度与取消（可预留字段）

### 10.2 阶段 2（Client 能力）

- 增加 `ClientToolRequest/Response` 全链路
- 上线客户端工具（相册、日历只读）
- 引入在线状态感知与离线回退策略

### 10.3 阶段 3（Hybrid + 安全增强）

- Hybrid 路由策略引擎（延迟/可用性/隐私优先）
- 高风险工具二次确认工作流
- 更细粒度审计与策略中心

### 10.4 阶段 4（MCP 生态）

- MCP adapter 正式接入
- 租户级 MCP 工具安装与管理
- 配额、成本、SLO、熔断治理

---

## 11. 非功能要求（建议基线）

- **可观测性**：trace_id 贯穿 tool_call、event、db 记录。
- **可靠性**：工具执行失败不破坏主会话，错误可回填可解释。
- **可扩展性**：Tool trait 稳定，adapter 可插拔。
- **多租户隔离**：凭证、配额、日志、策略按 tenant 隔离。

---

## 12. 与验收标准对照

- [x] 工具生命周期完整（第 3 节）
- [x] 4 种工具类型描述（第 4 节）
- [x] Tool trait 接口完整（第 5 节）
- [x] ToolRegistry 注册/发现/作用域（第 6 节）
- [x] Server/Client 执行流程覆盖（第 7 节）
- [x] 权限与安全策略明确（第 8 节）
- [x] 与 Runtime 与 ChatEvent 对接一致（第 2/7 节）
- [x] MVP 与演进路线明确（第 10 节）

---

## 13. 结论

该设计以 Domain 抽象稳定性为核心，在 MVP 阶段保持 YAGNI（先做 server-side），同时通过 `ExecutionTarget`、统一 I/O 契约、策略化注册/发现/执行/授权，预留 Client 与 MCP 的无缝演进路径，满足 SaaS 多租户平台的可治理、可扩展与安全要求。