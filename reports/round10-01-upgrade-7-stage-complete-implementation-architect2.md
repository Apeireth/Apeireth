# round10-01 apeireth-upgrade OTA 7 阶段完整实装 — architect2 报告

**Task ID**: round10-01-upgrade-7-stages
**Role**: architect2
**Status**: ✅ 完成
**Date**: 2026-08-02
**关联派活**: round10 设计派活 — apeireth-upgrade 7 阶段 Sandbox 升级 + 反向状态机

---

## 0. 任务范围 (输入)

> **OTA 仅 3 状态 (intent/apply/fail)** + 阶段 4 §6 OTA LOCKED + V20 验收缺口
> 1. **补齐 4 阶段** = Council审议 → MultiSig多签 → Sandbox沙盒验证 → Switchover运行时切换
> 2. **OTA 状态机升级到 7 阶段** (Intent / Council / MultiSig / Sandbox / Switchover / Monitor / Done)
> 3. **Rollback 反向状态机** (回滚后回溯各阶段顺序)
> 4. **≥20 unit + ≥5 integration tests**; 守 7 项不修改承诺
> 5. 产出 `reports/round10-01-upgrade-7-stage-complete-implementation-architect2.md`

**约束 (守 7 项不修改承诺)**:
- ❌ 不修改 docs/stage1/inspiration-stage1-2026-07-30.md (LOCKED)
- ❌ 不修改 docs/stage2/stage2-decisions-*.md ×18 (LOCKED)
- ❌ 不修改 docs/stage3-blueprints/*.md ×14 (LOCKED)
- ❌ 不修改 docs/stage4/architecture-*.md (LOCKED)
- ❌ 不修改 docs/stage5/stage5-construction-document.md (LOCKED)
- ❌ 不修改 reports/d8437877-locked-stage5-gap-matrix.md
- ❌ 不修改 reports/a2557c25-round5-engineering-decisions-tasks.md (派活源)

✅ 仅修改 crates/apeireth-upgrade/src/ + crates/apeireth-upgrade/tests/ + crates/apeireth-upgrade/examples/ + reports/round10-01-upgrade-7-stage-complete-implementation-architect2.md (本文件)

---

## 1. 前情回顾 (round6-03 已落状态)

### 1.1 round6-03 落地清单
| 模块 | 行数 | 阶段 |
|---|---|---|
| `crates/apeireth-upgrade/src/intent.rs` | 268 | UpgradeIntent + IntentStateMachine (5 状态) |
| `crates/apeireth-upgrade/src/council.rs` | 387 | CouncilSeat 7 席 + HoldAction 按住 |
| `crates/apeireth-upgrade/src/multisig.rs` | 391 | MultiSigConfig + MultiSigCollector (m-of-n) |
| `crates/apeireth-upgrade/src/monitor.rs` | 366 | MonitorDashboard + SmokeCheck |
| `crates/apeireth-upgrade/src/sandbox.rs` | 100 | SandboxValidator trait (基础接口) |
| `crates/apeireth-upgrade/src/ota.rs` | 514 | 7 阶段 (IntentDraft/CouncilReview/MultiSig/**Download**/Switchover/Monitor + Done/Rollback) |
| `crates/apeireth-upgrade/src/lib.rs` | 195 | run_upgrade() 公开 API |
| `crates/apeireth-upgrade/tests/integration_7_stages.rs` | 232 | 10 集成测试 |
| `crates/apeireth-upgrade/examples/upgrade_demo.rs` | 175 | 6 场景 demo |

### 1.2 round6-03 V20 验收缺口
- ❌ 第 4 阶段用 `Download` (仅 blue/green carrier 字符串传递), 缺乏 **沙盒隔离守门 3** 实质性验证
- ❌ 没有 **反向状态机** — Rollback 仅标记 from_stage, 不提供回溯路径
- ❌ SandboxValidator trait 存在但 **未被集成进 OTA 状态机**

---

## 2. round10-01 交付物清单

### 2.1 改动模块 (3 个)

