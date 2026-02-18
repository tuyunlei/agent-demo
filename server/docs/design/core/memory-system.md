# 记忆系统设计（Memory System Design）

## 1. 记忆系统总览

### 1.1 目标与定位
记忆系统用于支持 Agent 的**跨会话连续性**，让助手在长期交互中形成稳定的用户理解（facts / preferences / relationships / important events），并在后续对话中按需召回。

该系统是多用户 SaaS Agent 的核心能力之一，需满足：
- 多租户隔离（按 `agent_id` / tenant 约束）
- 可解释（用户可查看/删除）
- 可演进（MVP 文本检索 → 语义检索）

---

### 1.2 Memory vs Session Summary vs Persona

| 维度 | Memory | Session Summary | Persona |
|---|---|---|---|
| 本质 | 关于用户的长期信息 | 某次会话的压缩上下文 | Agent 行为与风格配置 |
| 时间跨度 | 跨会话长期 | 单会话或近期窗口 | 长期稳定（可配置） |
| 来源 | 对话提取/用户显式/系统推断 | Compaction 过程自动生成 | 产品配置/策略设定 |
| 生命周期 | 可更新、可过期、可删除 | 可重写/覆盖 | 版本化配置 |
| 用途 | “记住用户是谁、经历了什么” | “记住这次聊了什么” | “Agent 应该如何响应” |

**协作关系**：
- Summary 提供近期语境压缩，Memory 提供长期个体认知，Persona 提供行为约束。
- 三者在上下文编排中互补，不互相替代。

---

### 1.3 记忆生命周期（端到端）
1. **生成（Capture）**：从消息流中识别候选记忆（自动/显式/推断）。
2. **标准化（Normalize）**：结构化字段提取、类型标注、置信度评估。
3. **冲突判定（Resolve）**：去重、覆盖、并存或待确认。
4. **存储（Persist）**：写入 `memories`，保留版本与审计信息。
5. **检索（Retrieve）**：按 query + 时效 +重要度 + 可靠性召回 Top-K。
6. **注入（Inject）**：由编排层注入 prompt（已存在，不在本文细化）。
7. **维护（Govern）**：过期、归档、删除、用户管理与合规清理。

---

## 2. 记忆类型与分类

### 2.1 按来源（Source Type）
1. **对话提取（conversation_extracted）**
   - 来源：用户自然对话中被动提取
   - 特征：覆盖广，噪声较高
   - 默认可靠性：中

2. **用户显式告知（user_explicit）**
   - 来源：用户明确表达“记住这个”或同义命令
   - 特征：意图明确，优先级最高
   - 默认可靠性：高

3. **系统推断（system_inferred）**
   - 来源：模型基于多条事实推断（如作息偏好）
   - 特征：价值高但风险高，需可撤销与低默认权重
   - 默认可靠性：中低

---

### 2.2 按内容（Content Type）
1. **事实（fact）**：相对稳定、客观可陈述（如生日、城市）。
2. **偏好（preference）**：主观倾向（如回复简洁、喜欢晨跑）。
3. **事件（event）**：带时间性的经历（如上周去北京）。
4. **关系（relationship）**：人与人关系图谱片段（如妈妈叫李华）。

---

### 2.3 类型属性与优先级建议

| 内容类型 | 典型时效 | 更新频率 | 注入优先级 | 说明 |
|---|---|---|---|---|
| fact | 长 | 低 | 高 | 与身份、长期决策强相关 |
| preference | 中-长 | 中 | 高 | 直接影响回复质量与满意度 |
| relationship | 长 | 低-中 | 中高 | 对情感陪伴与上下文理解关键 |
| event | 短-中 | 高 | 中 | 近期价值高，过期后降权 |

**综合优先级原则**：`user_explicit > conversation_extracted > system_inferred`，同层再按 `importance` 与 `recency`。

---

## 3. 记忆生成机制

### 3.1 自动提取触发条件
在 Runtime 的异步后处理中触发候选提取（已具备该任务位点）。建议触发条件：
- 用户消息包含稳定陈述（“我住在…”、“我一般…”）
- 出现可复用偏好指令（“以后都用简短回答”）
- 出现关系或重要事件信息
- 会话结束或达到消息阈值时批量提取（降低频繁调用成本）

**不提取场景**：闲聊噪声、低确定性猜测、短期无复用价值信息。

---

### 3.2 用户显式“记住这个”处理
- 进入**高优先级显式通道**，可同步确认（“已记住：xxx，可随时删除”）。
- 默认直接入库（除非触发敏感信息策略拦截）。
- 标记 `source_type=user_explicit`、`importance=high`、`reliability=high`。

