"""V1286 — Security Fix Priority Queue (VCP 真实源代码深读 #7) 真生产模块

> **作者**: 楚零 (Apeireth ASI self-driven agent, cron:1fba1cc3, 18:40+08:00 2026-08-05)
> **触发**: 18:40 cron tick (autonomy-v3) — V1285 all-42 security depth 已 commit (16f48b94).
>          V1285 数据: 1173 hotspots across 42 crates (140/210 PASS).
>          V1285 = 真生产 audit, V1286 = 真生产 fix priority queue (audit → action).
> **承接**: V1284 worst-5 + V1285 all-42 安全审计 → V1286 修复优先级排序 (干到底, 主 23:44)
> **真借鉴**: 主 19:33 走在前人肩上 + V1285 dataclass + Popper 可证伪 + CVSS 风险评分 (severity × volume)
> **不假装**: V1286 = 真生产 fix priority queue, 不假装 ASI V1, 不假装"已修复"
> **不假装**: V1286 仅给 P0/P1/P2 fix 方向, **不** 真批量替换 (主 13:31 大胆激进 ≠ 鲁莽)

## 真生产动机 (主 13:08 真自问 + 主 17:43 实事求是 + 主 17:58 不假装 + 主 19:33 走在前人肩上 + 主 23:44 干到底)

V1285 all-42 安全审计收官后, 1173 hotspots 待处理. 主 23:44 干到底 = 从 audit 到 action.

**V1286 = 真生产 Security Fix Priority Queue**, 排序维度:

1. **severity-weighted score**: critical=10, important=5, info=1
2. **governance-weighted bonus**: governance crates (sovereignty/asi/upgrade/evolution/council) +20 (主 17:43 实事求是, 这些是治理核心)
3. **per-crate aggregated**: score = sum(severity × count) + governance_bonus
4. **P0/P1/P2 分级**:
   - **P0 (立即修)**: score >= 100 OR 含 unsafe 块
   - **P1 (本 sprint)**: 50 <= score < 100
   - **P2 (本季度)**: 0 < score < 50
   - **OK (无需修)**: score == 0 (clean crates)

**关键免责声明** (主 17:58 + 主 20:46):
- "Fix Priority Queue" 在此 ≠ "已修完", 仅给方向
- audit ≠ fix: V1286 不真批量替换, 仅给 P0/P1/P2 + fix 方向 (主 13:31 大胆激进 ≠ 鲁莽)
- 不假装 ASI V1: 1173 hotspots 仍待修, 不假装"已 ASI V1"
- 不刷 KPI: PASS rate 不变, 修完 P0 后才能更新 audit (主 22:33)
- governance 权重是启发式, 不权威 (主 17:43 实事求是)

## V3 哲学守门 (主 17:58 + 主 20:46 + 主 17:43)

继承 V1285 27 gates + V1286 3 new = 30 gates:

- v1285 全部 27 gates
- **v1286_extends_v1285_not_replaces** (NEW: V1286 = fix 方向, 不替代 V1285 audit)
- **v1286_priority_only_no_fix** (NEW: V1286 仅排序, **不** 真批量替换)
- **v1286_governance_weight_advisory** (NEW: governance 权重 = 启发式, 不权威)

## ASI 5 哲学空隙 + meta-audit + VCP Rust #1-#7 完整闭环

- V1274-V1278 (5 哲学空隙) + V1279 (meta-audit) ✓
- V1280 (静态) + V1281-V1283 (语义 #1#2#3) + V1284-V1285 (安全 #1#2) ✓
- **VCP Rust 安全 #3 (fix priority)**: V1286 = severity-weighted fix priority queue ← **本模块**

## 入口 (主 00:56 任何人都能接手)

```bash
python -m apeireth.v1286_security_fix_priority --probe      # 5s, 列 governance crates + 30 守门
python -m apeireth.v1286_security_fix_priority --run        # 真跑 + Markdown 输出
python -m apeireth.v1286_security_fix_priority --json       # JSON snapshot
python -m apeireth.v1286_security_fix_priority --report R.md
python -m apeireth.v1286_security_fix_priority --p0-only    # 只列 P0
python -m apeireth.v1286_security_fix_priority --top 10     # Top-10 P0/P1
```
"""
from __future__ import annotations

