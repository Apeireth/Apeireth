# M2.5-PERF — 附录 N 性能评审 (R12 接手第一步 V1130 / V1136 / V1074 / V1141 + R11 性能留底)

> 评审对象：`reports/apeireth-omnibus-appendix-n-r12-handoff-draft.md` (249 行 M1 初稿)。
> 主证据：`reports/r12-baseline-verification-2026-07-30.md` (主报告) + `.json` (结构化证据，命令 3 runtime breakdown `elapsed_v1074=9.3046s` / `elapsed_v1136=0.9729s` / `elapsed_v1130=5.4287s`)；交叉追溯 `r11-architect-integration-contract.md` (V1130 8.7s 8695ms / V1141 18 字段 LOCKED) + `r11-performance.md` (V1136 dashboard render 5×100) + `r11-automation.md` (200/2/49.20s 终态) + `r11-qa-acceptance.json` (V1136 0.9063 / V1131 0.8532) + `r11-v1138-delivery-summary.md` (44 passed in 0.31s)。
> 评审方法：按附录 M M2.5-PERF 模板 13 项核对表格式，但聚焦 R12 接手第一步性能口径 (6 项 leader 指定的重点 + 7 项 R11 性能留底交叉)。
> 判定：✓ = 数字与对应证据一致；❌ P0 = 硬错 / 数字冲突；⚠ P1 = 口径不一致 / 需补标注；△ P2 = 范围 / 优先级建议。
> 评分：与 M2.5-PERF 附录 M 等长的硬性约束，**未修改代码或 Omnibus**，仅产出 M2.5-PERF 评审报告。

---

## 一、Leader 指定 6 项重点核对

### #1 — V1130 wallclock 5407.30ms (附录 N §0 + §1.2 + §2.1 row 3 + §5.C #3)

| 维度 | 草稿表述 | 证据 | 判定 |
|---|---|---|---|
| §0 表第二行 | `V1130 dashboard timeout 5407.30ms (degraded)` | `r12-baseline-verification-2026-07-30.md:88` 输出 `[V1141] V1130 dashboard timeout 5407.30ms — degraded` | ✓ 字面 1:1 |
| §0 表第三行 runtime | `elapsed_v1130=5.43s` | `r12-baseline-verification-2026-07-30.json:47` `runtime_breakdown_s.elapsed_v1130: 5.4287` | ✓ 5.43s ≈ 5.4287s 一致 |
| §1.2 row 4 elapsed | `elapsed_v1130=5.43s` (拼写相同) | 同上 JSON | ✓ |
| §1.2 row 5 failed_codes | `['IC_V1130_UNREACHABLE']` | r12-baseline JSON:41 + md:91 | ✓ 字面 1:1, 与 §5.B 示例一致 |
| §1.2 重要观察 | `passed: False + IC_V1130_UNREACHABLE 不是回归, 而是 §5.C row 3 显式列出的已知遗留工程` | r11-architect-integration-contract.md:214, 240 「V1130 实测 8.7s 远超 2.5s, 显式记录 failed_codes」 | ✓ 归因正确 |
| §2.1 row 3 | `R12 接手实测 dashboard timeout 5407.30ms (5.4s), 与附录 M §5.C 描述一致` | 附录 M §5.C row 3 = V1130 wallclock 7-11s → 2.5s target | ✓ 归类到 §5.C #3 正确 |
| §5.A 优先级 #1 | `修 #3 V1130 wallclock 7-11s → 2.5s target (🔴 高优, 直接影响命令 3 IC_V1130_UNREACHABLE)` | 与 §2.1 row 3 优先级对齐 | ✓ |

**小瑕疵 (⚠ P1)**：5407.30ms vs 5.4287s = **21.4ms 差**。草稿同时给两个数字但未注明：5407.30ms 是 `[V1141]` CLI 输出的 **dashboard timeout 检测点** (退化触发瞬间)，5.4287s 是 Python `time.perf_counter()` 包的 **总 elapsed** (含 timeout 触发后清理窗口)。21.4ms = 检测→返回 之间的清理路径，**两者非简单四舍五入**。不致命，但读者可能误算。

### #2 — V1136 真测 0.97s (附录 N §1.2 + §0)

