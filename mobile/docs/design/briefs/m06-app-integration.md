# M06 Brief: 应用集成层设计

## 任务目标

设计客户端应用集成层（App Integration Layer）：启动流程、依赖注入/服务注册、导航/路由架构。

## 输出位置

`mobile/docs/design/app-integration.md`（Markdown）

## 成功标准

1. App 启动流程设计（从 App 入口到首屏展示的完整链路）
2. 依赖注入方案（服务注册与分发方式）
3. 导航/路由架构（页面间跳转规则、deep link 预留）
4. Root 页面逻辑（如何根据 auth 状态决定展示登录 vs 主界面）
5. 与 SwiftUI App lifecycle 的对接方式
6. 三端一致性说明
7. 不确定的地方标注 [待确认]

## 必要上下文

### 已定的业务模块（来自 M05）
- Auth 模块：Login/Register 页面
- Session 模块：SessionList 页面
- Chat 模块：ChatRoom 页面
- Settings 模块（可选）：Settings 页面

### 已定的服务层（来自 M04）
- AuthService, APIClientService, ChatService, SessionService, StorageService, ConnectivityService, ToolExecutionService

### 已定的基础层（来自 M03）
- NetworkCore, StorageCore, SecureStoreCore, LogCore, ConcurrencyCore, PlatformCore

### 应用集成层定义（来自 M01）
- 组合根 + 启动编排 + 服务注册 + Root 页面装配
- 平台特定
- 不承载具体业务规则

### 技术栈
- SwiftUI App lifecycle（`@main struct MyApp: App`）
- Swift Concurrency
- iOS 17+（@Observable 可用）

### MVP 导航结构（参考）
```
App Entry
  └─ Root (auth check)
      ├─ Auth Flow (Login → Register)
      └─ Main Flow
          ├─ Session List
          │   └─ Chat Room
          └─ Settings (tab or menu)
```

## 参考文档

- 架构原则：`mobile/docs/design/principles.md`
- 业务模块：`mobile/docs/design/business-modules.md`
- 服务层：`mobile/docs/design/services.md`
- 基础层：`mobile/docs/design/foundation.md`

## 约束

- iOS 特定（SwiftUI），不需要写 Android/HarmonyOS 的启动实现
- 但要说明哪些概念可三端复用（如 DI 策略、路由模型）
- DI 方案要轻量，不引入重框架（如 Swinject），优先手动 DI 或 protocol + factory
- 保持简洁，不重复 M03-M05 已定义的接口
