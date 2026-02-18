# 可观测性设计（design/observability.md）

## 1. 目标与设计原则

面向**多租户托管 AI Agent SaaS**（Rust + Tokio + gRPC + PostgreSQL，六边形架构），可观测性目标是：

1. **快速定位故障**：5~15 分钟内定位到模块级根因（入口、运行时、LLM 网关、通道、数据库）。
2. **可量化 SLO**：以错误率、延迟、可用性、成本为核心服务指标。
3. **可审计可追责**：认证与权限变更全链路留痕，满足审计要求。
4. **成本可控**：token 与供应商成本可视化，支持按租户/模型/渠道归因。

设计原则：

- **横切关注点注入**：通过 gRPC middleware/interceptor、应用 service decorator、基础设施 adapter wrapper 注入，避免业务代码分散埋点。
- **先 MVP 后增强**：MVP 聚焦 Metrics + 结构化日志；Tracing 先做基础链路与采样，逐步细化。
- **统一语义约定**：指标命名、日志字段、trace attributes 统一标准，降低维护成本。
- **多租户安全优先**：所有观测数据遵循最小暴露原则，敏感字段默认脱敏。

---

## 2. 三支柱策略（Logging / Metrics / Tracing）

## 2.1 Logging（结构化日志）

### 2.1.1 目标

- 支撑**审计**、**故障排查**、**安全事件追踪**。
- 与 metrics/tracing 联动：日志必须带 `trace_id`、`span_id`（若存在）与 `request_id`。

### 2.1.2 策略

- 输出格式：**JSON Lines（每行一个 JSON）**。
- 日志分流：
  - `application.log`（业务日志）
  - `audit.log`（认证与权限变更审计）
  - `security.log`（风控/异常访问）
- 日志等级：`DEBUG` / `INFO` / `WARN` / `ERROR` / `FATAL`。
- 生产环境默认：`INFO` 及以上；按租户/模块支持动态提级（短时 DEBUG）。
- 日志采样：高频成功日志可按比例采样（如 10%），错误日志不采样。

### 2.1.3 保留策略

- 热存储（近 7~14 天）：用于实时检索。
- 温存储（30~90 天）：合规与复盘。
- 审计日志建议保留更久（按合规要求，如 180 天或 1 年）。

---

## 2.2 Metrics（指标）

### 2.2.1 目标

- 实时反映健康度、容量、性能、成本。
- 作为告警与 SLO 的核心数据源。

### 2.2.2 策略

- 采用 Prometheus 模型：
  - `Counter`：累计事件（请求数、错误数、token 数）
  - `Gauge`：瞬时状态（队列长度、连接池占用、内存水位）
  - `Histogram`：延迟分布（gRPC、LLM、工具执行、E2E）
- 标签（labels）控制：
  - 保留低基数字段：`service`, `module`, `method`, `provider`, `model`, `channel`, `tenant_tier`
  - 禁止高基数字段直接上指标：`user_id`, `session_id`, `trace_id`, `prompt_hash` 等
- 指标生命周期：版本演进时维护 deprecate 周期，避免仪表盘断裂。

---

## 2.3 Tracing（分布式追踪）

### 2.3.1 目标

- 建立从 gRPC 入口 → 应用编排 → LLM 调用 → DB/外部通道 的调用拓扑。
- 支持“慢请求”和“错误请求”快速钻取。

### 2.3.2 MVP 策略（简化）

- 默认低比例采样（如 1%）；
- **错误请求、超时请求、慢请求**强制采样（tail-based 可后续增强）；
- 关键 span：`grpc.server`, `agent.orchestrate`, `llm.call`, `tool.exec`, `db.query`, `channel.send`。

---

## 3. 分布式追踪传播机制（trace_id/span_id）

## 3.1 传播总体路径

`Client -> gRPC Ingress -> Auth Interceptor -> Agent Runtime -> LLM Gateway -> Provider API`

并行支路：
- `Agent Runtime -> Tool Adapter`
- `Agent Runtime -> Channel Adapter`
- `Agent Runtime -> PostgreSQL`

## 3.2 上下文注入与提取

1. **gRPC 入口（Server Interceptor）**
   - 从 metadata 提取 trace context（W3C Trace Context 或 OTEL 兼容格式）。
   - 若无上下文，生成新 trace_id。
   - 在 request context 中写入：`trace_id`, `span_id`, `request_id`, `tenant_id`（脱敏/内部标识）。

