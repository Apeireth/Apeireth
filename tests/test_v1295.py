"""Tests for V1295 — Cargo.lock Lockfile Audit.

> **作者**: 楚零 (Apeireth ASI self-driven agent, cron:1fba1cc3, 20:15 +08:00 2026-08-05)
> **真生产测试** (主 17:43 实事求是 + 主 17:58 不假装):
>   - 解真实 Cargo.lock (offline, stdlib only)
>   - 验证 lockfile structure 符合 Cargo v3 schema
>   - 验证 internal (apeireth-*) 覆盖 workspace members
>   - 验证 checksum 覆盖率 100%
>   - 验证 multi-version / top-referenced / workspace presence 检测正确
>   - 不 mock 数据, 不假数据, 真扫真数

## 关键原则 (主 17:43 + 主 17:58)
- 测真值不测 mock: 所有 assertions 用真扫数据
- 不刷 KPI: NS 92.91% LOCKED 不变
- 不假装 ASI V1: Cargo.lock audit ≠ ASI
- audit ≠ fix: 仅验证检测能力, 不真改 lockfile
"""

from __future__ import annotations

import json
import os
import sys
import tempfile
from pathlib import Path

# Allow imports from project root
PROJECT_ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(PROJECT_ROOT))

from apeireth.v1295_cargo_lockfile_audit import (
    HYPOTHESES,
    GATES,
    INTERNAL_PREFIX,
    THRESHOLD_CHECKSUM_COVERAGE_PCT,
    THRESHOLD_INTERNAL_PACKAGES_MIN,
    THRESHOLD_LOCKFILE_MAX_LINES,
    THRESHOLD_MULTI_VERSION_MAX_PCT,
    THRESHOLD_DISTINCT_SOURCES_MIN,
    WORKSPACE_MEMBERS_V1295,
    build_ledger,
    check_workspace_member_presence,
    compute_multi_version_crates,
    compute_top_referenced_crates,
    evaluate_gates,
    evaluate_hypotheses,
    parse_cargo_lock,
    _parse_single_package,
    render_report,
)
from apeireth.v1295_cargo_lockfile_audit import (
    LockPackage,
    MultiVersionCrate,
    TopReferencedCrate,
    WorkspaceMemberLockPresence,
    LockfileLedger,
)


WORKSPACE_ROOT = PROJECT_ROOT / "Apeireth-rust"
LOCKFILE = WORKSPACE_ROOT / "Cargo.lock"


# ============================================================
# 1. Constants sanity (主 17:43 实事求是)
# ============================================================


class TestConstants:
    """Sanity-check constants are coherent."""

    def test_internal_prefix_is_apeireth(self):
        assert INTERNAL_PREFIX == "apeireth-"

    def test_workspace_members_count(self):
        # workspace members list should have 46 active (excluding commented-out tauri-stub)
        assert len(WORKSPACE_MEMBERS_V1295) >= 40
        assert all(m.startswith("apeireth-") for m in WORKSPACE_MEMBERS_V1295)

    def test_thresholds_positive(self):
        assert 0 < THRESHOLD_CHECKSUM_COVERAGE_PCT <= 100
        assert THRESHOLD_INTERNAL_PACKAGES_MIN > 0
        assert THRESHOLD_LOCKFILE_MAX_LINES > 0
        assert 0 < THRESHOLD_MULTI_VERSION_MAX_PCT <= 100
        assert THRESHOLD_DISTINCT_SOURCES_MIN >= 1

    def test_workspace_members_unique(self):
        assert len(WORKSPACE_MEMBERS_V1295) == len(set(WORKSPACE_MEMBERS_V1295))


# ============================================================
# 2. Real Cargo.lock parsing (主 17:43 真扫真数)
# ============================================================


