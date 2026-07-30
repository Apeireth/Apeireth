"""R11 Requisite Variety Controller 真生产回归测试.

主 17:43 实事求是: 9+ 真测试覆盖关键不变量:
  1. Shannon 熵真测 (数学正确性)
  2. 满足 vs 不满足 (Ashby 律)
  3. Missing states 检测 (channel 真实情况)
  4. Amplification 建议 (Conant-Ashby)
  5. 与 V47 flat check 的差异 (R11 更严)
  6. 接入 V47 双层报告
  7. 真场景: Central AI 4 类 query × 6 类 response
"""
from __future__ import annotations

import math
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent.parent))

import pytest

from apeireth.r11_requisite_variety import (
    R11_VERSION,
    AttachedReport,
    ChannelSample,
    Disturbance,
    RequisiteVarietyController,
    RequisiteVarietyReport,
    Response,
)
from apeireth.v47_self_organizing_core import V47SelfOrganizingCore


# --------------------- Shannon 熵真测 ---------------------

class TestShannonEntropy:
    def test_uniform_distribution(self):
        """均匀 4 分布: H = log2(4) = 2.0 bits."""
        rvc = RequisiteVarietyController()
        for s in ["A", "B", "C", "D"]:
            for _ in range(25):
                rvc.observe_disturbance("env", s)
        r = rvc.measure()
        assert abs(r.H_D - 2.0) < 1e-9, f"H(D) expected 2.0, got {r.H_D}"

    def test_peaked_distribution(self):
        """极端偏分布: 99% 一个状态 -> H ≈ 0.08 bits."""
        rvc = RequisiteVarietyController()
        for _ in range(99):
            rvc.observe_disturbance("env", "A")
        for _ in range(1):
            rvc.observe_disturbance("env", "B")
        r = rvc.measure()
        # H = -[0.99 * log2(0.99) + 0.01 * log2(0.01)]
        expected = -(0.99 * math.log2(0.99) + 0.01 * math.log2(0.01))
        assert abs(r.H_D - expected) < 1e-9

    def test_two_state_uniform(self):
        """2 状态均匀 (50-50): H = 1.0 bits."""
        rvc = RequisiteVarietyController()
        for _ in range(50):
            rvc.observe_disturbance("env", "A")
        for _ in range(50):
            rvc.observe_disturbance("env", "B")
        r = rvc.measure()
        assert abs(r.H_D - 1.0) < 1e-9


# --------------------- Ashby 律真测 (满足/不满足) ---------------------

class TestAshbyLaw:
    def test_requisite_satisfied_more_responses(self):
        """H_R > H_D: 系统多样性足够, 不 deficit."""
        rvc = RequisiteVarietyController()
        # 2 个 env 状态, 各 50 次 (均匀)
        for _ in range(50):
            rvc.observe_disturbance("env", "A")
        for _ in range(50):
            rvc.observe_disturbance("env", "B")
        # 4 个 sys actions (log2(4)=2 bits), 各 25 次
        for s in ["x", "y", "z", "w"]:
            for _ in range(25):
                rvc.record_response("sys", s)
        # channel: 两个 env state 都响应
        rvc.sample_channel("A", "x")
        rvc.sample_channel("B", "y")
        r = rvc.measure()
        assert r.H_D == 1.0
        assert r.H_R == 2.0
        assert r.H_R > r.H_D
        assert r.deficit is False
        assert r.is_requisite is True

    def test_requisite_unsatisfied_fewer_responses(self):
        """H_R < H_D: 系统多样性不够, deficit."""
        rvc = RequisiteVarietyController()
        # 4 个 env states (log2(4)=2 bits)
        for s in ["A", "B", "C", "D"]:
            for _ in range(25):
                rvc.observe_disturbance("env", s)
        # 仅 2 个 sys actions (log2(2)=1 bits)
        for s in ["x", "y"]:
            for _ in range(50):
                rvc.record_response("sys", s)
        # channel: 全部 4 状态都响应
        for ds in ["A", "B", "C", "D"]:
            rvc.sample_channel(ds, "x")
        r = rvc.measure()
        assert r.H_D == 2.0
        assert r.H_R == 1.0
        assert r.H_R < r.H_D
        assert r.deficit is True
        assert r.is_requisite is False
        # amplification 必须建议 diversify
        assert any("diversify_responses" in s for s in r.amplification_suggestions)


# --------------------- Missing States 检测 ---------------------

