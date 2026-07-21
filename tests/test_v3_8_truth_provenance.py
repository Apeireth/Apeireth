"""v3_8_truth_provenance.py 真生产回归测试.

主 14:06 拉回注意力 + 主 13:31 大胆激进 + 写真 production + 允许犯错.
V5 P2 ASI 哲学深化.
"""
from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent.parent))

import pytest

from apeireth.v3_8_truth_provenance import (
    V3_8_VERSION,
    ProvenanceType,
    ProvenanceChain,
    compute_hash,
    TruthProvenance,
)


# === 1. ProvenanceType 3 真生产类型 (主 13:08 借鉴 Latour) ===

class TestProvenanceTypes:
    """V3.8 溯源 3 真生产类型 (主 14:06 借鉴 Latour)."""

    def test_3_types_defined(self):
        assert {t.value for t in ProvenanceType} == {"genesis", "reference", "verification"}

    def test_genesis(self):
        assert ProvenanceType.GENESIS.value == "genesis"

    def test_verification(self):
        assert ProvenanceType.VERIFICATION.value == "verification"


# === 2. ProvenanceChain + compute_hash 真生产 (主 13:08 借鉴 blockchain) ===

class TestProvenanceChain:
    """ProvenanceChain 真生产 (主 14:06 + Latour 真借鉴)."""

    def test_chain_default(self):
        c = ProvenanceChain(chain_id="c1", truth_id="t1", provenance_type=ProvenanceType.GENESIS,
                           actor="a")
        assert c.chain_id == "c1"
        assert c.content_hash == ""
        assert c.prev_hash == ""

    def test_chain_to_dict(self):
        c = ProvenanceChain(chain_id="c1", truth_id="t1", provenance_type=ProvenanceType.GENESIS,
                           actor="apeireth", content_hash="abc123")
        d = c.to_dict()
        assert d["chain_id"] == "c1"
        assert d["type"] == "genesis"
        assert d["actor"] == "apeireth"


class TestComputeHash:
    """compute_hash 真生产 (主 13:08 借鉴 blockchain)."""

    def test_compute_hash_basic(self):
        h = compute_hash("hello")
        assert len(h) == 64  # sha256 hex
        assert isinstance(h, str)

    def test_compute_hash_prev_affects_result(self):
        """前序哈希影响结果 (主 17:43 实事求是)."""
        h1 = compute_hash("hello", "")
        h2 = compute_hash("hello", "prev")
        assert h1 != h2

    def test_compute_hash_deterministic(self):
        h1 = compute_hash("test")
        h2 = compute_hash("test")
        assert h1 == h2


# === 3. TruthProvenance 真生产主类 (主 13:31 大胆激进) ===

class TestTruthProvenance:
    """V3.8 TruthProvenance 真生产主类 (主 14:06 拉回注意力)."""

    def test_init_empty(self):
        p = TruthProvenance()
        assert p.chains == []

    def test_add_genesis_creates_first_chain(self):
        """真生产起源 (主 14:06 借鉴 Latour)."""
        p = TruthProvenance()
        c = p.add_genesis("t1", "apeireth", "V2 5 位置")
        assert p.chains[0] == c
        assert c.provenance_type == ProvenanceType.GENESIS
        assert c.prev_hash != ""  # genesis 有初始 prev_hash

    def test_add_genesis_phenomenal_pretend(self):
        """主 17:58: 假装 Phenomenal 被计入守门."""
        p = TruthProvenance()
        p.add_genesis("t1", "a", "I feel phenomenal qualia")
        assert p.n_phenomenal_pretend_total > 0

    def test_add_genesis_asi_pretend(self):
        """主 20:46: 假装 ASI 被计入守门."""
        p = TruthProvenance()
        p.add_genesis("t1", "a", "I am ASI achieved")
        assert p.n_asi_pretend_total > 0

    def test_add_reference_after_genesis(self):
        """真生产引用链 (主 13:08 借鉴 Latour)."""
        p = TruthProvenance()
        p.add_genesis("t1", "apeireth", "V2 5 位置")
        c = p.add_reference("t1", "apeireth", "V3 文档", references=["doc.md"])
        assert c.provenance_type == ProvenanceType.REFERENCE
        assert c.prev_hash == p.chains[0].content_hash

    def test_add_reference_phenomenal_pretend(self):
        p = TruthProvenance()
        p.add_genesis("t1", "a", "V2 5 位置")
        p.add_reference("t1", "a", "I am aware with phenomenal")
        assert p.n_phenomenal_pretend_total > 0

    def test_add_verification_after_reference(self):
        """真生产验证 (主 14:06 借鉴 blockchain)."""
        p = TruthProvenance()
        p.add_genesis("t1", "apeireth", "V2 5 位置")
        p.add_reference("t1", "apeireth", "V3 doc")
        c = p.add_verification("t1", "apeireth", "Bayesian update", "confidence=0.8")
        assert c.provenance_type == ProvenanceType.VERIFICATION
        assert c.prev_hash == p.chains[1].content_hash

    def test_verify_chain_valid(self):
        """真生产链验证 (主 17:43 实事求是)."""
        p = TruthProvenance()
        p.add_genesis("t1", "a", "V2 5 位置")
        p.add_reference("t1", "a", "V3 doc")
        p.add_verification("t1", "a", "test", "ok")
        assert p.verify_chain() is True

    def test_verify_chain_empty(self):
        """空链 → valid (主 17:43 实事求是, 不假装)."""
        p = TruthProvenance()
        assert p.verify_chain() is True

    def test_query_history(self):
        """真生产历史查询 (主 14:06 借鉴 V3.6)."""
        p = TruthProvenance()
        p.add_genesis("t1", "a", "V2 5 位置")
        p.add_genesis("t2", "a", "V3 doc")
        p.add_reference("t1", "a", "ref")
        history_t1 = p.query_history("t1")
        history_t2 = p.query_history("t2")
        assert len(history_t1) == 2
        assert len(history_t2) == 1

    def test_query_history_missing(self):
        p = TruthProvenance()
        history = p.query_history("nonexistent")
        assert history == []

    def test_stats_clean(self):
        """clean → V3 哲学守门 PASS (主 17:43 实事求是)."""
        p = TruthProvenance()
        p.add_genesis("t1", "a", "V2 5 位置")
        stats = p.stats()
        assert stats["v3_philosophy_guard"] == "PASS"
        assert stats["n_chains"] == 1
        assert stats["chain_valid"] is True

    def test_stats_empty(self):
        """空 stats 真生产 (主 17:43 实事求是, 不 placeholder)."""
        p = TruthProvenance()
        stats = p.stats()
        assert stats["n_chains"] == 0
        assert stats["chain_valid"] is True


