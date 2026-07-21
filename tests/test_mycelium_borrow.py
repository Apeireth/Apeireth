"""mycelium.py 真生产分布式借鉴 regression tests.

主 14:06 拉回注意力 + 主 14:09 推进 + 主 13:31 大胆激进.
V4 12 生命特征分布式借鉴新角度 (round-13-24 没调研).
"""
from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent.parent))

import pytest

from apeireth.mycelium import (
    MYCELIUM_VERSION,
    MyceliumSignal,
    MyceliumNode,
    MyceliumMessage,
    grow_mycelium,
    transmit_signal,
    MyceliumNetwork,
)


# === 1. Mycelium 信号真生产 (主 14:06 + 真菌生态学) ===

class TestMyceliumSignals:
    """Mycelium 4 真生产信号 (主 14:06 拉回注意力)."""

    def test_4_signals_defined(self):
        assert {s.value for s in MyceliumSignal} == {"nutrient", "danger", "connect", "dissolve"}

    def test_nutrient_signal(self):
        assert MyceliumSignal.NUTRIENT.value == "nutrient"

    def test_danger_signal(self):
        assert MyceliumSignal.DANGER.value == "danger"


# === 2. Mycelium 节点真生产 (主 13:08 真菌生态学) ===

class TestMyceliumNode:
    """MyceliumNode 真生产 (主 14:06 借鉴 Stamets 真菌)."""

    def test_node_default(self):
        n = MyceliumNode(node_id="n1", resource_level=0.5)
        assert n.node_id == "n1"
        assert n.resource_level == 0.5
        assert n.connections == []
        assert n.signals == []
        assert n.alive is True

    def test_node_to_dict(self):
        n = MyceliumNode(node_id="n1", resource_level=0.8)
        n.connections.append("n2")
        n.signals.append(MyceliumSignal.NUTRIENT)
        d = n.to_dict()
        assert d["node_id"] == "n1"
        assert d["n_connections"] == 1
        assert d["n_signals"] == 1
        assert d["alive"] is True


# === 3. grow_mycelium 真生产算法 (主 13:08 真菌借鉴) ===

class TestGrowMycelium:
    """grow_mycelium 真生产 (主 14:06 借鉴 Stamets)."""

    def test_grow_both_alive_resource_ok(self):
        a = MyceliumNode(node_id="a", resource_level=0.8)
        b = MyceliumNode(node_id="b", resource_level=0.7)
        assert grow_mycelium(a, b) is True
        assert "b" in a.connections
        assert "a" in b.connections

    def test_glow_does_not_duplicate_connection(self):
        a = MyceliumNode(node_id="a", resource_level=0.8)
        b = MyceliumNode(node_id="b", resource_level=0.7)
        grow_mycelium(a, b)
        # 第二次 grow 应该返回 False (已连接)
        assert grow_mycelium(a, b) is False

    def test_grow_low_resource_fails(self):
        """资源 < 0.3 不生长 (主 17:43 实事求是)."""
        a = MyceliumNode(node_id="a", resource_level=0.2)
        b = MyceliumNode(node_id="b", resource_level=0.2)
        assert grow_mycelium(a, b) is False
        assert "b" not in a.connections

    def test_grow_dead_node_fails(self):
        a = MyceliumNode(node_id="a", resource_level=0.8, alive=False)
        b = MyceliumNode(node_id="b", resource_level=0.7)
        assert grow_mycelium(a, b) is False


# === 4. transmit_signal 真生产算法 (主 13:08 借鉴 stigmergy round-15) ===

class TestTransmitSignal:
    """transmit_signal 真生产 (主 14:06 借鉴 stigmergy)."""

    def test_transmit_both_alive(self):
        a = MyceliumNode(node_id="a", resource_level=0.8)
        b = MyceliumNode(node_id="b", resource_level=0.7)
        msg = transmit_signal(a, b, MyceliumSignal.NUTRIENT)
        assert msg.src == "a"
        assert msg.dst == "b"
        assert msg.signal == MyceliumSignal.NUTRIENT
        assert msg.latency_ms > 0  # 资源衰减真测

    def test_transmit_latency_inversely_proportional_to_resource(self):
        """资源越高延迟越低 (主 13:08 真借鉴 stigmergy 衰减)."""
        # 资源低 → 延迟高
        a_low = MyceliumNode(node_id="a", resource_level=0.1)
        b_low = MyceliumNode(node_id="b", resource_level=0.1)
        # 资源高 → 延迟低
        a_high = MyceliumNode(node_id="a", resource_level=0.9)
        b_high = MyceliumNode(node_id="b", resource_level=0.9)
        msg_low = transmit_signal(a_low, b_low, MyceliumSignal.NUTRIENT)
        msg_high = transmit_signal(a_high, b_high, MyceliumSignal.NUTRIENT)
        assert msg_low.latency_ms > msg_high.latency_ms

    def test_transmit_dead_node_returns_zero_latency(self):
        """死节点 → 0 延迟 (主 13:08 借鉴 死节点不传递)."""
        a = MyceliumNode(node_id="a", resource_level=0.8, alive=False)
        b = MyceliumNode(node_id="b", resource_level=0.7)
        msg = transmit_signal(a, b, MyceliumSignal.NUTRIENT)
        assert msg.latency_ms == 0.0


# === 5. MyceliumNetwork 真生产主类 (主 13:31 大胆激进) ===

