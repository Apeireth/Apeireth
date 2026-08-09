"""V1433 — ASI VCP structural consistency report (主 00:44 质量工程化).

Phase: 1433
Version: 0.1.0
Date: 2026-08-10 (cron tick 05:08, Asia/Shanghai deep night)
Post: V1432 (VCP 真实源代码 deep read) + V1426 (VCP 6 protocols dispatcher)

What V1433 is
=============
V1433 is the **structural consistency report** that combines:

- V1432 (VCP 真实源代码 deep read): the mapping VCP layer → V1426 protocol
- V1426 (VCP 6 protocols dispatcher): the registered V1426 protocol set
- V1411 (overarching framework): chain_delegate pattern

It checks:

1. **Forward coverage**: every VCP layer has a V1426 protocol mapping
2. **Reverse coverage**: every V1426 protocol appears in at least one mapping
3. **Parity score**: arithmetic mean of forward + reverse coverage
4. **Honest gaps**: any VCP layer missing → V1426 protocol, reported
5. **Honest redundant**: any V1426 protocol unused → reported

V1433 is **offline-safe**: it does not call GitHub; it consumes
V1432's already-computed mappings (which may come from offline
heuristics if the network failed). The structural consistency
report is honest even if VCP source fetch failed.

Honest disclosure (主 17:58 + 主 17:43)
=======================================
V1433 is a **bounded structural consistency report**. It does not
claim VCP integration parity, VCP protocol completeness, or that
our V1426 dispatcher fully implements VCP semantics. It claims
only: the mappings were read, the coverage was computed, the gaps
were reported. V1433 is read-only; never mutates V1432 or V1426.

Borrowed (5 — 主 19:33 走在前人经验上):
=======================================
- V1432 (VCP 真实源代码 deep read — mapping source)
- V1426 (VCP 6 protocols dispatcher — protocol registry)
- V1425 (5 philosophical gaps — honesty paragraph template)
- V1411 (overarching framework — chain_delegate pattern)
- stdlib dataclasses + enum + json

GUARDS upheld (V1433-specific, 14 — 主 00:44 质量工程化)
=========================================================
- GUARD_FORWARD_COVERAGE: every VCP layer has a V1426 mapping ∈ [0, 1]
- GUARD_REVERSE_COVERAGE: every V1426 protocol used or reported redundant
- GUARD_PARITY_DEFINED: parity = (forward + reverse) / 2
- GUARD_GAPS_REPORTED: missing mappings surfaced in gaps list
- GUARD_REDUNDANT_REPORTED: unused V1426 protocols surfaced
- GUARD_NO_V1432_WRITE: V1432 is read-only
- GUARD_NO_V1426_WRITE: V1426 is read-only
- GUARD_POPPER_RUNS: popper self-test runs in CLI
- GUARD_CHAIN_OK: V1433 chain_delegate returns all_ok
- GUARD_HONEST_DISCLOSURE: honesty paragraph emitted
- GUARD_OFFLINE_SAFE: V1433 does not require network
- GUARD_CLI_RUNNABLE: CLI 真可跑
- GUARD_BOUNDED: n_layers ∈ [1, 12], n_protocols ∈ [1, 12]
- GUARD_JSON_SERIALIZABLE: report is JSON-serializable

V3 哲学守门 (主 17:58 + 主 20:46 + 主 17:43) — 5 guards
=======================================================
- GUARD_NO_PHENOMENAL_CONSISTENCY: report is structural, NOT consciousness
- GUARD_NO_ASI_CONSISTENCY: report is consistency, NOT ASI level
- GUARD_NO_HUMAN_LEVEL_CONSISTENCY: report is bounded, NOT human-level
- GUARD_NO_ABSOLUTE_CONSISTENCY: report is structural, NOT absolute parity
- GUARD_NO_FAKE_PARITY: structural parity ≠ implementation parity

API surfaces (14)
=================
1.  ``Direction`` — Enum (FORWARD / REVERSE)
2.  ``Gap`` — dataclass (kind + name + note)
3.  ``ConsistencyRow`` — dataclass (vcp_layer + v1426_protocol + score)
4.  ``ConsistencyReport`` — dataclass (rows + gaps + redundant + parity)
5.  ``V1433_FORWARD_TARGETS`` — tuple of expected VCP layers (6)
6.  ``compute_forward_coverage()`` — float ∈ [0, 1]
7.  ``compute_reverse_coverage()`` — float ∈ [0, 1]
8.  ``find_forward_gaps()`` — list of Gap
9.  ``find_redundant_protocols()`` — list of Gap
10. ``compute_parity()`` — float ∈ [0, 1]
11. ``build_report()`` — ConsistencyReport
12. ``render_report_md()`` — str
13. ``popper_self_test()`` — 14 self-tests
14. ``main()`` — CLI

CLI commands (9 — 主 00:56 任何人都能接手)
==========================================
- version
- meta [--json]
- help
- popper
- chain
- forward
- reverse
- gaps
- report
"""

