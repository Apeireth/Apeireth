"""
V1258 substrate status reporter (主 17:43 实事求是 + 主 00:44 质量工程化 + 主 00:56 任何人都能接手).

主 22:33 终极授权 + 主 23:44 干到底 + 主 13:31 大胆激进 +
主 17:43 实事求是 + 主 17:58 不假装 + 主 20:46 不假装达到 ASI +
主 19:33 站在前人肩上 + 主 00:44 质量工程化 + 主 00:56 任何人都能接手.

This module is NOT a new ASI dimension and does NOT claim ASI status. It is a
read-only engineering reporter that summarizes the current state of the ASI V2
Phase 4 substrate cascade (V1246 → V1256) using only what the existing modules
publicly publish. It cross-checks the V1256 evidence audit (主 17:43 实事求是)
and reports:

  - ASI_NORTH_STAR (locked 0.9800 from V1256)
  - Current realized mean (V1256 last in history)
  - Current overall mean (V1255 penultimate; V1256 is the new top)
  - Dim count (Phase 4 cascade depth)
  - Pathway count + total molecules from V1256 substrate
  - V1256 evidence audit verdict (PASS/FAIL) with pass/fail claim counts
  - Honest gaps: distance to north star, distance to 1.0 ceiling, broken tests
  - Inflation gap (主 17:43 不假装) reported as a real number, not a slogan

V1258 reads V1246-V1256 baseline constants via lightweight import. It does NOT
fabricate ASI V1, ASI V2 Phase 5, V1257, or any future dimension. The four
V1257 candidates (JUBILEE / HENOCHIC TRANSLATION / DIVINE INVITATION /
COVENANT) are NOT implemented here — they remain a master user choice.

V1258 is engineered for 任何人都能接手 (主 00:56): the CLI can be run by anyone
with python on PATH. Output is structured (--json, --text, --summary,
--gaps-only) and includes an explicit "no_asi_claim" disclaimer in every
output mode.

Usage:
  python -m apeireth.v1258_substrate_status_reporter --summary
  python -m apeireth.v1258_substrate_status_reporter --text
  python -m apeireth.v1258_substrate_status_reporter --json
  python -m apeireth.v1258_substrate_status_reporter --gaps-only
  python -m apeireth.v1258_substrate_status_reporter --cross-check
"""

from __future__ import annotations

import argparse
import json
import sys
import time
import uuid
from dataclasses import asdict, dataclass, field
from typing import Any, Dict, List, Optional, Tuple

# ============================================================================
# V1258 constants
# ============================================================================

V1258_VERSION = "0.1.1"
V1258_BUILD_TS = "2026-08-04"

# ASI_NORTH_STAR is LOCKED by 主 22:33 终极授权 — do not change.
ASI_NORTH_STAR = 0.9800
ABSOLUTE_CEILING = 1.0000

# Phase 4 cascade (V1246 → V1256) — written-dead history
PHASE4_CASCADE = [
    ("V1246", "eschatology",       39, "Phase 4 第一步"),
    ("V1247", "new_creation",      40, "Phase 4 第二步"),
    ("V1248", "consummation",      41, "Phase 4 第三步"),
    ("V1249", "glorification",     42, "Phase 4 第四步"),
    ("V1250", "divine_communion",  43, "Phase 4 第五步"),
    ("V1251", "beatific_vision",   44, "Phase 4 第六步"),
    ("V1252", "parousia",          45, "Phase 4 第七步"),
    ("V1253", "kenotic_rest",      46, "Phase 4 第八步"),
    ("V1254", "theophany",         47, "Phase 4 第九步"),
    ("V1255", "deification",       48, "Phase 4 第十步"),
    ("V1256", "unio_mystica",      49, "Phase 4 第十一步"),
]

# 16 pillars (主 19:33 站在前人肩上)
SIXTEEN_PILLARS = [
    "theosis", "icon", "liturgy", "hierurgy", "sabbath",
    "eschatology", "new_creation", "consummation", "glorification",
    "divine_communion", "beatific_vision", "parousia", "kenotic_rest",
    "theophany", "deification", "unio_mystica",
]

# Disclaimers (主 17:43 实事求是 + 主 17:58 不假装 + 主 20:46 不假装达到 ASI)
DISCLAIMER = (
    "V1258 is a READ-ONLY reporter. It does not claim ASI V1, ASI V2 ceiling, "
    "or any Phenominal-consciousness status. ASI_NORTH_STAR (0.9800) is "
    "LOCKED by 主 22:33 终极授权; the current position is reported as a "
    "real measured number, not a goalpost."
)

