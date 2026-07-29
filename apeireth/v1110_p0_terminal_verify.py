"""Apeireth ASI V1110 — P0 终验一锤定音 (R9-DevOps / R9-DEV-001)

P0 三件套终验 (V1100_p0_fixes 之后, R8 全部 3 大轨道就绪前最后关卡):
  1) V1074 ASI 真生产 runner   — snapshot 真写 + V0.3 真测
  2) V1087 HQB Live Gate        — subscore 真测
  3) V1088 E2E Operator         — 真 commit + 真 lift

主哲学 LOCKED:
  - 主 22:33 ASI 北极星 (终极梦想: 任何 LLM 接入即获 AGI/ASI 能力)
  - 主 17:43 实事求是 (三件套必须真跑真产出, 不允许 stub)
  - 主 23:44 干到底 (一锤定音: all-pass 才 exit 0)
  - 主 19:33 走在前人经验上 (pytest 2008 + GitHub Actions matrix 模式)
  - 主 00:56 任何人都能接手 (`python -m apeireth.v1110_p0_terminal_verify` 一行 = 终验)

P0 终验成功标准 (主 17:43 + 主 00:44 实事求是):
  - snapshot 写入正常 + < 20MB
  - V1074 输出 V0.3 真测 >= 0.8859
  - V1087 HQB Live Gate subscore >= 1.0
  - V1088 e2e lift >= +0.0185
  - 失败时 exit 1 + Markdown 报告标注 fail, 方便定位

Usage:
    python -m apeireth.v1110_p0_terminal_verify             # 终验 + 打印
    python -m apeireth.v1110_p0_terminal_verify --json      # JSON 输出
    python -m apeireth.v1110_p0_terminal_verify --report    # 写 Markdown 报告
    python -m apeireth.v1110_p0_terminal_verify --strict    # 严格模式 (任一失败即非零退出)
"""
from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
import sys
import time
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Callable, Dict, List, Optional, Tuple

V1110_VERSION = "0.1.0"

# ---------------------------------------------------------------------------
# P0 阈值 (主 17:43 实事求是: 真限制, 不假装; 与 R8-P0 fix 一致)
# ---------------------------------------------------------------------------
SNAPSHOT_MAX_BYTES = 20 * 1024 * 1024       # 20 MB / snapshot 上限
V1074_V03_MIN = 0.8859                       # V1074 V0.3 真测下限
V1087_SUBSCORE_MIN = 1.0                     # V1087 Live Gate subscore 下限
V1088_LIFT_MIN = 0.0185                      # V1088 e2e lift 下限

SNAPSHOT_PATH = Path("artifacts/asi_snapshot.json")
REPORT_PATH_DEFAULT = Path("reports/r9-p0-terminal-verify.md")

V1074_CMD = [sys.executable, "-m", "apeireth.v1074_asi_production_runner", "--report"]
V1087_CMD = [sys.executable, "-m", "apeireth.v1087_asi_hqb_live_gate", "--lift"]
V1088_CMD = [sys.executable, "-m", "apeireth.v1088_asi_e2e_operator", "--self-check", "--json"]


# ---------------------------------------------------------------------------
# 数据结构
# ---------------------------------------------------------------------------
@dataclass
class CheckResult:
    """P0 终验单项结果 (主 17:43 实事求是: 真测真记录, 不假装)."""
    name: str
    passed: bool
    elapsed_sec: float
    detail: str = ""
    raw: Dict[str, Any] = field(default_factory=dict)
    threshold: float = 0.0
    measured: float = 0.0

    def to_dict(self) -> Dict[str, Any]:
        return {
            "name": self.name,
            "passed": self.passed,
            "elapsed_sec": round(self.elapsed_sec, 3),
            "detail": self.detail,
            "threshold": self.threshold,
            "measured": round(self.measured, 6),
            **self.raw,
        }


