"""Tests for V1090 — Real Write-Ahead Log (主 23:44 干到底 + 主 17:43 实事求是).

测试覆盖 (≥30 真测试):
  1. WalEntry dataclass + checksum + verify + JSONL round trip
  2. WriteAheadLog 基本 append / replay / verify / stats
  3. 真 fsync 行为验证 (fsync_calls 计数 + 文件落盘)
  4. 真损坏容错 (手动写损坏行 → replay 跳过 + stats 累计)
  5. 真 append-only 语义 (无 truncate API)
  6. 真 rotate 行为
  7. 持久化恢复 (新实例从 path 重建)
  8. 并发 append (threading)
  9. atomic_write_jsonl helper
 10. read_only_wal_replay helper
 11. CLI subcommand smoke
 12. 哲学守门 sanity refs
 13. ASI subscore + version

不假装守门 (主 17:58+20:46):
  - 不声称 "byte-exact replay" — 只声称 sha256 校验通过的条目被恢复
  - 不声称 "absolute durability" — fsync 成功 ≠ 永不丢
"""
from __future__ import annotations

import hashlib
import json
import os
import shutil
import sys
import tempfile
import threading
import time
from pathlib import Path
from typing import List

import pytest

ROOT = Path(__file__).resolve().parents[1]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from apeireth.v1090_memory_wal import (  # noqa: E402
    V1090_VERSION,
    DEFAULT_FSYNC,
    DEFAULT_MAX_BYTES,
    SCHEMA_VERSION,
    WalEntry,
    WalStats,
    WriteAheadLog,
    atomic_write_jsonl,
    read_only_wal_replay,
    v1090_subscore,
    _cli,
)


# ============================================================================
# 共享 helper
# ============================================================================


@pytest.fixture
def tmp_wal_path(tmp_path: Path) -> Path:
    return tmp_path / "wal.jsonl"


@pytest.fixture
def wal_in_mem() -> WriteAheadLog:
    return WriteAheadLog(path=None, fsync=False)


@pytest.fixture
def wal_on_disk(tmp_wal_path: Path) -> WriteAheadLog:
    return WriteAheadLog(path=tmp_wal_path, fsync=False)


# ============================================================================
# 1. WalEntry dataclass — 8 tests
# ============================================================================


def test_wal_entry_basic_construction():
    """WalEntry 基本构造 + 字段."""
    e = WalEntry(sequence=1, ts=1.0, op="tag_set", payload={"k": "v"})
    assert e.sequence == 1
    assert e.ts == 1.0
    assert e.op == "tag_set"
    assert e.payload == {"k": "v"}
    assert e.schema_version == SCHEMA_VERSION
    assert e.checksum == ""  # 默认空


def test_wal_entry_compute_checksum_deterministic():
    """compute_checksum 确定性: 同输入必同输出."""
    e1 = WalEntry(sequence=1, ts=1.0, op="x", payload={"a": 1})
    e2 = WalEntry(sequence=1, ts=1.0, op="x", payload={"a": 1})
    assert e1.compute_checksum() == e2.compute_checksum()
    # 不同 payload → 不同 checksum
    e3 = WalEntry(sequence=1, ts=1.0, op="x", payload={"a": 2})
    assert e3.compute_checksum() != e1.compute_checksum()


def test_wal_entry_compute_checksum_format():
    """checksum 是 sha256 hex (64 chars)."""
    e = WalEntry(sequence=1, ts=1.0, op="x")
    chk = e.compute_checksum()
    assert len(chk) == 64
    assert all(c in "0123456789abcdef" for c in chk)


def test_wal_entry_verify_true():
    """verify 在 checksum 匹配时返回 True."""
    e = WalEntry(sequence=1, ts=1.0, op="x", payload={"a": 1})
    object.__setattr__(e, "checksum", e.compute_checksum())
    assert e.verify() is True


def test_wal_entry_verify_false_on_tamper():
    """verify 在 payload 被改后返回 False."""
    e = WalEntry(sequence=1, ts=1.0, op="x", payload={"a": 1})
    object.__setattr__(e, "checksum", e.compute_checksum())
    # 模拟篡改 payload (用 object.__setattr__ 因为 frozen)
    object.__setattr__(e, "payload", {"a": 999})
    assert e.verify() is False


def test_wal_entry_frozen():
    """frozen dataclass: 直接赋值会抛 FrozenInstanceError."""
    e = WalEntry(sequence=1, ts=1.0, op="x")
    with pytest.raises(Exception):
        e.sequence = 999  # type: ignore[misc]


