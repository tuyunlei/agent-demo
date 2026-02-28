# PostgreSQL 适配器设计（D-INFRA-02）

**状态**：Draft（MVP 可实施）  
**更新时间**：2026-02-24  
**所属层**：Layer 4 Infrastructure  
**目标实现位置**：`crates/agent-storage`  

**关联文档**：

- `docs/design/architecture.md`
- `docs/design/core/event-model.md`
- `docs/design/capabilities/session-lifecycle.md`
- `docs/design/decisions/002-postgresql.md`
- `crates/agent-storage/migrations/20260222000002_create_sessions_messages.sql`
- `crates/agent-storage/migrations/20260222000003_add_message_sequence.sql`

---

## 1. 设计目标

### 1.1 适配器定位

PostgreSQL 适配器属于 Layer 4。

它的职责是：

- 实现 Layer 3 定义的 `EventStore` trait。
- 把事件流可靠落地到 PostgreSQL。
- 提供会话元数据的读写能力。
- 不引入业务规则与流程编排。

### 1.2 不做的事情

本适配器**不负责**：

- 判断“是否要压缩会话”。
- 判断“是否要归档会话”。
- 决定工具循环次数。
- 拼装 LLM prompt。
- 执行任何工具逻辑。

以上逻辑属于 Layer 2（Orchestration）。

### 1.3 与分层架构对齐

依赖方向必须满足：

- Layer 4 实现 Layer 3 trait。
- Layer 4 不依赖 Layer 2 concrete 逻辑。
- Layer 2 不依赖 Layer 4 concrete 类型。

因此本设计强调：

- `PostgresEventStore` 只暴露 trait 行为。
- SQL 细节封装在 adapter 内部。
- 错误映射为 `EventStoreError`。

### 1.4 PostgreSQL 特性利用目标

MVP 需要主动利用 PG 的如下能力：

- `JSONB`：存储多类型事件 payload。
- 事务：批量追加事件原子提交。
- 索引：支撑按 session 范围读取。
- 约束：保证序号唯一与数据一致性。

### 1.5 与 ADR-002 一致性

ADR-002 已决策主存储为 PostgreSQL。

本设计继承该约束：

- 单实例起步。
- 可平滑扩展索引与分区策略。
- 查询尽量带 `user_id` / `tenant_id` 维度。

### 1.6 迁移兼容目标

当前系统已有 `messages` 表。

MVP 目标不是“推倒重来”，而是：

- 在不破坏现有路径前提下引入 `events`。
- 提供新旧 schema 过渡方案。
- 支持逐步切换读路径到事件流。

---

## 2. 数据库 Schema 设计

### 2.1 设计原则

Schema 设计遵循三条原则：

1. 以事件为中心（event-centric）。
2. 查询模式驱动索引，而非预先过度索引。
3. 以最小字段集支撑 `EventStore` trait。

### 2.2 events 表（对应 EventEnvelope）

`events` 是事件流主表。

每一行是一条不可变事件记录。

#### 2.2.1 字段定义

- `event_id UUID PRIMARY KEY`
- `session_id UUID NOT NULL`
- `sequence_number BIGINT NOT NULL`
- `event_type VARCHAR(64) NOT NULL`
- `payload JSONB NOT NULL`
- `tenant_id UUID NOT NULL`
- `user_id UUID NOT NULL`
- `created_at TIMESTAMPTZ NOT NULL`

#### 2.2.2 语义约束

- `event_id`：全局幂等键。
- `session_id + sequence_number`：会话内严格单调。
- `event_type`：payload 类型路由键。
- `payload`：具体事件内容。
- `tenant_id`：多租户隔离。
- `user_id`：用户维度过滤与分片预留。

### 2.3 sessions 表（对应 Session）

`sessions` 承载会话元数据。

会话正文与事实历史在 `events`。

#### 2.3.1 字段建议（MVP）

基于现有 `sessions` 增量扩展：

