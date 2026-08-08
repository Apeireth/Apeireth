"""Phase 1391 v1391_policy_gate — V1391 ASI 真生产 policy gate (主 06:15 + 主 23:44 + 主 17:43 + 主 19:33 + 主 22:33 + 主 00:56 + 主 13:31).

V1391 = real production policy gate: YAML policy file defines pass/fail criteria.
- 真借鉴: OPA (Open Policy Agent) Rego / Sentinel / Conftest / Hadolint config
- 任何人能接手 (主 00:56): 1 个 YAML schema + 1 个 evaluator
- 实事求是 (主 17:43): 真给 YAML 解析 + 真给 pass/fail, 不假装 policy
- 不假装 (主 17:58): policy 是建议, 决策可 override

V1391 真生产 数据结构:
- PolicyRule: rule_id, severity, max_count (= -1 = unlimited)
- Policy: name, version, schema, rules: List[PolicyRule]
- PolicyResult: passed, violations, n_findings, n_violations, score (0-100)

V1391 真生产 CLI:
- version: V1391 version
- schema: 输出 YAML schema
- evaluate <policy.yaml> <target>: 真 evaluate policy 真 target
- evaluate <policy.yaml> --json-findings: 真 evaluate with JSON input
- demo: 内置真 policy 真 bad-deploy 真 evaluate
- popper: 10 真测试 pass
"""
from __future__ import annotations

import argparse
import json
import sys
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple

# V1391 真生产: YAML 解析 (主 17:43 实事求是; PyYAML 已装)
try:
    import yaml  # PyYAML 6.0.3+
    _YAML_AVAILABLE = True
except Exception:  # pragma: no cover
    yaml = None  # type: ignore[assignment]
    _YAML_AVAILABLE = False


V1391_VERSION = "0.1.0"
V1391_SCHEMA = "v1391.policy-gate/v1"

# V1391 真生产 默认 policy (主 17:43)
DEFAULT_POLICY_YAML = """\
# V1391 default policy — true production safe defaults
name: apeireth-default-policy
version: "0.1.0"
schema: v1391.policy-gate/v1
description: |
  Pass criteria:
  - zero errors (errors cause immediate failure)
  - max 5 warnings
  - max 20 info
  - no :latest tags
  - no plaintext secrets
rules:
  - rule_id: DEFAULT
    severity: error
    max_count: 0
  - rule_id: DEFAULT
    severity: warning
    max_count: 5
  - rule_id: DEFAULT
    severity: info
    max_count: 20
  - rule_id: COMPOSE-LATEST-TAG
    severity: error
    max_count: 0
  - rule_id: K8S-LATEST-TAG
    severity: error
    max_count: 0
  - rule_id: V1384-FROM-LATEST
    severity: error
    max_count: 0
  - rule_id: COMPOSE-PLAINTEXT-SECRET
    severity: error
    max_count: 0
  - rule_id: K8S-PLAINTEXT-SECRET
    severity: error
    max_count: 0
"""

# V1391 真生产 YAML schema (主 00:56 任何人都能接手)
V1391_POLICY_SCHEMA = """\
# V1391 Policy YAML schema (v1391.policy-gate/v1)
# A policy file has the following top-level fields:
#   name:        string (required) — policy name
#   version:     string (required) — policy version
#   schema:      string (default: v1391.policy-gate/v1)
#   description: string (optional) — human description
#   rules:       list of policy rules (required, non-empty)
#
# Each rule has:
#   rule_id:    string (default: "DEFAULT" = apply to all rules of this severity)
#   severity:   one of "error" / "warning" / "info" (required)
#   max_count:  integer (default: -1 = unlimited)
#               0 = outright fail; positive = max allowed count
#   description: string (optional)
#
# DEFAULT severity-count rules apply to all rules of that severity.
# Specific rule_id rules override the DEFAULT for that rule_id.
"""


# V1391 真生产 GUARDS (主 17:43 实事求是)
V1391_GUARDS: tuple = (
    "GUARD_POLICY_REAL",       # policy 真有 .rules 真非空
    "GUARD_YAML_PARSED",       # policy file 真 YAML 解析过
    "GUARD_EVALUATE_REAL",     # 真 evaluate 真 findings
    "GUARD_NO_CAP_CHANGE",     # 不改 ASI cap
    "GUARD_DETERMINISTIC",     # same input → same output
    "GUARD_HONEST_DISCLOSURE", # 不假装 policy
    "GUARD_CLI_RUNNABLE",      # CLI 真可跑
    "GUARD_DEFAULT_POLICY",    # 默认 policy 内置
)


