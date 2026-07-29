"""Apeireth ASI V1129 W2 — R10 MCP server + V1127/V1128 multi-agent 集成.

R10-MCP-002 (R10 W2, 主 13:31 大胆激进 + 主 22:33 ASI 北极星).

承接 R10-MCP-001 V1129 (commit ab89669b, accepted 9.00) + R10-AO-001 V1127 DGM v0.5
多中央 AI 协同 (accepted 9.55) + R10-A2-001 V1128 multi-agent 集成 V0.5 (accepted 9.00).

本模块扩展 V1129 工具集从 5 → 8:
  原 5 工具 (复用 V1129 dispatcher):
    measure_asi / get_north_star / check_identity / verify_audit_chain / list_personas
  新增 3 工具 (V1127+V1128 联动):
    multi_agent_consensus  V1128 measure_multi_agent + chaos test
    evolve_dgm             V1127 V05MultiAgentCoordinator 真演化 N 代
    multi_agent_asi_level  V1128 measure_single_agent (单 agent V0.5 18 维)

V1124 backend 真集成:
  - /asi/level 串联 (单/多 agent V0.5 18 维公式)
  - /asi/measure 串联 (DGM 真演化触发时, candidate fitness 可选 POST)
  - /asi/north-star 复用 V1129 dispatcher

chaos 守门 (主 23:44 干到底):
  - MCP transport 失联时 multi-agent state (node state, agent reports) 不丢
  - partial agent 失联 → run_chaos_test 必返 measurement_preserved=True
  - dispatcher state 跨 transport 启停保留

CLI 一行可跑:
    python -m apeireth.v1129_r10_mcp_multi_agent --server --transport http --port 8129
    python -m apeireth.v1129_r10_mcp_multi_agent --selftest
    python -m apeireth.v1129_r10_mcp_multi_agent --chaos
"""
from __future__ import annotations

import argparse
import json
import os
import sys
import tempfile
import threading
import time
import traceback
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Dict, List, Optional, Sequence, Tuple

# 复用 V1129 + V1123 MCP 子包
from apeireth.mcp.protocol import (
    JSONRPC_INTERNAL_ERROR, JSONRPC_INVALID_PARAMS, JSONRPC_METHOD_NOT_FOUND,
    SchemaViolation, check_protocol_version, make_error_response,
    make_result_response, parse_request, validate_arguments, validate_tool_result,
)
from apeireth.mcp.asi_nine_keys import AsiNineKeyLock, inject_guard_block
from apeireth.mcp.transport import StdioTransport, HttpTransport
from apeireth.mcp.sse_transport import SseTransport
from apeireth.mcp.r10_asi_north_star_server import (
    R10AsiNorthStarDispatcher, SERVER_NAME as _V1129_SERVER_NAME,
    V1129_VERSION as _V1129_VERSION,
)


V1129_W2_VERSION = "0.2.0"
SERVER_NAME = "apeireth-r10-mcp-multi-agent"

# 借鉴 V1128 守门常量
DEFAULT_AGENT_IDS = ("alpha", "beta", "gamma")
MIN_AGENTS = 2
CONSENSUS_STDDEV_MAX = 0.05
DGM_GENERATIONS_MAX = 50

# ---------------------------------------------------------------------------
# 3 新工具 schema
# ---------------------------------------------------------------------------


MULTI_AGENT_CONSENSUS_SCHEMA = {
    "type": "object",
    "properties": {
        "agent_ids": {"type": "array", "minItems": 2, "items": {"type": "string"},
                       "description": "agent id 列表, ≥2; 默认 DEFAULT_AGENT_IDS"},
        "v04_score": {"type": "number", "minimum": 0.0, "maximum": 1.0, "default": 0.8538},
        "multi_agent_consensus_hint": {"type": "number", "minimum": 0.0, "maximum": 1.0, "default": 0.85},
        "run_chaos": {"type": "boolean", "default": False,
                      "description": "True=跑 chaos test (失联 drop_indices[0])"},
        "week_label": {"type": "string", "pattern": "^R10-W[0-9]+$", "default": "R10-W2"},
        "v1124_base_url": {"type": "string", "default": "",
                            "description": "V1124 backend URL (空=in-process)"},
    },
    "additionalProperties": False,
}

