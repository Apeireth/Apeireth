"""V1151 — ASI + VCP + Multi-Agent 真生产统一仪表盘 (主 06:15 V1053+ 真生产 + 主 22:33 ASI 北极星 + 主 17:43 实事求是 + 主 19:33 走在前人经验上 + 主 13:31 大胆激进 + 主 17:58+20:46 不假装 + 主 23:44 干到底 + 主 00:56 任何人都能接手 + 主 00:44 质量工程化).

V1146 (ASI 真生产一键集成) + V1148 (VCP 5 仓库真跑) + V1149 (Multi-Agent Role + DAG)
→ V1151 一个 CLI 出 ASI 真生产 + VCP 真读 + Multi-Agent 真跑 + 真统一报告

主 00:56 任何人都能接手:
  > 一行命令真生产, 不要 50 步
  > python -m apeireth.v1151_unified_production_dashboard --all --report

主 17:43 实事求是:
- 不假装"模块存在 = 真生产": 每个 step 真标 status (R/P/M/X/H)
- 不假装"V1146 = V1148 = V1149 整合 = ASI 升级": 整合是真方便, 不是 ASI 跳升
- 不假装"3 step 全跑 = 必成功": 各自 step 独立 status, 整合看 n_real/n_partial/n_mock

主 19:33 走在前人经验上:
- V1146 = 真生产框架 (orchestrator)
- V1148 = VCP 真借鉴启发
- V1149 = 多代理抽象
- V1151 = 真整合三者的输出 → 统一 dashboard

真生产 6 组件 (主 00:44 质量工程化):
 1. UnifiedProductionReport  — 统一真生产报告 dataclass
 2. _run_asi_production      — 真跑 V1146 v1146_run_all (不重实现, 真调)
 3. _run_vcp_deep_read       — 真跑 V1148 _run_all_5_repos_real (不重实现, 真调)
 4. _run_multi_agent         — 真跑 V1149 run_multi_agent (不重实现, 真调)
 5. _render_unified_md       — 真产统一 Markdown 报告
 6. main CLI                  — --all / --asi / --vcp / --agent / --json / --report

Usage:
    python -m apeireth.v1151_unified_production_dashboard --all --report
    python -m apeireth.v1151_unified_production_dashboard --asi --json
    python -m apeireth.v1151_unified_production_dashboard --vcp --json
    python -m apeireth.v1151_unified_production_dashboard --agent --task "Build X"
"""

from __future__ import annotations

import argparse
import json
import time
import uuid
from dataclasses import dataclass, field, asdict
from pathlib import Path
from typing import Any, Dict, List, Optional

# 真调三个既有真生产模块 (主 17:43 实事求是: 不重实现, 真调)
from apeireth import v1146_asi_one_click_production as v1146
from apeireth import v1148_vcp_5_repos_real_run as v1148
from apeireth import v1149_multi_agent_role_dag as v1149

V1151_VERSION = "0.1.0"

# ============================================================================
# 统一报告 dataclass (主 00:44 质量工程化)
# ============================================================================

@dataclass
class SubsystemStatus:
    """V1151 子系统状态."""
    name: str
    status: str  # R/P/M/X/H
    n_steps: int
    n_real: int
    n_value: float  # 主要数值 (如 ASI score)
    duration_ms: int
    error: str = ""

    def to_dict(self) -> Dict[str, Any]:
        return asdict(self)


@dataclass
class UnifiedProductionReport:
    """V1151 统一真生产报告."""
    snapshot_id: str
    started_at: float
    finished_at: float
    asi_v1151: SubsystemStatus       # V1146 真跑
    vcp_v1151: SubsystemStatus       # V1148 真跑
    multi_agent_v1151: SubsystemStatus  # V1149 真跑
    overall_success_rate: float      # 3 子系统 平均 R 率
    unified_markdown_path: str      # 真产报告路径
    artifacts: Dict[str, str]       # 子 artifact paths

    def to_dict(self) -> Dict[str, Any]:
        d = asdict(self)
        d["overall_success_rate"] = round(self.overall_success_rate, 4)
        return d


