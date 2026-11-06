# v1 → v2 调研与审计卷宗（轮次考古归拢册）

> **本册定位**：把 2026-08-27 → 2026-10-10 期间的**轮次/编号考古稿**按**主题**重组一册，保留原结论与残余，让后来者**不必再做编号考古**。**历史内容不改写**——叙述压缩，事实、表格、残余清单、owner 决策位不压缩；需要现行口径时看 §11。

## 覆盖范围

合并自 ``docs/04-internal/`` 下 **20** 份原件（路径相对仓库根）：

| # | 原件路径 | 轮次 | 性质 |
|---|---|---|---|
| 1 | [`docs/04-internal/round-8-verifications-2026-08-28.md`](../03-audits/round-8-verifications-2026-08-28.md) | Round 8 | 历史（带 2026-09-05 对账批注） |
| 2 | [`docs/04-internal/v2-reference-handbook-2026-08-28.md`](../02-research/v2-reference-handbook-2026-08-28.md) | Round 9 | 历史（带 2026-09-05 对账批注） |
| 3 | [`docs/04-internal/r9-r10-spec-drift-audit-2026-08-28.md`](../03-audits/r9-r10-spec-drift-audit-2026-08-28.md) | R9 + R10 | 历史 |
| 4 | [`docs/04-internal/round-10-research-plan-2026-08-28.md`](../02-research/round-10-research-plan-2026-08-28.md) | Round 10 | 历史 |
| 5 | [`docs/04-internal/youyou-list-research-2026-08-28.md`](../02-research/youyou-list-research-2026-08-28.md) | Round 10 | 历史（带 2026-09-05 对账批注） |
| 6 | [`docs/04-internal/round-10-youyou-list-mainagent-verify-2026-08-28.md`](../05-plans/round-10-youyou-list-mainagent-verify-2026-08-28.md) | Round 10 | 历史（带 2026-09-05 对账批注） |
| 7 | [`docs/04-internal/apeireth-1-0-vs-2-0-functional-gap-2026-08-28.md`](../03-audits/apeireth-1-0-vs-2-0-functional-gap-2026-08-28.md) | Round 11 | 历史（2026-10-06 复核批已降级为历史记录） |
| 8 | [`docs/04-internal/r11-storage-gap-research-2026-08-28.md`](../02-research/r11-storage-gap-research-2026-08-28.md) | Round 11 | 历史（2026-10-06 复核批补横幅） |
| 9 | [`docs/04-internal/r11-longterm-memory-gap-research-2026-08-28.md`](../02-research/r11-longterm-memory-gap-research-2026-08-28.md) | Round 11 | 历史 |
| 10 | [`docs/04-internal/r11-coordination-context-gap-research-2026-08-28.md`](../02-research/r11-coordination-context-gap-research-2026-08-28.md) | Round 11 | 历史（带 2026-09-05 对账批注） |
| 11 | [`docs/04-internal/r11-meta-cognition-gap-research-2026-08-28.md`](../02-research/r11-meta-cognition-gap-research-2026-08-28.md) | Round 11 | 历史 |
| 12 | [`docs/04-internal/r11-species-core-gap-research-2026-08-28.md`](../02-research/r11-species-core-gap-research-2026-08-28.md) | Round 11 | 历史 |
| 13 | [`docs/04-internal/r11-species-form-gap-research-2026-08-28.md`](../02-research/r11-species-form-gap-research-2026-08-28.md) | Round 11 | 历史 |
| 14 | [`docs/04-internal/round-12-final-gap-audit-2026-08-28.md`](../03-audits/round-12-final-gap-audit-2026-08-28.md) | Round 12 | 历史 |
| 15 | [`docs/04-internal/round-13-1-0-maturity-audit-2026-08-28.md`](../03-audits/round-13-1-0-maturity-audit-2026-08-28.md) | Round 13 | 历史 |
| 16 | [`docs/04-internal/round-14-v2-completion-plan-2026-08-28.md`](../05-plans/round-14-v2-completion-plan-2026-08-28.md) | Round 14 | 历史 |
| 17 | [`docs/04-internal/r12-r13-r14-implementation-brief-template-2026-08-28.md`](../00-governance/r12-r13-r14-implementation-brief-template-2026-08-28.md) | Round 12-14 | 历史 |
| 18 | [`docs/04-internal/v2-unabsorbed-features.md`](../v2-unabsorbed-features.md) | 2026-08-27 | 历史 |
| 19 | [`docs/04-internal/stage123-gap-matrix.md`](../03-audits/stage123-gap-matrix.md) | 阶段 1/2/3 标尺审计 | 历史 |
| 20 | [`docs/04-internal/w3-legacy-gap-archaeology-2026-10-10.md`](../02-research/w3-legacy-gap-archaeology-2026-10-10.md) | W3 / 2026-10-10 | 历史 |
| 21 | [`docs/04-internal/r12-coordination-context-1-merge-spec-2026-08-28.md`](../05-plans/r12-coordination-context-1-merge-spec-2026-08-28.md) | Round 12 / R12 | 历史（主代理亲做的 spec 决策冻结） |

**未合并（保留原位）**：[`docs/04-internal/absorption-ledger.md`](../03-audits/absorption-ledger.md)、[`docs/04-internal/absorption-candidates-2026-10.md`](../03-audits/absorption-candidates-2026-10.md)——它们是**现行**吸收台账（不是轮次考古稿），由现行真源管理；[`docs/04-internal/v1-vs-v2-capability-gap-audit-2026-10-06.md`](../v1-vs-v2-capability-gap-audit-2026-10-06.md)——它是 **2026-10-06 的复核审计**，是本册多处旧结论的**现行判定依据**，保留原位并在 §11 指向。

## 时间跨度

- **2026-08-27**：v1→v2 未吸收功能盘查（原件 18）。
- **2026-08-28**：Round 8 → Round 15 的密集考古期（原件 1-17，绝大部分）。
- **2026-09-05 / 2026-10-06**：两次对账/复核批，给多篇加了「历史记录」横幅与修正对照。
- **2026-10-10**：W3 九缺口 legacy 考古（原件 19）。

## 读法

这一册解决三件事：

1. **「编号 → 主题」对照**——§0 是**核心交付**：一张表把 Round 8 / 9 / 10 / 11 / 12 / 13 / 14 与 R9/R10/R11/R12 各编号映射到本册主题节，后来者不必再从文件名反推内容。
2. **「同一主题散在几篇里」**——例如「存储 gap」同时出现在 1.0-vs-2.0 真账、R11-Storage、Round 12 终极审计、Round 13 补查四处；「物种化」横跨 R11-SpeciesCore / R11-SpeciesForm / Round 10 五篇 R7 调研。本册按主题串起来并保留每篇的**独有结论与残余**。
3. **「当年的结论今天还成立吗」**——多数原件已自带「本文为历史记录，写作时数字属实于当时」的横幅；现行口径统一看 §11。

---

## §0 编号 → 主题对照表（**核心交付**）

| 原编号 / 原件 | 轮次 | 主题（**本册节**） | 该篇的独有贡献 |
|---|---|---|---|
| `round-8-verifications-2026-08-28.md` | Round 8 | §1 派单制度与守门验证 | 5 项 verify 真账；Round 7「写 20 处实修 16 处」的自曝；5 重守门 baseline（1739 passed / legacy 36 / 9 锚） |
| `v2-reference-handbook-2026-08-28.md` | Round 9 | §1 | 一站式接手 reference：5 步读完 + 6 项决策冻结 + Round 1-9 commit 链 + 13 处 0 装诚实标中的 7 条 + 风险总账 + action items |
| `r9-r10-spec-drift-audit-2026-08-28.md` | R9 + R10 | §1 | spec drift 9 条分类（真错账 / 演化 / 未亲验 / 0 drift）；**子代理 flag 偏差 ≠ spec 错账**的方法论教训 |
| `r12-r13-r14-implementation-brief-template-2026-08-28.md` | Round 12-14 | §1 | 派单 brief **7 段模板** + 11 派单具体模板 + 3 条设计原则 + 主代理/子代理分工 |
| `round-14-v2-completion-plan-2026-08-28.md` | Round 14 | §1 | 9 哲学锚 × 完成计划：主代理亲做 10 项 spec（~2 周）+ 派 sub-agent 11 项（12-14 周）+ release Week 18-20；6 phase 顺序；总 15-18 周 |
| `apeireth-1-0-vs-2-0-functional-gap-2026-08-28.md` | Round 11 | §2 v1→v2 功能差距总账 | 用户 directive 真账（功能不能缺）；1.0 功能全集 ~100 modules 的分层清单；release 阻断条件 |
| `round-12-final-gap-audit-2026-08-28.md` | Round 12 | §2 | 调研完成度 100%（~5400 行 / 19 篇）；v2 已就位 ~30 / Partial ~15 / **缺 ~25 项 P0** 六类；critical path 4-6 月 → 6-9 月 |
| `round-13-1-0-maturity-audit-2026-08-28.md` | Round 13 | §2 | 1.0 maturity **实测 8 个 .rs**；~28 REAL / ~4 trait 口 / ~5 PARTIAL / ~10 处 0 装 PASS 标注；release 路径修订表 |
| `v2-unabsorbed-features.md` | 2026-08-27 | §2 | A/B/C/D 四级未吸收分级（6 A + 7 B + 14 C + 14 D）；P1-P6 路线图；总工作量 15-23 周；**7 项待拍板决策**；13 键降级拍板记录（加权 0.28/5） |
| `stage123-gap-matrix.md` | 阶段 1/2/3 标尺审计 | §2 | 27 条标尺 × 18-crate 现实逐条对账（✅5 / 🟡15 / 🔄6 / ⬇️1 / ❌0）；12 条显式降级漂移；结论①-③ + 最重 10 差距 |
| `r11-storage-gap-research-2026-08-28.md` | Round 11 | §3 存储 / 向量 / 图 | **主代理真账 §1.1 三项 ❌ 是误读**的修订；1.0 真账真路径在 `_research_mem/apeireth-rust-fork/`；P0/P1/P2 六项 + 总 8-11 周 |
| `r11-longterm-memory-gap-research-2026-08-28.md` | Round 11 | §4 长期记忆塑形 | 6 模块 ~1734 行；consolidation_writeback pipeline 全链；P0/P1 五项排序；`cross_diary` 被 storage graph 卡住 |
| `r11-coordination-context-gap-research-2026-08-28.md` | Round 11 | §5 协调与上下文 | 9 项最重（11-13 周）；**v1 `rot_score` 重复实现**；**`hello.rs` 概念 collision**（Windows Hello NGC ≠ 启动/装配）；主代理 §1.8 三项状态修订 |
| `r12-coordination-context-1-merge-spec-2026-08-28.md` | Round 12 / R12 | §5 协调与上下文 | **rot_score 融合 spec 决策冻结**：方案 A/B/C 三案 + 主代理推荐 A + 命名统一表；「1.0 自 flag 0 装 PASS，不假装 rot_score 准确」的诚实标 |
| `r11-meta-cognition-gap-research-2026-08-28.md` | Round 11 | §6 反思与元认知 | 6 模块认知层循环图；R22 reflection / meta_thinking / thought_cluster 三件套同期；Brier 校准 + BetaBinomial 置信度 |
| `r11-species-core-gap-research-2026-08-28.md` | Round 11 | §7 物种化核心与塑形 | vision L47-49 / L51 对位表；principles → partner → community → education 的依赖次序；P0 四项各 4 周 |
| `r11-species-form-gap-research-2026-08-28.md` | Round 11 | §7 | timeline 79 / tone 374 / morphology 284 = 737 行 + 26 测试；**全维度 9 维表**（含 1.0 已有的 frontend 容器、记忆、主动三行）；估时 5-8 周 |
| `round-10-research-plan-2026-08-28.md` | Round 10 | §8 外部参考边界 | 170+ 清单 **主代理初评**（10 类）；评估维度定义；两条「想法随记」（U 盘便携 / 刷视频）评估 |
| `youyou-list-research-2026-08-28.md` | Round 10 | §8 | 170 项目 **sub-agent 真账**（分类总览 / TOP 10 / MED / LOW+NONE / 决策建议）；190 含重复、净 170+ |
| `round-10-youyou-list-mainagent-verify-2026-08-28.md` | Round 10 | §8 | 主代理亲验修订：**TOP 5 不矛盾，是两个维度**（Agent 框架类 vs AI 伴侣类）；sub-agent 用 Star 数评估 HIGH 的方法偏差自曝 |
| `w3-legacy-gap-archaeology-2026-10-10.md` | W3 / 2026-10-10 | §9 legacy 九缺口考古 | 逐个读 v1 真代码的 9 缺口结论表；**两处翻案更正**（沙箱 / community 分诊）；「一次只做一个」的裁决约束 |

---

## §1 主题 A —— 派单制度、计划模板与守门验证

### §1.1 Round 8 验证（原件 1）

- **派活真账**：用户原话「硬件测试先不急, 其他的你有能干的吗」→「那就干, 别忘了哲学锚和我们的最高要求, 边干边写文档」。Round 8 的 5 项：① ✅ verify #1 修 4 处 v2-gateway-frontend HEAD stale（Round 7 batch 漏的 L258/447/487/561）；② ✅ verify #2 读 `.github/workflows/o6-anchor.yml` 验 5 重守门 CI 真账（166 行 real workflow）；③ ✅ verify #3 读 `cognitive-module-wiring.md` 验 12 slot ledger 与 R15 spec 一致；④ 🟡（in-progress）verify #4 C 块 preference_learning 真实施 mapping（派 sub-agent 调研）；⑤ 🟡（in-progress）verify #5 摸清 v2 gateway 当前 3 路由（B 块前端对接前置）。
- **verify #1 修法（原表）**：L258 / L447 / L487 / L561 四处的 stale HEAD（`b9026186` → `7d990297`（Round 6 完；历史 v2.0.0-rc.1 tag @ `b9026186`））。验收：`grep b9026186 ../../02-guides/v2-gateway-frontend-integration-spec.md` → **0 命中**（除 §6「历史 v2.0.0-rc.1 tag」标注）。
- **0 装诚实标**：Round 7 commit `155a9450` 的 commit message 写「20 处 stale doc batch fix」，**实际只修了 16 处**（16 file，26 ins/26 del，1 file 含 2 处），漏数 4 处。Round 8 收尾显式 flag + 补 commit，**不假装 20 处全修**。
- **5 重守门 baseline（Round 8 实测）**：clippy 0 警告 ✅（Round 7 baseline）；tests 0 失败 ✅ **1739 passed / 0 failed**（Round 7 baseline）；legacy compat path < 100 ✅ **36**（Round 1 baseline）；LOCKED 5 项 0 触碰 ✅；哲学锚表头 0 减 ✅ **9 锚**。
- **后续动作**：push origin/main；派 sub-agent 调研 #4 + #5；主代理亲验 + 写 doc + commit + push；准备 C 块 preference_learning 真实施启动（R15 spec → 真实施）。

