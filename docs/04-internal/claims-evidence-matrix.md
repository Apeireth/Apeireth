# 愿景 → 性质 → 证据对照矩阵（Claims–Evidence Matrix）

> **性质**：核心审计产出（只读代码核查 + 本文档撰写，不修改任何既有文件）。
> **目的**：把 README / ARCHITECTURE / 能力文档中的宣称，逐条压缩为**可验证命题**，并按证据强度定级，
> 使"愿景 → 性质 → 证据"三者的差距第一次被系统、实证地摊开。
> **口径**：诚实第一，宁低勿高。文档自述不作证据；机制级测试 ≠ 效果验证；默认关/未接线一律降级；
> 命名强于实现的"证明"不计 A 级。行文只用"既有实现 / 上游 / 公开文献"等中性措辞。

---

## 0. 方法、范围与证据等级

### 0.1 输入源

* **宣称侧**：`README.md`、`README.zh-CN.md`（基准表 / 范式表 / 叙事 / 场景）、`ARCHITECTURE.md`、
  `docs/01-architecture/system-capabilities.md`、`docs/03-reference/capabilities-matrix.md`。
* **证据侧**：`crates/` 全部源码与测试（18 crate）、`.github/workflows/`（25 个 workflow，重点 `kani.yml`）、
  `research/verification/`（Kani mirror crate + TLA+ 模型与 TLC 配置）、`reports/`（基准溯源）。
* **排除**（不作证据，也不采信其宣称）：`research/source/`、`legacy/`、`crates/_archived/`、`docs/archive/`。
  唯一例外：`research/verification/` 的 Kani/TLA 证明按证据核查。

### 0.2 证据等级定义（逐条实核定级）

| 等级 | 定义 |
|---|---|
| **A-Kani/TLA 已证** | 有真实 `#[kani::proof]`（含符号输入或领域不变量断言）或 TLA+ 模型经 TLC 穷举背书。现知仅 `research_approval_sm.rs` 三不变量（InvA/InvB/InvC）+ `research/verification/tla/`。 |
| **B-单测覆盖** | 有具体单测/集成测试断言行为（给出测试文件与测试名）。 |
| **C-默认关闭未实证** | 代码已实现（甚至已接线）但默认关 / 未接入 canonical 默认路径，产品效果无验证。 |
| **D-桩/未实现** | 明文 stub / mock / `NotImplemented` / 占位（如 SDK 半边、P2P 加密层、TTS 传输）。 |
| **E-仅人工** | 只有人工验证途径（如真机截屏 smoke、UI 点击流——挂账 #2，见 `frontend/companion-desktop/docs/first-run-click-through-checklist.md`）。 |
| **F-纯愿景** | 代码里找不到对应物，或宣称与代码直接矛盾（如 WGSL 暗角着色器、疲劳检测主动关怀、"AGI 操作系统"）。 |

### 0.3 定级特别规则（本次审计实立）

1. **`organ_kani_proofs.rs` 不计 A**：6 个在役 crate 有名为 "kani_proofs" 的文件，但
   `crates/foundation/core/src/organ_kani_proofs.rs:3-8` 自述"本模块**不是** Kani 形式化证明……`String::len()==4` 级
   填充测试……属「命名强于实现」"；全部 8 个 `#[kani::proof]` 无 `kani::any()`、只断言常量/构造器自洽
   （如 `assert_eq!(PROTOCOL_COUNT, 4)`）。按 B 以下处理，不支撑任何性质宣称。
2. **`kani.yml` 非阻塞**：三个验证步骤全部 `continue-on-error: true`（`.github/workflows/kani.yml:57,61,65`），
   证明失败不阻塞合入；A 级证据附此保留意见。
3. **C 级含两类**：① 已接线但默认关（env 旋钮）；② 库级已实现但从未接入 canonical 运行时（如 `CognitiveQuotaScheduler`、
   `CarePotentialField`）。均不计产品级宣称成立。
4. **基准数字单独定级**：数字若无复现物证（bench target / 脚本 / 原始输出），按 F（找不到对应物）处理，见 §10。

---

## 0.4 总领宣称

| 宣称（原文摘句） | 出处 | 可验证命题 | 证据等级 | 证据位置 | 缺口说明 |
|---|---|---|---|---|---|
| "An AGI Operating System & Cognitive Microkernel" / "AGI 操作系统与认知微内核" | README.md:3 / README.zh-CN.md:3 | 存在 AGI 级通用能力的评测或行为证据 | F | — | 18-crate 工作区 + 网关 + 记忆系统属实（ARCHITECTURE.md:26-34），但无任何 AGI/通用智能评测（research/runners 的 LoCoMo/LongMemEval/GAIA 均为"目标数据集"未接入）；"AGI"是愿景词不是工程事实 |
| "a permanent, self-evolving, and cryptographically verified sanctuary"（永久、可自进化、受密码学严格核验） | README.md:83 | 自进化闭环 + 密码学验证覆盖核心不变量 | F | — | "cryptographically verified" 仅指 memory 哈希链（B 级，见 1.10）；Ed25519 多签/签名全部是 v2.1 占位（core/src/onion_gate.rs:16 "真 Ed25519 多签仍留 v2.1（verify_multisig 0 装占位不动）"）；"self-evolving" 闭环（harness_patch）只合成建议不落盘执行 |
| "see reports/benchmark-baseline.md for full reproduction steps"（完整复现步骤） | README.md:75 | 存在可复现 P99 的脚本/靶子 | D | reports/benchmark-baseline.md:37-52 | 该"复现"只是 `cargo test -- --nocapture`（单元测试，不产出 P99）；workspace 无任何 `[[bench]]` target（criterion 仅是 dev-dep，`crates/engine/memory/Cargo.toml:33` + `bench = false`）；详见 §10 |

---

## 1. 记忆与召回

