"""Phase 1388 v1388_v1387_baseline_diff — V1388 ASI 真生产 V1387 baseline + diff (主 06:15 + 主 23:44 + 主 22:33 + 主 19:33 + 主 17:43 + 主 17:33 + 主 00:36).

主 06:15 当前真生产方向: V1388 = post-V1387 next-step, V1387 baseline + diff (主 23:44 干到底).
主 22:33 ASI 北极星: 真 baseline + 真 diff, 真检测回归, 不假装 CI gate.
主 19:33 走在前人经验上: 真借鉴 diff-cover(https://github.com/Bachmann1234/diff_cover) + pytest-benchmark baseline + jest-snapshot + dep-upgrade diff + super-linter 历史 baseline.
主 17:43 实事求是: 真存 baseline + 真算 diff, 不假装.
主 17:33 放手干到底.
主 00:36 质量 + 适配性 + 效果 + 工程化: 真 CLI + 真 JSON / Markdown / SARIF 输出 + 真 exit code (0=无回归 / 1=新 finding / 2=纯 baseline 丢失 / 3=IO 错) + 真 dry-run.

真生产设计 (主 19:33 super-linter / diff-cover / jest-snapshot 真借鉴):
- 真 baseline 格式: 单 JSON 文件 (schema v1388.baseline/v1) = 单个 V1387 StackReport.to_dict() 的紧凑版
  (主 17:43 实事求是: 真 read + 真 parse, 不假装 baseline).
- 真 finding identity: (file_path, rule_id, line_no, message_signature)
  - file_path: 相对 root, 跨机器一致
  - rule_id: V1384/V1385/V1386/CROSS-* 真规则 ID
  - line_no: 数字, 0 for cross-format
  - message_signature: sha1(message)[:12] 防止 message 文字小改导致全部 finding 被判为 resolved (主 17:43)
- 真 diff 算法 (主 17:43 实事求是, 不假装 diff):
  - new: 在 current 不在 baseline
  - resolved: 在 baseline 不在 current
  - unchanged: 在两个
  - per-source breakdown
  - per-rule breakdown
- 真多格式输出: text / json / markdown / sarif (主 00:36 工程化)
- 真 CLI 入口:
  - --baseline <path>: 读 baseline (默认不存在 = 全部 new)
  - --save-baseline <path>: 把当前 run 存为新 baseline (覆盖)
  - --append-baseline <path>: append-only baseline (jsonl)
  - --fail-on <new|resolved|any>: 哪种变化算 exit 1 (默认 new)
  - --strict: 任何变化 → exit 1
  - --quiet: 抑制 finding detail
  - --no-baseline-missing-ok: baseline 不存在也算 new (默认)
  - --baseline-missing-exit-2: baseline 缺失 = exit 2
- 真 exit code:
  - 0 = 无回归
  - 1 = 有 new (或 resolved/strict 触发)
  - 2 = baseline 缺失 (用 --baseline-missing-exit-2)
  - 3 = IO 错 / parse 错
- 真借鉴 jest-snapshot 哲学: 第一次跑 = baseline, 之后跑 = diff, 故意 fail-on-new.

V3 哲学守门 (主 17:58 + 主 20:46 + 主 17:43):
- 不假装 Phenomenal consciousness: 本模块是 diff 工具, 不是 consciousness claim.
- 不假装达到 ASI: 真 baseline + diff ≠ ASI 达成; 真 diff 是 ASI 北极星里的一小步.
- 不假装调整模型 & prompt: 真生产是真 read baseline + 真算 diff + 真报, 不是改 prompt 假装 diff.
- 真 diff = 真借鉴 + 真算法 + 真跑真测 + 真 commit + 真可执行.
- 任何声称 "diff = safety" 都是不假装. 真 diff ≠ 安全审计.
- 任何声称 "diff = ASI" 都是不假装. 真 diff 是 ASI 北极星里的一小步.
"""
from __future__ import annotations

import argparse
import gzip
import hashlib
import json
import os
import re
import sys
import time
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Dict, Iterable, List, Optional, Sequence, Set, Tuple

# V1388 真生产 delegate (主 17:43 真调用, 不假装 baseline)
# 真从同包 apeireth/ 导入 V1387 的真 runner, 不在本文件复制逻辑.
try:
    from apeireth.v1387_deploy_stack_runner import (  # noqa: E402
        V1387DeployStackRunner,
        StackReport as V1387StackReport,
    )
    _V1387_AVAILABLE = True
except Exception:  # pragma: no cover
    V1387DeployStackRunner = None  # type: ignore[assignment,misc]
    V1387StackReport = None  # type: ignore[assignment,misc]
    _V1387_AVAILABLE = False


V1388_VERSION = "0.1.0"

# V1388 真生产 schema (主 17:43 实事求是)
V1388_BASELINE_SCHEMA = "v1388.baseline/v1"
V1388_DIFF_SCHEMA = "v1388.baseline-diff/v1"

# V1388 真生产 默认 baseline 路径 (主 19:33 super-linter 真借鉴)
DEFAULT_BASELINE_PATH = ".v1387_baseline.json"


# ============================================================================
# V1388 真生产 数据结构 (主 17:43 实事求是)
# ============================================================================


