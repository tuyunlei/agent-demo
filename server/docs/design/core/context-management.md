# 上下文编排策略设计（Context Management）

## 1. 目标与边界

本文定义 Agent Runtime 在调用 LLM 前的**上下文编排策略**，目标是：

- 在不同模型上下文窗口（4k~200k）下稳定工作
- 显式控制 token 成本与回复质量
- 保持六边形架构下的关注点分离（组装逻辑不绑定存储实现）
- 为 MVP 提供可落地方案，并保留演进扩展点

**不包含**：
- Memory 的生成/写入策略
- Persona 的创建与演化
- LLM 网关调用实现

---

## 2. 上下文结构总览

### 2.1 注入顺序（固定）

遵循 ADR-005 约定，输入上下文按以下顺序拼装：

1. `System Prompt`（平台级，不可改）
2. `Persona`（persona_text + persona_data）
3. `Tool Declarations`（可用工具 schema）
4. `Summary`（会话压缩摘要）
5. `Memory`（长期记忆片段）
6. `Recent Messages`（最近 N 条历史）
7. `Current User Message`（当前用户输入）
8. `Reply Reserve`（仅预算概念，不注入文本）

> 说明：第 8 项是预算保留，不是 prompt 文本块。

### 2.2 分段边界（推荐统一模板）

建议每段使用明确边界标签，降低提示词歧义：

- `<<SYSTEM>> ... <<END_SYSTEM>>`
- `<<PERSONA>> ... <<END_PERSONA>>`
- `<<TOOLS>> ... <<END_TOOLS>>`
- `<<SUMMARY>> ... <<END_SUMMARY>>`
- `<<MEMORY>> ... <<END_MEMORY>>`
- `<<HISTORY>> ... <<END_HISTORY>>`
- `<<USER>> ... <<END_USER>>`

### 2.3 固定部分 vs 动态部分

**固定高优先级（尽量不裁剪）**
- System Prompt
- Current User Message
- 最小 Persona（可降级到精简版）

**动态可裁剪**
- Tool Declarations（可按需子集注入）
- Summary（可重摘要）
- Memory（Top-K 选择）
- Recent Messages（窗口裁剪）

---

## 3. Token 预算管理

### 3.1 基本公式

- `total_budget = context_window - reply_reserve`
- `input_budget = total_budget - protocol_overhead`

其中：
- `context_window`：模型声明窗口（provider/model 配置）
- `reply_reserve`：给模型输出预留 token
- `protocol_overhead`：角色标签、结构边界、工具调用协议额外 token

### 3.2 预算分层（推荐）

先划分为两层：

1. **Hard Reserved（硬保留）**
   - System Prompt
   - Current User Message
   - Safety/Policy 相关必需段

2. **Elastic Budget（弹性池）**
   - Persona / Tools / Summary / Memory / History 竞争共享

### 3.3 默认预算建议（MVP）

按 `input_budget` 比例分配（可配置）：

- Persona：10%（上限）
- Tools：20%（上限）
- Summary：15%（目标）
- Memory：15%（上限）
- History：40%（优先消费剩余）

> 若某段实际不足上限，剩余回流给 History。

### 3.4 预算不足时的裁剪顺序（显式）

推荐从“对当前轮影响较小”到“影响较大”裁剪：

1. Tools（先删不相关工具，再压缩 schema）
2. Memory（降低 Top-K）
3. Summary（重摘要为更短版本）
4. History（减少 N）
5. Persona（降级为精简 persona）

**不裁剪**：System Prompt、Current User Message、最低输出预留。

### 3.5 Reply Reserve 策略

按任务类型设置预留比例（可配置）：

- 简短问答：10%~15%
- 常规助手：20%~30%
- 长文本生成：35%~45%
- 工具编排/结构化输出：25%~35%

---

## 4. 历史消息窗口策略

### 4.1 Fixed-N（MVP 默认）

MVP 使用固定条数窗口：
- 默认 `N = 12`（可配）
- 包含 user/assistant/tool 相关消息
- 始终保证当前用户消息单独注入

优点：简单、稳定、易解释。

### 4.2 Dynamic-Token Window（演进）

