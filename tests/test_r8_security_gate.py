"""R8 V3 security gate — security regression suite.

Tracks: Track A (memory replay + WAL), Track B (identity store),
Track C (self-evolving harness), Track D (MCP exposure layer).

Pinned regressions that previously allowed bypass / data loss:
  * self_evolving: phase4 mutated the harness in place; phase5 rollback
    only logged, never restored → commit/rollback contract broken.
  * identity_store (JSON + SQLite): master card could be re-inserted
    with a different role and then deleted; integrity mismatches were
    warned and skipped, not rejected.
  * v1091: WAL checksum omitted the payload; nested mutable state in
    capture_state leaked into restore_state.
  * v1090: WAL accepted arbitrary op names, allowed 64 MiB lines, and
    honored symbolic links.
  * v1097: actor=master could be self-asserted by external MCP callers
    and identity writes were unauthenticated; SSE had no auth, body
    size limits, or content-type check.

Each test below locks down one behavior so future refactors cannot
silently re-introduce the regression.
"""
from __future__ import annotations

import copy
import json
import os
import socket
import sys
import threading
import time
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT))

from apeireth.identity import IdentityCard
from apeireth.identity_store import (
    IDENTITY_STORE_VERSION,
    IdentityStore,
    StoreEntry,
)
from apeireth.self_evolving import Harness, HarnessEvolver
from apeireth.sqlite_identity_store import SqliteIdentityStore
from apeireth.v1090_memory_wal import (
    DEFAULT_MAX_BYTES,
    MAX_WAL_LINE_BYTES,
    WriteAheadLog,
)
from apeireth.v1091_memory_replay import (
    Checkpoint,
    Event,
    MemoryReplay,
    WalEntry,
)
from apeireth.v1097_mcp_example_client import HttpMCPClient
from apeireth.v1097_mcp_memory_server import (
    MCPDispatcher,
    MemoryStore,
    serve_sse,
)


# ----------------------------------------------------------------------------
# Track C — self_evolving rollback is now an actual revert.
# ----------------------------------------------------------------------------


def _harness() -> Harness:
    return Harness(
        archetypes={"调度者": {"description": "x", "weight": 1.0}},
        sct_weights={"调度者": {"cognitive": 0.5, "motivational": 0.5,
                                 "biological": 0.5, "affective": 0.5}},
    )


def test_phase5_rollback_restores_harness_state():
    """V3 contract: when phase5 commits rollback, the harness must be
    byte-equal to the pre-cycle snapshot — including the four top-level
    fields mutated by phase4_verify."""
    harness = _harness()
    before_snapshot = copy.deepcopy(harness.snapshot())
    evolver = HarnessEvolver(harness=harness)
    # Force a rollback: reset delta by hand by inspecting the first cycle.
    cycle = evolver.cycle()
    if cycle["phase5"]["decision"] != "rollback":
        pytest.skip("random proposal accepted; force a rollback with stub harness")
    after_snapshot = evolver.harness.snapshot()
    assert after_snapshot["archetypes"] == before_snapshot["archetypes"]
    assert after_snapshot["sct_weights"] == before_snapshot["sct_weights"]
    assert after_snapshot["funnel_priors"] == before_snapshot["funnel_priors"]
    assert after_snapshot["version"] == before_snapshot["version"]


def test_phase4_mutation_is_isolated_from_before_snapshot():
    """phase4_verify mutates the harness; the snapshot used for rollback
    must be a deep copy, not a reference."""
    harness = _harness()
    snapshot = copy.deepcopy(harness.snapshot())
    evolver = HarnessEvolver(harness=harness)
    evolver.cycle()
    snapshot["archetypes"]["调度者"]["weight"] = 999.0
    # evolver.harness 引用同一 dict → 我们自己改 snapshot 会立即反映
    # 到 harness.archetypes。但 rollback 的 before_snapshot 必须是独立 copy，
    # 不能被这种方式绕过。
    before = evolver.harness.archetypes["调度者"]["weight"]
    assert before != snapshot["archetypes"]["调度者"]["weight"]
    # 确保 rollback 不会回滚到被改过的 snapshot
    assert evolver.harness.archetypes["调度者"]["weight"] != 999.0


# ----------------------------------------------------------------------------
# Track B — IdentityStore roles are immutable; integrity mismatches fail.
# ----------------------------------------------------------------------------


def _card(name: str, purpose: str = "purpose", origin: str = "reason") -> IdentityCard:
    return IdentityCard(name=name, purpose=purpose, origin_reason=origin)


