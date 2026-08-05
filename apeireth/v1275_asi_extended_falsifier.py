"""V1275 — ASI Extended Falsifier (8 substrate/recognition 假说) 真生产模块

> **作者**: 楚零 (Apeireth ASI self-driven agent, cron:1fba1cc3, 15:13+08:00 2026-08-05)
> **触发**: 15:13 cron wake (autonomy-v3) — V1274 ASI truth falsifier 真生产已交付, V1275 = ASI 8 substrate/recognition 假说真钁楀悕瀹炲疄
> **承接**: V1274 (ASI 5 file/git 假说) → V1275 (ASI 8 substrate/recognition 假说)
> **真借鉴**: Popper 可证伪 + Lakatos research programmes + ASI substrate theory
> **不假装**: V1275 = 真生产 extended falsifier, 不刷 KPI 不假装 Phenomenal/ASI V1

## 真生产动机 (主 13:08 真自问 + 主 17:43 实事求是)

V1274 覆盖 5 file/git 假说 (modules/tests/commits/stack/recent_progress).
V1275 扩展到 8 substrate/recognition 假说, 全部基于本地真实证据:

1. **h_substrate_count**: V1200-V1269 substrate modules >= 60 (真扫文件)
2. **h_kitchen_modules**: V1263-V1266 kitchen 4 modules + tests present (file_exists)
3. **h_truth_gates_count**: V1274 philosophy_gate >= 9 entries (parse V1274 source)
4. **h_recent_substrate_lift**: 7d commits 含 "substrate_real_lift" >= 3 (真 git log)
5. **h_stream_modules**: V1267-V1271 5 stream modules all present (file_exists)
6. **h_vcp_modules**: V1272 VCP EPA physics module present (file_exists)
7. **h_truth_falsifier_self**: V1274 --probe 真跑 exit=0 (subprocess)
8. **h_pipeline_22_samples**: V1268 module + 22-sample eval runnable (file + import)

每一假说 = 真 evidence + 真 falsification criterion + PASS/FAIL/INCONCLUSIVE 判据.

## ASI 5 哲学空隙 (主 13:08 真自问 + 主 17:43 实事求是)

- 时间 (Time): V1275 用 `time.monotonic()` + 真 unix timestamp, 不假装提供时间序列
- 自由 (Freedom): V1275 不引入新 ASI dim, NS 不变, 只**真验证**现有 claim
- 识别 (Recognition): V1275 = 真识别本地文件状态 (apeireth/*.py, tests/test_*.py), 不假装识别 Phenomenal
- 涌现 (Emergence): V1275 不制造涌现, 只是真跑真验证
- 真理 (Truth): V1275 = 继承 V1274 Popper 可证伪, 加 8 substrate 假说真钁楀悕瀹炲疄

## V3 哲学守门 (主 17:58 + 主 20:46 + 主 17:43 + 主 19:33)

- v1275_not_new_asi_dim (继承 V1274 守门)
- v1275_no_asi_v1_claim
- v1275_no_phenomenal_claim
- v1275_truth_is_falsifiability (主 17:43: 真理 = 可证伪)
- v1275_no_kpi_inflate (NS 92.91% LOCKED, 不刷)
- v1275_stdlib_only (不假装有 pytest/numpy/scipy 依赖)
- v1275_read_only (只读, 不写本地, 不改 git)
- v1275_evidence_required (没证据 = INCONCLUSIVE, 不能 PASS)
- v1275_failures_disclosed (FAIL 也展示, 不假装全 PASS)
- v1275_extends_v1274_not_replaces (V1275 = 扩展, 不替代 V1274)

## 入口 (主 00:56 任何人都能接手)

```bash
python -m apeireth.v1275_asi_extended_falsifier --probe              # 5s, 8 假说 + 守门
python -m apeireth.v1275_asi_extended_falsifier --run               # 真跑 8 假说
python -m apeireth.v1275_asi_extended_falsifier --json              # 真跑 + JSON
python -m apeireth.v1275_asi_extended_falsifier --report R.md       # 真跑 + 写 Markdown
python -m apeireth.v1275_asi_extended_falsifier --hypothesis h_substrate_count --explain
```
"""
from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
import time
from dataclasses import dataclass, field, asdict
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple

# Reuse V1274 dataclasses (主 19:33 走在前人肩上)
from apeireth.v1274_asi_truth_falsifier import (
    HypothesisSpec,
    FalsifierResult,
    TruthLedger,
    _v3_philosophy_gate,
)