按 token 动态回溯：
- 从最新消息向前累加
- 直到达到 `history_budget`
- 停止并保留完整消息边界

适用于：消息长度波动大、模型窗口差异大。

### 4.3 裁剪粒度

建议优先级：

1. **整条消息裁剪**（默认）
2. **超长单条截断**（仅对异常长消息启用）
   - 保留开头 + 结尾
   - 中间替换为 `[...truncated...]`

不建议常态化句级截断，易破坏语义与工具调用上下文。

### 4.4 Tool 消息处理

对 tool call/result：
- 近期工具结果优先保留
- 历史工具结果可只留摘要（工具名+关键输出）

---

## 5. Compaction（压缩/摘要）机制

### 5.1 触发条件

满足任一条件触发：

1. 组装后 `history` 超预算阈值（如连续 2 轮超阈）
2. 会话累计消息数超过阈值（如 80 条）
3. 会话累计 token 估算超过阈值

### 5.2 输入与输出

**输入**：
- 旧 `session.summary`（若存在）
- 被压缩区间的历史消息（通常是“较早段”）

**输出**：
- 新的 `session.summary`（结构化摘要文本）

建议摘要结构：
- 用户长期偏好
- 已完成事项
- 未完成事项 / 待跟进
- 关键事实与约束
- 最近阶段结论

### 5.3 存储位置

- 持久化到 `sessions.summary`（TEXT）
- 可选记录版本号/更新时间（由 session 元数据承载）

### 5.4 与在线对话关系

**MVP 推荐：异步 compaction**
- 当前请求优先返回（低延迟）
- 后台任务更新 summary，下轮生效

**兜底：同步轻量 compaction**
- 当本轮无法组装到最小可用上下文时，执行一次同步紧急摘要

### 5.5 多轮 compaction（防 summary 膨胀）

采用“摘要再摘要（summary-of-summary）”策略：

- 为 summary 设置硬上限（如 600~1200 tokens）
- 超限时触发重写，不是简单 append
- 保留“稳定事实 + 当前阶段目标”，删除细枝末节

建议使用双层摘要模型：
- `Core Summary`：长期稳定信息（低频变化）
- `Rolling Summary`：近期阶段信息（高频更新）

最终注入时合并并受统一预算约束。

---

## 6. Memory 注入策略

### 6.1 注入原则

只注入“对当前轮有帮助”的 memory，而非全量注入。

### 6.2 选择与排序（Top-K）

对候选 memory 打分（可配置权重）：

- Relevance（与当前用户消息语义相关）
- Recency（近期性）
- Importance（业务重要级）
- Reliability（置信度/已确认程度）

按综合分排序，取 `Top-K` 直至 `memory_budget`。

### 6.3 注入格式（推荐）

每条 memory 使用轻量结构：

- `- [type] fact | confidence | timestamp`

示例：
- `[preference] 用户偏好简洁回答 | high | 2026-02-10`

### 6.4 过多时的处理

- 先降 K
- 再仅保留高 importance/high confidence
- 最后可将低优先 memory 合并为一条“聚合记忆”

---

## 7. 工具声明注入策略

### 7.1 控制 token 占用

工具声明是高成本段，采用分级注入：

1. **MVP**：注入“当前启用工具全集”的精简 schema
2. **演进**：按意图路由，仅注入候选工具子集

### 7.2 Schema 压缩建议

- 保留：工具名、用途、必填参数、关键约束
- 删除：冗长描述、重复示例、非关键元数据
- 统一字段命名与描述模板，减少冗余 token

### 7.3 工具过多时策略

- 先按域分组（calendar/email/file/...）
- 通过轻量意图分类选 Top-M 工具
- 保留一个“fallback 通用工具”或“工具发现说明”

裁剪优先顺序：
- 去掉低频工具 → 去掉长描述 → 限制参数说明深度

---

## 8. ContextAssemblerPort 设计

> 与 agent-runtime.md 对齐：Runtime 调用 ContextAssemblerPort，返回 `LlmInputContext` 给 LlmGatewayPort。

### 8.1 接口职责（自然语言）

