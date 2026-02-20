# agent-demo/mobile — ROADMAP

> Phase 0 架构设计已完成。进入 Phase 1 Walking Skeleton 实现。
> 详细设计文档见 `docs/design/`

---

## 技术选型

- **gRPC 库**：grpc-swift v2（grpc-swift-protobuf + grpc-swift-nio-transport）— Swift Concurrency 原生
- **Server 地址**：`REDACTED_HOST:443`（gRPC over TLS，Caddy 反代）

---

## Phase 0：架构设计（已完成）

<details>
<summary>展开查看</summary>

| # | 任务 | 状态 | 位置 |
|---|------|------|------|
| M01 | 架构原则 + 分层规范 | ✅ | `docs/design/principles.md` |
| M02 | 技术选型 + 约束 | ✅ | `docs/design/tech-stack.md` |
| M03 | 基础层设计 | ✅ | `docs/design/foundation.md` |
| M04 | 服务层设计 | ✅ | `docs/design/services.md` |
| M05 | 业务模块划分 | ✅ | `docs/design/business-modules.md` |
| M06 | 应用集成层设计 | ✅ | `docs/design/app-integration.md` |
| M07 | Proto 定义 | ✅ | `../proto/` |

</details>

---

## Phase 1：Walking Skeleton

### Step 3：iOS 项目起步

| # | Task | 状态 | 内容 |
|---|------|------|------|
| TM3.1a | Xcode 项目初始化 | ✅ | 涂涂创建，SwiftUI，iOS 26.2 |
| TM3.1b | grpc-swift v2 配置 | 🔲 | SPM 依赖 + protoc 编译 .proto → Swift + TLS channel 配置 |
| TM3.2 | GitHub Actions iOS CI | ✅ | macOS runner，Xcode 26.2，xcbeautify + raw log artifact |
| TM3.3 | Login 页面 | 🔲 | 登录 UI + AuthService.Login 调用 + token 存储（Keychain） |
| TM3.4 | Chat 页面 | 🔲 | 聊天 UI + SendMessage 调用 + 显示 AI 回复（整块返回） |

### Step 4：持久化

| # | Task | 状态 | 内容 |
|---|------|------|------|
| TM4.1 | GRDB + SQLite | 🔲 | 本地消息缓存，离线查看历史 |

### Step 5：韧性

| # | Task | 状态 | 内容 |
|---|------|------|------|
| TM5.1 | 网络错误处理 | 🔲 | 连接失败/超时的 UI 反馈 |
| TM5.2 | Token 自动刷新 | 🔲 | RefreshToken 逻辑，过期自动续期 |

### Step 6：会话管理

| # | Task | 状态 | 内容 |
|---|------|------|------|
| TM6.1 | 会话列表页 | 🔲 | （MVP 后续，暂不实现） |

---

*最后更新：2026-02-21*
