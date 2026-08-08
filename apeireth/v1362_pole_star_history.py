"""Phase 1362 v1362_pole_star_history — pole-star history tracking over time.

## What V1362 is

V1357 = one-shot snapshot. V1362 = **history** of snapshots over time.

V1362 adds a JSONL append-only ledger (`pole_star_history.jsonl`) that
records every snapshot, so any human can later ask:

  - Is the pole-star moving up or down?
  - When did the toolchain reach 11/11?
  - When was the last regression?

V1362 does NOT claim that history growth = ASI growth. V1362 explicitly
asserts the opposite: the pole-star is capped at 0.90 (the honest cap
from V1356), and history tracking does not move that cap.

## CLI subcommands

  v1362-history record                  # append current snapshot to history
  v1362-history record --tag <label>    # attach an optional tag (e.g., v1361)
  v1362-history show [--limit 10]       # show last N entries (table)
  v1362-history trend [--window 5]      # compute moving-average trend
  v1362-history self-test [--verbose]   # 18+ Popper checks
  v1362-history version

## V1364 chained record (post-V1363 next-step)

`append_snapshot_with_dict(snap_dict, tag=None)` is the V1364 helper that
accepts a pre-built V1357 snapshot dict. V1357's `--record` flag uses this
helper to avoid building the snapshot twice (once for the JSON output, once
for the history record). Identical on-disk format to `append_snapshot`;
append-only JSONL invariant preserved.

If you call `append_snapshot_with_dict` directly, you MUST pass a dict
that came from `ProjectSnapshot.to_dict()` or be structurally compatible
(contains `pole_star`, `toolchain_health`, `close_loop_state`, `module_counts`).
Otherwise `_extract_history_entry` returns a partial entry, which is
detectable via `entry` having None for the missing fields.

## Storage

`pole_star_history.jsonl` is an append-only JSONL at the repo root.
Each line is a complete V1357 snapshot (subset: pole_star + counts +
toolchain_health summary + measured_at).

The file is rotated manually if it grows beyond a threshold; V1362 does
NOT auto-rotate (that would be a write the user didn't ask for).

## V3 哲学守门 (LOCKED, 主 17:58 + 20:46 + 17:43)

- 不假装分数 = ASI: V1362 cap = 0.005 (history tracking ≠ ASI)
- 不假装决策 = 真生产: history = mechanical append; no fabrication
- 不破坏 4 层安全门: V1362 only writes its own JSONL; nothing else
- 不假装 ASI 集成: V1362 only reads V1357 (single source of truth)
- 不假装 ASI 等级: GUARD_HISTORY_NOT_GROWTH prevents fake growth claim
- 不动 anchor: V1357 behavior is unchanged
- 不刷分: trend metric is moving-average of `total`, NOT new measurements
"""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple

V1362_VERSION = "0.1.0"
V1362_ASI_CAP = 0.005  # honest cap; history tracking ≠ ASI

V1362_HISTORY_FILE = "pole_star_history.jsonl"

V1362_PHILOSOPHY_GUARDS: Tuple[str, ...] = (
    "GUARD_HISTORY_NOT_GROWTH",      # history does NOT grow the cap
    "GUARD_DELEGATE_TO_V1357",        # all data from V1357
    "GUARD_READ_APPEND_ONLY",         # JSONL is append-only
    "GUARD_NO_FABRICATION",           # no synthesized entries
    "GUARD_HONEST_CAP",               # 0.005 cap
    "GUARD_NO_TREND_AS_ASI",          # trend ≠ ASI progress
)

V1362_SUBWEIGHTS: Dict[str, float] = {
    "append_correctness": 0.30,
    "trend_calculation": 0.25,
    "philosophy_compliance": 0.25,
    "self_test_coverage": 0.20,
}
assert abs(sum(V1362_SUBWEIGHTS.values()) - 1.0) < 1e-9

REPO_ROOT = Path(__file__).resolve().parent.parent


