#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
test_v1329_dailynote_plugin_deep_read.py — Pytest 验证 V1329 DailyNote Plugin Real Source Code Deep Read

- 15 sections, ≥ 87 canonical tests
- Mirrors V1328 test pattern but for DailyNote plugin (post-V1328 chain step)
- Each section has explicit assertions on substrate behavior + V3 守门 + ASI pole-star LOCKED

Sections:
 1. Module constants (6)
 2. ASI pole-star anchors (5)
 3. File matrix (8)
 4. PathSanitizationSubstrate (12 — sampled from 9-step + reserved + truncate)
 5. PathTraversalSubstrate (5)
 6. FolderResolutionSubstrate (10)
 7. FolderPrivacySubstrate (6)
 8. CommandSubstrate (7)
 9. TagStrategySubstrate (8)
10. FuzzyDiffSubstrate (5)
11. TagMasterAISubstrate (5)
12. FolderAliasNormalizationSubstrate (4)
13. Aggregator + Bridge (5)
14. V3 哲学守门 (8)
15. Popper self-test + module entry point (3)
"""
import sys
from pathlib import Path

import pytest

# Add apeireth root to path so the module can be imported
APEIRETH_ROOT = Path(__file__).resolve().parent.parent
if str(APEIRETH_ROOT) not in sys.path:
    sys.path.insert(0, str(APEIRETH_ROOT))

import v1329_dailynote_plugin_deep_read as m  # noqa: E402


# ============================================================================
# 1. Module constants (6)
# ============================================================================

def test_module_constants_present():
    assert hasattr(m, "ASI_POLE_STAR")
    assert hasattr(m, "DAILYNOTE_ROOT")
    assert hasattr(m, "DAILYNOTE_4_FILES")
    assert hasattr(m, "_popper_self_test")
    assert hasattr(m, "DailyNoteFileSubstrate")
    assert hasattr(m, "DailyNotePluginMatrix")


def test_dailynote_root_is_path():
    assert isinstance(m.DAILYNOTE_ROOT, Path)
    assert "DailyNote" in str(m.DAILYNOTE_ROOT)


def test_dailynote_4_files_count():
    assert len(m.DAILYNOTE_4_FILES) == 4


def test_dailynote_files_have_required_keys():
    for f in m.DAILYNOTE_4_FILES:
        assert "file_id" in f
        assert "filename" in f
        assert "declared_lines" in f
        assert "role" in f
        assert "expected_sha256_first16" in f


def test_module_version_constant():
    # Module version embedded in docstring
    assert "0.1.0" in m.__doc__


def test_module_author_in_docstring():
    assert "楚零" in m.__doc__ or "Chu Ling" in m.__doc__


# ============================================================================
# 2. ASI pole-star anchors (5)
# ============================================================================

def test_pole_star_v01_unchanged():
    assert m.ASI_POLE_STAR["V0_1_actual_measured"] == 0.7905


def test_pole_star_v02_unchanged():
    assert m.ASI_POLE_STAR["V0_2_baseline"] == 0.4467


def test_pole_star_v1256_unchanged():
    assert m.ASI_POLE_STAR["V1256_unio_mystica_realized"] == 0.9105


def test_v1329_does_not_modify_pole_star():
    assert m.ASI_POLE_STAR["V1329_modifies_pole_star"] is False


def test_asi_not_achieved_flag():
    # V1329 explicitly marks ASI not achieved (per 主 17:58 + 20:46 不假装 ASI)
    assert m.ASI_POLE_STAR["asi_achieved_false"] is True


# ============================================================================
# 3. File matrix (8)
# ============================================================================

def test_matrix_scans_4_files():
    matrix = m.DailyNotePluginMatrix.scan(m.DAILYNOTE_ROOT, m.DAILYNOTE_4_FILES)
    assert len(matrix.files) == 4


def test_matrix_all_files_exist():
    matrix = m.DailyNotePluginMatrix.scan(m.DAILYNOTE_ROOT, m.DAILYNOTE_4_FILES)
    assert matrix.all_exist is True


def test_matrix_all_integrity_ok():
    matrix = m.DailyNotePluginMatrix.scan(m.DAILYNOTE_ROOT, m.DAILYNOTE_4_FILES)
    assert matrix.all_integrity_ok is True


def test_matrix_total_lines_1665():
    matrix = m.DailyNotePluginMatrix.scan(m.DAILYNOTE_ROOT, m.DAILYNOTE_4_FILES)
    assert matrix.total_actual_lines == 1665


def test_matrix_sha256_match_count_4():
    matrix = m.DailyNotePluginMatrix.scan(m.DAILYNOTE_ROOT, m.DAILYNOTE_4_FILES)
    assert matrix.sha256_match_count == 4


def test_matrix_f1_dailynote_js():
    matrix = m.DailyNotePluginMatrix.scan(m.DAILYNOTE_ROOT, m.DAILYNOTE_4_FILES)
    f1 = matrix.files[0]
    assert f1.filename == "dailynote.js"
    assert f1.actual_lines == 1533
    assert f1.sha256_first16 == "4eee260c13965283"


def test_matrix_f2_manifest():
    matrix = m.DailyNotePluginMatrix.scan(m.DAILYNOTE_ROOT, m.DAILYNOTE_4_FILES)
    f2 = matrix.files[1]
    assert f2.filename == "plugin-manifest.json"
    assert f2.actual_lines == 96
    assert f2.sha256_first16 == "a3d73021cc4b3c1e"


def test_matrix_summary_dict():
    matrix = m.DailyNotePluginMatrix.scan(m.DAILYNOTE_ROOT, m.DAILYNOTE_4_FILES)
    summary = matrix.summary()
    assert summary["files_count"] == 4
    assert summary["all_exist"] is True
    assert summary["all_integrity_ok"] is True
    assert summary["total_actual_lines"] == 1665


# ============================================================================
# 4. PathSanitizationSubstrate (12)
# ============================================================================

def test_sanitize_step1_strip_separators():
    s = m.PathSanitizationSubstrate.sanitize("hello/world:test*file")
    assert "/" not in s.sanitized
    assert ":" not in s.sanitized
    assert "*" not in s.sanitized
    assert "1_separators_stripped" in s.steps_applied


def test_sanitize_step2_strip_control_chars():
    s = m.PathSanitizationSubstrate.sanitize("hello\x00\x01\x1fworld")
    assert "\x00" not in s.sanitized
    assert "\x01" not in s.sanitized
    assert "2_ctrl_stripped" in s.steps_applied


def test_sanitize_step3_strip_directional():
    s = m.PathSanitizationSubstrate.sanitize("hello\u200e\u200fworld")
    assert "\u200e" not in s.sanitized
    assert "\u200f" not in s.sanitized
    assert "3_directional_stripped" in s.steps_applied


def test_sanitize_step4_strip_zerowidth():
    s = m.PathSanitizationSubstrate.sanitize("hello\u200b\u200dworld")
    assert "\u200b" not in s.sanitized
    assert "4_zerowidth_stripped" in s.steps_applied


def test_sanitize_step5_whitespace_to_underscore():
    s = m.PathSanitizationSubstrate.sanitize("hello world test")
    assert " " not in s.sanitized
    assert "5_whitespace_to_underscore" in s.steps_applied


def test_sanitize_step6_strip_edge_dots():
    s = m.PathSanitizationSubstrate.sanitize("..hello..")
    assert not s.sanitized.startswith(".")
    assert not s.sanitized.endswith(".")
    assert "6_edge_dots_stripped" in s.steps_applied


def test_sanitize_step7_collapse_underscore():
    s = m.PathSanitizationSubstrate.sanitize("hello___world")
    assert "___" not in s.sanitized
    assert "7_underscore_collapsed" in s.steps_applied


def test_sanitize_step8_reserved_renamed():
    s = m.PathSanitizationSubstrate.sanitize("CON")
    assert s.reserved_renamed is True
    assert s.sanitized.startswith("_")
    # Other reserved names
    s2 = m.PathSanitizationSubstrate.sanitize("com1")
    assert s2.reserved_renamed is True


def test_sanitize_step9_truncate_long():
    s = m.PathSanitizationSubstrate.sanitize("a" * 200)
    assert s.truncated is True
    assert len(s.sanitized) <= m.PathSanitizationSubstrate.MAX_FOLDER_NAME_LENGTH


def test_sanitize_fallback_empty():
    s = m.PathSanitizationSubstrate.sanitize("")
    assert s.sanitized == "Untitled"


def test_sanitize_fallback_none():
    s = m.PathSanitizationSubstrate.sanitize(None)
    assert s.sanitized == "Untitled"


def test_sanitize_9_steps_constants_exist():
    # Verify all 9 patterns are defined
    cls = m.PathSanitizationSubstrate
    assert hasattr(cls, "PATH_SEPAR_PATTERN")
    assert hasattr(cls, "CTRL_PATTERN")
    assert hasattr(cls, "DIRECTIONAL_PATTERN")
    assert hasattr(cls, "ZEROWIDTH_PATTERN")
    assert hasattr(cls, "WHITESPACE_PATTERN")
    assert hasattr(cls, "EDGE_DOTS_PATTERN")
    assert hasattr(cls, "COLLAPSE_UNDERSCORE_PATTERN")
    assert hasattr(cls, "WINDOWS_RESERVED_PATTERN")
    assert hasattr(cls, "MAX_FOLDER_NAME_LENGTH")


# ============================================================================
# 5. PathTraversalSubstrate (5)
# ============================================================================

def test_traversal_within_ok():
    pt = m.PathTraversalSubstrate.is_path_within_base("/base/sub/file", "/base")
    assert pt.is_within is True


def test_traversal_reject_similar_prefix():
    # /base_other should NOT be within /base (sep-suffix defense)
    pt = m.PathTraversalSubstrate.is_path_within_base("/base_other_evil/file", "/base")
    assert pt.is_within is False


def test_traversal_exact_match():
    pt = m.PathTraversalSubstrate.is_path_within_base("/base", "/base")
    assert pt.is_within is True


def test_traversal_sep_suffix_defense():
    # /basefoo must NOT match /base
    pt = m.PathTraversalSubstrate.is_path_within_base("/basefoo", "/base")
    assert pt.is_within is False


def test_traversal_deep_within():
    pt = m.PathTraversalSubstrate.is_path_within_base("/base/sub/sub2/file.txt", "/base")
    assert pt.is_within is True


# ============================================================================
# 6. FolderResolutionSubstrate (10)
# ============================================================================

def test_resolve_exact_match():
    fr = m.FolderResolutionSubstrate.resolve("小克", ["小克", "公共", "小明的日记本"])
    assert fr.best_match == "小克"
    assert fr.best_score == 100000 + len("小克")


def test_resolve_contains_existing_in_requested():
    # requested='小克的日记', existing='小克' → 40000+len(requested)
    fr = m.FolderResolutionSubstrate.resolve("小克的日记", ["小克", "小明的日记"])
    assert fr.best_match == "小克"


def test_resolve_contains_requested_in_existing():
    # requested='小克', existing='小克的日记' → 50000+len(existing)
    fr = m.FolderResolutionSubstrate.resolve("小克", ["小克的日记", "小明的日记"])
    assert fr.best_match == "小克的日记"
    assert fr.best_score == 50000 + len("小克的日记")


def test_resolve_strip_noise_word():
    fr = m.FolderResolutionSubstrate.resolve("小克日记本", ["小克"])
    assert "日记本" not in fr.normalized_alias


def test_resolve_empty_input():
    fr = m.FolderResolutionSubstrate.resolve("", ["小克"])
    assert fr.normalized_alias == ""


def test_resolve_no_match():
    fr = m.FolderResolutionSubstrate.resolve("完全不匹配XYZ", ["小克", "小明"])
    assert fr.best_match is None
    assert fr.best_score == 0


def test_match_score_exact():
    score = m.FolderResolutionSubstrate.match_score("小克", "小克")
    assert score == 100000 + len("小克")


def test_match_score_40000_existing_in_requested():
    # existing in requested → 40000 + len(requested)
    score = m.FolderResolutionSubstrate.match_score("小克的日记", "小克")
    assert score == 40000 + len("小克的日记")


def test_match_score_50000_requested_in_existing():
    # requested in existing → 50000 + len(existing)
    score = m.FolderResolutionSubstrate.match_score("小克", "小克的日记")
    assert score == 50000 + len("小克的日记")


def test_match_score_zero_no_overlap():
    score = m.FolderResolutionSubstrate.match_score("ABC", "XYZ")
    assert score == 0


# ============================================================================
# 7. FolderPrivacySubstrate (6)
# ============================================================================

def test_privacy_public_ownerless_ok():
    fp = m.FolderPrivacySubstrate.allowed("公共", "公共的日记", "")
    assert fp.owner_match_ok is True
    assert fp.requested_is_public is True
    assert fp.existing_is_public is True


def test_privacy_public_to_private_reject():
    # public → private should reject
    fp = m.FolderPrivacySubstrate.allowed("公共", "小克的日记", "小克")
    assert fp.owner_match_ok is False


def test_privacy_private_to_owner_ok():
    fp = m.FolderPrivacySubstrate.allowed("小克", "小克的日记", "小克")
    assert fp.owner_match_ok is True


def test_privacy_private_to_other_reject():
    fp = m.FolderPrivacySubstrate.allowed("小克", "小明的日记", "小克")
    assert fp.owner_match_ok is False


def test_privacy_public_prefix_ok():
    fp = m.FolderPrivacySubstrate.allowed("公共", "公共_全员", "")
    assert fp.owner_match_ok is True


def test_privacy_ownerless_default_ok():
    fp = m.FolderPrivacySubstrate.allowed("小克", "小克的日记", "")
    assert fp.owner_match_ok is True


# ============================================================================
# 8. CommandSubstrate (7)
# ============================================================================

def test_cmd_create_required_present():
    cmd = m.CommandSubstrate.analyze("create", {"maid": "x", "Date": "2026-08-08", "Content": "hi"})
    assert cmd.required_present is True
    assert "maid" in cmd.required_params
    assert "Date" in cmd.required_params
    assert "Content" in cmd.required_params


def test_cmd_create_no_optional():
    cmd = m.CommandSubstrate.analyze("create", {"maid": "x", "Date": "2026-08-08", "Content": "hi"})
    assert cmd.optional_present is False


def test_cmd_create_with_optional_tag():
    cmd = m.CommandSubstrate.analyze("create", {"maid": "x", "Date": "2026-08-08", "Content": "hi", "Tag": "x, y"})
    assert cmd.optional_present is True
    assert "Tag" in cmd.optional_params


def test_cmd_update_required_present():
    cmd = m.CommandSubstrate.analyze("update", {"target": "old content here!!!", "replace": "new"})
    assert cmd.required_present is True
    assert "target" in cmd.required_params
    assert "replace" in cmd.required_params


def test_cmd_create_missing_required():
    cmd = m.CommandSubstrate.analyze("create", {"maid": "x"})
    assert cmd.required_present is False


def test_cmd_unknown_no_required_schema():
    cmd = m.CommandSubstrate.analyze("delete", {})
    assert not cmd.required_params  # unknown command has no schema
    assert cmd.command_present is True


def test_cmd_empty_command():
    cmd = m.CommandSubstrate.analyze("", {})
    assert cmd.command_present is False


# ============================================================================
# 9. TagStrategySubstrate (8)
# ============================================================================

def test_tag_detect_inline():
    ts = m.TagStrategySubstrate.resolve("Hello world\nTag: x, y, z", None)
    assert ts.detected_tag_line is not None


def test_tag_fix_format():
    ts = m.TagStrategySubstrate.resolve("Hello world\nTag: x, y, z", None)
    assert ts.fixed_tag_line == "Tag: x, y, z"


def test_tag_count_3():
    ts = m.TagStrategySubstrate.resolve("Hello world\nTag: x, y, z", None)
    assert ts.tag_count == 3


def test_tag_no_override():
    ts = m.TagStrategySubstrate.resolve("Hello\nTag: a, b", None)
    assert ts.override_used is False


def test_tag_override_used():
    ts = m.TagStrategySubstrate.resolve("Hello\nTag: a, b", "c, d, e, f")
    assert ts.override_used is True
    assert ts.tag_count == 4


def test_tag_no_tag_in_content():
    ts = m.TagStrategySubstrate.resolve("Hello world", None)
    assert ts.final_tag is None
    assert ts.tag_count == 0


def test_tag_detect_lowercase():
    ts = m.TagStrategySubstrate.resolve("Hello\ntag: Lower, Case", None)
    assert ts.detected_tag_line is not None


def test_tag_fix_comma_format():
    ts = m.TagStrategySubstrate.resolve("x\nTag: a,b,c", None)
    # fix_tag_format normalizes ", " spacing
    assert ts.fixed_tag_line is not None
    assert ", " in ts.fixed_tag_line


# ============================================================================
# 10. FuzzyDiffSubstrate (5)
# ============================================================================

def test_fuzzy_diff_enabled():
    fd = m.FuzzyDiffSubstrate.from_config(True)
    assert fd.enabled is True


def test_fuzzy_diff_disabled():
    fd = m.FuzzyDiffSubstrate.from_config(False)
    assert fd.enabled is False


def test_fuzzy_diff_target_ok():
    fd = m.FuzzyDiffSubstrate.from_config(True)
    ok, msg = fd.validate_target("this is a long enough target string")
    assert ok is True
    assert msg == "ok"


def test_fuzzy_diff_target_too_short():
    fd = m.FuzzyDiffSubstrate.from_config(True)
    ok, msg = fd.validate_target("short")
    assert ok is False
    assert "too_short" in msg


def test_fuzzy_diff_min_length_15():
    fd = m.FuzzyDiffSubstrate.from_config(True)
    assert fd.min_target_length == 15


# ============================================================================
# 11. TagMasterAISubstrate (5)
# ============================================================================

def test_tag_master_enabled():
    env = {"TagMaster": "true", "TagModel": "claude-4-8-opus",
           "TagModelPrompt": "TagMaster.txt",
           "TagModelMaxOutPutTokens": "30000",
           "TagModelMaxTokens": "40000"}
    tm = m.TagMasterAISubstrate.from_config(env)
    assert tm.enabled is True


def test_tag_master_disabled_default():
    tm = m.TagMasterAISubstrate.from_config({"TagMaster": "false"})
    assert tm.enabled is False


def test_tag_master_model_default():
    tm = m.TagMasterAISubstrate.from_config({"TagMaster": "false"})
    assert tm.model == "claude-4-8-opus"


def test_tag_master_extract_strict_format():
    extracted = m.TagMasterAISubstrate.extract_tag_from_ai_response(
        "Some preamble [[Tag: VCP, 日记, 提示词]] end"
    )
    assert extracted is not None
    assert "VCP" in extracted
    assert "日记" in extracted


def test_tag_master_extract_no_match():
    extracted = m.TagMasterAISubstrate.extract_tag_from_ai_response("no tags here")
    assert extracted is None


# ============================================================================
# 12. FolderAliasNormalizationSubstrate (4)
# ============================================================================

def test_alias_normalize_clean():
    fan = m.FolderAliasNormalizationSubstrate.normalize("小克的日记")
    assert fan.normalized == "小克的日记"


def test_alias_normalize_noise_stripped():
    fan = m.FolderAliasNormalizationSubstrate.normalize("小克的日记本")
    assert fan.noise_stripped is True
    assert "日记本" not in fan.normalized


def test_alias_normalize_empty_after():
    fan = m.FolderAliasNormalizationSubstrate.normalize("")
    assert fan.empty_after is True


def test_alias_normalize_strip_separator():
    fan = m.FolderAliasNormalizationSubstrate.normalize("小克/日记")
    assert "/" not in fan.normalized


# ============================================================================
# 13. Aggregator + Bridge (5)
# ============================================================================

def test_report_substrates_count_10():
    matrix = m.DailyNotePluginMatrix.scan(m.DAILYNOTE_ROOT, m.DAILYNOTE_4_FILES)
    report = m.DailyNoteDeepReadReport.build(matrix)
    assert report.substrates_count == 10


def test_report_safety_components_3():
    matrix = m.DailyNotePluginMatrix.scan(m.DAILYNOTE_ROOT, m.DAILYNOTE_4_FILES)
    report = m.DailyNoteDeepReadReport.build(matrix)
    assert report.safety_components == 3


def test_bridge_chain_position_17():
    bridge = m.DailyNoteDeepReadBridge()
    assert bridge.chain_position == 17


def test_bridge_summary_pole_locked():
    bridge = m.DailyNoteDeepReadBridge()
    summary = bridge.chain_summary()
    assert summary["pole_star_locked"] is True
    assert summary["v3_guards_locked"] is True


def test_bridge_files_cumulative_7():
    bridge = m.DailyNoteDeepReadBridge()
    summary = bridge.chain_summary()
    # 3 (V1328 AnySearch) + 4 (V1329 DailyNote) = 7
    assert summary["files_cumulative"] == 7


# ============================================================================
# 14. V3 哲学守门 (8)
# ============================================================================

def test_v3_no_pole_star_modify():
    assert m.ASI_POLE_STAR["V1329_modifies_pole_star"] is False


def test_v3_real_disk_read_not_scraped():
    # All 4 files have actual content > 0 (proving disk read)
    matrix = m.DailyNotePluginMatrix.scan(m.DAILYNOTE_ROOT, m.DAILYNOTE_4_FILES)
    for f in matrix.files:
        assert f.actual_lines > 0
        assert f.sha256_full != ""


def test_v3_substrate_only_not_port():
    # V1329 = Python substrate extraction, NOT JS port
    # We can verify by checking no JS execution path
    assert not any(name.startswith("exec_") or name.startswith("run_js")
                   for name in dir(m))


def test_v3_no_real_tool_execution():
    # Substrates do NOT call real tools
    # Check no subprocess / os.system calls in module source
    src = Path(m.__file__).read_text(encoding="utf-8")
    assert "subprocess.run" not in src
    assert "os.system" not in src
    assert "child_process" not in src


def test_v3_no_phenomenal_claim():
    # V1329 does NOT claim phenomenal consciousness
    assert "Phenomenal" not in m.ASI_POLE_STAR
    assert "consciousness" not in str(m.ASI_POLE_STAR).lower() or "Phenomenal" not in str(m.ASI_POLE_STAR)


def test_v3_asi_not_achieved():
    # Per 主 17:58: 不假装 ASI 已有 consciousness
    assert m.ASI_POLE_STAR["asi_achieved_false"] is True


def test_v3_chain_continuation_v1328_to_v1329():
    bridge = m.DailyNoteDeepReadBridge()
    assert bridge.parent_module == "v1328_anysearch_plugin_deep_read"
    assert bridge.this_module == "v1329_dailynote_plugin_deep_read"


def test_v3_vcp_plugin_chain_semantics():
    # V1329 explicitly chains from V1328 (VCP plugin deep-read)
    assert "plugin" in m.__doc__.lower() or "Plugin" in m.__doc__


# ============================================================================
# 15. Popper self-test + module entry point (3)
# ============================================================================

def test_popper_self_test_runs():
    delta = m._popper_self_test()
    # delta = passed - total; 0 means all pass
    assert delta == 0


def test_module_main_no_args():
    # main() with no args prints usage and returns 0
    rc = m.main([])
    assert rc == 0


def test_module_main_report():
    rc = m.main(["--report"])
    assert rc == 0


if __name__ == "__main__":
    sys.exit(pytest.main([__file__, "-v"]))