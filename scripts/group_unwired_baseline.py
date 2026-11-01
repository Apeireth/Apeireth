#!/usr/bin/env python3
"""把孤儿 API 基线按「为什么零引用」分组。

## 为什么需要这个工具

`scripts/unwired-baseline.txt` 的分组（哪些是公开 API 表面、哪些是集成测试辅助、
哪些是待判的真债）**是人写的判断依据**，不是工具能推导出来的。但手工重分组容易
出错 —— 一次手滑就会把「已核实有主路径的」重新丢回待判组。

所以把分组规则写成**可复现的脚本**（规则集中在 `unwired_baseline_common.py`）：
- 判定规则（按文件路径、符号前缀、显式清单）集中维护；
- 跑完自动自校验：**每条基线条目必须恰好落进一个组**（不重不漏）；
- 人工判断的地方（`VERIFIED_REDUNDANT_OR_HOST`）用**显式清单**列出，
  并在注释里写清依据（主路径的文件:行号）。

## 用法

```
python3 scripts/group_unwired_baseline.py            # 重排 + 自校验
python3 scripts/group_unwired_baseline.py --check    # 只校验，不写文件
```

## 维护约定

- 改动分组规则后，**必须**同时更新 `docs/04-internal/backend-enhancement/
  unwired-debt-taxonomy-2026-10-24.md`（那份文档是分组理由的正文）；
- 新增条目若靠「显式清单」归组，要在清单里写清**依据**，不许只写名字 ——
  否则下一个来的人无法复核。
"""

from __future__ import annotations

import argparse
import pathlib
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))

from unwired_baseline_common import (  # noqa: E402
    BASELINE,
    GROUP_DESC,
    classify,
    find_file,
    render,
    split_entry,
)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true", help="只校验，不写文件")
    args = parser.parse_args()

    if not BASELINE.exists():
        print(f"基线不存在: {BASELINE}", file=sys.stderr)
        return 1

    entries: dict[str, str] = {}
    symbols: list[str] = []
    for raw_line in BASELINE.read_text(encoding="utf-8").splitlines():
        stripped = raw_line.strip()
        if not stripped or stripped.startswith("#"):
            continue
        _, symbol, _, reason = split_entry(stripped)
        if not symbol:
            continue
        if symbol in entries:
            print(f"基线里有重复符号: {symbol} —— 先去重", file=sys.stderr)
            return 1
        entries[symbol] = reason
        symbols.append(symbol)

    groups: dict[str, list[str]] = {group: [] for group in GROUP_DESC}
    for symbol in symbols:
        groups[classify(symbol, find_file(symbol))].append(symbol)

    # 自校验：每条恰好落进一个组，且总数守恒
    total = sum(len(items) for items in groups.values())
    if total != len(symbols):
        print(
            f"分组自校验失败：{total} != {len(symbols)}（有条目落不进任何组）",
            file=sys.stderr,
        )
        return 1

    rendered = render(entries, groups)

    if args.check:
        current = BASELINE.read_text(encoding="utf-8")
        if current != rendered:
            print("分组与 `group_unwired_baseline.py` 的结果不一致 —— 跑一次不带 --check")
            return 1
        print(f"分组一致（{total} 条）")
        return 0

    BASELINE.write_text(rendered, encoding="utf-8")
    print("分组完成：")
    for group in sorted(groups):
        print(f"  {group}: {len(groups[group])}")
    print(f"  合计: {total}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