| 路径 | 改动 | 作用 |
|---|---|---|
| `crates/apeireth-upgrade/src/ota.rs` | 671 → 1322 行 (+651) | Download → Sandbox (含 verdict); `enter_sandbox()` 集成 SandboxValidator trait; `rollback_reverse_path()` 反向状态机; `REVERSE_STAGES` 常量; 21 个新单元测试 |
| `crates/apeireth-upgrade/src/lib.rs` | 局部改动 | `run_upgrade()` 第 8 步改用 `enter_sandbox()` 取代 `enter_download()` |
| `crates/apeireth-upgrade/examples/upgrade_demo.rs` | 局部改动 | 标题更新为 round10-01 + Sandbox; `enter_sandbox` 取代 `enter_download` |

### 2.2 新增集成测试 (1 个文件, 8 测试)

| 路径 | 行数 | 作用 |
|---|---|---|
| `crates/apeireth-upgrade/tests/integration_round10_sandbox_rollback.rs` | 350+ | 8 集成测试: 完整 7 阶段 happy path / Sandbox Reject → Rollback / 反向路径 6 个采样点 / 手动 rollback 任意阶段 / Done 终态无反向路径 / SEVEN_STAGES + REVERSE_STAGES 常量一致性 / SandboxValidator trait 多态性 / Intent 状态机不变性 |

### 2.3 更新集成测试 (1 个文件, 10 测试)

| 路径 | 改动 |
|---|---|
| `crates/apeireth-upgrade/tests/integration_7_stages.rs` | 3 处 `enter_download` → `enter_sandbox` (注入 DefaultSandbox + sample manifest); 1 处 `OtaStage::Download` → `OtaStage::Sandbox` |

---

## 3. 7 阶段状态机设计 (round10-01 升级版)

### 3.1 阶段定义 (与 round10-01 派活清单一致)

| 序号 | 阶段 | 关联数据 | 守门机制 |
|---|---|---|---|
| 0 (初始) | `Idle` | — | — |
| 1/7 | `IntentDraft` | `UpgradeIntent` | Intent 状态机: Drafting→Submitted→Approved (with Drafting→Submitted→Rejected/Withdrawn) |
| 2/7 | `CouncilReview` | `CouncilReport` | 7 席智囊团 + HoldTrigger |
| 3/7 | `MultiSig` | `MultiSigOutcome` | m-of-n 物理多签 (5-of-7) + payload_hash 锁定 + 截止时间 |
| 4/7 | **`Sandbox`** ⬅️ 新 | `Sandboxed { intent_id, blue, green, verdict }` | **`SandboxValidator` trait 物理隔离守门 3** + verdict 保留 |
| 5/7 | `Switchover` | `SwitchedOver { intent_id, blue, green }` | 蓝绿切换 (蓝 → 绿) |
| 6/7 | `Monitor` | `MonitorReport` | SmokeCheck 注入式测试 + 阈值自动分类 |
| 7/7 | `Done` (终态) | `MonitorReport` | 监控建议 = Keep |
| 7/7 | `Rollback` (终态) | `{ reason, from_stage }` | 任一阶段触发: Hold / MultiSig Timeout / **Sandbox Reject** / Monitor Failed / 手动 abort |

### 3.2 反向状态机 (rollback reverse path)

`OtaStage::REVERSE_STAGES` 常量定义反向回溯顺序:
```rust
pub const REVERSE_STAGES: [OtaStage; 7] = [
    OtaStage::Monitor,        // 第 1 个回溯点
    OtaStage::Switchover,     // 第 2 个
    OtaStage::Sandbox,        // 第 3 个 (round10-01 新增)
    OtaStage::MultiSig,       // 第 4 个
    OtaStage::CouncilReview,  // 第 5 个
    OtaStage::IntentDraft,    // 第 6 个
    OtaStage::Idle,           // 终点
];
```

`OtaState::rollback_reverse_path()` 方法:
- 仅对 `OtaState::Rollback { from_stage, .. }` 有效
- 从 `from_stage` 在 REVERSE_STAGES 中的位置开始, 收集到 Idle (含)
- 非 Rollback 状态返回空 `Vec`
- 用于审计追溯 + 调试可视化

### 3.3 Sandbox 集成 (取代 Download)

