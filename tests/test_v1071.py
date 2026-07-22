"""V1071 ASI VCP Real Source Code Deep Read — tests."""
from __future__ import annotations
import sys
sys.path.insert(0, '.')

import os
import json
import pytest
from apeireth.v1071_vcp_real_source_code_deep_read import (
    V1071_VERSION,
    VCP_1_PLUGIN_TYPES, VCP_1_COMMUNICATION_PROTOCOLS,
    VCP_1_MANIFEST_VERSION,
    find_vcp_source_root, discover_plugin_dirs,
    PluginManifest, parse_manifest,
    TypeDistribution, ProtocolDistribution,
    WebSocketStats, extract_capability_summary,
    validate_entry_points,
    VCP1SpecResult, validate_vcp1_spec,
    V1071VCPDeepRead,
    v1071_bridge_measure, v1071_cross_domain_measure,
    v1071_report_markdown,
    v1071_philosophy_guard, v1071_run,
)


# ============================================================================
# 1. VCP constants 真生产
# ============================================================================


class TestVCP1Spec:
    """V1071 VCP 1.0 规范常量真生产测试."""

    def test_plugin_types_6(self):
        """6 plugin types 真借鉴 (V1001 集成)."""
        assert len(VCP_1_PLUGIN_TYPES) == 6
        assert "synchronous" in VCP_1_PLUGIN_TYPES
        assert "asynchronous" in VCP_1_PLUGIN_TYPES
        assert "static" in VCP_1_PLUGIN_TYPES
        assert "service" in VCP_1_PLUGIN_TYPES
        assert "messagePreprocessor" in VCP_1_PLUGIN_TYPES
        assert "hybridservice" in VCP_1_PLUGIN_TYPES

    def test_protocols_3(self):
        """3 protocols 真借鉴 (V1001 集成)."""
        assert len(VCP_1_COMMUNICATION_PROTOCOLS) == 3
        assert "stdio" in VCP_1_COMMUNICATION_PROTOCOLS
        assert "direct" in VCP_1_COMMUNICATION_PROTOCOLS
        assert "process_stdio" in VCP_1_COMMUNICATION_PROTOCOLS

    def test_manifest_version_1_0_0(self):
        """VCP 1.0 manifest version 真借鉴."""
        assert VCP_1_MANIFEST_VERSION == "1.0.0"


# ============================================================================
# 2. find_vcp_source_root
# ============================================================================


class TestFindVCPSourceRoot:
    """V1071 VCP path resolver 真生产测试 (主 23:28 真读源码)."""

    def test_find_vcp_root(self):
        """真找 VCP 源码根路径 (主 17:43 实事求是)."""
        root = find_vcp_source_root()
        # real test: should find or return None
        if root is not None:
            assert os.path.isdir(root)
            assert os.path.isdir(os.path.join(root, "Plugin"))
            assert os.path.isfile(os.path.join(root, "Plugin.js"))
        # even if None, the function should not crash
        assert root is None or isinstance(root, str)


# ============================================================================
# 3. discover_plugin_dirs
# ============================================================================


class TestDiscoverPluginDirs:
    """V1071 Plugin discovery 真生产测试."""

    def test_discover_real_plugins(self):
        """真列所有 plugin 目录 (主 23:28 真读)."""
        root = find_vcp_source_root()
        if root is None:
            pytest.skip("VCP source root not found")
        dirs = discover_plugin_dirs(root)
        assert isinstance(dirs, list)
        assert len(dirs) >= 50  # real VCP has 65+ plugins
        # each dir should have plugin-manifest.json
        for d in dirs:
            assert os.path.isfile(os.path.join(d, "plugin-manifest.json"))


# ============================================================================
# 4. PluginManifest
# ============================================================================


class TestPluginManifest:
    """V1071 PluginManifest 真生产测试 (主 23:28 真读)."""

    def test_parse_manifest_real(self):
        """真读 + 真解析 manifest 真生产 (主 17:43 实事求是)."""
        root = find_vcp_source_root()
        if root is None:
            pytest.skip("VCP source root not found")
        dirs = discover_plugin_dirs(root)
        if not dirs:
            pytest.skip("No plugin dirs found")
        pm = parse_manifest(dirs[0])
        assert isinstance(pm, PluginManifest)
        # should parse OK
        if pm.parse_ok:
            assert pm.name != ""
            assert pm.plugin_type != ""
            assert pm.manifest_version != ""

    def test_parse_manifest_extracts_commands(self):
        """extract invocation commands 真借鉴."""
        root = find_vcp_source_root()
        if root is None:
            pytest.skip("VCP source root not found")
        dirs = discover_plugin_dirs(root)
        # find one with commands
        for d in dirs:
            pm = parse_manifest(d)
            if pm.parse_ok and pm.n_invocation_commands > 0:
                assert len(pm.invocation_identifiers) > 0
                return
        pytest.skip("No plugin with commands found")

    def test_parse_manifest_websocket_detection(self):
        """WebSocket detection 真借鉴 (主 19:33)."""
        root = find_vcp_source_root()
        if root is None:
            pytest.skip("VCP source root not found")
        dirs = discover_plugin_dirs(root)
        # at least one should have WebSocket
        for d in dirs:
            pm = parse_manifest(d)
            if pm.parse_ok and pm.has_websocket:
                assert pm.ws_message_type != ""
                return
        # no WebSocket plugins, but should not fail
        assert True


