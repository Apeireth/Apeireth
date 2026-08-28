# A 块 5 commit O-6 三阶审查 0 装诚实复盘 (主代理自检, 2026-08-28)

> **何时**: A 块 5 commit (fc159288 / ea9aa14f / ed6353f4 / 1972b040 / 24d163ff) 全部 push 后, 主代理被用户提醒"修"后自检发现 0 装诚实标.
> **目的**: 修正之前 5 commit O-6 三阶审查 sections 的不优情况 (描述 WHAT, 没真正回答 WHY 最优 vs alternatives). O-5 不假装 + O-6 永远追求最优.
> **关系文档**: `organ-orchestrator-completion-plan.md` §5 + §6 (本文件是该 plan 的复盘补充).

---

## 0 装诚实标 (per O-5 不假装)

之前 5 commit O-6 三阶审查 sections 多是"描述我做了什么" (what), **不是** "为什么这是最优 vs alternatives" (why). 例:

- "与 v1 organs.rs:108-114 1:1 翻译" — 这是 v1 alignment, **不是**总体最优. v1 怎么干 ≠ v2 最优.
- "新增 extract_emotion_mood() helper" — 描述 WHAT, 没回答"为什么不直接内联 / 为什么不放 OrganChainOutputs / 为什么不放 F1 organ".
- "check_8_gates 加 chain 参数" — 描述 change, 没讨论"为什么不用 generic / 为什么不用 trait method".

真正 O-6 三阶审查 = 在每个层级比较 candidates + 选最优 + **拒理由**. 之前 5 commit 没做到. **修**.

---

## 修订版 5 commit O-6 三阶审查 sections (per 八锚本体 O-6 description: "总体/系统/架构三阶审查 + 不做借口清单")

### Stage 1 — ratify_fresh_policy() 走完整 5 状态 transition 链 (commit `fc159288`)

- **总体最优**:
  - v2.0 release 估 2027-Q1, 4 块真实施估时 A (1-3 周) < B (4-6 周) < C (6-10 周) < D (2-3 周需硬件). A 块是 B/C 依赖前提, 优先选 A.
  - A 块 5 缺口按依赖排序: D (local) < B (local) < A (跨 trait) < C (跨 dep) < E (跨 crate).
  - 选 D 为 Stage 1: 改动纯 local (orchestrator.rs + tests), 0 引新外部 dep, 0 跨 crate. **拒** A 先做: A 需改 4 文件 (organ/plugin/runtime/tests) + 跨 crate enum 移动, 风险 > D.
- **系统最优**:
  - 改动在 `engine/runtime` crate orchestrator.rs. v1 `AwakeCompanion::ratify_fresh_policy` 1:1 翻译.
  - **拒** 放 orchestration crate: Orchestrator service (orchestration/lib.rs:181) 责任是 workflow service, agent loop 是 runtime 层, 责任错位.
  - **拒** 放 governance crate: governance 是决策层, 不应 own state machine, 违反 workspace 单向依赖 (foundation 不依赖 engine).
- **架构最优**:
  - 公开 API 增量 = 新 `RatificationChain` struct + impl + `pub use` 暴露. 0 引新外部 dep, 0 改 Cargo.toml.
  - **拒** 方案 A: RatificationChain 放 OrganOrchestrator 内部字段 — 失去 telemetry 外部访问, 可观测性失守.
  - **拒** 方案 B: generic parameter `Orchestrator<RS, Chain>` — generic 膨胀传染 build_orchestrator helper (3 处测试), 工程债 > 收益.
  - **拒** 方案 C: 改 transition_policy 返 `Result<PolicyStage, TransitionError>` 加 error enum — 过度工程, 当前 `Result<(), ()>` 已够 (per v1 EvolutionStateMachine 1:1).

### Stage 2 — tick 步骤 3 F1 PAD mood 真实路径 (commit `ea9aa14f`)

- **总体最优**:
  - Stage 1 (缺口 D) 完成后, B (F1 emotion) 是下一缺口. B 改动 local (orchestrator.rs + tests), 0 跨 crate.
  - **拒** A 先做: A (E7 gate) 需改 OrganOutput schema 跨 crate, 风险 > B.
  - **拒** C/E: C (Council) 需 Orchestrator.new 签名变 breaking change; E (L0-L5 cycle) 跨 4 crate 复用, 早期做基础未稳.
- **系统最优**:
  - 改动在 `engine/runtime` crate orchestrator.rs. v1 `organs.rs:108-114` mood_floor 抑制是 AwakeCompanion::tick 第 3 步, 1:1 翻译.
  - **拒** 调 F1 EmotionOrgan 直接拿 mood (跨 OrganTrait 调用): OrganTrait 是单 input/output 契约, 跨 organ 调用破坏 9 organ 独立设计. orchestrator 提是边界清的角色 (串接 9 organ).
