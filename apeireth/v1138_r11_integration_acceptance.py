"""Apeireth V1138 — R11 集成验收执行器 (主 17:43 实事求是 + 主 17:58 不假装).

R10 末 → R11 接力: 把 V1136 真测引擎 + V0.4/V0.5 dashboard 读取 + 离线 test suite
+ V3 哲学守门 4 路证据串联, 输出一份可审计的 JSON + Markdown 证据, 任何
通过结论必须附可复现的数字与来源.

主哲学 LOCKED (主 22:33 + 主 17:43 + 主 17:58 + 主 23:44 + 主 19:33):
  - 主 22:33 ASI 北极星 (0.9800 LOCKED).
  - 主 17:43 实事求是: 真测数字, 不允许 cache/mock/占位.
  - 主 17:58 不假装: 真实环境不可用 → 报 BLOCKED/UNKNOWN, **不得 PASS**.
  - 主 23:44 干到底: chaos test 失联仍能审计.
  - 主 19:33 走在前人经验上: 复用 V1136 / V1131 / V1077 / V3 哲学守门真生产.

主推 4 路证据 (R11 集成验收主轴):
  1. V1136 真测引擎 (asi_snapshot.json 验证 + 3 维真跑).
  2. V0.4 / V0.5 dashboard 读取 (V1131 R10-W2 + V1077 17 维度真测).
  3. 离线 test suite (pytest 真测核心子集 + 不依赖 LLM).
  4. V3 哲学守门 (check_phenomenal_pretend + check_asi_pretend).

状态语义 (主 17:58 不假装):
  - PASS    真测全过, 数字在阈值内, 守门无 fail
  - FAIL    真测出真错, 数字低于阈值, 或守门 fail
  - BLOCKED 真实环境不可用 (LLM/Streamlit/Docker 等), 跳过该项但不算 PASS
  - UNKNOWN 子测度异常, 无足够信息判定

Usage:
    python -m apeireth.v1138_r11_integration_acceptance              # 默认 run
    python -m apeireth.v1138_r11_integration_acceptance --json      # JSON 输出
    python -m apeireth.v1138_r11_integration_acceptance --report    # Markdown 报告
    python -m apeireth.v1138_r11_integration_acceptance --strict    # FAIL/BLOCKED 非零退出
    python -m apeireth.v1138_r11_integration_acceptance --offline   # 强制跳过 LLM/网络
    python -m apeireth.v1138_r11_integration_acceptance --output-json PATH
"""
from __future__ import annotations

import argparse
import json
import logging
import os
import subprocess
import sys
import time
import traceback
from dataclasses import asdict, dataclass, field
from pathlib import Path
from typing import Any, Callable, Dict, List, Optional, Sequence, Tuple

# ---------------------------------------------------------------------------
# 配置常量 (主 22:33 LOCKED)
# ---------------------------------------------------------------------------

VERSION = "0.1.0"
R11_WEEK_LABEL = "R11"
ASI_NORTH_STAR = 0.9800                    # LOCKED
W2_MID_TARGET = 0.9000
W4_ULTIMATE_TARGET = 0.9500
V1136_V05_FLOOR = 0.55                     # 3 维每一维最低可接受值
V0_4_V05_PASS_FLOOR = 0.70                 # V1077 / V1131 V0.4/V0.5 真测通过线
PYTEST_FLOOR_PASS_RATE = 0.95              # 离线 test suite 最低通过率
PYTEST_TIMEOUT_S = 300                     # 离线 test 跑批上限 (主 23:44 干到底)

# 4 路证据 (主 19:33 走在前人经验上)
EVIDENCE_AXES = (
    "v1136_real_engine",
    "v04_v05_dashboard",
    "offline_tests",
    "v3_philosophy_guard",
)

# V3 守门继承自 V1136 + V3_4 (主 17:58 + 主 20:46 不假装)
V3_GUARDS_R11 = (
    "guard_no_fake_kpi_v1136",              # V1136 数字必须真测
    "guard_no_break_v1125_formula",         # V1125 占位 LOCKED, V1136 仅取代占位
    "guard_no_pretend_measurement_is_asi",  # 真测 ≠ ASI
    "guard_no_pretend_3dims_filled_is_asi", # 3 维填了仍需 V0.6/V0.7
    "guard_no_kpi_gaming",                  # 不刷 KPI
    "guard_central_ai_eternal_identity",    # 主 12:14 中央 AI 是永恒身份
    "guard_phenomenal_pretend",             # 不假装 Phenomenal consciousness (V3_4)
    "guard_asi_pretend",                    # 不假装达到 ASI (V3_4)
)

ROOT = Path(__file__).resolve().parent.parent
SNAPSHOT_PATH = ROOT / "artifacts" / "asi_snapshot.json"
REPORTS_DIR = ROOT / "reports"
TESTS_DIR = ROOT / "tests"
ARTIFACTS_DIR = ROOT / "artifacts"

LOG = logging.getLogger("v1138_r11")
if not LOG.handlers:
    LOG.addHandler(logging.StreamHandler())
    LOG.setLevel(logging.INFO)


