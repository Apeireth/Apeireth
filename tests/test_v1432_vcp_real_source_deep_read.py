"""Tests for V1432 — ASI VCP 真实源代码 deep read (主 00:44 质量工程化)."""

from __future__ import annotations

import json
import sys
from unittest.mock import patch

import pytest


def test_v1432_importable():
    import apeireth.v1432_vcp_real_source_deep_read as m
    assert m.V1432_VERSION == "0.1.0"


def test_v1432_guards_count():
    import apeireth.v1432_vcp_real_source_deep_read as m
    assert len(m.V1432_GUARDS) == 14
    assert len(m.V1432_V3_GUARDS) == 5


def test_v1432_borrowed_count():
    import apeireth.v1432_vcp_real_source_deep_read as m
    assert len(m.V1432_BORROWED) == 5


def test_v1432_selected_paths_bounded():
    """SELECTED_PATHS must be 1..50 (GUARD_FETCH_BOUNDED)."""
    import apeireth.v1432_vcp_real_source_deep_read as m
    assert 1 <= len(m.SELECTED_PATHS) <= 50


def test_v1432_vcp_layers_six():
    """VCP 6 layers (I-T-S-A-M-E)."""
    import apeireth.v1432_vcp_real_source_deep_read as m
    assert len(m.VCP_LAYERS) == 6
    assert "Identity" in m.VCP_LAYERS
    assert "Transport" in m.VCP_LAYERS
    assert "Economic" in m.VCP_LAYERS


def test_v1432_v1426_protocols_six():
    """V1426 6 protocols."""
    import apeireth.v1432_vcp_real_source_deep_read as m
    assert len(m.V1426_PROTOCOLS) == 6
    assert "sync" in m.V1426_PROTOCOLS
    assert "async" in m.V1426_PROTOCOLS
    assert "hybrid" in m.V1426_PROTOCOLS


def test_v1432_vcp_v1426_map_six_entries():
    """Mapping has 6 entries (one per VCP layer)."""
    import apeireth.v1432_vcp_real_source_deep_read as m
    assert len(m.VCP_TO_V1426_MAP) == 6
    # Every VCP layer is mapped
    for layer in m.VCP_LAYERS:
        assert layer in m.VCP_TO_V1426_MAP


def test_v1432_map_to_v1426_returns_six():
    import apeireth.v1432_vcp_real_source_deep_read as m
    mappings = m.map_to_v1426()
    assert len(mappings) == 6
    # Each mapping has bounded match_score (GUARD_MODULE_BOUNDED)
    for mapping in mappings:
        assert 0.0 <= mapping.match_score <= 1.0
        assert mapping.vcp_module in m.VCP_LAYERS
        assert mapping.v1426_protocol in m.V1426_PROTOCOLS


def test_v1432_module_meta_keys():
    import apeireth.v1432_vcp_real_source_deep_read as m
    meta = m.module_meta()
    assert meta["version"] == "0.1.0"
    assert meta["module"] == "v1432_vcp_real_source_deep_read"
    assert meta["schema"] == "v1432.vcp-real-source-deep-read/v1"
    assert meta["n_guards"] == 14
    assert meta["n_v3_guards"] == 5
    assert meta["n_borrowed"] == 5
    assert meta["vcp_repo"] == "Creed-Space/VCP-SDK"


def test_v1432_chain_delegate_returns_v1426():
    import apeireth.v1432_vcp_real_source_deep_read as m
    chain = m.chain_delegate()
    assert "all_ok" in chain
    assert "chain" in chain
    assert "V1426" in chain["chain"]
    assert chain["chain"]["V1426"]["ok"] is True
    assert chain["chain"]["V1426"]["version"] == "0.1.0"


def test_v1432_popper_self_test_all_pass():
    """Popper self-test: 14/14 must pass."""
    import apeireth.v1432_vcp_real_source_deep_read as m
    result = m.popper_self_test()
    assert result["n_total"] == 14
    assert result["n_pass"] == 14, f"failed checks: {[k for k, v in result['results'].items() if not v[0]]}"
    assert result["ok"] is True


