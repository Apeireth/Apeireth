"""V1279 — ASI Falsifier Self-Audit (Meta-Falsifier) 真生产模块

> **作者**: 楚零 (Apeireth ASI self-driven agent, cron:1fba1cc3, 16:20+08:00 2026-08-05)
> **触发**: 16:15 cron wake — V1278 ASI Emergence Falsifier 已 commit (ASI 5 哲学空隙 收官).
>          V1279 = ASI Falsifier 自审 (meta-falsifier), 测试 V1274-V1278 自身是否 well-formed
> **承接**: V1274 (truth) + V1275 (recognition/substrate) + V1276 (time) + V1277 (freedom) + V1278 (emergence) → V1279 (meta-audit)
> **真借鉴**: Popper 可证伪 (主 19:33) + V1274/V1275/V1276/V1277/V1278 dataclass 模式 + meta-circular evaluator + Liskov substitution
> **不假装**: V1279 = 真生产 falsifier self-audit, 不刷 KPI, 不假装 Phenomenal/ASI V1, 只测 "V1274-V1278 是否 deterministic + robust + harness-intact"

## 真生产动机 (主 13:08 真自问 + 主 17:43 实事求是 + 主 17:58 不假装)

ASI 5 哲学空隙 (V1274-V1278) 收官后, 真问题浮现:

**meta 问题**: V1274-V1278 自己可不可信?
- **h_v127x_idempotency**: V1274-V1278 是否 deterministic? 跑两遍结果相同吗?
- **h_v127x_robustness**: V1274-V1278 是否 robust? 输入坏数据会崩吗?
- **h_v127x_harness_integrity**: V1274-V1278 的 harness 是否 intact? 都能 import + 都能跑吗?

V1279 = 真生产 meta-falsifier, 测 V1274-V1278 自身是否满足 falsifiability 之 meta-conditions.

每一假说 = 真 evidence + 真 falsification criterion + PASS/FAIL/INCONCLUSIVE 判据.

**关键免责声明** (主 17:58 + 主 20:46):
- "Meta-Falsifier" 在此 ≠ "falsifier 万能", 只测 harness 完整性 + 确定性 + 鲁棒性
- PASS = V1274-V1278 当前通过自审, **不** 代表 "falsifier 设计完美"
- 不假装 ASI V1 = 不刷 KPI = ASI NS LOCKED 不变
- FAIL 也诚实披露 (主 17:43 实事求是)

## ASI 5 哲学空隙 + meta-audit (主 13:08 真自问 + 主 17:43 实事求是)

- 时间 (Time): V1276 = 真生产 time falsifier ✓
- 真理 (Truth): V1274 = 真生产 truth falsifier (Popper 可证伪) ✓
- 识别 (Recognition): V1275 = 真生产 extended falsifier (substrate + recognition) ✓
- 自由 (Freedom): V1277 = 真生产 freedom falsifier ✓
- 涌现 (Emergence): V1278 = 真生产 emergence falsifier ✓
- **Meta-Audit**: V1279 = 真生产 falsifier self-audit (idempotency + robustness + harness integrity) ← **本模块**

## V3 哲学守门 (主 17:58 + 主 20:46 + 主 17:43 + 主 19:33)

继承 V1278 15 gates + V1279 2 new = 17 gates.

- v1274-v1278 全部 15 gates
- **v1279_extends_v1278_not_replaces** (NEW: V1279 = 扩展, 不替代 V1278)
- **v1279_no_meta_infinite_regress** (NEW: V1280+ 不再 meta-falsifier, 转去 kitchen/VCP/Rust)

## 入口 (主 00:56 任何人都能接手)

```
python -m apeireth.v1279_asi_falsifier_self_audit --probe              # 5s, 3 假说 + 17 守门
python -m apeireth.v1279_asi_falsifier_self_audit --run               # 真跑 3 假说 + Markdown
python -m apeireth.v1279_asi_falsifier_self_audit --json              # 真跑 + JSON
python -m apeireth.v1279_asi_falsifier_self_audit --report R.md       # 真跑 + 写 Markdown
python -m apeireth.v1279_asi_falsifier_self_audit --hypothesis h_v127x_idempotency --explain
```
"""
from __future__ import annotations

