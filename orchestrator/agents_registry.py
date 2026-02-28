from enum import Enum


class AgentRole(Enum):
    PM = "Project manager coordinating scope and priorities"
    DEV = "Developer implementing code changes"
    REVIEWER = "Reviewer validating quality and correctness"
    ARCHITECT = "Architect defining system-level design decisions"