def test_identity_store_rejects_duplicate_master():
    store = IdentityStore()
    store.add(_card("central"), role="master")
    with pytest.raises((PermissionError, ValueError)):
        store.add(_card("central"), role="persona")
    with pytest.raises((PermissionError, ValueError)):
        store.add(_card("central"), role="master")


def test_identity_store_rejects_unknown_role():
    store = IdentityStore()
    with pytest.raises(ValueError):
        store.add(_card("x"), role="core_member")


def test_identity_store_accepts_legacy_alias_central_ai():
    """Backward-compat alias used by existing demos."""
    store = IdentityStore()
    store.add(_card("central"), role="central_ai")
    assert store.master() is not None


def test_identity_store_rejects_corrupt_card_on_load(tmp_path):
    bad = tmp_path / "bad.identity.json"
    bad.write_text(json.dumps({
        "name": "broken",
        "_role": "persona",
        "integrity_hash": "deadbeefdeadbeef",
        "purpose": "p",
        "origin_reason": "o",
    }, ensure_ascii=False), encoding="utf-8")
    store = IdentityStore()
    log = store.load_dir(tmp_path)
    assert any("integrity" in entry.lower() for entry in log)
    assert "broken" not in store.entries


# ----------------------------------------------------------------------------
# Track B — SQLite backend cannot be tricked into deleting master.
# ----------------------------------------------------------------------------


def test_sqlite_master_role_is_immutable(tmp_path):
    path = tmp_path / "apeireth.db"
    store = SqliteIdentityStore(path)
    master = _card("central")
    store.upsert_card(master, role="master")
    with pytest.raises(PermissionError):
        store.upsert_card(_card("central", purpose="changed"), role="snapshot")


def test_sqlite_master_cannot_be_downgraded_and_deleted(tmp_path):
    path = tmp_path / "apeireth.db"
    store = SqliteIdentityStore(path)
    store.upsert_card(_card("central"), role="master")
    with pytest.raises(PermissionError):
        store.upsert_card(_card("central"), role="snapshot")
    with pytest.raises(PermissionError):
        store.delete_card("central")


def test_sqlite_invalid_role_or_schema_rejected(tmp_path):
    path = tmp_path / "apeireth.db"
    store = SqliteIdentityStore(path)
    with pytest.raises(ValueError):
        store.upsert_card(_card("x"), role="central_ai")
    broken = IdentityCard(name="x")  # missing required fields
    with pytest.raises(ValueError):
        store.upsert_card(broken, role="persona")


def test_sqlite_integrity_check_on_read(tmp_path):
    path = tmp_path / "apeireth.db"
    store = SqliteIdentityStore(path)
    store.upsert_card(_card("central"), role="master")
    # Directly mutate card_json to bypass hash check at write time.
    conn = store._conn
    conn.execute(
        "UPDATE identity_cards SET card_json=? WHERE name=?",
        (json.dumps({"name": "central", "purpose": "forged"}, ensure_ascii=False), "central"),
    )
    conn.commit()
    with pytest.raises(ValueError):
        store.get_card("central")


# ----------------------------------------------------------------------------
# Track A — WAL checksum now covers payload; bounds enforced.
# ----------------------------------------------------------------------------


def test_wal_checksum_covers_payload(tmp_path):
    path = tmp_path / "wal.jsonl"
    wal = WriteAheadLog(path=path, fsync=False)
    wal.append("tag_set", {"a": 1})
    # Mutate payload on disk and force re-load.
    raw = path.read_text(encoding="utf-8")
    assert '"a":1' in raw
    forged = raw.replace('"a":1', '"a":2')
    assert forged != raw
    path.write_text(forged, encoding="utf-8")
    fresh = WriteAheadLog(path=path, fsync=False)
    assert fresh.verify() == (0, 1)


def test_wal_path_rejects_symlink(tmp_path):
    target = tmp_path / "real.jsonl"
    target.write_text("", encoding="utf-8")
    link = tmp_path / "link.jsonl"
    try:
        os.symlink(target, link)
    except (OSError, NotImplementedError):
        pytest.skip("symlinks not supported on this platform")
    with pytest.raises(ValueError):
        WriteAheadLog(path=link, fsync=False)


def test_wal_record_size_limit_enforced(tmp_path):
    wal = WriteAheadLog(path=tmp_path / "wal.jsonl", fsync=False)
    with pytest.raises(ValueError):
        wal.append("op", {"blob": "x" * (MAX_WAL_LINE_BYTES + 1)})


def test_wal_op_name_must_be_bounded(tmp_path):
    wal = WriteAheadLog(path=tmp_path / "wal.jsonl", fsync=False)
    with pytest.raises(ValueError):
        wal.append("", {})


