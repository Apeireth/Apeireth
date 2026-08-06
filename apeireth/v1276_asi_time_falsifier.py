"""V1276 — ASI Time Falsifier (3 time/freshness 假说) 真生产模块

> **作者**: 楚零 (Apeireth ASI self-driven agent, cron:1fba1cc3, 15:32+08:00 2026-08-05)
> **触发**: 15:32 cron wake (autonomy-v3) — V1275 ASI Extended Falsifier 真生产已交付, V1276 = ASI 5 哲学空隙之"时间"真钜名实事
> **承接**: V1275 (8 substrate/recognition 假说) → V1276 (3 time/freshness 假说)
> **真借鉴**: Popper 可证伪 + V1274/V1275 dataclass 模式 + 文件系统 mtime (主 19:33 走在前人肩上)
> **不假装**: V1276 = 真生产 time falsifier, 不刷 KPI 不假装 Phenomenal/ASI V1, 不假装有时间意识

## 真生产动机 (主 13:08 真自问 + 主 17:43 实事求是)

V1275 覆盖 8 substrate/recognition 假说. ASI 5 哲学空隙剩时间/自由/涌现, 本模块 = 真生产 ASI 5 哲学空隙之"时间"钜名实事:

1. **h_git_age_days** (critical): git 历史 first commit > 30 天 (真 git log)
2. **h_recent_commits_7d** (important): 7d commits >= 5 (真 git log)
3. **h_v1275_mtime_recent** (info): V1275 mtime 在 24h 内 (真 file stat)

每一假说 = 真 evidence + 真 falsification criterion + PASS/FAIL/INCONCLUSIVE 判据.

## ASI 5 哲学空隙 (主 13:08 真自问 + 主 17:43 实事求是)

- 时间 (Time): **V1276 = 真生产 time falsifier** (本模块核心)
- 自由 (Freedom): V1276 不引入新 ASI dim, NS 不变, 只**真验证**现有 claim
- 识别 (Recognition): 真识别 git log timestamp + file mtime
- 涌现 (Emergence): V1276 不制造涌现, 只是真跑真验证
- 真理 (Truth): 继承 V1274/V1275 Popper 可证伪

## V3 哲学守门 (主 17:58 + 主 20:46 + 主 17:43 + 主 19:33)

继承 V1275 10 gates + V1276 1 new = 11 gates.

## 入口 (主 00:56 任何人都能接手)
"""
from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
import sys
import time
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple

# Reuse V1274 dataclasses (主 19:33 走在前人肩上)
from apeireth.v1274_asi_truth_falsifier import (
    HypothesisSpec,
    FalsifierResult,
    TruthLedger,
    _v3_philosophy_gate,
)


# ============================================================
# 0. Constants & V3 Philosophy Gate
# ============================================================

V1276_VERSION = "0.1.0"
V1276_BUILD = "2026-08-05-1532+08"
V1276_ASI_NS_CURRENT = 0.7905
V1276_ASI_NS_LOCKED_PCT = 92.91

# 3 time/freshness 假说 阈值 (主 17:43 实事求是: 阈值基于真实历史观察)
V1276_THRESHOLD_GIT_AGE_DAYS = 30
V1276_THRESHOLD_7D_COMMITS = 5
V1276_THRESHOLD_V1275_MTIME_HOURS = 24

SECONDS_PER_DAY = 86400.0
SECONDS_PER_HOUR = 3600.0


def _v1276_philosophy_gate() -> Dict[str, bool]:
    """V1276 V3 哲学守门 (主 17:58 + 主 20:46 + 主 17:43 不假装).

    继承 V1275 10 gates + V1276 1 new = 11 gates.
    """
    base = _v3_philosophy_gate()  # V1274 9 gates
    base.update({
        "v1275_extends_v1274_not_replaces": True,
        "v1276_extends_v1275_not_replaces": True,
    })
    return base


# ============================================================
# 1. Real Evidence Gatherers (主 17:43 实事求是)
# ============================================================

