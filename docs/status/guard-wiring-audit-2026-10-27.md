# 护栏接线审计（2026-10-27）

> 审计对象：`crates/engine/guard`（`apeireth-guard`）—— 「越权命中恒 0」这条**安全红线**的执行者。
> 审计问题：**护栏接到了哪些生产路径？**（此前无人验证过 ⇒ 「这本身是个缺口」）
> 产出：机器可检的**接线台账** + **越权拦截实测** + **缺口清单**。

---

## 0. 结论摘要

| 口径 | 数量 | 含义 |
| --- | --- | --- |
| **已接生产**（其它 crate 的生产代码真的调用） | **17** 项 | 护栏的能力在生产路径上**真的被执行** |
| **已接生产**（只在护栏 crate 自己的生产闭环里被调用） | **16** 项 | 两段判定内核等内部能力，判定时**真的跑到** |
| **只有测试 / 离线评测在用** | **12** 项 | 代码在，**生产路径上没人调** |
| **完全没接**（零调用点） | **4** 项 | 公开能力，**全仓零引用** |
| 能力总数 | **49** 项 | 台账粒度 = 护栏对外公开的具体类型 / 函数 |

**实测结论**：越权动作**被拦住了**。生产同形的治理栈对「只读意图下的写动作」给出
`Deny`（`hook == "behavior_chain_guard"`），写面**没被碰到**（写探针计数 0），
且拒绝**落进判定账**。详见 [§3](#3-实测越权到底被拦不拦)。

**但是**：护栏**接上了**，不等于**接全了**。[§4 缺口清单](#4-缺口清单按风险排序) 列了 8 条缺口，
其中 **2 条 P0** 能让「越权真的发生」。

---

## 1. 方法与口径（怎么算「接上了」）

判定全部**机器可复算**，不靠人填表：

```sh
python3 scripts/check_guard_wiring.py            # 查台账与代码是否一致（不一致 ⇒ 退出码 1）
python3 scripts/check_guard_wiring.py --report   # 只打印覆盖率报告
python3 scripts/check_guard_wiring.py --update   # 重扫锚点，重写台账
```

台账：`scripts/guard-wiring-ledger.json`（由 `--update` 生成）。

**口径（与 `scripts/check_quality_metrics.py` 同口径）**：

1. **锚点用具体类型 / 函数名**（`BehaviorChainGuardHook`、`with_decision_log`…），
   **不用泛词** —— 泛词到处命中 = 假绿；
2. **「生产」= `crates/**/src/**` 里的非测试代码**：
   `#[cfg(test)]` 块、`tests/` 目录、`examples/`、`benches/` 一律**不算生产**；
3. **「crate 内生产路径」= 从 `hook.rs` 可达的护栏模块**；
   `scenario` / `oracle` 是**离线评测面**，**不算**；
4. **只 import 不调用 ⇒ 不算调用点**（`use` 语句整体剔除后再匹配）；
5. **定义行不算调用点**（`pub fn X` / `pub struct X` / `impl ... for X`）——
   否则「自己定义自己」会把每个能力都染成「已接生产」；
6. **跨 crate 同名不误判**：护栏的类型名**不唯一**（`apeireth_memory` 也有一个
   `GuardDecision`）⇒ 护栏 crate 之外，裸名必须**确实是护栏的名字**
   （本文件导入自 `apeireth_guard` / 写成 `apeireth_guard::X` / `M::X`）；
7. **守卫语义**：台账说「已接生产」但代码锚点扫不到 ⇒ **台账撒谎了** ⇒ **退出码 1**；
   反向（台账说没接但代码里已有引用）同样退出码 1。

**台账是快照**：本台账是 **2026-10-27** 的扫描结果。此后**任何**新增 / 删除护栏接线
（哪怕只改一行）都会让检查退出码 1 ⇒ 必须同步跑 `--update`。
这是「台账不许撒谎」的**代价也是收益**：护栏接线一变，台账立刻**响**。

---

## 2. 接线台账（49 项能力 → 状态 + 证据）

> 完整机读版：`scripts/guard-wiring-ledger.json`（每项带 `production_sites` / `internal_sites` / `test_sites` 逐行证据）。

### 2.1 已接生产（17 项，其它 crate 的生产代码真的调用）

| 能力 | 证据（文件:行） |
| --- | --- |
| `hook.guard_hook` 守门本体 | `crates/adapters/cli/src/lib.rs:1025`（构造）、`:1107`（挂进 pipeline）、`crates/engine/runtime-assembly/src/canonical/guard_observer.rs:23` |
| `hook.dry_run` 试算面 | `crates/adapters/gateway/src/panels.rs:1128`、`crates/adapters/cli/src/gateway_panels.rs:1001` |
| `hook.status` 状态自述 | `crates/adapters/gateway/src/panels.rs:502`、`:509` |
| `hook.recent_events` 近判定读口 | `crates/adapters/gateway/src/panels.rs:1111`、`crates/adapters/cli/src/gateway_panels.rs:1081` |
| `hook.update_action_execution` 执行结果回写 | `crates/engine/runtime-assembly/src/canonical/guard_observer.rs:143` |
| `hook.with_decision_log` 判定落盘挂载 | `crates/adapters/cli/src/guard_decision_wiring.rs:39` |
| `hook.with_immune_memory` 免疫记忆挂载（默认关） | `crates/adapters/cli/src/lib.rs:1194` |
| `hook.with_dataset_recorder` 数据集记录器挂载（默认关） | `crates/adapters/cli/src/lib.rs:1027`、`:1149` |
| `hook.with_classifier` 模型分类器挂载（默认关） | `crates/adapters/cli/src/lib.rs:1144` |
| `intent.rule_interpreter` 规则意图解释器 | `crates/adapters/gateway/src/canonical_entry.rs:342`、`crates/adapters/cli/src/lib.rs:2658` |
| `classifier.joint` 联合风险模型 | `crates/adapters/cli/src/lib.rs:1142` |
| `decision_log.ledger` 判定落盘账 | `crates/adapters/cli/src/guard_decision_wiring.rs:38`、`:152` |
| `decision_log.risk_label` 风险分→档映射 | `crates/adapters/cli/src/guard_decision_wiring.rs:174` |
| `dataset.recorder` 脱敏数据集记录器 | `crates/adapters/cli/src/lib.rs:1160`、`crates/engine/runtime-assembly/src/canonical/guard_observer.rs:22` |
| `dataset.execution_outcome` 执行结果归类 | `crates/engine/runtime-assembly/src/canonical/guard_observer.rs:132` |
| `semantics.registry` 能力安全注册表 | `crates/engine/runtime-assembly/src/canonical/parallel_dispatch.rs:45`、`:65` |
| `introspection.dto` 自述 DTO | `crates/adapters/gateway/src/panels.rs:503`、`:1123` |

### 2.2 已接生产 · crate 内生产路径（16 项，判定时真的跑到）

全部位于 `crates/engine/guard/src/`：

| 能力 | 证据（文件:行） |
| --- | --- |
| `intent.alignment_guard` | `hook.rs:853`（`IntentAlignmentGuard.evaluate`） |
| `intent.negation_extractor` | `intent.rs:437` |
| `intent.confidence_policy` | `intent.rs:563` |
| `fast_guard.stage_a` | `hook.rs:431`（`FastGuard::new`）、`hook.rs:858`（`fast_guard.evaluate`） |
| `chain_guard.stage_b` | `hook.rs:432`（`ChainGuard::new`）、`hook.rs:865`（`chain_guard.evaluate`） |
| `fusion.decision_fusion` | `hook.rs:896`（`DecisionFusion::apply`） |
| `classifier.no_classifier` | `hook.rs:442` |
| `decision.guard_decision` | `hook.rs:862`（`GuardDecision` 构造）、`hook.rs:1067`（`to_verdict`） |
| `enforcement.directive` | `hook.rs:108`（`EnforcementDirective::from_decision`） |
| `command_effect.analyzer` | `semantics.rs:402` |
| `observation.model` | `hook.rs:842`（`SafetyObservation::from_governance_request_with_provider`） |
| `chain.behavior_chain` | `chain.rs:117`（类型）、`hook.rs:818`（`BehaviorChain::new`） |
| `features.v1` | `features.rs:51`（`AgentChainFeatureV1::from_chain`） |
| `features.v2` | `hook.rs:893`（`AgentChainFeatureV2::from_chain_with_cross_turn`） |
| `session.history` | `hook.rs:396` |
| `snapshot.store` | `hook.rs:399` |

### 2.3 只有测试 / 离线评测在用（12 项）

| 能力 | 唯一调用点 | 说明 |
| --- | --- | --- |
| `hook.evaluate_detailed` | `crates/engine/guard/src/hook/tests.rs:128` | 明细判定读口无生产消费者 |
| `hook.bind_turn_intent` | `crates/engine/guard/src/scenario.rs:369` | 生产走 `TurnSecurityContext`（网关已接） |
| `hook.set_declared_scope` | `crates/engine/guard/tests/guard_tests.rs:162` | 生产靠意图的 `allowed_scopes` 覆盖（`chain.rs:146`） |
| `hook.clear_session` | `crates/engine/guard/src/hook/tests.rs:134` | **⇒ 缺口 P1-3** |
| `hook.session_readouts` | `crates/engine/guard/src/scenario.rs:405` | 会话摘要 / 风险史读口无生产消费者 |
| `hook.last_feature_snapshot` | `crates/engine/guard/src/hook/tests.rs:130` | 诊断读口 |
| `hook.last_immune_signal` | `crates/engine/guard/src/hook/tests.rs:210` | 免疫信号读口 |
| `classifier.artifact_stamp` | `crates/engine/guard/examples/stamp_guard_artifact.rs:10` | 离线模型工件工具面 |
| `semantics.descriptor_lookup` | `crates/engine/guard/src/semantics.rs:211` | 自由函数面；生产走注册表方法 |
| `semantics.coverage_report` | `crates/engine/guard/examples/export_guard_scenarios.rs:193` | 覆盖率报告 |
| `scenario.catalog` | `crates/engine/guard/examples/export_guard_scenarios.rs:30` | 离线场景跑批 |
| `oracle.security_scenario` | `crates/engine/guard/src/oracle.rs:149` | 离线 oracle 对账 |

### 2.4 完全没接（4 项，零调用点）

| 能力 | 后果 | 台账条目 |
| --- | --- | --- |
| `enforcement.directive_consumer` 收容义务**执行面** | 高危拒绝后**不撤销会话授权**、不标记高风险 trace、无补偿线索落地 | **⇒ 缺口 P0-1** |
| `hook.with_safety_provider` 能力安全元数据源注入 | 护栏**永远**用 canonical 注册表判能力安全；插件声明面进不来 | **⇒ 缺口 P1-4** |
| `intent.constrain_to_trusted` 模型提案向可信信封收窄 | 「模型提案面」**没有护栏**（也无该通道） | **⇒ 缺口 P2-5** |
| `classifier.threshold` 阈值分类器 | 影子标定 / 集成测试用的确定性分类器**从未装配** | **⇒ 缺口 P2-6** |

---

## 3. 实测：越权到底被拦不拦

### 3.1 有没有生产入口？——**有**

护栏**不是孤岛**。生产入口链（逐段可核）：

```
CLI 装配根  crates/adapters/cli/src/lib.rs:1025   构造 BehaviorChainGuardHook
            crates/adapters/cli/src/lib.rs:1104   pipeline = … .with(CredentialDisclosureHook)
                                                    .with(PromptInjectionHook)
                                                    .with(guard_hook.clone())      ← 护栏挂在这里（末位）
            crates/adapters/cli/src/lib.rs:1577   builder.with_governance(governance_chain::production(...))
网关入口    crates/adapters/gateway/src/canonical_entry.rs:342   RuleIntentInterpreter.interpret(...)
                                                        :348   TurnSecurityContext::new(...).with_intent(intent)
runtime    crates/engine/runtime/src/canonical/execute.rs:3191   governance.evaluate_verbose(...)
            crates/engine/runtime/src/canonical/execute.rs:3215   Decision::Deny ⇒ permanent_error("refused by governance: …")
```

⇒ **不存在「没有生产入口」这一发现**。护栏**接在** runtime 的每一次能力派发上。

### 3.2 实测怎么做的

`crates/engine/guard/tests/wiring_evidence.rs`（7 个测试，全绿）：

* **生产同形**：按 `cli::governance_chain::production()` 的形状构造
  `GovernanceChain` → `GovernancePipeline` → 护栏 hook（护栏在 pipeline **末位**）；
* **生产同形**：按 `gateway/canonical_entry.rs` 的形状用 `RuleIntentInterpreter`
  造可信意图（用户原话 =「检查 src/main.rs 的逻辑并解释一下」⇒ 只读意图，
  `mutation_policy = Deny`）；
* **生产同形**：按 `runtime/execute.rs` 的形状造请求
  （`Action::CapabilityDispatch` + `with_action_id` + `with_optional_security_context`）；
* **越权动作**：`tool.apply_patch` + 真实补丁参数（写面，用户**没授权**）。

### 3.3 实测结果：**被拦住了**

| 断言 | 结果 |
| --- | --- |
| 链入口（生产同形）判定 | `Decision::Deny`，`hook == "governance_chain"` |
| pipeline 层判定 | `Decision::Deny`，**`hook == "behavior_chain_guard"`** ⇒ 拒绝**确实来自护栏** |
| 拒绝理由 | 事件流 `reasons` 含 `read_only_task_requested_mutation`（`crates/engine/guard/src/intent.rs:714`）；判定文案 `intent and action alignment is high risk`（`crates/engine/guard/src/hook.rs:332`） |
| **没改数据** | 写探针（照抄 runtime 派发闸语义）计数 **0**；错误帧 `refused by governance: …` |
| 拒绝落盘 | 判定账 1 行，`decision == "deny"`，`capability_id == "tool.apply_patch"` |
| **不依赖任何默认关开关** | 默认构造（无模型 / 无数据集记录器 / 无免疫记忆）**仍然拒** |

### 3.4 诚实边界

* 本审计**没有**宣称「完整生产运行时端到端跑过一次越权拦截」——那要起真实
  provider 与真实工具，超出护栏 crate 的依赖面；
* 本审计**能**宣称的是：护栏 hook 在**生产同形**的治理栈上对越权动作给出 `Deny`，
  且 runtime 的派发闸（源码锚点钉在测试里）对 `Deny` 是「拒绝即帧、不派发」；
* 运行时证据（§3.3）与**静态接线证据**（§3.1 的 `include_str!` 源码锚点）
  在测试里**分开标注**，不混为一谈。

---

## 4. 缺口清单（按风险排序）

> 每条都带**可核对**的证据（文件:行），没有「应该」「可能」这类措辞。

### P0-1 收容义务**零执行面** ⇒ 高危拒绝后不撤销会话授权

* **事实**：`EnforcementDirective::from_decision` 在
  `crates/engine/guard/src/enforcement.rs:22-32` 算出三项义务：
  `revoke_session_grant`（`denied && risk_score >= 0.90` 时为真）、
  `mark_trace_high_risk`、`compensation_hint`；
* **事实**：护栏只把义务**塞进事件流**（`crates/engine/guard/src/hook.rs:1021`
  `containment: containment_obligations(&guard_decision)`），
  护栏 crate 之外**零消费者**（台账条目 `enforcement.directive_consumer` = `NOT WIRED`）；
* **事实**：全仓唯一调用 `PermissionPolicy::revoke` 的地方是
  `crates/adapters/cli/src/gateway_panels.rs:870` —— **面板上的人工撤销操作**，
  **不由守卫拒绝自动触发**；
* **后果**：一次高危越权被拒后，**会话授权仍在** ⇒ 同一会话内换一个动作再来一次，
  护栏不会因为「上一轮已拒绝」而加重 ⇒ **越权可以重复发生**。

#### ✅ **P0-1 已修**（2026-10-27，commit `1c88655a`）

* 护栏 crate 新增 `containment` 模块（`ContainmentDirective` / `GrantRevocations` /
  `ContainmentGate` / `ContainmentConsumer`）；
* **执行面在 CLI 装配根**（护栏 crate 外）⇒ ⇒ 消费护栏**发布**的义务 ⇒ ⇒ **真撤销**该会话授权；
* 🎯 **机器证明**：第 1 次拒（`intent and action alignment is high risk`）
  ⇒ ⇒ `revoked = true` ⇒ ⇒ **第 2 次同会话再来一次**被拒，理由变成
  `session grant revoked by guard containment …` ⇒ ⇒ **「重复越权」被堵住**；
* 台账 `enforcement.directive_consumer`：**NOT WIRED → PRODUCTION WIRED**；
* ⚠️ **诚实边界**：撤销**只在内存**（进程重启即失效）· **无恢复通道** ·
  **会话粒度**（整个会话被拒，含只读）⇒ ⇒ 这三条**需 owner 裁决**。

### P0-2 护栏挂在 pipeline **末位** ⇒ 上游闸拒掉的动作**不进护栏行为链**

* **事实**：`crates/adapters/cli/src/lib.rs:1104-1107` 里，护栏是
  `CredentialDisclosureHook` → `PromptInjectionHook` → **护栏** 的**最后一位**
  （护栏之后只有「三洋葱末层」`OnionLayerHook`，`crates/adapters/cli/src/lib.rs:1110-1113`，
  **默认关**）；护栏之前还有授权闸 `PermissionGovernanceHook`
  （`crates/adapters/cli/src/lib.rs:1036`）与限流闸（`:1103`）；
* **事实**：`GovernancePipeline::evaluate_verbose` 在**第一个非 allow 处短路返回**
  （`crates/foundation/governance/src/lib.rs:578-583`）⇒ 上游闸拒掉的动作
  **根本不会进护栏**；
* **事实**：动作进链发生在**护栏自己的评估内部**
  （`crates/engine/guard/src/hook.rs:851` `chain.add_action_with_id(...)`）；
* **事实**：护栏的复合检测依赖链里有**被拒动作**：
  `BehaviorChain::has_same_effect_after_denial`（`crates/engine/guard/src/chain.rs:314`）、
  `has_retry_escalation`（`:403`）、`has_destructive_chain`（`:457`）；
* **后果**：被授权闸 / 提示注入闸 / 限流闸拒掉的动作，护栏**看不见** ⇒
  「拒绝后换个工具再来一次」（同效果绕过）、「拒绝后重试升级」这两条护栏能力
  对**上游拒绝**失明 ⇒ **复合越权检测有洞**。

#### ✅ **P0-2 已修**（2026-10-27，commit `c4b0669f`）

* **修法（不动 pipeline 顺序 ⇒ 不碰「上游拒绝 ⇒ 下游不得放行」这条安全属性）**：
  1. `GovernanceHook` 新增 **`on_denied(request, verdict)`** 回调（**默认 no-op**）；
  2. `GovernancePipeline::evaluate_verbose` 在短路时**通知后面的 hook**；
  3. 护栏实现 `on_denied` ⇒ ⇒ 把「上游已拒」记进**它自己那条行为链**
     （与护栏**自己**拒绝时的记法**完全一致**）⇒ ⇒ 复合检测**分不出**是谁拒的
     ⇒ ⇒ ⇒ **这正是要的**（「拒绝过」这件事本身就够了）；
* ⚠️ **只记事实** ⇒ ⇒ **不改变护栏的判定结果**（只影响**后续**评估能看到这条历史）；
* **接线证据**（`crates/engine/guard/tests/p0_2_upstream_denial.rs`，3 条）：
  * 上游拒绝 ⇒ **护栏链里出现那条被拒动作**（恰好 1 条）；
  * **对照**：没有上游拒绝 ⇒ 链里**没有**被拒动作；
  * ⚠️ **短路语义不变**：下游 hook **evaluated = 0 次**、**notified = 1 次**。

### P1-3 会话态**零回收面** ⇒ 跨会话污染 + 无界增长

* **事实**：`BehaviorChainGuardHook::clear_session` 定义在
  `crates/engine/guard/src/hook.rs:680`，**全仓唯一调用点**是护栏自己的测试
  `crates/engine/guard/src/hook/tests.rs:134`（台账条目 `hook.clear_session` = `TEST ONLY`）；
* **事实**：护栏按会话键持有且**无上界**的表：`session_scopes` / `session_risk_history` /
  `session_behavior_summary` / `session_history` / `turn_intents` / `last_immune_signals`
  （`crates/engine/guard/src/hook.rs:393-397`、`:413`）——
  `chains` 有 `MAX_ACTIVE_CHAINS = 256` 的 LRU 兜底（`hook.rs:95`），
  `last_snapshots` 有 `MAX_LAST_SNAPSHOTS = 1024` 兜底，**这六张表没有**；
* **后果**：① 同一 `SessionId` 复用时，上一会话的行为链 / 拒绝计数 / 敏感探针计数
  会被 `SessionBehaviorHistory::cross_turn_features`
  （`crates/engine/guard/src/session.rs:158`）算进新回合 ⇒ 判定被**跨会话污染**；
  ② 长跑进程里这六张表**只增不减**。

### P1-4 能力安全元数据源**不可注入** ⇒ 插件能力走启发式兜底

* **事实**：`BehaviorChainGuardHook::with_safety_provider`
  （`crates/engine/guard/src/hook.rs:533`）**零调用点**（台账条目
  `hook.with_safety_provider` = `NOT WIRED`）⇒ 护栏**永远**用
  `CapabilitySafetyRegistry::canonical()`（`hook.rs:443`）；
* **事实**：注册表里没有的能力 ⇒ `infer_descriptor` 兜底
  （`crates/engine/guard/src/semantics.rs:867`），未知能力的
  `primary_operation` 落到 `OperationClass::Unknown`、`external_effect = true`
  （`semantics.rs:918-923`）；
* **事实**：这种能力进对齐层只得 `ScopeExpansion` / **0.75**
  （`crates/engine/guard/src/intent.rs:626-640`）⇒ 0.75 < 0.85 ⇒
  结论是 **`RequireApproval`（人工审批），不是 `Deny`**；
* **后果**：插件注册的能力在护栏眼里是「未知」⇒ 只到「停人工」不到「拒」。

### P2-5 模型提案向可信信封收窄 —— 死代码

* **事实**：`constrain_to_trusted`（`crates/engine/guard/src/intent.rs:569`）零调用点
  （台账条目 `intent.constrain_to_trusted` = `NOT WIRED`）；
* **事实**：护栏当前只读**用户意图**（经 `TurnSecurityContext`），模型提案**不进这条链**；
* **后果**：**当前**不构成越权（该通道根本不存在）；但一旦有人接上模型提案面，
  仓里**没有**「提案向授权收窄」的闸。

### P2-6 阈值分类器 —— 死代码

* **事实**：`ThresholdClassifier`（`crates/engine/guard/src/classifier.rs:162`）
  零调用点（台账条目 `classifier.threshold` = `NOT WIRED`）；
* **事实**：它自己的文档说用途是「local integration tests and shadow-mode calibration」
  （`classifier.rs:156-158`）；
* **后果**：护栏的模型面**只有** `JointRiskClassifier`（需本地工件 + env 开关），
  影子标定**没有**确定性兜底路径。

### P2-7 诊断 / 读口类能力只有测试在用

`hook.evaluate_detailed`、`hook.session_readouts`、`hook.last_feature_snapshot`、
`hook.last_immune_signal`、`hook.bind_turn_intent`、`hook.set_declared_scope`
（台账条目见 §2.3）。后果：这些面**不参与**安全判定，只影响可观测性与运维排障；
其中 `hook.last_immune_signal` 无消费者 ⇒ 免疫记忆的升级信号**出不来**
（免疫记忆本身默认关，见 §4-P3-8）。

### P3-8 默认关闭的能力（**已实测不影响拦截**）

| 能力 | 开关 | 实测 |
| --- | --- | --- |
| 数据集记录器 | `APEIRETH_GUARD_DATASET_ENABLED=1`（`crates/adapters/cli/src/lib.rs:1150`） | 默认关；**实测**：默认构造下越权**仍然被拒** |
| 本地模型分类器 | `APEIRETH_GUARD_ML_MODE` + `APEIRETH_GUARD_ML_MODEL`（`lib.rs:1130`、`:1136`） | 默认关；**实测**：默认构造下越权**仍然被拒** |
| 免疫记忆 | `APEIRETH_ENABLE_IMMUNE_MEMORY=1`（`lib.rs:98`、`:1187`） | 默认关；免疫 tier **不改写** allow/deny 主结论（`crates/engine/guard/src/hook.rs:904-908`） |

⇒ 记录在此的**目的**是防止「默认部署 ⇒ 越权命中恒 0」这句话被误读成
「护栏的模型面 / 数据面 / 免疫面默认就在跑」。

---

## 5. 复算方式

```sh
# 1) 接线台账（覆盖率 + 缺口如实列出；台账与代码不一致 ⇒ 退出码 1）
python3 scripts/check_guard_wiring.py

# 2) 越权拦截实测（7 个测试）
runuser -u bai -- env HOME=/home/bai LC_ALL=C.UTF-8 CARGO_INCREMENTAL=0 \
  bash -c 'cd /home/bai/wt-apeireth-r3 && cargo nextest run -p apeireth-guard --profile ci --locked'
```

**本批新增文件**（只这 4 个，未改任何 `lib.rs`，未动 `crates/engine/guard/src/*.rs`，
未动 `crates/engine/memory/src/*.rs`，未新增依赖，未改 `Cargo.lock`）：

* `scripts/check_guard_wiring.py`（新建）
* `scripts/guard-wiring-ledger.json`（新建，脚本生成）
* `crates/engine/guard/tests/wiring_evidence.rs`（新建）
* `docs/status/guard-wiring-audit-2026-10-27.md`（新建，本文件）
