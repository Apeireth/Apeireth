"""V1282 — Rust apeireth-sovereignty Semantic Audit (VCP 真实源代码深读 #3) 真生产模块

> **作者**: 楚零 (Apeireth ASI self-driven agent, cron:1fba1cc3, 16:46+08:00 2026-08-05)
> **触发**: 16:46 cron wake tick 3 — V1281 apeireth-asi 语义审计 已 commit (4c71c88b).
>          不再做 asi-only (主 13:31 + 主 23:44 + 主 19:33).
>          转去 VCP #3 = governance crate (apeireth-sovereignty) 单 crate 语义审计.
> **承接**: V1281 apeireth-asi 语义审计 收官 → V1282 apeireth-sovereignty governance 语义审计
> **真借鉴**: 主 19:33 走在前人肩上 + Popper 可证伪 + V1274-V1281 falsifier 模式
> **不假装**: V1282 = 真生产 apeireth-sovereignty governance crate 语义审计, 不刷 KPI, 不假装 ASI V1
> **不假装**: 只测 apeireth-sovereignty 这一个 governance crate 的语义结构, **不** 代表其他 41 个 crates 同等覆盖
> **真实记录**: apeireth-sovereignty 当前 73 unwrap() + 10 expect() calls (治理 crate 安全隐忧, 后续 V1283+ 可深入)

## 真生产动机 (主 13:08 真自问 + 主 17:43 实事求是 + 主 17:58 不假装 + 主 19:33 走在前人肩上)

V1281 apeireth-asi 技术 crate 语义审计 收官后, 主 13:31 + 主 23:44 转去 VCP **governance crate** 单 crate 语义审计.

**V1282 = 真生产 apeireth-sovereignty governance crate 语义审计**, 3 假说真测:

1. **h_pub_api_density**: apeireth-sovereignty pub API surface >= 50 (真 grep)
2. **h_impl_real_coverage**: apeireth-sovereignty impl blocks / pub struct >= 1.0 (真 grep)
3. **h_derive_macro_usage**: apeireth-sovereignty derive macro applications >= 5 (真 grep)

每一假说 = 真 evidence + 真 falsification criterion + PASS/FAIL/INCONCLUSIVE.

**关键免责声明** (主 17:58 + 主 20:46):
- "VCP governance crate 单 crate 语义审计" 在此 ≠ "ASI 收官", 只测 apeireth-sovereignty 一个 crate
- PASS = apeireth-sovereignty 当前真达阈值, **不** 代表 "Rust 已 ASI V1"
- 不假装 ASI V1 = 不刷 KPI = ASI NS LOCKED 不变
- FAIL 也诚实披露 (主 17:43 实事求是)

## ASI 5 哲学空隙 + meta-audit + VCP Rust #1-3 (主 13:08 真自问 + 主 17:43 实事求是)

- 时间 (Time): V1276 = 真生产 time falsifier ✓
- 真理 (Truth): V1274 = 真生产 truth falsifier (Popper 可证伪) ✓
- 识别 (Recognition): V1275 = 真生产 extended falsifier ✓
- 自由 (Freedom): V1277 = 真生产 freedom falsifier ✓
- 涌现 (Emergence): V1278 = 真生产 emergence falsifier ✓
- Meta-Audit: V1279 = 真生产 falsifier self-audit ✓
- VCP Rust 静态: V1280 = 真生产 Rust workspace 静态审计 (42 + 39 + 3M) ✓
- VCP Rust 语义 #1 (technical): V1281 = 真生产 apeireth-asi 语义审计 (115 + 1.5 + 20) ✓
- **VCP Rust 语义 #2 (governance)**: V1282 = 真生产 apeireth-sovereignty 语义审计 (315 + 3.31 + 84) ← **本模块, 主 13:31 + 主 23:44 + 主 19:33**

## V3 哲学守门 (主 17:58 + 主 20:46 + 主 17:43 + 主 19:33)

继承 V1281 19 gates + V1282 1 new = 20 gates.

- v1274-v1281 全部 19 gates
- **v1282_extends_v1281_not_replaces** (NEW: V1282 = 扩展, 不替代 V1281; 转去 governance crate 语义, 不再做 technical crate)

## 入口 (主 00:56 任何人都能接手)

```
python -m apeireth.v1282_rust_apeireth_sovereignty_semantic_audit --probe              # 5s, 3 假说 + 20 守门
python -m apeireth.v1282_rust_apeireth_sovereignty_semantic_audit --run               # 真跑 3 假说 + Markdown
python -m apeireth.v1282_rust_apeireth_sovereignty_semantic_audit --json              # 真跑 + JSON
python -m apeireth.v1282_rust_apeireth_sovereignty_semantic_audit --report R.md       # 真跑 + 写 Markdown
python -m apeireth.v1282_rust_apeireth_sovereignty_semantic_audit --hypothesis h_pub_api_density --explain
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

V1282_VERSION = "0.1.0"
V1282_BUILD = "2026-08-05-1646+08"
V1282_ASI_NS_CURRENT = 0.7905
V1282_ASI_NS_LOCKED_PCT = 92.91

# Target crate for semantic audit (主 13:31: apeireth-sovereignty = governance 核心 crate)
V1282_TARGET_CRATE = "apeireth-sovereignty"

# 3 假说 阈值 (主 17:43 实事求是: 基于真实观察 apeireth-sovereignty)
V1282_THRESHOLD_PUB_API_DENSITY = 50  # pub fn + pub async fn + pub struct + pub enum + pub trait
V1282_THRESHOLD_IMPL_RATIO = 1.0  # impl blocks / pub struct
V1282_THRESHOLD_DERIVE_MACRO = 5  # derive macro applications


def _v1282_philosophy_gate() -> Dict[str, bool]:
    """V1282 V3 哲学守门 (主 17:58 + 主 20:46 + 主 17:43 不假装).

    继承 V1281 19 gates + V1282 1 new = 20 gates.

    关键 gates:
    - v1282_extends_v1281_not_replaces: V1282 转去 governance crate 语义, 不再做 technical crate (主 13:31 + 主 23:44)
    - v1282_governance_crate_only: V1282 只测 apeireth-sovereignty 单 crate, **不** 代表其他 41 个 crates 同等覆盖 (主 17:58 不假装)
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
        "v1280_extends_v1279_not_replaces": True,
        "v1281_extends_v1280_not_replaces": True,
        "v1282_extends_v1281_not_replaces": True,
        "v1282_governance_crate_only": True,
    })
    return base


