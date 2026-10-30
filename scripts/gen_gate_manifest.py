#!/usr/bin/env python3
"""**门禁清单生成器 / 漂移守卫**（P0 · 2026-10-30）。

# 要解决什么

仓库的**规则**在 `docs/architecture/contract.toml`（唯一真源），
但**人读的副本**（PR 模板、AGENTS.md）里还留着**手写的旧数字**
⇒ 手写副本会**默默过期**（实测：PR 模板写「24 LOCKED crate」「1.2.0」，
而契约早已是 `2.0.0-rc.4` / 30 members ⇒ **CI 守的与模板写的不一致**）。

⇒ ⇒ 本脚本做两件事：

1. `--gen`：**从契约生成**人读副本里的自动事实块（`<!-- BEGIN:auto-facts -->` 区间）
   + 机器可读的 `scripts/gate-manifest.json`；
2. `--check`：**校验**人读副本里的块与契约一致 ⇒ 不一致 ⇒ **红**（退出 1）。

# 口径

* **只读契约**（`docs/architecture/contract.toml`）⇒ 不引入第二真源；
* 生成块用 `<!-- BEGIN:auto-facts -->` / `<!-- END:auto-facts -->` 包住
  ⇒ 与 `scripts/check_batch_status.py` 的既有约定一致；
* **幂等**：连跑两次产物逐字节相同。

# 用法

    python3 scripts/gen_gate_manifest.py --gen     # 写副本 + manifest
    python3 scripts/gen_gate_manifest.py --check   # 只校验（CI 用）
"""

from __future__ import annotations

import argparse
import json
import re
import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
CONTRACT = ROOT / "docs" / "architecture" / "contract.toml"
MANIFEST = ROOT / "scripts" / "gate-manifest.json"
PR_TEMPLATE = ROOT / ".github" / "PULL_REQUEST_TEMPLATE.md"

BEGIN = "<!-- BEGIN:auto-facts -->"
END = "<!-- END:auto-facts -->"


def load_contract() -> dict:
    try:
        with CONTRACT.open("rb") as handle:
            return tomllib.load(handle)
    except FileNotFoundError:
        print(f"FAIL: 找不到契约 {CONTRACT}", file=sys.stderr)
        raise SystemExit(2)
    except tomllib.TOMLDecodeError as error:
        print(f"FAIL: 契约 TOML 解析失败：{error}", file=sys.stderr)
        raise SystemExit(2)


def gates(contract: dict) -> list[dict]:
    return list(contract.get("ci_gates", {}).get("gate", []))


def locked_sources(contract: dict) -> list[str]:
    """`[locked]` 里登记的 **.rs 源码**（CI hard-walls 实际守的就是这些）。"""
    locked = contract.get("locked", {})
    return sorted(
        value
        for value in locked.values()
        if isinstance(value, str) and value.endswith(".rs")
    )


def render_block(contract: dict) -> str:
    """生成人读副本里的自动事实块（**不含** BEGIN/END 标记本身）。"""
    meta = contract.get("meta", {})
    workspace = contract.get("workspace", {})
    registry = gates(contract)
    required = [g for g in registry if g.get("required")]
    local = [g for g in registry if g.get("env") != "cargo" and g.get("env") != "ci"]

    lines: list[str] = [
        BEGIN,
        "<!-- 本块由 `scripts/gen_gate_manifest.py` 从 "
        "`docs/architecture/contract.toml` **生成** ⇒ **请勿手改** -->",
        "<!-- 手改会被 `scripts/gen_gate_manifest.py --check` 拦下（守卫会红） -->",
        "",
        "### 仓库事实（**自动生成** · 真源 = `docs/architecture/contract.toml`）",
        "",
        f"- **workspace.version**：`{workspace.get('version', '?')}`"
        f"（契约 `{meta.get('version', '?')}` · 状态 `{meta.get('status', '?')}`）",
        f"- **workspace 成员**：`{workspace.get('members', '?')}`"
        f"（产品 crate `{workspace.get('product_crates', '?')}`）",
        "- **受锁源码（0 触碰 · CI hard-walls 实际清单）**：",
    ]
    for path in locked_sources(contract):
        lines.append(f"  - `{path}`")
    lines += [
        f"- **门禁登记表**：`docs/architecture/contract.toml` 的 `[ci_gates]`"
        f"（共 **{len(registry)}** 条 · required **{len(required)}** 条）",
        f"  - 本地可跑（`{len(local)}` 条）："
        + " · ".join(f"`{g['id']}`" for g in local),
        "  - 完整清单与命令：**不要在这里复制**，改看契约 `[ci_gates]`"
        "（`scripts/harness.py plan` 会按变更范围列出该跑哪几条）",
        "",
        "### 硬约束（**数字全部来自上面那块，不要在此手写**）",
        "",
        "- [ ] **0 触碰受锁源码**（清单见上，CI `hard-walls` 逐文件 word-diff 守）",
        "- [ ] **0 改 `workspace.version`**（除非带 release 拍板证据，见 `docs/versioning/`）",
        "- [ ] `cargo test --workspace` 全绿 + `cargo test --doc` 全绿",
        "- [ ] **0 假装**（skeleton 标 ⏳，不假装已实现，编译期 hardcode 守门）",
        "",
    ]
    return "\n".join(lines) + END


def read_block(text: str) -> str | None:
    if BEGIN not in text or END not in text:
        return None
    start = text.index(BEGIN)
    stop = text.index(END) + len(END)
    return text[start:stop]