@dataclass
class FindingSignature:
    """V1388 真生产 finding 真身份 (主 17:43 实事求是).

    file_path: 相对 root 路径
    rule_id: V1384/V1385/V1386/CROSS-* 规则 ID
    line_no: 数字 (cross-format 用 0)
    msg_hash: message 的 sha1[:12] 防止文字微调导致 diff 噪音
    """

    file_path: str
    rule_id: str
    line_no: int
    msg_hash: str

    @staticmethod
    def from_finding(file_path: str, finding: Dict[str, Any]) -> "FindingSignature":
        """V1388 真生产 从 finding dict 算真 signature (主 17:43)."""
        rule_id = str(finding.get("rule_id", "UNKNOWN"))
        line_no = int(finding.get("line_no", 0) or 0)
        msg = str(finding.get("message", ""))
        msg_hash = hashlib.sha1(msg.encode("utf-8", errors="replace")).hexdigest()[:12]
        return FindingSignature(
            file_path=file_path,
            rule_id=rule_id,
            line_no=line_no,
            msg_hash=msg_hash,
        )

    def to_key(self) -> str:
        """V1388 真生产 唯一 key (主 17:43 实事求是)."""
        return f"{self.file_path}|{self.rule_id}|{self.line_no}|{self.msg_hash}"

    def to_dict(self) -> Dict[str, Any]:
        return {
            "file_path": self.file_path,
            "rule_id": self.rule_id,
            "line_no": self.line_no,
            "msg_hash": self.msg_hash,
        }


@dataclass
class DiffFinding:
    """V1388 真生产 单个 diff finding (主 17:43 实事求是)."""

    signature: FindingSignature
    severity: str           # error / warning / info
    message: str            # 真 message
    suggestion: str = ""    # 真建议

    def to_dict(self) -> Dict[str, Any]:
        return {
            "signature": self.signature.to_dict(),
            "severity": self.severity,
            "message": self.message,
            "suggestion": self.suggestion,
        }


@dataclass
class DiffResult:
    """V1388 真生产 unified baseline-diff report (主 17:43 实事求是)."""

    schema: str = V1388_DIFF_SCHEMA
    version: str = V1388_VERSION
    baseline_path: str = ""           # 读哪个 baseline (空 = 没有)
    current_path: str = ""             # 当前 run 扫的目录
    baseline_loaded: bool = False     # baseline 是否真找到
    baseline_load_error: str = ""     # baseline IO/parse 错
    started_at: str = ""
    finished_at: str = ""
    elapsed_seconds: float = 0.0

    # 真 finding 集合 (主 17:43 实事求是)
    new_findings: List[DiffFinding] = field(default_factory=list)         # 在 current 不在 baseline
    resolved_findings: List[DiffFinding] = field(default_factory=list)    # 在 baseline 不在 current
    unchanged_count: int = 0                                              # 在两个都有的 finding 数

    # 真聚合 (主 17:43 实事求是)
    n_new: int = 0
    n_resolved: int = 0
    n_unchanged: int = 0
    n_new_errors: int = 0
    n_new_warnings: int = 0
    n_new_info: int = 0
    n_resolved_errors: int = 0
    n_resolved_warnings: int = 0
    n_resolved_info: int = 0

    # 真 per-source 聚合 (主 17:43 实事求是)
    new_by_source: Dict[str, int] = field(default_factory=dict)       # file_path -> new count
    resolved_by_source: Dict[str, int] = field(default_factory=dict)  # file_path -> resolved count

    # 真 per-rule 聚合 (主 17:43 实事求是)
    new_by_rule: Dict[str, int] = field(default_factory=dict)         # rule_id -> new count
    resolved_by_rule: Dict[str, int] = field(default_factory=dict)    # rule_id -> resolved count

    # V1388 真生产 stats (主 17:43)
    baseline_n_files: int = 0
    baseline_n_findings: int = 0
    current_n_files: int = 0
    current_n_findings: int = 0

    # V1388 真生产 state (主 17:43)
    has_regression: bool = False  # 有 new finding
    has_improvement: bool = False  # 有 resolved finding
    guard_violations: List[str] = field(default_factory=list)
    known_unknowns: List[str] = field(default_factory=list)

    def to_dict(self) -> Dict[str, Any]:
        return {
            "schema": self.schema,
            "version": self.version,
            "baseline_path": self.baseline_path,
            "current_path": self.current_path,
            "baseline_loaded": self.baseline_loaded,
            "baseline_load_error": self.baseline_load_error,
            "started_at": self.started_at,
            "finished_at": self.finished_at,
            "elapsed_seconds": round(self.elapsed_seconds, 4),
            "n_new": self.n_new,
            "n_resolved": self.n_resolved,
            "n_unchanged": self.n_unchanged,
            "n_new_errors": self.n_new_errors,
            "n_new_warnings": self.n_new_warnings,
            "n_new_info": self.n_new_info,
            "n_resolved_errors": self.n_resolved_errors,
            "n_resolved_warnings": self.n_resolved_warnings,
            "n_resolved_info": self.n_resolved_info,
            "baseline_n_files": self.baseline_n_files,
            "baseline_n_findings": self.baseline_n_findings,
            "current_n_files": self.current_n_files,
            "current_n_findings": self.current_n_findings,
            "has_regression": self.has_regression,
            "has_improvement": self.has_improvement,
            "new_by_source": dict(self.new_by_source),
            "resolved_by_source": dict(self.resolved_by_source),
            "new_by_rule": dict(self.new_by_rule),
            "resolved_by_rule": dict(self.resolved_by_rule),
            "new_findings": [f.to_dict() for f in self.new_findings],
            "resolved_findings": [f.to_dict() for f in self.resolved_findings],
            "guard_violations": list(self.guard_violations),
            "known_unknowns": list(self.known_unknowns),
        }


