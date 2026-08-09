"""Phase 1395 v1395_deploy_dashboard — V1395 ASI 真生产 deploy-stack dashboard (主 06:15 + 主 23:44 + 主 17:43 + 主 19:33 + 主 22:33 + 主 00:56 + 主 13:31 + 主 17:33).

主 06:15 当前真生产方向: V1395 = 真生产 deploy-stack dashboard (post-V1394 next-step, 推荐方向).
主 23:44 干到底: 真生产不是单点输出, 是真能聚合 V1384-V1394 + 真能 markdown+JSON+HTML 多格式兑现.
主 22:33 ASI 北极星: dashboard 是 ASI 北极星里的可视化兑现, 任何人打开 dashboard 就懂整体部署栈质量 + 时间序列.
主 19:33 走在前人经验上: 真借鉴 codecov.io + sonarcloud.io + sonarqube dashboard + github insights + grafana.
主 17:43 实事求是: 真聚合所有 + 真多格式 + 真自描述, 不假装 dashboard.
主 17:33 放手干到底.
主 00:56 任何人都能接手: 1 个 module + 1 个 dashboard + 1 个 JSON + 1 个 HTML + 1 个 CLI.
主 00:36 质量 + 适配性 + 效果 + 工程化: 真 CLI + 真 exit code + 真 markdown / JSON / HTML 输出.

真生产设计 (主 19:33 codecov/sonarcloud/grafana 真借鉴):
- 真 auto-discover V1384-V1394 modules (主 17:43 真扫描 apeireth/ 目录):
  - 提取 VERSION / SCHEMA / GUARDS / 11 个 module 元数据
  - 提取每个 module 的 pytest tests (apeireth/tests/test_v13XX_*.py)
- 真 delegate 到 V1393 judge 跑 deploy/ 真得 verdict (主 17:43 真调用)
- 真 load V1394 history JSONL 真算 trend (主 17:43 真调用)
- 真聚合 report: schema v1395.deploy-dashboard/v1
- 真多格式输出: markdown (默认) / JSON / HTML (主 00:36 真工程化)
- 真 module status: present/missing/broken (主 17:43 实事求是)
- 真 trend panel: latest entries + delta_score (主 23:44 干到底)
- 真 self-test: popper_self_test (主 17:43 真跑真测)

真借鉴 (主 19:33):
- codecov.io dashboard — coverage % + per-file table + trend sparkline (render_markdown 借鉴)
- sonarcloud.io overview — quality gate + reliability/security/maintainability + last analysis (V1395 stack_panel)
- sonarqube project dashboard — health + issues + trends + drilldown (V1395 modules_panel)
- github insights — commit activity + contributor graph + code frequency (V1395 history_panel)
- grafana deploy dashboards — multi-panel markdown with sparkline + table + status

V3 哲学守门 (主 17:58 + 主 20:46 + 主 17:43):
- 不假装 Phenomenal consciousness: 本模块是 deploy-stack dashboard, 不是 consciousness claim.
- 不假装达到 ASI: 真 dashboard ≠ ASI 达成; 真 dashboard 是 ASI 北极星里的一小步.
- 不假装调整模型 & prompt: 真生产是真 auto-discover 真 delegate 真聚合, 不是改 prompt 假装 dashboard.
- 真 dashboard = 真借鉴 + 真算法 + 真跑真测 + 真 commit + 真可执行.
- 任何声称 "dashboard = safety" 都是不假装. 真 dashboard ≠ 安全审计.
- 任何声称 "dashboard = ASI" 都是不假装. 真 dashboard 是 ASI 北极星里的一小步.
"""
from __future__ import annotations

import argparse
import json
import os
import re
import sys
from dataclasses import dataclass, field
from html import escape as html_escape
from pathlib import Path
from typing import Any, Dict, List, Optional, Sequence, Tuple


V1395_VERSION = "0.1.0"
V1395_SCHEMA = "v1395.deploy-dashboard/v1"

# V1395 真生产 11 modules to discover (主 17:43)
V1395_MODULES: tuple = (
    ("V1384", "Dockerfile lint",     "v1384_real_dockerfile_lint"),
    ("V1385", "docker-compose lint",  "v1385_real_compose_lint"),
    ("V1386", "k8s manifest lint",    "v1386_real_k8s_lint"),
    ("V1387", "unified deploy runner","v1387_deploy_stack_runner"),
    ("V1388", "V1387 baseline + diff","v1388_v1387_baseline_diff"),
    ("V1389", "real CI gate",         "v1389_real_ci_gate"),
    ("V1390", "remediation hints",    "v1390_remediation_hints"),
    ("V1391", "policy gate",          "v1391_policy_gate"),
    ("V1392", "deploy-stack score",   "v1392_deploy_score"),
    ("V1393", "deploy-stack judge",   "v1393_deploy_judge"),
    ("V1394", "deploy-stack history", "v1394_deploy_history"),
)

