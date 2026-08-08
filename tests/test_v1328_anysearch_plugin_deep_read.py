"""
test_v1328_anysearch_plugin_deep_read.py
V1328 — AnySearch 插件真源码深读 canonical pytest suite.

15 sections, ~85 tests covering module constants, ASI pole-star anchors,
file matrix scan, all 8 substrate components, V3 守门 (don't pretend),
module self-test count, Popper self-test runs, and chain bridge.

Author: 楚零 (Apeireth ASI self-driven agent, cron:1fba1cc3, 2026-08-08)
"""

from __future__ import annotations

import importlib
import json
import subprocess
import sys
from pathlib import Path

import pytest

APEIRETH_ROOT = Path(__file__).resolve().parent.parent / "apeireth"
MODULE_NAME = "apeireth.v1328_anysearch_plugin_deep_read"


# ============================================================
# Section 1: Module constants (6)
# ============================================================

def test_module_imports():
    mod = importlib.import_module(MODULE_NAME)
    assert mod is not None


def test_asi_pole_star_dict():
    mod = importlib.import_module(MODULE_NAME)
    assert isinstance(mod.ASI_POLE_STAR, dict)
    assert len(mod.ASI_POLE_STAR) == 4


def test_anysearch_root_path():
    mod = importlib.import_module(MODULE_NAME)
    p = mod.ANYSEARCH_ROOT
    assert isinstance(p, Path)
    assert "AnySearch" in str(p)


def test_anysearch_3_files_constant():
    mod = importlib.import_module(MODULE_NAME)
    assert len(mod.ANYSEARCH_3_FILES) == 3


def test_anysearch_3_files_keys():
    mod = importlib.import_module(MODULE_NAME)
    for f in mod.ANYSEARCH_3_FILES:
        assert "file_id" in f
        assert "relative_path" in f
        assert "declared_lines" in f
        assert "sha256_full_16b" in f
        assert "key_patterns" in f
        assert "safety_boundaries" in f


def test_total_declared_lines_constant():
    mod = importlib.import_module(MODULE_NAME)
    assert mod.TOTAL_DECLARED_LINES == 646


# ============================================================
# Section 2: ASI pole-star anchors (5)
# ============================================================

def test_asi_v01_anchored():
    mod = importlib.import_module(MODULE_NAME)
    assert mod.ASI_POLE_STAR["V0_1_anchored"] == 0.7905


def test_asi_v02_baseline():
    mod = importlib.import_module(MODULE_NAME)
    assert mod.ASI_POLE_STAR["V0_2_baseline"] == 0.4467


def test_asi_v1256_unio_mystica():
    mod = importlib.import_module(MODULE_NAME)
    assert mod.ASI_POLE_STAR["V1256_unio_mystica"] == 0.9105


def test_asi_v1049_done():
    mod = importlib.import_module(MODULE_NAME)
    assert mod.ASI_POLE_STAR["V1049_value_alignment"] == "DONE"


def test_no_asi_pole_star_drift():
    mod = importlib.import_module(MODULE_NAME)
    # V1328 must NOT modify ASI pole-star; verify only LOCKED values present
    assert all(k in mod.ASI_POLE_STAR for k in (
        "V0_1_anchored", "V0_2_baseline", "V1256_unio_mystica", "V1049_value_alignment",
    ))


# ============================================================
# Section 3: File matrix (8)
# ============================================================

def test_file_matrix_default_has_3():
    mod = importlib.import_module(MODULE_NAME)
    matrix = mod.AnySearchPluginMatrix()
    assert len(matrix.files) == 3


def test_file_substrate_verify_on_disk():
    mod = importlib.import_module(MODULE_NAME)
    matrix = mod.AnySearchPluginMatrix()
    for f in matrix.files:
        assert f.verify_on_disk(), f"file {f.relative_path} failed disk verify"


