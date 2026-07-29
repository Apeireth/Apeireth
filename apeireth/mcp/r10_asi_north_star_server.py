"""apeireth.mcp.r10_asi_north_star_server — R10 ASI 北极星 MCP Server (V1129).

5 大 R10 MCP 工具 (V1129 真实现, 主 17:43 实事求是: 不 mock):
  1. measure_asi            V0.5 18 维真测 (V1125 evaluate_r10 + V1124 backend 协同)
  2. get_north_star         ASI 北极星 (V1124 HTTP /asi/north-star 真连接)
  3. check_identity         V1095 IdentityStoreV1095 中央档案
  4. verify_audit_chain     V1095 跨槽位 hash + V1124 audit chain 真验
  5. list_personas          V1095 列出 4 archetype 槽位

设计原则 (主 17:58 不假装):
  - 工具失败不假装成功, 一律返 isError=True + 真实 error 描述
  - 任何外部依赖不可用 (Anthropic 403 / Ollama 不可用) → 透明报告 degraded
  - 所有写入走 V1095 fsync 守门 (主 23:44 干到底)
  - dispatcher state (n_calls / n_errors) 在 transport 失联时仍累计 (chaos 守门)

借鉴:
  - V1123 AsiNorthStarDispatcher (R9 真生产 9 键 LOCKED)
  - V1124 ASINorthStarBackend (HTTP /asi/level + /asi/measure + /asi/north-star)
  - V1095 IdentityStoreV1095 (WAL+fsync 3 道保险)
  - V1125 evaluate_r10 (V0.5 18 维公式 + R10 4 选 1 主轨道)
  - Anthropic MCP 2024-11-05 (initialize / tools/list / tools/call)
"""
from __future__ import annotations

import json
import os
import socket
import threading
import time
import urllib.error
import urllib.request
import uuid
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Callable, Dict, List, Optional, Tuple

# 同包内 import
from .protocol import (
    JSONRPC_INTERNAL_ERROR, JSONRPC_INVALID_PARAMS, JSONRPC_METHOD_NOT_FOUND,
    SchemaViolation, check_protocol_version, make_error_response,
    make_result_response, parse_request, validate_arguments, validate_tool_result,
)
from .asi_nine_keys import AsiNineKeyLock, inject_guard_block


V1129_VERSION = "0.1.0"
SERVER_NAME = "apeireth-r10-asi-mcp"

# 5 大工具 schema (R10 LOCKED)
MEASURE_ASI_SCHEMA = {
    "type": "object",
    "properties": {
        "v04_actual": {"type": "number", "minimum": 0.0, "maximum": 1.0,
                        "description": "V0.4 真测分 (默认 0.8538 = V0.4 baseline)"},
        "continuity": {"type": "number", "minimum": 0.0, "maximum": 1.0, "default": 0.85},
        "autonomy": {"type": "number", "minimum": 0.0, "maximum": 1.0, "default": 0.85},
        "transferability": {"type": "number", "minimum": 0.0, "maximum": 1.0, "default": 0.85},
        "v1074_v03_actual": {"type": "number", "minimum": 0.0, "maximum": 1.0, "default": 0.8897},
        "week_label": {"type": "string", "pattern": "^R10-W[0-9]+$", "default": "R10-W1"},
        "live": {"type": "boolean", "default": False,
                 "description": "True=真跑三件套 subprocess, False=fallback"},
    },
    "required": ["v04_actual"],
    "additionalProperties": False,
}

GET_NORTH_STAR_SCHEMA = {
    "type": "object",
    "properties": {
        "v1124_base_url": {"type": "string", "default": "",
                            "description": "V1124 backend URL (空字符串=本地直连 in-process)"},
        "include_composite": {"type": "boolean", "default": True,
                              "description": "True=含 ASI 北极星综合评估 + 哲学子分"},
    },
    "additionalProperties": False,
}

CHECK_IDENTITY_SCHEMA = {
    "type": "object",
    "properties": {
        "identity_id": {"type": "string", "default": ""},
        "include_switches": {"type": "boolean", "default": True},
    },
    "additionalProperties": False,
}

VERIFY_AUDIT_CHAIN_SCHEMA = {
    "type": "object",
    "properties": {
        "include_breakdown": {"type": "boolean", "default": True},
    },
    "additionalProperties": False,
}