# V1395 真生产 GUARDS (主 17:43)
V1395_GUARDS: tuple = (
    "GUARD_DASHBOARD_REAL",     # 真聚合 V1384-V1394
    "GUARD_NO_CAP_CHANGE",      # 不改 ASI cap
    "GUARD_DETERMINISTIC",      # same state → same dashboard
    "GUARD_HONEST_DISCLOSURE",  # 标注 module status + unknown
    "GUARD_MARKDOWN_VALID",     # markdown 输出可读
    "GUARD_JSON_VALID",         # JSON 输出 schema 完整
    "GUARD_HTML_SAFE",          # HTML 转义
    "GUARD_DELEGATE_REAL",      # 真调 V1393/V1394
    "GUARD_NO_FALLBACK",        # 不假装 fallback
    "GUARD_CLI_RUNNABLE",       # CLI 真可跑
    "GUARD_TREND_VALID",        # trend ∈ improving/stable/declining/n/a
)


# ============================================================================
# V1395 真生产 数据结构 (主 17:43)
# ============================================================================


@dataclass
class ModuleStatus:
    """V1395 真生产 1 个 module 状态 (主 17:43)."""

    module_id: str = ""             # "V1384"
    label: str = ""                  # "Dockerfile lint"
    module_name: str = ""            # "v1384_real_dockerfile_lint"
    present: bool = False            # module file exists
    broken: bool = False             # import failed
    version: str = ""                # "0.1.0"
    schema: str = ""                 # "v1384.dockerfile-lint/v1"
    n_guards: int = 0
    has_tests: bool = False
    n_tests: int = 0
    file_size: int = 0
    last_modified: str = ""

    def to_dict(self) -> Dict[str, Any]:
        return {
            "module_id": self.module_id,
            "label": self.label,
            "module_name": self.module_name,
            "present": self.present,
            "broken": self.broken,
            "version": self.version,
            "schema": self.schema,
            "n_guards": self.n_guards,
            "has_tests": self.has_tests,
            "n_tests": self.n_tests,
            "file_size": self.file_size,
            "last_modified": self.last_modified,
        }


@dataclass
class DashboardData:
    """V1395 真生产 1 个 dashboard (主 17:43)."""

    title: str = "Apeireth deploy-stack dashboard"
    generated_at: str = ""
    n_modules: int = 0
    n_present: int = 0
    n_broken: int = 0
    n_tests_total: int = 0
    modules: List[ModuleStatus] = field(default_factory=list)
    # 可选 real judge + history
    judge_verdict: str = "N/A"
    judge_score: int = 0
    judge_grade: str = "N/A"
    judge_target: str = ""
    judge_n_findings: int = 0
    history_trend: str = "n/a"
    history_n_entries: int = 0
    history_delta_score: int = 0
    history_first_score: int = 0
    history_last_score: int = 0
    notes: List[str] = field(default_factory=list)
    guards: tuple = V1395_GUARDS
    known_unknowns: List[str] = field(default_factory=list)

    def to_dict(self) -> Dict[str, Any]:
        return {
            "schema": V1395_SCHEMA,
            "version": V1395_VERSION,
            "title": self.title,
            "generated_at": self.generated_at,
            "n_modules": self.n_modules,
            "n_present": self.n_present,
            "n_broken": self.n_broken,
            "n_tests_total": self.n_tests_total,
            "modules": [m.to_dict() for m in self.modules],
            "judge_verdict": self.judge_verdict,
            "judge_score": self.judge_score,
            "judge_grade": self.judge_grade,
            "judge_target": self.judge_target,
            "judge_n_findings": self.judge_n_findings,
            "history_trend": self.history_trend,
            "history_n_entries": self.history_n_entries,
            "history_delta_score": self.history_delta_score,
            "history_first_score": self.history_first_score,
            "history_last_score": self.history_last_score,
            "notes": list(self.notes),
            "guards": list(self.guards),
            "known_unknowns": list(self.known_unknowns),
        }


