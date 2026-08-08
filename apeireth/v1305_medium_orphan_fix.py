"""Phase 1305 v1305_medium_orphan_fix — V1305 Medium Risk Orphan Crates Fix.

V1303 修真规划执行 — V1304 已修真 low risk (sdk-sandbox), V1305 修真 medium risk 三件套:
1. apeireth-integration-e2e (medium, sub-workspace-removal)
2. apeireth-integration-r20-stage4 (medium, sub-workspace-removal)
3. apeireth-rate-limiter (medium, sub-workspace-removal)

修真策略 (V1303 audit):
- 删 [workspace] 块 (空 sub-workspace table)
- 加 "crates/apeireth-xxx" 到 Apeireth-rust/Cargo.toml members
- 0 改 [package] / 0 改 deps / 0 改 dev-deps (version = "1.0.0" matches workspace, edition 2021 matches workspace, 等)
- cargo metadata 验证 members 数 85 -> 88

主 17:43 实事求是: 真修真 + 真验证 (cargo metadata) + 标缺 + 不假装 PASS.
主 13:08 真自问: medium risk 真修真 3 件套 vs 修真 1 件 — V1304 修真 low 1 件安全, V1305 medium 3 件同 batch 真修真.
V3 哲学守门: 不假装 metadata parse = build; 不假装修真 = ASI; 修真仅 3 件中风险项.
"""
from __future__ import annotations

import json
import re
import subprocess
import sys
from pathlib import Path
from typing import Any, Dict, List, Tuple

V1305_VERSION = "0.1.0"

WORKSPACE_ROOT = Path(__file__).resolve().parents[1] / "Apeireth-rust"
CRATES_TO_FIX = [
    "apeireth-integration-e2e",
    "apeireth-integration-r20-stage4",
    "apeireth-rate-limiter",
]


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


def parse_active_members(cargo_toml_path: Path) -> List[str]:
    """Parse Cargo.toml members list (returns full path string)."""
    text = cargo_toml_path.read_text(encoding="utf-8")
    return re.findall(r'^\s*"crates/([^"]+)"\s*,?\s*$', text, re.MULTILINE)


def detect_subworkspace(cargo_toml_text: str) -> bool:
    """Detect if crate has its own [workspace] block."""
    return bool(re.search(r"^\s*\[workspace\]\s*$", cargo_toml_text, re.MULTILINE))


def remove_subworkspace_block(cargo_toml_text: str) -> Tuple[str, int]:
    """Remove the [workspace] block (and following comments) from the start of file.

    Returns (new_text, removed_lines).
    """
    lines = cargo_toml_text.split("\n")
    new_lines: List[str] = []
    removed = 0
    in_workspace_block = False
    workspace_pattern = re.compile(r"^\s*\[workspace\]\s*$")

    i = 0
    while i < len(lines):
        line = lines[i]
        if workspace_pattern.match(line):
            # Start of [workspace] block
            in_workspace_block = True
            removed += 1
            i += 1
            # Skip comment lines after [workspace]
            while i < len(lines):
                if lines[i].strip().startswith("#") or lines[i].strip() == "":
                    removed += 1
                    i += 1
                    continue
                # Next non-comment, non-blank line — is it another section?
                if re.match(r"^\s*\[[\w\.\-]+\]\s*$", lines[i]):
                    in_workspace_block = False
                    break
                # Otherwise, this line might be part of workspace table
                # (e.g., members = [...]) — stop removing and emit
                in_workspace_block = False
                break
            continue
        new_lines.append(line)
        i += 1

    return "\n".join(new_lines), removed


def fix_crate_cargo_toml(crate_name: str) -> Dict[str, Any]:
    """Fix a single crate's Cargo.toml by removing [workspace] block.

    Returns dict with before/after state.
    """
    cargo_toml = WORKSPACE_ROOT / "crates" / crate_name / "Cargo.toml"
    if not cargo_toml.exists():
        return {"name": crate_name, "status": "FAIL", "error": "Cargo.toml not found"}

    before_text = cargo_toml.read_text(encoding="utf-8")
    before_has_subworkspace = detect_subworkspace(before_text)

    if not before_has_subworkspace:
        return {
            "name": crate_name,
            "status": "SKIP",
            "reason": "no [workspace] block to remove (already fixed)",
        }

    new_text, removed = remove_subworkspace_block(before_text)
    after_has_subworkspace = detect_subworkspace(new_text)

    # Write back
    cargo_toml.write_text(new_text, encoding="utf-8")

    return {
        "name": crate_name,
        "status": "FIXED" if not after_has_subworkspace else "FAIL",
        "before_has_subworkspace": before_has_subworkspace,
        "after_has_subworkspace": after_has_subworkspace,
        "removed_lines": removed,
    }