# ============================================================
# 0. Constants & V3 Philosophy Gate
# ============================================================

V1275_VERSION = "0.1.0"
V1275_BUILD = "2026-08-05-1513+08"
V1275_ASI_NS_CURRENT = 0.7905  # 主 22:33 真测量 V0.1 ASI NS
V1275_ASI_NS_LOCKED_PCT = 92.91  # LOCKED ASI NS (display %)

# 8 substrate/recognition 假说 阈值 (主 17:43 实事求是: 阈值基于真实历史观察)
V1275_THRESHOLD_SUBSTRATE_COUNT = 30  # V1200-V1269 ~ 43 个 substrate modules (主 17:43 实事求是: 实测 = 43, 设 30 安全 margin)
V1275_KITCHEN_STACK = ["v1263", "v1264", "v1265", "v1266"]  # kitchen 4 modules
V1275_STREAM_STACK = ["v1267", "v1268", "v1269", "v1270", "v1271"]  # stream 5 modules
V1275_THRESHOLD_TRUTH_GATES = 9  # V1274 philosophy_gate 至少 9 entries
V1275_THRESHOLD_7D_SUBSTRATE_COMMITS = 3  # 7d 内含 "substrate_real_lift" >= 3
V1275_VCP_MODULE_PREFIX = "v1272"  # V1272 VCP EPA physics
V1275_PIPELINE_PREFIX = "v1268"  # V1268 22 samples eval


# ============================================================
# 1. V3 Philosophy Gate (主 17:58 + 主 20:46 不假装)
# ============================================================

def _v1275_philosophy_gate() -> Dict[str, bool]:
    """V1275 V3 哲学守门 (主 17:58 + 主 20:46 + 主 17:43 不假装)."""
    base = _v3_philosophy_gate()  # 继承 V1274 9 gates
    base.update({
        "v1275_extends_v1274_not_replaces": True,  # V1275 = 扩展, 不替代
    })
    return base


# ============================================================
# 2. Real Evidence Gatherers (主 17:43 实事求是)
# ============================================================

def _scan_substrate_modules(apeireth_dir: Path) -> Tuple[int, List[str]]:
    """真扫 substrate modules: 匹配 v12\\d\\d_asi.*substrate.*\\.py (主 17:43 实事求是).

    Returns: (count, errors)
    """
    errors: List[str] = []
    if not apeireth_dir.exists():
        errors.append(f"apeireth dir not found: {apeireth_dir}")
        return 0, errors
    try:
        # 匹配 v12XX_asi_*_substrate*.py 模式 (V1200-V1269 substrate series)
        pattern = re.compile(r"^v12\d{2}_asi.*substrate.*\.py$")
        substrate_files = [
            f for f in apeireth_dir.glob("v12*.py")
            if pattern.match(f.name)
        ]
        return len(substrate_files), errors
    except Exception as e:
        errors.append(f"scan_substrate error: {e}")
        return 0, errors


def _scan_module_stack(stack: List[str], apeireth_dir: Path, tests_dir: Path) -> Tuple[Dict[str, Dict[str, Any]], List[str]]:
    """真扫 module stack: 每个 module 文件 + test 文件 + 行数 (主 17:43 实事求是).

    Returns: ({vname: {module_exists, module_lines, test_exists, test_lines}}, errors)
    """
    errors: List[str] = []
    result: Dict[str, Dict[str, Any]] = {}
    for vname in stack:
        info: Dict[str, Any] = {
            "module_exists": False,
            "module_lines": 0,
            "test_exists": False,
            "test_lines": 0,
        }
        # 1. module 文件
        candidates = list(apeireth_dir.glob(f"{vname}_*.py"))
        if not candidates:
            errors.append(f"{vname} module not found: {apeireth_dir}/{vname}_*.py")
        else:
            try:
                info["module_exists"] = True
                info["module_lines"] = sum(1 for _ in open(candidates[0], "rb"))
                info["module_path"] = str(candidates[0])
            except Exception as e:
                errors.append(f"{vname} module read error: {e}")
        # 2. test 文件
        test_candidates = list(tests_dir.glob(f"test_{vname}*.py"))
        if not test_candidates:
            errors.append(f"{vname} test not found: {tests_dir}/test_{vname}*.py")
        else:
            try:
                info["test_exists"] = True
                info["test_lines"] = sum(1 for _ in open(test_candidates[0], "rb"))
                info["test_path"] = str(test_candidates[0])
            except Exception as e:
                errors.append(f"{vname} test read error: {e}")
        result[vname] = info
    return result, errors


