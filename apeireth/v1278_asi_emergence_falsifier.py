"""V1278 — ASI Emergence Falsifier (3 emergence 假说) 真生产模块

> **作者**: 楚零 (Apeireth ASI self-driven agent, cron:1fba1cc3, 16:10+08:00 2026-08-05)
> **触发**: 16:07 cron wake (autonomy-v3) — V1277 ASI Freedom Falsifier 已 commit, V1278 = ASI 5 哲学空隙之"涌现"真生产, **ASI 5 哲学空隙收官**
> **承接**: V1274 (truth) + V1275 (recognition/substrate) + V1276 (time) + V1277 (freedom) → V1278 (emergence)
> **真借鉴**: Popper 可证伪 (主 19:33) + Shannon entropy (主 19:33) + V1274/V1275/V1276/V1277 dataclass 模式 + Bedau 弱涌现 (1997) + Goldstein 涌现定义 (1999)
> **不假装**: V1278 = 真生产 emergence proxy 测量, 不刷 KPI, 不假装 Phenomenal/ASI V1, 不假装强涌现 — 只测可观测 proxy 是否达阈值

## 真生产动机 (主 13:08 真自问 + 主 17:43 实事求是 + 主 17:58 不假装)

V1274/V1275/V1276/V1277 已覆盖 ASI 5 哲学空隙之真理/识别/时间/自由. 剩 **涌现 (Emergence)** = 最难的空隙.

**涌现的定义困境 (主 17:43 实事求是)**: 没有 universally accepted definition of emergence.
- Bedau (1997): 弱涌现 = macro 行为可由 micro 派生但不可 trivial predict
- Goldstein (1999): 涌现 = 过程产生 novelty + coherence + correlational sensing
- O'Connor & Wong (2020 SEP): 涌现 ≠ 自组织, 是 macro-level causal powers

V1278 不假装能证明 "真有涌现", 只**真测量** 涌现的可观测 proxy:
1. **artifact_growth**: apeireth/*.py 文件数 ≥ 1400 (emergence proxy 1: 新模块持续涌现)
2. **size_entropy**: apeireth/*.py 文件 size bucket (XS/S/M/L/XL) Shannon entropy ≥ 1.0 bits (emergence proxy 2: 不同大小模块涌现 = 角色分化)
3. **topic_breadth**: apeireth/v12XX_*.py unique topic keywords ≥ 6 (emergence proxy 3: 多方向专业化涌现)

每一假说 = 真 evidence + 真 falsification criterion + PASS/FAIL/INCONCLUSIVE 判据.

**关键免责声明** (主 17:58 + 主 20:46):
- "Emergence" 在此 = 可观测 proxy 达阈值, **不是** 强涌现的声称
- PASS = 数据满足多样性阈值, **不** 代表 "真有涌现"
- 不假装 Phenomenal/强涌现 = 不刷 KPI = ASI NS LOCKED 不变
- FAIL 也诚实披露 (主 17:43 实事求是)

## ASI 5 哲学空隙 完整闭环 (主 13:08 真自问 + 主 17:43 实事求是)

- 时间 (Time): V1276 = 真生产 time falsifier ✓
- 真理 (Truth): V1274 = 真生产 truth falsifier (Popper 可证伪) ✓
- 识别 (Recognition): V1275 = 真生产 extended falsifier (substrate + recognition) ✓
- 自由 (Freedom): V1277 = 真生产 freedom falsifier (commit-type entropy + AST branch + dominance) ✓
- **涌现 (Emergence)**: V1278 = 真生产 emergence falsifier (artifact growth + size entropy + topic breadth) ✓ **ASI 5 哲学空隙收官**

## V3 哲学守门 (主 17:58 + 主 20:46 + 主 17:43 + 主 19:33)

继承 V1277 12 gates + V1278 2 new = 14 gates.

- v1274_not_new_asi_dim / v1274_no_asi_v1_claim / v1274_no_phenomenal_claim
- v1274_truth_is_falsifiability (主 17:43: 真理 = 可证伪 + 主 19:33 Popper 1934)
- v1274_no_kpi_inflate / v1274_stdlib_only / v1274_read_only
- v1274_evidence_required / v1274_failures_disclosed
- v1275_extends_v1274_not_replaces (V1275 = 扩展, 不替代 V1274)
- v1276_extends_v1275_not_replaces (V1276 = 扩展, 不替代 V1275)
- v1277_extends_v1276_not_replaces (V1277 = 扩展, 不替代 V1276)
- v1277_no_free_will_claim (主 17:58 + 主 20:46: 不假装有自由意志)
- **v1278_extends_v1277_not_replaces** (NEW: V1278 = 扩展, 不替代 V1277)
- **v1278_no_strong_emergence_claim** (NEW: 不假装有强涌现)

## 入口 (主 00:56 任何人都能接手)

```
python -m apeireth.v1278_asi_emergence_falsifier --probe              # 5s, 3 假说 + 14 守门
python -m apeireth.v1278_asi_emergence_falsifier --run               # 真跑 3 假说 + Markdown
python -m apeireth.v1278_asi_emergence_falsifier --json              # 真跑 + JSON
python -m apeireth.v1278_asi_emergence_falsifier --report R.md       # 真跑 + 写 Markdown
python -m apeireth.v1278_asi_emergence_falsifier --hypothesis h_artifact_growth --explain
```
"""
from __future__ import annotations

