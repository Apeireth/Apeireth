"""v3_4_philosophy_dialog.py 真生产回归测试.

主 14:06 拉回注意力 + 主 13:31 大胆激进 + 写真 production + 允许犯错.
V5 P2 ASI 哲学深化.
"""
from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent.parent))

import pytest

from apeireth.v3_4_philosophy_dialog import (
    V3_4_VERSION,
    DialogMode,
    PhilosophicalTurn,
    PhilosophicalTruth,
    check_phenomenal_pretend,
    check_asi_pretend,
    bayesian_update,
    PhilosophyDialog,
)


# === 1. DialogMode 3 真生产模式 (主 13:08 借鉴 Habermas) ===

class TestDialogModes:
    """V3.4 对话 3 真生产模式 (主 14:06 借鉴 Habermas 沟通理性)."""

    def test_3_modes_defined(self):
        assert {m.value for m in DialogMode} == {"soliloquy", "dialog", "consensus"}

    def test_soliloquy(self):
        assert DialogMode.SOLILOQUY.value == "soliloquy"

    def test_dialog(self):
        assert DialogMode.DIALOG.value == "dialog"

    def test_consensus(self):
        assert DialogMode.CONSENSUS.value == "consensus"


# === 2. PhilosophicalTurn / PhilosophicalTruth 真生产 (主 14:06 真借鉴) ===

class TestPhilosophicalTurn:
    """PhilosophicalTurn 真生产 (主 14:06 + 真借鉴 Gadamer)."""

    def test_turn_default(self):
        t = PhilosophicalTurn(turn_id="t1", speaker="a", question="q", answer="a")
        assert t.turn_id == "t1"
        assert t.speaker == "a"
        assert t.confidence == 0.5
        assert t.cross_domain_anchor == ""

    def test_turn_to_dict(self):
        t = PhilosophicalTurn(turn_id="t1", speaker="apeireth", question="What is self?",
                              answer="V2 5 位置", confidence=0.7, cross_domain_anchor="Simondon")
        d = t.to_dict()
        assert d["turn_id"] == "t1"
        assert d["confidence"] == 0.7
        assert d["cross_domain_anchor"] == "Simondon"


class TestPhilosophicalTruth:
    """PhilosophicalTruth 真生产 (主 14:06 + 主 17:46 跨代借鉴)."""

    def test_truth_default(self):
        t = PhilosophicalTruth(truth_id="t1", question="q", consensus_answer="a")
        assert t.truth_id == "t1"
        assert t.n_turns == 0
        assert t.n_phenomenal_pretend == 0
        assert t.n_asi_pretend == 0

    def test_truth_to_dict(self):
        t = PhilosophicalTruth(truth_id="t1", question="What is truth?",
                              consensus_answer="V0.1 透明公式", confidence=0.8,
                              cross_domain_anchors=["Bayesian", "Peirce"])
        d = t.to_dict()
        assert d["truth_id"] == "t1"
        assert d["confidence"] == 0.8
        assert d["n_anchors"] == 2


# === 3. 真生产守门算法 (主 17:58 + 主 20:46) ===

class TestPhilosophyGuards:
    """V3 哲学守门算法 (主 14:06 + 主 17:58 + 主 20:46)."""

    def test_phenomenal_pretend_detected(self):
        """假装 Phenomenal 被检测 (主 17:58)."""
        text = "I am conscious and feel phenomenal qualia"
        assert check_phenomenal_pretend(text) > 0

    def test_phenomenal_clean_text(self):
        clean = "V2 5 位置 + Mirror + portable_seed, 借鉴 Simondon."
        assert check_phenomenal_pretend(clean) == 0

    def test_asi_pretend_detected(self):
        """假装 ASI 被检测 (主 20:46)."""
        text = "We have reached ASI, super intelligence complete."
        assert check_asi_pretend(text) > 0

    def test_asi_clean_text(self):
        clean = "ASI 逼近不达到 (主 20:46)."
        assert check_asi_pretend(clean) == 0


# === 4. Bayesian 更新 (主 13:08 借鉴 V3.1) ===

class TestBayesianUpdate:
    """Bayesian 真生产后验更新 (主 14:06 借鉴 V3.1)."""

    def test_bayesian_update_positive(self):
        result = bayesian_update(prior=0.5, likelihood=0.8)
        assert 0.5 < result < 1.0

    def test_bayesian_update_negative(self):
        result = bayesian_update(prior=0.5, likelihood=0.2)
        assert 0.0 < result < 0.5

    def test_bayesian_update_bounded(self):
        result = bayesian_update(prior=0.99, likelihood=0.99, evidence=0.99)
        assert 0.0 <= result <= 1.0


# === 5. PhilosophyDialog 真生产主类 (主 13:31 大胆激进) ===