- **架构最优**:
  - 公开 API 增量 = `OrganOrchestrator::extract_emotion_mood(&self, chain) -> Option<f64>`.
  - 0 改 OrganTrait, 0 改 F1 EmotionOrgan, 0 跨 crate.
  - **拒** 方案 A: 改 `OrganOutput::Emotion` 加 `mood: Option<f64>` 字段 — plugin trait schema 改 = 9 organ 全受影响, 工程债 > 收益.
  - **拒** 方案 B: emotion 提取放 `chain_9_organs()` 内部, 改 OrganChainOutputs 加 `f1_mood` — chain_9_organs 应保持 9 organ 独立调用语义, 不混入 orchestrator 派生逻辑.

### Stage 3 — check_8_gates() 接 E7 last_hold 真路径 (commit `ed6353f4`)

- **总体最优**:
  - Stage 1-2 完成后, A (E7 gate 接入 check_8_gates) 是下一缺口. 3 候选方案比较:
  - **拒** 方案 A1: Orchestrator 持 `Arc<EmergenceOrgan>` downcast — 破坏 trait 抽象, 0 装诚实失守 (per R12 orchestrator.rs:78-81 0 装诚实标).
  - **拒** 方案 A2: 加 OrganTrait::last_gate() 方法, 9 organ 全实现 (8 返 None) — 9 文件改, 边界污染.
  - **选** 方案 A3: 扩展 OrganOutput::Emergence 加 `gate` 字段, E7 organ 写, Orchestrator 读 — 边界最清, 改动最小, 单向数据流.
- **系统最优**:
  - 改动跨 3 crate (`engine/runtime` + `foundation/plugin` + `engine/organ`).
  - InitiativeGate 移到 `foundation/plugin`: plugin 是 capability trait 边界层, InitiativeGate 是 E7 organ capability 描述, 应与 OrganTrait 同位.
  - `engine/organ` re-export plugin InitiativeGate 是单向依赖 (engine/organ → apeireth_plugin), 不破 workspace 拓扑.
  - **拒** 方案 B1: InitiativeGate 留 engine/organ, Orchestrator 持 Arc<EmergenceOrgan> 拿 last_hold — 边界污染.
  - **拒** 方案 B2: 新 trait `E7GateProvider` 仅 EmergenceOrgan 实现 — 1 schema 字段扩已够, 加 trait 工程债 > 收益.
- **架构最优**:
  - 公开 API 增量 = InitiativeGate enum (13 variant) + 3 helper (ALL_13/is_emergence_gate/as_str) + OrganOutput::Emergence.gate field + extract_e7_gate.
  - 0 引新外部 dep, 0 改 OrganTrait, 0 改 9 organ process() 实现 (除 E7 写 gate).
  - **拒** 方案 C1: OrganTrait::last_gate() default method — 9 organ 全要 impl, 工程债 > 收益.
  - **拒** 方案 C2: `e7_last_gate` 作为 OrganChainOutputs parallel 字段 — e7 已 OrganOutput 序列化, 加 parallel 冗余.

### Stage 4 — tick 步骤 4 Council decide_with_invoker 真路径 (commit `1972b040`)

- **总体最优**:
  - A 块缺口 D/B/A 完成后, C (Council decide_with_invoker) 是下一缺口. 3 候选方案比较:
  - **拒** 方案 C1: 保持 legacy `Council::decide()` — R12 已 0 装诚实标 (orchestrator.rs:898-906 注释), 与 cognitive-module-wiring.md:99 spec 不对齐.
  - **选** 方案 C2: 改 `decide_with_invoker` 加 CouncilInvoker 参数 — 真接 production.
  - **拒** 方案 C3: 加新 method `deliberate_with_module_invoker` 不改 decide_with_invoker — 重复 API, 工程债 > 收益.
- **系统最优**:
  - 改动在 `engine/runtime` crate orchestrator.rs + tests.
  - CouncilInvoker trait 已在 `foundation/orchestration/lib.rs:279` (Council trait adapter pattern, 与 Council trait 设计一致).
  - Orchestrator.new 加 `Arc<dyn CouncilInvoker>` 参数 = **required** (不 Option): Rust 没 default trait object, Option<Arc<dyn ...>> 增加 API 复杂度, required 更优.
  - 真生产路径 = governance composition root 注入 `ModuleInvokerCouncilAdapter` (桥 runtime ModuleInvoker → CouncilInvoker), Stage 4 留口子, Stage 5 实施. 当前 MockCouncilInvoker 仅测试.
  - **拒** 方案 D1: generic parameter `Orchestrator<RS, CI>` 替 Arc<dyn CouncilInvoker> — generic 膨胀 + Rust trait object 已是 0 成本抽象.
  - **拒** 方案 D2: `Orchestrator::with_council_invoker()` builder method — Rust builder 仅 >5 可选参数引入, 当前 16 params 已多, builder over-engineering.
- **架构最优**:
  - 公开 API 增量 = Orchestrator.new 加 1 参数 (16 → 17 params), MockCouncilInvoker + MockCouncilDecision (test helper public 暴露).
  - 是 breaking change (Orchestrator.new 签名改), 但 caller 仅 build_orchestrator 测试 helper 3 处 (统一更新). 真生产路径 0 caller (Stage 5 才接入 composition root).
  - **拒** 方案 E1: 加 Orchestrator::with_council_invoker() builder 不改 new 签名 — 拒, 同 D2 (builder over-engineering).

