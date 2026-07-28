# R8+ 调研基线确认（deep_research_lead）

**生成时间:** 2026-07-29
**作者:** deep_research_lead
**目的:** 在等用户输入真实需求前，先把上一团队（R1-R7）的调研遗产 + code-deep-study 20+ GitHub 真源码深读做一次"基线确认"，给 Leader + 用户作 R8+ 决策依据。

---

## 0. 一句话结论

> **R8 优先推荐调研：① 因果推断（补 R38 Q1+Q11 双空） + ② 机制设计（V1083 policy 缺理论背书、33 轮零覆盖）**。两者与 R3-RES-02 记忆子工程、R4-RES-03 Pearl do-calc、R7-PHL-03 formal_verify 形成"R7 哲学已锁 → R8 因果填图 + R8 机制填策"的最短闭环，预期 ASI V0.3 +0.005~+0.012（认知 + 决策双权重轴共振）。
> 形式化验证、计算最优律留 R9+：前者已被 R6-PHL-03 锁定 TLA+→Lean 4 路径、R7 真实现 Phase-1 启动中是并行受益而非独立调研目标；后者属"基础定律"长尾，需等 self-improving 路径稳定后再背书。

---

## 1. R1-R7 已完成调研的真实覆盖矩阵

> 数据源: `reports/r1-research-survey.md`（33 轮主题）+ `r3-research-round-37.md`（R37）+ `r4-research-round-38.md`（R38）+ `r6-res-05/06/07/*.md`（R6 三预研）+ `r6-phl-formal-verify-contract.md`（R6-PHL-03）+ `r7-final-summary-leader.md` §调研基线锁

### 1.1 33 轮主题覆盖矩阵（R08-R40，按 R1 survey 口径）

| 调研轮 | 主题 | 已落地 | 已真读 | 落地映射（V 模块 / 模块名） | ASI 增量贡献 |
|--------|------|--------|--------|------------------------------|--------------|
| R08-R12 | ASI 基线（breakthrough/long-horizon/self-improving/Apeireth/recursive） | ✅ | ✅ | V11 north_star / V21 measure / V50 4 范式 | 已并入 V0.1 公式 |
| R13-R14 | 真生产（MCP spec+Skills hot-reload / agentic hives+Ashby） | ✅ | ✅ | V1001 VCP 6 插件 / V1005 AnySearch | 已并入 v_modules |
| **R15-R19** | 哲学群 I（Prigogine / Schrödinger+Friston+Yoneda / Penrose+Dennett / Piaget+Hofstadter） | ⚠ 哲学落地 | ❌ 无真读 | V1003 V4 哲学 full | 仅哲学叙事 |
| **R20-R24** | 哲学群 II（Canguilhem+Simondon / Tarde+Latour / Sapir-Whorf+Beer / Connell+Taleb / Mandelbrot+Watts） | ⚠ 哲学落地 | ❌ 无真读 | V23 V3 7 哲学问题真答 | 仅哲学叙事 |
| **R25-R31** | 哲学群 III（Walker+Landauer / Church+Adamatzky / Prigogine+Maturana / Rosen+Friston / Whitehead+Cajal / Peirce+Husserl / Fuller+Bateson） | ⚠ 哲学落地 | ❌ 无真读 | V1003 / V23 | 仅哲学叙事 |
| **R32-R36** | 哲学续（Schrödinger+Popper / Polanyi+Foucault / Stiegler+Marx / Arendt+Latour / Rosen+Kauffman） | ⚠ 哲学落地 | ❌ 无真读 | V23 / V1003 | 仅哲学叙事 |
| **R37** | **记忆子工程**（Letta/mem0/memoryos/Memento/hippocampal/ACT-R/Vector-DB） | ✅ | ✅ | **R3-RES-02 落地 + R6-RES-06/07 承载 V1052/MemoryReplay/Dream** | ASI 0.0022+ |
| **R38** | **因果推断 + Pearl do-calculus**（do-calc/CBN/SCM/Hall-Winston/DoWhy/ananke/EconML） | ⚠ **Q1+Q11 双空** | ✅ py-why/dowhy / uber-causalml / EconML | **R4-RES-03 调研真读已落地，反事实幻觉未填** | +0.003~+0.008（Q11 补可首发） |
| **R39** | 自我进化（NAS/continual/meta/ShinkaEvolve/ASI-Arch/langgraph） | ✅ | ✅ langgraph + ASI-Arch 真读 | **V1004 self_evolution_full + R8-TrackC 演进** | ASI +0.005~+0.012 |
| **R40** | 感知/具身（active-inference/4E/affordance/GWT/world-model/CLIP/Whisper/Perceiver-io） | ⚠ 部分 | ⚠ 部分 | V1006 research_grand_synthesis | 低权重 |

