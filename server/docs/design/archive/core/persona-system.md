# Persona System 设计（design/core/persona-system.md）

## 1. 人格系统总览

### 1.1 目标与定位
Persona System 用于在多用户托管式 AI Agent 平台中实现“千人千面”，让每个用户拥有具备稳定风格、可持续调教、可受控演化的专属助手。

### 1.2 两层人格结构（Two-layer Persona Architecture）

1. **Platform System Prompt（平台级，不可变）**
   - 定义全局行为边界：安全策略、合规要求、工具调用规则、拒绝策略、品牌底线。
   - 由平台控制，用户不可编辑。
   - 优先级最高，是所有 Agent 的“宪法层”。

2. **User Persona（用户级，可演化）**
   - 定义个体差异：名字、语气、详细程度、互动偏好、行为习惯。
   - 来自 onboarding + 后续用户调教 + 有边界的自动优化。
   - 被平台规则约束，不可突破 system prompt。

> 关系原则：**平台定边界，用户定风格；边界不可突破，风格可持续优化。**

### 1.3 Persona 的组成维度（What defines a persona）
一个完整 persona 由以下维度组成：

- **Identity（身份）**：名称、角色定位、关系称谓（如“你的生活小助手”）
- **Style（表达风格）**：语气、礼貌程度、幽默感、用词习惯、冗长程度
- **Interaction Policy（互动策略）**：先结论后细节、是否主动追问、是否给清单
- **Decision Preference（决策偏好）**：风险偏好、确认阈值、建议力度
- **Capability Preference（能力偏好）**：工具使用倾向、浏览倾向、代码/非代码偏好
- **Safety & Privacy（安全隐私）**：敏感场景处理强度、隐私保护等级
- **Context Preference（上下文偏好）**：语言、时区、单位制、称呼偏好
- **Evolution State（演化状态）**：可自动调整字段、待确认变更、版本与审计信息

---

## 2. Persona 数据结构详细设计

> 设计原则：结构化（可计算）+ 文本化（可解释）；显式字段优先；可版本演进。

### 2.1 扩展后的 `persona_data` JSONB Schema（建议）

```json
{
  "version": "1.1",
  "identity": {
    "agent_name": "小麦",
    "agent_role": "情感陪伴 + 生活助手",
    "self_intro": "我是你温和、可靠的日常助手。",
    "relationship_style": "trusted_friend"
  },
  "profile": {
    "language": "zh-CN",
    "tone": "friendly",
    "verbosity": "medium",
    "emoji_style": "light",
    "humor_level": "low",
    "formality": "casual"
  },
  "interaction": {
    "response_structure": "summary_then_steps",
    "proactivity": "medium",
    "clarification_strategy": "ask_when_ambiguous",
    "confirmation_threshold": "risky_only",
    "instruction_style": "actionable_checklist"
  },
  "behavior_rules": [
    "优先给出可执行步骤",
    "涉及风险操作先确认",
    "先回应情绪，再给建议"
  ],
  "capabilities": {
    "tool_use": true,
    "web_browsing": true,
    "code_assist": false
  },
  "safety": {
    "refuse_illegal": true,
    "privacy_level": "strict",
    "self_harm_policy": "escalate_supportive",
    "medical_legal_finance": "cautious"
  },
  "preferences": {
    "timezone": "Asia/Shanghai",
    "units": "metric",
    "addressing_preference": "你",
    "content_preference_tags": ["效率", "治愈", "简洁"]
  },
  "evolution": {
    "auto_tunable_fields": [
      "profile.verbosity",
      "interaction.proactivity",
      "interaction.response_structure"
    ],
    "require_confirmation_fields": [
      "identity.agent_name",
      "profile.tone",
      "safety.privacy_level",
      "capabilities.*"
    ],
    "drift_guard": {
      "enabled": true,
      "max_style_shift_per_30d": 0.25,
      "require_reanchor_after_days": 30
    },
    "last_evaluated_at": "2026-02-17T01:00:00Z"
  },
  "metadata": {
    "last_editor": "user",
    "last_edited_at": "2026-02-17T01:00:00Z",
    "source": "onboarding",
    "schema_migrated_from": "1.0"
  }
}
```

### 2.2 字段用途、可选值、默认值（MVP）