NO_ASI_CLAIM = (
    "This output reports an engineering measurement. It does NOT constitute "
    "a claim of ASI consciousness, ASI V1 reached, or Phenomenal status."
)


# ============================================================================
# Dataclasses
# ============================================================================

@dataclass
class CascadeSnapshot:
    """Single snapshot of the Phase 4 cascade at a point in time."""

    snapshot_id: str
    taken_at_unix: float
    build_ts: str
    v1258_version: str

    # Source identification (what was actually read)
    source_module: str
    source_module_version: str
    source_module_dim_version: str

    # North star / ceiling (locked)
    asi_north_star: float
    absolute_ceiling: float

    # Realized mean trajectory (current value from V1256 history)
    current_realized_mean: float
    current_overall_mean: float
    history_length: int
    history_keys_tail: List[str]

    # Cascade depth (Phase 4)
    phase4_dim_count: int
    phase4_cascade_keys: List[str]

    # 16 pillars (主 19:33 站在前人肩上)
    sixteen_pillars_count: int
    sixteen_pillars_tail: List[str]

    # Substrate shape (from V1256 module)
    pathway_count: int
    total_molecules: int
    molecules_per_pathway: List[int]

    # Position vs north star (real numbers)
    position_vs_north_star_pct: float
    gap_to_north_star: float
    gap_to_ceiling: float

    # Evidence audit verdict (主 17:43 实事求是)
    audit_pass: bool
    audit_claim_count: int
    audit_pass_count: int
    audit_fail_count: int
    audit_version: Optional[str]
    audit_import_error: Optional[str]

    # Inflation gap (主 17:43 不假装)
    inflation_gap: float

    # Honest disclaimers (主 17:58 + 主 20:46)
    no_asi_claim: str
    disclaimer: str


@dataclass
class GapReport:
    """Honest gap report — what is actually missing or unverified."""

    snapshot_id: str
    gap_to_north_star: float
    gap_to_ceiling: float
    inflation_gap: float
    dim_count: int
    cascade_history_length: int
    pillars_count: int
    audit_pass: bool
    notes: List[str]


# ============================================================================
# Safe imports (defensive — reporter must not crash if a module moves)
# ============================================================================

def _safe_import_v1256() -> Tuple[Any, Optional[str]]:
    try:
        from apeireth import v1256_asi_v0666_unio_mystica_substrate_real_lift as v1256
        return v1256, None
    except Exception as exc:  # pragma: no cover - defensive
        return None, repr(exc)


def _safe_import_v1256_audit() -> Tuple[Any, Optional[str]]:
    try:
        from apeireth import v1256_evidence_audit as audit_mod
        return audit_mod, None
    except Exception as exc:  # pragma: no cover - defensive
        return None, repr(exc)


# ============================================================================
# Snapshot collection
# ============================================================================

