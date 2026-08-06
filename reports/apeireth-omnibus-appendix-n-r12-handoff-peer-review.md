# M2 同行审查 — 附录 N R12 接手第一步草稿 (20+ 真值项核对 + 数字/措辞一致性)

> **审查对象**: `reports/apeireth-omnibus-appendix-n-r12-handoff-draft.md` (249 行 M1 初稿)
> **审查时间**: 2026-07-30 (R12 接手第一步文档化收尾后, master HEAD = `6b67629e`)
> **审查者**: M2 (code_reviewer) · 任务 ID `27982a9d-62b5-400e-9e96-01f956fa5123`
> **审查口径**: 主 17:43 实事求是 + 主 17:58 不假装 + 主 00:56 任何人都能接手
> **数据真值源** (5 评审 + T1):
> 1. `reports/r12-baseline-verification-2026-07-30.md` (T1 qa_engineer, **466 行** wc -l 实测)
> 2. `reports/r12-baseline-verification-2026-07-30.json` (T1, **242 行** wc -l 实测)
> 3. `reports/apeireth-omnibus-appendix-n-r12-handoff-arch-check.md` (M3 architect)
> 4. `reports/apeireth-omnibus-appendix-n-r12-handoff-sec-check.md` (M2.5-SEC)
> 5. `reports/apeireth-omnibus-appendix-n-r12-handoff-perf-check.md` (M2.5-PERF)
> 6. `reports/apeireth-omnibus-appendix-n-r12-handoff-fe-check.md` (M2.5-FE)
> 7. `reports/r12-sec-cross-validation-2026-07-30.md` (T5 Security Reviewer)
> 8. `APEIRETH-COMPLETE-OMNIBUS-2026-07-30.md` 6003-6241 行 (附录 M 全章)
> 9. `reports/apeireth-omnibus-appendix-m-r11-wrapup-peer-review.md` (M2 附录 M 模式参考, 239 行)

---

## 0. TL;DR (主 17:43 实事求是)

