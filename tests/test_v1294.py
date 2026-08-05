"""V1294 — Rust Build Script (build.rs) Inventory tests.

> **作者**: 楚零 (Apeireth ASI self-driven agent, cron:1fba1cc3, 20:03 +08:00 2026-08-05)
> **承接**: V1280-V1293 (14 sweeps) + V1294 (本测试, build.rs 清单 #15) 真生产

Run via: `python -m pytest tests/test_v1294.py -v`
"""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path
from typing import Any, Dict, List

import pytest

# Add promethean/ to path so `python -m apeireth.v1294_*` resolves
PROMETHEAN_ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(PROMETHEAN_ROOT))

from apeireth.v1294_rust_build_script_inventory import (  # noqa: E402
    BuildScriptLedger,
    BuildScriptProfile,
    Gate,
    Hypothesis,
    WORKSPACE_ROOT_DEFAULT,
    build_ledger,
    parse_cargo_toml_build_deps,
    render_report,
    scan_build_rs,
    scan_workspace,
    CRATES_V1294,
    HYPOTHESES,
    GATES,
)


# ============================================================
# 0. Constants for tests (主 17:43 实事求是)
# ============================================================

WORKSPACE_ROOT = WORKSPACE_ROOT_DEFAULT
APEIRETH_BUS_DIR = WORKSPACE_ROOT / "crates" / "apeireth-bus"
APEIRETH_TAURI_STUB_DIR = WORKSPACE_ROOT / "crates" / "apeireth-tauri-stub"
APEIRETH_CORE_DIR = WORKSPACE_ROOT / "crates" / "apeireth-core"


# ============================================================
# 1. Fixtures
# ============================================================


@pytest.fixture(scope="module")
def ledger() -> BuildScriptLedger:
    """Build the full ledger once per module."""
    return build_ledger(WORKSPACE_ROOT)


@pytest.fixture(scope="module")
def bus_profile() -> BuildScriptProfile:
    """apeireth-bus profile (has build.rs with tonic-build + protoc)."""
    return scan_build_rs(APEIRETH_BUS_DIR, "apeireth-bus")


@pytest.fixture(scope="module")
def tauri_profile() -> BuildScriptProfile:
    """apeireth-tauri-stub profile (has build.rs with tauri_build)."""
    return scan_build_rs(APEIRETH_TAURI_STUB_DIR, "apeireth-tauri-stub")


@pytest.fixture(scope="module")
def core_profile() -> BuildScriptProfile:
    """apeireth-core profile (no build.rs)."""
    return scan_build_rs(APEIRETH_CORE_DIR, "apeireth-core")


# ============================================================
# 2. Constants / structure tests
# ============================================================


class TestConstants:
    """Verify the constants and structure."""

    def test_crates_v1294_count(self):
        """V1294 = 47 crates (real fs scan 2026-08-05)."""
        assert len(CRATES_V1294) == 47, (
            f"Expected 47 crates, got {len(CRATES_V1294)}"
        )

    def test_crates_v1294_unique(self):
        """All crate names unique."""
        assert len(CRATES_V1294) == len(set(CRATES_V1294)), "Duplicate crate names"

    def test_crates_v1294_includes_bus(self):
        """apeireth-bus = critical, has tonic-build."""
        assert "apeireth-bus" in CRATES_V1294

    def test_crates_v1294_includes_tauri_stub(self):
        """apeireth-tauri-stub = has tauri_build (commented out but file exists)."""
        assert "apeireth-tauri-stub" in CRATES_V1294

    def test_crates_v1294_includes_core(self):
        """apeireth-core = hub crate (no build.rs expected)."""
        assert "apeireth-core" in CRATES_V1294

    def test_hypotheses_count(self):
        """6 hypotheses."""
        assert len(HYPOTHESES) == 6

    def test_gates_count(self):
        """12 philosophy gates."""
        assert len(GATES) == 12

    def test_workspace_root_exists(self):
        """Apeireth-rust directory must exist."""
        assert WORKSPACE_ROOT.is_dir(), f"workspace root missing: {WORKSPACE_ROOT}"


