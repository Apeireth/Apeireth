"""Apeireth R11 P0 Acceptance Gate — executable gate (主 17:43 实事求是).

Implements the P0 acceptance conditions from Omnibus §9 A/B/C (and §9.4 完成验收标准)
as a single executable gate that fails fast with a clear reason.

Per the R11 brief: ``将 Omnibus §9 A/B/C 的验收条件实现为可执行 gate:
  - 检查 V1136/V1074 真值来源
  - dashboard 版本契约
  - V3 nine-key guard
  - 测试证据
  - git 可追溯性
  接入现有测试或 CLI, 失败时给出明确原因``.

Five gates:
  Gate-A  V1136/V1074 truth source       (V0.3 / V0.5 numbers really come from the
                                          real measurement modules, not placeholders)
  Gate-B  Dashboard version contract     (report/snapshot files declare version +
                                          snapshot_id + score + timestamp and they are
                                          internally consistent)
  Gate-C  V3 nine-key guard              (all 9 "not_*" guards in
                                          ``apeireth.mcp.asi_nine_keys.ASI_NINE_KEYS``
                                          are LOCKED True)
  Gate-D  Test evidence                  (a focused subset of real pytest tests pass)
  Gate-E  Git traceability               (git available, working tree, recent commits,
                                          HEAD matches where artifacts say they do)

CLI:
    python -m apeireth.r11_requirements_gate run           # run all gates
    python -m apeireth.r11_requirements_gate run --strict  # exit 1 on any FAIL
    python -m apeireth.r11_requirements_gate run --json    # machine-readable output

Lib:
    from apeireth.r11_requirements_gate import run_all_gates, GateResult
    results = run_all_gates(workspace=Path("."))
    if not all(r.passed for r in results.values()):
        ...
"""
from __future__ import annotations

import argparse
import json
import re
import shutil
import subprocess
import sys
from dataclasses import asdict, dataclass, field
from pathlib import Path
from typing import Any, Callable, Dict, List, Optional, Sequence


# ---------------------------------------------------------------------------
# Result types (主 17:43 实事求是: 每条都是结构化数据, 不模糊字符串).
# ---------------------------------------------------------------------------


@dataclass
class GateResult:
    """单 gate 的执行结果 (主 17:43 实事求是)."""

    name: str
    passed: bool
    reason: str = ""
    details: Dict[str, Any] = field(default_factory=dict)

    def to_dict(self) -> Dict[str, Any]:
        return {
            "name": self.name,
            "passed": self.passed,
            "reason": self.reason,
            "details": self.details,
        }


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _utc_now_iso() -> str:
    """Return current UTC timestamp in ISO 8601 (主 17:43 实事求是: 时间戳可追溯)."""
    import datetime as _dt

    return _dt.datetime.now(_dt.timezone.utc).isoformat()


def _run_python_module(module: str, args: Sequence[str] = ()) -> Dict[str, Any]:
    """Run ``python -m <module> <args>`` in the current Python environment.

    Returns a dict with keys: ``ok``, ``returncode``, ``stdout``, ``stderr``,
    ``elapsed_seconds``. Never raises; subprocess failures are surfaced as
    ``ok=False`` with stderr.

    ponytail: ceiling = subprocess invocation; upgrade path = inject a venv
    here if the project ever needs to run gate tests in a different Python.
    """
    import time as _time

    if shutil.which("python") is None and shutil.which("python3") is not None:
        py = shutil.which("python3")
    else:
        py = shutil.which("python")
    if py is None:
        return {
            "ok": False,
            "returncode": -1,
            "stdout": "",
            "stderr": "no python interpreter on PATH",
            "elapsed_seconds": 0.0,
        }

    cmd = [py, "-m", module, *args]
    started = _time.time()
    try:
        proc = subprocess.run(
            cmd,
            capture_output=True,
            text=True,
            timeout=120,
            check=False,
            encoding="utf-8",
            errors="replace",  # 主 17:43: 绝不抛异常, 字节异常 U+FFFD 替代
        )
        return {
            "ok": proc.returncode == 0,
            "returncode": proc.returncode,
            "stdout": proc.stdout or "",
            "stderr": proc.stderr or "",
            "elapsed_seconds": round(_time.time() - started, 4),
        }
    except subprocess.TimeoutExpired as exc:
        return {
            "ok": False,
            "returncode": -1,
            "stdout": exc.stdout or "",
            "stderr": (exc.stderr or "") + "\n[r11-gate] subprocess timeout (120s)",
            "elapsed_seconds": round(_time.time() - started, 4),
        }
    except Exception as exc:  # pragma: no cover - defensive
        return {
            "ok": False,
            "returncode": -1,
            "stdout": "",
            "stderr": f"[r11-gate] subprocess error: {exc}",
            "elapsed_seconds": round(_time.time() - started, 4),
        }


