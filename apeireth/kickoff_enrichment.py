"""Kickoff v0.4 enrichment — 给 IdentityCard 注入"召回锚点" + 证据引用 + 完整度
依据: TOP-DESIGN-V1 §3.4 (启动创世) + §4.1 (Identity Store)

Phase 1.0 → 1.4 演化:
- v0.1: 8 问 + JSON 身份卡 (raw)
- v0.2: 加入 recall_anchor / evidence_refs 字段 (但 kickoff 不填)
- v0.3: SqliteIdentityStore 真持久化 + FTS5
- v0.4 (本次): kickoff 输出 → 立刻富化 (enrich) → 落到 SqliteIdentityStore

enrich() 是 Phase 1 与 Phase 1.5 (AnySearch) / Phase 2 (Memory) / Phase 3 (Graph) 之间的胶水:
  - recall_anchor: 1 句话, 危急时 recall 用 (Bostrom "stable identity under stress")
  - evidence_refs: 跨层 (memory/graph) 锚点占位, 等 Phase 2/3 落地后回填真 id
  - completeness_score: 0-1, 主人随时可查"中央 AI 长成度"
  - version_status: card_version vs current CARD_VERSION → 是否需要 migrate
"""

from __future__ import annotations
import re
import time
from dataclasses import dataclass, field, asdict
from typing import Iterable

from .identity import IdentityCard, CARD_VERSION
from .identity_store import (
    IDENTITY_STORE_VERSION,
    FIELD_SCHEMA,
    validate_card,
    migrate_card,
)

# 用于剪裁长句的截断长度
ANCHOR_MAX = 80
EVIDENCE_PREFIX = "seed://"


# ---------- 派生: recall_anchor (1 句话召回) ----------
def derive_recall_anchor(card: IdentityCard) -> str:
    """从 name + purpose + relationship_contract 提炼 1 句话 — 危急时 recall 用。

    规则:
      1. 若三者都空 → 返回 "(尚未形成 anchor — 完成 8 问后回填)"
      2. 拼接模板: "{name} · {purpose_cap} · {rel_cap}"
      3. 截断到 ANCHOR_MAX 字符 (汉字按 1 计)
      4. 末尾用 "—" 分隔符, 不加句号 (master 12:14 原话"造地基不能有杂质")
    """
    name = (card.name or "").strip()
    purpose = (card.purpose or "").strip()
    rel = (card.relationship_contract or "").strip()

    if not (name or purpose or rel):
        return "(尚未形成 anchor — 完成 8 问后回填)"

    parts: list[str] = []
    if name:
        parts.append(name)
    if purpose:
        # 截前 32 字, 防 recall_anchor 过长
        parts.append(_truncate(purpose, 32))
    if rel:
        parts.append(_truncate(rel, 32))

    text = " · ".join(parts)
    return _truncate(text, ANCHOR_MAX)


def _truncate(s: str, n: int) -> str:
    if len(s) <= n:
        return s
    return s[: n - 1].rstrip() + "…"


# ---------- 派生: evidence_refs (跨层锚点占位) ----------
def suggest_evidence_refs(card: IdentityCard) -> list[str]:
    """为 8 问的每个非空字段生成一个 seed:// 引用 — 等 Phase 2/3 落地后回填真 id。

    命名约定 (确定性, 跨 session 稳定):
      seed://kickoff/Q{n}/{field_slug}
      seed://master/{field_slug}     # 中心节点引用
    """
    refs: list[str] = []

    # 1) 8 问的 field → question n 映射
    from .kickoff import KICKOFF_QUESTIONS
    qfield_to_n: dict[str, int] = {item["field"]: item["n"] for item in KICKOFF_QUESTIONS}

    for f in (
        "name",
        "purpose",
        "origin_reason",
        "archetypes",
        "ask_when",
        "relationship_contract",
        "_q7_dual",
        "funnel_questions",
    ):
        n = qfield_to_n.get(f)
        if n is None:
            continue
        # 检查对应字段是否非空
        val = _peek(card, f)
        if val:
            slug = "q7" if f == "_q7_dual" else f
            refs.append(f"{EVIDENCE_PREFIX}kickoff/Q{n}/{slug}")

    # 2) 中心节点引用 — 必填
    refs.append(f"{EVIDENCE_PREFIX}master/central_ai")

    # 3) 去重保序
    seen: set[str] = set()
    out: list[str] = []
    for r in refs:
        if r not in seen:
            seen.add(r)
            out.append(r)
    return out


