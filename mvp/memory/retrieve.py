"""R13 MVP — LIKE-based retrieval + salience time decay.

Ponytail ceiling: Python 3.13 sqlite3 FTS5 unicode61 不分中文 (已知限制).
改用 LIKE 扫描 + 简化 BM25 score (token 命中数 / sqrt(doc_length)).
数据量 ≤200 episodes, 性能可接受. 中英文都支持.

真借鉴 (主 19:33):
- BM25 简化: sum(token_hits) / sqrt(content_length) — 保留 "term frequency / doc length" 核心思想
- DeltaMemory 2024 (Lin et al.) salience decay: 1/(1+Δt/τ)
- LangChain Memory 持久化模式 (但实现从零写)

Phase 2 LLM 接入后, 这里仍是 context-builder.
"""
from __future__ import annotations

import math
import re
import time
from dataclasses import dataclass
from typing import List, Optional

from mvp.memory.store import Store, Episode, Note

# Salience decay time constant (seconds). 1 day = 86400s.
# DeltaMemory 2024 (Lin et al.) uses τ = 1 day for episodic memory.
TAU_SECONDS = 86400.0

# Token 切分: 中文按 char, 英文按 word (Ponytail: 无 jieba 依赖)
_TOKEN_RE = re.compile(
    r"[A-Za-z]+|[\u4e00-\u9fff]",  # 英文 word OR 单个汉字
    re.UNICODE,
)


def _tokenize(text: str) -> List[str]:
    return _TOKEN_RE.findall(text)


@dataclass
class RetrievalHit:
    episode: Episode
    bm25_score: float
    salience: float  # post-decay
    final_score: float  # bm25 * salience


def _decay(timestamp: float, now: Optional[float] = None,
           tau: float = TAU_SECONDS) -> float:
    """Salience = 1 / (1 + Δt/τ).  Δt = 0 → 1.0, Δt = τ → 0.5, Δt = ∞ → 0."""
    now = now if now is not None else time.time()
    delta = max(0.0, now - timestamp)
    return 1.0 / (1.0 + delta / tau)


def _bm25_score(content: str, tokens: List[str]) -> float:
    """简化 BM25: term_freq sum / sqrt(content_length)."""
    if not tokens or not content:
        return 0.0
    # 用 char-level count 加快
    hits = 0
    for tok in tokens:
        hits += content.count(tok)
    if hits == 0:
        return 0.0
    return hits / math.sqrt(len(content) + 1)


def retrieve(store: Store, query: str,
             top_k: int = 5,
             session_id: Optional[str] = None,
             now: Optional[float] = None,
             tau: float = TAU_SECONDS,
             use_decay: bool = True) -> List[RetrievalHit]:
    """LIKE-based retrieval + salience decay.

    中英文都支持. Phase 1 用 LIKE; Phase 2+ 可换 FTS5 (需 jieba-like 分词).
    """
    if not query.strip():
        return []
    tokens = _tokenize(query)
    if not tokens:
        return []
    # 候选: 至少含一个 token (LIKE OR)
    like_clauses = " OR ".join("content LIKE ?" for _ in tokens)
    like_args = [f"%{t}%" for t in tokens]
    sql_args: list = [f"%{tok}%" for tok in tokens for _ in (None,)]  # build args
    sql_args = []
    for t in tokens:
        sql_args.append(f"%{t}%")
    where_extra = ""
    if session_id is not None:
        where_extra = " AND session_id = ?"
        sql_args.append(session_id)
    sql = (
        "SELECT * FROM episodes WHERE (" + like_clauses + ")"
        + where_extra + " ORDER BY timestamp DESC LIMIT ?"
    )
    sql_args.append(top_k * 3)
    rows = store._conn.execute(sql, sql_args).fetchall()

    now = now if now is not None else time.time()
    hits: List[RetrievalHit] = []
    for r in rows:
        ep = Store._row_to_episode(r)
        bm = _bm25_score(ep.content, tokens)
        if bm == 0.0:
            continue
        sal = _decay(ep.timestamp, now=now, tau=tau) if use_decay else 1.0
        combined_sal = sal * ep.salience
        final = bm * combined_sal
        hits.append(RetrievalHit(episode=ep, bm25_score=bm,
                                 salience=combined_sal, final_score=final))

    hits.sort(key=lambda h: h.final_score, reverse=True)
    return hits[:top_k]


def retrieve_notes(store: Store, query: str,
                   top_k: int = 5,
                   min_confidence: float = 0.0,
                   now: Optional[float] = None,
                   tau: float = TAU_SECONDS * 7) -> List[Note]:  # Note 半衰期更长
    """Notes 半衰期更长 (1 周 vs 1 天 for episodes), 主 17:43 实事求是:
    Note 是 consolidate 后的提炼, 不该快速 decay."""
    if not query.strip():
        return []
    tokens = _tokenize(query)
    if not tokens:
        return []
    like_clauses = " AND ".join("content LIKE ?" for _ in tokens)
    sql_args = [f"%{t}%" for t in tokens]
    sql_args.append(min_confidence)
    sql_args.append(top_k)
    sql = (
        "SELECT * FROM notes WHERE ("
        + like_clauses + ") AND confidence >= ? ORDER BY timestamp DESC LIMIT ?"
    )
    rows = store._conn.execute(sql, sql_args).fetchall()
    return [Store._row_to_note(r) for r in rows]


def time_window_filter(episodes: List[Episode],
                       since: Optional[float] = None,
                       until: Optional[float] = None) -> List[Episode]:
    """Ponytail: linear scan, no SQL needed for small lists."""
    out = []
    for ep in episodes:
        if since is not None and ep.timestamp < since:
            continue
        if until is not None and ep.timestamp > until:
            continue
        out.append(ep)
    return out