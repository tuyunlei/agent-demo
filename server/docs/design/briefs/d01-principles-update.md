# Brief: D01 — 更新架构原则文档

## 任务目标

更新 `docs/design/principles.md`，补充三块目前缺失的内容：
模块化树形结构要求、Walking Skeleton 原则、Rust async trait 策略。

## 输出位置

`server/docs/design/principles.md`

直接修改该文件，在合适的章节插入新内容，不要重写已有内容。

## 成功标准

1. 新增"模块化结构"小节，覆盖：
   - 分层（Infrastructure → Application → Domain）是系统骨架
   - 复杂模块内部再树形拆分（3-4 层子模块），以 Agent Runtime 为例
   - 每个模块的认知载荷要求（一次 session 能完全理解）
   - 每个模块必须可以独立测试（mock 其依赖的 Port）

2. 新增"Walking Skeleton 原则"小节，覆盖：
   - 架构设计要支持最小路径先跑通（不是所有模块都实现完才能运行）
   - 每个 Port 必须能用 Stub 实现替代，不影响其他模块
   - 最小路径示例：SendMessage → Agent Runtime（最简版）→ LLM → 返回
   - 设计时的检查问题：这个模块缺失时，能否用 Stub 先跑整个流程？

3. 在第四节（Rust 特有考量）补充：
   - async trait 策略需要提前决定（dyn Trait + async 还是泛型参数，影响所有 Port 定义，事后改代价大）
   - 所有权即设计：数据所有权关系应该体现模块边界，不能靠 Arc 共享状态来黏合模块（Arc 是工具，不是设计）

## 必要上下文

**架构选型**：六边形架构（Ports & Adapters），已在文档中有详细说明，保持一致。

**分层模型**：
```
Infrastructure Layer（gRPC、PostgreSQL、LLM 客户端）
       ↓ 依赖
Application Layer（Use Cases，如 SendMessageUseCase）
       ↓ 依赖
Domain Layer（Agent、Session、Memory 等核心业务实体）
```
依赖只能由外向内，Domain 不依赖任何框架。

**Agent Runtime 内部树形拆分示例**（说明"复杂模块内部再拆"的概念）：
```
Agent Runtime（Application Layer 内的一个模块）
├── RequestDispatcher   接收请求，路由给正确的 handler
├── ContextAssembler    组装 LLM context（历史 + 人格 + 记忆）
├── ExecutionLoop       驱动 LLM 调用 + 工具执行的主循环
│   ├── LlmCaller       封装 LLM 流式调用
│   └── ToolCoordinator 协调工具请求/结果的收发
└── PostProcessor       处理完成后的收尾（存储、事件发送）
```

**现有文件路径**：
`server/docs/design/principles.md`

## 约束

- 不要重写已有内容，只在合适位置插入新章节或小节
- 不要在文档里讨论实现细节（具体的 Rust 代码、具体的函数签名）
- 保持文档风格一致（中文，markdown，与现有章节格式匹配）
- 不要展开其他模块的设计（只用 Agent Runtime 做举例，不要写 Memory System 等模块的细节）