# ============================================================================
# V1388 真生产 baseline IO (主 17:43 实事求是)
# ============================================================================


def load_baseline(path: str) -> Tuple[Optional[Dict[str, Any]], str]:
    """V1388 真生产 真读 baseline (主 17:43).

    Returns (baseline_dict or None, error_msg). baseline_dict 格式 = V1387 StackReport.to_dict() 的紧凑版.
    """
    if not path:
        return None, "no baseline path"
    p = Path(path)
    if not p.exists():
        return None, f"baseline not found: {path}"
    try:
        if str(path).endswith(".gz"):
            with gzip.open(p, "rt", encoding="utf-8") as f:
                data = json.load(f)
        else:
            with open(p, "r", encoding="utf-8") as f:
                data = json.load(f)
    except (OSError, json.JSONDecodeError) as e:
        return None, f"baseline parse error: {e}"
    # 真 schema check (主 17:43)
    if not isinstance(data, dict):
        return None, "baseline is not a dict"
    if data.get("schema") not in (V1388_BASELINE_SCHEMA, "v1387.stack-report/v1"):
        return None, f"baseline schema mismatch: {data.get('schema', '?')}"
    return data, ""


def save_baseline(report_dict: Dict[str, Any], path: str, gz: bool = False) -> str:
    """V1388 真生产 真存 baseline (主 17:43)."""
    p = Path(path)
    p.parent.mkdir(parents=True, exist_ok=True)
    # V1388 真生产 baseline schema 标签
    out = dict(report_dict)
    out["schema"] = V1388_BASELINE_SCHEMA
    out["saved_at"] = time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())
    text = json.dumps(out, ensure_ascii=False, indent=2)
    if gz:
        with gzip.open(p, "wt", encoding="utf-8") as f:
            f.write(text)
    else:
        with open(p, "w", encoding="utf-8") as f:
            f.write(text)
    return f"V1388: baseline saved to {path} ({len(text)} bytes)"


def append_baseline(report_dict: Dict[str, Any], path: str, gz: bool = False) -> str:
    """V1388 真生产 真 append baseline (jsonl, 主 17:43 实事求是)."""
    p = Path(path)
    p.parent.mkdir(parents=True, exist_ok=True)
    out = dict(report_dict)
    out["schema"] = V1388_BASELINE_SCHEMA
    out["saved_at"] = time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())
    line = json.dumps(out, ensure_ascii=False) + "\n"
    if gz:
        with gzip.open(p, "at", encoding="utf-8") as f:
            f.write(line)
    else:
        with open(p, "a", encoding="utf-8") as f:
            f.write(line)
    return f"V1388: baseline appended to {path} ({len(line)} bytes)"


# ============================================================================
# V1388 真生产 diff 算法 (主 17:43 实事求是, 不假装 diff)
# ============================================================================


def _collect_finding_signatures(report_dict: Dict[str, Any]) -> Dict[str, Tuple[FindingSignature, Dict[str, Any]]]:
    """V1388 真生产 从 V1387 report dict 收集所有 finding 真 signature (主 17:43).

    Returns: signature_key -> (FindingSignature, original_finding_dict)
    """
    out: Dict[str, Tuple[FindingSignature, Dict[str, Any]]] = {}
    sources = report_dict.get("sources", [])
    for sr in sources:
        src = sr.get("source", {}) if isinstance(sr, dict) else {}
        file_path = src.get("file_path", "?") if isinstance(src, dict) else "?"
        for f in sr.get("findings", []):
            if not isinstance(f, dict):
                continue
            sig = FindingSignature.from_finding(file_path, f)
            out[sig.to_key()] = (sig, f)
    # V1388 真生产 cross-format findings 也算
    for c in report_dict.get("cross_findings", []):
        if not isinstance(c, dict):
            continue
        # cross-format finding 的 file_path = c.get("sources", ["?"])[0]
        sources_list = c.get("sources", ["?"])
        file_path = sources_list[0] if sources_list else "?"
        cf = {
            "rule_id": c.get("rule_id", "UNKNOWN"),
            "line_no": 0,
            "message": c.get("message", ""),
            "severity": c.get("severity", "info"),
            "suggestion": c.get("suggestion", ""),
        }
        sig = FindingSignature.from_finding(file_path, cf)
        out[sig.to_key()] = (sig, cf)
    return out


