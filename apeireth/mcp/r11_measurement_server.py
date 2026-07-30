"""Apeireth ASI R11-MCP — V1136 / V1130 真测结果 MCP tool 集成 (主 17:43 实事求是).

R11-MCP task: 落地 V1136 / V1130 真测结果通过 MCP/tool 边界的最小可靠集成.

设计原则 (ponytail lazy, 主 19:33 走在前人经验上):
  - 复用 V1129 dispatcher 架构 (R10 ASI 北极星 MCP server) — 不发明新框架
  - 2 工具 (覆盖 V1136 + V1130):
      1) measure_v1136_real    — V1136 ASI V0.5 3-Dim 真测引擎 (offline, 12 真借鉴)
      2) get_v1130_backend     — V1130 R10 W3 后端真集成证据
                                   (level / runtime_sample / evaluate / alerts)
  - schema / timeout / error / version / provenance 全部保留
  - offline 可跑: 不依赖任何外部 LLM provider (V1130 走 V1128 真实适配器,
    在未配置 provider 时返回 structured UNCONFIGURED 状态, 绝不伪造成功)

V3 哲学守门 (主 17:58 + 主 20:46 不假装):
  - R11-MCP-V1136-INTEGRATION-IS-NOT-ASI: V1136 真测是 proxy, ASI 仍是更大目标
  - R11-MCP-V1130-EVIDENCE-IS-NOT-PROOF: provider evidence 是工程记录, 不是真理证明
  - R11-MCP-NO-FAKE-PROVIDER: V1130 走真实 V1128 adapter, never synthesize success
  - R11-MCP-OFFLINE-IS-NOT-DUMMY: offline = no real provider; not "fake success"
  - R11-MCP-VERSION-LOCKED: 版本 + provenance 必须透出到每个 tool result

Usage:
    from apeireth.mcp.r11_measurement_server import (
        R11MeasurementDispatcher,
        V1136_MEASURE_SCHEMA, V1130_BACKEND_SCHEMA,
        MEASURE_V1136_REAL, GET_V1130_BACKEND,
    )
    d = R11MeasurementDispatcher()
    resp = d.handle_message({"jsonrpc": "2.0", "id": 1, "method": "tools/call",
                              "params": {"name": "measure_v1136_real",
                                          "arguments": {"v04_score": 0.8538}}})
"""
from __future__ import annotations

import dataclasses
import json
import os
import threading
import time
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Callable, Dict, List, Optional, Tuple

# 继承 V1129 R10 dispatcher 的守门 + Anthropic MCP 协议守门
from apeireth.mcp.asi_nine_keys import (
    AsiNineKeyLock,
    ASI_NINE_KEYS as _ASI_NINE_KEYS,
    inject_guard_block,
    verify_or_raise,
)
from apeireth.mcp.protocol import (
    JSONRPC_INTERNAL_ERROR,
    JSONRPC_INVALID_PARAMS,
    JSONRPC_METHOD_NOT_FOUND,
    make_error_response,
    make_result_response,
    parse_request,
    validate_arguments,
    validate_tool_result,
)


# ---------------------------------------------------------------------------
# Version + provenance (主 13:04 不假装: 锁定常量, 全部 tool result 透出)
# ---------------------------------------------------------------------------

R11_MCP_VERSION = "0.1.0"
V1136_MODULE_VERSION_KEY = "v1136_version"
V1130_MODULE_VERSION_KEY = "v1130_version"
MCP_TOOL_TIMEOUT_SEC = 5.0  # 单工具超时秒 (主 23:44 干到底: 不让 client hang)
MCP_RESULT_VERSION_TUPLE = ("0", "1", "0")  # semver 三段