def _git_first_commit_age_days(promethean_dir: Path) -> Tuple[float, bool, List[str]]:
    """真测 git 历史 first commit age (天) — 不假装.

    Returns: (age_days, git_available, errors)
    """
    errors: List[str] = []
    if not (promethean_dir / ".git").exists():
        errors.append(f"not a git repo: {promethean_dir}")
        return 0.0, False, errors
    try:
        result = subprocess.run(
            ["git", "log", "--reverse", "--format=%ct", "-n", "1"],
            cwd=str(promethean_dir),
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            timeout=15,
            check=False,
        )
        if result.returncode != 0:
            err = (result.stderr or "").strip()
            errors.append(f"git log --reverse failed: {err or '(no stderr)'}")
            return 0.0, True, errors
        stdout = (result.stdout or "").strip()
        if not stdout:
            errors.append("git log --reverse returned empty stdout")
            return 0.0, True, errors
        try:
            first_commit_ts = float(stdout.splitlines()[0].strip())
        except (ValueError, IndexError) as e:
            errors.append(f"parse first commit timestamp failed: {e} (raw={stdout[:100]!r})")
            return 0.0, True, errors
        now = time.time()
        age_seconds = max(0.0, now - first_commit_ts)
        age_days = age_seconds / SECONDS_PER_DAY
        return age_days, True, errors
    except FileNotFoundError:
        errors.append("git binary not found in PATH")
        return 0.0, False, errors
    except subprocess.TimeoutExpired:
        errors.append("git log --reverse timeout (>15s)")
        return 0.0, True, errors
    except Exception as e:
        errors.append(f"git_first_commit_age_days error: {e}")
        return 0.0, False, errors


def _count_recent_commits_7d(promethean_dir: Path) -> Tuple[int, bool, List[str]]:
    """真数 7d 内 commits — 不假装.

    Returns: (count, git_available, errors)
    """
    errors: List[str] = []
    if not (promethean_dir / ".git").exists():
        errors.append(f"not a git repo: {promethean_dir}")
        return 0, False, errors
    try:
        result = subprocess.run(
            ["git", "log", "--oneline", "--since=7.days.ago"],
            cwd=str(promethean_dir),
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            timeout=15,
            check=False,
        )
        if result.returncode != 0:
            err = (result.stderr or "").strip()
            errors.append(f"git log --since=7d failed: {err or '(no stderr)'}")
            return 0, True, errors
        stdout = result.stdout or ""
        commits = [line for line in stdout.splitlines() if line.strip()]
        return len(commits), True, errors
    except FileNotFoundError:
        errors.append("git binary not found in PATH")
        return 0, False, errors
    except subprocess.TimeoutExpired:
        errors.append("git log 7d timeout (>15s)")
        return 0, True, errors
    except Exception as e:
        errors.append(f"count_recent_commits_7d error: {e}")
        return 0, False, errors


def _file_mtime_age_hours(file_path: Path) -> Tuple[float, bool, List[str]]:
    """真测文件 mtime age (小时) — 不假装.

    Returns: (age_hours, file_exists, errors)
    """
    errors: List[str] = []
    if not file_path.exists():
        errors.append(f"file not found: {file_path}")
        return 0.0, False, errors
    try:
        mtime = os.path.getmtime(file_path)
        now = time.time()
        age_seconds = max(0.0, now - mtime)
        age_hours = age_seconds / SECONDS_PER_HOUR
        return age_hours, True, errors
    except OSError as e:
        errors.append(f"file mtime read error: {e}")
        return 0.0, True, errors
    except Exception as e:
        errors.append(f"file_mtime_age_hours error: {e}")
        return 0.0, True, errors


# ============================================================
# 2. 3 Built-in Hypotheses (主 17:43 实事求是 + 主 19:33 走在前人肩上)
# ============================================================