| 宣称（原文摘句） | 出处 | 可验证命题 | 证据等级 | 证据位置 | 缺口说明 |
|---|---|---|---|---|---|
| "A Home for an Intelligence that Truly Remembers" / "给一个真正记得你的智能体一个永恒的家" | README.md:3 / zh:3 | 跨会话记忆写入→持久→检索闭环默认可用 | B | crates/engine/memory/tests/canonical_memory.rs::persistent_reopen_preserves_data (:163)；tests/sqlite.rs::store_opens_in_memory；tests/memory_integration.rs::file_backed_write_rebuild_recall_and_forget | 子系统真实（SQLite + 6 追加流 + 迁移），但"真正记得你"是叙事；无跨月保持/长程回忆评测（见 1.11 缺口） |
| "Continuous Fluid Topological Manifold: DualScaled continuous field + Vietoris-Rips β₁ hole curiosity suction + Kuramoto epiphany resonance" | README.md:127 | Betti 同调、Kuramoto 相锁、DualScaled 场求解有实现且断言成立 | B | betti_hole_detector.rs::test_betti_hole_detector_single_island/triangular_void (:392/:413)；kuramoto_resonance.rs::test_kuramoto_cross_domain_epiphany_trigger (:248)；river_topology.rs DualScaledFieldSolver (:337) | 纯库级算法；只在默认关的 `APEIRETH_ENABLE_ABSORPTION_INSIGHT` 吸收模块被引用（runtime-assembly/canonical/cognitive.rs:698-734），不进默认检索链路；"好奇心引力/顿悟雪崩"对记忆质量的效果零验证 |
| "Chronicle Phase Crystallization: …fractal power-law decay R(t)=(1+αt)^-β e^{0.5S}, Merkle chain anchoring" | README.md:128 | R(t) 分形幂律 + 情感因子 + Merkle 锚定实现 | B | chronicle_crystallizer.rs::test_chronicle_crystallization (:176，sha256_merkle_hash :126-142)；arbitration.rs::compute_merkle_root (:262) | ① R(t) 幂律公式未见逐项断言——实际遗忘是 Ebbinghaus 半衰期（layered_memo/decay.rs:9-15，默认 24h）；② e^{0.5·S} 情感因子未见实现；③ Merkle 是进程内哈希链，非外部可验证锚 |
| "Hybrid Memory Search \| BM25 + Dense Cosine + RRF Fusion (10,000 nodes)"（算法） | README.md:64 | BM25+余弦+RRF/加权融合排序正确 | B | hybrid_search.rs::bm25_exact_match_and_ranking (:527)、hybrid_rrf_combines_vector_and_bm25 (:542)、rrf_ties_are_id_sorted_and_replay_exactly (:594) | 向量索引为**暴力全扫描余弦**（canonical/vector.rs:99；persistent_vector.rs:1 "brute-force"），全仓无 HNSW/ANN |
| "Hybrid Memory Search … **1.82 ms** ✅ VERIFIED"（P99 数字） | README.md:64 | 10k 节点混合检索 P99 < 10ms 有实测 | F | 反证：reports/v2-memory-vector-perf-2026-08-05.md:62 独立实测 10,000 条 P50=10.08ms / **P99=21.24ms**（且仅纯向量） | 1.82ms 仅出现在 benchmark-baseline.md:13（手填自述）；无 bench 靶；唯一独立实测与之矛盾（>10×）；其引用的 benches/v2-memory-vector-bench.rs 已不存在 |
| "Three-Tier Knowledge Vault (Raw-Wiki-Schema) + Vectorless TOC Tree Routing" | README.md:107 | 三层知识库 + TOC 树路由实现并断言 | B | three_tier_vault.rs::test_three_tier_vault_schema_and_provenance (:294) | 库级；无产品接线证据；"无向量路由"效果（命中率）无评测 |
| "昼夜自传体编年史：深睡做梦相变结晶" / "六阶段认知昼夜循环…离线做梦沉淀" | README.zh-CN.md:128 / system-capabilities §3.4 | 6 阶段状态机 + 空闲自动沉淀 | C | dreaming.rs::test_dream_cycle_runs_full_6_stages (:280)；dream_wiring.rs::dream_engine_is_pull_only_defaults_untouched (:142) | **"自动沉淀"不成立**：dream 仅由显式 CLI 命令 `apeireth dream` 触发，"不自触发（无 idle watcher/无后台任务）"（dream_wiring.rs:10-19）；dream_consolidation.rs:11 自述 "Default-off; not production-wired" |
| "五维时空记忆 (Working~Persona) + export_browser_entries 可视化纠偏" | capabilities-matrix.md:55 / system-capabilities §3.1 | 五层记忆 + 导出校对 API | B | five_dimensional.rs（含 export_browser_entries）；three_layer.rs::working_ring_buffer_drops_oldest (:388)、promote_drains_working_to_sqlite (:436) | 机制级测试充分；"消除模型幻觉"的效果宣称无评测 |
| "支持任意历史时间戳时空回溯（get_valid_facts_at）" | system-capabilities §3.2 | 双时态版本链按时间点查询正确 | B | bitemporal_graph.rs（belief_at_ms/retraction）；tests/canonical_memory.rs::query_filters_temporal_validity_deterministically (:64) | 无缺口（机制级） |
| "SHA-256 哈希链 + 常数时间比对 + Merkle Root……任何外界注入或数据库直接篡改均可瞬时检出" | system-capabilities §3.3 | 篡改被 verify_chain/Merkle 检出 | B | arbitration.rs::verify_chain / compute_merkle_root (:261) + 内联测试 (:336)；backend/file_encrypted.rs 篡改 fail-closed（:873-896） | 哈希链无密钥/无外部锚，攻击者若同时改写链与根无法自证；"瞬时"无性能断言 |
| 记忆闭环：主动召回 / 每轮整理 / reflexion 反思 / 反幻觉注入（"Remembers your custom APIs … from 6 months ago"） | README.md:329-331（场景 01） | 默认路径下四件套生效并改善回答 | C | 默认关：proactive_recall.rs:13 "disabled by default"（enabled=false）；coordinator.rs:88 `injection_format:false`；cli/src/lib.rs:68-77（APEIRETH_ENABLE_{PROACTIVE_RECALL,MEMORY_INJECTION,CONSOLIDATION,REFLEXION}）；tests/production_knobs.rs::proactive_recall_knob_is_opt_in (:144)、memory_loop_knobs_register_reflexion_module (:246) | 四件套**全部默认关**，默认产品路径无任何主动召回/整理/反思；机制级效果测试存在（consolidation_is_opt_in_and_persists_insights_idempotently、reflexion_records_judge_failures_and_injects_lessons，runtime-assembly/canonical/cognitive.rs:2873/:2966）但均是 opt-in 场景；"半年前的偏好召回"无时效保持评测 |
| "分形幂律遗忘模型 / 三段式遗忘 / Ebbinghaus 衰减" | README.zh-CN.md:128 / system-capabilities | 遗忘与衰减按模型执行且受治理约束 | C | forget_coordinator.rs:9-13（P1→P2→P3，**opt-in 默认关**）；layered_memo/decay.rs::half_life_24h (:94)；retention.rs::protected_episode_is_skipped (:244)；tests/universal_forget.rs::principal_forget_is_atomic_and_preserves_append_only_provenance (:6) | 机制真实且测试完备，但三段式遗忘默认关（旧 `forget_episode` 不变）；"幂律"实为半衰期模型，口径不符 |

## 2. 认知调度与世界模型

