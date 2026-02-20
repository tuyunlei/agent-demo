# M02 Brief: 技术选型 + 约束

## 任务目标

编写客户端技术选型文档，记录已确认的技术决策及其理由，为后续设计提供约束基线。

## 输出位置

`mobile/docs/design/tech-stack.md`（Markdown）

## 成功标准

1. 覆盖以下所有已确认的选型决策，每项包含：选择、理由、备选方案简述
2. 包含三端（iOS/Android/HarmonyOS）的技术栈对照表
3. 包含 MVP（iOS）的具体版本约束
4. 标注 Android 和 HarmonyOS 的技术选型为 [待确认]（MVP 先做 iOS）
5. 文档简洁，不展开具体的架构设计（那是 M03-M06 的事）

## 已确认的技术决策

### iOS MVP 技术栈

| 项 | 选择 | 理由 |
|---|---|---|
| 最低 iOS 版本 | iOS 17 | 可用 Observation 框架（@Observable），SwiftUI 成熟度更好 |
| 语言 | Swift（跟随 Xcode 最新稳定版） | Apple 原生，无特殊版本要求 |
| UI 框架 | SwiftUI | 声明式 UI，与 SARE 模式天然契合 |
| 依赖管理 | SPM (Swift Package Manager) | Apple 官方，轻量，与 Xcode 集成好 |
| gRPC 客户端 | grpc-swift（Apple 官方维护） | 官方库，SPM 支持，活跃维护 |
| 本地存储 | SQLite（通过 GRDB 封装） | 三端统一（iOS GRDB / Android Room / HarmonyOS 原生 SQLite），成熟稳定 |
| 并发模型 | Swift Concurrency（async/await + actor） | 原生支持，不引入 Combine/RxSwift |

### 三端技术栈对照（规划）

| 层面 | iOS | Android | HarmonyOS |
|---|---|---|---|
| 语言 | Swift | Kotlin | ArkTS |
| UI 框架 | SwiftUI | Jetpack Compose | ArkUI |
| 依赖管理 | SPM | Gradle | ohpm |
| gRPC | grpc-swift | grpc-kotlin | [待确认] |
| 本地存储 | GRDB (SQLite) | Room (SQLite) | 原生 SQLite |
| 并发 | async/await + actor | Coroutines + Flow | [待确认] |
| 页面模式 | SARE（自实现） | MVI（等价于 SARE） | SARE（自实现） |

### 存储选型决策过程

考虑过的方案：
- **SwiftData**：iOS 最省事，但 iOS 独占，三端无法对齐 → 排除
- **Core Data**：成熟但 ObjC 遗产重，学习曲线陡，iOS 独占 → 排除
- **Realm**：跨平台但无 HarmonyOS 支持，MongoDB 收购后方向不明 → 排除
- **SQLite (GRDB)**：三端都有 SQLite，统一存储接口，成熟稳定 → ✅ 选定

核心决策因素：**三端对齐优先于单端便利性。**

### 关键约束

1. **不引入响应式框架**：不用 Combine、RxSwift，统一使用 Swift Concurrency
2. **三方库最小化**：只引入必要的库（grpc-swift、GRDB），其他能力优先自建
3. **存储层接口统一**：服务层定义存储协议（Protocol），GRDB 是 iOS 的实现细节

## 参考文档

- 架构原则：`mobile/docs/design/principles.md`
- 服务端 gRPC 接口定义：`server/docs/design/protocols/grpc-services.md`

## 约束

- 不要展开具体的服务层/基础层设计（M03-M04 的事）
- Android 和 HarmonyOS 的选型标注 [待确认]，不要自行决定
- 保持简洁，每个决策写清楚选了什么、为什么、排除了什么