### §1.2 Round 9 一站式 reference（原件 2）

- **一句话接手（原文）**：Apeireth v2.0.0-rc.1 在 `origin/main @ 70281cc6`（Round 9 完；**2026-09-05 对账：origin/main @ `7647d2c9`，17 crates / 3120 passed / 0 failed / 13 ignored / workspace.version 2.0.0-rc.1**），**9 哲学锚 LOCKED，1739 tests PASS［写作时；现 3120］，0 clippy 警告，0 触碰 LOCKED 5 项**。A 块（OrganOrchestrator 完整化）已落地，B/C/D 3 块调研真账已写完（~1586 行），**派单顺序 + critical path 5-7 周调研就位**。
- **接手工程师「5 步读完」（估 1-2 小时）**：① 读本 handbook；② 读 `ENGINEER-MANIFESTO.md`（596 行，14 章，工程规范 + 真实陷阱 12 条）；③ 读 `handoff-log-2026-08-28-mavis.md`（124 行，Round 1-3 接手真账）；④ 读 `sub-agent-audit-round-4-2026-08-28.md`（201 行，Round 4 audit 真账）；⑤ 读 `round-8-verifications-2026-08-28.md`（256 行，Round 8 CI/wiring 真账）。
- **选 6 项决策冻结起点（主代理亲做 1-2 天）**：R9 spec 4 处错账修；R10 spec 数字错账修；R12 不续派（已落地，n/a）；**9 organ UI 暴露范围冻结**（候选 A+D，spec 决策不 commit）；**主人审批 modal 行为冻结**（候选 A）；**Tauri keyring 冻结**（候选 E）。
- **Round 1-9 commit 链（真账）**：Round 1 接手 MANIFESTO + 数字漂移修（8 commit：`bbbfb75b` → `c003e078` → `087ab2ac` → `50ba2e57` → `29e5ce66` → `0afa733f`（A 块 5 stage）→ `1d885299` → `5e18e65b`）；Round 2 `13e73891`（organ-orchestrator-spec 8→9 锚漂移修）；Round 3 `aa488f1f`（5 docs 8 处 8→9 锚 LOCKED drift batch + 1 force push 真账）；Round 4 `bde6268b`（sub-agent audit）+ `0ca16572`（§13 增 3 工序）；Round 5 `dd4a72de`（src 漂移 + 3 docs R11 path + §13 #12 author env）。
- **0 装诚实 doctrine（7 条）**：① 不假装 OK（cargo test fail 不 commit，撤 + 写真账）；② 不「等以后修」（O-6 失守 flag 即改）；③ 不「删 commit 重做」（amend 历史 commit 风险 > 收益，用 followup commit）；④ 不「只读 review」（sub-agent 调研读当前 spec 实测，不只读 review 标）；⑤ 不「派了不管」（主代理必亲验 5 重守门 + LOCKED 0 触碰）；⑥ Round 1 amend `5e18e65b` 没真修 doc 本体（write-tree 没 git add，msg 写「修 3 处」实际 0 修）→ followup `e3300347` 真修 + flag；⑦ Round 1 commit `e3300347` msg 谎报 `.harness-msg/` ignore（当时未加规则）。
- **风险总账（原表）**：B 块 frontend 估时溢出（中-高，派 A+B+C 并行 + 1 主代理亲做决策 + 6 周 buffer）；C 块 6 DEFERRED slot 估时溢出（中，R20 + R21+R22 并行 + R23+R24 后 + 6-10 周估时）；D 块 RC-7 需硬件（**高，硬阻塞**，等硬件到位，mock test 先做）；sub-agent workflow O-6 失守（中，brief 必含「跑 5 重守门 + 不写真账以外」）；网络阻塞 push（中，用户开代理 / 等恢复 / force-with-lease 老 tip）。
- **Round 9 自身 0 装诚实标**：R20 sub-agent 真实施 O-6 失守（~6h，改 plan 后撤，写真账 257 行）；B-A sub-agent 真实施 O-6 失守（~3h，cargo test EXIT 101 + 3 test fail，主代理撤，写真账 170 行）；sub-agent 2 的 R9 spec flag 误判（R13 review 历史快照 vs 当前 R9 spec 实测，sub-agent `70f782ed` 修订真账）；Round 4 author 失守（4 commits author = minimax-m3-agent，非 Mavis，Round 4 author amend）。
- **后续 action items（按 §8.1 顺序，原表节选）**：P0 主代理亲做 6 项决策冻结（1-2 天）；P0 派 sub-agent A 真实施 B 块 gateway SSE（含修 3 test fail spec，2-3 天）；P0 派 sub-agent R20 真实施 preference_learning（2-3 周，阻塞 = R10 OrganKind 决策）；P1 派 sub-agent B 真实施 frontend `runtime.ts` + 主人审批 modal（2-3 周）；P1 派 sub-agent C 真实施 Tauri shell keyring（3-5 天）；P1 派 sub-agent D 真实施 E2E + 5 重守门 baseline（1-2 周）；P1 派 sub-agent R21 critic + R22 reflection 真实施（2 周）；P2 派 sub-agent R23 planner + R24 orchestrator 真实施（LLM Adapter 新设计，6 周）；P2 RC-7 Perception 真 modality（需硬件麦克风 + Windows，2-3 周）；P1 修 gitignore 3 真漏（`reconstruction_v2/` + `_scripts/` + `.gitignore-research`）+ 缩 `.py`（30 min）；P1 修 R15 spec §5.1 L364 critic 路径错账（用 `critic.rs` 不是 `judge.rs`，5 min）。
- **5 重守门 baseline（每 commit 严守）**：`cargo test --workspace --locked`（期望 3662 passed / 0 failed / 21 ignored）；`cargo clippy --workspace --all-targets --locked -- -D warnings`（期望 0 warning）；`git diff` LOCKED 5 项（期望 0 行）；legacy compat path < 100（期望 < 100，现 36）；9 哲学锚表头 0 减（期望 9）。
- **收尾结论（原文）**：2027-Q1 启动，2027-Q2 完。

### §1.3 R9 + R10 spec drift 审计（原件 3）

- **方法**：读 6 个文件 + 实地核验当前 R9 spec 行号，**不 commit**；HEAD `22c6e72b`（R13 review §4.2 报）。
- **drift 表（9 条，原表）**：分类为 **真错账 2 处**（R9 spec §3.6 L258 与 §9.1 L447 的 HEAD 漂移 `7d990297` → `22c6e72b`）、**演化（历史快照 OK 保留）7 处**（R9 §0 L25、§5.1 L330、§5.2 L340/342、§10 L483、R9 quickstart §1 L23 推测已修未亲验、R10 §1.2 L51、R10 §10.1 L719-727）、**未亲验 1 处**（R9 spec §1.1 §44 tools commit `18d6bf36`，信任 spec）、**0 drift 8 处**（R9 §0 §21/§23/§27、§1.1、§2.x、§3.1-3.5、§4.1/4.3、§6.x/§7.x、§8/§9.x/§11/§12；R10 §1.2/§4.x/§5.1/§6.x/§7.x/§8.x/§9.x/§11.x/§13）。
- **方法论教训（原文）**：子代理 2 flag 的「R9 spec 错账」是依据 **R13 review（接力审）的旧账**，不是当前 R9 spec 真账——子代理没亲验当前 spec，**这是 sub-agent 调研偏差，不是 R9 spec 错账**。
- **主代理亲做 6 项核验**：R9 spec 4 处 + R10 spec 3 处错账修 = **✅ 0 待修**；核验 #3-#6 与 spec drift 无关。**派单建议**：主代理亲做核验 #3（R12 working tree 跑通）+ #4-#6（UI/keyring 决策）；**spec 错账修 = 0 派单**。

### §1.4 Round 12-14 派单 brief 模板（原件 17）

- **设计原则 4 条**：① **统一模板**——11 项派单 brief 用同一模板（R12-CoordinationContext-1/2/3 + R12-SpeciesCore-1/2 + R12-LongTermMemory + R12-Storage + R13-SpeciesForm + R13-MetaCognition + R13-ToolsSecurity）；② **必含 7 段**（任务 + 必读 + 必输出 + 0 装诚实标 + 5 重守门 + LOCKED 0 触碰 + 真实施流程）；③ **不依赖网络**（本地 working tree 已就位，0 git clone 必要）；④ **主代理亲做 + 派 sub-agent 分工**（主代理亲做 spec ~2 周 7 项决策冻结；派 sub-agent ~12-14 周 critical path 11 项派单）。
- **brief 模板要点**：任务段含「背景（v2.0 真账缺 ~35 项 1.0 真实施）/ 承接 / 物种化维度（三面一体 + 五原型 + 物种化）」；必读段含 7 份必读文档（`round-14-v2-completion-plan`、`apeireth-true-understanding`、`apeireth-1-0-vs-2-0-functional-gap` 等）；必输出段规定写真账 ≤ 200 行、必含实现摘要 / 参考点 / 0 触碰 LOCKED 验证 / 参考路径 / 下一步；0 装诚实标段要求跑 5 重守门 baseline（主代理亲验前**不假装 PASS**——per Round 9 B-A sub-agent 失守教训）；约束段含 0 引新外部 dep、0 写真账以外的 file、不 git add/commit/push。
- **真实施流程**：主代理亲做 10 项 spec（~2 周）→ 派 sub-agent 真实施 11 项（~12-14 周 critical path）→ release 流程 Week 18-20（1-2 周）：5 重守门 baseline 实测 + ROADMAP §7 + MANIFESTO §14 + ROADMAP §12 check + `git tag v2.0.0` + release notes + announcement。
- **结束语（原文）**：v2.0 release 2027-Q3（修订估时，真实施 critical path 12-14 周 + release 1-2 周，总 15-18 周 ~4 月）。

### §1.5 Round 14 v2.0 完成计划（原件 16）

- **触发**：用户原话「OK 把文档先更新了, 注意实事求是, 注意哲学锚, 然后给我一个你下一步的想法, 要怎么完成 2.0」。主代理自省：文档已更新（Round 14 commit `3ea454f1` push 成功，ROADMAP §7 + MANIFESTO §14 + 真账 §6.3 §3.1 修订）；实事求是（4-6 月 → **6-9 月**）；9 哲学锚 LOCKED 0 触碰。
- **9 哲学锚在计划中的落点**（各带一段）：S-1 北极星（Everything serves the ASI north star 五原型）；S-2 实事求是（Verify before writing; truth over narrative）；S-3 质量工程化（3120 passed / 0 failed / 13 ignored / 0 clippy / LOCKED 0）；O-1 安全优先（9 重 v9 守门 + 13 键 verdict cache + 3 项不可变脊柱）；O-2 前人肩上（Borrow, attribute, adapt）；O-3 干到底（Finish what we start; no half-measures）；O-4 任何人都能接手（Any newcomer can onboard from docs alone）；O-5 不假装（Never fake it）；O-6 永远追求最优（总体 / 系统 / 架构 三阶审查）。
- **主代理亲做 10 项 spec（~2 周）**：① v1 `context.rs` + `context_rot.rs` rot_score 融合（1-2 天）；② cognitive module `consolidation_writeback_pipeline` + `reflection_writeback_pipeline` trait spec（1-2 天）；③ **hello.rs 主题确认**（已知 Windows Hello NGC 121 行，不是「启动/装配」）（1 小时）；④ education 真 CAS spec（1-2 周）；⑤ confidence BetaBinomial trait spec（1 周）；⑥ reflexion 3 trait 口实接线 spec（1 周）；⑦ 6 真实施派单 brief 模板（1-2 天）；⑧ **主代理实测 27 项 1.0 .rs maturity 补查 + 2.0 crates 真账实测**（2-3 天，**不依赖网络**，本地 working tree 已就位）；⑨ ROADMAP §7 + MANIFESTO §14 release timeline 修订 ✅（Round 14 commit `3ea454f1` 已 done）；⑩ §3.1 估时修订 ✅（Round 13 真账已 done）。
- **派 sub-agent 真实施 11 项（12-14 周 critical path）**：R12-CoordinationContext-1/2/3（3-4 / 3-4 / 2-3 周）、R12-SpeciesCore-1/2（各 2 周）、R12-LongTermMemory（5-7 周）、R12-Storage（1-2 周）、R13-SpeciesForm（5-8 周）、R13-MetaCognition（5-7 周）、R13-ToolsSecurity（6-10 周，阻塞 = D 块硬件）、R20 preference_learning（2-3 周，阻塞 = R10 OrganKind 决策 + 主代理 #2 spec）。**真实施总估时（含主代理亲做）14-16 周（~4 月）。**
- **6 phase 顺序**：Phase 1（主代理 spec 2 周 + 派 R12-CoordinationContext-1 准备 1 周）→ Phase 2（R12-CoordinationContext 3 sub-agent，Week 3-11，最重）→ Phase 3（R12-SpeciesCore 2 sub-agent，Week 6-10）→ Phase 4（R12-LongTermMemory + R20，Week 7-14，跟 R22 reflection 并行）→ Phase 5（R13 后续，Week 10-20）→ Phase 6（release 流程 Week 18-20）。**总 critical path Week 1-20（约 5 月）。**
- **release 估时修订表**：Release 时间 2027-Q1-Q2（4-6 月）→ **2027-Q3（6-9 月）**；总进度 80% → **70-75%**；真实施 critical path 3-4 周（主代理 §3.1 估）→ **12-14 周**；总 critical path（含主代理亲做）**14-16 周（~4 月）**。
- **Round 14 0 装诚实标（5 条）**：Round 11 6 sub-agent 调研 0 实测 1.0 .rs；主代理 §3.1 估 3-4 周 ❌ 偏乐观；release timeline 修订（4-6 → 6-9 月）；**0 实测 2.0 master branch**（本地 working tree 已就位 ~86-crate v1 + 17 crates v2 真账）；**1.0 maturity 35 项中 8 项实测（~23%）**，余 27 项仅凭推断。

---

## §2 主题 B —— v1 → v2 功能差距总账

### §2.1 1.0 vs 2.0 功能差距真账（原件 7，Round 11）