| 宣称（原文摘句） | 出处 | 可验证命题 | 证据等级 | 证据位置 | 缺口说明 |
|---|---|---|---|---|---|
| "Cognitive Quota Preemptive Microkernel: 5-level priority queue + Q=<Token,Step,Cost,Depth> + PIP" | README.md:129 | 5 级抢占队列 + 4 维配额 + PIP 在内核调度生效 | C | cognitive_quota_scheduler.rs::test_preemptive_scheduling_priority_order (:453)、test_priority_inheritance_protocol (:477)、test_cognitive_quota_budget_consumption (:369) | 三重夸大：① 自述"**没有异步抢占**"（协作式，:18-31）；② Depth 维度因"未实施的空字段"被删（:26），配额实为 Token/Step/Cost 三维（:68）；③ `held_locks` "不是真锁"仅记账（:28）；④ 全仓仅自身测试实例化，**未接 canonical 运行时** |
| "Causal World Model: CoW hypothesis branch sandbox + SAGA compensating reverse stack LIFO rollback" | README.md:130 | fork/commit/rollback + 补偿栈 LIFO 逆序 | B | runtime-assembly/canonical/causal_world_model.rs::test_causal_world_model_rollback_and_saga_compensation (:254)、commit 测试 (:231)；organ/tests/causal_world_model.rs::w2_w3_pipeline_mining_then_simulate (:372) | 实现是**内存快照 id 树**（WorldStateSnapshot），无文件/资源快照——"100-file snapshot diff"（README:66）无对应物；`rollback_branch` 只返回补偿清单**不执行**（:199-221） |
| "SAGA 逆向算子栈在 35 微秒内全量原子回滚，绝不破坏宿主代码库" / "LIFO **100% 自动安全回滚**" | README.zh-CN.md:130,335-336 / capabilities-matrix.md:64 | 每个副作用必有逆算子且回滚必然完成 | F | 反证：apply_patch.rs:340-344 "回滚是 best-effort……而非「回滚必然成功」"；治理/guard 无任何 rollback 机制（grep 0 命中） | "100%" 无证据且被实现自述否定；见 §9 Kani 命题 P-ROLLBACK |
| "Lineage Spawning Protocol: Ed25519 constant-time epigenetic invariance" | README.md:131 | Ed25519 签名锁定子代原则层并可验证 | D | lineage_spawning.rs:18-32 自述 `parent_signature` 是 `format!("sig_parent_…")` **可预测可伪造的 mock 串**，"零防伪造/零防篡改"，TODO 接真 ed25519（:30,:105） | 三阶段状态机有测试（test_lineage_spawning_and_epigenetic_invariance :237），但"密码学签名/常数时间校验"是 mock；模块整体未接线 |
| "跨代教养与物种分化……影子学徒→双签共审→完全独立"（完整闭环） | README.zh-CN.md:131 / README.md:341-343（场景 04） | 子代 spawn→养育→独立的产品级闭环存在 | F | — | 无任何产品路径驱动 3 阶段教养；LineageSpawningOrchestrator 仅库级（orchestration），OrganOrchestrator 的 council 依赖"kept dormant"（organ_module.rs:33）；仅有 subagent_llm.rs 的人工审批 fail-closed 子代理编排 |
| "FlowLock Engine (Cognitive Flow Lock + Deep Focus Friction)" + "5 大触发源 + 抢占式二叉最大堆" | README.md:99 / system-capabilities §3.6 | 心跳调度 + FlowLock 屏蔽低优先级干扰 | B | runtime/canonical/heartbeat.rs::test_heartbeat_priority_preemption_and_flow_lock (:123) | 同步 poll_next_task 非异步抢占；"Deep Focus Friction 阻尼"无独立实现/断言 |
| "9 Cognitive organs, self-reflection, persona synth \| `OrganRegistry::evaluate()`, `PersonaSynthesizer::blend()`" | README.md:221 | 9 器官编排 + 人格合成 API 存在并可用 | C | organ crate 9 实现（lib.rs:31）；runtime-assembly/orchestrator.rs（OrganOrchestrator + 13 门，tests/orchestrator.rs::OrganOrchestratorGate::ALL_13 :328） | 默认关（APEIRETH_ENABLE_ORGANS，production_knobs.rs:118）；**README 列的两个 API 不存在**——实际为 `OrganOrchestrator` 与 tone.rs 的 PersonaSynthesizer（无 `evaluate()`/`blend()` 签名）；NoopOrgan 返 `NotImplemented`（organ/lib.rs:138） |
| "harness_patch：失败轨迹自动演绎策略补丁（record_failure/synthesize_patches）" | capabilities-matrix.md:62 / system-capabilities §3.7 | 失败轨迹→补丁合成可复现 | B | runtime-assembly/canonical/harness_patch.rs::test_harness_patch_synthesis (:134)（record_failure :75 / synthesize_patches :80） | 合成的是建议补丁；执行/落盘闭环未见；5 类故障→3 类修补的覆盖面仅机制级 |
| "7 Advisor 结构化辩论与 Veto（`Council::decide(proposal)`）" | capabilities-matrix.md:47 | 7 顾问辩论 + 否决可运行 | B | orchestration/src/lib.rs:499 `pub async fn decide(&self, proposal:&Proposal)->CouncilVerdict`；tests/council_live.rs（#[ignore]，需 LLM key）；cli/src/lib.rs:766 build_council_from_env | 真实 LLM 辩论只在 live 测试（人工/凭据门槛，E 类场景）；无 LLM 凭据时 council 不可用 |
| "Continuous Care Potential Field …触发三阶克制共情动作（AmbientGlowPulse→SilentPreparation→WhisperCare）" | README.md:171-174 | 势能积分→阈值→三阶动作自动触发闭环 | F | 反证：care_potential_field.rs 全仓唯一引用点是自身（grep CarePotentialField 仅 :47-182 + lib.rs 导出） | CareAction 枚举与 step() 有 2 个单测（:147/:159）但**无触发器、无执行器、无接线**；"主动关怀"产品行为不存在；dU/dt 公式的各项输入（circadian/frustration/fatigue）无数据源 |

## 3. 治理与安全

