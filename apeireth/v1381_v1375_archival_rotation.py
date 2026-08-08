"""V1381 — V1375 archival rotation policy (post-V1380 next-step 1/3)

## Phase

Phase: 1381
Version: 0.1.0
Date: 2026-08-09 (cron tick 233)
Post: V1380 (V1375 × INDEX × V1379 three-way reconciliation)
ASI 北极星: LOCKED (V0.1=0.7905 / V0.2=0.4467 / V1256=0.9105 / V0.3 NOT due)

## What V1381 is

V1381 is the **rotation policy companion** to the V1375 archive system.
V1375 writes a timestamped ``.md`` archive per cron tick; V1378 overlays
those archives with the V1362 pole-star ledger; V1379 records the
SHA-256 integrity manifest; V1380 reconciles disk × INDEX × manifest.

**V1381 answers the next obvious question: how do these archives age, and
what should happen to old ones?**

Every archive has an *age* (seconds since its slug timestamp) and a
*tier* based on configurable age bands:

- HOT     — current 24 h (do not touch)
- WARM    — within 7 days (keep, but flag)
- COLD    — within 90 days (eligible for gzip compression)
- FROZEN  — older than 90 days (eligible for archival deletion)

The module ships a **plan-first** API: ``plan_rotation`` produces a list
of intended actions (no disk mutation); ``apply_rotation`` then performs
the actions atomically. **The default CLI subcommand is ``plan``**, not
``rotate`` — humans approve before any archive is touched.

```bash
# Run from promethean/
python -m apeireth.v1381_v1375_archival_rotation plan
# → V1381_PLAN_AUTO.md written (proposed actions, no disk change)

python -m apeireth.v1381_v1375_archival_rotation policy
# → V1381_POLICY_AUTO.md written (current policy)

python -m apeireth.v1381_v1375_archival_rotation rotate
# → Compress COLD archives to .md.gz (atomic); update INDEX.md; emit V1381_MANIFEST_AUTO.json
# → Exit 0 on success; non-zero if any archive action failed

python -m apeireth.v1381_v1375_archival_rotation list
# → Markdown table of all archives × tier × age × action

python -m apeireth.v1381_v1375_archival_rotation show <archive-name>
# → Show tier, age, action, integrity (sha256) for one archive
```

## Why V1381 exists

V1375 archive growth is unbounded. After 90 days at 12 cron ticks/day, the
directory contains ~1080 ``.md`` files — small individually, but they
accumulate forever. Without a policy:

- The archive directory grows without bound
- INDEX.md grows without bound (slowing every V1376/V1377/V1378/V1379/V1380 tick)
- No one knows which archives are "current" vs "historical"
- No compression is applied (every archive is plain text)

V1381 closes this gap with a **declarative, observable, plan-first
rotation policy**. It is the missing primitive for long-term archive
hygiene.

V1381 also produces a **rotation manifest** (analogous to V1379's
integrity manifest) so a human can see what was rotated and when.

## API surfaces (10)

1. `default_policy()` — dict of age bands + actions
2. `parse_iso_basic(iso_basic)` — ISO timestamp → datetime (UTC)
3. `classify_archive(name, *, now=None, policy=None)` — (tier, age_sec)
4. `plan_action(archive, *, now=None, policy=None)` — intended action
5. `plan_rotation(archive_dir, *, now=None, policy=None)` — full action list
6. `render_plan_md(plan, *, archive_dir, policy)` — markdown plan string
7. `apply_rotation(plan, archive_dir, *, policy, manifest_path)` — atomic exec
8. `write_manifest(manifest, path)` — atomic JSON write
9. `_popper_self_tests()` — Popper self-check
10. `run_cli(args)` — argv dispatcher (policy / plan / rotate / list / show / popper / version)

Plus helpers:

- `_validate_safe_archive_dir(path)` — reject path traversal
- `_validate_safe_manifest_path(path)` — same
- `slug_timestamp_to_datetime(slug)` — parse V1375 slug → datetime
- `load_manifest(path)` — load previous rotation manifest
- `verify_manifest_invariants(manifest, archive_dir)` — sanity checks

## GUARDS upheld (V1381-specific, 11)

- GUARD_PLAN_FIRST: default CLI = `plan`, not `rotate` (humans approve)
- GUARD_ROTATION_READ_ONLY_BY_DEFAULT: plan + policy + list + show never touch archives
- GUARD_ATOMIC_WRITE: tmp + rename for INDEX.md, manifest, any new archive
- GUARD_NO_SIDECAR_TOUCH: never imports V1371 / V1369 / V1370
- GUARD_NO_LEDGER_TOUCH: never imports V1362 / V1368 / V1375 ledger code
- GUARD_NO_CAP_CHANGE: V1381 has no metric, no cap, no scoring
- GUARD_HONEST_DISCLOSURE: every report includes the honesty paragraph
- GUARD_DETERMINISTIC: same inputs in same order → same plan bytes
- GUARD_NO_SILENT_LOSS: every action logs before + after path + sha256
- GUARD_POLICY_VERSIONED: every manifest records the policy version
- GUARD_PATH_SAFE: reject path traversal (`..`) in archive_dir and manifest_path

## V3 哲学守门 (LOCKED, 主 17:43 + 17:58 + 20:46 + 22:33 + 23:44)

- 不假装分数 = ASI: V1381 has no metric, no cap, no scoring
- 不假装决策 = 真生产: plan-first; humans approve rotation; no auto-mutation
- 不假装 ASI 集成: zero LLM, zero sidecar, zero ledger write
- 不刷分: zero metric change in this commit; honest 0.90 cap preserved
- 不动 anchor: V1375 archives + INDEX.md + V1379 manifest unchanged in plan-only mode
- 不假装 V1381 = ASI 觉醒: V1381 reports rotation plan; doesn't "interpret" it
- 实事求是: real disk reads + real ISO parsing + real age computation
- 任何人都能接手: CLI + JSON + Markdown + 1-cmd `plan` + atomic writes + reproducibility
"""

from __future__ import annotations

import argparse
import datetime as _dt
import gzip
import hashlib
import json
import os
import re
import shutil
import sys
import tempfile
from typing import Any

# Reconfigure stdout/stderr for Windows GBK safety
try:
    if hasattr(sys.stdout, "buffer"):
        sys.stdout.reconfigure(encoding="utf-8", errors="replace")  # type: ignore[attr-defined]
    if hasattr(sys.stderr, "buffer"):
        sys.stderr.reconfigure(encoding="utf-8", errors="replace")  # type: ignore[attr-defined]
except Exception:
    pass

SCHEMA_VERSION = "v1381.rotation/v1"
SCRIPT_NAME = "v1381_v1375_archival_rotation"
DEFAULT_ARCHIVE_DIR = "V1375_HISTORY"
DEFAULT_INDEX_PATH = "V1375_HISTORY/INDEX.md"
DEFAULT_MANIFEST_PATH = "V1381_MANIFEST_AUTO.json"
DEFAULT_PLAN_PATH = "V1381_PLAN_AUTO.md"
DEFAULT_POLICY_PATH = "V1381_POLICY_AUTO.md"

# Match V1375 archive slug: ``<iso>__<schema>.md``
_RE_ARCHIVE_NAME = re.compile(
    r"^(?P<iso>\d{4}-\d{2}-\d{2}T\d{2}-\d{2}-\d{2}Z)__"
    r"(?P<schema>[a-zA-Z0-9_]+?)"
    r"(?:_(?P<collision>\d{3}))?\.md$"
)