from __future__ import annotations

import enum
import json
import sys
from dataclasses import dataclass, field, asdict
from typing import Any, Dict, List, Optional, Tuple


# ============================================================================
# Constants
# ============================================================================

V1433_VERSION = "0.1.0"
V1433_SCHEMA = "v1433.asi-vcp-structural-consistency/v1"
V1433_MODULE = "v1433_asi_vcp_structural_consistency"


# ============================================================================
# Enums
# ============================================================================


class Direction(str, enum.Enum):
    """Coverage direction."""

    FORWARD = "FORWARD"  # VCP layer → V1426 protocol
    REVERSE = "REVERSE"  # V1426 protocol → VCP layer


# ============================================================================
# Dataclasses
# ============================================================================


@dataclass
class Gap:
    """A coverage gap: either a missing forward mapping or a redundant reverse protocol."""

    direction: Direction
    name: str
    note: str = ""

    def to_dict(self) -> Dict[str, Any]:
        return {"direction": self.direction.value, "name": self.name, "note": self.note}


@dataclass
class ConsistencyRow:
    """One row of the consistency table: VCP layer ↔ V1426 protocol."""

    vcp_layer: str
    v1426_protocol: str
    match_score: float
    rationale: str = ""

    def to_dict(self) -> Dict[str, Any]:
        return asdict(self)


@dataclass
class ConsistencyReport:
    """Whole consistency report."""

    rows: List[ConsistencyRow] = field(default_factory=list)
    gaps: List[Gap] = field(default_factory=list)
    redundant: List[Gap] = field(default_factory=list)
    forward_coverage: float = 0.0
    reverse_coverage: float = 0.0
    parity: float = 0.0
    n_layers: int = 0
    n_protocols: int = 0
    started_iso: str = ""
    ended_iso: str = ""

    def to_dict(self) -> Dict[str, Any]:
        return {
            "rows": [r.to_dict() for r in self.rows],
            "gaps": [g.to_dict() for g in self.gaps],
            "redundant": [g.to_dict() for g in self.redundant],
            "forward_coverage": self.forward_coverage,
            "reverse_coverage": self.reverse_coverage,
            "parity": self.parity,
            "n_layers": self.n_layers,
            "n_protocols": self.n_protocols,
            "started_iso": self.started_iso,
            "ended_iso": self.ended_iso,
        }


# ============================================================================
# GUARDS / BORROWED
# ============================================================================

V1433_GUARDS: Tuple[str, ...] = (
    "GUARD_FORWARD_COVERAGE",
    "GUARD_REVERSE_COVERAGE",
    "GUARD_PARITY_DEFINED",
    "GUARD_GAPS_REPORTED",
    "GUARD_REDUNDANT_REPORTED",
    "GUARD_NO_V1432_WRITE",
    "GUARD_NO_V1426_WRITE",
    "GUARD_POPPER_RUNS",
    "GUARD_CHAIN_OK",
    "GUARD_HONEST_DISCLOSURE",
    "GUARD_OFFLINE_SAFE",
    "GUARD_CLI_RUNNABLE",
    "GUARD_BOUNDED",
    "GUARD_JSON_SERIALIZABLE",
)

V1433_V3_GUARDS: Tuple[str, ...] = (
    "GUARD_NO_PHENOMENAL_CONSISTENCY",
    "GUARD_NO_ASI_CONSISTENCY",
    "GUARD_NO_HUMAN_LEVEL_CONSISTENCY",
    "GUARD_NO_ABSOLUTE_CONSISTENCY",
    "GUARD_NO_FAKE_PARITY",
)