def test_wal_entry_jsonl_round_trip():
    """JSONL round trip: to_jsonl → from_jsonl → 等价."""
    e = WalEntry(sequence=42, ts=123.456, op="tag_set",
                 payload={"key": "value", "n": 7}, checksum="placeholder")
    object.__setattr__(e, "checksum", e.compute_checksum())
    line = e.to_jsonl()
    e2 = WalEntry.from_jsonl(line)
    assert e2.sequence == e.sequence
    assert e2.ts == e.ts
    assert e2.op == e.op
    assert e2.payload == e.payload
    assert e2.checksum == e.checksum
    assert e2.verify() is True


def test_wal_entry_from_jsonl_bad_json():
    """from_jsonl 损坏行 → ValueError."""
    with pytest.raises(ValueError):
        WalEntry.from_jsonl("{not valid json")


def test_wal_entry_from_jsonl_missing_field():
    """from_jsonl 缺字段 → ValueError."""
    with pytest.raises(Exception):
        WalEntry.from_jsonl(json.dumps({"sequence": 1, "ts": 1.0}))


# ============================================================================
# 2. WriteAheadLog 基本 append / replay / verify — 10 tests
# ============================================================================


def test_append_increments_sequence(wal_in_mem: WriteAheadLog):
    """append 返回单调 sequence."""
    s1 = wal_in_mem.append("tag_set", {"k": "v1"})
    s2 = wal_in_mem.append("anchor_link", {"a": 1})
    s3 = wal_in_mem.append("phase_emit", {"phase": "demo"})
    assert s1 == 1 and s2 == 2 and s3 == 3
    assert s1 < s2 < s3


def test_replay_returns_all(wal_in_mem: WriteAheadLog):
    """replay 返回所有 entry (按 append 顺序)."""
    wal_in_mem.append("tag_set", {"k": "v1"})
    wal_in_mem.append("anchor_link", {"a": 1})
    entries = wal_in_mem.replay()
    assert len(entries) == 2
    assert entries[0].op == "tag_set"
    assert entries[1].op == "anchor_link"


def test_replay_empty(wal_in_mem: WriteAheadLog):
    """空 WAL replay → 空 list."""
    assert wal_in_mem.replay() == []


def test_iter_replay_is_iterator(wal_in_mem: WriteAheadLog):
    """iter_replay 是 Iterator (生成器)."""
    wal_in_mem.append("a", {})
    it = wal_in_mem.iter_replay()
    assert iter(it) is it
    assert next(it).op == "a"


def test_verify_returns_valid_count(wal_in_mem: WriteAheadLog):
    """verify 返回 (valid, corrupt) 计数; 无损坏全 valid."""
    wal_in_mem.append("a", {})
    wal_in_mem.append("b", {})
    v, c = wal_in_mem.verify()
    assert v == 2 and c == 0


def test_stats_basic_fields(wal_in_mem: WriteAheadLog):
    """stats 包含 entries_total / entries_valid / entries_corrupt / fsync_calls."""
    wal_in_mem.append("a", {})
    s = wal_in_mem.stats()
    assert s["version"] == V1090_VERSION
    assert s["entries_total"] == 1
    assert s["entries_valid"] == 1
    assert s["entries_corrupt"] == 0
    assert s["last_sequence"] == 1
    assert s["fsync_calls"] == 0  # 无 path
    assert s["fsync_skipped"] == 0


def test_stats_corruption_rate(wal_in_mem: WriteAheadLog):
    """stats corruption_rate 公式: corrupt / total."""
    wal_in_mem.append("a", {})
    s = wal_in_mem.stats()
    assert s["corruption_rate"] == 0.0


def test_append_with_empty_payload(wal_in_mem: WriteAheadLog):
    """append 可省略 payload (=None)."""
    seq = wal_in_mem.append("tag_set")
    assert seq == 1
    e = wal_in_mem.replay()[0]
    assert e.payload == {}


def test_append_with_default_payload(wal_in_mem: WriteAheadLog):
    """append(payload={}) 与 append() 等价."""
    seq1 = wal_in_mem.append("x", {})
    seq2 = wal_in_mem.append("x")
    assert seq1 == 1 and seq2 == 2