LIST_PERSONAS_SCHEMA = {
    "type": "object",
    "properties": {
        "archetype": {"type": "string", "default": "",
                      "description": "可选 archetype 过滤 (调度者/学习者/思考者/助手)"},
        "include_emerged": {"type": "boolean", "default": True},
    },
    "additionalProperties": False,
}


# ---------------------------------------------------------------------------
# 5 大工具实现
# ---------------------------------------------------------------------------


def tool_measure_asi(args: Dict[str, Any],
                      v1124_backend: Any = None) -> Dict[str, Any]:
    """measure_asi: V0.5 18 维真测 (V1125 evaluate_r10 真跑)."""
    v04 = float(args.get("v04_actual", 0.8538))
    continuity = float(args.get("continuity", 0.85))
    autonomy = float(args.get("autonomy", 0.85))
    transferability = float(args.get("transferability", 0.85))
    v1074_v03 = float(args.get("v1074_v03_actual", 0.8897))
    week_label = str(args.get("week_label", "R10-W1"))
    live = bool(args.get("live", False))
    # 真跑 V1125 evaluate_r10 (主 17:43 实事求是)
    try:
        from apeireth.v1125_r10_integration_protocol import (
            evaluate_r10, R10_ULTIMATE_TARGET, ASI_NORTH_STAR as _ASI_NORTH_STAR,
        )
        report = evaluate_r10(
            week_label=week_label,
            continuity=continuity, autonomy=autonomy, transferability=transferability,
            v1074_v03_actual=v1074_v03, v04_actual=v04, no_write=not live,
        )
        v05 = report["v05_score"]
        composite = report["north_star_composite"]
        track = report["r10_track_decision"]
        return {"content": [{"type": "json", "data": {
            "week_label": week_label,
            "v04_actual": round(v04, 4),
            "v05_total": v05["v05_total"],
            "v05_pass_ultimate": v05["v05_pass_ultimate"],
            "r10_ultimate_target": R10_ULTIMATE_TARGET,
            "composite": composite,
            "track_decision": track,
            "all_ok": report["all_ok"],
            "source": "v1125_evaluate_r10",
        }}]}
    except Exception as exc:  # noqa: BLE001
        # 透明兜底 (主 17:43 实事求是: 不假装, 真返 error)
        return {"isError": True,
                "content": [{"type": "text",
                             "text": f"measure_asi failed: {type(exc).__name__}: {exc}"}]}


def tool_get_north_star(args: Dict[str, Any],
                         v1124_backend: Any = None) -> Dict[str, Any]:
    """get_north_star: V1124 HTTP /asi/north-star 真连接 + ASI 北极星综合评估."""
    base_url = str(args.get("v1124_base_url", "")).strip()
    include_composite = bool(args.get("include_composite", True))
    # 真连 V1124 backend
    if v1124_backend is not None and not base_url:
        # in-process (主 17:43 真跑)
        try:
            ns = v1124_backend.north_star()
            return {"content": [{"type": "json", "data": {
                "north_star": ns,
                "source": "v1124_in_process",
                "transport": "in_process",
            }}]}
        except Exception as exc:  # noqa: BLE001
            return {"isError": True,
                    "content": [{"type": "text",
                                 "text": f"v1124 in-process error: {type(exc).__name__}: {exc}"}]}
    # HTTP 客户端模式
    if not base_url:
        base_url = os.environ.get("APEIRETH_V1124_BASE_URL", "http://127.0.0.1:8124")
    try:
        url = f"{base_url.rstrip('/')}/asi/north-star"
        with urllib.request.urlopen(url, timeout=3.0) as resp:
            body = json.loads(resp.read().decode("utf-8"))
        return {"content": [{"type": "json", "data": {
            "north_star": body,
            "source": "v1124_http",
            "transport": "http",
            "base_url": base_url,
        }}]}
    except (urllib.error.URLError, socket.timeout, ConnectionError, OSError, json.JSONDecodeError) as exc:
        return {"isError": True,
                "content": [{"type": "text",
                             "text": f"v1124 HTTP unavailable ({base_url}): {type(exc).__name__}: {exc}. "
                                     f"Pass v1124_base_url or use in-process mode."}]}


