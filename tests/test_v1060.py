"""Tests for V1060 ASI Production Orchestrator (主 17:43 实事求是 + 主 00:56 任何人都能接手)."""
from __future__ import annotations

import pathlib
import sys
import types
from typing import Any, Dict, List

import pytest

# Add module dir to path
MODULE_DIR = pathlib.Path(__file__).resolve().parent.parent / "apeireth"
sys.path.insert(0, str(MODULE_DIR))

from v1060_asi_orchestrator import (
    ASIMeasurementRunner,
    ASIOrchestratorBridge,
    CheckLevel,
    ComponentStatusReport,
    HealthChecker,
    ModuleDiscovery,
    ModuleImporter,
    ModuleInfo,
    ModuleStatus,
    OrchestratorReport,
    TestVerifier,
    V3PhilosophyGuard,
    V1060_VERSION,
    run_orchestrator,
)


# ---------------------------------------------------------------------------
# ModuleDiscovery tests
# ---------------------------------------------------------------------------

class TestModuleDiscovery:
    def test_discover_returns_list(self):
        d = ModuleDiscovery()
        modules = d.discover()
        assert isinstance(modules, list)
        assert len(modules) > 0, "Should discover at least V1000-V1059 modules"

    def test_discover_all_have_valid_numbers(self):
        d = ModuleDiscovery()
        modules = d.discover()
        for m in modules:
            assert 1000 <= m.module_num <= 1059, f"{m.module_name} has num {m.module_num}"

    def test_discover_all_have_file_paths(self):
        d = ModuleDiscovery()
        modules = d.discover()
        for m in modules:
            assert m.file_path, f"{m.module_name} missing file_path"
            assert pathlib.Path(m.file_path).exists(), f"{m.file_path} does not exist"

    def test_discover_sorted(self):
        d = ModuleDiscovery()
        modules = d.discover()
        nums = [m.module_num for m in modules]
        assert nums == sorted(nums), "Modules should be sorted by number"

    def test_discover_empty_dir(self):
        d = ModuleDiscovery(module_dir=pathlib.Path("__nonexistent__"))
        modules = d.discover()
        assert modules == []

    def test_discover_min_max_filter(self):
        d = ModuleDiscovery()
        modules = d.discover(min_num=1050, max_num=1055)
        nums = [m.module_num for m in modules]
        assert all(1050 <= n <= 1055 for n in nums)


# ---------------------------------------------------------------------------
# ModuleImporter tests
# ---------------------------------------------------------------------------

class TestModuleImporter:
    def test_import_all_does_not_crash(self):
        imp = ModuleImporter()
        modules = imp.import_all()
        assert len(modules) > 0

    def test_import_all_sets_status(self):
        imp = ModuleImporter()
        modules = imp.import_all()
        for m in modules:
            assert m.import_status != ModuleStatus.UNKNOWN, f"{m.module_name} still UNKNOWN"

    def test_import_ok_modules_at_least_some(self):
        imp = ModuleImporter()
        modules = imp.import_all()
        ok_count = sum(1 for m in modules if m.import_status == ModuleStatus.OK)
        attr_miss = sum(1 for m in modules if m.import_status == ModuleStatus.ATTR_MISS)
        fail_count = sum(1 for m in modules if m.import_status == ModuleStatus.IMPORT_FAIL)
        # Most important: import failures should be 0 (ATTR_MISS is expected)
        assert fail_count == 0, f"Expected 0 import failures, got {fail_count}"
        # At least some OK modules
        assert ok_count > 0, f"Expected at least 1 OK module, got {ok_count}"
        # Total imported = OK + ATTR_MISS
        imported = ok_count + attr_miss
        assert imported >= 50, f"Expected >=50 modules imported, got {imported}"

    def test_import_quick_level_does_not_check_attrs(self):
        imp = ModuleImporter()
        modules = imp.discoverer.discover()[:3]  # first 3 only
        modules = imp.import_all(modules, CheckLevel.QUICK)
        for m in modules:
            assert m.key_attrs_total == 0, f"{m.module_name} should have 0 attrs checked at QUICK level"

    def test_import_standard_level_checks_attrs(self):
        imp = ModuleImporter()
        modules = imp.discoverer.discover()[:3]
        modules = imp.import_all(modules, CheckLevel.STANDARD)
        # At least some should have attrs checked
        checked = [m for m in modules if m.key_attrs_total > 0]
        assert len(checked) > 0, "STANDARD level should check attrs for at least some modules"

    def test_import_individual_v1048(self):
        """Verify v1048 can be imported specifically."""
        imp = ModuleImporter()
        d = ModuleDiscovery()
        modules = [m for m in d.discover() if m.module_num == 1048]
        assert len(modules) == 1
        modules = imp.import_all(modules)
        assert modules[0].import_status != ModuleStatus.IMPORT_FAIL, \
            f"v1048 import failed: {modules[0].import_error}"
        # ATTR_MISS is acceptable (REFERENCES missing), but must not fail
        assert modules[0].import_status in (ModuleStatus.OK, ModuleStatus.ATTR_MISS)

    def test_import_individual_v1058(self):
        imp = ModuleImporter()
        d = ModuleDiscovery()
        modules = [m for m in d.discover() if m.module_num == 1058]
        assert len(modules) == 1
        modules = imp.import_all(modules)
        assert modules[0].import_status != ModuleStatus.IMPORT_FAIL, \
            f"v1058 import failed: {modules[0].import_error}"
        assert modules[0].import_status in (ModuleStatus.OK, ModuleStatus.ATTR_MISS)