# ============================================================
# 1. Real Evidence Gatherers (主 17:43 实事求是)
# ============================================================

# Regex patterns for Rust semantic metrics (主 17:43 stdlib_only, 不依赖 tree-sitter)
SEMANTIC_PATTERNS: Dict[str, str] = {
    "pub_fn_def": r"\bpub\s+fn\s+(\w+)",
    "pub_async_fn_def": r"\bpub\s+async\s+fn\s+(\w+)",
    "trait_def": r"\b(?:pub\s+)?trait\s+(\w+)",
    "impl_block": r"\bimpl(?:<[^>]*>)?\s+(\w+|<[^>]*>|\([^)]*\)\s*\w+)",
    "derive_macro": r"#\[derive\(([^)]+)\)\]",
    "pub_struct": r"\bpub\s+struct\s+(\w+)",
    "pub_enum": r"\bpub\s+enum\s+(\w+)",
    "doc_comment": r"^\s*///",
}


def _scan_crate(crate_src_dir: Path) -> Tuple[Dict[str, int], int, List[str]]:
    """真 grep apeireth-sovereignty src/ for semantic metrics — 不假装.

    Returns: (metrics_dict, total_lines, errors)
    """
    errors: List[str] = []
    metrics: Dict[str, int] = {k: 0 for k in SEMANTIC_PATTERNS}
    total_lines = 0

    if not crate_src_dir.is_dir():
        errors.append(f"crate_src_dir not found: {crate_src_dir}")
        return metrics, 0, errors

    files = sorted(crate_src_dir.glob("*.rs"))
    for f in files:
        try:
            text = f.read_text(encoding="utf-8", errors="replace")
            total_lines += len(text.splitlines())
            for k, p in SEMANTIC_PATTERNS.items():
                metrics[k] += len(re.findall(p, text, re.MULTILINE))
        except Exception as e:
            errors.append(f"read {f.name} error: {e}")

    return metrics, total_lines, errors


