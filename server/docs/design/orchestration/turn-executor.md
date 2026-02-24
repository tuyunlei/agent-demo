# TurnExecutor 详细设计（D-ORCH-01）

**状态**：Draft（MVP 可实施）  
**更新时间**：2026-02-24  
**所属层**：Layer 2 Orchestration / `agent-orchestrator`  
**关联文档**：

- `docs/design/architecture.md`
- `docs/design/core/event-model.md`
- `docs/design/capabilities/llm-provider.md`
- `docs/design/capabilities/tool-system.md`
- `docs/design/capabilities/context-builder.md`
- `docs/design/capabilities/session-lifecycle.md`
- 参考：`~/code/references/zeroclaw/src/agent/agent.rs`
- 参考：`~/code/references/picoclaw/pkg/agent/loop.go`

---

## 1. 设计目标

### 1.1 目标总述

TurnExecutor 是 Layer 2 的单次 turn 编排器。

它负责把一次用户输入转成完整处理闭环：

1. 读取/创建会话
2. 记录事件
3. 组装上下文
4. 调用 LLM
5. 工具循环
6. 持久化与压缩
7. 产出响应

### 1.2 单一职责

TurnExecutor 只做流程编排。

TurnExecutor 不做：

1. 不自己拼 system prompt
2. 不自己实现工具调用细节
3. 不自己实现事件存储 SQL
4. 不自己实现 Session 状态机底层逻辑
5. 不感知 provider SDK 差异

### 1.3 依赖抽象

TurnExecutor 只能依赖 Layer 3 trait：

1. `ContextBuilder`
2. `LlmProvider`
3. `ToolRuntime`
4. `EventStore`
5. `SessionLifecycle`

TurnExecutor 禁止依赖 Layer 4 concrete type。

例如禁止：

- `PostgresEventStore`
- `OpenAiProviderAdapter`
- `RedisLockImpl`

### 1.4 可测试

TurnExecutor 所有依赖都可 mock。

最小可测场景：

1. 无工具直出
2. 单轮工具调用
3. 多轮工具调用
4. tool loop 超限
5. LLM length 截断
6. persist 失败降级
7. lifecycle/load 失败快速返回

### 1.5 工具循环有上限

工具循环使用显式上限：

`while iteration < max_tool_iterations`

必要性：

1. 避免无限循环
2. 控制 token 成本
3. 控制响应时延
4. 保障服务可预测性

### 1.6 与事件流模型一致

TurnExecutor 产生的核心事件：

1. `UserMessage`
2. `AssistantMessage`
3. `ToolCallRequest`
4. `ToolCallResult`
5. `SystemEvent`（TurnStarted / TurnCompleted / TurnFailed）

在触发压缩时还会协同产生：

6. `Summary`
7. `CompactionMarker`

### 1.7 MVP 目标边界

本设计优先支持同步 request/response turn。

流式响应、异步 tool callback、并发 fan-out 属于后续增强。

---

## 2. TurnExecutor 结构

### 2.1 依赖注入对象列表

TurnExecutor 构造所需依赖：

1. `Arc<dyn SessionLifecycle>`
2. `Arc<dyn EventStore>`
3. `Arc<dyn ContextBuilder>`
4. `Arc<dyn LlmProvider>`
5. `Arc<dyn ToolRuntime>`
6. `Arc<dyn Clock>`（可选，便于测试）
7. `Arc<dyn IdGenerator>`（可选，便于事件 ID 测试）
8. `TurnExecutorConfig`

> 注：`SessionLifecycle` 已依赖 `EventStore`，但 TurnExecutor 仍可直接持有 `EventStore` 用于局部读写优化与诊断读取。也可仅保留 Lifecycle，具体以实现版本决定。

### 2.2 TurnExecutorConfig（Rust 伪代码）

```rust
#[derive(Debug, Clone)]
pub struct TurnExecutorConfig {
    pub max_tool_iterations: u8,
    pub llm_retry_max: u8,
    pub llm_retry_backoff_ms: u64,
    pub enable_llm_fallback: bool,
    pub persist_failure_degrade: bool,
    pub emit_diagnostics: bool,
}
```

### 2.3 TurnInput 定义（Rust 伪代码）

```rust
#[derive(Debug, Clone)]
pub struct TurnInput {
    pub session_key: SessionKey,
    pub user_message: UserMessageInput,
    pub metadata: TurnMetadata,
}

#[derive(Debug, Clone)]
pub struct UserMessageInput {
    pub text: String,
    pub attachments: Vec<AttachmentRef>,
    pub client_message_id: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct TurnMetadata {
    pub tenant_id: Option<String>,
    pub user_id: Option<String>,
    pub agent_id: Option<String>,
    pub trace_id: Option<String>,
    pub channel: Option<String>,
    pub request_id: Option<String>,
    pub route_key: Option<String>,
}
```

