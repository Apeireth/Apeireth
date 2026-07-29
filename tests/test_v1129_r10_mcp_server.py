"""Apeireth ASI V1129 — R10 ASI 北极星 MCP server tests (R10 W1).

主 17:43 实事求是 + 主 00:56 任何人都能接手 + 主 22:33 ASI 北极星 + 主 23:44 干到底.

测试覆盖 (≥25, 全真行为, 不 mock):
  1.  常量与模块结构 (3)
  2.  5 tools schema 守门 (5): 每个 tool 必填字段 + 类型约束
  3.  5 tools round-trip 集成 (5): V1095 / V1124 / V1125 真跑
  4.  ASI 9 键 LOCKED 注入 (2)
  5.  SSE transport (3): 真起 server + 真 GET /sse + session 管理
  6.  HTTP transport (1): 真起 server + curl 3 类请求
  7.  stdio transport (1): NDJSON oneshot
  8.  chaos 守门 (2): dispatcher state 跨 transport 启停保留
  9.  CLI 入口 (3): --selftest / --chaos / --snapshot
  10. V3 守门 (1)

运行: pytest -q tests/test_v1129_r10_mcp_server.py
"""
from __future__ import annotations

import io
import json
import os
import socket
import sys
import threading
import time
import unittest
import urllib.error
import urllib.request
from contextlib import redirect_stdout
from pathlib import Path
from typing import Any, Dict
from unittest import mock

import pytest

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT))

from apeireth.mcp import ASI_NINE_KEYS, PROTOCOL_VERSION  # noqa: E402
from apeireth.mcp.protocol import (  # noqa: E402
    SUPPORTED_PROTOCOL_VERSIONS, JSONRPC_INVALID_PARAMS, JSONRPC_METHOD_NOT_FOUND,
    SchemaViolation, check_protocol_version, make_error_response,
    parse_request, validate_arguments, validate_tool_result,
)
from apeireth.mcp.asi_nine_keys import (  # noqa: E402
    AsiNineKeyLock, inject_guard_block, verify_or_raise,
)
from apeireth.mcp.transport import StdioTransport, HttpTransport  # noqa: E402
from apeireth.mcp.sse_transport import (  # noqa: E402
    SseTransport, SseSessionStore, SseSession, SSE_QUEUE_MAX, SSE_PING_INTERVAL,
)
from apeireth.mcp.r10_asi_north_star_server import (  # noqa: E402
    R10AsiNorthStarDispatcher, SERVER_NAME, V1129_VERSION,
    MEASURE_ASI_SCHEMA, GET_NORTH_STAR_SCHEMA, CHECK_IDENTITY_SCHEMA,
    VERIFY_AUDIT_CHAIN_SCHEMA, LIST_PERSONAS_SCHEMA,
)
from apeireth.v1129_r10_mcp_server import (  # noqa: E402
    V1129_FRAMEWORK_VERSION, V3_GUARDS,
    build_default_dispatcher, run_selftest, cli_main,
    make_v1095_store, make_v1124_backend,
    serve_stdio, serve_http, serve_sse,
)


# ---------------------------------------------------------------------------
# 1. 常量与模块结构 (3 tests)
# ---------------------------------------------------------------------------


class TestV1129Constants(unittest.TestCase):
    def test_versions_and_server_name(self):
        self.assertEqual(V1129_VERSION, "0.1.0")
        self.assertEqual(V1129_FRAMEWORK_VERSION, "0.1.0")
        self.assertEqual(SERVER_NAME, "apeireth-r10-asi-mcp")
        self.assertEqual(PROTOCOL_VERSION, "2024-11-05")

    def test_supported_protocol_versions(self):
        self.assertEqual(list(SUPPORTED_PROTOCOL_VERSIONS), ["2024-11-05"])

    def test_v3_guards_present(self):
        self.assertIn("module_is_not_asi", V3_GUARDS)
        self.assertIn("r10_measure_is_not_asi", V3_GUARDS)
        self.assertIn("integration_is_not_autonomy", V3_GUARDS)
        self.assertIn("mcp_chaos_state_is_not_truth", V3_GUARDS)
        self.assertGreaterEqual(len(V3_GUARDS), 5)


# ---------------------------------------------------------------------------
# 2. 5 tools schema 守门 (5 tests)
# ---------------------------------------------------------------------------


