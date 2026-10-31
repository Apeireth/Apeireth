#!/usr/bin/env python3
"""自述词表纪律检查器（**只报告，不阻塞**）。

依据：ADR-0014 §6「最小诚实自述集」与外部顾问的硬判据 ——
「每一句自述都必须能对应到一个当前可验证的观察、一个明确的测量程序，或一个可追责的承诺」。

**检查对象 = 用户真正看到的东西**：渲染函数里写进 JSON 的**键名与字符串字面量**
（不是源码注释 —— 注释给开发者看，用户看不到；顾问的判据管的是「自述」）。

两类命中：

1. **体验性词汇进入自述面**：用户会在自述里看到 `Nociception` / `Attachment` / `Ritual`
   这类**内在体验名**。顾问的可接受词表是「检测到 / 分类为 / 估计 / 依据 /
   发现冲突 / 当前无法验证 / 已执行 / 已请求 / 未执行」—— 体验性名词不是其中任何一类。
2. **格式化无知**：**单个**体验变量写成 `null`（形如 `pain: null` 配 `reason:
   sensor unavailable`）—— 这把「没有证据」伪装成「有一个变量，只是传感器坏了」。
   区分：**整节**未启用写 `null + 原因` 是**合理的**（缺失原因对用户有决策价值，
   顾问原文明确认可这两种例外），本检查器只抓**单个体验变量**这一层。

为什么只报告不阻塞：
- 21 节自述是**冻结合同**（`SELF_STATUS_SECTIONS`），改它要同步 render / schema / fixture 三侧；
- 直接删改冻结合同会同时破坏「历史不追溯改写」与「冻结合同」两条纪律；
- 正确顺序是**先让违反显形，再还债**（债务预算每波 ≤3 权重分）。

用法:
  scripts/check_self_status_vocab.py            # 报告（退出码 0；命中只提示）
  scripts/check_self_status_vocab.py --strict   # 命中即退出码 1（供未来转 blocking 前演练）
"""
import argparse
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
TOOLS = ROOT / "crates/capabilities/tools/src"

# 顾问给的可接受词表（用于报告，不做自动替换）。
ALLOWED_HINTS = (
    "检测到", "分类为", "估计", "依据", "发现冲突", "当前无法验证",
    "已执行", "已请求", "未执行",
)

# 体验性词汇 → 语义说明。这些词进入用户可见的自述 JSON 时会被点名。
EXPERIENCE_TERMS = {
    "nociception": "痛觉体验",
    "pain": "痛觉体验",
    "attachment": "依恋体验",
    "ritual": "仪式/关系行为",
    "quorum": "群体感应",
    "play": "幼态延续/游戏",
    "affection": "情感状态",
    "emotion": "情绪体验",
    "feeling": "情绪体验",
    "mood": "情绪体验",
    "bond": "羁绊关系",
}

# 单个体验变量的 `null`（格式化无知）；整节级别的 `null` 不算。
SINGLE_VAR_NULL_RE = re.compile(
    r'"(?P<key>[a-z0-9_]+)"\s*:\s*null',
    re.IGNORECASE,
)

# 机制 wire 名（`BioMechanismKind::as_str` 的取值 = 用户在自述里看到的键位）。
#
# 这些名**不是** JSON 字面量（是 `kind.as_str()` 的运行时值），所以静态扫
# `json!` 块抓不到 —— 必须扫「以引号字面量形式出现」的位置（`as_str()` 实现处），
# 因为那正是它们成为用户可见键位的地方。
MECHANISM_WIRE_NAMES = {
    "nociception": "痛觉机制（读数名仍是体验名）",
    "attachment": "依恋机制（读数名仍是体验名）",
    "ritual": "仪式机制（关系行为名）",
    "quorum": "群体感应机制",
    "play": "游戏/幼态延续机制",
}