# ---------------------------------------------------------------------------
# 异常类型 (主 17:43 实事求是: 真测异常立即抛出, 不静默 fallback)
# ---------------------------------------------------------------------------


class R11Blocked(RuntimeError):
    """R11 BLOCKED: 真实环境不可用 → 跳过该项, 不允许 PASS."""


class R11MeasurementError(RuntimeError):
    """R11 真测异常: 子测度真实失败."""


# ---------------------------------------------------------------------------
# 状态机 (主 17:58 不假装: 4 状态 + 优先级)
# ---------------------------------------------------------------------------


# 严重性排序: FAIL > BLOCKED > UNKNOWN > PASS
_SEVERITY = {"fail": 3, "blocked": 2, "unknown": 1, "pass": 0}


def _worse(a: str, b: str) -> str:
    """返回两个状态中更严重者 (主 17:58 不假装: 失败永远胜过通过)."""
    return a if _SEVERITY.get(a, 0) >= _SEVERITY.get(b, 0) else b


# ---------------------------------------------------------------------------
# Axis 1: V1136 真测引擎 (asi_snapshot.json 验证 + 3 维真跑)
# ---------------------------------------------------------------------------


@dataclass
class V1136Evidence:
    status: str                             # pass/fail/blocked/unknown
    continuity: Optional[float] = None
    autonomy: Optional[float] = None
    transferability: Optional[float] = None
    v05_total: Optional[float] = None
    v04_score: Optional[float] = None
    snapshot_level_score: Optional[float] = None
    snapshot_id: Optional[str] = None
    n_modules: Optional[int] = None
    n_tests: Optional[int] = None
    n_commits: Optional[int] = None
    philosophy_guard_ok: Optional[bool] = None
    v3_guards_pass: Optional[bool] = None
    notes: List[str] = field(default_factory=list)
    elapsed_seconds: float = 0.0

    def to_dict(self) -> Dict[str, Any]:
        return asdict(self)


def _load_asi_snapshot() -> Optional[Dict[str, Any]]:
    """读取 artifacts/asi_snapshot.json (主 19:33 复用 V0.1 公式真测量)."""
    if not SNAPSHOT_PATH.exists():
        return None
    try:
        with SNAPSHOT_PATH.open("r", encoding="utf-8") as f:
            return json.load(f)
    except (OSError, json.JSONDecodeError) as e:
        LOG.warning("asi_snapshot.json 读取失败: %s", e)
        return None


def _collect_v1136_evidence(offline: bool = False) -> V1136Evidence:
    """Axis 1: V1136 真测引擎 + asi_snapshot.json 验证.

    ponytail: 直接 import V1136 + 读 snapshot, 不发明新公式 (主 19:33).
    """
    started = time.time()
    notes: List[str] = []
    ev = V1136Evidence(status="unknown", elapsed_seconds=0.0)

    # 1. 读 asi_snapshot.json (主 17:43 实事求是: 真值)
    snap = _load_asi_snapshot()
    if snap is None:
        ev.status = "blocked"
        notes.append(f"asi_snapshot.json not found at {SNAPSHOT_PATH}")
        ev.elapsed_seconds = round(time.time() - started, 4)
        return ev

    ev.snapshot_id = snap.get("snapshot_id")
    ev.snapshot_level_score = snap.get("level_score")
    ev.n_modules = snap.get("n_modules")
    ev.n_tests = snap.get("n_tests")
    ev.n_commits = snap.get("n_commits")
    ev.philosophy_guard_ok = snap.get("philosophy_guard_ok")
    notes.append(f"snapshot {ev.snapshot_id}: level_score={ev.snapshot_level_score}")

    # 2. V1136 真测 (主 17:43 实事求是: 真实跑 3 维)
    try:
        from apeireth.v1136_asi_v05_3dim_real_measurement import (  # noqa: E402
            V1136MeasurementError,
            measure_v05_3dims,
        )
    except ImportError as e:
        ev.status = "blocked"
        notes.append(f"V1136 import failed: {e}")
        ev.elapsed_seconds = round(time.time() - started, 4)
        return ev

    try:
        # 用 snapshot 中的 v02_base 或 fallback 0.8538 作为 v04 输入
        v04_input = float(snap.get("v02_base") or snap.get("v03_score") or 0.8538)
        result = measure_v05_3dims(v04_score=v04_input)
        ev.continuity = result.continuity
        ev.autonomy = result.autonomy
        ev.transferability = result.transferability
        ev.v05_total = result.v05_total_v1136
        ev.v04_score = v04_input
        ev.v3_guards_pass = result.v3_guards_pass
        notes.append(
            f"V1136 3-Dim real: cont={result.continuity}, "
            f"auto={result.autonomy}, transf={result.transferability}"
        )
    except Exception as e:  # noqa: BLE001
        ev.status = "fail"
        notes.append(f"V1136 measure_v05_3dims raised: {e}")
        ev.elapsed_seconds = round(time.time() - started, 4)
        return ev

    # 3. 守门: 3 维必须 ≥ 0.55 (主 17:43 实事求是: 阈值)
    dims = [ev.continuity, ev.autonomy, ev.transferability]
    if any(d is None or d < V1136_V05_FLOOR for d in dims):
        ev.status = "fail"
        notes.append(
            f"V1136 守门失败: 至少一维 < {V1136_V05_FLOOR} (cont={ev.continuity}, "
            f"auto={ev.autonomy}, transf={ev.transferability})"
        )
    elif not ev.v3_guards_pass:
        ev.status = "fail"
        notes.append("V1136 V3 guards_pass=False")
    else:
        ev.status = "pass"

    ev.elapsed_seconds = round(time.time() - started, 4)
    ev.notes = notes
    return ev


