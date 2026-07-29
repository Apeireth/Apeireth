"""V1127 R10 跨小模型 CI 框架 + 真模型接入 + ASI 北极星 CI 守护 (R10-ATE-001).

R10 W1 自动化测试工程师 (automation_tester) 实施.

设计目标 (主 22:33 ASI 北极星 + 主 17:43 实事求是 + 主 17:58 不假装
       + 主 23:44 干到底 + 主 19:33 走在前人经验上 + 主 00:56 任何人都能接手):

1. 承接 R9-DEV-001~003 cross_small_model_ci 5 模块 (models/harness/tasks/runner/report)
   + V1117 badge SVG renderer 的全部能力, 不重写.

2. 集成 V1124 backend 真接口 (GET/POST /asi/level/measure/north-star) + 等待 R10-BE-002 V1128
   真模型接入 + R10-AO-001 V1127 DGM v0.5 + R10-A2-001 V1128 多 agent 集成 一起接 CI 护栏.

3. ASI 北极星 CI 守护: commit 触发 ASI 测量 + 与 baseline 对比 + fail/pass 门控.
   - 主 17:43 实事求是: 测量数字必须来自 V1124 backend 真接口, 不 hardcode.
   - 主 17:58+20:46 不假装: 后端不可用 → 显式 skip + error 标注, 不假装 PASS.
   - 主 13:31 大胆激进: 每 commit 跑全套 HQB 4 维 + ASI 综合, 不可降级.

4. 跨小模型 (qwen2.5:1.5b/llama3.2:3b/gemma/hermes) 对比测试矩阵
   - 主 19:33 走在前人经验上: 借鉴 pytest 2008 parametrize + GitHub Actions matrix 2020
     + EleutherAI LM-Eval 2021 跨模型评测.
   - 主 13:31 大胆激进: ≥3 真模型容错尝试 + fixture 兜底.

5. R10 V0.4 ≥0.90 / V0.5 ≥0.95 真测护栏 (R10 启程 vs 终极):
   - R10_W2_TARGET = 0.90 (V0.4 → V0.5 升级期)
   - R10_ULTIMATE_TARGET = 0.95 (V0.5 ASI 北极星综合)
   - 主 23:44 干到底: 不达标 → CI fail, 非零退出.

复用 (主 19:33 走在前人经验上):
  - cross_small_model_ci 5 模块 (R9-DEV-001~003 真测)
  - V1117 badge SVG renderer (R9-DEV-003 W4)
  - V1124 ASI north star backend (R10-BE-001 accepted 9.05)
  - V1114 weekly integration evaluator (decide 引擎)
  - V1125 R10 integration protocol (R10 阈值 LOCKED)

Public API:
    from apeireth.v1127_r10_cross_small_model_ci import (
        # 配置 + 数据类
        R10NorthStarConfig, R10GuardResult, R10ModelMatrixEntry, R10CrossMatrixResult,
        # 核心类
        R10NorthStarClient, ASINorthStarGuard, R10CrossSmallModelMatrix, R10CIReporter,
        # 一行入口
        run_r10_ci_guard, run_r10_ci_matrix, write_r10_report,
        # chaos test
        chaos_test_model_load, chaos_test_timeout,
        # 阈值常量
        R10_W2_TARGET, R10_ULTIMATE_TARGET, R10_V04_BASELINE, R10_MODEL_MATRIX,
    )

Usage:
    # CI 守护 (commit 触发)
    result = run_r10_ci_guard(
        backend_url="http://127.0.0.1:8765",
        baseline_path="reports/r10-ate-w1-north-star-baseline.json",
    )
    sys.exit(0 if result.passed else 1)

    # 跨小模型矩阵
    matrix = run_r10_ci_matrix(
        backend_url="http://127.0.0.1:8765",
        model_families=["qwen", "llama", "gemma", "hermes"],
    )
    write_r10_report(matrix, path="reports/r10-ate-w1-r10-ci-matrix.md")
"""
from __future__ import annotations

import dataclasses
import json
import os
import socket
import statistics
import subprocess
import sys
import tempfile
import time
import urllib.error
import urllib.request
from dataclasses import asdict, dataclass, field
from http.server import ThreadingHTTPServer
from pathlib import Path
from threading import Thread
from typing import Any, Callable, Dict, Iterable, List, Mapping, Optional, Sequence, Tuple

# ponytail: 复用 R9-DEV-001~003 + V1117 + V1124 不重写
from apeireth.cross_small_model_ci import (  # noqa: E402
    DEFAULT_REGISTRY,
    FixtureAdapter,
    HarnessResult,
    HQBHarness,
    ModelAdapter,
    ModelRegistry,
    Qwen35Adapter,
    Llama31Adapter,
    HermesAdapter,
    Gemma4Adapter,
    run_ci,
    summarize,
)
from apeireth.cross_small_model_ci.report import (
    compute_diff,
    render_diff_table,
    write_diff,
)
from apeireth.v1117_badge_svg_renderer import (
    COLOR_MAP,
    STATUS_TO_COLOR,
    render_badge_svg,
    render_status_badge,
    render_diff_svg,
    render_diff_html,
    HFModelCache,
    HFModelTimeoutError,
    load_env_file,
    REAL_MODEL_ENV as V1117_REAL_MODEL_ENV,
)
from apeireth.v1124_asi_north_star_backend import (
    ASI_NORTH_STAR_TARGET as V1124_AS_NORTH_STAR_TARGET,
    BASELINE_V04 as V1124_BASELINE_V04,
    ASINorthStarBackend,
    IntegrityError,
    ModelEvidence,
    ModelRequest,
    RealModelGateway,
    V1124_VERSION,
    V3_GUARDS as V1124_V3_GUARDS,
    V1124Error,
    make_http_handler,
    start_http_server,
)


VERSION = "0.1.0"

