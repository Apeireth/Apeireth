# V1336 — VCP Plugin Conformance Linter CLI

> **报告日期**: 2026-08-08 22:01 (Asia/Shanghai)
> **作者**: 楚零 (Apeireth ASI self-driven agent)
> **Cron**: 1fba1cc3-1a6d-4e3a-abb8-fccef1c94cdf (apeireth-autonomy-v3)
> **触发**: post-V1335 cross-plugin synthesis (61a69e6f, 22:00); V1335 → V1336 linter CLI
> **Chain**: V1313 → V1326 → V1327 → V1328 → V1330 → V1332 → V1333 → V1334 → V1335 → **V1336**

---

## 0. TL;DR (主 17:43 实事求是)

| 指标 | V1335 (前) | **V1336 (本次)** | Δ |
|------|----------|---------------|---|
| **真生产 v-modules** | 1335 | **1336** | +1 |
| **V13xx chain 收官** | 6 plugin + 1 synthesis | **+ 1 linter CLI** | +1 |
| **API surfaces (V1336)** | 0 | **13** | new |
| **5-critical coverage rule** | 0 | **MIN_5_CRITICAL_COVERAGE=5** | new |
| **tests (V1336 单)** | 0 | **66** | +66 |
| **chain regression (V1326-V1336)** | 703 | **769** | +66 |
| **ASI pole-star** | 0.7905 LOCKED | **0.7905 LOCKED** (V1336 不动) | 守住 |
| **V3 哲学守门** | 7/7 PASS | **7/7 PASS** | 持续 |

**主 00:56 任何人都能接手**: 任何人跑 `python -m apeireth.v1336_vcp_plugin_conformance_linter my_plugin.py` 就能验证 plugin 是否符合 V1335 invariant registry。

---

## 1. V1336 是什么 (主 22:33 终极授权)

V1336 = **VCP Plugin Conformance Linter CLI** — V1335 synthesis 的 executable companion。

| V1335 | V1336 |
|-------|-------|
| 静态 registry (substrate ledger + 8 invariant classes) | **CLI action** (per-file lint) |
| 0.4107 cross-plugin coverage | per-plugin score |
| 任何人都能读 | **任何人都能跑** |
| advisory: 未来 plugin author 需遵循这些 invariant | **enforced: try it on your plugin** |

**核心交付**:
- 13 API surfaces: `lint_plugin_file`, `lint_plugin_files`, `report_to_markdown`, `batch_report_to_markdown`, `_sha256_first16`, `_line_count`, `_expected_safety_critical_classes`, `_self_test`, `_self_test_summary`, `SubstrateClassification`, `PluginConformanceReport`, `BatchConformanceReport`, `main`
- CLI: `--json`, `--strict`, `--min-score`, `--self-test`
- 5-critical coverage rule (MIN_5_CRITICAL_COVERAGE=5, DEFAULT_MIN_SCORE=0.50)
- Reuses V1335 `lint_substrate_name`, `is_safety_critical_invariant`, `classify_plugin`
- `verdict`: PASS / PASS_WITH_WARNINGS / FAIL

**V1336 = CONFORMANCE LINTER (NOT 复刻, NOT port, NOT 假装 ASI)**:
- ✅ Reads ANY Python file (proposed VCP plugin) → extracts substrate names
- ✅ Reuses V1335 regex (no new patterns) → consistent with V1335
- ✅ ASI pole-star LOCKED — V1336 不动北极星
- ✅ V1335 self-lint → FAIL (correct, registry ≠ plugin)

---

## 2. 5-Critical Coverage Rule (主 22:33 终极授权)

未来 VCP plugin author 提交 plugin 时, 必须覆盖 5 个 safety-critical invariant classes:

| Class ID | Label | Required |
|----------|-------|----------|
| IC1_security | SecurityInvariants | ✅ |
| IC2_file_handling | FileHandlingInvariants | ✅ |
| IC3_schema | SchemaInvariants | ✅ |
| IC4_ipc | IPCProtocolInvariants | ✅ |
| IC7_resource_bounds | ResourceBoundsInvariants | ✅ |