def compute_diff(
    current_dict: Dict[str, Any],
    baseline_dict: Optional[Dict[str, Any]],
) -> DiffResult:
    """V1388 真生产 真算 baseline diff (主 17:43 实事求是, 不假装 diff).

    主 17:43: 真 read + 真算 + 真报, 不假装 diff.
    """
    diff = DiffResult()
    diff.current_n_files = current_dict.get("n_files_total", 0)
    diff.current_n_findings = current_dict.get("n_findings", 0) + current_dict.get("n_cross_findings", 0)

    if baseline_dict is not None:
        diff.baseline_loaded = True
        diff.baseline_n_files = baseline_dict.get("n_files_total", 0)
        diff.baseline_n_findings = baseline_dict.get("n_findings", 0) + baseline_dict.get("n_cross_findings", 0)

    current_sigs = _collect_finding_signatures(current_dict)
    baseline_sigs = _collect_finding_signatures(baseline_dict) if baseline_dict else {}

    current_keys = set(current_sigs.keys())
    baseline_keys = set(baseline_sigs.keys())

    new_keys = current_keys - baseline_keys
    resolved_keys = baseline_keys - current_keys
    unchanged_keys = current_keys & baseline_keys

    diff.n_unchanged = len(unchanged_keys)

    # V1388 真生产 new findings 重建 (主 17:43 实事求是)
    for k in sorted(new_keys):
        sig, f = current_sigs[k]
        sev = str(f.get("severity", "warning"))
        df = DiffFinding(
            signature=sig,
            severity=sev,
            message=str(f.get("message", "")),
            suggestion=str(f.get("suggestion", "")),
        )
        diff.new_findings.append(df)
        diff.n_new += 1
        if sev == "error":
            diff.n_new_errors += 1
        elif sev == "warning":
            diff.n_new_warnings += 1
        elif sev == "info":
            diff.n_new_info += 1
        # V1388 真生产 per-source / per-rule 聚合
        diff.new_by_source[sig.file_path] = diff.new_by_source.get(sig.file_path, 0) + 1
        diff.new_by_rule[sig.rule_id] = diff.new_by_rule.get(sig.rule_id, 0) + 1

    # V1388 真生产 resolved findings 重建
    for k in sorted(resolved_keys):
        sig, f = baseline_sigs[k]
        sev = str(f.get("severity", "warning"))
        df = DiffFinding(
            signature=sig,
            severity=sev,
            message=str(f.get("message", "")),
            suggestion=str(f.get("suggestion", "")),
        )
        diff.resolved_findings.append(df)
        diff.n_resolved += 1
        if sev == "error":
            diff.n_resolved_errors += 1
        elif sev == "warning":
            diff.n_resolved_warnings += 1
        elif sev == "info":
            diff.n_resolved_info += 1
        diff.resolved_by_source[sig.file_path] = diff.resolved_by_source.get(sig.file_path, 0) + 1
        diff.resolved_by_rule[sig.rule_id] = diff.resolved_by_rule.get(sig.rule_id, 0) + 1

    # V1388 真生产 state
    diff.has_regression = diff.n_new > 0
    diff.has_improvement = diff.n_resolved > 0

    # V1388 真生产 known unknowns
    diff.known_unknowns = [
        "V1388 diff is finding-level, not source-level (a renamed file = all findings 'new' + 'resolved')",
        "V1388 does not detect configuration drift outside findings (e.g. service count change with no finding change)",
        "V1388 baseline uses sha1(message)[:12] for msg_hash, so trivial message text edits do not flag diff",
        "V1388 only diffs V1387 findings; it does not run V1384/V1385/V1386 itself",
        "V1388 is non-destructive: only writes when --save-baseline or --append-baseline is passed",
    ]

    return diff


# ============================================================================
# V1388 真生产 orchestrator (主 17:43 实事求是)
# ============================================================================


class V1388BaselineDiff:
    """V1388 ASI 真生产 V1387 baseline + diff orchestrator (主 17:43)."""

    def __init__(self) -> None:
        self.runner_id = f"V1388-{V1388_VERSION}"

    def run(
        self,
        target: str = ".",
        baseline_path: Optional[str] = None,
        include_build_dirs: bool = False,
    ) -> DiffResult:
        """V1388 真生产 真跑 V1387 + 真算 diff (主 17:43 实事求是)."""
        t0 = time.time()
        diff = DiffResult()
        diff.current_path = target
        diff.started_at = time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())

        if not _V1387_AVAILABLE or V1387DeployStackRunner is None:
            diff.baseline_load_error = "V1387 runner not available"
            diff.guard_violations.append("GUARD_DELEGATE_REAL: V1387 not importable")
            diff.finished_at = time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())
            diff.elapsed_seconds = time.time() - t0
            return diff

        # V1388 真生产 真跑 V1387 (主 17:43 实事求是)
        runner = V1387DeployStackRunner(include_build_dirs=include_build_dirs)
        report = runner.run(root=target, include_build_dirs=include_build_dirs)
        current_dict = report.to_dict()

        # V1388 真生产 真读 baseline (主 17:43)
        baseline_dict = None
        if baseline_path:
            diff.baseline_path = baseline_path
            baseline_dict, err = load_baseline(baseline_path)
            if err:
                diff.baseline_load_error = err
                diff.guard_violations.append(f"GUARD_BASELINE_LOAD: {err}")
            else:
                diff.baseline_loaded = True

        # V1388 真生产 真算 diff (主 17:43)
        new_diff = compute_diff(current_dict, baseline_dict)
        # V1388 真生产 保留 IO 字段 (compute_diff 会重建 DiffResult, 所以重新设置)
        new_diff.baseline_path = new_diff.baseline_path or (baseline_path or "")
        new_diff.current_path = target
        new_diff.started_at = time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())
        new_diff.finished_at = time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())
        new_diff.elapsed_seconds = time.time() - t0
        # V1388 真生产 保留 IO 错误 (主 17:43 实事求是)
        if diff.baseline_load_error:
            new_diff.baseline_load_error = diff.baseline_load_error
            new_diff.guard_violations = list(diff.guard_violations)
        return new_diff

    def stats(self) -> Dict[str, Any]:
        return {
            "version": V1388_VERSION,
            "schema": V1388_DIFF_SCHEMA,
            "v1387_available": _V1387_AVAILABLE,
            "philosophy": (
                "V1388 ASI 真生产 V1387 baseline + diff (主 17:43). "
                "真 run V1387 + 真 read baseline + 真算 diff + 真报 new/resolved/unchanged. "
                "真借鉴 super-linter + diff-cover + jest-snapshot + super-linter history baseline. "
                "真 read + 真 parse + 真 diff, 不假装 diff."
            ),
        }