### 2.4 TurnOutput 定义（Rust 伪代码）

```rust
#[derive(Debug, Clone)]
pub struct TurnOutput {
    pub session_id: SessionId,
    pub assistant_text: String,
    pub finish_reason: TurnFinishReason,
    pub usage: Option<LlmUsage>,
    pub tool_iterations: u8,
    pub persisted: bool,
    pub warnings: Vec<TurnWarning>,
}

#[derive(Debug, Clone)]
pub enum TurnFinishReason {
    Stop,
    ToolLoopExceeded,
    LengthTruncated,
    ErrorFallback,
}

#[derive(Debug, Clone)]
pub enum TurnWarning {
    PersistFailed(String),
    LlmFallbackUsed(String),
    ToolCallFailed { request_id: String, code: String },
    ContextCompacted,
}
```

### 2.5 TurnError 定义（Rust 伪代码）

```rust
#[derive(Debug, thiserror::Error)]
pub enum TurnError {
    #[error("session lifecycle failed: {0}")]
    SessionLifecycle(String),

    #[error("context build failed: {0}")]
    ContextBuild(String),

    #[error("llm failed: {0}")]
    Llm(String),

    #[error("tool loop exceeded max iterations: {max}")]
    ToolLoopExceeded { max: u8 },

    #[error("internal: {0}")]
    Internal(String),
}
```

### 2.6 TurnExecutor Struct（Rust 伪代码）

```rust
pub struct TurnExecutor {
    lifecycle: Arc<dyn SessionLifecycle>,
    event_store: Arc<dyn EventStore>,
    context_builder: Arc<dyn ContextBuilder>,
    llm: Arc<dyn LlmProvider>,
    tools: Arc<dyn ToolRuntime>,
    clock: Arc<dyn Clock>,
    id_gen: Arc<dyn IdGenerator>,
    config: TurnExecutorConfig,
}
```

### 2.7 内部运行态结构（Rust 伪代码）

```rust
#[derive(Debug)]
struct TurnState {
    session: Session,
    correlation_id: String,
    new_events: Vec<NewEvent>,
    iteration: u8,
    warnings: Vec<TurnWarning>,
    last_llm_response: Option<LlmResponse>,
}
```

---

## 3. run_turn 完整流程

### 3.1 流程总览

一次 `run_turn` 的标准路径：

1. 接收 TurnInput
2. `SessionLifecycle.load_or_create(session_key)`
3. 追加 `UserMessage` 事件
4. `ContextBuilder.build(events + config)`
5. `LlmProvider.complete(request)`
6. 判断 finish_reason（Stop / ToolCalls / Length）
7. 必要时进入工具循环
8. `SessionLifecycle.persist_turn(new_events)`
9. `SessionLifecycle.compact_if_needed()`
10. 组装 TurnOutput 返回

### 3.2 Step 1：接收 TurnInput

输入：

- `session_key`
- `user_message`
- `metadata`

动作：

1. 基础校验（空消息、长度上限）
2. 生成 `correlation_id`
3. 初始化 `TurnState`

可能错误：

- 参数非法（`TurnError::Internal` / 输入层提前拦截）

产生事件：

- 无（仅初始化内存态）

### 3.3 Step 2：load_or_create

调用：

- `SessionLifecycle.load_or_create(session_key)`

输入：

- `SessionKey`

输出：

- `Session`

可能错误：

- `SessionLifecycleError::Store`
- `SessionLifecycleError::RetryableConflict`
- `SessionLifecycleError::SessionArchived`

错误策略：

- 直接返回 `TurnError::SessionLifecycle`
- 不继续后续流程

产生事件：

- 由 SessionLifecycle 内部负责（如 `SystemEvent::SessionCreated`）

### 3.4 Step 3：追加 UserMessage

动作：

1. 构造 `UserMessageEvent`
2. 附加 `SystemEvent::TurnStarted`（推荐）
3. 追加到 `state.new_events`

调用 trait：

- 本步骤先在内存态积累，不立即落盘
- 最终由 `persist_turn` 一次提交

可能错误：

- ID 生成失败（极少，视实现）

产生事件：

1. `UserMessage`
2. `SystemEvent(TurnStarted)`（可选但推荐）

### 3.5 Step 4：build context

调用：

- `ContextBuilder.build(ContextInput)`

输入来源：