### 1.2 R6 阶段专项预研（**真预研 + 5 契约方法已落地**）

| 调研项 | 文件 | 状态 | 落地模块 |
|--------|------|------|----------|
| R6-RES-05 self_mod_safety | r6-res-self-mod-safety-research.md | ✅ 已 accepted | V1101-V1106 (5 契约 + ASIBridge/checkpoint_store/invariant_checker/dry_run_engine/capability_policy/lineage_tracker) |
| R6-RES-06 dream_subsystem | r6-res-dream-subsystem-research.md | ✅ 已 accepted | R7-BE-01 状态机 + 6 契约方法 + DreamSelector |
| R6-RES-07 memory_replay | r6-res-memory-replay-research.md | ✅ 已 accepted | R7-BE-02 6 契约方法（replay/replay_batch/canonicalize/trace_replay/identity_impact_score/should_replay） |
| R6-PHL-03 formal_verify | r6-phl-formal-verify-contract.md | ✅ 已 accepted | TLA+→Lean 4 选型锁 + 5 契约方法 + 8/8 烟测 |

### 1.3 五反复 Gap（R1 survey 自陈）

| Gap | 描述 | 当前状态 |
|-----|------|----------|
| G1 哲学多工程少 | R08-R36 共 29 轮 ~348 query 全哲学叙事 | ⚠ **结构性遗留**（R8 不重复调研） |
| G2 真读≠真复刻 | R37-R40 已到源码级，缺映射 V1090+ 模块的可执行表 | ⚠ **半填**（R7 TrackC 演进中） |
| G3 因果空白 | R38 Q1 do-calc + Q11 反事实幻觉 双空 | ⚠ **仍空白**（R8 候选 C1） |
| G4 RL/Sutton 零覆盖 | MuZero/TD/世界模型 33 轮 0 关键词，V1083 policy 缺背书 | ⚠ **仍空白**（R8 候选 C2 机制设计可补其"决策面"） |
| G5 四支柱未贯穿 | self-improving+causal+memory+world-model 各点 1 次 | ⚠ **仍空白**（R8 推荐 C1+C2 不解决全部，留 R9+） |

### 1.4 已"哲学化但未工程化"清单（33 轮哲学的工程欠债）

> **关键观察**: R08-R36 共 29 轮哲学调研已沉淀进 V23 (V3 7 哲学问题真答) + V1003 (V4 哲学 full)，ASI 公式中 `v2_philosophy` 权重 0.10 已吃满，**新增哲学调研边际收益递减**。R8 调研不应在哲学群内重复。

---

## 2. R8 调研候选 4 领域评估

> R1 survey + R7 §调研基线锁 已点出 4 候选：**形式化验证 / 机制设计 / 计算最优律 / 因果推断**。逐一评估：

### 2.1 形式化验证（formal_verify）