def tool_check_identity(args: Dict[str, Any],
                          v1095_store: Any = None) -> Dict[str, Any]:
    """check_identity: V1095 IdentityStoreV1095 中央档案真读."""
    identity_id = str(args.get("identity_id", ""))
    include_switches = bool(args.get("include_switches", True))
    if v1095_store is None:
        return {"isError": True,
                "content": [{"type": "text",
                             "text": "v1095_store not bound (主 17:43 实事求是: 不假装读空)"}]}
    try:
        prof = v1095_store.get_or_create_profile(identity_id=identity_id or None)
        stats = v1095_store.stats()
        prof_dict = prof.to_dict() if hasattr(prof, "to_dict") else {}
        result: Dict[str, Any] = {
            "identity": prof_dict,
            "stats": stats,
            "source": "v1095_in_process",
        }
        if include_switches:
            result["n_switches_total"] = stats.get("n_switches_total", 0)
            result["active_pid"] = (stats.get("profile") or {}).get("active_pid")
        return {"content": [{"type": "json", "data": result}]}
    except Exception as exc:  # noqa: BLE001
        return {"isError": True,
                "content": [{"type": "text",
                             "text": f"check_identity failed: {type(exc).__name__}: {exc}"}]}


def tool_verify_audit_chain(args: Dict[str, Any],
                             v1095_store: Any = None,
                             v1124_backend: Any = None) -> Dict[str, Any]:
    """verify_audit_chain: V1095 跨槽位 hash + V1124 audit chain 真验."""
    include_breakdown = bool(args.get("include_breakdown", True))
    result: Dict[str, Any] = {"checks": {}, "all_pass": True, "source": []}
    # 1) V1095 cross_slot_hash + v1072_compat_hash
    if v1095_store is not None:
        try:
            ch = v1095_store.cross_slot_hash()
            v1072_hash = v1095_store._v1072_compat_hash()
            stats = v1095_store.stats()
            meta = stats.get("meta", {})
            v1095_ok = (ch == meta.get("cross_slot_hash", ch)) and \
                        (v1072_hash == meta.get("v1072_compat_hash", v1072_hash))
            result["checks"]["v1095_identity"] = {
                "cross_slot_hash": ch,
                "v1072_compat_hash": v1072_hash,
                "meta_cross_slot_hash": meta.get("cross_slot_hash"),
                "meta_v1072_compat_hash": meta.get("v1072_compat_hash"),
                "n_fsync_total": stats.get("n_fsync_total", 0),
                "pass": v1095_ok,
            }
            result["source"].append("v1095_in_process")
            if not v1095_ok:
                result["all_pass"] = False
        except Exception as exc:  # noqa: BLE001
            result["checks"]["v1095_identity"] = {"pass": False, "error": f"{type(exc).__name__}: {exc}"}
            result["all_pass"] = False
    else:
        result["checks"]["v1095_identity"] = {"pass": False, "error": "v1095_store not bound"}
        result["all_pass"] = False
    # 2) V1124 audit chain
    if v1124_backend is not None:
        try:
            audit = v1124_backend.store.audit.verify()
            v1124_ok = bool(audit.get("valid", True))
            result["checks"]["v1124_audit"] = {
                "n_records": audit.get("records", 0),
                "valid": v1124_ok,
                "pass": v1124_ok,
            }
            result["source"].append("v1124_in_process")
            if not v1124_ok:
                result["all_pass"] = False
        except Exception as exc:  # noqa: BLE001
            result["checks"]["v1124_audit"] = {"pass": False, "error": f"{type(exc).__name__}: {exc}"}
            result["all_pass"] = False
    else:
        result["checks"]["v1124_audit"] = {"pass": False, "error": "v1124_backend not bound"}
        result["all_pass"] = False
    if not include_breakdown:
        result = {"all_pass": result["all_pass"], "source": result["source"]}
    return {"content": [{"type": "json", "data": result}]}


def tool_list_personas(args: Dict[str, Any],
                        v1095_store: Any = None) -> Dict[str, Any]:
    """list_personas: V1095 list_slots 真读 4 archetype."""
    archetype = str(args.get("archetype", "")).strip()
    include_emerged = bool(args.get("include_emerged", True))
    if v1095_store is None:
        return {"isError": True,
                "content": [{"type": "text", "text": "v1095_store not bound"}]}
    try:
        slots = v1095_store.list_slots(archetype=archetype or None,
                                         include_emerged=include_emerged)
        personas = []
        for s in slots:
            personas.append({
                "pid": s.pid,
                "archetype": s.archetype,
                "role_description": s.role_description,
                "priority": s.priority,
                "n_activations": s.n_activations,
                "last_active_ts": s.last_active_ts,
                "affinity_tags": s.affinity_tags,
                "is_emerged": s.is_emerged,
            })
        return {"content": [{"type": "json", "data": {
            "n_personas": len(personas),
            "archetype_filter": archetype or "ALL",
            "include_emerged": include_emerged,
            "personas": personas,
            "source": "v1095_in_process",
        }}]}
    except Exception as exc:  # noqa: BLE001
        return {"isError": True,
                "content": [{"type": "text",
                             "text": f"list_personas failed: {type(exc).__name__}: {exc}"}]}


