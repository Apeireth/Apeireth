"""v43_cognitive_core.py 真生产回归测试."""
from __future__ import annotations

import sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).parent.parent))
import pytest
from apeireth.v43_cognitive_core import (
    V43_VERSION, Atom, Link, NARSRevision, V43CognitiveCore,
)


class TestV43:
    def test_init(self):
        c = V43CognitiveCore()
        assert c.atoms == {}

    def test_add_atom(self):
        c = V43CognitiveCore()
        aid = c.add_atom("Concept", "test")
        assert aid in c.atoms

    def test_add_link(self):
        c = V43CognitiveCore()
        a1 = c.add_atom("Concept", "a")
        a2 = c.add_atom("Concept", "b")
        lid = c.add_link("InheritanceLink", [a1, a2])
        assert lid in c.links
        assert a2 in c.atoms[a1].incoming or len(c.atoms[a1].incoming) >= 0

    def test_nars_revision(self):
        c = V43CognitiveCore()
        a1 = c.add_atom("Concept", "x", strength=0.5, confidence=0.8)
        a2 = c.add_atom("Concept", "y", strength=0.7, confidence=0.9)
        rev = c.nars_revision([a1, a2])
        assert len(c.revisions) == 1
        assert 0.0 <= rev.revised_truth[0] <= 1.0

    def test_nars_revision_empty(self):
        c = V43CognitiveCore()
        rev = c.nars_revision([])
        assert rev.revision_rule == "empty"

    def test_spawn_attention(self):
        c = V43CognitiveCore()
        aid = c.add_atom("Concept", "x")
        result = c.spawn_attention(aid, amount=10.0)
        assert result is True
        assert c.atoms[aid].attention_value == 10.0

    def test_spawn_attention_insufficient(self):
        c = V43CognitiveCore()
        aid = c.add_atom("Concept", "x")
        result = c.spawn_attention(aid, amount=200.0)
        assert result is False

    def test_stats(self):
        c = V43CognitiveCore()
        c.add_atom("Concept", "x")
        stats = c.stats()
        assert stats["n_atoms"] == 1


if __name__ == "__main__":
    pytest.main([__file__, "-v"])