1. 会话已有事件（由 EventStore 读取，或 Lifecycle 提供）
2. 本轮内存事件（至少包含 UserMessage）
3. token 预算与策略

输出：

- `ContextOutput { system_prompt, messages, diagnostics }`

可能错误：

- `ContextError`（映射为 `TurnError::ContextBuild`）

错误策略：

- 直接返回
- 不进入 LLM 调用

产生事件：

- 无（可记录诊断日志）

### 3.6 Step 5：LLM complete

调用：

- `LlmProvider.complete(LlmRequest)`

关键输入：

1. `messages`
2. `tool_specs`（来自 `ToolRuntime.list_specs`）
3. `model/config/metadata`

输出：

- `LlmResponse { content, tool_calls, finish_reason, usage, ... }`

可能错误：

- timeout/rate_limit/provider_down
- invalid_request
- invalid_response

错误策略：

1. 可重试错误：按策略重试
2. 可 fallback 错误：切换 provider（若启用）
3. 不可恢复错误：返回 `TurnError::Llm`

产生事件：

- 本步不立即产事件
- 在 finish_reason 分支中决定要写哪些事件

### 3.7 Step 6：finish_reason 分支

#### 3.7.1 Stop

动作：

1. 追加 `AssistantMessage` 事件
2. 追加 `SystemEvent(TurnCompleted)`（推荐）
3. 结束 LLM/工具阶段

可能错误：

- 事件构造失败（理论上极少）

#### 3.7.2 ToolCalls

动作：

1. 若 response 有文本，可先记一条 `AssistantMessage`（可选）
2. 进入工具循环

可能错误：

- 无直接错误，交给工具循环处理

#### 3.7.3 Length

动作（MVP 推荐）：

1. 优先触发一次压缩重建上下文再重试（限 1 次）
2. 若仍 Length，生成友好截断回复
3. 追加 `AssistantMessage(finish_reason=Length)`
4. 追加 `SystemEvent(PolicyNotice/TurnCompleted)`

可能错误：

- 压缩失败 -> 降级为截断提示文本

### 3.8 Step 7：工具循环

说明：

- 详细逻辑见第 4 章
- 本章只描述在主流程中的位置

可能结果：

1. Stop 结束
2. Length 结束
3. 超过上限，受控失败

### 3.9 Step 8：persist_turn

调用：

- `SessionLifecycle.persist_turn(TurnDelta)`

输入：

- `new_events`
- `expected_version`
- `last_active_at`
- `estimated_prompt_tokens`

成功：

- 事件持久化成功
- 会话元数据更新

失败：

- 若 `persist_failure_degrade = true`：
  - 记录 warning
  - 继续返回 AI 回复
- 若关闭降级：
  - 返回 `TurnError::SessionLifecycle`

### 3.10 Step 9：compact_if_needed

调用：

- `SessionLifecycle.compact_if_needed(session_key)`

策略：

- 后置执行，不阻塞主回复（可同步轻执行）
- 失败不影响本轮主输出

成功：

- 可能追加 `Summary + CompactionMarker`
- 在 output warning 中可标注 `ContextCompacted`

失败：

- 记录 warning
- 不升级为 turn 致命错误

### 3.11 Step 10：组装 TurnOutput

输出字段来源：

1. `assistant_text`：最终 Assistant 文本
2. `finish_reason`：最终终止原因
3. `usage`：最后一轮或聚合 usage
4. `tool_iterations`：实际迭代次数
5. `persisted`：持久化是否成功
6. `warnings`：降级/fallback/compact 等信息

### 3.12 run_turn 主伪代码（Rust）