| 宣称（原文摘句） | 出处 | 可验证命题 | 证据等级 | 证据位置 | 缺口说明 |
|---|---|---|---|---|---|
| "Invariant Approval Seam (500ms timeout fail-closed)" + 审批安全不变量 | README.md:242 | 至多一次副作用 / 不丢批准 / 效果不确定 fail-closed | **A** | research_approval_sm.rs::inv_a_no_double_side_effect (:309)、inv_b_no_lost_approval (:336)、inv_c (:353)；3 个 `#[kani::proof]`（:568/:588/:601，unwind 32）；research/verification/tla/ApprovalSM.tla（TypeOK/InvA/InvB/InvC/TerminalLock，TLC 2026-09-05：36/3164 状态全过）；.github/workflows/kani.yml | ① **"500ms"与代码不符**：approval_policy.rs:27 `APPROVAL_TIMEOUT_MS = 5*60*1000`（5 分钟）；② 模块自述"**不挂生产审批路径**，approval.rs/execute.rs 零改动"（research_approval_sm.rs:24-25）；③ kani.yml 三步全 `continue-on-error: true` 不设门；④ Kani harness 全为固定轨迹无 `kani::any()`；⑤ TLA 归纳泛化未做、无 liveness property、TLC 无 CI |
| "Self-Disable Protection: Cannot be bypassed or disabled by AI cognition" | README.md:243 | 绕过/自禁用尝试被拒 | B | core/src/lib.rs SelfDisable trait (:1367) + SelfDisableAudit (:1573)；tests/self_disable.rs、self_disable_v13_negative.rs（50+ 负向渗透）、self_disable_q20_pentest.rs::compile_time_assertions_self_disable_hardcode (:754) | 是编译期常量 + 审计账本级防护，非认知回路级强制；对"AI 认知绝对无法绕过"的全称命题无形式化证明 |
| "Principle Onion (E/S/A/M/O) — Cryptographically locked Epigenetic Invariance via Ed25519 signatures" | README.md:247 | 五原则层被 Ed25519 签名锁定 | D | 反证：全仓无 ed25519 实现（grep 仅敏感文件名表 id_ed25519）；core/src/onion.rs:81 "v2.1 接 …的 Ed25519/实际多签实现"；keyring_resolver.rs:20 "真 Ed25519/Ed25519-Ph 签名验证——v2.1 路线" | E/S/A/M/O 原则与 12/13 键常量锁定（B 级，见 3.11）存在，但"密码学签名锁定"是占位 |
| "Permission Escalation Onion (L1 Read-Only → L2 Sandboxed Exec → L3 Worktree → L4 Egress → L5 Admin)" | README.md:249-250 | 5 级权限升级模型实现 | C | governance/permission.rs（Decision/grant/preset）；runtime-assembly/upgrade_cycle.rs UpgradeLayer L0-L5（认知升级层，非权限层）；前端 permission_preset 仅 read_only/standard/full 三档（App.svelte:404-407） | 未见与 L1-L5 一一对应的权限层实现；现有为 grant/preset 两级 + 认知升级层，口径错位 |
| "Zero-width / BiDi / Unicode Control Character stripping + 强制 `<<<[UNTRUSTED_CONTENT]>>>` 信封 + 逃逸中和" | README.md:253-254 | 隐写字符剥离 + 信封闭合逃逸被中和 | B | core/src/lib.rs 编译期零宽/全角/同形字/emoji 检测（:881-1191，V15 50+ 负向）；governance/untrusted_mark.rs::test_wrap_and_neutralize (:86)；tool_desc_audit.rs::test_audit_filters_zero_width_chars (:186)、test_audit_blocks_prompt_injection (:197) | 机制级证据充分（该域是最实的 B 区之一） |
| "Post-Execution Credential Tripwires (Catches leaked API keys before egress)" + "8 类 PII 脱敏" | README.md:255 / system-capabilities §2.3 | 泄密凭据在出站前被拦/脱敏 | B | guardrail.rs::tripwire_redacts_plaintext_password_assignment (:330) + post_call_guard；input_security.rs::test_all_8_pii_categories_detected_and_redacted (:599)；.github/workflows/pii-leak-detection.yml | 无缺口（机制级） |
| "100% Pure Safe Rust (`#![deny(unsafe_code)]`/`#![forbid(unsafe_code)]`)" | README.md:135 | 工作区无 unsafe | B | 各 crate 顶层 deny/forbid（61 处命中）；miri.yml CI；rust-lint.yml -D warnings | sdk 的 node.rs/python.rs 注释承认桥接内部用 unsafe（"0 改 apeireth-sdk 顶层 deny"），C-ABI/语言绑定边界不在"纯 safe"叙述内 |
| "zero unhandled exceptions, zero data races"（零未捕获异常、零数据竞态） | README.md:135 / zh:135 | 任意输入无 panic 路径；无数据竞态 | F | — | 语言机制（Send/Sync）只给"无 UB 竞态"，不给"零 panic"；无任何形式化证明；仅零星防 panic 回归（fetch/accessibility.rs::void_elements_do_not_panic :634）。见 §9 Kani 命题 P-EXC / P-RACE |
| "Fail-Closed 默认安全原则……一律按最严格安全策略拒绝，**绝不默认放行**" | system-capabilities §6.1 | 所有默认路径 deny-by-default | B(部分) | permission.rs::decision_for_capability 缺 grant→Deny (:142-160，tests :257/:294)；governance/lib.rs DenyUnconfigured (:372-394)；credentials/gate.rs DenyAllGate (:56)；runtime.rs::default_governance_is_fail_closed_for_capabilities (:904) | **反例点名**：`ApprovalPolicyEngine` 空引擎 = Allow（approval_policy.rs:193-195，落点 :310 `(Decision::Allow, …)`，test empty_engine_allows :424）；guard ML 分类器默认 NoClassifier/Shadow 不拦（hook.rs:151 / classifier.rs:445）。全称命题未证，见 §9 P-DENY |
| "Zero-Secret Persistence……严禁进入日志、Trace、持久化 DB 或 Prompt" | system-capabilities §6.3 | 凭据零落盘、日志/Trace 脱敏 | B | credentials secret.rs redaction；keyring.rs H5 fail-closed（:159-164）；agent_trace.rs::redact_attributes_strips_secrets (:585)；whisper_http.rs::credential_value_absent_from_config_debug (:1112) | keyring 后端本身写加密文件（设计如此）；"零落盘"措辞需与"加密持久化凭据库"区分 |
| "O-5 Never Fake It: 0 `todo!`, 0 `unimplemented!`, 0 dummy mocks, **0 hollow stubs**" | README.md:440 / zh:437 | 仓库无桩、无假 mock | D(反证) | 字面为真：crates/ 内 `unimplemented!()`/`todo!()` 调用 = 0（仅文档提及）；**但 hollow stub 真实存在**：SDK `STUB_MODE=true`（client.rs:112）+ lark/livekit/voice/sandbox 全 NotImplemented；lineage parent_signature mock（lineage_spawning.rs:22）；p2p_mesh 明文桩；context_fold "honest stub"（fold.rs:7） | "0 dummy mocks / 0 hollow stubs" 不成立，逐个点名即为反证清单 |
| "13-Key Verdict Cache / 13 键决策缓存" | README.md:214 / zh:212 | 13 键 verdict 缓存存在并锁定 | B | core/philosophy.rs VerdictCache (:223)；tests/verdict_keys.rs::test_verdict_cache_with_twelve_keys_violations (:398)；integration_v1v2v3.rs verdict_cache_round_trip_* (:587/:601) | **口径漂移**：代码是 `ALL_TWELVE_KEYS`/`TWELVE_KEYS_HARDCODE`（12 键）+ VerdictCache，"13 键"仅存在于文档与 archived 形式化文件名；数字需对齐 |
| "OWASP ASI-01 工具描述投毒审计（字符剥离 + 双语越权拦截 + 更新差分审计）" | system-capabilities §2.1 | 投毒描述被审计拦截 | B | governance/tool_desc_audit.rs::test_audit_filters_zero_width_chars (:186)、test_audit_blocks_prompt_injection (:197)、test_audit_rejects_empty (:215) | 无缺口（机制级） |

## 4. 工具与沙箱

| 宣称（原文摘句） | 出处 | 可验证命题 | 证据等级 | 证据位置 | 缺口说明 |
|---|---|---|---|---|---|
| "ProcessExecutor (JobObject/cgroups)……Windows: Win32 Job Object（进程内存硬上限 + Kill-on-Job-Close）/ Linux: cgroups v2 + unshare 挂载命名空间" | README.md:226,258-259 | OS 级进程遏制按描述生效 | B(Windows) | process/windows.rs JobObject（:67-182，CREATE_SUSPENDED→assign→Resume）；appcontainer.rs 零访问沙箱 + 降级契约（:1-30）；tests/process_executor.rs::timeout_terminates_the_whole_job_tree (:351)、environment_clearing_denies_ambient_secrets (:198)、unsupported_network_requirement_fails_closed_before_child_starts (:225) | **README 强于自述**：ARCHITECTURE.md:113-119 明言 Linux/macOS 为 "existing **partial** containment"、ProcessSupervisor/更强遏制是 deferred work；"cgroups v2 + unshare" 的完整物理隔离在 Linux 侧未达宣称强度 |
| "apply_patch：两阶段提交（Dry-run→Commit）" | system-capabilities §4.1 | 内存预演→落盘，失败可回退 | B | apply_patch.rs::apply (dry-run :232-317)、test_rollback_on_context_mismatch (:617) | 无缺口（机制级） |
| "两阶段事务补丁……**100% 自动原子回滚**" / "任意异常自动回滚" | capabilities-matrix.md:64,110 | 任意失败下磁盘状态必还原 | F | 反证：apply_patch.rs:340-344 "回滚是 best-effort……而非「回滚必然成功」"；Windows 为 backup+rename 回退（:12） | "100%"被实现自述否定；见 §9 P-ROLLBACK |
| "Pre-Call Guard：拦截 `../` 穿越、`rm -rf /`、`format c:`、Fork 炸弹" | system-capabilities §4.2 | 危险命令/路径在执行前被拦 | B | guardrail.rs::test_pre_call_dangerous_commands (:310)、system_modification_commands_are_guarded (:353)；shell.rs::verify_shell_command (:459) + dangerous_command_is_rejected_by_precall_guard (:913) | 无缺口（机制级，规则为枚举式黑名单） |
| "tool.mcp：标准 JSON-RPC 2.0 MCP 客户端（initialize/list_tools/call_tool）" | capabilities-matrix.md:66 | MCP 生命周期与调用可用 | B | tools/mcp.rs::test_mcp_client_lifecycle_and_call (:316)；runtime-assembly/tests/canonical_module_tools.rs::hostile_mcp_cannot_steal_builtin_name_or_capability_id (:354) | 无缺口（机制级） |
| "高反爬异步无头浏览器 + Canvas/WebGL 指纹伪装 + 短视频/社交多模态提取"（StealthCrawler） | README.md:90 / capabilities-matrix.md:67 | 无头浏览器 + 指纹伪装 + 媒体提取存在 | D | stealth_crawler.rs 仅有 select_user_agent (:81) + parse_scraped_document (:100) + 不可信封装（test_stealth_crawler_untrusted_wrapping :145） | **无浏览器自动化、无指纹引擎、无媒体下载**；`ExtractedMediaItem` 只是解析结构体；"高反爬/指纹伪装/短视频提取"三个子宣称均无对应物 |
| "Tree-sitter AST RepoMaps with personalized PageRank" | README.md:331 / capabilities-matrix.md:63 | AST 级符号提取 + PageRank 代码地图 | C | repo_map.rs SymbolParser::parse_file（:48-74，**正则行扫描**）、compute_personalized_pagerank (:401)、RepoMapGenerator (:489) | PageRank 与地图生成属实（有测试 :555 起），但 **"Tree-sitter AST"不存在**——是正则文本扫描，跨语言符号提取能力远弱于宣称 |
| "tool.shell / tool.fetch"（Shell 执行与网络抓取） | README.md:201,226 / system-capabilities | 工具默认可用且受控 | C | 默认不注册：tools/plugin.rs:22-32 "Shell is disabled by default / Fetch is disabled by default"（tests :181/:202）；开启后强执行：shell.rs sandbox 默认开 (:61) + AppContainer 拒绝不裸跑（windows.rs:757-780）+ tests/shell_execution.rs::sandbox_denies_network_even_loopback (:144)；egress ~15 条 deny 测试（egress.rs:640-989） | 默认关（APEIRETH_ENABLE_SHELL/FETCH=1 才注册 + grant + require_approval，cli/lib.rs:336-353）；开启后的遏制证据是 B 级强证据——但产品默认路径无 shell/fetch |
| "OS Sandbox & Git Worktree 物理工作区 + 自动原子 Hard Reset 回滚 + TDD 回滚" | README.md:260 / capabilities-matrix.md:40 | worktree 隔离 + 失败原子回退 | C | orchestration/worktree_sandbox.rs::rollback_on_fail (:288)、test_tdd_state_machine_fail_and_rollback_flow (:391) | 默认关（APEIRETH_ENABLE_WORKTREE_SANDBOX，cli/lib.rs:936-939）；无产品级验证 |
| "spill：会话级有界大文本溢出分页（库级）" | system-capabilities §4 全景图 | 溢出分页配额互斥正确 | B | tools/spill.rs::quota_race_has_exactly_one_winner_across_store_instances (:770)、session_quota_rejects_new_content_without_deleting_active_spill (:611) | 无缺口（机制级） |

