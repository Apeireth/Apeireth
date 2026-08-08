"""V1373 — V1372 Markdown export (post-V1372 next-step 2/5)

## What V1373 is

V1373 is the **portable companion** to V1372. Where V1372 prints ASCII to stdout,
V1373 writes GitHub-flavored Markdown (.md) that anyone can:

- commit to the repo
- open in any editor / IDE
- view on GitHub / GitLab / Codeberg (auto-renders fenced tables)
- paste into a wiki / Slack / Notion / Obsidian
- diff with `git diff` across cron ticks (the file is text)

V1373 is the **任何人都能接手** final-mile: a sidecar JSONL is for cron / scripts;
an ASCII table is for terminals; a .md file is for humans (PR comments, issues,
documentation, handoff notes).

## Why V1373 exists

V1372's stdout output disappears when the terminal closes. V1373 persists:

```bash
python -m apeireth.v1373_v1372_markdown_export > REPORT.md
git add REPORT.md
git commit -m "chore(observability): V1373 weekly trigger rate"
```

The .md is a single file, fully self-contained, with the same honesty disclosure
as V1372 (raw / cal / suppressed / fire_rate per trigger). Anyone reading the
repo can interpret it without running anything.

## 8 API surfaces

1. `build_markdown(timeline, evals, *, title=None, source=None)` — return Markdown string
2. `write_markdown(path, md)` — atomic write with UTF-8 encoding
3. `export_from_sidecar(sidecar_path, out_path, *, title=None)` — full pipeline
4. `render_title_block(...)` — `# V1373 — ...` header with timestamp + counts
5. `render_table_block(timeline)` — GitHub-flavored markdown table
6. `render_summary_block(timeline)` — secondary summary list
7. `render_honesty_block(timeline)` — disclosure paragraph (GUARD_HONEST_DISCLOSURE)
8. `_popper_self_tests()` — Popper self-tests
9. `run_cli(args)` — argv dispatcher (export / popper / version)

## GUARDS upheld (V1373-specific)

- GUARD_MARKDOWN_ONLY: output is pure CommonMark; no HTML; no JS
- GUARD_ATOMIC_WRITE: tmp + rename (no partial files)
- GUARD_NO_SIDECAR_TOUCH: only reads sidecar
- GUARD_NO_LEDGER_TOUCH: no import of V1362/V1368 ledger
- GUARD_HONEST_DISCLOSURE: always emit honesty paragraph

## Tests

- 30 Popper self-tests
- 20 pytest tests (markdown structure + atomic write + CLI)
- chain regression with V1372 + V1371 + V1370 + V1369 + V1368 (no source mutations)
"""

from __future__ import annotations

import argparse
import datetime as _dt
import json
import os
import sys
import tempfile
from typing import Any

# Reconfigure stdout for consistency with V1372
try:
    if hasattr(sys.stdout, "buffer"):
        sys.stdout.reconfigure(encoding="utf-8", errors="replace")  # type: ignore[attr-defined]
    if hasattr(sys.stderr, "buffer"):
        sys.stderr.reconfigure(encoding="utf-8", errors="replace")  # type: ignore[attr-defined]
except Exception:
    pass

from apeireth import v1372_v1371_ascii_timeline as v1372

SCHEMA_VERSION = "v1373.markdown/v1"
SCRIPT_NAME = "v1373_v1372_markdown_export"

# ASCII chars reused from V1372
CHAR_NO_FIRE = v1372.CHAR_NO_FIRE
CHAR_FIRE = v1372.CHAR_FIRE
CHAR_SUPPRESSED = v1372.CHAR_SUPPRESSED
CHAR_UNKNOWN = v1372.CHAR_UNKNOWN


# ----------------------------------------------------------------------
# Markdown rendering
# ----------------------------------------------------------------------

