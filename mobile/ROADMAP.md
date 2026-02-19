# agent-demo/mobile — ROADMAP

> 三端（iOS/Android/HarmonyOS）统一架构规范，MVP 先做 iOS。
> 架构原则和四层分层已定（见 STATE.md），现在进入详细设计阶段。

---

## Phase 0：架构设计

| # | 任务 | 状态 | 位置 | 内容 |
|---|------|------|------|------|
| M01 | 架构原则 + 分层规范 | ✅ | `docs/design/principles.md` | 四层分层详细定义、架构原则、AI 时代强化、页面级 SARE 模式 |
| M02 | 技术选型 + 约束 | 🔲 | `docs/design/tech-stack.md` | Swift/iOS 最低版本、SPM、gRPC 客户端库、本地存储方案 |
| M03 | 基础层设计 | 🔲 | `docs/design/foundation.md` | 通用组件清单、三方库封装策略、跨平台共享模块 |
| M04 | 服务层设计 | 🔲 | `docs/design/services.md` | 网络服务、本地存储服务、认证服务 |
| M05 | 业务模块划分 | 🔲 | `docs/design/business-modules.md` | MVP 业务模块清单、职责边界、模块间依赖 |
| M06 | 应用集成层设计 | 🔲 | `docs/design/app-integration.md` | 启动流程、服务注册、导航/路由 |
| M07 | Proto 定义 | 🔲 | `../proto/` | .proto 文件，与 server 端 grpc-services.md 对齐 |

### 依赖关系

```
M01 → M02（原则确定后才选技术栈）
M02 → M03（技术栈确定后设计基础层）
M03 → M04（基础层确定后设计服务层）
M04 → M05（服务层接口确定后划分业务模块）
M05 → M06（业务模块确定后设计集成层）
M04 ←→ M07（服务层网络设计和 Proto 定义互相影响，可并行）
```

---

## Phase 1：Walking Skeleton（待 Phase 0 完成后开始）

最小可运行 iOS App：登录 → 进入聊天 → 发消息 → 收到回复

---

*最后更新：2026-02-20 05:15*
