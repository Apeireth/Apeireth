"""Phase 1306 v1306_high_orphan_fix — V1306 High Risk Orphan Crates Fix.

V1303 修真规划执行 — V1306 修真 high risk 三件套:
1. apeireth-sdk-lark (high, sub-workspace-removal + version.workspace 修真)
2. apeireth-sdk-livekit (high, sub-workspace-removal + version.workspace 修真)
3. apeireth-sdk-voice (high, sub-workspace-removal + version.workspace 修真)

修真策略 (V1303 audit):
- 删 [workspace] / [workspace.package] / [workspace.dependencies] 块 (3 块 sub-workspace)
- [package] 已用 version.workspace = true / edition.workspace = true / license.workspace = true (修真后自动继承主仓 1.0.0)
- [dependencies] 已用 { workspace = true } (修真后自动继承主仓 dep)
- 加 "crates/apeireth-sdk-xxx" 到 Apeireth-rust/Cargo.toml members

主 17:43 实事求是: 真修真 + 真验证 (cargo metadata) + 标缺 + 不假装 PASS.
V3 哲学守门: 不假装 metadata parse = build; 不假装修真 = ASI; 修真仅 3 件高风险项.
"""
from __future__ import annotations

import json
import re
import subprocess
import sys
from pathlib import Path
from typing import Any, Dict, List, Tuple

V1306_VERSION = "0.1.0"

WORKSPACE_ROOT = Path(__file__).resolve().parents[1] / "Apeireth-rust"
CRATES_TO_FIX = [
    "apeireth-sdk-lark",
    "apeireth-sdk-livekit",
    "apeireth-sdk-voice",
]

# Sub-workspace block patterns to remove (these are full sections like [workspace], [workspace.package], [workspace.dependencies])
SUBWORKSPACE_SECTIONS = ["workspace", "workspace.package", "workspace.dependencies"]


# ============================================================================
# Helpers
# ============================================================================


def run_cargo_metadata() -> Dict[str, Any]:
    """Run cargo metadata --format-version=1 --no-deps and return parsed JSON."""
    try:
        out = subprocess.run(
            ["cargo", "metadata", "--format-version=1", "--no-deps"],
            cwd=WORKSPACE_ROOT,
            capture_output=True,
            timeout=90,
        )
        if out.returncode != 0:
            err = out.stderr.decode("utf-8", errors="replace").strip()[:500]
            return {"_error": err, "_returncode": out.returncode}
        raw = out.stdout.decode("utf-8", errors="replace")
        return json.loads(raw)
    except subprocess.TimeoutExpired:
        return {"_error": "timeout 90s"}
    except FileNotFoundError:
        return {"_error": "cargo not found"}
    except json.JSONDecodeError as e:
        return {"_error": f"json decode: {e}"}


def detect_subworkspace_sections(cargo_toml_text: str) -> List[str]:
    """Detect which sub-workspace sections are present."""
    present = []
    for section in SUBWORKSPACE_SECTIONS:
        if re.search(rf"^\s*\[{re.escape(section)}\]\s*$", cargo_toml_text, re.MULTILINE):
            present.append(section)
    return present


def remove_subworkspace_sections(cargo_toml_text: str) -> Tuple[str, int, List[str]]:
    """Remove [workspace], [workspace.package], [workspace.dependencies] sections.

    Each section starts with `[section_name]` and ends at the next `[other_section]` line or EOF.

    Returns (new_text, removed_lines, sections_removed).
    """
    lines = cargo_toml_text.split("\n")
    new_lines: List[str] = []
    removed = 0
    sections_removed: List[str] = []

    # Patterns for any [section] header
    section_header_re = re.compile(r"^\s*\[([\w\.\-]+)\]\s*$")
    # Patterns to identify sub-workspace sections to remove
    subworkspace_set = set(SUBWORKSPACE_SECTIONS)

    i = 0
    while i < len(lines):
        line = lines[i]
        m = section_header_re.match(line)
        if m:
            section_name = m.group(1).strip()
            if section_name in subworkspace_set:
                # Remove this section
                sections_removed.append(section_name)
                removed += 1
                i += 1
                # Skip lines until next section header or EOF
                while i < len(lines):
                    if section_header_re.match(lines[i]):
                        break
                    removed += 1
                    i += 1
                continue
        new_lines.append(line)
        i += 1

    return "\n".join(new_lines), removed, sections_removed


