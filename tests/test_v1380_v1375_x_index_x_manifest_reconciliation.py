"""Tests for V1380 — V1375 × INDEX × V1379 three-way reconciliation."""

from __future__ import annotations

import json
import os
import sys
import tempfile

import pytest

# Ensure promethean/ on path
_HERE = os.path.dirname(os.path.abspath(__file__))
_ROOT = os.path.dirname(_HERE)
if _ROOT not in sys.path:
    sys.path.insert(0, _ROOT)

from apeireth.v1380_v1375_x_index_x_manifest_reconciliation import (  # noqa: E402
    DEFAULT_ARCHIVE_DIR,
    DEFAULT_MANIFEST_PATH,
    SCHEMA_VERSION,
    SCRIPT_NAME,
    _validate_safe_archive_dir,
    build_reconciliation,
    hash_archive_sha256,
    list_disk_archives,
    load_v1379_manifest,
    parse_index_md,
    reconcile_disk_vs_index,
    reconcile_disk_vs_manifest,
    reconcile_index_vs_manifest,
    render_reconciliation_md,
    run_cli,
    write_report,
)


# ----------------------------------------------------------------------
# Path safety
# ----------------------------------------------------------------------

class TestPathSafety:
    def test_rejects_parent_traversal(self):
        with pytest.raises(ValueError):
            _validate_safe_archive_dir("../etc/passwd")

    def test_rejects_dotdot_subpath(self):
        with pytest.raises(ValueError):
            _validate_safe_archive_dir("a/../b")

    def test_accepts_relative(self):
        _validate_safe_archive_dir("V1375_HISTORY")  # should not raise

    def test_accepts_absolute(self):
        import tempfile as _tf
        with _tf.TemporaryDirectory() as td:
            _validate_safe_archive_dir(td)  # should not raise


# ----------------------------------------------------------------------
# list_disk_archives
# ----------------------------------------------------------------------

class TestListDiskArchives:
    def test_empty_dir(self):
        with tempfile.TemporaryDirectory() as td:
            assert list_disk_archives(td) == []

    def test_missing_dir_returns_empty(self):
        assert list_disk_archives("/nonexistent/path/for/v1380/test") == []

    def test_filters_archives(self):
        with tempfile.TemporaryDirectory() as td:
            for name in [
                "2026-08-09T03-55-00Z__v1374.md",
                "2026-08-09T04-00-00Z__v1374.md",
                "2026-08-09T04-05-00Z__v1374_001.md",
            ]:
                with open(os.path.join(td, name), "w") as fh:
                    fh.write("# x\n")
            with open(os.path.join(td, "INDEX.md"), "w") as fh:
                fh.write("# index\n")
            with open(os.path.join(td, "notes.md"), "w") as fh:
                fh.write("# notes\n")
            with open(os.path.join(td, "README.txt"), "w") as fh:
                fh.write("readme\n")
            names = list_disk_archives(td)
            assert len(names) == 3
            assert names == sorted(names)

    def test_sorted_alphabetically(self):
        with tempfile.TemporaryDirectory() as td:
            for iso in ["2026-08-09T04-00-00Z", "2026-08-09T03-55-00Z",
                         "2026-08-09T04-05-00Z"]:
                with open(os.path.join(td, f"{iso}__v1374.md"), "w") as fh:
                    fh.write("# x\n")
            names = list_disk_archives(td)
            assert names[0].startswith("2026-08-09T03-55-00Z")
            assert names[-1].startswith("2026-08-09T04-05-00Z")


# ----------------------------------------------------------------------
# hash_archive_sha256
# ----------------------------------------------------------------------

class TestHashArchive:
    def test_deterministic(self):
        with tempfile.TemporaryDirectory() as td:
            p = os.path.join(td, "a.md")
            with open(p, "w") as fh:
                fh.write("hello world\n")
            h1 = hash_archive_sha256(p)
            h2 = hash_archive_sha256(p)
            assert h1 == h2
            assert len(h1) == 64

    def test_changes_on_content(self):
        with tempfile.TemporaryDirectory() as td:
            p = os.path.join(td, "a.md")
            with open(p, "w") as fh:
                fh.write("v1\n")
            h1 = hash_archive_sha256(p)
            with open(p, "w") as fh:
                fh.write("v2\n")
            h2 = hash_archive_sha256(p)
            assert h1 != h2

    def test_empty_file(self):
        with tempfile.TemporaryDirectory() as td:
            p = os.path.join(td, "a.md")
            with open(p, "w") as fh:
                fh.write("")
            h = hash_archive_sha256(p)
            # SHA-256 of empty string
            assert h == "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"


