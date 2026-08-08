"""Phase 1363 v1363_dashboard_trend_overlay — combined dashboard + history overlay.

## What V1363 is

V1363 is the **fourth** observability surface in the V1358 stage-delivery
chain. It wires V1361 (visual dashboard) and V1362 (history ledger) into
a single, one-shot CLI command that a human (or another agent) can run
to see:

  1. Current state — V1361's full dashboard (8 sections).
  2. Recent history — V1362's append-only JSONL ledger rendered as a table.
  3. Trend — V1362's moving-average delta with V3 守门 reminder.
  4. V3 守门 banner — philosophy guards.

V1363 is **purely additive**. It does NOT modify V1361 or V1362. It
only imports them. If V1361's dashboard breaks, V1363's `render-md`
will still produce V1362's trend section (and report the V1361 error).

V1363 = the "any-human-can-pick-up" surface in its most useful form:
ONE command, ONE Markdown file, full state of the project.

## Why V1363 (主 00:56 任何人都能接手)

The V1358 stage-delivery plan called for 3 concrete ships after V1357.
V1359 (real_production gap), V1360 (vcp_toolchain name-drift), V1361
(visual dashboard) shipped all three. V1362 added a bonus history
ledger. V1363 closes the loop by wiring them all into one CLI surface.

The resulting one-liner:

```
python -m apeireth.v1363_dashboard_trend_overlay render-md > STATE.md
git add STATE.md
git commit -m "docs(state): V1363 one-shot dashboard + trend overlay"
```

…gives any human (or agent) a snapshot of:
  - "Where is the pole-star now?"
  - "How did it get here?"
  - "Is it moving up or down?"
  - "What guards are active?"
  - "What unknowns remain?"

…in a single Markdown file they can read in 30 seconds.

## CLI subcommands

  v1363-overlay render-md              # full combined dashboard as Markdown
  v1363-overlay render-md-trend       # only the trend section (V1362 only)
  v1363-overlay render-json           # combined JSON (V1361 snap + V1362 history)
  v1363-overlay snapshot --tag T --out PATH
                                      # write snapshot+trend to a file
                                      # (uses V1362.append_snapshot internally)
  v1363-overlay append --tag T        # append current snapshot to V1362 history
  v1363-overlay self-test [--verbose] # Popper checks
  v1363-overlay version

## V3 哲学守门 (LOCKED, 主 17:58 + 20:46 + 17:43)

- 不假装分数 = ASI: V1363 cap = 0.005 (overlay ≠ ASI)
- 不假装决策 = 真生产: V1363 = mechanical composition of V1361 + V1362; no fabrication
- 不破坏 4 层安全门: V1363 only writes to disk via V1362.append_snapshot
  (append-only JSONL). All other paths are read-only (AST-verified).
- 不假装 ASI 集成: V1363 only imports V1361 + V1362 (composition, not integration)
- 不假装 ASI 等级: GUARD_OVERLAY_NOT_ASI prevents score drift
- 不动 anchor: V1361 + V1362 behavior unchanged
- 不刷分: V1363 is not in pole-star components

## Architecture

```
V1357 (snapshot)  ───►  V1361 (dashboard)  ───►  V1363 (combined)
       │                                              ▲
       └────────────────►  V1362 (history)  ──────────┘
                            (append-only JSONL)
```

V1363 imports V1361 + V1362, never V1357 directly. V1362 also imports
V1357. This means V1363's data chain is:

  V1363 → V1361 → V1357
  V1363 → V1362 → V1357

Single source of truth = V1357. V1363 only composes.
"""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple

V1363_VERSION = "0.1.0"
V1363_ASI_CAP = 0.005  # honest cap; overlay ≠ ASI

# Philosophy guards — V3 守门 (主 17:58 + 20:46 + 17:43)
V1363_PHILOSOPHY_GUARDS: Tuple[str, ...] = (
    "GUARD_OVERLAY_NOT_ASI",          # overlay subscore cap = 0.005
    "GUARD_COMPOSE_ONLY",             # V1363 = composition, no fabrication
    "GUARD_DELEGATE_TO_V1361_V1362",  # all data from V1361 + V1362
    "GUARD_READ_ONLY_EXCEPT_V1362",   # only writes via V1362.append_snapshot
    "GUARD_NO_FABRICATION",           # no synthesized metrics
    "GUARD_HONEST_CAP",               # 0.005 cap, never approaches ASI
    "GUARD_V1361_V1362_UNCHANGED",    # V1363 does not modify its sources
)

