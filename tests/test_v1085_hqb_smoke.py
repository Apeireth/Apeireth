"""V1085 HQB core (HonestDecisionModule) 烟测 (主 21:15 + R2-REQ-01 A).

≥3 烟测: threshold_validation + 3 verdict paths + veto (哲学守门) + stats + invalid input.
不测 V1074 / V1081 / philosophy_guard (边界外).
"""
from __future__ import annotations

import sys
from pathlib import Path

import pytest

APEIRETH_DIR = Path(__file__).resolve().parent.parent / "apeireth"
if str(APEIRETH_DIR.parent) not in sys.path:
    sys.path.insert(0, str(APEIRETH_DIR.parent))

from apeireth.v36_hqb_benchmark import HQBScore
from apeireth.v1085_hqb_core import (  # noqa: E402
    DEFAULT_ACCEPT_THRESHOLD,
    DEFAULT_REJECT_THRESHOLD,
    V1085_VERSION,
    HonestDecisionModule,
    Verdict,
)


def _hqb(sc: float, sid: str = "s") -> HQBScore:
    return HQBScore(score_id=sid, sc=sc, nr=sc, ev=sc, cdt=sc)


class TestV1085Thresholds:
    """烟测 1: 阈值默认合法 + 非法拒绝."""

    def test_default_thresholds_valid(self):
        m = HonestDecisionModule()
        assert m.accept_threshold == DEFAULT_ACCEPT_THRESHOLD
        assert m.reject_threshold == DEFAULT_REJECT_THRESHOLD
        assert m.veto_threshold == 0.95
        assert V1085_VERSION == "0.1.0"

    def test_invalid_thresholds_rejected(self):
        with pytest.raises(ValueError):
            HonestDecisionModule(accept_threshold=0.5, reject_threshold=0.6)  # reject >= accept


class TestV1085Verdicts:
    """烟测 2: 3 个核心 verdict 路径 (accept/review/reject) + veto."""

    def test_evaluate_accept_high(self):
        m = HonestDecisionModule()
        d = m.evaluate(_hqb(0.85), context="smoke")
        assert d.verdict == Verdict.ACCEPT
        assert d.score_used == pytest.approx(0.85)
        assert "accept" in d.reason
        assert d.hqb_score_id == "s"

    def test_evaluate_review_mid(self):
        m = HonestDecisionModule()
        d = m.evaluate(_hqb(0.55), context="smoke")
        assert d.verdict == Verdict.REVIEW
        assert "borderline" in d.reason

    def test_evaluate_reject_low(self):
        m = HonestDecisionModule()
        d = m.evaluate(_hqb(0.20), context="smoke")
        assert d.verdict == Verdict.REJECT
        assert "insufficient" in d.reason

    def test_evaluate_veto_perfect_score(self):
        """烟测: 1.0 → veto (主 17:58 不假装)."""
        m = HonestDecisionModule()
        d = m.evaluate(_hqb(1.0), context="smoke")
        assert d.verdict == Verdict.VETO
        assert "philosophy" in d.reason or "guard" in d.reason


class TestV1085Stats:
    """烟测 3: stats 计数正确 + 哲学字段存在."""

    def test_stats_empty(self):
        m = HonestDecisionModule()
        s = m.stats()
        assert s["n_decisions"] == 0
        assert s["by_verdict"] == {}
        assert "philosophy" in s
        assert s["version"] == V1085_VERSION

    def test_stats_increments(self):
        m = HonestDecisionModule()
        m.evaluate(_hqb(0.85, "a"))
        m.evaluate(_hqb(0.55, "b"))
        m.evaluate(_hqb(0.20, "c"))
        m.evaluate(_hqb(1.0, "d"))
        s = m.stats()
        assert s["n_decisions"] == 4
        assert s["by_verdict"]["accept"] == 1
        assert s["by_verdict"]["review"] == 1
        assert s["by_verdict"]["reject"] == 1
        assert s["by_verdict"]["veto"] == 1

    def test_decision_to_dict_serializable(self):
        m = HonestDecisionModule()
        d = m.evaluate(_hqb(0.80))
        dd = d.to_dict()
        assert dd["verdict"] == "accept"
        assert isinstance(dd["decision_id"], str)
        assert isinstance(dd["score_used"], float)
        assert "reason" in dd


if __name__ == "__main__":
    pytest.main([__file__, "-v"])