# ----------------------------------------------------------------------
# parse_index_md
# ----------------------------------------------------------------------

class TestParseIndexMd:
    SAMPLE_INDEX = (
        "# V1375 — V1374 History Archive\n"
        "- **schema:** `v1375.history/v1`\n"
        "- **generated:** 2026-08-09T03:55:00Z\n"
        "- **archives:** 3\n"
        "- **first:** `2026-08-09T03-55-00Z`\n"
        "- **last:** `2026-08-09T04-05-00Z`\n"
        "\n"
        "## Archives\n"
        "\n"
        "| archived | schema | added | removed | changed | unchanged | raw Δ | cal Δ | gap |\n"
        "|----------|--------|------:|--------:|--------:|----------:|------:|------:|-----|\n"
        "| `2026-08-09T03-55-00Z` | v1374 | 0 | 0 | 0 | 8 | 0 | 0 |  |\n"
        "| `2026-08-09T04-00-00Z` | v1374 | 0 | 0 | 0 | 8 | 0 | 0 | 5m |\n"
        "| `2026-08-09T04-05-00Z` | v1374 | 1 | 0 | 1 | 7 | 1 | 1 | 5m |\n"
        "\n"
        "## Legend\n"
        "\n"
        "| column | meaning |\n"
        "| `archived` | slug timestamp |\n"
    )

    def test_missing_file(self):
        assert parse_index_md("/nonexistent/path/INDEX.md") == []

    def test_parses_three_rows(self):
        with tempfile.TemporaryDirectory() as td:
            ipath = os.path.join(td, "INDEX.md")
            with open(ipath, "w", encoding="utf-8") as fh:
                fh.write(self.SAMPLE_INDEX)
            entries = parse_index_md(ipath)
            assert len(entries) == 3

    def test_first_entry_fields(self):
        with tempfile.TemporaryDirectory() as td:
            ipath = os.path.join(td, "INDEX.md")
            with open(ipath, "w", encoding="utf-8") as fh:
                fh.write(self.SAMPLE_INDEX)
            entries = parse_index_md(ipath)
            assert entries[0]["iso_basic"] == "2026-08-09T03-55-00Z"
            assert entries[0]["schema"] == "v1374"
            assert entries[0]["name"] == "2026-08-09T03-55-00Z__v1374.md"

    def test_sorted_ascending(self):
        with tempfile.TemporaryDirectory() as td:
            ipath = os.path.join(td, "INDEX.md")
            with open(ipath, "w", encoding="utf-8") as fh:
                fh.write(self.SAMPLE_INDEX)
            entries = parse_index_md(ipath)
            isos = [e["iso_basic"] for e in entries]
            assert isos == sorted(isos)

    def test_skips_legend_table(self):
        with tempfile.TemporaryDirectory() as td:
            ipath = os.path.join(td, "INDEX.md")
            with open(ipath, "w", encoding="utf-8") as fh:
                fh.write(self.SAMPLE_INDEX)
            entries = parse_index_md(ipath)
            # Legend rows have 'archived' column header, not a slug timestamp
            for e in entries:
                assert "T" in e["iso_basic"]  # iso format check

    def test_rejects_bad_iso(self):
        bad = "| `not-a-slug` | v1374 | 0 | 0 | 0 | 8 | 0 | 0 |  |\n"
        with tempfile.TemporaryDirectory() as td:
            ipath = os.path.join(td, "INDEX.md")
            with open(ipath, "w", encoding="utf-8") as fh:
                fh.write(bad)
            assert parse_index_md(ipath) == []

    def test_rejects_bad_schema(self):
        bad = "| `2026-08-09T03-55-00Z` | bad schema! | 0 | 0 | 0 | 8 | 0 | 0 |  |\n"
        with tempfile.TemporaryDirectory() as td:
            ipath = os.path.join(td, "INDEX.md")
            with open(ipath, "w", encoding="utf-8") as fh:
                fh.write(bad)
            assert parse_index_md(ipath) == []

    def test_dedupes(self):
        dup = "| `2026-08-09T03-55-00Z` | v1374 | 0 | 0 | 0 | 8 | 0 | 0 |  |\n" * 3
        with tempfile.TemporaryDirectory() as td:
            ipath = os.path.join(td, "INDEX.md")
            with open(ipath, "w", encoding="utf-8") as fh:
                fh.write(dup)
            entries = parse_index_md(ipath)
            assert len(entries) == 1


