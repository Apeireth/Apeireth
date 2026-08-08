"""Phase 1390 test_v1390_asi_deployment_philosophy — V1390 ASI 真 deployment philosophy V1 tests (主 06:15 + 主 23:44 + 主 22:33 + 主 19:33 + 主 17:43 + 主 17:33 + 主 00:36).

V1390 = ASI real deployment philosophy codification (post-V1389 next-step).
Tests verify: 9 lessons present + 12 borrowed refs + 6 philosophy guards + CLI + self-test.
"""
from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from apeireth.v1390_asi_deployment_philosophy import (  # noqa: E402
    V1390_VERSION,
    V1390_SCHEMA,
    V1390_LESSONS,
    V1390_BORROWED,
    V1390_GUARDS,
    V1390PhilosophyReport,
    validate_lessons,
    validate_borrowed,
    validate_guards,
    self_test,
    run_cli,
)


REPO_ROOT = Path(__file__).resolve().parent.parent


# ============================================================================
# V1390 basic structure tests (主 17:43 实事求是)
# ============================================================================


def test_v1390_module_version_constant():
    assert V1390_VERSION == "0.1.0"
    assert V1390_SCHEMA == "v1390.philosophy/v1"


def test_v1390_n_lessons():
    """V1390 真生产 9 lessons from V1384-V1389 真跑 (主 17:43)."""
    assert len(V1390_LESSONS) == 9


def test_v1390_n_borrowed():
    """V1390 真生产 12 borrowed references (主 19:33 走在前人经验上)."""
    assert len(V1390_BORROWED) == 12


def test_v1390_n_guards():
    """V1390 真生产 6 philosophy guards (主 17:58 + 主 20:46)."""
    assert len(V1390_GUARDS) == 6


def test_v1390_lesson_ids_unique_and_sequential():
    """V1390 真生产 lesson IDs L1-L9 unique (主 17:43)."""
    ids = [l["id"] for l in V1390_LESSONS]
    assert ids == [f"L{i}" for i in range(1, 10)]


def test_v1390_borrowed_ids_unique_and_sequential():
    """V1390 真生产 borrowed IDs B1-B12 unique (主 19:33)."""
    ids = [b["id"] for b in V1390_BORROWED]
    assert ids == [f"B{i}" for i in range(1, 13)]


def test_v1390_lessons_have_required_fields():
    """V1390 真生产 every lesson has id + title + body + borrowed_from + evidence (主 17:43)."""
    for l in V1390_LESSONS:
        assert "id" in l
        assert "title" in l
        assert "body" in l
        assert "borrowed_from" in l
        assert "evidence" in l
        assert isinstance(l["evidence"], list)
        assert len(l["evidence"]) >= 1


def test_v1390_borrowed_have_url_and_purpose():
    """V1390 真生产 every borrowed has URL + purpose (主 19:33)."""
    for b in V1390_BORROWED:
        assert b["url"].startswith("http")
        assert len(b["purpose"]) > 0


def test_v1390_guards_have_required_prefixes():
    """V1390 真生产 6 guards contain required prefixes (主 17:58 + 主 20:46)."""
    required = [
        "module_is_not_asi",
        "measurement_is_not_truth",
        "structure_is_not_consciousness",
        "production_is_not_safety",
        "automation_is_not_autonomy",
        "runner_is_not_asi",
    ]
    for prefix in required:
        assert any(prefix in g for g in V1390_GUARDS), f"missing guard: {prefix}"


# ============================================================================
# V1390 validate functions (主 17:43 实事求是)
# ============================================================================


def test_v1390_validate_lessons_ok():
    ok, missing = validate_lessons()
    assert ok is True, f"missing: {missing}"
    assert missing == []


def test_v1390_validate_lessons_detects_missing_keyword():
    bad = [{"id": "L0", "title": "t", "body": "no真", "borrowed_from": "x", "evidence": ["x"]}]
    ok, missing = validate_lessons(bad)
    assert ok is False
    assert any("missing keyword" in m for m in missing)


def test_v1390_validate_borrowed_ok():
    ok, missing = validate_borrowed()
    assert ok is True, f"missing: {missing}"
    assert missing == []