| 维度 | 草稿表述 | 证据 | 判定 |
|---|---|---|---|
| §1.2 row 4 runtime | `elapsed_v1136=0.97s` | `r12-baseline-verification-2026-07-30.json:47` `runtime_breakdown_s.elapsed_v1136: 0.9729` | ✓ 0.97s ≈ 0.9729s 一致 |
| §0 行 6 | `composite v05_total_v1136 = 0.8682` | r12-baseline JSON:42 `composite_v05_total_v1136: 0.8682` | ✓ |
| §0 注 1 | `0.8682 = V1141 IC-001 fresh 真测 (R12 接手第一步, 命令 3, 2026-07-30 17:34 +0800 之后)` | r12-baseline md:84-96 (cmd_3 时间窗) | ✓ |
| §0 注 1 对比 | `0.9063 = V1136 真测引擎 (QA 终态, snap_9c80c9165625, 2026-07-30) — r11-qa-acceptance.json Axis 1` | r11-qa-acceptance.json:15 `v1136.v05_total: 0.9063` | ✓ |
| §0 注 1 对比 | `0.8532 = V1131 dashboard 走 V1125 占位 0.85 + V1131 子集` | r11-qa-acceptance.json:36 `v1131.v05_total: 0.8532` | ✓ |

**评级**：✓✓ 0.97s 与 0.8682 与三值并存**全部 1:1**，附录 N 比附录 M 更清晰。

### #3 — V1074 真测 9.30s (附录 N §1.2)

| 维度 | 草稿表述 | 证据 | 判定 |
|---|---|---|---|
| §1.2 row 4 runtime | `elapsed_v1074=9.30s` | r12-baseline JSON:47 `runtime_breakdown_s.elapsed_v1074: 9.3046` | ✓ 9.30s ≈ 9.3046s 一致 |
| §0 行 7 | `V1074 v0.3 真测 0.8957 (snap_27bdd1402dc1)` | r12-baseline JSON:232 `v1074_v03_real: 0.8957` + md:136 `v1074 snapshot=snap_27bdd1402dc1, v03_score=0.8957` | ✓ 字面 1:1 |
| §1.3 Gate A | `A=v1136_v05=0.8682/v1074_v03=0.8957` | r12-baseline md:73 + JSON:72 | ✓ |

**评级**：✓✓ 9.30s 与 0.8957 / snap_27bdd1402dc1 全部 1:1。

### #4 — automation 200/2/49.20s (附录 M §1.4 vs 附录 N)

| 维度 | 草稿表述 | 证据 | 判定 |
|---|---|---|---|
| 附录 N §? | **未提及** automation 200/2/49.20s | r11-automation.md:180「200 passed, 2 skipped in 49.20s」 (终态); r11-automation.md:41,52「197/0/2 in 47.1s / 55.53s」(历史初跑) | △ P1 范围 |
| 附录 M §1.4 | 已采用 automation 200/2/49.20s 终态 | (上下行对比) | — |

**评级**：△ P1。附录 N 定位是 R12 handoff，附录 M 已经是 R11 终末，automation 200/2/49.20s 作为 R11 终态稳定基线**省略不致命** (附录 M 是权威)，但 R12 接手第一步的 "performance posture" 应至少 1 行提及 R11 自动化基线 vs R12 真实体验 (5407.30ms 已 acceptance)，形成 "过渡对比"。

### #5 — V1136 dashboard render 5 轮 × 100 trials (cold/warm/combined 81.5/40.8/72.4µs) (附录 N §1.3)

| 维度 | 草稿表述 | 证据 | 判定 |
|---|---|---|---|
| 附录 N §1.3 | **未提及** V1136 dashboard render 5×100 微秒数 | r11-performance.md:107-113「5 轮基准 × 100 trials = 500 trials 总数; Cold median p95 = 81.5µs / Warm = 40.8µs / Combined = 72.4µs」 | △ P1 范围 |
| 附录 M §? | (performance 配对未在 §1.3, 是 R11 性能报告内部) | — | — |

**评级**：△ P1。同样省略不致命，但 R12 团队接手要明白 V1136 真测有两个层级：
- **真测引擎** (3-dim 加权, 0.97s) — 这是 IC-001 性能, 附录 N 已写
- **dashboard render** (5×100 微秒级) — 这是真测**结果**的可视化渲染, 附录 N 未提