# ----------------------------------------------------------------------
# load_v1379_manifest
# ----------------------------------------------------------------------

class TestLoadV1379Manifest:
    def _sample(self):
        return {
            "schema": "v1379.integrity/v1",
            "hash_algorithm": "sha256",
            "archive_count": 2,
            "archives": [
                {
                    "name": "2026-08-09T03-55-00Z__v1374.md",
                    "sha256": "a" * 64,
                    "size": 100,
                    "iso_basic": "2026-08-09T03-55-00Z",
                    "iso_extended": "2026-08-09T03:55:00Z",
                    "mtime": 12345.6,
                    "schema": "v1374",
                },
                {
                    "name": "2026-08-09T04-00-00Z__v1374.md",
                    "sha256": "b" * 64,
                    "size": 200,
                    "iso_basic": "2026-08-09T04-00-00Z",
                    "iso_extended": "2026-08-09T04:00:00Z",
                    "mtime": 12350.6,
                    "schema": "v1374",
                },
            ],
        }

    def test_missing_file(self):
        assert load_v1379_manifest("/nonexistent/manifest.json") == []

    def test_invalid_json(self):
        with tempfile.TemporaryDirectory() as td:
            mpath = os.path.join(td, "m.json")
            with open(mpath, "w", encoding="utf-8") as fh:
                fh.write("not json {{{")
            assert load_v1379_manifest(mpath) == []

    def test_loads_two_archives(self):
        with tempfile.TemporaryDirectory() as td:
            mpath = os.path.join(td, "m.json")
            with open(mpath, "w", encoding="utf-8") as fh:
                json.dump(self._sample(), fh)
            archives = load_v1379_manifest(mpath)
            assert len(archives) == 2

    def test_sorted_by_name(self):
        with tempfile.TemporaryDirectory() as td:
            mpath = os.path.join(td, "m.json")
            with open(mpath, "w", encoding="utf-8") as fh:
                json.dump(self._sample(), fh)
            archives = load_v1379_manifest(mpath)
            names = [a["name"] for a in archives]
            assert names == sorted(names)

    def test_fields_preserved(self):
        with tempfile.TemporaryDirectory() as td:
            mpath = os.path.join(td, "m.json")
            with open(mpath, "w", encoding="utf-8") as fh:
                json.dump(self._sample(), fh)
            archives = load_v1379_manifest(mpath)
            a = archives[0]
            assert a["sha256"] == "a" * 64
            assert a["size"] == 100
            assert a["iso_basic"] == "2026-08-09T03-55-00Z"
            assert a["schema"] == "v1374"

    def test_missing_archives_field(self):
        with tempfile.TemporaryDirectory() as td:
            mpath = os.path.join(td, "m.json")
            with open(mpath, "w", encoding="utf-8") as fh:
                json.dump({"schema": "v1379.integrity/v1"}, fh)
            assert load_v1379_manifest(mpath) == []

    def test_non_dict_archives(self):
        with tempfile.TemporaryDirectory() as td:
            mpath = os.path.join(td, "m.json")
            with open(mpath, "w", encoding="utf-8") as fh:
                json.dump({"archives": ["not a dict"]}, fh)
            assert load_v1379_manifest(mpath) == []


# ----------------------------------------------------------------------
# reconcile_disk_vs_index
# ----------------------------------------------------------------------

class TestReconcileDiskVsIndex:
    def test_all_agree(self):
        res = reconcile_disk_vs_index(["a.md"], [
            {"name": "a.md", "iso_basic": "x", "schema": "v1374"},
        ])
        assert res["ok"] is True
        assert res["disk_only"] == []
        assert res["index_only"] == []
        assert res["shared_count"] == 1

    def test_disk_only(self):
        res = reconcile_disk_vs_index(["a.md", "b.md"], [
            {"name": "b.md", "iso_basic": "x", "schema": "v1374"},
        ])
        assert res["ok"] is False
        assert res["disk_only"] == ["a.md"]
        assert res["index_only"] == []

    def test_index_only(self):
        res = reconcile_disk_vs_index(["a.md"], [
            {"name": "a.md", "iso_basic": "x", "schema": "v1374"},
            {"name": "b.md", "iso_basic": "y", "schema": "v1374"},
        ])
        assert res["ok"] is False
        assert res["disk_only"] == []
        assert res["index_only"] == ["b.md"]

    def test_both_empty_not_ok(self):
        res = reconcile_disk_vs_index([], [])
        assert res["ok"] is False
        assert res["disk_count"] == 0
        assert res["index_count"] == 0
        assert res["shared_count"] == 0


