"""Tests for V1097 R8 MCP Memory + Identity Server.

主 22:33 ASI 北极星 + 主 23:44 干到底 + 主 20:46 不假装 + V3 + V1081.

设计:
  - 多数 test 通过 MemoryStore + MCPDispatcher 直接调用 (快)
  - 1 个 test 用 stdio 子进程验证 transport 真的工作
  - 1 个 test 起 HTTP server in a thread 验证 SSE transport
  - 全部用临时目录隔离 (tmp_path pytest fixture)

覆盖率:
  - 7 工具 (memory_add/search/get, identity_get/set_persona, replay/dream) 全部 round-trip
  - JSON-RPC 错误路径 (parse / method-not-found / invalid)
  - 持久化: fsync 后文件真在磁盘上
  - V1081 守门: external_agent importance 上限
  - V1091 兼容: WAL 损坏行不阻塞 replay
  - 幂等: 同 memory_id 重复 add 返回 deduplicated
"""
from __future__ import annotations

import json
import os
import socket
import subprocess
import sys
import threading
import time
from pathlib import Path
from typing import Any, Dict

import pytest

# 让测试能找到 apeireth 包
ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT))

from apeireth.v1097_mcp_memory_server import (  # noqa: E402
    EXTERNAL_IMPORTANCE_CAP,
    MEMORY_KINDS,
    V1097_SCHEMA_VERSION,
    V1097_VERSION,
    MCPDispatcher,
    MemoryStore,
    WalRecord,
    _fsync_write_atomic,
)


# ============================================================================
# Fixtures
# ============================================================================


@pytest.fixture
def store(tmp_path: Path) -> MemoryStore:
    """每个 test 一个干净 base dir."""
    return MemoryStore(tmp_path)


@pytest.fixture
def dispatcher(store: MemoryStore) -> MCPDispatcher:
    return MCPDispatcher(store)


@pytest.fixture
def privileged_dispatcher(store: MemoryStore) -> MCPDispatcher:
    return MCPDispatcher(store, allow_privileged_tools=True)


def _call_tool(d: MCPDispatcher, name: str, args: Dict[str, Any]) -> Dict[str, Any]:
    """调用一个 MCP tool (绕过 JSON-RPC, 直接走 dispatcher).

    错误规约:
      - JSON-RPC error  → {"error": message}
      - isError=True    → {"error": text}
      - 正常            → tool 返回的 data 字典
    """
    res = d.handle_message({"jsonrpc": "2.0", "id": 1, "method": "tools/call",
                            "params": {"name": name, "arguments": args}})
    assert res is not None, "tool call must produce a response"
    if res.get("error"):
        return {"error": res["error"]["message"]}
    result = res["result"]
    if result.get("isError"):
        for c in result.get("content", []):
            if c.get("type") == "text":
                return {"error": c.get("text", "")}
        return {"error": "unknown tool error"}
    for c in result.get("content", []):
        if c.get("type") == "json":
            return c["data"]
        if c.get("type") == "text":
            return {"text": c.get("text", "")}
    return {}


# ============================================================================
# 1. Initialize / tools list
# ============================================================================


def test_handshake_initialize_returns_server_info(dispatcher: MCPDispatcher) -> None:
    res = dispatcher.handle_message({"jsonrpc": "2.0", "id": 1, "method": "initialize",
                                     "params": {"clientInfo": {"name": "tester"}}})
    assert res is not None
    assert res["result"]["serverInfo"]["name"] == "apeireth-memory"
    assert res["result"]["serverInfo"]["version"] == V1097_VERSION
    assert "tools" in res["result"]["capabilities"]
    assert res["result"]["protocolVersion"] == "2024-11-05"


def test_tools_list_returns_7_tools(dispatcher: MCPDispatcher) -> None:
    res = dispatcher.handle_message({"jsonrpc": "2.0", "id": 1, "method": "tools/list",
                                     "params": {}})
    tools = res["result"]["tools"]
    names = {t["name"] for t in tools}
    assert names == {
        "memory_add", "memory_search", "memory_get",
        "identity_get", "identity_set_persona",
        "memory_replay", "memory_dream",
    }
    for t in tools:
        assert "inputSchema" in t
        assert "description" in t
        assert t["inputSchema"]["type"] == "object"


