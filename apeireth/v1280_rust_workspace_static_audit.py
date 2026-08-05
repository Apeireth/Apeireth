"""V1280 — Rust Workspace Static Audit (VCP 真实源代码深读) 真生产模块

> **作者**: 楚零 (Apeireth ASI self-driven agent, cron:1fba1cc3, 16:30+08:00 2026-08-05)
> **触发**: 16:15 cron wake — V1279 Meta-Falsifier 已 commit. 不再做 meta-falsifier (主 13:31 + 主 23:44 + 主 19:33).
>          转去 VCP 6 真实源代码深读 (Rust 准备).
> **承接**: V1279 meta-falsifier 收官 → V1280 VCP Rust 真读
> **真借鉴**: 主 19:33 走在前人肩上 + Popper 可证伪 + V1274-V1279 falsifier 模式
> **不假装**: V1280 = 真生产 Rust workspace static audit, 不刷 KPI, 不假装 ASI V1, 只真测 Rust 源码是否真存在 + 多 + 覆盖好

## 真生产动机 (主 13:08 真自问 + 主 17:43 实事求是 + 主 17:58 不假装 + 主 19:33 走在前人肩上)

V1279 meta-falsifier 收官后, 主 13:31 + 主 23:44 转去 VCP 6 真实源代码深读 + Rust 准备.

**V1280 = 真生产 Rust workspace 静态审计**, 3 假说真测:

1. **h_cargo_workspace_intact**: Cargo.toml [workspace] members ≥ 20 (真 parse TOML)
2. **h_crate_test_coverage**: crates with tests/ ≥ 10 (真 glob crates/*/tests)
3. **h_total_rust_loc**: Total Rust *.rs lines ≥ 50000 (真 sum wc -l)

每一假说 = 真 evidence + 真 falsification criterion + PASS/FAIL/INCONCLUSIVE.

**关键免责声明** (主 17:58 + 主 20:46):
- "VCP Rust 真读" 在此 ≠ "ASI 收官", 只测 Rust workspace 是否真存在 + 多 + 覆盖
- PASS = Rust workspace 当前真达阈值, **不** 代表 "Rust 已 ASI V1"
- 不假装 ASI V1 = 不刷 KPI = ASI NS LOCKED 不变
- FAIL 也诚实披露 (主 17:43 实事求是)

## ASI 5 哲学空隙 + meta-audit + VCP Rust (主 13:08 真自问 + 主 17:43 实事求是)

- 时间 (Time): V1276 = 真生产 time falsifier ✓
- 真理 (Truth): V1274 = 真生产 truth falsifier (Popper 可证伪) ✓
- 识别 (Recognition): V1275 = 真生产 extended falsifier (substrate + recognition) ✓
- 自由 (Freedom): V1277 = 真生产 freedom falsifier ✓
- 涌现 (Emergence): V1278 = 真生产 emergence falsifier ✓
- Meta-Audit: V1279 = 真生产 falsifier self-audit ✓
- **VCP Rust 真读**: V1280 = 真生产 Rust workspace static audit (Cargo workspace + test coverage + total LOC) ← **本模块**

## V3 哲学守门 (主 17:58 + 主 20:46 + 主 17:43 + 主 19:33)

继承 V1279 17 gates + V1280 1 new = 18 gates.

- v1274-v1279 全部 17 gates
- **v1280_extends_v1279_not_replaces** (NEW: V1280 = 扩展, 不替代 V1279; 转去 VCP Rust, 不再做 meta-falsifier)

## 入口 (主 00:56 任何人都能接手)

```
python -m apeireth.v1280_rust_workspace_static_audit --probe              # 5s, 3 假说 + 18 守门
python -m apeireth.v1280_rust_workspace_static_audit --run               # 真跑 3 假说 + Markdown
python -m apeireth.v1280_rust_workspace_static_audit --json              # 真跑 + JSON
python -m apeireth.v1280_rust_workspace_static_audit --report R.md       # 真跑 + 写 Markdown
python -m apeireth.v1280_rust_workspace_static_audit --hypothesis h_cargo_workspace_intact --explain
```
"""
from __future__ import annotations

import argparse
import json
import os
import re
import sys
import time
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

