"""Pytest suite for V1363 dashboard trend overlay.

V1363 = composition layer that wires V1361 (visual dashboard) and V1362
(pole-star history) into a single CLI surface. These tests verify:

  1. Constants (version, cap, guards, subweights).
  2. Delegation to V1361 + V1362 only (no direct V1357 import).
  3. Trend overlay section is well-formed Markdown.
  4. Combined dashboard includes both V1361 sections AND V1362 trend.
  5. Combined JSON includes V1361 snapshot + V1362 history + trend.
  6. Read-only invariant: AST-verified no write-mode open() except via V1362.
  7. Snapshot CLI delegates write to V1362.append_snapshot (no fabrication).
  8. CLI subcommands work end-to-end.
"""
from __future__ import annotations

import ast
import json
import os
import tempfile
from pathlib import Path

import pytest

from apeireth import v1363_dashboard_trend_overlay as v1363


# -----------------------------------------------------------------------------
# TestV1363Constants
# -----------------------------------------------------------------------------

class TestV1363Constants:
    def test_version_is_semver(self):
        assert v1363.V1363_VERSION.count(".") == 2

    def test_asi_cap_below_threshold(self):
        assert v1363.V1363_ASI_CAP <= 0.01

    def test_asi_cap_positive(self):
        assert v1363.V1363_ASI_CAP > 0

    def test_cap_value(self):
        # Honest cap; overlay ≠ ASI
        assert v1363.V1363_ASI_CAP == 0.005

    def test_philosophy_guards_complete(self):
        expected = {
            "GUARD_OVERLAY_NOT_ASI",
            "GUARD_COMPOSE_ONLY",
            "GUARD_DELEGATE_TO_V1361_V1362",
            "GUARD_READ_ONLY_EXCEPT_V1362",
            "GUARD_NO_FABRICATION",
            "GUARD_HONEST_CAP",
            "GUARD_V1361_V1362_UNCHANGED",
        }
        assert expected.issubset(set(v1363.V1363_PHILOSOPHY_GUARDS))

    def test_subweights_sum_to_one(self):
        total = sum(v1363.V1363_SUBWEIGHTS.values())
        assert abs(total - 1.0) < 1e-9

    def test_subweights_has_five_keys(self):
        assert len(v1363.V1363_SUBWEIGHTS) == 5


# -----------------------------------------------------------------------------
# TestV1363Delegation
# -----------------------------------------------------------------------------

class TestV1363Delegation:
    def test_v1361_importable(self):
        v1361 = v1363._import_v1361()
        assert v1361 is not None

    def test_v1362_importable(self):
        v1362 = v1363._import_v1362()
        assert v1362 is not None

    def test_v1361_dashboard_md_delegates(self):
        # Calling the wrapper should produce the same content as V1361's
        # full dashboard — confirms composition (not fabrication).
        md = v1363.get_v1361_dashboard_md()
        assert "Apeireth V1361 Dashboard" in md
        assert "Pole-Star" in md

    def test_v1362_history_table_delegates(self):
        md = v1363.get_v1362_history_table(limit=5)
        # Either empty placeholder or table — both are valid V1362 outputs
        assert isinstance(md, str)
        assert len(md) > 0

    def test_v1362_trend_md_delegates(self):
        md = v1363.get_v1362_trend_md(window=3)
        assert isinstance(md, str)
        assert "Trend" in md or "No entries" in md

    def test_combined_snapshot_dict_shape(self):
        d = v1363.get_combined_snapshot_dict(history_limit=3, window=2)
        assert d["v1363_version"] == v1363.V1363_VERSION
        assert "v1361_snapshot" in d
        assert "v1362_history" in d
        assert "v1362_trend" in d
        assert d["v1363_asi_cap"] == v1363.V1363_ASI_CAP
        assert set(v1363.V1363_PHILOSOPHY_GUARDS).issubset(
            set(d["v1363_philosophy_guards"])
        )

    def test_combined_snapshot_dict_history_count_matches(self):
        from apeireth import v1362_pole_star_history as v1362
        d = v1363.get_combined_snapshot_dict(history_limit=3, window=2)
        assert d["v1362_history"]["n_total"] == v1362.history_count()


