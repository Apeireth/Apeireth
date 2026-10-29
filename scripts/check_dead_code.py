#!/usr/bin/env python3
"""D-1 死代码棘轮（只减不增）：`dead_code = 'warn'` 的豁免清单不得增长。

背景
----
`Cargo.toml` 的 `[workspace.lints.rust] dead_code` 原为 `'allow'`（2026-08 为绕开
"别的协作者正在改的 src 文件"而整体压掉）。owner 2026-10-25 裁定 D-1 = A：
**改成 `warn` + 基线棘轮**。

三道闸
------
1. **真守卫（编译期）**：`dead_code = 'warn'` ⇒ 任何**没有局部豁免**的死代码都会被
   clippy 报出来；CI 的 `cargo clippy --workspace --all-targets -- -D warnings`
   把它变成**失败**。这才是主要防线。
2. **豁免棘轮（本脚本）**：既有豁免（`#[allow(dead_code)]` / `#![allow(dead_code)]`）
   **只减不增** —— 没有这条，任何人加一个 `#[allow(dead_code)]` 就能把新债藏起来。
3. **理由登记**：每处豁免都要在 `scripts/dead-code-reasons.txt` 里有**理由**
   （按文件登记）—— 没有理由的豁免同样算失败。

用法
----
    python3 scripts/check_dead_code.py            # 比对基线（退出码 1 = 超标/缺理由）
    python3 scripts/check_dead_code.py --update   # 重新生成基线（须人工复核）

口径
----
* 站点 = 一处 `#[allow(dead_code)]`（项级）或 `#![allow(dead_code)]`（文件级）。
* 标识 = `路径` + `被豁免项的锚点`（项前 48 字符，**不含行号** —— 行号会漂移）。
* 理由按**文件**登记（同一文件的同类豁免共用一条理由）。

为什么不把理由写在 .rs 里
--------------------------
rustfmt 会把「属性 + 行尾注释」挪成「属性独占行 + 注释进函数体」（位置不对），
而 doc 注释方案会多占一行文件体积（尺寸棘轮零余量）。本仓既有惯例
（孤儿棘轮：`unwired-baseline.txt` + 分组标记）也是**理由进登记文件**。
"""

from __future__ import annotations

import argparse
import hashlib
import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
BASELINE = ROOT / "scripts" / "dead-code-baseline.txt"
REASONS = ROOT / "scripts" / "dead-code-reasons.txt"
# 扫描范围 = **workspace 真正编译的** Rust 源码（与 CI 的
# `cargo clippy --workspace --all-targets` 覆盖面一致）：
#   crates/  —— 全部产品 crate；
#   benches/ —— workspace 成员（基准复现 harness，publish = false）。
#
# 刻意**不含**：
#   legacy/ research/ —— Cargo.toml 的 `exclude` 项（归档冻结，不编译 ⇒
#                         编译器根本不会在那里报 dead_code ⇒ 无需登记）；
#   fuzz/ examples/    —— 非 workspace 成员（cargo-fuzz 单独构建），
#                         同样不进 `--workspace --all-targets` 的编译面。
# 范围与编译面不一致 = 守卫有洞（多登记）或误报（少适用）。
SOURCE_DIRS = ["crates", "benches"]

ITEM_ALLOW = re.compile(r"^\s*#\[allow\(dead_code\)\]\s*$")
FILE_ALLOW = re.compile(r"^\s*#!\[allow\(dead_code\)\]\s*$")


def collect() -> list[tuple[str, str]]:
    """返回 [(站点标识, 文件路径)]。"""
    sites: list[tuple[str, str]] = []
    paths: list[pathlib.Path] = []
    for name in SOURCE_DIRS:
        base = ROOT / name
        if base.is_dir():
            paths.extend(base.rglob("*.rs"))
    for path in sorted(paths):
        rel = path.relative_to(ROOT).as_posix()
        lines = path.read_text(encoding="utf-8", errors="ignore").splitlines()
        for index, line in enumerate(lines):
            if FILE_ALLOW.match(line):
                sites.append((f"{rel}#file", rel))
            elif ITEM_ALLOW.match(line):
                sites.append((f"{rel}:{_item_anchor(lines, index)}", rel))
    return sites


