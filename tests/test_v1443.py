"""Tests for V1443 — ASI V2 5 位置 交叉交互探针.

Phase: 1443
Date: 2026-08-10 (cron tick 06:25)
"""
from __future__ import annotations

import json
import os
import subprocess
import sys

# Ensure apeireth package is importable when running tests directly
_PROMETHEAN_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
if _PROMETHEAN_ROOT not in sys.path:
    sys.path.insert(0, _PROMETHEAN_ROOT)

# ----------------------- helpers -----------------------


def _run_module(*args: str) -> subprocess.CompletedProcess:
    """Run python -m apeireth.v1443_asi_v2_cross_position_interaction <args>."""
    cmd = [sys.executable, "-m", "apeireth.v1443_asi_v2_cross_position_interaction", *args]
    return subprocess.run(cmd, capture_output=True, text=True, timeout=60)


def _ok(p: subprocess.CompletedProcess) -> bool:
    return p.returncode == 0


# ----------------------- tests -----------------------


def test_version_cli():
    p = _run_module("version")
    assert _ok(p), f"version failed: rc={p.returncode} stderr={p.stderr[:200]}"
    assert p.stdout.strip() == "0.1.0", f"version mismatch: {p.stdout!r}"
    print("test_version_cli OK")


def test_help_cli():
    p = _run_module("help")
    assert _ok(p), f"help failed: rc={p.returncode} stderr={p.stderr[:200]}"
    assert "cross-position interaction" in p.stdout.lower() or "cross" in p.stdout.lower()
    print("test_help_cli OK")


def test_popper_cli():
    p = _run_module("popper")
    assert _ok(p), f"popper failed: rc={p.returncode} stdout={p.stdout!r} stderr={p.stderr[:200]}"
    assert "popper:13/13:True" in p.stdout, f"popper result: {p.stdout!r}"
    print("test_popper_cli OK")


def test_chain_cli():
    p = _run_module("chain")
    assert _ok(p), f"chain failed: rc={p.returncode} stderr={p.stderr[:200]}"
    out = json.loads(p.stdout)
    assert out.get("all_ok") is True, f"chain all_ok not true: {out}"
    assert out.get("module") == "v1443_asi_v2_cross_position_interaction"
    assert "popper=popper:13/13:True" in out.get("evidence", "")
    print("test_chain_cli OK")


def test_list_pairs_cli():
    p = _run_module("list-pairs")
    assert _ok(p), f"list-pairs failed: rc={p.returncode} stderr={p.stderr[:200]}"
    lines = [ln.strip() for ln in p.stdout.splitlines() if ln.strip()]
    # 5 positions × 4 neighbors = 20 directed non-self pairs
    assert len(lines) == 20, f"expected 20 pairs, got {len(lines)}"
    # Sample a known pair
    assert "scheduler -> cogitator" in lines
    assert "asi_occupier -> max_authority" in lines
    # No self-loops
    assert all("->" in ln for ln in lines)
    for ln in lines:
        parts = ln.split(" -> ")
        assert parts[0] != parts[1], f"self-loop found: {ln}"
    print("test_list_pairs_cli OK")


def test_probe_pair_cli():
    p = _run_module("probe-pair", "scheduler", "cogitator")
    assert _ok(p), f"probe-pair failed: rc={p.returncode} stderr={p.stderr[:200]}"
    out = json.loads(p.stdout)
    assert out["source_position"] == "scheduler"
    assert out["target_position"] == "cogitator"
    assert out["n_total"] == 3
    assert 0.0 <= out["interaction_rate"] <= 1.0
    assert len(out["probes"]) == 3
    kinds = {pr["probe_kind"] for pr in out["probes"]}
    expected = {
        "probe_source_to_target_invoke",
        "probe_data_handoff_attr",
        "probe_chain_delegate_handoff",
    }
    assert kinds == expected, f"kinds mismatch: {kinds}"
    print("test_probe_pair_cli OK")