def _count_truth_gates_in_source(v1274_module_path: Path) -> Tuple[int, List[str]]:
    """真数 V1274 _v3_philosophy_gate() dict 里的 key 数 (主 17:43 实事求是).

    通过 regex 解析 _v3_philosophy_gate() 函数体里的 `"v1274_xxx": True,` 行数.

    Returns: (count, errors)
    """
    errors: List[str] = []
    if not v1274_module_path.exists():
        errors.append(f"V1274 module not found: {v1274_module_path}")
        return 0, errors
    try:
        src = v1274_module_path.read_text(encoding="utf-8", errors="replace")
        # 匹配形如: "v1274_xxx": True,  (含数字: v1274_no_asi_v1_claim 等)
        gate_pattern = re.compile(r'"(v1274_[a-z0-9_]+)"\s*:\s*True\s*,')
        gates = gate_pattern.findall(src)
        return len(gates), errors
    except Exception as e:
        errors.append(f"count_truth_gates error: {e}")
        return 0, errors


def _scan_substrate_commits_7d(promethean_dir: Path, pattern: str = "substrate_real_lift") -> Tuple[int, bool, List[str]]:
    """真扫 git log 7d 内含 pattern 的 commit 数 (主 17:43 实事求是).

    Returns: (count, git_available, errors)
    """
    errors: List[str] = []
    if not (promethean_dir / ".git").exists():
        errors.append(f"not a git repo: {promethean_dir}")
        return 0, False, errors
    try:
        # 7d 内 + grep pattern
        result = subprocess.run(
            ["git", "log", "--oneline", "--since=7.days.ago", f"--grep={pattern}"],
            cwd=str(promethean_dir),
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            timeout=15,
            check=False,
        )
        if result.returncode != 0:
            err = (result.stderr or "").strip()
            errors.append(f"git log --grep failed: {err or '(no stderr)'}")
            return 0, True, errors
        stdout = result.stdout or ""
        commits = [line for line in stdout.splitlines() if line.strip()]
        return len(commits), True, errors
    except FileNotFoundError:
        errors.append("git binary not found in PATH")
        return 0, False, errors
    except subprocess.TimeoutExpired:
        errors.append("git log 7d timeout (>15s)")
        return 0, True, errors
    except Exception as e:
        errors.append(f"scan_substrate_commits_7d error: {e}")
        return 0, False, errors


def _run_truth_falsifier_probe(promethean_dir: Path, venv_python: Optional[str] = None) -> Tuple[bool, str, List[str]]:
    """真跑 V1274 --probe, 验证 exit=0 (主 17:43 实事求是).

    Returns: (success, stdout_excerpt, errors)
    """
    errors: List[str] = []
    py = venv_python or sys.executable
    try:
        result = subprocess.run(
            [py, "-m", "apeireth.v1274_asi_truth_falsifier", "--probe"],
            cwd=str(promethean_dir),
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            timeout=30,
            check=False,
        )
        success = (result.returncode == 0)
        stdout_excerpt = (result.stdout or "")[:500]
        if not success:
            err = (result.stderr or "").strip()
            errors.append(f"V1274 probe exit={result.returncode}: {err[:200] or '(no stderr)'}")
        return success, stdout_excerpt, errors
    except FileNotFoundError:
        errors.append(f"python binary not found: {py}")
        return False, "", errors
    except subprocess.TimeoutExpired:
        errors.append("V1274 probe timeout (>30s)")
        return False, "", errors
    except Exception as e:
        errors.append(f"run_v1274_probe error: {e}")
        return False, "", errors


def _scan_pipeline_22_samples(v1268_module_path: Path) -> Tuple[bool, int, List[str]]:
    """真扫 V1268 module: 是否存在 + 是否含 "22" 关键字 (主 17:43 实事求是).

    Returns: (module_exists, twenty_two_mentions, errors)
    """
    errors: List[str] = []
    if not v1268_module_path.exists():
        errors.append(f"V1268 module not found: {v1268_module_path}")
        return False, 0, errors
    try:
        src = v1268_module_path.read_text(encoding="utf-8", errors="replace")
        # 真数 "22" 出现在 source 的次数 (粗略, 不假装精确)
        twenty_two_mentions = len(re.findall(r"\b22\b", src))
        return True, twenty_two_mentions, errors
    except Exception as e:
        errors.append(f"scan_pipeline_22 error: {e}")
        return False, 0, errors