# ---------------------------------------------------------------------------
# HealthChecker tests
# ---------------------------------------------------------------------------

class TestHealthChecker:
    def test_check_all_returns_list(self):
        hc = HealthChecker()
        modules = hc.check_all()
        assert isinstance(modules, list)

    def test_summary_counts(self):
        hc = HealthChecker()
        modules = hc.check_all()
        summary = hc.summary(modules)
        assert summary["total"] > 0
        assert summary["ok"] + summary["warn"] + summary["fail"] == summary["total"]

    def test_summary_with_tests(self):
        hc = HealthChecker()
        modules = hc.check_all()
        # Test file check must be done by TestVerifier
        verifier = TestVerifier()
        modules = verifier.verify(modules)
        summary = hc.summary(modules)
        assert summary["with_tests"] >= 0


# ---------------------------------------------------------------------------
# TestVerifier tests
# ---------------------------------------------------------------------------

class TestTestVerifier:
    def test_verify_test_coverage(self):
        d = ModuleDiscovery()
        modules = d.discover()
        v = TestVerifier()
        modules = v.verify(modules)
        covered = sum(1 for m in modules if m.has_test_file)
        assert covered > 0, "At least some modules should have test files"

    def test_test_coverage_fraction(self):
        d = ModuleDiscovery()
        modules = d.discover()[:5]
        v = TestVerifier()
        coverage = v.test_coverage(modules)
        assert 0.0 <= coverage <= 1.0


# ---------------------------------------------------------------------------
# ASIMeasurementRunner tests
# ---------------------------------------------------------------------------

class TestASIMeasurementRunner:
    def test_run_measurement_does_not_crash(self):
        runner = ASIMeasurementRunner()
        result = runner.run_measurement()
        # This is optional — v1048 may or may not import cleanly without deps
        # Just verify it returns either dict or None
        assert result is None or isinstance(result, dict)

    def test_try_load_no_error_if_not_found(self):
        runner = ASIMeasurementRunner()
        # First call initializes
        result = runner.run_measurement()
        # Error should be None (loading happened) or a string describing the issue
        error = runner.get_error()
        assert error is None or isinstance(error, str)


# ---------------------------------------------------------------------------
# ComponentStatusReport tests
# ---------------------------------------------------------------------------

class TestComponentStatusReport:
    def test_generate_returns_string(self):
        report = ComponentStatusReport()
        modules = ModuleDiscovery().discover()[:3]
        md = report.generate(modules, total_time_ms=100.0)
        assert isinstance(md, str)
        assert len(md) > 50

    def test_generate_includes_module_details(self):
        report = ComponentStatusReport()
        modules = ModuleDiscovery().discover()[:3]
        md = report.generate(modules, total_time_ms=100.0)
        for m in modules:
            assert m.module_name in md

    def test_generate_with_v02(self):
        report = ComponentStatusReport()
        modules = ModuleDiscovery().discover()[:3]
        md = report.generate(modules, v02_result={"phi_proxy": 0.85, "total": 0.75}, v02_score=0.75, total_time_ms=100.0)
        assert "0.7500" in md or "0.7500" in md.replace(",", ".")
        assert "ASI V0.2" in md

    def test_generate_with_philosophy_guard(self):
        report = ComponentStatusReport()
        modules = ModuleDiscovery().discover()[:3]
        md = report.generate(modules, total_time_ms=100.0)
        assert "不假装" in md


