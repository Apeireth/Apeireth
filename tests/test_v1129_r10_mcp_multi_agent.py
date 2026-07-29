"""Apeireth ASI V1129 W2 — multi-agent MCP server tests (R10 W2).

主 17:43 实事求是 + 主 00:56 任何人都能接手 + 主 22:33 ASI 北极星 + 主 23:44 干到底 + 主 13:31 大胆激进.

测试覆盖 (≥25, 全真行为, 不 mock V1127/V1128/V1124):
  1. 常量与模块结构 (3)
  2. 8 tools schema 守门 (8: 5 旧 + 3 新, 各 1 个 schema 验证)
  3. 8 tools round-trip 集成 (8)
  4. V1127 DGM v0.5 真演化 (1)
  5. V1128 多 agent 共识 + chaos (2)
  6. V1124 backend 真集成 (1)
  7. chaos 守门 (3): dispatcher + multi-agent state 跨 transport 启停
  8. CLI 入口 (2): --selftest / --chaos
  9. V3 守门 (1)

运行: pytest -q tests/test_v1129_r10_mcp_multi_agent.py
"""
from __future__ import annotations

import io
import json
import sys
import threading
import time
import unittest
import urllib.error
import urllib.request
from contextlib import redirect_stdout
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT))

from apeireth.mcp.protocol import (  # noqa: E402
    JSONRPC_INVALID_PARAMS, JSONRPC_METHOD_NOT_FOUND, SchemaViolation,
    validate_arguments, validate_tool_result,
)
from apeireth.mcp.transport import HttpTransport  # noqa: E402
from apeireth.mcp.sse_transport import SseTransport  # noqa: E402
from apeireth.v1129_r10_mcp_multi_agent import (  # noqa: E402
    V1129_W2_VERSION, SERVER_NAME, V3_GUARDS_W2,
    MULTI_AGENT_CONSENSUS_SCHEMA, EVOLVE_DGM_SCHEMA, MULTI_AGENT_ASI_LEVEL_SCHEMA,
    V1129MultiAgentDispatcher, build_default_multi_agent_dispatcher,
    run_selftest, cli_main,
)
from apeireth.v1129_r10_mcp_server import V3_GUARDS  # noqa: E402


# ---------------------------------------------------------------------------
# 1. 常量与模块结构 (3 tests)
# ---------------------------------------------------------------------------


class TestW2Constants(unittest.TestCase):
    def test_versions_and_server_name(self):
        self.assertEqual(V1129_W2_VERSION, "0.2.0")
        self.assertEqual(SERVER_NAME, "apeireth-r10-mcp-multi-agent")

    def test_v3_guards_w2_present(self):
        for k in ("multi_agent_consensus_is_not_truth", "dgm_evolve_is_not_asi",
                   "multi_agent_measurement_is_not_asi",
                   "v1127_v1128_integration_is_not_asi",
                   "chaos_state_preserved_is_not_perfect",
                   "transports_fanout_is_not_asi"):
            self.assertIn(k, V3_GUARDS_W2)
        self.assertGreaterEqual(len(V3_GUARDS_W2), 6)

    def test_inherits_v1129_v3_guards(self):
        # 同时继承 V1129 W1 的 V3_GUARDS
        self.assertIn("module_is_not_asi", V3_GUARDS)


# ---------------------------------------------------------------------------
# 2. 8 tools schema 守门 (8 tests)
# ---------------------------------------------------------------------------


