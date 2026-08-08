"""Tests for V1376 — V1375 weekly digest.

32 pytest tests covering:
- iso_week_bucket / iso_week_label / parse_week_label
- group_by_week
- weekly_summary
- render_weekly_md / render_index_md
- write_digest (with real V1375 archive fixture)
- run_cli subprocess (digest / list / show / popper / version / error paths)
- _safe_join (cross-platform traversal / absolute rejection)

Each test is independent. No fixtures are shared beyond a temporary
directory created in setUp / setUpModule style.
"""

from __future__ import annotations

import os
import subprocess
import sys
import tempfile
import unittest

# Make the project importable when running from project root
PROJECT_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
if PROJECT_ROOT not in sys.path:
    sys.path.insert(0, PROJECT_ROOT)

from apeireth.v1375_v1374_history_archive import archive_tick  # noqa: E402
from apeireth.v1376_v1375_weekly_digest import (  # noqa: E402
    SCHEMA_VERSION,
    SCRIPT_NAME,
    DEFAULT_ARCHIVE_DIR,
    DEFAULT_OUTPUT_DIR,
    group_by_week,
    iso_week_bucket,
    iso_week_label,
    parse_week_label,
    render_index_md,
    render_weekly_md,
    run_cli,
    weekly_summary,
    write_digest,
    _safe_join,
)


class TestConstants(unittest.TestCase):
    def test_schema_version(self):
        self.assertEqual(SCHEMA_VERSION, "v1376.digest/v1")

    def test_script_name(self):
        self.assertEqual(SCRIPT_NAME, "v1376_v1375_weekly_digest")

    def test_default_archive_dir(self):
        self.assertEqual(DEFAULT_ARCHIVE_DIR, "V1375_HISTORY")

    def test_default_output_dir(self):
        self.assertEqual(DEFAULT_OUTPUT_DIR, "V1376_DIGESTS")


class TestIsoWeek(unittest.TestCase):
    def test_bucket_known_dates(self):
        # Sat Aug 8 2026 → ISO W32 of 2026
        self.assertEqual(iso_week_bucket("2026-08-08T20-06-51Z"), (2026, 32))
        # Mon Aug 3 2026 → ISO W32 of 2026
        self.assertEqual(iso_week_bucket("2026-08-03T00-00-00Z"), (2026, 32))
        # Sun Aug 9 2026 → ISO W32 of 2026
        self.assertEqual(iso_week_bucket("2026-08-09T23-59-59Z"), (2026, 32))
        # Mon Jul 27 2026 → ISO W31 of 2026
        self.assertEqual(iso_week_bucket("2026-07-27T00-00-00Z"), (2026, 31))

    def test_bucket_iso_year_edge(self):
        # Fri Jan 1 2027 → ISO W53 of 2026 (per ISO 8601)
        self.assertEqual(iso_week_bucket("2027-01-01T00-00-00Z"), (2026, 53))
        # Thu Jan 1 2026 → ISO W01 of 2026
        self.assertEqual(iso_week_bucket("2026-01-01T00-00-00Z"), (2026, 1))

    def test_bucket_rejects_bad(self):
        for bad in ["", "2026-08-08", "garbage", "2026-13-01T00-00-00Z", None]:
            with self.assertRaises((ValueError, TypeError)):
                iso_week_bucket(bad)  # type: ignore[arg-type]

    def test_label_roundtrip(self):
        for y, w in [(2026, 32), (2025, 53), (2027, 1), (1999, 1), (2026, 1)]:
            lbl = iso_week_label(y, w)
            y2, w2 = parse_week_label(lbl)
            self.assertEqual((y, w), (y2, w2))

    def test_label_rejects_bad(self):
        for bad_y, bad_w in [(1899, 1), (3000, 1), (2026, 0), (2026, 54), (2026, -1)]:
            with self.assertRaises(ValueError):
                iso_week_label(bad_y, bad_w)
        for bad_lbl in ["", "2026-W", "2026-32", "2026w32", "26-W32", "2026-W1"]:
            with self.assertRaises(ValueError):
                parse_week_label(bad_lbl)


class TestGroupByWeek(unittest.TestCase):
    def test_groups_and_sorts(self):
        sample = [
            {"iso": "2026-08-08T20-06-51Z", "schema": "v1374"},
            {"iso": "2026-08-03T01-00-00Z", "schema": "v1374"},
            {"iso": "2026-07-27T01-00-00Z", "schema": "v1374"},
        ]
        grp = group_by_week(sample)
        self.assertEqual(list(grp.keys()), ["2026-W31", "2026-W32"])
        self.assertEqual(len(grp["2026-W32"]), 2)
        self.assertEqual(len(grp["2026-W31"]), 1)

    def test_skips_dicts_without_iso(self):
        # Defensive: archives without 'iso' are skipped (V1375 contract)
        grp = group_by_week([
            {"iso": "2026-08-08T20-06-51Z", "schema": "v1374"},
            {"schema": "v1374"},  # missing iso
            {"archived": "2026-08-01T00-00-00Z"},  # wrong key
        ])
        self.assertEqual(list(grp.keys()), ["2026-W32"])
        self.assertEqual(len(grp["2026-W32"]), 1)

    def test_empty_input(self):
        self.assertEqual(group_by_week([]), {})