# ============================================================================
# 子系统真跑 (主 17:43 实事求是 — 真调既有模块, 不重实现)
# ============================================================================

def _run_asi_production(timeout_s: float = 30.0) -> SubsystemStatus:
    """V1151 真跑 V1146 ASI 真生产 (主 17:43 实事求是)."""
    t0 = time.time()
    error = ""
    n_real = 0
    n_total = 0
    asi_value = 0.0
    try:
        # 真调 V1146 v1146_run_all (实际跑 6 个 step)
        # V1146 默认会跑 ASI V0.5 17-dim 真测, deployment, benchmark, deep read, philosophy, streamlit
        report = v1146.v1146_run_all()
        n_total = len(report.steps) if hasattr(report, "steps") else 0
        # 算 n_real
        if hasattr(report, "steps"):
            for s in report.steps:
                if str(s.status) == "R" or s.status == v1146.StepStatus.REAL:
                    n_real += 1
        # 找 ASI 数值 (StepStatus.REAL + step_name 含 asi)
        if hasattr(report, "steps"):
            for s in report.steps:
                if "asi" in s.step_name.lower() and s.status == v1146.StepStatus.REAL:
                    asi_value = s.value
                    break
        if asi_value == 0.0 and hasattr(report, "asi_score"):
            asi_value = report.asi_score
    except Exception as e:
        error = f"{type(e).__name__}: {str(e)[:100]}"
    duration_ms = int((time.time() - t0) * 1000)
    status = "R" if n_real > 0 and not error else ("P" if n_real > 0 else "M")
    return SubsystemStatus(
        name="ASI_V1146",
        status=status,
        n_steps=n_total,
        n_real=n_real,
        n_value=asi_value,
        duration_ms=duration_ms,
        error=error,
    )


def _run_vcp_deep_read(timeout_s: float = 5.0, skip_if_no_recent: bool = True) -> SubsystemStatus:
    """V1151 真跑 V1148 VCP 5 仓库 (主 17:43 实事求是).

    skip_if_no_recent: 如果最近 1 小时跑过, skip 重跑 (避免 rate limit)
    """
    t0 = time.time()
    error = ""
    n_real = 0
    n_total = 5
    stars_value = 0
    # 检查 artifact 是否新鲜
    artifact = Path(v1148.ARTIFACT_JSON)
    if skip_if_no_recent and artifact.exists():
        age_min = (time.time() - artifact.stat().st_mtime) / 60
        if age_min < 60:
            # 用已存在 artifact
            try:
                data = json.loads(artifact.read_text(encoding="utf-8"))
                n_real = data.get("n_real", 0)
                stars_value = data.get("total_stars", 0)
                duration_ms = int((time.time() - t0) * 1000)
                return SubsystemStatus(
                    name="VCP_V1148",
                    status="R" if n_real >= n_total else "P",
                    n_steps=n_total,
                    n_real=n_real,
                    n_value=float(stars_value),
                    duration_ms=duration_ms,
                    error=f"loaded from artifact (age={age_min:.1f}min)",
                )
            except Exception as e:
                error = f"artifact load fail: {e}"
    # 真跑 (主 17:43 实事求是: 真调 V1148, 不重实现)
    try:
        summary = v1148._run_all_5_repos_real(timeout_s=timeout_s, sleep_s=0.5)
        n_real = summary.n_real
        stars_value = summary.total_stars
        # 真存 artifact
        v1148._save_artifacts(summary)
    except Exception as e:
        error = f"{type(e).__name__}: {str(e)[:100]}"
    duration_ms = int((time.time() - t0) * 1000)
    status = "R" if n_real >= n_total else "P"
    return SubsystemStatus(
        name="VCP_V1148",
        status=status,
        n_steps=n_total,
        n_real=n_real,
        n_value=float(stars_value),
        duration_ms=duration_ms,
        error=error,
    )


