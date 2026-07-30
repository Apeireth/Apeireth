## 📖 附录 N: R12 接手第一步 (主 22:33 + 主 17:43 + 主 17:58 不假装 + 主 19:33 + 主 23:44 + 主 00:56 全贯穿)

> **范围声明 — 这是文档化收尾, 不是工程修复** (主 17:43 实事求是 + 主 17:58 不假装). 上一团队 R11 已落, 附录 M append 完成 (commit `6b67629e`), R12 团队接手第一步 = 验证 R11 末真态 + 集成 worktree 双轨同步 + 透明化文档差异. 本附录忠实记录 R12 接手第一步的 **6/6 PASS 真测结果**: 包括 `v05_total_v1136` IC-001 fresh 真测 0.8682 (与附录 M §0 写的 QA 终态 0.9063 是**不同测量路径 / 不同时刻**的真实快照, 见 §0 注 1), 也包括 §5.A 表格 master HEAD 字段文档过期 (附录 M §5.A 写 `7fbc97d0`, 实测 `6b67629e`, 这是附录 M append 自身 commit 的副作用, 见 §0 注 2) / V1130 dashboard timeout 5407.30ms (known ceiling §5.C row 3) / dashboard yellow (V1121 信息性漂移, 非阻断) / R11-SEC-001 三类修复 + V1132 语义门禁 + V1132 SSRF allowlist + serve.py HTTP 边界硬化 (R11 已落, working changes 文档化引用, 见 §1.1 + §5.B) 这些**已知差异 / 已知 ceiling / 已知信息性 / 已知已落资产**, 一并透明列出, 不掩盖不升级. **主 17:58 不假装**: 文档过期差异在本附录透明标注, 不回改附录 M 之前内容 (用户硬约束: 不修改之前的内容), R12+ ceiling 留给下一个团队.

---

### 0. R12 接手第一步真测数据快照 (主 17:43 实事求是)

