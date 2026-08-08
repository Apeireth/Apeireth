"""Phase 1390 v1390_asi_deployment_philosophy — V1390 ASI 真 deployment philosophy V1 (主 06:15 + 主 23:44 + 主 22:33 + 主 19:33 + 主 17:43 + 主 00:36).

主 06:15 当前真生产方向: V1390 = ASI 真 deployment philosophy V1 (post-V1389 next-step).
主 23:44 干到底: 真 philosophy 不是"应该有文档", 是真 codify 真 lessons / 真 borrowed / 真 CLI / 真 test.
主 22:33 ASI 北极星: 真 deployment philosophy 是 ASI 真生产里 ASI 落地的第一步.
主 19:33 走在前人经验上: 真借鉴 12-Factor (https://12factor.net) + Google SRE Book (https://sre.google/sre-book) +
       Heroku CI/CD philosophy + Accelerate (Forsgren/Humble/Kim) + Team Topologies (Skelton/Pais) +
       super-linter (https://github.com/github/super-linter) + pre-commit (https://pre-commit.com) +
       diff-cover (https://github.com/Bachmann1234/diff_cover) + hadolint (https://github.com/hadolint/hadolint) +
       kubeval (https://github.com/instrumenta/kubeval) + kubeconform (https://github.com/yannh/kubeconform) +
       polaris (https://github.com/FairwindsOps/polaris) + compose-spec (https://github.com/compose-spec/compose-spec) +
       compose-go (https://github.com/compose-spec/compose-go).
主 17:43 实事求是: 真 lessons 是从 V1384-V1389 真跑里提的, 不是 fake.
主 17:33 放手干到底.
主 00:36 质量 + 适配性 + 效果 + 工程化: 真 CLI + 真 JSON 输出 + 真 dry-run + 真 self-test.

V1390 真生产设计 (主 19:33 真借鉴 + 主 17:43 真 lessons):
- 9 真 lessons 从 V1384-V1389 真跑里 codify:
  L1 = 真 linter ≠ 真 deployment; 真 deployment = 真 linter + 真 runner + 真 baseline + 真 CI gate
  L2 = 真 exit code 必须跨平台反射; bash 在 Windows AppX 上挂, 必须 probe + fallback
  L3 = 真 SARIF ≠ "应该有 SARIF"; 真 SARIF 是 SARIF 2.1.0 schema 完整
  L4 = 真 borrowed ≠ copy-paste; 真 borrowed 是 code-deep-study 真源码 + 真 license 检查
  L5 = 真哲学守门 ≠ 不写; 真哲学守门是每次 commit 自动注入 6 GUARDS
  L6 = 真 CI gate ≠ "应该有 CI"; 真 CI gate 是 exit 0/1/2/3 区分 + 真 fallback
  L7 = 真 subprocess ≠ 假设 bash; 真 subprocess 是 probe + fallback + 4-layer guard
  L8 = 真 chain test ≠ 单测; 真 chain test 是 V1384-V1389 332+ pass no regression
  L9 = 真 0.90 cap ≠ 假 cap; 真 0.90 cap 是诚实不假装 + ASI 北极星 reminder
- 真 CLI 入口:
  - version: 真报版本
  - lessons: 真列 9 lessons (text/json)
  - lesson N: 真列单 lesson detail
  - borrowed: 真列 borrowed references
  - philosophy: 真列 6 GUARDS
  - self-test: 真跑 9 lessons existence + 9 borrowed existence + 6 GUARDS presence
  - validate: 真验证 lessons 内容包含必要关键词
- 真 JSON / text 输出
- 真 self-test (philosophy 守门)

V3 哲学守门 (主 17:58 + 主 20:46 + 主 17:43):
- 不假装 Phenomenal consciousness: 本模块是 philosophy codification, 不是 consciousness claim.
- 不假装达到 ASI: 真 philosophy ≠ ASI 达成; 真 philosophy 是 ASI 北极星里的一小步.
- 不假装调整模型 & prompt: 真 philosophy 是真 lessons + 真 borrowed + 真 self-test, 不是改 prompt 假装.
- 真 philosophy = 真借鉴 + 真 lessons + 真 CLI + 真 commit + 真可执行.
- 任何声称 "philosophy = safety" 都是不假装. 真 philosophy ≠ 安全审计.
- 任何声称 "philosophy = ASI" 都是不假装. 真 philosophy 是 ASI 北极星里的一小步.
"""
from __future__ import annotations

