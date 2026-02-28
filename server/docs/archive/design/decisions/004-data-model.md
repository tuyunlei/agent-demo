# ADR-004: 数据模型设计（从消息快照到事件流）

**状态**: Updated（原决策已扩展）
**日期**: 2026-02-24

## 上下文

随着架构升级到四层模型（Channel → Orchestration → Capability ← Infrastructure），原 ADR-004 中以 `messages` 快照为核心的数据建模已无法完整支撑以下需求：

- 工具调用与结果需要一等建模，而不仅是消息附属字段
- 会话压缩需要可追踪的替代边界（Summary + Marker）
- 编排层需要可回放、可审计的 turn 事实序列
- ContextBuilder 需要事件级输入以支持稳定的 prompt 构建与裁剪策略

审计文档 `AUDIT.md` 明确指出 ADR-004 需更新为 append-only 事件流方向。

## 决策

在保留 PostgreSQL 存储基础上，将会话数据模型从“消息快照模型”升级为“append-only 事件流模型”：

1. **事件流为会话事实源（source of truth）**，按 `session_id + sequence_number` 追加写入。
2. 消息列表、上下文窗口、压缩结果均视为事件流投影，而非唯一真相。
3. 事件类型至少包括：
   - UserMessage
   - AssistantMessage
   - ToolCallRequest
   - ToolCallResult
   - SystemEvent
   - ConfigChange
   - Summary
   - CompactionMarker
4. System Prompt 配置变化通过 `ConfigChange` 事件表达，不回写历史记录。
5. 原 `messages` 相关设计作为历史阶段可参考模型，不再作为未来主模型。

## 理由

- **审计与可回放**：事件不可变追加天然适合排障、合规、行为回放。
- **压缩友好**：Summary/CompactionMarker 可显式表达“被替代历史区间”。
- **编排清晰**：TurnExecutor 可把每一步关键动作沉淀为事件，流程可解释。
- **Prompt cache 友好**：通过 ConfigChange 事件驱动配置演进，避免每轮重写 system prompt。
- **演进性强**：新增领域行为通常可新增事件类型，不需重塑消息表语义。

## 后果

### 正面影响
- 数据语义更完整，支持工具链路与生命周期事件。
- 调试能力增强，可还原 turn 级执行过程。
- 与 ContextBuilder、TurnExecutor、新分层架构完全对齐。
- 为后续分析、审计、离线重建提供统一数据基础。

### 负面影响
- 读模型复杂度提升，需要投影与裁剪策略。
- 迁移期需兼容旧 `messages` 读写路径。
- 开发团队需适应事件思维（而非“直接改状态”）。

## 参考

- `docs/design/core/event-model.md`
- `docs/design/architecture.md`
- `docs/design/AUDIT.md`

## 历史背景（保留）

以下内容来自 2026-02-16 的原 ADR-004，代表消息快照阶段的设计基线：

- 实体关系：`User 1:N Agent 1:N Session 1:N Message`，以及 `Agent 1:N Memory`
- Session 被定义为用户视角对话，压缩对用户透明
- Message 表承载 role/content/tool_calls/tool_result 等字段
- 索引重点为 `messages (session_id, created_at)`
- 明确“存储与上下文分离”：持久化完整历史，上下文只取 summary + 最近 N 条

该历史基线在 MVP 早期有效，现已被事件流模型扩展与替代。