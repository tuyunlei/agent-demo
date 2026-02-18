# ADR-004: 数据模型设计

**日期**：2026-02-16
**状态**：已决定

## 背景

需要设计核心实体的表结构和关系，支撑用户管理、agent 人格、会话、消息、记忆等功能。

## 决策

### 实体关系

```
User 1:N Agent 1:N Session 1:N Message
                Agent 1:N Memory
```

- MVP 先 User:Agent = 1:1，但表结构用 agent_id 关联，未来支持 1:N
- MVP 先单会话，但 Session 表支持多会话
- Session = 用户视角的"一个对话"，不是上下文窗口。压缩是透明的后台操作

### 表结构

**User**
- id (UUID), device_key (BYTEA), invite_code (VARCHAR), created_at

**Agent**
- id (UUID), user_id → User, name, persona_text (TEXT), persona_data (JSONB), created_at, updated_at

**Session**
- id (UUID), agent_id → Agent, title, summary (TEXT, 压缩后的历史摘要), created_at, updated_at

**Message**
- id (UUID), session_id → Session, role ('user'/'assistant'/'system'/'tool'), content (TEXT), tool_calls (JSONB), tool_result (JSONB), token_count (INT), created_at

**Memory**
- id (UUID), agent_id → Agent, content (TEXT), source (VARCHAR), created_at

### 关键索引

- `messages (session_id, created_at)` — 按会话查消息，按时间排序
- 所有表的查询都带 user_id 条件（通过 join 或冗余字段），为未来分片做准备

## 考虑过的替代方案

- **每个 Session 一个 Message 表**：运维噩梦，动态建表、无法跨 session 查询。否决。
- **Session = 上下文窗口（压缩后新建 session）**：用户不应感知压缩，session 应是用户视角的概念。否决。
- **MongoDB 等文档型数据库**：数据结构规整，PG + JSONB 能覆盖灵活需求，无需额外引入。否决。

## 存储 vs 上下文

存储和 context 是分开的两件事：
- **存储**：所有消息都持久化，用户能翻看完整历史
- **Context**：发给 LLM 时只取 summary + 最近 N 条消息（上下文管理策略单独讨论）

## 扩展路径

- 消息量大时：PG 原生分区表（PARTITION BY），应用代码不用改
- 需要缓存时：加 Redis 缓存活跃会话
- 需要向量搜索时：pgvector 扩展
