#!/usr/bin/env python3
"""⭐ **本地 CI 等价运行器** —— 在本机跑一遍 CI 工作流里的真实命令。

## 为什么需要它（2026-10-26 实测触发）

本仓的 GitHub Actions runner **长期积压**（实测：提交后 8~10 小时仍全部 `queued`）
⇒ ⇒ ⇒ **CI 结论一次都没拿到过**
⇒ ⇒ ⇒ 于是「本地绿」与「CI 会绿」之间**长期没有闭环**
⇒ ⇒ ⇒ 本工具把 CI 工作流里**实际执行的命令**（从 yml **自动解析**，不手抄）
⇒ ⇒ ⇒ 在本机**逐条真跑** ⇒ ⇒ ⇒ 得到一份**「本地等价 CI 结论」**。

## 它是什么 / 不是什么

- ✅ **是**：把「CI 会跑什么」**原样**在本机跑一遍 ⇒ ⇒ 提前发现「本地就没过」的项；
- ✅ **是**：把 `${{ secrets.* }}` 依赖的 job **显式跳过**并**记明**（不是静默跳过）；
- ❌ **不是** CI 本身 ⇒ ⇒ **不能**用它宣称「CI 绿了」⇒ ⇒ 本工具的结论叫
  **「本地等价结论」**，与 CI 结论**严格分开**；
- ❌ **不是** `act`（不需要 Docker ⇒ ⇒ 在**任何**装了 Python + 工具链的机器上都能跑）。

## 用法

```bash
# 只看会跑什么（不执行）
python3 scripts/run-ci-local.py --dry-run

# 跑 ui-engine-ci 的全部 job
python3 scripts/run-ci-local.py --workflow ui-engine-ci.yml

# 只跑某几个 job
python3 scripts/run-ci-local.py --workflow ui-engine-ci.yml --job test --job clippy

# 只列出命令（便于复制到 CI 之外的机器跑）
python3 scripts/run-ci-local.py --workflow ui-engine-ci.yml --print-only
```

## 退出码

`0` = 全部（执行的）job 通过；`1` = 有 job 失败；`2` = 用法/解析错误。
"""

from __future__ import annotations

import argparse
import os
import pathlib
import re
import shlex
import shutil
import subprocess
import sys
import time

import yaml

REPO_ROOT = pathlib.Path(__file__).resolve().parent.parent
WORKFLOW_DIR = REPO_ROOT / ".github" / "workflows"

#: 需要 secret 才能跑的 job / 步骤 ⇒ ⇒ **显式跳过并记明**（绝不静默）。
SECRET_RE = re.compile(r"\$\{\{\s*secrets\.")
#: 其它插值（env / matrix / github.*）⇒ ⇒ 能替换的替换，不能替换的置空。
EXPR_RE = re.compile(r"\$\{\{(.*?)\}\}", re.S)


def _resolve_expr(text: str, env: dict) -> tuple[str, bool]:
    """替换 ``${{ ... }}`` ⇒ ⇒ 返回 `(替换后文本, 是否含 secret)`。"""
    has_secret = bool(SECRET_RE.search(text))

    def replace(match: re.Match) -> str:
        expr = match.group(1).strip()
        if expr.startswith("secrets."):
            return ""  # ⇒ secret 依赖 ⇒ ⇒ 由调用方**整步跳过**。
        if expr.startswith("env."):
            return env.get(expr[4:].strip(), "")
        # matrix.* / github.* / 其它 ⇒ ⇒ 本地**取不到**⇒ ⇒ 置空。
        return ""

    return EXPR_RE.sub(replace, text), has_secret


def _steps_of(job: dict) -> list[dict]:
    return [s for s in (job.get("steps") or []) if isinstance(s, dict)]


def _job_commands(job: dict, env: dict) -> list[tuple[str, str, bool]]:
    """抽出 job 里所有要执行的 shell 命令。

    返回 `(步骤名, 命令, 是否含 secret)`。
    """
    out: list[tuple[str, str, bool]] = []
    for step in _steps_of(job):
        run = step.get("run")
        if not run:
            continue  # ⇒ `uses:` 步骤（checkout / setup-node …）⇒ ⇒ 本地已有 ⇒ ⇒ 跳过。
        name = str(step.get("name") or run.strip().splitlines()[0][:60])
        text, has_secret = _resolve_expr(run, env)
        if has_secret:
            out.append((name, text, True))
        else:
            out.append((name, text, False))
    return out


def _working_dir(job: dict) -> pathlib.Path:
    default = (job.get("defaults") or {}).get("run") or {}
    wd = default.get("working-directory") or "."
    path = (REPO_ROOT / wd).resolve()
    return path