```rust
pub async fn run_turn(&self, input: TurnInput) -> Result<TurnOutput, TurnError> {
    let correlation_id = self.id_gen.correlation_id();

    let session = self
        .lifecycle
        .load_or_create(input.session_key.clone())
        .await
        .map_err(|e| TurnError::SessionLifecycle(e.to_string()))?;

    let mut state = TurnState {
        session,
        correlation_id,
        new_events: vec![],
        iteration: 0,
        warnings: vec![],
        last_llm_response: None,
    };

    state.new_events.push(self.new_turn_started_event(&state, &input));
    state.new_events.push(self.new_user_message_event(&state, &input));

    let mut context = self
        .build_context(&state, &input)
        .map_err(|e| TurnError::ContextBuild(e.to_string()))?;

    let mut llm_resp = self
        .complete_with_retry_and_fallback(&input, &context)
        .await
        .map_err(|e| TurnError::Llm(e.to_string()))?;

    state.last_llm_response = Some(llm_resp.clone());

    let mut final_text = String::new();
    let final_reason: TurnFinishReason;

    match llm_resp.finish_reason {
        FinishReason::Stop => {
            final_text = llm_resp.content.clone();
            state.new_events.push(self.new_assistant_event(&state, &llm_resp));
            final_reason = TurnFinishReason::Stop;
        }
        FinishReason::ToolCalls => {
            let loop_outcome = self
                .run_tool_loop(&input, &mut state, llm_resp, &mut context)
                .await?;
            final_text = loop_outcome.final_text;
            llm_resp = loop_outcome.final_response;
            final_reason = loop_outcome.finish_reason;
        }
        FinishReason::Length => {
            let truncated = self.handle_length_case(&input, &mut state, &mut context).await?;
            final_text = truncated.text.clone();
            state.new_events.push(truncated.assistant_event);
            final_reason = TurnFinishReason::LengthTruncated;
        }
        _ => {
            final_text = llm_resp.content.clone();
            state.new_events.push(self.new_assistant_event(&state, &llm_resp));
            final_reason = TurnFinishReason::ErrorFallback;
        }
    }

    state.new_events.push(self.new_turn_completed_event(&state));

    let persisted = match self.persist_turn(&state).await {
        Ok(_) => true,
        Err(e) => {
            if self.config.persist_failure_degrade {
                state.warnings.push(TurnWarning::PersistFailed(e.to_string()));
                false
            } else {
                return Err(TurnError::SessionLifecycle(e.to_string()));
            }
        }
    };

    let _ = self.try_compact(&input, &mut state).await;

    Ok(TurnOutput {
        session_id: state.session.session_id.clone(),
        assistant_text: final_text,
        finish_reason: final_reason,
        usage: llm_resp.usage.clone(),
        tool_iterations: state.iteration,
        persisted,
        warnings: state.warnings,
    })
}
```

---

## 4. 工具循环详细设计

### 4.1 循环结构

核心结构：

```rust
while iteration < max_tool_iterations
```

其中：

- `iteration` 从 0 开始
- 每次处理一轮 LLM -> tools -> LLM

### 4.2 每轮 7 个步骤

每次迭代固定执行：

1. 从 LLM response 提取 `tool_calls`
2. 追加 `ToolCallRequest` 事件
3. `ToolRuntime.execute_calls(tool_calls)`
4. 追加 `ToolCallResult` 事件
5. 重新组装上下文（包含工具结果）
6. 再次调用 LLM
7. 判断 `finish_reason`

### 4.3 第 1 步：提取 tool_calls

输入：

- 当前 `LlmResponse`

逻辑：

1. 正常解析统一 `ToolCall` 模型
2. 空列表视为协议异常（与 `ToolCalls` finish_reason 不一致）

异常处理：

- 若 finish_reason=ToolCalls 但 calls 为空：
  - 记录 warning
  - 直接降级为 Stop 文本回复

### 4.4 第 2 步：记录 ToolCallRequest

每个 call 追加一条 `ToolCallRequest` 事件。

关键字段：

1. `request_id = call.id`
2. `tool_name = call.name`
3. `arguments_json = call.arguments`
4. `attempt = iteration + 1`

错误：

- 事件构造失败：终止本轮并返回 `TurnError::Internal`

### 4.5 第 3 步：执行工具

调用：

- `ToolRuntime.execute_calls(ctx, calls)`

要求：

1. 单 call 隔离
2. 部分失败不影响其它 call
3. 超时/取消映射为结构化 result

返回：

- `Vec<ToolResult>`

### 4.6 第 4 步：记录 ToolCallResult

每个 result 追加 `ToolCallResult` 事件。

关键映射：

1. `status`（Success/Failed/Timeout/Cancelled）
2. `output_json`
3. `error_code`
4. `error_message`
5. `latency_ms`

处理策略：

- 即使 result 为失败，也照常记录并回填 LLM

### 4.7 第 5 步：重建上下文

调用：

- `ContextBuilder.build(...)`

输入包含：

1. 历史事件
2. 本轮新增 tool request/result 事件
3. 原始用户消息

目标：

- 让下一次 LLM 能看到工具结果并继续推理

### 4.8 第 6 步：再次调用 LLM

调用：

- `LlmProvider.complete(...)`

可能出现：

1. `Stop`
2. `ToolCalls`（继续下一轮）
3. `Length`

### 4.9 第 7 步：判断 finish_reason

#### Stop

动作：

1. 追加 `AssistantMessage`
2. 退出循环

#### ToolCalls

动作：

1. iteration += 1
2. 继续循环

#### Length

动作：

1. 尝试一次压缩或截断处理
2. 追加 `AssistantMessage(finish_reason=Length)`
3. 退出循环

