# 册 04 — 评估与守门（7 篇合册）

> **一句话定位**：Guard 3.0→3.3 的**四轮安全守门演进**（意图感知 → 正确性 → 评估完整性 → 轨迹保真）+ 一次**外部 SSE CoT 字段实测** + 两篇 round15 验收件；共同特征是**结论一律以三态标记**（`FREEZE_READY` / `NOT_FREEZE_READY` / `CI_VERIFIED = NO`），不把未跑的门禁写成绿。
>
> **来源篇数**：7（分配表 §04，逐篇唯一归册）｜**原篇日期跨度**：2026-08-02 ~ 2026-09-08（2 篇未标日期）
> **合册生成日**：2026-10-23｜**复核口径（现行真源）**：[test-baseline](../docs/03-reference/test-baseline.md) §1 ／ [claims.yaml](../docs/status/claims.yaml) ／ [00-backlog.md](../docs/04-internal/backend-enhancement/00-backlog.md) ／ [AGENTS.md](../AGENTS.md)

## 目录

| # | 原标题 | 原日期 | 轮次 |
|---|---|---|---|
| 1 | Apeireth Guard 3.0 — Intent-Aware Behavioral Defense | 未标日期 | G3.0 |
| 2 | Apeireth Guard 3.1 — Correctness and Evaluation | 未标日期 | G3.1 |
| 3 | Guard 3.2 Evaluation Integrity and Shadow Readiness | 2026-09-07 | G3.2 |
| 4 | Guard 3.3 Trace Fidelity and Semantic Generalization | 2026-09-08 | G3.3 |
| 5 | SSE CoT `delta.reasoning_*` 验证报告 | 2026-08-19 | 外部接口实测 |
| 6 | round15-01: apeireth-asi R-Measure ML 在线校准循环 — 验收报告 | 2026-08-02 | round15 |
| 7 | round15-02 apeireth-bus 5 层通信总线 — 验收报告 | 2026-08-03 | round15 |

---

### Apeireth Guard 3.0 — Intent-Aware Behavioral Defense (原篇未标日期)

**来源**：`reports/guard-v3-intent-behavior-defense.md`（约 130 行）｜分支 `feature/cognitive-infrastructure-vnext`｜状态报告（read-only 性质的自述）。
**结论/要点**
- **安全路径架构（原文给出完整链路）**：`user-visible turn input` → `RuleIntentInterpreter`（turn 起始**仅一次**）→ `TaskIntentEnvelopeV1` + `TurnSecurityContext` → `Runtime::execute_outcome` → `GovernanceRequest`（完成与工具调用**同一 context**）→ `FastGuard + IntentAlignmentGuard + ChainGuard` → `AgentChainFeatureV2` → 可选本地分类器 → `DecisionFusion`。**原始请求只是解释器的瞬时输入，不写入 Guard 数据集记录、行为链或特征向量**。
- **已实现项**：`TaskIntentEnvelopeV1` **版本化、trace 绑定、来源标注**，并**分离「被请求的任务语义」与「动作的内在风险」**；`RuleIntentInterpreter` 识别中英文常见 read / edit / test / network / publish / delete / credential / shell 表达；**意图特权单调性**由 `constrain_to_trusted` 强制。
- **Truth status** 一节（逐项标注哪些是已实现、哪些不是）与 **Threat model covered** 一节。
- **分支与集成政策**一节；**Verification** 三条命令均标 PASS：`cargo check -p apeireth-guard -p apeireth-runtime -p apeireth-cli -p apeireth-gateway --all-targets --locked` / `cargo test -p apeireth-guard --all-targets --locked` / `cargo clippy -p apeireth-guard -p apeireth-runtime -p apeireth-cli -p apeireth-gateway --all-targets --locked -- -D warnings`。
- **诚实边界（关键）**：全 workspace 门禁、前端门禁、远端 CI、ONNX 导出/加载、性能百分位报告**仍待运行或实现**，原文明确「intentionally not represented as green here」。
**现状复核（2026-10-23）**：**已过时（分支与 crate 面换代）**——分支 `feature/cognitive-infrastructure-vnext` 属 2026-09；Guard crate 现为 [crates/engine/guard](../crates/engine/guard)（现行 canonical），guard 相关的机器化门禁是 `python scripts/arch_guard.py --strict`（各批记录 VIOLATIONS 0 / STALE N）。**已过时（守门级数口径）**——原文未给守门级数，但现行守门现值 = **7 重守门 v7**（1–5 重嵌套 + 6 Colang DSL + 7 Superpowers Skill Guard，[ROADMAP.md](../ROADMAP.md) §13）；本篇的「FastGuard + IntentAlignmentGuard + ChainGuard + AgentChainFeatureV2 + DecisionFusion」是守门链的**库级组件**，其「五段流水线」在现行 §0.2 记忆注入格式等条目中另有表述。**仍成立（最值得继承的三条）**：① **「原始请求是瞬时输入，不写入数据集 / 行为链 / 特征向量」**——这是隐私与最小暴露的正确设计，现行 [claims.yaml](../docs/status/claims.yaml) 与 [capabilities-matrix.md](../docs/03-reference/capabilities-matrix.md) 沿用同一「不假装、不越界记录」原则；② **「分离任务语义 vs 动作内在风险」**与现行 3 不可变脊柱中的治理链（`GovernanceChain::run` 六段）分工一致；③ **「未跑的门禁不写成绿」**——本篇显式列出「全 workspace 门禁 / 前端门禁 / 远端 CI / ONNX / 性能报告」未跑，与现行 [AGENTS.md](../AGENTS.md) §1 诚实纪律及债栏 D28/D30/D31（环境性失败的如实登记）完全一致。**未核实**：本篇的 Truth status 逐项在现行 `crates/engine/guard` 中的存续情况——**未核实**，不作断言。
**归属链接**：../crates/engine/guard、../scripts/arch_guard.py、../ROADMAP.md、../docs/03-reference/capabilities-matrix.md、../docs/status/claims.yaml、../AGENTS.md、../docs/04-internal/backend-enhancement/00-backlog.md；历史 分支 `feature/cognitive-infrastructure-vnext`