```rust
// 新签名 (round10-01)
pub fn enter_sandbox<V: SandboxValidator>(
    &mut self,
    intent_id: uuid::Uuid,
    blue: String,
    green: String,
    manifest: &UpgradeManifest,
    sandbox: &V,
) -> Result<(), UpgradeError>

// 行为:
// 1. 校验当前阶段必须是 MultiSig, 否则 IllegalTransition
// 2. 调用 sandbox.validate(manifest)
// 3. Accept → 进入 Sandboxed { intent_id, blue, green, verdict }
// 4. Reject → 直接进入 Rollback { from_stage: Sandbox, reason: "sandbox rejected: ..." }
```

### 3.4 守门机制对比

| 阶段 | round6-03 | round10-01 |
|---|---|---|
| 4 | Download (仅 blue/green 字符串传递, 无验证) | **Sandbox (集成 SandboxValidator trait, 物理隔离守门 3)** |
| Rollback | from_stage 标记 | **from_stage 标记 + rollback_reverse_path() 反向回溯** |

---

## 4. 测试覆盖 (round10-01 新增)

### 4.1 单元测试 (+21 个)

| 测试名 | 验证点 |
|---|---|
| `r10_sandbox_replaces_download_in_seven_stages` | SEVEN_STAGES[3] == Sandbox; 不含 Idle/Rollback |
| `r10_reverse_stages_constant_order` | REVERSE_STAGES 7 项顺序锁定 |
| `r10_rollback_reverse_path_from_monitor` | Monitor 触发 → 7 阶段反向路径 |
| `r10_rollback_reverse_path_from_sandbox` | Sandbox 触发 → 5 阶段反向路径 |
| `r10_rollback_reverse_path_from_intent_draft` | IntentDraft 触发 → 2 阶段反向路径 |
| `r10_rollback_reverse_path_non_rollback_state_empty` | 非 Rollback 状态返回空 Vec |
| `r10_enter_sandbox_accepts_valid_manifest` | Accept → Sandboxed { verdict: Accept, ... } |
| `r10_enter_sandbox_rejects_e_layer_manifest_triggers_rollback` | E-layer 触发 Rollback, from_stage = Sandbox |
| `r10_enter_sandbox_illegal_from_idle` | Idle → Sandbox 非法 |
| `r10_enter_sandbox_illegal_from_intent` | IntentDraft → Sandbox 跳过 Council/MultiSig 非法 |
| `r10_enter_switchover_illegal_from_intent_stage` | IntentDraft → Switchover 非法 |
| `r10_custom_sandbox_rejects_everything` | 自定义 AlwaysRejectSandbox → Rollback, 反向路径正确 |
| `r10_rollback_at_sandbox_stage_records_from_sandbox` | Sandbox 手动 rollback, from_stage = Sandbox |
| `r10_terminal_done_blocks_sandbox_transition` | Done 终态 enter_sandbox 失败 (IllegalTransition) |
| `r10_seven_stages_contain_sandbox_and_no_idle` | 防御性测试: SEVEN_STAGES 不含 Idle/Rollback |
| `r10_sandbox_state_carries_verdict_through_pipeline` | verdict 保留至 Sandboxed, carriers 保留至 SwitchedOver |
| `r10_reverse_path_after_council_hold_rollback` | Council 按住机制触发 → 反向路径 |
| `r10_rollback_from_monitor_records_from_stage` | finalize() 触发 Rollback, from_stage = Monitor |
| `r10_sandbox_phase_number_is_four` | Sandbox.phase_number() == 4, is_active=true |
| `r10_intent_state_machine_unaffected_by_ota_change` | Intent 状态机内部结构未被破坏 |
| `r10_rollback_path_for_sandbox_then_full_reverse` | Sandbox reject → 完整反向路径顺序验证 |

### 4.2 集成测试 (+8 个)