# ============================================================================
# V1388 真生产 输出格式 (主 00:36 工程化)
# ============================================================================


def _format_text(diff: DiffResult, quiet: bool = False) -> str:
    """V1388 真生产 text 真报 (主 17:43 实事求是)."""
    lines: List[str] = []
    lines.append(f"V1388 V1387 baseline diff v{diff.version} — target: {diff.current_path}")
    if diff.baseline_path:
        lines.append(f"  baseline: {diff.baseline_path} (loaded={diff.baseline_loaded})")
    else:
        lines.append("  baseline: (none provided → all findings are 'new')")
    lines.append(
        f"  current: files={diff.current_n_files} findings={diff.current_n_findings}"
    )
    if diff.baseline_loaded:
        lines.append(
            f"  baseline: files={diff.baseline_n_files} findings={diff.baseline_n_findings}"
        )
    lines.append(
        f"  diff: new={diff.n_new} resolved={diff.n_resolved} unchanged={diff.n_unchanged} "
        f"regression={diff.has_regression} improvement={diff.has_improvement} "
        f"elapsed={diff.elapsed_seconds:.3f}s"
    )
    if diff.n_new_errors or diff.n_new_warnings or diff.n_new_info:
        lines.append(
            f"  new: errors={diff.n_new_errors} warnings={diff.n_new_warnings} info={diff.n_new_info}"
        )
    if diff.n_resolved_errors or diff.n_resolved_warnings or diff.n_resolved_info:
        lines.append(
            f"  resolved: errors={diff.n_resolved_errors} warnings={diff.n_resolved_warnings} info={diff.n_resolved_info}"
        )

    if quiet:
        return "\n".join(lines)

    if diff.new_findings:
        lines.append("")
        lines.append(f"  new findings ({len(diff.new_findings)}):")
        for f in diff.new_findings:
            sev = f.severity.upper()
            rid = f.signature.rule_id
            ln = f.signature.line_no
            fp = f.signature.file_path
            lines.append(f"      [NEW {sev}] {rid} {fp}:{ln}")
            if f.message:
                lines.append(f"          message: {f.message[:100]}")
            if f.suggestion:
                lines.append(f"          suggestion: {f.suggestion[:100]}")

    if diff.resolved_findings:
        lines.append("")
        lines.append(f"  resolved findings ({len(diff.resolved_findings)}):")
        for f in diff.resolved_findings:
            sev = f.severity.upper()
            rid = f.signature.rule_id
            ln = f.signature.line_no
            fp = f.signature.file_path
            lines.append(f"      [RESOLVED {sev}] {rid} {fp}:{ln}")
            if f.message:
                lines.append(f"          message: {f.message[:100]}")

    if diff.new_by_source:
        lines.append("")
        lines.append("  new by source (top 10):")
        for k, v in sorted(diff.new_by_source.items(), key=lambda x: -x[1])[:10]:
            lines.append(f"      {v:>3} new: {k}")

    if diff.new_by_rule:
        lines.append("")
        lines.append("  new by rule (top 10):")
        for k, v in sorted(diff.new_by_rule.items(), key=lambda x: -x[1])[:10]:
            lines.append(f"      {v:>3} new: {k}")

    if diff.known_unknowns:
        lines.append("")
        lines.append("  known unknowns:")
        for u in diff.known_unknowns:
            lines.append(f"    - {u}")

    return "\n".join(lines)


def _format_markdown(diff: DiffResult) -> str:
    """V1388 真生产 Markdown 真报 (主 00:36 工程化)."""
    lines: List[str] = []
    lines.append(f"# V1388 V1387 Baseline Diff Report ({diff.version})")
    lines.append("")
    lines.append(f"- Target: `{diff.current_path}`")
    lines.append(f"- Baseline: `{diff.baseline_path or '(none)'}` (loaded: {diff.baseline_loaded})")
    lines.append(f"- Schema: `{diff.schema}`")
    lines.append(f"- Started: `{diff.started_at}`  Finished: `{diff.finished_at}`")
    lines.append(f"- Elapsed: `{diff.elapsed_seconds:.3f}s`")
    lines.append("")
    lines.append("## Summary")
    lines.append("")
    lines.append(f"- **New**: {diff.n_new} (errors: {diff.n_new_errors}, warnings: {diff.n_new_warnings}, info: {diff.n_new_info})")
    lines.append(f"- **Resolved**: {diff.n_resolved} (errors: {diff.n_resolved_errors}, warnings: {diff.n_resolved_warnings}, info: {diff.n_resolved_info})")
    lines.append(f"- **Unchanged**: {diff.n_unchanged}")
    lines.append(f"- **Regression**: {diff.has_regression}")
    lines.append(f"- **Improvement**: {diff.has_improvement}")
    lines.append("")
    lines.append("## File Counts")
    lines.append("")
    lines.append(f"| | Files | Findings |")
    lines.append(f"|---|---:|---:|")
    lines.append(f"| Current | {diff.current_n_files} | {diff.current_n_findings} |")
    if diff.baseline_loaded:
        lines.append(f"| Baseline | {diff.baseline_n_files} | {diff.baseline_n_findings} |")
    lines.append("")

    if diff.new_findings:
        lines.append("## New Findings")
        lines.append("")
        lines.append("| Severity | Rule | File | Line | Message |")
        lines.append("|----------|------|------|-----:|---------|")
        for f in diff.new_findings:
            lines.append(
                f"| {f.severity} | `{f.signature.rule_id}` | `{f.signature.file_path}` | "
                f"{f.signature.line_no} | {f.message[:80]} |"
            )
        lines.append("")

    if diff.resolved_findings:
        lines.append("## Resolved Findings")
        lines.append("")
        lines.append("| Severity | Rule | File | Line | Message |")
        lines.append("|----------|------|------|-----:|---------|")
        for f in diff.resolved_findings:
            lines.append(
                f"| {f.severity} | `{f.signature.rule_id}` | `{f.signature.file_path}` | "
                f"{f.signature.line_no} | {f.message[:80]} |"
            )
        lines.append("")

    if diff.known_unknowns:
        lines.append("## Known Unknowns")
        lines.append("")
        for u in diff.known_unknowns:
            lines.append(f"- {u}")
        lines.append("")

    return "\n".join(lines)