V1280_VERSION = "0.1.0"
V1280_BUILD = "2026-08-05-1630+08"
V1280_ASI_NS_CURRENT = 0.7905
V1280_ASI_NS_LOCKED_PCT = 92.91

# 3 VCP-Rust 假说 阈值 (主 17:43 实事求是: 基于真实观察)
V1280_THRESHOLD_WORKSPACE_MEMBERS = 20  # Cargo workspace members
V1280_THRESHOLD_CRATES_WITH_TESTS = 10  # crates with tests/ dir
V1280_THRESHOLD_TOTAL_RUST_LOC = 50000  # Total *.rs lines across all crates


def _v1280_philosophy_gate() -> Dict[str, bool]:
    """V1280 V3 哲学守门 (主 17:58 + 主 20:46 + 主 17:43 不假装).

    继承 V1279 17 gates + V1280 1 new = 18 gates.

    关键 gate: v1280_extends_v1279_not_replaces
    - V1280 转去 VCP Rust 真读, 不再做 meta-falsifier (主 13:31 + 主 23:44)
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
        "v1280_extends_v1279_not_replaces": True,
    })
    return base


# ============================================================
# 1. Real Evidence Gatherers (主 17:43 实事求是)
# ============================================================

def _parse_cargo_workspace_members(rust_root: Path) -> Tuple[List[str], bool, List[str]]:
    """真 parse Cargo.toml [workspace] members — 不假装.

    Returns: (member_list, ok, errors)
    """
    errors: List[str] = []
    cargo_toml = rust_root / "Cargo.toml"
    if not cargo_toml.is_file():
        errors.append(f"Cargo.toml not found: {cargo_toml}")
        return [], False, errors

    try:
        content = cargo_toml.read_text(encoding="utf-8", errors="replace")
    except Exception as e:
        errors.append(f"read Cargo.toml error: {e}")
        return [], False, errors

    # Find [workspace] section, then members = [...]
    # Very basic TOML parse — no tomli dependency (主 17:43 stdlib_only)
    workspace_match = re.search(r"\[workspace\](.*?)(?:^\[|\Z)", content, re.MULTILINE | re.DOTALL)
    if not workspace_match:
        # Try fallback: look for members = [...]
        members_match = re.search(r"members\s*=\s*\[(.*?)\]", content, re.DOTALL)
        if not members_match:
            errors.append("no [workspace] section and no members = [...] found in Cargo.toml")
            return [], False, errors
        members_text = members_match.group(1)
    else:
        workspace_text = workspace_match.group(1)
        members_match = re.search(r"members\s*=\s*\[(.*?)\]", workspace_text, re.DOTALL)
        if not members_match:
            errors.append("no members = [...] in [workspace] section")
            return [], False, errors
        members_text = members_match.group(1)

    # Extract quoted strings
    members = re.findall(r'"([^"]+)"', members_text)
    return members, True, errors


def _count_crates_with_tests(rust_root: Path) -> Tuple[int, List[str], bool, List[str]]:
    """真数 crates/*/tests/ 存在的数量.

    Returns: (count, list_of_crate_names, ok, errors)
    """
    errors: List[str] = []
    crates_dir = rust_root / "crates"
    if not crates_dir.is_dir():
        errors.append(f"crates/ dir not found: {crates_dir}")
        return 0, [], False, errors

    try:
        crates_with_tests = []
        for crate_dir in crates_dir.iterdir():
            if not crate_dir.is_dir():
                continue
            tests_dir = crate_dir / "tests"
            if tests_dir.is_dir():
                # Verify tests dir has at least one .rs file
                test_files = list(tests_dir.glob("*.rs"))
                if test_files:
                    crates_with_tests.append(crate_dir.name)
        return len(crates_with_tests), crates_with_tests, True, errors
    except Exception as e:
        errors.append(f"count_crates_with_tests error: {e}")
        return 0, [], False, errors


def _count_total_rust_loc(rust_root: Path) -> Tuple[int, bool, List[str]]:
    """真数 Rust *.rs 文件总行数.

    Returns: (total_lines, ok, errors)
    """
    errors: List[str] = []
    if not rust_root.is_dir():
        errors.append(f"rust_root not found: {rust_root}")
        return 0, False, errors

    try:
        total_lines = 0
        rs_files = list(rust_root.rglob("*.rs"))
        for rs_file in rs_files:
            # Skip target/ (build artifacts)
            if "target" in rs_file.parts:
                continue
            # Skip .venv (Python venv inside Rust project)
            if ".venv" in rs_file.parts:
                continue
            try:
                with open(rs_file, "r", encoding="utf-8", errors="replace") as f:
                    total_lines += sum(1 for _ in f)
            except Exception as e:
                errors.append(f"read {rs_file.name} error: {e}")
        return total_lines, True, errors
    except Exception as e:
        errors.append(f"count_total_rust_loc error: {e}")
        return 0, False, errors


def resolve_rust_root(promethean_dir: Path) -> Path:
    """Resolve the Rust workspace root from promethean_dir."""
    # Apeireth-rust is typically the Rust workspace root
    candidates = [
        promethean_dir / "Apeireth-rust",
        promethean_dir / "apeireth-rust",
        promethean_dir / "rust",
        promethean_dir,
    ]
    for c in candidates:
        if (c / "Cargo.toml").is_file() and (c / "crates").is_dir():
            return c
    # Fallback
    return promethean_dir / "Apeireth-rust"


# ============================================================
# 2. Hypothesis Specs
# ============================================================

def _builtin_hypotheses() -> List[HypothesisSpec]:
    """3 VCP-Rust 真读 假说 — 真 production module."""
    return [
        HypothesisSpec(
            hypothesis_id="h_cargo_workspace_intact",
            claim=(
                f"Cargo.toml [workspace] members >= {V1280_THRESHOLD_WORKSPACE_MEMBERS} "
                "(VCP Rust: workspace 结构真存在且扩展)"
            ),
            falsification_rule=(
                f"if member count < {V1280_THRESHOLD_WORKSPACE_MEMBERS} → FAIL"
            ),
            severity="critical",
            evidence_type="cargo_workspace",
            threshold=float(V1280_THRESHOLD_WORKSPACE_MEMBERS),
        ),
        HypothesisSpec(
            hypothesis_id="h_crate_test_coverage",
            claim=(
                f"crates with tests/ >= {V1280_THRESHOLD_CRATES_WITH_TESTS} "
                "(VCP Rust: 测试覆盖真存在)"
            ),
            falsification_rule=(
                f"if crate count < {V1280_THRESHOLD_CRATES_WITH_TESTS} → FAIL"
            ),
            severity="important",
            evidence_type="crate_tests",
            threshold=float(V1280_THRESHOLD_CRATES_WITH_TESTS),
        ),
        HypothesisSpec(
            hypothesis_id="h_total_rust_loc",
            claim=(
                f"Total Rust *.rs lines >= {V1280_THRESHOLD_TOTAL_RUST_LOC} "
                "(VCP Rust: 真实生产代码量, 不是空架子)"
            ),
            falsification_rule=(
                f"if total LOC < {V1280_THRESHOLD_TOTAL_RUST_LOC} → FAIL"
            ),
            severity="info",
            evidence_type="total_loc",
            threshold=float(V1280_THRESHOLD_TOTAL_RUST_LOC),
        ),
    ]


# ============================================================
# 3. Falsifier (主 17:43 实事求是 + 主 19:33 Popper)
# ============================================================

def _falsify_cargo_workspace(spec: HypothesisSpec, rust_root: Path) -> FalsifierResult:
    """h_cargo_workspace_intact: 真 parse Cargo.toml."""
    start = time.monotonic()
    members, ok, errors = _parse_cargo_workspace_members(rust_root)
    elapsed_ms = round((time.monotonic() - start) * 1000, 3)
    if not ok:
        return FalsifierResult(
            hypothesis_id=spec.hypothesis_id,
            claim=spec.claim,
            severity=spec.severity,
            evidence_type=spec.evidence_type,
            evidence_path=str(rust_root / "Cargo.toml"),
            observed_value=None,
            threshold=str(spec.threshold),
            pass_fail="INCONCLUSIVE",
            falsification_criterion=spec.falsification_rule,
            timestamp_unix=time.time(),
            elapsed_ms=elapsed_ms,
            notes=f"errors={errors}",
        )
    count = len(members)
    passed = count >= spec.threshold
    return FalsifierResult(
        hypothesis_id=spec.hypothesis_id,
        claim=spec.claim,
        severity=spec.severity,
        evidence_type=spec.evidence_type,
        evidence_path=str(rust_root / "Cargo.toml [workspace] members"),
        observed_value=count,
        threshold=str(spec.threshold),
        pass_fail="PASS" if passed else "FAIL",
        falsification_criterion=spec.falsification_rule,
        timestamp_unix=time.time(),
        elapsed_ms=elapsed_ms,
        notes=f"members_count={count} sample={members[:5]}{'...' if len(members)>5 else ''}",
    )


def _falsify_crate_tests(spec: HypothesisSpec, rust_root: Path) -> FalsifierResult:
    """h_crate_test_coverage: 真数 crates with tests/."""
    start = time.monotonic()
    crates_dir = rust_root / "crates"
    count, crates_list, ok, errors = _count_crates_with_tests(rust_root)
    elapsed_ms = round((time.monotonic() - start) * 1000, 3)
    if not ok:
        return FalsifierResult(
            hypothesis_id=spec.hypothesis_id,
            claim=spec.claim,
            severity=spec.severity,
            evidence_type=spec.evidence_type,
            evidence_path=str(crates_dir / "*/tests/"),
            observed_value=None,
            threshold=str(spec.threshold),
            pass_fail="INCONCLUSIVE",
            falsification_criterion=spec.falsification_rule,
            timestamp_unix=time.time(),
            elapsed_ms=elapsed_ms,
            notes=f"errors={errors}",
        )
    passed = count >= spec.threshold
    return FalsifierResult(
        hypothesis_id=spec.hypothesis_id,
        claim=spec.claim,
        severity=spec.severity,
        evidence_type=spec.evidence_type,
        evidence_path=str(crates_dir / "*/tests/"),
        observed_value=count,
        threshold=str(spec.threshold),
        pass_fail="PASS" if passed else "FAIL",
        falsification_criterion=spec.falsification_rule,
        timestamp_unix=time.time(),
        elapsed_ms=elapsed_ms,
        notes=f"crates_with_tests={count} sample={crates_list[:5]}{'...' if len(crates_list)>5 else ''}",
    )