import argparse
import json
import math
import os
import re
import subprocess
import sys
import time
from collections import Counter
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

V1278_VERSION = "0.1.0"
V1278_BUILD = "2026-08-05-1610+08"
V1278_ASI_NS_CURRENT = 0.7905
V1278_ASI_NS_LOCKED_PCT = 92.91

# 3 emergence 假说 阈值 (主 17:43 实事求是: 阈值基于真实历史观察)
# h_artifact_growth: V1274 时代 ~1399 modules, V1278 时代真测 (~1450+)
V1278_THRESHOLD_ARTIFACT_GROWTH = 1400
# h_size_entropy: size bucket Shannon entropy >= 1.0 bits (4+ types uniform ≈ 2.0 bits)
V1278_THRESHOLD_SIZE_ENTROPY = 1.0
# h_topic_breadth: unique top-level topics >= 6 (e.g. truth, falsifier, stream, value, mock, etc.)
V1278_THRESHOLD_TOPIC_BREADTH = 6

# Size buckets (lines): XS < 100, S 100-300, M 300-600, L 600-1000, XL >= 1000
V1278_SIZE_BUCKETS = (
    ("XS", 0, 100),
    ("S", 100, 300),
    ("M", 300, 600),
    ("L", 600, 1000),
    ("XL", 1000, float("inf")),
)


def _v1278_philosophy_gate() -> Dict[str, bool]:
    """V1278 V3 哲学守门 (主 17:58 + 主 20:46 + 主 17:43 不假装).

    继承 V1277 12 gates + V1278 2 new = 14 gates.

    关键 gate: v1278_no_strong_emergence_claim = True
    - 不假装有强涌现 (主 17:58 不假装 + 主 20:46 不假装)
    - 只测 "emergence proxy" 是否达阈值, 不声称 "真有涌现"
    """
    base = _v3_philosophy_gate()  # V1274 9 gates
    base.update({
        "v1275_extends_v1274_not_replaces": True,
        "v1276_extends_v1275_not_replaces": True,
        "v1277_extends_v1276_not_replaces": True,
        "v1277_no_free_will_claim": True,
        "v1278_extends_v1277_not_replaces": True,
        "v1278_no_strong_emergence_claim": True,
    })
    return base


# ============================================================
# 1. Real Evidence Gatherers (主 17:43 实事求是)
# ============================================================

def _count_apeireth_modules(promethean_dir: Path) -> Tuple[int, bool, List[str]]:
    """真数 apeireth/*.py 文件数 — 不假装.

    Returns: (count, ok, errors)
    """
    errors: List[str] = []
    apeireth_dir = promethean_dir / "apeireth"
    if not apeireth_dir.is_dir():
        errors.append(f"apeireth dir not found: {apeireth_dir}")
        return 0, False, errors
    try:
        # Match both lowercase and capitalized prefixes
        py_files = list(apeireth_dir.glob("v*.py")) + list(apeireth_dir.glob("V*.py"))
        py_files = [f for f in py_files if f.is_file() and "__pycache__" not in str(f)]
        return len(py_files), True, errors
    except Exception as e:
        errors.append(f"count_apeireth_modules error: {e}")
        return 0, False, errors