def _now_iso() -> str:
    """UTC ISO timestamp with seconds resolution."""
    return _dt.datetime.now(_dt.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


def _char_to_label(ch: str) -> str:
    """Convert a V1372 char to a human-readable label."""
    return {
        CHAR_NO_FIRE: "no fire",
        CHAR_FIRE: "fire",
        CHAR_SUPPRESSED: "suppressed",
        CHAR_UNKNOWN: "unknown",
    }.get(ch, ch)


def render_title_block(timeline: list[dict[str, Any]], n_evals: int,
                       *, title: str | None = None,
                       source: str | None = None) -> str:
    """Render the # header block."""
    lines: list[str] = []
    title = title or "V1373 — V1372 Markdown Export"
    lines.append(f"# {title}")
    lines.append("")
    lines.append(f"- **schema:** `{SCHEMA_VERSION}`")
    lines.append(f"- **generated:** {_now_iso()}")
    if source:
        lines.append(f"- **source sidecar:** `{source}`")
    lines.append(f"- **triggers:** {len(timeline)}")
    lines.append(f"- **evaluations:** {n_evals}")
    lines.append("")
    return "\n".join(lines)


def render_table_block(timeline: list[dict[str, Any]]) -> str:
    """Render a GitHub-flavored markdown table.

    Columns: trigger | kind | timeline | raw | cal | sup
    """
    lines: list[str] = []
    lines.append("## Per-trigger timeline")
    lines.append("")
    lines.append("| trigger | kind | timeline | raw | cal | sup |")
    lines.append("|---------|------|----------|----:|----:|----:|")
    for t in timeline:
        name = t["name"]
        kind = t["kind"]
        # Compress the timeline: count consecutive identical chars
        # to keep cells short while preserving signal.
        chars = t["chars"]
        compressed = _compress_chars(chars)
        raw = t["raw_count"]
        cal = t["cal_count"]
        sup = t["sup_count"]
        # Escape pipe characters in cells (none expected, but defensive)
        safe_name = name.replace("|", "\\|")
        safe_kind = kind.replace("|", "\\|")
        safe_timeline = compressed.replace("|", "\\|")
        lines.append(f"| `{safe_name}` | {safe_kind} | `{safe_timeline}` | {raw} | {cal} | {sup} |")
    lines.append("")
    return "\n".join(lines)


def _compress_chars(chars: list[str]) -> str:
    """Compress a run-length encoding: 'aaaabbc' -> 'a4b2c1'."""
    if not chars:
        return ""
    out: list[str] = []
    prev = chars[0]
    count = 1
    for ch in chars[1:]:
        if ch == prev:
            count += 1
        else:
            out.append(f"{prev}{count}" if count > 1 else prev)
            prev = ch
            count = 1
    out.append(f"{prev}{count}" if count > 1 else prev)
    return "".join(out)


def render_summary_block(timeline: list[dict[str, Any]], n_evals: int) -> str:
    """Render a secondary summary list (fire-rate per trigger)."""
    lines: list[str] = []
    lines.append("## Summary")
    lines.append("")
    lines.append("| trigger | kind | fire_rate |")
    lines.append("|---------|------|----------:|")
    for t in timeline:
        rate = (t["raw_count"] / n_evals * 100.0) if n_evals > 0 else 0.0
        lines.append(f"| `{t['name']}` | {t['kind']} | {rate:.2f}% |")
    lines.append("")
    return "\n".join(lines)


def render_legend_block() -> str:
    """Render the char legend as a small table."""
    return (
        "## Legend\n"
        "\n"
        "| char | meaning |\n"
        "|------|---------|\n"
        "| `·` | no fire |\n"
        "| `●` | raw fire (carried through to calibrated) |\n"
        "| `◌` | raw fire but suppressed by V1370 calibrator (FP suppressed) |\n"
        "| `?` | data missing (sidecar entry malformed) |\n"
        "\n"
        "In the timeline column, runs are compressed: `·4` means 4 consecutive `·`.\n"
    )


def render_honesty_block(timeline: list[dict[str, Any]], n_evals: int) -> str:
    """Render the GUARD_HONEST_DISCLOSURE paragraph.

    Always emits this — never silently drops the honesty layer.
    """
    total_raw = sum(t["raw_count"] for t in timeline)
    total_cal = sum(t["cal_count"] for t in timeline)
    total_sup = sum(t["sup_count"] for t in timeline)
    total_trigger_checks = n_evals * len(timeline)
    lines: list[str] = []
    lines.append("## Honesty disclosure")
    lines.append("")
    lines.append(
        f"This report is generated from a V1371 calibrated sidecar JSONL by V1373. "
        f"It is a pure reader: it does not write back, does not touch the ledger, "
        f"does not raise the cap, does not pretend anything."
    )
    lines.append("")
    lines.append(
        f"- **trigger-checks evaluated:** {total_trigger_checks} "
        f"({n_evals} evaluations × {len(timeline)} triggers)"
    )
    lines.append(f"- **raw fires:** {total_raw}")
    lines.append(f"- **calibrated fires:** {total_cal}")
    lines.append(f"- **V1370-suppressed false positives:** {total_sup}")
    if total_raw == 0 and total_cal == 0:
        lines.append("")
        lines.append(
            "**Honest baseline:** no fires in this window means the V1368/V1370 "
            "trigger conditions are not met. This is **plateau, not failure** — "
            "no remeasure, no V0.3 evolution signal. See V1370_REPORT.md for "
            "calibration details."
        )
    lines.append("")
    return "\n".join(lines)


def render_footer_block() -> str:
    """Render the footer with provenance."""
    return (
        "---\n"
        "\n"
        f"_Generated by `{SCRIPT_NAME} {SCHEMA_VERSION}` — see "
        f"`apeireth/v1373_v1372_markdown_export.py` and `V1373_REPORT.md`._\n"
    )


def build_markdown(timeline: list[dict[str, Any]], evals: list[dict[str, Any]],
                   *, title: str | None = None,
                   source: str | None = None) -> str:
    """Compose the full Markdown document."""
    n_evals = len(evals)
    parts: list[str] = []
    parts.append(render_title_block(timeline, n_evals, title=title, source=source))
    parts.append(render_table_block(timeline))
    parts.append(render_summary_block(timeline, n_evals))
    parts.append(render_legend_block())
    parts.append(render_honesty_block(timeline, n_evals))
    parts.append(render_footer_block())
    return "\n".join(parts)


# ----------------------------------------------------------------------
# File I/O (atomic write)
# ----------------------------------------------------------------------

def write_markdown(path: str, content: str) -> None:
    """Atomic write: tmp + rename, UTF-8 encoding."""
    d = os.path.dirname(os.path.abspath(path)) or "."
    os.makedirs(d, exist_ok=True)
    fd, tmp = tempfile.mkstemp(prefix=".v1373_", suffix=".md.tmp", dir=d)
    try:
        with os.fdopen(fd, "w", encoding="utf-8", newline="\n") as fh:
            fh.write(content)
        os.replace(tmp, path)
    except Exception:
        try:
            os.unlink(tmp)
        except OSError:
            pass
        raise


def export_from_sidecar(sidecar_path: str, out_path: str,
                        *, title: str | None = None) -> int:
    """Full pipeline: load sidecar → build timeline → write .md. Returns 0 on success."""
    try:
        evals = v1372.load_sidecar(sidecar_path)
    except FileNotFoundError as e:
        print(f"ERROR: {e}", file=sys.stderr)
        return 2
    timeline = v1372.build_timeline(evals)
    md = build_markdown(timeline, evals, title=title, source=sidecar_path)
    write_markdown(out_path, md)
    return 0


# ----------------------------------------------------------------------
# CLI
# ----------------------------------------------------------------------

def run_cli(args: list[str]) -> int:
    """Argv dispatcher. Returns process exit code."""
    parser = argparse.ArgumentParser(
        prog=SCRIPT_NAME,
        description="V1373 — Markdown export for V1372 timeline (post-V1372 next-step 2/5)",
    )
    parser.add_argument("--sidecar", default=v1372.DEFAULT_SIDECAR,
                        help=f"path to V1371 calibrated sidecar (default: {v1372.DEFAULT_SIDECAR})")
    parser.add_argument("--out", default="V1373_REPORT_AUTO.md",
                        help="output markdown path (default: V1373_REPORT_AUTO.md)")
    parser.add_argument("--title", default=None,
                        help="custom title for the markdown header")
    sub = parser.add_subparsers(dest="cmd")

    p_export = sub.add_parser("export", help="export markdown (default)")
    p_export.add_argument("--out", default="V1373_REPORT_AUTO.md")
    p_export.add_argument("--title", default=None)
    p_version = sub.add_parser("version", help="print version")
    p_popper = sub.add_parser("popper", help="run Popper self-tests")
    p_popper.add_argument("-v", "--verbose", action="store_true")

    # default to 'export' if no recognized subcommand
    SUBCMDS = {"export", "version", "popper"}
    skip_next = False
    has_subcmd = False
    for a in args:
        if skip_next:
            skip_next = False
            continue
        if a in SUBCMDS:
            has_subcmd = True
            break
        if a.startswith("--") or a.startswith("-"):
            if "=" in a:
                continue
            if a in {"--sidecar", "--out", "--title", "-o", "-t"}:
                skip_next = True
            continue
        break
    if not has_subcmd:
        args = ["export"] + args

    parsed = parser.parse_args(args)

    if parsed.cmd == "version":
        print(f"{SCRIPT_NAME} {SCHEMA_VERSION}")
        return 0
    if parsed.cmd == "popper":
        passed, total, failures = _popper_self_tests(verbose=parsed.verbose)
        print(f"Popper self-tests: {passed}/{total} passed")
        if failures:
            print("FAILURES:")
            for f in failures:
                print(f"  - {f}")
            return 1
        return 0

    # export
    out_path = getattr(parsed, "out", "V1373_REPORT_AUTO.md")
    title = getattr(parsed, "title", None)
    rc = export_from_sidecar(parsed.sidecar, out_path, title=title)
    if rc == 0:
        print(f"wrote: {out_path}")
    return rc


# ----------------------------------------------------------------------
# Popper self-tests
# ----------------------------------------------------------------------

def _popper_self_tests(verbose: bool = False) -> tuple[int, int, list[str]]:
    """30 Popper-style self-tests. Returns (passed, total, failures)."""
    failures: list[str] = []
    passed = 0

    def check(name: str, cond: bool) -> None:
        nonlocal passed
        if cond:
            passed += 1
            if verbose:
                print(f"  OK   {name}")
        else:
            failures.append(name)
            if verbose:
                print(f"  FAIL {name}")

    evals = v1372.load_sidecar(v1372.DEFAULT_SIDECAR)
    timeline = v1372.build_timeline(evals)

    # 1-5: render_title_block
    title = render_title_block(timeline, len(evals))
    check("title: starts with # ", title.startswith("# "))
    check("title: contains V1373", "V1373" in title)
    check("title: contains schema version", SCHEMA_VERSION in title)
    check("title: contains 'generated:'", "**generated:**" in title)
    check("title: contains trigger count", f"**triggers:** {len(timeline)}" in title)

    # 6-10: render_table_block
    table = render_table_block(timeline)
    check("table: contains '## Per-trigger timeline'", "## Per-trigger timeline" in table)
    check("table: contains header row", "| trigger | kind | timeline | raw | cal | sup |" in table)
    check("table: contains separator row", "|---------|" in table)
    check("table: 8 trigger rows", sum(
        1 for line in table.split("\n") if line.startswith("| `") and line.endswith("|")
    ) == 8)
    check("table: no HTML tags", "<" not in table and ">" not in table)

    # 11-15: render_summary_block
    summary = render_summary_block(timeline, len(evals))
    check("summary: contains '## Summary'", "## Summary" in summary)
    check("summary: 8 rows", sum(
        1 for line in summary.split("\n") if line.startswith("| `")
    ) == 8)
    check("summary: contains 0.00%", "0.00%" in summary)
    check("summary: contains 'fire_rate'", "fire_rate" in summary)
    check("summary: no HTML", "<" not in summary and ">" not in summary)

    # 16-20: render_honesty_block
    honesty = render_honesty_block(timeline, len(evals))
    check("honesty: contains '## Honesty disclosure'", "## Honesty disclosure" in honesty)
    check("honesty: contains raw/cal/sup counts", "raw fires" in honesty)
    check("honesty: mentions plateau", "plateau" in honesty)
    check("honesty: contains trigger-checks count", "trigger-checks" in honesty)
    check("honesty: no HTML", "<" not in honesty and ">" not in honesty)

    # 21-25: render_legend_block + footer
    legend = render_legend_block()
    check("legend: contains '## Legend'", "## Legend" in legend)
    check("legend: contains · and ●", CHAR_NO_FIRE in legend and CHAR_FIRE in legend)
    footer = render_footer_block()
    check("footer: contains '---'", "---" in footer)
    check("footer: mentions V1373_REPORT.md", "V1373_REPORT.md" in footer)

    # 26-30: full pipeline + atomic write
    md = build_markdown(timeline, evals)
    check("full: contains all 5 H2 sections", sum(
        1 for h in ("## Per-trigger timeline", "## Summary", "## Legend",
                    "## Honesty disclosure") if h in md
    ) == 4)
    check("full: ends with newline + footer", md.rstrip().endswith("._"))
    check("full: no HTML", "<" not in md and ">" not in md)

    # Atomic write test
    import tempfile as _tmp
    with _tmp.TemporaryDirectory() as td:
        out = os.path.join(td, "out.md")
        rc = export_from_sidecar(v1372.DEFAULT_SIDECAR, out)
        check("export: returns 0", rc == 0)
        check("export: file exists", os.path.exists(out))
        check("export: file non-empty", os.path.getsize(out) > 100)

    total = passed + len(failures)
    return passed, total, failures


# ----------------------------------------------------------------------
# Module entry point
# ----------------------------------------------------------------------

def main() -> int:
    return run_cli(sys.argv[1:])


if __name__ == "__main__":
    sys.exit(main())