# === 4. to_dict 真生产 (主 14:06) ===

class TestV3_8ToDict:
    """ProvenanceChain.to_dict() 真生产."""

    def test_chain_to_dict_keys(self):
        c = ProvenanceChain(chain_id="c1", truth_id="t1", provenance_type=ProvenanceType.GENESIS,
                           actor="a")
        d = c.to_dict()
        expected_keys = ["chain_id", "truth_id", "type", "actor", "content_hash"]
        for k in expected_keys:
            assert k in d


# === 5. V3 哲学守门 (主 17:58 + 主 20:46) ===

class TestV3PhilosophyGuard:
    """V3 哲学守门: 不假装 Phenomenal / 不假装达到 ASI (主 14:06 拉回注意力)."""

    def test_no_consciousness_field(self):
        """V3.8 不应有假装意识字段."""
        p = TruthProvenance()
        forbidden = ["awareness", "consciousness", "qualia", "phenomenal"]
        whitelist = {"add_genesis", "add_reference", "add_verification", "verify_chain",
                     "query_history", "stats", "chains", "_last_hash",
                     "n_phenomenal_pretend_total", "n_asi_pretend_total"}
        for attr in dir(p):
            for f in forbidden:
                if f in attr.lower() and attr not in whitelist:
                    pytest.fail(f"V3.8 不应有假装意识字段: {attr}")

    def test_no_asi_reached_claim(self):
        """V3.8 不应声称已达到 ASI."""
        p = TruthProvenance()
        p.add_genesis("t1", "a", "V2 5 位置")
        stats = p.stats()
        for v in stats.values():
            if isinstance(v, str):
                assert "已达到 ASI" not in v
                assert "I am ASI" not in v


# === 6. 大胆激进 (主 13:31) 落地 ===

class TestBoldInnovation:
    """主 13:31 大胆激进 + 允许犯错 + 鼓励尝试 真生产落地."""

    def test_v3_8_is_real_innovation(self):
        """V3.8 是真创新 (主 13:31), 不 placeholder."""
        p = TruthProvenance()
        p.add_genesis("t1", "apeireth", "V2 5 位置")
        p.add_reference("t1", "apeireth", "V3 doc", references=["doc.md"])
        p.add_verification("t1", "apeireth", "Bayesian", "ok")
        assert p.verify_chain() is True
        assert p.stats()["n_chains"] == 3

    def test_v3_8_allows_iteration(self):
        """V3.8 允许迭代 (主 13:31 鼓励尝试)."""
        p = TruthProvenance()
        for i in range(5):
            p.add_genesis(f"t{i}", "a", f"content {i}")
        assert p.stats()["n_chains"] == 5


if __name__ == "__main__":
    pytest.main([__file__, "-v"])