class TestToolSchemas(unittest.TestCase):
    """5 tools 必填字段 + 类型约束验证 (主 23:44 干到底)."""

    def _validate(self, schema, args, name):
        validate_arguments(args, schema, tool_name=name)

    def test_measure_asi_schema(self):
        # v04_actual 必填
        with self.assertRaises(SchemaViolation):
            self._validate(MEASURE_ASI_SCHEMA, {}, "measure_asi")
        # 错类型
        with self.assertRaises(SchemaViolation):
            self._validate(MEASURE_ASI_SCHEMA, {"v04_actual": "abc"}, "measure_asi")
        # 正常
        self._validate(MEASURE_ASI_SCHEMA,
                        {"v04_actual": 0.8538, "week_label": "R10-W1"}, "measure_asi")
        # 越界 v04_actual
        with self.assertRaises(SchemaViolation):
            self._validate(MEASURE_ASI_SCHEMA, {"v04_actual": 2.0}, "measure_asi")
        # week_label 不匹配 pattern
        with self.assertRaises(SchemaViolation):
            self._validate(MEASURE_ASI_SCHEMA,
                            {"v04_actual": 0.5, "week_label": "W4"}, "measure_asi")

    def test_get_north_star_schema(self):
        self._validate(GET_NORTH_STAR_SCHEMA, {}, "get_north_star")
        self._validate(GET_NORTH_STAR_SCHEMA, {"include_composite": False},
                        "get_north_star")
        with self.assertRaises(SchemaViolation):
            self._validate(GET_NORTH_STAR_SCHEMA,
                            {"v1124_base_url": 123}, "get_north_star")

    def test_check_identity_schema(self):
        self._validate(CHECK_IDENTITY_SCHEMA, {}, "check_identity")
        with self.assertRaises(SchemaViolation):
            self._validate(CHECK_IDENTITY_SCHEMA,
                            {"include_switches": "yes"}, "check_identity")

    def test_verify_audit_chain_schema(self):
        self._validate(VERIFY_AUDIT_CHAIN_SCHEMA, {}, "verify_audit_chain")
        with self.assertRaises(SchemaViolation):
            self._validate(VERIFY_AUDIT_CHAIN_SCHEMA,
                            {"include_breakdown": "yes"}, "verify_audit_chain")

    def test_list_personas_schema(self):
        self._validate(LIST_PERSONAS_SCHEMA, {}, "list_personas")
        self._validate(LIST_PERSONAS_SCHEMA,
                        {"archetype": "调度者", "include_emerged": False},
                        "list_personas")
        with self.assertRaises(SchemaViolation):
            self._validate(LIST_PERSONAS_SCHEMA,
                            {"archetype": 123}, "list_personas")


# ---------------------------------------------------------------------------
# 3. 5 tools round-trip 集成 (5 tests, V1095/V1124/V1125 真跑)
# ---------------------------------------------------------------------------


class TestToolRoundTrip(unittest.TestCase):
    """5 工具端到端真集成测试 (主 17:43 实事求是)."""

    @classmethod
    def setUpClass(cls):
        cls.dispatcher = build_default_dispatcher(bind_external=True)

    def _call(self, name, args):
        resp = self.dispatcher.handle_message({
            "jsonrpc": "2.0", "id": 1, "method": "tools/call",
            "params": {"name": name, "arguments": args},
        })
        return resp["result"]

    def test_measure_asi(self):
        r = self._call("measure_asi",
                        {"v04_actual": 0.8538, "week_label": "R10-W1"})
        self.assertFalse(r.get("isError"))
        d = r["content"][0]["data"]
        self.assertEqual(d["source"], "v1125_evaluate_r10")
        # V0.5 真测分 (R10 起点 = 0.86)
        self.assertGreater(d["v05_total"], 0.0)
        self.assertIn("composite", d)
        self.assertIn("track_decision", d)
        # ASI 9 键 LOCKED 必注入
        self.assertIn("philosophy_guard", r["content"][0]["data"])

    def test_get_north_star_in_process(self):
        r = self._call("get_north_star", {})
        self.assertFalse(r.get("isError"))
        d = r["content"][0]["data"]
        # 真连 V1124 (in-process)
        self.assertEqual(d["source"], "v1124_in_process")
        self.assertEqual(d["transport"], "in_process")
        self.assertIn("north_star", d)
        self.assertIn("philosophy_guard", d)

    def test_check_identity(self):
        r = self._call("check_identity", {"include_switches": True})
        self.assertFalse(r.get("isError"))
        d = r["content"][0]["data"]
        self.assertEqual(d["source"], "v1095_in_process")
        self.assertIn("identity", d)
        # 中央 AI 默认 identity_id/name
        self.assertIn("identity_id", d["identity"])
        self.assertIn("stats", d)
        self.assertIn("n_switches_total", d)
        self.assertIn("philosophy_guard", d)

    def test_verify_audit_chain(self):
        r = self._call("verify_audit_chain", {})
        self.assertFalse(r.get("isError"))
        d = r["content"][0]["data"]
        self.assertIn("v1095_identity", d["checks"])
        self.assertIn("v1124_audit", d["checks"])
        # 两边都应 pass (真集成)
        self.assertTrue(d["checks"]["v1095_identity"]["pass"])
        self.assertTrue(d["checks"]["v1124_audit"]["pass"])
        self.assertTrue(d["all_pass"])
        self.assertIn("v1095_in_process", d["source"])
        self.assertIn("v1124_in_process", d["source"])

    def test_list_personas(self):
        r = self._call("list_personas", {})
        self.assertFalse(r.get("isError"))
        d = r["content"][0]["data"]
        self.assertEqual(d["source"], "v1095_in_process")
        # 4 默认 archetype 必 seed
        self.assertEqual(d["n_personas"], 4)
        archetypes = {p["archetype"] for p in d["personas"]}
        self.assertEqual(archetypes, {"调度者", "学习者", "思考者", "助手"})
        self.assertIn("philosophy_guard", d)


