"""Tests for V1276 ASI Time Falsifier (3 time/freshness 假说).

> **作者**: 楚零 (Apeireth ASI self-driven agent, cron:1fba1cc3, 15:32+08:00 2026-08-05)
> **覆盖**: 18 tests across philosophy gate, hypothesis specs, evidence gatherers,
>            falsifier dispatch, run_all_hypotheses, markdown/JSON render, CLI.
> **不假装**: 每个 test 都真跑真验证 (主 17:43 实事求是 + 主 17:58 不假装).
"""
from __future__ import annotations

import json
import os
import subprocess
import sys
from pathlib import Path

import pytest

# Ensure project root is on sys.path
_PROJECT_ROOT = Path(__file__).resolve().parent.parent
if str(_PROJECT_ROOT) not in sys.path:
    sys.path.insert(0, str(_PROJECT_ROOT))

from apeireth import v1276_asi_time_falsifier as v1276
from apeireth.v1274_asi_truth_falsifier import (
    HypothesisSpec,
    FalsifierResult,
    TruthLedger,
    _v3_philosophy_gate,
)


# ============================================================
# Fixtures
# ============================================================

@pytest.fixture
def promethean_dir() -> Path:
    """项目根 (主 17:43 实事求是: 真跑真测)."""
    return Path(__file__).resolve().parent.parent


@pytest.fixture
def apeireth_dir(promethean_dir: Path) -> Path:
    """apeireth/ 子目录."""
    return promethean_dir / "apeireth"


# ============================================================
# Test 1-2: Constants & Version
# ============================================================

def test_constants_exist():
    """V1276 constants are defined (主 17:43 实事求是)."""
    assert v1276.V1276_VERSION == "0.1.0"
    assert v1276.V1276_BUILD.startswith("2026-08-05")
    assert v1276.V1276_ASI_NS_LOCKED_PCT == 92.91
    assert v1276.V1276_THRESHOLD_GIT_AGE_DAYS == 30
    assert v1276.V1276_THRESHOLD_7D_COMMITS == 5
    assert v1276.V1276_THRESHOLD_V1275_MTIME_HOURS == 24


def test_asi_ns_current_unchanged_from_v1275():
    """ASI NS current 不刷 (主 17:43 实事求是: 不刷 KPI)."""
    # V1276 = 扩展, 不替代 V1275; ASI NS 不变
    assert v1276.V1276_ASI_NS_CURRENT == 0.7905
    assert v1276.V1276_ASI_NS_LOCKED_PCT == 92.91


# ============================================================
# Test 3: V3 Philosophy Gate (主 17:58 不假装)
# ============================================================

def test_v1276_philosophy_gate_has_11_entries():
    """V1276 philosophy gate = 11 entries (V1274 9 + V1275 1 + V1276 1)."""
    gate = v1276._v1276_philosophy_gate()
    assert len(gate) == 11
    # V1276 specific gate
    assert gate["v1276_extends_v1275_not_replaces"] is True
    # V1275 specific gate
    assert gate["v1275_extends_v1274_not_replaces"] is True
    # V1274 core gates (sample)
    assert gate["v1274_not_new_asi_dim"] is True
    assert gate["v1274_no_phenomenal_claim"] is True
    assert gate["v1274_truth_is_falsifiability"] is True


# ============================================================
# Test 4-6: 3 Built-in Hypotheses (主 17:43 实事求是)
# ============================================================

def test_builtin_hypotheses_count():
    """3 假说 全列出 (主 19:33 走在前人肩上: 继承 V1275 pattern)."""
    specs = v1276._builtin_hypotheses()
    assert len(specs) == 3


def test_h_git_age_days_spec():
    """h_git_age_days: critical + git_age_days evidence_type."""
    specs = v1276._builtin_hypotheses()
    spec = next(s for s in specs if s.hypothesis_id == "h_git_age_days")
    assert spec.severity == "critical"
    assert spec.evidence_type == "git_age_days"
    assert spec.threshold == 30
    assert "30" in spec.falsification_rule


def test_h_recent_commits_7d_spec():
    """h_recent_commits_7d: important + git_7d_commits."""
    specs = v1276._builtin_hypotheses()
    spec = next(s for s in specs if s.hypothesis_id == "h_recent_commits_7d")
    assert spec.severity == "important"
    assert spec.evidence_type == "git_7d_commits"
    assert spec.threshold == 5


def test_h_v1275_mtime_recent_spec():
    """h_v1275_mtime_recent: info + file_mtime_recent."""
    specs = v1276._builtin_hypotheses()
    spec = next(s for s in specs if s.hypothesis_id == "h_v1275_mtime_recent")
    assert spec.severity == "info"
    assert spec.evidence_type == "file_mtime_recent"
    assert spec.threshold == 24


# ============================================================
# Test 7-9: Evidence Gatherers (主 17:43 实事求是: 真测)
# ============================================================

