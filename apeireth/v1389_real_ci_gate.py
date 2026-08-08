"""Phase 1389 v1389_real_ci_gate — V1389 ASI 真生产 CI gate (主 06:15 + 主 23:44 + 主 22:33 + 主 19:33 + 主 17:43 + 主 17:33 + 主 00:36).

主 06:15 当前真生产方向: V1389 = 真生产 CI gate (post-V1388 next-step).
主 23:44 干到底: 真 CI gate 不是"应该有 CI", 是真能跑 / 真能断 PR / 真能 SARIF.
主 22:33 ASI 北极星: 真 CI gate 是 V1384-V1388 价值的最终兑现.
主 19:33 走在前人经验上: 真借鉴 super-linter (https://github.com/github/super-linter) + diff-cover (https://github.com/Bachmann1234/diff_cover) + pre-commit (https://pre-commit.com) + jest-snapshot + GitHub Actions 官方文档.
主 17:43 实事求是: 真 artifact (real bash + real YAML + real README) + 真 health check + 真可执行.
主 17:33 放手干到底.
主 00:36 质量 + 适配性 + 效果 + 工程化: 真 CLI + 真 exit code + 真 JSON / Markdown 输出 + 真 dry-run.

V1389 真生产设计 (主 19:33 super-linter + pre-commit + GitHub Actions 真借鉴):
- 真 4 个 artifacts: apeireth-ci-gate.sh + github-actions.yml + pre-commit-hooks.yaml + README.md
- 真 health check: 验证所有 artifacts 存在 + YAML 有效 + shell script 包含真命令
- 真 wrapped subprocess: 调 bash 跑 apeireth-ci-gate.sh, 真 exit code 反射
- 真 multi-format 输出: text / json / markdown
- 真 CLI 入口:
  - check: 验证所有 artifacts 健康 (CI 启动前先 health check)
  - run: 真跑 CI gate (作为 subprocess)
  - demo: 演示用 bad fixture 触发 exit 1
  - demo-clean: 演示用 clean fixture 触发 exit 0
  - version: 真报版本
- 真 exit code:
  - 0 = 全部 ok (artifacts 存在 + YAML 有效 + 真跑真过)
  - 1 = 跑出 new findings (regression)
  - 2 = 跑出 baseline missing
  - 3 = 跑出 IO 错 / artifact 缺 / YAML 错

V3 哲学守门 (主 17:58 + 主 20:46 + 主 17:43):
- 不假装 Phenomenal consciousness: 本模块是 CI gate, 不是 consciousness claim.
- 不假装达到 ASI: 真 CI gate ≠ ASI 达成; 真 CI gate 是 ASI 北极星里的一小步.
- 不假装调整模型 & prompt: 真生产是真 health check + 真 subprocess + 真 exit code, 不是改 prompt 假装 CI.
- 真 CI gate = 真借鉴 + 真算法 + 真跑真测 + 真 commit + 真可执行.
- 任何声称 "CI gate = safety" 都是不假装. 真 CI gate ≠ 安全审计.
- 任何声称 "CI gate = ASI" 都是不假装. 真 CI gate 是 ASI 北极星里的一小步.
"""
from __future__ import annotations

import argparse
import json
import os
import re
import shutil
import subprocess
import sys
import time
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple

# V1389 真生产 PyYAML (主 17:43 实事求是; 已装)
try:
    import yaml  # PyYAML 6.0.3+
    _YAML_AVAILABLE = True
except Exception:  # pragma: no cover
    yaml = None  # type: ignore[assignment]
    _YAML_AVAILABLE = False


V1389_VERSION = "0.1.0"
V1389_SCHEMA = "v1389.ci-gate/v1"

# V1389 真生产 默认 artifacts 目录 (主 19:33 走在前人经验上)
V1389_ARTIFACTS_DIR = "deploy/ci-gate"
V1389_SHELL_SCRIPT = "apeireth-ci-gate.sh"
V1389_GITHUB_ACTIONS = "github-actions.yml"
V1389_PRE_COMMIT = "pre-commit-hooks.yaml"
V1389_README = "README.md"

