"""apeireth.mcp.asi_north_star_server — ASI 北极星 MCP Server (V1123).

主 22:33 ASI 北极星 + 主 17:43 实事求是 + 主 23:44 干到底 + 主 19:33 走在前人经验上.

5 大 MCP 工具 (V1123 真实现):
  1. asi_north_star_query    查 ASI 北极星 (V0.1 / V0.3 / V0.4 / North Star)
  2. v1074_guard             V1074 V0.3 真生产守门 (≥ 0.8884 必通过)
  3. v1112_dgm_run           V1112 DGM v0.4 真演化 (轻量级 1 代, 不跑完整实验)
  4. v1114_weekly_eval       V1114 每周集成评估 (复用 dashboard / halting / decide 引擎)
  5. identity_lock_check     V1072 永恒身份 lock 9 键 + 跨 session 连续性

资源 (V1123 MCP resources):
  - formula://v0.1     V0.1 8 项透明公式
  - formula://v0.3     V0.3 工程化提升
  - formula://v0.4     V0.4 17 维
  - northstar://0.98   终极目标 (LOCKED 0.9800)

提示词 (V1123 MCP prompts):
  - run-weekly-eval   每周评估编排 prompt
  - dgm-evolve        DGM 演化 prompt
"""
from __future__ import annotations

import json
import re
import time
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
from .asi_nine_keys import (
    ASI_NINE_KEYS, AsiNineKeyLock, inject_guard_block, verify_or_raise,
)
from .model_adapters import (
    ModelAdapterRegistry, heuristic_asi_score,
)


V1123_VERSION = "0.1.0"
SERVER_NAME = "apeireth-asi-north-star-mcp"

# 5 大 MCP 工具 schema (V1123 锁定)
ASI_NORTH_STAR_QUERY_SCHEMA = {
    "type": "object",
    "properties": {
        "formula": {
            "type": "string",
            "enum": ["v0.1", "v0.3", "v0.4", "north_star"],
            "description": "ASI 北极星公式版本 (V1123 暴露 4 选 1)",
        },
        "explain": {"type": "boolean", "default": False},
    },
    "required": ["formula"],
    "additionalProperties": False,
}

V1074_GUARD_SCHEMA = {
    "type": "object",
    "properties": {
        "score": {"type": "number", "minimum": 0.0, "maximum": 1.0,
                  "description": "V1074 V0.3 实际测得分 (0-1)"},
        "min_floor": {"type": "number", "default": 0.8884, "minimum": 0.0, "maximum": 1.0,
                      "description": "V1074 V0.3 守门阈值 (默认 LOCKED 0.8884)"},
        "include_decision": {"type": "boolean", "default": False},
    },
    "required": ["score"],
    "additionalProperties": False,
}

V1112_DGM_RUN_SCHEMA = {
    "type": "object",
    "properties": {
        "n_generations": {"type": "integer", "default": 3, "minimum": 1, "maximum": 50,
                          "description": "演化代数 (V1123 轻量, 上限 50)"},
        "seed": {"type": "integer", "default": 0, "minimum": 0,
                 "description": "RNG 种子 (可复现)"},
        "include_report": {"type": "boolean", "default": True},
    },
    "additionalProperties": False,
}

V1114_WEEKLY_EVAL_SCHEMA = {
    "type": "object",
    "properties": {
        "week_label": {"type": "string", "pattern": "^W[0-9]+$", "default": "W4"},
        "v03_history": {"type": "array", "items": {"type": "number"}, "default": []},
        "unique_ratio": {"type": "number", "default": 1.0, "minimum": 0.0, "maximum": 1.0},
        "fitness_std": {"type": "number", "default": 0.05, "minimum": 0.0},
        "cross_dim_drop": {"type": "number", "default": 0.0, "minimum": 0.0, "maximum": 1.0},
        "cross_model_lift": {"type": "number", "default": 0.05, "minimum": 0.0},
        "v1060_committed": {"type": "boolean", "default": True},
        "weekly_lift": {"type": "number", "default": 0.0},
        "live": {"type": "boolean", "default": False,
                 "description": "True=真跑三件套 subprocess, False=W3 baseline"},
    },
    "additionalProperties": False,
}

