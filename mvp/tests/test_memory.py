"""R13 MVP — test_memory.py (8+ tests, 跨 session 验证).

Ponytail ceiling: tmp_path pytest fixture, ephemeral SQLite, no global
state leak between tests. Phase 1 测试面:
- Episode append-only + rolling 200
- Note 合并 / 遗忘
- FTS5 BM25 retrieval
- Salience decay
- Time window filter
- IdentityCard evolution
- Cross-session persistence (关 → 重开 → 上下文完整)
"""
from __future__ import annotations

import time
from pathlib import Path

import pytest

from mvp.memory.store import Store, MAX_EPISODES, MIN_CONFIDENCE
from mvp.memory.retrieve import (
    retrieve, retrieve_notes, time_window_filter, _decay, TAU_SECONDS,
)
from mvp.identity import card as idcard


# ----- Episode -----

def test_episode_append(tmp_path: Path):
    s = Store(tmp_path / "m.db")
    sid = s.start_session("s1")
    ep = s.append_episode("user", "今天聊少数民族语翻译", sid)
    assert ep.id
    assert ep.role == "user"
    assert ep.content == "今天聊少数民族语翻译"
    assert ep.session_id == "s1"
    assert ep.salience == 1.0
    s.close()


def test_episode_rolling_window(tmp_path: Path):
    s = Store(tmp_path / "m.db")
    sid = s.start_session("s2")
    for i in range(MAX_EPISODES + 10):
        s.append_episode("user", f"ep-{i}", sid)
    recent = s.list_episodes(session_id=sid, limit=1000)
    assert len(recent) == MAX_EPISODES
    # 最新保留
    assert recent[0].content == f"ep-{MAX_EPISODES + 9}"
    s.close()


def test_session_id_autocreation(tmp_path: Path):
    s = Store(tmp_path / "m.db")
    sid = s.start_session()
    s.append_episode("user", "test", sid)
    assert s.last_session() == sid
    s.close()


# ----- Note -----

def test_note_consolidation(tmp_path: Path):
    s = Store(tmp_path / "m.db")
    n1 = s.add_note("主人关心地方养老", confidence=0.5)
    n2 = s.merge_note(n1.id, "主人关心地方养老 (主人亲述 + 多次确认)",
                      bump_confidence=0.1)
    assert n2 is not None
    assert n2.confidence == pytest.approx(0.6, abs=1e-6)
    assert "亲述" in n2.content
    s.close()


def test_note_forget_low_confidence(tmp_path: Path):
    s = Store(tmp_path / "m.db")
    s.add_note("低置信 1", confidence=0.05)
    s.add_note("低置信 2", confidence=0.04)
    s.add_note("高置信", confidence=0.9)
    forgotten = s.forget_low_confidence(threshold=MIN_CONFIDENCE)
    assert forgotten == 2
    remaining = s.list_notes()
    assert len(remaining) == 1
    assert remaining[0].content == "高置信"
    s.close()


# ----- Retrieval -----

def test_fts5_bm25_retrieve(tmp_path: Path):
    s = Store(tmp_path / "m.db")
    sid = s.start_session()
    s.append_episode("user", "地方养老是长期议题", sid)
    s.append_episode("user", "少数民族语翻译测试场", sid)
    s.append_episode("user", "研究生在读", sid)
    hits = retrieve(s, "养老", top_k=3, session_id=sid, use_decay=False)
    assert len(hits) >= 1
    assert "养老" in hits[0].episode.content
    s.close()


def test_salience_decay():
    # now=1000, tau=100 → decay at ts=900 should be 1/(1+1)=0.5
    sal = _decay(timestamp=900.0, now=1000.0, tau=100.0)
    assert sal == pytest.approx(0.5, abs=1e-6)
    # ts=now → 1.0
    assert _decay(timestamp=1000.0, now=1000.0, tau=100.0) == pytest.approx(1.0)
    # ts=0 (far past), 1/(1+10)=0.0909; ts=-10000 → 0.0099 < 0.01
    sal_far = _decay(timestamp=0.0, now=1000.0, tau=100.0)
    assert sal_far == pytest.approx(1.0 / 11.0, abs=1e-6)
    sal_very_far = _decay(timestamp=-10000.0, now=1000.0, tau=100.0)
    assert sal_very_far < 0.01


def test_time_window_filter(tmp_path: Path):
    s = Store(tmp_path / "m.db")
    sid = s.start_session()
    s.append_episode("user", "t1", sid)
    time.sleep(0.05)
    cutoff = time.time()
    time.sleep(0.05)
    s.append_episode("user", "t2", sid)
    s.append_episode("user", "t3", sid)
    eps = s.list_episodes(session_id=sid, limit=10)
    filtered = time_window_filter(eps, since=cutoff)
    contents = [e.content for e in filtered]
    assert "t1" not in contents
    assert "t2" in contents
    assert "t3" in contents
    s.close()


# ----- Identity -----

def test_identity_card_evolution(tmp_path: Path, monkeypatch):
    # 临时 home → tmp_path
    monkeypatch.setattr(idcard, "DEFAULT_CARD", tmp_path / "card.json")
    c = idcard.load()
    assert "" in c.owner_background
    c.evolve("owner_background.该少数民族文化深度研究者", value=True)
    # arbitrary custom key (无点号前缀 → 落入 custom dict 原 key)
    c.evolve("foo", value="bar")
    idcard.save(c)
    c2 = idcard.load()
    assert "该少数民族文化深度研究者" in c2.owner_background
    assert c2.custom["foo"] == "bar"
    assert len(c2.evolution_log) == 2


# ----- Cross-session persistence (主 17:43 实事求是 + 主 23:09 干到底) -----

def test_cross_session_persistence(tmp_path: Path):
    """关 → 重开 → 上下文完整."""
    db = tmp_path / "persist.db"

    # session 1: 写入
    s1 = Store(db_path=db)
    sid = s1.start_session("long-session")
    s1.append_episode("user", "主人是地方的", sid)
    s1.append_episode("agent", "收到, 已记录", sid)
    s1.add_note("主人关心养老问题", confidence=0.8)
    s1.close()

    # session 2: 重开 (新 Store 实例 = 模拟进程重启)
    s2 = Store(db_path=db)
    sid_back = s2.last_session()
    assert sid_back == "long-session"
    eps = s2.list_episodes(session_id="long-session", limit=10)
    contents = [e.content for e in eps]
    assert "主人是地方的" in contents
    assert "收到, 已记录" in contents
    notes = s2.list_notes()
    assert any("养老问题" in n.content for n in notes)
    s2.close()


def test_retrieve_notes_long_half_life(tmp_path: Path):
    """Note 半衰期 (7天) > Episode (1天), 主 17:43 实事求是."""
    s = Store(tmp_path / "m.db")
    n = s.add_note("少数民族语翻译测试场", confidence=0.7)
    # 1 天前 timestamp
    one_day_ago = time.time() - TAU_SECONDS  # 1 day
    s._conn.execute("UPDATE notes SET timestamp = ? WHERE id = ?",
                    (one_day_ago, n.id))
    s._conn.commit()
    # Episode 半衰期 1 天 → 0.5; Note 半衰期 7 天 → ~0.875
    note_hits = retrieve_notes(s, "少数民族语", top_k=1, tau=TAU_SECONDS * 7)
    assert len(note_hits) >= 1
    s.close()