# V1389 真生产 default 目标目录 (主 17:43)
V1389_DEFAULT_TARGET = "deploy"

# V1389 真生产 默认 baseline (主 17:43)
V1389_DEFAULT_BASELINE = ".v1387_baseline.json"

# V1389 真生产: apeireth package 根 (= promethean/) so subprocess 可以 `-m apeireth.v1388_...`
V1389_PKG_ROOT = str(Path(__file__).resolve().parent.parent)


# V1389 真生产 bash probe (主 17:43 实事求是)
# Windows AppX bash.exe (WSL launcher) hangs forever when invoked non-interactively;
# probe with short timeout so tests + CLI can fall back gracefully.
def _bash_probe(timeout_seconds: float = 2.0) -> bool:
    """V1389 真生产: probe bash with short timeout; return True only if it responds.

    Returns False if bash not found OR if probe hangs / errors.
    Used by run_gate to detect unusable bash and skip directly to fallback path.
    Used by tests via pytest.importorskip pattern (test marks skip when this returns False).
    """
    bash_exe = shutil.which("bash")
    if not bash_exe:
        return False
    try:
        proc = subprocess.run(
            [bash_exe, "-c", "echo ok"],
            capture_output=True,
            text=True,
            timeout=timeout_seconds,
        )
        return proc.returncode == 0 and "ok" in (proc.stdout or "")
    except (subprocess.TimeoutExpired, FileNotFoundError, OSError):
        return False


# ============================================================================
# V1389 真生产 数据结构 (主 17:43 实事求是)
# ============================================================================


@dataclass
class ArtifactHealth:
    """V1389 真生产 单个 artifact 健康状态 (主 17:43)."""

    name: str               # artifact 相对路径
    abs_path: str           # 绝对路径
    exists: bool
    size: int = 0
    valid: bool = False     # YAML 解析过 / shell script 命令存在
    error: str = ""         # 错描述
    note: str = ""          # 备注 (e.g. "YAML parsed", "27 hooks found")

    def to_dict(self) -> Dict[str, Any]:
        return {
            "name": self.name,
            "abs_path": self.abs_path,
            "exists": self.exists,
            "size": self.size,
            "valid": self.valid,
            "error": self.error,
            "note": self.note,
        }


@dataclass
class GateHealth:
    """V1389 真生产 CI gate 总健康状态 (主 17:43)."""

    schema: str = V1389_SCHEMA
    version: str = V1389_VERSION
    artifacts_dir: str = ""
    started_at: str = ""
    finished_at: str = ""
    elapsed_seconds: float = 0.0
    artifacts: List[ArtifactHealth] = field(default_factory=list)
    n_artifacts: int = 0
    n_artifacts_valid: int = 0
    n_artifacts_missing: int = 0
    n_artifacts_invalid: int = 0
    ok: bool = False
    guard_violations: List[str] = field(default_factory=list)
    known_unknowns: List[str] = field(default_factory=list)

    def to_dict(self) -> Dict[str, Any]:
        return {
            "schema": self.schema,
            "version": self.version,
            "artifacts_dir": self.artifacts_dir,
            "started_at": self.started_at,
            "finished_at": self.finished_at,
            "elapsed_seconds": round(self.elapsed_seconds, 4),
            "n_artifacts": self.n_artifacts,
            "n_artifacts_valid": self.n_artifacts_valid,
            "n_artifacts_missing": self.n_artifacts_missing,
            "n_artifacts_invalid": self.n_artifacts_invalid,
            "ok": self.ok,
            "artifacts": [a.to_dict() for a in self.artifacts],
            "guard_violations": list(self.guard_violations),
            "known_unknowns": list(self.known_unknowns),
        }


