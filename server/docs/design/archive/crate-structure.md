# Rust Workspace Crate 划分设计（D02）

## 1. 设计目标

在 `agent-demo/server` 中建立**稳定、可演进、无环**的 crate 结构，满足：

- 依赖方向严格由外向内（Hexagonal）
- 核心业务不依赖框架/存储/协议实现
- 新增 Provider/Channel 时尽量以“新增实现”为主，而非修改核心

---

## 2. Crate 列表（8 个）

> 控制在 6-8 个范围内，当前方案为 8 个，覆盖 Domain / Application / Infrastructure / Protocol 四层。

| Crate | 层级 | 一句话职责 |
|---|---|---|
| `agent-types` | Domain（基础类型） | 跨层共享的纯数据类型与错误类型（仅 struct/enum + serde） |
| `agent-domain` | Domain | 核心实体与领域规则；定义所有 Port trait（如 `LlmProvider`、`MessageStore`） |
| `agent-app` | Application | Use Case 编排层；通过 `Arc<dyn PortTrait + Send + Sync>` 注入依赖并驱动业务流程 |
| `agent-proto` | Protocol | `.proto` 编译产物与协议 DTO；只负责传输契约，不含业务逻辑 |
| `agent-storage` | Infrastructure | PostgreSQL/sqlx 适配器，实现 `agent-domain` 定义的存储相关 Port |
| `agent-llm` | Infrastructure | 各 LLM provider 适配器，实现 `LlmProvider` Port |
| `agent-channel` | Infrastructure | Channel Adapter（gRPC/WebSocket 等）统一封装，请求/事件与应用层对接 |
| `agent-server` | Infrastructure（Binary Entry + Composition Root） | binary 入口、依赖装配（composition root）、进程启动；协议无关 |

---

## 3. 依赖关系（逐 crate）

## 3.1 最内层

- `agent-types`
  - 依赖：无（或仅 serde / thiserror 等基础库）

- `agent-domain`
  - 依赖：`agent-types`
  - 不依赖：`agent-app` / 任一基础设施 crate / tonic / sqlx

## 3.2 应用层

- `agent-app`
  - 依赖：`agent-domain`, `agent-types`
  - 说明：只依赖 Port trait，不依赖任何具体实现

## 3.3 协议层

- `agent-proto`
  - 依赖：`agent-types`（用于协议与内部共享类型映射）
  - 不依赖：`agent-app` / `agent-domain`（避免协议反向污染核心）

## 3.4 基础设施层

- `agent-storage`
  - 依赖：`agent-domain`, `agent-types`

- `agent-llm`
  - 依赖：`agent-domain`, `agent-types`

- `agent-channel`
  - 依赖：`agent-app`, `agent-domain`, `agent-types`
  - 说明：作为外部渠道与应用层之间的适配层

- `agent-server`
  - 依赖：`agent-proto`, `agent-app`, `agent-channel`, `agent-storage`, `agent-llm`, `agent-domain`, `agent-types`
  - 说明：
    - 作为 binary 入口
    - 作为 Composition Root 装配 `Arc<dyn PortTrait>`
    - 启动进程与运行时
    - 不关心底层协议；gRPC 只是 `agent-channel` 内的一个 ChannelAdapter 实现

---

## 4. ASCII 依赖图（无环）

```text
                         +----------------------+
                         |     agent-server     |
                         | (entry + composition)|
                         +----------+-----------+
                                    |
          +-------------------------+--------------------------+
          |                         |                          |
          v                         v                          v
   +-------------+          +---------------+           +--------------+
   | agent-proto |          | agent-channel |           |  agent-llm   |
   +------+------+          +-------+-------+           +------+-------+
          |                         |                          |
          |                         v                          |
          |                  +-------------+                   |
          +----------------->|  agent-app  |<------------------+
                             +------+------+                   |
                                    |                          |
                                    v                          v
                              +-----------+              +-----------+
                              |agent-domain|<-------------|agent-storage|
                              +-----+-----+              +-----------+
                                    |
                                    v
                               +---------+
                               |agent-types|
                               +---------+
```

> 检查结果：图中不存在回边（back edge），因此依赖图为 DAG（有向无环图）。

---

## 5. 分层归属总结

- **Domain**：`agent-types`, `agent-domain`
- **Application**：`agent-app`
- **Protocol**：`agent-proto`
- **Infrastructure**：`agent-storage`, `agent-llm`, `agent-channel`, `agent-server`

---

## 6. 关键设计说明

### 6.1 为什么 `agent-types` 放在最底层

`agent-types` 作为“纯数据共享包”，允许 domain/app/infra/protocol 同时依赖，避免在 `agent-domain` 与 `agent-proto` 之间互相引用造成循环。

### 6.2 为什么 Composition Root 应放在 `agent-server`

`agent-server` 承担 binary 入口 + Composition Root：在 `main()` 中装配所有 Port 实现并启动服务。这里不应以具体协议命名；底层协议（gRPC/WebSocket/Push）属于 `agent-channel` 的 ChannelAdapter 实现，替换协议不应影响组合逻辑。

### 6.3 Port 注入策略（与 S01 对齐）

`agent-app` 通过：

- `Arc<dyn PortTrait + Send + Sync>`
- `#[async_trait]` 定义异步 Port

来依赖抽象而不是实现，保证外层可替换。

---

## 7. 扩展性问答

## 新增一个 LLM provider，实现时需要改哪个 crate？

**主改 `agent-llm`。**

典型改动：

1. 在 `agent-llm` 新增 `XxxProvider`，实现 `agent-domain::LlmProvider` trait。
2. 在 `agent-server` 的装配处（composition root）增加 provider 选择/注册配置。

`agent-domain` 与 `agent-app` 无需改动（除非新增了跨 provider 的全新抽象能力）。

---

## 8. 约束核对（Brief Checklist）

- [x] 列出所有 crate 名称 + 职责
- [x] 明确依赖关系 + ASCII 依赖图
- [x] 标注 Domain / Application / Infrastructure / Protocol 归属
- [x] 依赖方向外向内，且无循环依赖
- [x] 明确回答新增 LLM provider 的修改范围
