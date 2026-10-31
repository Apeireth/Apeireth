#!/usr/bin/env python3
"""`mod` 行 ↔ **文件入库性**守卫（防「引用提交了、文件没提交」）。

**为什么要有它（2026-10-26 实战）**：
总指挥提交 `crates/engine/memory/src/lib.rs` 时，把**子代理并行写进工作树的**
`pub mod git_corpus;` 一并带走了，而 `git_corpus.rs` 本体当时**还是未跟踪文件**
⇒ ⇒ **引用提交了、文件没提交** ⇒ 那两个 commit 单独 checkout **编译不过**
⇒ ⇒ 连带 **PR #94 的 10 个 workflow 全红**。

**关键设计（我踩了两次假阳性之后才想清楚的）**：
- **「文件存不存在」由编译器管**（`cargo check` 会立刻报 `file not found for module`）
  ⇒ ⇒ **本脚本不重复做这件事** ⇒ 否则我得手写 Rust 2018 的模块路径解析
  （stem 目录规则 / 行内模块嵌套层 / 集成测试是独立 crate root / 文档注释里的花括号…）
  ⇒ ⇒ **我第一版就是这么做的 ⇒ 88 处假阳性**；两次修正后仍有 10 处
  ⇒ ⇒ **证明手写解析器不可靠** ⇒ ⇒ 交给编译器。
- **编译器查不到的那一件**：**文件在工作树里存在，但没进 git**
  ⇒ ⇒ 这才是「静默编译空洞」的真正成因 ⇒ ⇒ **本脚本只查这一件**。

**判定**：对每个 `mod X;` / `pub mod X;` 声明，沿目录链找**可能的**后继文件
（`X.rs` / `X/mod.rs`；**候选存在即认为解析成功**，因为存在性由编译器兜底）
⇒ 若候选文件**未被 git 跟踪** ⇒ **报错**。

**诚实边界**：
- 只查 `mod` 声明，**不查 `include!` 宏**；
- `#[path = "..."]` 改写了模块文件位置 ⇒ **跳过**该声明（规则不适用）；
- 「候选存在即认为解析成功」是**宽松**判定：万一解析到别的同名文件，
  **编译器仍会兜住**（它报的是真实路径）⇒ ⇒ 本脚本**只负责「没入库」这一件**。

用法:
    python3 scripts/check_mod_files.py            # 报告（有问题则退出码 1）
    python3 scripts/check_mod_files.py --quiet    # 只在有问题时输出
"""
import argparse
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

# `mod X;` / `pub mod X;`（**行内模块** `mod X {` 不匹配 ⇒ 末尾必须是分号）
MOD_DECL_RE = re.compile(r"^\s*(?:pub(?:\([^)]*\))?\s+)?mod\s+([A-Za-z_][A-Za-z0-9_]*)\s*;", re.M)
# `#[path = "..."]`（改写模块文件位置 ⇒ 规则不适用 ⇒ 跳过该声明）
PATH_ATTR_RE = re.compile(r'^\s*#\s*\[\s*path\s*=\s*"')

# 只查这些目录下的 Rust 源码（`docs/` 下的试验田是**独立 cargo 工作区**，不查）
SCAN_ROOTS = ("crates", "benches", "tools")


def tracked_files() -> set[str]:
    try:
        out = subprocess.run(
            ["git", "-C", str(ROOT), "ls-files"],
            capture_output=True,
            text=True,
            timeout=60,
            check=False,
        ).stdout
    except Exception:
        return set()
    return {line for line in out.splitlines() if line.strip()}


def candidate_bases(src: Path) -> list[Path]:
    """可能的模块解析基目录（**由宽到窄**）：
    - 集成测试 / 基准：文件所在目录（它们是**独立 crate root**）
    - 普通模块文件：`<stem>/`（Rust 2018：以文件自己的 stem 命名的目录）
    - 再加上沿目录链向上（覆盖行内模块嵌套层 ⇒ 我不精确算栈，用**宽松搜索**代替）
    """
    bases: list[Path] = []
    parent = src.parent
    bases.append(parent)
    if src.stem not in ("lib", "main", "mod"):
        bases.append(parent / src.stem)
    current = parent
    for _ in range(3):
        if current == ROOT:
            break
        current = current.parent
        if ROOT not in current.parents:
            break
        bases.append(current)
        if current.stem not in ("lib", "main", "mod"):
            bases.append(current / current.stem)
    return bases


def main() -> int:
    parser = argparse.ArgumentParser(description="mod 声明 ↔ 文件入库性检查")
    parser.add_argument("--quiet", action="store_true", help="只在有问题时输出")
    args = parser.parse_args()

    tracked = tracked_files()
    problems: list[str] = []
    checked = 0
    resolved = 0

    for scan_root in SCAN_ROOTS:
        base = ROOT / scan_root
        if not base.exists():
            continue
        for src in base.rglob("*.rs"):
            try:
                text = src.read_text(encoding="utf-8", errors="replace")
            except Exception:
                continue
            rel_src = str(src.relative_to(ROOT))
            lines = text.splitlines()
            skip_next = False
            for line in lines:
                if skip_next:
                    skip_next = False
                    continue
                if PATH_ATTR_RE.match(line):
                    # `#[path = ...]` 可能与下一行的 `mod X;` 配对 ⇒ 跳过下一条声明
                    skip_next = True
                    continue
                decl = MOD_DECL_RE.match(line)
                if not decl:
                    continue
                name = decl.group(1)
                checked += 1
                found: Path | None = None
                for candidate_base in candidate_bases(src):
                    for candidate in (
                        candidate_base / f"{name}.rs",
                        candidate_base / name / "mod.rs",
                    ):
                        if candidate.exists():
                            found = candidate
                            break
                    if found:
                        break
                if found is None:
                    # 「不存在」由编译器兜底（cargo check 会报 file not found）⇒
                    # **不在这里报**（我第一版因此产生 88 处假阳性）。
                    continue
                resolved += 1
                rel_found = str(found.relative_to(ROOT))
                if rel_found not in tracked:
                    problems.append(
                        f"{rel_src}: `mod {name};` 指向的 **{rel_found} 未被 git 跟踪**"
                        f" ⇒ **引用提交了、文件没提交** ⇒ 该 commit 单独 checkout 编译不过"
                    )

    if not problems:
        if not args.quiet:
            print(
                f"[mod-files] ✅ {checked} 个 `mod` 声明（解析到 {resolved} 个文件）："
                f"**全部已被 git 跟踪**"
            )
        return 0

    print(f"[mod-files] ❌ {len(problems)} 处：")
    for line in problems:
        print(f"   {line}")
    print("   → 「文件在树里但没进 git」是编译器**查不到**的那种洞（只在 checkout 别处时才炸）")
    print("   → 提交任何 `lib.rs`/`mod.rs` 前先跑本脚本（或 `git add` 后看 `git status` 有无对应 `??`）")
    return 1


if __name__ == "__main__":
    sys.exit(main())
