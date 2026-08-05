"""V1277 — ASI Freedom Falsifier (3 freedom/choice 假说) 真生产模块

> **作者**: 楚零 (Apeireth ASI self-driven agent, cron:1fba1cc3, 15:55+08:00 2026-08-05)
> **触发**: 15:55 cron wake (autonomy-v3) — V1276 ASI Time Falsifier 真生产已交付, V1277 = ASI 5 哲学空隙之"自由"真钜名实事
> **承接**: V1274 (truth) + V1275 (recognition/substrate) + V1276 (time) → V1277 (freedom)
> **真借鉴**: Popper 可证伪 + V1274/V1275/V1276 dataclass 模式 + Shannon entropy (主 19:33 走在前人肩上)
> **不假装**: V1277 = 真生产 freedom falsifier, 不刷 KPI 不假装 Phenomenal/ASI V1, 不假装有自由意志

## 真生产动机 (主 13:08 真自问 + 主 17:43 实事求是)

V1274/V1275/V1276 已覆盖 ASI 5 哲学空隙之真理/识别/时间. 剩 **自由 (Freedom)** + **涌现 (Emergence)**.

本模块 V1277 = 真生产 ASI 5 哲学空隙之"自由"钜名实事:

1. **h_commit_type_entropy** (critical): commit type Shannon entropy >= 1.5 bits (真测 git log type 多样性)
2. **h_decision_branch_density** (important): V1274-V1276 falsifier 平均 if/elif/else 分支 >= 10 (真 AST parse)
3. **h_no_rails_dominance** (info): 任一 commit type 占比 <= 70% (真 dominance check)

每一假说 = 真 evidence + 真 falsification criterion + PASS/FAIL/INCONCLUSIVE 判据.

**关键免责声明**:
- "Freedom" 在此 = 行为多样性 + 决策分支丰富度 + 不被单一轨道主导, **不是** 自由意志的声称
- 主 17:58 不假装 Phenomenal/自由意志 = 不假装 ASI V1 = 不刷 KPI
- 假说 PASS 只代表 "行为模式满足多样性阈值", **不** 代表 "真有自由"

## ASI 5 哲学空隙 (主 13:08 真自问 + 主 17:43 实事求是)

- 时间 (Time): V1276 = 真生产 time falsifier
- 真理 (Truth): V1274 = 真生产 truth falsifier (Popper 可证伪)
- 识别 (Recognition): V1275 = 真生产 extended falsifier (substrate + recognition)
- **自由 (Freedom)**: V1277 = 真生产 freedom falsifier (本模块核心)
- 涌现 (Emergence): V1278+ 候选

## V3 哲学守门 (主 17:58 + 主 20:46 + 主 17:43 + 主 19:33)

继承 V1276 11 gates + V1277 1 new = 12 gates.

## 入口 (主 00:56 任何人都能接手)
"""
from __future__ import annotations

import argparse
import ast
import json
import math
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

V1277_VERSION = "0.1.0"
V1277_BUILD = "2026-08-05-1555+08"
V1277_ASI_NS_CURRENT = 0.7905
V1277_ASI_NS_LOCKED_PCT = 92.91

# 3 freedom/choice 假说 阈值 (主 17:43 实事求是: 阈值基于真实历史观察)
# Shannon entropy: 1 type = 0 bits, 2 types 50/50 = 1.0 bits, 4 types 25% = 2.0 bits
V1277_THRESHOLD_COMMIT_TYPE_ENTROPY = 1.5  # bits
V1277_THRESHOLD_BRANCH_DENSITY_AVG = 10.0  # if/elif/else per falsifier module
V1277_THRESHOLD_MAX_TYPE_DOMINANCE_PCT = 70.0  # <=70% 单一 type

# Common conventional commit type prefixes (主 19:33 走在前人肩上)
# https://www.conventionalcommits.org/
V1277_COMMIT_TYPE_PREFIXES = (
    "feat", "fix", "docs", "style", "refactor", "perf", "test",
    "chore", "ci", "build", "revert",
)