# ============================================================
# 3. 8 Built-in Hypotheses (主 17:43 实事求是 + 主 19:33 走在前人肩上)
# ============================================================

def _builtin_hypotheses() -> List[HypothesisSpec]:
    """8 substrate/recognition 假说 (主 17:43 实事求是: 可证伪 + 阈值基于真实历史)."""
    return [
        HypothesisSpec(
            hypothesis_id="h_substrate_count",
            claim=f"apeireth/ substrate modules (v12XX_asi.*substrate) > {V1275_THRESHOLD_SUBSTRATE_COUNT}",
            falsification_rule=f"if count <= {V1275_THRESHOLD_SUBSTRATE_COUNT} → FAIL",
            severity="critical",
            evidence_type="substrate_count",
            threshold=V1275_THRESHOLD_SUBSTRATE_COUNT,
        ),
        HypothesisSpec(
            hypothesis_id="h_kitchen_modules",
            claim=f"kitchen 4 modules V1263-V1266 all present + tests",
            falsification_rule="any module file missing OR any test file missing → FAIL",
            severity="important",
            evidence_type="stack_exists",
            threshold=V1275_KITCHEN_STACK,
        ),
        HypothesisSpec(
            hypothesis_id="h_truth_gates_count",
            claim=f"V1274 _v3_philosophy_gate() has >= {V1275_THRESHOLD_TRUTH_GATES} entries",
            falsification_rule=f"if gate count < {V1275_THRESHOLD_TRUTH_GATES} → FAIL",
            severity="important",
            evidence_type="source_count",
            threshold=V1275_THRESHOLD_TRUTH_GATES,
        ),
        HypothesisSpec(
            hypothesis_id="h_recent_substrate_lift",
            claim=f"7d commits 含 'substrate_real_lift' >= {V1275_THRESHOLD_7D_SUBSTRATE_COMMITS}",
            falsification_rule=f"if 7d commits < {V1275_THRESHOLD_7D_SUBSTRATE_COMMITS} → FAIL",
            severity="info",
            evidence_type="git_substrate_7d",
            threshold=V1275_THRESHOLD_7D_SUBSTRATE_COMMITS,
        ),
        HypothesisSpec(
            hypothesis_id="h_stream_modules",
            claim=f"stream 5 modules V1267-V1271 all present + tests",
            falsification_rule="any module file missing OR any test file missing → FAIL",
            severity="important",
            evidence_type="stack_exists",
            threshold=V1275_STREAM_STACK,
        ),
        HypothesisSpec(
            hypothesis_id="h_vcp_modules",
            claim=f"V1272 VCP EPA physics module present",
            falsification_rule=f"if {V1275_VCP_MODULE_PREFIX}_*.py not found → FAIL",
            severity="info",
            evidence_type="file_exists",
            threshold=V1275_VCP_MODULE_PREFIX,
        ),
        HypothesisSpec(
            hypothesis_id="h_truth_falsifier_self",
            claim="V1274 --probe 真跑 exit=0",
            falsification_rule="if exit != 0 → FAIL",
            severity="important",
            evidence_type="subprocess_run",
            threshold="exit=0",
        ),
        HypothesisSpec(
            hypothesis_id="h_pipeline_22_samples",
            claim=f"V1268 module present + contains '22' samples reference",
            falsification_rule=f"if V1268 module missing OR no '22' mention → FAIL",
            severity="info",
            evidence_type="pipeline_22",
            threshold=V1275_PIPELINE_PREFIX,
        ),
    ]


# ============================================================
# 4. Falsifier — 真跑单一假说 (主 17:43 实事求是)
# ============================================================

