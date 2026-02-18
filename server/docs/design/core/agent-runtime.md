# Agent Runtime 详细设计（design/core/agent-runtime.md）

## 1. 目标与范围

本文定义 Agent Runtime 在多租户 SaaS AI Agent 平台中的详细设计。Runtime 负责**请求编排**：接收消息、构建上下文、调用 LLM、驱动工具循环、流式输出、持久化与异步后处理。

> 与 ADR-005/006 对齐：
> - 主流程骨架保持一致
> - 流式事件使用 `ChatEvent`（`TextDelta` / `ToolCallStart` / `ToolCallResult` / `ClientToolRequest` / `Done` / `Error`）
> - 数据持久化遵循 data-model.md 中 Repository Port 与 `user_id` 租户隔离

**非目标（不在本文展开）**
- Context 组装算法细节（见 `context-management.md`）
- LLM Gateway 内部实现（见 `llm-gateway/overview.md`）
- Tool Runtime 内部实现（见 `tool-system/overview.md`）
- 任何 Rust 代码实现细节

---

## 2. Runtime 在六边形架构中的位置

### 2.1 分层定位

Agent Runtime 属于 **Application Layer（应用编排层）**，是用例级 Orchestrator：
- 负责跨 Port 的流程控制与状态机推进
- 不关心底层协议（gRPC/HTTP）、数据库（sqlx/Postgres）或具体 SDK
- 依赖抽象 Port（trait）而非具体 Adapter

### 2.2 与上下游关系

```text
                    +-----------------------+
                    |      API Gateway      |
                    | (gRPC/HTTP Streaming) |
                    +-----------+-----------+
                                |
                                v
+-------------------------------------------------------------+
|                  Application: Agent Runtime                 |
|  - request orchestration                                    |
|  - tool loop state machine                                  |
|  - streaming event ordering                                 |
|  - transaction boundary coordination                        |
+-----------+-------------------+------------------+----------+
            |                   |                  |
            v                   v                  v
    +---------------+   +---------------+   +------------------+
    |  LLM Gateway  |   | Tool Runtime  |   |    Data Layer    |
    |  (Port)       |   | (Port)        |   | Repositories     |
    +---------------+   +---------------+   +------------------+
```

### 2.3 调用方向

- **被调用方**：API Gateway 调用 `AgentRuntime` 发起一次对话回合（turn）
- **依赖方**：Runtime 调用 LLM Gateway、Tool Runtime、Data Repositories、异步任务调度 Port
- **原则**：单向依赖（Application -> Ports），Adapter 反向实现 Port

---

## 3. 核心接口设计（自然语言）

## 3.1 AgentRuntime 主方法

### 方法 A：发起对话（流式）
- 语义：处理单次用户输入并返回 `ChatEvent` 流
- 输入：
  - `user_id`（租户隔离主键）
  - `agent_id`
  - `session_id`
  - `user_message`（文本/附件引用/客户端能力）
  - `request_id`（幂等与追踪）
  - `runtime_options`（超时、最大工具轮次、是否允许并行工具等）
- 输出：
  - 有序 `ChatEvent` 流（直到 `Done` 或 `Error` 结束）

### 方法 B：异步后处理入口
- 语义：对已完成 turn 进行非阻塞后处理
- 输入：`turn_id`、`user_id`、`session_id`、运行摘要（token、时延、工具统计）
- 输出：成功/失败（失败只记录，不影响已返回给用户的主链路）

## 3.2 Runtime 依赖的 Port trait 列表

1. **UserRepository**：用户存在性与状态校验（租户合法性）
2. **AgentRepository**：Agent 配置、persona、工具白名单、模型策略
3. **SessionRepository**：会话读取、状态锁、版本检查、元数据更新
4. **MessageRepository**：消息读写（user/assistant/tool_call/tool_result/token）
5. **MemoryRepository**：长期记忆读取（用于上下文组装）
6. **ContextAssemblerPort**（应用内抽象）：把 system/persona/summary/history/tools 组装为 LLM 输入
7. **LlmGatewayPort**：模型调用（流式文本 + 工具调用结构）与 fallback 能力
8. **ToolRuntimePort**：工具执行（含 server tool/client tool）
9. **EventStreamPort**：向 API 层输出标准化 `ChatEvent`
10. **UnitOfWork/TransactionPort**：事务边界控制
11. **AsyncTaskPort**：后处理任务投递
12. **ObservabilityPort**：metrics/tracing/log/audit 统一埋点
13. **IdempotencyPort（可选）**：请求去重（`request_id`）

