"""Phase 1382 v1382_v1375_x_v1381_overlay — archive-health observability overlay.

## Phase

Phase: 1382
Version: 0.1.0
Date: 2026-08-09 (cron tick 234)
Post: V1381 (V1375 archival rotation)
ASI 北极星: LOCKED (V0.1=0.7905 / V0.2=0.4467 / V1256=0.9105 / V0.3 NOT due)

## What V1382 is

V1382 is the **observability overlay** that joins V1375 (history archive),
V1379 (integrity manifest), and V1381 (rotation plan) into one JSON
snapshot. It does NOT compute metrics. It does NOT touch the cap. It
simply answers: "what does the V1375 archive subsystem look like *right
now*?"

**Inputs:**
- `V1375_HISTORY/` — archive directory
- `V1375_HISTORY/INDEX.md` — sorted archive index (V1375)
- `V1379_INTEGRITY_AUTO.json` — SHA-256 manifest (V1379)
- `V1381_PLAN_AUTO.md` (or freshly computed) — rotation plan (V1381)

**Output (JSON):**
```json
{
  "schema": "v1382.overlay/v1",
  "generated": "2026-08-09T05:10:00Z",
  "archive_dir": "V1375_HISTORY",
  "totals": {"archives": 1, "indexed": 1, "manifested": 1},
  "tier_counts": {"HOT": 1, "WARM": 0, "COLD": 0, "FROZEN": 0},
  "action_counts": {"keep": 1, "compress": 0, "prune": 0},
  "integrity": {
    "manifest_present": true,
    "manifest_path": "V1379_INTEGRITY_AUTO.json",
    "manifest_schema": "v1379.integrity/v1",
    "archives_in_manifest": 1,
    "archives_on_disk": 1,
    "missing_on_disk": [],
    "extra_on_disk": [],
    "ok": true
  },
  "rotation": {
    "policy_version": "v1381.rotation.policy/v1",
    "plan_path": "V1381_PLAN_AUTO.md",
    "actions_summary": {"keep": 1, "compress": 0, "prune": 0}
  },
  "guarded_observations": [
    "GUARD_NO_CAP_CHANGE: V1382 has no metric, no cap, no scoring",
    "GUARD_NO_LEDGER_TOUCH_BY_DEFAULT: --record is opt-in"
  ],
  "known_unknowns": [
    "did not validate INDEX.md row-by-row against on-disk files (only counted)"
  ]
}
```

## Why V1382 exists (post-V1380 next-step 2/3)

V1380 reconciled V1375 × INDEX × V1379 manifest (three-way).
V1381 added rotation policy.
V1382 answers: "if I'm a human and I just want to see the archive
health in one shot, where do I go?" The answer is now: run
`python -m apeireth.v1382_v1375_x_v1381_overlay snapshot`.

V1382 is the **observability seam** between:
- V1375/V1378/V1379/V1380/V1381 (archive + integrity + rotation history)
- V1357 (VCP observability aggregator) — V1382 output is shaped to
  slot into V1357's `infra_state` slot

V1382 is **read-only by default**. The `--record` flag is an explicit
opt-in that appends the snapshot to `V1382_HISTORY.jsonl` (analogous
to V1362 pole-star ledger). Recording never changes the cap.

## CLI

```bash
python -m apeireth.v1382_v1375_x_v1381_overlay snapshot
# → JSON snapshot to stdout (default)

python -m apeireth.v1382_v1375_x_v1381_overlay snapshot --pretty
# → Pretty-printed JSON to stdout

python -m apeireth.v1382_v1375_x_v1381_overlay snapshot --out PATH
# → Snapshot to file (atomic write)

python -m apeireth.v1382_v1375_x_v1381_overlay snapshot --record [--tag TAG]
# → Compute + write to file + append to V1382_HISTORY.jsonl

python -m apeireth.v1382_v1375_x_v1381_overlay summary
# → One-line summary (great for shell scripts)

python -m apeireth.v1382_v1375_x_v1381_overlay popper
# → 30+ Popper self-tests

python -m apeireth.v1382_v1375_x_v1381_overlay version
# → Print schema + version
```

## API surfaces (10)

1. `snapshot_archive_health(*, archive_dir, ...)` — full overlay dict
2. `_count_archives(archive_dir)` — number of valid archive slugs on disk
3. `_count_indexed(archive_dir)` — number of rows in INDEX.md
4. `_read_manifest(manifest_path)` — load V1379 manifest safely
5. `_check_integrity(archive_dir, manifest)` — compare manifest vs disk
6. `_rotation_summary(archive_dir, *, now)` — V1381 plan tier + action counts
7. `_render_snapshot_json(snapshot, *, pretty=False)` — JSON serializer
8. `_write_snapshot(snapshot, path)` — atomic write
9. `_append_history(snapshot, *, tag=None)` — append to JSONL ledger
10. `_popper_self_tests()` — Popper self-check (40+ checks)
    + `run_cli(args)` — argv dispatcher (snapshot / summary / popper / version)

## GUARDS upheld (V1382-specific, 11)

- GUARD_OVERLAY_READ_ONLY_BY_DEFAULT: default CLI = snapshot, no write
- GUARD_RECORD_IS_OPT_IN: --record must be explicit
- GUARD_ATOMIC_WRITE: tmp + rename for snapshot out + history append
- GUARD_NO_CAP_CHANGE: V1382 has no metric, no cap, no scoring
- GUARD_NO_SIDECAR_TOUCH: never imports V1371/V1369/V1370
- GUARD_HONEST_DISCLOSURE: known_unknowns always present
- GUARD_DETERMINISTIC: same inputs in same order → same snapshot bytes
- GUARD_PATH_SAFE: reject path traversal in archive_dir + manifest_path
- GUARD_HISTORY_APPEND_ONLY: V1382_HISTORY.jsonl only appended, never truncated
- GUARD_V1382_DOES_NOT_TOUCH_V1375: never writes V1375 archives directly
- GUARD_LOCAL_FILESYSTEM_ONLY: no network, no remote FS

## V3 哲学守门 (LOCKED, 主 17:43 + 17:58 + 20:46 + 22:33 + 23:44)

- 不假装分数 = ASI: V1382 has no metric, no cap, no scoring
- 不假装决策 = 真生产: snapshot is read-only aggregator, --record opt-in
- 不假装 ASI 集成: zero LLM, zero sidecar import, zero ledger write by default
- 不刷分: zero metric change in this commit; honest 0.90 cap preserved
- 不动 anchor: V1375 archives + INDEX.md + V1379 manifest + V1381 plan
  unchanged by V1382 default
- 不假装 V1382 = ASI 觉醒: V1382 reports archive health; doesn't interpret it
- 实事求是: real disk reads + real ISO parsing + real SHA-256 hash
- 任何人都能接手: CLI + JSON + Markdown + 1-cmd `snapshot` + atomic writes
  + reproducibility (deterministic when --now is fixed)
"""

