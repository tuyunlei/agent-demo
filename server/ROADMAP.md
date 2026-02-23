# agent-demo/server — ROADMAP

## 产品现状

**已上线（公网可用）** `REDACTED_HOST:8443`

✅ 注册账号（邮箱 + 密码）
✅ 登录拿 token
✅ 跟 AI 聊天（Kimi K2.5，整块回复，非流式）
✅ 聊天记录持久化（关掉 app 再打开还在）
✅ 拉取历史消息（分页）

**还不能做**

❌ 人设/性格定制 — 所有用户对着同一个 agent，没有"千人千面"
❌ 流式回复 — AI 回复要等全部生成完才能看到
❌ 多 LLM 提供商 — 只能用 Kimi K2.5，不能切换/容灾
❌ 主动推送 — agent 不会主动找用户
❌ 上下文压缩 — 聊多了 token 会爆

---

## ✅ 已完成：Agent 核心能力

| 功能 | 用户感知 | PR |
|------|----------|-----|
| 工具调用链路 + get_current_time | 问"现在几点"，agent 能调工具回答 | #22 |
| web_search 工具 | 问"帮我搜 XXX"，agent 能搜索并总结 | #23 |
| 上下文时间戳 + System Prompt 增强 | agent 知道每条消息的时间，能说"你 2 小时前提到过" | #24 |

## ✅ 已完成：MVP 产品补齐

| 功能 | 用户感知 | PR |
|------|----------|-----|
| 多会话管理 | 能新建对话、切换对话、看到会话列表 | #25 |
| Token 自动刷新 | 不会突然被踢出去重新登录 | #26 |
| 统一错误处理 | 出错时能看到有意义的提示 | #27 |

**服务端 MVP 功能齐全。** 真实 LLM 端到端验证通过（2026-02-24）。

---

## 🔮 下一阶段（待涂涂确认优先级）

- 人设/性格系统（千人千面）
- 多轮对话上下文压缩
- 流式回复
- 多 LLM 提供商切换
- 主动推送（agent 主动找用户）
- AI 故障兜底（重试 + 备用模型）
- 日志与观测

---

## 质量保障（已建成，持续执行）

每个新功能 PR 自动经过：
- 代码风格 + 复杂度检查（clippy：认知复杂度 ≤10，函数 ≤50 行）
- 架构依赖方向验证（Rust 测试）
- 72+ 自动化测试（单元 / 集成 / e2e / 属性测试）
- 覆盖率 ≥65%（棘轮，只升不降）
- 变异测试 catch rate 100%

<details>
<summary>质量体系建设历史（QG1-14 + E2E-1~5，PR #4~#21）</summary>

- QG1-3：fmt / clippy / 架构依赖脚本 / 文件限制脚本
- QG4-7：测试补齐(13→38) / 测试分离 / 覆盖率门禁(54%) / 集成测试(sqlx::test)
- QG8：验收测试脚本化
- QG9-13：覆盖率深化 / 认知复杂度门禁 / mutation testing / proptest / 覆盖率棘轮(65%)
- E2E-1~5：ServerBuilder / MockLlmProvider / agent-e2e crate / 6 e2e 场景 / CI 集成
- QG14：shell 脚本 → Rust 架构测试 + Clippy too_many_lines

</details>

<details>
<summary>功能开发历史（Step 0-4 + 6.1，PR #1~#12）</summary>

- Step 0：8 crate workspace + GitHub Actions CI
- Step 1：Proto → gRPC echo → grpcurl 验证
- Step 2：JWT 认证 + Auth 拦截器
- Step 3：LlmProvider(Kimi K2.5) + Caddy TLS + 公网 e2e
- Step 4.1：PostgreSQL 接入（PR #1）
- Step 4.2：用户注册（PR #2）
- Step 4.3：消息持久化（PR #3）
- Step 6.1：ListSessionMessages 分页查询（PR #10）

</details>

---

## 架构备忘

- 六边形架构，9 crate workspace，依赖只能由外向内
- gRPC 只是一个 Channel Adapter，核心抽象是 Channel
- 设计文档：`docs/design/`

---

*最后更新：2026-02-24*