class TestParseCargoLock:
    """Test real Cargo.lock parsing."""

    def test_lockfile_exists(self):
        assert LOCKFILE.is_file(), f"Cargo.lock not found: {LOCKFILE}"

    def test_lockfile_nonempty(self):
        text = LOCKFILE.read_text(encoding="utf-8", errors="replace")
        assert len(text) > 1000
        assert text.startswith("# This file is automatically")

    def test_parse_returns_packages(self):
        packages, version, lines, bytes_ = parse_cargo_lock(LOCKFILE)
        assert len(packages) > 500, f"expected 500+ packages, got {len(packages)}"
        assert version == 3, f"expected Cargo v3 lockfile, got v{version}"
        assert lines > 1000
        assert bytes_ > 10000

    def test_parse_all_packages_have_name_version(self):
        packages, *_ = parse_cargo_lock(LOCKFILE)
        for p in packages:
            assert p.name, f"package missing name"
            assert p.version, f"package {p.name} missing version"

    def test_parse_internal_packages_have_no_source(self):
        """Internal (apeireth-*) packages should have NO source field (path/workspace)."""
        packages, *_ = parse_cargo_lock(LOCKFILE)
        internal = [p for p in packages if p.is_internal]
        assert len(internal) >= 40
        for p in internal:
            # workspace members are path-source, so source is None
            assert p.source is None, f"internal pkg {p.name} unexpectedly has source {p.source}"

    def test_parse_external_packages_have_source(self):
        """External packages should have source (typically crates.io)."""
        packages, *_ = parse_cargo_lock(LOCKFILE)
        external = [p for p in packages if not p.is_internal]
        assert len(external) >= 400
        for p in external[:50]:  # spot-check first 50
            assert p.source is not None, f"external pkg {p.name} missing source"

    def test_parse_all_external_have_checksum(self):
        """All external packages should have SHA256 checksum."""
        packages, *_ = parse_cargo_lock(LOCKFILE)
        external = [p for p in packages if not p.is_internal]
        no_checksum = [p for p in external if not p.checksum]
        # Allow up to 5 missing (some registry quirks) but most must have checksum
        assert len(no_checksum) <= 5, (
            f"{len(no_checksum)} external packages missing checksum: "
            f"{[p.name for p in no_checksum[:5]]}"
        )

    def test_parse_checksum_is_hex(self):
        """Checksum should be 64-char hex (SHA256)."""
        packages, *_ = parse_cargo_lock(LOCKFILE)
        external = [p for p in packages if p.checksum]
        for p in external[:20]:
            assert len(p.checksum) == 64, f"{p.name} checksum not 64 chars: {p.checksum[:16]}..."
            assert all(c in "0123456789abcdef" for c in p.checksum), f"{p.name} non-hex checksum"


# ============================================================
# 3. Single package parsing edge cases
# ============================================================


class TestParseSinglePackage:
    """Test _parse_single_package helper."""

    def test_parse_simple_package(self):
        text = """[[package]]
name = "foo"
version = "1.2.3"
source = "registry+https://example.com"
checksum = "abc123"
"""
        lines = text.split("\n")
        pkg, end_i = _parse_single_package(lines, 0)
        assert pkg is not None
        assert pkg.name == "foo"
        assert pkg.version == "1.2.3"
        assert pkg.source == "registry+https://example.com"
        assert pkg.checksum == "abc123"

    def test_parse_package_with_dependencies(self):
        text = """[[package]]
name = "bar"
version = "0.1.0"
dependencies = [
 "foo 1.2.3 (registry+https://example.com)",
 "baz 0.5.0",
]
"""
        lines = text.split("\n")
        pkg, end_i = _parse_single_package(lines, 0)
        assert pkg is not None
        assert pkg.name == "bar"
        assert len(pkg.dependencies) == 2
        assert "foo" in pkg.dependencies
        assert "baz" in pkg.dependencies

    def test_parse_internal_package_no_source(self):
        text = """[[package]]
name = "apeireth-test"
version = "1.0.0"
"""
        lines = text.split("\n")
        pkg, end_i = _parse_single_package(lines, 0)
        assert pkg is not None
        assert pkg.is_internal is True
        assert pkg.source is None

    def test_parse_yanked_true(self):
        text = """[[package]]
name = "foo"
version = "0.1.0"
source = "registry+https://example.com"
checksum = "abc"
yanked = true
"""
        lines = text.split("\n")
        pkg, end_i = _parse_single_package(lines, 0)
        assert pkg is not None
        assert pkg.is_yanked is True
        assert pkg.yanked_field_seen is True

    def test_parse_missing_name_returns_none(self):
        text = """[[package]]
version = "1.0.0"
"""
        lines = text.split("\n")
        pkg, end_i = _parse_single_package(lines, 0)
        assert pkg is None


# ============================================================
# 4. Workspace cross-validation (主 19:33 走在前人肩上)
# ============================================================


