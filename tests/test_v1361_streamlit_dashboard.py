"""Pytest suite for V1361 streamlit dashboard.

V1361 = visual observability dashboard that wraps V1357 snapshot.
These tests verify:
  1. Constants (version, cap, guards, subweights).
  2. Single-source-of-truth delegation to V1357.
  3. Render functions return correct shape.
  4. Streamlit is OPTIONAL (no top-level import).
  5. No filesystem writes (AST-level read-only assertion).
  6. V3 philosophy guards present.
"""
from __future__ import annotations

import json

import pytest

from apeireth import v1361_streamlit_dashboard as v1361


# -----------------------------------------------------------------------------
# TestV1361Constants
# -----------------------------------------------------------------------------

class TestV1361Constants:
    def test_version_is_semver(self):
        assert v1361.V1361_VERSION.count(".") == 2

    def test_asi_cap_below_threshold(self):
        assert v1361.V1361_ASI_CAP <= 0.01

    def test_asi_cap_positive(self):
        assert v1361.V1361_ASI_CAP > 0

    def test_cap_value(self):
        # honest cap; dashboard ≠ ASI
        assert v1361.V1361_ASI_CAP == 0.005

    def test_philosophy_guards_complete(self):
        expected = {
            "GUARD_DASHBOARD_NOT_ASI",
            "GUARD_DELEGATE_TO_V1357",
            "GUARD_READ_ONLY",
            "GUARD_NO_WRITES",
            "GUARD_NO_FAKE_METRICS",
            "GUARD_HONEST_CAP",
        }
        assert expected.issubset(set(v1361.V1361_PHILOSOPHY_GUARDS))

    def test_subweights_sum_to_one(self):
        total = sum(v1361.V1361_SUBWEIGHTS.values())
        assert abs(total - 1.0) < 1e-9


# -----------------------------------------------------------------------------
# TestV1361DataSource
# -----------------------------------------------------------------------------

class TestV1361DataSource:
    def test_snapshot_returns_object(self):
        snap = v1361.get_snapshot()
        assert snap is not None

    def test_snapshot_to_dict(self):
        d = v1361.snapshot_to_dict()
        assert "pole_star" in d
        assert "toolchain_health" in d
        assert "close_loop_state" in d
        assert "recent_commits" in d
        assert "infra_state" in d
        assert "philosophy_guards" in d

    def test_pole_star_has_8_components(self):
        d = v1361.snapshot_to_dict()
        components = d["pole_star"]["components"]
        assert len(components) == 8

    def test_pole_star_total_is_capped(self):
        """V1361 doesn't enforce a cap; it just reads V1357's already-capped total.
        The total should be at or below honest_cap."""
        d = v1361.snapshot_to_dict()
        total = d["pole_star"]["total"]
        cap = d["pole_star"]["honest_cap"]
        assert total is not None
        assert total <= cap + 1e-9

    def test_pole_star_components_present(self):
        d = v1361.snapshot_to_dict()
        components = d["pole_star"]["components"]
        # 8 named components per V1356
        assert len(components) == 8
        names = {c["name"] for c in components}
        # Some expected names from V1356
        assert "phi_proxy" in names
        assert "real_production" in names
        assert "vcp_toolchain" in names

    def test_toolchain_has_11_modules(self):
        d = v1361.snapshot_to_dict()
        assert d["toolchain_health"]["n_modules_total"] == 11

    def test_toolchain_present_matches_total(self):
        # V1360 closed the name-drift; V1357 should now see 11/11
        d = v1361.snapshot_to_dict()
        assert d["toolchain_health"]["n_modules_present"] == 11

    def test_close_loop_has_7_scenarios(self):
        d = v1361.snapshot_to_dict()
        scenarios = d["close_loop_state"]["scenarios"]
        assert len(scenarios) == 7

    def test_close_loop_all_pass(self):
        d = v1361.snapshot_to_dict()
        n_pass = d["close_loop_state"]["n_pass"]
        assert n_pass == 7


# -----------------------------------------------------------------------------
# TestV1361Renders
# -----------------------------------------------------------------------------

class TestV1361Renders:
    def test_header_markdown(self):
        snap = v1361.snapshot_to_dict()
        md = v1361.render_header_md(snap)
        assert "Apeireth V1361 Dashboard" in md
        assert "pole-star" in md.lower() or "Pole-Star" in md
        assert "toolchain" in md

    def test_pole_star_markdown_has_components(self):
        snap = v1361.snapshot_to_dict()
        md = v1361.render_pole_star_md(snap)
        for c in snap["pole_star"]["components"]:
            assert c["name"] in md

    def test_toolchain_markdown_has_all_modules(self):
        snap = v1361.snapshot_to_dict()
        md = v1361.render_toolchain_md(snap)
        # All 11 modules should appear in toolchain table
        from apeireth.v1357_vcp_observability_snapshot import VCP_TOOLCHAIN_MODULES
        for mod in VCP_TOOLCHAIN_MODULES:
            assert mod in md, f"missing module: {mod}"

    def test_close_loop_markdown_has_scenarios(self):
        snap = v1361.snapshot_to_dict()
        md = v1361.render_close_loop_md(snap)
        for s in snap["close_loop_state"]["scenarios"]:
            assert s["name"] in md

    def test_infra_markdown_has_keys(self):
        snap = v1361.snapshot_to_dict()
        md = v1361.render_infra_md(snap)
        for k in snap["infra_state"].keys():
            assert k in md

    def test_commits_markdown_has_hash(self):
        snap = v1361.snapshot_to_dict()
        md = v1361.render_commits_md(snap)
        if snap["recent_commits"]:
            assert snap["recent_commits"][0]["hash"] in md

    def test_unknowns_markdown_nonempty(self):
        snap = v1361.snapshot_to_dict()
        md = v1361.render_unknowns_md(snap)
        assert len(md) > 20

    def test_guards_markdown_has_cap(self):
        md = v1361.render_guards_md()
        assert "0.005" in md or str(v1361.V1361_ASI_CAP) in md

    def test_full_markdown_composite(self):
        md = v1361.render_full_markdown()
        assert "Apeireth V1361" in md
        assert "Pole-Star" in md
        assert "VCP Toolchain Health" in md
        assert "V1355 Close-Loop" in md
        assert "VCP Infra Files" in md
        assert "Recent Commits" in md
        assert "Philosophy Guards" in md or "Philosophy" in md
        # All sections present
        assert md.count("###") >= 7  # at least 7 H3 sections

    def test_full_markdown_deterministic_with_frozen_snapshot(self):
        """When given the same frozen snapshot dict, render must be deterministic.
        (Without frozen input, V1357 re-runs wet-run on each call and durations drift.)"""
        frozen = v1361.snapshot_to_dict()
        md1 = v1361.render_full_markdown(frozen)
        md2 = v1361.render_full_markdown(frozen)
        assert md1 == md2

    def test_full_markdown_starts_with_header(self):
        md = v1361.render_full_markdown()
        first_line = md.split("\n", 1)[0]
        assert first_line.startswith("## Apeireth V1361 Dashboard")