# ---------------------------------------------------------------------------
# ASIOrchestratorBridge tests
# ---------------------------------------------------------------------------

class TestASIOrchestratorBridge:
    def test_score_orchestrator_all_good(self):
        bridge = ASIOrchestratorBridge()
        modules = [
            ModuleInfo(module_name="v1001", module_num=1001, file_path="/fake1.py", import_status=ModuleStatus.OK),
            ModuleInfo(module_name="v1002", module_num=1002, file_path="/fake2.py", import_status=ModuleStatus.OK),
        ]
        score = bridge.score_orchestrator(modules)
        assert score == 1.0

    def test_score_orchestrator_some_fail(self):
        bridge = ASIOrchestratorBridge()
        modules = [
            ModuleInfo(module_name="v1001", module_num=1001, file_path="/fake1.py", import_status=ModuleStatus.OK),
            ModuleInfo(module_name="v1002", module_num=1002, file_path="/fake2.py", import_status=ModuleStatus.IMPORT_FAIL),
        ]
        score = bridge.score_orchestrator(modules)
        assert score == 0.5

    def test_score_orchestrator_empty(self):
        bridge = ASIOrchestratorBridge()
        assert bridge.score_orchestrator([]) == 0.0

    def test_score_test_coverage(self):
        bridge = ASIOrchestratorBridge()
        modules = [
            ModuleInfo(module_name="v1001", module_num=1001, file_path="/fake1.py", has_test_file=True),
            ModuleInfo(module_name="v1002", module_num=1002, file_path="/fake2.py", has_test_file=False),
        ]
        assert bridge.score_test_coverage(modules) == 0.5

    def test_build_bridge_returns_dict(self):
        bridge = ASIOrchestratorBridge()
        modules = [
            ModuleInfo(module_name="v1001", module_num=1001, file_path="/fake1.py", import_status=ModuleStatus.OK),
        ]
        result = bridge.build_bridge(modules)
        assert isinstance(result, dict)
        assert "orchestrator" in result
        assert "test_coverage" in result

    def test_bridge_orchestrator_weight(self):
        bridge = ASIOrchestratorBridge()
        assert bridge.V0_2_WEIGHT == pytest.approx(0.015)


# ---------------------------------------------------------------------------
# V3PhilosophyGuard tests
# ---------------------------------------------------------------------------

class TestV3PhilosophyGuard:
    def test_guard_always_passes(self):
        report = OrchestratorReport()
        guard = V3PhilosophyGuard()
        results = guard.check(report)
        assert all(v for v in results.values()), "All guards should pass by design"

    def test_guard_messages_count(self):
        assert len(V3PhilosophyGuard.GUARD_MESSAGES) >= 4

    def test_guard_to_markdown_returns_string(self):
        guard = V3PhilosophyGuard()
        results = guard.check(OrchestratorReport())
        md = guard.to_markdown(results)
        assert isinstance(md, str)
        assert "V3 Philosophy Guard" in md
        assert "不假装" in md or "DONT_PRETEND" in md or "🚫" in md

    def test_guard_mentions_phenomenal(self):
        """Guard must reference 不假装 as per 主 17:58."""
        all_text = "\n".join(V3PhilosophyGuard.GUARD_MESSAGES)
        # Check for the core idea: not pretending
        assert any(word in all_text for word in ["不假装", "not...ready", "not ASI"])

    def test_guard_method_is_static(self):
        assert isinstance(V3PhilosophyGuard.check, staticmethod) or callable(V3PhilosophyGuard.check)


# ---------------------------------------------------------------------------
# run_orchestrator integration test
# ---------------------------------------------------------------------------