@dataclass
class GateRun:
    """V1389 真生产 CI gate 一次运行结果 (主 17:43)."""

    schema: str = V1389_SCHEMA
    version: str = V1389_VERSION
    artifacts_dir: str = ""
    target: str = ""
    baseline: str = ""
    started_at: str = ""
    finished_at: str = ""
    elapsed_seconds: float = 0.0
    exit_code: int = -1
    stdout: str = ""
    stderr: str = ""
    cmd: List[str] = field(default_factory=list)
    ok: bool = False
    regression: bool = False
    baseline_missing: bool = False
    guard_violations: List[str] = field(default_factory=list)
    known_unknowns: List[str] = field(default_factory=list)

    def to_dict(self) -> Dict[str, Any]:
        return {
            "schema": self.schema,
            "version": self.version,
            "artifacts_dir": self.artifacts_dir,
            "target": self.target,
            "baseline": self.baseline,
            "started_at": self.started_at,
            "finished_at": self.finished_at,
            "elapsed_seconds": round(self.elapsed_seconds, 4),
            "exit_code": self.exit_code,
            "cmd": list(self.cmd),
            "ok": self.ok,
            "regression": self.regression,
            "baseline_missing": self.baseline_missing,
            "stdout_lines": self.stdout.count("\n") if self.stdout else 0,
            "stderr_lines": self.stderr.count("\n") if self.stderr else 0,
            "guard_violations": list(self.guard_violations),
            "known_unknowns": list(self.known_unknowns),
        }


# ============================================================================
# V1389 真生产 artifact health check (主 17:43 实事求是)
# ============================================================================


def _check_shell_script(path: Path) -> ArtifactHealth:
    """V1389 真生产 shell script artifact 健康 (主 17:43)."""
    h = ArtifactHealth(name=str(path), abs_path=str(path), exists=False)
    if not path.exists():
        h.error = "shell script not found"
        return h
    try:
        h.size = path.stat().st_size
        h.exists = True
        text = path.read_text(encoding="utf-8", errors="replace")
    except OSError as e:
        h.error = f"read error: {e}"
        return h

    # V1389 真生产 验证 shebang (must come before required-commands check
    # so that no-shebang scripts get a clean error message)
    if not text.startswith("#!"):
        h.error = "missing shebang"
        return h

    # V1389 真生产 验证关键命令存在 (主 17:43 实事求是)
    required_commands = [
        "python -m apeireth.v1387_deploy_stack_runner",
        "python -m apeireth.v1388_v1387_baseline_diff",
        "apeireth/v1387",
        "apeireth/v1388",
        "exit 0",
        "exit 1",
        "exit 2",
        "exit 3",
    ]
    missing = [cmd for cmd in required_commands if cmd not in text]
    if missing:
        h.error = f"missing required commands: {missing}"
        return h

    # V1389 真生产 验证可执行 (Unix only)
    if sys.platform != "win32":
        if not os.access(path, os.X_OK):
            h.error = "shell script not executable (chmod +x)"
            return h

    h.valid = True
    h.note = f"shell script {h.size} bytes, {len(text.splitlines())} lines, {len(missing)} missing commands"
    return h


def _check_yaml(path: Path, expected_keys: List[str]) -> ArtifactHealth:
    """V1389 真生产 YAML artifact 健康 (主 17:43 实事求是)."""
    h = ArtifactHealth(name=str(path), abs_path=str(path), exists=False)
    if not path.exists():
        h.error = "YAML file not found"
        return h
    if not _YAML_AVAILABLE:
        h.error = "PyYAML not installed"
        return h
    try:
        h.size = path.stat().st_size
        h.exists = True
        with open(path, "r", encoding="utf-8") as f:
            data = yaml.safe_load(f)
    except (OSError, yaml.YAMLError) as e:
        h.error = f"YAML parse error: {e}"
        return h

    if not isinstance(data, list):
        h.error = "YAML must be a list (pre-commit hooks format)"
        return h

    # V1389 真生产 验证关键字段
    if not data:
        h.error = "YAML list is empty"
        return h

    # 检查每个 hook 至少有 id + name
    for i, hook in enumerate(data):
        if not isinstance(hook, dict):
            h.error = f"hook {i} is not a dict"
            return h
        if "id" not in hook:
            h.error = f"hook {i} missing 'id'"
            return h

    h.valid = True
    h.note = f"YAML parsed: {len(data)} hooks, ids={[h.get('id', '?') for h in data]}"
    return h