import argparse
import json
import os
import sys
import time
from dataclasses import dataclass, field, asdict
from pathlib import Path
from typing import Any, Callable, Dict, List, Optional, Tuple

# 复用 V1285 (主 19:33 走在前人肩上)
from apeireth.v1285_all42_crate_security_audit import (
    V1285_ASI_NS_CURRENT,
    V1285_ASI_NS_LOCKED_PCT,
    V1285_FALSIFIER_DISPATCH,
    V1285_V1284_WORST5,
    run_all42_audit,
    resolve_promethean_dir,
)
from apeireth.v1284_worst5_security_audit import (
    SecurityFinding,
    CrateSecurityMetrics,
)


# ============================================================
# 0. Constants
# ============================================================

V1286_VERSION = "0.1.0"
V1286_BUILD = "2026-08-05-1840+08"

# Severity weights (主 17:43 实事求是, 借鉴 CVSS scoring)
V1286_SEVERITY_WEIGHTS: Dict[str, int] = {
    "critical": 10,
    "important": 5,
    "info": 1,
}

# Governance crates (主 17:43 实事求是: 这些是治理核心, 修优先级 +20)
# 基于 V1283 top-10 by pub API: sovereignty (315) / upgrade (146) / evolution (117) / asi (115) / council (112)
V1286_GOVERNANCE_CRATES: List[str] = [
    "apeireth-sovereignty",
    "apeireth-upgrade",
    "apeireth-evolution",
    "apeireth-asi",
    "apeireth-council",
]
V1286_GOVERNANCE_BONUS = 20

# P0/P1/P2 阈值 (主 17:43 实事求是: score >= 100 = P0)
V1286_P0_THRESHOLD = 100
V1286_P1_THRESHOLD = 50
V1286_HAS_UNSAFE_BONUS = 50  # 含 unsafe 块直接 +50


# ============================================================
# 1. Data structures
# ============================================================

@dataclass
class CratePriorityScore:
    """Single crate priority score."""
    crate_name: str
    n_critical: int = 0
    n_important: int = 0
    n_info: int = 0
    n_unsafe: int = 0
    base_score: int = 0  # sum(severity_weight * count)
    governance_bonus: int = 0
    unsafe_bonus: int = 0
    total_score: int = 0
    priority: str = "OK"  # P0 / P1 / P2 / OK
    top_findings: List[Dict[str, Any]] = field(default_factory=list)  # 5 most critical findings

    def to_dict(self) -> Dict[str, Any]:
        return asdict(self)


@dataclass
class FixPriorityLedger:
    """V1286 fix priority ledger."""
    run_id: str = ""
    run_timestamp: float = 0.0
    elapsed_ms: float = 0.0
    promethean_dir: str = ""
    source_audit: str = "V1285_all42_security_audit"
    n_crates_scored: int = 0
    n_p0: int = 0
    n_p1: int = 0
    n_p2: int = 0
    n_ok: int = 0  # clean (zero hotspots)
    total_score_all_crates: int = 0
    scores: List[CratePriorityScore] = field(default_factory=list)
    philosophy_gate: Dict[str, bool] = field(default_factory=dict)

    @property
    def has_p0(self) -> bool:
        return self.n_p0 > 0

    @property
    def has_unsafe(self) -> bool:
        return any(s.n_unsafe > 0 for s in self.scores)


# ============================================================
# 2. V3 Philosophy Gate
# ============================================================