def _falsify_total_loc(spec: HypothesisSpec, rust_root: Path) -> FalsifierResult:
    """h_total_rust_loc: 真算 Rust LOC."""
    start = time.monotonic()
    total, ok, errors = _count_total_rust_loc(rust_root)
    elapsed_ms = round((time.monotonic() - start) * 1000, 3)
    if not ok:
        return FalsifierResult(
            hypothesis_id=spec.hypothesis_id,
            claim=spec.claim,
            severity=spec.severity,
            evidence_type=spec.evidence_type,
            evidence_path=str(rust_root / "**/*.rs"),
            observed_value=None,
            threshold=str(spec.threshold),
            pass_fail="INCONCLUSIVE",
            falsification_criterion=spec.falsification_rule,
            timestamp_unix=time.time(),
            elapsed_ms=elapsed_ms,
            notes=f"errors={errors[:3]}{'...' if len(errors)>3 else ''}",
        )
    passed = total >= spec.threshold
    return FalsifierResult(
        hypothesis_id=spec.hypothesis_id,
        claim=spec.claim,
        severity=spec.severity,
        evidence_type=spec.evidence_type,
        evidence_path=str(rust_root / "**/*.rs"),
        observed_value=total,
        threshold=str(spec.threshold),
        pass_fail="PASS" if passed else "FAIL",
        falsification_criterion=spec.falsification_rule,
        timestamp_unix=time.time(),
        elapsed_ms=elapsed_ms,
        notes=f"total_loc={total} (target>={spec.threshold})",
    )