#: 本地 CI 专用 venv（避开 PEP 668「系统 Python 不允许 pip install」）。
_CI_VENV = pathlib.Path.home() / ".cache" / "apeireth-ci-venv"


def _ensure_ci_venv() -> pathlib.Path:
    """确保本地 venv 存在，返回其 `bin` 目录。

    ## 为什么需要（2026-10-28 实测）

    两个浏览器 job 的第一步是 `python3 -m pip install --quiet websocket-client pillow`；
    本机（Kali / Python 3.14）启用 **PEP 668** ⇒ ⇒ 直接 pip 到系统环境会**退出码 1**
    ⇒ ⇒ 本地等价 CI 的 8 个 job 里有 2 个**永远失败**，而它们在 GitHub CI 上是正常的
    ⇒ ⇒ 本地结论与 CI 结论之间出现**假差异**。

    ⇒ ⇒ 修法：CI 里的 `python3 -m pip` 在本机改走**专用 venv**（不污染系统 Python）。
    """
    bin_dir = _CI_VENV / "bin"
    if (bin_dir / "python").exists():
        return bin_dir
    print(f"  （首次）创建本地 CI venv：{_CI_VENV}", flush=True)
    _CI_VENV.parent.mkdir(parents=True, exist_ok=True)
    subprocess.run(["python3", "-m", "venv", str(_CI_VENV)], check=True)
    return bin_dir


def _rewrite_local(command: str, venv_bin: pathlib.Path) -> str:
    """把 CI 里「对系统 Python 装包」的写法改成本地 venv 的写法。"""
    replaced = command.replace(
        "python3 -m pip", f"{venv_bin / 'python'} -m pip"
    )
    return replaced


def _already_satisfied(name: str, command: str, cwd: pathlib.Path) -> str:
    """判断某个**安装类**步骤在本地是否已经被满足 ⇒ 返回原因字符串（空串=未满足）。

    ⚠️ 跳过必须**显式记录**（打印原因），不能静默 —— 否则本地结论会虚高。
    """
    if "pnpm install" in command and (cwd / "node_modules" / ".bin" / "vite").exists():
        return f"本地已存在 {cwd / 'node_modules'}（含 vite）⇒ 跳过 pnpm install"
    if "cargo install wasm-bindgen-cli" in command and shutil.which("wasm-bindgen"):
        return f"本地已安装 wasm-bindgen（{shutil.which('wasm-bindgen')}）⇒ 跳过 cargo install"
    if "pip install" in command:
        return ""  # 交给 venv 真正安装（幂等且很快）
    return ""