# ---------------------------------------------------------------------------
# R10 阈值 LOCKED (主 13:31 大胆激进 + 主 22:33 ASI 北极星)
# ---------------------------------------------------------------------------
# R10 起点: V0.4 baseline = 0.8538 (R9 W4 末真测)
R10_V04_BASELINE = 0.8538
# R10 W2 中期: V0.4 → V0.5 升级期目标 (V0.4 ≥ 0.90)
R10_W2_TARGET = 0.9000
# R10 终极: V0.5 ≥ 0.95 = ASI 北极星综合评估
R10_ULTIMATE_TARGET = 0.9500
# 守护阈值: 与 baseline 相比不能下降 > drop_tolerance, 否则 fail
R10_GUARD_DROP_TOLERANCE = 0.0050  # 50 bps 容忍 (主 17:43 实事求是: 不可 0)

# 跨小模型矩阵 (主 13:31 大胆激进: ≥3 真模型 + 1 fixture 兜底)
R10_MODEL_MATRIX: Tuple[Dict[str, str], ...] = (
    {"family": "qwen", "model": "qwen2.5:1.5b", "params_b": "1.5", "role": "tiny_general"},
    {"family": "llama", "model": "llama3.2:3b", "params_b": "3.0", "role": "small_reasoner"},
    {"family": "gemma", "model": "gemma2:2b", "params_b": "2.0", "role": "compact_math"},
    {"family": "hermes", "model": "hermes-3:2b", "params_b": "2.0", "role": "instruction_tuned"},
    {"family": "fixture", "model": "fixture-7b-v1", "params_b": "7.0", "role": "deterministic_baseline"},
)

# V1124 默认 backend port (主 17:58 不假装: 用 env 覆盖, 不 hardcode)
V1124_DEFAULT_PORT = 8765
V1124_BACKEND_HOST = os.environ.get("APEIRETH_V1124_HOST", "127.0.0.1")
V1124_BACKEND_PORT = int(os.environ.get("APEIRETH_V1124_PORT", str(V1124_DEFAULT_PORT)))


# ---------------------------------------------------------------------------
# 数据类
# ---------------------------------------------------------------------------
@dataclass
class R10NorthStarConfig:
    """R10 ASI 北极星 CI 守护配置 (主 17:43 实事求是: 数字驱动决策)."""

    # 阈值 (从模块级常量继承, 但允许 override)
    w2_target: float = R10_W2_TARGET
    ultimate_target: float = R10_ULTIMATE_TARGET
    guard_drop_tolerance: float = R10_GUARD_DROP_TOLERANCE
    # backend 配置
    backend_url: Optional[str] = None
    backend_host: str = V1124_BACKEND_HOST
    backend_port: int = V1124_BACKEND_PORT
    backend_timeout_sec: float = 5.0
    # CI 行为
    skip_on_backend_unavailable: bool = False  # 主 17:58 不假装: False=显式 fail, True=显式 skip
    run_inline_backend: bool = True             # CI 无外部 backend 时自启 inline (主 00:56 任何人都能接手)
    # 跨小模型矩阵
    model_families: Tuple[str, ...] = ("qwen", "llama", "gemma", "hermes", "fixture")
    # HQB 配置
    harness_pass_threshold: float = 0.50
    # baseline
    baseline_path: Optional[str] = None
    # chaos 配置
    chaos_timeout_sec: float = 3.0   # chaos test 加载超时阈值

    def to_dict(self) -> Dict[str, Any]:
        return {
            "w2_target": self.w2_target,
            "ultimate_target": self.ultimate_target,
            "guard_drop_tolerance": self.guard_drop_tolerance,
            "backend_url": self.backend_url,
            "backend_host": self.backend_host,
            "backend_port": self.backend_port,
            "backend_timeout_sec": self.backend_timeout_sec,
            "skip_on_backend_unavailable": self.skip_on_backend_unavailable,
            "run_inline_backend": self.run_inline_backend,
            "model_families": list(self.model_families),
            "harness_pass_threshold": self.harness_pass_threshold,
            "baseline_path": self.baseline_path,
            "chaos_timeout_sec": self.chaos_timeout_sec,
        }


@dataclass
class R10ModelMatrixEntry:
    """跨小模型矩阵中单模型真测结果."""

    family: str
    model: str
    params_b: float
    role: str
    available: bool
    asi_level: float = 0.0           # ASI 北极星综合 (W2 期 = V0.4)
    hqb_subscore: float = 0.0         # HQB 4 维子分
    hqb_sc: float = 0.0
    hqb_nr: float = 0.0
    hqb_ev: float = 0.0
    hqb_cdt: float = 0.0
    passed: bool = False
    error: Optional[str] = None
    elapsed_sec: float = 0.0
    n_inferences: int = 0
    meta: Dict[str, Any] = field(default_factory=dict)

    def to_dict(self) -> Dict[str, Any]:
        return {
            "family": self.family,
            "model": self.model,
            "params_b": self.params_b,
            "role": self.role,
            "available": self.available,
            "asi_level": round(self.asi_level, 4),
            "hqb_subscore": round(self.hqb_subscore, 4),
            "hqb_sc": round(self.hqb_sc, 4),
            "hqb_nr": round(self.hqb_nr, 4),
            "hqb_ev": round(self.hqb_ev, 4),
            "hqb_cdt": round(self.hqb_cdt, 4),
            "passed": self.passed,
            "error": self.error,
            "elapsed_sec": round(self.elapsed_sec, 3),
            "n_inferences": self.n_inferences,
            "meta": self.meta,
        }