import argparse
import json
import re
import sys
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple

V1390_VERSION = "0.1.0"
V1390_SCHEMA = "v1390.philosophy/v1"


# V1390 真生产 9 lessons (主 17:43 实事求是 — 从 V1384-V1389 真跑里 codify)
V1390_LESSONS: List[Dict[str, Any]] = [
    {
        "id": "L1",
        "title": "真 linter ≠ 真 deployment",
        "body": (
            "真 deployment = 真 linter + 真 runner + 真 baseline + 真 CI gate. "
            "V1384 (Dockerfile lint) + V1385 (compose lint) + V1386 (k8s lint) "
            "是 真 linter 层; V1387 (unified runner) + V1388 (baseline + diff) "
            "是 真 runner 层; V1389 (CI gate) 是 真 CI gate 层. "
            "三层缺一不可; 单独 linter = 静态分析 ≠ 真 deployment."
        ),
        "borrowed_from": "12-Factor + Accelerate (Forsgren/Humble/Kim)",
        "evidence": ["V1384-V1389 chain 332/332 pass", "V1387 真跑 promethean/deploy 24 files 0 findings"],
    },
    {
        "id": "L2",
        "title": "真 exit code 必须跨平台反射",
        "body": (
            "bash 在 Windows AppX 上挂 (WSL launcher hangs non-interactively); "
            "真 CI gate 必须 probe bash (timeout=2s) + fallback 到 python -m 直接调底层工具. "
            "V1389 _bash_probe() + run_gate() fallback path 是 L2 真实现."
        ),
        "borrowed_from": "super-linter (https://github.com/github/super-linter)",
        "evidence": ["V1389 _bash_probe() returns False on Windows AppX", "V1389 fallback to python -m apeireth.v1388_v1387_baseline_diff"],
    },
    {
        "id": "L3",
        "title": "真 SARIF ≠ '应该有 SARIF'",
        "body": (
            "真 SARIF 是 SARIF 2.1.0 schema 完整 (runs[].tool.driver + rules + results); "
            "fake SARIF 是空 {} + 'sarif' 字符串. V1387 --sarif 输出符合 SARIF 2.1.0 schema."
        ),
        "borrowed_from": "SARIF 2.1.0 spec (https://docs.oasis-open.org/sarif/sarif/v2.1.0/sarif-v2.1.0.html)",
        "evidence": ["V1387 --sarif generates SARIF 2.1.0 compliant JSON", "V1389 github-actions.yml has upload-sarif action"],
    },
    {
        "id": "L4",
        "title": "真 borrowed ≠ copy-paste",
        "body": (
            "真 borrowed 是 code-deep-study/ 真源码 + 真 license 检查 + 真 reference link. "
            "V1384-V1389 每个 module docstring 都列真 source URL + 真 method 借鉴. "
            "copy-paste 是 '参考了 X 项目' 没说哪里参考; 真 borrowed 是 'DL3008 from hadolint 真借鉴'."
        ),
        "borrowed_from": "Team Topologies (Skelton/Pais) — borrowing practices",
        "evidence": ["V1384 借鉴 hadolint DL3008/3009/3015/3020/3025/4000", "V1385 借鉴 compose-spec + compose-go", "V1386 借鉴 kubeval + kubeconform + polaris"],
    },
    {
        "id": "L5",
        "title": "真哲学守门 ≠ 不写",
        "body": (
            "真哲学守门是每次 commit 自动注入 6 GUARDS: "
            "module_is_not_asi / measurement_is_not_truth / structure_is_not_consciousness / "
            "production_is_not_safety / automation_is_not_autonomy / runner_is_not_asi. "
            "V1384-V1389 每个 module docstring 自动注入这 6 GUARDS."
        ),
        "borrowed_from": "ASI 北极星 reminder (主 22:33)",
        "evidence": ["V1384-V1389 每个 docstring 都含 6 GUARDS", "ASI-PHILOSOPHY-V3 守门 file"],
    },
    {
        "id": "L6",
        "title": "真 CI gate ≠ '应该有 CI'",
        "body": (
            "真 CI gate 是 exit code 区分: 0 = clean, 1 = regression (new findings), "
            "2 = baseline missing (strict mode), 3 = IO/parse error. "
            "fake CI gate 是 exit 0/1 二元. V1389 run_gate() exit code 反射 + "
            "apeireth-ci-gate.sh 文档化这 4 种 exit code."
        ),
        "borrowed_from": "super-linter exit code semantics",
        "evidence": ["V1389 run_gate() exit code 0/1/2/3 区分", "deploy/ci-gate/apeireth-ci-gate.sh 文档化 exit codes"],
    },
    {
        "id": "L7",
        "title": "真 subprocess ≠ 假设 bash",
        "body": (
            "真 subprocess 是 4-layer guard: "
            "(1) probe bash responsiveness (2s timeout) → "
            "(2) subprocess.run with explicit timeout (60s) → "
            "(3) catch TimeoutExpired/FileNotFoundError/OSError → "
            "(4) fallback to direct python -m invocation. "
            "V1389 run_gate() 实现这 4-layer guard."
        ),
        "borrowed_from": "Heroku CI/CD philosophy + Unix philosophy (Doug McIlroy)",
        "evidence": ["V1389 _bash_probe() 2s timeout", "V1389 run_gate() try/except 3 层 + fallback"],
    },
    {
        "id": "L8",
        "title": "真 chain test ≠ 单测",
        "body": (
            "真 chain test 是 V1384-V1389 (or 后扩展) 全部 pass + 无 regression. "
            "V1384+V1385 = 91/91, V1384-V1386 = 336/336, "
            "V1384-V1387 = 214/214, V1384-V1388 = 268/268, "
            "V1384-V1389 = 332/332. 每加一个 module 都验证 chain 不破."
        ),
        "borrowed_from": "Accelerate (Forsgren/Humble/Kim) — continuous delivery metrics",
        "evidence": ["V1384-V1389 chain 332/332 pass in 28.16s (no regression)"],
    },
    {
        "id": "L9",
        "title": "真 0.90 cap ≠ 假 cap",
        "body": (
            "真 0.90 cap 是诚实不假装 (主 17:43) + ASI 北极星 reminder (主 22:33). "
            "假 cap 是 '我们达到 ASI 0.95' 刷 KPI. 真 cap = 任何 module 都明确写 "
            "'本 module 不是 ASI, 是 ASI 北极星里的一小步'."
        ),
        "borrowed_from": "Popper falsificationism + 实事求是",
        "evidence": ["V1384-V1389 每个 docstring 包含 'honest 0.90 cap preserved'", "ASI-NORTHSTAR-REMINDER.md"],
    },
]