def test_replay_preserves_payload_types(wal_in_mem: WriteAheadLog):
    """replay 后 payload 类型保留 (int / str / list / dict)."""
    wal_in_mem.append("complex", {
        "int": 42, "str": "hello", "list": [1, 2, 3],
        "nested": {"k": "v"}, "bool": True, "null": None,
    })
    e = wal_in_mem.replay()[0]
    assert e.payload["int"] == 42
    assert e.payload["list"] == [1, 2, 3]
    assert e.payload["nested"] == {"k": "v"}
    assert e.payload["bool"] is True
    assert e.payload["null"] is None


# ============================================================================
# 3. 真 fsync 行为 — 4 tests
# ============================================================================


def test_fsync_calls_counter_increments(tmp_wal_path: Path):
    """fsync=True 时每次 append fsync_calls 递增."""
    wal = WriteAheadLog(tmp_wal_path, fsync=True)
    wal.append("a", {})
    wal.append("b", {})
    wal.append("c", {})
    s = wal.stats()
    assert s["fsync_calls"] >= 3
    assert s["fsync_skipped"] == 0


def test_fsync_skipped_when_disabled(tmp_wal_path: Path):
    """fsync=False 时 fsync_skipped 递增; fsync_calls = 0."""
    wal = WriteAheadLog(tmp_wal_path, fsync=False)
    for i in range(5):
        wal.append("a", {"i": i})
    s = wal.stats()
    assert s["fsync_calls"] == 0
    assert s["fsync_skipped"] == 5


def test_fsync_persists_to_disk(tmp_wal_path: Path):
    """fsync 后文件确实包含 JSONL 内容."""
    wal = WriteAheadLog(tmp_wal_path, fsync=True)
    wal.append("tag_set", {"key": "value"})
    assert tmp_wal_path.exists()
    content = tmp_wal_path.read_text(encoding="utf-8")
    assert "tag_set" in content
    assert "key" in content


def test_fsync_default_is_on(tmp_wal_path: Path):
    """默认 fsync=True (V3 守门: 不假装 fsync 是默认)."""
    wal = WriteAheadLog(tmp_wal_path)
    wal.append("a", {})
    s = wal.stats()
    assert s["fsync_calls"] >= 1


# ============================================================================
# 4. 真损坏容错 — 5 tests
# ============================================================================


def test_corrupt_line_skipped_on_recover(tmp_wal_path: Path):
    """手动写损坏行 → recover 后 replay 跳过 + stats.corrupt 累计."""
    # 先写一条合法的
    wal = WriteAheadLog(tmp_wal_path, fsync=False)
    wal.append("tag_set", {"k": "v"})
    # 手动追加一行损坏的 JSON
    with tmp_wal_path.open("a", encoding="utf-8") as fh:
        fh.write("{this is not valid json\n")
    # 重启: 新实例从 path 恢复
    wal2 = WriteAheadLog(tmp_wal_path, fsync=False)
    entries = wal2.replay()
    assert len(entries) == 1
    assert wal2.stats()["entries_corrupt"] == 1


def test_corrupt_checksum_skipped(tmp_wal_path: Path):
    """sha256 校验失败 → 跳过 (但 JSON 合法)."""
    wal = WriteAheadLog(tmp_wal_path, fsync=False)
    wal.append("tag_set", {"k": "v"})
    # 篡改最后一行 checksum
    lines = tmp_wal_path.read_text(encoding="utf-8").strip().split("\n")
    obj = json.loads(lines[-1])
    obj["checksum"] = "0" * 64
    lines[-1] = json.dumps(obj)
    tmp_wal_path.write_text("\n".join(lines) + "\n", encoding="utf-8")
    # 重启
    wal2 = WriteAheadLog(tmp_wal_path, fsync=False)
    assert wal2.replay() == []
    assert wal2.stats()["entries_corrupt"] == 1


def test_empty_lines_skipped(tmp_wal_path: Path):
    """空行 / 空白行被 skip (不计入 corrupt)."""
    wal = WriteAheadLog(tmp_wal_path, fsync=False)
    wal.append("a", {})
    with tmp_wal_path.open("a", encoding="utf-8") as fh:
        fh.write("\n\n   \n\n")
    wal2 = WriteAheadLog(tmp_wal_path, fsync=False)
    assert len(wal2.replay()) == 1
    assert wal2.stats()["entries_corrupt"] == 0


def test_recover_no_file(tmp_path: Path):
    """path 不存在 → recover 静默成功, _seq=0."""
    p = tmp_path / "nope.jsonl"
    wal = WriteAheadLog(p, fsync=False)
    assert wal._seq == 0
    assert wal.replay() == []


