# LLM Gateway 设计（overview）

## 1. 目标与边界

LLM Gateway 是平台与外部大模型 API 的**统一抽象层**，负责：

- 屏蔽 OpenAI / Anthropic / 国产模型的协议差异
- 提供统一的流式推理接口（文本 delta + tool_calls）
- 处理路由、重试、fallback、降级
- 做 token 计量、成本估算、配额与预算控制

不在本设计范围内：

- Agent Runtime 的详细执行流程（已完成）
- ContextAssembler 的上下文编排策略（已完成）
- 具体 Rust 代码实现

---

## 2. 六边形架构定位

### 2.1 分层位置

在六边形架构中，Gateway 相关职责拆分如下：

- **Domain（核心）**
  - `LlmGatewayPort`：Runtime 调用入口（已存在）
  - `LlmProviderPort`：单个 provider 的统一能力抽象
  - `ModelRoutingPolicyPort`：模型与 provider 选择策略
  - `TokenMeteringPort`：token/成本计量接口
  - `QuotaPolicyPort`：配额/限流/预算判断接口

- **Application（用例编排）**
  - `LlmGatewayService`：编排路由 → 调用 → 重试/fallback → 计量 → 事件输出
  - 负责跨 Port 协作，不承载 provider 细节

- **Adapter（基础设施实现）**
  - OpenAI / Anthropic / 国产 provider adapter
  - API key 仓储实现（DB/KMS/Secrets）
  - 计费配置加载器（模型价格表）
  - 观测性实现（日志、metrics、trace）

### 2.2 与 Runtime、ContextAssembler 的关系

- **Runtime → LlmGatewayPort**：提交已组装上下文与工具声明，消费流式结果。
- **ContextAssembler → Gateway**：
  - 上游需要模型 `context_window`，由 Gateway 暴露可查询的模型元数据（或 Registry 提供）
  - token 估算通过 `TokenEstimatorPort` 完成，Gateway 负责在调用前后接入估算与实测 usage
- **边界原则**：
  - Runtime 不感知 provider 差异
  - ContextAssembler 不关心调用细节，仅依赖模型能力与 token 约束信息

---

## 3. `LlmProvider` 统一接口设计（自然语言）

> 该接口属于 Domain Port，具体 provider 在 Adapter 层实现。

### 3.1 输入（标准化请求）

统一请求对象建议包含：

1. **消息列表（messages）**
   - 角色：system / user / assistant / tool
   - 内容块：text（MVP），预留 image/audio/document（多模态）
2. **工具声明（tools）**
   - 工具名、描述、JSON Schema 参数
   - 工具调用策略（auto / forced / none）
3. **模型配置（model config）**
   - provider、model_id、temperature、top_p、max_output_tokens、stop
   - 流式开关（默认 stream=true）
   - 可选 provider 扩展字段（通过显式 `provider_options` 承载）
4. **调用上下文（invoke context）**
   - tenant_id、user_id、agent_id、request_id
   - 超时、重试预算、幂等键（可选）

### 3.2 输出（标准化流）

统一输出为事件流（内部协议），核心事件：

- `TextDelta`：增量文本
- `ToolCallDelta`：工具调用增量（name/arguments 分片）
- `ToolCallCompleted`：某个工具调用参数闭合
- `MessageCompleted`：本轮模型消息结束
- `Usage`：prompt_tokens / completion_tokens / total_tokens（若 provider 支持流中 usage，可多次更新，最终以结束值为准）
- `ProviderMeta`：finish_reason、model_version、provider_request_id

### 3.3 Provider 差异在 Adapter 层吸收

Adapter 负责把外部差异映射到统一协议，包括：

- 请求结构差异（Chat Completions / Responses / Messages API）
- 流式协议差异（SSE event 名称、chunk 结构、JSON 路径）
- tool calling 差异（一次性返回 vs 增量拼接）
- system prompt 传递方式差异（独立字段 vs 首条 system message）
- usage 返回时机差异（结束后返回 vs 流中返回）

Domain/Application 仅处理统一事件，不接触 provider 原始字段。

---

## 4. 多 Provider 管理