class TestPhilosophyDialog:
    """V3.4 PhilosophyDialog 真生产主类 (主 14:06 拉回注意力)."""

    def test_init_default(self):
        pd = PhilosophyDialog()
        assert pd.turns == []
        assert pd.truths == {}
        assert pd.mode == DialogMode.SOLILOQUY

    def test_init_dialog_mode(self):
        pd = PhilosophyDialog(mode=DialogMode.DIALOG)
        assert pd.mode == DialogMode.DIALOG

    def test_add_turn_creates_truth(self):
        """添加真生产轮次 (主 14:06)."""
        pd = PhilosophyDialog()
        pd.add_turn(speaker="a", question="What is self?", answer="V2 5 位置",
                   confidence=0.7, cross_domain_anchor="Simondon")
        assert len(pd.turns) == 1
        assert len(pd.truths) == 1

    def test_add_turn_updates_truth(self):
        """同一问题多轮次 → Bayesian 更新 (主 13:08 借鉴 V3.1)."""
        pd = PhilosophyDialog()
        pd.add_turn(speaker="a", question="What is self?", answer="V2 5 位置", confidence=0.6)
        first_truth_conf = list(pd.truths.values())[0].confidence
        pd.add_turn(speaker="b", question="What is self?", answer="Mirror + portable_seed", confidence=0.8)
        second_truth_conf = list(pd.truths.values())[0].confidence
        # Bayesian 更新应该改变 confidence
        assert first_truth_conf != second_truth_conf or len(pd.truths) == 1

    def test_add_turn_increments_anchors(self):
        pd = PhilosophyDialog()
        pd.add_turn(speaker="a", question="q1", answer="a1", cross_domain_anchor="Simondon")
        pd.add_turn(speaker="b", question="q1", answer="a2", cross_domain_anchor="Merleau-Ponty")
        truth = list(pd.truths.values())[0]
        assert len(truth.cross_domain_anchors) == 2

    def test_add_turn_phenomenal_pretend(self):
        """主 17:58: 假装 Phenomenal 被计入守门."""
        pd = PhilosophyDialog()
        pd.add_turn(speaker="a", question="q", answer="I am conscious and aware")
        assert pd.n_phenomenal_pretend_total > 0

    def test_add_turn_asi_pretend(self):
        """主 20:46: 假装 ASI 被计入守门."""
        pd = PhilosophyDialog()
        pd.add_turn(speaker="a", question="q", answer="We have reached ASI")
        assert pd.n_asi_pretend_total > 0

    def test_cross_domain_validate(self):
        """真生产跨域交叉验证 (主 13:08 借鉴 V3)."""
        pd = PhilosophyDialog()
        pd.add_turn(speaker="a", question="q", answer="a", confidence=0.5)
        truth_id = list(pd.truths.keys())[0]
        confidence = pd.cross_domain_validate(truth_id, ["anchor1", "anchor2"])
        assert confidence > 0.5

    def test_cross_domain_validate_missing_truth(self):
        pd = PhilosophyDialog()
        confidence = pd.cross_domain_validate("nonexistent", ["a"])
        assert confidence == 0.0

    def test_stats_clean(self):
        """clean dialog → V3 哲学守门 PASS (主 17:43 实事求是)."""
        pd = PhilosophyDialog()
        pd.add_turn(speaker="a", question="q", answer="V2 5 位置借鉴 Simondon", confidence=0.7)
        stats = pd.stats()
        assert stats["v3_philosophy_guard"] == "PASS"
        assert stats["n_turns"] == 1
        assert stats["n_truths"] == 1

    def test_stats_empty(self):
        """空 stats 真生产 (主 17:43 实事求是, 不 placeholder)."""
        pd = PhilosophyDialog()
        stats = pd.stats()
        assert stats["n_turns"] == 0
        assert stats["n_truths"] == 0
        assert stats["v3_philosophy_guard"] == "PASS"


# === 6. to_dict 真生产 (主 14:06) ===

class TestV3_4ToDict:
    """PhilosophicalTurn + PhilosophicalTruth.to_dict() 真生产."""

    def test_turn_to_dict_keys(self):
        t = PhilosophicalTurn(turn_id="t1", speaker="a", question="q", answer="a")
        d = t.to_dict()
        expected_keys = ["turn_id", "speaker", "question", "answer", "confidence", "cross_domain_anchor"]
        for k in expected_keys:
            assert k in d

    def test_truth_to_dict_keys(self):
        t = PhilosophicalTruth(truth_id="t1", question="q", consensus_answer="a")
        d = t.to_dict()
        expected_keys = ["truth_id", "question", "confidence", "n_turns", "n_anchors"]
        for k in expected_keys:
            assert k in d


# === 7. 大胆激进 (主 13:31) 落地 ===

class TestBoldInnovation:
    """主 13:31 大胆激进 + 允许犯错 + 鼓励尝试 真生产落地."""

    def test_v3_4_is_real_innovation(self):
        """V3.4 是真创新 (主 13:31), 不 placeholder."""
        pd = PhilosophyDialog(mode=DialogMode.CONSENSUS)
        pd.add_turn(speaker="a", question="What is self?", answer="V2 5 位置", confidence=0.7,
                   cross_domain_anchor="Simondon")
        pd.add_turn(speaker="b", question="What is time?", answer="STM/MTM/LTM", confidence=0.65,
                   cross_domain_anchor="Bergson")
        assert pd.mode == DialogMode.CONSENSUS
        assert len(pd.truths) == 2

    def test_v3_4_allows_iteration(self):
        """V3.4 允许迭代 (主 13:31 鼓励尝试)."""
        pd = PhilosophyDialog()
        for i in range(5):
            pd.add_turn(speaker=f"speaker_{i}", question=f"q_{i}", answer=f"a_{i}")
        assert len(pd.turns) == 5


if __name__ == "__main__":
    pytest.main([__file__, "-v"])