- **用户 directive（原文）**：「多派几个子代理调研, 你同时研究 Apeireth 还缺什么, 还缺什么, 就得从功能性视角来看, 且得看 1.0 缺什么, 因为 2.0 最后的功能应该和 1.0 相同, 但架构不同而已」。
- **3 条关键 directive（per O-5 + S-2）**：① **v2.0 release 必须 = v1.0 功能全集 + 架构升级，不能缺功能——这是 release 阻断条件**；② **从功能性视角** vs **架构视角**（之前看的是「v2 工程现状 17 crates / 9 organ / 12 slot」= 架构视角）；③ **架构可不同，功能不能缺**（1.0 master：legacy/donor/、86-crate、God Object UnifiedRuntimeHost → 2.0 canonical：17-crate workspace、三面一体 + 五原型 + 物种化）。
- **主代理自省（原文）**：Round 1-10 一直画「v2 工程架构现状」，**缺 1.0 vs 2.0 功能对比——这就是 release 必补盲点**；v2 吸收链 + R7 5 真调研都是架构 + 设计思路对齐，**不是功能全集对比**；v2 release 估 2027-Q1-Q2（per MANIFESTO §14），**必须 0 缺 1.0 功能全集**。
- **1.0 功能全集真账**：per `legacy/donor/apeireth-companion/src/` **~100 modules** + `master-functionality-port-audit.md` L197-771；分层为 Storage 层 / Tools & Sandbox / Runtime Services / Companion / Provider-Protocol / Gateway-Streaming-MCP / SDK-Voice-Avatar-Bridge（详见原件 §1.1-1.9 的逐项表）。
- **Round 12 §2.4 的 P0 必补六类**（原件 7 的差距表在 Round 12 被升级为 25 项，见 §2.2）：A Storage 抽象层 3 项；B Tools + Sandbox 4 项；C Companion 物种化核心 ~10 项；D 物种化塑形维度 3 项；E 反思 + 元认知 6 项；F 协调 + 上下文 9 项。
- **原件自带的 2026-10-06 复核批注（保留，历史不改写）**：本文 🔴/🟢 结论**大面积过时**；已知修正对照三条——「v2 缺 BM25 混合检索」❌ 错（`crates/engine/memory/src/hybrid_search.rs` 有 Okapi BM25 + RRF 真实现，IMPLEMENTED）；「organ 仅 E4 真实现，其余 0 装」❌ 陈旧（engine 侧 9/9 IMPLEMENTED，`organ/lib.rs:31`；`plugin/src/organ.rs` 假账已由 `e4f2f451` 更正）；「perception 5 modality 仅 Text 真」⚠️ 基本成立（5 modality 类型/后端抽象 IMPLEMENTED，真后端仅 Text，其余 Noop fail-closed，均未 PRODUCTION WIRED）。**状态：本文降级为历史记录，仅保留「v1 功能全集清单」用途。**

### §2.2 Round 12 终极审计（原件 14）

- **触发**：用户原话「你现在研究下所有调研和 1.0 功能的现状, 告诉我 Apeireth 还缺什么. 调研完了吗」。
- **调研完成度真账（Round 9-11，19 真账 doc，~5400 行，原表节选）**：Round 9 = 真理解 229 行 + reference handbook 613 行 + sub-agent audit Round 4 201 行 + handoff log 124 行 + MANIFESTO 596+ 行；Round 10 = youyou-list 真账 156 行 + plan 310 行 + mainagent verify 204 行 + 5 篇 R7 物种化（318/309/180/239/224 行）；Round 11 = 1.0-vs-2.0 主代理真账 361 行 + 6 篇 R11（303/311/261/338/222/283 行）。覆盖率：真理解 100%、9 organ 语义对齐 100%、12 slot 100%、LOCKED 5 项 100%、1.0 vs 2.0 功能全集 **~95%**、6 R11 gap 100%、5 R7 100%、youyou-list 100%、Round 8 verify 100%。
- **调研完成度结论（原文）**：**🟢 调研部分 100% 完成**；**🟡 未调研部分**（主代理亲做，不可分派）——① **真实施前必亲验**：0 git clone v2 master branch（~86-crate），仅读 1.0 真账 + 2.0 handbook + R7 + R11 推论；② 10 commits ahead origin 已 push；③ **新发现**：v1 **重复实现警告**（`context.rs` + `context_rot.rs` 同一 `rot_score` 两 file 各实现一遍，真实施前必先融合）、**hello.rs 概念 collision**（v1 `hello.rs` 是 Windows Hello 生物识别 NGC 探测，跟「启动/装配」主题 mismatch）。**🟢 调研完成度 100%，但真实施 0%。**
- **v2 现状三分档**：🟢 **已就位 ~30 项**（Storage SQLite pool + migrations + MemoryStore v2；Tools 核心 Shell/Filesystem/Fetch/Search/Repo；Governance 5-gate + onion + PII + AuditHashChain + SelfDisableGuard；EventBus core；SessionManager；UnifiedRuntimeHost DROP；Protocol normalized DTOs；Provider adapter DTOs；Gateway router + egress；Companion emotion(F1) + Borbely(E7) + curiosity(E4) + world_model(W1/W2/W3)；Round 11 merge 含 4 真实施：lock microkernel freeze / govern isolated completions / single module-owned capability path / preserve approval identity）；🟡 **Partial ~15 项**（Memory support modules / VectorIndex + Graph primitives / Browser tool / Learning tool + SystemMonitor / WorktreeSandbox + ToolSynthesizer / MCP 协议-client-server / SovereignControl / EventBusBackbone + Scheduler + Telemetry / WsFrame + voice VAD / Gateway SSE broadcaster + MCP handler / DreamEngine + EpistemicHealer + PromptAssembler / TopicPredictor + PreloadChannel / memory_extractor + memory_graph / presence / streaming_chat + voice_session）；🔴 **缺 ~25 项 P0 必补**（六类，见下）。
- **🔴 P0 必补六类（逐项）**：**A Storage 抽象层 3 项**——VectorIndex (cosine + BM25 hybrid)、Graph primitives / causal graph、Memory support modules；**B Tools + Sandbox 4 项**——ToolSynthesizer、Invest tool、Vision ScreenCapture / pHash、OmniParser window enumeration、DesktopActionTool；**C Companion 物种化核心 ~10 项**——education、partner、community、principles、daily_summary + diary、cross_diary、memory_injection、reflexion / reflection；**D 物种化塑形维度 3 项**——timeline、tone、morphology；**E 反思 + 元认知 6 项**——meta_thinking、thought_cluster、intent_brier、confidence、HybridCognitiveRouter（+1 partial）；**F 协调 + 上下文 9 项**——onering、oracle / oracle_adapters、context / context_rot、continuation / continuity / spill、assemble / hello、milestone、experiment_field、proactive / progressive / pentest、Kani proofs（+1 partial）。
- **critical path 修订表**：主代理真账 §3.1 估 3-4 周 ❌ → R11 调研实际 **11-13 周**（R11-CoordinationContext 最重）；6 R11 调研估时 = Storage 6-8 周 / LongTermMemory 6-8 周 / SpeciesCore 4 周 / SpeciesForm 5-8 周 / MetaCognition 5-7 周 / CoordinationContext **11-13 周**；**总 4-6 月 → 6-9 月；v2 release 2027-Q1 → 2027-Q3**。
- **Round 12 的 7 条 0 装诚实标**：§1.8 状态 ❌ → ⚠️ partial 修订（oracle/proactive/Kani 三项）；v1 重复实现警告；hello.rs 概念 collision；**调研 0 实测**（全部 sub-agent + 主代理 0 git clone v2 master branch）；R11 sub-agent 多次超约束（SpeciesForm 338 + LongTermMemory 311 + Storage 303 行均超 ≤300 约束且未 flag）；主代理 §3.1 估时偏乐观。

### §2.3 Round 13 1.0 maturity 补查与 release 路径修订（原件 15）

- **Q1 答（原文）**：「release 路径」= v2.0 release 的 timeline + critical path + dependencies 计划；**修订 release 路径** = 重新估 release 时间表 + critical path + 必补清单。
- **Q2 答 + 自省（原文）**：用户 catch **对**——R11 6 sub-agent 真调研**没真读 1.0 代码**，仅凭 1.0 路径列表（grep 过 ~100 modules）+ master audit L197-254 的 maturity label + 公开领域知识 + R7 推断；**sub-agent 与主代理一样 0 实测 1.0 真代码**。调研深度修订：✅ 1.0 真账**路径**已知；⚠️ 1.0 真账**成熟度**（REAL/PARTIAL/STUB）仅靠 master audit + 推断，**未逐个 .rs 实测**；✅ 0 装 PASS 标注已知；⚠️ **实接线断点未逐个 .rs 实测**，推测约 5-10 项标「trait 口」/「未接线」需真实施时主代理亲做 spec。
- **主代理亲测 8 个核心 1.0 .rs**：daily_summary.rs（99 行，REAL，0 LLM 纯函数）、diary.rs（442 行，REAL，DiaryStore 完整 + DiaryInjector trait 口，注入实接线延后）、cross_diary.rs（301 行，REAL，跨日记关联 + memory_graph 联动，确定性 token 匹配，注入侧留 trait 口）、memory_injection.rs（66 行，REAL，反幻觉闭世界证据 + 反幻觉指令）、reflexion.rs（497 行，REAL，E1 口头强化闭环 + 4 段职责链，LLM 版 CRITIC 未接 / 失败事件实接线未接 / 注入块消费侧未接线）、education.rs（402 行，PARTIAL，换元法 dx 检查器 + 插件装配，**v1 是字符串级规则表，不是真实符号计算（无 CAS 引擎，不宣称能解积分）**）、partner.rs（141 行，REAL，PartnerId + 关系数据）、+ storage path 修订。
- **maturity 汇总表（原表）**：Storage 抽象层 2 项 = 0 REAL + 2 trait 口待亲做；长期记忆塑形 6 项 = 5 REAL + 1 trait 口（reflexion 3 个 trait 口）；物种化核心 4 项 = 3 REAL + 1 部分 + 1 PARTIAL（education 字符串规则 vs 真 CAS）；物种化塑形维度 3 项 = 3 REAL；反思 + 元认知 6 项 = 5 REAL + 1 PARTIAL（HybridCognitiveRouter rule-based fast path with hardcoded templates）；协调 + 上下文 15 项 = 12 REAL + 3 PARTIAL；**总 ~35 项 = ~28 REAL（逐项可实现）+ ~4 trait 口待主代理亲做 + ~5 PARTIAL + ~10 处 0 装 PASS 标注**。
- **修订主代理真账 §2**：v2 真实施 = **28 项语义对齐 + 4 项语义对齐 + 主代理亲做 trait 口实接线 spec + 5 项 PARTIAL 0 装诚实标**。
- **修订真账 §3.1 估时**：长期记忆塑形 5-7 周 → **6-8 周**（+reflexion 3 trait 口 spec 1 周）；物种化核心 4 周 → **5-6 周**（+education 真 CAS spec 1-2 周）；反思 + 元认知 5-7 周 → **6-8 周**（+confidence BetaBinomial trait 1 周）；协调 + 上下文 11-13 周 → **12-14 周**（+v1 重复实现融合 1-2 周 + hello 主题确认 1 小时）；**总 critical path 11-13 周 → 12-14 周**。
- **修订 release 路径**：ROADMAP §7 总进度 80% → **70-75%**；MANIFESTO §14 release timeline 2027-Q1-Q2 → **2027-Q3**；ROADMAP §12 release path 4-6 月 → **6-9 月**。
- **Round 13 修订后派单顺序 11 项**：① 主代理 rot_score 融合（1-2 天，优先）→ ② R12-CoordinationContext-1（3-4 周）→ ③ -2（3-4 周）→ ④ -3（2-3 周）→ ⑤ R12-SpeciesCore-1（2 周）→ ⑥ -2（2 周，主代理亲做 education 真 CAS spec）→ ⑦ R12-LongTermMemory（4-6 周）→ ⑧ R12-Storage（1-2 周）→ ⑨ R13-SpeciesForm（5-8 周）→ ⑩ R13-MetaCognition（5-7 周）→ ⑪ R13-ToolsSecurity（6-10 周，硬件）。
- **Round 13 主代理亲做 10 项**：见 §1.5 的 ①-⑧ + ⑨⑩。

### §2.4 v1 → v2 未吸收功能分级（原件 18，2026-08-27）

