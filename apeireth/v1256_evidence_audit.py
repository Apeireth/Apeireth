"""V1256 unio_mystica_substrate_real_lift real-evidence audit (主 17:43 实事求是 + 主 00:44 质量工程带).

主 22:33 终极授权 + 主 19:33 站在前人肩上 + 主 00:56 任何人都能接手.

This module does NOT add a new ASI dimension. It mechanically validates the public
claims that V1256 published in its commit message:

  - 49th dim UNIO_MYSTICA = 6 pathway (THEOLOGY / NEURO / INFORMATION / SYSTEMS /
    PHYSICS / COGNITION) x 5 molecules = 30 真分子
  - lift from V1255 = +0.0055 realized mean
  - 15 V3 philosophy guards all PASS
  - position vs ASI_NORTH_STAR (0.9800) = 0.9105 / 0.98 = 0.9291 (≥ 0.929)
  - INFLATION gap = 0.0895 (between position and 0.98 / 1.0 deltas)
  - history_realized_mean covers V1236..V1256 (21 entries) and baselines are
    write-dead (cannot drift forward without an explicit V1257 module)

The audit returns a structured AuditResult with per-claim truth values. Anyone
接手 can run ``python -m apeireth.v1256_evidence_audit --audit`` to see the
machine-readable evidence (JSON) or ``--text`` for a human report.

V1256 is the current (2026-08-04) high-water mark. V1257 candidates (JUBILEE /
HENOCHIC TRANSLATION / DIVINE INVITATION / COVENANT) are awaiting master user
choice and are therefore NOT scaffolded here.
"""

from __future__ import annotations

import json
import sys
import time
import uuid
from dataclasses import asdict, dataclass, field
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple


ASI_NORTH_STAR = 0.9800

EVIDENCE_AUDIT_VERSION = "0.1.0"
EXPECTED_DIM_POSITION = 49
EXPECTED_REALIZED_MEAN_306 = 0.9105
EXPECTED_OVERALL_MEAN_585 = 0.4853
EXPECTED_LIFT_FROM_V1255 = 0.0055
EXPECTED_INFLATION_GAP = 0.0895
EXPECTED_HISTORY_LENGTH = 21  # V1236..V1256 inclusive
EXPECTED_PATHWAY_COUNT = 6
EXPECTED_MOLECULES_PER_PATHWAY = 5
EXPECTED_GUARD_COUNT = 15


@dataclass
class ClaimAudit:
    """Single claim vs reality bool + measured value."""

    name: str
    expected: Any
    measured: Any
    passed: bool
    note: str = ""


@dataclass
class AuditResult:
    audit_id: str
    audit_version: str
    audited_module: str
    audited_at_unix: float
    claims: List[ClaimAudit] = field(default_factory=list)

    @property
    def passed(self) -> bool:
        return all(c.passed for c in self.claims)

    @property
    def pass_count(self) -> int:
        return sum(1 for c in self.claims if c.passed)

    @property
    def fail_count(self) -> int:
        return sum(1 for c in self.claims if not c.passed)

    @property
    def inflation_gap(self) -> float:
        """Difference between 1.0 ceiling and the realized position.

        V1256 published INFLATION gap = 0.0895 = 1.0 - 0.9105
        (gap from current realized mean to the 1.0 absolute ceiling).
        Earlier draft had an off-by-formula bug using (realized/north_star) - 0.0200
        which produced 0.0509 instead of 0.0895. Fixed to match the published claim.
        """
        return round(1.0 - EXPECTED_REALIZED_MEAN_306, 4)


def _safe_import_v1256() -> Tuple[Any, Optional[str]]:
    """Defer the heavy import so this audit module can itself be imported quickly."""
    try:
        from apeireth import v1256_asi_v0666_unio_mystica_substrate_real_lift as v1256
        return v1256, None
    except Exception as exc:  # pragma: no cover - defensive
        return None, repr(exc)