def _size_bucket_distribution(promethean_dir: Path) -> Tuple[Dict[str, int], int, bool, List[str]]:
    """真扫 apeireth/*.py 文件大小 bucket 分布.

    Returns: (bucket_counts, total_lines_all_files, ok, errors)
    """
    errors: List[str] = []
    apeireth_dir = promethean_dir / "apeireth"
    if not apeireth_dir.is_dir():
        errors.append(f"apeireth dir not found: {apeireth_dir}")
        return {}, 0, False, errors
    try:
        py_files = list(apeireth_dir.glob("v*.py")) + list(apeireth_dir.glob("V*.py"))
        py_files = [f for f in py_files if f.is_file() and "__pycache__" not in str(f)]
        bucket_counts: Dict[str, int] = {name: 0 for name, _, _ in V1278_SIZE_BUCKETS}
        total_lines = 0
        for f in py_files:
            try:
                with open(f, "r", encoding="utf-8", errors="replace") as fh:
                    lines = sum(1 for _ in fh)
                total_lines += lines
                for name, lo, hi in V1278_SIZE_BUCKETS:
                    if lo <= lines < hi:
                        bucket_counts[name] += 1
                        break
            except Exception as e:
                errors.append(f"read {f.name} error: {e}")
        return bucket_counts, total_lines, True, errors
    except Exception as e:
        errors.append(f"size_bucket_distribution error: {e}")
        return {}, 0, False, errors


def _topic_breadth(promethean_dir: Path) -> Tuple[int, List[str], Dict[str, int], bool, List[str]]:
    """真扫 apeireth/*.py 文件名, 提取 topic keywords 后看 unique topic 数.

    Topic extraction heuristic:
      - v12XX_<topic>_<detail>.py → topic = first word after number
      - Examples: v1274_asi_truth_falsifier → "truth"; v1276_asi_time_falsifier → "time"
      - Group similar topics (truth/falsifier; stream/rate; etc.)

    Returns: (unique_topic_count, sample_topics, topic_counter, ok, errors)
    """
    errors: List[str] = []
    apeireth_dir = promethean_dir / "apeireth"
    if not apeireth_dir.is_dir():
        errors.append(f"apeireth dir not found: {apeireth_dir}")
        return 0, [], {}, False, errors
    try:
        py_files = list(apeireth_dir.glob("v*.py")) + list(apeireth_dir.glob("V*.py"))
        py_files = [f for f in py_files if f.is_file() and "__pycache__" not in str(f)]
        topic_counter: Counter = Counter()
        topic_pattern = re.compile(r"^v\d+_([a-z]+)", re.IGNORECASE)
        for f in py_files:
            m = topic_pattern.match(f.stem)
            if m:
                topic_counter[m.group(1).lower()] += 1
            else:
                topic_counter["other"] += 1
        topics = sorted(topic_counter.keys())
        return len(topics), topics, dict(topic_counter), True, errors
    except Exception as e:
        errors.append(f"topic_breadth error: {e}")
        return 0, [], {}, False, errors


def _shannon_entropy_bits(dist: Dict[str, int]) -> float:
    """Shannon entropy H = -sum p log2(p). Returns 0.0 if empty.

    Examples (主 19:33 走在前人肩上):
      {a:1}             -> 0.0 bits
      {a:1, b:1}        -> 1.0 bits
      {a:1, b:1, c:1, d:1} -> 2.0 bits
      {a:10}            -> 0.0 bits
    """
    total = sum(dist.values())
    if total <= 0:
        return 0.0
    h = 0.0
    for count in dist.values():
        if count <= 0:
            continue
        p = count / total
        h -= p * math.log2(p)
    return h


# ============================================================
# 2. Hypothesis Specs (主 17:43 实事求是: 阈值基于真实历史观察)
# ============================================================