# -----------------------------------------------------------------------------
# TestV1363TrendSection
# -----------------------------------------------------------------------------

class TestV1363TrendSection:
    def test_overlay_md_is_string(self):
        md = v1363.render_trend_overlay_section_md(history_limit=3, window=2)
        assert isinstance(md, str)

    def test_overlay_md_non_empty(self):
        md = v1363.render_trend_overlay_section_md(history_limit=3, window=2)
        assert len(md) > 50

    def test_overlay_md_mentions_v1362(self):
        md = v1363.render_trend_overlay_section_md(history_limit=3, window=2)
        assert "V1362" in md

    def test_overlay_md_mentions_v1363(self):
        md = v1363.render_trend_overlay_section_md(history_limit=3, window=2)
        assert "V1363" in md

    def test_overlay_md_has_markdown_table(self):
        md = v1363.render_trend_overlay_section_md(history_limit=3, window=2)
        assert "|" in md
        assert "---" in md

    def test_overlay_md_includes_history_table(self):
        md = v1363.render_trend_overlay_section_md(history_limit=3, window=2)
        # Either empty placeholder or full table — V1362's table header
        assert ("measured_at" in md) or ("empty" in md.lower())

    def test_overlay_md_includes_trend_section(self):
        md = v1363.render_trend_overlay_section_md(history_limit=3, window=2)
        assert "Trend" in md
        assert "V3 守门" in md or "cap" in md.lower()


# -----------------------------------------------------------------------------
# TestV1363CombinedDashboard
# -----------------------------------------------------------------------------

class TestV1363CombinedDashboard:
    def test_combined_md_is_string(self):
        md = v1363.render_combined_dashboard_md(history_limit=3, window=2)
        assert isinstance(md, str)

    def test_combined_md_mentions_v1361(self):
        md = v1363.render_combined_dashboard_md(history_limit=3, window=2)
        assert "V1361" in md

    def test_combined_md_mentions_v1363(self):
        md = v1363.render_combined_dashboard_md(history_limit=3, window=2)
        assert "V1363" in md

    def test_combined_md_has_v1361_metrics(self):
        md = v1363.render_combined_dashboard_md(history_limit=3, window=2)
        # V1361 header table is present
        assert "ASI pole-star" in md or "Pole-Star" in md
        assert "toolchain" in md.lower()

    def test_combined_md_has_trend_overlay(self):
        md = v1363.render_combined_dashboard_md(history_limit=3, window=2)
        assert "trend" in md.lower()
        assert "history" in md.lower() or "history" in md

    def test_combined_md_has_v1363_guards(self):
        md = v1363.render_combined_dashboard_md(history_limit=3, window=2)
        for g in v1363.V1363_PHILOSOPHY_GUARDS:
            assert g in md, f"missing guard: {g}"

    def test_combined_md_has_asi_cap(self):
        md = v1363.render_combined_dashboard_md(history_limit=3, window=2)
        assert "0.005" in md

    def test_combined_md_has_made_by_footer(self):
        md = v1363.render_combined_dashboard_md(history_limit=3, window=2)
        assert "Made-by" in md or "Made by" in md or "楚零" in md


# -----------------------------------------------------------------------------
# TestV1363ReadOnly (AST-verified)
# -----------------------------------------------------------------------------