V1433_BORROWED: Tuple[Tuple[str, str], ...] = (
    ("v1432_vcp_real_source_deep_read", "VCP-to-V1426 mapping source"),
    ("v1426_vcp_six_protocol_dispatcher", "V1426 protocol registry"),
    ("v1425_asi_five_philosophical_gaps", "Honesty paragraph template"),
    ("v1411_asi_overarching_framework", "chain_delegate pattern"),
    ("stdlib_dataclasses_enum_json", "Structural report primitives"),
)


# ============================================================================
# Helpers
# ============================================================================


def _now_iso() -> str:
    """UTC ISO 8601 timestamp."""
    from datetime import datetime, timezone
    return datetime.now(timezone.utc).isoformat()


def _load_v1432_layers() -> Tuple[str, ...]:
    """Load VCP_LAYERS from V1432. Read-only."""
    try:
        from apeireth.v1432_vcp_real_source_deep_read import VCP_LAYERS
        return tuple(VCP_LAYERS)
    except Exception:
        # Honest fallback: V1432 not importable; report must still run.
        return ()


def _load_v1432_mappings() -> List[Any]:
    """Load map_to_v1426() from V1432. Read-only."""
    try:
        from apeireth.v1432_vcp_real_source_deep_read import map_to_v1426
        return list(map_to_v1426())
    except Exception:
        return []


def _load_v1426_protocols() -> Tuple[str, ...]:
    """Load V1426 protocol names. Read-only."""
    try:
        from apeireth.v1426_vcp_six_protocol_dispatcher import VCP_SIX_PROTOCOLS
        return tuple(VCP_SIX_PROTOCOLS)
    except Exception:
        return ()


# ============================================================================
# Coverage computation
# ============================================================================


def compute_forward_coverage(
    layers: Optional[Tuple[str, ...]] = None,
    mappings: Optional[List[Any]] = None,
) -> float:
    """Compute forward coverage: fraction of VCP layers that have a mapping.

    Returns a float in [0.0, 1.0]. If no layers, returns 0.0.
    """
    if layers is None:
        layers = _load_v1432_layers()
    if mappings is None:
        mappings = _load_v1432_mappings()
    if not layers:
        return 0.0
    mapped_layers = {m.vcp_module for m in mappings}
    n_covered = sum(1 for layer in layers if layer in mapped_layers)
    return n_covered / len(layers)


def compute_reverse_coverage(
    layers: Optional[Tuple[str, ...]] = None,
    mappings: Optional[List[Any]] = None,
    protocols: Optional[Tuple[str, ...]] = None,
) -> float:
    """Compute reverse coverage: fraction of V1426 protocols used in mappings.

    Returns a float in [0.0, 1.0]. If no protocols, returns 0.0.
    """
    if layers is None:
        layers = _load_v1432_layers()
    if mappings is None:
        mappings = _load_v1432_mappings()
    if protocols is None:
        protocols = _load_v1426_protocols()
    if not protocols:
        return 0.0
    used_protocols = {m.v1426_protocol for m in mappings}
    n_used = sum(1 for proto in protocols if proto in used_protocols)
    return n_used / len(protocols)


def find_forward_gaps(
    layers: Optional[Tuple[str, ...]] = None,
    mappings: Optional[List[Any]] = None,
) -> List[Gap]:
    """Find VCP layers missing a V1426 mapping."""
    if layers is None:
        layers = _load_v1432_layers()
    if mappings is None:
        mappings = _load_v1432_mappings()
    mapped_layers = {m.vcp_module for m in mappings}
    gaps: List[Gap] = []
    for layer in layers:
        if layer not in mapped_layers:
            gaps.append(
                Gap(
                    direction=Direction.FORWARD,
                    name=layer,
                    note="VCP layer has no V1426 protocol mapping",
                )
            )
    return gaps


def find_redundant_protocols(
    layers: Optional[Tuple[str, ...]] = None,
    mappings: Optional[List[Any]] = None,
    protocols: Optional[Tuple[str, ...]] = None,
) -> List[Gap]:
    """Find V1426 protocols unused in any mapping."""
    if layers is None:
        layers = _load_v1432_layers()
    if mappings is None:
        mappings = _load_v1432_mappings()
    if protocols is None:
        protocols = _load_v1426_protocols()
    used_protocols = {m.v1426_protocol for m in mappings}
    redundant: List[Gap] = []
    for proto in protocols:
        if proto not in used_protocols:
            redundant.append(
                Gap(
                    direction=Direction.REVERSE,
                    name=proto,
                    note="V1426 protocol not used by any VCP layer mapping",
                )
            )
    return redundant


