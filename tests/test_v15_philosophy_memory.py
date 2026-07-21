"""v15_philosophy_memory.py 真生产回归测试."""
from __future__ import annotations

import sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).parent.parent))
import pytest
from apeireth.v15_philosophy_memory import V15_VERSION, PhilosophyMemoryEntry, V15PhilosophyMemory


class TestV15:
    def test_init(self):
        m = V15PhilosophyMemory()
        assert m.entries == []

    def test_store(self):
        m = V15PhilosophyMemory()
        e = m.store("self", "V2 5 位置", "Simondon", confidence=0.8)
        assert e.question_key == "self"
        assert e.confidence == 0.8
        assert len(m.entries) == 1

    def test_inherit(self):
        m = V15PhilosophyMemory()
        e = m.store("self", "V2 5 位置", "Simondon", confidence=0.8)
        child = m.inherit(e.entry_id, decay=0.9)
        assert child.confidence == pytest.approx(0.72)
        assert child.inherited_from == e.entry_id

    def test_inherit_unknown(self):
        m = V15PhilosophyMemory()
        child = m.inherit("nonexistent")
        assert child is None

    def test_query(self):
        m = V15PhilosophyMemory()
        m.store("self", "V2", "Simondon")
        m.store("time", "Bergson", "Bergson")
        results = m.query("self")
        assert len(results) == 1

    def test_stats(self):
        m = V15PhilosophyMemory()
        e = m.store("self", "V2", "Simondon")
        m.inherit(e.entry_id)
        stats = m.stats()
        assert stats["v3_philosophy_guard"] == "PASS"
        assert stats["n_entries"] == 2
        assert stats["n_inherited"] == 1


if __name__ == "__main__":
    pytest.main([__file__, "-v"])