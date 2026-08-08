"""Phase 1361 v1361_streamlit_dashboard — visual observability dashboard.

## What V1361 is

V1357 = the **command-line** observability aggregator (any human can run
`snapshot` / `summary` / `recipe` to know project state in 5 seconds).
V1361 = the **visual** observability dashboard — a Streamlit app that
renders the V1357 snapshot as:

  1. Big metrics row — pole-star total, toolchain 11/11, close-loop 7/7,
     module count, test count.
  2. Pole-star breakdown — 8 components as a horizontal bar chart + table.
  3. Toolchain table — 11 VCP modules with presence status.
  4. Close-loop table — 7 scenarios with pass/fail, duration, apply counts.
  5. Infra file badges — ledger, migration_audit, remediation_history.
  6. Recent commits — last 10 with hash, date, subject.
  7. Known unknowns (honest disclosure).
  8. Philosophy guards banner — V3 守门.

## Why V1361 (主 00:56 any-human-can-pick-up)

The "any human tomorrow picks up" requirement (主 00:56) is satisfied by
two surfaces:

  - V1357 = command-line (headless, terminal-friendly, JSON-pipeable)
  - V1361 = visual dashboard (browser-friendly, screenshot-friendly, no
           install beyond `pip install streamlit` which is already present)

Both surfaces share the SAME data source (`v1357.build_snapshot()`), so
they cannot disagree. V1361 ≠ ASI, V1361 ≠ phenomenology; it is a thin
presentation layer over V1357.

## CLI subcommands

  v1361-dashboard serve                       # launch Streamlit app
  v1361-dashboard serve --port 8765           # custom port
  v1361-dashboard render-json                 # JSON the dashboard would consume
  v1361-dashboard render-md                   # Markdown dashboard (no streamlit)
  v1361-dashboard self-test [--verbose]       # Popper checks
  v1361-dashboard version

## Exit codes

  0  OK
  2  fatal: missing dependency (e.g., streamlit not installed) on `serve`
  3  invalid usage

## V3 哲学守门 (LOCKED, 主 17:58 + 20:46 + 17:43)

- 不假装 Phenomenal: V1361 = visual presentation layer; no phenomenology
- 不假装 ASI 智慧: dashboard = mechanical rendering of V1357 data
- 不假装 ASI 集成: V1361 only imports V1357 (the single source of truth)
- 不假装 ASI 等级: dashboard subscore cap = 0.005 (presentation ≠ ASI)
- 不动 anchor: V1361 is read-only; never writes to disk
- 不刷分: dashboard = render-only; never modifies V1357 score
- GUARD_DASHBOARD_NOT_ASI: explicit cap to prevent ASI drift
- GUARD_DELEGATE_TO_V1357: all data flows from V1357
- GUARD_READ_ONLY: no filesystem writes
"""
from __future__ import annotations

import argparse
import json
import sys
from typing import Any, Dict, List, Optional, Tuple

V1361_VERSION = "0.1.0"
V1361_ASI_CAP = 0.005  # honest cap; dashboard ≠ ASI

# Philosophy guards — V3 守门 (主 17:58 + 20:46 + 17:43)
V1361_PHILOSOPHY_GUARDS: Tuple[str, ...] = (
    "GUARD_DASHBOARD_NOT_ASI",
    "GUARD_DELEGATE_TO_V1357",
    "GUARD_READ_ONLY",
    "GUARD_NO_WRITES",
    "GUARD_NO_FAKE_METRICS",
    "GUARD_HONEST_CAP",
)

V1361_SUBWEIGHTS: Dict[str, float] = {
    # If one ever aggregates V1361 into a pole-star component, use this.
    # Today V1361 is presentation-only and not aggregated.
    "render_correctness": 0.40,
    "data_fidelity": 0.30,
    "philosophy_compliance": 0.20,
    "self_test_coverage": 0.10,
}
assert abs(sum(V1361_SUBWEIGHTS.values()) - 1.0) < 1e-9, "subweights must sum to 1.0"


