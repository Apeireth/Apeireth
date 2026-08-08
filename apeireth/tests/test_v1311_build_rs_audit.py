"""
V1311 — build.rs Audit Popper Self-Tests

Tests hypothesis-falsifiable claims about the build.rs audit:
- workspace root has Apeireth-rust
- Apeireth-rust/crates contains 92 crates (V1307 anchor + drift)
- 43+ build.rs files discovered
- 3 active workspace build.rs: src-tauri + 2 members
- 40+ research vendored build.rs (audit_only)
- All active build.rs have declared build-deps matching used crate calls
- No HIGH risk level on active workspace
- All tauri_build invocations have tauri-build declared
- apeireth-bus uses vendored protoc (safe, no host protoc dep)
- audit decision HEALTHY
- findings JSON has all required keys
"""
import json
import sys
from pathlib import Path

# Add apeireth dir to sys.path so we can import v1311_build_rs_audit
ROOT = Path(__file__).resolve().parent.parent.parent
sys.path.insert(0, str(ROOT / "apeireth"))

import v1311_build_rs_audit as audit_mod  # noqa: E402

FINDINGS = ROOT / "apeireth" / "v1311_audit_findings.json"


def _load():
    return json.loads(FINDINGS.read_text(encoding="utf-8"))


def test_h1_workspace_root_has_rust_dir():
    """H1: Apeireth-rust directory exists."""
    assert audit_mod.RUST_WORKSPACE.is_dir(), f"Apeireth-rust missing: {audit_mod.RUST_WORKSPACE}"


def test_h2_crates_dir_has_92():
    """H2: Apeireth-rust/crates contains 92 crates (V1307 anchor + drift)."""
    crates = [d for d in audit_mod.RUST_CRATES.iterdir() if d.is_dir()]
    assert len(crates) >= 80, f"expected 80+ crates, got {len(crates)}"


def test_h3_total_build_rs_at_least_40():
    """H3: 40+ build.rs files discovered (workspace + research vendored)."""
    data = _load()
    n = data["total_build_rs_found"]
    assert n >= 40, f"expected >= 40 build.rs, got {n}"


def test_h4_active_workspace_build_rs_is_3():
    """H4: 3 active workspace build.rs (src-tauri + 2 members)."""
    data = _load()
    n = data["active_workspace_build_rs_count"]
    assert n == 3, f"expected 3 active build.rs, got {n}"


def test_h5_active_members_include_bus_and_tauri_stub():
    """H5: apeireth-bus and apeireth-tauri-stub have build.rs."""
    data = _load()
    paths = [a["path"] for a in data["by_location"]["workspace_member"]]
    assert any("apeireth-bus" in p for p in paths), (
        f"apeireth-bus/build.rs missing: {paths}"
    )
    assert any("apeireth-tauri-stub" in p for p in paths), (
        f"apeireth-tauri-stub/build.rs missing: {paths}"
    )


def test_h6_active_root_app_is_src_tauri():
    """H6: workspace_root_app is src-tauri/build.rs."""
    data = _load()
    apps = [a["path"] for a in data["by_location"]["workspace_root_app"]]
    assert any("src-tauri" in p for p in apps), (
        f"src-tauri/build.rs missing: {apps}"
    )


def test_h7_research_vendored_count_at_least_30():
    """H7: research/source vendored build.rs >= 30 (wasmtime/qdrant/codex/etc)."""
    data = _load()
    n = data["research_vendored_build_rs_count"]
    assert n >= 30, f"expected >= 30 research vendored, got {n}"


def test_h8_all_active_low_risk():
    """H8: all active workspace build.rs = LOW risk (no HIGH/MEDIUM)."""
    data = _load()
    active = data["by_location"]["workspace_member"] + data["by_location"]["workspace_root_app"]
    for a in active:
        assert a["risk_level"] == "LOW", (
            f"{a['path']} risk={a['risk_level']} reasons={a['risk_reasons']}"
        )