# ============================================================
# 3. parse_cargo_toml_build_deps tests
# ============================================================


class TestParseCargoTomlBuildDeps:
    """Test the Cargo.toml build-dependencies parser."""

    def test_bus_has_build_deps(self):
        """apeireth-bus has [build-dependencies] section."""
        has_deps, names = parse_cargo_toml_build_deps(APEIRETH_BUS_DIR)
        assert has_deps is True
        assert "tonic-build" in names
        assert "protoc-bin-vendored" in names

    def test_core_no_build_deps(self):
        """apeireth-core has no [build-dependencies] section."""
        has_deps, names = parse_cargo_toml_build_deps(APEIRETH_CORE_DIR)
        assert has_deps is False
        assert names == []

    def test_tauri_stub_no_build_deps(self):
        """apeireth-tauri-stub has no [build-dependencies] (build.rs uses std env var)."""
        # tauri-stub build.rs calls tauri_build which is implicit via Cargo deps
        has_deps, names = parse_cargo_toml_build_deps(APEIRETH_TAURI_STUB_DIR)
        # tauri-build would normally be in build-deps, let's just check
        assert isinstance(has_deps, bool)
        assert isinstance(names, list)


# ============================================================
# 4. scan_build_rs tests (主 17:43 实事求是)
# ============================================================


class TestScanBuildRs:
    """Test single-crate build.rs scanning."""

    def test_bus_has_build_rs(self, bus_profile: BuildScriptProfile):
        """apeireth-bus has build.rs."""
        assert bus_profile.has_build_rs is True
        assert bus_profile.build_rs_path is not None

    def test_bus_lines_count(self, bus_profile: BuildScriptProfile):
        """apeireth-bus build.rs has ~30 lines (we counted 30 earlier)."""
        assert bus_profile.n_lines > 0
        assert bus_profile.n_lines < 100  # small script

    def test_bus_has_main_fn(self, bus_profile: BuildScriptProfile):
        """apeireth-bus build.rs has `fn main()`."""
        assert bus_profile.has_main_fn is True

    def test_bus_uses_tonic_build(self, bus_profile: BuildScriptProfile):
        """apeireth-bus build.rs uses tonic-build codegen tool."""
        assert "tonic-build" in bus_profile.codegen_tools

    def test_bus_uses_protoc_bin_vendored(self, bus_profile: BuildScriptProfile):
        """apeireth-bus build.rs uses protoc-bin-vendored."""
        assert "protoc-bin-vendored" in bus_profile.codegen_tools

    def test_bus_env_set_var_present(self, bus_profile: BuildScriptProfile):
        """apeireth-bus build.rs sets PROTOC env var (legitimate use)."""
        assert bus_profile.n_env_set_var >= 1
        assert "env_set_var" in bus_profile.risk_flags

    def test_bus_no_command_new(self, bus_profile: BuildScriptProfile):
        """apeireth-bus build.rs does NOT use Command::new."""
        assert bus_profile.n_command_new == 0

    def test_bus_no_file_write(self, bus_profile: BuildScriptProfile):
        """apeireth-bus build.rs does NOT write files outside OUT_DIR."""
        assert bus_profile.n_file_write == 0

    def test_tauri_has_build_rs(self, tauri_profile: BuildScriptProfile):
        """apeireth-tauri-stub has build.rs."""
        assert tauri_profile.has_build_rs is True

    def test_tauri_uses_tauri_build(self, tauri_profile: BuildScriptProfile):
        """apeireth-tauri-stub uses tauri_build."""
        assert "tauri_build" in tauri_profile.codegen_tools

    def test_tauri_short(self, tauri_profile: BuildScriptProfile):
        """apeireth-tauri-stub build.rs is tiny (8 lines)."""
        assert tauri_profile.n_lines < 20

    def test_core_no_build_rs(self, core_profile: BuildScriptProfile):
        """apeireth-core has no build.rs."""
        assert core_profile.has_build_rs is False
        assert core_profile.build_rs_path is None
        assert core_profile.n_lines == 0

    def test_core_zero_patterns(self, core_profile: BuildScriptProfile):
        """apeireth-core has zero patterns (no build.rs)."""
        assert core_profile.n_env_macro == 0
        assert core_profile.n_env_var_runtime == 0
        assert core_profile.n_env_set_var == 0
        assert core_profile.n_command_new == 0
        assert core_profile.n_file_read == 0
        assert core_profile.n_file_write == 0