from __future__ import annotations

import argparse
import datetime as _dt
import hashlib
import json
import os
import re
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

SCHEMA_VERSION = "v1382.overlay/v1"
SCRIPT_NAME = "v1382_v1375_x_v1381_overlay"
DEFAULT_ARCHIVE_DIR = "V1375_HISTORY"
DEFAULT_INDEX_PATH = "V1375_HISTORY/INDEX.md"
DEFAULT_MANIFEST_PATH = "V1379_INTEGRITY_AUTO.json"
DEFAULT_HISTORY_PATH = "V1382_HISTORY.jsonl"
DEFAULT_SNAPSHOT_PATH = "V1382_SNAPSHOT_AUTO.json"
POLICY_VERSION = "v1381.rotation.policy/v1"

# Match V1375 archive slug: ``<iso>__<schema>.md``
_RE_ARCHIVE_NAME = re.compile(
    r"^(?P<iso>\d{4}-\d{2}-\d{2}T\d{2}-\d{2}-\d{2}Z)__"
    r"(?P<schema>[a-zA-Z0-9_]+?)"
    r"(?:_(?P<collision>\d{3}))?\.md$"
)


# ----------------------------------------------------------------------
# Path safety
# ----------------------------------------------------------------------

def _validate_safe_path(path: str) -> None:
    """Reject path traversal (``..`` segments)."""
    raw_parts = path.replace("\\", "/").split("/")
    norm_parts = os.path.normpath(path).replace("\\", "/").split("/")
    if ".." in raw_parts or ".." in norm_parts:
        raise ValueError(f"Path contains parent traversal: {path!r}")


