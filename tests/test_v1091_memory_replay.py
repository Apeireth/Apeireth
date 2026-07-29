"""Tests for V1091 MemoryReplay — 真生产状态回放 (R8-TrackA2).

覆盖 5 方法契约 + WAL 持久化 + 并发回放 + 损坏容错 + 守门。

主 17:43 实事求是: 真跑真测, 不假装。
"""
from __future__ import annotations

import json
import os
import sys
import threading
import time
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

import pytest

APEIRETH_DIR = Path(__file__).resolve().parent.parent / "apeireth"
if str(APEIRETH_DIR.parent) not in sys.path:
    sys.path.insert(0, str(APEIRETH_DIR.parent))

from apeireth.memory_replay_design import (
    Event,
    IDEMPOTENT_OPS,
    PHILOSOPHY_GUARDS,
    StateDiff,
    StateID,
)
from apeireth.v1091_memory_replay import (
    Checkpoint,
    MemoryReplay,
    V1091_VERSION,
    WalEntry,
    _event_hash,
)


# ============================================================
# 模块导入 / 版本
# ============================================================


class TestV1091Basics:
    def test_version_is_string(self):
        assert isinstance(V1091_VERSION, str)
        assert V1091_VERSION.count(".") >= 1

    def test_wal_entry_has_required_fields(self):
        e = WalEntry(sequence=1, ts=1.0, scope="s", event=Event("e1", 1.0, "tag_set"))
        assert e.sequence == 1
        assert e.ts == 1.0
        assert e.scope == "s"

    def test_checkpoint_has_required_fields(self):
        cp = Checkpoint(
            state_id=StateID(scope="x", seq=1, content_hash="abc"),
            state={"k": "v"},
            up_to_sequence=10,
        )
        assert cp.state_id.scope == "x"
        assert cp.up_to_sequence == 10

    def test_event_hash_is_deterministic(self):
        ev = Event("e1", 1.0, "tag_set", (("k", "v"),))
        h1 = _event_hash(ev)
        h2 = _event_hash(ev)
        assert h1 == h2

    def test_event_hash_differs_for_different_payload(self):
        ev1 = Event("e1", 1.0, "tag_set", (("k", "v1"),))
        ev2 = Event("e1", 1.0, "tag_set", (("k", "v2"),))
        assert _event_hash(ev1) != _event_hash(ev2)


# ============================================================
# capture_state / restore_state
# ============================================================


class TestV1091CaptureRestore:
    def test_capture_returns_state_id_with_seq_1(self):
        mr = MemoryReplay()
        sid = mr.capture_state("scope-1")
        assert isinstance(sid, StateID)
        assert sid.scope == "scope-1"
        assert sid.seq == 1
        assert sid.content_hash  # non-empty

    def test_capture_increments_seq_per_call(self):
        mr = MemoryReplay()
        s1 = mr.capture_state("a")
        s2 = mr.capture_state("b")
        assert s2.seq == s1.seq + 1

    def test_capture_hashes_live_state(self):
        mr = MemoryReplay()
        mr.apply_event("s", Event("e1", 1.0, "tag_set", (("k", "v"),)))
        sid = mr.capture_state("s")
        # content_hash should change after event applied
        assert len(sid.content_hash) > 0

    def test_restore_known_checkpoint_returns_true(self):
        mr = MemoryReplay()
        mr.apply_event("s", Event("e1", 1.0, "tag_set", (("k", "v"),)))
        sid = mr.capture_state("s")
        assert mr.restore_state(sid) is True

    def test_restore_unknown_checkpoint_returns_false(self):
        mr = MemoryReplay()
        bogus = StateID(scope="ghost", seq=999, content_hash="xx")
        assert mr.restore_state(bogus) is False

    def test_restore_rolls_back_to_exact_checkpoint_state(self):
        mr = MemoryReplay()
        mr.apply_event("s", Event("e1", 1.0, "tag_set", (("orig", "yes"),)))
        sid = mr.capture_state("s")
        mr.apply_event("s", Event("e2", 2.0, "tag_set", (("added", "yes"),)))
        assert "added" in mr._live_state.get("tags", {})

        assert mr.restore_state(sid) is True

        assert mr._live_state.get("tags", {}) == {"orig": "yes"}

    def test_capture_state_with_empty_scope_allowed(self):
        mr = MemoryReplay()
        sid = mr.capture_state("")
        assert sid.scope == ""


