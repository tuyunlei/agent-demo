# Brief: D02 — Crate 划分设计

## 任务目标

设计 `agent-demo/server` 的 Cargo workspace crate 结构：每个 crate 叫什么、
负责什么、依赖哪些 crate。确保依赖方向只能由外向内，不出现循环依赖。

## 输出位置

`server/docs/design/crate-structure.md`

新建该文件，markdown 格式。

## 成功标准

1. 列出所有 crate 名称 + 一句话职责描述
2. 明确每个 crate 的依赖关系（画出依赖图，文本 ASCII 即可）
3. 每个 crate 属于哪一层（Domain / Application / Infrastructure / Protocol）
4. 没有循环依赖，依赖方向严格由外向内
5. 能回答：新增一个 LLM provider 实现，只需要改哪个 crate？

## 必要上下文

**架构分层（三层）**：
```
Infrastructure Layer（最外层）
  ├── agent-grpc        gRPC 服务实现，tonic 框架
  ├── agent-storage     PostgreSQL 读写，sqlx
  ├── agent-llm         LLM provider 实现（OpenAI 等）
  └── agent-channel     Channel Adapter 实现（gRPC / WebSocket）

Application Layer（中间层）
  └── agent-app         Use Cases：SendMessage、Subscribe 等业务流程编排
                        持有并注入所有 Domain Port 的实现

Domain Layer（最内层，零外部依赖）
  ├── agent-domain      核心实体（Session、Message、Persona、Memory）
  │                     + 所有 Port trait 定义（LlmProvider、MessageStore 等）
  └── agent-types       跨层共享的纯数据类型（AgentEvent、ChatError 等）
                        不含业务逻辑，只含 struct/enum + serde
```

**S01 结论（已确认）**：
- Port 注入方式：`Arc<dyn PortTrait + Send + Sync>`
- Port trait 统一使用 `#[async_trait]`

**需要注意的设计点**：
- `agent-types` 是最底层的共享类型包，连 domain 都可以依赖它，避免循环
- `agent-domain` 只依赖 `agent-types`，不依赖任何 infra 或框架
- `agent-app` 依赖 domain 的 Port trait，持有 `Arc<dyn Port>`，但不依赖具体实现
- Composition Root（在 `agent-grpc` 或单独的 `agent-server` 入口）负责装配所有依赖

**参考文件**：
- `server/docs/design/principles.md`（架构原则）
- `server/docs/design/overview.md`（架构总览）

## 约束

- 不创建任何实际目录或代码文件，只写设计文档
- 不需要列出每个 crate 内部的模块细节，只到 crate 粒度
- crate 数量不要过多（6-8 个为宜），避免过度拆分
- 不要引入 event bus / message queue 等当前 MVP 不需要的组件
