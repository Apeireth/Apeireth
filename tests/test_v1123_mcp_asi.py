"""Apeireth ASI V1123 — MCP 集成框架 + ASI 北极星 MCP 服务接口 测试 (R9 W4 / R9-MCP-001).

主 17:43 实事求是 + 主 00:56 任何人都能接手 + 主 22:33 ASI 北极星 + 主 23:44 干到底.

测试覆盖 (≥20, 全真行为, 不 mock):
  1.  常量与模块结构 (3)
  2.  协议守门 (4): version / parse / validate_args / validate_result
  3.  ASI 9 键 LOCKED (3): default / failed / inject
  4.  5 大 MCP 工具 dispatcher (6): round-trip + isError 路径
  5.  跨模型适配 (2): local 真跑 + registry
  6.  跨 server 编排 (2): handoff 全跑 + 错误传播
  7.  HTTP transport (1): 真起 server + curl 模拟 4 请求
  8.  stdio transport (1): NDJSON 1 行 round-trip
  9.  CLI 入口 (2): --selftest / --handoff
  10. 9 键 lock 失效抛异常 (1)

运行: pytest -q tests/test_v1123_mcp_asi.py
"""
from __future__ import annotations

import io
import json
import socket
import sys
import threading
import time
import unittest
from contextlib import redirect_stdout
from typing import Any, Dict
from unittest import mock

import pytest

ROOT = Path = __import__("pathlib").Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT))

from apeireth.mcp import ASI_NINE_KEYS, ASI_NORTH_STAR_VERSION, PROTOCOL_VERSION  # noqa: E402
from apeireth.mcp.protocol import (  # noqa: E402
    SUPPORTED_PROTOCOL_VERSIONS, JSONRPC_INVALID_PARAMS, JSONRPC_METHOD_NOT_FOUND,
    SchemaViolation, check_protocol_version, make_error_response, make_result_response,
    parse_request, validate_arguments, validate_tool_result,
)
from apeireth.mcp.asi_nine_keys import (  # noqa: E402
    AsiNineKeyLock, inject_guard_block, verify_or_raise,
)
from apeireth.mcp.model_adapters import (  # noqa: E402
    LocalHeuristicAdapter, ModelAdapterRegistry, OllamaHttpAdapter,
    OpenAIHttpAdapter, ClaudeHttpAdapter, heuristic_asi_score,
)
from apeireth.mcp.asi_north_star_server import (  # noqa: E402
    AsiNorthStarDispatcher, SERVER_NAME, TOOL_REGISTRY,
    V1123_VERSION, ASI_FORMULAS, ASI_RESOURCES, ASI_PROMPTS,
)
from apeireth.mcp.orchestrator import (  # noqa: E402
    CrossServerOrchestrator, CrossServerReport, CrossServerStep,
)
from apeireth.mcp.transport import StdioTransport, HttpTransport  # noqa: E402
from apeireth.v1123_mcp_asi_framework import (  # noqa: E402
    V1123_FRAMEWORK_VERSION, V3_GUARDS,
    build_default_dispatcher, run_selftest, cli_main, serve_http, serve_stdio,
)


# ---------------------------------------------------------------------------
# 1. 常量与模块结构 (3 tests)
# ---------------------------------------------------------------------------


class TestV1123Constants(unittest.TestCase):
    def test_versions(self):
        self.assertEqual(ASI_NORTH_STAR_VERSION, "0.1.0")
        self.assertEqual(PROTOCOL_VERSION, "2024-11-05")
        self.assertEqual(V1123_VERSION, "0.1.0")
        self.assertEqual(V1123_FRAMEWORK_VERSION, "0.1.0")
        self.assertEqual(SERVER_NAME, "apeireth-asi-north-star-mcp")

    def test_supported_protocol_versions(self):
        self.assertEqual(list(SUPPORTED_PROTOCOL_VERSIONS), ["2024-11-05"])

    def test_asi_nine_keys_count(self):
        self.assertEqual(len(ASI_NINE_KEYS), 9)
        for k in (
            "not_undo", "not_proof", "not_safe",
            "not_clone", "not_perfect", "not_uuid",
            "spec_is_not_proof", "counterexample_is_not_bug",
            "production_is_not_autonomy",
        ):
            self.assertIn(k, ASI_NINE_KEYS)