def test_v1432_fetched_file_defaults_to_skipped():
    import apeireth.v1432_vcp_real_source_deep_read as m
    f = m.FetchedFile(path="/test/file.py")
    assert f.status == m.ModuleReadStatus.SKIPPED
    assert f.size == 0
    assert f.content == ""
    assert f.error == ""


def test_v1432_fetched_file_to_dict():
    import apeireth.v1432_vcp_real_source_deep_read as m
    f = m.FetchedFile(
        path="/test/file.py",
        status=m.ModuleReadStatus.FETCHED,
        size=100,
        content="hello",
        sha="abc123",
    )
    d = f.to_dict()
    assert d["path"] == "/test/file.py"
    assert d["status"] == "FETCHED"
    assert d["size"] == 100
    assert d["content_length"] == 5
    assert d["sha"] == "abc123"


def test_v1432_module_mapping_to_dict():
    import apeireth.v1432_vcp_real_source_deep_read as m
    mapping = m.ModuleMapping(
        vcp_module="Identity",
        v1426_protocol="static",
        match_score=0.6,
        rationale="Identity tokens are pre-computed",
    )
    d = mapping.to_dict()
    assert d["vcp_module"] == "Identity"
    assert d["v1426_protocol"] == "static"
    assert d["match_score"] == 0.6


def test_v1432_vcp_deep_read_report_defaults():
    import apeireth.v1432_vcp_real_source_deep_read as m
    r = m.VCPDeepReadReport()
    assert r.n_fetched == 0
    assert r.n_failed == 0
    assert r.n_skipped == 0
    assert r.n_total == 0
    assert r.avg_match_score == 0.0


def test_v1432_vcp_deep_read_report_to_dict():
    import apeireth.v1432_vcp_real_source_deep_read as m
    r = m.VCPDeepReadReport(
        n_fetched=2,
        n_failed=1,
        n_skipped=3,
        n_total=6,
        avg_match_score=0.6,
        started_iso="2026-08-10T00:00:00+00:00",
        ended_iso="2026-08-10T00:00:01+00:00",
    )
    d = r.to_dict()
    assert d["n_fetched"] == 2
    assert d["n_total"] == 6
    assert d["avg_match_score"] == 0.6


def test_v1432_render_report_md_with_fake_report():
    import apeireth.v1432_vcp_real_source_deep_read as m
    mappings = m.map_to_v1426()
    fake_report = m.VCPDeepReadReport(
        fetched_files=[
            m.FetchedFile(path="/test/file.py", status=m.ModuleReadStatus.FETCHED,
                          size=100, content="hello", sha="abc")
        ],
        mappings=mappings,
        n_fetched=1,
        n_total=1,
        avg_match_score=0.6,
        started_iso="2026-08-10T00:00:00+00:00",
        ended_iso="2026-08-10T00:00:01+00:00",
    )
    md = m.render_report_md(fake_report)
    assert "V1432" in md
    assert "Honest disclosure" in md
    assert "Creed-Space/VCP-SDK" in md
    assert "0.6000" in md  # avg match score formatted


def test_v1432_render_report_md_contains_all_mappings():
    import apeireth.v1432_vcp_real_source_deep_read as m
    mappings = m.map_to_v1426()
    fake_report = m.VCPDeepReadReport(
        mappings=mappings,
        avg_match_score=0.6,
    )
    md = m.render_report_md(fake_report)
    for mapping in mappings:
        assert mapping.vcp_module in md
        assert mapping.v1426_protocol in md


def test_v1432_module_read_status_enum():
    import apeireth.v1432_vcp_real_source_deep_read as m
    assert m.ModuleReadStatus.FETCHED.value == "FETCHED"
    assert m.ModuleReadStatus.SKIPPED.value == "SKIPPED"
    assert m.ModuleReadStatus.FAILED.value == "FAILED"


