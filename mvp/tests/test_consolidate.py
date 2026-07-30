"""R13 MVP Phase 1.2 — test_consolidate.py (6+ tests, 提取层 + 合并 + 遗忘).

Ponytail ceiling: 纯函数 + IdentityCard 临时 path. 不污染 Phase 1.1 测试.
"""
from __future__ import annotations

import time
from pathlib import Path

import pytest

from mvp.memory.store import Store, Episode, Note
from mvp.memory.consolidate import (
    extract_notes, merge_similar_notes, update_confidence,
    dedupe_by_content, _cosine, _tokenize,
)
from mvp.memory.forget import (
    forget_low_confidence_notes,
    forget_old_episodes,
    forget_by_salience,
    forget_episodes,
    DEFAULT_NOTE_THRESHOLD,
)
from mvp.identity import card as idcard


# ----- 1. extract_notes 启发式 -----

def test_extract_notes_basic(tmp_path: Path):
    """5 episode → ≥3 note (命中 IdentityCard 关键词)."""
    store = Store(tmp_path / "m.db")
    sid = store.start_session("s1")
    store.append_episode("user", "主人是地方的", sid)
    store.append_episode("user", "我老家养老问题", sid)
    store.append_episode("user", "研究生在读", sid)
    store.append_episode("user", "AgentMemory 是自研方向", sid)
    store.append_episode("user", "少数民族语翻译测试场", sid)
    eps = store.list_episodes(session_id=sid, limit=10)
    card = idcard.IdentityCard()
    notes = extract_notes(eps, card)
    assert len(notes) >= 3
    # 所有 note 都应至少 source_episode_ids 长度=1
    for n in notes:
        assert len(n.source_episode_ids) >= 1
        assert 0.0 < n.confidence <= 0.9
    store.close()


def test_extract_notes_empty():
    """空 episodes → [] (Ponytail: 边界)."""
    notes = extract_notes([], idcard.IdentityCard())
    assert notes == []


def test_extract_notes_no_overlap_skipped():
    """无 IdentityCard 关键词 + 无第一人称 → 不提炼."""
    eps = [Episode(id="e1", timestamp=time.time(), role="user",
                   content="今天天气真好",
                   session_id="s1", salience=1.0)]
    notes = extract_notes(eps, idcard.IdentityCard())
    assert notes == []


# ----- 2. merge_similar_notes -----

def test_merge_similar_notes(tmp_path: Path):
    """2 高相似 note → 1 merged."""
    n1 = Note(id="n1", timestamp=time.time(),
              content="主人是",
              source_episode_ids=["e1"], confidence=0.5)
    n2 = Note(id="n2", timestamp=time.time(),
              content="主人是, 老家人",
              source_episode_ids=["e2"], confidence=0.6)
    n3 = Note(id="n3", timestamp=time.time(),
              content="研究生在读",
              source_episode_ids=["e3"], confidence=0.7)
    merged = merge_similar_notes([n1, n2, n3], threshold=0.7)
    # n1 + n2 cosine 应 > 0.7, 合并 → 1; n3 不合并 → 1; 共 2
    assert len(merged) == 2
    # merged 保留 longer content
    merged_pair = [m for m in merged if "甘肃" in m.content][0]
    assert "老家人" in merged_pair.content  # longer content
    # confidence 累加 (max + 0.05 * (cluster-1))
    assert merged_pair.confidence == pytest.approx(0.65, abs=1e-6)


def test_cosine_zero_when_empty():
    """空 Counter → 0.0 (Ponytail: 边界保护)."""
    from collections import Counter
    assert _cosine(Counter(), Counter("abc")) == 0.0
    assert _cosine(Counter("abc"), Counter()) == 0.0


def test_dedupe_by_content():
    """完全相同 content 保留置信度最高."""
    n1 = Note(id="a", timestamp=1.0, content="X", confidence=0.5)
    n2 = Note(id="b", timestamp=1.0, content="X", confidence=0.7)
    n3 = Note(id="c", timestamp=1.0, content="Y", confidence=0.6)
    out = dedupe_by_content([n1, n2, n3])
    assert len(out) == 2
    contents = {n.content: n.confidence for n in out}
    assert contents["X"] == 0.7
    assert contents["Y"] == 0.6


# ----- 3. update_confidence 反馈驱动 -----

def test_update_confidence_positive():
    n = Note(id="x", timestamp=1.0, content="t", confidence=0.5)
    out = update_confidence(n, feedback=True)
    assert out.confidence == pytest.approx(0.55, abs=1e-6)
    # 原对象未变 (Ponytail: 不可变)
    assert n.confidence == 0.5


def test_update_confidence_negative():
    n = Note(id="x", timestamp=1.0, content="t", confidence=0.5)
    out = update_confidence(n, feedback=False)
    assert out.confidence == pytest.approx(0.40, abs=1e-6)


def test_update_confidence_clamp():
    """feedback=True 不会 > 1.0."""
    n = Note(id="x", timestamp=1.0, content="t", confidence=0.99)
    out = update_confidence(n, feedback=True)
    assert out.confidence <= 1.0
    # feedback=False 不会 < 0.0
    n2 = Note(id="x", timestamp=1.0, content="t", confidence=0.05)
    out2 = update_confidence(n2, feedback=False)
    assert out2.confidence >= 0.0


# ----- 4. forget_low_confidence_notes -----