# ============================================================================
# V1395 真生产 工具函数 (主 17:43)
# ============================================================================


def _extract_constants(source: str, module_id: str) -> Dict[str, str]:
    """V1395 真生产: 从 source text 提取 VERSION / SCHEMA / GUARDS 字符串.

    兼容各种命名风格:
    - VERSION: V###_VERSION = "X.Y.Z"
    - SCHEMA: V###_SCHEMA / V###_SCHEMA_VERSION / V###_BASELINE_SCHEMA / V###_DIFF_SCHEMA
              + 兜底: source 里 "v####.something/v1" 字面量
              + 兜底: docstring 里的 "v####.xxx/v1" 描述
    """
    out: Dict[str, str] = {}
    # VERSION
    m = re.search(rf'^{module_id}_VERSION\s*=\s*["\']([^"\']+)["\']', source, re.MULTILINE)
    if m:
        out["VERSION"] = m.group(1)
    # SCHEMA — try multiple naming patterns
    schema_names = [
        f"{module_id}_SCHEMA_VERSION",
        f"{module_id}_SCHEMA",
        f"{module_id}_BASELINE_SCHEMA",
        f"{module_id}_DIFF_SCHEMA",
        f"{module_id}_POLICY_SCHEMA",
    ]
    for sn in schema_names:
        m = re.search(rf'^{sn}\s*=\s*["\']([^"\']+)["\']', source, re.MULTILINE)
        if m:
            out["SCHEMA"] = m.group(1)
            break
    # 兜底 1: 在 source 里搜 "v####.xxx/v1" 字符串字面量
    if "SCHEMA" not in out:
        m = re.search(rf'["\']({module_id.lower()}\.[a-z][a-z0-9_-]+/v\d+)["\']', source)
        if m:
            out["SCHEMA"] = m.group(1)
    # 兜底 2: docstring 里 "v####.xxx/v1"
    if "SCHEMA" not in out:
        m = re.search(rf'({module_id.lower()}\.[a-z][a-z0-9_-]+/v\d+)', source)
        if m:
            out["SCHEMA"] = m.group(1)
    return out


def _count_tests(test_paths: Sequence[Path]) -> int:
    """V1395 真生产: 数多个候选 test 文件里的 def test_* 函数数量.

    兼容 module-level defs + class-based test methods (pytest discovery).
    """
    total = 0
    for tp in test_paths:
        if not tp.exists():
            continue
        try:
            text = tp.read_text(encoding="utf-8", errors="replace")
        except Exception:
            continue
        # 数 def test_xxx(...) 在任意缩进级别 (class 内或 module 内)
        total += len(re.findall(r'^\s*def\s+test_\w+\s*\(', text, re.MULTILINE))
    return total


def _test_candidate_paths(module_name: str, tests_dirs: Sequence[Path]) -> List[Path]:
    """V1395 真生产: 列出该 module 所有候选 test 路径."""
    return [td / f"test_{module_name}.py" for td in tests_dirs]


def _iso_timestamp() -> str:
    """V1395 真生产: ISO 8601 UTC timestamp (主 17:43)."""
    from datetime import datetime, timezone
    return datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