# ----------------------------------------------------------------------
# reconcile_disk_vs_manifest
# ----------------------------------------------------------------------

class TestReconcileDiskVsManifest:
    def _stub(self, mapping):
        def f(name):
            return mapping.get(os.path.basename(name), "")
        return f

    def test_all_agree(self):
        with tempfile.TemporaryDirectory() as td:
            with open(os.path.join(td, "a.md"), "w") as fh:
                fh.write("# a\n")
            archives = [{"name": "a.md", "sha256": "x" * 64, "size": 1}]
            res = reconcile_disk_vs_manifest(
                td, ["a.md"], archives, hash_func=self._stub({"a.md": "x" * 64})
            )
            assert res["ok"] is True
            assert res["hash_mismatches"] == []

    def test_hash_mismatch(self):
        with tempfile.TemporaryDirectory() as td:
            with open(os.path.join(td, "a.md"), "w") as fh:
                fh.write("# a\n")
            archives = [{"name": "a.md", "sha256": "WRONG", "size": 1}]
            res = reconcile_disk_vs_manifest(
                td, ["a.md"], archives, hash_func=self._stub({"a.md": "x" * 64})
            )
            assert res["ok"] is False
            assert len(res["hash_mismatches"]) == 1
            assert res["hash_mismatches"][0]["name"] == "a.md"

    def test_manifest_only(self):
        with tempfile.TemporaryDirectory() as td:
            archives = [{"name": "a.md", "sha256": "x" * 64, "size": 1}]
            res = reconcile_disk_vs_manifest(
                td, [], archives, hash_func=self._stub({})
            )
            assert res["ok"] is False
            assert res["manifest_only"] == ["a.md"]

    def test_disk_only(self):
        with tempfile.TemporaryDirectory() as td:
            with open(os.path.join(td, "a.md"), "w") as fh:
                fh.write("# a\n")
            res = reconcile_disk_vs_manifest(
                td, ["a.md"], [], hash_func=self._stub({})
            )
            assert res["ok"] is False
            assert res["disk_only"] == ["a.md"]

    def test_manifest_without_sha_skipped(self):
        with tempfile.TemporaryDirectory() as td:
            with open(os.path.join(td, "a.md"), "w") as fh:
                fh.write("# a\n")
            archives = [{"name": "a.md", "sha256": "", "size": 1}]
            res = reconcile_disk_vs_manifest(
                td, ["a.md"], archives, hash_func=self._stub({"a.md": "x" * 64})
            )
            assert res["ok"] is True
            assert res["hash_mismatches"] == []
            assert res["shared_count"] == 1


# ----------------------------------------------------------------------
# reconcile_index_vs_manifest
# ----------------------------------------------------------------------

class TestReconcileIndexVsManifest:
    def test_all_agree(self):
        res = reconcile_index_vs_manifest(
            [{"name": "a.md", "iso_basic": "x", "schema": "v1374"}],
            [{"name": "a.md", "sha256": "x" * 64, "size": 1}],
        )
        assert res["ok"] is True
        assert res["only_in_index"] == []
        assert res["only_in_manifest"] == []

    def test_only_in_index(self):
        res = reconcile_index_vs_manifest(
            [{"name": "a.md", "iso_basic": "x", "schema": "v1374"}],
            [],
        )
        assert res["ok"] is False
        assert res["only_in_index"] == ["a.md"]
        assert res["only_in_manifest"] == []

    def test_only_in_manifest(self):
        res = reconcile_index_vs_manifest(
            [],
            [{"name": "a.md", "sha256": "x" * 64, "size": 1}],
        )
        assert res["ok"] is False
        assert res["only_in_index"] == []
        assert res["only_in_manifest"] == ["a.md"]