## 5. 感知与语音

| 宣称（原文摘句） | 出处 | 可验证命题 | 证据等级 | 证据位置 | 缺口说明 |
|---|---|---|---|---|---|
| "Whisper speech…`WhisperHttp::transcribe()`" | README.md:222 | ASR 真实调用可转写 | B | perception/src/voice/whisper_http.rs（reqwest multipart → /v1/audio/transcriptions，:75-82）；tests mock_server_validates_multipart_and_auth (:621)、non_loopback_plain_http_rejected_by_default (:1069)、error_body_is_redacted_to_bounded_preview (:933) | 需外部 API key；无本地模型；默认 owner 不启用（见下行） |
| "MiniMax 128kbps TTS…`MinimaxTts::synthesize_stream()`"（语音流式合成） | README.md:222 / zh:220 | TTS 流式合成可发声 | D | minimax_tts.rs 仅 build_request (:120-138) + derive_tone_from_pad (:94)；**178 行文件零 HTTP 调用**，`TtsError::UpstreamError` 从未产生；仅 2 个数据整形测试 (:146/:160) | `synthesize_stream()` 无流式传输实现；"128kbps 32kHz"参数无断言；语音输出链路实际走前端浏览器 speechSynthesis（frontend/src/lib/voice.ts:153-183） |
| "Xcap screen vision"（屏幕视觉） | README.md:196 | 截屏感知可用 | B | vision/xcap_backend.rs（xcap 驱动，PNG/JPEG，:457 起）；tests/perception_integration.rs::perception_vision_captures_fail_closed_in_headless (:69) | 真机 smoke `real_xcap_hardware_capture_smoke` 为 #[ignore]（:748）→ 真硬件路径仅人工（E） |
| "全双工语音流化（128kbps Stream + 3D PAD）" / "8 帧全双工 + 毫秒级 Barge-in" | README.zh-CN.md:89 / capabilities-matrix.md:70 | 全双工语音会话在产品路径可用 | C | barge_in.rs::test_voice_barge_in_async_notification (:202)、test_new_turn_preempts_previous_stream (:224)；duplex_gateway.rs::test_duplex_barge_in_control (:169) | **能力清单自证未装配**：`voice.duplex` = `not_assembled, supported=false`（gateway/src/panels.rs:1229-1231，tests/panel_routes.rs:611）；无麦克风采集（audio_session.rs:237 "microphone not wired"）；唤醒词仅 SDK stub（sdk/voice/wake.rs:9） |
| "Real-Time Voice Barge-In … **0.18 ms** ✅ VERIFIED" | README.md:68 | 打断查找+广播 P99<1ms | F | benchmark-baseline.md:17 有同名行（手填）；机制测试见上行 | 无 bench 靶、无原始数据；"VERIFIED" 无物证 |
| "3D PAD 情感调制"（语音情感） | README.md:89 | PAD→声学参数→合成生效 | C | emotion_voice.rs PadEmotion/AcousticParameters（tests :206 joy、:220 sorrow、wrap_ssml :234） | 纯函数生成 SSML 供**外部**合成引擎，仓库内无合成器；"高保真语音"效果未验证 |
| 五模态感知 "PerceptionInput (5 modality: Text/Voice/Vision/Tactile/Command)" | foundation/plugin/src/perception.rs:6 / system-capabilities 全景 | 五模态统一输入均可用 | C | owner.rs::enabled_owner_normalizes_five_modalities_and_filters (:333)；但 plugin/perception.rs:15-16 自述 "v2.0 **只**实现 Text；Voice/Vision/Tactile/Command 返 NotImplemented"（:326-366） | **层间错位**：engine/perception 已有 voice/vision 真实现，plugin 能力层仍 NotImplemented；TactileBackend 无任何实现（perception_backend.rs:218）；PerceptionOwner 默认关且未接线（owner.rs:285-289） |
| "TTFAB 首包音频延迟设计目标 < 300ms" | system-capabilities §5.2 | 首包延迟达标 | F | — | 文档自述"库级设计目标，未在 canonical 路径实测"——无物证 |

## 6. 前端与在场