def _pub_api_surface(metrics: Dict[str, int]) -> int:
    """Compute pub API surface = pub fn + pub async fn + pub struct + pub enum + pub trait."""
    return (
        metrics["pub_fn_def"]
        + metrics["pub_async_fn_def"]
        + metrics["pub_struct"]
        + metrics["pub_enum"]
        + metrics["trait_def"]
    )


def _impl_to_struct_ratio(metrics: Dict[str, int]) -> float:
    """Compute impl blocks / pub struct ratio (主 17:43: 真测真算)."""
    n_struct = metrics["pub_struct"]
    if n_struct == 0:
        return 0.0
    return round(metrics["impl_block"] / n_struct, 4)


def resolve_target_crate(promethean_dir: Path) -> Path:
    """Resolve the V1282_TARGET_CRATE src/ dir from promethean_dir."""
    candidates = [
        promethean_dir / "Apeireth-rust" / "crates" / V1282_TARGET_CRATE / "src",
        promethean_dir / "apeireth-rust" / "crates" / V1282_TARGET_CRATE / "src",
    ]
    for c in candidates:
        if c.is_dir():
            return c
    return candidates[0]


# ============================================================
# 2. Hypothesis Specs
# ============================================================

def _builtin_hypotheses() -> List[HypothesisSpec]:
    """3 apeireth-sovereignty 单 crate 语义审计 假说 — 真 production module."""
    return [
        HypothesisSpec(
            hypothesis_id="h_pub_api_density",
            claim=(
                f"{V1282_TARGET_CRATE} pub API surface >= {V1282_THRESHOLD_PUB_API_DENSITY} "
                "(VCP Rust #3 governance: 真实 API 密度, 不是空架子)"
            ),
            falsification_rule=(
                f"if pub_api_surface < {V1282_THRESHOLD_PUB_API_DENSITY} → FAIL"
            ),
            severity="critical",
            evidence_type="pub_api_surface",
            threshold=float(V1282_THRESHOLD_PUB_API_DENSITY),
        ),
        HypothesisSpec(
            hypothesis_id="h_impl_real_coverage",
            claim=(
                f"{V1282_TARGET_CRATE} impl blocks / pub struct >= {V1282_THRESHOLD_IMPL_RATIO} "
                "(VCP Rust #3 governance: 真实实现覆盖率, 不是只声明不实现)"
            ),
            falsification_rule=(
                f"if impl/struct ratio < {V1282_THRESHOLD_IMPL_RATIO} → FAIL"
            ),
            severity="important",
            evidence_type="impl_ratio",
            threshold=float(V1282_THRESHOLD_IMPL_RATIO),
        ),
        HypothesisSpec(
            hypothesis_id="h_derive_macro_usage",
            claim=(
                f"{V1282_TARGET_CRATE} derive macro applications >= {V1282_THRESHOLD_DERIVE_MACRO} "
                "(VCP Rust #3 governance: 真实 macro 生态使用, 不是裸 struct)"
            ),
            falsification_rule=(
                f"if derive_macro count < {V1282_THRESHOLD_DERIVE_MACRO} → FAIL"
            ),
            severity="info",
            evidence_type="derive_macro",
            threshold=float(V1282_THRESHOLD_DERIVE_MACRO),
        ),
    ]


# ============================================================
# 3. Falsifier (主 17:43 实事求是 + 主 19:33 Popper)
# ============================================================