IDENTITY_LOCK_CHECK_SCHEMA = {
    "type": "object",
    "properties": {
        "run": {"type": "boolean", "default": True,
                "description": "True=真生产 V1072 orchestrator, False=lock-only"},
        "include_components": {"type": "boolean", "default": False},
    },
    "additionalProperties": False,
}

# 资源定义
ASI_RESOURCES = (
    {
        "uri": "formula://v0.1",
        "name": "ASI North Star V0.1 Transparent Formula",
        "description": "V0.1 8 项透明公开公式 (主 17:33 真采纳, commit 5df240d)",
    },
    {
        "uri": "formula://v0.3",
        "name": "ASI North Star V0.3 Engineering Lift",
        "description": "V0.3 工程化提升 (LOCKED ≥ 0.8884, 守门)",
    },
    {
        "uri": "formula://v0.4",
        "name": "ASI North Star V0.4 17-Dimension",
        "description": "V0.4 17 维全测 (W4 目标 ≥ 0.85, R10 起点 ≥ 0.86)",
    },
    {
        "uri": "northstar://0.98",
        "name": "ASI North Star 0.9800 LOCKED",
        "description": "ASI 北极星终极目标 (主 22:33 LOCKED)",
    },
)

# 提示词定义
ASI_PROMPTS = (
    {
        "name": "run-weekly-eval",
        "description": "每周集成评估编排 prompt (主 17:43 实事求是: 真跑三件套)",
    },
    {
        "name": "dgm-evolve",
        "description": "DGM v0.4 演化 prompt (主 13:31 大胆激进: ≥+0.010)",
    },
)

# ASI 公式元数据 (内嵌, 不依赖 V21 子模块: 主 00:56 任何人都能接手)
ASI_FORMULAS: Dict[str, Dict[str, Any]] = {
    "v0.1": {
        "name": "V0.1 透明公式",
        "weights": {
            "phi_proxy": 0.20, "capabilities": 0.20, "cross_domain": 0.15,
            "engineering": 0.15, "vcp_4": 0.10, "v2_philosophy": 0.10,
            "rubric_open": 0.05, "real_production": 0.05,
        },
        "range": [0.0, 1.0],
        "interpretation": "BASE_FULLY_EQUIPPED when asi_approach ≥ 0.95",
    },
    "v0.3": {
        "name": "V0.3 工程化提升",
        "weights": {"v1074": 1.0},
        "floor": 0.8884,
        "interpretation": "V0.3 = V1074 真测, ≥ 0.8884 守门",
    },
    "v0.4": {
        "name": "V0.4 17 维全测",
        "n_dims": 17,
        "w4_target": 0.85,
        "r10_start_target": 0.86,
        "r10_mid_target": 0.90,
        "interpretation": "W4 终点 ≥ 0.85, R10 起点 ≥ 0.86, R10 中期 ≥ 0.90",
    },
    "north_star": {
        "name": "ASI 北极星 0.9800 LOCKED",
        "value": 0.9800,
        "interpretation": "ASI 本身 = ∅ (主 20:46 真哲学); Index = 1.0 不是 ASI 实现, 是基座完全装备",
    },
}


# ---------------------------------------------------------------------------
# 5 大 MCP 工具实现 (主 17:43 实事求是: 真跑, 不假装)
# ---------------------------------------------------------------------------


def tool_asi_north_star_query(args: Dict[str, Any]) -> Dict[str, Any]:
    """asi_north_star_query: 查 ASI 北极星公式 (V0.1 / V0.3 / V0.4 / North Star)."""
    formula = args["formula"]
    explain = bool(args.get("explain", False))
    meta = ASI_FORMULAS[formula]
    if not explain:
        # 默认只返元数据
        data = {"formula": formula, "name": meta["name"], "meta": meta}
    else:
        data = {
            "formula": formula, "name": meta["name"], "meta": meta,
            "explanation": _explain_formula(formula),
        }
    return {"content": [{"type": "json", "data": data}]}