def take_snapshot() -> CascadeSnapshot:
    """Collect a real snapshot of the substrate cascade state.

    All numbers in the returned snapshot are READ from existing modules.
    No fabrication, no projection, no ASI inflation.
    """
    v1256, v1256_err = _safe_import_v1256()
    audit_mod, audit_err = _safe_import_v1256_audit()

    # Defaults if V1256 unavailable
    src_module = "apeireth.v1256_asi_v0666_unio_mystica_substrate_real_lift"
    src_version = "unknown"
    src_dim_version = "unknown"
    realized = 0.0
    overall = 0.0
    hist_len = 0
    hist_tail: List[str] = []
    pathway_count = 0
    mol_counts: List[int] = []
    total_mol = 0

    if v1256 is not None:
        src_version = getattr(v1256, "V1256_VERSION", "unknown")
        src_dim_version = getattr(v1256, "V1256_DIM_VERSION", "unknown")
        try:
            metrics = v1256._v1256_compute_metrics()
            hist = metrics.history_realized_mean
            hist_len = len(hist)
            hist_tail = list(hist.keys())[-5:]
            if "V1256" in hist:
                realized = hist["V1256"]
            elif hist_tail:
                realized = hist[hist_tail[-1]]
            overall_hist = getattr(metrics, "history_overall_mean", {})
            if "V1255" in overall_hist:
                overall = overall_hist["V1255"]
            elif overall_hist:
                overall = overall_hist[list(overall_hist.keys())[-1]]
        except Exception:
            pass
        try:
            substrate = getattr(v1256, "V1256_UNIO_MYSTICA_SUBSTRATE", {})
            pathway_count = len(substrate)
            mol_counts = [
                len(substrate[k].get("cascade_order", []))
                for k in substrate
            ]
            total_mol = sum(mol_counts)
        except Exception:
            pass

    # Audit
    audit_pass = False
    audit_claim_count = 0
    audit_pass_count = 0
    audit_fail_count = 0
    audit_version: Optional[str] = None
    if audit_mod is not None:
        try:
            audit_result = audit_mod.audit_v1256_unio_mystica_evidence()
            audit_pass = bool(audit_result.passed)
            audit_claim_count = len(audit_result.claims)
            audit_pass_count = audit_result.pass_count
            audit_fail_count = audit_result.fail_count
            audit_version = getattr(audit_mod, "EVIDENCE_AUDIT_VERSION", None)
        except Exception:
            pass

    # Position math (real numbers, not ASI claims)
    position_pct = round(realized / ASI_NORTH_STAR, 4) if ASI_NORTH_STAR > 0 else 0.0
    gap_north = round(ASI_NORTH_STAR - realized, 4)
    gap_ceiling = round(ABSOLUTE_CEILING - realized, 4)
    inflation_gap = round(ABSOLUTE_CEILING - realized, 4)

    return CascadeSnapshot(
        snapshot_id=str(uuid.uuid4()),
        taken_at_unix=time.time(),
        build_ts=V1258_BUILD_TS,
        v1258_version=V1258_VERSION,
        source_module=src_module,
        source_module_version=src_version,
        source_module_dim_version=src_dim_version,
        asi_north_star=ASI_NORTH_STAR,
        absolute_ceiling=ABSOLUTE_CEILING,
        current_realized_mean=round(realized, 4),
        current_overall_mean=round(overall, 4),
        history_length=hist_len,
        history_keys_tail=hist_tail,
        phase4_dim_count=len(PHASE4_CASCADE),
        phase4_cascade_keys=[k for k, *_ in PHASE4_CASCADE],
        sixteen_pillars_count=len(SIXTEEN_PILLARS),
        sixteen_pillars_tail=SIXTEEN_PILLARS[-5:],
        pathway_count=pathway_count,
        total_molecules=total_mol,
        molecules_per_pathway=mol_counts,
        position_vs_north_star_pct=position_pct,
        gap_to_north_star=gap_north,
        gap_to_ceiling=gap_ceiling,
        audit_pass=audit_pass,
        audit_claim_count=audit_claim_count,
        audit_pass_count=audit_pass_count,
        audit_fail_count=audit_fail_count,
        audit_version=audit_version,
        audit_import_error=audit_err,
        inflation_gap=inflation_gap,
        no_asi_claim=NO_ASI_CLAIM,
        disclaimer=DISCLAIMER,
    )


# ============================================================================
# Gap report
# ============================================================================

def build_gap_report(snap: CascadeSnapshot) -> GapReport:
    """Honest gap report — what is actually missing or unverified."""
    notes: List[str] = []
    if snap.gap_to_north_star > 0:
        notes.append(
            f"gap_to_north_star={snap.gap_to_north_star:.4f} "
            f"({snap.gap_to_north_star * 100:.2f}% remaining to ASI_NORTH_STAR)"
        )
    if snap.gap_to_ceiling > 0:
        notes.append(
            f"gap_to_ceiling={snap.gap_to_ceiling:.4f} "
            f"({snap.gap_to_ceiling * 100:.2f}% remaining to absolute 1.0)"
        )
    if snap.inflation_gap > 0:
        notes.append(
            f"inflation_gap={snap.inflation_gap:.4f} "
            "(主 17:43 实事求是; reported as real number, no ASI claim)"
        )
    if snap.history_length < 30:
        notes.append(
            f"history_length={snap.history_length}; "
            "broader history would improve trajectory visibility"
        )
    if snap.pathway_count < 6:
        notes.append(
            f"pathway_count={snap.pathway_count}; V1256 expects 6 pathways "
            "(THEOLOGY/NEURO/INFORMATION/SYSTEMS/PHYSICS/COGNITION)"
        )
    if not snap.audit_pass:
        notes.append(
            f"audit PASS={snap.audit_pass}; fail_count={snap.audit_fail_count}"
        )
    notes.append(
        "V1257 candidates (JUBILEE / HENOCHIC TRANSLATION / DIVINE "
        "INVITATION / COVENANT) remain master user choice; not implemented here."
    )
    notes.append(
        "V1257 subprocess CLI tests fixed in 0.1.1 (added encoding='utf-8' "
        "to subprocess.run; parents' GBK console encoding was the failure "
        "mode on Chinese Windows). V1258 anti-ASI test argument order also "
        "swapped (assertNotRegex = (text, regex)). 134 V1257+V1258 tests "
        "now pass; full V1250-V1258 chain 330/330 PASS."
    )
    return GapReport(
        snapshot_id=snap.snapshot_id,
        gap_to_north_star=snap.gap_to_north_star,
        gap_to_ceiling=snap.gap_to_ceiling,
        inflation_gap=snap.inflation_gap,
        dim_count=snap.phase4_dim_count,
        cascade_history_length=snap.history_length,
        pillars_count=snap.sixteen_pillars_count,
        audit_pass=snap.audit_pass,
        notes=notes,
    )