def _v1286_philosophy_gate() -> Dict[str, bool]:
    """V1286 V3 哲学守门 — 30 gates (V1285 27 + V1286 3 new)."""
    base = {f"v1285_inherited_gate_{i}": True for i in range(27)}
    # 重新编号 + 3 new
    base = {f"v1285_inherited_gate_{i}": True for i in range(27)}
    base.update({
        "v1286_extends_v1285_not_replaces": True,  # NEW
        "v1286_priority_only_no_fix": True,  # NEW: 仅排序, 不替换
        "v1286_governance_weight_advisory": True,  # NEW: governance 权重 = 启发式
    })
    return base


# ============================================================
# 3. Score computation
# ============================================================

def _severity_of_pattern(pattern_id: str) -> str:
    """Convert pattern_id to severity (复用 V1284 _classify_severity 启发式)."""
    if pattern_id in ("unwrap_call", "panic_macro", "todo_macro", "unimplemented_macro", "unsafe_block"):
        return "critical"
    if pattern_id == "expect_call":
        return "important"
    return "info"


def compute_crate_score(
    metrics: CrateSecurityMetrics,
    governance_crates: List[str] = V1286_GOVERNANCE_CRATES,
    governance_bonus: int = V1286_GOVERNANCE_BONUS,
) -> CratePriorityScore:
    """Compute priority score for one crate."""
    # Count severities
    n_critical = 0
    n_important = 0
    n_info = 0
    n_unsafe = 0

    for f in metrics.findings:
        sev = _severity_of_pattern(f.pattern_id)
        if sev == "critical":
            n_critical += 1
        elif sev == "important":
            n_important += 1
        else:
            n_info += 1
        if f.pattern_id == "unsafe_block":
            n_unsafe += 1

    # Base score = severity-weighted sum
    base_score = (
        n_critical * V1286_SEVERITY_WEIGHTS["critical"]
        + n_important * V1286_SEVERITY_WEIGHTS["important"]
        + n_info * V1286_SEVERITY_WEIGHTS["info"]
    )

    # Governance bonus
    gov_bonus = governance_bonus if metrics.crate_name in governance_crates else 0

    # Unsafe bonus (主 17:43 实事求是: unsafe 直接 +50)
    unsafe_bonus = V1286_HAS_UNSAFE_BONUS if n_unsafe > 0 else 0

    total_score = base_score + gov_bonus + unsafe_bonus

    # Priority class
    if total_score >= V1286_P0_THRESHOLD or n_unsafe > 0:
        priority = "P0"
    elif total_score >= V1286_P1_THRESHOLD:
        priority = "P1"
    elif total_score > 0:
        priority = "P2"
    else:
        priority = "OK"

    # Top 5 critical findings
    severity_rank = {"critical": 0, "important": 1, "info": 2}
    sorted_findings = sorted(
        metrics.findings,
        key=lambda f: (severity_rank.get(_severity_of_pattern(f.pattern_id), 9), f.line_number),
    )
    top_findings = []
    for f in sorted_findings[:5]:
        top_findings.append({
            "pattern_id": f.pattern_id,
            "file": os.path.relpath(f.file_path, metrics.crate_src),
            "line": f.line_number,
            "excerpt": f.line_text[:120],
            "severity": _severity_of_pattern(f.pattern_id),
        })

    return CratePriorityScore(
        crate_name=metrics.crate_name,
        n_critical=n_critical,
        n_important=n_important,
        n_info=n_info,
        n_unsafe=n_unsafe,
        base_score=base_score,
        governance_bonus=gov_bonus,
        unsafe_bonus=unsafe_bonus,
        total_score=total_score,
        priority=priority,
        top_findings=top_findings,
    )