# ============================================================================
# 2. memory_add — 真 fsync 持久化
# ============================================================================


def test_memory_add_persists_and_returns_id(store: MemoryStore, dispatcher: MCPDispatcher) -> None:
    out = _call_tool(dispatcher, "memory_add", {
        "content": "ASI 北极星: 真生产不停",
        "kind": "episode",
        "actor": "external_agent",
        "tags": ["philosophy"],
        "importance": 0.5,
    })
    assert "id" in out
    assert out["kind"] == "episode"
    assert "checksum" in out
    assert out["wal_sequence"] >= 1

    # 文件真在磁盘
    mem_path = store.mem_dir / f"{out['id']}.json"
    assert mem_path.exists()
    assert mem_path.stat().st_size > 0


def test_memory_add_fsync_real_means_file_on_disk(store: MemoryStore, dispatcher: MCPDispatcher) -> None:
    """V1081 不假装: add 后文件真存在 + WAL 追加."""
    out = _call_tool(dispatcher, "memory_add", {
        "content": "fsync test content",
        "kind": "note",
        "actor": "external_agent",
        "importance": 0.6,
        "memory_id": "fixed_id_for_test",
    })
    mem_path = store.mem_dir / "fixed_id_for_test.json"
    assert mem_path.exists()
    # 文件内容必须能 parse, 而且必须包含我们写的内容
    data = json.loads(mem_path.read_text(encoding="utf-8"))
    assert data["content"] == "fsync test content"
    assert data["kind"] == "note"
    assert data["checksum"]
    # WAL 也必须有
    assert store.wal_path.exists()
    lines = store.wal_path.read_bytes().splitlines()
    assert len(lines) >= 1
    last = json.loads(lines[-1])
    assert last["event"]["payload"].get("content_len") == len("fsync test content")


# ============================================================================
# 3. memory_get
# ============================================================================


def test_memory_get_returns_added(store: MemoryStore, dispatcher: MCPDispatcher) -> None:
    add = _call_tool(dispatcher, "memory_add", {
        "content": "unique string: zxcvbnm",
        "kind": "episode", "actor": "external_agent", "importance": 0.4,
    })
    got = _call_tool(dispatcher, "memory_get", {"memory_id": add["id"]})
    assert got["content"] == "unique string: zxcvbnm"
    assert got["kind"] == "episode"
    assert got["id"] == add["id"]


def test_memory_get_not_found(dispatcher: MCPDispatcher) -> None:
    out = _call_tool(dispatcher, "memory_get", {"memory_id": "no_such_id_zzz"})
    assert "error" in out
    assert "not found" in out["error"].lower()


def test_memory_get_blocks_path_traversal(dispatcher: MCPDispatcher) -> None:
    out = _call_tool(dispatcher, "memory_get", {"memory_id": "../../etc/passwd"})
    assert "error" in out
    # 必须不抛异常, 而且必须拒绝


# ============================================================================
# 4. memory_search
# ============================================================================


def test_memory_search_by_query_finds_match(store: MemoryStore, dispatcher: MCPDispatcher) -> None:
    _call_tool(dispatcher, "memory_add", {
        "content": "刺猬型 ASI 是主人生态学的隐喻", "kind": "note",
        "actor": "external_agent", "importance": 0.55,
    })
    _call_tool(dispatcher, "memory_add", {
        "content": "完全无关的内容: 苹果香蕉", "kind": "note",
        "actor": "external_agent", "importance": 0.3,
    })
    res = _call_tool(dispatcher, "memory_search", {"query": "刺猬"})
    assert res["count"] >= 1
    assert any("刺猬" in r["content"] for r in res["results"])


