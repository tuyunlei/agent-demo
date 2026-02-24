# Channel 系统设计总览（多渠道通信层）

## 1. 目标与范围

Channel 系统是 AI Agent 平台中连接「用户交互入口」与「内部 Agent 能力」的通信中台。其核心目标：

- **统一接入**：屏蔽各渠道协议差异（gRPC / WebSocket / HTTP Webhook / IM API）
- **统一消息语义**：将外部消息归一为内部 Inbound/Outbound 模型
- **统一分发与同步**：支持单用户多设备、多渠道一致会话体验
- **统一可运营能力**：支持主动消息、推送触达、限流与安全控制

> 非目标：不重复设计 Agent Runtime 内部流程；不细化 gRPC 字段级协议；不包含代码实现。

---

## 2. 六边形架构中的定位与职责边界

## 2.1 架构位置

在六边形架构中：

- **Domain 层（Port）**：定义 `ChannelAdapter` 抽象接口（能力声明、入站标准化、出站渲染）
- **Adapter 层（实现）**：各渠道具体实现（NativeAdapter、WebAdapter、TelegramAdapter、FeishuAdapter 等）
- **Application 层（编排）**：
  - `ChannelIngressService`：接收入站事件并调用适配器标准化
  - `MessageRoutingService`：路由策略、终端选择、同步分发
  - `ChannelEgressService`：将 Runtime 出站事件交由适配器下发
  - `PushOrchestrator`：推送策略与 APNs/FCM 对接

## 2.2 职责边界

Channel 系统负责：

1. **格式转换**：外部消息结构 ↔ 内部统一模型
2. **协议适配**：流式/非流式、长连接/回调、消息确认机制等
3. **多端同步**：同一用户多设备状态一致与可控分发
4. **渠道能力降级**：不同平台能力不对等时做显式降级策略
5. **安全与限流**：Webhook 验签、频率限制、渠道策略

Channel 系统不负责：

- Agent 推理执行与工具链内部过程
- 会话业务语义（如任务编排、记忆算法）
- 第三方推送供应商底层 SDK 细节（由 Push 基础设施层负责）

## 2.3 与 Runtime、推送系统关系

- **与 Agent Runtime**：通过内部统一事件（如已定义 ChatEvent 语义）解耦；Runtime 只输出标准事件，不关心渠道类型。
- **与推送系统**：Channel 负责“推送内容的渠道语义适配”，Push 基础设施负责“设备令牌与供应商投递”。

---

## 3. Channel 分类与能力差异

## 3.1 Native Channel（iOS/Android/Harmony）

**协议形态**：gRPC 双向流（MVP 首选 iOS）

**特点**：
- 低延迟、强交互、可携带设备态
- 完整支持流式输出（TextDelta / Tool 状态）
- 适合多模态（语音、图片上传）与强认证（App Token + Device）

**能力级别**：最高（可完整映射内部事件）

## 3.2 Web Channel（WebSocket / SSE）

**协议形态**：
- WebSocket：双向实时（推荐）
- SSE：服务端单向流 + 客户端 HTTP 上行

**特点**：
- 覆盖广，部署门槛低
- 浏览器环境受限（后台保活、推送权限、媒体能力）
- 适合文本与轻交互组件

**能力级别**：高（略低于 Native）

## 3.3 IM Channel（微信/Telegram/飞书等）

**协议形态**：Webhook 入站 + 平台 API 出站

**特点**：
- 用户触达成本低、获客友好
- 平台限制多：速率、格式、模板、审核、回调重试
- 实时性与展示一致性受平台约束

**能力级别**：中（强依赖平台能力）

## 3.4 API Channel（开放 API）

**协议形态**：REST / Webhook Callback / Streaming API

**特点**：
- 面向开发者二次集成
- 需强调稳定 SLA、签名机制、幂等与版本管理
- 不直接承诺具体 UI，仅承诺协议语义

**能力级别**：可配置（取决于调用方式）

## 3.5 能力差异矩阵（概念级）

| 能力 | Native | Web | IM | API |
|---|---|---|---|---|
| 双向实时流 | 强 | 中-强 | 弱（多为伪实时） | 可选 |
| 富交互组件 | 强 | 中 | 弱-中 | 协议化 |
| 多模态上传 | 强 | 中 | 平台相关 | 可选 |
| 推送唤醒 | 强（APNs/FCM） | 中（Web Push） | 平台内通知 | 由调用方实现 |
| 能力可控性 | 高 | 高 | 低（受平台约束） | 高 |

---

## 4. 统一消息模型（Canonical Message Model）

设计原则：**显式字段、可追踪来源、可降级渲染、可审计扩展**。

## 4.1 Inbound（入站）模型

核心字段建议：

