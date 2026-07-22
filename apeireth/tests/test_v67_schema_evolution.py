"""v67_schema_evolution.py 真生产回归测试."""
from __future__ import annotations

import sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).parent.parent))
import pytest
from apeireth.v67_schema_evolution import (
    V67_VERSION, Schema, SchemaEvolution, V67SchemaEvolution,
)


class TestV67:
    def test_init(self):
        se = V67SchemaEvolution()
        assert se.schemas == {}

    def test_create_schema(self):
        se = V67SchemaEvolution()
        sid = se.create_schema("test", {"a": "int"})
        assert sid in se.schemas

    def test_evolve_schema_add(self):
        se = V67SchemaEvolution()
        sid = se.create_schema("test", {"a": "int"})
        evo_id = se.evolve_schema(sid, added_fields={"b": "str"})
        assert evo_id != ""
        assert se.schemas[sid].version == 2
        assert se.schemas[sid].is_backward_compatible is True

    def test_evolve_schema_remove(self):
        se = V67SchemaEvolution()
        sid = se.create_schema("test", {"a": "int", "b": "str"})
        se.evolve_schema(sid, removed_fields=["b"])
        assert se.schemas[sid].is_backward_compatible is False

    def test_n_evolutions(self):
        se = V67SchemaEvolution()
        sid = se.create_schema("test", {"a": "int"})
        se.evolve_schema(sid, added_fields={"b": "str"})
        se.evolve_schema(sid, added_fields={"c": "str"})
        assert se.n_evolutions() == 2

    def test_stats(self):
        se = V67SchemaEvolution()
        se.create_schema("test", {})
        stats = se.stats()
        assert stats["n_schemas"] == 1


if __name__ == '__main__':
    pytest.main([__file__, '-v'])