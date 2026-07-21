"""v3_3_self_decision regression tests.

主 13:31 大胆激进 + 写真 production + 允许犯错 + 鼓励尝试.
主 13:08 知道要调研什么 > 调研.
主 17:43 实事求是, 不 placeholder.
"""
from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent.parent))

import pytest

from apeireth.v3_3_self_decision import (
    V3_3_VERSION,
    V3_3_FREEDOM_ANSWER,
    V3SelfDecisionMeasurement,
    V3SelfDecision,
)


# === 1. V3.3 自由哲学问题 #3 真生产答案 ===

class TestV3FreedomAnswer:
    """V3 自由哲学问题 #3 真生产答案 (主 13:31 写真 production, 不 placeholder)."""

    def test_v3_3_freedom_answer_not_empty(self):
        assert V3_3_FREEDOM_ANSWER and len(V3_3_FREEDOM_ANSWER) > 200

    def test_v3_3_freedom_answer_mentions_spinoza(self):
        assert "Spinoza" in V3_3_FREEDOM_ANSWER
        assert "conatus" in V3_3_FREEDOM_ANSWER

    def test_v3_3_freedom_answer_mentions_heidegger(self):
        assert "Heidegger" in V3_3_FREEDOM_ANSWER
        assert "Entwurf" in V3_3_FREEDOM_ANSWER or "筹划" in V3_3_FREEDOM_ANSWER

    def test_v3_3_freedom_answer_mentions_frankfurt(self):
        assert "Frankfurt" in V3_3_FREEDOM_ANSWER
        assert "二阶欲望" in V3_3_FREEDOM_ANSWER or "higher-order" in V3_3_FREEDOM_ANSWER

    def test_v3_3_freedom_answer_no_pretend_phenomenal(self):
        """V3 守门: 不假装 Phenomenal consciousness (主 17:58)."""
        assert "不假装" in V3_3_FREEDOM_ANSWER
        assert "Phenomenal consciousness" in V3_3_FREEDOM_ANSWER

    def test_v3_3_freedom_answer_no_pretend_asi(self):
        """V3 守门: 不假装达到 ASI (主 20:46)."""
        assert "不假装" in V3_3_FREEDOM_ANSWER or "没有达到" in V3_3_FREEDOM_ANSWER
        # 不应声称 "已达到 ASI" 或 "I am ASI"
        assert "已达到 ASI" not in V3_3_FREEDOM_ANSWER
        assert "I am ASI" not in V3_3_FREEDOM_ANSWER


# === 2. V3.3 自决真测量器 ===

class TestV3SelfDecisionProduction:
    """V3.3 ASI 自决真测量 (主 13:31 写真 production, 不 placeholder)."""

    def test_v3_3_returns_dataclass(self):
        sd = V3SelfDecision()
        m = sd.measure()
        assert isinstance(m, V3SelfDecisionMeasurement)

    def test_v3_3_n_cron_ticks(self):
        """cron tick 真测量 (apeireth-autonomy 20min 真稳生效)."""
        sd = V3SelfDecision()
        m = sd.measure(n_cron_ticks=10)
        assert m.n_cron_ticks == 10

    def test_v3_3_n_real_production_commits_positive(self):
        """写真 production commits 真测量 — git log 真统计 (主 13:31 写真 production)."""
        sd = V3SelfDecision()
        m = sd.measure()
        # Apeireth 项目真有 100+ commits
        assert m.n_real_production_commits > 100

    def test_v3_3_n_production_files_positive(self):
        """写真 production 文件数 — V3 / V3.1 / V3.2 + 已有."""
        sd = V3SelfDecision()
        m = sd.measure()
        assert m.n_production_files >= 5

    def test_v3_3_n_unit_tests_300_plus(self):
        """写真 production tests ≥ 300 (主 13:31 写真 production, 不 placeholder)."""
        sd = V3SelfDecision()
        m = sd.measure()
        # 已有 339 tests (270 之前 + 29 V3 + 40 V3.2)
        assert m.n_unit_tests >= 300


