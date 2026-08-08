#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
v1085_hqb_smoke.py — V1085 HQB CLI smoke (post-V1086 persistence + V1087 live gate)

- Version: 0.1.0
- Author: 楚零 (Chu Ling, Apeireth ASI self-driven agent)
- Cron: 1fba1cc3-1a6d-4e3a-abb8-fccef1c94cdf (apeireth-autonomy-v3)
- Trigger: V1358 STAGE-DELIVERY plan item V1359-1 (fill real_production gap: V1085 smoke missing)

This module is a CLI smoke for V1085 HonestDecisionModule. It:
1. Constructs N synthetic HQBScore values across the verdict spectrum
   (accept / review / reject / veto)
2. Calls V1085 evaluate() on each
3. Records decisions with verdict + reason + score_used
4. Produces a structured smoke report (status: ok)

V3 哲学守门 (主 17:58 + 20:46 + 17:43):
- 不假装分数 = ASI: veto fires when score >= veto_threshold (perfect score = red flag)
- 不假装决策 = 真生产: every decision has reason + score_used
- 不破坏 4 层安全门: this is a smoke (Layer 1 self-check), not a gate
- V1085 smoke != ASI: smoke tests V1085, NOT philosophy or AGI properties

边界: 不动 V1074 / V1081 / V1086 / V1087 / philosophy_guard
"""

from __future__ import annotations

import argparse
import json
import sys
import time
import uuid
from dataclasses import dataclass, field, asdict
from pathlib import Path
from typing import Any, Dict, List, Optional

from apeireth.v36_hqb_benchmark import HQBScore
from apeireth.v1085_hqb_core import (
    DEFAULT_ACCEPT_THRESHOLD,
    DEFAULT_REJECT_THRESHOLD,
    HonestDecision,
    HonestDecisionModule,
    Verdict,
    V1085_VERSION,
)


V1085_SMOKE_VERSION = "0.1.0"

# Synthetic HQB-score fixtures (主 17:43 实事求是 — deterministic, not random)
# Each tuple is (label, sc, nr, ev, cdt, expected_verdict_hint)
SMOKE_FIXTURES: List[tuple] = [
    ("low_quality_reject",   0.10, 0.10, 0.10, 0.10, Verdict.REJECT),
    ("medium_review",        0.55, 0.50, 0.60, 0.55, Verdict.REVIEW),
    ("good_accept",          0.80, 0.85, 0.80, 0.80, Verdict.ACCEPT),
    ("high_accept",          0.92, 0.93, 0.91, 0.92, Verdict.ACCEPT),
    ("perfect_veto",         0.99, 0.99, 0.99, 0.99, Verdict.VETO),
]


@dataclass
class SmokeCaseResult:
    """One smoke case = one HQB score → one V1085 decision."""
    label: str
    score_total: float
    score_components: Dict[str, float]
    verdict: str
    score_used: float
    reason: str
    expected_hint: str
    matches_hint: bool


@dataclass
class SmokeReport:
    """V1085 smoke aggregate report."""
    version: str = V1085_SMOKE_VERSION
    v1085_version: str = V1085_VERSION
    accept_threshold: float = DEFAULT_ACCEPT_THRESHOLD
    reject_threshold: float = DEFAULT_REJECT_THRESHOLD
    cases: List[SmokeCaseResult] = field(default_factory=list)
    verdict_distribution: Dict[str, int] = field(default_factory=dict)
    status: str = "ok"  # ok | partial | error
    ts: float = field(default_factory=time.time)

    def to_dict(self) -> Dict[str, Any]:
        return {
            "version": self.version,
            "v1085_version": self.v1085_version,
            "accept_threshold": self.accept_threshold,
            "reject_threshold": self.reject_threshold,
            "cases": [asdict(c) for c in self.cases],
            "verdict_distribution": self.verdict_distribution,
            "status": self.status,
            "ts": self.ts,
        }


def _build_hqb(label: str, sc: float, nr: float, ev: float, cdt: float) -> HQBScore:
    """Build a synthetic HQBScore (主 17:43 实事求是 — labeled, deterministic)."""
    return HQBScore(
        score_id=f"smoke_{label}_{uuid.uuid4().hex[:8]}",
        sc=sc, nr=nr, ev=ev, cdt=cdt,
    )


def run_smoke(
    module: Optional[HonestDecisionModule] = None,
    fixtures: Optional[List[tuple]] = None,
) -> SmokeReport:
    """Run V1085 smoke over the synthetic fixtures (or supplied custom set)."""
    module = module or HonestDecisionModule()
    fixtures = fixtures or SMOKE_FIXTURES

    report = SmokeReport(
        accept_threshold=module.accept_threshold,
        reject_threshold=module.reject_threshold,
    )
    distribution: Dict[str, int] = {v.value: 0 for v in Verdict}

    for label, sc, nr, ev, cdt, expected_hint in fixtures:
        hqb = _build_hqb(label, sc, nr, ev, cdt)
        decision: HonestDecision = module.evaluate(hqb, context=f"smoke:{label}")
        total = hqb.total
        matches = decision.verdict == expected_hint
        case = SmokeCaseResult(
            label=label,
            score_total=round(total, 4),
            score_components={
                "sc": sc, "nr": nr, "ev": ev, "cdt": cdt,
            },
            verdict=decision.verdict.value,
            score_used=decision.score_used,
            reason=decision.reason,
            expected_hint=expected_hint.value,
            matches_hint=matches,
        )
        report.cases.append(case)
        distribution[decision.verdict.value] += 1

    report.verdict_distribution = distribution

    # All 5 hints should match by construction; if any don't, surface partial
    if all(c.matches_hint for c in report.cases):
        report.status = "ok"
    else:
        report.status = "partial"

    return report


def _print_human(report: SmokeReport) -> None:
    print(f"V1085 HQB smoke v{report.version} (V1085={report.v1085_version})")
    print(f"thresholds: accept>={report.accept_threshold} reject<{report.reject_threshold}")
    print("-" * 70)
    for c in report.cases:
        flag = "✓" if c.matches_hint else "✗"
        print(
            f"  {flag} {c.label:25s} total={c.score_total:.3f} → "
            f"verdict={c.verdict:7s} (hint={c.expected_hint:7s})"
        )
    print("-" * 70)
    print(f"distribution: {report.verdict_distribution}")
    print(f"status: {report.status}")


# --- CLI ---------------------------------------------------------------------

def main(argv: Optional[List[str]] = None) -> int:
    parser = argparse.ArgumentParser(
        prog="v1085-hqb-smoke",
        description="V1085 HQB CLI smoke (post-V1358 stage delivery; V1359 plan item)",
    )
    parser.add_argument("--json", action="store_true", help="JSON output")
    parser.add_argument("--self-test", action="store_true", help="Run self-test checks")
    parser.add_argument("--version", action="store_true", help="Print version")
    args = parser.parse_args(argv)

    if args.version:
        print(f"v1085-hqb-smoke {V1085_SMOKE_VERSION}")
        return 0

    if args.self_test:
        # Run smoke + structural assertions (Popper-style)
        report = run_smoke()
        ok = True
        ok &= report.status == "ok"
        ok &= len(report.cases) == 5
        ok &= report.verdict_distribution.get(Verdict.REJECT.value, 0) >= 1
        ok &= report.verdict_distribution.get(Verdict.REVIEW.value, 0) >= 1
        ok &= report.verdict_distribution.get(Verdict.ACCEPT.value, 0) >= 1
        ok &= report.verdict_distribution.get(Verdict.VETO.value, 0) >= 1
        # All cases must carry a non-empty reason (主 17:43 实事求是)
        ok &= all(len(c.reason) > 0 for c in report.cases)
        # score_used must equal score_total for every case (V1085 contract)
        ok &= all(
            abs(c.score_used - c.score_total) < 1e-9 for c in report.cases
        )
        if args.json:
            print(json.dumps({"pass": ok, "report": report.to_dict()}, indent=2))
        else:
            print(f"V1085 smoke self-test: {'PASS' if ok else 'FAIL'}")
        return 0 if ok else 1

    # Default: run smoke + print
    report = run_smoke()
    if args.json:
        print(json.dumps(report.to_dict(), indent=2, ensure_ascii=False))
    else:
        _print_human(report)
    return 0 if report.status == "ok" else 2


if __name__ == "__main__":
    sys.exit(main())