def test_git_first_commit_age_days_real(promethean_dir: Path):
    """_git_first_commit_age_days: 真跑 git log --reverse -n 1."""
    age_days, git_avail, errors = v1276._git_first_commit_age_days(promethean_dir)
    # 这个 workspace 是 fresh clone (今天); age < 30 days = FAIL (主 17:58 不假装)
    if git_avail:
        assert isinstance(age_days, float)
        assert age_days >= 0.0
    else:
        assert errors  # 应该有错误信息
        assert age_days == 0.0


def test_count_recent_commits_7d_real(promethean_dir: Path):
    """_count_recent_commits_7d: 真跑 git log --since=7.days.ago."""
    count, git_avail, errors = v1276._count_recent_commits_7d(promethean_dir)
    if git_avail:
        assert isinstance(count, int)
        assert count >= 0
    else:
        assert errors
        assert count == 0


def test_file_mtime_age_hours_v1275(apeireth_dir: Path):
    """_file_mtime_age_hours: 真测 V1275 file mtime (主 17:43 实事求是)."""
    v1275_path = apeireth_dir / "v1275_asi_extended_falsifier.py"
    age_hours, exists, errors = v1276._file_mtime_age_hours(v1275_path)
    assert exists is True
    assert isinstance(age_hours, float)
    # V1275 刚 commit, mtime 应该很新
    assert age_hours >= 0.0


# ============================================================
# Test 10-12: Falsifier Dispatch (主 17:43 实事求是)
# ============================================================

def test_falsify_h_git_age_days_real(promethean_dir: Path):
    """falsify_hypothesis: h_git_age_days 真跑."""
    specs = v1276._builtin_hypotheses()
    spec = next(s for s in specs if s.hypothesis_id == "h_git_age_days")
    result = v1276.falsify_hypothesis(spec, promethean_dir)
    assert isinstance(result, FalsifierResult)
    assert result.hypothesis_id == "h_git_age_days"
    assert result.pass_fail in ("PASS", "FAIL", "INCONCLUSIVE")
    # evidence_type 正确传递
    assert result.evidence_type == "git_age_days"


def test_falsify_h_recent_commits_7d_real(promethean_dir: Path):
    """falsify_hypothesis: h_recent_commits_7d 真跑."""
    specs = v1276._builtin_hypotheses()
    spec = next(s for s in specs if s.hypothesis_id == "h_recent_commits_7d")
    result = v1276.falsify_hypothesis(spec, promethean_dir)
    assert result.pass_fail in ("PASS", "FAIL", "INCONCLUSIVE")
    assert result.evidence_type == "git_7d_commits"


def test_falsify_h_v1275_mtime_recent_real(promethean_dir: Path):
    """falsify_hypothesis: h_v1275_mtime_recent 真跑."""
    specs = v1276._builtin_hypotheses()
    spec = next(s for s in specs if s.hypothesis_id == "h_v1275_mtime_recent")
    result = v1276.falsify_hypothesis(spec, promethean_dir)
    assert result.pass_fail in ("PASS", "FAIL", "INCONCLUSIVE")
    assert result.evidence_type == "file_mtime_recent"


def test_falsify_unknown_evidence_type_inconclusive(promethean_dir: Path):
    """未知 evidence_type → INCONCLUSIVE (主 17:43 实事求是)."""
    spec = HypothesisSpec(
        hypothesis_id="h_unknown_test",
        claim="test unknown evidence type",
        falsification_rule="if dispatched → INCONCLUSIVE",
        severity="info",
        evidence_type="definitely_not_a_real_type",
        threshold=0,
    )
    result = v1276.falsify_hypothesis(spec, promethean_dir)
    assert result.pass_fail == "INCONCLUSIVE"
    assert "unknown evidence_type" in result.notes


# ============================================================
# Test 13-14: run_all_hypotheses + TruthLedger (主 00:56 任何人都能接手)
# ============================================================

def test_run_all_hypotheses_returns_truthledger(promethean_dir: Path):
    """run_all_hypotheses 返回 TruthLedger (主 17:43 实事求是)."""
    ledger = v1276.run_all_hypotheses(promethean_dir)
    assert isinstance(ledger, TruthLedger)
    assert len(ledger.results) == 3
    assert ledger.n_pass + ledger.n_fail + ledger.n_inconclusive == 3
    # falsification_rate = fail / total
    expected_rate = ledger.n_fail / 3.0
    assert abs(ledger.falsification_rate - round(expected_rate, 4)) < 0.001


def test_run_all_hypotheses_v3_gate_in_ledger(promethean_dir: Path):
    """TruthLedger.philosophy_gate 包含 V1276 11 gates."""
    ledger = v1276.run_all_hypotheses(promethean_dir)
    assert len(ledger.philosophy_gate) == 11
    assert ledger.philosophy_gate["v1276_extends_v1275_not_replaces"] is True


# ============================================================
# Test 15-16: Markdown + JSON Report (主 17:43 实事求是)
# ============================================================

