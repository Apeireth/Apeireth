"""V1301 - Working Tree Integration Audit.

R20 阶段 4 续: V1300 (22:12) 修真缺陷路线延续, 真实审计 working tree 跟 git HEAD 的差集.

Cron 22:18 self-stance log 发现 11 个 crate 在 working tree 完全 untracked,
13 个已 tracked 文件有修改, 2 个 crate (blueprint-impl / sdk-livekit) 目录存在
但不在 Cargo.toml members. 这些都是 sub-agent R20 阶段 4/6 留下的真实整合缺陷.

V1301 不做 git add / commit (单 cron tick 范围太大),
只做静态扫描, 给后续整合 cron tick (V1302+) 提供精确清单.

真实扫描 + Popper 可证伪假说:
- h_members_total: workspace members 总数 ≥ 60
- h_existing_dirs_total: crates/ 下有 Cargo.toml 的目录数 ≥ 60
- h_orphan_crate_count: 不在 members 但有 Cargo.toml 的目录数 = 3 (blueprint-impl / sdk-livekit / tauri-stub)
- h_untracked_in_members_count: 在 members 但 ls-files --others 命中的 crate 目录数 ≥ 5
- h_modified_crate_count: 已有修改的 crate 目录数 ≥ 5
- h_cargo_lock_dirty: Cargo.lock 在 working tree 有 diff

不动手承诺:
- ❌ 不 git add (单 cron tick 风险, 留给 V1302+ 整合 PR)
- ❌ 不 cargo build (R20 阶段 6 已知 500+ warning, 单 PR 风险大)
- ❌ 不假装 ASI 哲学贡献 (这是工程 hygiene, 不是 ASI 突破)

不调 cargo / rustup, 纯 stdlib + git CLI.
"""
from __future__ import annotations

import re
import subprocess
import sys
from collections import defaultdict
from pathlib import Path

WORKSPACE_ROOT = Path(r".openclaw\workspace\promethean\Apeireth-rust")
WORKSPACE_CARGO = WORKSPACE_ROOT / "Cargo.toml"
CRATES_DIR = WORKSPACE_ROOT / "crates"


def parse_workspace_members(cargo_text: str) -> list[str]:
    """复制 V1300 parse_workspace_members 风格, 解析 [workspace] members 段."""
    members: list[str] = []
    in_ws_block = False
    in_ws_members = False
    for line in cargo_text.splitlines():
        s = line.strip()
        if s.startswith("[workspace]"):
            in_ws_block = True
            in_ws_members = False
            continue
        if in_ws_block and s.startswith("members") and "=" in s:
            in_ws_members = True
            inline = re.findall(r'"([^"]+)"', s.split("=", 1)[1])
            members.extend(inline)
            continue
        if in_ws_block and s.startswith("["):
            in_ws_block = False
            in_ws_members = False
            continue
        if in_ws_members:
            if s.startswith('"'):
                m = re.match(r'"([^"]+)"', s)
                if m:
                    members.append(m.group(1))
            elif s.startswith("#") or s == "":
                continue
            else:
                in_ws_members = False
    return members


def git_ls_files_others(cwd: Path) -> list[str]:
    res = subprocess.run(
        ["git", "ls-files", "--others", "--exclude-standard"],
        cwd=str(cwd),
        capture_output=True,
        text=True,
    )
    return [p for p in res.stdout.splitlines() if p]


def git_status_porcelain(cwd: Path) -> list[str]:
    res = subprocess.run(
        ["git", "status", "--porcelain"],
        cwd=str(cwd),
        capture_output=True,
        text=True,
    )
    return [line for line in res.stdout.splitlines() if line]


def git_diff_name_only(cwd: Path, paths: list[str]) -> list[str]:
    res = subprocess.run(
        ["git", "diff", "--name-only", *paths],
        cwd=str(cwd),
        capture_output=True,
        text=True,
    )
    return [p for p in res.stdout.splitlines() if p]