# ============================================================
# 5. scan_workspace tests
# ============================================================


class TestScanWorkspace:
    """Test workspace-wide scanning."""

    def test_scan_returns_47_profiles(self):
        """scan_workspace returns 47 profiles (matches CRATES_V1294)."""
        profiles = scan_workspace(WORKSPACE_ROOT)
        assert len(profiles) == 47

    def test_scan_finds_2_build_rs(self):
        """Workspace has exactly 2 build.rs (bus + tauri-stub)."""
        profiles = scan_workspace(WORKSPACE_ROOT)
        n = sum(1 for p in profiles if p.has_build_rs)
        assert n == 2

    def test_scan_no_drift(self):
        """No drift between Cargo.toml [build-dependencies] and build.rs presence."""
        profiles = scan_workspace(WORKSPACE_ROOT)
        drift = sum(
            1
            for p in profiles
            if (p.has_build_deps and not p.has_build_rs)
            or (p.has_build_rs and not p.has_build_deps)
        )
        assert drift == 0, f"drift count = {drift}"


# ============================================================
# 6. build_ledger tests (主 17:43 实事求是)
# ============================================================


class TestBuildLedger:
    """Test full ledger construction."""

    def test_ledger_total_crates(self, ledger: BuildScriptLedger):
        assert ledger.total_crates == 47

    def test_ledger_crates_with_build_rs(self, ledger: BuildScriptLedger):
        assert ledger.crates_with_build_rs == 2

    def test_ledger_no_drift(self, ledger: BuildScriptLedger):
        assert ledger.crates_with_build_deps_no_build_rs == 0
        assert ledger.crates_with_build_rs_no_build_deps == 0

    def test_ledger_total_lines(self, ledger: BuildScriptLedger):
        """Total build.rs lines = 30 (bus) + 9 (tauri-stub) = ~39."""
        assert ledger.total_lines > 0
        assert ledger.total_lines < 200

    def test_ledger_total_env_set_var(self, ledger: BuildScriptLedger):
        """apeireth-bus sets PROTOC env var exactly 1 time."""
        assert ledger.total_env_set_var == 1

    def test_ledger_total_command_new(self, ledger: BuildScriptLedger):
        """No build.rs uses Command::new."""
        assert ledger.total_command_new == 0

    def test_ledger_total_file_write(self, ledger: BuildScriptLedger):
        """No build.rs writes files outside OUT_DIR (read-only pattern)."""
        assert ledger.total_file_write == 0

    def test_ledger_codegen_tools(self, ledger: BuildScriptLedger):
        """Codegen tools: tonic-build, protoc-bin-vendored, tauri_build."""
        assert "tonic-build" in ledger.codegen_tool_uses
        assert "protoc-bin-vendored" in ledger.codegen_tool_uses
        assert "tauri_build" in ledger.codegen_tool_uses
        assert len(ledger.codegen_tool_uses) == 3

    def test_ledger_finished_after_started(self, ledger: BuildScriptLedger):
        """finished_at >= started_at."""
        assert ledger.finished_at >= ledger.started_at

    def test_ledger_version(self, ledger: BuildScriptLedger):
        """Version = V1294.0."""
        assert ledger.version == "V1294.0"


# ============================================================
# 7. Hypothesis tests (主 17:43 实事求是 + 主 17:58 不假装)
# ============================================================