### 4.1 Provider 注册与配置

设计 `ProviderRegistry`（可热更新，先支持静态配置 + 重载）：

- provider 基础信息：名称、状态（enabled/disabled）、超时、默认重试策略
- model catalog：
  - model_id、context_window、max_output
  - 能力标签（tool_calling、vision、json_mode、reasoning）
  - 价格（input/output token 单价，按币种）
- 健康状态：最近成功率、延迟、限流状态

### 4.2 模型路由策略

路由决策输入：tenant/user/agent/scenario（聊天、总结、工具密集、低成本模式）

建议分层路由：

1. **策略层**（业务）：根据场景选“目标能力档”
2. **候选层**（技术）：从 registry 过滤满足能力与上下文窗口的模型
3. **排序层**（运行态）：按成本、延迟、健康度、用户偏好排序
4. **选择层**：给出主选 + 备用链路（fallback chain）

可支持：

- 按租户白名单/黑名单限制 provider
- 按用户套餐限制模型档位
- 按 agent 配置固定主模型 + 自动备用

### 4.3 API Key 管理（多租户）

支持两类 key：

1. **平台级 Key Pool（shared）**
   - 平台维护，按策略轮换
   - 用于默认托管模式
2. **用户自带 Key（BYOK）**
   - 租户/用户绑定
   - 优先使用用户 key，失败后可按策略回退平台 key（需租户授权）

关键机制：

- 密钥加密存储（KMS/Secrets Manager），最小权限解密
- key 健康度与配额状态跟踪（余额不足、429、封禁）
- 调用时选择可用 key（避免热点，支持平滑轮转）
- 审计：记录 key_id（脱敏）与计费归属

---

## 5. 重试、Fallback、降级

### 5.1 单 provider 内重试

按错误类型分类：

- **可重试**：超时、网络抖动、429、5xx
- **不可重试**：鉴权失败、参数非法、上下文超限（未启用自动裁剪时）

策略建议：

- 指数退避 + 抖动（如 200ms, 800ms）
- 小次数（MVP：1~2 次）
- 受“请求总超时预算”约束，避免无限拖延

### 5.2 Fallback 触发条件

触发 fallback 到备用 provider/model：

- 主 provider 达到重试上限仍失败
- 命中 provider 级熔断（短时高失败率）
- key pool 不可用（余额/权限问题）

### 5.3 降级策略

按优先级逐级降级：

1. 同 provider 更低成本/更快模型
2. 切换备用 provider 同能力模型
3. 缩减上下文（由上游重新组装或裁剪后重试）
4. 降低输出上限（max_output_tokens）

降级必须显式记录 `degrade_reason`，便于观测与用户解释。

### 5.4 全部失败处理

返回统一领域错误 `LlmGatewayError`，包含：

- user-safe 错误码（如 `MODEL_UNAVAILABLE`, `QUOTA_EXCEEDED`）
- 最后失败原因摘要（脱敏）
- 已尝试 provider/model 列表
- 是否建议上层重试

并输出失败遥测事件，便于告警与后续容量治理。

---

## 6. Token 计量与成本控制

### 6.1 请求级计量

每次请求记录：

- estimated_prompt_tokens（调用前估算）
- actual_prompt/completion/total_tokens（调用后实测）
- provider/model/key_id/request_id
- cost_estimate（按价格表折算）

若 provider 未返回 usage：

- 使用 estimator 回填并标记 `usage_source=estimated`

### 6.2 用户级计量与账本

建立租户/用户维度聚合：

- 日/周/月 token 使用量
- 按模型与 provider 的成本拆分
- BYOK 与平台代付分账

账本建议“事件追加写入 + 异步聚合”，避免在线路径强耦合。

### 6.3 配额与限流

控制维度：

- 每用户每分钟请求数（RPM）
- 每用户每分钟 token（TPM）
- 每日/月 token 或金额配额

执行时机：

- **前置检查**：拒绝明显超限请求
- **后置结算**：按实际 usage 扣减
- 可配置“软限额”（告警）与“硬限额”（拒绝）

### 6.4 预算告警

