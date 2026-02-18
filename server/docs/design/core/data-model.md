# 数据模型详细设计（design/core/data-model.md）

## 1. 设计目标与边界

本文档基于 ADR-004 已确定的实体关系与基础结构，给出可落地的 PostgreSQL 数据模型详细设计，服务于 Rust + sqlx 后端。

**目标：**
- 满足 MVP（User:Agent = 1:1）并保留 1:N 扩展能力
- 强化多租户隔离（所有查询必须显式带 `user_id`）
- 支撑高频读写（会话消息流）与后续可扩展能力（分区、向量检索）
- 保持六边形架构：Domain 定义 Repository Port，Infrastructure 实现

**非目标：**
- 不给出 SQL DDL
- 不给出 Rust 代码实现
- 不设计缓存层
- 不涉及上下文组装策略

---

## 2. 实体关系图（ASCII）

```text
+-------------------+
|      users        |
+-------------------+
| id (PK, UUID)     |
| device_key        |
| invite_code       |
| created_at        |
+---------+---------+
          | 1:N
          |
+---------v---------+
|      agents       |
+-------------------+
| id (PK, UUID)     |
| user_id (FK)      |----+
| name              |    |
| persona_text      |    |
| persona_data      |    |
| created_at        |    |
| updated_at        |    |
+---------+---------+    |
          | 1:N          | 1:N
          |              |
+---------v---------+  +-v----------------+
|     sessions      |  |     memories     |
+-------------------+  +------------------+
| id (PK, UUID)     |  | id (PK, UUID)    |
| agent_id (FK)     |  | agent_id (FK)    |
| title             |  | content          |
| summary           |  | source           |
| created_at        |  | created_at       |
| updated_at        |  +------------------+
+---------+---------+
          | 1:N
          |
+---------v---------+
|     messages      |
+-------------------+
| id (PK, UUID)     |
| session_id (FK)   |
| role              |
| content           |
| tool_calls        |
| tool_result       |
| token_count       |
| created_at        |
+-------------------+
```

> 关系与 ADR-004 保持一致：`User 1:N Agent 1:N Session 1:N Message`，`Agent 1:N Memory`。

---

## 3. 表定义（字段 / 约束 / 用途）

以下为逻辑模型定义（非 DDL）。

### 3.1 users

| 字段 | 类型 | 约束 | 用途 |
|---|---|---|---|
| id | UUID | PK, NOT NULL | 用户主键，全局唯一标识 |
| device_key | BYTEA | NOT NULL, UNIQUE | 设备公钥/设备标识密钥（用于设备身份校验） |
| invite_code | VARCHAR(64) | NOT NULL | 邀请码（MVP 注册准入校验来源，建议保留审计值） |
| created_at | TIMESTAMPTZ | NOT NULL, DEFAULT now() | 用户创建时间 |

**补充约束说明：**
- `device_key` 需要唯一，避免一台设备重复绑定多个账户（MVP 假设）。
- `invite_code` 建议保留原值而非仅存引用，便于审计与运营追溯。

---

### 3.2 agents

| 字段 | 类型 | 约束 | 用途 |
|---|---|---|---|
| id | UUID | PK, NOT NULL | Agent 主键 |
| user_id | UUID | NOT NULL, FK -> users.id, ON DELETE CASCADE | 所属用户（多租户主隔离键） |
| name | VARCHAR(128) | NOT NULL | Agent 展示名 |
| persona_text | TEXT | NOT NULL | 人设自然语言描述（直接可读） |
| persona_data | JSONB | NOT NULL, DEFAULT '{}'::jsonb | 人设结构化配置（风格、规则、偏好等） |
| created_at | TIMESTAMPTZ | NOT NULL, DEFAULT now() | 创建时间 |
| updated_at | TIMESTAMPTZ | NOT NULL, DEFAULT now() | 最近更新时间 |

**补充约束说明：**
- MVP 可在应用层限制一个用户仅一个 agent；数据库层不强制唯一，预留 1:N。
- `updated_at` 在任何可变字段更新时刷新。