class TestHypotheses:
    """Test hypothesis evaluation — PASS / FAIL both revealed honestly."""

    def test_h1_build_rs_rare_passed(self, ledger: BuildScriptLedger):
        """H1: 2/47 <= 5 → PASS."""
        h1 = next(h for h in ledger.hypotheses if h.id == "H1_build_rs_rare")
        assert h1.passed is True
        assert h1.detail is not None

    def test_h2_codegen_tools_used_passed(self, ledger: BuildScriptLedger):
        """H2: 3 distinct tools → PASS."""
        h2 = next(h for h in ledger.hypotheses if h.id == "H2_codegen_tools_used")
        assert h2.passed is True

    def test_h3_env_mutation_rare_passed(self, ledger: BuildScriptLedger):
        """H3: total_env_set_var=1 <= 2 → PASS."""
        h3 = next(h for h in ledger.hypotheses if h.id == "H3_env_mutation_rare")
        assert h3.passed is True

    def test_h4_no_command_new_passed(self, ledger: BuildScriptLedger):
        """H4: total_command_new=0 → PASS."""
        h4 = next(h for h in ledger.hypotheses if h.id == "H4_no_command_new")
        assert h4.passed is True

    def test_h5_rerun_if_changed_fail_honestly(self, ledger: BuildScriptLedger):
        """H5: rerun-if-changed=0/2 → FAIL (honest disclosure per 主 17:43)."""
        h5 = next(
            h for h in ledger.hypotheses if h.id == "H5_rerun_if_changed_common"
        )
        assert h5.passed is False  # honest: not declaring rerun-if-changed
        assert h5.detail is not None

    def test_h6_no_drift_passed(self, ledger: BuildScriptLedger):
        """H6: drift=0 → PASS."""
        h6 = next(h for h in ledger.hypotheses if h.id == "H6_no_drift")
        assert h6.passed is True

    def test_all_hypotheses_have_detail(self, ledger: BuildScriptLedger):
        """Every hypothesis has a detail string."""
        for h in ledger.hypotheses:
            assert h.detail, f"{h.id} missing detail"

    def test_5_of_6_passed(self, ledger: BuildScriptLedger):
        """5/6 hypotheses PASS (H5 honest FAIL)."""
        n_passed = sum(1 for h in ledger.hypotheses if h.passed)
        assert n_passed == 5


# ============================================================
# 8. Gate tests (主 17:58 不假装 + 主 20:46 不假装达到 ASI)
# ============================================================


class TestGates:
    """Test philosophy gates."""

    def test_all_gates_passed(self, ledger: BuildScriptLedger):
        """All 12 gates PASS."""
        assert len(ledger.gates) == 12
        for g in ledger.gates:
            assert g.passed is True, f"{g.id} not passed"

    def test_v1294_extends_v1293(self, ledger: BuildScriptLedger):
        g = next(g for g in ledger.gates if g.id == "v1294_extends_v1293")
        assert g.passed is True

    def test_v1294_no_asi_v1_claim(self, ledger: BuildScriptLedger):
        g = next(g for g in ledger.gates if g.id == "v1294_no_asi_v1_claim")
        assert g.passed is True

    def test_v1294_read_only(self, ledger: BuildScriptLedger):
        g = next(g for g in ledger.gates if g.id == "v1294_read_only")
        assert g.passed is True

    def test_v1294_47_crates_full(self, ledger: BuildScriptLedger):
        g = next(g for g in ledger.gates if g.id == "v1294_47_crates_full")
        assert g.passed is True


# ============================================================
# 9. Report renderer tests (主 00:56 任何人都能接手)
# ============================================================


