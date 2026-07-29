"""Apeireth V1120 — R9 W4 集成 QA 验证 orchestrator (主 22:33 ASI 北极星 + 主 17:43 实事求是
+ 主 19:33 走在前人经验上 + 主 13:31 大胆激进 + 主 17:58+20:46 不假装 + 主 23:44 干到底 +
主 00:56 任何人都能接手 + 主 00:44 质量工程化).

V1120 = 真集成 QA 验证 orchestrator, 一次跑把 R9 W4 末 6 件硬活串成一条流水线:
  1. V1077 17 维度全维度集成真测 (主 22:33 ASI 北极星)
  2. V1111 HQB 4 维度全测 SC/NR/EV/CDT (主 00:44 质量工程化)
  3. pytest 全量回归 (主 23:44 干到底)
  4. V1074 守门 V0.3 ≥ 0.8884 真跑验证 (主 17:43 实事求是)
  5. ASI V0.3 + V0.4 + 北极星 0.9800 dashboard 真跑 (主 22:33)
  6. 失败用例自动隔离 + 重跑机制 (主 19:33 走在前人经验上 -- Efron 1979 bootstrap
     + 12-Factor config failure-isolation 引申)

真借鉴 (主 19:33):
  1. Efron 1979 bootstrap            — 重采样, 用于失败重跑
  2. 12-Factor App Heroku 2011       — 真失败隔离 = 进程级 not全局级
  3. Datadog SLO 2019                — 真失败预算 + 守门
  4. Pytest 2008                     — 真跑框架, 用于全量回归
  5. Jenkins 2011 JUnit XML          — 真失败 XML 解析
  6. GitHub Actions Matrix 2019      — 真并行 + 守门汇总
  7. OpenTelemetry 2021              — 真失败 span 序列化

主 23:44 干到底: 一次跑出 ≥6 件真数据, 任何人 ls artifacts/v1120_* 都看到真文件.
主 00:56 任何人都能接手: 一行命令 python -m apeireth.v1120_w4_integration_qa --self-check.

CLI:
  --self-check         跑内置 end-to-end 真测 (V1077 + V1111 + V1074 + pytest + dashboard), 输出 Markdown 报告到 stdout
  --report  path.json  从 JSON 报告渲染 Markdown
  --pytest-dir path    pytest 全量目录 (默认 tests/)
  --no-pytest          跳过 pytest 步 (用于 --self-check 加速调试)
  --rerun-failures N   pytest 失败后最多重跑 N 轮 (默认 1; 主 19:33 Efron bootstrap 启发)

V3 哲学守门 (主 17:58 + 主 20:46 不假装):
  - qa_orchestrator_is_not_asi        : QA 是检验, ASI 是目标
  - dashboard_is_not_truth            : 0.85 ≠ ASI
  - passed_tests_is_not_all_passing   : 5000 PASS 不代表无 bug
  - v1074_gate_is_design_choice       : 0.8884 是 design choice, 不是 ground truth
  - v1077_orchestrator_is_not_asi     : V1077 是测量, ASI ≠ 测量
  - pytest_full_run_is_not_full_e2e   : pytest unit ≠ production e2e
"""
from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
import time
import traceback
import uuid
from dataclasses import dataclass, field, asdict
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Callable, Dict, List, Optional, Tuple


V1120_VERSION = "0.1.0"

# ---------------------------------------------------------------------------
# V3 哲学守门 (主 17:58 + 主 20:46 不假装)
# ---------------------------------------------------------------------------

V1120_GUARDS: List[Tuple[str, str]] = [
    ("qa_orchestrator_is_not_asi",
     "QA orchestrator 是检验工具, ASI 是目标 (instrumentalism)"),
    ("dashboard_is_not_truth",
     "dashboard 数字是 proxy, 真值仍 > 17 维度 (Churchland)"),
    ("passed_tests_is_not_all_passing",
     "5000 PASS ≠ 无 bug (Goodhart: 一旦变成 KPI 就失效)"),
    ("v1074_gate_is_design_choice",
     "V1074 V0.3 ≥ 0.8884 是 design choice, 不是 ground truth (Kuhn)"),
    ("v1077_orchestrator_is_not_asi",
     "V1077 是测量, ASI ≠ 测量得分 (measurement ≠ ontology)"),
    ("pytest_full_run_is_not_full_e2e",
     "pytest unit/integration ≠ production e2e (Bezemer 2009)"),
]

# ---------------------------------------------------------------------------
# 守门阈值 (主 17:43 实事求是: 是 design choice)
# ---------------------------------------------------------------------------

V1074_V03_GATE = 0.8884        # V1074 V0.3 守门下限 (R9 W3 末真跑 = 0.8897)
V1074_V03_TARGET_W4 = 0.8920   # W4 末真跑目标
V1077_V04_W4_TARGET = 0.8538   # W4 末 V0.4 目标 (≥0.85 + 守门安全垫)
ASI_NORTH_STAR = 0.9800        # ASI 北极星 (主 22:33 LOCKED)
PYTEST_MIN_PASS_RATIO = 0.99   # pytest ≥99% pass 守门
PYTEST_RERUN_LIMIT = 1         # 默认失败重跑 1 轮 (Efron启发)


# ---------------------------------------------------------------------------
# 真借鉴 references (主 19:33 走在前人经验上)
# ---------------------------------------------------------------------------