# -----------------------------------------------------------------------------
# Data source (delegate to V1357)
# -----------------------------------------------------------------------------

def _import_v1357():
    """Import V1357 lazily so pytest doesn't need streamlit at module-load."""
    try:
        from apeireth import v1357_vcp_observability_snapshot as v1357
        return v1357
    except ImportError as exc:
        raise RuntimeError(
            "V1361 requires apeireth.v1357_vcp_observability_snapshot (the single source of truth). "
            f"Import error: {exc}"
        )


def get_snapshot() -> Any:
    """Return the V1357 ProjectSnapshot object (single source of truth)."""
    v1357 = _import_v1357()
    return v1357.build_snapshot()


def snapshot_to_dict() -> Dict[str, Any]:
    """Return the snapshot as a plain dict."""
    return get_snapshot().to_dict()


# -----------------------------------------------------------------------------
# Render functions (pure — no streamlit dependency)
# -----------------------------------------------------------------------------

def render_header_md(snap_dict: Dict[str, Any]) -> str:
    """Big metrics row as Markdown table."""
    pole = snap_dict.get("pole_star", {})
    tool = snap_dict.get("toolchain_health", {})
    close = snap_dict.get("close_loop_state", {})
    counts = snap_dict.get("module_counts", {})

    total = pole.get("total")
    cap = pole.get("honest_cap")
    delta = pole.get("delta_vs_v01")

    lines: List[str] = []
    lines.append("## Apeireth V1361 Dashboard (visual observability)")
    lines.append("")
    lines.append("| Metric | Value | Note |")
    lines.append("|--------|-------|------|")
    if total is not None:
        lines.append(f"| ASI pole-star V0.2 | **{total:.4f}** | cap {cap} |")
    else:
        lines.append(f"| ASI pole-star V0.2 | unknown | cap {cap} |")
    if delta is not None:
        lines.append(f"| Δ vs V0.1 baseline | {delta:+.4f} | baseline 0.7905 |")
    lines.append(f"| VCP toolchain | {tool.get('n_modules_present')}/{tool.get('n_modules_total')} | presence ratio {tool.get('presence_ratio')} |")
    lines.append(f"| Close-loop (V1355) | {close.get('n_pass')}/{close.get('n_scenarios')} pass | exit={close.get('exit_code')} |")
    lines.append(f"| apeireth v-modules | {counts.get('apeireth_v_modules')} | source count |")
    lines.append(f"| pytest files | {counts.get('test_files')} | source count |")
    lines.append("")
    return "\n".join(lines)


def render_pole_star_md(snap_dict: Dict[str, Any]) -> str:
    """8 components breakdown as Markdown table."""
    pole = snap_dict.get("pole_star", {})
    components = pole.get("components", [])
    lines: List[str] = []
    lines.append("### ASI Pole-Star V0.2 — Component Breakdown")
    lines.append("")
    lines.append("| Component | Weight | Raw | Weighted | Bar |")
    lines.append("|-----------|-------:|----:|---------:|-----|")
    lines.append("")
    for c in components:
        name = c.get("name", "?")
        w = c.get("weight", 0.0)
        raw = c.get("raw_value", 0.0)
        weighted = c.get("weighted_value", 0.0)
        # ASCII bar (10 chars)
        bar_len = int(round(raw * 10))
        bar = "█" * bar_len + "░" * (10 - bar_len)
        lines.append(f"| {name} | {w:.2f} | {raw:.4f} | {weighted:.4f} | `{bar}` |")
    lines.append("")
    return "\n".join(lines)