# 渲染面：`json!({ ... })` 里的键名与字符串字面量。
JSON_KEY_RE = re.compile(r'"([a-z0-9_]+)"\s*:')
JSON_STR_RE = re.compile(r'"([^"\\]{2,40})"')

# **读数标注标记**（D32 建议②，2026-10-26 落地）。
#
# 外部顾问的硬判据：「每一句自述都必须能对应到一个当前可验证的观察、一个明确的
# 测量程序，或一个可追责的承诺」。14 条生物机制里有 5 条的 wire 名仍是**体验名**
# （`nociception` / `attachment` / `ritual` / `quorum` / `play`）。
#
# 键位属**冻结合同**（改它要同步 render / schema / fixture 三侧）⇒ 本波不改键位
# ⇒ 采用「**保留键位 + 在节旁显式标注是读数**」（`bio_mechanisms.note`）。
#
# **本检查器怎么跟这件事对齐**（关键：**不能靠人工记忆**）：
# - 标注**在场**（渲染面出现本标记）⇒ 那 5 个 wire 名按「已标注读数」处理，
#   **不算裸命中**（因为用户看到的那一行已经把它们说成读数了）；
# - 标注**被撤掉**（标记消失）⇒ 那 5 个 wire 名**立刻回到命中**
#   ⇒ **标注不是一劳永逸的豁免，它是有守卫的**（撤标注 = 立刻显形）。
ANNOTATION_MARKER = "BIO_MECHANISMS_READING_NOTE"


def tokens(key: str) -> list[str]:
    """把键名按下划线/驼峰切成 token（避免 `display` 误命中 `play`）。"""
    parts = re.split(r"[_\-\s]+", key)
    out: list[str] = []
    for part in parts:
        if not part:
            continue
        out.extend(re.findall(r"[A-Z]+(?![a-z])|[A-Z][a-z0-9]*|[a-z0-9]+", part) or [part.lower()])
    return [token.lower() for token in out]


def render_surface_lines(text: str) -> list[tuple[int, str]]:
    """返回「渲染面」的行：(行号, 行文本)。

    渲染面 = 两处：
    1. `json!({ ... })` 块内部（主渲染函数）；
    2. `fn render_*(...)` 辅助函数体（它们也产出用户可见 JSON，
       例如 `render_bio_mechanisms` 把机制名写进 JSON 键位）。

    关键：机制名（`nociception` / `attachment` / `ritual` / `quorum` …）正是
    经由第 2 类辅助函数进入自述的 —— 只扫第 1 类会漏掉真正的命中面。
    """
    lines = text.splitlines()
    out: list[tuple[int, str]] = []
    depth = 0
    in_json = False
    in_helper = False
    helper_depth = 0
    for index, line in enumerate(lines, 1):
        stripped = line.strip()
        # 机制 wire 名以字面量出现的位置 = 它成为用户可见键位的位置。
        if any(f'"{wire}"' in line for wire in MECHANISM_WIRE_NAMES):
            out.append((index, line))
            continue
        if stripped.startswith("fn render_") and "fn " in stripped:
            in_helper = True
            helper_depth = 0
        if "json!(" in line:
            in_json = True
            depth = 0
        if in_json or in_helper:
            out.append((index, line))
        if in_json:
            depth += line.count("{") - line.count("}")
            if depth <= 0 and ")" in line:
                in_json = False
        if in_helper:
            helper_depth += line.count("{") - line.count("}")
            if helper_depth <= 0 and stripped == "}":
                in_helper = False
    return out


