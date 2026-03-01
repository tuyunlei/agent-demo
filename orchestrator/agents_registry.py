from enum import Enum


class AgentRole(Enum):
    PLANNER = "Task decomposition and coordination"
    DEVELOPER = "Code implementation via Codex MCP"
    REVIEWER = "Code review and quality validation"