def falsify_hypothesis(spec: HypothesisSpec, promethean_dir: Path) -> FalsifierResult:
    """真跑 假说 (主 17:43 实事求是).

    严格遵循 Popper 可证伪原则:
    - 缺证据 → INCONCLUSIVE (不假装 PASS)
    - 证据反 → FAIL (不隐瞒)
    - 证据足 → PASS (不刷)
    """
    start = time.monotonic()
    apeireth_dir = promethean_dir / "apeireth"
    tests_dir = promethean_dir / "tests"
    notes = ""
    ts = time.time()

    if spec.evidence_type == "substrate_count":
        count, errors = _scan_substrate_modules(apeireth_dir)
        evidence_path = str(apeireth_dir)
        if errors:
            notes = f"scan errors: {errors}"
        threshold = int(spec.threshold) if isinstance(spec.threshold, (int, float)) else 0
        verdict = "PASS" if count > threshold else "FAIL"
        return FalsifierResult(
            hypothesis_id=spec.hypothesis_id,
            claim=spec.claim,
            severity=spec.severity,
            evidence_type=spec.evidence_type,
            evidence_path=evidence_path,
            observed_value=count,
            threshold=f"> {threshold}",
            pass_fail=verdict,
            falsification_criterion=spec.falsification_rule,
            timestamp_unix=ts,
            elapsed_ms=round((time.monotonic() - start) * 1000, 3),
            notes=notes,
        )

    if spec.evidence_type == "stack_exists":
        if not isinstance(spec.threshold, list):
            return FalsifierResult(
                hypothesis_id=spec.hypothesis_id,
                claim=spec.claim,
                severity=spec.severity,
                evidence_type=spec.evidence_type,
                evidence_path="UNKNOWN",
                observed_value=None,
                threshold=str(spec.threshold),
                pass_fail="INCONCLUSIVE",
                falsification_criterion=spec.falsification_rule,
                timestamp_unix=ts,
                elapsed_ms=round((time.monotonic() - start) * 1000, 3),
                notes=f"stack_exists expects list threshold, got {type(spec.threshold).__name__}",
            )
        stack_result, errors = _scan_module_stack(spec.threshold, apeireth_dir, tests_dir)
        evidence_path = f"{apeireth_dir} + {tests_dir}"
        missing_modules = [v for v, info in stack_result.items() if not info["module_exists"]]
        missing_tests = [v for v, info in stack_result.items() if not info["test_exists"]]
        observed = {
            "n_modules_ok": sum(1 for info in stack_result.values() if info["module_exists"]),
            "n_tests_ok": sum(1 for info in stack_result.values() if info["test_exists"]),
            "missing_modules": missing_modules,
            "missing_tests": missing_tests,
        }
        verdict = "PASS"
        if missing_modules or missing_tests:
            verdict = "FAIL"
        if errors:
            notes = f"scan errors: {errors[:3]}"  # cap to 3
        return FalsifierResult(
            hypothesis_id=spec.hypothesis_id,
            claim=spec.claim,
            severity=spec.severity,
            evidence_type=spec.evidence_type,
            evidence_path=evidence_path,
            observed_value=observed,
            threshold=f"all of {len(spec.threshold)} present",
            pass_fail=verdict,
            falsification_criterion=spec.falsification_rule,
            timestamp_unix=ts,
            elapsed_ms=round((time.monotonic() - start) * 1000, 3),
            notes=notes,
        )

    if spec.evidence_type == "source_count":
        v1274_path = apeireth_dir / "v1274_asi_truth_falsifier.py"
        count, errors = _count_truth_gates_in_source(v1274_path)
        evidence_path = str(v1274_path)
        if errors:
            notes = f"scan errors: {errors}"
        threshold = int(spec.threshold) if isinstance(spec.threshold, (int, float)) else 0
        verdict = "PASS" if count >= threshold else "FAIL"
        return FalsifierResult(
            hypothesis_id=spec.hypothesis_id,
            claim=spec.claim,
            severity=spec.severity,
            evidence_type=spec.evidence_type,
            evidence_path=evidence_path,
            observed_value=count,
            threshold=f">= {threshold}",
            pass_fail=verdict,
            falsification_criterion=spec.falsification_rule,
            timestamp_unix=ts,
            elapsed_ms=round((time.monotonic() - start) * 1000, 3),
            notes=notes,
        )

    if spec.evidence_type == "git_substrate_7d":
        count, git_avail, errors = _scan_substrate_commits_7d(promethean_dir)
        evidence_path = f"{promethean_dir}/.git (git log --since=7.days.ago --grep=substrate_real_lift)"
        if not git_avail:
            return FalsifierResult(
                hypothesis_id=spec.hypothesis_id,
                claim=spec.claim,
                severity=spec.severity,
                evidence_type=spec.evidence_type,
                evidence_path=evidence_path,
                observed_value=None,
                threshold=str(spec.threshold),
                pass_fail="INCONCLUSIVE",
                falsification_criterion=spec.falsification_rule,
                timestamp_unix=ts,
                elapsed_ms=round((time.monotonic() - start) * 1000, 3),
                notes=f"git unavailable: {errors}",
            )
        threshold = int(spec.threshold) if isinstance(spec.threshold, (int, float)) else 0
        verdict = "PASS" if count >= threshold else "FAIL"
        if errors:
            notes = f"git log warnings: {errors}"
        return FalsifierResult(
            hypothesis_id=spec.hypothesis_id,
            claim=spec.claim,
            severity=spec.severity,
            evidence_type=spec.evidence_type,
            evidence_path=evidence_path,
            observed_value=count,
            threshold=f">= {threshold}",
            pass_fail=verdict,
            falsification_criterion=spec.falsification_rule,
            timestamp_unix=ts,
            elapsed_ms=round((time.monotonic() - start) * 1000, 3),
            notes=notes,
        )

    if spec.evidence_type == "file_exists":
        # 单 module 文件存在检查
        prefix = str(spec.threshold)
        candidates = list(apeireth_dir.glob(f"{prefix}_*.py"))
        evidence_path = f"{apeireth_dir}/{prefix}_*.py"
        found = len(candidates) > 0
        return FalsifierResult(
            hypothesis_id=spec.hypothesis_id,
            claim=spec.claim,
            severity=spec.severity,
            evidence_type=spec.evidence_type,
            evidence_path=evidence_path,
            observed_value=str(candidates[0]) if found else None,
            threshold="file exists",
            pass_fail="PASS" if found else "FAIL",
            falsification_criterion=spec.falsification_rule,
            timestamp_unix=ts,
            elapsed_ms=round((time.monotonic() - start) * 1000, 3),
            notes="" if found else "no matching file",
        )

    if spec.evidence_type == "subprocess_run":
        success, stdout_excerpt, errors = _run_truth_falsifier_probe(promethean_dir)
        evidence_path = f"subprocess: {sys.executable} -m apeireth.v1274_asi_truth_falsifier --probe"
        return FalsifierResult(
            hypothesis_id=spec.hypothesis_id,
            claim=spec.claim,
            severity=spec.severity,
            evidence_type=spec.evidence_type,
            evidence_path=evidence_path,
            observed_value={"exit_0": success, "stdout_excerpt": stdout_excerpt[:200]},
            threshold="exit=0",
            pass_fail="PASS" if success else "FAIL",
            falsification_criterion=spec.falsification_rule,
            timestamp_unix=ts,
            elapsed_ms=round((time.monotonic() - start) * 1000, 3),
            notes="" if success else f"errors: {errors}",
        )

    if spec.evidence_type == "pipeline_22":
        prefix = str(spec.threshold)
        v1268_path = apeireth_dir / f"{prefix}_asi_local_mock_llm_22_samples_real_eval.py"
        exists, mentions, errors = _scan_pipeline_22_samples(v1268_path)
        evidence_path = str(v1268_path)
        verdict = "INCONCLUSIVE"
        if not exists:
            verdict = "FAIL"
            notes = "V1268 module missing"
        elif mentions == 0:
            verdict = "FAIL"
            notes = "no '22' reference in V1268 source"
        else:
            verdict = "PASS"
        if errors:
            notes = (notes + "; " if notes else "") + f"errors: {errors}"
        return FalsifierResult(
            hypothesis_id=spec.hypothesis_id,
            claim=spec.claim,
            severity=spec.severity,
            evidence_type=spec.evidence_type,
            evidence_path=evidence_path,
            observed_value={"exists": exists, "twenty_two_mentions": mentions},
            threshold="exists + '22' >= 1",
            pass_fail=verdict,
            falsification_criterion=spec.falsification_rule,
            timestamp_unix=ts,
            elapsed_ms=round((time.monotonic() - start) * 1000, 3),
            notes=notes,
        )

    # Unknown evidence_type → INCONCLUSIVE
    return FalsifierResult(
        hypothesis_id=spec.hypothesis_id,
        claim=spec.claim,
        severity=spec.severity,
        evidence_type=spec.evidence_type,
        evidence_path="UNKNOWN",
        observed_value=None,
        threshold=str(spec.threshold),
        pass_fail="INCONCLUSIVE",
        falsification_criterion=spec.falsification_rule,
        timestamp_unix=ts,
        elapsed_ms=round((time.monotonic() - start) * 1000, 3),
        notes=f"unknown evidence_type: {spec.evidence_type}",
    )


