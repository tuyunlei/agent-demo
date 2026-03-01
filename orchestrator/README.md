# orchestrator — Multi-Agent Developer Team

Multi-agent collaborative development system based on OpenAI Agents SDK + Codex MCP.

## Architecture

```
TuTu → XiaoXiaoTu (mechanism layer)
              ↓
     ┌── Planner (Sonnet 4.6)── Task breakdown + coordination
     ├── Developer (gpt-5.3-codex)── Code implementation (Codex MCP)
     └── Reviewer (gpt-5.3-codex)── Code review (read-only)
```

- Event-driven: agents communicate via `send_message` + asyncio.Queue
- Planner is central node, Developer/Reviewer don't communicate directly
- LiteLLM unified multi-model interface

## Dependencies

- Python 3.10+
- openai-agents[litellm] (orchestration + multi-model)
- Codex CLI (code execution, MCP server mode)

## Usage

```bash
cd orchestrator
source .venv/bin/activate
python main.py "Implement Sandbox trait"
```

## Progress

See `ROADMAP.md`.