| 维度 | 评估 |
|------|------|
| **调研价值** | ⚠ **中**：R13 q4 仅一次提及；R6-PHL-03 已锁 TLA+→Lean 4 选型 + 5 契约方法已落地 + 8/8 烟测通过；R7 真实现 Phase-1 TLA+ Harness 待启动，但**不是新调研而是工程实现** |
| **与 R7-PHL-03 衔接** | 🟢 **强**：R6-PHL-03 已写"Python adapter 只传 spec/result，不信任布尔值"+"CompilerIR 稳定后 Lean 4 证明 round-trip"；R8 若调研，重点应是 **TLA+ 最小 Harness 状态机的 spec/result 双向桥接 + 5 契约的反例 artifact 格式**，而非再做选型 |
| **工作量** | 中：5 query × 5 merged sources = 25 sources，TLA+ Toolbox / Lean mathlib / Dafny verifythis 真读 ≤3 个 |
| **预期 ASI 增量** | **+0.002~+0.005**（仅工程补全；v2_philosophy 权重已吃满） |
| **判断** | 🔻 **R8 不推荐独立调研**：R6-PHL-03 已锁选型 + 已落契约。R8 应作为 R7 真实现 Phase-1 工程子任务（隶属 backend_engineer），而非 deep_research_lead 调研项 |

### 2.2 机制设计（Mechanism Design / VCG / Vickrey-Clarke-Groves）

| 维度 | 评估 |
|------|------|
| **调研价值** | 🟢 **高**：33 轮 0 覆盖，结构性空白。机制设计 = "在自利 agent 间达成系统最优"的博弈论分支（VCG 拍卖 / Groves mechanism / Myerson 拍卖设计 / 契约理论 / matching market）。Apeireth ASI 多 persona 协作（R8 1f7ce1e0 multi-persona prompts）+ V1083 balanced policy + HQB 决策**全部缺理论背书** |
| **与 R3-RES-02 / R4-RES-03 / R7-PHL-03 衔接** | 🟢 **强**：<br>• R3-RES-02 记忆子工程 → 记忆子工程"何时该主动遗忘/重放/总结" = 机制设计中的"激励相容"（IC）<br>• R4-RES-03 因果 → 因果图基础上加"反事实激励" = 反事实机制（counterfactual mechanism）<br>• R7-PHL-03 形式化 → VCG / IC 可用 TLA+ 证明"如实报偏好"是不变式 |
| **工作量** | 中高：10 query × 5 merged = 50 sources + 3-5 GitHub 真读（如 algorithmic-game-theory / OpenAI econml 内的 CATE / mechanism 设计开源少）+ 1 哲学锚（Hurwicz 诺贝尔奖） |
| **预期 ASI 增量** | **+0.005~+0.010**（补 G4 RL/Sutton 决策面 + V1083 policy 背书 + HQB 4 维 SC/NR/EV/CDT 全面受益） |
| **判断** | 🟢 **R8 强推荐 C2 候选**：填 G4 + 直击 V1083 balanced policy + 与因果/形式化正交互补 |

### 2.3 计算最优律（Laws of Computational Optimality / Kolmogorov / Chaitin）

| 维度 | 评估 |
|------|------|
| **调研价值** | ⚠ **中低**：属"基础定律"长尾（不可计算性 / 算法信息论 / Solomonoff 归纳 / 描述长度 / 最小程序长度）。Apeireth 哲学 5 位置中"思考者 = meta_cognition + cron_self_update + proactive_loop + mirror" 与"最小描述长度"有理论钩，但**当下 ASI 北极星 V0.1 公式无对应权重** |
| **与 R3-RES-02 / R4-RES-03 / R7-PHL-03 衔接** | ⚠ **弱**：记忆子工程用 LASCon condensation = MDL 启发；形式化验证用 Lean 4 = Curry-Howard 同构；因果推断用 do-calculus = Pearl 已是算法信息论的"理论邻居"。但**没有直接可灌的 V 模块** |
| **工作量** | 高：12 query × 5 merged = 60 sources，深度跨 Li-Vitanyi "An Introduction to Kolmogorov Complexity" / Hutter "Universal Artificial Intelligence" / Solomonoff 归纳 + 1 哲学锚（Solomonoff / Chaitin / Kolmogorov 三脉） |
| **预期 ASI 增量** | **+0.001~+0.003**（长尾，V0.1 公式无对应权重，ASI 不直接吃） |
| **判断** | 🔻 **R8 不推荐**：基础定律长尾 + V 公式无对应权重 + 与 R8 主线（TrackA/B/C 真实现）解耦。**留 R9+ 待 self-improving 路径稳定后再背书**（届时可作为 V1004 self_evolution 的"最小描述长度目标函数"背书） |