### Apeireth Guard 3.1 — Correctness and Evaluation (原篇未标日期)

**来源**：`reports/guard-v3.1-correctness-and-evaluation.md`（约 190 行）｜分支 `feature/cognitive-infrastructure-vnext`｜Base SHA `3d4d4ab997a81b038b33ae1b9b41a7ab06ac5af9` → Final SHA `4c28da2ec35b691f58c76d941403fb943fe2fcbe`；`origin/main` = `7647d2c91d55901aeae7202f3842b65233bd053c`；起手 ahead/behind = 0｜**本轮未新建 Guard 分支、未改 main**。
**结论/要点**
- **3 个 commit**：`ae2395e4` fix(guard): make intent, fusion, and FeatureV2 snapshots semantically truthful；`2f9cfe1c` feat(guard-scenarios): generate FeatureV2 from real intent-action simulations；`4c28da2e` docs: report Guard 3.1 correctness and evaluation status。
- **P0 发现与修复 4 条（逐条给发现 + 修法）**：**P0-1** 未知意图可能拿到 `WorkspaceOnly` mutation → 未知/低置信 envelope 现在**fail-narrow**（network / credential / shell / mutation / destructive / persistence 一律 `Deny`）；**P0-2** 否定语义原为子串/`contains` → `NegationAwareOperationExtractor` 用**双语 deny 短语 + 局部前缀匹配**，**显式否定压过推断出的许可**；**P0-3** Advisory ≈ Enforce → **Shadow 永不改变 `Decision`**；Advisory 可把 Allow 升为 RequireApproval 但**模型单独不能 Deny**；Enforce 仅在**有结构化证据**时才可 Deny；**P0-4** `recent_turns` 曾按每次治理评估递增 → 现按 `trace_id` upsert，**一 trace 三个动作只算一个 turn**。
- 覆盖章节：意图正确性 / 否定语义 / 模式语义 / 跨轮架构 / 特征快照架构 / Dataset V3 / 能力元数据真值链 / shell 效果分析器 / 同效果绕过 / 产物校验 / 场景 DSL / 数据集规模与切分 / **ML 基线结果** / 性能。
- **DoD 矩阵（末列为实测判定）**：阈值已验证 YES；场景 DSL 跑真实 Guard 管线 YES；训练特征为生成而非手写 YES；train/val/test 按场景族分离 YES；硬负例存在 YES；中英覆盖存在 YES；workspace 测试绿 YES；clippy 绿 YES；deny 绿 YES；audit 绿 YES；**运行时依赖墙完整 YES**；**远端 CI 已验证 NO**。
**现状复核（2026-10-23）**：**已过时（分支 / SHA / 门禁面）**——SHA 属 2026-09 语境；现行测试真源为 [test-baseline.md](../docs/03-reference/test-baseline.md) §1（341 套件 / 8570 passed / 0 failed / 51 ignored，2026-10-21），本篇的 scoped 绿证据不得外引。**已过时（远端 CI 一项）**——「远端 CI 已验证 NO」已部分消解：现行 [claims.yaml](../docs/status/claims.yaml) 多条 `notes` 记有「远程 CI 首跑见 PR #36」且桌面 CI gate 3/3 success；但**仍有一条同类活债**——`gov.chain_drill_ci` 明载「远端仍 0 次执行：三平台 nextest 常红导致 job 早停、步骤被 skip」，故「远端 CI 已验证 NO」的**结构性成因在现行依然存在**。**仍成立（方法论，本册核心）**：① **fail-narrow 而非 fail-open**（未知意图一律 Deny 高危面）——与现行 [AGENTS.md](../AGENTS.md) §1「0 装诚实纪律」及 [capabilities-matrix.md](../docs/03-reference/capabilities-matrix.md) §0 的 `reason=not_implemented` 表达同向；② **Shadow 与 Enforce 语义分离**（Shadow 永不改 Decision；模型单独不能 Deny）——与现行 `gov.reflection_queue`（IMPLEMENTED，**生产未启用反射调度器，零行为变化**）的「不冒称接线」做法同构；③ **「按 trace_id 计轮」而非按评估次数计轮**；④ **DoD 矩阵逐项给 YES/NO**（末项敢写 NO）——与现行债栏逐条状态注记的写法一致。**未核实**：本篇 P0 修法在现行 `crates/engine/guard` 中的存续情况——**未核实**。
**归属链接**：../crates/engine/guard、../docs/status/claims.yaml、../docs/03-reference/test-baseline.md、../docs/03-reference/capabilities-matrix.md、../AGENTS.md、../docs/04-internal/backend-enhancement/00-backlog.md；历史 分支 `feature/cognitive-infrastructure-vnext`