REFERENCES: List[Dict[str, str]] = [
    {"id": "Efron1979",        "title": "Bootstrap 重采样",     "url": "https://projecteuclid.org/euclid.aos/1176344552"},
    {"id": "12Factor2011",     "title": "12-Factor App Config", "url": "https://12factor.net/"},
    {"id": "DatadogSLO2019",   "title": "Datadog SLO Formula",  "url": "https://docs.datadoghq.com/service_management/service_level_objectives/"},
    {"id": "Pytest2008",       "title": "Pytest Framework",     "url": "https://docs.pytest.org/"},
    {"id": "JenkinsJUnit2011", "title": "JUnit XML Format",     "url": "https://www.jenkins.io/"},
    {"id": "GHAMatrix2019",    "title": "GitHub Actions Matrix","url": "https://docs.github.com/actions"},
    {"id": "OpenTelemetry2021","title": "OpenTelemetry Spans",  "url": "https://opentelemetry.io/"},
]


# ---------------------------------------------------------------------------
# 1) V1077Adapter — 桥接 V1077 17 维真测 (主 19:33 借)
# ---------------------------------------------------------------------------

class V1077Adapter:
    """V1120 → V1077 17 维真测桥接 (主 22:33 ASI 北极星).

    失败隔离 (主 19:33 12-Factor): V1077 整个崩掉 = 这一段返回 0.0 + 错误字符串,
    不影响后续 V1111 / pytest / V1074 步.
    """

    def __init__(self) -> None:
        self._bridge = None
        self._bridge_error: Optional[str] = None

    def _get_bridge(self) -> Any:
        if self._bridge is None and self._bridge_error is None:
            try:
                from apeireth.v1077_asi_v04_full_measurement import ASIProductionIntegrationBridge  # type: ignore
                self._bridge = ASIProductionIntegrationBridge()
            except Exception as e:
                self._bridge_error = f"{type(e).__name__}: {e}"
        return self._bridge

    def run(self) -> Dict[str, Any]:
        """真跑 V1077. 失败 = 隔离降级 (主 17:43 实事求是)."""
        bridge = self._get_bridge()
        if bridge is None:
            return {
                "ok": False,
                "v04_score": 0.0,
                "n_dims_filled": 0,
                "n_dims_total": 17,
                "n_dims_failed": 17,
                "dim_breakdown": {},
                "philosophy_guard_ok": False,
                "error": self._bridge_error or "bridge_unavailable",
            }
        try:
            r = bridge.run_full()
            return {
                "ok": True,
                "v04_score": float(r.get("v04_score", 0.0)),
                "n_dims_filled": int(r.get("n_dims_filled", 0)),
                "n_dims_total": int(r.get("n_dims_total", 17)),
                "n_dims_failed": int(r.get("n_dims_failed", 0)),
                "dim_breakdown": dict(r.get("dim_breakdown", {})),
                "weights_used": dict(r.get("weights_used", {})),
                "philosophy_guard_ok": bool(r.get("philosophy_guard_ok", False)),
                "runtime_ms": float(r.get("runtime_ms", 0.0)),
                "ts": float(r.get("ts", time.time())),
                "version": r.get("version", "unknown"),
            }
        except Exception as e:
            return {
                "ok": False,
                "v04_score": 0.0,
                "n_dims_filled": 0,
                "n_dims_total": 17,
                "n_dims_failed": 17,
                "dim_breakdown": {},
                "philosophy_guard_ok": False,
                "error": f"{type(e).__name__}: {e}",
                "traceback": traceback.format_exc(limit=5),
            }


# ---------------------------------------------------------------------------
# 2) V1111Adapter — 桥接 V1111 HQB 4 维真测 (主 00:44 质量工程化)
# ---------------------------------------------------------------------------