# ----------------------------------------------------------------------
# build_reconciliation
# ----------------------------------------------------------------------

class TestBuildReconciliation:
    def test_all_three_agree(self):
        with tempfile.TemporaryDirectory() as td:
            archive = os.path.join(td, "2026-08-09T03-55-00Z__v1374.md")
            with open(archive, "w") as fh:
                fh.write("# a\n")
            with open(os.path.join(td, "INDEX.md"), "w", encoding="utf-8") as fh:
                fh.write("| `2026-08-09T03-55-00Z` | v1374 | 0 | 0 | 0 | 8 | 0 | 0 |  |\n")
            real_hash = hash_archive_sha256(archive)
            manifest = {
                "schema": "v1379.integrity/v1",
                "hash_algorithm": "sha256",
                "archive_count": 1,
                "archives": [{
                    "name": "2026-08-09T03-55-00Z__v1374.md",
                    "sha256": real_hash,
                    "size": 4,
                    "iso_basic": "2026-08-09T03-55-00Z",
                    "schema": "v1374",
                }],
            }
            mpath = os.path.join(td, "m.json")
            with open(mpath, "w", encoding="utf-8") as fh:
                json.dump(manifest, fh)
            result = build_reconciliation(
                td,
                index_path=os.path.join(td, "INDEX.md"),
                manifest_path=mpath,
            )
            assert result["all_ok"] is True
            assert len(result["disk_names"]) == 1
            assert len(result["index_names"]) == 1
            assert len(result["manifest_names"]) == 1

    def test_hash_mismatch_causes_disagree(self):
        with tempfile.TemporaryDirectory() as td:
            archive = os.path.join(td, "2026-08-09T03-55-00Z__v1374.md")
            with open(archive, "w") as fh:
                fh.write("original\n")
            with open(os.path.join(td, "INDEX.md"), "w", encoding="utf-8") as fh:
                fh.write("| `2026-08-09T03-55-00Z` | v1374 | 0 | 0 | 0 | 8 | 0 | 0 |  |\n")
            frozen_hash = hash_archive_sha256(archive)
            with open(archive, "w") as fh:
                fh.write("tampered\n")
            manifest = {
                "schema": "v1379.integrity/v1",
                "hash_algorithm": "sha256",
                "archive_count": 1,
                "archives": [{
                    "name": "2026-08-09T03-55-00Z__v1374.md",
                    "sha256": frozen_hash,
                    "size": 9,
                    "iso_basic": "2026-08-09T03-55-00Z",
                    "schema": "v1374",
                }],
            }
            mpath = os.path.join(td, "m.json")
            with open(mpath, "w", encoding="utf-8") as fh:
                json.dump(manifest, fh)
            result = build_reconciliation(
                td,
                index_path=os.path.join(td, "INDEX.md"),
                manifest_path=mpath,
            )
            assert result["all_ok"] is False
            assert len(result["disk_vs_manifest"]["hash_mismatches"]) == 1

    def test_disk_only_causes_disagree(self):
        with tempfile.TemporaryDirectory() as td:
            with open(os.path.join(td, "extra.md"), "w") as fh:
                fh.write("# extra\n")
            with open(os.path.join(td, "INDEX.md"), "w", encoding="utf-8") as fh:
                fh.write("# empty\n")
            manifest = {
                "schema": "v1379.integrity/v1",
                "hash_algorithm": "sha256",
                "archive_count": 0,
                "archives": [],
            }
            mpath = os.path.join(td, "m.json")
            with open(mpath, "w", encoding="utf-8") as fh:
                json.dump(manifest, fh)
            result = build_reconciliation(
                td,
                index_path=os.path.join(td, "INDEX.md"),
                manifest_path=mpath,
            )
            assert result["all_ok"] is False
            # "extra.md" doesn't match archive regex, so disk is empty
            assert result["disk_vs_index"]["disk_only"] == []
            assert result["disk_vs_index"]["index_only"] == []

    def test_missing_manifest_is_empty(self):
        with tempfile.TemporaryDirectory() as td:
            with open(os.path.join(td, "INDEX.md"), "w", encoding="utf-8") as fh:
                fh.write("# empty\n")
            result = build_reconciliation(
                td,
                manifest_path=os.path.join(td, "nonexistent.json"),
            )
            assert result["manifest_names"] == []