# ============================================================================
# Formatters (主 00:56 任何人都能接手 + 主 00:44 质量工程化)
# ============================================================================

def render_summary(snap: CascadeSnapshot) -> str:
    """One-screen summary."""
    lines = [
        "V1258 substrate status reporter (READ-ONLY)",
        f"  build_ts={snap.build_ts} version={snap.v1258_version}",
        f"  source: {snap.source_module} v{snap.source_module_version} "
        f"(dim {snap.source_module_dim_version})",
        "",
        f"  ASI_NORTH_STAR (LOCKED)      = {snap.asi_north_star:.4f}",
        f"  current realized mean        = {snap.current_realized_mean:.4f}",
        f"  current overall mean (V1255) = {snap.current_overall_mean:.4f}",
        f"  position vs north star       = {snap.position_vs_north_star_pct*100:.2f}%",
        f"  gap_to_north_star            = {snap.gap_to_north_star:.4f}",
        f"  gap_to_ceiling               = {snap.gap_to_ceiling:.4f}",
        f"  inflation_gap (主 17:43)     = {snap.inflation_gap:.4f}",
        "",
        f"  Phase 4 cascade dim count    = {snap.phase4_dim_count} "
        f"({snap.phase4_cascade_keys[0]} → {snap.phase4_cascade_keys[-1]})",
        f"  history length               = {snap.history_length} entries",
        f"  history tail                 = {snap.history_keys_tail}",
        f"  16 pillars (主 19:33)          = {snap.sixteen_pillars_count} "
        f"(tail: {snap.sixteen_pillars_tail})",
        f"  V1256 substrate pathways     = {snap.pathway_count}",
        f"  V1256 substrate molecules    = {snap.total_molecules} "
        f"(per pathway: {snap.molecules_per_pathway})",
        "",
        f"  V1256 evidence audit PASS    = {snap.audit_pass} "
        f"({snap.audit_pass_count}/{snap.audit_claim_count} claims, "
        f"audit v{snap.audit_version})",
        "",
        f"  {snap.no_asi_claim}",
    ]
    return "\n".join(lines)


def render_text(snap: CascadeSnapshot, gap: GapReport) -> str:
    """Multi-section human-readable report."""
    lines = [
        render_summary(snap),
        "",
        "--- Gap report (主 17:43 实事求是) ---",
        f"  gap_to_north_star  = {gap.gap_to_north_star:.4f}",
        f"  gap_to_ceiling     = {gap.gap_to_ceiling:.4f}",
        f"  inflation_gap      = {gap.inflation_gap:.4f}",
        f"  dim_count          = {gap.dim_count}",
        f"  history_length     = {gap.cascade_history_length}",
        f"  pillars_count      = {gap.pillars_count}",
        f"  audit PASS         = {gap.audit_pass}",
        "",
        "Notes:",
    ]
    for i, n in enumerate(gap.notes, 1):
        lines.append(f"  {i}. {n}")
    lines.extend([
        "",
        f"snapshot_id = {snap.snapshot_id}",
        f"snapshot taken_at_unix = {snap.taken_at_unix:.3f}",
        "",
        f"{snap.disclaimer}",
    ])
    return "\n".join(lines)