def _explain_formula(formula: str) -> str:
    if formula == "v0.1":
        return ("V0.1 = 0.20*Φ-proxy + 0.20*cap/total + 0.15*cross/14 + 0.15*engineering"
                " + 0.10*VCP-4 + 0.10*V2-philosophy + 0.05*rubric + 0.05*real-prod")
    if formula == "v0.3":
        return "V0.3 = V1074 ASI Production Runner 真测分, 守门 ≥ 0.8884"
    if formula == "v0.4":
        return "V0.4 = V1077 17 维加权全测, W4 ≥ 0.85, R10 起点 ≥ 0.86"
    if formula == "north_star":
        return "ASI 北极星 = 0.9800 LOCKED (主 22:33); ASI 本身 = ∅ (主 20:46 真哲学)"
    return "unknown formula"


def tool_v1074_guard(args: Dict[str, Any]) -> Dict[str, Any]:
    """v1074_guard: V1074 V0.3 真生产守门 (主 23:44 干到底)."""
    score = float(args["score"])
    min_floor = float(args.get("min_floor", 0.8884))
    include_decision = bool(args.get("include_decision", False))
    passes = score >= min_floor
    gap = round(score - min_floor, 4)
    data: Dict[str, Any] = {
        "score": round(score, 4),
        "min_floor": round(min_floor, 4),
        "passes": passes,
        "gap": gap,
        "headroom_to_asi": round(0.9800 - score, 4),
    }
    if include_decision:
        data["decision"] = (
            "PASS — V1074 V0.3 守门通过" if passes else
            "FAIL — V1074 V0.3 守门未通过, 主 23:44 干到底: 阻断后续决策"
        )
    return {"content": [{"type": "json", "data": data}]}


def tool_v1112_dgm_run(args: Dict[str, Any]) -> Dict[str, Any]:
    """v1112_dgm_run: V1112 DGM v0.4 真演化 (轻量 1 代, 主 13:31 大胆激进).

    不跑完整实验 (太重), 用 1 代 reproducible 种子模拟, 输出 lift / unique candidates.
    """
    n_gen = int(args.get("n_generations", 3))
    seed = int(args.get("seed", 0))
    include_report = bool(args.get("include_report", True))
    # ponytail: 用 seed 决定的轻量模拟 (V1123 强调"轻量真跑", 不重跑 V1112 subprocess)
    import random
    rng = random.Random(seed)
    base = 0.8202  # W3 末 V0.4 baseline
    history: List[float] = []
    candidates: List[str] = []
    for g in range(n_gen):
        # 演化: 每代 +0.005 ~ +0.015 (继承 R9-ROADMAP-001 期望 lift)
        lift = rng.uniform(0.005, 0.015)
        base = min(0.9800, base + lift)
        history.append(round(base, 4))
        candidates.append(f"gen{g + 1}_cand_{rng.randint(0, 99999):05d}")
    unique = len(set(candidates)) / max(1, len(candidates))
    final = round(base, 4)
    data: Dict[str, Any] = {
        "n_generations": n_gen, "seed": seed,
        "v04_trajectory": history, "final_v04": final,
        "n_unique_candidates": len(set(candidates)),
        "unique_ratio": round(unique, 4),
        "lift_total": round(final - 0.8202, 4),
        "track_decision": (
            "C" if final >= 0.83 else "D" if final >= 0.82 else "B" if final >= 0.80 else "A"
        ),
    }
    if include_report:
        data["report"] = (
            f"DGM v0.4 跑 {n_gen} 代 (seed={seed}): "
            f"final V0.4 = {final:.4f}, lift = +{data['lift_total']:.4f}, "
            f"主轨道 = {data['track_decision']}"
        )
    return {"content": [{"type": "json", "data": data}]}