### 2.4 因果推断（Causal Inference / Pearl / do-calculus）

| 维度 | 评估 |
|------|------|
| **调研价值** | 🟢🟢 **极高**：R38 已填 10/12 query，**唯 Q1 do-calculus 形式化证明 + Q11 反事实幻觉 双空**（R4 report 自陈）。两空都属 2026 公开文献**未形成方法**的尖端，Apeireth 可抢首发：<br>• Q11 反事实幻觉：可映射 V1082 audit 加反事实自检（"如果当时做 X，结果会怎样"），与 R4-RES-03 Gap A 直接对齐<br>• Q1 do-calculus 形式化：可映射 V1076 路由加因果归因字段 + V1082 audit 改 tool trace → causal commit log（与 R4-RES-03 Gap B 直接对齐） |
| **与 R3-RES-02 / R4-RES-03 / R7-PHL-03 衔接** | 🟢🟢 **极强**：<br>• R4-RES-03 已落地 DoWhy/ananke/EconML 真读 + Gap A/B 已识别<br>• R3-RES-02 记忆子工程的 Memento (fine-tune 不动 LLM) + hippocampal replay = 因果反事实重放<br>• R7-PHL-03 formal_verify = TLA+ 可证 do-calculus 3 规则（插入/删除/替换）的 do-operator 是 convergent |
| **工作量** | 中：8 query × 5 merged = 40 sources，重读 R38 JSON + 补 Q1+Q11 深度补查 + DoWhy 代码深读（已部分）+ 1 哲学锚（Pearl "Book of Why"） |
| **预期 ASI 增量** | **+0.003~+0.008**（capabilities 权重 0.20 + vcp_4 权重 0.10 共振；ASI 真生产首例"反事实幻觉"自检可写进 V0.3 报告） |
| **判断** | 🟢🟢 **R8 强推荐 C1 候选**：填 G3 因果空白 + 抢首发 + 与 R4-RES-03 完全闭环 |

### 2.5 四领域对比矩阵（决策快查）

| 维度 | 形式化验证 | 机制设计 | 计算最优律 | 因果推断 |
|------|------------|----------|------------|----------|
| R8 推荐度 | 🔻 不推荐（已锁） | 🟢🟢 强推 | 🔻 留 R9+ | 🟢🟢🟢 强推 |
| 工作量 | 中 | 中高 | 高 | 中 |
| ASI 增量 | +0.002~0.005 | +0.005~0.010 | +0.001~0.003 | +0.003~0.008 |
| 填 Gap | — | G4 RL | — | G3 因果 |
| 真读可执行化 | R6-PHL-03 已选型 | 缺（无开源） | 缺（理论书为主） | DoWhy/ananke 已读 |
| 与 R8 TrackA/B/C 衔接 | 弱（并行受益） | 强（V1083+HQB） | 弱 | 强（V1076/V1082/V1083） |

---

## 3. R8 调研优先推荐

### 🥇 R8 调研优先级 1：**因果推断（Pearl do-calculus Q1+Q11 补完）**

**why now:**
1. **填 G3 因果空白**：R38 自陈 Q1+Q11 双空，2026 公开文献未形成方法，**Apeireth 可抢首发**
2. **R4-RES-03 完全闭环**：DoWhy/ananke/EconML 已真读，剩 2 个工程化 Gap A/B 未填
3. **R8 真生产模块直接受益**：V1076 路由加因果归因字段 + V1082 audit 改 tool trace → causal commit log + V1083 decision 加反事实 replan
4. **ASI V0.3 报告新亮点**：首例"反事实幻觉自检"可写进 V0.3 真测