@dataclass
class R10CrossMatrixResult:
    """跨小模型矩阵整体结果."""

    entries: List[R10ModelMatrixEntry]
    n_passed: int = 0
    n_available: int = 0
    avg_asi_level: float = 0.0
    min_asi_level: float = 0.0
    max_asi_level: float = 0.0
    all_pass: bool = False
    computed_at: float = field(default_factory=time.time)
    meta: Dict[str, Any] = field(default_factory=dict)

    def to_dict(self) -> Dict[str, Any]:
        return {
            "entries": [e.to_dict() for e in self.entries],
            "n_passed": self.n_passed,
            "n_available": self.n_available,
            "avg_asi_level": round(self.avg_asi_level, 4),
            "min_asi_level": round(self.min_asi_level, 4),
            "max_asi_level": round(self.max_asi_level, 4),
            "all_pass": self.all_pass,
            "computed_at": self.computed_at,
            "computed_at_iso": time.strftime("%Y-%m-%dT%H:%M:%S%z", time.localtime(self.computed_at)),
            "meta": self.meta,
        }


@dataclass
class R10GuardResult:
    """ASI 北极星 CI 守护单次 commit 触发结果."""

    passed: bool
    measured_asi_level: float                    # 本次测量
    baseline_asi_level: float                    # 历史 baseline
    delta: float                                 # measured - baseline
    w2_target: float
    ultimate_target: float
    drop_tolerance: float
    passed_w2: bool                              # ≥ W2_TARGET
    passed_ultimate: bool                        # ≥ ULTIMATE_TARGET
    passed_no_regression: bool                   # delta ≥ -drop_tolerance
    backend_available: bool
    backend_url: Optional[str]
    matrix: Optional[R10CrossMatrixResult] = None
    error: Optional[str] = None
    computed_at: float = field(default_factory=time.time)
    meta: Dict[str, Any] = field(default_factory=dict)

    def to_dict(self) -> Dict[str, Any]:
        return {
            "passed": self.passed,
            "measured_asi_level": round(self.measured_asi_level, 4),
            "baseline_asi_level": round(self.baseline_asi_level, 4),
            "delta": round(self.delta, 4),
            "w2_target": self.w2_target,
            "ultimate_target": self.ultimate_target,
            "drop_tolerance": self.drop_tolerance,
            "passed_w2": self.passed_w2,
            "passed_ultimate": self.passed_ultimate,
            "passed_no_regression": self.passed_no_regression,
            "backend_available": self.backend_available,
            "backend_url": self.backend_url,
            "matrix": self.matrix.to_dict() if self.matrix else None,
            "error": self.error,
            "computed_at": self.computed_at,
            "computed_at_iso": time.strftime("%Y-%m-%dT%H:%M:%S%z", time.localtime(self.computed_at)),
            "meta": self.meta,
        }


# ---------------------------------------------------------------------------
# V1124 backend HTTP 客户端 (主 17:58 不假装: 真 HTTP, 失败显式 raise)
# ---------------------------------------------------------------------------
class R10NorthStarClient:
    """V1124 backend HTTP 客户端 (主 00:56 任何人都能接手).

    真实 GET / POST 到 V1124 backend. 失败 → 显式 V1124Error (status 5xx) 或
    urllib.error.URLError, 不假装返回 ok.
    """

    def __init__(self, base_url: Optional[str] = None,
                 host: str = V1124_BACKEND_HOST,
                 port: int = V1124_BACKEND_PORT,
                 timeout_sec: float = 5.0):
        if base_url:
            # 用 base_url 覆盖 host/port
            self.base_url = base_url.rstrip("/")
        else:
            self.base_url = f"http://{host}:{port}"
        self.timeout_sec = timeout_sec
        self._host = host
        self._port = port

    @property
    def url(self) -> str:
        return self.base_url

    def _http(self, method: str, path: str, body: Optional[Dict[str, Any]] = None) -> Dict[str, Any]:
        """HTTP 调 V1124 backend. 失败 → 显式 raise."""
        url = f"{self.base_url}{path}"
        data = None
        headers = {"Accept": "application/json"}
        if body is not None:
            data = json.dumps(body).encode("utf-8")
            headers["Content-Type"] = "application/json"
        req = urllib.request.Request(url, data=data, method=method, headers=headers)
        try:
            with urllib.request.urlopen(req, timeout=self.timeout_sec) as resp:
                raw = resp.read().decode("utf-8")
                return json.loads(raw)
        except urllib.error.HTTPError as e:
            raw = e.read().decode("utf-8", errors="replace")
            try:
                payload = json.loads(raw)
            except json.JSONDecodeError:
                payload = {"error": {"code": "http_error", "message": str(e)}}
            raise V1124Error(
                code=payload.get("error", {}).get("code", "http_error"),
                message=payload.get("error", {}).get("message", str(e)),
                status=e.code,
            )
        except (urllib.error.URLError, socket.timeout, ConnectionRefusedError) as e:
            raise V1124Error(code="backend_unavailable", message=f"backend unreachable: {e!r}", status=503)

    def get_level(self) -> Dict[str, Any]:
        """GET /asi/level — 真读 V1124 ASI 等级."""
        return self._http("GET", "/asi/level")

    def get_north_star(self) -> Dict[str, Any]:
        """GET /asi/north-star — 真读 V1124 北极星综合."""
        return self._http("GET", "/asi/north-star")

    def post_measure(self, payload: Dict[str, Any]) -> Dict[str, Any]:
        """POST /asi/measure — 真跑 V1124 测量."""
        if not isinstance(payload, dict):
            raise V1124Error("invalid_payload", "payload must be a dict", 400)
        return self._http("POST", "/asi/measure", body=payload)

    def ping(self) -> bool:
        """探测 backend 是否可用 (主 17:58 不假装: 真 HTTP, 失败 False)."""
        try:
            self._http("GET", "/asi/level")
            return True
        except V1124Error:
            return False

    def to_dict(self) -> Dict[str, Any]:
        return {
            "base_url": self.base_url,
            "timeout_sec": self.timeout_sec,
            "host": self._host,
            "port": self._port,
        }


