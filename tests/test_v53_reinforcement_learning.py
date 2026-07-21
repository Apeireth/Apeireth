"""v53_reinforcement_learning.py 真生产回归测试."""
from __future__ import annotations

import sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).parent.parent))
import pytest
from apeireth.v53_reinforcement_learning import (
    V53_VERSION, PPOClip, compute_ppo_clip,
    Transition, RLTrajectory, V53ReinforcementLearning,
)


class TestV53Helpers:
    def test_ppo_clip_basic(self):
        clip = compute_ppo_clip(old_log_prob=-1.0, new_log_prob=-0.5, advantage=1.0)
        assert clip.ratio > 0
        assert clip.loss < 0  # negative loss for positive advantage


class TestV53:
    def test_init(self):
        rl = V53ReinforcementLearning()
        assert rl.n_trajectories() == 0

    def test_add_transition(self):
        rl = V53ReinforcementLearning()
        rl.add_transition("s", "a", 1.0, "s2")
        assert rl.n_transitions() == 1

    def test_ppo_clip_method(self):
        rl = V53ReinforcementLearning()
        clip_id = rl.compute_ppo_clip(-1.0, -0.9, 0.5)
        assert clip_id.startswith("clip_")

    def test_total_reward(self):
        rl = V53ReinforcementLearning()
        rl.add_transition("s", "a", 1.0, "s2")
        rl.add_transition("s2", "b", 2.0, "s3", done=True)
        assert rl.total_reward() == 3.0

    def test_stats(self):
        rl = V53ReinforcementLearning()
        rl.add_transition("s", "a", 1.0, "s2")
        stats = rl.stats()
        assert stats["n_transitions"] == 1


if __name__ == "__main__":
    pytest.main([__file__, "-v"])