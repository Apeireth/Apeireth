"""Kickoff v2 — 8 问 (主人 13:04 认可)
依据: TOP-DESIGN-V1 §3.4, KICKOFF-V2-2026-07-20.md
原则: 离线 priors (Pep 范式) — 让 LLM 带着主人预设玩味

v0.1.1 修复: Q7 (永远记得 / 永远不提) 现在分到两个字段, 而不是全塞 remember_forever。
   之前 master 卡的 never_mention 字段空, 是因为解析器只切 句号, 整个 Q7 答案进了 1 个 list item。
   现在用 启发式: 含"不提/永不提/别提/禁止/never"等标记 → never_mention, 其它 → remember_forever。
"""

from __future__ import annotations
import re
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
        "field": "_q7_dual",
        "q": "你希望我永远记得什么?永远不提起什么?",
        "why": "主人 13:04: 没硬性红线 — 但问了就知道 — 双字段特殊处理",
    },
    {
        "n": 8,
        "field": "funnel_questions",
        "q": "你希望我以后不断问你什么问题?",
        "why": "Funnel 触发器 — 让提问引擎永远跑",
    },
]

# Q7 否定标记 — 命中则该项进入 never_mention
_NEG_MARKERS = ("不提", "永不提", "不许提", "别提", "禁止", "禁提", "不要提", "不要问", "never")

# 列表字段分隔符 — 中文/英文 句号/分号/逗号/顿号/换行
_LIST_SPLIT = re.compile(r"[。;；,，\n、]+")


def _clean_phrase(p: str) -> str:
    """去掉前缀修饰词 + 端部标点 — 让 remember_forever / never_mention 字段干净"""
    # 复合修饰词优先 (永远记得 / 永远不要), 否则单字
    prefixes = (
        r"(?:永远(?:记得|不提|不提) |永远(?:记得|不提))"
        r"|(?:记得|不提|不提|不要|永不|永远|都|必须|一定|绝对|请)"
    )
    pat = rf"^(?:{prefixes})[\s:：—\-、,，]*"
    # 应用 2 次 — 防止"永远记得 X"残留
    p = re.sub(pat, "", p)
    p = re.sub(pat, "", p)
    return p.strip(" \t:：—-.，,;；")


def _split_keep_ban(text: str) -> tuple[list[str], list[str]]:
    """Q7 专用解析 — 返回 (remember_forever, never_mention)。

    启发式:
      1. 按 _LIST_SPLIT 切成候选短语
      2. 含否定标记的短语 → never_mention (去掉标记后保留目标)
      3. 其余 → remember_forever
      4. 兜底: 若都空, 整段进 remember_forever (行为兼容 v0.1.0)
    """
    parts = [p for p in _LIST_SPLIT.split(text or "") if p.strip()]
    keep, ban = [], []
    for p in parts:
        hit = next((m for m in _NEG_MARKERS if m in p), None)
        if hit:
            target = _clean_phrase(p.replace(hit, "", 1))
            if target:
                ban.append(target)
        else:
            cleaned = _clean_phrase(p)
            if cleaned:
                keep.append(cleaned)

    if not keep and not ban and (text or "").strip():
        keep = [text.strip()]
    return keep, ban


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

        if field_name == "_q7_dual":
            keep, ban = _split_keep_ban(text)
            card.remember_forever = keep
            card.never_mention = ban
            continue

        if field_name in {"name", "purpose", "origin_reason", "relationship_contract"}:
            setattr(card, field_name, text)
            continue

        # 列表型字段 (Q1/Q3/Q4/Q5/Q8)
        parts = [p.strip(" \n,。.;") for p in text.replace("\n", "。").split("。") if p.strip()]
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
        if f == "_q7_dual":
            keep, ban = _split_keep_ban(ans)
            card.remember_forever = keep
            card.never_mention = ban
        elif f in {"name", "purpose", "origin_reason", "relationship_contract"}:
            setattr(card, f, ans)
        else:
            parts = [s.strip(" \n,。.;") for s in ans.replace("\n", "。").split("。") if s.strip()]
            setattr(card, f, parts if parts else [ans])
    return card