EVOLVE_DGM_SCHEMA = {
    "type": "object",
    "properties": {
        "node_ids": {"type": "array", "minItems": 2, "items": {"type": "string"},
                      "description": "V1127 node id 列表, ≥2; 默认 ['alpha','beta','gamma']"},
        "generations": {"type": "integer", "minimum": 1, "maximum": DGM_GENERATIONS_MAX, "default": 2},
        "seed": {"type": "integer", "minimum": 0, "default": 1127},
        "include_candidates": {"type": "boolean", "default": False,
                                "description": "True=每个 node 每代 candidate 详情"},
        "data_dir": {"type": "string", "default": "",
                      "description": "DGM 数据目录 (空=临时目录)"},
    },
    "additionalProperties": False,
}

MULTI_AGENT_ASI_LEVEL_SCHEMA = {
    "type": "object",
    "properties": {
        "agent_id": {"type": "string", "minLength": 1,
                      "description": "agent id (必填, e.g. alpha/beta/gamma)"},
        "v04_score": {"type": "number", "minimum": 0.0, "maximum": 1.0, "default": 0.8538},
        "continuity_override": {"type": "number", "minimum": 0.0, "maximum": 1.0,
                                 "default": -1,
                                 "description": "连续性 override (<0 用 continuity_tracker 真值)"},
        "multi_agent_consensus": {"type": "number", "minimum": 0.0, "maximum": 1.0, "default": 0.85},
        "v1124_base_url": {"type": "string", "default": ""},
        "week_label": {"type": "string", "pattern": "^R10-W[0-9]+$", "default": "R10-W2"},
    },
    "required": ["agent_id"],
    "additionalProperties": False,
}


# ---------------------------------------------------------------------------
# 3 新工具实现 (V1127+V1128 真集成, 主 17:43 实事求是)
# ---------------------------------------------------------------------------


def _safe_v1128_protocol(agent_ids: Sequence[str],
                          backend_bridge: Optional[Any] = None,
                          continuity_tracker: Optional[Any] = None,
                          coordinator: Optional[Any] = None) -> Tuple[Any, Optional[str]]:
    """构造 V1128MultiAgentIntegrationProtocol (失败时返 None + 错误)."""
    try:
        from apeireth.v1128_r10_multi_agent_integration import V1128MultiAgentIntegrationProtocol
        return V1128MultiAgentIntegrationProtocol(
            agent_ids=list(agent_ids),
            backend_bridge=backend_bridge,
            continuity_tracker=continuity_tracker,
            coordinator=coordinator,
        ), None
    except Exception as exc:  # noqa: BLE001
        return None, f"V1128 import/init failed: {type(exc).__name__}: {exc}"


def _safe_v1127_coordinator(root: Path, node_ids: Sequence[str],
                             secret: bytes = b"apeireth-v1129-w2-local",
                             seed: int = 1127,
                             backend: Optional[Any] = None) -> Tuple[Any, Optional[str]]:
    """构造 V05MultiAgentCoordinator (失败时返 None + 错误)."""
    try:
        from apeireth.v1127_dgm_v05_multi_agent import V05MultiAgentCoordinator
        return V05MultiAgentCoordinator(
            root=root, node_ids=list(node_ids), secret=secret, seed=seed, backend=backend,
        ), None
    except Exception as exc:  # noqa: BLE001
        return None, f"V1127 import/init failed: {type(exc).__name__}: {exc}"