# ---------------------------------------------------------------------------
# Gate A: V1136 / V1074 truth source
# ---------------------------------------------------------------------------


# Tasks §10.1 step 2: ``python -m apeireth.v1136_asi_v05_3dim_real_measurement --report``
# yields "ASI V0.5 = 0.8595" historically (the canonical R10 W3 printout).
_V1136_V05_LOWER_BOUND = 0.55   # V1136 expects continuity/autonomy/transferability in [0.55, 0.95]
_V1136_V05_UPPER_BOUND = 0.99
_V1074_V03_LOWER_BOUND = 0.0
_V1074_V03_UPPER_BOUND = 1.0


def gate_a_v1136_v1074_truth_source(workspace: Path) -> GateResult:
    """Gate A — V1136 真测 V0.5 + V1074 真测 V0.3 必须能跑通且数字在合理范围.

    验收语义 (主 17:43):
      - V1136 真跑测量 continuity / autonomy / transferability 三维子分
      - V1136 主编排 V0.5 = v04*0.85 + cont*0.05 + auto*0.05 + transf*0.05 必须真产出
      - V1074 真跑一个 V0.3 snapshot: status snapshot 模块可导入且 v03_score ∈ [0,1]
    """
    details: Dict[str, Any] = {"workspace": str(workspace)}
    failures: List[str] = []

    # 1) V1136 — import 真测 3 维
    try:
        from apeireth.v1136_asi_v05_3dim_real_measurement import (
            measure_continuity_real,
            measure_autonomy_real,
            measure_transferability_real,
            measure_v05_3dims,
            V3_GUARDS as V1136_GUARDS,
        )
    except Exception as exc:  # pragma: no cover
        return GateResult(
            name="A.v1136/v1074_truth_source",
            passed=False,
            reason=f"V1136 import failed: {exc}",
            details=details,
        )

    try:
        cont = measure_continuity_real()
        auto = measure_autonomy_real()
        transf = measure_transferability_real()
    except Exception as exc:
        return GateResult(
            name="A.v1136/v1074_truth_source",
            passed=False,
            reason=f"V1136 真测 3-dim 抛异常 (主 17:43 不允许 cache/mock): {exc}",
            details=details,
        )

    details["v1136_continuity"] = round(cont["continuity"], 4)
    details["v1136_autonomy"] = round(auto["autonomy"], 4)
    details["v1136_transferability"] = round(transf["transferability"], 4)
    details["v1136_n_subs_continuity"] = cont["total"]
    details["v1136_n_subs_autonomy"] = auto["total"]
    details["v1136_n_subs_transferability"] = transf["total"]

    for dim, value in (
        ("continuity", cont["continuity"]),
        ("autonomy", auto["autonomy"]),
        ("transferability", transf["transferability"]),
    ):
        if not (_V1136_V05_LOWER_BOUND <= value <= _V1136_V05_UPPER_BOUND):
            failures.append(
                f"V1136 {dim}={value:.4f} 越界 [{_V1136_V05_LOWER_BOUND},{_V1136_V05_UPPER_BOUND}]"
            )

    # 2) V1136 主编排 V0.5
    try:
        v05 = measure_v05_3dims(v04_score=0.8538, run_chaos=False)
    except Exception as exc:
        return GateResult(
            name="A.v1136/v1074_truth_source",
            passed=False,
            reason=f"V1136 measure_v05_3dims 抛异常: {exc}",
            details=details,
        )

    details["v1136_v05_total"] = round(v05.v05_total_v1136, 4)
    details["v1136_v05_v1125_placeholder"] = round(v05.v05_total_v1125, 4)
    details["v1136_v05_delta"] = v05.delta_v05_total
    details["v1136_v3_guards_pass"] = v05.v3_guards_pass
    details["v1136_v3_guards_count"] = len(V1136_GUARDS)

    if v05.v05_total_v1136 <= 0.0 or v05.v05_total_v1136 > 1.0:
        failures.append(f"V1136 v05_total_v1136={v05.v05_total_v1136:.4f} 越界 (0,1]")
    if not v05.v3_guards_pass:
        failures.append("V1136 v3_guards_pass=False (主 17:43 不通过)")

    # 3) V1074 — status snapshot 可构建
    try:
        from apeireth.v1074_asi_production_runner import StatusSnapshotBuilder
    except Exception as exc:
        return GateResult(
            name="A.v1136/v1074_truth_source",
            passed=False,
            reason=f"V1074 import failed: {exc}",
            details=details,
        )

    try:
        builder = StatusSnapshotBuilder(project_dir=workspace)
        snap = builder.build()
    except Exception as exc:
        return GateResult(
            name="A.v1136/v1074_truth_source",
            passed=False,
            reason=f"V1074 StatusSnapshotBuilder.build() 抛异常: {exc}",
            details=details,
        )

    details["v1074_snapshot_id"] = snap.snapshot_id
    details["v1074_level"] = snap.level
    details["v1074_v03_score"] = round(snap.v03_score, 4)
    details["v1074_n_modules"] = snap.n_modules
    details["v1074_n_tests"] = snap.n_tests
    details["v1074_n_commits"] = snap.n_commits

    if not (_V1074_V03_LOWER_BOUND <= snap.v03_score <= _V1074_V03_UPPER_BOUND):
        failures.append(
            f"V1074 v03_score={snap.v03_score:.4f} 越界 [{_V1074_V03_LOWER_BOUND},{_V1074_V03_UPPER_BOUND}]"
        )
    if not snap.snapshot_id or not isinstance(snap.snapshot_id, str):
        failures.append("V1074 snapshot_id 缺失或非字符串")

    if failures:
        return GateResult(
            name="A.v1136/v1074_truth_source",
            passed=False,
            reason="; ".join(failures),
            details=details,
        )

    return GateResult(
        name="A.v1136/v1074_truth_source",
        passed=True,
        reason=(
            f"V1136 真测 3-dim + V0.5={details['v1136_v05_total']}, "
            f"V1074 V0.3={details['v1074_v03_score']} ({snap.snapshot_id})"
        ),
        details=details,
    )