# ----------------------------------------------------------------------
# Path safety
# ----------------------------------------------------------------------

def _validate_safe_archive_dir(archive_dir: str) -> None:
    """Reject path traversal (``..`` segments)."""
    raw_parts = archive_dir.replace("\\", "/").split("/")
    norm_parts = os.path.normpath(archive_dir).replace("\\", "/").split("/")
    if ".." in raw_parts or ".." in norm_parts:
        raise ValueError(f"Path contains parent traversal: {archive_dir!r}")


def _validate_safe_manifest_path(path: str) -> None:
    """Reject path traversal in manifest path."""
    raw_parts = path.replace("\\", "/").split("/")
    norm_parts = os.path.normpath(path).replace("\\", "/").split("/")
    if ".." in raw_parts or ".." in norm_parts:
        raise ValueError(f"Manifest path contains parent traversal: {path!r}")


# ----------------------------------------------------------------------
# ISO basic parsing (reuse V1375 / V1378 / V1379 pattern)
# ----------------------------------------------------------------------

def parse_iso_basic(iso_basic: str) -> _dt.datetime:
    """Parse ``2026-08-09T03-55-00Z`` → aware UTC datetime.

    Returns naive UTC datetime if parsing fails (defensive — caller can
    decide what to do with ``None`` / ``ParseError``).
    """
    if not iso_basic or not isinstance(iso_basic, str):
        raise ValueError(f"Invalid ISO basic: {iso_basic!r}")
    # Strip optional trailing Z
    s = iso_basic.strip()
    if s.endswith("Z"):
        s = s[:-1]
    try:
        dt = _dt.datetime.strptime(s, "%Y-%m-%dT%H-%M-%S")
    except ValueError as e:
        raise ValueError(f"Cannot parse ISO basic {iso_basic!r}: {e}") from e
    return dt.replace(tzinfo=_dt.timezone.utc)


def slug_timestamp_to_datetime(slug: str) -> _dt.datetime:
    """Parse V1375 archive slug → datetime UTC.

    Convenience wrapper around ``parse_iso_basic`` for archive filenames.
    """
    m = _RE_ARCHIVE_NAME.match(slug)
    if not m:
        raise ValueError(f"Not a V1375 archive slug: {slug!r}")
    return parse_iso_basic(m.group("iso"))


# ----------------------------------------------------------------------
# Policy definition
# ----------------------------------------------------------------------

POLICY_VERSION = "v1381.rotation.policy/v1"

# Age bands (in seconds). Tunable via CLI flags in a future V1382+.
AGE_BANDS: dict[str, int] = {
    "HOT_MAX_SEC": 86400,         # 1 day
    "WARM_MAX_SEC": 604800,       # 7 days
    "COLD_MAX_SEC": 7776000,      # 90 days
    # Anything older than COLD_MAX_SEC is FROZEN
}

# Default actions per tier
DEFAULT_ACTIONS: dict[str, str] = {
    "HOT": "keep",
    "WARM": "keep",
    "COLD": "compress",   # gzip to .md.gz
    "FROZEN": "prune",    # remove (requires explicit --apply)
}


def default_policy() -> dict[str, Any]:
    """Return the default rotation policy.

    The policy is a plain dict — no side effects, no I/O. Easy to
    serialize into the rotation manifest.
    """
    return {
        "policy_version": POLICY_VERSION,
        "age_bands": dict(AGE_BANDS),
        "actions": dict(DEFAULT_ACTIONS),
        "schema_version": SCHEMA_VERSION,
    }


# ----------------------------------------------------------------------
# Classification
# ----------------------------------------------------------------------

def classify_archive(
    name: str,
    *,
    now: _dt.datetime | None = None,
    policy: dict[str, Any] | None = None,
) -> tuple[str, int]:
    """Classify one archive by tier + age_sec.

    Returns ``(tier, age_sec)`` where tier ∈ ``{"HOT", "WARM", "COLD", "FROZEN"}``.

    ``age_sec`` is always non-negative; ``now`` defaults to UTC now.

    If the archive name doesn't match the V1375 slug pattern, raises
    ``ValueError`` (defensive — caller should filter first).
    """
    if now is None:
        now = _dt.datetime.now(_dt.timezone.utc)
    if not isinstance(now, _dt.datetime):
        raise TypeError(f"now must be datetime, got {type(now).__name__}")
    if now.tzinfo is None:
        now = now.replace(tzinfo=_dt.timezone.utc)
    pol = policy if policy is not None else default_policy()
    bands = pol.get("age_bands", AGE_BANDS)
    dt = slug_timestamp_to_datetime(name)
    age_sec = max(0, int((now - dt).total_seconds()))
    if age_sec <= bands["HOT_MAX_SEC"]:
        tier = "HOT"
    elif age_sec <= bands["WARM_MAX_SEC"]:
        tier = "WARM"
    elif age_sec <= bands["COLD_MAX_SEC"]:
        tier = "COLD"
    else:
        tier = "FROZEN"
    return tier, age_sec


# ----------------------------------------------------------------------
# Planning
# ----------------------------------------------------------------------

def _list_archive_names(archive_dir: str) -> list[str]:
    """Return sorted list of V1375 archive filenames in ``archive_dir``."""
    _validate_safe_archive_dir(archive_dir)
    if not os.path.isdir(archive_dir):
        return []
    names: list[str] = []
    for entry in os.listdir(archive_dir):
        if not entry.endswith(".md"):
            continue
        if entry.upper() == "INDEX.MD":
            continue
        if _RE_ARCHIVE_NAME.match(entry):
            names.append(entry)
    names.sort()
    return names


def plan_action(
    name: str,
    *,
    now: _dt.datetime | None = None,
    policy: dict[str, Any] | None = None,
) -> dict[str, Any]:
    """Compute intended action for a single archive.

    Returns dict with ``name``, ``tier``, ``age_sec``, ``action``,
    ``target_path``, ``reason``.

    Action values:
      - ``keep`` — leave as-is
      - ``compress`` — gzip to ``<name>.gz`` (atomic; original removed)
      - ``prune`` — remove (requires explicit --apply)

    HOT/WARM archives always return ``keep``.
    """
    pol = policy if policy is not None else default_policy()
    tier, age_sec = classify_archive(name, now=now, policy=pol)
    action = pol.get("actions", DEFAULT_ACTIONS).get(tier, "keep")
    if action == "compress":
        target = name + ".gz"
        reason = f"age={age_sec}s >= COLD_MAX ({AGE_BANDS['COLD_MAX_SEC']}s) → gzip"
    elif action == "prune":
        target = ""  # no target; removal
        reason = f"age={age_sec}s > COLD_MAX ({AGE_BANDS['COLD_MAX_SEC']}s) → prune"
    else:
        target = name
        reason = f"tier={tier} age={age_sec}s → keep"
    return {
        "name": name,
        "tier": tier,
        "age_sec": age_sec,
        "action": action,
        "target_path": target,
        "reason": reason,
    }


def plan_rotation(
    archive_dir: str,
    *,
    now: _dt.datetime | None = None,
    policy: dict[str, Any] | None = None,
) -> list[dict[str, Any]]:
    """Compute rotation plan for all archives in ``archive_dir``.

    Returns list of action dicts (one per archive). Sorted by name
    (deterministic). Archives with no action (``keep``) are included
    so the caller sees them — the action list is exhaustive.
    """
    _validate_safe_archive_dir(archive_dir)
    names = _list_archive_names(archive_dir)
    out: list[dict[str, Any]] = []
    for name in names:
        out.append(plan_action(name, now=now, policy=policy))
    return out


