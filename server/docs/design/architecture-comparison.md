# AI Agent 架构对比报告（ZeroClaw vs OpenClaw vs agent-demo）

> 调研范围：核心代码与架构文档（优先 src/crates 与 design docs），聚焦架构哲学、分层、核心抽象、Channel 抽象、LLM 抽象、扩展机制。

---

## 1) ZeroClaw（Rust）

代码位置：`(internal reference)`

## 1.1 整体架构哲学

**结论：偏“模块化单体 + Ports 风格 trait 抽象”，不是严格六边形分 crate。**

- 在 `src/` 内按子系统拆分（`agent/`, `channels/`, `providers/`, `tools/`, `memory/` 等）
- 通过 Rust trait 抽象关键边界（`Channel`, `Provider`, `Tool`, `Memory` 等）
- 但核心/应用/基础设施不做物理隔离（同一个主 crate 内），因此是“**有 DIP 思路的工程化单体**”，不是强约束的 Hexagonal 包结构。

## 1.2 分层结构（目录）

```text
zeroclaw/
├── src/
│   ├── agent/        # Agent loop / dispatcher / prompt 等
│   ├── channels/     # 各消息渠道 + Channel trait
│   ├── providers/    # 各 LLM provider + Provider trait
│   ├── tools/        # 工具集合 + Tool trait
│   ├── memory/       # 记忆接口与实现
│   ├── security/ runtime/ gateway/ ...
│   └── main.rs       # CLI + 运行时编排入口
└── crates/
    └── robot-kit/    # 独立子crate，复用 Tool trait风格
```

## 1.3 核心抽象（关键 trait）

- `src/tools/traits.rs`
  - `pub trait Tool { name/description/parameters_schema/execute/spec }`
- `src/channels/traits.rs`
  - `pub trait Channel { send/listen/health_check/start_typing/... }`
- `src/providers/traits.rs`
  - `pub trait Provider { chat/chat_with_history/chat_with_tools/capabilities/... }`
  - 提供 `ProviderCapabilities` + `ToolsPayload`（Gemini/Anthropic/OpenAI/PromptGuided）
- `src/agent/dispatcher.rs`
  - `ToolDispatcher`：支持 XML prompt-tool 与 Native tool calling 两种分发策略

## 1.4 Channel 抽象

- 统一 `Channel` trait（`send`, `listen`）
- `channels/mod.rs` 聚合多渠道实现：telegram/discord/slack/signal/irc/... 
- 各渠道实现同一 trait，消息以 `ChannelMessage` / `SendMessage` 统一建模

## 1.5 LLM 抽象

- 统一 `Provider` trait
- `providers/mod.rs` 注册多个 provider（anthropic/openai/openrouter/gemini/ollama/bedrock...）
- provider 能力显式声明：是否原生 tool-calling、是否 vision
- tool calling 兼容两路：
  1) 原生 API tool calling（structured）
  2) prompt-guided fallback（XML `<tool_call>`）

## 1.6 扩展机制（工具/插件）

- 主要扩展方式是**新增 trait 实现 + 注册**（`tools/mod.rs` / `providers/mod.rs` / `channels/mod.rs`）
- 工具注册是代码级装配（`all_tools` 等函数），不是独立插件市场式 runtime 动态加载
- 优点：类型安全、性能稳定；代价：运行时热插拔能力较弱

---

## 2) OpenClaw（Node.js/TypeScript）

代码位置：`(internal reference)`

## 2.1 整体架构哲学

**结论：插件化平台架构（Plugin-first）+ 统一 Dock/Registry 抽象。**

- 核心不是“在主程序里硬编码渠道/provider”，而是“注册到插件注册表”
- `plugins/loader.ts` + `plugins/registry.ts` 负责发现、校验、装配插件
- channel/provider/tool/hook/command/http route 都可以由插件注册

## 2.2 分层结构（目录）

```text
openclaw/src/
├── agents/           # 运行器、工具策略、model config、session/subagent
├── channels/         # channel registry + dock + plugin channel contract
│   └── plugins/      # channel 插件接口与各 adapter 能力分面
├── providers/        # provider oauth/auth helper（部分通过模型配置系统接入）
├── plugins/          # 插件发现/加载/注册中心/hook 生命周期
├── gateway/ infra/ routing/ sessions/ ...
└── telegram/discord/slack/... # 具体渠道实现模块
```