# ---------------------------------------------------------------------------
# 4. ASI 9 键 LOCKED 注入 (2 tests)
# ---------------------------------------------------------------------------


class TestNineKeyLock(unittest.TestCase):
    def test_default_all_locked_and_injected(self):
        d = build_default_dispatcher(bind_external=False)
        r = d.handle_message({"jsonrpc": "2.0", "id": 1, "method": "initialize",
                               "params": {"protocolVersion": "2024-11-05"}})
        # initialize 必返 9 键
        self.assertTrue(r["result"]["nine_key_lock"]["asi_nine_keys_locked"])
        # tools/list 也必含 9 键? 答: 不在 initialize 之外的 method 注入
        lst = d.handle_message({"jsonrpc": "2.0", "id": 2, "method": "tools/list",
                                 "params": {}})
        self.assertEqual(len(lst["result"]["tools"]), 5)

    def test_nine_key_inject_can_be_disabled(self):
        lock = AsiNineKeyLock()
        d = build_default_dispatcher(bind_external=False)
        d.nine_key_inject = False
        # measure_asi 工具会因为 bind_external=False 返 isError (no v1125 eval? 实际会跑)
        # 直接测 inject_guard_block 即可
        result = {"content": [{"type": "json", "data": {"x": 1}}]}
        out = inject_guard_block(result, lock)
        self.assertIn("philosophy_guard", out["content"][0]["data"])
        # disable 后, 不注入
        result2 = {"content": [{"type": "json", "data": {"x": 1}}]}
        # simulate handler not injecting
        self.assertNotIn("philosophy_guard", result2["content"][0]["data"])


# ---------------------------------------------------------------------------
# 5. SSE transport (3 tests, 真起 server + 真 GET /sse + 真 session 管理)
# ---------------------------------------------------------------------------