# ============================================================================
# V1391 真生产 数据结构 (主 17:43)
# ============================================================================


@dataclass
class PolicyRule:
    """V1391 真生产 单条 policy rule (主 17:43)."""

    rule_id: str = "DEFAULT"     # "DEFAULT" = 适用所有
    severity: str = "error"      # error / warning / info
    max_count: int = 0           # -1 = unlimited
    description: str = ""

    def to_dict(self) -> Dict[str, Any]:
        return {
            "rule_id": self.rule_id,
            "severity": self.severity,
            "max_count": self.max_count,
            "description": self.description,
        }


@dataclass
class Policy:
    """V1391 真生产 policy = 1 个 name + N 条 rules (主 17:43)."""

    name: str = "default"
    version: str = V1391_VERSION
    schema: str = V1391_SCHEMA
    description: str = ""
    rules: List[PolicyRule] = field(default_factory=list)

    @classmethod
    def from_yaml(cls, path: str) -> "Policy":
        """V1391 真生产: 从 YAML 文件 load policy (主 17:43)."""
        if not _YAML_AVAILABLE:
            raise RuntimeError("PyYAML not available")
        with open(path, "r", encoding="utf-8") as f:
            data = yaml.safe_load(f)
        if not isinstance(data, dict):
            raise ValueError(f"policy file {path} not dict")
        rules_data = data.get("rules", [])
        if not rules_data:
            raise ValueError(f"policy file {path} has no rules")
        rules: List[PolicyRule] = []
        for rd in rules_data:
            rules.append(PolicyRule(
                rule_id=rd.get("rule_id", "DEFAULT"),
                severity=rd.get("severity", "error"),
                max_count=rd.get("max_count", -1),
                description=rd.get("description", ""),
            ))
        return cls(
            name=data.get("name", "default"),
            version=data.get("version", V1391_VERSION),
            schema=data.get("schema", V1391_SCHEMA),
            description=data.get("description", ""),
            rules=rules,
        )

    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> "Policy":
        """V1391 真生产: 从 dict 构造 policy (主 17:43)."""
        rules_data = data.get("rules", [])
        rules = [PolicyRule(
            rule_id=rd.get("rule_id", "DEFAULT"),
            severity=rd.get("severity", "error"),
            max_count=rd.get("max_count", -1),
            description=rd.get("description", ""),
        ) for rd in rules_data]
        return cls(
            name=data.get("name", "default"),
            version=data.get("version", V1391_VERSION),
            schema=data.get("schema", V1391_SCHEMA),
            description=data.get("description", ""),
            rules=rules,
        )

    @classmethod
    def default_policy(cls) -> "Policy":
        """V1391 真生产: 内置 default policy (主 17:43)."""
        return cls.from_dict(yaml.safe_load(DEFAULT_POLICY_YAML))

    def to_dict(self) -> Dict[str, Any]:
        return {
            "name": self.name,
            "version": self.version,
            "schema": self.schema,
            "description": self.description,
            "rules": [r.to_dict() for r in self.rules],
        }


@dataclass
class PolicyViolation:
    """V1391 真生产 单条 violation (主 17:43)."""

    rule_id: str       # "DEFAULT" or specific
    severity: str
    actual_count: int
    max_count: int     # -1 = unlimited
    why: str           # violation reason

    def to_dict(self) -> Dict[str, Any]:
        return {
            "rule_id": self.rule_id,
            "severity": self.severity,
            "actual_count": self.actual_count,
            "max_count": self.max_count,
            "why": self.why,
        }


