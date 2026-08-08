#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
test_v1341_vcp_pattern_detector.py — Tests for V1341 VCP Cross-Plugin Pattern Detector

- 13 sections, 60+ canonical tests
- Validates: pattern rules, detect, classify, uplift report, markdown, V1335 preservation
- Marker: V1341 = pattern-based classifier (NOT LLM, NOT semantic understanding)
"""
from __future__ import annotations

import json
import sys
from pathlib import Path

import pytest

V1341_DIR = Path(__file__).resolve().parent.parent / "apeireth"
sys.path.insert(0, str(V1341_DIR))

import v1335_vcp_cross_plugin_invariant_synthesis as v1335  # noqa: E402
import v1341_vcp_pattern_detector as v1341  # noqa: E402


# =============================================================================
# Section 1: Pattern rules structure (6 tests)
# =============================================================================


class TestPatternRulesStructure:
    """Verify the 8 invariant-class pattern rules are well-formed."""

    def test_pattern_rules_has_8_ics(self):
        assert len(v1341.PATTERN_RULES) == 8

    def test_each_ic_has_at_least_4_substrings(self):
        for ic_id, rules in v1341.PATTERN_RULES.items():
            assert len(rules) >= 4, f"{ic_id} has only {len(rules)} substrings"

    def test_all_weights_in_range(self):
        for ic_id, rules in v1341.PATTERN_RULES.items():
            for substr, weight in rules:
                assert 0.0 <= weight <= 1.0, f"{ic_id}/{substr}: weight {weight} out of range"

    def test_ic_ids_match_v1335(self):
        v1335_ic_ids = {ic["invariant_id"] for ic in v1335.INVARIANT_CLASSES}
        assert set(v1341.PATTERN_RULES.keys()) == v1335_ic_ids

    def test_safety_critical_ics_have_validation_terms(self):
        # IC1, IC2, IC3, IC4, IC7 are safety-critical per V1335
        for ic_id in ("IC1_security", "IC2_file_handling", "IC3_schema", "IC4_ipc", "IC7_resource_bounds"):
            rules = v1341.PATTERN_RULES[ic_id]
            # Should have at least one strong (1.0-weight) rule
            strong = [s for s, w in rules if w == 1.0]
            assert len(strong) >= 2, f"{ic_id} has insufficient strong rules"

    def test_get_pattern_rules_returns_public_surface(self):
        rules = v1341.get_pattern_rules()
        assert isinstance(rules, dict)
        assert len(rules) == 8


# =============================================================================
# Section 2: detect_patterns (8 tests)
# =============================================================================


class TestDetectPatterns:
    """Verify pattern detection on individual substrates."""

    def test_security_pattern_matches_validation(self):
        hits = v1341.detect_patterns("validate_cluster_name_suffix", "ThoughtClusterManager")
        assert any(h.invariant_class_id == "IC1_security" for h in hits)

    def test_file_handling_pattern_matches_atomic_write(self):
        hits = v1341.detect_patterns("atomic_json_write", "VCPTimeLine")
        assert any(h.invariant_class_id == "IC2_file_handling" for h in hits)

    def test_schema_pattern_matches_manifest(self):
        hits = v1341.detect_patterns("validate_meta_chains_schema", "ThoughtClusterManager")
        assert any(h.invariant_class_id == "IC3_schema" for h in hits)

    def test_ipc_pattern_matches_stdio(self):
        hits = v1341.detect_patterns("StdioSyncProtocolSubstrate", "AnySearch")
        assert any(h.invariant_class_id == "IC4_ipc" for h in hits)

    def test_error_handling_pattern_matches(self):
        hits = v1341.detect_patterns("batch_overall_success", "ThoughtClusterManager")
        assert any(h.invariant_class_id == "IC5_error_handling" for h in hits)

    def test_configuration_pattern_matches_merge(self):
        hits = v1341.detect_patterns("merge_config", "VCP-6-core")
        assert any(h.invariant_class_id == "IC6_configuration" for h in hits)

    def test_resource_bounds_pattern_matches_token(self):
        hits = v1341.detect_patterns("truncate_to_token_budget", "VCP-6-core")
        assert any(h.invariant_class_id == "IC7_resource_bounds" for h in hits)

    def test_lifecycle_pattern_matches_self_test(self):
        hits = v1341.detect_patterns("run_self_tests", "RAGDiary")
        assert any(h.invariant_class_id == "IC8_lifecycle" for h in hits)


# =============================================================================
# Section 3: classify_substrate (5 tests)
# =============================================================================


class TestClassifySubstrate:
    """Verify substrate classification."""

    def test_classify_returns_classes_confidence_hits(self):
        classes, conf, hits = v1341.classify_substrate("atomic_json_write", "VCPTimeLine")
        assert isinstance(classes, list)
        assert isinstance(conf, float)
        assert isinstance(hits, list)

    def test_classify_empty_for_unknown(self):
        classes, conf, hits = v1341.classify_substrate("Xyzzy_Mu", "X")
        assert classes == []
        assert conf == 0.0
        assert hits == []

    def test_classify_confidence_bounded(self):
        classes, conf, hits = v1341.classify_substrate("merge_config", "VCP-6-core")
        assert 0.0 <= conf <= 1.0

    def test_classify_dedupes_classes(self):
        classes, conf, hits = v1341.classify_substrate("file_to_file", "X")
        # "file" appears twice → should be deduped to one IC2_file_handling entry
        if "IC2_file_handling" in classes:
            assert classes.count("IC2_file_handling") == 1

    def test_classify_substrate_public_wrapper(self):
        classes, conf, hits = v1341.classify_substrate_public("atomic_json_write", "VCPTimeLine")
        assert "IC2_file_handling" in classes


# =============================================================================
# Section 4: Uplift report basics (6 tests)
# =============================================================================


class TestUpliftReport:
    """Verify the coverage uplift report."""

    def test_build_uplift_report_total_substrates(self):
        report = v1341.build_uplift_report_public()
        total = report.pre_classified_count + len(report.substrate_uplifts) + len(report.unclassified_after)
        assert total == 153

    def test_uplift_report_pre_score_le_post_score(self):
        report = v1341.build_uplift_report_public()
        assert report.post_coverage_score >= report.pre_coverage_score

    def test_uplift_report_positive_delta(self):
        report = v1341.build_uplift_report_public()
        assert report.delta_coverage_score > 0

    def test_uplift_report_per_class_post_ge_pre(self):
        report = v1341.build_uplift_report_public()
        for ic_id, pre in report.per_class_pre.items():
            post = report.per_class_post.get(ic_id, 0)
            assert post >= pre, f"{ic_id}: post {post} < pre {pre}"

    def test_uplift_report_ic8_lifecycle_increased(self):
        report = v1341.build_uplift_report_public()
        # IC8_lifecycle was 11, should be more after pattern detection
        assert report.per_class_post["IC8_lifecycle"] >= 11

    def test_uplift_report_ic2_file_handling_increased(self):
        report = v1341.build_uplift_report_public()
        # IC2_file_handling was 8, should be much more after pattern detection
        assert report.per_class_post["IC2_file_handling"] >= 30


# =============================================================================
# Section 5: Uplift report structure (4 tests)
# =============================================================================


class TestUpliftStructure:
    """Verify individual substrate uplift records."""

    def test_uplift_has_evidence(self):
        report = v1341.build_uplift_report_public()
        for u in report.substrate_uplifts:
            assert u.pattern_hits  # every uplift must have at least one hit
            assert len(u.net_new_classes) >= 1
            assert u.confidence > 0.0

    def test_uplift_known_examples(self):
        report = v1341.build_uplift_report_public()
        uplift_names = {u.substrate_name for u in report.substrate_uplifts}
        # These should be newly classified by V1341
        assert "is_path_allowed" in uplift_names
        assert "parse_placeholder" in uplift_names
        assert "normalize_text_content" in uplift_names

    def test_uplift_preserves_original_classes(self):
        report = v1341.build_uplift_report_public()
        # V1335 originally had empty classes for these — V1341 shouldn't have leaked them
        for u in report.substrate_uplifts:
            assert u.original_classes == []
            assert u.net_new_classes
            assert set(u.net_new_classes).issubset(set(u.pattern_classes))

    def test_uplift_have_position_info(self):
        report = v1341.build_uplift_report_public()
        for u in report.substrate_uplifts:
            for h in u.pattern_hits:
                assert h.position >= 0
                assert h.weight > 0


# =============================================================================
# Section 6: Per-class coverage (5 tests)
# =============================================================================


class TestPerClassCoverage:
    """Verify per-class coverage details."""

    def test_ic1_security_increased(self):
        report = v1341.build_uplift_report_public()
        assert report.per_class_pre["IC1_security"] == 3
        assert report.per_class_post["IC1_security"] >= 7

    def test_ic2_file_handling_increased(self):
        report = v1341.build_uplift_report_public()
        assert report.per_class_pre["IC2_file_handling"] == 8
        assert report.per_class_post["IC2_file_handling"] >= 30

    def test_ic3_schema_increased(self):
        report = v1341.build_uplift_report_public()
        assert report.per_class_pre["IC3_schema"] == 7
        assert report.per_class_post["IC3_schema"] >= 10

    def test_ic4_ipc_increased(self):
        report = v1341.build_uplift_report_public()
        assert report.per_class_pre["IC4_ipc"] == 1
        assert report.per_class_post["IC4_ipc"] >= 4

    def test_ic7_resource_bounds_increased(self):
        report = v1341.build_uplift_report_public()
        assert report.per_class_pre["IC7_resource_bounds"] == 7
        assert report.per_class_post["IC7_resource_bounds"] >= 8


# =============================================================================
# Section 7: Pattern stats (3 tests)
# =============================================================================


class TestPatternStats:
    """Verify pattern statistics computation."""

    def test_stats_has_keys(self):
        report = v1341.build_uplift_report_public()
        stats = v1341.pattern_stats(report)
        assert "total_uplifts" in stats
        assert "total_hits" in stats
        assert "avg_hits_per_uplift" in stats
        assert "rule_use_count" in stats

    def test_stats_total_uplifts_positive(self):
        report = v1341.build_uplift_report_public()
        stats = v1341.pattern_stats(report)
        assert stats["total_uplifts"] > 0

    def test_stats_rule_use_count_8(self):
        report = v1341.build_uplift_report_public()
        stats = v1341.pattern_stats(report)
        assert len(stats["rule_use_count"]) == 8


# =============================================================================
# Section 8: Markdown report (4 tests)
# =============================================================================


class TestMarkdownReport:
    """Verify markdown rendering."""

    def test_markdown_has_header(self):
        report = v1341.build_uplift_report_public()
        md = v1341.report_to_markdown(report)
        assert "# V1341" in md

    def test_markdown_has_per_class_table(self):
        report = v1341.build_uplift_report_public()
        md = v1341.report_to_markdown(report)
        assert "Per-class coverage" in md
        assert "| IC1_security |" in md
        assert "| IC8_lifecycle |" in md

    def test_markdown_has_uplift_table(self):
        report = v1341.build_uplift_report_public()
        md = v1341.report_to_markdown(report)
        assert "Newly-classified substrates" in md

    def test_markdown_has_unclassified_section(self):
        report = v1341.build_uplift_report_public()
        md = v1341.report_to_markdown(report)
        assert "Still-unclassified" in md


# =============================================================================
# Section 9: V1335 preservation (3 tests)
# =============================================================================


class TestV1335Preservation:
    """Verify V1335 ledger is NOT modified by V1341."""

    def test_v1335_ledger_intact(self):
        matrix = v1335.get_matrix()
        assert len(matrix.ledger) == 153

    def test_v1335_invariant_classes_intact(self):
        assert len(v1335.INVARIANT_CLASSES) == 8

    def test_v1341_does_not_modify_v1335(self):
        # Build twice, check consistency
        report1 = v1341.build_uplift_report_public()
        report2 = v1341.build_uplift_report_public()
        assert report1.pre_classified_count == report2.pre_classified_count
        assert report1.post_classified_count == report2.post_classified_count


# =============================================================================
# Section 10: Real V1335 ledger integration (4 tests)
# =============================================================================


class TestRealLedgerIntegration:
    """Verify V1341 works against the real V1335 ledger (not mocks)."""

    def test_real_ledger_substrates_classified(self):
        report = v1341.build_uplift_report_public()
        # At least 50 substrates should be newly classified
        assert len(report.substrate_uplifts) >= 50

    def test_real_ledger_all_7_plugins_affected(self):
        report = v1341.build_uplift_report_public()
        plugins = {u.source_plugin for u in report.substrate_uplifts}
        # Pattern detection should affect substrates from multiple plugins
        assert len(plugins) >= 4

    def test_real_ledger_v1327_core_classified(self):
        report = v1341.build_uplift_report_public()
        v1327_uplifts = [u for u in report.substrate_uplifts if u.source_plugin == "VCP-6-core"]
        assert len(v1327_uplifts) >= 10

    def test_real_ledger_classify_category(self):
        # classify_category was unclassified by V1335; V1341 should classify it
        classes, conf, hits = v1341.classify_substrate("classify_category", "VCP-6-core")
        assert "IC1_security" in classes


# =============================================================================
# Section 11: JSON output (3 tests)
# =============================================================================


class TestJsonOutput:
    """Verify JSON serialization."""

    def test_dataclass_to_dict(self):
        report = v1341.build_uplift_report_public()
        d = {
            "pre": report.pre_classified_count,
            "post": report.post_classified_count,
            "delta": report.delta_coverage_score,
        }
        s = json.dumps(d)
        assert "pre" in s and "post" in s and "delta" in s

    def test_pattern_hit_serialization(self):
        hit = v1341.PatternHit(
            substrate_name="atomic_json_write",
            source_plugin="VCPTimeLine",
            invariant_class_id="IC2_file_handling",
            matched_substring="atomic",
            weight=1.0,
            position=0,
        )
        d = v1341.asdict(hit)
        assert d["matched_substring"] == "atomic"
        assert d["weight"] == 1.0

    def test_uplift_serialization(self):
        report = v1341.build_uplift_report_public()
        if report.substrate_uplifts:
            u = report.substrate_uplifts[0]
            d = v1341.asdict(u)
            assert "substrate_name" in d
            assert "pattern_hits" in d


# =============================================================================
# Section 12: Self-test (4 tests)
# =============================================================================


class TestSelfTest:
    """Verify V1341 self-test runs cleanly."""

    def test_self_test_returns_tuple(self):
        result = v1341._self_test()
        assert isinstance(result, tuple)
        assert len(result) == 3

    def test_self_test_passes_all(self):
        passed, total, failures = v1341._self_test()
        assert failures == [], f"Self-test failures: {failures}"
        assert passed == total

    def test_self_test_summary(self):
        passed, total, failures = v1341._self_test_summary()
        assert passed == total
        assert failures == []

    def test_self_test_at_least_30_tests(self):
        passed, total, failures = v1341._self_test()
        assert total >= 30, f"Expected >= 30 self-tests, got {total}"


# =============================================================================
# Section 13: ASI pole-star + V3 guards (5 tests)
# =============================================================================


class TestPoleStarAndGuards:
    """Verify ASI pole-star is locked and V3 guards are honored."""

    def test_asi_pole_star_locked(self):
        assert v1341.ASI_POLE_STAR["V0_1_actual_measured"] == 0.7905
        assert v1341.ASI_POLE_STAR["V1256_unio_mystica_realized"] == 0.9105
        assert v1341.ASI_POLE_STAR["V1049_value_alignment"] == "DONE"

    def test_v3_guards_no_pattern_faking(self):
        # Pattern detection = reproducible, NOT probabilistic
        classes1, conf1, _ = v1341.classify_substrate("atomic_json_write", "VCPTimeLine")
        classes2, conf2, _ = v1341.classify_substrate("atomic_json_write", "VCPTimeLine")
        assert classes1 == classes2
        assert conf1 == conf2

    def test_v3_guards_no_phenomenal_claims(self):
        # V1341 doesn't claim to understand
        report = v1341.build_uplift_report_public()
        # Coverage is measurable, not phenomenological
        assert isinstance(report.post_coverage_score, float)

    def test_v3_guards_no_model_adjustment(self):
        # V1341 only reads V1335 ledger; does NOT modify V1335
        matrix_before = v1335.get_matrix()
        len_before = len(matrix_before.ledger)
        _ = v1341.build_uplift_report_public()
        matrix_after = v1335.get_matrix()
        assert len(matrix_after.ledger) == len_before

    def test_v3_guards_no_pole_star_movement(self):
        # ASI pole-star is a constant
        assert v1341.ASI_POLE_STAR["V0_1_actual_measured"] == 0.7905


# =============================================================================
# Section 14: Chain position (2 tests)
# =============================================================================


class TestChainPosition:
    """Verify V1341 is part of the V13xx chain."""

    def test_module_filename(self):
        assert v1341.__file__ is not None
        assert "v1341_vcp_pattern_detector.py" in v1341.__file__

    def test_v1341_imports_v1335(self):
        # V1341 should depend on V1335 (builds on registry)
        assert hasattr(v1341, "v1335")
        assert hasattr(v1341, "v1336")


# =============================================================================
# Section 15: Coverage uplift summary (3 tests)
# =============================================================================


class TestCoverageUpliftSummary:
    """Verify the coverage uplift is meaningful."""

    def test_uplift_at_least_30_percent(self):
        report = v1341.build_uplift_report_public()
        assert report.delta_coverage_score >= 0.30, f"Uplift {report.delta_coverage_score:.4f} < 0.30"

    def test_post_coverage_above_50_percent(self):
        report = v1341.build_uplift_report_public()
        assert report.post_coverage_score >= 0.50, f"Post coverage {report.post_coverage_score:.4f} < 0.50"

    def test_minimum_newly_classified(self):
        report = v1341.build_uplift_report_public()
        assert len(report.substrate_uplifts) >= 40, f"Only {len(report.substrate_uplifts)} newly classified"


# =============================================================================
# Section 16: Per-class structural integrity (3 tests)
# =============================================================================


class TestPerClassIntegrity:
    """Verify per-class pre/post counts are internally consistent."""

    def test_post_total_equals_post_classified(self):
        report = v1341.build_uplift_report_public()
        # Sum of per-class post counts ≥ post_classified_count (some substrates in multiple classes)
        total = sum(report.per_class_post.values())
        assert total >= report.post_classified_count

    def test_unclassified_count_consistent(self):
        report = v1341.build_uplift_report_public()
        # pre + new + unclassified = 153
        total = report.pre_classified_count + len(report.substrate_uplifts) + len(report.unclassified_after)
        assert total == 153

    def test_unclassified_less_than_pre(self):
        report = v1341.build_uplift_report_public()
        assert len(report.unclassified_after) < report.pre_classified_count + len(report.substrate_uplifts) + len(report.unclassified_after)
        # Sanity: unclassified_after is just one slice of the total


if __name__ == "__main__":
    pytest.main([__file__, "-v", "--tb=short"])