def test_probe_pair_unknown_source():
    p = _run_module("probe-pair", "nonsense_position", "cogitator")
    assert p.returncode != 0, f"unknown source should fail, got rc={p.returncode}"
    assert "unknown" in p.stdout.lower() or "unknown" in p.stderr.lower()
    print("test_probe_pair_unknown_source OK")


def test_probe_pair_self_loop():
    p = _run_module("probe-pair", "scheduler", "scheduler")
    assert p.returncode != 0, f"self-loop should fail, got rc={p.returncode}"
    print("test_probe_pair_self_loop OK")


def test_probe_all_cli():
    p = _run_module("probe-all")
    assert _ok(p), f"probe-all failed: rc={p.returncode} stderr={p.stderr[:200]}"
    # 20 pair lines + 4 meta lines
    lines = [ln for ln in p.stdout.splitlines() if ln.strip()]
    pair_lines = [ln for ln in lines if " -> " in ln and "rate=" in ln]
    meta_lines = [ln for ln in lines if ln.startswith("META ")]
    assert len(pair_lines) == 20, f"expected 20 pair lines, got {len(pair_lines)}"
    assert len(meta_lines) == 4, f"expected 4 meta lines, got {len(meta_lines)}"
    # All rates parseable
    for ln in pair_lines:
        assert "rate=" in ln and "/" in ln
    print("test_probe_all_cli OK")


def test_audit_cli():
    p = _run_module("audit")
    assert _ok(p), f"audit failed: rc={p.returncode} stderr={p.stderr[:200]}"
    out = p.stdout
    assert "n_pairs=20" in out
    assert "n_pair_probes=60" in out
    assert "meta_pass=4/4" in out
    assert "total_interaction_rate=" in out
    print("test_audit_cli OK")


def test_report_cli():
    p = _run_module("report")
    assert _ok(p), f"report failed: rc={p.returncode} stderr={p.stderr[:200]}"
    out = p.stdout
    assert "wrote" in out and ".json" in out and ".md" in out
    # The files should exist now
    json_path = ".v1443-asi-v2-cross-position-interaction-report.json"
    md_path = ".v1443-asi-v2-cross-position-interaction-report.md"
    assert os.path.exists(json_path), f"missing {json_path}"
    assert os.path.exists(md_path), f"missing {md_path}"
    # JSON should be valid
    with open(json_path, "r", encoding="utf-8") as f:
        payload = json.load(f)
    assert payload["module"] == "v1443_asi_v2_cross_position_interaction"
    assert payload["version"] == "0.1.0"
    assert payload["phase"] == 1443
    assert payload["aggregates"]["n_directed_pairs"] == 20
    assert payload["aggregates"]["n_pair_probes_total"] == 60
    assert payload["aggregates"]["n_meta_probes"] == 4
    assert 0.0 <= payload["aggregates"]["mean_pair_rate"] <= 1.0
    assert 0.0 <= payload["aggregates"]["total_interaction_rate"] <= 1.0
    assert len(payload["pairs"]) == 20
    assert len(payload["meta_probes"]) == 4
    print("test_report_cli OK")


def test_run_all_cli():
    """run-all is alias for report."""
    p = _run_module("run-all")
    assert _ok(p), f"run-all failed: rc={p.returncode} stderr={p.stderr[:200]}"
    assert "wrote" in p.stdout
    print("test_run_all_cli OK")


def test_meta_cli():
    p = _run_module("meta")
    assert _ok(p), f"meta failed: rc={p.returncode} stderr={p.stderr[:200]}"
    out = json.loads(p.stdout)
    assert out["module"] == "v1443_asi_v2_cross_position_interaction"
    assert out["version"] == "0.1.0"
    assert out["phase"] == 1443
    assert out["schema"] == "v1443.asi-v2-cross-position-interaction/v1"
    assert len(out["guards"]) == 14
    assert len(out["v3_guards"]) == 5
    assert len(out["borrowed"]) == 5
    assert len(out["probe_kinds"]) == 3
    assert len(out["meta_probe_kinds"]) == 4
    assert "v1442_asi_v2_five_position_real_occupier" in out["depends_on"]
    print("test_meta_cli OK")


