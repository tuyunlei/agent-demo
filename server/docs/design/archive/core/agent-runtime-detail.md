# Agent Runtime 内部拆分设计（design/core/agent-runtime-detail.md）

> 本文是 `agent-runtime.md` 的内部补充：只展开 Runtime 内部树形结构、模块边界与并发模型。
> 不重复总览中的 8 步主流程、不展开 context 算法/LLM gateway/tool system 内部。

---

## 1. 设计目标

在 Application Layer 内，将 Agent Runtime 拆为可独立演进的子模块，并满足：

1. 保持 **session actor** 作为唯一并发边界（S03）
2. Runtime 内部编排逻辑可测试、可替换（基于 Port）
3. 流式事件顺序稳定，职责归属清晰

---

## 2. 树形模块拆分（3+ 层）

```text
L1 AgentRuntime (Application, actor 边界入口)
├── SessionRegistry
│   └── SessionHandle
├── RequestDispatcher
└── SessionActor (per session task)
    ├── ContextAssembler
    ├── ExecutionLoop
    │   ├── LlmCaller
    │   ├── ToolCoordinator
    │   └── EventPublisher
    └── PostProcessor
```

说明：
- L1 是 Runtime 作为应用编排总入口。
- L2 里既包含系统级模块（SessionRegistry / RequestDispatcher），也包含会话内模块（ContextAssembler / ExecutionLoop / PostProcessor）。
- L3 仅对 ExecutionLoop 继续下钻，定义其最小稳定内部边界。

---

## 3. 子模块职责、输入输出与 Port 依赖

## 3.1 L1/L2 模块

### 3.1.1 AgentRuntime（L1）
- **一句话职责**：提供“处理一条入站消息”的用例入口，协调 Dispatcher 与 Session 生命周期。
- **输入**：InboundMessage（user_id / agent_id / session_id / request_id / payload）。
- **输出**：本次 turn 的处理结果（流式事件由下游模块推送）。
- **依赖 Port**：无直接业务 Port；通过组合根注入 L2 模块。

### 3.1.2 SessionRegistry（L2）
- **一句话职责**：管理所有 session actor 的创建、复用、退出与代际（generation）安全。
- **输入**：session_id + 派发命令（process/shutdown）。
- **输出**：SessionHandle（可发送命令到 mailbox）。
- **依赖 Port**：无业务 Port；依赖运行时并发原语（mpsc、task）。

### 3.1.3 SessionHandle（L3 for Registry）
- **一句话职责**：对外暴露向目标 session actor 投递命令的稳定句柄。
- **输入**：ProcessMessage / SubmitToolResult / Shutdown。
- **输出**：投递成功/失败、或 await 处理结果。
- **依赖 Port**：无（仅 channel 通信）。

### 3.1.4 RequestDispatcher（L2）
- **一句话职责**：接收 InboundMessage，定位 session actor 并投递命令，处理背压策略。
- **输入**：InboundMessage。
- **输出**：消息已入队、或 fast-fail/timeout。
- **依赖 Port**：无业务 Port；依赖 SessionRegistry。

### 3.1.5 SessionActor（L2 容器）
- **一句话职责**：单 session 串行执行 turn 生命周期，封装会话内状态机。
- **输入**：mailbox 命令流（bounded mpsc）。
- **输出**：状态更新、副作用调用（LLM/存储/事件）。
- **依赖 Port**：通过内部子模块间接依赖各业务 Port。

### 3.1.6 ContextAssembler（L2）
- **一句话职责**：将会话历史、人格与记忆摘要组装为 LLM 可消费上下文。
- **输入**：当前用户消息 + session 标识。
- **输出**：LlmInputContext。
- **依赖 Port**：MessageStore、PersonaStore、MemoryStore。

### 3.1.7 ExecutionLoop（L2）
- **一句话职责**：执行“LLM -> 工具 -> 再 LLM”的主循环，直到产出最终回复或错误终止。
- **输入**：LlmInputContext + 运行时约束（max rounds/timeout）。
- **输出**：最终 assistant 结果、工具交互轨迹、事件流。
- **依赖 Port**：LlmProvider、ToolRuntime（经 ToolCoordinator）、ChannelAdapter（经 EventPublisher）。

### 3.1.8 PostProcessor（L2）
- **一句话职责**：turn 完成后的收尾：持久化与记忆摘要更新。
- **输入**：本轮执行结果（assistant 文本、tool call/result、usage、状态）。
- **输出**：持久化成功/失败、异步后处理触发结果。
- **依赖 Port**：MessageStore、MemoryStore（必要时 SessionStore）。