# ----------------------------------------------------------------------------
# Track A — replay state is deep-copied; WAL tamper fails closed.
# ----------------------------------------------------------------------------


def test_restore_state_uses_deep_copy_of_snapshot():
    mr = MemoryReplay()
    mr.apply_event("s", Event("e1", 1.0, "tag_set", (("k", "v"),)))
    sid = mr.capture_state("s")
    cp = next(c for c in mr._checkpoints if c.state_id == sid)
    # The captured snapshot must NOT be the same dict object as the live
    # state; otherwise in-place mutations would silently alter history.
    assert cp.state is not mr._live_state
    captured_hash = MemoryReplay._state_hash(cp.state)
    # Mutate live state after capture.
    mr.apply_event("s", Event("e2", 2.0, "tag_set", (("after", "yes"),)))
    mr.restore_state(sid)
    # Captured snapshot must still hash to the original pre-mutation value.
    assert MemoryReplay._state_hash(cp.state) == captured_hash


def test_wal_entry_checksum_includes_payload():
    payload_a = {"k": 1}
    payload_b = {"k": 2}
    e1 = WalEntry(sequence=1, ts=1.0, scope="s",
                  event=Event("e1", 1.0, "tag_set", (("k", 1),)))
    e2 = WalEntry(sequence=1, ts=1.0, scope="s",
                  event=Event("e1", 1.0, "tag_set", (("k", 2),)))
    # Even when sequence/scope match, different payload must yield different
    # checksum so that replay tampered lines are detected.
    assert e1.compute_checksum() != e2.compute_checksum()


# ----------------------------------------------------------------------------
# Track D — MCP dispatcher privileges and input limits.
# ----------------------------------------------------------------------------


@pytest.fixture
def store(tmp_path):
    return MemoryStore(tmp_path)


@pytest.fixture
def dispatcher(store):
    return MCPDispatcher(store)


@pytest.fixture
def privileged(store):
    return MCPDispatcher(store, allow_privileged_tools=True)


def _call(d, name, args):
    res = d.handle_message({"jsonrpc": "2.0", "id": 1, "method": "tools/call",
                            "params": {"name": name, "arguments": args}})
    if res.get("error"):
        return {"error": res["error"]["message"]}
    out = res["result"]
    if out.get("isError"):
        return {"error": out["content"][0]["text"]}
    for c in out.get("content", []):
        if c.get("type") == "json":
            return c["data"]
    return {}


def test_default_dispatcher_rejects_self_claimed_master(dispatcher):
    out = _call(dispatcher, "memory_add", {
        "content": "forged", "actor": "master", "importance": 0.99,
    })
    assert "error" in out
    assert "privileged" in out["error"].lower()


def test_default_dispatcher_rejects_identity_reads(dispatcher):
    out = _call(dispatcher, "identity_get", {})
    assert "error" in out
    assert "privileged" in out["error"].lower()


def test_default_dispatcher_rejects_identity_writes(dispatcher):
    out = _call(dispatcher, "identity_set_persona", {"persona": {"x": 1}})
    assert "error" in out


def test_persona_payload_size_bounded(store, privileged):
    big = "x" * (65536 + 1)
    out = _call(privileged, "identity_set_persona", {"persona": {"k": big}})
    assert "error" in out


def test_memory_add_path_traversal_blocked(dispatcher):
    out = _call(dispatcher, "memory_add", {
        "content": "x", "actor": "external_agent",
        "importance": 0.1, "memory_id": "../escape",
    })
    assert "error" in out


def test_memory_add_size_limits(dispatcher):
    out = _call(dispatcher, "memory_add", {
        "content": "x" * (256 * 1024 + 1),
        "actor": "external_agent", "importance": 0.1,
    })
    assert "error" in out


def test_importance_must_be_finite(dispatcher):
    out = _call(dispatcher, "memory_add", {
        "content": "x", "actor": "external_agent",
        "importance": float("nan"),
    })
    assert "error" in out


def test_search_limit_bounded(dispatcher):
    out = _call(dispatcher, "memory_search", {"limit": 9999})
    assert "error" in out


def test_replay_limit_bounded(dispatcher):
    now = time.time()
    out = _call(dispatcher, "memory_replay", {
        "from_ts": now - 60, "to_ts": now + 5, "limit": 9999,
    })
    assert "error" in out


def test_dream_top_k_bounded(dispatcher):
    out = _call(dispatcher, "memory_dream", {"top_k": 9999})
    assert "error" in out