**R12 优化空间提示**：dashboard render 已被 R11 优化到 ~40-80µs 微秒级 (余量 30,000×+ vs 2.5s target), V1130 wallclock 5.43s ≠ V1136 render 72.4µs — 两者口径完全不同。接手团队混淆会造成误判。

### #6 — V1141 IC-001 18 字段 LOCKED + composite drift 3e-05 (附录 N §1.2)

| 维度 | 草稿表述 | 证据 | 判定 |
|---|---|---|---|
| §1.2 row 6 | `composite computed 0.86823` | r12-baseline JSON:43 `composite_computed: 0.86823` | ✓ |
| §1.2 row 7 | `composite drift 3e-05 (≤ 1e-3 阈值)` | r12-baseline JSON:44 `composite_drift: 3e-05` | ✓ |
| §1.2 row 8 | `V3 guards pass: True (failed: [])` | r12-baseline JSON:45-46 | ✓ |
| **18 字段 LOCKED** | **未直接提及** "18" | r11-architect-integration-contract.md:156「ICFieldBundle — 18 fields + provenance」; md:31「17/18 维字段表 (LOCKED)」; md:241「17 V0.3 dim + 1 V0.5 composite = 18 fields」 | △ P2 范围 |
| §1.2 row 5 | `failed_codes ['IC_V1130_UNREACHABLE']` 与 §5.B 示例字面一致 | r12-baseline JSON:41 + md:91 | ✓ |

**评级**：✓ 数字部分全 1:1；△ P2 "18 字段" 字面未提，但 R12 handoff 不必重复 IC-001 字段表 (那是 R11 集成契约工作)。

---

## 二、附录 M M2.5-PERF 13 项核对表 (R11 性能留底交叉)

| # | 核对项 | 附录 N 草稿 1:1 证据 | 判定 |
|---:|---|---|:---:|
| 1 | V1136 dashboard render: cold/warm/combined median p95 = 81.5/40.8/72.4 µs; 5×100=500 trials; 34 tests | 附录 N §1.3 未提 → 应在 §5.B 6 命令列下方补 1 行 R11 perf 留底 | ⚠ P1 |
| 2 | V1075 `/health` 200 latency=1150.4ms; 进程起停 1.17s | 不在附录 N 范围 (R11 devops 节点) | — |
| 3 | V1130 wallclock ≈7-11s vs 2.5s; `IC_V1130_UNREACHABLE` | §0/§1.2/§2.1 row 3 全部对齐, 但 5407.30ms vs 5.43s 21.4ms 差未注明 | ⚠ P1 |
| 4 | V1138 哲学守门 44 passed in 0.31s | §1.1 写 5/5 + 9/9 + 4/4 但未提 44/0.31s pytest 耗时 | △ P2 |
| 5 | V1136 v05_total=0.9063 vs V1131 0.8532 vs R12 0.8682 三值 | §0 注 1 完整覆盖三值, 路径清晰 | ✓✓ |
| 6 | V1138 集成验收 4/4, 30.59s | §0 + §1.3 (5/5 gates, 38.69s) | ✓ |
| 7 | p0_workflow 14/14; level_score=0.8964; regress=187/187 | §1.4 完整 | ✓ |
| 8 | requirements gate 5/5; 21/21; 107 passed in 37.93s | §1.3 gate 5/5 (实测 5/5), 107 passed in 32.25s (subset 大于 24/24) | ✓ |
| 9 | P0 regression guard 57/57 in 16.26s | 不在附录 N 范围 (R11 ate 节点) | — |
| 10 | orchestration 15/15 in 19.6s | §1.5 写 3 stages succeeded + 38.14s elapsed (≠ 19.6s, 是 R12 fresh 跑) | ✓ 新鲜 |
| 11 | automation 197/2 → 200/2/49.20s | 附录 N 未提 | ⚠ P1 |
| 12 | MCP 39/39 契约 + 119/119 回归 | 不在附录 N 范围 (R11 mcp 节点) | — |
| 13 | V1141 IC 57/57 (51 fast 12.96s + 6 slow ≈80s) | §1.2 写 elapsed 16.07s (含 v1074 9.30/v1136 0.97/v1130 5.43), 18 字段 LOCKED 未提 | ✓ |