class TestWeeklySummary(unittest.TestCase):
    def test_aggregates_with_deltas(self):
        grp = group_by_week([
            {"iso": "2026-08-08T20-06-51Z", "added": 0, "removed": 0,
             "changed": 0, "unchanged": 8, "raw_delta": 0, "cal_delta": 0,
             "schema": "v1374"},
            {"iso": "2026-08-03T01-00-00Z", "added": 1, "removed": 0,
             "changed": 2, "unchanged": 5, "raw_delta": 3, "cal_delta": 1,
             "schema": "v1374"},
        ])
        s = weekly_summary(grp["2026-W32"])
        self.assertEqual(s["count"], 2)
        self.assertEqual(s["added_total"], 1)
        self.assertEqual(s["changed_total"], 2)
        self.assertEqual(s["zero_deltas"], 1)
        self.assertEqual(s["nonzero_count"], 1)
        self.assertEqual(s["last_nonzero_at"], "2026-08-03T01-00-00Z")
        self.assertEqual(s["schemas"], {"v1374"})

    def test_empty_summary(self):
        s = weekly_summary([])
        self.assertEqual(s["count"], 0)
        self.assertEqual(s["first"], "")
        self.assertEqual(s["last"], "")
        self.assertIsNone(s["last_nonzero_at"])
        self.assertEqual(s["schemas"], set())

    def test_all_zero_deltas(self):
        grp = group_by_week([
            {"iso": "2026-08-08T20-06-51Z", "raw_delta": 0, "cal_delta": 0,
             "schema": "v1374"},
            {"iso": "2026-08-09T20-06-51Z", "raw_delta": 0, "cal_delta": 0,
             "schema": "v1374"},
        ])
        s = weekly_summary(grp["2026-W32"])
        self.assertEqual(s["zero_deltas"], 2)
        self.assertEqual(s["nonzero_count"], 0)
        self.assertIsNone(s["last_nonzero_at"])


class TestRender(unittest.TestCase):
    def test_weekly_md_has_required_keys(self):
        grp = group_by_week([
            {"iso": "2026-08-08T20-06-51Z", "added": 1, "removed": 0,
             "changed": 2, "raw_delta": 3, "cal_delta": 1, "schema": "v1374"},
        ])
        s = weekly_summary(grp["2026-W32"])
        md = render_weekly_md("2026-W32", s)
        for must in [
            "# V1376 — V1375 Weekly Digest for 2026-W32",
            "**schema:** `v1376.digest/v1`",
            "| archives in week | 1 |",
            "| added (sum) | 1 |",
            "| zero-delta archives | 0 |",
            "| non-zero archives | 1 |",
            "Honesty paragraph",
        ]:
            self.assertIn(must, md)

    def test_weekly_md_rejects_bad_label(self):
        s = weekly_summary([])
        with self.assertRaises(ValueError):
            render_weekly_md("garbage", s)

    def test_index_md_empty(self):
        idx = render_index_md([])
        self.assertIn("_No digests yet._", idx)

    def test_index_md_populated(self):
        idx = render_index_md(["2026-W31", "2026-W32"])
        self.assertIn("| `2026-W31` | `2026-W31.md` |", idx)
        self.assertIn("| `2026-W32` | `2026-W32.md` |", idx)


