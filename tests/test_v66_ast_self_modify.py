"""v66_ast_self_modify.py 真生产回归测试."""
from __future__ import annotations

import sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).parent.parent))
import pytest
from apeireth.v66_ast_self_modify import (
    V66_VERSION, ASTNode, SelfModification, V66ASTSelfModify,
)


class TestV66:
    def test_init(self):
        ast_core = V66ASTSelfModify()
        assert ast_core.nodes == {}

    def test_add_node(self):
        ast_core = V66ASTSelfModify()
        nid = ast_core.add_node("Function", "main")
        assert nid in ast_core.nodes

    def test_add_node_with_parent(self):
        ast_core = V66ASTSelfModify()
        p = ast_core.add_node("Module", "root")
        c = ast_core.add_node("Function", "child", parent_id=p)
        assert c in ast_core.nodes[p].children

    def test_self_modify(self):
        ast_core = V66ASTSelfModify()
        nid = ast_core.add_node("Function", "f")
        mid = ast_core.self_modify(nid, "old", "new")
        assert mid in [m.modification_id for m in ast_core.modifications]

    def test_safe_modify(self):
        ast_core = V66ASTSelfModify()
        nid = ast_core.add_node("Function", "f")
        mid = ast_core.safe_modify(nid, "old", "new")
        assert ast_core.n_safe_modifications() == 1

    def test_stats(self):
        ast_core = V66ASTSelfModify()
        ast_core.add_node("M", "root")
        stats = ast_core.stats()
        assert stats["n_nodes"] == 1


if __name__ == "__main__":
    pytest.main([__file__, "-v"])