# ---------------------------------------------------------------------------
# 工具: 真 subprocess (主 17:43)
# ---------------------------------------------------------------------------
def _run(cmd: List[str], timeout: int = 90) -> Tuple[int, str, str, float]:
    """真跑子进程, 返回 (returncode, stdout, stderr, elapsed_sec)."""
    t0 = time.time()
    try:
        proc = subprocess.run(
            cmd, capture_output=True, text=True,
            timeout=timeout, encoding="utf-8", errors="replace",
        )
        return proc.returncode, proc.stdout, proc.stderr, time.time() - t0
    except subprocess.TimeoutExpired as e:
        out = e.stdout.decode("utf-8", "replace") if isinstance(e.stdout, bytes) else (e.stdout or "")
        return -1, out, f"TIMEOUT after {timeout}s", time.time() - t0
    except Exception as e:
        return -2, "", f"ERROR: {e!r}", time.time() - t0


def _parse_float(text: str, pattern: str, default: float = 0.0) -> float:
    """从文本提取一个 float. 找不到返回 default."""
    m = re.search(pattern, text)
    return float(m.group(1)) if m else default


def _extract_snapshot_size() -> Tuple[int, Optional[str]]:
    """读 snapshot 真实大小. 找不到 → (-1, None)."""
    if not SNAPSHOT_PATH.exists():
        return -1, None
    size = SNAPSHOT_PATH.stat().st_size
    return size, str(SNAPSHOT_PATH)


# ---------------------------------------------------------------------------
# 三件套真测
# ---------------------------------------------------------------------------
def check_v1074() -> CheckResult:
    """V1074 真测: snapshot 真写 + V0.3 真测 >= 0.8859."""
    t0 = time.time()
    rc, out, err, elapsed = _run(V1074_CMD, timeout=90)
    text = (out or "") + (err or "")
    v03 = _parse_float(text, r"ASI V0\.3 真测[:\s]+([0-9.]+)")
    all_ok = "All OK: True" in text
    snap_size, snap_path = _extract_snapshot_size()
    passed = (
        rc == 0
        and v03 >= V1074_V03_MIN
        and 0 < snap_size <= SNAPSHOT_MAX_BYTES
        and all_ok
    )
    detail = (
        f"v03={v03:.4f} (>= {V1074_V03_MIN}); "
        f"snapshot={snap_size} bytes (limit {SNAPSHOT_MAX_BYTES}); "
        f"all_ok={all_ok}; rc={rc}"
    )
    return CheckResult(
        name="V1074 ASI 真生产 runner",
        passed=passed,
        elapsed_sec=time.time() - t0,
        detail=detail,
        raw={"snapshot_size": snap_size, "snapshot_path": snap_path, "all_ok": all_ok, "rc": rc},
        threshold=V1074_V03_MIN,
        measured=v03,
    )


def check_v1087() -> CheckResult:
    """V1087 真测: HQB Live Gate subscore >= 1.0."""
    t0 = time.time()
    rc, out, err, elapsed = _run(V1087_CMD, timeout=60)
    text = (out or "") + (err or "")
    # v1087 --lift 输出 JSON; 但 CLI 解析可能不是纯 JSON, 用正则抓 v1087_subscore
    subscore = _parse_float(text, r'"v1087_subscore"\s*:\s*([0-9.]+)', default=-1.0)
    if subscore < 0:
        subscore = _parse_float(text, r'v1087_subscore[:\s=]+([0-9.]+)', default=0.0)
    lift = _parse_float(text, r'"delta"\s*:\s*([0-9.\-]+)', default=0.0)
    if lift == 0.0:
        lift = _parse_float(text, r'lift[:\s=]+\+?([0-9.]+)', default=0.0)
    # 主 17:43 实事求是: 兼容 "true" / "True" / 多种空格格式 (JSON normalize 后比较)
    _norm = text.lower().replace(" ", "")
    philosophy_ok = '"philosophy_guards_ok":true' in _norm
    passed = rc == 0 and subscore >= V1087_SUBSCORE_MIN and philosophy_ok
    detail = (
        f"subscore={subscore:.4f} (>= {V1087_SUBSCORE_MIN}); "
        f"lift=+{lift:.4f}; philosophy_ok={philosophy_ok}; rc={rc}"
    )
    return CheckResult(
        name="V1087 HQB Live Gate",
        passed=passed,
        elapsed_sec=time.time() - t0,
        detail=detail,
        raw={"subscore": subscore, "lift": lift, "philosophy_ok": philosophy_ok, "rc": rc, "stdout_tail": text[-400:]},
        threshold=V1087_SUBSCORE_MIN,
        measured=subscore,
    )