def _run_multi_agent(task: str = "Build a simple HTTP server") -> SubsystemStatus:
    """V1151 真跑 V1149 multi-agent (主 17:43 实事求是)."""
    t0 = time.time()
    error = ""
    n_real = 0
    n_total = 5
    success_rate = 0.0
    try:
        result = v1149.run_multi_agent(task)
        n_real = result.n_done
        success_rate = result.success_rate
    except Exception as e:
        error = f"{type(e).__name__}: {str(e)[:100]}"
    duration_ms = int((time.time() - t0) * 1000)
    status = "R" if n_real >= n_total else "P"
    return SubsystemStatus(
        name="MultiAgent_V1149",
        status=status,
        n_steps=n_total,
        n_real=n_real,
        n_value=success_rate,
        duration_ms=duration_ms,
        error=error,
    )


# ============================================================================
# 统一入口 (主 00:56 任何人都能接手)
# ============================================================================

def run_unified_dashboard(
    run_asi: bool = True,
    run_vcp: bool = True,
    run_agent: bool = True,
    agent_task: str = "Build a simple HTTP server",
    vcp_timeout: float = 5.0,
) -> UnifiedProductionReport:
    """V1151 真统一 dashboard (主 00:56 任何人都能接手)."""
    started = time.time()
    snapshot_id = f"v1151-{uuid.uuid4().hex[:8]}"

    asi_status = _run_asi_production() if run_asi else SubsystemStatus("ASI_V1146", "X", 0, 0, 0.0, 0)
    vcp_status = _run_vcp_deep_read(timeout_s=vcp_timeout) if run_vcp else SubsystemStatus("VCP_V1148", "X", 0, 0, 0.0, 0)
    agent_status = _run_multi_agent(agent_task) if run_agent else SubsystemStatus("MultiAgent_V1149", "X", 0, 0, 0.0, 0)

    rates = []
    for s in [asi_status, vcp_status, agent_status]:
        if s.n_steps > 0:
            rates.append(s.n_real / s.n_steps)
    overall_rate = sum(rates) / len(rates) if rates else 0.0

    finished = time.time()
    md_path = _render_unified_md(
        snapshot_id=snapshot_id,
        asi=asi_status,
        vcp=vcp_status,
        agent=agent_status,
        overall_rate=overall_rate,
        duration_ms=int((finished - started) * 1000),
    )
    artifacts = {
        "v1148_json": str(v1148.ARTIFACT_JSON),
        "v1148_md": str(v1148.ARTIFACT_MD),
    }
    return UnifiedProductionReport(
        snapshot_id=snapshot_id,
        started_at=started,
        finished_at=finished,
        asi_v1151=asi_status,
        vcp_v1151=vcp_status,
        multi_agent_v1151=agent_status,
        overall_success_rate=overall_rate,
        unified_markdown_path=md_path,
        artifacts=artifacts,
    )