---

### 3.3 sessions

| 字段 | 类型 | 约束 | 用途 |
|---|---|---|---|
| id | UUID | PK, NOT NULL | 会话主键 |
| agent_id | UUID | NOT NULL, FK -> agents.id, ON DELETE CASCADE | 所属 agent |
| title | VARCHAR(256) | NOT NULL | 会话标题（列表展示） |
| summary | TEXT | NOT NULL, DEFAULT '' | 会话摘要（压缩/归纳结果） |
| created_at | TIMESTAMPTZ | NOT NULL, DEFAULT now() | 创建时间 |
| updated_at | TIMESTAMPTZ | NOT NULL, DEFAULT now() | 最近活跃时间/更新时间 |

**补充约束说明：**
- `updated_at` 用于“最近会话”排序。
- `summary` 默认空字符串，避免空值分支判断。

---

### 3.4 messages

| 字段 | 类型 | 约束 | 用途 |
|---|---|---|---|
| id | UUID | PK, NOT NULL | 消息主键 |
| session_id | UUID | NOT NULL, FK -> sessions.id, ON DELETE CASCADE | 所属会话 |
| role | VARCHAR(16) | NOT NULL, CHECK in ('user','assistant','system','tool') | 消息角色 |
| content | TEXT | NOT NULL | 消息正文 |
| tool_calls | JSONB | NOT NULL, DEFAULT '[]'::jsonb | assistant 发起的工具调用集合 |
| tool_result | JSONB | NOT NULL, DEFAULT '{}'::jsonb | 工具执行结果（tool 角色或 assistant 聚合结果） |
| token_count | INTEGER | NOT NULL, DEFAULT 0, CHECK >= 0 | 单条消息 token 计数（计费/观测） |
| created_at | TIMESTAMPTZ | NOT NULL, DEFAULT now() | 消息创建时间 |

**补充约束说明：**
- `tool_calls` 统一用数组（即使单调用），避免类型漂移。
- `tool_result` 统一对象结构，便于扩展 status/latency/error。
- 不做“role 与 JSON 字段联动”的数据库硬约束，避免过度复杂；在 Domain Service 做一致性校验。

---

### 3.5 memories

| 字段 | 类型 | 约束 | 用途 |
|---|---|---|---|
| id | UUID | PK, NOT NULL | 记忆主键 |
| agent_id | UUID | NOT NULL, FK -> agents.id, ON DELETE CASCADE | 所属 agent |
| content | TEXT | NOT NULL | 记忆内容 |
| source | VARCHAR(64) | NOT NULL | 记忆来源（如 conversation/manual/system） |
| created_at | TIMESTAMPTZ | NOT NULL, DEFAULT now() | 创建时间 |

**补充约束说明：**
- `source` 建议使用受控枚举值集合（应用层约束 + 文档约定）。

---

## 4. JSONB 字段 Schema 设计

> 目标：结构稳定、可演进、便于索引与诊断。以下为“推荐 JSON Schema 形态示例”（非强制数据库校验器实现）。

### 4.1 agents.persona_data

**语义：** 人设结构化配置，补充 `persona_text`，用于可控行为参数。

**建议结构（示例）：**

```json
{
  "version": "1.0",
  "profile": {
    "language": "zh-CN",
    "tone": "friendly",
    "verbosity": "medium"
  },
  "behavior_rules": [
    "优先给出可执行步骤",
    "涉及风险操作先确认"
  ],
  "capabilities": {
    "tool_use": true,
    "web_browsing": true,
    "code_assist": false
  },
  "safety": {
    "refuse_illegal": true,
    "privacy_level": "strict"
  },
  "preferences": {
    "timezone": "Asia/Shanghai",
    "units": "metric"
  },
  "metadata": {
    "last_editor": "user",
    "last_edited_at": "2026-02-17T01:00:00Z"
  }
}
```

**字段约束建议：**
- `version` 必填，用于后续 schema 演进。
- 允许附加字段（`additionalProperties=true`）以支持灰度扩展。

---

### 4.2 messages.tool_calls