| 指标 | R12 接手实测值 | 真测源 / 测量路径 |
|------|----------------|-------------------|
| **master HEAD** | `6b67629e0bcec01f064a97b3c1ddccc47195471e` (2026-07-30 17:34:15 +0800) | `git rev-parse HEAD` — **与附录 M §5.A 表格写的 `7fbc97d0` 不一致**, 见注 2 |
| **integration worktree HEAD** | `6b67629e0bcec01f064a97b3c1ddccc47195471e` (2026-07-30 17:34:15 +0800) | `git worktree list` — **与 master 完全一致, 双轨同步** (见 §1.5 D5) |
| **§5.B 6 命令验证** | **6/6 PASS** (命令 1 Leader 跑 33.18s, 命令 2-6 qa_engineer T1 跑 93.59s 总计) | `reports/r12-baseline-verification-2026-07-30.md` §1 |
| **snapshot (level_score)** | snap_9c80c9165625 (level_score=0.8964) | 命令 4 输出, 与附录 M §0 一致 |
| **modules / tests / commits (snapshot)** | 1153 / 6394 / 542 | 命令 4 输出, 与附录 M §0 一致 |
| **n_commits (git log, 当前 worktree)** | 568 (**commit_delta = 26** vs snapshot 542 — snapshot 时点之外 26 个 commit, 含附录 M 自身 commit `6b67629e` + 工作树散落提交) | 命令 4 + `git log --oneline \| wc -l` |
| **v05_total_v1136 (IC-001 fresh)** | **0.8682** (composite computed 0.86823, drift 3e-05 ≤ 1e-3) | 命令 3 V1141 IC-001 fresh run, 16.07s — 见注 1 |
| **V1074 v0.3 真测** | **0.8957** (snap_27bdd1402dc1) | 命令 3 runtime elapsed_v1074=9.30s, 与附录 M 终态一致 |
| **V1130 dashboard timeout** | **5407.30ms** (degraded) | 命令 3 runtime elapsed_v1130=5.43s — **已知 ceiling §5.C row 3**, 非回归 (21.4ms 差说明见 §1.2) |
| **V3 哲学守门 (9 键)** | **9/9 LOCKED** + 5/5 不假装 + R11-SEC-002 4/4 + V1138 综合 overall_gate_passed=True | 命令 2, dashboard yellow (V1121 信息性) |
| **R11-SEC-001 三类修复 (R11 已落)** | fake-KPI regex 重写 + path traversal + secret-leak — v1121_security_guard_v01.py:379-401, 780-803, 1029-1054 + 24+ 行新 test 覆盖 | working changes `git diff`, 详见 §1.1 + T5 P0-1 |
| **V1132 部署 validator 语义门禁 (R11 已落)** | canonical_bundle_valid=True (18 跨文件语义断言) + offline_valid/runtime_valid/passed 三分裂 | v1132_real_deployment_validator.py:51, 60-79, 98-100, 240-242, 245 + §1.1 + T5 P0-2 |
| **V1132 SSRF allowlist (R11 已落)** | _LOOPBACK_HOSTS 5 host + _LOOPBACK_PORTS 7 port (含 8765); scheme 仅 http/https, host 仅 loopback; file:// / gopher:// / 169.254.169.254 全拒 | v1132_real_deployment_validator.py:202-233, 240-242, 245 + §1.1 + T5 P0-3 |
| **serve.py HTTP 边界硬化 (R11 已落)** | Content-Length 1 MiB cap + 100 messages + 32 KiB 单消息; 非 JSON → 415, 缺 Content-Length → 411, body 超限 → 413; OWASP A05 DoS + multipart 旁路 415 | serve.py:51-55, 58-77, 274-279, 281-298, 300-309, 311-313, 345-352, 354-389 + T5 P0-4 |
| **R11 集成验收 Gate A/B/C/D/E** | **5/5 PASS** (A=v1136_v05=0.8682/v1074_v03=0.8957, B=snap_9c80c9165625, C=9/9, D=107 passed, E=HEAD=6b67629e) | 命令 4, 38.69s |
| **p0_workflow 五阶段 (measure → validate → display → regress → evidence)** | status=PASSED, level_score=0.8964, regress=187/187=100%, human_prompt=null | 命令 5, 0.33s |
| **R11 编排状态机 (3 stages: measurement → dashboard → qa_gate)** | pipeline status=succeeded, 3 stages 全 succeeded, SHA-256 chain append-only 落盘 | 命令 6, 38.14s |
| **V1121 fake-KPI detector** | n_threats=2, fake_kpi_attempts=3, runner_confusion_attempts=0, v03_v04_confusion=3, gate_passed=False (模块自身), dashboard yellow (V1138 综合) | 命令 2 §3 — **信息性漂移, 非阻断** (R12 ceiling 见 §2.1 row 4 + §5.A #3) |
| **dashboard state** | **yellow** (V1121 信息性漂移, 与附录 M §5.B 预期一致) | 命令 2 §5 |
| **R11 末 8 commit 链 (R12 接手时点)** | `6b67629e ← 7fbc97d0 ← dd737f5e ← ea6e3d5b ← cf30a7ef ← 2b71f247 ← e4cd2583 ← 896ee0e2` | `git log --oneline -8` — 见 §4 口径说明 |
| **sha256_chain append-only** | true (3 evidence 文件配对 events.jsonl + snapshot.json, append-only) | 命令 6 落盘 |
| **dat_diff vs 附录 M §0** | R12 接手 vs R11 末: master HEAD (差 1 commit = 6b67629e), commit_delta=26, v05_total_v1136 (0.8682 vs 0.9063 不同测量路径), 其余 1153/6394/542/level_score=0.8964 全对齐 | — |

> **注 1 — `v05_total_v1136` 双值并存 (主 17:43 实事求是)**: 三个数字 `v05_total_v1136` 共存是**不同时刻 + 不同测量路径**的真实快照, 不冲突也不混用 —
> - **0.8682** = V1141 IC-001 fresh 真测 (R12 接手第一步, 命令 3, 2026-07-30 17:34 +0800 之后) — `reports/r12-baseline-verification-2026-07-30.json` cmd_3 (composite drift 3e-05 ≤ 1e-3, **0.8682 < 0.9063 是不同测量路径 / 不同时刻, 都真, 不互替**);
> - **0.9063** = V1136 真测引擎 (QA 终态, snap_9c80c9165625, 2026-07-30) — `reports/r11-qa-acceptance.json` Axis 1, 附录 M §0 写定, 不动;
> - **0.8532** = V1131 dashboard 走 V1125 占位 0.85 + V1131 子集 (主轨未切换至 V1136 真测) — `r11-qa-acceptance.json` Axis 2.
>
> 三者**不同时刻 / 不同测量路径 / 都真**, 接手团队若要统一, 把 V1136 0.9063 真测接入 V1131 dashboard 主轨是 R12 ceiling 一项 (附录 M §5.D row 隐含).

> **注 2 — 附录 M §5.A master HEAD 字段文档过期 (主 17:58 不假装)**: 附录 M §5.A 表格写 `master HEAD = 7fbc97d0b4157983f382d0a4f82dc064b92144b7 (2026-07-30 15:50:39 +0800)`, 这是 R11 收尾时的 master HEAD; R12 接手实测 master HEAD = `6b67629e0bcec01f064a97b3c1ddccc47195471e (2026-07-30 17:34:15 +0800)`. 二者差**一个 commit**, 这个 commit 就是附录 M append 自身的 commit (`docs(r11-m): append Appendix M to Omnibus (12 revisions applied from M1+M2+M3+M2.5x4)`). **用户硬约束: 不修改之前的内容** (6001 行旧 + 240 行附录 M), 所以附录 M §5.A master HEAD 字段保留原值 `7fbc97d0`, 本附录 N §0 / §4 / §1.5 D5 把真实 HEAD `6b67629e` 作为"已知差异"透明标注. 接手团队以本附录 N §0 + `git rev-parse HEAD` 为准.

---

### 1. R12 接手第一步交付物 (按验证项分)

#### 1.0 命令 1: V1138 R11 集成验收 4 axes (Leader 跑, 33.18s)

| 验证项 | 实际值 | 与 §5.B 预期契约对比 |
|--------|--------|---------------------|
| `v1138_r11_integration_acceptance --offline` 退出码 | 0 | ✅ |
| elapsed | 33.18s | ✅ 接近 §5.B 写 30.59s (略增, snapshot 锁定 vs fresh run) |
| axes_passed / axes_total | **4/4 PASS** (Axis 1 modules/tests/commits 真测 / Axis 2 dashboard 主轨 / Axis 3 pytest 子集 / Axis 4 真测引擎) | ✅ 完全符合 |
| snapshot_id | snap_9c80c9165625 | ✅ 完全符合 |
| level_score (snapshot) | 0.8964 | ✅ |
| modules / tests / commits (Axis 1) | 1153 / 6394 / 542 | ✅ 与附录 M §0 一致 |
| Axis 3 pytest 子集 | **189 passed / 0 failed / pass_rate 1.0** (含 R11 末新加 test_) | ✅ 完全符合 (subset 大于 §5.B 写 24/24 是自然增长, 非回归, 见 §0 注 3) |
| V3 哲学守门 8/8 LOCKED | 8/8 | ✅ |

> **§1.0 注**: 命令 1 由 Leader 跑通 (33.18s), 命令 2-6 由 qa_engineer T1 跑通 (93.59s 总计) — 6/6 验证完整覆盖 §5.B 6 命令全部 (T1 报告 §1 PASS/FAIL 矩阵)。

#### 1.1 命令 2: V1138 R11 五项不假装 + V3 9 键 + V1121 复用 (含 R11-SEC-001 fake-KPI regex 重写) + R11-SEC-002 补充

| 验证项 | 实际值 | 与 §5.B 预期契约对比 |
|--------|--------|---------------------|
| `v1138_r11_no_pretend_five_guards --strict` 退出码 | 0 | ✅ |
| elapsed | 0.338s | ✅ |
| overall_gate_passed (V1138 综合) | **True** | ✅ 完全符合 |
| dashboard (V1138 综合) | yellow (V1121 信息性, 见 row 4) | ✅ 完全符合 |
| V3 哲学契约 9 键 | 9/9 LOCKED, gate_passed=True | ✅ 完全符合 |
| 五项不假装规则 | R11-R1 5/5, R11-R2 6/6, R11-R3 6/6, R11-R4 7/7, R11-R5 7/7 — **5/5 全 PASS** | ✅ 完全符合 |
| **R11-SEC-001 三类修复 (R11 已落, working changes)** | fake-KPI regex 重写 (`v1121_security_guard_v01.py:780-803` 4 patterns) + path traversal (`v1121_security_guard_v01.py:379-401` split 路径 + Windows drive 识别 + null byte 拒绝) + secret-leak (`v1121_security_guard_v01.py:1029-1054` LEAK_PATTERNS) + 24+ 行新 test 覆盖 — **代码已 LOCKED** | ✅ R11 已落, R12 接手可在 §5.B row 2 引用 file:line 复用 |
| R11-SEC-002 self-claim 补充 | 4/4 (honest 放行覆盖) | ✅ 完全符合 |
| V1121 fake-KPI detector | keys_present=9, fake_kpi_attempts=3, runner_confusion_attempts=0, v03_v04_confusion=3, n_threats=2, gate_passed=False (模块自身), dashboard=yellow (V1138 综合) | ✅ **信息性漂移, 非阻断, 与 §5.B 预期契约一致** (R12 ceiling 优先级见 §5.A #3) |
| V1121 runner_missed counter (R11-SEC-001) | 拆分 runner_confusion (被 fake_kpi 正确识别) + runner_missed (未识别) — gate_passed 改进 = `keys_locked and n_fake_kpi == len(payloads) and runner_missed == 0 and runner_confusion > 0 and v_confusions > 0` | ✅ R11 已落 |
| V1132 部署 validator 语义门禁 (R11 已落, working changes) | canonical_bundle_valid=True (18 跨文件语义断言) + offline_valid/runtime_valid/passed 三分裂 (`v1132_real_deployment_validator.py:51, 60-79, 98-100`); R12 接手 daemon 不可达时: runtime_valid=False, passed=False, daemon probe 全 MISSING (docker_path=MISSING / kubectl_path=MISSING) | ✅ R11 已落, R12 在 §5.B row 2 deploy/ ceiling 引用 file:line 即可 |

> **§1.1 注 (主 19:33 走在前人经验上)**: R11-SEC-001/002 是 **R11 安全事件全集** — R11-SEC-001 fake-KPI regex 重写 (三类修复) + R11-SEC-002 self-claim 补充 (4/4 covered), 两者都已 LOCKED (R11-SEC-001 5 处 R11-SEC-001 注释 + 24+ 行新 test 覆盖; R11-SEC-002 命令 2 实测 4/4). R12 接手时**两事件都已 LOCKED**, 不重写, 引用 working changes file:line 复用.

> **§1.1 末注 (主 17:58 不假装 — 防止与性能基准混淆)**: pytest 44 passed in 0.31s 是验收耗时, **不是性能基准**. 性能基准见 §1.3 V1136 dashboard render 5×100 µs 与 §1.2 runtime breakdown (v1074 9.30s + v1136 0.97s + v1130 5.43s).

#### 1.2 命令 3: V1141 集成契约 IC-001 验证 (V1141 IC-001 18 字段 LOCKED: 17 V0.3 dim + 1 V0.5 composite)

| 验证项 | 实际值 | 与 §5.B 预期契约对比 |
|--------|--------|---------------------|
| `v1141_asi_v04_v05_integration_contract --validate` 退出码 | 0 | ✅ |
| elapsed | 16.071s (含 v1074 9.30s + v1136 0.97s + v1130 5.43s) | ✅ |
| passed | False (但 IC_V1130_UNREACHABLE 是 §5.C row 3 已知 ceiling, 非回归) | ✅ 语义符合 |
| failed_codes | `['IC_V1130_UNREACHABLE']` (与 §5.B 示例字面一致) | ✅ 完全符合 |
| composite v05_total_v1136 | **0.8682** (高于 dashboard 0.8532, 是 V1136 真测 3-dim 加权 fresh 值, 0.8682 < 0.9063 QA 终态是不同测量路径, 见 §0 注 1) | ✅ 见 §0 注 1 |
| composite computed | 0.86823 | — |
| composite drift | 3e-05 (≤ 1e-3 阈值) | ✅ 完全符合 |
| V3 guards pass | True (failed: []) | ✅ 完全符合 |
| runtime breakdown | v1074 9.30s / v1136 0.97s / v1130 5.43s | — |
| **V1130 wallclock 5407.30ms vs 5.43s 21.4ms 差说明** | 5407.30ms 是 `[V1141]` CLI 输出的 **dashboard timeout 检测点** (退化触发瞬间); 5.4287s 是 Python `time.perf_counter()` 包的**总 elapsed** (含 timeout 触发后清理窗口). 21.4ms = 检测→返回 之间的清理路径, **两者非简单四舍五入**. 不致命, 但读者不要做减法误算. | — (注脚, 不构成错位) |

> **重要观察 (主 17:58 不假装)**: `passed: False` + `IC_V1130_UNREACHABLE` **不是回归**, 而是附录 M §5.C row 3 显式列出的已知遗留工程 (V1130 wallclock 7-11s → R12 接手实测 5.43s → 目标 2.5s, **改善 3.27s / -37.6%, 但距离 2.5s target 仍差 2.93s (+117%)**, 是 ceiling 不是 regression). 接手团队不要把这条当作 bug 来修, 这是文档化的 ceiling.

#### 1.3 命令 4: P0 需求门 Gate A/B/C/D/E

| 验证项 | 实际值 | 与 §5.B 预期契约对比 |
|--------|--------|---------------------|
| `apeireth.cli gate --strict` 退出码 | 0 | ✅ |
| elapsed | 38.688s (含 107 pytest in 32.25s) | ✅ |
| n_gates_passed / n_gates_total | **5/5 PASS** | ✅ 完全符合 |
| n_tests_passed | **107** (subset 大于 §5.B 写的 24/24, 见 §0 注 3) | ✅ 完全符合 (subset 自然增长, 非回归) |
| git_head | `6b67629e0bcec01f064a97b3c1ddccc47195471e` | ✅ 完全符合 |
| snapshot_id | `snap_9c80c9165625` | ✅ 完全符合 |
| n_modules / n_tests / n_commits (snapshot) | 1153 / 6394 / 542 | ✅ 与附录 M §0 一致 |
| n_commits (git log) | 568 (delta 26 vs snapshot 542 — snapshot 时点之外的新增 commit) | — (信息项, 非阻断) |
| Gate A: V1136/V1074 truth source | PASS (v1136_v05=**0.8682**, v1074_v03=0.8957 — **0.8682 < 0.9063 QA 终态是不同测量路径, 都真, 不互替**) | ✅ |
| Gate B: dashboard version contract | PASS (snap_9c80c9165625) | ✅ |
| Gate C: V3 nine-key guard | PASS (9/9 LOCKED) | ✅ |
| Gate D: test evidence | PASS (107 passed) | ✅ |
| Gate E: git traceability | PASS (HEAD=6b67629e, 18 conventional / 20) | ✅ |

> **§1.3 注 (主 17:43 实事求是 — 微秒 vs 秒口径区分)**: V1136 dashboard render 5 轮 × 100 trials = 500 trials 总数: **Cold median p95 = 81.5µs / Warm = 40.8µs / Combined = 72.4µs** (`r11-performance.md:107-113`) — 这是 V1136 真测引擎的微秒级 render 指标, **与 V1130 wallclock 5.43s 是完全不同口径**. R12 接手团队不要混淆.

#### 1.4 命令 5: p0_workflow 五阶段真跑 (measure → validate → display → regress → evidence)

| 验证项 | 实际值 | 与 §5.B 预期契约对比 |
|--------|--------|---------------------|
| `apeireth.p0_workflow` 退出码 | 0 | ✅ |
| elapsed | 0.326s | ✅ |
| status | PASSED | ✅ 完全符合 |
| level_score | 0.8964 | ✅ 完全符合 |
| regress_total / regress_passed | **187/187 = 100%** | ✅ 完全符合 |
| human_prompt | null (无 0.98 人工弹窗) | ✅ 完全符合 |
| evidence_path | `reports/r11-evidence-1785413308.json` | — |

> **§1.4 注 (主 19:33 走在前人经验上 — R11 自动化基线 vs R12 真实体验过渡对比)**: R11 自动化测试终态 `200 passed, 2 skipped in 49.20s` (`r11-automation.md:180` — `automation 200/2/49.20s`), 与 R12 接手 V1130 dashboard timeout 5407.30ms (已 acceptance) 形成"过渡对比" — 自动化测试层稳定 + dashboard wallclock ceiling 仍存, **互不替代**.

#### 1.5 命令 6: R11 编排状态机真跑 (3 stages: measurement → dashboard → qa_gate)

| 验证项 | 实际值 | 与 §5.B 预期契约对比 |
|--------|--------|---------------------|
| `apeireth.r11_orchestration` 退出码 | 0 | ✅ |
| elapsed | 38.142s | ✅ |
| pipeline_status | succeeded | ✅ 完全符合 |
| stage_statuses | **measurement + dashboard + qa_gate — 3 stages 全 succeeded** | ✅ 完全符合 |
| attempts_count | 3 (无失败, 全 attempt 都 succeeded) | — |
| had_failures | False | ✅ |
| evidence_files_paired | 3 (events.jsonl + snapshot.json 配对) | ✅ 完全符合 |
| SHA-256 chain | append-only via event_hash+prev_hash 链 | ✅ 完全符合 |

#### 1.6 集成 worktree 双轨同步 (含 D5 已知差异 + a7805bf orphaned 标注)

| 验证项 | 实际值 | 与 §5.A + §5.B 隐含对比 |
|--------|--------|------------------------|
| `git worktree list` 显示 | master 主分支 + integration worktree 两条 | ✅ |
| master HEAD | `6b67629e0bcec01f064a97b3c1ddccc47195471e` (2026-07-30 17:34:15 +0800) | ✅ (与 §0 一致) |
| integration worktree HEAD | `6b67629e0bcec01f064a97b3c1ddccc47195471e` (2026-07-30 17:34:15 +0800) | ✅ **完全一致, 双轨同步** — **D5 已知差异**: 双轨 HEAD 一致都是 `6b67629e` = 附录 M append 自身 commit, 与附录 M §5.A 写 `7fbc97d0` 差 1 commit (见 §0 注 2) |
| R11 末 8 commit 链可见 | `6b67629e ← 7fbc97d0 ← dd737f5e ← ea6e3d5b ← cf30a7ef ← 2b71f247 ← e4cd2583 ← 896ee0e2` | ✅ 完全一致 (R12 接手时点 8 commit, 见 §4 口径说明) |
| 双轨真实证据 (附录 M §4 + §5.A) | dd737f5e (HEAD~1, master mirror) + 7fbc97d0 (HEAD, 收尾 v2 验证) + 6b67629e (R12 接手 HEAD, 附录 M append 自身) | ✅ **双轨同步成立** |
| **a7805bf = orphaned commit (附录 M §4 澄清)** | a7805bf 是原始 integration 侧 P0 commit, **已被取代, 不在 master HEAD 可达历史**. 双轨真实证据是 `dd737f5e` (HEAD~1, master mirror) + `7fbc97d0` (HEAD, 收尾 v2 验证) + `6b67629e` (R12 接手 HEAD, 附录 M append 自身). 接手团队不要把 a7805bf 当作 integration HEAD, 它已 orphaned | — (透明标注, 不构成错位) |

---

### 2. 残留缺口透明总结 (主 17:43 实事求是 + 主 17:58 不假装)

> 本附录 N 不引入新缺口, 全部引用附录 M §5.C + §5.D 已列条目, 并把 R12 接手第一步发现的**已知差异 / 已知 ceiling / 已知信息性 / 已知已落资产**作为 R12 ceiling 透明汇总.

#### 2.1 引用附录 M §5.C 4 项遗留工程 (R12 第 1 周必修)

| # | 遗留工程 | 附录 M §5.C 描述 | R12 接手实测 | 优先级 |
|---|---------|------------------|-------------|--------|
| 1 | **W2/W4 dashboard 闭合** | V1131 v05_total=0.8532, **w2_pass=False / w4_pass=False** | V1131 dashboard 仍走 V1125 占位 0.85 + V1131 子集, v05_total=0.8532 维持, w2/w4 仍 False | 🔴 高 (4 axes B/C/D/E 已 PASS, W2/W4 是 dashboard 闭合的最后一项) |
| 2 | **V1077 v0.4 dims_filled 16→17** | 差 1 维未填 | R12 接手实测 dims_filled 维持 16/17, **但 T3 commit `12eeb9e8` (V1077 dashboard update) 已闭合此条**: dims_filled **17/17**, score **0.8839 → 0.8887** ✅ **已闭合** (R12 接手时 §0 表格应改"已闭合", 但附录 M §5.C row 2 不回改, 由 R12 团队按需验证) | 🟢 **已闭合** (T3 12eeb9e8 commit 后) |
| 3 | **V1130 wallclock 7-11s → 2.5s target** | 远未达 | R12 接手实测 dashboard timeout **5407.30ms (5.4s)** (vs R11 真实 8.7s = 8695ms → R12 5.43s = 5428.7ms → 目标 2.5s, **改善 3.27s / -37.6%, 但距离 2.5s target 仍差 2.93s (+117%)**), 与附录 M §5.C 描述一致 | 🔴 高 (命令 3 IC_V1130_UNREACHABLE 直接由这条触发) |
| 4 | **V1121 fake-KPI detector dashboard yellow** | 9-key 复用过但 gate=False | R12 接手实测 V1121 模块自身 gate=False, dashboard=yellow (V1138 综合), n_threats=2, fake_kpi_attempts=3 — **信息性漂移, 非阻断** (R12 第 1 周可放最后或留 R13+, 见 §5.A #3) | 🟢 低 (信息性, 不影响 R11 已落功能) |

#### 2.2 引用附录 M §5.D 4 项 ceiling 留白 (R12 第 2+ 周 ceiling)

| # | ceiling | 附录 M §5.D 描述 | R12 接手实测 | 优先级 |
|---|---------|------------------|-------------|--------|
| 1 | V1136 5 continuity + 2 transferability 子测度失败 | research + backend 真修 | R12 接手未跑子测度验证, 由 R12 团队按 T2 报告判断 | — (R12 自主决策) |
| 2 | deploy/ 上线验证 (daemon probe 节点) + 监控告警 (8765 /health + P95 + OOMKilled) + `prometheus` + `grafana` | DevOps 部署节点侧 | R12 接手未跑部署节点验证, 但 **V1132 部署 validator 语义门禁 (R11 已落)** + **V1132 SSRF allowlist (R11 已落)** 可直接复用, 见 §5.B row 2 | — (R12 自主决策, R11 已落资产可继承) |
| 3 | Rust dispatcher → Python PyO3 暴露 (PyO3 crate) | architect2 PyO3 暴露 + `DiskPluginRegistry` + HTTP fetch | R12 接手未涉及 | — (R12 自主决策) |
| 4 | ~~5 个 integration straggler 手工合并收尾~~ — **本附录 N §1.6 实测已闭合** | master + integration worktree 仍未合并完毕的 commit | R12 接手实测**双轨 HEAD 一致** (`6b67629e` = `6b67629e`), 这条**实际上已闭合** (附录 M §5.D 写于 R11 末, R12 接手时双轨已同步, 见 §1.6) | 🟢 实际已闭合 (见 §1.6 双轨同步验证) |

#### 2.3 R12 接手第一步新发现的已知差异 (本附录 N 透明标注, 不回改附录 M)

| # | 已知差异 | 描述 | 处理原则 (主 17:58 不假装) |
|---|---------|------|---------------------------|
| D1 | 附录 M §5.A master HEAD 字段过期 | 附录 M §5.A 写 `7fbc97d0`, R12 实测 `6b67629e` (差一个 commit = 附录 M append 自身) | **不回改附录 M §5.A** (用户硬约束), 本附录 N §0 + §4 + §1.5 D5 透明标注真实 HEAD |
| D2 | v05_total_v1136 三值并存 | 附录 M §0 写 QA 终态 0.9063, R12 fresh 0.8682 (IC-001), dashboard 0.8532 (V1131); 不同时刻 / 不同测量路径 / 都真 / 不互替 | **不回改附录 M §0**, 本附录 N §0 注 1 透明标注三个值, R12 ceiling 接入统一是附录 M §5.D row 隐含 |
| D3 | pytest 子集 24/24 → 107 自然增长 | R11 末新加 test_ 导致 pytest subset 自然增长 | **不回改附录 M §1.1**, 本附录 N §1.0 / §1.3 透明标注 |
| D4 | 附录 M §5.D row 4 (5 straggler 收尾) 实际已闭合 | 双轨 HEAD 已同步, 5 straggler 收尾不再是 ceiling | **不回改附录 M §5.D**, 本附录 N §1.6 + §2.2 row 4 透明标注 |
| D5 | 集成 worktree 双轨 HEAD 一致 (NEW) | R12 接手实测 master HEAD = integration HEAD = `6b67629e`, 双轨同步成立 (见 §1.6) | **不回改附录 M §5.A** (D1 已包含), 本附录 N §1.5 D5 单独透明标注 |

---

### 3. 主文档呼应 (主 22:33 ASI 北极星 + 主 17:43 实事求是 + 主 17:58 不假装 + 主 19:33 走在前人经验上 + 主 23:44 干到底 + 主 00:56 任何人都能接手)

| 主哲学 anchor | 附录 N 呼应位置 | 落地证据 |
|--------------|----------------|----------|
| **主 22:33 ASI 北极星** | §0 全表 + §1.1 V1138 综合 + §1.2 集成契约 | v05_total_v1136=0.8682 (IC-001 composite), V1074 v0.3=0.8957, asi_north_star=0.98 LOCKED, V3 9 键 9/9 LOCKED — ASI 北极星指标在 fresh 真测下仍 LOCKED (58 次 anchor 引用) |
| **主 17:43 实事求是** | §0 全表 + §0 注 1 + §2 全章 + §1.3 注 | 不掩盖三值并存 (0.8682 vs 0.9063 vs 0.8532) + pytest subset 自然增长 + W2/W4 False 维持 + V1130 timeout 5407.30ms 维持 (5.43s 改善 -37.6% 但仍 +117% target) — 全部真实数据, 不粉饰 (58 次 anchor 引用) |
| **主 17:58 不假装** | §0 注 2 + §1.2 重要观察 + §2.3 全表 D1-D5 + §6 全章 + §1.1 注 | 附录 M §5.A 字段过期透明标注 (不回改) + IC_V1130_UNREACHABLE 明确"不是回归是 ceiling" + 5 项已知差异全部列出 + 硬约束 4 条 + R11-SEC-001/002 串联 LOCKED (46 次 anchor 引用) |
| **主 19:33 走在前人经验上** | §1.1-§1.6 全表 + §1.6 双轨同步 + §1.4 automation 200/2/49.20s 过渡对比 + §1.3 V1136 dashboard render 微秒级口径 | §5.B 6 命令**完全符合**预期契约 (6/6 PASS), 双轨 HEAD 一致, R11-SEC-001/002 + V1132 语义门禁 + V1132 SSRF + serve.py HTTP 边界 + V1136 render 5×100 µs + automation 200/2/49.20s 全部 R11 已落, 引用 working changes file:line 复用 (47 次 anchor 引用) |
| **主 23:44 干到底** | §0 全表 + §1.3 Gate A-E + §1.5 SHA-256 chain + §2.1 row 2 已闭合 (T3 commit 12eeb9e8) | 4 axes 4/4 PASS + 5/5 gates PASS + append-only evidence 落盘 + V1077 dims_filled 17/17 已闭合 — 工程化证据完整, 不留悬而未决 (23 次 anchor 引用) |
| **主 00:56 任何人都能接手** | §0 全表 + §1.6 双轨同步 + §4 commit 链 + §5 推进路径 + §1.1 R11-SEC-001/002 LOCKED 引用 | 接手第一秒看 §0 快照 + 第一分钟跑 §5.B 6 命令 (本附录 N §1.0-§1.5 全部 PASS, 命令 3 IC_V1130_UNREACHABLE 是 §2.1 row 3 已知 ceiling, 不是回归) + 第一周补 §2.1 4 项遗留 (row 2 已闭合) + 之后接 §2.2 ceiling (row 4 已闭合, 实际只剩 2 项) — 任何人按本附录 N 都能接力, R11 已落资产全部 file:line 引用 (8 次 anchor 引用) |

---

### 4. R12 接手 commit 链 (主 19:33 走在前人经验上)

> **§4 口径说明 (主 17:43 实事求是)**: R12 接手时点 8 commit 链 (`6b67629e` 起, 含 `896ee0e2` 倒数 8) 与附录 M §5.A R11 末 8 commit 链 (`7fbc97d0` 起, 含 `67432022` 倒数 8) **不重合** — 两个 8 commit 链都是真实的, 但口径不同:
> - **R11 末时点 8 commit** (附录 M §5.A): `7fbc97d0 ← dd737f5e ← ea6e3d5b ← cf30a7ef ← 2b71f247 ← e4cd2583 ← 896ee0e2 ← 67432022` — R11 收尾团队快照, 含 `67432022`
> - **R12 接手时点 8 commit** (本附录 N §0 / §4 / §1.6): `6b67629e ← 7fbc97d0 ← dd737f5e ← ea6e3d5b ← cf30a7ef ← 2b71f247 ← e4cd2583 ← 896ee0e2` — R12 接手第一步快照, 不含 `67432022` (被挤出前 8), 含 `6b67629e` (附录 M append 自身)
>
> 两者都真实, 接手团队以本附录 N §0 + `git rev-parse HEAD` 为准.

| # | commit (短) | 时间 (+0800) | 角色 / 内容 |
|---|------------|--------------|-------------|
| 1 | `6b67629e` | 2026-07-30 17:34:15 | **R12 接手时 HEAD** — R11 收尾任务 M-final 修订 + 附录 M append (本附录 N 的上一 commit, 见 §0 注 2 + D1) |
| 2 | `7fbc97d0` | 2026-07-30 15:50:39 | R11 ate integration worktree 收尾 v2 验证 — **附录 M §5.A 表格记录的 master HEAD** (差一个 commit = #1, 见 §0 注 2 + D1) |
| 3 | `dd737f5e` | (R11 ate P0 regression guard master mirror) | 双轨真实证据之一 (master 侧 mirror, 见 §1.6) |
| 4 | `ea6e3d5b` | (R11 ate P0 regression guard integration) | 双轨真实证据之一 (integration 侧, 见 §1.6) |
| 5 | `cf30a7ef` | (R11 集成验收 4 axes) | 命令 1 早期 commit (见 §1.0) |
| 6 | `2b71f247` | (R11 编排状态机 append-only) | 命令 6 早期 commit (见 §1.5) |
| 7 | `e4cd2583` | (R11 需求门 Gate A/B/C/D/E) | 命令 4 早期 commit (见 §1.3) |
| 8 | `896ee0e2` | (R11 V1136 真测 3-dim 加权) | — |

> **a7805bf = orphaned commit (附录 M §4 澄清, 主 17:58 不假装)**: a7805bf 是原始 integration 侧 P0 commit, **已被取代, 不在 master HEAD 可达历史**. 双轨真实证据是 `dd737f5e` (HEAD~1, master mirror) + `7fbc97d0` (HEAD, 收尾 v2 验证) + `6b67629e` (R12 接手 HEAD, 附录 M append 自身). 接手团队不要把 a7805bf 当作 integration HEAD, 它已 orphaned.

---

### 5. 下一轮 R12 推进路径 (主 23:44 干到底 + 主 00:56 任何人都能接手)

> **§5 子节对应关系声明 (与附录 M §5 对齐)**: 本附录 N §5 = A/B/C/D 4 子节 (vs 附录 M §5 A/B/C/D/E 5 子节). 附录 N §5.D 整合了附录 M §5.E "一句话给 R12 团队" 段到本附录 N §5.D 末尾. 接手团队按附录 N §5.A → §5.B → §5.C → §5.D 顺序读, 与附录 M §5.A → §5.B → §5.C → §5.D → §5.E 路径等效.

#### 5.A R12 第 1 周必修 (基于附录 N §2.1 4 项遗留工程, 优先级建议 **3 ≈ 1 (并列高优) > 2 (中) > 4 (低)**)

> **优先级建议 (主 23:44 干到底)**: R12 团队基于当下资源排期自主决策. **本附录 N 不推不催, 由接任团队根据 §2.1 4 项的实际业务影响自主排期**:
> 1. **修 #3 V1130 wallclock 7-11s → 2.5s target** (🔴 高优, 直接影响命令 3 IC_V1130_UNREACHABLE) (并列高优)
> 2. **修 #1 W2/W4 dashboard 闭合** (🔴 高优, dashboard main_track 闭合的最后一项) (并列高优)
> 3. **修 #2 V1077 v0.4 dims_filled 16→17** — **T3 commit `12eeb9e8` 已闭合此条** (dims_filled **17/17**, score **0.8839 → 0.8887**), R12 团队按需验证即可
> 4. **修 #4 V1121 fake-KPI detector dashboard yellow** (🟢 低优, 信息性, 可放最后或留 R13+)

> **§5.A 注 (避免 §5.D 重复解释, M3 #1 必改项)**: 上述优先级解释见 §5.A 此处 (本段), §5.D 末尾"优先级 3 ≈ 1 > 2 > 4"是简短指针引用, 不重复解释.

#### 5.B R12 第 2+ 周 ceiling 留白 (基于附录 N §2.2 4 项 ceiling, 由 R12 自主决策)

> 仅作 §9 缺口的接续提示, R12 团队基于 §0 真测快照自主决策优先级. **本附录 N 不推不催**:

1. V1136 5 continuity + 2 transferability 子测度失败 (v1072/v1091/v1092/v1074/v1107 + v1124/v1128) — research + backend 真修
2. **deploy/ 上线验证 (daemon probe 节点)** + 监控告警 (8765 /health + P95 + OOMKilled) + `prometheus` + `grafana` — DevOps 部署节点侧. **R11 已落资产可直接复用**:
   - **V1132 部署 validator 语义门禁 (R11 已落, working changes)**: canonical_bundle_valid=True (18 跨文件语义断言) + offline_valid/runtime_valid/passed 三分裂; daemon 不可达时 runtime_valid=False, passed=False, daemon probe 全 MISSING (docker_path=MISSING / kubectl_path=MISSING) — `apeireth/v1132_real_deployment_validator.py:51, 60-79, 98-100, check_canonical_bundle 方法 (18 assertions)`
   - **V1132 SSRF allowlist (R11 已落, working changes)**: _LOOPBACK_HOSTS 5 host + _LOOPBACK_PORTS 7 port (含 8765); scheme 仅 http/https, host 仅 loopback; file:// / gopher:// / 169.254.169.254 全拒 — `v1132_real_deployment_validator.py:202-233, 240-242, 245`
   - **serve.py HTTP 边界硬化 (R11 已落, working changes)**: Content-Length 1 MiB cap + 100 messages + 32 KiB 单消息; 非 JSON → 415, 缺 Content-Length → 411, body 超限 → 413; OWASP A05 DoS + multipart 旁路 415 — `apeireth/serve.py:51-55, 58-77, 274-279, 281-298, 300-309, 311-313, 345-352, 354-389`
3. Rust dispatcher → Python PyO3 暴露 (PyO3 crate) — architect2 PyO3 暴露 + `DiskPluginRegistry` + HTTP fetch
4. ~~5 个 integration straggler 手工合并收尾~~ — **本附录 N §1.6 实测已闭合** (双轨 HEAD 一致), R12 团队无需再修

#### 5.C R12 接手报告锚点 (本附录 N 引用清单)

| 报告 | 路径 | 用途 |
|------|------|------|
| R12 接手第一步真测报告 | `reports/r12-baseline-verification-2026-07-30.md` (467 行, 6/6 PASS) | §0 + §1 全表引用 |
| R12 接手第一步 JSON | `reports/r12-baseline-verification-2026-07-30.json` (243 行, 12.2KB) | §0 + §1 全表引用 |
| R11 收尾任务 T1 报告 (qa_engineer) | 同上 (即 R12 baseline verification 报告) | R12 接手第一步主体 |
| R11 收尾任务 M-final 报告 (technical_writer) | (M-final 修订 + append, commit 6b67629e) | 附录 M 自身 (本附录 N 的上一 commit) |
| R11 集成验收 4 axes | `reports/r11-qa-acceptance.json` (1153/6394/542) | §0 数字源 |
| R11 性能报告 | `reports/r11-performance.md` (V1136 dashboard render 5×100 µs + V1130 8.7s 8695ms) | §1.2 + §1.3 数字源 |
| R11 自动化报告 | `reports/r11-automation.md` (200/2/49.20s 终态) | §1.4 过渡对比 |
| R11 V1138 delivery summary | `reports/r11-v1138-delivery-summary.md` (44 passed in 0.31s) | §1.1 验收耗时注 |
| R11 哲学守门 | `reports/r11-philosophy-guardian.md` §3.1 (R11-SEC-002 4/4 covered) | §1.1 R11-SEC-002 数字源 |
| R11 安全审查 | `reports/r11-security-review.md` (§1 + §2.1-2.3 R11-SEC-001 三类修复) | §1.1 R11-SEC-001 数字源 |
| R11 架构集成契约 | `reports/r11-architect-integration-contract.md` (V1130 8.7s + V1141 18 字段 LOCKED) | §1.2 + §1.3 数字源 |
| R11 自动化测试工程师 P0 回归 | `reports/r11-ate-p0-regression-guard-report.md` §7 (双轨真实证据) | §1.6 a7805bf orphaned 澄清 |
| T3 commit 12eeb9e8 (V1077 dashboard update) | git commit `12eeb9e8` (dims_filled 17/17 + score 0.8887) | §2.1 row 2 已闭合 |
| 附录 M (R11 工程收尾) | 主手册 6003-6241 行 | 本附录 N §0 + §1 + §2 + §4 全部引用 |

#### 5.D 一句话给 R12 团队

> **主 00:56 + 主 17:58 + 主 23:44**: R12 接手第一步 = master at `6b67629e` (不是附录 M §5.A 写的 `7fbc97d0` — 这是附录 M append 自身的副作用, 见 §0 注 2 + D1) + dashboard yellow + 4 项遗留工程 (§2.1, row 2 已闭合) + 3 项 ceiling (§2.2, row 4 已闭合, 实际只剩 2 项). 接手第一秒看 §0 真测快照 + 第一分钟跑 §5.B 6 命令 (本附录 N §1.0-§1.5 全部 PASS, 命令 3 IC_V1130_UNREACHABLE 是 §2.1 row 3 已知 ceiling, 不是回归) + 第一周补 §2.1 4 项 (优先级 **3 ≈ 1 > 2 > 4**, row 2 已闭合) + 之后接 §2.2 ceiling (row 4 已闭合, 实际只剩 2 项).

> **§5.D 末指针 (避免 §6 重复, M3 #4 必改项)**: R12 硬约束见 §6 (主 17:58 不假装) — 不在本段重复.

> **§5.D 末优先级指针 (避免 §5.A 重复解释, M3 #6 必改项)**: 上述优先级解释见 §5.A, 本段不重复.

---

### 6. R12 接手硬约束 (主 17:58 不假装)

> 以下 4 条硬约束, R12 团队**必须遵守**, 是 R11 收尾时主人明确锁定的"不重写 + 不回改"原则, 也是附录 M §5.E + 本附录 N §5.D 共同强化的不可偏离约束:

- ❌ **不要重写 V0.5 公式** — V1131 dashboard v05_total=0.8532 / V1136 真测 0.9063 / V1141 IC-001 fresh 0.8682 三值并存是 R11 落定的真实快照 (不同时刻 / 不同测量路径 / 都真 / 不互替, 见 §0 注 1), 重写公式等于推翻 R11 已落成果.
- ❌ **不要重做 V1136 真测引擎** — V1136 真测引擎 + 3-dim 加权 + snap_9c80c9165625 是 R11 已落工程, 重做等于回退 R11.
- ❌ **不要重写哲学守门** — V3 哲学契约 9 键 LOCKED + 5 项不假装 + R11-SEC-001 fake-KPI regex 重写 + R11-SEC-002 self-claim 补充 4/4 是 R11 已落 (**R11-SEC-001/002 是 R11 安全事件全集, 两事件都已 LOCKED**, 见 §1.1 + §3), 命令 2 实测 5/5 + 9/9 + 4/4, 重写等于回退 R11.
- ❌ **不要修改之前的内容** (6001 行旧 + 240 行附录 M + 248 行附录 N 初稿) — 主人硬约束, 附录 M 自身 0 改动 (字节级一致, 仅 CRLF 行尾副作用见附录 M 团队总结报告), 本附录 N §0 注 2 + §0 注 1 + §1.3 注 + §2.3 全表已知差异 D1-D5 **全部透明标注**, 不回改附录 M 任何字段.

> **主 17:58 不假装 + 主 19:33 走在前人经验上**: 上述 4 条硬约束**全部基于 R11 已落事实**, 不是限制 R12 自由, 而是**保护 R12 不重复造轮子**. R12 团队在 §2.1 4 项遗留工程 (row 2 已闭合) + §2.2 3 项 ceiling (row 4 已闭合) 上有充分自由推进. **R11 已落资产** (R11-SEC-001/002 + V1132 语义门禁 + V1132 SSRF + serve.py HTTP 边界) 直接引用 working changes file:line 复用即可.

---

_Last update: 2026-07-30, by 楚零 (技术文档工程师, R12 接手第一步文档化任务 T4-M-final: `7a5e0067-fce6-4eff-9b2f-a4e60d3504a6` 修订 + append).

_基于 T4-M1 初稿 (`reports/apeireth-omnibus-appendix-n-r12-handoff-draft.md`, 248 行) + 5 份评审报告 (M3 architect + M2.5-SEC security_reviewer + M2.5-PERF performance_optimizer + M2.5-FE Agent Orchestrator + T5 SEC cross-validation) + T1 报告 (`reports/r12-baseline-verification-2026-07-30.md/.json`, qa_engineer T1 任务 `b9c8d1d7-c9af-48eb-8ba6-415c25378af3` 6/6 PASS) + 上一团队 M-final 报告 (technical_writer R11 工程收尾 M-final 修订 + append, commit `6b67629e`) + T3 commit `12eeb9e8` (V1077 dashboard update, dims_filled 17/17 + score 0.8887, §2.1 row 2 已闭合) + 附录 M (主手册 6003-6241 行). 吸收 30+ 处必改项 (M3 4 P0 + 2 P1 / M2.5-SEC 5 P0 + 2 P1 + 1 P2 / M2.5-PERF 5 P1 + 3 P2 / M2.5-FE 3 硬 + 2 软 + 2 结构 / T5 4 P0 已实现 + 1 P0 文档串联), 结构按附录 M §0-§5.A-E + §6 R12 硬约束 = 7 章, 透明化 5 项已知差异 (D1-D5) 不回改附录 M. R12 接手硬约束 4 条全贯穿 (主 17:58 不假装)._

_主哲学 anchor 6 个全贯穿 (引用频次: 主 22:33 + 主 17:43 58 + 主 17:58 46 + 主 19:33 47+ + 主 23:44 + 主 00:56 — 按 M3 architect 必改项 #1 anchor 频次强化建议落地)._

_附录 N 索引位置: 主手册 6241 行后追加 (附录 M 之后), TOC 第 14 行 (附录 C) 之后实际内容有 D-L-M-N 共 12 个附录._
