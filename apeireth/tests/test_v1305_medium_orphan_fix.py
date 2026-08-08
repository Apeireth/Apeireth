"""Tests for V1305 medium risk orphan crates fix (主 17:43 实事求是 + 主 00:44 质量工程化 + 主 00:56 任何人都能接手).

These tests verify V1305 真修真:
- 3 crates (integration-e2e / integration-r20-stage4 / rate-limiter) have no [workspace] block in their Cargo.toml
- 3 crates are in Apeireth-rust/Cargo.toml members list
- cargo metadata --no-deps parses successfully
- 3 crates appear in workspace_members AND packages list
- workspace_members count is 88 (85 baseline + 3 new)
- Per-crate sub-workspace-removal pattern detected before/after
"""
from __future__ import annotations

import json
import os
import re
import subprocess
import sys
from pathlib import Path
from typing import Any, Dict, List

import pytest

# Ensure in-package import works
APEIRETH_PKG_PARENT = Path(__file__).resolve().parents[1]
WORKSPACE_ROOT = APEIRETH_PKG_PARENT.parent / "Apeireth-rust"

# Try both import styles
try:
    from apeireth import v1305_medium_orphan_fix as v05
except Exception:
    sys.path.insert(0, str(APEIRETH_PKG_PARENT))
    import v1305_medium_orphan_fix as v05


V1305_CRATES = [
    "apeireth-integration-e2e",
    "apeireth-integration-r20-stage4",
    "apeireth-rate-limiter",
]


def _read_text(p: Path) -> str:
    try:
        return p.read_text(encoding="utf-8")
    except FileNotFoundError:
        return ""


def _detect_subworkspace(cargo_toml_text: str) -> bool:
    return bool(re.search(r"^\s*\[workspace\]\s*$", cargo_toml_text, re.MULTILINE))


def _run_cargo_metadata() -> Dict[str, Any]:
    """Run cargo metadata --format-version=1 --no-deps and return parsed JSON."""
    try:
        out = subprocess.run(
            ["cargo", "metadata", "--format-version=1", "--no-deps"],
            cwd=str(WORKSPACE_ROOT),
            capture_output=True,
            timeout=90,
        )
        if out.returncode != 0:
            return {"_error": out.stderr.decode("utf-8", errors="replace").strip()[:500]}
        raw = out.stdout.decode("utf-8", errors="replace")
        return json.loads(raw)
    except subprocess.TimeoutExpired:
        return {"_error": "timeout 90s"}
    except FileNotFoundError:
        return {"_error": "cargo not found"}
    except json.JSONDecodeError as e:
        return {"_error": f"json decode: {e}"}


# ============================================================================
# Version + sanity
# ============================================================================


def test_v1305_version():
    assert v05.V1305_VERSION == "0.1.0"


def test_v1305_helper_remove_subworkspace():
    """Helper function: detect + remove [workspace] block from Cargo.toml text."""
    sample = """[workspace]
# comment 1
# comment 2

[package]
name = "test"
version = "0.1.0"
"""
    new_text, removed = v05.remove_subworkspace_block(sample)
    assert "[workspace]" not in new_text, f"[workspace] still in: {new_text!r}"
    assert "[package]" in new_text
    assert "name = \"test\"" in new_text
    assert removed >= 4, f"expected >=4 removed lines, got {removed}"


def test_v1305_helper_detect_subworkspace():
    """Helper function: detect [workspace] block in text."""
    with_ws = "[workspace]\nfoo = []\n[package]\nname = \"x\"\n"
    without_ws = "[package]\nname = \"x\"\n"
    assert _detect_subworkspace(with_ws)
    assert not _detect_subworkspace(without_ws)


# ============================================================================
# Per-crate fix verification (state on disk)
# ============================================================================


@pytest.mark.parametrize("crate_name", V1305_CRATES)
def test_v1305_crate_no_subworkspace(crate_name: str):
    """Each fixed crate has no [workspace] block in its Cargo.toml."""
    cargo_toml = WORKSPACE_ROOT / "crates" / crate_name / "Cargo.toml"
    assert cargo_toml.exists(), f"Cargo.toml missing: {cargo_toml}"
    text = _read_text(cargo_toml)
    assert not _detect_subworkspace(text), (
        f"[workspace] block still present in {crate_name}/Cargo.toml after V1305 fix"
    )


@pytest.mark.parametrize("crate_name", V1305_CRATES)
def test_v1305_crate_in_members_list(crate_name: str):
    """Each fixed crate appears in Apeireth-rust/Cargo.toml members list."""
    cargo_toml = WORKSPACE_ROOT / "Cargo.toml"
    text = _read_text(cargo_toml)
    target = f'"crates/{crate_name}"'
    assert target in text, f"{crate_name} not in workspace members"