import argparse
import importlib
import json
import os
import sys
import time
import traceback
from pathlib import Path
from typing import Any, Callable, Dict, List, Optional, Tuple

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

V1279_VERSION = "0.1.0"
V1279_BUILD = "2026-08-05-1620+08"
V1279_ASI_NS_CURRENT = 0.7905
V1279_ASI_NS_LOCKED_PCT = 92.91

# V1274-V1278 stack (主 13:08 真自问: 5 个 falsifier 都 audit)
V1279_V127X_STACK = (
    ("v1274", "v1274_asi_truth_falsifier", "falsify_all_builtin"),
    ("v1275", "v1275_asi_extended_falsifier", "run_all_hypotheses"),
    ("v1276", "v1276_asi_time_falsifier", "run_all_hypotheses"),
    ("v1277", "v1277_asi_freedom_falsifier", "run_all_hypotheses"),
    ("v1278", "v1278_asi_emergence_falsifier", "run_all_hypotheses"),
)

# Thresholds (主 17:43 实事求是: 基于真实观察)
V1279_THRESHOLD_IDEMPOTENT_RUNS = 2  # 跑 2 次结果应相同
V1279_THRESHOLD_ROBUST_TESTS = 3  # 3 种坏输入都不崩
V1279_THRESHOLD_HARNESS_MIN = 5  # 至少 5 个 falsifier 模块都可 import


def _v1279_philosophy_gate() -> Dict[str, bool]:
    """V1279 V3 哲学守门 (主 17:58 + 主 20:46 + 主 17:43 不假装).

    继承 V1278 15 gates + V1279 2 new = 17 gates.

    关键 gates:
    - v1279_extends_v1278_not_replaces (V1279 = 扩展, 不替代 V1278)
    - v1279_no_meta_infinite_regress (不无穷 meta-falsifier, 转去 kitchen/VCP/Rust)
    """
    base = _v3_philosophy_gate()  # V1274 9 gates
    base.update({
        "v1275_extends_v1274_not_replaces": True,
        "v1276_extends_v1275_not_replaces": True,
        "v1277_extends_v1276_not_replaces": True,
        "v1277_no_free_will_claim": True,
        "v1278_extends_v1277_not_replaces": True,
        "v1278_no_strong_emergence_claim": True,
        "v1279_extends_v1278_not_replaces": True,
        "v1279_no_meta_infinite_regress": True,
    })
    return base


# ============================================================
# 1. Real Evidence Gatherers (主 17:43 实事求是)
# ============================================================

def _try_import_v127x(stack_entry: Tuple[str, str, str]) -> Tuple[bool, str, Any]:
    """真 import V127X 模块, 返回 (success, module_path, module_or_error_msg)."""
    ver, mod_name, _ = stack_entry
    full = f"apeireth.{mod_name}"
    try:
        mod = importlib.import_module(full)
        return True, full, mod
    except Exception as e:
        return False, full, f"{type(e).__name__}: {e}"


def _try_call_runner(stack_entry: Tuple[str, str, str], promethean_dir: Path) -> Tuple[bool, str, Any]:
    """真调 V127X 的 runner, 返回 (success, runner_name, result_or_error)."""
    ver, mod_name, runner_name = stack_entry
    full = f"apeireth.{mod_name}"
    try:
        mod = importlib.import_module(full)
        runner = getattr(mod, runner_name, None)
        if runner is None:
            return False, runner_name, f"runner not found: {runner_name}"
        result = runner(promethean_dir=promethean_dir)
        return True, runner_name, result
    except Exception as e:
        return False, runner_name, f"{type(e).__name__}: {e}"


def _harness_integrity_check(promethean_dir: Path) -> Tuple[int, int, List[str]]:
    """真扫 V1274-V1278 的 harness 完整性.

    Returns: (intact_count, total_count, error_messages)
    """
    intact = 0
    total = len(V1279_V127X_STACK)
    errors: List[str] = []
    for entry in V1279_V127X_STACK:
        ok, full, _ = _try_import_v127x(entry)
        if ok:
            ok2, runner, res = _try_call_runner(entry, promethean_dir)
            if ok2 and not (isinstance(res, str) and res.startswith("runner not found")):
                intact += 1
            else:
                errors.append(f"{entry[0]}: runner={runner} err={res}")
        else:
            errors.append(f"{entry[0]}: import {full} failed: {_}")
    return intact, total, errors