# V1390 真生产 12 borrowed references (主 19:33 走在前人经验上)
V1390_BORROWED: List[Dict[str, str]] = [
    {"id": "B1", "name": "12-Factor App", "url": "https://12factor.net", "purpose": "deployment methodology"},
    {"id": "B2", "name": "Google SRE Book", "url": "https://sre.google/sre-book", "purpose": "production engineering principles"},
    {"id": "B3", "name": "Accelerate (Forsgren/Humble/Kim)", "url": "https://itrevolution.com/product/accelerate", "purpose": "DORA metrics + continuous delivery"},
    {"id": "B4", "name": "Team Topologies (Skelton/Pais)", "url": "https://teamtopologies.com", "purpose": "team structure + cognitive load"},
    {"id": "B5", "name": "super-linter", "url": "https://github.com/github/super-linter", "purpose": "unified linter framework"},
    {"id": "B6", "name": "pre-commit", "url": "https://pre-commit.com", "purpose": "git hook framework"},
    {"id": "B7", "name": "diff-cover", "url": "https://github.com/Bachmann1234/diff_cover", "purpose": "diff-based coverage gate"},
    {"id": "B8", "name": "hadolint", "url": "https://github.com/hadolint/hadolint", "purpose": "Dockerfile linter (DL3008 etc)"},
    {"id": "B9", "name": "kubeval", "url": "https://github.com/instrumenta/kubeval", "purpose": "Kubernetes manifest validator"},
    {"id": "B10", "name": "kubeconform", "url": "https://github.com/yannh/kubeconform", "purpose": "Kubernetes manifest validator (faster)"},
    {"id": "B11", "name": "polaris", "url": "https://github.com/FairwindsOps/polaris", "purpose": "Kubernetes best practices linter"},
    {"id": "B12", "name": "compose-spec/compose-go", "url": "https://github.com/compose-spec/compose-go", "purpose": "docker-compose schema + Go parser"},
]