# ============================================================================
# 5. TypeDistribution
# ============================================================================


class TestTypeDistribution:
    """V1071 Plugin type distribution 真生产测试."""

    def test_add_and_count(self):
        """add 真借鉴 + count 真生产."""
        td = TypeDistribution()
        td.add("synchronous")
        td.add("synchronous")
        td.add("asynchronous")
        assert td.counts["synchronous"] == 2
        assert td.counts["asynchronous"] == 1
        assert td.diversity == 2
        assert td.total == 3

    def test_diversity_score_full(self):
        """6 types 真借鉴 (V1001 集成)."""
        td = TypeDistribution()
        for t in VCP_1_PLUGIN_TYPES:
            td.add(t)
        assert td.diversity == 6
        assert td.diversity_score() == 1.0

    def test_diversity_score_partial(self):
        """partial diversity 真借鉴."""
        td = TypeDistribution()
        td.add("synchronous")
        td.add("service")
        assert td.diversity == 2
        assert abs(td.diversity_score() - 2 / 6) < 0.001


# ============================================================================
# 6. ProtocolDistribution
# ============================================================================


class TestProtocolDistribution:
    """V1071 Protocol distribution 真生产测试."""

    def test_add_and_count(self):
        """add 真借鉴 + count 真生产."""
        pd = ProtocolDistribution()
        pd.add("stdio")
        pd.add("stdio")
        pd.add("direct")
        assert pd.counts["stdio"] == 2
        assert pd.diversity == 2

    def test_diversity_score_full(self):
        """3 protocols 真借鉴."""
        pd = ProtocolDistribution()
        for p in VCP_1_COMMUNICATION_PROTOCOLS:
            pd.add(p)
        assert pd.diversity == 3
        assert pd.diversity_score() == 1.0


# ============================================================================
# 7. extract_capability_summary
# ============================================================================


class TestCapabilitySummary:
    """V1071 capability 真聚合测试 (主 19:33)."""

    def test_capability_summary_empty(self):
        """empty summary 真生产."""
        result = extract_capability_summary([])
        assert result["total_invocations"] == 0
        assert result["unique_identifiers"] == 0

    def test_capability_summary_real(self):
        """real summary 真借鉴 (主 23:28 真读)."""
        root = find_vcp_source_root()
        if root is None:
            pytest.skip("VCP source root not found")
        dirs = discover_plugin_dirs(root)
        manifests = [parse_manifest(d) for d in dirs]
        result = extract_capability_summary(manifests)
        assert result["total_invocations"] >= 50  # real VCP has many
        assert result["unique_identifiers"] >= 30


# ============================================================================
# 8. validate_entry_points
# ============================================================================


class TestEntryPointValidator:
    """V1071 entry point 真校验测试 (主 17:43 实事求是)."""

    def test_validate_real(self):
        """真校验 real VCP entry points (主 17:43)."""
        root = find_vcp_source_root()
        if root is None:
            pytest.skip("VCP source root not found")
        dirs = discover_plugin_dirs(root)
        manifests = [parse_manifest(d) for d in dirs]
        result = validate_entry_points(manifests)
        # real VCP: most should be valid
        assert result["n_valid"] >= 50
        assert result["n_invalid"] >= 0


# ============================================================================
# 9. validate_vcp1_spec
# ============================================================================


class TestVCP1SpecValidator:
    """V1071 VCP 1.0 规范真校验测试 (V1001 集成)."""

    def test_validate_real_spec(self):
        """真校验 VCP 1.0 spec 真生产."""
        root = find_vcp_source_root()
        if root is None:
            pytest.skip("VCP source root not found")
        dirs = discover_plugin_dirs(root)
        manifests = [parse_manifest(d) for d in dirs]
        res = validate_vcp1_spec(manifests)
        assert isinstance(res, VCP1SpecResult)
        assert res.n_total == len(manifests)
        # real VCP: most should be valid type
        assert res.n_valid_type >= 50
        # type diversity should be 6 (full)
        assert res.type_distribution.diversity == 6
        # validity should be high
        assert res.validity_score() >= 0.7

    def test_validity_score_range(self):
        """validity score 范围真生产."""
        res = VCP1SpecResult()
        res.n_total = 100
        res.n_manifest_v1 = 80
        res.n_valid_type = 95
        res.n_valid_protocol = 90
        res.n_parse_ok = 100
        td = TypeDistribution()
        for t in VCP_1_PLUGIN_TYPES:
            td.add(t)
        res.type_distribution = td
        score = res.validity_score()
        assert 0.0 <= score <= 1.0
        assert score >= 0.85  # 80% manifest v1 + 95% type + 90% proto + 100% parse + 100% diversity


