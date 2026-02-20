# M03 Brief: 基础层设计

## 任务目标

设计客户端基础层（Foundation Layer）的模块划分、职责定义和三方库封装策略。

## 输出位置

`mobile/docs/design/foundation.md`（Markdown）

## 成功标准

1. 明确基础层包含哪些模块（模块清单 + 每个模块的职责）
2. 三方库封装策略：哪些库需要封装、封装的原则是什么
3. 跨平台共享策略：哪些模块三端可共享、哪些平台特定
4. 每个模块的对外接口草案（Protocol/Interface 级别，不到方法签名）
5. 与服务层（M04）的边界说明：什么属于基础层、什么属于服务层
6. 不确定的地方标注 [待确认]

## 必要上下文

### 已确认的技术栈（来自 M02）

- iOS 17+，Swift，SwiftUI
- SPM 依赖管理
- grpc-swift（Apple 官方）
- GRDB（SQLite 封装）
- Swift Concurrency（async/await + actor）
- 不引入 Combine/RxSwift
- 三方库最小化

### 基础层定义（来自 M01 principles.md）

- 通用组件、三方库封装、跨平台共享代码
- 不引入业务语义
- 尽可能跨平台共享
- 不依赖上层

### 产品背景

这是一个 AI agent 聊天平台（SaaS），核心功能：
- 用户与 AI agent 进行文字对话
- 消息通过 gRPC 收发（SendMessage + Subscribe 流式接收）
- 本地缓存会话和消息
- JWT 认证
- MVP 功能极简：注册/登录 → 会话列表 → 聊天

### 基础层可能包含的模块方向（参考，不限于此）

- **网络基础**：HTTP/gRPC 底层封装（注意：网络"服务"在服务层，基础层只做协议封装）
- **存储基础**：SQLite/GRDB 封装（同上，存储"服务"在服务层）
- **日志**：统一日志接口
- **扩展工具**：Swift 语言扩展、常用工具函数
- **UI 基础组件**：可复用的 UI 原子组件（如果有的话）
- **安全基础**：Keychain 封装等

### 基础层 vs 服务层的边界原则

- **基础层**：提供通用能力，不含业务语义，不关心"用来做什么"
  - 例：SQLite 连接管理、HTTP 客户端封装、Keychain 读写
- **服务层**：使用基础层能力组装出业务可用的服务
  - 例：消息存储服务（用 SQLite 存消息）、认证服务（用 Keychain 存 token + HTTP 刷新）

## 参考文档

- 架构原则：`mobile/docs/design/principles.md`
- 技术选型：`mobile/docs/design/tech-stack.md`
- 服务端 crate 结构（参考分层思路）：`server/docs/design/crate-structure.md`

## 约束

- 不要展开服务层设计（M04 的事）
- 不要展开业务模块设计（M05 的事）
- MVP 聚焦 iOS，但模块设计要考虑三端可移植性
- 保持简洁，每个模块一段话说清楚职责和边界
