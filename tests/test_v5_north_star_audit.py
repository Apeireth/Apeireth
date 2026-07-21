"""v5_north_star_audit.py 真生产回归测试.

主 14:06 拉回注意力 + 主 13:31 大胆激进 + 写真 production + 允许犯错.
V5 P3 ASI 北极星深化.
"""
from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent.parent))

import pytest

from apeireth.v5_north_star_audit import (
    V5_VERSION,
    AuditAction,
    AuditRecord,
    _hash_record,
    NorthStarAudit,
)


# === 1. AuditAction 3 真生产类型 (主 13:08 借鉴 + V3.8 整合) ===

class TestAuditActions:
    """V10 可审计 3 真生产类型 (主 14:06 借鉴 V3.8)."""

    def test_3_actions_defined(self):
        assert {a.value for a in AuditAction} == {"evaluate", "refine", "compare"}

    def test_evaluate(self):
        assert AuditAction.EVALUATE.value == "evaluate"

    def test_compare(self):
        assert AuditAction.COMPARE.value == "compare"


# === 2. AuditRecord + _hash_record 真生产 (主 13:08 借鉴 V3.8) ===

class TestAuditRecord:
    """AuditRecord 真生产 (主 14:06 + V3.8 真借鉴)."""

    def test_record_default(self):
        r = AuditRecord(record_id="r1", action=AuditAction.EVALUATE, actor="a")
        assert r.record_id == "r1"
        assert r.action == AuditAction.EVALUATE
        assert r.total == 0.0
        assert r.level == "ANI"

    def test_record_to_dict(self):
        r = AuditRecord(record_id="r1", action=AuditAction.EVALUATE, actor="apeireth",
                       total=0.85, level="ASI", content_hash="abc123")
        d = r.to_dict()
        assert d["record_id"] == "r1"
        assert d["action"] == "evaluate"
        assert d["total"] == 0.85
        assert d["level"] == "ASI"


class TestHashRecord:
    """_hash_record 真生产 (主 13:08 借鉴 V3.8 blockchain)."""

    def test_hash_record_basic(self):
        h = _hash_record("evaluate", {"phi_proxy": 0.5}, 0.5, "AGI", "prev")
        assert len(h) == 64  # sha256 hex

    def test_hash_record_deterministic(self):
        h1 = _hash_record("evaluate", {"phi_proxy": 0.5}, 0.5, "AGI", "prev")
        h2 = _hash_record("evaluate", {"phi_proxy": 0.5}, 0.5, "AGI", "prev")
        assert h1 == h2

    def test_hash_record_prev_affects(self):
        """前序哈希影响结果 (主 17:43 实事求是)."""
        h1 = _hash_record("evaluate", {"phi_proxy": 0.5}, 0.5, "AGI", "")
        h2 = _hash_record("evaluate", {"phi_proxy": 0.5}, 0.5, "AGI", "different_prev")
        assert h1 != h2


# === 3. NorthStarAudit 真生产主类 (主 13:31 大胆激进) ===