- `id UUID PRIMARY KEY`
- `tenant_id UUID NOT NULL`
- `user_id UUID NOT NULL`
- `agent_id TEXT NOT NULL`
- `title TEXT NOT NULL DEFAULT ''`
- `status TEXT NOT NULL DEFAULT 'active'`
- `version BIGINT NOT NULL DEFAULT 0`
- `event_count BIGINT NOT NULL DEFAULT 0`
- `last_sequence BIGINT NOT NULL DEFAULT 0`
- `estimated_prompt_tokens INTEGER`
- `compacted_until_sequence BIGINT`
- `created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()`
- `updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()`
- `last_message_at TIMESTAMPTZ`
- `archived BOOLEAN NOT NULL DEFAULT FALSE`

#### 2.3.2 与现有 summary 字段关系

当前 `sessions.summary` 可暂时保留。

在事件流模型下：

- `Summary` 事件是真相源。
- `sessions.summary` 应降级为缓存投影。
- 新逻辑不依赖其一致性。

### 2.4 users 表（保持现有）

`users` 表无需大改。

建议仅确认两点：

- 主键类型与 `events.user_id` 一致（UUID）。
- 若未来引入 `tenant_id` 到 users，需同步 FK 策略。

MVP 可保持现有 users 结构不变。

### 2.5 外键与完整性约束

建议约束：

- `events.session_id REFERENCES sessions(id)`
- `events.user_id REFERENCES users(id)`（可选，视多租户映射策略）
- `UNIQUE(session_id, sequence_number)`

### 2.6 索引设计（查询模式驱动）

#### 2.6.1 events 必备索引

1) 会话顺序读取：

- `UNIQUE INDEX ux_events_session_seq ON events(session_id, sequence_number)`

2) 最近事件读取（倒序）：

- `INDEX idx_events_session_seq_desc ON events(session_id, sequence_number DESC)`

3) 租户隔离过滤：

- `INDEX idx_events_tenant_session ON events(tenant_id, session_id)`

4) 用户过滤（审计/列表）：

- `INDEX idx_events_user_created ON events(user_id, created_at DESC)`

#### 2.6.2 sessions 必备索引

1) 用户会话列表：

- `INDEX idx_sessions_user_updated ON sessions(user_id, updated_at DESC)`

2) 租户 + 用户：

- `INDEX idx_sessions_tenant_user_updated ON sessions(tenant_id, user_id, updated_at DESC)`

3) 状态过滤：

- `INDEX idx_sessions_user_archived ON sessions(user_id, archived)`

### 2.7 SQL DDL 伪代码（完整）

```sql
-- sessions（在现有基础上演进）
CREATE TABLE IF NOT EXISTS sessions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL,
    user_id UUID NOT NULL REFERENCES users(id),
    agent_id TEXT NOT NULL DEFAULT '',
    title TEXT NOT NULL DEFAULT '',
    summary TEXT NOT NULL DEFAULT '',
    status TEXT NOT NULL DEFAULT 'active',
    version BIGINT NOT NULL DEFAULT 0,
    event_count BIGINT NOT NULL DEFAULT 0,
    last_sequence BIGINT NOT NULL DEFAULT 0,
    estimated_prompt_tokens INTEGER,
    compacted_until_sequence BIGINT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_message_at TIMESTAMPTZ,
    archived BOOLEAN NOT NULL DEFAULT FALSE,
    CHECK (status IN ('active', 'compacting', 'archived'))
);

CREATE INDEX IF NOT EXISTS idx_sessions_user_updated
    ON sessions(user_id, updated_at DESC);

CREATE INDEX IF NOT EXISTS idx_sessions_tenant_user_updated
    ON sessions(tenant_id, user_id, updated_at DESC);

CREATE INDEX IF NOT EXISTS idx_sessions_user_archived
    ON sessions(user_id, archived);

-- events（新增）
CREATE TABLE IF NOT EXISTS events (
    event_id UUID PRIMARY KEY,
    session_id UUID NOT NULL REFERENCES sessions(id),
    sequence_number BIGINT NOT NULL,
    event_type VARCHAR(64) NOT NULL,
    payload JSONB NOT NULL,
    tenant_id UUID NOT NULL,
    user_id UUID NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CHECK (sequence_number > 0)
);

CREATE UNIQUE INDEX IF NOT EXISTS ux_events_session_seq
    ON events(session_id, sequence_number);

CREATE INDEX IF NOT EXISTS idx_events_session_seq_desc
    ON events(session_id, sequence_number DESC);

CREATE INDEX IF NOT EXISTS idx_events_tenant_session
    ON events(tenant_id, session_id);

CREATE INDEX IF NOT EXISTS idx_events_user_created
    ON events(user_id, created_at DESC);

-- 可选：只在确认需要 JSONB 过滤时再加
-- CREATE INDEX idx_events_payload_gin ON events USING GIN (payload jsonb_path_ops);
```