def render_toolchain_md(snap_dict: Dict[str, Any]) -> str:
    """11 VCP modules presence table."""
    tool = snap_dict.get("toolchain_health", {})
    present = set(tool.get("modules_present", []))
    absent = tool.get("modules_absent", [])
    # Full canonical list is in V1357; we read from V1357 directly via import
    v1357 = _import_v1357()
    all_mods = list(v1357.VCP_TOOLCHAIN_MODULES)

    lines: List[str] = []
    lines.append("### VCP Toolchain Health")
    lines.append("")
    lines.append(f"Presence: **{len(present)}/{len(all_mods)}** (ratio {tool.get('presence_ratio')})")
    lines.append("")
    lines.append("| # | Module | Status |")
    lines.append("|---|--------|--------|")
    for i, mod in enumerate(all_mods, 1):
        status = "✅ present" if mod in present else "❌ absent"
        lines.append(f"| {i} | `{mod}` | {status} |")
    if absent:
        lines.append("")
        lines.append(f"Missing: {', '.join(absent)}")
    lines.append("")
    return "\n".join(lines)


def render_close_loop_md(snap_dict: Dict[str, Any]) -> str:
    """V1355 wet-run scenario table."""
    close = snap_dict.get("close_loop_state", {})
    scenarios = close.get("scenarios", [])
    lines: List[str] = []
    lines.append("### V1355 Close-Loop (Wet-Run)")
    lines.append("")
    lines.append(f"Scenarios: **{close.get('n_pass')}/{close.get('n_scenarios')} pass**, exit={close.get('exit_code')}")
    lines.append("")
    lines.append("| Scenario | Pass | Apply OK | Duration | Reason |")
    lines.append("|----------|------|----------|---------:|--------|")
    for s in scenarios:
        flag = "✅" if s.get("pass") else "❌"
        reason = s.get("failure_reason") or "—"
        lines.append(
            f"| {s.get('name','?')} | {flag} | {s.get('apply_ok')}/{s.get('apply_attempted')} | "
            f"{s.get('duration_ms', 0):.1f} ms | {reason[:40]} |"
        )
    lines.append("")
    return "\n".join(lines)


def render_infra_md(snap_dict: Dict[str, Any]) -> str:
    """VCP infra files badges."""
    infra = snap_dict.get("infra_state", {})
    lines: List[str] = []
    lines.append("### VCP Infra Files")
    lines.append("")
    for k, v in infra.items():
        flag = "✅" if v else "❌"
        lines.append(f"- {flag} **{k}**")
    lines.append("")
    return "\n".join(lines)


def render_commits_md(snap_dict: Dict[str, Any], limit: int = 10) -> str:
    """Recent commits list."""
    commits = snap_dict.get("recent_commits", [])[:limit]
    lines: List[str] = []
    lines.append(f"### Recent Commits (latest {len(commits)})")
    lines.append("")
    lines.append("| Hash | Date | Subject |")
    lines.append("|------|------|---------|")
    for c in commits:
        h = c.get("hash", "?")
        d = c.get("date", "?")[:10]
        s = c.get("subject", "?")[:80]
        lines.append(f"| `{h}` | {d} | `{s}` |")
    lines.append("")
    return "\n".join(lines)


def render_unknowns_md(snap_dict: Dict[str, Any]) -> str:
    """Honest unknowns disclosure."""
    unknowns = snap_dict.get("known_unknowns", [])
    lines: List[str] = []
    lines.append("### Known Unknowns (Honest Disclosure)")
    lines.append("")
    if not unknowns:
        lines.append("_None — all sections determined._")
    else:
        for u in unknowns:
            lines.append(f"- {u}")
    lines.append("")
    return "\n".join(lines)


def render_guards_md() -> str:
    """Philosophy guards banner."""
    lines: List[str] = []
    lines.append("### V3 Philosophy Guards (守门)")
    lines.append("")
    for g in V1361_PHILOSOPHY_GUARDS:
        lines.append(f"- **{g}**")
    lines.append("")
    lines.append(f"V1361 ASI cap = **{V1361_ASI_CAP}** (presentation ≠ ASI)")
    lines.append("")
    return "\n".join(lines)