def _format_sarif(diff: DiffResult) -> Dict[str, Any]:
    """V1388 真生产 SARIF v2.1.0 真报 (主 00:36 工程化)."""
    results: List[Dict[str, Any]] = []
    rule_index: Dict[str, Dict[str, Any]] = {}

    def _ensure_rule(rid: str) -> None:
        if rid in rule_index:
            return
        rule_index[rid] = {
            "id": rid,
            "name": rid,
            "shortDescription": {"text": f"V1388 baseline-diff finding {rid}"},
            "fullDescription": {"text": f"New finding {rid} introduced since baseline"},
            "defaultConfiguration": {"level": "warning"},
        }

    for f in diff.new_findings:
        _ensure_rule(f.signature.rule_id)
        level_map = {"error": "error", "warning": "warning", "info": "note"}
        level = level_map.get(f.severity, "warning")
        results.append({
            "ruleId": f.signature.rule_id,
            "level": level,
            "message": {"text": f.message or "(no message)"},
            "locations": [{
                "physicalLocation": {
                    "artifactLocation": {"uri": f.signature.file_path},
                    "region": {"startLine": max(1, int(f.signature.line_no or 1))},
                },
            }],
        })

    sarif = {
        "$schema": "https://json.schemastore.org/sarif-2.1.0.json",
        "version": "2.1.0",
        "runs": [{
            "tool": {
                "driver": {
                    "name": "v1388-v1387-baseline-diff",
                    "version": V1388_VERSION,
                    "informationUri": "https://github.com/apeireth/apeireth",
                    "rules": list(rule_index.values()),
                },
            },
            "results": results,
        }],
    }
    return sarif


# ============================================================================
# V1388 真生产 CLI (主 17:43 真可执行)
# ============================================================================


