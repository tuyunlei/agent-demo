# orchestrator — Multi-Agent Developer Team

基于 OpenAI Agents SDK + Codex MCP 的多 agent 协作开发系统。

## 架构

```
涂涂 → 小小涂（机制层）
              ↓
     ┌── Planner（Sonnet 4.6）── 任务拆解 + 协调
     ├── Developer（gpt-5.3-codex）── 代码实现（Codex MCP）
     └── Reviewer（gpt-5.3-codex）── 代码审查（只读）
```

- 事件驱动：agent 间通过 `send_message` + asyncio.Queue 通信
- Planner 是中心节点，Developer/Reviewer 互不直接通信
- LiteLLM 统一多模型接口

## 依赖

- Python 3.10+
- openai-agents[litellm]（编排 + 多模型）
- Codex CLI（代码执行，MCP server 模式）

## 使用

```bash
cd orchestrator
source .venv/bin/activate
python main.py "实现 Sandbox trait"
```

## 进度

见 `ROADMAP.md`。