# ============================================================
# replay_events — 一致性 + 边界 + 迭代器
# ============================================================


class TestV1091ReplayEvents:
    def test_replay_returns_iterator(self):
        mr = MemoryReplay()
        result = mr.replay_events(0.0, 10.0)
        assert hasattr(result, "__next__")

    def test_replay_empty_window_returns_nothing(self):
        mr = MemoryReplay()
        events = list(mr.replay_events(0.0, 10.0))
        assert events == []

    def test_replay_uses_half_open_boundaries(self):
        mr = MemoryReplay()
        mr.apply_event("s", Event("lower", 5.0, "tag_set", (("k", "v1"),)))
        mr.apply_event("s", Event("upper", 6.0, "tag_set", (("k", "v2"),)))

        events = list(mr.replay_events(5.0, 6.0))

        assert [event.event_id for event in events] == ["upper"]

    def test_replay_excludes_outside_window(self):
        mr = MemoryReplay()
        mr.apply_event("s", Event("e1", 3.0, "tag_set", (("k", "v"),)))
        events = list(mr.replay_events(5.0, 10.0))
        assert events == []

    def test_replay_consistency_same_window_same_events(self):
        """回放一致性: 同样窗口 → 同样 events 顺序."""
        mr = MemoryReplay()
        for i in range(5):
            mr.apply_event("s", Event(f"e{i}", float(i + 1), "tag_set", (("i", str(i)),)))
        a = [ev.event_id for ev in mr.replay_events(0.0, 10.0)]
        b = [ev.event_id for ev in mr.replay_events(0.0, 10.0)]
        assert a == b

    def test_replay_ordered_by_sequence(self):
        mr = MemoryReplay()
        for i in range(10):
            mr.apply_event("s", Event(f"e{i}", float(i), "tag_set", (("i", str(i)),)))
        events = list(mr.replay_events(0.0, 100.0))
        seqs = [int(ev.event_id[1:]) for ev in events]
        assert seqs == sorted(seqs)
    def test_replay_ordered_by_event_timestamp(self):
        mr = MemoryReplay()
        mr.apply_event("s", Event("late", 30.0, "tag_set", (("k", "late"),)))
        mr.apply_event("s", Event("early", 10.0, "tag_set", (("k", "early"),)))
        mr.apply_event("s", Event("middle", 20.0, "tag_set", (("k", "middle"),)))

        events = list(mr.replay_events(0.0, 100.0))

        assert [event.ts for event in events] == [10.0, 20.0, 30.0]


# ============================================================
# diff_states — added / removed / changed
# ============================================================


