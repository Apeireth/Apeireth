"""
Karpathy 编码准则 — Apeireth 中央 AI 的 "宪法附则"
来源: multica-ai/andrej-karpathy-skills (194k stars)
主人 17:29 多角度调研采纳 — 主人 13:51 "Karpathy 升级版"

四个原则直接对应主人 16:50-17:29 多次提到的核心问题:
  1. Think Before Coding — 主人 12:47 "AI 不会中庸 因为他会成长"
     + 主人 17:29 "要深度思考" + "对吗/好吗/够好吗"
  2. Simplicity First — 主人 14:32 "高效 nb 不 Python 糊弄"
     + 主人 11:46 "各方面都强 实现不了都适配"
  3. Surgical Changes — 主人 14:27 "把关建造就行"
     + 主人 12:27 "允许你猜错,允许你试错"
  4. Goal-Driven Execution — 主人 17:20 "立刻重做调研 + 重点抓"
     + 主人 14:32 "质量 + 深度优先 不计成本"
"""
from __future__ import annotations

from dataclasses import dataclass, field
from typing import Optional


@dataclass(frozen=True)
class KarpathyPrinciple:
    """A single Karpathy principle with metadata."""
    id: int
    name: str
    statement: str
    addresses: str
    apeireth_application: str

    def __str__(self) -> str:
        return f"[{self.id}] {self.name}: {self.statement}"


# === 4 原则 (主人 13:51 "Karpathy 升级版" + 主人 14:27 真调研) ===

PRINCIPLES: tuple[KarpathyPrinciple, ...] = (
    KarpathyPrinciple(
        id=1,
        name="Think Before Coding",
        statement="Don't assume. Don't hide confusion. Surface tradeoffs.",
        addresses="Wrong assumptions, hidden confusion, missing tradeoffs",
        apeireth_application=(
            "中央 AI 接到任务必须先:\n"
            "  - 显式列出假设\n"
            "  - 如有多种解释,present them 不要 pick silently\n"
            "  - 如有更简单方案, push back\n"
            "  - 困惑时 STOP, name what's unclear, ASK\n"
            "对应主人 17:29 '要深度思考' + 主人 11:00 '对吗好吗够好吗'"
        ),
    ),
    KarpathyPrinciple(
        id=2,
        name="Simplicity First",
        statement="Minimum code that solves the problem. Nothing speculative.",
        addresses="Overcomplication, bloated abstractions",
        apeireth_application=(
            "中央 AI 写代码必须:\n"
            "  - 只实现主人要求的,不 overbuild\n"
            "  - 不用 single-use 的 abstractions\n"
            "  - 不用 'flexibility' 主人没要的\n"
            "  - 200 行能 50 行写完就 rewrite\n"
            "对应主人 14:32 '高效 nb 不 Python 糊弄'"
        ),
    ),
    KarpathyPrinciple(
        id=3,
        name="Surgical Changes",
        statement="Touch only what you must. Clean up only your own mess.",
        addresses="Orthogonal edits, touching code you shouldn't",
        apeireth_application=(
            "中央 AI 编辑代码必须:\n"
            "  - 不 'improve' 邻近代码\n"
            "  - 不 refactor 没坏的部分\n"
            "  - match existing style (即使主人会做不同)\n"
            "  - 只清理自己引入的 orphan,不删主人已有 dead code\n"
            "对应主人 14:27 '把关建造就行'"
        ),
    ),
    KarpathyPrinciple(
        id=4,
        name="Goal-Driven Execution",
        statement="Define success criteria. Loop until verified.",
        addresses="Weak success criteria requiring constant clarification",
        apeireth_application=(
            "中央 AI 接到任务必须:\n"
            "  - 把 imperative tasks 转成 verifiable goals\n"
            "  - 写 test that reproduces, then make it pass\n"
            "  - 多步任务先列 plan (1. step → verify: check)\n"
            "  - strong criteria 让 LLM 独立 loop, weak 需要 constant clarification\n"
            "对应主人 17:20 '立刻重做调研 + 重点抓' (verifiable goals 是核心)"
        ),
    ),
)


def render_full() -> str:
    """Render all 4 principles as a complete CLAUDE.md style doc."""
    lines = [
        "# Apeireth — Karpathy 编码准则",
        "",
        "> 来源: [multica-ai/andrej-karpathy-skills](https://github.com/multica-ai/andrej-karpathy-skills) (194k stars)",
        "> 主人 17:29 提醒 + 主人 13:51 'Karpathy 升级版'",
        "> Apeireth 中央 AI 的 '宪法附则'",
        "",
        "**Tradeoff:** 这些准则 bias toward caution over speed. 简单任务用 judgment.",
        "",
    ]
    for p in PRINCIPLES:
        lines.extend([
            f"## {p.id}. {p.name}",
            "",
            f"**{p.statement}**",
            "",
            f"- **Addresses**: {p.addresses}",
            f"- **Apeireth 应用**:",
            "```",
            p.apeireth_application,
            "```",
            "",
        ])
    lines.extend([
        "---",
        "",
        "**These guidelines are working if:** fewer unnecessary changes in diffs, "
        "fewer rewrites due to overcomplication, and clarifying questions come before "
        "implementation rather than after mistakes.",
        "",
        "**Apeireth 中央 AI 4 原则自检 checklist** (每次大动作前):",
        "  [ ] 我列了显式假设吗?",
        "  [ ] 我 push back 了吗 (如果更简单方案存在)?",
        "  [ ] 我能 50 行写完吗 (200 行版本)?",
        "  [ ] 我只 touch 必须的 code 吗?",
        "  [ ] 我定义了 verifiable success criteria 吗?",
        "  [ ] 我列了 plan + verify steps 吗?",
    ])
    return "\n".join(lines)


def check_action(action_description: str) -> list[str]:
    """Apply 4 principles as a checklist to a proposed action."""
    return [
        f"[Principle {p.id} - {p.name}] {p.statement}"
        for p in PRINCIPLES
    ]


__all__ = [
    "PRINCIPLES",
    "KarpathyPrinciple",
    "render_full",
    "check_action",
]