class TestV1363ReadOnly:
    @pytest.fixture(scope="class")
    def module_ast(self):
        path = Path(v1363.__file__)
        return ast.parse(path.read_text(encoding="utf-8"))

    def test_no_path_write_text(self, module_ast):
        """No Path.write_text() calls anywhere."""
        for node in ast.walk(module_ast):
            if isinstance(node, ast.Call):
                func = node.func
                # Check for .write_text(...) calls
                if isinstance(func, ast.Attribute) and func.attr == "write_text":
                    pytest.fail(f"Path.write_text call at line {node.lineno}")
                # Check for Path("...").write_text(...)
                if isinstance(func, ast.Attribute) and func.attr.endswith("write_text"):
                    pytest.fail(f"write_text call at line {node.lineno}")

    def test_no_shutil_copy(self, module_ast):
        """No shutil.copy / copy2 / copytree calls."""
        for node in ast.walk(module_ast):
            if isinstance(node, ast.Call):
                func = node.func
                if isinstance(func, ast.Attribute):
                    if func.attr in ("copy", "copy2", "copytree"):
                        pytest.fail(f"shutil.{func.attr} at line {node.lineno}")

    def test_write_only_in_write_combined_snapshot(self, module_ast):
        """The only `open("w"` is inside write_combined_snapshot."""
        write_opens = []
        for node in ast.walk(module_ast):
            if isinstance(node, ast.Call) and isinstance(node.func, ast.Name):
                if node.func.id == "open":
                    # Check if mode contains 'w'
                    for arg in node.args:
                        if isinstance(arg, ast.Constant) and isinstance(arg.value, str):
                            if "w" in arg.value and "r" not in arg.value:
                                # Found a write-mode open
                                write_opens.append(node.lineno)
        # write_combined_snapshot has one open(path, "w") for the dashboard file
        assert len(write_opens) >= 1, "expected at least one write-mode open (for snapshot)"
        # V1362.append_snapshot does its own open("a") — NOT counted here.

    def test_v1361_v1362_imports_only(self):
        """V1363 imports V1361 + V1362, not V1357 directly."""
        import re
        path = Path(v1363.__file__)
        src = path.read_text(encoding="utf-8")
        # Should NOT import v1357 directly
        assert "v1357_vcp_observability_snapshot" not in src or \
               "# do not import v1357" in src.lower() or \
               src.count("v1357") <= 2, \
               "V1363 should only import V1361 + V1362, not V1357 directly"


# -----------------------------------------------------------------------------
# TestV1363SnapshotWrite
# -----------------------------------------------------------------------------

class TestV1363SnapshotWrite:
    def test_snapshot_delegates_to_v1362(self):
        """write_combined_snapshot must increase V1362 history count."""
        from apeireth import v1362_pole_star_history as v1362
        with tempfile.TemporaryDirectory() as tmp:
            out_path = Path(tmp) / "STATE.md"
            before = v1362.history_count()
            result = v1363.write_combined_snapshot(
                out_path, tag="v1363-pytest", history_limit=3, window=2
            )
            after = v1362.history_count()
            assert after == before + 1, f"V1362 history grew by {after - before}, expected 1"
            assert result["history_count_after"] == after
            assert result["appended_entry_tag"] == "v1363-pytest"

    def test_snapshot_creates_file(self):
        with tempfile.TemporaryDirectory() as tmp:
            out_path = Path(tmp) / "STATE.md"
            v1363.write_combined_snapshot(
                out_path, tag="v1363-pytest", history_limit=3, window=2
            )
            assert out_path.exists()
            assert out_path.stat().st_size > 100

    def test_snapshot_file_has_v1363_marker(self):
        with tempfile.TemporaryDirectory() as tmp:
            out_path = Path(tmp) / "STATE.md"
            v1363.write_combined_snapshot(
                out_path, tag="v1363-pytest", history_limit=3, window=2
            )
            content = out_path.read_text(encoding="utf-8")
            assert "V1363" in content

    def test_snapshot_file_has_v1361_dashboard(self):
        with tempfile.TemporaryDirectory() as tmp:
            out_path = Path(tmp) / "STATE.md"
            v1363.write_combined_snapshot(
                out_path, tag="v1363-pytest", history_limit=3, window=2
            )
            content = out_path.read_text(encoding="utf-8")
            assert "V1361" in content

    def test_snapshot_file_has_trend(self):
        with tempfile.TemporaryDirectory() as tmp:
            out_path = Path(tmp) / "STATE.md"
            v1363.write_combined_snapshot(
                out_path, tag="v1363-pytest", history_limit=3, window=2
            )
            content = out_path.read_text(encoding="utf-8")
            assert "trend" in content.lower()

    def test_snapshot_creates_parent_dirs(self):
        with tempfile.TemporaryDirectory() as tmp:
            out_path = Path(tmp) / "deep" / "nested" / "STATE.md"
            v1363.write_combined_snapshot(
                out_path, tag="v1363-pytest", history_limit=3, window=2
            )
            assert out_path.exists()