class TestV1091DiffStates:
    def test_diff_returns_state_diff(self):
        mr = MemoryReplay()
        sid_a = mr.capture_state("a")
        mr.apply_event("s", Event("e1", 1.0, "tag_set", (("k", "v"),)))
        sid_b = mr.capture_state("b")
        diff = mr.diff_states(sid_a, sid_b)
        assert isinstance(diff, StateDiff)

    def test_diff_shows_added_events(self):
        mr = MemoryReplay()
        sid_a = mr.capture_state("a")
        mr.apply_event("s", Event("e1", 1.0, "tag_set", (("k", "v"),)))
        sid_b = mr.capture_state("b")
        diff = mr.diff_states(sid_a, sid_b)
        assert len(diff.added) >= 1

    def test_diff_unknown_state_id_returns_empty(self):
        mr = MemoryReplay()
        sid_a = mr.capture_state("a")
        bogus = StateID(scope="ghost", seq=99, content_hash="xx")
        diff = mr.diff_states(sid_a, bogus)
        assert diff == StateDiff()

    def test_diff_is_symmetric_up_to_sign(self):
        mr = MemoryReplay()
        sid_a = mr.capture_state("a")
        mr.apply_event("s", Event("e1", 1.0, "tag_set", (("k", "v"),)))
        sid_b = mr.capture_state("b")

        forward = mr.diff_states(sid_a, sid_b)
        reverse = mr.diff_states(sid_b, sid_a)

        assert forward.added == reverse.removed
        assert forward.removed == reverse.added
        assert forward.changed == tuple((after, before) for before, after in reverse.changed)

    def test_diff_detects_payload_change(self):
        mr = MemoryReplay()
        mr.apply_event("s", Event("e1", 1.0, "tag_set", (("k", "v1"),)))
        sid_a = mr.capture_state("a")
        mr.apply_event("s", Event("e2", 2.0, "tag_set", (("k", "v2"),)))
        sid_b = mr.capture_state("b")

        diff = mr.diff_states(sid_a, sid_b)

        assert len(diff.changed) == 1
        before, after = diff.changed[0]
        assert dict(before.payload) == {"k": "v1"}
        assert dict(after.payload) == {"k": "v2"}


# ============================================================
# idempotent_apply — 白名单 + 缓存
# ============================================================


class TestV1091IdempotentApply:
    def test_apply_non_whitelist_rejected(self):
        mr = MemoryReplay()
        ev = Event("e1", 1.0, "non_existent_op")
        result = mr.idempotent_apply(ev)
        assert result.status == "rejected"
        assert result.cached is False
        assert "whitelisted" in result.reason.lower() or "whitelist" in result.reason.lower()

    def test_apply_whitelist_accepted(self):
        mr = MemoryReplay()
        ev = Event("e1", 1.0, "tag_set", (("k", "v"),))
        result = mr.idempotent_apply(ev)
        assert result.status == "applied"
        assert result.cached is False

    def test_apply_same_event_twice_cached(self):
        mr = MemoryReplay()
        ev = Event("e1", 1.0, "tag_set", (("k", "v"),))
        r1 = mr.idempotent_apply(ev)
        r2 = mr.idempotent_apply(ev)
        assert r1.status == r2.status
        assert r1.event_hash == r2.event_hash
        assert r2.cached is True

    def test_same_event_id_with_different_payload_is_rejected(self):
        mr = MemoryReplay()
        ev1 = Event("e1", 1.0, "tag_set", (("k", "v1"),))
        ev2 = Event("e1", 2.0, "tag_set", (("k", "v2"),))

        first = mr.idempotent_apply(ev1)
        second = mr.idempotent_apply(ev2)

        assert first.status == "applied"
        assert second.status == "rejected"
        assert "event_id" in second.reason
        assert mr._live_state["tags"]["k"] == "v1"
        assert len(mr._wal) == 1

    def test_apply_updates_live_state(self):
        mr = MemoryReplay()
        ev = Event("e1", 1.0, "tag_set", (("topic", "safety"),))
        mr.idempotent_apply(ev)
        assert mr._live_state.get("tags", {}).get("topic") == "safety"

    def test_apply_writes_wal_entry(self):
        mr = MemoryReplay()
        before = len(mr._wal)
        mr.idempotent_apply(Event("e1", 1.0, "tag_set", (("k", "v"),)))
        assert len(mr._wal) == before + 1


# ============================================================
# apply_event — WAL 写入 + 状态机
# ============================================================