class TestMissingStates:
    def test_missing_state_detected(self):
        """env state 有但 channel 无样本 -> missing."""
        rvc = RequisiteVarietyController()
        for s in ["A", "B", "C"]:
            for _ in range(10):
                rvc.observe_disturbance("env", s)
        for s in ["x", "y"]:
            for _ in range(15):
                rvc.record_response("sys", s)
        # 只响应 A, B, 不响应 C
        rvc.sample_channel("A", "x")
        rvc.sample_channel("B", "y")
        r = rvc.measure()
        assert "C" in r.missing_states
        assert r.deficit is True

    def test_no_missing_when_all_responded(self):
        """所有 env state 都有 channel sample -> 无 missing."""
        rvc = RequisiteVarietyController()
        for s in ["A", "B", "C"]:
            for _ in range(10):
                rvc.observe_disturbance("env", s)
        for s in ["x", "y", "z"]:
            for _ in range(10):
                rvc.record_response("sys", s)
        for ds, ra in [("A", "x"), ("B", "y"), ("C", "z")]:
            rvc.sample_channel(ds, ra)
        r = rvc.measure()
        assert r.missing_states == set()
        assert r.deficit is False


# --------------------- Channel Capacity (Mutual Information) ---------------------

class TestChannelCapacity:
    def test_perfect_channel(self):
        """完美通道: 1-1 对应, I(D;R) = H(D)."""
        rvc = RequisiteVarietyController()
        # 2 env states 均匀
        for _ in range(50):
            rvc.observe_disturbance("env", "A")
        for _ in range(50):
            rvc.observe_disturbance("env", "B")
        # 2 sys actions 均匀
        for _ in range(50):
            rvc.record_response("sys", "X")
        for _ in range(50):
            rvc.record_response("sys", "Y")
        # 完美映射
        for _ in range(50):
            rvc.sample_channel("A", "X")
        for _ in range(50):
            rvc.sample_channel("B", "Y")
        r = rvc.measure()
        # 完美通道: I(D;R) = H(D) = 1.0 bits
        assert abs(r.channel_capacity - 1.0) < 1e-6
        assert r.deficit is False

    def test_zero_channel_capacity_no_samples(self):
        """无 channel sample: I(D;R) = 0, 必有 deficit."""
        rvc = RequisiteVarietyController()
        for _ in range(10):
            rvc.observe_disturbance("env", "A")
        for _ in range(10):
            rvc.record_response("sys", "X")
        r = rvc.measure()
        assert r.channel_capacity == 0.0
        assert r.deficit is True

    def test_bottleneck_channel(self):
        """通道瓶颈: D 高但 channel_capacity 低."""
        rvc = RequisiteVarietyController()
        # 4 个 env states
        for s in ["A", "B", "C", "D"]:
            for _ in range(25):
                rvc.observe_disturbance("env", s)
        # 4 个 sys actions
        for s in ["w", "x", "y", "z"]:
            for _ in range(25):
                rvc.record_response("sys", s)
        # 但 channel 全部映射到 "w" (瓶颈 — 无法区分 env states)
        for ds in ["A", "B", "C", "D"]:
            rvc.sample_channel(ds, "w")
        r = rvc.measure()
        # H(D) = 2 bits, H(R) = 2 bits (但响应总是 w = H(R) ≈ 0)
        # 实际上: 4 个 actions 都被记录但 sample channel 总是 w
        # channel_capacity 应该接近 0 (因为 response 分布不被 env 状态影响)
        assert r.channel_capacity < 0.5, (
            f"bottleneck expected low capacity, got {r.channel_capacity}"
        )


# --------------------- Amplification Suggestions ---------------------

class TestAmplification:
    def test_three_suggestion_types(self):
        """缺 variety + 缺 missing + 缺 channel = 3 类建议."""
        rvc = RequisiteVarietyController()
        # 5 env states
        for s in ["A", "B", "C", "D", "E"]:
            for _ in range(20):
                rvc.observe_disturbance("env", s)
        # 1 sys action (严重不足)
        for _ in range(100):
            rvc.record_response("sys", "only_x")
        # 只响应 1 个 state
        rvc.sample_channel("A", "only_x")
        r = rvc.measure()
        assert r.deficit is True
        # 至少 3 类建议: add_response_for_state × N, diversify_responses × 1
        has_missing_suggestion = any(
            "add_response_for_state" in s for s in r.amplification_suggestions
        )
        has_diversify_suggestion = any(
            "diversify_responses" in s for s in r.amplification_suggestions
        )
        assert has_missing_suggestion
        assert has_diversify_suggestion
        # missing states: 4 个 (B, C, D, E)
        assert len(r.missing_states) == 4


# --------------------- 接入 V47 (composition) ---------------------

