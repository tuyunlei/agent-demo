# ADR-005: Agent Runtime 核心流程

**日期**：2026-02-16
**状态**：已决定

## 核心流程

```
用户消息
  → ① 接收：验证身份，定位 agent/session
  → ② 上下文组装：system prompt + persona + summary + 最近 N 条 + 工具列表
  → ③ 调 LLM
  → ④ 如果 LLM 返回 tool_call → 执行工具 → 把结果塞回 context → 回到 ③
  → ⑤ LLM 返回最终文字回复
  → ⑥ 持久化所有消息（用户/assistant/tool_call/tool_result）
  → ⑦ 流式返回给客户端
  → ⑧ 异步后处理（检查是否需要 compaction、更新 session 等）
```

④→③ 可能循环多次（agent 连续调多个工具）。

## 失败处理策略

两层分离：
- **工具执行失败**：把失败结果（错误信息）作为 tool_result 交给 LLM，让 LLM 决定怎么处理（换个工具、告诉用户、重试）
- **LLM 本身失败**：降级机制（fallback 到备用 LLM provider）→ 都失败则给用户错误提示

## 设计要点

- 工具调用循环上限：暂不设，后续按需加（配置项，不影响架构）
- 流式回复 + 工具调用配合：MVP 不做中间状态反馈，架构上预留（如"正在搜索..."）
- 客户端工具：流程中 ④ 需要区分 server-side / client-side tool（见 note-004）
