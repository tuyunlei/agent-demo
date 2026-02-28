# ZeroClaw Agent Loop 架构调研

## Scope & reading notes

本调研聚焦 **Agent Loop 的执行模型**（生命周期、触发事件、并发、状态），不展开每个工具/provider 的细节实现。

阅读文件：
- `src/agent/agent.rs`（完整）
- `src/agent/dispatcher.rs`（完整）
- `src/agent/loop_.rs`（前 200 行 + 关键函数/loop/事件路径）
- `src/agent/mod.rs`
- `src/agent/memory_loader.rs`
- `src/agent/classifier.rs`

---

## TL;DR

ZeroClaw 的“Agent Loop”本质是 **每个 turn 内部的工具调用迭代循环**（LLM→解析tool call→执行tool→把结果回填history→再问LLM），而不是一个永不退出的全局守护循环。核心函数是 `run_tool_call_loop(...)`（`loop_.rs:2067`）。

外层“持续运行”来自 **宿主通道/CLI 驱动**：
- CLI 交互模式自己 `loop { read_line ... }`（`loop_.rs:3053`）或 channel recv 循环（`agent.rs:582`）
- 频道/daemon 场景通常是一次消息触发一次 `process_message(...)`（`loop_.rs:3215`）

所以是：**事件驱动 + turn 内循环**，不是单一中心事件总线常驻 agent actor。

---

## 1) What is the Agent Loop?

### 1.1 核心循环定义
`run_tool_call_loop` 的注释直接定义了循环不变量和退出条件（`loop_.rs:2052-2062`）：
- 迭代体：发送会话给 LLM，解析工具调用，执行工具，写回 history，继续
- 退出：
  1. 无 tool calls（得到最终文本响应）
  2. 达到 `max_iterations`
  3. cancellation token 被触发

主循环是 `for iteration in 0..max_iterations`（`loop_.rs:2100`），而不是无限 `loop`。

### 1.2 它如何“活着”
- 在 **单次 turn** 维度：`run_tool_call_loop` 运行到收敛或失败即返回（`loop_.rs:2337-2380`, `2674-2687`）。
- 在 **会话维度**：由外层入口反复调用 turn：
  - `agent.rs` 的 `run_interactive` 在 `while let Some(msg) = rx.recv().await` 中调用 `self.turn(...)`（`agent.rs:582-584`）。
  - `loop_.rs` 的 CLI 模式在 `loop {}` 中每次输入调用 `run_tool_call_loop(...)`（`loop_.rs:3053`, `3145`）。

结论：这是 **分层 loop**：
- 外层 input loop（可有可无，取决于入口）
- 内层 tool loop（每 turn 必经）

---

## 2) What events trigger the agent?

从 `agent` 模块本身看，触发源主要是“有消息进入入口函数”：

1. **CLI 输入**
   - `stdin.read_line` 触发一轮（`loop_.rs:3058-3069`）
2. **Channel 消息**
   - `Channel::listen` -> `rx.recv` -> `turn`（`agent.rs:578-584`）
   - channel 侧可直接调用 `process_message(config, message)`（`loop_.rs:3215`）
3. **单次命令调用**
   - `run(..., message: Some(...))` 直接执行一轮（`agent.rs:634-636`; `loop_.rs:3023-3042`）

不是在该模块内直接实现的触发：
- 定时器/cron/webhook 监听器不在 agent loop 核心里；更像是外部系统触发后把消息喂给 `process_message`。
- 虽然工具列表包含 `cron_*`/scheduler 类工具（`loop_.rs:2877-2917`），但这是 **模型可调用能力**，不是 agent 自己的唤醒机制。

---

## 3) How does a "turn" relate to the "loop"?

### 在 `agent.rs` 路径
- `turn(user_message)` 是一轮完整对话处理入口（`agent.rs:467`）。
- turn 内部包含 `for _ in 0..max_tool_iterations`（`agent.rs:501`），即 turn 内 loop。

### 在 `loop_.rs` 路径
- `agent_turn(...)` 只是薄封装，直接调用 `run_tool_call_loop(...)`（`loop_.rs:1865-1895`）。
- 所以 `run_tool_call_loop` 才是真正统一的 turn 执行器。

结论：
- **Turn 是“单次输入到单次最终输出”的事务边界**。
- **Loop 是 turn 内部求解过程**（可多轮 tool use）。

---

## 4) How does the dispatcher work?

注意：这里的 `dispatcher.rs` 不是“事件总线 dispatcher”，而是 **tool I/O 协议适配器**。

`ToolDispatcher` trait（`dispatcher.rs:21-27`）负责：
1. `parse_response`: 从 LLM 响应提取工具调用
2. `format_results`: 把工具执行结果编码回会话消息
3. `to_provider_messages`: 将内部 `ConversationMessage` 转换为 provider 请求消息
4. `prompt_instructions`: 非 native 模式下注入 tool protocol 文本
5. `should_send_tool_specs`: 是否将结构化 tool specs 发给 provider

两种实现：
- **XmlToolDispatcher**（文本协议）
  - 从 `<tool_call>...</tool_call>` 中解析 JSON（`dispatcher.rs:33-82`）
  - 工具结果以 `[Tool results]` 文本块回填（`dispatcher.rs:95-106`, `139-149`）
- **NativeToolDispatcher**（provider 原生 tool calling）
  - 从 `response.tool_calls` 读取结构化调用（`dispatcher.rs:162-180`）
  - 结果回填为 `ConversationMessage::ToolResults` / role=tool 消息（`dispatcher.rs:183-195`, `220-231`）

