#!/usr/bin/env python3
"""文档链接完整性检查 —— 断链=0 是每一批文档治理的验收判据。

用法::

    python3 scripts/docs_links.py --check          # 检查断链，坏链 exit 1
    python3 scripts/docs_links.py --check --anchors # 额外校验 #锚点 是否存在
    python3 scripts/docs_links.py --report         # 人类可读报告，不改退出码

设计要点：

* **重定向桩受支持**：若链接指向的路径已登记在 ``docs/_redirects.tsv``
  （旧路径 -> 新路径），且新路径存在，则视为**可达**，不算断链。
  这是"原件进 archive、原路径留桩"策略的配套。
* **跳过范围**：``target/``、``legacy/``、``.git/``、``docs/archive/`` 的**正文**
  不参与扫描（archive 按纪律只引用不改）。但 archive 内文件**指向仓内其它目录**
  的链接仍会被检查，只是不检查 archive 内部的互相引用。
* 只检查**相对链接**与**根相对链接**；``http(s)``/``mailto:``/纯锚点一律跳过。
"""

from __future__ import annotations

import argparse
import csv
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

# 扫描时跳过的目录（相对 ROOT）
SKIP_DIRS = {".git", "target", "legacy", "node_modules", "dist", ".venv"}

# GitHub 仓库 UI 的伪路径（README 里的 `../../issues` 之类，渲染为网页而非仓内文件）
GITHUB_UI_PATHS = {
    "issues", "releases", "stargazers", "watchers", "network", "pulse",
    "graphs", "blame", "commits", "compare", "tags", "trees", "actions",
    "projects", "wiki", "security", "insights", "pulls", "discussions",
}

# 这些目录的**内部**互相引用不检查（archive 只引用不改）
NO_INTERNAL_LINK_CHECK = {Path("docs/archive")}

REDIRECTS = Path("docs/_redirects.tsv")
KNOWN_MISSING = Path("docs/_known-missing.tsv")

# 行内链接：[text](target) —— 允许 <> 包裹与嵌套一层括号
LINK_RE = re.compile(r"\[(?:[^\]\\]|\\.)*\]\(\s*<?([^)\s>]+)>?(?:\s+[\"'][^)]*[\"'])?\s*\)")
# 裸路径引用（代码块外），如 `docs/04-internal/foo.md`
BARE_RE = re.compile(r"(?<![\w/`])(docs/[\w./-]+\.md|reports/[\w./-]+\.[A-Za-z0-9]+)(?![\w/])")


def slugify_heading(text: str) -> str:
    """GitHub 风格锚点 slug：转小写、去标点、空格转连字符。"""
    text = text.strip().lower()
    text = re.sub(r"[^\w\- ]", "", text)
    return re.sub(r"\s+", "-", text)


def load_known_missing() -> dict[str, str]:
    """已知缺失清单：按仓内政策**故意保留**的历史引用（不计断链）。"""
    path = ROOT / KNOWN_MISSING
    if not path.exists():
        return {}
    out: dict[str, str] = {}
    with path.open(encoding="utf-8") as fh:
        for line in fh:
            line = line.rstrip("\n")
            if not line or line.startswith("#"):
                continue
            parts = line.split("\t")
            if len(parts) >= 2:
                out[parts[0].strip()] = parts[1].strip()
    return out


def load_redirects() -> dict[str, str]:
    path = ROOT / REDIRECTS
    if not path.exists():
        return {}
    out: dict[str, str] = {}
    with path.open(encoding="utf-8") as fh:
        reader = csv.reader(fh, delimiter="\t")
        for row in reader:
            if not row or row[0].startswith("#"):
                continue
            if len(row) >= 2:
                out[row[0].strip()] = row[1].strip()
    return out


def iter_md_files() -> list[Path]:
    out: list[Path] = []
    for p in ROOT.rglob("*.md"):
        rel = p.relative_to(ROOT)
        if any(part in SKIP_DIRS for part in rel.parts):
            continue
        out.append(p)
    return sorted(out)


