#!/usr/bin/env python3
"""文档索引与分层一致性检查 —— 让"文档归类"变成可复核的机器判据。

用法::

    python3 scripts/docs_index.py --check    # 校验（有问题 exit 1）
    python3 scripts/docs_index.py --gen      # 重新生成 docs/INDEX.csv
    python3 scripts/docs_index.py --report   # 人类可读统计

被校验的四条纪律：

1. **每个目录有索引**：`docs/` 下每个子目录（含 `archive/`）必须有 `README.md`。
2. **索引覆盖**：`docs/INDEX.csv` 必须覆盖 `docs/` 与 `reports/` 下每个 .md。
3. **重定向表自洽**：`docs/_redirects.tsv` 里每条旧路径要么已不存在、要么只一个
   短桩（<= 10 行）；每条新路径必须真实存在。
4. **层与题材合法**：`INDEX.csv` 的 `layer` 必须是 L1..L5，`topic` 必须是 1..12。

设计取舍：脚本只做**机器能判**的事（存在性、覆盖、枚举合法性、行数上限）。
内容质量、分类是否恰当这类需要人判断的，只报告不阻塞。
"""

from __future__ import annotations

import argparse
import csv
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

DOCS = Path("docs")
REPORTS = Path("reports")
INDEX_CSV = Path("docs/INDEX.csv")
REDIRECTS = Path("docs/_redirects.tsv")

SKIP_DIRS = {".git", "target", "legacy", "node_modules", "dist", ".venv"}



def nested_worktree_dirs(root: Path) -> set[str]:
    """仓内**嵌套**的其他 git 工作树根（相对 root 的第一层目录名）。

    判据：该目录下有 `.git` **文件**（主仓的 `.git` 是目录；`git worktree add`
    出来的副本是 `.git` 文件，内容 `gitdir: ...`）。

    为什么必须剪：嵌套工作树里带着**本仓的完整副本**。不剪的话，副本中的文档会被
    当成本仓文档**重复扫描** —— 路径前缀判断随之失准（副本里的
    `xxx/docs/archive/foo.md` 相对主仓不以 `docs/archive/` 开头，历史卷宗被当成活跃文档）。
    口径门禁实测：仓内建一个 worktree 后，活跃文档数从 415 涨到 1190、
    凭空多出 24 处「新增违规」，全是副本造成的假阳性。
    """
    out: set[str] = set()
    try:
        for child in root.iterdir():
            if child.is_dir() and (child / ".git").is_file():
                out.add(child.name)
    except OSError:
        pass
    return out

_NESTED_WT = nested_worktree_dirs(ROOT)
LAYERS = {"L1", "L2", "L3", "L4", "L5"}
TOPICS = {str(i) for i in range(1, 13)}

TOPIC_NAMES = {
    "1": "决策",
    "2": "契约与纪律",
    "3": "现状真源",
    "4": "设计与规格",
    "5": "手册与操作",
    "6": "研究与调研",
    "7": "审计与核对",
    "8": "交接与接手",
    "9": "交付与验收",
    "10": "经验",
    "11": "教训",
    "12": "历史与卷宗",
}

LAYER_NAMES = {
    "L1": "真源",
    "L2": "决策",
    "L3": "常青",
    "L4": "在飞",
    "L5": "卷宗",
}

STUB_MAX_LINES = 10
DATE_RE = re.compile(r"(20\d{2})[-/](\d{2})[-/](\d{2})")
HEADING_RE = re.compile(r"^\s{0,3}#\s+(.+?)\s*$", re.M)


def iter_md(base: Path) -> list[Path]:
    root = ROOT / base
    if not root.exists():
        return []
    out = []
    for p in root.rglob("*.md"):
        rel = p.relative_to(ROOT)
        if any(part in SKIP_DIRS for part in rel.parts):
            continue
        if rel.parts and rel.parts[0] in _NESTED_WT:
            continue
        out.append(p)
    return sorted(out)


def title_of(path: Path) -> str:
    try:
        text = path.read_text(encoding="utf-8", errors="replace")
    except OSError:
        return ""
    m = HEADING_RE.search(text)
    return (m.group(1).strip() if m else path.stem)[:120]


def date_of(path: Path) -> str:
    m = DATE_RE.search(path.name)
    if m:
        return f"{m.group(1)}-{m.group(2)}-{m.group(3)}"
    try:
        text = path.read_text(encoding="utf-8", errors="replace")[:2000]
    except OSError:
        return ""
    m = DATE_RE.search(text)
    return f"{m.group(1)}-{m.group(2)}-{m.group(3)}" if m else ""