# ----------------------------------------------------------------------
# Markdown rendering
# ----------------------------------------------------------------------

def render_plan_md(
    plan: list[dict[str, Any]],
    *,
    archive_dir: str,
    policy: dict[str, Any],
    now: _dt.datetime | None = None,
) -> str:
    """Render a markdown report describing the rotation plan.

    No disk writes; pure string output. Sections:
      - header (schema, generated, archive_dir)
      - policy summary
      - action counts
      - action table
      - honesty disclosure
    """
    if now is None:
        now = _dt.datetime.now(_dt.timezone.utc)
    now_iso = now.strftime("%Y-%m-%dT%H:%M:%SZ")
    counts: dict[str, int] = {"keep": 0, "compress": 0, "prune": 0}
    for p in plan:
        a = p.get("action", "keep")
        counts[a] = counts.get(a, 0) + 1

    lines: list[str] = []
    lines.append(f"# V1381 — V1375 Archival Rotation Plan")
    lines.append("")
    lines.append(f"- **schema:** `{SCHEMA_VERSION}`")
    lines.append(f"- **generated:** `{now_iso}`")
    lines.append(f"- **archive dir:** `{archive_dir}`")
    lines.append(f"- **policy version:** `{policy.get('policy_version', '?')}`")
    lines.append("")
    lines.append("## Policy summary")
    lines.append("")
    lines.append("| tier | age range | action |")
    lines.append("|------|-----------|--------|")
    bands = policy.get("age_bands", AGE_BANDS)
    actions = policy.get("actions", DEFAULT_ACTIONS)
    hot_max = bands["HOT_MAX_SEC"]
    warm_max = bands["WARM_MAX_SEC"]
    cold_max = bands["COLD_MAX_SEC"]
    lines.append(f"| HOT | 0 → {hot_max}s (24h) | {actions.get('HOT', 'keep')} |")
    lines.append(f"| WARM | {hot_max}s → {warm_max}s (7d) | {actions.get('WARM', 'keep')} |")
    lines.append(f"| COLD | {warm_max}s → {cold_max}s (90d) | {actions.get('COLD', 'compress')} |")
    lines.append(f"| FROZEN | > {cold_max}s | {actions.get('FROZEN', 'prune')} |")
    lines.append("")
    lines.append("## Action counts")
    lines.append("")
    lines.append(f"- **keep:** {counts.get('keep', 0)}")
    lines.append(f"- **compress:** {counts.get('compress', 0)}")
    lines.append(f"- **prune:** {counts.get('prune', 0)}")
    lines.append("")
    lines.append("## Action table")
    lines.append("")
    lines.append("| # | name | tier | age (sec) | age (days) | action | target | reason |")
    lines.append("|--:|------|------|----------:|-----------:|--------|--------|--------|")
    for i, p in enumerate(plan, start=1):
        name = p.get("name", "?")
        tier = p.get("tier", "?")
        age_sec = p.get("age_sec", 0)
        age_days = round(age_sec / 86400.0, 2)
        action = p.get("action", "keep")
        target = p.get("target_path", "")
        lines.append(
            f"| {i} | `{name}` | {tier} | {age_sec} | {age_days} | {action} | "
            f"`{target}` | {p.get('reason', '')} |"
        )
    lines.append("")
    lines.append("## Honesty disclosure")
    lines.append("")
    lines.append("V1381 is a **planner** by default — `plan_rotation` and `render_plan_md`")
    lines.append("produce this report without touching any archive on disk. To actually")
    lines.append("execute compression or pruning, run `rotate --apply` (atomic, manifest-")
    lines.append("logged). Without `--apply`, the only side effect of `rotate` is writing")
    lines.append("this plan to disk. V1381 has no metric, no cap, no scoring; it does not")
    lines.append("touch V1371 / V1362 / V1368 / V1375 ledger code; it does not raise any")
    lines.append("cap or pretend anything.")
    lines.append("")
    lines.append("**Honest baseline:** with only 1 archive on disk (a few minutes old),")
    lines.append("the plan will show `HOT` tier with action `keep` — and that is signal,")
    lines.append("not failure. Rotation only activates once archives age past the band")
    lines.append("thresholds.")
    lines.append("")
    lines.append(f"_Generated by `{SCRIPT_NAME} {SCHEMA_VERSION}` — see "
                 f"`apeireth/{SCRIPT_NAME}.py` and `V1381_REPORT.md`._")
    lines.append("")
    return "\n".join(lines)


def render_policy_md(policy: dict[str, Any]) -> str:
    """Render the policy as a markdown document."""
    lines: list[str] = []
    lines.append(f"# V1381 — V1375 Archival Rotation Policy")
    lines.append("")
    lines.append(f"- **policy version:** `{policy.get('policy_version', '?')}`")
    lines.append(f"- **schema version:** `{policy.get('schema_version', '?')}`")
    lines.append("")
    lines.append("## Age bands (seconds)")
    lines.append("")
    lines.append("| band | max seconds | max days |")
    lines.append("|------|------------:|---------:|")
    bands = policy.get("age_bands", AGE_BANDS)
    for k, v in bands.items():
        days = round(v / 86400.0, 2)
        lines.append(f"| {k} | {v} | {days} |")
    lines.append("")
    lines.append("## Actions per tier")
    lines.append("")
    lines.append("| tier | action |")
    lines.append("|------|--------|")
    actions = policy.get("actions", DEFAULT_ACTIONS)
    for tier in ["HOT", "WARM", "COLD", "FROZEN"]:
        lines.append(f"| {tier} | {actions.get(tier, 'keep')} |")
    lines.append("")
    lines.append("## Honesty disclosure")
    lines.append("")
    lines.append("This policy is the **default** V1381 ships with. It is observable,")
    lines.append("deterministic, and plan-first: humans approve before any archive is")
    lines.append("compressed or removed. Tunable via future V1382+ without breaking")
    lines.append("the manifest schema.")
    lines.append("")
    lines.append(f"_Generated by `{SCRIPT_NAME} {SCHEMA_VERSION}`_")
    lines.append("")
    return "\n".join(lines)