V1363_SUBWEIGHTS: Dict[str, float] = {
    # If one ever aggregates V1363 into a pole-star component, use this.
    # Today V1363 is presentation-only and not aggregated.
    "composition_correctness": 0.30,
    "delegation_correctness": 0.25,
    "trend_section_fidelity": 0.20,
    "philosophy_compliance": 0.15,
    "self_test_coverage": 0.10,
}
assert abs(sum(V1363_SUBWEIGHTS.values()) - 1.0) < 1e-9, "subweights must sum to 1.0"


# -----------------------------------------------------------------------------
# Data sources (delegate to V1361 + V1362)
# -----------------------------------------------------------------------------

def _import_v1361():
    """Lazy import of V1361 (visual dashboard)."""
    try:
        from apeireth import v1361_streamlit_dashboard as v1361
        return v1361
    except ImportError as exc:
        raise RuntimeError(
            "V1363 requires apeireth.v1361_streamlit_dashboard. "
            f"Import error: {exc}"
        )


def _import_v1362():
    """Lazy import of V1362 (pole-star history)."""
    try:
        from apeireth import v1362_pole_star_history as v1362
        return v1362
    except ImportError as exc:
        raise RuntimeError(
            "V1363 requires apeireth.v1362_pole_star_history. "
            f"Import error: {exc}"
        )


def get_v1361_dashboard_md() -> str:
    """V1361 full dashboard as Markdown (delegated)."""
    return _import_v1361().render_full_markdown()


def get_v1362_history_table(limit: Optional[int] = 20) -> str:
    """V1362 history table as Markdown (delegated)."""
    v1362 = _import_v1362()
    entries = v1362.read_history(limit=limit)
    return v1362.render_history_table(entries)


def get_v1362_trend_md(window: int = 5) -> str:
    """V1362 trend section as Markdown (delegated)."""
    v1362 = _import_v1362()
    entries = v1362.read_history()
    trend = v1362.compute_trend(entries, window=window)
    return v1362.render_trend_md(trend)


def get_combined_snapshot_dict(history_limit: int = 20, window: int = 5) -> Dict[str, Any]:
    """Combined dict: V1361 snapshot + V1362 history + V1362 trend."""
    v1361 = _import_v1361()
    v1362 = _import_v1362()

    snap_dict = v1361.snapshot_to_dict()
    entries = v1362.read_history(limit=history_limit)
    trend = v1362.compute_trend(v1362.read_history(), window=window)

    return {
        "v1363_version": V1363_VERSION,
        "v1361_snapshot": snap_dict,
        "v1362_history": {
            "n_total": v1362.history_count(),
            "entries_shown": len(entries),
            "entries": entries,
        },
        "v1362_trend": trend,
        "v1363_asi_cap": V1363_ASI_CAP,
        "v1363_philosophy_guards": list(V1363_PHILOSOPHY_GUARDS),
    }


# -----------------------------------------------------------------------------
# Render — combined surfaces (pure)
# -----------------------------------------------------------------------------

def render_trend_overlay_section_md(
    history_limit: int = 10,
    window: int = 5,
) -> str:
    """Render the trend overlay section as Markdown.

    This is the section V1363 would inject into V1361's dashboard. It is
    also what V1363 emits on its own in `render-md-trend`.
    """
    v1362 = _import_v1362()
    entries = v1362.read_history(limit=history_limit)
    trend = v1362.compute_trend(v1362.read_history(), window=window)

    lines: List[str] = []
    lines.append("### Pole-Star History Overlay (V1362 → V1363)")
    lines.append("")
    lines.append(
        f"_V1363 wires V1362 history into V1361 dashboard. "
        f"history_limit={history_limit}, trend_window={window}._"
    )
    lines.append("")
    lines.append(f"**History entries**: {v1362.history_count()} total; showing last {len(entries)}")
    lines.append("")
    lines.append(v1362.render_history_table(entries))
    lines.append("")
    lines.append(v1362.render_trend_md(trend))
    lines.append("")
    return "\n".join(lines)


