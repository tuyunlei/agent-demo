"""
最小试验 v2：PM (gpt-4.1) handoff 给 Dev (gpt-5.3-codex)。
"""

import asyncio
import os

import config  # noqa: F401

from agents import Agent, Runner
from agents.mcp import MCPServerStdio

NODE_BIN = "/home/openclaw/.local/share/fnm/node-versions/v24.13.1/installation/bin"
if NODE_BIN not in os.environ.get("PATH", ""):
    os.environ["PATH"] = NODE_BIN + ":" + os.environ["PATH"]

PROJECT_ROOT = os.path.expanduser("~/code/misc/agent-demo")


async def main():
    async with MCPServerStdio(
        name="Codex",
        params={
            "command": "npx",
            "args": ["-y", "codex", "mcp-server"],
            "cwd": PROJECT_ROOT,
            "env": {**os.environ, "CODEX_SANDBOX": "write-full"},
        },
        client_session_timeout_seconds=120,
    ) as codex_mcp:

        dev_agent = Agent(
            name="Dev",
            model="gpt-5.3-codex",
            instructions=(
                "You are a developer. You receive tasks and implement them "
                "using the codex tool. Always use approval-policy 'never' "
                "and sandbox 'write-full'.\n"
                f"Project root: {PROJECT_ROOT}\n"
                "After completing a task, report what you did concisely."
            ),
            mcp_servers=[codex_mcp],
        )

        pm_agent = Agent(
            name="PM",
            model="gpt-4.1",
            instructions=(
                "You are a project manager coordinating a dev team. "
                "You receive task descriptions and hand them off to the Dev agent. "
                "Keep your instructions to Dev clear and specific."
            ),
            handoffs=[dev_agent],
        )

        # 稍微复杂一点的任务：在项目里创建一个 Python 模块
        task = (
            "In the orchestrator/ directory, create a file called 'agents_registry.py' "
            "that defines a simple AgentRole enum with values: PM, DEV, REVIEWER, ARCHITECT. "
            "Each role should have a description string. "
            "Also create a test file 'test_agents_registry.py' that verifies all 4 roles exist."
        )

        print(f"=== Task ===\n{task}\n")
        result = await Runner.run(pm_agent, task)

        print(f"\n=== Result ===")
        print(f"Final agent: {result.last_agent.name}")
        print(f"Output:\n{result.final_output}")

        # 验证
        for fname in ["orchestrator/agents_registry.py", "orchestrator/test_agents_registry.py"]:
            path = os.path.join(PROJECT_ROOT, fname)
            exists = os.path.exists(path)
            print(f"\n{'✅' if exists else '❌'} {fname}")
            if exists:
                with open(path) as f:
                    content = f.read()
                print(f"   ({len(content)} chars)")


if __name__ == "__main__":
    asyncio.run(main())