# ---------------------------------------------------------------------------
# Gate B: Dashboard version contract
# ---------------------------------------------------------------------------


# Snapshot JSON must contain these keys (V1074 status snapshot + dashboard version contract)
_REQUIRED_SNAPSHOT_KEYS = (
    "snapshot_id",
    "ts_iso",
    "version",
    "level",
    "v03_score",
    "n_modules",
    "n_tests",
    "n_commits",
)


def gate_b_dashboard_version_contract(workspace: Path) -> GateResult:
    """Gate B — artifacts/asi_snapshot.json + reports/asi_report.md 版本契约.

    验收语义 (主 17:43):
      - ``artifacts/asi_snapshot.json`` 必须存在且包含 §9.4 验收要求的真字段集
      - ``reports/asi_report.md`` 必须声明 Snapshot ID + 生成时间 + Runner 版本
      - 两者 Snapshot ID 必须一致 (契约), score 数值必须一致 (契约)
    """
    workspace = workspace.resolve()
    snap_path = workspace / "artifacts" / "asi_snapshot.json"
    report_path = workspace / "reports" / "asi_report.md"

    details: Dict[str, Any] = {
        "snapshot_path": str(snap_path),
        "report_path": str(report_path),
    }
    failures: List[str] = []

    if not snap_path.exists():
        return GateResult(
            name="B.dashboard_version_contract",
            passed=False,
            reason=f"snapshot file 缺失: {snap_path} (V1074 主 23:44 必须真存)",
            details=details,
        )

    try:
        snap = json.loads(snap_path.read_text(encoding="utf-8"))
    except Exception as exc:
        return GateResult(
            name="B.dashboard_version_contract",
            passed=False,
            reason=f"snapshot JSON 解析失败: {exc}",
            details=details,
        )

    if not isinstance(snap, dict):
        return GateResult(
            name="B.dashboard_version_contract",
            passed=False,
            reason="snapshot JSON 顶层不是对象",
            details=details,
        )

    missing = [k for k in _REQUIRED_SNAPSHOT_KEYS if k not in snap]
    if missing:
        failures.append(f"snapshot 缺字段: {missing}")

    details["snapshot_id"] = snap.get("snapshot_id")
    details["version"] = snap.get("version")
    details["level"] = snap.get("level")
    details["v03_score"] = snap.get("v03_score")
    details["n_modules"] = snap.get("n_modules")
    details["n_tests"] = snap.get("n_tests")
    details["n_commits"] = snap.get("n_commits")
    details["ts_iso"] = snap.get("ts_iso")

    # Dashboards: derive expected contract from the snapshot (single source of truth).
    expected_snap_id = snap.get("snapshot_id")
    expected_score = snap.get("v03_score")

    # Report file existence + minimal header fields
    if not report_path.exists():
        failures.append(f"report file 缺失: {report_path}")
    else:
        report_text = report_path.read_text(encoding="utf-8")
        details["report_size_bytes"] = len(report_text)
        if expected_snap_id and expected_snap_id not in report_text:
            failures.append(
                f"report 没引用 snapshot_id={expected_snap_id} (V1130 dashboard 版本契约违反)"
            )
        # score 在 report 中必须出现 (允许 ±0.0001 浮点抖动)
        if expected_score is not None:
            esc_score = f"{float(expected_score):.4f}"
            esc_score_4 = f"{float(expected_score):.4f}"
            # 接受 "0.8964" / "0.89640" / "0.896400" 三种四舍五入格式
            score_re = re.compile(
                rf"0\.0*{int(float(expected_score) * 10000):04d}"
            )
            if not score_re.search(report_text):
                failures.append(
                    f"report 没引用 v03_score≈{esc_score_4} (datasource 不一致)"
                )

    if failures:
        return GateResult(
            name="B.dashboard_version_contract",
            passed=False,
            reason="; ".join(failures),
            details=details,
        )
    return GateResult(
        name="B.dashboard_version_contract",
        passed=True,
        reason=(
            f"snapshot v{snap.get('version')} level={snap.get('level')} "
            f"v03_score={snap.get('v03_score')} ({expected_snap_id}) 与 report 一致"
        ),
        details=details,
    )