def test_memory_search_by_tags_filters(dispatcher: MCPDispatcher) -> None:
    _call_tool(dispatcher, "memory_add", {
        "content": "tagged A", "kind": "note", "actor": "external_agent",
        "tags": ["alpha", "shared"], "importance": 0.4,
    })
    _call_tool(dispatcher, "memory_add", {
        "content": "tagged B", "kind": "note", "actor": "external_agent",
        "tags": ["beta"], "importance": 0.4,
    })
    res = _call_tool(dispatcher, "memory_search", {"tags": ["shared"]})
    assert res["count"] == 1
    assert res["results"][0]["content"] == "tagged A"


def test_memory_search_empty_returns_empty(dispatcher: MCPDispatcher) -> None:
    res = _call_tool(dispatcher, "memory_search", {"query": "nothing_matches_hopefully"})
    assert res["count"] == 0
    assert res["results"] == []


# ============================================================================
# 5. identity_get / identity_set_persona
# ============================================================================


def test_identity_get_returns_v3_card(store: MemoryStore) -> None:
    card = store.get_identity()
    assert "version" in card
    assert card["is_orchestrator"] is True
    assert card["is_thinker"] is True
    assert card["is_infinite_relations_aggregate"] is True
    assert card["has_max_authority"] is True
    assert card["holds_asi_position"] is True
    assert "vcp_4_paradigms" in card
    assert "phenomenal_consciousness" in card
    assert "persona" in card


def test_identity_set_persona_persists_with_fsync(store: MemoryStore, privileged_dispatcher: MCPDispatcher) -> None:
    _call_tool(privileged_dispatcher, "identity_set_persona", {
        "persona": {"stance": "test-stance-123", "language": "zh-CN"},
    })
    # reload from disk
    fresh = json.loads(store.identity_path.read_text(encoding="utf-8"))
    assert fresh["persona"]["stance"] == "test-stance-123"
    assert fresh["persona"]["language"] == "zh-CN"
    assert "persona_updated_at" in fresh


# ============================================================================
# 6. memory_replay
# ============================================================================


def test_memory_replay_returns_events_in_window(store: MemoryStore, dispatcher: MCPDispatcher) -> None:
    t0 = time.time()
    _call_tool(dispatcher, "memory_add", {
        "content": "replay-A", "kind": "episode", "actor": "external_agent", "importance": 0.4,
    })
    time.sleep(0.01)
    _call_tool(dispatcher, "memory_add", {
        "content": "replay-B", "kind": "note", "actor": "external_agent", "importance": 0.5,
    })
    t1 = time.time()
    res = _call_tool(dispatcher, "memory_replay", {
        "from_ts": t0 - 1, "to_ts": t1 + 1,
    })
    assert res["count"] >= 2
    kinds = [e["event_kind"] for e in res["events"]]
    assert "memory_episode" in kinds
    assert "memory_note" in kinds


def test_memory_replay_filters_by_scope(dispatcher: MCPDispatcher) -> None:
    _call_tool(dispatcher, "memory_add", {
        "content": "x", "kind": "episode", "actor": "external_agent", "importance": 0.3,
    })
    _call_tool(dispatcher, "identity_set_persona", {"persona": {"x": 1}})
    now = time.time()
    res_mem = _call_tool(dispatcher, "memory_replay", {
        "from_ts": now - 60, "to_ts": now + 5, "scope": "memory",
    })
    res_id = _call_tool(dispatcher, "memory_replay", {
        "from_ts": now - 60, "to_ts": now + 5, "scope": "identity",
    })
    assert all(e["scope"] == "memory" for e in res_mem["events"])
    assert all(e["scope"] == "identity" for e in res_id["events"])


def test_wal_corrupted_line_skipped_but_alive(store: MemoryStore, dispatcher: MCPDispatcher) -> None:
    """V1091 兼容: 损坏的 WAL 行不阻塞 replay."""
    _call_tool(dispatcher, "memory_add", {
        "content": "valid before corruption", "kind": "episode",
        "actor": "external_agent", "importance": 0.3,
    })
    # 注入损坏行
    with open(store.wal_path, "ab") as f:
        f.write(b"this-is-not-json\n")
    _call_tool(dispatcher, "memory_add", {
        "content": "valid after corruption", "kind": "episode",
        "actor": "external_agent", "importance": 0.3,
    })
    now = time.time()
    res = _call_tool(dispatcher, "memory_replay", {
        "from_ts": now - 60, "to_ts": now + 5,
    })
    assert res["count"] >= 2
    assert res["skipped"] >= 1