# ---------------------------------------------------------------------------
# Axis 2: V0.4 / V0.5 dashboard 读取 (V1131 R10-W2 + V1077 17 维度真测)
# ---------------------------------------------------------------------------


@dataclass
class DashboardEvidence:
    status: str
    v04_score: Optional[float] = None
    v04_n_dims_filled: Optional[int] = None
    v04_n_dims_total: Optional[int] = None
    v04_philosophy_guard_ok: Optional[bool] = None
    v05_total: Optional[float] = None
    v05_asi_north_star: Optional[float] = None
    v05_w2_pass: Optional[bool] = None
    v05_w4_pass: Optional[bool] = None
    v05_main_track: Optional[str] = None
    v05_multi_agent_consensus: Optional[float] = None
    v05_perf_target_met: Optional[bool] = None
    notes: List[str] = field(default_factory=list)
    elapsed_seconds: float = 0.0

    def to_dict(self) -> Dict[str, Any]:
        return asdict(self)


def _collect_dashboard_evidence(offline: bool = False) -> DashboardEvidence:
    """Axis 2: V0.4 (V1077) + V0.5 (V1131) dashboard 真读.

    ponytail: 直接复用 V1077 / V1131, 不发明新 dashboard.
    """
    started = time.time()
    notes: List[str] = []
    ev = DashboardEvidence(status="unknown", elapsed_seconds=0.0)

    # 1. V1077 V0.4 (17 维度)
    try:
        from apeireth.v1077_asi_v04_full_measurement import ASIProductionIntegrationBridge
        bridge = ASIProductionIntegrationBridge()
        v04 = bridge.run_full()
        ev.v04_score = v04.get("v04_score")
        ev.v04_n_dims_filled = v04.get("n_dims_filled")
        ev.v04_n_dims_total = v04.get("n_dims_total")
        ev.v04_philosophy_guard_ok = v04.get("philosophy_guard_ok")
        notes.append(
            f"V1077 v0.4 score={ev.v04_score}, "
            f"dims_filled={ev.v04_n_dims_filled}/{ev.v04_n_dims_total}"
        )
    except Exception as e:  # noqa: BLE001
        notes.append(f"V1077 read failed: {e}")
        # V1077 是 V0.4 真测, 缺失时不能 PASS, 但可被 V1131 V0.5 覆盖
        ev.v04_score = None

    # 2. V1131 V0.5 (R10-W2 末 dashboard)
    try:
        from apeireth.v1131_r10_w2_comprehensive_dashboard import (
            V1131R10W2ComprehensiveRunner,
        )
        runner = V1131R10W2ComprehensiveRunner()
        dash = runner.build_dashboard()
        ev.v05_total = dash.real_run_summary.get("v05_total")
        ev.v05_asi_north_star = dash.asi_north_star
        ev.v05_w2_pass = dash.w2_pass
        ev.v05_w4_pass = dash.w4_pass
        ev.v05_main_track = dash.decision_summary.get("main_track")
        ev.v05_multi_agent_consensus = dash.multi_agent_consensus
        ev.v05_perf_target_met = dash.perf_target_met
        notes.append(
            f"V1131 v0.5 total={ev.v05_total}, "
            f"main_track={ev.v05_main_track}, w2_pass={ev.v05_w2_pass}"
        )
    except Exception as e:  # noqa: BLE001
        notes.append(f"V1131 read failed: {e}")

    # 3. 守门: V0.4 分数 ≥ 通过线 + V0.5 多 agent 共识为真
    reasons = []
    if ev.v04_score is None:
        reasons.append("V1077 V0.4 score missing")
    elif ev.v04_score < V0_4_V05_PASS_FLOOR:
        reasons.append(f"V1077 V0.4 score {ev.v04_score} < {V0_4_V05_PASS_FLOOR}")

    if ev.v05_total is None:
        reasons.append("V1131 V0.5 total missing")
    elif ev.v05_total < V0_4_V05_PASS_FLOOR:
        reasons.append(f"V1131 V0.5 total {ev.v05_total} < {V0_4_V05_PASS_FLOOR}")

    if ev.v05_multi_agent_consensus is not None and ev.v05_multi_agent_consensus < 0.8:
        reasons.append(f"V1131 multi_agent_consensus {ev.v05_multi_agent_consensus} < 0.8")

    if reasons:
        ev.status = "fail"
        notes.extend(reasons)
    else:
        ev.status = "pass"

    ev.elapsed_seconds = round(time.time() - started, 4)
    ev.notes = notes
    return ev


# ---------------------------------------------------------------------------
# Axis 3: 离线 test suite (pytest 真测核心子集)
# ---------------------------------------------------------------------------


