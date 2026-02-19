# Brief: D02-Rev — agent-grpc → agent-server 重命名 + 定位修正

## 任务目标

把设计文档里的 `agent-grpc` 全部改为 `agent-server`，并修正其职责描述。

## 核心认知（必须理解）

**之前的错误**：`agent-grpc` 被定位为"gRPC 服务入口"，让 gRPC 协议的名字占据了 binary
入口的位置，暗示 gRPC 是架构核心。

**正确的理解**：
- gRPC 是 `agent-channel` 里的一个 ChannelAdapter 实现，和 WebSocket/Push 地位完全
  相同，是架构里非常小的一部分，不重要
- binary 入口 + 组合根（Composition Root）是独立的概念，应该叫 `agent-server`
- `agent-server` 知道所有 Port 实现，在 main() 里做装配，然后启动服务
- `agent-server` 不关心底层用什么协议，那是 `agent-channel` 的事

## 需要修改的文件

### 1. `docs/design/crate-structure.md`（最重要）
- 把所有 `agent-grpc` 出现处改为 `agent-server`
- 更新该 crate 的职责描述：
  - 旧：tonic gRPC 服务入口 + 组合根
  - 新：**binary 入口 + 组合根（Composition Root）**。启动服务，装配所有 Port 实现。
    不关心底层协议——gRPC 只是 agent-channel 里的一个 ChannelAdapter 实现，
    agent-server 不以 gRPC 命名，因为换协议不影响这里的组合逻辑。
- 更新 crate DAG 图中的节点名称

### 2. `docs/design/error-types.md`
- 把 `agent-grpc` 出现处改为 `agent-server`
- 保持其他内容不变

### 3. `docs/design/ports/core-ports.md`
- 把 `agent-grpc` 出现处改为 `agent-server`
- 保持其他内容不变

### 4. `docs/design/channel-system/port.md`
- 把 `agent-grpc` 出现处改为 `agent-server`
- 如果有提到 gRPC 是"入口"或"核心"，修正措辞：gRPC 是 ChannelAdapter 的一个实现

### 5. `docs/design/principles.md`
- 检查是否有 `agent-grpc`，有则改为 `agent-server`

## 成功标准

- 所有设计文档里不再出现 `agent-grpc`（全部改为 `agent-server`）
- `agent-server` 的描述清楚表达：binary 入口 + 组合根，协议无关
- `agent-channel` 的描述清楚表达：包含 gRPC ChannelAdapter 实现（和 WS/Push 并列）
- 完成后汇报：改了哪几个文件，共几处替换

## 约束

- 只改文档，不改 spikes/ 目录下的 Rust 代码
- 不改 briefs/ 目录下的文件（历史记录）
- 保持文档其他内容不变