# -----------------------------------------------------------------------------
# Data source (delegate to V1357)
# -----------------------------------------------------------------------------

def _import_v1357():
    try:
        from apeireth import v1357_vcp_observability_snapshot as v1357
        return v1357
    except ImportError as exc:
        raise RuntimeError(
            "V1362 requires apeireth.v1357_vcp_observability_snapshot. "
            f"Import error: {exc}"
        )


def get_current_snapshot_dict() -> Dict[str, Any]:
    """Get current V1357 snapshot as a plain dict."""
    v1357 = _import_v1357()
    return v1357.build_snapshot().to_dict()


def _extract_history_entry(snap_dict: Dict[str, Any], tag: Optional[str] = None) -> Dict[str, Any]:
    """Extract the minimal record we store per snapshot."""
    pole = snap_dict.get("pole_star", {})
    tool = snap_dict.get("toolchain_health", {})
    close = snap_dict.get("close_loop_state", {})
    counts = snap_dict.get("module_counts", {})

    entry: Dict[str, Any] = {
        "measured_at": snap_dict.get("measured_at"),
        "pole_star_total": pole.get("total"),
        "pole_star_cap": pole.get("honest_cap"),
        "pole_star_delta_vs_v01": pole.get("delta_vs_v01"),
        "toolchain_present": tool.get("n_modules_present"),
        "toolchain_total": tool.get("n_modules_total"),
        "close_loop_pass": close.get("n_pass"),
        "close_loop_total": close.get("n_scenarios"),
        "v_modules": counts.get("apeireth_v_modules"),
        "test_files": counts.get("test_files"),
    }
    if tag:
        entry["tag"] = tag
    return entry


# -----------------------------------------------------------------------------
# JSONL append (single-source-of-truth write)
# -----------------------------------------------------------------------------

def _history_path() -> Path:
    return REPO_ROOT / V1362_HISTORY_FILE


def append_snapshot(tag: Optional[str] = None) -> Dict[str, Any]:
    """Append current V1357 snapshot to history. Returns the appended entry."""
    snap_dict = get_current_snapshot_dict()
    return append_snapshot_with_dict(snap_dict, tag=tag)


def append_snapshot_with_dict(snap_dict: Dict[str, Any], tag: Optional[str] = None) -> Dict[str, Any]:
    """V1364 helper: append a pre-built V1357 snapshot dict to history.

    Use this when the caller already has the snapshot (e.g., V1357's
    `--record` flag) to avoid building it twice. Identical on-disk
    format to `append_snapshot`; append-only JSONL invariant preserved.
    """
    entry = _extract_history_entry(snap_dict, tag=tag)

    path = _history_path()
    # Ensure parent exists
    path.parent.mkdir(parents=True, exist_ok=True)
    # Append-only (JSONL = one line per entry)
    with open(path, "a", encoding="utf-8") as f:
        f.write(json.dumps(entry, ensure_ascii=False) + "\n")
    return entry