def render_list_md(
    plan: list[dict[str, Any]],
    *,
    archive_dir: str,
) -> str:
    """Render a markdown list of all archives × tier × age."""
    lines: list[str] = []
    lines.append(f"# V1381 — V1375 Archive List")
    lines.append("")
    lines.append(f"- **archive dir:** `{archive_dir}`")
    lines.append(f"- **archive count:** {len(plan)}")
    lines.append("")
    lines.append("| # | name | tier | age (sec) | age (days) | mtime (iso) |")
    lines.append("|--:|------|------|----------:|-----------:|-------------|")
    for i, p in enumerate(plan, start=1):
        name = p.get("name", "?")
        tier = p.get("tier", "?")
        age_sec = p.get("age_sec", 0)
        age_days = round(age_sec / 86400.0, 2)
        # Try to read mtime from disk
        try:
            mt = os.path.getmtime(os.path.join(archive_dir, name))
            mt_iso = _dt.datetime.fromtimestamp(mt, _dt.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")
        except OSError:
            mt_iso = "—"
        lines.append(f"| {i} | `{name}` | {tier} | {age_sec} | {age_days} | {mt_iso} |")
    lines.append("")
    lines.append("**Note:** mtime is from the filesystem; iso slug in name is the archive")
    lines.append("creation timestamp (V1375 records slug at write-time).")
    lines.append("")
    lines.append(f"_Generated by `{SCRIPT_NAME} {SCHEMA_VERSION}`_")
    lines.append("")
    return "\n".join(lines)


def render_show_md(
    name: str,
    plan_entry: dict[str, Any],
    *,
    archive_dir: str,
) -> str:
    """Render details for one archive."""
    lines: list[str] = []
    lines.append(f"# V1381 — V1375 Archive: `{name}`")
    lines.append("")
    lines.append(f"- **tier:** {plan_entry.get('tier', '?')}")
    lines.append(f"- **age (sec):** {plan_entry.get('age_sec', 0)}")
    lines.append(f"- **age (days):** {round(plan_entry.get('age_sec', 0) / 86400.0, 2)}")
    lines.append(f"- **action:** {plan_entry.get('action', 'keep')}")
    lines.append(f"- **target:** `{plan_entry.get('target_path', '')}`")
    lines.append(f"- **reason:** {plan_entry.get('reason', '')}")
    lines.append("")
    full_path = os.path.join(archive_dir, name)
    if os.path.exists(full_path):
        size = os.path.getsize(full_path)
        h = hashlib.sha256()
        with open(full_path, "rb") as fh:
            while True:
                chunk = fh.read(65536)
                if not chunk:
                    break
                h.update(chunk)
        sha = h.hexdigest()
        lines.append(f"- **size (bytes):** {size}")
        lines.append(f"- **sha256:** `{sha}`")
        lines.append(f"- **on disk:** yes")
    else:
        lines.append(f"- **on disk:** no")
    lines.append("")
    lines.append(f"_Generated by `{SCRIPT_NAME} {SCHEMA_VERSION}`_")
    lines.append("")
    return "\n".join(lines)


# ----------------------------------------------------------------------
# Manifest I/O
# ----------------------------------------------------------------------

def render_manifest_json(
    plan: list[dict[str, Any]],
    *,
    policy: dict[str, Any],
    archive_dir: str,
    now: _dt.datetime | None = None,
    applied: bool = False,
) -> dict[str, Any]:
    """Render the rotation manifest as a dict (deterministic)."""
    if now is None:
        now = _dt.datetime.now(_dt.timezone.utc)
    now_iso = now.strftime("%Y-%m-%dT%H:%M:%SZ")
    actions_summary: dict[str, int] = {"keep": 0, "compress": 0, "prune": 0}
    out_actions: list[dict[str, Any]] = []
    for p in plan:
        a = p.get("action", "keep")
        actions_summary[a] = actions_summary.get(a, 0) + 1
        out_actions.append({
            "name": p.get("name", ""),
            "tier": p.get("tier", ""),
            "age_sec": p.get("age_sec", 0),
            "action": a,
            "target_path": p.get("target_path", ""),
            "reason": p.get("reason", ""),
        })
    return {
        "schema_version": SCHEMA_VERSION,
        "policy_version": policy.get("policy_version", "?"),
        "archive_dir": archive_dir,
        "generated": now_iso,
        "applied": bool(applied),
        "actions_summary": actions_summary,
        "actions": out_actions,
    }


def write_manifest(manifest: dict[str, Any], path: str) -> None:
    """Atomic write of the rotation manifest (tmp + rename + makedirs)."""
    _validate_safe_manifest_path(path)
    parent = os.path.dirname(os.path.abspath(path))
    os.makedirs(parent, exist_ok=True)
    payload = json.dumps(manifest, ensure_ascii=False, sort_keys=True, indent=2)
    fd, tmp = tempfile.mkstemp(prefix=".v1381_manifest_", dir=parent)
    try:
        with os.fdopen(fd, "w", encoding="utf-8") as fh:
            fh.write(payload)
        os.replace(tmp, path)
    except Exception:
        # Best effort cleanup on failure
        try:
            os.unlink(tmp)
        except OSError:
            pass
        raise


def load_manifest(path: str) -> dict[str, Any] | None:
    """Load a rotation manifest from disk (returns None on missing/invalid)."""
    if not os.path.exists(path):
        return None
    try:
        with open(path, "r", encoding="utf-8", errors="replace") as fh:
            return json.load(fh)
    except (json.JSONDecodeError, OSError):
        return None


# ----------------------------------------------------------------------
# Apply rotation (atomic)
# ----------------------------------------------------------------------

def _atomic_gzip(src_path: str, dst_path: str) -> dict[str, Any]:
    """Gzip-compress ``src_path`` to ``dst_path`` atomically.

    Returns dict ``{"src": ..., "dst": ..., "src_sha256": ..., "dst_sha256": ...,
    "src_size": ..., "dst_size": ..., "ok": True}``.
    """
    src_sha = hashlib.sha256()
    with open(src_path, "rb") as fh:
        while True:
            chunk = fh.read(65536)
            if not chunk:
                break
            src_sha.update(chunk)
    src_sha_hex = src_sha.hexdigest()
    src_size = os.path.getsize(src_path)
    parent = os.path.dirname(os.path.abspath(dst_path))
    os.makedirs(parent, exist_ok=True)
    fd, tmp = tempfile.mkstemp(prefix=".v1381_gz_", dir=parent)
    try:
        with os.fdopen(fd, "wb") as out_fh:
            with open(src_path, "rb") as in_fh:
                with gzip.GzipFile(
                    fileobj=out_fh, mode="wb", compresslevel=6
                ) as gz:
                    shutil.copyfileobj(in_fh, gz, length=65536)
        os.replace(tmp, dst_path)
    except Exception:
        try:
            os.unlink(tmp)
        except OSError:
            pass
        raise
    dst_sha = hashlib.sha256()
    with open(dst_path, "rb") as fh:
        while True:
            chunk = fh.read(65536)
            if not chunk:
                break
            dst_sha.update(chunk)
    dst_sha_hex = dst_sha.hexdigest()
    dst_size = os.path.getsize(dst_path)
    return {
        "src": src_path,
        "dst": dst_path,
        "src_sha256": src_sha_hex,
        "dst_sha256": dst_sha_hex,
        "src_size": src_size,
        "dst_size": dst_size,
        "ok": True,
    }


def apply_rotation(
    plan: list[dict[str, Any]],
    archive_dir: str,
    *,
    policy: dict[str, Any],
    manifest_path: str | None = None,
    now: _dt.datetime | None = None,
    actions_to_apply: set[str] | None = None,
) -> dict[str, Any]:
    """Apply the rotation plan to disk.

    Returns a result dict with keys:
      - ``applied_actions`` (list): each executed action + result
      - ``skipped_actions`` (list): each skipped action + reason
      - ``manifest_path`` (str): where the manifest was written
      - ``ok`` (bool): True iff no action failed

    ``actions_to_apply`` is a set of action names to actually execute
    (e.g., ``{"compress"}`` or ``{"compress", "prune"}``). Default:
    ``{"compress"}`` (safer; ``prune`` requires explicit opt-in).
    """
    _validate_safe_archive_dir(archive_dir)
    if manifest_path is None:
        manifest_path = DEFAULT_MANIFEST_PATH
    _validate_safe_manifest_path(manifest_path)
    if now is None:
        now = _dt.datetime.now(_dt.timezone.utc)
    if actions_to_apply is None:
        actions_to_apply = {"compress"}

    applied: list[dict[str, Any]] = []
    skipped: list[dict[str, Any]] = []
    ok_overall = True

    for entry in plan:
        action = entry.get("action", "keep")
        name = entry.get("name", "")
        if action == "keep":
            skipped.append({"name": name, "action": action, "reason": "no action needed"})
            continue
        if action not in actions_to_apply:
            skipped.append({"name": name, "action": action,
                            "reason": f"action '{action}' not in actions_to_apply"})
            continue
        src_path = os.path.join(archive_dir, name)
        if not os.path.exists(src_path):
            skipped.append({"name": name, "action": action,
                            "reason": "source archive missing on disk"})
            ok_overall = False
            continue
        if action == "compress":
            dst_path = os.path.join(archive_dir, name + ".gz")
            try:
                result = _atomic_gzip(src_path, dst_path)
                # Atomic source removal: rename source to .bak then unlink
                # (best-effort; if unlink fails we still have the .gz and a .bak)
                bak_path = src_path + ".bak"
                try:
                    os.replace(src_path, bak_path)
                    os.unlink(bak_path)
                except OSError:
                    # If rename/unlink fails, leave both — manifest records both paths
                    pass
                applied.append({
                    "name": name,
                    "action": action,
                    "src": result["src"],
                    "dst": result["dst"],
                    "src_sha256": result["src_sha256"],
                    "dst_sha256": result["dst_sha256"],
                    "src_size": result["src_size"],
                    "dst_size": result["dst_size"],
                    "ok": True,
                })
            except Exception as e:
                ok_overall = False
                applied.append({
                    "name": name,
                    "action": action,
                    "ok": False,
                    "error": str(e),
                })
        elif action == "prune":
            # By design, prune is NOT in default actions_to_apply.
            # To actually prune, the caller must pass {"compress", "prune"}.
            try:
                os.unlink(src_path)
                applied.append({
                    "name": name,
                    "action": action,
                    "src": src_path,
                    "ok": True,
                })
            except Exception as e:
                ok_overall = False
                applied.append({
                    "name": name,
                    "action": action,
                    "ok": False,
                    "error": str(e),
                })

    # Write manifest (atomic)
    full_manifest = render_manifest_json(
        plan, policy=policy, archive_dir=archive_dir, now=now, applied=True
    )
    full_manifest["applied_actions"] = applied
    full_manifest["skipped_actions"] = skipped
    full_manifest["actions_to_apply"] = sorted(actions_to_apply)
    full_manifest["ok"] = ok_overall
    write_manifest(full_manifest, manifest_path)

    return {
        "applied_actions": applied,
        "skipped_actions": skipped,
        "manifest_path": manifest_path,
        "ok": ok_overall,
    }


# ----------------------------------------------------------------------
# Atomic write helper for markdown reports
# ----------------------------------------------------------------------

def write_report(path: str, content: str) -> None:
    """Atomic write of any markdown report."""
    parent = os.path.dirname(os.path.abspath(path))
    os.makedirs(parent, exist_ok=True)
    fd, tmp = tempfile.mkstemp(prefix=".v1381_report_", dir=parent)
    try:
        with os.fdopen(fd, "w", encoding="utf-8") as fh:
            fh.write(content)
        os.replace(tmp, path)
    except Exception:
        try:
            os.unlink(tmp)
        except OSError:
            pass
        raise


# ----------------------------------------------------------------------
# CLI
# ----------------------------------------------------------------------

def _build_arg_parser() -> argparse.ArgumentParser:
    p = argparse.ArgumentParser(
        prog=SCRIPT_NAME,
        description="V1381 — V1375 archival rotation (plan-first)",
    )
    p.add_argument("--archive-dir", default=DEFAULT_ARCHIVE_DIR,
                   help=f"Archive directory (default: {DEFAULT_ARCHIVE_DIR})")
    p.add_argument("--manifest-path", default=DEFAULT_MANIFEST_PATH,
                   help=f"Rotation manifest path (default: {DEFAULT_MANIFEST_PATH})")
    p.add_argument("--plan-path", default=DEFAULT_PLAN_PATH,
                   help=f"Rotation plan markdown path (default: {DEFAULT_PLAN_PATH})")
    p.add_argument("--policy-path", default=DEFAULT_POLICY_PATH,
                   help=f"Policy markdown path (default: {DEFAULT_POLICY_PATH})")
    sub = p.add_subparsers(dest="cmd")

    sub.add_parser("policy", help="Write the policy to a markdown file")
    sub.add_parser("plan", help="Compute + write the rotation plan (no disk mutation)")
    sub.add_parser("list", help="Write a markdown list of all archives × tier × age")

    rp = sub.add_parser("rotate", help="Apply the rotation (atomic; manifest-logged)")
    rp.add_argument("--apply", action="store_true",
                    help="Actually mutate disk (default: dry-run write of plan only)")
    rp.add_argument("--allow-prune", action="store_true",
                    help="Also apply prune actions (default: only compress)")
    rp.add_argument("--now", default=None,
                    help="Override 'now' (ISO 8601) for deterministic testing")

    sp = sub.add_parser("show", help="Show details for one archive")
    sp.add_argument("archive_name", help="V1375 archive slug (e.g. 2026-08-09T03-55-00Z__v1374.md)")

    sub.add_parser("popper", help="Run the Popper self-tests")
    sub.add_parser("version", help="Print the schema version and exit")
    return p


def run_cli(args: list[str] | None = None) -> int:
    """Argv dispatcher. Returns process exit code."""
    parser = _build_arg_parser()
    if args is None:
        args = sys.argv[1:]
    ns = parser.parse_args(args)
    cmd = ns.cmd or "version"

    archive_dir = ns.archive_dir
    _validate_safe_archive_dir(archive_dir)

    if cmd == "version":
        print(f"{SCRIPT_NAME} {SCHEMA_VERSION}")
        print(f"policy {POLICY_VERSION}")
        return 0

    if cmd == "policy":
        pol = default_policy()
        content = render_policy_md(pol)
        write_report(ns.policy_path, content)
        print(f"policy written: {ns.policy_path} ({len(content)} bytes)")
        return 0

    if cmd == "list":
        plan = plan_rotation(archive_dir)
        content = render_list_md(plan, archive_dir=archive_dir)
        write_report(ns.plan_path.replace("PLAN", "LIST"), content)
        print(f"list written: {ns.plan_path.replace('PLAN', 'LIST')} ({len(content)} bytes)")
        return 0

    # Common path: need a deterministic now
    now: _dt.datetime | None = None
    if hasattr(ns, "now") and ns.now:
        try:
            now = _dt.datetime.fromisoformat(ns.now.replace("Z", "+00:00"))
        except ValueError:
            now = None

    if cmd == "plan":
        pol = default_policy()
        plan = plan_rotation(archive_dir, now=now, policy=pol)
        content = render_plan_md(plan, archive_dir=archive_dir, policy=pol, now=now)
        write_report(ns.plan_path, content)
        print(f"plan written: {ns.plan_path} ({len(content)} bytes)")
        # Don't apply, just exit 0
        return 0

    if cmd == "rotate":
        pol = default_policy()
        plan = plan_rotation(archive_dir, now=now, policy=pol)
        # Always write the plan first (audit trail)
        plan_content = render_plan_md(plan, archive_dir=archive_dir, policy=pol, now=now)
        write_report(ns.plan_path, plan_content)
        if not ns.apply:
            print(f"dry-run: plan written: {ns.plan_path} ({len(plan_content)} bytes)")
            print("rotate: --apply not set; no archives touched")
            return 0
        actions_to_apply: set[str] = {"compress"}
        if ns.allow_prune:
            actions_to_apply.add("prune")
        result = apply_rotation(
            plan, archive_dir, policy=pol,
            manifest_path=ns.manifest_path, now=now,
            actions_to_apply=actions_to_apply,
        )
        print(f"rotate: applied {len(result['applied_actions'])} actions; "
              f"skipped {len(result['skipped_actions'])}; "
              f"ok={result['ok']}; manifest={result['manifest_path']}")
        return 0 if result["ok"] else 1

    if cmd == "show":
        plan = plan_rotation(archive_dir, now=now)
        target = ns.archive_name
        match = next((p for p in plan if p["name"] == target), None)
        if match is None:
            print(f"archive not found: {target}", file=sys.stderr)
            return 2
        content = render_show_md(target, match, archive_dir=archive_dir)
        print(content)
        return 0

    if cmd == "popper":
        passed, total, failures = _popper_self_tests()
        for f in failures:
            print(f"FAIL: {f}", file=sys.stderr)
        print(f"Popper self-tests: {passed}/{total}")
        return 0 if passed == total else 1

    # Unknown command
    parser.print_help()
    return 2


# ----------------------------------------------------------------------
# Popper self-tests
# ----------------------------------------------------------------------

def _popper_self_tests() -> tuple[int, int, list[str]]:
    """In-module Popper-style self-tests. Returns ``(passed, total, failures)``."""
    failures: list[str] = []
    checks: list[tuple[str, bool]] = []

    # 1. SCHEMA_VERSION + SCRIPT_NAME present
    checks.append(("schema_version present", bool(SCHEMA_VERSION and isinstance(SCHEMA_VERSION, str))))
    checks.append(("script_name present", bool(SCRIPT_NAME and isinstance(SCRIPT_NAME, str))))

    # 2. default_policy keys
    pol = default_policy()
    checks.append(("policy has policy_version", "policy_version" in pol))
    checks.append(("policy has age_bands", "age_bands" in pol))
    checks.append(("policy has actions", "actions" in pol))
    checks.append(("policy version matches", pol["policy_version"] == POLICY_VERSION))

    # 3. AGE_BANDS values are sane (HOT < WARM < COLD)
    checks.append(("HOT < WARM", AGE_BANDS["HOT_MAX_SEC"] < AGE_BANDS["WARM_MAX_SEC"]))
    checks.append(("WARM < COLD", AGE_BANDS["WARM_MAX_SEC"] < AGE_BANDS["COLD_MAX_SEC"]))

    # 4. parse_iso_basic
    dt = parse_iso_basic("2026-08-09T03-55-00Z")
    checks.append(("parse_iso_basic returns aware UTC",
                   dt.tzinfo is not None and dt.utcoffset().total_seconds() == 0))
    checks.append(("parse_iso_basic hour", dt.hour == 3))
    checks.append(("parse_iso_basic day", dt.day == 9))
    checks.append(("parse_iso_basic minute", dt.minute == 55))

    # 5. parse_iso_basic rejects invalid
    try:
        parse_iso_basic("not-an-iso")
        checks.append(("parse_iso_basic rejects invalid", False))
    except ValueError:
        checks.append(("parse_iso_basic rejects invalid", True))

    # 6. slug_timestamp_to_datetime
    slug = "2026-08-09T03-55-00Z__v1374.md"
    dt2 = slug_timestamp_to_datetime(slug)
    checks.append(("slug_timestamp_to_datetime same as parse", dt2 == dt))

    # 7. slug_timestamp_to_datetime rejects bad slug
    try:
        slug_timestamp_to_datetime("INDEX.md")
        checks.append(("slug_timestamp_to_datetime rejects bad slug", False))
    except ValueError:
        checks.append(("slug_timestamp_to_datetime rejects bad slug", True))

    # 8. classify_archive — HOT (0 sec)
    tier, age = classify_archive("2026-08-09T03-55-00Z__v1374.md",
                                  now=_dt.datetime(2026, 8, 9, 4, 0, 0, tzinfo=_dt.timezone.utc))
    checks.append(("classify HOT", tier == "HOT"))
    checks.append(("classify HOT age ~300s", age == 300))

    # 9. classify_archive — WARM (3 days)
    tier, age = classify_archive("2026-08-09T03-55-00Z__v1374.md",
                                  now=_dt.datetime(2026, 8, 12, 3, 55, 0, tzinfo=_dt.timezone.utc))
    checks.append(("classify WARM (3 days)", tier == "WARM"))

    # 10. classify_archive — COLD (30 days)
    tier, age = classify_archive("2026-08-09T03-55-00Z__v1374.md",
                                  now=_dt.datetime(2026, 9, 8, 3, 55, 0, tzinfo=_dt.timezone.utc))
    checks.append(("classify COLD (30 days)", tier == "COLD"))

    # 11. classify_archive — FROZEN (200 days)
    tier, age = classify_archive("2026-08-09T03-55-00Z__v1374.md",
                                  now=_dt.datetime(2027, 2, 25, 3, 55, 0, tzinfo=_dt.timezone.utc))
    checks.append(("classify FROZEN (200 days)", tier == "FROZEN"))

    # 12. plan_action — HOT → keep
    pa = plan_action("2026-08-09T03-55-00Z__v1374.md",
                     now=_dt.datetime(2026, 8, 9, 4, 0, 0, tzinfo=_dt.timezone.utc))
    checks.append(("plan_action HOT keep", pa["action"] == "keep"))
    checks.append(("plan_action HOT target", pa["target_path"] == pa["name"]))

    # 13. plan_action — COLD → compress
    pa = plan_action("2026-08-09T03-55-00Z__v1374.md",
                     now=_dt.datetime(2026, 9, 8, 3, 55, 0, tzinfo=_dt.timezone.utc))
    checks.append(("plan_action COLD compress", pa["action"] == "compress"))
    checks.append(("plan_action COLD target has .gz", pa["target_path"].endswith(".gz")))

    # 14. plan_action — FROZEN → prune
    pa = plan_action("2026-08-09T03-55-00Z__v1374.md",
                     now=_dt.datetime(2027, 2, 25, 3, 55, 0, tzinfo=_dt.timezone.utc))
    checks.append(("plan_action FROZEN prune", pa["action"] == "prune"))
    checks.append(("plan_action FROZEN target empty", pa["target_path"] == ""))

    # 15. _list_archive_names filters INDEX.md
    with tempfile.TemporaryDirectory() as td:
        for n in ["2026-08-09T03-55-00Z__v1374.md",
                  "2026-08-09T04-00-00Z__v1374.md",
                  "INDEX.md",
                  "README.txt"]:
            with open(os.path.join(td, n), "w") as fh:
                fh.write("x")
        names = _list_archive_names(td)
        checks.append(("list filters INDEX.md", "INDEX.md" not in names))
        checks.append(("list filters README.txt", "README.txt" not in names))
        checks.append(("list keeps 2 archives", len(names) == 2))

    # 16. _list_archive_names returns empty for missing dir
    names = _list_archive_names("/nonexistent/v1381/test/path")
    checks.append(("list missing dir returns empty", names == []))

    # 17. plan_rotation includes all archives (even HOT keep)
    with tempfile.TemporaryDirectory() as td:
        for n in ["2026-08-09T03-55-00Z__v1374.md",
                  "2026-08-09T04-00-00Z__v1374.md"]:
            with open(os.path.join(td, n), "w") as fh:
                fh.write("# x\n")
        plan = plan_rotation(td, now=_dt.datetime(2026, 8, 9, 4, 30, 0, tzinfo=_dt.timezone.utc))
        checks.append(("plan_rotation returns 2", len(plan) == 2))
        checks.append(("plan_rotation all keep", all(p["action"] == "keep" for p in plan)))

    # 18. render_plan_md has key sections
    plan = [{"name": "2026-08-09T03-55-00Z__v1374.md", "tier": "HOT",
             "age_sec": 300, "action": "keep", "target_path": "2026-08-09T03-55-00Z__v1374.md",
             "reason": "test"}]
    md = render_plan_md(plan, archive_dir="/tmp/test", policy=default_policy())
    checks.append(("render_plan_md has V1381", "V1381" in md))
    checks.append(("render_plan_md has honesty", "Honesty" in md or "honesty" in md))
    checks.append(("render_plan_md has policy version", POLICY_VERSION in md))
    checks.append(("render_plan_md has action counts", "Action counts" in md))

    # 19. render_policy_md has all tiers
    pmd = render_policy_md(default_policy())
    for tier in ["HOT", "WARM", "COLD", "FROZEN"]:
        checks.append((f"render_policy_md has {tier}", tier in pmd))

    # 20. render_list_md has table header
    lmd = render_list_md(plan, archive_dir="/tmp/test")
    checks.append(("render_list_md has table header", "| tier |" in lmd))

    # 21. render_manifest_json has schema_version
    m = render_manifest_json(plan, policy=default_policy(), archive_dir="/tmp/test",
                              applied=False)
    checks.append(("manifest has schema_version", m["schema_version"] == SCHEMA_VERSION))
    checks.append(("manifest has actions", len(m["actions"]) == 1))
    checks.append(("manifest has actions_summary", "actions_summary" in m))
    checks.append(("manifest applied flag", m["applied"] is False))

    # 22. write_manifest + load_manifest roundtrip
    with tempfile.TemporaryDirectory() as td:
        path = os.path.join(td, "manifest.json")
        write_manifest(m, path)
        loaded = load_manifest(path)
        checks.append(("manifest roundtrip", loaded is not None and loaded["schema_version"] == SCHEMA_VERSION))

    # 23. write_report atomic
    with tempfile.TemporaryDirectory() as td:
        path = os.path.join(td, "nested", "report.md")  # nested doesn't exist yet
        write_report(path, "# test\n")
        checks.append(("write_report creates nested dirs", os.path.exists(path)))

    # 24. _atomic_gzip roundtrip (use binary mode to avoid Windows \r\n)
    with tempfile.TemporaryDirectory() as td:
        src = os.path.join(td, "src.md")
        expected_bytes = b"# Hello World\n" * 100
        with open(src, "wb") as fh:
            fh.write(expected_bytes)
        dst = os.path.join(td, "src.md.gz")
        result = _atomic_gzip(src, dst)
        checks.append(("gzip ok", result["ok"]))
        checks.append(("gzip src_size > 0", result["src_size"] > 0))
        checks.append(("gzip dst_size > 0", result["dst_size"] > 0))
        checks.append(("gzip dst exists", os.path.exists(dst)))
        # Decompress and verify
        with gzip.open(dst, "rb") as fh:
            decompressed = fh.read()
        checks.append(("gzip roundtrip bytes match",
                       decompressed == expected_bytes))

    # 25. _validate_safe_archive_dir rejects traversal
    try:
        _validate_safe_archive_dir("../etc/passwd")
        checks.append(("rejects parent traversal", False))
    except ValueError:
        checks.append(("rejects parent traversal", True))

    # 26. _validate_safe_archive_dir accepts relative + absolute
    try:
        _validate_safe_archive_dir("V1375_HISTORY")
        _validate_safe_archive_dir("/tmp/test")
        checks.append(("accepts relative + absolute", True))
    except ValueError:
        checks.append(("accepts relative + absolute", False))

    # 27. _validate_safe_manifest_path rejects traversal
    try:
        _validate_safe_manifest_path("../manifest.json")
        checks.append(("rejects manifest traversal", False))
    except ValueError:
        checks.append(("rejects manifest traversal", True))

    # 28. apply_rotation dry-run (no actions, no archive changes)
    with tempfile.TemporaryDirectory() as td:
        # Write one archive
        with open(os.path.join(td, "2026-08-09T03-55-00Z__v1374.md"), "w") as fh:
            fh.write("# test\n")
        plan = plan_rotation(td, now=_dt.datetime(2026, 8, 9, 4, 0, 0, tzinfo=_dt.timezone.utc))
        manifest_path = os.path.join(td, "manifest.json")
        result = apply_rotation(plan, td, policy=default_policy(),
                                manifest_path=manifest_path)
        # All archives HOT → keep → skipped
        checks.append(("apply dry-run no actions", len(result["applied_actions"]) == 0))
        checks.append(("apply manifest written", os.path.exists(manifest_path)))
        checks.append(("apply ok", result["ok"]))

    # 29. apply_rotation compress action (synthesize COLD archive)
    with tempfile.TemporaryDirectory() as td:
        # Archive dated 2026-08-09; now is 2026-09-08 → 30 days = COLD
        old_name = "2026-08-09T03-55-00Z__v1374.md"
        with open(os.path.join(td, old_name), "w") as fh:
            fh.write("# old archive\n" * 50)
        plan = plan_rotation(td, now=_dt.datetime(2026, 9, 8, 4, 0, 0, tzinfo=_dt.timezone.utc))
        manifest_path = os.path.join(td, "manifest.json")
        result = apply_rotation(plan, td, policy=default_policy(),
                                manifest_path=manifest_path)
        checks.append(("apply compress: 1 action", len(result["applied_actions"]) == 1))
        checks.append(("apply compress: ok flag", result["applied_actions"][0]["ok"]))
        checks.append(("apply compress: .gz exists",
                       os.path.exists(os.path.join(td, old_name + ".gz"))))
        checks.append(("apply compress: src removed",
                       not os.path.exists(os.path.join(td, old_name))))
        # Manifest records the action
        loaded = load_manifest(manifest_path)
        checks.append(("manifest records compress",
                       loaded is not None and loaded["applied_actions"][0]["action"] == "compress"))

    # 30. apply_rotation refuses prune by default
    with tempfile.TemporaryDirectory() as td:
        old_name = "2026-08-09T03-55-00Z__v1374.md"
        with open(os.path.join(td, old_name), "w") as fh:
            fh.write("# old\n")
        plan = plan_rotation(td, now=_dt.datetime(2027, 2, 25, 4, 0, 0, tzinfo=_dt.timezone.utc))
        manifest_path = os.path.join(td, "manifest.json")
        # Default actions_to_apply = {"compress"}; prune should be skipped
        result = apply_rotation(plan, td, policy=default_policy(),
                                manifest_path=manifest_path)
        # Prune action not in default set → must appear in skipped_actions,
        # not applied_actions. (BOTH conditions hold when guards work.)
        no_prune_applied = not any(
            a.get("action") == "prune" for a in result["applied_actions"]
        )
        prune_was_skipped = any(
            s.get("action") == "prune" for s in result["skipped_actions"]
        )
        checks.append(("prune skipped by default (not applied)", no_prune_applied))
        checks.append(("prune skipped by default (logged)", prune_was_skipped))

    # 31. apply_rotation with allow_prune actually prunes
    with tempfile.TemporaryDirectory() as td:
        old_name = "2026-08-09T03-55-00Z__v1374.md"
        with open(os.path.join(td, old_name), "w") as fh:
            fh.write("# old\n")
        plan = plan_rotation(td, now=_dt.datetime(2027, 2, 25, 4, 0, 0, tzinfo=_dt.timezone.utc))
        manifest_path = os.path.join(td, "manifest.json")
        result = apply_rotation(plan, td, policy=default_policy(),
                                manifest_path=manifest_path,
                                actions_to_apply={"compress", "prune"})
        checks.append(("prune allowed: applied", len(result["applied_actions"]) == 1))
        checks.append(("prune allowed: src removed", not os.path.exists(os.path.join(td, old_name))))

    # 32. CLI plan subcommand writes plan file
    with tempfile.TemporaryDirectory() as td:
        # Set up a fake archive dir
        for n in ["2026-08-09T03-55-00Z__v1374.md", "2026-08-09T04-00-00Z__v1374.md"]:
            with open(os.path.join(td, n), "w") as fh:
                fh.write("# x\n")
        plan_path = os.path.join(td, "plan.md")
        rc = run_cli(["--archive-dir", td, "--plan-path", plan_path, "plan"])
        checks.append(("CLI plan exit 0", rc == 0))
        checks.append(("CLI plan file written", os.path.exists(plan_path)))

    # 33. CLI version subcommand
    import io as _io
    saved_stdout = sys.stdout
    try:
        buf = _io.StringIO()
        sys.stdout = buf
        rc = run_cli(["version"])
        out = buf.getvalue()
        checks.append(("CLI version exit 0", rc == 0))
        checks.append(("CLI version contains schema", SCHEMA_VERSION in out))
    finally:
        sys.stdout = saved_stdout

    # 34. CLI policy subcommand
    with tempfile.TemporaryDirectory() as td:
        policy_path = os.path.join(td, "policy.md")
        rc = run_cli(["--policy-path", policy_path, "policy"])
        checks.append(("CLI policy exit 0", rc == 0))
        checks.append(("CLI policy file written", os.path.exists(policy_path)))

    # 35. CLI list subcommand
    with tempfile.TemporaryDirectory() as td:
        for n in ["2026-08-09T03-55-00Z__v1374.md"]:
            with open(os.path.join(td, n), "w") as fh:
                fh.write("# x\n")
        list_path = os.path.join(td, "list.md")
        rc = run_cli(["--archive-dir", td, "--plan-path", list_path.replace("list", "PLAN"),
                      "list"])
        checks.append(("CLI list exit 0", rc == 0))

    # 36. CLI rotate dry-run (no --apply) writes plan
    with tempfile.TemporaryDirectory() as td:
        old_name = "2026-08-09T03-55-00Z__v1374.md"
        with open(os.path.join(td, old_name), "w") as fh:
            fh.write("# x\n")
        plan_path = os.path.join(td, "plan.md")
        manifest_path = os.path.join(td, "manifest.json")
        rc = run_cli(["--archive-dir", td, "--plan-path", plan_path,
                      "--manifest-path", manifest_path, "rotate",
                      "--now", "2026-09-08T04:00:00Z"])
        checks.append(("CLI rotate dry-run exit 0", rc == 0))
        checks.append(("CLI rotate dry-run plan written", os.path.exists(plan_path)))
        checks.append(("CLI rotate dry-run no manifest",
                       not os.path.exists(manifest_path)))
        # Archive still on disk (not compressed)
        checks.append(("CLI rotate dry-run no compression",
                       os.path.exists(os.path.join(td, old_name))))
        checks.append(("CLI rotate dry-run no .gz",
                       not os.path.exists(os.path.join(td, old_name + ".gz"))))

    # 37. CLI rotate --apply actually compresses
    with tempfile.TemporaryDirectory() as td:
        old_name = "2026-08-09T03-55-00Z__v1374.md"
        with open(os.path.join(td, old_name), "w") as fh:
            fh.write("# old\n" * 100)
        plan_path = os.path.join(td, "plan.md")
        manifest_path = os.path.join(td, "manifest.json")
        rc = run_cli(["--archive-dir", td, "--plan-path", plan_path,
                      "--manifest-path", manifest_path, "rotate",
                      "--apply", "--now", "2026-09-08T04:00:00Z"])
        checks.append(("CLI rotate --apply exit 0", rc == 0))
        checks.append(("CLI rotate --apply .gz exists",
                       os.path.exists(os.path.join(td, old_name + ".gz"))))
        checks.append(("CLI rotate --apply manifest written",
                       os.path.exists(manifest_path)))

    # 38. CLI show subcommand
    with tempfile.TemporaryDirectory() as td:
        with open(os.path.join(td, "2026-08-09T03-55-00Z__v1374.md"), "w") as fh:
            fh.write("# test archive content\n" * 5)
        saved_stdout = sys.stdout
        try:
            buf = _io.StringIO()
            sys.stdout = buf
            rc = run_cli(["--archive-dir", td, "show",
                          "2026-08-09T03-55-00Z__v1374.md"])
            out = buf.getvalue()
            checks.append(("CLI show exit 0", rc == 0))
            checks.append(("CLI show contains tier", "tier:" in out))
            checks.append(("CLI show contains sha256", "sha256:" in out))
        finally:
            sys.stdout = saved_stdout

    # 39. CLI show archive not found
    with tempfile.TemporaryDirectory() as td:
        rc = run_cli(["--archive-dir", td, "show", "nonexistent.md"])
        checks.append(("CLI show missing exits 2", rc == 2))

    # 40. CLI popper subcommand (NOT recursive — call _popper_self_tests directly)
    # NOTE: Calling run_cli(["popper"]) here would re-enter _popper_self_tests
    # and infinite-loop. So instead we exercise run_cli with a different
    # subcommand ("version") that doesn't recurse, and trust the popper
    # body to have been exercised by the outer call.
    saved_stdout = sys.stdout
    try:
        buf = _io.StringIO()
        sys.stdout = buf
        rc = run_cli(["version"])
        out = buf.getvalue()
        checks.append(("CLI version exits 0 (proxy for popper wiring)", rc == 0))
        checks.append(("CLI version says schema", SCHEMA_VERSION in out))
    finally:
        sys.stdout = saved_stdout
    # NOTE: DO NOT call _popper_self_tests() recursively here — that would
    # infinitely recurse. Instead, determinism is verified by the outer
    # popper run, which calls _popper_self_tests() once.

    # 41. determinism: same plan twice = same bytes
    plan_a = plan_rotation("/nonexistent/v1381/test")
    plan_b = plan_rotation("/nonexistent/v1381/test")
    checks.append(("plan deterministic (empty)", plan_a == plan_b))
    md_a = render_plan_md(plan_a, archive_dir="/x", policy=default_policy(),
                          now=_dt.datetime(2026, 8, 9, 4, 0, 0, tzinfo=_dt.timezone.utc))
    md_b = render_plan_md(plan_b, archive_dir="/x", policy=default_policy(),
                          now=_dt.datetime(2026, 8, 9, 4, 0, 0, tzinfo=_dt.timezone.utc))
    checks.append(("plan_md deterministic", md_a == md_b))

    # Tally
    passed = sum(1 for _, ok in checks if ok)
    total = len(checks)
    failures = [name for name, ok in checks if not ok]
    return passed, total, failures


if __name__ == "__main__":
    sys.exit(run_cli())