#!/usr/bin/env python3
"""清 cargo **测试二进制**（可再生产物）⇒ ⇒ 治「磁盘满 ⇒ 文件被静默截断」那个坑。

# 为什么需要它（实测的坑）
2026-11-06：磁盘 100% 时，本仓的生成脚本写文件**不报错** ⇒ ⇒ 留下 0 字节 / 半截的 JSON
⇒ ⇒ 症状是**下游报 `JSONDecodeError`** ⇒ ⇒ 我第一反应「生成脚本坏了」⇒ ⇒ **错**
⇒ ⇒ ⇒ **真因在磁盘** ⇒ ⇒ ⇒ 所以「跑完验证就把测试二进制清掉」是**根治**动作。

# 为什么只清「测试二进制」
* cargo 的磁盘占用 **90% 来自测试二进制**（每个测试 = 一个可执行文件）
  ⇒ ⇒ 实测：一次全量 nextest ⇒ ⇒ **1129 个 executables = 256.3 G**（vs 17.0 G 的 `.rlib`）
  ⇒ ⇒ ⇒ 跑完不清理 ⇒ ⇒ 磁盘很快见底；
* ⚠️ **必须保留 `*.so`** ⇒ ⇒ 那些是 **proc-macro**（动态库）
  ⇒ ⇒ 删了 ⇒ ⇒ 并发编译时**所有宏都要重编** ⇒ ⇒ 反而更慢；
* ⚠️ **不删 `debug/incremental`** ⇒ ⇒ 它正被 rustc 用着 ⇒ ⇒ 删它可能打断别人的编译
  ⇒ ⇒ ⇒ 只清**最终产物**（测试二进制）⇒ ⇒ **不打断任何正在跑的 cargo/rustc**。

# 安全判定（⭐ 「正在运行」精确到 inode）
* 扫 `/proc/<pid>/exe` ⇒ ⇒ 收集**正在被执行**的测试二进制路径
  ⇒ ⇒ **排除**它们 ⇒ ⇒ ⇒ 删掉正在跑的那个 ⇒ ⇒ 会让测试进程崩 ⇒ ⇒ **必须排除**；
* 其余 `debug/deps/*` 里的**可执行普通文件**（排除 `*.so`）⇒ ⇒ 一律删；
* ⇒ ⇒ 只删**构建产物** ⇒ ⇒ **源码零影响**。

# 用法
```sh
python3 scripts/clean_test_binaries.py            # 真清（默认）
python3 scripts/clean_test_binaries.py --dry-run  # 只报告
```
"""

from __future__ import annotations

import argparse
import os
import pathlib
import sys
from collections import defaultdict

# 候选 target 根目录（**按存在性过滤** ⇒ ⇒ 不假设布局）
CANDIDATE_ROOTS = [
    pathlib.Path("/home/bai"),
    pathlib.Path("/var/tmp"),
    pathlib.Path("/tmp"),
]


def running_executables() -> set[str]:
    """**正在被执行**的可执行文件路径集合（扫 `/proc/<pid>/exe`）。

    ⇒ ⇒ 精确到 inode ⇒ ⇒ 「某个测试二进制正在跑」能被识别出来
    ⇒ ⇒ ⇒ 绝不能删它（会让那个进程崩）。
    """
    out: set[str] = set()
    for entry in pathlib.Path("/proc").iterdir():
        if not entry.name.isdigit():
            continue
        try:
            target = os.readlink(entry / "exe")
        except OSError:
            continue
        out.add(target)
    return out


def candidate_targets() -> list[pathlib.Path]:
    """所有**存在的** `target` 目录（深度有限 ⇒ ⇒ 别扫穿整个盘）。"""
    found: list[pathlib.Path] = []
    for root in CANDIDATE_ROOTS:
        if not root.is_dir():
            continue
        try:
            children = list(root.iterdir())
        except OSError:
            continue
        for child in children:
            for cand in (child, child / "target"):
                if cand.is_dir() and (cand / "debug" / "deps").is_dir():
                    found.append(cand)
                    break
    return sorted(set(found))


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--dry-run", action="store_true", help="只报告，不删")
    args = ap.parse_args()

    busy = running_executables()
    busy_resolved: set[str] = set()
    for p in busy:
        try:
            busy_resolved.add(str(pathlib.Path(p).resolve()))
        except OSError:
            busy_resolved.add(p)

    grand_files = 0
    grand_bytes = 0
    per: dict[str, tuple[int, int]] = defaultdict(lambda: (0, 0))

    for target in candidate_targets():
        deps = target / "debug" / "deps"
        files = 0
        size = 0
        try:
            entries = list(deps.iterdir())
        except OSError:
            continue
        for f in entries:
            try:
                if not f.is_file() or f.suffix == ".so":
                    continue
                st = f.stat()
                if not (st.st_mode & 0o111):  # 不可执行 ⇒ 不是测试二进制
                    continue
                try:
                    real = str(f.resolve())
                except OSError:
                    real = str(f)
                if real in busy_resolved:
                    continue  # ⭐ 正在运行 ⇒ 不删
                files += 1
                size += st.st_size
                if not args.dry_run:
                    f.unlink(missing_ok=True)
            except OSError:
                continue
        if files:
            per[str(target)] = (files, size)
            grand_files += files
            grand_bytes += size

    verb = "可清" if args.dry_run else "已清"
    for t, (n, s) in sorted(per.items(), key=lambda kv: -kv[1][1]):
        print(f"  · {t}: {verb} {n} 个测试二进制（{s / 1024 ** 3:.1f} G）")
    print(
        f"  ⇒ ⇒ {verb} **{grand_files}** 个测试二进制（**{grand_bytes / 1024 ** 3:.1f} G**）"
        + ("（dry-run ⇒ 未真删）" if args.dry_run else "")
        + f" · 正在运行的（{len(busy_resolved)} 个可执行）**已排除**"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