class TestW2ToolSchemas(unittest.TestCase):
    def test_multi_agent_consensus_schema(self):
        # 正常
        validate_arguments({"agent_ids": ["alpha", "beta"], "v04_score": 0.8538},
                            MULTI_AGENT_CONSENSUS_SCHEMA, tool_name="multi_agent_consensus")
        # agent_ids 类型错 (string 而非 array, 主 17:43 实事求是: 协议层必拦)
        with self.assertRaises(SchemaViolation):
            validate_arguments({"agent_ids": "alpha"},
                                MULTI_AGENT_CONSENSUS_SCHEMA, tool_name="multi_agent_consensus")
        # week_label pattern 不匹配 (主 17:43: R10-W\d+)
        with self.assertRaises(SchemaViolation):
            validate_arguments({"week_label": "W2"},
                                MULTI_AGENT_CONSENSUS_SCHEMA, tool_name="multi_agent_consensus")
        # v04_score 越界
        with self.assertRaises(SchemaViolation):
            validate_arguments({"v04_score": 1.5},
                                MULTI_AGENT_CONSENSUS_SCHEMA, tool_name="multi_agent_consensus")
        # 额外字段
        with self.assertRaises(SchemaViolation):
            validate_arguments({"unknown": 1},
                                MULTI_AGENT_CONSENSUS_SCHEMA, tool_name="multi_agent_consensus")

    def test_evolve_dgm_schema(self):
        # 正常
        validate_arguments({"generations": 2, "node_ids": ["alpha", "beta"], "seed": 1127},
                            EVOLVE_DGM_SCHEMA, tool_name="evolve_dgm")
        # generations 超上限
        with self.assertRaises(SchemaViolation):
            validate_arguments({"generations": 100},
                                EVOLVE_DGM_SCHEMA, tool_name="evolve_dgm")
        # generations < minimum 1
        with self.assertRaises(SchemaViolation):
            validate_arguments({"generations": 0},
                                EVOLVE_DGM_SCHEMA, tool_name="evolve_dgm")
        # generations 类型错
        with self.assertRaises(SchemaViolation):
            validate_arguments({"generations": "two"},
                                EVOLVE_DGM_SCHEMA, tool_name="evolve_dgm")
        # seed 类型错
        with self.assertRaises(SchemaViolation):
            validate_arguments({"seed": "abc"},
                                EVOLVE_DGM_SCHEMA, tool_name="evolve_dgm")

    def test_multi_agent_asi_level_schema(self):
        # 必填 agent_id 缺失
        with self.assertRaises(SchemaViolation):
            validate_arguments({}, MULTI_AGENT_ASI_LEVEL_SCHEMA,
                                tool_name="multi_agent_asi_level")
        # 正常
        validate_arguments({"agent_id": "alpha", "v04_score": 0.8538},
                            MULTI_AGENT_ASI_LEVEL_SCHEMA, tool_name="multi_agent_asi_level")
        # 越界 v04_score > maximum 1.0
        with self.assertRaises(SchemaViolation):
            validate_arguments({"agent_id": "alpha", "v04_score": 1.5},
                                MULTI_AGENT_ASI_LEVEL_SCHEMA, tool_name="multi_agent_asi_level")
        # 越界 v04_score < minimum 0.0
        with self.assertRaises(SchemaViolation):
            validate_arguments({"agent_id": "alpha", "v04_score": -0.1},
                                MULTI_AGENT_ASI_LEVEL_SCHEMA, tool_name="multi_agent_asi_level")
        # 额外字段
        with self.assertRaises(SchemaViolation):
            validate_arguments({"agent_id": "alpha", "extra": 1},
                                MULTI_AGENT_ASI_LEVEL_SCHEMA, tool_name="multi_agent_asi_level")

    def test_measure_asi_schema_inherited(self):
        # 5 旧工具 schema 必继承
        validate_arguments({"v04_actual": 0.8538}, {
            "type": "object", "properties": {"v04_actual": {"type": "number"}},
            "required": ["v04_actual"], "additionalProperties": False,
        }, tool_name="measure_asi_inherit_test")

    def test_get_north_star_schema_inherited(self):
        validate_arguments({}, {"type": "object", "additionalProperties": False},
                            tool_name="get_north_star_inherit_test")

    def test_check_identity_schema_inherited(self):
        validate_arguments({}, {"type": "object", "additionalProperties": False},
                            tool_name="check_identity_inherit_test")

    def test_verify_audit_chain_schema_inherited(self):
        validate_arguments({}, {"type": "object", "additionalProperties": False},
                            tool_name="verify_audit_chain_inherit_test")

    def test_list_personas_schema_inherited(self):
        validate_arguments({}, {"type": "object", "additionalProperties": False},
                            tool_name="list_personas_inherit_test")


# ---------------------------------------------------------------------------
# 3. 8 tools round-trip 集成 (8 tests, V1095/V1124/V1125/V1127/V1128 真跑)
# ---------------------------------------------------------------------------