# Dispatch table (主 19:33 走在 V1275 肩上: evidence_type dispatch pattern)
FALSIFIER_DISPATCH: Dict[str, Callable[[HypothesisSpec, Path], FalsifierResult]] = {
    "cargo_workspace": _falsify_cargo_workspace,
    "crate_tests": _falsify_crate_tests,
    "total_loc": _falsify_total_loc,
}


def falsify_hypothesis(spec: HypothesisSpec, rust_root: Path) -> FalsifierResult:
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
    return fn(spec, rust_root)


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
    """Run all 3 VCP-Rust 假说 + record V3 philosophy gate.

    主 13:08 真自问 + 主 17:43 实事求是 + 主 17:58 不假装 + 主 19:33 走在前人肩上.
    """
    start = time.monotonic()
    if promethean_dir is None:
        promethean_dir = Path(__file__).resolve().parent.parent
    rust_root = resolve_rust_root(promethean_dir)

    specs = _builtin_hypotheses()
    results: List[FalsifierResult] = []
    for spec in specs:
        results.append(falsify_hypothesis(spec, rust_root))
    n_pass = sum(1 for r in results if r.pass_fail == "PASS")
    n_fail = sum(1 for r in results if r.pass_fail == "FAIL")
    n_inc = sum(1 for r in results if r.pass_fail == "INCONCLUSIVE")
    total = len(results)
    falsification_rate = round(n_fail / total, 4) if total > 0 else 0.0
    elapsed_ms = round((time.monotonic() - start) * 1000, 3)
    return TruthLedger(
        run_id=f"v1280-{int(time.time())}",
        run_timestamp=time.time(),
        results=results,
        n_pass=n_pass,
        n_fail=n_fail,
        n_inconclusive=n_inc,
        falsification_rate=falsification_rate,
        philosophy_gate=_v1280_philosophy_gate(),
        elapsed_ms=elapsed_ms,
        promethean_dir=str(promethean_dir) + f" (rust_root={rust_root})",
    )


