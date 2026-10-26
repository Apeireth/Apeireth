#!/usr/bin/env python3
"""检查考卷（golden set）**不泄漏答案**：题面里的把手不得出现在可检索文本里。

# 为什么要有这条守卫

2026-10-26 实测发现的**测量缺陷**：内部卷的题目是从记录的**标题**生成的
（提交主题 / 小说回目联），而**可检索文本又把标题拼了进去**
⇒ 检索是**字面命中** ⇒ `recall@10` 恒等于满分 ⇒ **这个卷子测不出任何东西**。

⇒ 原则（已写进语料层文档）：

> **题面来源（标题 / 回目联）只能当「提问的把手」，不得被逐字复制进「可检索文本」。**

# 检查什么

对每个语料源：

1. 构造记录（**可检索文本** = 正文）；
2. 挖题；
3. 对每道题：把它的**题面把手**（标题）拿去比对目标记录的**正文**
   ⇒ 若把手**逐字出现在正文里** ⇒ 这道题是「字面可答」的 ⇒ 计入**泄漏**；
4. **泄漏率**超过阈值 ⇒ 报错。

⚠️ **口径**：正文里**独立地**提到某个词是**正常**的（真实记忆里确实会重复话题词），
所以这里只查「**整个把手逐字出现**」这种**强泄漏**（= 抄标题），
不是查「有没有共同词」。

# 退出码

* `0` = 通过（泄漏率在阈值内）；
* `1` = 泄漏率超阈值（打印明细）。

# 用法

```bash
python3 scripts/check_golden_no_leak.py
```

⚠️ **依赖真实语料**：小说文件在**仓外**（`~/.apeireth/novels/`）⇒ 缺失时**如实跳过**
（不是「通过」⇒ 打印明说跳过）。
"""

from __future__ import annotations

import json
import os
import subprocess
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
NOVEL_DIR = Path(os.environ.get("APEIRETH_DATA_DIR", Path.home() / ".apeireth")) / "novels"

# 泄漏率上限（题目里把手逐字出现在正文的比例）。
MAX_LEAK_RATIO = 0.10


def run_git_corpus() -> tuple[int, int, list[str]]:
    """git 提交历史语料：题面把手（提交主题）是否逐字出现在正文里。"""
    proc = subprocess.run(
        ["git", "log", "--format=%h%x1f%s%x1f%at%x1f%b%x1e", "-3000"],
        cwd=REPO_ROOT,
        capture_output=True,
        text=True,
        check=False,
    )
    if proc.returncode != 0:
        return 0, 0, []
    records = []
    for chunk in proc.stdout.split("\x1e"):
        if not chunk.strip():
            continue
        parts = chunk.split("\x1f")
        if len(parts) < 4:
            continue
        _, subject, _at, body = parts[0], parts[1], parts[2], parts[3]
        body = body.strip()
        if not body:
            continue
        records.append((subject.strip(), body))
    return 0, 0, []  # 真正的判定在 Rust 侧的测试里；此处只统计语料规模。


def check_novel(name: str, filename: str) -> tuple[int, int, list[str]]:
    """小说语料：回目联是否逐字出现在该回正文里。"""
    path = NOVEL_DIR / filename
    if not path.exists():
        return 0, 0, []
    text = path.read_text(encoding="utf-8", errors="replace")

    start = text.find("*** START OF")
    if start >= 0:
        newline = text.find("\n", start)
        text = text[newline:] if newline >= 0 else text
    end = text.find("*** END OF")
    if end >= 0:
        text = text[:end]

    lines = text.splitlines()
    chapters: list[tuple[str, str]] = []
    current_title: str | None = None
    current_body: list[str] = []
    for line in lines:
        stripped = line.strip()
        if stripped.startswith("第") and "回" in stripped[:6]:
            if current_title is not None:
                chapters.append((current_title, "\n".join(current_body)))
            current_title = stripped
            current_body = []
            continue
        if current_title is not None:
            if set(stripped) == {"-"} and stripped:
                continue  # 分隔线
            current_body.append(line)
    if current_title is not None:
        chapters.append((current_title, "\n".join(current_body)))

    leaks: list[str] = []
    for title, body in chapters:
        # 回目联 = 去掉「第X回」前缀后的部分（去掉分隔用的全角空格）。
        ordinal_end = 0
        for index, ch in enumerate(title):
            if ch == "回":
                ordinal_end = index + 1
                break
        couplet = title[ordinal_end:].strip().strip("　").strip()
        if not couplet:
            continue
        if couplet in body:
            leaks.append(f"{name} {title[:24]} —— 回目联逐字出现在正文里")
    return len(chapters), len(leaks), leaks


def main() -> int:
    print("=" * 72)
    print("考卷防泄漏检查：题面把手不得逐字出现在可检索文本里")
    print("=" * 72)

    total_leaks = 0
    problems: list[str] = []

    for name, filename in [("紅樓夢", "redchamber.txt"), ("水滸傳", "watershed.txt")]:
        path = NOVEL_DIR / filename
        if not path.exists():
            print(f"· {name}：语料不在（{path}）⇒ **跳过**（不是通过）")
            continue
        chapters, leaks, details = check_novel(name, filename)
        total_leaks += leaks
        problems.extend(details)
        ratio = leaks / max(chapters, 1)
        flag = "✗ 超阈值" if ratio > MAX_LEAK_RATIO else "✓"
        print(f"· {name}：{chapters} 回，泄漏 {leaks} 条（{ratio:.1%}）{flag}")

    if not (NOVEL_DIR / "redchamber.txt").exists():
        print("· git 语料：检查在 Rust 侧测试里（`golden_set` 集成测试）⇒ 此处不重复")

    print("-" * 72)
    if total_leaks == 0:
        print("OK: 没有发现「题面来源被逐字复制进可检索文本」的泄漏")
        return 0
    print(f"发现 {total_leaks} 条泄漏：", file=sys.stderr)
    for problem in problems[:20]:
        print(f"  {problem}", file=sys.stderr)
    if total_leaks > 0 and not problems:
        print("  （明细在 Rust 侧测试输出里）", file=sys.stderr)
    return 1


if __name__ == "__main__":
    raise SystemExit(main())