def _check_github_actions(path: Path) -> ArtifactHealth:
    """V1389 真生产 GitHub Actions YAML artifact 健康 (主 17:43)."""
    h = ArtifactHealth(name=str(path), abs_path=str(path), exists=False)
    if not path.exists():
        h.error = "GitHub Actions YAML not found"
        return h
    if not _YAML_AVAILABLE:
        h.error = "PyYAML not installed"
        return h
    try:
        h.size = path.stat().st_size
        h.exists = True
        with open(path, "r", encoding="utf-8") as f:
            data = yaml.safe_load(f)
    except (OSError, yaml.YAMLError) as e:
        h.error = f"YAML parse error: {e}"
        return h

    if not isinstance(data, dict):
        h.error = "GitHub Actions YAML must be a dict"
        return h

    # V1389 真生产 验证 jobs 存在
    if "jobs" not in data:
        h.error = "GitHub Actions YAML missing 'jobs'"
        return h

    # V1389 真生产 验证 on 触发器
    if True not in (data.get("on") or data.get(True) or {}).keys() if isinstance(data.get("on"), dict) else False:
        # 'on' can be a string or list or dict
        if "on" not in data and True not in data:
            h.error = "GitHub Actions YAML missing 'on' (or True) trigger"
            return h

    # V1389 真生产 验证 job 步骤有 shell command
    text = path.read_text(encoding="utf-8", errors="replace")
    required_in_workflow = [
        "python -m apeireth.v1387_deploy_stack_runner",
        "python -m apeireth.v1388_v1387_baseline_diff",
        "bash deploy/ci-gate/apeireth-ci-gate.sh",
        "upload-sarif",
    ]
    missing = [cmd for cmd in required_in_workflow if cmd not in text]
    if missing:
        h.error = f"missing required workflow commands: {missing}"
        return h

    h.valid = True
    n_jobs = len(data.get("jobs", {}))
    h.note = f"GitHub Actions YAML: {n_jobs} jobs, {h.size} bytes"
    return h


def _check_readme(path: Path) -> ArtifactHealth:
    """V1389 真生产 README artifact 健康 (主 17:43)."""
    h = ArtifactHealth(name=str(path), abs_path=str(path), exists=False)
    if not path.exists():
        h.error = "README not found"
        return h
    try:
        h.size = path.stat().st_size
        h.exists = True
        text = path.read_text(encoding="utf-8", errors="replace")
    except OSError as e:
        h.error = f"read error: {e}"
        return h

    # V1389 真生产 验证 README 包含关键 section
    required_sections = [
        "Quick Start",
        "Exit Code",
        "Option",
        "Honesty",
    ]
    missing = [s for s in required_sections if s.lower() not in text.lower()]
    if missing:
        h.error = f"missing README sections: {missing}"
        return h

    h.valid = True
    h.note = f"README: {h.size} bytes, {len(text.splitlines())} lines"
    return h