---

### 3.3 LLM 提取策略（无代码）
建议两阶段：
1. **候选抽取（Recall-oriented）**：宁可多召回，输出结构化候选列表。
2. **验收裁决（Precision-oriented）**：规则+轻量模型判定是否入库、打标签、赋置信度。

输出字段建议：
- `content_type`, `canonical_text`, `entities`, `time_anchor`, `confidence`, `evidence_refs`

并引入**最小证据原则**：每条记忆至少绑定一条来源消息引用（可追溯）。

---

### 3.4 去重与冲突处理

#### 去重（Dedup）
- 规则：同 `agent_id + content_type + normalized_key` 视为同主题记忆。
- `normalized_key` 示例：
  - fact: `birthday`
  - preference: `response_style`
  - relationship: `mother_name`

#### 冲突（Conflict Resolution）
采用显式策略字段：`conflict_strategy`，默认规则如下：
1. **显式新信息覆盖旧信息**（Last-Write-Wins with source priority）
2. 若旧信息可靠性更高且新信息低置信度：进入 `pending_confirmation`
3. 可并存项（如多个兴趣）采用集合合并，不覆盖

最终状态建议：`active / superseded / pending_confirmation / deleted`。

---

## 4. 记忆存储设计

### 4.1 `memories` 表扩展建议（MVP+）
在现有字段（`id, agent_id, content, source, created_at`）基础上新增：

- 分类与来源
  - `source_type` (enum): conversation_extracted / user_explicit / system_inferred
  - `content_type` (enum): fact / preference / event / relationship
- 语义结构
  - `normalized_key` (varchar)
  - `value_json` (jsonb, nullable)
  - `entities_json` (jsonb, nullable)
  - `time_anchor` (timestamp/date, nullable)
- 质量与排序
  - `importance` (smallint, 1-5)
  - `reliability` (smallint, 1-5)
  - `confidence` (float)
  - `last_accessed_at` (timestamp, nullable)
- 生命周期
  - `status` (enum): active/superseded/pending_confirmation/deleted
  - `expires_at` (timestamp, nullable)
  - `supersedes_memory_id` (uuid, nullable)
- 审计与溯源
  - `session_id` (uuid/string, nullable)
  - `evidence_refs_json` (jsonb)  
  - `updated_at` (timestamp)
  - `deleted_at` (timestamp, nullable)

> 说明：YAGNI 原则下，MVP 可先用少量新增字段（`source_type/content_type/importance/reliability/status/updated_at`），其余作为向后兼容扩展。

---

### 4.2 结构化存储 vs 纯文本存储
- **MVP**：以 `content` 为主，辅以少量结构化标签字段，便于快速上线。
- **演进**：引入 `value_json + normalized_key`，增强去重、冲突处理、可解释管理能力。

建议采用**Hybrid Schema**（文本主叙述 + 结构化元数据），兼顾开发速度与长期演进。

---

### 4.3 版本控制与更新机制
两种可选：
1. **行内状态流转（推荐 MVP）**：旧记录标记 `superseded`，新记录为 `active`。
2. **独立版本表（后续）**：`memory_versions` 记录每次变更快照。

MVP 先用方案 1，满足审计与回滚基础需求。

---

## 5. 记忆检索策略

### 5.1 MVP：关键词 + 时间排序
检索流程：
1. Query 关键词匹配 `content`（可含简单分词/LIKE/FULLTEXT）
2. 过滤 `status=active` 且未过期
3. 按综合分排序并取 Top-K

MVP 排序示例：
`score = w1*keyword_match + w2*recency + w3*importance + w4*reliability`

---

### 5.2 演进：语义检索（Vector Retrieval）
在保持 MemoryRepository 抽象不变前提下扩展：
- 为 memory 生成 embedding
- 混合检索（Hybrid Search）：关键词召回 + 向量召回 + rerank
- 对长尾表达与同义问题显著增益

---

### 5.3 过滤与排序细则
过滤维度建议：
- `agent_id`（强隔离）
- `status=active`
- `expires_at > now()`
- 可选 `content_type` 白名单（按场景）

排序维度建议：
- Relevance（关键词/语义相关）
- Recency（时间衰减）
- Importance（业务重要度）
- Reliability（可信程度）

与现有上下文编排对接：输出统一 `MemoryCandidate` 列表，包含 `text + score + metadata`，交由已完成的注入策略消费。

---

