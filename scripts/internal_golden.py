#!/usr/bin/env python3
"""**内部卷**（B0-b）的记录抽取与考卷校验。

# 设计稿要什么（§8 B0-b）

> 内部卷：从本仓日记/记忆挖 2000 条候选 → 精标 300
> 验收：**卷子定稿 + 基线数字**

# 本脚本负责「可复现的两头」

1. **记录抽取**（`--records`）：把仓库自己的文档切成**章节级记录**，给**确定性 id**
   ⇒ ⇒ id = `sha256(相对路径 + '#' + 标题)[:12]`
   ⇒ ⇒ 同一份文档 ⇒ 同一批 id（**换机器/换时间都不变**）；
2. **考卷校验**（`--check`）：精标文件里每条都必须
   * `expect_ids` 出现在当前记录集里（**防止标了不存在的记录**）；
   * `expect_fragments` 是**对应记录正文的真子串**（**防止永远 ground 不上的片段**）；
   * `query` 非空、`id` 全局唯一。
3. **统计**（`--stats`）：各类标签/长度分布 ⇒ 看卷子是否失衡。

# 为什么不用日记

本机 `/home/bai/.apeireth/diary/` 只有 **1 篇**（1168 字节）⇒ 不够两千条。
⇒ 改用**仓库自己的工程文档**（`docs/` 活跃文档）—— 这正是设计稿说的
「**中文 + 长期工程记忆 + 出处要求**」场景，而且**出处天然可指**（文件 + 章节）。

# 用法

    python3 scripts/internal_golden.py --records > /tmp/records.jsonl
    python3 scripts/internal_golden.py --check
    python3 scripts/internal_golden.py --stats
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DOCS = ROOT / "docs"

#: 精标文件（**自有资产**，入库）。
GOLDEN = DOCS / "03-reference" / "internal-golden-v1.jsonl"

#: 抽取范围：只取**活跃**文档（历史卷宗不参与）。
SKIP_PARTS = {"archive"}


def record_id(rel: str, heading: str) -> str:
    """**确定性** id：路径 + 标题的 sha256 前 12 位。

    ⇒ 同文档同标题 ⇒ 永远同 id（换机器、换时间都不变）⇒ 考卷可百年后复算。
    """
    digest = hashlib.sha256(f"{rel}#{heading}".encode()).hexdigest()
    return f"doc:{digest[:12]}"


def sections(path: Path) -> list[tuple[str, str]]:
    """把 Markdown 切成 `(标题, 正文)`；正文含标题行本身（便于检索命中标题词）。"""
    try:
        text = path.read_text(encoding="utf-8")
    except (OSError, UnicodeDecodeError):
        return []
    out: list[tuple[str, str]] = []
    heading = "(前言)"
    buffer: list[str] = []
    for line in text.splitlines():
        if re.match(r"^#{1,4}\s", line):
            if buffer:
                out.append((heading, "\n".join(buffer).strip()))
            heading = line.strip()
            buffer = [line]
        else:
            buffer.append(line)
    if buffer:
        out.append((heading, "\n".join(buffer).strip()))
    return [(name, body) for name, body in out if len(body) >= 80]


def all_records(min_chars: int = 80) -> dict[str, dict[str, str]]:
    """全部活跃文档的章节级记录：`id → {path, heading, text}`。"""
    records: dict[str, dict[str, str]] = {}
    for path in sorted(DOCS.rglob("*.md")):
        rel = str(path.relative_to(ROOT))
        if SKIP_PARTS & set(path.relative_to(DOCS).parts):
            continue
        for heading, body in sections(path):
            if len(body) < min_chars:
                continue
            rid = record_id(rel, heading)
            records[rid] = {"path": rel, "heading": heading, "text": body}
        # 代码/脚本里的「正文」不进记录（避免把源码当记忆）。
    return records


def load_golden() -> list[dict]:
    if not GOLDEN.exists():
        return []
    cases: list[dict] = []
    for line in GOLDEN.read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        try:
            cases.append(json.loads(line))
        except json.JSONDecodeError as error:
            print(f"FAIL: 不是合法 JSON 行：{line[:80]}…（{error}）", file=sys.stderr)
            raise SystemExit(1)
    return cases


def cmd_records(args: argparse.Namespace) -> int:
    records = all_records(args.min_chars)
    for rid, meta in sorted(records.items()):
        print(
            json.dumps(
                {"id": rid, "path": meta["path"], "heading": meta["heading"], "text": meta["text"]},
                ensure_ascii=False,
            )
        )
    print(f"# 共 {len(records)} 条记录", file=sys.stderr)
    return 0


def cmd_check(args: argparse.Namespace) -> int:
    records = all_records(args.min_chars)
    cases = load_golden()
    if not cases:
        print(f"FAIL: 考卷不存在或为空（{GOLDEN.relative_to(ROOT)}）", file=sys.stderr)
        return 1

    problems: list[str] = []
    seen: set[str] = set()
    for index, case in enumerate(cases, start=1):
        cid = case.get("id", f"<第{index}条无 id>")
        if cid in seen:
            problems.append(f"{cid}: **id 重复**")
        seen.add(cid)
        query = (case.get("query") or "").strip()
        if not query:
            problems.append(f"{cid}: `query` 为空")
        expect_ids = case.get("expect_ids") or []
        fragments = case.get("expect_fragments") or []
        if not expect_ids:
            problems.append(f"{cid}: `expect_ids` 为空（无标准答案 ⇒ 不可评）")
        if len(expect_ids) != len(fragments):
            problems.append(
                f"{cid}: `expect_ids`({len(expect_ids)}) 与 `expect_fragments`({len(fragments)}) 数量不等"
            )
        for rid in expect_ids:
            if rid not in records:
                problems.append(f"{cid}: `expect_ids` 里的 `{rid}` **不在当前记录集**（文档改过/标题变了）")
        for rid, fragment in zip(expect_ids, fragments):
            if rid not in records:
                continue
            text = records[rid]["text"]
            if fragment and fragment not in text:
                problems.append(
                    f"{cid}: 片段 **不是** `{rid}` 正文的子串 ⇒ 永远 ground 不上："
                    f"{fragment[:40]!r}"
                )

    if problems:
        print(f"FAIL: 内部卷校验（{len(problems)} 处）", file=sys.stderr)
        for line in problems:
            print(f"  · {line}", file=sys.stderr)
        print(
            "\n修法：① 记录 id 变了（文档标题改过）⇒ 用 `--records` 重新拿 id 再改考卷；"
            "② 片段必须**逐字**来自记录正文（可截短，但不可改写）。",
            file=sys.stderr,
        )
        return 1
    print(
        f"OK: 内部卷校验通过（{len(cases)} 条题 · {len(records)} 条记录 · "
        f"均指向存在的记录且片段为正文子串）"
    )
    return 0


def cmd_stats(args: argparse.Namespace) -> int:
    records = all_records(args.min_chars)
    cases = load_golden()
    from collections import Counter

    tags: Counter[str] = Counter()
    for case in cases:
        for tag in case.get("tags") or []:
            tags[tag] += 1
    print(f"记录数：{len(records)} · 题数：{len(cases)}")
    print(f"标签分布：{dict(tags)}")
    if cases:
        lens = [len((case.get("query") or "")) for case in cases]
        print(f"问题长度：最小 {min(lens)} · 中位 {sorted(lens)[len(lens) // 2]} · 最大 {max(lens)}")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    group = parser.add_mutually_exclusive_group(required=True)
    group.add_argument("--records", action="store_true", help="输出章节级记录 JSONL")
    group.add_argument("--check", action="store_true", help="校验精标考卷")
    group.add_argument("--stats", action="store_true", help="打印分布")
    parser.add_argument("--min-chars", type=int, default=80, help="章节正文最短字符数（默认 80）")
    args = parser.parse_args()
    if args.records:
        return cmd_records(args)
    if args.check:
        return cmd_check(args)
    return cmd_stats(args)


if __name__ == "__main__":
    raise SystemExit(main())