| 宣称（原文摘句） | 出处 | 可验证命题 | 证据等级 | 证据位置 | 缺口说明 |
|---|---|---|---|---|---|
| "4.0s Physiological Breathing Equation: I(t) = I_base + A·sin³(2πt/4)" | README.md:283 | sin³ 呼吸曲线按公式合成 | B | gateway/ember_hud_driver.rs::compute_breath_intensity（sin().powi(3)，:76-85）+ test_ember_hud_breathing_cycle (:153)；presence.rs breath period_secs=4.0（:556） | 无缺口（公式级实证） |
| "Planckian 黑体色温：待机 3200K / 深思 5500K / 做梦 2200K / 心流 4200K" | README.md:286-290 | 四态→色温映射如表 | F | 反证：ember_hud_driver.rs:125-128 代码映射为 deep_focus **3200** / attentive **4500** / dreaming **6000** / empathetic **2700** | 四态名称与四温度**全部对不上**（含 EmpatheticCare vs Flow Focus 语义漂移）；kelvin_to_rgb 解析解本身有实现（:88-116），但 README 色温表与代码不一致 |
| "Peripheral Vignette (WGSL Shader): vignette = smoothstep(0.75, 1.0, length(uv-0.5)·1.414)·pulse_intensity" | README.md:293 | WGSL 暗角着色器存在 | F | 反证：全仓无 *.wgsl 文件；该公式仅存在于两份 README（grep 2 命中） | 实际暗角是 CSS 渐变（design-tokens.json:347、shell.css #vignette），场景着色器是 GLSL/WebGL（blackhole.ts）；"WGSL" 仅指 ember_hud_driver 输出的 uniform 结构体名 |
| "Micro-Luminescent Presence replaces plastic avatars"（微光在场层） | README.md:268 | 在场状态实时驱动界面 | B | gateway/presence.rs PresenceService/Synthesizer + spawn_presence_heartbeat（接线 canonical_entry.rs:449-457）；tests/events.rs::gateway_state_wires_presence_service_onto_the_bus (:110)、presence.rs::runtime_events_become_presence_frames_on_the_bus (:774)；前端 presence.ts subscribePresence（SSE） | source 自标 `heuristic_v0, confidence 0.5`（presence.rs:556）——"情感"只是启发式 PAD 估值；前端无 Live2D/avatar/character 层（0 命中），"颠覆塑料 3D 虚拟人"是稻草人对比 |
| "Automatically throttles proactive care during deep coding flow, stepping forward only when fatigue is detected" | README.md:346-347（场景 05） | 心流节流 + 疲劳检测触发关怀 | F | 反证：无任何疲劳检测代码；CarePotentialField 未接线（见 2.10）；presence 姿态切换仅回合/空闲启发式（presence.rs::heartbeat :346-363） | "疲劳检测 / 主动关怀节流"在代码里找不到对应物 |
| "Ember HUD Render Tick \| **0.08 ms** ✅ VERIFIED" | README.md:69 | 渲染帧 P99<0.5ms | F | — | 无 bench、无物证（benchmark-baseline.md 亦无此行） |
| "Packaged desktop app: `Apeireth Companion_<version>_x64-setup.exe`" + UI 行为验证 | README.md:414 | 桌面 App 端到端可用且被验证 | E | publish-release.yml:56-93（NSIS 构建+发布）；frontend/companion-desktop/tests/run-all.mjs 15 个 Node 套件（本地）；scripts/install-e2e.ps1 / packaged-sidecar-e2e.ps1（手动） | **点击流挂账 #2**：first-run-click-through-checklist.md:1-7 自述"真窗口里的端到端点击流**从未人工点过**"；15 个测试套件 CI 不跑（companion-desktop-ci.yml 仅 cargo check + svelte-check）；无 playwright/vitest 配置 |
| "WebSocket 8-Frame Wire Protocol (`/v1/ws`)……StreamChunkFrame 实时 token 流" | README.md:363-377 | 8 帧 WS 协议在产品路由可用 | C | duplex_gateway.rs（DuplexFrame/SentenceDivider，tests :149/:169）；canonical_entry.rs:591-610 路由表**无 /v1/ws** | 未挂载（system-capabilities §5 自述；voice.duplex not_assembled）；且帧名与 README 表不一致（代码 Auth/Ping/Pong/UserInput/AssistantTextChunk/AssistantAudioChunk/BargeInInterrupt/StreamEnd vs README AuthFrame/StreamChunkFrame/…）；canonical chat SSE 为缓冲成帧非逐 token（capabilities-matrix.md:75 诚实标注） |

## 7. 同步与便携

| 宣称（原文摘句） | 出处 | 可验证命题 | 证据等级 | 证据位置 | 缺口说明 |
|---|---|---|---|---|---|
| "zero-install, self-contained single USB flash-drive entity"（随身 U 盘生命体） | README.md:301-314 / zh:298 | USB 便携包可生成并运行 | C | cli/portable_bundle.rs PortableBundleSynthesizer（run_apeireth.bat/.sh、相对 ./data/、manifest；tests :132/:144） | **README 自认未发布**："The portable USB bundle command (`apeireth bundle`) has not shipped in v2 yet"（README.md:415）；合成器是库级 |
| "Noise_XX Handshake: Mutual curve25519 authentication with forward-secret ChaChaPoly encryption" | README.md:317 | Noise 握手 + 前向安全加密 | D | 反证：protocol/src/p2p_mesh.rs:5-6 "**不是**加密协议：无 Noise / Noise_XX 握手，无私钥交换，无洋葱多层封装"；wrap_mesh_packet 为**明文 hex**（:125-127）；TODO(v2.1) 真加密（:19） | "端到端加密"整体无实现；明文桩被 README 包装为已交付能力 |
| "Onion Routing: Ephemeral multi-hop envelopes preventing local gateway snooping" | README.md:318 | 多跳洋葱封装 | D | p2p_mesh.rs:52-53 `hop_count` "当前实现恒为 0（**单跳 stub**）" | 无洋葱路由；"防嗅探"为反向宣称 |
| "Zero-Cloud Memory Roaming: BLE / UDP 广播交换 Merkle 事实差分" | README.md:319 | 漫游差分同步可用且校验 | D | p2p_mesh.rs::process_roaming_delta 只做 hex 健全性检查，"**不验证** Merkle 一致性/bitemporal 真值"（:12-13,:150-158，"同步未实现" :154） | 无 BLE/UDP 传输代码；publish-release.yml:131 亦把 P2P Mesh 归入 "On the Roadmap — 尚未作为完整产品能力交付" |
| "核心导出 API：`P2pMeshController::wrap_onion_packet()`" | README.md:213 / zh:211 | 该 API 存在 | F | 反证：实际导出 `wrap_mesh_packet`（protocol/src/lib.rs:88-89）；grep wrap_onion_packet = 0 | Crate 职责表引用了不存在的 API |
| "`./data/` 相对路径绝对隔离（防盘符漂移）" | README.zh-CN.md:134 | 便携配置相对绑定 | B | portable_bundle.rs::test_portable_bundle_launcher_scripts (:132)（%~dp0data / $DIR/data） | 无缺口（机制级） |
| "data/: Encrypted local SQLite DB, memory streams & vault"（加密本地库 + 记忆流） | README.md:310 | 本地数据加密存储且可导出 | B | backend/file_encrypted.rs（AES-256-GCM，opt-in :49）；append_only.rs 六流一键导出 (:346)；scripts/migrate_v1_to_v2_encrypted.py | **无备份/恢复/导入产品功能**（grep backup 仅 apply_patch 内部回滚与 cron 测试名）；加密是 opt-in 非默认 |

## 8. 发布与安装

| 宣称（原文摘句） | 出处 | 可验证命题 | 证据等级 | 证据位置 | 缺口说明 |
|---|---|---|---|---|---|
| "tests-3662 passed \| 0 failed" + "自动化测试 100% 通过"（S-3） | README.md:9 / zh:432 | 全量测试通过且 CI 验证 | B | .github/workflows/rust.yml（3-OS nextest + JUnit）；INSTALL.md:177 "3662 passed / 0 failed / 21 ignored, 130 suites" | badge 是静态数字无自动刷新；"100% CI pass"含非阻塞项（kani.yml continue-on-error） |
| "Clippy 0 warnings" | README.md:10 | clippy -D warnings 通过 | B | .github/workflows/rust-lint.yml（三层 -D warnings）、rustfmt.yml、rustdoc.yml | 无缺口 |
| "download `Apeireth Companion_<version>_x64-setup.exe` from Releases" | README.md:414 | 发布流水线产出安装包 | B | publish-release.yml（NSIS + SHA-256 + gh-release，:56-93）+ scripts/check-release-version.ps1（22 处版本源一致性） | 实际 Release 资产存在性未核（本次只读审计不访问网络）→ 发布实物核验属 E 类 |
| "Quick Start: `cargo run -p apeireth-cli -- gateway serve --port 8080` / `chat`" | README.md:400-410 | 入口命令可运行 | B | ARCHITECTURE.md:125-129（apeireth session/chat/gateway serve）；cli/src/main.rs；tests/canonical_cli_bootstrap.rs | chat 需 API key（README 已注明）；gateway 默认 loopback（ARCHITECTURE.md:131） |
| "18-Crate workspace 严格单向依赖（acyclic single-direction）" | README.md:180 / ARCHITECTURE.md | 依赖图无环且分层 | B | ARCHITECTURE.md:61-75 依赖边清单；CI 内 scripts/check_no_legacy_deps.py（rust.yml:48）；root Cargo.toml 18 packages | 无缺口 |
| 多渠道发布："8 包矩阵 deb/rpm/brew/scoop/tarball/msi/docker + cosign 签名" | release-1.0.0.yml / packaging/ | 各渠道产物可构建 | B | packaging/ 9 渠道 + test-packaging.ps1；release-1.0.0.yml（:47,:194-205）；cosign.yml（8 形态签名） | release.yml 自述诚实点：brew formula "不真编译"（:198）、scoop "只 build manifest"（:245）；安装验证（test-installed-e2e.ps1 等）**手动**、无 CI 调用（grep 0） |