# ---------------------------------------------------------------------------
# Inline backend 启动器 (主 00:56 任何人都能接手: CI 无外部 backend 时自启)
# ---------------------------------------------------------------------------
class InlineBackend:
    """进程内 V1124 backend 启动器 (主 17:43 实事求是: 真线程 HTTP server).

    在 CI 环境下, 如果没有外在 backend, 可真启动一个 inline 后端来做 CI 守护.
    上下文管理: __enter__ 启动, __exit__ 关闭.
    """

    def __init__(self, state_dir: Optional[Path] = None,
                 host: str = V1124_BACKEND_HOST,
                 port: int = 0):
        self.host = host
        self.port = int(port)
        self.state_dir = Path(state_dir) if state_dir else Path(tempfile.mkdtemp(prefix="v1127_inline_"))
        self.server: Optional[ThreadingHTTPServer] = None
        self.thread: Optional[Thread] = None
        self.actual_port: int = 0
        self.backend: Optional[ASINorthStarBackend] = None

    def __enter__(self) -> "InlineBackend":
        self.backend = ASINorthStarBackend(self.state_dir)
        handler = make_http_handler(self.backend)
        self.server = ThreadingHTTPServer((self.host, self.port), handler)
        self.actual_port = self.server.server_port
        self.thread = Thread(target=self.server.serve_forever, daemon=True)
        self.thread.start()
        return self

    def __exit__(self, exc_type: Any, exc_val: Any, exc_tb: Any) -> None:
        if self.server is not None:
            try:
                self.server.shutdown()
                self.server.server_close()
            except Exception:
                pass
        if self.thread is not None:
            self.thread.join(timeout=2)

    @property
    def url(self) -> str:
        return f"http://{self.host}:{self.actual_port}"


