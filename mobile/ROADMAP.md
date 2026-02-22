# agent-demo/mobile — ROADMAP

> Phase 0 架构设计已完成。进入 Phase 1 Walking Skeleton 实现。
> 详细设计文档见 `docs/design/`
>
> **MVP 产品范围**：Login + Chat 两个页面，AI 回复整块返回（unary，不做流式）

---

## 技术选型

- **gRPC 库**：grpc-swift v2（grpc-swift-protobuf + grpc-swift-nio-transport）— Swift Concurrency 原生
- **Server 地址**：`REDACTED_HOST:8443`（gRPC over TLS，Caddy 反代）

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

### Step 3：iOS 项目起步（已完成）

| # | Task | 状态 | 内容 |
|---|------|------|------|
| TM3.1a | Xcode 项目初始化 | ✅ | SwiftUI，iOS 26.2 |
| TM3.1b | grpc-swift v2 配置 | ✅ | SPM 依赖 + protoc .pb.swift + TLS channel + APIClient |
| TM3.2 | GitHub Actions iOS CI | ✅ | macOS runner，Xcode 26.2，xcbeautify + raw log artifact |
| TM3.3 | Login 页面 | ✅ | 登录 UI + AuthService.Login 调用 |
| TM3.4 | Chat 页面 | ✅ | 聊天 UI + SendMessage 调用 |

---

## Quality Gate：质量保障体系

> 所有检查在 CI（macOS runner）中执行。VPS 无 Xcode，仅文件大小检查可本地跑。

### 代码质量（自动门禁）

| # | Task | 状态 | 内容 |
|---|------|------|------|
| MQG1 | SwiftLint + SwiftFormat | ✅ | SwiftLint --strict + SwiftFormat --lint，CI 门禁（PR #7） |
| MQG2 | 文件大小 & 复杂度 | ✅ | 单文件 ≤300 行，单函数 ≤50 行；脚本 + CI 门禁（PR #8） |

### 基础设施

| # | Task | 状态 | 内容 |
|---|------|------|------|
| MQG-infra | SPM Build Plugin 替代手动 protoc | ✅ | PR #9，grpc-swift v2 SPM plugin，proto 变更零成本同步 |

### 功能质量（测试保障）

| # | Task | 状态 | 内容 |
|---|------|------|------|
| MQG3 | ViewModel 重构 + 单元测试 | 🔲 | 业务逻辑从 View 抽到 ViewModel，ChatServiceClient 协议化可 mock，验证 AI 回复内容正确显示 |

### 质量铁律

- CI 红 = 不能 merge
- 新功能 PR 必须包含对应测试
- SwiftLint 警告视为错误（--strict）

---

## 修复与补完

> 质量门禁完成后，修复现有功能问题。

| # | Task | 状态 | 内容 |
|---|------|------|------|
| FIX-1 | Chat 页面显示 AI 回复 | ✅ | PR #10，解析 assistantContent 中的 TextBlock |

---

## Phase 1（续）：功能开发

> 质量门禁 + 修复完成后恢复功能开发。
> **Mobile 做完以下内容后暂停，等涂涂验收，之后与服务端同步推进。**

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

## 未来建设（待讨论）

| # | Task | 状态 | 内容 |
|---|------|------|------|
| OBS-1 | 日志与观测体系 | 🔲 | 客户端日志收集、错误上报、性能监控。待涂涂讨论后细化 |

---

*最后更新：2026-02-22*