- `message_id`：外部消息唯一 ID（用于幂等）
- `channel_type`：native/web/im/api
- `channel_provider`：ios_app/web/telegram/feishu/wechat...
- `user_id`：平台内部用户标识（统一账号）
- `channel_user_id`：渠道侧用户标识
- `conversation_id`：统一会话 ID（内部）
- `channel_conversation_ref`：渠道会话引用（chat_id/thread_id 等）
- `sent_at` / `received_at`
- `content[]`：标准内容片段（文本/图片/语音/文件/富文本/交互输入）
- `context`：语言、时区、设备、地理粗粒度、客户端版本
- `security`：签名校验结果、风险评分、来源 IP/UA 摘要
- `trace`：request_id、correlation_id

## 4.2 Outbound（出站）模型

核心字段建议：

- `event_id`：内部事件 ID
- `user_id` / `conversation_id`
- `target_channels[]`：目标渠道列表（可单播/多播）
- `delivery_mode`：realtime / async / push_fallback
- `payload[]`：标准输出片段（支持增量与完整）
- `render_hints`：降级策略、长度截断策略、格式偏好
- `priority`：normal/high/urgent
- `requires_ack`：是否需要投递确认
- `retry_policy`：重试次数、退避策略
- `trace`

## 4.3 内容类型（统一枚举）

- `text`：纯文本/Markdown 子集
- `image`：图片 URL 或媒体引用
- `audio`：语音消息（含时长、编码）
- `file`：通用文件（mime、大小、下载凭据）
- `rich_text`：结构化富文本（段落、链接、代码块）
- `interactive`：按钮、选项、表单输入、快捷操作
- `system`：状态型消息（typing、error、done、notice）

## 4.4 元数据规范

必须包含：

1. **来源元数据**：渠道、平台、版本
2. **设备元数据**：device_id、os、app_version（可选匿名化）
3. **用户上下文**：locale、timezone、access_scope
4. **审计元数据**：trace_id、idempotency_key、risk_tags

---

## 5. ChannelAdapter 接口设计（Domain Port）

> 仅定义职责与方法语义，不给 Rust 代码。

## 5.1 核心接口职责

`ChannelAdapter` 应提供四类能力：

1. **身份与能力声明**
2. **入站标准化（decode/normalize）**
3. **出站渲染与发送（render/send）**
4. **投递反馈处理（ack/error/retry）**

## 5.2 方法语义（概念）

- `adapter_id()`：返回适配器标识
- `channel_kind()`：native/web/im/api
- `capabilities()`：声明支持的内容类型、长度限制、是否支持流式、是否支持交互组件
- `validate_inbound(raw_request)`：验签、结构校验、基础反滥用检查
- `to_inbound(raw_request)`：外部协议 → 统一 Inbound
- `to_outbound(channel_context, canonical_outbound)`：统一 Outbound → 渠道可发送消息序列
- `send(rendered_messages)`：执行发送并返回投递回执
- `parse_delivery_callback(callback)`：平台回执/失败回调标准化
- `health_check()`：适配器健康状态（认证、配额、网络）

## 5.3 能力协商机制

在路由前执行 `capability negotiation`：

- 输入：目标渠道能力 + Outbound 需求
- 输出：
  - **原样发送**（支持）
  - **降级发送**（如 rich_text → text，interactive → 列表编号）
  - **拒绝并回退**（如语音必须展示但渠道不支持）

要求：
- 降级规则可配置且显式可审计
- 记录 `degrade_reason` 供观测与产品决策

---

## 6. 多端同步与消息路由

## 6.1 同步模型

以 `user_id + conversation_id` 为主键维护“会话主时间线”，各渠道为“投影视图”。

- 主时间线保存完整语义事件
- 各渠道保存投递映射（外部 message_id、状态、时间）
- 支持消息状态同步：sent/delivered/read/failed

## 6.2 路由策略

支持可配置策略（按用户偏好 + 系统默认）：

1. **活跃端优先（MVP 默认）**
   - 若用户当前在 Native/Web 在线会话，优先实时下发该端
   - 其他端可选“静默同步”

2. **全端广播**
   - 面向通知型/提醒型消息，同步到所有绑定渠道

3. **主端 + 备份端**
   - 首投主端，超时未确认再投备份端（如 IM 或 Push）

4. **渠道白名单路由**
   - 某类消息仅允许指定渠道（安全/合规场景）

## 6.3 离线处理

- 维护终端在线状态与最后活跃时间
- 离线时写入待投递队列（带 TTL）
- 恢复在线后按时间线补发（可配置是否合并摘要）
- 超过 TTL 转换为推送摘要或未读计数，不无限重放

---

## 7. IM 平台接入通用模式

## 7.1 通用收发链路

**入站（Webhook）**
1. 平台回调到 `IM Ingress Endpoint`
2. 验签与幂等去重
3. 适配器标准化为 Inbound
4. 投递到内部消息总线/应用服务

**出站（主动推送）**
1. Runtime 产出 Outbound
2. 路由选择目标 IM 渠道
3. 适配器渲染与拆分（长度/格式）
4. 调用平台发送 API
5. 记录回执与失败重试