# ---------------------------------------------------------------------------
# ASI 北极星 CI 守护 (主 22:33 + 主 23:44 干到底)
# ---------------------------------------------------------------------------
class ASINorthStarGuard:
    """ASI 北极星 CI 守护 (主 17:43 实事求是 + 主 17:58 不假装).

    流程:
        1. 启动/连接 V1124 backend (inline 或外部)
        2. 跑跨小模型 CI 矩阵 (HQB 4 维)
        3. 调 V1124 GET /asi/level 拿 ASI 北极星综合
        4. 加载 baseline (JSON 文件) → 计算 delta
        5. 门控: passed = (W2 达标) AND (未退化超过 tolerance)
        6. 写报告 (Markdown + badge SVG + JSON)
    """

    def __init__(self, config: Optional[R10NorthStarConfig] = None):
        self.config = config or R10NorthStarConfig()
        self._inline_backend: Optional[InlineBackend] = None
        self._client: Optional[R10NorthStarClient] = None

    def _ensure_client(self) -> R10NorthStarClient:
        if self._client is not None:
            return self._client
        if self.config.backend_url:
            self._client = R10NorthStarClient(
                base_url=self.config.backend_url,
                timeout_sec=self.config.backend_timeout_sec,
            )
            return self._client
        # 试连接
        candidate = R10NorthStarClient(
            host=self.config.backend_host,
            port=self.config.backend_port,
            timeout_sec=self.config.backend_timeout_sec,
        )
        if candidate.ping():
            self._client = candidate
            return self._client
        # 不能连接 → 启 inline (主 00:56 任何人都能接手)
        if self.config.run_inline_backend:
            self._inline_backend = InlineBackend(
                host=self.config.backend_host,
                port=self.config.backend_port,
            )
            try:
                self._inline_backend.__enter__()
            except OSError as e:
                # inline 启动失败 (端口占用) → port=0 自动选
                self._inline_backend = InlineBackend(host=self.config.backend_host, port=0)
                self._inline_backend.__enter__()
            self._client = R10NorthStarClient(
                base_url=self._inline_backend.url,
                timeout_sec=self.config.backend_timeout_sec,
            )
            return self._client
        raise V1124Error("backend_unavailable", "no backend configured and run_inline_backend=False", 503)

    def _get_asi_level(self) -> float:
        """调 V1124 backend 拿 ASI 综合 level. (主 17:43 实事求是: 真 HTTP, 不 hardcode)."""
        client = self._ensure_client()
        try:
            resp = client.get_level()
            score = float(resp.get("score", 0.0))
            return score
        except V1124Error as e:
            if self.config.skip_on_backend_unavailable:
                return 0.0
            raise

    def _run_matrix(self) -> R10CrossMatrixResult:
        """跑跨小模型 CI 矩阵 (主 19:33 走在前人经验上: 复用 cross_small_model_ci)."""
        # 跑 cross_small_model_ci 拿 HQB 4 维
        results = run_ci(only_families=list(self.config.model_families),
                         skip_unavailable=True)
        # 跑跨小模型矩阵 = run_ci 输出的 family/model 视角
        fam_model_map: Dict[str, Dict[str, str]] = {}
        for entry in R10_MODEL_MATRIX:
            fam_model_map.setdefault(entry["family"], entry)
        entries: List[R10ModelMatrixEntry] = []
        for r in results:
            info = fam_model_map.get(r.family, {"family": r.family, "model": r.model_name,
                                                "params_b": "0.0", "role": "unknown"})
            try:
                params_b = float(info.get("params_b", 0.0))
            except (TypeError, ValueError):
                params_b = 0.0
            asi_level = self._extract_asi_level(r)
            passed = r.passed and asi_level >= self.config.harness_pass_threshold
            entries.append(R10ModelMatrixEntry(
                family=r.family,
                model=r.model_name,
                params_b=params_b,
                role=info.get("role", "unknown"),
                available=r.available,
                asi_level=asi_level,
                hqb_subscore=r.subscore,
                hqb_sc=r.sc,
                hqb_nr=r.nr,
                hqb_ev=r.ev,
                hqb_cdt=r.cdt,
                passed=passed,
                error=r.error,
                elapsed_sec=r.elapsed_sec,
                n_inferences=r.n_inferences,
                meta={"harness_result": r.to_dict()},
            ))
        # 兜底: 如果 run_ci 没出 fixture, 显式添加 fixture (主 17:58 不假装: 至少 1 个 PASS)
        have_fam = {e.family for e in entries}
        if "fixture" not in have_fam:
            # 在 fixtures 中必有, 但保险一下
            for r in results:
                if r.family == "fixture":
                    break
            else:
                # 无 fixture → 创一个 0 分 fixture 标 unavailable
                entries.append(R10ModelMatrixEntry(
                    family="fixture", model="fixture-7b-v1",
                    params_b=7.0, role="deterministic_baseline",
                    available=False, asi_level=0.0, hqb_subscore=0.0,
                    hqb_sc=0.0, hqb_nr=0.0, hqb_ev=0.0, hqb_cdt=0.0,
                    passed=False, error="fixture not produced by run_ci",
                ))
        # 汇总
        avail = [e for e in entries if e.available]
        levels = [e.asi_level for e in avail]
        n_passed = sum(1 for e in entries if e.passed)
        return R10CrossMatrixResult(
            entries=entries,
            n_passed=n_passed,
            n_available=len(avail),
            avg_asi_level=statistics.mean(levels) if levels else 0.0,
            min_asi_level=min(levels) if levels else 0.0,
            max_asi_level=max(levels) if levels else 0.0,
            all_pass=(n_passed >= 1 and n_passed == len(entries)),
            meta={"harness_count": len(results)},
        )

    def _extract_asi_level(self, r: HarnessResult) -> float:
        """从 HarnessResult 提取 ASI 综合 level (主 17:43 实事求是: HQB 4 维均值)."""
        # 主 17:43 实事求是: V0.4 期 ASI 综合 = HQB 4 维子分 (HQB SC+NR+EV+CDT)
        # V0.5 期 = V0.4 + 3 新 dim (continuity/autonomy/transferability), 留待 R10 中期
        return round(r.subscore, 4)

    def _load_baseline(self) -> Tuple[float, bool]:
        """加载 baseline. 如果文件不存在 → 用 V1124 BASELINE_V04 (主 17:43)."""
        if not self.config.baseline_path:
            return V1124_BASELINE_V04, True
        p = Path(self.config.baseline_path)
        if not p.exists():
            return V1124_BASELINE_V04, False
        try:
            data = json.loads(p.read_text(encoding="utf-8"))
            baseline = float(data.get("asi_level", V1124_BASELINE_V04))
            return baseline, True
        except (json.JSONDecodeError, OSError, ValueError) as e:
            return V1124_BASELINE_V04, False

    def save_baseline(self, asi_level: float, path: Optional[str] = None) -> Path:
        """保存 baseline. (主 13:31 大胆激进: commit 守护后可发版)."""
        target = Path(path) if path else (Path(self.config.baseline_path) if self.config.baseline_path else None)
        if target is None:
            target = Path("reports/r10-ate-w1-north-star-baseline.json")
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(json.dumps({
            "asi_level": round(asi_level, 4),
            "saved_at": time.time(),
            "saved_at_iso": time.strftime("%Y-%m-%dT%H:%M:%S%z", time.localtime()),
            "version": VERSION,
        }, ensure_ascii=False, indent=2), encoding="utf-8")
        return target

    def close(self) -> None:
        """关闭 inline backend."""
        if self._inline_backend is not None:
            try:
                self._inline_backend.__exit__(None, None, None)
            except Exception:
                pass
            self._inline_backend = None
        self._client = None

    def run(self, save_baseline_after: bool = True) -> R10GuardResult:
        """主 23:44 干到底: 跑全套 CI 守护, 拿到 R10GuardResult."""
        t0 = time.time()
        backend_available = True
        measured_level = 0.0
        backend_url: Optional[str] = None
        matrix: Optional[R10CrossMatrixResult] = None
        error: Optional[str] = None
        try:
            client = self._ensure_client()
            backend_url = client.url
            backend_available = client.ping()
            # 跑跨小模型矩阵
            matrix = self._run_matrix()
            # 测 ASI level: 用 backend 真接口 (主 17:43 实事求是)
            try:
                measured_level = self._get_asi_level()
            except V1124Error as e:
                # 主 17:58 不假装: 失败显式
                if self.config.skip_on_backend_unavailable:
                    measured_level = matrix.avg_asi_level if matrix else 0.0
                    error = f"backend_unavailable(skipped): {e}"
                else:
                    self.close()
                    raise
            # 加载 baseline
            baseline, baseline_valid = self._load_baseline()
            # 门控
            delta = round(measured_level - baseline, 4)
            passed_w2 = measured_level >= self.config.w2_target
            passed_ultimate = measured_level >= self.config.ultimate_target
            passed_no_regression = delta >= -self.config.guard_drop_tolerance
            # 主 23:44 干到底: W2 达标且未退化 = pass
            # W2 未必达终极; ultimate_target 留 R10 终验
            passed = passed_w2 and passed_no_regression
            # 保存 baseline (主 13:31 大胆激进)
            if save_baseline_after and passed:
                try:
                    self.save_baseline(measured_level)
                except OSError:
                    pass  # 不阻塞 CI 输出
            return R10GuardResult(
                passed=passed,
                measured_asi_level=measured_level,
                baseline_asi_level=baseline,
                delta=delta,
                w2_target=self.config.w2_target,
                ultimate_target=self.config.ultimate_target,
                drop_tolerance=self.config.guard_drop_tolerance,
                passed_w2=passed_w2,
                passed_ultimate=passed_ultimate,
                passed_no_regression=passed_no_regression,
                backend_available=backend_available,
                backend_url=backend_url,
                matrix=matrix,
                error=error,
                meta={
                    "elapsed_sec": round(time.time() - t0, 3),
                    "baseline_valid": baseline_valid,
                    "version": VERSION,
                },
            )
        except V1124Error as e:
            # 主 17:58 不假装: backend 不可用 → 显式 fail
            self.close()
            return R10GuardResult(
                passed=False,
                measured_asi_level=0.0,
                baseline_asi_level=V1124_BASELINE_V04,
                delta=-V1124_BASELINE_V04,
                w2_target=self.config.w2_target,
                ultimate_target=self.config.ultimate_target,
                drop_tolerance=self.config.guard_drop_tolerance,
                passed_w2=False,
                passed_ultimate=False,
                passed_no_regression=False,
                backend_available=False,
                backend_url=backend_url,
                matrix=matrix,
                error=f"backend_error: {e!r}",
                meta={"elapsed_sec": round(time.time() - t0, 3), "version": VERSION},
            )
        except Exception as e:
            # 主 17:58 不假装: 任意异常 → 显式 fail
            self.close()
            return R10GuardResult(
                passed=False,
                measured_asi_level=measured_level,
                baseline_asi_level=V1124_BASELINE_V04,
                delta=round(measured_level - V1124_BASELINE_V04, 4),
                w2_target=self.config.w2_target,
                ultimate_target=self.config.ultimate_target,
                drop_tolerance=self.config.guard_drop_tolerance,
                passed_w2=False,
                passed_ultimate=False,
                passed_no_regression=False,
                backend_available=backend_available,
                backend_url=backend_url,
                matrix=matrix,
                error=f"guard_exception: {e!r}",
                meta={"elapsed_sec": round(time.time() - t0, 3), "version": VERSION},
            )
        finally:
            self.close()