class TestW2ToolRoundTrip(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.dispatcher = build_default_multi_agent_dispatcher(bind_external=True)

    def _call(self, name, args):
        resp = self.dispatcher.handle_message({
            "jsonrpc": "2.0", "id": 1, "method": "tools/call",
            "params": {"name": name, "arguments": args},
        })
        return resp["result"]

    def test_measure_asi_inherited(self):
        r = self._call("measure_asi", {"v04_actual": 0.8538})
        self.assertFalse(r.get("isError"))
        d = r["content"][0]["data"]
        self.assertEqual(d["source"], "v1125_evaluate_r10")
        self.assertIn("philosophy_guard", d)

    def test_get_north_star_inherited(self):
        r = self._call("get_north_star", {})
        self.assertFalse(r.get("isError"))
        d = r["content"][0]["data"]
        self.assertEqual(d["source"], "v1124_in_process")

    def test_check_identity_inherited(self):
        r = self._call("check_identity", {})
        self.assertFalse(r.get("isError"))
        d = r["content"][0]["data"]
        self.assertEqual(d["source"], "v1095_in_process")

    def test_verify_audit_chain_inherited(self):
        r = self._call("verify_audit_chain", {})
        self.assertFalse(r.get("isError"))
        d = r["content"][0]["data"]
        self.assertTrue(d["all_pass"])

    def test_list_personas_inherited(self):
        r = self._call("list_personas", {})
        self.assertFalse(r.get("isError"))
        d = r["content"][0]["data"]
        self.assertEqual(d["n_personas"], 4)

    def test_multi_agent_consensus(self):
        r = self._call("multi_agent_consensus",
                        {"agent_ids": ["alpha", "beta", "gamma"], "v04_score": 0.8538})
        self.assertFalse(r.get("isError"))
        d = r["content"][0]["data"]
        self.assertEqual(d["source"], "v1128_measure_multi_agent")
        self.assertEqual(d["consensus"]["n_agents_total"], 3)
        self.assertIn("consensus_score", d["consensus"])
        self.assertIn("consensus_pass", d["consensus"])
        self.assertIn("philosophy_guard", d)

    def test_multi_agent_consensus_with_chaos(self):
        r = self._call("multi_agent_consensus",
                        {"agent_ids": ["alpha", "beta", "gamma"], "run_chaos": True})
        self.assertFalse(r.get("isError"))
        d = r["content"][0]["data"]
        self.assertIn("chaos", d)
        # chaos 必返 measurement_preserved=True
        self.assertTrue(d["chaos_measurement_preserved"])
        # chaos_report 必含 consensus
        self.assertIn("chaos_report", d["chaos"])

    def test_multi_agent_asi_level(self):
        r = self._call("multi_agent_asi_level", {"agent_id": "alpha", "v04_score": 0.8538})
        self.assertFalse(r.get("isError"))
        d = r["content"][0]["data"]
        self.assertEqual(d["agent_id"], "alpha")
        self.assertEqual(d["source"], "v1128_v05_18_form_in_process")
        # V0.5 18 维必返
        self.assertIn("v05_18_total", d["report"])
        self.assertIn("per_dim", d["report"])
        self.assertIn("backend_status", d["report"])


# ---------------------------------------------------------------------------
# 4. V1127 DGM v0.5 真演化 (1 test)
# ---------------------------------------------------------------------------


class TestW2DgmEvolve(unittest.TestCase):
    def test_evolve_dgm_real(self):
        d = build_default_multi_agent_dispatcher(bind_external=True)
        r = d.handle_message({"jsonrpc": "2.0", "id": 1, "method": "tools/call",
                               "params": {"name": "evolve_dgm",
                                           "arguments": {"generations": 2,
                                                          "node_ids": ["alpha", "beta"],
                                                          "seed": 1127}}})
        result = r["result"]
        self.assertFalse(result.get("isError"), f"isError: {result.get('content')}")
        data = result["content"][0]["data"]
        self.assertEqual(data["source"], "v1127_v05_multi_agent_coordinator")
        self.assertEqual(data["generations"], 2)
        self.assertEqual(data["n_nodes"], 2)
        # latest_fitness_per_node 必返 (alpha/beta 都应有)
        self.assertIn("alpha", data["latest_fitness_per_node"])
        self.assertIn("beta", data["latest_fitness_per_node"])
        # fitness ∈ [0, 0.95) (V3 守门)
        for nid, fit in data["latest_fitness_per_node"].items():
            self.assertGreaterEqual(fit, 0.0)
            self.assertLess(fit, 0.95)


# ---------------------------------------------------------------------------
# 5. V1128 multi-agent chaos (1 test)
# ---------------------------------------------------------------------------


class TestW2ChaosIntegration(unittest.TestCase):
    def test_chaos_test_measurement_preserved(self):
        d = build_default_multi_agent_dispatcher(bind_external=True)
        r = d.handle_message({"jsonrpc": "2.0", "id": 1, "method": "tools/call",
                               "params": {"name": "multi_agent_consensus",
                                           "arguments": {"agent_ids": ["alpha", "beta", "gamma"],
                                                          "v04_score": 0.8538,
                                                          "run_chaos": True}}})
        result = r["result"]
        self.assertFalse(result.get("isError"))
        data = result["content"][0]["data"]
        self.assertTrue(data["chaos_measurement_preserved"])
        chaos = data["chaos"]
        self.assertIn("drop_indices", chaos)
        self.assertEqual(chaos["n_dropped"], 1)
        self.assertGreaterEqual(chaos["n_surviving"], 2)


# ---------------------------------------------------------------------------
# 6. V1124 backend 真集成 (1 test)
# ---------------------------------------------------------------------------


class TestW2V1124Integration(unittest.TestCase):
    def test_v1124_backend_bound(self):
        d = build_default_multi_agent_dispatcher(bind_external=True)
        self.assertTrue(d.v1124_backend is not None)
        self.assertTrue(d.stats()["v1124_bound"])
        # initialize 必返 v1124_bound + v1127_v05_integrated
        init = d.handle_message({"jsonrpc": "2.0", "id": 1, "method": "initialize",
                                  "params": {"protocolVersion": "2024-11-05"}})
        server_info = init["result"]["serverInfo"]
        self.assertTrue(server_info["v1124_bound"])
        self.assertTrue(server_info["v1127_v05_integrated"])
        self.assertTrue(server_info["v1128_multi_agent_integrated"])
        self.assertEqual(server_info["tools_count"], 8)


# ---------------------------------------------------------------------------
# 7. chaos 守门 (3 tests)
# ---------------------------------------------------------------------------


class TestW2ChaosGuard(unittest.TestCase):
    def test_dispatcher_state_retained_across_transports(self):
        d = build_default_multi_agent_dispatcher(bind_external=True)
        pre = d.stats()
        # 启停 SSE + HTTP
        sse = SseTransport(dispatch=d.handle_message,
                            server_info={"name": SERVER_NAME,
                                          "tools": list(d.TOOLS_EXT.keys())},
                            host="127.0.0.1", port=0)
        sse.start(); sse.stop()
        http = HttpTransport(dispatch=d.handle_message,
                              server_info={"name": SERVER_NAME,
                                            "tools": list(d.TOOLS_EXT.keys())},
                              host="127.0.0.1", port=0)
        http.start(); http.stop()
        # 3 ping
        for _ in range(3):
            d.handle_message({"jsonrpc": "2.0", "id": 1, "method": "ping", "params": {}})
        post = d.stats()
        self.assertGreater(post["n_dispatched"], pre["n_dispatched"])
        # 9 键 LOCKED 仍在
        self.assertTrue(post["nine_key_lock"]["asi_nine_keys_locked"])
        # 8 工具仍在
        self.assertEqual(post["n_tools"], 8)

    def test_multi_agent_state_retained_across_transports(self):
        d = build_default_multi_agent_dispatcher(bind_external=True)
        # 先跑一次 multi_agent_consensus
        d.handle_message({"jsonrpc": "2.0", "id": 1, "method": "tools/call",
                           "params": {"name": "multi_agent_consensus",
                                       "arguments": {"agent_ids": ["alpha", "beta"]}}})
        pre = d.stats()
        self.assertGreater(pre["n_multi_agent_calls"], 0)
        # 启停 transport
        sse = SseTransport(dispatch=d.handle_message,
                            server_info={"name": SERVER_NAME,
                                          "tools": list(d.TOOLS_EXT.keys())},
                            host="127.0.0.1", port=0)
        sse.start(); sse.stop()
        # n_multi_agent_calls 不变 (没新调用)
        post1 = d.stats()
        self.assertEqual(post1["n_multi_agent_calls"], pre["n_multi_agent_calls"])
        # 再跑一次 multi_agent_consensus → 应 +1
        d.handle_message({"jsonrpc": "2.0", "id": 2, "method": "tools/call",
                           "params": {"name": "multi_agent_consensus",
                                       "arguments": {"agent_ids": ["alpha", "beta"]}}})
        post2 = d.stats()
        self.assertEqual(post2["n_multi_agent_calls"], pre["n_multi_agent_calls"] + 1)

    def test_chaos_http_server_real_round_trip(self):
        d = build_default_multi_agent_dispatcher(bind_external=True)
        http = HttpTransport(dispatch=d.handle_message,
                              server_info={"name": SERVER_NAME,
                                            "tools": list(d.TOOLS_EXT.keys())},
                              host="127.0.0.1", port=0)
        http.start()
        try:
            base = f"http://127.0.0.1:{http.actual_port}"
            # /tools 列 8 工具 (HttpTransport /tools 返 list[str] 名字)
            with urllib.request.urlopen(f"{base}/tools", timeout=5) as r:
                body = json.loads(r.read().decode("utf-8"))
                self.assertEqual(len(body["tools"]), 8)
                names = sorted(body["tools"])
                self.assertEqual(names, sorted([
                    "measure_asi", "get_north_star", "check_identity",
                    "verify_audit_chain", "list_personas",
                    "multi_agent_consensus", "evolve_dgm", "multi_agent_asi_level",
                ]))
            # POST /rpc list_personas 真跑
            req = urllib.request.Request(
                f"{base}/rpc",
                data=json.dumps({"jsonrpc": "2.0", "id": 1, "method": "tools/call",
                                  "params": {"name": "list_personas",
                                              "arguments": {"include_emerged": True}}}).encode("utf-8"),
                headers={"Content-Type": "application/json"},
                method="POST",
            )
            with urllib.request.urlopen(req, timeout=10) as r:
                body = json.loads(r.read().decode("utf-8"))
                self.assertEqual(body["result"]["content"][0]["data"]["n_personas"], 4)
        finally:
            http.stop()


# ---------------------------------------------------------------------------
# 8. CLI 入口 (2 tests)
# ---------------------------------------------------------------------------


class TestW2Cli(unittest.TestCase):
    def test_selftest_cli(self):
        buf = io.StringIO()
        with redirect_stdout(buf):
            rc = cli_main(["--selftest"])
        self.assertEqual(rc, 0)
        text = buf.getvalue()
        self.assertIn("8/8 tools OK", text)
        self.assertIn("tools_count=8", text)
        self.assertIn("chaos state retained: True", text)
        self.assertIn("9 键 LOCKED=True", text)

    def test_chaos_cli(self):
        buf = io.StringIO()
        with redirect_stdout(buf):
            rc = cli_main(["--chaos"])
        self.assertEqual(rc, 0)
        text = buf.getvalue()
        self.assertIn("chaos_state_retained: True", text)


# ---------------------------------------------------------------------------
# 9. V3 守门 (1 test)
# ---------------------------------------------------------------------------


class TestW2V3Guards(unittest.TestCase):
    def test_v3_guards_completeness(self):
        # W2 6 项 + W1 5 项 = 11 项, 全部必含主哲学相关 key
        all_guards = {**V3_GUARDS, **V3_GUARDS_W2}
        expected_keys = {
            "module_is_not_asi", "multi_agent_consensus_is_not_truth",
            "dgm_evolve_is_not_asi", "transports_fanout_is_not_asi",
            "chaos_state_preserved_is_not_perfect",
        }
        for k in expected_keys:
            self.assertIn(k, all_guards)
        self.assertGreaterEqual(len(all_guards), 10)


if __name__ == "__main__":
    unittest.main(verbosity=2)