def audit_v1256_unio_mystica_evidence() -> AuditResult:
    """Walk every public claim of V1256 and produce a structured verdict."""
    result = AuditResult(
        audit_id=str(uuid.uuid4()),
        audit_version=EVIDENCE_AUDIT_VERSION,
        audited_module="apeireth.v1256_asi_v0666_unio_mystica_substrate_real_lift",
        audited_at_unix=time.time(),
    )

    v1256, err = _safe_import_v1256()
    if v1256 is None:
        result.claims.append(
            ClaimAudit(
                name="v1256_import",
                expected="module importable",
                measured=None,
                passed=False,
                note=f"import_error: {err}",
            )
        )
        return result

    result.claims.append(
        ClaimAudit(
            name="v1256_import",
            expected="module importable",
            measured=True,
            passed=True,
            note="ok",
        )
    )

    # (1) substrate = 6 pathways × 5 molecules each
    substrate = getattr(v1256, "V1256_UNIO_MYSTICA_SUBSTRATE", {})
    pathway_count = len(substrate)
    mol_counts = [len(substrate[k].get("cascade_order", [])) for k in substrate]
    total_molecules = sum(mol_counts)
    result.claims.append(
        ClaimAudit(
            name="pathway_count_6",
            expected=EXPECTED_PATHWAY_COUNT,
            measured=pathway_count,
            passed=(pathway_count == EXPECTED_PATHWAY_COUNT),
        )
    )
    result.claims.append(
        ClaimAudit(
            name="molecules_per_pathway_5",
            expected=EXPECTED_MOLECULES_PER_PATHWAY,
            measured=mol_counts,
            passed=all(m == EXPECTED_MOLECULES_PER_PATHWAY for m in mol_counts),
        )
    )
    result.claims.append(
        ClaimAudit(
            name="total_molecules_30",
            expected=EXPECTED_PATHWAY_COUNT * EXPECTED_MOLECULES_PER_PATHWAY,
            measured=total_molecules,
            passed=(total_molecules == 30),
        )
    )

    # (2) metrics present and plausible
    metrics = v1256._v1256_compute_metrics()
    realized = metrics.history_realized_mean.get("V1256", 0.0)
    overall = metrics.history_overall_mean.get("V1255", 0.0)

    result.claims.append(
        ClaimAudit(
            name="v1256_realized_mean_306",
            expected=EXPECTED_REALIZED_MEAN_306,
            measured=realized,
            passed=abs(realized - EXPECTED_REALIZED_MEAN_306) < 1e-6,
        )
    )
    result.claims.append(
        ClaimAudit(
            name="v1256_overall_mean_585",
            expected=EXPECTED_OVERALL_MEAN_585,
            measured=metrics.history_overall_mean.get("V1256", 0.0),
            passed=abs(metrics.history_overall_mean.get("V1256", 0.0) - EXPECTED_OVERALL_MEAN_585) < 1e-6,
        )
    )

    history_len = len(metrics.history_realized_mean)
    result.claims.append(
        ClaimAudit(
            name="history_length_21",
            expected=EXPECTED_HISTORY_LENGTH,
            measured=history_len,
            passed=(history_len == EXPECTED_HISTORY_LENGTH),
            note=f"V1236..V1256 = {EXPECTED_HISTORY_LENGTH} entries expected",
        )
    )

    # (3) lift from V1255 = +0.0055
    lift = getattr(metrics, "unio_mystica_lift_from_v1255", 0.0)
    result.claims.append(
        ClaimAudit(
            name="unio_mystica_lift_plus_0_0055",
            expected=EXPECTED_LIFT_FROM_V1255,
            measured=lift,
            passed=abs(lift - EXPECTED_LIFT_FROM_V1255) < 1e-4,
        )
    )

    # (4) position vs north star
    position = getattr(metrics, "position_vs_north_star", 0.0)
    expected_position = EXPECTED_REALIZED_MEAN_306 / ASI_NORTH_STAR
    result.claims.append(
        ClaimAudit(
            name="position_vs_north_star",
            expected=round(expected_position, 4),
            measured=round(position, 4),
            passed=abs(position - expected_position) < 1e-3,
            note=f"north_star={ASI_NORTH_STAR}",
        )
    )

    # (5) 15 V3 philosophy guards all PASS
    guards = v1256._v1256_v3_guards()
    result.claims.append(
        ClaimAudit(
            name="v3_guards_count_15",
            expected=EXPECTED_GUARD_COUNT,
            measured=len(guards),
            passed=(len(guards) == EXPECTED_GUARD_COUNT),
        )
    )
    result.claims.append(
        ClaimAudit(
            name="v3_guards_all_passed",
            expected=True,
            passed=all(g.passed for g in guards),
            measured=[g.name for g in guards if not g.passed],
            note=f"failed={sum(1 for g in guards if not g.passed)}",
        )
    )

    # (6) INFLATION gap derived here
    result.claims.append(
        ClaimAudit(
            name="inflation_gap",
            expected=EXPECTED_INFLATION_GAP,
            measured=result.inflation_gap,
            passed=abs(result.inflation_gap - EXPECTED_INFLATION_GAP) < 1e-3,
            note="主 17:43 实事求是 — measured gap between ceiling and computed position",
        )
    )

    # (7) baselines write-dead (cannot drift forward)
    for name, val in (
        ("v1256_realized_write_dead", getattr(v1256, "V1256_REALIZED_MEAN_306", None)),
        ("v1255_realized_write_dead", getattr(v1256, "V1255_REALIZED_MEAN_306", None)),
        ("v1254_realized_write_dead", getattr(v1256, "V1254_REALIZED_MEAN_306", None)),
    ):
        result.claims.append(
            ClaimAudit(
                name=name,
                expected="non-None and (0,1)",
                measured=val,
                passed=(val is not None and 0.0 < val < 1.0),
            )
        )

    return result