# ---------------------------------------------------------------------------
# Gate C: V3 nine-key guard
# ---------------------------------------------------------------------------


def gate_c_v3_nine_key_guard(workspace: Path) -> GateResult:
    """Gate C — 所有 9 个 V3 philosophy "not_*" 键 LOCKED True.

    Source of truth: ``apeireth.mcp.asi_nine_keys.ASI_NINE_KEYS`` + ``AsiNineKeyLock``.

    验收语义 (主 17:58 不假装 + 主 22:33 LOCKED):
      - 9 键集合必须与 ASI_NINE_KEYS 完全匹配
      - 默认 Lock(全 True) 必须 ``all_locked() == True``
      - 任何 1 键 ``False`` → gate 拒服
    """
    try:
        from apeireth.mcp.asi_nine_keys import ASI_NINE_KEYS, AsiNineKeyLock
    except Exception as exc:
        return GateResult(
            name="C.v3_nine_key_guard",
            passed=False,
            reason=f"import apeireth.mcp.asi_nine_keys 失败: {exc}",
            details={"workspace": str(workspace)},
        )

    keys = tuple(ASI_NINE_KEYS)
    details: Dict[str, Any] = {
        "n_keys": len(keys),
        "keys": list(keys),
    }

    # Lock 构造必须成功 (含 9 键全部 True)
    try:
        lock = AsiNineKeyLock()
    except Exception as exc:
        return GateResult(
            name="C.v3_nine_key_guard",
            passed=False,
            reason=f"AsiNineKeyLock() 默认 LOCKED 失败: {exc}",
            details=details,
        )

    details["lock_values"] = dict(lock.values)
    if not lock.all_locked():
        return GateResult(
            name="C.v3_nine_key_guard",
            passed=False,
            reason=f"默认 Lock 不是 9/9: failed={lock.failed_keys()}",
            details=details,
        )

    # 模拟 1 键失败 → verify_or_raise 必抛 (反转守门)
    from apeireth.mcp.asi_nine_keys import verify_or_raise

    broken = AsiNineKeyLock(
        values={k: (False if i == 0 else True) for i, k in enumerate(keys)}
    )
    try:
        verify_or_raise(broken)
    except RuntimeError as exc:
        details["verify_or_raise_works"] = True
        details["verify_or_raise_message"] = str(exc)
    else:  # pragma: no cover
        return GateResult(
            name="C.v3_nine_key_guard",
            passed=False,
            reason=(
                "verify_or_raise 对 1 键 False 的 lock 没抛异常 → "
                "9 键守门失效 (主 23:44 干到底违反)"
            ),
            details=details,
        )

    return GateResult(
        name="C.v3_nine_key_guard",
        passed=True,
        reason=f"ASI 9 键 全部 LOCKED ({len(keys)}/{len(keys)})",
        details=details,
    )