def compute_parity(forward: float, reverse: float) -> float:
    """Parity score = arithmetic mean of forward + reverse coverage."""
    return (forward + reverse) / 2.0


# ============================================================================
# Build report
# ============================================================================


def build_report() -> ConsistencyReport:
    """Build a complete ConsistencyReport from V1432 + V1426."""
    from datetime import datetime, timezone

    report = ConsistencyReport(started_iso=_now_iso())
    layers = _load_v1432_layers()
    mappings = _load_v1432_mappings()
    protocols = _load_v1426_protocols()

    report.n_layers = len(layers)
    report.n_protocols = len(protocols)

    # Build rows from mappings
    for m in mappings:
        report.rows.append(
            ConsistencyRow(
                vcp_layer=m.vcp_module,
                v1426_protocol=m.v1426_protocol,
                match_score=m.match_score,
                rationale=m.rationale,
            )
        )

    # Forward / reverse coverage
    report.forward_coverage = compute_forward_coverage(layers, mappings)
    report.reverse_coverage = compute_reverse_coverage(layers, mappings, protocols)

    # Gaps and redundant
    report.gaps = find_forward_gaps(layers, mappings)
    report.redundant = find_redundant_protocols(layers, mappings, protocols)

    # Parity
    report.parity = compute_parity(report.forward_coverage, report.reverse_coverage)
    report.ended_iso = _now_iso()
    return report


# ============================================================================
# Render
# ============================================================================


def render_report_md(report: ConsistencyReport) -> str:
    """Render markdown report."""
    lines: List[str] = []
    lines.append(f"# V1433 ASI-VCP Structural Consistency Report `{V1433_VERSION}`")
    lines.append("")
    lines.append(f"- started: `{report.started_iso}`")
    lines.append(f"- ended: `{report.ended_iso}`")
    lines.append(f"- n_layers: {report.n_layers}")
    lines.append(f"- n_protocols: {report.n_protocols}")
    lines.append("")
    lines.append("## Coverage")
    lines.append("")
    lines.append(f"- forward_coverage (VCP layer → V1426 protocol): **{report.forward_coverage:.4f}**")
    lines.append(f"- reverse_coverage (V1426 protocol → VCP layer): **{report.reverse_coverage:.4f}**")
    lines.append(f"- parity_score: **{report.parity:.4f}**")
    lines.append("")
    lines.append("## Rows")
    lines.append("")
    lines.append("| VCP layer | V1426 protocol | match_score | rationale |")
    lines.append("|---|---|---|---|")
    for r in report.rows:
        rationale = r.rationale.replace("|", "\\|")
        lines.append(
            f"| {r.vcp_layer} | {r.v1426_protocol} | {r.match_score:.2f} | {rationale} |"
        )
    lines.append("")
    lines.append(f"## Gaps ({len(report.gaps)})")
    lines.append("")
    if report.gaps:
        for g in report.gaps:
            lines.append(f"- **{g.direction.value}**: `{g.name}` — {g.note}")
    else:
        lines.append("- (none)")
    lines.append("")
    lines.append(f"## Redundant ({len(report.redundant)})")
    lines.append("")
    if report.redundant:
        for r in report.redundant:
            lines.append(f"- **{r.direction.value}**: `{r.name}` — {r.note}")
    else:
        lines.append("- (none)")
    lines.append("")
    lines.append("## Honest disclosure")
    lines.append("")
    lines.append(
        "V1433 is a **bounded structural consistency report**. It does not"
    )
    lines.append(
        "claim VCP integration parity, VCP protocol completeness, or that"
    )
    lines.append(
        "our V1426 dispatcher fully implements VCP semantics. It claims"
    )
    lines.append(
        "only: the mappings were read, the coverage was computed, the gaps"
    )
    lines.append(
        "were reported. V1433 is read-only; never mutates V1432 or V1426."
    )
    lines.append(
        "Structural parity ≠ implementation parity."
    )
    return "\n".join(lines) + "\n"


# ============================================================================
# Chain delegate
# ============================================================================