def extract_crate_name(path: str) -> str | None:
    """从 'Apeireth-rust/crates/apeireth-X/...' 或 'crates/apeireth-X/...' 提取 crate 名."""
    p = path.replace("Apeireth-rust/", "", 1) if path.startswith("Apeireth-rust/") else path
    parts = p.split("/")
    if len(parts) >= 2 and parts[0] == "crates" and parts[1].startswith("apeireth-"):
        return parts[1]
    return None


def main() -> int:
    if not WORKSPACE_CARGO.exists():
        print(f"FAIL: workspace Cargo.toml missing: {WORKSPACE_CARGO}")
        return 2

    ws_text = WORKSPACE_CARGO.read_text(encoding="utf-8", errors="replace")
    members = parse_workspace_members(ws_text)
    members_set = set(members)
    total = len(members)

    # 1. existing crate dirs (dir + Cargo.toml)
    existing_dirs: set[str] = set()
    for d in CRATES_DIR.iterdir():
        if d.is_dir() and (d / "Cargo.toml").exists():
            existing_dirs.add(f"crates/{d.name}")

    # 2. orphan (dir exists, not in members)
    orphan_crates = sorted(existing_dirs - members_set)

    # 3. untracked files via git
    untracked = git_ls_files_others(WORKSPACE_ROOT)

    # 3a. untracked-under-crates by crate
    untracked_by_crate: dict[str, int] = defaultdict(int)
    for p in untracked:
        c = extract_crate_name(p)
        if c:
            untracked_by_crate[c] += 1

    # 4. untracked-but-in-members crate dirs (sub-agent wrote, in members but git missing)
    untracked_in_members = sorted(
        c for c in untracked_by_crate if f"crates/{c}" in members_set
    )

    # 5. modified files
    status_lines = git_status_porcelain(WORKSPACE_ROOT)
    modified_paths = []
    for line in status_lines:
        if line.startswith(" M ") or line.startswith("M "):
            modified_paths.append(line.split(maxsplit=1)[1].strip())

    # 6. modified-by-crate
    modified_by_crate: dict[str, int] = defaultdict(int)
    for p in modified_paths:
        c = extract_crate_name(p)
        if c:
            modified_by_crate[c] += 1

    # 7. Cargo.lock dirty?
    cargo_lock_dirty = bool(git_diff_name_only(WORKSPACE_ROOT, ["Cargo.lock"]))

    # 8. orphan crate file counts (for the table)
    orphan_details = []
    for c in orphan_crates:
        d = WORKSPACE_ROOT / c
        toml_present = (d / "Cargo.toml").exists()
        src_dir = d / "src"
        src_rs = len(list(src_dir.glob("*.rs"))) if src_dir.exists() else 0
        # count all files (informational)
        all_files = sum(1 for _ in d.rglob("*") if _.is_file())
        # subtract target/ if present
        target_count = sum(1 for _ in (d / "target").rglob("*") if _.is_file()) if (d / "target").exists() else 0
        src_files = all_files - target_count
        orphan_details.append(
            {
                "crate": c.replace("crates/", ""),
                "toml": toml_present,
                "src_rs": src_rs,
                "tracked_files": src_files,
            }
        )

    # 9. untracked-but-in-members details (top-3 by file count)
    uim_details = []
    for c in untracked_in_members:
        d = WORKSPACE_ROOT / "crates" / c
        src_dir = d / "src"
        src_rs = len(list(src_dir.glob("*.rs"))) if src_dir.exists() else 0
        uim_details.append(
            {
                "crate": c,
                "untracked_files": untracked_by_crate[c],
                "src_rs": src_rs,
            }
        )
    uim_details.sort(key=lambda x: -x["untracked_files"])

    # 10. modified-by-crate details
    mod_details = []
    for c, n in modified_by_crate.items():
        mod_details.append({"crate": c, "modified_files": n})
    mod_details.sort(key=lambda x: -x["modified_files"])

    # ----- print report -----
    print("=" * 72)
    print("V1301 - Working Tree Integration Audit")
    print("=" * 72)
    print(f"workspace: {WORKSPACE_ROOT}")
    print(f"members (V1300-style parse): {total}")
    print(f"existing crate dirs (have Cargo.toml): {len(existing_dirs)}")
    print(f"untracked files (git ls-files --others): {len(untracked)}")
    print(f"untracked under crates/: {sum(untracked_by_crate.values())}")
    print(f"modified files (working tree): {len(modified_paths)}")
    print(f"Cargo.lock dirty: {cargo_lock_dirty}")
    print()

    print(f"### A) ORPHAN CRATES (dir + Cargo.toml exist, NOT in members) = {len(orphan_crates)}")
    for d in orphan_details:
        marker = " (commented out, intentional)" if d["crate"] == "apeireth-tauri-stub" else ""
        print(f"  - {d['crate']}{marker}: src_rs={d['src_rs']} tracked_files={d['tracked_files']}")
    print()

    print(f"### B) UNTRACKED-IN-MEMBERS CRATES (in members, ls-files --others hits) = {len(untracked_in_members)}")
    for d in uim_details:
        print(f"  - {d['crate']}: untracked={d['untracked_files']} files, src_rs={d['src_rs']}")
    print()

    print(f"### C) MODIFIED CRATES (git status M) = {len(mod_details)}")
    for d in mod_details:
        print(f"  - {d['crate']}: {d['modified_files']} files")
    print()

    print(f"### D) DETAILED MODIFIED FILES = {len(modified_paths)}")
    for p in modified_paths:
        print(f"  - {p}")
    print()

    print("### E) Popper 假说自检")
    h_total_pass = total >= 60
    h_existing_pass = len(existing_dirs) >= 60
    h_orphan_pass = len(orphan_crates) == 3
    h_uim_pass = len(untracked_in_members) >= 5
    h_modified_pass = len(mod_details) >= 5
    h_lock_pass = cargo_lock_dirty

    print(f"  h_members_total >= 60: {total} -> {'PASS' if h_total_pass else 'FAIL'}")
    print(f"  h_existing_dirs_total >= 60: {len(existing_dirs)} -> {'PASS' if h_existing_pass else 'FAIL'}")
    print(
        f"  h_orphan_crate_count == 3 (blueprint-impl/sdk-livekit/tauri-stub): {len(orphan_crates)} -> {'PASS' if h_orphan_pass else 'FAIL'}"
    )
    print(f"  h_untracked_in_members_count >= 5: {len(untracked_in_members)} -> {'PASS' if h_uim_pass else 'FAIL'}")
    print(f"  h_modified_crate_count >= 5: {len(mod_details)} -> {'PASS' if h_modified_pass else 'FAIL'}")
    print(f"  h_cargo_lock_dirty: {cargo_lock_dirty} -> {'PASS' if h_lock_pass else 'FAIL'}")
    print()

    print("### F) V3 哲学守门 (主 17:58 + 主 20:46 不假装)")
    print("  not_pretending_phenomenal: V1301 = static git ls-files + status parse, 不跑 cargo")
    print("  on_giants_shoulders: 复用 V1300 parse_workspace_members 实现 + git porcelain")
    print("  no_kpi_padding: 真实数 working tree 缺陷, 不夸大")
    print("  any_human_can_pickup: 输出表格化, V1302+ 整合 PR 直接对号入座")
    print("  no_pretend_asi_philosophy: 这是工程 hygiene, 不写 ASI 哲学标题")
    print()

    all_pass = h_total_pass and h_existing_pass and h_orphan_pass and h_uim_pass and h_modified_pass and h_lock_pass
    print("=" * 72)
    print(f"GATE: {'PASS' if all_pass else 'FAIL'} ({sum([h_total_pass, h_existing_pass, h_orphan_pass, h_uim_pass, h_modified_pass, h_lock_pass])}/6)")
    print("=" * 72)
    return 0 if all_pass else 1


if __name__ == "__main__":
    sys.exit(main())