def render_full_markdown(snap_dict: Optional[Dict[str, Any]] = None) -> str:
    """Render the complete dashboard as Markdown."""
    if snap_dict is None:
        snap_dict = snapshot_to_dict()
    parts: List[str] = []
    parts.append(render_header_md(snap_dict))
    parts.append(render_pole_star_md(snap_dict))
    parts.append(render_toolchain_md(snap_dict))
    parts.append(render_close_loop_md(snap_dict))
    parts.append(render_infra_md(snap_dict))
    parts.append(render_commits_md(snap_dict))
    parts.append(render_unknowns_md(snap_dict))
    parts.append(render_guards_md())
    return "\n".join(parts)


# -----------------------------------------------------------------------------
# Streamlit renderer (lazy import — only when serve() is called)
# -----------------------------------------------------------------------------

def serve(port: int = 8765) -> int:
    """Launch the Streamlit app. Requires streamlit installed."""
    try:
        import streamlit as st  # type: ignore
    except ImportError:
        print("ERROR: streamlit not installed. pip install streamlit", file=sys.stderr)
        return 2

    # Set page config (must be first st call)
    st.set_page_config(
        page_title="Apeireth V1361 Dashboard",
        page_icon="📊",
        layout="wide",
    )

    # Cache snapshot for the duration of this session
    @st.cache_data(ttl=60)
    def _cached_snapshot_dict() -> Dict[str, Any]:
        return get_snapshot().to_dict()

    snap_dict = _cached_snapshot_dict()

    # Header
    pole = snap_dict["pole_star"]
    close = snap_dict["close_loop_state"]
    tool = snap_dict["toolchain_health"]
    counts = snap_dict["module_counts"]

    st.title("📊 Apeireth V1361 — Visual Observability Dashboard")
    st.caption(
        f"V1361 v{V1361_VERSION} · single-source-of-truth = V1357 · "
        f"any-human-can-pick-up (主 00:56)"
    )

    # Big metrics row
    col1, col2, col3, col4, col5 = st.columns(5)
    total = pole.get("total")
    with col1:
        st.metric(
            label="ASI pole-star V0.2",
            value=f"{total:.4f}" if total is not None else "?",
            delta=f"{pole.get('delta_vs_v01', 0):+.4f} vs V0.1",
            delta_color="normal",
        )
    with col2:
        st.metric(
            label="VCP toolchain",
            value=f"{tool['n_modules_present']}/{tool['n_modules_total']}",
        )
    with col3:
        st.metric(
            label="Close-loop (V1355)",
            value=f"{close['n_pass']}/{close['n_scenarios']} pass",
        )
    with col4:
        st.metric(
            label="apeireth v-modules",
            value=f"{counts.get('apeireth_v_modules', 0)}",
        )
    with col5:
        st.metric(
            label="pytest files",
            value=f"{counts.get('test_files', 0)}",
        )

    st.divider()

    # Pole-star breakdown
    st.subheader("ASI Pole-Star V0.2 — Component Breakdown")
    components = pole.get("components", [])
    if components:
        # Table
        st.table([
            {
                "Component": c["name"],
                "Weight": c["weight"],
                "Raw": c["raw_value"],
                "Weighted": c["weighted_value"],
            }
            for c in components
        ])
        # Bar chart using DataFrame (streamlit 1.x requirement)
        try:
            import pandas as pd  # type: ignore
            df = pd.DataFrame([
                {
                    "Component": c["name"],
                    "Raw": c["raw_value"],
                    "Weighted": c["weighted_value"],
                }
                for c in components
            ]).set_index("Component")
            st.bar_chart(df)
        except ImportError:
            # Fallback: plain text bars (no pandas)
            for c in components:
                bar_len = int(round(c["raw_value"] * 10))
                bar = "█" * bar_len + "░" * (10 - bar_len)
                st.markdown(f"`{c['name']:<28}` `{bar}` raw={c['raw_value']:.4f}")
    st.divider()

    # Toolchain
    st.subheader("VCP Toolchain Health")
    v1357 = _import_v1357()
    all_mods = list(v1357.VCP_TOOLCHAIN_MODULES)
    present_set = set(tool.get("modules_present", []))
    rows = []
    for i, mod in enumerate(all_mods, 1):
        rows.append({
            "#": i,
            "Module": mod,
            "Status": "✅ present" if mod in present_set else "❌ absent",
        })
    st.table(rows)
    st.divider()

    # Close-loop
    st.subheader("V1355 Close-Loop (Wet-Run)")
    scenarios = close.get("scenarios", [])
    if scenarios:
        st.table([
            {
                "Scenario": s["name"],
                "Pass": "✅" if s["pass"] else "❌",
                "Apply OK": f"{s['apply_ok']}/{s['apply_attempted']}",
                "Duration (ms)": s["duration_ms"],
            }
            for s in scenarios
        ])
    st.divider()

    # Infra
    st.subheader("VCP Infra Files")
    infra_cols = st.columns(len(snap_dict["infra_state"]))
    for col, (k, v) in zip(infra_cols, snap_dict["infra_state"].items()):
        with col:
            st.markdown(f"{'✅' if v else '❌'} **{k}**")
    st.divider()

    # Commits
    st.subheader("Recent Commits")
    commits = snap_dict.get("recent_commits", [])
    if commits:
        st.table([
            {
                "Hash": c["hash"],
                "Date": c["date"][:10],
                "Subject": c["subject"][:100],
            }
            for c in commits[:10]
        ])
    st.divider()

    # Unknowns
    st.subheader("Known Unknowns (Honest Disclosure)")
    unknowns = snap_dict.get("known_unknowns", [])
    if unknowns:
        for u in unknowns:
            st.warning(u)
    else:
        st.success("None — all sections determined.")
    st.divider()

    # Guards
    st.subheader("V3 Philosophy Guards (守门)")
    st.caption(f"V1361 ASI cap = {V1361_ASI_CAP} (presentation ≠ ASI)")
    for g in V1361_PHILOSOPHY_GUARDS:
        st.markdown(f"- **{g}**")

    st.caption(
        "Made-by 楚零 (Apeireth ASI self-driven agent, cron:1fba1cc3). "
        "V3 不假装 Phenomenal / 不假装 ASI / 不刷分."
    )
    return 0


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

    # Constants
    check("V1361_VERSION is semver", V1361_VERSION.count(".") == 2)
    check("V1361_ASI_CAP <= 0.01", V1361_ASI_CAP <= 0.01)
    check("V1361_ASI_CAP > 0", V1361_ASI_CAP > 0)
    check("philosophy guards >= 4", len(V1361_PHILOSOPHY_GUARDS) >= 4)
    check("GUARD_DASHBOARD_NOT_ASI present",
          "GUARD_DASHBOARD_NOT_ASI" in V1361_PHILOSOPHY_GUARDS)
    check("GUARD_DELEGATE_TO_V1357 present",
          "GUARD_DELEGATE_TO_V1357" in V1361_PHILOSOPHY_GUARDS)
    check("subweights sum to 1.0",
          abs(sum(V1361_SUBWEIGHTS.values()) - 1.0) < 1e-9)

    # Data source — delegates to V1357
    snap = get_snapshot()
    check("snapshot not None", snap is not None)
    snap_dict = snap.to_dict()
    check("snap has pole_star", "pole_star" in snap_dict)
    check("snap has toolchain_health", "toolchain_health" in snap_dict)
    check("snap has close_loop_state", "close_loop_state" in snap_dict)

    # Render functions (pure, no streamlit)
    header = render_header_md(snap_dict)
    check("header non-empty", len(header) > 100)
    check("header mentions pole_star", "pole-star" in header.lower() or "Pole-Star" in header)

    pole_md = render_pole_star_md(snap_dict)
    check("pole_star_md non-empty", len(pole_md) > 100)
    n_components = len(snap_dict["pole_star"]["components"])
    check("pole_star_md has all 8 components",
          all(c["name"] in pole_md for c in snap_dict["pole_star"]["components"]),
          f"expected {n_components} components")

    tool_md = render_toolchain_md(snap_dict)
    check("tool_md non-empty", len(tool_md) > 100)
    check("tool_md mentions VCP_TOOLCHAIN_MODULES count (11)",
          "11" in tool_md)

    close_md = render_close_loop_md(snap_dict)
    check("close_md non-empty", len(close_md) > 50)
    n_scenarios = len(snap_dict["close_loop_state"].get("scenarios", []))
    check("close_md has all scenarios",
          all(s["name"] in close_md for s in snap_dict["close_loop_state"].get("scenarios", [])),
          f"expected {n_scenarios} scenarios")

    infra_md = render_infra_md(snap_dict)
    check("infra_md non-empty", len(infra_md) > 30)
    check("infra_md mentions all 3 infra keys",
          all(k in infra_md for k in snap_dict["infra_state"].keys()))

    commits_md = render_commits_md(snap_dict)
    check("commits_md non-empty", len(commits_md) > 30)

    unknowns_md = render_unknowns_md(snap_dict)
    check("unknowns_md non-empty", len(unknowns_md) > 30)

    guards_md = render_guards_md()
    check("guards_md non-empty", len(guards_md) > 30)
    check("guards_md mentions cap", "0.005" in guards_md or str(V1361_ASI_CAP) in guards_md)

    full_md = render_full_markdown(snap_dict)
    check("full_md non-empty", len(full_md) > 500)
    check("full_md starts with header", "Apeireth V1361" in full_md)
    check("full_md ends with guards", "GUARD_NO_FAKE_METRICS" in full_md)

    # Roundtrip: full_md is stable (no hidden state)
    full_md_2 = render_full_markdown(snap_dict)
    check("full_md deterministic (same input → same output)",
          full_md == full_md_2)

    # JSON output
    json_out = json.dumps(snap_dict, ensure_ascii=False)
    check("json serializable", len(json_out) > 100)

    # Delegate check — V1361 uses V1357's source-of-truth
    v1357 = _import_v1357()
    check("V1357 reachable", hasattr(v1357, "build_snapshot"))
    check("V1357 has VCP_TOOLCHAIN_MODULES", hasattr(v1357, "VCP_TOOLCHAIN_MODULES"))
    check("V1357 toolchain list has 11",
          len(v1357.VCP_TOOLCHAIN_MODULES) == 11,
          f"got {len(v1357.VCP_TOOLCHAIN_MODULES)}")

    # Read-only guarantee — V1361 doesn't write any files.
    # Use AST inspection: walk the module source and assert no
    # `open(..., mode containing write/append)`, no `Path.write_text`,
    # no `shutil.copy`, no `subprocess` write commands.
    import ast as _ast
    import apeireth.v1361_streamlit_dashboard as self_mod
    with open(self_mod.__file__, "r", encoding="utf-8") as f:
        src = f.read()
    tree = _ast.parse(src)
    write_calls: List[str] = []
    for node in _ast.walk(tree):
        if isinstance(node, _ast.Call):
            func = node.func
            name = None
            if isinstance(func, _ast.Name):
                name = func.id
            elif isinstance(func, _ast.Attribute):
                name = func.attr
            # Only flag these as writes if called as Attribute on a module name
            # or directly. (String method `.replace(...)` is NOT a write — skip.)
            if name in {"write_text", "write_bytes"}:
                write_calls.append(name)
            if name in {"copy", "copy2", "copytree", "move", "rename"}:
                write_calls.append(name)
            if name == "open":
                # Check mode kwarg / arg
                for kw in node.keywords:
                    if kw.arg == "mode" and isinstance(kw.value, _ast.Constant):
                        if isinstance(kw.value.value, str) and any(c in kw.value.value for c in "wax+"):
                            write_calls.append(f"open(mode={kw.value.value!r})")
                for arg in node.args:
                    if isinstance(arg, _ast.Constant) and isinstance(arg.value, str) and any(c in arg.value for c in "wax+"):
                        # positional mode (rare)
                        write_calls.append(f"open(<mode={arg.value!r}>)")
    check("V1361 has no write-mode open() / Path.write_text / shutil.copy",
          len(write_calls) == 0,
          f"found write calls: {write_calls}")

    # Streamlit is OPTIONAL — render_full_markdown should work without it
    # (we already proved it above; this is the structural assertion)
    check("V1361 source does not import streamlit at module top",
          "import streamlit" not in src.split("def serve")[0],
          "streamlit must be imported lazily inside serve()")

    return passed, total, failures