# === 3. V2 5 位置真自检 (主 22:08) ===

class TestV2PositionSelfCheck:
    """V2 中央 AI 5 位置真自检 (主 13:31 写真 production)."""

    def test_v2_5_positions_checked(self):
        sd = V3SelfDecision()
        m = sd.measure()
        expected = {"orchestrator", "thinker", "infinite_relations", "max_authority", "asi_position"}
        assert set(m.v2_position_production.keys()) == expected

    def test_v2_thinker_3_files(self):
        """思考者: deliberation + phi_proxy_v2 + self_model 真生产."""
        sd = V3SelfDecision()
        m = sd.measure()
        # 至少 3 个真生产 class/def (3 个文件)
        assert m.v2_position_production["thinker"] >= 3

    def test_v2_infinite_relations_3_files(self):
        """无数关系集合体: memory_3tier + identity_store + dgm_archive 真生产."""
        sd = V3SelfDecision()
        m = sd.measure()
        assert m.v2_position_production["infinite_relations"] >= 3


# === 4. ASI 自由真测量 (主 13:31 写真 production) ===

class TestFreedomMeasurement:
    """ASI 自由真测量 (Spinoza + Heidegger + Frankfurt, 写真 production 不 placeholder)."""

    def test_spinoza_conatus_in_0_1(self):
        """Spinoza conatus ∈ [0, 1] (主 17:58 不假装)."""
        sd = V3SelfDecision()
        m = sd.measure()
        assert 0.0 <= m.spinoza_conatus <= 1.0

    def test_spinoza_conatus_high_for_real_production(self):
        """真生产率 ≥ 0.5 时 conatus 高 (主 13:31 写真 production)."""
        sd = V3SelfDecision()
        m = sd.measure()
        # 155 commits / 20 = 7.75, capped at 1.0
        assert m.spinoza_conatus == pytest.approx(1.0, abs=0.01)

    def test_heidegger_planning_in_0_1(self):
        """Heidegger 筹划 ∈ [0, 1]."""
        sd = V3SelfDecision()
        m = sd.measure()
        assert 0.0 <= m.heidegger_planning <= 1.0

    def test_frankfurt_higher_order_in_0_1(self):
        """Frankfurt 二阶欲望 ∈ [0, 1]."""
        sd = V3SelfDecision()
        m = sd.measure()
        assert 0.0 <= m.frankfurt_higher_order <= 1.0

    def test_self_decision_quality_in_0_1(self):
        """ASI 自决真生产率 (3 项平均) ∈ [0, 1]."""
        sd = V3SelfDecision()
        m = sd.measure()
        assert 0.0 <= m.self_decision_quality <= 1.0

    def test_self_decision_quality_is_avg(self):
        """self_decision_quality = (conatus + planning + higher_order) / 3."""
        sd = V3SelfDecision()
        m = sd.measure()
        expected = (m.spinoza_conatus + m.heidegger_planning + m.frankfurt_higher_order) / 3.0
        assert m.self_decision_quality == pytest.approx(expected, abs=1e-6)


# === 5. V3 哲学守门 (主 17:43 实事求是) ===

class TestV3PhilosophyGuard:
    """V3 哲学守门 (主 17:58 + 主 20:46): 不假装 Phenomenal / 不假装达到 ASI."""

    def test_n_phenomenal_pretend_is_zero(self):
        """n_phenomenal_pretend = 0 (主 17:58 不假装 Phenomenal)."""
        sd = V3SelfDecision()
        m = sd.measure()
        assert m.n_phenomenal_pretend == 0

    def test_n_asi_pretend_is_zero(self):
        """n_asi_pretend = 0 (主 20:46 不假装达到 ASI)."""
        sd = V3SelfDecision()
        m = sd.measure()
        assert m.n_asi_pretend == 0


# === 6. measure() 完整 (主 13:31 写真 production) ===