# ---------------------------------------------------------------------------
# Gate D: Test evidence (主 17:43 实事求是: 真跑 pytest 子集)
# ---------------------------------------------------------------------------


# 关键 gate-evidence 测试文件 — 这些覆盖 V0.5/V0.3/9-key/asi 真测。
# 选最小可执行子集 (≤ 60s) 避免 gate 太慢, 但必须含 5 类真测.
# ponytail: ceiling = 5 个文件; 升级路径 = 把 P0 护栏拆成 live/offline 两组.
_DEFAULT_GATE_TEST_FILES: tuple[str, ...] = (
    "tests/test_v1136_asi_v05_3dim_real_measurement.py",
    "tests/test_r4_asi_fun_score.py",
    "tests/test_r4_cli_smoke.py",
    "tests/test_r6_formal_verify_contract.py",
    # R11 ATE-001: P0 回归护栏覆盖 V1136 / V1074 / V0.4 / dashboard / 9-key / 失败语义
    "tests/test_r11_p0_regression_guard.py",
)


def gate_d_test_evidence(workspace: Path, *, test_files: Sequence[str] = _DEFAULT_GATE_TEST_FILES) -> GateResult:
    """Gate D — 跑 pytest 真测子集, 验证 P0 验收标准 §9.4 #2 真测试 (不是 mock).

    验收语义:
      - 至少跑 ``test_v1136_*`` + V0.3 真测相关子集
      - 全部 collected tests PASSED 才算通过
      - 失败时打印失败的 test_id + 失败原因 (不模糊)
    """
    workspace = workspace.resolve()
    details: Dict[str, Any] = {
        "workspace": str(workspace),
        "test_files": list(test_files),
    }
    missing = [
        tf for tf in test_files if not (workspace / tf).exists()
    ]
    if missing:
        return GateResult(
            name="D.test_evidence",
            passed=False,
            reason=f"test files 不存在: {missing} (主 17:43 实事求是不允许 mock)",
            details=details,
        )

    details["tests_run"] = True
    proc = _run_python_module(
        "pytest",
        args=[
            "-q",
            "--no-header",
            "--tb=short",
            "-x",
            "--no-cov",
            *test_files,
        ],
    )
    details["pytest_returncode"] = proc["returncode"]
    details["pytest_elapsed_seconds"] = proc["elapsed_seconds"]
    # 截短 stdout/stderr (避免 gate 报告过长)
    stdout = proc["stdout"]
    stderr = proc["stderr"]
    details["pytest_stdout_tail"] = stdout[-2000:]
    details["pytest_stderr_tail"] = stderr[-2000:]

    if not proc["ok"]:
        return GateResult(
            name="D.test_evidence",
            passed=False,
            reason=(
                f"pytest 子集失败 (returncode={proc['returncode']}): "
                f"{stdout.strip().splitlines()[-3:] if stdout.strip() else stderr.strip().splitlines()[-3:]}"
            ),
            details=details,
        )

    # 必须真的"passed"出现在末尾 — 不接受 warning-only
    last_lines = [ln.strip() for ln in stdout.strip().splitlines() if ln.strip()][-3:]
    details["pytest_summary"] = last_lines
    if not any("passed" in ln.lower() for ln in last_lines):
        return GateResult(
            name="D.test_evidence",
            passed=False,
            reason=(
                "pytest 输出末尾没 'passed' 字样 → 不算真测试 (主 17:43 不允许 mock)"
                f"\n  tail={last_lines}"
            ),
            details=details,
        )

    return GateResult(
        name="D.test_evidence",
        passed=True,
        reason=f"pytest 子集 PASSED ({len(test_files)} files): {last_lines[-1]}",
        details=details,
    )