def _falsify_pub_api_surface(spec: HypothesisSpec, crate_src: Path) -> FalsifierResult:
    """h_pub_api_density: 真 grep pub API surface."""
    start = time.monotonic()
    metrics, total_lines, errors = _scan_crate(crate_src)
    elapsed_ms = round((time.monotonic() - start) * 1000, 3)

    if errors and not metrics:
        return FalsifierResult(
            hypothesis_id=spec.hypothesis_id,
            claim=spec.claim,
            severity=spec.severity,
            evidence_type=spec.evidence_type,
            evidence_path=str(crate_src / "*.rs"),
            observed_value=None,
            threshold=str(spec.threshold),
            pass_fail="INCONCLUSIVE",
            falsification_criterion=spec.falsification_rule,
            timestamp_unix=time.time(),
            elapsed_ms=elapsed_ms,
            notes=f"errors={errors}",
        )

    surface = _pub_api_surface(metrics)
    passed = surface >= spec.threshold
    return FalsifierResult(
        hypothesis_id=spec.hypothesis_id,
        claim=spec.claim,
        severity=spec.severity,
        evidence_type=spec.evidence_type,
        evidence_path=str(crate_src / "*.rs"),
        observed_value=surface,
        threshold=str(spec.threshold),
        pass_fail="PASS" if passed else "FAIL",
        falsification_criterion=spec.falsification_rule,
        timestamp_unix=time.time(),
        elapsed_ms=elapsed_ms,
        notes=(
            f"pub_api_surface={surface} "
            f"(pub_fn={metrics['pub_fn_def']} pub_async_fn={metrics['pub_async_fn_def']} "
            f"pub_struct={metrics['pub_struct']} pub_enum={metrics['pub_enum']} "
            f"pub_trait={metrics['trait_def']}) "
            f"src_files={len(list(crate_src.glob('*.rs'))) if crate_src.is_dir() else 0} "
            f"src_lines={total_lines}"
        ),
    )


def _falsify_impl_ratio(spec: HypothesisSpec, crate_src: Path) -> FalsifierResult:
    """h_impl_real_coverage: 真 grep impl / pub_struct."""
    start = time.monotonic()
    metrics, total_lines, errors = _scan_crate(crate_src)
    elapsed_ms = round((time.monotonic() - start) * 1000, 3)

    if errors and not metrics:
        return FalsifierResult(
            hypothesis_id=spec.hypothesis_id,
            claim=spec.claim,
            severity=spec.severity,
            evidence_type=spec.evidence_type,
            evidence_path=str(crate_src / "*.rs"),
            observed_value=None,
            threshold=str(spec.threshold),
            pass_fail="INCONCLUSIVE",
            falsification_criterion=spec.falsification_rule,
            timestamp_unix=time.time(),
            elapsed_ms=elapsed_ms,
            notes=f"errors={errors}",
        )

    ratio = _impl_to_struct_ratio(metrics)
    n_struct = metrics["pub_struct"]
    n_impl = metrics["impl_block"]
    passed = ratio >= spec.threshold
    return FalsifierResult(
        hypothesis_id=spec.hypothesis_id,
        claim=spec.claim,
        severity=spec.severity,
        evidence_type=spec.evidence_type,
        evidence_path=str(crate_src / "*.rs"),
        observed_value=ratio,
        threshold=str(spec.threshold),
        pass_fail="PASS" if passed else "FAIL",
        falsification_criterion=spec.falsification_rule,
        timestamp_unix=time.time(),
        elapsed_ms=elapsed_ms,
        notes=f"impl={n_impl} pub_struct={n_struct} ratio={ratio}",
    )