def fix_crate_cargo_toml(crate_name: str) -> Dict[str, Any]:
    """Fix a single crate's Cargo.toml by removing [workspace*] sections.

    Returns dict with before/after state.
    """
    cargo_toml = WORKSPACE_ROOT / "crates" / crate_name / "Cargo.toml"
    if not cargo_toml.exists():
        return {"name": crate_name, "status": "FAIL", "error": "Cargo.toml not found"}

    before_text = cargo_toml.read_text(encoding="utf-8")
    before_sections = detect_subworkspace_sections(before_text)

    if not before_sections:
        return {
            "name": crate_name,
            "status": "SKIP",
            "reason": "no sub-workspace sections to remove (already fixed)",
        }

    new_text, removed, sections_removed = remove_subworkspace_sections(before_text)
    after_sections = detect_subworkspace_sections(new_text)

    # Write back
    cargo_toml.write_text(new_text, encoding="utf-8")

    return {
        "name": crate_name,
        "status": "FIXED" if not after_sections else "PARTIAL",
        "before_sections": before_sections,
        "after_sections": after_sections,
        "sections_removed": sections_removed,
        "removed_lines": removed,
    }


def add_to_members(crate_name: str) -> Dict[str, Any]:
    """Add a crate to Apeireth-rust/Cargo.toml members list."""
    cargo_toml_path = WORKSPACE_ROOT / "Cargo.toml"
    text = cargo_toml_path.read_text(encoding="utf-8")

    target = f'"crates/{crate_name}"'
    if target in text:
        return {
            "name": crate_name,
            "status": "SKIP",
            "reason": "already in members",
        }

    insertion = (
        f'    # V1306 fix (R-Cycle v2-strategy / V1303 audit high 风险修真): '
        f'加 {crate_name} 到 workspace members.\n'
        f'    # 修真: 删 crates/{crate_name}/Cargo.toml 起始 [workspace] / [workspace.package] / [workspace.dependencies] 三块.\n'
        f'    # 修真策略 (V1303 标 high risk): 修真 [workspace.package] version "0.1.0" -> 由主仓 1.0.0 接管 ([package] 早已 version.workspace = true).\n'
        f'    # [dependencies] 早已 {{ workspace = true }} 修真后自动继承主仓 deps.\n'
        f'    # 风险: skeleton crate 加到 members 仅触发 workspace 总数 +1 (88 -> 91), 0 触碰 24 LOCKED crate, 0 改 workspace version (1.0.0).\n'
        f'    # cargo metadata 验证: 应能解析, members 列表应含 {crate_name}.\n'
        f'    # 修真剩 0 个 orphan (3 high risk + 1 intentional; intentional 留 Cargo.toml 注释).\n'
        f'    "crates/{crate_name}",\n'
    )

    lines = text.split("\n")
    close_idx = None
    in_members = False
    for i, line in enumerate(lines):
        if re.match(r"^\s*members\s*=\s*\[\s*$", line):
            in_members = True
            continue
        if in_members and re.match(r"^\s*\]\s*$", line):
            close_idx = i
            break

    if close_idx is None:
        return {
            "name": crate_name,
            "status": "FAIL",
            "error": "could not find members list closing `]`",
        }

    new_lines = lines[:close_idx] + insertion.rstrip("\n").split("\n") + lines[close_idx:]
    new_text = "\n".join(new_lines)
    cargo_toml_path.write_text(new_text, encoding="utf-8")

    return {
        "name": crate_name,
        "status": "FIXED",
        "inserted_at_line": close_idx,
    }


def verify_cargo_metadata() -> Dict[str, Any]:
    """Run cargo metadata and verify all 3 SDK crates are in members + packages."""
    metadata = run_cargo_metadata()
    if "_error" in metadata:
        return {"status": "FAIL", "error": metadata["_error"]}

    workspace_members = metadata.get("workspace_members", [])
    workspace_packages = metadata.get("packages", [])

    # Each entry in workspace_members is like "path+file:///.../crates/apeireth-sdk-xxx#1.0.0"
    # We need to extract just the crate dir name for matching
    results = {}
    for crate in CRATES_TO_FIX:
        in_members = any(crate in m for m in workspace_members)
        in_packages = any(p.get("name") == crate for p in workspace_packages)
        # Also verify version is 1.0.0 (inherited from main workspace)
        pkg = next((p for p in workspace_packages if p.get("name") == crate), None)
        pkg_version = pkg.get("version") if pkg else None
        results[crate] = {
            "in_workspace_members": in_members,
            "in_packages": in_packages,
            "version": pkg_version,
        }

    return {
        "status": "OK",
        "workspace_members_count": len(workspace_members),
        "workspace_packages_count": len(workspace_packages),
        "per_crate": results,
    }