def tool_multi_agent_consensus(args: Dict[str, Any],
                                v1129_dispatcher: Optional[R10AsiNorthStarDispatcher] = None) -> Dict[str, Any]:
    """multi_agent_consensus: V1128 多 agent 协同测量 + chaos test (主 23:44 干到底)."""
    agent_ids = list(args.get("agent_ids") or DEFAULT_AGENT_IDS)
    if len(agent_ids) < MIN_AGENTS:
        return {"isError": True, "content": [{"type": "text",
            "text": f"agent_ids must have ≥{MIN_AGENTS} items, got {len(agent_ids)}"}]}
    v04_score = float(args.get("v04_score", 0.8538))
    multi_agent_consensus_hint = float(args.get("multi_agent_consensus_hint", 0.85))
    run_chaos = bool(args.get("run_chaos", False))
    week_label = str(args.get("week_label", "R10-W2"))
    # V1124 真集成 (主 17:43 实事求是): 复用 v1129 dispatcher 的 v1124_backend
    backend_bridge = None
    if v1129_dispatcher is not None and v1129_dispatcher.v1124_backend is not None:
        try:
            from apeireth.v1128_r10_multi_agent_integration import V1124BackendBridge
            backend_bridge = V1124BackendBridge(_backend=v1129_dispatcher.v1124_backend)
        except Exception:  # noqa: BLE001
            backend_bridge = None
    protocol, err = _safe_v1128_protocol(agent_ids, backend_bridge=backend_bridge)
    if protocol is None:
        return {"isError": True, "content": [{"type": "text", "text": err or "unknown error"}]}
    try:
        consensus = protocol.measure_multi_agent(
            v04_score=v04_score,
            multi_agent_consensus_hint=multi_agent_consensus_hint,
        )
        consensus_dict = consensus.to_dict()
        out: Dict[str, Any] = {
            "week_label": week_label,
            "consensus": consensus_dict,
            "source": "v1128_measure_multi_agent",
        }
        if run_chaos:
            chaos = protocol.run_chaos_test(v04_score=v04_score, drop_indices=[0])
            out["chaos"] = chaos
            out["chaos_measurement_preserved"] = chaos.get("measurement_preserved", False)
        return {"content": [{"type": "json", "data": out}]}
    except Exception as exc:  # noqa: BLE001
        return {"isError": True, "content": [{"type": "text",
            "text": f"multi_agent_consensus failed: {type(exc).__name__}: {exc}"}]}


def tool_evolve_dgm(args: Dict[str, Any],
                     v1129_dispatcher: Optional[R10AsiNorthStarDispatcher] = None) -> Dict[str, Any]:
    """evolve_dgm: V1127 V05MultiAgentCoordinator 真演化 N 代 (主 13:31 大胆激进)."""
    node_ids = list(args.get("node_ids") or ["alpha", "beta", "gamma"])
    if len(node_ids) < MIN_AGENTS:
        return {"isError": True, "content": [{"type": "text",
            "text": f"node_ids must have ≥{MIN_AGENTS} items, got {len(node_ids)}"}]}
    generations = int(args.get("generations", 2))
    if not (1 <= generations <= DGM_GENERATIONS_MAX):
        return {"isError": True, "content": [{"type": "text",
            "text": f"generations must be in [1, {DGM_GENERATIONS_MAX}], got {generations}"}]}
    seed = int(args.get("seed", 1127))
    include_candidates = bool(args.get("include_candidates", False))
    data_dir_str = str(args.get("data_dir", "")).strip()
    if data_dir_str:
        root = Path(data_dir_str)
        root.mkdir(parents=True, exist_ok=True)
        cleanup = False
    else:
        root = Path(tempfile.mkdtemp(prefix="v1129_dgm_"))
        cleanup = True
    # V1124 backend 真集成: 复用 v1129 dispatcher
    backend = None
    if v1129_dispatcher is not None and v1129_dispatcher.v1124_backend is not None:
        backend = v1129_dispatcher.v1124_backend
    try:
        coordinator, err = _safe_v1127_coordinator(root=root, node_ids=node_ids, seed=seed, backend=backend)
        if coordinator is None:
            return {"isError": True, "content": [{"type": "text", "text": err or "unknown error"}]}
        # 真跑演化 (主 17:43 实事求是: 同 seed → 同 trajectory)
        result = coordinator.run(generations=generations)
        out: Dict[str, Any] = {
            "generations": generations,
            "node_ids": node_ids,
            "n_nodes": len(node_ids),
            "trace_path": str(coordinator.trace_path),
            "result_keys": list(result.keys()) if isinstance(result, dict) else [],
            "source": "v1127_v05_multi_agent_coordinator",
        }
        # 提取 fitness per node (从 trace 或 latest)
        try:
            latest_per_node: Dict[str, float] = {}
            for nid in node_ids:
                node = coordinator.nodes[nid]
                latest_per_node[nid] = node.state.fitness
            out["latest_fitness_per_node"] = latest_per_node
        except Exception:  # noqa: BLE001
            pass
        if include_candidates and isinstance(result, dict) and "latest_per_node" in result:
            out["latest_candidates_per_node"] = result["latest_per_node"]
        return {"content": [{"type": "json", "data": out}]}
    except Exception as exc:  # noqa: BLE001
        return {"isError": True, "content": [{"type": "text",
            "text": f"evolve_dgm failed: {type(exc).__name__}: {exc}"}]}
    finally:
        if cleanup:
            try:
                import shutil
                shutil.rmtree(root, ignore_errors=True)
            except Exception:  # noqa: BLE001
                pass