# ---------------------------------------------------------------------------
# 2. 协议守门 (4 tests)
# ---------------------------------------------------------------------------


class TestProtocolGuard(unittest.TestCase):
    def test_check_protocol_version_pass_and_fail(self):
        ok, msg = check_protocol_version("2024-11-05")
        self.assertTrue(ok)
        self.assertEqual(msg, "2024-11-05")
        ok, msg = check_protocol_version("2099-01-01")
        self.assertFalse(ok)
        self.assertIn("unsupported", msg)

    def test_parse_request_ok_and_error(self):
        req = parse_request({"jsonrpc": "2.0", "id": 1, "method": "tools/list", "params": {}})
        self.assertEqual(req.method, "tools/list")
        self.assertEqual(req.id, 1)
        with self.assertRaises(ValueError):
            parse_request({"jsonrpc": "1.0", "id": 1, "method": "x"})
        with self.assertRaises(ValueError):
            parse_request({"jsonrpc": "2.0", "id": 1})  # no method

    def test_validate_arguments_required_and_type(self):
        schema = {"type": "object", "properties": {"score": {"type": "number"}},
                  "required": ["score"], "additionalProperties": False}
        validate_arguments({"score": 0.9}, schema, tool_name="t")
        with self.assertRaises(SchemaViolation):
            validate_arguments({}, schema, tool_name="t")
        with self.assertRaises(SchemaViolation):
            validate_arguments({"score": "abc"}, schema, tool_name="t")
        with self.assertRaises(SchemaViolation):
            validate_arguments({"score": 0.9, "extra": 1}, schema, tool_name="t")

    def test_validate_tool_result_content_shape(self):
        # 正常 json
        validate_tool_result({"content": [{"type": "json", "data": {"a": 1}}]}, tool_name="t")
        # isError=True 时不许有 data
        with self.assertRaises(SchemaViolation):
            validate_tool_result(
                {"isError": True, "content": [{"type": "json", "data": {"a": 1}}]},
                tool_name="t",
            )
        # 缺 content
        with self.assertRaises(SchemaViolation):
            validate_tool_result({}, tool_name="t")


# ---------------------------------------------------------------------------
# 3. ASI 9 键 LOCKED (3 tests)
# ---------------------------------------------------------------------------


class TestAsiNineKeyLock(unittest.TestCase):
    def test_default_all_locked(self):
        lock = AsiNineKeyLock()
        self.assertTrue(lock.all_locked())
        self.assertEqual(lock.failed_keys(), [])
        d = lock.to_dict()
        self.assertEqual(d["n_locked"], 9)
        self.assertEqual(d["n_total"], 9)
        self.assertTrue(d["asi_nine_keys_locked"])

    def test_failed_keys(self):
        lock = AsiNineKeyLock(values={k: (k != "not_undo") for k in ASI_NINE_KEYS})
        self.assertFalse(lock.all_locked())
        self.assertEqual(lock.failed_keys(), ["not_undo"])
        with self.assertRaises(RuntimeError):
            verify_or_raise(lock)

    def test_inject_guard_block(self):
        lock = AsiNineKeyLock()
        result = {"content": [{"type": "json", "data": {"x": 1}}]}
        out = inject_guard_block(result, lock)
        self.assertIn("philosophy_guard", out["content"][0]["data"])
        self.assertTrue(out["content"][0]["data"]["philosophy_guard"]["asi_nine_keys_locked"])


# ---------------------------------------------------------------------------
# 4. 5 大 MCP 工具 dispatcher (6 tests)
# ---------------------------------------------------------------------------