---

## 3. EventStore Trait 实现

### 3.1 实现原则

实现策略：

- 方法语义与 trait 一一对应。
- SQL 简单可审计。
- 在事务中保证关键原子性。

### 3.2 append_events

语义：

- 给某个 session 批量追加事件。
- 在同一事务内分配连续 sequence。
- 保证 `session_id + sequence_number` 唯一。

SQL 伪代码：

```sql
BEGIN;

-- 1) 锁会话行并读取 last_sequence
SELECT id, tenant_id, user_id, version, last_sequence, event_count
FROM sessions
WHERE id = $1
FOR UPDATE;

-- 2) 应用层按 last_sequence 生成本批 sequence_number
--    例如: next = last_sequence + row_number

-- 3) 批量插入 events
INSERT INTO events (
    event_id,
    session_id,
    sequence_number,
    event_type,
    payload,
    tenant_id,
    user_id,
    created_at
)
VALUES
    (...), (...), (...);

-- 4) 更新 sessions 统计与版本
UPDATE sessions
SET last_sequence = $new_last_sequence,
    event_count = event_count + $batch_size,
    updated_at = NOW(),
    version = version + 1
WHERE id = $1;

COMMIT;
```

实现注记：

- 事务级别可用 `READ COMMITTED` + `FOR UPDATE`。
- 唯一冲突时映射为 `SequenceConflict`。
- 幂等重试依赖 `event_id` 主键冲突识别。

### 3.3 read_events(range)

语义：

- 读取指定 session 的区间事件。
- 按 `sequence_number ASC` 返回。

SQL 伪代码：

```sql
SELECT event_id,
       session_id,
       sequence_number,
       event_type,
       payload,
       tenant_id,
       user_id,
       created_at
FROM events
WHERE session_id = $1
  AND sequence_number BETWEEN $2 AND $3
ORDER BY sequence_number ASC;
```

扩展：

- 若 `start/end` 为空，可动态改写条件。
- 也可使用 `>=` / `<=` 组合。

### 3.4 read_recent_events(limit)

语义：

- 取最近 N 条。
- 先倒序 limit，再在应用层反转为升序。

SQL 伪代码：

```sql
SELECT event_id,
       session_id,
       sequence_number,
       event_type,
       payload,
       tenant_id,
       user_id,
       created_at
FROM events
WHERE session_id = $1
ORDER BY sequence_number DESC
LIMIT $2;
```

返回处理：

- 结果集反转后给上层。
- 确保消费端总是看到升序事件流。

### 3.5 get_session

语义：

- 按 `session_id` 读取会话元数据。

SQL 伪代码：

```sql
SELECT id,
       tenant_id,
       user_id,
       agent_id,
       title,
       status,
       version,
       event_count,
       last_sequence,
       estimated_prompt_tokens,
       compacted_until_sequence,
       created_at,
       updated_at,
       last_message_at,
       archived
FROM sessions
WHERE id = $1;
```

### 3.6 create_session

语义：

- 创建会话初始元数据。
- 不写任何业务事件（由编排层决定何时写 SystemEvent）。

SQL 伪代码：

```sql
INSERT INTO sessions (
    id,
    tenant_id,
    user_id,
    agent_id,
    title,
    status,
    version,
    event_count,
    last_sequence,
    created_at,
    updated_at,
    archived
)
VALUES (
    COALESCE($id, gen_random_uuid()),
    $tenant_id,
    $user_id,
    $agent_id,
    COALESCE($title, ''),
    'active',
    0,
    0,
    0,
    NOW(),
    NOW(),
    FALSE
)
RETURNING *;
```

### 3.7 list_sessions

语义：

- 按用户列出会话。
- 支持是否包含 archived。
- 按更新时间倒序分页。

SQL 伪代码：