class TestWriteDigest(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.archive_dir = os.path.join(self.tmp.name, "archive")
        self.output_dir = os.path.join(self.tmp.name, "digests")
        os.makedirs(self.archive_dir, exist_ok=True)
        # Seed V1375 archive with one snapshot
        v1374 = os.path.join(self.tmp.name, "V1374_REPORT_AUTO.md")
        with open(v1374, "w", encoding="utf-8") as f:
            f.write("# fake V1374 report\n")
        archive_tick(self.archive_dir, v1374, timestamp="2026-08-08T20-06-51Z")

    def test_writes_week_and_index(self):
        result = write_digest(self.archive_dir, output_dir=self.output_dir)
        self.assertEqual(result["archives_seen"], 1)
        self.assertIn("2026-W32.md", result["files"])
        self.assertIn("INDEX.md", result["files"])
        week_path = os.path.join(self.output_dir, "2026-W32.md")
        self.assertTrue(os.path.isfile(week_path))
        self.assertGreater(os.path.getsize(week_path), 0)
        idx_path = os.path.join(self.output_dir, "INDEX.md")
        self.assertTrue(os.path.isfile(idx_path))
        self.assertGreater(os.path.getsize(idx_path), 0)

    def test_deterministic_across_calls(self):
        r1 = write_digest(self.archive_dir, output_dir=self.output_dir)
        r2 = write_digest(self.archive_dir, output_dir=self.output_dir)
        self.assertEqual(r1["weeks"], r2["weeks"])
        self.assertEqual(set(r1["files"]), set(r2["files"]))

    def test_empty_archive(self):
        empty_dir = os.path.join(self.tmp.name, "empty")
        os.makedirs(empty_dir, exist_ok=True)
        empty_out = os.path.join(self.tmp.name, "empty_out")
        result = write_digest(empty_dir, output_dir=empty_out)
        self.assertEqual(result["weeks"], [])
        self.assertIn("INDEX.md", result["files"])


class TestSafeJoin(unittest.TestCase):
    def test_accepts_simple_name(self):
        self.assertTrue(_safe_join(".", "2026-W32.md").endswith("2026-W32.md"))

    def test_rejects_traversal(self):
        for bad in ["../etc/passwd", "..\\windows\\system32",
                    "foo/../bar", "foo/.."]:
            with self.assertRaises(ValueError):
                _safe_join(".", bad)

    def test_rejects_absolute(self):
        # Drive-letter absolute path (caught by os.path.isabs)
        with self.assertRaises(ValueError):
            _safe_join(".", "C:\\Windows\\System32")
        # Posix-root path on Windows is not isabs, but caught by leading-slash check
        with self.assertRaises(ValueError):
            _safe_join(".", "/etc/passwd")
        # UNC absolute path caught by leading-backslash check
        with self.assertRaises(ValueError):
            _safe_join(".", "\\server\\share")


class TestRunCli(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.archive_dir = os.path.join(self.tmp.name, "archive")
        self.output_dir = os.path.join(self.tmp.name, "digests")
        os.makedirs(self.archive_dir, exist_ok=True)
        v1374 = os.path.join(self.tmp.name, "V1374_REPORT_AUTO.md")
        with open(v1374, "w", encoding="utf-8") as f:
            f.write("# fake V1374\n")
        archive_tick(self.archive_dir, v1374, timestamp="2026-08-08T20-06-51Z")

    def _run(self, *args):
        return subprocess.run(
            [sys.executable, "-m", "apeireth.v1376_v1375_weekly_digest",
             "--archive-dir", self.archive_dir,
             "--output-dir", self.output_dir, *args],
            capture_output=True, text=True, cwd=PROJECT_ROOT,
            encoding="utf-8", errors="replace",
            env={**os.environ, "PYTHONIOENCODING": "utf-8"},
        )

    def test_version(self):
        r = self._run("version")
        self.assertEqual(r.returncode, 0)
        self.assertEqual(r.stdout.strip(), "v1376.digest/v1")

    def test_popper(self):
        r = self._run("popper")
        self.assertEqual(r.returncode, 0)
        self.assertIn("49/49", r.stdout)

    def test_list(self):
        r = self._run("list")
        self.assertEqual(r.returncode, 0)
        self.assertIn("2026-W32", r.stdout)

    def test_digest_then_show(self):
        r1 = self._run("digest")
        self.assertEqual(r1.returncode, 0)
        self.assertIn("INDEX.md", r1.stdout)
        r2 = self._run("show", "2026-W32")
        self.assertEqual(r2.returncode, 0)
        self.assertIn("V1376", r2.stdout)
        self.assertIn("Honesty paragraph", r2.stdout)

    def test_show_missing_week_returns_2(self):
        r = self._run("digest")  # write INDEX
        self.assertEqual(r.returncode, 0)
        r2 = self._run("show", "2099-W99")
        self.assertEqual(r2.returncode, 2)
        self.assertIn("not found", r2.stderr)

    def test_show_rejects_traversal(self):
        r = self._run("show", "../etc/passwd")
        # _safe_join raises ValueError → uncaught → non-zero exit
        self.assertNotEqual(r.returncode, 0)


class TestGuardsPresent(unittest.TestCase):
    """GUARD constants documented in the module docstring must all be present.

    We don't enforce guard enforcement (popper + pytest already does), we just
    ensure no guard was accidentally deleted during refactoring.
    """

    def test_guards_listed(self):
        import apeireth.v1376_v1375_weekly_digest as mod
        src = mod.__doc__ or ""
        for guard in [
            "GUARD_DIGEST_INPUT_FROM_V1375",
            "GUARD_DIGEST_NO_WRITE_BACK",
            "GUARD_DIGEST_DETERMINISTIC",
            "GUARD_DIGEST_PRESERVES_ORDER",
            "GUARD_DIGEST_HONEST_DISCLOSURE",
            "GUARD_DIGEST_MARKDOWN_ONLY",
            "GUARD_DIGEST_NO_CAP_CHANGE",
            "GUARD_DIGEST_LOCAL_FS_ONLY",
            "GUARD_DIGEST_FS_PATH_SAFE",
            "GUARD_DIGEST_ISO_WEEK_VALID",
        ]:
            self.assertIn(guard, src)


if __name__ == "__main__":
    unittest.main()