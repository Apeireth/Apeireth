"""Tests for P0 Omnibus Acceptance Workflow runner."""
from __future__ import annotations

import json
import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT))

from apeireth import p0_workflow as p0  # noqa: E402


# ---- helpers ----

def _ok_measure():
    return {
        "level_score": 0.90,
        "n_modules": 1200,
        "n_tests": 6000,
        "n_commits": 500,
        "philosophy_guard_ok": True,
    }


def _ok_regress():
    return {"total": 100, "passed": 100, "failed": 0}


def _bad_measure_missing_field():
    return {"level_score": 0.90, "n_modules": 1200, "n_tests": 6000, "n_commits": 500}


def _fail_validate_low_score():
    return {
        "level_score": 0.70,  # below 0.85 floor
        "n_modules": 1200,
        "n_tests": 6000,
        "n_commits": 500,
        "philosophy_guard_ok": True,
    }


def _fail_validate_guard_false():
    return {
        "level_score": 0.90,
        "n_modules": 1200,
        "n_tests": 6000,
        "n_commits": 500,
        "philosophy_guard_ok": False,
    }


def _fail_regress_low_rate():
    return {"total": 100, "passed": 80, "failed": 20}


def _raise_measure():
    raise RuntimeError("snapshot unavailable")


def _raise_regress():
    raise RuntimeError("pytest crashed")


# ---- tests ----

def test_run_happy_path(tmp_path):
    r = p0.run(rollout_root=tmp_path, measure_fn=_ok_measure, regress_fn=_ok_regress)
    assert r.status == "PASSED"
    assert len(r.stages) == 4  # measure + validate + display + regress (evidence not a stage)
    assert [s.id for s in r.stages] == ["measure", "validate", "display", "regress"]
    assert r.evidence_path and Path(r.evidence_path).exists()
    assert r.rollback_path is None
    assert r.human_prompt is None  # 0.90 < 0.98
    payload = json.loads(Path(r.evidence_path).read_text(encoding="utf-8"))
    assert payload["status"] == "PASSED"


def test_major_milestone_098_triggers_human_prompt(tmp_path):
    def m():
        out = _ok_measure()
        out["level_score"] = 0.98
        return out

    r = p0.run(rollout_root=tmp_path, measure_fn=m, regress_fn=_ok_regress)
    assert r.status == "PASSED"
    assert r.human_prompt is not None
    assert "MAJOR_MILESTONE" in r.human_prompt


def test_measure_missing_field_fails_fast(tmp_path):
    r = p0.run(rollout_root=tmp_path, measure_fn=_bad_measure_missing_field, regress_fn=_ok_regress)
    assert r.status == "FAILED"
    assert r.stages[0].ok is False
    assert "missing field" in (r.stages[0].error or "")
    assert r.evidence_path and Path(r.evidence_path).exists()


def test_measure_exception_fails_fast(tmp_path):
    r = p0.run(rollout_root=tmp_path, measure_fn=_raise_measure, regress_fn=_ok_regress)
    assert r.status == "FAILED"
    assert "raised" in (r.stages[0].error or "")


def test_validate_hard_gate_rollback_on_low_score(tmp_path):
    r = p0.run(rollout_root=tmp_path, measure_fn=_fail_validate_low_score, regress_fn=_ok_regress)
    assert r.status == "ROLLED_BACK"
    validate = next(s for s in r.stages if s.id == "validate")
    assert validate.ok is False
    assert "level_score" in (validate.error or "")
    assert r.rollback_path and Path(r.rollback_path).exists()
    rb = json.loads(Path(r.rollback_path).read_text(encoding="utf-8"))
    assert rb["workflow_id"] == "p0_omnibus_acceptance"
    assert r.evidence_path and Path(r.evidence_path).exists()


def test_validate_hard_gate_rollback_on_guard_false(tmp_path):
    r = p0.run(rollout_root=tmp_path, measure_fn=_fail_validate_guard_false, regress_fn=_ok_regress)
    assert r.status == "ROLLED_BACK"
    assert "philosophy_guard_ok" in (r.stages[1].error or "")


def test_regress_low_pass_rate_rolls_back(tmp_path):
    r = p0.run(rollout_root=tmp_path, measure_fn=_ok_measure, regress_fn=_fail_regress_low_rate)
    assert r.status == "ROLLED_BACK"
    regress = next(s for s in r.stages if s.id == "regress")
    assert regress.ok is False
    assert "pass_rate" in (regress.error or "")
    assert r.rollback_path and Path(r.rollback_path).exists()


def test_regress_exception_rolls_back(tmp_path):
    r = p0.run(rollout_root=tmp_path, measure_fn=_ok_measure, regress_fn=_raise_regress)
    assert r.status == "ROLLED_BACK"
    assert "raised" in (r.stages[-1].error or "")


def test_display_failure_is_non_blocking(tmp_path):
    def bad_display(_summary):
        raise RuntimeError("renderer crash")

    r = p0.run(
        rollout_root=tmp_path,
        measure_fn=_ok_measure,
        regress_fn=_ok_regress,
        display_fn=bad_display,
    )
    # display 失败应被吞掉, workflow 仍 PASSED
    assert r.status == "PASSED"
    display_stage = next(s for s in r.stages if s.id == "display")
    assert display_stage.ok is True
    assert "warning" in display_stage.output


def test_evidence_always_written_even_on_rollback(tmp_path):
    r = p0.run(rollout_root=tmp_path, measure_fn=_fail_validate_low_score, regress_fn=_ok_regress)
    assert r.status == "ROLLED_BACK"
    assert r.evidence_path and Path(r.evidence_path).exists()
    assert r.rollback_path and Path(r.rollback_path).exists()
    # evidence 必须含 rollback 标记
    payload = json.loads(Path(r.evidence_path).read_text(encoding="utf-8"))
    assert payload["status"] == "ROLLED_BACK"
    assert payload["rollback_path"]


def test_no_human_prompt_below_098(tmp_path):
    def m():
        out = _ok_measure()
        out["level_score"] = 0.95
        return out

    r = p0.run(rollout_root=tmp_path, measure_fn=m, regress_fn=_ok_regress)
    assert r.human_prompt is None  # 0.95 < 0.98, auto_continue


def test_requires_callbacks():
    with pytest.raises(ValueError):
        p0.run(measure_fn=None, regress_fn=_ok_regress)  # type: ignore[arg-type]
    with pytest.raises(ValueError):
        p0.run(measure_fn=_ok_measure, regress_fn=None)  # type: ignore[arg-type]


def test_config_loads_and_has_five_stages():
    cfg = p0._load_config(p0.DEFAULT_CONFIG)
    assert cfg["workflow_id"] == "p0_omnibus_acceptance"
    assert len(cfg["stages"]) == 5
    ids = {s["id"] for s in cfg["stages"]}
    assert ids == {"measure", "validate", "display", "regress", "evidence"}
    hard = next(s for s in cfg["stages"] if s["id"] == "validate")["hard_gates"]
    assert hard["level_score_min"] == 0.8500
    assert hard["philosophy_guard_ok_required"] is True


def test_to_dict_roundtrip(tmp_path):
    r = p0.run(rollout_root=tmp_path, measure_fn=_ok_measure, regress_fn=_ok_regress)
    d = r.to_dict()
    assert d["workflow_id"] == "p0_omnibus_acceptance"
    assert d["status"] == "PASSED"
    # 必须可 JSON 序列化 (无不可序列化对象)
    json.dumps(d)