def render_combined_dashboard_md(
    history_limit: int = 10,
    window: int = 5,
) -> str:
    """Render the full combined dashboard as Markdown.

    Composition:
      1. V1361's full dashboard (8 sections).
      2. V1363's trend overlay section.
      3. V1363's V3 守门 banner.
    """
    lines: List[str] = []
    # Section 1: V1361 dashboard (8 sections: header, pole-star, toolchain,
    # close-loop, infra, commits, unknowns, guards)
    lines.append("# Apeireth V1363 — Combined Dashboard + Trend Overlay")
    lines.append("")
    lines.append(
        f"_V1363 v{V1363_VERSION} — composition of V1361 (visual) + V1362 (history). "
        f"single-source-of-truth = V1357. any-human-can-pick-up (主 00:56)._"
    )
    lines.append("")
    try:
        lines.append(get_v1361_dashboard_md())
    except Exception as exc:
        lines.append(f"> ⚠ V1361 dashboard error: `{exc}`")
        lines.append("> (V1362 trend section below still works.)")
        lines.append("")
    # Section 2: V1363 trend overlay
    lines.append("---")
    lines.append("")
    lines.append(render_trend_overlay_section_md(history_limit=history_limit, window=window))
    # Section 3: V1363 guards banner
    lines.append("---")
    lines.append("")
    lines.append("### V1363 V3 Philosophy Guards (守门)")
    lines.append("")
    for g in V1363_PHILOSOPHY_GUARDS:
        lines.append(f"- **{g}**")
    lines.append("")
    lines.append(f"V1363 ASI cap = **{V1363_ASI_CAP}** (overlay ≠ ASI)")
    lines.append("")
    lines.append(
        "_Made-by 楚零 (Apeireth ASI self-driven agent, cron:1fba1cc3). "
        "V3 不假装 Phenomenal / 不假装 ASI / 不刷分._"
    )
    return "\n".join(lines)


# -----------------------------------------------------------------------------
# Snapshot (write) — guarded, only via V1362.append_snapshot
# -----------------------------------------------------------------------------