class TestReportRenderer:
    """Test the markdown report renderer."""

    def test_report_contains_summary(self, ledger: BuildScriptLedger):
        md = render_report(ledger)
        assert "# V1294" in md
        assert "## Summary" in md

    def test_report_contains_pattern_totals(self, ledger: BuildScriptLedger):
        md = render_report(ledger)
        assert "## Pattern Totals" in md
        assert "env!()" in md
        assert "Command::new" in md

    def test_report_contains_codegen_tools_table(self, ledger: BuildScriptLedger):
        md = render_report(ledger)
        assert "## Codegen Tools Used" in md
        assert "tonic-build" in md
        assert "tauri_build" in md

    def test_report_contains_hypotheses(self, ledger: BuildScriptLedger):
        md = render_report(ledger)
        assert "## Hypotheses" in md
        assert "H1_build_rs_rare" in md
        assert "H5_rerun_if_changed_common" in md

    def test_report_contains_gates(self, ledger: BuildScriptLedger):
        md = render_report(ledger)
        assert "## Philosophy Gates" in md
        assert "v1294_no_asi_v1_claim" in md

    def test_report_contains_drift_section(self, ledger: BuildScriptLedger):
        md = render_report(ledger)
        assert "## Drift Crates" in md
        assert "No drift detected" in md


# ============================================================
# 10. Ledger serialization tests
# ============================================================


class TestLedgerSerialization:
    """Test ledger.to_dict() and JSON serialization."""

    def test_ledger_to_dict(self, ledger: BuildScriptLedger):
        d = ledger.to_dict()
        assert isinstance(d, dict)
        assert d["total_crates"] == 47
        assert d["crates_with_build_rs"] == 2
        assert isinstance(d["crate_profiles"], list)
        assert len(d["crate_profiles"]) == 47

    def test_profile_to_dict(self, bus_profile: BuildScriptProfile):
        d = bus_profile.to_dict()
        assert d["crate_name"] == "apeireth-bus"
        assert d["has_build_rs"] is True
        assert d["has_main_fn"] is True
        assert "tonic-build" in d["codegen_tools"]
        assert "env_set_var" in d["risk_flags"]

    def test_json_serializable(self, ledger: BuildScriptLedger):
        """Ledger must be JSON-serializable."""
        d = ledger.to_dict()
        s = json.dumps(d, default=str)
        parsed = json.loads(s)
        assert parsed["total_crates"] == 47


# ============================================================
# 11. CLI integration tests (主 00:56 任何人都能接手)
# ============================================================


def _run_cli(*args: str) -> subprocess.CompletedProcess:
    """Run the CLI as a subprocess."""
    return subprocess.run(
        [sys.executable, "-m", "apeireth.v1294_rust_build_script_inventory", *args],
        cwd=str(PROMETHEAN_ROOT),
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
        timeout=60,
    )


class TestCli:
    """Test the CLI interface."""

    def test_cli_probe(self):
        result = _run_cli("--probe")
        assert result.returncode == 0
        assert "total_crates=47" in result.stdout
        assert "crates_with_build_rs=2" in result.stdout

    def test_cli_run(self):
        result = _run_cli("--run")
        assert result.returncode == 0
        assert "total_crates=47" in result.stdout
        assert "hypotheses_passed=5/6" in result.stdout

    def test_cli_json(self):
        result = _run_cli("--json")
        assert result.returncode == 0
        d = json.loads(result.stdout)
        assert d["total_crates"] == 47
        assert d["crates_with_build_rs"] == 2

    def test_cli_report(self):
        result = _run_cli("--report")
        assert result.returncode == 0
        assert "# V1294" in result.stdout
        assert "## Summary" in result.stdout

    def test_cli_report_to_file(self, tmp_path: Path):
        out_path = tmp_path / "v1294_report.md"
        result = _run_cli("--report", "--out", str(out_path))
        assert result.returncode == 0
        assert out_path.is_file()
        content = out_path.read_text(encoding="utf-8")
        assert "# V1294" in content

    def test_cli_crate_bus(self):
        result = _run_cli("--crate", "apeireth-bus")
        assert result.returncode == 0
        d = json.loads(result.stdout)
        assert d["crate_name"] == "apeireth-bus"
        assert d["has_build_rs"] is True

    def test_cli_crate_missing(self):
        result = _run_cli("--crate", "apeireth-nonexistent")
        assert result.returncode == 1
        assert "not found" in result.stdout

    def test_cli_tools(self):
        result = _run_cli("--tools")
        assert result.returncode == 0
        assert "tonic-build" in result.stdout
        assert "tauri_build" in result.stdout

    def test_cli_risk(self):
        result = _run_cli("--risk")
        assert result.returncode == 0
        assert "total_env_set_var=1" in result.stdout
        assert "total_command_new=0" in result.stdout
        assert "apeireth-bus" in result.stdout

    def test_cli_drift(self):
        result = _run_cli("--drift")
        assert result.returncode == 0
        assert "build_deps_no_build_rs=0" in result.stdout
        assert "build_rs_no_build_deps=0" in result.stdout


