# Provider 高阶能力调研

**调研时间**：2026-02-28
**目的**：评估主流 LLM Provider 在服务端状态管理、内置工具、上下文压缩等方面的能力现状，为 agent-demo 架构调整提供决策依据。

---

## 1. 服务端状态管理（Stateful Conversation）

### 1.1 概念

传统 Chat Completions API 是无状态的——每次请求必须发送完整消息历史。Stateful API 由 provider 服务端维护对话状态，客户端只需发送增量消息 + 一个指向上一轮的 ID。

### 1.2 各家实现

| Provider | API | 状态管理机制 | 状态 |
|---|---|---|---|
| **OpenAI** | Responses API | `previous_response_id` 链式引用；`Conversations API` 持久化对话对象（跨 session/设备） | GA |
| **Google** | Interactions API | `previous_interaction_id`；服务端保留对话历史 | GA (2025.12) |
| **xAI** | Responses API | `previous_response_id` + `store_messages=True` | GA |
| **Anthropic** | Messages API | ❌ 无服务端状态，每次重发完整历史 | 无 |
| **火山引擎** | Chat API | 无服务端状态；有"上下文缓存 API"可缓存固定前缀（类似 prompt cache） | 部分 |

### 1.3 共同模式

三家有状态 API（OpenAI、Google、xAI）的共同设计：

- **请求只含增量**：新用户消息 + `previous_xxx_id`
- **服务端拼接上下文**：provider 负责把历史拼回去
- **参数是 interaction-scoped**：system prompt、tools、temperature 等每次请求独立指定，不随状态持久化
- **可选回退**：随时可以不传 ID、发完整历史（stateless 模式）
- **OpenAI 特有**：Conversations API 是独立的持久化对象，比 `previous_response_id` 更重（有自己的 CRUD 接口）

### 1.4 关键细节

**OpenAI**:
- `previous_response_id`：轻量链式，适合单次对话续接
- `Conversations API`：独立对话对象 `conv_xxx`，可跨 session/设备复用，有 items（messages/tool_calls/tool_outputs）
- `store: true` 时 response 被持久化，才能用 previous_response_id 续接
- 即使使用 previous_response_id，所有之前的 input tokens 仍按 input 计费（不是免费的）
- 社区反馈：stateful 模式延迟可能比 stateless 高（需要服务端加载历史）

**Google**:
- Interactions API 是统一的 model + agent 接口
- `previous_interaction_id` 只保留对话历史（输入输出），不保留 tools/system_instruction 等参数
- 可同时使用 stateful 和 stateless 模式

**xAI**:
- 接口与 OpenAI Responses API 高度兼容
- 内置工具（web_search、x_search）的状态也会被保留
- 后续 turn 不需要重新声明相同的 tools

**Anthropic**:
- 明确无服务端状态管理
- 通过 prompt caching 降低重复 token 成本（自动缓存前缀匹配）
- 通过 Compaction API（见第 3 节）管理长对话

### 1.5 对架构的影响

- Stateless 是所有 provider 的基线能力，必须作为默认模式
- Stateful 是增强能力，LlmProvider 需要能声明和使用
- 需要回退机制：system prompt 变更、工具集变更、手动上下文编辑等场景需要回退到 stateless
- Token 计费方面：OpenAI stateful 仍然计费所有历史 input tokens，优势在于带宽和 cache，不在于计费

---

## 2. 内置工具（Provider-Side Tools）

### 2.1 概念

传统工具调用：模型输出 tool_call → 客户端执行 → 客户端回传 tool_result → 模型继续。
内置工具：模型输出 tool_call → **provider 服务端直接执行** → 结果直接注入上下文 → 模型继续。客户端不参与执行。

### 2.2 各家内置工具清单

**OpenAI** (Responses API)
| 工具 | 功能 | 执行位置 |
|---|---|---|
| `web_search` | 联网搜索 | Provider |
| `file_search` | 向量检索（Vector Store） | Provider |
| `code_interpreter` | 沙箱 Python 执行 | Provider |
| `computer_use` | 屏幕操控 | 客户端（Provider 发指令） |
| `image_generation` | 生成图片 | Provider |
| `remote MCP` | 连接外部 MCP 服务器 | 外部 |