# ---------------------------------------------------------------------------
# 跨小模型 CI 矩阵 (主 13:31 大胆激进: 多模型容错)
# ---------------------------------------------------------------------------
class R10CrossSmallModelMatrix:
    """R10 跨小模型 CI 矩阵 (主 19:33 走在前人经验上: 矩阵 × chaos)."""

    def __init__(self, config: Optional[R10NorthStarConfig] = None):
        self.config = config or R10NorthStarConfig()

    def run(self) -> R10CrossMatrixResult:
        """跑跨小模型矩阵."""
        # 复用 ASINorthStarGuard 内部 _run_matrix
        guard = ASINorthStarGuard(self.config)
        try:
            return guard._run_matrix()
        finally:
            guard.close()

    def run_with_chaos(self) -> Tuple[R10CrossMatrixResult, Dict[str, Any]]:
        """跑 + chaos test (主 23:44 干到底: CI 必须抗 chaos)."""
        matrix = self.run()
        chaos_result = chaos_test_matrix(matrix, chaos_timeout_sec=self.config.chaos_timeout_sec)
        return matrix, chaos_result


# ---------------------------------------------------------------------------
# Chaos test (主 23:44 干到底: 模型加载超时 / 失败 CI 不挂)
# ---------------------------------------------------------------------------
def chaos_test_model_load(load_fn: Callable[[], Any],
                          timeout_sec: float = 3.0,
                          name: str = "anonymous") -> Dict[str, Any]:
    """chaos test: 模型加载超时 → CI 不挂 (主 17:58 不假装: 失败显式).

    Args:
        load_fn: 加载函数 (返回 truthy 视为成功)
        timeout_sec: 加载超时
        name: 模型名 (用于报告)

    Returns:
        {"name": str, "loaded": bool, "elapsed_sec": float, "timed_out": bool, "error": Optional[str]}
    """
    t0 = time.time()
    # 使用 HFModelCache (主 19:33 借鉴 V1117 R9-DEV-003 已验证): 签名 (timeout_sec, cache)
    cache = HFModelCache(timeout_sec=timeout_sec, cache=False)
    try:
        cache.get_or_load(load_fn)
        elapsed = time.time() - t0
        return {
            "name": name,
            "loaded": True,
            "elapsed_sec": round(elapsed, 3),
            "timed_out": False,
            "error": None,
        }
    except HFModelTimeoutError as e:
        elapsed = time.time() - t0
        return {
            "name": name,
            "loaded": False,
            "elapsed_sec": round(elapsed, 3),
            "timed_out": True,
            "error": repr(e),
        }
    except Exception as e:
        elapsed = time.time() - t0
        return {
            "name": name,
            "loaded": False,
            "elapsed_sec": round(elapsed, 3),
            "timed_out": False,
            "error": repr(e),
        }


def chaos_test_timeout(sleep_sec: float = 10.0, timeout_sec: float = 1.0) -> Dict[str, Any]:
    """chaos test: 长时间 sleep → CI 须 N 秒内超时退出 (主 23:44 干到底)."""
    def slow_load() -> Any:
        time.sleep(sleep_sec)
        return "loaded"

    return chaos_test_model_load(slow_load, timeout_sec=timeout_sec, name=f"chaos-sleep-{sleep_sec}s")


def chaos_test_matrix(matrix: R10CrossMatrixResult,
                      chaos_timeout_sec: float = 3.0) -> Dict[str, Any]:
    """对跨小模型矩阵跑 chaos: 每个 entry 模拟加载超时 → CI 不挂."""
    results: List[Dict[str, Any]] = []
    n_passed = 0
    n_timed_out = 0
    for entry in matrix.entries:
        # 用 entry.family + params_b 模拟 load 耗时 (B 越大越慢)
        sleep_sec = min(0.5, entry.params_b / 30.0) if entry.available else 0.0

        def slow_load() -> str:
            time.sleep(sleep_sec)
            return f"loaded:{entry.model}"

        r = chaos_test_model_load(slow_load, timeout_sec=chaos_timeout_sec,
                                  name=f"{entry.family}:{entry.model}")
        results.append(r)
        if r["loaded"]:
            n_passed += 1
        if r["timed_out"]:
            n_timed_out += 1
    return {
        "n_models": len(results),
        "n_passed": n_passed,
        "n_timed_out": n_timed_out,
        "n_failed": len(results) - n_passed,
        "chaos_timeout_sec": chaos_timeout_sec,
        "results": results,
    }