@dataclass
class PolicyResult:
    """V1391 真生产 evaluation result (主 17:43)."""

    policy_name: str = "default"
    policy_version: str = V1391_VERSION
    target: str = ""
    passed: bool = True
    score: int = 100           # 0-100
    n_findings: int = 0
    n_errors: int = 0
    n_warnings: int = 0
    n_info: int = 0
    n_violations: int = 0
    by_severity: Dict[str, int] = field(default_factory=dict)
    by_rule: Dict[str, int] = field(default_factory=dict)
    violations: List[PolicyViolation] = field(default_factory=list)

    def to_dict(self) -> Dict[str, Any]:
        return {
            "policy_name": self.policy_name,
            "policy_version": self.policy_version,
            "target": self.target,
            "passed": self.passed,
            "score": self.score,
            "n_findings": self.n_findings,
            "n_errors": self.n_errors,
            "n_warnings": self.n_warnings,
            "n_info": self.n_info,
            "n_violations": self.n_violations,
            "by_severity": self.by_severity,
            "by_rule": self.by_rule,
            "violations": [v.to_dict() for v in self.violations],
        }


# ============================================================================
# V1391 真生产 evaluate (主 17:43)
# ============================================================================


def get_severity_for_rule(rule_id: str, hint_severity: Optional[str] = None) -> str:
    """V1391 真生产: 返回 rule 的 severity. 用 V1390 hints 库推断; fallback 到 'warning'."""
    if hint_severity:
        return hint_severity
    # V1390 fallback
    try:
        from v1390_remediation_hints import get_hint as _get_hint
        h = _get_hint(rule_id)
        if h:
            return h.severity
    except Exception:
        pass
    return "warning"


def evaluate(
    policy: Policy,
    findings: List[Dict[str, Any]],
    target: str = "",
) -> PolicyResult:
    """V1391 真生产: 真 evaluate policy vs findings (主 17:43).

    findings: list of dict with 'rule_id' field (or 'new_by_rule' V1388 格式).
    """
    res = PolicyResult(
        policy_name=policy.name,
        policy_version=policy.version,
        target=target,
    )
    # 1. 展开 findings
    rule_counts: Dict[str, int] = {}
    for f in findings:
        if not isinstance(f, dict):
            continue
        if "rule_id" in f:
            rid = f["rule_id"]
            rule_counts[rid] = rule_counts.get(rid, 0) + 1
        elif "new_by_rule" in f and isinstance(f["new_by_rule"], dict):
            for rid, count in f["new_by_rule"].items():
                rule_counts[rid] = rule_counts.get(rid, 0) + count
    res.n_findings = sum(rule_counts.values())
    res.by_rule = dict(rule_counts)
    # 2. 按 severity 汇总
    sev_counts: Dict[str, int] = {"error": 0, "warning": 0, "info": 0}
    for rid, count in rule_counts.items():
        sev = get_severity_for_rule(rid)
        sev_counts[sev] = sev_counts.get(sev, 0) + count
    res.n_errors = sev_counts["error"]
    res.n_warnings = sev_counts["warning"]
    res.n_info = sev_counts["info"]
    res.by_severity = dict(sev_counts)
    # 3. 真 evaluate vs policy
    # 3a. DEFAULT severity rules
    for sev in ("error", "warning", "info"):
        # 找最严的 DEFAULT rule for this severity
        default_rules = [r for r in policy.rules if r.rule_id == "DEFAULT" and r.severity == sev]
        if not default_rules:
            continue
        for dr in default_rules:
            actual = sev_counts[sev]
            if dr.max_count >= 0 and actual > dr.max_count:
                res.violations.append(PolicyViolation(
                    rule_id="DEFAULT",
                    severity=sev,
                    actual_count=actual,
                    max_count=dr.max_count,
                    why=f"{sev} count {actual} > max {dr.max_count} (DEFAULT)",
                ))
    # 3b. Specific rule_id rules (override DEFAULT)
    for r in policy.rules:
        if r.rule_id == "DEFAULT":
            continue
        actual = rule_counts.get(r.rule_id, 0)
        if r.max_count >= 0 and actual > r.max_count:
            res.violations.append(PolicyViolation(
                rule_id=r.rule_id,
                severity=r.severity,
                actual_count=actual,
                max_count=r.max_count,
                why=f"{r.rule_id} count {actual} > max {r.max_count}",
            ))
    # 4. 决策
    res.n_violations = len(res.violations)
    res.passed = res.n_violations == 0
    # 5. score 0-100
    # error: -10 each, warning: -3 each, info: -1 each; floor at 0
    score = 100 - res.n_errors * 10 - res.n_warnings * 3 - res.n_info * 1
    res.score = max(0, score)
    return res


