# CI 反馈

CI 红：SwiftLint `identifier_name` 规则要求变量名 3-40 字符。

## 需要修复

两个 `MockGRPCServer.swift`（AgentDemoTests + AgentDemoUITests）各 2 处：

1. `guard let p = _port.withLock(...)` → 改成 `guard let port = _port.withLock(...)`
2. `task.withLock { t in t?.cancel(); t = nil }` → 改成 `task.withLock { current in current?.cancel(); current = nil }`

共 4 处，改完 push 即可，CI 会自动重跑。