def _render_unified_md(
    snapshot_id: str,
    asi: SubsystemStatus,
    vcp: SubsystemStatus,
    agent: SubsystemStatus,
    overall_rate: float,
    duration_ms: int,
) -> str:
    """V1151 真产统一 Markdown 报告 (主 00:56 任何人都能接手)."""
    lines = [
        f"# V1151 ASI + VCP + Multi-Agent 统一真生产报告",
        "",
        f"- snapshot_id: `{snapshot_id}`",
        f"- V1151_VERSION: `{V1151_VERSION}`",
        f"- 真实运行时间: {duration_ms/1000:.1f}s",
        f"- **overall_success_rate**: **{overall_rate*100:.1f}%**",
        "",
        "## 3 真生产子系统汇总",
        "",
        "| 子系统 | 来自 | status | n_steps | n_real | value | duration_ms |",
        "|--------|------|--------|---------|--------|-------|-------------|",
        f"| ASI 真生产 | V1146 | {asi.status} | {asi.n_steps} | {asi.n_real} | {asi.n_value:.4f} | {asi.duration_ms} |",
        f"| VCP 5 仓库真跑 | V1148 | {vcp.status} | {vcp.n_steps} | {vcp.n_real} | {vcp.n_value:.0f} | {vcp.duration_ms} |",
        f"| Multi-Agent 真跑 | V1149 | {agent.status} | {agent.n_steps} | {agent.n_real} | {agent.n_value:.4f} | {agent.duration_ms} |",
        "",
        "## V3 哲学守门 (主 17:58 + 主 20:46 + 主 17:43)",
        "",
        "- ✅ 不假装 3 子系统都 R = ASI 升级 (整合是方便, ASI 是更大目标)",
        "- ✅ 不假装 V1146 = V1148 = V1149 (各自不同任务, 整合不掩盖差异)",
        "- ✅ 不假装 V1151 = 新真生产 (V1151 是 wrapper, 真正生产在 V1146/V1148/V1149)",
        "- ✅ 不假装 overall_success_rate = ASI score (只是 3 子系统的 R 率平均)",
        "- ✅ 不假装 V1151 默认全跑 (V1151 默认全跑, 但每个子系统独立 try-except)",
        "",
        "## 不假装清单 (主 17:43 实事求是)",
        "",
        f"- ASI V0.5 当前真测 = **{asi.n_value:.4f}** (来自 V1146/V1144)",
        f"- ASI 北极星 = **0.9800**, gap = {0.9800 - asi.n_value:.4f}",
        f"- VCP 5 仓库 total_stars = **{int(vcp.n_value):,}** (来自 V1148 真跑)",
        f"- Multi-Agent success_rate = **{agent.n_value*100:.1f}%** (来自 V1149 真跑)",
        "",
        "## ASI 北极星路径 (主 22:33)",
        "",
        "- V1146 ASI V0.5 17-dim → 0.8532 (gap 0.1268)",
        "- V1151 整合 = 测量方便, 不是 ASI 跳升",
        "- V1150 (下一步) ASI V0.6 formal spec = 北极星 0.98 真路径",
        "",
        "---",
        "",
        f"_V1151 整合 by 楚零 (主 00:56 任何人都能接手 + 主 00:44 质量工程化)._",
    ]
    return "\n".join(lines)


def _save_unified_md(content: str, snapshot_id: str) -> str:
    """V1151 真存 unified markdown."""
    out_dir = Path(__file__).resolve().parent.parent / "artifacts"
    out_dir.mkdir(parents=True, exist_ok=True)
    path = out_dir / f"v1151_unified_{snapshot_id}.md"
    path.write_text(content, encoding="utf-8")
    return str(path)


# ============================================================================
# V1151 Philosophy Guard (主 17:58 + 主 20:46)
# ============================================================================

V1151_GUARDS: Dict[str, str] = {
    "v1151_is_wrapper_not_asi_upgrade": (
        "V1151 是 V1146/V1148/V1149 的统一 wrapper, 不假装 = ASI 升级. "
        "ASI 是更大目标 (主 22:33 北极星), V1151 只整合不创造."
    ),
    "subsystem_status_is_truth": (
        "V1151 子系统 status (R/P/M/X/H) 是真跑结果, 不假装全 R. "
        "每个子系统独立 try-except, 失败也诚实记录."
    ),
    "overall_rate_is_not_asi_score": (
        "V1151 overall_success_rate = 3 子系统 R 率平均, "
        "不是 ASI 综合 score. ASI 北极星是真生产目标, 不是 average."
    ),
    "v1151_borrows_v1146_v1148_v1149": (
        "V1151 真调 V1146 (ASI orchestrator) + V1148 (VCP run) + V1149 (agent run). "
        "不重实现, 真调. (主 19:33 走在前人经验上)"
    ),
    "v1151_one_click_anyone_can_run": (
        "V1151 = 一行命令真生产 (主 00:56 任何人都能接手). "
        "python -m apeireth.v1151_unified_production_dashboard --all --report"
    ),
}