**Anthropic**
| 工具 | 功能 | 执行位置 |
|---|---|---|
| `web_search` | 联网搜索 + 自动引用 | Provider |
| `code_execution` | 沙箱代码执行 | Provider |
| `computer_use` | 屏幕操控 | 客户端（Provider 发指令） |
| `text_editor` | 文件编辑 | 客户端 |
| `bash` | Shell 命令 | 客户端 |
| `memory` | 跨会话记忆 | Provider |
| `tool_search` | 动态工具发现（defer_loading） | Provider |
| `MCP connector` | 连接 MCP 服务器 | 外部 |

**xAI**
| 工具 | 功能 | 执行位置 |
|---|---|---|
| `web_search` | 联网搜索 | Provider |
| `x_search` | X/Twitter 全量搜索 | Provider |
| `code_execution` | 代码执行 | Provider |

**Google Gemini**
| 工具 | 功能 | 执行位置 |
|---|---|---|
| `google_search` | Google 搜索 grounding | Provider |
| `code_execution` | 代码执行 | Provider |
| `google_search_retrieval` | 搜索增强检索 | Provider |

**火山引擎**
| 工具 | 功能 | 执行位置 |
|---|---|---|
| 知识库检索 | 类 file_search | Provider |
| 联网搜索插件 | 联网搜索 | Provider |

### 2.3 工具执行位置分类

从调研中可以归纳出 **五种** 工具执行位置：

1. **Provider-side**（provider 服务端执行）
   - web_search、code_execution、file_search、image_generation
   - 客户端不参与执行，结果直接注入上下文
   - 对客户端透明

2. **Server-side**（我们的 server 执行）
   - 传统自定义工具：模型输出 tool_call → 我们执行 → 回传 result
   - 当前设计覆盖的场景

3. **Client-side**（终端用户设备执行）
   - 客户端 App 上的能力（位置、相机、本地文件）
   - 需要 server → client App 的通道

4. **Remote service**（外部服务执行）
   - MCP 服务器、第三方 API
   - 通过标准协议（MCP/HTTP）连接

5. **Delegated agent**（委托其他 agent 执行）
   - A2A（Agent-to-Agent）协议
   - 一个 agent 作为另一个 agent 的工具

### 2.4 Anthropic 高阶工具特性

**Tool Search Tool（工具动态发现）**
- 问题：大量工具定义（50+ tools = 55K+ tokens）吃掉上下文
- 方案：`defer_loading: true` 标记工具，Claude 通过 tool_search 按需发现
- 效果：上下文占用从 ~77K 降到 ~8.7K（减少 85%）
- 准确率提升：Opus 4 从 49% → 74%，Opus 4.5 从 79.5% → 88.1%

**Programmatic Tool Calling（代码化工具调用）**
- 问题：每次工具调用都需要完整推理 pass，中间结果堆积
- 方案：Claude 在代码执行环境中直接调用工具，用代码做循环/条件/转换
- 实例：Claude for Excel 用此方式操作数千行数据

**Tool Use Examples（工具使用示例）**
- JSON schema 定义了结构，但无法表达使用模式
- 通过 `input_examples` 字段提供使用示例

### 2.5 对架构的影响

- 当前 ToolRuntime 假设所有工具在我们的 server 执行，需要扩展
- Provider 内置工具不走 tool_call → execute → result 的客户端循环
- 需要区分工具的执行位置，Tool trait 或 ToolRuntime 需要知道"谁来执行"
- Tool Search 机制值得借鉴：当工具数量增长时，动态发现比全量注入更高效
- MCP 作为工具连接标准值得关注，但 MVP 阶段不需要

---

## 3. 服务端上下文压缩（Server-Side Compaction）

### 3.1 概念

长对话逼近上下文窗口上限时，provider 服务端自动摘要旧内容，用摘要替代原始历史。

### 3.2 各家实现

| Provider | 功能 | 机制 | 状态 |
|---|---|---|---|
| **Anthropic** | Compaction API | `context_management.edits` 参数；触发阈值可配（默认 150K tokens）；返回 `compaction` block | Beta（compact-2026-01-12） |
| **OpenAI** | Compaction（Responses API） | 类似机制 | 有提及，细节待确认 |
| **其他** | — | 无原生支持 | — |

### 3.3 Anthropic Compaction 详情

