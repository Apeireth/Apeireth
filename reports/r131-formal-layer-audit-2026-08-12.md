# R131.6 Formal Layer Audit (apeireth-formal 覆盖 + 缺失 proof)

> 2026-08-12 R131.6 — `cargo test -p apeireth-formal --lib` → **213 passed / 0 failed**

## 1. 已覆盖不变量 (stage5_2 10 模块)

| 模块 | 覆盖不变量 |
|---|---|
| `borrow_8_id_formal` | 8 个借鉴项目 ID 守门 |
| `cross_module_proof` | 跨模块边界守门 |
| `eight_anchors_formal` | 8 哲学锚 (S-1/S-2/S-3/O-1/O-2/O-3/O-4/O-5) 完整覆盖 |
| `integration_4_commit_formal` | 4 commit 集成守门 |
| `integration_proof` | 综合集成证明 |
| `locked_24_entry_formal` | 24 LOCKED entry 入口签名 0 改守门 |
| `r11_baseline_formal` | R11 baseline 三值守门 |
| `six_gates_v7_formal` | 6 重守门 v7 编译期 hardcode |
| `v05_30dim_formal` | V0.5 30 维评估体系 sum=1.00 守门 |
| `verdict_cache_13keys_formal` | 13 键 verdict cache 编译期守门 |

## 2. 已覆盖不变量 (stage5_3 10 模块, 跨模块集成)

| 模块 | 覆盖不变量 |
|---|---|
| `cross_anchor_integration_proof` | 跨 8 锚交叉 |
| `cross_borrow_integration_proof` | 跨借鉴 ID |
| `cross_commit_integration_proof` | 跨 commit |
| `cross_crate_integration_proof` | 跨 crate 集成 |
| `cross_decision_integration_proof` | 跨决策链路 |
| `cross_gate_integration_proof` | 跨 6 重守门 |
| `cross_locked_integration_proof` | 跨 24 LOCKED entry |
| `cross_push_integration_proof` | 跨推送链路 |
| `cross_stage_integration_proof` | 跨阶段集成 |
| `cross_version_integration_proof` | 跨版本集成 |

## 3. Kani 5 harness (`kani_harness.rs`)

| harness | 验证属性 |
|---|---|
| `kani_verify_backoff_policy_step_within_cap` | `tier_at(idx) ≤ cap` (BACKOFF_MAX_TIERS=8) |
| `kani_verify_jitter_sleep_returns_value_in_range` | `jittered_sleep(..., Full) ∈ [0, cap]` (≤ 600s) |
| `kani_verify_response_cache_capacity_respected` | POD LRU ≤ cap (RESPONSE_CACHE_MAX_CAP=8) |
| `kani_verify_response_replay_lookup_consistent` | POD replay 任意 key lookup 0 panic |
| `kani_verify_role_divide_wrap_unwrap_round_trip` | POD role wrap + parse 闭环 |

## 4. Invariants 模块 (5 模块)

| 模块 | 覆盖不变量 |
|---|---|
| `permission_grant_l0` | L0 权限授予 (人类权威必需) |
| `double_onion_sample` | 双洋葱 6 层配置守门 (l0_requires_ha_invariant) |
| `mid_task_atomicity` | 中任务原子性 |
| `seven_advisor_voting` | 7 advisor 投票一致性 |
| `e_layer_isolation` | E 层隔离 |

## 5. 编译期守门 (Kani-friendly POD 模型)

```
PermissionLayerConfig { kind: u8, requires_ha: bool }
BackoffPolicyPod { tiers_ms: [u32; 8], tier_count: u8, cap_ms: u32 }
ResponseCachePod (cap=8), ResponseReplayPod (cap=8)
Role: u8 (ROLE_COUNT=6)
```

## 6. Critical 缺失 3 个 proof (R132-R133 续填)

### Missing 1: Self-Disable 5 机制 不可绕过 (核心安全)

**为什么 critical**: R131.5 attack test (11 scenarios) 揭示 5 机制核心守门, 但 attack_1/2 暴露:
- no_degrade 严格小写白名单 (case variant 视为未知等级)
- no_bypass 用 `eq_ignore_ascii_case` (case-insensitive, 与 degrade 不一致)
- no_patch 严格小写 (Princial_Keys_Count 不触发)

**应有 formal proof**:
- `kani_verify_self_disable_5_mechanisms_no_bypass`: 任意 owner_token / rule_name / risk_level / trigger_id / window_id 输入, 5 机制至少 1 个触发 OR 正确放行, 0 状态泄漏
- `kani_verify_disarm_rearm_history_immutable`: 反复 disarm/rearm 不能改 records, trigger_id 单调递增

### Missing 2: Perceptual Evidence Guard (守门 9, R131 P1.3)

**为什么 critical**: R131 P1.3 新增守门 9, 对应 S-2 实事求是 + O-5 不假装. 但没 formal proof.

**应有 formal proof**:
- `kani_verify_evidence_guard_5_kinds_complete`: 任意 input + 5 类 EvidenceKind, verify() 返 (Pass / PassInferred / Fail / Missing) 之一, 0 panic
- `kani_verify_nine_fold_guards_hardcode_eq_9`: 编译期 hardcode 守门 9 重 = 6 + 1 + 1 + 1 (B4 6 + Skill + Reflection + Mewg + Evidence)

### Missing 3: apeireth-memory semantic_persist flush_noop 显性 (R131 P0-2)

**为什么 critical**: R131 P0-2 拆分 `save()` 为 `flush_noop()` + deprecated `save()`, 防止"假装 fsync". 但没 formal proof 验证 deprecated 真生效.

**应有 formal proof**:
- `kani_verify_semantic_persist_deprecation_warns`: 调用 deprecated `save()` 必须产生 `#[deprecated]` warning (编译期)
- `kani_verify_flush_noop_does_not_modify_state`: `flush_noop()` 调任意次数, 内部 state 0 变

## 7. R131.6 总结

- 已覆盖: **30 不变量 (10 stage5_2 + 10 stage5_3 + 5 Kani harness + 5 invariants)** + 213 unit test 全过
- 缺失 critical: **3 个 proof** (Self-Disable / Evidence Guard / semantic_persist deprecation)
- 推荐: R132 续填 3 missing proof (预计 1-2 周工作量)

## 跑法

```bash
cargo test -p apeireth-formal --lib  # 213 passed
cargo install --locked kani-verifier && cargo install --locked cargo-kani  # 装 Kani
cargo kani --harness kani_verify_backoff_policy_step_within_cap  # 跑单个
```