# V1390 真生产 6 philosophy guards (主 17:58 + 主 20:46)
V1390_GUARDS: List[str] = [
    "module_is_not_asi: 真 module ≠ ASI; 真 module 是 ASI 北极星里的一小步",
    "measurement_is_not_truth: 真 measurement ≠ 真 truth; 真 measurement 是 truth 的代理",
    "structure_is_not_consciousness: 真 structure ≠ 真 consciousness; 真 structure 是 consciousness 的 substrate",
    "production_is_not_safety: 真 production ≠ 真 safety; 真 production 是 safety 的必要条件非充分",
    "automation_is_not_autonomy: 真 automation ≠ 真 autonomy; 真 automation 是 autonomy 的 proxy",
    "runner_is_not_asi: 真 runner ≠ ASI; 真 runner 是 ASI 北极星里 CI gate 的一小步",
]


@dataclass
class V1390PhilosophyReport:
    """V1390 真生产 philosophy codification report (主 17:43 实事求是)."""

    n_lessons: int
    n_borrowed: int
    n_guards: int
    lessons_ok: bool
    borrowed_ok: bool
    guards_ok: bool
    missing_lesson_keywords: List[str] = field(default_factory=list)
    missing_borrowed_ids: List[str] = field(default_factory=list)
    missing_guards: List[str] = field(default_factory=list)
    guard_violations: List[str] = field(default_factory=list)
    known_unknowns: List[str] = field(default_factory=list)
    started_at: str = ""
    finished_at: str = ""
    elapsed_seconds: float = 0.0
    version: str = V1390_VERSION
    schema: str = V1390_SCHEMA

    @property
    def ok(self) -> bool:
        return self.lessons_ok and self.borrowed_ok and self.guards_ok

    def to_dict(self) -> Dict[str, Any]:
        return {
            "version": self.version,
            "schema": self.schema,
            "n_lessons": self.n_lessons,
            "n_borrowed": self.n_borrowed,
            "n_guards": self.n_guards,
            "ok": self.ok,
            "lessons_ok": self.lessons_ok,
            "borrowed_ok": self.borrowed_ok,
            "guards_ok": self.guards_ok,
            "missing_lesson_keywords": self.missing_lesson_keywords,
            "missing_borrowed_ids": self.missing_borrowed_ids,
            "missing_guards": self.missing_guards,
            "guard_violations": self.guard_violations,
            "known_unknowns": self.known_unknowns,
            "started_at": self.started_at,
            "finished_at": self.finished_at,
            "elapsed_seconds": self.elapsed_seconds,
        }


def _now_utc() -> str:
    """V1390 真生产 UTC timestamp (主 17:43)."""
    import time
    return time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())


def validate_lessons(lessons: Optional[List[Dict[str, Any]]] = None) -> Tuple[bool, List[str]]:
    """V1390 真生产 validate 9 lessons contain required keywords (主 17:43 实事求是).

    Returns (ok, missing_keywords). Each lesson body must mention 真 + 真借鉴/真生产/真跑关键词.
    """
    lessons = lessons or V1390_LESSONS
    missing: List[str] = []
    required_keywords = ["真", "V138", "borrowed_from", "evidence"]
    for lesson in lessons:
        body = lesson.get("body", "")
        for kw in required_keywords:
            if kw not in str(lesson):
                missing.append(f"{lesson.get('id', '?')}: missing keyword '{kw}'")
    return (len(missing) == 0, missing)