def test_v1432_now_iso_is_string():
    import apeireth.v1432_vcp_real_source_deep_read as m
    iso = m._now_iso()
    assert isinstance(iso, str)
    assert "T" in iso  # ISO 8601 contains T


def test_v1432_http_get_json_bad_url_returns_error():
    """Bad URL → (0, None, error)."""
    import apeireth.v1432_vcp_real_source_deep_read as m
    code, data, err = m._http_get_json("http://127.0.0.1:1/nope", timeout=1.0)
    assert code == 0
    assert data is None
    assert err != ""


def test_v1432_fetch_repo_root_handles_error():
    """fetch_repo_root returns [] on error (graceful)."""
    import apeireth.v1432_vcp_real_source_deep_read as m
    with patch.object(m, "_http_get_json", return_value=(0, None, "mock error")):
        result = m.fetch_repo_root()
        assert result == []


def test_v1432_fetch_file_marks_failed_on_error():
    """fetch_file returns FAILED when network fails."""
    import apeireth.v1432_vcp_real_source_deep_read as m
    with patch.object(m, "_http_get_json", return_value=(0, None, "mock error")):
        f = m.fetch_file("python/src/vcp/__init__.py", timeout=1.0)
        assert f.status == m.ModuleReadStatus.FAILED
        assert f.error == "mock error"


def test_v1432_fetch_file_marks_fetched_on_success():
    """fetch_file returns FETCHED when network returns valid base64."""
    import apeireth.v1432_vcp_real_source_deep_read as m
    import base64
    content = "# VCP module\nprint('hello')\n"
    b64 = base64.b64encode(content.encode()).decode()
    mock_data = {"content": b64, "size": 100, "sha": "abc123"}
    with patch.object(m, "_http_get_json", return_value=(200, mock_data, "")):
        f = m.fetch_file("python/src/vcp/__init__.py", timeout=1.0)
        assert f.status == m.ModuleReadStatus.FETCHED
        assert f.content == content
        assert f.size == 100
        assert f.sha == "abc123"


def test_v1432_fetch_file_decodes_base64_correctly():
    """fetch_file base64-decodes content properly."""
    import apeireth.v1432_vcp_real_source_deep_read as m
    import base64
    content = "class Foo:\n    pass\n"
    b64 = base64.b64encode(content.encode()).decode()
    mock_data = {"content": b64, "size": len(content), "sha": "sha"}
    with patch.object(m, "_http_get_json", return_value=(200, mock_data, "")):
        f = m.fetch_file("python/src/vcp/types.py", timeout=1.0)
        assert f.status == m.ModuleReadStatus.FETCHED
        assert "class Foo" in f.content


def test_v1432_run_deep_read_handles_total_failure_gracefully():
    """Even if all fetches fail, run_deep_read returns a valid report."""
    import apeireth.v1432_vcp_real_source_deep_read as m
    with patch.object(m, "_http_get_json", return_value=(0, None, "mock error")):
        report = m.run_deep_read(timeout=1.0)
        # We don't assert all-failed (some paths may SKIPPED), but report must be valid
        assert report.n_total == len(m.SELECTED_PATHS)
        # All should be FAILED since we mocked _http_get_json to error
        assert report.n_failed == len(m.SELECTED_PATHS)
        # Mappings still happen (offline)
        assert len(report.mappings) == 6
        assert report.avg_match_score == 0.6


def test_v1432_cli_help_runs():
    """CLI help runs without error."""
    import apeireth.v1432_vcp_real_source_deep_read as m
    rc = m.main(["help"])
    assert rc == 0


def test_v1432_cli_version_runs():
    """CLI version runs without error."""
    import apeireth.v1432_vcp_real_source_deep_read as m
    rc = m.main(["version"])
    assert rc == 0


