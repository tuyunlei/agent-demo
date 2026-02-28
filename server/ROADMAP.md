# server ROADMAP

## 已完成

<details>
<summary>重构阶段 R-01 ~ R-09（PR #5-14）</summary>

- [x] **R-01：agent-domain 重构 + agent-types 合并** (PR #5)
- [x] **R-02：agent-context crate** (PR #6)
- [x] **R-03：agent-tools crate** (PR #7)
- [x] **R-04：agent-llm 重构** (PR #8)
- [x] **R-05：agent-memory crate** (PR #9)
- [x] **R-06a：agent-orchestrator rename** (PR #10)
- [x] **R-06b：TurnExecutor 实现** (PR #11)
- [x] **R-07：agent-storage EventStore** (PR #12)
- [x] **R-08：agent-channel + agent-server 重构** (PR #13)
- [x] **R-09：arch 测试更新 + 覆盖率修复** (PR #14)
- [x] **清理废弃文件** (PR #15)
</details>

<details>
<summary>测试覆盖率 T-01 ~ T-05（PR #16-17）</summary>

- [x] **T-01：agent-llm 测试补全** (PR #16)
- [x] **T-02+T-03+T-04：web_search + ports + 零散覆盖** (PR #17)
- [x] **T-05：CI 覆盖率阈值提升** — 65% → 77% → 85%
</details>

<details>
<summary>其他已完成</summary>

- [x] **Docker Compose 部署** (PR #18) — preview + production 双环境
- [x] **Provider Capabilities** (PR #19) — stateful API、builtin tools、compaction layering、ADR-009
</details>

---

## 当前：Lint 强化（代码质量门禁）

### L-01：基础设施 — lint 配置骨架

- `Cargo.toml` 加 `[workspace.lints.clippy]` section，各 crate 继承
- 新建 `clippy.toml`（`too-many-lines-threshold = 50`，`cognitive-complexity-threshold = 25`）
- `agent-proto` crate 级别 `#![allow]` 排除生成代码
- 纯配置，零代码改动

### L-02：安全类型转换 — cast 审计

- 开启 `cast_possible_truncation`、`cast_sign_loss`、`cast_lossless`
- 7 处违规逐个审视：转换语义是否正确、边界情况是否处理
- 重点：JWT 时间戳 `i64 → usize`、DB 行 `f64 → i64` / `i64 → u32`、timeout `i32 → u64`

### L-03：unwrap 禁令 — 错误处理门禁

- 开启 `unwrap_used`（生产代码 deny，`#[cfg(test)]` allow）
- 当前生产代码 0 违规，此步为防止未来引入
- 扫描现有 `expect()` 的 reason 是否有意义

### L-04：函数规模控制 — 拆分大函数

- 开启 `too_many_lines`（阈值 50）、`cognitive_complexity`（阈值 25）
- 4 处真实违规：
  - `web_search.rs` (56 行)、`provider.rs` (51 行)
  - `turn_executor.rs` (133 行，核心 turn loop)
  - `event_store.rs` (53 行)
- 拆分边界需审视抽象层次一致性
- `mock.rs` (73 行) 加 `#[allow]`（测试辅助，拆了反而难读）

### L-05：must_use 审计

- 开启 `must_use_candidate`
- 32 处逐个判断：返回值被忽略是否真有问题
- 重点关注：有没有调用方忽略了重要返回值的 bug
- 确实不需要的加 `#[allow]` + 注释

---

## 后续：测试覆盖率提升（目标 90%）

当前覆盖率：85.96%（CI 阈值 85%）。长期目标 90%。