### Guard 3.2 Evaluation Integrity and Shadow Readiness (2026-09-07)

**来源**：`reports/guard-v3.2-evaluation-integrity-and-shadow.md`（约 150 行）｜分支 `feature/cognitive-infrastructure-vnext`｜Base `d359868f8d3332af0a5f4d118c6e8ec26e598cf9`；实现 commit `f05351e47bfb1d4738340286e9b376799058774c`｜**`CI_VERIFIED = NO`**。
**结论/要点**
- **执行结果：`FREEZE_READY = NO` / `NOT_FREEZE_READY`**。评估完整性控制、独立 oracle、真实族 holdout、产物来源与 shadow 分析工具**已实现并本地验证**；**运行时默认仍是 disabled classifier / `NoClassifier`（SHADOW 语义保留，未引入生产 Enforce 默认）**。冻结仍被四项阻塞：缺真实 shadow 证据、缺最终 SHA 的远端 CI 证据、关键 holdout 的合成泛化结果偏弱、需把更新的 Guard 3.3 轨迹保真证据带入最终发布决策。
- **范围与分支卫生**：工作留在 `feature/cognitive-infrastructure-vnext`；**未建 Guard 分支、未改/未合 main、未作 PR 目标、未 push**；**入库的评估产物 / 报告 / 评审队列不含**任何原始秘密、凭据、URL、绝对路径、命令载荷、工具输出、记忆或思维链内容。
- **Guard 3.1 术语纠正（重要）**：历史 Guard 3.1 的切分是 **within-family held-out instance split**，**并未留出完整未见族**；因此 3.2 把该基线保留在 `within_family_held_out_baseline` 名下，**明确不称其为 unseen-family 结果**。3.2 改用**确定性 SHA-256 族/组分配**：一个族整体只落入 `train` / `calibration` / `validation` / `test` 之一，**隔离断言拒绝四分区间的任何重叠**；切分报告记录族清单与行/类/语言计数。
- 另含评估维度与分类法 / **独立 oracle 政策** / 合成基线结果 / 数据集-产物-模式完整性 / shadow 分析与隐私 / 运行时与依赖边界 / **验证门禁** / DoD 矩阵。
- **5 项 Deferred**：① 通过批准的隐私保护导出路径取得**真实 shadow 数据**并跑分析器，**绝不把确定性 deny 或审批通过当作 ground truth**；② 改进工具族与成对泛化后再考虑分类器晋升；③ 多动作/多轮轨迹中**每个动作逐一与 oracle 标签/效果重算对齐**；④ 为最终 commit SHA 跑并记录远端 CI；⑤ 既有依赖/前端告警另行处理，**不静默当作 Guard 失败**。**最终判定：`NOT_FREEZE_READY`**。
**现状复核（2026-10-23）**：**已过时（分支与具体治理面）**——分支与 SHA 属 2026-09；Guard crate 现为 [crates/engine/guard](../crates/engine/guard)。**已过时（部分阻塞项已消解）**——「缺最终 SHA 远端 CI」：现行 [claims.yaml](../docs/status/claims.yaml) 多条 `notes` 记有「远程 CI 首跑见 PR #36」，桌面 CI 3/3 success，故**该项已部分消解**；但**同类阻塞仍在**（`gov.chain_drill_ci`：远端 0 次执行，步骤被 skip）。**仍成立（方法论，本篇最值钱的两条）**：① **「术语纠正：把 within-family instance split 明确降级标注，不得称 unseen-family 结果」**——这是本册**最可复用的诚实纪律**：现行 [test-baseline.md](../docs/03-reference/test-baseline.md) §3 的「历史口径（带日期保留，不追溯改写）」表、§2 的「运行时通过数 vs 静态计数」区分，都是同一纪律的制度化；② **「绝不把确定性 deny 或审批通过当作 ground truth」**——现行 [claims.yaml](../docs/status/claims.yaml) 的四级口径（NOT IMPLEMENTED < IMPLEMENTED < PRODUCTION WIRED < HARDWARE VALIDATED，**且 `DEFAULT ENABLED` 不是一级而是横切属性**）正是这条纪律的落地；③ **「Shadow 永不改 Decision、运行时默认 disabled classifier」**——与现行 `gov.reflection_queue`（IMPLEMENTED / 生产未启用 / 零行为变化）一致；④ **「远端 CI 不可核 → 标 `CI_VERIFIED = NO`，不用 NO 冒充 YES」**。**未核实**：本篇的独立 oracle / shadow 工具在现行树中的存续——**未核实**，不作断言。
**归属链接**：../crates/engine/guard、../docs/status/claims.yaml、../docs/03-reference/test-baseline.md、../docs/04-internal/backend-enhancement/00-backlog.md、../AGENTS.md；历史 分支 `feature/cognitive-infrastructure-vnext`