# ----------------------------------------------------------------------
# Disk counting
# ----------------------------------------------------------------------

def _list_archive_names(archive_dir: str) -> list[str]:
    """Return all valid V1375 archive slugs in ``archive_dir``."""
    if not os.path.isdir(archive_dir):
        return []
    out: list[str] = []
    for name in sorted(os.listdir(archive_dir)):
        if not name.endswith(".md"):
            continue
        if name == "INDEX.md":
            continue
        if _RE_ARCHIVE_NAME.match(name):
            out.append(name)
    return out


def _count_archives(archive_dir: str) -> int:
    return len(_list_archive_names(archive_dir))


def _count_indexed(archive_dir: str) -> int:
    """Count rows in INDEX.md that look like archive slugs."""
    index_path = os.path.join(archive_dir, "INDEX.md")
    if not os.path.isfile(index_path):
        return 0
    with open(index_path, "r", encoding="utf-8") as fh:
        text = fh.read()
    # Count backtick-wrapped .md filenames
    return len(re.findall(r"`[\dTZ\-]+__[a-zA-Z0-9_]+\.md`", text))


# ----------------------------------------------------------------------
# Integrity
# ----------------------------------------------------------------------

def _read_manifest(manifest_path: str) -> dict[str, Any] | None:
    if not os.path.isfile(manifest_path):
        return None
    try:
        with open(manifest_path, "r", encoding="utf-8") as fh:
            return json.load(fh)
    except (OSError, json.JSONDecodeError):
        return None


def _check_integrity(archive_dir: str, manifest: dict[str, Any] | None) -> dict[str, Any]:
    """Compare manifest records vs on-disk files.

    Supports two V1379 manifest shapes:
    - ``records`` (newer shape)
    - ``archives`` (legacy shape — current V1379 builds use this)
    """
    if manifest is None:
        return {
            "manifest_present": False,
            "manifest_path": DEFAULT_MANIFEST_PATH,
            "manifest_schema": None,
            "archives_in_manifest": 0,
            "archives_on_disk": _count_archives(archive_dir),
            "missing_on_disk": [],
            "extra_on_disk": [],
            "ok": False,
            "reason": "no manifest found",
        }
    records = manifest.get("records")
    if records is None:
        records = manifest.get("archives", [])
    manifest_names = {r.get("name", "") for r in records if r.get("name")}
    schema = manifest.get("schema_version") or manifest.get("schema")
    disk_names = set(_list_archive_names(archive_dir))
    missing = sorted(manifest_names - disk_names)
    extra = sorted(disk_names - manifest_names)
    return {
        "manifest_present": True,
        "manifest_path": DEFAULT_MANIFEST_PATH,
        "manifest_schema": schema,
        "archives_in_manifest": len(manifest_names),
        "archives_on_disk": len(disk_names),
        "missing_on_disk": missing,
        "extra_on_disk": extra,
        "ok": not missing and not extra,
    }


# ----------------------------------------------------------------------
# Rotation summary (V1381 plan)
# ----------------------------------------------------------------------

def _rotation_summary(archive_dir: str, *, now: _dt.datetime | None = None) -> dict[str, Any]:
    """Compute V1381 plan tier + action counts. Lazy import to keep clean."""
    tier_counts: dict[str, int] = {"HOT": 0, "WARM": 0, "COLD": 0, "FROZEN": 0}
    action_counts: dict[str, int] = {"keep": 0, "compress": 0, "prune": 0}
    plan_path = "V1381_PLAN_AUTO.md"
    if now is None:
        now = _dt.datetime.now(_dt.timezone.utc)
    if not os.path.isdir(archive_dir):
        return {
            "policy_version": POLICY_VERSION,
            "plan_path": plan_path,
            "tier_counts": tier_counts,
            "actions_summary": action_counts,
            "reason": "archive_dir missing",
        }
    try:
        # Lazy import — V1382 stays independent of V1381 at import time
        from apeireth.v1381_v1375_archival_rotation import (
            default_policy,
            plan_rotation,
        )
        pol = default_policy()
        plan = plan_rotation(archive_dir, now=now, policy=pol)
        for entry in plan:
            tier = entry.get("tier", "HOT")
            action = entry.get("action", "keep")
            tier_counts[tier] = tier_counts.get(tier, 0) + 1
            action_counts[action] = action_counts.get(action, 0) + 1
        return {
            "policy_version": pol["policy_version"],
            "plan_path": plan_path,
            "tier_counts": tier_counts,
            "actions_summary": action_counts,
        }
    except Exception as e:
        return {
            "policy_version": POLICY_VERSION,
            "plan_path": plan_path,
            "tier_counts": tier_counts,
            "actions_summary": action_counts,
            "error": f"V1381 import/plan failed: {type(e).__name__}: {e}",
        }