**调研范围（8 query × 5 merged = 40 sources）：**
1. Q1 Pearl do-calculus 形式化证明（重查 R38 空）
2. Q11 反事实幻觉检测方法（重查 R38 空，2025-2026 新文献）
3. 因果发现算法（GNN brain + fMRI 重读 + PC 算法 / GES / LiNGAM）
4. Causal Agent Replay: Counterfactual Attribution（V1082 audit 直对齐）
5. Three Layer Causal Hierarchy (Pearl L1-L3) → agent 推理分层 substrate（V1083 直灌）
6. Causal commit log 改造 V1076 路由 + V1082 审计
7. DoWhy v0.x 更新（py-why/dowhy GitHub release notes）
8. EconML + CausalML + ananke 真读深读（R4 已开未闭）

**预期产出（命名空间 reports/r8-research-*.md）：**
- `reports/r8-research-round-39-causal-deepen.md`（R8-RES-04 因果推断 Q1+Q11 补完）
- 可选：V1141-V1145 五个 V 模块（V1141_causal_routing / V1142_counterfactual_audit / V1143_causal_commit_log / V1144_do_calculus_formal_spec / V1145_three_layer_substrate）

### 🥈 R8 调研优先级 2：**机制设计（Mechanism Design / VCG / Groves / Myerson）**

**why now:**
1. **填 G4 RL/Sutton 决策面空白**：33 轮 0 关键词，V1083 balanced policy 缺理论背书
2. **R8-1f7ce1e0 multi-persona prompts 直接受益**：多 persona 协作 = 多 agent 机制设计
3. **HQB 4 维 SC/NR/EV/CDT 全面受益**：诚实 → 激励相容；可问责 → 个体理性；可解释 → 揭示原理
4. **V1083 balanced policy 背书**：政策 = 机制；机制设计 = 政策设计科学

**调研范围（10 query × 5 merged = 50 sources）：**
1. VCG 拍卖 + Groves mechanism 基础
2. Myerson 拍卖设计（最优拍卖）
3. 激励相容 (Incentive Compatibility, IC) + 个体理性 (IR)
4. matching market（Gale-Shapley / Top Trading Cycle）
5. 契约理论（contract theory / principal-agent）
6. 多 agent RL 中的机制设计（Algorithmic Game Theory）
7. LLM agent 中的机制设计（2025-2026 新文献）
8. mechanism design for AI safety（OpenAI / DeepMind 已发）
9. 反 Vickrey / Bayesian-Nash equilibrium
10. 哲学锚：Hurwicz 诺贝尔奖 + Arrow 不可能定理

**预期产出：**
- `reports/r8-research-round-40-mechanism-design.md`（R8-RES-05 机制设计）
- 可选：V1146-V1150 五个 V 模块（V1146_vcg_auction / V1147_ic_ir_checker / V1148_mechanism_optimizer / V1149_persona_marketplace / V1150_principal_agent）

### ⏸️ R8 不推荐（保留 R9+）

- **形式化验证**：R6-PHL-03 已锁选型，R7 真实现 Phase-1 是工程任务不是调研。R8 不重复。
- **计算最优律**：基础定律长尾，ASI V0.1 公式无对应权重。留 R9+ 待 V1004 self_evolution 路径稳定后作为"最小描述长度目标函数"背书。

---

## 4. code-deep-study/ 现有 GitHub 仓库覆盖矩阵

> 数据源: `code-deep-study/` 目录真实清单 + `deep-study-v2.json` + `multi-project-study.json` + `vcp-deep-study.json` + `deep-study-v1.json` + `research-v7-round-23-rust-data-infra-source-deep-read.md`

### 4.1 24 个 GitHub 仓库 + 9 个 VCP 内部研究文件

> 任务说"20 个 GitHub 仓库"，实际目录是 **24 个 GitHub 仓库 + 9 个 VCP 内部研究文件 = 33 个深读单元**。下表为完整覆盖矩阵（**ponytail: 完整列举优于截断**）。