def anchors_of(md_path: Path) -> set[str]:
    slugs: set[str] = set()
    try:
        text = md_path.read_text(encoding="utf-8", errors="replace")
    except OSError:
        return slugs
    for line in text.splitlines():
        m = re.match(r"^\s{0,3}#{1,6}\s+(.*?)\s*#*\s*$", line)
        if m:
            slugs.add(slugify_heading(m.group(1)))
    return slugs


def in_archive(rel: Path) -> bool:
    return any(part == "archive" for part in rel.parts)


# 断裂的 markdown 链接语法：`[label]target` —— 缺 `(` 与 `)`。
# 这类损坏**根本不是链接**，普通链接检查看不见它（2026-10-23 因此漏检 48 处 / 26 个文件）。
#
# 排除规则：目标**不得以 `-` 开头**。真链接目标不会以 `-` 起；而模板占位符
# （如 `r12-[项名]-implementation-2026-08-28.md`）恰以 `-` 起，据此区分，避免误报。
BROKEN_SYNTAX_RE = re.compile(
    r"\[([^\]\n]{1,140}?)\]"                        # 标签
    r"([\w./][\w./\-]*\.(?:md|rs|ts|tsx|py|yml|yaml|toml|ps1|sh))"  # 紧跟其后的路径
    r"(?![\w/])"                                    # 后面不能再接路径字符
)


def check(check_anchors: bool, scope: str = "live") -> tuple[list[str], list[str]]:
    """scope='live'：只检查活文档内部的链接（archive 内的历史链接不算断链）。

    理由：``docs/archive/`` 是 L5 卷宗，按仓内纪律**只引用不改写**。
    它的链接指向的是**当时**的路径（例如 v1.0 时期的 ``ROADMAP.md``、``NOTICE``），
    这些目标今天不存在是**历史事实**，不是断链缺陷；去"修"它们等于改写历史。
    但**从活文档指向 archive 的链接必须可达**——那条在下面照常检查。
    """
    redirects = load_redirects()
    known = load_known_missing()
    broken: list[str] = []
    known_hits: list[str] = []
    anchor_warn: list[str] = []

    md_files = iter_md_files()

    for md in md_files:
        rel_md = md.relative_to(ROOT)
        if scope == "live" and in_archive(rel_md):
            continue
        try:
            text = md.read_text(encoding="utf-8", errors="replace")
        except OSError:
            continue
        # 去掉代码块，避免把示例里的路径当真链接
        text = re.sub(r"```.*?```", "", text, flags=re.S)
        text = re.sub(r"`[^`\n]*`", "", text)

        # 断裂的链接语法（`[label]target`）——必须在剥离反引号**之后**查：
        # 这类损坏恰恰常发生在原本被反引号包裹的路径上。
        for bm in BROKEN_SYNTAX_RE.finditer(text):
            broken.append(
                f"{rel_md} -> [{bm.group(1)}]{bm.group(2)}  (链接语法断裂：缺 `(` 与 `)`)"
            )

        targets: list[str] = []
        for m in LINK_RE.finditer(text):
            targets.append(m.group(1))
        for m in BARE_RE.finditer(text):
            targets.append(m.group(1))

        for raw in targets:
            if not raw or raw.startswith(("http://", "https://", "mailto:", "tel:", "#", "data:")):
                continue
            # GitHub UI 伪路径：README 里的 `../../issues`、`../../releases` 等，
            # 在 GitHub 上渲染为仓库的 issues/releases 页，不是仓内文件路径。
            if raw.split("/")[-1] in GITHUB_UI_PATHS:
                continue
            raw = raw.strip()
            path_part, _, anchor = raw.partition("#")
            if not path_part:
                continue  # 纯锚点，跳过（需要页内上下文）
            path_part = path_part.replace("%20", " ")

            if path_part.startswith("/"):
                resolved = ROOT / path_part.lstrip("/")
            else:
                resolved = (md.parent / path_part).resolve()

            # 归一到仓库内
            try:
                resolved_rel = resolved.relative_to(ROOT)
            except ValueError:
                continue  # 指向仓外，忽略

            exists = resolved.exists()

            if not exists and not path_part.startswith("/"):
                # 回退一：**仓根相对**约定。仓内大量文档写 `docs/xxx` / `reports/xxx`
                # 表示"从仓库根起"，而按"相对本文件目录"解析会落到不存在的子路径。
                alt = (ROOT / path_part).resolve()
                if alt.exists() and str(alt).startswith(str(ROOT)):
                    resolved = alt
                    try:
                        resolved_rel = resolved.relative_to(ROOT)
                    except ValueError:
                        continue
                    exists = True

            if not exists:
                # 查重定向表：旧路径（仓库相对）-> 新路径
                new_rel = None
                for key in redirects:
                    k = key.replace("\\", "/")
                    if k == resolved_rel.as_posix():
                        new_rel = redirects[key]
                        break
                if new_rel:
                    new_abs = ROOT / new_rel
                    if new_abs.exists():
                        if anchor and check_anchors and new_abs.suffix == ".md":
                            if anchor not in anchors_of(new_abs):
                                anchor_warn.append(
                                    f"{rel_md} -> {raw}  (经重定向后锚点未找到: {new_rel}#{anchor})"
                                )
                        continue
                    broken.append(f"{rel_md} -> {raw}  (重定向目标不存在: {new_rel})")
                    continue
                # 已知缺失（按仓内政策故意保留的历史引用）——不计断链，但登记
                if raw in known or path_part in known:
                    known_hits.append(f"{rel_md} -> {raw}  (已知缺失: {known.get(raw, known.get(path_part, ''))})")
                    continue
                # ⚠️ 落穿分支（2026-10-23 修）：未命中重定向表的断链必须报出，
                #    否则门禁对"真断链"完全失明（曾漏检 403 处）。
                broken.append(f"{rel_md} -> {raw}  (目标不存在)")
                continue

            # 目标存在，检查锚点
            if anchor and check_anchors and resolved.suffix == ".md":
                if anchor not in anchors_of(resolved):
                    anchor_warn.append(f"{rel_md} -> {raw}  (锚点未找到)")

    return broken, anchor_warn, known_hits