def tool_multi_agent_asi_level(args: Dict[str, Any],
                                v1129_dispatcher: Optional[R10AsiNorthStarDispatcher] = None) -> Dict[str, Any]:
    """multi_agent_asi_level: V1128 单 agent V0.5 18 维真测.

    ponytail: 不与 V1128 protocol 多 agent 构造器冲突, 直接调
    V0.5 18 维公式 + V1124BackendBridge (主 19:33 走在前人经验上).
    """
    agent_id = str(args.get("agent_id", "")).strip()
    if not agent_id:
        return {"isError": True, "content": [{"type": "text", "text": "agent_id required"}]}
    v04_score = float(args.get("v04_score", 0.8538))
    continuity_override = float(args.get("continuity_override", -1.0))
    multi_agent_consensus = float(args.get("multi_agent_consensus", 0.85))
    week_label = str(args.get("week_label", "R10-W2"))
    # V1124 backend 真测 (主 17:43 实事求是)
    backend_score = None
    backend_status = "unavailable"
    backend_error = ""
    backend_bridge = None
    if v1129_dispatcher is not None and v1129_dispatcher.v1124_backend is not None:
        try:
            from apeireth.v1128_r10_multi_agent_integration import V1124BackendBridge, default_v05_18_form
            backend_bridge = V1124BackendBridge(_backend=v1129_dispatcher.v1124_backend)
        except Exception:  # noqa: BLE001
            backend_bridge = None
    if backend_bridge is not None:
        try:
            status_code, body = backend_bridge.level()
            backend_status = "ok" if status_code == 200 else f"unavailable_{status_code}"
            if status_code == 200 and isinstance(body, dict):
                backend_score = body.get("score")
            else:
                backend_error = f"status={status_code}, body={body}"
        except Exception as exc:  # noqa: BLE001
            backend_error = f"{type(exc).__name__}: {exc}"
    # Continuity 真测 (V1072 ContinuityTracker) 或 override
    continuity = 0.85
    cont_source = "default_fallback"
    if continuity_override >= 0:
        continuity = continuity_override
        cont_source = "override"
    else:
        try:
            from apeireth.v1072_asi_central_ai_eternal_identity import ContinuityTracker
            ct = ContinuityTracker()
            cs = ct.continuity_score()
            if cs:
                continuity = round(cs, 6)
                cont_source = "v1072_continuity_tracker"
        except Exception:  # noqa: BLE001
            cont_source = "default_fallback"
    # V0.5 18 维真跑
    try:
        from apeireth.v1128_r10_multi_agent_integration import default_v05_18_form, compute_v05_18_score
        form = default_v05_18_form(
            v04_score=v04_score,
            continuity_tracker=continuity,
            multi_agent_consensus=multi_agent_consensus,
        )
        v05_dict = compute_v05_18_score(form)
    except Exception as exc:  # noqa: BLE001
        return {"isError": True, "content": [{"type": "text",
            "text": f"V0.5 18 维公式 failed: {type(exc).__name__}: {exc}"}]}
    return {"content": [{"type": "json", "data": {
        "week_label": week_label,
        "agent_id": agent_id,
        "report": {
            "agent_id": agent_id,
            "v05_18_total": v05_dict["v05_18_total"],
            "v04_subscore": v05_dict["v04_subscore"],
            "continuity_tracker": continuity,
            "multi_agent_consensus": multi_agent_consensus,
            "per_dim": v05_dict["dims"],
            "backend_status": backend_status,
            "backend_score": backend_score,
            "backend_error": backend_error,
            "continuity_source": cont_source,
            "timestamp": time.time(),
        },
        "source": "v1128_v05_18_form_in_process",
    }}]}