def test_recover_after_partial_corruption_keeps_valid(tmp_wal_path: Path):
    """中间一行损坏, 前后的合法行仍可恢复."""
    wal = WriteAheadLog(tmp_wal_path, fsync=False)
    wal.append("a", {"i": 1})
    with tmp_wal_path.open("a", encoding="utf-8") as fh:
        fh.write("garbage\n")
    wal2 = WriteAheadLog(tmp_wal_path, fsync=False)
    # WriteAheadLog 在 __init__ 末尾又调用一次 _recover, 但 _seq 已被设
    # _seq 应 = 1 (最高有效 sequence)
    assert wal2._seq == 1
    entries = wal2.replay()
    assert len(entries) == 1
    assert wal2.stats()["entries_corrupt"] == 1


# ============================================================================
# 5. 真 append-only — 2 tests
# ============================================================================


def test_no_truncate_api():
    """WriteAheadLog 公开 API 不暴露 truncate / overwrite."""
    public_methods = [m for m in dir(WriteAheadLog) if not m.startswith("_")]
    forbidden = {"truncate", "overwrite", "delete_entry", "remove_entry", "edit_entry"}
    for f in forbidden:
        assert f not in public_methods, f"append-only violated: {f} is exposed"


def test_append_only_after_corruption(tmp_wal_path: Path):
    """append-only: 即使前一行损坏, 新 append 在文件末尾追加."""
    wal = WriteAheadLog(tmp_wal_path, fsync=False)
    wal.append("a", {"i": 1})
    with tmp_wal_path.open("a", encoding="utf-8") as fh:
        fh.write("garbage\n")
    wal.append("b", {"i": 2})
    # 文件末尾应包含 b
    last_line = tmp_wal_path.read_text(encoding="utf-8").strip().split("\n")[-1]
    obj = json.loads(last_line)
    assert obj["op"] == "b"


# ============================================================================
# 6. 真 rotate — 4 tests
# ============================================================================


def test_rotate_creates_backup(tmp_wal_path: Path):
    """rotate 把当前 WAL 改名为 .bak."""
    wal = WriteAheadLog(tmp_wal_path, fsync=False)
    wal.append("a", {})
    wal.append("b", {})
    backup = wal.rotate()
    assert backup is not None
    assert backup.exists()
    assert backup.name.endswith(".bak")
    assert tmp_wal_path.exists()  # 新空文件已建立


def test_rotate_increments_counter(tmp_wal_path: Path):
    """rotate 增加 rotates 计数."""
    wal = WriteAheadLog(tmp_wal_path, fsync=False)
    wal.append("a", {})
    wal.rotate()
    wal.append("b", {})
    assert wal.stats()["rotates"] == 1


def test_rotate_no_path_returns_none(wal_in_mem: WriteAheadLog):
    """无 path 时 rotate 返回 None."""
    assert wal_in_mem.rotate() is None


def test_rotate_no_file_returns_none(tmp_wal_path: Path):
    """path 不存在时 rotate 返回 None."""
    wal = WriteAheadLog(tmp_wal_path, fsync=False)
    assert wal.rotate() is None


# ============================================================================
# 7. 持久化恢复 — 4 tests
# ============================================================================


def test_persist_and_reload_preserves_entries(tmp_wal_path: Path):
    """append → 新实例从 path 重建 → 等价."""
    wal1 = WriteAheadLog(tmp_wal_path, fsync=False)
    for i in range(10):
        wal1.append("op_x", {"i": i, "name": f"item-{i}"})
    wal2 = WriteAheadLog(tmp_wal_path, fsync=False)
    entries = wal2.replay()
    assert len(entries) == 10
    assert wal2._seq == 10


def test_recover_preserves_payload_integrity(tmp_wal_path: Path):
    """reload 后 payload 字节级一致."""
    payload = {"key": "val", "list": [1, 2, 3], "nested": {"a": "b"}}
    wal1 = WriteAheadLog(tmp_wal_path, fsync=False)
    wal1.append("op_y", payload)
    wal2 = WriteAheadLog(tmp_wal_path, fsync=False)
    e = wal2.replay()[0]
    assert e.payload == payload


def test_recover_preserves_checksum(tmp_wal_path: Path):
    """reload 后 entry.verify() 仍 True."""
    wal1 = WriteAheadLog(tmp_wal_path, fsync=False)
    wal1.append("op_z", {"x": 1})
    wal2 = WriteAheadLog(tmp_wal_path, fsync=False)
    assert wal2.replay()[0].verify() is True