```sql
SELECT id,
       tenant_id,
       user_id,
       agent_id,
       title,
       status,
       version,
       event_count,
       last_sequence,
       created_at,
       updated_at,
       last_message_at,
       archived
FROM sessions
WHERE user_id = $1
  AND tenant_id = $2
  AND ($3::boolean = TRUE OR archived = FALSE)
ORDER BY updated_at DESC
LIMIT $4 OFFSET $5;
```

### 3.8 方法与 SQL 映射总表

- `append_events` -> `SELECT ... FOR UPDATE` + `INSERT batch` + `UPDATE sessions`
- `read_events` -> `SELECT ... BETWEEN ... ORDER BY ASC`
- `read_recent_events` -> `SELECT ... ORDER BY DESC LIMIT`
- `get_session` -> `SELECT FROM sessions WHERE id`
- `create_session` -> `INSERT INTO sessions RETURNING`
- `list_sessions` -> `SELECT FROM sessions WHERE user_id`

---

## 4. JSONB Payload 策略

### 4.1 核心策略

采用“类型路由 + JSONB 内容”模式：

- `event_type` 列存事件类型标识。
- `payload` 列存该类型的具体字段。

示意：

- `event_type = 'ToolCallRequest'`
- `payload = {"request_id":"...","tool_name":"web_search",...}`

### 4.2 为什么选 JSONB（MVP 取舍）

原因 1：事件类型会增长。

- 若每类事件建一张子表，迁移频繁。
- MVP 阶段迭代速度更重要。

原因 2：事件结构天然异构。

- `UserMessage` 与 `CompactionMarker` 字段几乎无交集。
- 强行关系建模会产生大量空列/关联复杂度。

原因 3：PostgreSQL 原生 JSONB 成熟。

- 支持字段查询、路径查询。
- 需要时可加 GIN 索引。

### 4.3 为什么不做多表拆分

多表拆分优点：

- 强约束更强。
- 某些统计查询更易优化。

但 MVP 缺点更大：

- schema 演进成本高。
- 插入流程需要按类型分流到不同表。
- 读取重建事件流需要 UNION。

结论：

- MVP 选 JSONB。
- 后续按热点字段“上提”为顶层列。

### 4.4 JSONB 查询场景示例

场景：按工具名检索工具调用请求。

SQL 伪代码：

```sql
SELECT event_id, session_id, sequence_number, payload
FROM events
WHERE event_type = 'ToolCallRequest'
  AND payload ->> 'tool_name' = 'web_search'
ORDER BY created_at DESC
LIMIT 100;
```

场景：按 request_id 关联 request/result。

```sql
SELECT *
FROM events
WHERE session_id = $1
  AND (
      (event_type = 'ToolCallRequest' AND payload ->> 'request_id' = $2)
   OR (event_type = 'ToolCallResult'  AND payload ->> 'request_id' = $2)
  )
ORDER BY sequence_number ASC;
```

### 4.5 JSONB 索引策略

默认不立即建 GIN。

理由：

- GIN 写入开销较高。
- MVP 先观察查询画像。

当出现稳定 JSONB 过滤热点时再加：

- 全 payload GIN：`USING GIN(payload jsonb_path_ops)`
- 或表达式索引：`(payload->>'tool_name')`

### 4.6 未来优化方向

若某字段成为高频过滤条件，可上提：

- `tool_name`（ToolCallRequest）
- `request_id`（ToolCallRequest/Result）
- `finish_reason`（AssistantMessage）

策略：

- 先保持 JSONB 真相。
- 新增冗余列做读优化。
- 由写入路径双写该列。

---

## 5. 与现有 Schema 的差异和迁移

### 5.1 当前 schema 现状

来自现有 migration：

`sessions`：

- `id`
- `user_id`
- `agent_id`
- `title`
- `summary`
- `created_at`
- `updated_at`
- `last_message_at`
- `archived`

`messages`：

- `id`
- `session_id`
- `role`
- `content`
- `created_at`
- `sequence_num BIGSERIAL`（全局序列）

### 5.2 与新 events 模型差异

差异 1：粒度。

- 旧：仅消息。
- 新：所有事实事件。

差异 2：顺序语义。