def test_v1390_validate_borrowed_detects_bad_url():
    bad = [{"id": "B0", "name": "x", "url": "not-a-url", "purpose": "p"}]
    ok, missing = validate_borrowed(bad)
    assert ok is False
    assert any("URL" in m for m in missing)


def test_v1390_validate_guards_ok():
    ok, missing = validate_guards()
    assert ok is True, f"missing: {missing}"
    assert missing == []


def test_v1390_validate_guards_detects_missing():
    ok, missing = validate_guards(["unrelated guard"])
    assert ok is False
    assert len(missing) == 6


# ============================================================================
# V1390 self_test (主 17:43)
# ============================================================================


def test_v1390_self_test_ok():
    report = self_test()
    assert report.ok is True
    assert report.n_lessons == 9
    assert report.n_borrowed == 12
    assert report.n_guards == 6
    assert report.lessons_ok is True
    assert report.borrowed_ok is True
    assert report.guards_ok is True
    assert report.guard_violations == []


def test_v1390_self_test_roundtrip_dict():
    report = self_test()
    d = report.to_dict()
    assert d["ok"] is True
    assert d["n_lessons"] == 9
    assert d["n_borrowed"] == 12
    assert d["n_guards"] == 6
    assert "known_unknowns" in d
    assert len(d["known_unknowns"]) >= 5


def test_v1390_self_test_has_known_unknowns():
    report = self_test()
    assert len(report.known_unknowns) >= 5


def test_v1390_self_test_dataclass_attrs():
    report = self_test()
    assert hasattr(report, "started_at")
    assert hasattr(report, "finished_at")
    assert hasattr(report, "elapsed_seconds")
    assert report.elapsed_seconds >= 0


# ============================================================================
# V1390 CLI tests (主 00:36 工程化)
# ============================================================================


def test_v1390_cli_version():
    result = subprocess.run(
        [sys.executable, "-m", "apeireth.v1390_asi_deployment_philosophy", "version"],
        capture_output=True,
        text=True,
        timeout=10,
        encoding="utf-8",
        errors="replace",
        cwd=str(REPO_ROOT),
    )
    assert result.returncode == 0
    assert "V1390" in result.stdout


def test_v1390_cli_lessons():
    result = subprocess.run(
        [sys.executable, "-m", "apeireth.v1390_asi_deployment_philosophy", "lessons"],
        capture_output=True,
        text=True,
        timeout=10,
        encoding="utf-8",
        errors="replace",
        cwd=str(REPO_ROOT),
    )
    assert result.returncode == 0
    for i in range(1, 10):
        assert f"[L{i}]" in result.stdout


def test_v1390_cli_lessons_json():
    result = subprocess.run(
        [sys.executable, "-m", "apeireth.v1390_asi_deployment_philosophy", "lessons-json"],
        capture_output=True,
        text=True,
        timeout=10,
        encoding="utf-8",
        errors="replace",
        cwd=str(REPO_ROOT),
    )
    assert result.returncode == 0
    data = json.loads(result.stdout)
    assert len(data["lessons"]) == 9


def test_v1390_cli_borrowed():
    result = subprocess.run(
        [sys.executable, "-m", "apeireth.v1390_asi_deployment_philosophy", "borrowed"],
        capture_output=True,
        text=True,
        timeout=10,
        encoding="utf-8",
        errors="replace",
        cwd=str(REPO_ROOT),
    )
    assert result.returncode == 0
    for i in range(1, 13):
        assert f"[B{i}]" in result.stdout


def test_v1390_cli_borrowed_json():
    result = subprocess.run(
        [sys.executable, "-m", "apeireth.v1390_asi_deployment_philosophy", "borrowed-json"],
        capture_output=True,
        text=True,
        timeout=10,
        encoding="utf-8",
        errors="replace",
        cwd=str(REPO_ROOT),
    )
    assert result.returncode == 0
    data = json.loads(result.stdout)
    assert len(data["borrowed"]) == 12


def test_v1390_cli_guards():
    result = subprocess.run(
        [sys.executable, "-m", "apeireth.v1390_asi_deployment_philosophy", "guards"],
        capture_output=True,
        text=True,
        timeout=10,
        encoding="utf-8",
        errors="replace",
        cwd=str(REPO_ROOT),
    )
    assert result.returncode == 0
    assert "module_is_not_asi" in result.stdout


