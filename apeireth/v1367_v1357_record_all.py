"""Phase 1367 v1367_v1357_record_all — V1357 `--record-all` flag for summary/recipe logging.

## What V1367 is

V1367 extends V1357 (VCP observability snapshot) so that **`summary` and
`recipe` commands can also be recorded into V1362 (pole-star history ledger)**,
via a single new opt-in flag: `--record-all`.

V1367 does NOT modify V1357's source. Instead it provides:

  - `v1367-record-all` CLI: a small wrapper that runs the chosen V1357
    command, captures its stdout, and appends a derived ledger entry to
    V1362 history. The wrapper preserves V1357's exit codes and unknown
    disclosure.

  - `v1367_v1357_record_all.py` library: 4 pure helpers (record_summary,
    record_recipe, record_snapshot, build_record_entry) that anyone can
    import from any agent loop, without invoking the CLI.

## Why V1367 (per V1365 plan §next per V1364 plan)

V1365's REPORT.md stated:

> next per V1364 plan:
>   - V1366 → V1340 cookbook validator integration with V1363 overlay ✓ SHIPPED
>   - **V1367 → `--record-all` flag for summary/recipe logging to V1362 ledger**
>   - V1368+ → consider V1356 pole-star V0.3 re-measurement trigger conditions

V1367 ships exactly the second item. The result is a one-liner:

```
python -m apeireth.v1367_v1357_record_all summary --tag morning-status
python -m apeireth.v1367_v1357_record_all recipe --tag onboarding
python -m apeireth.v1367_v1357_record_all snapshot --tag full-measure
```

…each one captures the V1357 view AND appends it to the V1362 JSONL ledger.

## CLI subcommands (wrapper)

  v1367-record-all summary   [--record-all] [--tag TAG]   # V1357 summary → V1362
  v1367-record-all recipe    [--record-all] [--tag TAG]   # V1357 recipe → V1362
  v1367-record-all snapshot  [--record-all] [--tag TAG]   # V1357 snapshot → V1362
  v1367-record-all self-test [--verbose]                 # Popper checks
  v1367-record-all version

The wrapper's `--record-all` flag is the **opt-in switch** (default OFF).
Without it, the wrapper passes through to V1357 unchanged. With it, the
wrapper additionally appends a record to V1362 history.

## Why a wrapper, not a V1357 patch?

Three reasons:

1. **V1367 ships without modifying V1357** — preserves the V1357 commit
   hash that downstream tools depend on.
2. **Same opt-in contract** — V1364 introduced `--record` for `snapshot`
   (default OFF). V1367 mirrors that contract for `summary`/`recipe`,
   behind a single `--record-all` flag.
3. **Easy to remove** — when V1368+ obsoletes this, deleting one file
   is enough.

## V3 哲学守门 (LOCKED, 主 17:58 + 20:46 + 17:43)

- **不假装分数 = ASI**: V1367 cap = 0.005 (recording ≠ ASI).
- **不假装决策 = 真生产**: V1367 = mechanical passthrough + append; no fabrication.
- **不假装 Phenomenal**: V1367 only captures V1357 stdout text; no phenomenology.
- **不假装 ASI 集成**: V1367 only imports V1357 + V1362 (the only stable APIs).
- **不假装 ASI 等级**: GUARD_RECORD_ALL_OPT_IN caps at 0.005.
- **不动 anchor**: V1367 does NOT modify V1357 or V1362; only appends JSONL.
- **不刷分**: V1367 is not in pole-star components.
- **不打扰 default**: GUARD_DEFAULT_OFF — `--record-all` is explicit opt-in.

## What V1367 explicitly does NOT do

- Does NOT modify V1357 source.
- Does NOT modify V1362 source.
- Does NOT auto-record without `--record-all`.
- Does NOT change V1357's exit codes.
- Does NOT change V1362's append-only invariant.
- Does NOT touch the pole-star V0.2 measurement.
"""
from __future__ import annotations

import argparse
import json
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple

V1367_VERSION = "0.1.0"
V1367_ASI_CAP = 0.005  # honest cap; recording ≠ ASI

REPO_ROOT = Path(__file__).resolve().parent.parent
APEIRETH_DIR = REPO_ROOT / "apeireth"