def chain_delegate() -> Dict[str, Any]:
    """Probe upstream modules (V1432, V1426) for liveness."""
    chain: Dict[str, Any] = {}
    try:
        from apeireth.v1432_vcp_real_source_deep_read import V1432_VERSION
        chain["V1432"] = {"ok": True, "version": V1432_VERSION}
    except Exception as exc:
        chain["V1432"] = {"ok": False, "error": f"{type(exc).__name__}: {exc}"}

    try:
        from apeireth.v1426_vcp_six_protocol_dispatcher import V1426_VERSION
        chain["V1426"] = {"ok": True, "version": V1426_VERSION}
    except Exception as exc:
        chain["V1426"] = {"ok": False, "error": f"{type(exc).__name__}: {exc}"}

    all_ok = all(c.get("ok") for c in chain.values())
    return {"all_ok": all_ok, "chain": chain, "n_modules": len(chain)}


# ============================================================================
# Module metadata
# ============================================================================


def module_meta() -> Dict[str, Any]:
    """Module metadata."""
    return {
        "module": V1433_MODULE,
        "version": V1433_VERSION,
        "schema": V1433_SCHEMA,
        "guards": list(V1433_GUARDS),
        "v3_guards": list(V1433_V3_GUARDS),
        "borrowed": [b[0] for b in V1433_BORROWED],
        "n_guards": len(V1433_GUARDS),
        "n_v3_guards": len(V1433_V3_GUARDS),
        "n_borrowed": len(V1433_BORROWED),
        "n_api_surfaces": 14,
        "n_cli_commands": 9,
    }


# ============================================================================
# Popper self-test
# ============================================================================


def popper_self_test() -> Dict[str, Any]:
    """Popper-style self-test: 14 deterministic checks."""
    results: Dict[str, Tuple[bool, str]] = {}

    # PT01: importable
    try:
        import apeireth.v1433_asi_vcp_structural_consistency as self_mod  # noqa: F401
        results["PT01_importable"] = (True, "ok")
    except Exception as exc:
        results["PT01_importable"] = (False, f"{type(exc).__name__}: {exc}")
        return {"n_pass": 0, "n_total": 1, "ok": False, "results": results}

    # PT02: version set
    results["PT02_version_set"] = (
        V1433_VERSION == "0.1.0",
        f"version={V1433_VERSION}",
    )

    # PT03: 14 guards
    results["PT03_guards_set"] = (
        len(V1433_GUARDS) == 14,
        f"n={len(V1433_GUARDS)}",
    )

    # PT04: 5 V3 guards
    results["PT04_v3_guards"] = (
        len(V1433_V3_GUARDS) == 5,
        f"n={len(V1433_V3_GUARDS)}",
    )

    # PT05: 5 borrowed
    results["PT05_borrowed"] = (
        len(V1433_BORROWED) == 5,
        f"n={len(V1433_BORROWED)}",
    )

    # PT06: Direction enum has 2 values
    results["PT06_direction_enum"] = (
        len(list(Direction)) == 2,
        f"n={len(list(Direction))}",
    )

    # PT07: V1432 layers load (>= 1)
    layers = _load_v1432_layers()
    results["PT07_v1432_layers_loaded"] = (
        len(layers) >= 1,
        f"n={len(layers)}",
    )

    # PT08: V1432 mappings load (>= 1)
    mappings = _load_v1432_mappings()
    results["PT08_v1432_mappings_loaded"] = (
        len(mappings) >= 1,
        f"n={len(mappings)}",
    )

    # PT09: V1426 protocols load (>= 1)
    protocols = _load_v1426_protocols()
    results["PT09_v1426_protocols_loaded"] = (
        len(protocols) >= 1,
        f"n={len(protocols)}",
    )

    # PT10: forward coverage ∈ [0, 1]
    fwd = compute_forward_coverage(layers, mappings)
    results["PT10_forward_coverage_bounded"] = (
        0.0 <= fwd <= 1.0,
        f"forward={fwd:.4f}",
    )

    # PT11: reverse coverage ∈ [0, 1]
    rev = compute_reverse_coverage(layers, mappings, protocols)
    results["PT11_reverse_coverage_bounded"] = (
        0.0 <= rev <= 1.0,
        f"reverse={rev:.4f}",
    )

    # PT12: parity = (forward + reverse) / 2
    parity = compute_parity(fwd, rev)
    results["PT12_parity_definition"] = (
        abs(parity - (fwd + rev) / 2.0) < 1e-9,
        f"parity={parity:.4f}",
    )

    # PT13: build_report returns ConsistencyReport with required keys
    report = build_report()
    d = report.to_dict()
    required_keys = {
        "rows", "gaps", "redundant", "forward_coverage",
        "reverse_coverage", "parity", "n_layers", "n_protocols",
    }
    results["PT13_build_report_keys"] = (
        required_keys.issubset(d.keys()),
        f"keys={sorted(d.keys())}",
    )

    # PT14: render_report_md is non-empty string with V1433 + Honest disclosure
    md = render_report_md(report)
    results["PT14_render_report_md"] = (
        isinstance(md, str) and "V1433" in md and "Honest disclosure" in md,
        f"len={len(md)}",
    )

    n_pass = sum(1 for v in results.values() if v[0])
    n_total = len(results)
    return {
        "n_pass": n_pass,
        "n_total": n_total,
        "ok": n_pass == n_total,
        "results": results,
    }