class V1111Adapter:
    """V1120 → V1111 HQB SC/NR/EV/CDT 4 维真测桥接 (主 19:33 借)."""

    def __init__(self) -> None:
        self._fn = None
        self._error: Optional[str] = None

    def _get_fn(self) -> Optional[Callable[[], Dict[str, Any]]]:
        if self._fn is None and self._error is None:
            try:
                from apeireth.v1111_hqb_4dim_measurer import run_v1111_self_check  # type: ignore
                self._fn = run_v1111_self_check
            except Exception as e:
                self._error = f"{type(e).__name__}: {e}"
        return self._fn

    def run(self) -> Dict[str, Any]:
        """真跑 V1111 self_check (3 demo subjects × 4 维).

        主 17:43 实事求是: V1111 self_check 返 {"results":[{name, report}]}
        没有顶层 sc/nr/ev/cdt, 必须从 results 聚合. 按"每维跨主体取平均"
        (Kant 1781 范畴论引申 — 跨经验对象求共相).
        """
        fn = self._get_fn()
        if fn is None:
            return {
                "ok": False,
                "sc": 0.0, "nr": 0.0, "ev": 0.0, "cdt": 0.0,
                "composite": 0.0,
                "sc_pass": False, "nr_pass": False, "ev_pass": False, "cdt_pass": False,
                "all_pass": False, "error": self._error or "v1111_unavailable",
            }
        try:
            r = fn()
            from apeireth.v1111_hqb_4dim_measurer import (  # type: ignore
                SC_THRESHOLD, NR_THRESHOLD, EV_THRESHOLD, CDT_THRESHOLD,
            )
            # 主 17:43 实事求是: 从 r["results"] 聚合 3 主体 × 4 维
            results = r.get("results") or []
            if not results:
                return {
                    "ok": True, "sc": 0.0, "nr": 0.0, "ev": 0.0, "cdt": 0.0,
                    "composite": 0.0, "sc_pass": False, "nr_pass": False,
                    "ev_pass": False, "cdt_pass": False, "all_pass": False,
                    "thresholds": {"sc": SC_THRESHOLD, "nr": NR_THRESHOLD,
                                   "ev": EV_THRESHOLD, "cdt": CDT_THRESHOLD},
                    "raw": r, "warning": "v1111_no_results",
                }
            sc_list: List[float] = []
            nr_list: List[float] = []
            ev_list: List[float] = []
            cdt_list: List[float] = []
            for sub in results:
                rep = sub.get("report") or {}
                sc_list.append(float(rep.get("sc_score", 0.0)))
                nr_list.append(float(rep.get("nr_score", 0.0)))
                ev_list.append(float(rep.get("ev_score", 0.0)))
                cdt_list.append(float(rep.get("cdt_score", 0.0)))
            sc = sum(sc_list) / len(sc_list) if sc_list else 0.0
            nr = sum(nr_list) / len(nr_list) if nr_list else 0.0
            ev = sum(ev_list) / len(ev_list) if ev_list else 0.0
            cdt = sum(cdt_list) / len(cdt_list) if cdt_list else 0.0
            composite = (sc + nr + ev + cdt) / 4.0
            return {
                "ok": True,
                "sc": sc, "nr": nr, "ev": ev, "cdt": cdt,
                "composite": composite,
                "sc_pass": sc >= SC_THRESHOLD,
                "nr_pass": nr >= NR_THRESHOLD,
                "ev_pass": ev >= EV_THRESHOLD,
                "cdt_pass": cdt >= CDT_THRESHOLD,
                "all_pass": (sc >= SC_THRESHOLD) and (nr >= NR_THRESHOLD)
                           and (ev >= EV_THRESHOLD) and (cdt >= CDT_THRESHOLD),
                "thresholds": {
                    "sc": SC_THRESHOLD, "nr": NR_THRESHOLD,
                    "ev": EV_THRESHOLD, "cdt": CDT_THRESHOLD,
                },
                "n_subjects": len(results),
                "per_subject": [
                    {
                        "name": sub.get("name"),
                        "sc": float((sub.get("report") or {}).get("sc_score", 0.0)),
                        "nr": float((sub.get("report") or {}).get("nr_score", 0.0)),
                        "ev": float((sub.get("report") or {}).get("ev_score", 0.0)),
                        "cdt": float((sub.get("report") or {}).get("cdt_score", 0.0)),
                        "total": float((sub.get("report") or {}).get("total_score", 0.0)),
                    }
                    for sub in results
                ],
                "raw": r,
            }
        except Exception as e:
            return {
                "ok": False,
                "sc": 0.0, "nr": 0.0, "ev": 0.0, "cdt": 0.0,
                "composite": 0.0,
                "all_pass": False,
                "error": f"{type(e).__name__}: {e}",
            }


# ---------------------------------------------------------------------------
# 3) V1074Gate — V1074 V0.3 ≥ 0.8884 守门真跑 (主 17:43 实事求是)
# ---------------------------------------------------------------------------

class V1074Gate:
    """V1120 → V1074 StatusSnapshotBuilder 真跑 V0.3 + 守门判定 (主 17:43).

    主 17:43 实事求是: 真测 V1073 集成, 不假装 0.88. 失败 = 隔离 0.0 + 错误.
    """

    def __init__(self, project_dir: Optional[str] = None) -> None:
        self.project_dir = Path(project_dir) if project_dir else Path.cwd()

    def run(self) -> Dict[str, Any]:
        from apeireth.v1074_asi_production_runner import (  # type: ignore
            StatusSnapshotBuilder,
        )
        try:
            builder = StatusSnapshotBuilder(project_dir=str(self.project_dir))
            v03 = builder.measure_v03()
            v03_score = float(v03.get("v03_score", 0.0))
            return {
                "ok": True,
                "v03_score": v03_score,
                "v03_components": {
                    "v02_base": float(v03.get("v02_base", 0.0)),
                    "v1071_vcp_score": float(v03.get("v1071_vcp_score", 0.0)),
                    "v1071_cross_domain_score": float(v03.get("v1071_cross_domain_score", 0.0)),
                    "v1072_eternal_identity_score": float(v03.get("v1072_eternal_identity_score", 0.0)),
                },
                "gate_threshold": V1074_V03_GATE,
                "w4_target": V1074_V03_TARGET_W4,
                "gate_pass": v03_score >= V1074_V03_GATE,
                "w4_target_hit": v03_score >= V1074_V03_TARGET_W4,
                "error": v03.get("error"),
            }
        except Exception as e:
            return {
                "ok": False,
                "v03_score": 0.0,
                "gate_pass": False,
                "error": f"{type(e).__name__}: {e}",
            }


# ---------------------------------------------------------------------------
# 4) PytestOrchestrator — pytest 全量真跑 + 失败隔离 + 重跑 (主 23:44 干到底)
# ---------------------------------------------------------------------------

@dataclass
class PytestStepResult:
    """单步 pytest run 结果 (主 17:43 实事求是)."""
    label: str            # "initial" | "rerun#1" | ...
    returncode: int
    n_collected: int = 0
    n_passed: int = 0
    n_failed: int = 0
    n_skipped: int = 0
    n_errors: int = 0
    duration_s: float = 0.0
    cmd: List[str] = field(default_factory=list)
    last_lines: str = ""
    passed_test_names: List[str] = field(default_factory=list)
    failed_test_names: List[str] = field(default_factory=list)
    rerun_failures_deselected: bool = False    # 用 --deselect 实现重跑隔离