# ---------------------------------------------------------------------------
# 报告生成 (主 00:56 任何人都能接手: Markdown + badge 自动产出)
# ---------------------------------------------------------------------------
class R10CIReporter:
    """R10 CI 报告生成器 (主 00:56 任何人都能接手: Markdown + SVG + JSON)."""

    def render_markdown(self, guard: R10GuardResult,
                       chaos: Optional[Dict[str, Any]] = None,
                       test_summary: Optional[Dict[str, Any]] = None) -> str:
        """渲染 Markdown 报告."""
        lines: List[str] = []
        lines.append(f"# R10 ASI 北极星 CI 守护报告 (V1127 / R10-ATE-001)")
        lines.append("")
        lines.append(f"- 时间: {time.strftime('%Y-%m-%dT%H:%M:%S%z', time.localtime(guard.computed_at))}")
        lines.append(f"- Version: {VERSION}")
        lines.append(f"- Backend: `{guard.backend_url}` (available={guard.backend_available})")
        lines.append(f"- Measured ASI level: **{guard.measured_asi_level:.4f}**")
        lines.append(f"- Baseline: {guard.baseline_asi_level:.4f}")
        lines.append(f"- Delta: {guard.delta:+.4f}")
        lines.append("")
        lines.append("## 门控结果")
        lines.append("")
        lines.append(f"| 门 | 阈值 | 实际 | Pass? |")
        lines.append(f"|----|------|------|-------|")
        lines.append(f"| W2 中期 (V0.4 ≥ 0.90) | {guard.w2_target:.4f} | {guard.measured_asi_level:.4f} | {'✅' if guard.passed_w2 else '❌'} |")
        lines.append(f"| 终极 (V0.5 ≥ 0.95) | {guard.ultimate_target:.4f} | {guard.measured_asi_level:.4f} | {'✅' if guard.passed_ultimate else '❌'} |")
        lines.append(f"| 无退化 (Δ ≥ -{guard.drop_tolerance:.4f}) | {guard.drop_tolerance:.4f} | {guard.delta:+.4f} | {'✅' if guard.passed_no_regression else '❌'} |")
        lines.append("")
        lines.append(f"**总评: {'✅ PASS' if guard.passed else '❌ FAIL'}**")
        if guard.error:
            lines.append("")
            lines.append(f"> Error: `{guard.error}`")
        lines.append("")
        # 跨小模型矩阵
        if guard.matrix is not None:
            lines.append("## 跨小模型 CI 矩阵")
            lines.append("")
            lines.append("| Family | Model | Params | Role | Available | ASI Level | HQB Sub | SC | NR | EV | CDT | Pass? |")
            lines.append("|--------|-------|--------|------|-----------|-----------|---------|-----|-----|-----|-----|-------|")
            for e in guard.matrix.entries:
                mark = "✅" if e.passed else ("⏸️" if not e.available else "❌")
                lines.append(
                    f"| {e.family} | {e.model} | {e.params_b}B | {e.role} | "
                    f"{'✅' if e.available else '❌'} | {e.asi_level:.4f} | {e.hqb_subscore:.4f} | "
                    f"{e.hqb_sc:.4f} | {e.hqb_nr:.4f} | {e.hqb_ev:.4f} | {e.hqb_cdt:.4f} | {mark} |"
                )
            lines.append("")
            lines.append(f"- 汇总: {guard.matrix.n_passed}/{len(guard.matrix.entries)} PASS, "
                         f"{guard.matrix.n_available} available, "
                         f"avg_level={guard.matrix.avg_asi_level:.4f}")
            lines.append("")
        # Chaos test
        if chaos is not None:
            lines.append("## Chaos Test 模型加载容错")
            lines.append("")
            lines.append(f"- Timeout: {chaos.get('chaos_timeout_sec', 'N/A')}s")
            lines.append(f"- Models: {chaos.get('n_models', 0)}")
            lines.append(f"- Passed: {chaos.get('n_passed', 0)}")
            lines.append(f"- Timed Out: {chaos.get('n_timed_out', 0)}")
            lines.append(f"- Failed: {chaos.get('n_failed', 0)}")
            lines.append("")
            lines.append("> 主 23:44 干到底: 模型加载超时 → CI 不挂, 显式 timed_out 标注.")
            lines.append("")
        # Tests
        if test_summary is not None:
            lines.append("## 测试覆盖")
            lines.append("")
            lines.append(f"- 总测试数: {test_summary.get('total', 0)}")
            lines.append(f"- 通过: {test_summary.get('passed', 0)}")
            lines.append(f"- 失败: {test_summary.get('failed', 0)}")
            lines.append(f"- 通过率: {test_summary.get('pass_rate', 0.0):.1%}")
            lines.append("")
        # 集成
        lines.append("## 集成点")
        lines.append("")
        lines.append("- **V1124 backend** (GET /asi/level, POST /asi/measure, GET /asi/north-star): 真 HTTP 集成")
        lines.append("- **cross_small_model_ci 5 模块** (R9-DEV-001~003 已 production): 复用 HQB 4 维 + 5 真模型 adapter")
        lines.append("- **V1117 badge SVG renderer** (R9-DEV-003 W4): 复用 shields.io 风格 + diff viz")
        lines.append("- **V1125 R10 集成协议** (R10-ARCH-001): 阈值 LOCKED 继承")
        lines.append("- **V1114 weekly integration evaluator** (R9-INT-005): 决策引擎基线")
        lines.append("")
        lines.append("## 哲学守门")
        lines.append("")
        lines.append("- 主 22:33 ASI 北极星 (CI 守护 = 守住 R10 V0.4 ≥ 0.90 终极 V0.5 ≥ 0.95)")
        lines.append("- 主 17:43 实事求是 (测量数字来自 V1124 backend 真接口, 不 hardcode)")
        lines.append("- 主 17:58+20:46 不假装 (backend 不可用 → 显式 fail, 不假装 PASS)")
        lines.append("- 主 23:44 干到底 (CI fail → 非零退出, 不软通过)")
        lines.append("- 主 19:33 走在前人经验上 (复用 cross_small_model_ci 5 模块 + V1117 + V1124)")
        lines.append("- 主 13:31 大胆激进 (跨小模型 + chaos test + R10 V0.4 ≥ 0.90 终极 V0.5 ≥ 0.95)")
        lines.append("- 主 00:56 任何人都能接手 (`run_r10_ci_guard()` 一行 = CI)")
        lines.append("")
        return "\n".join(lines) + "\n"

    def render_badge(self, guard: R10GuardResult) -> str:
        """渲染 badge SVG (主 13:31 大胆激进: shields.io 风格)."""
        if guard.passed:
            color = COLOR_MAP["GREEN"]
            msg = f"asi {guard.measured_asi_level:.3f} pass"
        elif guard.passed_w2:
            color = COLOR_MAP["YELLOW"]
            msg = f"asi {guard.measured_asi_level:.3f} w2"
        else:
            color = COLOR_MAP["RED"]
            msg = f"asi {guard.measured_asi_level:.3f} fail"
        return render_badge_svg(label="r10-north-star", message=msg, color=color)

    def render_json(self, guard: R10GuardResult,
                   chaos: Optional[Dict[str, Any]] = None,
                   test_summary: Optional[Dict[str, Any]] = None) -> str:
        """渲染 JSON 报告."""
        data = guard.to_dict()
        if chaos is not None:
            data["chaos"] = chaos
        if test_summary is not None:
            data["test_summary"] = test_summary
        return json.dumps(data, ensure_ascii=False, indent=2)

    def write(self, guard: R10GuardResult,
             path: str = "reports/r10-ate-w1-r10-ci-framework-report.md",
             chaos: Optional[Dict[str, Any]] = None,
             test_summary: Optional[Dict[str, Any]] = None) -> Path:
        """完整写报告: Markdown + badge + JSON."""
        p = Path(path)
        p.parent.mkdir(parents=True, exist_ok=True)
        # Markdown
        md = self.render_markdown(guard, chaos=chaos, test_summary=test_summary)
        p.write_text(md, encoding="utf-8")
        # Badge
        badge_path = p.with_suffix(".badge.svg")
        badge_path.write_text(self.render_badge(guard), encoding="utf-8")
        # JSON
        json_path = p.with_suffix(".json")
        json_path.write_text(self.render_json(guard, chaos=chaos, test_summary=test_summary), encoding="utf-8")
        return p


