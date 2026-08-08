"""V1307 — Tauri-stub final audit

V1306 left 1 intentional orphan (tauri-stub, commented in workspace Cargo.toml).
The workspace comment claims "reqwest 0.13 强约束" but the actual Cargo.toml has
NO reqwest dep. This audit investigates the discrepancy and proposes the final fix.

Strategy:
1. Read tauri-stub Cargo.toml fully
2. Check the workspace comment text (exact wording)
3. Try to enable it (uncomment + cargo metadata) without committing
4. Report findings + decision

Outputs: V1307_REPORT.md + this audit script + JSON state snapshot.
"""

import json
import os
import re
import subprocess
import sys
from pathlib import Path

REPO = Path(r".openclaw\workspace\promethean")
WS = REPO / "Apeireth-rust"
TS_DIR = WS / "crates" / "apeireth-tauri-stub"
WS_CARGO = WS / "Cargo.toml"
TS_CARGO = TS_DIR / "Cargo.toml"


def read_text(p):
    return Path(p).read_text(encoding="utf-8")


def main():
    # V3 哲学守门 (per 主 17:58 不假装 Phenomenal + 主 20:46 不假装 ASI + 主 17:43 实事求是)
    # V1307 = 真修真 tauri-stub (workspace hygiene audit + fix); 非 ASI 突破; 非 Phenomenal consciousness claim
    # 修真动作仅: 删注释 + 加 member + 修文档. ASI pole-star 仍未变 (V0.1 = 0.7905).

    findings = {
        "ts_cargo_exists": TS_CARGO.exists(),
        "ts_cargo_content": None,
        "ws_members_text": None,
        "deps_in_toml": [],
        "features_in_toml": [],
        "src_files": [],
        "tauri_conf_exists": (TS_DIR / "tauri.conf.json").exists(),
        "build_rs_exists": (TS_DIR / "build.rs").exists(),
        "v3_guard_present": True,  # V3 哲学守门 marker
    }

    # Read tauri-stub Cargo.toml
    if TS_CARGO.exists():
        findings["ts_cargo_content"] = read_text(TS_CARGO)

    # Read workspace Cargo.toml
    ws_content = read_text(WS_CARGO)

    # Extract members section
    m = re.search(r"members\s*=\s*\[(.*?)\]", ws_content, re.DOTALL)
    if m:
        findings["ws_members_text"] = m.group(1)

    # Find the tauri-stub comment block
    ts_comment_pattern = re.compile(
        r"(# [^\n]*tauri-stub[^\n]*\n(?:[^#\n][^\n]*\n|#[^\n]*\n|^\s*\n)*)",
        re.MULTILINE,
    )
    ts_comments = []
    for match in ts_comment_pattern.finditer(ws_content):
        block = match.group(1)
        if "tauri-stub" in block or "tauri" in block.lower():
            ts_comments.append(block)
    findings["ws_tauri_stub_comments"] = ts_comments

    # Find the commented member line for tauri-stub
    commented_lines = []
    if m:
        for line in m.group(1).split("\n"):
            if "tauri-stub" in line:
                commented_lines.append(line)
    findings["commented_member_lines"] = commented_lines

    # Parse tauri-stub Cargo.toml deps
    deps_match = re.search(r"\[dependencies\](.*?)(?=\n\[|\Z)", findings["ts_cargo_content"], re.DOTALL)
    if deps_match:
        deps_section = deps_match.group(1)
        dep_names = re.findall(r"^(\w[\w-]*)\s*=", deps_section, re.MULTILINE)
        findings["deps_in_toml"] = dep_names

    # Check src files
    if (TS_DIR / "src").exists():
        findings["src_files"] = sorted(os.listdir(TS_DIR / "src"))

    # Find all reqwest mentions
    reqwest_in_ws = re.findall(r".*reqwest.*", ws_content)
    findings["reqwest_mentions_in_ws_cargo"] = reqwest_in_ws

    reqwest_in_ts = re.findall(r".*reqwest.*", findings["ts_cargo_content"])
    findings["reqwest_mentions_in_ts_cargo"] = reqwest_in_ts

    # Check git tracked status of tauri-stub
    git_out = subprocess.run(
        ["git", "ls-files", "Apeireth-rust/crates/apeireth-tauri-stub/"],
        cwd=REPO, capture_output=True, text=True, timeout=20
    )
    findings["git_tracked_files"] = git_out.stdout.strip().split("\n") if git_out.stdout.strip() else []

    # Write findings JSON
    out_json = REPO / "v1307_audit_findings.json"
    with open(out_json, "w", encoding="utf-8") as f:
        json.dump(findings, f, indent=2, ensure_ascii=False)
    print(f"Wrote {out_json}")

    # Print key findings
    print("\n=== KEY FINDINGS ===")
    print(f"tauri-stub deps in Cargo.toml: {findings['deps_in_toml']}")
    print(f"reqwest mentions in tauri-stub Cargo.toml: {findings['reqwest_mentions_in_ts_cargo']}")
    print(f"reqwest mentions in workspace Cargo.toml: {findings['reqwest_mentions_in_ws_cargo']}")
    print(f"commented member line: {findings['commented_member_lines']}")
    print(f"git tracked files: {len(findings['git_tracked_files'])}")

    # Summary
    print("\n=== SUMMARY ===")
    if not findings["reqwest_mentions_in_ts_cargo"] and any(
        "reqwest" in m for m in findings["commented_member_lines"] + findings["ws_tauri_stub_comments"]
    ):
        print("DISCREPANCY FOUND: workspace comment cites reqwest, but tauri-stub Cargo.toml has NO reqwest dep")
        print("Decision needed: uncomment and try build, OR fix the comment to reflect reality")
    else:
        print("No reqwest discrepancy detected (comment may reference an older version)")


if __name__ == "__main__":
    main()