def test_unknown_cmd():
    p = _run_module("not-a-real-command")
    assert p.returncode != 0, f"unknown cmd should fail, got rc={p.returncode}"
    print("test_unknown_cmd OK")


def test_position_constants_have_5_positions():
    """V1442 must have 5 positions (assumed dependency)."""
    import apeireth.v1442_asi_v2_five_position_real_occupier as v1442
    positions = v1442.V1442_POSITIONS
    assert len(positions) == 5, f"V1442 should have 5 positions, got {len(positions)}"
    expected_ids = {"scheduler", "cogitator", "aggregator", "max_authority", "asi_occupier"}
    actual_ids = {p["id"] for p in positions}
    assert actual_ids == expected_ids, f"position ids: {actual_ids}"
    print("test_position_constants_have_5_positions OK")


def test_v1443_direct_run_all():
    """Direct (in-process) run_all returns expected shape."""
    import apeireth.v1443_asi_v2_cross_position_interaction as v1443
    rep = v1443.run_all()
    assert rep.n_positions == 5
    assert rep.n_directed_pairs == 20
    assert rep.n_probes_per_pair == 3
    assert rep.n_pair_probes_total == 60
    assert rep.n_meta_probes == 4
    assert len(rep.pairs) == 20
    assert len(rep.meta_probes) == 4
    # All pairs have 3 probes
    for p in rep.pairs:
        assert len(p.probes) == 3
        assert p.n_total == 3
        assert 0.0 <= p.interaction_rate <= 1.0
        assert p.source_position != p.target_position  # no self-loop
    # All meta probes have passed bool
    for m in rep.meta_probes:
        assert isinstance(m.passed, bool)
        assert m.probe_kind in v1443.V1443_META_PROBE_KINDS
    # Aggregate rates are valid
    assert 0.0 <= rep.mean_pair_rate <= 1.0
    assert 0.0 <= rep.meta_pass_rate <= 1.0
    assert 0.0 <= rep.total_interaction_rate <= 1.0
    # Caveats present (honest disclosure)
    assert len(rep.caveats) >= 1
    assert any("runtime" in c.lower() or "static" in c.lower() for c in rep.caveats)
    print("test_v1443_direct_run_all OK")


def test_v1443_meta_probes_have_no_self_loops():
    """Meta probe no_self_loop must pass (zero self-loop pairs)."""
    import apeireth.v1443_asi_v2_cross_position_interaction as v1443
    rep = v1443.run_all()
    no_self = next(m for m in rep.meta_probes if m.probe_kind == "probe_no_self_loop")
    assert no_self.passed is True, f"no_self_loop failed: {no_self.evidence}"
    print("test_v1443_meta_probes_have_no_self_loops OK")


def test_v1443_round_trip_pass():
    """Meta probe round_trip must pass for scheduler-cogitator pair."""
    import apeireth.v1443_asi_v2_cross_position_interaction as v1443
    rep = v1443.run_all()
    rt = next(m for m in rep.meta_probes if m.probe_kind == "probe_round_trip_ok")
    assert rt.passed is True, f"round_trip failed: {rt.evidence}"
    print("test_v1443_round_trip_pass OK")


def test_v1443_chain_delegate_callable():
    """chain_delegate must return dict with all_ok=True when popper passes."""
    import apeireth.v1443_asi_v2_cross_position_interaction as v1443
    res = v1443.chain_delegate(prev_ok=True)
    assert isinstance(res, dict)
    assert res.get("all_ok") is True
    assert res.get("module") == "v1443_asi_v2_cross_position_interaction"
    print("test_v1443_chain_delegate_callable OK")


def test_v1443_chain_delegate_with_prev_false():
    """chain_delegate should be False if prev_ok=False."""
    import apeireth.v1443_asi_v2_cross_position_interaction as v1443
    res = v1443.chain_delegate(prev_ok=False)
    assert res.get("all_ok") is False
    print("test_v1443_chain_delegate_with_prev_false OK")