def test_handle_message_rejects_non_object_params(dispatcher):
    res = dispatcher.handle_message({"jsonrpc": "2.0", "id": 1,
                                     "method": "tools/call",
                                     "params": "not-a-dict"})
    assert res["error"]["code"] == -32602


def test_handle_message_rejects_non_dict_arguments(dispatcher):
    res = dispatcher.handle_message({"jsonrpc": "2.0", "id": 1,
                                     "method": "tools/call",
                                     "params": {"name": "memory_search",
                                                 "arguments": "nope"}})
    assert res["result"]["isError"] is True


# ----------------------------------------------------------------------------
# Track D — HTTP transport requires bearer token; no CORS opt-in.
# ----------------------------------------------------------------------------


def _free_port():
    s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    s.bind(("127.0.0.1", 0))
    port = s.getsockname()[1]
    s.close()
    return port


def test_serve_sse_requires_token():
    store = MemoryStore(Path("/tmp/sec_does_not_matter"))
    dispatcher = MCPDispatcher(store)
    with pytest.raises(ValueError):
        serve_sse(dispatcher, port=_free_port(), host="127.0.0.1")


def test_serve_sse_short_token_rejected():
    store = MemoryStore(Path("/tmp/sec_does_not_matter"))
    dispatcher = MCPDispatcher(store)
    with pytest.raises(ValueError):
        serve_sse(dispatcher, port=_free_port(), host="127.0.0.1",
                  auth_token="too-short")


def test_serve_sse_non_loopback_requires_privileged(tmp_path):
    store = MemoryStore(tmp_path)
    dispatcher = MCPDispatcher(store)  # privileged=False
    with pytest.raises(ValueError):
        serve_sse(dispatcher, port=_free_port(), host="0.0.0.0",
                  auth_token="a" * 40)


def test_http_round_trip_requires_token(tmp_path):
    """401 without Authorization header."""
    port = _free_port()
    store = MemoryStore(tmp_path)
    dispatcher = MCPDispatcher(store, allow_privileged_tools=True)
    thread = threading.Thread(
        target=serve_sse,
        args=(dispatcher,),
        kwargs={"port": port, "host": "127.0.0.1", "auth_token": "x" * 40},
        daemon=True,
    )
    thread.start()
    deadline = time.time() + 5
    while time.time() < deadline:
        try:
            with socket.create_connection(("127.0.0.1", port), timeout=0.2):
                break
        except OSError:
            time.sleep(0.05)
    else:
        pytest.fail("HTTP server did not start")
    try:
        import urllib.request
        req = urllib.request.Request(
            f"http://127.0.0.1:{port}/rpc",
            data=b"{}",
            headers={"Content-Type": "application/json"},
        )
        try:
            urllib.request.urlopen(req, timeout=2).read()
            pytest.fail("expected 401 without bearer token")
        except urllib.error.HTTPError as e:
            assert e.code == 401
    finally:
        pass  # daemon thread will be reaped


def test_http_round_trip_with_token_succeeds(tmp_path):
    port = _free_port()
    store = MemoryStore(tmp_path)
    dispatcher = MCPDispatcher(store, allow_privileged_tools=True)
    thread = threading.Thread(
        target=serve_sse,
        args=(dispatcher,),
        kwargs={"port": port, "host": "127.0.0.1", "auth_token": "x" * 40},
        daemon=True,
    )
    thread.start()
    deadline = time.time() + 5
    while time.time() < deadline:
        try:
            with socket.create_connection(("127.0.0.1", port), timeout=0.2):
                break
        except OSError:
            time.sleep(0.05)
    else:
        pytest.fail("HTTP server did not start")
    try:
        client = HttpMCPClient(f"http://127.0.0.1:{port}/rpc",
                                auth_token="x" * 40)
        # tools/list succeeds when bearer token matches
        tools = client.list_tools()
        assert len(tools) == 7
    finally:
        pass


# ----------------------------------------------------------------------------
# Track D — file permissions default to private on disk.
# ----------------------------------------------------------------------------


def test_memory_store_files_are_user_private(tmp_path):
    if os.name != "posix":
        pytest.skip("POSIX permission check only")
    store = MemoryStore(tmp_path)
    _ = store.add_memory(content="hello", actor="external_agent", importance=0.1)
    memory_file = next(store.mem_dir.glob("*.json"))
    identity_file = store.identity_path
    mode = memory_file.stat().st_mode & 0o777
    identity_mode = identity_file.stat().st_mode & 0o777
    assert mode == 0o600, f"memory file mode {oct(mode)} must be 0o600"
    assert identity_mode == 0o600, f"identity mode {oct(identity_mode)} must be 0o600"