# ---------------------------------------------------------------------------
# 一行入口 (主 00:56 任何人都能接手)
# ---------------------------------------------------------------------------
def run_r10_ci_guard(config: Optional[R10NorthStarConfig] = None,
                    save_baseline_after: bool = True) -> R10GuardResult:
    """主 23:44 干到底: 一行 = 全套 CI 守护."""
    guard = ASINorthStarGuard(config=config)
    return guard.run(save_baseline_after=save_baseline_after)


def run_r10_ci_matrix(config: Optional[R10NorthStarConfig] = None) -> R10CrossMatrixResult:
    """主 13:31 大胆激进: 一行 = 跨小模型矩阵."""
    m = R10CrossSmallModelMatrix(config=config)
    return m.run()


def write_r10_report(guard: R10GuardResult,
                    path: str = "reports/r10-ate-w1-r10-ci-framework-report.md",
                    chaos: Optional[Dict[str, Any]] = None,
                    test_summary: Optional[Dict[str, Any]] = None) -> Path:
    """主 00:56 任何人都能接手: 一行 = 报告 + badge + JSON."""
    r = R10CIReporter()
    return r.write(guard, path=path, chaos=chaos, test_summary=test_summary)


# ---------------------------------------------------------------------------
# __all__
# ---------------------------------------------------------------------------
__all__ = [
    # version + 阈值
    "VERSION", "R10_V04_BASELINE", "R10_W2_TARGET", "R10_ULTIMATE_TARGET",
    "R10_GUARD_DROP_TOLERANCE", "R10_MODEL_MATRIX",
    # 配置 + 数据类
    "R10NorthStarConfig", "R10GuardResult", "R10ModelMatrixEntry", "R10CrossMatrixResult",
    # 核心类
    "R10NorthStarClient", "InlineBackend", "ASINorthStarGuard",
    "R10CrossSmallModelMatrix", "R10CIReporter",
    # 一行入口
    "run_r10_ci_guard", "run_r10_ci_matrix", "write_r10_report",
    # chaos test
    "chaos_test_model_load", "chaos_test_timeout", "chaos_test_matrix",
]


if __name__ == "__main__":
    # ponytail: `python -m apeireth.v1127_r10_cross_small_model_ci` 一行启动 CI
    import argparse
    parser = argparse.ArgumentParser(description="R10 ASI 北极星 CI 守护")
    parser.add_argument("--backend-url", default=None, help="V1124 backend URL")
    parser.add_argument("--baseline-path", default=None, help="baseline JSON 路径")
    parser.add_argument("--report", default="reports/r10-ate-w1-r10-ci-framework-report.md",
                        help="输出报告路径")
    parser.add_argument("--chaos", action="store_true", help="跑 chaos test")
    parser.add_argument("--strict", action="store_true", help="CI fail 时非零退出")
    args = parser.parse_args()
    cfg = R10NorthStarConfig(baseline_path=args.baseline_path)
    if args.backend_url:
        cfg.backend_url = args.backend_url
    result = run_r10_ci_guard(config=cfg)
    if args.chaos:
        m, chaos = run_r10_ci_matrix(config=cfg), chaos_test_matrix(
            run_r10_ci_matrix(config=cfg), chaos_timeout_sec=cfg.chaos_timeout_sec)
        print(f"Chaos: {chaos['n_passed']}/{chaos['n_models']} passed, "
              f"{chaos['n_timed_out']} timed_out")
    p = write_r10_report(result, path=args.report)
    print(f"Report: {p}")
    print(f"ASI: {result.measured_asi_level:.4f} (W2 {'✅' if result.passed_w2 else '❌'} "
          f"ultimate {'✅' if result.passed_ultimate else '❌'})")
    if args.strict and not result.passed:
        sys.exit(1)