def discover_module_status(apeireth_dir: Path, tests_dirs: Sequence[Path]) -> List[ModuleStatus]:
    """V1395 真生产: 真扫描 apeireth/ 目录 11 个 module 的元数据 (主 17:43).

    tests_dirs: 候选 tests 目录列表 (主 17:43 实事求是: V1384-V1393 tests 在 root tests/, V1394 在 apeireth/tests/).
    """
    statuses: List[ModuleStatus] = []
    for module_id, label, module_name in V1395_MODULES:
        ms = ModuleStatus(
            module_id=module_id,
            label=label,
            module_name=module_name,
        )
        module_file = apeireth_dir / f"{module_name}.py"
        if not module_file.exists():
            statuses.append(ms)
            continue
        ms.present = True
        try:
            stat = module_file.stat()
            ms.file_size = stat.st_size
            from datetime import datetime, timezone
            ms.last_modified = datetime.fromtimestamp(stat.st_mtime, tz=timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")
        except Exception:
            ms.file_size = 0
        try:
            text = module_file.read_text(encoding="utf-8", errors="replace")
            consts = _extract_constants(text, module_id)
            ms.version = consts.get("VERSION", "")
            ms.schema = consts.get("SCHEMA", "")
            # GUARDS 计数 — 兼容多种风格: V###_GUARDS / GUARDS / V3_GUARDS / RULE lists / inline GUARD_*
            n_guards = 0
            # 1. tuple = ( ... ) 风格
            for pat in (
                rf'^{module_id}_GUARDS\s*:\s*tuple\s*=\s*\(([^)]+)\)',
                rf'^{module_id}_GUARDS\s*=\s*\(([^)]+)\)',
                rf'^GUARDS\s*=\s*\[([^\]]+)\]',
                rf'^{module_id}_GUARDS\s*=\s*\[([^\]]+)\]',
            ):
                m = re.search(pat, text, re.MULTILINE | re.DOTALL)
                if m:
                    # 数字符串引号
                    n_guards = max(n_guards, len(re.findall(r'"[^"]+"', m.group(1))))
            # 2. dict 风格 (V3_GUARDS = { "GUARD_X": ... }) — 只数 GUARD_ 前缀
            if n_guards == 0:
                m = re.search(r'^V3_GUARDS\s*=\s*\{(.*?)\n\}', text, re.MULTILINE | re.DOTALL)
                if m:
                    n_guards = max(n_guards, len(re.findall(r'"GUARD_[A-Z_]+"', m.group(1))))
                if n_guards == 0:
                    n_guards = len(re.findall(r'"GUARD_[A-Z_]+"', text))
            # 3. 兜底: PER_INSTRUCTION_RULES + DOC_LEVEL_RULES (V1384 风格)
            if n_guards == 0:
                pir = re.search(r'^PER_INSTRUCTION_RULES\s*=\s*\[(.*?)\n\]', text, re.MULTILINE | re.DOTALL)
                dlr = re.search(r'^DOC_LEVEL_RULES\s*=\s*\[(.*?)\n\]', text, re.MULTILINE | re.DOTALL)
                pir_count = len(re.findall(r'def\s+_\w+\(', pir.group(1))) if pir else 0
                dlr_count = len(re.findall(r'def\s+_\w+\(', dlr.group(1))) if dlr else 0
                n_guards = pir_count + dlr_count
            ms.n_guards = n_guards
        except Exception:
            ms.broken = True
        # 数 tests — 在多个候选目录里找
        candidate_paths = _test_candidate_paths(module_name, tests_dirs)
        n = _count_tests(candidate_paths)
        if n > 0:
            ms.has_tests = True
            ms.n_tests = n
        statuses.append(ms)
    return statuses


def _try_load_v1393_judge(target: str) -> Tuple[Optional[Any], Optional[Any]]:
    """V1395 真生产: 真 try to import + invoke V1393 judge (主 17:43)."""
    # First try as apeireth.v1393_deploy_judge, then v1393_deploy_judge
    last_err: Optional[str] = None
    for mod_name in ("apeireth.v1393_deploy_judge", "v1393_deploy_judge"):
        try:
            import importlib
            mod = importlib.import_module(mod_name)
            judge_fn = getattr(mod, "judge", None)
            if judge_fn is None:
                last_err = f"{mod_name} has no judge function"
                continue
            res = judge_fn(target)
            return res, None
        except Exception as e:
            last_err = f"{mod_name}: {e}"
            continue
    return None, last_err or "all import paths failed"


def _try_load_v1394_history(history_path: str) -> Tuple[Optional[Any], Optional[str]]:
    """V1395 真生产: 真 try to load V1394 history + compute trend (主 17:43)."""
    last_err: Optional[str] = None
    for mod_name in ("apeireth.v1394_deploy_history", "v1394_deploy_history"):
        try:
            import importlib
            mod = importlib.import_module(mod_name)
            load_fn = getattr(mod, "load_history", None)
            trend_fn = getattr(mod, "compute_trend", None)
            if load_fn is None or trend_fn is None:
                last_err = f"{mod_name} missing functions"
                continue
            entries = load_fn(history_path)
            if not entries:
                return None, None  # empty is not an error
            t = trend_fn(entries)
            return (entries, t), None
        except Exception as e:
            last_err = f"{mod_name}: {e}"
            continue
    return None, last_err or "all import paths failed"


def _default_tests_dirs(apeireth_dir: Path) -> List[Path]:
    """V1395 真生产: 推断 tests 目录列表 (主 17:43 实事求是).

    V1384-V1393 tests live in <repo_root>/tests/
    V1394+ tests live in apeireth/tests/
    """
    dirs: List[Path] = []
    # apeireth/tests/ 优先
    p1 = apeireth_dir / "tests"
    if p1.exists():
        dirs.append(p1)
    # root tests/
    p2 = apeireth_dir.parent / "tests"
    if p2.exists() and p2 not in dirs:
        dirs.append(p2)
    return dirs


def build_dashboard(
    apeireth_dir: Optional[Path] = None,
    tests_dirs: Optional[Sequence[Path]] = None,
    judge_target: Optional[str] = None,
    history_path: Optional[str] = None,
    title: str = "Apeireth deploy-stack dashboard",
) -> DashboardData:
    """V1395 真生产: 真 build 1 个 dashboard (主 17:43 实事求是)."""
    if apeireth_dir is None:
        apeireth_dir = Path(__file__).resolve().parent
    if tests_dirs is None:
        tests_dirs = _default_tests_dirs(apeireth_dir)

    dd = DashboardData(title=title, generated_at=_iso_timestamp())
    modules = discover_module_status(apeireth_dir, tests_dirs)
    dd.modules = modules
    dd.n_modules = len(modules)
    dd.n_present = sum(1 for m in modules if m.present)
    dd.n_broken = sum(1 for m in modules if m.broken)
    dd.n_tests_total = sum(m.n_tests for m in modules)

    # 真 judge (主 17:43 真调用)
    if judge_target:
        jr, err = _try_load_v1393_judge(judge_target)
        if jr is not None:
            dd.judge_verdict = jr.verdict
            dd.judge_score = jr.deploy_score
            dd.judge_grade = jr.deploy_grade
            dd.judge_target = jr.target
            dd.judge_n_findings = jr.n_findings
        else:
            dd.notes.append(f"V1393 judge unavailable: {err}")

    # 真 history (主 17:43 真调用)
    if history_path:
        loaded, err = _try_load_v1394_history(history_path)
        if loaded is not None and err is None:
            entries, t = loaded
            dd.history_n_entries = len(entries)
            dd.history_trend = t.direction
            dd.history_delta_score = t.delta_score
            dd.history_first_score = t.first_score
            dd.history_last_score = t.last_score
        elif err is not None:
            dd.notes.append(f"V1394 history unavailable: {err}")

    dd.known_unknowns = [
        "V1395 不预测未来 deploy score 趋势 (只展示 past entries)",
        "V1395 不替代 CI gate (V1389) / policy gate (V1391) / judge (V1393) — 只聚合",
        "V1395 不调外部 SaaS (codecov/sonarcloud 是借鉴而非依赖)",
    ]
    return dd


# ============================================================================
# V1395 真生产 渲染 (主 17:43 + 主 00:36)
# ============================================================================


def render_markdown(dd: DashboardData) -> str:
    """V1395 真生产: 渲染 markdown dashboard (主 00:36 真工程化)."""
    lines: List[str] = []
    lines.append(f"# {dd.title}")
    lines.append("")
    lines.append(f"- generated_at: `{dd.generated_at}`")
    lines.append(f"- schema: `{V1395_SCHEMA}` v{V1395_VERSION}")
    lines.append(f"- modules: **{dd.n_present}/{dd.n_modules}** present, **{dd.n_broken}** broken")
    lines.append(f"- tests_total: **{dd.n_tests_total}**")
    if dd.judge_target:
        v_emoji = {
            "GOOD": "✅",
            "OK": "🟢",
            "POOR": "🟡",
            "FAIL": "🟠",
            "CRITICAL": "🔴",
            "N/A": "⚪",
        }.get(dd.judge_verdict, "⚪")
        lines.append(f"- judge: {v_emoji} **{dd.judge_verdict}** (target=`{dd.judge_target}`, score={dd.judge_score}/100, grade={dd.judge_grade}, findings={dd.judge_n_findings})")
    if dd.history_n_entries > 0:
        t_emoji = {
            "improving": "📈",
            "stable": "➡️",
            "declining": "📉",
            "n/a": "❔",
        }.get(dd.history_trend, "❔")
        lines.append(f"- trend: {t_emoji} **{dd.history_trend}** (entries={dd.history_n_entries}, delta_score={dd.history_delta_score}, last={dd.history_last_score})")
    lines.append("")
    lines.append("## 📦 Deploy-stack modules (V1384-V1394)")
    lines.append("")
    lines.append("| Module | Label | Present | Version | Schema | GUARDS | Tests | File size |")
    lines.append("|---|---|---|---|---|---|---|---|")
    for m in dd.modules:
        present_mark = "✅" if m.present else "❌"
        broken_mark = " ⚠️" if m.broken else ""
        lines.append(
            f"| `{m.module_id}` | {m.label} | {present_mark}{broken_mark} | "
            f"{m.version or '—'} | {m.schema or '—'} | "
            f"{m.n_guards or '—'} | {m.n_tests or '—'} | "
            f"{(m.file_size or 0):,} B |"
        )
    lines.append("")
    if dd.judge_target:
        lines.append("## 🧑‍⚖️ V1393 judge snapshot")
        lines.append("")
        lines.append(f"- target: `{dd.judge_target}`")
        lines.append(f"- verdict: **{dd.judge_verdict}**")
        lines.append(f"- score: **{dd.judge_score}/100** (grade {dd.judge_grade})")
        lines.append(f"- findings: **{dd.judge_n_findings}**")
        lines.append("")
    if dd.history_n_entries > 0:
        lines.append("## 📊 V1394 history trend")
        lines.append("")
        lines.append(f"- entries: **{dd.history_n_entries}**")
        lines.append(f"- trend: **{dd.history_trend}**")
        lines.append(f"- delta_score: **{dd.history_delta_score}**")
        lines.append(f"- first → last: {dd.history_first_score} → {dd.history_last_score}")
        lines.append("")
    lines.append("## 🛡️ GUARDS")
    lines.append("")
    for g in dd.guards:
        lines.append(f"- `{g}`")
    lines.append("")
    if dd.notes:
        lines.append("## 📝 Notes")
        lines.append("")
        for n in dd.notes:
            lines.append(f"- {n}")
        lines.append("")
    lines.append("## ❔ Known unknowns")
    lines.append("")
    for u in dd.known_unknowns:
        lines.append(f"- {u}")
    lines.append("")
    lines.append("---")
    lines.append(f"*V1395 dashboard v{V1395_VERSION} (schema {V1395_SCHEMA}). Real aggregate, not pretend.*")
    return "\n".join(lines)


def render_html(dd: DashboardData) -> str:
    """V1395 真生产: 渲染 HTML dashboard (主 00:36 真工程化)."""
    rows: List[str] = []
    for m in dd.modules:
        present_cell = "✅" if m.present else "❌"
        broken_cell = " ⚠️" if m.broken else ""
        rows.append(
            f"<tr><td><code>{html_escape(m.module_id)}</code></td>"
            f"<td>{html_escape(m.label)}</td>"
            f"<td>{present_cell}{broken_cell}</td>"
            f"<td>{html_escape(m.version or '—')}</td>"
            f"<td><code>{html_escape(m.schema or '—')}</code></td>"
            f"<td>{m.n_guards or '—'}</td>"
            f"<td>{m.n_tests or '—'}</td>"
            f"<td>{(m.file_size or 0):,} B</td></tr>"
        )
    v_color = {
        "GOOD": "#16a34a",
        "OK": "#22c55e",
        "POOR": "#eab308",
        "FAIL": "#f97316",
        "CRITICAL": "#dc2626",
        "N/A": "#94a3b8",
    }.get(dd.judge_verdict, "#94a3b8")
    t_color = {
        "improving": "#16a34a",
        "stable": "#64748b",
        "declining": "#dc2626",
        "n/a": "#94a3b8",
    }.get(dd.history_trend, "#94a3b8")
    body = f"""<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>{html_escape(dd.title)}</title>
<style>
body {{ font-family: ui-monospace, "SF Mono", Consolas, monospace; margin: 2rem; color: #0f172a; background: #f8fafc; }}
h1 {{ font-size: 1.5rem; }}
table {{ border-collapse: collapse; width: 100%; background: white; }}
th, td {{ border: 1px solid #cbd5e1; padding: 6px 10px; text-align: left; }}
th {{ background: #e2e8f0; }}
code {{ background: #f1f5f9; padding: 1px 4px; border-radius: 3px; }}
.panel {{ background: white; border: 1px solid #cbd5e1; padding: 1rem; margin: 1rem 0; border-radius: 6px; }}
.verdict {{ color: {v_color}; font-weight: bold; }}
.trend {{ color: {t_color}; font-weight: bold; }}
ul {{ margin: 0.25rem 0; }}
</style>
</head>
<body>
<h1>{html_escape(dd.title)}</h1>
<div class="panel">
<p>generated_at: <code>{html_escape(dd.generated_at)}</code></p>
<p>schema: <code>{html_escape(V1395_SCHEMA)}</code> v{V1395_VERSION}</p>
<p>modules: <b>{dd.n_present}/{dd.n_modules}</b> present, <b>{dd.n_broken}</b> broken</p>
<p>tests_total: <b>{dd.n_tests_total}</b></p>
{"<p>judge: <span class=\"verdict\">" + html_escape(dd.judge_verdict) + "</span> (target=<code>" + html_escape(dd.judge_target) + "</code>, score=" + str(dd.judge_score) + "/100, grade=" + html_escape(dd.judge_grade) + ", findings=" + str(dd.judge_n_findings) + ")</p>" if dd.judge_target else ""}
{"<p>trend: <span class=\"trend\">" + html_escape(dd.history_trend) + "</span> (entries=" + str(dd.history_n_entries) + ", delta_score=" + str(dd.history_delta_score) + ", last=" + str(dd.history_last_score) + ")</p>" if dd.history_n_entries > 0 else ""}
</div>
<h2>Deploy-stack modules (V1384-V1394)</h2>
<table>
<thead><tr><th>Module</th><th>Label</th><th>Present</th><th>Version</th><th>Schema</th><th>GUARDS</th><th>Tests</th><th>File size</th></tr></thead>
<tbody>
{chr(10).join(rows)}
</tbody>
</table>
<h2>GUARDS</h2>
<ul>
{chr(10).join("<li><code>" + html_escape(g) + "</code></li>" for g in dd.guards)}
</ul>
<h2>Known unknowns</h2>
<ul>
{chr(10).join("<li>" + html_escape(u) + "</li>" for u in dd.known_unknowns)}
</ul>
<hr>
<p><em>V1395 dashboard v{V1395_VERSION}. Real aggregate, not pretend.</em></p>
</body>
</html>"""
    return body


def render_json(dd: DashboardData) -> str:
    """V1395 真生产: 渲染 JSON dashboard (主 00:36 真工程化)."""
    return json.dumps(dd.to_dict(), indent=2, ensure_ascii=False)


# ============================================================================
# V1395 Popper self-test (主 17:43 真跑真测)
# ============================================================================


def popper_self_test() -> Dict[str, Any]:
    """V1395 真生产: Popper self-test (主 17:43 真跑真测)."""
    failures: List[str] = []

    # Test 1: discover 11 modules
    statuses = discover_module_status(
        Path(__file__).resolve().parent,
        _default_tests_dirs(Path(__file__).resolve().parent),
    )
    if len(statuses) != 11:
        failures.append(f"discover expected 11 modules, got {len(statuses)}")
    if sum(1 for m in statuses if m.present) < 11:
        failures.append(f"all 11 modules should be present, got {sum(1 for m in statuses if m.present)}")

    # Test 2: VERSION extracted for present modules (SCHEMA is informational)
    for m in statuses:
        if m.present and not m.version:
            failures.append(f"{m.module_id} version not extracted")

    # Test 3: at least 1 module has GUARDS
    if not any(m.n_guards > 0 for m in statuses):
        failures.append("no module has GUARDS extracted")

    # Test 4: at least 1 module has tests
    if not any(m.has_tests for m in statuses):
        failures.append("no module has tests discovered")

    # Test 5: build_dashboard returns valid
    dd = build_dashboard(judge_target=None, history_path=None)
    if dd.n_modules != 11:
        failures.append(f"build_dashboard n_modules={dd.n_modules} (expected 11)")
    if dd.n_present != 11:
        failures.append(f"build_dashboard n_present={dd.n_present} (expected 11)")

    # Test 6: render_markdown non-empty + has key sections
    md = render_markdown(dd)
    for needle in ("Deploy-stack modules", "GUARDS", "Known unknowns"):
        if needle not in md:
            failures.append(f"markdown missing {needle}")

    # Test 7: render_html non-empty + escaped
    html = render_html(dd)
    if "<table>" not in html or "</html>" not in html:
        failures.append("html missing table or html close")
    if "<script>" in html.lower():
        failures.append("html should not contain <script>")

    # Test 8: render_json valid JSON
    js = render_json(dd)
    try:
        parsed = json.loads(js)
        if parsed.get("schema") != V1395_SCHEMA:
            failures.append(f"json schema mismatch: {parsed.get('schema')}")
    except Exception as e:
        failures.append(f"json invalid: {e}")

    # Test 9: GUARDS count
    if len(V1395_GUARDS) < 8:
        failures.append(f"GUARDS < 8: {len(V1395_GUARDS)}")

    # Test 10: deterministic — same state → same dashboard
    dd2 = build_dashboard(judge_target=None, history_path=None)
    if dd2.n_modules != dd.n_modules:
        failures.append("build_dashboard not deterministic on n_modules")
    if dd2.n_present != dd.n_present:
        failures.append("build_dashboard not deterministic on n_present")

    return {
        "passed": len(failures) == 0,
        "failures": failures,
        "n_tested": 10,
    }


# ============================================================================
# V1395 CLI (主 17:43 真可执行)
# ============================================================================


def run_cli(argv: Optional[List[str]] = None) -> int:
    """V1395 真生产 CLI 主入口 (主 17:43 真可执行)."""
    parser = argparse.ArgumentParser(
        prog="v1395-deploy-dashboard",
        description=f"V1395 real production deploy-stack dashboard (v{V1395_VERSION})",
    )
    sub = parser.add_subparsers(dest="cmd", required=False)

    sub.add_parser("version", help="V1395 version")

    p_dash = sub.add_parser("dashboard", help="render markdown dashboard")
    p_dash.add_argument("--judge-target", default=None, help="optional target path to judge")
    p_dash.add_argument("--history", default=None, help="optional V1394 history JSONL")
    p_dash.add_argument("--title", default="Apeireth deploy-stack dashboard", help="dashboard title")
    p_dash.add_argument("--out", default=None, help="output file (default stdout)")

    p_html = sub.add_parser("html", help="render HTML dashboard")
    p_html.add_argument("--judge-target", default=None, help="optional target path to judge")
    p_html.add_argument("--history", default=None, help="optional V1394 history JSONL")
    p_html.add_argument("--title", default="Apeireth deploy-stack dashboard", help="dashboard title")
    p_html.add_argument("--out", default=None, help="output file (default stdout)")

    p_json = sub.add_parser("json", help="render JSON dashboard")
    p_json.add_argument("--judge-target", default=None, help="optional target path to judge")
    p_json.add_argument("--history", default=None, help="optional V1394 history JSONL")
    p_json.add_argument("--title", default="Apeireth deploy-stack dashboard", help="dashboard title")
    p_json.add_argument("--out", default=None, help="output file (default stdout)")

    p_modules = sub.add_parser("modules", help="list discovered modules")
    p_modules.add_argument("--out", default=None, help="output file (default stdout)")

    sub.add_parser("popper", help="V1395 Popper self-test")
    sub.add_parser("demo", help="V1395 demo")

    args = parser.parse_args(argv)
    cmd = args.cmd or "version"

    def _emit(text: str, out_path: Optional[str]) -> int:
        if out_path:
            Path(out_path).write_text(text, encoding="utf-8")
            print(f"wrote: {out_path} ({len(text)} chars)")
        else:
            print(text)
        return 0

    if cmd == "version":
        print(f"V1395 deploy dashboard v{V1395_VERSION} (schema {V1395_SCHEMA})")
        return 0
    if cmd == "dashboard":
        dd = build_dashboard(
            judge_target=args.judge_target,
            history_path=args.history,
            title=args.title,
        )
        return _emit(render_markdown(dd), args.out)
    if cmd == "html":
        dd = build_dashboard(
            judge_target=args.judge_target,
            history_path=args.history,
            title=args.title,
        )
        return _emit(render_html(dd), args.out)
    if cmd == "json":
        dd = build_dashboard(
            judge_target=args.judge_target,
            history_path=args.history,
            title=args.title,
        )
        return _emit(render_json(dd), args.out)
    if cmd == "modules":
        statuses = discover_module_status(
            Path(__file__).resolve().parent,
            _default_tests_dirs(Path(__file__).resolve().parent),
        )
        lines = [f"discovered {len(statuses)} modules:"]
        for m in statuses:
            mark = "✓" if m.present else "✗"
            lines.append(
                f"  {mark} {m.module_id} {m.label} ({m.module_name}.py) "
                f"v{m.version} guards={m.n_guards} tests={m.n_tests}"
            )
        return _emit("\n".join(lines), args.out)
    if cmd == "popper":
        r = popper_self_test()
        print(json.dumps(r, indent=2, ensure_ascii=False))
        return 0 if r["passed"] else 1
    if cmd == "demo":
        dd = build_dashboard(title="V1395 demo dashboard")
        print(render_markdown(dd))
        return 0

    parser.print_help()
    return 1


if __name__ == "__main__":
    sys.exit(run_cli())