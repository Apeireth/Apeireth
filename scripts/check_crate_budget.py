#!/usr/bin/env python3
"""**crate 体量预算守卫**（`scripts/check_crate_budget.py`）—— 堵住「搬家式漂移」。

# 漏洞（2026-10-28 审计认定 H4）

`arch_guard` 的尺寸棘轮量的是**单文件行数**
⇒ 于是「把代码拆成两个文件」就能满足棘轮，而**复杂度只是横向移动**。
⇒ ⇒ 实测：2026-10-27 我把 `hook.rs` 从 1221 行拆到 1093 行（**完全合规**，也是合理重构），
但「这个 crate 有没有变胖」**没有任何守卫**。

# 本脚本判什么

**每个 crate 的 `src/` 总量**（行数 + 文件数）进基线 ⇒ **只减不增**。

* `--gen`：重写基线（**只应在"真的减少"之后跑**）；
* `--check`：任一 crate 的 LOC 或文件数**超过**基线 ⇒ 失败。

# ⚠️ 与 `arch_guard` 的分工（别重复）

* `arch_guard` 管**单文件**尺寸 + 依赖边 + 分层；
* 本脚本管**每 crate 的总量** ⇒ 两者合起来，"拆文件"就不再能规避尺寸约束。

# 用法

    python3 scripts/check_crate_budget.py --gen
    python3 scripts/check_crate_budget.py --check
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
BASELINE = ROOT / "scripts" / "crate-budget-baseline.json"


def crate_stats() -> dict[str, dict[str, int]]:
    """每 crate 的 `src/` 总量（行数 / 文件数）。"""
    out: dict[str, dict[str, int]] = {}
    for manifest in sorted((ROOT / "crates").rglob("Cargo.toml")):
        crate_dir = manifest.parent
        src = crate_dir / "src"
        if not src.is_dir():
            continue
        rel = str(crate_dir.relative_to(ROOT))
        loc = 0
        files = 0
        for path in src.rglob("*.rs"):
            if "target" in path.parts:
                continue
            files += 1
            try:
                loc += sum(1 for _ in path.open(encoding="utf-8", errors="ignore"))
            except OSError:
                continue
        if files:
            out[rel] = {"loc": loc, "files": files}
    return out


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    group = parser.add_mutually_exclusive_group(required=True)
    group.add_argument("--check", action="store_true", help="门禁：任一 crate 超基线即失败")
    group.add_argument("--gen", action="store_true", help="重写基线（只应在减少后跑）")
    args = parser.parse_args()

    stats = crate_stats()
    if not stats:
        print("FAIL: 一个 crate 都没扫到 ⇒ 路径变了？", file=sys.stderr)
        return 1

    if args.gen:
        BASELINE.write_text(
            json.dumps(
                {
                    "version": "crate-budget/v1",
                    "note": "每个 crate 的 src/ 总量；只减不增（--gen 只应在真的减少后跑）",
                    "crates": stats,
                },
                ensure_ascii=False,
                indent=2,
                sort_keys=True,
            )
            + "\n",
            encoding="utf-8",
        )
        total = sum(item["loc"] for item in stats.values())
        print(f"✓ 已写 {BASELINE.name}：{len(stats)} 个 crate · 合计 {total} 行")
        return 0

    if not BASELINE.exists():
        print(f"FAIL: 基线不存在 ⇒ 跑 --gen", file=sys.stderr)
        return 1
    baseline = json.loads(BASELINE.read_text(encoding="utf-8")).get("crates", {})

    grown: list[str] = []
    new_crates: list[str] = []
    shrunk: list[str] = []
    for name, stat in stats.items():
        old = baseline.get(name)
        if old is None:
            new_crates.append(f"{name}: 新增 crate（{stat['loc']} 行 / {stat['files']} 文件）")
            continue
        if stat["loc"] > old["loc"] or stat["files"] > old["files"]:
            grown.append(
                f"{name}: {old['loc']}→{stat['loc']} 行 · {old['files']}→{stat['files']} 文件"
                f"（**超基线**）"
            )
        elif stat["loc"] < old["loc"] or stat["files"] < old["files"]:
            shrunk.append(f"{name}: {old['loc']}→{stat['loc']} 行")

    problems = grown + new_crates
    if problems:
        print(f"FAIL: crate 体量预算（{len(problems)} 处）", file=sys.stderr)
        for line in problems:
            print(f"  · {line}", file=sys.stderr)
        print(
            "\n修法：① 把增长**说清并接受** ⇒ 跑 `--gen` 重写基线（**要在提交信息里说明为什么**）；"
            "② 或者把代码减回去。\n"
            "⚠️ 注意：本守卫**故意**比 `arch_guard` 粗（按 crate 总量）"
            "⇒ 它防的是「拆文件规避尺寸棘轮」，不是精确定位。",
            file=sys.stderr,
        )
        return 1

    if shrunk:
        print(
            f"⚠️ 有 {len(shrunk)} 个 crate **变小了** ⇒ 可跑 `--gen` 缩表（棘轮只减不增）："
        )
        for line in shrunk[:10]:
            print(f"  · {line}")
    total = sum(item["loc"] for item in stats.values())
    print(f"OK: crate 体量预算通过（{len(stats)} 个 crate · 合计 {total} 行）")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
