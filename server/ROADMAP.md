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

---

## 当前：测试覆盖率提升（目标 90%）

当前覆盖率：70.92%（844/1190 行）。长期目标 90%。

### T-01：agent-llm 测试补全（预计最大）

| 文件 | 现状 | 目标 | 预估工作 |
|------|------|------|---------|
| `provider.rs` | 49/83 | ~80/83 | 需 mock HTTP：build_request_body、parse_success_body、map_http_error、parse_tool_calls、parse_finish_reason |
| `provider_compat.rs` | 0/17 | 17/17 | 纯映射函数，直接测 |
| `error.rs` | 0/6 | 6/6 | is_retryable / is_fallbackable |
| `traits.rs` | 0/6 | ~4/6 | stream default impl 返回 UnsupportedCapability |
| `mock.rs` | 22/59 | ~45/59 | 测试工具，优先级低但能提覆盖率 |

**预估**：~15 个测试，中等复杂度（provider.rs 需要 mock HTTP 或重构为可测试结构）

### T-02：agent-tools web_search 测试

| 文件 | 现状 | 目标 | 预估工作 |
|------|------|------|---------|
| `web_search.rs` | 18/65 | ~55/65 | HTTP 调用 mock，parse 逻辑测试 |

**预估**：~5 个测试，需要 mock HTTP client 或提取纯函数

### T-03：agent-domain ports 测试

| 文件 | 现状 | 目标 | 预估工作 |
|------|------|------|---------|
| `ports.rs` | 4/16 | ~14/16 | EventStoreError Display、NewEvent/AppendResult/EventRange 构造 |

**预估**：~5 个测试，简单

### T-04：零散补全（多个 crate 小文件）

| 文件 | 现状 | 目标 |
|------|------|------|
| `agent-channel/grpc/error.rs` | 12/21 | ~20/21 |
| `agent-context/section.rs` | 1/6 | ~5/6 |
| `agent-orchestrator/turn_types.rs` | 2/9 | ~8/9 |
| `agent-orchestrator/turn_executor.rs` | 106/120 | ~115/120 |
| `agent-storage/event_store_row.rs` | 54/66 | ~62/66 |

**预估**：~10 个测试，简单到中等

### T-05：CI 配置更新 + 白名单机制

- 更新 tarpaulin exclude-files 加入白名单
- 提升 THRESHOLD 到当前实际值（棘轮）
- CLAUDE.md 已更新

---

## 预估总计

| 任务 | 测试数 | 复杂度 | 可否合并 |
|------|--------|--------|---------|
| T-01 | ~15 | 中 | 独立（最大任务） |
| T-02 | ~5 | 中 | 可与 T-04 合 |
| T-03 | ~5 | 低 | 可与 T-04 合 |
| T-04 | ~10 | 低 | — |
| T-05 | 0 | 低 | 独立（CI 配置） |

**推荐拆分**：T-01 单独一个 sub-agent，T-02+T-03+T-04 合并一个 sub-agent，T-05 PM 直接做。
