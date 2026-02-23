# CLAUDE.md — iOS 客户端开发指南

## 项目概述

AI 陪伴 Agent 平台的 iOS 客户端。Swift + SwiftUI，gRPC 通信。

- **代码**：当前目录（`mobile/`）
- **服务端**：`../server/`（Rust，部署地址见 `deploy/.env`）
- **Proto**：`../proto/`（SPM Build Plugin 自动生成 Swift 代码）
- **设计文档**：`docs/design/`
- **CI**：GitHub Actions macOS runner

## 工作流

1. 读 `tasks/` 目录，选文件名数字最小的任务目录
2. 读该目录下的 `brief.md`（任务目标 + 验收标准）
3. 如果有 `feedback.md`，先看——那是上次 CI/review 的问题
4. 创建 `feature/*` 分支
5. 编码 + 本地测试
6. commit + push + `gh pr create --base develop`
7. **不要修改 `tasks/` 下的任何文件**——tasks 由 PM 管理

## 分支规则

- 在 `feature/*` 分支开发，禁止直接提交 develop
- PR 目标分支：develop
- 一个任务一个分支一个 PR

## 质量要求

- **SwiftLint**：`--strict` 模式，警告视为错误
- **SwiftFormat**：`--lint` 模式
- **测试**：新功能必须有对应的 Swift Testing 单元测试
- **文件大小**：单文件 ≤300 行（业务代码），单函数 ≤50 行
- **CI 必须绿**：PR 不过 CI 不会被 merge

## 架构约束

- 分层架构：详见 `docs/design/principles.md`
- ViewModel 通过 Protocol 注入依赖，方便测试
- Service 层抽象为 Protocol（如 ChatServiceProtocol、SessionServiceProtocol）
- 现有模式参考：`ChatViewModel` + `ChatServiceProtocol` 的做法

## 技术栈

- **UI**：SwiftUI
- **gRPC**：grpc-swift v2（grpc-swift-protobuf + grpc-swift-nio-transport）
- **Proto 生成**：SPM Build Plugin（proto 变更零成本同步）
- **测试**：Swift Testing 框架
- **最低版本**：iOS 26.2

## 注意事项

- ⚠️ **仓库是 public 的** — 禁止写入 IP 地址、密码、API key、内部域名等敏感信息。凭证走环境变量，地址走配置文件（gitignored）
- `tasks/` 只读——不创建、不修改、不删除其中的文件
- 有问题或不确定的地方，写在 PR description 里，PM 会看到
- 服务端 API 文档：`../server/docs/design/` + `../proto/`