### 4.10 终止条件

循环终止条件三类：

1. `Stop`
2. `Length`
3. `iteration >= max_tool_iterations`

### 4.11 超限处理

当超过上限：

1. 生成友好错误文本
2. 追加 `AssistantMessage`（说明工具循环达到上限）
3. 追加 `SystemEvent(TurnFailed/PolicyNotice)`
4. 返回 `TurnFinishReason::ToolLoopExceeded`

友好文本示例：

> “我尝试了多轮工具调用但仍未收敛。为避免无限循环，本次先停在这里。你可以缩小问题范围或指定更明确的目标，我再继续。”

### 4.12 工具循环完整伪代码（Rust）

```rust
async fn run_tool_loop(
    &self,
    input: &TurnInput,
    state: &mut TurnState,
    mut resp: LlmResponse,
    context: &mut ContextOutput,
) -> Result<ToolLoopOutcome, TurnError> {
    while state.iteration < self.config.max_tool_iterations {
        let calls = resp.tool_calls.clone();

        if calls.is_empty() {
            let txt = if resp.content.is_empty() {
                "".to_string()
            } else {
                resp.content.clone()
            };
            state.new_events.push(self.new_assistant_event(state, &resp));
            return Ok(ToolLoopOutcome::stop(txt, resp, state.iteration));
        }

        for call in &calls {
            state.new_events.push(self.new_tool_call_request_event(state, call));
        }

        let results = self
            .tools
            .execute_calls(&self.tool_exec_ctx(input, state), calls)
            .await;

        for r in &results {
            if r.status != ToolCallStatus::Success {
                state.warnings.push(TurnWarning::ToolCallFailed {
                    request_id: r.request_id.clone(),
                    code: r.error_code.clone().unwrap_or_else(|| "UNKNOWN".into()),
                });
            }
            state.new_events.push(self.new_tool_call_result_event(state, r));
        }

        *context = self
            .build_context(state, input)
            .map_err(|e| TurnError::ContextBuild(e.to_string()))?;

        resp = self
            .complete_with_retry_and_fallback(input, context)
            .await
            .map_err(|e| TurnError::Llm(e.to_string()))?;

        match resp.finish_reason {
            FinishReason::Stop => {
                let txt = resp.content.clone();
                state.new_events.push(self.new_assistant_event(state, &resp));
                return Ok(ToolLoopOutcome::stop(txt, resp, state.iteration + 1));
            }
            FinishReason::Length => {
                let txt = self.length_friendly_text();
                let mut truncated_resp = resp.clone();
                truncated_resp.content = txt.clone();
                state.new_events.push(self.new_assistant_event(state, &truncated_resp));
                return Ok(ToolLoopOutcome::length(txt, truncated_resp, state.iteration + 1));
            }
            FinishReason::ToolCalls => {
                state.iteration += 1;
                continue;
            }
            _ => {
                let txt = resp.content.clone();
                state.new_events.push(self.new_assistant_event(state, &resp));
                return Ok(ToolLoopOutcome::error_fallback(txt, resp, state.iteration + 1));
            }
        }
    }

    let txt = self.tool_loop_exceeded_text();
    state.new_events.push(self.new_assistant_text_event(state, txt.clone(), FinishReason::Error));
    state.new_events.push(self.new_turn_failed_event(state, "tool_loop_exceeded"));

    Ok(ToolLoopOutcome::exceeded(txt, state.iteration))
}
```

### 4.13 并发与顺序保证

工具循环内事件顺序应固定：

1. AssistantToolCalls（可选）
2. ToolCallRequest * N
3. ToolCallResult * N
4. AssistantMessage（收敛时）

这样可保证 event replay 的可解释性。

### 4.14 工具失败不炸穿原则

`ToolRuntime` 失败必须封装成 `ToolCallResult` 回给 LLM。

TurnExecutor 不应在单个工具失败时直接 abort。

仅在 runtime 全局不可用（例如 panic/严重协议损坏）才终止 turn。

---

## 5. 错误处理

### 5.1 错误处理总表

| 环节 | 典型错误 | 处理策略 | 是否继续 |
|---|---|---|---|
| SessionLifecycle.load_or_create | 存储错误/冲突/归档态 | 直接返回错误 | 否 |
| ContextBuilder.build | 上下文构建失败 | 直接返回错误 | 否 |
| LlmProvider.complete | timeout/rate_limit/provider_down | retry/fallback，失败则返回 | 视结果 |
| ToolRuntime.execute_calls | 单工具失败/超时/未注册 | 封装 ToolCallResult 回给 LLM | 是 |
| SessionLifecycle.persist_turn | 持久化失败 | 记录 warning，返回 AI 文本（可配置） | 是（默认） |
| SessionLifecycle.compact_if_needed | 压缩失败 | 记录 warning，不影响本轮输出 | 是 |