TEXT_SUFFIXES = {".md", ".rs", ".yml", ".yaml", ".toml", ".ps1", ".py", ".sh"}
PATH_TOKEN_RE = re.compile(r"[\w][\w./-]*\.(?:md|rs|yml|yaml|toml|ps1|py|sh)")


def build_ref_counts(docs: list[Path], reports: list[Path]) -> dict[str, int]:
    """单次扫描全仓文本，统计每个文档的被引用次数。

    不用 `grep -rn <文件名>`：那会让所有同名文件（尤其是 README.md）串味，
    也无法区分 `docs/x/foo.md` 与 `docs/y/foo.md`。这里改为先抽出所有
    路径型 token，再按"最长后缀匹配"归到具体文档。
    """
    targets = [p.relative_to(ROOT).as_posix() for p in docs + reports]
    by_name: dict[str, list[str]] = {}
    for t in targets:
        by_name.setdefault(Path(t).name, []).append(t)
    # 长名优先，避免 README.md 抢先匹配
    names_sorted = sorted(by_name, key=len, reverse=True)

    counts: dict[str, int] = {t: 0 for t in targets}

    for src in ROOT.rglob("*"):
        if not src.is_file() or src.suffix not in TEXT_SUFFIXES:
            continue
        rel = src.relative_to(ROOT)
        if any(part in SKIP_DIRS for part in rel.parts):
            continue
        if rel.parts and rel.parts[0] in _NESTED_WT:
            continue
        try:
            text = src.read_text(encoding="utf-8", errors="replace")
        except OSError:
            continue
        seen: set[str] = set()
        for token in PATH_TOKEN_RE.findall(text):
            token = token.lstrip("./")
            if not token.endswith(".md"):
                continue
            base = Path(token).name
            cands = by_name.get(base)
            if not cands:
                continue
            # token 必须以目标路径结尾（相对或绝对引用都覆盖）
            for cand in cands:
                if token == cand or token.endswith("/" + cand) or cand.endswith("/" + token):
                    seen.add(cand)
                    break
            else:
                # 同名多文件：只能按目录前缀再判一次
                for cand in cands:
                    if token.endswith("/" + base) and token.count("/") >= cand.count("/"):
                        seen.add(cand)
                        break
        for c in seen:
            counts[c] += 1
    return counts


def layer_of(rel: Path) -> str:
    s = rel.as_posix()
    if s.startswith("docs/03-reference/") or s.startswith("docs/status/"):
        return "L1"
    if s.startswith("docs/adr/"):
        return "L2"
    if s.startswith("docs/architecture/"):
        return "L2"
    if s.startswith("docs/archive/"):
        return "L5"
    if s.startswith("docs/04-internal/"):
        return "L4"
    if s.startswith("docs/"):
        return "L3"
    if s.startswith("reports/"):
        return "L5"
    return "L3"


def topic_of(rel: Path) -> str:
    """粗分类：按路径/文件名关键词。仅作初值，后续人工在 INDEX.csv 里改。"""
    name = rel.name.lower()
    p = rel.as_posix()
    if rel.name == "README.md":
        return "5"
    if "handoff" in name or "hand-off" in name or "restart-brief" in name or "next-team" in name:
        return "8"
    if "audit" in name or "gap" in name or "reconcil" in name or "caliber" in name or "drill" in name:
        return "7"
    if "delivery" in name or "closeout" in name or "achievement" in name or "report" in name:
        return "9"
    if "backlog" in name or "ledger" in name or "inventory" in name or "matrix" in name or "baseline" in name:
        return "3"
    if "playbook" in name or "handbook" in name or "guide" in name or "runbook" in name or "checklist" in name:
        return "5"
    if "research" in name or "landscape" in name or "scan" in name or "archaeology" in name:
        return "6"
    if "design" in name or "spec" in name or "blueprint" in name:
        return "4"
    if "policy" in name or "discipline" in name or "manifesto" in name or "charter" in name:
        return "2"
    if "adr" in p.split("/")[:2] and p.endswith(".md"):
        return "1"
    if "experience" in p or "failure-anatomy" in name or "postmortem" in name:
        return "10"
    if "archive" in p or "history" in p:
        return "12"
    return "4"