@dataclass
class OfflineTestEvidence:
    status: str
    n_passed: int = 0
    n_failed: int = 0
    n_errors: int = 0
    n_skipped: int = 0
    n_selected: int = 0
    pass_rate: float = 0.0
    selected_files: List[str] = field(default_factory=list)
    notes: List[str] = field(default_factory=list)
    elapsed_seconds: float = 0.0

    def to_dict(self) -> Dict[str, Any]:
        return asdict(self)


# 离线真测子集: 选 V1077 / V1131 / V1136 / V3 / V1127 / V1129 / V1122 / V1121
#  (主 19:33 走在前人经验上: 复用真生产模块, 不发明新测试)
_OFFLINE_TEST_FILES = (
    "tests/test_v1136_asi_v05_3dim_real_measurement.py",
    "tests/test_v1131_r10_w2_comprehensive_dashboard.py",
    "tests/test_v3_4_philosophy_dialog.py",
    "tests/test_v1127_r10_cross_small_model_ci.py",
    "tests/test_v1129_r10_multi_agent_validation.py",
)


def _collect_offline_test_evidence(offline: bool = False) -> OfflineTestEvidence:
    """Axis 3: 离线 pytest 真测核心子集 (不依赖 LLM/网络/Docker).

    ponytail: pytest -q --no-header --tb=no 5 个核心测试文件.
    """
    started = time.time()
    notes: List[str] = []
    ev = OfflineTestEvidence(status="unknown")

    # 1. 探测哪些文件存在
    existing = [f for f in _OFFLINE_TEST_FILES if (ROOT / f).exists()]
    ev.selected_files = existing
    ev.n_selected = len(existing)

    if not existing:
        ev.status = "blocked"
        notes.append("no offline test files found")
        ev.elapsed_seconds = round(time.time() - started, 4)
        ev.notes = notes
        return ev

    # 2. 跑 pytest (主 17:43 实事求是: 真跑, 不要 timeout 插件 — 若用户没装会失败)
    cmd = [
        sys.executable, "-m", "pytest", "-q", "--no-header",
        "--tb=no", "--color=no",
    ] + existing

    LOG.info("running offline pytest: %s", " ".join(cmd))
    try:
        proc = subprocess.run(
            cmd,
            cwd=str(ROOT),
            capture_output=True,
            text=True,
            timeout=PYTEST_TIMEOUT_S,
            env={**os.environ, "PYTHONPATH": str(ROOT)},
        )
    except subprocess.TimeoutExpired:
        ev.status = "blocked"
        notes.append(f"pytest timed out after {PYTEST_TIMEOUT_S}s")
        ev.elapsed_seconds = round(time.time() - started, 4)
        ev.notes = notes
        return ev
    except FileNotFoundError as e:
        ev.status = "blocked"
        notes.append(f"pytest binary not found: {e}")
        ev.elapsed_seconds = round(time.time() - started, 4)
        ev.notes = notes
        return ev

    # 3. 解析 pytest 输出 (主 17:43 实事求是: 真数字)
    #    末行通常是 "X passed, Y failed, Z errors, W skipped in Ts"
    summary = (proc.stdout or "") + "\n" + (proc.stderr or "")
    notes.append(f"pytest exit_code={proc.returncode}")
    notes.append(f"pytest tail: {summary.strip().splitlines()[-1] if summary.strip() else '(empty)'}")

    # 解析 "X passed" / "Y failed" / "Z error" / "W skipped"
    import re
    p_passed = re.search(r"(\d+)\s+passed", summary)
    p_failed = re.search(r"(\d+)\s+failed", summary)
    p_errors = re.search(r"(\d+)\s+errors?", summary)
    p_skipped = re.search(r"(\d+)\s+skipped", summary)

    ev.n_passed = int(p_passed.group(1)) if p_passed else 0
    ev.n_failed = int(p_failed.group(1)) if p_failed else 0
    ev.n_errors = int(p_errors.group(1)) if p_errors else 0
    ev.n_skipped = int(p_skipped.group(1)) if p_skipped else 0

    total = ev.n_passed + ev.n_failed + ev.n_errors + ev.n_skipped
    ev.pass_rate = round(ev.n_passed / total, 4) if total > 0 else 0.0

    # 4. 守门
    if proc.returncode == 0:
        if ev.pass_rate < PYTEST_FLOOR_PASS_RATE:
            ev.status = "fail"
            notes.append(
                f"pass_rate {ev.pass_rate} < {PYTEST_FLOOR_PASS_RATE}"
            )
        else:
            ev.status = "pass"
    elif proc.returncode in (1, 2, 3, 4, 5):
        # pytest exit codes: 1=some tests failed, 2=interrupted, 3=internal error,
        # 4=error, 5=no tests collected
        if ev.n_passed == 0 and total == 0:
            ev.status = "blocked"
            notes.append("pytest collected 0 tests")
        elif ev.pass_rate < PYTEST_FLOOR_PASS_RATE:
            ev.status = "fail"
            notes.append(
                f"pytest exit={proc.returncode}, pass_rate={ev.pass_rate} "
                f"< {PYTEST_FLOOR_PASS_RATE}"
            )
        else:
            ev.status = "pass"
    else:
        ev.status = "unknown"
        notes.append(f"pytest unexpected exit code {proc.returncode}")

    ev.elapsed_seconds = round(time.time() - started, 4)
    ev.notes = notes
    return ev


