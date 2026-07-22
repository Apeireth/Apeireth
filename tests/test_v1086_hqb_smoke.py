"""V1086 HQB persistence (HQBPersistence) 烟测 (主 21:15 + R2-REQ-01 A).

≥3 烟测: baseline 读 + record 持久化 + asi_delta + 不污染 V1074 artifacts.
边界: 只读 V1074 asi_snapshot.json, 写独立 artifacts/v1086/.
"""
from __future__ import annotations

import json
import sys
from pathlib import Path

import pytest

APEIRETH_DIR = Path(__file__).resolve().parent.parent / "apeireth"
if str(APEIRETH_DIR.parent) not in sys.path:
    sys.path.insert(0, str(APEIRETH_DIR.parent))

from apeireth.v36_hqb_benchmark import HQBScore
from apeireth.v1085_hqb_core import HonestDecisionModule
from apeireth.v1086_hqb_persistence import (  # noqa: E402
    DEFAULT_ARTIFACT_DIR,
    DEFAULT_GUARD_LOG,
    V1086_VERSION,
    HQBPersistence,
)


@pytest.fixture
def tmp_workspace(tmp_path):
    art = tmp_path / "artifacts" / "v1086"
    snap = tmp_path / "artifacts" / "asi_snapshot.json"
    snap.parent.mkdir(parents=True, exist_ok=True)
    return art, snap


def _make_decision(score: float = 0.80, context: str = "smoke"):
    mod = HonestDecisionModule()
    s = HQBScore(score_id="s_smoke", sc=score, nr=score, ev=score, cdt=score)
    return mod.evaluate(s, context=context)


class TestV1086Baseline:
    """烟测 1: baseline 读 (缺失=0, 存在=parse)."""

    def test_baseline_missing_returns_zero(self, tmp_workspace, tmp_path):
        art, _ = tmp_workspace
        missing_snap = tmp_path / "no_snapshot.json"
        p = HQBPersistence(artifact_dir=art, snapshot_path=missing_snap)
        assert p.read_baseline_asi_v03() == 0.0

    def test_baseline_present_parsed(self, tmp_workspace):
        art, snap = tmp_workspace
        snap.write_text(json.dumps({"asi_v03_score": 0.8836}), encoding="utf-8")
        p = HQBPersistence(artifact_dir=art, snapshot_path=snap)
        assert p.read_baseline_asi_v03() == pytest.approx(0.8836)

    def test_baseline_malformed_returns_zero(self, tmp_workspace):
        art, snap = tmp_workspace
        snap.write_text("not-json", encoding="utf-8")
        p = HQBPersistence(artifact_dir=art, snapshot_path=snap)
        assert p.read_baseline_asi_v03() == 0.0


class TestV1086Record:
    """烟测 2: record 写 JSONL + entries 增长."""

    def test_record_appends_jsonl(self, tmp_workspace):
        art, snap = tmp_workspace
        snap.write_text(json.dumps({"asi_v03_score": 0.88}), encoding="utf-8")
        p = HQBPersistence(artifact_dir=art, snapshot_path=snap)
        d = _make_decision(0.80)
        entry = p.record(d)
        assert entry.decision.verdict.value == "accept"
        assert entry.asi_v03_at_record == pytest.approx(0.88)
        # JSONL 文件实际写盘
        assert p.guard_log_path.exists()
        lines = p.guard_log_path.read_text(encoding="utf-8").strip().splitlines()
        assert len(lines) == 1
        row = json.loads(lines[0])
        assert row["verdict"] == "accept"
        assert row["asi_v03_at_record"] == pytest.approx(0.88)

    def test_record_multiple_appends(self, tmp_workspace):
        art, snap = tmp_workspace
        snap.write_text(json.dumps({"asi_v03_score": 0.88}), encoding="utf-8")
        p = HQBPersistence(artifact_dir=art, snapshot_path=snap)
        for score in (0.85, 0.55, 0.20, 1.0):
            d = _make_decision(score)
            p.record(d)
        lines = p.guard_log_path.read_text(encoding="utf-8").strip().splitlines()
        assert len(lines) == 4
        assert len(p.entries) == 4


class TestV1086AsiDelta:
    """烟测 3: asi_delta = current - baseline (主 17:43 delta ≠ ASI)."""

    def test_asi_delta_positive(self, tmp_workspace):
        art, snap = tmp_workspace
        snap.write_text(json.dumps({"asi_v03_score": 0.80}), encoding="utf-8")
        p = HQBPersistence(artifact_dir=art, snapshot_path=snap)
        delta = p.asi_delta(0.90)
        assert delta == pytest.approx(0.10)

    def test_asi_delta_negative(self, tmp_workspace):
        art, snap = tmp_workspace
        snap.write_text(json.dumps({"asi_v03_score": 0.8836}), encoding="utf-8")
        p = HQBPersistence(artifact_dir=art, snapshot_path=snap)
        delta = p.asi_delta(0.80)
        assert delta == pytest.approx(-0.0836)

    def test_asi_delta_zero_when_equal(self, tmp_workspace):
        art, snap = tmp_workspace
        snap.write_text(json.dumps({"asi_v03_score": 0.50}), encoding="utf-8")
        p = HQBPersistence(artifact_dir=art, snapshot_path=snap)
        assert p.asi_delta(0.50) == pytest.approx(0.0)


class TestV1086Isolation:
    """烟测 4: 不污染 V1074 artifacts (主 07-19 4 层安全门)."""

    def test_writes_only_to_v1086_dir(self, tmp_workspace):
        art, snap = tmp_workspace
        snap.write_text(json.dumps({"asi_v03_score": 0.88}), encoding="utf-8")
        p = HQBPersistence(artifact_dir=art, snapshot_path=snap)
        d = _make_decision(0.80)
        p.record(d)
        # V1074 asi_snapshot.json 内容未变 (字节级)
        original = snap.read_bytes()
        p.record(d)  # 再 record 一次
        assert snap.read_bytes() == original
        # V1086 写在自己目录
        assert art.exists()
        assert (art / DEFAULT_GUARD_LOG).exists()

    def test_stats_has_philosophy_and_version(self, tmp_workspace):
        art, snap = tmp_workspace
        snap.write_text(json.dumps({"asi_v03_score": 0.88}), encoding="utf-8")
        p = HQBPersistence(artifact_dir=art, snapshot_path=snap)
        s = p.stats()
        assert s["version"] == V1086_VERSION
        assert "philosophy" in s
        assert s["n_entries"] == 0
        assert "artifact_dir" in s
        assert "baseline_asi_v03" in s
        assert DEFAULT_ARTIFACT_DIR.name == "v1086"


if __name__ == "__main__":
    pytest.main([__file__, "-v"])