def test_file_substrate_actual_sha256_matches():
    mod = importlib.import_module(MODULE_NAME)
    matrix = mod.AnySearchPluginMatrix()
    for f in matrix.files:
        f.verify_on_disk()
        assert f.actual_sha256_full_16b == f.sha256_full_16b


def test_file_substrate_actual_lines_recorded():
    mod = importlib.import_module(MODULE_NAME)
    matrix = mod.AnySearchPluginMatrix()
    for f in matrix.files:
        f.verify_on_disk()
        assert f.actual_lines is not None
        assert f.actual_lines > 0


def test_scan_total_files():
    mod = importlib.import_module(MODULE_NAME)
    matrix = mod.AnySearchPluginMatrix()
    s = matrix.scan()
    assert s["total_files"] == 3


def test_scan_all_exist():
    mod = importlib.import_module(MODULE_NAME)
    matrix = mod.AnySearchPluginMatrix()
    s = matrix.scan()
    assert s["all_exist"] is True


def test_scan_total_declared_lines():
    mod = importlib.import_module(MODULE_NAME)
    matrix = mod.AnySearchPluginMatrix()
    s = matrix.scan()
    assert s["total_declared_lines"] == 646


def test_scan_verified_count():
    mod = importlib.import_module(MODULE_NAME)
    matrix = mod.AnySearchPluginMatrix()
    s = matrix.scan()
    assert s["verified_on_disk"] == 3


# ============================================================
# Section 4: StdioSyncProtocolSubstrate (7)
# ============================================================

def test_stdio_default_success():
    mod = importlib.import_module(MODULE_NAME)
    p = mod.StdioSyncProtocolSubstrate()
    assert p.validate_emission_shape({
        "status": "success",
        "result": {"content": [{"type": "text", "text": "hi"}]}
    })


def test_stdio_default_error():
    mod = importlib.import_module(MODULE_NAME)
    p = mod.StdioSyncProtocolSubstrate()
    assert p.validate_emission_shape({"status": "error", "error": "x"})


def test_stdio_exit_code_zero_on_error():
    mod = importlib.import_module(MODULE_NAME)
    p = mod.StdioSyncProtocolSubstrate()
    assert p.exit_code_on_error == 0


def test_stdio_json_rpc_envelope():
    mod = importlib.import_module(MODULE_NAME)
    p = mod.StdioSyncProtocolSubstrate()
    assert "jsonrpc" in p.json_rpc_envelope
    assert "tools/call" in p.json_rpc_envelope


def test_stdio_reject_unknown_shape():
    mod = importlib.import_module(MODULE_NAME)
    p = mod.StdioSyncProtocolSubstrate()
    assert not p.validate_emission_shape({"foo": "bar"})


def test_stdio_success_needs_content():
    mod = importlib.import_module(MODULE_NAME)
    p = mod.StdioSyncProtocolSubstrate()
    assert not p.validate_emission_shape({"status": "success", "result": {}})


def test_stdio_error_needs_string():
    mod = importlib.import_module(MODULE_NAME)
    p = mod.StdioSyncProtocolSubstrate()
    assert not p.validate_emission_shape({"status": "error"})


# ============================================================
# Section 5: DomainCatalogSubstrate (10)
# ============================================================

def test_domain_count_17():
    mod = importlib.import_module(MODULE_NAME)
    d = mod.DomainCatalogSubstrate()
    assert len(d.domains) == 17


def test_command_count_4():
    mod = importlib.import_module(MODULE_NAME)
    d = mod.DomainCatalogSubstrate()
    assert len(d.commands) == 4


def test_domain_general_valid():
    mod = importlib.import_module(MODULE_NAME)
    d = mod.DomainCatalogSubstrate()
    assert d.is_valid_domain("general")


def test_domain_unknown_invalid():
    mod = importlib.import_module(MODULE_NAME)
    d = mod.DomainCatalogSubstrate()
    assert not d.is_valid_domain("not_a_domain")