def write_block(text: str, block: str) -> str:
    """把生成块写回原文（替换旧块；没有就插到 `## Description` 之前）。"""
    existing = read_block(text)
    if existing is not None:
        return text.replace(existing, block, 1)
    marker = "## Description"
    if marker in text:
        return text.replace(marker, block + "\n\n" + marker, 1)
    return block + "\n\n" + text


def build_manifest(contract: dict) -> dict:
    registry = gates(contract)
    return {
        "schema": "apeireth.gate-manifest.v1",
        "generated_from": "docs/architecture/contract.toml [ci_gates]",
        "workspace_version": contract.get("workspace", {}).get("version"),
        "workspace_members": contract.get("workspace", {}).get("members"),
        "locked_sources": locked_sources(contract),
        "gates": [
            {
                "id": g.get("id"),
                "cmd": list(g.get("cmd", [])),
                "scope": list(g.get("scope", [])),
                "blocking": bool(g.get("blocking", False)),
                "required": bool(g.get("required", False)),
                "env": g.get("env", "python"),
                "timeout": int(g.get("timeout", 900)),
                "note": g.get("note", ""),
            }
            for g in registry
        ],
    }


def cmd_gen(contract: dict) -> int:
    block = render_block(contract)
    manifest = build_manifest(contract)

    text = PR_TEMPLATE.read_text(encoding="utf-8")
    updated = write_block(text, block)
    if updated != text:
        PR_TEMPLATE.write_text(updated, encoding="utf-8")
        print(f"✓ 已更新 {PR_TEMPLATE.relative_to(ROOT)} 的自动事实块")
    else:
        print(f"✓ {PR_TEMPLATE.relative_to(ROOT)} 自动事实块已是最新")

    MANIFEST.write_text(
        json.dumps(manifest, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
    )
    print(
        f"✓ 已写 {MANIFEST.relative_to(ROOT)}："
        f"{len(manifest['gates'])} 条门禁 · "
        f"{sum(1 for g in manifest['gates'] if g['required'])} 条 required"
    )
    return 0


def cmd_check(contract: dict) -> int:
    problems: list[str] = []

    # ① PR 模板的自动事实块是否与契约一致
    if not PR_TEMPLATE.exists():
        problems.append(f"缺少 {PR_TEMPLATE.relative_to(ROOT)}")
    else:
        text = PR_TEMPLATE.read_text(encoding="utf-8")
        existing = read_block(text)
        if existing is None:
            problems.append(
                f"{PR_TEMPLATE.relative_to(ROOT)} 里**没有**自动事实块"
                f"（缺 `{BEGIN}` … `{END}`）⇒ 跑 `--gen` 生成"
            )
        else:
            expected = render_block(contract)
            if existing != expected:
                problems.append(
                    f"{PR_TEMPLATE.relative_to(ROOT)} 的自动事实块**与契约不一致**"
                    f" ⇒ 有人手改了副本（或契约改了没重新生成）⇒ 跑 `--gen`"
                )

    # ② manifest 是否与契约一致
    if not MANIFEST.exists():
        problems.append(f"缺少 {MANIFEST.relative_to(ROOT)} ⇒ 跑 `--gen`")
    else:
        try:
            current = json.loads(MANIFEST.read_text(encoding="utf-8"))
        except json.JSONDecodeError as error:
            current = None
            problems.append(f"{MANIFEST.relative_to(ROOT)} 不是合法 JSON：{error}")
        if current is not None and current != build_manifest(contract):
            problems.append(
                f"{MANIFEST.relative_to(ROOT)} **与契约不一致** ⇒ 跑 `--gen`"
            )

    # ③ 每条门禁引用的脚本必须存在（登记了不存在的门禁 = 假门禁）
    registry = gates(contract)
    ids = [g.get("id") for g in registry]
    if len(ids) != len(set(ids)):
        problems.append("门禁 id 有重复")
    for gate in registry:
        for arg in gate.get("cmd", []):
            if not isinstance(arg, str) or not arg.startswith("scripts/"):
                continue
            if not (ROOT / arg).exists():
                problems.append(
                    f"门禁 `{gate.get('id')}` 引用的脚本**不存在**：`{arg}`"
                )
    for path in locked_sources(contract):
        if not (ROOT / path).exists():
            problems.append(f"契约 `[locked]` 登记的受锁源码**不存在**：`{path}`")

    if problems:
        print(f"FAIL: 门禁清单守卫（{len(problems)} 处）", file=sys.stderr)
        for line in problems:
            print(f"  · {line}", file=sys.stderr)
        return 1
    print(
        f"OK: 门禁清单一致（{len(registry)} 条门禁 · PR 模板自动事实块与契约一致 · "
        f"受锁源码 {len(locked_sources(contract))} 个存在）"
    )
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    group = parser.add_mutually_exclusive_group(required=True)
    group.add_argument("--gen", action="store_true", help="生成副本 + manifest")
    group.add_argument("--check", action="store_true", help="校验（CI 用）")
    args = parser.parse_args()

    contract = load_contract()
    if not gates(contract):
        print(
            "FAIL: 契约里没有 `[ci_gates]` 门禁登记表 ⇒ 无从校验",
            file=sys.stderr,
        )
        return 1
    if args.gen:
        return cmd_gen(contract)
    return cmd_check(contract)


if __name__ == "__main__":
    raise SystemExit(main())
