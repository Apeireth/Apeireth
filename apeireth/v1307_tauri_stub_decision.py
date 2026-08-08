"""V1307 — Tauri-stub decision: try enable, measure, decide

Strategy:
1. Save current workspace Cargo.toml as backup
2. Uncomment "crates/apeireth-tauri-stub" line
3. Run cargo metadata to see if it parses cleanly
4. Run cargo check -p apeireth-tauri-stub to see if it actually builds
5. Based on result, recommend final fix:
   - PASS: uncomment permanently (proper enable)
   - FAIL: keep commented, fix comment text (audit-only)
6. Restore backup, write decision report

Outputs: V1307_REPORT.md + v1307_audit_findings.json + audit script + tests.
"""

import json
import os
import re
import shutil
import subprocess
import sys
import time
from pathlib import Path

REPO = Path(r".openclaw\workspace\promethean")
WS = REPO / "Apeireth-rust"
WS_CARGO = WS / "Cargo.toml"


def read_text(p):
    return Path(p).read_text(encoding="utf-8")


def write_text(p, content):
    Path(p).write_text(content, encoding="utf-8")


def cargo_metadata():
    out = subprocess.run(
        ["cargo", "metadata", "--format-version=1", "--no-deps"],
        cwd=WS, capture_output=True, timeout=60
    )
    if out.returncode != 0:
        return {"error": out.stderr.decode("utf-8", "replace")[:500]}
    return json.loads(out.stdout)


def cargo_check_member(pkg):
    out = subprocess.run(
        ["cargo", "check", "-p", pkg],
        cwd=WS, capture_output=True, timeout=300
    )
    return {
        "returncode": out.returncode,
        "stdout_tail": out.stdout.decode("utf-8", "replace")[-2000:],
        "stderr_tail": out.stderr.decode("utf-8", "replace")[-2000:],
    }


def main():
    # Step 1: Backup
    backup_path = WS_CARGO.with_suffix(".toml.v1307bak")
    shutil.copy2(WS_CARGO, backup_path)
    print(f"Backup: {backup_path}")

    original_content = read_text(WS_CARGO)

    # Step 2: Find the commented line
    m = re.search(r"(    # \"crates/apeireth-tauri-stub\",)", original_content)
    if not m:
        print("ERROR: commented line not found")
        sys.exit(1)

    # Uncomment
    modified_content = original_content.replace(
        '    # "crates/apeireth-tauri-stub",',
        '    "crates/apeireth-tauri-stub",',
    )
    write_text(WS_CARGO, modified_content)
    print("Uncommented: 'crates/apeireth-tauri-stub'")

    # Step 3: cargo metadata (must succeed first)
    print("\n=== cargo metadata ===")
    md = cargo_metadata()
    if "error" in md:
        print(f"FAILED: {md['error']}")
        write_text(WS_CARGO, original_content)
        print("Restored backup")
        sys.exit(1)
    members = md.get("workspace_members", [])
    in_members = "apeireth-tauri-stub" in members
    print(f"workspace_members count: {len(members)}")
    print(f"tauri-stub in members: {in_members}")

    # Step 4: cargo check (this is the real test)
    print("\n=== cargo check -p apeireth-tauri-stub ===")
    check_result = cargo_check_member("apeireth-tauri-stub")
    print(f"returncode: {check_result['returncode']}")
    if check_result['returncode'] != 0:
        print(f"STDERR tail:\n{check_result['stderr_tail']}")
    else:
        print(f"STDOUT tail:\n{check_result['stdout_tail']}")

    # Step 5: restore backup, write decision
    write_text(WS_CARGO, original_content)
    print("\nRestored backup (no permanent change yet)")

    decision = {
        "metadata_passed": in_members,
        "check_passed": check_result["returncode"] == 0,
        "check_returncode": check_result["returncode"],
        "check_stderr_tail": check_result["stderr_tail"],
        "check_stdout_tail": check_result["stdout_tail"],
    }

    out_json = REPO / "v1307_decision.json"
    with open(out_json, "w", encoding="utf-8") as f:
        json.dump(decision, f, indent=2, ensure_ascii=False)
    print(f"\nWrote {out_json}")

    if decision["check_passed"]:
        print("\nDECISION: tauri-stub BUILDS CLEAN — recommend uncomment permanently")
    else:
        print("\nDECISION: tauri-stub BUILD FAILED — keep commented, fix workspace comment to be truthful")

    # Cleanup backup (we have it in git anyway via commit process)
    backup_path.unlink()
    print("Backup removed (revert clean)")


if __name__ == "__main__":
    main()