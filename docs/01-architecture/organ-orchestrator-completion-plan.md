# A 块 OrganOrchestrator 完整化方案 (主代理 Mavis 拍板, 2026-08-28)

> **本文档定位**: A 块真实施 1-3 周估的 5 缺口分阶段方案. 每阶段独立 commit + 跑 5 重守门.
> **HEAD**: `2a99332c` (v2.0.0-rc.1 收盘交付, 1726 tests / 0 clippy)
> **关系文档**: `organ-orchestrator-spec.md` (R11) + `orchestrator.rs` (R12 真实施) +
> `cognitive-module-wiring.md` (12 slot ledger) + `TO-NEW-TEAM.md` §4 A 块 (估时 1-3 周).

---

## 0. 现状真账 (主代理亲验, 2026-08-28 调研)

| 维度 | 实测 |
|---|---|
| R12 orchestrator.rs | 1525 行 (9 organ handle + 13 InitiativeGate + 5 PolicyStage forward-declared + 6 步 tick + L0-L5 enum) |
| 3 integration tests | 1726 passed / 0 failed / 12 ignored (per `cargo test --workspace --locked`) |
| clippy | 0 警告 / 0 错误 (per `cargo clippy --workspace --all-targets --locked -- -D warnings`) |
| 5 重守门 | 0 触碰 LOCKED 5 项 (per R12 注释 + 子代理 J 复核) |

---

## 1. 5 缺口 + 接入路径 (主代理调研结论)

### 缺口 A — `check_8_gates()` 缺后 3 重 (RhythmUnknown/RhythmVeto/DriveLow)

- **现状**: `check_8_gates()` 对后 3 重返 `None` 占位 (orchestrator.rs:880-888); `chain_9_organs()` 完整 discard E7 输出 (orchestrator.rs:964 `let _chain = ...`).
- **真生产路径**: E7 `EmergenceOrgan::last_hold()` 已存在 (emergence.rs:827-833), 返 `Option<InitiativeGate>`. 但 Orchestrator 拿的是 `Arc<dyn OrganTrait>`, 调不到 last_hold().
- **方案**: 移动 `InitiativeGate` (organ crate emergence.rs:423-442) 到 `plugin crate` 作 canonical 13-variant → 在 `OrganOutput::Emergence` 加 `gate: Option<InitiativeGate>` 字段 → EmergenceOrgan `process()` 写入 gate → Orchestrator `check_8_gates()` 从 `outputs.e7.gate` 提取.

### 缺口 B — tick 步骤 3 情绪调制 (EmotionLow gate)

- **现状**: `let _ = self.loop_config.mood_floor;` 占位 (orchestrator.rs:986).
- **真生产路径**: F1 `EmotionOrgan::process()` 返 `OrganOutput::Emotion { pleasure, arousal, dominance, trend }`. mood = (pleasure + 1.0) / 2.0 (per v1 organs.rs:109). 若 mood < mood_floor → EmotionLow.
- **方案**: tick 步骤 3 改为: 检查 `outputs.f1`, 若 `Emotion` variant → 算 mood → 比 mood_floor; 若 `NotImplemented` → 跳过 (0 装诚实).

### 缺口 C — tick 步骤 4 Council `decide_with_invoker`

- **现状**: `Council::decide()` 返 `CouncilVerdict` (legacy 3-variant), orchestrator.rs:899. **已含** 60s timeout 内部 (per lib.rs:386-426), 但**无** failure category + side_call_count + timed_out 信息.
- **真生产路径**: `Council::decide_with_invoker(proposal, &dyn CouncilInvoker)` (lib.rs:434) 返 `CouncilResult` 含 typed `CouncilDecision` (Continue/Retry/Stop/DeferToHuman) + per-advisor failure + side_call_count + timed_out.
- **方案**: Orchestrator.new() 加 `Arc<dyn CouncilInvoker>` 参数; tick 步骤 4 改调 `decide_with_invoker`; 把 `CouncilDecision::Stop`/`DeferToHuman` 翻译为 `OrganOrchestratorGate::CouncilVeto`; `Continue`/`Retry` 通过. 集成测试用 `CouncilInvoker` mock 返 `AdvisorVerdict::allow`.

### 缺口 D — `ratify_fresh_policy()` 完整 5 状态链

- **现状**: `policy_stage = Active` 单步跳 (orchestrator.rs:915-918), **不走** v1 `organs.rs:73-84` 的 4 transition 调用.
- **真生产路径**: v1 走 4 transition: Idle→Draft→Proposed→Ratified→Active. v2 用 `transition_policy()` 4 次, 每次返回 `Result<(), ()>`.
- **方案**: 改 `ratify_fresh_policy()` 为 4 次 `transition_policy()` 调用, 记录每步结果到 `Vec<(PolicyStage, Result<(), ()>)>`. 现有 `PolicyStage::allowed_next()` 仍返单 next, 不破坏.