`Agent::from_config` 根据配置或 provider capability 选择 dispatcher（`agent.rs:305-311`）。

---

## 5) Concurrency model

### 5.1 每 turn 的并发
- Agent tool loop 默认按顺序迭代 LLM 回合。
- 单回合内多个 tool calls 可并行执行：
  - `agent.rs` 路径：`config.parallel_tools` 控制 `join_all`（`agent.rs:427-440`）。
  - `loop_.rs` 路径：根据 `should_execute_tools_in_parallel(...)` 选择 parallel/sequential（`loop_.rs:2397`, `2562-2578`）。

### 5.2 每会话/每实例
- `Agent` 对象持有 `history`（`agent.rs:36`），因此一个 Agent 实例对应一个会话上下文容器。
- `run_interactive` 是单循环消费消息（`agent.rs:582`），同一实例内 turn 串行处理。
- 是否多会话并行由更上层 runtime/channel 管理；本模块不展示“每租户一个loop actor”这类中心并发调度。

### 5.3 取消与中断
- tool loop 接受 `CancellationToken`（`loop_.rs:2080`），在 LLM call 与 streaming/tool 执行处检查（如 `2101-2106`, `2176-2180`, `2361-2366`）。

---

## 6) Proactive behavior (without user input?)

从 agent loop 核心代码看：
- **不具备内建“自主苏醒”定时循环**。
- 需要外部触发（消息、CLI 输入、外部调用）。
- 能做“看起来主动”的事情主要是通过已触发 turn 中调用 scheduler/cron 工具去安排未来任务（工具能力），不是 loop 自身后台主动运行。

所以：**Proactive 是外部系统+工具层能力，不是 Agent Loop 内核常驻自治。**

---

## 7) State model (what persists across iterations?)

### 7.1 turn 内状态（短时）
`run_tool_call_loop` 内：
- `history`（可变引用）持续增长（`loop_.rs:2069`, `2643-2670`）
- `seen_tool_signatures` 用于同 turn 去重（`loop_.rs:2098`, `2494-2525`）
- `turn_id` 用于观测追踪（`loop_.rs:2097`）

### 7.2 session 内状态（跨 turn）
- `agent.rs` 的 `self.history` 是 Agent 实例字段（`agent.rs:36`），跨 turn 保留，支持连续会话。
- 有 `trim_history` 控制窗口（`agent.rs:348-373`；`loop_.rs:146-164` 也有类似逻辑）。
- `loop_.rs` CLI interactive 也显式维持 `history` 变量（`loop_.rs:3051`）。

### 7.3 长期状态（跨进程）
- Memory 通过 `MemoryLoader` 在每 turn 前召回注入（`memory_loader.rs:6-8`, `35-66`; `agent.rs:483-487`; `loop_.rs:3003-3016`）。
- 可选 auto-save 用户输入到 memory（`agent.rs:476-481`; `loop_.rs:3119-3124`）。
- 这部分是“外部持久化状态”，不依赖进程内 history。

---

## Memory loader / classifier 角色补充

### Memory Loader
`DefaultMemoryLoader`：
- recall top-k（默认 5）+ relevance 阈值过滤（默认 0.4）
- 生成 `[Memory context]` 前缀块注入用户消息前
- 跳过 assistant autosave 旧键，避免污染（`memory_loader.rs:41-57`, `48-54`）

### Classifier
`classify_with_decision`：
- 基于规则（关键词/模式、长度约束、priority）命中 hint（`classifier.rs:20-66`）
- `Agent::classify_model` 将命中的 hint 转为 `hint:<name>` 路由给 provider/router（`agent.rs:443-465`）

这不是“是否进入 loop”的前置门控，而是 **进入 loop 后选择模型路由**。

---

## 与 agent-demo TurnExecutor 的简短对比

基于 `agent-demo` 现有文档（如 `docs/archive/design/decisions/005-agent-runtime.md`、`docs/archive/design/orchestration/turn-executor.md`）可以做如下映射：

1. **共同点**
- 都是“单次 turn 编排器 + turn 内 tool loop 上限保护”。
- 都把 LLM 调用、tool 执行、结果回填作为主路径。

2. **ZeroClaw 当前形态**
- 更偏“运行时代码内聚”：prompt 构建、memory 注入、tool protocol、循环执行都在 agent 层直接串起。
- `run_tool_call_loop` 接口参数很多（`loop_.rs:2067-2084`），体现了较强 runtime 组装特征。

3. **agent-demo TurnExecutor 目标形态**
- 更强调分层编排（TurnExecutor 只管状态机，ContextBuilder/LlmProvider/ToolRuntime 解耦）。
- 在架构边界与可测试性上更“显式模块化”。

可把 ZeroClaw 的 `run_tool_call_loop` 看作一个“功能完备但内聚较高”的 TurnExecutor 原型。

---

## Unclear / caveats

1. 本次未展开 `channels/*`、`runtime/*`、`daemon` 入口，因此“外部事件源全景（webhook/queue/timer）”无法在本文件组内完全确认。
2. `src/agent/dispatcher.rs` 的“dispatcher”是 tool 协议分发器，不是系统事件 dispatcher；命名上容易误解。
3. `agent.rs` 与 `loop_.rs` 存在功能重叠（两套 turn/tool-loop 路径），可能是历史演进或不同入口并存，后续若要统一建议再追一次调用图。