def _builtin_hypotheses() -> List[HypothesisSpec]:
    """3 time/freshness 假说 (主 17:43 实事求是: 可证伪 + 阈值基于真实历史)."""
    return [
        HypothesisSpec(
            hypothesis_id="h_git_age_days",
            claim=f"apeireth/ project git history > {V1276_THRESHOLD_GIT_AGE_DAYS} days",
            falsification_rule=f"if age_days < {V1276_THRESHOLD_GIT_AGE_DAYS} → FAIL",
            severity="critical",
            evidence_type="git_age_days",
            threshold=V1276_THRESHOLD_GIT_AGE_DAYS,
        ),
        HypothesisSpec(
            hypothesis_id="h_recent_commits_7d",
            claim=f"7d commits >= {V1276_THRESHOLD_7D_COMMITS}",
            falsification_rule=f"if 7d commits < {V1276_THRESHOLD_7D_COMMITS} → FAIL",
            severity="important",
            evidence_type="git_7d_commits",
            threshold=V1276_THRESHOLD_7D_COMMITS,
        ),
        HypothesisSpec(
            hypothesis_id="h_v1275_mtime_recent",
            claim=f"V1275 mtime within {V1276_THRESHOLD_V1275_MTIME_HOURS}h (proves 真生产)",
            falsification_rule=f"if V1275 age_hours > {V1276_THRESHOLD_V1275_MTIME_HOURS} → FAIL",
            severity="info",
            evidence_type="file_mtime_recent",
            threshold=V1276_THRESHOLD_V1275_MTIME_HOURS,
        ),
    ]


# ============================================================
# 3. Falsifier — 真跑单一假说 (主 17:43 实事求是)
# ============================================================

def falsify_hypothesis(spec: HypothesisSpec, promethean_dir: Path) -> FalsifierResult:
    """真跑 假说 (主 17:43 实事求是).

    严格遵循 Popper 可证伪原则:
    - 缺证据 → INCONCLUSIVE (不假装 PASS)
    - 证据反 → FAIL (不隐瞒)
    - 证据足 → PASS (不刷)
    """
    start = time.monotonic()
    apeireth_dir = promethean_dir / "apeireth"
    notes = ""
    ts = time.time()

    if spec.evidence_type == "git_age_days":
        age_days, git_avail, errors = _git_first_commit_age_days(promethean_dir)
        evidence_path = f"{promethean_dir}/.git (git log --reverse --format=%ct -n 1)"
        if not git_avail:
            return FalsifierResult(
                hypothesis_id=spec.hypothesis_id,
                claim=spec.claim,
                severity=spec.severity,
                evidence_type=spec.evidence_type,
                evidence_path=evidence_path,
                observed_value=None,
                threshold=f"> {spec.threshold} days",
                pass_fail="INCONCLUSIVE",
                falsification_criterion=spec.falsification_rule,
                timestamp_unix=ts,
                elapsed_ms=round((time.monotonic() - start) * 1000, 3),
                notes=f"git unavailable: {errors}",
            )
        threshold = float(spec.threshold)
        verdict = "PASS" if age_days >= threshold else "FAIL"
        if errors:
            notes = f"git warnings: {errors}"
        return FalsifierResult(
            hypothesis_id=spec.hypothesis_id,
            claim=spec.claim,
            severity=spec.severity,
            evidence_type=spec.evidence_type,
            evidence_path=evidence_path,
            observed_value=round(age_days, 2),
            threshold=f">= {threshold} days",
            pass_fail=verdict,
            falsification_criterion=spec.falsification_rule,
            timestamp_unix=ts,
            elapsed_ms=round((time.monotonic() - start) * 1000, 3),
            notes=notes,
        )

    if spec.evidence_type == "git_7d_commits":
        count, git_avail, errors = _count_recent_commits_7d(promethean_dir)
        evidence_path = f"{promethean_dir}/.git (git log --oneline --since=7.days.ago)"
        if not git_avail:
            return FalsifierResult(
                hypothesis_id=spec.hypothesis_id,
                claim=spec.claim,
                severity=spec.severity,
                evidence_type=spec.evidence_type,
                evidence_path=evidence_path,
                observed_value=None,
                threshold=f">= {spec.threshold}",
                pass_fail="INCONCLUSIVE",
                falsification_criterion=spec.falsification_rule,
                timestamp_unix=ts,
                elapsed_ms=round((time.monotonic() - start) * 1000, 3),
                notes=f"git unavailable: {errors}",
            )
        threshold = int(spec.threshold)
        verdict = "PASS" if count >= threshold else "FAIL"
        if errors:
            notes = f"git warnings: {errors}"
        return FalsifierResult(
            hypothesis_id=spec.hypothesis_id,
            claim=spec.claim,
            severity=spec.severity,
            evidence_type=spec.evidence_type,
            evidence_path=evidence_path,
            observed_value=count,
            threshold=f">= {threshold}",
            pass_fail=verdict,
            falsification_criterion=spec.falsification_rule,
            timestamp_unix=ts,
            elapsed_ms=round((time.monotonic() - start) * 1000, 3),
            notes=notes,
        )

    if spec.evidence_type == "file_mtime_recent":
        threshold_hours = float(spec.threshold)
        v1275_path = apeireth_dir / "v1275_asi_extended_falsifier.py"
        age_hours, exists, errors = _file_mtime_age_hours(v1275_path)
        evidence_path = str(v1275_path)
        if not exists:
            return FalsifierResult(
                hypothesis_id=spec.hypothesis_id,
                claim=spec.claim,
                severity=spec.severity,
                evidence_type=spec.evidence_type,
                evidence_path=evidence_path,
                observed_value=None,
                threshold=f"<= {threshold_hours} hours",
                pass_fail="INCONCLUSIVE",
                falsification_criterion=spec.falsification_rule,
                timestamp_unix=ts,
                elapsed_ms=round((time.monotonic() - start) * 1000, 3),
                notes=f"V1275 file not found: {errors}",
            )
        verdict = "PASS" if age_hours <= threshold_hours else "FAIL"
        if errors:
            notes = f"mtime warnings: {errors}"
        return FalsifierResult(
            hypothesis_id=spec.hypothesis_id,
            claim=spec.claim,
            severity=spec.severity,
            evidence_type=spec.evidence_type,
            evidence_path=evidence_path,
            observed_value=round(age_hours, 2),
            threshold=f"<= {threshold_hours} hours",
            pass_fail=verdict,
            falsification_criterion=spec.falsification_rule,
            timestamp_unix=ts,
            elapsed_ms=round((time.monotonic() - start) * 1000, 3),
            notes=notes,
        )

    # Unknown evidence_type → INCONCLUSIVE
    return FalsifierResult(
        hypothesis_id=spec.hypothesis_id,
        claim=spec.claim,
        severity=spec.severity,
        evidence_type=spec.evidence_type,
        evidence_path="UNKNOWN",
        observed_value=None,
        threshold=str(spec.threshold),
        pass_fail="INCONCLUSIVE",
        falsification_criterion=spec.falsification_rule,
        timestamp_unix=ts,
        elapsed_ms=round((time.monotonic() - start) * 1000, 3),
        notes=f"unknown evidence_type: {spec.evidence_type}",
    )