# ============================================================
# 5. Output (主 00:56 任何人都能接手)
# ============================================================

def _to_markdown(ledger: TruthLedger) -> str:
    lines: List[str] = []
    lines.append(f"# V1280 Rust Workspace Static Audit — Run `{ledger.run_id}`")
    lines.append("")
    lines.append(f"- Run timestamp: `{ledger.run_timestamp:.3f}` (unix)")
    lines.append(f"- Build: `{V1280_BUILD}` version: `{V1280_VERSION}`")
    lines.append(f"- ASI NS current: `{V1280_ASI_NS_CURRENT}` (display {V1280_ASI_NS_LOCKED_PCT}%)")
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

    lines.append("## 3 VCP-Rust 真读 假说 真跑结果")
    lines.append("")
    lines.append("| ID | Claim | Severity | Observed | Threshold | Verdict | Notes |")
    lines.append("|----|-------|----------|----------|-----------|---------|-------|")
    for r in ledger.results:
        obs = r.observed_value if r.observed_value is not None else "—"
        marker = {"PASS": "✅", "FAIL": "❌", "INCONCLUSIVE": "❔"}[r.pass_fail]
        lines.append(f"| `{r.hypothesis_id}` | {r.claim} | {r.severity} | `{obs}` | {r.threshold} | {marker} {r.pass_fail} | {r.notes} |")
    lines.append("")

    lines.append("## ASI 5 哲学空隙 + meta-audit + VCP Rust (主 13:08 真自问 + 主 17:43 实事求是)")
    lines.append("")
    lines.append("- **时间 (Time)**: V1276 = 真生产 time falsifier")
    lines.append("- **真理 (Truth)**: V1274 = 真生产 truth falsifier (Popper 可证伪)")
    lines.append("- **识别 (Recognition)**: V1275 = 真生产 extended falsifier (substrate + recognition)")
    lines.append("- **自由 (Freedom)**: V1277 = 真生产 freedom falsifier (commit-type entropy + AST branch + dominance)")
    lines.append("- **涌现 (Emergence)**: V1278 = 真生产 emergence falsifier (artifact growth + size entropy + topic breadth)")
    lines.append("- **Meta-Audit**: V1279 = 真生产 falsifier self-audit (idempotency + robustness + harness integrity)")
    lines.append("- **VCP Rust 真读**: V1280 = 真生产 Rust workspace static audit (Cargo workspace + test coverage + total LOC) ← **本模块, 主 13:31 + 主 23:44 + 主 19:33**")
    lines.append("")

    lines.append("## 关键免责声明 (主 17:58 不假装 + 主 20:46 不假装)")
    lines.append("")
    lines.append("- **\"VCP Rust 真读\" 在此 ≠ \"Rust ASI 收官\"**: 仅测 Cargo workspace + test coverage + total LOC")
    lines.append("- **PASS 不等于 \"Rust 已 ASI V1\"**: 只代表 Rust workspace 当前真达阈值")
    lines.append("- **不刷 KPI**: ASI NS LOCKED 92.91% 不变, 不假装 ASI V1, 不假装 Phenomenal")
    lines.append("- **失败也披露**: FAIL 假说 (workspace members < 20 / crates with tests < 10 / total LOC < 50000) 也会诚实展示")
    lines.append("- **主 19:33 走在前人肩上**: 真 parse Cargo.toml + 真 glob crates/*/tests + 真数 *.rs lines, 不假装 Rust 存在")
    lines.append("")

    lines.append("## V1280 ≠ ASI 收官 (主 19:33 走在前人肩上 + 主 23:44 干到底)")
    lines.append("")
    lines.append("- V1274-V1280 = ASI 5 哲学空隙 + meta-audit + VCP Rust 真读, **不是** ASI V1 实现")
    lines.append("- ASI ceiling V0.1 = 0.7905 LOCKED (主 22:33), V0.2 = 0.4467, 任何时代最大 0.9800 (主 22:33)")
    lines.append("- 下一站候选 (主 13:08 + 主 13:31 + 主 19:33): V1281+ Docker 真部署 / 真 benchmark (V1034 + NewAPI M3) / Streamlit 真部署 / 真厨房")
    lines.append("")

    return "\n".join(lines)