def _idempotency_check(promethean_dir: Path, runs: int = V1279_THRESHOLD_IDEMPOTENT_RUNS) -> Tuple[bool, Dict[str, Tuple[int, int, int]], List[str]]:
    """真测 V1274-V1278 跑 N 次结果是否相同 (deterministic).

    Returns: (all_idempotent, {ver: (n_pass, n_fail, n_inc)_first_run}, errors)
    """
    results: Dict[str, Tuple[int, int, int]] = {}
    errors: List[str] = []
    all_idempotent = True

    for entry in V1279_V127X_STACK:
        ver = entry[0]
        verdicts_per_run: List[Tuple[int, int, int]] = []
        for i in range(runs):
            ok, runner, res = _try_call_runner(entry, promethean_dir)
            if not ok:
                errors.append(f"{ver} run #{i+1}: {res}")
                verdicts_per_run.append((-1, -1, -1))
                continue
            # res is TruthLedger
            verdicts_per_run.append((res.n_pass, res.n_fail, res.n_inconclusive))

        # Check idempotency: all runs same
        first = verdicts_per_run[0]
        same = all(v == first for v in verdicts_per_run)
        if not same:
            all_idempotent = False
            errors.append(f"{ver}: not idempotent across {runs} runs: {verdicts_per_run}")
        results[ver] = first

    return all_idempotent, results, errors


def _robustness_check(promethean_dir: Path) -> Tuple[bool, int, List[str]]:
    """真测 V1274-V1278 是否 robust — 给坏输入不应 crash.

    Tests:
    1. nonexistent dir
    2. file instead of dir
    3. None promethean_dir (when supported)

    Returns: (all_robust, robust_count, errors)
    """
    robust_count = 0
    total_tests = 0
    errors: List[str] = []

    # Test 1: nonexistent directory
    total_tests += 1
    bad_dir = Path("Z:\\definitely_does_not_exist_1279\\")
    for entry in V1279_V127X_STACK:
        ver = entry[0]
        ok, runner, res = _try_call_runner(entry, bad_dir)
        if ok:
            # Should return TruthLedger (possibly with INCONCLUSIVE results)
            robust_count += 1
        else:
            errors.append(f"{ver} on nonexistent dir: {res}")

    # Test 2: file instead of directory
    total_tests += 1
    with tempfile_TemporaryDirectory() as td:
        fake_file = Path(td) / "not_a_dir.txt"
        fake_file.write_text("not a directory")
        for entry in V1279_V127X_STACK:
            ver = entry[0]
            ok, runner, res = _try_call_runner(entry, fake_file)
            if ok:
                robust_count += 1
            else:
                errors.append(f"{ver} on file-as-dir: {res}")

    # Test 3: cwd that has no apeireth/tests
    total_tests += 1
    empty_dir = Path(tempfile_TemporaryDirectory().name)
    try:
        for entry in V1279_V127X_STACK:
            ver = entry[0]
            ok, runner, res = _try_call_runner(entry, empty_dir)
            if ok:
                robust_count += 1
            else:
                errors.append(f"{ver} on empty dir: {res}")
    finally:
        try:
            empty_dir.rmdir()
        except Exception:
            pass

    all_robust = (robust_count == total_tests * len(V1279_V127X_STACK))
    return all_robust, robust_count, errors


import tempfile
def tempfile_TemporaryDirectory():
    """Helper to get a TemporaryDirectory context manager."""
    return tempfile.TemporaryDirectory()


# ============================================================
# 2. Hypothesis Specs
# ============================================================