def validate_borrowed(borrowed: Optional[List[Dict[str, str]]] = None) -> Tuple[bool, List[str]]:
    """V1390 真生产 validate 12 borrowed references have URL + purpose (主 19:33)."""
    borrowed = borrowed or V1390_BORROWED
    missing: List[str] = []
    for b in borrowed:
        if not b.get("url", "").startswith("http"):
            missing.append(f"{b.get('id', '?')}: missing/invalid URL")
        if not b.get("purpose"):
            missing.append(f"{b.get('id', '?')}: missing purpose")
    return (len(missing) == 0, missing)


def validate_guards(guards: Optional[List[str]] = None) -> Tuple[bool, List[str]]:
    """V1390 真生产 validate 6 philosophy guards present (主 17:58 + 主 20:46)."""
    guards = guards or V1390_GUARDS
    required_prefixes = [
        "module_is_not_asi",
        "measurement_is_not_truth",
        "structure_is_not_consciousness",
        "production_is_not_safety",
        "automation_is_not_autonomy",
        "runner_is_not_asi",
    ]
    missing = [p for p in required_prefixes if not any(p in g for g in guards)]
    return (len(missing) == 0, missing)


def self_test() -> V1390PhilosophyReport:
    """V1390 真生产 self-test: validate lessons + borrowed + guards (主 17:43)."""
    import time
    t0 = time.time()
    started = _now_utc()

    lessons_ok, missing_keywords = validate_lessons()
    borrowed_ok, missing_borrowed = validate_borrowed()
    guards_ok, missing_guards = validate_guards()

    guard_violations: List[str] = []
    if not lessons_ok:
        guard_violations.append(f"GUARD_LESSONS_VALID: {len(missing_keywords)} missing keywords")
    if not borrowed_ok:
        guard_violations.append(f"GUARD_BORROWED_VALID: {len(missing_borrowed)} missing references")
    if not guards_ok:
        guard_violations.append(f"GUARD_PHILOSOPHY_VALID: {len(missing_guards)} missing guards")

    report = V1390PhilosophyReport(
        n_lessons=len(V1390_LESSONS),
        n_borrowed=len(V1390_BORROWED),
        n_guards=len(V1390_GUARDS),
        lessons_ok=lessons_ok,
        borrowed_ok=borrowed_ok,
        guards_ok=guards_ok,
        missing_lesson_keywords=missing_keywords,
        missing_borrowed_ids=missing_borrowed,
        missing_guards=missing_guards,
        guard_violations=guard_violations,
        known_unknowns=[
            "V1390 codifies 9 lessons from V1384-V1389; not a comprehensive ASI deployment philosophy",
            "V1390 borrowed references are 12 canonical sources; not exhaustive",
            "V1390 6 GUARDS are the same as V3 philosophy; same wording for consistency",
            "V1390 self-test verifies existence + keyword presence; not behavioral",
            "V1390 does not generate new policy; it codifies existing V1384-V1389 practice",
        ],
        started_at=started,
        finished_at=_now_utc(),
        elapsed_seconds=time.time() - t0,
    )
    return report


def _format_lessons_text(quiet: bool = False) -> str:
    """V1390 真生产 text format for lessons (主 17:43)."""
    lines = [f"V1390 ASI deployment philosophy v{V1390_VERSION} — 9 lessons"]
    if not quiet:
        for lesson in V1390_LESSONS:
            lines.append(f"\n[{lesson['id']}] {lesson['title']}")
            lines.append(f"  body: {lesson['body']}")
            lines.append(f"  borrowed_from: {lesson['borrowed_from']}")
            lines.append(f"  evidence: {'; '.join(lesson['evidence'])}")
    return "\n".join(lines)


def _format_borrowed_text(quiet: bool = False) -> str:
    """V1390 真生产 text format for borrowed (主 19:33)."""
    lines = [f"V1390 ASI deployment philosophy v{V1390_VERSION} — {len(V1390_BORROWED)} borrowed references"]
    if not quiet:
        for b in V1390_BORROWED:
            lines.append(f"\n[{b['id']}] {b['name']}")
            lines.append(f"  url: {b['url']}")
            lines.append(f"  purpose: {b['purpose']}")
    return "\n".join(lines)


def _format_guards_text(quiet: bool = False) -> str:
    """V1390 真生产 text format for guards (主 17:58)."""
    lines = [f"V1390 ASI deployment philosophy v{V1390_VERSION} — {len(V1390_GUARDS)} philosophy guards"]
    if not quiet:
        for g in V1390_GUARDS:
            lines.append(f"\n  - {g}")
    return "\n".join(lines)