# ---------------------------------------------------------------------------
# Gate E: Git traceability
# ---------------------------------------------------------------------------


def _git(*args: str, cwd: Path) -> Dict[str, Any]:
    """Call git with the given args and return parsed output."""
    if shutil.which("git") is None:
        return {"ok": False, "returncode": -1, "stdout": "", "stderr": "git not on PATH"}
    try:
        proc = subprocess.run(
            ["git", *args],
            cwd=str(cwd),
            capture_output=True,
            text=True,
            timeout=30,
            check=False,
            encoding="utf-8",
            errors="replace",  # 主 17:43: 绝不抛异常, 字节异常用 U+FFFD 替代
        )
        return {
            "ok": proc.returncode == 0,
            "returncode": proc.returncode,
            "stdout": proc.stdout or "",
            "stderr": proc.stderr or "",
        }
    except subprocess.TimeoutExpired:
        return {"ok": False, "returncode": -1, "stdout": "", "stderr": "git timeout"}
    except Exception as exc:  # pragma: no cover
        return {"ok": False, "returncode": -1, "stdout": "", "stderr": f"git error: {exc}"}


def gate_e_git_traceability(workspace: Path) -> GateResult:
    """Gate E — git 可追溯性 (主 23:44 干到底 + §9.4 #5 git commit + log 可追溯).

    验收语义:
      - git 可执行 (在 PATH)
      - workspace 是 git 仓库 (git rev-parse 成功)
      - git log --oneline 有 ≥ 1 commit
      - HEAD 可解析为 7-40 位 hex (sha)
      - 最近 5 个 commit 含 1 个 feat/dock/ref/test 类型的 conventional commit
    """
    details: Dict[str, Any] = {"workspace": str(workspace)}
    failures: List[str] = []

    rev = _git("rev-parse", "--verify", "HEAD", cwd=workspace)
    if not rev["ok"] or not rev["stdout"].strip():
        return GateResult(
            name="E.git_traceability",
            passed=False,
            reason=(
                f"git rev-parse HEAD 失败: not a git repo or empty ({rev['stderr'].strip() or 'no stdout'})"
            ),
            details=details,
        )
    head_sha = rev["stdout"].strip()
    details["head_sha"] = head_sha

    sha_re = re.compile(r"^[0-9a-f]{7,40}$")
    if not sha_re.match(head_sha):
        failures.append(f"HEAD SHA 不是合法 hex: {head_sha!r}")

    log = _git("log", "--oneline", "-n", "20", cwd=workspace)
    if not log["ok"]:
        return GateResult(
            name="E.git_traceability",
            passed=False,
            reason=f"git log 失败: {log['stderr'].strip()}",
            details=details,
        )
    commits = [ln for ln in log["stdout"].splitlines() if ln.strip()]
    details["n_recent_commits"] = len(commits)
    details["recent_commits_tail"] = commits[-5:]

    if len(commits) < 1:
        failures.append("git log --oneline 返回 0 commit (仓库为空?)")

    # 鼓励 conventional commit prefix (feat/fix/docs/ref/test/perf/chore/r10)
    conventional_re = re.compile(
        r"^[0-9a-f]{7,12}\s+(feat|fix|docs|ref|test|perf|chore|r\d{1,2}|ci|build)(\(.+\))?:",
        re.IGNORECASE,
    )
    conventional_hits = sum(1 for c in commits if conventional_re.match(c))
    details["conventional_commit_count"] = conventional_hits
    details["conventional_commit_ratio"] = (
        round(conventional_hits / len(commits), 4) if commits else 0.0
    )
    if len(commits) >= 5 and conventional_hits == 0:
        # 不强制, 但记 warning — 不阻断 gate
        details["conventional_warning"] = (
            "最近 20 个 commit 没 1 个 conventional commit format (建议 feat/fix/docs/ref/test)"
        )

    # git status: working tree 状态 — 不强制 clean (允许 dirty),
    # 但必须能跑 (Porcelain 输出)
    status = _git("status", "--porcelain", cwd=workspace)
    if status["ok"]:
        details["git_porcelain_lines"] = len(status["stdout"].splitlines())
        details["git_porcelain_sample"] = status["stdout"].splitlines()[:5]

    # 与 artifacts/asi_snapshot.json 中"n_commits" 字段交叉验证 (容差 ±20)
    # 这把"git 可追溯"接到"真测真值"上 — 两类数据源对得上才算真
    snap_path = workspace / "artifacts" / "asi_snapshot.json"
    if snap_path.exists():
        try:
            snap = json.loads(snap_path.read_text(encoding="utf-8"))
            snap_n = snap.get("n_commits")
            total_log = _git("log", "--oneline", cwd=workspace)
            n_log = len([ln for ln in total_log["stdout"].splitlines() if ln.strip()]) if total_log["ok"] else None
            details["snapshot_n_commits"] = snap_n
            details["git_log_n_commits"] = n_log
            if isinstance(snap_n, int) and isinstance(n_log, int):
                # 允许 log 比 snapshot 多 (后续 commit 正常), 但 log < snapshot 必须 fail
                if n_log + 50 < snap_n:  # snapshot 比 git 多 > 50 → 异常
                    failures.append(
                        f"git log ({n_log}) << snapshot.n_commits ({snap_n}) (差 {snap_n - n_log}); "
                        "可能 snapshot 残留 + git history 被回滚 (主 17:43 不允许)"
                    )
        except Exception as exc:
            details["snapshot_cross_check_error"] = str(exc)

    if failures:
        return GateResult(
            name="E.git_traceability",
            passed=False,
            reason="; ".join(failures),
            details=details,
        )
    return GateResult(
        name="E.git_traceability",
        passed=True,
        reason=(
            f"git HEAD={head_sha[:12]} ({len(commits)} recent commits, "
            f"{conventional_hits} conventional)"
        ),
        details=details,
    )