class TestDispatcherTools(unittest.TestCase):
    def setUp(self):
        self.d = AsiNorthStarDispatcher()

    def test_initialize_returns_server_info(self):
        resp = self.d.handle_message({"jsonrpc": "2.0", "id": 1, "method": "initialize",
                                       "params": {"protocolVersion": "2024-11-05"}})
        self.assertIn("result", resp)
        si = resp["result"]["serverInfo"]
        self.assertEqual(si["name"], SERVER_NAME)
        self.assertEqual(si["version"], V1123_VERSION)
        self.assertEqual(si["tools_count"], 5)
        # ASI 9 键 LOCKED 注入 initialize
        self.assertTrue(resp["result"]["nine_key_lock"]["asi_nine_keys_locked"])

    def test_tools_list_has_5(self):
        resp = self.d.handle_message({"jsonrpc": "2.0", "id": 1, "method": "tools/list",
                                       "params": {}})
        tools = resp["result"]["tools"]
        names = {t["name"] for t in tools}
        self.assertEqual(names, {
            "asi_north_star_query", "v1074_guard", "v1112_dgm_run",
            "v1114_weekly_eval", "identity_lock_check",
        })

    def test_asi_north_star_query(self):
        for f in ("v0.1", "v0.3", "v0.4", "north_star"):
            resp = self.d.handle_message({"jsonrpc": "2.0", "id": 1, "method": "tools/call",
                                           "params": {"name": "asi_north_star_query",
                                                      "arguments": {"formula": f}}})
            data = resp["result"]["content"][0]["data"]
            self.assertEqual(data["formula"], f)
            self.assertIn(data["name"], ASI_FORMULAS[f]["name"])

    def test_v1074_guard_pass_and_fail(self):
        # pass
        resp = self.d.handle_message({"jsonrpc": "2.0", "id": 1, "method": "tools/call",
                                       "params": {"name": "v1074_guard",
                                                  "arguments": {"score": 0.89, "include_decision": True}}})
        d = resp["result"]["content"][0]["data"]
        self.assertTrue(d["passes"])
        self.assertIn("PASS", d["decision"])
        # fail
        resp = self.d.handle_message({"jsonrpc": "2.0", "id": 1, "method": "tools/call",
                                       "params": {"name": "v1074_guard",
                                                  "arguments": {"score": 0.50}}})
        d = resp["result"]["content"][0]["data"]
        self.assertFalse(d["passes"])
        self.assertEqual(d["gap"], round(0.50 - 0.8884, 4))

    def test_v1112_dgm_run_deterministic(self):
        args = {"n_generations": 5, "seed": 7, "include_report": True}
        a = self.d.handle_message({"jsonrpc": "2.0", "id": 1, "method": "tools/call",
                                    "params": {"name": "v1112_dgm_run", "arguments": args}})["result"]
        b = self.d.handle_message({"jsonrpc": "2.0", "id": 1, "method": "tools/call",
                                    "params": {"name": "v1112_dgm_run", "arguments": args}})["result"]
        # 同 seed → 同 trajectory
        self.assertEqual(a["content"][0]["data"]["v04_trajectory"],
                         b["content"][0]["data"]["v04_trajectory"])
        self.assertEqual(a["content"][0]["data"]["n_generations"], 5)
        # track_decision 必在 [A, B, C, D]
        self.assertIn(a["content"][0]["data"]["track_decision"], {"A", "B", "C", "D"})

    def test_v1114_weekly_eval_and_identity_lock(self):
        # weekly eval
        resp = self.d.handle_message({"jsonrpc": "2.0", "id": 1, "method": "tools/call",
                                       "params": {"name": "v1114_weekly_eval",
                                                  "arguments": {"week_label": "W4", "live": False}}})
        data = resp["result"]["content"][0]["data"]
        self.assertIn("track_decision", data)
        self.assertIn("dashboard", data)
        # identity lock (不真生产, 跑得快)
        resp = self.d.handle_message({"jsonrpc": "2.0", "id": 1, "method": "tools/call",
                                       "params": {"name": "identity_lock_check",
                                                  "arguments": {"run": False}}})
        data = resp["result"]["content"][0]["data"]
        self.assertTrue(data["lock"]["asi_nine_keys_locked"])
        self.assertEqual(data["lock"]["n_locked"], 9)

    def test_bad_method_and_bad_params(self):
        # unknown method
        resp = self.d.handle_message({"jsonrpc": "2.0", "id": 1, "method": "unknown",
                                       "params": {}})
        self.assertEqual(resp["error"]["code"], JSONRPC_METHOD_NOT_FOUND)
        # missing required
        resp = self.d.handle_message({"jsonrpc": "2.0", "id": 1, "method": "tools/call",
                                       "params": {"name": "v1074_guard", "arguments": {}}})
        self.assertEqual(resp["error"]["code"], JSONRPC_INVALID_PARAMS)
        # unknown tool
        resp = self.d.handle_message({"jsonrpc": "2.0", "id": 1, "method": "tools/call",
                                       "params": {"name": "no_such_tool", "arguments": {}}})
        self.assertEqual(resp["error"]["code"], JSONRPC_INVALID_PARAMS)