`ContextAssemblerPort` 接收一次推理请求所需的原始上下文数据与组装配置，执行 token 预算计算、裁剪与排序，输出可直接发送给 LLM 网关的标准化上下文对象。

### 8.2 输入（原始数据）

- `model_profile`：provider/model/context_window/tokenizer 信息
- `system_prompt`
- `persona_text + persona_data`
- `tool_definitions`（来自 ToolRuntimePort）
- `session_summary`
- `memory_candidates`
- `message_history`（按时间有序）
- `current_user_message`
- `assembly_policy`（预算、裁剪、触发阈值）

### 8.3 输出：`LlmInputContext`（结构描述）

建议包含：

- `segments[]`：按顺序的上下文段（type + content + token_estimate）
- `final_prompt_payload`：供网关直接使用的消息结构
- `token_accounting`：
  - context_window
  - reply_reserve
  - input_used
  - per_segment_usage
  - truncation_events
- `diagnostics`：
  - compaction_hint（是否建议触发压缩）
  - dropped_items（被裁剪的工具/memory/消息）

### 8.4 可配置项（显式）

- reply_reserve 策略
- 各 segment 预算上限/下限
- 历史窗口模式（fixed-N / dynamic-token）
- 默认 N、最小 N
- memory Top-K 与评分权重
- tool 注入模式（全量精简 / 意图子集）
- compaction 触发阈值与摘要上限
- 裁剪优先级序列

---

## 9. 多 Provider 适配

### 9.1 Context Window 差异适配

按模型 profile 驱动策略：

- 小窗口模型（4k~16k）：
  - 更激进裁剪 tools/memory/history
  - 更依赖 summary
- 中窗口模型（32k~64k）：
  - 平衡 history 与 memory
- 大窗口模型（128k~200k）：
  - 可放宽历史窗口，降低 compaction 频率

### 9.2 Token 计算差异适配

不同 provider/tokenizer 计数规则不同，采用分层计数：

1. **Provider 精确计数器**（首选）
2. **本地 tokenizer 估算器**（次选）
3. **字符数近似 + 安全系数**（兜底）

引入 `safety_margin`（如 5%~12%）防止超窗。

### 9.3 统一抽象

在 ContextAssembler 内只依赖 `TokenEstimatorPort` 抽象，不绑定具体 provider SDK。

---

## 10. MVP 与演进路线

### 10.1 MVP（先上线）

- 固定顺序注入
- Fixed-N 历史窗口（默认 N=12）
- 显式预算与裁剪顺序
- Top-K memory 注入（简单打分）
- 工具 schema 精简版全量注入
- 异步 compaction + `sessions.summary`

### 10.2 演进方向

- Dynamic-token 历史窗口
- 意图感知工具子集注入
- 分层摘要（Core/Rolling）
- 自适应预算（按任务类型/用户行为）
- 更精准 token 计数与实时观测

---

## 11. 可观测性与运营建议

建议记录以下指标（用于成本与质量优化）：

- 每轮输入/输出 token
- 各 segment token 占比
- 裁剪发生率与裁剪类型
- compaction 触发次数与摘要长度变化
- 工具注入数量与工具调用成功率

这些指标可支持后续 A/B 测试：
- 不同 N 的质量/成本对比
- memory Top-K 配置对回复命中率影响
- 工具子集策略对 token 与成功率影响

---

## 12. 验收对照

- [x] prompt 结构完整，顺序与边界清晰
- [x] token 预算分配明确，裁剪优先级有序
- [x] 历史窗口覆盖 fixed 与 dynamic 两种策略
- [x] compaction 覆盖触发、输入输出、存储、异步关系、多轮策略
- [x] memory 注入包含选择与排序机制
- [x] ContextAssemblerPort 与 Runtime→Assembler→Gateway 链路一致
- [x] 考虑不同 context window 与 token 计数差异
- [x] 给出 MVP 与未来演进方向

---

## 13. 一句话总结

ContextAssembler 的核心是：在**固定注入顺序**下，用**显式预算 + 可解释裁剪 + 可持续 compaction**，把多源上下文稳定压缩到模型可承载范围内，并在成本、质量、可扩展性之间取得可运营的平衡。