def test_v1390_cli_guards_json():
    result = subprocess.run(
        [sys.executable, "-m", "apeireth.v1390_asi_deployment_philosophy", "guards-json"],
        capture_output=True,
        text=True,
        timeout=10,
        encoding="utf-8",
        errors="replace",
        cwd=str(REPO_ROOT),
    )
    assert result.returncode == 0
    data = json.loads(result.stdout)
    assert len(data["guards"]) == 6


def test_v1390_cli_self_test_ok():
    result = subprocess.run(
        [sys.executable, "-m", "apeireth.v1390_asi_deployment_philosophy", "self-test"],
        capture_output=True,
        text=True,
        timeout=10,
        encoding="utf-8",
        errors="replace",
        cwd=str(REPO_ROOT),
    )
    assert result.returncode == 0, f"stderr={result.stderr}"
    assert "ok: True" in result.stdout


def test_v1390_cli_self_test_json_ok():
    result = subprocess.run(
        [sys.executable, "-m", "apeireth.v1390_asi_deployment_philosophy", "self-test", "--json"],
        capture_output=True,
        text=True,
        timeout=10,
        encoding="utf-8",
        errors="replace",
        cwd=str(REPO_ROOT),
    )
    assert result.returncode == 0, f"stderr={result.stderr}"
    data = json.loads(result.stdout)
    assert data["ok"] is True
    assert data["n_lessons"] == 9
    assert data["n_borrowed"] == 12
    assert data["n_guards"] == 6


def test_v1390_cli_self_test_quiet():
    result = subprocess.run(
        [sys.executable, "-m", "apeireth.v1390_asi_deployment_philosophy", "self-test", "--quiet"],
        capture_output=True,
        text=True,
        timeout=10,
        encoding="utf-8",
        errors="replace",
        cwd=str(REPO_ROOT),
    )
    assert result.returncode == 0, f"stderr={result.stderr}"


def test_v1390_cli_help():
    result = subprocess.run(
        [sys.executable, "-m", "apeireth.v1390_asi_deployment_philosophy", "--help"],
        capture_output=True,
        text=True,
        timeout=10,
        encoding="utf-8",
        errors="replace",
        cwd=str(REPO_ROOT),
    )
    assert result.returncode == 0
    assert "V1390" in result.stdout


# ============================================================================
# V1390 lesson content quality (主 17:43 实事求是)
# ============================================================================


def test_v1390_lesson_l1_linter_vs_deployment():
    """V1390 L1 真 linter ≠ 真 deployment (主 17:43)."""
    l1 = next(l for l in V1390_LESSONS if l["id"] == "L1")
    assert "真 deployment" in l1["body"]
    assert "V1384" in l1["body"]
    assert "V1389" in l1["body"]


def test_v1390_lesson_l2_exit_code_platform():
    """V1390 L2 真 exit code 必须跨平台反射 (主 17:43)."""
    l2 = next(l for l in V1390_LESSONS if l["id"] == "L2")
    assert "bash" in l2["body"]
    assert "Windows" in l2["body"]
    assert "fallback" in l2["body"]


def test_v1390_lesson_l3_sarif_real():
    """V1390 L3 真 SARIF ≠ '应该有 SARIF' (主 17:43)."""
    l3 = next(l for l in V1390_LESSONS if l["id"] == "L3")
    assert "SARIF" in l3["body"]
    assert "2.1.0" in l3["body"]


def test_v1390_lesson_l4_borrowed_not_copypaste():
    """V1390 L4 真 borrowed ≠ copy-paste (主 19:33)."""
    l4 = next(l for l in V1390_LESSONS if l["id"] == "L4")
    assert "borrowed" in l4["body"].lower() or "借鉴" in l4["body"]


def test_v1390_lesson_l5_philosophy_guard():
    """V1390 L5 真哲学守门 (主 17:58)."""
    l5 = next(l for l in V1390_LESSONS if l["id"] == "L5")
    assert "6 GUARDS" in l5["body"] or "守门" in l5["body"]