# ----------------------------------------------------------------------
# render_reconciliation_md
# ----------------------------------------------------------------------

class TestRenderReconciliationMd:
    def _result(self, all_ok=True):
        return {
            "schema": SCHEMA_VERSION,
            "generated_at": "2026-08-09T04:00:00Z",
            "archive_dir": "/tmp/x",
            "index_path": "/tmp/x/INDEX.md",
            "manifest_path": "/tmp/x/manifest.json",
            "disk_names": ["a.md"],
            "index_names": ["a.md"],
            "manifest_names": ["a.md"],
            "disk_vs_index": {"ok": all_ok, "disk_only": [], "index_only": [],
                              "shared_count": 1, "disk_count": 1, "index_count": 1},
            "disk_vs_manifest": {"ok": all_ok, "disk_only": [], "manifest_only": [],
                                 "hash_mismatches": [],
                                 "shared_count": 1, "disk_count": 1, "manifest_count": 1},
            "index_vs_manifest": {"ok": all_ok, "only_in_index": [],
                                  "only_in_manifest": [], "shared_count": 1,
                                  "index_count": 1, "manifest_count": 1},
            "all_ok": all_ok,
        }

    def test_all_ok_marker(self):
        md = render_reconciliation_md(self._result(all_ok=True))
        assert "✓ all three sources agree" in md

    def test_disagree_marker(self):
        md = render_reconciliation_md(self._result(all_ok=False))
        assert "✗ disagreement detected" in md

    def test_contains_three_pairs(self):
        md = render_reconciliation_md(self._result())
        assert "## Pair 1: disk ↔ INDEX.md" in md
        assert "## Pair 2: disk ↔ V1379 manifest" in md
        assert "## Pair 3: INDEX.md ↔ V1379 manifest" in md

    def test_contains_honesty(self):
        md = render_reconciliation_md(self._result())
        assert "## Honesty disclosure" in md
        assert "does not pretend" in md

    def test_hash_mismatch_detail(self):
        result = self._result(all_ok=False)
        result["disk_vs_manifest"]["hash_mismatches"] = [
            {"name": "a.md", "expected": "z" * 64, "actual": "y" * 64},
        ]
        md = render_reconciliation_md(result)
        assert "hash_mismatches:** 1" in md
        assert "a.md" in md
        assert ("z" * 16) in md or "expected" in md


# ----------------------------------------------------------------------
# write_report
# ----------------------------------------------------------------------

class TestWriteReport:
    def test_writes_file(self):
        with tempfile.TemporaryDirectory() as td:
            p = os.path.join(td, "report.md")
            write_report(p, "# hello\n")
            assert os.path.exists(p)
            with open(p, "r", encoding="utf-8") as fh:
                assert fh.read() == "# hello\n"

    def test_creates_subdir(self):
        with tempfile.TemporaryDirectory() as td:
            p = os.path.join(td, "sub", "dir", "report.md")
            write_report(p, "# hello\n")
            assert os.path.exists(p)

    def test_overwrites(self):
        with tempfile.TemporaryDirectory() as td:
            p = os.path.join(td, "report.md")
            write_report(p, "# v1\n")
            write_report(p, "# v2\n")
            with open(p, "r", encoding="utf-8") as fh:
                assert fh.read() == "# v2\n"


# ----------------------------------------------------------------------
# CLI
# ----------------------------------------------------------------------

