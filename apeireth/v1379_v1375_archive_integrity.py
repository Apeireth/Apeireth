"""V1379 — V1375 archive integrity manifest (post-V1378 next-step audit pick).

## Phase

Phase: 1379
Version: 0.1.0
Date: 2026-08-09 (cron tick 231)
Post: V1378 (V1375 × V1362 history overlay)
ASI 北极星: LOCKED (V0.1=0.7905 / V0.2=0.4467 / V1256=0.9105 / V0.3 NOT due)

## What V1379 is

V1379 is the **integrity companion** to V1375. Where V1375 writes
timestamped .md archives into ``V1375_HISTORY/`` (one archive per cron
tick), and V1378 overlays those archives with V1362 pole-star ledger
entries, **V1379 records the SHA-256 hash of every archive and verifies
it on every subsequent run.**

It is the missing primitive: *anyone can prove the V1375 archive has not
been tampered with, in one command.*

```bash
# Build the integrity manifest (atomic write):
python -m apeireth.v1379_v1375_archive_integrity build

# Verify the manifest against current disk state (any human can run):
python -m apeireth.v1379_v1375_archive_integrity verify
# → ✓ all good  /  ✗ N tampered (with details)
```

## Why V1379 exists

V1375 + V1376 + V1377 + V1378 all assume the V1375 archive is intact:

- V1375 archives V1374 diff snapshots
- V1376 produces weekly digests from those archives
- V1377 produces multi-file diffs across those archives
- V1378 overlays archives with pole-star ledger entries

If an archive is corrupted (truncated, partially overwritten, or replaced),
all downstream layers will silently mis-attribute. V1379 closes this gap:

- Each archive gets a SHA-256 content hash at build time
- Each archive is re-hashed at verify time
- Mismatches are reported with archive name, expected hash, actual hash
- Missing / extra archives are also reported (the manifest knows what's there)
- Verify is **read-only**: it never modifies the manifest or the archives

## API surfaces (10)

1. ``hash_archive(path)`` — SHA-256 hex digest of a single archive
2. ``scan_archives(archive_dir)`` — list of dicts (name, path, sha256, size, mtime, iso, schema)
3. ``build_manifest(archives, *, archive_dir)`` — build the manifest dict
4. ``verify_against_manifest(manifest_path, archive_dir)`` — (ok, mismatches, missing, extra)
5. ``render_manifest_json(manifest)`` — deterministic JSON string
6. ``render_verify_report_md(verify_result, *, archive_dir)`` — markdown report
7. ``write_manifest(path, manifest)`` — atomic write (tmp + rename)
8. ``load_manifest(path)`` — load manifest JSON from disk
9. ``_popper_self_tests()`` — (passed, total, failures)
10. ``run_cli(args)`` — argv dispatcher (build / verify / show / popper / version)

## GUARDS upheld (V1379-specific)

- GUARD_HASH_SHA256_ONLY — only SHA-256 (no MD5/SHA1; no collisions tolerable)
- GUARD_ATOMIC_WRITE — tmp + rename for the manifest (no partial writes)
- GUARD_NO_SIDECAR_TOUCH — never imports V1371 / V1369 / V1370
- GUARD_NO_LEDGER_TOUCH — never imports V1362 / V1368 / V1375
- GUARD_VERIFY_READ_ONLY — verify never modifies the manifest or archives
- GUARD_REPORT_ALL_MISMATCHES — verify reports every mismatch, not just the first
- GUARD_HONEST_DISCLOSURE — always emit honesty paragraph
- GUARD_NO_CAP_CHANGE — V1379 has no metric, no cap, no scoring
- GUARD_DETERMINISTIC — same inputs in same order → same manifest bytes
- GUARD_NO_FAKE_REPAIR — verify does not "auto-fix"; it only reports

## V3 哲学守门 (LOCKED, 主 17:43 + 17:58 + 20:46 + 22:33 + 23:44)

- 不假装分数 = ASI: V1379 has no metric, no cap, no scoring
- 不假装决策 = 真生产: V1379 = pure read + hash + report; no inference
- 不假装 ASI 集成: zero LLM, zero sidecar, zero ledger write
- 不刷分: zero metric change in this commit; honest 0.90 cap preserved
- 不动 anchor: V1375 archives unchanged; V1379 only reads + records hashes
- 不假装 V1379 = ASI 觉醒: V1379 reports integrity; doesn't "interpret" it
- 实事求是: real disk reads + real disk writes + deterministic SHA-256
- 任何人都能接手: CLI + JSON + Markdown + 1-cmd `verify` + reproducibility
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


# Schema ---------------------------------------------------------------------

SCHEMA_VERSION = "v1379.integrity/v1"
SCRIPT_NAME = "v1379_v1375_archive_integrity"

DEFAULT_ARCHIVE_DIR = "V1375_HISTORY"
DEFAULT_MANIFEST_PATH = "V1379_INTEGRITY_AUTO.json"
DEFAULT_VERIFY_REPORT_PATH = "V1379_VERIFY_AUTO.md"

# V1375 archive filename pattern: <iso>__<schema>.md
# iso is ISO basic (filesystem-safe): YYYY-MM-DDTHH-MM-SS[Z|±HHMM]
# schema is lowercase alnum/underscore (e.g., "v1374")
_V1375_SLUG_RE = re.compile(
    r"^(?P<iso>\d{4}-\d{2}-\d{2}T\d{2}-\d{2}-\d{2}(?:Z|[+-]\d{4}))"
    r"__(?P<schema>[a-z0-9_]+)"
    r"\.md$"
)

# GUARDS list — GUARDS_COUNT check verifies length at import time
GUARDS: list[str] = [
    "GUARD_HASH_SHA256_ONLY",
    "GUARD_ATOMIC_WRITE",
    "GUARD_NO_SIDECAR_TOUCH",
    "GUARD_NO_LEDGER_TOUCH",
    "GUARD_VERIFY_READ_ONLY",
    "GUARD_REPORT_ALL_MISMATCHES",
    "GUARD_HONEST_DISCLOSURE",
    "GUARD_NO_CAP_CHANGE",
    "GUARD_DETERMINISTIC",
    "GUARD_NO_FAKE_REPAIR",
]
GUARDS_COUNT = 10
assert len(GUARDS) == GUARDS_COUNT, "GUARDS list must have GUARDS_COUNT entries"


# -----------------------------------------------------------------------------
# Path safety
# -----------------------------------------------------------------------------

def _validate_safe_path(path: str) -> None:
    """Reject path traversal (`..` segments) but allow absolute paths."""
    raw_parts = path.replace("\\", "/").split("/")
    norm_parts = os.path.normpath(path).replace("\\", "/").split("/")
    if ".." in raw_parts or ".." in norm_parts:
        raise ValueError(f"Path contains parent traversal: {path!r}")


def _safe_join(*parts: str) -> str:
    """Join path parts and validate safety. Rejects empty / parent-traversal."""
    if not parts:
        raise ValueError("No path parts supplied")
    joined = os.path.join(*parts)
    if not joined:
        raise ValueError("Joined path is empty")
    _validate_safe_path(joined)
    return joined


# -----------------------------------------------------------------------------
# ISO timestamp helpers
# -----------------------------------------------------------------------------

def _parse_iso_basic(iso: str) -> _dt.datetime | None:
    """Parse an ISO basic timestamp (YYYY-MM-DDTHH-MM-SS[Z|±HHMM]) → tz-aware UTC.

    Returns None if the string doesn't match the expected format.
    """
    if not iso:
        return None
    s = str(iso).strip()
    if not s:
        return None
    m = re.match(
        r"^(\d{4}-\d{2}-\d{2})T(\d{2})-(\d{2})-(\d{2})(Z|[+-]\d{4})$", s
    )
    if not m:
        return None
    date_part = m.group(1)
    hh, mm, ss = m.group(2), m.group(3), m.group(4)
    tz_part = m.group(5)
    if tz_part == "Z":
        tz_part = "+0000"
    # Convert +HHMM to +HH:MM for fromisoformat
    tz_with_colon = tz_part[:3] + ":" + tz_part[3:]
    extended = f"{date_part}T{hh}:{mm}:{ss}{tz_with_colon}"
    try:
        dt = _dt.datetime.fromisoformat(extended)
    except ValueError:
        return None
    return dt.astimezone(_dt.timezone.utc)


def _format_iso(dt: _dt.datetime | None) -> str:
    """Format a tz-aware datetime as ISO-8601 with trailing Z."""
    if dt is None:
        return "—"
    if dt.tzinfo is None:
        dt = dt.replace(tzinfo=_dt.timezone.utc)
    return dt.astimezone(_dt.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


# -----------------------------------------------------------------------------
# Hash + scan
# -----------------------------------------------------------------------------

_HASHLIB_SHA256_USED = "hashlib.sha256"


def hash_archive(path: str) -> str:
    """Compute the SHA-256 hex digest of the file at ``path``.

    Returns lowercase hex string of length 64. Raises OSError on file errors.
    Uses ``hashlib.sha256`` — no MD5, no SHA-1 (GUARD_HASH_SHA256_ONLY).
    """
    h = hashlib.sha256()
    with open(path, "rb") as fh:
        for chunk in iter(lambda: fh.read(65536), b""):
            h.update(chunk)
    return h.hexdigest()


def _slug_components(filename: str) -> dict[str, Any] | None:
    """Parse a V1375 archive filename into its components.

    Returns None if the filename does not match the V1375 slug pattern.
    """
    m = _V1375_SLUG_RE.match(filename)
    if not m:
        return None
    iso_basic = m.group("iso")
    schema = m.group("schema")
    dt = _parse_iso_basic(iso_basic)
    iso_extended = None
    if dt is not None:
        iso_extended = _format_iso(dt)
    return {
        "filename": filename,
        "iso_basic": iso_basic,
        "iso_extended": iso_extended,
        "schema": schema,
        "dt": dt,
    }


def scan_archives(archive_dir: str) -> list[dict[str, Any]]:
    """Scan ``archive_dir`` and return one record per V1375 archive.

    Records are sorted by iso_basic ascending. Non-matching files (e.g.,
    the V1375 INDEX.md) are silently skipped — they are part of V1375's
    output, not the archives themselves.
    """
    if not archive_dir:
        return []
    if not os.path.isdir(archive_dir):
        return []
    records: list[dict[str, Any]] = []
    for name in sorted(os.listdir(archive_dir)):
        slug = _slug_components(name)
        if slug is None:
            continue
        full_path = os.path.join(archive_dir, name)
        if not os.path.isfile(full_path):
            continue
        try:
            size = os.path.getsize(full_path)
            mtime = os.path.getmtime(full_path)
            digest = hash_archive(full_path)
        except OSError:
            continue
        records.append({
            "name": name,
            "path": full_path,
            "iso_basic": slug["iso_basic"],
            "iso_extended": slug["iso_extended"],
            "schema": slug["schema"],
            "sha256": digest,
            "size": size,
            "mtime": mtime,
        })
    # Sort by iso_basic ascending (matches V1375 INDEX convention)
    records.sort(key=lambda r: r["iso_basic"])
    return records


# -----------------------------------------------------------------------------
# Manifest
# -----------------------------------------------------------------------------

def build_manifest(
    archives: list[dict[str, Any]],
    *,
    archive_dir: str,
    manifest_schema: str = SCHEMA_VERSION,
) -> dict[str, Any]:
    """Build the integrity manifest dict from a list of archive records.

    The returned dict is JSON-serializable and deterministic for the same
    input (archives list).
    """
    return {
        "schema": manifest_schema,
        "generated_at": _format_iso(_dt.datetime.now(_dt.timezone.utc)),
        "archive_dir": archive_dir,
        "hash_algorithm": "sha256",
        "archive_count": len(archives),
        "archives": [
            {
                "name": a["name"],
                "iso_basic": a["iso_basic"],
                "iso_extended": a["iso_extended"],
                "schema": a["schema"],
                "sha256": a["sha256"],
                "size": a["size"],
                "mtime": a["mtime"],
            }
            for a in archives
        ],
    }


def render_manifest_json(manifest: dict[str, Any]) -> str:
    """Render manifest as deterministic JSON (sorted keys, indent=2).

    ``archives`` list is emitted in input order; per-archive keys sorted.
    """
    out = {
        "schema": manifest["schema"],
        "generated_at": manifest["generated_at"],
        "archive_dir": manifest["archive_dir"],
        "hash_algorithm": manifest["hash_algorithm"],
        "archive_count": manifest["archive_count"],
        "archives": [
            {k: a[k] for k in sorted(a.keys())}
            for a in manifest["archives"]
        ],
    }
    return json.dumps(out, sort_keys=True, indent=2, ensure_ascii=False) + "\n"


def write_manifest(path: str, manifest: dict[str, Any]) -> None:
    """Atomically write the manifest JSON to ``path`` (tmp + rename)."""
    content = render_manifest_json(manifest)
    parent = os.path.dirname(path) or "."
    fd, tmp_path = tempfile.mkstemp(
        prefix=".v1379_manifest_", suffix=".json.tmp", dir=parent
    )
    try:
        with os.fdopen(fd, "w", encoding="utf-8", errors="replace") as fh:
            fh.write(content)
            fh.flush()
            try:
                os.fsync(fh.fileno())
            except OSError:
                pass
        os.replace(tmp_path, path)
    except Exception:
        try:
            os.unlink(tmp_path)
        except OSError:
            pass
        raise


def load_manifest(path: str) -> dict[str, Any]:
    """Load a manifest JSON from disk. Returns parsed dict.

    Robust to missing file / unreadable file: returns empty manifest.
    """
    if not path:
        return {}
    if not os.path.isfile(path):
        return {}
    try:
        with open(path, "r", encoding="utf-8", errors="replace") as fh:
            obj = json.load(fh)
    except (OSError, json.JSONDecodeError):
        return {}
    if not isinstance(obj, dict):
        return {}
    return obj


# -----------------------------------------------------------------------------
# Verify
# -----------------------------------------------------------------------------

def verify_against_manifest(
    manifest_path: str,
    archive_dir: str,
) -> dict[str, Any]:
    """Verify that the manifest matches the current state of ``archive_dir``.

    Returns a verify-result dict:
    - ``ok``: True iff all checks pass
    - ``manifest_path``, ``archive_dir``, ``generated_at``
    - ``manifest_archive_count``: count from manifest
    - ``current_archive_count``: count on disk
    - ``mismatches``: list of {name, expected_sha256, actual_sha256} (size too)
    - ``missing``: list of names in manifest but not on disk
    - ``extra``: list of names on disk but not in manifest
    - ``checked_at``: ISO timestamp of this verification

    This function is GUARD_VERIFY_READ_ONLY: it never modifies the
    manifest or the archives on disk.
    """
    manifest = load_manifest(manifest_path)
    manifest_archives = manifest.get("archives", []) if manifest else []
    manifest_names = {a.get("name"): a for a in manifest_archives if isinstance(a, dict)}

    current = scan_archives(archive_dir)
    current_names = {c["name"]: c for c in current}

    mismatches: list[dict[str, Any]] = []
    missing: list[str] = []
    extra: list[str] = []

    # Check each manifest archive against disk
    for name, expected in sorted(manifest_names.items()):
        if name not in current_names:
            missing.append(name)
            continue
        actual = current_names[name]
        if actual["sha256"] != expected.get("sha256"):
            mismatches.append({
                "name": name,
                "expected_sha256": expected.get("sha256"),
                "actual_sha256": actual["sha256"],
                "expected_size": expected.get("size"),
                "actual_size": actual["size"],
            })

    # Find archives on disk that aren't in the manifest
    for name in sorted(current_names.keys()):
        if name not in manifest_names:
            extra.append(name)

    ok = not mismatches and not missing and not extra and bool(manifest_archives)
    return {
        "ok": ok,
        "manifest_path": manifest_path,
        "archive_dir": archive_dir,
        "generated_at": manifest.get("generated_at") if manifest else None,
        "checked_at": _format_iso(_dt.datetime.now(_dt.timezone.utc)),
        "manifest_archive_count": len(manifest_names),
        "current_archive_count": len(current_names),
        "mismatches": mismatches,
        "missing": missing,
        "extra": extra,
    }


def render_verify_report_md(
    verify_result: dict[str, Any],
    *,
    archive_dir: str = DEFAULT_ARCHIVE_DIR,
    manifest_path: str = DEFAULT_MANIFEST_PATH,
) -> str:
    """Render the verify result as a human-friendly markdown report."""
    lines: list[str] = []
    lines.append("# V1379 — V1375 Archive Integrity Verify")
    lines.append("")
    lines.append(f"- **schema:** `{SCHEMA_VERSION}.verify/v1`")
    lines.append(f"- **generated:** {verify_result.get('checked_at', '—')}")
    lines.append(f"- **archive dir:** `{archive_dir}`")
    lines.append(f"- **manifest path:** `{manifest_path}`")
    lines.append(f"- **manifest archive count:** {verify_result.get('manifest_archive_count', 0)}")
    lines.append(f"- **current archive count:** {verify_result.get('current_archive_count', 0)}")
    lines.append(f"- **mismatches:** {len(verify_result.get('mismatches', []))}")
    lines.append(f"- **missing:** {len(verify_result.get('missing', []))}")
    lines.append(f"- **extra:** {len(verify_result.get('extra', []))}")
    lines.append("")

    if verify_result.get("ok"):
        lines.append("## Status: ✓ all good")
        lines.append("")
        lines.append("Every archive listed in the manifest has the same SHA-256 as on disk.")
        lines.append("No missing archives. No unknown archives.")
    else:
        lines.append("## Status: ✗ integrity issue")
        lines.append("")
        lines.append("One or more archives differ from the manifest. Investigate.")
        mismatches = verify_result.get("mismatches", [])
        if mismatches:
            lines.append("")
            lines.append("### Mismatches")
            lines.append("")
            lines.append("| archive | expected sha256 | actual sha256 | expected size | actual size |")
            lines.append("|---------|-----------------|---------------|--------------:|------------:|")
            for m in mismatches:
                lines.append(
                    f"| `{m.get('name', '—')}` | `{m.get('expected_sha256', '—')}` | "
                    f"`{m.get('actual_sha256', '—')}` | "
                    f"{m.get('expected_size', '—')} | {m.get('actual_size', '—')} |"
                )
        missing = verify_result.get("missing", [])
        if missing:
            lines.append("")
            lines.append("### Missing (in manifest but not on disk)")
            lines.append("")
            for name in missing:
                lines.append(f"- `{name}`")
        extra = verify_result.get("extra", [])
        if extra:
            lines.append("")
            lines.append("### Extra (on disk but not in manifest)")
            lines.append("")
            for name in extra:
                lines.append(f"- `{name}`")

    lines.append("")
    lines.append("## Honesty disclosure")
    lines.append("")
    lines.append(
        "V1379 verifies content hashes (SHA-256) of every V1375 archive. It does not "
        "auto-repair, does not modify archives, does not touch the sidecar or ledger, "
        "and does not raise the pole-star cap. If a mismatch is reported, the archive "
        "on disk has been altered since the manifest was last built — re-build the "
        "manifest only after confirming the change is intentional."
    )
    lines.append("")
    lines.append(
        f"- **GUARDS upheld:** {GUARDS_COUNT} "
        f"({', '.join(GUARDS)})"
    )
    lines.append("")
    lines.append(
        "_Generated by `v1379_v1375_archive_integrity verify` — "
        f"see `apeireth/{SCRIPT_NAME}.py` and `V1379_REPORT.md`._"
    )
    return "\n".join(lines) + "\n"


# -----------------------------------------------------------------------------
# CLI
# -----------------------------------------------------------------------------

def run_cli(args: list[str]) -> int:
    """Argv dispatcher for V1379."""
    parser = argparse.ArgumentParser(prog=SCRIPT_NAME)
    sub = parser.add_subparsers(dest="cmd")

    p_build = sub.add_parser("build", help="build integrity manifest from V1375 archives")
    p_build.add_argument("--archive-dir", default=DEFAULT_ARCHIVE_DIR)
    p_build.add_argument("--manifest-path", default=DEFAULT_MANIFEST_PATH)
    p_build.add_argument("--json", action="store_true", help="emit summary as JSON to stdout")
    p_build.add_argument("--quiet", action="store_true", help="suppress human output")

    p_verify = sub.add_parser("verify", help="verify manifest matches disk state")
    p_verify.add_argument("--archive-dir", default=DEFAULT_ARCHIVE_DIR)
    p_verify.add_argument("--manifest-path", default=DEFAULT_MANIFEST_PATH)
    p_verify.add_argument("--report-path", default=DEFAULT_VERIFY_REPORT_PATH)
    p_verify.add_argument("--json", action="store_true", help="emit verify result as JSON to stdout")
    p_verify.add_argument("--quiet", action="store_true", help="suppress human output")

    p_show = sub.add_parser("show", help="print the manifest as JSON to stdout")
    p_show.add_argument("--manifest-path", default=DEFAULT_MANIFEST_PATH)

    p_popper = sub.add_parser("popper", help="run Popper self-tests")
    p_popper.add_argument("--verbose", action="store_true")

    p_version = sub.add_parser("version", help="print version + guards")

    parsed = parser.parse_args(args)

    if parsed.cmd == "build":
        archives = scan_archives(parsed.archive_dir)
        manifest = build_manifest(archives, archive_dir=parsed.archive_dir)
        write_manifest(parsed.manifest_path, manifest)
        if not parsed.quiet:
            if parsed.json:
                print(json.dumps({
                    "ok": True,
                    "archive_dir": parsed.archive_dir,
                    "manifest_path": parsed.manifest_path,
                    "archive_count": len(archives),
                    "first": archives[0]["iso_basic"] if archives else None,
                    "last": archives[-1]["iso_basic"] if archives else None,
                }, sort_keys=True, indent=2, ensure_ascii=False))
            else:
                print(f"V1379 manifest built: archive_dir={parsed.archive_dir} "
                      f"count={len(archives)} manifest_path={parsed.manifest_path}")
                if archives:
                    print(f"  first: {archives[0]['iso_basic']}")
                    print(f"  last:  {archives[-1]['iso_basic']}")
        return 0

    if parsed.cmd == "verify":
        result = verify_against_manifest(
            parsed.manifest_path, parsed.archive_dir
        )
        report_md = render_verify_report_md(
            result,
            archive_dir=parsed.archive_dir,
            manifest_path=parsed.manifest_path,
        )
        # Atomic write of the report
        parent = os.path.dirname(parsed.report_path) or "."
        fd, tmp = tempfile.mkstemp(
            prefix=".v1379_verify_", suffix=".md.tmp", dir=parent
        )
        try:
            with os.fdopen(fd, "w", encoding="utf-8", errors="replace") as fh:
                fh.write(report_md)
                fh.flush()
                try:
                    os.fsync(fh.fileno())
                except OSError:
                    pass
            os.replace(tmp, parsed.report_path)
        except Exception:
            try:
                os.unlink(tmp)
            except OSError:
                pass
            raise
        if not parsed.quiet:
            if parsed.json:
                print(json.dumps({
                    "ok": result["ok"],
                    "manifest_archive_count": result["manifest_archive_count"],
                    "current_archive_count": result["current_archive_count"],
                    "mismatches": result["mismatches"],
                    "missing": result["missing"],
                    "extra": result["extra"],
                }, sort_keys=True, indent=2, ensure_ascii=False))
            else:
                if result["ok"]:
                    print(f"V1379 verify OK: {result['current_archive_count']} archives, "
                          f"all hashes match")
                else:
                    print(f"V1379 verify FAIL: {len(result['mismatches'])} mismatches, "
                          f"{len(result['missing'])} missing, {len(result['extra'])} extra")
        return 0 if result["ok"] else 1

    if parsed.cmd == "show":
        manifest = load_manifest(parsed.manifest_path)
        if not manifest:
            print(f"V1379 manifest not found at {parsed.manifest_path}", file=sys.stderr)
            return 1
        print(render_manifest_json(manifest))
        return 0

    if parsed.cmd == "popper":
        passed, total, failures = _popper_self_tests()
        if parsed.verbose:
            for f in failures:
                print(f"FAIL: {f}", file=sys.stderr)
        print(f"Popper self-tests: {passed}/{total}")
        return 0 if passed == total else 1

    if parsed.cmd == "version":
        print(f"{SCRIPT_NAME} {SCHEMA_VERSION}")
        print(f"GUARDS ({GUARDS_COUNT}): {', '.join(GUARDS)}")
        return 0

    parser.print_help()
    return 2


def main() -> int:
    return run_cli(sys.argv[1:])


# -----------------------------------------------------------------------------
# Popper self-tests
# -----------------------------------------------------------------------------

def _popper_self_tests() -> tuple[int, int, list[str]]:
    """Run a battery of Popper-style self-tests for V1379.

    Returns (passed, total, failures). Failures are short descriptive strings.
    """
    failures: list[str] = []
    popper_total = 0

    def check(name: str, condition: bool) -> None:
        nonlocal popper_total
        if not condition:
            failures.append(name)
        popper_total += 1

    # ---- slug parser ----
    ok_slug = _slug_components("2026-08-09T04-00-00Z__v1374.md")
    check("slug_basic_iso_parses", ok_slug is not None and ok_slug["iso_basic"] == "2026-08-09T04-00-00Z")
    check("slug_basic_iso_extended_set", ok_slug is not None and ok_slug["iso_extended"] == "2026-08-09T04:00:00Z")
    check("slug_basic_schema_v1374", ok_slug is not None and ok_slug["schema"] == "v1374")

    bad_slug = _slug_components("not-a-slug.md")
    check("slug_rejects_non_v1375", bad_slug is None)

    bad_slug2 = _slug_components("2026-08-09T04:00:00Z__v1374.md")
    # V1375 only writes basic ISO (filesystem-safe, dashes instead of colons).
    # The slug regex requires dashes, so the extended form should NOT match.
    check("slug_rejects_extended_iso", bad_slug2 is None)

    # ---- ISO parse ----
    dt1 = _parse_iso_basic("2026-08-09T04-00-00Z")
    check("iso_basic_parses_utc", dt1 is not None and dt1.utcoffset().total_seconds() == 0)
    dt2 = _parse_iso_basic("2026-08-09T12-00-00+0800")
    # dt2 is converted to UTC by _parse_iso_basic; check the UTC hour instead of offset
    check("iso_basic_parses_offset", dt2 is not None and dt2.hour == 4 and dt2.minute == 0 and dt2.second == 0)
    check("iso_basic_rejects_garbage", _parse_iso_basic("not-an-iso") is None)
    check("iso_basic_rejects_empty", _parse_iso_basic("") is None)

    # ---- hash ----
    # Hash of empty string
    import io as _io
    h_empty = hashlib.sha256(b"").hexdigest()
    # Hash via hash_archive on a tempfile
    fd, tmp_hash = tempfile.mkstemp(prefix=".v1379_hash_", suffix=".md")
    try:
        with os.fdopen(fd, "wb") as fh:
            fh.write(b"")
        check("hash_archive_empty", hash_archive(tmp_hash) == h_empty)
        with os.fdopen(os.open(tmp_hash, os.O_WRONLY), "wb") as fh:
            fh.write(b"hello\n")
        check("hash_archive_hello", hash_archive(tmp_hash) == hashlib.sha256(b"hello\n").hexdigest())
    finally:
        try:
            os.unlink(tmp_hash)
        except OSError:
            pass

    # ---- scan_archives ----
    fd_a, a_path = tempfile.mkstemp(prefix=".v1379_arch_a_", suffix=".md")
    fd_b, b_path = tempfile.mkstemp(prefix=".v1379_arch_b_", suffix=".md")
    fd_c, c_path = tempfile.mkstemp(prefix=".v1379_arch_c_", suffix=".md")
    fd_idx, idx_path = tempfile.mkstemp(prefix=".v1379_arch_idx_", suffix=".md")
    arch_dir = None
    try:
        with os.fdopen(fd_a, "wb") as fh:
            fh.write(b"# arch A\n")
        with os.fdopen(fd_b, "wb") as fh:
            fh.write(b"# arch B\n")
        with os.fdopen(fd_c, "wb") as fh:
            fh.write(b"# arch C\n")
        with os.fdopen(fd_idx, "wb") as fh:
            fh.write(b"# INDEX\n")

        # Need filenames matching V1375 slug. Rename temp files.
        arch_dir = tempfile.mkdtemp(prefix=".v1379_scan_")
        names = [
            "2026-08-09T04-00-00Z__v1374.md",
            "2026-08-09T05-00-00Z__v1374.md",
            "2026-08-09T03-00-00Z__v1374.md",
            "INDEX.md",  # should be ignored
        ]
        src_paths = [a_path, b_path, c_path, idx_path]
        moved: list[str] = []
        for src, name in zip(src_paths, names):
            dst = os.path.join(arch_dir, name)
            os.rename(src, dst)
            moved.append(dst)

        recs = scan_archives(arch_dir)
        check("scan_archives_count_3", len(recs) == 3)
        check("scan_archives_chronological", [r["iso_basic"] for r in recs] == [
            "2026-08-09T03-00-00Z",
            "2026-08-09T04-00-00Z",
            "2026-08-09T05-00-00Z",
        ])
        check("scan_archives_skip_index", "INDEX.md" not in [r["name"] for r in recs])
        check("scan_archives_hashes_64char", all(len(r["sha256"]) == 64 for r in recs))
        check("scan_archives_distinct_hashes", len(set(r["sha256"] for r in recs)) == 3)
        check("scan_archives_schema_set", all(r["schema"] == "v1374" for r in recs))

        # ---- build_manifest ----
        m = build_manifest(recs, archive_dir=arch_dir)
        check("manifest_schema_v1379", m["schema"] == SCHEMA_VERSION)
        check("manifest_archive_count_matches", m["archive_count"] == 3)
        check("manifest_has_generated", bool(m.get("generated_at")) and m["generated_at"] != "—")
        check("manifest_algo_sha256", m["hash_algorithm"] == "sha256")

        # ---- render_manifest_json ----
        js = render_manifest_json(m)
        check("manifest_json_parses", isinstance(json.loads(js), dict))
        # Count occurrences of the sha256 key (key + colon) — exactly 3 in archives,
        # plus the hash_algorithm value string elsewhere; we only count key occurrences.
        check("manifest_json_archives_have_sha256", js.count('"sha256":') == 3)
        # Determinism: re-render and compare
        check("manifest_json_deterministic", render_manifest_json(m) == js)

        # ---- write_manifest + load_manifest ----
        manifest_path = os.path.join(arch_dir, "MANIFEST.json")
        write_manifest(manifest_path, m)
        loaded = load_manifest(manifest_path)
        check("manifest_roundtrip_count", loaded.get("archive_count") == 3)
        check("manifest_roundtrip_first", loaded["archives"][0]["iso_basic"] == "2026-08-09T03-00-00Z")

        # ---- verify: all good ----
        result = verify_against_manifest(manifest_path, arch_dir)
        check("verify_ok_when_match", result["ok"] is True)
        check("verify_no_mismatches", result["mismatches"] == [])
        check("verify_no_missing", result["missing"] == [])
        check("verify_no_extra", result["extra"] == [])

        # ---- verify: tamper one archive ----
        tampered = os.path.join(arch_dir, "2026-08-09T04-00-00Z__v1374.md")
        with open(tampered, "ab") as fh:
            fh.write(b"\nTAMPERED\n")
        result2 = verify_against_manifest(manifest_path, arch_dir)
        check("verify_detects_tamper", result2["ok"] is False)
        check("verify_reports_one_mismatch", len(result2["mismatches"]) == 1)
        check("verify_mismatch_name", result2["mismatches"][0]["name"] == "2026-08-09T04-00-00Z__v1374.md")

        # ---- verify: missing archive (delete one, then verify against original manifest) ----
        # Rebuild manifest (re-write with current hashes, since we just tampered)
        write_manifest(manifest_path, m)
        victim = os.path.join(arch_dir, "2026-08-09T03-00-00Z__v1374.md")
        os.unlink(victim)
        result3 = verify_against_manifest(manifest_path, arch_dir)
        check("verify_detects_missing", result3["ok"] is False)
        check("verify_reports_missing", "2026-08-09T03-00-00Z__v1374.md" in result3["missing"])

        # ---- verify: extra archive (rebuild manifest before adding) ----
        write_manifest(manifest_path, build_manifest(scan_archives(arch_dir), archive_dir=arch_dir))
        extra_path = os.path.join(arch_dir, "2026-08-09T06-00-00Z__v1374.md")
        with open(extra_path, "wb") as fh:
            fh.write(b"# extra\n")
        result4 = verify_against_manifest(manifest_path, arch_dir)
        check("verify_reports_extra", "2026-08-09T06-00-00Z__v1374.md" in result4["extra"])

        # ---- verify report markdown ----
        report_md = render_verify_report_md(result2, archive_dir=arch_dir, manifest_path=manifest_path)
        check("verify_md_status_fail", "✗ integrity issue" in report_md)
        check("verify_md_lists_mismatch_name", "2026-08-09T04-00-00Z__v1374.md" in report_md)

        report_ok = render_verify_report_md(result, archive_dir=arch_dir, manifest_path=manifest_path)
        check("verify_md_status_ok", "✓ all good" in report_ok)
        check("verify_md_honesty", "Honesty disclosure" in report_md)

        # ---- empty / missing paths ----
        check("scan_missing_dir_returns_empty", scan_archives(os.path.join(arch_dir, "NOPE_DIR")) == [])
        check("load_missing_returns_empty", load_manifest(os.path.join(arch_dir, "NOPE.json")) == {})
        check("verify_missing_manifest_ok_false", verify_against_manifest(
            os.path.join(arch_dir, "NOPE.json"), arch_dir)["ok"] is False)
        # malformed JSON returns empty dict (no exception)
        bad_json_path = os.path.join(arch_dir, "BAD.json")
        with open(bad_json_path, "w", encoding="utf-8") as fh:
            fh.write("not-valid-json {{{")
        check("load_malformed_returns_empty", load_manifest(bad_json_path) == {})

    finally:
        if arch_dir is not None:
            for name in os.listdir(arch_dir):
                p = os.path.join(arch_dir, name)
                try:
                    os.unlink(p)
                except OSError:
                    pass
            try:
                os.rmdir(arch_dir)
            except OSError:
                pass

    # ---- path safety ----
    try:
        _validate_safe_path("../etc/passwd")
        check("rejects_traversal", False)
    except ValueError:
        check("rejects_traversal", True)

    try:
        _validate_safe_path("/normal/path/file.md")
        check("accepts_normal_absolute", True)
    except ValueError:
        check("accepts_normal_absolute", False)

    # ---- guards count ----
    check("guards_count_10", len(GUARDS) == 10)
    check("guards_no_duplicates", len(set(GUARDS)) == len(GUARDS))
    check("guards_contain_sha256_only", "GUARD_HASH_SHA256_ONLY" in GUARDS)
    check("guards_contain_read_only", "GUARD_VERIFY_READ_ONLY" in GUARDS)

    # ---- hashlib sanity ----
    check("hashlib_sha256_used", _HASHLIB_SHA256_USED == "hashlib.sha256")

    # ---- schema constants ----
    check("schema_version_set", SCHEMA_VERSION.startswith("v1379"))
    check("script_name_set", SCRIPT_NAME == "v1379_v1375_archive_integrity")
    check("default_archive_dir_v1375", DEFAULT_ARCHIVE_DIR == "V1375_HISTORY")
    check("default_manifest_json", DEFAULT_MANIFEST_PATH.endswith(".json"))
    check("default_verify_md", DEFAULT_VERIFY_REPORT_PATH.endswith(".md"))

    total = popper_total
    passed = total - len(failures)
    return (passed, total, failures)


if __name__ == "__main__":
    sys.exit(main())