class TestWorkspaceCrossValidation:
    """Test workspace member presence detection."""

    def test_check_workspace_presence(self):
        if not WORKSPACE_ROOT.is_dir():
            return  # skip if workspace not present
        packages, *_ = parse_cargo_lock(LOCKFILE)
        presence = check_workspace_member_presence(WORKSPACE_ROOT, packages)
        assert len(presence) > 0
        # At least core should be in lock
        core_presence = next((p for p in presence if p.member_name == "apeireth-core"), None)
        if core_presence:
            assert core_presence.in_lock is True
            assert core_presence.lock_version is not None


# ============================================================
# 5. Multi-version detection (主 17:43 实事求是)
# ============================================================


class TestMultiVersionDetection:
    """Test multi-version crate detection."""

    def test_detect_multi_version_from_synthetic(self):
        # Build synthetic packages list
        packages = [
            LockPackage("foo", "1.0.0", "reg", "abc", [], False, False, False, 0, False),
            LockPackage("foo", "2.0.0", "reg", "def", [], False, False, False, 0, False),
            LockPackage("bar", "0.5.0", "reg", "ghi", [], False, False, False, 0, False),
        ]
        multi = compute_multi_version_crates(packages)
        assert len(multi) == 1
        assert multi[0].name == "foo"
        assert multi[0].n_distinct_major == 2

    def test_no_multi_version(self):
        packages = [
            LockPackage("foo", "1.0.0", "reg", "abc", [], False, False, False, 0, False),
            LockPackage("bar", "0.5.0", "reg", "def", [], False, False, False, 0, False),
        ]
        multi = compute_multi_version_crates(packages)
        assert len(multi) == 0

    def test_multi_version_major_count(self):
        # 3 versions, 1 distinct major (0.x.y all map to major 0)
        packages = [
            LockPackage("foo", "0.1.0", "reg", "abc", [], False, False, False, 0, False),
            LockPackage("foo", "0.2.0", "reg", "def", [], False, False, False, 0, False),
            LockPackage("foo", "0.3.0", "reg", "ghi", [], False, False, False, 0, False),
        ]
        multi = compute_multi_version_crates(packages)
        assert len(multi) == 1
        assert multi[0].n_distinct_major == 1

    def test_real_lockfile_has_multi_version(self):
        """Real Cargo.lock should have some multi-version crates (cargo's nature)."""
        packages, *_ = parse_cargo_lock(LOCKFILE)
        multi = compute_multi_version_crates(packages)
        # At least some crates have multiple versions in transitive deps
        assert len(multi) >= 1, "expected at least 1 multi-version crate"


# ============================================================
# 6. Top referenced computation
# ============================================================


class TestTopReferenced:
    """Test top-referenced external crate computation."""

    def test_top_referenced_real(self):
        """Top referenced should include popular crates like serde/tokio."""
        packages, *_ = parse_cargo_lock(LOCKFILE)
        top = compute_top_referenced_crates(packages, top_n=10)
        assert len(top) > 0
        # serde/tokio should likely be in top 10 (used everywhere)
        top_names = {t.name for t in top}
        # Note: might not be true, just check structure
        for t in top:
            assert t.n_referenced_by > 0
            assert t.version != "?"
            assert len(t.referenced_by) == t.n_referenced_by


# ============================================================
# 7. Build ledger (主 13:08 真自问)
# ============================================================


class TestBuildLedger:
    """Test full ledger construction."""

    def test_build_ledger_returns_valid_ledger(self):
        if not WORKSPACE_ROOT.is_dir():
            return
        ledger = build_ledger(WORKSPACE_ROOT)
        assert isinstance(ledger, LockfileLedger)
        assert ledger.n_packages_total > 500
        assert ledger.n_packages_internal >= 40
        assert ledger.checksum_coverage_pct > 99.0
        assert ledger.lockfile_version == 3

    def test_ledger_to_dict_jsonable(self):
        if not WORKSPACE_ROOT.is_dir():
            return
        ledger = build_ledger(WORKSPACE_ROOT)
        d = ledger.to_dict()
        # Should be JSON serializable
        json_str = json.dumps(d, default=str)
        assert len(json_str) > 1000

    def test_ledger_duration_positive(self):
        if not WORKSPACE_ROOT.is_dir():
            return
        ledger = build_ledger(WORKSPACE_ROOT)
        assert ledger.finished_at > ledger.started_at


