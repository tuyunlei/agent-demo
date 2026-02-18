# 认证授权系统设计总览（MVP → 演进）

## 0. 文档目标与范围

本文定义多用户托管式 AI Agent SaaS 平台的认证（Authentication）、授权（Authorization）与会话（Session）设计。

- 技术背景：Rust + 六边形架构（Hexagonal Architecture）
- 当前阶段：MVP
- 已知前提：邀请码注册、JWT、多租户以 `user_id` 为隔离主维度

> 本文不包含：gRPC 字段细节、密码学底层实现细节、Rust 代码实现。

---

## 1. 认证授权总览

## 1.1 在六边形架构中的位置

### Domain（核心业务层）
- 定义认证/授权相关核心抽象（Trait / Port）：
  - `AuthService`：注册、登录、刷新、登出等用例语义
  - `AuthorizationService`：角色判定、资源访问判定、工具能力判定
  - `SessionService`：会话生命周期管理（创建、续期、撤销、列举）
- 定义领域对象与策略：
  - 用户身份（Identity）
  - 角色（Role）与能力（Capability）
  - 资源所有权（Resource Ownership）

### Application（用例编排层）
- 编排 Register / Login / Refresh / Logout 等流程
- 在调用业务用例前完成“身份解析 + 权限校验”
- 明确失败语义（未认证、无权限、会话失效、风控拦截）

### Infrastructure（适配器与实现层）
- JWT 签发与验证实现
- Refresh Token 持久化、撤销列表、会话存储
- 密码哈希与校验实现
- 外部身份提供商（OAuth/OIDC）适配
- 网关/入口鉴权中间件（HTTP/gRPC）

### Interface（入口层）
- Web/Native/API/IM 的身份凭证接入
- 统一将凭证转换为平台内部身份上下文（`user_id`, `role`, `capabilities`）

## 1.2 Authentication vs Authorization 边界

### Authentication（你是谁）
解决“身份真实性”：
- 注册、登录、Token 刷新、登出
- 令牌签发与校验
- 设备与会话识别

输出：可信身份上下文（如 `user_id`、`session_id`、认证强度）。

### Authorization（你能做什么）
解决“操作合法性”：
- 是否具备执行某动作的角色/能力
- 是否可访问某资源（agent/session/memory）
- 是否允许调用某工具

输出：允许/拒绝 + 可审计理由。

### Session Management（会话）
独立关注点，不与认证/授权混杂：
- 管理登录状态生命周期
- 多设备策略
- 强制下线与风险处置

---

## 2. 用户注册与认证

## 2.1 邀请码注册流程（MVP）

1. 用户提交：邀请码 + 手机号/邮箱 + 密码
2. 系统校验邀请码：
   - 存在、未过期、未超使用次数、状态有效
3. 校验账号唯一性（手机号/邮箱）
4. 校验密码强度
5. 密码哈希后写入 `users` 表（`password_hash`）
6. 标记邀请码已使用（或扣减次数）
7. 创建初始会话并签发 Access + Refresh Token
8. 返回登录态

**关键约束**：邀请码校验与用户创建需具备事务一致性，避免并发下重复消费。

## 2.2 登录方式（MVP）

- 账号类型：手机号或邮箱
- 凭证：密码
- 登录流程：
  1. 查询用户
  2. 校验密码哈希
  3. 执行风控（失败次数/限流/设备异常）
  4. 通过后创建会话并签发双 Token

## 2.3 第三方登录预留（演进）

MVP 不上线第三方登录，但预留统一身份绑定模型：
- `identity_providers`（示意）：`provider`, `provider_user_id`, `user_id`, `bind_state`
- 支持后续接入：微信、Apple ID、Google 等
- 原则：**一个外部身份唯一映射一个平台用户**；允许同一平台用户绑定多个外部身份

## 2.4 密码安全策略

- 哈希算法：使用现代抗 GPU 暴力破解算法（推荐 Argon2id；兼容可选 bcrypt/scrypt）
- 存储：仅存 `password_hash`，不存明文、不可逆加密
- 强度要求（MVP 可执行）：
  - 最小长度（如 ≥ 8）
  - 禁止常见弱口令
  - 可逐步升级为更高复杂度策略