### 缺口 E — L0-L5 `UpgradeCycle` driver

- **现状**: 仅 enum `UpgradeLayer` (orchestrator.rs:1214-1252) + `CycleStep` (orchestrator.rs:1254-1267), 无 driver. 注释 "本 R12 spec 仅定义骨架, 真实施 1-3 周估待".
- **真生产路径**:
  - L0: governance crate `GovernanceHook::evaluate()` (lib.rs:229-252, 已实接)
  - L1: cognitive.self_assessment slot (per `production.rs:42` + `cognitive.rs:40 SELF_ASSESSMENT_MODULE_ID`)
  - L2: `Council::with_factory(...)` + `decide_with_invoker` (per lib.rs:355-358 + lib.rs:434)
  - L3: Orchestrator `chain_9_organs()` (orchestrator.rs:765)
  - L4: governance `GovernancePipeline` (lib.rs:347-379, AllowAll/DenyCapabilities/MaxRounds 5 重 hook)
  - L5: **不自动跑**, 返建议 (0 装诚实: 不假装"git tag 已自动执行")
- **方案**: struct `UpgradeCycle` 持 `Arc<dyn GovernanceHook>` (L0+L4) + `Arc<dyn SelfAssessmentStore>` (L1) + `Arc<Council>` + `Arc<dyn CouncilInvoker>` (L2) + `Arc<OrganOrchestrator<...>>` (L3). `run_full_cycle(proposal)` 6 步顺序跑, 返 `UpgradeCycleResult { layer_outcomes: Vec<(UpgradeLayer, CycleStep)>, tag_suggestion: Option<String> }`.

---

## 2. 分阶段实施计划 (5 stage × 1 commit each, 5 重守门跑通)

| Stage | 内容 | 估时 | 风险 | 涉及文件 |
|---|---|---|---|---|
| **1** | **缺口 D** — ratify_fresh_policy() 4 transition 链 | 1h | 0 (纯本地状态机) | orchestrator.rs |
| **2** | **缺口 B** — tick 步骤 3 F1 PAD mood 真路径 | 1-2h | 低 (orchestrator 内部 + mock 处理) | orchestrator.rs + tests |
| **3** | **缺口 A** — InitiativeGate → plugin + OrganOutput::Emergence.gate + Orchestrator 读取 | 3-4h | 中 (改 trait enum + 9 organ + orchestrator) | plugin/src/organ.rs + organ/src/emergence.rs + orchestrator.rs |
| **4** | **缺口 C** — CouncilInvoker 参数 + decide_with_invoker + MockCouncilInvoker 测试 | 4-6h | 中-高 (改 Orchestrator.new() 是 breaking change) | orchestrator.rs + tests |
| **5** | **缺口 E** — UpgradeCycle driver (L0-L5 串联) | 6-8h | 中 (跨 crate: governance + cognitive + orchestrator) | orchestrator.rs + 新文件 upgrade_cycle.rs |

**总估**: 16-21h (~3-4 工作日), 短于 A 块估时 1-3 周的下限.

**0 触碰 LOCKED 5 项** (每 stage 验证):
1. 9 哲学锚本体 (`crates/foundation/core/src/eight_anchors.rs:58-79`) 0 改
2. 13 键 (`crates/foundation/core/src/philosophy.rs:142`) 0 改
3. 3 项不可变脊柱 (`crates/foundation/core/src/onion.rs:249`) 0 改
4. workspace.version (`Cargo.toml` "1.2.0") 0 改
5. R11 baseline (`crates/foundation/core/src/cognitive.rs` 0.8682/0.8532/0.9063 + `Cargo.lock`) 0 改

**commit message 模板** (per §5.2):
```
refactor(runtime): OrganOrchestrator 完整化 stage N — <一句话>

- 0 装诚实真账: <数字 + 0 触碰 LOCKED 声明>
- O-6 三阶审查:
  - 总体最优: <与 v2 整体语境对齐>
  - 系统最优: <在子系统依赖图里位置对>
  - 架构最优: <workspace 边界清晰>
```

---

## 3. Stage 1 详细方案 (缺口 D, 最先做)

### 改动

`crates/engine/runtime/src/canonical/orchestrator.rs`:

1. **加 helper struct** (记 4 transition 结果):
```rust
/// ratify_fresh_policy() 走完整 5 状态 transition 链结果 (per v1 `AwakeCompanion::ratify_fresh_policy` 1:1)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RatificationChain {
    pub steps: Vec<(PolicyStage, Result<(), ()>)>,
}

impl RatificationChain {
    /// 是否全部 transition 成功 (走完 Idle → Draft → Proposed → Ratified → Active)
    pub fn all_ok(&self) -> bool {
        self.steps.iter().all(|(_, r)| r.is_ok())
    }
}
```

2. **改 ratify_fresh_policy()** (orchestrator.rs:915-918):
```rust
pub fn ratify_fresh_policy(&mut self) -> Result<RatificationChain, ()> {
    // per v1 organs.rs:73-84 1:1: 4 transition 调用走完整 5 状态链
    self.policy_stage = PolicyStage::Idle; // reset (per *evolution = EvolutionStateMachine::new())
    let mut chain = Vec::with_capacity(4);
    for (target, reason) in [
        (PolicyStage::Draft, PolicyTransitionReason::Start),
        (PolicyStage::Proposed, PolicyTransitionReason::Submit),
        (PolicyStage::Ratified, PolicyTransitionReason::CouncilApprove),
        (PolicyStage::Active, PolicyTransitionReason::Activate),
    ] {
        let r = self.transition_policy(target, reason);
        chain.push((target, r));
        if r.is_err() {
            return Err(());
        }
    }
    Ok(RatificationChain { steps: chain })
}
```

3. **改 Orchestrator.new() 默认 policy_stage** (orchestrator.rs:746):
   - 当前 `policy_stage: PolicyStage::Active` 保留 (per integration test 假设)
   - 注: 默认 Active 是 per v1 AwakeCompanion::new() 注释 "7 强制 advisor 已召集; 主动策略全链路 Idle→Draft→Proposed→Ratified→Active (默认生效)"

### 测试

新增 1 integration test (per `crates/engine/runtime/tests/orchestrator.rs`):
```rust
#[tokio::test]
async fn orchestrator_ratify_fresh_policy_walks_5_state_chain() {
    let mut orch = build_orchestrator();
    // 默认 Active (per ratify_fresh_policy 终点, new() 默认)
    assert_eq!(orch.policy_stage(), PolicyStage::Active);

    // 重置 + 走完整 5 状态链
    let chain = orch.ratify_fresh_policy().expect("ratify should succeed");
    assert_eq!(chain.steps.len(), 4, "4 transitions: Draft→Proposed→Ratified→Active");
    assert!(chain.all_ok());
    assert_eq!(orch.policy_stage(), PolicyStage::Active);

    // 已 Active 后再 ratify_fresh_policy() → 第一步 transition Draft 失败 (Active 终态)
    let result = orch.ratify_fresh_policy();
    assert!(result.is_err(), "Active 后 ratify_fresh_policy 应失败");
}
```

### 既有测试兼容

- `organ_orchestrator_construct_9_organ_8_gate_5_state` (orchestrator.rs:1479) — 不动
- `orchestrator_5_state_machine_transitions` (orchestrator.rs:1316) — 不动, 因为 `transition_policy` 行为不变
- `tick 6 步骤` 集成测试 — 不动, ratify_fresh_policy() 返回类型从 `Result<(), ()>` 变 `Result<RatificationChain, ()>`, 但 tick() 不调 ratify_fresh_policy()

### 改动量预估

- 新增 ~15 行 (RatificationChain struct + impl)
- 修改 ~10 行 (ratify_fresh_policy() body)
- 新增 ~20 行 (新 integration test)
- **总计 ~45 行**, 1 commit, 0 引新外部 dep, 0 触碰 LOCKED

### 5 重守门验证

```bash
# 跑前
git status --short  # clean baseline

# 改后
git diff --stat  # < 100 行 diff
cargo test --workspace --locked  # 期望 1728 passed (1726 + 2 新增)
cargo clippy --workspace --all-targets --locked -- -D warnings  # 0 警告
cargo test --doc --workspace --locked  # 0 FAILED

# LOCKED 5 项 0 触碰验证
grep -n "NINE_ANCHORS_HARDCODE" crates/foundation/core/src/eight_anchors.rs  # 0 行 diff
grep -n "RUNTIME_ENFORCED" crates/foundation/core/src/philosophy.rs  # 0 行 diff
grep "version" Cargo.toml | head -5  # workspace.version = "1.2.0" 0 改
```

---

## 4. Stage 2-5 略 (后续 stage, 本文档持续更新)

每完成 1 stage, commit + 更新本文档 §5 "已完成 stage 真账".

---

## 5. 已完成 stage 真账 (主代理亲验)