- **盘点范围**：盘点对象 `legacy/donor/` 77 crate + `legacy/archived/` 15 + `legacy/frozen/` 13；对照对象 v2 工作区 `crates/{foundation/{core,protocol,plugin,governance,credentials}, engine/{runtime,provider,storage,memory}, capabilities/tools, adapters/{gateway,cli,sdk}}`；跳过已被 ROADMAP §4 P0/P1/P2/P3 覆盖的（governance 接线、credentials 接线、core drain、M1B 记忆落地）与纯 v1 内部工具。
- **分级原则（原表）**：**A = S1 关键**（v1 哲学/愿景明文提到，v2 不吸收则长程任务跑不通或核心哲学层落地不了 → 必须进 roadmap）；**B = S2 重要**（缺了能力下降但能跑 → 评估后进 roadmap）；**C = S3 一般**（用户体验下降但不影响核心 → 排到 P-arch 之后或独立 minor）；**D = S4 边缘**（特定场景才用 → 文档保留指针，不实现）。核心判定三问：① v1 哲学层（8 锚 + 13 键 + 三洋葱 + L0 HA + Self-Disable）落地是否依赖此功能？② v2 主链（gateway + canonical agent loop）能否承载完整长程任务（>3 轮，含多 agent / 多模态 / 长程评估）？③ 用户拿 main 跑任务时，会不会因为缺这个功能而**拿不到 v1 能拿到的结果**？
- **A 类 6 项**：**A1 `apeireth-council`**（7 强制 advisor 域：safety/performance/philosophy/history/strategy/ethics/legal；3 生命周期 persistent/ephemeral/dynamic；按住机制 = 30% 强反对即 veto + 一致反对即 reject + 60s 裁决超时转 L0 HA；多意见加权 synthesis；拟人化独立 session/persona/立场/可辩论 3 轮）；**A2 `apeireth-team-lead`**（14 调度工具 + worktree 集成）；**A3 `apeireth-perception`**（多模态输入 + Attention 策略）；**A4 `apeireth-memory-extensions`**（持久化记忆后端扩展，9 个 backend）；**A5 `apeireth-tool-runtime`**（工具调用解析与执行）；**A6 `apeireth-tool-approval`**（工具调用审批 v1 模型）。
- **B 类 7 项**：B1 `apeireth-experience`（经验沉淀提炼回流）、B2 `apeireth-state`（全局状态机 + 持久化）、B3 `apeireth-cognition`（Attention + 主动提议）、B4 `apeireth-sovereignty`（HA 多签 + 物理隔离）、B5 `apeireth-supervisor`（进程级监督树）、B6 `apeireth-action`（动作空间 + 决策选择）、B7 `apeireth-eval`（评估 harness）。**C 类 14 项 / D 类 14 项**（简表见原件）。
- **路线图（与 ROADMAP §4 对齐）**：P0 ✅ 已完成（upstream `873d2857` governance 接线）；**P1** A4 MemoryBackend trait（File impl + SQLite 增强）+ B4 sovereignty 多签扩展 + core drain + credentials 接线（2-3 周，进 `crates/engine/storage` + `crates/foundation/governance`）；**P2** 13 键降级为哲学标准 ✅ 已完成 + scene-d 设计 + ROADMAP 同步（0）；**P3** A6 tool-approval frequency/blacklist + B1 experience + B2 state machine + C 类 memory 周边（3-4 周）；**P4** A3 perception trait + Text impl（1 周，进 `crates/engine/perception` 新 crate）；**P5** A5 tool-runtime ParsedToolCall + B5 process supervisor + scene-d 例 2 SelfAssessmentCache multi-instance（3-4 周）；**P6** A1 council + A2 team-lead + B3 cognition + scene-d 例 3（6-8 周，进 `crates/foundation/orchestration` 新 crate）；**P-arch 后** B6 action / B7 eval / D 类独立评估。**总 v2 实现工作量 15-23 周（约 4-6 个月一人）**，与 ROADMAP §4 P1-P6 时间线对齐。
- **13 键降级拍板记录（2026-08-27，5 维评分 1=强烈支持降级 / 5=强烈支持接线）**：安全性 **1**（self-introspection 是「AI 评 AI」路径，被 prompt injection 污染则 verdict cache 同样被污染；external hook 0 模型参与、0 污染路径）；延迟 **1**（verdict cache lookup O(1) 命中，cache miss 调 LLM O(seconds)；hook 字符串匹配 O(μs)，6 数量级差）；正确性覆盖 **2**（少量互补，但 v2 治理「走外部不靠 AI」已划边界）；审计/可观测 **2**（append-only 可重放 vs reason 字符串，相当）；场景 D 互补 **1**（例 1 主人偏好 + 例 2 SelfAssessmentCache + 例 3 多 agent 互审已覆盖 self-introspection 全部应用场景，接进是边际冗余）；**加权 0.28/5 → 降级**（保持 L2 哲学标准，`RUNTIME_ENFORCED=false` **永久**）。13 键仍用于 hook deny reason 引用 + CapabilityDescriptor risk 分级 + 哲学语义定义。
- **7 项待拍板决策（原表，owner 决策位）**：① A4 MemoryBackend trait 形态 —— (a) trait + SQLite impl（向后兼容）/ (b) enum-dispatch 静态分发 → **推荐 (a)**（trait 更利于 v2 plugin 架构）；② A3 perception alpha 深度 —— (a) 仅 Text + 4 trait forward-declared / (b) Text + 简单 Voice → **推荐 (a)**（P4 不阻塞 alpha）；③ A1 council advisor 是否每个独立 LLM 调用 —— (a) 是（严格隔离）/ (b) 多 instance 复用（同 provider 不同 model/temperature）→ **推荐 (b)**（成本可控 + 隔离足够）；④ A2 team-lead worktree 工具 —— (a) 完全砍 / (b) 砍但留 trait 扩展位 / (c) 全实现 → **推荐 (b)**；⑤ A6 frequency limit 触发粒度 —— (a) 进程级 / (b) 用户级 / (c) Capability 级 → **推荐 (c)**（粒度最细，避免误伤合法用例）；⑥ A1 council 决策与 governance Decision 关系 —— (a) council 输出新 GovernanceDecision 4 态扩展 / (b) council 输出 Allow/Deny，runtime 转 4 态 → **推荐 (a)**（清晰分层）；⑦ A4 File impl 加密 —— (a) keyring + AES-GCM / (b) 纯 AES-GCM（keyring 是可选密钥源）/ (c) 不加密（文件系统权限）→ **推荐 (a)**（与 credentials 接线同路径）。
- **RC 进展（2026-08-27 收盘，HEAD `67c06d95`）**：✅ 7/10 RC 真实现完成（RC-1 MemoryBackend SqliteBackend / RC-2 Experience / RC-3 PreferenceStore / RC-4 SelfAssessmentStore / RC-8 SubSupervisor（改名 `TokioSubSupervisor` → `StdSubSupervisor`，诚实反映 std::process）/ RC-9 keyring / RC-10 File AES-GCM）；⏳ 3/10 待 LLM API key + 硬件（RC-5 Orchestrator runtime LLM harness / RC-6 Council multi-LLM + 60s timeout / RC-7 Perception 真 modality Whisper·screen capture）；认知模块由其他 dev 推（hook ABI + 集成 + lifecycle invariants，3 commit `a699c5f5`/`1d227d6a`/`64e64f46`），不在本工作范围。
- **v2-native 设计原则（原文）**：多 instance LLM 隔离（v1 全独立 session 太重）/ JSON 协议（v1 free text）/ 单一事实源 plugin registry（v1 多 registry）/ trait-based 多态（v1 if-else 散落）。**0 漂移承诺**：9 哲学锚（8→9，加 O-6）/ 13 键 LOCKED 数据 / 3 项不可变脊柱语义 / workspace.version 0 改；不触碰 LOCKED public API。
- **接手人 5 条 actionable advice（per 子代理 D handoff）**：① 优先做 RC-5/6/7（需 LLM key），不要重做 RC-1/2/3/4/8/9/10（写完真完成，0 改）；② 哲学锚编号 ledger 待核（12 vs 23 项），别重复兑现；③ 12 consumer 弃用迁移（alpha `#[allow(deprecated)]` → v2.0 release 前删，rc 后必破）；④ RC-10 补 line header AAD tamper 保护；⑤ 认知模块不变量（看 commits `64e64f46` / `1d227d6a` / `a699c5f5`）。
- **子代理 4 项报告（A/B/C/D，全部采纳）**：A（`5dc29cb`）Send+Sync 注释（不写 `#[derive(Send, Sync)]` 因 unsafe trait）；B（`792f5a97`）v1 vs v2 **41 项差异**（A6 / B7 / C14 / D14）+ 5 风险（RC-7 硬件依赖 / Council 60s timeout / 12 consumer 弃用 / schema 兼容 / O-6 守门回归）；C（`9d60deea`）P0 build break（RC-2 untracked）+ RC-8 命名错位（commit `4e4fba89` 修）。

### §2.5 阶段标尺 × 代码现实差距矩阵（原件：stage123-gap-matrix.md）

- **性质**：只读代码审计 + 逐条对照文档，**不改任何代码、不改任何标尺原文**，只做「标尺条目 → 当前实现 → 判定 → 证据」对账。标尺来源 = 创世设计三部曲（[`docs/archive/_archived/r14-design/review-stage1-stage2-stage3.md`](../../archive/_archived/r14-design/review-stage1-stage2-stage3.md) §1.3 十三条上层共识 / §2.3 十项工程边界 / §3.2 图纸交付 / §4 六阶段路线）+ ``docs/archive/stage2/stage2-decisions-*.md``（14 决策 + 2 补充）+ ``docs/archive/stage3-blueprints/``（01-overall-architecture / 02-process-topology / 03-decision-flow / 04-upgrade-flow / double-onion-explicitization）。核查对象 = `crates/` 18 crate 全部源码与测试 + `frontend/companion-desktop` + `.github/workflows` + `docs/` 现状文档；排除 `legacy/`、`research/source/`、`crates/_archived/`。判定图例：✅ 完全实现 / 🟡 部分实现 / ⬇️ 显式降级 / 🔄 替换实现 / ❌ 未实现 / ❓ 无法判定。**词义警示**：本仓库存在两套「阶段 6」——(a) R14 六阶段路线的「阶段 6 = 里程碑验证机制」（本矩阵附录 1 的对象）；(b) R20 发布周期的「阶段 6 = 1.0 release 清单」及 SDK 注释中的「阶段 6 stub」（均与本标尺无关，勿混淆）。
- **§A 阶段 1 十三条上层共识（逐条）**：A1 平台不定义关系（🟡——平台侧只做提供/约束/记录三件事，但 `partner.rs` 预设了 `BondStage` 七阶段枚举；建议标尺明确「平台可提供记录词汇表、不得强制关系语义」）；A2 中央 AI 完整自由（🟡——思想/判断两层自由成立，**目标层缺位**（`GoalService` 自述 "Production wiring: none"；SGI 写入流显式记为未实现），「三层自由」实际是「两层自由」）；A3 不假装灵魂同一（✅——IdentityCard continuity_id UNIQUE + 迁移历史 + tombstone 物理禁硬 DELETE + `migrate_subject` 迁移副本写真实 continuity_id）；A4 关系开放（🟡——多用户并存达成，关系类型与阶段是平台预设枚举，无用户自定义关系类型入口）；A5 平台三件套（🟡——三件套齐备，但**对称性只做到「人类行为被记录并拥有否决权」**，未做到「人类行为同样受规则约束」）；A6 双根可演化但需重治理（🟡——双根结构齐，但 5 重守门**未作为统一串联机制落地**：第 3 重「多 AI 一致」仅由 Council 承担且默认席数被旋钮压到 3，第 5 重「反思期审计」默认不运行）；A7 双洋葱正交（✅——`PrincipleOnion` E/S/A/M/O 5 切片 + `PermissionOnion` L0-L5 6 切片 + `DoubleOnionGate::unify_check` 三段门 + 5+6=11 编译期断言 + r177 等价测试）；A8 七席审议庭（✅——7 个 `AdvisorKind` 固定，编译期封闭，各一 `LlmAdvisor` 真接 LLM + live 测试）；A9 分层验证网（🟡——defense-in-depth 精神落地，但 L1-L5 五层验证网**仍是文档清单**，未成为逐层打分/通过标准的机制，「阈值分层可调」无统一旋钮面）；A10 anchor 对应表（🔄——6 主锚人工对应表被 9 哲学锚编译期枚举替换，覆盖是超集，属合理演进，**建议标尺正式改写为 9 锚**）；A11 与后续 5 阶段衔接（✅ 文档层面——交接清单以文档形态存在并被实际使用）；A12 勘误与边界声明（🟡——P0 漂移跟踪表 5 项保留 verbatim + 状态机，承诺不可删行，但 5 项 P0 **至今全部停留在「待修订」**；P0-05 在 v2 单进程形态下已事实消解）；A13 §19 增补 4 项（🟡——七席不新增 ✅、风险=触及的权限 ✅、HA=Windows 认证 🟡（`RealHuman` 带 biometric_data/Windows Hello 字段、keyring 走 Windows Credential Manager，但人脸/指纹/声纹认证无实接线）、双洋葱比喻可替换 ✅）。
- **§B 阶段 2 十项工程边界（逐条）**：B1 Rust 单栈（🔄——Rust + tokio 1.40 保持、无 Python/Node 主栈（pyo3 仅 feature 门控桥），但「sled+Qdrant+Tantivy」被 **SQLite 单库 + sqlite-vec** 替换）；B2 B+E supervisor（🟡——5 子树枚举 + OTP 风格重启策略 + 真实进程监督都做了，但 **B+E 进程树形态本身不存在**：无 PID 1 root supervisor、`StdSubSupervisor` 未被 gateway/cli 装配、无 permanent/one_for_all、无 cgroup 限制；当前产品是「单进程 gateway + 桌面侧车监督」）；B3 9 大 crate（🔄——18 crate 四组制替换 9 大/30 crate 分层制，映射关系大体保留，**建议正式修订标尺**）；B4 5 层总线（🟡——L0 进程内 broadcast + L4 HTTP/WS 达成，**L1 unix socket / L2 异构子进程 pipe / L3 gRPC 全仓 0 命中**，这也直接卡死 B2 的多进程 supervisor 树）；B5 6 DB 协同（🔄——**SQLite 单库 + sqlite-vec + 自研向量/图基元 + 联想 trait** 替换 6 DB；Wave 联想网络只有 trait 口与图存储基元，无独立联想网络引擎）；B6a 8+ LLM providers（🟡——3 家 canonical provider + 协议适配层 + embeddings provider，OpenAI-compatible 口可承载 DeepSeek 等；**本地管家（Ollama/LlamaCpp/VLLM）无实现**，fallback chain 无「本地兜底」层）；B6b SemanticModelRouter（🔄——`ProviderRouter` 按 model id / 健康度路由，**语义级模型选择退化为路线图项**）；B7a V3 9 键 verdict 键表（🔄——13 键 = V3 9 + v4.1 3 + PHL-07 1，**加严方向的超集**）；B7b 编译时 hardcode（✅——13 键数组长度 + 分组数全部编译期断言，增删键立即编译失败）；B7c 13 键 verdict 的运行时强制（⬇️ **显式降级**——`RUNTIME_ENFORCED: bool = false`，13 键降级为「哲学标准/判别词汇表」，**不是运行时拦截器**；决定出处 `ROADMAP.md:31`、MANIFESTO §95-109「已拍板降级，不要接回 runtime 强制」、`o6-session-log-2026-08-27.md §2.1`（2026-08-27 五维评分 0.28/5 拍板）；替代机制 = external hook 闸 + 场景 D 长程 AI 判断）；B7d 物理多签（🟡——M-of-N 逻辑多签 ✅（required/total、去重真实人类、single/multi/offline 三模式、越界校验、负向测试），**物理（密码学）多签 ❌**：签名是 `"name:digest"` 字符串票，自述「0 装占位（signature 是 hex string，不是真 crypto 校验）」，真 Ed25519 多签仍留 v2.1）；B7e 反思期（🟡——`Gate::ReflectionAudit` + 24h 反思期 IDLE 状态机 + Self-Disable 4 项违规扫描 + `ReflexionModule` 已接线但**默认关**（`APEIRETH_ENABLE_REFLEXION=1` 才注册）+ 守夜人 Nightwatch 离线审计 report-only；**无 24h 周期调度器**自动跑）；B8 7+N 席智囊团（🟡——7 强制席 ✅ 真 LLM advisor + N 动态注册口，但**运行时旋钮 `APEIRETH_COUNCIL_ADVISORS`（1-7）默认 3**，与「7 强制席」字面承诺不一致）；B9 OTA 7 阶段（🔄——实际是 **L0-L5 六步自升级 cycle**：Intent 🟡（无 UpgradeIntent/6 历史流写入）→ Council ✅ → MultiSig ❌（无多签环节，仅主人 Veto 口）→ Sandbox 🟡（L3 留白）→ Switchover ❌ → Monitor ❌ → Done 🟡（tag 建议）；**部署侧安全闭环整块缺失**）；B10a 自主目标 D2 §3 SGI（🟡——`GoalService` 单目标状态机（CAS revision、阶段迁移、崩溃安全持久化）为机制库，自述 "Production wiring: none"，SGI 写入流 + C-SGI-1..7 显式记为 "Not recovered"）；B10b 主体连续性 ID（✅——`IdentityCard.continuity_id` UNIQUE 跨载体 + 迁移历史 + tombstone + `continuity_sessions`/`history_streams` 按主体审计；唯一瑕疵 `EpisodeStore::put_episode` 仍硬编码 `continuity_id="default"` 占位，已用 `put_episode_for_subject` 收口并有测试拒绝空 id）；B10c 双根可演化但需重治理 MEWG（🟡——双根演化流程在，**加权治理的权重表（Safety=1.00…History=0.55，7 域）实现为 `rubric.rs` 但自述「不接入任何循环」**，根层变更**没走 MEWG 加权裁决的代码路径**，5 重守门也未在根变更点串联）。 〔四级口径：原篇表述，归属见本节现状复核〕
- **附录 2：已记录的降级 / 漂移（12 条，逐条）**：① 13 键 verdict cache 降级为哲学标准（`RUNTIME_ENFORCED=false`，依据 2026-08-27 五维评分 0.28/5）→ **生效中且被标注「永久」**；② stage2 §14 P0 漂移 5 项标「待修订」→ **5 项全部仍为「待修订」，无一进入「已修订」**（P0-04 的 X1/X2/X3 阈值至今未校准；P0-05 事实消解但仍挂账）；③ 24 LOCKED 入口签名形式撤销，仅保 3 项不可变脊柱（R148，主人 2026-08-11 拍板）→ 生效中；④ 物理（Ed25519）多签显式推迟 v2.1 → 挂账中（M-of-N 逻辑多签已实现）；⑤ UpgradeCycle 三处留白（L3 sandbox regression / L4 主人 Veto dashboard / L5 git tag 只建议）→ 挂账中（**OTA 7 阶段的 Switchover/Monitor/回滚无对应物且无单独降级记录，属实现缺口而非已拍板降级**）；⑥ 评审类机制只降级、不枪毙主任务（judge/council 预算耗尽、空响应、坏 JSON → 降级放行并留痕；显式否决必须用诚实错误码）→ 生效中，**与「分层验证网 fail-closed」的标尺语义方向相反，属有意识的体验取舍**；⑦ **生产旋钮整体默认关**（reflexion / council / onion layer / onering ledger / community triage / shell / fetch 等均 opt-in）→ 生效中，意味着本矩阵多处「🟡 接线在、默认不跑」是**同一策略的产物**；⑧ 86 crate → 13 crate → 18 crate 拓扑收敛（替换 stage2 的 9 大/30 crate 划分）→ 生效中，但 stage2 原决策文本未同步改写；⑨ `ROADMAP.md:31` 的部分「未实现」表述已过时（现 memory/vector/graph/MCP/voice 均已有实现）→ **文档漂移（未更新）**；⑩ council live 假绿洞封堵（曾以降级出口冒充 live 通过，撤销 key 下 0.67s 假绿）→ 已修复并立规「live 测试先证通道活着，不许降级出口冒充绿」；⑪ 风险→席位触发矩阵与按住阈值未校准（critical 7/high 5/medium 3/low 1/info 0 仅注释态；30%/60s/3 轮 = X1/X2/X3 待校准）→ 挂账中；⑫ Kani 形式化证明以等价 Rust 断言测试替代（r177 编号保留，真 Kani 接线为后续项）→ 挂账中。 〔四级口径：原篇表述，归属见本节现状复核〕
- **结论① 达成率（27 条 = 13 + 10 + 4）**：✅ 完全实现 **5（19%）**（A3/A7/A8/A11/C4）；🟡 部分实现 **15（56%）**；🔄 替换实现 **6（22%）**（A10/B1/B3/B5/B9/C2）；⬇️ 显式降级 **1（4%）**（B7c）；❌ 未实现 **0**（组件级 ❌ 大量，见各行）；❓ 无法判定 **0**。**读法（原文）**：27 条中 26 条有对应物（哲学与结构层几乎全部「落在代码某处」），但**只有 5 条完全达成**；**真正的空洞集中在「最后一公里接线」与「部署形态」两处**——判定层完整而执行面默认关/未接（C1、B7c、B7e、B10c）、决策闭环完整而部署闭环整块缺失（B9/C2 的 Switchover/Monitor/回滚、B2/C3 的 supervisor 树、B4 的 L1-L3 总线）。
- **结论② 最重的 10 个差距（按损伤度排序）**：① 阶段 6 里程碑验证机制从未启动（验证清单是文档不是关卡、真实人类批准没成为机制环节、反思没随里程碑归档）；② OTA 7 阶段的部署闭环整块缺失；③ 13 键 verdict 运行时强制被显式降级 + V1+V2 AND 门不在执行主链；④ 物理多签缺位；⑤ 5 层总线只剩 L0/L4 两层；⑥ B+E supervisor 树不存在；⑦ 风险→席位触发矩阵与按住阈值未接线未校准；⑧ 6 DB → SQLite 单库的联想/向量/全文能力弱化；⑨ 目标层自由空缺（SGI/GoalService 未接线）；⑩ 本地推理管家缺位。（并列候补：A9 分层验证网 L1-L5 未机制化；B10c MEWG 未接线；B7e 反思期无常驻调度。）
- **结论③「该做的缺口」vs「合理演进 / 该正式修订标尺」**：**A. 该做的缺口**——阶段 6 里程碑验证机制（**欠的不是代码量，是「验证清单当关卡 + 真实人类签核 + 反思归档」这一条流程闭环，成本低、价值最高**）；OTA 部署闭环；物理多签；风险→席位触发 + X1/X2/X3 阈值校准；目标层（SGI）与本地 provider 兜底；哲学守门热路径的最终形态（要么把 13 键语义并入 hook 闸并验证等价性，要么由主人正式拍板「词汇表 + hook 闸」为新标尺——**现状的「永久降级」缺一次对标尺的正式改写**）。**B. 合理演进（建议正式修订设计文档）**——6 DB → SQLite 单库 + sqlite-vec；9 大/30 crate → 18-crate 四组制；5 层总线 → 「L0 进程内 + L4 HTTP/WS」两级（L1 unix socket 在 Windows 上本就可疑）；B+E supervisor 树 → 「单进程内核 + 桌面侧车监督」；V3 9 键 → 13 键；SemanticModelRouter → ProviderRouter；OTA 7 阶段 vs L0-L5 cycle（**两者互补：7 阶段 = 部署形态、L0-L5 = 决策步骤，建议标尺合并而非二选一**）；P0-05 建议正式标「撤回（形态变化）」别再挂「待修订」。**给主人的决策提示（原文）**：差距分布很清楚——**哲学层「想清楚了、写成了结构、但没有强制力」；工程层「零件全、形态换」**。**「追理想」主战场 = 结论②-1/2/3/4；「改标尺」主战场 = 存储/总线/拓扑/crate 四项形态类条目**。两者不冲突：建议先以附录 2 为基础把显式降级正式写回标尺（一次标尺修订会议），再按 A 清单立阶段 6 的最小验证闭环。