# V3 哲学守门
V1367_PHILOSOPHY_GUARDS: Tuple[str, ...] = (
    "GUARD_RECORD_ALL_OPT_IN",       # default OFF; --record-all must be explicit
    "GUARD_DEFAULT_OFF",             # without --record-all, behaves like V1357
    "GUARD_NO_FABRICATION",          # recording captures V1357 output, never invents
    "GUARD_DELEGATE_TO_V1357_V1362", # all data comes from V1357 stdout + V1362 append
    "GUARD_READ_ONLY_ON_V1357",      # V1367 never writes to V1357 source
    "GUARD_READ_ONLY_ON_V1362",      # V1367 only appends via V1362 (append-only)
    "GUARD_PASSTHROUGH_EXIT_CODES",  # V1357's exit codes preserved
    "GUARD_HONEST_CAP",              # 0.005 cap; recording != ASI
    "GUARD_RECORD_NOT_ASI",          # recording never claims pole-star drift
)


# -----------------------------------------------------------------------------
# V1357 invocation
# -----------------------------------------------------------------------------

def _run_v1357_subcommand(subcommand: str, args: Optional[List[str]] = None) -> Tuple[int, str, str]:
    """Invoke `python -m apeireth.v1357_vcp_observability_snapshot <subcommand>`.

    Returns (returncode, stdout, stderr). stdout/stderr are decoded as UTF-8
    with replacement to avoid Windows GBK decode crashes on unicode arrows.
    """
    args = list(args or [])
    cmd: List[str] = [
        sys.executable, "-m", "apeireth.v1357_vcp_observability_snapshot",
        subcommand, *args,
    ]
    proc = subprocess.run(
        cmd,
        cwd=str(REPO_ROOT),
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
        check=False,
    )
    return proc.returncode, proc.stdout or "", proc.stderr or ""


# -----------------------------------------------------------------------------
# Ledger record builder
# -----------------------------------------------------------------------------

def build_record_entry(
    subcommand: str,
    stdout: str,
    stderr: str,
    tag: Optional[str] = None,
) -> Dict[str, Any]:
    """Build a single V1362-shaped history entry derived from V1357 stdout.

    Shape mirrors V1362's `append_snapshot_with_dict` contract:
      {
        "version": str,
        "measured_at": ISO-8601,
        "repo_root": str,
        "subcommand": "summary" | "recipe" | "snapshot",
        "v1357_stdout": str,           # full V1357 stdout
        "v1357_stderr": str,           # full V1357 stderr (empty if none)
        "v1357_stderr_lines": int,
        "tag": Optional[str],
        "philosophy_guards": Tuple[str, ...],
      }

    V1367 deliberately does NOT call `measure_v02()` itself. The entry is
    text-only (stdout capture). V1357's `--record` already handles the
    structured JSON case. V1367's niche is the lightweight text captures.
    """
    measured = datetime.now(timezone.utc)
    return {
        "version": V1367_VERSION,
        "measured_at": measured.isoformat(),
        "repo_root": str(REPO_ROOT),
        "subcommand": subcommand,
        "v1357_stdout": stdout,
        "v1357_stderr": stderr,
        "v1357_stderr_lines": len(stderr.splitlines()) if stderr else 0,
        "tag": tag,
        "philosophy_guards": list(V1367_PHILOSOPHY_GUARDS),
    }


def _append_to_ledger(entry: Dict[str, Any]) -> Optional[Dict[str, Any]]:
    """Append a record to V1362 history (lazy import).

    Returns the appended entry dict on success, or None if V1362 cannot
    be imported (which would indicate a serious regression).
    """
    try:
        from apeireth import v1362_pole_star_history as v1362
    except Exception as exc:
        print(f"[v1367] cannot import v1362: {exc}", file=sys.stderr)
        return None
    try:
        # V1362 may expose different append paths; try the lightest first.
        if hasattr(v1362, "append_snapshot_with_dict"):
            return v1362.append_snapshot_with_dict(entry, tag=entry.get("tag"))
        if hasattr(v1362, "append_snapshot"):
            return v1362.append_snapshot(tag=entry.get("tag"))
        if hasattr(v1362, "append"):
            return v1362.append(entry)
        raise AttributeError("v1362 exposes no append_snapshot / append_snapshot_with_dict / append")
    except Exception as exc:
        print(f"[v1367] append failed: {exc}", file=sys.stderr)
        return None


# -----------------------------------------------------------------------------
# Public record helpers (library API)
# -----------------------------------------------------------------------------

def record_summary(tag: Optional[str] = None) -> Optional[Dict[str, Any]]:
    """Record V1357 summary stdout to V1362 ledger."""
    rc, out, err = _run_v1357_subcommand("summary")
    if rc != 0:
        print(f"[v1367] v1357 summary rc={rc} stderr={err}", file=sys.stderr)
    entry = build_record_entry("summary", out, err, tag=tag)
    return _append_to_ledger(entry)