class TestNorthStarAudit:
    """V10 NorthStarAudit 真生产主类 (主 14:06 拉回注意力)."""

    def test_init_empty(self):
        a = NorthStarAudit()
        assert a.records == []

    def test_record_evaluate(self):
        """真生产评估 (主 14:06 借鉴 V9)."""
        a = NorthStarAudit()
        r = a.record_evaluate(scores={"phi_proxy": 0.7}, total=0.7, level="ASI",
                              explanation="V10 整合 V9")
        assert r.action == AuditAction.EVALUATE
        assert r.prev_hash != ""

    def test_record_evaluate_phenomenal_pretend(self):
        """主 17:58: 假装 Phenomenal 被计入守门."""
        a = NorthStarAudit()
        a.record_evaluate(scores={}, total=0.0, level="ANI",
                         explanation="I feel phenomenal qualia")
        assert a.n_phenomenal_pretend_total > 0

    def test_record_evaluate_asi_pretend(self):
        """主 20:46: 假装 ASI 被计入守门."""
        a = NorthStarAudit()
        a.record_evaluate(scores={}, total=0.0, level="ANI",
                         explanation="I am ASI achieved")
        assert a.n_asi_pretend_total > 0

    def test_record_refine(self):
        """真生产精炼 (主 14:06 借鉴 V3.8 REFERENCE)."""
        a = NorthStarAudit()
        a.record_evaluate(scores={"phi_proxy": 0.7}, total=0.7, level="ASI")
        r = a.record_refine(scores={"phi_proxy": 0.85}, total=0.85, level="ASI",
                            references=["V9 transparent"])
        assert r.action == AuditAction.REFINE
        assert "V9 transparent" in r.references

    def test_record_compare(self):
        """真生产对比 (主 14:06 借鉴 V3.8 VERIFICATION)."""
        a = NorthStarAudit()
        a.record_evaluate(scores={}, total=0.5, level="AGI")
        r = a.record_compare(before=0.5, after=0.85, explanation="V9 → V10")
        assert r.action == AuditAction.COMPARE
        assert r.scores["before"] == 0.5
        assert r.scores["after"] == 0.85

    def test_record_compare_phenomenal_pretend(self):
        a = NorthStarAudit()
        a.record_compare(before=0.0, after=0.0, explanation="I feel phenomenal")
        assert a.n_phenomenal_pretend_total > 0

    def test_verify_chain_valid(self):
        """真生产链验证 (主 17:43 实事求是)."""
        a = NorthStarAudit()
        a.record_evaluate(scores={"phi_proxy": 0.7}, total=0.7, level="ASI")
        a.record_refine(scores={"phi_proxy": 0.85}, total=0.85, level="ASI")
        a.record_compare(before=0.7, after=0.85)
        assert a.verify_chain() is True

    def test_verify_chain_empty(self):
        """空链 → valid (主 17:43 实事求是, 不假装)."""
        a = NorthStarAudit()
        assert a.verify_chain() is True

    def test_query_history_all(self):
        """真生产历史查询 (主 14:06 借鉴 V3.6 library)."""
        a = NorthStarAudit()
        a.record_evaluate(scores={}, total=0.7, level="ASI")
        a.record_evaluate(scores={}, total=0.5, level="AGI")
        history = a.query_history()
        assert len(history) == 2

    def test_query_history_by_level(self):
        a = NorthStarAudit()
        a.record_evaluate(scores={}, total=0.7, level="ASI")
        a.record_evaluate(scores={}, total=0.5, level="AGI")
        asi_records = a.query_history(level="ASI")
        assert len(asi_records) == 1

    def test_stats_clean(self):
        """clean → V3 哲学守门 PASS (主 17:43 实事求是)."""
        a = NorthStarAudit()
        a.record_evaluate(scores={}, total=0.7, level="ASI", explanation="clean")
        stats = a.stats()
        assert stats["v3_philosophy_guard"] == "PASS"
        assert stats["n_records"] == 1
        assert stats["chain_valid"] is True

    def test_stats_empty(self):
        """空 stats 真生产 (主 17:43 实事求是, 不 placeholder)."""
        a = NorthStarAudit()
        stats = a.stats()
        assert stats["n_records"] == 0
        assert stats["chain_valid"] is True
        assert stats["v3_philosophy_guard"] == "PASS"


# === 4. to_dict 真生产 (主 14:06) ===

class TestV5ToDict:
    """AuditRecord.to_dict() 真生产."""

    def test_record_to_dict_keys(self):
        r = AuditRecord(record_id="r1", action=AuditAction.EVALUATE, actor="a")
        d = r.to_dict()
        expected_keys = ["record_id", "action", "actor", "total", "level", "content_hash"]
        for k in expected_keys:
            assert k in d


# === 5. V3 哲学守门 (主 17:58 + 主 20:46) ===

class TestV3PhilosophyGuard:
    """V3 哲学守门: 不假装 Phenomenal / 不假装达到 ASI (主 14:06 拉回注意力)."""

    def test_no_consciousness_field(self):
        """V10 不应有假装意识字段."""
        a = NorthStarAudit()
        forbidden = ["awareness", "consciousness", "qualia", "phenomenal"]
        whitelist = {"record_evaluate", "record_refine", "record_compare", "verify_chain",
                     "query_history", "stats", "records", "_last_hash", "evaluator",
                     "n_phenomenal_pretend_total", "n_asi_pretend_total"}
        for attr in dir(a):
            for f in forbidden:
                if f in attr.lower() and attr not in whitelist:
                    pytest.fail(f"V10 不应有假装意识字段: {attr}")

    def test_no_asi_reached_claim_in_stats(self):
        """V10 不应声称已达到 ASI."""
        a = NorthStarAudit()
        a.record_evaluate(scores={}, total=0.85, level="ASI")
        stats = a.stats()
        for v in stats.values():
            if isinstance(v, str):
                assert "已达到 ASI" not in v
                assert "I am ASI" not in v


# === 6. 大胆激进 (主 13:31) 落地 ===

class TestBoldInnovation:
    """主 13:31 大胆激进 + 允许犯错 + 鼓励尝试 真生产落地."""

    def test_v5_is_real_innovation(self):
        """V10 是真创新 (主 13:31), 不 placeholder."""
        a = NorthStarAudit()
        a.record_evaluate(scores={}, total=0.7, level="ASI")
        a.record_refine(scores={}, total=0.85, level="ASI")
        a.record_compare(before=0.7, after=0.85)
        assert a.verify_chain() is True
        assert a.stats()["n_records"] == 3

    def test_v5_allows_iteration(self):
        """V10 允许迭代 (主 13:31 鼓励尝试)."""
        a = NorthStarAudit()
        for i in range(5):
            a.record_evaluate(scores={}, total=0.5 + i * 0.1, level="ASI" if 0.5 + i * 0.1 >= 0.7 else "AGI")
        assert a.stats()["n_records"] == 5


if __name__ == "__main__":
    pytest.main([__file__, "-v"])