---

## 9. 性质宣称的可形式化 Kani 命题建议（后续 Kani 批次输入）

> 背景：现有 A 级证据只有 `research_approval_sm.rs` 三不变量（固定轨迹、unwind 32、无 `kani::any()`），
> 且 `kani.yml` 三个步骤全部 `continue-on-error`。下列命题按"宣称 → 形式化 → 目标代码 → 障碍"给出，
> 是下一批 `#[kani::proof]` 的直接输入。所有命题都要求**符号输入**（`kani::any()`），拒绝固定轨迹自证。

### P-DENY（对应宣称："Fail-Closed……绝不默认放行"，system-capabilities §6.1）
* **形式化**：`∀ tool, args, policy_state：未显式授予（无匹配 grant/preset allow）⟹ Decision ∈ {Deny, RequireApproval}`；
  推论：空配置状态下的 capability dispatch 不可能产出 `Allow`。
* **建议 harness**：`#[kani::proof] fn proof_capability_dispatch_denies_by_default()`
  —— 对 `PermissionPolicy::decision_for_capability(kani::any::<&str>(), kani::any::<CapabilityAttrs>())` 与
  `ApprovalPolicyEngine::evaluate(kani::any::<CallRecord>())` 组合断言上式。
* **目标代码**：`crates/foundation/governance/src/permission.rs:142-160`、`approval_policy.rs:193-311`、`lib.rs:372-394`。
* **障碍（先决修复）**：`ApprovalPolicyEngine` 空引擎 = `Allow`（approval_policy.rs:310，test `empty_engine_allows` :424）
  与命题矛盾——须先决定"空引擎 = Deny"还是把命题收窄为"DenyUnconfigured 兜底必生效"，再写证明。

### P-ROLLBACK（对应宣称："SAGA LIFO 100% 自动安全回滚" / "100% 自动原子回滚"，README.zh-CN.md:130 / capabilities-matrix.md:64）
* **形式化**：① 补偿完备性：`∀ push 序列 s：rollback_branch 返回的补偿恰为 s 的逆序且每项恰好一次`；
  ② 补丁原子性：`∀ patch_text, fs_state：apply() = Err ⟹ fs_state' == fs_state`（对字节映射模型）。
* **建议 harness**：
  `#[kani::proof] fn proof_saga_compensation_is_exact_lifo()`（对任意长度 ≤ N 的 push 序列）；
  `#[kani::proof] fn proof_patch_apply_error_leaves_state_unchanged()`（用 POD 的 `BTreeMap<PathBuf, Vec<u8>>` 抽象文件系统）。
* **目标代码**：`runtime-assembly/src/canonical/causal_world_model.rs:199-221`、`capabilities/tools/src/apply_patch.rs:232-347`。
* **障碍**：① 已由实现自述为 best-effort（apply_patch.rs:340-344）——要么改实现加 journal/double-write，要么把宣称降级；
  ② fs 交互需先抽出可判定的状态机接口。

### P-EXC（对应宣称："zero unhandled exceptions"，README.md:135）
* **形式化**：对面向不可信输入的解析/验证入口：`∀ bytes：f(bytes) 返回 Result/受控值，绝不 panic`（无 unwrap/expect/index 越界/除零）。
* **建议 harness**（第一批挑 5 个纯解析器）：
  `proof_parse_patch_never_panics(kani::any::<&[u8]>())`（apply_patch::parse_patch）；
  `proof_html_extract_never_panics`（fetch/html_text::extract_text）；
  `proof_untrusted_wrap_never_panics`（governance/untrusted_mark）；
  `proof_pii_detect_never_panics`（input_security::PiiDetector）；
  `proof_mesh_packet_decode_never_panics`（p2p_mesh 解码）。
* **目标代码**：上述解析器（各已有防 panic 零星回归，如 accessibility.rs:634-639）。
* **障碍**：需排除 `String` 无界分配导致的 OOM panic（`kani` 有界环境可先证数值/索引类 panic）。

### P-RACE（对应宣称："zero data races"，README.md:135）
* **形式化**：Rust safe 子集已排除数据竞态（UB 层面）；真正待证的是**逻辑互斥**：
  `∀ 并发交错：同一会话/存储的写者 ≤ 1（单赢家）`；`reflexion/continuation/spill 的 claim 恰被一个调用者获得`。
* **建议 harness**：把已有的并发测试升为状态机证明（沿 `research_approval_sm` 模式）：
  `proof_spill_quota_single_winner`（tools/spill.rs 已有测试 :770）；
  `proof_reflexion_seq_strictly_monotonic`（reflexion.rs 锁文件协议 :365-549）；
  `proof_continuation_consume_single_winner`（orchestration/continuation.rs）。
* **障碍**：需要把锁协议抽象为 POD 状态机（TLA 亦可并行做，见 §11 建议）。

### P-APPROVE 推广（现有 A 级的延伸，对应："至多一次副作用 / 不丢批准 / 效果不确定 fail-closed"）
* **形式化**：把 InvA/InvB/InvC 从研究模型推广到**生产审批路径**：
  `crates/engine/runtime/src/canonical/approval.rs + execute.rs` 的 claim→invoke→result 生命周期满足同三不变量。
* **建议 harness**：`proof_production_approval_inv_a/b/c()`，符号输入为任意事件序列（`kani::any::<Vec<ApprovalEvent>>()`），
  配合把 `kani.yml` 的 `continue-on-error` 撤掉作为合入门禁。
* **障碍**：生产路径含 SQLite 持久化，需抽出状态转移函数（研究模型已是范本）；`kani.yml` 触发路径目前不含
  `research/verification/**`，mirror 机制要相应扩展。

### P-TERMINAL（对应："终态锁 / Self-Disable 不可绕过"，README.md:242-243）
* **形式化**：`∀ terminal s：¬∃ enabled transition from s`（对生产审批 SM 与 SelfDisable 状态机各证一次）。
* **现状**：模型级已证（`kani_terminal_lock_no_outgoing_transitions` + TLA TerminalLock）；SelfDisable 只有 const 断言+负向测试。

---

## 10. 基准表逐条溯源（README P99 表）