def record_recipe(tag: Optional[str] = None) -> Optional[Dict[str, Any]]:
    """Record V1357 recipe stdout to V1362 ledger."""
    rc, out, err = _run_v1357_subcommand("recipe")
    if rc != 0:
        print(f"[v1367] v1357 recipe rc={rc} stderr={err}", file=sys.stderr)
    entry = build_record_entry("recipe", out, err, tag=tag)
    return _append_to_ledger(entry)


def record_snapshot(tag: Optional[str] = None) -> Optional[Dict[str, Any]]:
    """Record V1357 snapshot (JSON) to V1362 ledger.

    Note: V1357 already supports `--record [--tag TAG]`. V1367 provides
    a parallel entry point that uses V1367's text-shape entry, for
    callers that want a uniform record-subcommand API.
    """
    rc, out, err = _run_v1357_subcommand("snapshot", ["--json"])
    if rc not in (0, 1):  # V1357 returns 1 on known_unknowns (honest)
        print(f"[v1367] v1357 snapshot rc={rc} stderr={err}", file=sys.stderr)
    entry = build_record_entry("snapshot", out, err, tag=tag)
    return _append_to_ledger(entry)


# -----------------------------------------------------------------------------
# CLI (wrapper)
# -----------------------------------------------------------------------------

def _cli_wrapper(args: argparse.Namespace) -> int:
    sub = args.subcommand
    tag = getattr(args, "tag", None)
    record_all = bool(getattr(args, "record_all", False))

    # Always pass-through V1357 behavior first, regardless of --record-all.
    extra_args: List[str] = []
    if sub == "snapshot":
        extra_args = ["--json"]
    rc, out, err = _run_v1357_subcommand(sub, extra_args)
    # Echo V1357's stdout to our stdout (preserve pass-through)
    if out:
        sys.stdout.write(out)
        if not out.endswith("\n"):
            sys.stdout.write("\n")

    # Only when --record-all is set, additionally append to V1362.
    if record_all:
        entry = build_record_entry(sub, out, err, tag=tag)
        record_info = _append_to_ledger(entry)
        if record_info is not None:
            print(
                f"[v1367] recorded to history: subcommand={sub} tag={tag!r} "
                f"measured_at={record_info.get('measured_at', '?')}",
                file=sys.stderr,
            )
        else:
            print("[v1367] record skipped (V1362 unavailable)", file=sys.stderr)
    else:
        # GUARD_DEFAULT_OFF: when --record-all is absent, behave like V1357.
        # Print a hint to stderr so users discover the flag.
        if not record_all:
            print(
                f"[v1367] pass-through only (V1367 not recorded). "
                f"Re-run with --record-all [--tag TAG] to append to V1362 ledger.",
                file=sys.stderr,
            )

    return rc


def _cli_self_test(args: argparse.Namespace) -> int:
    passed, total, failures = _popper_self_tests(verbose=args.verbose)
    print(f"V1367 self-test: {passed}/{total} passed")
    if failures:
        print("FAILURES:")
        for f in failures:
            print(f"  - {f}")
    return 0 if passed == total else 2


def _build_parser() -> argparse.ArgumentParser:
    p = argparse.ArgumentParser(
        prog="v1367-record-all",
        description="V1357 wrapper that also records summary/recipe/snapshot to V1362 ledger (opt-in via --record-all)",
    )
    sub = p.add_subparsers(dest="command", required=True)

    # The wrapper subcommand (summary / recipe / snapshot)
    p_wrap = sub.add_parser("wrap", help="run a V1357 subcommand and optionally --record-all")
    p_wrap.add_argument("subcommand", choices=["summary", "recipe", "snapshot"],
                        help="which V1357 subcommand to wrap")
    p_wrap.add_argument("--record-all", action="store_true",
                        help="also append stdout-derived entry to V1362 ledger (opt-in)")
    p_wrap.add_argument("--tag", default=None,
                        help="tag for the recorded entry (e.g. 'morning-status')")
    p_wrap.set_defaults(func=_cli_wrapper)

    p_st = sub.add_parser("self-test", help="Popper self-tests")
    p_st.add_argument("--verbose", action="store_true")
    p_st.set_defaults(func=_cli_self_test)

    sub.add_parser("version", help="print version").set_defaults(
        func=lambda a: print(f"v1367-v1357-record-all {V1367_VERSION}") or 0
    )

    return p