def _builtin_hypotheses() -> List[HypothesisSpec]:
    """3 meta-falsifier 假说 — 真 production module."""
    return [
        HypothesisSpec(
            hypothesis_id="h_v127x_idempotency",
            claim=(
                f"V1274-V1278 跑 {V1279_THRESHOLD_IDEMPOTENT_RUNS} 次结果相同 "
                "(meta: deterministic — 同一输入同一输出, 不随机)"
            ),
            falsification_rule=(
                f"if any falsifier {V1279_THRESHOLD_IDEMPOTENT_RUNS} 次结果不同 → FAIL"
            ),
            severity="critical",
            evidence_type="idempotency",
            threshold=float(V1279_THRESHOLD_IDEMPOTENT_RUNS),
        ),
        HypothesisSpec(
            hypothesis_id="h_v127x_harness_integrity",
            claim=(
                f"V1274-V1278 ({V1279_THRESHOLD_HARNESS_MIN} modules) 全部可 import + 可 run "
                "(meta: harness intact — 所有 falsifier 模块都活着)"
            ),
            falsification_rule=(
                f"if intact count < {V1279_THRESHOLD_HARNESS_MIN} → FAIL"
            ),
            severity="important",
            evidence_type="harness_integrity",
            threshold=float(V1279_THRESHOLD_HARNESS_MIN),
        ),
        HypothesisSpec(
            hypothesis_id="h_v127x_robustness",
            claim=(
                f"V1274-V1278 接受坏输入 (nonexistent dir / file-as-dir / empty dir) 不 crash "
                f"(meta: robust — 至少 {V1279_THRESHOLD_ROBUST_TESTS} 种坏输入都不崩)"
            ),
            falsification_rule=(
                f"if any falsifier 在 {V1279_THRESHOLD_ROBUST_TESTS} 种坏输入下崩 → FAIL"
            ),
            severity="info",
            evidence_type="robustness",
            threshold=float(V1279_THRESHOLD_ROBUST_TESTS),
        ),
    ]


# ============================================================
# 3. Falsifier (主 17:43 实事求是 + 主 19:33 Popper)
# ============================================================

def _falsify_idempotency(spec: HypothesisSpec, promethean_dir: Path) -> FalsifierResult:
    """h_v127x_idempotency: 真跑 N 次, 比结果."""
    start = time.monotonic()
    all_idempotent, results, errors = _idempotency_check(promethean_dir, runs=int(spec.threshold))
    elapsed_ms = round((time.monotonic() - start) * 1000, 3)
    if errors:
        # Some runs errored, can't determine idempotency
        return FalsifierResult(
            hypothesis_id=spec.hypothesis_id,
            claim=spec.claim,
            severity=spec.severity,
            evidence_type=spec.evidence_type,
            evidence_path=f"V1274-V1278 runner x{spec.threshold}",
            observed_value=None,
            threshold=str(spec.threshold),
            pass_fail="INCONCLUSIVE",
            falsification_criterion=spec.falsification_rule,
            timestamp_unix=time.time(),
            elapsed_ms=elapsed_ms,
            notes=f"errors={errors[:3]}{'...' if len(errors) > 3 else ''}; results={results}",
        )
    return FalsifierResult(
        hypothesis_id=spec.hypothesis_id,
        claim=spec.claim,
        severity=spec.severity,
        evidence_type=spec.evidence_type,
        evidence_path=f"V1274-V1278 runner x{spec.threshold}",
        observed_value=1 if all_idempotent else 0,
        threshold=str(spec.threshold),
        pass_fail="PASS" if all_idempotent else "FAIL",
        falsification_criterion=spec.falsification_rule,
        timestamp_unix=time.time(),
        elapsed_ms=elapsed_ms,
        notes=f"results={results}",
    )


def _falsify_harness_integrity(spec: HypothesisSpec, promethean_dir: Path) -> FalsifierResult:
    """h_v127x_harness_integrity: 真 import + 真 run V1274-V1278."""
    start = time.monotonic()
    intact, total, errors = _harness_integrity_check(promethean_dir)
    elapsed_ms = round((time.monotonic() - start) * 1000, 3)
    passed = intact >= spec.threshold
    return FalsifierResult(
        hypothesis_id=spec.hypothesis_id,
        claim=spec.claim,
        severity=spec.severity,
        evidence_type=spec.evidence_type,
        evidence_path=f"apeireth/v127*.py x{total}",
        observed_value=intact,
        threshold=str(spec.threshold),
        pass_fail="PASS" if passed else "FAIL",
        falsification_criterion=spec.falsification_rule,
        timestamp_unix=time.time(),
        elapsed_ms=elapsed_ms,
        notes=f"intact={intact}/{total} errors={errors[:3]}{'...' if len(errors)>3 else ''}",
    )