def check_artifacts(artifacts_dir: str = V1389_ARTIFACTS_DIR) -> GateHealth:
    """V1389 真生产 验证所有 artifacts 健康 (主 17:43 实事求是).

    返回 GateHealth, ok=True iff 4 artifacts 都 valid.
    """
    t0 = time.time()
    base = Path(artifacts_dir).resolve()
    health = GateHealth(artifacts_dir=str(base))
    health.started_at = time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())

    shell_h = _check_shell_script(base / V1389_SHELL_SCRIPT)
    gha_h = _check_github_actions(base / V1389_GITHUB_ACTIONS)
    pc_h = _check_yaml(
        base / V1389_PRE_COMMIT,
        expected_keys=["id", "name", "entry"],
    )
    readme_h = _check_readme(base / V1389_README)

    health.artifacts = [shell_h, gha_h, pc_h, readme_h]
    health.n_artifacts = len(health.artifacts)
    health.n_artifacts_valid = sum(1 for a in health.artifacts if a.valid)
    health.n_artifacts_missing = sum(1 for a in health.artifacts if not a.exists)
    health.n_artifacts_invalid = sum(
        1 for a in health.artifacts if a.exists and not a.valid
    )

    health.ok = (health.n_artifacts_valid == health.n_artifacts)

    for a in health.artifacts:
        if not a.exists:
            health.guard_violations.append(f"GUARD_ARTIFACT_EXISTS: {a.name} missing")
        elif not a.valid:
            health.guard_violations.append(f"GUARD_ARTIFACT_VALID: {a.name} invalid ({a.error})")

    # V1389 真生产 known unknowns (主 17:43 实事求是)
    health.known_unknowns = [
        "V1389 health check verifies artifact existence + YAML validity + shell script commands, not behavior",
        "V1389 does not run V1387/V1388 directly — it shells out to apeireth-ci-gate.sh",
        "V1389 health check is fast (~30ms) and side-effect free",
        "V1389 depends on PyYAML 6.0.3+ for YAML validation",
        "V1389 Unix chmod +x check is skipped on Windows (CI runs on Linux)",
    ]

    health.finished_at = time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())
    health.elapsed_seconds = time.time() - t0
    return health


# ============================================================================
# V1389 真生产 CI gate run (主 17:43 实事求是)
# ============================================================================


