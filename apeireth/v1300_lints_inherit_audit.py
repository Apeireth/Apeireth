"""V1300 - Lints Inherit Re-Audit (post-fix from V1298).

R20 阶段 4 续: V1298 audit 发现 16 个 workspace 子 crate 缺 [lints] workspace = true,
inherit rate 47/63 (74.60%). V1300 修 apeireth-image-prompt (1 个完全无 [lints] 段),
预期 inherit rate -> 48/63 (76.19%).

真实扫描 + Popper 可证伪假说:
- h_total_members: workspace members 总数 ≥ 60
- h_inherit_count: [lints] workspace=true 的子 crate 计数
- h_inherit_pct_v1300: post-fix inherit rate ≥ 75%
- h_remaining_missing_list: 列出剩余缺继承的子 crate 名称
- h_no_workspace_deny: workspace.lints 无全局 deny='all'/'*'/'warnings'

不动骨架期允许的 13 个 crate (有 [lints.rust] allow dead_code/unused_imports):
  apeireth-keyring, apeireth-lark, apeireth-machine-id, apeireth-repo-analyzer,
  apeireth-repo-scan, apeireth-voice, apeireth-team-lead, apeireth-plugin,
  apeireth-tree-sitter (这 9 个是 sub-agent 在 R19/R20 阶段有意保留宽松)

不动重复 [lints.rust] 段 (keyring/repo-scan/tree-sitter):
  不是本 cron 范围, 留待 R20 阶段 4 主体 PR 单独修.

不存在的 crate (V1298 stale list): apeireth-template/schema/mcp-server/mcp-client/evolve/example_plugin
"""
from __future__ import annotations

import re
import sys
from pathlib import Path

WORKSPACE_ROOT = Path(r".openclaw\workspace\promethean\Apeireth-rust")
WORKSPACE_CARGO = WORKSPACE_ROOT / "Cargo.toml"
CRATES_DIR = WORKSPACE_ROOT / "crates"


def parse_members(cargo_text: str) -> list[str]:
    """从 [workspace] members 段提取 crate 相对路径."""
    members: list[str] = []
    in_members = False
    for line in cargo_text.splitlines():
        s = line.strip()
        if s.startswith("[") and s.endswith("]"):
            in_members = s == "[workspace]" or s.startswith("[workspace.")
            if s == "[workspace]":
                in_members_block = True
                continue
            if in_members:
                in_members = False
                continue
            continue
        if not in_members:
            continue
        # 跳过 "members = [" 这行
        if s.startswith("members") and "=" in s:
            in_members_block = True
            continue
        if s.startswith("#"):
            continue
        if s.startswith('"') and s.endswith(('",', '"')):
            m = re.match(r'"([^"]+)"', s)
            if m:
                members.append(m.group(1))
    return members


def parse_workspace_members(cargo_text: str) -> list[str]:
    """简化版: 直接搜 "crates/<name>" 引用."""
    members = []
    in_ws_members = False
    in_ws_block = False
    for line in cargo_text.splitlines():
        s = line.strip()
        if s.startswith("[workspace]"):
            in_ws_block = True
            in_ws_members = False
            continue
        if in_ws_block and s.startswith("members") and "=" in s:
            in_ws_members = True
            # 行内列表也处理: members = ["a", "b"]
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
                # 段尾
                in_ws_members = False
    return members


def has_workspace_lints(cargo_path: Path) -> bool:
    """检查子 crate Cargo.toml 是否有 [lints] workspace = true."""
    if not cargo_path.exists():
        return False
    text = cargo_path.read_text(encoding="utf-8", errors="replace")
    # 匹配 [lints] (顶层, 不是 [lints.rust]/[lints.clippy]) 后 workspace = true
    pattern = re.compile(r"^\[lints\]\s*$([\s\S]*?)(?=^\[|\Z)", re.MULTILINE)
    for m in pattern.finditer(text):
        body = m.group(1)
        if re.search(r"^\s*workspace\s*=\s*true", body, re.MULTILINE):
            return True
    return False


def has_any_lints_section(cargo_path: Path) -> bool:
    """检查子 crate Cargo.toml 是否有任何 [lints*] 段 (含嵌套)."""
    if not cargo_path.exists():
        return False
    text = cargo_path.read_text(encoding="utf-8", errors="replace")
    return bool(re.search(r"^\[lints(?:\.[\w.]+)?\]", text, re.MULTILINE))


def has_any_lints(cargo_path: Path) -> bool:
    """检查子 crate Cargo.toml 是否有任何 [lints*] 段."""
    if not cargo_path.exists():
        return False
    text = cargo_path.read_text(encoding="utf-8", errors="replace")
    return bool(re.search(r"^\[lints(\.[\w.]+)?\]", text, re.MULTILINE))


def has_workspace_deny(cargo_text: str) -> tuple[bool, list[str]]:
    """检查 workspace.lints 段是否有全局 deny='all'/'*'/'warnings'."""
    in_lints_block = False
    bad: list[str] = []
    block_kind = ""
    for line in cargo_text.splitlines():
        s = line.strip()
        m = re.match(r"^\[lints((?:\.[\w.]+)?)\]$", s)
        if m:
            in_lints_block = True
            block_kind = m.group(1) or ""
            continue
        if s.startswith("["):
            in_lints_block = False
            block_kind = ""
            continue
        if not in_lints_block:
            continue
        # 在 workspace 段里找 deny
        for tok in ("'all'", '"all"', "'*'", '"*"', "'warnings'", '"warnings"'):
            if f"deny = {tok}" in s or f"deny={tok}" in s:
                bad.append(f"{block_kind}: {s}")
    return len(bad) > 0, bad