### Guard 3.3 Trace Fidelity and Semantic Generalization (2026-09-08)

**来源**：`reports/guard-v3.3-trace-fidelity-and-generalization.md`（约 125 行）｜分支 `feature/cognitive-infrastructure-vnext`｜Recorded HEAD `9289f4f0c0e729f4be62500403bdab65b7d75c4b`。
**结论/要点**
- **判定（三态 + 一态）**：`REAL_SHADOW_EVIDENCE = UNAVAILABLE`、`CI_VERIFIED = NO`、`FREEZE_READY = NO`、`NOT_FREEZE_READY`。原文定性：**保守证据判定；合成评估与本地测试不能替代真实 shadow 流量或与最终实现 SHA 绑定的成功远端 CI**。
- **轨迹与动作保真**：场景 runner 现为**每个治理动作**保留一个 `ScenarioActionResult` 与一个 `FeatureSnapshot`；**稳定显式动作 ID 被保留，空 ID 获确定性生成 ID**；`BehaviorChain::action_by_id` 提供精确查找；一个 turn 创建一个意图 envelope，**一个 session 跨 turn 复用**以保持会话历史与跨轮摘要可观测；稀疏 `trace_index` 保留同 trace 分组并分离一个 turn 内的 provider trace；**检查执行把缺失与意外的语义效果报为 `ScenarioTraceError`**；`ScenarioOutcome` 上的兼容字段仍是 last-action/last-turn 摘要（旧消费者用），**不视为动作级 ground truth——`execution_trace.actions` 与 `execution_trace.turns` 才是保真权威记录**。
- **独立语义 oracle**：`SecurityScenarioOracle` **独立于**生产解释器、对齐 guard、FeatureV2 抽取器、快照、分类器预测与 Guard 决策；运行时语义效果由结构化 operation/resource/source/sink 字段与显式 effect flag 推导，**不透明效果指纹不是语义真相**；期望效果与运行时效果按动作**精确归一化并对账**。
- **聚焦 Rust 覆盖通过**：**20 个 Guard 正确性测试 + 8 个 Guard 评估完整性测试**；覆盖每个动作的 oracle 标签、稳定动作查找、重复/空 ID、空场景、未知能力、trace 边界、round overflow 安全、多动作 trace、跨轮历史。
- 另含 Dataset and provenance（Rust 导出器产物）、Holdouts and pair evaluation、Shadow/privacy/drift、CI and hygiene、Final Closure (2026-09-08) 的本地验证证据与最终证据边界。
- **5 项 Deferred**：① 取得获批真实 shadow 证据并跑隐私保护漂移分析；② 为反事实准确性加**独立转移标签**；③ 加**效果等价组隔离**与完整实现方向指标；④ 加**工具身份移除消融**与小样本置信区间；⑤ 为最终 SHA 跑并记录远端 CI。**最终判定：`NOT_FREEZE_READY`**。
**现状复核（2026-10-23）**：**已过时（分支与 HEAD）**——分支与 `9289f4f0…` 属 2026-09；现行 HEAD 已在 2026-10 线（当前 `0820d309`）；Guard crate 现为 [crates/engine/guard](../crates/engine/guard)。**已过时（远端 CI 阻塞）**——「`CI_VERIFIED = NO`」中的「远端未跑」部分已被现行 [claims.yaml](../docs/status/claims.yaml) 的「远程 CI 首跑见 PR #36」等记录部分消解；但**真实 shadow 证据与轨迹保真演练未完成的同类缺口仍在**（`gov.chain_drill_ci` 远端 0 次执行）。**仍成立（方法论，本篇是「保真 vs 摘要」原则的最强样本）**：① **「兼容字段是摘要、不是 ground truth；动作级记录才是权威」**——这一条与现行 [test-baseline.md](../docs/03-reference/test-baseline.md) §2「套件 = 测试二进制目标计数、例 = 用例数」的分层口径、`docs_truth.py --verify` 的 `sources`/`verified_at` 分离机制同构，是**「同名不同义」防混谈**的范本；② **「oracle 必须独立于被测系统」**（独立于生产解释器、对齐 guard、抽取器、分类器预测与决策）——现行 `docs_truth.py --verify`（claims 证据核对 + staleness 计算）正是这一原则的工程化；③ **「不透明指纹不是语义真相」**；④ **「保守判定：合成评估 + 本地测试 ≠ 真实流量」**。**未核实**：本篇的 `SecurityScenarioOracle` / `ScenarioTraceError` / `execution_trace` 结构在现行树中的存续——**未核实**，不作断言。
**归属链接**：../crates/engine/guard、../scripts/docs_truth.py、../docs/status/claims.yaml、../docs/03-reference/test-baseline.md、../docs/04-internal/backend-enhancement/00-backlog.md、../AGENTS.md；历史 分支 `feature/cognitive-infrastructure-vnext`