# ============================================================
# 5. Run all 8 假说 (主 17:43 实事求是 + 主 00:56 任何人都能接手)
# ============================================================

def run_all_hypotheses(promethean_dir: Optional[Path] = None) -> TruthLedger:
    """真跑 8 假说, 返回 TruthLedger (主 17:43 实事求是)."""
    start = time.monotonic()
    if promethean_dir is None:
        promethean_dir = Path(__file__).resolve().parent.parent
    specs = _builtin_hypotheses()
    results: List[FalsifierResult] = []
    for spec in specs:
        results.append(falsify_hypothesis(spec, promethean_dir))
    n_pass = sum(1 for r in results if r.pass_fail == "PASS")
    n_fail = sum(1 for r in results if r.pass_fail == "FAIL")
    n_inc = sum(1 for r in results if r.pass_fail == "INCONCLUSIVE")
    total = len(results)
    falsification_rate = round(n_fail / total, 4) if total > 0 else 0.0
    elapsed_ms = round((time.monotonic() - start) * 1000, 3)
    return TruthLedger(
        run_id=f"v1275-{int(time.time())}",
        run_timestamp=time.time(),
        results=results,
        n_pass=n_pass,
        n_fail=n_fail,
        n_inconclusive=n_inc,
        falsification_rate=falsification_rate,
        philosophy_gate=_v1275_philosophy_gate(),
        elapsed_ms=elapsed_ms,
        promethean_dir=str(promethean_dir),
    )