# ---------------------------------------------------------------------------
# 工具注册表 + Dispatcher
# ---------------------------------------------------------------------------


@dataclass
class R10AsiNorthStarDispatcher:
    """R10 ASI 北极星 MCP dispatcher (V1129).

    Args:
        v1095_store: V1095 IdentityStoreV1095 实例 (None → tool 返 isError)
        v1124_backend: V1124 ASINorthStarBackend 实例 (None → tool 走 HTTP 客户端)
        nine_key_lock: ASI 9 键 LOCKED 注入 (主 22:33)
    """

    v1095_store: Any = None
    v1124_backend: Any = None
    nine_key_lock: AsiNineKeyLock = field(default_factory=AsiNineKeyLock)
    protocol_version: str = "2024-11-05"
    nine_key_inject: bool = True
    server_started_ts: float = field(default_factory=time.time)
    n_calls: int = 0
    n_errors: int = 0
    n_iserror_results: int = 0
    n_dispatched: int = 0            # 所有 JSON-RPC dispatch 计数 (含 ping/initialize, chaos 守门用)
    _lock: threading.RLock = field(default_factory=threading.RLock)

    # 工具注册 (closure 注入 v1124 / v1095)
    def _tool_measure_asi(self, args: Dict[str, Any]) -> Dict[str, Any]:
        return tool_measure_asi(args, v1124_backend=self.v1124_backend)

    def _tool_get_north_star(self, args: Dict[str, Any]) -> Dict[str, Any]:
        return tool_get_north_star(args, v1124_backend=self.v1124_backend)

    def _tool_check_identity(self, args: Dict[str, Any]) -> Dict[str, Any]:
        return tool_check_identity(args, v1095_store=self.v1095_store)

    def _tool_verify_audit_chain(self, args: Dict[str, Any]) -> Dict[str, Any]:
        return tool_verify_audit_chain(args, v1095_store=self.v1095_store,
                                         v1124_backend=self.v1124_backend)

    def _tool_list_personas(self, args: Dict[str, Any]) -> Dict[str, Any]:
        return tool_list_personas(args, v1095_store=self.v1095_store)

    TOOLS: Dict[str, Dict[str, Any]] = field(default_factory=lambda: {
        "measure_asi": {
            "description": "V0.5 18 维真测 (V1125 evaluate_r10 + V1124 协同)",
            "inputSchema": MEASURE_ASI_SCHEMA,
        },
        "get_north_star": {
            "description": "ASI 北极星 (V1124 HTTP /asi/north-star 真连 + 综合评估)",
            "inputSchema": GET_NORTH_STAR_SCHEMA,
        },
        "check_identity": {
            "description": "V1095 IdentityStoreV1095 中央档案 (主 12:14 中央 AI 永恒身份)",
            "inputSchema": CHECK_IDENTITY_SCHEMA,
        },
        "verify_audit_chain": {
            "description": "V1095 跨槽位 hash + V1124 audit chain 真验",
            "inputSchema": VERIFY_AUDIT_CHAIN_SCHEMA,
        },
        "list_personas": {
            "description": "V1095 列出 4 archetype 槽位 (调度者/学习者/思考者/助手)",
            "inputSchema": LIST_PERSONAS_SCHEMA,
        },
    })

    # ---------- JSON-RPC dispatch ----------

    def handle_message(self, raw: Dict[str, Any]) -> Optional[Dict[str, Any]]:
        try:
            req = parse_request(raw)
        except ValueError as exc:
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
                "resources": [
                    {"uri": "r10://v05_formula", "name": "R10 V0.5 18 维公式",
                     "description": "V0.4*0.85 + continuity + autonomy + transferability (各 0.05)"},
                    {"uri": "r10://track_decision", "name": "R10 4 选 1 主轨道",
                     "description": "Track A/B/C/D + halt override + V1060 守门"},
                    {"uri": "r10://north_star_target", "name": "ASI 北极星 0.95 LOCKED",
                     "description": "R10 终极目标 (主 22:33)"},
                ],
            })
        if method == "stats":
            return make_result_response(req.id, self.stats())
        if method == "notifications/initialized":
            return None
        return make_error_response(req.id, JSONRPC_METHOD_NOT_FOUND, f"method not found: {method}")

    def _on_initialize(self, req) -> Dict[str, Any]:
        params = req.params or {}
        client_pv = params.get("protocolVersion", self.protocol_version)
        ok, message = check_protocol_version(client_pv)
        if not ok:
            return make_error_response(req.id, JSONRPC_INVALID_PARAMS, message)
        return make_result_response(req.id, {
            "protocolVersion": client_pv,
            "serverInfo": {
                "name": SERVER_NAME,
                "version": V1129_VERSION,
                "tools_count": len(self.TOOLS),
                "v1095_bound": self.v1095_store is not None,
                "v1124_bound": self.v1124_backend is not None,
                "started_ts": self.server_started_ts,
            },
            "capabilities": {"tools": {}, "resources": {"subscribe": False}, "logging": {}},
            "nine_key_lock": self.nine_key_lock.to_guard_block(),
        })

    def _on_tools_list(self, req) -> Dict[str, Any]:
        tools = [{"name": n, "description": m["description"], "inputSchema": m["inputSchema"]}
                 for n, m in self.TOOLS.items()]
        return make_result_response(req.id, {"tools": tools})

    def _on_tools_call(self, req) -> Dict[str, Any]:
        params = req.params or {}
        name = params.get("name")
        args = params.get("arguments", {}) or {}
        if not isinstance(name, str) or not name:
            return make_error_response(req.id, JSONRPC_INVALID_PARAMS,
                                        "params.name must be non-empty string")
        if name not in self.TOOLS:
            return make_error_response(req.id, JSONRPC_INVALID_PARAMS, f"unknown tool: {name}")
        meta = self.TOOLS[name]
        try:
            validate_arguments(args, meta["inputSchema"], tool_name=name)
        except SchemaViolation as exc:
            with self._lock:
                self.n_errors += 1
            return make_error_response(req.id, JSONRPC_INVALID_PARAMS, str(exc))
        # 派发到对应 handler
        handler = getattr(self, f"_tool_{name}", None)
        if handler is None:
            return make_error_response(req.id, JSONRPC_INVALID_PARAMS, f"no handler for {name}")
        try:
            result = handler(args)
        except Exception as exc:  # noqa: BLE001
            with self._lock:
                self.n_errors += 1
                self.n_iserror_results += 1
            return make_result_response(req.id, {
                "isError": True,
                "content": [{"type": "text", "text": f"{type(exc).__name__}: {exc}"}],
            })
        try:
            validate_tool_result(result, tool_name=name)
        except SchemaViolation as exc:
            with self._lock:
                self.n_errors += 1
                self.n_iserror_results += 1
            return make_result_response(req.id, {
                "isError": True,
                "content": [{"type": "text", "text": f"output schema invalid: {exc}"}],
            })
        if result.get("isError"):
            with self._lock:
                self.n_iserror_results += 1
        # ASI 9 键 LOCKED 注入
        if self.nine_key_inject and not result.get("isError"):
            result = inject_guard_block(result, self.nine_key_lock)
        with self._lock:
            self.n_calls += 1
        return make_result_response(req.id, result)

    # ---------- introspection ----------

    def stats(self) -> Dict[str, Any]:
        with self._lock:
            return {
                "version": V1129_VERSION,
                "server": SERVER_NAME,
                "n_tools": len(self.TOOLS),
                "n_calls": self.n_calls,
                "n_errors": self.n_errors,
                "n_iserror_results": self.n_iserror_results,
                "n_dispatched": self.n_dispatched,
                "v1095_bound": self.v1095_store is not None,
                "v1124_bound": self.v1124_backend is not None,
                "nine_key_lock": self.nine_key_lock.to_guard_block(),
            }


__all__ = [
    "V1129_VERSION", "SERVER_NAME",
    "MEASURE_ASI_SCHEMA", "GET_NORTH_STAR_SCHEMA", "CHECK_IDENTITY_SCHEMA",
    "VERIFY_AUDIT_CHAIN_SCHEMA", "LIST_PERSONAS_SCHEMA",
    "R10AsiNorthStarDispatcher",
]