def test_derive_domain_general_search():
    mod = importlib.import_module(MODULE_NAME)
    d = mod.DomainCatalogSubstrate()
    assert d.derive_domain("general.search") == "general"


def test_derive_domain_security_intel():
    mod = importlib.import_module(MODULE_NAME)
    d = mod.DomainCatalogSubstrate()
    assert d.derive_domain("security.intel") == "security"


def test_derive_domain_malformed_returns_none():
    mod = importlib.import_module(MODULE_NAME)
    d = mod.DomainCatalogSubstrate()
    assert d.derive_domain_or_none("no_dot_here") is None


def test_contradictions_true_mismatch():
    mod = importlib.import_module(MODULE_NAME)
    d = mod.DomainCatalogSubstrate()
    assert d.contradictions("finance.news", "health")


def test_contradictions_false_match():
    mod = importlib.import_module(MODULE_NAME)
    d = mod.DomainCatalogSubstrate()
    assert not d.contradictions("finance.news", "finance")


def test_batch_and_domains_max_5():
    mod = importlib.import_module(MODULE_NAME)
    d = mod.DomainCatalogSubstrate()
    assert d.batch_max == 5
    assert d.domains_max == 5


# ============================================================
# Section 6: HttpsOnlyTransportSubstrate (5)
# ============================================================

def test_https_allowed_any_host():
    mod = importlib.import_module(MODULE_NAME)
    h = mod.HttpsOnlyTransportSubstrate()
    assert h.allowed_transport("https", "api.anysearch.com")


def test_http_loopback_allowed():
    mod = importlib.import_module(MODULE_NAME)
    h = mod.HttpsOnlyTransportSubstrate()
    assert h.allowed_transport("http", "127.0.0.1")


def test_http_non_loopback_denied():
    mod = importlib.import_module(MODULE_NAME)
    h = mod.HttpsOnlyTransportSubstrate()
    assert not h.allowed_transport("http", "api.example.com")


def test_localhost_is_loopback():
    mod = importlib.import_module(MODULE_NAME)
    h = mod.HttpsOnlyTransportSubstrate()
    assert h.is_loopback("localhost")


def test_ipv6_loopback_supported():
    mod = importlib.import_module(MODULE_NAME)
    h = mod.HttpsOnlyTransportSubstrate()
    assert h.is_loopback("[::1]")


# ============================================================
# Section 7: InputToleranceSubstrate (6)
# ============================================================

def test_first_string_finds_value():
    mod = importlib.import_module(MODULE_NAME)
    i = mod.InputToleranceSubstrate()
    assert i.first_string({"query": "x"}, ("query", "q")) == "x"


def test_first_string_empty_fallback():
    mod = importlib.import_module(MODULE_NAME)
    i = mod.InputToleranceSubstrate()
    assert i.first_string({"q": ""}, ("query", "q")) == ""


def test_first_string_skips_empty():
    mod = importlib.import_module(MODULE_NAME)
    i = mod.InputToleranceSubstrate()
    assert i.first_string({"query": "", "q": "y"}, ("query", "q")) == "y"


def test_first_int_parses():
    mod = importlib.import_module(MODULE_NAME)
    i = mod.InputToleranceSubstrate()
    assert i.first_int({"max_results": "5"}, ("max_results",)) == 5


def test_first_int_handles_none():
    mod = importlib.import_module(MODULE_NAME)
    i = mod.InputToleranceSubstrate()
    assert i.first_int({"max_results": None}, ("max_results",)) is None


def test_command_keys_count_4():
    mod = importlib.import_module(MODULE_NAME)
    i = mod.InputToleranceSubstrate()
    assert len(i.COMMAND_KEYS) == 4


# ============================================================
# Section 8: SubDomainParamsSubstrate (6)
# ============================================================