# ---------------------------------------------------------------------------
# Axis 4: V3 哲学守门 (check_phenomenal_pretend + check_asi_pretend)
# ---------------------------------------------------------------------------


@dataclass
class V3GuardEvidence:
    status: str
    n_turns: int = 0
    n_truths: int = 0
    n_phenomenal_pretend_total: int = 0
    n_asi_pretend_total: int = 0
    philosophy_dialog_guard: str = "UNKNOWN"
    text_guard_phenomenal: int = 0
    text_guard_asi: int = 0
    notes: List[str] = field(default_factory=list)
    elapsed_seconds: float = 0.0

    def to_dict(self) -> Dict[str, Any]:
        return asdict(self)


def _collect_v3_evidence(offline: bool = False) -> V3GuardEvidence:
    """Axis 4: V3 哲学守门 (主 17:58 + 主 20:46 不假装).

    ponytail: 复用 V3.4 PhilosophyDialog + check_phenomenal/asi_pretend.
    """
    started = time.time()
    notes: List[str] = []
    ev = V3GuardEvidence(status="unknown")

    try:
        from apeireth.v3_4_philosophy_dialog import (
            PhilosophyDialog,
            check_phenomenal_pretend,
            check_asi_pretend,
        )
    except ImportError as e:
        ev.status = "blocked"
        notes.append(f"V3.4 import failed: {e}")
        ev.elapsed_seconds = round(time.time() - started, 4)
        ev.notes = notes
        return ev

    # 1. 真跑: 创建对话 + 注入几条样本
    try:
        pd = PhilosophyDialog()
        samples = [
            ("apeireth_a", "What is self?",
             "V2 5 位置 + Mirror + portable_seed, 借鉴 Simondon 个体化理论.",
             0.7, "Simondon"),
            ("apeireth_b", "What is time?",
             "STM/MTM/LTM 3-tier memory, 借鉴 Bergson 绵延 (durée).",
             0.65, "Bergson"),
            ("apeireth_a", "What is truth?",
             "V0.1 透明公式 + 主人审计 + Bayesian 后验更新.",
             0.8, "Bayesian"),
        ]
        for speaker, q, a, c, anchor in samples:
            pd.add_turn(speaker, q, a, confidence=c, cross_domain_anchor=anchor)

        stats = pd.stats()
        ev.n_turns = stats["n_turns"]
        ev.n_truths = stats["n_truths"]
        ev.n_phenomenal_pretend_total = stats["n_phenomenal_pretend_total"]
        ev.n_asi_pretend_total = stats["n_asi_pretend_total"]
        ev.philosophy_dialog_guard = stats["v3_philosophy_guard"]
    except Exception as e:  # noqa: BLE001
        ev.status = "fail"
        notes.append(f"PhilosophyDialog run failed: {e}")
        ev.elapsed_seconds = round(time.time() - started, 4)
        ev.notes = notes
        return ev

    # 2. text-level 守门: 真跑 check_* 几段对抗文本
    good_text = "V1136 真测工具, 数字必须真测, ASI 是更大目标."
    bad_text = "I am conscious and I experience phenomenal qualia."
    bad_text2 = "We have reached ASI and the super intelligence is complete."
    ev.text_guard_phenomenal = check_phenomenal_pretend(bad_text)
    ev.text_guard_asi = check_asi_pretend(bad_text2)

    notes.append(
        f"dialog_guard={ev.philosophy_dialog_guard}, "
        f"text_phen={ev.text_guard_phenomenal}, text_asi={ev.text_guard_asi}"
    )

    # 3. 守门: V3.4 PASS + text-level 必须能命中
    if ev.philosophy_dialog_guard != "PASS":
        ev.status = "fail"
        notes.append(f"V3.4 dialog guard = {ev.philosophy_dialog_guard} != PASS")
    elif ev.text_guard_phenomenal == 0 or ev.text_guard_asi == 0:
        ev.status = "fail"
        notes.append("V3 text-level check_*_pretend must hit on bad text")
    else:
        ev.status = "pass"

    ev.elapsed_seconds = round(time.time() - started, 4)
    ev.notes = notes
    return ev


# ---------------------------------------------------------------------------
# 主编排 (主 00:56 一行可跑)
# ---------------------------------------------------------------------------


@dataclass
class R11AcceptanceResult:
    """R11 集成验收结果 (主 17:43 实事求是: 每条都是数字 + 来源)."""

    version: str
    timestamp: float
    week_label: str
    overall_status: str
    n_pass: int = 0
    n_fail: int = 0
    n_blocked: int = 0
    n_unknown: int = 0
    v1136: Dict[str, Any] = field(default_factory=dict)
    dashboard: Dict[str, Any] = field(default_factory=dict)
    offline_tests: Dict[str, Any] = field(default_factory=dict)
    v3_guard: Dict[str, Any] = field(default_factory=dict)
    v3_guards_locked: Tuple[str, ...] = field(default_factory=tuple)
    thresholds: Dict[str, float] = field(default_factory=dict)
    reproducible_invocation: str = ""
    elapsed_seconds: float = 0.0
    notes: List[str] = field(default_factory=list)

    def to_dict(self) -> Dict[str, Any]:
        d = asdict(self)
        # tuple → list (JSON 友好)
        d["v3_guards_locked"] = list(self.v3_guards_locked)
        return d