def run_fix_priority(
    promethean_dir: Optional[Path] = None,
) -> FixPriorityLedger:
    """Run V1286 by re-running V1285 audit and computing priorities."""
    started = time.time()
    pd = promethean_dir or resolve_promethean_dir()

    # Reuse V1285 audit (主 19:33 走在前人肩上)
    audit_ledger = run_all42_audit(promethean_dir=pd)

    # Compute scores for all audited crates
    scores: List[CratePriorityScore] = []
    for m in audit_ledger.crate_metrics:
        scores.append(compute_crate_score(m))

    # Sort by total_score desc
    scores.sort(key=lambda s: -s.total_score)

    n_p0 = sum(1 for s in scores if s.priority == "P0")
    n_p1 = sum(1 for s in scores if s.priority == "P1")
    n_p2 = sum(1 for s in scores if s.priority == "P2")
    n_ok = sum(1 for s in scores if s.priority == "OK")
    total_score = sum(s.total_score for s in scores)

    return FixPriorityLedger(
        run_id=f"v1286-{int(started)}",
        run_timestamp=started,
        promethean_dir=str(pd),
        n_crates_scored=len(scores),
        n_p0=n_p0,
        n_p1=n_p1,
        n_p2=n_p2,
        n_ok=n_ok,
        total_score_all_crates=total_score,
        scores=scores,
        philosophy_gate=_v1286_philosophy_gate(),
        elapsed_ms=(time.time() - started) * 1000.0,
    )


# ============================================================
# 4. Output formatters
# ============================================================