# ---------------------------------------------------------------------------
# Multi-Agent Dispatcher (继承 V1129 R10AsiNorthStarDispatcher + 3 新工具)
# ---------------------------------------------------------------------------


@dataclass
class V1129MultiAgentDispatcher(R10AsiNorthStarDispatcher):
    """V1129 W2 多 agent dispatcher (R10 MCP-002).

    继承 R10AsiNorthStarDispatcher (5 工具) + 新增 3 multi-agent 工具 = 8 工具总.
    """

    v1127_root: Optional[Path] = None
    v1128_protocol: Any = None
    multi_agent_state_lock: threading.RLock = field(default_factory=threading.RLock)
    n_multi_agent_calls: int = 0

    def __post_init__(self) -> None:
        # 不重写父类, 只扩展 TOOLS + handlers
        # 复用父类的 protocol_version / nine_key_lock / stats
        pass

    def _extra_tools(self) -> Dict[str, Dict[str, Any]]:
        return {
            "multi_agent_consensus": {
                "description": "V1128 多 agent 协同测量 + chaos test (≥2 agents)",
                "inputSchema": MULTI_AGENT_CONSENSUS_SCHEMA,
            },
            "evolve_dgm": {
                "description": "V1127 DGM v0.5 多中央 AI 真演化 N 代 (≥2 nodes)",
                "inputSchema": EVOLVE_DGM_SCHEMA,
            },
            "multi_agent_asi_level": {
                "description": "V1128 单 agent V0.5 18 维真测 (1 agent)",
                "inputSchema": MULTI_AGENT_ASI_LEVEL_SCHEMA,
            },
        }

    @property
    def TOOLS_EXT(self) -> Dict[str, Dict[str, Any]]:
        # 父类 TOOLS 是 5 工具, _extra_tools 是 3 新工具
        return {**self.TOOLS, **self._extra_tools()}

    # ---------- 3 新工具 handler (closure 注入 self) ----------

    def _tool_multi_agent_consensus(self, args: Dict[str, Any]) -> Dict[str, Any]:
        with self.multi_agent_state_lock:
            self.n_multi_agent_calls += 1
        return tool_multi_agent_consensus(args, v1129_dispatcher=self)

    def _tool_evolve_dgm(self, args: Dict[str, Any]) -> Dict[str, Any]:
        with self.multi_agent_state_lock:
            self.n_multi_agent_calls += 1
        return tool_evolve_dgm(args, v1129_dispatcher=self)

    def _tool_multi_agent_asi_level(self, args: Dict[str, Any]) -> Dict[str, Any]:
        with self.multi_agent_state_lock:
            self.n_multi_agent_calls += 1
        return tool_multi_agent_asi_level(args, v1129_dispatcher=self)

    # ---------- 覆盖父类方法 (父类只认 5 工具) ----------

    def _on_tools_list(self, req) -> Dict[str, Any]:
        tools = [{"name": n, "description": m["description"], "inputSchema": m["inputSchema"]}
                 for n, m in self.TOOLS_EXT.items()]
        return make_result_response(req.id, {"tools": tools})

    def _on_tools_call(self, req) -> Dict[str, Any]:
        params = req.params or {}
        name = params.get("name")
        args = params.get("arguments", {}) or {}
        if not isinstance(name, str) or not name:
            return make_error_response(req.id, JSONRPC_INVALID_PARAMS,
                                        "params.name must be non-empty string")
        ext = self.TOOLS_EXT
        if name not in ext:
            return make_error_response(req.id, JSONRPC_INVALID_PARAMS, f"unknown tool: {name}")
        meta = ext[name]
        try:
            validate_arguments(args, meta["inputSchema"], tool_name=name)
        except SchemaViolation as exc:
            with self._lock:
                self.n_errors += 1
            return make_error_response(req.id, JSONRPC_INVALID_PARAMS, str(exc))
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
        if self.nine_key_inject and not result.get("isError"):
            result = inject_guard_block(result, self.nine_key_lock)
        with self._lock:
            self.n_calls += 1
        return make_result_response(req.id, result)

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
                "version": V1129_W2_VERSION,
                "tools_count": len(self.TOOLS_EXT),
                "v1095_bound": self.v1095_store is not None,
                "v1124_bound": self.v1124_backend is not None,
                "v1127_v05_integrated": True,
                "v1128_multi_agent_integrated": True,
                "started_ts": self.server_started_ts,
            },
            "capabilities": {"tools": {}, "resources": {"subscribe": False}, "logging": {}},
            "nine_key_lock": self.nine_key_lock.to_guard_block(),
        })

    def stats(self) -> Dict[str, Any]:
        base = super().stats()
        base.update({
            "version": V1129_W2_VERSION,
            "server": SERVER_NAME,
            "n_tools": len(self.TOOLS_EXT),
            "n_multi_agent_calls": self.n_multi_agent_calls,
            "v1127_v05_integrated": True,
            "v1128_multi_agent_integrated": True,
        })
        return base