---

## 4. 详细流程设计（8 步）

## 4.1 ① 接收与验证

- **输入**：`user_id`, `agent_id`, `session_id`, `user_message`, `request_id`
- **处理**：
  1. 校验 user/agent/session 归属关系（都必须属于同一 `user_id`）
  2. 校验 session 可写状态（未关闭、未归档或可自动唤醒）
  3. 幂等检查：若 `request_id` 已完成，返回历史结果或拒绝重复
  4. 申请 session 写入控制（见并发安全）
- **输出**：已验证的 `RuntimeContextHandle`
- **依赖**：User/Agent/Session Repository, IdempotencyPort
- **错误处理**：
  - 鉴权/归属失败：`Error(unauthorized/not_found)`
  - session 冲突：`Error(conflict)` 或排队
  - 参数无效：`Error(validation_failed)`

## 4.2 ② 上下文组装（概述）

- **输入**：会话标识 + 当前用户消息
- **处理（仅编排，不展开算法）**：
  - 拉取 system prompt、persona、summary、最近 N 条消息、Memory 摘要、工具声明
  - 生成 LLM 请求上下文
- **输出**：`LlmInputContext`
- **依赖**：MessageRepository, MemoryRepository, AgentRepository, ContextAssemblerPort
- **错误处理**：
  - 上下文源读取失败：可降级（减少历史窗口）或返回 `Error(context_unavailable)`

## 4.3 ③ LLM 调用

- **输入**：`LlmInputContext`
- **处理**：
  - 调用主 provider（流式）
  - 若 provider 失败，按策略 fallback 到备用 provider
- **输出**（二选一）：
  1. `assistant_text_delta` 流 + 最终文本
  2. `tool_calls[]`
- **依赖**：LlmGatewayPort
- **错误处理**：
  - 单 provider 超时/错误 -> fallback
  - 全部失败 -> `Error(llm_unavailable)` 并结束

## 4.4 ④ 工具循环

- **输入**：`tool_calls[]` + 当前上下文
- **处理**：
  - 执行工具（串行或受限并行）
  - 每个结果回写为 `tool_result` 消息，再送回 LLM 下一轮
- **输出**：更新后的上下文，直到出现最终文本或触发终止条件
- **依赖**：ToolRuntimePort, MessageRepository（暂存）, LlmGatewayPort
- **错误处理**：
  - 工具失败不短路：封装错误为 `tool_result(error)` 回给 LLM
  - 循环超限/超时：`Error(tool_loop_limit_exceeded)`

## 4.5 ⑤ 最终回复处理

- **输入**：LLM 最终文本
- **处理**：
  - 聚合流式 delta 为最终 assistant message
  - 计算 token usage（若 provider 返回）
- **输出**：`AssistantFinalMessage`
- **依赖**：LlmGatewayPort（usage 元数据）
- **错误处理**：
  - 流中断且无法恢复：`Error(stream_interrupted)`

## 4.6 ⑥ 持久化策略（事务边界与顺序）

- **持久化对象**：
  - user message
  - assistant tool_call message（可多条）
  - tool_result message（可多条，含失败）
  - assistant final message
- **建议顺序（同一 turn）**：按生成先后顺序写入，保证回放一致性
- **事务策略**：
  - **主方案（MVP）**：turn 结束后单事务批量写入，保证原子性与顺序一致
  - **扩展方案**：长对话可分段提交（需 sequence_no 与幂等写保障）