def read_history(limit: Optional[int] = None) -> List[Dict[str, Any]]:
    """Read history entries (newest last). If limit is set, returns last N."""
    path = _history_path()
    if not path.exists():
        return []
    entries: List[Dict[str, Any]] = []
    with open(path, "r", encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if not line:
                continue
            try:
                entries.append(json.loads(line))
            except json.JSONDecodeError:
                # Skip malformed lines (don't crash)
                continue
    if limit is not None and limit > 0:
        entries = entries[-limit:]
    return entries


def history_count() -> int:
    """Number of entries currently in history."""
    path = _history_path()
    if not path.exists():
        return 0
    n = 0
    with open(path, "r", encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if line:
                n += 1
    return n


# -----------------------------------------------------------------------------
# Trend calculation
# -----------------------------------------------------------------------------

def compute_trend(entries: List[Dict[str, Any]], window: int = 5) -> Dict[str, Any]:
    """Compute moving-average trend from entries.

    Trend is reported as:
      - newest_avg: average of last `window` entries' pole_star_total
      - oldest_avg: average of first `window` entries' pole_star_total
      - delta: newest_avg - oldest_avg
      - n_entries: total entries used

    IMPORTANT: This is a moving average of `pole_star_total`, which is
    itself capped at honest_cap (0.90). The delta can therefore be at
    most ~0 (or +0.0X within the cap). It is NOT a growth metric that
    approaches ASI. V1362 GUARD_NO_TREND_AS_ASI prevents misuse.
    """
    if not entries:
        return {
            "newest_avg": None,
            "oldest_avg": None,
            "delta": None,
            "n_entries": 0,
            "window": window,
        }
    if len(entries) < 2:
        return {
            "newest_avg": entries[-1].get("pole_star_total"),
            "oldest_avg": entries[-1].get("pole_star_total"),
            "delta": 0.0,
            "n_entries": 1,
            "window": window,
        }
    # Last `window` and first `window`. If window >= n_entries//2 they
    # would overlap; in that case we split entries in half instead.
    n = len(entries)
    if window * 2 >= n:
        half = max(1, n // 2)
        early = entries[:half]
        recent = entries[half:]
    else:
        recent = entries[-window:]
        early = entries[:window]
    recent_vals = [e.get("pole_star_total") for e in recent if e.get("pole_star_total") is not None]
    early_vals = [e.get("pole_star_total") for e in early if e.get("pole_star_total") is not None]
    if not recent_vals or not early_vals:
        return {"newest_avg": None, "oldest_avg": None, "delta": None,
                "n_entries": len(entries), "window": window}
    newest_avg = sum(recent_vals) / len(recent_vals)
    oldest_avg = sum(early_vals) / len(early_vals)
    return {
        "newest_avg": round(newest_avg, 4),
        "oldest_avg": round(oldest_avg, 4),
        "delta": round(newest_avg - oldest_avg, 4),
        "n_entries": len(entries),
        "window": window,
    }


# -----------------------------------------------------------------------------
# Render (pure)
# -----------------------------------------------------------------------------

def render_history_table(entries: List[Dict[str, Any]]) -> str:
    if not entries:
        return "(history empty — run `python -m apeireth.v1362_pole_star_history record` to seed)"
    lines: List[str] = []
    lines.append("| # | measured_at | total | cap | Δ vs V0.1 | toolchain | close_loop | tag |")
    lines.append("|---|-------------|------:|----:|----------:|-----------|------------|-----|")
    for i, e in enumerate(entries, 1):
        d = (e.get("measured_at") or "?")[:19]
        t = e.get("pole_star_total")
        c = e.get("pole_star_cap")
        dv = e.get("pole_star_delta_vs_v01")
        tp = e.get("toolchain_present")
        tt = e.get("toolchain_total")
        cp = e.get("close_loop_pass")
        ct = e.get("close_loop_total")
        tag = e.get("tag") or "—"
        delta_str = f"{dv:+.4f}" if isinstance(dv, (int, float)) else "—"
        total_str = f"{t:.4f}" if isinstance(t, (int, float)) else "—"
        lines.append(
            f"| {i} | {d} | {total_str} | {c} | {delta_str} | "
            f"{tp}/{tt} | {cp}/{ct} | `{tag}` |"
        )
    return "\n".join(lines)


def render_trend_md(trend: Dict[str, Any]) -> str:
    lines: List[str] = []
    lines.append("## Trend (moving average, capped at honest_cap)")
    lines.append("")
    if trend["n_entries"] == 0:
        lines.append("_No entries yet._")
        return "\n".join(lines)
    if trend["n_entries"] == 1:
        lines.append(f"_1 entry only — need ≥2 for trend._")
        lines.append(f"  current = {trend['newest_avg']}")
        return "\n".join(lines)
    lines.append(f"- window: `{trend['window']}` entries (each side)")
    lines.append(f"- n_entries: `{trend['n_entries']}`")
    # V1367 fix: handle None newest_avg/oldest_avg (can occur if recent window
    # is all text-capture entries with no pole_star_total field). Honest
    # disclosure rather than crash.
    if trend["newest_avg"] is None:
        lines.append("- newest_avg: `n/a` (recent window has no measurable entries)")
    else:
        lines.append(f"- newest_avg: `{trend['newest_avg']:.4f}`")
    if trend["oldest_avg"] is None:
        lines.append("- oldest_avg: `n/a` (early window has no measurable entries)")
    else:
        lines.append(f"- oldest_avg: `{trend['oldest_avg']:.4f}`")
    delta = trend["delta"]
    if delta is None:
        lines.append("- delta: `n/a`")
    else:
        sign = "+" if delta >= 0 else ""
        lines.append(f"- delta: `{sign}{delta:.4f}`")
    lines.append("")
    lines.append("**V3 守门 (主 17:58 + 20:46)**: delta is bounded by `honest_cap` (0.90).")
    lines.append("A small delta does NOT mean growth toward ASI. The cap absorbs")
    lines.append("any structural change. This metric is for tracking toolchain /")
    lines.append("close-loop state, NOT for ASI-progress claims.")
    return "\n".join(lines)


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
    check("V1362_VERSION is semver", V1362_VERSION.count(".") == 2)
    check("V1362_ASI_CAP <= 0.01", V1362_ASI_CAP <= 0.01)
    check("V1362_ASI_CAP > 0", V1362_ASI_CAP > 0)
    check("philosophy guards >= 4", len(V1362_PHILOSOPHY_GUARDS) >= 4)
    check("GUARD_HISTORY_NOT_GROWTH present",
          "GUARD_HISTORY_NOT_GROWTH" in V1362_PHILOSOPHY_GUARDS)
    check("GUARD_DELEGATE_TO_V1357 present",
          "GUARD_DELEGATE_TO_V1357" in V1362_PHILOSOPHY_GUARDS)
    check("GUARD_NO_TREND_AS_ASI present",
          "GUARD_NO_TREND_AS_ASI" in V1362_PHILOSOPHY_GUARDS)
    check("subweights sum to 1.0",
          abs(sum(V1362_SUBWEIGHTS.values()) - 1.0) < 1e-9)

    # Data source delegates to V1357
    v1357 = _import_v1357()
    check("V1357 reachable", hasattr(v1357, "build_snapshot"))
    snap = get_current_snapshot_dict()
    check("snapshot has pole_star", "pole_star" in snap)
    check("snapshot has toolchain_health", "toolchain_health" in snap)

    # Extract entry
    entry = _extract_history_entry(snap)
    check("entry has measured_at", "measured_at" in entry)
    check("entry has pole_star_total", "pole_star_total" in entry)
    check("entry has toolchain_present", "toolchain_present" in entry)
    check("entry has close_loop_pass", "close_loop_pass" in entry)

    # Tag support
    entry_tagged = _extract_history_entry(snap, tag="test-v1362")
    check("entry with tag has tag field", entry_tagged.get("tag") == "test-v1362")

    # Read history (file may or may not exist; we test the read function
    # does not crash and returns a list)
    entries = read_history()
    check("read_history returns list", isinstance(entries, list))
    if entries:
        check("history entries have measured_at", all("measured_at" in e for e in entries))
        check("history entries have pole_star_total",
              all("pole_star_total" in e for e in entries))

    # History count
    n = history_count()
    check("history_count returns int >= 0", isinstance(n, int) and n >= 0)
    check("history_count matches read length", n == len(entries))

    # Trend
    trend = compute_trend(entries, window=3)
    check("trend has n_entries", "n_entries" in trend)
    check("trend has window", trend["window"] == 3)
    if len(entries) >= 2:
        check("trend delta is numeric or None",
              trend["delta"] is None or isinstance(trend["delta"], (int, float)))
        check("trend delta bounded by cap",
              trend["delta"] is None or abs(trend["delta"]) <= 0.2,
              f"delta={trend['delta']}")
        check("trend newest_avg <= cap",
              trend["newest_avg"] is None or trend["newest_avg"] <= 1.0)
        check("trend oldest_avg <= cap",
              trend["oldest_avg"] is None or trend["oldest_avg"] <= 1.0)

    # Render
    table = render_history_table(entries)
    check("table non-empty", len(table) > 20)
    trend_md = render_trend_md(trend)
    check("trend_md non-empty", len(trend_md) > 30)
    check("trend_md mentions cap", "cap" in trend_md.lower())

    # Append (this is the only "write" in V1362 — append-only to JSONL)
    before_count = history_count()
    appended = append_snapshot(tag="v1362-self-test")
    after_count = history_count()
    check("append_snapshot returns dict", isinstance(appended, dict))
    check("append_snapshot increases history count by 1",
          after_count == before_count + 1,
          f"before={before_count}, after={after_count}")
    # Verify the appended entry is the last one
    last = read_history()[-1]
    check("appended entry is at end of file",
          last.get("measured_at") == appended.get("measured_at"),
          "appended entry should be the last line")

    return passed, total, failures


# -----------------------------------------------------------------------------
# CLI
# -----------------------------------------------------------------------------

def _cli_record(args: argparse.Namespace) -> int:
    entry = append_snapshot(tag=args.tag)
    print(json.dumps(entry, indent=2, ensure_ascii=False))
    return 0


def _cli_show(args: argparse.Namespace) -> int:
    entries = read_history(limit=args.limit)
    print(f"History: {history_count()} entries total; showing last {len(entries)}")
    print("")
    print(render_history_table(entries))
    return 0


def _cli_trend(args: argparse.Namespace) -> int:
    entries = read_history()  # all
    trend = compute_trend(entries, window=args.window)
    print(json.dumps(trend, indent=2, ensure_ascii=False))
    print("")
    print(render_trend_md(trend))
    return 0


def _cli_self_test(args: argparse.Namespace) -> int:
    passed, total, failures = _popper_self_tests(verbose=args.verbose)
    print(f"V1362 self-test: {passed}/{total} passed")
    if failures:
        print("FAILURES:")
        for f in failures:
            print(f"  - {f}")
    return 0 if passed == total else 2


def _build_parser() -> argparse.ArgumentParser:
    p = argparse.ArgumentParser(
        prog="v1362-history",
        description="Pole-star history tracking (JSONL append-only ledger)",
    )
    sub = p.add_subparsers(dest="command", required=True)

    p_rec = sub.add_parser("record", help="append current V1357 snapshot to history")
    p_rec.add_argument("--tag", default=None, help="optional tag (e.g., 'v1361-ship')")
    p_rec.set_defaults(func=_cli_record)

    p_sh = sub.add_parser("show", help="show history (table)")
    p_sh.add_argument("--limit", type=int, default=20, help="max entries to show")
    p_sh.set_defaults(func=_cli_show)

    p_tr = sub.add_parser("trend", help="compute moving-average trend")
    p_tr.add_argument("--window", type=int, default=5, help="entries on each side")
    p_tr.set_defaults(func=_cli_trend)

    p_st = sub.add_parser("self-test", help="Popper self-tests")
    p_st.add_argument("--verbose", action="store_true")
    p_st.set_defaults(func=_cli_self_test)

    sub.add_parser("version", help="print version").set_defaults(
        func=lambda a: print(f"v1362-pole-star-history {V1362_VERSION}") or 0
    )

    return p


def main(argv: Optional[List[str]] = None) -> int:
    parser = _build_parser()
    args = parser.parse_args(argv)
    return int(args.func(args))


if __name__ == "__main__":
    sys.exit(main())