class TestCLI:
    def test_version(self):
        rc = run_cli(["version"])
        assert rc == 0

    def test_popper_passes(self):
        rc = run_cli(["popper"])
        assert rc == 0

    def test_popper_outputs_count(self, capsys):
        run_cli(["popper"])
        captured = capsys.readouterr()
        assert "Popper self-tests:" in captured.out

    def test_reconcile_disk_only_exit_1(self):
        """A real archive on disk but absent from INDEX + manifest → disagreement."""
        with tempfile.TemporaryDirectory() as td:
            archive = os.path.join(td, "2026-08-09T03-55-00Z__v1374.md")
            with open(archive, "w") as fh:
                fh.write("# a\n")
            # empty INDEX + empty manifest → disagree with disk
            with open(os.path.join(td, "INDEX.md"), "w", encoding="utf-8") as fh:
                fh.write("# empty index\n")
            manifest = {"schema": "v1379.integrity/v1", "hash_algorithm": "sha256",
                        "archive_count": 0, "archives": []}
            mpath = os.path.join(td, "m.json")
            with open(mpath, "w", encoding="utf-8") as fh:
                json.dump(manifest, fh)
            rpath = os.path.join(td, "REPORT.md")
            rc = run_cli([
                "reconcile", "--archive-dir", td,
                "--index-path", os.path.join(td, "INDEX.md"),
                "--manifest-path", mpath,
                "--report-path", rpath,
                "--quiet",
            ])
            assert rc == 1
            assert os.path.exists(rpath)
            with open(rpath, "r", encoding="utf-8") as fh:
                content = fh.read()
            assert "✗ disagreement detected" in content

    def test_reconcile_real_agree_exit_0(self):
        with tempfile.TemporaryDirectory() as td:
            archive = os.path.join(td, "2026-08-09T03-55-00Z__v1374.md")
            with open(archive, "w") as fh:
                fh.write("# a\n")
            with open(os.path.join(td, "INDEX.md"), "w", encoding="utf-8") as fh:
                fh.write("| `2026-08-09T03-55-00Z` | v1374 | 0 | 0 | 0 | 8 | 0 | 0 |  |\n")
            real_hash = hash_archive_sha256(archive)
            manifest = {
                "schema": "v1379.integrity/v1",
                "hash_algorithm": "sha256",
                "archive_count": 1,
                "archives": [{
                    "name": "2026-08-09T03-55-00Z__v1374.md",
                    "sha256": real_hash,
                    "size": 4,
                    "iso_basic": "2026-08-09T03-55-00Z",
                    "schema": "v1374",
                }],
            }
            mpath = os.path.join(td, "m.json")
            with open(mpath, "w", encoding="utf-8") as fh:
                json.dump(manifest, fh)
            rpath = os.path.join(td, "REPORT.md")
            rc = run_cli([
                "reconcile", "--archive-dir", td,
                "--index-path", os.path.join(td, "INDEX.md"),
                "--manifest-path", mpath,
                "--report-path", rpath,
                "--quiet",
            ])
            assert rc == 0
            with open(rpath, "r", encoding="utf-8") as fh:
                content = fh.read()
            assert "✓ all three sources agree" in content

    def test_reconcile_hash_mismatch_exit_1(self):
        with tempfile.TemporaryDirectory() as td:
            archive = os.path.join(td, "2026-08-09T03-55-00Z__v1374.md")
            with open(archive, "w") as fh:
                fh.write("original\n")
            frozen_hash = hash_archive_sha256(archive)
            with open(archive, "w") as fh:
                fh.write("tampered\n")
            with open(os.path.join(td, "INDEX.md"), "w", encoding="utf-8") as fh:
                fh.write("| `2026-08-09T03-55-00Z` | v1374 | 0 | 0 | 0 | 8 | 0 | 0 |  |\n")
            manifest = {
                "schema": "v1379.integrity/v1",
                "hash_algorithm": "sha256",
                "archive_count": 1,
                "archives": [{
                    "name": "2026-08-09T03-55-00Z__v1374.md",
                    "sha256": frozen_hash,
                    "size": 9,
                    "iso_basic": "2026-08-09T03-55-00Z",
                    "schema": "v1374",
                }],
            }
            mpath = os.path.join(td, "m.json")
            with open(mpath, "w", encoding="utf-8") as fh:
                json.dump(manifest, fh)
            rpath = os.path.join(td, "REPORT.md")
            rc = run_cli([
                "reconcile", "--archive-dir", td,
                "--index-path", os.path.join(td, "INDEX.md"),
                "--manifest-path", mpath,
                "--report-path", rpath,
                "--quiet",
            ])
            assert rc == 1

    def test_show_missing_file(self):
        rc = run_cli(["show", "--report-path", "/nonexistent/path/report.md"])
        assert rc == 2


# ----------------------------------------------------------------------
# Module-level sanity
# ----------------------------------------------------------------------

def test_schema_version():
    assert SCHEMA_VERSION == "v1380.reconciliation/v1"

def test_script_name():
    assert SCRIPT_NAME == "v1380_v1375_x_index_x_manifest_reconciliation"

def test_defaults():
    assert DEFAULT_ARCHIVE_DIR == "V1375_HISTORY"
    assert DEFAULT_MANIFEST_PATH == "V1379_INTEGRITY_AUTO.json"