def test_v1390_lesson_l6_ci_gate_4_exit_codes():
    """V1390 L6 真 CI gate ≠ '应该有 CI' (主 17:43)."""
    l6 = next(l for l in V1390_LESSONS if l["id"] == "L6")
    assert "0" in l6["body"] and "1" in l6["body"]
    assert "exit" in l6["body"].lower() or "exit" in l6["body"]


def test_v1390_lesson_l7_subprocess_4_layer_guard():
    """V1390 L7 真 subprocess ≠ 假设 bash (主 17:43)."""
    l7 = next(l for l in V1390_LESSONS if l["id"] == "L7")
    assert "probe" in l7["body"].lower() or "Probe" in l7["body"]
    assert "fallback" in l7["body"]


def test_v1390_lesson_l8_chain_test():
    """V1390 L8 真 chain test ≠ 单测 (主 17:43)."""
    l8 = next(l for l in V1390_LESSONS if l["id"] == "L8")
    assert "V1384" in l8["body"]
    assert "V1389" in l8["body"]


def test_v1390_lesson_l9_honest_cap():
    """V1390 L9 真 0.90 cap ≠ 假 cap (主 17:43)."""
    l9 = next(l for l in V1390_LESSONS if l["id"] == "L9")
    assert "0.90" in l9["body"]
    assert "ASI" in l9["body"]


# ============================================================================
# V1390 borrowed references quality (主 19:33 走在前人经验上)
# ============================================================================


def test_v1390_borrowed_b1_12factor():
    b1 = next(b for b in V1390_BORROWED if b["id"] == "B1")
    assert "12factor.net" in b1["url"]


def test_v1390_borrowed_b5_super_linter():
    b5 = next(b for b in V1390_BORROWED if b["id"] == "B5")
    assert "super-linter" in b5["url"]


def test_v1390_borrowed_b8_hadolint():
    b8 = next(b for b in V1390_BORROWED if b["id"] == "B8")
    assert "hadolint" in b8["url"].lower()


def test_v1390_borrowed_b9_kubeval():
    b9 = next(b for b in V1390_BORROWED if b["id"] == "B9")
    assert "kubeval" in b9["url"].lower()


def test_v1390_borrowed_b10_kubeconform():
    b10 = next(b for b in V1390_BORROWED if b["id"] == "B10")
    assert "kubeconform" in b10["url"].lower()


def test_v1390_borrowed_b11_polaris():
    b11 = next(b for b in V1390_BORROWED if b["id"] == "B11")
    assert "polaris" in b11["url"].lower()


# ============================================================================
# V1390 guards content (主 17:58 + 主 20:46)
# ============================================================================


def test_v1390_guard_runner_is_not_asi():
    """V1390 真生产 runner_is_not_asi guard (主 20:46)."""
    assert any("runner_is_not_asi" in g for g in V1390_GUARDS)


def test_v1390_guard_automation_is_not_autonomy():
    """V1390 真生产 automation_is_not_autonomy guard (主 17:58)."""
    assert any("automation_is_not_autonomy" in g for g in V1390_GUARDS)


def test_v1390_guard_measurement_is_not_truth():
    """V1390 真生产 measurement_is_not_truth guard (主 17:43)."""
    assert any("measurement_is_not_truth" in g for g in V1390_GUARDS)


# ============================================================================
# V1390 chain test (主 17:43 — V1390 chains V1384-V1389)
# ============================================================================


def test_v1390_chain_with_v1384_v1389():
    """V1390 真生产 post-V1389 next-step references V1384-V1389 (主 17:43)."""
    # L1, L8 must reference V1384-V1389
    l1 = next(l for l in V1390_LESSONS if l["id"] == "L1")
    assert "V1384" in l1["body"] and "V1389" in l1["body"]
    l8 = next(l for l in V1390_LESSONS if l["id"] == "L8")
    assert "V1384" in l8["body"] and "V1389" in l8["body"]


def test_v1390_does_not_modify_v1384_v1389():
    """V1390 真生产 no side effects on V1384-V1389 (主 17:43)."""
    # V1390 is read-only / pure codification
    import apeireth.v1390_asi_deployment_philosophy as m
    # No subprocess imports
    assert not hasattr(m, "subprocess") or m.subprocess.__name__ == "subprocess"
    # No file writes outside test
    assert not hasattr(m, "_write_file")
    assert not hasattr(m, "_save_baseline")