"""v72_code_generator.py 真生产回归测试."""
from __future__ import annotations

import sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).parent.parent))
import pytest
from apeireth.v72_code_generator import (
    V72_VERSION, GeneratedCode, V72CodeGenerator,
)


class TestV72:
    def test_init(self):
        cg = V72CodeGenerator()
        assert cg.generated == []

    def test_generate_python_function(self):
        cg = V72CodeGenerator()
        cid = cg.generate_python_function(
            name="hello",
            body="return 'hi'",
            args={"x": "str"},
        )
        assert cid in [g.code_id for g in cg.generated]

    def test_n_generated(self):
        cg = V72CodeGenerator()
        cg.generate_python_function(name="f1", body="pass")
        cg.generate_python_function(name="f2", body="pass")
        assert cg.n_generated() == 2

    def test_n_python(self):
        cg = V72CodeGenerator()
        cg.generate_python_function(name="f1", body="pass")
        assert cg.n_python() == 1

    def test_generated_code(self):
        cg = V72CodeGenerator()
        cid = cg.generate_python_function(
            name="test_fn",
            body="return 42",
            args={"x": "int"},
            return_type="int",
        )
        gen = [g for g in cg.generated if g.code_id == cid][0]
        assert "def test_fn" in gen.code
        assert "int" in gen.type_signature

    def test_stats(self):
        cg = V72CodeGenerator()
        cg.generate_python_function(name="f", body="pass")
        stats = cg.stats()
        assert stats["n_generated"] == 1


if __name__ == "__main__":
    pytest.main([__file__, "-v"])