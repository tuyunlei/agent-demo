from orchestrator.greeter import greet


def test_greet():
    assert greet("Alice") == "Hello, Alice! Welcome to the agent team."