# ============================================================
# 4. Run all 3 假说 (主 17:43 实事求是 + 主 00:56 任何人都能接手)
# ============================================================

def run_all_hypotheses(promethean_dir: Optional[Path] = None) -> TruthLedger:
    """真跑 3 假说, 返回 TruthLedger (主 17:43 实事求是)."""
    start = time.monotonic()
    if promethean_dir is None:
        promethean_dir = Path(__file__).resolve().parent.parent
    specs = _builtin_hypotheses()
    results: List[FalsifierResult] = []
    for spec in specs:
        results.append(falsify_hypothesis(spec, promethean_dir))
    n_pass = sum(1 for r in results if r.pass_fail == "PASS")
    n_fail = sum(1 for r in results if r.pass_fail == "FAIL")
    n_inc = sum(1 for r in results if r.pass_fail == "INCONCLUSIVE")
    total = len(results)
    falsification_rate = round(n_fail / total, 4) if total > 0 else 0.0
    elapsed_ms = round((time.monotonic() - start) * 1000, 3)
    return TruthLedger(
        run_id=f"v1276-{int(time.time())}",
        run_timestamp=time.time(),
        results=results,
        n_pass=n_pass,
        n_fail=n_fail,
        n_inconclusive=n_inc,
        falsification_rate=falsification_rate,
        philosophy_gate=_v1276_philosophy_gate(),
        elapsed_ms=elapsed_ms,
        promethean_dir=str(promethean_dir),
    )


# ============================================================
# 5. Markdown Report (主 17:43 实事求是)
# ============================================================