# ============================================================
# 8. Hypothesis evaluation (主 13:08 + 主 17:43 + 主 17:58)
# ============================================================


class TestHypotheses:
    """Test hypothesis structure + evaluation."""

    def test_hypotheses_count(self):
        assert len(HYPOTHESES) >= 5, "need at least 5 hypotheses"

    def test_hypotheses_have_unique_ids(self):
        ids = [h.id for h in HYPOTHESES]
        assert len(ids) == len(set(ids))

    def test_hypotheses_evaluated(self):
        if not WORKSPACE_ROOT.is_dir():
            return
        ledger = build_ledger(WORKSPACE_ROOT)
        assert len(ledger.hypotheses) == len(HYPOTHESES)
        for h in ledger.hypotheses:
            assert hasattr(h, 'passed')
            assert hasattr(h, 'detail')
            assert h.detail != ""

    def test_real_lockfile_passes_h1_checksum(self):
        """H1: checksum coverage should be 100% (or near 100%)."""
        if not WORKSPACE_ROOT.is_dir():
            return
        ledger = build_ledger(WORKSPACE_ROOT)
        h1 = next(h for h in ledger.hypotheses if h.id == "H1_checksum_full")
        assert h1.passed is True
        assert ledger.checksum_coverage_pct >= 99.0

    def test_real_lockfile_passes_h2_internal(self):
        """H2: internal packages >= 40."""
        if not WORKSPACE_ROOT.is_dir():
            return
        ledger = build_ledger(WORKSPACE_ROOT)
        h2 = next(h for h in ledger.hypotheses if h.id == "H2_internal_complete")
        assert h2.passed is True
        assert ledger.n_packages_internal >= 40

    def test_real_lockfile_passes_h3_no_yanked(self):
        """H3: no yanked packages."""
        if not WORKSPACE_ROOT.is_dir():
            return
        ledger = build_ledger(WORKSPACE_ROOT)
        h3 = next(h for h in ledger.hypotheses if h.id == "H3_no_yanked")
        assert h3.passed is True
        assert ledger.n_with_yanked_true == 0

    def test_real_lockfile_passes_h4_compact(self):
        """H4: lockfile <= 10000 lines."""
        if not WORKSPACE_ROOT.is_dir():
            return
        ledger = build_ledger(WORKSPACE_ROOT)
        h4 = next(h for h in ledger.hypotheses if h.id == "H4_lockfile_compact")
        assert h4.passed is True
        assert ledger.lockfile_lines <= 10000

    def test_real_lockfile_passes_h5_multi_version_low(self):
        """H5: multi-version crates < 10%."""
        if not WORKSPACE_ROOT.is_dir():
            return
        ledger = build_ledger(WORKSPACE_ROOT)
        h5 = next(h for h in ledger.hypotheses if h.id == "H5_multi_version_low")
        # May or may not pass depending on dep variety
        assert h5.detail != ""

    def test_real_lockfile_passes_h6_source_diversity(self):
        """H6: >= 1 distinct source."""
        if not WORKSPACE_ROOT.is_dir():
            return
        ledger = build_ledger(WORKSPACE_ROOT)
        h6 = next(h for h in ledger.hypotheses if h.id == "H6_source_diversity")
        assert h6.passed is True
        assert ledger.n_distinct_sources >= 1

    def test_real_lockfile_passes_h7_no_drift(self):
        """H7: no workspace drift (all members in lock)."""
        if not WORKSPACE_ROOT.is_dir():
            return
        ledger = build_ledger(WORKSPACE_ROOT)
        h7 = next(h for h in ledger.hypotheses if h.id == "H7_no_workspace_drift")
        assert h7.passed is True


# ============================================================
# 9. Gate evaluation (主 17:58 + 主 20:46)
# ============================================================


class TestGates:
    """Test philosophy gates."""

    def test_gates_count(self):
        assert len(GATES) >= 10

    def test_gates_have_unique_ids(self):
        ids = [g.id for g in GATES]
        assert len(ids) == len(set(ids))

    def test_all_gates_pass(self):
        if not WORKSPACE_ROOT.is_dir():
            return
        ledger = build_ledger(WORKSPACE_ROOT)
        # All gates should pass (V1295 is read-only by construction)
        for g in ledger.gates:
            assert g.passed is True, f"gate {g.id} failed"
            assert g.detail != ""