def gen_index() -> int:
    rows: list[list[str]] = []
    doc_files = iter_md(DOCS)
    report_files = iter_md(REPORTS)
    counts = build_ref_counts(doc_files, report_files)
    # 保留人工标注过的列（--gen 不得重置）。
    # ⚠️ 关键：topic / layer 也要保留 —— 题材分类是人（或代理）判定的，
    #    启发式 `topic_of()` 只能给初值。若 --gen 覆盖它们，
    #    每跑一次生成器就擦掉一次人工分类，12 类题材体系会自己烂掉。
    prev = load_index()
    for p in doc_files + report_files:
        rel = p.relative_to(ROOT)
        old_row = prev.get(rel.as_posix(), {})
        disp = old_row.get("disposition") or ""
        tgt = old_row.get("target") or ""
        if not disp:
            disp = "归档" if "archive" in rel.parts else ("索引" if p.name == "README.md" else "保留")
        disp_final, tgt_final = disp, tgt
        layer_final = old_row.get("layer") or layer_of(rel)
        topic_final = old_row.get("topic") or topic_of(rel)
        rows.append(
            [
                rel.as_posix(),
                layer_final,
                topic_final,
                title_of(p),
                date_of(p),
                "现行" if layer_of(rel) in {"L1", "L2", "L3"} else ("在飞" if layer_of(rel) == "L4" else "归档"),
                str(counts.get(rel.as_posix(), 0)),
                disp_final,
                tgt_final,
            ]
        )
    out = ROOT / INDEX_CSV
    out.parent.mkdir(parents=True, exist_ok=True)
    with out.open("w", encoding="utf-8", newline="") as fh:
        w = csv.writer(fh)
        w.writerow(
            ["path", "layer", "topic", "title", "date", "status", "ref_count", "disposition", "target"]
        )
        w.writerows(sorted(rows))
    print(f"[docs_index] 已生成 {INDEX_CSV}（{len(rows)} 行）")
    return 0


def load_index() -> dict[str, dict[str, str]]:
    path = ROOT / INDEX_CSV
    if not path.exists():
        return {}
    out: dict[str, dict[str, str]] = {}
    with path.open(encoding="utf-8") as fh:
        for row in csv.DictReader(fh):
            if row.get("path"):
                out[row["path"].strip()] = {k: (v or "").strip() for k, v in row.items()}
    return out


def check() -> list[str]:
    problems: list[str] = []

    # 1. 每目录有 README（**递归**）
    #
    # 2026-10-24 修盲区：本规则原先只遍历 `(ROOT/DOCS).iterdir()` 一层，
    # 于是新增的**二级子目录**（如 `docs/04-internal/04-logs/`）完全不在检查面内 ——
    # 删掉它们的 README 门禁照过（对抗性复核的变异测试 C2 实证）。
    # 改为递归：凡是 `docs/` 下**含 .md 的目录**都必须有 README.md。
    for d in sorted(p for p in (ROOT / DOCS).rglob("*") if p.is_dir()):
        rel = d.relative_to(ROOT)
        if any(part in SKIP_DIRS for part in rel.parts):
            continue
        if rel.parts and rel.parts[0] in _NESTED_WT:
            continue
        if "archive" in rel.parts:  # 历史卷宗不要求索引（L5 只读）
            continue
        if not any(d.glob("*.md")):
            continue
        if not (d / "README.md").exists():
            problems.append(f"目录缺索引 README.md: {rel.as_posix()}/")

    # 2. INDEX 覆盖
    index = load_index()
    if not index:
        problems.append(f"索引文件缺失: {INDEX_CSV.as_posix()}（先跑 --gen）")
    else:
        for p in iter_md(DOCS) + iter_md(REPORTS):
            rel = p.relative_to(ROOT).as_posix()
            if rel not in index:
                problems.append(f"INDEX 未覆盖: {rel}")

    # 3. 层与题材合法
    for path, row in index.items():
        if row.get("layer") and row["layer"] not in LAYERS:
            problems.append(f"层非法: {path} -> {row['layer']}")
        if row.get("topic") and row["topic"] not in TOPICS:
            problems.append(f"题材非法: {path} -> {row['topic']}")

    # 4. 重定向表自洽
    rpath = ROOT / REDIRECTS
    if rpath.exists():
        with rpath.open(encoding="utf-8") as fh:
            for i, line in enumerate(fh, 1):
                line = line.strip()
                if not line or line.startswith("#"):
                    continue
                parts = line.split("\t")
                if len(parts) < 2:
                    problems.append(f"重定向表格式错（行 {i}）: {line}")
                    continue
                old, new = parts[0].strip(), parts[1].strip()
                old_abs = ROOT / old
                new_abs = ROOT / new
                if not new_abs.exists():
                    problems.append(f"重定向目标不存在（行 {i}）: {old} -> {new}")
                if old_abs.exists():
                    try:
                        n = len(old_abs.read_text(encoding="utf-8", errors="replace").splitlines())
                    except OSError:
                        n = 0
                    if n > STUB_MAX_LINES:
                        problems.append(
                            f"旧路径仍是实体文档而非短桩（行 {i}，{n} 行 > {STUB_MAX_LINES}）: {old}"
                        )
    return problems


