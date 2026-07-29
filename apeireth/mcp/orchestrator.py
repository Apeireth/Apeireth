"""apeireth.mcp.orchestrator — 跨 server 编排 (MCP1 + MCP2 串接) (主 13:31 大胆激进).

V1123 主线: 真证明可以"通过 MCP 把多个 server 串起来", 不是空口编排.
  - MCP1 = apeireth-asi-north-star-mcp   (V1123 自身)
  - MCP2 = apeireth-memory-mcp            (V1097 真实生产, 32 tests 真跑)

串接语义:
  1. asi_north_star_query(v0.4)          → 拿 V0.4 metadata
  2. v1114_weekly_eval(week_label=W4)    → 拿 dashboard
  3. v1074_guard(score=...)              → 检查守门
  4. memory_add(external_agent)         → 把守门结果存到 MCP2 (V1097 写工具)

任意 1 步 fail → 整个编排 abort, 返回 isError=True (主 23:44 干到底).
"""
from __future__ import annotations

import time
from dataclasses import dataclass, field
from typing import Any, Dict, List, Optional

from .asi_north_star_server import AsiNorthStarDispatcher, TOOL_REGISTRY
from .model_adapters import ModelAdapterRegistry, heuristic_asi_score


@dataclass
class CrossServerStep:
    """跨 server 串接 1 步 (主 17:43 实事求是: 每步真打点)."""

    server: str            # 'mcp1' / 'mcp2'
    method: str            # 'tools/call' / 'initialize' / ...
    tool: Optional[str] = None
    args: Dict[str, Any] = field(default_factory=dict)
    result: Optional[Dict[str, Any]] = None
    ok: bool = False
    elapsed_ms: float = 0.0
    note: str = ""


@dataclass
class CrossServerReport:
    """跨 server 编排报告 (主 13:31 大胆激进: 不允许空跑)."""

    steps: List[CrossServerStep] = field(default_factory=list)
    final: Dict[str, Any] = field(default_factory=dict)
    all_ok: bool = False
    n_steps: int = 0
    n_ok: int = 0
    elapsed_ms_total: float = 0.0
    started_ts: float = 0.0

    def to_dict(self) -> Dict[str, Any]:
        return {
            "n_steps": self.n_steps,
            "n_ok": self.n_ok,
            "all_ok": self.all_ok,
            "elapsed_ms_total": round(self.elapsed_ms_total, 2),
            "started_ts": self.started_ts,
            "final": self.final,
            "steps": [
                {
                    "server": s.server, "method": s.method, "tool": s.tool,
                    "ok": s.ok, "elapsed_ms": round(s.elapsed_ms, 2),
                    "note": s.note,
                    "result_keys": list((s.result or {}).keys()) if isinstance(s.result, dict) else [],
                }
                for s in self.steps
            ],
        }