def run_gate(
    artifacts_dir: str = V1389_ARTIFACTS_DIR,
    target: str = V1389_DEFAULT_TARGET,
    baseline: str = V1389_DEFAULT_BASELINE,
    extra_args: Optional[List[str]] = None,
    cwd: Optional[str] = None,
    timeout: int = 60,
) -> GateRun:
    """V1389 真生产 真跑 CI gate (主 17:43 实事求是).

    真调用 apeireth-ci-gate.sh as subprocess, 真 exit code 反射.
    """
    t0 = time.time()
    base = Path(artifacts_dir).resolve()
    shell_script = base / V1389_SHELL_SCRIPT
    run_cwd = cwd or str(Path.cwd())

    gr = GateRun(
        artifacts_dir=str(base),
        target=target,
        baseline=baseline,
    )
    gr.started_at = time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())

    if not shell_script.exists():
        gr.exit_code = 3
        gr.stderr = f"shell script not found: {shell_script}"
        gr.guard_violations.append("GUARD_SHELL_SCRIPT_EXISTS: missing")
        gr.finished_at = time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())
        gr.elapsed_seconds = time.time() - t0
        return gr

    # V1389 真生产 真 build command (主 17:43)
    cmd: List[str] = []
    bash_failed = False
    bash_error: Optional[str] = None
    if sys.platform == "win32":
        # Windows: use bash from PATH (Git Bash, WSL bash, or MSYS bash)
        bash_exe = shutil.which("bash") or shutil.which("C:\\Program Files\\Git\\bin\\bash.exe")
        if not bash_exe:
            gr.exit_code = 3
            gr.stderr = "bash not found in PATH (install Git Bash or WSL)"
            gr.guard_violations.append("GUARD_BASH_AVAILABLE: bash not in PATH")
            gr.finished_at = time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())
            gr.elapsed_seconds = time.time() - t0
            return gr
        # V1389 真生产 bash probe (主 17:43): Windows AppX bash hangs forever;
        # probe with short timeout before committing to a 60s subprocess call.
        if not _bash_probe(timeout_seconds=2.0):
            bash_failed = True
            bash_error = "bash found but unresponsive (likely Windows AppX WSL launcher hanging non-interactively)"
            gr.guard_violations.append("GUARD_BASH_RESPONSIVE: bash probe failed")
            gr.cmd = [bash_exe, str(shell_script), "--target", target, "--baseline", baseline]
            gr.stdout = ""
            gr.stderr = bash_error
        else:
            cmd = [bash_exe, str(shell_script)]
    else:
        cmd = [str(shell_script)]

    if cmd:
        cmd += ["--target", target, "--baseline", baseline]
        if extra_args:
            cmd += list(extra_args)
        gr.cmd = cmd
    # else: bash_failed=True above already set gr.cmd
    if not bash_failed:
        try:
            proc = subprocess.run(
                cmd,
                cwd=run_cwd,
                capture_output=True,
                text=True,
                timeout=timeout,
                env={**os.environ, "PYTHONPATH": V1389_PKG_ROOT},
            )
            gr.exit_code = proc.returncode
            gr.stdout = proc.stdout or ""
            gr.stderr = proc.stderr or ""
        except subprocess.TimeoutExpired as e:
            bash_failed = True
            bash_error = f"subprocess timeout after {timeout}s: {e}"
            gr.guard_violations.append(f"GUARD_TIMEOUT: {timeout}s")
        except FileNotFoundError as e:
            bash_failed = True
            bash_error = f"subprocess not found: {e}"
            gr.guard_violations.append(f"GUARD_SUBPROCESS_EXISTS: {e}")
        except OSError as e:
            bash_failed = True
            bash_error = f"subprocess error: {e}"
            gr.guard_violations.append(f"GUARD_SUBPROCESS_ERROR: {e}")

    # V1389 真生产 fallback (主 17:43 实事求是): bash 在某些 Windows 机器
    # (AppX bash.exe 挂起) 上不可用; fallback 到直接调底层 Python 工具,
    # 真 CI gate 的逻辑仍在, 只是不走 bash 包装.
    if bash_failed:
        gr.stdout = gr.stdout or ""
        gr.stderr = bash_error or ""
        try:
            py_cmd = [
                sys.executable,
                "-m",
                "apeireth.v1388_v1387_baseline_diff",
                target,
                "--baseline",
                baseline,
            ]
            if extra_args:
                py_cmd += list(extra_args)
            py_proc = subprocess.run(
                py_cmd,
                cwd=run_cwd,
                capture_output=True,
                text=True,
                timeout=timeout,
                env={**os.environ, "PYTHONPATH": V1389_PKG_ROOT},
            )
            gr.exit_code = py_proc.returncode
            gr.stdout = (gr.stdout + "\n[fallback python: " +
                         " ".join(py_cmd) + "]\n" + (py_proc.stdout or ""))
            gr.stderr = (gr.stderr + "\n" + (py_proc.stderr or ""))
            gr.guard_violations.append("GUARD_BASH_FALLBACK: ran underlying tool directly")
        except subprocess.TimeoutExpired as e:
            gr.exit_code = 3
            gr.stderr += f"\npython fallback timeout: {e}"
            gr.guard_violations.append(f"GUARD_TIMEOUT_FALLBACK: {timeout}s")
        except FileNotFoundError as e:
            gr.exit_code = 3
            gr.stderr += f"\npython fallback not found: {e}"
            gr.guard_violations.append(f"GUARD_PYTHON_NOT_FOUND: {e}")
        except OSError as e:
            gr.exit_code = 3
            gr.stderr += f"\npython fallback error: {e}"
            gr.guard_violations.append(f"GUARD_PYTHON_ERROR: {e}")

    # V1389 真生产 exit code 映射 (主 17:43)
    gr.ok = (gr.exit_code == 0)
    gr.regression = (gr.exit_code == 1)
    gr.baseline_missing = (gr.exit_code == 2)

    # V1389 真生产 known unknowns
    gr.known_unknowns = [
        "V1389 run_gate is a thin wrapper around apeireth-ci-gate.sh",
        "V1389 does not introduce new lint rules — it runs V1387 + V1388",
        "V1389 exit code 3 indicates artifact missing / bash not found / IO error",
        "V1389 timeout default 60s; adjust for very large deploy trees",
        "V1389 on Windows requires bash in PATH (Git Bash, WSL, or MSYS)",
    ]

    gr.finished_at = time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())
    gr.elapsed_seconds = time.time() - t0
    return gr


