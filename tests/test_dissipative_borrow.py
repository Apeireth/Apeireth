"""dissipative.py 真生产回归测试.

主 14:06 拉回注意力 + 主 13:31 大胆激进 + 写真 production + 允许犯错.
V4 12 生命特征涌现 (#6) 深化 (dissipative structure) — P1 final.
"""
from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent.parent))

import pytest

from apeireth.dissipative import (
    DISSIPATIVE_VERSION,
    DissipativeState,
    DissipativeStructure,
    entropy_production,
    bifurcation,
    order_parameter_evolution,
    DissipativeNetwork,
)


# === 1. DissipativeState 3 真生产状态 (主 13:08 借鉴 Prigogine) ===

class TestDissipativeStates:
    """Prigogine 3 真生产状态 (主 14:06 借鉴 Prigogine 1977)."""

    def test_3_states_defined(self):
        assert {s.value for s in DissipativeState} == {"equilibrium", "near_equilibrium", "far_equilibrium"}

    def test_equilibrium(self):
        assert DissipativeState.EQUILIBRIUM.value == "equilibrium"

    def test_far_equilibrium(self):
        assert DissipativeState.FAR_EQUILIBRIUM.value == "far_equilibrium"


# === 2. DissipativeStructure 真生产 (主 14:06 真借鉴) ===

class TestDissipativeStructure:
    """DissipativeStructure 真生产 (主 14:06 + Prigogine 1977 诺贝尔奖)."""

    def test_structure_default(self):
        s = DissipativeStructure(structure_id="s1")
        assert s.structure_id == "s1"
        assert s.state == DissipativeState.EQUILIBRIUM
        assert s.order_parameter == 0.0

    def test_structure_to_dict(self):
        s = DissipativeStructure(structure_id="s1", state=DissipativeState.FAR_EQUILIBRIUM,
                                  order_parameter=0.7, flux=1.0, entropy_production=0.7)
        d = s.to_dict()
        assert d["structure_id"] == "s1"
        assert d["state"] == "far_equilibrium"
        assert d["order_parameter"] == 0.7


# === 3. 真生产算法 (主 13:08 借鉴 Prigogine 1977) ===

class TestDissipativeAlgorithms:
    """Dissipative 真生产算法 (主 14:06 借鉴 Prigogine + Nicolis)."""

    def test_entropy_production(self):
        """熵产生真生产 (主 13:08 借鉴 1977 诺贝尔奖)."""
        assert entropy_production(flux=1.0, gradient=1.0) == 1.0
        assert entropy_production(flux=2.0, gradient=0.5) == 1.0
        assert entropy_production(flux=0.0, gradient=1.0) == 0.0

    def test_bifurcation_true(self):
        """control_param > threshold → 分岔 (主 13:08 借鉴 Prigogine)."""
        assert bifurcation(order_parameter=0.0, control_param=0.6, critical_threshold=0.5) is True

    def test_bifurcation_false(self):
        """control_param <= threshold → 不分岔."""
        assert bifurcation(order_parameter=0.0, control_param=0.4, critical_threshold=0.5) is False

    def test_order_parameter_evolution_decay(self):
        """control_param < threshold → 衰减 (主 17:43 实事求是)."""
        result = order_parameter_evolution(order_param=0.5, control_param=0.3)
        assert result < 0.5  # 衰减

    def test_order_parameter_evolution_grow(self):
        """control_param > threshold → 增长 (主 13:08 借鉴 Prigogine)."""
        result = order_parameter_evolution(order_param=0.5, control_param=0.7)
        assert result > 0.5


# === 4. DissipativeNetwork 真生产主类 (主 13:31 大胆激进) ===