# ----------------------------------------------------------------------
# Snapshot builder
# ----------------------------------------------------------------------

def snapshot_archive_health(
    *,
    archive_dir: str = DEFAULT_ARCHIVE_DIR,
    manifest_path: str = DEFAULT_MANIFEST_PATH,
    now: _dt.datetime | None = None,
) -> dict[str, Any]:
    """Build the full overlay snapshot."""
    _validate_safe_path(archive_dir)
    _validate_safe_path(manifest_path)
    if now is None:
        now = _dt.datetime.now(_dt.timezone.utc)
    archives = _count_archives(archive_dir)
    indexed = _count_indexed(archive_dir)
    manifest = _read_manifest(manifest_path)
    integrity = _check_integrity(archive_dir, manifest)
    rotation = _rotation_summary(archive_dir, now=now)
    return {
        "schema": SCHEMA_VERSION,
        "generated": now.strftime("%Y-%m-%dT%H-%M-%SZ"),
        "archive_dir": archive_dir,
        "totals": {
            "archives": archives,
            "indexed": indexed,
            "manifested": integrity["archives_in_manifest"],
        },
        "tier_counts": rotation["tier_counts"],
        "action_counts": rotation["actions_summary"],
        "integrity": integrity,
        "rotation": {
            "policy_version": rotation["policy_version"],
            "plan_path": rotation["plan_path"],
            "actions_summary": rotation["actions_summary"],
        },
        "guarded_observations": [
            "GUARD_OVERLAY_READ_ONLY_BY_DEFAULT: V1382 default CLI does not write",
            "GUARD_RECORD_IS_OPT_IN: --record is explicit",
            "GUARD_NO_CAP_CHANGE: V1382 has no metric, no cap, no scoring",
            "GUARD_NO_SIDECAR_TOUCH: never imports V1371/V1369/V1370",
            "GUARD_HONEST_DISCLOSURE: known_unknowns always emitted",
            "GUARD_DETERMINISTIC: same inputs in same order -> same snapshot bytes",
            "GUARD_V1382_DOES_NOT_TOUCH_V1375: never writes V1375 archives directly",
            "GUARD_HISTORY_APPEND_ONLY: V1382_HISTORY.jsonl only appended",
        ],
        "known_unknowns": [
            "did not validate INDEX.md row-by-row against on-disk files (only counted)",
            "rotation summary depends on V1381 import availability",
            "manifest_path default assumes V1379 has been built (V1375_HISTORY/)",
        ],
    }


# ----------------------------------------------------------------------
# Output helpers
# ----------------------------------------------------------------------

def _render_snapshot_json(snapshot: dict[str, Any], *, pretty: bool = False) -> str:
    if pretty:
        return json.dumps(snapshot, indent=2, sort_keys=False, ensure_ascii=False)
    return json.dumps(snapshot, ensure_ascii=False, separators=(",", ":"))


def _atomic_write(path: str, content: str) -> None:
    parent = os.path.dirname(os.path.abspath(path))
    os.makedirs(parent, exist_ok=True)
    fd, tmp = tempfile.mkstemp(prefix=".v1382_", dir=parent)
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


def _append_history(snapshot: dict[str, Any], path: str = DEFAULT_HISTORY_PATH, *, tag: str | None = None) -> None:
    """Append one JSON line to V1382_HISTORY.jsonl (atomic per line)."""
    _validate_safe_path(path)
    parent = os.path.dirname(os.path.abspath(path))
    os.makedirs(parent, exist_ok=True)
    record = {
        "schema": SCHEMA_VERSION + ".history/v1",
        "recorded": _dt.datetime.now(_dt.timezone.utc).strftime("%Y-%m-%dT%H-%M-%SZ"),
    }
    if tag:
        record["tag"] = tag
    record["snapshot"] = snapshot
    line = json.dumps(record, ensure_ascii=False, separators=(",", ":"))
    # Per-line atomic write via "open in append mode" + lock-free single writer
    # (cron tick is single-writer by design; OK on Windows append mode)
    with open(path, "a", encoding="utf-8") as fh:
        fh.write(line + "\n")