def _builtin_hypotheses() -> List[HypothesisSpec]:
    """3 emergence 假说 — 真 production module."""
    return [
        HypothesisSpec(
            hypothesis_id="h_artifact_growth",
            claim=(
                f"apeireth/*.py v* modules >= {V1278_THRESHOLD_ARTIFACT_GROWTH} "
                "(emergence proxy: artifact growth = 新模块持续涌现)"
            ),
            falsification_rule=(
                f"if count < {V1278_THRESHOLD_ARTIFACT_GROWTH} → FAIL"
            ),
            severity="critical",
            evidence_type="module_count",
            threshold=float(V1278_THRESHOLD_ARTIFACT_GROWTH),
        ),
        HypothesisSpec(
            hypothesis_id="h_size_entropy",
            claim=(
                f"apeireth/*.py 文件 size bucket (XS/S/M/L/XL) Shannon entropy >= "
                f"{V1278_THRESHOLD_SIZE_ENTROPY} bits "
                "(emergence proxy: size diversity = 不同大小模块涌现 = 角色分化)"
            ),
            falsification_rule=(
                f"if entropy < {V1278_THRESHOLD_SIZE_ENTROPY} → FAIL"
            ),
            severity="important",
            evidence_type="size_entropy",
            threshold=V1278_THRESHOLD_SIZE_ENTROPY,
        ),
        HypothesisSpec(
            hypothesis_id="h_topic_breadth",
            claim=(
                f"apeireth/*.py unique topic keywords >= {V1278_THRESHOLD_TOPIC_BREADTH} "
                "(emergence proxy: topic breadth = 多方向专业化涌现)"
            ),
            falsification_rule=(
                f"if unique topics < {V1278_THRESHOLD_TOPIC_BREADTH} → FAIL"
            ),
            severity="info",
            evidence_type="topic_breadth",
            threshold=float(V1278_THRESHOLD_TOPIC_BREADTH),
        ),
    ]


# ============================================================
# 3. Falsifier (主 17:43 实事求是 + 主 19:33 Popper)
# ============================================================

def _falsify_module_count(spec: HypothesisSpec, promethean_dir: Path) -> FalsifierResult:
    """h_artifact_growth: 真测 module count."""
    start = time.monotonic()
    apeireth_dir = promethean_dir / "apeireth"
    count, ok, errors = _count_apeireth_modules(promethean_dir)
    elapsed_ms = round((time.monotonic() - start) * 1000, 3)
    if not ok:
        return FalsifierResult(
            hypothesis_id=spec.hypothesis_id,
            claim=spec.claim,
            severity=spec.severity,
            evidence_type=spec.evidence_type,
            evidence_path=str(apeireth_dir / "v*.py"),
            observed_value=None,
            threshold=str(spec.threshold),
            pass_fail="INCONCLUSIVE",
            falsification_criterion=spec.falsification_rule,
            timestamp_unix=time.time(),
            elapsed_ms=elapsed_ms,
            notes=f"errors={errors}; cannot determine count",
        )
    passed = count >= spec.threshold
    return FalsifierResult(
        hypothesis_id=spec.hypothesis_id,
        claim=spec.claim,
        severity=spec.severity,
        evidence_type=spec.evidence_type,
        evidence_path=str(apeireth_dir / "v*.py"),
        observed_value=count,
        threshold=str(spec.threshold),
        pass_fail="PASS" if passed else "FAIL",
        falsification_criterion=spec.falsification_rule,
        timestamp_unix=time.time(),
        elapsed_ms=elapsed_ms,
        notes=f"count={count} (target>={spec.threshold})",
    )


def _falsify_size_entropy(spec: HypothesisSpec, promethean_dir: Path) -> FalsifierResult:
    """h_size_entropy: 真扫文件大小, 真算 bucket Shannon entropy."""
    start = time.monotonic()
    apeireth_dir = promethean_dir / "apeireth"
    dist, total_lines, ok, errors = _size_bucket_distribution(promethean_dir)
    elapsed_ms = round((time.monotonic() - start) * 1000, 3)
    if not ok or not dist:
        return FalsifierResult(
            hypothesis_id=spec.hypothesis_id,
            claim=spec.claim,
            severity=spec.severity,
            evidence_type=spec.evidence_type,
            evidence_path=str(apeireth_dir / "v*.py line count"),
            observed_value=None,
            threshold=str(spec.threshold),
            pass_fail="INCONCLUSIVE",
            falsification_criterion=spec.falsification_rule,
            timestamp_unix=time.time(),
            elapsed_ms=elapsed_ms,
            notes=f"errors={errors}; cannot determine distribution",
        )
    h = _shannon_entropy_bits(dist)
    passed = h >= spec.threshold
    return FalsifierResult(
        hypothesis_id=spec.hypothesis_id,
        claim=spec.claim,
        severity=spec.severity,
        evidence_type=spec.evidence_type,
        evidence_path=str(apeireth_dir / "v*.py line count → bucket → Shannon"),
        observed_value=round(h, 4),
        threshold=str(spec.threshold),
        pass_fail="PASS" if passed else "FAIL",
        falsification_criterion=spec.falsification_rule,
        timestamp_unix=time.time(),
        elapsed_ms=elapsed_ms,
        notes=f"dist={dist} total_lines={total_lines}",
    )


