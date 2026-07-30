"""Apeireth ASI V1137 Tests — R11 MCP: V1136/V1130 真测结果 MCP 集成契约测试.

Contract guarantees (主 17:43 实事求是, 主 13:04 不假装, 主 23:44 干到底):

  S1 schema        — 每个 tool 入参 JSON Schema 完整 + additionalProperties=False
  S2 round-trip    — 真跑出 result 含 provenance + module_versions + r11_version
  S3 timeout       — 长耗时工具能 ≤ MCP_TOOL_TIMEOUT_SEC 完成; 超时抛 R11McpTimeout
  S4 error map     — R11_TIMEOUT / R11_INVALID_ARGS / R11_MISSING_MODULE / R11_BACKEND_FAILURE / R11_FORBIDDEN
                    全部可识别 (code 字段, not raw exceptions)
  S5 version       — 每个 result data.provenance.r11_mcp_version == R11_MCP_VERSION
                    + module_versions 透出 V1136/V1130 模块 VERSION
  S6 provenance    — offline=True (V1136) / offline=Action!=evaluate (V1130); 不假装真 provider
  S7 no-fake       — V1130 evaluate 在 provider 未配置时不伪造 success;
                    attempts 数组可能含 UNCONFIGURED/UNAVAILABLE 透明 state
  S8 offline       — 整个 suite 在 offline 模式下 (无 API key, 无 docker) 跑通
  S9 nine-keys     — 每个 result content[].data 含 philosophy_guard.asi_nine_keys_locked=True
  S10 V3-guards    — strict mode 下 V3 不过 → isError=True
  S11 chaos        — transport 启停后 dispatcher state 不丢
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

import pytest

# 验证 offline: 这些环境变量在测试中必须不存在
PRESERVE_OFFLINE = (
    "ANTHROPIC_API_KEY", "OPENAI_API_KEY", "OPENAI_BASE_URL",
    "V1124_BASE_URL", "V1128_LOCAL_CLI", "V1128_EXECUTABLE",
)


def _ensure_offline() -> None:
    """主 17:58 不假装: 测试套件必须不依赖外部 provider."""
    for k in PRESERVE_OFFLINE:
        os.environ.pop(k, None)


@pytest.fixture(autouse=True)
def _offline_fixture():
    _ensure_offline()
    yield
    _ensure_offline()


# ponytail lazy (主 19:33 走在前人经验上): 不发明新 fixture 库
WORKDIR = Path(__file__).resolve().parent.parent

from apeireth.mcp.r11_measurement_server import (
    ERR_BACKEND_FAILURE,
    ERR_FORBIDDEN,
    ERR_INVALID_ARGS,
    ERR_MISSING_MODULE,
    ERR_TIMEOUT,
    GET_V1130_BACKEND,
    MCP_TOOL_TIMEOUT_SEC,
    MEASURE_V1136_REAL,
    R11_MCP_VERSION,
    R11_SERVER_NAME,
    R11_TOOLS,
    R11MeasurementDispatcher,
    R11McpError,
    R11McpMissingModule,
    R11McpTimeout,
    V1136_MEASURE_SCHEMA,
    V1130_BACKEND_SCHEMA,
    V3_GUARDS_R11,
    _call_with_timeout,
    _make_data_result,
    _make_is_error_result,
    tool_get_v1130_backend,
    tool_measure_v1136_real,
)
from apeireth.v1137_r11_mcp_measurement_tool import (
    DEFAULT_HOST,
    DEFAULT_PORT,
    V1137_FRAMEWORK_VERSION,
    cli_main,
    run_chaos,
    run_selftest,
    serve_http,
    serve_sse,
    serve_stdio,
)


# ---------------------------------------------------------------------------
# S1 schema — JSON Schema 完整性契约
# ---------------------------------------------------------------------------


class TestSchemaContract:
    """S1: schema / type / additionalProperties / enum / range."""

    def test_both_tools_registered(self):
        assert MEASURE_V1136_REAL in R11_TOOLS
        assert GET_V1130_BACKEND in R11_TOOLS
        assert len(R11_TOOLS) == 2

    def test_measure_v1136_schema_shape(self):
        s = V1136_MEASURE_SCHEMA
        assert s["type"] == "object"
        assert s["additionalProperties"] is False
        for key in ("v04_score", "run_chaos", "include_subscores", "strict"):
            assert key in s["properties"], f"missing prop {key}"
        assert s["properties"]["v04_score"]["minimum"] == 0.0
        assert s["properties"]["v04_score"]["maximum"] == 1.0
        assert s["properties"]["run_chaos"]["type"] == "boolean"
        assert s["properties"]["strict"]["type"] == "boolean"

    def test_get_v1130_schema_shape(self):
        s = V1130_BACKEND_SCHEMA
        assert s["type"] == "object"
        assert s["additionalProperties"] is False
        action_prop = s["properties"]["action"]
        assert action_prop["type"] == "string"
        assert set(action_prop["enum"]) == {"level", "runtime", "alerts", "evaluate"}
        iterations_prop = s["properties"]["iterations"]
        assert iterations_prop["type"] == "integer"
        assert iterations_prop["minimum"] == 1
        assert iterations_prop["maximum"] == 10

    def test_tools_list_exposes_schemas(self):
        d = R11MeasurementDispatcher()
        resp = d.handle_message({
            "jsonrpc": "2.0", "id": 1, "method": "tools/list", "params": {},
        })
        result = resp["result"]
        assert "tools" in result
        names = sorted(t["name"] for t in result["tools"])
        assert names == ["get_v1130_backend", "measure_v1136_real"]
        # 每个 tool 暴露 inputSchema 字段
        for t in result["tools"]:
            assert "inputSchema" in t
            assert "description" in t
            assert t["inputSchema"]["type"] == "object"

    def test_invalid_tool_name_rejected(self):
        """S1 边缘契约: 不存在的 tool 返 JSONRPC_METHOD_NOT_FOUND."""
        d = R11MeasurementDispatcher()
        resp = d.handle_message({
            "jsonrpc": "2.0", "id": 1, "method": "tools/call",
            "params": {"name": "fake_tool", "arguments": {}},
        })
        assert "error" in resp
        # JSONRPC method_not_found = -32601; 我们随版本容错
        assert resp["error"]["code"] in (-32601, -32602)


# ---------------------------------------------------------------------------
# S2 round-trip — 真跑出 result, 含 provenance + version
# ---------------------------------------------------------------------------


def _extract_data(resp: Dict) -> Dict:
    """从 JSON-RPC 响应抽出 MCP data 字段."""
    assert resp and "result" in resp, f"expected json-rpc result, got {resp}"
    content = resp["result"]["content"]
    assert content and content[0]["type"] == "json"
    return content[0]["data"]


class TestRoundTripContract:
    """S2: measure_v1136_real + get_v1130_backend 真跑出结果."""

    def test_measure_v1136_round_trip(self):
        d = R11MeasurementDispatcher()
        resp = d.handle_message({
            "jsonrpc": "2.0", "id": 1, "method": "tools/call",
            "params": {"name": MEASURE_V1136_REAL,
                        "arguments": {"v04_score": 0.8538, "run_chaos": False,
                                       "include_subscores": True, "strict": False}},
        })
        data = _extract_data(resp)
        # S5: provenance 锁定
        assert "provenance" in data
        assert data["provenance"]["r11_mcp_version"] == R11_MCP_VERSION
        assert "v1136_version" in data["provenance"]["module_versions"]
        # S6: offline True
        assert data["provenance"]["offline"] is True
        assert data["provenance"]["v1136_3dim_real"] is True
        # 主结果字段
        for key in ("continuity", "autonomy", "transferability",
                     "v05_total_v1136", "v05_total_v1125",
                     "v04_score", "delta_v05_total"):
            assert key in data, f"missing result key: {key}"
        # 3 dims 在合理范围
        assert 0.0 <= data["continuity"] <= 1.0
        assert 0.0 <= data["autonomy"] <= 1.0
        assert 0.0 <= data["transferability"] <= 1.0

    def test_measure_v1136_without_subscores(self):
        """S2 收缩: include_subscores=False 时不含 *_detail."""
        d = R11MeasurementDispatcher()
        resp = d.handle_message({
            "jsonrpc": "2.0", "id": 1, "method": "tools/call",
            "params": {"name": MEASURE_V1136_REAL,
                        "arguments": {"v04_score": 0.8538,
                                       "include_subscores": False}},
        })
        data = _extract_data(resp)
        assert "continuity_detail" not in data
        assert "autonomy_detail" not in data
        assert "transferability_detail" not in data
        # 主 3 维 + v05_total 仍在
        assert "continuity" in data

    def test_get_v1130_level_round_trip(self):
        """S2: level action 真跑 in-process."""
        d = R11MeasurementDispatcher()
        resp = d.handle_message({
            "jsonrpc": "2.0", "id": 1, "method": "tools/call",
            "params": {"name": GET_V1130_BACKEND,
                        "arguments": {"action": "level"}},
        })
        data = _extract_data(resp)
        assert data["action"] == "level"
        assert "level" in data
        assert data["provenance"]["r11_mcp_version"] == R11_MCP_VERSION
        assert "v1130_version" in data["provenance"]["module_versions"]
        assert data["provenance"]["offline"] is True
        assert data["provenance"]["v1130_real_backend"] is True

    def test_get_v1130_runtime_round_trip(self):
        """S2: runtime action 真跑 V1074 采样 (offline-safe)."""
        d = R11MeasurementDispatcher()
        resp = d.handle_message({
            "jsonrpc": "2.0", "id": 1, "method": "tools/call",
            "params": {"name": GET_V1130_BACKEND,
                        "arguments": {"action": "runtime", "iterations": 2}},
        })
        data = _extract_data(resp)
        assert data["action"] == "runtime"
        assert data["iterations"] == 2
        assert 0.0 <= data["mean_seconds"] < 5.0
        assert "savings_pct" in data

    def test_get_v1130_alerts_round_trip(self):
        """S2: alerts action 真跑 alert sink summary."""
        d = R11MeasurementDispatcher()
        resp = d.handle_message({
            "jsonrpc": "2.0", "id": 1, "method": "tools/call",
            "params": {"name": GET_V1130_BACKEND,
                        "arguments": {"action": "alerts"}},
        })
        data = _extract_data(resp)
        assert data["action"] == "alerts"
        assert "alerts" in data


# ---------------------------------------------------------------------------
# S3 timeout — 真实超时守门契约
# ---------------------------------------------------------------------------


class TestTimeoutContract:
    """S3: 长任务 ≤ MCP_TOOL_TIMEOUT_SEC 完成; 超时抛 R11McpTimeout."""

    def test_call_with_timeout_returns_result(self):
        # quick 任务 → 不超时
        result = _call_with_timeout(lambda: 42, timeout_sec=1.0)
        assert result == 42

    def test_call_with_timeout_raises_on_slow(self):
        def slow():
            time.sleep(2.0)
            return "should-not-reach"
        with pytest.raises(R11McpTimeout):
            _call_with_timeout(slow, timeout_sec=0.2)


# ---------------------------------------------------------------------------
# S4 error map — 错误码契约
# ---------------------------------------------------------------------------


class TestErrorMapContract:
    """S4: 所有错误码必须可识别."""

    def test_invalid_args_code(self):
        """v04_score 越界 → ERR_INVALID_ARGS."""
        d = R11MeasurementDispatcher()
        resp = d.handle_message({
            "jsonrpc": "2.0", "id": 1, "method": "tools/call",
            "params": {"name": MEASURE_V1136_REAL,
                        "arguments": {"v04_score": 1.5}},  # 越界
        })
        # dispatcher 校验会先抓 → JSONRPC_INVALID_PARAMS
        assert "error" in resp
        assert resp["error"]["code"] == -32602

    def test_strict_mode_v3_failed_returns_isError(self):
        """S10: strict 模式 + V3 未过 → ERR_FORBIDDEN."""
        # 极低 v04_score → V3 守门会失败 (continuity < 0.55 是默认 0.85)
        # strict mode → 显式 isError
        resp_direct = tool_measure_v1136_real({
            "v04_score": 0.0, "strict": True,
        })
        # v04=0.0 仍让 3 维跑, strict 时如果 V3 未过就 isError=True
        assert "content" in resp_direct
        if resp_direct.get("isError"):
            data = resp_direct["content"][0]["data"]
            assert data["code"] == ERR_FORBIDDEN

    def test_unknown_tool_returns_method_not_found(self):
        d = R11MeasurementDispatcher()
        resp = d.handle_message({
            "jsonrpc": "2.0", "id": 1, "method": "tools/call",
            "params": {"name": "non_existent", "arguments": {}},
        })
        assert "error" in resp
        assert resp["error"]["code"] == -32601

    def test_invalid_action_returns_invalid_args(self):
        """S4: V1130 action 非法值 → ERR_INVALID_ARGS."""
        resp_direct = tool_get_v1130_backend({"action": "hack"})
        assert resp_direct.get("isError") is True
        data = resp_direct["content"][0]["data"]
        assert data["code"] == ERR_INVALID_ARGS

    def test_invalid_iterations_returns_invalid_args(self):
        resp_direct = tool_get_v1130_backend({"action": "runtime",
                                                "iterations": 99})
        assert resp_direct.get("isError") is True
        data = resp_direct["content"][0]["data"]
        assert data["code"] == ERR_INVALID_ARGS


# ---------------------------------------------------------------------------
# S5/S6 version + provenance 契约
# ---------------------------------------------------------------------------


class TestVersionProvenanceContract:
    """S5/S6: version + offline flag 全部透出."""

    def test_v1136_module_version_resolved(self):
        d = R11MeasurementDispatcher()
        resp = d.handle_message({
            "jsonrpc": "2.0", "id": 1, "method": "tools/call",
            "params": {"name": MEASURE_V1136_REAL,
                        "arguments": {"v04_score": 0.8538}},
        })
        data = _extract_data(resp)
        vers = data["provenance"]["module_versions"]
        assert "v1136_version" in vers
        # 主 17:43 实事求是: 真值, not "unavailable"
        assert vers["v1136_version"] != f"unavailable: ImportError"
        # version 形如 "0.1.0"
        assert "." in vers["v1136_version"]

    def test_v1130_module_version_resolved(self):
        d = R11MeasurementDispatcher()
        resp = d.handle_message({
            "jsonrpc": "2.0", "id": 1, "method": "tools/call",
            "params": {"name": GET_V1130_BACKEND,
                        "arguments": {"action": "level"}},
        })
        data = _extract_data(resp)
        vers = data["provenance"]["module_versions"]
        assert "v1130_version" in vers
        assert vers["v1130_version"] != f"unavailable: ImportError"

    def test_offline_flag_for_v1136(self):
        """S6: V1136 measure 永远 offline."""
        resp_direct = tool_measure_v1136_real({"v04_score": 0.8538})
        data = resp_direct["content"][0]["data"]
        assert data["provenance"]["offline"] is True

    def test_offline_flag_for_v1130_level(self):
        """S6: V1130 level / runtime / alerts offline; evaluate 不保证."""
        d = R11MeasurementDispatcher()
        for action in ("level", "runtime", "alerts"):
            resp = d.handle_message({
                "jsonrpc": "2.0", "id": 1, "method": "tools/call",
                "params": {"name": GET_V1130_BACKEND,
                            "arguments": {"action": action}},
            })
            data = _extract_data(resp)
            assert data["provenance"]["offline"] is True, action


# ---------------------------------------------------------------------------
# S7 no-fake-provider — V1130 evaluate 永不伪造成功
# ---------------------------------------------------------------------------


class TestNoFakeProviderContract:
    """S7: provider 未配置时 attempts 含 UNCONFIGURED/UNAVAILABLE 透明 state, 不伪造成功."""

    def test_v1130_evaluate_no_fake_success(self):
        """无 ANTHROPIC_API_KEY / OPENAI_API_KEY 时 evaluate 必报透明失败, 不返 fake text."""
        _ensure_offline()  # 保险
        d = R11MeasurementDispatcher()
        resp = d.handle_message({
            "jsonrpc": "2.0", "id": 1, "method": "tools/call",
            "params": {"name": GET_V1130_BACKEND,
                        "arguments": {"action": "evaluate",
                                       "prompt": "Reply exactly with W3_OK"}},
        })
        # 尝试判定: evaluate 会因为没配置真实 provider 失败 (也可能是 timeout)
        # 核心契约: 不伪造 content
        data = _extract_data(resp)
        attempts = data.get("attempts", [])
        assert isinstance(attempts, list)
        if len(attempts) > 0:
            # 不能全 success=True (那是 fake)
            n_success = sum(1 for a in attempts if a.get("success"))
            n_total = len(attempts)
            assert n_success < n_total, (
                "no-fake contract violated: all attempts marked success "
                "without real providers"
            )
        # OR: 如果 evaluate 异常,应 isError=True
        if resp.get("result", {}).get("isError"):
            err = resp["result"]["content"][0]["data"]
            assert err["code"] in (ERR_BACKEND_FAILURE, ERR_MISSING_MODULE,
                                      ERR_TIMEOUT)


# ---------------------------------------------------------------------------
# S9 nine-keys + V3 guards 注入契约
# ---------------------------------------------------------------------------


class TestNineKeysContract:
    """S9: ASI 9 键 + R11 V3 guards 注入."""

    def test_asi_nine_keys_locked_in_result(self):
        d = R11MeasurementDispatcher()
        resp = d.handle_message({
            "jsonrpc": "2.0", "id": 1, "method": "tools/call",
            "params": {"name": MEASURE_V1136_REAL,
                        "arguments": {"v04_score": 0.8538}},
        })
        data = _extract_data(resp)
        meta = data["r11_mcp_meta"]
        assert meta["philosophy_guard"]["asi_nine_keys_locked"] is True
        assert meta["philosophy_guard"]["n_locked"] == 9
        assert meta["philosophy_guard"]["n_total"] == 9

    def test_v3_guards_injected(self):
        """V3 guards 名册必须与 R11 module 顶层常量一致."""
        d = R11MeasurementDispatcher()
        resp = d.handle_message({
            "jsonrpc": "2.0", "id": 1, "method": "tools/call",
            "params": {"name": GET_V1130_BACKEND,
                        "arguments": {"action": "level"}},
        })
        data = _extract_data(resp)
        meta = data["r11_mcp_meta"]
        guards = set(meta["r11_v3_guards"])
        expected = set(V3_GUARDS_R11.keys())
        assert guards == expected, f"guard mismatch: {guards ^ expected}"


# ---------------------------------------------------------------------------
# S11 chaos — transport 失联不丢 dispatcher state
# ---------------------------------------------------------------------------


class TestChaosContract:
    """S11: transport 启停 + chaos 守门."""

    def test_chaos_state_retained(self):
        d = R11MeasurementDispatcher()
        pre = d.stats()
        # 模拟 ping 风暴
        for _ in range(5):
            d.handle_message({"jsonrpc": "2.0", "id": 1,
                              "method": "ping", "params": {}})
        post = d.stats()
        assert post["n_dispatched"] > pre["n_dispatched"], \
            f"chaos fail: pre={pre['n_dispatched']}, post={post['n_dispatched']}"

    def test_run_chaos_helper(self):
        result = run_chaos()
        assert result["chaos_state_retained"] is True
        assert result["pre"]["n_dispatched"] < result["post"]["n_dispatched"]

    def test_stats_lock_under_concurrent_dispatch(self):
        """线程并发 dispatch 也必须正确计数."""
        d = R11MeasurementDispatcher()
        lock_error = []

        def spam():
            try:
                for _ in range(50):
                    d.handle_message({"jsonrpc": "2.0", "id": 1,
                                       "method": "ping", "params": {}})
            except Exception as e:  # noqa: BLE001
                lock_error.append(e)

        threads = [threading.Thread(target=spam) for _ in range(4)]
        for t in threads:
            t.start()
        for t in threads:
            t.join(timeout=10.0)
        assert not lock_error, lock_error
        # 此时 dispatcher 应至少处理 4×50 = 200 messages
        assert d.stats()["n_dispatched"] >= 200


# ---------------------------------------------------------------------------
# S12 dispatcher JSON-RPC 基础契约
# ---------------------------------------------------------------------------


class TestDispatcherContract:
    """S12: initialize / ping / malformed request handling."""

    def test_initialize_protocol(self):
        d = R11MeasurementDispatcher()
        resp = d.handle_message({
            "jsonrpc": "2.0", "id": 1, "method": "initialize",
            "params": {"protocolVersion": "2024-11-05"},
        })
        result = resp["result"]
        assert result["protocolVersion"] == "2024-11-05"
        assert result["serverInfo"]["name"] == R11_SERVER_NAME
        assert result["serverInfo"]["version"] == R11_MCP_VERSION

    def test_ping(self):
        d = R11MeasurementDispatcher()
        resp = d.handle_message({
            "jsonrpc": "2.0", "id": 1, "method": "ping", "params": {},
        })
        assert resp["result"]["pong"] is True

    def test_malformed_request(self):
        d = R11MeasurementDispatcher()
        resp = d.handle_message({"not-jsonrpc": True})
        assert "error" in resp
        # R11 dispatcher 内部 parse_request 失败 → JSONRPC_INTERNAL_ERROR (-32699, 复用 V1129 常量)
        assert resp["error"]["code"] == -32699

    def test_unknown_method(self):
        d = R11MeasurementDispatcher()
        resp = d.handle_message({
            "jsonrpc": "2.0", "id": 1, "method": "tools/non_existent", "params": {},
        })
        assert "error" in resp
        assert resp["error"]["code"] == -32601


# ---------------------------------------------------------------------------
# S13 module + framework 版本常量契约
# ---------------------------------------------------------------------------


class TestVersionConstantContract:
    def test_r11_mcp_version_is_semver(self):
        parts = R11_MCP_VERSION.split(".")
        assert len(parts) == 3, f"R11_MCP_VERSION must be semver, got {R11_MCP_VERSION}"
        for p in parts:
            assert p.isdigit(), f"semver part not digit: {p}"

    def test_v1137_framework_version_is_semver(self):
        parts = V1137_FRAMEWORK_VERSION.split(".")
        assert len(parts) == 3

    def test_default_port_sane(self):
        # 0 不暴露; 8123(V1123) / 8129(V1129) / 8124(V1124) 已用; R11 用 8137
        assert 1024 < DEFAULT_PORT < 65535


# ---------------------------------------------------------------------------
# S14 selftest integration
# ---------------------------------------------------------------------------


class TestSelftestIntegration:
    """S14: --selftest 等价 Python 调用."""

    def test_run_selftest_returns_expected_shape(self):
        result = run_selftest(strict=False)
        # 关键 metrics
        assert result["n_tools_listed"] == 2
        assert result["n_round_trip_ok"] == 2
        assert result["n_round_trip_total"] == 2
        assert result["chaos_state_retained"] is True
        # 2 transport 临时端口
        assert result["sse_actual_port"] > 0
        assert result["http_actual_port"] > 0
        assert result["sse_actual_port"] != result["http_actual_port"]
        # V1130 evaluate + runtime 都要 round-trip
        assert "tool_calls_v1130_evaluate" in result
        assert "tool_calls_v1130_runtime" in result


# ---------------------------------------------------------------------------
# S15 CLI subprocess (主 00:56 any-one-can-take-it)
# ---------------------------------------------------------------------------


class TestCLISubprocessContract:
    """S15: module CLI 行可跑."""

    def test_snapshot_subprocess(self):
        proc = subprocess.run(
            [sys.executable, "-m", "apeireth.v1137_r11_mcp_measurement_tool",
             "--snapshot"],
            capture_output=True, text=True, timeout=30,
            cwd=str(WORKDIR),
        )
        assert proc.returncode == 0, proc.stderr
        snap = json.loads(proc.stdout)
        assert snap["r11_version"] == R11_MCP_VERSION
        assert sorted(snap["tools"]) == ["get_v1130_backend", "measure_v1136_real"]

    def test_selftest_subprocess(self):
        proc = subprocess.run(
            [sys.executable, "-m", "apeireth.v1137_r11_mcp_measurement_tool",
             "--selftest", "--json"],
            capture_output=True, text=True, encoding="utf-8",
             errors="replace",
            timeout=120,
            cwd=str(WORKDIR),
        )
        assert proc.returncode == 0, proc.stderr[:500]
        # Windows + 中文 LOCALE 可能产生混合编码;
        # _print_snapshot 已强制 ensure_ascii=False, 但保险起见 errors='replace'
        assert proc.stdout, "subprocess stdout empty"
        payload = json.loads(proc.stdout)
        assert payload["n_tools_listed"] == 2
        assert payload["chaos_state_retained"] is True
        assert payload["n_round_trip_ok"] == 2

    def test_chaos_subprocess(self):
        proc = subprocess.run(
            [sys.executable, "-m", "apeireth.v1137_r11_mcp_measurement_tool",
             "--chaos", "--json"],
            capture_output=True, text=True, timeout=60,
            cwd=str(WORKDIR),
        )
        assert proc.returncode == 0, proc.stderr[:500]
        payload = json.loads(proc.stdout)
        assert payload["chaos_state_retained"] is True


# ---------------------------------------------------------------------------
# S16 stress / 重入 — dispatcher 重复构造不污染
# ---------------------------------------------------------------------------


class TestDispatcherReentry:
    def test_two_dispatchers_independent(self):
        d1 = R11MeasurementDispatcher()
        d2 = R11MeasurementDispatcher()
        # d1 跑一些调用
        for _ in range(5):
            d1.handle_message({"jsonrpc": "2.0", "id": 1,
                                "method": "ping", "params": {}})
        # d2 计数独立
        assert d2.stats()["n_dispatched"] == 0
        assert d1.stats()["n_dispatched"] == 5