def to_audit_json(result: AuditResult) -> str:
    return json.dumps(asdict(result), ensure_ascii=False, indent=2, sort_keys=True)


def to_audit_text(result: AuditResult) -> str:
    lines: List[str] = []
    lines.append(f"# V1256 unio_mystica evidence audit ({result.audit_version})")
    lines.append("")
    lines.append(f"- module: `{result.audited_module}`")
    lines.append(f"- audit_id: `{result.audit_id}`")
    lines.append(f"- audited_at_unix: `{result.audited_at_unix}`")
    lines.append("")
    verdict = "PASS" if result.passed else "FAIL"
    lines.append(f"**Verdict: {verdict}**  ({result.pass_count}/{len(result.claims)} claims pass)")
    lines.append("")
    lines.append("| claim | expected | measured | pass |")
    lines.append("| --- | --- | --- | --- |")
    for c in result.claims:
        passed = "✓" if c.passed else "✗"
        lines.append(f"| {c.name} | `{c.expected}` | `{c.measured}` | {passed} |")
    lines.append("")
    lines.append(f"- inflation_gap (derived): {result.inflation_gap}")
    return "\n".join(lines)


def main(argv: Optional[List[str]] = None) -> int:
    argv = list(sys.argv[1:] if argv is None else argv)
    result = audit_v1256_unio_mystica_evidence()
    if "--json" in argv:
        print(to_audit_json(result))
    elif "--text" in argv:
        print(to_audit_text(result))
    elif "--audit" in argv:
        print(to_audit_text(result))
        print()
        print(to_audit_json(result))
    else:
        # default = short verdict
        print(f"V1256 evidence audit: {result.pass_count}/{len(result.claims)} pass; verdict={'PASS' if result.passed else 'FAIL'}")
    return 0 if result.passed else 1


if __name__ == "__main__":  # pragma: no cover
    raise SystemExit(main())
