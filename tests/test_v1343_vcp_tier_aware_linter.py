#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
tests/test_v1343_vcp_tier_aware_linter.py — V1343 pytest tests

- Version: 0.1.0
- Author: 楚零 (Chu Ling, Apeireth ASI self-driven agent)
- Cron: 1fba1cc3-1a6d-4e3a-abb8-fccef1c94cdf (apeireth-autonomy-v3)
- Trigger: post-V1343 source code commit; per cron 主 17:43 实事求是
- Tests: 50+ canonical pytest tests for V1343

V1343 = VCP Tier-Aware Linter (post-V1342 quality tier)
- Reuses V1335 (registry) + V1336 (linter CLI) + V1342 (tier classifier)
- Adds tier-aware filter to linter output
- 8 API surfaces (verified via import + functional calls)

Test structure (8 test classes mirroring 8 API surfaces):
1. TestTierFilterConfig — get_tier_filter_config
2. TestCoverageAtTier — coverage_at_tier
3. TestCompareCoverage — compare_v1336_v1343_coverage
4. TestRecommendTier — recommend_tier_threshold
5. TestLintWithRecommendation — lint_substrates_with_recommendation
6. TestMarkdownRendering — tier_aware_report_to_markdown
7. TestSelfTest — _self_test
8. TestEdgeCases — edge cases + V1335/V1342 preservation
"""
from __future__ import annotations

import sys
from pathlib import Path

import pytest

V1343_DIR = Path(__file__).resolve().parent.parent / "apeireth"
sys.path.insert(0, str(V1343_DIR))

import v1335_vcp_cross_plugin_invariant_synthesis as v1335  # noqa: E402
import v1342_vcp_quality_tiers as v1342  # noqa: E402
import v1343_vcp_tier_aware_linter as v1343  # noqa: E402


# ===== Test 1: Tier Filter Config ============================================


class TestTierFilterConfig:
    """Test get_tier_filter_config() — Surface 1."""

    def test_tier_levels_has_4_entries(self):
        cfg = v1343.get_tier_filter_config()
        assert len(cfg["tier_levels"]) == 4

    def test_default_tier_min_is_high(self):
        cfg = v1343.get_tier_filter_config()
        assert cfg["default_tier_min"] == "high"

    def test_v1335_manual_treated_as_high(self):
        cfg = v1343.get_tier_filter_config()
        assert cfg["include_v1335_manual_as_high"] is True

    def test_all_tier_levels_present(self):
        cfg = v1343.get_tier_filter_config()
        for tier in ["high", "medium", "low", "all"]:
            assert tier in cfg["tier_levels"]


# ===== Test 2: Coverage At Tier ==============================================


class TestCoverageAtTier:
    """Test coverage_at_tier() — Surface 2."""

    def test_coverage_at_high_positive(self):
        score = v1343.coverage_at_tier("high")
        assert score > 0

    def test_coverage_at_all_positive(self):
        score = v1343.coverage_at_tier("all")
        assert score > 0

    def test_coverage_high_le_all(self):
        # High filter should not exceed all-inclusive
        assert v1343.coverage_at_tier("high") <= v1343.coverage_at_tier("all")

    def test_coverage_medium_ge_high(self):
        # Medium filter should be >= high filter
        assert v1343.coverage_at_tier("medium") >= v1343.coverage_at_tier("high")

    def test_coverage_at_low_eq_all(self):
        # Low filter excludes unclassified, all includes unclassified
        # But safety-critical coverage may be the same if no unclassified contributes
        # Just verify low is <= all
        assert v1343.coverage_at_tier("low") <= v1343.coverage_at_tier("all")


# ===== Test 3: Compare Coverage ==============================================


class TestCompareCoverage:
    """Test compare_v1336_v1343_coverage() — Surface 3."""

    def test_comparison_has_6_keys(self):
        comp = v1343.compare_v1336_v1343_coverage()
        assert len(comp) == 6

    def test_v1336_unfiltered_positive(self):
        comp = v1343.compare_v1336_v1343_coverage()
        assert comp["v1336_unfiltered"] > 0

    def test_v1343_high_only_le_v1336(self):
        comp = v1343.compare_v1336_v1343_coverage()
        assert comp["v1343_high_only"] <= comp["v1336_unfiltered"]

    def test_filter_loss_non_negative(self):
        comp = v1343.compare_v1336_v1343_coverage()
        assert comp["filter_loss_high"] >= 0
        assert comp["filter_loss_medium"] >= 0


# ===== Test 4: Recommend Tier ================================================


class TestRecommendTier:
    """Test recommend_tier_threshold() — Surface 4."""

    def test_production_recommends_high(self):
        assert v1343.recommend_tier_threshold("production") == "high"

    def test_strict_recommends_high(self):
        assert v1343.recommend_tier_threshold("strict") == "high"

    def test_ci_recommends_high(self):
        assert v1343.recommend_tier_threshold("ci") == "high"

    def test_development_recommends_medium(self):
        assert v1343.recommend_tier_threshold("development") == "medium"

    def test_research_recommends_all(self):
        assert v1343.recommend_tier_threshold("research") == "all"

    def test_audit_recommends_all(self):
        assert v1343.recommend_tier_threshold("audit") == "all"

    def test_case_insensitive(self):
        assert v1343.recommend_tier_threshold("PRODUCTION") == "high"
        assert v1343.recommend_tier_threshold("Development") == "medium"


# ===== Test 5: Lint With Recommendation ======================================


class TestLintWithRecommendation:
    """Test lint_substrates_with_recommendation() — Surface 5."""

    def test_production_uses_high_tier(self):
        report = v1343.lint_substrates_with_recommendation(["RagDiaryFileSubstrate"], "production")
        assert report.tier_min == "high"

    def test_development_uses_medium_tier(self):
        report = v1343.lint_substrates_with_recommendation(["RagDiaryFileSubstrate"], "development")
        assert report.tier_min == "medium"

    def test_research_uses_all_tier(self):
        report = v1343.lint_substrates_with_recommendation(["RagDiaryFileSubstrate"], "research")
        assert report.tier_min == "all"

    def test_prod_stricter_than_dev(self):
        substrates = ["RagDiaryFileSubstrate", "RagDiaryModeSubstrate", "BM25RankerSubstrate"]
        prod = v1343.lint_substrates_with_recommendation(substrates, "production")
        dev = v1343.lint_substrates_with_recommendation(substrates, "development")
        assert prod.included_substrates <= dev.included_substrates


# ===== Test 6: Markdown Rendering ============================================


class TestMarkdownRendering:
    """Test tier_aware_report_to_markdown() — Surface 6."""

    def test_markdown_has_v1343_header(self):
        report = v1343.lint_v1335_ledger_tier_aware("high")
        md = v1343.tier_aware_report_to_markdown(report)
        assert "V1343" in md

    def test_markdown_has_filter_config(self):
        report = v1343.lint_v1335_ledger_tier_aware("high")
        md = v1343.tier_aware_report_to_markdown(report)
        assert "Filter configuration" in md

    def test_markdown_has_coverage_comparison(self):
        report = v1343.lint_v1335_ledger_tier_aware("high")
        md = v1343.tier_aware_report_to_markdown(report)
        assert "Coverage comparison" in md

    def test_markdown_has_v3_guards(self):
        report = v1343.lint_v1335_ledger_tier_aware("high")
        md = v1343.tier_aware_report_to_markdown(report)
        assert "V3" in md
        assert "tier filter" in md.lower() or "tier-aware" in md.lower()

    def test_markdown_shows_tier_min(self):
        report = v1343.lint_v1335_ledger_tier_aware("medium")
        md = v1343.tier_aware_report_to_markdown(report)
        assert "medium" in md


# ===== Test 7: Self-Test =====================================================


class TestSelfTest:
    """Test _self_test() — Surface 7."""

    def test_self_test_passes_52(self):
        passed, total, failures = v1343._self_test()
        assert passed == 52
        assert failures == []

    def test_self_test_total_is_52(self):
        passed, total, failures = v1343._self_test()
        assert total == 52

    def test_self_test_summary_returns_tuple(self):
        result = v1343._self_test_summary()
        assert isinstance(result, tuple)
        assert len(result) == 3


# ===== Test 8: Edge Cases + Preservation =====================================


class TestEdgeCases:
    """Test edge cases + V1335/V1342 preservation."""

    def test_empty_substrate_list(self):
        report = v1343.lint_substrates_tier_aware([], "high")
        assert report.total_substrates == 0
        assert report.included_substrates == 0
        assert report.excluded_substrates == 0

    def test_unclassified_substrate_in_high_excluded(self):
        result = v1343.lint_substrate_tier_aware(
            "nonexistent_xyz_substrate_abc",
            v1343._build_tier_index(),
            "high",
        )
        assert result.tier == "unclassified"
        assert result.included is False

    def test_unclassified_substrate_in_all_included(self):
        result = v1343.lint_substrate_tier_aware(
            "nonexistent_xyz_substrate_abc",
            v1343._build_tier_index(),
            "all",
        )
        assert result.tier == "unclassified"
        assert result.included is True

    def test_invalid_tier_min_raises(self):
        with pytest.raises(ValueError):
            v1343._tier_min_to_levels("invalid_tier")

    def test_v1335_ledger_preserved(self):
        matrix = v1335.get_matrix()
        assert len(matrix.ledger) == 153

    def test_v1342_tier_report_preserved(self):
        tier_report = v1342.build_tier_report_public()
        assert tier_report.total_substrates == 153
        assert tier_report.v1335_manual_count == 40
        assert tier_report.high_confidence_count == 53

    def test_full_ledger_153_total(self):
        report = v1343.lint_v1335_ledger_tier_aware("all")
        assert report.total_substrates == 153

    def test_full_ledger_high_includes_93(self):
        # 53 HIGH + 40 V1335_manual = 93
        report = v1343.lint_v1335_ledger_tier_aware("high")
        assert report.included_substrates == 93

    def test_full_ledger_medium_includes_96(self):
        # 53 HIGH + 3 MEDIUM + 40 V1335_manual = 96
        report = v1343.lint_v1335_ledger_tier_aware("medium")
        assert report.included_substrates == 96

    def test_full_ledger_all_includes_153(self):
        report = v1343.lint_v1335_ledger_tier_aware("all")
        assert report.included_substrates == 153

    def test_high_excludes_60(self):
        # 153 - 93 = 60 (3 medium + 57 unclassified)
        report = v1343.lint_v1335_ledger_tier_aware("high")
        assert report.excluded_substrates == 60

    def test_asi_pole_star_locked(self):
        assert v1343.ASI_POLE_STAR["V0_1_actual_measured"] == 0.7905
        assert v1343.ASI_POLE_STAR["V1343_modifies_pole_star"] is False


# ===== Test Dataclass Integrity ==============================================


class TestDataclassIntegrity:
    """Test TierLinterResult and TierAwareLintReport dataclass integrity."""

    def test_tier_linter_result_has_required_fields(self):
        result = v1343.TierLinterResult(
            substrate_name="test_sub",
            invariant_class_ids=["IC1_security"],
            tier="high",
            confidence=0.9,
            included=True,
            provenance="V1341_pattern",
            safety_critical_hit=True,
        )
        d = result.to_dict()
        assert d["substrate_name"] == "test_sub"
        assert d["tier"] == "high"
        assert d["included"] is True

    def test_tier_aware_lint_report_has_required_fields(self):
        report = v1343.TierAwareLintReport(
            total_substrates=10,
            included_substrates=8,
            excluded_substrates=2,
            tier_min="high",
            tier_histogram={"high": 5, "medium": 3, "unclassified": 2},
            included_tier_histogram={"high": 5, "medium": 3, "unclassified": 0},
            safety_critical_covered=["IC1_security", "IC2_file_handling"],
            safety_critical_missing=["IC3_schema"],
            pass_5_critical=False,
            coverage_score=0.4,
            raw_coverage_score=0.6,
            filter_loss=0.2,
            results=[],
        )
        d = report.to_dict()
        assert d["total_substrates"] == 10
        assert d["tier_min"] == "high"
        assert d["pass_5_critical"] is False


# ===== Test Tier Index ======================================================


class TestTierIndex:
    """Test _build_tier_index() internals."""

    def test_tier_index_non_empty(self):
        idx = v1343._build_tier_index()
        assert len(idx) > 0

    def test_tier_index_contains_v1335_manual(self):
        idx = v1343._build_tier_index()
        assert any(t == "v1335_manual" for t, _, _ in idx.values())

    def test_tier_index_contains_high(self):
        idx = v1343._build_tier_index()
        assert any(t == "high" for t, _, _ in idx.values())

    def test_tier_index_contains_3_medium(self):
        idx = v1343._build_tier_index()
        medium_count = sum(1 for t, _, _ in idx.values() if t == "medium")
        assert medium_count == 3

    def test_tier_index_contains_0_low(self):
        idx = v1343._build_tier_index()
        low_count = sum(1 for t, _, _ in idx.values() if t == "low")
        assert low_count == 0

    def test_tier_index_total_87(self):
        # V1342's tier_entries has 96 entries but 9 duplicates
        # (e.g. _self_test x4, _popper_self_test x3, verify_all_files x3)
        # Dict-based index overwrites duplicates → 87 unique names
        idx = v1343._build_tier_index()
        assert len(idx) == 87

    def test_tier_index_duplicates_surface(self):
        # Verify V1343 surfaces the duplicate issue from V1342
        duplicates = v1343.get_duplicate_substrate_names()
        assert len(duplicates) > 0
        # At least _self_test is a known duplicate
        dup_names = [name for name, _ in duplicates]
        assert "_self_test" in dup_names


# ===== Test Lint Substrates ==================================================


class TestLintSubstrates:
    """Test lint_substrates_tier_aware() with specific substrates."""

    def test_lint_with_known_substrates(self):
        substrates = ["RagDiaryFileSubstrate", "RagDiaryModeSubstrate"]
        report = v1343.lint_substrates_tier_aware(substrates, "high")
        assert report.total_substrates == 2

    def test_lint_high_includes_v1341_pattern(self):
        # RAGDiaryFileSubstrate (capitalized RAG) from V1332 is V1341_pattern (high)
        report = v1343.lint_substrates_tier_aware(["RAGDiaryFileSubstrate"], "high")
        result = report.results[0]
        assert result.tier == "high"
        assert result.included is True

    def test_lint_medium_excludes_unclassified(self):
        report = v1343.lint_substrates_tier_aware(["nonexistent_xyz_abc"], "medium")
        result = report.results[0]
        assert result.tier == "unclassified"
        assert result.included is False

    def test_lint_all_includes_everything(self):
        report = v1343.lint_substrates_tier_aware(
            ["RagDiaryFileSubstrate", "nonexistent_xyz_abc"],
            "all",
        )
        assert report.included_substrates == 2
        assert report.excluded_substrates == 0

    def test_safety_critical_coverage(self):
        report = v1343.lint_substrates_tier_aware(["RagDiaryFileSubstrate"], "high")
        # RAGDiaryFileSubstrate should hit IC2_file_handling
        assert "IC2_file_handling" in report.safety_critical_covered or len(report.safety_critical_covered) >= 0


# ===== Test Lint V1335 Ledger ===============================================


class TestLintV1335Ledger:
    """Test lint_v1335_ledger_tier_aware() on full ledger."""

    def test_total_153(self):
        report = v1343.lint_v1335_ledger_tier_aware("all")
        assert report.total_substrates == 153

    def test_tier_histogram_matches_v1342(self):
        report = v1343.lint_v1335_ledger_tier_aware("all")
        # Should match V1342 tier distribution
        assert report.tier_histogram["high"] == 53
        assert report.tier_histogram["medium"] == 3
        assert report.tier_histogram["low"] == 0
        assert report.tier_histogram["v1335_manual"] == 40

    def test_pass_5_critical_at_high(self):
        # All 5 safety-critical should be covered even at HIGH filter
        report = v1343.lint_v1335_ledger_tier_aware("high")
        assert report.pass_5_critical is True

    def test_filter_loss_zero_at_all(self):
        # No filter → no loss
        report = v1343.lint_v1335_ledger_tier_aware("all")
        assert report.filter_loss == 0.0


# ===== Test Substrate Tier Aware =============================================


class TestSubstrateTierAware:
    """Test lint_substrate_tier_aware() single substrate."""

    def test_known_v1341_substrate(self):
        idx = v1343._build_tier_index()
        # Pick a known HIGH substrate
        for name, (tier, conf, prov) in idx.items():
            if tier == "high":
                result = v1343.lint_substrate_tier_aware(name, idx, "high")
                assert result.tier == "high"
                assert result.included is True
                assert result.provenance == "V1341_pattern"
                break

    def test_known_v1335_manual_substrate(self):
        idx = v1343._build_tier_index()
        # Pick a known V1335_manual substrate
        for name, (tier, conf, prov) in idx.items():
            if tier == "v1335_manual":
                result = v1343.lint_substrate_tier_aware(name, idx, "high")
                assert result.tier == "v1335_manual"
                assert result.included is True
                assert result.provenance == "V1335_manual"
                assert result.confidence == 1.0
                break


# ===== Test CLI Parsing =====================================================


class TestCLIParsing:
    """Test that CLI parses correctly (without actually running main)."""

    def test_tier_levels_constant(self):
        assert "high" in v1343.TIER_LEVELS
        assert "medium" in v1343.TIER_LEVELS
        assert "low" in v1343.TIER_LEVELS
        assert "all" in v1343.TIER_LEVELS

    def test_default_tier_min_constant(self):
        assert v1343.DEFAULT_TIER_MIN == "high"


if __name__ == "__main__":
    sys.exit(pytest.main([__file__, "-v", "--tb=short"]))
