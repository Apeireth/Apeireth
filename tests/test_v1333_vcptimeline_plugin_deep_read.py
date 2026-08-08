#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
test_v1333_vcptimeline_plugin_deep_read.py — pytest suite for V1333

V1333 = VCPTimeLine VCP Plugin 真源码深读 (5th VCP plugin deep-read, post-V1332 RAGDiary)

Tests cover:
1.  File integrity (2 files, 824 lines, 38685 bytes)
2.  TimelinePlaceholderSubstrate (regex + parse + default_k/threshold)
3.  CaseInsensitiveDirSubstrate (Linux fs大小写敏感 → existing variant lookup)
4.  AtomicJsonWriteSubstrate (temp + rename, render, atomic OS support)
5.  SingleDeclAntiRecursionSubstrate (anti-recursion + once-only expand)
6.  WeightedQueryVectorSubstrate (0.7 user + 0.3 ai, normalize None vectors)
7.  TagMemoGeodesicSubstrate (candidate_k = max(K*8,20), score-per-month aggregation)
8.  MapReduceSummarySubstrate + TokenEstimatorSubstrate (zh*1.5 + ascii*0.25)
9.  LockStatusDualSubstrate (5 phases + idle + conflict code)
10. RouteSignatureProbeSubstrate + ManifestSubstrate (4-param check + 11 routes)
11. Bridge (chain_position=20, 5 plugins read)
12. ASI pole-star integrity (V0.1=0.7905 LOCKED, V1333 does NOT modify)
13. Run-all self-test gate (54 checks)
"""
import pytest
import sys
from pathlib import Path

# Ensure promethean root is on sys.path
PROMETHEAN_ROOT = Path(__file__).resolve().parent.parent
if str(PROMETHEAN_ROOT) not in sys.path:
    sys.path.insert(0, str(PROMETHEAN_ROOT))

from apeireth.v1333_vcptimeline_plugin_deep_read import (
    ASI_POLE_STAR,
    VCPTIMELINE_2_FILES,
    VCPTIMELINE_ROOT,
    PlaceholderSpec,
    TimelinePlaceholderSubstrate,
    CaseInsensitiveDirSubstrate,
    AtomicJsonWriteSubstrate,
    SingleDeclAntiRecursionSubstrate,
    WeightedQueryVectorSubstrate,
    TagMemoGeodesicSubstrate,
    MapReduceSummarySubstrate,
    TokenEstimatorSubstrate,
    GenerationStatus,
    LockStatusDualSubstrate,
    RouteSignatureProbeSubstrate,
    VCPTimeLineManifestSubstrate,
    VCPTimeLinePluginMatrix,
    VCPTimeLineDeepReadBridge,
    verify_all_files,
    parse_placeholder,
    _self_test,
)


# ===========================================================================
# 1. File integrity
# ===========================================================================
class TestFileIntegrity:
    def test_2_files_specified(self):
        assert len(VCPTIMELINE_2_FILES) == 2

    def test_root_path_set(self):
        assert VCPTIMELINE_ROOT.exists(), f"VCPTimeLine root not found: {VCPTIMELINE_ROOT}"

    def test_total_declared_lines_824(self):
        total = sum(f["declared_lines"] for f in VCPTIMELINE_2_FILES)
        assert total == 824

    def test_f1_main_declared_804(self):
        f1 = next(f for f in VCPTIMELINE_2_FILES if f["file_id"] == "F1_main_coordinator")
        assert f1["declared_lines"] == 804
        assert f1["expected_byte_size"] == 38050

    def test_f2_manifest_declared_20(self):
        f2 = next(f for f in VCPTIMELINE_2_FILES if f["file_id"] == "F2_manifest")
        assert f2["declared_lines"] == 20

    def test_main_file_exists_with_correct_bytes(self):
        main_path = VCPTIMELINE_ROOT / "VCPTimeLine.js"
        assert main_path.exists()
        assert main_path.stat().st_size == 38050

    def test_actual_lines_match_declared(self):
        files = verify_all_files()
        for f in files:
            if f["file_id"] == "F1_main_coordinator":
                assert f["actual_lines"] == 804, f"VCPTimeLine.js actual lines = {f['actual_lines']}, expected 804"
            elif f["file_id"] == "F2_manifest":
                assert f["actual_lines"] == 20, f"manifest actual lines = {f['actual_lines']}, expected 20"

    def test_sha256_present(self):
        files = verify_all_files()
        for f in files:
            assert len(f["sha256_first16"]) == 16, f"{f['file_id']} sha256 missing or wrong length"

    def test_matrix_total_lines_824(self):
        files = verify_all_files()
        matrix = VCPTimeLinePluginMatrix(files=files)
        assert matrix.total_lines() == 824

    def test_matrix_total_bytes(self):
        files = verify_all_files()
        matrix = VCPTimeLinePluginMatrix(files=files)
        assert matrix.total_bytes() == 38050 + 635  # main + manifest

    def test_matrix_integrity_pass(self):
        files = verify_all_files()
        matrix = VCPTimeLinePluginMatrix(files=files)
        assert matrix.integrity_pass() is True


# ===========================================================================
# 2. TimelinePlaceholderSubstrate
# ===========================================================================
class TestPlaceholderSubstrate:
    def test_default_k_3(self):
        sub = TimelinePlaceholderSubstrate()
        assert sub.default_k == 3

    def test_default_threshold_0_5(self):
        sub = TimelinePlaceholderSubstrate()
        assert sub.default_threshold == 0.5

    def test_compile_returns_pattern(self):
        sub = TimelinePlaceholderSubstrate()
        pat = sub.compile()
        assert pat is not None

    def test_parse_first_simple(self):
        sub = TimelinePlaceholderSubstrate()
        spec = sub.parse_first("[[VCPTimeLine::小克日记本]]")
        assert spec is not None
        assert spec.agent_name == "小克日记本"
        assert spec.k == 3
        assert spec.threshold == 0.5

    def test_parse_first_with_k(self):
        sub = TimelinePlaceholderSubstrate()
        spec = sub.parse_first("[[VCPTimeLine::小克:5]]")
        assert spec is not None
        assert spec.k == 5
        assert spec.threshold == 0.5

    def test_parse_first_with_threshold(self):
        sub = TimelinePlaceholderSubstrate()
        spec = sub.parse_first("[[VCPTimeLine::小克:0.42]]")
        assert spec is not None
        assert spec.k == 3
        assert abs(spec.threshold - 0.42) < 1e-9

    def test_parse_first_full(self):
        sub = TimelinePlaceholderSubstrate()
        spec = sub.parse_first("[[VCPTimeLine::DevLog:5:0.42]]")
        assert spec is not None
        assert spec.agent_name == "DevLog"
        assert spec.k == 5
        assert abs(spec.threshold - 0.42) < 1e-9

    def test_parse_first_no_match(self):
        sub = TimelinePlaceholderSubstrate()
        assert sub.parse_first("no placeholder here") is None

    def test_parse_all_multiple(self):
        sub = TimelinePlaceholderSubstrate()
        specs = sub.parse_all("first [[VCPTimeLine::A:1]] second [[VCPTimeLine::B:2:0.3]]")
        assert len(specs) == 2
        assert specs[0].agent_name == "A" and specs[0].k == 1
        assert specs[1].agent_name == "B" and specs[1].k == 2 and abs(specs[1].threshold - 0.3) < 1e-9

    def test_parse_placeholder_function(self):
        spec = parse_placeholder("[[VCPTimeLine::Test:7:0.6]]")
        assert spec.agent_name == "Test"
        assert spec.k == 7
        assert abs(spec.threshold - 0.6) < 1e-9


# ===========================================================================
# 3. CaseInsensitiveDirSubstrate
# ===========================================================================
class TestCaseInsensitiveDir:
    def test_default_suffix_timeline(self):
        sub = CaseInsensitiveDirSubstrate()
        assert sub.expected_suffix == "timeline"

    def test_resolve_existing_variant(self):
        sub = CaseInsensitiveDirSubstrate()
        siblings = ["小克timeline", "公共", "node_modules"]
        actual = sub.resolve_actual_name("小克", siblings)
        assert actual == "小克timeline"

    def test_resolve_uppercase_variant(self):
        sub = CaseInsensitiveDirSubstrate()
        siblings = ["小克TIMELINE"]
        actual = sub.resolve_actual_name("小克", siblings)
        assert actual == "小克TIMELINE"

    def test_resolve_no_siblings_returns_default(self):
        sub = CaseInsensitiveDirSubstrate()
        actual = sub.resolve_actual_name("小克", [])
        assert actual == "小克timeline"

    def test_candidate_variants_count(self):
        sub = CaseInsensitiveDirSubstrate()
        variants = sub.candidate_variants("Agent")
        assert len(variants) == 3

    def test_candidate_variants_lowercased_match(self):
        sub = CaseInsensitiveDirSubstrate()
        variants = sub.candidate_variants("Agent")
        # All variants should lowercase to "agenttimeline"
        assert all(v.lower() == "agenttimeline" for v in variants)


# ===========================================================================
# 4. AtomicJsonWriteSubstrate
# ===========================================================================
class TestAtomicJsonWrite:
    def test_default_indent_2(self):
        sub = AtomicJsonWriteSubstrate()
        assert sub.indent == 2

    def test_default_encoding_utf8(self):
        sub = AtomicJsonWriteSubstrate()
        assert sub.encoding in ("utf-8", "utf8")

    def test_stage_writes_pattern(self):
        sub = AtomicJsonWriteSubstrate()
        temp, final = sub.stage_writes("/dailynote/x/timeline_summaries.json")
        assert temp.endswith(".tmp")
        assert not final.endswith(".tmp")
        assert "<pid>" in temp and "<ts>" in temp

    def test_render_newline_terminated(self):
        sub = AtomicJsonWriteSubstrate()
        rendered = sub.render({"x": 1, "y": [1, 2]})
        assert rendered.endswith("\n")
        assert "\"x\": 1" in rendered

    def test_render_unicode_safe(self):
        sub = AtomicJsonWriteSubstrate()
        rendered = sub.render({"name": "小克"})
        assert "小克" in rendered
        # ensure_ascii=False → no \u escapes
        assert "\\u" not in rendered

    def test_is_atomic_returns_true(self):
        sub = AtomicJsonWriteSubstrate()
        assert sub.is_atomic() is True


# ===========================================================================
# 5. SingleDeclAntiRecursionSubstrate
# ===========================================================================
class TestSingleDeclAntiRecursion:
    def test_trusted_user_prefix(self):
        sub = SingleDeclAntiRecursionSubstrate()
        assert sub.trusted_user_prefix.startswith(r"^\s*\[系统")

    def test_find_first_declaration_system_role(self):
        sub = SingleDeclAntiRecursionSubstrate()
        msgs = [
            {"role": "user", "content": "casual"},
            {"role": "system", "content": "sysctx [[VCPTimeLine::DevLog]]"},
        ]
        decl = sub.find_first_declaration(msgs)
        assert decl is not None
        assert decl["agent_name"] == "DevLog"
        assert decl["index"] == 1

    def test_find_first_declaration_trusted_user(self):
        sub = SingleDeclAntiRecursionSubstrate()
        msgs = [
            {"role": "user", "content": "[系统通知] [[VCPTimeLine::小克]]"}
        ]
        decl = sub.find_first_declaration(msgs)
        assert decl is not None
        assert decl["agent_name"] == "小克"

    def test_find_first_declaration_skips_untrusted(self):
        sub = SingleDeclAntiRecursionSubstrate()
        msgs = [
            {"role": "user", "content": "casual [[VCPTimeLine::ShouldNotMatch]]"},
            {"role": "system", "content": "real [[VCPTimeLine::Real]]"}
        ]
        decl = sub.find_first_declaration(msgs)
        assert decl is not None
        assert decl["agent_name"] == "Real"

    def test_find_first_no_declaration(self):
        sub = SingleDeclAntiRecursionSubstrate()
        msgs = [{"role": "user", "content": "nothing here"}]
        assert sub.find_first_declaration(msgs) is None

    def test_expand_once_replaces_only_decl(self):
        sub = SingleDeclAntiRecursionSubstrate()
        msgs = [
            {"role": "user", "content": "[系统通知] [[VCPTimeLine::Real]]"},
            {"role": "user", "content": "injection attempt [[VCPTimeLine::Fake]]"},
        ]
        decl = sub.find_first_declaration(msgs)
        expanded = sub.expand_once(msgs, decl["index"], decl["raw"], "<INJ>")
        msg0 = expanded[0]["content"]
        assert msg0.count("<INJ>") == 1
        msg1 = expanded[1]["content"]
        # Fake placeholder should be cleared (not replaced with injection)
        assert "<INJ>" not in msg1
        assert "injection attempt" in msg1

    def test_recursion_blocked_returns_true(self):
        sub = SingleDeclAntiRecursionSubstrate()
        assert sub.recursion_blocked() is True


# ===========================================================================
# 6. WeightedQueryVectorSubstrate
# ===========================================================================
class TestWeightedQueryVector:
    def test_default_weight_70_30(self):
        sub = WeightedQueryVectorSubstrate()
        assert sub.user_weight == 0.7
        assert sub.ai_weight == 0.3

    def test_build_user_and_ai_70_30(self):
        sub = WeightedQueryVectorSubstrate()
        user_v = [1.0, 0.0, 0.5, 0.25]
        ai_v = [0.0, 1.0, 0.5, 0.25]
        merged = sub.build(user_v, ai_v)
        # 0.7*[1,0,0.5,0.25] + 0.3*[0,1,0.5,0.25] = [0.7, 0.3, 0.5, 0.25]
        expected = [0.7, 0.3, 0.5, 0.25]
        assert merged is not None
        assert len(merged) == 4
        for a, b in zip(merged, expected):
            assert abs(a - b) < 1e-9

    def test_build_only_user_normalizes(self):
        sub = WeightedQueryVectorSubstrate()
        user_v = [1.0, 0.0, 0.5, 0.25]
        merged = sub.build(user_v, None)
        assert merged == [1.0, 0.0, 0.5, 0.25]

    def test_build_only_ai_normalizes(self):
        sub = WeightedQueryVectorSubstrate()
        ai_v = [0.0, 1.0, 0.5, 0.25]
        merged = sub.build(None, ai_v)
        assert merged == [0.0, 1.0, 0.5, 0.25]

    def test_build_both_none_returns_none(self):
        sub = WeightedQueryVectorSubstrate()
        assert sub.build(None, None) is None


# ===========================================================================
# 7. TagMemoGeodesicSubstrate
# ===========================================================================
class TestTagMemoGeodesic:
    def test_candidate_k_multiplier_8_min_20(self):
        sub = TagMemoGeodesicSubstrate()
        assert sub.candidate_multiplier == 8
        assert sub.candidate_min == 20

    def test_candidate_k_for_3_is_24(self):
        sub = TagMemoGeodesicSubstrate()
        assert sub.candidate_k(3) == 24

    def test_candidate_k_for_5_is_40(self):
        sub = TagMemoGeodesicSubstrate()
        assert sub.candidate_k(5) == 40

    def test_candidate_k_floor_20(self):
        sub = TagMemoGeodesicSubstrate()
        assert sub.candidate_k(2) == 20  # 2*8=16 < 20

    def test_select_top_k_filters_threshold(self):
        sub = TagMemoGeodesicSubstrate()
        fake_months = {f"2025-{m:02d}" for m in range(1, 9)}
        chunks = [
            {"fullPath": f"/dailynote/小克timeline/2025-0{i}.md", "rerank_score": 0.91 - i*0.09}
            for i in range(1, 9)
        ]
        selected = sub.select_top_k_by_score(chunks, fake_months, 0.5, 3)
        assert len(selected) == 3
        assert all(s["score"] >= 0.5 for s in selected)
        assert [s["score"] for s in selected] == [0.91 - i*0.09 for i in range(1, 4)]

    def test_select_top_k_handles_missing_score(self):
        sub = TagMemoGeodesicSubstrate()
        fake_months = {"2025-01"}
        chunks = [
            {"fullPath": "/x/2025-01.md", "rerank_score": None},  # non-finite
            {"fullPath": "/x/2025-01.md", "rerank_score": 0.7},
        ]
        selected = sub.select_top_k_by_score(chunks, fake_months, 0.5, 2)
        assert len(selected) == 1
        assert selected[0]["score"] == 0.7

    def test_mode_tagmemo_with_geodesic(self):
        sub = TagMemoGeodesicSubstrate()
        assert sub.mode_descriptor({"tagMemoUsed": True, "geodesicRerankUsed": True}) == "TagMemo 浪潮 + 测地线重排"

    def test_mode_tagmemo_only(self):
        sub = TagMemoGeodesicSubstrate()
        assert sub.mode_descriptor({"tagMemoUsed": True, "geodesicRerankUsed": False}) == "TagMemo 浪潮"

    def test_mode_vector(self):
        sub = TagMemoGeodesicSubstrate()
        assert sub.mode_descriptor({}) == "向量检索"


# ===========================================================================
# 8. MapReduce + TokenEstimator
# ===========================================================================
class TestMapReduceAndTokenEstimator:
    def test_effective_input_budget(self):
        sub = MapReduceSummarySubstrate()
        assert sub.effective_input_budget() == max(512, 60000 - 4000 - 1000)

    def test_reduce_strategy_single(self):
        sub = MapReduceSummarySubstrate()
        assert sub.reduce_strategy(1) == "single-pass"

    def test_reduce_strategy_two(self):
        sub = MapReduceSummarySubstrate()
        assert sub.reduce_strategy(2) == "two-stage-merge"

    def test_reduce_strategy_iterative(self):
        sub = MapReduceSummarySubstrate()
        assert sub.reduce_strategy(5) == "iterative-merge"

    def test_te_estimate_chinese(self):
        sub = TokenEstimatorSubstrate()
        assert sub.estimate("你好世界") == 6  # 4 zh chars * 1.5 = 6

    def test_te_estimate_ascii(self):
        sub = TokenEstimatorSubstrate()
        assert sub.estimate("hello") == 2  # 5 * 0.25 = 1.25 → ceil=2

    def test_te_estimate_mixed(self):
        sub = TokenEstimatorSubstrate()
        # "hi你好" = 2 ascii + 2 zh → 2*0.25 + 2*1.5 = 0.5 + 3 = 3.5 → ceil=4
        assert sub.estimate("hi你好") == 4

    def test_te_estimate_empty(self):
        sub = TokenEstimatorSubstrate()
        assert sub.estimate("") == 0

    def test_te_split_by_budget_separates(self):
        sub = TokenEstimatorSubstrate()
        # Each "你好" is 3 tokens (2 chars * 1.5 → ceil=3), budget=4 → 1 per chunk
        items = ["你好", "你好你好", "你好你好你好"]
        chunks = sub.split_by_budget(items, budget=4)
        assert len(chunks) >= 2

    def test_te_split_by_budget_joins_with_separator(self):
        sub = TokenEstimatorSubstrate()
        items = ["你好", "你好"]
        chunks = sub.split_by_budget(items, budget=500)
        assert len(chunks) == 1
        assert "\n\n---\n\n" in chunks[0]


# ===========================================================================
# 9. LockStatusDualSubstrate
# ===========================================================================
class TestLockStatusDual:
    def test_phase_count(self):
        sub = LockStatusDualSubstrate()
        assert len(sub.phases) == 5

    def test_phase_labels_present(self):
        sub = LockStatusDualSubstrate()
        for label_key in ("preparing", "generating", "summarizing", "completed", "failed", "idle"):
            assert label_key in sub.phase_labels

    def test_new_status_running(self):
        sub = LockStatusDualSubstrate()
        s = sub.new_status("小克", "timeline")
        assert s.agent_name == "小克"
        assert s.kind == "timeline"
        assert s.running is True
        assert s.phase == "preparing"
        assert s.phase_label == "准备源数据"
        assert s.completed == 0
        assert s.total == 0

    def test_idle_status(self):
        sub = LockStatusDualSubstrate()
        s = sub.idle_status("小克")
        assert s.running is False
        assert s.phase == "idle"
        assert s.phase_label == "空闲"
        assert s.kind is None

    def test_conflict_code(self):
        sub = LockStatusDualSubstrate()
        assert sub.conflict_code == "TIMELINE_GENERATION_IN_PROGRESS"

    def test_status_to_dict(self):
        sub = LockStatusDualSubstrate()
        s = sub.new_status("A", "summary")
        d = s.to_dict()
        assert isinstance(d, dict)
        assert d["agent_name"] == "A"
        assert d["kind"] == "summary"


# ===========================================================================
# 10. RouteSignatureProbe + Manifest
# ===========================================================================
class TestRouteSignatureProbe:
    def test_required_param_count_4(self):
        sub = RouteSignatureProbeSubstrate()
        assert sub.required_param_count == 4

    def test_4_param_signature_valid(self):
        sub = RouteSignatureProbeSubstrate()
        def f(app, adminApiRouter, pluginConfig, projectBasePath): return None
        assert sub.signature_valid(f) is True

    def test_2_param_signature_invalid(self):
        sub = RouteSignatureProbeSubstrate()
        def f(app, pluginConfig): return None
        assert sub.signature_valid(f) is False

    def test_3_param_signature_invalid(self):
        sub = RouteSignatureProbeSubstrate()
        def f(app, adminApiRouter, pluginConfig): return None
        assert sub.signature_valid(f) is False

    def test_5_param_signature_valid(self):
        sub = RouteSignatureProbeSubstrate()
        def f(a, b, c, d, e): return None
        assert sub.signature_valid(f) is True

    def test_all_11_routes(self):
        sub = RouteSignatureProbeSubstrate()
        assert sub.all_routes_registered() is True

    def test_routes_include_generate_endpoints(self):
        sub = RouteSignatureProbeSubstrate()
        routes = list(sub.admin_protected_routes)
        assert any("generate-timelines" in r for r in routes)
        assert any("generate-summaries" in r for r in routes)

    @pytest.fixture
    def live_manifest_substrate(self):
        m = VCPTIMELINE_ROOT / "plugin-manifest.json"
        if not m.exists():
            pytest.skip("plugin-manifest.json not present on this host")
        return VCPTimeLineManifestSubstrate.parse(m)

    def test_manifest_name(self, live_manifest_substrate):
        assert live_manifest_substrate.name == "VCPTimeLine"

    def test_manifest_display_name(self, live_manifest_substrate):
        assert "时间线" in live_manifest_substrate.display_name

    def test_manifest_plugin_type_hybridservice(self, live_manifest_substrate):
        assert live_manifest_substrate.plugin_type == "hybridservice"

    def test_manifest_requires_context_bridge(self, live_manifest_substrate):
        assert live_manifest_substrate.requires_context_bridge is True

    def test_manifest_timeout_300000(self, live_manifest_substrate):
        assert live_manifest_substrate.communication_timeout_ms == 300000

    def test_manifest_no_api_routes(self, live_manifest_substrate):
        assert live_manifest_substrate.has_api_routes is False

    def test_manifest_entry_script(self, live_manifest_substrate):
        assert live_manifest_substrate.entry_point_script == "VCPTimeLine.js"

    def test_manifest_default_parse(self):
        m = VCPTimeLineManifestSubstrate()
        assert m.name == "VCPTimeLine"
        assert m.communication_timeout_ms == 300000


# ===========================================================================
# 11. Bridge
# ===========================================================================
class TestBridge:
    def test_bridge_parent_v1332(self):
        b = VCPTimeLineDeepReadBridge.build()
        assert b.parent_module == "V1332"

    def test_bridge_chain_position_20(self):
        b = VCPTimeLineDeepReadBridge.build()
        assert b.chain_position == 20

    def test_bridge_this_module_v1333(self):
        b = VCPTimeLineDeepReadBridge.build()
        assert b.this_module == "V1333"

    def test_bridge_5_plugins_read(self):
        b = VCPTimeLineDeepReadBridge.build()
        assert len(b.vcp_plugins_deep_read) == 5
        assert "V1333_VCPTimeLine" in b.vcp_plugins_deep_read

    def test_bridge_cumulative_files_21(self):
        b = VCPTimeLineDeepReadBridge.build()
        assert b.cumulative_files == 21

    def test_bridge_cumulative_modules_22(self):
        b = VCPTimeLineDeepReadBridge.build()
        assert b.cumulative_modules == 22


# ===========================================================================
# 12. ASI pole-star integrity
# ===========================================================================
class TestPoleStar:
    def test_v0_1_actual_measured_0_7905(self):
        assert ASI_POLE_STAR["V0_1_actual_measured"] == 0.7905

    def test_v0_2_baseline_0_4467(self):
        assert ASI_POLE_STAR["V0_2_baseline"] == 0.4467

    def test_v0_max_0_9800(self):
        assert ASI_POLE_STAR["V0_max_any_epoch"] == 0.9800

    def test_v1049_value_alignment_done(self):
        assert ASI_POLE_STAR["V1049_value_alignment_done"] is True

    def test_asi_not_achieved(self):
        assert ASI_POLE_STAR["asi_achieved_false"] is True

    def test_v1333_does_not_modify_pole_star(self):
        assert ASI_POLE_STAR["V1333_modifies_pole_star"] is False


# ===========================================================================
# 13. Run-all self-test gate
# ===========================================================================
class TestSelfTestGate:
    def test_self_test_54_checks(self):
        results = _self_test()
        assert isinstance(results, dict)
        assert len(results) == 54

    def test_self_test_all_pass(self):
        results = _self_test()
        failed = [k for k, v in results.items() if not v]
        assert failed == [], f"failed checks: {failed}"


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