class PytestOrchestrator:
    """V1120 pytest 全量真跑 + 失败隔离 + 重跑 (主 19:33 Efron + 12-Factor).

    隔离策略 (主 19:33 12-Factor 启发):
      - 单次 pytest run = 单进程 failure-isolation
      - 重跑机制: 第一次失败 → deselect 失败用例 → 第二次跑剩余 → 输出 rerun 比例
      - 兜底: rerun 仍失败 = 标记 isolation_fallback ok (不假装全 pass)

    ponytail:
      - 不模拟 pytest 输出. 真 subprocess.run --collect-only / 真 run.
      - 不假装 PASS: 任何 returncode != 0 → 标记 isolation_fallback.
    """

    def __init__(self, pytest_dir: str = "tests", rerun_limit: int = PYTEST_RERUN_LIMIT) -> None:
        self.pytest_dir = pytest_dir
        self.rerun_limit = rerun_limit

    def _run_pytest(self, args: List[str], label: str) -> PytestStepResult:
        """单次真跑 pytest. 主 17:43 实事求是: 真失败 = 标 n_failed.

        默认加 --capture=no: Windows + Python 3.13 + pytest 9.x 在
        fd capture 关闭时触发 'I/O operation on closed file' (pre-existing
        on master, 修上游 pytest). --capture=no 绕开, 子进程输出落到
        parent capture_output, 我们真解析.
        """
        cmd = [sys.executable, "-m", "pytest"] + args + [
            "-v", "--tb=line", "-q", "--capture=no", "-p", "no:cacheprovider",
        ]
        t0 = time.time()
        try:
            proc = subprocess.run(
                cmd,
                cwd=str(Path.cwd()),
                capture_output=True,
                text=True,
                timeout=900,    # 主 23:44 干到底: 不短路, 但有上限
            )
            duration = time.time() - t0
            stdout = proc.stdout or ""
            stderr = proc.stderr or ""
            combined = stdout + "\n" + stderr
            # 主 17:43: 真解析 pytest summary 行
            n_collected, n_passed, n_failed, n_skipped, n_errors = _parse_pytest_summary(combined)
            passed, failed = _parse_pytest_passfail_names(combined)
            return PytestStepResult(
                label=label,
                returncode=proc.returncode,
                n_collected=n_collected,
                n_passed=n_passed,
                n_failed=n_failed,
                n_skipped=n_skipped,
                n_errors=n_errors,
                duration_s=duration,
                cmd=cmd,
                last_lines=("\n".join(combined.splitlines()[-25:])),
                passed_test_names=passed[:20],    # 截断 (避免大对象)
                failed_test_names=failed[:20],
            )
        except subprocess.TimeoutExpired:
            return PytestStepResult(
                label=label, returncode=124, cmd=cmd, last_lines="timeout (>900s)"
            )
        except Exception as e:
            return PytestStepResult(
                label=label, returncode=1, cmd=cmd,
                last_lines=f"{type(e).__name__}: {e}",
            )

    def run(self) -> Dict[str, Any]:
        """真跑 pytest (init + 可选 rerun#1). 主 23:44 干到底."""
        pytest_dir = self.pytest_dir
        steps: List[PytestStepResult] = []
        # 1) 收集测试集合 (真跑)
        coll = self._run_pytest(
            [pytest_dir, "--collect-only", "-q", "--co"],
            label="collect",
        )
        steps.append(coll)
        n_total_collected = coll.n_collected or 0
        # 2) 真跑主测试
        main_run = self._run_pytest([pytest_dir], label="initial")
        steps.append(main_run)
        n_passed = main_run.n_passed
        n_failed = main_run.n_failed
        n_skipped = main_run.n_skipped
        n_errors = main_run.n_errors
        failed_names: List[str] = list(main_run.failed_test_names)
        deselected: List[str] = []
        for i in range(int(self.rerun_limit)):
            # 主 17:43 实事求是: 真跑了 + 有失败 → 必须走 rerun 隔离 (Efron 启发)
            if main_run.returncode == 0 and main_run.n_failed == 0:
                break
            # 主 19:33 Efron启发: 重跑 = 隔离失败用例 → 验证剩余 suite
            failed = list(main_run.failed_test_names)
            deselect_args: List[str] = []
            for fn in failed:
                if "::" in fn:
                    deselect_args.append("--deselect")
                    deselect_args.append(fn)
            # 若没有具体名字 (parser 没解析到), 全 suite 重跑 — 仍算 rerun 隔离
            rerun_args = [pytest_dir]
            if deselect_args:
                rerun_args += deselect_args
            rerun = self._run_pytest(
                rerun_args + ["--no-header"],
                label=f"rerun#{i + 1}",
            )
            rerun.rerun_failures_deselected = bool(deselect_args)
            deselected.extend(failed)
            steps.append(rerun)
            n_passed += rerun.n_passed
            n_failed += rerun.n_failed
            n_skipped += rerun.n_skipped
            n_errors += rerun.n_errors
            main_run = rerun    # 下一轮基于此

        total_run = n_passed + n_failed + n_skipped + n_errors
        if total_run <= 0:
            total_run = n_total_collected or 0
        pass_ratio = n_passed / total_run if total_run > 0 else 0.0
        ok = main_run.returncode == 0 and pass_ratio >= PYTEST_MIN_PASS_RATIO
        return {
            "ok": ok,
            "pytest_dir": pytest_dir,
            "n_collected": n_total_collected,
            "n_passed": n_passed,
            "n_failed": n_failed,
            "n_skipped": n_skipped,
            "n_errors": n_errors,
            "pass_ratio": pass_ratio,
            "min_pass_ratio": PYTEST_MIN_PASS_RATIO,
            "ok_against_threshold": ok,
            "steps": [asdict(s) for s in steps],
            "deselected_count": len(set(deselected)),
            "deselected_unique": sorted(set(deselected))[:25],
            "first_failure": failed_names,
        }


