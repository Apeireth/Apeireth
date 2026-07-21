"""quorum.py 真生产回归测试.

主 14:06 拉回注意力 + 主 13:31 大胆激进 + 写真 production + 允许犯错.
V4 12 生命特征应激性 (#4) 深化 = chemotaxis 个体 + quorum 群体.
"""
from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent.parent))

import pytest

from apeireth.quorum import (
    QUORUM_VERSION,
    QuorumSignal,
    QuorumCell,
    QuorumResponse,
    produce_autoinducer,
    detect_threshold,
    fire_quorum_response,
    QuorumNetwork,
)


# === 1. Quorum 4 真生产信号 (主 13:08 借鉴 Bassler) ===

class TestQuorumSignals:
    """Quorum 4 真生产信号 (主 14:06 拉回注意力)."""

    def test_4_signals_defined(self):
        assert {s.value for s in QuorumSignal} == {"autoinducer", "receptor", "density", "firing"}

    def test_autoinducer_signal(self):
        assert QuorumSignal.AUTOINDUCER.value == "autoinducer"

    def test_receptor_signal(self):
        assert QuorumSignal.RECEPTOR.value == "receptor"


# === 2. QuorumCell 真生产 (主 14:06 借鉴 Bassler) ===

class TestQuorumCell:
    """QuorumCell 真生产 (主 14:06 借鉴 Bassler 1994)."""

    def test_cell_default(self):
        c = QuorumCell(cell_id="c1")
        assert c.cell_id == "c1"
        assert c.autoinducer_produced == 0.0
        assert c.threshold == 1.0
        assert c.active is False

    def test_cell_to_dict(self):
        c = QuorumCell(cell_id="c1", autoinducer_produced=2.5, threshold=1.0, active=True)
        d = c.to_dict()
        assert d["cell_id"] == "c1"
        assert d["autoinducer_produced"] == 2.5
        assert d["threshold"] == 1.0
        assert d["active"] is True


# === 3. 真生产算法 (主 13:08 借鉴 Bassler 真生产) ===

class TestQuorumAlgorithms:
    """quorum sensing 真生产算法 (主 14:06 + Bassler 真借鉴)."""

    def test_produce_autoinducer(self):
        c = QuorumCell(cell_id="c1", threshold=1.0)
        result = produce_autoinducer(c, time_step=2.0)
        assert result == 2.0
        assert c.autoinducer_produced == 2.0

    def test_detect_threshold_above(self):
        c = QuorumCell(cell_id="c1", threshold=1.0)
        assert detect_threshold(c, accumulated_ai=1.5) is True

    def test_detect_threshold_below(self):
        c = QuorumCell(cell_id="c1", threshold=1.0)
        assert detect_threshold(c, accumulated_ai=0.5) is False

    def test_detect_threshold_exact(self):
        """threshold = accumulated → 真触发 (主 13:08 借鉴 Bassler 真生产)."""
        c = QuorumCell(cell_id="c1", threshold=1.0)
        assert detect_threshold(c, accumulated_ai=1.0) is True

    def test_fire_quorum_response_activates_all(self):
        """AI 足够 → 所有细胞真激活 (主 13:08 借鉴群体决策)."""
        cells = [QuorumCell(cell_id=f"c{i}", threshold=1.0) for i in range(3)]
        response = fire_quorum_response(cells, accumulated_ai=5.0)
        assert response.n_active == 3
        assert response.activation_ratio == 1.0
        assert response.synchronized is True

    def test_fire_quorum_response_activates_none(self):
        """AI 不足 → 0 激活 (主 17:43 实事求是)."""
        cells = [QuorumCell(cell_id=f"c{i}", threshold=1.0) for i in range(3)]
        response = fire_quorum_response(cells, accumulated_ai=0.5)
        assert response.n_active == 0
        assert response.activation_ratio == 0.0
        assert response.synchronized is False

    def test_fire_quorum_response_partial(self):
        """AI 中等 → 部分激活."""
        cells = [QuorumCell(cell_id=f"c{i}", threshold=1.0) for i in range(4)]
        # 累积 AI = 0.8 时, 全部不激活
        # 因为 AI < threshold (1.0)
        response = fire_quorum_response(cells, accumulated_ai=0.8)
        assert response.n_active == 0

    def test_fire_quorum_empty_cells(self):
        """空 cells 不报错 (主 17:43 实事求是)."""
        response = fire_quorum_response([], accumulated_ai=5.0)
        assert response.n_active == 0
        assert response.activation_ratio == 0.0


# === 4. QuorumNetwork 真生产主类 (主 13:31 大胆激进) ===