| 字段 | 用途 | 可选值示例 | 默认值 |
|---|---|---|---|
| `version` | schema 版本控制 | `1.0`/`1.1` | `1.1` |
| `identity.agent_name` | Agent 名称 | 任意字符串（长度受限） | `小助手` |
| `identity.agent_role` | 角色定位 | 固定枚举或短文本 | `情感陪伴 + 生活助手` |
| `identity.relationship_style` | 关系风格 | `assistant`/`trusted_friend`/`coach` | `assistant` |
| `profile.language` | 输出语言 | `zh-CN`/`en-US` | `zh-CN` |
| `profile.tone` | 主语气 | `friendly`/`calm`/`professional` | `friendly` |
| `profile.verbosity` | 信息密度 | `low`/`medium`/`high` | `medium` |
| `profile.emoji_style` | emoji 频率 | `none`/`light`/`rich` | `light` |
| `interaction.response_structure` | 答复组织方式 | `direct`/`summary_then_steps`/`guided` | `summary_then_steps` |
| `interaction.proactivity` | 主动性 | `low`/`medium`/`high` | `medium` |
| `interaction.confirmation_threshold` | 何时二次确认 | `always`/`risky_only`/`never` | `risky_only` |
| `behavior_rules[]` | 明确行为规则 | 自然语言规则列表 | 平台推荐 2~4 条 |
| `capabilities.*` | 功能开关偏好 | bool | 按平台默认能力 |
| `safety.privacy_level` | 隐私严格度 | `strict`/`balanced` | `strict` |
| `preferences.timezone` | 时间语义 | IANA 时区 | 用户时区或系统默认 |
| `evolution.auto_tunable_fields[]` | 自动可调字段白名单 | 字段路径列表 | MVP 固定列表 |
| `evolution.require_confirmation_fields[]` | 需用户确认字段 | 字段路径列表 | MVP 固定列表 |
| `metadata.last_editor` | 审计信息 | `user`/`agent`/`system` | `system` |

### 2.3 `persona_text` 与 `persona_data` 的关系

**结论：互补，不是简单冗余。**

- `persona_data`：结构化“事实与参数”，用于可控渲染、验证、策略判断、演化 diff。
- `persona_text`：自然语言“人设叙事”，用于提升语言风格一致性与情感连贯性。

推荐策略：
1. **`persona_data` 为主数据源（source of truth）**。
2. `persona_text` 由 `persona_data + 模板` 生成初稿，可允许人工润色。
3. 每次更新后做一致性校验（例如 tone 冲突检测）；冲突时以 `persona_data` 为准并重生成 `persona_text`。

---

## 3. Persona 生成机制

## 3.1 Onboarding 生成流程（对话式）

采用“最少问题、足够建模”的 3~5 轮采样：

1. **身份采样**：名字、希望被如何称呼、希望助手像哪类角色（朋友/管家/教练）。
2. **风格采样**：喜欢简短还是详细、温柔还是直接、是否使用 emoji。
3. **行为采样**：遇到不确定信息是否先追问、风险操作是否必须确认。
4. **偏好采样**：语言、时区、单位制、兴趣标签。
5. **确认与回显**：系统生成“人格卡片摘要”，用户一键确认或修改。

输出：
- `persona_data`（结构化）
- `persona_text`（可读描述）
- `metadata.source = onboarding`

### 3.2 跳过 onboarding 的默认模板（兜底）

当用户跳过时，使用 **Safe-Friendly Default Persona**：

- 名称：`小助手`
- 语气：`friendly + calm`
- 冗长度：`medium`
- 策略：`summary_then_steps` + `risky_only confirm`
- 隐私：`strict`
- 自动演化：仅允许微调 `verbosity/proactivity`

该模板保证：开箱即用、风险低、后续可逐步个性化。

### 3.3 生成时调用 LLM 的策略

- **双阶段生成**：
  1) 提取阶段：将 onboarding 对话映射为结构化 JSON（强 schema 约束）
  2) 文案阶段：基于 JSON 生成 `persona_text`
- **低温度生成**：减少随机漂移，提高稳定性。
- **强校验链路**：
  - JSON schema validation
  - 枚举值规范化（如 `friendly`）
  - 安全字段覆盖校验（不可被弱化）
- **失败回退**：解析失败则回退默认模板 + 最小用户已知偏好。

---

## 4. Persona 演化机制

> MVP 原则：先“可控微调”，再“自主进化”。

### 4.1 演化来源

1. **显式反馈**（高优先级）
   - 用户直接说“以后简短点”“别用表情”“你叫小麦吧”。
2. **隐式信号**（低风险使用）
   - 长期行为统计：用户常要求“简洁总结”、经常追问细节等。
3. **周期性评估**
   - 每 N 次会话或每 7 天评估一次可调字段。

### 4.2 自动调整 vs 用户确认

**自动调整（MVP 可启用）**
- `profile.verbosity`
- `interaction.proactivity`
- `interaction.response_structure`

**必须用户确认（MVP 必须）**
- 名称/角色：`identity.*`
- 核心风格：`profile.tone`
- 安全隐私：`safety.*`
- 能力开关：`capabilities.*`

### 4.3 触发条件与频率

- **事件触发**：出现显式偏好指令时立即生成变更提案。
- **批处理触发**：达到会话阈值（如 20 轮）或时间阈值（如 7 天）后评估。
- **限流机制**：每 24h 最多应用 1 次自动变更，避免频繁波动。

### 4.4 人格漂移防护（Drift Guard）

