# 框架分层架构深度对比（源码级）

> 调研时间：2026-02-24
> 调研范围：OpenClaw / ZeroClaw / PicoClaw 源码（~/code/references/）
> 目的：为 agent-demo 架构重新设计提供参考

---

## 1. 整体分层

三个框架都呈现 4 层结构（名字不同但职责一致）：

```
Layer 1: 入口/接入层（CLI/Channel）
    ↓ 用户消息进来
Layer 2: 编排层（Agent/Loop）——turn handler 在这里
    ↓ 调用下面的能力
Layer 3: 能力层（Provider/Tool/Memory/Context）
    ↓ 各自独立
Layer 4: 适配器层（具体 API 实现）
```

### OpenClaw (TypeScript)

| 层 | 核心模块 | 职责 |
|---|---|---|
| 接入层 | `src/main.ts`, `src/commands/*` | CLI 命令路由 |
| 编排层 | `src/agents/pi-embedded-runner/run.ts`, `run/attempt.ts` | 队列、重试、failover、compaction 策略 |
| 内核层 | `@mariozechner/pi-coding-agent`（外部依赖） | `createAgentSession`, `activeSession.prompt` |
| 基础设施层 | `src/agents/*tools*`, `src/plugins/*`, `src/config/sessions/*` | 工具、插件、会话存储 |

特点：核心 agent 循环**委托给外部库** pi-coding-agent，OpenClaw 本身主要做编排和治理。

### ZeroClaw (Rust)

| 层 | 核心模块 | 职责 |
|---|---|---|
| CLI 层 | `src/main.rs` | 命令解析，`Commands::Agent` |
| 编排层 | `src/agent/agent.rs` | `Agent::turn()`, AgentBuilder |
| 领域抽象层 | 各域 `traits.rs` | Provider/Tool/Memory/Channel/Runtime trait |
| 适配器层 | `src/providers/*`, `src/tools/*`, `src/memory/*` 等 | 具体实现 |

特点：**最典型的 Rust 端口-适配器架构**，trait-first，Agent 持有 trait object。

### PicoClaw (Go)

| 层 | 核心模块 | 职责 |
|---|---|---|
| CLI 层 | `cmd/picoclaw/*` | 命令入口 |
| 编排层 | `pkg/agent/loop.go` | `runAgentLoop()`, `runLLMIteration()` |
| 上下文/会话层 | `pkg/agent/context.go`, `pkg/session/manager.go` | ContextBuilder, SessionManager |
| 能力层 | `pkg/tools/*`, `pkg/providers/*` | 工具和 LLM provider |

特点：**结构最直白**，ContextBuilder/SessionManager 边界非常清楚。

---

## 2. Turn Handler 位置

三个框架都把 turn handler 放在 Layer 2 编排层：

| 框架 | Turn Handler 函数 | 文件 | 工具循环位置 |
|---|---|---|---|
| **ZeroClaw** | `Agent::turn()` | `src/agent/agent.rs:467` | 内嵌 `for _ in 0..max_tool_iterations` |
| **PicoClaw** | `runAgentLoop()` → `runLLMIteration()` | `pkg/agent/loop.go` | 内嵌 `for iteration < MaxIterations` |
| **OpenClaw** | `runEmbeddedAttempt()` | `src/agents/pi-embedded-runner/run/attempt.ts` | 委托 `activeSession.prompt()` |

### ZeroClaw turn 流程（最值得参考）

```rust
pub async fn turn(&mut self, user_message: &str) -> Result<String> {
    // 1. 首轮构建 system prompt
    if self.history.is_empty() {
        let system_prompt = self.build_system_prompt()?;
        self.history.push(ChatMessage::system(system_prompt));
    }

    // 2. 记忆加载
    let context = self.memory_loader.load_context(self.memory.as_ref(), user_message).await;

    // 3. 时间戳 + 记忆上下文拼入用户消息
    let enriched = format!("{context}[{now}] {user_message}");
    self.history.push(ChatMessage::user(enriched));

    // 4. 工具循环
    for _ in 0..self.config.max_tool_iterations {
        let messages = self.tool_dispatcher.to_provider_messages(&self.history);
        let response = self.provider.chat(ChatRequest { messages, tools }).await?;
        let (text, calls) = self.tool_dispatcher.parse_response(&response);

        if calls.is_empty() {
            // 无工具调用 → 最终文本
            self.history.push(ChatMessage::assistant(final_text));
            self.trim_history();
            return Ok(final_text);
        }

        // 执行工具 → 结果回填 → 继续循环
        self.history.push(AssistantToolCalls { ... });
        let results = self.execute_tools(&calls).await;
        self.history.push(formatted_results);
        self.trim_history();
    }

    bail!("exceeded max tool iterations")
}
```

关键设计点：
- system prompt **只在首轮构建**，之后不变（利于 prompt cache）
- 时间戳注入用户消息而不是改 system prompt
- `ToolDispatcher` 负责工具协议转换（native/XML 两种模式），不在 turn 里硬编码

---

## 3. trait/interface 组织方式

### ZeroClaw — 按域分布（最佳实践）

```
src/
├── providers/traits.rs   → pub trait Provider { async fn chat(...) }
├── tools/traits.rs       → pub trait Tool { fn spec(&self); async fn execute(...) }
├── memory/traits.rs      → pub trait Memory { async fn store(...); async fn search(...) }
├── channels/traits.rs    → pub trait Channel { async fn listen(...); async fn send(...) }
├── runtime/traits.rs     → pub trait RuntimeAdapter { ... }
├── security/traits.rs    → pub trait Sandbox { ... }
├── hooks/traits.rs       → pub trait HookHandler { ... }
├── observability/traits.rs → pub trait Observer { fn on_event(...) }
```