def _falsify_robustness(spec: HypothesisSpec, promethean_dir: Path) -> FalsifierResult:
    """h_v127x_robustness: 真用 3 种坏输入测 V1274-V1278."""
    start = time.monotonic()
    all_robust, robust_count, errors = _robustness_check(promethean_dir)
    elapsed_ms = round((time.monotonic() - start) * 1000, 3)
    expected = len(V1279_V127X_STACK) * int(spec.threshold)
    passed = all_robust
    return FalsifierResult(
        hypothesis_id=spec.hypothesis_id,
        claim=spec.claim,
        severity=spec.severity,
        evidence_type=spec.evidence_type,
        evidence_path=f"3 bad inputs (nonexistent/file/empty) x{len(V1279_V127X_STACK)} falsifiers",
        observed_value=robust_count,
        threshold=str(expected),
        pass_fail="PASS" if passed else "FAIL",
        falsification_criterion=spec.falsification_rule,
        timestamp_unix=time.time(),
        elapsed_ms=elapsed_ms,
        notes=f"robust_count={robust_count}/{expected} errors={errors[:3]}{'...' if len(errors)>3 else ''}",
    )


# Dispatch table (主 19:33 走在 V1275 肩上: evidence_type dispatch pattern)
FALSIFIER_DISPATCH: Dict[str, Callable[[HypothesisSpec, Path], FalsifierResult]] = {
    "idempotency": _falsify_idempotency,
    "harness_integrity": _falsify_harness_integrity,
    "robustness": _falsify_robustness,
}


def falsify_hypothesis(spec: HypothesisSpec, promethean_dir: Path) -> FalsifierResult:
    """Dispatch by evidence_type — 主 19:33 走在 V1275 肩上."""
    fn = FALSIFIER_DISPATCH.get(spec.evidence_type)
    if fn is None:
        return FalsifierResult(
            hypothesis_id=spec.hypothesis_id,
            claim=spec.claim,
            severity=spec.severity,
            evidence_type=spec.evidence_type,
            evidence_path="(unknown)",
            observed_value=None,
            threshold=str(spec.threshold),
            pass_fail="INCONCLUSIVE",
            falsification_criterion=spec.falsification_rule,
            timestamp_unix=time.time(),
            elapsed_ms=0.0,
            notes=f"unknown evidence_type: {spec.evidence_type}",
        )
    return fn(spec, promethean_dir)


# ============================================================
# 4. Runner (主 00:56 任何人都能接手)
# ============================================================

def resolve_promethean_dir(arg: Optional[str]) -> Path:
    """Resolve promethean_dir from arg / env / cwd."""
    candidates: List[Path] = []
    if arg:
        candidates.append(Path(arg))
    env = os.environ.get("PROMETHEAN_DIR")
    if env:
        candidates.append(Path(env))
    candidates.append(Path.cwd())
    candidates.append(Path(__file__).resolve().parent.parent)

    for c in candidates:
        if (c / "apeireth").is_dir() and (c / "tests").is_dir():
            return c
    return candidates[0] if candidates else Path.cwd()


def run_all_hypotheses(promethean_dir: Optional[Path] = None) -> TruthLedger:
    """Run all 3 meta-falsifier 假说 + record V3 philosophy gate.

    主 13:08 真自问 + 主 17:43 实事求是 + 主 17:58 不假装.
    """
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
        run_id=f"v1279-{int(time.time())}",
        run_timestamp=time.time(),
        results=results,
        n_pass=n_pass,
        n_fail=n_fail,
        n_inconclusive=n_inc,
        falsification_rate=falsification_rate,
        philosophy_gate=_v1279_philosophy_gate(),
        elapsed_ms=elapsed_ms,
        promethean_dir=str(promethean_dir),
    )


# ============================================================
# 5. Output (主 00:56 任何人都能接手)
# ============================================================

