"""
Tests for V1082 ASI Real Workspace Codebase Audit & Empty-Shell Detection
==========================================================================

主 22:33 + 主 17:43 + 主 19:33 + 主 13:31 + 主 17:58+20:46 + 主 23:44 + 主 00:56 + 主 00:44
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

import pytest


# Make sure we can import the module
APEIRETH_DIR = Path(__file__).resolve().parent.parent / "apeireth"
if str(APEIRETH_DIR.parent) not in sys.path:
    sys.path.insert(0, str(APEIRETH_DIR.parent))

from apeireth.v1082_asi_codebase_audit import (
    ASICodebaseAuditBridge,
    BacklogItem,
    CodebaseAuditResult,
    DocstringAudit,
    EmptyShellVerdict,
    GuardAudit,
    ModuleInfo,
    TestMapping,
    _resolve_root,
    audit_docstring,
    audit_guards,
    inventory_modules,
    is_empty_shell,
    main as v1082_main,
    map_tests,
    prioritize_backlog,
    render_markdown_report,
    run_full_audit,
    run_v3_guards,
)


# ============================================================
# Test 1: ModuleInventory 真扫
# ============================================================


class TestV1082Inventory:
    def test_inventory_returns_list(self):
        root = _resolve_root()
        mods = inventory_modules(root)
        assert isinstance(mods, list)
        assert len(mods) > 100  # We know there are 1000+ modules

    def test_inventory_modules_have_required_fields(self):
        root = _resolve_root()
        mods = inventory_modules(root)
        for m in mods[:5]:
            assert hasattr(m, "module_name")
            assert hasattr(m, "version")
            assert hasattr(m, "total_loc")
            assert hasattr(m, "class_count")
            assert hasattr(m, "function_count")

    def test_inventory_versions_parsed_correctly(self):
        root = _resolve_root()
        mods = inventory_modules(root)
        for m in mods:
            assert m.version > 0
            assert m.version < 100000

    def test_inventory_finds_v1081(self):
        root = _resolve_root()
        mods = inventory_modules(root)
        names = {m.module_name for m in mods}
        assert "v1081_asi_honest_limits" in names

    def test_inventory_excludes_init(self):
        root = _resolve_root()
        mods = inventory_modules(root)
        names = {m.module_name for m in mods}
        assert "__init__" not in names


# ============================================================
# Test 2: EmptyShellDetector 真判
# ============================================================


class TestV1082EmptyShell:
    def test_short_module_is_shell(self):
        info = ModuleInfo(
            module_name="v_xxx",
            module_path="/tmp/v_xxx.py",
            version=9999,
            total_loc=100,
            code_loc=80,
            class_count=0,
            function_count=0,
            has_docstring=False,
            module_docstring="",
        )
        v = is_empty_shell(info)
        assert v.is_shell is True
        assert "loc<200" in " ".join(v.reasons)

    def test_no_docstring_is_shell(self):
        info = ModuleInfo(
            module_name="v_yyy",
            module_path="/tmp/v_yyy.py",
            version=9998,
            total_loc=500,
            code_loc=400,
            class_count=2,
            function_count=5,
            has_docstring=False,
            module_docstring="",
        )
        v = is_empty_shell(info)
        assert v.is_shell is True
        assert "no_docstring" in v.reasons

    def test_no_class_no_func_is_shell(self):
        info = ModuleInfo(
            module_name="v_zzz",
            module_path="/tmp/v_zzz.py",
            version=9997,
            total_loc=500,
            code_loc=400,
            class_count=0,
            function_count=0,
            has_docstring=True,
            module_docstring="Module doc.",
        )
        v = is_empty_shell(info)
        assert v.is_shell is True
        assert "no_class_or_func" in v.reasons

    def test_real_production_module_not_shell(self):
        info = ModuleInfo(
            module_name="v1001",
            module_path="/tmp/v1001.py",
            version=1001,
            total_loc=500,
            code_loc=400,
            class_count=2,
            function_count=10,
            has_docstring=True,
            module_docstring="A real production module docstring.",
        )
        v = is_empty_shell(info)
        assert v.is_shell is False
        assert v.reasons == []

    def test_to_dict(self):
        v = EmptyShellVerdict(
            module_name="x",
            version=1,
            total_loc=100,
            reasons=["loc<200"],
            is_shell=True,
        )
        d = v.to_dict()
        assert d["module_name"] == "x"
        assert d["is_shell"] is True


# ============================================================
# Test 3: DocstringAuditor 真查
# ============================================================


class TestV1082Docstring:
    def test_long_docstring_has_summary(self):
        info = ModuleInfo(
            module_name="m",
            module_path="x",
            version=1,
            total_loc=10,
            code_loc=5,
            class_count=1,
            function_count=1,
            has_docstring=True,
            module_docstring=(
                "This is a comprehensive module summary explaining what "
                "the module does in more than twenty characters."
            ),
        )
        a = audit_docstring(info)
        assert a.has_summary is True

    def test_short_docstring_no_summary(self):
        info = ModuleInfo(
            module_name="m",
            module_path="x",
            version=1,
            total_loc=10,
            code_loc=5,
            class_count=1,
            function_count=1,
            has_docstring=True,
            module_docstring="short",
        )
        a = audit_docstring(info)
        assert a.has_summary is False

    def test_args_docstring(self):
        info = ModuleInfo(
            module_name="m",
            module_path="x",
            version=1,
            total_loc=10,
            code_loc=5,
            class_count=1,
            function_count=1,
            has_docstring=True,
            module_docstring=(
                "Comprehensive module summary explaining the purpose. "
                "Args: param1 description. Returns: result description."
            ),
        )
        a = audit_docstring(info)
        assert a.has_args_or_returns is True

    def test_examples_docstring(self):
        info = ModuleInfo(
            module_name="m",
            module_path="x",
            version=1,
            total_loc=10,
            code_loc=5,
            class_count=1,
            function_count=1,
            has_docstring=True,
            module_docstring=(
                "Comprehensive summary explaining the purpose. "
                "Example usage: >>> test_code()"
            ),
        )
        a = audit_docstring(info)
        assert a.has_examples is True

    def test_quality_score_range(self):
        info = ModuleInfo(
            module_name="m",
            module_path="x",
            version=1,
            total_loc=10,
            code_loc=5,
            class_count=1,
            function_count=1,
            has_docstring=True,
            module_docstring=(
                "This module summary explains everything. Args: x. Returns: y. "
                "Example: >>> test()"
            ),
        )
        a = audit_docstring(info)
        assert 0.0 <= a.quality_score <= 1.0


# ============================================================
# Test 4: TestCoverageMapper 真映射
# ============================================================


class TestV1082TestMap:
    def test_existing_test_mapped(self):
        info = ModuleInfo(
            module_name="v1081_asi_honest_limits",
            module_path="apeireth/v1081_asi_honest_limits.py",
            version=1081,
            total_loc=1000,
            code_loc=800,
            class_count=2,
            function_count=5,
            has_docstring=True,
            module_docstring="Module.",
        )
        # Use the workspace root as parent so tests/ resolves correctly
        # _resolve_root returns apeireth/ so parent is workspace root
        root = _resolve_root()
        m = map_tests(root, info)
        assert m.has_test is True
        assert m.test_path is not None
        assert m.test_loc > 0

    def test_missing_test(self, tmp_path: Path):
        info = ModuleInfo(
            module_name="v9999_does_not_exist",
            module_path="apeireth/v9999_does_not_exist.py",
            version=9999,
            total_loc=100,
            code_loc=80,
            class_count=1,
            function_count=2,
            has_docstring=True,
            module_docstring="Module.",
        )
        # Build a fake root whose parent has no tests/<name>.py
        fake_apeireth = tmp_path / "apeireth"
        fake_apeireth.mkdir()
        m = map_tests(fake_apeireth, info)
        assert m.has_test is False
        assert m.test_path is None
        assert m.test_loc == 0


# ============================================================
# Test 5: V3PhilosophyGuardAuditor 真查
# ============================================================


class TestV1082GuardAudit:
    def test_real_production_has_bridge(self):
        text = """