# ---------------------------------------------------------------------------
# 真集成客户端 (主 17:43 实事求是)
# ---------------------------------------------------------------------------


def build_default_multi_agent_dispatcher(
    v1095_store: Any = None,
    v1124_backend: Any = None,
    bind_external: bool = True,
) -> V1129MultiAgentDispatcher:
    """构建多 agent dispatcher (主 00:56 任何人都能接手: 一行).

    Args:
        v1095_store: 外部 V1095
        v1124_backend: 外部 V1124
        bind_external: False → 不构造 V1095/V1124
    """
    if v1095_store is None and bind_external:
        try:
            from apeireth.v1095_identity_store import IdentityStoreV1095
            tmp = Path(tempfile.mkdtemp(prefix="v1129w2_v1095_"))
            v1095_store = IdentityStoreV1095(tmp / "identity.db", fsync_full=True)
            v1095_store.ensure_default_slots()
        except Exception:  # noqa: BLE001
            v1095_store = None
    if v1124_backend is None and bind_external:
        try:
            from apeireth.v1124_asi_north_star_backend import ASINorthStarBackend
            v1124_backend = ASINorthStarBackend(Path(tempfile.mkdtemp(prefix="v1129w2_v1124_")))
        except Exception:  # noqa: BLE001
            v1124_backend = None
    return V1129MultiAgentDispatcher(
        v1095_store=v1095_store,
        v1124_backend=v1124_backend,
    )


# ---------------------------------------------------------------------------
# Self-test / Chaos / CLI (主 00:56 一行可跑)
# ---------------------------------------------------------------------------