def test_v1432_cli_meta_runs():
    """CLI meta runs without error."""
    import apeireth.v1432_vcp_real_source_deep_read as m
    rc = m.main(["meta"])
    assert rc == 0


def test_v1432_cli_demo_runs():
    """CLI demo runs without error (offline mapping)."""
    import apeireth.v1432_vcp_real_source_deep_read as m
    rc = m.main(["demo"])
    assert rc == 0


def test_v1432_cli_compare_runs():
    """CLI compare runs without error."""
    import apeireth.v1432_vcp_real_source_deep_read as m
    rc = m.main(["compare"])
    assert rc == 0


def test_v1432_cli_popper_runs():
    """CLI popper runs without error."""
    import apeireth.v1432_vcp_real_source_deep_read as m
    rc = m.main(["popper"])
    assert rc == 0


def test_v1432_cli_chain_runs():
    """CLI chain runs without error."""
    import apeireth.v1432_vcp_real_source_deep_read as m
    rc = m.main(["chain"])
    assert rc == 0


def test_v1432_cli_unknown_command_returns_1():
    """Unknown command returns 1."""
    import apeireth.v1432_vcp_real_source_deep_read as m
    rc = m.main(["bogus_command"])
    assert rc == 1


def test_v1432_cli_no_args_returns_help():
    """No args returns 0 (help)."""
    import apeireth.v1432_vcp_real_source_deep_read as m
    rc = m.main([])
    assert rc == 0


def test_v1432_vcp_to_v1426_mapping_layer_protocol_pairs():
    """Every VCP layer maps to a valid V1426 protocol."""
    import apeireth.v1432_vcp_real_source_deep_read as m
    for layer, (protocol, rationale) in m.VCP_TO_V1426_MAP.items():
        assert protocol in m.V1426_PROTOCOLS
        assert isinstance(rationale, str)
        assert len(rationale) > 10


def test_v1432_honest_disclosure_in_docstring():
    """Honest disclosure is present in module docstring (GUARD_HONEST_DISCLOSURE)."""
    import apeireth.v1432_vcp_real_source_deep_read as m
    assert "Honest disclosure" in (m.__doc__ or "")


def test_v1432_no_v3_phenomenal_guard():
    """GUARD_NO_PHENOMENAL_VCP is present."""
    import apeireth.v1432_vcp_real_source_deep_read as m
    assert "GUARD_NO_PHENOMENAL_VCP" in m.V1432_V3_GUARDS


def test_v1432_no_asi_vcp_guard():
    """GUARD_NO_ASI_VCP is present."""
    import apeireth.v1432_vcp_real_source_deep_read as m
    assert "GUARD_NO_ASI_VCP" in m.V1432_V3_GUARDS


def test_v1432_no_fake_integration_guard():
    """GUARD_NO_FAKE_INTEGRATION is present (fetch ≠ integration)."""
    import apeireth.v1432_vcp_real_source_deep_read as m
    assert "GUARD_NO_FAKE_INTEGRATION" in m.V1432_V3_GUARDS


def test_v1432_user_agent_set():
    """USER_AGENT is non-empty (GUARD_USER_AGENT)."""
    import apeireth.v1432_vcp_real_source_deep_read as m
    assert m.USER_AGENT != ""
    assert "apeireth" in m.USER_AGENT.lower()


def test_v1432_vcp_api_base_https():
    """VCP_API_BASE uses HTTPS."""
    import apeireth.v1432_vcp_real_source_deep_read as m
    assert m.VCP_API_BASE.startswith("https://")


def test_v1432_vcp_repo_format():
    """VCP_REPO is owner/name format."""
    import apeireth.v1432_vcp_real_source_deep_read as m
    assert "/" in m.VCP_REPO
    owner, name = m.VCP_REPO.split("/", 1)
    assert owner and name


if __name__ == "__main__":
    # Allow running as a script for quick smoke.
    import apeireth.v1432_vcp_real_source_deep_read as m
    sys.exit(m.main(["popper"]))