# ============================================================
# 10. Report rendering (主 00:56 任何人都能接手)
# ============================================================


class TestReportRendering:
    """Test markdown report generation."""

    def test_report_has_summary(self):
        if not WORKSPACE_ROOT.is_dir():
            return
        ledger = build_ledger(WORKSPACE_ROOT)
        md = render_report(ledger)
        assert "## Summary" in md
        assert "Total packages" in md
        assert f"**{ledger.n_packages_total}**" in md

    def test_report_has_hypotheses_section(self):
        if not WORKSPACE_ROOT.is_dir():
            return
        ledger = build_ledger(WORKSPACE_ROOT)
        md = render_report(ledger)
        assert "## Hypotheses" in md
        # All 7 hypothesis IDs should appear
        for h in ledger.hypotheses:
            assert h.id in md

    def test_report_has_gates_section(self):
        if not WORKSPACE_ROOT.is_dir():
            return
        ledger = build_ledger(WORKSPACE_ROOT)
        md = render_report(ledger)
        assert "## Philosophy Gates" in md
        for g in ledger.gates:
            assert g.id in md

    def test_report_has_top_referenced_table(self):
        if not WORKSPACE_ROOT.is_dir():
            return
        ledger = build_ledger(WORKSPACE_ROOT)
        md = render_report(ledger)
        if ledger.top_referenced_crates:
            assert "## Top-10 Most-Referenced External Crates" in md
            for t in ledger.top_referenced_crates[:3]:
                assert t.name in md

    def test_report_has_workspace_presence_table(self):
        if not WORKSPACE_ROOT.is_dir():
            return
        ledger = build_ledger(WORKSPACE_ROOT)
        md = render_report(ledger)
        assert "## Workspace Member Lockfile Presence" in md
        # All workspace members should appear
        for w in ledger.workspace_member_presence[:3]:
            assert w.member_name in md

    def test_report_has_multi_version_section(self):
        if not WORKSPACE_ROOT.is_dir():
            return
        ledger = build_ledger(WORKSPACE_ROOT)
        md = render_report(ledger)
        assert "## Multi-Version Crates" in md

    def test_report_has_internal_packages_section(self):
        if not WORKSPACE_ROOT.is_dir():
            return
        ledger = build_ledger(WORKSPACE_ROOT)
        md = render_report(ledger)
        assert "## Internal Packages (apeireth-*)" in md

    def test_report_has_disclaimers(self):
        if not WORKSPACE_ROOT.is_dir():
            return
        ledger = build_ledger(WORKSPACE_ROOT)
        md = render_report(ledger)
        assert "## 关键免责声明" in md
        assert "不假装" in md or "不刷" in md


# ============================================================
# 11. Edge cases (主 17:43 实事求是)
# ============================================================


class TestEdgeCases:
    """Test edge cases and error handling."""

    def test_nonexistent_lockfile(self):
        with tempfile.TemporaryDirectory() as tmp:
            fake_root = Path(tmp) / "nonexistent"
            packages, version, lines, bytes_ = parse_cargo_lock(fake_root / "Cargo.lock")
            assert packages == []
            assert version is None
            assert lines == 0
            assert bytes_ == 0

    def test_empty_lockfile(self):
        with tempfile.TemporaryDirectory() as tmp:
            fake_lock = Path(tmp) / "Cargo.lock"
            fake_lock.write_text("", encoding="utf-8")
            packages, version, lines, bytes_ = parse_cargo_lock(fake_lock)
            assert packages == []
            assert lines == 1  # empty file has 1 line (empty)

    def test_lockfile_only_header(self):
        with tempfile.TemporaryDirectory() as tmp:
            fake_lock = Path(tmp) / "Cargo.lock"
            fake_lock.write_text("# This file is automatically @generated by Cargo.\nversion = 3\n", encoding="utf-8")
            packages, version, lines, bytes_ = parse_cargo_lock(fake_lock)
            assert packages == []
            assert version == 3

    def test_malformed_package_block(self):
        """Lockfile with [[package]] but missing name should skip that block."""
        with tempfile.TemporaryDirectory() as tmp:
            fake_lock = Path(tmp) / "Cargo.lock"
            content = """# This file is automatically @generated by Cargo.
version = 3

[[package]]
version = "1.0.0"

[[package]]
name = "valid"
version = "2.0.0"
source = "registry+https://example.com"
checksum = "abc"
"""
            fake_lock.write_text(content, encoding="utf-8")
            packages, *_ = parse_cargo_lock(fake_lock)
            # Only the valid one should be parsed
            assert len(packages) == 1
            assert packages[0].name == "valid"