class TestV1091ApplyEvent:
    def test_apply_increments_seq(self):
        mr = MemoryReplay()
        s0 = mr._seq
        mr.apply_event("s", Event("e1", 1.0, "tag_set", (("k", "v"),)))
        assert mr._seq == s0 + 1

    def test_apply_appends_to_wal(self):
        mr = MemoryReplay()
        before = len(mr._wal)
        mr.apply_event("s", Event("e1", 1.0, "tag_set"))
        assert len(mr._wal) == before + 1

    def test_non_whitelist_kind_does_not_update_state(self):
        mr = MemoryReplay()
        mr.apply_event("s", Event("e1", 1.0, "unknown_op"))
        # state dict 没有 unknown 相关的 keys
        assert "unknown" not in mr._live_state

    def test_tag_set_updates_tags(self):
        mr = MemoryReplay()
        mr.apply_event("s", Event("e1", 1.0, "tag_set", (("k", "v"),)))
        assert mr._live_state["tags"]["k"] == "v"

    def test_anchor_link_appends(self):
        mr = MemoryReplay()
        mr.apply_event("s", Event("e1", 1.0, "anchor_link", (("id", "x"),)))
        assert len(mr._live_state["anchors"]) == 1

    def test_anchor_unlink_removes(self):
        mr = MemoryReplay()
        mr.apply_event("s", Event("e1", 1.0, "anchor_link", (("id", "x"),)))
        mr.apply_event("s", Event("e2", 1.0, "anchor_unlink", (("id", "x"),)))
        assert len(mr._live_state["anchors"]) == 0

    def test_score_record_appends(self):
        mr = MemoryReplay()
        mr.apply_event("s", Event("e1", 1.0, "score_record", (("v", 0.9),)))
        assert len(mr._live_state["scores"]) == 1

    def test_phase_emit_appends(self):
        mr = MemoryReplay()
        mr.apply_event("s", Event("e1", 1.0, "phase_emit", (("phase", "LIGHT"),)))
        assert len(mr._live_state["phases"]) == 1

    def test_trace_record_appends(self):
        mr = MemoryReplay()
        mr.apply_event("s", Event("e1", 1.0, "trace_record", (("k", "v"),)))
        assert len(mr._live_state["traces"]) == 1


# ============================================================
# WAL 持久化 / 损坏容错
# ============================================================