def report() -> None:
    index = load_index()
    print(f"[docs_index] INDEX 行数: {len(index)}")
    by_layer: dict[str, int] = {}
    by_topic: dict[str, int] = {}
    for row in index.values():
        by_layer[row.get("layer", "?")] = by_layer.get(row.get("layer", "?"), 0) + 1
        by_topic[row.get("topic", "?")] = by_topic.get(row.get("topic", "?"), 0) + 1
    print("[docs_index] 按层:")
    for k in sorted(by_layer):
        print(f"    {k} {LAYER_NAMES.get(k, ''):6} {by_layer[k]:5}")
    print("[docs_index] 按题材:")
    for k in sorted(by_topic, key=lambda x: (len(x), x)):
        print(f"    {k:>2} {TOPIC_NAMES.get(k, '?'):8} {by_topic[k]:5}")


def health() -> None:
    """体系自检：让分类体系的失衡**自己浮出来**（而非等人发现）。

    这不是"自我进化"——没人会让文档自己重写自己。能做的是**让漂移可见**：
    题材空缺（分类体系需要增删）、目录密度失衡（哪里还乱）、
    豁免表规模（腐化风险面）。有了信号，演进才是有人/有代理在**有依据**地做。
    """
    index = load_index()
    by_topic: dict[str, int] = {}
    by_layer: dict[str, int] = {}
    for row in index.values():
        by_topic[row.get("topic", "?")] = by_topic.get(row.get("topic", "?"), 0) + 1
        by_layer[row.get("layer", "?")] = by_layer.get(row.get("layer", "?"), 0) + 1

    print("[health] 题材分布（0 篇 = 该题材已空，可考虑并入他类）")
    for k in sorted(TOPIC_NAMES, key=lambda x: int(x)):
        n = by_topic.get(k, 0)
        mark = "  ← 空" if n == 0 else ""
        print(f"    {k:>2} {TOPIC_NAMES[k]:<10} {n:5}{mark}")
    extra = sorted(set(by_topic) - set(TOPIC_NAMES) - {"?"})
    if extra:
        print(f"    ⚠️ 索引里有表外题材: {extra}（应归入 1-12 之一）")

    print("[health] 层分布")
    for k in sorted(LAYER_NAMES):
        print(f"    {k} {LAYER_NAMES[k]:<6} {by_layer.get(k, 0):5}")

    print("[health] 目录密度 TOP 12（.md 数量；密度高 = 导航成本高）")
    dirs: dict[str, int] = {}
    for p in (ROOT / DOCS).rglob("*.md"):
        rel = p.relative_to(ROOT)
        if "archive" in rel.parts:
            continue
        dirs[str(rel.parent)] = dirs.get(str(rel.parent), 0) + 1
    for k, v in sorted(dirs.items(), key=lambda kv: -kv[1])[:12]:
        flag = "  ← 偏密" if v > 15 else ""
        print(f"    {v:5}  {k}{flag}")

    km = ROOT / "docs/_known-missing.tsv"
    rd = ROOT / "docs/_redirects.tsv"
    n_km = sum(1 for ln in km.read_text(encoding="utf-8").splitlines() if ln.strip() and not ln.startswith("#")) if km.exists() else 0
    n_rd = sum(1 for ln in rd.read_text(encoding="utf-8").splitlines() if ln.strip() and not ln.startswith("#")) if rd.exists() else 0
    print(f"[health] 豁免表 {n_km} 条（腐化风险面：每条都是门禁放过的一处）")
    print(f"[health] 重定向表 {n_rd} 条（旧路径 → 新位置的可达性保证）")


def main() -> int:
    ap = argparse.ArgumentParser(description="文档索引与分层一致性检查")
    ap.add_argument("--check", action="store_true")
    ap.add_argument("--gen", action="store_true")
    ap.add_argument("--report", action="store_true")
    ap.add_argument("--health", action="store_true", help="体系自检：题材/层/目录密度/豁免表规模")
    args = ap.parse_args()

    if args.gen:
        return gen_index()

    if args.report:
        report()

    if args.health:
        health()

    if args.check:
        problems = check()
        print(f"[docs_index] 问题数: {len(problems)}")
        for p in problems[:100]:
            print(f"  {p}")
        if len(problems) > 100:
            print(f"  ... 其余 {len(problems) - 100} 条省略")
        return 1 if problems else 0
    return 0


if __name__ == "__main__":
    sys.exit(main())