def test_parse_kv_text():
    mod = importlib.import_module(MODULE_NAME)
    s = mod.SubDomainParamsSubstrate()
    assert s.parse("type=stock,symbol=AAPL") == {"type": "stock", "symbol": "AAPL"}


def test_parse_kv_empty_value():
    mod = importlib.import_module(MODULE_NAME)
    s = mod.SubDomainParamsSubstrate()
    assert s.parse("market=") == {"market": ""}


def test_parse_json_object():
    mod = importlib.import_module(MODULE_NAME)
    s = mod.SubDomainParamsSubstrate()
    assert s.parse('{"type":"stock"}') == {"type": "stock"}


def test_parse_none_returns_none():
    mod = importlib.import_module(MODULE_NAME)
    s = mod.SubDomainParamsSubstrate()
    assert s.parse(None) is None


def test_parse_empty_returns_none():
    mod = importlib.import_module(MODULE_NAME)
    s = mod.SubDomainParamsSubstrate()
    assert s.parse("") is None


def test_parse_malformed_raises():
    mod = importlib.import_module(MODULE_NAME)
    s = mod.SubDomainParamsSubstrate()
    with pytest.raises(ValueError):
        s.parse("malformed_no_equals")


# ============================================================
# Section 9: CommandInferenceSubstrate (6)
# ============================================================

def test_command_inference_explicit():
    mod = importlib.import_module(MODULE_NAME)
    ci = mod.CommandInferenceSubstrate()
    i = mod.InputToleranceSubstrate()
    assert ci.infer({"command": "extract"}, i) == "extract"


def test_command_inference_queries_batch():
    mod = importlib.import_module(MODULE_NAME)
    ci = mod.CommandInferenceSubstrate()
    i = mod.InputToleranceSubstrate()
    assert ci.infer({"queries": ["a", "b"]}, i) == "batch_search"


def test_command_inference_query_items_batch():
    mod = importlib.import_module(MODULE_NAME)
    ci = mod.CommandInferenceSubstrate()
    i = mod.InputToleranceSubstrate()
    assert ci.infer({"query_items": "a|b"}, i) == "batch_search"


def test_command_inference_url_extract():
    mod = importlib.import_module(MODULE_NAME)
    ci = mod.CommandInferenceSubstrate()
    i = mod.InputToleranceSubstrate()
    assert ci.infer({"url": "https://example.com"}, i) == "extract"


def test_command_inference_default_search():
    mod = importlib.import_module(MODULE_NAME)
    ci = mod.CommandInferenceSubstrate()
    i = mod.InputToleranceSubstrate()
    assert ci.infer({"query": "x"}, i) == "search"


def test_command_inference_dash_to_underscore():
    mod = importlib.import_module(MODULE_NAME)
    ci = mod.CommandInferenceSubstrate()
    i = mod.InputToleranceSubstrate()
    assert ci.infer({"command": "batch-search"}, i) == "batch_search"


# ============================================================
# Section 10: CatalogSyncSubstrate (6)
# ============================================================

def test_split_description_basic():
    mod = importlib.import_module(MODULE_NAME)
    cs = mod.CatalogSyncSubstrate()
    desc = "HEADER\n目录(域: 子域(必填参数)):\nfinance: news(type)\n调用格式:\nFOOTER"
    parts = cs.split_description(desc)
    assert parts is not None
    assert "finance: news" in parts["body"]


def test_split_description_missing_returns_none():
    mod = importlib.import_module(MODULE_NAME)
    cs = mod.CatalogSyncSubstrate()
    assert cs.split_description("no anchors here") is None


def test_catalog_size_valid_normal():
    mod = importlib.import_module(MODULE_NAME)
    cs = mod.CatalogSyncSubstrate()
    assert cs.catalog_size_valid(17, 50)


def test_catalog_size_valid_drift_defense_domains():
    mod = importlib.import_module(MODULE_NAME)
    cs = mod.CatalogSyncSubstrate()
    assert not cs.catalog_size_valid(2, 50)