- 安全策略：
  - 登录失败次数限制 + 冷却
  - 密码变更后可触发全会话失效（可配置）

---

## 3. Token 体系

## 3.1 双 Token 机制

- **Access Token（JWT）**：短期、用于接口鉴权
- **Refresh Token**：长期、用于换发新 Access Token

设计目标：减少长期凭证暴露面，平衡安全与体验。

## 3.2 Token Claims 结构（Access Token）

建议最小必要声明：
- `iss`：签发方
- `sub`：用户标识（`user_id`）
- `aud`：受众（平台服务）
- `exp` / `iat` / `nbf`：有效期相关
- `jti`：Token 唯一 ID（用于审计与撤销关联）
- `sid`：会话 ID（支持会话级失效）
- `role`：角色（user/pro/admin）
- `cap`：能力集合（可选，避免过大）
- `channel`：来源渠道（web/native/im/api）

> 原则：最小化 claims，避免放入敏感或频繁变化信息。

## 3.3 生命周期（签发/刷新/撤销）

### 签发
- 登录或注册成功后：
  - 签发短期 Access Token（如 15~30 分钟）
  - 签发长期 Refresh Token（如 7~30 天，按风控策略）

### 刷新
- 客户端携带 Refresh Token 调用刷新接口
- 服务端校验：
  - Token 完整性与有效期
  - 是否在撤销列表
  - 是否匹配当前会话状态
- 建议采用 **Refresh Token Rotation（轮转）**：
  - 每次刷新签发新 Refresh Token，旧 token 立即失效
  - 发现重放可判定会话被盗并强制下线

### 撤销
支持三类撤销粒度：
1. Token 级：单 `jti` 失效
2. 会话级：按 `sid` 失效（单设备下线）
3. 用户级：按 `user_id` 全部失效（改密/风控事件）

## 3.4 Token 存储策略

### 客户端
- Native：系统安全存储（Keychain/Keystore）
- Web：
  - Refresh Token 优先 HttpOnly + Secure + SameSite Cookie
  - Access Token 放内存（避免长期落地）

### 服务端
- 不保存 Access Token 全量（默认无状态校验）
- 必须保存 Refresh Token 元数据（哈希后）与会话状态：
  - `sid`, `user_id`, `expires_at`, `revoked_at`, `device_info`, `ip/risk_meta`

---

## 4. 会话管理

## 4.1 多设备登录策略

MVP 建议：
- 允许多设备并存登录（提升可用性）
- 每次登录创建独立 `sid`
- 提供“设备会话列表”与“下线某设备”能力（可先后台可用，前台后补）

## 4.2 会话有效期与续期

- 会话绝对有效期（例如 30 天）
- Refresh 滑动续期（例如每次刷新延长，但不超过绝对上限）
- 高风险行为（异地/异常设备）可触发重新认证

## 4.3 强制登出机制

触发场景：
- 用户主动退出
- 用户修改密码
- 管理员风控操作
- 检测到 Refresh Token 重放

执行策略：
- 将目标 `sid` 或 `user_id` 下所有会话标记为 revoked
- 后续 Access Token 在网关校验/业务校验中被拒绝（结合短 TTL 实现快速收敛）

---

## 5. 授权模型

## 5.1 RBAC 基础模型

角色分层（MVP）：
- `user`（普通用户）
- `pro_user`（高级用户）
- `admin`（管理员）

建议采用“角色授予能力（Capability）”的组合：
- 角色负责默认权限集合
- 能力负责细粒度开关（便于套餐与灰度）

## 5.2 资源级权限（强制所有权校验）

核心原则：**默认拒绝，显式放行**。

- 用户仅能访问自己的：
  - `agent`
  - `session`
  - `memory`
  - 以及其他用户私有域资源
- 校验规则：`resource.owner_user_id == requester.user_id`
- 管理员访问需走受控路径（审计可追踪）

## 5.3 工具级权限

结合既有工具系统（RBAC + Capability）：
- 为每个工具定义：
  - 最低角色要求
  - 必需 capabilities
  - 风险等级（普通/敏感/危险）
- 危险操作启用二次确认（已定方向）
- 工具执行前统一走授权决策器

## 5.4 数据隔离实施层

