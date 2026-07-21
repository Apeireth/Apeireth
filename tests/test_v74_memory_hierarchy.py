"""v74_memory_hierarchy.py 真生产回归测试."""
from __future__ import annotations

import sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).parent.parent))
import pytest
from apeireth.v74_memory_hierarchy import (
    V74_VERSION, MemoryTier, MemoryEntry, V74MemoryHierarchy,
)


class TestV74:
    def test_init(self):
        mem = V74MemoryHierarchy()
        assert mem.entries == {}

    def test_add_entry(self):
        mem = V74MemoryHierarchy()
        eid = mem.add_entry("test", tier=MemoryTier.STM)
        assert eid in mem.entries

    def test_recall(self):
        mem = V74MemoryHierarchy()
        mem.add_entry("Apeireth ASI", tier=MemoryTier.STM)
        results = mem.recall("Apeireth")
        assert len(results) > 0

    def test_recall_tier_filter(self):
        mem = V74MemoryHierarchy()
        mem.add_entry("LTM content", tier=MemoryTier.LTM)
        results = mem.recall("content", tier=MemoryTier.LTM)
        assert len(results) > 0

    def test_promote_to_ltm(self):
        mem = V74MemoryHierarchy()
        eid = mem.add_entry("test", tier=MemoryTier.STM, importance=0.8)
        # 模拟 3 次访问
        mem.entries[eid].access_count = 5
        result = mem.promote_to_ltm(eid)
        assert result is True
        assert mem.entries[eid].tier == MemoryTier.LTM

    def test_promote_to_ltm_no_meet_criteria(self):
        mem = V74MemoryHierarchy()
        eid = mem.add_entry("test", tier=MemoryTier.STM, importance=0.3)
        mem.entries[eid].access_count = 5
        result = mem.promote_to_ltm(eid)
        assert result is False

    def test_promote_unknown(self):
        mem = V74MemoryHierarchy()
        assert mem.promote_to_ltm("unknown") is False

    def test_n_entries(self):
        mem = V74MemoryHierarchy()
        mem.add_entry("a")
        mem.add_entry("b")
        assert mem.n_entries() == 2

    def test_stats(self):
        mem = V74MemoryHierarchy()
        mem.add_entry("test", tier=MemoryTier.LTM)
        stats = mem.stats()
        assert stats["n_entries"] == 1


if __name__ == "__main__":
    pytest.main([__file__, "-v"])