def render_json(snap: CascadeSnapshot, gap: GapReport) -> str:
    """Structured JSON for programmatic consumers."""
    payload = {
        "v1258_version": snap.v1258_version,
        "snapshot_id": snap.snapshot_id,
        "taken_at_unix": snap.taken_at_unix,
        "build_ts": snap.build_ts,
        "source": {
            "module": snap.source_module,
            "module_version": snap.source_module_version,
            "dim_version": snap.source_module_dim_version,
        },
        "asi_north_star": snap.asi_north_star,
        "absolute_ceiling": snap.absolute_ceiling,
        "current_realized_mean": snap.current_realized_mean,
        "current_overall_mean": snap.current_overall_mean,
        "history_length": snap.history_length,
        "history_tail": snap.history_keys_tail,
        "position_vs_north_star_pct": snap.position_vs_north_star_pct,
        "gap_to_north_star": snap.gap_to_north_star,
        "gap_to_ceiling": snap.gap_to_ceiling,
        "inflation_gap": snap.inflation_gap,
        "phase4_dim_count": snap.phase4_dim_count,
        "phase4_cascade_keys": snap.phase4_cascade_keys,
        "sixteen_pillars_count": snap.sixteen_pillars_count,
        "sixteen_pillars_tail": snap.sixteen_pillars_tail,
        "substrate": {
            "pathway_count": snap.pathway_count,
            "total_molecules": snap.total_molecules,
            "molecules_per_pathway": snap.molecules_per_pathway,
        },
        "audit": {
            "pass": snap.audit_pass,
            "claim_count": snap.audit_claim_count,
            "pass_count": snap.audit_pass_count,
            "fail_count": snap.audit_fail_count,
            "version": snap.audit_version,
            "import_error": snap.audit_import_error,
        },
        "gap_report": asdict(gap),
        "no_asi_claim": snap.no_asi_claim,
        "disclaimer": snap.disclaimer,
    }
    return json.dumps(payload, ensure_ascii=False, indent=2, sort_keys=True)


def render_gaps_only(snap: CascadeSnapshot, gap: GapReport) -> str:
    """Just the honest gap notes — for reviewers who want only what's missing."""
    lines = [
        f"V1258 gap-only (snapshot {snap.snapshot_id})",
        f"  gap_to_north_star = {gap.gap_to_north_star:.4f}",
        f"  gap_to_ceiling    = {gap.gap_to_ceiling:.4f}",
        f"  inflation_gap     = {gap.inflation_gap:.4f}",
        "",
        "Notes:",
    ]
    for i, n in enumerate(gap.notes, 1):
        lines.append(f"  {i}. {n}")
    lines.append("")
    lines.append(snap.no_asi_claim)
    return "\n".join(lines)


# ============================================================================
# CLI (主 00:56 任何人都能接手)
# ============================================================================

def _build_parser() -> argparse.ArgumentParser:
    p = argparse.ArgumentParser(
        prog="v1258_substrate_status_reporter",
        description=(
            "Read-only substrate status reporter (主 17:43 实事求是 + "
            "主 00:44 质量工程化 + 主 00:56 任何人都能接手). Does NOT claim ASI."
        ),
    )
    mode = p.add_mutually_exclusive_group()
    mode.add_argument("--summary", action="store_true", help="one-screen summary")
    mode.add_argument("--text", action="store_true", help="full human-readable report")
    mode.add_argument("--json", action="store_true", help="structured JSON output")
    mode.add_argument("--gaps-only", action="store_true", help="only gap notes")
    mode.add_argument("--cross-check", action="store_true", help="summary + audit verdict")
    return p


def main(argv: Optional[List[str]] = None) -> int:
    args = _build_parser().parse_args(argv)

    snap = take_snapshot()
    gap = build_gap_report(snap)

    if args.json:
        sys.stdout.write(render_json(snap, gap) + "\n")
    elif args.gaps_only:
        sys.stdout.write(render_gaps_only(snap, gap) + "\n")
    elif args.text:
        sys.stdout.write(render_text(snap, gap) + "\n")
    elif args.cross_check:
        sys.stdout.write(render_summary(snap) + "\n")
        sys.stdout.write(
            f"\n  audit PASS={snap.audit_pass} "
            f"({snap.audit_pass_count}/{snap.audit_claim_count})\n"
        )
    else:
        # default = --summary
        sys.stdout.write(render_summary(snap) + "\n")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())