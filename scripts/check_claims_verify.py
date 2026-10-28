#!/usr/bin/env python3
"""**状态声明验证守卫**（`scripts/check_claims_verify.py`）—— 让 `claims.yaml` 的 `verify_cmd` **真的被跑**。

# 漏洞（2026-10-28 审计认定 H2）

`docs/truth/claims.yaml` 里**每条**声明都写了 `verify_cmd`，
但 `scripts/docs_truth.py` **从不执行它**（只用 `git grep` 判存在性 + `git diff` 判 STALE）
⇒ ⇒ **`verify_cmd` 是装饰品** ⇒ ⇒ 一条声明可以标 `PRODUCTION WIRED`，
而它声称的验证命令**早已失败或被删**，**没有任何守卫会发现**。

# 本脚本的纪律（「要么真跑，要么显式声明为什么不能跑」）

对每条 claim：

1. **能跑** ⇒ 真跑 `verify_cmd` ⇒ 要求 **退出码 0**；
   若是 `cargo test` / `nextest` ⇒ 还要求输出里出现 `test result: ok` 且**没有** `FAILED`；
2. **不能跑**（含 `<占位符>`、依赖本机没有的工具如 `pwsh` / `pnpm`）⇒
   **必须**在 claim 里写 `verify_skip_reason:`（一句话说明为什么），
   ⇒ ⇒ 否则**失败**（"不能跑又不说明" = 装饰品，与没有等价）。

# 用法

    python3 scripts/check_claims_verify.py --check          # 门禁（会真的跑命令，慢）
    python3 scripts/check_claims_verify.py --list           # 只列出「会跑哪些 / 哪些豁免」
    python3 scripts/check_claims_verify.py --only <claim_id>  # 只跑一条（调试用）
    python3 scripts/check_claims_verify.py --check --timeout 600

⚠️ **CI 里应当独立成一个 job**（它需要编译产物，比 `docs_truth --verify` 慢得多）。
"""

from __future__ import annotations

import argparse
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
CLAIMS = ROOT / "docs" / "status" / "claims.yaml"

#: 这些工具在本机/CI 未必有 ⇒ 命中即视为「不能跑」（但仍需 `verify_skip_reason`）。
OPTIONAL_TOOLS = ("pwsh", "powershell", "pnpm", "npm", "docker", "node")

#: PowerShell 专用 cmdlet（命令里直接写它 ⇒ 只能在 pwsh 下跑）。
POWERSHELL_CMDLETS = ("Test-Path", "Get-Content", "Set-Content", "Select-String", "Write-Host")

#: `cargo test` 系命令 ⇒ 额外检查「0 failed」。
TEST_COMMANDS = ("cargo test", "cargo nextest")


def parse_claims() -> list[dict[str, str]]:
    """解析 `claims.yaml`（同仓的极简 YAML 子集：`- id:` 开头 + `key: value`）。"""
    text = CLAIMS.read_text(encoding="utf-8")
    claims: list[dict[str, str]] = []
    current: dict[str, str] | None = None
    for line in text.splitlines():
        id_match = re.match(r"^\s*- id:\s*(\S+)", line)
        if id_match:
            current = {"id": id_match.group(1)}
            claims.append(current)
            continue
        if current is None:
            continue
        kv = re.match(r"^\s{4,}([a-z_]+):\s*(.*)$", line)
        if kv:
            value = kv.group(2).strip()
            if value.startswith('"') and value.endswith('"') and len(value) >= 2:
                value = value[1:-1]
            current[kv.group(1)] = value
    return claims


def unrunnable_reason(command: str) -> str | None:
    """判断命令**为什么不能跑**；能跑返回 `None`。"""
    if re.search(r"<[^>]+>", command):
        return "命令含 `<…>` 占位符（需要真实参数才能跑）"
    for cmdlet in POWERSHELL_CMDLETS:
        if re.search(rf"(^|[;&|]\s*){re.escape(cmdlet)}\b", command):
            return f"含 PowerShell 专用 cmdlet `{cmdlet}`（只能在 pwsh 下跑）"
    for tool in OPTIONAL_TOOLS:
        if re.search(rf"(^|[;&|]\s*|\b){re.escape(tool)}\b", command):
            present = subprocess.run(
                ["bash", "-lc", f"command -v {tool} >/dev/null 2>&1"],
                capture_output=True,
            ).returncode
            if present != 0:
                return f"依赖本机没有的工具 `{tool}`"
    return None