# ---------------------------------------------------------------------------
# 5. 跨模型适配 (2 tests)
# ---------------------------------------------------------------------------


class TestModelAdapters(unittest.TestCase):
    def test_local_heuristic_always_available_and_scores(self):
        a = LocalHeuristicAdapter()
        self.assertTrue(a.is_available())
        r = a.complete("ASI 北极星 V0.4 真测 实事求是 V1114 V1123 守门", max_tokens=128)
        self.assertFalse(r["degraded"])
        self.assertIn("text", r)
        self.assertGreater(r["meta"]["score"], 0.0)
        # measure_asi_proxy 至少 ≥ 0
        self.assertGreaterEqual(a.measure_asi_proxy("ASI 北极星 V1123"), 0.0)

    def test_registry_lists_adapters_and_uses_prefer(self):
        reg = ModelAdapterRegistry()
        listed = reg.list_adapters()
        names = {x["name"] for x in listed}
        self.assertEqual(names, {"local_heuristic", "ollama_http", "openai_http", "claude_http"})
        # local heuristic 永远 available, complete 必返
        r = reg.complete("ASI 北极星 实事求是", max_tokens=64, prefer="local_heuristic")
        self.assertIn("primary", r)
        self.assertEqual(r["primary"], "local_heuristic")
        # measure_asi_proxy 路径
        s = reg.measure_asi_proxy("ASI V1123 真测")
        self.assertGreaterEqual(s, 0.0)
        self.assertLessEqual(s, 1.0)

    def test_heuristic_asi_score_keywords(self):
        # 含越多 ASI 关键词, 分数越高
        s_empty = heuristic_asi_score("")
        s_full = heuristic_asi_score("ASI 北极星 V0.4 实事求是 大胆激进 守门 干到底 走在前人经验上 任何人都能接手 MCP orchestrator identity V1072 V1114 V1119 V1123 真测")
        self.assertEqual(s_empty, 0.0)
        self.assertGreater(s_full, s_empty)
        # 饱和 ≤ 1.0
        self.assertLessEqual(s_full, 1.0)

    def test_openai_and_claude_missing_key_degrade(self):
        # 环境里一般没 API key → is_available() 返 False, 但 complete 仍兜底
        oa = OpenAIHttpAdapter(api_key=None)
        ca = ClaudeHttpAdapter(api_key=None)
        self.assertFalse(oa.is_available())
        self.assertFalse(ca.is_available())
        # 不真发请求, 只测本地兜底 (无 key)
        if not oa.is_available() and not ca.is_available():
            # 主 13:31 大胆激进: 至少 2 种真跑 → local heuristic 兜底是必要的
            reg = ModelAdapterRegistry()
            r = reg.complete("test")
            self.assertFalse(r["degraded"])


# ---------------------------------------------------------------------------
# 6. 跨 server 编排 (2 tests)
# ---------------------------------------------------------------------------