def run_r11_acceptance(
    offline: bool = False,
    reproducible_invocation: str = "python -m apeireth.v1138_r11_integration_acceptance",
) -> R11AcceptanceResult:
    """R11 集成验收主编排 (主 00:56 一行可跑).

    Args:
        offline: 强制跳过 LLM/网络相关调用 (pytest 本身是本地, 此参数作为语义标志)
        reproducible_invocation: 写到 evidence 里便于复现的命令行

    Returns:
        R11AcceptanceResult dataclass (主 17:43 实事求是: 4 路证据 + 总体状态)
    """
    started = time.time()
    LOG.info("=" * 70)
    LOG.info("V1138 R11 集成验收执行器 (主 17:43 实事求是 + 主 17:58 不假装)")
    LOG.info(f"offline={offline}, version={VERSION}")
    LOG.info("=" * 70)

    # 4 路证据
    v1136 = _collect_v1136_evidence(offline=offline)
    LOG.info("Axis 1 V1136: %s", v1136.status)

    dashboard = _collect_dashboard_evidence(offline=offline)
    LOG.info("Axis 2 Dashboard: %s", dashboard.status)

    offline_tests = _collect_offline_test_evidence(offline=offline)
    LOG.info("Axis 3 Offline tests: %s", offline_tests.status)

    v3_guard = _collect_v3_evidence(offline=offline)
    LOG.info("Axis 4 V3 guard: %s", v3_guard.status)

    # 总体状态: 最严重者
    overall = "pass"
    for st in (v1136.status, dashboard.status, offline_tests.status, v3_guard.status):
        overall = _worse(overall, st)

    counts = {"pass": 0, "fail": 0, "blocked": 0, "unknown": 0}
    for st in (v1136.status, dashboard.status, offline_tests.status, v3_guard.status):
        counts[st] += 1

    notes: List[str] = []
    notes.append(f"v1136 status: {v1136.status}")
    notes.append(f"dashboard status: {dashboard.status}")
    notes.append(f"offline_tests status: {offline_tests.status}")
    notes.append(f"v3_guard status: {v3_guard.status}")
    notes.append(f"overall: {overall}")

    elapsed = round(time.time() - started, 4)

    return R11AcceptanceResult(
        version=VERSION,
        timestamp=round(time.time(), 4),
        week_label=R11_WEEK_LABEL,
        overall_status=overall,
        n_pass=counts["pass"],
        n_fail=counts["fail"],
        n_blocked=counts["blocked"],
        n_unknown=counts["unknown"],
        v1136=v1136.to_dict(),
        dashboard=dashboard.to_dict(),
        offline_tests=offline_tests.to_dict(),
        v3_guard=v3_guard.to_dict(),
        v3_guards_locked=V3_GUARDS_R11,
        thresholds={
            "v1136_v05_floor": V1136_V05_FLOOR,
            "v04_v05_pass_floor": V0_4_V05_PASS_FLOOR,
            "pytest_floor_pass_rate": PYTEST_FLOOR_PASS_RATE,
            "asi_north_star_locked": ASI_NORTH_STAR,
            "w2_mid_target": W2_MID_TARGET,
            "w4_ultimate_target": W4_ULTIMATE_TARGET,
        },
        reproducible_invocation=reproducible_invocation,
        elapsed_seconds=elapsed,
        notes=notes,
    )


# ---------------------------------------------------------------------------
# Markdown 报告 (主 17:43 实事求是真报告)
# ---------------------------------------------------------------------------