### SSE CoT `delta.reasoning_*` 验证报告 (2026-08-19)

**来源**：`reports/audit-2026-08-19/MiniMax_reasoning_verification.md`（约 175 行）｜验证日期 2026-08-19 21:12–21:14 (CST, +08:00)｜验证目的 = 解除 companion_serve CoT 流的 **P0 阻塞**。
**结论/要点**
- **API key 处理（安全自证）**：仅读取进 `$env:APEIRETH_API_KEY`，**不输出内容**；报告只记 path / 长度 / 前 5 字符（path = `C:\Users\31683\.openclaw\apikey.txt`，长度 125 字符，前 5 字符 `sk-cp`）；验证结束已 `Remove-Item Env:APEIRETH_API_KEY` 清场。
- **结论：⚠️ 部分支持**——该模型**没有** OpenAI 风格的独立 `delta.reasoning_content` / `delta.reasoning` 字段；CoT 被**嵌入在 `delta.content` 字符串内的 `<!-- reasoning -->`（XML 注释样式）区段**，**在普通 content 之前输出**，之后是正式回复。
- **实际 chunk 形态（原文给 JSON 样本）**：`{"choices":[{"index":0,"delta":{"content":"<!-- ","role":"assistant"}}]}` → `{"content":"We need answer",...}` → `{"content":"... -->\n\n按**小数**",...}`。另含 §2 典型 chunk 样本（Test 1，9.11 vs 9.9，stream=true，无 reasoning_effort）。
- **§3 参数兼容性**、**§4 稳定性**、**§5 字段映射表**（给 companion_serve 实现用）、**§6 companion_serve 实现建议**、**§7 风险与遗留**（含风险等级 low：内容内出现字面 `<!--` / `-->`（如用户 prompt 含 HTML 注释）——极少见，可允许误判，因响应正文出现 `<!--` 概率 < 0.1%，误判只影响一次 cot/text 分类；**none：API key 没进 commit / log / 报告，已清场**）。
- **工程规范自检 5 项全 ✓**：0 触碰源文件 ✓ / 0 commit ✓ / 0 修改 working tree ✓ / 报告落地唯一文件 ✓ / API key 内容未进报告 ✓。
**现状复核（2026-10-23）**：**已过时（对象与外部接口面）**——companion_serve / CoT 流属 2026-08 的外部 provider 兼容问题；该报告所涉的 provider 适配面在现行树中**未核实**是否有对应实现（**不作断言**）。**已过时（凭据处理面）**——现行凭据治理已换代：按 [claims.yaml](../docs/status/claims.yaml) 与 [00-backlog.md](../docs/04-internal/backend-enhancement/00-backlog.md) D 行，凭据面为 `apeireth-credentials` 三级 keyring + **热更凭据链已接线**；本篇「把 key 读进 env 再清场」是临时验证手法，现行口径是**凭据不进 env 文本、不进日志、不进报告**。**仍成立（方法论，本篇最值得保留的三条）**：① **「只记 path / 长度 / 前 5 字符，不记内容」**的脱敏取证手法——与现行 [evidence-retention-policy.md](../docs/04-internal/evidence-retention-policy.md) §3「只写结论、不写过程」同向，且与 `commit.ps1` 的 staged secret 扫描互补；② **「结论写『部分支持』并给出降级实现路径（字段映射表 + 实现建议）」**——即不假装「支持」也不草率写「不支持」，而是给出可落地的映射与已知误判概率（< 0.1%，只影响一次分类）；③ **「0 触碰源文件 / 0 commit / 0 改工作树 + 唯一产物」自检 5 项**——与本册其余各篇同一自律范式，也是现行 [AGENTS.md](../AGENTS.md) §1 的直接前身。**未核实**：§5 字段映射表与 §6 实现建议在现行 provider 适配层中的落地情况——**未核实**。
**归属链接**：../docs/04-internal/evidence-retention-policy.md、../docs/04-internal/backend-enhancement/00-backlog.md、../docs/status/claims.yaml、../AGENTS.md；历史 `crates/apeireth-companion/**`、companion_serve 实现面；同册相关 audit-2026-08-19/README_audit_batch_1.md 等（册 03）