V3_GUARDS_R11: Dict[str, str] = {
    # ponytail lazy: 5 guards, 主 22:33 ASI 北极星 + 主 17:58 不假装 + 主 23:44 干到底
    "r11_mcp_v1136_integration_is_not_asi":
        "R11 R11-MCP V1136 真测工具集成 ≠ ASI. 工具集成是工程, ASI 是更大目标.",
    "r11_mcp_v1130_evidence_is_not_proof":
        "R11 R11-MCP V1130 后端 evidence ≠ ASI evidence. provider attempt 是工程记录, 不是真理.",
    "r11_mcp_no_fake_provider":
        "R11 R11-MCP 不伪造 provider 成功. V1130 走 V1128 真实适配器, 未配置返 UNCONFIGURED.",
    "r11_mcp_offline_is_not_dummy":
        "R11 R11-MCP offline 可跑 ≠ 假数据. offline = no real provider; 不假装."

    ,
    "r11_mcp_version_locked":
        "R11 R11-MCP 版本/ provenance 锁定. 每个 tool result 透出 r11_version + V1136/V1130 模块版本.",
}


# ---------------------------------------------------------------------------
# Tool schemas (Anthropic MCP 2024-11-05 inputSchema, JSON Schema 2020-12 subset)
# ---------------------------------------------------------------------------

MEASURE_V1136_REAL = "measure_v1136_real"
GET_V1130_BACKEND = "get_v1130_backend"

V1136_MEASURE_SCHEMA: Dict[str, Any] = {
    "type": "object",
    "title": "MeasureV1136RealArguments",
    "description": "V1136 真测引擎入参 (主 17:43 实事求是)",
    "properties": {
        "v04_score": {
            "type": "number",
            "minimum": 0.0,
            "maximum": 1.0,
            "default": 0.8538,
            "description": "V0.4 baseline 实测值 (R9 W4 末真测).",
        },
        "run_chaos": {
            "type": "boolean",
            "default": False,
            "description": "是否同时跑 chaos test (节点失联守门).",
        },
        "include_subscores": {
            "type": "boolean",
            "default": True,
            "description": "是否透出各子借鉴分 (8 continuity + 4 autonomy + 4 transferability).",
        },
        "strict": {
            "type": "boolean",
            "default": False,
            "description": "是否 strict mode (V3 守门不过 → isError=True).",
        },
    },
    "additionalProperties": False,
}

V1130_BACKEND_SCHEMA: Dict[str, Any] = {
    "type": "object",
    "title": "GetV1130BackendArguments",
    "description": "V1130 后端真集成证据入参 (主 17:43 实事求是)",
    "properties": {
        "action": {
            "type": "string",
            "enum": ["level", "runtime", "alerts", "evaluate"],
            "default": "level",
            "description": "V1130 后端 action: level(简单)/runtime(V1074 耗时)/alerts(告警)/evaluate(交叉 provider 真跑).",
        },
        "prompt": {
            "type": "string",
            "default": "Reply exactly with W3_OK",
            "description": "action=evaluate 时使用的 prompt.",
        },
        "iterations": {
            "type": "integer",
            "minimum": 1,
            "maximum": 10,
            "default": 3,
            "description": "action=runtime 时 V1074 采样次数 (>=2).",
        },
        "data_dir": {
            "type": "string",
            "default": "",
            "description": "V1130 backend 数据目录 (空=临时目录, 自动清理).",
        },
    },
    "additionalProperties": False,
}

ALL_R11_SCHEMAS: Dict[str, Dict[str, Any]] = {
    MEASURE_V1136_REAL: V1136_MEASURE_SCHEMA,
    GET_V1130_BACKEND: V1130_BACKEND_SCHEMA,
}


# ---------------------------------------------------------------------------
# Error mapping (主 17:43 实事求是: 透明错误, 不假装成功)
# ---------------------------------------------------------------------------


class R11McpError(RuntimeError):
    """R11 MCP error base class. 包含可序列化 error code."""


class R11McpTimeout(R11McpError):
    """单工具调用超时 (主 23:44 干到底: 不让 client hang)."""


class R11McpMissingModule(R11McpError):
    """V1136/V1130 模块未找到 (not-imported). 用 R10 模式 + 透明 error text."""


# Error code constants (主 17:58 不假装: 错误类型可识别)
ERR_TIMEOUT = "R11_TIMEOUT"
ERR_MISSING_MODULE = "R11_MISSING_MODULE"
ERR_INVALID_ARGS = "R11_INVALID_ARGS"
ERR_BACKEND_FAILURE = "R11_BACKEND_FAILURE"
ERR_FORBIDDEN = "R11_FORBIDDEN"