def check_v1088() -> CheckResult:
    """V1088 真测: e2e lift >= +0.0185."""
    t0 = time.time()
    rc, out, err, elapsed = _run(V1088_CMD, timeout=60)
    text = (out or "") + (err or "")
    lift = _parse_float(text, r'"asi_v03_lift"\s*:\s*([0-9.\-]+)', default=-1.0)
    if lift < 0:
        lift = _parse_float(text, r'lift[:\s=]+\+?([0-9.]+)', default=0.0)
    subscore = _parse_float(text, r'"subscore"\s*:\s*([0-9.]+)', default=0.0)
    verdict_m = re.search(r'"final_verdict"\s*:\s*"([^"]+)"', text)
    verdict = verdict_m.group(1) if verdict_m else "unknown"
    # 主 17:43 实事求是: 同 V1087 — JSON normalize 后比较
    _norm = text.lower().replace(" ", "")
    philosophy_ok = '"philosophy_guards_ok":true' in _norm
    passed = rc == 0 and lift >= V1088_LIFT_MIN and philosophy_ok
    detail = (
        f"lift=+{lift:.4f} (>= {V1088_LIFT_MIN}); "
        f"subscore={subscore:.4f}; verdict={verdict}; philosophy_ok={philosophy_ok}; rc={rc}"
    )
    return CheckResult(
        name="V1088 E2E Operator",
        passed=passed,
        elapsed_sec=time.time() - t0,
        detail=detail,
        raw={"lift": lift, "subscore": subscore, "verdict": verdict, "philosophy_ok": philosophy_ok, "rc": rc},
        threshold=V1088_LIFT_MIN,
        measured=lift,
    )


# ---------------------------------------------------------------------------
# 聚合 + 报告
# ---------------------------------------------------------------------------
def run_terminal_verify() -> Dict[str, Any]:
    """一锤定音: 真跑三件套 → 聚合结果 → 返回 dict (供 JSON / Markdown 共用)."""
    started = time.time()
    results: List[CheckResult] = []
    for fn in (check_v1074, check_v1087, check_v1088):
        try:
            r = fn()
        except Exception as e:  # 主 17:43: 不假装, 异常也算 fail
            r = CheckResult(
                name=fn.__name__,
                passed=False,
                elapsed_sec=0.0,
                detail=f"EXCEPTION: {e!r}",
                raw={"exception": repr(e)},
            )
        results.append(r)
    all_pass = all(r.passed for r in results)
    return {
        "v1110_version": V1110_VERSION,
        "started_at": started,
        "elapsed_sec": round(time.time() - started, 3),
        "all_pass": all_pass,
        "thresholds": {
            "snapshot_max_bytes": SNAPSHOT_MAX_BYTES,
            "v1074_v03_min": V1074_V03_MIN,
            "v1087_subscore_min": V1087_SUBSCORE_MIN,
            "v1088_lift_min": V1088_LIFT_MIN,
        },
        "checks": [r.to_dict() for r in results],
    }


