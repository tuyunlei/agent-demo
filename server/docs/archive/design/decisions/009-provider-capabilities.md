# ADR-009: Provider 高阶能力建模（Stateful / Tool Execution Class / Compaction 分层）

- **状态**: Accepted
- **日期**: 2026-02-28
- **决策者**: agent-demo server 设计
- **相关文档**:
  - `docs/design/capabilities/llm-provider.md`
  - `docs/design/capabilities/tool-system.md`
  - `docs/design/capabilities/context-builder.md`

## 背景

随着 provider 能力演进，出现了三类新需求：

1. 部分 provider 支持 stateful 会话续接（`previous_response_id`）
2. 工具执行位置不再只有 server 本地执行（provider 内置/远端/客户端）
3. 长上下文压缩既可能由 provider 原生提供，也可能由服务端自建

目标是在不破坏现有分层和 trait 稳定性的前提下，给出可扩展的统一建模。

## 决策 1：Stateful 采用统一接口 + 可选字段（而非多方法）

### 方案

在统一 `LlmRequest` 上新增可选字段：

- `previous_response_id: Option<String>`
- `builtin_tools: Vec<String>`（同属 provider 高阶能力请求面）

在 provider 能力声明中新增：

- `LlmProviderCapabilities.supports_stateful: bool`

### 理由

- 避免 trait 膨胀（不引入 `complete_stateful` / `complete_stateless` 双方法）
- 编排层始终走同一调用路径，减少分支复杂度
- 对不支持 stateful 的 provider 保持零成本兼容（字段忽略即可）

## 决策 2：工具执行位置不在 trait 层建模流程分支，只增加元数据

### 方案

在 `agent-tools::ToolSpec` 增加：

- `execution_class: ExecutionClass`
- `ExecutionClass = { Local, Remote, ProviderBuiltin, Client }`

其中 `execution_class` 作为元数据用于审计/策略/观测，不作为编排主流程分支条件。

### 理由

- 工具执行位置属于实现细节与部署细节，强绑定到 trait 会导致接口过早固化
- 当前编排主流程仍可保持统一 tool loop
- 通过元数据先满足可见性与策略需求，保留后续演进空间

## 决策 3：Compaction 支持分层委托（ProviderCompaction / SelfCompaction）

### 方案

`CompactionService` 允许两类实现：

1. `ProviderCompaction`：委托 provider 原生压缩
2. `SelfCompaction`：服务端自建 summary

同一 turn 互斥，仅走一条路径。Summary event 增加 `source` 区分来源（`provider` / `self`）。

### 理由

- 兼容 provider 能力差异，支持渐进增强
- 保持上层 ContextBuilder 消费统一事件模型
- 有利于审计与回放（来源可追踪）

## 影响与后果

### 正向

- 保持 `LlmProvider` 接口稳定，同时支持 stateful 能力扩展
- 工具系统获得执行位置可观测元数据，不破坏现有编排路径
- compaction 架构可按 provider 能力逐步落地

### 代价

- 编排层需要维护 stateful->stateless 回退策略
- `execution_class` 初期是“声明性字段”，短期不直接驱动执行逻辑
- Summary event schema 后续需要补充 `source` 字段并同步实现

## 备选方案（未采纳）

1. **为 stateful 新增独立 trait 方法**：接口复杂度上升，调用方分支增多，未采纳。
2. **在 ToolRuntime/Trait 层强制建模执行位置分派**：过早绑定实现细节，未采纳。
3. **仅保留自建 compaction**：无法利用 provider 原生能力，未采纳。