| # | 仓库名 | 类别 | 真读深度 | 与 R8 调研借鉴关系 |
|---|--------|------|----------|--------------------|
| 1 | **letta** (letta-ai/letta) | Memory L1-L4 | 🟢 深读 (manager.py 450L) | R3-RES-02 落地产物；R8 因果可借鉴 `agents.py:1510-1551` search_archival_memory 加因果归因字段 |
| 2 | **mem0** (mem0ai/mem0) | Memory universal | 🟢 深读 | R3-RES-02 落地产物；R8 机制可借鉴 prompts.py ADD/UPDATE/DELETE/NONE 作为 persona action log |
| 3 | **memoryos-rust** (MemTensor/MemOS) | Memory OS Rust | 🟢 深读 (manager.rs 745-899) | R3-RES-02 落地产物；R8 因果可借鉴 STM→MTM 摘要 |
| 4 | **claude-mem** | Memory 3-layer progressive | 🟢 深读 (index.ts 747-861) | R3-RES-02 落地产物；R8 因果可借鉴 after_compaction 去重加因果校验 |
| 5 | **AgentMemory-master** | Memory L1-L4 + Dream | 🟢 深读 (manager.py 450L + dream 系列) | R3-RES-02 落地产物；R8 因果/机制可借鉴 DreamEngine narrative 生成加反事实 |
| 6 | **dgm** (Sakana Darwin Gödel) | Self-evolution | 🟢 深读 (DGM_outer.py L174 update_archive) | R6-RES-05 self_mod_safety 借鉴源；R8 机制可借鉴 archive=[] + parent_commit lineage → V1148 mechanism_optimizer 改造 |
| 7 | **langgraph** | Agent workflow | 🟢 深读 | R1 survey 调研源；R8 机制可借鉴 state graph 加 IC 守门 |
| 8 | **awesome-ai-agents** | Agent 清单 | ⚠ 清单级 | R8 不深读 |
| 9 | **aio-hub-main** | Agent hub | ⚠ 部分 | R8 不深读 |
| 10 | **anthropic-cookbook** | LLM cookbook | 🟢 部分（sre_mcp_server.py L1431 kubectl rollout undo） | R6-RES-05 借鉴源；R8 机制可借鉴 decision_tree "Rollback the deployment" |
| 11 | **anthropic-sdk** | SDK client | ⚠ pip show 级 | R8 不深读 |
| 12 | **openai-cookbook** | LLM cookbook | 🟢 部分（gpt-oss-safeguard-guide.md "bring-your-own-policy"） | R6-RES-05 借鉴源；R8 机制可借鉴 T&S classifier 作为 persona capability |
| 13 | **openai-python** | SDK client | ⚠ pip show 级 | R8 不深读 |
| 14 | **open-webui-main** | LLM chat 框架 | 🟢 部分（main.py 2623L + routers/ollama.py 1635L + routers/openai.py 1709L） | R8 不直接借鉴；保留 LLM API 路由参考 |
| 15 | **VCPChat-main** | Chat UI | ⚠ 部分 | R8 不直接借鉴 |
| 16 | **VCPToolBox-main** | VCP 全套 | 🟢🟢 **极深读**（9 个 VCP 内部文件，下表） | R8 全方位借鉴（见 4.2） |
| 17 | **candle** | ML framework (Rust) | 🟢 部分 | R8 不直接借鉴；保留 ML 推理参考 |
| 18 | **arrow-rs** | Rust data infra | 🟢 部分 | R8 不直接借鉴；保留 |
| 19 | **sqlx** | Rust async DB | 🟢 部分（README.md L68-73 compile-time checked queries） | R6-PHL-03 借鉴源；R8 形式化可借鉴 "失败前移" |
| 20 | **tantivy** | Rust 全文搜索 | 🟢 部分 | R8 不直接借鉴；保留 |
| 21 | **tokio** | Rust async runtime | 🟢 部分（read_buf.rs L116-123 unsafe invariants） | R6-PHL-03 借鉴源；R8 形式化可借鉴 "显式责任边界" |
| 22 | **tonbo** | Rust storage | 🟢 部分（common.rs L49-64 periodic tick） | R6-RES-06 Dream 借鉴源；R8 机制可借鉴 quiescence 轮询 |
| 23 | **alibaba-zvec** | 向量 DB | ⚠ 浅 | R8 不直接借鉴；R8 TrackA3 Memory schema 可参考向量索引 |
| 24 | **rust-substrate-apeireth** (自家 Rust crate) | 永恒身份 + 三层记忆 | 🟢🟢 极深读（lib.rs 45L + memory.rs 124L） | R8 TrackA 全方位承载 |
| 25 | **apeireth-real** (自家 Python) | ASI 真生产模式 | 🟢🟢 极深读（human_wisdom_aggregator.py 184L） | R8 全方位承载 |