def _item_anchor(lines: list[str], index: int) -> str:
    """取被豁免项的标识（下一非空非注释行的前 48 字符），避开行号漂移。"""
    for pos in range(index + 1, min(index + 8, len(lines))):
        text = lines[pos].strip()
        if not text or text.startswith("//"):
            continue
        return re.sub(r"\s+", " ", text)[:48].strip()
    return f"line{index + 1}"


def load_reasons() -> dict[str, str]:
    """文件路径 → 理由。"""
    reasons: dict[str, str] = {}
    if not REASONS.exists():
        return reasons
    for line in REASONS.read_text(encoding="utf-8").splitlines():
        stripped = line.strip()
        if not stripped or stripped.startswith("#"):
            continue
        path, _, reason = stripped.partition("  ")
        reasons[path.strip()] = reason.strip()
    return reasons


def fingerprint(sites: list[tuple[str, str]]) -> str:
    payload = "\n".join(sorted(ident for ident, _ in sites))
    return hashlib.sha256(payload.encode("utf-8")).hexdigest()[:16]


def main() -> int:
    parser = argparse.ArgumentParser(description="D-1 死代码豁免棘轮（只减不增）")
    parser.add_argument("--update", action="store_true", help="重新生成基线")
    args = parser.parse_args()

    sites = collect()
    reasons = load_reasons()
    digest = fingerprint(sites)
    no_reason = sorted({rel for _, rel in sites if not reasons.get(rel)})

    if args.update:
        lines = [
            "# D-1 死代码豁免基线（只减不增）",
            "#",
            "# 由 `dead_code = 'warn'`（workspace lint）+ 本清单共同构成机检守卫：",
            "# ① 任何**新增**的死代码（没有局部 allow）⇒ clippy 报 warning ⇒ CI 的",
            "#    `-D warnings` 直接失败（这是真正的守卫）；",
            "# ② 本清单里的既有豁免**只减不增** —— 新增豁免必须写清理由并登记；",
            "# ③ 理由在 `scripts/dead-code-reasons.txt`（按文件登记）。",
            "#",
            f"# 合计: {len(sites)} 处 · 指纹: {digest}",
            "#",
            "# 格式: <站点标识>  <理由>",
            "",
        ]
        for ident, rel in sorted(sites):
            lines.append(f"{ident}  {reasons.get(rel, '⚠ 缺理由')}")
        BASELINE.write_text("\n".join(lines) + "\n", encoding="utf-8")
        print(f"基线已更新: {len(sites)} 处 · 指纹 {digest}")
        if no_reason:
            print(f"FAIL: {len(no_reason)} 个文件缺理由登记：")
            for rel in no_reason:
                print(f"  - {rel}")
            return 1
        return 0

    if not BASELINE.exists():
        print(f"FAIL: 基线缺失 {BASELINE}（先跑 --update）")
        return 1

    baseline_ids = {
        line.rstrip().split("  ", 1)[0].strip()
        for line in BASELINE.read_text(encoding="utf-8").splitlines()
        if line.strip() and not line.startswith("#")
    }
    current_ids = {ident for ident, _ in sites}
    added = sorted(current_ids - baseline_ids)
    removed = sorted(baseline_ids - current_ids)

    print(f"豁免站点: 当前 {len(sites)} 处 / 基线 {len(baseline_ids)} 处 · 指纹 {digest}")
    failed = False
    if added:
        print(f"FAIL: 新增 {len(added)} 处豁免（棘轮只减不增）：")
        for ident in added[:20]:
            print(f"  + {ident}")
        print("  处理: ① 能删就删；② 确需保留则写明理由并登记进 scripts/dead-code-reasons.txt")
        failed = True
    if no_reason:
        print(f"FAIL: {len(no_reason)} 个文件的豁免缺理由登记：")
        for rel in no_reason:
            print(f"  - {rel}")
        print("  处理: 在 scripts/dead-code-reasons.txt 补一行「<文件路径>  <理由>」")
        failed = True
    if failed:
        return 1
    if removed:
        print(f"OK: 豁免减少 {len(removed)} 处（棘轮方向正确）")
    else:
        print("OK: 与基线一致")
    return 0


if __name__ == "__main__":
    sys.exit(main())
