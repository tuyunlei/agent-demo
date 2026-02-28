# orchestrator — Multi-Agent Dev Team

基于 OpenAI Agents SDK + Codex MCP 的项目迭代 agent team。

## 架构

```
OpenAI Agents SDK（编排）
  ├── PM Agent（任务拆解 + 分配）
  ├── Dev Agent（代码实现，通过 Codex MCP）
  ├── Reviewer Agent（代码审查，只读）
  └── Architect Agent（全局架构审计，只读）
```

## 依赖

- Python 3.10+
- openai-agents + litellm（编排 + 多模型支持）
- Codex CLI（代码执行层，作为 MCP server）

## 使用

```bash
cd orchestrator
source .venv/bin/activate
python main.py "实现 Sandbox trait"
```