## 7.2 常见限制（设计必须显式处理）

- 消息长度限制（需自动分片）
- Markdown/富文本子集差异（需格式降级）
- 媒体上传流程差异（先上传后引用）
- 速率限制（QPS、分钟配额）
- Webhook 重试与乱序（需幂等键 + 顺序控制）
- 会话窗口限制（部分平台 24h 规则）

## 7.3 适配器注册与配置

采用“声明式注册”：

- `adapter_name`
- `provider_type`
- `credentials_ref`（密钥引用，不明文）
- `webhook_path` / `allowed_ips`
- `rate_limit_profile`
- `capability_profile`
- `enabled` / `gray_release_ratio`

支持热更新配置（不重启核心服务）与灰度发布。

---

## 8. 推送与主动消息设计

## 8.1 主动触达触发来源

- Agent 定时任务/事件触发
- 用户订阅型提醒（如日程、待办、告警）
- 会话外补充信息（异步结果、任务完成）

## 8.2 触达决策流程

1. 判断用户当前在线渠道
2. 若在线：优先实时会话内发送
3. 若离线：调用 PushOrchestrator 选择 APNs/FCM/Web Push/IM 通知
4. 若推送不可达：回退到次优渠道（如短信不在本设计范围，可预留）

## 8.3 APNs/FCM 对接边界

- Push 基础设施层管理 device token 生命周期
- Channel 层提供：
  - 推送摘要内容（title/body）
  - 深链参数（conversation_id/message_ref）
  - 优先级与静默推送标记

## 8.4 推送内容渠道适配

同一 Outbound 生成不同推送模板：

- Native：简短摘要 + 深链
- Web Push：标题 + 摘要 + 打开 URL
- IM：平台可见文本 + 操作链接（如支持）

原则：推送只放必要信息，敏感内容默认折叠或脱敏。

---

## 9. 安全与限流

## 9.1 Webhook 安全

- HMAC/RSA 签名校验
- 时间戳窗口防重放
- nonce 幂等去重
- IP allowlist（平台支持时）
- 非法来源快速拒绝与告警

## 9.2 频率限制（Rate Limiting）

分层限流：

1. **用户级**：每用户每分钟消息数
2. **渠道级**：每渠道 API 调用速率
3. **适配器级**：第三方平台配额保护
4. **全局级**：系统熔断阈值

策略：令牌桶 + 指数退避 + 可观测告警。

## 9.3 渠道级安全策略

- 渠道鉴权（OAuth token/API key/session）
- 设备绑定与风控标签
- 敏感操作二次确认（按渠道能力降级）
- 审计日志全链路可追踪（request_id/correlation_id）

---

## 10. MVP 方案与演进路线

## 10.1 MVP（阶段 1）

聚焦 Native iOS：

- 完成 Native gRPC ChannelAdapter
- 打通 Inbound/Outbound 统一模型
- 实现基础路由（活跃端优先）
- 实现 APNs 推送回退
- 建立基础限流与审计

## 10.2 阶段 2（Web 扩展）

- 增加 WebSocket/SSE Adapter
- 引入多端同步状态（read/delivered）
- 完善能力协商与消息降级规则

## 10.3 阶段 3（IM 与开放生态）

- 接入 1~2 个 IM（建议 Telegram/飞书先行）
- 建立统一 Webhook 网关与适配器注册中心
- 开放 API Channel（含签名、版本化、幂等）

## 10.4 阶段 4（运营与智能化）

- 智能路由（基于送达率/响应率优化）
- 渠道健康评分与自动故障切换
- 主动消息策略中心（用户偏好 + 免打扰）

---

## 11. 可观测性与运维要点（建议）

关键指标：

- 入站成功率、出站成功率、端到端时延
- 渠道分布与失败原因 TopN
- 降级发送比例（按消息类型/渠道）
- 推送到达率、点击率、回流会话率
- 限流触发率与误伤率

关键日志字段：

- `trace_id`, `user_id`, `channel`, `adapter`, `message_type`, `delivery_status`, `degrade_reason`

---

## 12. 验收对照清单

- [x] Channel 在架构中的位置和职责清晰
- [x] 4 种 Channel 类型均有描述
- [x] 统一消息模型覆盖 Inbound/Outbound
- [x] ChannelAdapter trait 接口语义完整
- [x] 多端同步和消息路由方案明确
- [x] IM 接入通用模式与限制明确
- [x] 推送/主动消息机制完整
- [x] MVP 与后续演进路径清晰

---

## 13. 结论

该设计以六边形架构为基础，将 Channel 作为独立通信层能力，确保：

1. **MVP 可快速落地**（Native 优先）
2. **后续渠道可平滑扩展**（Web/IM/API）
3. **用户体验一致且可控**（统一模型 + 能力协商 + 路由同步）
4. **工程可运维可治理**（安全、限流、审计、观测齐备）

满足当前产品目标，并为多渠道增长与主动触达能力预留明确扩展路径。