---

## §3 主题 C —— 存储 / 向量 / 图（Round 11 · R11-Storage，原件 8）

- **5 条关键发现（主代理必读，原文）**：① **🔴 主代理真账 §1.1 Storage 层 3 项标 ❌ 0 真实施是误读** —— v2 已在 `crates/engine/memory/src/canonical/` 真实施 VectorIndex + MemoryGraph + ACT-R 检索，真账 §1.1 需修订；② **🟡 hybrid (cosine + BM25) 实际只实现 cosine + ACT-R activation**，BM25 在 `lightmemo/search.rs` 是 LightMemo 4 层局部；③ **🟡 1.0 真账在 `_research_mem/apeireth-rust-fork/crates/{apeireth-vector,apeireth-graph-primitive}/`**，`legacy/donor/apeireth-storage/` 不存在；④ **🔴 v2 `reconstruction_v2/crates/apeireth-storage/` 是 empty shell**，真账在 `crates/engine/memory/`；⑤ **🟢 参考脉络逐项可用**：1.0 `apeireth-vector` + `apeireth-graph-primitive` 比 v2 canonical 更完整。
- **1.0 VectorIndex 真账（REAL）**：`_research_mem/apeireth-rust-fork/crates/apeireth-vector/src/` —— `lib.rs:1-39`（向量子系统 V2 P1 战区 4，BM25 + vector 共存）、`traits.rs`（`VectorStore` trait 抽象）、`sqlite_backend.rs`（`SqliteVecBackend`，R19 P2 真接 `sqlite-vec` vec0，10w × 768 维 KNN P99 < 50ms）、`qdrant_compat.rs`（`QdrantClient` + `ScoredPoint`，R150 P1 #6）、`distance.rs`、`organ_kani_proofs.rs`（R177 organ invariants，5 tests + 2 Kani）；仅 1 fn 需 unsafe。原始路径 `legacy/donor/apeireth-storage/src/vector.rs`（per master audit L206）：in-memory only，**0 持久化**，hybrid cosine + BM25 混合检索器。
- **真账修订表（原表）**：#1 VectorIndex（主代理原文 🔴 缺 → 真调研 🟡 partial：cosine ✅、BM25 hybrid ❌，P0 估 1-2 周，派 sub-agent，借 `apeireth-vector::{traits,sqlite_backend,distance}`）；#2 Graph primitives（🔴 缺 → 🟡 partial：graph primitives ✅、causal engine ❌，P0 估 1-2 周 + P1 估 2 周，借 `apeireth-graph-primitive::{lib.rs,query}`）；#3 Memory support（🟡 partial → 🟢 OK：22 modules 语义对齐，ONNX stub 待决策，P1 估 1 周 + P2 估 2 周，**主代理亲做 ONNX 决策**）。
- **参考脉络（O-2 四条）**：1.0 真实施（完整 fork，含 trait 抽象 + 双 backend + BFS/DFS + predicate query + 4 关系枚举 + Kani）；2.0 canonical 已就位（`crates/engine/memory/src/canonical/{vector,graph,retrieval,domain,error,repository,sqlite}.rs`）；研究/source 参考（R10：Qdrant + sqlite-vec + SurrealDB + Neo4j + Cypher + Kani）；**未采用候选**：pgvector + LanceDB + llamacpp。
- **优先级表（原表）**：P0 VectorIndex trait + SqliteVecBackend 语义对齐（1-2 周）、P0 BM25 hybrid fusion 入口（1 周，借 `lightmemo::MultiPipeSearch` + `dailynote::search`）、P0 RelationKind enum + predicate query（1 周，借 `apeireth-graph-primitive::{lib.rs 4 enum,query}`）、P1 DFS iterator + causal engine（2-3 周，**主代理亲做 spec**）、P1 ONNX stub 决策（DROP / 真接 / ADAPT，1 周，**主代理必亲做**）、P2 QdrantVectorIndex + Kani 5+2 invariants + experience_store 接入（2-3 周）。**总估时 P0 3-4 周 + P1 3-4 周 + P2 2-3 周 = 8-11 周（2-3 月）**。
- **5 条 0 装诚实标（主代理修订项）**：主代理真账 §1.1 三项 ❌ 误读；**本真调研 0 实测 cargo**（未运行 test/clippy/build 验证 `canonical/vector.rs` + `canonical/graph.rs` 编译与 7 个 canonical test 通过：canonical_vector_graph / canonical_memory / canonical_retrieval / canonical_organ_kani_proofs / migration_v1_to_v2 / canonical_sqlite_repository / canonical_trait_bounds）；**路径混淆**（主代理写 `crates/apeireth-storage/src/vector.rs`，实际 `legacy/donor/apeireth-storage/` 不存在，完整 1.0 fork 在 `_research_mem/apeireth-rust-fork/`）；`reconstruction_v2/crates/apeireth-storage` 是 empty shell（无 Cargo.toml / 无 src/）；v2 `reconstruction_v2` crates 仅 5（cli / gateway / pybridge / runtime / storage，**不完整**，完整 v2 crates 在 `crates/`）。

---

## §4 主题 D —— 长期记忆塑形（Round 11 · R11-LongTermMemory，原件 9）

- **范围与规模**：6 模块 = daily_summary 99 + diary 442 + cross_diary 301 + memory_injection 66 + reflexion 497 + reflection 329 = **1734 行** 1.0 真账（1+1+1+1+1+1 = 6 文件，实读 wc -l 推论）。
- **0 装诚实标（派单原话 + 三条）**：派单原话「daily_summary / diary + cross_diary + memory_injection（长期记忆塑形），估 2-3 周，派 sub-agent 真调研（r7 调研真账 §5 #5 已推荐）+ 真实施（跟 R20 + R22 critical path）」；**0 实测**（未 git clone v2 master branch，仅读 1.0 真账 + 2.0 handbook + r7 调研真账 + 1.0 vs 2.0 gap 真账推论）；**0 数字漂移**（上述行数为 1.0 真账行数）；**5 重守门 + LOCKED 0 触碰**（doc-only，不改 src，不触碰 9 哲学锚 / 13 键 / 3 项不可变脊柱 / Cargo.toml version / R11 baseline）；**0 装诱导 prevention**（不假装「v2 已实施 daily_summary」/「v2 已实施 cross_diary」——实际 0 真实施）。
- **consolidation_writeback pipeline（原文图）**：`daily_summary`（日粒度统计）→ `cross_diary`（跨日记关联索引，shared_tokens 共享）→ `memory_injection`（反幻觉闭世界证据）→ `reflection`（R22，周期到 OR importance > 150 触发；可选深度反思器 LLM）→ `reflexion`（R21 + 强化闭环，失败轨迹 → 反思 → 重试注入）→ `memory_writeback`（append-only Episodes，cognitive.slot #6 WIRED）→ `consolidation`（跨日记 / 记忆图 / 反思记忆聚合，物种化塑形固化）。
- **真实施路径（3 步）**：① cognitive module `consolidation_writeback_pipeline` trait 增维（主代理 spec，1-2 天，LOCKED 5 项 0 触碰，走扩展 trait 接口）；② §1.4/§2.4/§3.4/§4.4 真实施派单并发（3-4 周）；③ CognitiveOrchestrator 真接线（AfterTurn 钩子挂 pipeline，1-2 周）。**总估时 4-6 周 critical path**（§5.3 表另给 6-9 周版本，含 E2E + 5 重守门）。
- **决策表（原表）**：P0 #1 **memory_injection**（66 行，low hanging fruit，2-3 天，阻塞 0）；P0 #2 **R22 reflection**（1 周，`apeireth-cognitive::reflection` 语义对齐 329 行，DEFERRED INTO SELF-ASSESSMENT）；P0 #3 **daily_summary + diary**（541 行，1-2 周，阻塞 #1-#2）；P0 #4 **reflexion 闭环**（497 行，1-2 周，阻塞 R22 done，跟 R21 并行）；P1 **cross_diary**（301 行，1 周 + 2-3 周 storage graph，**依赖 `apeireth-storage::memory_graph` storage 抽象层**——即被 §3 的 R11-Storage 卡住）。
- **与 vision.md 对位（原表）**：「物种而非个体」↔ per-user memory / preference / curiosity / emotional timeline 物理独立；「记忆/偏好/好奇形状被共同生活塑形」↔ 长期记忆塑形全链路；「同一个 Apeireth, 不同的『人生』」↔ per-user consolidation pipeline 实例独立（本地优先 + portable binary）。