def popper_self_test() -> Dict[str, Any]:
    """V1391 真生产 Popper self-test (主 17:43)."""
    failures: List[str] = []
    # Test 1: default policy 真有 rules
    p = Policy.default_policy()
    if len(p.rules) < 5:
        failures.append(f"default policy has <5 rules: {len(p.rules)}")
    # Test 2: 真 evaluate 真 findings
    findings = [
        {"rule_id": "DL3008"},
        {"rule_id": "DL3008"},
        {"rule_id": "COMPOSE-PRIVILEGED"},
        {"rule_id": "COMPOSE-LATEST-TAG"},
        {"rule_id": "COMPOSE-LATEST-TAG"},
        {"rule_id": "COMPOSE-LATEST-TAG"},
        {"rule_id": "K8S-NO-RESOURCE-LIMITS"},
    ]
    res = evaluate(p, findings, target="bad")
    if res.n_findings != 7:
        failures.append(f"n_findings expected 7, got {res.n_findings}")
    # Test 3: clean findings pass
    clean = evaluate(p, [], target="clean")
    if not clean.passed:
        failures.append("clean findings should pass")
    if clean.score != 100:
        failures.append(f"clean score should be 100, got {clean.score}")
    # Test 4: errors → -10 each
    if not res.passed:
        if res.n_errors < 1:
            failures.append("expected at least 1 error")
        # 3 COMPOSE-LATEST-TAG is error, 1 COMPOSE-PRIVILEGED is error
        # 2 DL3008 is warning, 1 K8S-NO-RESOURCE-LIMITS is warning = 3 warnings
        if res.n_errors != 4:
            failures.append(f"expected 4 errors, got {res.n_errors}")
    # Test 5: 0-findings clean has 0 violations
    if clean.n_violations != 0:
        failures.append(f"clean n_violations should be 0, got {clean.n_violations}")
    # Test 6: violations 至少 1 (COMPOSE-LATEST-TAG max_count=0)
    if res.n_violations < 1:
        failures.append("expected at least 1 violation")
    # Test 7: from_dict / to_dict roundtrip
    d = p.to_dict()
    p2 = Policy.from_dict(d)
    if len(p2.rules) != len(p.rules):
        failures.append(f"roundtrip rules mismatch: {len(p2.rules)} vs {len(p.rules)}")
    # Test 8: from_yaml 解析 DEFAULT_POLICY_YAML
    import tempfile
    with tempfile.NamedTemporaryFile(mode="w", suffix=".yaml", delete=False, encoding="utf-8") as f:
        f.write(DEFAULT_POLICY_YAML)
        tp = f.name
    try:
        p3 = Policy.from_yaml(tp)
        if len(p3.rules) < 5:
            failures.append(f"from_yaml: <5 rules")
    finally:
        Path(tp).unlink()
    # Test 9: get_severity_for_rule 真能 infer
    sev = get_severity_for_rule("DL3008")
    if sev not in ("error", "warning", "info"):
        failures.append(f"get_severity_for_rule(DL3008) bad: {sev}")
    # Test 10: bad_yaml 抛错
    try:
        with tempfile.NamedTemporaryFile(mode="w", suffix=".yaml", delete=False, encoding="utf-8") as f:
            f.write("not a dict\n")
            tp = f.name
        try:
            Policy.from_yaml(tp)
            failures.append("from_yaml should fail on non-dict")
        except ValueError:
            pass
        finally:
            Path(tp).unlink()
    except Exception:
        pass
    return {
        "passed": len(failures) == 0,
        "failures": failures,
        "n_tested": 10,
    }


# ============================================================================
# V1391 CLI (主 17:43 真可执行)
# ============================================================================


def _run_v1390_apply(target: str) -> List[Dict[str, Any]]:
    """V1391 真生产: 真调 V1390 apply 拿 findings."""
    try:
        from v1390_remediation_hints import apply_to_findings, _run_scan_v1387  # type: ignore
    except Exception:
        import sys as _sys
        _v1391_dir = Path(__file__).resolve().parent
        if str(_v1391_dir) not in _sys.path:
            _sys.path.insert(0, str(_v1391_dir))
        try:
            from v1390_remediation_hints import _run_scan_v1387  # type: ignore
        except Exception:
            return []
    raw = _run_scan_v1387(target)
    # 真转换为 rule_id findings
    findings: List[Dict[str, Any]] = []
    for f in raw:
        if isinstance(f, dict) and "rule_id" in f:
            findings.append(f)
    return findings


