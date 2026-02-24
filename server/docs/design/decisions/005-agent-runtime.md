# ADR-005: Agent Runtime 核心流程（TurnExecutor + ContextBuilder）

**状态**: Updated（原决策已扩展）
**日期**: 2026-02-24

## 上下文

原 ADR-005 确立了“用户输入→上下文组装→LLM→工具循环→持久化”的主流程，但当时上下文构建职责仍较为内嵌，且未与新四层边界完全对齐。

新架构要求：

- 编排层由 TurnExecutor 统一驱动一次 turn 状态机
- ContextBuilder 从编排层中独立为能力层模块
- 事件流成为 runtime 的事实输入/输出
- gRPC/HTTP 等接入层只调用编排入口，不携带流程细节

因此需要更新 runtime 决策，明确职责收敛与依赖方向。

## 决策

采用“**编排层 TurnExecutor + 能力层 ContextBuilder**”的运行时结构：

1. **TurnExecutor（Layer 2）** 负责 turn 状态机编排：
   - load/create session
   - 追加用户事件
   - 调用 ContextBuilder
   - 调用 LLM
   - 驱动工具循环
   - 持久化事件
   - 触发压缩/后处理
2. **ContextBuilder（Layer 3）** 独立负责：
   - PromptSection 组合构建 system prompt
   - 事件流到模型消息的映射与裁剪
   - token 预算感知的历史选择
3. TurnExecutor 仅依赖能力层 trait（ContextBuilder/LlmProvider/ToolRuntime/SessionLifecycle），不得依赖基础设施具体实现。
4. 工具循环必须有显式上限（`max_tool_iterations`），避免失控。
5. 失败策略分层：
   - 工具失败以 ToolCallResult 回填模型
   - LLM 失败按 retry/fallback 处理
   - 持久化失败可配置降级（默认返回文本并告警）

## 理由

- **关注点分离**：上下文策略与流程编排解耦，便于演进与测试。
- **复杂度可控**：TurnExecutor 聚焦状态机，不承担 prompt 拼装细节。
- **可测试性提升**：trait 注入后可完整 mock 工具循环与失败路径。
- **与事件流一致**：编排的每个关键步骤都可沉淀为事件。
- **跨协议复用**：接入层统一调用 run_turn，减少逻辑重复。

## 后果

### 正面影响
- runtime 结构更清晰，模块边界稳定。
- 上下文策略可独立迭代（例如 Summary/Marker/Token 策略）。
- 对多 provider、多工具组合场景更易扩展。

### 负面影响
- 初期需要重构旧的内嵌上下文逻辑。
- trait 边界增加了接口设计与契约测试成本。
- 若边界治理不严，仍可能出现编排层“偷穿”基础设施风险。

## 参考

- `docs/design/orchestration/turn-executor.md`
- `docs/design/capabilities/context-builder.md`
- `docs/design/architecture.md`
- `docs/design/AUDIT.md`

## 历史背景（保留）

以下内容来自 2026-02-16 的原 ADR-005（早期 runtime 基线）：

- 主流程：接收消息 → 组装上下文 → 调 LLM → 工具循环 → 持久化 → 返回 → 异步后处理
- 工具执行失败：将错误作为 tool_result 回给 LLM
- LLM 失败：使用 fallback provider，最终失败再向用户报错
- 保留了流式与客户端工具扩展意图

以上结论仍有效，但现已通过 TurnExecutor/ContextBuilder 分层进行结构化落地。