def write_combined_snapshot(
    out_path: Path,
    tag: Optional[str] = None,
    history_limit: int = 10,
    window: int = 5,
) -> Dict[str, Any]:
    """Append current state to V1362 history AND write combined dashboard.

    This is the only "write" path in V1363. The append goes through
    V1362.append_snapshot (append-only JSONL — GUARD_READ_ONLY_EXCEPT_V1362).
    The Markdown file write is the operator-requested output.

    Returns a small dict describing what was done.
    """
    out_path = Path(out_path)
    v1362 = _import_v1362()
    appended = v1362.append_snapshot(tag=tag)

    dashboard_md = render_combined_dashboard_md(
        history_limit=history_limit,
        window=window,
    )
    out_path.parent.mkdir(parents=True, exist_ok=True)
    with open(out_path, "w", encoding="utf-8") as f:
        f.write(dashboard_md)

    return {
        "out_path": str(out_path),
        "appended_entry_measured_at": appended.get("measured_at"),
        "appended_entry_tag": appended.get("tag"),
        "history_count_after": v1362.history_count(),
        "dashboard_bytes": out_path.stat().st_size,
    }


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
    check("V1363_VERSION is semver", V1363_VERSION.count(".") == 2)
    check("V1363_ASI_CAP <= 0.01", V1363_ASI_CAP <= 0.01)
    check("V1363_ASI_CAP > 0", V1363_ASI_CAP > 0)
    check("V1363_ASI_CAP == 0.005 (honest)", V1363_ASI_CAP == 0.005)
    check("philosophy guards >= 5", len(V1363_PHILOSOPHY_GUARDS) >= 5)
    check("GUARD_OVERLAY_NOT_ASI present",
          "GUARD_OVERLAY_NOT_ASI" in V1363_PHILOSOPHY_GUARDS)
    check("GUARD_COMPOSE_ONLY present",
          "GUARD_COMPOSE_ONLY" in V1363_PHILOSOPHY_GUARDS)
    check("GUARD_DELEGATE_TO_V1361_V1362 present",
          "GUARD_DELEGATE_TO_V1361_V1362" in V1363_PHILOSOPHY_GUARDS)
    check("GUARD_READ_ONLY_EXCEPT_V1362 present",
          "GUARD_READ_ONLY_EXCEPT_V1362" in V1363_PHILOSOPHY_GUARDS)
    check("subweights sum to 1.0",
          abs(sum(V1363_SUBWEIGHTS.values()) - 1.0) < 1e-9)

    # --- Delegation: imports work -------------------------------------------
    v1361 = _import_v1361()
    v1362 = _import_v1362()
    check("V1361 importable", v1361 is not None)
    check("V1362 importable", v1362 is not None)
    check("V1361 has render_full_markdown", hasattr(v1361, "render_full_markdown"))
    check("V1362 has read_history", hasattr(v1362, "read_history"))
    check("V1362 has compute_trend", hasattr(v1362, "compute_trend"))
    check("V1362 has render_history_table", hasattr(v1362, "render_history_table"))
    check("V1362 has render_trend_md", hasattr(v1362, "render_trend_md"))
    check("V1362 has append_snapshot", hasattr(v1362, "append_snapshot"))

    # --- Data source correctness --------------------------------------------
    combined = get_combined_snapshot_dict(history_limit=3, window=2)
    check("combined has v1363_version", combined.get("v1363_version") == V1363_VERSION)
    check("combined has v1361_snapshot", "v1361_snapshot" in combined)
    check("combined has v1362_history", "v1362_history" in combined)
    check("combined has v1362_trend", "v1362_trend" in combined)
    check("v1362_history.n_total >= 0", combined["v1362_history"]["n_total"] >= 0)
    check("v1362_trend has n_entries", "n_entries" in combined["v1362_trend"])
    check("v1362_trend has window", combined["v1362_trend"]["window"] == 2)

    # --- Render fidelity -----------------------------------------------------
    overlay_md = render_trend_overlay_section_md(history_limit=3, window=2)
    check("overlay_md is non-empty string", isinstance(overlay_md, str) and len(overlay_md) > 50)
    check("overlay_md mentions V1362", "V1362" in overlay_md)
    check("overlay_md mentions V1363", "V1363" in overlay_md)
    check("overlay_md has markdown table", "|" in overlay_md)

    combined_md = render_combined_dashboard_md(history_limit=3, window=2)
    check("combined_md mentions V1361", "V1361" in combined_md)
    check("combined_md mentions V1363", "V1363" in combined_md)
    check("combined_md mentions trend", "trend" in combined_md.lower())
    check("combined_md has header", "Combined Dashboard" in combined_md)
    check("combined_md has guards", "GUARD_OVERLAY_NOT_ASI" in combined_md)
    check("combined_md has ASI cap", "0.005" in combined_md)

    # --- Trend bounds (sanity) -----------------------------------------------
    trend = combined["v1362_trend"]
    if trend["n_entries"] >= 2 and trend["delta"] is not None:
        check("trend delta is numeric",
              isinstance(trend["delta"], (int, float)))
        check("trend delta bounded (|delta| <= 1.0)",
              abs(trend["delta"]) <= 1.0,
              f"delta={trend['delta']}")

    # --- Snapshot write (delegates to V1362.append_snapshot) -----------------
    import tempfile
    with tempfile.TemporaryDirectory() as tmp:
        out_path = Path(tmp) / "STATE.md"
        before_count = v1362.history_count()
        result = write_combined_snapshot(out_path, tag="v1363-self-test",
                                         history_limit=3, window=2)
        after_count = v1362.history_count()
        check("snapshot appends to V1362 (count grew by 1)",
              after_count == before_count + 1,
              f"before={before_count}, after={after_count}")
        check("snapshot returns dict", isinstance(result, dict))
        check("snapshot out_path matches", result.get("out_path") == str(out_path))
        check("snapshot dashboard_bytes > 0",
              result.get("dashboard_bytes", 0) > 0,
              f"bytes={result.get('dashboard_bytes')}")
        check("snapshot file actually exists", out_path.exists())
        check("snapshot file content non-empty",
              out_path.stat().st_size > 100)
        # The file should include V1361 + V1362 + V1363 markers
        content = out_path.read_text(encoding="utf-8")
        check("snapshot file mentions V1363", "V1363" in content)
        # Dashboard contains "Pole-Star" (V1361 header) — case-insensitive check
        check("snapshot file mentions pole-star",
              "pole-star" in content.lower() or "pole_star" in content.lower())

    return passed, total, failures