- 旧：`sequence_num` 全局。
- 新：`session_id + sequence_number` 局部单调。

差异 3：类型表达。

- 旧：`role`。
- 新：`event_type + payload`。

差异 4：可审计性。

- 旧：缺少 causation/correlation 语义承载。
- 新：可通过 payload/meta 表达完整链路。

差异 5：压缩机制。

- 旧：依赖 `sessions.summary`。
- 新：`Summary + CompactionMarker` 事件化。

### 5.3 迁移方案 A：并行运行

定义：

- 新老表同时写入一段时间。
- 读路径逐步从 messages 切到 events。

步骤建议：

1. 上线 `events` + 新索引。
2. 写路径双写（messages + events）。
3. 增加灰度开关：新读路径按租户/比例启用。
4. 验证一致性与性能。
5. 完全切换后逐步下线 messages 读依赖。

优点：

- 风险可控。
- 可回滚。
- 适合在线业务连续性。

缺点：

- 一段时间有双写复杂度。
- 运维与监控成本更高。

### 5.4 迁移方案 B：一次性迁移

定义：

- 停机窗口内把历史 messages 转换导入 events。
- 迁移后只保留新路径。

优点：

- 逻辑简单。
- 没有双写期复杂度。

缺点：

- 有停机时间。
- 回滚成本高。
- 对迁移脚本质量依赖高。

### 5.5 MVP 建议方案

**建议选方案 A（并行运行）。**

理由：

1. agent-demo 正在快速迭代，停机窗口不稳定。
2. 新事件模型尚需真实流量验证。
3. 并行可提供分批验证与快速回退能力。
4. 与“单体 MVP + 可演进”路线一致。

### 5.6 迁移边界说明

本设计只描述策略。

不包含：

- 具体迁移 SQL。
- 历史回填脚本实现。
- 线上切流脚本实现。

---

## 6. 性能考虑

### 6.1 写入性能

核心策略：

- 批量 INSERT。
- 单事务提交一批事件。
- 利用 `FOR UPDATE` 锁住会话元数据短临界区。

影响因素：

- 每个 turn 事件数（通常 2~10）。
- 索引数量（越多写放大越高）。
- JSONB 大小（payload 体积）。

MVP 建议：

- 控制 payload 尺寸。
- 避免过早增加 JSONB GIN 索引。

### 6.2 读取性能

关键查询：

- `read_events(range)`
- `read_recent_events(limit)`

核心索引：

- `(session_id, sequence_number)`
- `(session_id, sequence_number DESC)`

预期行为：

- 范围读取走有序索引扫描。
- 最近读取走 desc 索引 + limit。

### 6.3 JSONB 查询性能

策略：

- 默认只依赖 `event_type` 过滤 + 顺序扫描局部范围。
- 当出现高频 JSONB 条件时再补索引。

优先级：

1. 表达式索引（更轻）
2. 局部索引（按 event_type）
3. 全表 GIN（最后手段）

### 6.4 连接池管理（sqlx pool）

建议参数（MVP 起步）：

- `max_connections`: 20
- `min_connections`: 5
- `acquire_timeout`: 3s
- `idle_timeout`: 10m
- `max_lifetime`: 30m

实践建议：

- 把 pool 作为进程级共享单例注入。
- 对热点 SQL 使用 prepared statement。
- 监控 pool wait time 与超时率。

### 6.5 事务边界控制

原则：

- 事务内只做数据库操作。
- 禁止事务内调用外部网络（LLM/工具）。
- 事务尽可能短。

### 6.6 1000 事件单会话查询估算

假设：

- 单 session 1000 条事件。
- 平均 payload 1~2 KB。
- 索引命中正常。

粗略预估（同机房 PG）：

- 读取最近 50 条：通常 < 10 ms。
- 读取区间 1..1000：通常 10~40 ms。
- 插入单 turn 5 条事件：通常 < 10 ms（不含应用处理）。

说明：

- 这是工程估算，不是压测承诺。
- 真实值取决于硬件、并发、索引、payload 大小。

### 6.7 可观测性建议

建议记录指标：

- `event_store_append_latency_ms`
- `event_store_read_latency_ms`
- `event_store_conflict_count`
- `event_store_pool_acquire_timeout_count`
- `event_store_rows_per_append`

