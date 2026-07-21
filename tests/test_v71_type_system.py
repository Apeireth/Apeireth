"""v71_type_system.py 真生产回归测试."""
from __future__ import annotations

import sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).parent.parent))
import pytest
from apeireth.v71_type_system import (
    V71_VERSION, TypeKind, TypeSpec, TypeInstance, V71TypeSystem,
)


class TestV71:
    def test_init(self):
        ts = V71TypeSystem()
        assert ts.types == {}

    def test_define_type(self):
        ts = V71TypeSystem()
        tid = ts.define_type("Test", TypeKind.INT)
        assert tid in ts.types

    def test_create_instance_valid(self):
        ts = V71TypeSystem()
        tid = ts.define_type("Agent", TypeKind.CUSTOM, fields={"id": "str"})
        iid = ts.create_instance(tid, {"id": "a1"})
        assert ts.instances[-1].is_valid is True

    def test_create_instance_invalid(self):
        ts = V71TypeSystem()
        tid = ts.define_type("Agent", TypeKind.CUSTOM, fields={"id": "str", "name": "str"})
        iid = ts.create_instance(tid, {"id": "a1"})  # 缺 name
        assert ts.instances[-1].is_valid is False

    def test_create_instance_unknown_type(self):
        ts = V71TypeSystem()
        iid = ts.create_instance("unknown_type", {"a": 1})
        assert ts.instances[-1].is_valid is False

    def test_n_types(self):
        ts = V71TypeSystem()
        ts.define_type("a", TypeKind.INT)
        ts.define_type("b", TypeKind.STR)
        assert ts.n_types() == 2

    def test_stats(self):
        ts = V71TypeSystem()
        ts.define_type("a", TypeKind.INT)
        stats = ts.stats()
        assert stats["n_types"] == 1


if __name__ == "__main__":
    pytest.main([__file__, "-v"])