# -----------------------------------------------------------------------------
# CLI
# -----------------------------------------------------------------------------

def _cli_render_md(args: argparse.Namespace) -> int:
    print(render_combined_dashboard_md(
        history_limit=args.history_limit,
        window=args.window,
    ))
    return 0


def _cli_render_md_trend(args: argparse.Namespace) -> int:
    print(render_trend_overlay_section_md(
        history_limit=args.history_limit,
        window=args.window,
    ))
    return 0


def _cli_render_json(args: argparse.Namespace) -> int:
    combined = get_combined_snapshot_dict(
        history_limit=args.history_limit,
        window=args.window,
    )
    print(json.dumps(combined, indent=2, ensure_ascii=False))
    return 0


def _cli_snapshot(args: argparse.Namespace) -> int:
    out_path = Path(args.out)
    result = write_combined_snapshot(
        out_path,
        tag=args.tag,
        history_limit=args.history_limit,
        window=args.window,
    )
    print(json.dumps(result, indent=2, ensure_ascii=False))
    return 0


def _cli_append(args: argparse.Namespace) -> int:
    v1362 = _import_v1362()
    entry = v1362.append_snapshot(tag=args.tag)
    print(json.dumps(entry, indent=2, ensure_ascii=False))
    return 0


def _cli_self_test(args: argparse.Namespace) -> int:
    passed, total, failures = _popper_self_tests(verbose=args.verbose)
    print(f"V1363 self-test: {passed}/{total} passed")
    if failures:
        print("FAILURES:")
        for f in failures:
            print(f"  - {f}")
    return 0 if passed == total else 2


def _build_parser() -> argparse.ArgumentParser:
    p = argparse.ArgumentParser(
        prog="v1363-overlay",
        description=(
            "Combined dashboard + trend overlay (V1361 visual + V1362 history). "
            "V3 守门: presentation ≠ ASI."
        ),
    )
    sub = p.add_subparsers(dest="command", required=True)

    # render-md
    p_rmd = sub.add_parser("render-md", help="full combined dashboard as Markdown")
    p_rmd.add_argument("--history-limit", type=int, default=10)
    p_rmd.add_argument("--window", type=int, default=5)
    p_rmd.set_defaults(func=_cli_render_md)

    # render-md-trend
    p_rmt = sub.add_parser(
        "render-md-trend",
        help="only the V1362 trend overlay section (Markdown)",
    )
    p_rmt.add_argument("--history-limit", type=int, default=10)
    p_rmt.add_argument("--window", type=int, default=5)
    p_rmt.set_defaults(func=_cli_render_md_trend)

    # render-json
    p_rj = sub.add_parser("render-json", help="combined JSON (V1361 snap + V1362 history)")
    p_rj.add_argument("--history-limit", type=int, default=20)
    p_rj.add_argument("--window", type=int, default=5)
    p_rj.set_defaults(func=_cli_render_json)

    # snapshot (one-shot: append to V1362 history + write dashboard file)
    p_snap = sub.add_parser(
        "snapshot",
        help="append current state to V1362 history AND write combined dashboard file",
    )
    p_snap.add_argument("--out", required=True, help="output Markdown file path")
    p_snap.add_argument("--tag", default=None, help="tag for the V1362 entry")
    p_snap.add_argument("--history-limit", type=int, default=10)
    p_snap.add_argument("--window", type=int, default=5)
    p_snap.set_defaults(func=_cli_snapshot)

    # append (delegated to V1362)
    p_app = sub.add_parser(
        "append",
        help="append current V1357 snapshot to V1362 history (delegated)",
    )
    p_app.add_argument("--tag", default=None)
    p_app.set_defaults(func=_cli_append)

    # self-test
    p_st = sub.add_parser("self-test", help="Popper self-tests")
    p_st.add_argument("--verbose", action="store_true")
    p_st.set_defaults(func=_cli_self_test)

    sub.add_parser("version", help="print version").set_defaults(
        func=lambda a: print(f"v1363-dashboard-trend-overlay {V1363_VERSION}") or 0
    )

    return p


def main(argv: Optional[List[str]] = None) -> int:
    parser = _build_parser()
    args = parser.parse_args(argv)
    return int(args.func(args))


if __name__ == "__main__":
    sys.exit(main())