- **依赖**：MessageRepository, SessionRepository, UnitOfWork
- **错误处理**：
  - 事务失败：返回 `Error(persistence_failed)`；不发送 `Done`
  - 若流已发送部分文本，客户端以 `Error` 收尾并可重试拉取历史

## 4.7 ⑦ 流式输出编排

- **目标**：事件顺序稳定、客户端可确定性渲染
- **发送原则**：
  - 文本增量 -> `TextDelta`
  - 工具开始 -> `ToolCallStart`
  - 工具完成 -> `ToolCallResult`（含错误）
  - 需要客户端执行工具 -> `ClientToolRequest`
  - 成功结束 -> `Done`
  - 失败结束 -> `Error`
- **依赖**：EventStreamPort
- **错误处理**：
  - 事件通道断开：记录并中止主链路，释放锁

## 4.8 ⑧ 异步后处理

- **典型任务**：
  1. session compaction 判定与触发
  2. session 元数据更新（last_active_at、turn_count、token 累计）
  3. 记忆抽取候选（写入待处理队列）
  4. 可观测性聚合上报（异步）
- **触发方式**：主流程成功后投递 `PostTurnJob`
- **失败策略**：
  - 不影响已完成响应
  - 重试（指数退避 + 最大重试次数）
  - 超限进入死信队列/告警

---

## 5. 工具循环详细设计

## 5.1 循环语义（while）

```text
initialize loop_count = 0
while loop_count < max_tool_rounds:
  llm_result = call_llm(context)
  if llm_result is final_text:
    return final_text
  if llm_result has tool_calls:
    execute tools (serial or bounded-parallel)
    append tool_call/tool_result into context
    loop_count += 1
    continue
return error(tool_loop_limit_exceeded)
```

## 5.2 终止条件（显式）

1. LLM 返回最终文本（无 tool_call）
2. 达到 `max_tool_rounds`（配置项）
3. 超过 `max_turn_wall_clock_timeout_ms`
4. 安全策略触发（禁用工具、参数违规）
5. 上下文超过 hard token limit 且裁剪后仍不可用

## 5.3 配置建议（MVP 默认值）

- `max_tool_rounds`: 8
- `single_tool_timeout_ms`: 10_000
- `max_parallel_tools_per_round`: 4
- `max_turn_wall_clock_timeout_ms`: 60_000
- `tool_retry_policy`: 默认不在 Runtime 侧重试，由 LLM 决策下一步

## 5.4 单轮多工具并行策略

- 当 LLM 一次返回多个 `tool_call`：
  - 若工具声明 `parallel_safe=true`，可并行执行（受 `max_parallel_tools_per_round` 限制）
  - 否则按声明顺序串行执行
- 结果回填顺序：
  - 存储层使用稳定顺序（按 tool_call index）
  - 流式事件可按完成先后发出，但必须带 `tool_call_id` 关联

## 5.5 工具失败后的 LLM 重试语义

- Runtime 不做“静默重试工具”
- 失败统一转成 `tool_result`（结构化错误：code/message/retryable）交还 LLM
- LLM 可选择：
  1. 改参数再调工具
  2. 换工具路径
  3. 直接向用户解释失败并给替代方案

---

## 6. 并发安全设计

## 6.1 同一用户并发两条消息

- 允许同一 `user_id` 下不同 session 并行
- 对同一 `session_id` 启用**单写者模型**（single-writer per session）

## 6.2 同一 session 并发写保护

可选实现（推荐组合）：
1. **会话级租约锁**（Redis/DB advisory lock）
2. **乐观并发控制**（session version）
3. **消息序列号**（`sequence_no` 单调递增）

### 冲突处理策略
- 默认：后到请求返回 `Error(conflict)`（提示客户端重试）
- 可选：进入同 session FIFO 队列（牺牲时延换顺序保证）

### 幂等
- `request_id` + `session_id` 建唯一约束
- 重试请求可安全返回同一结果或明确“处理中”状态

---

## 7. 流式输出协议（ChatEvent 序列）

## 7.1 通用规则

- 一次 turn 只能以 `Done` 或 `Error` 之一结束（二选一）
- `Done`/`Error` 后不得再发送其他事件
- 每个事件携带：`request_id`, `session_id`, `turn_id`, `event_seq`