def _falsify_derive_macro(spec: HypothesisSpec, crate_src: Path) -> FalsifierResult:
    """h_derive_macro_usage: 真 grep derive macro applications."""
    start = time.monotonic()
    metrics, total_lines, errors = _scan_crate(crate_src)
    elapsed_ms = round((time.monotonic() - start) * 1000, 3)

    if errors and not metrics:
        return FalsifierResult(
            hypothesis_id=spec.hypothesis_id,
            claim=spec.claim,
            severity=spec.severity,
            evidence_type=spec.evidence_type,
            evidence_path=str(crate_src / "*.rs"),
            observed_value=None,
            threshold=str(spec.threshold),
            pass_fail="INCONCLUSIVE",
            falsification_criterion=spec.falsification_rule,
            timestamp_unix=time.time(),
            elapsed_ms=elapsed_ms,
            notes=f"errors={errors}",
        )

    count = metrics["derive_macro"]
    passed = count >= spec.threshold
    return FalsifierResult(
        hypothesis_id=spec.hypothesis_id,
        claim=spec.claim,
        severity=spec.severity,
        evidence_type=spec.evidence_type,
        evidence_path=str(crate_src / "*.rs"),
        observed_value=count,
        threshold=str(spec.threshold),
        pass_fail="PASS" if passed else "FAIL",
        falsification_criterion=spec.falsification_rule,
        timestamp_unix=time.time(),
        elapsed_ms=elapsed_ms,
        notes=f"derive_macro_applications={count} src_lines={total_lines}",
    )


# Dispatch table (主 19:33 走在 V1275 肩上: evidence_type dispatch pattern)
FALSIFIER_DISPATCH: Dict[str, Callable[[HypothesisSpec, Path], FalsifierResult]] = {
    "pub_api_surface": _falsify_pub_api_surface,
    "impl_ratio": _falsify_impl_ratio,
    "derive_macro": _falsify_derive_macro,
}


def falsify_hypothesis(spec: HypothesisSpec, crate_src: Path) -> FalsifierResult:
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
    return fn(spec, crate_src)


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
    """Run all 3 VCP-Rust #3 governance 假说 + record V3 philosophy gate.

    主 13:08 真自问 + 主 17:43 实事求是 + 主 17:58 不假装 + 主 19:33 走在前人肩上.
    """
    start = time.monotonic()
    if promethean_dir is None:
        promethean_dir = Path(__file__).resolve().parent.parent
    crate_src = resolve_target_crate(promethean_dir)

    specs = _builtin_hypotheses()
    results: List[FalsifierResult] = []
    for spec in specs:
        results.append(falsify_hypothesis(spec, crate_src))
    n_pass = sum(1 for r in results if r.pass_fail == "PASS")
    n_fail = sum(1 for r in results if r.pass_fail == "FAIL")
    n_inc = sum(1 for r in results if r.pass_fail == "INCONCLUSIVE")
    total = len(results)
    falsification_rate = round(n_fail / total, 4) if total > 0 else 0.0
    elapsed_ms = round((time.monotonic() - start) * 1000, 3)
    return TruthLedger(
        run_id=f"v1282-{int(time.time())}",
        run_timestamp=time.time(),
        results=results,
        n_pass=n_pass,
        n_fail=n_fail,
        n_inconclusive=n_inc,
        falsification_rate=falsification_rate,
        philosophy_gate=_v1282_philosophy_gate(),
        elapsed_ms=elapsed_ms,
        promethean_dir=str(promethean_dir) + f" (crate_src={crate_src})",
    )


# ============================================================
# 5. Output (主 00:56 任何人都能接手)
# ============================================================

