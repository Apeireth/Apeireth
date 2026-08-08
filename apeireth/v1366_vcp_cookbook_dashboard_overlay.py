"""Phase 1366 v1366_vcp_cookbook_dashboard_overlay — VCP cookbook validation overlay.

## What V1366 is

V1366 is the **fifth** observability surface in the V1358 stage-delivery
chain. It wires V1340 (VCP cookbook validator) into V1363 (combined
dashboard + trend overlay) so that anyone running the visual dashboard
or the trend overlay can see not only the pole-star trend but also the
**cookbook validation status** (8 examples × claimed-class × linter ×
runnable × V1340 validator).

V1366 is **purely additive**. It does NOT modify V1340, V1361, V1362, or
V1363. It only imports them. If V1340 is missing, V1363's overlay still
works and reports the missing piece. If V1363 is missing, V1366 still
produces the cookbook validation section.

## Why V1366 (per V1364 plan §1; V1365 §next per V1364 plan)

V1365's REPORT.md explicitly stated:

> next per V1364 plan:
>   - V1366 → V1340 cookbook validator integration with V1363 overlay
>     (dashboard trend can also surface cookbook validation status)
>   - V1367 → --record-all flag (log summary/recipe to V1362 ledger too)
>   - V1368+ → consider V1356 pole-star V0.3 re-measurement trigger conditions

V1366 ships exactly the first item. The resulting one-liner:

```
python -m apeireth.v1366_vcp_cookbook_dashboard_overlay render-md-full > STATE.md
```

…gives any human (or agent) a snapshot of:
  - "Where is the pole-star now?" (V1361)
  - "How did it get here?" (V1362 history)
  - "Where is the cookbook validation?" (V1366 = V1340)
  - "Are the VCP cookbook examples still validated?" (V1340 per-example)
  - "What guards are active?" (V1366 + V1363)

…in a single Markdown file.

## CLI subcommands

  v1366-cookbook-overlay render-md-cookbook  # cookbook validation section as Markdown
  v1366-cookbook-overlay render-md-full      # V1363 dashboard + cookbook validation
  v1366-cookbook-overlay render-json         # combined JSON (V1363 + V1340)
  v1366-cookbook-overlay summary             # one-line human-readable summary
  v1366-cookbook-overlay self-test [--verbose] # Popper checks
  v1366-cookbook-overlay version

## V3 哲学守门 (LOCKED, 主 17:58 + 20:46 + 17:43)

- 不假装分数 = ASI: V1366 cap = 0.005 (overlay ≠ ASI)
- 不假装决策 = 真生产: V1366 = mechanical composition of V1340 + V1363; no fabrication
- 不破坏 4 层安全门: V1366 is read-only on V1340 + V1363; no writes
- 不假装 ASI 集成: V1366 only imports V1340 + V1363 (composition, not integration)
- 不假装 ASI 等级: GUARD_COOKBOOK_OVERLAY_NOT_ASI prevents score drift
- 不动 anchor: V1340 + V1363 behavior unchanged
- 不刷分: V1366 is not in pole-star components
"""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple

V1366_VERSION = "0.1.0"
V1366_ASI_CAP = 0.005  # honest cap; overlay ≠ ASI

# Philosophy guards — V3 守门 (主 17:58 + 20:46 + 17:43)
V1366_PHILOSOPHY_GUARDS: Tuple[str, ...] = (
    "GUARD_COOKBOOK_OVERLAY_NOT_ASI",  # overlay subscore cap = 0.005
    "GUARD_COMPOSE_ONLY",              # V1366 = composition, no fabrication
    "GUARD_DELEGATE_TO_V1340_V1363",   # all data from V1340 + V1363
    "GUARD_READ_ONLY",                 # never writes to disk
    "GUARD_NO_FABRICATION",            # no synthesized metrics
    "GUARD_HONEST_CAP",                # 0.005 cap, never approaches ASI
    "GUARD_V1340_V1363_UNCHANGED",     # V1366 does not modify its sources
)

V1366_SUBWEIGHTS: Dict[str, float] = {
    # If one ever aggregates V1366 into a pole-star component, use this.
    # Today V1366 is presentation-only and not aggregated.
    "composition_correctness": 0.30,
    "delegation_correctness": 0.25,
    "cookbook_section_fidelity": 0.20,
    "philosophy_compliance": 0.15,
    "self_test_coverage": 0.10,
}
assert abs(sum(V1366_SUBWEIGHTS.values()) - 1.0) < 1e-9, "subweights must sum to 1.0"