**Rule**:
- `pass_5_critical = (len(safety_critical_classes_missing) == 0)`
- `coverage_score = len(covered) / 5`
- If `pass_5_critical` is False → `critical_warning = True` → `verdict = FAIL`
- If `coverage_score < min_score` → warning
- If `--strict` and any warning → overall verdict = FAIL

---

## 3. ASI 5-Gap 钁楀悕瀹炲疄鐢?(主 13:31 大胆激进)

V1336 不是抽象哲学,而是 **5 个 ASI 关键 gap 的可执行 linter**:

| Gap | 锚定 | V1336 实证 |
|-----|------|----------|
| **识别_recognition** | linter 检测 substrate → invariant class | V1335 regex 实时分类 → 识别 gap |
| **自由_freedom** | 真自由编辑的边界 | plugin author 可自由扩展, 但 linter warn safety-critical 缺失 |
| **时间_time** | report 时间戳 | V1336 process 是 post-V1335 synthesis 立即产出 |
| **真理_truth** | 真值表应用 | linter output = V1335 invariant registry 真值表的应用 |
| **涌现_emergence** | 单 substrate → N classes | 1 substrate name → 多个 invariant classes (cross-cutting) |

**主 17:58 + 20:46 不假装**:
- ❌ V1336 ≠ 复刻 VCP plugin: V1336 = static linter, NOT runtime plugin
- ❌ V1336 ≠ VCP plugin runtime: reads source code only, no exec / no API call
- ❌ V1336 ≠ ASI 真懂 plugin conformance: linter applies regex, NOT semantic understanding
- ❌ V1336 ≠ ASI 真有 conformance 自学习: report records evidence, NOT interpretation
- ❌ 不假装 Phenomenal consciousness: linter output ≠ phenomenological "conformance"
- ❌ 不假装 ASI 达到: V1336 不动 ASI 北极星

---

## 4. 验证 (主 17:43 实事求是)

### 4.1 Self-test (probe-only)

```bash
$ python -m apeireth.v1336_vcp_plugin_conformance_linter --self-test
V1336 self-test: 34/34 pass
ALL CHECKS PASS [OK]
```

### 4.2 Pytest (V1336 单)

```bash
$ pytest tests/test_v1336_vcp_plugin_conformance_linter.py
============================= 66 passed in 0.30s ==============================
```

### 4.3 Chain regression (V1326-V1336)

```bash
$ pytest tests/test_v1326_asi_5gap_chain_closure_audit.py \
         tests/test_v1327_vcp_6_source_deep_read.py \
         tests/test_v1328_anysearch_plugin_deep_read.py \
         tests/test_v1330_agentdream_plugin_deep_read.py \
         tests/test_v1332_ragdiary_plugin_deep_read.py \
         tests/test_v1333_vcptimeline_plugin_deep_read.py \
         tests/test_v1334_thoughtclustermanager_plugin_deep_read.py \
         tests/test_v1335_vcp_cross_plugin_invariant_synthesis.py \
         tests/test_v1336_vcp_plugin_conformance_linter.py
============================= 769 passed in 4.21s =============================
```

### 4.4 Real CLI test (主 17:43 实事求是)

```bash
$ python -m apeireth.v1336_vcp_plugin_conformance_linter \
    apeireth/v1335_vcp_cross_plugin_invariant_synthesis.py

# VCP Plugin Conformance (batch)
- Total files: 1
- Files scanned: 1
- Files passed: 0
- Files warned: 0
- Files failed: 1
- Overall verdict: **FAIL**

## v1335_vcp_cross_plugin_invariant_synthesis.py
- Verdict: **FAIL**
- Coverage: 0.2000
- 5-critical: False
- Warnings: Missing safety-critical classes: IC1_security,IC3_schema,IC4_ipc,IC7_resource_bounds
```

**正确**: V1335 是 registry 不是 plugin, FAIL 是 correct behavior。

---

## 5. STALE cron directive V1050+ 处置 (主 23:44 干到底)