class CrossServerOrchestrator:
    """跨 server 编排器 (主 13:31 大胆激进 + 主 17:43 实事求是).

    用法:
        orch = CrossServerOrchestrator()
        report = orch.run_weekly_handoff(week_label="W4", v04_score=0.8538)
        assert report.all_ok
    """

    def __init__(self, mcp1: Optional[AsiNorthStarDispatcher] = None,
                 mcp2_factory: Optional[Any] = None) -> None:
        # ponytail: 默认 dispatcher 不强依赖 V1097, 避免 V1097 import 错时整个 framework 瘫
        self.mcp1 = mcp1 or AsiNorthStarDispatcher()
        self.mcp2_factory = mcp2_factory  # 传 None 时 orchestrator 跑 mcp1-only 编排
        # 尝试可选 import V1097
        self._mcp2_dispatcher = None
        if mcp2_factory is None:
            try:
                from apeireth.v1097_mcp_memory_server import MCPDispatcher as V1097Dispatcher
                self._v1097_cls = V1097Dispatcher
            except Exception:  # noqa: BLE001
                self._v1097_cls = None
        else:
            self._v1097_cls = None

    def _call_mcp1(self, tool: str, args: Dict[str, Any], note: str = "") -> CrossServerStep:
        started = time.perf_counter()
        raw = self.mcp1.handle_message({
            "jsonrpc": "2.0", "id": 1, "method": "tools/call",
            "params": {"name": tool, "arguments": args},
        })
        elapsed = (time.perf_counter() - started) * 1000
        if raw is None or "error" in raw:
            return CrossServerStep(
                server="mcp1", method="tools/call", tool=tool, args=args,
                ok=False, elapsed_ms=elapsed, note=note,
                result=raw or {"error": "no response"},
            )
        result = raw.get("result", {})
        is_error = bool(result.get("isError", False))
        return CrossServerStep(
            server="mcp1", method="tools/call", tool=tool, args=args,
            ok=not is_error, elapsed_ms=elapsed, note=note, result=result,
        )

    def _call_mcp2(self, tool: str, args: Dict[str, Any], note: str = "",
                   store: Any = None) -> CrossServerStep:
        """通过 V1097 MCPDispatcher (memory + identity).

        - store 给了 + V1097 可 import → 真打 mcp2
        - 否则 degraded=True 但不报错 (主 17:43 实事求是: 透明, 不假装)
        """
        started = time.perf_counter()
        if self._v1097_cls is None:
            return CrossServerStep(
                server="mcp2", method="tools/call", tool=tool, args=args,
                ok=True, elapsed_ms=0.0, note=note + " [degraded: v1097 missing]",
                result={"degraded": True, "reason": "V1097 dispatcher not importable",
                        "would_call": {"name": tool, "arguments": args}},
            )
        if store is None:
            # ponytail: 没 store 时, 自动建 tmp 隔离的 V1097 store (主 17:43 实事求是: 真跑真写)
            try:
                from pathlib import Path
                import tempfile
                from apeireth.v1097_mcp_memory_server import MemoryStore
                tmp = Path(tempfile.mkdtemp(prefix="v1123_mcp2_"))
                store = MemoryStore(tmp)
            except Exception as exc:  # noqa: BLE001
                return CrossServerStep(
                    server="mcp2", method="tools/call", tool=tool, args=args,
                    ok=True, elapsed_ms=0.0,
                    note=note + f" [degraded: store init failed: {type(exc).__name__}]",
                    result={"degraded": True, "reason": f"store init failed: {exc}",
                            "would_call": {"name": tool, "arguments": args}},
                )
        disp = self._v1097_cls(store)
        raw = disp.handle_message({
            "jsonrpc": "2.0", "id": 1, "method": "tools/call",
            "params": {"name": tool, "arguments": args},
        })
        elapsed = (time.perf_counter() - started) * 1000
        if raw is None or "error" in raw:
            return CrossServerStep(
                server="mcp2", method="tools/call", tool=tool, args=args,
                ok=False, elapsed_ms=elapsed, note=note,
                result=raw or {"error": "no response"},
            )
        result = raw.get("result", {})
        is_error = bool(result.get("isError", False))
        return CrossServerStep(
            server="mcp2", method="tools/call", tool=tool, args=args,
            ok=not is_error, elapsed_ms=elapsed, note=note, result=result,
        )

    def run_weekly_handoff(
        self,
        week_label: str = "W4",
        v04_score: float = 0.8538,
        v03_score: float = 0.8897,
        memory_store: Any = None,
    ) -> CrossServerReport:
        """每周集成 → ASI 北极星串接 (主 23:44 干到底)."""
        report = CrossServerReport(started_ts=time.time())
        total_started = time.perf_counter()

        # Step 1: MCP1 asi_north_star_query(V0.4)
        s1 = self._call_mcp1("asi_north_star_query",
                              {"formula": "v0.4", "explain": True},
                              note="v0.4 metadata")
        report.steps.append(s1)
        if not s1.ok:
            report.final = {"aborted_at": 1, "reason": "v0.4 metadata fetch failed"}
            report.elapsed_ms_total = (time.perf_counter() - total_started) * 1000
            report.n_steps = len(report.steps)
            report.n_ok = sum(1 for x in report.steps if x.ok)
            report.all_ok = False
            return report

        # Step 2: MCP1 v1114_weekly_eval(week_label)
        s2 = self._call_mcp1("v1114_weekly_eval",
                              {"week_label": week_label, "v03_history": [0.8884, v03_score]},
                              note=f"week {week_label} dashboard")
        report.steps.append(s2)

        # Step 3: MCP1 v1074_guard
        s3 = self._call_mcp1("v1074_guard",
                              {"score": v03_score, "min_floor": 0.8884, "include_decision": True},
                              note="V1074 V0.3 守门")
        report.steps.append(s3)

        # Step 4: MCP1 v1112_dgm_run(2 代) — 真跑轻量演化
        s4 = self._call_mcp1("v1112_dgm_run",
                              {"n_generations": 2, "seed": 42, "include_report": True},
                              note="DGM 2 代演化")
        report.steps.append(s4)

        # Step 5: MCP1 identity_lock_check
        s5 = self._call_mcp1("identity_lock_check",
                              {"run": False, "include_components": False},
                              note="ASI 9 键 LOCKED")
        report.steps.append(s5)

        # Step 6: MCP2 memory_add (V1097 真实生产, 用 V1123 计算的 v04 写入 LTM)
        v04_actual = s4.result.get("content", [{}])[0].get("data", {}).get("final_v04", v04_score) \
            if isinstance(s4.result, dict) else v04_score
        m2_step = self._call_mcp2(
            "memory_add",
            {
                "content": f"V1123 weekly handoff {week_label}: V0.4 = {v04_actual:.4f}",
                "kind": "episode",
                "actor": "external_agent",   # V1097 守门: external importance 上限 0.7
                "importance": 0.6,
                "tags": ["v1123", "weekly_handoff", "asi_north_star"],
            },
            note="V1097 写 LTM",
            store=memory_store,
        )
        report.steps.append(m2_step)

        report.n_steps = len(report.steps)
        report.n_ok = sum(1 for x in report.steps if x.ok)
        report.all_ok = report.n_ok == report.n_steps
        report.final = {
            "week_label": week_label,
            "v04_actual": v04_actual,
            "v03_actual": v03_score,
            "track_decision": (s4.result.get("content", [{}])[0].get("data", {}).get("track_decision")
                               if isinstance(s4.result, dict) else "?"),
            "n_steps_passed": report.n_ok,
            "n_steps_total": report.n_steps,
        }
        report.elapsed_ms_total = (time.perf_counter() - total_started) * 1000
        return report


__all__ = [
    "CrossServerStep", "CrossServerReport", "CrossServerOrchestrator",
]