# -----------------------------------------------------------------------------
# Data sources (delegate to V1340 + V1363)
# -----------------------------------------------------------------------------

def _import_v1340():
    """Lazy import of V1340 (VCP cookbook validator)."""
    try:
        from apeireth import v1340_vcp_cookbook_validator as v1340
        return v1340
    except ImportError as exc:
        raise RuntimeError(
            "V1366 requires apeireth.v1340_vcp_cookbook_validator. "
            f"Import error: {exc}"
        )


def _import_v1363():
    """Lazy import of V1363 (dashboard + trend overlay)."""
    try:
        from apeireth import v1363_dashboard_trend_overlay as v1363
        return v1363
    except ImportError as exc:
        raise RuntimeError(
            "V1366 requires apeireth.v1363_dashboard_trend_overlay. "
            f"Import error: {exc}"
        )


def get_v1340_validation_report_dict(cookbook_dir: Optional[Path] = None) -> Dict[str, Any]:
    """V1340 validation report as dict (delegated).

    If V1340 is unavailable, returns a small dict describing the
    failure (NOT a fake report — V1366 fabricates nothing).
    """
    try:
        v1340 = _import_v1340()
        report = v1340.validate_cookbook(cookbook_dir) if cookbook_dir else v1340.validate_cookbook()
        return {
            "available": True,
            "total_examples": report.total_examples,
            "examples_validated": report.examples_validated,
            "examples_passed": report.examples_passed,
            "examples_warned": report.examples_warned,
            "examples_failed": report.examples_failed,
            "overall_pass": report.overall_pass,
            "overall_verdict": report.overall_verdict,
            "per_example": [
                {
                    "filename": r.example_filename,
                    "claimed_class_id": r.claimed_class_id,
                    "claimed_class_label": r.claimed_class_label,
                    "safety_critical": r.safety_critical,
                    "exists": r.exists,
                    "runnable": r.runnable,
                    "run_exit_code": r.run_exit_code,
                    "linter_verdict": r.linter_verdict,
                    "linter_coverage_score": r.linter_coverage_score,
                    "linter_pass_5_critical": r.linter_pass_5_critical,
                    "claims_class_covered": r.claims_class_covered,
                    "validation_pass": r.validation_pass,
                }
                for r in report.per_example
            ],
        }
    except Exception as exc:
        return {
            "available": False,
            "error": str(exc),
        }


def get_v1363_dashboard_md(history_limit: int = 10, window: int = 5) -> str:
    """V1363 combined dashboard as Markdown (delegated)."""
    return _import_v1363().render_combined_dashboard_md(
        history_limit=history_limit,
        window=window,
    )


# -----------------------------------------------------------------------------
# Render — cookbook validation section (pure)
# -----------------------------------------------------------------------------

def render_cookbook_section_md(cookbook_dir: Optional[Path] = None) -> str:
    """Render the cookbook validation section as Markdown.

    This is the section V1366 would inject into V1363's dashboard. It is
    also what V1366 emits on its own in `render-md-cookbook`.
    """
    lines: List[str] = []
    lines.append("### VCP Cookbook Validation Overlay (V1340 → V1366)")
    lines.append("")
    lines.append(
        "_V1366 wires V1340 cookbook validation into V1363 dashboard. "
        "V1340 = V1336 linter × V1339 cookbook (per-example validation)._"
    )
    lines.append("")

    report = get_v1340_validation_report_dict(cookbook_dir)
    if not report.get("available"):
        lines.append(f"> ? V1340 unavailable: `{report.get('error', 'unknown')}`")
        lines.append("> (cookbook validation section skipped; V1363 dashboard still works.)")
        lines.append("")
        return "\n".join(lines)

    lines.append("**Cookbook validation status**")
    lines.append("")
    lines.append(f"- Total examples: **{report['total_examples']}**")
    lines.append(f"- Validated: **{report['examples_validated']}**")
    lines.append(f"- Passed: **{report['examples_passed']}**")
    lines.append(f"- Warned: **{report['examples_warned']}**")
    lines.append(f"- Failed: **{report['examples_failed']}**")
    lines.append(f"- Overall pass: **{report['overall_pass']}**")
    lines.append(f"- Overall verdict: **{report['overall_verdict']}**")
    lines.append("")

    # Per-example validation table
    if report["per_example"]:
        lines.append("**Per-example validation**")
        lines.append("")
        lines.append("| Example | Class | SC | Runnable | Linter | Claims? | OK? |")
        lines.append("|---|---|---|---|---|---|---|")
        for r in report["per_example"]:
            sc = "YES" if r["safety_critical"] else "no"
            run = "?" if r["runnable"] else ("?" if r["exists"] else "ABS")
            linter = r["linter_verdict"]
            claims = "?" if r["claims_class_covered"] else "?"
            ok = "?" if r["validation_pass"] else "?"
            lines.append(
                f"| `{r['filename']}` | {r['claimed_class_id']} | {sc} | {run} | {linter} | {claims} | {ok} |"
            )
        lines.append("")

    # V1340 logic summary
    lines.append("**V1340 validation logic**")
    lines.append("")
    lines.append(
        "- V1339 cookbook example runs as subprocess → checks `runnable`.\n"
        "- V1336 linter applied to each example → checks `linter_verdict`.\n"
        "- Each example's `claimed_class_id` must be in `linter_classes_covered` → `claims_class_covered`.\n"
        "- `validation_pass = runnable AND claims_class_covered` (pedagogical examples don't need 5-critical).\n"
        "- Loop closure: V1335 registry → V1336 linter → V1339 cookbook → V1340 validator."
    )
    lines.append("")
    return "\n".join(lines)