2. **应用层（UseCase / Service）**
   - 每个核心用例创建子 span（如 `agent.plan`, `agent.execute_step`）。
   - 埋点统一通过 observability facade，不在 domain 逻辑散落 SDK 调用。

3. **出站调用（Client Interceptor）**
   - LLM Gateway 出站 gRPC/HTTP 请求注入 trace context。
   - 工具调用、通道发送、数据库访问分别创建子 span 并继承父上下文。

4. **日志关联**
   - 日志中固定输出 `trace_id` + `span_id`，实现 logs <-> traces 双向跳转。

## 3.3 错误与重试语义

- 同一次业务请求内的重试应挂在同一 `trace_id` 下，不同尝试使用不同 `span_id`，并标注：
  - `retry.attempt`
  - `retry.max`
  - `retry.reason`（timeout/rate_limit/5xx）
- 超时与取消统一标注：`error.type=timeout|canceled`。

## 3.4 建议字段（trace attributes）

- 通用：`service.name`, `env`, `region`, `tenant.id_hash`, `request.id`
- gRPC：`rpc.system=grpc`, `rpc.service`, `rpc.method`, `rpc.grpc.status_code`
- LLM：`llm.provider`, `llm.model`, `llm.input_tokens`, `llm.output_tokens`, `llm.cost_usd`
- Tool：`tool.name`, `tool.success`, `tool.latency_ms`
- Channel：`channel.type`, `channel.direction`, `channel.degrade_mode`

---

## 4. 关键指标体系（按模块）

命名规范：

- 前缀：`agent_platform_`
- 单位后缀：`_seconds`, `_milliseconds`, `_bytes`, `_total`, `_ratio`
- 直方图统一命名：`*_duration_seconds`

## 4.1 Gateway / gRPC 入口

- `agent_platform_grpc_requests_total{service,method,code}` (Counter)
- `agent_platform_grpc_request_duration_seconds{service,method}` (Histogram)
- `agent_platform_grpc_inflight_requests{service,method}` (Gauge)
- `agent_platform_grpc_payload_bytes{service,method,direction}` (Histogram)

## 4.2 Agent Runtime（含既有 10 项能力）

- `agent_platform_agent_runs_total{tenant_tier,status}`
- `agent_platform_agent_run_duration_seconds{scenario}`
- `agent_platform_agent_step_duration_seconds{step_type}`
- `agent_platform_agent_tool_calls_total{tool_name,status}`
- `agent_platform_agent_tool_success_ratio{tool_name}`（可由 recording rule 计算）
- `agent_platform_agent_tool_duration_seconds{tool_name}`
- `agent_platform_agent_token_input_total{provider,model}`
- `agent_platform_agent_token_output_total{provider,model}`
- `agent_platform_agent_context_window_usage_ratio{model}`
- `agent_platform_agent_failures_total{error_type}`

> 说明：上述覆盖延迟、token 使用、工具成功率等核心项。

## 4.3 LLM Gateway

- `agent_platform_llm_requests_total{provider,model,status}`
- `agent_platform_llm_request_duration_seconds{provider,model}`
- `agent_platform_llm_time_to_first_token_seconds{provider,model}`
- `agent_platform_llm_tokens_input_total{provider,model}`
- `agent_platform_llm_tokens_output_total{provider,model}`
- `agent_platform_llm_cost_usd_total{provider,model,tenant_tier}`
- `agent_platform_llm_rate_limit_total{provider,model}`
- `agent_platform_llm_retries_total{provider,model,reason}`

## 4.4 并发与运行时资源（Tokio / 系统）

- `agent_platform_runtime_queue_length{queue_name}`
- `agent_platform_runtime_task_wait_duration_seconds{queue_name}`
- `agent_platform_runtime_worker_busy_ratio{pool}`
- `agent_platform_runtime_db_pool_inuse{pool}`
- `agent_platform_runtime_db_pool_wait_seconds{pool}`
- `agent_platform_runtime_memory_usage_bytes{type=rss|heap}`
- `agent_platform_runtime_cpu_usage_ratio{core_group}`

## 4.5 Channel（入站/出站）