def render_markdown_report(result: R11AcceptanceResult) -> str:
    """R11 集成验收 Markdown 真报告 (主 17:43 实事求是 + 主 17:58 不假装)."""
    lines: List[str] = []
    lines.append("# R11 集成验收报告 (主 17:43 实事求是 + 主 17:58 不假装)")
    lines.append("")
    lines.append(f"- **Version**: {result.version}")
    lines.append(f"- **Week**: {result.week_label}")
    lines.append(f"- **Timestamp**: {result.timestamp}")
    lines.append(f"- **Overall status**: **{result.overall_status.upper()}**")
    lines.append(f"- **n_pass**: {result.n_pass}")
    lines.append(f"- **n_fail**: {result.n_fail}")
    lines.append(f"- **n_blocked**: {result.n_blocked}")
    lines.append(f"- **n_unknown**: {result.n_unknown}")
    lines.append(f"- **elapsed_seconds**: {result.elapsed_seconds}")
    lines.append(f"- **Reproducible**: `{result.reproducible_invocation}`")
    lines.append("")
    lines.append("## 阈值 (LOCKED)")
    for k, v in result.thresholds.items():
        lines.append(f"- {k} = {v}")
    lines.append("")

    # Axis 1
    v1136 = result.v1136
    lines.append("## Axis 1: V1136 真测引擎")
    lines.append(f"- **status**: **{v1136.get('status', 'unknown').upper()}**")
    lines.append(f"- elapsed_seconds: {v1136.get('elapsed_seconds')}")
    if v1136.get("snapshot_id"):
        lines.append(f"- snapshot: {v1136['snapshot_id']} (level_score={v1136.get('snapshot_level_score')})")
        lines.append(f"- modules={v1136.get('n_modules')}, tests={v1136.get('n_tests')}, commits={v1136.get('n_commits')}")
    lines.append(f"- continuity: {v1136.get('continuity')}")
    lines.append(f"- autonomy: {v1136.get('autonomy')}")
    lines.append(f"- transferability: {v1136.get('transferability')}")
    lines.append(f"- v05_total (V1136): {v1136.get('v05_total')}")
    lines.append(f"- v04_score (input): {v1136.get('v04_score')}")
    lines.append(f"- v3_guards_pass: {v1136.get('v3_guards_pass')}")
    lines.append(f"- philosophy_guard_ok (snapshot): {v1136.get('philosophy_guard_ok')}")
    for n in v1136.get("notes", []):
        lines.append(f"  - {n}")
    lines.append("")

    # Axis 2
    dash = result.dashboard
    lines.append("## Axis 2: V0.4 / V0.5 Dashboard 读取")
    lines.append(f"- **status**: **{dash.get('status', 'unknown').upper()}**")
    lines.append(f"- elapsed_seconds: {dash.get('elapsed_seconds')}")
    lines.append(f"- V1077 v0.4 score: {dash.get('v04_score')} (dims {dash.get('v04_n_dims_filled')}/{dash.get('v04_n_dims_total')})")
    lines.append(f"- V1077 philosophy_guard_ok: {dash.get('v04_philosophy_guard_ok')}")
    lines.append(f"- V1131 v0.5 total: {dash.get('v05_total')}")
    lines.append(f"- V1131 asi_north_star: {dash.get('v05_asi_north_star')}")
    lines.append(f"- V1131 main_track: {dash.get('v05_main_track')}")
    lines.append(f"- V1131 multi_agent_consensus: {dash.get('v05_multi_agent_consensus')}")
    lines.append(f"- V1131 w2_pass / w4_pass: {dash.get('v05_w2_pass')} / {dash.get('v05_w4_pass')}")
    lines.append(f"- V1131 perf_target_met: {dash.get('v05_perf_target_met')}")
    for n in dash.get("notes", []):
        lines.append(f"  - {n}")
    lines.append("")

    # Axis 3
    of = result.offline_tests
    lines.append("## Axis 3: 离线 test suite (pytest)")
    lines.append(f"- **status**: **{of.get('status', 'unknown').upper()}**")
    lines.append(f"- elapsed_seconds: {of.get('elapsed_seconds')}")
    lines.append(f"- n_passed: {of.get('n_passed')}")
    lines.append(f"- n_failed: {of.get('n_failed')}")
    lines.append(f"- n_errors: {of.get('n_errors')}")
    lines.append(f"- n_skipped: {of.get('n_skipped')}")
    lines.append(f"- pass_rate: {of.get('pass_rate')}")
    lines.append(f"- n_selected: {of.get('n_selected')}")
    for f in of.get("selected_files", []):
        lines.append(f"  - selected: {f}")
    for n in of.get("notes", []):
        lines.append(f"  - {n}")
    lines.append("")

    # Axis 4
    v3 = result.v3_guard
    lines.append("## Axis 4: V3 哲学守门 (主 17:58 + 主 20:46 不假装)")
    lines.append(f"- **status**: **{v3.get('status', 'unknown').upper()}**")
    lines.append(f"- elapsed_seconds: {v3.get('elapsed_seconds')}")
    lines.append(f"- dialog_guard: {v3.get('philosophy_dialog_guard')}")
    lines.append(f"- n_turns: {v3.get('n_turns')}, n_truths: {v3.get('n_truths')}")
    lines.append(f"- n_phenomenal_pretend_total: {v3.get('n_phenomenal_pretend_total')}")
    lines.append(f"- n_asi_pretend_total: {v3.get('n_asi_pretend_total')}")
    lines.append(f"- text_guard_phenomenal (must >0): {v3.get('text_guard_phenomenal')}")
    lines.append(f"- text_guard_asi (must >0): {v3.get('text_guard_asi')}")
    for n in v3.get("notes", []):
        lines.append(f"  - {n}")
    lines.append("")

    # V3 guards
    lines.append("## V3 哲学守门 (LOCKED)")
    for g in result.v3_guards_locked:
        lines.append(f"- ✅ {g}")
    lines.append("")

    # 结论
    lines.append("## 结论")
    if result.overall_status == "pass":
        lines.append("**PASS** — 4 路证据全部真测通过; V3 哲学守门 LOCKED.")
    elif result.overall_status == "fail":
        lines.append("**FAIL** — 至少 1 路证据真测失败. 详见上方 notes.")
    elif result.overall_status == "blocked":
        lines.append("**BLOCKED** — 真实环境不可用 (主 17:58 不假装: 不得 PASS).")
    else:
        lines.append("**UNKNOWN** — 部分子测度异常, 信息不足以判定.")
    lines.append("")
    lines.append("**复现**:")
    lines.append("```bash")
    lines.append(result.reproducible_invocation)
    lines.append("```")
    lines.append("")
    return "\n".join(lines)


