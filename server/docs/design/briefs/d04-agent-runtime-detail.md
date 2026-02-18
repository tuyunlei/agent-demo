# Brief: D04 — Agent Runtime 内部树形拆分

## 任务目标

在现有 `agent-runtime.md`（总览级别）基础上，写 Agent Runtime 的内部详细拆分文档：
把 Application Layer 内的 Agent Runtime 模块拆到 3-4 层子模块，
明确每个子模块的职责、接口边界、依赖关系，以及并发模型（基于 S03 actor 结论）。

## 输出位置

新建：`server/docs/design/core/agent-runtime-detail.md`

## 成功标准

1. 清楚展示 Agent Runtime 的树形子模块结构（至少 3 层）：
   - 第 1 层：Agent Runtime 整体（actor 边界）
   - 第 2 层：主要子模块（RequestDispatcher、ContextAssembler、ExecutionLoop、PostProcessor）
   - 第 3 层：ExecutionLoop 内部再拆（LlmCaller、ToolCoordinator、EventPublisher）
2. 每个子模块包含：
   - 一句话职责
   - 输入/输出（自然语言，不需要精确 Rust 签名）
   - 依赖哪些 Port
3. 明确并发模型：session actor 是并发边界，内部串行，基于 S03 结论
4. 画出子模块关系图（ASCII）
5. 明确：哪个子模块负责把 AgentEvent 推给 ChannelAdapter

## 必要上下文

**现有总览文档**：
`server/docs/design/core/agent-runtime.md`
（已有主流程描述，本文档是它的内部拆分补充，不要重复总览内容）

**S03 Tokio actor 结论**（必须体现在设计中）：
- 每个 session 对应一个 actor task，mailbox = bounded mpsc channel（容量 8）
- actor 内部串行处理：状态读取 → LLM 调用 → 存储写入 → 事件推送
- 不同 session 的 actor 并发运行
- idle timeout 自动退出，代际 ID 防竞态，SessionRegistry 管理生命周期
- back pressure：队列满时调用方 await（可配置 fast-fail 或超时）

**子模块设计参考**（基础框架，可调整）：

```
Agent Runtime（actor 边界）
├── SessionRegistry          管理所有 session actor 的生命周期
│   └── SessionHandle        对外暴露 process_message / shutdown API
│
├── RequestDispatcher        接收 InboundMessage，找到对应 session actor，发送命令
│
└── SessionActor（每 session 一个 task）
    ├── ContextAssembler     组装 LLM 请求上下文（历史、人格、记忆摘要）
    │                        Port：MessageStore, PersonaStore, MemoryStore
    │
    ├── ExecutionLoop        主推理循环（LLM → 工具 → 再 LLM）
    │   ├── LlmCaller        调用 LlmProvider，接收流式 token
    │   │                    Port：LlmProvider
    │   ├── ToolCoordinator  发出 ToolRequest，等待 SubmitToolResult
    │   │                    通过 pending map + oneshot 协调
    │   └── EventPublisher   把 LLM token / ToolRequest / RoundComplete 转换为
    │                        AgentEvent 并推给 ChannelAdapter
    │                        Port：ChannelAdapter
    │
    └── PostProcessor        一轮完成后的收尾
                             Port：MessageStore（持久化），MemoryStore（更新摘要）
```

**六边形架构约束**：
- SessionActor 在 Application Layer，不能直接依赖 Infrastructure 实现
- 所有外部调用都通过 Port（trait），组合根装配具体 Adapter

## 约束

- 不需要写精确的 Rust 类型签名（留给 D05 Port 精确定义）
- 不展开 ContextAssembler 的具体算法（context-management.md 已有）
- 不展开 LLM Gateway 内部（llm-gateway/overview.md 已有）
- 不展开工具系统内部（tool-system/overview.md 已有）
- 用中文写，ASCII 图可中英混用