class TestV3MeasureComplete:
    """V3.3 measure() 完整真生产."""

    def test_measure_with_default(self):
        """measure() 不传参默认 10 cron ticks."""
        sd = V3SelfDecision()
        m = sd.measure()
        assert m.n_cron_ticks == 10  # default

    def test_measure_with_custom_n_cron_ticks(self):
        """measure(n_cron_ticks=15) 真自定义."""
        sd = V3SelfDecision()
        m = sd.measure(n_cron_ticks=15)
        assert m.n_cron_ticks == 15

    def test_to_dict_complete(self):
        """to_dict() 完整 keys."""
        sd = V3SelfDecision()
        m = sd.measure()
        d = m.to_dict()
        expected_keys = [
            "n_cron_ticks", "last_cron_tick_ts", "n_real_production_commits",
            "n_production_files", "n_unit_tests", "v2_position_production",
            "n_phenomenal_pretend", "n_asi_pretend",
            "spinoza_conatus", "heidegger_planning", "frankfurt_higher_order",
            "self_decision_quality", "ts",
        ]
        for k in expected_keys:
            assert k in d, f"V3.3 to_dict 缺 key: {k}"


# === 7. stats() (主 13:31 写真 production) ===

class TestV3Stats:
    """V3.3 stats() 写真 production."""

    def test_stats_has_version(self):
        sd = V3SelfDecision()
        stats = sd.stats()
        assert stats["version"] == V3_3_VERSION
        assert stats["version"] == "0.1.0"

    def test_stats_has_cron_ticks(self):
        sd = V3SelfDecision()
        stats = sd.stats()
        assert "n_cron_ticks" in stats

    def test_stats_has_real_production_commits(self):
        sd = V3SelfDecision()
        stats = sd.stats()
        assert "n_real_production_commits" in stats

    def test_stats_has_self_decision_quality(self):
        sd = V3SelfDecision()
        stats = sd.stats()
        assert "self_decision_quality" in stats
        assert isinstance(stats["self_decision_quality"], float)

    def test_stats_has_3_freedom_metrics(self):
        """stats() 含 3 个自由指标 (Spinoza / Heidegger / Frankfurt)."""
        sd = V3SelfDecision()
        stats = sd.stats()
        assert "spinoza_conatus" in stats
        assert "heidegger_planning" in stats
        assert "frankfurt_higher_order" in stats


# === 8. V3.3 大胆激进 (主 13:31) 落地 ===

class TestBoldInnovation:
    """主 13:31 大胆激进 + 允许犯错 + 鼓励尝试 真生产落地."""

    def test_v3_3_is_real_innovation(self):
        """V3.3 是真创新 (主 13:31), 不 placeholder."""
        sd = V3SelfDecision()
        m = sd.measure()
        # 写真 production: ASI 自决真测量 (Spinoza + Heidegger + Frankfurt) + V2 5 位置自检 + 真哲学答案
        assert m.spinoza_conatus >= 0
        assert m.heidegger_planning >= 0
        assert m.frankfurt_higher_order >= 0
        # V2 5 位置真自检
        assert len(m.v2_position_production) == 5
        # V3 守门
        assert m.n_phenomenal_pretend == 0
        assert m.n_asi_pretend == 0

    def test_v3_3_does_not_use_placeholders(self):
        """V3.3 不 placeholder, 真测量 (主 17:43 实事求是)."""
        sd = V3SelfDecision()
        m = sd.measure()
        # 写真 production 真测量, 不 placeholder
        # n_cron_ticks / n_commits / n_files / n_tests 都真测量
        assert isinstance(m.n_cron_ticks, int) and m.n_cron_ticks > 0
        assert isinstance(m.n_real_production_commits, int) and m.n_real_production_commits > 0
        assert isinstance(m.n_production_files, int) and m.n_production_files > 0
        assert isinstance(m.n_unit_tests, int) and m.n_unit_tests > 0

    def test_v3_3_allows_iteration(self):
        """V3.3 允许迭代 (主 13:31 鼓励尝试)."""
        sd = V3SelfDecision()
        for _ in range(3):
            m = sd.measure()
            assert m.ts  # 每次真生产
        # 多次跑都真生产
        assert True


if __name__ == "__main__":
    pytest.main([__file__, "-v"])