def main() -> int:
    parser = argparse.ArgumentParser(description="自述词表纪律检查（只报告）")
    parser.add_argument("--strict", action="store_true", help="命中即退出码 1")
    args = parser.parse_args()

    path = TOOLS / "self_status.rs"
    text = path.read_text(encoding="utf-8")
    surface = render_surface_lines(text)

    term_hits: list[tuple[int, str, str]] = []
    annotated_hits: list[tuple[int, str, str]] = []
    ignorance_hits: list[tuple[int, str]] = []
    seen: set[tuple[int, str]] = set()
    # 读数标注是否在场（在场 ⇒ 机制 wire 名按「已标注读数」处理）。
    annotated = any(ANNOTATION_MARKER in line for _, line in surface)
    for line_no, line in surface:
        for wire, why in MECHANISM_WIRE_NAMES.items():
            if f'"{wire}"' in line and (line_no, wire) not in seen:
                seen.add((line_no, wire))
                if annotated:
                    # 已标注读数 ⇒ **不计入裸命中**，但要单独记账（留痕）。
                    annotated_hits.append((line_no, wire, why))
                else:
                    term_hits.append((line_no, wire, why))
        for match in JSON_KEY_RE.finditer(line):
            key = match.group(1).lower()
            key_tokens = set(tokens(match.group(1)))
            for term, why in EXPERIENCE_TERMS.items():
                if term in key_tokens and (line_no, match.group(1)) not in seen:
                    seen.add((line_no, match.group(1)))
                    term_hits.append((line_no, match.group(1), why))
        for match in SINGLE_VAR_NULL_RE.finditer(line):
            key_tokens = set(tokens(match.group("key")))
            if key_tokens & set(EXPERIENCE_TERMS):
                ignorance_hits.append((line_no, match.group("key")))
        for match in JSON_STR_RE.finditer(line):
            value_tokens = set(tokens(match.group(1)))
            for term, why in EXPERIENCE_TERMS.items():
                if term in value_tokens and (line_no, match.group(1)) not in seen:
                    seen.add((line_no, match.group(1)))
                    term_hits.append((line_no, match.group(1), why))

    print("[vocab] 自述词表纪律检查（只报告，不阻塞）")
    print(f"  检查面: {path.relative_to(ROOT)} 的渲染面（json! 块）")
    print(f"  可接受词表: {' / '.join(ALLOWED_HINTS)}")

    if term_hits:
        print(f"\n  ① 体验性词汇进入自述面 {len(term_hits)} 处：")
        for line_no, word, why in term_hits:
            print(f"     L{line_no}: 「{word}」 — {why}")
        print("     → 改词方向：把「体验名」换成「检测/分类/估计」式读数名，")
        print("       或在机制名后显式标注是读数（例：pain_reading / detected_failure_class）")
    else:
        print("  ① 体验性词汇：0 命中 ✅")

    if annotated_hits:
        print(f"\n  ①' 已标注读数的机制 wire 名 {len(annotated_hits)} 处"
              f"（**不计入裸命中**，键位属冻结合同，本波不改）：")
        for line_no, word, why in annotated_hits:
            print(f"     L{line_no}: 「{word}」 — {why}")
        print(f"     → 依据：`{ANNOTATION_MARKER}` 在场（节旁已标注「这是检测读数」）")
        print("     → **标注被撤 ⇒ 立刻回到裸命中**（本检查器守着这条）")

    if ignorance_hits:
        print(f"\n  ② 格式化无知 {len(ignorance_hits)} 处（单个体验变量写成 null）：")
        for line_no, key in ignorance_hits:
            print(f"     L{line_no}: `{key}`")
        print("     → 正确写法：**直接缺席**（如「未提供『痛觉』状态，因为系统没有可验证的主观体验测量」）")
        print("     ⚠️ 整节未启用的 `null + reason` **不算**命中（缺失原因对用户有决策价值）")
    else:
        print("  ② 格式化无知：0 命中 ✅（整节级 null+原因不算）")

    print("\n  还债顺序：①落检查器（本脚本）→ ②清单进债栏（00-backlog.md）→ ③逐节裁决（每波 ≤3 权重分）")
    print("  纪律：21 节自述是冻结合同，不因一次检查就批量改。")
    return 1 if (args.strict and (term_hits or ignorance_hits)) else 0


if __name__ == "__main__":
    sys.exit(main())