### Stage 5 — L0-L5 UpgradeCycle driver (commit `24d163ff`)

- **总体最优**:
  - A 块 5 缺口最后一缺口 (缺口 E L0-L5 UpgradeCycle driver), Stage 1-4 完成后才能接 (依赖 Orchestrator.council_deliberate + chain_9_organs).
  - 5 候选 cycle driver 位置比较:
  - **拒** 方案 E1: governance crate — governance 是决策层, 不应 owning 自升级 (cross-cutting concern).
  - **拒** 方案 E2: engine/runtime Orchestrator struct method — Orchestrator 是 tick 层, cycle 是 meta-orchestration, 责任错位.
  - **拒** 方案 E3: engine/orchestration crate — orchestration 已有 Orchestrator service, cycle 是 reflection layer 不是 service.
  - **选** 方案 E4: engine/runtime 新 module `canonical::upgrade_cycle` — runtime 是 composition root owner, 与 Orchestrator 同 module 文件路径.
  - **拒** 方案 E5: Orchestrator::run_upgrade_cycle() method — cycle 不应寄生在 orchestrator struct.
- **系统最优**:
  - UpgradeCycle 持 Orchestrator + governance + SelfAssessmentStore + TagSuggester, 跨 4 crate 复用. runtime 是 composition root, 是 4 dep 最合适 owner.
  - 5 步骤串行 (L0→L5), 任一 Rejected → 早停, 不尝试后续步骤.
  - 不尝试 L5 自动 git tag (0 装诚实: 不假装"已自动 tag").
  - **拒** 方案 F1: 6 步骤并行 — L1 依赖 L0 self_assessment, 串行才能保证 LOCKED 0 触碰.
  - **拒** 方案 F2: tokio::join! 并行非依赖步 (L2 + L4) — 复杂度 > 收益.
- **架构最优**:
  - 公开 API 增量 = UpgradeCycle struct + run_full_cycle() + CycleStep enum + UpgradeCycleResult + TagSuggester trait + DefaultTagSuggester.
  - 新增 2 文件 (upgrade_cycle.rs lib + tests/upgrade_cycle.rs integration).
  - L5 `DefaultTagSuggester` 用 string bump patch, 不调 git — 这是 0 装诚实标: **不假装"已自动 tag"**. 真生产路径 = 主人手跑 git tag + 推 master.
  - **拒** 方案 G1: DefaultTagSuggester 调 `Command::new("git", ["tag"])` — 跨进程副作用, 0 装诚实失守 (Orchestrator 不应 side-effect OS).
  - **拒** 方案 G2: L4 governance pipeline 真接 GovernanceHook chain (3 hook: Permission/PromptInjection/CredentialDisclosure) — Stage 5 简化 = 单 `GovernanceHook::evaluate()`, 完整 pipeline 留 v2.0.0 release 接入, 已在 plan doc §6 标.

---

## 后续 commit O-6 三阶审查 标准 (per 八锚本体 O-6 description)

每 commit message O-6 三阶审查 sections 必须:

1. **总体最优**: 在更大语境 (release 路线图 / 当前工作量约束 / 上下游依赖) 里, 这个改动是不是最优切入点? **与 alternatives 比较 + 选最优 + 拒理由**.
2. **系统最优**: 在 Apeireth 子系统依赖图 (governance → orchestration → memory → runtime → organ) 里, 改动放在哪一层最合适? **与 alternatives 比较 + 选最优 + 拒理由**.
3. **架构最优**: 在 workspace 16-crate 拓扑 + 单向依赖 + trait object 设计下, 公开 API 形状 + crate 边界 + 0 引新外部 dep, 这个方案是不是最优? **拒的 alternatives + 拒理由**.

不复用 v1 alignment 代替 v2 总体最优. 不描述 WHAT 代替 WHY. 每段需有具体拒的 alternative + 拒理由.

不假装 "已追求最优" 但 commit message 没说理由 — 这是 O-5 0 装诚实标 + O-6 违约. 修.

---

## 后续 amend 决策 (待用户拍板)

5 commit history 是 O-6 不优情况. 正确修法 = `git rebase -i HEAD~5` amend 5 commit message + `git push --force-with-lease`. 但:
- Windows PowerShell 非交互环境 amend 5 commits 工程复杂 (git rebase -i 需 interactive editor)
- force push 影响 origin/main 历史, 若有下游 collaborator 需协调
- 替代: 本文件作为 additive 复盘, 修订版三阶审查 sections 已在此, commit history 不改

**待用户拍板**:
- 选项 A: amend 5 commits + force push (历史改, 工程复杂)
- 选项 B: 保留 additive 复盘 (本文件), commit history 不改 (默认)
- 选项 C: 重做 5 commit (新 5 commit 包含修订版三阶审查, 旧 5 commit revert) — 工程债大, **不推荐**

---

_本文档 v1 首发 (2026-08-28, 主代理 Mavis 自检, 0 装诚实标复盘 + 修订版 + 后续标准). 下次更新: 用户拍板 amend vs additive 决策后._