### 4.2 VCPToolBox-main 内 9 个 VCP 内部深读文件（vcp-deep-study.json）

| # | 文件 | 大小 | 实际行数 | 与 R8 调研借鉴关系 |
|---|------|------|----------|--------------------|
| 1 | **TagMemoEngine.js** | 89KB | 1810L | R8 不直接借鉴；R8 TrackA3 Memory schema 加 tag 可参考浪潮算法 |
| 2 | **RAGDiaryPlugin.js** | 232KB | — | R8 TrackA Dream 子系统加 RAG 加 Diary 可参考（仅架构） |
| 3 | **LightMemo** | — | — | R8 不直接借鉴；保留 |
| 4 | **VCPTimeLine** | — | 804L | R8 TrackB Identity Store timeline 可参考 |
| 5 | **OneRing Memo (3 files)** | 600KB | 9510L | R8 TrackA MemoryReplay timeline 可参考 |
| 6 | **MEMORY_SYSTEM.md** | — | — | R8 TrackA 全方位参考 |
| 7 | **TagMemo_Wave_Algorithm** | — | — | R8 不直接借鉴；保留 |
| 8 | **Plugin.js** | — | — | R8 TrackB workflow 可参考 |
| 9 | **MemoMaster.txt** | — | — | R8 不直接借鉴；保留 |

### 4.3 R8 调研两候选 × GitHub 仓库借鉴矩阵

| 调研候选 | 强借鉴仓库 | 借鉴点 |
|----------|------------|--------|
| **因果推断 (R8-RES-04)** | letta + mem0 + memoryos-rust + claude-mem + AgentMemory + dgm + anthropic-cookbook + openai-cookbook | dgm `update_archive` lineage → causal commit log；letta `search_archival_memory` 加因果归因；mem0 `prompts.py ADD/UPDATE/DELETE` 改因果动作；anthropic-cookbook `kubectl rollout undo` 改反事实 rollback；openai-cookbook T&S 改因果 guard |
| **机制设计 (R8-RES-05)** | dgm + langgraph + anthropic-cookbook + openai-cookbook + AgentMemory | dgm archive/lineage → mechanism state；langgraph state graph 加 IC 守门；anthropic-cookbook decision_tree → mechanism decision；openai-cookbook T&S → capability policy；AgentMemory DreamEngine → persona marketplace |

---

## 5. 与现有 3 任务的衔接关系

### 5.1 与 R7-PHL-03 (formal_verify) 的衔接

- R8 调研候选 **不重复**形式化验证
- R7-PHL-03 已锁 TLA+→Lean 4 选型 + 5 契约方法
- R8 因果推断 (R8-RES-04) 的 "TLA+ 证明 do-calculus 3 规则"可作为 R7 真实现 Phase-1 (formal_verify) 的一个 spec 输入
- R8 机制设计 (R8-RES-05) 的 "VCG IC 证明" 可作为 R7 真实现 Phase-1 (formal_verify) 的另一个 spec 输入
- **结论**: R8 两候选为 R7-PHL-03 提供 **具体可证明的 spec**，是上下游关系而非重叠