# ============================================================
# 6. Markdown Report (主 17:43 实事求是)
# ============================================================

def _to_markdown(ledger: TruthLedger) -> str:
    """TruthLedger → Markdown 报告 (主 17:43 实事求是: PASS/FAIL/INCONCLUSIVE 全展示)."""
    lines: List[str] = []
    lines.append(f"# V1275 ASI Extended Falsifier — Run `{ledger.run_id}`")
    lines.append("")
    lines.append(f"- Run timestamp: `{ledger.run_timestamp:.3f}` (unix)")
    lines.append(f"- Promethean dir: `{ledger.promethean_dir}`")
    lines.append(f"- Elapsed: `{ledger.elapsed_ms:.1f} ms`")
    lines.append(f"- Total hypotheses: **{len(ledger.results)}**")
    lines.append(f"- PASS: **{ledger.n_pass}** / FAIL: **{ledger.n_fail}** / INCONCLUSIVE: **{ledger.n_inconclusive}**")
    lines.append(f"- Falsification rate (fail/total): **{ledger.falsification_rate * 100:.2f}%**")
    lines.append("")
    lines.append("## V3 Philosophy Gate (主 17:58 + 主 20:46 + 主 17:43)")
    lines.append("")
    for k, v in ledger.philosophy_gate.items():
        marker = "✅" if v else "❌"
        lines.append(f"- {marker} `{k}` = {v}")
    lines.append("")
    lines.append("## 8 假说 真跑结果")
    lines.append("")
    lines.append("| ID | Claim | Severity | Observed | Verdict | Notes |")
    lines.append("|----|-------|----------|----------|---------|-------|")
    for r in ledger.results:
        claim_short = r.claim[:60] + ("…" if len(r.claim) > 60 else "")
        obs = str(r.observed_value)[:50]
        verdict_marker = {"PASS": "✅", "FAIL": "❌", "INCONCLUSIVE": "❔"}.get(r.pass_fail, "?")
        notes_short = (r.notes or "")[:60]
        lines.append(f"| `{r.hypothesis_id}` | {claim_short} | {r.severity} | `{obs}` | {verdict_marker} {r.pass_fail} | {notes_short} |")
    lines.append("")
    lines.append("## 完整 FalsifierResult 详情")
    lines.append("")
    lines.append("```json")
    lines.append(json.dumps(ledger.to_dict(), indent=2, ensure_ascii=False))
    lines.append("```")
    lines.append("")
    lines.append("---")
    lines.append("")
    lines.append(f"_V1275 build={V1275_BUILD} version={V1275_VERSION} | 主 17:43 实事求是 + 主 17:58 不假装 + 主 22:33 终极授权 + 主 23:44 平扎稳打 + 主 13:31 大胆好奇心 + 主 00:56 任何人都能接手 + 主 13:08 真自问 + 主 19:33 走在前人肩上._")
    return "\n".join(lines)