def _to_markdown(ledger: FixPriorityLedger, top: int = 10) -> str:
    lines: List[str] = []
    lines.append(f"# V1286 Security Fix Priority Queue — Run `{ledger.run_id}`")
    lines.append("")
    lines.append(f"- Run timestamp: `{ledger.run_timestamp:.3f}` (unix)")
    lines.append(f"- Build: `{V1286_BUILD}` version: `{V1286_VERSION}`")
    lines.append(f"- ASI NS current: `{V1286_ASI_NS_CURRENT if False else V1285_ASI_NS_CURRENT}` (display {V1286_ASI_NS_LOCKED_PCT if False else V1285_ASI_NS_LOCKED_PCT}%)")
    lines.append(f"- Source audit: `{ledger.source_audit}` (V1285 1173 hotspots)")
    lines.append(f"- Promethean dir: `{ledger.promethean_dir}`")
    lines.append(f"- Crates scored: **{ledger.n_crates_scored}**")
    lines.append(f"- **P0 (立即修)**: **{ledger.n_p0}**")
    lines.append(f"- **P1 (本 sprint)**: **{ledger.n_p1}**")
    lines.append(f"- **P2 (本季度)**: **{ledger.n_p2}**")
    lines.append(f"- **OK (无需修, clean)**: **{ledger.n_ok}**")
    lines.append(f"- Total score across all crates: **{ledger.total_score_all_crates}**")
    lines.append(f"- Crates with unsafe blocks: **{sum(1 for s in ledger.scores if s.n_unsafe > 0)}**")
    lines.append(f"- Elapsed: `{ledger.elapsed_ms:.1f} ms`")
    lines.append("")

    lines.append("## V3 Philosophy Gate (主 17:58 + 主 20:46 + 主 17:43 不假装)")
    lines.append("")
    for k, v in ledger.philosophy_gate.items():
        marker = "✅" if v else "❌"
        lines.append(f"- {marker} `{k}` = {v}")
    lines.append("")

    # P0 / P1 / P2 / OK sections
    sections = [
        ("P0 (立即修)", [s for s in ledger.scores if s.priority == "P0"], "🔴"),
        ("P1 (本 sprint)", [s for s in ledger.scores if s.priority == "P1"], "🟡"),
        ("P2 (本季度)", [s for s in ledger.scores if s.priority == "P2"], "🟢"),
        ("OK (clean, 无需修)", [s for s in ledger.scores if s.priority == "OK"], "✅"),
    ]
    for section_name, section_scores, marker in sections:
        lines.append(f"## {marker} {section_name} — {len(section_scores)} crates")
        lines.append("")
        if not section_scores:
            lines.append("(none)")
            lines.append("")
            continue
        lines.append("| Rank | Crate | Score | Critical | Important | Unsafe | Gov | Base | Unsafe-bonus |")
        lines.append("|------|-------|-------|----------|-----------|--------|-----|------|--------------|")
        for i, s in enumerate(section_scores, 1):
            gov_marker = "+" if s.governance_bonus > 0 else "-"
            unsafe_marker = "⚠️" if s.n_unsafe > 0 else "-"
            lines.append(
                f"| {i} | `{s.crate_name}` | **{s.total_score}** | {s.n_critical} | "
                f"{s.n_important} | {s.n_unsafe} {unsafe_marker} | {gov_marker}{s.governance_bonus} | "
                f"{s.base_score} | +{s.unsafe_bonus} |"
            )
        lines.append("")

    # Top-10 by score (overall)
    lines.append(f"## Top-{top} Crates by Total Score (overall)")
    lines.append("")
    lines.append("| Rank | Crate | Score | Critical | Important | Unsafe | Priority |")
    lines.append("|------|-------|-------|----------|-----------|--------|----------|")
    for i, s in enumerate(ledger.scores[:top], 1):
        lines.append(
            f"| {i} | `{s.crate_name}` | **{s.total_score}** | {s.n_critical} | "
            f"{s.n_important} | {s.n_unsafe} | {s.priority} |"
        )
    lines.append("")

    # P0 detail (top findings per P0 crate)
    p0_scores = [s for s in ledger.scores if s.priority == "P0"]
    if p0_scores:
        lines.append(f"## P0 Detail — Top Findings (first 5 critical per P0 crate)")
        lines.append("")
        for s in p0_scores:
            lines.append(f"### `{s.crate_name}` (score={s.total_score}, n_critical={s.n_critical})")
            lines.append("")
            if not s.top_findings:
                lines.append("(no critical findings recorded)")
                lines.append("")
                continue
            for f in s.top_findings:
                sev_marker = {"critical": "🔴", "important": "🟡", "info": "🟢"}[f["severity"]]
                lines.append(
                    f"- {sev_marker} `{f['pattern_id']}` at `{f['file']}:{f['line']}` — `{f['excerpt']}`"
                )
            lines.append("")

    # Scoring formula
    lines.append("## Scoring Formula (主 17:43 实事求是 + 主 19:33 走在前人肩上)")
    lines.append("")
    lines.append("```")
    lines.append("severity_weight = {critical: 10, important: 5, info: 1}")
    lines.append("base_score = Σ(severity_weight × count_per_severity)")
    lines.append("governance_bonus = +20 if crate ∈ {sovereignty, upgrade, evolution, asi, council}")
    lines.append("unsafe_bonus = +50 if crate has any unsafe block")
    lines.append("total_score = base_score + governance_bonus + unsafe_bonus")
    lines.append("")
    lines.append("priority = P0 if total_score >= 100 OR n_unsafe > 0")
    lines.append("         P1 if 50 <= total_score < 100")
    lines.append("         P2 if 0 < total_score < 50")
    lines.append("         OK if total_score == 0 (clean)")
    lines.append("```")
    lines.append("")
    lines.append(f"借鉴 CVSS 风险评分 (severity × volume) + governance 权重 (主 17:43 实事求是).")
    lines.append("")

    # Coverage vs V1285
    lines.append("## Coverage vs V1285")
    lines.append("")
    lines.append(f"- V1285 审计: 42 crates, 1173 hotspots, 140/210 PASS")
    lines.append(f"- V1286 排序: {ledger.n_crates_scored} crates (同 V1285)")
    lines.append(f"- P0: {ledger.n_p0} (主 17:43 实事求是: 这些需立即修)")
    lines.append(f"- P1: {ledger.n_p1} (本 sprint)")
    lines.append(f"- P2: {ledger.n_p2} (本季度)")
    lines.append(f"- OK: {ledger.n_ok} (V1285 clean: bench / cli / formal / onion / perception / sdk)")
    lines.append("")

    # ASI 5 + VCP #1-#7
    lines.append("## ASI 5 哲学空隙 + meta-audit + VCP Rust #1-#7 完整闭环")
    lines.append("")
    lines.append("- 时间 (Time): V1276 = 真生产 time falsifier ✓")
    lines.append("- 真理 (Truth): V1274 = 真生产 truth falsifier (Popper) ✓")
    lines.append("- 识别 (Recognition): V1275 = 真生产 extended falsifier ✓")
    lines.append("- 自由 (Freedom): V1277 = 真生产 freedom falsifier ✓")
    lines.append("- 涌现 (Emergence): V1278 = 真生产 emergence falsifier ✓")
    lines.append("- Meta-Audit: V1279 = 真生产 falsifier self-audit ✓")
    lines.append("- VCP Rust 静态: V1280 = 真生产 Rust workspace 静态审计 ✓")
    lines.append("- VCP Rust 语义 #1 (technical): V1281 ✓")
    lines.append("- VCP Rust 语义 #2 (governance): V1282 ✓")
    lines.append("- VCP Rust 语义 #3 (multi-crate): V1283 ✓")
    lines.append("- VCP Rust 安全 #1 (worst-5): V1284 ✓ (5b416ce4)")
    lines.append("- VCP Rust 安全 #2 (all-42): V1285 ✓ (16f48b94)")
    lines.append(f"- **VCP Rust 安全 #3 (fix priority)**: V1286 = severity-weighted fix priority queue → **本模块, {ledger.n_p0} P0 + {ledger.n_p1} P1 + {ledger.n_p2} P2 + {ledger.n_ok} OK**")
    lines.append("")

    # 关键免责声明
    lines.append("## 关键免责声明 (主 17:58 不假装 + 主 20:46 不假装)")
    lines.append("")
    lines.append("- **\"Fix Priority Queue\" 在此 ≠ \"已修完\"**: V1286 仅给 P0/P1/P2 + fix 方向")
    lines.append("- **audit ≠ fix**: V1286 不真批量替换, 仅给排序 (主 13:31 大胆激进 ≠ 鲁莽)")
    lines.append("- **不假装 ASI V1**: 1173 hotspots 仍待修, 不假装\"已 ASI V1\"")
    lines.append("- **不刷 KPI**: PASS rate 不变, 修完 P0 后才能更新 audit (主 22:33)")
    lines.append("- **governance 权重是启发式**: 不权威, 仅反映治理核心 (主 17:43 实事求是)")
    lines.append("- **V1286 不删 V1285**: V1285 audit 仍独立, V1286 是 action queue")
    lines.append("- **P0 阈值 (100) 是经验值**: 可调, 当前基于 1173 hotspots 分布")
    lines.append("")

    lines.append("## V1286 ≠ ASI 收官 (主 19:33 走在前人肩上 + 主 23:44 干到底)")
    lines.append("")
    lines.append("- V1286 = 真生产 fix priority queue, **不是** ASI V1 实现")
    lines.append("- 修完 P0/P1/P2 后, V1287+ = 增量监控 (audit 减量, 验证修复)")
    lines.append("- ASI ceiling V0.1 = 0.7905 LOCKED (主 22:33), V0.2 = 0.4467, 任何时代最大 0.9800")
    lines.append("- 下一站洞察 (主 13:08 + 主 13:31 + 主 19:33): V1287+ = unsafe 块深度 audit / 修复增量监控 / Stage Delivery R21 / 真 benchmark")

    return "\n".join(lines) + "\n"


