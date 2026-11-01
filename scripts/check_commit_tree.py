#!/usr/bin/env python3
"""提交完整性自检：**提交后的树**里必须有的文件都在。

## 为什么需要这个工具

2026-10-24 踩过一个大坑，值得记下来：

1. 新建了三个仓库常驻脚本（`unwired_baseline_common.py` 等）；
2. `.gitignore` 里有 `*.py`（为了挡 sub-agent scratch），仓库工具靠逐个
   `!scripts/xxx.py` 例外放行；
3. 我新建的脚本**忘了加例外** ⇒ `git add -A` 静默跳过它们，
   `git status` 也不显示（被 ignore 规则吃掉）；
4. **本地全绿**（文件在磁盘上，跑得通）⇒ 以为提交完整；
5. 推上去后 CI 的「孤儿 API 棘轮」**红了** —— 因为 `audit_unwired.py`
   import 了那个没进提交的文件。

**教训**：本地绿 ≠ 提交完整。`.gitignore` 的静默忽略比报错更危险 ——
它让「我以为提交了」和「实际提交了」之间出现**没人察觉的差距**。

所以这条纪律：**提交后必须验证提交树本身**，而不是信任工作区状态。

## 用法

```
python3 scripts/check_commit_tree.py               # 检查 HEAD 的树
python3 scripts/check_commit_tree.py --expect <路径>  # 额外要求某些文件在树里
```

## 判据

1. 工作区里存在、但**不在 HEAD 树里**、且**不是被 ignore** 的 `.py` 文件
   ⇒ 报出来（这正是上面那次事故的形态）；
2. `--expect` 列出的路径必须都在树里（缺一即失败）；
3. 被 `.gitignore` 明确忽略的文件**不算**问题（scratch 文件是合法的），
   但如果它是 `scripts/` 下的**仓库工具**（即 `audit_unwired.py` 等已跟踪文件的同目录），
   仍然报出来 —— 因为那正是「忘了加例外」的形态。
"""

from __future__ import annotations

import argparse
import pathlib
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent


def head_tree_files() -> set[str]:
    out = subprocess.run(
        ["git", "ls-tree", "-r", "--name-only", "HEAD"],
        capture_output=True,
        text=True,
        cwd=ROOT,
        check=False,
    ).stdout
    return {line for line in out.splitlines() if line}


def is_ignored(path: pathlib.Path) -> bool:
    result = subprocess.run(
        ["git", "check-ignore", "-q", str(path.relative_to(ROOT))],
        cwd=ROOT,
        check=False,
    )
    return result.returncode == 0


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--expect",
        action="append",
        default=[],
        help="必须在 HEAD 树里存在的路径（可重复）",
    )
    args = parser.parse_args()

    tracked = head_tree_files()
    problems: list[str] = []

    # ① 必须存在的路径
    for rel in args.expect:
        if rel not in tracked:
            problems.append(f"必须存在但不在 HEAD 树里: {rel}")

    # ② 工作区里有、但没进提交、且**不是**被 ignore 的 .py
    for path in sorted((ROOT / "scripts").glob("*.py")):
        rel = str(path.relative_to(ROOT))
        if rel in tracked:
            continue
        if is_ignored(path):
            problems.append(
                f"scripts/ 下的仓库工具被 .gitignore 忽略（会静默漏提交）: {rel}"
            )
            continue
        problems.append(f"工作区有但不在 HEAD 树里: {rel}")

    if problems:
        print("提交树完整性检查失败：", file=sys.stderr)
        for item in problems:
            print(f"  - {item}", file=sys.stderr)
        print(
            "\n提示：.gitignore 的 `*.py` 会静默吃掉新建脚本。"
            "已改成 `!scripts/*.py` 整目录放行；若仍失败，用 `git add -f` 显式加入。",
            file=sys.stderr,
        )
        return 1

    print(f"提交树完整（HEAD 含 {len(tracked)} 个文件）")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