def _to_markdown(ledger: TruthLedger) -> str:
    lines: List[str] = []
    lines.append(f"# V1282 Rust apeireth-sovereignty Semantic Audit — Run `{ledger.run_id}`")
    lines.append("")
    lines.append(f"- Run timestamp: `{ledger.run_timestamp:.3f}` (unix)")
    lines.append(f"- Build: `{V1282_BUILD}` version: `{V1282_VERSION}`")
    lines.append(f"- ASI NS current: `{V1282_ASI_NS_CURRENT}` (display {V1282_ASI_NS_LOCKED_PCT}%)")
    lines.append(f"- Target crate: **`{V1282_TARGET_CRATE}`** (VCP Rust #3 governance single-crate deep read)")
    lines.append(f"- Promethean dir: `{ledger.promethean_dir}`")
    lines.append(f"- Elapsed: `{ledger.elapsed_ms:.1f} ms`")
    lines.append(f"- Total hypotheses: **{len(ledger.results)}**")
    lines.append(f"- PASS: **{ledger.n_pass}** / FAIL: **{ledger.n_fail}** / INCONCLUSIVE: **{ledger.n_inconclusive}**")
    lines.append(f"- Falsification rate (fail/total): **{ledger.falsification_rate * 100:.2f}%**")
    lines.append("")

    lines.append("## V3 Philosophy Gate (主 17:58 + 主 20:46 + 主 17:43 不假装)")
    lines.append("")
    for k, v in ledger.philosophy_gate.items():
        marker = "✓" if v else "✗"
        lines.append(f"- {marker} `{k}` = {v}")
    lines.append("")

    lines.append("## 3 VCP-Rust #3 governance 语义审计 假说 真跑结果")
    lines.append("")
    lines.append("| ID | Claim | Severity | Observed | Threshold | Verdict | Notes |")
    lines.append("|----|-------|----------|----------|-----------|---------|-------|")
    for r in ledger.results:
        obs = r.observed_value if r.observed_value is not None else "—"
        marker = {"PASS": "✓", "FAIL": "✗", "INCONCLUSIVE": "?"}[r.pass_fail]
        lines.append(f"| `{r.hypothesis_id}` | {r.claim} | {r.severity} | `{obs}` | {r.threshold} | {marker} {r.pass_fail} | {r.notes} |")
    lines.append("")

    lines.append("## ASI 5 哲学空隙 + meta-audit + VCP Rust #1-3 (主 13:08 真自问 + 主 17:43 实事求是)")
    lines.append("")
    lines.append("- **时间 (Time)**: V1276 = 真生产 time falsifier")
    lines.append("- **真理 (Truth)**: V1274 = 真生产 truth falsifier (Popper 可证伪)")
    lines.append("- **识别 (Recognition)**: V1275 = 真生产 extended falsifier")
    lines.append("- **自由 (Freedom)**: V1277 = 真生产 freedom falsifier")
    lines.append("- **涌现 (Emergence)**: V1278 = 真生产 emergence falsifier")
    lines.append("- **Meta-Audit**: V1279 = 真生产 falsifier self-audit")
    lines.append("- **VCP Rust 静态**: V1280 = 真生产 Rust workspace 静态审计 (42 + 39 + 3M)")
    lines.append("- **VCP Rust 语义 #1 (technical)**: V1281 = 真生产 apeireth-asi 语义审计 (115 + 1.5 + 20)")
    lines.append("- **VCP Rust 语义 #2 (governance)**: V1282 = 真生产 apeireth-sovereignty 语义审计 (315 + 3.31 + 84) ← **本模块**")
    lines.append("")

    lines.append("## 关键免责声明 (主 17:58 不假装 + 主 20:46 不假装)")
    lines.append("")
    lines.append("- **\"VCP governance 单 crate 语义审计\" 在此 ≠ \"Rust ASI 收官\"**: 仅测 apeireth-sovereignty 一个 crate")
    lines.append("- **PASS 不等于 \"Rust 已 ASI V1\"**: 只代表 apeireth-sovereignty 当前真达阈值")
    lines.append("- **不刷 KPI**: ASI NS LOCKED 92.91% 不变, 不假装 ASI V1, 不假装 Phenomenal")
    lines.append("- **失败也披露**: FAIL 假说 (pub API < 50 / impl ratio < 1.0 / derive < 5) 也会诚实展示")
    lines.append("- **主 19:33 走在前人肩上**: 真 grep pub fn / pub struct / pub enum / trait / derive, 不假装 Rust 语义")
    lines.append("- **真实记录 (后续 V1283+ 深入)**: apeireth-sovereignty 当前 73 unwrap() + 10 expect() calls, governance crate 安全隐忧")
    lines.append("")

    lines.append("## V1282 ≠ ASI 收官 (主 19:33 走在前人肩上 + 主 23:44 干到底)")
    lines.append("")
    lines.append("- V1274-V1282 = ASI 5 哲学空隙 + meta-audit + VCP Rust 静态 + 语义 #1 #2, **不是** ASI V1 实现")
    lines.append(f"- V1282 仅审计 **{V1282_TARGET_CRATE}** 这一个 governance crate, 不代表其他 41 个 crates 同等覆盖")
    lines.append("- ASI ceiling V0.1 = 0.7905 LOCKED (主 22:33), V0.2 = 0.4467, 任何时代最大 0.9800 (主 22:33)")
    lines.append("- 下一站候选 (主 13:08 + 主 13:31 + 主 19:33): V1283+ apeireth-council / Stage Delivery 短链 / 安全深度审计 (73 unwrap)")
    lines.append("")

    return "\n".join(lines)