def test_forget_low_confidence_notes():
    """confidence < 0.2 → 删除."""
    notes = [
        Note(id="1", timestamp=1.0, content="a", confidence=0.5),
        Note(id="2", timestamp=1.0, content="b", confidence=0.1),
        Note(id="3", timestamp=1.0, content="c", confidence=0.05),
        Note(id="4", timestamp=1.0, content="d", confidence=0.9),
    ]
    out = forget_low_confidence_notes(notes, threshold=DEFAULT_NOTE_THRESHOLD)
    assert len(out) == 2
    ids = {n.id for n in out}
    assert ids == {"1", "4"}


# ----- 5. forget_old_episodes_rolling_window -----

def test_forget_old_episodes_rolling_window():
    """210 episodes → 保留最近 200."""
    now = time.time()
    eps = [Episode(id=f"e{i}", timestamp=now - (210 - i),
                   role="user", content=f"ep-{i}",
                   session_id="s1", salience=1.0) for i in range(210)]
    out = forget_old_episodes(eps, max_count=200)
    assert len(out) == 200
    # sorted DESC: out[0] 是最新 (ep-209, i=209, ts=now-1), out[-1] 是最旧保留的 (ep-10, i=10)
    assert out[0].content == "ep-209"
    assert out[-1].content == "ep-10"


def test_forget_by_salience():
    """Salience decay 驱动的遗忘."""
    now = 10000.0
    # tau=1000, ts=0 (10000s 前) → decay = 1/(1+10) ≈ 0.091 < 0.05? 不, 0.091 > 0.05
    # 改 tau=100 → 1/(1+100) ≈ 0.0099 < 0.05 → drop
    eps = [
        Episode(id="new", timestamp=now, role="u", content="new",
                session_id="s", salience=1.0),
        Episode(id="old", timestamp=now - 10000, role="u", content="old",
                session_id="s", salience=1.0),
    ]
    out = forget_by_salience(eps, tau=100.0, cutoff=0.05, now=now)
    assert len(out) == 1
    assert out[0].id == "new"


def test_forget_episodes_combined():
    """综合: salience + rolling window."""
    now = 10000.0
    # 造 250 条: 前 100 条 timestamp=0 (会因 salience drop), 后 150 条 timestamp=now
    eps = []
    for i in range(100):
        eps.append(Episode(id=f"old{i}", timestamp=now - 10000,
                           role="u", content=f"old{i}",
                           session_id="s", salience=1.0))
    for i in range(150):
        eps.append(Episode(id=f"new{i}", timestamp=now,
                           role="u", content=f"new{i}",
                           session_id="s", salience=1.0))
    out = forget_episodes(eps, max_count=120, tau=100.0,
                          salience_cutoff=0.05, now=now)
    # old 100 全 drop, new 150 保留前 120
    assert len(out) == 120
    assert all(e.id.startswith("new") for e in out)


# ----- 6. IdentityCard.consolidate -----

def test_identity_card_consolidate(tmp_path: Path, monkeypatch):
    """从 Note 更新 IdentityCard (Phase 1.3 主入口).

    Ponytail: 用英文 note 验证 (中文单字被设计过滤, 避免噪音).
    """
    monkeypatch.setattr(idcard, "DEFAULT_CARD", tmp_path / "card.json")
    card = idcard.load()
    initial_bg = list(card.owner_background)
    notes = [
        Note(id="n1", timestamp=1.0,
             content="research on reinforcement learning and alignment",
             confidence=0.8, source_episode_ids=["e1"]),
        Note(id="n2", timestamp=1.0,
             content="alignment safety interpretability research",
             confidence=0.7, source_episode_ids=["e2"]),
        Note(id="n3", timestamp=1.0,
             content="noise",
             confidence=0.3, source_episode_ids=["e3"]),  # 低于 min_confidence
    ]
    card.consolidate(notes, min_freq=1, min_confidence=0.5)
    new_bg = list(card.owner_background)
    # 多字符 token "research" freq=2, "alignment" freq=2, "learning"/"safety"/"interpretability" freq=1
    # min_freq=1 → 至少 5 个新 token 入 bg
    assert len(new_bg) > len(initial_bg)
    # consolidation log 应记录
    assert any(e.get("key") == "consolidate.added" for e in card.evolution_log)


def test_consolidate_idempotent(tmp_path: Path, monkeypatch):
    """重复 consolidate 不引入循环 (主 17:43 实事求是: 不刷 KPI)."""
    monkeypatch.setattr(idcard, "DEFAULT_CARD", tmp_path / "card.json")
    card = idcard.load()
    notes = [
        Note(id="n1", timestamp=1.0, content="research alignment safety",
             confidence=0.8, source_episode_ids=["e1"]),
    ]
    bg_before = list(card.owner_background)
    card.consolidate(notes, min_freq=1, min_confidence=0.5)
    bg_after_first = list(card.owner_background)
    # 第二次 consolidate 同样的 notes → 不重复添加
    card.consolidate(notes, min_freq=1, min_confidence=0.5)
    bg_after_second = list(card.owner_background)
    assert bg_after_first == bg_after_second


def test_consolidate_empty_noop():
    """空 notes → noop (Ponytail: 边界)."""
    card = idcard.IdentityCard()
    bg = list(card.owner_background)
    card.consolidate([])
    assert card.owner_background == bg