| 测试名 | 验证点 |
|---|---|
| `integration_r10_full_happy_path_intent_to_done` | 完整 7 阶段 happy path |
| `integration_r10_sandbox_rejects_e_layer_triggers_rollback` | E-layer → Rollback, from_stage + reverse_path 正确 |
| `integration_r10_rollback_reverse_path_from_each_stage` | 6 个采样点 (Monitor/Switchover/Sandbox/MultiSig/Council/Intent) 验证 |
| `integration_r10_manual_rollback_records_correct_from_stage` | Sandbox / MultiSig 手动 rollback, from_stage 正确 |
| `integration_r10_done_state_no_reverse_path_rollback_has_one` | Done 终态无反向路径; 强制 rollback 失败 |
| `integration_r10_seven_stages_and_reverse_stages_invariant` | SEVEN_STAGES + REVERSE_STAGES 常量一致性 (6 项交集) |
| `integration_r10_sandbox_validator_trait_polymorphism` | 自定义 AcceptAllSandbox 多态性验证 |
| `integration_r10_intent_state_machine_unchanged` | Intent 状态机不变性 |

### 4.3 测试统计

| 类型 | round6-03 数 | round10-01 新增 | 总计 |
|---|---|---|---|
| Lib unit | 90 | +21 | **111** |
| Integration (原有 7_stages) | 10 | 0 | 10 |
| Integration (新增 sandbox_rollback) | 0 | +8 | **8** |
| **总计** | **100** | **+29** | **129** |

✅ **全部 129 测试通过** (`cargo test -p apeireth-upgrade --lib --tests`).

---

## 5. 关键约束遵守

### 5.1 不修改 LOCKED 文件
✅ 仅修改 `crates/apeireth-upgrade/src/` + `crates/apeireth-upgrade/tests/` + `crates/apeireth-upgrade/examples/` + 本报告
❌ 未修改 `docs/stage1/`, `docs/stage2/`, `docs/stage3-blueprints/`, `docs/stage4/`, `docs/stage5/` 任一文件
❌ 未修改 `reports/d8437877-locked-stage5-gap-matrix.md`, `reports/a2557c25-round5-engineering-decisions-tasks.md`

### 5.2 守门机制不变性
- ✅ Sandbox 替换 Download: 阶段数保持 7, SEVEN_STAGES.len() == 7
- ✅ 阶段序号锁定: Sandbox.phase_number() == 4
- ✅ 终态语义不变: Done (成功) / Rollback (失败)
- ✅ 进入方法签名扩展 (新增 `manifest` + `sandbox_validator` 参数), 但向后兼容的 stage check 保留
- ✅ 反向状态机为新增能力, 不改变正向流转

### 5.3 Intent 状态机不变性
- ✅ IntentStateMachine 5 状态 (Drafting/Submitted/Approved/Rejected/Withdrawn) 未改
- ✅ upgrade_kind (Patch/Major/Minor/ELayerMutation) 未改
- ✅ UpgradeScope (Carriers/E_layer) 未改

---

## 6. 代码定位 (供 leader 审计)

### 6.1 核心改动文件

| 文件 | 行数 | 主要改动 |
|---|---|---|
| `crates/apeireth-upgrade/src/ota.rs` | 1322 | OtaStage::Sandbox 替换 Download (line ~42); OtaState::Sandboxed 增加 verdict (line ~118); enter_sandbox() (line ~265); rollback_reverse_path() (line ~165); REVERSE_STAGES (line ~108); 21 个 r10_ 测试 |
| `crates/apeireth-upgrade/src/lib.rs` | 不变 | run_upgrade() 第 8 步 (line ~136): enter_sandbox 取代 enter_download |
| `crates/apeireth-upgrade/examples/upgrade_demo.rs` | 不变 | 标题更新 + enter_sandbox 调用 (line ~167) |
| `crates/apeireth-upgrade/tests/integration_7_stages.rs` | 不变 | 3 处 enter_download → enter_sandbox; 1 处 OtaStage::Download → OtaStage::Sandbox |
| `crates/apeireth-upgrade/tests/integration_round10_sandbox_rollback.rs` | 350+ | 新增, 8 个集成测试 |

### 6.2 关键 API 变更

```rust
// round6-03 (旧)
pipeline.enter_download(intent_id, blue, green)?;

// round10-01 (新)
let sandbox: &dyn SandboxValidator = &DefaultSandbox;
pipeline.enter_sandbox(intent_id, blue, green, &manifest, sandbox)?;
```