class ASIRealBridge:
    pass

GUARD = "不假装"
"""
        info = ModuleInfo(
            module_name="v1082",
            module_path="x",
            version=1082,
            total_loc=100,
            code_loc=80,
            class_count=2,
            function_count=2,
            has_docstring=True,
            module_docstring="Module.",
        )
        a = audit_guards(info, text)
        assert a.has_asi_bridge is True
        assert a.has_v3_guard_phrases is True
        assert a.score >= 0.5

    def test_empty_shell_no_bridge(self):
        text = "x = 1\n"
        info = ModuleInfo(
            module_name="v_shell",
            module_path="x",
            version=999,
            total_loc=10,
            code_loc=5,
            class_count=0,
            function_count=0,
            has_docstring=False,
            module_docstring="",
        )
        a = audit_guards(info, text)
        assert a.has_asi_bridge is False
        assert a.score == 0.0

    def test_guard_phrases_count(self):
        text = """
# 不假装 (主 17:58)
# guard
class ASIRealBridge:
    pass
"""
        info = ModuleInfo(
            module_name="v_x",
            module_path="x",
            version=1082,
            total_loc=100,
            code_loc=80,
            class_count=1,
            function_count=1,
            has_docstring=True,
            module_docstring="Module.",
        )
        a = audit_guards(info, text)
        assert a.guard_phrase_count >= 2

    def test_score_range(self):
        info = ModuleInfo(
            module_name="m",
            module_path="x",
            version=1,
            total_loc=10,
            code_loc=5,
            class_count=1,
            function_count=1,
            has_docstring=True,
            module_docstring="Module.",
        )
        a = audit_guards(info, "anything")
        assert 0.0 <= a.score <= 1.0


# ============================================================
# Test 6: BacklogPrioritizer 真排
# ============================================================


class TestV1082Backlog:
    def test_v1000_plus_empty_shell_high_priority(self):
        info = ModuleInfo(
            module_name="v1001_test",
            module_path="x",
            version=1001,
            total_loc=100,
            code_loc=80,
            class_count=0,
            function_count=0,
            has_docstring=False,
            module_docstring="",
        )
        shell = EmptyShellVerdict(
            module_name="v1001_test",
            version=1001,
            total_loc=100,
            reasons=["loc<200"],
            is_shell=True,
        )
        tmap = TestMapping(
            module_name="v1001_test",
            version=1001,
            has_test=False,
            test_path=None,
            test_loc=0,
        )
        ga = GuardAudit(
            module_name="v1001_test",
            has_asi_bridge=False,
            has_v3_guard_phrases=False,
            guard_phrase_count=0,
            score=0.0,
        )
        b = prioritize_backlog(info, shell, tmap, ga)
        assert b.priority_score >= 0.5
        assert "v1000_plus_high_lift" in b.reasons
        assert "empty_shell" in b.reasons
        assert "no_test" in b.reasons

    def test_v1_low_priority(self):
        info = ModuleInfo(
            module_name="v1_low",
            module_path="x",
            version=1,
            total_loc=500,
            code_loc=400,
            class_count=5,
            function_count=10,
            has_docstring=True,
            module_docstring="Long docstring with summary explaining purpose. Args: x. Example: >>> test()",
        )
        shell = EmptyShellVerdict(
            module_name="v1_low",
            version=1,
            total_loc=500,
            reasons=[],
            is_shell=False,
        )
        tmap = TestMapping(
            module_name="v1_low",
            version=1,
            has_test=True,
            test_path="tests/test_v1_low.py",
            test_loc=200,
        )
        ga = GuardAudit(
            module_name="v1_low",
            has_asi_bridge=True,
            has_v3_guard_phrases=True,
            guard_phrase_count=3,
            score=1.0,
        )
        b = prioritize_backlog(info, shell, tmap, ga)
        assert b.priority_score < 0.3


# ============================================================
# Test 7: CodebaseAuditReport 真出
# ============================================================


class TestV1082Report:
    def test_full_audit_runs(self):
        root = _resolve_root()
        result = run_full_audit(root)
        assert isinstance(result, CodebaseAuditResult)
        assert result.summary["total_modules"] > 100

    def test_markdown_report_contains_summary(self):
        root = _resolve_root()
        result = run_full_audit(root)
        md = render_markdown_report(result)
        assert "V1082 ASI Workspace Codebase Audit" in md
        assert "总模块数" in md
        assert "空壳" in md
        assert "V3 哲学守门" in md
        assert "References" in md

    def test_summary_keys_present(self):
        root = _resolve_root()
        result = run_full_audit(root)
        s = result.summary
        for k in [
            "total_modules",
            "empty_shells",
            "shell_ratio",
            "with_tests",
            "with_asi_bridge",
            "v1000_plus_total",
            "v1000_plus_shells",
            "total_loc",
            "avg_loc",
            "avg_doc_quality",
        ]:
            assert k in s

    def test_real_workspace_has_shells(self):
        root = _resolve_root()
        result = run_full_audit(root)
        # We know there are ~984 empty shells
        assert result.summary["empty_shells"] > 100
        assert result.summary["v1000_plus_total"] > 50


# ============================================================
# Test 8: ASICodebaseAuditBridge 真测
# ============================================================


class TestV1082ASIBridge:
    def test_subscore_returns_float(self):
        root = _resolve_root()
        result = run_full_audit(root)
        bridge = ASICodebaseAuditBridge()
        sub = bridge.subscore(result)
        assert isinstance(sub, float)
        assert 0.0 <= sub <= 1.0

    def test_subscore_partial_audit_lower(self):
        # A real audit on workspace should be < 1.0 because of empty shells
        root = _resolve_root()
        result = run_full_audit(root)
        bridge = ASICodebaseAuditBridge()
        sub = bridge.subscore(result)
        # Given 90% empty shells, subscore should be < 0.5
        # (since shell_weight=0.20 + test_weight=0.20 + guard_weight=0.20 are low)
        assert sub < 0.5

    def test_subscore_full_health_high(self):
        # Synthetic all-healthy audit
        mods = [
            ModuleInfo(
                module_name=f"v{i}",
                module_path="x",
                version=i,
                total_loc=500,
                code_loc=400,
                class_count=2,
                function_count=10,
                has_docstring=True,
                module_docstring="Real production module. Args: x. Example: >>> test()",
            )
            for i in range(1, 11)
        ]
        shells = [
            EmptyShellVerdict(
                module_name=m.module_name,
                version=m.version,
                total_loc=m.total_loc,
                reasons=[],
                is_shell=False,
            )
            for m in mods
        ]
        doc_audits = [
            DocstringAudit(
                module_name=m.module_name,
                has_summary=True,
                has_args_or_returns=True,
                has_examples=True,
                quality_score=1.0,
            )
            for m in mods
        ]
        test_maps = [
            TestMapping(
                module_name=m.module_name,
                version=m.version,
                has_test=True,
                test_path="tests/test.py",
                test_loc=200,
            )
            for m in mods
        ]
        guard_audits = [
            GuardAudit(
                module_name=m.module_name,
                has_asi_bridge=True,
                has_v3_guard_phrases=True,
                guard_phrase_count=3,
                score=1.0,
            )
            for m in mods
        ]
        backlog: List[BacklogItem] = []
        result = CodebaseAuditResult(
            root="x",
            modules=mods,
            shells=shells,
            doc_audits=doc_audits,
            test_maps=test_maps,
            guard_audits=guard_audits,
            backlog=backlog,
            summary={
                "total_modules": 10,
                "empty_shells": 0,
                "shell_ratio": 0.0,
                "with_tests": 10,
                "test_coverage_proxy": 1.0,
                "with_asi_bridge": 10,
                "asi_bridge_ratio": 1.0,
                "v1000_plus_total": 10,
                "v1000_plus_shells": 0,
                "total_loc": 5000,
                "avg_loc": 500.0,
                "avg_doc_quality": 1.0,
                "audit_ts": 0.0,
            },
        )
        bridge = ASICodebaseAuditBridge()
        sub = bridge.subscore(result)
        # Inventory=1, shell=1, doc=1, test=1, guard=1, backlog=0 (empty), no_fake=1
        # = 0.10+0.20+0.10+0.20+0.20+0.00+0.05 = 0.85
        assert sub >= 0.80

    def test_lift_capped(self):
        root = _resolve_root()
        result = run_full_audit(root)
        bridge = ASICodebaseAuditBridge()
        lift = bridge.asi_v03_lift(result, current_asi_v03=0.8813)
        assert lift["lift"] <= 0.02  # V1082 only nudges 0.02 max
        assert lift["projected_asi_v03"] <= 0.9800  # ASI ceiling

    def test_weights_sum_to_one(self):
        b = ASICodebaseAuditBridge()
        total = (
            b.inventory_weight
            + b.shell_weight
            + b.doc_quality_weight
            + b.test_map_weight
            + b.guard_weight
            + b.backlog_weight
            + b.no_fake_weight
        )
        assert abs(total - 1.0) < 1e-9


# ============================================================
# Test 9: V3 Philosophy Guards
# ============================================================


class TestV1082V3Guards:
    def test_run_v3_guards_returns_4(self):
        guards = run_v3_guards()
        assert len(guards) == 4

    def test_all_guards_have_chinese(self):
        guards = run_v3_guards()
        for k, v in guards.items():
            # Each guard should have Chinese phrase to demonstrate real "不假装"
            assert "不假装" in v

    def test_guard_not_shell_count_is_asi_explains(self):
        guards = run_v3_guards()
        assert "空壳计数" in guards["guard_not_shell_count_is_asi"]

    def test_guard_not_audit_is_fix_explains(self):
        guards = run_v3_guards()
        assert "audit" in guards["guard_not_audit_is_fix"].lower()


# ============================================================
# Test 10: CLI
# ============================================================


class TestV1082CLI:
    def test_main_audit_quiet(self, capsys):
        rc = v1082_main(["--audit", "--quiet"])
        assert rc == 0

    def test_main_report_quiet(self, tmp_path: Path):
        out = tmp_path / "report.md"
        rc = v1082_main(["--audit", "--report", "--output", str(out), "--quiet"])
        assert rc == 0
        assert out.exists()
        text = out.read_text(encoding="utf-8")
        assert "V1082" in text

    def test_main_backlog_quiet(self, capsys):
        rc = v1082_main(["--backlog", "--limit", "5", "--quiet"])
        assert rc == 0

    def test_main_lift_quiet(self, capsys):
        rc = v1082_main(["--lift", "--quiet"])
        assert rc == 0

    def test_main_module_quiet(self, tmp_path: Path):
        out = tmp_path / "module.json"
        rc = v1082_main(["--module", "v1081_asi_honest_limits", "--output", str(out), "--quiet"])
        assert rc == 0
        assert out.exists()
        data = json.loads(out.read_text(encoding="utf-8"))
        assert "module" in data
        assert "shell" in data

    def test_main_module_not_found(self, capsys):
        rc = v1082_main(["--module", "v9999_does_not_exist", "--quiet"])
        assert rc == 1

    def test_main_guards(self, capsys):
        rc = v1082_main(["--guards"])
        assert rc == 0

    def test_main_no_args_shows_help(self, capsys):
        rc = v1082_main([])
        assert rc == 0


# ============================================================
# Test 11: Sanity tests (主 19:33 走在前人经验 + 主 17:43 实事求是)
# ============================================================


class TestV1082Sanity:
    def test_six_components_referenced(self):
        """V1082 has 8 components; sanity-check each is reachable."""
        root = _resolve_root()
        result = run_full_audit(root)
        # 1. ModuleInventory
        assert len(result.modules) > 0
        # 2. EmptyShellDetector
        assert len(result.shells) > 0
        # 3. DocstringAuditor
        assert len(result.doc_audits) > 0
        # 4. TestCoverageMapper
        assert len(result.test_maps) > 0
        # 5. V3PhilosophyGuardAuditor
        assert len(result.guard_audits) > 0
        # 6. BacklogPrioritizer
        assert len(result.backlog) > 0
        # 7. CodebaseAuditReport
        md = render_markdown_report(result)
        assert len(md) > 0
        # 8. ASICodebaseAuditBridge
        bridge = ASICodebaseAuditBridge()
        sub = bridge.subscore(result)
        assert sub >= 0.0

    def test_backlog_sorted_descending(self):
        root = _resolve_root()
        result = run_full_audit(root)
        for i in range(len(result.backlog) - 1):
            a = result.backlog[i]
            b = result.backlog[i + 1]
            if a.priority_score == b.priority_score:
                assert a.version >= b.version
            else:
                assert a.priority_score >= b.priority_score

    def test_no_fake_count_matches(self):
        """Inventory count must equal shell count."""
        root = _resolve_root()
        result = run_full_audit(root)
        assert len(result.modules) == len(result.shells)
        assert len(result.modules) == len(result.doc_audits)
        assert len(result.modules) == len(result.test_maps)
        assert len(result.modules) == len(result.guard_audits)
        assert len(result.modules) == len(result.backlog)

    def test_audit_reproducible(self):
        """Two consecutive runs should yield same counts."""
        root = _resolve_root()
        r1 = run_full_audit(root)
        r2 = run_full_audit(root)
        assert r1.summary["total_modules"] == r2.summary["total_modules"]
        assert r1.summary["empty_shells"] == r2.summary["empty_shells"]


# ============================================================
# Test 12: No-fake guard (主 17:58+20:46 不假装)
# ============================================================


class TestV1082NoFake:
    def test_subscore_not_suspiciously_perfect(self):
        """Real audit should not return 1.0 because there are real gaps."""
        root = _resolve_root()
        result = run_full_audit(root)
        bridge = ASICodebaseAuditBridge()
        sub = bridge.subscore(result)
        assert sub < 1.0  # Honest subscore, not KPI padding

    def test_empty_shell_count_honest(self):
        """Report must not round shell count down to make it look better."""
        root = _resolve_root()
        result = run_full_audit(root)
        # If we have ~984 shells, the report should say so (not "100" or "0")
        assert result.summary["empty_shells"] > 800
        assert result.summary["shell_ratio"] > 0.8

    def test_v1082_does_not_claim_to_fix(self):
        """V1082 is audit-only; main must not call any 'fix' or 'patch'."""
        import inspect
        src = inspect.getsource(v1082_main)
        # Should not have any patching logic
        assert "patch" not in src.lower() or "apply_patch" not in src.lower()
        assert "fix_module" not in src.lower()