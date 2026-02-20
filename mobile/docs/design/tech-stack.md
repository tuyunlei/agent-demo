# M02 技术选型与约束（客户端）

> 范围：iOS / Android / HarmonyOS  
> 目标：沉淀已确认技术决策，作为后续 M03-M06 设计约束基线。

## 1. 选型结论（MVP 范围）

MVP 当前仅实现 iOS，以下为 **已确认并生效** 的技术选型：

| 项 | 选择 | 理由 | 备选方案简述 |
|---|---|---|---|
| 最低 iOS 版本 | iOS 17 | 可使用 Observation（`@Observable`），SwiftUI 成熟度更好 | 更低版本会增加状态管理兼容成本，影响开发效率 |
| 语言 | Swift（跟随 Xcode 最新稳定版） | Apple 原生一等支持，生态与工具链最稳定 | 无需引入跨平台语言 |
| UI 框架 | SwiftUI | 声明式 UI，天然契合 SARE 单向数据流 | UIKit 可行但样板代码更多，与模式一致性较弱 |
| 依赖管理 | SPM | Apple 官方方案，轻量、与 Xcode 集成最佳 | CocoaPods/Carthage 增加额外维护负担 |
| gRPC 客户端 | grpc-swift（Apple 官方维护） | 官方维护、SPM 支持好、社区活跃 | 其他非主流实现在维护活跃度与集成体验上不占优 |
| 本地存储 | SQLite（GRDB 封装） | 与 Android/HarmonyOS 可对齐，成熟稳定，便于统一存储抽象 | SwiftData/Core Data/Realm 见第 3 节排除说明 |
| 并发模型 | Swift Concurrency（async/await + actor） | 原生并发模型，语义清晰，避免额外响应式框架成本 | 不引入 Combine/RxSwift |

---

## 2. 三端技术栈对照（规划，非 iOS 侧为 [待确认]）

> 说明：MVP 仅落地 iOS；Android/HarmonyOS 当前仅保留方向性草案，最终方案状态均为 **[待确认]**。

| 层面 | iOS（MVP 已确认） | Android（[待确认]） | HarmonyOS（[待确认]） |
|---|---|---|---|
| 语言 | Swift | Kotlin（规划）[待确认] | ArkTS（规划）[待确认] |
| UI 框架 | SwiftUI | Jetpack Compose（规划）[待确认] | ArkUI（规划）[待确认] |
| 依赖管理 | SPM | Gradle（规划）[待确认] | ohpm（规划）[待确认] |
| gRPC | grpc-swift | grpc-kotlin（规划）[待确认] | [待确认] |
| 本地存储 | GRDB (SQLite) | Room (SQLite)（规划）[待确认] | 原生 SQLite（规划）[待确认] |
| 并发 | async/await + actor | Coroutines + Flow（规划）[待确认] | [待确认] |
| 页面模式 | SARE（自实现） | MVI（语义等价 SARE）（规划）[待确认] | SARE（自实现）（规划）[待确认] |

---

## 3. 本地存储选型决策记录

核心决策因素：**三端对齐优先于单端便利性**。

- **SwiftData（排除）**：iOS 侧开发效率高，但平台独占，无法三端对齐。
- **Core Data（排除）**：成熟但历史负担重、学习曲线陡，且 iOS 独占。
- **Realm（排除）**：跨平台能力存在，但 HarmonyOS 支持不足，长期演进确定性偏弱。
- **SQLite + GRDB（选定）**：三端均可落在 SQLite，接口统一、稳定性高、演进风险可控。

结论：服务层定义统一存储协议，GRDB 仅作为 iOS 实现细节。

---

## 4. 关键技术约束（对后续设计生效）

1. **不引入响应式框架**：iOS 不使用 Combine / RxSwift，统一 Swift Concurrency。  
2. **三方库最小化**：仅引入必要库（当前为 grpc-swift、GRDB），其余能力优先自建。  
3. **存储接口统一**：业务/服务层仅依赖存储协议，不依赖具体数据库实现。  
4. **遵循分层与单向依赖**：选型需满足四层架构与“依赖只能向下”（与 `principles.md` 一致）。  
5. **SARE 一致性优先**：技术栈选择服务于 State/Action/Reducer/Effect 模式，不破坏单向数据流。

---

## 5. 与架构原则对齐说明（简要）

- 本文仅定义技术基线与约束，不展开服务层/基础层具体设计。  
- 选型遵循“规范共享、实现分离”：iOS 先落地，Android/HarmonyOS 保留 [待确认]，后续在同一语义契约下补齐。  
- 所有后续模块设计（M03-M06）需在本基线之上推进。

---

完成时间：2026-02-20 08:36 (GMT+8)
