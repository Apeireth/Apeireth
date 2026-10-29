#!/usr/bin/env python3
"""**文档分类守卫**（`scripts/check_doc_class.py`）—— 让 AI 落下的每一份文档**先分类、再被约束**。

# 为什么要有它（owner 2026-10-28 指令）

> 「漏洞全补上，让以后 AI 侧就算没有约束，**AI 落的文档都有分类**，
>   **不论后续生产与否都能严格约束**。」

⇒ 现状的漂移根源是：**文档约束靠 AI 自觉**（`AGENTS.md` / 宪章是"行为要求"）。
⇒ 本脚本把它搬到**仓库机器**上：

1. **每份活跃文档必须声明 `Class`**（在 `[Document-Meta]` 块里）；
2. **每类有强制结构**，且这些结构是**可判定**的（不是"写得好"这种主观标准）；
3. **与是否接生产无关** —— 未接线的、实验的、待决的文档**一样要分类**，
   只是各类的强制项不同（例如 `EVIDENCE` 必须写口径与复现命令、`AUDIT` 必须写未修项）。

# 文档分类（**九类**）

| Class | 是什么 | 机检强制项 |
|---|---|---|
| `SPEC` | 目标形态 / 设计稿（将来时） | 含「批次」或「验收」或「不做」之一；**且**在 `workline-registry.md` 被登记 |
| `STATUS` | 进度页（做到哪了） | 含守卫命令（`scripts/` 或 `cargo `）；**且**含「还差 / 未完成 / 待 / ❌」之一 |
| `AUDIT` | 核查 / 审计（发现了什么） | **同时**含「已修」与「未修」或「暂缓」；**且**含复核命令 |
| `ROUTE` | 路线图（已证 / 已否的路） | **同时**含「已证」与「已否」 |
| `DECISION` | 决策请求 / 裁定记录 | 含「选项」**且**含「建议」 |
| `EVIDENCE` | 单项实测 / 证据 | 含四级口径之一；**且**含复现命令；**且**含「边界 / 没测」之一 |
| `GUIDE` | 使用 / 操作指南 | 含代码块（```` ``` ````）|
| `LEDGER` | 机器读的台账 | 含「生成 / --gen / generator」之一 **且** 含「--check / 守卫」之一 |
| `INDEX` | 索引 / 清单（**由脚本生成**） | 跳过（`docs/INDEX.csv` 等） |

`docs/archive/**` 视为**历史卷宗**（`HISTORY`）⇒ 不要求分类。

# 棘轮（存量渐进，**只减不增**）

* 存量未分类的文档进基线 `scripts/doc-class-baseline.txt`
  ⇒ ⇒ **新文档必须分类**（不在基线里且没 `Class` ⇒ 失败）；
* 基线里某文档**补上分类后必须从基线删掉**（否则失败：基线不许虚高）；
* 已分类文档**违反所属类强制项** ⇒ 失败（这条没有豁免）。

# 用法

    python3 scripts/check_doc_class.py --check      # 门禁
    python3 scripts/check_doc_class.py --gen        # 重写基线（只应在"补分类"后缩表）
    python3 scripts/check_doc_class.py --report     # 打印各类分布，不改文件
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from collections import Counter, defaultdict
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DOCS = ROOT / "docs"
BASELINE = ROOT / "scripts" / "doc-class-baseline.txt"
REGISTRY = DOCS / "status" / "workline-registry.md"

#: 九类（见模块文档表）。
CLASSES = {
    "SPEC",
    "STATUS",
    "AUDIT",
    "ROUTE",
    "DECISION",
    "EVIDENCE",
    "GUIDE",
    "LEDGER",
    "INDEX",
}

#: 四级口径（与 `docs/01-architecture/system-capabilities.md` 头部定义一致）。
CALIBERS = ("NOT IMPLEMENTED", "IMPLEMENTED", "PRODUCTION WIRED", "DEFAULT ENABLED", "HARDWARE VALIDATED")

#: 这些目录不参与（历史卷宗 / 自动生成）。
SKIP_DIR_PARTS = {"archive"}
SKIP_NAMES = {"INDEX.csv", "README.md"}


def active_docs() -> list[Path]:
    """活跃文档（`docs/archive/**` 除外）。"""
    out: list[Path] = []
    for path in sorted(DOCS.rglob("*.md")):
        rel = path.relative_to(DOCS)
        if SKIP_DIR_PARTS & set(rel.parts):
            continue
        if path.name in SKIP_NAMES:
            continue
        out.append(path)
    return out


def parse_meta(text: str) -> dict[str, str]:
    """解析头部的 `[Document-Meta]` 块（`Key: Value` 行，直到空行/标题/`-->`）。

    ⚠️ **兼容两种写法**：
    * 宪章 §7.1 的裸块 `[Document-Meta]` + 后续 `Key: Value`；
    * HTML 注释包裹 `<!-- [Document-Meta] ... -->`（markdown 渲染时不可见）。
    """
    head = "\n".join(text.splitlines()[:80])
    match = re.search(r"\[Document-Meta\]", head)
    if not match:
        return {}
    tail = head[match.end() :]
    meta: dict[str, str] = {}
    for raw in tail.splitlines():
        line = raw.strip()
        if not line:
            if meta:
                break
            continue
        if line.startswith("-->") or line.startswith("#") or line.startswith("**"):
            break
        kv = re.match(r"^([A-Za-z][A-Za-z _-]*)\s*[:：]\s*(.+)$", line)
        if kv:
            meta[kv.group(1).strip().lower().replace(" ", "-")] = kv.group(2).strip()
    return meta


def has_command(text: str) -> bool:
    """正文里是否出现**可跑的命令**（脚本或 cargo）。"""
    return ("scripts/" in text and ("python3 " in text or "bash " in text)) or bool(
        re.search(r"cargo\s+(run|test|nextest|build|clippy|fmt)", text)
    )


def has_code_block(text: str) -> bool:
    return "```" in text


def heading_hits(text: str, words: tuple[str, ...]) -> list[str]:
    """**标题级**命中：关键词出现在 Markdown 标题里（不是随便出现在正文）。

    ⚠️ 为什么必须锚定标题：正文里写「本文**没有**批次章节」也会命中关键词
    ⇒ ⇒ 那样守卫就变成「形式可满足」（我自己就踩过：探针文档反着写却通过了）。
    """
    hits: list[str] = []
    for line in text.splitlines():
        if not line.lstrip().startswith("#"):
            continue
        for word in words:
            if word in line and word not in hits:
                hits.append(word)
    return hits


def marker_lines(text: str, words: tuple[str, ...]) -> list[str]:
    """**行首/表格**级命中：关键词出现在列表项、表格行或标题里。

    用于「状态标记」（已修 / 未修 / 暂缓）⇒ 防止「我们**未修**的很少」这种散文蒙过去。
    """
    hits: list[str] = []
    for line in text.splitlines():
        stripped = line.lstrip()
        anchored = (
            stripped.startswith(("|", "-", "*", "#", ">", "+"))
            or bool(re.match(r"^\d+[.)]\s", stripped))
        )
        if not anchored:
            continue
        for word in words:
            if word in line and word not in hits:
                hits.append(word)
    return hits


def class_violations(name: str, calss: str, text: str, registry_text: str) -> list[str]:
    """按类检查强制项，返回违规描述（空 = 合规）。"""
    problems: list[str] = []
    if calss == "SPEC":
        if not heading_hits(text, ("批次", "验收", "不做")):
            problems.append(
                "SPEC 必须有**标题**含「批次 / 验收 / 不做」之一"
                "（正文里随口提到不算 ⇒ 目标形态要可验收）"
            )
        # ⚠️ 登记表里用的是**相对链接** ⇒ 按**文件名**匹配（全路径匹配会永远失败）
        stem = Path(name).name
        if stem not in registry_text:
            problems.append(
                f"SPEC 必须在 docs/status/workline-registry.md 登记"
                f"（按文件名 `{stem}` 未找到 ⇒ 接手者查不到进度页）"
            )
    elif calss == "STATUS":
        if not has_command(text):
            problems.append("STATUS 必须含**可跑的守卫命令**（否则「做到哪」不可复核）")
        if not marker_lines(text, ("还差", "未完成", "❌", "待做", "待补")):
            problems.append(
                "STATUS 必须在**列表/表格**里写「还差什么 / 未完成 / ❌」"
                "（只写「已完成」不算进度页）"
            )
    elif calss == "AUDIT":
        fixed = marker_lines(text, ("已修", "已修复"))
        open_ = marker_lines(text, ("未修", "暂缓"))
        if not (fixed and open_):
            problems.append(
                "AUDIT 必须在**列表项/表格行**里**同时**写「已修」与「未修 / 暂缓」"
                "（只报好消息的审计不算审计）"
            )
        if not has_command(text):
            problems.append("AUDIT 必须含复核命令（否则结论无法复算）")
    elif calss == "ROUTE":
        if not (heading_hits(text, ("已证",)) and heading_hits(text, ("已否",))):
            problems.append(
                "ROUTE 必须有**标题**分别含「已证」与「已否」"
                "（只写一条路的路线图不算路线图）"
            )
    elif calss == "DECISION":
        if "选项" not in text or "建议" not in text:
            problems.append("DECISION 必须含「选项」与「建议」（否则 owner 无法拍板）")
    elif calss == "EVIDENCE":
        if not any(caliber in text for caliber in CALIBERS):
            problems.append("EVIDENCE 必须声明**四级口径**（否则说不清是「能跑」还是「接了生产」）")
        if not has_command(text):
            problems.append("EVIDENCE 必须含**复现命令**（无脚本不写数字）")
        if not any(word in text for word in ("边界", "没测", "未测")):
            problems.append("EVIDENCE 必须写**边界 / 没测什么**（否则数字会被当全称结论引用）")
    elif calss == "GUIDE":
        if not has_code_block(text):
            problems.append("GUIDE 必须含代码块（指南要能照做）")
    elif calss == "LEDGER":
        if not any(word in text for word in ("生成", "--gen", "generator")):
            problems.append("LEDGER 必须说明**由什么生成**（手写台账会漂）")
        if not any(word in text for word in ("--check", "守卫")):
            problems.append("LEDGER 必须配套 `--check` 守卫（否则台账不许撒谎这条无法执行）")
    return problems


def collect() -> tuple[dict[str, dict], list[str]]:
    """返回 (每份文档的信息, 未分类文档列表)。"""
    registry_text = REGISTRY.read_text(encoding="utf-8") if REGISTRY.exists() else ""
    infos: dict[str, dict] = {}
    unclassified: list[str] = []
    for path in active_docs():
        rel = str(path.relative_to(ROOT))
        try:
            text = path.read_text(encoding="utf-8")
        except (OSError, UnicodeDecodeError):
            continue
        meta = parse_meta(text)
        calss = (meta.get("class") or "").strip().upper()
        info = {"path": rel, "class": calss or None, "problems": []}
        if not calss:
            unclassified.append(rel)
        elif calss not in CLASSES:
            info["problems"].append(f"Class `{calss}` 不在九类之内（{sorted(CLASSES)}）")
        elif calss == "INDEX":
            pass  # 索引类不查结构
        else:
            info["problems"] = class_violations(rel, calss, text, registry_text)
        infos[rel] = info
    return infos, unclassified


def read_baseline() -> list[str]:
    if not BASELINE.exists():
        return []
    out: list[str] = []
    for line in BASELINE.read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        out.append(line.split("#")[0].strip())
    return out


def write_baseline(unclassified: list[str]) -> None:
    header = [
        "# 文档分类**棘轮基线**（`python3 scripts/check_doc_class.py --gen` 生成）",
        "#",
        "# 语义：这里列的是「**尚未补 Class 的存量文档**」⇒ 只许变短。",
        "#   · 新文档不许进基线（必须当场分类）；",
        "#   · 补了 Class 的文档必须从这里删掉（`--gen` 自动缩表）；",
        "#   · 基线不许虚高（有已分类的残留在里面 ⇒ 门禁失败）。",
        "#",
        f"# 已登记（待分类）：{len(unclassified)} 份",
    ]
    BASELINE.write_text(
        "\n".join(header + sorted(unclassified)) + "\n", encoding="utf-8"
    )


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    group = parser.add_mutually_exclusive_group(required=True)
    group.add_argument("--check", action="store_true", help="门禁：违规即失败")
    group.add_argument("--gen", action="store_true", help="重写基线（缩表）")
    group.add_argument("--report", action="store_true", help="打印分布")
    args = parser.parse_args()

    infos, unclassified = collect()
    baseline = read_baseline()
    baseline_set = set(baseline)
    unclassified_set = set(unclassified)

    if args.report:
        counts = Counter(info["class"] or "(未分类)" for info in infos.values())
        print(f"活跃文档 {len(infos)} 份")
        for name, count in sorted(counts.items(), key=lambda kv: -kv[1]):
            print(f"  {name:<20} {count:>4}")
        broken = {path: info["problems"] for path, info in infos.items() if info["problems"]}
        print(f"违反所属类强制项：{len(broken)} 份")
        for path, problems in list(broken.items())[:10]:
            print(f"  ✗ {path}")
            for problem in problems:
                print(f"      · {problem}")
        return 0

    if args.gen:
        write_baseline(unclassified)
        print(f"✓ 已写 {BASELINE.name}：待分类 {len(unclassified)} 份")
        return 0

    # ---------------- --check ----------------
    failures: list[str] = []

    # ① 新文档必须分类：不在基线里却没有 Class ⇒ 失败
    new_unclassified = sorted(unclassified_set - baseline_set)
    if new_unclassified:
        failures.append(
            f"**新增未分类文档 {len(new_unclassified)} 份**（必须在头部 `[Document-Meta]` 里加 `Class:`）"
        )
        failures.extend(f"   · {path}" for path in new_unclassified[:20])

    # ② 基线不许虚高：已分类却还留在基线里 ⇒ 失败（要求缩表）
    stale_baseline = sorted(baseline_set - unclassified_set)
    if stale_baseline:
        failures.append(
            f"**基线虚高 {len(stale_baseline)} 份**（已补 Class 但没从基线删 ⇒ 跑 `--gen` 缩表）"
        )
        failures.extend(f"   · {path}" for path in stale_baseline[:20])

    # ③ 已分类文档违反所属类强制项 ⇒ 失败（无豁免）
    for path, info in sorted(infos.items()):
        if info["problems"]:
            failures.append(f"{path} 违反 `{info['class']}` 类强制项：")
            failures.extend(f"   · {problem}" for problem in info["problems"])

    if failures:
        print("FAIL: 文档分类守卫", file=sys.stderr)
        for line in failures:
            print(line, file=sys.stderr)
        print(
            f"\n修法：给文档头部加\n  [Document-Meta]\n  Class: <九类之一>\n"
            f"九类：{sorted(CLASSES)}\n"
            "存量未分类的用 `--gen` 登记（只许缩表）。",
            file=sys.stderr,
        )
        return 1

    classified = sum(1 for info in infos.values() if info["class"])
    counts = Counter(info["class"] for info in infos.values() if info["class"])
    print(
        f"OK: 文档分类守卫通过（活跃 {len(infos)} 份 · 已分类 {classified} · "
        f"基线待分类 {len(unclassified)}）"
    )
    print(f"  分布：{dict(sorted(counts.items()))}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