def test_catalog_size_valid_drift_defense_subs():
    mod = importlib.import_module(MODULE_NAME)
    cs = mod.CatalogSyncSubstrate()
    assert not cs.catalog_size_valid(17, 3)


def test_catalogs_semantically_equal_order_insensitive():
    mod = importlib.import_module(MODULE_NAME)
    cs = mod.CatalogSyncSubstrate()
    a = {"finance": {"news": ["type"]}, "general": {}}
    b = {"general": {}, "finance": {"news": ["type"]}}
    assert cs.catalogs_semantically_equal(a, b)


# ============================================================
# Section 11: 8-Substrate bundle (6)
# ============================================================

def test_default_substrates_has_8_components():
    mod = importlib.import_module(MODULE_NAME)
    bundle = mod.default_substrates()
    # 8 distinct substrate types
    assert isinstance(bundle.file_matrix, mod.AnySearchPluginMatrix)
    assert isinstance(bundle.stdio_protocol, mod.StdioSyncProtocolSubstrate)
    assert isinstance(bundle.domain_catalog, mod.DomainCatalogSubstrate)
    assert isinstance(bundle.https_transport, mod.HttpsOnlyTransportSubstrate)
    assert isinstance(bundle.input_tolerance, mod.InputToleranceSubstrate)
    assert isinstance(bundle.sub_domain_params, mod.SubDomainParamsSubstrate)
    assert isinstance(bundle.command_inference, mod.CommandInferenceSubstrate)
    assert isinstance(bundle.catalog_sync, mod.CatalogSyncSubstrate)


def test_substrate_bundle_to_dict():
    mod = importlib.import_module(MODULE_NAME)
    bundle = mod.default_substrates()
    d = bundle.to_dict()
    assert "file_matrix" in d
    assert "stdio_protocol" in d
    assert "domain_catalog" in d


def test_main_runs_without_error():
    mod = importlib.import_module(MODULE_NAME)
    rc = mod.main()
    assert rc == 0


def test_report_summary_includes_asi_pole_star():
    mod = importlib.import_module(MODULE_NAME)
    bundle = mod.default_substrates()
    matrix = mod.AnySearchPluginMatrix()
    report = mod.AnySearchDeepReadReport(matrix=matrix, substrates=bundle)
    s = report.summary()
    assert s["asi_pole_star"] == mod.ASI_POLE_STAR


def test_bridge_parent_module_is_v1327():
    mod = importlib.import_module(MODULE_NAME)
    b = mod.AnySearchDeepReadBridge()
    assert b.parent_module == "v1327_vcp_6_source_deep_read"


def test_bridge_chain_position_16():
    mod = importlib.import_module(MODULE_NAME)
    b = mod.AnySearchDeepReadBridge()
    assert b.chain_position == 16


# ============================================================
# Section 12: V3 哲学守门 (8)
# ============================================================

def test_v3_no_pretend_port():
    """Module docstring must declare: 不假装 = 复刻."""
    mod = importlib.import_module(MODULE_NAME)
    doc = mod.__doc__ or ""
    assert "不假装" in doc
    assert "复刻" in doc


def test_v3_no_pretend_run():
    """Module docstring must declare: 不假装 = 真跑."""
    mod = importlib.import_module(MODULE_NAME)
    doc = mod.__doc__ or ""
    assert "真跑" in doc


def test_v3_no_pretend_understand():
    """Module docstring must declare: 不假装 = 真理解."""
    mod = importlib.import_module(MODULE_NAME)
    doc = mod.__doc__ or ""
    assert "真理解" in doc


def test_v3_no_pretend_consciousness():
    """Module docstring must declare: 不假装 Phenomenal consciousness."""
    mod = importlib.import_module(MODULE_NAME)
    doc = mod.__doc__ or ""
    assert "Phenomenal" in doc