# ============================================================================
# 7. memory_dream
# ============================================================================


def test_memory_dream_returns_top_clusters(dispatcher: MCPDispatcher) -> None:
    # 制造 4 个 notes, 3 个共享 tag cluster-A + 1 个孤立
    for i in range(3):
        _call_tool(dispatcher, "memory_add", {
            "content": f"note {i} about cluster-A", "kind": "note",
            "actor": "external_agent", "tags": ["cluster-A", "philosophy"],
            "importance": 0.5 + 0.1 * i,
        })
    _call_tool(dispatcher, "memory_add", {
        "content": "isolated note", "kind": "note",
        "actor": "external_agent", "tags": ["solo"], "importance": 0.1,
    })
    res = _call_tool(dispatcher, "memory_dream", {"top_k": 2})
    assert res["count"] >= 1
    # 第一个 cluster 必须是 cluster-A (importance 之和最高)
    top = res["clusters"][0]
    assert "cluster-A" in top["tags"]
    assert top["n_notes"] == 3


def test_memory_dream_no_notes_returns_empty(dispatcher: MCPDispatcher) -> None:
    res = _call_tool(dispatcher, "memory_dream", {"top_k": 5})
    assert res["count"] == 0
    assert res["insights"] == []


# ============================================================================
# 8. V1081 / V3 守门
# ============================================================================


def test_external_agent_importance_capped(dispatcher: MCPDispatcher) -> None:
    """V1081: external_agent importance > 0.7 必须拒绝."""
    out = _call_tool(dispatcher, "memory_add", {
        "content": "external trying to inflate",
        "kind": "episode", "actor": "external_agent",
        "importance": 0.9,
    })
    assert "error" in out
    assert str(EXTERNAL_IMPORTANCE_CAP) in out["error"] or "capped" in out["error"].lower()


def test_master_can_use_high_importance(privileged_dispatcher: MCPDispatcher) -> None:
    """V1081 例外: master actor 可以用高 importance (需 privileged dispatcher)."""
    out = _call_tool(privileged_dispatcher, "memory_add", {
        "content": "master note", "kind": "note", "actor": "master",
        "importance": 0.95,
    })
    assert "id" in out
    assert "error" not in out


def test_invalid_actor_rejected(dispatcher: MCPDispatcher) -> None:
    out = _call_tool(dispatcher, "memory_add", {
        "content": "x", "kind": "episode", "actor": "evil_actor",
        "importance": 0.5,
    })
    assert "error" in out


def test_invalid_kind_rejected(privileged_dispatcher: MCPDispatcher) -> None:
    out = _call_tool(privileged_dispatcher, "memory_add", {
        "content": "x", "kind": "weird_kind", "actor": "master", "importance": 0.5,
    })
    assert "error" in out


# ============================================================================
# 9. 幂等
# ============================================================================


def test_idempotent_add_same_id_returns_existing(store: MemoryStore, privileged_dispatcher: MCPDispatcher) -> None:
    fixed = "idempotent_test_xyz"
    first = _call_tool(privileged_dispatcher, "memory_add", {
        "content": "first", "kind": "note", "actor": "master",
        "importance": 0.5, "memory_id": fixed,
    })
    second = _call_tool(privileged_dispatcher, "memory_add", {
        "content": "second-different-content", "kind": "note", "actor": "master",
        "importance": 0.5, "memory_id": fixed,
    })
    assert first["id"] == second["id"] == fixed
    assert second.get("deduplicated") is True
    # 磁盘上 content 必须是 first (second 被去重跳过)
    data = json.loads((store.mem_dir / f"{fixed}.json").read_text(encoding="utf-8"))
    assert data["content"] == "first"


# ============================================================================
# 10. JSON-RPC 错误路径
# ============================================================================


