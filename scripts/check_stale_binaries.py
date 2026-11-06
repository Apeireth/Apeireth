#!/usr/bin/env python3
"""扫出**陈旧测试二进制**（编译于已不存在的路径）—— 防止「假红 / 假绿」。

# 为什么要有这个脚本

`env!("CARGO_MANIFEST_DIR")` 是**编译期**常量 ⇒ 它被**烙进**二进制里。

⇒ **worktree 迁移**（如从 `/tmp/apeireth-r3` 搬到 `/home/bai/wt-apeireth-r3`）后，
`cargo` **不会**因为「路径变了」而重新编译（**构建指纹里不含 `CARGO_MANIFEST_DIR`**）
⇒ ⇒ `target/` 里会**留下**编译于**旧路径**的陈旧二进制
⇒ ⇒ 那些测试运行时去读**已不存在的目录**里的文件 ⇒ **`NotFound`**
⇒ ⇒ 表现为「**某条测试在本地全量跑时失败、单独跑却通过**」的**幽灵红**。

⇒ 本脚本扫 `target/*/deps/` 里的**可执行文件**，找出**嵌了已不存在路径**的那些
⇒ ⇒ 报出来（并可 `--clean` 清掉 ⇒ 让 cargo 重新编出正确的）。

# 实测到的两个实例（2026-10-27）

* `target/debug/deps/guard_v32_evaluation-8dafb20bd6a9a6ad`
  ⇒ 嵌 `/tmp/apeireth-r3/crates/engine/guard/...`
  ⇒ 运行时读 `/tmp/apeireth-r3/artifacts/guard-joint-shadow-v0.json` ⇒ **NotFound**
  ⇒ 表现为 `trained_artifact_hash_verifies` 失败；
* `target/debug/deps/regen-d00c0694522971fb`
  ⇒ 嵌 `/tmp/apeireth-r3/crates/engine/image/cms/...`
  ⇒ 运行时读 `/tmp/apeireth-r3/crates/engine/image/cms/profiles/coated-cmyk-synth.icc`
  ⇒ **NotFound** ⇒ 表现为 `regen_or_check_coated_cmyk` 失败。

⇒ ⚠️ 这两条**单独跑必过、全量跑偶发/必现失败** ⇒ 极易被误判成「产品坏了」。

# 口径

* 只看**可执行文件**（`os.access(path, os.X_OK)` 且不是 `.d`/`.rlib`/`.rmeta`）；
* 只看**确实嵌了路径**的文件（用二进制字节搜索，避免误报普通字符串）；
* 判定「陈旧」= 该路径**在当前文件系统上不存在**；
* ⚠️ **不**清理**当前 worktree** 的路径（它存在 ⇒ 不算陈旧）。

# 用法

```sh
python3 scripts/check_stale_binaries.py            # 只查（退出码 1 = 有陈旧产物）
python3 scripts/check_stale_binaries.py --clean    # 查 + 删（让 cargo 重编）
python3 scripts/check_stale_binaries.py --report   # 人话报告
```
"""

from __future__ import annotations

import argparse
import os
import re
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
TARGET_DIRS = [REPO_ROOT / "target" / "debug" / "deps", REPO_ROOT / "target" / "release" / "deps"]

# 只关心「**像工作树路径**」的绝对路径前缀（避免把系统路径当陈旧产物）。
PATH_PREFIX_HINTS = (b"/tmp/apeireth", b"/home/bai/wt-apeireth", b"/home/bai/Projects/Apeireth")

# 绝对路径候选（正则，C 速度 ⇒ 扫几百 MB 也只要几秒）。
PATH_RE = re.compile(rb"/[A-Za-z0-9_./+-]{6,}")

# 「真绝对路径」的根（只有这些根下的路径才可能是**工作树路径**）。
ABSOLUTE_ROOTS = (
    "/tmp/apeireth",
    "/home/bai/wt-apeireth",
    "/home/bai/Projects/Apeireth",
    "/home/bai/Projects/apeireth",
)