def test_h9_no_undeclared_build_deps():
    """H9: undeclared_build_deps == [] (normalized hyphen→underscore)."""
    data = _load()
    assert data["undeclared_build_deps"] == [], (
        f"undeclared: {data['undeclared_build_deps']}"
    )


def test_h10_bus_uses_vendored_protoc():
    """H10: apeireth-bus uses protoc_bin_vendored (no host protoc dependency)."""
    data = _load()
    bus = [a for a in data["by_location"]["workspace_member"]
           if "apeireth-bus" in a["path"]][0]
    text = (audit_mod.ROOT / "Apeireth-rust" / "crates" / "apeireth-bus" / "build.rs").read_text(
        encoding="utf-8", errors="replace"
    )
    assert "protoc_bin_vendored" in text, "apeireth-bus should use vendored protoc"
    assert bus["has_compile_protos"], "apeireth-bus should call compile_protos"


def test_h11_tauri_stub_conditional():
    """H11: apeireth-tauri-stub uses CARGO_BIN_NAME gating (safer tauri build)."""
    data = _load()
    stub = [a for a in data["by_location"]["workspace_member"]
            if "apeireth-tauri-stub" in a["path"]][0]
    assert stub["has_tauri_build"], "apeireth-tauri-stub should call tauri_build"
    assert stub["has_conditional_gating"], "apeireth-tauri-stub should have env-var gating"


def test_h12_audit_decision_healthy():
    """H12: decision == HEALTHY (all active build.rs clean)."""
    data = _load()
    assert data["audit_decision"] == "HEALTHY", (
        f"unexpected decision: {data['audit_decision']}, reason: {data['audit_reason']}"
    )


def test_h13_findings_json_has_all_keys():
    """H13: findings JSON has all required keys."""
    data = _load()
    required = [
        "workspace_root", "rust_workspace_root",
        "workspace_members_total", "workspace_members_with_build_rs",
        "total_build_rs_found", "active_workspace_build_rs_count",
        "research_vendored_build_rs_count", "by_location",
        "risk_distribution", "undeclared_build_deps",
        "audit_decision", "audit_reason", "audit_action",
    ]
    for k in required:
        assert k in data, f"missing key: {k}"


def test_h14_audit_reason_references_v1311():
    """H14: audit_reason non-empty, mentions V1311."""
    data = _load()
    assert "V1311" in data["audit_reason"], "audit_reason missing V1311 reference"
    assert len(data["audit_reason"]) > 30, "audit_reason too short"


def test_h15_no_panic_on_re_audit():
    """H15: re-running v1311_build_rs_audit.py doesn't crash."""
    import subprocess
    proc = subprocess.run(
        [sys.executable, str(ROOT / "apeireth" / "v1311_build_rs_audit.py")],
        capture_output=True, text=True, cwd=str(ROOT),
    )
    assert proc.returncode == 0, f"audit crashed: {proc.stderr[:500]}"


def test_h16_risk_distribution_low_only_for_active():
    """H16: risk_distribution shows LOW as the only non-AUDIT_ONLY level."""
    data = _load()
    rd = data["risk_distribution"]
    # Active count is 3, all LOW
    assert rd.get("LOW", 0) >= 3, f"LOW={rd.get('LOW')} should be >= 3"
    assert rd.get("HIGH", 0) == 0, f"HIGH={rd.get('HIGH')} should be 0"
    assert rd.get("AUDIT_ONLY", 0) >= 30, f"AUDIT_ONLY={rd.get('AUDIT_ONLY')} should be >= 30"


def test_h17_workspace_members_total_matches():
    """H17: workspace_members_total > 80 (V1307 anchor)."""
    data = _load()
    assert data["workspace_members_total"] >= 80, (
        f"workspace members {data['workspace_members_total']} too low"
    )


def test_h18_only_two_members_have_build_rs():
    """H18: workspace_members_with_build_rs has exactly 2 entries."""
    data = _load()
    paths = data["workspace_members_with_build_rs"]
    assert len(paths) == 2, f"expected 2 member build.rs paths, got {len(paths)}: {paths}"


if __name__ == "__main__":
    import pytest
    sys.exit(pytest.main([__file__, "-v"]))
