# TM4.1: GRDB + SQLite 本地消息缓存

## 目标

用 GRDB + SQLite 做本地消息缓存，实现离线查看聊天历史。

## 验收标准

- [ ] GRDB 依赖引入（SPM）
- [ ] 本地数据库 schema：messages 表（id, session_id, role, content, timestamp）
- [ ] MessageStore protocol + GRDB 实现
- [ ] 收到服务端消息时写入本地
- [ ] 启动时先加载本地缓存，再异步拉取服务端增量
- [ ] 离线时能查看已缓存的历史消息
- [ ] 单元测试：写入、读取、增量同步逻辑
- [ ] SwiftLint + SwiftFormat 通过

## 上下文

- 设计文档：`docs/design/foundation.md`（基础层设计，含持久化方案）
- 现有消息加载：`ChatViewModel.loadHistory()` → `SessionServiceClient`
- 消息模型：`AgentDemo/ViewModels/ChatMessage.swift`

## 约束

- 不要用 CoreData 或 Realm，用 GRDB
- 数据库文件放 Application Support 目录
- schema migration 用 GRDB 的 DatabaseMigrator