## 7.2 场景 A：无工具的正常文本回复

```text
TextDelta*
Done
```

## 7.3 场景 B：含工具调用

```text
ToolCallStart (for each tool_call)
[ClientToolRequest]* (仅客户端工具时)
ToolCallResult (for each tool_call, success or failure)
... (可能多轮)
TextDelta*
Done
```

## 7.4 场景 C：错误流程

### C1 LLM 全部失败
```text
Error
```

### C2 工具循环超限/超时
```text
ToolCallStart*
ToolCallResult*
Error
```

### C3 持久化失败
```text
(可能已有 TextDelta*)
Error
```

## 7.5 客户端判定结束

- 收到 `Done`：成功结束，可落 UI 最终态
- 收到 `Error`：失败结束，可展示错误并提供重试
- 若连接中断且无 `Done/Error`：视为未知状态，客户端应走“恢复查询/重放”流程

---

## 8. 可观测性设计

至少在以下节点埋点（建议 tracing span + metrics + structured log）：

1. **runtime.turn.total**：单 turn 端到端时延（p50/p95/p99）
2. **runtime.validation.fail.count**：鉴权/参数/冲突失败计数
3. **runtime.llm.call.latency** + **runtime.llm.fallback.count**
4. **runtime.tool.call.count / latency / error.count**（按 tool_name、error_code 维度）
5. **runtime.tool.loop.rounds**（分布，监控是否逼近上限）
6. **runtime.tokens.input/output/total**（按 model/provider/agent）
7. **runtime.persistence.latency / failure.count**
8. **runtime.stream.events.count**（按事件类型）
9. **runtime.postprocess.enqueue.fail.count** 与重试次数
10. **runtime.session.lock.wait_ms / conflict.count**（并发热点识别）

### 追踪上下文字段（建议）
- `request_id`, `turn_id`, `user_id`, `agent_id`, `session_id`
- `model_provider`, `model_name`, `tool_name`, `tool_call_id`
- `error_code`, `retryable`, `fallback_used`

---

## 9. 错误模型与对外语义

- 错误分层：
  1. `ValidationError`（输入/权限/状态）
  2. `DependencyError`（LLM/Tool/Repo 超时或不可用）
  3. `PolicyError`（安全限制、超限）
  4. `PersistenceError`
- 对客户端统一映射为 `ChatEvent::Error`，携带：
  - `code`（稳定错误码）
  - `message`（可读描述）
  - `retryable`（是否建议重试）

---

## 10. 配置与扩展点

## 10.1 必配项（显式配置）
- LLM 主/备 provider 顺序
- 工具循环上限与超时
- 并行工具开关与并发度
- session 并发策略（拒绝/排队）
- 后处理重试策略

## 10.2 MVP 与演进

- **MVP**：
  - 单 session 单写者
  - 工具失败不短路，交给 LLM 决策
  - turn 结束后一次性事务写入
- **后续扩展**：
  - 分段持久化（超长流）
  - 更细粒度中间态事件
  - 动态并行策略（按工具成本与依赖图）

---

## 11. 一致性检查（对 ADR 与数据模型）

- 与 ADR-005 一致：8 步流程完整保留并细化
- 与失败策略一致：
  - 工具失败 -> `tool_result(error)` 回给 LLM
  - LLM 失败 -> fallback -> 全失败报错
- 与 ADR-006 一致：仅使用既定 `ChatEvent` 类型
- 与 data-model 一致：
  - 使用 5 个 Repository Port
  - 所有读写显式携带 `user_id`
  - 消息包含 `role/content/tool_calls/tool_result/token_count`

---

## 12. 实施建议（落地顺序）

1. 先实现最小可用主链路：验证 -> context -> LLM -> Done/Error
2. 再接入单工具循环（串行）
3. 再加入并行工具与会话并发控制
4. 最后补齐后处理队列与全量观测指标

该顺序可在保证 MVP 可用的同时，逐步提升吞吐、稳定性与可运维性。