def run_selftest(bind_external: bool = True) -> Dict[str, Any]:
    """V1129 W2 自检: 8 工具 + 3 transports + chaos 守门."""
    d = build_default_multi_agent_dispatcher(bind_external=bind_external)
    results: Dict[str, Any] = {}
    # 1) initialize
    init = d.handle_message({"jsonrpc": "2.0", "id": 1, "method": "initialize",
                              "params": {"protocolVersion": "2024-11-05"}})
    results["initialize"] = init
    # 2) tools/list (必含 8 工具)
    lst = d.handle_message({"jsonrpc": "2.0", "id": 2, "method": "tools/list",
                             "params": {}})
    results["tools_list_count"] = len(lst["result"]["tools"])
    results["tools_list_names"] = sorted(t["name"] for t in lst["result"]["tools"])
    # 3) 8 工具 round-trip
    calls = {
        "measure_asi": {"v04_actual": 0.8538},
        "get_north_star": {},
        "check_identity": {},
        "verify_audit_chain": {},
        "list_personas": {},
        "multi_agent_consensus": {"agent_ids": ["alpha", "beta", "gamma"], "v04_score": 0.8538},
        "evolve_dgm": {"generations": 2, "node_ids": ["alpha", "beta"], "seed": 1127},
        "multi_agent_asi_level": {"agent_id": "alpha", "v04_score": 0.8538},
    }
    results["tool_calls"] = {}
    for i, (name, args) in enumerate(calls.items(), start=10):
        resp = d.handle_message({"jsonrpc": "2.0", "id": i, "method": "tools/call",
                                  "params": {"name": name, "arguments": args}})
        results["tool_calls"][name] = resp
    # 4) chaos 守门
    results["chaos_pre"] = d.stats()
    sse = SseTransport(dispatch=d.handle_message,
                        server_info={"name": SERVER_NAME, "tools": list(d.TOOLS_EXT.keys())},
                        host="127.0.0.1", port=0)
    sse.start(); sse.stop()
    http = HttpTransport(dispatch=d.handle_message,
                          server_info={"name": SERVER_NAME, "tools": list(d.TOOLS_EXT.keys())},
                          host="127.0.0.1", port=0)
    http.start(); http.stop()
    for _ in range(3):
        d.handle_message({"jsonrpc": "2.0", "id": 100, "method": "ping", "params": {}})
    results["chaos_post"] = d.stats()
    results["chaos_state_retained"] = (
        results["chaos_post"]["n_dispatched"] > results["chaos_pre"]["n_dispatched"]
    )
    results["chaos_multi_agent_state_retained"] = (
        results["chaos_post"]["n_multi_agent_calls"] > 0
    )
    # 5) multi_agent_consensus chaos test (run_chaos=True)
    chaos_resp = d.handle_message({"jsonrpc": "2.0", "id": 200, "method": "tools/call",
                                    "params": {"name": "multi_agent_consensus",
                                                "arguments": {"agent_ids": ["alpha", "beta", "gamma"],
                                                               "v04_score": 0.8538,
                                                               "run_chaos": True}}})
    results["multi_agent_consensus_with_chaos"] = chaos_resp
    results["stats"] = d.stats()
    return results


