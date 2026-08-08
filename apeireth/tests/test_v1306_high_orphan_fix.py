"""Tests for V1306 high risk orphan crates fix (主 17:43 实事求是 + 主 00:44 质量工程化 + 主 00:56 任何人都能接手).

These tests verify V1306 真修真:
- 3 high-risk crates (apeireth-sdk-lark / apeireth-sdk-livekit / apeireth-sdk-voice) have NO [workspace*] sections
  (each had independent [workspace] / [workspace.package] / [workspace.dependencies] blocks that conflict
   with main workspace version 1.0.0 — V1303 audit high risk).
- 3 crates appear in Apeireth-rust/Cargo.toml members list
- cargo metadata --no-deps parses successfully
- 3 crates appear in workspace_members AND packages list with version 1.0.0 (inherited from main)
- workspace_members count is 91 (88 baseline from V1305 + 3 new)
- V1303 sub-workspace-removal pattern detected before/after (regex on Cargo.toml text)
"""
from __future__ import annotations

import json
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
    from apeireth import v1306_high_orphan_fix as v06
except Exception:
    sys.path.insert(0, str(APEIRETH_PKG_PARENT))
    import v1306_high_orphan_fix as v06


V1306_CRATES = [
    "apeireth-sdk-lark",
    "apeireth-sdk-livekit",
    "apeireth-sdk-voice",
]

V1306_SUBWORKSPACE_SECTIONS = ["workspace", "workspace.package", "workspace.dependencies"]


def _read_text(p: Path) -> str:
    try:
        return p.read_text(encoding="utf-8")
    except FileNotFoundError:
        return ""


def _detect_subworkspace_sections(cargo_toml_text: str) -> List[str]:
    """Detect which [workspace*] sections are present (mirrors script helper)."""
    present = []
    for section in V1306_SUBWORKSPACE_SECTIONS:
        if re.search(rf"^\s*\[{re.escape(section)}\]\s*$", cargo_toml_text, re.MULTILINE):
            present.append(section)
    return present


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
# Version + sanity (helpers from script)
# ============================================================================


def test_v1306_version():
    assert v06.V1306_VERSION == "0.1.0"


def test_v1306_helper_detect_subworkspace_sections():
    """Helper function: detect which [workspace*] sections are present."""
    text_with_all = (
        "[workspace]\n"
        "resolver = \"2\"\n\n"
        "[workspace.package]\n"
        "version = \"0.1.0\"\n\n"
        "[workspace.dependencies]\n"
        "tokio = \"1.40\"\n\n"
        "[package]\n"
        "name = \"test\"\n"
    )
    detected = _detect_subworkspace_sections(text_with_all)
    assert "workspace" in detected
    assert "workspace.package" in detected
    assert "workspace.dependencies" in detected

    text_clean = "[package]\nname = \"x\"\n"
    assert _detect_subworkspace_sections(text_clean) == []


def test_v1306_helper_remove_subworkspace_sections():
    """Helper function: remove [workspace*] sections from Cargo.toml text."""
    sample = """# header comment
[workspace]
resolver = "2"

[workspace.package]
version = "0.1.0"
edition = "2021"

[workspace.dependencies]
tokio = "1.40"

[package]
name = "test"
version.workspace = true
"""
    new_text, removed, sections = v06.remove_subworkspace_sections(sample)
    # All sub-workspace sections must be gone
    assert "workspace.package" not in new_text, f"[workspace.package] still in: {new_text!r}"
    assert "workspace.dependencies" not in new_text, f"[workspace.dependencies] still in: {new_text!r}"
    # [workspace] header alone: re.search for ^[workspace]$ — the regex pattern is at line start
    # We expect no [workspace] section header at column 0 either
    assert not re.search(r"^\[workspace\]\s*$", new_text, re.MULTILINE), \
        f"[workspace] header still in: {new_text!r}"
    # [package] must remain
    assert "[package]" in new_text
    assert 'name = "test"' in new_text
    # removed lines should be > 0
    assert removed >= 8, f"expected >=8 removed lines, got {removed}"
    # sections list must contain all three
    assert "workspace" in sections
    assert "workspace.package" in sections
    assert "workspace.dependencies" in sections