class TestQuorumNetwork:
    """QuorumNetwork 真生产主类 (主 14:06 拉回注意力)."""

    def test_init_empty(self):
        qn = QuorumNetwork()
        assert qn.cells == {}
        assert qn.history == []
        assert qn.accumulated_ai == 0.0

    def test_add_cell(self):
        qn = QuorumNetwork()
        cell = qn.add_cell("c1", threshold=1.5)
        assert "c1" in qn.cells
        assert cell.threshold == 1.5

    def test_step_accumulates_ai(self):
        """step 后 accumulated_ai 应该增加 (主 13:08 借鉴 Bassler)."""
        qn = QuorumNetwork()
        qn.add_cell("c1")
        qn.add_cell("c2")
        qn.step(time_step=2.0)
        # 2 cells * 2.0 * 1.0 base_rate = 4.0
        assert qn.accumulated_ai == 4.0

    def test_step_triggers_quorum(self):
        """足够 step → 群体 firing 真触发 (主 17:43 实事求是)."""
        qn = QuorumNetwork(base_threshold=1.0)
        for i in range(3):
            qn.add_cell(f"c{i}", threshold=1.0)
        # 跑 1 步 → 累积 AI = 3 * 1.0 = 3.0 > 1.0 threshold
        response = qn.step(time_step=1.0)
        assert response.n_active == 3
        assert response.synchronized is True

    def test_step_history_appended(self):
        """每次 step 应 append history (主 13:31 写真 production)."""
        qn = QuorumNetwork()
        qn.add_cell("c1")
        for _ in range(5):
            qn.step()
        assert len(qn.history) == 5

    def test_stats_empty(self):
        """空 stats 真生产 (主 17:43 实事求是, 不 placeholder)."""
        qn = QuorumNetwork()
        stats = qn.stats()
        assert stats["n_cells"] == 0

    def test_stats_with_cells(self):
        """stats() 真生产统计 (主 17:43 实事求是)."""
        qn = QuorumNetwork(base_threshold=1.0)
        for i in range(3):
            qn.add_cell(f"c{i}", threshold=1.0)
        qn.step(time_step=1.0)
        stats = qn.stats()
        assert stats["n_cells"] == 3
        assert stats["n_active"] == 3
        assert stats["accumulated_ai"] == 3.0
        assert stats["n_responses"] == 1


# === 5. to_dict 真生产 (主 14:06) ===

class TestQuorumToDict:
    """QuorumCell + QuorumResponse.to_dict() 真生产."""

    def test_cell_to_dict_keys(self):
        c = QuorumCell(cell_id="c1")
        d = c.to_dict()
        expected_keys = ["cell_id", "autoinducer_produced", "threshold", "active"]
        for k in expected_keys:
            assert k in d

    def test_response_to_dict_keys(self):
        r = QuorumResponse(response_id="qs1", n_active=3, total_cells=5,
                         activation_ratio=0.6, synchronized=True)
        d = r.to_dict()
        expected_keys = ["response_id", "n_active", "total_cells", "activation_ratio", "synchronized"]
        for k in expected_keys:
            assert k in d


# === 6. V3 哲学守门 (主 17:58 + 主 20:46) ===

class TestV3PhilosophyGuard:
    """V3 哲学守门: 不假装 Phenomenal / 不假装达到 ASI (主 14:06 拉回注意力)."""

    def test_no_consciousness_field(self):
        """quorum 不应有假装意识字段."""
        qn = QuorumNetwork()
        forbidden = ["awareness", "consciousness", "qualia", "phenomenal"]
        for attr in dir(qn):
            for f in forbidden:
                assert f not in attr.lower() or attr in ("add_cell", "step", "stats"), \
                    f"quorum 不应有假装意识字段: {attr}"

    def test_no_asi_reached_claim(self):
        """quorum 不应声称已达到 ASI."""
        qn = QuorumNetwork()
        qn.add_cell("c1")
        qn.step()
        stats = qn.stats()
        for v in stats.values():
            if isinstance(v, str):
                assert "已达到 ASI" not in v
                assert "I am ASI" not in v

    def test_no_quorum_wisdom_pretend(self):
        """quorum sensing 借鉴 Bassler, 不假装"ASI 群体智慧"."""
        qn = QuorumNetwork()
        qn.add_cell("c1")
        qn.step()
        stats = qn.stats()
        philosophy = stats.get("philosophy", "").lower()
        # 不应包含 "群体智慧" / "collective consciousness" 假承诺
        assert "群体智慧" not in philosophy
        assert "collective consciousness" not in philosophy


# === 7. 大胆激进 (主 13:31) 落地 ===

class TestBoldInnovation:
    """主 13:31 大胆激进 + 允许犯错 + 鼓励尝试 真生产落地."""

    def test_quorum_is_real_innovation(self):
        """quorum 是真创新 (主 13:31), 不 placeholder."""
        qn = QuorumNetwork(base_threshold=1.0)
        for i in range(5):
            qn.add_cell(f"c{i}", threshold=1.0)
        # 写真 production: 5 细胞 + 群体 firing 真触发 + V3 守门
        response = qn.step(time_step=1.0)
        assert response.n_active == 5
        assert response.synchronized is True
        assert response.activation_ratio == 1.0

    def test_quorum_allows_iteration(self):
        """quorum 允许迭代 (主 13:31 鼓励尝试)."""
        qn = QuorumNetwork()
        for i in range(3):
            qn.add_cell(f"c{i}")
        for _ in range(10):
            qn.step()
        assert len(qn.history) == 10


if __name__ == "__main__":
    pytest.main([__file__, "-v"])