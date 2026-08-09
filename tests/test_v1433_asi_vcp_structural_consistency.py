"""Tests for V1433 — ASI VCP structural consistency report (主 00:44 质量工程化)."""

from __future__ import annotations

import sys

import pytest


def test_v1433_importable():
    import apeireth.v1433_asi_vcp_structural_consistency as m
    assert m.V1433_VERSION == "0.1.0"


def test_v1433_guards_count():
    import apeireth.v1433_asi_vcp_structural_consistency as m
    assert len(m.V1433_GUARDS) == 14
    assert len(m.V1433_V3_GUARDS) == 5


def test_v1433_borrowed_count():
    import apeireth.v1433_asi_vcp_structural_consistency as m
    assert len(m.V1433_BORROWED) == 5


def test_v1433_direction_enum():
    """Direction enum has 2 values (FORWARD / REVERSE)."""
    import apeireth.v1433_asi_vcp_structural_consistency as m
    assert m.Direction.FORWARD.value == "FORWARD"
    assert m.Direction.REVERSE.value == "REVERSE"
    assert len(list(m.Direction)) == 2


def test_v1433_gap_dataclass():
    import apeireth.v1433_asi_vcp_structural_consistency as m
    g = m.Gap(direction=m.Direction.FORWARD, name="TestLayer", note="missing")
    assert g.direction == m.Direction.FORWARD
    d = g.to_dict()
    assert d["direction"] == "FORWARD"
    assert d["name"] == "TestLayer"
    assert d["note"] == "missing"


def test_v1433_consistency_row_dataclass():
    import apeireth.v1433_asi_vcp_structural_consistency as m
    r = m.ConsistencyRow(
        vcp_layer="Identity",
        v1426_protocol="static",
        match_score=0.6,
        rationale="test",
    )
    assert r.vcp_layer == "Identity"
    d = r.to_dict()
    assert d["match_score"] == 0.6


def test_v1433_consistency_report_defaults():
    import apeireth.v1433_asi_vcp_structural_consistency as m
    r = m.ConsistencyReport()
    assert r.forward_coverage == 0.0
    assert r.reverse_coverage == 0.0
    assert r.parity == 0.0
    assert r.n_layers == 0


def test_v1433_load_v1432_layers():
    """V1432 layers load (>= 1)."""
    import apeireth.v1433_asi_vcp_structural_consistency as m
    layers = m._load_v1432_layers()
    assert len(layers) >= 1


def test_v1433_load_v1432_mappings():
    """V1432 mappings load (>= 1)."""
    import apeireth.v1433_asi_vcp_structural_consistency as m
    mappings = m._load_v1432_mappings()
    assert len(mappings) >= 1


def test_v1433_load_v1426_protocols():
    """V1426 protocols load (>= 1)."""
    import apeireth.v1433_asi_vcp_structural_consistency as m
    protocols = m._load_v1426_protocols()
    assert len(protocols) >= 1


def test_v1433_compute_forward_coverage_bounded():
    """Forward coverage ∈ [0, 1]."""
    import apeireth.v1433_asi_vcp_structural_consistency as m
    layers = m._load_v1432_layers()
    mappings = m._load_v1432_mappings()
    fwd = m.compute_forward_coverage(layers, mappings)
    assert 0.0 <= fwd <= 1.0


def test_v1433_compute_forward_coverage_with_full_mappings():
    """When all layers mapped, forward_coverage = 1.0."""
    import apeireth.v1433_asi_vcp_structural_consistency as m
    from apeireth.v1433_asi_vcp_structural_consistency import _load_v1432_layers, _load_v1432_mappings
    layers = _load_v1432_layers()
    mappings = _load_v1432_mappings()
    fwd = m.compute_forward_coverage(layers, mappings)
    # Current V1432 mapping is 1-1 so coverage is full
    assert fwd >= 0.99


def test_v1433_compute_forward_coverage_zero_when_no_layers():
    """If layers is empty, returns 0.0 (GUARD_BOUNDED)."""
    import apeireth.v1433_asi_vcp_structural_consistency as m
    fwd = m.compute_forward_coverage(layers=(), mappings=[])
    assert fwd == 0.0


def test_v1433_compute_reverse_coverage_bounded():
    """Reverse coverage ∈ [0, 1]."""
    import apeireth.v1433_asi_vcp_structural_consistency as m
    layers = m._load_v1432_layers()
    mappings = m._load_v1432_mappings()
    protocols = m._load_v1426_protocols()
    rev = m.compute_reverse_coverage(layers, mappings, protocols)
    assert 0.0 <= rev <= 1.0


def test_v1433_compute_reverse_coverage_zero_when_no_protocols():
    """If protocols is empty, returns 0.0 (GUARD_BOUNDED)."""
    import apeireth.v1433_asi_vcp_structural_consistency as m
    rev = m.compute_reverse_coverage(layers=(), mappings=[], protocols=())
    assert rev == 0.0


