"""memory + memory_store mem0 borrow regression tests.

主 9:41 round-19 真源码深读 (commit 87c1189) 推荐:
  - mem0 hash dedup + observation_date (低风险, 改 memory_store.py + memory.py)
  - 不做: entity linking (重复 linkage.py), BM25 hybrid (复杂度 ↑ 收益 ↓)

借鉴自 mem0 (主 9:41 round-19):
  - observation_date field (Episode + Note) - temporal grounding
  - Note fingerprint hash dedup (Episode 已有, Note 借鉴)
  - Anti-hallucination: 严格区分 observation_date vs ts (event 实际发生时间 vs 录入时间)

本测试锁住:
  1. Episode + Note dataclass 有 observation_date field (默认 = time.time())
  2. SqliteMemoryStore v0.3.0 migration 加 observation_date column (idempotent)
  3. append_episode 存储 + 查询 observation_date
  4. add_note 用 fingerprint hash dedup (同 topic + claim 视为重复)
  5. episodes_by_observation_date 查询方法 (temporal grounding)
  6. 向后兼容: 旧 v0.2.0 db 也能升级 (ALTER TABLE)
  7. V2 哲学守门: 不假装 Phenomenal
"""
from __future__ import annotations

import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent.parent))

import pytest

from apeireth.memory import Episode, Note
from apeireth.memory_store import (
    SqliteMemoryStore,
    integrity_hash_for_note,
    migrate_from_memory_store,
)


# === 1. Episode + Note observation_date field 测试 ===

class TestObservationDateField:
    """借鉴 mem0 (主 9:41 round-19): Episode + Note 都有 observation_date field."""

    def test_episode_has_observation_date(self):
        ep = Episode(eid="e1", actor="master", content="hello")
        assert hasattr(ep, "observation_date")
        # None sentinel — append_episode 用 ts 作为 fallback
        assert ep.observation_date is None

    def test_episode_observation_date_defaults_to_none(self):
        """Default: observation_date=None (向后兼容, append_episode fallback 到 ts)."""
        ep = Episode(eid="e1", actor="master", content="hello")
        assert ep.observation_date is None

    def test_episode_observation_date_can_be_set(self):
        """mem0: observation_date 是关键 (历史回溯必须 anchor 到录入时间)."""
        ts = 1700000000.0  # 2023-11-14
        ep = Episode(eid="e1", actor="master", content="test",
                     ts=ts, observation_date=ts + 86400)  # 录入时间比事件时间晚 1 天
        assert ep.ts == ts
        assert ep.observation_date == ts + 86400

    def test_note_has_observation_date(self):
        n = Note(nid="n1", topic="t", claim="c")
        assert hasattr(n, "observation_date")
        assert n.observation_date is None  # None sentinel

    def test_note_observation_date_can_be_set(self):
        n = Note(nid="n1", topic="t", claim="c",
                 created_at=1700000000.0,
                 observation_date=1700100000.0)
        assert n.created_at == 1700000000.0
        assert n.observation_date == 1700100000.0

    def test_episode_to_dict_includes_observation_date(self):
        ep = Episode(eid="e1", actor="master", content="hello")
        d = ep.to_dict()
        assert "observation_date" in d

    def test_note_to_dict_includes_observation_date(self):
        n = Note(nid="n1", topic="t", claim="c")
        d = n.to_dict()
        assert "observation_date" in d


# === 2. SqliteMemoryStore v0.3.0 migration 测试 ===

class TestSchemaV030Migration:
    """v0.3.0: 升级加 observation_date column + Note fingerprint (idempotent)."""

    def test_schema_version_is_030(self, tmp_path):
        store = SqliteMemoryStore(tmp_path / "memory.db")
        cur = store._conn.execute("SELECT v FROM memory_meta WHERE k='schema_version'")
        version = cur.fetchone()[0]
        assert version == "0.3.0"
        store.close()

    def test_episodes_table_has_observation_date_column(self, tmp_path):
        store = SqliteMemoryStore(tmp_path / "memory.db")
        cur = store._conn.execute("PRAGMA table_info(episodes)")
        cols = {row[1] for row in cur.fetchall()}
        assert "observation_date" in cols
        store.close()

    def test_notes_table_has_observation_date_column(self, tmp_path):
        store = SqliteMemoryStore(tmp_path / "memory.db")
        cur = store._conn.execute("PRAGMA table_info(notes)")
        cols = {row[1] for row in cur.fetchall()}
        assert "observation_date" in cols
        store.close()

    def test_notes_table_has_fingerprint_column(self, tmp_path):
        store = SqliteMemoryStore(tmp_path / "memory.db")
        cur = store._conn.execute("PRAGMA table_info(notes)")
        cols = {row[1] for row in cur.fetchall()}
        assert "fingerprint" in cols
        store.close()

    def test_migration_is_idempotent(self, tmp_path):
        """多次 init 同一个 db, 不应该报错."""
        store1 = SqliteMemoryStore(tmp_path / "memory.db")
        store1.close()
        # 第二次 open, 应该 work (migration 是 idempotent)
        store2 = SqliteMemoryStore(tmp_path / "memory.db")
        cur = store2._conn.execute("PRAGMA table_info(episodes)")
        cols = {row[1] for row in cur.fetchall()}
        assert "observation_date" in cols
        store2.close()

    def test_index_on_observation_date_created(self, tmp_path):
        """observation_date 应该有 index (mem0: temporal grounding 需要高效查询)."""
        store = SqliteMemoryStore(tmp_path / "memory.db")
        cur = store._conn.execute("SELECT name FROM sqlite_master WHERE type='index'")
        indexes = {row[0] for row in cur.fetchall()}
        assert "idx_episodes_observation_date" in indexes
        store.close()


