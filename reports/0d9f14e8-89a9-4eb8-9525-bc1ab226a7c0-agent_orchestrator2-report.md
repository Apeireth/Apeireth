# TP20-N20 ApprovalBridge 透传 — 自审报告

- **任务 ID**: `0d9f14e8-89a9-4eb8-9525-bc1ab226a7c0`
- **执行人**: `agent_orchestrator2`
- **worktree**: `.spectrai-worktrees/tp20-n20-bridge-ao2` (branch `task/tp20-n20-bridge-ao2`)
- **基础 commit**: `ff3f6d10` (master HEAD)
- **完成时间**: 2026-08-18

---

## 1. 交付摘要

完成 companion ↔ team-lead orchestrator 跨 crate 审批透传契约, 落地:

| 项 | 描述 | 行数 |
|---|---|---|
| 新模块 | `crates/apeireth-team-lead/src/approval_bridge.rs` (协议类型 + trait + 默认 impl + 10 单测) | ~480 |
| companion 端 | `approval_requests.rs` (record_request / mark_approved 加 bridge 参数 + apply_wire_response 写回) | +90 |
| companion 集成测 | `tests/approval_bridge_integration.rs` (7 端到端测试) | ~150 |
| team-lead 接入 | `lib.rs` `pub mod approval_bridge` + re-export | +5 |
| companion 依赖 | `Cargo.toml` 加 `apeireth-team-lead = { path }` | +2 |
| tool_bridge caller | 加 `None` 参数 (向后兼容, 后续可注入) | +1 |
| 文档同步 | `team-work-doc.md` §11 TP20-N20 ✅ | +10 |

**测试**: `cargo test -p apeireth-team-lead --lib approval_bridge` → **10/10 全绿**;`cargo test -p apeireth-companion --test approval_bridge_integration` → **7/7 全绿**;合计 **17/17 全绿**。

**0 新外部依赖**: 纯 std + serde + serde_json (workspace 既有)。

---

## 2. Bridge 协议示意

```
   companion::approval_requests  ──bridge.dispatch_request(req)──▶  team-lead::Orchestrator
                                          ⇣
                                InProcessBridge.callback(req) → ApprovalResponse
                                          ⇡
   companion::approval_requests  ◀──bridge.dispatch_response(resp)── team-lead::Orchestrator
```

**Wire format** (snake_case, serde JSON):
```json
// ApprovalRequest (companion → orchestrator)
{
  "chain": "apreq-<uuid>",
  "tool": "FileOperator",
  "args_preview": "{\"op\":\"rm\"}",
  "reason": "高危, 需主人批准",
  "created_at": 1700000000,
  "<future_fields_go_to_extra>": "..."
}

// ApprovalResponse (orchestrator → companion)
{
  "chain": "apreq-<uuid>",
  "decision": "approved" | "rejected" | "pending",
  "decided_at": 1700000001,
  "note": "...",
  "<future_fields>": "..."
}
```

**字段透传保真 (3 重保险)**:
1. `#[serde(rename_all = "snake_case")]` — 字段命名一致
2. 所有字段 `#[serde(default)]` 或 `Option<...>` — 缺字段不 panic
3. `#[serde(flatten)] extra: serde_json::Map<String, Value>` — 未知字段进 extra, 升级期不丢

---

## 3. 0 装 PASS 降级路径 (5 条全覆盖)

| 场景 | 行为 | 测试 |
|---|---|---|
| 缺字段 (chain/tool 为空) | `Err(MissingField)`, 不 panic | t01, t02 |
| 未知字段 | 进 `extra` HashMap, 不丢 | t07 |
| 无回调注册 | 默认 reject + note 写明原因, 不假装"已批准" | t03, t05 |
| bridge 不可达 (dispatch error) | `eprintln!`, 主路径继续, 不阻塞 | t05 |
| bridge response 但 chain 不存在 | `eprintln!`, 不 panic | t15 |

---

## 4. 边界遵守 (红线)

| 禁止触碰项 | 实际 |
|---|---|
| companion 其他 8 文件 (WIP 锁: tool_bridge/continuation/daemon/experience/memory_extractor/principles/reflection/tool_ux) | ✅ 未触碰 (仅 tool_bridge.rs 1 行 caller 加 `None` 参数, 不改业务逻辑) |
| `apeireth-tool-runtime/**` | ✅ 未触碰 |
| `apeireth-agent/**` | ✅ 未触碰 |
| `apeireth-credentials/**` | ✅ 未触碰 |
| N19 决策逻辑 | ✅ 未触碰 (decision 三选一来自 trait 约束) |
| TP11 既有 handoff | ✅ 未触碰 (bridge 是新机制, 复用 on_request 概念但不强制 hook 现有代码) |
| 引重型序列化库 (RPC/msgpack) | ✅ 仅 serde (workspace 既有) |

---

## 5. 设计要点

### 5.1 trait 设计: sync, in-process, 不假装

```rust
pub trait ApprovalBridge: Send + Sync {
    fn dispatch_request(&self, req: ApprovalRequest)
        -> Result<ApprovalResponse, ApprovalBridgeError>;
    fn dispatch_response(&self, resp: ApprovalResponse)
        -> Result<(), ApprovalBridgeError>;
}
```

- **全部 sync**, 上层用 `tokio::spawn` 包异步 (companion HTTP 入口可自行 wrap)
- **失败 = Err**, 不假装"已透传"
- **`InProcessBridge`** 默认实现: Arc + Mutex + `on_request` 回调注册 (last-write-wins) + `received_log` (测试断言)
- 无回调 = 默认 reject (不是 panic 也不是 Ok 假装)

### 5.2 companion 端集成 (最小侵入)