def test_v1443_probe_pair_data_structure():
    """PairInteraction dataclass fields."""
    import apeireth.v1443_asi_v2_cross_position_interaction as v1443
    rep = v1443.run_all()
    p = rep.pairs[0]
    assert hasattr(p, "source_position")
    assert hasattr(p, "target_position")
    assert hasattr(p, "source_modules")
    assert hasattr(p, "target_modules")
    assert hasattr(p, "probes")
    assert hasattr(p, "interaction_rate")
    assert hasattr(p, "module_referenced")
    assert hasattr(p, "handoff_attr_present")
    assert hasattr(p, "chain_source_ok")
    assert hasattr(p, "chain_target_ok")
    print("test_v1443_probe_pair_data_structure OK")


def test_v1443_module_id_safety():
    """_import_module_safely must reject unsafe module ids."""
    import apeireth.v1443_asi_v2_cross_position_interaction as v1443
    # Spaces should be rejected
    ok, ev, _ = v1443._import_module_safely("foo bar")
    assert not ok
    assert "unsafe_module_id" in ev
    # Dots in module id should be rejected (no traversal)
    ok, ev, _ = v1443._import_module_safely("foo.bar")
    assert not ok
    assert "unsafe_module_id" in ev
    # Empty string should be rejected
    ok, ev, _ = v1443._import_module_safely("")
    assert not ok
    assert "invalid_module_id" in ev
    print("test_v1443_module_id_safety OK")


def test_v1443_helpers_clip01():
    """_clip01 must bound to [0,1]."""
    import apeireth.v1443_asi_v2_cross_position_interaction as v1443
    assert v1443._clip01(0.5) == 0.5
    assert v1443._clip01(-0.1) == 0.0
    assert v1443._clip01(1.5) == 1.0
    assert v1443._clip01(0.0) == 0.0
    assert v1443._clip01(1.0) == 1.0
    print("test_v1443_helpers_clip01 OK")


def test_v1443_helpers_safe_path():
    """_safe_path must strip traversal."""
    import apeireth.v1443_asi_v2_cross_position_interaction as v1443
    assert "../foo" not in v1443._safe_path("../foo/../bar")
    assert "../" not in v1443._safe_path("a/../b")
    print("test_v1443_helpers_safe_path OK")


def test_v1443_guards_have_14():
    """V1443_GUARDS must have 14 entries."""
    import apeireth.v1443_asi_v2_cross_position_interaction as v1443
    assert len(v1443.V1443_GUARDS) == 14, f"got {len(v1443.V1443_GUARDS)}"
    print("test_v1443_guards_have_14 OK")


def test_v1443_v3_guards_have_5():
    """V1443_V3_GUARDS must have 5 entries (no-pretend)."""
    import apeireth.v1443_asi_v2_cross_position_interaction as v1443
    assert len(v1443.V1443_V3_GUARDS) == 5, f"got {len(v1443.V1443_V3_GUARDS)}"
    expected = {
        "GUARD_V2_IS_NOT_PHENOMENAL",
        "GUARD_V2_IS_NOT_ASI",
        "GUARD_V2_IS_NOT_HUMAN_LEVEL",
        "GUARD_V2_IS_NOT_ABSOLUTE",
        "GUARD_V2_IS_NOT_RUNTIME",
    }
    assert set(v1443.V1443_V3_GUARDS) == expected
    print("test_v1443_v3_guards_have_5 OK")


def test_v1443_borrowed_has_5():
    """V1443_BORROWED must have 5 entries."""
    import apeireth.v1443_asi_v2_cross_position_interaction as v1443
    assert len(v1443.V1443_BORROWED) == 5, f"got {len(v1443.V1443_BORROWED)}"
    # V1442 must be in borrowed (compositional)
    assert any("v1442" in b for b in v1443.V1443_BORROWED)
    print("test_v1443_borrowed_has_5 OK")


def test_v1443_md_report_exists_and_valid():
    """MD report should exist after report run."""
    p = _run_module("report")
    assert _ok(p)
    md_path = ".v1443-asi-v2-cross-position-interaction-report.md"
    assert os.path.exists(md_path)
    with open(md_path, "r", encoding="utf-8") as f:
        md = f.read()
    assert "V1443" in md
    assert "5 位置" in md or "5 位置" in md
    assert "Caveats" in md or "caveats" in md
    assert "Meta probes" in md or "meta_probes" in md
    print("test_v1443_md_report_exists_and_valid OK")


