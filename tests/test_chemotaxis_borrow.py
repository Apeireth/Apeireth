"""chemotaxis.py 真生产生物学借鉴 regression tests.

主 14:06 拉回注意力 + 主 13:31 大胆激进 + 写真 production + 允许犯错.
主 13:08 哲学/科学/跨领域 — chemotaxis 真借鉴.
"""
from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent.parent))

import pytest

from apeireth.chemotaxis import (
    CHEMOTAXIS_VERSION,
    ChemotaxisPhase,
    ChemotaxisSignal,
    ChemotaxisResponse,
    detect_signal,
    adapt_signal,
    regulate_response,
    act_motor,
    Chemotaxis,
)


# === 1. chemotaxis 4 阶段 (主 13:08 借鉴 bacterial CheY/CheZ) ===

class TestChemotaxis4Phases:
    """chemotaxis 4 阶段真生产 (主 14:06 + 借鉴 CheY/CheZ)."""

    def test_4_phases_defined(self):
        assert {p.value for p in ChemotaxisPhase} == {"detect", "adapt", "regulate", "act"}

    def test_phase_order(self):
        """4 阶段顺序: detect → adapt → regulate → act (主 14:06 借鉴 bacterial)."""
        phases = [p.value for p in ChemotaxisPhase]
        assert phases == ["detect", "adapt", "regulate", "act"]


# === 2. chemotaxis 真信号 (主 14:06 真生产) ===

class TestChemotaxisSignal:
    """chemotaxis 真信号真生产 (主 14:06)."""

    def test_signal_default(self):
        s = ChemotaxisSignal(signal_id="s1", ligand="glucose", concentration=0.5, delta_concentration=+0.01)
        assert s.signal_id == "s1"
        assert s.ligand == "glucose"
        assert s.concentration == 0.5
        assert s.delta_concentration == +0.01


# === 3. 4 阶段函数真生产 ===

class TestPhaseFunctions:
    """4 阶段函数真生产 (主 14:06 + 主 13:31 写真 production)."""

    def test_detect_signal_strong(self):
        """强信号应被检测."""
        s = ChemotaxisSignal(signal_id="s1", ligand="glucose", concentration=0.5, delta_concentration=+0.01)
        assert detect_signal(s, threshold=0.001) is True

    def test_detect_signal_weak(self):
        """弱信号应被过滤 (主 17:43 实事求是)."""
        s = ChemotaxisSignal(signal_id="s1", ligand="leucine", concentration=0.05, delta_concentration=0.0001)
        assert detect_signal(s, threshold=0.001) is False

    def test_regulate_attract(self):
        """吸引信号 → 顺梯度 → run 增长, tumble 减少 (主 13:08 真借鉴 CheY-P 减少)."""
        s = ChemotaxisSignal(signal_id="s1", ligand="glucose", concentration=0.5, delta_concentration=+0.01)
        r = regulate_response(s)
        assert r.direction_bias < 0
        assert r.run_count > 0
        assert r.tumble_count == 0

    def test_regulate_repel(self):
        """驱避信号 → 逆梯度 → tumble 增长, run 减少 (主 13:08 真借鉴 CheY-P 增加)."""
        s = ChemotaxisSignal(signal_id="s2", ligand="leucine", concentration=0.3, delta_concentration=-0.01)
        r = regulate_response(s)
        assert r.direction_bias > 0
        assert r.tumble_count > 0
        assert r.run_count == 0

    def test_act_motor_sets_phase_act(self):
        """act_motor 应设置 phase=ACT (主 14:06 真生产)."""
        s = ChemotaxisSignal(signal_id="s1", ligand="glucose", concentration=0.5, delta_concentration=+0.01)
        r = regulate_response(s)
        r = act_motor(r)
        assert r.phase == ChemotaxisPhase.ACT
        assert r.latency_ms > 0


# === 4. chemotaxis 真生产主类 (主 13:31 大胆激进) ===