def test_recover_resets_stats_corrupt_counter(tmp_wal_path: Path):
    """新实例加载时 _stats.entries_corrupt 被准确重置."""
    wal1 = WriteAheadLog(tmp_wal_path, fsync=False)
    wal1.append("a", {})
    with tmp_wal_path.open("a", encoding="utf-8") as fh:
        fh.write("corrupt\n")
    wal2 = WriteAheadLog(tmp_wal_path, fsync=False)
    assert wal2.stats()["entries_corrupt"] == 1
    assert wal2.stats()["entries_valid"] == 1


# ============================================================================
# 8. 并发 append — 2 tests
# ============================================================================


def test_concurrent_append_thread_safe(tmp_wal_path: Path):
    """多线程 append 不丢不串 (sequence 单调, 内容完整)."""
    wal = WriteAheadLog(tmp_wal_path, fsync=False)
    N_THREADS = 4
    N_PER_THREAD = 25

    def worker(tid: int) -> None:
        for i in range(N_PER_THREAD):
            wal.append("op_concurrent", {"tid": tid, "i": i})

    threads = [threading.Thread(target=worker, args=(t,)) for t in range(N_THREADS)]
    for t in threads:
        t.start()
    for t in threads:
        t.join()

    entries = wal.replay()
    assert len(entries) == N_THREADS * N_PER_THREAD
    seqs = [e.sequence for e in entries]
    assert seqs == sorted(set(seqs))  # 单调
    assert max(seqs) == N_THREADS * N_PER_THREAD


def test_concurrent_replay_safe(tmp_wal_path: Path):
    """append 与 replay 并发不抛错 (snapshot 语义)."""
    wal = WriteAheadLog(tmp_wal_path, fsync=False)
    for i in range(20):
        wal.append("op_a", {"i": i})

    errors: List[Exception] = []

    def replay_worker() -> None:
        try:
            for _ in range(50):
                list(wal.replay())
        except Exception as e:
            errors.append(e)

    def append_worker() -> None:
        try:
            for i in range(50):
                wal.append("op_b", {"i": i})
        except Exception as e:
            errors.append(e)

    t1 = threading.Thread(target=replay_worker)
    t2 = threading.Thread(target=append_worker)
    t1.start(); t2.start()
    t1.join(); t2.join()
    assert errors == []


# ============================================================================
# 9. atomic_write_jsonl helper — 3 tests
# ============================================================================


def test_atomic_write_creates_file(tmp_path: Path):
    """atomic_write_jsonl 写入文件."""
    p = tmp_path / "out.jsonl"
    n = atomic_write_jsonl(p, ['{"a":1}', '{"b":2}'])
    assert p.exists()
    assert n > 0
    lines = p.read_text(encoding="utf-8").strip().split("\n")
    assert lines == ['{"a":1}', '{"b":2}']


def test_atomic_write_overwrites(tmp_path: Path):
    """atomic_write_jsonl 覆盖旧内容."""
    p = tmp_path / "out.jsonl"
    p.write_text("stale\n", encoding="utf-8")
    atomic_write_jsonl(p, ['{"new":1}'])
    content = p.read_text(encoding="utf-8")
    assert "stale" not in content
    assert '{"new":1}' in content


def test_atomic_write_terminates_with_newline(tmp_path: Path):
    """atomic_write_jsonl 确保末尾有 \\n."""
    p = tmp_path / "out.jsonl"
    atomic_write_jsonl(p, ['{"a":1}'])
    assert p.read_text(encoding="utf-8").endswith("\n")


# ============================================================================
# 10. read_only_wal_replay helper — 2 tests
# ============================================================================


def test_read_only_replay_returns_valid(tmp_wal_path: Path):
    """read_only_wal_replay 返回 (entries, corrupt_count)."""
    wal = WriteAheadLog(tmp_wal_path, fsync=False)
    wal.append("a", {"i": 1})
    wal.append("b", {"i": 2})
    entries, corrupt = read_only_wal_replay(tmp_wal_path)
    assert len(entries) == 2
    assert corrupt == 0


def test_read_only_replay_detects_corruption(tmp_wal_path: Path):
    """read_only_wal_replay 正确报告损坏数."""
    wal = WriteAheadLog(tmp_wal_path, fsync=False)
    wal.append("a", {})
    with tmp_wal_path.open("a", encoding="utf-8") as fh:
        fh.write("garbage\n")
    entries, corrupt = read_only_wal_replay(tmp_wal_path)
    assert len(entries) == 1
    assert corrupt == 1


# ============================================================================
# 11. flush + context manager — 3 tests
# ============================================================================