def test_v1443_chain_delegate_shape():
    """chain_delegate output dict must have all_ok, module, version, evidence."""
    import apeireth.v1443_asi_v2_cross_position_interaction as v1443
    res = v1443.chain_delegate()
    assert "all_ok" in res
    assert "module" in res
    assert "version" in res
    assert "evidence" in res
    print("test_v1443_chain_delegate_shape OK")


def test_v1443_no_pretend_in_meta():
    """Meta JSON must include no-pretend caveats (V3 guards visible)."""
    p = _run_module("meta")
    out = json.loads(p.stdout)
    assert "GUARD_V2_IS_NOT_PHENOMENAL" in out["v3_guards"]
    assert "GUARD_V2_IS_NOT_ASI" in out["v3_guards"]
    assert "GUARD_V2_IS_NOT_RUNTIME" in out["v3_guards"]
    print("test_v1443_no_pretend_in_meta OK")


# ----------------------- runner -----------------------


def run_all_tests():
    """Run all V1443 tests."""
    import traceback
    tests = [
        ("test_version_cli", test_version_cli),
        ("test_help_cli", test_help_cli),
        ("test_popper_cli", test_popper_cli),
        ("test_chain_cli", test_chain_cli),
        ("test_list_pairs_cli", test_list_pairs_cli),
        ("test_probe_pair_cli", test_probe_pair_cli),
        ("test_probe_pair_unknown_source", test_probe_pair_unknown_source),
        ("test_probe_pair_self_loop", test_probe_pair_self_loop),
        ("test_probe_all_cli", test_probe_all_cli),
        ("test_audit_cli", test_audit_cli),
        ("test_report_cli", test_report_cli),
        ("test_run_all_cli", test_run_all_cli),
        ("test_meta_cli", test_meta_cli),
        ("test_unknown_cmd", test_unknown_cmd),
        ("test_position_constants_have_5_positions", test_position_constants_have_5_positions),
        ("test_v1443_direct_run_all", test_v1443_direct_run_all),
        ("test_v1443_meta_probes_have_no_self_loops", test_v1443_meta_probes_have_no_self_loops),
        ("test_v1443_round_trip_pass", test_v1443_round_trip_pass),
        ("test_v1443_chain_delegate_callable", test_v1443_chain_delegate_callable),
        ("test_v1443_chain_delegate_with_prev_false", test_v1443_chain_delegate_with_prev_false),
        ("test_v1443_probe_pair_data_structure", test_v1443_probe_pair_data_structure),
        ("test_v1443_module_id_safety", test_v1443_module_id_safety),
        ("test_v1443_helpers_clip01", test_v1443_helpers_clip01),
        ("test_v1443_helpers_safe_path", test_v1443_helpers_safe_path),
        ("test_v1443_guards_have_14", test_v1443_guards_have_14),
        ("test_v1443_v3_guards_have_5", test_v1443_v3_guards_have_5),
        ("test_v1443_borrowed_has_5", test_v1443_borrowed_has_5),
        ("test_v1443_md_report_exists_and_valid", test_v1443_md_report_exists_and_valid),
        ("test_v1443_chain_delegate_shape", test_v1443_chain_delegate_shape),
        ("test_v1443_no_pretend_in_meta", test_v1443_no_pretend_in_meta),
    ]
    passed = 0
    failed = 0
    failures: list = []
    for name, fn in tests:
        try:
            fn()
            passed += 1
        except Exception as exc:
            failed += 1
            failures.append((name, exc, traceback.format_exc()))
            print(f"FAIL {name}: {exc}")
    print(f"\n=== V1443 tests: {passed}/{passed+failed} passed ===")
    if failures:
        for name, exc, tb in failures:
            print(f"\n--- {name} ---\n{tb}")
    return failed == 0


if __name__ == "__main__":
    ok = run_all_tests()
    sys.exit(0 if ok else 1)