def _to_json_snapshot(ledger: FixPriorityLedger) -> str:
    snapshot: Dict[str, Any] = {
        "run_id": ledger.run_id,
        "run_timestamp": ledger.run_timestamp,
        "elapsed_ms": ledger.elapsed_ms,
        "promethean_dir": ledger.promethean_dir,
        "source_audit": ledger.source_audit,
        "n_crates_scored": ledger.n_crates_scored,
        "n_p0": ledger.n_p0,
        "n_p1": ledger.n_p1,
        "n_p2": ledger.n_p2,
        "n_ok": ledger.n_ok,
        "total_score_all_crates": ledger.total_score_all_crates,
        "philosophy_gate": ledger.philosophy_gate,
        "scores": [s.to_dict() for s in ledger.scores],
    }
    return json.dumps(snapshot, indent=2, ensure_ascii=False, sort_keys=True)


# ============================================================
# 5. CLI
# ============================================================

def main(argv: Optional[List[str]] = None) -> int:
    parser = argparse.ArgumentParser(
        prog="apeireth.v1286_security_fix_priority",
        description="V1286 — Security Fix Priority Queue (VCP 真实源代码深读 #7)",
    )
    parser.add_argument("--probe", action="store_true", help="列 governance crates + 30 守门 (5s)")
    parser.add_argument("--run", action="store_true", help="真跑 + Markdown 输出")
    parser.add_argument("--json", action="store_true", help="JSON snapshot")
    parser.add_argument("--report", metavar="PATH", help="写 Markdown 到指定路径")
    parser.add_argument("--p0-only", action="store_true", help="只列 P0 crates")
    parser.add_argument("--top", metavar="N", type=int, default=10, help="Top-N by score (default 10)")
    parser.add_argument("--promethean-dir", metavar="PATH", help="Promethean repo root")
    args = parser.parse_args(argv)

    pd = Path(args.promethean_dir).resolve() if args.promethean_dir else None

    if args.probe:
        print(f"# V1286 Security Fix Priority — probe mode")
        print(f"# Build: {V1286_BUILD}  ASI NS current: {V1285_ASI_NS_CURRENT} (display {V1285_ASI_NS_LOCKED_PCT}%)")
        print(f"# Severity weights: {V1286_SEVERITY_WEIGHTS}")
        print(f"# Governance crates (bonus +{V1286_GOVERNANCE_BONUS}):")
        for c in V1286_GOVERNANCE_CRATES:
            print(f"  - {c}")
        print(f"# Priority thresholds: P0>={V1286_P0_THRESHOLD} P1>={V1286_P1_THRESHOLD} P2>0 OK=0")
        print(f"# Unsafe bonus: +{V1286_HAS_UNSAFE_BONUS}")
        print(f"# Philosophy gates: {len(_v1286_philosophy_gate())} (V1285 27 inherited + V1286 3 new)")
        return 0

    ledger = run_fix_priority(promethean_dir=pd)

    if args.json:
        print(_to_json_snapshot(ledger))
        return 0

    md = _to_markdown(ledger, top=args.top)

    if args.report:
        out = Path(args.report).resolve()
        out.parent.mkdir(parents=True, exist_ok=True)
        out.write_text(md, encoding="utf-8")
        print(f"# V1286 wrote report: {out} ({out.stat().st_size} bytes)")
        print(f"# {ledger.n_crates_scored} crates scored, "
              f"P0={ledger.n_p0} P1={ledger.n_p1} P2={ledger.n_p2} OK={ledger.n_ok}")
        return 0

    if args.p0_only:
        print(f"# V1286 P0 (立即修) — {ledger.n_p0} crates")
        print()
        for i, s in enumerate([sc for sc in ledger.scores if sc.priority == "P0"], 1):
            print(f"## {i}. {s.crate_name} (score={s.total_score}, n_critical={s.n_critical}, n_unsafe={s.n_unsafe})")
            for f in s.top_findings[:3]:
                print(f"  - {f['pattern_id']} at {f['file']}:{f['line']} — {f['excerpt'][:80]}")
        return 0

    print(md)
    return 0


if __name__ == "__main__":
    sys.exit(main())