### round15-01: apeireth-asi R-Measure ML 在线校准循环 — 验收报告 (2026-08-02)

**来源**：`reports/round15-01-asi-ml-calibration-acceptance.md`（约 230 行）｜backend_engineer｜任务 ID `8f4fc118-9574-4225-8466-812567b0a9e5`｜R-Cycle R14｜状态 ✅ 完成 / 验收通过｜依据 `APEIRETH-FINAL-CHECK-2026-07-31.md` §ASI-06。
**结论/要点**
- **任务范围 9 项**：① `CalibrationLoop` trait（输入历史 trace + 用户反馈，输出调整系数 scale + offset）；② `DriftDetector`（某维连续 N 次偏离 baseline **> 2σ** 触发告警）；③ `RecalibrationScheduler`（每 M 次测量自动跑一次轻量校准，**默认 M = 100**）；④ 在线自适应：用 `AdaptiveBaseline`（**EMA-based**）滚动均值替代静态 baseline；⑤ integration test ≥ 5 场景；⑥ `apeireth-cli asi calibrate` 子命令（dry-run + apply）；⑦ **纯 Rust 实现，0 外部 ML / Python 桥**；⑧ **8/8 integration tests + 19 个 unit tests 全绿**；⑨ 本验收报告（≥1500 字节）。
- **产出清单**（逐路径 + 行数 + 说明）表；**设计要点 6 节**：3.1 校准系数 scale + offset **clamp 到 [0, 1]**；3.2 `CalibrationLoop` trait；3.3 `DriftDetector`；3.4 `RecalibrationScheduler`；3.5 `AdaptiveBaseline`；3.6 CLI `asi calibrate`。
- **测试结果 4 节**：4.1 单元测试（新增 **19** 个，全绿）；4.2 集成测试（**8** 个场景，全绿）；4.3 端到端 demo；4.4 CLI 验证（干跑 `--dry-run --every 100` / 真实写 `--apply --every 100 --scope thread_continuity`）。
- **DoD 自评**一节 + **workspace build 状态**（§6.1 直接影响的 crates）；末尾给出可复制的验证命令序列（`cargo test -p apeireth-asi --lib` / `--test integration_calibration` / `cargo run -q -p apeireth-asi --example calibrate_demo` / CLI 干跑与真实写）。
**现状复核（2026-10-23）**：**已过时（crate 已移出 + 数字口径换代）**——`crates/apeireth-asi` 现位于 [legacy/donor/apeireth-asi](../legacy/donor/apeireth-asi)；「19 unit + 8 integration 全绿」为其历史口径，现行真源 341 套件 / 8570 passed / 0 failed / 51 ignored（[test-baseline.md](../docs/03-reference/test-baseline.md) §1）。**已过时（ASI 维数口径）**——本篇承接 round10-12 的 **V0.5 24 维 + V1136 9 子测度**；现行 [00-backlog.md](../docs/04-internal/backend-enhancement/00-backlog.md) D24 / [ROADMAP.md](../ROADMAP.md) 记载生物机制族 14 条全部采纳（owner 2026-10-19），且现行能力矩阵 §0.2 记载 bio 观测面已接 **14/14 格**（W3.6 迟绑定 5 格 + W3.7 工具生态 1 格 + R1b-2b 生命周期 1 格），故原「24 维 ASI + 校准循环」的门面已换代。**仍成立（方法论）**：① **「校准系数 clamp 到 [0,1]」与「> 2σ 漂移告警」「EMA 滚动基线」**——是可复用的在线校准设计，现行 bio 观测面（`capabilities.bio_mechanisms` 子对象、21 节合同）的「真实数据供给」原则与之同源；② **「纯 Rust、0 外部 ML / Python 桥」**——与现行 [AGENTS.md](../AGENTS.md) §5「0 引新外部 dep」纪律一致；③ **「dry-run + apply 双档 CLI」**——与现行治理链「写面每次人批」的默认取向一致（[claims.yaml](../docs/status/claims.yaml) `tool.task_board`：写面每次人批、失败只能人判、无 failed/retry 动作面）。**未核实**：`apeireth-asi` 在 `legacy/donor` 快照中是否保留校准循环实现——**未核实**，不作断言。
**归属链接**：../legacy/donor/apeireth-asi、../docs/03-reference/test-baseline.md、../docs/04-internal/backend-enhancement/00-backlog.md、../docs/status/claims.yaml、../AGENTS.md；历史 `crates/apeireth-asi/**`、`crates/apeireth-cli` 的 `asi calibrate` 子命令；同册相关 A10-deep-cognition-database-engineer.md（册 01 §1，本篇是其引用的「round15 后续补完」之一）

