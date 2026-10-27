#!/usr/bin/env python3
"""检查 `mod xxx;` 声明指向的源文件**确实存在**（且没有同名文件的孤儿）。

# 为什么要有这条守卫

2026-10-26 实测事故：提交 `crates/engine/memory/src/lib.rs` 时，
工作树里已有一行并行开发加进来的 `pub mod git_corpus;`，
但 `git_corpus.rs` **本体当时还是未跟踪文件** ⇒
**引用提交了、文件没提交** ⇒ 那两个 commit 单独 checkout **编译不过**
⇒ 连带 PR 的 **10 个 workflow 全红**（clippy / fmt / rustdoc / test --doc / tarpaulin …）。

这类错误的危险在于：**它发生在提交的一瞬**，而 CI 只在**推上去之后**才告诉你。

⇒ 本守卫把这条不变量变成**提交前**就能查的东西：
**每个 `mod` 行指向的文件，必须与该 `mod` 行在同一个 commit 里。**

# 检查什么

对每个 `src/lib.rs`（与 `src/main.rs`）：

1. 抽出所有 `mod <name>;` 声明（含 `pub mod`）；
2. 解析每个 `<name>` 应该对应的文件：
   `<dir>/<name>.rs`（模块文件）或 `<dir>/<name>/mod.rs`（目录模块）；
3. 文件**不存在** ⇒ 报 **MISSING**（这就是那次事故的形态）；
4. 存在 `<dir>/<name>.rs` 但同目录**另有** `<name>/mod.rs` ⇒ 报 **DUPLICATE**
   （Rust 会直接报 "file for module found at both" ⇒ 编译不过）。

⚠️ **不检查「`<name>/` 目录存在但没有 mod.rs」**：`foo.rs` + `foo/` 是 Rust 的
**标准布局**（`foo.rs` 内的 `mod bar;` ⇒ `foo/bar.rs`）⇒ 那是常态，不是问题。

# 退出码

* `0` = 全部通过；
* `1` = 有问题（打印清单）。

# 用法

```bash
python3 scripts/check_mod_targets.py           # 检查全仓
python3 scripts/check_mod_targets.py --root crates/engine/memory   # 只查一棵子树
```
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent

# `mod foo;` / `pub mod foo;` / `pub(crate) mod foo;`
MOD_RE = re.compile(r"^\s*(?:pub(?:\([^)]*\))?\s+)?mod\s+([A-Za-z_][A-Za-z0-9_]*)\s*;", re.MULTILINE)

# `#[path = "..."]` 覆盖（自定义文件路径）—— 出现时跳过该 mod（路径不按惯例推导）。
PATH_ATTR_RE = re.compile(r"#\s*\[\s*path\s*=")


def candidate_files(module_dir: Path, name: str) -> list[Path]:
    """一个 `mod <name>;` 可能对应的文件（按 Rust 惯例）。"""
    return [module_dir / f"{name}.rs", module_dir / name / "mod.rs"]


def display_path(path: Path) -> str:
    """给人看的路径：仓内显示相对路径，仓外显示绝对路径。

    ⚠️ **守卫自己不能崩**：`--root` 可以指向仓外（测试用）⇒
    直接 `relative_to(REPO_ROOT)` 会抛 `ValueError` ⇒ 守卫在**最需要它的时候**挂掉。
    """
    try:
        return str(path.relative_to(REPO_ROOT))
    except ValueError:
        return str(path)


def check_tree(root: Path) -> list[str]:
    """检查 root 下所有 `lib.rs` / `main.rs` 的 mod 声明。返回问题清单。"""
    problems: list[str] = []
    manifests = sorted(set(root.rglob("src/lib.rs")) | set(root.rglob("src/main.rs")))
    # ⚠️ **`legacy/` 是归档区**（不参与构建）⇒ 不查它的模块声明。
    manifests = [m for m in manifests if "legacy" not in m.parts]
    for manifest in manifests:
        text = manifest.read_text(encoding="utf-8", errors="replace")
        # 按行切分，逐行判断该 mod 是否带 `#[path]` 覆盖。
        for line_match in MOD_RE.finditer(text):
            name = line_match.group(1)
            # 往上找最近的非空行，看是不是 `#[path = ...]`（同一语句的属性）。
            prefix = text[: line_match.start()].rstrip()
            if PATH_ATTR_RE.search(prefix.split("\n")[-1] if prefix else ""):
                # 有 `#[path]` 覆盖 ⇒ 文件名不按惯例 ⇒ 跳过（但仍要确保文件在）。
                continue
            module_dir = manifest.parent
            candidates = candidate_files(module_dir, name)
            existing = [candidate for candidate in candidates if candidate.exists()]
            if not existing:
                wanted = " 或 ".join(display_path(candidate) for candidate in candidates)
                problems.append(
                    f"MISSING  {display_path(manifest)}: `mod {name};` "
                    f"指向的文件都不存在（应为 {wanted}）"
                )
                continue
            if len(existing) > 1:
                found = " / ".join(display_path(candidate) for candidate in existing)
                problems.append(
                    f"DUPLICATE {display_path(manifest)}: `mod {name};` "
                    f"有多个候选文件（{found}）⇒ 只有一个会被编译，另一个是死文件"
                )
                continue
            # ⚠️ **不检查「`<name>/` 目录存在但没有 mod.rs」**：
            # `foo.rs` + `foo/` 目录是 Rust 的**标准布局**（`foo.rs` 里的
            # `mod bar;` ⇒ `foo/bar.rs`）⇒ 那不是问题，是常态。
        problems.extend(find_orphan_module_files(manifest))
    return problems


def find_orphan_module_files(manifest: Path) -> list[str]:
    """找「文件在、但没有任何 `mod` 声明指向它」的模块文件。

    # 口径

    * **按目录**判定：同一目录下可能有**多个 crate 根**（`lib.rs` 与 `main.rs`
      共享 `src/`）⇒ 一个文件只要被**任一**根声明就算「已声明」；
    * 只看 crate 根**同目录**下的 `*.rs`（不含 `lib.rs`/`main.rs`）。
    """
    problems: list[str] = []
    root_dir = manifest.parent
    declared: set[str] = set()
    for sibling_root in sorted(root_dir.glob("lib.rs")) + sorted(root_dir.glob("main.rs")):
        text = sibling_root.read_text(encoding="utf-8", errors="replace")
        declared.update(match.group(1) for match in MOD_RE.finditer(text))
    for candidate in sorted(root_dir.glob("*.rs")):
        stem = candidate.stem
        if stem in ("lib", "main"):
            continue
        # ⚠️ **白名单**：`*_tests.rs` / `tests_*.rs` 命名 ⇒ 本仓惯例是
        # 「**放在 src/ 里的测试模块**」，由 `#[cfg(test)] mod` 内联引用
        # ⇒ 不该按「crate 根声明」判它们。
        if stem.endswith("_tests") or stem.startswith("tests_"):
            continue
        if stem not in declared:
            problems.append(
                f"ORPHAN  {display_path(candidate)}: 文件存在，但同目录下没有任何 "
                f"`mod {stem};` / `pub mod {stem};` 声明 ⇒ **它不会被编译**"
                f"（若它的测试引用了它 ⇒ 远端编译失败）"
            )
    return problems


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--root",
        default=str(REPO_ROOT),
        help="要检查的子树根（默认：整个仓库）",
    )
    args = parser.parse_args()
    root = Path(args.root).resolve()
    if not root.exists():
        print(f"路径不存在：{root}", file=sys.stderr)
        return 1
    problems = check_tree(root)
    if problems:
        print(f"[check_mod_targets] 发现 {len(problems)} 处问题：", file=sys.stderr)
        for problem in problems:
            print(f"  {problem}", file=sys.stderr)
        return 1
    print("OK: 所有 `mod` 声明都指向存在的源文件")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