至少三层防线：
1. **入口层**：身份解析 + 基础授权拦截
2. **应用层**：用例级资源所有权校验
3. **数据访问层**：查询条件强制带 `user_id` 作用域（防越权漏查）

可演进补充：数据库行级安全策略（RLS）或等价机制。

---

## 6. 多渠道认证适配

## 6.1 Native / Web

- 采用统一 JWT 双 Token 流程
- 渠道差异仅体现在凭证存储与风控策略
- 服务端统一映射到内部身份上下文

## 6.2 IM 渠道（平台身份绑定）

- 通过平台 OAuth/平台身份体系拿到外部用户标识
- 在平台侧建立绑定关系：`(platform, platform_user_id) -> user_id`
- 绑定规则：
  - 一个外部身份只能绑定一个平台用户
  - 同一平台用户可绑定多个渠道身份
- IM 请求进入业务前，先完成“外部身份 → 平台用户”映射

## 6.3 API Channel

MVP 与演进分层：
- MVP：API Key（服务到服务/开发者调用）
- 演进：OAuth 2.0（授权码或客户端模式视场景）

统一要求：
- Key/Token 绑定 `user_id` 或租户上下文
- 支持最小权限作用域（scopes）
- 可独立撤销与轮换

---

## 7. 安全防护

## 7.1 暴力破解防护

- 登录接口限流（IP + 账号维度）
- 连续失败锁定与指数退避
- 注册/登录关键路径可接入验证码（按风险触发）
- 审计异常登录模式（撞库特征）

## 7.2 Token 泄露应对

- Access Token 短有效期
- Refresh Token 轮转 + 重放检测
- 异常行为触发会话级/用户级强制下线
- 支持密钥轮换（kid 管理），必要时批量令牌失效

## 7.3 Web 场景 CSRF / XSS 防护

### CSRF
- 若 Refresh Token 使用 Cookie：
  - `SameSite`（Lax/Strict 视业务）
  - 敏感操作要求 CSRF Token 双重校验

### XSS
- 严格输入输出编码
- 内容安全策略（CSP）
- 避免在可被脚本读取的位置持久化敏感 Token

## 7.4 审计日志

覆盖事件：
- 注册、登录成功/失败、刷新、登出
- 会话创建/撤销、密码修改
- 授权拒绝、敏感工具调用、管理员操作

日志字段建议：
- `event_type`, `user_id`, `sid`, `jti`, `channel`, `ip`, `user_agent`, `result`, `reason`, `timestamp`, `trace_id`

要求：
- 审计日志防篡改、可检索、保留周期可配置
- 隐私合规（最小必要、脱敏）

---

## 8. MVP 与演进路线

## 8.1 MVP（当前实现范围）

- 邀请码注册 + 手机/邮箱密码登录
- JWT Access + Refresh 双 Token
- 基础会话管理（创建、刷新、登出、撤销）
- RBAC 三角色 + 资源所有权校验 + 工具 capability 校验
- Native/Web/IM 基础接入
- 基础安全防护（登录限流、审计日志、CSRF/XSS 基础措施）

## 8.2 后续演进

- 第三方登录（微信/Apple/Google）
- API Channel OAuth 2.0 完整化（scopes、授权同意）
- 更细粒度策略引擎（ABAC/策略中心）
- 风险感知认证（设备指纹、异常地理行为、MFA）
- 数据层更强隔离（RLS/分区/租户密钥策略）
- 会话可视化管理与用户自助安全中心

---

## 9. 与约束对齐说明

- 六边形架构：认证抽象在 Domain，Token/哈希/OAuth 适配在 Infrastructure ✅
- 关注点分离：认证、授权、会话三者独立建模 ✅
- YAGNI：MVP 聚焦邀请码+密码+JWT，第三方/OAuth 后置 ✅
- 显式优于隐式：所有关键决策均显式声明（角色、能力、资源归属、会话状态） ✅
- 无代码实现、无协议字段细节 ✅

---

## 10. 验收清单（自检）

- [x] 认证与授权边界清晰
- [x] 注册和登录流程完整
- [x] Token 体系（签发/刷新/撤销）完整
- [x] 多设备会话管理有方案
- [x] 授权模型覆盖 RBAC + 资源级 + 工具级
- [x] 多渠道认证适配有说明
- [x] 安全防护措施完整
- [x] MVP 与演进方向明确