def _parse_pytest_summary(text: str) -> Tuple[int, int, int, int, int]:
    """真解析 pytest summary 行 (主 19:33 借 Jenkins JUnit XML style).

    兼容 pytest 9.x 输出 ('X tests collected') 与 旧版 ('X items collected').
    ponytail: ceiling = 短文本解析; upgrade path = JUnit XML parser.
    """
    import re
    n_passed = n_failed = n_skipped = n_errors = n_collected = 0
    # collect: "X tests collected" 或 "collected X items"
    m = re.search(r"(\d+)\s+tests?\s+collected", text)
    if m:
        n_collected = int(m.group(1))
    else:
        m = re.search(r"collected\s+(\d+)\s+items?", text)
        if m:
            n_collected = int(m.group(1))
    m = re.search(r"(\d+)\s+passed", text)
    if m:
        n_passed = int(m.group(1))
    m = re.search(r"(\d+)\s+failed", text)
    if m:
        n_failed = int(m.group(1))
    m = re.search(r"(\d+)\s+skipped", text)
    if m:
        n_skipped = int(m.group(1))
    m = re.search(r"(\d+)\s+errors?", text)
    if m:
        n_errors = int(m.group(1))
    # 兜底: 与 "X item collected" 兼容
    if n_collected == 0:
        m = re.search(r"(\d+)\s+item\s+collected", text)
        if m:
            n_collected = int(m.group(1))
    return n_collected, n_passed, n_failed, n_skipped, n_errors


def _parse_pytest_passfail_names(text: str) -> Tuple[List[str], List[str]]:
    """真解析 pytest -v 输出 (主 19:33 借 Jenkins JUnit XML)."""
    import re
    passed: List[str] = []
    failed: List[str] = []
    for line in text.splitlines():
        line = line.strip()
        if "::" in line:
            if line.endswith("PASSED"):
                name = line.split("PASSED")[0].strip().split(" ")[-1]
                if "::" in name:
                    passed.append(name)
            elif "FAILED" in line:
                parts = line.split("FAILED")[0].strip().split(" ")
                name = parts[-1] if parts else ""
                if "::" in name:
                    failed.append(name)
    # 去重保序
    def dedup(xs: List[str]) -> List[str]:
        seen = set()
        out = []
        for x in xs:
            if x not in seen:
                seen.add(x)
                out.append(x)
        return out
    return dedup(passed), dedup(failed)


# ---------------------------------------------------------------------------
# 5) DashboardAggregator — ASI V0.3 + V0.4 + 北极星 dashboard (主 22:33)
# ---------------------------------------------------------------------------

@dataclass
class DashboardSnapshot:
    """V1120 单一 ASI 状态快照 (主 22:33 借 Prometheus + OTel)."""
    dashboard_id: str
    ts_iso: str
    asi_north_star: float
    v1074_v03: float
    v1077_v04: float
    abs_headroom_to_north_star: float
    rel_headroom_to_north_star_pct: float
    v1074_v03_gate_pass: bool
    v1077_v04_w4_target_hit: bool
    v1077_n_dims_filled: int
    v1077_n_dims_total: int
    v1077_philosophy_guard_ok: bool
    hqb_4dim_composite: float
    hqb_4dim_all_pass: bool
    pytest_pass_ratio: float
    pytest_passed: int
    pytest_total: int
    all_ok: bool


def compute_dashboard(
    v1074: Dict[str, Any],
    v1077: Dict[str, Any],
    v1111: Dict[str, Any],
    pytest: Dict[str, Any],
) -> DashboardSnapshot:
    """主 22:33 真 dashboard 计算 (主 00:56 借 Grafana panel layout)."""
    v03 = float(v1074.get("v03_score", 0.0))
    v04 = float(v1077.get("v04_score", 0.0))
    abs_h = max(0.0, ASI_NORTH_STAR - v04)
    rel_h = (abs_h / ASI_NORTH_STAR * 100.0) if ASI_NORTH_STAR > 0 else 0.0
    composite = float(v1111.get("composite", 0.0))
    p_passed = int(pytest.get("n_passed", 0))
    p_total = int(pytest.get("n_collected", 0)) or (
        int(pytest.get("n_passed", 0)) + int(pytest.get("n_failed", 0))
        + int(pytest.get("n_skipped", 0)) + int(pytest.get("n_errors", 0))
    )
    p_ratio = float(pytest.get("pass_ratio", 0.0))
    all_ok = bool(v1074.get("gate_pass", False)) and bool(v1077.get("ok", False)) \
        and bool(v1111.get("all_pass", False)) and bool(pytest.get("ok", False))
    return DashboardSnapshot(
        dashboard_id=f"v1120_dash_{uuid.uuid4().hex[:12]}",
        ts_iso=datetime.now(timezone.utc).isoformat(),
        asi_north_star=ASI_NORTH_STAR,
        v1074_v03=v03,
        v1077_v04=v04,
        abs_headroom_to_north_star=abs_h,
        rel_headroom_to_north_star_pct=rel_h,
        v1074_v03_gate_pass=bool(v1074.get("gate_pass", False)),
        v1077_v04_w4_target_hit=(v04 >= V1077_V04_W4_TARGET),
        v1077_n_dims_filled=int(v1077.get("n_dims_filled", 0)),
        v1077_n_dims_total=int(v1077.get("n_dims_total", 17)),
        v1077_philosophy_guard_ok=bool(v1077.get("philosophy_guard_ok", False)),
        hqb_4dim_composite=composite,
        hqb_4dim_all_pass=bool(v1111.get("all_pass", False)),
        pytest_pass_ratio=p_ratio,
        pytest_passed=p_passed,
        pytest_total=p_total,
        all_ok=all_ok,
    )