### 5.2 SessionLifecycle 失败

失败点：

1. `load_or_create`
2. `persist_turn`
3. `compact_if_needed`

策略：

- `load_or_create` 失败：立即失败返回
- `persist_turn` 失败：默认降级（但保留开关）
- `compact_if_needed` 失败：仅 warning

### 5.3 ContextBuilder 失败

策略：

1. 若关键构建失败（Identity/Safety 缺失）
   - 直接返回 `TurnError::ContextBuild`
2. 非关键 section 失败
   - 在 builder 内可跳过并产诊断

TurnExecutor 层保持简单：builder 报错即失败。

### 5.4 LlmProvider 失败

策略分层：

1. 可重试错误：同 provider 重试
2. 可 fallback 错误：切换备选 provider
3. 不可恢复错误：直接返回

并记录 warning：

- `LlmFallbackUsed(provider/model)`

### 5.5 ToolRuntime 失败

分两类：

1. call 级失败
2. runtime 级失败

#### call 级失败

- 转成 `ToolCallResult(status=Failed/Timeout/Cancelled)`
- 继续下一步

#### runtime 级失败（整批不可用）

- 为每个 call 生成框架错误结果
- 仍回填 LLM

### 5.6 persist_turn 失败降级

目标：

- 尽量不因为落库抖动让用户收不到回复

策略：

1. 返回 assistant 文本
2. `persisted=false`
3. warnings 带上失败原因
4. 由外层观测系统告警

### 5.7 错误事件记录

建议记录以下系统事件：

1. `SystemEvent::TurnFailed`
2. `SystemEvent::PolicyNotice`
3. `SystemEvent::ProviderFallback`

记录时机：

- 致命错误前（若来得及）
- 非致命降级发生时

### 5.8 错误映射伪代码

```rust
fn map_error(e: anyhow::Error, phase: TurnPhase) -> TurnError {
    match phase {
        TurnPhase::LoadSession => TurnError::SessionLifecycle(e.to_string()),
        TurnPhase::BuildContext => TurnError::ContextBuild(e.to_string()),
        TurnPhase::CallLlm => TurnError::Llm(e.to_string()),
        TurnPhase::ToolLoop => TurnError::Internal(e.to_string()),
        TurnPhase::Persist => TurnError::SessionLifecycle(e.to_string()),
    }
}
```

### 5.9 重试边界

避免无限 retry：

1. LLM 重试次数有上限
2. tool loop 有上限
3. persist 重试最多 1~2 次（可选）

### 5.10 用户可见错误策略

用户可见文本应友好且不泄漏内部细节。

示例：

- LLM 不可用：
  - “当前模型服务暂时不可用，请稍后再试。”
- 工具超限：
  - “我尝试了多轮工具调用但未收敛，已安全停止。”
- 截断：
  - “上下文过长，我已基于可用信息给出简化答案。”

---

## 6. 与各层的协作图

### 6.1 正常无工具路径（文本序列图）

```text
Layer1 Channel
  -> Layer2 TurnExecutor.run_turn(input)
      -> Layer2 SessionLifecycle.load_or_create(session_key)
          -> Layer3 EventStore.get/create session
              -> Layer4 PostgresEventStore (DB)
      -> Layer3 ContextBuilder.build(events + config)
      -> Layer3 ToolRuntime.list_specs(ctx)
      -> Layer3 LlmProvider.complete(request)
          -> Layer4 ProviderAdapter (OpenAI/Anthropic/...)
      -> [finish_reason=Stop]
      -> Layer2 SessionLifecycle.persist_turn(new_events)
          -> Layer3 EventStore.append/update
              -> Layer4 PostgresEventStore (DB)
      -> Layer2 SessionLifecycle.compact_if_needed(session_key)
          -> Layer3 EventStore.read/append
              -> Layer4 PostgresEventStore (DB)
  <- Layer2 TurnOutput
<- Layer1 response dto
```

### 6.2 工具循环路径（文本序列图）