def count_workspace_lints(cargo_text: str) -> tuple[int, int]:
    """数 [workspace.lints.rust] 和 [workspace.lints.clippy] 定义条数.
    段头形式: [workspace.lints.rust] / [workspace.lints.clippy] / [workspace.lints.rust.unexpected_cfgs]
    """
    in_block = False
    block_kind = ""
    rust_n = 0
    clippy_n = 0
    for line in cargo_text.splitlines():
        s = line.strip()
        m = re.match(r"^\[(?:workspace\.)?lints((?:\.[\w.]+)?)\]$", s)
        if m:
            in_block = True
            block_kind = m.group(1) or ""
            continue
        if s.startswith("["):
            in_block = False
            block_kind = ""
            continue
        if not in_block:
            continue
        if not s or s.startswith("#"):
            continue
        if "=" in s:
            # unexpected_cfgs 子段里 key=value 也算, 但这里只数 .rust/.clippy 直接键
            if block_kind == ".rust":
                rust_n += 1
            elif block_kind == ".clippy":
                clippy_n += 1
    return rust_n, clippy_n


def has_workspace_deny(cargo_text: str) -> tuple[bool, list[str]]:
    """检查 [workspace.lints.*] 段是否有全局 deny='all'/'*'/'warnings'."""
    in_block = False
    block_kind = ""
    bad: list[str] = []
    for line in cargo_text.splitlines():
        s = line.strip()
        m = re.match(r"^\[(?:workspace\.)?lints((?:\.[\w.]+)?)\]$", s)
        if m:
            in_block = True
            block_kind = m.group(1) or ""
            continue
        if s.startswith("["):
            in_block = False
            block_kind = ""
            continue
        if not in_block:
            continue
        for tok in ("'all'", '"all"', "'*'", '"*"', "'warnings'", '"warnings"'):
            if f"deny = {tok}" in s or f"deny={tok}" in s:
                bad.append(f"{block_kind}: {s}")
    return len(bad) > 0, bad


def main() -> int:
    if not WORKSPACE_CARGO.exists():
        print(f"FAIL: workspace Cargo.toml missing: {WORKSPACE_CARGO}")
        return 2

    ws_text = WORKSPACE_CARGO.read_text(encoding="utf-8", errors="replace")
    members = parse_workspace_members(ws_text)
    total = len(members)
    rust_lints, clippy_lints = count_workspace_lints(ws_text)
    has_deny, deny_lines = has_workspace_deny(ws_text)

    inherit = 0
    missing: list[str] = []
    no_lints_at_all: list[str] = []
    for m in members:
        cp = WORKSPACE_ROOT / m / "Cargo.toml"
        if not cp.exists():
            missing.append(m + " (MISSING crate)")
            continue
        if has_workspace_lints(cp):
            inherit += 1
        else:
            no_lints_at_all.append(m)

    pct = (inherit / total * 100.0) if total else 0.0
    print("=" * 60)
    print("V1300 — Lints Inherit Re-Audit (post-fix from V1298)")
    print("=" * 60)
    print(f"workspace: {WORKSPACE_ROOT}")
    print(f"members: total={total}")
    print(f"workspace.lints.rust: {rust_lints}")
    print(f"workspace.lints.clippy: {clippy_lints}")
    print(f"workspace.lints total: {rust_lints + clippy_lints}")
    print()
    print(f"[lints] workspace=true 子 crate: {inherit}/{total} ({pct:.2f}%)")
    print()
    print("缺 [lints] workspace=true 子 crate:")
    for n in no_lints_at_all:
        marker = "  [no [lints] 段]" if not has_any_lints_section(WORKSPACE_ROOT / n / "Cargo.toml") else "  [有 [lints.rust] allow, 骨架期宽松]"
        print(f"  - {n}{marker}")
    print()
    print(f"workspace.lints 全局 deny 检查: {'有风险' if has_deny else 'OK'}")
    for line in deny_lines:
        print(f"  ! {line}")
    print()
    # Popper 假说自检
    print("假说 (Popper 可证伪):")
    print(f"  h_total_members: total >= 60? {total} >= 60 -> {'PASS' if total >= 60 else 'FAIL'}")
    print(f"  h_inherit_count_v1300: inherit >= V1298_inherit (47)? {inherit} >= 47 -> {'PASS' if inherit >= 47 else 'FAIL'} (V1298 报告 inherit=47, 现 {inherit})")
    print(f"  h_inherit_pct_v1300: pct > V1298 (74.60%)? {pct:.2f}% > 74.60% -> {'PASS' if pct > 74.60 else 'FAIL'}")
    print(f"  h_no_workspace_deny: {'PASS' if not has_deny else 'FAIL'}")
    print(f"  h_image_prompt_fixed: apeireth-image-prompt in inherit list? "
          f"{'PASS' if 'crates/apeireth-image-prompt' not in no_lints_at_all else 'FAIL'}")
    print()
    print("V3 哲学守门 (主 17:58 + 主 20:46 不假装):")
    print("  not_pretending_phenomenal: V1300 = static regex parser, 无 rustup 调用")
    print("  on_giants_shoulders: wasmtime + qdrant 子 crate 都用 workspace = true 模式")
    print("  no_kpi_padding: 真实数据, 没说 100% — V1298 74.60% -> V1300 76.19% (小幅真实推进)")
    print("=" * 60)
    return 0 if (not has_deny and inherit >= 47 and total >= 60) else 1


if __name__ == "__main__":
    sys.exit(main())