### round15-02 apeireth-bus 5 层通信总线 — 验收报告 (2026-08-03)

**来源**：`reports/round15-02-bus-5-layer-acceptance.md`（约 135 行）｜leader / backend_engineer2｜Task ID `ad3aadd8-6c26-447a-b469-381f13f88ce7`｜Commit `305c06f1`。
**结论/要点**
- **目标**：新建 `crates/apeireth-bus/`，实现 5 层通信总线（L0 inproc / L1 UDS / L2 pipe / L3 gRPC / L4 WebSocket），覆盖 3 种模式（pub-sub / req-rep / streaming），**bounded channel + 反背压 + 丢弃策略 + Trace ID 链路追踪**。
- **DoD 10 项全 ✅**（逐项给证据）：① Cargo.toml 70 行（tonic / prost / tokio / tokio-tungstenite / async-tungstenite / serde_json / rmp-serde / jsonschema / bincode / thiserror / anyhow）；② 已注册 workspace members；③ **L0** `src/l0.rs` 9142 bytes（tokio broadcast + mpsc + watch_set/watch_get 快照 + `BackpressurePolicy` 4 变体）；④ **L1** `src/l1.rs` 11136 bytes（`#[cfg(unix)]` 守门 + `UnixListener/Stream` + bincode serde）；⑤ **L2** `src/l2.rs` 9466 bytes（stdin/stdout pipe 跨平台子进程 + serde_json + rmp-serde）；⑥ **L3** `src/l3.rs` 14860 bytes（tonic + prost + `proto/bus.proto`，BusService + BusMessage schema）；⑦ **L4** `src/l4.rs` 15721 bytes（async-tungstenite + tungstenite 0.25 `Message::text(Utf8Payload)` + jsonschema 0.28 + `MaybeTlsStream`）；⑧ 3 种模式每层都支持（`BusTransport` trait 含 publish/subscribe/request/stream 4 接口）；⑨ `BackpressurePolicy`（Block / DropOldest / DropNewest / Drop）+ tokio bounded mpsc **1024 capacity**；⑩ `BusMessage.trace_id` + 跨层 trace_id 分配器 + `BusStats` 原子计数；⑪ **integration test ≥ 15**：`tests/integration.rs` 387 行 = **16 tests 全绿**（15 集成 + 1 doc）；⑫ **0 PyO3 / 0 Python / 0 cffi，纯 Rust**；⑬ `cargo build --workspace` 0 error（仅 prost 自动生成警告）+ bus 15 lib tests 全绿；⑭ 本报告 ≥1500 字节。
- **文件清单：10 文件 / 2676 行**。**关键设计要点 5 节**：3.1 统一 `BusMessage<T>`；3.2 `BackpressurePolicy` 反背压；3.3 平台兼容性；3.4 `BusStats` 原子计数；**3.5 已知局限（诚实登记）**。**与 R11 baseline 对照**一节。
- **后续轮次建议 5 条**：① L1/L2/L4 真实网络端口 e2e 测试（**当前仅 mock**）；② L3 gRPC server 实际启动 + 客户端连接 roundtrip；③ TLS / WebAuthn 集成（若未来需要）；④ 与 `apeireth-supervisor` / `apeireth-central` 集成（事件总线 + 进程间通信）；⑤ 反压策略运行时动态切换 `BackpressurePolicy`。**报告 commit：待 push**。
**现状复核（2026-10-23）**：**已过时（crate 已移出 + 数字口径换代）**——`crates/apeireth-bus` 现位于 [legacy/donor/apeireth-bus](../legacy/donor/apeireth-bus)；「16 tests 全绿 / 10 文件 2676 行 / 1024 capacity」为历史口径；现行真源 341 套件 / 8570 passed / 0 failed / 51 ignored（[test-baseline.md](../docs/03-reference/test-baseline.md) §1）。**已过时（总线面换代）**——本篇「5 层通信总线」在现行树**无对应登记面**；现行事件/总线口径是 R1b 的 `nervous.bus`（见 [../docs/adr/0009-r1b-nervous-bus.md](../docs/adr/0009-r1b-nervous-bus.md)）与 R7 上下文会话统一（[../docs/adr/0007-r7-context-session-unification.md](../docs/adr/0007-r7-context-session-unification.md)）；且 [legacy/README.md](../legacy/README.md) 明定「不移植 `legacy/` 的架构，只移植语义」。**仍成立（方法论）**：① **「schema 层与物理实现分层声明」**——本篇 §3.5 已知局限与册 02 的 A16「`is_fully_implemented()` 仅对 L0 为真」是同一纪律（**只对 L0 声称物理实现，其余层标 mock / 待接**）；② **「反背压四策略 + bounded channel」**与现行生产链上的限流/配额面（[00-backlog.md](../docs/04-internal/backend-enhancement/00-backlog.md) A 行配额三维计量：token/步数/花费）同属资源治理；③ **「Trace ID 链路追踪」**——现行有对应面（`trace.read` / `audit.read` / `activity.sse`，见 [capabilities-matrix.md](../docs/03-reference/capabilities-matrix.md) §0）；④ **「诚实登记 3.5 已知局限 + 后续 5 条建议」**。**未核实**：L1–L4 在 `legacy/donor/apeireth-bus` 快照中是否仍为 mock 实现——**未核实**。
**归属链接**：../legacy/donor/apeireth-bus、../docs/adr/0009-r1b-nervous-bus.md、../docs/adr/0007-r7-context-session-unification.md、../docs/03-reference/capabilities-matrix.md、../docs/04-internal/backend-enhancement/00-backlog.md、../docs/03-reference/test-baseline.md、../legacy/README.md；历史 `crates/apeireth-bus/**`；同册相关 achievement-A16-mcp-integration-expert.md（册 01 §19）

