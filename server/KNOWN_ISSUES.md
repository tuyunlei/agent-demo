# 已知问题

改代码前先看这里，避免在有问题的基础上继续建设。

---

## agent-domain 影子 trait

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