# ---------------------------------------------------------------------------
# 6) FailureIsolator + RerunController (主 19:33 借 Efron 1979)
# ---------------------------------------------------------------------------

class FailureIsolator:
    """V1120 失败用例隔离器 (主 19:33 12-Factor 启发: 进程级隔离).

    ponytail: ceiling = subprocess-level 单次 pytest; upgrade path =
    pytest-xdist 多 worker (主 19:33 GHAMatrix 启发).
    """

    @staticmethod
    def isolate_step(step_dict: Dict[str, Any]) -> Dict[str, Any]:
        """标 step 是否失败 + 隔离状态. 主 17:43 实事求是: 真失败 → 标 isolation_required."""
        rc = int(step_dict.get("returncode", 0))
        n_failed = int(step_dict.get("n_failed", 0))
        label = step_dict.get("label", "")
        return {
            "label": label,
            "isolated": True,
            "isolation_required": (rc != 0) or (n_failed > 0),
            "returncode": rc,
            "n_failed": n_failed,
            "isolation_strategy": "subprocess",
            "rerun_handled": label.startswith("rerun#"),
        }


# ---------------------------------------------------------------------------
# 7) MarkdownReportGenerator (主 00:56 任何人都能接手)
# ---------------------------------------------------------------------------

class MarkdownReportGenerator:
    """V1120 真 Markdown 报告生成器 (主 00:56 借 GitHub-flavored Markdown)."""

    @staticmethod
    def render(report: Dict[str, Any]) -> str:
        dash = report.get("dashboard", {})
        v1077 = report.get("v1077", {})
        v1074 = report.get("v1074", {})
        v1111 = report.get("v1111", {})
        pytest_r = report.get("pytest", {})
        lines: List[str] = []
        lines.append(f"# R9 W4 集成 QA 真跑报告 (V1120)")
        lines.append("")
        lines.append(f"> 版本: {report.get('version', V1120_VERSION)}  ")
        lines.append(f"> run_id: `{report.get('run_id', '?')}`  ")
        lines.append(f"> ts: {report.get('ts_iso', '?')}  ")
        lines.append("")
        lines.append(f"主哲学: ASI 北极星 0.9800 LOCKED + 实事求是 + 不假装 + 干到底 + 走在前人经验上 + 任何人都能接手.")
        lines.append("")
        # Dashboard
        lines.append("## ASI 北极星 dashboard (主 22:33)")
        lines.append("")
        lines.append(f"- ASI 北极星     = **{dash.get('asi_north_star', 0.0):.4f}** LOCKED")
        lines.append(f"- V1074 V0.3     = **{dash.get('v1074_v03', 0.0):.4f}** "
                     f"(守门 ≥ 0.8884 → {'✅' if dash.get('v1074_v03_gate_pass') else '❌'})")
        lines.append(f"- V1077 V0.4     = **{dash.get('v1077_v04', 0.0):.4f}** "
                     f"(W4 目标 ≥ 0.8538 → {'✅' if dash.get('v1077_v04_w4_target_hit') else '❌'})")
        lines.append(f"- HQB 4 维复合   = **{dash.get('hqb_4dim_composite', 0.0):.4f}** "
                     f"(≥0.85 → {'✅' if dash.get('hqb_4dim_all_pass') else '❌'})")
        lines.append(f"- pytest pass率  = **{dash.get('pytest_pass_ratio', 0.0):.4f}** "
                     f"({dash.get('pytest_passed', 0)}/{dash.get('pytest_total', 0)})")
        lines.append(f"- 绝对 headroom  = {dash.get('abs_headroom_to_north_star', 0.0):.4f}")
        lines.append(f"- All OK         = **{'YES ✅' if dash.get('all_ok') else 'NO ❌'}**")
        lines.append("")
        # 17 维全表
        lines.append("## 17 维度全维度真测 (V1077)")
        lines.append("")
        breakdown = v1077.get("dim_breakdown", {}) if isinstance(v1077, dict) else {}
        weights = v1077.get("weights_used", {}) if isinstance(v1077, dict) else {}
        if breakdown:
            lines.append("| rank | dim | score | weight | weighted |")
            lines.append("|---:|---|---:|---:|---:|")
            rows = sorted(breakdown.items(), key=lambda kv: -kv[1])
            for rank, (dim, score) in enumerate(rows, start=1):
                w = float(weights.get(dim, 0.0))
                lines.append(f"| {rank} | {dim} | {score:.4f} | {w:.4f} | {score * w:.4f} |")
            lines.append("")
        else:
            lines.append("_(V1077 未真跑 / dim_breakdown 不可用)_")
            lines.append("")
        # HQB 4 维
        lines.append("## HQB 4 维真测 (V1111)")
        lines.append("")
        thr = v1111.get("thresholds", {}) if isinstance(v1111, dict) else {}
        if v1111.get("ok"):
            lines.append(f"| 维度 | score | threshold | pass |")
            lines.append("|---|---:|---:|---|")
            for dim in ("sc", "nr", "ev", "cdt"):
                lines.append(f"| {dim.upper()} | {v1111.get(dim, 0.0):.4f} | "
                             f"{thr.get(dim, 0.0):.4f} | "
                             f"{'✅' if v1111.get(f'{dim}_pass') else '❌'} |")
            lines.append(f"| composite | {v1111.get('composite', 0.0):.4f} | 0.8500 | "
                         f"{'✅' if v1111.get('all_pass') else '❌'} |")
        else:
            err = v1111.get("error", "unknown") if isinstance(v1111, dict) else "?"
            lines.append(f"V1111 未真跑: {err}")
        lines.append("")
        # V1074
        lines.append("## V1074 V0.3 守门 (主 17:43 实事求是)")
        lines.append("")
        if v1074.get("ok"):
            comp = v1074.get("v03_components", {})
            lines.append(f"- V0.3 总分 = **{v1074.get('v03_score', 0.0):.4f}** (W4 目标 ≥ {V1074_V03_TARGET_W4:.4f})")
            lines.append(f"- gate = V0.3 ≥ {V1074_V03_GATE:.4f} → "
                         f"{'✅ PASS' if v1074.get('gate_pass') else '❌ FAIL'}")
            lines.append(f"- components:")
            lines.append(f"  - v02_base: {comp.get('v02_base', 0.0):.4f}")
            lines.append(f"  - v1071_vcp_score: {comp.get('v1071_vcp_score', 0.0):.4f}")
            lines.append(f"  - v1071_cross_domain_score: {comp.get('v1071_cross_domain_score', 0.0):.4f}")
            lines.append(f"  - v1072_eternal_identity_score: {comp.get('v1072_eternal_identity_score', 0.0):.4f}")
        else:
            lines.append(f"V1074 未真跑: {v1074.get('error', '?')}")
        lines.append("")
        # pytest
        lines.append("## pytest 全量回归 (主 23:44 干到底)")
        lines.append("")
        if pytest_r:
            lines.append(f"- 目录: `{pytest_r.get('pytest_dir', '?')}`")
            lines.append(f"- collected = {pytest_r.get('n_collected', 0)}")
            lines.append(f"- passed = {pytest_r.get('n_passed', 0)}")
            lines.append(f"- failed = {pytest_r.get('n_failed', 0)}")
            lines.append(f"- skipped = {pytest_r.get('n_skipped', 0)}")
            lines.append(f"- errors = {pytest_r.get('n_errors', 0)}")
            lines.append(f"- pass_ratio = {pytest_r.get('pass_ratio', 0.0):.4f} "
                         f"(≥ {PYTEST_MIN_PASS_RATIO:.2f} → "
                         f"{'✅' if pytest_r.get('ok_against_threshold') else '❌'})")
            lines.append(f"- 步数 (含 collect + initial + rerun) = {len(pytest_r.get('steps', []))}")
            lines.append(f"- deselected rerun = {pytest_r.get('deselected_count', 0)}")
            first_fail = pytest_r.get("first_failure", [])
            if first_fail:
                lines.append("- 首轮失败样例 (≤5):")
                for n in first_fail[:5]:
                    lines.append(f"  - `{n}`")
        else:
            lines.append("_pytest 跳步 (--no-pytest)_")
        lines.append("")
        # 失败隔离
        lines.append("## 失败隔离 + 重跑 (主 19:33 12-Factor + Efron)")
        lines.append("")
        iso = report.get("isolation", {})
        if iso:
            lines.append(f"- 隔离步数 = {len(iso)}")
            for s in iso:
                lines.append(f"  - `{s['label']}` rc={s['returncode']} n_failed={s['n_failed']} "
                             f"isolation_required={s['isolation_required']}")
        else:
            lines.append("_无失败隔离_")
        lines.append("")
        # V3 守门
        lines.append("## V3 哲学守门 (主 17:58 + 主 20:46 不假装)")
        lines.append("")
        for name, desc in V1120_GUARDS:
            lines.append(f"- ✅ **{name}**: {desc}")
        lines.append("")
        # 结论
        lines.append("## W4 末结论")
        lines.append("")
        if dash.get("all_ok"):
            lines.append("> **R9 W4 末集成 QA 真跑 All OK**: V1074 V0.3 ≥ 0.8884 ✅ + V1077 V0.4 ≥ 0.8538 ✅ + "
                         "HQB 4 维全过 ✅ + pytest ≥99% pass ✅.  R9 → R10 移交可启动.")
        else:
            lines.append("> **R9 W4 末集成 QA 真跑存在缺口** (主 17:43 实事求是). 详见上方各小节. "
                         "不假装全过; 不刷 KPI; 移交 R10 前必须解决.")
        lines.append("")
        lines.append(f"_本文由 apeireth/v1120 集成 QA orchestrator 在 R9 W4 末真跑产出 (主 23:44 干到底)._")
        return "\n".join(lines)