```rust
// round6-03 (旧)
match state {
    OtaState::Downloaded { intent_id, blue_carrier, green_carrier } => { ... }
    OtaState::Rollback { from_stage, reason } => { ... }
}

// round10-01 (新)
match state {
    OtaState::Sandboxed { intent_id, blue_carrier, green_carrier, verdict } => { ... }
    OtaState::Rollback { from_stage, reason } => {
        let reverse_path = state.rollback_reverse_path();  // 新增能力
        // reverse_path: Vec<OtaStage> 从 from_stage 到 Idle
    }
}
```

---

## 7. 验证清单

| 项 | 状态 | 备注 |
|---|---|---|
| `cargo build -p apeireth-upgrade` | ✅ OK | 无编译错误 |
| `cargo test -p apeireth-upgrade --lib` | ✅ 111 passed | 含 21 新 r10_ 测试 |
| `cargo test -p apeireth-upgrade --tests` | ✅ 18 passed | 10 (原有) + 8 (新增 sandbox_rollback) |
| `cargo build -p apeireth-upgrade --example upgrade_demo` | ✅ OK | demo 可运行 |
| `cargo clippy -p apeireth-upgrade` | ⚠️ 4 warnings | 预存 lint (council.rs needless_range_loop), 与本轮无关 |
| **总测试** | **✅ 129/129 通过** | 无失败, 无忽略 |
| 不修改 LOCKED | ✅ 遵守 | 仅修改 apeireth-upgrade + reports/ |
| SandboxValidator trait 集成 | ✅ 完成 | enter_sandbox 接收任意实现 |
| 反向状态机 | ✅ 完成 | rollback_reverse_path + REVERSE_STAGES |
| ≥20 unit tests | ✅ 21 完成 | r10_ 前缀 21 个 |
| ≥5 integration tests | ✅ 8 完成 | integration_r10_ 前缀 8 个 |

---

## 8. 后续 round 建议

### 8.1 未做 (超出 round10-01 范围)

- ❌ SandboxVerdict 不携带详细错误上下文 (可加 `Reject { reason: String, sandbox_id: String }`)
- ❌ 真实 sandbox 隔离实现 (WASM / 进程隔离 / 文件权限隔离) — 仅有 trait 接口
- ❌ 反向状态机的"执行" (rollback reverse path 仅为元数据, 未实际撤销各阶段副作用)
- ❌ 反向状态机持久化 (rollback 路径应写入历史流)

### 8.2 round11+ 升级路径 (Ponytail lazy 提示)

- **P1**: `OtaState::Sandboxed` 的 verdict 增加详细字段 (`SandboxId`, `Timestamp`, `Metrics`)
- **P2**: 反向状态机驱动实际撤销 (取消 carrier 注册, 清除 multisig 记录)
- **P3**: WASM sandbox 默认实现 (替代 DefaultSandbox 的字符串检查)
- **P4**: 真实 7 阶段历史流 + 审计导出

升级触发条件: V21 主流程真实运行 OTA 升级时 (当前仅单元测试 + demo).

---

## 9. 提交与状态

### 9.1 git 状态
```bash
git status
# On branch rebase/d7d8-into-integration
# Changes not staged for commit:
#   modified:   crates/apeireth-upgrade/src/ota.rs
#   modified:   crates/apeireth-upgrade/src/lib.rs
#   modified:   crates/apeireth-upgrade/examples/upgrade_demo.rs
#   modified:   crates/apeireth-upgrade/tests/integration_7_stages.rs
# Untracked files:
#   reports/round10-01-upgrade-7-stage-complete-implementation-architect2.md
#   crates/apeireth-upgrade/tests/integration_round10_sandbox_rollback.rs
```

### 9.2 提交建议
```bash
git add crates/apeireth-upgrade/ reports/round10-01-upgrade-7-stage-complete-implementation-architect2.md
git commit -m "round10-01: apeireth-upgrade 7 阶段 Sandbox 升级 + 反向状态机 (architect2)"
```

---

**Architect2 报告完结**. 等待 Leader 评审与下一轮派活.