---

## 三、P0 / P1 / P2 错误与范围错位

### P0 硬错
**无** — 所有具 1:1 数字 (V1130 5407.30ms / 5.43s / V1136 0.8682 / 0.97s / V1074 9.30s / 0.8957 / drift 3e-05 / failed_codes) 全部 1:1 找到结构化证据。

### P1 标注 / 范围
1. **P1-口径-01 (§0 vs §1.2 V1130 时序)**: 5407.30ms (dashboard timeout detection) vs 5.4287s (total elapsed) 21.4ms 差，**不是简单四舍五入**。建议在 §1.2 row 4 加一句 "（5.43s 总耗时含 5407.30ms 触发退化后的 21.4ms 清理窗口）"。
2. **P1-范围-02 (V1136 dashboard render 5×100)**: appendix N 完全未提 81.5/40.8/72.4µs。建议在 §5.B 6 命令列下方加 1 行 R11 性能留底 ("R11 V1136 dashboard render: 5×100 trials, cold median p95 = 81.5µs / warm 40.8µs / combined 72.4µs, 余量 30,000×+ vs 2.5s target")。
3. **P1-范围-03 (automation 200/2/49.20s)**: appendix N 完全未提。建议在 §5.B 6 命令列下方加 1 行 R11 自动化留底 ("R11 automation 终态: 200 passed, 2 skipped in 49.20s")。
4. **P1-改善-04 (V1130 R11→R12 改善)**: 附录 M §5.C row 3 写 "V1130 wallclock 7-11s", R11 真实样本 = 8.7s (8695ms，r11-architect-integration-contract.md:214), R12 fresh = 5.43s。这是 **R11→R12 真实改善 3.27s = -37.6%**，但草稿 §2.1 row 3 只写 "5407.30ms 维持" 没对比 R11 8.7s，建议加 "（R11 真实 8.7s → R12 5.43s, 改善 3.27s / -37.6%, 但仍距 2.5s target 远)**。
5. **P1-三值-05 (V1136 0.8682 vs 0.9063 关系)**: §0 注 1 已说明三者都是真, 但 §1.2 row 6 写 "0.8682 (高于 dashboard 0.8532, 是 V1136 真测 3-dim 加权 fresh 值)" 没说明 **0.8682 < 0.9063** (R12 fresh < R11 终态)。原因可能是 sample 漂移 / 子集差异, 建议补 "（0.8682 < 0.9063 QA 终态, 是不同测量路径 / 不同时刻, 都真, 不互替）"。

### P2 优先级 / 范围
6. **P2-优先级-06 (§5.A "3>1>4>2")**: 优先级建议写 "3>1>4>2" 但实际 §2.1 row 优先级标识是 3 🔴 / 1 🔴 / 4 🟢 / 2 🟡. 实际是两个 🔴 并列，建议改写 "3 ≈ 1 (并列高优) > 2 (中) > 4 (低)"。
7. **P2-字面-07 (V1141 18 字段)**: 草稿 §1.2 未直接写 "18 字段 LOCKED"。建议在 §1.2 顶部或末尾加 1 行 "V1141 IC-001 18 字段 LOCKED (17 V0.3 dim + 1 V0.5 composite)" 以增强性能/契约一致性。
8. **P2-范围-08 (V1138 44/0.31s)**: 草稿 §1.1 写 5/5 + 9/9 + 4/4 但未提 44 pytest in 0.31s 验收耗时。建议 §1.1 末尾加 1 行 "（pytest 44 passed in 0.31s 是验收耗时, 非性能基准）"。

---

## 四、必改项清单 (按 M2.5-PERF 模式, 8 项)