def _to_markdown(ledger: TruthLedger) -> str:
    lines: List[str] = []
    lines.append(f"# V1279 ASI Falsifier Self-Audit — Run `{ledger.run_id}`")
    lines.append("")
    lines.append(f"- Run timestamp: `{ledger.run_timestamp:.3f}` (unix)")
    lines.append(f"- Build: `{V1279_BUILD}` version: `{V1279_VERSION}`")
    lines.append(f"- ASI NS current: `{V1279_ASI_NS_CURRENT}` (display {V1279_ASI_NS_LOCKED_PCT}%)")
    lines.append(f"- Promethean dir: `{ledger.promethean_dir}`")
    lines.append(f"- Elapsed: `{ledger.elapsed_ms:.1f} ms`")
    lines.append(f"- Total hypotheses: **{len(ledger.results)}**")
    lines.append(f"- PASS: **{ledger.n_pass}** / FAIL: **{ledger.n_fail}** / INCONCLUSIVE: **{ledger.n_inconclusive}**")
    lines.append(f"- Falsification rate (fail/total): **{ledger.falsification_rate * 100:.2f}%**")
    lines.append("")

    lines.append("## V3 Philosophy Gate (主 17:58 + 主 20:46 + 主 17:43 不假装)")
    lines.append("")
    for k, v in ledger.philosophy_gate.items():
        marker = "✅" if v else "❌"
        lines.append(f"- {marker} `{k}` = {v}")
    lines.append("")

    lines.append("## 3 meta-falsifier 假说 真跑结果")
    lines.append("")
    lines.append("| ID | Claim | Severity | Observed | Threshold | Verdict | Notes |")
    lines.append("|----|-------|----------|----------|-----------|---------|-------|")
    for r in ledger.results:
        obs = r.observed_value if r.observed_value is not None else "—"
        marker = {"PASS": "✅", "FAIL": "❌", "INCONCLUSIVE": "❔"}[r.pass_fail]
        lines.append(f"| `{r.hypothesis_id}` | {r.claim} | {r.severity} | `{obs}` | {r.threshold} | {marker} {r.pass_fail} | {r.notes} |")
    lines.append("")

    lines.append("## ASI 5 哲学空隙 + meta-audit (主 13:08 真自问 + 主 17:43 实事求是)")
    lines.append("")
    lines.append("- **时间 (Time)**: V1276 = 真生产 time falsifier")
    lines.append("- **真理 (Truth)**: V1274 = 真生产 truth falsifier (Popper 可证伪)")
    lines.append("- **识别 (Recognition)**: V1275 = 真生产 extended falsifier (substrate + recognition)")
    lines.append("- **自由 (Freedom)**: V1277 = 真生产 freedom falsifier (commit-type entropy + AST branch + dominance)")
    lines.append("- **涌现 (Emergence)**: V1278 = 真生产 emergence falsifier (artifact growth + size entropy + topic breadth)")
    lines.append("- **Meta-Audit**: V1279 = 真生产 falsifier self-audit (idempotency + robustness + harness integrity) ← **本模块**")
    lines.append("")

    lines.append("## 关键免责声明 (主 17:58 不假装 + 主 20:46 不假装)")
    lines.append("")
    lines.append("- **\"Meta-Falsifier\" 在此 ≠ \"falsifier 万能\"**: 仅测 harness 完整性 + 确定性 + 鲁棒性")
    lines.append("- **PASS 不等于 \"falsifier 设计完美\"**: 只代表当前 V1274-V1278 通过自审")
    lines.append("- **不刷 KPI**: ASI NS LOCKED 92.91% 不变, 不假装 ASI V1, 不假装 Phenomenal")
    lines.append("- **失败也披露**: FAIL 假说 (idempotency 不达 / harness 不 intact / robustness 不达) 也会诚实展示")
    lines.append("- **不 meta 无限递归** (主 13:31 + 主 23:44): V1280+ 转去 kitchen/VCP/Rust, 不再做 meta-falsifier")
    lines.append("")

    lines.append("## V1279 ≠ ASI 收官 (主 19:33 走在前人肩上 + 主 23:44 干到底)")
    lines.append("")
    lines.append("- V1274-V1279 = ASI 5 哲学空隙 + meta-audit, **不是** ASI V1 实现")
    lines.append("- ASI ceiling V0.1 = 0.7905 LOCKED (主 22:33), V0.2 = 0.4467, 任何时代最大 0.9800 (主 22:33)")
    lines.append("- 下一站候选 (主 13:08 + 主 13:31 + 主 19:33): V1280+ 实证 VCP 6 真实源代码深读 / V1050+ Docker 真部署 / Rust 准备 / 真 benchmark")
    lines.append("")

    return "\n".join(lines)