def _peek(card: IdentityCard, f: str) -> str:
    """取字段值;Q7 双字段任一非空即视为非空"""
    if f == "_q7_dual":
        return card.remember_forever or card.never_mention
    return getattr(card, f, "") or ""


# ---------- 派生: completeness_score ----------
# 8 问 + 6 派生字段 (mission / domains / boundaries / emergence_space 等) — 总 14 项
COMPLETENESS_FIELDS: tuple[str, ...] = (
    "name",
    "purpose",
    "origin_reason",
    "archetypes",
    "ask_when",
    "relationship_contract",
    "_q7_keep",        # remember_forever
    "_q7_ban",         # never_mention
    "funnel_questions",
    # 派生 (master 想"地基完整"应该填的)
    "mission",
    "domains",
    "boundaries",
    "emergence_space",
    "recall_anchor",   # enrichment 自身也会写
)


def compute_completeness(card: IdentityCard) -> float:
    """完整度评分 0-1 = 非空字段数 / 总字段数。

    _q7_keep / _q7_ban 至少一个有就算非空 (Q7 双字段本质是一问)。
    recall_anchor 是 enrichment 注入的, 但纳入评分以鼓励"先跑 enrichment 再打分"。
    """
    filled = 0
    for f in COMPLETENESS_FIELDS:
        if f == "_q7_keep":
            if card.remember_forever:
                filled += 1
        elif f == "_q7_ban":
            if card.never_mention:
                filled += 1
        else:
            v = getattr(card, f, None)
            if v:  # 非空 str / 非空 list
                filled += 1
    return round(filled / len(COMPLETENESS_FIELDS), 3)


# ---------- 版本检查 ----------
def check_version(card: IdentityCard) -> dict:
    """返回 card 版本对齐状态 — 决定是否需要 migrate。

    Returns:
      {"card_identity_version": str,
       "card_store_version": str | None,
       "current_identity_version": str,
       "current_store_version": str,
       "needs_migration": bool,
       "schema_valid": bool,
       "schema_errors": list[str]}
    """
    raw: dict = card.to_dict()
    sv = raw.get("store_version")  # v0.3+ 才会有
    valid_errors = validate_card(card) or []
    needs = (card.apeireth_version != CARD_VERSION) or bool(valid_errors)
    return {
        "card_identity_version": card.apeireth_version,
        "card_store_version": sv,
        "current_identity_version": CARD_VERSION,
        "current_store_version": IDENTITY_STORE_VERSION,
        "needs_migration": needs,
        "schema_valid": not valid_errors,
        "schema_errors": list(valid_errors),
    }


# ---------- 顶层入口: enrich ----------
@dataclass
class EnrichmentReport:
    """enrich() 的产出 — 一并写入 master 卡时作为 metadata 用"""

    card: IdentityCard
    recall_anchor: str
    evidence_refs: list[str]
    completeness_score: float
    version_status: dict
    enriched_at: float = field(default_factory=time.time)

    def to_dict(self) -> dict:
        return {
            "card": self.card.to_dict(),
            "recall_anchor": self.recall_anchor,
            "evidence_refs": list(self.evidence_refs),
            "completeness_score": self.completeness_score,
            "version_status": self.version_status,
            "enriched_at": self.enriched_at,
        }


def enrich(card: IdentityCard, *, write_back: bool = True) -> EnrichmentReport:
    """给 IdentityCard 注入 enrichment (recall_anchor + evidence_refs + completeness + version_status)。

    Args:
        card: kickoff 输出 (或 load_card 回读的旧卡)
        write_back: True 时直接把 recall_anchor / evidence_refs 写回 card (in-place)
    Returns:
        EnrichmentReport — 包含完整可观测产物
    """
    # 1) 派生 recall_anchor
    anchor = derive_recall_anchor(card)

    # 2) 派生 evidence_refs
    refs = suggest_evidence_refs(card)

    # 3) 完整度 (enrichment 之前算, 看 raw 完整度)
    raw_score = compute_completeness(card)

    # 4) 版本状态
    vstatus = check_version(card)

    # 5) write-back
    if write_back:
        card.recall_anchor = anchor
        card.evidence_refs = refs

    # 6) 写回后再算一次完整度 (recall_anchor 已注入, 应更高)
    final_score = compute_completeness(card) if write_back else raw_score

    return EnrichmentReport(
        card=card,
        recall_anchor=anchor,
        evidence_refs=refs,
        completeness_score=final_score,
        version_status=vstatus,
    )


def migrate(card: IdentityCard) -> IdentityCard:
    """显式 migrate — Phase 1 PoC 老卡升级到当前 schema。"""
    return migrate_card(card)
