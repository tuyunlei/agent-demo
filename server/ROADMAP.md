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

## 已完成：Lint 强化（L-01~L-05，PRs #20-22）

<details>
<summary>展开查看</summary>

- [x] L-01：lint 配置骨架
- [x] L-02：cast 审计（cast_possible_truncation, cast_sign_loss, cast_lossless）
- [x] L-03：unwrap 禁令（deny unwrap_used）
- [x] L-04：函数规模控制（≤30行，复杂度≤10）
- [x] L-05：must_use 审计
</details>

---

## 待办

详见 `tasks/QUEUE.md`。

### 测试覆盖率提升（目标 90%）

当前覆盖率：85.96%（CI 阈值 85%）。长期目标 90%。