# ============================================================================
# Per-crate fix verification (state on disk)
# ============================================================================


@pytest.mark.parametrize("crate_name", V1306_CRATES)
def test_v1306_crate_no_subworkspace_sections(crate_name: str):
    """Each fixed high-risk crate has NO [workspace*] sections in its Cargo.toml."""
    cargo_toml = WORKSPACE_ROOT / "crates" / crate_name / "Cargo.toml"
    assert cargo_toml.exists(), f"Cargo.toml missing: {cargo_toml}"
    text = _read_text(cargo_toml)
    present = _detect_subworkspace_sections(text)
    assert present == [], (
        f"[workspace*] sections still present in {crate_name}/Cargo.toml: {present}"
    )


@pytest.mark.parametrize("crate_name", V1306_CRATES)
def test_v1306_crate_in_members_list(crate_name: str):
    """Each fixed crate appears in Apeireth-rust/Cargo.toml members list."""
    cargo_toml = WORKSPACE_ROOT / "Cargo.toml"
    text = _read_text(cargo_toml)
    target = f'"crates/{crate_name}"'
    assert target in text, f"{crate_name} not in workspace members"


@pytest.mark.parametrize("crate_name", V1306_CRATES)
def test_v1306_crate_package_section_intact(crate_name: str):
    """Each fixed crate's [package] section is intact (version.workspace=true, etc)."""
    cargo_toml = WORKSPACE_ROOT / "crates" / crate_name / "Cargo.toml"
    text = _read_text(cargo_toml)
    # [package] block must exist
    assert "[package]" in text
    # name = "<crate>" must exist
    assert f'name = "{crate_name}"' in text
    # version.workspace = true must exist (inherited from main workspace = 1.0.0)
    assert re.search(r'^\s*version\.workspace\s*=\s*true\s*$', text, re.MULTILINE), \
        f"version.workspace = true missing in {crate_name}/Cargo.toml"
    # edition.workspace = true must exist
    assert re.search(r'^\s*edition\.workspace\s*=\s*true\s*$', text, re.MULTILINE), \
        f"edition.workspace = true missing in {crate_name}/Cargo.toml"


# ============================================================================
# cargo metadata verification
# ============================================================================


def test_v1306_cargo_metadata_parses():
    """cargo metadata --format-version=1 --no-deps parses successfully."""
    metadata = _run_cargo_metadata()
    assert "_error" not in metadata, f"cargo metadata failed: {metadata.get('_error')}"
    assert "workspace_members" in metadata
    assert "packages" in metadata


def test_v1306_cargo_metadata_members_count_at_least_91():
    """workspace_members count >= 91 (88 baseline from V1305 + 3 new in V1306)."""
    metadata = _run_cargo_metadata()
    assert "_error" not in metadata
    n = len(metadata["workspace_members"])
    assert n >= 91, f"expected >=91 members, got {n}"


@pytest.mark.parametrize("crate_name", V1306_CRATES)
def test_v1306_crate_in_workspace_members(crate_name: str):
    """Each fixed crate is in cargo metadata workspace_members (substring match on URL)."""
    metadata = _run_cargo_metadata()
    assert "_error" not in metadata
    members = metadata["workspace_members"]
    # workspace_members are URLs like 'path+file:///...crates/apeireth-xxx#1.0.0'
    found = any(crate_name in m for m in members)
    assert found, f"{crate_name} not in workspace_members. members sample: {members[:3]}"


