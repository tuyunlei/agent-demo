# orchestrator — Roadmap

## 架构决策（已确定）

- **角色**：Planner（任务拆解）、Dev（写代码）、Reviewer（审查）— 不要 Architect
- **模型**：Planner = Claude Sonnet 4.6，Dev = gpt-5.3-codex，Reviewer = gpt-5.3-codex
- **通信**：事件驱动，`send_message` function tool + asyncio.Queue 消息总线
- **拓扑**：Planner 是中心节点，Dev/Reviewer 互不通信
- **三层**：涂涂 → 小小涂（机制层，管 orchestrator 本身）→ Planner/Dev/Reviewer（项目层）
- **语言**：Python（LiteLLM 开箱即用）
- **运行模式**：当前一次性脚本，后续考虑 daemon

---

## Phase 1 — 能跑

最小可用：三个 agent 能协作完成一个真实任务。

| 步骤 | 内容 | 状态 |
|------|------|------|
| S1 | 环境搭建：venv + openai-agents + Codex MCP | ✅ |
| S2 | 验证：API 连通、Agents SDK、Codex MCP 文件操作 | ✅ |
| S3 | 三个角色的 instructions | 🔜 |
| S4 | 消息编排完善：from 字段、循环上限、错误处理、Human 恢复 | |
| S5 | 真实任务验证：从 ROADMAP 挑一个小任务跑完整流程 | |

### S3 细节

Planner instructions 最复杂，需要：
- 读 server/ROADMAP.md 了解待办
- 读 server/AGENTS.md 了解架构
- 读 server/KNOWN_ISSUES.md 了解已知问题
- 拆解任务为 Dev 可执行的粒度
- 判断 Dev 产出是否需要 Reviewer
- 判断任务是否完成、报告给 Human

Dev instructions：
- Codex MCP 操作项目文件
- 写代码 + 写测试
- 完成后 send_message 给 Planner

Reviewer instructions：
- 只读审查（不改代码）
- 对照架构约束检查
- 报告问题给 Planner

---

## Phase 2 — 能用

加护栏，让流程可靠。

| 步骤 | 内容 |
|------|------|
| S6 | Guardrails：输出校验、敏感信息过滤 |
| S7 | Git 工作流集成：自动创建 feature 分支、commit、开 PR |
| S8 | CI 检查：PR push 后等 CI 结果，红了自动修 |
| S9 | Human 恢复机制：流程卡住时人工介入入口 |

---

## Phase 3 — 可观测

看得见发生了什么。

| 步骤 | 内容 |
|------|------|
| S10 | 事件日志：所有消息、工具调用、agent 决策持久化 |
| S11 | 效率指标：任务耗时、token 消耗、轮次数 |
| S12 | 质量指标：review 拒绝率、CI 失败率 |
| S13 | 产出指标：文件/行数变更统计 |
| S14 | Dashboard：汇总展示 |

---

## Phase 4 — 通用化

不只服务 agent-demo。

| 步骤 | 内容 |
|------|------|
| S15 | 项目配置解耦：Planner instructions 参数化，换项目只换配置 |
| S16 | 多项目支持：同时管理多个项目 |

---

*最后更新：2026-03-02*