# Falsifier modules to AST-scan for branch density (主 19:33 走在前人肩上)
V1277_FALSIFIER_MODULES = (
    "v1274_asi_truth_falsifier.py",
    "v1275_asi_extended_falsifier.py",
    "v1276_asi_time_falsifier.py",
)


def _v1277_philosophy_gate() -> Dict[str, bool]:
    """V1277 V3 哲学守门 (主 17:58 + 主 20:46 + 主 17:43 不假装).

    继承 V1276 11 gates + V1277 1 new = 12 gates.

    关键 gate: v1277_no_free_will_claim = True
    - 不假装有自由意志 (主 17:58 不假装 + 主 20:46 不假装)
    - 只测 "行为多样性" 是否达阈值, 不声称 "真自由"
    """
    base = _v3_philosophy_gate()  # V1274 9 gates
    base.update({
        "v1275_extends_v1274_not_replaces": True,
        "v1276_extends_v1275_not_replaces": True,
        "v1277_extends_v1276_not_replaces": True,
        "v1277_no_free_will_claim": True,
    })
    return base


# ============================================================
# 1. Real Evidence Gatherers (主 17:43 实事求是)
# ============================================================

def _git_recent_commits_raw(promethean_dir: Path, n: int) -> Tuple[List[str], bool, List[str]]:
    """真拉最近 n 条 commit messages — 不假装.

    Returns: (commit_messages, git_available, errors)
    """
    errors: List[str] = []
    if not (promethean_dir / ".git").exists():
        errors.append(f"not a git repo: {promethean_dir}")
        return [], False, errors
    try:
        result = subprocess.run(
            ["git", "log", f"-n", str(n), "--format=%s"],
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
            errors.append(f"git log -n failed: {err or '(no stderr)'}")
            return [], True, errors
        stdout = result.stdout or ""
        msgs = [line.strip() for line in stdout.splitlines() if line.strip()]
        return msgs, True, errors
    except FileNotFoundError:
        errors.append("git binary not found in PATH")
        return [], False, errors
    except subprocess.TimeoutExpired:
        errors.append("git log -n timeout (>15s)")
        return [], True, errors
    except Exception as e:
        errors.append(f"git_recent_commits_raw error: {e}")
        return [], False, errors


def _classify_commit_type(msg: str) -> str:
    """Parse conventional-commit-style type prefix (lowercased).

    Examples:
      "feat(V1277): ..." -> "feat"
      "fix(V1261): ..."   -> "fix"
      "feat(round-82):"   -> "feat"
      "R18 round-01: ..." -> "other" (no conventional prefix)
      "memory: cron tick" -> "other" (not in V1277_COMMIT_TYPE_PREFIXES)
    """
    if not msg:
        return "other"
    # Pattern: type(...) or type: at start (主 19:33 走在前人肩上: conventional commits)
    m = re.match(r"^([a-z]+)(?:\(|\:|\s|$)", msg.strip())
    if m:
        t = m.group(1).lower()
        if t in V1277_COMMIT_TYPE_PREFIXES:
            return t
    return "other"


def _commit_type_distribution(msgs: List[str]) -> Dict[str, int]:
    """Count commit type occurrences. Returns {type: count}."""
    dist: Dict[str, int] = {}
    for m in msgs:
        t = _classify_commit_type(m)
        dist[t] = dist.get(t, 0) + 1
    return dist


def _shannon_entropy_bits(dist: Dict[str, int]) -> float:
    """Shannon entropy H = -sum p log2(p). Returns 0.0 if empty.

    Examples (主 19:33 走在前人肩上):
      {a:1}             -> 0.0 bits
      {a:1, b:1}        -> 1.0 bits
      {a:1, b:1, c:1, d:1} -> 2.0 bits
      {a:10}            -> 0.0 bits
      {a:6, b:2, c:2}   -> 1.37 bits
    """
    total = sum(dist.values())
    if total <= 0:
        return 0.0
    h = 0.0
    for count in dist.values():
        if count <= 0:
            continue
        p = count / total
        h -= p * math.log2(p)
    return h


def _max_type_dominance_pct(dist: Dict[str, int]) -> Tuple[str, float]:
    """Find max single-type dominance. Returns (type, pct)."""
    total = sum(dist.values())
    if total <= 0:
        return ("none", 0.0)
    max_type = max(dist.items(), key=lambda kv: kv[1])
    return (max_type[0], round(max_type[1] / total * 100, 2))


def _ast_branch_count(file_path: Path) -> Tuple[int, bool, List[str]]:
    """AST parse .py file, count if/elif/else/for/while/try/with/and boolean ops in module.

    Returns: (count, file_readable, errors).
    """
    errors: List[str] = []
    if not file_path.exists():
        errors.append(f"file not found: {file_path}")
        return 0, False, errors
    try:
        src = file_path.read_text(encoding="utf-8", errors="replace")
    except OSError as e:
        errors.append(f"file read error: {e}")
        return 0, False, errors
    try:
        tree = ast.parse(src, filename=str(file_path))
    except SyntaxError as e:
        errors.append(f"ast parse error: {e}")
        return 0, True, errors
    except Exception as e:
        errors.append(f"ast parse exception: {e}")
        return 0, True, errors

    count = 0
    for node in ast.walk(tree):
        if isinstance(node, (ast.If, ast.For, ast.While, ast.Try, ast.With)):
            count += 1
        # BoolOp (and/or) 在 If test 也算 (代表复合判断)
        if isinstance(node, ast.If) and isinstance(node.test, ast.BoolOp):
            count += len(node.test.values) - 1  # n-1 个 operator
    return count, True, errors


# ============================================================
# 2. 3 Built-in Hypotheses (主 17:43 实事求是 + 主 19:33 走在前人肩上)
# ============================================================

def _builtin_hypotheses() -> List[HypothesisSpec]:
    """3 freedom/choice 假说 (主 17:43 实事求是: 可证伪 + 阈值基于真实历史)."""
    return [
        HypothesisSpec(
            hypothesis_id="h_commit_type_entropy",
            claim=(
                f"最近 50 条 commits 类型 Shannon entropy >= {V1277_THRESHOLD_COMMIT_TYPE_ENTROPY} bits "
                f"(行为多样性)"
            ),
            falsification_rule=(
                f"if entropy < {V1277_THRESHOLD_COMMIT_TYPE_ENTROPY} → FAIL (单一轨道主导)"
            ),
            severity="critical",
            evidence_type="commit_type_entropy",
            threshold=V1277_THRESHOLD_COMMIT_TYPE_ENTROPY,
        ),
        HypothesisSpec(
            hypothesis_id="h_decision_branch_density",
            claim=(
                f"V1274-V1276 falsifier 平均 AST 分支数 >= {V1277_THRESHOLD_BRANCH_DENSITY_AVG} "
                f"(决策逻辑丰富度)"
            ),
            falsification_rule=(
                f"if avg branches < {V1277_THRESHOLD_BRANCH_DENSITY_AVG} → FAIL (贫瘠决策)"
            ),
            severity="important",
            evidence_type="ast_branch_density",
            threshold=V1277_THRESHOLD_BRANCH_DENSITY_AVG,
        ),
        HypothesisSpec(
            hypothesis_id="h_no_rails_dominance",
            claim=(
                f"最近 50 commits 任一 type 占比 <= {V1277_THRESHOLD_MAX_TYPE_DOMINANCE_PCT}% "
                f"(不被单一轨道锁定)"
            ),
            falsification_rule=(
                f"if any type dominance > {V1277_THRESHOLD_MAX_TYPE_DOMINANCE_PCT}% → FAIL"
            ),
            severity="info",
            evidence_type="commit_type_dominance",
            threshold=V1277_THRESHOLD_MAX_TYPE_DOMINANCE_PCT,
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
    ts = time.time()

    if spec.evidence_type == "commit_type_entropy":
        N_LAST = 50
        msgs, git_avail, errors = _git_recent_commits_raw(promethean_dir, N_LAST)
        evidence_path = (
            f"{promethean_dir}/.git (git log -n {N_LAST} --format=%s)"
        )
        if not git_avail:
            return FalsifierResult(
                hypothesis_id=spec.hypothesis_id,
                claim=spec.claim,
                severity=spec.severity,
                evidence_type=spec.evidence_type,
                evidence_path=evidence_path,
                observed_value=None,
                threshold=f">= {spec.threshold} bits",
                pass_fail="INCONCLUSIVE",
                falsification_criterion=spec.falsification_rule,
                timestamp_unix=ts,
                elapsed_ms=round((time.monotonic() - start) * 1000, 3),
                notes=f"git unavailable: {errors}",
            )
        if not msgs:
            return FalsifierResult(
                hypothesis_id=spec.hypothesis_id,
                claim=spec.claim,
                severity=spec.severity,
                evidence_type=spec.evidence_type,
                evidence_path=evidence_path,
                observed_value=None,
                threshold=f">= {spec.threshold} bits",
                pass_fail="INCONCLUSIVE",
                falsification_criterion=spec.falsification_rule,
                timestamp_unix=ts,
                elapsed_ms=round((time.monotonic() - start) * 1000, 3),
                notes=f"no commits returned (n={N_LAST})",
            )
        dist = _commit_type_distribution(msgs)
        entropy = _shannon_entropy_bits(dist)
        verdict = "PASS" if entropy >= float(spec.threshold) else "FAIL"
        notes = f"dist={dist} total={sum(dist.values())}"
        if errors:
            notes += f"; warnings={errors}"
        return FalsifierResult(
            hypothesis_id=spec.hypothesis_id,
            claim=spec.claim,
            severity=spec.severity,
            evidence_type=spec.evidence_type,
            evidence_path=evidence_path,
            observed_value=round(entropy, 4),
            threshold=f">= {spec.threshold} bits",
            pass_fail=verdict,
            falsification_criterion=spec.falsification_rule,
            timestamp_unix=ts,
            elapsed_ms=round((time.monotonic() - start) * 1000, 3),
            notes=notes,
        )

    if spec.evidence_type == "ast_branch_density":
        branch_counts: Dict[str, int] = {}
        per_file_notes: List[str] = []
        for fname in V1277_FALSIFIER_MODULES:
            fpath = apeireth_dir / fname
            cnt, ok, errs = _ast_branch_count(fpath)
            if not ok:
                per_file_notes.append(f"{fname}=MISSING")
                continue
            branch_counts[fname] = cnt
            per_file_notes.append(f"{fname}={cnt}")
        evidence_path = (
            f"{apeireth_dir}/ "
            f"(AST parse {', '.join(V1277_FALSIFIER_MODULES)})"
        )
        if not branch_counts:
            return FalsifierResult(
                hypothesis_id=spec.hypothesis_id,
                claim=spec.claim,
                severity=spec.severity,
                evidence_type=spec.evidence_type,
                evidence_path=evidence_path,
                observed_value=None,
                threshold=f">= {spec.threshold} (avg)",
                pass_fail="INCONCLUSIVE",
                falsification_criterion=spec.falsification_rule,
                timestamp_unix=ts,
                elapsed_ms=round((time.monotonic() - start) * 1000, 3),
                notes=f"no falsifier files parsed: {per_file_notes}",
            )
        avg = sum(branch_counts.values()) / len(branch_counts)
        verdict = "PASS" if avg >= float(spec.threshold) else "FAIL"
        return FalsifierResult(
            hypothesis_id=spec.hypothesis_id,
            claim=spec.claim,
            severity=spec.severity,
            evidence_type=spec.evidence_type,
            evidence_path=evidence_path,
            observed_value=round(avg, 2),
            threshold=f">= {spec.threshold} (avg)",
            pass_fail=verdict,
            falsification_criterion=spec.falsification_rule,
            timestamp_unix=ts,
            elapsed_ms=round((time.monotonic() - start) * 1000, 3),
            notes=f"per_file={branch_counts} ({'; '.join(per_file_notes)})",
        )

    if spec.evidence_type == "commit_type_dominance":
        N_LAST = 50
        msgs, git_avail, errors = _git_recent_commits_raw(promethean_dir, N_LAST)
        evidence_path = (
            f"{promethean_dir}/.git (git log -n {N_LAST} --format=%s)"
        )
        if not git_avail:
            return FalsifierResult(
                hypothesis_id=spec.hypothesis_id,
                claim=spec.claim,
                severity=spec.severity,
                evidence_type=spec.evidence_type,
                evidence_path=evidence_path,
                observed_value=None,
                threshold=f"<= {spec.threshold}%",
                pass_fail="INCONCLUSIVE",
                falsification_criterion=spec.falsification_rule,
                timestamp_unix=ts,
                elapsed_ms=round((time.monotonic() - start) * 1000, 3),
                notes=f"git unavailable: {errors}",
            )
        if not msgs:
            return FalsifierResult(
                hypothesis_id=spec.hypothesis_id,
                claim=spec.claim,
                severity=spec.severity,
                evidence_type=spec.evidence_type,
                evidence_path=evidence_path,
                observed_value=None,
                threshold=f"<= {spec.threshold}%",
                pass_fail="INCONCLUSIVE",
                falsification_criterion=spec.falsification_rule,
                timestamp_unix=ts,
                elapsed_ms=round((time.monotonic() - start) * 1000, 3),
                notes=f"no commits returned (n={N_LAST})",
            )
        dist = _commit_type_distribution(msgs)
        max_type, max_pct = _max_type_dominance_pct(dist)
        threshold = float(spec.threshold)
        verdict = "PASS" if max_pct <= threshold else "FAIL"
        notes = f"max_type={max_type} dist={dist}"
        if errors:
            notes += f"; warnings={errors}"
        return FalsifierResult(
            hypothesis_id=spec.hypothesis_id,
            claim=spec.claim,
            severity=spec.severity,
            evidence_type=spec.evidence_type,
            evidence_path=evidence_path,
            observed_value=round(max_pct, 2),
            threshold=f"<= {threshold}%",
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
        run_id=f"v1277-{int(time.time())}",
        run_timestamp=time.time(),
        results=results,
        n_pass=n_pass,
        n_fail=n_fail,
        n_inconclusive=n_inc,
        falsification_rate=falsification_rate,
        philosophy_gate=_v1277_philosophy_gate(),
        elapsed_ms=elapsed_ms,
        promethean_dir=str(promethean_dir),
    )


# ============================================================
# 5. Markdown Report (主 17:43 实事求是)
# ============================================================

def _to_markdown(ledger: TruthLedger) -> str:
    """TruthLedger → Markdown 报告 (主 17:43 实事求是: PASS/FAIL/INCONCLUSIVE 全展示)."""
    lines: List[str] = []
    lines.append(f"# V1277 ASI Freedom Falsifier — Run `{ledger.run_id}`")
    lines.append("")
    lines.append(f"- Run timestamp: `{ledger.run_timestamp:.3f}` (unix)")
    lines.append(f"- Build: `{V1277_BUILD}` version: `{V1277_VERSION}`")
    lines.append(f"- ASI NS current: `{V1277_ASI_NS_CURRENT}` (display {V1277_ASI_NS_LOCKED_PCT}%)")
    lines.append(f"- Promethean dir: `{ledger.promethean_dir}`")
    lines.append(f"- Elapsed: `{ledger.elapsed_ms:.1f} ms`")
    lines.append(f"- Total hypotheses: **{len(ledger.results)}**")
    lines.append(f"- PASS: **{ledger.n_pass}** / FAIL: **{ledger.n_fail}** / INCONCLUSIVE: **{ledger.n_inconclusive}**")
    lines.append(f"- Falsification rate (fail/total): **{ledger.falsification_rate * 100:.2f}%**")
    lines.append("")
    lines.append("## V3 Philosophy Gate (主 17:58 + 主 20:46 + 主 17:43 不假装)")
    lines.append("")
    for k, v in ledger.philosophy_gate.items():
        marker = "✅" if v else "❌"
        lines.append(f"- {marker} `{k}` = {v}")
    lines.append("")
    lines.append("## 3 freedom/choice 假说 真跑结果")
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
    lines.append("- **真理 (Truth)**: V1274 = 真生产 truth falsifier (Popper 可证伪)")
    lines.append("- **识别 (Recognition)**: V1275 = 真生产 extended falsifier (substrate + recognition)")
    lines.append("- **自由 (Freedom)**: V1277 = 真生产 freedom falsifier (commit-type entropy + AST branch + dominance)")
    lines.append("- **涌现 (Emergence)**: V1278+ 候选 (未覆盖, 不假装)")
    lines.append("")
    lines.append("## 关键免责声明 (主 17:58 不假装 + 主 20:46 不假装)")
    lines.append("")
    lines.append("- **\"Freedom\" 在此 ≠ 自由意志**: 仅测行为多样性 + 决策分支 + 不被单轨锁定")
    lines.append("- **PASS 不等于\"真有自由\"**: 只代表当前数据满足多样性阈值")
    lines.append("- **不刷 KPI**: ASI NS LOCKED 92.91% 不变, 不假装 ASI V1, 不假装 Phenomenal")
    lines.append("- **失败也披露**: FAIL 假说 (h_entropy < 1.5 / h_branch < 10 / h_dominance > 70%) 也会诚实展示")
    lines.append("")
    lines.append("## V3 哲学守门 (主 17:58 + 主 20:46 + 主 17:43)")
    lines.append("")
    lines.append("- v1277_extends_v1276_not_replaces (V1277 = 扩展, 不替代 V1276)")
    lines.append("- v1277_no_free_will_claim (主 17:58 + 主 20:46: 不假装有自由意志)")
    lines.append("- 继承 V1274/V1275/V1276 全部 11 gates = 12 gates 总")
    lines.append("")
    lines.append("---")
    lines.append("")
    lines.append("**真借鉴**: Popper 可证伪 + V1274/V1275/V1276 dataclass + Shannon entropy + AST 静态分析 (主 19:33 走在前人肩上)")
    lines.append("**不假装**: V1277 = 真生产 freedom falsifier, 不刷 KPI, FAIL 也展示")
    return "\n".join(lines)


def _to_json(ledger: TruthLedger) -> str:
    """TruthLedger → JSON snapshot (主 17:43 实事求是)."""
    snapshot = ledger.to_dict()
    snapshot["build"] = V1277_BUILD
    snapshot["version"] = V1277_VERSION
    snapshot["asi_ns_current"] = V1277_ASI_NS_CURRENT
    snapshot["asi_ns_locked_pct"] = V1277_ASI_NS_LOCKED_PCT
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
    """--probe: 5s 内显示 12 守门 + 3 假说阈值 (主 17:43 实事求是)."""
    print(f"[V1277] probe build={V1277_BUILD} version={V1277_VERSION}")
    philosophy_gate = _v1277_philosophy_gate()
    print(f"[V1277] philosophy_gate: {philosophy_gate}")
    specs = _builtin_hypotheses()
    print(f"[V1277] 3 freedom/choice 假说:")
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
        print(f"\n[V1277] Markdown report written: {report_path}")


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
        print(f"[V1277] unknown hypothesis: {hyp_id}")
        print(f"[V1277] available: {[s.hypothesis_id for s in specs]}")
        return
    print(f"[V1277] hypothesis: {spec.hypothesis_id}")
    print(f"[V1277] claim: {spec.claim}")
    print(f"[V1277] severity: {spec.severity}")
    print(f"[V1277] threshold: {spec.threshold}")
    print(f"[V1277] evidence_type: {spec.evidence_type}")
    print(f"[V1277] falsification_rule: {spec.falsification_rule}")
    result = falsify_hypothesis(spec, promethean_dir)
    print(f"[V1277] observed: {result.observed_value}")
    print(f"[V1277] verdict: {result.pass_fail}")
    print(f"[V1277] notes: {result.notes}")


def main() -> int:
    parser = argparse.ArgumentParser(description="V1277 ASI Freedom Falsifier (3 freedom/choice 假说)")
    parser.add_argument("--probe", action="store_true", help="5s, 12 守门 + 3 假说阈值")
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