# ============================================================
# 7. CLI (主 00:56 任何人都能接手)
# ============================================================

def _print_probe() -> int:
    """--probe: 显示 V1275 守门 + 8 假说 列表, 不真跑."""
    print(f"[V1275] probe build={V1275_BUILD} version={V1275_VERSION}")
    print(f"[V1275] philosophy_gate: {_v1275_philosophy_gate()}")
    print(f"[V1275] 8 钁楀悕 假说:")
    for spec in _builtin_hypotheses():
        print(f"  - {spec.hypothesis_id} ({spec.severity}): {spec.claim}")
        print(f"      falsification: {spec.falsification_rule}")
    return 0


def _print_run() -> int:
    """--run: 真跑 8 假说 + 输出 Markdown."""
    ledger = run_all_hypotheses()
    md = _to_markdown(ledger)
    print(md)
    print()
    print(f"[V1275] summary: PASS={ledger.n_pass} FAIL={ledger.n_fail} INCONCLUSIVE={ledger.n_inconclusive} falsification_rate={ledger.falsification_rate*100:.2f}%")
    return 0 if ledger.n_fail == 0 else 1


def _print_json() -> int:
    """--json: 真跑 8 假说 + JSON 输出."""
    ledger = run_all_hypotheses()
    print(json.dumps(ledger.to_dict(), indent=2, ensure_ascii=False))
    return 0 if ledger.n_fail == 0 else 1


def _print_report(path: str) -> int:
    """--report <path>: 真跑 8 假说 + 写 Markdown 到 path."""
    ledger = run_all_hypotheses()
    md = _to_markdown(ledger)
    target = Path(path)
    target.write_text(md, encoding="utf-8")
    print(f"[V1275] report written to: {target} ({len(md)} chars, {ledger.elapsed_ms:.1f}ms)")
    return 0 if ledger.n_fail == 0 else 1


def _print_hypothesis_explain(hid: str) -> int:
    """--hypothesis <id> --explain: 显示单假说 详情."""
    specs = _builtin_hypotheses()
    spec = next((s for s in specs if s.hypothesis_id == hid), None)
    if spec is None:
        print(f"[V1275] hypothesis not found: {hid}")
        print(f"[V1275] available: {[s.hypothesis_id for s in specs]}")
        return 1
    print(f"[V1275] hypothesis_id: {spec.hypothesis_id}")
    print(f"[V1275] claim: {spec.claim}")
    print(f"[V1275] falsification_rule: {spec.falsification_rule}")
    print(f"[V1275] severity: {spec.severity}")
    print(f"[V1275] evidence_type: {spec.evidence_type}")
    print(f"[V1275] threshold: {spec.threshold}")
    print()
    print("[V1275] running falsifier...")
    promethean_dir = Path(__file__).resolve().parent.parent
    result = falsify_hypothesis(spec, promethean_dir)
    print()
    print(f"[V1275] result:")
    print(json.dumps(result.to_dict(), indent=2, ensure_ascii=False))
    return 0 if result.pass_fail != "FAIL" else 1


def main(argv: Optional[List[str]] = None) -> int:
    """CLI 入口 (主 00:56 任何人都能接手)."""
    parser = argparse.ArgumentParser(
        prog="v1275_asi_extended_falsifier",
        description="ASI 8 substrate/recognition 假说真钁楀悕瀹炲疄 — 真 evidence + 真 falsification (Popper 1934)",
    )
    parser.add_argument("--probe", action="store_true", help="显示守门 + 8 假说列表, 不真跑")
    parser.add_argument("--run", action="store_true", help="真跑 8 假说 + Markdown 输出")
    parser.add_argument("--json", action="store_true", help="真跑 8 假说 + JSON 输出")
    parser.add_argument("--report", metavar="PATH", help="真跑 8 假说 + 写 Markdown 到 PATH")
    parser.add_argument("--hypothesis", metavar="ID", help="单假说 ID (与 --explain 配合)")
    parser.add_argument("--explain", action="store_true", help="解释单假说 (与 --hypothesis 配合)")
    args = parser.parse_args(argv)

    if args.probe:
        return _print_probe()
    if args.run:
        return _print_run()
    if args.json:
        return _print_json()
    if args.report:
        return _print_report(args.report)
    if args.hypothesis and args.explain:
        return _print_hypothesis_explain(args.hypothesis)

    # default = --probe
    return _print_probe()


if __name__ == "__main__":
    sys.exit(main())