def add_to_members(crate_name: str) -> Dict[str, Any]:
    """Add a crate to Apeireth-rust/Cargo.toml members list.

    Pattern: insert before the closing `]` of members array.
    Add: `    "crates/{crate_name}",`
    Plus: comment block above explaining the fix.
    """
    cargo_toml_path = WORKSPACE_ROOT / "Cargo.toml"
    text = cargo_toml_path.read_text(encoding="utf-8")

    target = f'"crates/{crate_name}"'
    if target in text:
        return {
            "name": crate_name,
            "status": "SKIP",
            "reason": "already in members",
        }

    # Build the insertion text — comment + member line
    insertion = (
        f'    # V1305 fix (R-Cycle v2-strategy / V1303 audit medium 风险修真): '
        f'加 {crate_name} 到 workspace members.\n'
        f'    # 修真: 删 crates/{crate_name}/Cargo.toml 起始空 [workspace] 块 (sub-workspace 隔离 hack).\n'
        f'    # 修真策略 (V1303 标 medium risk): 0 改 [package], 0 改 deps, 0 改 dev-deps (version 1.0.0 / edition 2021 已 match workspace).\n'
        f'    # 风险: skeleton crate 加到 members 仅触发 workspace 总数 +1 (87 -> 88), 0 触碰 24 LOCKED crate, 0 改 workspace version (1.0.0).\n'
        f'    # cargo metadata 验证: 应能解析, members 列表应含 {crate_name}.\n'
        f'    # V1306 修真 high risk crates (sdk-lark / sdk-livekit / sdk-voice 需改 version + 删 sub-workspace 块).\n'
        f'    "crates/{crate_name}",\n'
    )

    # Find the last `"crates/..."` member line and insert after it.
    # The members list ends with `]` on its own line.
    # We want to insert before the closing `]`.
    lines = text.split("\n")
    # Find the index of the closing `]` of members list.
    # Pattern: a line that is just `]` (possibly with whitespace).
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

    # Insert the new member line right before `]`
    new_lines = lines[:close_idx] + insertion.rstrip("\n").split("\n") + lines[close_idx:]
    new_text = "\n".join(new_lines)

    cargo_toml_path.write_text(new_text, encoding="utf-8")

    return {
        "name": crate_name,
        "status": "FIXED",
        "inserted_at_line": close_idx,
    }


def verify_cargo_metadata() -> Dict[str, Any]:
    """Run cargo metadata and verify all 3 crates are in members + packages."""
    metadata = run_cargo_metadata()
    if "_error" in metadata:
        return {"status": "FAIL", "error": metadata["_error"]}

    workspace_members = metadata.get("workspace_members", [])
    workspace_packages = metadata.get("packages", [])
    package_names = [p.get("name") for p in workspace_packages]

    results = {}
    for crate in CRATES_TO_FIX:
        short_name = crate.replace("apeireth-", "apeireth_")  # crate name vs lib name
        # The actual package name in Cargo.toml is `apeireth-{name}` (kebab-case)
        pkg_name = crate
        in_members = pkg_name in workspace_members
        in_packages = pkg_name in package_names
        results[crate] = {
            "in_workspace_members": in_members,
            "in_packages": in_packages,
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


def fix_v1305() -> Dict[str, Any]:
    """Execute V1305 fix: 3 crates, remove sub-workspace + add to members + verify."""
    fix_results: List[Dict[str, Any]] = []
    for crate in CRATES_TO_FIX:
        # Step 1: Remove [workspace] block from crate's Cargo.toml
        r1 = fix_crate_cargo_toml(crate)
        # Step 2: Add to parent workspace members
        r2 = add_to_members(crate)
        fix_results.append({
            "crate": crate,
            "step1_remove_subworkspace": r1,
            "step2_add_to_members": r2,
        })

    # Step 3: Verify with cargo metadata
    verify = verify_cargo_metadata()

    return {
        "v1305_version": V1305_VERSION,
        "crates_fixed": CRATES_TO_FIX,
        "fix_results": fix_results,
        "verify": verify,
    }


def render_report(result: Dict[str, Any]) -> str:
    lines: List[str] = []
    lines.append("=" * 72)
    lines.append("V1305 — Medium Risk Orphan Crates Fix")
    lines.append(f"v1305_version: {result['v1305_version']}")
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
        s1 = fr["step1_remove_subworkspace"]
        lines.append(f"  Step 1 (remove [workspace]): {s1['status']}")
        if "removed_lines" in s1:
            lines.append(f"    removed_lines: {s1['removed_lines']}")
            lines.append(f"    before_has_subworkspace: {s1['before_has_subworkspace']}")
            lines.append(f"    after_has_subworkspace: {s1['after_has_subworkspace']}")
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
            lines.append(f"    {cname}: in_members={cr['in_workspace_members']}, in_packages={cr['in_packages']}")
    if "error" in v:
        lines.append(f"  error: {v['error']}")
    lines.append("")
    return "\n".join(lines)


def _self_test() -> None:
    # Sanity check on helper functions
    test_text = """[workspace]
# comment 1
# comment 2

[package]
name = "test"
"""
    new_text, removed = remove_subworkspace_block(test_text)
    assert "[workspace]" not in new_text, f"[workspace] still in: {new_text}"
    assert "[package]" in new_text, f"[package] removed: {new_text}"
    assert removed >= 4, f"expected >=4 removed lines, got {removed}"


if __name__ == "__main__":
    if "--self-test" in sys.argv:
        _self_test()
        print("SELF-TEST PASS")
        sys.exit(0)

    result = fix_v1305()
    print(render_report(result))
    print()
    print("=" * 72)
    print("JSON (compact)")
    print("=" * 72)
    compact = {
        "v1305_version": result["v1305_version"],
        "crates_fixed": result["crates_fixed"],
        "fix_results": result["fix_results"],
        "verify": result["verify"],
    }
    print(json.dumps(compact, indent=2, ensure_ascii=False))