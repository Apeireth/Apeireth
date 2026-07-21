"""v38_change_manifest.py 真生产回归测试."""
from __future__ import annotations

import sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).parent.parent))
import pytest
from apeireth.v38_change_manifest import (
    V38_VERSION, ChangeManifest, MainLoopIteration,
    create_change_manifest, main_loop_step, V38ChangeManifestLoop,
)


class TestV38:
    def test_create_manifest(self):
        m = create_change_manifest(iteration=1, trigger="test")
        assert m.iteration == 1
        assert m.trigger == "test"

    def test_main_loop_step(self):
        result = main_loop_step(1, lambda: 0.5)
        assert result.verdict == "keep"  # delta=0.5 from 0

    def test_main_loop_step_with_prev(self):
        prev = MainLoopIteration(iteration_id="x", iteration=0, phase="", hqb_total=0.5)
        result = main_loop_step(1, lambda: 1.0, prev)
        assert result.verdict == "keep"

    def test_main_loop_step_revert(self):
        prev = MainLoopIteration(iteration_id="x", iteration=0, phase="", hqb_total=0.5)
        result = main_loop_step(1, lambda: -0.1, prev)
        assert result.verdict == "revert"

    def test_loop_init(self):
        loop = V38ChangeManifestLoop()
        assert loop.iterations == []

    def test_loop_run(self):
        loop = V38ChangeManifestLoop()
        iterations = loop.run_main_loop(3, lambda: 0.5)
        assert len(iterations) == 3

    def test_loop_stats(self):
        loop = V38ChangeManifestLoop()
        loop.run_main_loop(3, lambda: 0.5)
        stats = loop.stats()
        assert stats["n_iterations"] == 3


if __name__ == "__main__":
    pytest.main([__file__, "-v"])