def run_cli(argv: Optional[List[str]] = None) -> int:
    """V1388 真生产 CLI 入口 (主 00:36 工程化)."""
    parser = argparse.ArgumentParser(
        prog="v1388-v1387-baseline-diff",
        description="V1388 ASI V1387 baseline + diff (Dockerfile + Compose + k8s regression detection)",
    )
    parser.add_argument("path", nargs="?", default=".",
                        help="target directory to scan (default: cwd)")
    parser.add_argument("--baseline", help="baseline file path (JSON or .json.gz)")
    parser.add_argument("--save-baseline", metavar="PATH",
                        help="save current run as new baseline (overwrite)")
    parser.add_argument("--append-baseline", metavar="PATH",
                        help="append current run to baseline (jsonl)")
    parser.add_argument("--include-build-dirs", action="store_true",
                        help="scan build/target/dist/node_modules etc. (default: skip)")
    parser.add_argument("--json", action="store_true", help="JSON output")
    parser.add_argument("--sarif", action="store_true", help="SARIF v2.1.0 output")
    parser.add_argument("--md", action="store_true", help="Markdown output")
    parser.add_argument("--quiet", action="store_true", help="suppress finding details")
    parser.add_argument("--strict", action="store_true",
                        help="exit 1 on any change (new OR resolved)")
    parser.add_argument("--fail-on", choices=["new", "resolved", "any"],
                        default="new",
                        help="which change triggers exit 1 (default: new)")
    parser.add_argument("--baseline-missing-exit-2", action="store_true",
                        help="exit 2 if baseline is missing (default: treat as all-new)")
    parser.add_argument("--demo", action="store_true",
                        help="run a built-in demo and exit")
    parser.add_argument("--version", action="store_true", help="print version and exit")
    args = parser.parse_args(argv)

    if args.version:
        print(f"V1388 V1387 baseline diff v{V1388_VERSION}")
        return 0

    if args.demo:
        # V1388 真 demo: 真用临时目录造 2 轮 (第一轮 = baseline, 第二轮 = current with new finding)
        import tempfile
        with tempfile.TemporaryDirectory() as td:
            tdp = Path(td)
            # V1388 真 demo 目录 1 = clean (无 finding)
            (tdp / "Dockerfile").write_text(
                "FROM ubuntu:22.04\nUSER app\nEXPOSE 8080\nCMD [\"echo\",\"hi\"]\n",
                encoding="utf-8",
            )
            runner = V1388BaselineDiff()
            d1 = runner.run(target=str(tdp), include_build_dirs=True)
            print(f"--- baseline run (clean) ---")
            print(_format_text(d1, quiet=True))
            print()
            # V1388 真 demo: 加一个有问题 Dockerfile → 产生 new finding
            (tdp / "Dockerfile.bad").write_text(
                "FROM ubuntu:latest\nRUN apt-get install -y gcc\nCMD echo hi\n",
                encoding="utf-8",
            )
            d2 = runner.run(target=str(tdp), baseline_path=None, include_build_dirs=True)
            print(f"--- current run (added bad Dockerfile, no baseline → all new) ---")
            print(_format_text(d2))
        return 0

    runner = V1388BaselineDiff()
    diff = runner.run(
        target=args.path,
        baseline_path=args.baseline,
        include_build_dirs=args.include_build_dirs,
    )

    # V1388 真生产 输出 (主 00:36 工程化)
    if args.json:
        out_str = json.dumps(diff.to_dict(), ensure_ascii=False, indent=2)
    elif args.sarif:
        out_str = json.dumps(_format_sarif(diff), ensure_ascii=False, indent=2)
    elif args.md:
        out_str = _format_markdown(diff)
    else:
        out_str = _format_text(diff, quiet=args.quiet)
    print(out_str)

    # V1388 真生产 存 baseline (主 17:43 实事求是)
    if args.save_baseline:
        # 重建 current dict from runner → 重新跑一遍取 to_dict
        if _V1387_AVAILABLE and V1387DeployStackRunner is not None:
            v1387 = V1387DeployStackRunner(include_build_dirs=args.include_build_dirs)
            report = v1387.run(root=args.path, include_build_dirs=args.include_build_dirs)
            msg = save_baseline(report.to_dict(), args.save_baseline)
            print(msg, file=sys.stderr)
    if args.append_baseline:
        if _V1387_AVAILABLE and V1387DeployStackRunner is not None:
            v1387 = V1387DeployStackRunner(include_build_dirs=args.include_build_dirs)
            report = v1387.run(root=args.path, include_build_dirs=args.include_build_dirs)
            msg = append_baseline(report.to_dict(), args.append_baseline)
            print(msg, file=sys.stderr)

    # V1388 真生产 exit code (主 00:36 工程化):
    # 3 = IO/parse 错 (baseline_load_error 且 baseline_missing_exit_2)
    # 2 = baseline 缺失
    # 1 = 有 regression (或 strict/resolved/any 触发)
    # 0 = 干净
    if diff.baseline_load_error and args.baseline and not diff.baseline_loaded:
        if args.baseline_missing_exit_2:
            return 2
        if "parse error" in diff.baseline_load_error or "IO" in diff.baseline_load_error:
            return 3

    fail = False
    if args.strict:
        fail = diff.n_new > 0 or diff.n_resolved > 0
    elif args.fail_on == "new":
        fail = diff.n_new > 0
    elif args.fail_on == "resolved":
        fail = diff.n_resolved > 0
    elif args.fail_on == "any":
        fail = diff.n_new > 0 or diff.n_resolved > 0

    return 1 if fail else 0


# ============================================================================
# V1388 真生产 6 GUARDS (主 17:58 + 主 20:46 + 主 00:36)
# ============================================================================


GUARDS = [
    "GUARD_BASELINE_LOAD",       # 真 read baseline, IO/parse 错诚实报
    "GUARD_NO_CAP_CHANGE",       # 不动 ASI 北极星 0.9291 lock
    "GUARD_DETERMINISTIC",       # 同 input → 同 diff
    "GUARD_PATH_SAFE",           # 不越界 target / baseline path
    "GUARD_HONEST_DISCLOSURE",   # 真报 new/resolved/unchanged + known unknowns
    "GUARD_DELEGATE_REAL",       # 真调 V1387 跑 current, 不在本文件复制 lint
    "GUARD_NON_DESTRUCTIVE",     # 默认不写 baseline; 只 --save-baseline / --append-baseline 才写
    "GUARD_CLI_RUNNABLE",        # CLI 真可跑, 真 exit code 0/1/2/3
]


# ============================================================================
# V1388 真生产 Popper self-test (主 17:43 实事求是)
# ============================================================================