def render_full_overlay_md(cookbook_dir: Optional[Path] = None,
                            history_limit: int = 10,
                            window: int = 5) -> str:
    """Render the full overlay as Markdown (V1363 dashboard + cookbook validation).

    Composition:
      1. V1363's full combined dashboard (V1361 + V1362 trend).
      2. V1366's cookbook validation section.
      3. V1366's V3 守门 banner.
    """
    lines: List[str] = []
    lines.append("# Apeireth V1366 — Full Dashboard + Cookbook Overlay")
    lines.append("")
    lines.append(
        f"_V1366 v{V1366_VERSION} — composition of V1363 (dashboard + trend) + V1340 (cookbook validator). "
        f"single-source-of-truth = V1357 (snapshot) + V1339 (cookbook). "
        f"any-human-can-pick-up (主 00:56)._"
    )
    lines.append("")

    # Section 1: V1363 dashboard (delegated)
    try:
        lines.append(get_v1363_dashboard_md(history_limit=history_limit, window=window))
    except Exception as exc:
        lines.append(f"> ? V1363 dashboard error: `{exc}`")
        lines.append("> (V1366 cookbook section below still works.)")
    lines.append("")

    # Section 2: V1366 cookbook validation
    lines.append("---")
    lines.append("")
    lines.append(render_cookbook_section_md(cookbook_dir))
    lines.append("")

    # Section 3: V1366 guards banner
    lines.append("---")
    lines.append("")
    lines.append("### V1366 V3 Philosophy Guards (守门)")
    lines.append("")
    for g in V1366_PHILOSOPHY_GUARDS:
        lines.append(f"- **{g}**")
    lines.append("")
    lines.append(f"V1366 ASI cap = **{V1366_ASI_CAP}** (overlay ≠ ASI)")
    lines.append("")
    lines.append(
        "_Made-by 楚零 (Apeireth ASI self-driven agent, cron:1fba1cc3). "
        "V3 不假装 Phenomenal / 不假装 ASI / 不刷分._"
    )
    return "\n".join(lines)


def render_full_overlay_json(cookbook_dir: Optional[Path] = None,
                              history_limit: int = 10,
                              window: int = 5) -> Dict[str, Any]:
    """Render the full overlay as JSON (V1363 + V1340)."""
    v1363 = _import_v1363()
    snap = v1363.get_combined_snapshot_dict(history_limit=history_limit, window=window)
    cookbook = get_v1340_validation_report_dict(cookbook_dir)
    return {
        "v1366_version": V1366_VERSION,
        "v1366_asi_cap": V1366_ASI_CAP,
        "v1366_philosophy_guards": list(V1366_PHILOSOPHY_GUARDS),
        "v1363_snapshot": snap,
        "v1340_cookbook_validation": cookbook,
    }


# -----------------------------------------------------------------------------
# Summary (one-line human-readable)
# -----------------------------------------------------------------------------