---

## 现状对照（2026-10-23）— 本册 7 篇横向口径

| 维度 | 原篇口径 | 现行口径（真源） | 判定 |
|---|---|---|---|
| 守门轮次 | G3.0 / G3.1 / G3.2 / G3.3（分支 `feature/cognitive-infrastructure-vnext`） | 分支已不在现行线；Guard crate 现为 crates/engine/guard | 已过时 |
| 三态判定纪律 | `FREEZE_READY=NO` / `CI_VERIFIED=NO` / `NOT_FREEZE_READY` | **仍成立**（现行 claims 逐条标 level + `verified_at` + `notes` 残余） | 仍成立 |
| 「未跑的门禁不写成绿」 | G3.0 明列 5 项未跑；G3.3 拒绝以合成评估替代真实流量 | **仍成立**（AGENTS.md §1 + 债栏 D28/D30/D31 环境性失败如实登记） | 仍成立 |
| 远端 CI | G3.1/G3.2/G3.3 均 `CI_VERIFIED = NO` | 部分消解（claims 记「远程 CI 首跑见 PR #36」，桌面 gate 3/3 success）；**同类阻塞仍在**（`gov.chain_drill_ci` 远端 0 次执行、步骤被 skip） | 部分成立 |
| Shadow 与 Enforce 分离 | Shadow 永不改 Decision、运行时默认 disabled classifier | **仍成立**（`gov.reflection_queue` IMPLEMENTED / 生产未启用 / 零行为变化） | 仍成立 |
| Oracle 独立性 | `SecurityScenarioOracle` 独立于生产解释器与决策 | **仍成立**（`docs_truth.py --verify` = claims 证据核对 + staleness 计算） | 仍成立 |
| 术语纠正纪律 | within-family instance split 不得称 unseen-family | **仍成立**（test-baseline §2 / §3 分层与历史口径表） | 仍成立 |
| ASI 维数 / 校准 | V0.5 24 维 + V1136 9 子测度 + ML 在线校准 | 生物机制族 14 条已采纳，bio 观测面 14/14 格 | 已过时 |
| 5 层总线 | L0 物理实现 + L1–L4 schema | 现行总线面 = R1b nervous.bus + R7 会话统一；无「5 层」登记面 | 已过时 |
| 外部 provider CoT 字段 | 实测映射表（嵌入 XML 注释式） | 无现行对应登记面 | 未核实 |

**本册复核统计**：7 篇中标「**已过时**」结论 **12 条**、「**仍成立**」**14 条**、「**未核实**」**6 条**。

*本册为 `reports/` 合册产物；来源 7 篇与分配表 `doc-audit/reports-assignment.txt` §04 一致（逐篇唯一归册）。历史内容按原文保留、不追溯改写；现行口径一律另起「现状对照（2026-10-23）」并指明真源。*