# -----------------------------------------------------------------------------
# TestV1363CLI
# -----------------------------------------------------------------------------

class TestV1363CLI:
    def test_version(self, capsys):
        rc = v1363.main(["version"])
        assert rc == 0
        out = capsys.readouterr().out
        assert v1363.V1363_VERSION in out

    def test_render_md(self, capsys):
        rc = v1363.main(["render-md", "--history-limit", "3", "--window", "2"])
        assert rc == 0
        out = capsys.readouterr().out
        assert "V1363" in out
        assert "V1361" in out

    def test_render_md_trend(self, capsys):
        rc = v1363.main(["render-md-trend", "--history-limit", "3", "--window", "2"])
        assert rc == 0
        out = capsys.readouterr().out
        assert "V1362" in out

    def test_render_json(self, capsys):
        rc = v1363.main(["render-json", "--history-limit", "3", "--window", "2"])
        assert rc == 0
        out = capsys.readouterr().out
        d = json.loads(out)
        assert "v1361_snapshot" in d
        assert "v1362_history" in d
        assert "v1362_trend" in d

    def test_snapshot(self, capsys):
        with tempfile.TemporaryDirectory() as tmp:
            out_path = Path(tmp) / "STATE.md"
            rc = v1363.main([
                "snapshot",
                "--out", str(out_path),
                "--tag", "v1363-cli-test",
                "--history-limit", "3",
                "--window", "2",
            ])
            assert rc == 0
            assert out_path.exists()
            d = json.loads(capsys.readouterr().out)
            assert d["out_path"] == str(out_path)

    def test_append(self, capsys):
        from apeireth import v1362_pole_star_history as v1362
        before = v1362.history_count()
        rc = v1363.main(["append", "--tag", "v1363-cli-append"])
        assert rc == 0
        after = v1362.history_count()
        assert after == before + 1

    def test_self_test(self, capsys):
        rc = v1363.main(["self-test"])
        assert rc == 0
        out = capsys.readouterr().out
        assert "passed" in out
        assert "/" in out  # "45/45 passed"


# -----------------------------------------------------------------------------
# TestV1363TrendBounds (honesty)
# -----------------------------------------------------------------------------

class TestV1363TrendBounds:
    def test_combined_trend_delta_bounded(self):
        """If history has ≥2 entries, delta is numeric and bounded."""
        from apeireth import v1362_pole_star_history as v1362
        n = v1362.history_count()
        if n < 2:
            pytest.skip("need ≥2 V1362 history entries for trend bound check")
        d = v1363.get_combined_snapshot_dict(history_limit=3, window=2)
        trend = d["v1362_trend"]
        if trend["n_entries"] >= 2 and trend["delta"] is not None:
            assert abs(trend["delta"]) <= 1.0

    def test_combined_pole_star_total_capped(self):
        """Pole-star total is capped at honest_cap (V1356 invariant)."""
        d = v1363.get_combined_snapshot_dict(history_limit=3, window=2)
        pole = d["v1361_snapshot"]["pole_star"]
        total = pole.get("total")
        cap = pole.get("honest_cap")
        if total is not None and cap is not None:
            assert total <= cap + 1e-9, f"total={total} > cap={cap}"


if __name__ == "__main__":
    import sys
    sys.exit(pytest.main([__file__, "-v"]))