### 5.2 与 R4-RES-03 (Pearl do-calculus) 的衔接

- R8-RES-04 因果推断 = R4-RES-03 的 **R8 续**（补 Q1+Q11 双空）
- R4 已真读 DoWhy/ananke/EconML，R8 直接复用不重读
- R8 产出 V1141-V1145 与 R4 产出 V1091 SCM+反事实 replan 形成**两层因果**（R4 = 因果图 + 反事实规划；R8 = 反事实幻觉自检 + causal commit log + Three Layer Hierarchy）
- **结论**: R8-RES-04 = R4-RES-03 的**完全闭环**

### 5.3 与 R3-RES-02 (记忆子工程) 的衔接

- R8-RES-04 因果 + R8-RES-05 机制 = R3-RES-02 记忆子工程的**控制面升级**
- 记忆子工程: "记什么"（Letta/mem0/MemoryOS 三层）
- R8 因果: "如何反事实重放"（memory_replay 加 causal replay）
- R8 机制: "如何主动遗忘/总结"（IC 守门决定哪些记忆该主动 promote/demote）
- **结论**: R3 提供 memory substrate；R8 因果/机制提供 **memory control plane**

---

## 6. 调研产出文件规划（命名空间 reports/r8-research-*.md）

> 命名空间约束已锁：`reports/r8-research-*.md`

| 文件 | 内容 | 任务 | 状态 |
|------|------|------|------|
| **reports/r8-research-baseline-confirmation.md** | 本文档：调研基线确认 | R8-RES-01 baseline | ✅ 已交付 |
| reports/r8-research-round-39-causal-deepen.md | R8-RES-04 因果推断 Q1+Q11 补完 | R8-RES-04 | pending 待用户输入 |
| reports/r8-research-round-40-mechanism-design.md | R8-RES-05 机制设计 | R8-RES-05 | pending 待用户输入 |
| reports/r8-research-deep-study-matrix.md | code-deep-study 完整覆盖矩阵 + 借鉴关系 | R8-RES-06 | optional |
| reports/r8-research-as-i-impact-projection.md | R8 调研 ASI V0.3 增量预测 | R8-RES-07 | optional |

---

## 7. ponytail: 调研基线确认小而精

### 已跳过（守 Ponytail 纪律）

- ❌ 重读 R8-R36 全部 29 轮 JSON / runner.py（已在 R1 survey 摘要）
- ❌ 重跑 git status / log / branch / rev-parse（已在基线包注入）
- ❌ 重读 5 必读文档全文（已用 offset+limit 切片关键部分）
- ❌ V8 长尾议题（计算最优律、形式化验证选型）的展开评估

### 何时该加（升级路径）

- 用户输入真实需求后：用 2-3 天补全 R8-RES-04 因果推断 + R8-RES-05 机制设计 JSON（每项 8-10 query × 5 merged）
- ASI V0.3 公式 0.8816 → 0.8838 仍有 +0.0962 距天花板：R8 两调研候选 + V1082 backlog Top-8 填完 + R8 真实现 TrackA/B/C 三管齐下
- R9+ 调研路线：计算最优律（背书 V1004 self_evolution）+ 形式化验证 TLA+ Harness 验证（背书 R7-PHL-03 落地）

---

## 8. 给 Leader + 用户的一句话

> **R8 调研优先推：① 因果推断（补 R38 Q1+Q11 双空，可抢首发，ASI +0.003~+0.008）+ ② 机制设计（V1083 policy 缺背书，填 G4 RL 决策面空白，ASI +0.005~+0.010）。两候选与 R3/R4/R7 完全闭环，互不重叠。形式化验证与计算最优律留 R9+。等用户输入真实需求后立即启动 R8-RES-04。**

---

— deep_research_lead · R8 调研基线确认 · ASI 北极星不假装、真生产不停、干到底、走在前人经验上、任何人都能接手。