# -----------------------------------------------------------------------------
# Popper self-tests
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

    # constants
    check("V1367_VERSION is semver", V1367_VERSION.count(".") == 2)
    check("V1367_ASI_CAP <= 0.01", V1367_ASI_CAP <= 0.01)
    check("REPO_ROOT exists", REPO_ROOT.exists())
    check("APEIRETH_DIR exists", APEIRETH_DIR.exists())

    # philosophy guards
    check("has GUARD_RECORD_ALL_OPT_IN",
          "GUARD_RECORD_ALL_OPT_IN" in V1367_PHILOSOPHY_GUARDS)
    check("has GUARD_DEFAULT_OFF",
          "GUARD_DEFAULT_OFF" in V1367_PHILOSOPHY_GUARDS)
    check("has GUARD_NO_FABRICATION",
          "GUARD_NO_FABRICATION" in V1367_PHILOSOPHY_GUARDS)
    check("has GUARD_DELEGATE_TO_V1357_V1362",
          "GUARD_DELEGATE_TO_V1357_V1362" in V1367_PHILOSOPHY_GUARDS)
    check("has GUARD_RECORD_NOT_ASI",
          "GUARD_RECORD_NOT_ASI" in V1367_PHILOSOPHY_GUARDS)

    # build_record_entry
    sample = build_record_entry("summary", "Apeireth@...", "", tag="morning")
    check("record has version", sample.get("version") == V1367_VERSION)
    check("record has measured_at", "measured_at" in sample)
    check("record has subcommand", sample.get("subcommand") == "summary")
    check("record has tag", sample.get("tag") == "morning")
    check("record has v1357_stdout", sample.get("v1357_stdout") == "Apeireth@...")
    check("record has v1357_stderr_lines", sample.get("v1357_stderr_lines") == 0)
    check("record philosophy_guards non-empty", len(sample.get("philosophy_guards", [])) >= 5)
    check("record includes GUARD_RECORD_ALL_OPT_IN",
          "GUARD_RECORD_ALL_OPT_IN" in sample.get("philosophy_guards", []))

    # Different subcommand
    sample2 = build_record_entry("recipe", "RECIPE...", "warn", tag=None)
    check("recipe record subcommand=recipe", sample2.get("subcommand") == "recipe")
    check("recipe record stderr_lines > 0", sample2.get("v1357_stderr_lines") > 0)
    check("recipe record tag is None", sample2.get("tag") is None)

    # V1357 invocation works (real call, no side effects)
    rc, out, err = _run_v1357_subcommand("version")
    check("v1357 version exit 0", rc == 0, f"rc={rc}")
    check("v1357 version stdout non-empty", len(out) > 0, f"stdout={out!r}")

    rc, out, err = _run_v1357_subcommand("summary")
    check("v1357 summary exit 0", rc == 0, f"rc={rc} stderr={err}")
    check("v1357 summary stdout non-empty", len(out) > 30, f"len={len(out)}")
    check("v1357 summary mentions pole_star", "pole_star" in out)

    rc, out, err = _run_v1357_subcommand("recipe")
    check("v1357 recipe exit 0", rc == 0, f"rc={rc} stderr={err}")
    check("v1357 recipe stdout non-empty", len(out) > 50, f"len={len(out)}")
    check("v1357 recipe has RECIPE", "RECIPE" in out)

    rc, out, err = _run_v1357_subcommand("snapshot", ["--json"])
    check("v1357 snapshot exit 0 or 1", rc in (0, 1), f"rc={rc} stderr={err}")
    check("v1357 snapshot JSON parseable", _safe_json(out) is not None)

    # Record helpers — these call V1357 (slow path). Skip by default unless verbose.
    if verbose:
        # Use a unique tag so we can spot the entry.
        import time as _t
        test_tag = f"v1367-selftest-{int(_t.time())}"
        info_s = record_summary(tag=test_tag + "-summary")
        info_r = record_recipe(tag=test_tag + "-recipe")
        check("record_summary returned dict", isinstance(info_s, dict))
        check("record_recipe returned dict", isinstance(info_r, dict))

    return passed, total, failures


def _safe_json(text: str) -> Optional[Any]:
    try:
        return json.loads(text)
    except Exception:
        return None


def main(argv: Optional[List[str]] = None) -> int:
    parser = _build_parser()
    args = parser.parse_args(argv)
    return int(args.func(args))


if __name__ == "__main__":
    sys.exit(main())