@pytest.mark.parametrize("crate_name", V1306_CRATES)
def test_v1306_crate_in_packages_list_with_version_1_0_0(crate_name: str):
    """Each fixed crate appears in cargo metadata packages list with version 1.0.0."""
    metadata = _run_cargo_metadata()
    assert "_error" not in metadata
    packages = metadata["packages"]
    found = next((p for p in packages if p.get("name") == crate_name), None)
    assert found is not None, f"{crate_name} not in packages list"
    assert found.get("version") == "1.0.0", (
        f"{crate_name} version is {found.get('version')}, expected 1.0.0 (inherited from main workspace)"
    )


# ============================================================================
# Popper hypotheses summary (mimics V1306 fix script's own self-check)
# ============================================================================


def test_v1306_hypotheses_summary_all_pass():
    """Summary of all V1306 hypotheses PASS."""
    metadata = _run_cargo_metadata()
    assert "_error" not in metadata

    n_members = len(metadata["workspace_members"])
    n_packages = len(metadata["packages"])
    packages = metadata["packages"]
    members = metadata["workspace_members"]

    # H1: all 3 SDK crates in workspace_members
    all_in_members = all(any(c in m for m in members) for c in V1306_CRATES)
    # H2: all 3 SDK crates in packages
    all_in_packages = all(any(p.get("name") == c for p in packages) for c in V1306_CRATES)
    # H3: members count >= 91 (88 V1305 baseline + 3 new)
    h_members_ok = n_members >= 91
    # H4: packages count == members count
    h_equal = n_members == n_packages
    # H5: no sub-workspace block in any of the 3 fixed crates
    h_no_subworkspace = True
    for c in V1306_CRATES:
        cargo_toml = WORKSPACE_ROOT / "crates" / c / "Cargo.toml"
        if _detect_subworkspace_sections(_read_text(cargo_toml)):
            h_no_subworkspace = False
            break
    # H6: all 3 fixed crates have version 1.0.0 (inherited from main workspace)
    h_version_inherited = all(
        next((p for p in packages if p.get("name") == c), {}).get("version") == "1.0.0"
        for c in V1306_CRATES
    )

    assert all_in_members, "h_all_in_members: FAIL"
    assert all_in_packages, "h_all_in_packages: FAIL"
    assert h_members_ok, f"h_members_ok: FAIL (n_members={n_members})"
    assert h_equal, f"h_equal: FAIL (members={n_members}, packages={n_packages})"
    assert h_no_subworkspace, "h_no_subworkspace: FAIL"
    assert h_version_inherited, "h_version_inherited: FAIL (one or more crates not 1.0.0)"


# ============================================================================
# V3 philosophy gate (主 17:58 + 主 20:46 + 主 17:43)
# ============================================================================


def test_v1306_v3_philosophy_guards():
    """V3 philosophical guards must be present in V1306 fix module."""
    src = _read_text(APEIRETH_PKG_PARENT / "v1306_high_orphan_fix.py")
    assert "实事求是" in src
    assert "不假装" in src or "V3 哲学守门" in src
    # V1306 explicitly marks remaining 1 intentional orphan
    assert "intentional" in src.lower() or "tauri-stub" in src


def test_v1306_marker_marks_intentional_orphan():
    """V1306 must explicitly mark remaining 1 intentional orphan in script."""
    # Verify the script acknowledges the remaining intentional orphan (apeireth-tauri-stub)
    # even though the script itself only fixes 3 SDK crates.
    src = _read_text(APEIRETH_PKG_PARENT / "v1306_high_orphan_fix.py")
    assert "intentional" in src.lower(), \
        "v1306 script should mark remaining intentional orphan explicitly"
    # Also verify it's mentioned in main workspace Cargo.toml members (commented out, intentional)
    workspace_toml = _read_text(WORKSPACE_ROOT / "Cargo.toml")
    assert "tauri-stub" in workspace_toml, \
        "apeireth-tauri-stub must remain commented in main Cargo.toml members (V1301 intentional)"


if __name__ == "__main__":
    # Allow direct invocation: python test_v1306_high_orphan_fix.py
    pytest.main([__file__, "-v"])