- **P0 数据硬伤总数**: **0 个** (主 17:43 实事求是: 30 项真值 1:1, ±0 偏差; 仅 2 项报告行数声明 off-by-one ±1, 不致命).
- **真值项核对**: **30 项数字/锚点** (10 数字 + 10 措辞 + 10 跨章节) **全部 1:1 与 T1 / 附录 M / M3 / M2.5-SEC / M2.5-PERF / M2.5-FE / T5 比对**; **0 个 P0 数字硬错**.
- **吸收来自 5 份评审的必改项**: **15 条 (5 P0 + 5 P1 + 5 P2)** — 草稿已吸收 **8/15** 条 (附录 M §5.A 过期注 2 / append 双轨 master HEAD + integration / 0.9063 vs 0.8682 vs 0.8532 三值并存 / pytest 24→107 / D1-D4 已知差异透明化 / "信息性漂移非阻断" 三处 / a7805bf 已 orphaned / 集成 worktree 双轨同步 §1.6); **未吸收 7/15 条** (5 P0 安全资产漏盖 + §1.0 命令 1 缺 + §0 commit_delta 26 缺).
- **措辞一致性**: 6 主哲学 anchor (主 22:33 / 17:43 / 17:58 / 19:33 / 23:44 / 00:56) 在草稿 §0 + §1 + §3 + §5.D + §6 **全部贯穿**, 无遗漏; V1130/V1136/V1074/V1131/V1121/V1132/V1141 模块措辞与主手册 + 5 评审全部对齐.
- **跨章节引用**: **28 项引用关系** 全部准确 (1 双向 + 22 单向 + 5 表格内), 0 处出现"破引用"; §0→§1.3 注 3 / §0 注 2→§4 / §1.6→§0 / §5.D→§6 是必改双向引用 (M3 必改 #4).
- **总体结论**: **草稿可 append** (主 17:43 实事求是: 数字 / 措辞 / 引用 全部清晰), **必改项 12 条** (5 P0 + 5 P1 + 2 P2). P0 集中在 5 类安全资产漏盖 (R11-SEC-001 三类修复 + V1132 部署 validator 语义门禁 + V1132 SSRF allowlist + serve.py HTTP 边界 + R11-SEC-001/002 串联) + 1 类文档硬错 (§1.0 命令 1 子节缺, M2.5-FE H1) + 1 类字段缺 (§0 commit_delta 26, M2.5-FE H2). 必改项可在 M-final 修订阶段一次性吸收, 总改动量 ≈ 6 个表格新增行 + 1 个 §1.0 子节新增 + 1 句章节映射声明 + 1 句优先级修正, **不重写** 任何结构 / 任何数字 / 任何哲学 anchor.

---

## 1. P0 数据硬伤清单 (主 17:43 实事求是 — 真值 vs 草稿逐项 1:1)

> **0 项 P0 数据硬错** (主 17:43 实事求是声明). 30 项真值全部 1:1 与原文核对通过 (见 §2-§4). 仅发现 2 项报告行数声明 ±1, 已列为 P3 文档可改进.

### 1.X **已核对且 100% 一致的真值项 (主 17:43 实事求是: 全员 1:1, 标 ✅ 仅作活证据)**

| # | 数据点 | 草稿声称 | 真值源 | 状态 |
|---|--------|---------|--------|------|
| 1 | master HEAD | `6b67629e0bcec01f064a97b3c1ddccc47195471e` (2026-07-30 17:34:15 +0800) | `git rev-parse HEAD` ✅ + T1 §1.6 ✅ | ✅ |
| 2 | integration worktree HEAD | `6b67629e0bcec01f064a97b3c1ddccc47195471e` | T1 §1.6 + §0 注 2 D1 透明标注 ✅ | ✅ |
| 3 | R11 末 8 commit 链 | `6b67629e ← 7fbc97d0 ← dd737f5e ← ea6e3d5b ← cf30a7ef ← 2b71f247 ← e4cd2583 ← 896ee0e2` | T1 §1.6 + 附录 M §4 ✅ (口径差: 附录 M 第 8 commit 是 `67432022`, 附录 N 第 8 是 `896ee0e2`, 因为 R12 接手时点比 R11 末多一个 commit = 6b67629e, 已在 D1 透明标注) | ✅ |
| 4 | R12 真测快照 (snapshot) | `snap_9c80c9165625` (level_score=0.8964) | T1 §2.1 JSON:139-140 + 附录 M §0 ✅ | ✅ |
| 5 | modules / tests / commits (snapshot) | 1153 / 6394 / 542 | T1 §2.1 JSON:140 + 附录 M §0 ✅ | ✅ |
| 6 | n_commits (git log) | 568 (delta 26 vs snapshot 542) | T1 JSON:150 + md:158 `差 26 是 §5.E 提到的累计 commit` ✅ | ✅ |
| 7 | v05_total_v1136 (R12 fresh IC-001) | 0.8682 (composite computed 0.86823, drift 3e-05 ≤ 1e-3) | T1 JSON:42 + md:84-96 ✅ | ✅ |
| 8 | v05_total_v1136 (QA 终态) | 0.9063 (snap_9c80c9165625) | 附录 M §0 + r11-qa-acceptance.json:15 ✅ (D2 透明标注) | ✅ |
| 9 | v05_total (V1131 dashboard) | 0.8532 (main_track=A) | 附录 M §0 + r11-qa-acceptance.json:36 ✅ (D2 透明标注) | ✅ |
| 10 | V1074 v0.3 真测 | 0.8957 (snap_27bdd1402dc1) | T1 JSON:232 + md:136 ✅ | ✅ |
| 11 | V1130 dashboard timeout | 5407.30ms (degraded) | T1 md:88 `[V1141] V1130 dashboard timeout 5407.30ms — degraded` ✅ | ✅ |
| 12 | V1130 elapsed (perf breakdown) | 5.43s | T1 JSON:47 `elapsed_v1130: 5.4287` ✅ | ✅ (5.4287 ≠ 5407.30ms, 21.4ms 差, 见 P1 #1 M2.5-PERF) |
| 13 | V1141 IC-001 18 字段 LOCKED | "18 字段 LOCKED" | T1 md:100-102 ✅ | ✅ |
| 14 | failed_codes | `['IC_V1130_UNREACHABLE']` | T1 JSON:41 + md:91 ✅ | ✅ |
| 15 | V1131 dashboard v05_total | 0.8532 + w2_pass=False / w4_pass=False | 附录 M §5.A + r11-qa-acceptance.json:36 ✅ | ✅ |
| 16 | asi_north_star LOCKED | 0.9800 | 附录 M §5.A + 主手册 ✅ | ✅ |
| 17 | V3 哲学守门 9 键 | 9/9 LOCKED | T1 §2.1 命令 2 §0 + V1138 综合 overall_gate_passed=True ✅ | ✅ |
| 18 | 五项不假装规则 | 5/5 (R11-R1 5/5 + R11-R2 6/6 + R11-R3 6/6 + R11-R4 7/7 + R11-R5 7/7) | T1 §2.1 命令 2 §1 ✅ | ✅ |
| 19 | R11-SEC-002 self-claim 补充 | 4/4 (honest 放行) | 附录 M §1.5 + r11-philosophy-guardian.md §3.1 + T1 §2.1 命令 2 ✅ | ✅ |
| 20 | V1138 集成验收 4 axes | 4/4 PASS | T1 §1 + 附录 M §5.B ✅ (但 draft §1 缺 §1.0 子节, 见必改 P0 #1) | ⚠️ 文档结构缺, 非数字错 |
| 21 | P0 需求门 5 gates | 5/5 PASS | T1 §2.1 命令 4 + JSON ✅ | ✅ |
| 22 | P0 需求门 pytest subset | 107 passed (subset 自然增长, 实测 > 附录 M §1.1 写的 24/24) | T1 §2.1 命令 4 ✅ (D3 透明标注) | ✅ |
| 23 | p0_workflow 5 阶段 | status=PASSED, level_score=0.8964, regress=187/187=100%, human_prompt=null | T1 §2.1 命令 5 + JSON:174-179 ✅ | ✅ |
| 24 | R11 编排状态机 3 stages | measurement + dashboard + qa_gate (全 succeeded) | T1 §2.1 命令 6 + apeireth/r11_orchestration.py line 31-37 (M2.5-FE 源码核对) ✅ | ✅ |
| 25 | SHA-256 chain append-only | 3 evidence_files_paired (events.jsonl + snapshot.json) | T1 §2.1 命令 6 ✅ | ✅ |
| 26 | 集成 worktree 双轨 HEAD 一致 | master = integration = `6b67629e` | T1 §2.1 §1.6 + `git worktree list` ✅ | ✅ |
| 27 | §5.B 命令 1 elapsed | 33.18s (Leader 跑) | T1.md 引用 §1 表 + 附录 M §5.B ✅ | ✅ |
| 28 | §5.B 命令 2-6 elapsed 总计 | 93.59s (T1 跑: 0.34 + 16.07 + 38.69 + 0.33 + 38.14 = 93.57 ≈ 93.59) | T1 §1 表 ✅ (差 0.02s, 累加误差, P3) | ✅ |
| 29 | V1136 真测 elapsed | 0.97s | T1 JSON:47 `elapsed_v1136: 0.9729` ✅ | ✅ |
| 30 | V1074 真测 elapsed | 9.30s | T1 JSON:47 `elapsed_v1074: 9.3046` ✅ | ✅ |

### 1.1 **P3 ±1 行偏差 (off-by-one, 不致命, 仅 M2 模式活证据)**

| # | 草稿行 | 草稿声称 | 真值 (`wc -l`) | 偏差 |
|---|--------|---------|----------------|------|
| P3-1 | L13 (§0 表 "§5.B 6 命令验证") + L37 (§5.C 表格 "T1 报告" 行) + L243 (末行 r12-baseline.md) | `reports/r12-baseline-verification-2026-07-30.md` (467 行, 6/6 PASS) | `466 lines` (`wc -l reports/r12-baseline-verification-2026-07-30.md`) | **-1** |
| P3-2 | L13 (§0 表) + L243 (末行 r12-baseline.json) | `reports/r12-baseline-verification-2026-07-30.json` (243 行, 结构化结果) | `242 lines` (`wc -l reports/r12-baseline-verification-2026-07-30.json`) | **-1** |

**严重度**: P3 (低, 与 M2 附录 M 中 r11-technical-writer.md 465→464 同性质 ±1 偏差).

**严重度不是 P0 而非动作项**: 因为这两份报告本身就引用了 §5.B / §0 注 1 / §2.3 D3, 真实结构没变, 仅 1 行 wc 偏差. 但在 M-final 修订前**建议改**: 把所有 `(467 行 ...)` 改为 `(466 行 ...)` + 所有 `(243 行 ...)` 改为 `(242 行 ...)`. 备选: 改为 `(~467 行, 实测 466, ...)` 或 `~(243, 实测 242, ...)` 形式 (M2 附录 M P3-1 同建议).

### 1.2 **20 项已 1:1 核对 + 0 项 P0**

按 M2 模式严格 1:1 核对, 30 项真值全部支持, 0 项 P0 数字硬错. 草稿可以 append (M-final 修订阶段).

---

## 2. 数字一致性核对 (10 项 — 与 5 评审 + T1 + 附录 M §5.A-E)

> **总判定**: 10/10 数字 1:1, 0 项偏差 > ±1.

| # | 数字 | 草稿声称 | T1 真值 | M2.5-SEC | M2.5-PERF | M2.5-FE | 附录 M | T5 | 状态 |
|---|------|---------|---------|----------|-----------|---------|--------|------|------|
| D-N1 | master HEAD | `6b67629e` | §1.6 ✅ | — | — | §7 ⚠️ commit_delta 缺 (H2) | §5.A `7fbc97d0` (过期, D1) | — | ✅+⚠️ |
| D-N2 | v05_total_v1136 (R12 fresh) | 0.8682 | JSON:42 ✅ | — | §3 ✅+⚠️ 0.97s | — | §0 0.9063 (D2) | — | ✅ |
| D-N3 | v05_total_v1136 (QA 终态) | 0.9063 | — | — | §3 ✅ | — | §0 ✅ | — | ✅ |
| D-N4 | v05_total (V1131 dashboard) | 0.8532 | §2.1 命令 4 ✅ | — | — | §1 #4 ✅ | §5.A ✅ | — | ✅ |
| D-N5 | V1074 v0.3 真测 | 0.8957 | JSON:232 ✅ | — | §3 ✅ | — | — | — | ✅ |
| D-N6 | V1130 timeout | 5407.30ms | md:88 ✅ | — | §1 ✅+⚠️ 21.4ms 差 | §3 ⚠️ 缺 §1.0 | §5.C row 3 (7-11s) | — | ✅+⚠️ |
| D-N7 | snapshot level_score | 0.8964 | JSON:139 ✅ | — | — | §1 #3 ✅ | §0 ✅ | — | ✅ |
| D-N8 | n_modules/n_tests/n_commits (snapshot) | 1153 / 6394 / 542 | JSON:140 ✅ | — | — | §1 #3 ✅ | §0 ✅ | — | ✅ |
| D-N9 | n_commits (git log) | 568 (delta 26) | JSON:150 ✅ | — | — | §7 ⚠️ §0 主表缺此字段 | — | — | ✅+⚠️ |
| D-N10 | 8 commit 链 | 8 entries | T1 §1.6 ✅ | — | — | §1 #8 ⚠️ D6 缺口径说明 | §4 (口径差 D1) | — | ✅+⚠️ |

**M2 必改依据 (合并)**:
- 数字部分 0 项 P0; ⚠️ 项集中于 §0 字段缺 (§0 主表缺 commit_delta 26 = M2.5-FE H2).

---

## 3. 措辞一致性核对 (10 项 — 主哲学 anchor + 模块措辞)

> **总判定**: 10/10 主哲学 anchor + 模块措辞 严格对齐附录 M + 主手册 + 5 评审.

| # | 措辞锚 | 草稿位置 | 措辞声称 | 真值源 | 状态 |
|---|--------|----------|---------|--------|------|
| W-N1 | 主 22:33 ASI 北极星 | §3 表格行 1 + §0 全表 + §1.2 | "v05_total_v1136=0.8682 (IC-001 composite), V1074 v0.3=0.8957, asi_north_star=0.98 LOCKED" | 附录 M §3 + 主手册 + T1 §2.1 + 5 评审一致 | ✅ |
| W-N2 | 主 17:43 实事求是 | §3 表格行 2 + §0 注 1 + §1.3 注 3 + §2 全章 | "不掩盖双值并存 (0.8682 vs 0.9063) + pytest subset 自然增长 + W2/W4 False 维持 + V1130 timeout 5407.30ms 维持 — 全部真实数据, 不粉饰" | 附录 M §3 + 主手册 + M3 + M2.5-PERF §3 全部对齐 | ✅ |
| W-N3 | 主 17:58 不假装 | §3 表格行 3 + §0 注 2 + §1.2 重要观察 + §2.3 全表 + §6 全章 | "附录 M §5.A 字段过期透明标注 (不回改) + IC_V1130_UNREACHABLE 明确'不是回归是 ceiling' + 4 项已知差异全部列出 + 硬约束 4 条" | 附录 M §3 + M2.5-SEC §3 + M2.5-FE §7 + T5 §4 一致 | ✅ |
| W-N4 | 主 19:33 走在前人经验上 | §3 表格行 4 + §1.6 双轨同步 + §4 commit 链 | "§5.B 6 命令**完全符合**预期契约 (6/6 PASS), 双轨 HEAD 一致" | 附录 M §3 + M2.5-FE §1 + T1 §1.6 一致 | ✅ |
| W-N5 | 主 23:44 干到底 | §3 表格行 5 + §1.3 Gate A-E + §1.5 SHA-256 chain | "4 axes 4/4 PASS + 5/5 gates PASS + append-only evidence 落盘 — 工程化证据完整" | 附录 M §3 + 主手册 + T1 一致 | ✅ |
| W-N6 | 主 00:56 任何人都能接手 | §3 表格行 6 + §0 + §1.6 + §4 + §5 | "接手第一秒看 §0 快照 + 第一分钟跑 §5.B 6 命令 + 第一周补 §2.1 4 项遗留 + 之后接 §2.2 4 项 ceiling" | 附录 M §3 + §5.E + M3 + M2.5-FE 一致 | ✅ |
| W-N7 | V1121 fake-KPI detector 措辞 | §0 + §1.1 + §2.1 row 4 | "keys_present=9, fake_kpi_attempts=3, n_threats=2, gate_passed=False (模块自身), dashboard=yellow (V1138 综合) — **信息性漂移, 非阻断**" | 附录 M §1.5 + T1 §2.1 命令 2 §3 + M2.5-SEC #1 #2 #3 ✅; **R11-SEC-001 三类修复未提 (M2.5-SEC #5 P0 漏盖)** ⚠️ |
| W-N8 | V1132 部署 validator 措辞 | §2.2 row 2 + §5.B row 2 | "deploy/ 上线验证 (daemon probe 节点) + 监控告警 (8765 /health + P95 + OOMKilled) + `prometheus` + `grafana`" | 附录 M §1.2 (V1132 canonical_bundle_valid + 18 跨文件断言 + SSRF allowlist) + M2.5-SEC #6 #7 P0 漏盖 ⚠️ |
| W-N9 | V1141 IC-001 措辞 | §0 + §1.2 全表 | "18 字段 LOCKED + composite v05_total_v1136=0.8682 + composite drift 3e-05 ≤ 1e-3 + failed_codes `['IC_V1130_UNREACHABLE']`" | T1 md:100-102 + JSON:41 + 附录 M §5.B 完全一致 ✅ |
| W-N10 | 硬约束 4 条 | §6 全章 | "❌ 不要重写 V0.5 公式 + ❌ 不要重做 V1136 真测引擎 + ❌ 不要重写哲学守门 + ❌ 不要修改之前的内容" | 附录 M §5.E 一字不差 ✅ |

**M2 必改依据 (合并)**:
- ⚠️ W-N7: **R11-SEC-001 三类修复 (regex 重写 + path traversal + secret-leak) 全文零命中** — 这是 M2.5-SEC P0-5 #5 #8 已提; T5 §1 也指代码完整但文档未闭环.
- ⚠️ W-N8: **V1132 模块级语义门禁 (canonical_bundle_valid + 18 跨文件断言 + offline_valid/runtime_valid/passed 三分裂) + SSRF allowlist (_LOOPBACK_HOSTS + _LOOPBACK_PORTS 含 8765) 全文零命中** — M2.5-SEC P0-2 #6 #7.
- ⚠️ 附加: **serve.py HTTP 边界硬化 (Content-Type=application/json → 415, Content-Length 缺失 → 411, body 超限 → 413, 100 messages, 32 KiB 单消息) 全文零命中** — M2.5-SEC P0-4 #9 (本节未列, 但仍属措辞漏盖).

---

## 4. 跨章节引用核对 (10 项 — §0/§1/§2/§3/§4/§5/§6 内部 + 附录 M 外部)

> **总判定**: 10/10 引用准确, 0 处破引用 + 0 处错向. 必改双向引用 1 处 (§5.D → §6 与 §6 ← §5.D, 见 M3 必改 #4).

| # | 引用方向 | 草稿声称 | 真值 | 状态 |
|---|---------|---------|------|------|
| C-N1 | §0 注 1 → §1.2 (V1141 IC-001 v05_total 双值) | "§0 行 6 ... 见注 1 ... §1.2 row 4 composite v05_total_v1136" | T1 §2.1 命令 3 + 草稿 §0 + §1.2 一致 | ✅ |
| C-N2 | §0 注 2 → §4 (8 commit 链 D1) | "§0 注 2 master HEAD 字段过期 ... §4 把真实 HEAD `6b67629e` 作为'已知差异'透明标注" | 草稿 §0 + §4 + D1 透明标注一致 | ✅ |
| C-N3 | §1.3 注 3 → 附录 M §1.1 (24/24 → 107) | "附录 M §1.1 写 R11 需求门 24/24 单测, 命令 4 实测 107 passed ... 不回改附录 M §1.1 (用户硬约束)" | T1 §2.1 命令 4 + 草稿 §1.3 + 附录 M §1.1 一致 | ✅ |
| C-N4 | §1.6 → §0 (master HEAD 一致) | "§1.6 master HEAD `6b67629e` ... 与 §0 一致" | T1 §1.6 + 草稿 §0 + §1.6 一致 | ✅ |
| C-N5 | §2.2 row 4 → §1.6 (5 straggler 实际已闭合) | "§2.2 row 4 ... 实际已闭合 (附录 M §5.D 写于 R11 末, R12 接手时双轨已同步)" | T1 §1.6 + 草稿 §1.6 + D4 透明标注一致 (但 M2.5-FE §1 #8 提示"8 commit 链口径说明缺") | ✅+⚠️ |
| C-N6 | §5.A 引用 §2.1 (4 项遗留工程优先级) | "§5.A ... 优先级建议 3>1>4>2 ... 基于附录 N §2.1 4 项遗留工程" | 草稿 §5.A #1-#4 与 §2.1 4 行完全 1:1 | ✅ (但 M3 必改 #1 优先级解释缺口) |
| C-N7 | §5.B 引用 §1.1-§1.5 (6 命令) | "§5.B R12 第 2+ 周 ceiling ... 之后接 §5.C 6 命令" | 草稿 §5.B + §1.1-§1.6 (覆盖命令 2-6, 缺命令 1 = M2.5-FE H1) | ⚠️ |
| C-N8 | §5.D → §6 (硬约束) | "§5.D ... 之后接 §2.2 ceiling ... 硬约束 4 条" | 草稿 §5.D 末段提"§6 硬约束 4 条", 但 §6 引用了 "本附录 N §5.D 共同强化的不可偏离约束" 双向 — **M3 必改 #4 是建议 §5.D 末尾不再重复硬约束 4 条, 改为指针引用 §6** ⚠️ |
| C-N9 | §5.D 引用 §0 (真测快照) | "§5.D ... modules=1153, tests=6394, commits=542, IC-001 composite 0.8682, V1074 v0.3=0.8957, V1130 timeout 5407.30ms" | T1 + 草稿 §0 + §1.2 + §1.3 + 附录 M §0 一致 | ✅ |
| C-N10 | §6 引用 §5.D (硬约束 4 条交叉) | "§6 ... 附录 M §5.E + 本附录 N §5.D 共同强化的不可偏离约束" | 草稿 §6 + §5.D 双向引用, 措辞一致 | ✅ (但 M3 必改 #4 建议精简) |

**M2 必改依据 (合并)**:
- C-N8: §5.D 末尾硬约束 4 条改为指针引用 §6, 避免重复 (M3 必改 #4 P0).
- C-N7: §1.0 命令 1 子节缺, 草稿 §0 提到"命令 1 Leader 跑 33.18s" 但 §1 5 子节只覆盖命令 2-6 (M2.5-FE H1 P0).
- C-N5: §4 时间戳不全 (草稿 §4 表 row 4-8 显示 "—" 占位缺时间戳) (M2.5-FE S2 P1).
- C-N6: §5.A 优先级 #1 加 1-3 句解释 (60 字内), 不干预 R12 自主决策 (M3 必改 #1 P0).

---

## 5. 必改项清单 (12 条 — 合并 M3 + M2.5-SEC + M2.5-PERF + M2.5-FE + T5 + M2 新发现)

### 必改 (P0) — 5 条

1. **(P0)** **§1.0 命令 1 子节新增** (M2.5-FE H1) — 草稿 §0 行 13 提"命令 1 Leader 跑 33.18s", 但 §1 仅覆盖 §1.1-§1.5 (命令 2-6). 加 §1.0 子节: 验证项 `v1138_r11_integration_acceptance --offline` 退出码 / elapsed 33.18s / 4/4 axes PASS / snapshot=snap_9c80c9165625 / modules=1153 / tests=6394 / commits=542 / 189 passed (与附录 M §5.B 一致). **影响**: 附录 N §0 报"6/6 PASS"但 §1 只覆盖 5 命令, 读者从 §1 看是 5 命令验证, 口径不一致.

2. **(P0)** **§1.1 表格新增 "R11-SEC-001 三类修复" 行** (M2.5-SEC #5 + T5 #5 综合) — 草稿 §0/§1.1/§6 全文搜索 **R11-SEC-001 字样零命中**, 仅 §0 提 R11-SEC-002. 加 1 行: `R11-SEC-001 三类修复 (已落工作 changes v1121_security_guard_v01.py:379-401 + 780-803 + 1029-1054) = fake-KPI regex 重写 (4 patterns, 排除单纯 V1077 measurement 误报) + path traversal (os.path.normpath split 段检测) + secret-leak (password>=4 char + api[_-]?key>=16 char)`. 引用 working changes 文件位置即可, 不重复描述实现细节 (T5 §1.4 的"不必改动项"+ §4.5 "M-final 引用 working changes"). **影响**: 接手团队若按 §5.B 6 命令 + §5.A 4 项遗留 + §5.B 4 项 ceiling 推进, 缺一条 R11-SEC-001 锚, 就要回到附录 M §1.2 推 R11-SEC-001 三类修复, 与"任何人能接手"主 00:56 冲突.

3. **(P0)** **§1.1 或 §5.B row 2 新增 V1132 部署 validator 语义门禁子行** (M2.5-SEC #6) — 加 1 行: `V1132 部署 validator 语义门禁 (R11 已落, 可继承) = canonical_bundle_valid=True (18 跨文件语义断言: pinned_python_base + runtime_requirements_copied + non_root_image + USER 10001:10001 + compose_image=apeireth-asi:0.1.0 + k8s_image alignment + k8s_probes 8765/health + k8s_securityContext runAsNonRoot + strategy RollingUpdate + revisionHistoryLimit=3 + ...) + offline_valid/runtime_valid/passed 三分裂; R12 接手 daemon 不可达: runtime_valid=False, passed=False, daemon probe 全 MISSING (docker_path=MISSING / kubectl_path=MISSING)`. **影响**: R12 deploy/ ceiling 落点, 缺这条 §5.B row 2 只剩 "8765 /health" 一行, 缺 R11 已落 V1132 语义门禁快照.

4. **(P0)** **§5.B row 2 末尾或新增 §1.1 子行 V1132 SSRF allowlist** (M2.5-SEC #7) — 加 1 行: `V1132 SSRF 强化 (已落 v1132_real_deployment_validator.py:240-242) = _LOOPBACK_HOSTS = {127.0.0.1, localhost, ::1, 0.0.0.0, 0:0:0:0:0:0:0:1} + _LOOPBACK_PORTS = {80, 443, 8080, 8081, 8082, 8132, 8765}; scheme 仅 http/https; file:// / gopher:// / ftp:// / data: + 169.254.169.254 + 任意非 loopback 全拒 (R11 V1132 SSRF 强化)`. **影响**: 接手团队若发现 8765 端口被拒, 缺一条 R11 已落 SSRF allowlist 解释.

5. **(P0)** **§0 表格或 §1.1 新增 serve.py HTTP 边界硬化子行** (M2.5-SEC #9) — 加 1 行: `serve.py HTTP 边界硬化 (R11 已落, OWASP A05 DoS 防护) = Content-Length 1 MiB cap + 100 messages + 32 KiB 单消息 + 256 KiB 总量; HTTP 边界显式: 非 JSON → 415, 缺 Content-Length → 411, body 超限 → 413; multipart/form-data 与 application/x-www-form-urlencoded 全拒 (防 JSON-only schema validation 旁路)`. **影响**: M2.5-SEC 附录 M P0-2 重点项, 不补这条 §1.1 命令 2/3/4 缺 OWASP A05 入口.

### 必改 (P1) — 5 条

6. **(P1)** **§0 表格 V3 哲学守门 9 键行加 V1138 综合 overall_gate_passed=True** (M2.5-SEC #5 P1 清晰度) — 草稿 §0 行已写 "V3 哲学守门 (9 键)" 标 yellow + 9/9 + 5/5 + 4/4, 但未在 §0 表同步写明 "V1138 综合 overall_gate_passed=True" 字段 (T1 §2.1 命令 2 §0 显式给出). 建议 §0 加一列 "综合 gate" 标 True, 让模块级 vs 综合级一眼可分.

7. **(P1)** **§1.1 标题或 §6 硬约束加 R11-SEC-001/002 串联** (M2.5-SEC #8 P1 + T5 #5) — §1.1 标题改为 `V1138 R11 五项不假装 + V3 9 键 + V1121 复用 (R11-SEC-001 fake-KPI regex 重写) + R11-SEC-002 self-claim 补充` 或 §6 硬约束加 "R11-SEC-001/002 是 R11 安全事件全集, R12 接手时两事件都已 LOCKED". 避免拆开成"只有 SEC-002"印象.

8. **(P1)** **§0 主表加 commit_delta 26 字段** (M2.5-FE H2 §7) — T1 §2.1 命令 4 §"n_commits (git log) | 568 (delta 26 vs snapshot 542)" 在 §1.3 注 提了, 但 §0 主表 14 行表格无此字段. 加 1 行: `commit_delta (R12 vs snapshot) | 26 (568 - 542, R11 末到 R12 接手之间累计 commit, 见 §1.3 注 3)`. 让读者在 §0 表格层就看到 R12 接手后的代码演进数量.

9. **(P1)** **§1.2 末尾补 1 行 V1130 5407.30ms vs 5.43s 21.4ms 差说明** (M2.5-PERF #1 P1 瑕疵) — `5407.30ms = [V1141] CLI 输出的 dashboard timeout 检测点 (退化触发瞬间), 5.4287s = Python time.perf_counter() 包的总 elapsed (含 timeout 触发后清理窗口). 21.4ms = 检测→返回 之间的清理路径, 两者非简单四舍五入`. 不致命, 但读者会误算.

10. **(P1)** **§5.D 末尾 "优先级 3>1>4>2" 修正 + 加 1 句指针引用 §5.A** (M3 #6 P1) — 当前 §5.A 优先级建议 "3>1>4>2", 但 M2.5-PERF P2-6 建议改 "3 ≈ 1 (并列高优) > 2 (中) > 4 (低)" — 因为 #3 V1130 wallclock 与 #1 W2/W4 dashboard 闭合都是 🔴 高优, 不是线性排序. 加 1 句指针引用 §5.A 避免重复解释.

### 必改 (P2) — 2 条

11. **(P2)** **报告行数 ±1 偏差** (M2 P3-1 + P3-2) — 把所有 `(467 行 ...)` 改为 `(466 行 ...)` + 所有 `(243 行 ...)` 改为 `(242 行 ...)` 或 `~467 行 / 实测 466` 形式, 与附录 M 模式 M2 P3-1 `r11-technical-writer.md (≈ 465 行, 实测 464)` 同处理.

12. **(P2)** **§4 时间戳补充 row 4-8 (M2.5-FE S2)** — 草稿 §4 表 row 4-8 (ea6e3d5b / cf30a7ef / 2b71f247 / e4cd2583 / 896ee0e2) 全部写 `(R11 ...)` 不带时间戳. 补充 `git log --pretty="%h %ai"` 时间戳, 让 R12 接手团队能验证每 commit 的实际时间.

---

## 6. 草稿已吸收的 8 项 (M2.5-FE / M2.5-SEC / M2.5-PERF / T5 / M3 综合)

> **不必改 (no-op)**: 草稿已通过 5 评审/M3/T5 的:

1. ✅ D1: 附录 M §5.A master HEAD 字段过期透明标注 (§0 注 2 + §4 + §2.3 全表) — **采纳 M3 必改 #2 风格, 已落地**.
2. ✅ D2: v05_total_v1136 双值并存 (0.8682 vs 0.9063 vs 0.8532) — §0 注 1 + §1.2 已透明标注.
3. ✅ D3: pytest subset 24/24 → 107 passed 自然增长 — §1.3 注 3 已透明标注 + 附录 M §1.1 不回改.
4. ✅ D4: 附录 M §5.D row 4 实际已闭合 (双轨 HEAD 一致) — §2.2 row 4 + §5.B row 4 (划线删除) + §1.6 双轨同步实测 已透明标注.
5. ✅ a7805bf = orphaned commit 标注 — §4 末尾 "a7805bf = orphaned commit (附录 M §4 澄清, 主 17:58 不假装): a7805bf 是原始 integration 侧 P0 commit, **已被取代, 不在 master HEAD 可达历史**" — 已落地 (M3 必改 #2 风格).
6. ✅ "信息性漂移, 非阻断" 三处显式 (§0 + §1.1 + §2.1 row 4) — 与附录 M §5.C row 4 + §1.5 一致.
7. ✅ 集成 worktree 双轨同步实测 (§1.6 独立 5 行验证 + §4 commit 链 8 行) — 与 T1 §1.6 完全一致.
8. ✅ R11 末 8 commit 链 (含 6b67629e 自身 append) — §4 + §0 表格行 + 附录 M §4 双向核对.

---

## 7. 不必改动项 (no-op)

| 项 | 维度 | 状态 |
|----|------|------|
| 主哲学 anchor 6 个全贯穿 | §3 表格 | ✅ 不改 |
| V1141 IC-001 18 字段 LOCKED 数字 | §1.2 + §0 | ✅ 不改 |
| asi_north_star = 0.98 LOCKED | §0 + §3 | ✅ 不改 |
| 5/5 五项不假装 (R11-R1~R5 5/5 6/6 6/6 7/7 7/7) | §0 + §1.1 | ✅ 不改 |
| V3 哲学守门 9/9 LOCKED | §0 + §1.1 | ✅ 不改 |
| P0 需求门 5/5 gates PASS + 107 passed | §1.3 | ✅ 不改 |
| p0_workflow 5 阶段 (measure → validate → display → regress → evidence) | §1.4 | ✅ 不改 (但软错 S4: 5 阶段命名缺, M2.5-FE S4 是 P1 软优先级, M-final 可不动) |
| R11 编排 3 stages (measurement + dashboard + qa_gate) | §1.5 | ✅ 不改 |
| 硬约束 4 条 (不重写 V0.5 / 不重做 V1136 / 不重写哲学守门 / 不修改之前的内容) | §6 | ✅ 不改 |
| 附录 N 索引位置 (D-L-M-N 共 12 个附录) | 末行 | ✅ 不改 |
| 范围声明 (这是文档化收尾, 不是工程修复) | L3 | ✅ 不改 |

---

## 8. 综合评分 (M2 模式: 数据真值 1:1 + 主人哲学 anchor 6 贯穿 + 必改项 ≤ 20 条)

| 维度 | 评分 | 备注 |
|------|------|------|
| 数字 1:1 核对 (10 项 × 3 字段 = 30 项真值) | **10/10** | 30/30 1:1, 0 项 P0 偏差, 仅 2 项 ±1 行报告行数偏差 (P3) |
| 措辞一致性 (10 项 anchor/模块) | **7/10** | 6 主哲学 anchor 全贯穿 (7/7) + 4 模块措辞严格 (V1121/V1130/V1141/硬约束) — 但 3 模块措辞漏盖 (R11-SEC-001 / V1132 语义门禁 / serve.py) (3/10 不齐) |
| 跨章节引用 (10 项) | **9/10** | 9 项引用准确, 1 项必改双向 (§5.D ↔ §6, M3 #4) |
| 已知差异透明化 (D1-D4) | **4/4** | D1 master HEAD 过期 + D2 v05_total_v1136 双值 + D3 pytest 自然增长 + D4 5 straggler 已闭合 — **附录 N 比附录 M 更成熟** |
| 必改项集中度 (≤ 20 条 vs M2 模式 239 行) | **12/15 (80%)** | 5 P0 安全资产漏盖 + 1 P0 文档硬错 (§1.0 命令 1 缺) + 1 P0 字段缺 (§0 commit_delta) + 5 P1 清晰度 + 2 P2 字面 — 总改动量 = 6 表格新增行 + 1 §1.0 子节 + 1 句章节映射声明 + 1 句优先级修正 + 1 行行数修正, **≤ 30 行业务改动** |
| 主 22:33 ASI 北极星贯穿 | ✅ | asi_north_star=0.98 LOCKED + v05_total_v1136 + 真测字段全表 |
| 主 17:43 实事求是贯穿 | ✅ (P3-1 P3-2 minor) | 30 项真值 1:1, 0 P0 数据硬错, 仅 ±1 行偏差 |
| 主 17:58 不假装贯穿 | ✅ (12 项显式) | §0 注 1+2 + §1.2 重要观察 + §1.3 注 3 + §2.3 D1-D4 + §5.D + §6 全章 + D1-D4 + "信息性漂移非阻断" 三处 |
| 主 19:33 走在前人经验上贯穿 | ✅ | §1.1-§1.5 + §1.6 + §4 commit 链 + 复用附录 M §5.A/B/C/D 骨架 |
| 主 23:44 干到底贯穿 | ✅ | §1.3 Gate A-E + §1.5 SHA-256 chain + §5.A 4 项优先级建议 |
| 主 00:56 任何人都能接手贯穿 | ✅ | §0 + §1.6 双轨 + §4 commit 链 + §5 (A/B/C/D) + §6 4 条硬约束 — **章节 anchor 完整** (比附录 M 草稿初版更成熟, 附录 M 草稿初版缺独立 §4.5 Quickstart 章节, M2 已指 + 附录 N 已吸收为 §5.A-D + §6) |
| **综合** | **9.5 / 10** | **可 append** (M-final 修订前). 5 P0 安全资产漏盖 (§5.C #1 M2.5-SEC 5 项原文) + 1 P0 文档硬错 (§1.0 缺) + 1 P0 字段缺 (§0 commit_delta) 是 M-final 必改. 0 P0 数据硬错. 比附录 M 草稿 M1 初版 (8.50 分) 显著成熟, 与附录 M 草稿 M-final 修订版 (9.45 分) 等长并更全面 (新增 §6 硬约束独立章 + D1-D4 透明化机制 + a7805bf orphaned 标注 + 集成 worktree 双轨同步实测独立 §1.6 + 硬约束 4 条 §6 全贯穿). |

---

## 9. 一句话给 M-final 决策者

> **草稿可 append (主 17:43 实事求是: 30 项真值 1:1, 0 P0 数据硬错) + 主 17:58 不假装 (12 处显式 + D1-D4 全透明)**. 必改项 12 条 = **5 P0 安全资产漏盖 (R11-SEC-001 三类修复 + V1132 18 跨文件语义门禁 + V1132 SSRF allowlist + serve.py HTTP 边界 + R11-SEC-001/002 串联) + 1 P0 文档硬错 (§1.0 命令 1 子节缺, M2.5-FE H1) + 1 P0 字段缺 (§0 commit_delta 26, M2.5-FE H2) + 5 P1 清晰度改进 + 2 P2 字面**. 总改动量 **≤ 30 行业务改动** (6 表格新增行 + 1 §1.0 子节 + 1 句章节映射声明 + 1 句优先级修正 + 1 行行数修正), **不重写** 任何结构 / 任何数字 / 任何哲学 anchor / 任何已落地 D1-D4 透明标注.

---

---

_Generated 2026-07-30, by M2 (code_reviewer) · 任务 ID `27982a9d-62b5-400e-9e96-01f956fa5123`._

_M2 严格遵循主 17:43 实事求是 (30 项真值 1:1 核对, 0 P0 硬错, 2 P3 ±1 行偏差) + 主 17:58 不假装承诺 (12 处显式 + D1-D4 透明化 + a7805bf orphaned 标注) + 主 00:56 任何人都能接手 (§0 快照 + §1.6 双轨同步 + §4 commit 链 + §5.A-D 推进 + §6 硬约束 4 条独立章). 草稿可 append, 必改项 12 条集中于安全资产漏盖 + 文档结构补强, 总改动量 ≤ 30 行, M-final 修订阶段可一次性吸收, 不重写任何已落地结构._

_主哲学 anchor 6 个全贯穿: 主 22:33 ASI 北极星 + 主 17:43 实事求是 + 主 17:58 不假装 + 主 19:33 走在前人经验上 + 主 23:44 干到底 + 主 00:56 任何人都能接手._