**语义：** assistant 生成的工具调用计划与参数（一次消息可多次调用）。

**建议结构（示例）：**

```json
[
  {
    "call_id": "call_01HXYZ",
    "tool_name": "web_search",
    "arguments": {
      "query": "PostgreSQL partition best practices",
      "count": 5
    },
    "status": "requested",
    "requested_at": "2026-02-17T01:10:00Z"
  },
  {
    "call_id": "call_01HXYA",
    "tool_name": "web_fetch",
    "arguments": {
      "url": "https://example.com/article"
    },
    "status": "requested",
    "requested_at": "2026-02-17T01:10:02Z"
  }
]
```

**字段约束建议：**
- 顶层必须是数组。
- 每个元素至少包含：`call_id`、`tool_name`、`arguments`。
- `call_id` 在单条消息内唯一。

---

### 4.3 messages.tool_result

**语义：** 工具执行返回结果（可聚合多次调用结果），用于审计、回放、调试。

**建议结构（示例）：**

```json
{
  "version": "1.0",
  "results": [
    {
      "call_id": "call_01HXYZ",
      "status": "success",
      "output": {
        "items": [
          {
            "title": "Partitioning in PostgreSQL",
            "url": "https://example.com/p1"
          }
        ]
      },
      "error": null,
      "latency_ms": 842,
      "finished_at": "2026-02-17T01:10:01Z"
    },
    {
      "call_id": "call_01HXYA",
      "status": "failed",
      "output": null,
      "error": {
        "code": "TIMEOUT",
        "message": "request timeout after 10s",
        "retryable": true
      },
      "latency_ms": 10000,
      "finished_at": "2026-02-17T01:10:12Z"
    }
  ],
  "summary": {
    "total": 2,
    "success": 1,
    "failed": 1
  }
}
```

**字段约束建议：**
- `status` 枚举：`success | failed`。
- `output` 与 `error` 至少其一非空。
- `results[].call_id` 必须可回溯到 `tool_calls[].call_id`（应用层保证）。

---

## 5. 索引策略

> 原则：围绕“按租户过滤 + 时间序访问 + 关联链路稳定”设计，避免过度索引。

### 5.1 users

1. **PK(id)**，B-tree  
   - 用途：主键查找。
2. **UNIQUE(device_key)**，B-tree  
   - 场景：设备鉴权时按 device_key 查 user。

### 5.2 agents

1. **PK(id)**，B-tree
2. **INDEX(user_id, created_at DESC)**，B-tree  
   - 场景：查询某用户的 agent 列表（当前/未来多 agent）。
3. （可选）**INDEX(user_id, updated_at DESC)**，B-tree  
   - 场景：按最近更新的 agent 排序展示。

### 5.3 sessions

1. **PK(id)**，B-tree
2. **INDEX(agent_id, updated_at DESC)**，B-tree  
   - 场景：会话列表（最近活跃优先）。
3. **INDEX(agent_id, created_at DESC)**，B-tree  
   - 场景：按创建时间分页回溯历史会话。

### 5.4 messages

1. **PK(id)**，B-tree
2. **INDEX(session_id, created_at ASC)**，B-tree  
   - 场景：按会话拉取消息流（ADR-004 已指定关键索引）。
3. **INDEX(session_id, created_at DESC)**，B-tree  
   - 场景：取最近 N 条消息（倒序 limit 再反转）。
4. **INDEX(session_id, role, created_at DESC)**，B-tree（可选）  
   - 场景：筛选 tool/system 消息用于调试与审计。
5. **GIN(tool_calls jsonb_path_ops)**（可选）  
   - 场景：按工具名、call_id 做问题排查（低频运维查询）。
6. **GIN(tool_result jsonb_path_ops)**（可选）  
   - 场景：检索失败错误码、重试诊断。
7. **部分索引：INDEX(session_id, created_at DESC) WHERE role='user'**（可选）  
   - 场景：用户发言轨迹分析，高选择性时收益明显。

### 5.5 memories