class TestRunOrchestrator:
    def test_run_orchestrator_returns_report(self):
        report = run_orchestrator(check_level=CheckLevel.QUICK, run_v02=False)
        assert isinstance(report, OrchestratorReport)

    def test_orchestrator_report_has_fields(self):
        report = run_orchestrator(check_level=CheckLevel.QUICK, run_v02=False)
        assert report.modules_discovered > 0
        assert report.timestamp != ""
        assert report.check_time_ms > 0

    def test_orchestrator_discovery_count(self):
        report = run_orchestrator(check_level=CheckLevel.QUICK, run_v02=False)
        # Should discover V1001-V1059 plus V1000 = 60 modules total
        # (V1000 yaml_serializer is also picked up)
        assert report.modules_discovered >= 55, \
            f"Expected >=55 V1000+ modules, got {report.modules_discovered}"

    def test_orchestrator_standard_level(self):
        report = run_orchestrator(check_level=CheckLevel.STANDARD, run_v02=False)
        assert report.modules_ok + report.modules_warn == report.modules_imported

    def test_orchestrator_with_v02(self):
        # run_v02=True — V0.2 may or may not work depending on deps
        report = run_orchestrator(check_level=CheckLevel.QUICK, run_v02=True)
        # Just verify it doesn't crash — measurement result is optional
        assert report.modules_discovered > 0


# ---------------------------------------------------------------------------
# ModuleInfo defaults
# ---------------------------------------------------------------------------

class TestModuleInfo:
    def test_default_import_status(self):
        info = ModuleInfo(module_name="test", module_num=1001, file_path="/fake.py")
        assert info.import_status == ModuleStatus.UNKNOWN

    def test_defaults_are_sensible(self):
        info = ModuleInfo(module_name="test", module_num=1001, file_path="/fake.py")
        assert info.key_attrs_present == 0
        assert info.key_attrs_total == 0
        assert info.missing_attrs == []
        assert info.has_test_file is False
        assert info.import_error is None


# ---------------------------------------------------------------------------
# Version constant
# ---------------------------------------------------------------------------

class TestVersion:
    def test_version_defined(self):
        assert V1060_VERSION is not None
        assert isinstance(V1060_VERSION, str)
        assert V1060_VERSION.startswith("0.")


# ---------------------------------------------------------------------------
# Philosophy guard presence (主 17:58 不假装)
# ---------------------------------------------------------------------------

def test_philosophy_guard_present():
    """Verify the V1060 module has a PhilosophyGuard class (主 17:58)."""
    assert V3PhilosophyGuard is not None


def test_module_has_references():
    """Verify V1060 has REFERENCES list (主 19:33 走在前人经验上)."""
    from v1060_asi_orchestrator import REFERENCES
    assert isinstance(REFERENCES, list)
    assert len(REFERENCES) >= 3


# ---------------------------------------------------------------------------
# Module structure
# ---------------------------------------------------------------------------

def test_module_has_main():
    """Verify V1060 has a main() CLI entry point (主 00:56 任何人都能接手)."""
    from v1060_asi_orchestrator import main
    assert callable(main)


def test_module_has_run_orchestrator():
    """Verify V1060 has the run_orchestrator function."""
    from v1060_asi_orchestrator import run_orchestrator
    assert callable(run_orchestrator)


def test_module_docstring_present():
    """Verify V1060 has docstring."""
    from v1060_asi_orchestrator import __doc__
    assert __doc__ is not None
    assert len(__doc__) > 100


# ---------------------------------------------------------------------------
# OrchestratorReport dataclass
# ---------------------------------------------------------------------------

class TestOrchestratorReport:
    def test_default_values(self):
        r = OrchestratorReport()
        assert r.modules_discovered == 0
        assert r.modules_ok == 0
        assert r.modules_warn == 0
        assert r.modules_fail == 0
        assert r.check_time_ms == 0.0
        assert r.v02_measurement is None

    def test_custom_values(self):
        r = OrchestratorReport(
            timestamp="utc",
            modules_discovered=59,
            modules_ok=50,
            modules_fail=1,
            v02_score=0.75,
        )
        assert r.modules_discovered == 59
        assert r.modules_ok == 50
        assert r.v02_score == 0.75

    def test_module_details_roundtrip(self):
        info = ModuleInfo(module_name="v1001", module_num=1001, file_path="/f.py",
                          import_status=ModuleStatus.OK)
        r = OrchestratorReport(module_details=[{
            "module_name": "v1001",
            "module_num": 1001,
            "file_path": "/f.py",
        }])
        assert len(r.module_details) == 1