# -----------------------------------------------------------------------------
# TestV1361StreamlitOptional
# -----------------------------------------------------------------------------

class TestV1361StreamlitOptional:
    def test_streamlit_not_imported_at_module_top(self):
        """V1361 source must not `import streamlit` outside of `serve()`.
        This allows pytest to run without streamlit installed."""
        with open(v1361.__file__, "r", encoding="utf-8") as f:
            src = f.read()
        before_serve = src.split("def serve")[0]
        assert "import streamlit" not in before_serve

    def test_streamlit_inside_serve(self):
        with open(v1361.__file__, "r", encoding="utf-8") as f:
            src = f.read()
        # streamlit import should appear inside serve()
        assert "import streamlit" in src
        serve_idx = src.find("def serve")
        import_idx = src.find("import streamlit")
        assert import_idx > serve_idx, "streamlit must be imported lazily inside serve()"


# -----------------------------------------------------------------------------
# TestV1361ReadOnly
# -----------------------------------------------------------------------------

class TestV1361ReadOnly:
    def test_no_write_mode_open(self):
        """V1361 source must not contain any `open(..., mode containing w/a/x/+)`."""
        import ast
        with open(v1361.__file__, "r", encoding="utf-8") as f:
            src = f.read()
        tree = ast.parse(src)
        bad = []
        for node in ast.walk(tree):
            if isinstance(node, ast.Call) and isinstance(node.func, ast.Name) and node.func.id == "open":
                for kw in node.keywords:
                    if kw.arg == "mode" and isinstance(kw.value, ast.Constant) and isinstance(kw.value.value, str):
                        if any(c in kw.value.value for c in "wax+"):
                            bad.append(f"open(mode={kw.value.value!r})")
        assert bad == [], f"V1361 has write-mode open: {bad}"

    def test_no_path_write(self):
        import ast
        with open(v1361.__file__, "r", encoding="utf-8") as f:
            src = f.read()
        tree = ast.parse(src)
        bad = []
        for node in ast.walk(tree):
            if isinstance(node, ast.Call) and isinstance(node.func, ast.Attribute):
                if node.func.attr in {"write_text", "write_bytes"}:
                    bad.append(node.func.attr)
        assert bad == [], f"V1361 has write calls: {bad}"

    def test_no_shutil_copy(self):
        import ast
        with open(v1361.__file__, "r", encoding="utf-8") as f:
            src = f.read()
        tree = ast.parse(src)
        bad = []
        for node in ast.walk(tree):
            if isinstance(node, ast.Call) and isinstance(node.func, ast.Attribute):
                if node.func.attr in {"copy", "copy2", "copytree", "move", "rename"}:
                    bad.append(node.func.attr)
        assert bad == [], f"V1361 has shutil/fs writes: {bad}"


# -----------------------------------------------------------------------------
# TestV1361Json
# -----------------------------------------------------------------------------

class TestV1361Json:
    def test_snapshot_json_serializable(self):
        d = v1361.snapshot_to_dict()
        out = json.dumps(d, ensure_ascii=False)
        assert len(out) > 100
        # Roundtrip
        d2 = json.loads(out)
        assert d2["pole_star"]["total"] == d["pole_star"]["total"]


# -----------------------------------------------------------------------------
# TestV1361CLI
# -----------------------------------------------------------------------------

class TestV1361CLI:
    def test_version(self, capsys):
        rc = v1361.main(["version"])
        assert rc == 0
        out = capsys.readouterr().out
        assert v1361.V1361_VERSION in out

    def test_render_json(self, capsys):
        rc = v1361.main(["render-json"])
        assert rc == 0
        out = capsys.readouterr().out
        data = json.loads(out)
        assert "pole_star" in data

    def test_render_md(self, capsys):
        rc = v1361.main(["render-md"])
        assert rc == 0
        out = capsys.readouterr().out
        assert "Apeireth V1361" in out

    def test_serve_help(self, capsys):
        # serve just prints instructions, should not crash
        rc = v1361.main(["serve"])
        assert rc == 0
        out = capsys.readouterr().out
        assert "streamlit run" in out

    def test_self_test(self, capsys):
        rc = v1361.main(["self-test"])
        assert rc == 0
        out = capsys.readouterr().out
        assert "passed" in out


if __name__ == "__main__":
    import sys
    sys.exit(pytest.main([__file__, "-v"]))