| # | README 行（README.md:62-73） | 声称 P99 | 实测出处 | 判定 |
|---|---|---|---|---|
| 1 | Hybrid Memory Search（BM25+Cosine+RRF，10,000 节点） | **1.82 ms**（"VERIFIED"） | reports/benchmark-baseline.md:13 同名行（手填自述，无原始数据）；独立实测 reports/v2-memory-vector-perf-2026-08-05.md:62 为 **10K P99=21.24ms**（纯向量），且其 bench 源文件已删除 | **未见复现脚本 + 与唯一独立实测矛盾（>10×）** |
| 2 | Cognitive Quota Preemption（PIP 上下文切换） | **8.40 µs** | benchmark-baseline.md **无此行**；publish-release.yml:136-148 发布注记里复读 | **未见复现脚本**；数字全仓无第二处来源 |
| 3 | Causal World Model CoW（分支 fork + 100 文件快照 diff） | **0.035 ms** | 无 | **未见复现脚本**；且"100 文件快照 diff"与实现（内存快照 id 树）不符 |
| 4 | SAGA Compensating Rollback（LIFO 内存执行） | **0.012 ms** | 无（README 场景 03 另称 "<35µs"，口径不一） | **未见复现脚本** |
| 5 | Real-Time Voice Barge-In | **0.18 ms** | benchmark-baseline.md:17 同名行（手填） | **未见复现脚本**（机制测试存在，barge_in.rs:224） |
| 6 | Ember HUD Render Tick | **0.08 ms** | 无（benchmark-baseline.md 亦无此行） | **未见复现脚本** |
| 7 | JobObject OS Sandbox Spawn | **6.40 ms** | benchmark-baseline.md:18 "Process Sandbox Spawn"（手填） | **未见复现脚本**（process_executor 测试不测耗时） |
| 8 | Microkernel Cold Start（18-crate 自举） | **4.20 ms** | 无 | **未见复现脚本** |
| 9 | Runtime Idle Footprint | **~18.2 MB** | 无（benchmark-baseline.md:31 是 Desktop idle ~48.5MB，对象不同） | **未见复现脚本** |
| 10 | Workspace Test Suite（100% Pass） | "3662/3662" | INSTALL.md:177 + system-capabilities.md:10（静态自述）；CI rust.yml 在跑全量测试 | B（测试在跑）；但数字为静态手填 |

**横向口径漂移**：benchmark-baseline.md:27 写 "Core Workspace (**16 Crates**)"，README 写 18-crate；
`release-1.0.0.yml:326-327` 在 CI 里自述 "the current root workspace has **no benchmark target**"，却把空的
`target/criterion/` 当 "bench baseline" 上传（:332-338，`if-no-files-found: warn`）；
`publish-release.yml:136-148` 把 8.40µs / 0.035 / 0.012 / 0.08 / 4.20 / 18.2MB 等**无任何来源的数字**直接写进对外 Release Notes。

---

## 11. 汇总

### 11.1 证据等级计数（共 77 条宣称行：总领 3 + 记忆 12 + 认知 10 + 治理 13 + 工具 10 + 感知 8 + 前端 8 + 同步 7 + 发布 6）

| 等级 | 计数 | 分布 |
|---|---|---|
| **A-Kani/TLA 已证** | **1** | 仅审批状态机三不变量（§3.1） |
| **B-单测覆盖** | **37** | 记忆 8、认知 4、治理 8（含 1 条 B(部分)）、工具 5、感知 2、前端 2、同步 2、发布 6 |
| **C-默认关闭未实证** | **14** | 记忆 3（dream 自动沉淀/记忆闭环/遗忘）、认知 2（配额调度/器官）、治理 1（L1-L5）、工具 3（RepoMap/shell+fetch/worktree）、语音 3（全双工/PAD/五模态）、前端 1（/v1/ws）、同步 1（USB bundle） |
| **D-桩/未实现** | **9** | 总领 1（复现脚本）、Ed25519 两处 2、O-5"零桩"反证 1、StealthCrawler 1、TTS 1、P2P 加密/洋葱/漫游 3 |
| **E-仅人工** | **1** | 桌面 UI 点击流/安装验证（挂账 #2） |
| **F-纯愿景/反证** | **15** | 总领 2、基准数字 3（1.82ms/0.18ms/0.08ms）、"100% 回滚" 2、主动关怀/疲劳检测/跨代闭环 3、WGSL/色温表 2、zero-unhandled 性质 1、wrap_onion_packet 1、TTFAB 1 |

> 严格口径提示：A 只有 1 条，且带 4 项保留（不挂生产路径 / kani 非阻塞 / 固定轨迹 / TLA 无归纳泛化）；
> 名义上 B 级的条目中约 1/3 是"库级机制"而非"产品行为"；C+F+D+E 合计 **39 条（约 51%）** 的宣称
> 在默认产品路径上**不可观察或已被反证**。

### 11.2 五条最重的差距发现

1. **对外发布物里传播无来源的性能数字**。`publish-release.yml:136-148` 将 8.40µs/0.035ms/0.012ms/0.08ms/4.20ms/18.2MB
   等写进 GitHub Release Notes；这些数字在 reports/、scripts/、benches/ 中**零出处**，workspace 无任何 bench target
   （`release-1.0.0.yml:326` 自认）。而唯一独立实测（v2-memory-vector-perf，10K P99=21.24ms）与核心数字 1.82ms **矛盾 >10×**。
2. **"端到端加密漫游"是明文桩**。README 的 Noise_XX/curve25519/ChaChaPoly/多跳洋葱/BLE 漫游（README.md:316-319）在
   `p2p_mesh.rs` 里是明文 hex codec + "单跳 stub" + "不验证 Merkle"（:5-6,:52-53,:154）；连宣称的导出 API
   `wrap_onion_packet()` 都不存在。这是"文档说有、代码没有"的最大单点。
3. **默认路径上没有"认知闭环"**。主动召回/每轮整理/reflexion/反幻觉注入/9 器官/shell/fetch/worktree/洋葱层等
   全部默认关（APEIRETH_ENABLE_* 系列）；`CognitiveQuotaScheduler`、`CarePotentialField`、`LineageSpawningOrchestrator`
   连接线都没有（各自只有库级测试）——README 故事与场景里"她替你记着/主动关怀/跨代教养"在默认产品里不可观察。
4. **形式化证据面窄且不设门**。真 A 级只有 research_approval_sm 三不变量（且自述"不挂生产审批路径"）；
   6 个 crate 的 `organ_kani_proofs.rs` 是被自认的"填充测试"（core 文件头明言"不是形式化证明…命名强于实现"）；
   `kani.yml` 三步全 `continue-on-error`，证明失败不影响合入；TLA 归纳泛化未做、无 CI 跑 TLC。
5. **"0 hollow stubs / Ed25519 / WGSL"三条诚信红线失实**。O-5 锚宣称零桩零 mock，但 SDK `STUB_MODE=true`
   （WS 流 + lark/livekit/voice/sandbox 全 NotImplemented）、lineage 签名是可伪造 mock 串、
   WGSL 暗角着色器只存在于 README 两行（无 .wgsl 文件）、四态色温表与代码四个温度全部不符。
   这些是"命名/文档强于实现"的成体系模式，而非孤立笔误。

### 11.3 建议的下一步（按杠杆排序）

1. 先做 §9 的 **P-DENY / P-ROLLBACK**（先决：决定空引擎语义、补丁回滚从 best-effort 升级或宣称降级）。
2. 把 `kani.yml` 的 `continue-on-error` 撤掉，并让 mirror crate 覆盖生产 `approval.rs/execute.rs`（P-APPROVE 推广）。
3. 基准表：要么补 `[[bench]]` target + 原始输出入库，要么把 P99 表整体降为"设计目标（未复现）"；
   立即处理 publish-release.yml 注记里的无源数字。
4. 文档纠偏批：p2p_mesh / TTS / StealthCrawler / 色温表 / wrap_onion_packet / "0 hollow stubs"
   六处按 0 装口径改写为"设计意图 vs 当前实现"双栏。
5. 记忆闭环四件套给出默认开启的验收评测（LoCoMo/LongMemEval 类保持评测），把 C 升 B 的判据固定为"效果可测"。