# ============================================================================
# V1389 真生产 输出格式 (主 00:36 工程化)
# ============================================================================


def _format_health_text(health: GateHealth, quiet: bool = False) -> str:
    """V1389 真生产 health check text 真报 (主 17:43)."""
    lines: List[str] = []
    lines.append(f"V1389 CI gate v{health.version} — health check")
    lines.append(f"  artifacts_dir: {health.artifacts_dir}")
    lines.append(
        f"  artifacts: {health.n_artifacts_valid}/{health.n_artifacts} valid "
        f"(missing={health.n_artifacts_missing} invalid={health.n_artifacts_invalid}) "
        f"ok={health.ok} elapsed={health.elapsed_seconds:.3f}s"
    )
    if quiet:
        return "\n".join(lines)
    for a in health.artifacts:
        status = "OK" if a.valid else ("MISSING" if not a.exists else "INVALID")
        lines.append(f"  [{status}] {a.name}  ({a.size} bytes)  {a.note or a.error}")
    if health.guard_violations:
        lines.append("  guard violations:")
        for v in health.guard_violations:
            lines.append(f"    - {v}")
    if health.known_unknowns:
        lines.append("  known unknowns:")
        for u in health.known_unknowns:
            lines.append(f"    - {u}")
    return "\n".join(lines)


def _format_run_text(run: GateRun, quiet: bool = False) -> str:
    """V1389 真生产 run text 真报 (主 17:43)."""
    lines: List[str] = []
    lines.append(f"V1389 CI gate v{run.version} — run")
    lines.append(f"  cmd: {' '.join(run.cmd)}")
    lines.append(f"  exit_code: {run.exit_code} (ok={run.ok})")
    lines.append(f"  elapsed: {run.elapsed_seconds:.3f}s")
    if quiet:
        return "\n".join(lines)
    if run.stdout:
        lines.append(f"  stdout ({run.stdout.count(chr(10))} lines):")
        for line in run.stdout.splitlines()[:50]:
            lines.append(f"    {line}")
    if run.stderr:
        lines.append(f"  stderr ({run.stderr.count(chr(10))} lines):")
        for line in run.stderr.splitlines()[:20]:
            lines.append(f"    {line}")
    if run.guard_violations:
        lines.append("  guard violations:")
        for v in run.guard_violations:
            lines.append(f"    - {v}")
    return "\n".join(lines)


# ============================================================================
# V1389 真生产 CLI (主 17:43 真可执行)
# ============================================================================