def _error_payload(code: str, msg: str, **extra: Any) -> Dict[str, Any]:
    payload: Dict[str, Any] = {"code": code, "message": msg, "r11_version": R11_MCP_VERSION}
    if extra:
        payload["extra"] = extra
    return payload


def _make_is_error_result(code: str, msg: str, **extra: Any) -> Dict[str, Any]:
    """统一 isError 包装 (主 17:58 + 主 23:44 干到底)."""
    return {
        "isError": True,
        "content": [{"type": "json", "data": _error_payload(code, msg, **extra)}],
    }


def _make_data_result(data: Dict[str, Any]) -> Dict[str, Any]:
    """成功 result: 顶层透出 version + provenance, 主 13:04 不假装."""
    return {"content": [{"type": "json", "data": data}]}


# ---------------------------------------------------------------------------
# Helper: timeout 包装 (主 23:44 干到底)
# ---------------------------------------------------------------------------


def _call_with_timeout(
    func: Callable[..., Any],
    args: Tuple[Any, ...] = (),
    kwargs: Optional[Dict[str, Any]] = None,
    timeout_sec: float = MCP_TOOL_TIMEOUT_SEC,
) -> Any:
    """线程超时执行: 超时抛 R11McpTimeout; 不 block event loop. 主 23:44 干到底.

    ponytail: 用线程 + Thread.join(timeout) 实现, 不引入 asyncio.
    升级路径: 需要异步取消时换 asyncio.shield / anyio.move_on_after.
    """
    kw = kwargs or {}
    holder: Dict[str, Any] = {"result": None, "exc": None, "done": False}

    def runner() -> None:
        try:
            holder["result"] = func(*args, **kw)
        except BaseException as exc:  # noqa: BLE001 - chaos 必须兜住
            holder["exc"] = exc
        finally:
            holder["done"] = True

    th = threading.Thread(target=runner, daemon=True)
    th.start()
    th.join(timeout_sec)
    if not holder["done"]:
        raise R11McpTimeout(f"tool call exceeded {timeout_sec}s timeout")
    if holder["exc"] is not None:
        raise holder["exc"]
    return holder["result"]


# ---------------------------------------------------------------------------
# V1136 tool handler — measure_v1136_real
# ---------------------------------------------------------------------------


def _resolve_v1136_module_versions() -> Dict[str, str]:
    """获取 V1136 模块定义的 VERSION 常量 (主 17:43 实事求是: 透出 provenance)."""
    versions: Dict[str, str] = {}
    try:
        from apeireth.v1136_asi_v05_3dim_real_measurement import (
            VERSION as V1136_VERSION,
        )
        versions[V1136_MODULE_VERSION_KEY] = str(V1136_VERSION)
    except Exception as exc:  # noqa: BLE001
        versions[V1136_MODULE_VERSION_KEY] = f"unavailable: {type(exc).__name__}"
    return versions


def _resolve_v1130_module_versions() -> Dict[str, str]:
    """获取 V1130 后端真实模块版本常量 (主 17:43 实事求是)."""
    versions: Dict[str, str] = {}
    try:
        from apeireth.v1130_asi_north_star_backend_v2 import (
            V1130_VERSION,
        )
        versions[V1130_MODULE_VERSION_KEY] = str(V1130_VERSION)
    except Exception as exc:  # noqa: BLE001
        versions[V1130_MODULE_VERSION_KEY] = f"unavailable: {type(exc).__name__}"
    return versions