---

## 3.2 ExecutionLoop 内部（L3）

### 3.2.1 LlmCaller
- **一句话职责**：封装对 LlmProvider 的调用与流式 token 消费。
- **输入**：LlmInputContext。
- **输出**：TextDelta 流、ToolCall 列表、FinalText、Provider 元数据（usage/latency）。
- **依赖 Port**：LlmProvider。

### 3.2.2 ToolCoordinator
- **一句话职责**：协调工具执行请求与结果回流（包含 client tool 等待）。
- **输入**：ToolCall 列表。
- **输出**：ToolResult 列表（成功/失败都结构化返回）。
- **依赖 Port**：ToolRuntime；内部机制为 pending map + oneshot。

### 3.2.3 EventPublisher
- **一句话职责**：把内部运行事件标准化为 AgentEvent，并推送到 ChannelAdapter。
- **输入**：TextDelta / ToolRequest / ToolResult / RoundComplete / Error。
- **输出**：有序 AgentEvent 流。
- **依赖 Port**：ChannelAdapter（事件出口）。

---

## 4. 模块关系图（ASCII）

```text
InboundMessage
    |
    v
+---------------------+
|  RequestDispatcher  |
+----------+----------+
           |
           v   (find-or-create)
+---------------------+
|   SessionRegistry   |
+----------+----------+
           |
           v  SessionHandle.send(cmd)
+--------------------------------------------------+
| SessionActor (single session, serial execution)  |
|                                                  |
|  +------------------+                            |
|  | ContextAssembler | --(LlmInputContext)----+   |
|  +------------------+                        |   |
|                                              v   |
|  +----------------------------------------------+|
|  |                ExecutionLoop                 ||
|  | +-----------+  +----------------+  +-------+ ||
|  | | LlmCaller |->| ToolCoordinator|->|  ...  | ||
|  | +-----+-----+  +--------+-------+  +-------+ ||
|  |       |                 |                   | ||
|  |       +------event------+-------------------+ ||
|  |                         v                     ||
|  |                 +---------------+             ||
|  |                 |EventPublisher |-------------++--> ChannelAdapter
|  |                 +---------------+             ||
|  +----------------------------------------------+|
|                         |
|                         v
|                 +---------------+
|                 | PostProcessor |
|                 +---------------+
|                     |        |
|                     v        v
|               MessageStore  MemoryStore
+--------------------------------------------------+
```

---

## 5. 并发模型（落实 S03 结论）

## 5.1 并发边界
- **边界单位**：`session_id`
- **模型**：每个 session 一个 actor task
- **mailbox**：bounded mpsc（容量默认 8）

## 5.2 执行语义
- **actor 内部严格串行**：
  1) 状态/上下文读取  
  2) LLM 调用与工具循环  
  3) 持久化写入  
  4) 事件推送/结束
- **跨 session 并发**：不同 session actor 可并发运行，不共享可变状态。

## 5.3 生命周期与防竞态
- SessionRegistry 负责 actor 生命周期与 idle timeout 自动退出。
- 使用 generation id 防止“旧 handle 误投递到新 actor”的竞态。

## 5.4 背压策略
- mailbox 满时默认 `await`（上游感知排队）。
- 可配置策略：
  - fast-fail（立即返回拥塞错误）
  - timeout（等待指定时长后失败）

---

## 6. 边界约束（Hexagonal）

1. SessionActor 与其子模块位于 Application Layer。  
2. 不直接依赖任何 Infrastructure 实现。  
3. 所有外部能力（LLM/工具/存储/事件）必须走 Port。  
4. 具体 Adapter 仅在组合根装配，不渗透到 Runtime 内部。

---

## 7. 与总览文档的衔接

- `agent-runtime.md` 负责说明“做什么”（端到端流程与失败语义）。
- 本文负责说明“在 Runtime 内部由谁做”（模块职责与边界）。
- D05 再把这里的输入输出进一步收敛为精确 Port/DTO 签名。

---

## 8. 关键结论

1. Runtime 内部主干可稳定拆为：`RequestDispatcher + SessionActor(ContextAssembler, ExecutionLoop, PostProcessor)`。  
2. ExecutionLoop 继续下钻为：`LlmCaller + ToolCoordinator + EventPublisher`。  
3. **负责把 AgentEvent 推给 ChannelAdapter 的模块是：`EventPublisher`。**