- **支持模型**：Claude Opus 4.6、Claude Sonnet 4.6
- **工作方式**：
  1. 开启 `compact_20260112` 策略
  2. 输入 tokens 超过触发阈值时自动摘要
  3. 返回 `compaction` block（包含摘要）
  4. 后续请求中，API 自动丢弃 compaction block 之前的旧消息
- **可配参数**：
  - `trigger`：触发阈值（最低 50K tokens）
  - `pause_after_compaction`：摘要后是否暂停（便于检查）
  - `instructions`：自定义摘要 prompt
- **使用场景**：长时间聊天、工具密集的 agentic 任务
- **关键**：这不只是 token 管理——长上下文下模型注意力分散，压缩能提升输出质量

### 3.4 对架构的影响

- 当前设计中的 CompactionService / MemoryProvider 可以有两种实现：
  - **自建实现**：ContextBuilder 自己做 summary + truncate（当前设计）
  - **委托 Provider**：使用 Anthropic Compaction API 或类似能力
- 两者可以共存：provider 支持时委托，不支持时自建 fallback
- Anthropic 的 compaction 是 append-only 友好的——摘要作为 block 出现在 response 里，自然追加到消息流

---

## 4. 其他值得关注的能力

### 4.1 Prompt Caching / Context Caching

| Provider | 机制 | 特点 |
|---|---|---|
| **Anthropic** | 自动 prompt cache | 前缀匹配，相同前缀的后续请求读缓存（5 分钟 TTL），读缓存 token 费用降 90% |
| **OpenAI** | Automatic caching | 匹配前缀自动缓存，读缓存降 50%~75% |
| **Google** | 显式 Context Caching API | 手动创建 cache 对象（指定 TTL），适合大文档反复查询 |
| **火山引擎** | 上下文缓存 API | 类似 Google，手动创建缓存 |

### 4.2 Background / Async Execution

- **OpenAI**：Background mode — 异步执行长任务，完成后回调
- **Google**：Interactions API 支持异步 agent 执行

### 4.3 Structured Outputs

- **OpenAI**：`response_format: { type: "json_schema", schema: {...} }` 保证输出严格匹配 JSON Schema
- **Anthropic**：通过 tool_use 间接实现（tool 定义即 schema）
- **Google**：`response_schema` 参数

---

## 5. 总结：对当前架构设计的关键启示

### 5.1 LlmProvider 需要能力声明 + 模式切换

当前 `LlmProvider` trait 只有 `complete()` 和 `stream()`。需要扩展：

- **能力声明**：supports_stateful、supports_builtin_tools、supports_compaction 等
- **Stateful 模式**：`complete_stateful(msg, previous_id) -> (response, response_id)`
- **回退机制**：stateful 不可用或需要重建上下文时，回退到 stateless

### 5.2 工具执行位置需要建模

当前假设所有工具在 server 执行。实际有五种位置：

1. Provider-side（内置工具，server 不参与）
2. Server-side（当前设计覆盖）
3. Client-side（终端用户设备，需要 server→client 通道）
4. Remote service（MCP/API，server 代理调用）
5. Delegated agent（A2A）

ToolRuntime 或 Tool trait 需要知道执行位置，编排层需要根据位置选择不同的执行路径。

### 5.3 Compaction 可以分层

- Provider 原生支持时：委托 provider（Anthropic Compaction API）
- Provider 不支持时：自建（当前 ContextBuilder 设计的 summary + marker）
- 两者可以共存，通过 CompactionService 抽象统一

### 5.4 工具动态发现值得借鉴

Anthropic 的 Tool Search 机制在工具数量增长后非常有价值。可以在 ToolRuntime 层面引入类似概念：
- 核心工具始终加载
- 扩展工具按需发现
- 减少 system prompt 中的工具定义 token 消耗

### 5.5 Append-Only 上下文模型的适配

如果上下文严格 append-only，stateful API 是天然适配的——每轮只追加新消息。需要特殊处理的场景：
- System prompt 变更 → 需要重建完整上下文（回退 stateless）
- 手动编辑/删除历史 → 需要回退到 stateless
- Compaction → 摘要替代旧历史（provider-side compaction 自然处理，自建需要特殊编排）

---

*调研完成：2026-02-28*