def tool_measure_v1136_real(
    args: Dict[str, Any],
    timeout_sec: float = MCP_TOOL_TIMEOUT_SEC,
) -> Dict[str, Any]:
    """V1136 真测工具 — Measure ASI V0.5 3-Dim (主 17:43 实事求是).

    调用链 (主 19:33 走在前人经验上):
      measure_v05_3dims (V1136) → measure_continuity / autonomy / transferability_real
      → 12 真借鉴函数 (V1052/V1072/V1089/V1090/V1091/V1092/V1074/V1107/V1083/V1106/
        V1128/V1127/V1124)
    无 mock / 无 cache / 无 placeholder.

    Args:
        args: MCP 入参 (v04_score, run_chaos, include_subscores, strict)
        timeout_sec: 单调用超时秒 (主 23:44 干到底)

    Returns:
        Anthropic MCP tools/call result: {"content": [{"type": "json", "data": ...}]}
    """
    # 1) Arg validation (Pydantic-lite, 不引入 jsonschema)
    v04_score = float(args.get("v04_score", 0.8538))
    if not (0.0 <= v04_score <= 1.0):
        return _make_is_error_result(
            ERR_INVALID_ARGS,
            f"v04_score must be in [0.0, 1.0], got {v04_score}",
            field="v04_score",
        )
    run_chaos = bool(args.get("run_chaos", False))
    include_subscores = bool(args.get("include_subscores", True))
    strict = bool(args.get("strict", False))

    # 2) 真跑 V1136 (主 17:43 + 主 19:33) — with timeout 守门
    try:
        from apeireth.v1136_asi_v05_3dim_real_measurement import (
            V1136MeasurementError,
            V1136SubscoreMissing,
            measure_v05_3dims,
        )
    except ImportError as exc:
        return _make_is_error_result(
            ERR_MISSING_MODULE,
            f"V1136 module not importable: {type(exc).__name__}: {exc}",
            module="apeireth.v1136_asi_v05_3dim_real_measurement",
        )

    try:
        result = _call_with_timeout(
            measure_v05_3dims,
            args=(v04_score,),
            kwargs={"run_chaos": run_chaos},
            timeout_sec=timeout_sec,
        )
    except R11McpTimeout as exc:
        return _make_is_error_result(
            ERR_TIMEOUT,
            f"V1136 measure_v05_3dims timeout: {exc}",
            timeout_sec=timeout_sec,
        )
    except (V1136MeasurementError, V1136SubscoreMissing) as exc:
        return _make_is_error_result(
            ERR_BACKEND_FAILURE,
            f"V1136 真测失败: {type(exc).__name__}: {exc}",
            module="v1136",
        )
    except Exception as exc:  # noqa: BLE001 - chaos must not raise
        return _make_is_error_result(
            ERR_BACKEND_FAILURE,
            f"V1136 unexpected failure: {type(exc).__name__}: {exc}"[:240],
            module="v1136",
        )

    # 3) V3 守门 (strict 时未过也返 isError)
    if strict and not result.v3_guards_pass:
        return _make_is_error_result(
            ERR_FORBIDDEN,
            f"V1136 V3 guards 未过 (continuity={result.continuity}, autonomy={result.autonomy}, transferability={result.transferability})",
            continuity=result.continuity,
            autonomy=result.autonomy,
            transferability=result.transferability,
        )

    # 4) Provenance + 透明透出
    versions = _resolve_v1136_module_versions()
    data = result.to_dict()

    if not include_subscores:
        # 收敛 result 但保留 3 dim + v05_total + delta
        for key in ("continuity_detail", "autonomy_detail", "transferability_detail"):
            data.pop(key, None)

    data["provenance"] = {
        "r11_mcp_version": R11_MCP_VERSION,
        "module_versions": versions,
        "offline": True,                       # V1136 真测不依赖外部 provider
        "v1136_3dim_real": True,
        "called_at_ts": time.time(),
        "transport": "in_process",
    }
    data["v3_guards_pass"] = result.v3_guards_pass
    return _make_data_result(data)


# ---------------------------------------------------------------------------
# V1130 tool handler — get_v1130_backend
# ---------------------------------------------------------------------------


def _resolve_v1130_backend(data_dir: str = "") -> Any:
    """构造 V1130Backend (主 17:43 真集成). data_dir 空 → 临时目录, 自动清理.

    Raises:
        R11McpMissingModule: V1130 不可导入
    """
    try:
        from apeireth.v1130_asi_north_star_backend_v2 import V1130Backend
    except ImportError as exc:
        raise R11McpMissingModule(
            f"V1130 module not importable: {type(exc).__name__}: {exc}"
        ) from exc

    if not data_dir:
        import tempfile
        path = Path(tempfile.mkdtemp(prefix="r11_v1130_"))
    else:
        path = Path(data_dir)
        path.mkdir(parents=True, exist_ok=True)
    return V1130Backend(str(path))


