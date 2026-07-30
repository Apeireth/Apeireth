"""R13 MVP Phase 1.2 — 遗忘策略.

Ponytail ceiling: 纯函数. 借鉴 DeltaMemory 2024 + 主人哲学"不刷 KPI".

主 17:43 实事求是: 遗忘阈值是经验值, 可调. 不假装"绝对正确".
"""
from __future__ import annotations

import time
from typing import List, Optional

from mvp.memory.store import Episode, Note
from mvp.memory.retrieve import _decay, TAU_SECONDS

DEFAULT_NOTE_THRESHOLD = 0.2     # confidence < 0.2 遗忘
DEFAULT_EPISODE_MAX = 200         # 滚动窗口
DEFAULT_SALIENCE_CUTOFF = 0.05   # salience < 0.05 遗忘


# ----- Note 遗忘 -----

def forget_low_confidence_notes(
    notes: List[Note],
    threshold: float = DEFAULT_NOTE_THRESHOLD,
) -> List[Note]:
    """遗忘 confidence < threshold 的 Note.

    主 17:43 实事求是: 不假设 0.2 是绝对, 是经验值. Phase 1.3 可按
    主人实测调整.
    """
    return [n for n in notes if n.confidence >= threshold]


# ----- Episode 遗忘 -----

def forget_old_episodes(
    episodes: List[Episode],
    max_count: int = DEFAULT_EPISODE_MAX,
) -> List[Episode]:
    """保留最新 max_count 条 Episode (rolling window).

    Ponytail: 按 timestamp DESC 取前 max_count.
    """
    if len(episodes) <= max_count:
        return episodes
    sorted_eps = sorted(episodes, key=lambda e: e.timestamp, reverse=True)
    return sorted_eps[:max_count]


def forget_by_salience(
    episodes: List[Episode],
    tau: float = TAU_SECONDS,
    cutoff: float = DEFAULT_SALIENCE_CUTOFF,
    now: Optional[float] = None,
) -> List[Episode]:
    """Salience decay 驱动的遗忘. salience < cutoff 丢弃.

    借鉴 DeltaMemory 2024 (Lin et al.) 1/(1+Δt/τ).
    """
    now = now if now is not None else time.time()
    return [e for e in episodes if _decay(e.timestamp, now=now, tau=tau) >= cutoff]


# ----- 综合: forget_all -----

def forget_episodes(
    episodes: List[Episode],
    max_count: int = DEFAULT_EPISODE_MAX,
    tau: float = TAU_SECONDS,
    salience_cutoff: float = DEFAULT_SALIENCE_CUTOFF,
    use_salience: bool = True,
    now: Optional[float] = None,
) -> List[Episode]:
    """综合遗忘: 先按 salience, 再按 rolling window.

    Ponytail: 分两阶段, salience 后再做 rolling 防止顺序依赖.
    """
    out = episodes
    if use_salience:
        out = forget_by_salience(out, tau=tau, cutoff=salience_cutoff, now=now)
    out = forget_old_episodes(out, max_count=max_count)
    return out