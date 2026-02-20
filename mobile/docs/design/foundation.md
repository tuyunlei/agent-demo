# M03 基础层设计（Foundation Layer）

> 适用范围：iOS MVP（优先），并为 Android / HarmonyOS 预留三端对齐空间。  
> 约束基线：遵循 `principles.md` 四层模型与向下依赖；技术栈遵循 `tech-stack.md`（Swift + Swift Concurrency + grpc-swift + GRDB，三方库最小化）。

---

## 1. 设计目标与边界

基础层只提供**通用能力**与**技术封装**，不引入业务语义，不关心“会话/消息/登录”等业务概念。

- ✅ 可以做：协议客户端封装、数据库连接封装、Keychain 封装、日志抽象、通用工具
- ❌ 不做：认证流程、消息收发流程、会话列表聚合、重连策略编排（这些属于服务层/业务层）

---

## 2. 模块清单与职责

建议基础层拆分为 8 个模块（可按 package/target 组织）：

### 2.1 `FoundationCore`
**职责**：跨模块共享的最小基础类型与协议。
- 通用错误模型（非业务错误）
- 时间/时钟抽象、ID 生成抽象、JSON 编解码抽象
- 与平台无关的轻量工具类型

**边界**：不包含任何网络/存储/业务字段语义。

---

### 2.2 `ConcurrencyCore`
**职责**：并发与任务管理基础能力（基于 Swift Concurrency）。
- 任务取消包装、超时包装、重试策略原语（仅策略，不绑定业务场景）
- actor 隔离辅助工具

**边界**：不包含“消息订阅重连”“token 刷新重试”等业务策略。

---

### 2.3 `NetworkCore`
**职责**：网络传输层封装（HTTP/gRPC transport 级别）。
- gRPC Channel 生命周期管理
- Interceptor 链（鉴权 header 注入能力接口、日志打点钩子）
- 流式事件基础适配（stream -> async sequence）
- 统一 transport 错误映射（连接超时、不可达、证书问题等）

**边界**：不定义 `AuthService`/`MessageService` 等业务服务接口。

---

### 2.4 `StorageCore`
**职责**：本地存储基础能力封装（GRDB/SQLite）。
- 数据库连接与队列管理
- migration 执行器
- 事务执行器、基础 CRUD executor 抽象
- 通用序列化字段适配（如 JSON 列）

**边界**：不出现“消息表/会话表业务仓储”语义；仅提供存储原语。

---

### 2.5 `SecureStoreCore`
**职责**：安全存储封装。
- Keychain 读写/删除/更新
- 命名空间与访问级别策略封装
- 敏感值序列化与错误映射

**边界**：不定义 token 生命周期规则，仅提供“安全存取”能力。

---

### 2.6 `LogCore`
**职责**：统一日志协议与实现适配。
- Logger 协议（trace/debug/info/warn/error）
- 日志上下文（requestId、module、thread）
- 平台日志后端适配（iOS: OSLog）

**边界**：不包含业务埋点事件定义。

---

### 2.7 `PlatformCore`
**职责**：平台环境能力抽象（供上层按需注入）。
- 网络可达性抽象
- App 生命周期事件抽象
- 设备信息/系统版本等环境信息读取

**边界**：仅抽象系统能力，不含业务决策。

---

### 2.8 `UIFoundation`（可选）
**职责**：跨业务复用的 UI 原子组件与设计 Token 封装。
- 基础按钮/输入框/加载态/空态组件
- 颜色、字体、间距 token

**边界**：不包含业务页面组件（聊天气泡、会话卡片等）。  
**状态**：是否在 M03 纳入基础层，当前为 **[待确认]**（取决于 M05 是否需要独立 UI 组件层）。

---

## 3. 三方库封装策略

## 3.1 需要封装的库

1. **grpc-swift**（必须封装）  
   - 通过 `NetworkCore` 屏蔽 channel/interceptor/stream 细节
2. **GRDB**（必须封装）  
   - 通过 `StorageCore` 屏蔽数据库队列、事务与迁移细节
3. **OSLog / Keychain / NWPathMonitor**（系统框架，也做统一抽象）  
   - 避免上层直接依赖平台 API，保留可替换空间

## 3.2 封装原则

- **最小暴露面**：上层只见协议，不见三方具体类型
- **单向依赖**：三方类型只停留在基础层内部
- **错误统一**：输出基础层统一错误语义，避免把库错误透传到业务层
- **可替换性**：替换实现（如 GRDB -> 其他 SQLite wrapper）时，上层接口不变
- **测试友好**：每个能力都可通过 fake/mock 实现替换

---

## 4. 跨平台共享策略（iOS MVP + 三端规划）

## 4.1 三端可共享（优先语义共享）

- 模块边界与接口语义（`NetworkCore` / `StorageCore` / `SecureStoreCore` / `LogCore`）
- 错误分类与恢复语义（超时、不可达、鉴权失效、序列化失败）
- 并发策略语义（超时/取消/重试的行为约束）
- 设计 token 语义（若启用 `UIFoundation`）

## 4.2 平台特定实现

- gRPC 客户端具体实现（grpc-swift / grpc-kotlin / [待确认]）
- SQLite 封装实现（GRDB / Room / [待确认]）
- SecureStore（Keychain / Keystore / [待确认]）
- 日志后端（OSLog / Logcat / [待确认]）
- 生命周期与可达性 API

> 结论：基础层追求“**接口与语义共享**”，不强求三端物理代码完全复用。

---

## 5. 模块对外接口草案（Protocol/Interface 级）

> 仅定义能力边界，不下沉到方法签名。

- `Clock`, `IDGenerator`, `Codec`（`FoundationCore`）
- `TaskScheduler`, `RetryPolicy`, `TimeoutController`（`ConcurrencyCore`）
- `TransportClient`, `StreamingTransport`, `Interceptor`, `ConnectivityProbe`（`NetworkCore`）
- `DatabaseEngine`, `MigrationRunner`, `TransactionRunner`, `KVStore`（`StorageCore`）
- `SecureStore`（`SecureStoreCore`）
- `Logger`, `LogSink`（`LogCore`）
- `AppLifecycleObserver`, `DeviceInfoProvider`（`PlatformCore`）
- `DesignTokenProvider`, `AtomicComponentFactory`（`UIFoundation`，[待确认]）

---

## 6. 与服务层（M04）的边界说明

## 6.1 属于基础层

- “能力本身”的技术封装：
  - gRPC channel/stream/拦截器
  - SQLite 连接/事务/迁移
  - Keychain 安全读写
  - 统一日志协议

## 6.2 属于服务层（不在本文展开）

- “面向业务可用”的服务编排：
  - `AuthService`（调用 transport + secure store 组合 token 策略）
  - `MessageSyncService`（调用 transport + storage 做消息同步）
  - `SessionRepository`（会话聚合查询与缓存策略）

## 6.3 判定规则

- 若接口名称已带业务词汇（Auth/Message/Session），通常应归服务层
- 若能力可在任意业务复用且无业务语义，归基础层

---

## 7. 当前待确认项

1. `UIFoundation` 是否在 M03 纳入基础层，还是延后到 M05 再定（**[待确认]**）
2. HarmonyOS 侧 gRPC 具体方案（**[待确认]**）
3. HarmonyOS 侧安全存储与日志后端的最终技术映射（**[待确认]**）

---

完成时间：2026-02-20 09:16 (GMT+8)
