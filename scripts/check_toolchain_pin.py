#!/usr/bin/env python3
"""**CI 工具链钉版守卫**（2026-10-30）。

# 为什么要有这个脚本

`rust-toolchain.toml` 声明 `channel = "1.97.1"`（本地**确实**用这一版，
`rustup show active-toolchain` 可验），但 CI 的 35 个 workflow 写的是
`dtolnay/rust-toolchain@stable` ⇒ **CI 装的是浮动版本** ⇒

* 「可重现构建」在 **CI 里没兑现**（本地钉、CI 不钉）；
* 文件头那句「CI 仍用 @stable（per … 33 处）」的**计数也过期**（实为 35 处）。

⇒ ⇒ 本脚本把「**CI 用的版本 == 仓库声明的版本**」变成**机器判定**：

* 出现**浮动**引用（`@stable` / `@beta`）⇒ **失败**；
* 出现**版本号**引用但与 `rust-toolchain.toml` 的 `channel` 不一致 ⇒ **失败**；
* `@nightly` 允许（miri / kani 等确需 nightly），但**必须显式**（不许 `@nightly` 之外的浮动别名）。

# 为什么允许「两处都写版本号」

单一真源是 `rust-toolchain.toml`；workflow 里那 35 处**只能写版本号**
（GitHub Actions 的 `uses:` 不能引用仓库内文件）⇒ 这是**不可避免的重复**
⇒ ⇒ 与其追求「不重复」，不如让重复**被机器盯着**（这与本仓其它棘轮同思路）。

# 用法

    python3 scripts/check_toolchain_pin.py
"""

from __future__ import annotations

import re
import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
TOOLCHAIN_FILE = ROOT / "rust-toolchain.toml"
WORKFLOWS = ROOT / ".github" / "workflows"

#: 允许的**浮动**别名（仅 nightly：miri / kani 等确需 nightly 工具链）。
ALLOWED_FLOATING = {"nightly"}

#: 禁止出现的**浮动**别名（stable/beta 会随时间漂移 ⇒ 与「可重现」矛盾）。
FORBIDDEN_FLOATING = {"stable", "beta"}

REF_PATTERN = re.compile(r"dtolnay/rust-toolchain@([A-Za-z0-9._-]+)")


def declared_channel() -> str:
    try:
        with TOOLCHAIN_FILE.open("rb") as handle:
            data = tomllib.load(handle)
    except FileNotFoundError:
        print(f"FAIL: 找不到 {TOOLCHAIN_FILE}", file=sys.stderr)
        raise SystemExit(2)
    except tomllib.TOMLDecodeError as error:
        print(f"FAIL: {TOOLCHAIN_FILE.name} 解析失败：{error}", file=sys.stderr)
        raise SystemExit(2)
    channel = data.get("toolchain", {}).get("channel")
    if not isinstance(channel, str) or not channel:
        print(f"FAIL: {TOOLCHAIN_FILE.name} 没有 `toolchain.channel`", file=sys.stderr)
        raise SystemExit(2)
    return channel


def main() -> int:
    channel = declared_channel()
    problems: list[str] = []
    pinned = 0
    floating = 0
    nightly = 0

    for path in sorted(WORKFLOWS.glob("*.yml")):
        text = path.read_text(encoding="utf-8")
        for match in REF_PATTERN.finditer(text):
            ref = match.group(1)
            line = text[: match.start()].count("\n") + 1
            where = f"{path.relative_to(ROOT)}:{line}"
            if ref in FORBIDDEN_FLOATING:
                problems.append(
                    f"{where}: 浮动引用 `@{ref}` ⇒ CI 装的版本会随时间漂移"
                    f"（仓库声明 `{channel}`）⇒ 改成 `@{channel}`"
                )
                floating += 1
            elif ref in ALLOWED_FLOATING:
                nightly += 1
            elif ref == channel:
                pinned += 1
            else:
                problems.append(
                    f"{where}: 版本 `@{ref}` 与 `rust-toolchain.toml` 的"
                    f" `channel = \"{channel}\"` **不一致**"
                )

    if problems:
        print(f"FAIL: CI 工具链钉版守卫（{len(problems)} 处）", file=sys.stderr)
        for line in problems:
            print(f"  · {line}", file=sys.stderr)
        print(
            f"\n修法：把 `@stable` 改成 `@{channel}`（本地已验证这一版全绿）"
            f"；或改 `rust-toolchain.toml` 后**同步**所有 workflow。",
            file=sys.stderr,
        )
        return 1

    print(
        f"OK: CI 工具链钉版一致（仓库声明 `{channel}` · workflow 钉住 {pinned} 处 · "
        f"nightly {nightly} 处（允许）· 浮动 {floating} 处）"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
