"""
凭证加载 + 日志配置 + 模型工厂。
"""

import logging
import os
import re
from pathlib import Path

from openai import AsyncOpenAI

CREDENTIALS_FILE = Path("~/.openclaw/credentials/services.env").expanduser()

_REQUIRED_KEYS = {
    "OPENAI_API_KEY", "ANTHROPIC_API_KEY",
    "VOLCENGINE_CODING_BASE_URL", "VOLCENGINE_CODING_API_KEY",
}


def load_credentials() -> dict[str, str]:
    loaded = {}
    if not CREDENTIALS_FILE.exists():
        return loaded
    for line in CREDENTIALS_FILE.read_text().splitlines():
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        m = re.match(r"(?:export\s+)?([A-Z_]+)=(.*)", line)
        if m:
            key, value = m.group(1), m.group(2).strip("'\"")
            if key in _REQUIRED_KEYS:
                os.environ[key] = value
                loaded[key] = value
    return loaded


def setup_logging() -> None:
    class CodexEventFilter(logging.Filter):
        _pattern = re.compile(
            r"codex/event|Validation error|JsonRpcNotification|validation error"
        )
        def filter(self, record: logging.LogRecord) -> bool:
            return not self._pattern.search(record.getMessage())

    f = CodexEventFilter()
    for name in ("", "mcp", "openai.agents", "pydantic"):
        logging.getLogger(name).addFilter(f)


def kimi_model():
    """创建 Kimi K2.5 模型（通过 Volcengine Coding API）。"""
    from agents.models.openai_chatcompletions import OpenAIChatCompletionsModel
    client = AsyncOpenAI(
        base_url=os.environ["VOLCENGINE_CODING_BASE_URL"],
        api_key=os.environ["VOLCENGINE_CODING_API_KEY"],
    )
    return OpenAIChatCompletionsModel(model="kimi-k2.5", openai_client=client)


# 模块加载时自动执行
_loaded = load_credentials()
setup_logging()