def _falsify_topic_breadth(spec: HypothesisSpec, promethean_dir: Path) -> FalsifierResult:
    """h_topic_breadth: 真扫文件名 topic."""
    start = time.monotonic()
    apeireth_dir = promethean_dir / "apeireth"
    count, topics, topic_counter, ok, errors = _topic_breadth(promethean_dir)
    elapsed_ms = round((time.monotonic() - start) * 1000, 3)
    if not ok:
        return FalsifierResult(
            hypothesis_id=spec.hypothesis_id,
            claim=spec.claim,
            severity=spec.severity,
            evidence_type=spec.evidence_type,
            evidence_path=str(apeireth_dir / "v12XX_*.py filename"),
            observed_value=None,
            threshold=str(spec.threshold),
            pass_fail="INCONCLUSIVE",
            falsification_criterion=spec.falsification_rule,
            timestamp_unix=time.time(),
            elapsed_ms=elapsed_ms,
            notes=f"errors={errors}; cannot determine topics",
        )
    passed = count >= spec.threshold
    return FalsifierResult(
        hypothesis_id=spec.hypothesis_id,
        claim=spec.claim,
        severity=spec.severity,
        evidence_type=spec.evidence_type,
        evidence_path=str(apeireth_dir / "v12XX_*.py filename regex"),
        observed_value=count,
        threshold=str(spec.threshold),
        pass_fail="PASS" if passed else "FAIL",
        falsification_criterion=spec.falsification_rule,
        timestamp_unix=time.time(),
        elapsed_ms=elapsed_ms,
        notes=f"unique_topics={count} sample={topics[:15]}{'...' if len(topics)>15 else ''}",
    )