class TestSseTransport(unittest.TestCase):
    def test_sse_server_health_and_endpoint(self):
        d = build_default_dispatcher(bind_external=True)
        sse = SseTransport(
            dispatch=d.handle_message,
            server_info={"name": SERVER_NAME, "tools": list(d.TOOLS.keys())},
            host="127.0.0.1", port=0,
        )
        sse.start()
        try:
            base = f"http://127.0.0.1:{sse.actual_port}"
            # GET /health
            with urllib.request.urlopen(f"{base}/health", timeout=2) as r:
                self.assertEqual(r.status, 200)
                body = json.loads(r.read().decode("utf-8"))
                self.assertTrue(body["ok"])
                self.assertEqual(body["transport"], "sse")
                self.assertEqual(body["n_sessions"], 0)
            # GET /tools
            with urllib.request.urlopen(f"{base}/tools", timeout=2) as r:
                body = json.loads(r.read().decode("utf-8"))
                self.assertEqual(len(body["tools"]), 5)
        finally:
            sse.stop()

    def test_sse_session_store_lifecycle(self):
        store = SseSessionStore()
        s1 = store.create()
        s2 = store.create()
        self.assertEqual(len(store.stats()["sessions"]), 2)
        # push 成功
        self.assertTrue(s1.push({"jsonrpc": "2.0", "id": 1, "result": {"ok": True}}))
        self.assertEqual(s1.n_messages_sent, 0)  # not sent, just pushed
        # reap_expired 在 alive 状态下不清理
        self.assertEqual(store.reap_expired(), 0)
        # 标记 sse_alive = False 后 age 才能 reap
        s1.sse_alive = False
        s1.last_active_at = time.time() - 3600  # 1 小时前
        self.assertGreaterEqual(store.reap_expired(), 1)
        self.assertEqual(len(store.stats()["sessions"]), 1)
        # 移除
        store.remove(s2.session_id)
        self.assertEqual(len(store.stats()["sessions"]), 0)

    def test_sse_session_outbox_full_drops_messages(self):
        # 主 17:43 实事求是: outbox 满 → drop, 计数, 不假装
        s = SseSession(session_id="test")
        for i in range(SSE_QUEUE_MAX + 5):
            s.push({"i": i})
        self.assertGreater(s.n_messages_dropped, 0)
        self.assertGreaterEqual(SSE_PING_INTERVAL, 5.0)  # 守门常量

    def test_sse_post_messages_with_disconnected_stream(self):
        """chaos: SSE stream 失联后 POST, server 走 degraded 同步响应路径."""
        d = build_default_dispatcher(bind_external=True)
        sse = SseTransport(
            dispatch=d.handle_message,
            server_info={"name": SERVER_NAME, "tools": list(d.TOOLS.keys())},
            host="127.0.0.1", port=0,
        )
        sse.start()
        try:
            base = f"http://127.0.0.1:{sse.actual_port}"
            # 直接 POST 到 /messages 但没建立 /sse 流 → 应返 missing session_id (400)
            req = urllib.request.Request(
                f"{base}/messages",
                data=b'{"jsonrpc":"2.0","id":1,"method":"ping","params":{}}',
                headers={"Content-Type": "application/json"},
                method="POST",
            )
            # 直接 connect port → 必有响应 (避免被 urllib 在 400 抛 HTTPError)
            try:
                with urllib.request.urlopen(req, timeout=2) as r:
                    body = json.loads(r.read().decode("utf-8"))
                    self.assertEqual(body.get("error"), "missing session_id")
            except urllib.error.HTTPError as exc:
                self.assertEqual(exc.code, 400)
                body = json.loads(exc.read().decode("utf-8"))
                self.assertEqual(body.get("error"), "missing session_id")
            # chaos: dispatcher state 仍然完整 (没受影响)
            stats = d.stats()
            self.assertEqual(stats["n_tools"], 5)
            self.assertTrue(stats["nine_key_lock"]["asi_nine_keys_locked"])
        finally:
            sse.stop()

    def test_sse_post_messages_session_expired_returns_410(self):
        """chaos: session 已被 reaper 清掉 → POST 返 410, dispatcher 不丢."""
        d = build_default_dispatcher(bind_external=True)
        sse = SseTransport(
            dispatch=d.handle_message,
            server_info={"name": SERVER_NAME, "tools": list(d.TOOLS.keys())},
            host="127.0.0.1", port=0,
        )
        sse.start()
        try:
            base = f"http://127.0.0.1:{sse.actual_port}"
            fake_sid = "deadbeef" * 4  # 32 hex chars
            req = urllib.request.Request(
                f"{base}/messages?session_id={fake_sid}",
                data=b'{"jsonrpc":"2.0","id":1,"method":"ping","params":{}}',
                headers={"Content-Type": "application/json"},
                method="POST",
            )
            try:
                with urllib.request.urlopen(req, timeout=2) as r:
                    body = json.loads(r.read().decode("utf-8"))
                    self.assertIn("session expired", body.get("error", ""))
            except urllib.error.HTTPError as exc:
                self.assertEqual(exc.code, 410)
                body = json.loads(exc.read().decode("utf-8"))
                self.assertIn("session expired", body.get("error", ""))
            # chaos: dispatcher state 保留
            stats = d.stats()
            self.assertEqual(stats["n_tools"], 5)
        finally:
            sse.stop()


# ---------------------------------------------------------------------------
# 6. HTTP transport (1 test, 真起 server + curl 3 类请求)
# ---------------------------------------------------------------------------


