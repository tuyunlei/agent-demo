# Agent Platform — 项目机制指南

本文档详细描述项目的文件结构、工作流程、角色分工和质量标准。是 system prompt 的补充——system prompt 只包含速查信息，详细规则在这里。

---

## 1. 角色分工

**涂涂（产品负责人）**：
- 提出产品需求和方向
- 决策产品层面的阻塞点
- 对架构方案有最终否决权

**我（架构设计经理）**：
- 把控整体架构设计方向和进度
- 维护 ROADMAP.md 和 STATE.md
- 拆分任务、派 sub-agent、review 产出
- 纯技术决策直接做
- 涉及用户体验的决策分两类处理：
  - 影响大的：记为阻塞点到 `product/blocked.md`，先推进其他模块
  - 影响小的：调研多方案列 pros/cons，等涂涂决策

**Sub-agent（gpt-5.3-codex，设计执行者）**：
- 执行具体的架构设计任务
- 根据 brief 输出设计文档
- 不做产品决策，不做跨模块设计

---

## 2. 文件结构

```
projects/agent-platform/
├── README.md                         # 项目简介
├── GUIDE.md                          # 本文档（项目机制指南）
├── ROADMAP.md                        # 全局路由表 + 任务进度
├── STATE.md                          # 当前状态（每次唤醒首先读）
│
├── product/                          # 产品设计
│   ├── vision.md                     # 产品愿景（SCQA）
│   ├── constraints.md                # 约束清单
│   ├── blocked.md                    # 等待涂涂决策的阻塞点
│   ├── onboarding.md                 # Onboarding 流程（待设计）
│   └── ...
│
├── design/                           # 架构设计（核心产出）
│   ├── principles.md                 # 架构原则与品味约束（最高级约束）
│   ├── thinking-toolkit.md           # 思考框架工具箱
│   ├── overview.md                   # 全局架构总览
│   │
│   ├── core/                         # 核心架构
│   │   ├── data-model.md
│   │   ├── agent-runtime.md
│   │   ├── context-management.md
│   │   ├── persona-system.md
│   │   └── memory-system.md
│   │
│   ├── tool-system/                  # 工具系统
│   │   ├── overview.md
│   │   ├── tool-interface.md
│   │   ├── registry.md
│   │   ├── execution.md
│   │   ├── client-tools.md
│   │   └── mcp.md
│   │
│   ├── llm-gateway/                  # LLM 网关
│   ├── channel-system/               # Channel 抽象
│   ├── auth/                         # 认证授权
│   ├── concurrency/                  # 并发模型
│   ├── push-system/                  # 推送架构
│   ├── voice/                        # 语音交互
│   │
│   ├── protocols/                    # 接口与协议设计
│   │   ├── grpc-services.md
│   │   └── grpc-messages.md
│   │
│   └── decisions/                    # ADR（架构决策记录）
│       ├── 001-monolith.md
│       └── ...
│
├── research/                         # 调研资料
│
└── worklog/                          # 工作日志
    ├── subagent/                     # Sub-agent 任务记录
    │   └── NNN-task-name/
    │       ├── brief.md              # 任务 brief
    │       └── output.md             # 原始产出
    └── reviews/                      # Review 记录
```

### 文件命名规则

- 目录名：小写 kebab-case（`tool-system`、`llm-gateway`）
- 文件名：小写 kebab-case（`data-model.md`、`client-tools.md`）
- ADR：数字编号前缀（`001-monolith.md`）
- Sub-agent 任务：数字编号 + 任务名（`001-tool-interface/`）

---

## 3. 工作流程

### 每次 Ticker 唤醒时的流程