Agent 组合这些 trait object：
```rust
pub struct Agent {
    provider: Box<dyn Provider>,
    tools: Vec<Box<dyn Tool>>,
    memory: Arc<dyn Memory>,
    observer: Arc<dyn Observer>,
    prompt_builder: SystemPromptBuilder,
    tool_dispatcher: Box<dyn ToolDispatcher>,
    memory_loader: Box<dyn MemoryLoader>,
    // ...
}
```

**不是平铺在一个 `ports.rs`**，而是每个域自己定义自己的 trait。域内高内聚，域间低耦合。

### PicoClaw — 分散在各包

- `pkg/tools/base.go` → `Tool`, `ContextualTool`, `AsyncTool` 接口
- `pkg/providers/types.go` → `LLMProvider` 接口
- `pkg/channels/base.go` → `Channel` 接口

有"小接口组合"风格（Tool + ContextualTool + AsyncTool）。

### OpenClaw — 分散式 + 外部依赖

- 本仓接口较少（`AcpSessionStore`, 插件 hook 类型等）
- 核心 agent 接口在外部 `pi-coding-agent` 库

---

## 4. 上下文编排

三个框架都把上下文编排做成**独立模块**，不嵌在 turn handler 里：

### ZeroClaw — PromptSection 可插拔（设计最佳）

```rust
// src/agent/prompt.rs
pub trait PromptSection: Send + Sync {
    fn name(&self) -> &str;
    fn build(&self, ctx: &PromptContext<'_>) -> Result<String>;
}

pub struct SystemPromptBuilder {
    sections: Vec<Box<dyn PromptSection>>,
}

impl SystemPromptBuilder {
    pub fn with_defaults() -> Self {
        Self { sections: vec![
            Box::new(IdentitySection),
            Box::new(ToolsSection),
            Box::new(SafetySection),
            Box::new(SkillsSection),
            Box::new(WorkspaceSection),
            Box::new(DateTimeSection),
            Box::new(RuntimeSection),
        ]}
    }
}
```

- 每个 section 独立实现 `PromptSection` trait
- 需要加新内容就加 section，不改 builder
- 整体 system prompt = 按顺序拼接所有 section 的输出

### PicoClaw — ContextBuilder 职责清晰

```go
// pkg/agent/context.go
type ContextBuilder struct {
    workspace    string
    skillsLoader *skills.SkillsLoader
    memory       *MemoryStore
    tools        *tools.ToolRegistry
}

func (cb *ContextBuilder) BuildSystemPrompt() string { ... }
func (cb *ContextBuilder) BuildMessages(history, summary, currentMessage, ...) []Message { ... }
```

- system prompt 和 messages 构建都在 ContextBuilder 里
- 拼装顺序：system prompt（identity + bootstrap + skills + memory）→ summary → history → current message

### OpenClaw — 独立函数

- `src/agents/pi-embedded-runner/system-prompt.ts` → `buildEmbeddedSystemPrompt(...)`
- `run/attempt.ts` 中调用并组合 hook 结果

---

## 5. Session 生命周期管理

| | 持久化方式 | 压缩机制 | 独立 Manager | 锁/并发控制 |
|---|---|---|---|---|
| **OpenClaw** | 文件 + session store（元数据） | 专门 `compact.ts` + overflow 重试 | ✅ 双层 | ✅ session lock |
| **PicoClaw** | 文件（JSON） | `maybeSummarize()` in loop | ✅ `SessionManager` | ❌ |
| **ZeroClaw** | 内存 `Vec<ConversationMessage>` | `trim_history()` + `auto_compact_history()` | ❌ | ❌ |

OpenClaw 的 session 治理最完善（适合多租户），ZeroClaw 最轻（单用户够用）。

---

## 6. 对 agent-demo 的启发

### 必须学的

1. **trait 按域分布** — 学 ZeroClaw，每个域（provider/tool/memory/session）在自己的 crate/module 里定义 trait
2. **PromptSection 可插拔** — 学 ZeroClaw 的 `SystemPromptBuilder`，system prompt 是可组合的 section 而不是一个大字符串
3. **ContextBuilder 独立** — 学 PicoClaw，上下文编排是独立模块，turn handler 只做调度
4. **System prompt 首轮构建后不变** — ZeroClaw 的做法，对 prompt cache 友好
5. **Session 完整生命周期** — 学 OpenClaw 的 session store（create/read/lock/compact/archive），多租户必须有

### 不需要学的

- OpenClaw 把核心循环委托给外部库 — 我们自己实现更可控
- ZeroClaw 内存态 session — 我们是多租户 SaaS，必须持久化

### 推荐组合

> **用 ZeroClaw 定义"干净接口层"，用 OpenClaw 补"生产级会话治理"，用 PicoClaw 保持"模块边界清晰"。**

---

## 源码路径速查

| 关注点 | ZeroClaw | PicoClaw | OpenClaw |
|---|---|---|---|
| Turn handler | `src/agent/agent.rs:467` | `pkg/agent/loop.go` | `src/agents/pi-embedded-runner/run/attempt.ts` |
| System prompt | `src/agent/prompt.rs` | `pkg/agent/context.go:111` | `src/agents/pi-embedded-runner/system-prompt.ts` |
| Provider trait | `src/providers/traits.rs` | `pkg/providers/types.go` | 外部 pi-coding-agent |
| Tool trait | `src/tools/traits.rs` | `pkg/tools/base.go` | `src/agents/*tools*` |
| Memory trait | `src/memory/traits.rs` | `pkg/agent/memory.go` | memory tool 生态 |
| Session manager | N/A（内存态） | `pkg/session/manager.go` | `src/config/sessions/store.ts` |
| Compaction | `trim_history()` | `maybeSummarize()` | `src/agents/pi-embedded-runner/compact.ts` |