def run_command(command: str, timeout: int) -> tuple[bool, str]:
    """跑命令，返回 (是否通过, 摘要)。"""
    proc = subprocess.run(
        ["bash", "-lc", command],
        cwd=str(ROOT),
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
        timeout=timeout,
    )
    tail = (proc.stdout + proc.stderr).strip().splitlines()
    summary = tail[-1][:160] if tail else "(无输出)"
    if proc.returncode != 0:
        return False, f"退出码 {proc.returncode} · {summary}"
    combined = proc.stdout + proc.stderr
    if any(marker in command for marker in TEST_COMMANDS):
        match = re.search(r"test result: (\w+)\. (\d+) passed; (\d+) failed", combined)
        if match:
            failed = int(match.group(3))
            if failed != 0 or match.group(1) != "ok":
                return False, f"测试 {match.group(1)}：{match.group(2)} passed / {failed} failed"
            return True, f"{match.group(2)} passed"
        if "FAILED" in combined:
            return False, "输出里有 FAILED"
        return True, summary
    return True, summary


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    group = parser.add_mutually_exclusive_group(required=True)
    group.add_argument("--check", action="store_true", help="门禁：真跑并判定")
    group.add_argument("--list", action="store_true", help="只列出计划")
    group.add_argument("--only", metavar="CLAIM_ID", help="只处理一条")
    parser.add_argument("--timeout", type=int, default=600, help="单条超时秒数")
    args = parser.parse_args()

    claims = parse_claims()
    if args.only:
        claims = [claim for claim in claims if claim.get("id") == args.only]
        if not claims:
            print(f"FAIL: 找不到 claim `{args.only}`", file=sys.stderr)
            return 1

    failures: list[str] = []
    planned = 0
    skipped = 0
    deferred = 0
    for claim in claims:
        claim_id = claim.get("id", "?")
        command = claim.get("verify_cmd", "").strip()
        if not command:
            failures.append(f"{claim_id}: **没有 verify_cmd**（声明必须可验证）")
            continue
        # ⚠️ **延后**（`verify_defer_reason`）：命令本身能跑，但**当前环境不允许**
        # （缺系统库 / 缺 node 工具链 / 由另一条 CI 流水线覆盖）。
        # 与 `verify_skip_reason`（命令**根本跑不了**）分开 ⇒ 延后要单独可见、且**不算通过**。
        defer_reason = claim.get("verify_defer_reason", "").strip()
        if defer_reason:
            deferred += 1
            print(f"  ⏸ {claim_id}: 延后 · {defer_reason[:110]}")
            continue
        reason = unrunnable_reason(command)
        if reason is not None:
            skip_reason = claim.get("verify_skip_reason", "").strip()
            if not skip_reason:
                failures.append(
                    f"{claim_id}: 命令不能跑（{reason}）但**没有** `verify_skip_reason` "
                    f"⇒ 装饰品与没有等价\n      命令：{command}"
                )
            else:
                skipped += 1
                if args.list:
                    print(f"  ⊘ {claim_id}: 豁免（{reason}）· 理由：{skip_reason}")
            continue
        planned += 1
        if args.list:
            print(f"  ▶ {claim_id}: {command}")
            continue
        try:
            passed, summary = run_command(command, args.timeout)
        except subprocess.TimeoutExpired:
            failures.append(f"{claim_id}: **超时**（>{args.timeout}s）· 命令：{command}")
            continue
        if passed:
            print(f"  ✅ {claim_id}: {summary}")
        else:
            failures.append(f"{claim_id}: **验证失败** · {summary}\n      命令：{command}")

    mode = "list" if args.list else "check"
    if args.list:
        print(
            f"\n计划：会跑 **{planned}** 条 · 豁免 **{skipped}** 条 · 延后 **{deferred}** 条 "
            f"（共 {len(claims)} 条 claim）"
        )
        return 0

    if failures:
        print(f"FAIL: 状态声明验证（{len(failures)} 处）", file=sys.stderr)
        for line in failures:
            print(f"  · {line}", file=sys.stderr)
        print(
            "\n修法：① 命令能跑 ⇒ 让它通过（改代码/改测试/改命令）；"
            "② 命令跑不了 ⇒ 给该 claim 补 `verify_skip_reason: <一句话>`。",
            file=sys.stderr,
        )
        return 1
    print(
        f"OK: 状态声明验证通过（会跑 {planned} 条全过 · 豁免 {skipped} 条 · "
        f"**延后 {deferred} 条**——延后项需人工复核，不算通过）"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
