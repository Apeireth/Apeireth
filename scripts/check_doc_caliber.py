#!/usr/bin/env python3
"""文档四级口径守门 (W8③, per docs/04-internal/engineering-review-handoff-2026-10-06.md §5 W8).

规则: **活跃文档**中若含触发词 (已实现 / 已接线), 同段必须出现四级口径之一
(IMPLEMENTED / PRODUCTION WIRED / DEFAULT ENABLED / HARDWARE VALIDATED) 或 "四级口径"
字样。四级口径定义见 docs/01-architecture/system-capabilities.md 头部:
    IMPLEMENTED ≠ PRODUCTION WIRED ≠ DEFAULT ENABLED ≠ HARDWARE VALIDATED
目的: 把 0 装纪律 (O-5 锚) 变成机器可检查信号 —— 文档声称"已实现/已接线"时必须
同时声明它落在哪一级, 防止"库级已实现"被读者误读为"生产可用/真机验证"。

段落粒度: 散文段 (空行分隔) 整段为准; **列表项 / 表格行各自按行算段** (结构化块里
一条主张不该连坐整张表)。围栏代码块内不查 (注释/示例不算文档主张)。

跳过 (历史证据, 按仓内"dated 文档正文不改"约定):
- 目录: docs/archive/ docs/history/ legacy/ reports/ research/ _research_mem/
  previews/ artifacts/ target/ node_modules/ .git/ .wt/ .github/ (流程模板是提问,
  不是能力主张)
- 文件名: 含日期段 (2026-08-28 等) / round-N / rNN- 前缀 / *.backup*
- 文件头 12 行内带历史横幅 (历史记录 / 历史快照 / 历史证据 / 对账批标注 / 现状 (20..
  / 写作时 / 快照保留)

用法:
  python scripts/check_doc_caliber.py                       # 扫全部活跃 .md
  python scripts/check_doc_caliber.py --changed BASE HEAD   # 只扫两个提交间改动的 .md
  python scripts/check_doc_caliber.py file.md [dir ...]     # 只扫指定路径
  python scripts/check_doc_caliber.py --update-baseline     # 重写存量欠账基线

存量基线: scripts/doc-caliber-baseline.txt 登记 "rel:line" 形式的已知欠账;
命中基线的违规降级为 [baseline] 提示 (不红), **新增违规才 exit 1**。动到某条欠账的
段落时请补口径标注并顺手从基线删除该行 (ratchet: 基线只减不增)。

退出码: 0 = 无新增违规; 1 = 有新增违规 (逐条列出 file:段首行)。
"""

from __future__ import annotations

import re
import subprocess
import sys
from pathlib import Path

sys.stdout.reconfigure(encoding="utf-8", errors="replace")
sys.stderr.reconfigure(encoding="utf-8", errors="replace")

REPO_ROOT = Path(__file__).resolve().parent.parent

TRIGGERS = ("已实现", "已接线")
CALIBER_MARKERS = (
    "IMPLEMENTED",
    "PRODUCTION WIRED",
    "DEFAULT ENABLED",
    "HARDWARE VALIDATED",
    "四级口径",
)

SKIP_DIR_NAMES = {
    ".git",
    ".wt",
    ".github",
    "node_modules",
    "target",
    "legacy",
    "reports",
    "research",
    "_research_mem",
    "previews",
    "artifacts",
}
SKIP_DIR_PREFIXES = (
    "docs/archive/",
    "docs/history/",
    "docs/_archive/",
)
SKIP_NAME_RES = (
    re.compile(r"20\d{2}-\d{2}-\d{2}"),
    re.compile(r"^round-\d+", re.I),
    re.compile(r"^r\d+-", re.I),
    re.compile(r"\.backup", re.I),
)
HISTORY_BANNER_RES = re.compile(
    r"历史记录|历史快照|历史证据|快照保留|对账批标注|现状 \(20|写作时|LEGACY"
)
FENCE_RES = re.compile(r"^\s*(```|~~~)")
ITEM_RES = re.compile(r"^\s*(?:[-*+]|\d+[.)])\s+\S|^\s*\|")
NEGATION_RES = re.compile(r"假装已(?:实现|接线)")  # "0 假装已实现" 是纪律名, 不是主张


def is_historical(path: Path) -> str | None:
    """返回跳过理由; 不跳过则返回 None."""
    try:
        rel = path.relative_to(REPO_ROOT).as_posix()
    except ValueError:  # 仓库外路径 (自测/外部调用): 按裸文件名单独判定
        rel = path.name
    for prefix in SKIP_DIR_PREFIXES:
        if rel.startswith(prefix):
            return f"历史目录 {prefix}"
    parts = rel.split("/")
    for part in parts[:-1]:
        if part in SKIP_DIR_NAMES:
            return f"历史目录 {part}/"
    name = parts[-1]
    for rx in SKIP_NAME_RES:
        if rx.search(name):
            return "文件名含日期/轮次/backup"
    try:
        head = path.read_text(encoding="utf-8", errors="replace").splitlines()[:12]
    except OSError:
        return "读取失败(按历史处理)"
    if HISTORY_BANNER_RES.search("\n".join(head)):
        return "文件头带历史横幅"
    return None


