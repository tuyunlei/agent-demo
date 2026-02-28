# ADR-008: ContextBuilder 可插拔 PromptSection

**状态**: Accepted
**日期**: 2026-02-24

## 上下文

System Prompt 在 agent 行为稳定性、可维护性和成本控制上影响极大。早期用“单一大字符串模板”拼接虽快，但在新架构下暴露出问题：

- 变更耦合高：小改动常影响整段模板
- 可测试性差：难对局部规则做独立验证
- 审计与 diff 粒度粗：无法清晰定位哪部分变化导致行为变化
- prompt cache 不友好：动态内容污染稳定前缀

同时，ContextBuilder 已独立为能力层模块，需要更可插拔、更可治理的 system prompt 生成机制。

## 决策

采用“**ContextBuilder + 可插拔 PromptSection 组合**”构建 System Prompt：

1. System Prompt 不再由单一模板硬编码生成。
2. 每个 section 实现统一 trait（如 `name/build/enabled/order/is_stable`）。
3. 通过 composer 按顺序组装 section。
4. 稳定 section 与动态 section 可区分处理，支持缓存与版本化。
5. 配置变更通过事件流（ConfigChange）驱动重建，而不是每轮重建全部模板。

## 理由

对比三种方案：

1. **硬编码单模板**
   - 优点：实现最快
   - 缺点：维护困难、不可测试、易破坏缓存
2. **可插拔 PromptSection（本决策）**
   - 优点：模块化、可测试、可审计、易扩展、cache 友好
   - 缺点：需要定义 trait 与组合器，初期结构成本更高
3. **纯配置文件驱动模板**
   - 优点：运行时可配置强
   - 缺点：复杂逻辑表达弱、类型安全与调试体验较差（MVP 阶段）

当前阶段优先选择 PromptSection：在可维护性、可测试性和演进效率上最平衡。

## 后果

### 正面影响
- prompt 变更粒度清晰，可独立评审与回归。
- ContextBuilder 职责完整，编排层不再拼接 prompt。
- 便于按租户/场景注入扩展 section。
- 更利于 prompt cache 命中与成本控制。

### 负面影响
- 需要维护 section 顺序与依赖约束。
- 需要建立 section 级测试与诊断规范。
- 配置化能力若后续增强，需要与 trait 模型协同设计。

## 参考

- `docs/design/capabilities/context-builder.md`
- `docs/design/core/event-model.md`
- `docs/design/architecture.md`
- `docs/design/orchestration/turn-executor.md`