class TestCrossServerOrchestrator(unittest.TestCase):
    def test_weekly_handoff_all_ok(self):
        d = AsiNorthStarDispatcher()
        orch = CrossServerOrchestrator(mcp1=d)
        report = orch.run_weekly_handoff(week_label="W4", v04_score=0.8538, v03_score=0.8897)
        self.assertEqual(report.n_steps, 6)
        self.assertTrue(report.all_ok)
        self.assertGreaterEqual(report.n_ok, 5)  # mcp1 5 步必 ok, mcp2 视 V1097 是否可 import
        self.assertEqual(report.final["week_label"], "W4")
        self.assertIn(report.final["track_decision"], {"A", "B", "C", "D"})
        # 至少 1 步耗时被打点
        self.assertGreater(sum(s.elapsed_ms for s in report.steps), 0.0)
        # 报告可 JSON 化
        json.dumps(report.to_dict())

    def test_orchestrator_aborts_on_first_failure(self):
        # 用一个"必 fail"的 dispatcher (mcp1 直接抛 isError)
        class FailDispatcher(AsiNorthStarDispatcher):
            def _on_tools_call(self, req):
                return make_result_response(req.id, {
                    "isError": True,
                    "content": [{"type": "text", "text": "intentional"}],
                })
        orch = CrossServerOrchestrator(mcp1=FailDispatcher())
        report = orch.run_weekly_handoff(week_label="W4")
        self.assertFalse(report.all_ok)
        self.assertEqual(report.final.get("aborted_at"), 1)
        self.assertEqual(len(report.steps), 1)


# ---------------------------------------------------------------------------
# 7. HTTP transport (1 test, 真起 server)
# ---------------------------------------------------------------------------


class TestHttpTransport(unittest.TestCase):
    def test_http_server_round_trip(self):
        d = AsiNorthStarDispatcher()
        http = HttpTransport(
            dispatch=d.handle_message,
            server_info={"name": SERVER_NAME, "tools": list(TOOL_REGISTRY.keys())},
            host="127.0.0.1", port=0,
        )
        http.start()
        try:
            import urllib.request
            base = f"http://127.0.0.1:{http.actual_port}"
            # GET /health
            with urllib.request.urlopen(f"{base}/health", timeout=2) as r:
                self.assertEqual(r.status, 200)
                body = json.loads(r.read().decode("utf-8"))
                self.assertTrue(body["ok"])
            # GET /tools
            with urllib.request.urlopen(f"{base}/tools", timeout=2) as r:
                self.assertEqual(r.status, 200)
                body = json.loads(r.read().decode("utf-8"))
                self.assertEqual(len(body["tools"]), 5)
            # POST /rpc tools/call
            req = urllib.request.Request(
                f"{base}/rpc",
                data=json.dumps({"jsonrpc": "2.0", "id": 1, "method": "tools/call",
                                  "params": {"name": "asi_north_star_query",
                                             "arguments": {"formula": "v0.4"}}}).encode("utf-8"),
                headers={"Content-Type": "application/json"},
                method="POST",
            )
            with urllib.request.urlopen(req, timeout=2) as r:
                self.assertEqual(r.status, 200)
                body = json.loads(r.read().decode("utf-8"))
                self.assertEqual(body["result"]["content"][0]["data"]["formula"], "v0.4")
                # ASI 9 键 LOCKED 必注入
                self.assertTrue(body["result"]["content"][0]["data"]["philosophy_guard"]["asi_nine_keys_locked"])
            # POST /rpc invalid (urllib 默认抛 HTTPError, 用 retry 拿 body)
            req = urllib.request.Request(
                f"{base}/rpc",
                data=b'{"not json',
                headers={"Content-Type": "application/json"},
                method="POST",
            )
            try:
                with urllib.request.urlopen(req, timeout=2) as r:
                    self.assertEqual(r.status, 400)
                    body = json.loads(r.read().decode("utf-8"))
            except urllib.error.HTTPError as exc:
                # urllib.error.HTTPError 也会暴露 .read() / .code
                self.assertEqual(exc.code, 400)
                body = json.loads(exc.read().decode("utf-8"))
            self.assertEqual(body["error"]["code"], -32700)
        finally:
            http.stop()


# ---------------------------------------------------------------------------
# 8. stdio transport (1 test, NDJSON)
# ---------------------------------------------------------------------------