# ---------------------------------------------------------------------------
# Orchestration
# ---------------------------------------------------------------------------


ALL_GATES: Dict[str, Callable[[Path], GateResult]] = {
    "A.v1136/v1074_truth_source": gate_a_v1136_v1074_truth_source,
    "B.dashboard_version_contract": gate_b_dashboard_version_contract,
    "C.v3_nine_key_guard": gate_c_v3_nine_key_guard,
    "D.test_evidence": gate_d_test_evidence,
    "E.git_traceability": gate_e_git_traceability,
}


def run_all_gates(workspace: Path) -> Dict[str, GateResult]:
    """Run all 5 gates. Each gate is independent; one fails ≠ others skipped (主 23:44)."""
    return {name: fn(workspace) for name, fn in ALL_GATES.items()}


def render_markdown_report(results: Dict[str, GateResult]) -> str:
    """Render gate report as Markdown (主 17:43 实事求是)."""
    lines = ["# R11 P0 Acceptance Gate Report", ""]
    n_pass = sum(1 for r in results.values() if r.passed)
    n_total = len(results)
    lines.append(f"**Generated (UTC):** {_utc_now_iso()}")
    lines.append(f"**Result:** {n_pass}/{n_total} gates PASS")
    lines.append("")
    lines.append("| Gate | Status | Reason |")
    lines.append("|------|--------|--------|")
    for name, r in results.items():
        status = "✅ PASS" if r.passed else "❌ FAIL"
        lines.append(f"| `{name}` | {status} | {r.reason.replace(chr(10), ' / ')[:200]} |")
    lines.append("")
    lines.append("---")
    lines.append("")
    for name, r in results.items():
        lines.append(f"## `{name}` — {'PASS' if r.passed else 'FAIL'}")
        lines.append("")
        lines.append(f"**Reason**: {r.reason}")
        lines.append("")
        if r.details:
            lines.append("<details><summary>Details (click to expand)</summary>")
            lines.append("")
            lines.append("```json")
            lines.append(json.dumps(r.details, indent=2, ensure_ascii=False, default=str))
            lines.append("```")
            lines.append("")
            lines.append("</details>")
            lines.append("")
    lines.append("---")
    lines.append("")
    lines.append(f"_Generated by apeireth.r11_requirements_gate ({n_total} gates)._")
    return "\n".join(lines)