- `agent_platform_channel_inbound_total{channel,source,status}`
- `agent_platform_channel_outbound_total{channel,target,status}`
- `agent_platform_channel_e2e_duration_seconds{channel}`
- `agent_platform_channel_degrade_total{channel,mode}`
- `agent_platform_channel_degrade_ratio{channel}`（recording rule）
- `agent_platform_channel_retry_total{channel,reason}`

## 4.6 Auth & Audit

- `agent_platform_auth_login_total{result,method}`
- `agent_platform_auth_register_total{result}`
- `agent_platform_auth_permission_change_total{action,role}`
- `agent_platform_auth_failures_total{reason}`
- `agent_platform_auth_mfa_challenge_total{result}`

## 4.7 PostgreSQL / 存储

- `agent_platform_db_queries_total{operation,table,status}`
- `agent_platform_db_query_duration_seconds{operation,table}`
- `agent_platform_db_connections{state=active|idle|waiting}`
- `agent_platform_db_tx_duration_seconds{status}`

---

## 5. 结构化日志设计

## 5.1 日志标准字段（JSON）

必填字段建议：

- `ts`：ISO8601 时间
- `level`：日志级别
- `service` / `module` / `component`
- `message`
- `trace_id` / `span_id` / `request_id`
- `tenant_id_hash`
- `user_id_hash`（如涉及用户行为）
- `event`（如 `auth.login.failed`, `llm.call.completed`）
- `status`（success/failure/timeout）
- `error_code` / `error_type`（失败时）
- `latency_ms`（可选）

## 5.2 级别策略

- `DEBUG`：仅开发/临时问题定位（高频细节，默认关闭）
- `INFO`：关键状态变化（请求完成、任务完成、重试）
- `WARN`：可恢复异常（短时超时、降级、重试触发）
- `ERROR`：请求失败或功能不可用
- `FATAL`：进程不可恢复错误（应触发立即告警）

## 5.3 敏感信息脱敏规范

默认禁止落盘：

- 明文 token/API Key/密码/验证码
- 用户原始 prompt 全量文本（可仅存摘要或 hash）
- 邮箱/手机号/身份证件等 PII 明文

推荐策略：

- `Authorization`、`Cookie`、`Set-Cookie` 全字段掩码：`***`
- 邮箱：`a***@domain.com`
- 手机：`138****0000`
- IP：按策略部分掩码或 hash
- Prompt/Response：
  - 默认记录 `length`、`sha256`、`classification`（是否含敏感）
  - 仅在受控调试窗口允许采样内容（需审计开关）

## 5.4 审计日志（认证域）

审计事件最小集合：

- `auth.login.succeeded/failed`
- `auth.register.succeeded/failed`
- `auth.password.changed`
- `auth.role.changed`
- `auth.permission.granted/revoked`
- `auth.mfa.enabled/disabled`

每条审计日志必须包含：操作者、目标对象、动作、结果、来源 IP（脱敏规则后）、时间、追踪 ID。

---

## 6. 告警策略（错误率 / 延迟 / 资源）

采用“多窗口 + 严重等级”策略（P1/P2/P3），并区分系统级与租户级。

## 6.1 错误率告警

1. **gRPC 总体错误率高**（P1）
   - 条件：`5xx_rate > 5%` 持续 5 分钟
2. **LLM Provider 错误率高**（P1/P2）
   - 条件：按 provider+model 维度 `error_rate > 10%` 持续 10 分钟
3. **工具调用失败率异常**（P2）
   - 条件：某工具 `failure_rate > 20%` 且调用量超过阈值

## 6.2 延迟告警

1. **gRPC P95 延迟超 SLO**（P1）
   - 条件：`p95 > 2s` 持续 10 分钟
2. **LLM 首 token 时间恶化**（P2）
   - 条件：`ttft p95` 超基线 2 倍
3. **Channel 端到端延迟异常**（P2）
   - 条件：`e2e p95 > 5s` 持续 10 分钟

## 6.3 资源与容量告警

1. **队列堆积**（P1）
   - 条件：队列长度持续增长且处理速率下降
2. **DB 连接池耗尽**（P1）
   - 条件：`inuse/size > 0.9` 持续 5 分钟
3. **内存水位过高**（P1/P2）
   - 条件：RSS 超阈值（如 85%）持续 10 分钟

## 6.4 成本告警

1. **单小时 token 成本突增**（P2）
   - 条件：当前小时成本 > 过去 7 天同小时均值的 2~3 倍