# === 3. append_episode + observation_date 测试 ===

class TestAppendEpisodeObservationDate:
    """append_episode 存储 observation_date 字段 (借鉴 mem0)."""

    def test_append_episode_stores_observation_date(self, tmp_path):
        store = SqliteMemoryStore(tmp_path / "memory.db")
        ts = 1700000000.0
        ep = Episode(eid="e1", actor="master", content="test",
                     ts=ts, observation_date=ts + 86400)
        result = store.append_episode(ep)
        assert result is True
        # verify
        episodes = store.episodes()
        assert len(episodes) == 1
        assert episodes[0].observation_date == ts + 86400
        store.close()

    def test_append_episode_default_observation_date_equals_ts(self, tmp_path):
        """默认情况下 observation_date 应该 = ts (向后兼容)."""
        store = SqliteMemoryStore(tmp_path / "memory.db")
        ts = 1700000000.0
        ep = Episode(eid="e1", actor="master", content="test", ts=ts)
        # 不显式设 observation_date
        store.append_episode(ep)
        episodes = store.episodes()
        assert episodes[0].observation_date == ts
        store.close()


# === 4. Note hash dedup (mem0 fingerprint) 测试 ===

class TestNoteHashDedup:
    """借鉴 mem0 (主 9:41 round-19): 同 topic + claim 的 Note 视为重复, 不重复入库."""

    def test_add_note_first_time_succeeds(self, tmp_path):
        store = SqliteMemoryStore(tmp_path / "memory.db")
        n = Note(nid="n1", topic="Apeireth命名", claim="无限之中将要燃起的那一点")
        result = store.add_note(n)
        assert result is True
        store.close()

    def test_add_note_duplicate_fails(self, tmp_path):
        """同 topic + claim 视为重复."""
        store = SqliteMemoryStore(tmp_path / "memory.db")
        n1 = Note(nid="n1", topic="Apeireth命名", claim="无限之中将要燃起的那一点")
        n2 = Note(nid="n2", topic="Apeireth命名", claim="无限之中将要燃起的那一点")  # 同 topic + claim
        assert store.add_note(n1) is True
        assert store.add_note(n2) is False  # dedup 拒绝
        notes = store.notes()
        assert len(notes) == 1
        store.close()

    def test_add_note_different_topic_succeeds(self, tmp_path):
        store = SqliteMemoryStore(tmp_path / "memory.db")
        n1 = Note(nid="n1", topic="Apeireth命名", claim="claim 1")
        n2 = Note(nid="n2", topic="ASI北极星", claim="claim 2")
        assert store.add_note(n1) is True
        assert store.add_note(n2) is True
        notes = store.notes()
        assert len(notes) == 2
        store.close()

    def test_add_note_different_claim_succeeds(self, tmp_path):
        store = SqliteMemoryStore(tmp_path / "memory.db")
        n1 = Note(nid="n1", topic="Apeireth命名", claim="claim 1")
        n2 = Note(nid="n2", topic="Apeireth命名", claim="claim 2")
        assert store.add_note(n1) is True
        assert store.add_note(n2) is True
        store.close()

    def test_integrity_hash_for_note_consistent(self):
        """同 topic + claim 产出相同 hash."""
        n1 = Note(nid="n1", topic="A", claim="B")
        n2 = Note(nid="n2", topic="A", claim="B")
        assert integrity_hash_for_note(n1) == integrity_hash_for_note(n2)

    def test_integrity_hash_for_note_different_on_change(self):
        n1 = Note(nid="n1", topic="A", claim="B")
        n2 = Note(nid="n1", topic="A", claim="B2")
        assert integrity_hash_for_note(n1) != integrity_hash_for_note(n2)


# === 5. episodes_by_observation_date temporal grounding 测试 ===