@pytest.mark.parametrize("crate_name", V1305_CRATES)
def test_v1305_crate_package_section_intact(crate_name: str):
    """Each fixed crate's [package] section is intact (version 1.0.0, edition 2021)."""
    cargo_toml = WORKSPACE_ROOT / "crates" / crate_name / "Cargo.toml"
    text = _read_text(cargo_toml)
    # [package] block must exist
    assert "[package]" in text
    # name = "<crate>" must exist
    assert f'name = "{crate_name}"' in text
    # version = "1.0.0" must exist (matches workspace)
    assert re.search(r'^\s*version\s*=\s*"1\.0\.0"\s*$', text, re.MULTILINE)
    # edition = "2021" must exist
    assert re.search(r'^\s*edition\s*=\s*"2021"\s*$', text, re.MULTILINE)


# ============================================================================
# cargo metadata verification
# ============================================================================


def test_v1305_cargo_metadata_parses():
    """cargo metadata --format-version=1 --no-deps parses successfully."""
    metadata = _run_cargo_metadata()
    assert "_error" not in metadata, f"cargo metadata failed: {metadata.get('_error')}"
    assert "workspace_members" in metadata
    assert "packages" in metadata


def test_v1305_cargo_metadata_members_count_at_least_88():
    """workspace_members count >= 88 (85 baseline + 3 new in V1305)."""
    metadata = _run_cargo_metadata()
    assert "_error" not in metadata
    n = len(metadata["workspace_members"])
    assert n >= 88, f"expected >=88 members, got {n}"


@pytest.mark.parametrize("crate_name", V1305_CRATES)
def test_v1305_crate_in_workspace_members(crate_name: str):
    """Each fixed crate is in cargo metadata workspace_members (substring match on URL)."""
    metadata = _run_cargo_metadata()
    assert "_error" not in metadata
    members = metadata["workspace_members"]
    # workspace_members are URLs like 'path+file:///...crates/apeireth-xxx#1.0.0'
    found = any(crate_name in m for m in members)
    assert found, f"{crate_name} not in workspace_members. members: {members}"


@pytest.mark.parametrize("crate_name", V1305_CRATES)
def test_v1305_crate_in_packages_list(crate_name: str):
    """Each fixed crate appears in cargo metadata packages list (by package name)."""
    metadata = _run_cargo_metadata()
    assert "_error" not in metadata
    packages = metadata["packages"]
    found = any(p.get("name") == crate_name for p in packages)
    assert found, f"{crate_name} not in packages list"


# ============================================================================
# Popper hypotheses (mimics V1305 fix script's own self-check)
# ============================================================================


def test_v1305_hypotheses_summary():
    """Summary of all V1305 hypotheses PASS."""
    # Re-run the fix script's verify to get the same summary
    metadata = _run_cargo_metadata()
    assert "_error" not in metadata

    n_members = len(metadata["workspace_members"])
    n_packages = len(metadata["packages"])
    packages = metadata["packages"]
    members = metadata["workspace_members"]

    # H1: all 3 crates in workspace_members
    all_in_members = all(any(c in m for m in members) for c in V1305_CRATES)
    # H2: all 3 crates in packages
    all_in_packages = all(any(p.get("name") == c for p in packages) for c in V1305_CRATES)
    # H3: members count >= 88
    h_members_ok = n_members >= 88
    # H4: packages count == members count (every member is a package)
    h_equal = n_members == n_packages
    # H5: no sub-workspace block in any of the 3 fixed crates
    h_no_subworkspace = True
    for c in V1305_CRATES:
        cargo_toml = WORKSPACE_ROOT / "crates" / c / "Cargo.toml"
        if _detect_subworkspace(_read_text(cargo_toml)):
            h_no_subworkspace = False
            break

    assert all_in_members, "h_all_in_members: FAIL"
    assert all_in_packages, "h_all_in_packages: FAIL"
    assert h_members_ok, f"h_members_ok: FAIL (n_members={n_members})"
    assert h_equal, f"h_equal: FAIL (members={n_members}, packages={n_packages})"
    assert h_no_subworkspace, "h_no_subworkspace: FAIL"


# ============================================================================
# V3 philosophy gate (主 17:58 + 主 20:46 + 主 17:43)
# ============================================================================


def test_v1305_v3_philosophy_guards():
    """V3 philosophical guards must be present in V1305 fix module."""
    # The fix module itself should not pretend:
    # - no Phenomenal consciousness claim
    # - no ASI achievement claim
    # - 实事求是 (mark remaining orphans for V1306+)
    src = _read_text(APEIRETH_PKG_PARENT / "v1305_medium_orphan_fix.py")
    assert "实事求是" in src
    assert "不假装" in src or "不假装" in src or "V3 哲学守门" in src


if __name__ == "__main__":
    # Allow direct invocation: python test_v1305_medium_orphan_fix.py
    pytest.main([__file__, "-v"])