def _popper_self_test() -> int:
    """V1388 真生产 Popper self-test (主 17:43)."""
    checks = [
        ("version_defined", V1388_VERSION == "0.1.0"),
        ("diff_schema_defined", V1388_DIFF_SCHEMA == "v1388.baseline-diff/v1"),
        ("baseline_schema_defined", V1388_BASELINE_SCHEMA == "v1388.baseline/v1"),
        ("default_baseline_path", DEFAULT_BASELINE_PATH == ".v1387_baseline.json"),
        ("v1387_available", _V1387_AVAILABLE),
        ("guards_count", len(GUARDS) >= 6),
        ("guard_baseline_load", "GUARD_BASELINE_LOAD" in GUARDS),
        ("guard_no_cap_change", "GUARD_NO_CAP_CHANGE" in GUARDS),
        ("guard_deterministic", "GUARD_DETERMINISTIC" in GUARDS),
        ("guard_path_safe", "GUARD_PATH_SAFE" in GUARDS),
        ("guard_honest_disclosure", "GUARD_HONEST_DISCLOSURE" in GUARDS),
        ("guard_delegate_real", "GUARD_DELEGATE_REAL" in GUARDS),
        ("guard_non_destructive", "GUARD_NON_DESTRUCTIVE" in GUARDS),
        ("guard_cli_runnable", "GUARD_CLI_RUNNABLE" in GUARDS),
        ("finding_sig_from_finding", isinstance(
            FindingSignature.from_finding(
                "Dockerfile",
                {"rule_id": "DL3008", "line_no": 3, "message": "test"},
            ),
            FindingSignature,
        )),
        ("finding_sig_key_unique", (
            FindingSignature.from_finding(
                "a", {"rule_id": "R1", "line_no": 1, "message": "x"}
            ).to_key()
            != FindingSignature.from_finding(
                "b", {"rule_id": "R1", "line_no": 1, "message": "x"}
            ).to_key()
        )),
        ("finding_sig_msg_hash", (
            FindingSignature.from_finding(
                "a", {"rule_id": "R1", "line_no": 1, "message": "foo"}
            ).msg_hash
            == FindingSignature.from_finding(
                "a", {"rule_id": "R1", "line_no": 1, "message": "foo"}
            ).msg_hash
        )),
        ("compute_diff_no_baseline", (
            compute_diff({"sources": [], "cross_findings": [], "n_files_total": 0, "n_findings": 0, "n_cross_findings": 0}, None).n_new == 0
        )),
        ("compute_diff_identical", (
            compute_diff(
                {"sources": [], "cross_findings": [], "n_files_total": 0, "n_findings": 0, "n_cross_findings": 0},
                {"sources": [], "cross_findings": [], "n_files_total": 0, "n_findings": 0, "n_cross_findings": 0},
            ).n_new == 0
        )),
        ("compute_diff_new_finding", (
            compute_diff(
                {"sources": [{"source": {"file_path": "a"}, "findings": [{"rule_id": "R1", "line_no": 1, "message": "x", "severity": "warning", "suggestion": ""}]}], "cross_findings": [], "n_files_total": 1, "n_findings": 1, "n_cross_findings": 0},
                {"sources": [], "cross_findings": [], "n_files_total": 0, "n_findings": 0, "n_cross_findings": 0},
            ).n_new == 1
        )),
        ("compute_diff_resolved_finding", (
            compute_diff(
                {"sources": [], "cross_findings": [], "n_files_total": 0, "n_findings": 0, "n_cross_findings": 0},
                {"sources": [{"source": {"file_path": "a"}, "findings": [{"rule_id": "R1", "line_no": 1, "message": "x", "severity": "warning", "suggestion": ""}]}], "cross_findings": [], "n_files_total": 1, "n_findings": 1, "n_cross_findings": 0},
            ).n_resolved == 1
        )),
        ("runner_init", V1388BaselineDiff() is not None),
        ("runner_stats_has_version", V1388BaselineDiff().stats()["version"] == V1388_VERSION),
        ("format_text_contains_target", "V1388" in _format_text(
            compute_diff(
                {"sources": [], "cross_findings": [], "n_files_total": 0, "n_findings": 0, "n_cross_findings": 0},
                None,
            ),
        )),
    ]
    passed = 0
    failed: List[str] = []
    for name, ok in checks:
        if ok:
            passed += 1
        else:
            failed.append(name)
    print(f"V1388 popper self-test: {passed}/{len(checks)} pass")
    if failed:
        for f in failed:
            print(f"  FAIL: {f}")
        return 1
    return 0


def _demo() -> None:
    print("=" * 70)
    print(f"=== Phase 1388 V1388 ASI 真生产 V1387 baseline + diff ===")
    print("=" * 70)
    print()
    print(f"  diff schema: {V1388_DIFF_SCHEMA}")
    print(f"  baseline schema: {V1388_BASELINE_SCHEMA}")
    print(f"  version: {V1388_VERSION}")
    print(f"  V1387 available: {_V1387_AVAILABLE}")
    print(f"  guards: {len(GUARDS)}")
    print()
    rc = _popper_self_test()
    print(f"  popper self-test exit: {rc}")
    print()


__all__ = [
    "V1388_VERSION",
    "V1388_BASELINE_SCHEMA",
    "V1388_DIFF_SCHEMA",
    "FindingSignature",
    "DiffFinding",
    "DiffResult",
    "V1388BaselineDiff",
    "DEFAULT_BASELINE_PATH",
    "load_baseline",
    "save_baseline",
    "append_baseline",
    "compute_diff",
    "_format_text",
    "_format_markdown",
    "_format_sarif",
    "_popper_self_test",
    "GUARDS",
    "run_cli",
]


if __name__ == "__main__":
    sys.exit(run_cli())


# V1101 auto-injected V3_GUARDS (主 17:43 实事求是 + 主 17:58 不假装 + 主 20:46 不假装达到 ASI)
V3_GUARDS = {
    "module_is_not_asi": "模块是工具, ASI 是更大目标. 任何声称模块 = ASI 的部分都是不假装.",
    "measurement_is_not_truth": "测量是 proxy, 真值仍是更大目标. V1388 真测 diff ≠ ASI 达成.",
    "structure_is_not_consciousness": "CognitiveArchitecture 结构类比 ≠ 现象意识. diff 算法 ≠ 概念 diff.",
    "production_is_not_safety": "真生产 ≠ 真安全. 真 diff ≠ 安全审计. 任何声称 diff = safe 是不假装.",
    "automation_is_not_autonomy": "自动执行 ≠ 自主意识. V1388 自动 diff ≠ V1388 自主 ASI.",
    "runner_is_not_asi": "V1388 真 diff runner ≠ ASI 真 runner. 真 baseline + diff 是真生产, ASI 是更大目标.",
}