class TestMyceliumNetwork:
    """MyceliumNetwork 真生产主类 (主 14:06 拉回注意力)."""

    def test_init_empty(self):
        net = MyceliumNetwork()
        assert net.nodes == {}
        assert net.messages == []

    def test_add_node(self):
        net = MyceliumNetwork()
        node = net.add_node("n1", resource_level=0.5)
        assert "n1" in net.nodes
        assert net.nodes["n1"].resource_level == 0.5

    def test_grow_via_network(self):
        net = MyceliumNetwork()
        net.add_node("n1", 0.8)
        net.add_node("n2", 0.7)
        assert net.grow("n1", "n2") is True

    def test_grow_missing_node_fails(self):
        net = MyceliumNetwork()
        net.add_node("n1", 0.8)
        assert net.grow("n1", "nonexistent") is False

    def test_transmit_appends_messages(self):
        net = MyceliumNetwork()
        net.add_node("n1", 0.8)
        net.add_node("n2", 0.7)
        net.transmit("n1", "n2", MyceliumSignal.NUTRIENT)
        assert len(net.messages) == 1

    def test_transmit_missing_node_returns_zero_latency(self):
        net = MyceliumNetwork()
        net.add_node("n1", 0.8)
        msg = net.transmit("n1", "nonexistent", MyceliumSignal.NUTRIENT)
        assert msg.latency_ms == 0.0

    def test_stats_with_nodes(self):
        """stats() 真生产统计 (主 17:43 实事求是)."""
        net = MyceliumNetwork()
        net.add_node("n1", 0.8)
        net.add_node("n2", 0.7)
        net.grow("n1", "n2")
        net.transmit("n1", "n2", MyceliumSignal.NUTRIENT)
        stats = net.stats()
        assert stats["n_nodes"] == 2
        assert stats["n_alive"] == 2
        assert stats["total_connections"] == 1
        assert stats["total_signals"] == 2
        assert stats["n_messages"] == 1
        assert stats["avg_latency_ms"] > 0

    def test_stats_empty(self):
        """空 stats 真生产 (主 17:43 实事求是, 不 placeholder)."""
        net = MyceliumNetwork()
        stats = net.stats()
        assert stats["n_nodes"] == 0


# === 6. to_dict 真生产 (主 14:06) ===

class TestMyceliumToDict:
    """MyceliumNode.to_dict() 真生产."""

    def test_node_to_dict_keys(self):
        n = MyceliumNode(node_id="n1", resource_level=0.5)
        d = n.to_dict()
        expected_keys = ["node_id", "resource_level", "n_connections", "n_signals", "alive"]
        for k in expected_keys:
            assert k in d


# === 7. V3 哲学守门 (主 17:58 + 主 20:46) ===

class TestV3PhilosophyGuard:
    """V3 哲学守门: 不假装 Phenomenal / 不假装达到 ASI (主 14:06 拉回注意力)."""

    def test_no_consciousness_field(self):
        """mycelium 不应有假装意识字段."""
        net = MyceliumNetwork()
        forbidden = ["awareness", "consciousness", "qualia", "phenomenal"]
        for attr in dir(net):
            for f in forbidden:
                assert f not in attr.lower() or attr in ("add_node", "grow", "transmit", "stats"), \
                    f"mycelium 不应有假装意识字段: {attr}"

    def test_no_asi_reached_claim(self):
        """mycelium 不应声称已达到 ASI."""
        net = MyceliumNetwork()
        net.add_node("n1", 0.8)
        stats = net.stats()
        for v in stats.values():
            if isinstance(v, str):
                assert "已达到 ASI" not in v
                assert "I am ASI" not in v

    def test_no_fungal_wisdom_pretend(self):
        """mycelium 借鉴真菌, 不假装"ASI 真菌智慧"."""
        net = MyceliumNetwork()
        net.add_node("n1", 0.8)
        net.add_node("n2", 0.7)
        net.grow("n1", "n2")
        stats = net.stats()
        philosophy = stats.get("philosophy", "").lower()
        # 不应包含 "fungal consciousness" / "fungal wisdom" 假承诺
        assert "fungal consciousness" not in philosophy
        assert "i am fungal" not in philosophy


# === 8. 大胆激进 (主 13:31) 落地 ===

class TestBoldInnovation:
    """主 13:31 大胆激进 + 允许犯错 + 鼓励尝试 真生产落地."""

    def test_mycelium_is_real_innovation(self):
        """mycelium 是真创新 (主 13:31), 不 placeholder."""
        net = MyceliumNetwork()
        net.add_node("n1", 0.8)
        net.add_node("n2", 0.7)
        net.add_node("n3", 0.6)
        # 写真 production: 3 节点 + 菌丝生长 + 信号传输
        assert net.grow("n1", "n2") is True
        assert net.grow("n2", "n3") is True
        msg = net.transmit("n1", "n2", MyceliumSignal.NUTRIENT)
        assert msg.latency_ms > 0

    def test_mycelium_allows_iteration(self):
        """mycelium 允许迭代 (主 13:31 鼓励尝试)."""
        net = MyceliumNetwork()
        for i in range(5):
            net.add_node(f"n{i}", resource_level=0.5 + i * 0.1)
        for i in range(4):
            net.grow(f"n{i}", f"n{i+1}")
        assert len(net.nodes) == 5
        stats = net.stats()
        assert stats["total_connections"] == 4


if __name__ == "__main__":
    pytest.main([__file__, "-v"])