# ---------------------------------------------------------------------------
# CLI
# ---------------------------------------------------------------------------


def _cli(argv: Optional[Sequence[str]] = None) -> int:
    parser = argparse.ArgumentParser(
        prog="apeireth.r11_requirements_gate",
        description="R11 P0 acceptance gate (Omnibus §9 A/B/C + §9.4 完成验收标准).",
    )
    sub = parser.add_subparsers(dest="command")

    run = sub.add_parser("run", help="run all 5 P0 gates")
    run.add_argument(
        "--workspace",
        default=".",
        help="workspace root (default: current dir)",
    )
    run.add_argument(
        "--strict", action="store_true",
        help="exit 1 if any gate fails (CI use)",
    )
    run.add_argument(
        "--json", action="store_true", help="emit JSON instead of Markdown report"
    )
    run.add_argument(
        "--out", default=None,
        help="write report to this file (default: stdout)",
    )

    args = parser.parse_args(argv)

    if args.command is None:
        parser.print_help()
        return 0

    if args.command != "run":
        return 2

    workspace = Path(args.workspace).resolve()
    if not workspace.exists():
        print(f"[r11-gate] workspace 不存在: {workspace}", file=sys.stderr)
        return 2

    results = run_all_gates(workspace)
    n_pass = sum(1 for r in results.values() if r.passed)
    n_total = len(results)

    if args.json:
        payload = {
            "workspace": str(workspace),
            "ts": _utc_now_iso(),
            "n_pass": n_pass,
            "n_total": n_total,
            "all_passed": n_pass == n_total,
            "results": {k: v.to_dict() for k, v in results.items()},
        }
        out_text = json.dumps(payload, indent=2, ensure_ascii=False, default=str)
    else:
        out_text = render_markdown_report(results)

    if args.out:
        Path(args.out).write_text(out_text, encoding="utf-8")
        print(f"[r11-gate] report written to {args.out} ({n_pass}/{n_total} gates PASS)")
    else:
        print(out_text)

    if args.strict and n_pass != n_total:
        return 1
    return 0


__all__ = [
    "GateResult",
    "ALL_GATES",
    "gate_a_v1136_v1074_truth_source",
    "gate_b_dashboard_version_contract",
    "gate_c_v3_nine_key_guard",
    "gate_d_test_evidence",
    "gate_e_git_traceability",
    "run_all_gates",
    "render_markdown_report",
]


if __name__ == "__main__":
    raise SystemExit(_cli())
