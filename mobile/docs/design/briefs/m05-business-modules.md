# M05 Brief: 业务模块划分

## 任务目标

定义客户端业务层（Business Layer）的模块划分、模块间依赖、每个模块的内部结构（SARE 应用）。

## 输出位置

`mobile/docs/design/business-modules.md`（Markdown）

## 成功标准

1. MVP 业务模块清单（每个模块的职责 + 边界 + 拥有的页面）
2. 每个模块的内部结构（如何应用 SARE 模式）
3. 模块间依赖关系图
4. 每个模块依赖哪些服务层接口
5. 模块间通信方式（事件/接口/共享模型）
6. 三端一致性说明（哪些逻辑三端对齐，哪些平台特定）
7. 不确定的地方标注 [待确认]

## 必要上下文

### MVP 核心用户流程

1. **启动 → 登录/注册**（首次使用）
2. **会话列表**：看到所有对话
3. **进入聊天**：与 AI agent 对话，实时收到回复
4. **（可选）设置/个人信息**

### SARE 模式（来自 M01 principles.md）

每个页面/模块统一使用 State + Action + Reducer + Effect：
- State：完整不可变 UI 状态
- Action：用户操作或系统事件
- Reducer：纯函数 (State, Action) → NewState
- Effect：副作用，显式隔离

### 业务层定义（来自 M01 principles.md）

- 按业务垂直切模块
- 子层按需：接口层（可选）/ 实现层（必需）/ UI 层（必需，分离）/ Model 层（按需）
- 不直接操作底层 SDK
- 接口 + 逻辑三端对齐，UI 平台特定

### 服务层接口（来自 M04 services.md）

- `AuthService`：注册/登录/token/auth 状态
- `ChatService`：发消息/订阅流/提交工具结果
- `SessionService`：会话列表/详情/历史消息
- `StorageService`：本地缓存
- `ConnectivityService`：网络状态
- `ToolExecutionService`：工具执行

### MVP 模块候选（参考方向）

1. **Auth 模块**：登录/注册页面 + auth 流程
2. **Session 模块**：会话列表页
3. **Chat 模块**：聊天页面（核心，最复杂）
4. **Settings 模块**（可选）：个人设置

## 参考文档

- 架构原则：`mobile/docs/design/principles.md`
- 服务层设计：`mobile/docs/design/services.md`
- 基础层设计：`mobile/docs/design/foundation.md`

## 约束

- 只设计 MVP 必需的模块，不超前设计
- 每个模块必须展示 SARE 如何落地（State 长什么样、有哪些 Action）
- Chat 模块是重点，需要最详细的设计
- 不要展开应用集成层（M06 的事）
- 保持简洁，每个模块用统一结构描述