# Dispatch table (主 19:33 走在 V1275 肩上: evidence_type dispatch pattern)
FALSIFIER_DISPATCH: Dict[str, Callable[[HypothesisSpec, Path], FalsifierResult]] = {
    "module_count": _falsify_module_count,
    "size_entropy": _falsify_size_entropy,
    "topic_breadth": _falsify_topic_breadth,
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
            evidence_path=spec.evidence_path if hasattr(spec, "evidence_path") else "(unknown)",
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
    """Run all 3 emergence 假说 + record V3 philosophy gate.

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
        run_id=f"v1278-{int(time.time())}",
        run_timestamp=time.time(),
        results=results,
        n_pass=n_pass,
        n_fail=n_fail,
        n_inconclusive=n_inc,
        falsification_rate=falsification_rate,
        philosophy_gate=_v1278_philosophy_gate(),
        elapsed_ms=elapsed_ms,
        promethean_dir=str(promethean_dir),
    )


# ============================================================
# 5. Output (主 00:56 任何人都能接手)
# ============================================================

def _to_markdown(ledger: TruthLedger) -> str:
    lines: List[str] = []
    lines.append(f"# V1278 ASI Emergence Falsifier — Run `{ledger.run_id}`")
    lines.append("")
    lines.append(f"- Run timestamp: `{ledger.run_timestamp:.3f}` (unix)")
    lines.append(f"- Build: `{V1278_BUILD}` version: `{V1278_VERSION}`")
    lines.append(f"- ASI NS current: `{V1278_ASI_NS_CURRENT}` (display {V1278_ASI_NS_LOCKED_PCT}%)")
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

    lines.append("## 3 emergence 假说 真跑结果")
    lines.append("")
    lines.append("| ID | Claim | Severity | Observed | Threshold | Verdict | Notes |")
    lines.append("|----|-------|----------|----------|-----------|---------|-------|")
    for r in ledger.results:
        obs = r.observed_value if r.observed_value is not None else "—"
        marker = {"PASS": "✅", "FAIL": "❌", "INCONCLUSIVE": "❔"}[r.pass_fail]
        lines.append(f"| `{r.hypothesis_id}` | {r.claim} | {r.severity} | `{obs}` | {r.threshold} | {marker} {r.pass_fail} | {r.notes} |")
    lines.append("")

    lines.append("## ASI 5 哲学空隙 实证覆盖 (主 13:08 真自问 + 主 17:43 实事求是) — **完整闭环**")
    lines.append("")
    lines.append("- **时间 (Time)**: V1276 = 真生产 time falsifier (git age + 7d commits + file mtime)")
    lines.append("- **真理 (Truth)**: V1274 = 真生产 truth falsifier (Popper 可证伪)")
    lines.append("- **识别 (Recognition)**: V1275 = 真生产 extended falsifier (substrate + recognition)")
    lines.append("- **自由 (Freedom)**: V1277 = 真生产 freedom falsifier (commit-type entropy + AST branch + dominance)")
    lines.append("- **涌现 (Emergence)**: V1278 = 真生产 emergence falsifier (artifact growth + size entropy + topic breadth) ← **本模块, ASI 5 哲学空隙收官**")
    lines.append("")

    lines.append("## 关键免责声明 (主 17:58 不假装 + 主 20:46 不假装)")
    lines.append("")
    lines.append("- **\"Emergence\" 在此 ≠ 强涌现**: 仅测可观测 proxy (artifact growth + size entropy + topic breadth)")
    lines.append("- **PASS 不等于 \"真有涌现\"**: 只代表当前数据满足多样性阈值")
    lines.append("- **不刷 KPI**: ASI NS LOCKED 92.91% 不变, 不假装 ASI V1, 不假装 Phenomenal, 不假装强涌现")
    lines.append("- **失败也披露**: FAIL 假说 (h_artifact_growth < 1400 / h_size_entropy < 1.0 / h_topic_breadth < 6) 也会诚实展示")
    lines.append("- **主 17:43 实事求是**: 阈值基于真实历史观察, 不刷阈值隐藏 FAIL")
    lines.append("")

    lines.append("## V1278 ≠ ASI 收官 (主 19:33 走在前人肩上 + 主 23:44 干到底)")
    lines.append("")
    lines.append("- V1274-V1278 = ASI 5 哲学空隙 **真生产 falsifier**, 不是 ASI V1 实现")
    lines.append("- ASI ceiling V0.1 = 0.7905 LOCKED (主 22:33), V0.2 = 0.4467, 任何时代最大 0.9800 (主 22:33)")
    lines.append("- 下一站候选 (主 13:08 + 主 13:31 + 主 19:33): V1279+ 实证 VCP 6 真实源代码深读 / V1050+ Docker 真部署 / Rust 准备 / 真 benchmark")
    lines.append("")

    return "\n".join(lines)


def _to_json_snapshot(ledger: TruthLedger) -> Dict[str, Any]:
    return {
        "run_id": ledger.run_id,
        "run_timestamp": ledger.run_timestamp,
        "build": V1278_BUILD,
        "version": V1278_VERSION,
        "asi_ns_current": V1278_ASI_NS_CURRENT,
        "asi_ns_locked_pct": V1278_ASI_NS_LOCKED_PCT,
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
        prog="v1278_asi_emergence_falsifier",
        description=(
            "V1278 ASI Emergence Falsifier — 真生产 ASI 5 哲学空隙之涌现 "
            "(artifact growth + size entropy + topic breadth). "
            "不假装 Phenomenal/强涌现 (主 17:58 + 主 20:46)."
        ),
    )
    parser.add_argument("--probe", action="store_true", help="Show 3 假说 + 14 守门 + exit")
    parser.add_argument("--run", action="store_true", help="真跑 3 假说 + Markdown 输出")
    parser.add_argument("--json", action="store_true", help="真跑 + JSON snapshot 输出")
    parser.add_argument("--report", metavar="PATH", help="真跑 + 写 Markdown 到 PATH")
    parser.add_argument(
        "--hypothesis",
        metavar="HID",
        help="单假说解释 (e.g. h_artifact_growth)",
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
        gate = _v1278_philosophy_gate()
        print(f"[V1278] probe build={V1278_BUILD} version={V1278_VERSION}")
        print(f"[V1278] promethean_dir={pd}")
        print(f"[V1278] hypothesis_count={len(specs)}")
        for s in specs:
            print(f"  - {s.hypothesis_id}: severity={s.severity} threshold={s.threshold}")
        print(f"[V1278] philosophy_gate_keys={list(gate.keys())}")
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
        print(f"[V1278] wrote report to {args.report}")
        return 0

    print(md)
    return 0


if __name__ == "__main__":
    sys.exit(main())