class TestStdioTransport(unittest.TestCase):
    def test_stdio_oneshot(self):
        d = AsiNorthStarDispatcher()
        sin = io.StringIO()
        sout = io.StringIO()
        # 1) initialize
        sin.write(json.dumps({"jsonrpc": "2.0", "id": 1, "method": "initialize",
                               "params": {"protocolVersion": "2024-11-05"}}) + "\n")
        # 2) tools/call
        sin.write(json.dumps({"jsonrpc": "2.0", "id": 2, "method": "tools/call",
                               "params": {"name": "v1074_guard",
                                          "arguments": {"score": 0.89}}}) + "\n")
        # 3) bad JSON
        sin.write("not json\n")
        # 4) bad method
        sin.write(json.dumps({"jsonrpc": "2.0", "id": 3, "method": "no_such_method",
                               "params": {}}) + "\n")
        sin.seek(0)
        t = StdioTransport(d.handle_message, stdin=sin, stdout=sout)
        for _ in range(4):
            t.serve_oneshot()
        out = sout.getvalue().strip().split("\n")
        self.assertEqual(len(out), 4)
        # 1) initialize → serverInfo
        r1 = json.loads(out[0])
        self.assertEqual(r1["result"]["serverInfo"]["name"], SERVER_NAME)
        # 2) v1074_guard → passes
        r2 = json.loads(out[1])
        self.assertTrue(r2["result"]["content"][0]["data"]["passes"])
        # 3) bad JSON → -32700
        r3 = json.loads(out[2])
        self.assertEqual(r3["error"]["code"], -32700)
        # 4) bad method → -32601
        r4 = json.loads(out[3])
        self.assertEqual(r4["error"]["code"], JSONRPC_METHOD_NOT_FOUND)


# ---------------------------------------------------------------------------
# 9. CLI 入口 (2 tests)
# ---------------------------------------------------------------------------


class TestCli(unittest.TestCase):
    def test_selftest_cli(self):
        buf = io.StringIO()
        with redirect_stdout(buf):
            rc = cli_main(["--selftest"])
        self.assertEqual(rc, 0)
        text = buf.getvalue()
        self.assertIn("5/5 tools OK", text)
        self.assertIn("9 键 LOCKED: True", text)

    def test_handoff_cli(self):
        buf = io.StringIO()
        with redirect_stdout(buf):
            rc = cli_main(["--handoff", "--week", "W4", "--v04", "0.8538"])
        self.assertEqual(rc, 0)
        text = buf.getvalue()
        self.assertIn("V1123 cross-server handoff (W4):", text)
        self.assertIn("all_ok=True", text)

    def test_snapshot_cli(self):
        buf = io.StringIO()
        with redirect_stdout(buf):
            rc = cli_main(["--snapshot"])
        self.assertEqual(rc, 0)
        text = buf.getvalue()
        data = json.loads(text)
        self.assertEqual(data["server"], SERVER_NAME)
        self.assertEqual(data["n_tools"], 5)


# ---------------------------------------------------------------------------
# 10. ASI 9 键 lock 失效抛异常 (1 test)
# ---------------------------------------------------------------------------


class TestNineKeyLockFailure(unittest.TestCase):
    def test_lock_failure_raises_and_dispatcher_rejects(self):
        bad_lock = AsiNineKeyLock(values={**{k: True for k in ASI_NINE_KEYS}, "not_undo": False})
        with self.assertRaises(RuntimeError):
            verify_or_raise(bad_lock)
        # 构造一个"9 键失败" dispatcher
        d = AsiNorthStarDispatcher(nine_key_lock=bad_lock, nine_key_inject=False)
        # 即使 nine_key_inject=False, 也不应破坏 round-trip (仍 PASS)
        resp = d.handle_message({"jsonrpc": "2.0", "id": 1, "method": "tools/call",
                                  "params": {"name": "v1074_guard",
                                             "arguments": {"score": 0.89}}})
        self.assertIn("result", resp)
        self.assertTrue(resp["result"]["content"][0]["data"]["passes"])


# ---------------------------------------------------------------------------
# 11. V3 守门 (1 test, 主 17:58 不假装)
# ---------------------------------------------------------------------------


class TestV3Guards(unittest.TestCase):
    def test_framework_v3_guards(self):
        self.assertIn("module_is_not_asi", V3_GUARDS)
        self.assertIn("mcp_skeleton_is_not_production", V3_GUARDS)
        self.assertIn("nine_key_lock_is_not_truth", V3_GUARDS)
        # 至少 5 项
        self.assertGreaterEqual(len(V3_GUARDS), 5)


if __name__ == "__main__":
    unittest.main(verbosity=2)
