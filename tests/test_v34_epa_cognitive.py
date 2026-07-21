"""v34_epa_cognitive.py 真生产回归测试."""
from __future__ import annotations

import sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).parent.parent))
import pytest
from apeireth.v34_epa_cognitive import (
    V34_VERSION, EPAPhase, EPAEvent, EPAPerception, EPAAction,
    V34EPACognitiveLoop,
)


class TestV34:
    def test_init(self):
        loop = V34EPACognitiveLoop()
        assert loop.events == []
        assert loop.cycles == 0

    def test_receive_event(self):
        loop = V34EPACognitiveLoop()
        e = loop.receive_event("test", "payload")
        assert e.event_id in [ev.event_id for ev in loop.events]

    def test_perceive(self):
        loop = V34EPACognitiveLoop()
        e = loop.receive_event("test", "hello")
        p = loop.perceive(e)
        assert p.event_id == e.event_id

    def test_act(self):
        loop = V34EPACognitiveLoop()
        e = loop.receive_event("test", "hello")
        p = loop.perceive(e)
        a = loop.act(p)
        assert a.action_type == "respond"

    def test_run_cycle(self):
        loop = V34EPACognitiveLoop()
        r = loop.run_cycle("auto", "test")
        assert "event" in r
        assert "perception" in r

    def test_run_n_cycles(self):
        loop = V34EPACognitiveLoop()
        results = loop.run_n_cycles(3)
        assert len(results) == 3
        assert loop.cycles == 3

    def test_custom_fns(self):
        loop = V34EPACognitiveLoop()
        r = loop.run_cycle(
            "custom", "data",
            understanding_fn=lambda x: f"custom-understanding:{x}",
            action_fn=lambda p: {"custom_action": True},
        )
        assert "custom-understanding" in r["perception"]
        assert r["action_type"] == "respond"

    def test_stats(self):
        loop = V34EPACognitiveLoop()
        loop.run_n_cycles(2)
        stats = loop.stats()
        assert stats["v3_philosophy_guard"] == "PASS"
        assert stats["n_cycles"] == 2


if __name__ == "__main__":
    pytest.main([__file__, "-v"])