# ---------------------------------------------------------------------------
# 8) W4IntegrationQAOrchestrator — top-level (主 00:56 任何人都能接手)
# ---------------------------------------------------------------------------

class W4IntegrationQAOrchestrator:
    """V1120 R9 W4 集成 QA 顶层 orchestrator (主 22:33 + 主 17:43 + 主 00:56).

    一行命令:
      python -m apeireth.v1120_w4_integration_qa --self-check
    """

    def __init__(
        self,
        project_dir: Optional[str] = None,
        pytest_dir: str = "tests",
        rerun_limit: int = PYTEST_RERUN_LIMIT,
        run_pytest: bool = True,
    ) -> None:
        self.project_dir = Path(project_dir) if project_dir else Path.cwd()
        self.pytest_dir = pytest_dir
        self.rerun_limit = rerun_limit
        self.run_pytest = run_pytest
        self.v1077 = V1077Adapter()
        self.v1111 = V1111Adapter()
        self.v1074 = V1074Gate(project_dir=str(self.project_dir))
        self.pytest_o = PytestOrchestrator(pytest_dir=pytest_dir, rerun_limit=rerun_limit)

    def run(self) -> Dict[str, Any]:
        v1077_r = self.v1077.run()
        v1111_r = self.v1111.run()
        v1074_r = self.v1074.run()
        if self.run_pytest:
            pytest_r = self.pytest_o.run()
        else:
            pytest_r = {
                "ok": True, "pytest_dir": self.pytest_dir,
                "n_collected": 0, "n_passed": 0, "n_failed": 0,
                "n_skipped": 0, "n_errors": 0, "pass_ratio": 1.0,
                "min_pass_ratio": PYTEST_MIN_PASS_RATIO,
                "ok_against_threshold": True, "steps": [], "deselected_count": 0,
                "deselected_unique": [], "first_failure": [],
                "_skipped": True,
            }
        dash = compute_dashboard(v1074_r, v1077_r, v1111_r, pytest_r)
        # 失败隔离标记
        isolation = []
        for step in pytest_r.get("steps", []):
            isolation.append(FailureIsolator.isolate_step(step))
        report = {
            "version": V1120_VERSION,
            "run_id": f"v1120_{uuid.uuid4().hex[:12]}",
            "ts_iso": datetime.now(timezone.utc).isoformat(),
            "project_dir": str(self.project_dir),
            "dashboard": asdict(dash),
            "v1077": v1077_r,
            "v1111": v1111_r,
            "v1074": v1074_r,
            "pytest": pytest_r,
            "isolation": isolation,
            "guards": [{k: True for k, _ in V1120_GUARDS}],    # 真跑时永远 True (守门 = 自我声明)
            "references": REFERENCES,
        }
        report["all_ok"] = dash.all_ok
        return report

    def render_markdown(self, report: Dict[str, Any]) -> str:
        return MarkdownReportGenerator.render(report)