def main() -> int:
    ap = argparse.ArgumentParser(description="文档链接完整性检查")
    ap.add_argument("--check", action="store_true", help="有断链则 exit 1")
    ap.add_argument("--report", action="store_true", help="打印报告")
    ap.add_argument("--anchors", action="store_true", help="额外校验锚点")
    ap.add_argument("--scope", choices=["live", "all"], default="live",
                    help="live=只看活文档（archive 内部历史链接不算断链）；all=全查")
    args = ap.parse_args()

    broken, anchor_warn, known_hits = check(args.anchors, args.scope)

    print(f"[docs_links] 扫描 {len(iter_md_files())} 个 markdown 文件（作用域 {args.scope}）")
    print(f"[docs_links] 断链: {len(broken)}")
    if known_hits:
        print(f"[docs_links] 已知缺失（政策保留，不计断链）: {len(known_hits)}")
    if anchor_warn:
        print(f"[docs_links] 锚点警告: {len(anchor_warn)}")

    for line in broken[:80]:
        print(f"  BROKEN  {line}")
    if len(broken) > 80:
        print(f"  ... 其余 {len(broken) - 80} 条省略")
    for line in anchor_warn[:40]:
        print(f"  ANCHOR  {line}")
    if len(anchor_warn) > 40:
        print(f"  ... 其余 {len(anchor_warn) - 40} 条省略")

    if broken:
        return 1 if args.check else 0
    return 0


if __name__ == "__main__":
    sys.exit(main())