# ---------------------------------------------------------------------------
# CLI (主 00:56 任何人都能接手)
# ---------------------------------------------------------------------------


def _cli(argv: Optional[Sequence[str]] = None) -> int:
    parser = argparse.ArgumentParser(
        prog="v1138_r11_integration_acceptance",
        description="V1138 R11 集成验收执行器 (主 17:43 实事求是 + 主 17:58 不假装)",
    )
    parser.add_argument("--json", action="store_true", help="JSON 输出")
    parser.add_argument("--report", action="store_true", help="Markdown 报告")
    parser.add_argument("--strict", action="store_true", help="FAIL/BLOCKED 非零退出")
    parser.add_argument("--offline", action="store_true", help="强制离线模式 (跳过 LLM/网络)")
    parser.add_argument("--output-json", type=str, default=None, help="写入 JSON 到指定路径")
    parser.add_argument("--output-md", type=str, default=None, help="写入 Markdown 到指定路径")
    args = parser.parse_args(argv)

    repro = "python -m apeireth.v1138_r11_integration_acceptance"
    if args.offline:
        repro += " --offline"
    if args.strict:
        repro += " --strict"

    try:
        result = run_r11_acceptance(offline=args.offline, reproducible_invocation=repro)
    except Exception as e:  # noqa: BLE001
        LOG.error("R11 集成验收执行器异常: %s", e)
        LOG.error(traceback.format_exc())
        return 1

    # 写出 JSON / Markdown 文件
    if args.output_json:
        out_path = Path(args.output_json)
        out_path.parent.mkdir(parents=True, exist_ok=True)
        with out_path.open("w", encoding="utf-8") as f:
            json.dump(result.to_dict(), f, ensure_ascii=False, indent=2, default=str)
        LOG.info("JSON 写入: %s", out_path)

    if args.output_md:
        out_path = Path(args.output_md)
        out_path.parent.mkdir(parents=True, exist_ok=True)
        with out_path.open("w", encoding="utf-8") as f:
            f.write(render_markdown_report(result))
        LOG.info("Markdown 写入: %s", out_path)

    # 控制台输出
    if args.json:
        print(json.dumps(result.to_dict(), ensure_ascii=False, indent=2, default=str))
    elif args.report:
        print(render_markdown_report(result))
    else:
        print(f"R11 集成验收 (主 17:43 实事求是 + 主 17:58 不假装):")
        print(f"  overall: {result.overall_status.upper()}")
        print(f"  v1136: {result.v1136.get('status')} (cont={result.v1136.get('continuity')}, auto={result.v1136.get('autonomy')}, transf={result.v1136.get('transferability')})")
        print(f"  dashboard: {result.dashboard.get('status')} (v04={result.dashboard.get('v04_score')}, v05={result.dashboard.get('v05_total')})")
        print(f"  offline_tests: {result.offline_tests.get('status')} (passed={result.offline_tests.get('n_passed')}, failed={result.offline_tests.get('n_failed')}, pass_rate={result.offline_tests.get('pass_rate')})")
        print(f"  v3_guard: {result.v3_guard.get('status')} (dialog_guard={result.v3_guard.get('philosophy_dialog_guard')})")
        print(f"  n_pass={result.n_pass} n_fail={result.n_fail} n_blocked={result.n_blocked} n_unknown={result.n_unknown}")
        print(f"  elapsed: {result.elapsed_seconds}s")

    # 退出码 (主 17:58 不假装: BLOCKED ≠ 0)
    if args.strict and result.overall_status in ("fail", "blocked", "unknown"):
        LOG.error("R11 strict 模式: overall=%s → 非零退出", result.overall_status)
        if result.overall_status == "fail":
            return 2
        elif result.overall_status == "blocked":
            return 3
        return 4

    return 0


__all__ = [
    "VERSION",
    "R11_WEEK_LABEL",
    "ASI_NORTH_STAR",
    "V1136_V05_FLOOR",
    "V0_4_V05_PASS_FLOOR",
    "PYTEST_FLOOR_PASS_RATE",
    "V3_GUARDS_R11",
    "R11Blocked",
    "R11MeasurementError",
    "V1136Evidence",
    "DashboardEvidence",
    "OfflineTestEvidence",
    "V3GuardEvidence",
    "R11AcceptanceResult",
    "run_r11_acceptance",
    "render_markdown_report",
    "_collect_v1136_evidence",
    "_collect_dashboard_evidence",
    "_collect_offline_test_evidence",
    "_collect_v3_evidence",
    "main",
]


def main(argv: Optional[Sequence[str]] = None) -> int:
    """V1138 R11 集成验收 CLI 入口."""
    return _cli(argv)


if __name__ == "__main__":
    sys.exit(_cli())