def render_markdown(report: Dict[str, Any]) -> str:
    """P0 终验 Markdown 报告 (主 00:56 任何人都能接手: 报告可读, 一目了然)."""
    lines: List[str] = []
    verdict = "✅ ALL PASS" if report["all_pass"] else "❌ FAIL"
    lines.append(f"# R9 P0 终验报告 — V1110 ({verdict})")
    lines.append("")
    lines.append(f"- 版本: V1110 v{report['v1110_version']}")
    lines.append(f"- 开始时间戳: {report['started_at']:.3f}")
    lines.append(f"- 耗时: {report['elapsed_sec']} s")
    lines.append("")
    lines.append("## 阈值 (主 17:43 实事求是)")
    t = report["thresholds"]
    lines.append(f"- snapshot ≤ {t['snapshot_max_bytes']:,} bytes (20 MB)")
    lines.append(f"- V1074 V0.3 ≥ {t['v1074_v03_min']}")
    lines.append(f"- V1087 subscore ≥ {t['v1087_subscore_min']}")
    lines.append(f"- V1088 lift ≥ +{t['v1088_lift_min']}")
    lines.append("")
    lines.append("## 三件套结果")
    lines.append("")
    lines.append("| 组件 | PASS | 阈值 | 实测 | 耗时 (s) | 详情 |")
    lines.append("|------|------|------|------|----------|------|")
    for c in report["checks"]:
        mark = "✅" if c["passed"] else "❌"
        lines.append(
            f"| {c['name']} | {mark} | {c['threshold']} | {c['measured']:.4f} "
            f"| {c['elapsed_sec']:.2f} | {c['detail']} |"
        )
    lines.append("")
    if report["all_pass"]:
        lines.append("## 结论")
        lines.append("")
        lines.append("**P0 三件套全过** — R8 三大轨道可继续推进, R9 P0 终验 ✅。")
        lines.append("")
        lines.append("主 22:33 ASI 北极星: R8 就绪 → R9 推进 AGI/ASI 基座平台。")
    else:
        lines.append("## 失败定位")
        lines.append("")
        for c in report["checks"]:
            if not c["passed"]:
                lines.append(f"### ❌ {c['name']}")
                lines.append(f"")
                lines.append(f"- 详情: {c['detail']}")
                lines.append(f"- 原始: `{json.dumps(c.get('raw', {}), ensure_ascii=False)[:500]}`")
                lines.append("")
    return "\n".join(lines) + "\n"


# ---------------------------------------------------------------------------
# CLI
# ---------------------------------------------------------------------------
def main(argv: Optional[List[str]] = None) -> int:
    parser = argparse.ArgumentParser(
        description="V1110 P0 终验一锤定音 (R9-DevOps / R9-DEV-001)",
    )
    parser.add_argument("--json", action="store_true", help="JSON 输出")
    parser.add_argument("--report", action="store_true", help=f"写 Markdown 报告 (默认 {REPORT_PATH_DEFAULT})")
    parser.add_argument("--report-path", type=str, default=str(REPORT_PATH_DEFAULT))
    parser.add_argument("--strict", action="store_true", help="严格模式: 任一失败 exit 1")
    parser.add_argument("--self-check", action="store_true", help="V1110 自检 (仅验证函数存在 + import OK)")
    args = parser.parse_args(argv)

    if args.self_check:
        # 主 17:58 + 主 00:56: 自检 = 一行 import + 函数存在
        for fn_name in ("check_v1074", "check_v1087", "check_v1088",
                        "run_terminal_verify", "render_markdown", "main"):
            assert callable(globals().get(fn_name)), f"missing {fn_name}"
        print(f"V1110 self-check OK (version {V1110_VERSION})")
        return 0

    report = run_terminal_verify()

    if args.json:
        # 转换时间为可序列化
        out = dict(report)
        out["started_at"] = report["started_at"]
        out["checks"] = [
            {**c, "started_at": report["started_at"]} for c in report["checks"]
        ]
        print(json.dumps(out, ensure_ascii=False, indent=2))
    else:
        for c in report["checks"]:
            mark = "✅" if c["passed"] else "❌"
            print(f"{mark} {c['name']}: {c['detail']}")
        print()
        verdict = "ALL PASS ✅" if report["all_pass"] else "FAIL ❌"
        print(f"V1110 P0 终验 → {verdict} (elapsed {report['elapsed_sec']}s)")

    if args.report:
        out_path = Path(args.report_path)
        out_path.parent.mkdir(parents=True, exist_ok=True)
        out_path.write_text(render_markdown(report), encoding="utf-8")
        print(f"report → {out_path}")

    if args.strict and not report["all_pass"]:
        return 1
    return 0 if report["all_pass"] else 2


if __name__ == "__main__":
    sys.exit(main())