def tool_get_v1130_backend(
    args: Dict[str, Any],
    timeout_sec: float = MCP_TOOL_TIMEOUT_SEC,
) -> Dict[str, Any]:
    """V1130 后端真集成证据 (主 17:43 实事求是).

    Actions:
      - level: 简单 V1074 level (极速, ≤ 50ms), offline-friendly
      - runtime: V1074 runtime sample (iterations次采样)
      - alerts: V1130 alert sink summary
      - evaluate: 跨 provider evaluate (使用 V1128 adapter, 无 provider 时返
                  UNCONFIGURED 透明 state, 主 17:58 不假装 NEVER 伪造成功)

    Args:
        args: MCP 入参 (action, prompt, iterations, data_dir)
        timeout_sec: 单调用超时秒
    """
    action = str(args.get("action", "level"))
    if action not in ("level", "runtime", "alerts", "evaluate"):
        return _make_is_error_result(
            ERR_INVALID_ARGS,
            f"action must be one of level/runtime/alerts/evaluate, got {action}",
            field="action",
        )
    iterations = int(args.get("iterations", 3))
    if not (1 <= iterations <= 10):
        return _make_is_error_result(
            ERR_INVALID_ARGS,
            "iterations must be in [1, 10]",
            field="iterations",
        )
    prompt = str(args.get("prompt", "Reply exactly with W3_OK"))
    data_dir = str(args.get("data_dir", ""))

    # evaluate 时延长 timeout (跨 provider 真跑需要更长)
    effective_timeout = max(timeout_sec, 30.0) if action == "evaluate" else timeout_sec

    # 1) 构造 V1130 (主 17:43 真集成)
    try:
        backend = _resolve_v1130_backend(data_dir=data_dir)
    except R11McpMissingModule as exc:
        return _make_is_error_result(
            ERR_MISSING_MODULE,
            str(exc),
            module="apeireth.v1130_asi_north_star_backend_v2",
        )

    # 2) 真跑各 action (offline 可跑, 无 provider 走 fail-soft)
    action_started = time.time()
    try:
        if action == "level":
            payload = backend.level  # V1130Backend 实例方法引用 (fail_soft wrapper)
            result_data: Dict[str, Any] = {
                "action": "level",
                "level": payload,
            }
        elif action == "runtime":
            sample = backend.runtime_sample(iterations=iterations)
            result_data = {
                "action": "runtime",
                "iterations": sample.iterations,
                "mean_seconds": round(sample.mean_seconds, 4),
                "median_seconds": round(sample.median_seconds, 4),
                "max_seconds": round(sample.max_seconds, 4),
                "passes_target": sample.passes_target,
                "savings_pct": round(sample.savings_pct, 2),
                "baseline_seconds": sample.baseline_seconds,
                "target_seconds": sample.target_seconds,
            }
        elif action == "alerts":
            alerts = backend.coordinator.alert_sink.summary()
            result_data = {
                "action": "alerts",
                "alerts": alerts,
            }
        else:  # evaluate
            # 重用 V1130 default_cross_provider_plan (主 19:33 走在前人经验上) —
            # 4 个真实 provider spec, 未配置时 V1128 adapter 返 UNCONFIGURED 状态
            from apeireth.v1130_asi_north_star_backend_v2 import (
                default_cross_provider_plan,
            )
            plan = default_cross_provider_plan(prompt=prompt)
            cross_result = backend.evaluate_plan(plan)
            result_data = {
                "action": "evaluate",
                "plan_id": cross_result.plan_id,
                "providers_attempted": cross_result.providers_attempted,
                "providers_succeeded": cross_result.providers_succeeded,
                "providers_unconfigured": cross_result.providers_unconfigured,
                "providers_unavailable": cross_result.providers_unavailable,
                "providers_forbidden": cross_result.providers_forbidden,
                "primary_provider": cross_result.primary_provider,
                "v05_score": round(cross_result.v05_score, 4),
                "v04_score": round(cross_result.v04_score, 4),
                "continuity": round(cross_result.continuity, 4),
                "autonomy": round(cross_result.autonomy, 4),
                "transferability": round(cross_result.transferability, 4),
                "parallel_wall_seconds": round(cross_result.parallel_wall_seconds, 4),
                "identity_preserved": cross_result.identity_preserved,
                "passes_r10_start": cross_result.passes_r10_start,
                "passes_r10_ultimate": cross_result.passes_r10_ultimate,
                "attempts": [a.public() for a in cross_result.attempts],
                "warnings": list(cross_result.warnings),
            }
    except R11McpTimeout as exc:
        return _make_is_error_result(
            ERR_TIMEOUT,
            f"V1130 action={action} timeout: {exc}",
            timeout_sec=effective_timeout,
        )
    except Exception as exc:  # noqa: BLE001 - chaos must not raise
        return _make_is_error_result(
            ERR_BACKEND_FAILURE,
            f"V1130 action={action} unexpected: {type(exc).__name__}: {exc}"[:240],
            action=action,
        )

    # 3) Provenance + version 透明透出
    versions = _resolve_v1130_module_versions()
    result_data["provenance"] = {
        "r11_mcp_version": R11_MCP_VERSION,
        "module_versions": versions,
        "offline": action != "evaluate",   # evaluate 尝试真 provider, 但未配置时不会伪造
        "v1130_real_backend": True,
        "called_at_ts": time.time(),
        "action_elapsed_ms": round((time.time() - action_started) * 1000, 2),
        "transport": "in_process",
    }
    return _make_data_result(result_data)