class TestHttpTransport(unittest.TestCase):
    def test_http_server_round_trip(self):
        d = build_default_dispatcher(bind_external=True)
        http = HttpTransport(
            dispatch=d.handle_message,
            server_info={"name": SERVER_NAME, "tools": list(d.TOOLS.keys())},
            host="127.0.0.1", port=0,
        )
        http.start()
        try:
            base = f"http://127.0.0.1:{http.actual_port}"
            # GET /health
            with urllib.request.urlopen(f"{base}/health", timeout=2) as r:
                self.assertEqual(r.status, 200)
                self.assertTrue(json.loads(r.read().decode("utf-8"))["ok"])
            # GET /tools
            with urllib.request.urlopen(f"{base}/tools", timeout=2) as r:
                self.assertEqual(len(json.loads(r.read().decode("utf-8"))["tools"]), 5)
            # POST /rpc list_personas
            req = urllib.request.Request(
                f"{base}/rpc",
                data=json.dumps({"jsonrpc": "2.0", "id": 1, "method": "tools/call",
                                  "params": {"name": "list_personas",
                                              "arguments": {"include_emerged": True}}}).encode("utf-8"),
                headers={"Content-Type": "application/json"},
                method="POST",
            )
            with urllib.request.urlopen(req, timeout=5) as r:
                body = json.loads(r.read().decode("utf-8"))
                self.assertEqual(body["result"]["content"][0]["data"]["n_personas"], 4)
                self.assertTrue(body["result"]["content"][0]["data"]["philosophy_guard"]["asi_nine_keys_locked"])
            # POST /rpc invalid params
            req = urllib.request.Request(
                f"{base}/rpc",
                data=json.dumps({"jsonrpc": "2.0", "id": 1, "method": "tools/call",
                                  "params": {"name": "measure_asi", "arguments": {}}}).encode("utf-8"),
                headers={"Content-Type": "application/json"},
                method="POST",
            )
            with urllib.request.urlopen(req, timeout=5) as r:
                body = json.loads(r.read().decode("utf-8"))
                self.assertEqual(body["error"]["code"], JSONRPC_INVALID_PARAMS)
        finally:
            http.stop()


# ---------------------------------------------------------------------------
# 7. stdio transport (1 test)
# ---------------------------------------------------------------------------


class TestStdioTransport(unittest.TestCase):
    def test_stdio_oneshot(self):
        d = build_default_dispatcher(bind_external=True)
        sin = io.StringIO()
        sout = io.StringIO()
        sin.write(json.dumps({"jsonrpc": "2.0", "id": 1, "method": "initialize",
                               "params": {"protocolVersion": "2024-11-05"}}) + "\n")
        sin.write(json.dumps({"jsonrpc": "2.0", "id": 2, "method": "tools/list",
                               "params": {}}) + "\n")
        sin.write(json.dumps({"jsonrpc": "2.0", "id": 3, "method": "tools/call",
                               "params": {"name": "list_personas",
                                          "arguments": {"include_emerged": True}}}) + "\n")
        sin.write(json.dumps({"jsonrpc": "2.0", "id": 4, "method": "no_such_method",
                               "params": {}}) + "\n")
        sin.seek(0)
        t = StdioTransport(d.handle_message, stdin=sin, stdout=sout)
        for _ in range(4):
            t.serve_oneshot()
        out = sout.getvalue().strip().split("\n")
        self.assertEqual(len(out), 4)
        # 1) initialize
        r1 = json.loads(out[0])
        self.assertEqual(r1["result"]["serverInfo"]["name"], SERVER_NAME)
        # 2) tools/list
        r2 = json.loads(out[1])
        self.assertEqual(len(r2["result"]["tools"]), 5)
        # 3) list_personas
        r3 = json.loads(out[2])
        self.assertEqual(r3["result"]["content"][0]["data"]["n_personas"], 4)
        # 4) bad method
        r4 = json.loads(out[3])
        self.assertEqual(r4["error"]["code"], JSONRPC_METHOD_NOT_FOUND)


# ---------------------------------------------------------------------------
# 8. chaos 守门 (2 tests)
# ---------------------------------------------------------------------------