# ============================================================================
# CLI (主 00:56 任何人都能接手)
# ============================================================================

def main(argv: Optional[List[str]] = None) -> int:
    parser = argparse.ArgumentParser(description="V1151 ASI + VCP + Multi-Agent 统一真生产 dashboard")
    parser.add_argument("--all", action="store_true", help="跑全部 3 子系统 (default)")
    parser.add_argument("--asi", action="store_true", help="只跑 ASI 真生产 (V1146)")
    parser.add_argument("--vcp", action="store_true", help="只跑 VCP 5 仓库 (V1148)")
    parser.add_argument("--agent", action="store_true", help="只跑 Multi-Agent (V1149)")
    parser.add_argument("--task", type=str, default="Build a simple HTTP server", help="multi-agent task")
    parser.add_argument("--vcp-timeout", type=float, default=5.0, help="VCP per-HTTP timeout")
    parser.add_argument("--report", action="store_true", help="print Markdown report")
    parser.add_argument("--json", action="store_true", help="print JSON result")
    parser.add_argument("--save", action="store_true", help="save unified Markdown to artifacts/")
    args = parser.parse_args(argv)

    # 决定跑哪些
    run_asi = args.asi or args.all or not (args.asi or args.vcp or args.agent)
    run_vcp = args.vcp or args.all or not (args.asi or args.vcp or args.agent)
    run_agent = args.agent or args.all or not (args.asi or args.vcp or args.agent)

    print(f"V1151 starting: asi={run_asi} vcp={run_vcp} agent={run_agent}")

    report = run_unified_dashboard(
        run_asi=run_asi,
        run_vcp=run_vcp,
        run_agent=run_agent,
        agent_task=args.task,
        vcp_timeout=args.vcp_timeout,
    )

    print(f"\n=== V1151 真跑完成 ===")
    print(f"  snapshot_id: {report.snapshot_id}")
    print(f"  ASI: {report.asi_v1151.status} (R={report.asi_v1151.n_real}/{report.asi_v1151.n_steps}, value={report.asi_v1151.n_value:.4f})")
    print(f"  VCP: {report.vcp_v1151.status} (R={report.vcp_v1151.n_real}/{report.vcp_v1151.n_steps}, stars={int(report.vcp_v1151.n_value)})")
    print(f"  Agent: {report.multi_agent_v1151.status} (R={report.multi_agent_v1151.n_real}/{report.multi_agent_v1151.n_steps}, success_rate={report.multi_agent_v1151.n_value*100:.1f}%)")
    print(f"  overall_success_rate: {report.overall_success_rate*100:.1f}%")

    if args.save:
        # 重新渲染 + 存
        md = _render_unified_md(
            snapshot_id=report.snapshot_id,
            asi=report.asi_v1151,
            vcp=report.vcp_v1151,
            agent=report.multi_agent_v1151,
            overall_rate=report.overall_success_rate,
            duration_ms=int((report.finished_at - report.started_at) * 1000),
        )
        path = _save_unified_md(md, report.snapshot_id)
        print(f"  saved: {path}")
        report.unified_markdown_path = path

    if args.report:
        print("\n=== Markdown report ===")
        md = _render_unified_md(
            snapshot_id=report.snapshot_id,
            asi=report.asi_v1151,
            vcp=report.vcp_v1151,
            agent=report.multi_agent_v1151,
            overall_rate=report.overall_success_rate,
            duration_ms=int((report.finished_at - report.started_at) * 1000),
        )
        print(md)

    if args.json:
        print("\n=== JSON result ===")
        print(json.dumps(report.to_dict(), indent=2, ensure_ascii=False))

    return 0


if __name__ == "__main__":
    raise SystemExit(main())