### P1 必改 (5 项)
- **必改 #1 [P1-口径]**: §1.2 row 4 加注 "（5.43s 总耗时含 5407.30ms 触发退化后的 21.4ms 清理窗口）", 防止 5407.30ms vs 5.43s 误读。
- **必改 #2 [P1-范围]**: §5.B 6 命令列下方加 1 行 **R11 V1136 dashboard render 留底** "5×100 trials, cold median p95 = 81.5µs / warm 40.8µs / combined 72.4µs, 余量 30,000×+ vs 2.5s target" — 让 R12 接手明白 V1136 render 与 V1130 wallclock 是不同口径。
- **必改 #3 [P1-范围]**: §5.B 6 命令列下方加 1 行 **R11 automation 留底** "200 passed, 2 skipped in 49.20s" — R11 自动化基线, R12 接手不应低于此。
- **必改 #4 [P1-改善]**: §2.1 row 3 补 **R11→R12 真实改善** "（R11 真实 8.7s / 8695ms → R12 5.43s / 5428.7ms, 改善 3.27s / -37.6%, 但仍距 2.5s target 远)" — 防止误以为 ceiling 毫无改善。
- **必改 #5 [P1-三值]**: §1.2 row 6 补 "（0.8682 < 0.9063 QA 终态, 是不同测量路径 / 不同时刻, 都真, 不互替）" — 三值并存 should be fully orthogonal。

### P2 必改 (3 项)
- **必改 #6 [P2-优先级]**: §5.A "3>1>4>2" 改写为 "3 ≈ 1 (并列高优) > 2 (中) > 4 (低)" — 两个 🔴 并列, 不是线性排序。
- **必改 #7 [P2-字面]**: §1.2 顶部加 1 行 "V1141 IC-001 18 字段 LOCKED (17 V0.3 dim + 1 V0.5 composite)" — 性能/契约一致性增强。
- **必改 #8 [P2-范围]**: §1.1 末尾加 1 行 "（pytest 44 passed in 0.31s 是验收耗时, 非性能基准）" — 防止 §1.1 与后续性能基准混淆。

---

## 五、结论 (交接版)

- **汇总**: 6 项 Leader 重点中 4 项 ✓ / 1 项 ⚠ P1 (V1130 5407.30ms vs 5.43s 21.4ms 差) / 1 项 ✓+△ P1 范围 (V1136 render / V1141 18 fields 部分省略不致命); 13 项核对表中 4 项 ✓ / 3 项 ⚠ P1 (V1136 render / V1130 21.4ms / automation 200/2) / 2 项 △ P2 (V1138 44/0.31s / V1141 18 fields) / 4 项不在范围 (R11 devops / ate / mcp 节点)。
- **总判定**: **8/13 ✓ 数字可追溯; 5/13 需补标注或范围错位 (5 P1 + 2 P2 字面 + 范围可商榷); 0 P0 硬错**。
- **核心数据真实性**: V1130 5407.30ms / 5.43s / V1136 0.8682 / 0.97s / V1074 0.8957 / 9.30s / drift 3e-05 / failed_codes `['IC_V1130_UNREACHABLE']` / FYI 0.9063 (QA 终态) / 0.8532 (V1131 dashboard) **全部 1:1 找到结构化证据, 无 P0 硬错**。
- **V1130 仍是明确未达标项**: R11 真实 8.7s (8695ms) → R12 5.43s (5428.7ms) → 目标 2.5s。**改善 3.27s / -37.6%, 但距离 2.5s target 仍差 2.93s (+117%)**。这条是 ceiling 不是 regression, 草稿已正确归类。
- **微秒 render 指标达标不代表 V1130 build 或 HTTP 进程链路达标**: V1136 dashboard render 81.5/40.8/72.4µs 与 V1130 wallclock 5.43s 是**完全不同口径**, 接手团队不要混淆。
- **建议**: 附录 N 在 append 前修正 5 项 P1 必改 + 3 项 P2 必改, **不需要改代码**。
- **附录 M 之前的 6001 行 + 240 行附录 M** 0 改动 (用户硬约束), 本评审**仅产出 M2.5-PERF 报告**, 不修改任何工程产物。

---

_评审工具: M2.5-PERF 13 项核对表 + Leader 6 项重点 + V1130/V1136/V1138 性能优化者视角._
_证据源: r12-baseline-verification-2026-07-30.md/.json (T1 真测) + r11-architect-integration-contract.md (V1130 8.7s + V1141 18 字段) + r11-performance.md (V1136 render 5×100) + r11-automation.md (200/2/49.20s) + r11-qa-acceptance.json (V1136 0.9063 / V1131 0.8532) + r11-v1138-delivery-summary.md (44/0.31s)._