def test_v1433_find_forward_gaps_empty_when_full():
    """If all layers are mapped, no gaps."""
    import apeireth.v1433_asi_vcp_structural_consistency as m
    from apeireth.v1433_asi_vcp_structural_consistency import _load_v1432_layers, _load_v1432_mappings
    layers = _load_v1432_layers()
    mappings = _load_v1432_mappings()
    gaps = m.find_forward_gaps(layers, mappings)
    # V1432's VCP_TO_V1426_MAP has all 6 layers
    assert len(gaps) == 0


def test_v1433_find_forward_gaps_detects_missing_layer():
    """If a layer is missing, gap is reported."""
    import apeireth.v1433_asi_vcp_structural_consistency as m
    # Layers = (A, B, C), mappings only cover A, B → C is a gap
    fake_mapping_a = type("M", (), {"vcp_module": "A", "v1426_protocol": "sync", "match_score": 0.5, "rationale": ""})()
    fake_mapping_b = type("M", (), {"vcp_module": "B", "v1426_protocol": "async", "match_score": 0.5, "rationale": ""})()
    layers = ("A", "B", "C")
    mappings = [fake_mapping_a, fake_mapping_b]
    gaps = m.find_forward_gaps(layers, mappings)
    assert len(gaps) == 1
    assert gaps[0].name == "C"
    assert gaps[0].direction == m.Direction.FORWARD


def test_v1433_find_redundant_protocols_detects_unused():
    """If a protocol is unused, redundant is reported."""
    import apeireth.v1433_asi_vcp_structural_consistency as m
    fake_mapping_a = type("M", (), {"vcp_module": "A", "v1426_protocol": "sync", "match_score": 0.5, "rationale": ""})()
    protocols = ("sync", "async", "static")
    layers = ("A",)
    mappings = [fake_mapping_a]
    redundant = m.find_redundant_protocols(layers, mappings, protocols)
    assert len(redundant) == 2  # async, static unused
    assert {r.name for r in redundant} == {"async", "static"}


def test_v1433_compute_parity_definition():
    """Parity = (forward + reverse) / 2."""
    import apeireth.v1433_asi_vcp_structural_consistency as m
    parity = m.compute_parity(0.6, 0.4)
    assert abs(parity - 0.5) < 1e-9


def test_v1433_build_report_has_required_keys():
    """build_report returns ConsistencyReport with required keys."""
    import apeireth.v1433_asi_vcp_structural_consistency as m
    report = m.build_report()
    d = report.to_dict()
    required = {
        "rows", "gaps", "redundant", "forward_coverage",
        "reverse_coverage", "parity", "n_layers", "n_protocols",
    }
    assert required.issubset(d.keys())


def test_v1433_build_report_n_layers_n_protocols():
    """build_report fills n_layers and n_protocols from V1432/V1426."""
    import apeireth.v1433_asi_vcp_structural_consistency as m
    report = m.build_report()
    assert report.n_layers == 6  # V1432 has 6 layers
    assert report.n_protocols == 6  # V1426 has 6 protocols


def test_v1433_render_report_md_contains_v1433():
    """render_report_md output mentions V1433 + Honest disclosure."""
    import apeireth.v1433_asi_vcp_structural_consistency as m
    report = m.build_report()
    md = m.render_report_md(report)
    assert "V1433" in md
    assert "Honest disclosure" in md
    assert "forward_coverage" in md
    assert "reverse_coverage" in md
    assert "parity" in md


def test_v1433_render_report_md_contains_all_rows():
    """render_report_md contains all V1432 mappings."""
    import apeireth.v1433_asi_vcp_structural_consistency as m
    report = m.build_report()
    md = m.render_report_md(report)
    for row in report.rows:
        assert row.vcp_layer in md
        assert row.v1426_protocol in md


def test_v1433_module_meta_keys():
    """module_meta returns expected keys."""
    import apeireth.v1433_asi_vcp_structural_consistency as m
    meta = m.module_meta()
    assert meta["version"] == "0.1.0"
    assert meta["module"] == "v1433_asi_vcp_structural_consistency"
    assert meta["schema"] == "v1433.asi-vcp-structural-consistency/v1"
    assert meta["n_guards"] == 14
    assert meta["n_v3_guards"] == 5
    assert meta["n_borrowed"] == 5


def test_v1433_chain_delegate_returns_v1432_v1426():
    """chain_delegate probes V1432 + V1426."""
    import apeireth.v1433_asi_vcp_structural_consistency as m
    chain = m.chain_delegate()
    assert "all_ok" in chain
    assert "chain" in chain
    assert "V1432" in chain["chain"]
    assert "V1426" in chain["chain"]
    assert chain["chain"]["V1432"]["ok"] is True
    assert chain["chain"]["V1426"]["ok"] is True