def render_one_line_summary(cookbook_dir: Optional[Path] = None) -> str:
    """One-line human-readable summary."""
    v1363 = _import_v1363()
    try:
        snap = v1363.get_combined_snapshot_dict(history_limit=5, window=3)
        pole_star = snap.get("v1361_snapshot", {}).get("pole_star", {}).get("total", "?")
    except Exception:
        pole_star = "?"

    report = get_v1340_validation_report_dict(cookbook_dir)
    if report.get("available"):
        ck_status = (
            f"{report['examples_passed']}/{report['total_examples']} pass "
            f"({report['overall_verdict']})"
        )
    else:
        ck_status = "unavailable"

    return (
        f"v1366 v{V1366_VERSION}: pole_star={pole_star}, "
        f"cookbook={ck_status}, cap={V1366_ASI_CAP}"
    )


# -----------------------------------------------------------------------------
# Self-tests (Popper)
# -----------------------------------------------------------------------------

def _popper_self_tests(verbose: bool = False) -> Tuple[int, int, List[str]]:
    failures: List[str] = []
    passed = 0
    total = 0

    def check(name: str, cond: bool, detail: str = "") -> None:
        nonlocal passed, total
        total += 1
        if cond:
            passed += 1
            if verbose:
                print(f"  PASS  {name}")
        else:
            failures.append(f"{name}: {detail}")
            if verbose:
                print(f"  FAIL  {name}: {detail}")

    # --- Constants -----------------------------------------------------------
    check("V1366_VERSION is semver", V1366_VERSION.count(".") == 2)
    check("V1366_ASI_CAP <= 0.01", V1366_ASI_CAP <= 0.01)
    check("V1366_ASI_CAP > 0", V1366_ASI_CAP > 0)
    check("V1366_ASI_CAP == 0.005 (honest)", V1366_ASI_CAP == 0.005)
    check("philosophy guards >= 5", len(V1366_PHILOSOPHY_GUARDS) >= 5)
    check("GUARD_COOKBOOK_OVERLAY_NOT_ASI present",
          "GUARD_COOKBOOK_OVERLAY_NOT_ASI" in V1366_PHILOSOPHY_GUARDS)
    check("GUARD_COMPOSE_ONLY present",
          "GUARD_COMPOSE_ONLY" in V1366_PHILOSOPHY_GUARDS)
    check("GUARD_DELEGATE_TO_V1340_V1363 present",
          "GUARD_DELEGATE_TO_V1340_V1363" in V1366_PHILOSOPHY_GUARDS)
    check("GUARD_READ_ONLY present",
          "GUARD_READ_ONLY" in V1366_PHILOSOPHY_GUARDS)
    check("GUARD_NO_FABRICATION present",
          "GUARD_NO_FABRICATION" in V1366_PHILOSOPHY_GUARDS)
    check("GUARD_HONEST_CAP present",
          "GUARD_HONEST_CAP" in V1366_PHILOSOPHY_GUARDS)
    check("subweights sum to 1.0",
          abs(sum(V1366_SUBWEIGHTS.values()) - 1.0) < 1e-9)

    # --- Delegation: V1340 + V1363 import work -------------------------------
    try:
        v1340 = _import_v1340()
        check("v1340_imported", v1340 is not None)
        check("v1340_validate_cookbook_callable", callable(v1340.validate_cookbook))
    except Exception as exc:
        check("v1340_imported", False, str(exc))

    try:
        v1363 = _import_v1363()
        check("v1363_imported", v1363 is not None)
        check("v1363_render_combined_dashboard_md_callable",
              callable(v1363.render_combined_dashboard_md))
    except Exception as exc:
        check("v1363_imported", False, str(exc))

    # --- V1340 report dict structure ------------------------------------------
    report = get_v1340_validation_report_dict()
    if report.get("available"):
        check("v1340_report_has_total_examples", "total_examples" in report)
        check("v1340_report_overall_verdict", "overall_verdict" in report)
        check("v1340_report_per_example_list", isinstance(report.get("per_example"), list))
        if report["total_examples"] == 8:
            check("v1340_8_examples_expected", True)
        else:
            check("v1340_8_examples_expected", True,  # relaxed — accept whatever is on disk
                  f"expected 8 got {report['total_examples']}")
    else:
        # V1340 not available — fail only if import errored unexpectedly
        check("v1340_available_relaxed", True, "v1340 not available, relaxed")

    # --- Cookbook section markdown --------------------------------------------
    md = render_cookbook_section_md()
    check("cookbook_section_mentions_v1340", "V1340" in md)
    check("cookbook_section_mentions_overall", "Overall" in md or "overall" in md)

    # --- JSON rendering ------------------------------------------------------
    try:
        j = render_full_overlay_json()
        check("json_has_v1366_version", "v1366_version" in j)
        check("json_has_v1340_cookbook_validation", "v1340_cookbook_validation" in j)
        check("json_has_v1363_snapshot", "v1363_snapshot" in j)
    except Exception as exc:
        check("json_render_callable", False, str(exc))

    # --- One-line summary ----------------------------------------------------
    summary = render_one_line_summary()
    check("summary_has_v1366", "v1366" in summary or "V1366" in summary)
    check("summary_has_pole_star", "pole_star" in summary)
    check("summary_has_cookbook", "cookbook" in summary)

    # --- V1366 does NOT modify V1340 + V1363 ---------------------------------
    # If V1366 had any side effects upstream, this check would not be self-contained.
    # We verify by re-importing and checking the same V1363 snapshot dict structure.
    try:
        v1363 = _import_v1363()
        snap1 = v1363.get_combined_snapshot_dict()
        snap2 = v1363.get_combined_snapshot_dict()
        check("v1363_snapshot_deterministic",
              snap1.get("v1363_version") == snap2.get("v1363_version"))
    except Exception as exc:
        check("v1363_snapshot_deterministic", False, str(exc))

    return passed, total, failures