---

## §5 主题 E —— 协调与上下文（Round 11 · R11-CoordinationContext，原件 10）

- **4 条关键发现（原文）**：① **v2 9 项 0 接**——除 emergence organ + organ world_model 子集（oracle trait 语义对齐）+ 6 crate `organ_kani_proofs`（R177 已装）外，**其余 6+ 项 v2 workspace 完全 0 真实施**（grep：spill/milestone/experiment_field/bridge_kani_proofs/pentest/ProgressiveCatalog/OneRingLedger/ContinuationStore/assemble 0 hit）；② **重复实现警告**——v1 `context.rs`（L141-451）+ `context_rot.rs`（L140-174）**同一 rot_score 两 file 各实现一遍**，v2 真实施前必先融合；③ **概念 collision**——主代理真账把 `hello.rs` 列「启动/装配」，实际 v1 `hello.rs` 是 **Windows Hello 生物识别（NGC 凭据探测）**，真调研按 v1 真账画像；④ **主代理真账 §1.8 部分不准确**——`oracle/oracle_adapters` 标 ❌（实际 organ world_model trait 已语义对齐）、`proactive` 标 ❌（实际 E7 emergence organ 含 8 重 gate 已真实施）、`bridge_kani_proofs/organ_kani_proofs` 标 ❌（实际 6 crate organ_kani_proofs 已装，R177），**本真账修订**。
- **onering 1.0 真账（REAL，346 行）**：OneRing 统一上下文账本——跨前端（SSE/Web/同类 IM 2 项/CLI/proactive）**同一 Agent 唯一时间线**，每条发言留痕 `(seq, continuity_id, role, sender, frontend, content, ts)`，seq 单调自增确定性排序，账本只保留最近 N 条（默认 200，可配）；自有表 `onering_messages`（**不污染 episodes 记忆管线**）；API `OneRingLedger::new/record/record_as/recent/len/with_max_records`；8 测试（含 `cross_frontend_same_timeline` 核心项）。v2 现状 = ❌ 0 真实施（主代理真账准确），已就位 `crates/engine/memory/src/identity.rs`（IdentityCard continuity_id UNIQUE）+ `continuity_link.rs` + `episode.rs`（`EpisodeQuery::for_continuity`）；X-Apeireth-Continuity HTTP 头雏形（per master audit L231）未装。真实施路径 A（推荐，2 周）：新 crate `crates/engine/coordinate/src/onering.rs`。
- **9 项优先级表（原表，含物种化锚与估时）**：P0 continuity（1 周，锚「同一个她」基础）、P0 milestone（1 周，锚物种化塑形节点）、P0 onering（2 周，锚跨前端「同一个她」）、P0 context/context_rot（3 周，**必先融合**）；P1 progressive（1 周）、P1 continuation + spill（2 周，复用 v2 `FrozenTurnContinuation`）、P1 proactive（2 周，接 E7）、P1 pentest（2 周，走 governance onion）；P2 experiment_field（3 周，vision L40 自我改进独立实验场，smol-vm/libkrun 调研）、P2 oracle/oracle_adapters（3 周，复用 v2 organ world_model trait）、P2 Kani proofs（2-3 周）；P3 assemble（4 周，**物种化塑形主装配层**，4 源统一注入）、P3 hello（2-3 周，NGC 凭据探测）。
- **真实施 brief（9 项共用）**：任务 = 写 1.0 vs 2.0 [项名] 真调研 + 真实施路径；必读 = 本 R11 真账 + 1.0 真账 `.rs` + `v2-reference-handbook` + 5 R7 真调研 + master audit；输出 = ``docs/04-internal/r12-[项名]-implementation-2026-08-28.md`（≤ 200 行）；约束 = 不写真账以外 file / 不 git add-commit-push / 0 触碰 LOCKED / ≤ 4h / **必含物种化锚 + 0 装诚实标 + 5 重守门**。
- **4 条 0 装诚实标**：主代理真账 §1.8 状态不准确（oracle / E7 emergence / Kani proofs 三项修订）；v1 `context.rs` + `context_rot.rs` 重复实现（真实施前 1 天先融合）；v1 `hello.rs` 概念 collision；**R11 真账 0 实测**（未 git clone v2 master branch，仅 grep 验证）+ **11-13 周 critical path 是经验估时**（启发式估时，真实施每周回 R11 真账更新）。
- **LOCKED 0 触碰（当时口径）**：9 哲学锚本体 / 13 键 verdict cache / 3 项不可变脊柱 / workspace.version / R11 baseline 3 值（`R11_V1141_BASELINE: f64 = 0.8682` / `R11_V1131_BASELINE: f64 = 0.8532` / `R11_V1136_BASELINE: f64 = 0.9063`）；**新增：9 项语义对齐 v1 时必用新 crate 或新 module，不改现有 crate 公共 API**。
- **R12-CoordinationContext-1 融合 spec（主代理亲做 spec 决策冻结，原件 21）**：触发 = 用户原话「立刻 push, 立刻做点小的, 边做边更新文档, 注意文档工程规范, 哲学锚, 追求整体系统架构最优」（Round 15，commit `800bdb1a` push 成功）。**1.0 真账实测**：`context.rs`（L141-451，~770 行）= `RotBlock` + `RotConfig` + `RotWeights`（w_duplicate 0.4 + w_stale 0.3 + w_irrelevant 0.3）+ `DuplicatePair` + `RotBreakdown`；`context_rot.rs`（L1-180，~526 行）= `Segment` + `RotConfig`（w_repetition 0.4 + w_staleness 0.3 + w_relevance 0.3，含 `stale_half_life_turns`）+ `RotBreakdown` + `repetition_factor`（多行去重比 + char 6-gram 滑窗）+ `query_tokens`（ASCII 小写词 + CJK char-bigram）+ `rot_breakdown` + `rot_score`。**三方案**：**A（推荐）** = 在 `context.rs` 添加 `rot_score` 实现（从 `context_rot.rs` 参考 `repetition_factor` + `query_tokens` + `rot_breakdown`），`context_rot.rs` 保留独立 `Segment`（compaction 原语，不重复）；**B** = 完全删除 `context_rot.rs` 全部合并到 `context.rs`；**C** = 保留两 file 但去重 `rot_score`（`context_rot.rs` 借 `RotBlock` + `RotConfig`，保留 `Segment`）。**主代理推荐 A 的理由**：1.0 `context.rs` 提供 `RotBlock` + `RotConfig` + `RotWeights`（启发式 3 因子公式 + 重复度 + 权重配置），`context_rot.rs` 提供 `Segment` + 完整 `rot_score` 实现（含 `stale_half_life`）；对接 v2 cognitive memory 模块增维路径（per R11-LongTermMemory 6 项）；**0 装 PASS 标注——1.0 真账 self-flag「0 装 PASS」，不假装 `rot_score` 准确，明示启发式 + 待 A/B 调权重**；0 引新外部 dep。**命名统一表**：`RotWeights`(w_repetition + w_staleness + w_relevance = 0.4/0.3/0.3) + `RotConfig`(now_ms + stale_threshold_ms + ngram_size + duplicate_threshold + trigger_threshold + weights + latest_user_message + pinned_block_ids + min_chars_per_block) + `RotBreakdown`(repetition + staleness + irrelevance + score) + `rot_score(seg, query, cfg)` + `rot_breakdown(seg, query, cfg)` + `repetition_factor(content)` + `query_tokens(query)`。**派单**：派 sub-agent 真实施（R12-CoordinationContext-1，3-4 周 critical path，不依赖网络）——代码 = `context.rs` 添加 `rot_score` + `rot_breakdown` + `repetition_factor` + `query_tokens`，`context_rot.rs` 保留独立 `Segment`；集成测试 = cargo test + 真账对接 + species 塑形边界；主代理亲验 commit + push。**该融合是 Round 13 遗留 27 项未实测 .rs 中的 1 项。**

---

## §6 主题 F —— 反思与元认知（Round 11 · R11-MetaCognition，原件 11）

- **范围**：6 模块 = meta_thinking + reflexion + reflection + thought_cluster + intent_brier + confidence；用户 directive = 「看 1.0 缺什么, 2.0 最终功能应该和 1.0 相同, 但架构不同而已」；**v2.0 release 必须 = v1.0 功能全集 + 架构升级——release 阻断**。
- **认知层循环（原文图意）**：`meta_thinking`（元思考递归链）→ `thought_cluster`（思考聚类）→ `reflection`（反思周期写回）→ `confidence` ← `cognitive.council/judge`（多意见加权 + 反思证据）→ `intent_brier`（Brier 校准意图）→ `reflexion`（反思循环：失败 → 反思 → 重试注入）→ 回 `meta_thinking`。
- **对接真账（原文）**：R22 reflection = 语义对齐 v1 `reflection.rs`（Triggered→Reflecting→Consolidating→Concluded 4 阶段 cycle，写回 SQLite【反思周期】episode）；R11 长期记忆塑形 = 派 1 sub-agent 调研 daily_summary/diary + cross_diary + memory_injection（跟 R20 + R22 并行）；cognitive.self_assessment = **WIRED，Judge-backed**，AfterTurn 真实 Judge 结果（不假装评分）；meta_thinking + thought_cluster = 反思的元自学习读取口。**R20 preference_learning（2-3 周）+ R11 长期记忆塑形（2-3 周）+ R22 reflection 三者并发是 critical path**（per r7-mio §77）。
- **实施顺序 + 估时（原表）**：R22 reflection（1 周，阻塞 R12 ✅）→ meta_thinking（1 周，**含 R22 同期**，共享 `ReflectionReflector` trait）→ thought_cluster（1 周，含 R22 同期，共享 trait）→ R11 长期记忆塑形（2-3 周）→ reflexion（1-2 周，post-R22）→ intent_brier（1-2 周，**跟 W1/W2/W3 world_model 并行**，公式同源 per v1 L11-14）→ confidence（1 周，**跟 council + judge 绑定**）。**总估时：串行 12-14 周；并行 6-10 周。**
- **6 项真实施 brief 要点（原文）**：R22 → 新 crate `crates/engine/reflection/` + `OrganKind::Reflection` variant（R10 加不破 9 现有）+ `ReflectionScheduler` + `ReflectionReflector` trait（lib 0 LLM）+ N4 thought_reader 接入 + 写回 SQLite【反思周期】episode；**关键 0 装诚实标**：`cognitive-module-wiring.md` L32「DEFERRED INTO SELF-ASSESSMENT」——R22 要 OWN `cognitive.reflection` slot（短期 current-turn 仍是 self_assessment，长期 cycle = reflection module owner）。meta_thinking → `crates/engine/reflection/meta_thinking.rs` + `MetaThinkingChain` + `MetaThinker` + `ReflectionMetaThinker` + `ChainReflectionThinking` + `save_to_cluster` 集成 + 单测语义对齐 v1 8 测试。thought_cluster → 新 crate `crates/engine/thought_cluster/` + `ThoughtClusterManager` + `ThoughtClusterReader` trait + 集成 R22 reflection `with_thought_reader` 注入 + 单测语义对齐 8 测试。reflexion → 新 crate `crates/engine/reflexion/` + `ReflexionStore` + `Critic` trait + `RuleCritic`（确定性规则版先行）+ `LlmCritic`（LLM 留部署层）+ 失败事件接线（`cognitive.judge` AfterModelResponse：verdict = retry/stop；`cognitive.self_assessment` AfterTurn：score < 阈值）+ 重试注入接线（`cognitive.preference_recall` + `cognitive.memory_recall` TurnStart，预算 ≤ N chars）+ 单测语义对齐 5 测试。intent_brier → 新 crate `crates/engine/intent_brier/` + `IntentPrediction` + `FeedbackOutcome` + `IntentRecord` + `IntentLedger` + `brier_score`/`mean_brier`/`compute_window`/`compute_trend`/`domain_diagnostics`/`compute_report`/`render_report` + 集成 `cognitive.self_assessment`（反馈 = Agree/Correct/Silent；源 = Judge verdict）+ 集成 `cognitive.preference_recall`（render_report 注入 system prompt）+ 单测语义对齐 31 测试。confidence → 新 crate `crates/engine/confidence/` + `BetaBinomial` + `Strength` + `observe` + `mean` + `interval95` + `strength` + `report`，**保留本地 `organ::world_model::CalibrationStrength`**（per 0 装诚实 + 依赖最小，双向 optional borrow）+ 集成 `cognitive.council`（`AdvisorVerdict.confidence: Option<f64>` → `Option<BetaBinomial>`）+ `cognitive.judge`（critique 置信度基于 BetaBinomial，**不再 LLM 自报**）+ `cognitive.self_assessment`（AfterTurn 置信度）+ 单测语义对齐 4 测试。
- **0 装诚实标（5 条）**：本调研 0 git clone v2 active workspace；仅读 1.0 真账（6 模块）+ 2.0 handbook + `cognitive-module-wiring.md` + 5 R7 真调研 + R21-R24 推论；**6 模块在 v2 active workspace 0 真实施**（§0 grep 实测：no crate, no OrganTrait, no OrganKind variant；仅 `organ::world_model::CalibrationStrength` 本地简化版 in-place）；E1 reflexion + N4 thought_cluster + W6 intent_brier 已入库 `legacy/`，**active workspace 0 实现**；**真实施前主代理必亲验**。

---

## §7 主题 G —— 物种化核心与塑形维度（Round 11 · R11-SpeciesCore / R11-SpeciesForm，原件 12 / 13）

### §7.1 物种化核心 4 项（R11-SpeciesCore）

- **范围**：education + partner + community + principles 4 gap 真调研；**物种化核心 = vision L47-49 三大支柱真实施缺口，不是功能语义对齐，是物种化哲学层落地**（「教『如何成为自己』，而非复制自己」）。
- **vision 对位表（原表）**：L47 物种而非个体（机制/哲学/安全同源，记忆/偏好/好奇形状被共同生活塑形）↔ community + partner；L48 她能教养后代 ↔ education（物种化核心最终形态）；L49 跨墙的信任（存在论不可逾越但墙不影响生活）↔ partner（跨用户协作 + bond + privacy）；L51 三远合一（记忆 / 物种 / 传承 = 生命三个定义）↔ 4 项总和。
- **4 项相互关系与实施顺序（原文）**：`principles`（F6 价值内化层）= **基础**（哲学价值先内化，才能 species 价值层落地）→ `partner`（跨用户 + per-user 塑形，物种化塑形的工程承载；1.0 `partner.rs` BondStage + PrivacyBoundary）→ `community`（物种化社区；1.0 `community.rs` 360 行 `detect_communities` + `triage`）→ `education`（物种化核心最终形态——**必须在 principles + partner + community 都就位后**，否则 0 教学材料）。
- **决策表（原表）**：P0 #1 principles（4 周，阻塞 0，F6 organ WIRED ready，**风险最低 / 收益最高**）；P0 #2 partner（4 周，阻塞 0）；P0 #3 community（4 周，阻塞 = Graph primitives 缺）；P0 #4 education（4 周，**主代理亲做 spec**，spec 决策最重、真实施最后）。**4 项并行 critical path = 4 周**（主代理 §3.1 估 3-4 周正确；R11 估时 4 周，真实施需 4-6 周）。
- **5 重守门 / LOCKED**：principles 跟 F6 organ WIRED 集成不改 traits；partner 新 struct / bond 阶段机；community 的 Graph primitives 缺是真实施前置但 `community.rs` 语义对齐独立；education 完全新 organ，**Phase 1 spec 必须主代理亲做**；baseline = clippy 0 warning / tests 0 fail（1739 baseline）/ legacy compat path < 100（36）/ LOCKED 5 项 0 触碰 / 9 哲学锚 0 减。
- **0 装诚实标**：**R11 0 实测**——仅读 1.0 真账（4 个 .rs：402+141+360+478 = 1381 行）+ v2 真账（grep + handbook §1.2-1.3 + F6 organ 真账）推论，**未 git clone v2 master branch**。
- **与 Round 10 五篇 R7 调研的关系（原文）**：R7 五篇 = **物种化前端 + 物种化架构参考**；R11 4 gap = **物种化哲学层落地**；**互补**——5 真调研给参考点，R11 4 gap 真实施承接参考，物种化从「调研」到「工程」的 critical path 闭环。

### §7.2 物种化塑形维度 3 项（R11-SpeciesForm）

- **范围**：timeline（时间维度）+ tone（语言维度）+ morphology（frontend 维度）；1.0 三文件 **79 + 374 + 284 = 737 行 + 26 测试**；**0 增量设计、纯对齐实现**（LOCKED 5 项 0 触碰约束下走 extension trait 路径）。
- **全维度表（原表，含 1.0 已有的三行）**：时间 = timeline.rs 79 行 / v2 ❌ 0 真实施 /「你们一起走过的时间」关系里程碑序列 / 参考脉络 = 语义对齐 v1 + reflective 维 + 日记反思；语言 = tone.rs 374 行 + 8 测试 / ❌ 0 真实施 /「她用什么语气说话」关系 × 情绪 × 审议合成提示 / 语义对齐 v1 + 原声 TTS 互补；形态（检索）= morphology.rs 284 行 + 10 测试 / ❌ 0 真实施 /「她回应多深」per-query 检索深度档位 / 语义对齐 v1 + 五维 episodic·semantic + Graph primitives；声音 = v1 无专门 crate（voice crate 借用）/ ⚠️ RC-7 TTS trait 已落、0 真 backend /「她声音像谁」per-user 原声 TTS 训练 / GPT-SoVITS；frontend 视觉 = v1 仅 `live2d_traits` 占位 / ⚠️ companion-desktop Svelte 5 + Tauri 2 post-1.0.0 PR #1 /「她长什么样」Cubism 5 Live2D / Cubism 5 + Live2D + Windows 本地优先；frontend 容器 = v1 done 1411 行 `runtime.ts` SSE/WS/panel / ✅ companion-desktop frontend 已 done /「她在哪里跟你说话」/ 便携容器 + portable binary；记忆 = v1 `memory_v2.rs` + `graph.rs` / ✅ MemoryStore v2 WIRED + **VectorIndex/Graph primitives 🔴 缺** /「她记得你什么」5 维记忆 / 五维记忆增维 + 永久在线 compaction；主动 = v1 `emergence.rs` Borbely drive / ✅ E7 emergence 语义对齐 v1 /「她什么时候找你」8 重门控 + 节律 + 沉默压力 / 永久在线 + 双引擎主动 + 日记反思。
- **物种化维度对位（原文）**：记忆形状 = 5 维记忆（episodic/semantic/procedural/emotional/reflective）；偏好形状 = per-user preference learning（R20 真实施中）；好奇形状 = E4 curiosity + `memory_echo_bias` ✅ WIRED；时间形状 = Timeline；语言形状 = tone；声音形状 = GPT-SoVITS 原声 TTS；形态形状 = morphology；frontend 形状 = Live2D 模型 + 多形态；主动形状 = E7 emergence 8 重门控 + 沉默压力。**关键发现**：1.0 timeline + tone + morphology = **物种化塑形三大底层维度**，Round 10 五篇 R7 = **三大上层维度**（声音 / frontend 视觉 / 主动），整合 = **全维度（6+ 维）**，对位 vision.md L47。
- **实施顺序与估时（原表）**：P0 立即主代理亲 git clone 1.0 三个真账 + diff v2 现状（1-2 天）；P0 Timeline 语义对齐 v1 + Milestone + PartnerId（2-3 周）；P0 Tone 语义对齐 v1 + ToneRefiner trait + 跟 judge/council 衔接（1-2 周）；P0 Morphology 语义对齐 v1 + 跟 memory_recall 衔接 + Graph 抽象层（1-2 周 + 2-3 周）；P1 companion-desktop morphology depth indicator + SSE `morphology_event` 帧（1 周）；P1 Timeline-driven LLM context 注入（system prompt 增时间线段落，1 周）；P1 Tone 注入 `prompt_assembler.rs` 真实路径（1 周）。**总估时 P0（3-5 周，三个语义对齐并行）+ P1（2-3 周，三件集成）= 5-8 周 critical path**，跟 v2 release critical path 6-9 月并行不冲突。
- **新设计衔接 4 件（每件 1 周，估时保守）**：Timeline ↔ `memory_writeback`；`organ_tone` ↔ judge + emotion；morphology ↔ `memory_recall`；morphology ↔ companion-desktop。

---

## §8 主题 H —— 外部参考边界：170+ 项目清单（Round 10，原件 4 / 5 / 6）

> **考古要点**：同一份用户 170+ 清单有**两份互不相同的分类计数**——主代理初评（原件 4 §2.1/§2.4）与 sub-agent 真账（原件 5 §1）。**两版都保留，不做合并**；差异本身是这一轮的状态快照。

| 类别 | 主代理初评（原件 4 §2.1） | sub-agent 真账（原件 5 §1） | 主类 / 相关度（sub-agent） |
|---|---|---|---|
| AI Agent 框架与 Harness | ~37 | 40 | LLM harness / multi-agent；**HIGH**（15-20 直接可对齐，调研不重叠） |
| 开发工具与 CLI 增强 | ~38 | 48 | dev tools / OCR / memory / scraper；**MED** |
| 金融/量化/财务 | ~28 | 32 | finance / trading；**LOW**（远领域） |
| AI 伴侣 / VTuber / 桌宠 | ~22 | 27 | VTuber / desktop pet / Live2D；**HIGH**（5-8 直接可对齐，用户本位） |
| AI 语音/TTS | 6 | 6 | TTS / ASR / OCR；**HIGH**（3-4，与 RC-7 Perception 对接） |
| 安全与隐私 | ~14 | 15 | password / p2p / sandbox；**MED**（与 O-1 安全优先相关） |
| 资源汇总 | ~14 | 14 | awesome / wiki / skill；**LOW**（趋势合集） |
| 自建音乐流媒体 | 5 | 5 | music server；**NONE**（v2 是 Agent，不是 media server） |
| 即时通讯/机器人 | 2 | 2 | IM bot；**LOW**（v2 gateway 已有 SSE） |
| 地理信息 | 1 | 1 | GIS；**NONE**（单项目，完全无关） |
| **总计** | **170+** | **190（含 1 项重复计 ×2 + 资源类重复），净 170+** | HIGH ≈ 25 / MED ≈ 40 / LOW ≈ 90 / NONE ≈ 15（sub-agent 版）；主代理初评版 HIGH ~5 / MED ~51 / LOW ~44 / NONE ~69 |

- **评估维度（原件 4 §2.2，原表）**：**HIGH** = 直接可参考（代码 / 设计 / 思路）；**MED** = 同领域参考（了解趋势，不直接参考）；**LOW** = 远领域（知道就行）；**NONE** = 完全无关。Apeireth v2 路径 = AI Agent framework（17 crates, 9 organ, 12 slot, governance P0 hook 装）。**直接参考** = 代码可复用 / 参考边界 / 0 触碰 LOCKED；**趋势参考** = 设计思路；**不直接参考** = 太远或风险高。
- **sub-agent 0 装诚实标（原件 5）**：已读用户清单原文 284 行 + v2 handbook 250 行（完整 613 行），**未读** v1 legacy / research/source / `_research_mem`；时间紧真账 ≤250 行，**数字未实测，无 git/grep 命令**。
- **主代理亲验 catch（原件 6 §1.3，**关键**）**：sub-agent TOP 5 **全是 AI 伴侣类**；主代理预判 TOP 5 **全是 Agent 框架类**；**不矛盾，是两个维度**——Agent 框架类参考 = 12 slot cognitive module + 9 organ 抽象 + agent loop 模式（直接代码参考边界）；AI 伴侣类参考 = Live2D 视觉子模态 + 五维记忆系统 + ASR/TTS 抽象层（设计参考，**不直接参考 Live2D 渲染本身**）。**关键：Apeireth v2 是 AI Agent framework（后端 orchestrator），不是 AI 伴侣（前端 Live2D UI）。**
- **主代理 catch 的方法论失守登记（原件 6 §3，原表）**：**sub-agent TOP 5 评估方法偏差**——用「社区活跃度」（Star 数）评估 HIGH，而不是用「直接参考边界」（代码可复用性）；真账标 AI 伴侣类（5/5 TOP5），但 v2 是 Agent framework 不是 AI 伴侣；修法 = 主代理亲验 catch，修订参考边界。**sub-agent 0 实测**——未 git clone / grep 170 项目，仅基于用户清单 + v2 handbook 已知信息评估；真实施前主代理必亲验。**本轮无新失守**——O-6 三阶审查实跑、0 引新外部 dep、0 写真账以外的 file、5 重守门 baseline 维持。
- **修订后参考顺序（原件 6 §1.4）**：① **已参考（0 调研需要）** —— `research/source/` 里的 Agent 框架（~36 真开源）；② **P0 派单（1 周内）** —— sub-agent TOP 5（同类陪伴工程 5 项）真调研；③ **P1 排上（1 月内）** —— sub-agent P1 5 项 + 用户想法 #1。**关键修订：参考 AI 伴侣类不参考 Live2D 渲染本身**，只参考 Live2D 视觉子模态 + 五维记忆 + ASR/TTS 抽象层 + 多进程 WebSocket 架构 + 一键安装与极低开销。
- **决策建议摘要（原件 5 §6）**：**不要全吸收**——170 项目仅 25 HIGH，大部分 LOW/NONE 是 noise；优先派 sub-agent 调研 P0 TOP 5；用户 2 条想法随记都 P1/P2 排上，不阻塞 critical path；**吸收原则**（per O-2 前人肩上）= 必先语义对齐 v1 legacy，其次 research/source，最后本清单（顺序不重叠）。
- **两条「想法随记」评估（原件 4 §3，owner 决策位）**：① **「Apeireth 可以便携安装进 U 盘」**（用户 L277）→ **P1 排上（post-release），估时 1 周**——Cargo workspace portable binary（`cargo build --release --target x86_64-unknown-linux-musl` 或 windows-msvc）+ strip + UPX 压缩 + U 盘启动 script（Linux systemd / Windows service）+ portable data dir（`./data/` 而非 `/var/lib/`）+ README 与使用说明；0 触碰 LOCKED（新增 `scripts/portable.sh` + ``docs/portable-guide.md``）。② **「Apeireth 做自己刷视频的模块」**（用户 L278）→ **P2 后续，估时 2-4 周**——D 块 RC-7 Perception 真 modality 拓展（Video = 新 modality，已有 Voice + Vision）+ 视频 backend（YouTube/Bilibili API 集成 + TikTok/抖音 API 集成 + 本地视频文件 ffmpeg + frame extraction）+ 跟 R14 spec 的 modality 设计（`crates/foundation/plugin/src/perception_backend.rs`）+ 9 organ 流式集成（frontend video player 跟 cognitive module 流）；0 触碰 LOCKED。
- **Round 10 派单顺序与留 backlog（原件 4 §4/§6 + 原件 6 §2/§5）**：P0 = 派 5 sub-agent 真调研 TOP 5（各 1-2 天，写真账 ≤200 行到 ``docs/01-architecture/borrowed-from-{name}-2026-08-28.md``，必含实现摘要 / 参考点 / 0 触碰 LOCKED 验证 / 参考路径 / 下一步）+ 派 1 sub-agent 调研同类便携工程 portable 模式（sub-agent 推 P1，**主代理改 P0**）+ 参考五维记忆 → `cognitive.memory_recall` / `memory_writeback` 增维（跟 R20 并行）；P1 = 派 1 sub-agent 调研 P1 5 项（2-3 周）+ 对齐同类安全工程做法（Rust 密码库 + 敏感信息脱敏）→ O-1 安全优先 + U 盘便携；P2 = 视频模块（派 sub-agent 调研 R14 视频 modality，2-4 周）+ AI 伴侣 UI 参考 3 项 + 同类 IM 工程与本地 LLM 运行时（interface 扩展 + 第 4 家 provider）+ **金融/量化 + 音乐流媒体 + 地理信息（35 个）skip**。

---

## §9 主题 I —— legacy 九缺口考古（W3，2026-10-10，原件 19）

- **方法（原文）**：逐个读 v1 **真代码**（`legacy/donor/...`），**不用文件名下结论**——上一轮审计（`engineering-review-handoff-2026-10-06.md` §4.3）自称只做了名称级扫描并要求复核。每条主张带 `path:line`。九项清单依据 = 同交接包 §5 W3。**裁决约束：一次只做一个，禁止一次性全做。** 四级口径（[`docs/01-architecture/system-capabilities.md`](../../01-architecture/system-capabilities.md) 头部定义）标注在各 verdict 后。
- **一分钟结论表（原表，9 项）**：

| # | 缺口 | v1 真的做成了吗 | v2 现状 | 工作量 | 建议序 |
|---|---|---|---|---|---|
| 5 | `thought_cluster` 思维簇 | ✅ 真做成了，**且 v2 已实现**（= `cluster_store.rs` 改名，悬案闭合） | **库级缺口已闭合**（IMPLEMENTED）；欠生产接线 | S（核查接线即可） | **1** |
| 1 | `community` 社群识别与分诊 | ✅ 真做成了（图社群检测算法 + 确定性摘要 + 测试） | **✅ 已实现**（`memory/community.rs` 三件，9 测，台账 #57）+ **✅ 生产消费已接线**（all_facts 契约扩展 + 检索前置，台账 #64）——全闭环 | M | 2 〔四级口径：原篇表述，归属见本节现状复核〕 |
| 6 | `onering` 账本 | ✅ 真做成了（318 行 + 8 项测试） | **✅ 本体早已在 v2**（`context_ledger.rs` = companion ledger 打捞，API 同构；考古原判「仅注释提及」系误判——五审纠错）+ 本次补生产消费（cli 记账，台账 #58） | M | 3 |
| 2 | `experiment_field` 隔离实验场 | 🟡 部分：**机制真实施，执行后端 0 装** | **✅ 机制已实现**（runtime-assembly/experiment_field.rs：状态机 + VMRunner 口 + Noop 诚实 Err + 回滚学习 sink，5 测，台账 #59）；执行后端（真 VM）仍 0 装留后续 | M | 4 〔四级口径：原篇表述，归属见本节现状复核〕 |
| 7 | 真文件/网络沙箱 | ❌ **v1 也只有骨架**（**但见 §9.1 翻案**） | v2 进程树遏制**已反超 v1** | L（新造，走 W1 设计） | 5 |
| 9 | 三洋葱 L3-L5 | 🟡 部分：**层模型 + 判定逻辑真实施；无物理执行面** | v2 有数据模型 + hex 占位签名 | L | 6 |
| 3 | `HybridCognitiveRouter` | ❌ **不存在**（v1、v2 全部零命中） | 无 | XL（纯新造） | 8 |
| 4 | `ToolSynthesizer` | ❌ **不存在**（同上） | 无 | XL（纯新造） | 9 |
| 8 | SDK 真 HTTP·WS | （v1 有 `apeireth-api` 传输可参照） | **W5 已开挖** | L | 已开工 |

- **核心校准（原文）**：九项里**只有 2 项是真「实现」活**（community、onering）+ 1 项核查活（thought_cluster）；**2 项 v1 也没有**（HybridCognitiveRouter / ToolSynthesizer 是愿景新造）；沙箱 v1 同样是骨架——**把它当「实现」排期会严重低估**。
- **建议执行序（一次一个，原表）**：① §5 thought_cluster 接线核查（S 级收尾、零额外协调成本，可搭 W2 记忆闭环批次顺风车）→ ② §1 community 实现（v1 真货、边界清晰、落 `engine/memory` 图域，检索质量可感知）→ ③ §6 onering 实现（v1 真货 + 8 测试，建议与「会话转录重载 API」合并设计归属）→ ④ §2 experiment_field 机制实现（状态机/回路真货先落，与 §7 合并，与 `runtime-assembly/upgrade_cycle.rs` 衔接）→ ⑤ §7 真沙箱（=W1，**前置：主人对 shell-sandbox-lite 设计拍板**；九项中唯一有现成设计文档的）→ ⑥ §9 onion L3-L5 判定模型（✅ 已实现 `onion_gate.rs` 台账 #55；物理执行面装配 = 下一工作项；真签名留 v2.1）→ ⑦-⑨ §3/§4 愿景新造 + §8 SDK（XL 项先出设计再谈排期；SDK 已在 W5）。 〔四级口径：原篇表述，归属见本节现状复核〕
- **§7 沙箱的证据链（原文）**：v1 自己的注释——`legacy/donor/apeireth-tool-shell/src/lib.rs:25-28`（Linux seccomp BPF filter 需 `seccompiler` crate + unsafe pre_exec / Windows JobObject 需 `windows-sys` / Linux namespaces 需 `nix`）、`tool-shell:123`（TODO Linux seccomp / Windows JobObject / macOS sandbox_init）、`tool-shell:4`（保持 workspace ponytail ceiling，真 seccomp BPF / JobObject syscall filter 未做）、`sandbox_pass.rs:7-10`（`NetworkIsolation` trait + `NoopNetworkIsolation` netns/cgroup/WFP「接入后启用」、`VMSandbox` + `NoopVMSandbox` libkrun/Hyperlight/Firecracker「待选型」——**0 装守门文件的存在本身就证明执行面未落地**）。v2 正确路径 = [`docs/01-architecture/shell-sandbox-lite-design-2026-10-06.md`](../../01-architecture/shell-sandbox-lite-design-2026-10-06.md)（W1 轻量档 AppContainer + 工作区限定 + 断网），**动工前置 = 主人对设计拍板**。
- **§9 三洋葱 1.0 证据**：真实部分 = `lib.rs:21`（原则 5 层 E/S/A/M/O + 权限 6 层 L0-L5 + 11 节点电子环）、`lib.rs:66-79` `PermissionLayer` L0..L5 完整枚举（L3 关键操作 / L4 核心升级 / L5 核武器级）、`onion_demo.rs:65-72` `unify_check(&action)` 对 L3/L5 触碰给出判定——**判定模型是活的**；**但无物理执行面**（权限层是数据模型 + 检查函数，不接 OS 权限/进程边界，与 §7 同因）。v2 现状 = `crates/foundation/core/src/onion.rs` 有数据模型；M-of-N 多签是 **hex 占位签名**，真 Ed25519 标 v2.1。
- **本报告没有做的 4 件事（原文）**：① 未逐行验证 v1 各模块的**运行时行为**（结论基于静态代码阅读 + 测试存在性；`legacy/` 不参与构建，**没跑过它的测试**）；② 未评估实现的具体 API 形状；③ §3/§4 因 v1 无实现，**无从考证「该长什么样」**——立项前需要单独设计文档；④ **未替主人拍板顺序**（上表是**建议**；§5 W3 原裁决「一次一个」仍然有效）。
- **修正记录（2026-10-10 复盘更正，与 `handoff-w2-wiring-2026-10-06.md` §2.6 翻案表对账后）**：起因 = W2 线的翻案表与本报告冲突，**逐个验源码后确认对方正确**，原文不改、修正挂链。
  - **§9.1 §7 真文件/网络沙箱 —— 原判「❌ v1 也只有骨架」是过度概括，更正为 🟡 部分（v1 有整 crate 真实施）**：新证据 = `legacy/frozen/apeireth-sandbox/` 整 crate 存在且真接 Docker——`src/real.rs`（**1228 行**）`SandboxRealImpl` 真接 **Docker daemon HTTP API v1.43+**（`exec`=`POST /containers/create`+`/start`、`kill`=`POST /containers/{id}/kill`、`status`=`GET /containers/{id}/json`、`network`=`networks/create|delete|CONNECT|DISCONNECT`、`filesystem`=`containers/{id}/archive`+`volumes/create`、`resource_limit`=`POST /containers/{id}/update`（--cpus/--memory））；3 RuntimeKind（Container 真 / Process fallback / WASM 显式 0 装）；`DaemonClient` trait 三层（`HttpDaemonClient` 真 / `BollardDaemonClient` 留 R21+ / `MockDaemonClient` 测试）；测试 `tests/test_sandbox_real_wiremock.rs`（496 行）+ `examples/sandbox_real_demo.rs`（227 行）。**两说可并存（为什么我会错）**：原报告证据（`tool-shell` 的 TODO seccomp/JobObject、`sandbox_pass.rs`/`sandbox_net`/`vm_sandbox` 全 Noop）**仍全部成立**——它们是 companion 侧 trait 端口与 tool-shell，与 `frozen/apeireth-sandbox` 是**不同模块**；原判把「我没扫到」写成了「不存在」，**违反了本报告自己主张的三关判定法：归档区（frozen/archived）也要扫**。**对排期的影响**：工作量从「L 纯新造」修正为 **M–L 可回收实现**；与 W1 的关系 = **互补而非替代**（W1 shell-sandbox-lite 是 Windows 优先路线，Docker daemon 方案覆盖 Linux/CI 场景），开工仍以 W1 设计文档为准绳。
  - **§9.2 §1 community —— 「分诊在 v1 是 brief 级」低估，更正为真分诊函数**：新证据 = `legacy/donor/apeireth-companion/src/community.rs:155` `pub fn triage(...)` 真分诊入口存在，且有专项测试 `triage_entity_vs_broad`（`:323`）；v1 的社群识别 → 分诊是**真功能**（`detect_communities` + `triage` + `deterministic_summary` 三件套）；工作量 **M 不变**，但实现范围应含 triage 分诊逻辑。

---

## §10 残清单与 owner 决策位汇总

> 下列是**当时**的挂账与待拍板位，**不是**现行 backlog。保留以便追溯「哪一步留了尾巴、尾巴后来由谁接」。

- **§2.4 的 7 项待拍板决策**（A4 trait 形态 / A3 perception 深度 / A1 council LLM 隔离 / A2 team-lead worktree / A6 frequency limit 粒度 / A1 council 决策态 / A4 File 加密）——见 §2.4 表，**推荐项已给，未见落地记录**。
- **§1.5 的主代理亲做 10 项 spec**（rot_score 融合 / consolidation+reflection pipeline trait / hello.rs 主题确认 / education 真 CAS / confidence BetaBinomial / reflexion 3 trait 口 / brief 模板 / 27 项 1.0 .rs 实测 / ROADMAP+MANIFESTO 修订 ✅ / §3.1 估时修订 ✅）——前两项与后两项以外当时仍挂账。
- **§1.3 的 2 处真错账**（R9 spec L258 / L447 HEAD 漂移）——原文标注「主代理可改可保留」。
- **§1.2 的风险总账 5 条**（B 块估时溢出 / C 块 6 DEFERRED slot 估时溢出 / D 块 RC-7 硬件硬阻塞 / sub-agent O-6 失守 / 网络阻塞 push）。
- **§2.5 的 12 条显式降级 / 漂移**——逐条列出，其中①②⑤⑥⑦⑪⑫ 挂账中，③④⑧⑨⑩ 生效中。**⑦「生产旋钮整体默认关」是本册多处「🟡 接线在、默认不跑」的同一策略来源。**
- **§2.5 结论② 的 10 个最重差距**与 **结论③ 的 A/B 两张清单**（A = 该做的缺口 6 项 / B = 建议正式修订标尺 8 项）。
- **§5 的 9 项协调 + 上下文 P0-P3**（含 rot_score 融合前置、hello 主题确认）。
- **§4 的 `cross_diary` 被 storage graph 卡住**（跨文档依赖：§4 ← §3）。
- **§9 的 W3 裁决约束「一次只做一个，禁止一次性全做」**与 **未替主人拍板顺序**；§7 沙箱的**动工前置 = 主人对 W1 设计拍板**。
- **§8 的两条想法随记**（U 盘便携 P1 / 刷视频模块 P2）与**金融/量化 + 音乐流媒体 + 地理信息 35 个 skip**。

---

## §11 现状对照（2026-10-23）

> 本节**只指向现行真源**，不在历史叙述里改写数字。历史章节里的 crate 数、测试数、HEAD、键数、workspace.version、release 估时一律是**当时口径**。

| 主题 | 现行真源（唯一出处） | 说明 |
|---|---|---|
| 测试数字 | [`../../03-reference/test-baseline.md`](../../03-reference/test-baseline.md) | 全仓测试数字**单一真源**；本册中的 1739 / 3120 / 3662 / 1726 等均为历史口径 |
| 能力状态声明 | [`../../status/claims.yaml`](../../status/claims.yaml) | 口径阶梯 NOT IMPLEMENTED < IMPLEMENTED < PRODUCTION WIRED < HARDWARE VALIDATED；`DEFAULT ENABLED` 是横切属性 |
| **v1↔v2 能力差距的现行判定** | [`../v1-vs-v2-capability-gap-audit-2026-10-06.md`](../v1-vs-v2-capability-gap-audit-2026-10-06.md) | **2026-10-06 复核审计**：§7 两级化 / §8 附录 A 是本册 §2.1 旧 🔴/🟢 结论的**现行判定依据**（原件 7 已被降级为历史记录） |
| 现行待办 | [`../backend-enhancement/00-backlog.md`](../backend-enhancement/00-backlog.md) | 现行想法级索引；本册 §10 的历史残余清单**不**等于现行 backlog |
| 吸收台账（现行） | [`../absorption-ledger.md`](../03-audits/absorption-ledger.md)、[`../absorption-candidates-2026-10.md`](../03-audits/absorption-candidates-2026-10.md) | 保留原位未合并；本册 §8 的 170+ 清单调研是它们的前史 |
| 工程总纲 / 施工纪律 | [`../ENGINEER-MANIFESTO.md`](../ENGINEER-MANIFESTO.md)、[`../backend-enhancement/CONSTRUCTION-DISCIPLINE.md`](../backend-enhancement/CONSTRUCTION-DISCIPLINE.md) | 改 src / 改 doc / 派子代理前必读 |
| 仓库级路线 | [`../../../ROADMAP.md`](../../../ROADMAP.md) | §3「当前状态」是本册 release 估时的现行索引 |
| 默认开关实况 | [`../../03-reference/capabilities-matrix.md`](../../03-reference/capabilities-matrix.md) §0.2 | 与 `crates/engine/runtime-assembly/src/canonical/production.rs` 实况对应；对应本册 §2.5 漂移条目⑦ |
| 键集口径 | [`../../adr/0006-formalize-14-keys.md`](../../adr/0006-formalize-14-keys.md) | 本册多处「13 键」为 2026-10-11 PHL-08 之前的历史口径；现行为 **14 键（PHL-01..08）** |
| 版本 | 根 [`Cargo.toml`](../../../Cargo.toml) `[workspace.package] version` | 本册出现的 `2.0.0-rc.1` 为历史口径 |

**当前口径下的总体判断（依据上表真源，不改写历史）**：本册记录的是 **2026-08-27 → 2026-10-10 之间的调研与审计过程**；其「~25 项 P0 必补」「12-14 周 critical path」「release 2027-Q3」「~35 项 1.0 缺口」等结论均为**当时估时**，后续由 2026-10-06 复核审计与现行 backlog 重新裁定。本册不复述其现行结论。

---

_本册为**只读归拢**产物：只新建本文件，未修改任何被合并的原件，未触碰 ``docs/architecture/*``、``docs/status/*``、``docs/03-reference/*``、``docs/adr/*``、``docs/archive/**``，未执行任何 git 写操作。_