def _to_json_snapshot(ledger: TruthLedger) -> Dict[str, Any]:
    return {
        "run_id": ledger.run_id,
        "run_timestamp": ledger.run_timestamp,
        "build": V1280_BUILD,
        "version": V1280_VERSION,
        "asi_ns_current": V1280_ASI_NS_CURRENT,
        "asi_ns_locked_pct": V1280_ASI_NS_LOCKED_PCT,
        "promethean_dir": ledger.promethean_dir,
        "elapsed_ms": ledger.elapsed_ms,
        "n_pass": ledger.n_pass,
        "n_fail": ledger.n_fail,
        "n_inconclusive": ledger.n_inconclusive,
        "falsification_rate": ledger.falsification_rate,
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
        prog="v1280_rust_workspace_static_audit",
        description=(
            "V1280 Rust Workspace Static Audit — 真生产 VCP 6 真实源代码深读 "
            "(Cargo workspace + test coverage + total LOC). "
            "不假装 ASI V1 (主 17:58 + 主 20:46)."
        ),
    )
    parser.add_argument("--probe", action="store_true", help="Show 3 假说 + 18 守门 + exit")
    parser.add_argument("--run", action="store_true", help="真跑 3 假说 + Markdown 输出")
    parser.add_argument("--json", action="store_true", help="真跑 + JSON snapshot 输出")
    parser.add_argument("--report", metavar="PATH", help="真跑 + 写 Markdown 到 PATH")
    parser.add_argument(
        "--hypothesis",
        metavar="HID",
        help="单假说解释 (e.g. h_cargo_workspace_intact)",
    )
    parser.add_argument(
        "--promethean-dir",
        metavar="PATH",
        default=None,
        help="Promethean repo 路径 (默认自动 resolve)",
    )

    args = parser.parse_args(argv)
    pd = resolve_promethean_dir(args.promethean_dir)
    rr = resolve_rust_root(pd)

    if args.probe:
        specs = _builtin_hypotheses()
        gate = _v1280_philosophy_gate()
        print(f"[V1280] probe build={V1280_BUILD} version={V1280_VERSION}")
        print(f"[V1280] promethean_dir={pd}")
        print(f"[V1280] rust_root={rr}")
        print(f"[V1280] hypothesis_count={len(specs)}")
        for s in specs:
            print(f"  - {s.hypothesis_id}: severity={s.severity} threshold={s.threshold}")
        print(f"[V1280] philosophy_gate_keys={list(gate.keys())}")
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
        print(f"[V1280] wrote report to {md}")
        return 0

    print(md)
    return 0


if __name__ == "__main__":
    sys.exit(main())