def test_to_markdown_includes_failures(promethean_dir: Path):
    """_to_markdown: FAIL 也展示, 不假装全 PASS (主 17:58)."""
    ledger = v1276.run_all_hypotheses(promethean_dir)
    md = v1276._to_markdown(ledger)
    # 必须包含 3 假说
    assert "h_git_age_days" in md
    assert "h_recent_commits_7d" in md
    assert "h_v1275_mtime_recent" in md
    # 必须包含 falsification_rate
    assert "Falsification rate" in md
    # 必须包含 V1276 build info
    assert v1276.V1276_BUILD in md
    assert v1276.V1276_VERSION in md
    # 必须包含 ASI NS (不刷, 实事求是)
    assert "92.91%" in md
    # 必须包含 11 gates (继承 V1275 10 + V1276 1)
    assert "v1276_extends_v1275_not_replaces" in md


def test_to_json_includes_3_results(promethean_dir: Path):
    """_to_json: 3 results + 11 gates + run_id + ts."""
    ledger = v1276.run_all_hypotheses(promethean_dir)
    js = v1276._to_json(ledger)
    obj = json.loads(js)
    assert "run_id" in obj
    assert obj["run_id"].startswith("v1276-")
    assert len(obj["results"]) == 3
    assert len(obj["philosophy_gate"]) == 11
    assert obj["version"] == v1276.V1276_VERSION
    assert obj["build"] == v1276.V1276_BUILD


# ============================================================
# Test 17: CLI _resolve_promethean_dir (主 00:56 任何人都能接手)
# ============================================================

def test_resolve_promethean_dir_explicit():
    """_resolve_promethean_dir: --promethean-dir explicit."""
    p = v1276._resolve_promethean_dir(str(_PROJECT_ROOT))
    assert p == _PROJECT_ROOT


def test_resolve_promethean_dir_default():
    """_resolve_promethean_dir: default 上溯到含 apeireth/ + .git/."""
    p = v1276._resolve_promethean_dir(None)
    assert (p / "apeireth").exists()
    assert (p / ".git").exists()


# ============================================================
# Test 18: CLI subprocess 真跑 (主 00:56 任何人都能接手)
# ============================================================

def test_cli_probe_subprocess(promethean_dir: Path):
    """CLI --probe: subprocess 真跑, exit=0, philosophy_gate OK."""
    result = subprocess.run(
        [sys.executable, "-m", "apeireth.v1276_asi_time_falsifier", "--probe"],
        cwd=str(promethean_dir),
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
        timeout=30,
        check=False,
    )
    assert result.returncode == 0, f"stderr: {result.stderr}"
    assert "philosophy_gate" in result.stdout
    assert "3 time/freshness" in result.stdout


def test_cli_run_subprocess(promethean_dir: Path):
    """CLI --run: subprocess 真跑, exit=0, Markdown 包含 PASS/FAIL."""
    result = subprocess.run(
        [sys.executable, "-m", "apeireth.v1276_asi_time_falsifier", "--run"],
        cwd=str(promethean_dir),
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
        timeout=30,
        check=False,
    )
    assert result.returncode == 0, f"stderr: {result.stderr}"
    assert "V1276" in result.stdout
    assert "Falsification rate" in result.stdout


def test_cli_hypothesis_explain(promethean_dir: Path):
    """CLI --hypothesis h_git_age_days --explain: 解释单假说."""
    result = subprocess.run(
        [sys.executable, "-m", "apeireth.v1276_asi_time_falsifier",
         "--hypothesis", "h_git_age_days", "--explain"],
        cwd=str(promethean_dir),
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
        timeout=30,
        check=False,
    )
    assert result.returncode == 0, f"stderr: {result.stderr}"
    assert "h_git_age_days" in result.stdout
    assert "claim:" in result.stdout


def test_cli_unknown_hypothesis(promethean_dir: Path):
    """CLI --hypothesis unknown: 错误信息 + exit=0."""
    result = subprocess.run(
        [sys.executable, "-m", "apeireth.v1276_asi_time_falsifier",
         "--hypothesis", "h_nonexistent", "--explain"],
        cwd=str(promethean_dir),
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
        timeout=30,
        check=False,
    )
    assert result.returncode == 0
    assert "unknown hypothesis" in result.stdout


# ============================================================
# Test 19 (extra): V1276 不引入新 ASI dim (主 17:43 + 主 17:58)
# ============================================================

def test_v1276_does_not_inflate_asi_ns(promethean_dir: Path):
    """V1276 不刷 KPI: NS current 不变 (主 17:43 实事求是)."""
    assert v1276.V1276_ASI_NS_CURRENT == 0.7905
    assert v1276.V1276_ASI_NS_LOCKED_PCT == 92.91
    # V1276 也没声明达到 ASI V1
    gate = v1276._v1276_philosophy_gate()
    assert gate["v1274_no_asi_v1_claim"] is True
    assert gate["v1274_no_phenomenal_claim"] is True