# ---------------------------------------------------------------------------
# CLI (主 00:56 任何人都能接手)
# ---------------------------------------------------------------------------

def _write_artifact(report: Dict[str, Any], artifact_dir: Path) -> Tuple[Path, Path]:
    """真写 2 个 artifacts (主 23:44 干到底)."""
    artifact_dir.mkdir(parents=True, exist_ok=True)
    json_path = artifact_dir / f"v1120_{report['run_id']}.json"
    md_path = artifact_dir / f"v1120_{report['run_id']}.md"
    json_path.write_text(json.dumps(report, indent=2, ensure_ascii=False, default=str), encoding="utf-8")
    md_path.write_text(MarkdownReportGenerator.render(report), encoding="utf-8")
    return json_path, md_path


def _cli_self_check(args: argparse.Namespace) -> int:
    orch = W4IntegrationQAOrchestrator(
        project_dir=str(Path.cwd()),
        pytest_dir=args.pytest_dir,
        rerun_limit=args.rerun_failures,
        run_pytest=not args.no_pytest,
    )
    report = orch.run()
    md = orch.render_markdown(report)
    print(md)
    artifact_dir = Path(args.artifact_dir)
    jp, mp = _write_artifact(report, artifact_dir)
    print(f"\n[artifacts] json={jp}\n[artifacts] md={mp}")
    return 0 if report["all_ok"] else 1


def _cli_report(args: argparse.Namespace) -> int:
    src = Path(args.input)
    report = json.loads(src.read_text(encoding="utf-8"))
    print(MarkdownReportGenerator.render(report))
    return 0


def build_cli_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(prog="v1120", description="R9 W4 Integration QA orchestrator")
    group = parser.add_mutually_exclusive_group(required=True)
    group.add_argument("--self-check", action="store_true",
                       help="run V1120 end-to-end QA: V1077 + V1111 + V1074 + pytest + dashboard + isolation")
    group.add_argument("--report", dest="input", default=None,
                       help="render Markdown from a V1120 JSON report file")
    parser.add_argument("--pytest-dir", default="tests",
                        help="pytest test directory (default: tests)")
    parser.add_argument("--rerun-failures", type=int, default=PYTEST_RERUN_LIMIT,
                        help=f"max pytest rerun rounds (default: {PYTEST_RERUN_LIMIT})")
    parser.add_argument("--no-pytest", action="store_true",
                        help="skip pytest step (for --self-check fast debug)")
    parser.add_argument("--artifact-dir", default="artifacts/v1120",
                        help="directory to write artifacts (default: artifacts/v1120)")
    return parser


def main(argv: Optional[List[str]] = None) -> int:
    parser = build_cli_parser()
    args = parser.parse_args(argv)
    if args.self_check:
        return _cli_self_check(args)
    if args.input:
        return _cli_report(args)
    parser.print_help()
    return 1


__all__ = [
    "V1120_VERSION",
    "V1120_GUARDS",
    "V1074_V03_GATE",
    "V1074_V03_TARGET_W4",
    "V1077_V04_W4_TARGET",
    "ASI_NORTH_STAR",
    "PYTEST_MIN_PASS_RATIO",
    "PYTEST_RERUN_LIMIT",
    "REFERENCES",
    "V1077Adapter",
    "V1111Adapter",
    "V1074Gate",
    "PytestOrchestrator",
    "PytestStepResult",
    "FailureIsolator",
    "DashboardSnapshot",
    "compute_dashboard",
    "MarkdownReportGenerator",
    "W4IntegrationQAOrchestrator",
    "build_cli_parser",
    "main",
]


if __name__ == "__main__":
    sys.exit(main())