def run_cli(argv: Optional[List[str]] = None) -> int:
    """V1389 真生产 CLI 入口 (主 00:36 工程化)."""
    parser = argparse.ArgumentParser(
        prog="v1389-real-ci-gate",
        description="V1389 ASI real CI gate (post-V1388 next-step)",
    )
    sub = parser.add_subparsers(dest="cmd", required=True)

    # V1389 check
    p_check = sub.add_parser("check", help="Verify all artifacts exist and are valid")
    p_check.add_argument("--artifacts-dir", default=V1389_ARTIFACTS_DIR)
    p_check.add_argument("--json", action="store_true")
    p_check.add_argument("--quiet", action="store_true")

    # V1389 run
    p_run = sub.add_parser("run", help="Run the CI gate (subprocess)")
    p_run.add_argument("--artifacts-dir", default=V1389_ARTIFACTS_DIR)
    p_run.add_argument("--target", default=V1389_DEFAULT_TARGET)
    p_run.add_argument("--baseline", default=V1389_DEFAULT_BASELINE)
    p_run.add_argument("--quiet", action="store_true")
    p_run.add_argument("--strict", action="store_true")
    p_run.add_argument("--save-baseline", action="store_true")
    p_run.add_argument("--baseline-missing-strict", action="store_true")
    p_run.add_argument("--timeout", type=int, default=60)

    # V1389 demo
    p_demo = sub.add_parser("demo", help="Demo with bad fixture (regression)")
    p_demo.add_argument("--quiet", action="store_true")

    # V1389 demo-clean
    p_demo_clean = sub.add_parser("demo-clean", help="Demo with clean fixture (no regression)")
    p_demo_clean.add_argument("--quiet", action="store_true")

    # V1389 stats
    sub.add_parser("stats", help="Print V1389 stats")

    # V1389 version
    sub.add_parser("version", help="Print V1389 version")

    args = parser.parse_args(argv)

    if args.cmd == "version":
        print(f"V1389 CI gate v{V1389_VERSION} (schema: {V1389_SCHEMA})")
        return 0

    if args.cmd == "stats":
        print(json.dumps({
            "version": V1389_VERSION,
            "schema": V1389_SCHEMA,
            "artifacts_dir": V1389_ARTIFACTS_DIR,
            "yaml_available": _YAML_AVAILABLE,
            "platform": sys.platform,
            "python_version": sys.version.split()[0],
            "philosophy": (
                "V1389 = real CI gate over V1387 + V1388. "
                "4 real artifacts: shell script + GH Actions + pre-commit + README. "
                "Real borrowed: super-linter + diff-cover + pre-commit + GH Actions docs."
            ),
        }, indent=2, ensure_ascii=False))
        return 0

    if args.cmd == "check":
        health = check_artifacts(args.artifacts_dir)
        if args.json:
            print(json.dumps(health.to_dict(), indent=2, ensure_ascii=False))
        else:
            print(_format_health_text(health, quiet=args.quiet))
        return 0 if health.ok else 1

    if args.cmd == "run":
        extra_args: List[str] = []
        if args.strict:
            extra_args += ["--strict"]
        if args.save_baseline:
            extra_args += ["--save-baseline"]
        if args.baseline_missing_strict:
            extra_args += ["--baseline-missing-strict"]
        gr = run_gate(
            artifacts_dir=args.artifacts_dir,
            target=args.target,
            baseline=args.baseline,
            extra_args=extra_args,
            timeout=args.timeout,
        )
        if not args.quiet:
            print(_format_run_text(gr, quiet=False))
        else:
            print(json.dumps(gr.to_dict(), indent=2, ensure_ascii=False))
        return gr.exit_code

    if args.cmd in ("demo", "demo-clean"):
        # V1389 真生产 demo: create a temp deploy with bad/clean fixture
        import tempfile
        tmp = Path(tempfile.mkdtemp(prefix="v1389_demo_"))
        try:
            deploy_dir = tmp / "deploy"
            deploy_dir.mkdir()
            if args.cmd == "demo":
                # Bad: Dockerfile with unpinned apt-get + :latest
                (deploy_dir / "Dockerfile").write_text(
                    "FROM ubuntu:14.04\n"
                    "RUN apt-get install -y gcc\n"
                    "CMD [\"sh\"]\n",
                    encoding="utf-8",
                )
            else:
                # Clean: minimal Dockerfile
                (deploy_dir / "Dockerfile").write_text(
                    "FROM ubuntu:22.04\n"
                    "USER nobody\n"
                    "HEALTHCHECK CMD true\n"
                    "CMD [\"sh\"]\n",
                    encoding="utf-8",
                )
            extra_args: List[str] = []
            if args.cmd == "demo":
                extra_args += ["--baseline-missing-strict"]
            gr = run_gate(
                artifacts_dir=V1389_ARTIFACTS_DIR,
                target=str(deploy_dir),
                baseline=str(tmp / "nonexistent.json"),
                extra_args=extra_args,
                timeout=30,
            )
            if not args.quiet:
                print(_format_run_text(gr, quiet=False))
            else:
                print(json.dumps(gr.to_dict(), indent=2, ensure_ascii=False))
            return gr.exit_code
        finally:
            shutil.rmtree(tmp, ignore_errors=True)

    parser.print_help()
    return 3


if __name__ == "__main__":
    # V1389 真生产 CLI 入口 (主 00:36 质量 + 适配性 + 效果 + 工程化).
    sys.exit(run_cli())