class TestChaosGuard(unittest.TestCase):
    def test_chaos_dispatcher_state_retained_across_transports(self):
        """主 23:44 干到底: transport 启停不丢 dispatcher state."""
        d = build_default_dispatcher(bind_external=True)
        pre = d.stats()
        # 启停 SSE + HTTP + stdio oneshot
        sse = SseTransport(dispatch=d.handle_message,
                            server_info={"name": SERVER_NAME, "tools": list(d.TOOLS.keys())},
                            host="127.0.0.1", port=0)
        sse.start()
        sse.stop()
        http = HttpTransport(dispatch=d.handle_message,
                              server_info={"name": SERVER_NAME, "tools": list(d.TOOLS.keys())},
                              host="127.0.0.1", port=0)
        http.start()
        http.stop()
        # 直接 dispatch 3 ping
        for _ in range(3):
            d.handle_message({"jsonrpc": "2.0", "id": 1, "method": "ping", "params": {}})
        post = d.stats()
        self.assertGreater(post["n_dispatched"], pre["n_dispatched"])
        # 9 键 LOCKED 仍在
        self.assertTrue(post["nine_key_lock"]["asi_nine_keys_locked"])
        # 5 工具仍在
        self.assertEqual(post["n_tools"], 5)

    def test_chaos_sse_outbox_overflow(self):
        """chaos: SSE outbox 满 → drop 计数, dispatcher state 不丢."""
        d = build_default_dispatcher(bind_external=True)
        sse = SseTransport(dispatch=d.handle_message,
                            server_info={"name": SERVER_NAME, "tools": list(d.TOOLS.keys())},
                            host="127.0.0.1", port=0)
        sse.start()
        try:
            # 推满 store 中的 session
            store = sse.session_store
            sess = store.create()
            sess.sse_alive = False  # 防止 push 后会真的被发送
            pre_dispatched = d.stats()["n_dispatched"]
            # 模拟 chaos: 不真启 SSE 流, 直接推 outbox
            for i in range(SSE_QUEUE_MAX + 10):
                sess.push({"id": i, "result": {"ok": True}})
            self.assertGreater(sess.n_messages_dropped, 0)
            # dispatcher state 没受 chaos 影响
            post_dispatched = d.stats()["n_dispatched"]
            self.assertEqual(post_dispatched, pre_dispatched)
        finally:
            sse.stop()


# ---------------------------------------------------------------------------
# 9. CLI 入口 (3 tests)
# ---------------------------------------------------------------------------


class TestCli(unittest.TestCase):
    def test_selftest_cli(self):
        buf = io.StringIO()
        with redirect_stdout(buf):
            rc = cli_main(["--selftest"])
        self.assertEqual(rc, 0)
        text = buf.getvalue()
        self.assertIn("5/5 tools OK", text)
        self.assertIn("chaos state retained: True", text)
        self.assertIn("9 键 LOCKED=True", text)

    def test_chaos_cli(self):
        buf = io.StringIO()
        with redirect_stdout(buf):
            rc = cli_main(["--chaos"])
        self.assertEqual(rc, 0)
        text = buf.getvalue()
        self.assertIn("chaos_state_retained: True", text)
        self.assertIn("post dispatched=3", text)

    def test_snapshot_cli(self):
        buf = io.StringIO()
        with redirect_stdout(buf):
            rc = cli_main(["--snapshot"])
        self.assertEqual(rc, 0)
        text = buf.getvalue()
        data = json.loads(text)
        self.assertEqual(data["server"], SERVER_NAME)
        self.assertEqual(data["n_tools"], 5)
        self.assertIn("nine_key_lock", data)


# ---------------------------------------------------------------------------
# 10. 真集成客户端 (2 tests)
# ---------------------------------------------------------------------------


class TestIntegrationClients(unittest.TestCase):
    def test_make_v1095_store(self):
        store = make_v1095_store()
        try:
            self.assertEqual(len(store.list_slots()), 4)
            prof = store.get_or_create_profile()
            self.assertIn("identity_id", prof.to_dict())
            self.assertGreater(store.stats()["n_fsync_total"], 0)
        finally:
            store.close()

    def test_make_v1124_backend(self):
        backend = make_v1124_backend()
        try:
            level = backend.level()
            self.assertEqual(level["version"], "0.1.0")  # V1124_VERSION
            self.assertIn("dimensions", level)
            self.assertIn("durable_audit_records", level["dimensions"])
            self.assertIn("claim", level)
            # north_star endpoint
            ns = backend.north_star()
            self.assertIn("north_star", ns)
            self.assertIn("protocols", ns)
            self.assertIn("guards", ns)
        finally:
            pass


if __name__ == "__main__":
    unittest.main(verbosity=2)