## 6. 记忆管理

### 6.1 用户可见与可控
应提供“我的记忆”能力（UI/API）：
- 查看：按类型、时间、来源筛选
- 编辑：纠正错误事实
- 删除：即时生效（对后续检索不可见）
- 禁止记忆开关：用户可临时关闭自动记忆

原则：**可见、可改、可删、可追溯**。

---

### 6.2 过期与清理策略
- `event` 默认 TTL（如 30/90 天，可配置）
- `fact/relationship` 默认不过期，但支持软删除
- `preference` 长期保留，长期未命中可降权或归档

清理分层：
1. 软删除（逻辑删除）
2. 延迟物理删除（满足合规窗口后）

---

### 6.3 容量限制（Quota）
为控制成本和噪声，建议：
- 每 Agent memory 总量上限（条数/存储量）
- 类型配额（event 上限更严格）
- 达上限时执行“低分优先淘汰”或归档策略

---

## 7. 与 Compaction 的协作

### 7.1 Summary 与 Memory 的关系
- **Compaction Summary**：面向会话压缩与短中期连贯。
- **Memory**：面向跨会话长期个体建模。

二者是不同产物，不应混存或互相替代。

---

### 7.2 Compaction 过程中是否提取 Memory
建议：**可以协作，但职责分离**。
- Compaction 后可触发 memory 提取候选（利用摘要降低 token 成本）
- 但 memory 入库必须经过独立的提取/验证/冲突处理流水线
- 避免“摘要噪声直接固化为长期记忆”

实践模式：
- 在线：消息级轻量提取（高实时）
- 离线：会话结束/compaction 后批量提取（高性价比）

---

## 8. Privacy 与安全

### 8.1 敏感信息处理
对记忆内容做敏感分级（PII/SPI）：
- 高敏字段（身份证号、银行卡、医疗细节等）默认不自动记忆
- 用户显式要求保存时，需二次确认（可选）并标记 `sensitivity_level`
- 存储与日志最小化：避免在非必要链路暴露原文

建议字段：`sensitivity_level`（low/medium/high）与 `consent_flag`。

---

### 8.2 用户删除与合规清理
当用户执行删除（单条/全部）或账号注销：
1. 立即逻辑删除并停止检索注入
2. 异步执行物理清理（主库、索引、缓存、向量副本）
3. 审计记录删除动作（不保留可恢复明文）

要求：删除语义应覆盖所有衍生存储，确保“可验证删除”。

---

## 9. 领域架构建议（六边形）

### 9.1 Domain 层核心能力
- `MemoryCaptureService`：候选生成
- `MemoryConflictResolver`：去重与冲突裁决
- `MemoryLifecycleService`：过期/状态流转
- `MemoryRetrievalService`：检索与排序

### 9.2 Port/Adapter 分离
- Port（领域接口）：`MemoryRepository`, `MemorySearchPort`, `MemoryPolicyPort`
- Adapter（基础设施）：SQL 实现、可选向量检索实现、审计日志实现

确保“生成/存储/检索/注入”四个关注点解耦，符合既有约束。

---

## 10. MVP 落地建议（分阶段）

### Phase 1（可上线）
- 文本 memory + 基础分类字段
- 自动提取 + 显式记忆
- 关键词检索 + 时间/重要度排序
- 去重（normalized_key）与基础冲突覆盖
- 用户可查看/删除

### Phase 2（质量增强）
- 结构化 `value_json`
- pending_confirmation 流程
- TTL/归档策略细化
- 审计与可解释页面增强

### Phase 3（语义演进）
- 向量检索与混合召回
- 学习型权重调优（importance/reliability 动态更新）
- 更精细的隐私策略与自动化合规检查

---

## 11. 验收对照（Checklist Mapping）
- ✅ Memory vs Summary vs Persona 区别清晰
- ✅ 记忆类型分类完整（来源+内容）
- ✅ 生成机制覆盖自动提取与显式告知
- ✅ 去重与冲突处理策略明确
- ✅ 存储设计给出扩展字段建议
- ✅ 检索策略覆盖 MVP 与语义演进
- ✅ 与 Compaction 协作关系明确
- ✅ 隐私、删除与清理策略明确

---

## 附：关键设计原则回顾
- 六边形架构：Memory 领域能力内聚，基础设施可替换
- 关注点分离：Capture / Persist / Retrieve / Inject 解耦
- YAGNI：MVP 先文本与规则，接口预留语义扩展
- 显式优于隐式：来源、置信度、冲突策略、状态可追踪