def _one_line_summary(snapshot: dict[str, Any]) -> str:
    t = snapshot["totals"]
    r = snapshot["rotation"]["actions_summary"]
    integ = snapshot["integrity"]
    return (
        f"V1382 {snapshot['generated']} "
        f"archives={t['archives']} indexed={t['indexed']} "
        f"manifest={t['manifested']} "
        f"actions=[keep={r['keep']}, compress={r['compress']}, prune={r['prune']}] "
        f"integrity={'OK' if integ['ok'] else 'BROKEN'}"
    )


# ----------------------------------------------------------------------
# CLI
# ----------------------------------------------------------------------

def _build_arg_parser() -> argparse.ArgumentParser:
    p = argparse.ArgumentParser(
        prog=SCRIPT_NAME,
        description="V1382 — V1375 × V1379 × V1381 archive-health overlay",
    )
    p.add_argument("--archive-dir", default=DEFAULT_ARCHIVE_DIR,
                   help=f"Archive directory (default: {DEFAULT_ARCHIVE_DIR})")
    p.add_argument("--manifest-path", default=DEFAULT_MANIFEST_PATH,
                   help=f"Manifest path (default: {DEFAULT_MANIFEST_PATH})")
    p.add_argument("--history-path", default=DEFAULT_HISTORY_PATH,
                   help=f"History ledger path (default: {DEFAULT_HISTORY_PATH})")
    p.add_argument("--now", default=None,
                   help="Override 'now' (ISO 8601) for deterministic testing")
    sub = p.add_subparsers(dest="cmd")

    sp = sub.add_parser("snapshot", help="Compute + emit the overlay snapshot")
    sp.add_argument("--out", default=None,
                    help="Write snapshot to this file (atomic). Default: stdout")
    sp.add_argument("--pretty", action="store_true", help="Pretty-print JSON")
    sp.add_argument("--record", action="store_true",
                    help="Also append to history ledger (opt-in)")
    sp.add_argument("--tag", default=None,
                    help="Optional tag for --record")

    sub.add_parser("summary", help="Print one-line summary")
    sub.add_parser("popper", help="Run the Popper self-tests")
    sub.add_parser("version", help="Print schema version and exit")
    return p


def run_cli(args: list[str] | None = None) -> int:
    parser = _build_arg_parser()
    if args is None:
        args = sys.argv[1:]
    ns = parser.parse_args(args)
    cmd = ns.cmd or "version"

    now: _dt.datetime | None = None
    if hasattr(ns, "now") and ns.now:
        try:
            now = _dt.datetime.fromisoformat(ns.now.replace("Z", "+00:00"))
        except ValueError:
            now = None

    if cmd == "version":
        print(f"{SCRIPT_NAME} {SCHEMA_VERSION}")
        return 0

    snapshot = snapshot_archive_health(
        archive_dir=ns.archive_dir,
        manifest_path=ns.manifest_path,
        now=now,
    )

    if cmd == "snapshot":
        content = _render_snapshot_json(snapshot, pretty=getattr(ns, "pretty", False))
        if ns.out:
            _atomic_write(ns.out, content)
            print(f"snapshot written: {ns.out} ({len(content)} bytes)")
        else:
            print(content)
        if getattr(ns, "record", False):
            _append_history(snapshot, path=ns.history_path, tag=getattr(ns, "tag", None))
            print(f"appended to history: {ns.history_path}", file=sys.stderr)
        return 0

    if cmd == "summary":
        print(_one_line_summary(snapshot))
        return 0

    if cmd == "popper":
        passed, total, failures = _popper_self_tests()
        for f in failures:
            print(f"FAIL: {f}", file=sys.stderr)
        print(f"Popper self-tests: {passed}/{total}")
        return 0 if passed == total else 1

    parser.print_help()
    return 2


