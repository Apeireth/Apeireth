from __future__ import annotations

import json
from pathlib import Path

import pytest

from apeireth.v1093_dgm_archive import _diff, _hqb, ucb1


def test_ucb1_explores_unpulled_component():
    assert ucb1(0.0, 0, 1) == float("inf")


def test_ucb1_exploits_positive_mean():
    assert ucb1(0.8, 10, 20) > ucb1(0.2, 10, 20)


def test_diff_is_real_unified_diff():
    text = _diff({"generation": 0}, {"generation": 1})
    assert "--- harness.parent" in text
    assert '-  "generation": 0' in text
    assert '+  "generation": 1' in text


def test_hqb_has_v1087_four_dimensions():
    class Snapshot:
        v03_score = 0.8
        philosophy_guard_ok = True
    score = _hqb(Snapshot(), 100.0)
    assert set(score) == {"capability", "cost_efficiency", "latency_margin", "constraint_adherence", "composite"}
    assert 0.0 <= score["composite"] <= 1.0


def test_iterations_boundary():
    from apeireth.v1093_dgm_archive import run_experiment
    with pytest.raises(ValueError):
        run_experiment(0)


def test_static_validation_runs_once_per_experiment(monkeypatch, tmp_path):
    import apeireth.v1093_dgm_archive as module

    class Snapshot:
        v03_score = 0.8
        philosophy_guard_ok = True
        snapshot_id = "test-snapshot"

    class Builder:
        def __init__(self, project_dir):
            pass

        def build(self):
            return Snapshot()

    calls = []

    def fake_run(cmd):
        calls.append(cmd)
        return {"returncode": 0, "duration_ms": 1.0, "stdout_tail": "", "stderr_tail": ""}

    monkeypatch.setattr(module, "ROOT", tmp_path)
    monkeypatch.setattr(module, "OUT", tmp_path)
    monkeypatch.setattr(module, "STATE", tmp_path / "harness_state.json")
    monkeypatch.setattr(module, "StatusSnapshotBuilder", Builder)
    monkeypatch.setattr(module, "_run", fake_run)
    monkeypatch.setattr(module, "_v04", lambda: {"status": "not_measured"})

    archive = module.run_experiment(4)

    assert len(calls) == 2
    assert archive["iterations_completed"] == 3
    for artifact in archive["runs"][1:]:
        record = json.loads((module.ROOT / artifact).read_text(encoding="utf-8"))
        assert record["validation"]["compile"]["returncode"] == 0
        assert record["validation"]["tests"]["returncode"] == 0
