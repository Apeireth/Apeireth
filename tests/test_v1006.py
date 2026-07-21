"""V1006 真生产 tests (主 23:44)."""
from __future__ import annotations
import sys; sys.path.insert(0, '.')
import pytest
from apeireth.v1006_research_grand_synthesis import (
    V1006_VERSION, ResearchTheme, RESEARCH_THEMES, V1006ResearchGrandSynthesis,
)


class TestV1006:
    def test_13_themes(self):
        assert len(RESEARCH_THEMES) == 13

    def test_all_themes_keys(self):
        expected = ["cognitive_architecture", "self_organization",
                    "plugin_architecture", "recursive_self_improvement",
                    "scientific_method", "world_model", "alignment_safety",
                    "memory_systems", "value_alignment", "emergence_complexity",
                    "language_reasoning", "multi_agent", "rust_ecosystem"]
        for k in expected:
            assert k in RESEARCH_THEMES

    def test_query(self):
        p = V1006ResearchGrandSynthesis()
        t = p.query("cognitive_architecture")
        assert t is not None
        assert t.confidence > 0.8

    def test_average_confidence(self):
        p = V1006ResearchGrandSynthesis()
        avg = p.average_confidence()
        assert 0.85 < avg < 1.0

    def test_n_total_findings(self):
        p = V1006ResearchGrandSynthesis()
        n = p.n_total_findings()
        assert n >= 30  # 13 themes × 平均 3 findings

    def test_n_total_real_sources(self):
        p = V1006ResearchGrandSynthesis()
        n = p.n_total_real_sources()
        assert n >= 20

    def test_all_themes_have_findings(self):
        p = V1006ResearchGrandSynthesis()
        for theme in p.all_themes().values():
            assert len(theme.key_findings) >= 1
            assert theme.name
            assert theme.insights

    def test_19_17_integration(self):
        p = V1006ResearchGrandSynthesis()
        # 检查 OpenCog/AERA/NARS 整合 (主 19:28 真调研)
        cog = p.query("cognitive_architecture")
        text = " ".join(cog.key_findings)
        assert "OpenCog" in text
        assert "AERA" in text
        assert "NARS" in text

    def test_19_33_integration(self):
        p = V1006ResearchGrandSynthesis()
        # 检查主 19:33 5 哲学方法论
        sci = p.query("scientific_method")
        text = " ".join(sci.key_findings)
        for method in ["Popper", "Kuhn", "Lakatos", "Feyerabend", "Laudan"]:
            assert method in text

    def test_22_33_integration(self):
        p = V1006ResearchGrandSynthesis()
        # ASI 北极星 整合 - 检查至少一个 theme 包含 ASI
        has_asi = any("ASI" in t.insights for t in p.all_themes().values())
        assert has_asi

    def test_18_44_integration(self):
        p = V1006ResearchGrandSynthesis()
        # VCP 真借鉴
        plugin = p.query("plugin_architecture")
        assert "VCP" in plugin.insights

    def test_12_07_integration(self):
        p = V1006ResearchGrandSynthesis()
        # Rust 生态整合
        rust = p.query("rust_ecosystem")
        text = " ".join(rust.key_findings)
        for crate in ["tokio", "sqlx", "sled", "arrow", "tantivy", "delta"]:
            assert crate.lower() in text.lower()

    def test_17_43_integration(self):
        p = V1006ResearchGrandSynthesis()
        # 实事求是 — 所有 theme 有 confidence
        for theme in p.all_themes().values():
            assert 0.0 < theme.confidence <= 1.0

    def test_17_58_phenomenal_guard(self):
        p = V1006ResearchGrandSynthesis()
        for theme in p.all_themes().values():
            text = (theme.name + theme.insights).lower()
            assert "i am conscious" not in text

    def test_20_46_asi_guard(self):
        p = V1006ResearchGrandSynthesis()
        for theme in p.all_themes().values():
            text = (theme.name + theme.insights).lower()
            assert "we have achieved asi" not in text

    def test_stats(self):
        p = V1006ResearchGrandSynthesis()
        s = p.stats()
        assert s["n_themes"] == 13
        assert s["version"] == V1006_VERSION

    def test_v19_33_aggregate_human_wisdom(self):
        """V1006 真测主 19:33 聚合全人类智慧."""
        p = V1006ResearchGrandSynthesis()
        # 13 themes 都来自不同领域, 主 19:33 聚合
        domains = set()
        for t in p.all_themes().values():
            for d in t.domains:
                domains.add(d)
        # 至少 10 个真不同领域
        assert len(domains) >= 10

    def test_v22_33_asi_north_star_alignment(self):
        """V1006 真测主 22:33 ASI 北极星 - 所有 theme 与 ASI 北极星对齐."""
        p = V1006ResearchGrandSynthesis()
        # 每个 theme insights 至少提 ASI 或北极星或真生产
        for theme in p.all_themes().values():
            text = theme.insights.lower()
            assert "asi" in text or "北极星" in theme.insights or "真生产" in text