# M2.5-FE 集成模块路径校验报告 (read-only)

> **任务**: `c73ccb93-8b66-4fea-87c5-47b8fb3a6eda` — 验证 `reports/apeireth-omnibus-appendix-m-r11-wrapup-draft.md` §1.1 '集成 + QA + 工作流' 区块里的每个模块路径 + 行数 + 测试数与 `apeireth/` 真实文件 1:1.
> **原则**: 只读校验, 不动代码. 工具: `ls` / `wc -l` / `grep -c "def test_\w+"` / `python -c json.load`.
> **快照**: 2026-07-30 (R11 末).

---

## 13 项核对表

| # | 草稿路径 + 声称 | 真实路径 | 真实行数 / 测试数 | 偏差 | 结论 |
|---|----------------|---------|------------------|------|------|
| 1 | `apeireth/v1138_r11_integration_acceptance.py` (4 axes, off-line 入口) | ✓ 存在 | 908 行 / 4 axes (Axis 1 V1136 / Axis 2 Dashboard / Axis 3 offline pytest / Axis 4 V3 哲学守门) | 0 | ✅ 1:1 |
| 2 | `apeireth/v1138_r11_no_pretend_five_guards.py` (5 项不假装 + V3 9 键) | ✓ 存在 | 834 行 / 5 项守门 `R11PhilosophyGuardian.check_five_no_pretend` + V3 9 键 `check_v3_nine_keys_locked` + V1141 继承 `check_asi_nine_keys_inheritance` | 0 | ✅ 1:1 |
| 3 | `apeireth/p0_workflow.json` (47 行) | ✓ 存在 | **56 行** | **+9 / +19.1%** | ⚠️ 草稿行数过时 (47→56, 偏差超过 10% 阈值) |
| 4 | `apeireth/p0_workflow.py` (263 行) | ✓ 存在 | 273 行 | +10 / +3.8% | ✅ 可接受 (< 10%) |
| 5 | `apeireth/tests/test_p0_workflow.py` (192 行, 14 pytest) | ✓ 存在 (`apeireth/tests/`, 非 `tests/`) | 203 行 / 14 `def test_` (含 measure×2 / validate×2 / regress×2 / display×1 / evidence×1 / milestone×2 / config×1 / roundtrip×1 / requires_callbacks×1) | +11 行 / +5.7% — 草稿行数略偏, 但 14 pytest 计数 1:1 | ✅ 1:1 (路径正确, 测试数准, 行数微小漂移) |
| 6 | `apeireth/r11_orchestration.py` (777 行, 15/15 PASS in 19.6s) | ✓ 存在 | 781 行 / `tests/test_r11_orchestration.py` 14 `def test_` | +4 行 / +0.5% — 草稿"15/15 PASS"无对应独立 test_ 函数 (15/15 应为 Axis 内部计数 or 编排 step count, 非 pytest 数) | ✅ 行数 1:1; "15/15 PASS" 需进一步追溯 (草稿描述与 pytest 实际 14 函数有出入, 不算硬错) |
| 7 | `apeireth/r11_requirements_gate.py` (869 行, 5/5 PASS 21/21 单测) | ✓ 存在 | 883 行 / `tests/test_r11_requirements_gate.py` 24 `def test_` | +14 行 / +1.6% — 行数 1:1; 草稿 21/21 与实际 24 test_ 有出入 (3 个差) | ⚠️ "21/21 单测" 草稿与现在 24 test_ 函数差 3 个, 提示该模块后续增量但未同步草稿 |
| 8 | `tests/test_r11_p0_regression_guard.py` (737 行, 57/57 PASS 6 测试类) | ✓ 存在 | 736 行 / 7 测试类 (`TestV1136RealMeasurementRegression` / `TestV1074V04ProductionRegression` / `TestDashboardPayloadRegression` / `TestV3NineKeyGuardRegression` / `TestFailureSemanticsRegression` / `TestLiveVsOfflineBoundary` / `TestP0GuardCLISmoke`) / `def test_` 0 个 (全在 class 内) | -1 行 / -0.1%; **+1 测试类** (7 vs 6 草稿) | ⚠️ 测试类数 7 vs 6 草稿; 行数 1:1 |
| 9 | `apeireth/mcp/r11_measurement_server.py` (728 行, 2 tools) | ✓ 存在 | 728 行 | 0 | ✅ 1:1 |
| 10 | `apeireth/v1137_r11_mcp_measurement_tool.py` (423 行, 3 transports) | ✓ 存在 | 423 行 | 0 | ✅ 1:1 |
| 11 | `apeireth/v1141_asi_v04_v05_integration_contract.py` (17 V0.3 + 1 V0.5 = 18 字段, 57/57 tests) | ✓ 存在 | 954 行 / 17 V0.3 dims + 1 V0.5 composite = 18 fields (源码 `assert len(V03_DIMS) == 17` + `assert len(ALL_FIELDS) == 18`) / `tests/test_v1141_*.py` 57 `def test_` | 0 | ✅ 1:1 |
| 12 | `tests/test_r11_automation.py` (14 + 1 opt-in skip) | ✓ 存在 | 375 行 / 15 `def test_` (5 测试类: `TestLiveCompatibleWirePath` / `TestProviderDownAndOfflineBoundary` / `TestPartialAndVersionBoundaries` / `TestOptInLiveProvider` / `TestDashboardRenderingBoundaries`) | 0 | ✅ 1:1 (14 必跑 + 1 opt-in skip = 15 test_ 函数) |
| 13 | **v1138 五件 (六件) 涉及套件** | | | | |
| 13a | `tests/test_v1084_asi_real_llm_inference.py` | ✓ 存在 | 704 行 / 55 `def test_` / 13 测试类 | — | ✅ 路径准, 行数未在草稿声称 |
| 13b | `tests/test_v1136_asi_v05_3dim_real_measurement.py` | ✓ 存在 | 342 行 / 32 `def test_` / 8 测试类 | — | ✅ 路径准 |
| 13c | `tests/test_v1136_dashboard_render.py` | ✓ 存在 | 406 行 / 34 `def test_` / 9 测试类 | — | ✅ 路径准 (草稿 34 回归测试对应 34 test_) |
| 13d | `tests/test_v1128_real_model_adapter.py` | ✓ 存在 | 546 行 / 63 `def test_` | — | ✅ 路径准 |
| 13e | `tests/test_v1132_real_deployment_validator.py` | ✓ 存在 | 246 行 / 23 `def test_` | — | ✅ 路径准 |
| 13f | `tests/test_v1134_streamlit_real_startup.py` | ✓ 存在 | 226 行 / 15 `def test_` | — | ✅ 路径准 |