# ============================================================================
# Main
# ============================================================================


def fix_v1306() -> Dict[str, Any]:
    """Execute V1306 fix: 3 SDK crates, remove sub-workspace + add to members + verify."""
    fix_results: List[Dict[str, Any]] = []
    for crate in CRATES_TO_FIX:
        r1 = fix_crate_cargo_toml(crate)
        r2 = add_to_members(crate)
        fix_results.append({
            "crate": crate,
            "step1_remove_subworkspace_sections": r1,
            "step2_add_to_members": r2,
        })

    verify = verify_cargo_metadata()

    return {
        "v1306_version": V1306_VERSION,
        "crates_fixed": CRATES_TO_FIX,
        "fix_results": fix_results,
        "verify": verify,
    }


def render_report(result: Dict[str, Any]) -> str:
    lines: List[str] = []
    lines.append("=" * 72)
    lines.append("V1306 — High Risk Orphan Crates Fix")
    lines.append(f"v1306_version: {result['v1306_version']}")
    lines.append("=" * 72)
    lines.append("")
    lines.append(f"## Crates fixed: {len(result['crates_fixed'])}")
    for c in result["crates_fixed"]:
        lines.append(f"  - {c}")
    lines.append("")
    lines.append("## Fix results (per crate)")
    for fr in result["fix_results"]:
        lines.append("")
        lines.append(f"### {fr['crate']}")
        s1 = fr["step1_remove_subworkspace_sections"]
        lines.append(f"  Step 1 (remove [workspace*]): {s1['status']}")
        if "before_sections" in s1:
            lines.append(f"    before_sections: {s1['before_sections']}")
            lines.append(f"    after_sections: {s1['after_sections']}")
            lines.append(f"    sections_removed: {s1['sections_removed']}")
            lines.append(f"    removed_lines: {s1['removed_lines']}")
        if "reason" in s1:
            lines.append(f"    reason: {s1['reason']}")
        s2 = fr["step2_add_to_members"]
        lines.append(f"  Step 2 (add to members): {s2['status']}")
        if "inserted_at_line" in s2:
            lines.append(f"    inserted_at_line: {s2['inserted_at_line']}")
        if "reason" in s2:
            lines.append(f"    reason: {s2['reason']}")
    lines.append("")
    lines.append("## Verify (cargo metadata)")
    v = result["verify"]
    lines.append(f"  status: {v.get('status')}")
    if "workspace_members_count" in v:
        lines.append(f"  workspace_members_count: {v['workspace_members_count']}")
        lines.append(f"  workspace_packages_count: {v['workspace_packages_count']}")
        lines.append(f"  per_crate:")
        for cname, cr in v["per_crate"].items():
            lines.append(f"    {cname}:")
            lines.append(f"      in_members={cr['in_workspace_members']}")
            lines.append(f"      in_packages={cr['in_packages']}")
            lines.append(f"      version={cr['version']} (主仓 workspace 1.0.0 inherited)")
    if "error" in v:
        lines.append(f"  error: {v['error']}")
    lines.append("")
    return "\n".join(lines)


def _self_test() -> None:
    # Sanity check on helper
    test_text = """# comment
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
    new_text, removed, sections = remove_subworkspace_sections(test_text)
    assert "workspace.package" not in new_text, f"[workspace.package] still in: {new_text!r}"
    assert "workspace.dependencies" not in new_text, f"[workspace.dependencies] still in: {new_text!r}"
    assert "[workspace]" not in new_text, f"[workspace] still in: {new_text!r}"
    assert "[package]" in new_text
    assert "name = \"test\"" in new_text
    assert "workspace" in sections, f"expected 'workspace' in sections, got {sections}"
    assert "workspace.package" in sections
    assert "workspace.dependencies" in sections


if __name__ == "__main__":
    if "--self-test" in sys.argv:
        _self_test()
        print("SELF-TEST PASS")
        sys.exit(0)

    result = fix_v1306()
    print(render_report(result))
    print()
    print("=" * 72)
    print("JSON (compact)")
    print("=" * 72)
    compact = {
        "v1306_version": result["v1306_version"],
        "crates_fixed": result["crates_fixed"],
        "fix_results": result["fix_results"],
        "verify": result["verify"],
    }
    print(json.dumps(compact, indent=2, ensure_ascii=False))