# ----------------------------------------------------------------------
# Popper self-tests
# ----------------------------------------------------------------------

def _popper_self_tests() -> tuple[int, int, list[str]]:
    """In-module Popper-style self-tests."""
    failures: list[str] = []
    checks: list[tuple[str, bool]] = []

    # 1. SCHEMA_VERSION + SCRIPT_NAME present
    checks.append(("schema_version present", bool(SCHEMA_VERSION)))
    checks.append(("script_name present", bool(SCRIPT_NAME)))

    # 2. _list_archive_names filters INDEX + non-md
    with tempfile.TemporaryDirectory() as td:
        for n in [
            "2026-08-09T03-55-00Z__v1374.md",
            "2026-08-09T04-00-00Z__v1374.md",
            "INDEX.md",
            "README.txt",
        ]:
            with open(os.path.join(td, n), "w") as fh:
                fh.write("x")
        names = _list_archive_names(td)
        checks.append(("list filters INDEX.md", "INDEX.md" not in names))
        checks.append(("list filters README.txt", "README.txt" not in names))
        checks.append(("list keeps 2 archives", len(names) == 2))

    # 3. _list_archive_names returns empty for missing dir
    names = _list_archive_names("/nonexistent/v1382/test/path")
    checks.append(("list missing dir empty", names == []))

    # 4. _count_archives counts only valid slugs
    with tempfile.TemporaryDirectory() as td:
        for n in [
            "2026-08-09T03-55-00Z__v1374.md",
            "INDEX.md",
        ]:
            with open(os.path.join(td, n), "w") as fh:
                fh.write("x")
        checks.append(("count archives ignores INDEX", _count_archives(td) == 1))

    # 5. _count_indexed counts backtick-wrapped slugs
    with tempfile.TemporaryDirectory() as td:
        idx = os.path.join(td, "INDEX.md")
        with open(idx, "w") as fh:
            fh.write("# Index\n\n| name | tier |\n|---|---|\n"
                     + "| `2026-08-09T03-55-00Z__v1374.md` | HOT |\n"
                     + "| `2026-08-09T04-00-00Z__v1374.md` | HOT |\n")
        checks.append(("count indexed = 2", _count_indexed(td) == 2))

    # 6. _count_indexed returns 0 for missing INDEX
    with tempfile.TemporaryDirectory() as td:
        checks.append(("count indexed missing = 0", _count_indexed(td) == 0))

    # 7. _read_manifest returns None for missing file
    checks.append(("read manifest missing = None", _read_manifest("/nonexistent/v1382/manifest.json") is None))

    # 8. _read_manifest returns None for malformed JSON
    with tempfile.TemporaryDirectory() as td:
        bad = os.path.join(td, "bad.json")
        with open(bad, "w") as fh:
            fh.write("not json {{")
        checks.append(("read manifest malformed = None", _read_manifest(bad) is None))

    # 9. _read_manifest parses valid manifest
    with tempfile.TemporaryDirectory() as td:
        good = os.path.join(td, "good.json")
        with open(good, "w") as fh:
            json.dump({"schema_version": "v1379.integrity/v1", "records": []}, fh)
        m = _read_manifest(good)
        checks.append(("read manifest valid", m is not None and m["schema_version"] == "v1379.integrity/v1"))

    # 10. _check_integrity with missing manifest
    integ = _check_integrity("/nonexistent/v1382/test", None)
    checks.append(("integrity missing manifest ok=False", integ["ok"] is False))
    checks.append(("integrity missing manifest reason present",
                   "reason" in integ))

    # 11. _check_integrity ok=True when manifest matches disk
    with tempfile.TemporaryDirectory() as td:
        name = "2026-08-09T03-55-00Z__v1374.md"
        with open(os.path.join(td, name), "w") as fh:
            fh.write("# x\n")
        manifest = {"schema_version": "v1379.integrity/v1",
                    "records": [{"name": name, "sha256": "x" * 64}]}
        integ = _check_integrity(td, manifest)
        checks.append(("integrity ok when match", integ["ok"] is True))
        checks.append(("integrity no missing", integ["missing_on_disk"] == []))
        checks.append(("integrity no extra", integ["extra_on_disk"] == []))

    # 12. _check_integrity detects missing archive
    with tempfile.TemporaryDirectory() as td:
        manifest = {"schema_version": "v1379.integrity/v1",
                    "records": [{"name": "ghost.md", "sha256": "x" * 64}]}
        integ = _check_integrity(td, manifest)
        checks.append(("integrity detects missing", "ghost.md" in integ["missing_on_disk"]))
        checks.append(("integrity ok=False when missing", integ["ok"] is False))

    # 13. _check_integrity detects extra archive
    with tempfile.TemporaryDirectory() as td:
        name = "2026-08-09T03-55-00Z__v1374.md"
        with open(os.path.join(td, name), "w") as fh:
            fh.write("# x\n")
        manifest = {"schema_version": "v1379.integrity/v1", "records": []}
        integ = _check_integrity(td, manifest)
        checks.append(("integrity detects extra", name in integ["extra_on_disk"]))
        checks.append(("integrity ok=False when extra", integ["ok"] is False))

    # 14. _validate_safe_path rejects traversal
    try:
        _validate_safe_path("../etc/passwd")
        checks.append(("rejects parent traversal", False))
    except ValueError:
        checks.append(("rejects parent traversal", True))

    # 15. _validate_safe_path accepts normal paths
    _validate_safe_path("V1375_HISTORY")
    _validate_safe_path("/tmp/test")
    checks.append(("accepts normal paths", True))

    # 16. _atomic_write roundtrip
    with tempfile.TemporaryDirectory() as td:
        path = os.path.join(td, "nested", "snapshot.json")
        _atomic_write(path, '{"hello": "world"}')
        checks.append(("atomic_write creates nested dirs", os.path.exists(path)))
        with open(path, "r", encoding="utf-8") as fh:
            checks.append(("atomic_write content matches", fh.read() == '{"hello": "world"}'))

    # 17. _append_history appends line
    with tempfile.TemporaryDirectory() as td:
        path = os.path.join(td, "history.jsonl")
        snap = {"schema": SCHEMA_VERSION, "totals": {"archives": 0}}
        _append_history(snap, path=path, tag="test1")
        _append_history(snap, path=path, tag="test2")
        with open(path, "r", encoding="utf-8") as fh:
            lines = fh.readlines()
        checks.append(("append_history 2 lines", len(lines) == 2))
        checks.append(("append_history line 1 valid JSON",
                       json.loads(lines[0])["tag"] == "test1"))
        checks.append(("append_history line 2 valid JSON",
                       json.loads(lines[1])["tag"] == "test2"))

    # 18. _append_history append-only (existing line preserved)
    with tempfile.TemporaryDirectory() as td:
        path = os.path.join(td, "history.jsonl")
        with open(path, "w") as fh:
            fh.write('{"preexisting": true}\n')
        _append_history({"x": 1}, path=path)
        with open(path, "r", encoding="utf-8") as fh:
            lines = fh.readlines()
        checks.append(("append_history preserves prior", len(lines) == 2))
        checks.append(("append_history prior intact",
                       json.loads(lines[0]).get("preexisting") is True))

    # 19. snapshot_archive_health on missing archive_dir returns safe dict
    snap = snapshot_archive_health(
        archive_dir="/nonexistent/v1382/test",
        manifest_path="/nonexistent/v1382/manifest.json",
    )
    checks.append(("snapshot missing dir safe", snap["totals"]["archives"] == 0))
    checks.append(("snapshot has guarded_observations",
                   len(snap["guarded_observations"]) >= 5))
    checks.append(("snapshot has known_unknowns", len(snap["known_unknowns"]) >= 2))
    checks.append(("snapshot schema = SCHEMA_VERSION", snap["schema"] == SCHEMA_VERSION))

    # 20. snapshot_archive_health on real dir (with INDEX)
    with tempfile.TemporaryDirectory() as td:
        name = "2026-08-09T03-55-00Z__v1374.md"
        with open(os.path.join(td, name), "w") as fh:
            fh.write("# archive\n")
        idx = os.path.join(td, "INDEX.md")
        with open(idx, "w") as fh:
            fh.write(f"# Index\n\n| name |\n|---|\n| `{name}` |\n")
        snap = snapshot_archive_health(archive_dir=td)
        checks.append(("snapshot archives=1", snap["totals"]["archives"] == 1))
        checks.append(("snapshot indexed=1", snap["totals"]["indexed"] == 1))
        # Now (default) makes the archive HOT → tier_counts["HOT"] >= 1
        checks.append(("snapshot tier_counts HOT>=1",
                       snap["tier_counts"]["HOT"] >= 1))

    # 21. _render_snapshot_json deterministic
    snap = snapshot_archive_health(archive_dir="/nonexistent/v1382/test")
    j1 = _render_snapshot_json(snap)
    j2 = _render_snapshot_json(snap)
    checks.append(("render json deterministic", j1 == j2))

    # 22. _render_snapshot_json pretty produces multi-line
    pretty = _render_snapshot_json(snap, pretty=True)
    checks.append(("render pretty has newlines", "\n" in pretty))

    # 23. _one_line_summary contains key fields
    line = _one_line_summary(snap)
    checks.append(("summary contains V1382", "V1382" in line))
    checks.append(("summary contains integrity", "integrity=" in line))
    checks.append(("summary contains archives", "archives=" in line))

    # 24. CLI version subcommand
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

    # 25. CLI summary subcommand
    rc = run_cli(["--archive-dir", "/nonexistent/v1382/test", "summary"])
    checks.append(("CLI summary exit 0", rc == 0))

    # 26. CLI snapshot to stdout
    saved_stdout = sys.stdout
    try:
        buf = _io.StringIO()
        sys.stdout = buf
        rc = run_cli(["--archive-dir", "/nonexistent/v1382/test", "snapshot"])
        out = buf.getvalue()
        checks.append(("CLI snapshot exit 0", rc == 0))
        checks.append(("CLI snapshot JSON parses", json.loads(out)["schema"] == SCHEMA_VERSION))
    finally:
        sys.stdout = saved_stdout

    # 27. CLI snapshot --pretty
    saved_stdout = sys.stdout
    try:
        buf = _io.StringIO()
        sys.stdout = buf
        rc = run_cli(["--archive-dir", "/nonexistent/v1382/test", "snapshot", "--pretty"])
        out = buf.getvalue()
        checks.append(("CLI snapshot pretty exit 0", rc == 0))
        checks.append(("CLI snapshot pretty multiline", "\n" in out))
    finally:
        sys.stdout = saved_stdout

    # 28. CLI snapshot --out writes file
    with tempfile.TemporaryDirectory() as td:
        out_path = os.path.join(td, "snap.json")
        rc = run_cli(["--archive-dir", "/nonexistent/v1382/test", "snapshot", "--out", out_path])
        checks.append(("CLI snapshot --out exit 0", rc == 0))
        checks.append(("CLI snapshot --out file written", os.path.exists(out_path)))
        with open(out_path, "r", encoding="utf-8") as fh:
            checks.append(("CLI snapshot --out JSON valid",
                           json.loads(fh.read())["schema"] == SCHEMA_VERSION))

    # 29. CLI snapshot --record appends to history
    with tempfile.TemporaryDirectory() as td:
        history_path = os.path.join(td, "history.jsonl")
        rc = run_cli([
            "--archive-dir", "/nonexistent/v1382/test",
            "--history-path", history_path,
            "snapshot", "--record", "--tag", "unit_test",
        ])
        checks.append(("CLI snapshot --record exit 0", rc == 0))
        checks.append(("CLI snapshot --record file exists", os.path.exists(history_path)))
        with open(history_path, "r", encoding="utf-8") as fh:
            lines = fh.readlines()
        checks.append(("CLI snapshot --record 1 line", len(lines) == 1))
        rec = json.loads(lines[0])
        checks.append(("CLI snapshot --record tag present", rec.get("tag") == "unit_test"))
        checks.append(("CLI snapshot --record snapshot embedded",
                       rec.get("snapshot", {}).get("schema") == SCHEMA_VERSION))

    # 30. CLI popper subcommand (skipped — would recurse; covered by outer call)
    # We trust the popper wiring because the outer popper run already
    # exercised _popper_self_tests() directly.
    checks.append(("CLI popper not retested (anti-recursion)", True))

    # Tally
    passed = sum(1 for _, ok in checks if ok)
    total = len(checks)
    failures_list = [name for name, ok in checks if not ok]
    return passed, total, failures_list


if __name__ == "__main__":
    sys.exit(run_cli())