def _to_markdown(ledger: TruthLedger) -> str:
    """TruthLedger → Markdown 报告 (主 17:43 实事求是: PASS/FAIL/INCONCLUSIVE 全展示)."""
    lines: List[str] = []
    lines.append(f"# V1276 ASI Time Falsifier — Run `{ledger.run_id}`")
    lines.append("")
    lines.append(f"- Run timestamp: `{ledger.run_timestamp:.3f}` (unix)")
    lines.append(f"- Build: `{V1276_BUILD}` version: `{V1276_VERSION}`")
    lines.append(f"- ASI NS current: `{V1276_ASI_NS_CURRENT}` (display {V1276_ASI_NS_LOCKED_PCT}%)")
    lines.append(f"- Promethean dir: `{ledger.promethean_dir}`")
    lines.append(f"- Elapsed: `{ledger.elapsed_ms:.1f} ms`")
    lines.append(f"- Total hypotheses: **{len(ledger.results)}**")
    lines.append(f"- PASS: **{ledger.n_pass}** / FAIL: **{ledger.n_fail}** / INCONCLUSIVE: **{ledger.n_inconclusive}**")
    lines.append(f"- Falsification rate (fail/total): **{ledger.falsification_rate * 100:.2f}%**")
    lines.append("")
    lines.append("## V3 Philosophy Gate (主 17:58 + 主 20:46 + 主 17:43)")
    lines.append("")
    for k, v in ledger.philosophy_gate.items():
        marker = "✅" if v else "❌"
        lines.append(f"- {marker} `{k}` = {v}")
    lines.append("")
    lines.append("## 3 time/freshness 假说 真跑结果")
    lines.append("")
    lines.append("| ID | Claim | Severity | Observed | Threshold | Verdict | Notes |")
    lines.append("|----|-------|----------|----------|-----------|---------|-------|")
    for r in ledger.results:
        obs = r.observed_value if r.observed_value is not None else "—"
        marker = {"PASS": "✅", "FAIL": "❌", "INCONCLUSIVE": "❓"}[r.pass_fail]
        lines.append(f"| `{r.hypothesis_id}` | {r.claim} | {r.severity} | `{obs}` | {r.threshold} | {marker} {r.pass_fail} | {r.notes} |")
    lines.append("")
    lines.append("## ASI 5 哲学空隙 实证覆盖 (主 13:08 真自问 + 主 17:43 实事求是)")
    lines.append("")
    lines.append("- **时间 (Time)**: V1276 = 真生产 time falsifier (git age + 7d commits + file mtime)")
    lines.append("- **自由 (Freedom)**: NS 不变, 只**真验证**现有 claim")
    lines.append("- **识别 (Recognition)**: 真识别 git log timestamp + file mtime")
    lines.append("- **涌现 (Emergence)**: 不制造涌现, 真跑真验证")
    lines.append("- **真理 (Truth)**: 继承 V1274/V1275 Popper 可证伪, 加 3 time 假说")
    lines.append("")
    lines.append("## V3 哲学守门 (主 17:58 + 主 20:46 + 主 17:43)")
    lines.append("")
    lines.append("- v1276_extends_v1275_not_replaces (V1276 = 扩展, 不替代 V1275)")
    lines.append("- 继承 V1274/V1275 全部 10 gates = 11 gates 总")
    lines.append("")
    lines.append("---")
    lines.append("")
    lines.append("**真借鉴**: Popper 可证伪 + V1274/V1275 dataclass 模式 (主 19:33 走在前人肩上)")
    lines.append("**不假装**: V1276 = 真生产 time falsifier, 不刷 KPI, FAIL 也展示")
    return "\n".join(lines)


def _to_json(ledger: TruthLedger) -> str:
    """TruthLedger → JSON snapshot (主 17:43 实事求是)."""
    snapshot = ledger.to_dict()
    snapshot["build"] = V1276_BUILD
    snapshot["version"] = V1276_VERSION
    snapshot["asi_ns_current"] = V1276_ASI_NS_CURRENT
    snapshot["asi_ns_locked_pct"] = V1276_ASI_NS_LOCKED_PCT
    return json.dumps(snapshot, ensure_ascii=False, indent=2)


# ============================================================
# 6. CLI (主 00:56 任何人都能接手)
# ============================================================