class TestEpisodesByObservationDate:
    """借鉴 mem0 (主 9:41 round-19): temporal grounding 用 observation_date 不用 ts."""

    def test_query_by_observation_date(self, tmp_path):
        store = SqliteMemoryStore(tmp_path / "memory.db")
        # 录入 3 个 episode, observation_date 不同
        ep1 = Episode(eid="e1", actor="master", content="昨天说过",
                      ts=1700000000, observation_date=1700000000)
        ep2 = Episode(eid="e2", actor="master", content="今天说",
                      ts=1700086400, observation_date=1700086400)
        ep3 = Episode(eid="e3", actor="master", content="明天说",
                      ts=1700172800, observation_date=1700172800)
        store.append_episode(ep1)
        store.append_episode(ep2)
        store.append_episode(ep3)

        # 查询 "昨天 + 今天" = [1700000000, 1700086401)
        results = store.episodes_by_observation_date(
            since_ts=1700000000, until_ts=1700086401
        )
        assert len(results) == 2
        # 应该包含 e1 和 e2, 不包含 e3
        eids = {r.eid for r in results}
        assert eids == {"e1", "e2"}
        store.close()

    def test_temporal_grounding_vs_ts(self, tmp_path):
        """mem0 核心: observation_date != ts 时, 查询应该 anchor 到 observation_date."""
        store = SqliteMemoryStore(tmp_path / "memory.db")
        # ts=今天, observation_date=3天前 (主人事后录入历史事件)
        ep = Episode(eid="e1", actor="master", content="历史事件",
                     ts=1700259200, observation_date=1700000000)  # ts 晚 3 天
        store.append_episode(ep)

        # 按 ts 查询 "今天" 应该找到
        results_ts = store.episodes(since_ts=1700259200)
        assert len(results_ts) == 1

        # 按 observation_date 查询 "3天前" 应该找到 (不是 ts!)
        results_obs = store.episodes_by_observation_date(since_ts=1700000000, until_ts=1700000001)
        assert len(results_obs) == 1
        assert results_obs[0].eid == "e1"
        store.close()

    def test_query_only_since(self, tmp_path):
        """只传 since_ts, 没有 until."""
        store = SqliteMemoryStore(tmp_path / "memory.db")
        for i in range(5):
            ep = Episode(eid=f"e{i}", actor="master", content=f"msg{i}",
                         ts=1700000000 + i*1000, observation_date=1700000000 + i*1000)
            store.append_episode(ep)
        # 查询 since_ts=1700002000 (从第 3 个开始)
        results = store.episodes_by_observation_date(since_ts=1700002000)
        assert len(results) == 3  # e2, e3, e4
        store.close()


# === 6. 向后兼容测试 ===

class TestBackwardCompatibility:
    """旧 v0.2.0 db 应该能平滑升级到 v0.3.0."""

    def test_old_schema_without_observation_date_upgrades(self, tmp_path):
        """模拟旧 v0.2.0 schema (没 observation_date column), 应该 ALTER TABLE 加列."""
        import sqlite3
        db_path = tmp_path / "memory.db"
        conn = sqlite3.connect(str(db_path))
        # 旧 schema (没有 observation_date)
        conn.executescript("""
            CREATE TABLE episodes (
                eid TEXT PRIMARY KEY,
                actor TEXT NOT NULL,
                content TEXT NOT NULL,
                context TEXT DEFAULT '',
                kind TEXT DEFAULT 'utterance',
                ts REAL NOT NULL,
                linked_identity_hash TEXT DEFAULT '',
                fingerprint TEXT NOT NULL
            );
            CREATE TABLE notes (
                nid TEXT PRIMARY KEY,
                topic TEXT NOT NULL,
                claim TEXT NOT NULL,
                confidence REAL DEFAULT 0.5,
                importance INTEGER DEFAULT 5,
                evidence TEXT DEFAULT '[]',
                created_at REAL NOT NULL,
                last_consolidated REAL NOT NULL,
                supersedes TEXT DEFAULT '[]'
            );
            CREATE TABLE memory_meta (k TEXT PRIMARY KEY, v TEXT NOT NULL);
            INSERT INTO memory_meta VALUES ('schema_version', '0.2.0');
        """)
        conn.commit()
        conn.close()

        # 打开 SqliteMemoryStore — 应该自动迁移
        store = SqliteMemoryStore(db_path)
        cur = store._conn.execute("PRAGMA table_info(episodes)")
        cols = {row[1] for row in cur.fetchall()}
        assert "observation_date" in cols  # migration 添加了

        # schema_version 升级到 0.3.0
        cur = store._conn.execute("SELECT v FROM memory_meta WHERE k='schema_version'")
        assert cur.fetchone()[0] == "0.3.0"
        store.close()


# === 7. V2 哲学守门测试 ===

class TestV2PhilosophyGuard:
    """V2 哲学守门 (主 22:08): 不假装 Phenomenal / 跨域是工具."""

    def test_observation_date_not_philosophical_claim(self):
        """observation_date 是技术字段, 不假装意识."""
        ep = Episode(eid="e1", actor="master", content="hello")
        # observation_date 是 metadata, 不是 consciousness claim
        assert hasattr(ep, "observation_date")
        # 不应该有假装意识字段
        forbidden = ["awareness", "qualia", "phenomenal_experience", "self_aware"]
        for f in forbidden:
            assert not hasattr(ep, f)

    def test_no_mem0_branding_in_api(self):
        """借鉴 mem0 是工具 (主 20:55 隐喻), 不是哲学来源."""
        from apeireth.memory_store import SqliteMemoryStore
        # 不暴露 mem0 branding
        forbidden_attrs = ["mem0_origin", "mem0_attribution"]
        for attr in forbidden_attrs:
            assert not hasattr(SqliteMemoryStore, attr)


if __name__ == "__main__":
    pytest.main([__file__, "-v"])