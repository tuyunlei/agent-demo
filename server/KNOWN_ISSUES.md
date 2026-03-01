# 已知问题

改代码前先看这里，避免在有问题的基础上继续建设。

---

## 1. agent-context 是死 crate

**严重程度**：高 — 设计与实现脱节

agent-context 定义了完整的 system prompt 组合系统（`ContextBuilder`、`PromptSection`、`SystemPromptComposer`），包含身份、时区、安全、工具等多个 section。

**但没有任何 crate 依赖它。** `Cargo.toml` 中零外部引用。

实际的 system prompt 构建在 `agent-orchestrator/src/turn_compat.rs` 的 `build_system_prompt` 函数里硬编码完成，完全绕过了 agent-context 的设计。

**影响**：
- ~500 行精心设计的代码无人使用
- 新开发者看到 agent-context 会以为 system prompt 走这里，实际不是
- 设计意图（可插拔 section、token 预算管理）从未落地

**修复方向**：要么让 TurnExecutor 真正使用 ContextBuilder，要么删掉 agent-context 并在 turn_compat 里做好。二选一，不能两套并存。

---

## 2. agent-domain 影子 trait

**严重程度**：高 — 架构一致性问题

agent-domain 和 agent-llm 各自定义了一套 trait，概念重复：

| 概念 | agent-domain | agent-llm |
|------|-------------|-----------|
| LLM 调用 | `LlmProvider::generate` | `LlmProvider::complete` |
| 工具运行 | `ToolRuntime` | `ToolRuntime`（agent-tools） |
| 工具定义 | `ToolSpec`, `ToolCall` | `ToolSpec`, `ToolCall`（agent-tools） |

`OpenAiProvider` 同时实现两个 `LlmProvider` trait，中间用 `provider_compat` 转换层桥接。

**影响**：
- 新开发者不知道该用哪个 trait
- 转换层是纯胶水代码，不应该存在
- agent-domain 的版本是"影子 API"——被实现但不是真正驱动系统的

**修复方向**：统一到一个定义处，消除 provider_compat 转换层。

---

## 3. agent-channel 虚假 dev-dependencies

**严重程度**：中 — 测试依赖泄漏

agent-channel 的 `Cargo.toml` 中 dev-dependencies 包含 `agent-llm`、`agent-tools`、`agent-memory`。

Channel 层（六边形最外层）的测试不应该直接依赖具体适配器 crate。这意味着测试没有通过 trait 边界隔离，而是直接构造了内层的具体实现。

**影响**：
- 违反六边形架构的依赖规则（外层不应依赖内层具体实现）
- 测试与具体实现耦合，换适配器就要改测试
- 掩盖了 trait 接口是否真正可 mock 的问题

**修复方向**：Channel 测试应该用 mock 实现 trait，不引用具体适配器。如果 mock 困难，说明 trait 设计有问题，先修 trait。

---

## 4. TurnExecutor 构造暴露过多内部依赖

**严重程度**：中 — 接口设计问题

`TurnExecutor::new` 接受 5 个 `Arc<dyn Trait>` 参数：`LlmProvider`、`ToolRuntime`、`MessageStore`、`CompactionService`、`TurnExecutorConfig`。

调用者必须知道 TurnExecutor 内部需要哪些组件才能构造它，这是实现细节泄漏。

**影响**：
- 每次新增内部依赖，所有构造 TurnExecutor 的地方都要改
- Composition Root（agent-server）承担了过多的组装知识
- 测试需要构造全部依赖才能测试一个行为

**修复方向**：考虑 Builder pattern 或 Context/Config 结构体封装依赖。或者反过来审视：TurnExecutor 是否承担了过多职责需要拆分。

---

## 5. system prompt 硬编码在 turn_compat 中

**严重程度**：中 — 与问题 1 直接相关

`turn_compat.rs` 中的 `build_system_prompt` 函数直接用字符串拼接构建 system prompt，绕过了 agent-context 设计的整套组合系统。

```
turn_compat.rs::build_system_prompt  ←  实际使用
agent-context::ContextBuilder        ←  设计但未接入
```

**影响**：
- system prompt 内容和格式不可配置
- 无法按用户/场景动态组合不同 section
- 新增 prompt 内容要改 Rust 代码，重新编译

**修复方向**：与问题 1 一起解决。如果保留 agent-context，把 turn_compat 的逻辑迁移过去；如果删除 agent-context，至少把硬编码提取为可配置的模板。

---

## 6. AgentError 死代码

**严重程度**：低 — 仅 agent-domain 内部

`AgentError` 在 `agent-domain/src/ports.rs` 中定义，通过 `lib.rs` 导出，但没有任何外部 crate 引用它。仅在 `ports_tests.rs` 中被测试。

**修复方向**：确认是否有使用计划。如果没有，删除。

---

*最后更新：2026-03-02*