def test_v1433_popper_self_test_all_pass():
    """Popper self-test: 14/14 must pass."""
    import apeireth.v1433_asi_vcp_structural_consistency as m
    result = m.popper_self_test()
    assert result["n_total"] == 14
    assert result["n_pass"] == 14, f"failed: {[k for k, v in result['results'].items() if not v[0]]}"
    assert result["ok"] is True


def test_v1433_now_iso_is_string():
    """_now_iso returns ISO 8601 string."""
    import apeireth.v1433_asi_vcp_structural_consistency as m
    iso = m._now_iso()
    assert isinstance(iso, str)
    assert "T" in iso


def test_v1433_cli_help_runs():
    import apeireth.v1433_asi_vcp_structural_consistency as m
    rc = m.main(["help"])
    assert rc == 0


def test_v1433_cli_version_runs():
    import apeireth.v1433_asi_vcp_structural_consistency as m
    rc = m.main(["version"])
    assert rc == 0


def test_v1433_cli_meta_runs():
    import apeireth.v1433_asi_vcp_structural_consistency as m
    rc = m.main(["meta"])
    assert rc == 0


def test_v1433_cli_popper_runs():
    import apeireth.v1433_asi_vcp_structural_consistency as m
    rc = m.main(["popper"])
    assert rc == 0


def test_v1433_cli_chain_runs():
    import apeireth.v1433_asi_vcp_structural_consistency as m
    rc = m.main(["chain"])
    assert rc == 0


def test_v1433_cli_forward_runs():
    import apeireth.v1433_asi_vcp_structural_consistency as m
    rc = m.main(["forward"])
    assert rc == 0


def test_v1433_cli_reverse_runs():
    import apeireth.v1433_asi_vcp_structural_consistency as m
    rc = m.main(["reverse"])
    assert rc == 0


def test_v1433_cli_gaps_runs():
    import apeireth.v1433_asi_vcp_structural_consistency as m
    rc = m.main(["gaps"])
    assert rc == 0


def test_v1433_cli_report_runs():
    import apeireth.v1433_asi_vcp_structural_consistency as m
    rc = m.main(["report"])
    assert rc == 0


def test_v1433_cli_meta_json_runs():
    import apeireth.v1433_asi_vcp_structural_consistency as m
    rc = m.main(["meta", "--json"])
    assert rc == 0


def test_v1433_cli_unknown_command_returns_1():
    import apeireth.v1433_asi_vcp_structural_consistency as m
    rc = m.main(["bogus_command"])
    assert rc == 1


def test_v1433_cli_no_args_returns_help():
    import apeireth.v1433_asi_vcp_structural_consistency as m
    rc = m.main([])
    assert rc == 0


def test_v1433_v3_guards_no_phenomenal():
    """GUARD_NO_PHENOMENAL_CONSISTENCY is present."""
    import apeireth.v1433_asi_vcp_structural_consistency as m
    assert "GUARD_NO_PHENOMENAL_CONSISTENCY" in m.V1433_V3_GUARDS


def test_v1433_v3_guards_no_asi():
    """GUARD_NO_ASI_CONSISTENCY is present."""
    import apeireth.v1433_asi_vcp_structural_consistency as m
    assert "GUARD_NO_ASI_CONSISTENCY" in m.V1433_V3_GUARDS


def test_v1433_v3_guards_no_fake_parity():
    """GUARD_NO_FAKE_PARITY is present."""
    import apeireth.v1433_asi_vcp_structural_consistency as m
    assert "GUARD_NO_FAKE_PARITY" in m.V1433_V3_GUARDS


def test_v1433_offline_safe_no_network_calls():
    """build_report must not call the network (GUARD_OFFLINE_SAFE)."""
    import apeireth.v1433_asi_vcp_structural_consistency as m
    from unittest.mock import patch
    # If urllib.request.urlopen is called, fail the test
    with patch("urllib.request.urlopen") as mocked:
        report = m.build_report()
        assert mocked.call_count == 0
    assert report.parity >= 0.0


def test_v1433_honest_disclosure_in_docstring():
    """Honest disclosure is in the module docstring."""
    import apeireth.v1433_asi_vcp_structural_consistency as m
    assert "Honest disclosure" in (m.__doc__ or "")


def test_v1433_json_serializable():
    """Report is JSON-serializable (GUARD_JSON_SERIALIZABLE)."""
    import apeireth.v1433_asi_vcp_structural_consistency as m
    import json
    report = m.build_report()
    d = report.to_dict()
    # Should not raise
    s = json.dumps(d, ensure_ascii=False)
    assert isinstance(s, str)
    assert "V1433" not in s or "rows" in s  # either V1433 mention or rows key


if __name__ == "__main__":
    import apeireth.v1433_asi_vcp_structural_consistency as m
    sys.exit(m.main(["popper"]))