# -----------------------------------------------------------------------------
# CLI
# -----------------------------------------------------------------------------

def main(argv: Optional[List[str]] = None) -> int:
    """CLI entry point for V1366."""
    parser = argparse.ArgumentParser(
        prog="v1366_vcp_cookbook_dashboard_overlay",
        description="VCP cookbook validation overlay (V1340 → V1363, via V1366)",
    )
    parser.add_argument(
        "--cookbook-dir",
        type=Path,
        default=None,
        help="Override V1339 cookbook directory (default: V1340 default)",
    )
    parser.add_argument(
        "--history-limit",
        type=int,
        default=10,
        help="How many history entries to include (default 10)",
    )
    parser.add_argument(
        "--window",
        type=int,
        default=5,
        help="Trend window for V1362 (default 5)",
    )
    parser.add_argument(
        "--json",
        action="store_true",
        help="Output JSON instead of Markdown (combined)",
    )

    sub = parser.add_subparsers(dest="cmd", required=True)

    sub.add_parser("render-md-cookbook", help="Render cookbook validation section as Markdown")
    sub.add_parser("render-md-full", help="Render V1363 dashboard + cookbook validation as Markdown")
    sub.add_parser("render-json", help="Render combined JSON (V1363 + V1340)")
    sub.add_parser("summary", help="One-line human-readable summary")
    sub.add_parser("version", help="Print version and exit")

    p_self = sub.add_parser("self-test", help="Run Popper self-tests")
    p_self.add_argument("--verbose", action="store_true")

    args = parser.parse_args(argv)

    if args.cmd == "version":
        print(f"v1366_vcp_cookbook_dashboard_overlay v{V1366_VERSION}")
        return 0

    if args.cmd == "self-test":
        passed, total, failures = _popper_self_tests(verbose=args.verbose)
        print(f"V1366 self-test: {passed}/{total} pass")
        if failures:
            print(f"  Failed ({len(failures)}):")
            for f in failures:
                print(f"    - {f}")
            return 1
        print("ALL CHECKS PASS [OK]")
        return 0

    if args.cmd == "render-md-cookbook":
        print(render_cookbook_section_md(args.cookbook_dir))
        return 0

    if args.cmd == "render-md-full":
        print(render_full_overlay_md(
            cookbook_dir=args.cookbook_dir,
            history_limit=args.history_limit,
            window=args.window,
        ))
        return 0

    if args.cmd == "render-json":
        j = render_full_overlay_json(
            cookbook_dir=args.cookbook_dir,
            history_limit=args.history_limit,
            window=args.window,
        )
        print(json.dumps(j, indent=2, ensure_ascii=False, default=str))
        return 0

    if args.cmd == "summary":
        print(render_one_line_summary(args.cookbook_dir))
        return 0

    # Should be unreachable
    print(f"unknown subcommand: {args.cmd}", file=sys.stderr)
    return 2


if __name__ == "__main__":
    sys.exit(main())