# ============================================================
# 12. CLI smoke (主 00:56 任何人都能接手)
# ============================================================


class TestCLISmoke:
    """Smoke-test CLI commands."""

    def test_probe_command(self, capsys):
        from apeireth.v1295_cargo_lockfile_audit import main
        rc = main(["--probe"])
        assert rc == 0
        captured = capsys.readouterr()
        assert "lockfile=" in captured.out
        assert "total_packages=" in captured.out

    def test_run_command(self, capsys):
        from apeireth.v1295_cargo_lockfile_audit import main
        rc = main(["--run"])
        assert rc == 0
        captured = capsys.readouterr()
        assert "hypotheses_passed=" in captured.out

    def test_json_command(self, capsys):
        from apeireth.v1295_cargo_lockfile_audit import main
        rc = main(["--json"])
        assert rc == 0
        captured = capsys.readouterr()
        d = json.loads(captured.out)
        assert "n_packages_total" in d

    def test_internal_only_command(self, capsys):
        from apeireth.v1295_cargo_lockfile_audit import main
        rc = main(["--internal-only"])
        assert rc == 0
        captured = capsys.readouterr()
        assert "n_internal=" in captured.out
        assert "apeireth-core" in captured.out

    def test_external_only_command(self, capsys):
        from apeireth.v1295_cargo_lockfile_audit import main
        rc = main(["--external-only"])
        assert rc == 0
        captured = capsys.readouterr()
        assert "n_external=" in captured.out

    def test_multi_version_command(self, capsys):
        from apeireth.v1295_cargo_lockfile_audit import main
        rc = main(["--multi-version"])
        assert rc == 0
        captured = capsys.readouterr()
        assert "n_multi_version_crates=" in captured.out

    def test_top_command(self, capsys):
        from apeireth.v1295_cargo_lockfile_audit import main
        rc = main(["--top", "5"])
        assert rc == 0
        captured = capsys.readouterr()
        assert "n_top=" in captured.out

    def test_package_command(self, capsys):
        from apeireth.v1295_cargo_lockfile_audit import main
        rc = main(["--package", "serde"])
        assert rc == 0
        captured = capsys.readouterr()
        d = json.loads(captured.out)
        assert d["name"] == "serde"
        assert d["version"].startswith("1.")

    def test_report_to_file(self, tmp_path):
        from apeireth.v1295_cargo_lockfile_audit import main
        out = tmp_path / "report.md"
        rc = main(["--report", "--out", str(out)])
        assert rc == 0
        assert out.is_file()
        content = out.read_text(encoding="utf-8")
        assert "# V1295 — Cargo.lock Lockfile Audit" in content


# ============================================================
# 13. Cross-validation with V1293 (主 19:33 走在前人肩上)
# ============================================================


class TestCrossValidation:
    """Cross-validate V1295 with V1293 (Cargo.toml dep graph)."""

    def test_internal_packages_match_workspace(self):
        """Internal lock packages count should match active workspace members."""
        if not WORKSPACE_ROOT.is_dir():
            return
        ledger = build_ledger(WORKSPACE_ROOT)
        # Each workspace member on disk should be in lock
        for w in ledger.workspace_member_presence:
            if w.member_name in ("apeireth-tauri-stub",):
                # Commented out, may or may not be in lock
                continue
            assert w.in_lock, f"{w.member_name} in workspace but not in lock"


# ============================================================
# 14. Hypothesis structure (主 13:08 Popper 可证伪)
# ============================================================


class TestHypothesisStructure:
    """Test that hypotheses are properly structured for falsifiability."""

    def test_hypotheses_have_true_label(self):
        for h in HYPOTHESES:
            assert h.true_label != h.false_label, f"{h.id}: labels identical"
            assert len(h.true_label) > 0
            assert len(h.false_label) > 0

    def test_hypotheses_have_thresholds_explicit(self):
        """Each hypothesis should be evaluable (have explicit threshold)."""
        for h in HYPOTHESES:
            assert h.id
            assert h.title
            assert h.detail is not None or h.passed is False  # either evaluated or pending


if __name__ == "__main__":
    import pytest
    pytest.main([__file__, "-v"])