class TestChemotaxisClass:
    """Chemotaxis 真生产主类 (主 13:31 写真 production)."""

    def test_process_signal_strong_attract(self):
        chem = Chemotaxis()
        s = ChemotaxisSignal(signal_id="s1", ligand="glucose", concentration=0.5, delta_concentration=+0.01)
        r = chem.process_signal(s)
        assert r.phase == ChemotaxisPhase.ACT
        assert r.direction_bias < 0
        assert r.latency_ms > 0

    def test_process_signal_strong_repel(self):
        chem = Chemotaxis()
        s = ChemotaxisSignal(signal_id="s2", ligand="leucine", concentration=0.3, delta_concentration=-0.01)
        r = chem.process_signal(s)
        assert r.phase == ChemotaxisPhase.ACT
        assert r.direction_bias > 0

    def test_process_signal_weak_returns_detect_only(self):
        """弱信号 → 只 detect 不 regulate (主 17:43 实事求是)."""
        chem = Chemotaxis()
        s = ChemotaxisSignal(signal_id="s3", ligand="leucine", concentration=0.05, delta_concentration=0.0001)
        r = chem.process_signal(s)
        assert r.phase == ChemotaxisPhase.DETECT
        assert r.run_count == 0
        assert r.tumble_count == 0

    def test_history_appended(self):
        """每次 process_signal 应 append history (主 13:31 写真 production)."""
        chem = Chemotaxis()
        for i in range(3):
            chem.process_signal(ChemotaxisSignal(
                signal_id=f"s{i}", ligand="glucose",
                concentration=0.5, delta_concentration=+0.01,
            ))
        assert len(chem.history) == 3

    def test_stats_with_responses(self):
        """stats() 真生产统计 (主 17:43 实事求是)."""
        chem = Chemotaxis()
        chem.process_signal(ChemotaxisSignal(signal_id="s1", ligand="glucose", concentration=0.5, delta_concentration=+0.01))
        chem.process_signal(ChemotaxisSignal(signal_id="s2", ligand="leucine", concentration=0.3, delta_concentration=-0.01))
        stats = chem.stats()
        assert stats["n_responses"] == 2
        assert stats["n_attract_responses"] == 1
        assert stats["n_repel_responses"] == 1
        assert stats["n_neutral_responses"] == 0

    def test_stats_empty(self):
        """空 stats 真生产 (主 17:43 实事求是, 不 placeholder)."""
        chem = Chemotaxis()
        stats = chem.stats()
        assert stats["n_responses"] == 0


# === 5. to_dict 真生产 (主 13:31) ===

class TestChemotaxisToDict:
    """ChemotaxisResponse.to_dict() 真生产 (主 14:06)."""

    def test_response_to_dict_keys(self):
        r = ChemotaxisResponse(
            response_id="r1", signal_id="s1",
            phase=ChemotaxisPhase.ACT,
        )
        d = r.to_dict()
        expected_keys = ["response_id", "signal_id", "phase", "tumble_count", "run_count", "direction_bias", "latency_ms"]
        for k in expected_keys:
            assert k in d


# === 6. V3 哲学守门 (主 17:58 + 主 20:46) ===

class TestV3PhilosophyGuard:
    """V3 哲学守门: 不假装 Phenomenal / 不假装达到 ASI (主 14:06 拉回注意力)."""

    def test_no_consciousness_field(self):
        """chemotaxis 不应有假装意识字段."""
        chem = Chemotaxis()
        forbidden = ["awareness", "consciousness", "qualia", "phenomenal"]
        for attr in dir(chem):
            for f in forbidden:
                assert f not in attr.lower() or attr in ("process_signal", "stats"), \
                    f"chemotaxis 不应有假装意识字段: {attr}"

    def test_no_asi_reached_claim(self):
        """chemotaxis 不应声称已达到 ASI."""
        chem = Chemotaxis()
        stats = chem.stats()
        for v in stats.values():
            if isinstance(v, str):
                assert "已达到 ASI" not in v
                assert "I am ASI" not in v


# === 7. 大胆激进 (主 13:31) 落地 ===

class TestBoldInnovation:
    """主 13:31 大胆激进 + 允许犯错 + 鼓励尝试 真生产落地."""

    def test_chemotaxis_is_real_innovation(self):
        """chemotaxis 是真创新 (主 13:31), 不 placeholder."""
        chem = Chemotaxis()
        s = ChemotaxisSignal(signal_id="s1", ligand="glucose", concentration=0.5, delta_concentration=+0.01)
        r = chem.process_signal(s)
        # 写真 production: 4 阶段 + 真借鉴 CheY/CheZ + V3 守门
        assert r.phase == ChemotaxisPhase.ACT
        assert r.direction_bias < 0  # 吸引
        assert r.run_count > 0
        assert r.latency_ms > 0

    def test_chemotaxis_allows_iteration(self):
        """chemotaxis 允许迭代 (主 13:31 鼓励尝试)."""
        chem = Chemotaxis()
        for i in range(5):
            r = chem.process_signal(ChemotaxisSignal(
                signal_id=f"s{i}", ligand="glucose",
                concentration=0.5, delta_concentration=+0.01,
            ))
            assert r
        assert len(chem.history) == 5


if __name__ == "__main__":
    pytest.main([__file__, "-v"])