| Stage | 缺口 | Commit | Tests | Clippy | 备注 |
|---|---|---|---|---|---|
| 1 | D | `fc159288` (主代理亲做) | 1726 passed / 0 failed | 0 警告 | ✅ done (pushed); + RatificationChain struct + ratify_fresh_policy() 4 transition 走链 |
| 2 | B | `ea9aa14f` (主代理亲做) | 1727 passed / 0 failed (+1 new) | 0 警告 | ✅ done (pushed); + extract_emotion_mood() + tick 步骤 3 真路径 |
| 3 | A | `ed6353f4` (主代理亲做) | 1728 passed / 0 failed (+1 new) | 0 警告 | ✅ done (pushed); + InitiativeGate 移 plugin + OrganOutput::Emergence.gate + extract_e7_gate() |
| 4 | C | `1972b040` (主代理亲做) | 1729 passed / 0 failed (+1 new) | 0 警告 | ✅ done (pushed); + CouncilInvoker 参数 + MockCouncilInvoker + decide_with_invoker |
| 5 | E | `edc98170` (amended from `24d163ff`) | 1739 passed / 0 failed (+10 new) | 0 警告 | ✅ done (pushed); + UpgradeCycle + TagSuggester + 6 步骤 run_full_cycle |

> **O-6 复盘 amend 真账** (主代理被用户提醒 "修" 后): 之前 5 commit O-6 三阶审查 sections 多是描述 WHAT 不是 WHY 最优 vs alternatives, O-6 失守 + O-5 0 装诚实标. 详 `docs/04-internal/A-block-o6-true-account.md`. amend 后 5 commit message 修订版 sections 真回答"为什么最优 vs alternatives + 拒理由", 符合八锚本体 O-6 description "总体/系统/架构三阶审查 + 不做借口清单".

---

## 6. Stage 1 实战教训 (主代理亲验)

1. **`cargo fmt -- file1 file2` 不只格式化指定文件** — 实测会格式化整个 workspace 21 个文件。
   立即 `git checkout HEAD -- crates/...` 回滚非我的改动。本次提交只含 orchestrator.rs + tests/orchestrator.rs。
   **下次 commit 前**: 不再用 `cargo fmt -- file`，改用 `rustfmt file.rs` (单文件格式化)。

2. **嵌套 impl block Rust 不支持** — 第一次编辑把 `impl RatificationChain { ... }` 放在 `impl<RS> OrganOrchestrator<RS> { ... }` 内部，编译错 `implementation is not supported in 'trait's or 'impl's`。修法: 移 RatificationChain struct + impl 到 module-level（在 impl OrganOrchestrator<RS> 闭括号之后）。

3. **R12 内部 0 装诚实标** 准确 — orchestrator.rs:915-918 旧实现 `policy_stage = Active` 单步跳 + 注释"本地 driver 简化"确实是 0 装诚实标缺口。R11 spec §6.3 v1 `AwakeCompanion::ratify_fresh_policy` 真走 4 transition 调用，新实现对齐 v1 1:1。

---

---

## 7. 后续 commit O-6 三阶审查 标准 (per 八锚本体 O-6 description)

每 commit message O-6 三阶审查 sections 必须:

1. **总体最优**: 在更大语境 (release 路线图 / 当前工作量约束 / 上下游依赖) 里, 这个改动是不是最优切入点? **与 alternatives 比较 + 选最优 + 拒理由**.
2. **系统最优**: 在 Apeireth 子系统依赖图 (governance → orchestration → memory → runtime → organ) 里, 改动放在哪一层最合适? **与 alternatives 比较 + 选最优 + 拒理由**.
3. **架构最优**: 在 workspace 16-crate 拓扑 + 单向依赖 + trait object 设计下, 公开 API 形状 + crate 边界 + 0 引新外部 dep, 这个方案是不是最优? **拒的 alternatives + 拒理由**.

不复用 v1 alignment 代替 v2 总体最优. 不描述 WHAT 代替 WHY. 每段需有具体拒的 alternative + 拒理由.

O-6 doctrine (`eight_anchors.rs:83`): "工作量与麻烦不是拒绝重做的理由; 等以后做是借口; alpha 先这样是借口; 派子代理是手段不为目的 (哲学锚本体升级时, 子代理可调研, 主代理必须拍板); 三阶审查 (总体 > 系统 > 架构) 必在 commit message 写明".

---

_本文档 v1 首发 (2026-08-28, 主代理 Mavis 写). Stage 1 启动在即 (缺口 D, 最简单, 0 风险). v2: A 块 5 stage 全部完成 + amend, 后续按 §7 标准走 O-6 三阶审查._