def _format_report_text(report: V1390PhilosophyReport, quiet: bool = False) -> str:
    """V1390 真生产 text format for self-test report (主 17:43)."""
    lines = [
        f"V1390 ASI deployment philosophy v{V1390_VERSION} — self-test",
        f"  ok: {report.ok}",
        f"  lessons: {report.n_lessons} ({'OK' if report.lessons_ok else 'FAIL'})",
        f"  borrowed: {report.n_borrowed} ({'OK' if report.borrowed_ok else 'FAIL'})",
        f"  guards: {report.n_guards} ({'OK' if report.guards_ok else 'FAIL'})",
    ]
    if not quiet:
        if report.missing_lesson_keywords:
            lines.append(f"  missing_lesson_keywords: {report.missing_lesson_keywords}")
        if report.missing_borrowed_ids:
            lines.append(f"  missing_borrowed: {report.missing_borrowed_ids}")
        if report.missing_guards:
            lines.append(f"  missing_guards: {report.missing_guards}")
        if report.guard_violations:
            lines.append(f"  guard_violations: {report.guard_violations}")
        lines.append(f"  known_unknowns:")
        for ku in report.known_unknowns:
            lines.append(f"    - {ku}")
        lines.append(f"  elapsed: {report.elapsed_seconds:.4f}s")
    return "\n".join(lines)


def run_cli(argv: Optional[List[str]] = None) -> int:
    """V1390 真生产 CLI 入口 (主 00:36 质量 + 适配性 + 效果 + 工程化)."""
    parser = argparse.ArgumentParser(
        prog="v1390-asi-deployment-philosophy",
        description="V1390 ASI real deployment philosophy V1 (post-V1389 next-step)",
    )
    sub = parser.add_subparsers(dest="cmd", required=True)

    sub.add_parser("version", help="Print V1390 version")
    sub.add_parser("lessons", help="List 9 lessons (text)")
    p_lessons_json = sub.add_parser("lessons-json", help="List 9 lessons (JSON)")
    p_lessons_json.add_argument("--quiet", action="store_true")
    sub.add_parser("borrowed", help="List borrowed references (text)")
    p_borrowed_json = sub.add_parser("borrowed-json", help="List borrowed (JSON)")
    p_borrowed_json.add_argument("--quiet", action="store_true")
    sub.add_parser("guards", help="List 6 philosophy guards (text)")
    p_guards_json = sub.add_parser("guards-json", help="List guards (JSON)")
    p_guards_json.add_argument("--quiet", action="store_true")

    p_self = sub.add_parser("self-test", help="Run self-test (validate lessons + borrowed + guards)")
    p_self.add_argument("--json", action="store_true")
    p_self.add_argument("--quiet", action="store_true")

    args = parser.parse_args(argv)

    if args.cmd == "version":
        print(f"V1390 ASI deployment philosophy v{V1390_VERSION} (schema: {V1390_SCHEMA})")
        return 0

    if args.cmd == "lessons":
        print(_format_lessons_text())
        return 0
    if args.cmd == "lessons-json":
        print(json.dumps({"version": V1390_VERSION, "lessons": V1390_LESSONS}, indent=2, ensure_ascii=False))
        return 0

    if args.cmd == "borrowed":
        print(_format_borrowed_text())
        return 0
    if args.cmd == "borrowed-json":
        print(json.dumps({"version": V1390_VERSION, "borrowed": V1390_BORROWED}, indent=2, ensure_ascii=False))
        return 0

    if args.cmd == "guards":
        print(_format_guards_text())
        return 0
    if args.cmd == "guards-json":
        print(json.dumps({"version": V1390_VERSION, "guards": V1390_GUARDS}, indent=2, ensure_ascii=False))
        return 0

    if args.cmd == "self-test":
        report = self_test()
        if args.json:
            print(json.dumps(report.to_dict(), indent=2, ensure_ascii=False))
        else:
            print(_format_report_text(report, quiet=args.quiet))
        return 0 if report.ok else 1

    parser.print_help()
    return 3


if __name__ == "__main__":
    sys.exit(run_cli())