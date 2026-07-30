"""R13 MVP Phase 1.2 — 提取层 (对话 → Note 提炼 + 合并 / 遗忘策略).

Ponytail ceiling: 纯函数 (no Store side-effects). Phase 2 LLM 接入后这里
换 LLM 提炼, Phase 1.2 是启发式 (主 17:43 实事求是: 不假装智能).

真借鉴 (主 19:33):
- DeltaMemory 2024 (Lin et al.): episodic → semantic consolidation
- Mem0: feedback-driven confidence update
- LangChain MemoryRef: rolling consolidation window
"""
from __future__ import annotations

import math
import re
import time
from collections import Counter
from typing import Iterable, List, Set, Tuple

from mvp.memory.store import Episode, Note
from mvp.identity.card import IdentityCard

# Token 切分: 与 retrieve.py 保持一致
_TOKEN_RE = re.compile(r"[A-Za-z]+|[\u4e00-\u9fff]", re.UNICODE)


def _tokenize(text: str) -> List[str]:
    return _TOKEN_RE.findall(text)


def _token_freq(text: str) -> Counter:
    """token → count. Phase 1.2 char-level + word-level."""
    return Counter(_tokenize(text))


def _cosine(a: Counter, b: Counter) -> float:
    """Cosine similarity on token frequency vectors. 0.0 if either empty."""
    if not a or not b:
        return 0.0
    dot = sum(a[t] * b[t] for t in (a.keys() & b.keys()))
    na = math.sqrt(sum(v * v for v in a.values()))
    nb = math.sqrt(sum(v * v for v in b.values()))
    if na == 0.0 or nb == 0.0:
        return 0.0
    return dot / (na * nb)


# ----- 1. extract_notes: Episode → Note 启发式提炼 -----

# 主人 / 第一人称触发词 (中文 + 英文)
_OWNER_TRIGGERS = {
    "我", "主人", "本人", "我的", "咱们", "我们",
    "I", "me", "my", "mine", "we", "us", "our",
}

# 谓词触发词 (句末判断 "是/做/在/有/研究")
_PREDICATE_TRIGGERS = {"是", "做", "在", "有", "研究", "学", "做", "来自",
                       "is", "am", "do", "doing", "from", "research"}


def extract_notes(episodes: List[Episode],
                   identity_card: IdentityCard,
                   min_token_overlap: int = 1) -> List[Note]:
    """对话 → Note 启发式提炼 (Phase 1.2).

    主 17:43 实事求是: 不假装智能. 启发式:
    - episode 含 IdentityCard 关键词 → 提炼
    - episode 含第一人称 + 谓词 → 提炼
    - confidence = 0.4 + 0.1*overlap (cap 0.9)

    Phase 2 LLM 接入后: 换成 LLM 提炼 (主 19:33 借鉴而非闭门).
    """
    if not episodes:
        return []
    # 收集种子关键词
    seed_keywords: Set[str] = set()
    for bg in identity_card.owner_background:
        seed_keywords.update(_tokenize(bg))
    for v in identity_card.owner_values:
        seed_keywords.update(_tokenize(v))

    notes: List[Note] = []
    seen_episode_ids: Set[str] = set()
    for ep in episodes:
        if ep.id in seen_episode_ids:
            continue
        tokens = set(_tokenize(ep.content))
        if not tokens:
            continue
        overlap = tokens & seed_keywords
        # 第一人称 + 谓词触发
        has_owner = bool(tokens & _OWNER_TRIGGERS)
        has_predicate = bool(tokens & _PREDICATE_TRIGGERS)
        relevance = 0
        if len(overlap) >= min_token_overlap:
            relevance += 1
        if has_owner and has_predicate:
            relevance += 1
        if relevance == 0:
            continue
        confidence = min(0.9, 0.4 + 0.1 * len(overlap)
                         + (0.1 if has_owner and has_predicate else 0.0))
        notes.append(Note(
            id="",  # store 层分配
            timestamp=time.time(),
            content=ep.content,
            source_episode_ids=[ep.id],
            confidence=confidence,
            tags=list(overlap) if overlap else [],
        ))
        seen_episode_ids.add(ep.id)
    return notes


# ----- 2. merge_similar_notes: 高相似度 Note 合并 -----

def merge_similar_notes(notes: List[Note],
                        threshold: float = 0.85) -> List[Note]:
    """合并高相似度 Note (cosine > threshold). 保留更长 content + 累加 confidence.

    Ponytail: O(n^2) pairwise, 数据量小可接受. Phase 1.4 可换近似算法.
    """
    if not notes:
        return []
    freq_cache = [(n, _token_freq(n.content)) for n in notes]
    merged_flags = [False] * len(freq_cache)
    out: List[Note] = []
    for i, (n_i, f_i) in enumerate(freq_cache):
        if merged_flags[i]:
            continue
        cluster = [(i, n_i)]
        for j in range(i + 1, len(freq_cache)):
            if merged_flags[j]:
                continue
            n_j, f_j = freq_cache[j]
            if _cosine(f_i, f_j) >= threshold:
                cluster.append((j, n_j))
                merged_flags[j] = True
        # 合并 cluster: 保留 content 最长的, confidence 取 max, source_episode_ids 合并
        longest = max(cluster, key=lambda c: len(c[1].content))[1]
        max_conf = max(c[1].confidence for c in cluster)
        merged_sources: List[str] = []
        for _, n in cluster:
            merged_sources.extend(n.source_episode_ids)
        merged_tags: List[str] = []
        seen_tag: Set[str] = set()
        for _, n in cluster:
            for t in n.tags:
                if t not in seen_tag:
                    seen_tag.add(t)
                    merged_tags.append(t)
        out.append(Note(
            id=longest.id,
            timestamp=time.time(),
            content=longest.content,
            source_episode_ids=merged_sources,
            confidence=min(1.0, max_conf + 0.05 * (len(cluster) - 1)),
            tags=merged_tags,
        ))
    return out


# ----- 3. update_confidence: 反馈驱动更新 -----

def update_confidence(note: Note, feedback: bool,
                      up_step: float = 0.05,
                      down_step: float = 0.10) -> Note:
    """Feedback-driven confidence update.

    feedback=True (主人确认/同意) → +up_step
    feedback=False (主人否认/纠正) → -down_step (步长更大, 主 17:43 实事求是)

    Ponytail: 返回新 Note (不修改原对象, 触发 GC 时机可控).
    """
    delta = up_step if feedback else -down_step
    new_conf = max(0.0, min(1.0, note.confidence + delta))
    return Note(
        id=note.id,
        timestamp=time.time(),
        content=note.content,
        source_episode_ids=list(note.source_episode_ids),
        confidence=new_conf,
        tags=list(note.tags),
    )


# ----- 4. 工具: 按 content 去重 -----

def dedupe_by_content(notes: List[Note]) -> List[Note]:
    """完全相同 content 只保留置信度最高. (Ponytail: O(n) hash)."""
    by_content: dict = {}
    for n in notes:
        if n.content not in by_content or n.confidence > by_content[n.content].confidence:
            by_content[n.content] = n
    return list(by_content.values())