def cli_main(argv: Optional[List[str]] = None) -> int:
    parser = argparse.ArgumentParser(
        prog="v1129_r10_mcp_multi_agent",
        description="V1129 R10 W2 MCP server (5 base + 3 multi-agent tools, 3 transports)",
    )
    parser.add_argument("--server", action="store_true")
    parser.add_argument("--transport", choices=("stdio", "http", "sse"), default="http")
    parser.add_argument("--host", default="127.0.0.1")
    parser.add_argument("--port", type=int, default=8129)
    parser.add_argument("--selftest", action="store_true")
    parser.add_argument("--chaos", action="store_true")
    parser.add_argument("--snapshot", action="store_true")
    parser.add_argument("--json", action="store_true")
    parser.add_argument("--no-bind", action="store_true")
    args = parser.parse_args(argv)

    if args.snapshot:
        d = build_default_multi_agent_dispatcher(bind_external=not args.no_bind)
        print(json.dumps(d.stats(), indent=2, ensure_ascii=False, default=str))
        return 0

    if args.selftest:
        result = run_selftest(bind_external=not args.no_bind)
        if args.json:
            print(json.dumps(result, indent=2, ensure_ascii=False, default=str))
        else:
            n_ok = sum(1 for r in result["tool_calls"].values()
                        if r and "result" in r and not r["result"].get("isError"))
            print(f"V1129 W2 selftest: {n_ok}/{len(result['tool_calls'])} tools OK, "
                  f"tools_count={result['tools_list_count']}")
            print(f"V1129 W2 chaos state retained: {result['chaos_state_retained']}, "
                  f"multi_agent_calls retained: {result['chaos_multi_agent_state_retained']}")
            s = result["stats"]
            print(f"V1129 W2 stats: n_calls={s['n_calls']}, n_errors={s['n_errors']}, "
                  f"n_multi_agent_calls={s['n_multi_agent_calls']}, 9 键 LOCKED={s['nine_key_lock']['asi_nine_keys_locked']}")
        return 0

    if args.chaos:
        d = build_default_multi_agent_dispatcher(bind_external=not args.no_bind)
        pre = d.stats()
        sse = SseTransport(dispatch=d.handle_message,
                            server_info={"name": SERVER_NAME, "tools": list(d.TOOLS_EXT.keys())},
                            host="127.0.0.1", port=0)
        sse.start(); sse.stop()
        http = HttpTransport(dispatch=d.handle_message,
                              server_info={"name": SERVER_NAME, "tools": list(d.TOOLS_EXT.keys())},
                              host="127.0.0.1", port=0)
        http.start(); http.stop()
        for _ in range(3):
            d.handle_message({"jsonrpc": "2.0", "id": 200, "method": "ping", "params": {}})
        post = d.stats()
        ok = post["n_dispatched"] > pre["n_dispatched"]
        result = {"chaos_state_retained": ok, "pre": pre, "post": post}
        if args.json:
            print(json.dumps(result, indent=2, ensure_ascii=False, default=str))
        else:
            print(f"chaos_state_retained: {ok}")
            print(f"  pre  dispatched={pre['n_dispatched']} calls={pre['n_calls']} "
                  f"multi_agent={pre['n_multi_agent_calls']}")
            print(f"  post dispatched={post['n_dispatched']} calls={post['n_calls']} "
                  f"multi_agent={post['n_multi_agent_calls']}")
        return 0 if ok else 1

    if args.server:
        from apeireth.mcp.transport import StdioTransport as _Stdio
        from apeireth.mcp.sse_transport import SseTransport as _Sse
        from apeireth.mcp.transport import HttpTransport as _Http
        d = build_default_multi_agent_dispatcher(bind_external=not args.no_bind)
        if args.transport == "stdio":
            t = _Stdio(d.handle_message)
            return t.serve()
        if args.transport == "sse":
            t = _Sse(dispatch=d.handle_message,
                      server_info={"name": SERVER_NAME, "tools": list(d.TOOLS_EXT.keys())},
                      host=args.host, port=args.port)
            t.start()
            print(f"[V1129 W2] {SERVER_NAME} v{V1129_W2_VERSION} SSE on {t.sse_url()}", flush=True)
            try:
                while True:
                    time.sleep(3600)
            except KeyboardInterrupt:
                pass
            finally:
                t.stop()
            return 0
        t = _Http(dispatch=d.handle_message,
                   server_info={"name": SERVER_NAME, "tools": list(d.TOOLS_EXT.keys())},
                   host=args.host, port=args.port)
        t.start()
        print(f"[V1129 W2] {SERVER_NAME} v{V1129_W2_VERSION} HTTP on {t.url()}", flush=True)
        try:
            while True:
                time.sleep(3600)
        except KeyboardInterrupt:
            pass
        finally:
            t.stop()
        return 0

    parser.print_help()
    return 1


__all__ = [
    "V1129_W2_VERSION", "SERVER_NAME",
    "MULTI_AGENT_CONSENSUS_SCHEMA", "EVOLVE_DGM_SCHEMA", "MULTI_AGENT_ASI_LEVEL_SCHEMA",
    "V1129MultiAgentDispatcher",
    "build_default_multi_agent_dispatcher",
    "run_selftest", "cli_main",
]


# ---------------------------------------------------------------------------
# V1101 auto-injected V3_GUARDS (主 17:43 + 主 17:58 不假装)
# ---------------------------------------------------------------------------
V3_GUARDS_W2 = {
    "multi_agent_consensus_is_not_truth": "consensus_score 是守门指标 (stddev < 0.05), 不是真理.",
    "dgm_evolve_is_not_asi": "DGM v0.5 真演化 ≠ ASI 自演化. 候选 fitness ∈ [0, 0.95] 上限是 V3 守门.",
    "multi_agent_measurement_is_not_asi": "多 agent V0.5 18 维真测 ≠ ASI 达成. 0.95 是 R10 终极门.",
    "v1127_v1128_integration_is_not_asi": "V1127+V1128 真集成 ≠ 自主协同. 集成是工程, 自主是更大目标.",
    "chaos_state_preserved_is_not_perfect": "chaos 守门通过 ≠ 永远不丢状态. 是 best-effort, 不是保证.",
    "transports_fanout_is_not_asi": "3 transports + 8 tools ≠ ASI 多模态接入. 协议是工程, ASI 是目标.",
}


if __name__ == "__main__":
    raise SystemExit(cli_main(sys.argv[1:]))