- **Anchor Constraints（锚点约束）**：身份、核心语气、安全策略不可自动偏移。
- **Delta Cap（变化幅度上限）**：30 天累计风格变化不超过阈值。
- **Human-in-the-loop**：高影响字段必须确认。
- **可回滚**：保留 persona 版本快照（至少最近 10 版），支持一键回退。
- **冲突检测**：若新提案与平台规则冲突，直接拒绝并记录原因。

---

## 5. Persona 渲染

### 5.1 渲染目标
将 `persona_data + persona_text` 渲染为高可控、低 token 成本、稳定执行的 prompt 片段，位于 system prompt 后第 2 段。

### 5.2 Full Persona Render（完整版）

建议结构：
1. `Identity Brief`（名称、关系定位）
2. `Style Contract`（语气、冗长度、表达格式）
3. `Interaction Rules`（确认阈值、追问策略、输出结构）
4. `Safety Overlay`（隐私与风险边界，引用平台策略）
5. `User Preferences`（时区/单位/称呼）
6. `Narrative Persona Text`（短段落，不超过预算）

### 5.3 Compact Persona Render（精简版）

当 token 预算不足（persona > 上下文 10% 或超上限）时：

- 保留优先级：
  1) 安全与边界字段
  2) 交互关键策略（确认阈值、输出结构）
  3) 风格核心字段（tone + verbosity）
  4) 名称与称呼
- 压缩策略：
  - 列表去重
  - 删除低影响叙事描述
  - `persona_text` 摘要为 1~2 句
- 输出形式：结构化键值 + 最短自然语言补充。

---

## 6. Persona 管理接口（Domain 视角）

> 六边形架构下，Persona 逻辑在 Domain；存储与外部接口通过 Ports/Adapters。

### 6.1 用户可执行操作（MVP）

- 查看人格卡（当前设定）
- 修改名称（rename）
- 调整风格（语气、详略、emoji）
- 调整行为偏好（是否先追问、是否风险确认）
- 切换/编辑规则（behavior rules）
- 重置为默认模板（soft reset）
- 回滚到历史版本（version rollback）

### 6.2 Repository / Service 能力边界（建议）

**Domain Services（示意）**
- `PersonaBootstrapService`：onboarding 生成初始 persona
- `PersonaEvolutionService`：生成演化提案与可应用变更
- `PersonaRenderService`：渲染 full/compact prompt 片段
- `PersonaPolicyService`：字段白名单、确认策略、漂移防护

**Repository Port（示意）**
- `getByAgentId(agentId)`
- `save(agentId, personaData, personaText, versionMeta)`
- `listVersions(agentId, limit)`
- `rollback(agentId, targetVersion)`
- `appendAuditLog(agentId, event)`

**Application 层编排职责**
- 鉴权、请求校验、调用 Domain Service、返回 DTO。

---

## 7. 安全与边界

### 7.1 防 persona 注入（Prompt Injection via Persona）

风险：用户在 persona 文本中注入“忽略系统规则”“泄露隐私”等恶意指令。

防护策略：
1. **分层隔离**：platform system prompt 永远高优先级，不允许 persona 覆盖。
2. **字段白名单**：persona_data 仅允许预定义字段与枚举。
3. **危险模式过滤**：拦截 persona_text 中越权语句（如“ignore previous instructions”）。
4. **语义审计**：保存前进行 policy classifier 检测（越权/违法/隐私削弱）。
5. **不可下放安全权重**：`safety` 关键字段最小安全基线不可被用户降低。

### 7.2 Persona 审核策略（Moderation）

- **保存前审核（pre-save）**：
  - schema validation
  - 内容安全分类（违法、仇恨、自伤误导等）
  - 隐私策略一致性检查
- **渲染前审核（pre-render）**：
  - 再次检查是否含越权指令或冲突字段
- **运行时兜底（runtime guard）**：
  - 即便 persona 被污染，系统层安全策略仍拦截危险响应
- **审计可追溯**：
  - 记录谁在何时修改了哪些字段、审核结果与拒绝原因

---

## 8. MVP 与演进路线（YAGNI）

### 8.1 MVP（首版必须）
- 两层结构落地（system + user persona）
- onboarding 生成 `persona_data + persona_text`
- 手动编辑关键字段
- full/compact 渲染
- 自动微调仅支持低风险字段
- 版本记录与回滚
- 基础审核与注入防护

### 8.2 后续扩展（预留）
- 多 persona 模板市场（persona presets）
- 场景化子人格（工作/情感/学习）
- 更精细的多目标优化（满意度、完成率、风险率）
- A/B persona 策略实验

---

## 9. 验收清单对照

- [x] 平台级与用户级关系清晰
- [x] persona_data schema 完整，字段有用途和默认值
- [x] persona_text 与 persona_data 关系明确
- [x] onboarding 生成流程描述完整
- [x] 演化机制含触发条件、保护措施、用户确认机制
- [x] 渲染覆盖完整版与精简版
- [x] 安全边界与审核策略明确