## 2.3 核心抽象（关键 interface/type）

- Channel 核心：
  - `channels/plugins/types.plugin.ts` → `ChannelPlugin`
  - `channels/plugins/types.core.ts` / `types.adapters.ts` → 把 config、outbound、status、security、pairing、directory 等拆成可组合 adapter
- Plugin 核心：
  - `plugins/types.ts` → `OpenClawPluginApi`（registerTool/registerChannel/registerProvider/registerHook/...）
  - `plugins/registry.ts` → 统一保存 `tools/channels/providers/hooks/httpRoutes/...`
  - `plugins/loader.ts` → 发现 + schema 校验 + enable/disable + 实例化
- Agent/Tool 侧：
  - `agents/pi-tools.ts` 统一组装工具（内建 + channel-owned + plugin tools）

## 2.4 Channel 抽象

- “Channel 插件 + Dock”双层：
  1) `ChannelPlugin` 描述能力与适配器
  2) `channels/dock.ts` 提供轻量统一入口（共享逻辑依赖 dock，不直接依赖重型实现）
- 支持多账号、线程回复、mentions、group policy、message action 等细粒度能力
- 相比单一 `Channel` trait，OpenClaw 的 channel contract 更“分面化（faceted）”

## 2.5 LLM 抽象

- 模型/provider 配置集中在 `agents/models-config.providers.ts` 等
- 支持大量 provider 的发现/归一化与认证策略（api key + oauth）
- provider 也可插件注册（`registerProvider`）
- Agent runtime 偏“模型目录 + 策略 + 认证轮换”，而非单一 Provider interface

## 2.6 扩展机制（工具/插件）

- **强插件化**：工具、channel、provider、hook、命令、HTTP handler/route、后台 service 都可注册
- 支持插件配置 schema 与启停策略，具备较强运行时组合能力
- 代价：系统复杂度更高，理解成本和一致性约束要求更高

---

## 3) agent-demo（Rust，设计中）

文档位置：`server/docs/design/`

## 3.1 整体架构哲学

**结论：明确且严格的 Hexagonal（Ports & Adapters）+ 分 crate 物理边界。**

- `principles.md` 明确“依赖只能外向内、核心不依赖框架/SDK”
- `crate-structure.md` 明确 8 crate 分层与无环依赖 DAG
- 相比 ZeroClaw/OpenClaw，agent-demo 在“架构意图显式化、约束可审计性”上最强

## 3.2 分层结构（crate 级）

```text
agent-demo/server
├── agent-types     # 纯共享类型
├── agent-domain    # 核心领域 + Port traits
├── agent-app       # UseCase编排
├── agent-proto     # 协议DTO/proto产物
├── agent-storage   # 存储adapter (实现domain ports)
├── agent-llm       # LLM adapter (实现domain ports)
├── agent-channel   # 渠道adapter (gRPC/WS...)
└── agent-server    # composition root + binary入口
```

依赖图是严格 DAG，符合 Hexagonal + DIP。

## 3.3 核心抽象（Port）

`docs/design/ports/core-ports.md` 已定义核心 Port：

- `LlmProvider`
- `MessageStore`
- `SessionStore`
- `MemoryStore`
- `PersonaStore`

统一约束：
- `#[async_trait]`
- `Arc<dyn Trait + Send + Sync>` 注入
- 统一 `DomainError`
- 接口层强制多租户边界（`user_id`）

## 3.4 Channel 抽象

- 当前定位：`agent-channel` 作为统一 channel adapter 层，协议细节留在外层
- 文档中强调 MVP 可先 App/gRPC，后续扩 Telegram/WS
- 设计理念是“入口协议可替换，不侵入 app/domain”

## 3.5 LLM 抽象

- `LlmProvider` Port 统一流式事件：`TextDelta / ToolCallStart / ToolCallDelta / ToolCallEnd / Usage / Done`
- 支持 model capability 探测（`supports_model`）
- 明确把 provider 差异关在 adapter 内

## 3.6 扩展机制（工具/插件）

- 当前是“Port + adapter 扩展”路径（新增实现，少改核心）
- 还未定义 OpenClaw 那种运行时插件总线（registerTool/registerHook/...）
- 优点：边界干净、可测性强；代价：动态扩展能力暂弱