预算策略：

- 阈值告警：达到 50% / 80% / 100%
- 异常告警：单位时间成本突增、某模型异常放量
- 渠道：内部告警系统 + 管理后台可见

---

## 7. 流式输出统一协议

### 7.1 内部流事件规范

统一事件建议：

1. `StreamStarted`
2. 多个 `TextDelta` / `ToolCallDelta`
3. `UsageUpdated`（可选多次）
4. `StreamCompleted` 或 `StreamFailed`

要求：

- 顺序单调（带 sequence）
- 可追踪（request_id, provider_request_id）
- 事件粒度稳定，便于前端与 Runtime 消费

### 7.2 流中断处理

中断场景：网络断流、provider 主动断开、超时取消。

处理原则：

- 若可判定“语义完成”则发 `StreamCompleted`
- 否则发 `StreamFailed`，附带已生成内容与错误原因
- 对上游暴露“是否可恢复”标记（通常不可恢复，仅可整体重试）

### 7.3 与 ChatEvent 对接

映射关系（Runtime 已定义）：

- `TextDelta` → `ChatEvent::TextDelta`
- `ToolCallCompleted` → `ChatEvent::ToolCall`
- `StreamCompleted` → `ChatEvent::Done`
- `StreamFailed` → `ChatEvent::Error`

Gateway 负责保证事件语义一致，Runtime 无需理解 provider 原始流协议。

---

## 8. Provider 特定能力适配

### 8.1 Tool calling 差异

适配点：

- 工具参数格式（JSON Schema 支持程度不同）
- 工具调用返回方式（增量参数流 vs 完整对象）
- 多工具并行调用支持差异

统一策略：

- 内部采用标准工具声明与工具事件
- adapter 做能力探测：不支持并行时自动串行约束
- 不支持 tool calling 的模型在路由阶段即过滤

### 8.2 System prompt 差异

- 某些 provider 有独立 system 字段
- 某些需要把 system 作为首条 message

统一做法：

- Domain 请求固定保留 `system` 语义
- adapter 负责编码方式转换
- 避免在 Runtime 层写 provider 分支逻辑

### 8.3 Vision/多模态（预留）

MVP 文本优先，同时预留：

- 统一内容块结构（text/image_url/image_bytes/...）
- 模型能力标签（vision=true/false）
- 路由阶段基于能力筛选
- usage 计量可扩展到图像 token/帧成本

---

## 9. 关注点分离（核心原则落地）

为避免“网关巨石化”，明确拆分模块：

1. **ProviderAdapter**：只做协议转换与调用
2. **Router**：只做模型/provider/key 选择
3. **RetryOrchestrator**：只做重试/fallback/降级状态机
4. **MeteringService**：只做 usage 与成本记账
5. **QuotaGuard**：只做限流与配额判断

`LlmGatewayService` 仅负责编排，不内嵌复杂策略细节。

---

## 10. MVP 范围与演进

### 10.1 MVP（建议 1~2 个 provider）

- 支持 OpenAI + Anthropic（或 1 个国际 + 1 个国产）
- 支持文本流式 + tool calling
- 支持主路由 + 单级 fallback
- 支持基础 usage 记录与月度成本统计
- 支持平台 key pool（BYOK 可后置）

### 10.2 下一阶段

- 引入 BYOK 与分账
- 动态路由（健康度/成本实时优化）
- 熔断器与自愈恢复
- 多模态输入输出
- 更细粒度预算控制（按 agent/场景）

### 10.3 长期演进

- 策略引擎化（路由/降级可配置）
- 多区域/多云 provider 冗余
- 成本优化闭环（离线分析 → 在线策略调优）

---

## 11. 验收对照

- [x] Gateway 在六边形架构中的位置清晰
- [x] `LlmProvider` trait 接口完整（输入/输出/元数据）
- [x] 多 provider 管理覆盖注册、路由、key 管理
- [x] 重试/fallback/降级策略完整
- [x] Token 计量和成本控制有方案
- [x] 流式输出统一化方案明确
- [x] Provider 差异适配有说明
- [x] MVP 与演进方向明确