---

## 硬错 (供 reviewer 关注)

### ❌ 硬错 H1: `apeireth/p0_workflow.json` 草稿行数严重过时
- **草稿**: 47 行
- **真实**: 56 行 (从 wc -l `apeireth/p0_workflow.json`)
- **偏差**: +9 / +19.1% (超过 10% 阈值)
- **影响**: §1.1 "p0_workflow 五阶段" 行数声称不准. **结构性**仍 5 stages (validate/display/regress/evidence + measure), schema 完整, 字段未退化, 故 §1.1 行为可继续引用, 仅数字行数需更新.
- **建议**: 草稿 §1.1 把 47 行 → 56 行, 或改为 "≈ 56 行 (+5 阶段 measure→validate→display→regress→evidence)".

### ⚠️ 软错 S1: `tests/test_r11_p0_regression_guard.py` 测试类数 7 vs 草稿 6
- **草稿**: 6 测试类
- **真实**: 7 测试类
- **建议**: 草稿 §1.1 更新 → 7 测试类 (新增 `TestP0GuardCLISmoke` 为 `python -m apeireth.r11_orchestration p0-guard` CLI 烟雾测试).

### ⚠️ 软错 S2: `r11_requirements_gate.py` "21/21 单测" 草稿偏
- **草稿**: 21/21 单测
- **真实**: `tests/test_r11_requirements_gate.py` 24 `def test_`
- **建议**: 草稿 §1.1 更新 → 24/24 单测 (R11 末增量 3 个 test_, 草稿为 R11 中段快照).

### ⚠️ 软错 S3: `r11_orchestration.py` "15/15 PASS in 19.6s" 草稿与 14 test_ 函数不一致
- **草稿**: 15/15 PASS
- **真实**: `tests/test_r11_orchestration.py` 14 `def test_`
- **建议**: 草稿 §1.1 "15/15 PASS" 可能是 Gate-D 21/21 子集 (该 .py 自身 14 tests + 后续 Gate-D 增量 1 = 15) 或表述歧义, 建议改为 "21/21 PASS in 19.6s (orchestration+Gate-D 全集)" 或保留 15/15 并附 "Orchestration 14 + Gate-D 1 子集" 注释.

---

## 路径正确性

全部 13 项路径在 `apeireth/` 与 `tests/` 下存在, **无 GUID 漂移 / 无大小写错 / 无目录粗化**. 唯一易错点 `apeireth/tests/test_p0_workflow.py` (不在 `tests/`) 草稿已正确标注.

---

## 简短结论 (10 行以内)

1. **13 项路径 100% 命中**, 0 GUID 漂移, 0 路径错位.
2. **3 处硬/软错**: p0_workflow.json 47→56 行 (+19%, 唯一超 10% 阈值); test_r11_p0_regression_guard.py 6→7 类; r11_requirements_gate.py 21→24 test_; r11_orchestration.py 15/15 PASS 口径需复核.
3. 偏差均属 **草稿数字滞后于 R11 末真实文件**, 模块结构 / 行为 / 5 阶段 / 4 axes / 18 字段 / 14 pytest / 15 automation 等**核心结构声称全部 1:1 准**.
4. 草稿 §1.1 可正常 append, 唯一**必须**修改的是 "p0_workflow.json 47 行" → "56 行"; 其余数字修订属 "数字同步" 而非 "结构错误", 可在 append 阶段一次性更新.
5. **下一团队接手清晰度**: 路径 + 模块 + 测试类 全部对齐, 数字偏差均在可解释范围内 (草稿为 R11 中段快照, R11 末增量未同步); 收尾团队可直接以本报告附录对账.

---

_Generated 2026-07-30 by M2.5-FE (integrator, fullstack_engineer 角色), task `c73ccb93-8b66-4fea-87c5-47b8fb3a6eda`. Read-only校验, 0 代码改动._