def strip_fences(lines: list[str]) -> list[str]:
    """去掉围栏代码块内容, 保留行号占位."""
    out, in_fence = [], False
    for ln in lines:
        if FENCE_RES.match(ln):
            in_fence = not in_fence
            out.append("")
            continue
        out.append("" if in_fence else ln)
    return out


def _violates(seg: list[tuple[int, str]]) -> bool:
    text = NEGATION_RES.sub("", "\n".join(t for _, t in seg))
    return any(trig in text for trig in TRIGGERS) and not any(
        mk in text for mk in CALIBER_MARKERS
    )


def check_file(path: Path) -> list[tuple[int, str]]:
    """返回违规列表 (段首行号, 段首 80 字)."""
    lines = strip_fences(
        path.read_text(encoding="utf-8", errors="replace").splitlines()
    )
    violations: list[tuple[int, str]] = []
    prose: list[tuple[int, str]] = []

    def flush_prose() -> None:
        if prose and _violates(prose):
            violations.append((prose[0][0], prose[0][1].strip()[:80]))
        prose.clear()

    for i, ln in enumerate(lines, 1):
        if not ln.strip():
            flush_prose()
        elif ITEM_RES.match(ln):
            flush_prose()
            if _violates([(i, ln)]):
                violations.append((i, ln.strip()[:80]))
        else:
            prose.append((i, ln))
    flush_prose()
    return violations


def iter_active_md(targets: list[Path]) -> list[tuple[Path, str | None]]:
    found: list[tuple[Path, str | None]] = []
    for t in targets:
        if t.is_dir():
            for p in sorted(t.rglob("*.md")):
                found.append((p, is_historical(p)))
        elif t.suffix == ".md":
            found.append((t, is_historical(t)))
    return found


def changed_md(base: str, head: str) -> list[Path]:
    out = subprocess.run(
        ["git", "diff", "--name-only", base, head, "--", "*.md"],
        cwd=REPO_ROOT, capture_output=True, text=True, check=False,
    ).stdout.split()
    return [REPO_ROOT / rel for rel in out if (REPO_ROOT / rel).exists()]


BASELINE_PATH = REPO_ROOT / "scripts" / "doc-caliber-baseline.txt"


def load_baseline() -> set[str]:
    if not BASELINE_PATH.exists():
        return set()
    out = set()
    for raw in BASELINE_PATH.read_text(encoding="utf-8").splitlines():
        raw = raw.strip()
        if raw and not raw.startswith("#"):
            out.add(raw)
    return out


def main(argv: list[str]) -> int:
    args = argv[1:]
    update_baseline = False
    if args and args[0] == "--update-baseline":
        update_baseline = True
        args = args[1:]
    if len(args) >= 3 and args[0] == "--changed":
        targets = changed_md(args[1], args[2])
    elif args:
        targets = [(REPO_ROOT / a).resolve() for a in args]
    else:
        targets = [REPO_ROOT]

    scanned, skipped, violations = 0, 0, []
    for path, reason in iter_active_md(targets):
        if reason is not None:
            skipped += 1
            continue
        scanned += 1
        for line_no, preview in check_file(path):
            try:
                rel = path.relative_to(REPO_ROOT).as_posix()
            except ValueError:
                rel = str(path)
            violations.append((rel, line_no, preview))

    if update_baseline:
        BASELINE_PATH.write_text(
            "# 文档四级口径存量欠账基线 (check_doc_caliber.py --update-baseline 生成)\n"
            "# 格式: rel:line。命中基线的违规不红, 新增违规才红 (ratchet: 基线只减不增)。\n"
            + "".join(f"{rel}:{line_no}\n" for rel, line_no, _ in sorted(violations)),
            encoding="utf-8",
        )
        print(f"[caliber] 基线已写入 {BASELINE_PATH.relative_to(REPO_ROOT)} ({len(violations)} 条)")
        return 0

    baseline = load_baseline()
    fresh = [v for v in violations if f"{v[0]}:{v[1]}" not in baseline]
    aged = len(violations) - len(fresh)

    for rel, line_no, preview in fresh:
        print(f"[caliber] {rel}:{line_no}: 含'已实现/已接线'但无四级口径标注: {preview}")
    print(
        f"[caliber] 扫描 {scanned} 个活跃文档 (跳过 {skipped} 个历史文档), "
        f"新增违规 {len(fresh)} 处 (基线内 {aged} 处)"
    )
    return 1 if fresh else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