```text
Layer1 Channel
  -> Layer2 TurnExecutor
      -> Layer2 SessionLifecycle.load_or_create
      -> Layer3 ContextBuilder.build
      -> Layer3 LlmProvider.complete
      -> [finish_reason=ToolCalls]
      -> loop(iteration < max)
          -> extract tool_calls
          -> append ToolCallRequest (in-memory delta)
          -> Layer3 ToolRuntime.execute_calls
              -> (可能到 Layer4 外部 API/DB)
          -> append ToolCallResult (in-memory delta)
          -> Layer3 ContextBuilder.build (含 tool results)
          -> Layer3 LlmProvider.complete
          -> check finish_reason
      -> Layer2 SessionLifecycle.persist_turn
      -> Layer2 SessionLifecycle.compact_if_needed
  <- Layer2 TurnOutput
```

### 6.3 失败降级路径（persist 失败）

```text
Layer2 TurnExecutor
  -> 已拿到最终 assistant_text
  -> SessionLifecycle.persist_turn FAIL
  -> record warning PersistFailed
  -> compact_if_needed (optional skip)
  -> return TurnOutput(persisted=false, warnings=[...])
```

### 6.4 层边界标注

关键跨层调用：

1. L2 -> L3：所有能力调用（Context/LLM/Tools/Lifecycle Port）
2. L3 -> L4：具体适配器实现（DB/Provider/External Tool）
3. L1 -> L2：仅调用 run_turn，不穿透到 L3/L4

### 6.5 依赖方向检查清单（TurnExecutor 视角）

- [ ] `agent-orchestrator` 未 import `agent-storage` concrete impl
- [ ] TurnExecutor 构造仅接受 trait 对象
- [ ] 无 SQL、HTTP SDK 调用出现在 TurnExecutor
- [ ] 工具协议解析在 ToolRuntime/Provider 统一模型中完成

---

## 7. 与参考框架对比

### 7.1 ZeroClaw `Agent::turn()` 结构回顾

参考 `zeroclaw/src/agent/agent.rs`（~467 行附近）：

典型流程：

1. 首轮构建 system prompt
2. 用户消息入 history
3. 进入 `for _ in 0..max_tool_iterations`
4. provider chat
5. parse response + tool calls
6. 无 tools -> 结束
7. 有 tools -> 执行 -> 结果回填 -> 下一轮
8. 超限报错

### 7.2 本设计与 ZeroClaw 的共性

1. 都采用“单 turn 主循环 + 工具迭代上限”
2. 都在 LLM 与工具之间反复迭代直到收敛
3. 都把 tool result 回填给模型继续推理
4. 都有 context 压缩/裁剪意识

### 7.3 关键差异一：事件流优先

ZeroClaw 以会话 history（消息列表）为主。

本设计以 event stream 为主：

1. 每个动作可追溯事件
2. ToolCallRequest/Result 一等公民
3. Summary/CompactionMarker 明确建模

理由：

- agent-demo 已确定事件流为核心数据模型
- 需要更强审计与重放能力

### 7.4 关键差异二：SessionLifecycle 显式分层

ZeroClaw 的 turn 中会话治理相对内聚在 Agent 内。

本设计把会话治理明确抽到 `SessionLifecycle`：

1. load/create
2. persist
3. compact
4. archive

理由：

- 与 Layer 2 分层目标一致
- 并发与生命周期复杂度独立管理

### 7.5 关键差异三：persist 失败降级策略

ZeroClaw CLI 场景更偏本地交互，持久化不是强一致前提。

本设计面向服务端多租户：

- 默认允许 persist 失败时返回文本（可配置）
- 同时保留 warning 与告警能力

理由：

- 用户体验优先 + 系统韧性
- 避免瞬时存储抖动导致全量失败

### 7.6 关键差异四：层间契约更严格

ZeroClaw 是单项目内部模块协作。

本设计强约束四层边界：

1. L1 只入 L2
2. L2 只依赖 L3 trait
3. L4 实现 L3 trait

理由：

- 保持可替换与可测试性
- 支持后续按域演进

### 7.7 PicoClaw 对比补充

PicoClaw `runLLMIteration` 与本设计相近点：

1. 迭代 loop
2. tool defs -> llm -> tools -> llm
3. context/token 超限时压缩后重试

本设计差异：

1. 更严格的事件记录语义
2. 更明确的 SessionLifecycle 端口
3. 更明确的 persist/compact 错误分级

### 7.8 差异合理性总结

本设计不是否定参考框架，而是针对 agent-demo 场景做约束增强：

1. 服务端多租户
2. 事件流持久化
3. 分层边界治理
4. 可测性优先

---

## 附录 A：run_turn 更细粒度伪代码（可直接转实现）