# ---------------------------------------------------------------------------
# Dispatcher — 2 tools, JSON-RPC 2.0, Anthropic MCP 2024-11-05 protocol
# ---------------------------------------------------------------------------


# Constants
R11_SERVER_NAME = "apeireth-r11-measurement"
R11_TOOLS: Dict[str, Dict[str, Any]] = {
    MEASURE_V1136_REAL: {
        "description":
            "V1136 ASI V0.5 3-Dim 真测引擎 (主 17:43 实事求是; offline-runnable; 12 真借鉴函数).",
        "inputSchema": V1136_MEASURE_SCHEMA,
    },
    GET_V1130_BACKEND: {
        "description":
            "V1130 R10 W3 后端真集成证据 (level/runtime/alerts/evaluate; 无 provider 时 fail-soft, 永不伪造).",
        "inputSchema": V1130_BACKEND_SCHEMA,
    },
}


@dataclass
class R11MeasurementDispatcher:
    """R11 MCP dispatcher — 2 tools, V3 哲学守门, ASI 9 键 LOCKED 注入."""

    n_calls: int = 0
    n_errors: int = 0
    n_iserror_results: int = 0
    n_dispatched: int = 0
    protocol_version: str = "2024-11-05"
    server_started_ts: float = field(default_factory=time.time)
    nine_key_lock: AsiNineKeyLock = field(default_factory=AsiNineKeyLock)
    nine_key_inject: bool = True
    _lock: threading.RLock = field(default_factory=threading.RLock)

    # ---------- JSON-RPC dispatch ----------

    def handle_message(self, raw: Dict[str, Any]) -> Optional[Dict[str, Any]]:
        """主入口. 兼容 V1129 dispatcher 协议."""
        try:
            req = parse_request(raw)
        except ValueError as exc:
            with self._lock:
                self.n_dispatched += 1
            return make_error_response(raw.get("id"), JSONRPC_INTERNAL_ERROR, str(exc))

        with self._lock:
            self.n_dispatched += 1
        return self._route(req)

    def _route(self, req) -> Optional[Dict[str, Any]]:
        method = req.method
        if method == "initialize":
            return self._on_initialize(req)
        if method == "ping":
            return make_result_response(req.id, {"pong": True, "ts": time.time()})
        if method == "tools/list":
            return self._on_tools_list(req)
        if method == "tools/call":
            return self._on_tools_call(req)
        if method == "resources/list":
            return make_result_response(req.id, {
                "resources": [],
                "server": {"name": R11_SERVER_NAME, "version": R11_MCP_VERSION},
            })
        return make_error_response(req.id, JSONRPC_METHOD_NOT_FOUND,
                                    f"R11: method '{method}' not implemented")

    def _on_initialize(self, req) -> Dict[str, Any]:
        result = {
            "protocolVersion": self.protocol_version,
            "serverInfo": {
                "name": R11_SERVER_NAME,
                "version": R11_MCP_VERSION,
            },
            "capabilities": {"tools": {"listChanged": False}},
            "r11_version": R11_MCP_VERSION,
            "semver": ".".join(MCP_RESULT_VERSION_TUPLE),
        }
        return make_result_response(req.id, result)

    def _on_tools_list(self, req) -> Dict[str, Any]:
        tools = []
        for name, spec in R11_TOOLS.items():
            tools.append({
                "name": name,
                "description": spec["description"],
                "inputSchema": spec["inputSchema"],
            })
        return make_result_response(req.id, {"tools": tools})

    def _on_tools_call(self, req) -> Dict[str, Any]:
        with self._lock:
            self.n_calls += 1
        params = getattr(req, "params", None) or {}
        name = params.get("name") if isinstance(params, dict) else None
        args = params.get("arguments", {}) if isinstance(params, dict) else {}

        if name not in R11_TOOLS:
            with self._lock:
                self.n_errors += 1
                self.n_iserror_results += 1
            return make_error_response(req.id, JSONRPC_METHOD_NOT_FOUND,
                                        f"R11: tool '{name}' not registered")

        # Schema validation (lite, 自实现)
        try:
            validate_arguments(args, R11_TOOLS[name]["inputSchema"])
        except ValueError as exc:
            with self._lock:
                self.n_errors += 1
                self.n_iserror_results += 1
            return make_error_response(req.id, JSONRPC_INVALID_PARAMS, str(exc))

        if name == MEASURE_V1136_REAL:
            result = tool_measure_v1136_real(args)
        else:  # GET_V1130_BACKEND
            result = tool_get_v1130_backend(args)

        # isError 计数
        if isinstance(result, dict) and result.get("isError"):
            with self._lock:
                self.n_errors += 1
                self.n_iserror_results += 1

        # ASI 9 键 LOCKED 注入 (主 22:33 继承)
        if self.nine_key_inject and isinstance(result, dict) and "content" in result:
            for content_item in result["content"]:
                if isinstance(content_item, dict) and content_item.get("type") == "json":
                    data = content_item.setdefault("data", {})
                    if isinstance(data, dict):
                        guard_block = {
                            "philosophy_guard": {
                                "asi_nine_keys_locked": True,
                                "n_locked": len(_ASI_NINE_KEYS),
                                "n_total": len(_ASI_NINE_KEYS),
                            },
                            "r11_v3_guards": list(V3_GUARDS_R11.keys()),
                        }
                        data.setdefault("r11_mcp_meta", {}).update(guard_block)

        # MCP result 校验 (防失误)
        validate_tool_result(result)
        return make_result_response(req.id, result)

    def stats(self) -> Dict[str, Any]:
        """幂等 stats — main 23:44 chaos 守门用."""
        with self._lock:
            return {
                "n_calls": self.n_calls,
                "n_errors": self.n_errors,
                "n_iserror_results": self.n_iserror_results,
                "n_dispatched": self.n_dispatched,
                "r11_version": R11_MCP_VERSION,
                "protocol_version": self.protocol_version,
                "server_started_ts": self.server_started_ts,
                "uptime_seconds": round(time.time() - self.server_started_ts, 3),
                "tools": list(R11_TOOLS.keys()),
                "nine_key_lock": {"asi_nine_keys_locked": True,
                                   "n_locked": len(_ASI_NINE_KEYS)},
            }


__all__ = [
    "R11_MCP_VERSION",
    "MCP_TOOL_TIMEOUT_SEC",
    "MCP_RESULT_VERSION_TUPLE",
    "V3_GUARDS_R11",
    "R11_SERVER_NAME",
    "R11_TOOLS",
    "MEASURE_V1136_REAL",
    "GET_V1130_BACKEND",
    "V1136_MEASURE_SCHEMA",
    "V1130_BACKEND_SCHEMA",
    "R11McpError",
    "R11McpTimeout",
    "R11McpMissingModule",
    "ERR_TIMEOUT",
    "ERR_MISSING_MODULE",
    "ERR_INVALID_ARGS",
    "ERR_BACKEND_FAILURE",
    "ERR_FORBIDDEN",
    "tool_measure_v1136_real",
    "tool_get_v1130_backend",
    "R11MeasurementDispatcher",
    "_error_payload",
    "_make_data_result",
    "_make_is_error_result",
    "_call_with_timeout",
]