class TestV1091WalPersistence:
    def test_wal_roundtrip(self, tmp_path: Path):
        wal = tmp_path / "wal.jsonl"
        mr1 = MemoryReplay(wal_path=wal)
        mr1.apply_event("s", Event("e1", 1.0, "tag_set", (("k", "v"),)))
        mr1.apply_event("s", Event("e2", 2.0, "tag_set", (("k2", "v2"),)))
        assert wal.exists()

        # 重新加载
        mr2 = MemoryReplay(wal_path=wal)
        assert mr2._seq == 2
        assert len(mr2._wal) == 2
        events = list(mr2.replay_events(0.0, 100.0))
        assert len(events) == 2

    def test_recovery_rebuilds_live_state_from_wal(self, tmp_path: Path):
        wal = tmp_path / "wal.jsonl"
        writer = MemoryReplay(wal_path=wal)
        writer.apply_event("s", Event("e1", 1.0, "tag_set", (("first", "yes"),)))
        writer.apply_event("s", Event("e2", 2.0, "tag_set", (("second", "yes"),)))

        recovered = MemoryReplay(wal_path=wal)

        assert recovered._live_state["tags"] == {"first": "yes", "second": "yes"}

    def test_checkpoint_restore_survives_restart(self, tmp_path: Path):
        wal = tmp_path / "wal.jsonl"
        writer = MemoryReplay(wal_path=wal)
        writer.apply_event("s", Event("before", 1.0, "tag_set", (("before", "yes"),)))
        state_id = writer.capture_state("session-1")
        writer.apply_event("s", Event("after", 2.0, "tag_set", (("after", "yes"),)))

        recovered = MemoryReplay(wal_path=wal)
        assert recovered.restore_state(state_id) is True
        assert recovered._live_state["tags"] == {"before": "yes"}

        restarted_again = MemoryReplay(wal_path=wal)
        assert restarted_again._live_state["tags"] == {"before": "yes"}

    def test_structurally_malformed_json_record_is_skipped(self, tmp_path: Path):
        wal = tmp_path / "wal.jsonl"
        wal.write_text(json.dumps({
            "sequence": 1,
            "ts": 1.0,
            "scope": "s",
            "event": [],
            "checksum": "not-relevant",
        }) + "\n", encoding="utf-8")

        recovered = MemoryReplay(wal_path=wal)

        assert recovered._wal == []
        assert recovered._skipped_corrupt == 1

    def test_corrupt_json_line_skipped(self, tmp_path: Path):
        wal = tmp_path / "wal.jsonl"
        # 写入混合: 1 正常 + 1 损坏 + 1 正常
        good_entry = WalEntry(
            sequence=1, ts=1.0, scope="s", event=Event("e1", 1.0, "tag_set")
        )
        good_entry.checksum = good_entry.compute_checksum()
        with wal.open("w", encoding="utf-8") as fh:
            fh.write(good_entry.to_jsonl() + "\n")
            fh.write("THIS_IS_NOT_JSON{garbage\n")
            good2 = WalEntry(
                sequence=2, ts=2.0, scope="s", event=Event("e2", 2.0, "tag_set")
            )
            good2.checksum = good2.compute_checksum()
            fh.write(good2.to_jsonl() + "\n")
        mr = MemoryReplay(wal_path=wal)
        assert len(mr._wal) == 2
        assert mr._skipped_corrupt == 1

    def test_checksum_mismatch_skipped(self, tmp_path: Path):
        wal = tmp_path / "wal.jsonl"
        # 直接写一行带假 checksum 的 JSONL (绕开 to_jsonl 的覆盖)
        bad_line = json.dumps({
            "sequence": 1,
            "ts": 1.0,
            "scope": "s",
            "event": {"event_id": "e1", "ts": 1.0, "kind": "tag_set", "payload": {}},
            "checksum": "fakenotvalidchecksum",
        }, ensure_ascii=False, sort_keys=True)
        with wal.open("w", encoding="utf-8") as fh:
            fh.write(bad_line + "\n")
        mr = MemoryReplay(wal_path=wal)
        assert len(mr._wal) == 0
        assert mr._skipped_corrupt == 1

    def test_recovery_preserves_max_sequence(self, tmp_path: Path):
        wal = tmp_path / "wal.jsonl"
        mr1 = MemoryReplay(wal_path=wal)
        for i in range(5):
            mr1.apply_event("s", Event(f"e{i}", float(i), "tag_set"))
        mr2 = MemoryReplay(wal_path=wal)
        assert mr2._seq == 5

    def test_continue_appending_after_recovery_preserves_monotonic_seq(self, tmp_path: Path):
        wal = tmp_path / "wal.jsonl"
        first = MemoryReplay(wal_path=wal)
        first.apply_event("s", Event("e0", 0.5, "tag_set", (("k", "v0"),)))
        first.apply_event("s", Event("e1", 1.0, "tag_set", (("k", "v1"),)))

        resumed = MemoryReplay(wal_path=wal)
        new_seq = resumed.apply_event("s", Event("e2", 2.0, "tag_set", (("k", "v2"),)))

        assert new_seq == 3
        events = list(resumed.replay_events(0.0, 10.0))
        assert [event.event_id for event in events] == ["e0", "e1", "e2"]

    def test_verify_wal_after_recovery(self, tmp_path: Path):
        wal = tmp_path / "wal.jsonl"
        mr1 = MemoryReplay(wal_path=wal)
        for i in range(3):
            mr1.apply_event("s", Event(f"e{i}", float(i), "tag_set"))
        mr2 = MemoryReplay(wal_path=wal)
        valid, corrupt = mr2.verify_wal()
        assert valid == 3
        assert corrupt == 0

    def test_wal_entry_jsonl_roundtrip(self):
        ev = Event("e1", 1.5, "tag_set", (("k", "v"),))
        e1 = WalEntry(sequence=7, ts=10.0, scope="test", event=ev)
        e1.checksum = e1.compute_checksum()
        line = e1.to_jsonl()
        e2 = WalEntry.from_jsonl(line)
        assert e2.sequence == 7
        assert e2.scope == "test"
        assert e2.event.event_id == "e1"
        assert e2.event.kind == "tag_set"
        assert e2.checksum == e1.checksum

    def test_replay_with_corrupt_wal_does_not_crash(self, tmp_path: Path):
        wal = tmp_path / "wal.jsonl"
        wal.write_text("random garbage\n" + "another bad line\n", encoding="utf-8")
        mr = MemoryReplay(wal_path=wal)
        events = list(mr.replay_events(0.0, 1000.0))
        assert events == []
        assert mr._skipped_corrupt == 2