```rust
pub async fn run_turn(&self, input: TurnInput) -> Result<TurnOutput, TurnError> {
    let now = self.clock.now_utc();
    let correlation_id = self.id_gen.correlation_id();

    // A. Load/Create session
    let session = self.lifecycle
        .load_or_create(input.session_key.clone())
        .await
        .map_err(|e| TurnError::SessionLifecycle(e.to_string()))?;

    let mut state = TurnState::new(session, correlation_id);

    // B. Build turn-start events
    state.new_events.push(self.ev_turn_started(&state, &input, now));
    state.new_events.push(self.ev_user_message(&state, &input, now));

    // C. Build initial context
    let mut ctx = self.build_context_with_delta(&state, &input)?;

    // D. Initial LLM call
    let mut resp = self.complete_with_policy(&input, &ctx, &mut state).await?;

    let final_finish_reason = loop {
        match resp.finish_reason {
            FinishReason::Stop => {
                state.new_events.push(self.ev_assistant_from_resp(&state, &resp, self.clock.now_utc()));
                break TurnFinishReason::Stop;
            }
            FinishReason::ToolCalls => {
                let out = self.run_tool_loop(&input, &mut state, resp, &mut ctx).await?;
                resp = out.final_response;
                state.iteration = out.iteration;
                break out.finish_reason;
            }
            FinishReason::Length => {
                let fallback_txt = self.length_friendly_text();
                state.new_events.push(self.ev_assistant_text(
                    &state,
                    fallback_txt.clone(),
                    FinishReason::Length,
                    self.clock.now_utc(),
                ));
                resp.content = fallback_txt;
                break TurnFinishReason::LengthTruncated;
            }
            _ => {
                // 保守兜底
                let txt = if resp.content.is_empty() {
                    self.generic_fallback_text()
                } else {
                    resp.content.clone()
                };
                state.new_events.push(self.ev_assistant_text(
                    &state,
                    txt.clone(),
                    FinishReason::Error,
                    self.clock.now_utc(),
                ));
                resp.content = txt;
                break TurnFinishReason::ErrorFallback;
            }
        }
    };

    state.new_events.push(self.ev_turn_completed(&state, self.clock.now_utc()));

    // E. Persist
    let persisted = match self.lifecycle.persist_turn(TurnDelta {
        session_id: state.session.session_id.clone(),
        expected_version: state.session.version,
        events: state.new_events.clone(),
        estimated_prompt_tokens: ctx.diagnostics.estimated_prompt_tokens,
        last_active_at: self.clock.now_utc(),
    }).await {
        Ok(_) => true,
        Err(e) => {
            if self.config.persist_failure_degrade {
                state.warnings.push(TurnWarning::PersistFailed(e.to_string()));
                false
            } else {
                return Err(TurnError::SessionLifecycle(e.to_string()));
            }
        }
    };

    // F. Compact (best effort)
    if let Ok(outcome) = self.lifecycle.compact_if_needed(&input.session_key).await {
        if outcome.compacted() {
            state.warnings.push(TurnWarning::ContextCompacted);
        }
    }

    Ok(TurnOutput {
        session_id: state.session.session_id,
        assistant_text: resp.content,
        finish_reason: final_finish_reason,
        usage: resp.usage,
        tool_iterations: state.iteration,
        persisted,
        warnings: state.warnings,
    })
}
```

---

## 附录 B：实现检查清单

### B.1 编排职责

- [ ] TurnExecutor 不实现具体 DB/HTTP 调用
- [ ] 所有外部依赖通过 trait 注入
- [ ] run_turn 只负责单 turn

### B.2 工具循环

- [ ] `max_tool_iterations` 强制生效
- [ ] 每轮都记录 request/result 事件
- [ ] failure 走 ToolCallResult 而非 panic

### B.3 错误策略

- [ ] load/create 失败直接返回
- [ ] context 失败直接返回
- [ ] llm 失败先 retry/fallback
- [ ] persist 失败可降级返回

### B.4 事件一致性

- [ ] UserMessage 在 turn 起点产出
- [ ] AssistantMessage 在结束前产出
- [ ] TurnStarted/TurnCompleted 成对出现（失败时 TurnFailed）

### B.5 与层边界一致

- [ ] L1 仅调用 run_turn
- [ ] L2 仅依赖 L3 trait
- [ ] L4 不反向依赖 L2

---

## 结论

TurnExecutor 在本设计中是 Layer 2 的“单次 turn 状态机”。

它通过 trait 协调 ContextBuilder、LlmProvider、ToolRuntime、SessionLifecycle、EventStore。

其核心价值：

1. 流程清晰
2. 事件可追溯
3. 工具循环可控
4. 错误可降级
5. 依赖可替换
6. 单测可覆盖

这使 agent-demo 在 MVP 阶段即可具备“可运行 + 可维护 + 可演进”的编排基线。