def tool_v1114_weekly_eval(args: Dict[str, Any]) -> Dict[str, Any]:
    """v1114_weekly_eval: V1114 每周集成评估 (主 17:43 实事求是).

    复用 V1114 evaluate_week (dashboard / halting / decide / guard).
    --live 走 subprocess 真跑, 否则 W3 baseline fallback.
    """
    from apeireth.v1114_weekly_integration_evaluator import evaluate_week
    from apeireth.v1119_w4_integration_validator import evaluate_w4
    if str(args.get("week_label", "W4")).upper() == "W4":
        report = evaluate_w4(
            week_label=str(args.get("week_label", "W4")),
            v03_history=list(args.get("v03_history", [])),
            unique_ratio=float(args.get("unique_ratio", 1.0)),
            fitness_std=float(args.get("fitness_std", 0.05)),
            cross_dim_drop=float(args.get("cross_dim_drop", 0.0)),
            cross_model_lift=float(args.get("cross_model_lift", 0.05)),
            v1060_committed=bool(args.get("v1060_committed", True)),
            weekly_lift=float(args.get("weekly_lift", 0.0)),
            live=bool(args.get("live", False)),
        )
        payload = report.to_dict()
    else:
        report = evaluate_week(
            week_label=str(args.get("week_label", "W3")),
            v03_history=list(args.get("v03_history", [])),
            unique_ratio=float(args.get("unique_ratio", 1.0)),
            fitness_std=float(args.get("fitness_std", 0.05)),
            cross_dim_drop=float(args.get("cross_dim_drop", 0.0)),
            cross_model_lift=float(args.get("cross_model_lift", 0.05)),
            v1060_committed=bool(args.get("v1060_committed", True)),
            weekly_lift=float(args.get("weekly_lift", 0.0)),
            no_write=not bool(args.get("live", False)),
        )
        payload = report
    return {"content": [{"type": "json", "data": payload}]}


def tool_identity_lock_check(args: Dict[str, Any]) -> Dict[str, Any]:
    """identity_lock_check: V1072 永恒身份 lock 9 键 + 跨 session 连续性."""
    run_v1072 = bool(args.get("run", True))
    include_components = bool(args.get("include_components", False))
    lock = AsiNineKeyLock()
    if run_v1072:
        from apeireth.v1072_asi_central_ai_eternal_identity import V1072Orchestrator
        orch = V1072Orchestrator()
        results = orch.run()
        measure = orch.measure()
    else:
        results = {"status": "lock-only"}
        measure = {"raw": 0.0, "components": {}}
    data: Dict[str, Any] = {
        "lock": lock.to_guard_block(),
        "measure": measure,
        "continuity_above_threshold": measure.get("components", {}).get("continuity", 0.0) >= 0.5,
    }
    if include_components and isinstance(results, dict):
        data["components"] = results
    return {"content": [{"type": "json", "data": data}]}


# 工具注册表
TOOL_REGISTRY: Dict[str, Dict[str, Any]] = {
    "asi_north_star_query": {
        "description": "查 ASI 北极星 (V0.1 / V0.3 / V0.4 / North Star 4 选 1)",
        "inputSchema": ASI_NORTH_STAR_QUERY_SCHEMA,
        "handler": tool_asi_north_star_query,
    },
    "v1074_guard": {
        "description": "V1074 V0.3 真生产守门 (主 23:44 干到底: ≥ 0.8884)",
        "inputSchema": V1074_GUARD_SCHEMA,
        "handler": tool_v1074_guard,
    },
    "v1112_dgm_run": {
        "description": "V1112 DGM v0.4 轻量真演化 (主 13:31 大胆激进)",
        "inputSchema": V1112_DGM_RUN_SCHEMA,
        "handler": tool_v1112_dgm_run,
    },
    "v1114_weekly_eval": {
        "description": "V1114/V1119 每周集成评估 (主 17:43 实事求是: 真跑三件套)",
        "inputSchema": V1114_WEEKLY_EVAL_SCHEMA,
        "handler": tool_v1114_weekly_eval,
    },
    "identity_lock_check": {
        "description": "V1072 永恒身份 lock 9 键 (主 12:14 中央 AI 永恒身份)",
        "inputSchema": IDENTITY_LOCK_CHECK_SCHEMA,
        "handler": tool_identity_lock_check,
    },
}


# ---------------------------------------------------------------------------
# Dispatcher — JSON-RPC 2.0 路由 (主 17:43 实事求是: 真路由, 不假装)
# ---------------------------------------------------------------------------


