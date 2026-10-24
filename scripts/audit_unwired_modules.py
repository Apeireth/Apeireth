#!/usr/bin/env python3
"""模块级「未接线」审计 —— 找**整块**能力没接电的模块。

## 为什么符号级不够

`scripts/audit_unwired.py` 查的是**单个符号**有没有生产调用者。这能抓到
「一个函数没人调」，但抓不到「**整个模块**都没接电」—— 后者的每个符号
都是孤儿，逐条看像是零散小事，加起来却是一整块能力缺失。

实测例子（2026-10-24 第四轮子代理核查时发现的）：

| 模块 | 状态 | 缺的能力 |
|---|---|---|
| `foundation/orchestration/job_board.rs` | 全模块仅集成测试构造 | 并行工具派发、长任务过程输出留存 |
| `engine/runtime` 的 `CallScheduler` | `runtime.rs:719` 恒 `None` | 并发安全白名单的生效路径 |
| `engine/memory/hallways.rs` | 整模块零调用 | 记忆「走廊」结构 |
| `engine/memory/cross_diary.rs` | 整模块零调用 | 跨日记上下文 |

这四条在符号级基线里**看不出来**（它们的 pub API 有的是被别的模块 re-export 的，
有的是 `pub use` 形态，符号级判据看不见「整块都没接」这件事）。

## 判据

一个**模块**（一个 `.rs` 文件，或一个目录下的全部 `.rs`）算「整块未接线」，
当且仅当：

1. 它声明了**至少 1 个** pub API（`pub fn` / `pub struct` / `pub trait` / `pub enum`，
   含 `pub(crate)`）—— 排除纯内部实现文件；
2. 这些 pub API **全部**在生产代码里零引用（复用 `audit_unwired.py` 的判据 v3）；
3. 它**没有**被 `pub use` 重导出到别的 crate（重导出 = 有外部消费者，不算孤立）。

## 用法

```
python3 scripts/audit_unwired_modules.py            # 列出整块未接线的模块
python3 scripts/audit_unwired_modules.py --json     # 机器可读
```

## 为什么不做成「基线棘轮」

模块级未接线**不是缺陷** —— 一半是「默认关的功能族」（生物机制、IM 通道），
一半是「还没立项的预留」。把它们做成棘轮会逼着人给每个模块编理由，
反而制造台账噪音。所以这个工具**只报告，不拦**；
要拦的话用 `scripts/unwired-baseline.txt` 的分组（G7 = owner 裁定不接）。
"""

from __future__ import annotations

import argparse
import json
import pathlib
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))

from unwired_baseline_common import ROOT, SCAN_ROOTS, usage_counts  # noqa: E402

import audit_unwired as au  # noqa: E402


def collect() -> dict[str, dict]:
    """扫一遍源码，返回 {文件: {符号名: 是否生产零引用}}。"""
    code_text: dict[str, str] = {}
    declarations: list[tuple[str, str, str]] = []

    for root in SCAN_ROOTS:
        base = ROOT / root
        if not base.exists():
            continue
        for path in sorted(base.rglob("*.rs")):
            if "target" in path.parts:
                continue
            try:
                text = path.read_text(encoding="utf-8")
            except (OSError, UnicodeDecodeError):
                continue
            rel = str(path.relative_to(ROOT))
            if au.is_test_file(path):
                continue
            code, _ = au.split_code_test(text)
            code_text[rel] = code
            for line in code.splitlines():
                match = au.DECL.match(line)
                if match:
                    declarations.append((match.group(1), match.group(2), rel))

    # 生产代码里出现过的 token（**复用判据 v3 的同一份实现**）
    counts = usage_counts(code_text.values())

    per_file: dict[str, dict] = {}
    for _kind, name, rel in declarations:
        per_file.setdefault(rel, {})[name] = counts.get(name, 0)

    # **模块内部自调用不算「有人用」** ——
    # 否则 `cross_diary.rs` 里 `links_for_fact` 被同文件的 `render_cross_context`
    # 调用，整个模块看起来「有引用」，实际整块仍是孤立的。
    # 所以每个符号按「**除本文件外**」的引用数判定。
    outside: dict[str, dict] = {}
    for rel, symbols in per_file.items():
        outside[rel] = {}
        for name, count in symbols.items():
            own = usage_counts([code_text.get(rel, "")]).get(name, 0)
            outside[rel][name] = count - own
    return outside


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--json", action="store_true", help="机器可读输出")
    args = parser.parse_args()

    per_file = collect()
    report = []
    for rel, symbols in sorted(per_file.items()):
        if not symbols:
            continue
        # `symbols[name]` 已经是「**模块外**的引用次数」（定义行与模块内自调用
        # 都已扣除），所以 0 才是「没人从外面用」。
        unwired = {name: count for name, count in symbols.items() if count <= 0}
        if not unwired or len(unwired) != len(symbols):
            continue
        # 被 `pub use` 重导出的模块不算孤立（外部有消费者）
        if _reexported(rel):
            continue
        report.append({"file": rel, "symbols": sorted(unwired)})

    if args.json:
        print(json.dumps(report, ensure_ascii=False, indent=1))
        return 0

    if not report:
        print("没有整块未接线的模块")
        return 0
    print(f"整块未接线的模块：{len(report)} 个\n")
    for item in report:
        print(f"  {item['file']}  ({len(item['symbols'])} 个 pub API 全零引用)")
        for name in item["symbols"]:
            print(f"      {name}")
    print(
        "\n注：这些不是缺陷 —— 一半是「默认关的功能族」，一半是「还没立项的预留」。\n"
        "    要拦的话用 `scripts/unwired-baseline.txt` 的分组（G7 = owner 裁定不接）。"
    )
    return 0


def _reexported(rel: str) -> bool:
    """该文件的 pub API 是否被 `pub use` 重导出到别处。"""
    import re

    pattern = re.compile(r"pub\s+use\s+.*" + re.escape(rel.split("/")[-1].replace(".rs", "")))
    for root in SCAN_ROOTS:
        base = ROOT / root
        if not base.exists():
            continue
        for path in base.rglob("*.rs"):
            try:
                text = path.read_text(encoding="utf-8")
            except (OSError, UnicodeDecodeError):
                continue
            if str(path.relative_to(ROOT)) == rel:
                continue
            if pattern.search(text):
                return True
    return False


if __name__ == "__main__":
    raise SystemExit(main())