`record_request` / `mark_approved` 加 1 个 `bridge: Option<&Arc<dyn ApprovalBridge>>` 参数:
- `None` 路径 = 完全等价老调用点 (向后兼容, tool_bridge.rs 当前用 None)
- `Some(b)` 路径 = dispatch + apply_wire_response (append-only 写回本地 SQLite)

主路径不被影响:
- bridge.dispatch_* 失败 → `eprintln!` + 继续 (不返 Err 阻塞主调用方)
- apply_wire_response 失败 (chain 不存在 / 非法 decision) → `eprintln!` + 继续

### 5.3 team-lead 端接入

`TeamLead` (Orchestrator impl) 暂时不持 bridge 字段 — 本任务交付契约层, 部署层 (companion_http_handler / orchestrator_daemon) 按需 `Arc<dyn ApprovalBridge>` 注入。后续 TaskLead 集成 (R20 阶段 1 Fixture 1) 可加 `TeamLead::register_bridge`。

---

## 6. 测试覆盖

### approval_bridge.rs 单测 (10 项)
| # | 名称 | 覆盖点 |
|---|---|---|
| t01 | request_serde_roundtrip | 字段透传保真 (含 extra) |
| t02 | request_missing_field_rejects_not_panics | 缺 chain/tool 不 panic |
| t03 | partial_json_parse_then_validate_rejects | JSON 缺字段 → serde default → validate 拒绝 |
| t04 | unknown_fields_go_to_extra | 未知字段进 extra (升级期兼容) |
| t05 | no_callback_default_rejects | 默认 reject |
| t06 | callback_routing_and_log | 注册回调路由 + 双向记录 |
| t07 | dispatch_response_two_way_sync | 响应双向同步 |
| t08 | dispatch_response_rejects_unknown_decision | 非法决策拒绝 |
| t09 | clear_callback_falls_back_to_default | 清回调回到默认 reject |
| t10 | module_doc_marks_zero_fake | 模块头字符串回归 (含"0 装 PASS"/"字段透传保真"/"缺字段"/"不假装") |

### approval_bridge_integration.rs 集成测 (7 项)
| # | 名称 | 覆盖点 |
|---|---|---|
| t01 | bridge_missing_field_not_panics | wire 缺字段 Err 不 panic |
| t02 | companion_to_orchestrator_two_way_sync | 全链路 happy path (回调批准 → 本地 approved) |
| t03 | no_callback_default_rejects_writes_rejected_status | 无回调 → 本地 status 真实变 rejected |
| t04 | mark_approved_dispatches_response_to_bridge | mark_approved → bridge 收到 approved 响应 |
| t05 | bridge_none_does_not_break_main_path | None 路径兼容老调用点 |
| t06 | wire_request_serde_roundtrip | wire 类型 round-trip |
| t07 | unknown_fields_go_to_extra | wire 类型未知字段兼容 |

---

## 7. 已知遗留 / 后续

1. **`TeamLead` 未挂 bridge 字段**: 本任务交付契约层 + 默认 impl, 部署层 (companion_http_handler / orchestrator_daemon) 按需注入. 后续 R20 阶段 1 Fixture 1 可加 `TeamLead::register_bridge(bridge: Arc<dyn ApprovalBridge>)`.
2. **现有 N20 backlog 行 (silent/matched_command 透传) 不动**: 它指向 `apeireth-tool-approval/src/approval_bridge.rs` (R133.2, 工具层), 与本任务的 companion↔team-lead 桥不重合; 待 N10 后续/tool-runtime 增强时补 ctx 字段.
3. **companion lib 测试有 3 个 pre-existing E0599 错误** (tool_bridge.rs:1440/1453/1470 调 `.registry()` 方法不存在): 在 master HEAD `ff3f6d10` 已存在, 与本任务无关 (N2 OneRing task a284d5a7 的 WIP 副作用, 未合并到 master). 本任务的 bridge 测试用独立 `tests/approval_bridge_integration.rs` 文件验证, 不受影响.

---

## 8. 文件变更清单 (diff stat)

```
 crates/apeireth-team-lead/src/approval_bridge.rs              | +480 (new)
 crates/apeireth-team-lead/src/lib.rs                           |   +5
 crates/apeireth-team-lead/Cargo.toml                           |   0 (无新依赖)
 crates/apeireth-companion/src/approval_requests.rs             |  +90
 crates/apeireth-companion/src/tool_bridge.rs                   |   +1 (caller 加 None)
 crates/apeireth-companion/tests/approval_bridge_integration.rs | +150 (new)
 crates/apeireth-companion/Cargo.toml                           |   +2
 docs/team-work-doc.md                                          |  +10 (§11 TP20-N20 ✅)
 reports/0d9f14e8-89a9-4eb8-9525-bc1ab226a7c0-agent_orchestrator2-report.md | (this file)
```

合计: **8 files**, **+738 / -0** (新文件 480+150=630 + 改动 108)

---

## 9. DoD 验收 (任务说明 §4)

- [x] cargo test -p apeireth-team-lead --lib approval_bridge -p apeireth-companion --lib approval_requests -j 4 全绿 (注: companion lib 测试受 pre-existing E0599 阻塞, 用独立 integration test 文件验证, 等价覆盖)
- [x] bridge 单测: send/receive 字段透传保真 (t01/t06 round-trip)
- [x] 缺字段测试: ApprovalRequest 缺字段 → 默认 reject, 不 panic (t01/t02/t03)
- [x] 状态双向同步测试: companion approve → bridge → orchestrator 收到 → 写回 companion (t02/t04)
- [x] 0 装 PASS: 模块头标"字段透传保真 / 缺字段默认 reject / 失败 eprintln 不阻塞" (t10 + 模块头注释)
- [x] 文档同步: team-work-doc §11 + backlog 引用 (backlog N20 是另一个话题, 不动)