# -----------------------------------------------------------------------------
# CLI
# -----------------------------------------------------------------------------

def _cli_serve(args: argparse.Namespace) -> int:
    """Launch streamlit. We can't easily launch streamlit as a subprocess
    in the same process; instead, we instruct the user (or the cron agent)
    to run it as: streamlit run apeireth/v1361_streamlit_dashboard.py."""
    print("V1361 dashboard — to launch:")
    print("")
    print(f"  streamlit run apeireth/v1361_streamlit_dashboard.py --server.port {args.port}")
    print("")
    print("Or, programmatically, V1361.serve() can be called from a Python")
    print("script that has streamlit installed.")
    return 0


def _cli_render_json(_: argparse.Namespace) -> int:
    print(json.dumps(snapshot_to_dict(), indent=2, ensure_ascii=False))
    return 0


def _cli_render_md(_: argparse.Namespace) -> int:
    print(render_full_markdown())
    return 0


def _cli_self_test(args: argparse.Namespace) -> int:
    passed, total, failures = _popper_self_tests(verbose=args.verbose)
    print(f"V1361 self-test: {passed}/{total} passed")
    if failures:
        print("FAILURES:")
        for f in failures:
            print(f"  - {f}")
    return 0 if passed == total else 2


def _build_parser() -> argparse.ArgumentParser:
    p = argparse.ArgumentParser(
        prog="v1361-dashboard",
        description="Visual observability dashboard (any human can read)",
    )
    sub = p.add_subparsers(dest="command", required=True)

    p_sv = sub.add_parser("serve", help="launch streamlit app (instructions)")
    p_sv.add_argument("--port", type=int, default=8765)
    p_sv.set_defaults(func=_cli_serve)

    sub.add_parser("render-json", help="print snapshot JSON that dashboard consumes").set_defaults(
        func=_cli_render_json)
    sub.add_parser("render-md", help="print dashboard as Markdown (no streamlit needed)").set_defaults(
        func=_cli_render_md)

    p_st = sub.add_parser("self-test", help="Popper self-tests")
    p_st.add_argument("--verbose", action="store_true")
    p_st.set_defaults(func=_cli_self_test)

    sub.add_parser("version", help="print version").set_defaults(
        func=lambda a: print(f"v1361-streamlit-dashboard {V1361_VERSION}") or 0
    )

    return p


def main(argv: Optional[List[str]] = None) -> int:
    parser = _build_parser()
    args = parser.parse_args(argv)
    return int(args.func(args))


if __name__ == "__main__":
    # Dual-mode entry point:
    #   streamlit run apeireth/v1361_streamlit_dashboard.py
    #     → sys.argv == [script_path]; launch the streamlit UI.
    #   python -m apeireth.v1361_streamlit_dashboard <subcommand>
    #     → sys.argv == [script_path, subcommand, ...]; CLI mode.
    if len(sys.argv) == 1:
        sys.exit(serve())
    else:
        sys.exit(main(sys.argv[1:]))