---

## 4) 三方对比（重点）

| 维度 | ZeroClaw (Rust) | OpenClaw (TS) | agent-demo (Rust, 设计中) |
|---|---|---|---|
| 架构哲学 | 模块化单体 + trait抽象 | 插件优先平台架构 | 严格六边形 + 分crate |
| 分层方式 | 目录分模块（同crate） | 目录分模块 + registry/plugin runtime | crate 物理分层（DAG） |
| 核心抽象形态 | `Tool/Channel/Provider` trait | `ChannelPlugin` + `OpenClawPluginApi` + 各类 adapter types | `Port trait`（Llm/Store/Persona等） |
| Channel 模型 | 单一 `Channel` trait + 多实现 | 分面化 channel contract（config/outbound/status/threading...） | `agent-channel` adapter 层（概念清晰，细节待收敛） |
| LLM 模型 | `Provider` trait + native/prompt工具双路 | 模型目录 + provider配置 + auth轮换 + 可插件注册provider | `LlmProvider` 流式统一事件 + capability 探测 |
| 扩展机制 | 代码注册（编译期） | 运行时插件注册（tool/channel/provider/hook/route） | 预期 adapter 扩展（静态优先） |
| 类型安全/可控性 | 高 | 中（动态灵活） | 高 |
| 运行时灵活性 | 中 | 高 | 中（当前） |
| 架构约束显式性 | 中 | 中 | 高（principles+crate结构文档化） |

---

## 5) 关键差异总结 + 对 agent-demo 的建议

## 5.1 关键差异

1. **agent-demo vs ZeroClaw**
   - 共同点：Rust + trait 驱动抽象
   - 差异：agent-demo 的分层约束更强（crate 级），ZeroClaw 更偏工程实践与功能聚合

2. **agent-demo vs OpenClaw**
   - 共同点：都重视多渠道、多 provider、工具扩展
   - 差异：OpenClaw 强在运行时插件生态；agent-demo 强在架构纯度与可验证依赖边界

3. **ZeroClaw vs OpenClaw**
   - ZeroClaw：静态、安全、直观
   - OpenClaw：动态、平台化、扩展快但复杂度更高

## 5.2 agent-demo 可借鉴点（建议）

### A. 借鉴 OpenClaw：为 Port 体系增加“轻量插件注册层”

在不破坏 Hexagonal 的前提下，可增加一个 `agent-extension`（或放在 `agent-server`）用于：
- 注册 tool/provider/channel adapter
- 声明 metadata/capabilities
- 生命周期 hook（before/after llm/tool）

这样保留强分层，同时提升运行时扩展效率。

### B. 借鉴 OpenClaw 的“Channel 分面化”

agent-demo 当前 Channel 概念偏总线化。可提前把 channel 能力拆成小 trait：
- ConfigResolver
- OutboundSender
- ThreadingPolicy
- MentionPolicy
- AccountProbe

避免未来一个“万能 Channel trait”膨胀。

### C. 借鉴 ZeroClaw 的“双路 tool calling 兼容策略”

在 `LlmProvider` adapter 层明确：
- 优先 native tool calling
- fallback 到 prompt-guided

对新 provider 落地会更稳。

### D. 强化“组合根可观察性”

借鉴 OpenClaw 的 registry 思路（但保留静态类型）：
- 在 `agent-server` 启动时打印已注册 providers/channels/tools 清单
- 对冲突与缺失做 fail-fast

### E. 提前定义“扩展能力矩阵”

可参考 OpenClaw `capabilities` 思路，在 domain/types 中定义：
- channel capabilities（threads/media/reactions...）
- llm capabilities（tool/vision/stream/json-mode...）

让 app 层按能力决策，而不是硬编码 provider/channel 分支。

---

## 6) 结论（一句话）

- **ZeroClaw**：trait 抽象扎实，偏静态工程化单体；
- **OpenClaw**：插件化能力最强，平台化明显；
- **agent-demo**：架构原则最清晰、分层最严格，建议在保持 Hexagonal 纯度前提下，适度吸收 OpenClaw 的注册机制与分面抽象，以获得更好的“长期可演进 + 运行时扩展”平衡。