def test_jsonrpc_method_not_found(dispatcher: MCPDispatcher) -> None:
    res = dispatcher.handle_message({"jsonrpc": "2.0", "id": 9, "method": "tools/destroy",
                                     "params": {}})
    assert res is not None
    assert res["error"]["code"] == -32601
    assert "not found" in res["error"]["message"].lower()


def test_jsonrpc_notification_no_response(dispatcher: MCPDispatcher) -> None:
    res = dispatcher.handle_message({"jsonrpc": "2.0", "method": "notifications/initialized",
                                     "params": {}})
    assert res is None  # 通知不响应


def test_jsonrpc_tool_error_returns_isError(dispatcher: MCPDispatcher) -> None:
    res = dispatcher.handle_message({"jsonrpc": "2.0", "id": 1, "method": "tools/call",
                                     "params": {"name": "memory_get",
                                                "arguments": {"memory_id": "nope"}}})
    assert res["result"]["isError"] is True
    content = res["result"]["content"][0]
    assert "not found" in content["text"].lower()


# ============================================================================
# 11. 完整 lifecycle round-trip
# ============================================================================


def test_round_trip_full_lifecycle(store: MemoryStore, privileged_dispatcher: MCPDispatcher) -> None:
    """add episode → add note (引用 episode) → search → get → replay → dream."""
    # add episode
    ep = _call_tool(privileged_dispatcher, "memory_add", {
        "content": "Episode: 真生产不停", "kind": "episode",
        "actor": "master", "importance": 0.8, "tags": ["core"],
    })
    # add note 引用 episode
    nt = _call_tool(privileged_dispatcher, "memory_add", {
        "content": "Note: 真生产不停是 ASI 北极星", "kind": "note",
        "actor": "master", "importance": 0.7, "tags": ["core", "philosophy"],
        "evidence": [ep["id"]],
    })
    # search by tag
    sr = _call_tool(privileged_dispatcher, "memory_search", {"tags": ["core"]})
    assert sr["count"] == 2
    # get each
    got_ep = _call_tool(privileged_dispatcher, "memory_get", {"memory_id": ep["id"]})
    got_nt = _call_tool(privileged_dispatcher, "memory_get", {"memory_id": nt["id"]})
    assert "真生产不停" in got_ep["content"]
    assert ep["id"] in got_nt["evidence"]
    # replay
    now = time.time()
    rp = _call_tool(privileged_dispatcher, "memory_replay", {
        "from_ts": now - 60, "to_ts": now + 5,
    })
    assert rp["count"] >= 2
    # dream
    dr = _call_tool(privileged_dispatcher, "memory_dream", {"top_k": 3})
    assert dr["count"] >= 1
    # stats
    stats = store.stats()
    assert stats["n_memories"] == 2
    assert stats["wal_sequence"] >= 2
    assert stats["schema_version"] == V1097_SCHEMA_VERSION


# ============================================================================
# 12. stdio subprocess transport — 真的 spawn 一个 server
# ============================================================================


def test_stdio_subprocess_roundtrip(tmp_path: Path) -> None:
    """Spawn 一个真正的 stdio MCP server 子进程, 走 NDJSON, 验证 transport 真实."""
    from apeireth.v1097_mcp_example_client import StdioMCPClient
    with StdioMCPClient(base=str(tmp_path)) as client:
        tools = client.list_tools()
        assert len(tools) == 7
        ep = client.call_tool("memory_add", {
            "content": "stdio real roundtrip", "kind": "episode",
            "actor": "external_agent", "importance": 0.4,
        })
        assert "id" in ep
        got = client.call_tool("memory_get", {"memory_id": ep["id"]})
        assert got["content"] == "stdio real roundtrip"
        # 验证文件真在 tmp_path 下 (fsync 验证)
        on_disk = tmp_path / "memory" / f"{ep['id']}.json"
        assert on_disk.exists()
        data = json.loads(on_disk.read_text(encoding="utf-8"))
        assert data["content"] == "stdio real roundtrip"