class TestV47Attachment:
    def test_v47_and_r11_agree_satisfied(self):
        """V47 flat pass + R11 info-theoretic pass."""
        core = V47SelfOrganizingCore()
        rvc = RequisiteVarietyController()
        # 2 env states
        for _ in range(10):
            rvc.observe_disturbance("env", "A")
        for _ in range(10):
            rvc.observe_disturbance("env", "B")
        # 3 sys actions (count > env)
        for s in ["x", "y", "z"]:
            for _ in range(10):
                rvc.record_response("sys", s)
        # 全部 channel sample
        rvc.sample_channel("A", "x")
        rvc.sample_channel("B", "y")
        report = rvc.attach_to_v47(core, env_variety=2)
        assert isinstance(report, AttachedReport)
        assert report.v47_satisfied is True
        assert report.r11.is_requisite is True

    def test_r11_stricter_than_v47(self):
        """V47 flat pass 但 R11 检出 deficit (R11 更严 — 主 17:43 实事求是)."""
        core = V47SelfOrganizingCore()
        rvc = RequisiteVarietyController()
        # 10 env states, 10 sys actions (count 满足 V47 flat)
        for i in range(10):
            rvc.observe_disturbance("env", f"state_{i}")
        for i in range(10):
            rvc.record_response("sys", f"action_{i}")
        # 但 channel 只响应 1 个 state (其他 9 个 missing)
        rvc.sample_channel("state_0", "action_0")
        report = rvc.attach_to_v47(core, env_variety=10)
        # V47 flat: sys_variety=10 unique actions >= env_variety=10, satisfied=True
        assert report.v47_satisfied is True
        # R11 info-theoretic: 9 missing states -> deficit
        assert report.r11.deficit is True
        assert len(report.r11.missing_states) == 9


# --------------------- 真场景: Central AI 4×6 ---------------------

class TestRealScenario:
    def test_central_ai_4x6(self):
        """真场景: Central AI 接收 4 类 user intent, 用 6 类 response."""
        rvc = RequisiteVarietyController(name="central_ai_rvc")
        env_states = ["bug_report", "feature_request", "how_to", "philosophical"]
        sys_actions = ["acknowledge", "investigate", "code_fix", "explain", "defer", "escalate"]
        # 100 次交互 (主 19:33 真生产规模)
        mapping = {
            "bug_report": "investigate",
            "feature_request": "acknowledge",
            "how_to": "explain",
            "philosophical": "defer",
        }
        for i in range(100):
            env = env_states[i % 4]
            rvc.observe_disturbance("user_intent", env)
            action = mapping[env]
            rvc.record_response("central_ai", action)
            rvc.sample_channel(env, action)
        r = rvc.measure()
        # H(D) = log2(4) = 2 bits
        assert abs(r.H_D - 2.0) < 1e-9
        # 4 个不同 actions 均匀使用 -> H(R) = 2 bits
        assert abs(r.H_R - 2.0) < 1e-9
        # 完美通道: I(D;R) = 2 bits
        assert abs(r.channel_capacity - 2.0) < 1e-6
        # 全部状态都响应
        assert len(r.missing_states) == 0
        # Ashby ratio = 1.0
        assert abs(r.ratio - 1.0) < 1e-9
        assert r.deficit is False
        assert r.is_requisite is True

    def test_central_ai_unsatisfied_when_channel_breaks(self):
        """真场景退化: 中央 AI channel 部分断 (主 17:58 不假装)."""
        rvc = RequisiteVarietyController(name="central_ai_partial")
        # 只响应 bug_report + how_to, 不响应 feature_request + philosophical
        rvc.observe_disturbance("user_intent", "bug_report")
        rvc.observe_disturbance("user_intent", "feature_request")
        rvc.observe_disturbance("user_intent", "how_to")
        rvc.observe_disturbance("user_intent", "philosophical")
        rvc.record_response("central_ai", "investigate")
        rvc.record_response("central_ai", "explain")
        # channel 只覆盖 2 个状态
        rvc.sample_channel("bug_report", "investigate")
        rvc.sample_channel("how_to", "explain")
        r = rvc.measure()
        assert "feature_request" in r.missing_states
        assert "philosophical" in r.missing_states
        assert r.deficit is True
        assert len(r.amplification_suggestions) >= 2


# --------------------- Stats (dashboard) ---------------------

class TestStats:
    def test_stats_structure(self):
        rvc = RequisiteVarietyController(name="test_rvc")
        for _ in range(5):
            rvc.observe_disturbance("env", "A")
        for _ in range(5):
            rvc.observe_disturbance("env", "B")
        for s in ["x", "y"]:
            for _ in range(5):
                rvc.record_response("sys", s)
        rvc.sample_channel("A", "x")
        rvc.sample_channel("B", "y")
        s = rvc.stats()
        assert s["version"] == R11_VERSION
        assert s["controller_name"] == "test_rvc"
        assert s["n_disturbances"] == 10
        assert s["n_responses"] == 10
        assert s["n_channel_samples"] == 2
        assert "H_D_bits" in s
        assert "H_R_bits" in s
        assert "channel_capacity_bits" in s
        assert "deficit" in s
        assert "is_requisite" in s
        assert "n_missing_states" in s
        assert "amplification_suggestions" in s
        assert "philosophy" in s


# --------------------- 入口 ---------------------

if __name__ == "__main__":
    pytest.main([__file__, "-v"])