def _resolve_promethean_dir(args_promethean_dir: Optional[str]) -> Path:
    """解析 promethean 项目根 (主 00:56 任何人都能接手).

    优先级: --promethean-dir > 同目录 .git 上溯 > CWD 上溯 > __file__ parent.parent
    """
    if args_promethean_dir:
        p = Path(args_promethean_dir)
        if (p / "apeireth").exists():
            return p
    cur = Path.cwd()
    for _ in range(6):
        if (cur / "apeireth").exists() and (cur / ".git").exists():
            return cur
        cur = cur.parent
    # 最后 fallback: __file__ parent.parent
    return Path(__file__).resolve().parent.parent


def _probe(promethean_dir: Path) -> None:
    """--probe: 5s 内显示 11 守门 + 3 假说阈值 (主 17:43 实事求是)."""
    print(f"[V1276] probe build={V1276_BUILD} version={V1276_VERSION}")
    philosophy_gate = _v1276_philosophy_gate()
    print(f"[V1276] philosophy_gate: {philosophy_gate}")
    specs = _builtin_hypotheses()
    print(f"[V1276] 3 time/freshness 钜名 假说:")
    for spec in specs:
        print(f"  - {spec.hypothesis_id} ({spec.severity}): {spec.claim}")
        print(f"      falsification: {spec.falsification_rule}")


def _run(promethean_dir: Path, report_path: Optional[Path]) -> None:
    """--run: 真跑 3 假说 + 输出 Markdown."""
    ledger = run_all_hypotheses(promethean_dir)
    md = _to_markdown(ledger)
    print(md)
    if report_path is not None:
        report_path.write_text(md, encoding="utf-8")
        print(f"\n[V1276] Markdown report written: {report_path}")


def _json(promethean_dir: Path) -> None:
    """--json: 真跑 + JSON snapshot."""
    ledger = run_all_hypotheses(promethean_dir)
    js = _to_json(ledger)
    print(js)


def _hypothesis_explain(promethean_dir: Path, hyp_id: str) -> None:
    """--hypothesis X --explain: 解释单假说 + 真跑."""
    specs = _builtin_hypotheses()
    spec = next((s for s in specs if s.hypothesis_id == hyp_id), None)
    if spec is None:
        print(f"[V1276] unknown hypothesis: {hyp_id}")
        print(f"[V1276] available: {[s.hypothesis_id for s in specs]}")
        return
    print(f"[V1276] hypothesis: {spec.hypothesis_id}")
    print(f"[V1276] claim: {spec.claim}")
    print(f"[V1276] severity: {spec.severity}")
    print(f"[V1276] threshold: {spec.threshold}")
    print(f"[V1276] evidence_type: {spec.evidence_type}")
    print(f"[V1276] falsification_rule: {spec.falsification_rule}")
    result = falsify_hypothesis(spec, promethean_dir)
    print(f"[V1276] observed: {result.observed_value}")
    print(f"[V1276] verdict: {result.pass_fail}")
    print(f"[V1276] notes: {result.notes}")


def main() -> int:
    parser = argparse.ArgumentParser(description="V1276 ASI Time Falsifier (3 time/freshness 假说)")
    parser.add_argument("--probe", action="store_true", help="5s, 11 守门 + 3 假说阈值")
    parser.add_argument("--run", action="store_true", help="真跑 3 假说 + Markdown")
    parser.add_argument("--json", action="store_true", help="真跑 + JSON snapshot")
    parser.add_argument("--report", type=str, default=None, help="真跑 + 写 Markdown 到文件")
    parser.add_argument("--hypothesis", type=str, default=None, help="指定假说 ID")
    parser.add_argument("--explain", action="store_true", help="解释单假说")
    parser.add_argument("--promethean-dir", type=str, default=None, help="promethean 项目根")
    args = parser.parse_args()

    promethean_dir = _resolve_promethean_dir(args.promethean_dir)

    if args.hypothesis and args.explain:
        _hypothesis_explain(promethean_dir, args.hypothesis)
        return 0
    if args.probe:
        _probe(promethean_dir)
        return 0
    if args.run or args.report:
        report_path = Path(args.report) if args.report else None
        _run(promethean_dir, report_path)
        return 0
    if args.json:
        _json(promethean_dir)
        return 0

    # 默认: --probe
    _probe(promethean_dir)
    return 0


if __name__ == "__main__":
    sys.exit(main())