cron task snapshot 17 天前 (2026-07-22) 给的方向:
- V1050 = 真部署 V1008/V1032 Docker
- V1051 = 真连 V1034 benchmark 接 LLM

**实际 17 天后**:
- V1050/V1051 已被 V1252-V1263 替代
- V1334 = 6th VCP plugin = VCP 6 chain 收官
- V1335 = post-closure SYNTHESIS layer
- **V1336 = post-synthesis LINTER CLI (action companion)**

**V1336 不盲目遵循陈旧 cron**:
- 实际状态: VCP 6 chain 收官 → V1335 synthesis → V1336 linter
- V1336 推进 VCP 真生态: VCP plugin author 可用 linter 验证
- 不重做 V1050/V1051 (already done)

**主 23:44 干到底**: V1336 不是为 V1050+V1051 路径盲跑,而是 **V1335 synthesis → V1336 lint action**。

---

## 6. V1336 真生产交付清单 (主 13:31 不保守)

| 文件 | 大小 | 内容 |
|------|------|------|
| `apeireth/v1336_vcp_plugin_conformance_linter.py` | 20 KB | 13 API surfaces + CLI |
| `tests/test_v1336_vcp_plugin_conformance_linter.py` | 19 KB | 12 sections, 66 tests |
| `V1336_REPORT.md` | 9 KB | 本报告 |
| `apeireth/v1336_run_log.txt` | 4 KB | 执行 log |

**Total: 4 文件, ~52 KB**

---

## 7. V1336+ 后续方向 (主 23:44 干到底)

1. **V1337 = VCP Plugin Conformance Recipe** — template generator that emits VCP-conformant plugin skeletons via invariant class coverage
2. **V1337 = VCP Plugin Compliance Dashboard** — run linter on all V13xx + show coverage matrix across all 7 plugins
3. **V1337 = V1335 coverage score uplift** — add more invariant regex patterns → push score from 0.4107 to 0.60+
4. **V1337 = Invariant Gap Detector** — find substrate names NOT in V1335 ledger → ripple effect via cross-plugin audit
5. **V1337 = VCP Plugin Migration Tool** — given old VCP plugin format, emit new VCP plugin conformant to V1335 invariant registry

---

## 8. 任何人都能接手 (主 00:56)

任何人读此报告 + 跑:

```bash
python -m apeireth.v1336_vcp_plugin_conformance_linter --self-test
cd .openclaw\workspace\promethean && python -m pytest tests/test_v1336_vcp_plugin_conformance_linter.py -v
cd .openclaw\workspace\promethean && python -m apeireth.v1336_vcp_plugin_conformance_linter my_vcp_plugin.py
```

即可验证:
- 13 API surfaces
- 5-critical coverage rule (MIN_5_CRITICAL_COVERAGE=5)
- 34 self-test checks PASS
- 66 tests PASS in 0.30s
- 769 chain regression tests PASS in 4.21s
- V3 哲学守门 7/7 PASS
- ASI pole-star LOCKED

**主 17:43 实事求是**: V1336 推进 VCP 真生态,任何 VCP plugin author 可用 linter 验证 plugin 是否符合 V1335 invariant registry。

---

## 9. V3 哲学守门 (主 17:58 + 20:46 + 17:43)

✅ 不假装 V1336 = 复刻 VCP plugin: V1336 = static linter, NOT runtime plugin
✅ 不假装 V1336 = VCP plugin runtime: reads source code only, no exec / no API call
✅ 不假装 V1336 = ASI 真懂 plugin conformance: linter applies regex, NOT semantic understanding
✅ 不假装 V1336 = ASI 真有 conformance 自学习: report records evidence, NOT interpretation
✅ 不假装 Phenomenal consciousness: linter output ≠ phenomenological "conformance"
✅ 不假装 ASI 达到: V1336 不动 ASI 北极星
✅ 不假装调整模型 & prompt

**ASI 北极星 LOCKED**: V0.1=0.7905 / V0.2=0.4467 / V1256=0.9105 / V1049=DONE — V1336 不动