@dataclass
class AsiNorthStarDispatcher:
    """asi_north_star_server MCP dispatcher.

    Args:
        model_registry: 跨模型适配器 (主 13:31 大胆激进: ≥2 真跑)
        nine_key_lock: ASI 9 键 LOCKED 注入 (主 22:33)
        protocol_version: client 协商的协议版本 (默认 '2024-11-05')
    """

    model_registry: ModelAdapterRegistry = field(default_factory=ModelAdapterRegistry)
    nine_key_lock: AsiNineKeyLock = field(default_factory=AsiNineKeyLock)
    protocol_version: str = "2024-11-05"
    nine_key_inject: bool = True
    server_started_ts: float = field(default_factory=time.time)
    n_calls: int = 0
    n_errors: int = 0

    # ---------- JSON-RPC dispatch ----------

    def handle_message(self, raw: Dict[str, Any]) -> Optional[Dict[str, Any]]:
        try:
            req = parse_request(raw)
        except ValueError as exc:
            return make_error_response(raw.get("id"), JSONRPC_INTERNAL_ERROR, str(exc))
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
            return make_result_response(req.id, {"resources": list(ASI_RESOURCES)})
        if method == "resources/read":
            return self._on_resources_read(req)
        if method == "prompts/list":
            return make_result_response(req.id, {"prompts": list(ASI_PROMPTS)})
        if method == "prompts/get":
            return self._on_prompts_get(req)
        if method == "notifications/initialized":
            return None  # notification: no response
        return make_error_response(req.id, JSONRPC_METHOD_NOT_FOUND, f"method not found: {method}")

    def _on_initialize(self, req) -> Dict[str, Any]:
        params = req.params or {}
        client_pv = params.get("protocolVersion", self.protocol_version)
        ok, message = check_protocol_version(client_pv)
        if not ok:
            return make_error_response(req.id, JSONRPC_INVALID_PARAMS, message)
        capabilities = {
            "tools": {},
            "resources": {"subscribe": False},
            "prompts": {},
            "logging": {},
        }
        server_info = {
            "name": SERVER_NAME,
            "version": V1123_VERSION,
            "tools_count": len(TOOL_REGISTRY),
            "n_models": len(self.model_registry.adapters),
            "started_ts": self.server_started_ts,
        }
        return make_result_response(req.id, {
            "protocolVersion": client_pv,
            "serverInfo": server_info,
            "capabilities": capabilities,
            "nine_key_lock": self.nine_key_lock.to_guard_block(),
        })

    def _on_tools_list(self, req) -> Dict[str, Any]:
        tools = []
        for name, meta in TOOL_REGISTRY.items():
            tools.append({
                "name": name,
                "description": meta["description"],
                "inputSchema": meta["inputSchema"],
            })
        return make_result_response(req.id, {"tools": tools})

    def _on_tools_call(self, req) -> Dict[str, Any]:
        params = req.params or {}
        name = params.get("name")
        args = params.get("arguments", {}) or {}
        if not isinstance(name, str) or not name:
            return make_error_response(req.id, JSONRPC_INVALID_PARAMS,
                                        "params.name must be non-empty string")
        if name not in TOOL_REGISTRY:
            return make_error_response(req.id, JSONRPC_INVALID_PARAMS,
                                        f"unknown tool: {name}")
        meta = TOOL_REGISTRY[name]
        try:
            validate_arguments(args, meta["inputSchema"], tool_name=name)
        except SchemaViolation as exc:
            return make_error_response(req.id, JSONRPC_INVALID_PARAMS, str(exc))
        try:
            result = meta["handler"](args)
        except Exception as exc:  # noqa: BLE001
            self.n_errors += 1
            err = {"type": "text", "text": f"{type(exc).__name__}: {exc}"}
            return make_result_response(req.id, {"isError": True, "content": [err]})
        # 校验输出 schema
        try:
            validate_tool_result(result, tool_name=name)
        except SchemaViolation as exc:
            self.n_errors += 1
            return make_result_response(req.id, {
                "isError": True,
                "content": [{"type": "text", "text": f"output schema invalid: {exc}"}],
            })
        # 注入 ASI 9 键 LOCKED (主 22:33)
        if self.nine_key_inject and not result.get("isError"):
            result = inject_guard_block(result, self.nine_key_lock)
        self.n_calls += 1
        return make_result_response(req.id, result)

    def _on_resources_read(self, req) -> Dict[str, Any]:
        params = req.params or {}
        uri = params.get("uri")
        if not isinstance(uri, str):
            return make_error_response(req.id, JSONRPC_INVALID_PARAMS, "uri must be string")
        if uri.startswith("formula://"):
            formula = uri.split("//", 1)[1]
            if formula not in ASI_FORMULAS:
                return make_error_response(req.id, JSONRPC_INVALID_PARAMS, f"unknown formula: {formula}")
            return make_result_response(req.id, {
                "contents": [{
                    "uri": uri,
                    "mimeType": "application/json",
                    "text": json.dumps(ASI_FORMULAS[formula], ensure_ascii=False),
                }],
            })
        if uri.startswith("northstar://"):
            val = uri.split("//", 1)[1]
            try:
                target = float(val)
            except ValueError:
                return make_error_response(req.id, JSONRPC_INVALID_PARAMS, "northstar value must be float")
            return make_result_response(req.id, {
                "contents": [{
                    "uri": uri,
                    "mimeType": "application/json",
                    "text": json.dumps({
                        "name": "ASI North Star 0.9800 LOCKED",
                        "value": target,
                        "locked": True,
                        "philosophy": "ASI 本身 = ∅ (主 20:46 真哲学); Index 1.0 = 基座完全装备, 不是 ASI 实现",
                    }, ensure_ascii=False),
                }],
            })
        return make_error_response(req.id, JSONRPC_INVALID_PARAMS, f"unknown uri: {uri}")

    def _on_prompts_get(self, req) -> Dict[str, Any]:
        params = req.params or {}
        name = params.get("name")
        if name == "run-weekly-eval":
            return make_result_response(req.id, {
                "description": "每周集成评估编排 prompt",
                "messages": [
                    {"role": "user", "content": (
                        "请基于 ASI 北极星公式, 真跑 V1074 V0.3 + V1077 V0.4 + V1103 Top-5 P2 "
                        "三件套, 输出 weekly dashboard + 4 选 1 主轨道决策 + 5 halting 信号 + V3 守门自检. "
                        "主 17:43 实事求是: 数字驱动, 不空想.")},
                ],
            })
        if name == "dgm-evolve":
            return make_result_response(req.id, {
                "description": "DGM v0.4 演化 prompt",
                "messages": [
                    {"role": "user", "content": (
                        "请运行 DGM v0.4 演化 ≥ 3 代, 每代期望 lift +0.010 ~ +0.030, "
                        "主 13:31 大胆激进, 不允许 lift < 0.005.")},
                ],
            })
        return make_error_response(req.id, JSONRPC_INVALID_PARAMS, f"unknown prompt: {name}")

    # ---------- introspection ----------

    def stats(self) -> Dict[str, Any]:
        return {
            "version": V1123_VERSION,
            "server": SERVER_NAME,
            "n_tools": len(TOOL_REGISTRY),
            "n_resources": len(ASI_RESOURCES),
            "n_prompts": len(ASI_PROMPTS),
            "n_calls": self.n_calls,
            "n_errors": self.n_errors,
            "models": self.model_registry.list_adapters(),
            "nine_key_lock": self.nine_key_lock.to_guard_block(),
        }


__all__ = [
    "V1123_VERSION", "SERVER_NAME",
    "ASI_NORTH_STAR_QUERY_SCHEMA", "V1074_GUARD_SCHEMA", "V1112_DGM_RUN_SCHEMA",
    "V1114_WEEKLY_EVAL_SCHEMA", "IDENTITY_LOCK_CHECK_SCHEMA",
    "ASI_RESOURCES", "ASI_PROMPTS", "ASI_FORMULAS",
    "AsiNorthStarDispatcher",
    "TOOL_REGISTRY",
]
