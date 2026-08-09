#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
test_v1395_deploy_dashboard.py — Pytest 验证 V1395 ASI 真生产 deploy-stack dashboard

V1395 = real production dashboard aggregating V1384-V1394 outputs into one view
        (markdown + JSON + HTML). Anyone can open dashboard and see whole stack.

Sections:
 1. Module constants (5)
 2. ModuleStatus + DashboardData dataclass (6)
 3. _extract_constants 多模式兼容 (5)
 4. _count_tests 多目录 + class methods 兼容 (4)
 5. discover_module_status + _default_tests_dirs (5)
 6. build_dashboard basic + judge + history integration (6)
 7. render_markdown / render_html / render_json (6)
 8. popper_self_test passes (2)
 9. CLI: version / dashboard / html / json / modules / popper / demo (7)
 10. V3 哲学 守门 6 GUARDS 自动注入 (3)
"""
import json
import sys
import tempfile
from pathlib import Path

import pytest

# Add apeireth root to path so the module can be imported
APEIRETH_ROOT = Path(__file__).resolve().parent.parent
if str(APEIRETH_ROOT) not in sys.path:
    sys.path.insert(0, str(APEIRETH_ROOT))

import v1395_deploy_dashboard as m  # noqa: E402
from v1395_deploy_dashboard import (  # noqa: E402
    DashboardData,
    ModuleStatus,
    V1395_GUARDS,
    V1395_MODULES,
    V1395_SCHEMA,
    V1395_VERSION,
    _count_tests,
    _default_tests_dirs,
    _extract_constants,
    _iso_timestamp,
    build_dashboard,
    discover_module_status,
    popper_self_test,
    render_html,
    render_json,
    render_markdown,
    run_cli,
)


# ============================================================================
# 1. Module constants (5)
# ============================================================================


def test_version_nonempty():
    assert V1395_VERSION
    assert isinstance(V1395_VERSION, str)


def test_schema_nonempty():
    assert V1395_SCHEMA
    assert V1395_SCHEMA.startswith("v1395.")


def test_modules_count():
    assert len(V1395_MODULES) == 11


def test_modules_have_required_fields():
    for module_id, label, module_name in V1395_MODULES:
        assert module_id.startswith("V13")
        assert label
        assert module_name.endswith(".py") is False


def test_guards_count():
    assert len(V1395_GUARDS) >= 8
    assert "GUARD_DASHBOARD_REAL" in V1395_GUARDS
    assert "GUARD_NO_CAP_CHANGE" in V1395_GUARDS
    assert "GUARD_HONEST_DISCLOSURE" in V1395_GUARDS


# ============================================================================
# 2. ModuleStatus + DashboardData dataclass (6)
# ============================================================================


def test_module_status_defaults():
    ms = ModuleStatus()
    assert ms.module_id == ""
    assert ms.label == ""
    assert ms.present is False
    assert ms.broken is False
    assert ms.n_guards == 0
    assert ms.n_tests == 0


def test_module_status_to_dict():
    ms = ModuleStatus(module_id="V9999", label="x", module_name="x", present=True, n_guards=5)
    d = ms.to_dict()
    assert d["module_id"] == "V9999"
    assert d["present"] is True
    assert d["n_guards"] == 5


def test_dashboard_data_defaults():
    dd = DashboardData()
    assert dd.n_modules == 0
    assert dd.modules == []
    assert dd.judge_verdict == "N/A"
    assert dd.history_trend == "n/a"


def test_dashboard_data_to_dict():
    dd = DashboardData(title="t", generated_at="now")
    d = dd.to_dict()
    assert d["schema"] == V1395_SCHEMA
    assert d["version"] == V1395_VERSION
    assert d["title"] == "t"


def test_dashboard_data_guards_in_dict():
    dd = DashboardData()
    d = dd.to_dict()
    assert isinstance(d["guards"], list)
    assert len(d["guards"]) >= 8


def test_dashboard_data_known_unknowns():
    dd = build_dashboard()
    d = dd.to_dict()
    assert len(d["known_unknowns"]) >= 1


# ============================================================================
# 3. _extract_constants 多模式兼容 (5)
# ============================================================================


def test_extract_constants_version_only():
    text = 'V1395_VERSION = "0.2.0"\n'
    c = _extract_constants(text, "V1395")
    assert c.get("VERSION") == "0.2.0"


def test_extract_constants_schema_variants():
    text1 = 'V1387_SCHEMA_VERSION = "v1387.stack/v1"\n'
    text2 = 'V1389_SCHEMA = "v1389.ci/v1"\n'
    text3 = 'V1388_BASELINE_SCHEMA = "v1388.baseline/v1"\n'
    assert _extract_constants(text1, "V1387").get("SCHEMA") == "v1387.stack/v1"
    assert _extract_constants(text2, "V1389").get("SCHEMA") == "v1389.ci/v1"
    assert _extract_constants(text3, "V1388").get("SCHEMA") == "v1388.baseline/v1"


def test_extract_constants_schema_fallback_docstring():
    text = '"""v1384.real-lint/v1 docstring"""\nV1384_VERSION = "0.1.0"\n'
    c = _extract_constants(text, "V1384")
    assert c.get("VERSION") == "0.1.0"
    assert "SCHEMA" in c  # fallback to docstring


def test_extract_constants_missing_version():
    text = 'OTHER_VAR = "foo"\n'
    c = _extract_constants(text, "V1395")
    assert "VERSION" not in c


def test_extract_constants_empty():
    c = _extract_constants("", "V1395")
    assert c == {}


# ============================================================================
# 4. _count_tests 多目录 + class methods 兼容 (4)
# ============================================================================


def test_count_tests_nonexistent():
    assert _count_tests([Path("/nonexistent/file.py")]) == 0


def test_count_tests_module_level(tmp_path):
    p = tmp_path / "test_x.py"
    p.write_text("def test_a():\n    pass\ndef test_b():\n    pass\n", encoding="utf-8")
    assert _count_tests([p]) == 2


def test_count_tests_class_methods(tmp_path):
    p = tmp_path / "test_x.py"
    p.write_text(
        "class TestFoo:\n"
        "    def test_a(self):\n        pass\n"
        "    def test_b(self):\n        pass\n"
        "def test_c():\n    pass\n",
        encoding="utf-8",
    )
    assert _count_tests([p]) == 3


def test_count_tests_sums_multiple_paths(tmp_path):
    p1 = tmp_path / "test_a.py"
    p2 = tmp_path / "test_b.py"
    p1.write_text("def test_x():\n    pass\n", encoding="utf-8")
    p2.write_text("def test_y():\n    pass\ndef test_z():\n    pass\n", encoding="utf-8")
    assert _count_tests([p1, p2]) == 3


# ============================================================================
# 5. discover_module_status + _default_tests_dirs (5)
# ============================================================================


def test_default_tests_dirs_picks_existing():
    # Real paths in this repo
    apeireth_dir = Path(__file__).resolve().parent.parent
    dirs = _default_tests_dirs(apeireth_dir)
    assert len(dirs) >= 1
    assert all(d.exists() for d in dirs)


def test_discover_all_11_modules_present():
    apeireth_dir = Path(__file__).resolve().parent.parent
    dirs = _default_tests_dirs(apeireth_dir)
    statuses = discover_module_status(apeireth_dir, dirs)
    assert len(statuses) == 11
    present = sum(1 for s in statuses if s.present)
    assert present == 11, f"only {present}/11 modules present"


def test_discover_versions_extracted():
    apeireth_dir = Path(__file__).resolve().parent.parent
    dirs = _default_tests_dirs(apeireth_dir)
    statuses = discover_module_status(apeireth_dir, dirs)
    for s in statuses:
        if s.present:
            assert s.version, f"{s.module_id} missing version"


def test_discover_tests_counted():
    apeireth_dir = Path(__file__).resolve().parent.parent
    dirs = _default_tests_dirs(apeireth_dir)
    statuses = discover_module_status(apeireth_dir, dirs)
    has_tests = [s for s in statuses if s.has_tests]
    assert len(has_tests) >= 9, f"only {len(has_tests)}/11 modules have tests"


def test_discover_handles_missing_dir(tmp_path):
    statuses = discover_module_status(tmp_path, [])
    assert len(statuses) == 11
    assert sum(1 for s in statuses if s.present) == 0


# ============================================================================
# 6. build_dashboard basic + judge + history integration (6)
# ============================================================================


def test_build_dashboard_basic():
    dd = build_dashboard()
    assert dd.n_modules == 11
    assert dd.n_present == 11
    assert dd.n_tests_total >= 400
    assert dd.judge_verdict == "N/A"  # no judge target
    assert dd.history_trend == "n/a"  # no history


def test_build_dashboard_with_judge_target():
    dd = build_dashboard(judge_target="deploy")
    assert dd.judge_target == "deploy"
    assert dd.judge_verdict in ("GOOD", "OK", "POOR", "FAIL", "CRITICAL")
    assert dd.judge_score >= 0
    assert dd.judge_n_findings >= 0


def test_build_dashboard_with_history(tmp_path):
    # Build a fake history JSONL with 3 entries trending up
    from datetime import datetime, timezone, timedelta
    hist = tmp_path / "fake_history.jsonl"
    lines = []
    base = datetime(2026, 8, 9, 0, 0, 0, tzinfo=timezone.utc)
    for i, score in enumerate([50, 75, 90]):
        ts = (base + timedelta(hours=i)).strftime("%Y-%m-%dT%H:%M:%SZ")
        lines.append(json.dumps({
            "schema": "v1394.deploy-history/v1",
            "timestamp": ts,
            "target": "demo",
            "verdict": "OK",
            "score": score,
            "grade": "B" if score < 90 else "A",
            "n_findings": 0,
            "n_errors": 0,
            "n_warnings": 0,
            "n_info": 0,
            "policy_pass": True,
            "policy_score": 100,
            "n_hints": 0,
            "notes": [],
        }))
    hist.write_text("\n".join(lines), encoding="utf-8")
    dd = build_dashboard(history_path=str(hist))
    assert dd.history_n_entries == 3
    assert dd.history_trend == "improving"
    assert dd.history_delta_score == 40
    assert dd.history_first_score == 50
    assert dd.history_last_score == 90


def test_build_dashboard_history_missing(tmp_path):
    dd = build_dashboard(history_path=str(tmp_path / "nope.jsonl"))
    # missing history is not an error (returns n/a)
    assert dd.history_trend == "n/a"


def test_build_dashboard_judge_invalid_target():
    dd = build_dashboard(judge_target="/nonexistent/xyz_abc")
    # judge should still return something (likely CRITICAL or POOR)
    assert dd.judge_target in ("/nonexistent/xyz_abc", "")
    assert dd.judge_verdict in ("GOOD", "OK", "POOR", "FAIL", "CRITICAL", "N/A")


def test_build_dashboard_custom_title():
    dd = build_dashboard(title="My Custom Title")
    assert dd.title == "My Custom Title"


# ============================================================================
# 7. render_markdown / render_html / render_json (6)
# ============================================================================


def test_render_markdown_basic():
    dd = build_dashboard()
    md = render_markdown(dd)
    assert "Apeireth deploy-stack dashboard" in md
    assert "Deploy-stack modules" in md
    assert "GUARDS" in md
    assert "Known unknowns" in md
    assert "modules: **11/11**" in md


def test_render_markdown_includes_judge_section():
    dd = build_dashboard(judge_target="deploy")
    md = render_markdown(dd)
    assert "judge:" in md
    assert "deploy" in md


def test_render_html_basic():
    dd = build_dashboard()
    html = render_html(dd)
    assert "<!doctype html>" in html
    assert "<table>" in html
    assert "</html>" in html
    assert "V1395" in html


def test_render_html_escapes_special_chars():
    dd = build_dashboard(title='<script>alert("xss")</script>')
    html = render_html(dd)
    assert "<script>" not in html
    assert "&lt;script&gt;" in html


def test_render_json_valid():
    dd = build_dashboard()
    js = render_json(dd)
    parsed = json.loads(js)
    assert parsed["schema"] == V1395_SCHEMA
    assert parsed["n_modules"] == 11


def test_render_json_includes_modules():
    dd = build_dashboard()
    js = render_json(dd)
    parsed = json.loads(js)
    assert len(parsed["modules"]) == 11
    assert parsed["modules"][0]["module_id"] == "V1384"


# ============================================================================
# 8. popper_self_test passes (2)
# ============================================================================


def test_popper_passes():
    r = popper_self_test()
    assert r["passed"] is True, f"popper failures: {r['failures']}"


def test_popper_returns_dict():
    r = popper_self_test()
    assert "passed" in r
    assert "failures" in r
    assert "n_tested" in r


# ============================================================================
# 9. CLI: version / dashboard / html / json / modules / popper / demo (7)
# ============================================================================


def test_cli_version(capsys):
    rc = run_cli(["version"])
    captured = capsys.readouterr()
    assert rc == 0
    assert V1395_VERSION in captured.out


def test_cli_dashboard_no_judge(capsys):
    rc = run_cli(["dashboard"])
    captured = capsys.readouterr()
    assert rc == 0
    assert "Apeireth deploy-stack dashboard" in captured.out


def test_cli_dashboard_to_file(tmp_path):
    out = tmp_path / "dash.md"
    rc = run_cli(["dashboard", "--out", str(out)])
    assert rc == 0
    assert out.exists()
    content = out.read_text(encoding="utf-8")
    assert "Apeireth deploy-stack dashboard" in content


def test_cli_html(capsys):
    rc = run_cli(["html"])
    captured = capsys.readouterr()
    assert rc == 0
    assert "<!doctype html>" in captured.out


def test_cli_json(capsys):
    rc = run_cli(["json"])
    captured = capsys.readouterr()
    assert rc == 0
    parsed = json.loads(captured.out)
    assert parsed["schema"] == V1395_SCHEMA


def test_cli_modules(capsys):
    rc = run_cli(["modules"])
    captured = capsys.readouterr()
    assert rc == 0
    assert "V1384" in captured.out
    assert "V1394" in captured.out


def test_cli_popper(capsys):
    rc = run_cli(["popper"])
    captured = capsys.readouterr()
    assert rc == 0
    parsed = json.loads(captured.out)
    assert parsed["passed"] is True


# ============================================================================
# 10. V3 哲学 守门 6 GUARDS 自动注入 (3)
# ============================================================================


def test_v3_guard_module_is_not_asi():
    """V1395 module docstring should not claim to be ASI."""
    src = Path(__file__).resolve().parent.parent / "v1395_deploy_dashboard.py"
    text = src.read_text(encoding="utf-8")
    assert "V1395 不宣称是 ASI" not in text or "不假装" in text
    # 真 ref: 哲学守门里有明确的 anti-claim
    assert "不假装" in text
    assert "ASI 北极星" in text


def test_v3_guard_no_cap_change():
    """V1395 must include GUARD_NO_CAP_CHANGE."""
    assert "GUARD_NO_CAP_CHANGE" in V1395_GUARDS


def test_v3_guard_honest_disclosure():
    """V1395 must include GUARD_HONEST_DISCLOSURE."""
    assert "GUARD_HONEST_DISCLOSURE" in V1395_GUARDS
    # known_unknowns 字段存在
    dd = build_dashboard()
    assert len(dd.known_unknowns) >= 1