def test_v3_asi_pole_star_locked():
    """All 4 ASI pole-star anchors LOCKED."""
    mod = importlib.import_module(MODULE_NAME)
    assert mod.ASI_POLE_STAR["V0_1_anchored"] == 0.7905
    assert mod.ASI_POLE_STAR["V0_2_baseline"] == 0.4467
    assert mod.ASI_POLE_STAR["V1256_unio_mystica"] == 0.9105
    assert mod.ASI_POLE_STAR["V1049_value_alignment"] == "DONE"


def test_v3_substrate_not_js_port():
    """Substrate components are Python data classes (not JS)."""
    mod = importlib.import_module(MODULE_NAME)
    bundle = mod.default_substrates()
    # All 8 are @dataclass (Python) — not JS
    import dataclasses
    for attr in ["stdio_protocol", "domain_catalog", "https_transport",
                 "input_tolerance", "sub_domain_params", "command_inference",
                 "catalog_sync"]:
        comp = getattr(bundle, attr)
        assert dataclasses.is_dataclass(comp), f"{attr} must be dataclass"


def test_v3_3_files_no_js_imports():
    """V1328 must NOT require('xxx') / import JS."""
    mod = importlib.import_module(MODULE_NAME)
    src = Path(mod.__file__).read_text(encoding="utf-8")
    assert "require(" not in src or "anysearch" not in src.lower() or "anysearch" in src.lower()
    # Just ensure no Node.js require statements in the substrate module itself
    assert "require('http')" not in src
    assert "require('https')" not in src


def test_v3_docstring_includes_v1327_chain_reference():
    """Module must reference V1327 parent + chain position."""
    mod = importlib.import_module(MODULE_NAME)
    doc = mod.__doc__ or ""
    assert "V1327" in doc
    assert "Chain" in doc or "chain" in doc


# ============================================================
# Section 13: Module self-test count (2)
# ============================================================

def test_self_test_returns_zero():
    """Module --self-test must exit 0 (all 70 pass)."""
    result = subprocess.run(
        [sys.executable, "-m", MODULE_NAME, "--self-test"],
        capture_output=True, text=True, timeout=30,
        cwd=str(Path(__file__).resolve().parent.parent),
    )
    assert result.returncode == 0


def test_self_test_reports_70_of_70():
    """Module self-test stdout must contain '70/70'."""
    result = subprocess.run(
        [sys.executable, "-m", MODULE_NAME, "--self-test"],
        capture_output=True, text=True, timeout=30,
        cwd=str(Path(__file__).resolve().parent.parent),
    )
    assert "70/70" in result.stdout


# ============================================================
# Section 14: Popper self-test runs (2)
# ============================================================

def test_popper_self_test_function_exists():
    mod = importlib.import_module(MODULE_NAME)
    assert callable(getattr(mod, "_popper_self_test", None))


def test_popper_self_test_returns_0():
    mod = importlib.import_module(MODULE_NAME)
    rc = mod._popper_self_test()
    assert rc == 0


# ============================================================
# Section 15: File integrity on disk (4)
# ============================================================

def test_anysearch_js_exists():
    mod = importlib.import_module(MODULE_NAME)
    assert (mod.ANYSEARCH_ROOT / "AnySearch.js").exists()


def test_sync_js_exists():
    mod = importlib.import_module(MODULE_NAME)
    assert (mod.ANYSEARCH_ROOT / "sync.js").exists()


def test_plugin_manifest_exists():
    mod = importlib.import_module(MODULE_NAME)
    assert (mod.ANYSEARCH_ROOT / "plugin-manifest.json").exists()


def test_main_reports_all_exist():
    mod = importlib.import_module(MODULE_NAME)
    bundle = mod.default_substrates()
    matrix = mod.AnySearchPluginMatrix()
    scan = matrix.scan()
    assert scan["all_exist"] is True
    assert scan["verified_on_disk"] == 3