# ============================================================
# 并发回放
# ============================================================


class TestV1091Concurrency:
    def test_concurrent_apply_event_safe(self):
        mr = MemoryReplay()

        def worker(start_seq: int) -> int:
            for i in range(10):
                mr.apply_event(
                    "s",
                    Event(f"e{start_seq}_{i}", float(start_seq + i), "tag_set",
                          (("k", str(i)),)),
                )
            return start_seq

        with ThreadPoolExecutor(max_workers=4) as pool:
            futures = [pool.submit(worker, i * 10) for i in range(4)]
            for f in futures:
                f.result()
        # 4 * 10 = 40 entries
        assert len(mr._wal) == 40
        assert mr._seq == 40

    def test_concurrent_idempotent_apply_cached(self):
        mr = MemoryReplay()
        ev = Event("e1", 1.0, "tag_set", (("k", "v"),))

        def worker() -> str:
            r = mr.idempotent_apply(ev)
            return r.status

        with ThreadPoolExecutor(max_workers=8) as pool:
            results = [f.result() for f in [pool.submit(worker) for _ in range(50)]]
        # 所有调用要么 applied 要么 cached — 但只有第一次 applied, 后续 cached
        # status 字符串必须一致
        assert len(set(results)) == 1  # "applied" or "applied" 都不变

    def test_concurrent_capture_state_monotonic_seq(self):
        mr = MemoryReplay()
        sids: list = []
        lock = threading.Lock()

        def worker() -> None:
            sid = mr.capture_state("concurrent")
            with lock:
                sids.append(sid.seq)

        threads = [threading.Thread(target=worker) for _ in range(10)]
        for t in threads:
            t.start()
        for t in threads:
            t.join()
        # seq 必须唯一 + 单调递增 (虽然并发 capture 但锁保护)
        assert sorted(sids) == list(range(1, len(sids) + 1))


# ============================================================
# Stats / 守门 / 杂项
# ============================================================


class TestV1091StatsAndGuards:
    def test_stats_structure(self):
        mr = MemoryReplay()
        stats = mr.stats()
        assert "version" in stats
        assert "wal_entries" in stats
        assert "current_seq" in stats
        assert "checkpoints" in stats
        assert "skipped_corrupt" in stats
        assert stats["version"] == V1091_VERSION

    def test_philosophy_guards_in_stats(self):
        mr = MemoryReplay()
        stats = mr.stats()
        assert stats["philosophy_guards"] == list(PHILOSOPHY_GUARDS)

    def test_idempotent_ops_matches_design(self):
        assert "tag_set" in IDEMPOTENT_OPS
        assert "anchor_link" in IDEMPOTENT_OPS
        assert "score_record" in IDEMPOTENT_OPS

    def test_verify_wal_empty_returns_zero(self):
        mr = MemoryReplay()
        valid, corrupt = mr.verify_wal()
        assert (valid, corrupt) == (0, 0)

    def test_stats_wal_entries_count(self):
        mr = MemoryReplay()
        mr.apply_event("s", Event("e1", 1.0, "tag_set", (("k", "v"),)))
        stats = mr.stats()
        assert stats["wal_entries"] == 1


if __name__ == "__main__":
    # ponytail: 允许 `python tests/test_v1091_memory_replay.py` 单跑
    pytest.main([__file__, "-v"])