def test_flush_noop_without_path(wal_in_mem: WriteAheadLog):
    """无 path 时 flush 不抛错."""
    wal_in_mem.flush()  # 不应抛


def test_flush_after_append(tmp_wal_path: Path):
    """flush 不抛错, fsync_calls 可能递增."""
    wal = WriteAheadLog(tmp_wal_path, fsync=True)
    wal.append("a", {})
    pre = wal.stats()["fsync_calls"]
    wal.flush()
    # flush 在 append 过的文件末尾再次 flush + fsync
    assert wal.stats()["fsync_calls"] >= pre


def test_context_manager(tmp_wal_path: Path):
    """with 语句退出时自动 flush."""
    with WriteAheadLog(tmp_wal_path, fsync=True) as wal:
        wal.append("ctx", {"x": 1})
    assert tmp_wal_path.exists()
    entries = WriteAheadLog(tmp_wal_path, fsync=False).replay()
    assert len(entries) == 1


# ============================================================================
# 12. CLI smoke — 3 tests
# ============================================================================


def test_cli_append_replay_stats(tmp_path: Path, capsys: pytest.CaptureFixture[str]):
    """CLI: append → replay → stats 三步可走."""
    p = tmp_path / "cli.jsonl"
    # append
    rc = _cli(["append", str(p), "--op", "demo_op", "--payload", '{"k":1}'])
    assert rc == 0
    out = json.loads(capsys.readouterr().out.strip())
    assert out["sequence"] == 1
    # replay
    rc = _cli(["replay", str(p)])
    assert rc == 0
    out = json.loads(capsys.readouterr().out.strip())
    assert out["count"] == 1
    # stats
    rc = _cli(["stats", str(p)])
    assert rc == 0


def test_cli_verify_detects_corruption(tmp_path: Path, capsys: pytest.CaptureFixture[str]):
    """CLI verify 检测到损坏时返非 0 退出码."""
    p = tmp_path / "v.jsonl"
    wal = WriteAheadLog(p, fsync=False)
    wal.append("ok", {})
    with p.open("a", encoding="utf-8") as fh:
        fh.write("garbage\n")
    rc = _cli(["verify", str(p)])
    out = json.loads(capsys.readouterr().out.strip())
    assert out["corrupt"] == 1
    assert rc == 1


def test_cli_self_check(capsys: pytest.CaptureFixture[str]):
    """CLI self-check 输出 subscore."""
    rc = _cli(["self-check"])
    assert rc == 0
    out = json.loads(capsys.readouterr().out.strip())
    assert out["v1090_subscore"] == 1.0
    assert out["version"] == V1090_VERSION


# ============================================================================
# 13. 哲学守门 + ASI subscore + version — 3 tests
# ============================================================================


def test_v1090_subscore_is_one():
    """V1090 self-measured subscore = 1.0 (5/5 真实现)."""
    assert v1090_subscore() == 1.0


def test_version_constant_format():
    """V1090_VERSION 形如 'X.Y.Z'."""
    parts = V1090_VERSION.split(".")
    assert len(parts) == 3
    assert all(p.isdigit() for p in parts)


def test_no_lorem_in_module_source():
    """主 23:44: 模块源不包含 lorem ipsum / TODO 占位."""
    src_path = ROOT / "apeireth" / "v1090_memory_wal.py"
    src = src_path.read_text(encoding="utf-8")
    forbidden = ["lorem ipsum", "TODO", "FIXME", "XXX"]
    for f in forbidden:
        assert f.lower() not in src.lower(), f"forbidden placeholder found: {f}"


# ============================================================================
# 14. WalStats dataclass — 2 tests
# ============================================================================


def test_wal_stats_to_dict():
    """WalStats.to_dict 返回完整字段."""
    s = WalStats(entries_total=5, entries_valid=4, entries_corrupt=1,
                 bytes_on_disk=100, last_sequence=5, last_ts=1.0,
                 rotates=0, fsync_calls=5, fsync_skipped=0)
    d = s.to_dict()
    assert d["entries_total"] == 5
    assert d["entries_valid"] == 4
    assert d["entries_corrupt"] == 1
    assert d["corruption_rate"] == 0.2


def test_wal_stats_corruption_rate_zero_safe():
    """corruption_rate 在 entries_total=0 时不除零."""
    s = WalStats()
    d = s.to_dict()
    assert d["corruption_rate"] == 0.0


if __name__ == "__main__":
    pytest.main([__file__, "-q", "--tb=short"])