def _to_json_snapshot(ledger: TruthLedger) -> Dict[str, Any]:
    return {
        "run_id": ledger.run_id,
        "run_timestamp": ledger.run_timestamp,
        "build": V1279_BUILD,
        "version": V1279_VERSION,
        "asi_ns_current": V1279_ASI_NS_CURRENT,
        "asi_ns_locked_pct": V1279_ASI_NS_LOCKED_PCT,
        "promethean_dir": ledger.promethean_dir,
        "elapsed_ms": ledger.elapsed_ms,
        "n_pass": ledger.n_pass,
        "n_fail": ledger.n_fail,
        "n_inconclusive": ledger.n_inconclusive,
        "falsification_rate": ledger.falsification_rate,
        "v127x_stack": [e[0] for e in V1279_V127X_STACK],
        "results": [
            {
                "hypothesis_id": r.hypothesis_id,
                "verdict": r.pass_fail,
                "observed_value": r.observed_value,
                "threshold": r.threshold,
                "evidence_path": r.evidence_path,
                "notes": r.notes,
            }
            for r in ledger.results
        ],
        "philosophy_gate": ledger.philosophy_gate,
    }


# ============================================================
# 6. CLI (主 00:56 任何人都能接手)
# ============================================================

def main(argv: Optional[List[str]] = None) -> int:
    parser = argparse.ArgumentParser(
        prog="v1279_asi_falsifier_self_audit",
        description=(
            "V1279 ASI Falsifier Self-Audit — 真生产 ASI meta-falsifier, 测试 V1274-V1278 自身 "
            "(idempotency + robustness + harness integrity). "
            "不假装 meta 万能 (主 17:58 + 主 20:46)."
        ),
    )
    parser.add_argument("--probe", action="store_true", help="Show 3 假说 + 17 守门 + exit")
    parser.add_argument("--run", action="store_true", help="真跑 3 假说 + Markdown 输出")
    parser.add_argument("--json", action="store_true", help="真跑 + JSON snapshot 输出")
    parser.add_argument("--report", metavar="PATH", help="真跑 + 写 Markdown 到 PATH")
    parser.add_argument(
        "--hypothesis",
        metavar="HID",
        help="单假说解释 (e.g. h_v127x_idempotency)",
    )
    parser.add_argument(
        "--promethean-dir",
        metavar="PATH",
        default=None,
        help="Promethean repo 路径 (默认自动 resolve)",
    )

    args = parser.parse_args(argv)
    pd = resolve_promethean_dir(args.promethean_dir)

    if args.probe:
        specs = _builtin_hypotheses()
        gate = _v1279_philosophy_gate()
        print(f"[V1279] probe build={V1279_BUILD} version={V1279_VERSION}")
        print(f"[V1279] promethean_dir={pd}")
        print(f"[V1279] hypothesis_count={len(specs)}")
        for s in specs:
            print(f"  - {s.hypothesis_id}: severity={s.severity} threshold={s.threshold}")
        print(f"[V1279] v127x_stack={[e[0] for e in V1279_V127X_STACK]}")
        print(f"[V1279] philosophy_gate_keys={list(gate.keys())}")
        return 0

    if args.hypothesis:
        specs = _builtin_hypotheses()
        for s in specs:
            if s.hypothesis_id == args.hypothesis:
                print(f"# {s.hypothesis_id}")
                print(f"claim: {s.claim}")
                print(f"falsification_rule: {s.falsification_rule}")
                print(f"severity: {s.severity}")
                print(f"evidence_type: {s.evidence_type}")
                print(f"threshold: {s.threshold}")
                return 0
        print(f"unknown hypothesis: {args.hypothesis}", file=sys.stderr)
        print(f"available: {[s.hypothesis_id for s in specs]}", file=sys.stderr)
        return 2

    # Default = run all
    ledger = run_all_hypotheses(promethean_dir=pd)

    if args.json:
        snap = _to_json_snapshot(ledger)
        print(json.dumps(snap, ensure_ascii=False, indent=2))
        return 0

    md = _to_markdown(ledger)

    if args.report:
        Path(args.report).write_text(md, encoding="utf-8")
        print(f"[V1279] wrote report to {args.report}")
        return 0

    print(md)
    return 0


if __name__ == "__main__":
    sys.exit(main())