1. **PK(id)**，B-tree
2. **INDEX(agent_id, created_at DESC)**，B-tree  
   - 场景：拉取最新记忆。
3. **INDEX(agent_id, source, created_at DESC)**，B-tree（可选）  
   - 场景：按来源筛选记忆（如仅 conversation）。

### 5.6 多租户 user_id 过滤与索引配合

由于 `sessions/messages/memories` 不直接存 `user_id`，租户过滤通过 join 路径实现：
- `messages -> sessions -> agents -> users`
- `memories -> agents -> users`

**为何不在 MVP 冗余 user_id：**
- 避免写入一致性负担（多处更新风险）
- 当前 join 深度固定且可被索引覆盖

**扩展预案：** 若出现高压读场景，可在 `sessions/messages/memories` 增加冗余 `user_id` 并以触发器/应用层双写保证一致性（见第 7 节）。

---

## 6. 核心查询模式（>= 8 个场景）

以下所有场景都要求请求上下文提供 `user_id`，并在查询条件中显式使用。

### 场景 1：根据 device_key 查用户（登录）
- 路径：`users`
- 条件：`device_key = ?`
- user_id 生效方式：查询结果产出 `user_id`，作为后续全部业务调用的租户上下文。

### 场景 2：获取当前用户的 Agent 列表
- 路径：`users -> agents`
- 条件：`agents.user_id = :user_id`
- 排序：`updated_at DESC` 或 `created_at DESC`
- user_id 生效方式：首层过滤，阻断跨租户读取。

### 场景 3：创建会话并归属到用户 Agent
- 路径：先校验 `agents(id, user_id)`，再写 `sessions`
- 条件：`agent_id = :agent_id AND user_id = :user_id`
- user_id 生效方式：写前授权校验，防止向他人 agent 写入会话。

### 场景 4：查询某用户最近会话列表
- 路径：`agents -> sessions`
- 条件：`agents.user_id = :user_id`
- 排序：`sessions.updated_at DESC`, limit/offset 或 cursor
- user_id 生效方式：通过 agent 归属进行租户过滤。

### 场景 5：获取某会话最近 50 条消息
- 路径：`agents -> sessions -> messages`
- 条件：`sessions.id = :session_id AND agents.user_id = :user_id`
- 排序：`messages.created_at DESC LIMIT 50`（应用层再正序）
- user_id 生效方式：在 session 所属链路校验，杜绝跨租户会话读取。

### 场景 6：写入用户消息/助手消息
- 路径：校验 `session -> agent -> user` 后写 `messages`
- 条件：`session_id = :session_id AND agents.user_id = :user_id`
- user_id 生效方式：写入前必须完成归属校验。

### 场景 7：更新会话摘要与更新时间
- 路径：`sessions`（带 `agents` 归属条件）
- 条件：`sessions.id = :session_id AND agents.user_id = :user_id`
- user_id 生效方式：防止更新到其他租户会话。

### 场景 8：获取某 Agent 的最新 Memory
- 路径：`agents -> memories`
- 条件：`memories.agent_id = :agent_id AND agents.user_id = :user_id`
- 排序：`created_at DESC`
- user_id 生效方式：agent 归属过滤。

### 场景 9：新增/删除 Memory
- 路径：校验 `agent.user_id` 后写/删 `memories`
- 条件：`agent_id = :agent_id AND user_id = :user_id`
- user_id 生效方式：写操作租户授权。

### 场景 10：按工具失败码排查消息（运维）
- 路径：`agents -> sessions -> messages`
- 条件：`agents.user_id = :user_id` + `tool_result` JSON 条件
- user_id 生效方式：即便是诊断查询，也必须先按租户过滤。

---

## 7. 分区与扩展预案

### 7.1 Message 分区策略

#### 分区目标
- 控制单表膨胀导致的索引退化与 vacuum 压力
- 提升历史数据归档与维护效率

#### 推荐策略
- **一级策略：按时间 Range 分区（按月）**，分区键 `created_at`
- 原因：
  - 消息是时间序写入，天然适配时间分区
  - 最近消息热读集中，历史分区可冷处理
  - 便于按时间窗口运维（归档、压缩、备份）