def looks_like_worktree_path(path: str) -> bool:
    """这条路径**像**工作树路径吗？（含 `crates/` 或 `target/` ⇒ 是）

    # 为什么不用「硬编码根前缀」

    实测漏过 `/tmp/w21-verify/...`（**另一个**旧 worktree）⇒
    硬编码前缀会**漏** ⇒ ⇒ 改成「**形状**判定」⇒ 任何旧 worktree 都盖得住。
    """
    return "/crates/" in path or path.endswith("/crates") or "/target/" in path


def embedded_paths(blob: bytes) -> set[str]:
    """从文本里抽出**候选绝对路径**。

    ⚠️ **性能**：用**正则**（C 实现）⇒ `.d` 文件总量几百 MB 也能**秒级**扫完
    ⇒ 才配当**日常门禁**。
    """
    found: set[str] = set()
    for match in PATH_RE.finditer(blob):
        found.add(match.group(0).decode("ascii", errors="ignore"))
    return found


def scan() -> list[tuple[Path, str]]:
    """⇒ [(二进制, 已不存在的嵌入路径)]

    # ⚠️ **为什么扫 `.d` 文件**（不是扫二进制本身）

    * 二进制**很大**（`target/debug/deps` 动辄 10+ GB）⇒ 逐个读要好几分钟
      ⇒ 没法当**日常门禁**；
    * `.d`（依赖清单）**很小**（文本）⇒ 而且**精确**记录了**编译时的路径**
      ⇒ ⇒ 用它判定「这个产物是**在哪**编出来的」**又快又准**。

    ⇒ `.d` 里出现**已不存在的路径** ⇒ 该产物是**陈旧的**（从那个旧路径搬来的）
    ⇒ ⇒ 它的 `CARGO_MANIFEST_DIR` 烙的是**旧路径** ⇒ 运行时会读到 NotFound。
    """
    stale: list[tuple[Path, str]] = []
    for deps in TARGET_DIRS:
        if not deps.is_dir():
            continue
        for dep_file in sorted(deps.glob("*.d")):
            try:
                text = dep_file.read_text(encoding="utf-8", errors="replace")
            except OSError:
                continue
            for path in embedded_paths(text.encode("utf-8", errors="replace")):
                cut = len(path)
                for separator in (":", " ", "\n"):
                    index = path.find(separator)
                    if index > 0:
                        cut = min(cut, index)
                candidate_path = path[:cut]
                if not candidate_path or candidate_path == str(REPO_ROOT):
                    continue
                # ⚠️ **只认真工作树路径**（含 `crates/` 或 `target/` 的绝对路径）
                # ⇒ 否则 `crates/engine/guard/tests/x.rs` 这种**相对片段**会被误判
                # ⇒ 而**硬编码根前缀**会漏掉别的旧 worktree（实测漏过 `/tmp/w21-verify`）
                # ⇒ ⇒ 判据：**看起来像工作树路径** + **该路径已不存在**。
                if not looks_like_worktree_path(candidate_path):
                    continue
                if not Path(candidate_path).exists():
                    binary = dep_file.with_suffix("")
                    if binary.exists():
                        stale.append((binary, candidate_path))
                    break
    return stale


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--clean", action="store_true", help="删掉陈旧二进制（让 cargo 重编）")
    parser.add_argument("--report", action="store_true", help="只打印报告")
    args = parser.parse_args()

    stale = scan()
    if not stale:
        print("OK: 没有陈旧测试二进制（target/ 里没有嵌着已消失路径的产物）")
        return 0

    print(f"发现 **{len(stale)}** 个陈旧二进制（编译于已不存在的路径 ⇒ 运行时会读到 NotFound）：")
    for binary, path in stale:
        print(f"  · {binary.relative_to(REPO_ROOT)}")
        print(f"      嵌的路径已不存在：{path}")

    if args.clean:
        removed = 0
        for binary, _ in stale:
            try:
                binary.unlink()
                removed += 1
            except OSError as error:
                print(f"  ! 删不掉 {binary.name}：{error}", file=sys.stderr)
        print(f"  ✓ 已删 {removed} 个 ⇒ 重新跑 cargo 会编出正确的（路径是新的）")
        return 0

    print("\n⇒ 这些产物会让「**全量跑**」出现幽灵红（**单独跑必过**）")
    print("⇒ 修法：python3 scripts/check_stale_binaries.py --clean")
    return 1


if __name__ == "__main__":
    sys.exit(main())