---

## 7. 测试策略

### 7.1 测试目标

验证 `PostgresEventStore` 满足 trait 契约：

- 写入正确。
- 顺序正确。
- 并发冲突可控。
- 空数据返回语义正确。

### 7.2 测试类型

MVP 主体采用集成测试：

- `sqlx::test` 驱动真实 PostgreSQL。
- 每个测试隔离数据库状态。

### 7.3 fixture 设计

基础 fixture：

1. 预置一个 user。
2. 预置一个 session。
3. 可选预置多条 events。

辅助 fixture：

- `make_event(event_type, payload)`
- `append_n_events(session, n)`

### 7.4 关键场景 1：append + read round-trip

用例步骤：

1. 创建 session。
2. `append_events` 写入一批事件。
3. `read_events` 读取完整区间。
4. 校验：
   - 事件数一致。
   - `sequence_number` 连续。
   - `event_type/payload` 可反序列化。

### 7.5 关键场景 2：并发 append

用例步骤：

1. 同一 session 并发发起多个 append 任务。
2. 全部完成后读取全量事件。
3. 校验：
   - 无重复 sequence。
   - 无缺洞（1..N 连续）。
   - 冲突按预期重试/报错映射。

### 7.6 关键场景 3：空 session

用例步骤：

1. 新建 session，不写事件。
2. 调用 `read_events` / `read_recent_events`。
3. 校验返回空数组，不报错。

### 7.7 关键场景 4：list_sessions 过滤

用例步骤：

1. 为同一 user 建多个 session。
2. 部分标记 archived。
3. 分别测试 `include_archived=false/true`。
4. 校验数量与排序正确。

### 7.8 关键场景 5：create/get 一致性

用例步骤：

1. `create_session`。
2. `get_session` 回读。
3. 校验元数据默认值：
   - `status=active`
   - `version=0`
   - `event_count=0`
   - `last_sequence=0`

### 7.9 关键场景 6：JSONB 可查询性（可选）

用例步骤：

1. 写入 `ToolCallRequest` 事件。
2. 用 JSONB 条件查询 tool_name。
3. 校验命中记录正确。

### 7.10 测试实施建议

- 优先保证 trait 合约测试通过。
- 再补性能基准测试。
- CI 至少跑核心 3 场景：
  - round-trip
  - 并发 append
  - 空 session

---

## 附录 A：方法级 SQL 清单（速查）

- `append_events`
  - `SELECT ... FOR UPDATE sessions`
  - `INSERT INTO events ... VALUES (...batch...)`
  - `UPDATE sessions SET last_sequence/event_count/version`

- `read_events`
  - `SELECT ... FROM events WHERE session_id AND seq BETWEEN ... ORDER BY ASC`

- `read_recent_events`
  - `SELECT ... FROM events WHERE session_id ORDER BY seq DESC LIMIT ...`

- `get_session`
  - `SELECT ... FROM sessions WHERE id = ?`

- `create_session`
  - `INSERT INTO sessions ... RETURNING *`

- `list_sessions`
  - `SELECT ... FROM sessions WHERE user_id AND tenant_id ... ORDER BY updated_at DESC`

---

## 附录 B：风险与缓解

风险 1：并发写入导致 sequence 冲突。

- 缓解：`FOR UPDATE` + 唯一约束 + 重试。

风险 2：JSONB 查询变慢。

- 缓解：先观测，再按热点加表达式索引或 GIN。

风险 3：双写迁移期一致性偏差。

- 缓解：增加对账任务与灰度切流开关。

风险 4：sessions 冗余字段与事件真相不一致。

- 缓解：明确 events 为 source of truth，session 字段仅缓存/索引。

---

## 附录 C：结论

本设计给出了 PostgreSQL 适配器在 Layer 4 的可执行方案。

满足目标：

- 实现 `EventStore` trait。
- 充分使用 PostgreSQL（JSONB / 索引 / 事务）。
- 支持从现有 `messages` 平滑迁移。

MVP 推荐路线：

- 先落 `events` 表与 `PostgresEventStore`。
- 采用并行迁移方案 A。
- 用 `sqlx::test` 建立契约级集成测试基线。
