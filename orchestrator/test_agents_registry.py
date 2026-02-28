from agents_registry import AgentRole


def test_agent_roles_exist():
    expected = {"PM", "DEV", "REVIEWER", "ARCHITECT"}
    actual = {role.name for role in AgentRole}
    assert actual == expected
    assert len(AgentRole) == 4