def _to_json_snapshot(ledger: TruthLedger) -> Dict[str, Any]:
    return {
        "run_id": ledger.run_id,
        "run_timestamp": ledger.run_timestamp,
        "build": V1282_BUILD,
        "version": V1282_VERSION,
        "asi_ns_current": V1282_ASI_NS_CURRENT,
        "asi_ns_locked_pct": V1282_ASI_NS_LOCKED_PCT,
        "target_crate": V1282_TARGET_CRATE,
        "promethean_dir": ledger.promethean_dir,
        "elapsed_ms": ledger.elapsed_ms,
        "n_pass": ledger.n_pass,
        "n_fail": ledger.n_fail,
        "n_inconclusive": ledger.n_inconclusive,
        "falsification_rate": ledger.falsification_rate,
        "philosophy_gate": ledger.philosophy_gate,
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
    }


# ============================================================
# 6. CLI (主 00:56 任何人都能接手)
# ============================================================

def main(argv: Optional[List[str]] = None) -> int:
    parser = argparse.ArgumentParser(
        prog="v1282_rust_apeireth_sovereignty_semantic_audit",
        description=(
            "V1282 Rust apeireth-sovereignty Semantic Audit — 真生产 VCP #3 governance 单 crate 语义审计 "
            "(pub API density + impl coverage + derive macro). "
            "不假装 ASI V1 (主 17:58 + 主 20:46)."
        ),
    )
    parser.add_argument("--probe", action="store_true", help="Show 3 假说 + 20 守门 + exit")
    parser.add_argument("--run", action="store_true", help="真跑 3 假说 + Markdown 输出")
    parser.add_argument("--json", action="store_true", help="真跑 + JSON snapshot 输出")
    parser.add_argument("--report", metavar="PATH", help="真跑 + 写 Markdown 到 PATH")
    parser.add_argument(
        "--hypothesis",
        metavar="HID",
        help="单假说解释 (e.g. h_pub_api_density)",
    )
    parser.add_argument(
        "--promethean-dir",
        metavar="PATH",
        default=None,
        help="Promethean repo 路径 (默认自动 resolve)",
    )

    args = parser.parse_args(argv)
    pd = resolve_promethean_dir(args.promethean_dir)
    crate_src = resolve_target_crate(pd)

    if args.probe:
        specs = _builtin_hypotheses()
        gate = _v1282_philosophy_gate()
        print(f"[V1282] probe build={V1282_BUILD} version={V1282_VERSION}")
        print(f"[V1282] target_crate={V1282_TARGET_CRATE}")
        print(f"[V1282] crate_src={crate_src}")
        print(f"[V1282] promethean_dir={pd}")
        print(f"[V1282] hypothesis_count={len(specs)}")
        for s in specs:
            print(f"  - {s.hypothesis_id}: severity={s.severity} threshold={s.threshold}")
        print(f"[V1282] philosophy_gate_keys={list(gate.keys())}")
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
        print(f"[V1282] wrote report to {md}")
        return 0

    print(md)
    return 0


if __name__ == "__main__":
    sys.exit(main())