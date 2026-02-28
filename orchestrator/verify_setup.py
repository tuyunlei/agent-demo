"""
Step 1 验证脚本：确认 OpenAI Agents SDK + Codex MCP Server 链路正常。
运行方式: python verify_setup.py  (不需要手动 source services.env)
"""

import asyncio
import os
import sys

import config  # noqa: F401 — 自动加载凭证 + 压日志

from agents import Agent, Runner
from agents.mcp import MCPServerStdio

# Codex CLI 需要 node 在 PATH 里
NODE_BIN = "/home/openclaw/.local/share/fnm/node-versions/v24.13.1/installation/bin"
if NODE_BIN not in os.environ.get("PATH", ""):
    os.environ["PATH"] = NODE_BIN + ":" + os.environ["PATH"]


async def main():
    print("=== Step 1: 环境验证 ===\n")
    ok = True

    # 1. API keys
    for key in ("OPENAI_API_KEY", "ANTHROPIC_API_KEY"):
        val = os.environ.get(key)
        if val:
            print(f"✅ {key} ({len(val)} chars)")
        else:
            print(f"❌ {key} 未设置")
            ok = False

    if not ok:
        sys.exit(1)

    # 2. Agents SDK 基础调用
    print("\n--- Agents SDK ---")
    agent = Agent(
        name="Ping",
        instructions="Reply with exactly: pong",
        model="gpt-4.1-nano",
    )
    result = await Runner.run(agent, "ping")
    print(f"✅ 基础调用: {result.final_output.strip()}")

    # 3. Codex MCP Server
    print("\n--- Codex MCP Server ---")
    async with MCPServerStdio(
        name="Codex",
        params={
            "command": "npx",
            "args": ["-y", "codex", "mcp-server"],
        },
        client_session_timeout_seconds=60,
    ) as codex_mcp:
        codex_agent = Agent(
            name="FileReader",
            instructions=(
                "Use the codex tool to read a file. "
                'Call codex with prompt "cat README.md", '
                'approval-policy "never", and sandbox "read-only". '
                "Return the file content."
            ),
            model="gpt-4.1-nano",
            mcp_servers=[codex_mcp],
        )
        result = await Runner.run(
            codex_agent,
            "Read the README.md file in the current directory",
        )
        output = result.final_output.strip()
        print(f"✅ Codex MCP: {output[:300]}")

    # 4. LiteLLM + Claude（验证 Anthropic key 可用）
    print("\n--- LiteLLM / Claude ---")
    claude_agent = Agent(
        name="ClaudePing",
        instructions="Reply with exactly: pong",
        model="litellm/anthropic/claude-sonnet-4-6",
    )
    result = await Runner.run(claude_agent, "ping")
    print(f"✅ Claude via LiteLLM: {result.final_output.strip()}")

    print("\n=== 全部验证通过 ✅ ===")


if __name__ == "__main__":
    asyncio.run(main())