#### 触发条件（满足任一即可启动）
1. `messages` 总行数 > 5,000 万
2. 单表索引总大小 > 80 GB
3. 高峰期 P95 “按 session 拉取最近消息” 查询 > 150ms 且持续 7 天
4. autovacuum 无法在业务窗口内稳定追平

#### 迁移路径（无停机/低停机）
1. 新建分区母表与未来分区（不改业务接口）
2. 新写入双写到新分区结构（短期）或通过路由切换
3. 历史数据按时间批次回填
4. 校验行数、校验采样一致性
5. 切流到分区表
6. 下线旧表（保留只读观察期）

#### 分区后索引原则
- 每个分区保留 `(session_id, created_at)` 本地索引
- 谨慎增加 JSONB GIN（仅在确有诊断收益时）

---

### 7.2 pgvector 接入预案

#### 目标场景
- 记忆检索（Memory semantic search）
- 长会话语义召回（Message semantic retrieval）

#### Schema 变更建议（增量）
1. `memories` 增加向量列（如 `embedding`）与 `embedding_model`、`embedded_at`
2. 可选新增 `message_embeddings` 独立表：
   - `id`, `message_id`, `session_id`, `agent_id`, `user_id(可选冗余)`, `embedding`, `model`, `created_at`
   - 优点：解耦主消息写路径，便于异步补算与重建
3. 新增向量索引（IVFFlat 或 HNSW，按版本能力与数据量选择）

#### 演进策略
- MVP 阶段不强依赖向量字段
- 采用“异步补全 embedding”模式，避免阻塞主对话写入
- 检索查询仍强制带 `user_id` 过滤（先过滤租户，再做向量近邻）

---

## 8. Repository Port 设计（Domain 层）

> 以下为“接口语义定义”，不包含任何具体语言实现。

### 8.1 UserRepository

1. **create_user(device_key, invite_code) -> User**
   - 语义：创建用户。
   - 错误：`DeviceKeyAlreadyExists`、`InvalidInviteCode`、`PersistenceError`。

2. **get_user_by_id(user_id) -> User?**
   - 语义：按主键查用户。
   - 错误：`PersistenceError`。

3. **get_user_by_device_key(device_key) -> User?**
   - 语义：登录鉴权使用。
   - 错误：`PersistenceError`。

---

### 8.2 AgentRepository

1. **create_agent(user_id, name, persona_text, persona_data) -> Agent**
   - 错误：`UserNotFound`、`ValidationError`、`PersistenceError`。

2. **get_agent_by_id(user_id, agent_id) -> Agent?**
   - 语义：带租户边界读取。
   - 错误：`PersistenceError`。

3. **list_agents(user_id, page) -> AgentList**
   - 语义：分页获取用户 agent。
   - 错误：`PersistenceError`。

4. **update_agent_profile(user_id, agent_id, name?, persona_text?, persona_data?) -> Agent**
   - 语义：部分更新，并刷新 `updated_at`。
   - 错误：`AgentNotFound`、`ValidationError`、`ConcurrentModification`、`PersistenceError`。

5. **delete_agent(user_id, agent_id) -> void**
   - 语义：删除 agent（级联删除 sessions/messages/memories）。
   - 错误：`AgentNotFound`、`PersistenceError`。

---

### 8.3 SessionRepository

1. **create_session(user_id, agent_id, title) -> Session**
   - 错误：`AgentNotFoundOrForbidden`、`ValidationError`、`PersistenceError`。

2. **get_session_by_id(user_id, session_id) -> Session?**
   - 语义：按租户+会话读取。
   - 错误：`PersistenceError`。

3. **list_sessions(user_id, agent_id, page, sort=updated_desc) -> SessionList**
   - 错误：`AgentNotFoundOrForbidden`、`PersistenceError`。

4. **update_session_meta(user_id, session_id, title?, summary?) -> Session**
   - 语义：更新标题/摘要并刷新 `updated_at`。
   - 错误：`SessionNotFoundOrForbidden`、`ValidationError`、`ConcurrentModification`、`PersistenceError`。