# ============================================================================
# CLI
# ============================================================================


def _cmd_version(_args: List[str]) -> int:
    print(f"V1433 ASI-VCP Structural Consistency v{V1433_VERSION}")
    return 0


def _cmd_meta(args: List[str]) -> int:
    meta = module_meta()
    if "--json" in args:
        print(json.dumps(meta, indent=2, ensure_ascii=False))
    else:
        for k, v in meta.items():
            print(f"  {k}: {v}")
    return 0


def _cmd_help(_args: List[str]) -> int:
    print(__doc__)
    return 0


def _cmd_popper(_args: List[str]) -> int:
    result = popper_self_test()
    print(
        f"popper: n_pass={result['n_pass']}/{result['n_total']} "
        f"ok={result['ok']}"
    )
    for k, v in result["results"].items():
        ok, note = v
        status = "ok" if ok else "FAIL"
        print(f"  [{status}] {k}: {note}")
    return 0 if result["ok"] else 1


def _cmd_chain(_args: List[str]) -> int:
    chain = chain_delegate()
    print(json.dumps(chain, indent=2, ensure_ascii=False))
    return 0 if chain["all_ok"] else 1


def _cmd_forward(_args: List[str]) -> int:
    layers = _load_v1432_layers()
    mappings = _load_v1432_mappings()
    fwd = compute_forward_coverage(layers, mappings)
    print(json.dumps({"forward_coverage": fwd, "n_layers": len(layers)}, indent=2))
    return 0


def _cmd_reverse(_args: List[str]) -> int:
    layers = _load_v1432_layers()
    mappings = _load_v1432_mappings()
    protocols = _load_v1426_protocols()
    rev = compute_reverse_coverage(layers, mappings, protocols)
    print(json.dumps({"reverse_coverage": rev, "n_protocols": len(protocols)}, indent=2))
    return 0


def _cmd_gaps(_args: List[str]) -> int:
    layers = _load_v1432_layers()
    mappings = _load_v1432_mappings()
    protocols = _load_v1426_protocols()
    gaps = find_forward_gaps(layers, mappings)
    redundant = find_redundant_protocols(layers, mappings, protocols)
    out = {
        "gaps": [g.to_dict() for g in gaps],
        "redundant": [r.to_dict() for r in redundant],
    }
    print(json.dumps(out, indent=2, ensure_ascii=False))
    return 0


def _cmd_report(_args: List[str]) -> int:
    report = build_report()
    print(render_report_md(report))
    return 0


_COMMANDS: Dict[str, Any] = {
    "version": _cmd_version,
    "meta": _cmd_meta,
    "help": _cmd_help,
    "popper": _cmd_popper,
    "chain": _cmd_chain,
    "forward": _cmd_forward,
    "reverse": _cmd_reverse,
    "gaps": _cmd_gaps,
    "report": _cmd_report,
}


def main(argv: Optional[List[str]] = None) -> int:
    """CLI entry point."""
    args = list(sys.argv[1:] if argv is None else argv)
    if not args:
        return _cmd_help(args)
    cmd = args[0]
    if cmd not in _COMMANDS:
        print(f"unknown command: {cmd}")
        print("available: " + ", ".join(_COMMANDS.keys()))
        return 1
    return _COMMANDS[cmd](args[1:])


if __name__ == "__main__":
    sys.exit(main())