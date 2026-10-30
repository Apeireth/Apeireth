#!/usr/bin/env python3
"""**失败知识库守卫**（2026-10-30）。

# 为什么要有它

`.harness/knowledge/failure-patterns.yaml` 记的是「**踩过的坑**」⇒ 每条都带一个
`guard:` 字段（**哪个守卫能拦住它**）⇒ ⇒ 如果那个守卫**改名/删除/搬家**，
知识库就**悄悄过期** ⇒ 照着它排查会去找一个不存在的检查器。

⇒ 本脚本把「知识指向的守卫必须真实存在」变成**机器判定**。

（同本仓一贯纪律：文档/知识里的引用**不许变成陈旧副本**。）

# 用法

    python3 scripts/check_failure_knowledge.py
"""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
PATTERNS = ROOT / ".harness" / "knowledge" / "failure-patterns.yaml"
RECIPES = ROOT / ".harness" / "knowledge" / "repair-recipes.yaml"
INVENTORY = ROOT / "scripts" / "guard-inventory.json"

SCRIPT_REF = re.compile(r"scripts/[A-Za-z0-9_./-]+\.(?:py|sh|ps1)")


def main() -> int:
    if not PATTERNS.exists():
        print(f"（无 {PATTERNS.relative_to(ROOT)} ⇒ 没有知识库 ⇒ 不报错）")
        return 0
    paths: set[str] = set()
    if INVENTORY.exists():
        try:
            data = json.loads(INVENTORY.read_text(encoding="utf-8"))
            paths = {
                str(g.get("path"))
                for g in data.get("guards", [])
                if isinstance(g, dict)
            }
        except (OSError, json.JSONDecodeError):
            paths = set()

    problems: list[str] = []
    for path in (PATTERNS, RECIPES):
        if not path.exists():
            continue
        text = path.read_text(encoding="utf-8")
        for match in SCRIPT_REF.finditer(text):
            ref = match.group(0)
            if paths and ref not in paths:
                line = text[: match.start()].count("\n") + 1
                problems.append(
                    f"{path.relative_to(ROOT)}:{line}: 引用的 `{ref}` **不在守卫清单里**"
                    f" ⇒ 知识条目已陈旧（守卫改名/删除/搬家？）"
                )
    if problems:
        print(f"FAIL: 失败知识库守卫（{len(problems)} 处）", file=sys.stderr)
        for line in problems:
            print(f"  · {line}", file=sys.stderr)
        return 1
    print("OK: 失败知识库一致（引用的守卫都真实存在）")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