5. **delete_session(user_id, session_id) -> void**
   - 语义：删除会话（级联删除 messages）。
   - 错误：`SessionNotFoundOrForbidden`、`PersistenceError`。

---

### 8.4 MessageRepository

1. **append_message(user_id, session_id, role, content, tool_calls, tool_result, token_count) -> Message**
   - 语义：追加单条消息；在事务中校验 session 归属。
   - 错误：`SessionNotFoundOrForbidden`、`ValidationError`、`PersistenceError`。

2. **list_messages(user_id, session_id, cursor/page, limit, order) -> MessageList**
   - 语义：按时间窗口读取消息（支持顺序/倒序）。
   - 错误：`SessionNotFoundOrForbidden`、`InvalidPagination`、`PersistenceError`。

3. **get_latest_messages(user_id, session_id, limit) -> MessageList**
   - 语义：高频路径，返回最近 N 条。
   - 错误：`SessionNotFoundOrForbidden`、`PersistenceError`。

4. **delete_message(user_id, session_id, message_id) -> void**（可选管理能力）
   - 错误：`MessageNotFoundOrForbidden`、`PersistenceError`。

---

### 8.5 MemoryRepository

1. **create_memory(user_id, agent_id, content, source) -> Memory**
   - 错误：`AgentNotFoundOrForbidden`、`ValidationError`、`PersistenceError`。

2. **list_memories(user_id, agent_id, page, source?) -> MemoryList**
   - 错误：`AgentNotFoundOrForbidden`、`PersistenceError`。

3. **get_memory_by_id(user_id, agent_id, memory_id) -> Memory?**
   - 错误：`PersistenceError`。

4. **delete_memory(user_id, agent_id, memory_id) -> void**
   - 错误：`MemoryNotFoundOrForbidden`、`PersistenceError`。

---

### 8.6 跨 Repository 一致性约束

1. **租户约束统一**
   - 所有读写接口必须显式接收 `user_id`。
   - `NotFound` 与 `Forbidden` 对外可折叠为统一错误，避免泄露存在性。

2. **事务边界**
   - 消息写入与会话 `updated_at` 更新应在同一事务中完成。

3. **错误模型统一**
   - 分层：`Validation` / `NotFoundOrForbidden` / `Conflict` / `Persistence`。

4. **可观测性字段**
   - 对关键写路径记录 `request_id`（应用日志维度，不强制入库）。

---

## 9. 多租户硬约束落地清单

为满足“所有数据查询必须带 user_id”，实施以下约束：

1. Repository Port 方法签名强制包含 `user_id`（除 `create_user`、`get_user_by_device_key`）。
2. Infrastructure 查询模板统一使用“租户前置过滤”条件。
3. 禁止提供仅凭 `session_id/message_id` 的裸查询接口。
4. 管理后台与运维查询同样要求租户范围（除平台级审计专用通道）。
5. 代码评审检查项：任何新查询若缺 `user_id` 过滤则拒绝合并。

---

## 10. 验收对照（Checklist）

- [x] 实体关系图完整且与 ADR-004 一致
- [x] 每个表每个字段包含类型、约束、用途说明
- [x] 所有 JSONB 字段给出完整 schema 示例
- [x] 索引策略覆盖核心查询并解释原因
- [x] 查询模式覆盖 10 个核心场景（>=8）
- [x] 每个查询路径明确说明 `user_id` 生效方式
- [x] 分区策略包含触发条件与迁移路径
- [x] Repository Port 覆盖 CRUD + 核心查询
- [x] 文档不含 SQL DDL 或 Rust 代码

---

## 11. 后续演进建议（保持 YAGNI）

1. MVP 阶段保持当前 5 表模型，不新增冗余列。
2. 当消息规模达到分区阈值时再执行分区迁移。
3. 当语义检索成为刚需时再引入 pgvector 与 embedding pipeline。
4. 在不破坏 Domain Port 的前提下演进 Infrastructure 实现，保持上层业务无感。