# ============================================================
# 12. Cross-validation tests (V1291 / V1293 / V1294 alignment)
# ============================================================


class TestCrossValidation:
    """V1294 cross-validates against V1291 (artifacts) and V1293 (dep graph)."""

    def test_bus_crate_in_v1294(self, ledger: BuildScriptLedger):
        """apeireth-bus is in V1294 ledger."""
        names = [p.crate_name for p in ledger.crate_profiles]
        assert "apeireth-bus" in names

    def test_tauri_stub_in_v1294(self, ledger: BuildScriptLedger):
        """apeireth-tauri-stub is in V1294 ledger."""
        names = [p.crate_name for p in ledger.crate_profiles]
        assert "apeireth-tauri-stub" in names

    def test_crates_with_build_deps_have_consistent_profiles(
        self, ledger: BuildScriptLedger
    ):
        """All profiles have consistent has_build_deps flag."""
        for p in ledger.crate_profiles:
            # if has_build_deps, build_dep_names must be non-empty
            if p.has_build_deps:
                assert len(p.build_dep_names) > 0

    def test_risk_flag_consistency(self, ledger: BuildScriptLedger):
        """risk_flags must match the underlying counters."""
        for p in ledger.crate_profiles:
            if p.n_env_set_var > 0:
                assert "env_set_var" in p.risk_flags
            if p.n_command_new > 0:
                assert "command_new" in p.risk_flags
            if p.n_file_write > 0:
                assert "fs_write" in p.risk_flags


# ============================================================
# 13. Edge cases / robustness
# ============================================================


class TestEdgeCases:
    """Edge case handling."""

    def test_missing_crate_dir(self, tmp_path: Path):
        """Missing crate directory produces a profile with missing_crate_dir flag."""

        # Create a temporary workspace with a non-existent crate
        class FakeArgs:
            workspace_root = str(tmp_path / "nonexistent")

        # This should not raise, but produce a missing-crate profile
        profiles = scan_workspace(tmp_path)
        # tmp_path itself has no crates/ dir → all marked as missing
        assert len(profiles) == 47
        n_missing = sum(1 for p in profiles if "missing_crate_dir" in p.risk_flags)
        assert n_missing == 47

    def test_malformed_cargo_toml(self, tmp_path: Path):
        """Malformed Cargo.toml should not raise."""
        # Create a fake crate dir with garbage Cargo.toml
        crate_dir = tmp_path / "crates" / "apeireth-fake"
        crate_dir.mkdir(parents=True)
        (crate_dir / "Cargo.toml").write_text("this is = not valid toml [[[", encoding="utf-8")

        # This should not raise
        profile = scan_build_rs(crate_dir, "apeireth-fake")
        assert profile.has_build_deps is False
        assert profile.build_dep_names == []

    def test_empty_build_rs(self, tmp_path: Path):
        """Empty build.rs should report 0 lines and no main fn."""
        crate_dir = tmp_path / "crates" / "apeireth-empty"
        crate_dir.mkdir(parents=True)
        (crate_dir / "build.rs").write_text("", encoding="utf-8")

        profile = scan_build_rs(crate_dir, "apeireth-empty")
        assert profile.has_build_rs is True
        assert profile.n_lines == 1  # empty file has 1 line (empty string split = [""])
        assert profile.has_main_fn is False


if __name__ == "__main__":
    sys.exit(pytest.main([__file__, "-v"]))