2. **单租户成本异常**（P2）
   - 条件：tenant 成本突增且请求量无同比增长

---

## 7. 技术选型建议（MVP -> 增强）

## 7.1 MVP（优先落地）

- **Instrumentation**：OpenTelemetry SDK（metrics + logs context 字段）
- **Metrics**：Prometheus（抓取）
- **Visualization/Alert**：Grafana + Alertmanager
- **Logging**：结构化 JSON 输出到 Loki（或 ELK/OpenSearch，视团队存量）

MVP 建议优先级：

1. 指标埋点 + SLO 仪表盘
2. 结构化日志 + 审计日志
3. 基础 trace（低采样 + 错误强制采样）

## 7.2 增强阶段

- **Tracing Backend**：Jaeger 或 Tempo（推荐 Tempo 与 Grafana 深度集成）
- **OTel Collector**：统一接收、处理、路由 telemetry（后续必选）
- **Tail Sampling**：错误/慢请求保留，降低成本
- **Service Map**：自动拓扑图 + 关键依赖健康度

---

## 8. 成本与 LLM 用量仪表盘设计

## 8.1 核心看板（Grafana）

1. **LLM 总览**
   - QPS、成功率、P95 延迟、TTFT
   - 输入/输出 token 趋势（1h/24h/7d）

2. **成本看板**
   - `cost_usd_total`（按 provider/model/tenant/channel 拆分）
   - 单请求平均成本（avg cost per request）
   - 成本 TopN（租户、模型、场景）

3. **效率看板**
   - token 转化率（output/input）
   - cache 命中率（若有提示缓存）
   - 重试导致的额外 token 与成本

4. **异常看板**
   - 成本突增检测
   - 高失败高成本模型识别

## 8.2 计费归因模型（建议）

统一成本公式：

`cost = input_tokens * input_unit_price + output_tokens * output_unit_price + 额外费用`

归因维度：

- `tenant_id`
- `workspace_id / project_id`
- `provider`
- `model`
- `channel`
- `feature`（如 summarize/chat/reasoning/tool_use）

## 8.3 数据一致性建议

- 在线实时指标用于监控；
- 离线日结（DB/数仓）用于对账；
- 对账字段：request_id、provider response id、token 明细、计费版本。

---

## 9. 六边形架构落地方式（不侵入领域）

- **Inbound adapter（gRPC）**：统一 interceptor 注入 trace/log context，记录入口指标。
- **Application service 层**：通过 observability facade 记录业务步骤指标与事件。
- **Outbound adapter（LLM/DB/Channel/Tool）**：统一 wrapper 记录出站指标、日志、trace span。
- **Domain 层**：不直接依赖观测 SDK，仅暴露业务事件/结果供应用层记录。

这保证了“可观测性是横切关注点”，且不破坏六边形边界。

---

## 10. 实施路线图（建议）

## Phase 1（1~2 周，MVP）

- 完成统一日志 schema 与脱敏中间件
- 落地核心 metrics（gRPC、Agent、LLM、Channel、Auth、Runtime）
- 建立 3 个基础看板（服务健康、LLM、成本）
- 配置 8~12 条核心告警

## Phase 2（2~4 周）

- 接入基础 tracing（入口到 LLM）
- 错误/慢请求强制采样
- Grafana 中打通 metrics + logs + traces 跳转

## Phase 3（持续优化）

- 引入 OTel Collector + tail sampling
- 优化高基数治理与采样策略
- 完善租户级 SLO 与成本预算控制

---

## 11. 验收映射（对应要求）

- [x] 三支柱策略完整：第 2 章
- [x] trace_id 传播机制明确：第 3 章
- [x] 关键指标按模块列出：第 4 章
- [x] 日志格式和脱敏规则明确：第 5 章
- [x] 告警规则有建议：第 6 章
- [x] 技术选型有推荐：第 7 章
- [x] 成本与 LLM 用量仪表盘：第 8 章

---

## 附录 A：推荐标签白名单（防高基数）

建议保留：
`env, service, module, method, provider, model, channel, status, error_type, tenant_tier`

建议禁用（不要直接作为指标标签）：
`tenant_id, user_id, session_id, trace_id, request_id, prompt_hash, tool_args`

如需按 tenant 精细分析，使用日志/离线数仓，不走 Prometheus 高基数标签。