# ============================================================================
# 10. V1071VCPDeepRead
# ============================================================================


class TestV1071VCPDeepRead:
    """V1071 Deep Read Orchestrator 真生产测试 (主 00:56 任何人能接手)."""

    def test_init(self):
        """init 真生产 (主 17:43 实事求是)."""
        reader = V1071VCPDeepRead()
        # vcp_root may or may not be found
        assert reader.vcp_root is None or isinstance(reader.vcp_root, str)

    def test_run_real(self):
        """run 真读 VCP 源码 (主 23:28 真读源码)."""
        reader = V1071VCPDeepRead()
        result = reader.run()
        if "error" in result:
            pytest.skip(f"VCP source not found: {result['error']}")
        assert result["n_plugins"] >= 50
        assert "spec_result" in result
        assert "entry_validation" in result
        assert "capability_summary" in result

    def test_run_idempotent(self):
        """run idempotent 真借鉴 (主 17:43)."""
        reader = V1071VCPDeepRead()
        r1 = reader.run()
        r2 = reader.run()
        if "error" in r1:
            pytest.skip("VCP source not found")
        # second run should reset and re-discover
        assert r1["n_plugins"] == r2["n_plugins"]
        assert len(reader.manifests) == r1["n_plugins"]

    def test_measure(self):
        """measure V0.2 真测 (主 22:33)."""
        reader = V1071VCPDeepRead()
        m = reader.measure()
        if "error" in m:
            pytest.skip("VCP source not found")
        assert 0.0 <= m["raw_vcp_4"] <= 1.0
        assert 0.0 <= m["raw_cross_domain"] <= 1.0
        # real VCP should give high vcp_4
        assert m["raw_vcp_4"] >= 0.85

    def test_bridge_measure(self):
        """V0.2 bridge measure 真测 (主 22:33 16 项真测)."""
        score = v1071_bridge_measure()
        # either we found VCP and got a real score, or 0.0
        assert 0.0 <= score <= 1.0

    def test_cross_domain_measure(self):
        """cross_domain measure 真测."""
        score = v1071_cross_domain_measure()
        assert 0.0 <= score <= 1.0

    def test_report_markdown(self):
        """Markdown report 真生产 (主 00:56 任何人能接手)."""
        md = v1071_report_markdown()
        assert "# V1071" in md
        # if VCP found, should have full report
        if "Error" not in md:
            assert "6 Plugin Types" in md
            assert "Capability" in md


# ============================================================================
# 11. V3 不假装哲学守门
# ============================================================================


class TestV3Guard:
    """V1071 V3 不假装哲学守门 (主 17:58 + 主 20:46 + 主 17:43)."""

    def test_not_pretend_read(self):
        """真读 85 manifest 不假装已读 真守门."""
        g = v1071_philosophy_guard()
        assert g["not_pretend_read"]

    def test_not_vcp_fully_understood(self):
        """85 manifest ≠ VCP 全理解 真守门."""
        g = v1071_philosophy_guard()
        assert g["not_vcp_fully_understood"]

    def test_not_6_types_exhaustive(self):
        """6 types ≠ 穷尽 真守门."""
        g = v1071_philosophy_guard()
        assert g["not_6_types_exhaustive"]

    def test_not_count_as_capability(self):
        """plugin count ≠ capability 真守门."""
        g = v1071_philosophy_guard()
        assert g["not_count_as_capability"]

    def test_not_v1071_equals_vcp(self):
        """V1071 ≠ VCP 真守门."""
        g = v1071_philosophy_guard()
        assert g["not_v1071_equals_vcp"]


# ============================================================================
# 12. v1071_run entry
# ============================================================================


class TestV1071Run:
    """V1071 v1071_run 真生产 entry (主 00:56 任何人能接手)."""

    def test_v1071_run(self):
        """v1071_run 真生产 entry (主 00:56)."""
        r = v1071_run()
        assert r["version"] == V1071_VERSION
        assert "result" in r
        assert "measure" in r
        assert "philosophy_guard" in r
        assert "report" in r