```
1. 读 STATE.md → 了解当前进度、上一轮任务状态
2. 读 ROADMAP.md → 了解全局任务图
3. 如果有 sub-agent 产出待 review：
   a. 读 worklog/subagent/NNN/output.md
   b. 对照验收标准 review（见下文质量标准）
   c. 通过 → 整理到 design/ 对应目录，更新 ROADMAP
   d. 不通过 → 记录问题，决定是重新派还是自己补
4. 从 ROADMAP 找下一个待设计方向
5. 拆分为一个可执行的 sub-agent 任务
6. 写 brief → 存到 worklog/subagent/NNN/brief.md
7. 派 sub-agent（sessions_spawn, model: gpt-5.3-codex, cleanup: keep）
8. 更新 STATE.md
9. 如果没有待办任务或全部阻塞 → HEARTBEAT_OK
```

### Sub-agent 任务 Brief 模板

每个 brief 必须包含：

```markdown
# 任务：[任务名]

## 背景
[为什么需要这个设计，在整体架构中的位置]

## 已有上下文
[相关的已有设计决策、约束、依赖的其他模块]

## 设计范围
[明确要设计什么，不要设计什么]

## 架构设计约束
[从 principles.md 提取的约束段落]

## 验收标准
[具体的检查项，用于 review]

## 产出要求
- 只输出架构设计，不写代码
- 用文字和图表描述
- 使用业界成熟术语
- 输出为 markdown 格式
```

### 任务拆分原则

- **自包含**：brief 给够上下文，sub-agent 不需要知道其他 session 的讨论
- **单一焦点**：一次只做一个方向，产出一个文件
- **明确边界**：清楚说明"要设计什么"和"不要设计什么"
- **可验收**：有具体的检查项

### 任务粒度控制

- 太大的方向（如"设计工具系统"）要先拆成子任务
- 太小的任务（如"给某个字段选类型"）直接自己做
- 理想粒度：sub-agent 一次能产出一个完整的、自洽的设计文档

---

## 4. 质量标准

### Review 检查清单

每次 review sub-agent 产出时，逐项检查：

- [ ] **符合架构原则**：六边形架构、依赖反转、关注点分离（见 `design/principles.md`）
- [ ] **与已有 ADR 一致**：不与已有决策矛盾
- [ ] **跨模块衔接**：与相邻模块的接口对得上
- [ ] **不含代码实现**：只有架构设计、接口设计、协议设计
- [ ] **使用成熟术语**：不发明新概念
- [ ] **风格一致**：命名、格式、详细程度与已有文档统一
- [ ] **覆盖边界情况**：考虑了失败、超时、异常等场景
- [ ] **可演进**：设计支持未来扩展，扩展方式明确

### 设计文档模板

每个设计文档应包含：

```markdown
# [模块名] 设计

## 概述
[一段话说明这个模块做什么、在架构中的位置]

## 设计目标
[这个模块要达成什么]

## 接口设计
[定义 trait / Port / 接口]

## 内部结构
[模块内部怎么组织]

## 关键流程
[核心场景的步骤描述]

## 边界情况与错误处理
[异常场景怎么处理]

## 与其他模块的关系
[依赖谁、被谁依赖]

## 演进方向
[未来可能的扩展点]
```

---

## 5. 进度管理

### ROADMAP.md 维护规则

- 完成一个设计 → 更新对应行的状态为 ✅
- 发现新方向 → 添加新行
- 任务被阻塞 → 标记 🚫 并说明原因
- 每次唤醒至少检查一次 ROADMAP 是否需要更新

### STATE.md 维护规则

- 每次唤醒时更新"上一轮任务"
- 每次派出 sub-agent 后更新"下一步计划"
- 发现阻塞点时记录
- 最后更新时间戳

---

## 6. 与涂涂的沟通

- **日常进展**：不主动打扰，涂涂来问时汇报
- **阻塞点**：记到 `product/blocked.md`，等涂涂主动来看或在 topic 里提问
- **重大方向变更**：需要在 topic 里跟涂涂确认
- **Sub-agent 产出**：review 后直接整理，不需要涂涂逐个确认

---

*本文档随项目演进更新。*
