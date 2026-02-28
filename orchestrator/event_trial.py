"""
事件驱动试验：Planner + Dev 通过消息通信。
"""

import asyncio
import json
import os
from datetime import datetime

import config  # noqa: F401

from agents import Agent, Runner, function_tool

NODE_BIN = "/home/openclaw/.local/share/fnm/node-versions/v24.13.1/installation/bin"
if NODE_BIN not in os.environ.get("PATH", ""):
    os.environ["PATH"] = NODE_BIN + ":" + os.environ["PATH"]

PROJECT_ROOT = os.path.expanduser("~/code/misc/agent-demo")

# ── 消息总线 ──
message_bus: asyncio.Queue = asyncio.Queue()
message_log: list[dict] = []  # 记录所有消息


@function_tool
def send_message(to: str, content: str) -> str:
    """Send a message to another agent or to Human.
    
    Args:
        to: Target agent name. One of: "Planner", "Dev", "Human"
        content: Message content
    """
    msg = {
        "to": to,
        "content": content,
        "time": datetime.now().strftime("%H:%M:%S"),
    }
    message_bus.put_nowait(msg)
    message_log.append(msg)
    return f"Message sent to {to}"


# ── Agent 定义 ──
from agents.mcp import MCPServerStdio


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
            model=config.kimi_model(),
            instructions=(
                "You are a developer. You receive coding tasks and implement them "
                "using the codex tool (approval-policy 'never', sandbox 'write-full').\n"
                f"Project root: {PROJECT_ROOT}\n\n"
                "When done, use send_message to report back to Planner with what you did.\n"
                "If you're stuck, use send_message to ask Planner for help.\n"
                "Do NOT reply with plain text — always communicate via send_message."
            ),
            mcp_servers=[codex_mcp],
            tools=[send_message],
        )

        planner_agent = Agent(
            name="Planner",
            model=config.kimi_model(),
            instructions=(
                "You are a task planner. You receive tasks from the user, "
                "break them down, and delegate to Dev via send_message.\n\n"
                "After Dev reports completion, decide if the task is done.\n"
                "If done, use send_message to Human with a summary.\n"
                "If not satisfied, send_message back to Dev with feedback.\n"
                "Do NOT reply with plain text — always communicate via send_message."
            ),
            tools=[send_message],
        )

        agents = {"Planner": planner_agent, "Dev": dev_agent}

        # ── 初始任务 ──
        task = (
            "Create a Python file orchestrator/greeter.py that has a function "
            "greet(name: str) -> str returning 'Hello, {name}! Welcome to the agent team.' "
            "Also create orchestrator/test_greeter.py with a test for it."
        )

        print(f"=== User Task ===\n{task}\n")

        # 把初始任务发给 Planner
        await message_bus.put({
            "to": "Planner",
            "content": f"User request: {task}",
            "time": datetime.now().strftime("%H:%M:%S"),
        })

        # ── Dispatcher Loop ──
        max_rounds = 10
        for round_num in range(1, max_rounds + 1):
            if message_bus.empty():
                print("\n📭 消息队列空了，流程结束")
                break

            msg = await message_bus.get()
            target = msg["to"]
            content = msg["content"]

            print(f"\n── Round {round_num}: → {target} ──")
            print(f"   {content[:200]}...")

            if target == "Human":
                print(f"\n🧑 Human 收到消息:\n{content}")
                break

            agent = agents.get(target)
            if not agent:
                print(f"❌ Unknown agent: {target}")
                continue

            # 记录当前 agent 名字，给 send_message 用
            result = await Runner.run(agent, content)
            # Runner.run 过程中 agent 会调用 send_message 往队列里塞新消息
            
            if result.final_output and result.final_output.strip():
                # 如果 agent 还是回了纯文本（没用 send_message），打印出来
                print(f"   ⚠️ {target} 直接回复: {result.final_output[:200]}")

        # ── 结果 ──
        print(f"\n=== 消息日志 ({len(message_log)} messages) ===")
        for i, msg in enumerate(message_log):
            # msg 里的 from 由 dispatcher 上下文推断
            print(f"  {i+1}. [{msg['time']}] → {msg['to']}: {msg['content'][:100]}")

        # 验证文件
        for fname in ["orchestrator/greeter.py", "orchestrator/test_greeter.py"]:
            path = os.path.join(PROJECT_ROOT, fname)
            if os.path.exists(path):
                print(f"\n✅ {fname}")
                print(open(path).read())
            else:
                print(f"\n❌ {fname} 不存在")


if __name__ == "__main__":
    asyncio.run(main())