# ============================================================================
# 13. SSE/HTTP transport — 起 server in thread
# ============================================================================


def _free_port() -> int:
    s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    s.bind(("127.0.0.1", 0))
    port = s.getsockname()[1]
    s.close()
    return port


def test_sse_http_round_trip(tmp_path: Path) -> None:
    """起 HTTP server in a thread, 用 urllib 打 /rpc, 验证 SSE transport 真."""
    from apeireth.v1097_mcp_example_client import HttpMCPClient
    from apeireth.v1097_mcp_memory_server import (
        MCPDispatcher,
        MemoryStore,
        serve_sse,
    )

    port = _free_port()
    store = MemoryStore(tmp_path)
    dispatcher = MCPDispatcher(store, allow_privileged_tools=True)
    server = serve_sse  # noqa: F841

    http_thread = threading.Thread(
        target=serve_sse,
        args=(dispatcher,),
        kwargs={"port": port, "host": "127.0.0.1",
                "auth_token": "a" * 40},
        daemon=True,
    )
    http_thread.start()
    # 等 server 起来
    deadline = time.time() + 5
    while time.time() < deadline:
        try:
            with socket.create_connection(("127.0.0.1", port), timeout=0.2):
                break
        except OSError:
            time.sleep(0.05)
    else:
        pytest.fail("HTTP server did not start in time")

    try:
        client = HttpMCPClient(f"http://127.0.0.1:{port}/rpc", auth_token="a" * 40)
        # initialize
        init = client.call("initialize", {"clientInfo": {"name": "sse-test"}})
        assert init["serverInfo"]["name"] == "apeireth-memory"
        # tools/list
        tools = client.list_tools()
        assert len(tools) == 7
        # add
        ep = client.call_tool("memory_add", {
            "content": "sse roundtrip", "kind": "episode",
            "actor": "external_agent", "importance": 0.5,
        })
        assert "id" in ep
        # get
        got = client.call_tool("memory_get", {"memory_id": ep["id"]})
        assert got["content"] == "sse roundtrip"
        # 验证 on disk
        on_disk = tmp_path / "memory" / f"{ep['id']}.json"
        assert on_disk.exists()
    finally:
        # serve_sse 是阻塞的, 不好 graceful stop; 用 thread daemon=True 自动回收
        pass


# ============================================================================
# 14. WalRecord 自身 round-trip
# ============================================================================


def test_wal_record_to_from_jsonl_roundtrip() -> None:
    rec = WalRecord(
        sequence=42,
        ts=1234567890.123456,
        scope="memory",
        event_id="abc",
        event_kind="memory_episode",
        payload={"content_len": 5, "actor": "master", "tags": ["x"]},
    )
    line = rec.to_jsonl()
    back = WalRecord.from_jsonl(line)
    assert back.sequence == 42
    assert back.scope == "memory"
    assert back.event_id == "abc"
    assert back.event_kind == "memory_episode"
    assert back.payload == {"content_len": 5, "actor": "master", "tags": ["x"]}
    assert back.checksum == rec.checksum


# ============================================================================
# 15. stats / introspection
# ============================================================================


def test_stats_reports_philosophy_guards(store: MemoryStore) -> None:
    s = store.stats()
    assert s["version"] == V1097_VERSION
    assert "fsync-before-success" in s["philosophy_guards"]
    assert "external-importance-capped" in s["philosophy_guards"]
    assert "actor-whitelist" in s["philosophy_guards"]


# ============================================================================
# 16. _fsync_write_atomic 在损坏场景下的行为
# ============================================================================


def test_fsync_write_atomic_overwrites_existing(tmp_path: Path) -> None:
    p = tmp_path / "x.json"
    _fsync_write_atomic(p, b"first")
    _fsync_write_atomic(p, b"second")
    assert p.read_bytes() == b"second"


def test_fsync_write_atomic_creates_parent(tmp_path: Path) -> None:
    p = tmp_path / "deep" / "nested" / "x.json"
    _fsync_write_atomic(p, b"data")
    assert p.exists()
    assert p.read_bytes() == b"data"