class TestDissipativeNetwork:
    """DissipativeNetwork 真生产主类 (主 14:06 拉回注意力)."""

    def test_init_empty(self):
        dn = DissipativeNetwork()
        assert dn.structures == {}

    def test_add_structure(self):
        """添加真生产耗散结构 (主 14:06)."""
        dn = DissipativeNetwork()
        s = dn.add_structure("s1", order_parameter=0.3, flux=1.0)
        assert "s1" in dn.structures
        assert s.order_parameter == 0.3
        assert s.entropy_production == 0.3  # 1.0 * 0.3

    def test_add_structure_no_order(self):
        dn = DissipativeNetwork()
        s = dn.add_structure("s1", order_parameter=0.0, flux=1.0)
        assert s.entropy_production == 0.0

    def test_evolve_below_threshold(self):
        """control_param < threshold → 衰减."""
        dn = DissipativeNetwork()
        dn.add_structure("s1", order_parameter=0.5)
        result = dn.evolve("s1", control_param=0.3, time_steps=5)
        assert result.state == DissipativeState.EQUILIBRIUM
        assert result.order_parameter < 0.5

    def test_evolve_above_threshold(self):
        """control_param > threshold → 远离平衡."""
        dn = DissipativeNetwork()
        dn.add_structure("s1", order_parameter=0.5, flux=1.0)
        result = dn.evolve("s1", control_param=0.7, time_steps=10)
        assert result.state == DissipativeState.FAR_EQUILIBRIUM

    def test_evolve_missing_structure(self):
        dn = DissipativeNetwork()
        result = dn.evolve("nonexistent", control_param=0.7)
        assert result is None

    def test_detect_bifurcation_true(self):
        """真生产分岔检测 (主 13:08 借鉴 Prigogine 1977)."""
        dn = DissipativeNetwork(default_threshold=0.5)
        assert dn.detect_bifurcation("s1", control_param=0.7) is True

    def test_detect_bifurcation_false(self):
        dn = DissipativeNetwork()
        assert dn.detect_bifurcation("s1", control_param=0.3) is False

    def test_stats_with_structures(self):
        """stats() 真生产统计 (主 17:43 实事求是)."""
        dn = DissipativeNetwork()
        dn.add_structure("s1")
        dn.add_structure("s2", order_parameter=0.5, flux=1.0)
        stats = dn.stats()
        assert stats["n_structures"] == 2

    def test_stats_empty(self):
        """空 stats 真生产 (主 17:43 实事求是, 不 placeholder)."""
        dn = DissipativeNetwork()
        stats = dn.stats()
        assert stats["n_structures"] == 0


# === 5. to_dict 真生产 (主 14:06) ===

class TestDissipativeToDict:
    """DissipativeStructure.to_dict() 真生产."""

    def test_structure_to_dict_keys(self):
        s = DissipativeStructure(structure_id="s1")
        d = s.to_dict()
        expected_keys = ["structure_id", "state", "entropy_production", "order_parameter", "flux"]
        for k in expected_keys:
            assert k in d


# === 6. V3 哲学守门 (主 17:58 + 主 20:46) ===

class TestV3PhilosophyGuard:
    """V3 哲学守门: 不假装 Phenomenal / 不假装达到 ASI (主 14:06 拉回注意力)."""

    def test_no_consciousness_field(self):
        """dissipative 不应有假装意识字段."""
        dn = DissipativeNetwork()
        forbidden = ["awareness", "consciousness", "qualia", "phenomenal"]
        for attr in dir(dn):
            for f in forbidden:
                assert f not in attr.lower() or attr in ("add_structure", "evolve", "detect_bifurcation", "stats"), \
                    f"dissipative 不应有假装意识字段: {attr}"

    def test_no_asi_reached_claim(self):
        """dissipative 不应声称已达到 ASI."""
        dn = DissipativeNetwork()
        dn.add_structure("s1", order_parameter=0.5)
        stats = dn.stats()
        for v in stats.values():
            if isinstance(v, str):
                assert "已达到 ASI" not in v
                assert "I am ASI" not in v

    def test_no_dissipative_consciousness_pretend(self):
        """dissipative 借鉴 Prigogine, 不假装"耗散意识"."""
        dn = DissipativeNetwork()
        dn.add_structure("s1", order_parameter=0.5)
        stats = dn.stats()
        philosophy = stats.get("philosophy", "").lower()
        assert "dissipative consciousness" not in philosophy
        assert "asi entropy production" not in philosophy


# === 7. 大胆激进 (主 13:31) 落地 ===

class TestBoldInnovation:
    """主 13:31 大胆激进 + 允许犯错 + 鼓励尝试 真生产落地."""

    def test_dissipative_is_real_innovation(self):
        """dissipative 是真创新 (主 13:31), 不 placeholder."""
        dn = DissipativeNetwork()
        dn.add_structure("s1", order_parameter=0.5, flux=1.0)
        result = dn.evolve("s1", control_param=0.7, time_steps=10)
        assert result.state == DissipativeState.FAR_EQUILIBRIUM

    def test_dissipative_allows_iteration(self):
        """dissipative 允许迭代 (主 13:31 鼓励尝试)."""
        dn = DissipativeNetwork()
        dn.add_structure("s1", order_parameter=0.1)
        for _ in range(3):
            dn.evolve("s1", control_param=0.7, time_steps=5)
        assert dn.structures["s1"].state == DissipativeState.FAR_EQUILIBRIUM


if __name__ == "__main__":
    pytest.main([__file__, "-v"])