def main() -> int:
    parser = argparse.ArgumentParser(description="本地 CI 等价运行器")
    parser.add_argument(
        "--workflow",
        action="append",
        default=[],
        help="工作流文件名（可重复）；默认全部",
    )
    parser.add_argument("--job", action="append", default=[], help="只跑这些 job")
    parser.add_argument("--dry-run", action="store_true", help="只列命令，不执行")
    parser.add_argument("--print-only", action="store_true", help="只打印命令（便于复制）")
    parser.add_argument(
        "--continue-on-error",
        action="store_true",
        help="某个 job 失败后继续跑后面的 job（默认也继续）",
    )
    args = parser.parse_args()

    files = sorted(WORKFLOW_DIR.glob("*.yml"))
    if args.workflow:
        wanted = {w if w.endswith(".yml") else f"{w}.yml" for w in args.workflow}
        files = [f for f in files if f.name in wanted]
    if not files:
        print("找不到工作流文件", file=sys.stderr)
        return 2

    env = {k: os.environ.get(k, "") for k in os.environ}
    # ⭐ 本地 CI venv（PEP 668 对策）；把它的 bin 放到 PATH 最前
    #   ⇒ 后续 `python3` 仍指系统解释器，但 `pip install` 被重写到 venv。
    venv_bin = _ensure_ci_venv()
    env["PATH"] = f"{venv_bin}:{env.get('PATH', '')}"
    env["VIRTUAL_ENV"] = str(venv_bin.parent)
    results: list[tuple[str, str, str, float]] = []  # (workflow, job, 状态, 秒)
    failed = 0

    for workflow_file in files:
        try:
            workflow = yaml.safe_load(workflow_file.read_text(encoding="utf-8")) or {}
        except yaml.YAMLError as error:
            print(f"⚠️ 解析失败 {workflow_file.name}: {error}", file=sys.stderr)
            failed += 1
            continue
        jobs = workflow.get("jobs") or {}
        for job_name, job in jobs.items():
            if not isinstance(job, dict):
                continue
            if args.job and job_name not in args.job:
                continue
            # ⭐ **跳过带 `if:` 条件的 job**（2026-10-31 补）。
            #
            # ## 为什么
            #
            # 条件 job（如 `webview2`：只在 `workflow_dispatch` 时跑）依赖**特定事件上下文**
            # ⇒ ⇒ 本地等价 CI **没有**那个上下文 ⇒ ⇒ 硬跑它 = 跑一个**注定失败**的 job
            # ⇒ ⇒ 那会让「本地等价 CI 全绿」这个口径**被一个本地产物污染**。
            #
            # ## 为什么**不静默**跳过
            #
            # 静默跳过 ⇒ ⇒ 结论里少了一个 job 却看不出来 ⇒ ⇒ 读者会误以为「全跑了」
            # ⇒ ⇒ 所以这里**显式打印**（见下），让跳过**可见**。
            job_condition = job.get("if")
            if job_condition:
                print(
                    f"\n⏭ 跳过条件 job：{workflow_file.name} / {job_name}"
                    f"（if: {job_condition}）—— 本地等价 CI **不模拟**该事件上下文"
                    f" ⇒ ⇒ 需要真实 GitHub 运行（它的结论**不在**本地结论覆盖范围内）"
                )
                results.append((workflow_file.name, job_name, "跳过（条件 job）", 0.0))
                continue
            commands = _job_commands(job, env)
            if not commands:
                results.append((workflow_file.name, job_name, "无可执行步骤", 0.0))
                continue
            wd = _working_dir(job)
            skipped = [name for name, _, secret in commands if secret]
            runnable = [(n, c) for n, c, secret in commands if not secret]

            if args.print_only:
                print(f"\n=== {workflow_file.name} / {job_name} (cwd={wd}) ===")
                for name, command in runnable:
                    print(f"--- {name} ---")
                    print(command)
                continue

            if args.dry_run:
                print(f"\n=== {workflow_file.name} / {job_name} (cwd={wd}) ===")
                for name, command in runnable:
                    first = command.strip().splitlines()[0][:100]
                    print(f"  · {name}: {first}")
                for name in skipped:
                    print(f"  · {name}: ⏭️ 需 secret ⇒ 跳过")
                continue

            print(f"\n=== {workflow_file.name} / {job_name} (cwd={wd}) ===", flush=True)
            started = time.monotonic()
            job_ok = True
            for name, command in runnable:
                reason = _already_satisfied(name, command, wd)
                if reason:
                    print(f"  ⏭️ {name}：{reason}", flush=True)
                    continue
                command = _rewrite_local(command, venv_bin)
                print(f"  ▶ {name}", flush=True)
                proc = subprocess.run(
                    ["bash", "-euo", "pipefail", "-c", command],
                    cwd=wd,
                    env=env,
                    capture_output=True,
                    text=True,
                )
                if proc.stdout.strip():
                    tail = proc.stdout.strip().splitlines()[-12:]
                    for line in tail:
                        print(f"      {line}")
                if proc.returncode != 0:
                    job_ok = False
                    print(f"  ✗ {name} 退出码 {proc.returncode}", flush=True)
                    if proc.stderr.strip():
                        for line in proc.stderr.strip().splitlines()[-12:]:
                            print(f"      ! {line}")
                    break  # ⇒ 后续步骤多半依赖它 ⇒ ⇒ 停止本 job。
            for name in skipped:
                print(f"  ⏭️ {name}：需 secret ⇒ **跳过**（不等于通过）")
            elapsed = time.monotonic() - started
            status = "通过" if job_ok else "失败"
            if not job_ok:
                failed += 1
            results.append((workflow_file.name, job_name, status, elapsed))
            print(f"  ⇒ {status}（{elapsed:.1f}s）", flush=True)

    if args.print_only:
        return 0

    print("\n" + "=" * 72)
    print("本地等价 CI 结论（⚠️ **不等于** GitHub CI 的结论）")
    print("=" * 72)
    for workflow_name, job_name, status, elapsed in results:
        print(f"  {workflow_name:34} {job_name:20} {status:14} {elapsed:6.1f}s")
    print("-" * 72)
    print(f"  job 总数 {len(results)}；失败 {failed}")
    if failed:
        print("  ⚠️ 有 job 失败 ⇒ ⇒ 提交前必须修掉。")
    print("  ⚠️ 需要 secret 的步骤**已跳过** ⇒ ⇒ 它们**不在**本结论覆盖范围内。")
    return 1 if failed else 0


if __name__ == "__main__":
    raise SystemExit(main())