def _format_result_text(res: PolicyResult) -> str:
    """V1391 真生产: 文本格式 (主 17:43)."""
    lines = []
    status = "PASS" if res.passed else "FAIL"
    lines.append(f"V1391 policy gate {status} (score={res.score})")
    lines.append(f"  policy: {res.policy_name} v{res.policy_version}")
    lines.append(f"  target: {res.target}")
    lines.append(f"  findings: {res.n_findings} (errors={res.n_errors}, warnings={res.n_warnings}, info={res.n_info})")
    lines.append(f"  violations: {res.n_violations}")
    for v in res.violations[:5]:
        lines.append(f"    - {v.severity} {v.rule_id}: count={v.actual_count} max={v.max_count}")
    if len(res.violations) > 5:
        lines.append(f"    ... and {len(res.violations) - 5} more")
    return "\n".join(lines)


def run_cli(argv: Optional[List[str]] = None) -> int:
    """V1391 真生产 CLI 主入口 (主 17:43 真可执行)."""
    parser = argparse.ArgumentParser(
        prog="v1391-policy-gate",
        description=f"V1391 real production policy gate (v{V1391_VERSION})",
    )
    sub = parser.add_subparsers(dest="cmd", required=False)

    sub.add_parser("version", help="V1391 version")
    sub.add_parser("schema", help="print YAML schema")
    sub.add_parser("default-policy", help="print default policy YAML")

    p_eval = sub.add_parser("evaluate", help="evaluate policy against target")
    p_eval.add_argument("policy", help="policy YAML file")
    p_eval.add_argument("target", help="target directory to scan")
    p_eval.add_argument("--json", action="store_true", help="JSON output")

    p_eval2 = sub.add_parser("evaluate-json", help="evaluate policy against JSON findings")
    p_eval2.add_argument("policy", help="policy YAML file")
    p_eval2.add_argument("findings_json", help="JSON file with findings")

    p_demo = sub.add_parser("demo", help="run V1391 demo with default policy")

    sub.add_parser("popper", help="V1391 Popper self-test")

    args = parser.parse_args(argv)
    cmd = args.cmd or "version"

    if cmd == "version":
        print(f"V1391 policy gate v{V1391_VERSION} (schema {V1391_SCHEMA})")
        return 0
    if cmd == "schema":
        print(V1391_POLICY_SCHEMA)
        return 0
    if cmd == "default-policy":
        print(DEFAULT_POLICY_YAML)
        return 0
    if cmd == "evaluate":
        policy = Policy.from_yaml(args.policy)
        findings = _run_v1390_apply(args.target)
        if not findings:
            print(f"no findings or scan unavailable: {args.target}", file=sys.stderr)
        res = evaluate(policy, findings, target=args.target)
        if args.json:
            print(json.dumps(res.to_dict(), indent=2, ensure_ascii=False))
        else:
            print(_format_result_text(res))
        return 0 if res.passed else 1
    if cmd == "evaluate-json":
        policy = Policy.from_yaml(args.policy)
        with open(args.findings_json, "r", encoding="utf-8") as f:
            data = json.load(f)
        if isinstance(data, list):
            findings = data
        elif isinstance(data, dict):
            findings = data.get("findings", [])
        else:
            findings = []
        res = evaluate(policy, findings, target=args.findings_json)
        print(json.dumps(res.to_dict(), indent=2, ensure_ascii=False))
        return 0 if res.passed else 1
    if cmd == "demo":
        policy = Policy.default_policy()
        findings = [
            {"rule_id": "COMPOSE-LATEST-TAG"},
            {"rule_id": "COMPOSE-LATEST-TAG"},
            {"rule_id": "DL3008"},
        ]
        res = evaluate(policy, findings, target="demo")
        print(_format_result_text(res))
        return 0 if res.passed else 1
    if cmd == "popper":
        r = popper_self_test()
        print(json.dumps(r, indent=2, ensure_ascii=False))
        return 0 if r["passed"] else 1

    parser.print_help()
    return 1


if __name__ == "__main__":
    sys.exit(run_cli())
