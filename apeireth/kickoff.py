"""Kickoff v2 — 8 问 (主人 13:04 认可)
依据: TOP-DESIGN-V1 §3.4, KICKOFF-V2-2026-07-20.md
原则: 离线 priors (Pep 范式) — 让 LLM 带着主人预设玩味
"""

from __future__ import annotations
from typing import Callable
from .identity import IdentityCard, save_card

# 8 问 — 顺序锁定 (主人 13:04 第 1 条认可)
# field 是 IdentityCard 上的目标字段
KICKOFF_QUESTIONS: list[dict] = [
    {
        "n": 1,
        "field": "name",
        "q": "我能怎么称呼你?",
        "why": "中心节点的标签",
    },
    {
        "n": 2,
        "field": "purpose",
        "q": "你做什么的?你想达成什么?",
        "why": "使命 + 角色域",
    },
    {
        "n": 3,
        "field": "origin_reason",
        "q": "你为什么来找我?",
        "why": "上游因果 — 决定了平台的入口定义",
    },
    {
        "n": 4,
        "field": "archetypes",
        "q": "你希望我像什么?",
        "why": "主人 13:04: 交用户定义, 提醒不必太局限",
    },
    {
        "n": 5,
        "field": "ask_when",
        "q": "我应该什么时候问你?什么时候自己决定?什么时候提醒你?",
        "why": "自主权边界 — Funnel Question + Mom Test",
    },
    {
        "n": 6,
        "field": "relationship_contract",
        "q": "我们之间要建立什么样的关系?",
        "why": "主人 13:04: 造地基不能有杂质",
    },
    {
        "n": 7,
        "field": "remember_forever",
        "q": "你希望我永远记得什么?永远不提起什么?",
        "why": "主人 13:04: 没硬性红线 — 但问了就知道",
    },
    {
        "n": 8,
        "field": "funnel_questions",
        "q": "你希望我以后不断问你什么问题?",
        "why": "Funnel 触发器 — 让提问引擎永远跑",
    },
]


def run_kickoff(
    answerer: Callable[[str], str] | None = None,
) -> IdentityCard:
    """跑一遍 8 问 — 返回填充好的 IdentityCard。
    answerer 是可注入的回函数: 主 session 用 LLM, 测试用 lambda。
    如果不传 answerer, 就走 stdin (人工)。
    """
    card = IdentityCard()
    if answerer is None:
        return _interactive(card)

    answer = answerer
    for item in KICKOFF_QUESTIONS:
        field_name = item["field"]
        prompt = f"[Q{item['n']}/8] {item['q']}\n   (背景: {item['why']})"
        text = answer(prompt).strip()
        if field_name == "name":
            card.name = text
        elif field_name == "purpose":
            card.purpose = text
        elif field_name == "origin_reason":
            card.origin_reason = text
        elif field_name == "relationship_contract":
            card.relationship_contract = text
        else:
            # 列表型字段 (split by 句号 / 逗号)
            parts = [s.strip(" \n,。.;") for s in text.replace("\n", "。").split("。") if s.strip()]
            setattr(card, field_name, parts)
    return card


def _interactive(card: IdentityCard) -> IdentityCard:
    """人工 / stdin 模式 — 主 session 里手动跑"""
    print("=" * 60)
    print("🜂 Apeireth 中央 AI — Kickoff v2 (8 问)")
    print("   触发创世 — 主人预设 + AI 涌现")
    print("=" * 60)
    for item in KICKOFF_QUESTIONS:
        print(f"\n[Q{item['n']}/8] {item['q']}")
        print(f"      (背景: {item['why']})")
        ans = input("> ").strip()
        f = item["field"]
        if f in {"name", "purpose", "origin_reason", "relationship_contract"}:
            setattr(card, f, ans)
        else:
            parts = [s.strip(" \n,。.;") for s in ans.replace("\n", "。").split("。") if s.strip()]
            setattr(card, f, parts if parts else [ans])
    return card
