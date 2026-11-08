#!/usr/bin/env python3
"""**经验矿守卫**（2026-11-08）—— 让「经验沉淀体系」本身**不许漂成散文**。

# 为什么要有它

本仓的经验沉淀体系（`docs/04-internal/engineering-experience/`）是**提炼层 L3**：
从 L4 在飞与 L5 卷宗里提炼出的可复用经验与教训 ⇒ 它的价值全在「**可复用 + 可复核**」。

但在本守卫出现之前，这个体系**一条守卫都没有** ⇒ 按本仓自己的标准
（`docs/status/workline-registry.md` §三：「**没有守卫的文档 = 散文；散文不能当进度用**」）
它当时只是**散文**：

* 条目写成散段落 ⇒ 读者无法判断哪条是「做法」、哪条是「教训」；
* `★` 出处指向的文件被删/搬家 ⇒ 出处变**陈旧副本**，照着找会扑空；
* 经验矿（人读层）与 `.harness/knowledge/failure-patterns.yaml`（机器层）**各说各话**
  ⇒ 同一批教训有两个互不知情的副本 ⇒ 机器层更新了、人读层没更新 ⇒ 下次排查照旧扑空。

⇒ 本脚本把三件事变成**机器判定**：

1. **条目结构**：每条经验必须是「四段式 + 出处 + 守卫」，不许退化成一段散文；
2. **锚点可达**：每个 `★` 锚点指向的文件（及标题锚点）必须**真实存在**；
3. **机器层互引**：`failure-patterns.yaml` 的每个失败模式 id 必须被经验矿引用，
   经验矿引用的 id 必须存在于 yaml ⇒ 两层**不许各说各话**。

（另加两条轻量项：族内索引覆盖（README 的表不许漏篇）+ 新章必带 `Class:`。）

# 用法

    python3 scripts/check_experience_anchors.py --check      # 门禁（默认）
    python3 scripts/check_experience_anchors.py --report     # 只报告，不判成败

# 退出码

    0 = 结构合规 / 锚点可达 / 两层互引一致
    1 = 有违规（逐条打印差在哪 + 怎么修）
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
FAMILY = ROOT / "docs" / "04-internal" / "engineering-experience"
INDEX = FAMILY / "README.md"
KNOWLEDGE = ROOT / ".harness" / "knowledge" / "failure-patterns.yaml"

#: 矿型 → 该型条目的**必填段**（`【…】`）。
#: ⚠️ 键 = 文件名前缀（`NN-`）；值 = 该型每条条目必须同时出现的段名。
#: 「死法 4」是子条目型（4a/4b/4c 各自成段）⇒ 判定取「本节 + 其 `###` 子节」的并集。
REQUIRED_SEGMENTS: dict[str, tuple[str, ...]] = {
    "01-": ("情境", "做法", "效果"),
    "02-": ("症状", "死因", "药方", "防复发断言"),
    "03-": ("裁定", "被拒案", "理由链"),
    "04-": ("情境", "配方", "验收形状"),
    "05-": ("洞察", "制度化落点", "可复用判据"),
    "06-": ("情境", "做法", "效果", "锚点", "守卫"),
    "07-": ("情境", "做法", "效果", "锚点", "守卫"),
}

#: **导航册**（`00-`）：是工序卡不是矿 ⇒ 不判条目段型、不判分类（存量豁免，见 `check_class`）。
MANUAL_PREFIXES = ("00-",)

#: 条目标题的**识别式**（`## <关键词> N · …`）。
ENTRY_RE = re.compile(r"^##\s+(模式|死法|裁定|配方|洞察|条目)\s*\d+")

#: 行内链接：`[text](target)`。
LINK_RE = re.compile(r"\[[^\]]*\]\(\s*<?([^)\s>]+)>?(?:\s+[\"'][^)]*[\"'])?\s*\)")

#: 出处行（每条经验至少 1 行，且必须带链接 ⇒ 能指回仓内）。
SOURCE_RE = re.compile(r"^>\s*出处[：:]")

#: 可跑命令（用于【守卫】段的强制项）。
CMD_RE = re.compile(r"(?:python3?|cargo|pwsh|bash|git)\s+[A-Za-z0-9_./-]")

#: 机器层互引的两种写法：`FP:<id>` 或指向 yaml 的链接且同行含 `<id>`。
FP_INLINE_RE = re.compile(r"FP:([A-Za-z0-9][A-Za-z0-9._-]*)")

#: 标题锚点 slug（GitHub 风格：去标点、空格转连字符、保留 CJK）。
HEADING_RE = re.compile(r"^(#{1,6})\s+(.+?)\s*$", re.MULTILINE)


def slugify(text: str) -> str:
    """GitHub 风格锚点 slug。"""
    text = text.strip().lower()
    text = re.sub(r"[^\w\- \u4e00-\u9fff]", "", text)
    return re.sub(r"\s+", "-", text)


def mine_files() -> list[Path]:
    """族内的「矿」文件（`NN-*.md`，不含 README 与 templates/）。"""
    return sorted(p for p in FAMILY.glob("[0-9][0-9]-*.md"))


def load_knowledge_ids() -> tuple[set[str], list[str]]:
    """读 `.harness/knowledge/failure-patterns.yaml` ⇒ (id 集合, 解析问题)。"""
    problems: list[str] = []
    if not KNOWLEDGE.exists():
        return set(), [f"机器层知识库不存在: {KNOWLEDGE.relative_to(ROOT)}"]
    text = KNOWLEDGE.read_text(encoding="utf-8")
    ids: set[str] = set()
    for match in re.finditer(r"^\s*-\s+id:\s*([A-Za-z0-9][A-Za-z0-9._-]*)\s*$", text, re.MULTILINE):
        ids.add(match.group(1))
    if not ids:
        problems.append("机器层知识库里解析不到任何 pattern id（格式变了？）")
    return ids, problems


def split_sections(text: str) -> list[tuple[str, int, str]]:
    """切成 (标题, 起始行号, 正文)。`###` 小节并入其 `##` 父节。"""
    lines = text.splitlines()
    out: list[tuple[str, int, str]] = []
    current: list[str] = []
    current_title = ""
    current_line = 1
    for number, line in enumerate(lines, start=1):
        if line.startswith("## ") and not line.startswith("### "):
            if current_title:
                out.append((current_title, current_line, "\n".join(current)))
            current_title = line[3:].strip()
            current_line = number
            current = [line]
        elif current_title:
            current.append(line)
    if current_title:
        out.append((current_title, current_line, "\n".join(current)))
    return out


def segments_of(body: str) -> set[str]:
    """正文里出现的 `【段名】`。"""
    return {m.group(1) for m in re.finditer(r"【([^】]+)】", body)}


def segment_text(body: str, name: str) -> str:
    """取 `【name】` 段到下一个 `【…】` 之前的正文。"""
    match = re.search(rf"【{re.escape(name)}】(.*?)(?=【[^】]+】|\Z)", body, re.DOTALL)
    return match.group(1) if match else ""


def check_command_bearing_segments(path: Path, problems: list[str]) -> None:
    """06/07 型条目：**【守卫】段必须含一条可跑命令**；**【锚点】段必须含仓内链接**。

    为什么单列这两条：
    * 【守卫】只写「（无）」⇒ 经验就退化成散文 ⇒ 违反「每条可判定的断言都要有守卫」
      （`workline-registry.md` §二.3）⇒ 机器层面强制「命令必须真的能跑」。
    * 【锚点】只写 commit sha ⇒ 提交会沉进历史、读者点不到 ⇒ 必须有**仓内可点击**的位置。
    """
    prefix = path.name[:3]
    if prefix not in ("06-", "07-"):
        return
    text = path.read_text(encoding="utf-8")
    for title, line_no, body in split_sections(text):
        if not ENTRY_RE.match(f"## " + title):
            continue
        guard_seg = segment_text(body, "守卫")
        if guard_seg and not CMD_RE.search(guard_seg):
            problems.append(
                f"{path.relative_to(ROOT)}:{line_no}: 条目「{title}」的【守卫】段没有可跑命令"
                f"（须形如 `python3 scripts/xxx.py --check` / `cargo test …`）"
            )
        anchor_seg = segment_text(body, "锚点")
        if anchor_seg and not LINK_RE.search(anchor_seg):
            problems.append(
                f"{path.relative_to(ROOT)}:{line_no}: 条目「{title}」的【锚点】段没有仓内链接"
                f"（只写 commit sha ⇒ 读者点不到）"
            )


def check_segments(path: Path, problems: list[str]) -> None:
    prefix = path.name[:3]
    if prefix in MANUAL_PREFIXES:
        return  # 导航册：工序卡不是矿 ⇒ 不判条目段型
    required = REQUIRED_SEGMENTS.get(prefix)
    if required is None:
        problems.append(f"{path.relative_to(ROOT)}: 未登记条目段型制（前缀 {prefix!r} 不在守卫表里）")
        return
    text = path.read_text(encoding="utf-8")
    for title, line_no, body in split_sections(text):
        if not ENTRY_RE.match(f"## " + title):
            continue
        found = segments_of(body)
        missing = [seg for seg in required if seg not in found]
        if missing:
            problems.append(
                f"{path.relative_to(ROOT)}:{line_no}: 条目「{title}」缺必填段 "
                f"{missing}（该型必填 {list(required)}）"
            )
        sources = [ln for ln in body.splitlines() if SOURCE_RE.match(ln.strip())]
        if not sources:
            problems.append(
                f"{path.relative_to(ROOT)}:{line_no}: 条目「{title}」没有「> 出处：」行"
                f"（经验必须能指回仓内可复核的锚点）"
            )
        elif not any(LINK_RE.search(ln) for ln in sources):
            problems.append(
                f"{path.relative_to(ROOT)}:{line_no}: 条目「{title}」的出处行没有链接"
                f"（指不回仓内 = 散文）"
            )


def check_anchors(path: Path, problems: list[str]) -> None:
    """出处行上的相对链接目标必须存在（标题锚点也校验）。"""
    text = path.read_text(encoding="utf-8")
    for number, line in enumerate(text.splitlines(), start=1):
        if not SOURCE_RE.match(line.strip()):
            continue
        for match in LINK_RE.finditer(line):
            target = match.group(1)
            if target.startswith(("http://", "https://", "mailto:")):
                continue
            raw, _, anchor = target.partition("#")
            if not raw:
                continue
            candidate = (path.parent / raw).resolve()
            if not candidate.exists():
                problems.append(
                    f"{path.relative_to(ROOT)}:{number}: 出处指向不存在的文件 → {raw}"
                )
                continue
            if anchor and candidate.suffix == ".md":
                headings = {
                    slugify(h.group(2))
                    for h in HEADING_RE.finditer(candidate.read_text(encoding="utf-8"))
                }
                if slugify(anchor) not in headings:
                    problems.append(
                        f"{path.relative_to(ROOT)}:{number}: 出处的标题锚点不存在 → "
                        f"{raw}#{anchor}"
                    )


def check_index(problems: list[str]) -> None:
    """族内 README 的表必须覆盖每一篇矿（且不许列不存在的篇）。"""
    if not INDEX.exists():
        problems.append(f"族内索引不存在: {INDEX.relative_to(ROOT)}")
        return
    text = INDEX.read_text(encoding="utf-8")
    listed = {m.group(1) for m in LINK_RE.finditer(text)}
    for path in mine_files():
        if path.name not in listed:
            problems.append(
                f"{INDEX.relative_to(ROOT)}: 未列出矿篇 {path.name}（索引覆盖 ⇒ 接手者查不到）"
            )
    real = {p.name for p in mine_files()}
    for name in sorted(listed):
        if name.endswith(".md") and name[:2].isdigit() and name not in real:
            problems.append(f"{INDEX.relative_to(ROOT)}: 列了不存在的矿篇 {name}")


def check_class(problems: list[str]) -> None:
    """**新章**必带 `Class:`（与 `check_doc_class.py` 的九类同源；存量在基线里 ⇒ 不追责）。"""
    baseline = read_doc_class_baseline()
    for path in mine_files():
        rel = str(path.relative_to(ROOT))
        if rel in baseline:
            continue  # 存量未分类（基线已登记）⇒ 不在本守卫追责范围
        head = "\n".join(path.read_text(encoding="utf-8").splitlines()[:60])
        if "[Document-Meta]" not in head:
            problems.append(f"{rel}: 缺 `[Document-Meta]` 头")
        elif not re.search(r"^Class:\s*\S+", head, re.MULTILINE):
            problems.append(f"{rel}: 缺 `Class:` 分类（九类之一）")


def read_doc_class_baseline() -> set[str]:
    """读 `scripts/doc-class-baseline.txt`（存量未分类清单）⇒ 集合。"""
    path = ROOT / "scripts" / "doc-class-baseline.txt"
    if not path.exists():
        return set()
    out: set[str] = set()
    for line in path.read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        out.add(line.split("#")[0].strip())
    return out


def check_knowledge_link(problems: list[str]) -> None:
    """机器层 ⇄ 人读层互引：yaml 的每个 id 都被引用；引用的 id 都存在。"""
    ids, load_problems = load_knowledge_ids()
    problems.extend(load_problems)
    if not ids:
        return
    cited: set[str] = set()
    for path in FAMILY.rglob("*.md"):
        text = path.read_text(encoding="utf-8")
        for match in FP_INLINE_RE.finditer(text):
            cited.add(match.group(1))
        for match in LINK_RE.finditer(text):
            target = match.group(1)
            if target.endswith("failure-patterns.yaml"):
                window = text[max(0, match.start() - 200) : match.end() + 200]
                for ident in re.findall(r"([A-Za-z0-9][A-Za-z0-9._-]{3,})", window):
                    if ident in ids:
                        cited.add(ident)
    for ident in sorted(ids - cited):
        problems.append(
            f"{KNOWLEDGE.relative_to(ROOT)}: 失败模式 `{ident}` **未进经验矿**"
            f"（机器层与人读层各说各话 ⇒ 下次排查照旧扑空）"
        )
    for ident in sorted(cited - ids):
        problems.append(
            f"{KNOWLEDGE.relative_to(ROOT)}: 经验矿引用了不存在的失败模式 `FP:{ident}`"
        )


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(prog="scripts/check_experience_anchors.py", description=__doc__)
    group = parser.add_mutually_exclusive_group(required=True)
    group.add_argument("--check", action="store_true", help="门禁：有违规即 exit 1")
    group.add_argument("--report", action="store_true", help="只打印报告")
    args = parser.parse_args(argv[1:])

    if not FAMILY.exists():
        print(f"FAIL: 经验矿目录不存在: {FAMILY.relative_to(ROOT)}", file=sys.stderr)
        return 1

    problems: list[str] = []
    files = mine_files()
    for path in files:
        check_segments(path, problems)
        check_command_bearing_segments(path, problems)
        check_anchors(path, problems)
    check_index(problems)
    check_class(problems)
    check_knowledge_link(problems)

    if args.report:
        print(f"经验矿 {len(files)} 篇")
        for path in files:
            print(f"  - {path.relative_to(ROOT)}")
        print(f"问题 {len(problems)} 处")
        for problem in problems:
            print(f"  · {problem}")
        return 0

    if problems:
        print(f"FAIL: 经验矿守卫（{len(problems)} 处）", file=sys.stderr)
        for problem in problems:
            print(f"  · {problem}", file=sys.stderr)
        print(
            "\n修法：\n"
            "  ① 条目缺段 ⇒ 补齐该型必填段（段名见 scripts/check_experience_anchors.py 的 "
            "REQUIRED_SEGMENTS）；\n"
            "  ② `★` 出处失效 ⇒ 改指**现存**文件/标题，或该条作废（不许留陈旧副本）；\n"
            "  ③ 索引漏篇 ⇒ 在 engineering-experience/README.md 的表里补一行；\n"
            "  ④ 机器层 id 未进矿 ⇒ 在矿里写 `FP:<id>` 引用，或补 `★` 出处行。",
            file=sys.stderr,
        )
        return 1

    print(f"OK: 经验矿一致（{len(files)} 篇 · 段型合规 · 锚点可达 · 机器层互引一致）")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
