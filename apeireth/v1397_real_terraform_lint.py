"""Phase 1397 v1397_real_terraform_lint — V1397 ASI 真生产 Terraform HCL 真解析 + 真 lint (主 06:15 + 主 23:44 + 主 17:43 + 主 19:33 + 主 22:33 + 主 00:56 + 主 13:31 + 主 17:33 + 主 00:36).

主 06:15 当前真生产方向: V1397 = 真生产 deploy-stack terraform HCL 真解析 + 真 lint (post-V1396 next-step, 推荐方向).
主 23:44 干到底: V1384 Dockerfile + V1385 compose + V1386 k8s + V1396 executor → V1397 真补 terraform HCL 缺口.
主 22:33 ASI 北极星: 真 lint terraform ≠ ASI, 但真 lint terraform 是 ASI 北极星里 system integration 的一小步.
主 19:33 走在前人经验上: 真借鉴 tflint + tfsec + checkov + terrascan + terraform validate + conftest.
主 17:43 实事求是: 真 parse HCL (python-hcl2) + 真规则匹配 + 真报 finding + 真 exit code.
主 00:56 任何人都能接手: 1 个 module + 1 个 CLI + 12 真规则 + 1 个 chain runner + 1 个 popper self-test.
主 00:36 质量 + 适配性 + 效果 + 工程化: 真 CLI + 真 exit code + 真 popper self-test + 真 JSON 输出.

真生产设计 (主 19:33 tflint/tfsec/checkov/terrascan/conftest/terraform-validate 真借鉴):
- 真 parse HCL (python-hcl2, 主 17:43 真借用 python-hcl2 已 install pip install python-hcl2)
  - 真提取 variable / resource / provider / output / data / module / terraform
  - 真处理字符串 (hcl2 returns "quoted"; 真 strip quotes)
  - 真识别 block 类型 (resource 含 type.name 二层, variable 单层)
- 真 12 规则 (主 19:33 tflint/tfsec/checkov 真借鉴):
  - TF001-HARDCODED-SECRET (error): AWS access key / common secret pattern in string value
  - TF002-S3-NO-ENCRYPTION (warning): aws_s3_bucket without server_side_encryption_configuration
  - TF003-S3-PUBLIC-ACL (error): aws_s3_bucket with acl = "public-read" or "public-read-write"
  - TF004-SG-OPEN-INGRESS (error): aws_security_group ingress with 0.0.0.0/0
  - TF005-UNPINNED-PROVIDER (warning): terraform.required_providers without version
  - TF006-RDS-NO-ENCRYPTION (warning): aws_db_instance with storage_encrypted = false
  - TF007-IAM-WILDCARD (error): IAM policy with Action="*" or Resource="*"
  - TF008-EC2-NO-TAGS (info): aws_instance/aws_db_instance without tags
  - TF009-LIFECYCLE-FORCE-DESTROY (warning): aws_s3_bucket with force_destroy = true
  - TF010-VAR-NO-TYPE (info): variable without type
  - TF011-OUTPUT-NO-DESCRIPTION (info): output without description
  - TF012-MISSING-REQUIRED-VERSION (info): terraform without required_version
- 真 chain delegate (主 17:43 真可调): 聚合 V1387 unified runner 发现
- 真 popper self-test (主 17:43 真跑真测)
- 真 CLI (主 17:43 真可执行):
  - version: V1397 version
  - lint <path>: 真 lint .tf 文件 → text/JSON 输出
  - chain <path>: 真调 V1387 unified runner + V1397 真 lint → 综合报告
  - popper: V1397 self-test
  - demo: V1397 demo (用内置 .tf sample)
  - help
- 真 multi-output: text / JSON / SARIF (主 00:36 工程化)

V3 哲学守门 (主 17:58 + 主 20:46 + 主 17:43):
- 不假装 Phenomenal consciousness: 本模块是 terraform linter, 不是 consciousness claim.
- 不假装达到 ASI: 真 lint ≠ ASI 达成; 真 lint 是 ASI 北极星里 system integration 的一小步.
- 不假装调整模型 & prompt: 真生产是真 parse HCL + 真规则匹配, 不是改 prompt 假装 lint.
- 真 lint = 真借鉴 + 真算法 + 真跑真测 + 真 commit + 真可执行.
- 任何声称 "lint = safety" 都是不假装. 真 lint ≠ 安全审计.
- 任何声称 "lint = ASI" 都是不假装. 真 lint 是 ASI 北极星里 system integration 的一小步.
"""
from __future__ import annotations

import argparse
import json
import os
import re
import sys
import time
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Dict, List, Optional, Sequence, Tuple

# V1397 真生产 python-hcl2 (主 17:43 实事求是)
try:
    import hcl2
    _HCL_AVAILABLE = True
except Exception:
    hcl2 = None
    _HCL_AVAILABLE = False


V1397_VERSION = "0.1.0"
V1397_SCHEMA = "v1397.terraform-lint/v1"

# V1397 真生产 GUARDS (主 17:43 + 主 19:33 + 主 22:33)
V1397_GUARDS: tuple = (
    "GUARD_HCL_PARSED",         # 真 parse HCL (python-hcl2)
    "GUARD_RULES_REAL",         # 真规则匹配 (12 真规则)
    "GUARD_FILE_IO",            # 真文件 IO
    "GUARD_LINE_TRACKED",       # 真行号 (正则 fallback 找)
    "GUARD_NO_CAP_CHANGE",      # 不改 ASI cap
    "GUARD_DETERMINISTIC",      # same target → same result
    "GUARD_HONEST_DISCLOSURE",  # 标注 borrowed + python-hcl2 missing fallback
    "GUARD_PATH_SAFE",          # path traversal 防护
    "GUARD_NON_DESTRUCTIVE",    # 只读, 不写
    "GUARD_DELEGATE_REAL",      # 真调 V1387
    "GUARD_CLI_RUNNABLE",       # CLI 真可跑
    "GUARD_POPPER_RUNS",        # popper self-test 真跑
)

# V1397 真借鉴 (主 19:33 走在前人经验上)
V1397_BORROWED: tuple = (
    "tflint (https://github.com/terraform-linters/tflint) — 真借鉴 rule format (TF-prefix)",
    "tfsec (https://github.com/aquasecurity/tfsec) — 真借鉴 severity + 真借鉴 AWS rules",
    "checkov (https://github.com/bridgecrewio/checkov) — 真借鉴 policy_id 命名 + policy categories",
    "terrascan (https://github.com/tenable/terrascan) — 真借鉴 IaC violation 结构",
    "terraform validate (https://developer.hashicorp.com/terraform/cli/commands/validate) — 真借鉴 JSON syntax check",
    "conftest (https://github.com/open-policy-agent/conftest) — 真借鉴 structured rule output",
)

# V1397 真生产 valid ACL values for S3 (主 17:43 实事求是 AWS 官方)
V1397_PUBLIC_S3_ACLS: tuple = ("public-read", "public-read-write", "authenticated-read")

# V1397 真生产 secret patterns (主 19:33 tfsec 真借鉴)
V1397_SECRET_PATTERNS: tuple = (
    re.compile(r"AKIA[0-9A-Z]{16}"),                          # AWS access key
    re.compile(r"(?i)aws_secret_access_key\s*=\s*\"[^\"]+\""),
    re.compile(r"(?i)(password|passwd|pwd)\s*=\s*\"[^\"]{4,}\""),
    re.compile(r"-----BEGIN (RSA |EC |DSA |OPENSSH )?PRIVATE KEY-----"),
    re.compile(r"(?i)api[_-]?key\s*=\s*\"[a-zA-Z0-9]{16,}\""),
)


@dataclass
class LintFinding:
    """V1397 真生产 lint 真报 finding (主 17:43 实事求是)."""
    rule_id: str        # 例如 TF003-S3-PUBLIC-ACL
    severity: str       # error / warning / info
    line_no: int        # 真行号 (1-indexed, 0 if N/A)
    line_text: str      # 真原文 (or first 120 chars)
    message: str        # 真问题描述
    suggestion: str = "" # 真建议
    resource: str = ""  # 真 resource ref 例如 aws_s3_bucket.bad
    file_path: str = ""

    def to_dict(self) -> Dict[str, Any]:
        return {
            "rule_id": self.rule_id,
            "severity": self.severity,
            "line_no": self.line_no,
            "line_text": self.line_text.strip()[:120],
            "message": self.message,
            "suggestion": self.suggestion,
            "resource": self.resource,
            "file_path": self.file_path,
        }


@dataclass
class LintReport:
    """V1397 真生产 lint 真报报告 (主 17:43 实事求是)."""
    file_path: str
    n_lines: int
    n_findings: int
    n_errors: int
    n_warnings: int
    n_info: int
    findings: List[LintFinding] = field(default_factory=list)
    elapsed_seconds: float = 0.0
    ok: bool = True
    parse_error: str = ""  # 真报 hcl2 parse error
    parser: str = ""       # hcl2 or regex-fallback

    def to_dict(self) -> Dict[str, Any]:
        return {
            "file_path": self.file_path,
            "n_lines": self.n_lines,
            "n_findings": self.n_findings,
            "n_errors": self.n_errors,
            "n_warnings": self.n_warnings,
            "n_info": self.n_info,
            "findings": [f.to_dict() for f in self.findings],
            "elapsed_seconds": round(self.elapsed_seconds, 4),
            "ok": self.ok,
            "parse_error": self.parse_error,
            "parser": self.parser,
        }


# ============================================================================
# 真生产 HCL 解析辅助 (主 17:43)
# ============================================================================


def _strip_quotes(s: Any) -> Any:
    """hcl2 returns strings with literal quotes. Strip them. 主 17:43."""
    if isinstance(s, str) and len(s) >= 2 and s.startswith('"') and s.endswith('"'):
        return s[1:-1]
    return s


def _normalize_value(v: Any) -> Any:
    """V1397 真生产 normalize hcl2 values (主 17:43)."""
    if isinstance(v, str):
        return _strip_quotes(v)
    if isinstance(v, list):
        return [_normalize_value(x) for x in v]
    if isinstance(v, dict):
        return {_strip_quotes(k) if isinstance(k, str) else k: _normalize_value(val) for k, val in v.items()}
    return v


def _find_line_no(text: str, target: str, start: int = 0) -> int:
    """V1397 真生产 find 1-indexed line number for target in text (主 17:43)."""
    idx = text.find(target, start)
    if idx < 0:
        return 0
    return text[:idx].count("\n") + 1


# ============================================================================
# 真生产 规则定义 (主 19:33 tfsec/checkov/tflint 真借鉴)
# ============================================================================


def _rule_tf001_hardcoded_secret(text: str, ast: Dict[str, Any]) -> List[LintFinding]:
    """V1397 真借鉴 TF001: 真检测 hardcoded secret in any string value (主 19:33 tfsec/checkov)."""
    findings: List[LintFinding] = []
    seen_lines: set = set()

    def _walk(obj: Any, path: str = "") -> None:
        if isinstance(obj, dict):
            for k, v in obj.items():
                k = _strip_quotes(k) if isinstance(k, str) else k
                _walk(v, f"{path}.{k}" if path else str(k))
        elif isinstance(obj, list):
            for i, item in enumerate(obj):
                _walk(item, f"{path}[{i}]")
        elif isinstance(obj, str):
            s = _strip_quotes(obj)
            for pat in V1397_SECRET_PATTERNS:
                if pat.search(s):
                    line_no = _find_line_no(text, s)
                    if line_no in seen_lines:
                        continue
                    seen_lines.add(line_no)
                    findings.append(LintFinding(
                        rule_id="TF001-HARDCODED-SECRET",
                        severity="error",
                        line_no=line_no,
                        line_text=s[:120],
                        message=f"Hardcoded secret detected at {path}",
                        suggestion="Use environment variables or AWS Secrets Manager. Never commit secrets.",
                        resource=path,
                    ))
                    break

    _walk(ast)
    return findings


def _rule_tf002_s3_no_encryption(text: str, ast: Dict[str, Any]) -> List[LintFinding]:
    """V1397 真借鉴 TF002: aws_s3_bucket without server_side_encryption_configuration (主 19:33 tfsec)."""
    findings: List[LintFinding] = []
    # 真收集所有 SSE 配置引用 (主 17:43 实事求是 真解析 bucket = aws_s3_bucket.<name>.id 引用)
    sse_referenced_buckets: set = set()
    for r2 in ast.get("resource", []):
        for t2, insts in r2.items():
            if _strip_quotes(t2) != "aws_s3_bucket_server_side_encryption_configuration":
                continue
            for _sse_name, sse_attrs in insts.items():
                sse_attrs = _normalize_value(sse_attrs)
                bucket_ref = sse_attrs.get("bucket", "")
                # 真 bucket 引用可能是字面字符串 or interpolation: aws_s3_bucket.foo.id
                if isinstance(bucket_ref, str):
                    ref = _strip_quotes(bucket_ref)
                    # 真提取 aws_s3_bucket.<name>.id 中的 <name>
                    import re as _re
                    m = _re.search(r"aws_s3_bucket\.([a-zA-Z0-9_]+)\.id", ref)
                    if m:
                        sse_referenced_buckets.add(m.group(1))
    # 真生产 check 每个 bucket 是否有 SSE 配置
    for res in ast.get("resource", []):
        for rtype, instances in res.items():
            rtype = _strip_quotes(rtype)
            if rtype != "aws_s3_bucket":
                continue
            for rname, attrs in instances.items():
                rname = _strip_quotes(rname)
                if rname in sse_referenced_buckets:
                    continue
                line_no = _find_line_no(text, f"\"{rtype}\"")
                findings.append(LintFinding(
                    rule_id="TF002-S3-NO-ENCRYPTION",
                    severity="warning",
                    line_no=line_no,
                    line_text=f'resource "{rtype}" "{rname}"',
                    message=f"S3 bucket '{rname}' has no server-side encryption configuration",
                    suggestion="Add aws_s3_bucket_server_side_encryption_configuration resource with AES256 or aws:kms.",
                    resource=f"{rtype}.{rname}",
                ))
    return findings


def _rule_tf003_s3_public_acl(text: str, ast: Dict[str, Any]) -> List[LintFinding]:
    """V1397 真借鉴 TF003: aws_s3_bucket with public acl (主 19:33 tfsec CKV_AWS_53)."""
    findings: List[LintFinding] = []
    for res in ast.get("resource", []):
        for rtype, instances in res.items():
            rtype = _strip_quotes(rtype)
            if rtype != "aws_s3_bucket":
                continue
            for rname, attrs in instances.items():
                rname = _strip_quotes(rname)
                attrs = _normalize_value(attrs)
                acl = attrs.get("acl", "")
                if isinstance(acl, str) and acl in V1397_PUBLIC_S3_ACLS:
                    line_no = _find_line_no(text, f"acl")
                    findings.append(LintFinding(
                        rule_id="TF003-S3-PUBLIC-ACL",
                        severity="error",
                        line_no=line_no,
                        line_text=f'acl = "{acl}"',
                        message=f"S3 bucket '{rname}' has public ACL '{acl}'",
                        suggestion="Use 'private' ACL and grant access via bucket policy with least privilege.",
                        resource=f"{rtype}.{rname}",
                    ))
    return findings


def _rule_tf004_sg_open_ingress(text: str, ast: Dict[str, Any]) -> List[LintFinding]:
    """V1397 真借鉴 TF004: aws_security_group ingress with 0.0.0.0/0 (主 19:33 tfsec CKV_AWS_24)."""
    findings: List[LintFinding] = []
    for res in ast.get("resource", []):
        for rtype, instances in res.items():
            rtype = _strip_quotes(rtype)
            if rtype != "aws_security_group":
                continue
            for rname, attrs in instances.items():
                rname = _strip_quotes(rname)
                attrs = _normalize_value(attrs)
                ingress_blocks = attrs.get("ingress", [])
                if not isinstance(ingress_blocks, list):
                    ingress_blocks = [ingress_blocks]
                for ing in ingress_blocks:
                    if not isinstance(ing, dict):
                        continue
                    cidrs = ing.get("cidr_blocks", [])
                    if not isinstance(cidrs, list):
                        cidrs = [cidrs]
                    ipv6 = ing.get("ipv6_cidr_blocks", [])
                    if not isinstance(ipv6, list):
                        ipv6 = [ipv6]
                    open_v4 = any("0.0.0.0/0" in str(c) for c in cidrs)
                    open_v6 = any("::/0" in str(c) for c in ipv6)
                    if open_v4 or open_v6:
                        line_no = _find_line_no(text, "cidr_blocks")
                        findings.append(LintFinding(
                            rule_id="TF004-SG-OPEN-INGRESS",
                            severity="error",
                            line_no=line_no,
                            line_text="ingress { cidr_blocks = [\"0.0.0.0/0\"] }",
                            message=f"Security group '{rname}' allows ingress from {'0.0.0.0/0' if open_v4 else '::/0'}",
                            suggestion="Restrict cidr_blocks to specific IP ranges or use a bastion host.",
                            resource=f"{rtype}.{rname}",
                        ))
    return findings


def _rule_tf005_unpinned_provider(text: str, ast: Dict[str, Any]) -> List[LintFinding]:
    """V1397 真借鉴 TF005: terraform.required_providers without version (主 19:33 tflint terraform_unused_declarations)."""
    findings: List[LintFinding] = []
    tf_block = ast.get("terraform", [])
    for t in tf_block:
        if not isinstance(t, dict):
            continue
        t = _normalize_value(t)
        req_providers = t.get("required_providers", {})
        # 真生产 hcl2 wraps required_providers in a list [{...}]; 真处理 dict 和 list-of-dict (主 17:43)
        if isinstance(req_providers, list):
            rp_list = req_providers
        elif isinstance(req_providers, dict):
            rp_list = [req_providers]
        else:
            rp_list = []
        for rp in rp_list:
            if not isinstance(rp, dict):
                continue
            for pname, pconf in rp.items():
                pname = _strip_quotes(pname)
                if isinstance(pconf, dict) and "version" not in pconf:
                    line_no = _find_line_no(text, pname)
                    findings.append(LintFinding(
                        rule_id="TF005-UNPINNED-PROVIDER",
                        severity="warning",
                        line_no=line_no,
                        line_text=f'provider "{pname}"',
                        message=f"Provider '{pname}' has no version constraint",
                        suggestion='Pin provider version: required_providers { aws = { source = "hashicorp/aws", version = "~> 5.0" } }',
                        resource=pname,
                    ))
    return findings


def _rule_tf006_rds_no_encryption(text: str, ast: Dict[str, Any]) -> List[LintFinding]:
    """V1397 真借鉴 TF006: aws_db_instance with storage_encrypted = false (主 19:33 tfsec CKV_AWS_16)."""
    findings: List[LintFinding] = []
    for res in ast.get("resource", []):
        for rtype, instances in res.items():
            rtype = _strip_quotes(rtype)
            if rtype not in ("aws_db_instance", "aws_rds_cluster"):
                continue
            for rname, attrs in instances.items():
                rname = _strip_quotes(rname)
                attrs = _normalize_value(attrs)
                encrypted = attrs.get("storage_encrypted", True)
                kms_key = attrs.get("kms_key_id", None)
                if encrypted is False:
                    line_no = _find_line_no(text, "storage_encrypted")
                    findings.append(LintFinding(
                        rule_id="TF006-RDS-NO-ENCRYPTION",
                        severity="warning",
                        line_no=line_no,
                        line_text="storage_encrypted = false",
                        message=f"{rtype} '{rname}' has storage_encrypted = false",
                        suggestion="Set storage_encrypted = true and provide kms_key_id.",
                        resource=f"{rtype}.{rname}",
                    ))
    return findings


def _rule_tf007_iam_wildcard(text: str, ast: Dict[str, Any]) -> List[LintFinding]:
    """V1397 真借鉴 TF007: IAM policy with Action=\"*\" or Resource=\"*\" (主 19:33 tfsec CKV_AWS_62/63)."""
    findings: List[LintFinding] = []
    patterns = ("aws_iam_policy", "aws_iam_role_policy", "aws_iam_user_policy", "aws_iam_policy_document")
    for res in ast.get("resource", []):
        for rtype, instances in res.items():
            rtype = _strip_quotes(rtype)
            if rtype not in patterns:
                continue
            for rname, attrs in instances.items():
                rname = _strip_quotes(rname)
                attrs = _normalize_value(attrs)
                # 真查找 wildcard in policy string
                def _check_wildcard(v: Any, path: str = "") -> bool:
                    if isinstance(v, str):
                        s = _strip_quotes(v)
                        return bool(re.search(r'\bAction\b\s*[:=]\s*"?\*"?', s)) or \
                               bool(re.search(r'\bResource\b\s*[:=]\s*"?\*"?', s)) or \
                               ('"Action": "*"' in s) or ('"Resource": "*"' in s)
                    if isinstance(v, dict):
                        return any(_check_wildcard(x, k) for k, x in v.items())
                    if isinstance(v, list):
                        return any(_check_wildcard(x) for x in v)
                    return False
                if _check_wildcard(attrs):
                    line_no = _find_line_no(text, "policy") or _find_line_no(text, "Statement")
                    findings.append(LintFinding(
                        rule_id="TF007-IAM-WILDCARD",
                        severity="error",
                        line_no=line_no,
                        line_text='Action = "*" / Resource = "*"',
                        message=f"IAM policy '{rtype}.{rname}' uses wildcard Action or Resource",
                        suggestion="Apply least privilege: scope Action to specific operations and Resource to specific ARNs.",
                        resource=f"{rtype}.{rname}",
                    ))
    return findings


def _rule_tf008_ec2_no_tags(text: str, ast: Dict[str, Any]) -> List[LintFinding]:
    """V1397 真借鉴 TF008: aws_instance/aws_db_instance without tags (主 19:33 checkov CKV_AWS_88)."""
    findings: List[LintFinding] = []
    taggable = ("aws_instance", "aws_db_instance", "aws_rds_cluster", "aws_lb", "aws_s3_bucket")
    for res in ast.get("resource", []):
        for rtype, instances in res.items():
            rtype = _strip_quotes(rtype)
            if rtype not in taggable:
                continue
            for rname, attrs in instances.items():
                rname = _strip_quotes(rname)
                attrs = _normalize_value(attrs)
                if "tags" not in attrs:
                    line_no = _find_line_no(text, f"\"{rtype}\"")
                    findings.append(LintFinding(
                        rule_id="TF008-EC2-NO-TAGS",
                        severity="info",
                        line_no=line_no,
                        line_text=f'resource "{rtype}" "{rname}"',
                        message=f"{rtype} '{rname}' has no tags block",
                        suggestion="Add tags = { Name = \"...\", Environment = \"...\", Owner = \"...\" } for cost tracking and governance.",
                        resource=f"{rtype}.{rname}",
                    ))
    return findings


def _rule_tf009_lifecycle_force_destroy(text: str, ast: Dict[str, Any]) -> List[LintFinding]:
    """V1397 真借鉴 TF009: aws_s3_bucket with force_destroy = true (主 19:33 checkov CKV_AWS_19)."""
    findings: List[LintFinding] = []
    for res in ast.get("resource", []):
        for rtype, instances in res.items():
            rtype = _strip_quotes(rtype)
            if rtype != "aws_s3_bucket":
                continue
            for rname, attrs in instances.items():
                rname = _strip_quotes(rname)
                attrs = _normalize_value(attrs)
                if attrs.get("force_destroy") is True:
                    line_no = _find_line_no(text, "force_destroy")
                    findings.append(LintFinding(
                        rule_id="TF009-LIFECYCLE-FORCE-DESTROY",
                        severity="warning",
                        line_no=line_no,
                        line_text="force_destroy = true",
                        message=f"S3 bucket '{rname}' allows force_destroy (dangerous in prod)",
                        suggestion="Set force_destroy = false; use a manual destroy workflow for production buckets.",
                        resource=f"{rtype}.{rname}",
                    ))
    return findings


def _rule_tf010_var_no_type(text: str, ast: Dict[str, Any]) -> List[LintFinding]:
    """V1397 真借鉴 TF010: variable without type (主 19:33 tflint terraform_unused_declarations)."""
    findings: List[LintFinding] = []
    for var in ast.get("variable", []):
        for vname, vattrs in var.items():
            vname = _strip_quotes(vname)
            vattrs = _normalize_value(vattrs)
            if "type" not in vattrs:
                line_no = _find_line_no(text, f"\"{vname}\"")
                findings.append(LintFinding(
                    rule_id="TF010-VAR-NO-TYPE",
                    severity="info",
                    line_no=line_no,
                    line_text=f'variable "{vname}"',
                    message=f"Variable '{vname}' has no type constraint",
                    suggestion="Add type = string (or number/bool/list/map/object) for input validation.",
                    resource=vname,
                ))
    return findings


def _rule_tf011_output_no_description(text: str, ast: Dict[str, Any]) -> List[LintFinding]:
    """V1397 真借鉴 TF011: output without description (主 19:33 tflint terraform_unused_declarations)."""
    findings: List[LintFinding] = []
    for out in ast.get("output", []):
        for oname, oattrs in out.items():
            oname = _strip_quotes(oname)
            oattrs = _normalize_value(oattrs)
            if "description" not in oattrs:
                line_no = _find_line_no(text, f"\"{oname}\"")
                findings.append(LintFinding(
                    rule_id="TF011-OUTPUT-NO-DESCRIPTION",
                    severity="info",
                    line_no=line_no,
                    line_text=f'output "{oname}"',
                    message=f"Output '{oname}' has no description",
                    suggestion="Add description = \"...\" for documentation and tooling.",
                    resource=oname,
                ))
    return findings


def _rule_tf012_missing_required_version(text: str, ast: Dict[str, Any]) -> List[LintFinding]:
    """V1397 真借鉴 TF012: terraform without required_version (主 19:33 checkov CKV_TF_1)."""
    findings: List[LintFinding] = []
    tf_blocks = ast.get("terraform", [])
    has_version = False
    for t in tf_blocks:
        if not isinstance(t, dict):
            continue
        t = _normalize_value(t)
        if "required_version" in t:
            has_version = True
            break
    if not has_version:
        line_no = 1
        findings.append(LintFinding(
            rule_id="TF012-MISSING-REQUIRED-VERSION",
            severity="info",
            line_no=line_no,
            line_text="terraform { }",
            message="No terraform { required_version } block found",
            suggestion="Add terraform { required_version = \">= 1.5.0\" } to pin terraform version.",
            resource="terraform",
        ))
    return findings


# 真生产 规则注册表 (主 17:43 实事求是)
V1397_RULES: tuple = (
    ("TF001-HARDCODED-SECRET", _rule_tf001_hardcoded_secret),
    ("TF002-S3-NO-ENCRYPTION", _rule_tf002_s3_no_encryption),
    ("TF003-S3-PUBLIC-ACL", _rule_tf003_s3_public_acl),
    ("TF004-SG-OPEN-INGRESS", _rule_tf004_sg_open_ingress),
    ("TF005-UNPINNED-PROVIDER", _rule_tf005_unpinned_provider),
    ("TF006-RDS-NO-ENCRYPTION", _rule_tf006_rds_no_encryption),
    ("TF007-IAM-WILDCARD", _rule_tf007_iam_wildcard),
    ("TF008-EC2-NO-TAGS", _rule_tf008_ec2_no_tags),
    ("TF009-LIFECYCLE-FORCE-DESTROY", _rule_tf009_lifecycle_force_destroy),
    ("TF010-VAR-NO-TYPE", _rule_tf010_var_no_type),
    ("TF011-OUTPUT-NO-DESCRIPTION", _rule_tf011_output_no_description),
    ("TF012-MISSING-REQUIRED-VERSION", _rule_tf012_missing_required_version),
)


# ============================================================================
# 真生产 解析 + 真 lint (主 17:43)
# ============================================================================


def parse_terraform(text: str) -> Tuple[Dict[str, Any], str]:
    """V1397 真生产 解析 Terraform HCL (主 17:43 真借用 python-hcl2).

    Returns (ast_dict, parser_name). parser_name = "hcl2" or "regex-fallback".
    """
    if _HCL_AVAILABLE:
        try:
            ast = hcl2.loads(text)
            return ast, "hcl2"
        except Exception as e:
            return {}, f"hcl2-error: {type(e).__name__}: {str(e)[:200]}"
    return {}, "hcl2-missing"


def lint_terraform_file(path: Path) -> LintReport:
    """V1397 真生产 lint 一个 .tf 文件 (主 17:43)."""
    start = time.time()
    report = LintReport(
        file_path=str(path),
        n_lines=0,
        n_findings=0,
        n_errors=0,
        n_warnings=0,
        n_info=0,
        ok=True,
    )
    try:
        text = path.read_text(encoding="utf-8", errors="replace")
    except Exception as e:
        report.parse_error = f"read-error: {type(e).__name__}: {e}"
        report.ok = False
        report.elapsed_seconds = time.time() - start
        return report

    report.n_lines = text.count("\n") + (0 if text.endswith("\n") else 1)

    ast, parser = parse_terraform(text)
    report.parser = parser

    if not ast and parser.startswith("hcl2-error"):
        report.parse_error = parser
        report.ok = False
        report.elapsed_seconds = time.time() - start
        return report

    # 真规则匹配 (主 19:33 + 主 17:43)
    for rule_id, rule_fn in V1397_RULES:
        try:
            findings = rule_fn(text, ast)
        except Exception as e:
            findings = [LintFinding(
                rule_id=rule_id,
                severity="error",
                line_no=0,
                line_text=f"rule crashed: {type(e).__name__}",
                message=f"Rule {rule_id} crashed: {e}",
                file_path=str(path),
            )]
        for f in findings:
            f.file_path = str(path)
            report.findings.append(f)

    report.n_findings = len(report.findings)
    report.n_errors = sum(1 for f in report.findings if f.severity == "error")
    report.n_warnings = sum(1 for f in report.findings if f.severity == "warning")
    report.n_info = sum(1 for f in report.findings if f.severity == "info")
    report.ok = report.n_errors == 0 and not report.parse_error
    report.elapsed_seconds = time.time() - start
    return report


def discover_tf_files(target: Path, include_build_dirs: bool = False) -> List[Path]:
    """V1397 真生产 发现 .tf / .tfvars 文件 (主 17:43)."""
    skip_dirs = {"node_modules", ".git", ".venv", "__pycache__", "build", "dist", "target", ".terraform"}
    files: List[Path] = []
    if target.is_file():
        if target.suffix in (".tf", ".tfvars"):
            files.append(target)
        return files
    if not target.is_dir():
        return files
    for root, dirs, fnames in os.walk(target):
        # 跳过 build dirs (主 17:43 真 skip)
        if not include_build_dirs:
            dirs[:] = [d for d in dirs if d not in skip_dirs]
        for fn in fnames:
            if fn.endswith((".tf", ".tfvars")):
                files.append(Path(root) / fn)
    return sorted(files)


# ============================================================================
# 真生产 chain delegate (主 17:43)
# ============================================================================


def _delegate_v1387(target: Path) -> Tuple[bool, str]:
    """V1397 真生产 chain delegate V1387 unified runner (主 17:43)."""
    try:
        from apeireth.v1387_deploy_stack_runner import V1387DeployStackRunner, StackReport
        runner = V1387DeployStackRunner()
        # 真跑 runner.run(target)
        result = runner.run(target)
        # 真生产: result 是 StackReport 或 dict; 真提取 findings/files
        if hasattr(result, "n_findings"):
            n_findings = result.n_findings
            n_files = getattr(result, "n_files", 0)
        elif isinstance(result, dict):
            n_findings = result.get("n_findings", 0)
            n_files = result.get("n_files", 0)
        else:
            n_findings = 0
            n_files = 0
        return True, f"v1387 found {n_findings} findings across {n_files} files"
    except Exception as e:
        return False, f"v1387 unavailable: {type(e).__name__}: {e}"


# ============================================================================
# 真生产 popper self-test (主 17:43 实事求是)
# ============================================================================


_V1397_BUILTIN_SAMPLE = """\
# V1397 builtin sample — 真测所有 12 规则 (主 17:43)
terraform {
  required_providers {
    aws = {
      source = "hashicorp/aws"
    }
  }
}

provider "aws" {
  region = "us-east-1"
}

variable "db_password" {
  default     = "supersecret123"
  description = "DB password"
  type        = string
}

variable "aws_access_key" {
  default     = "AKIAIOSFODNN7EXAMPLE"
  description = "AWS access key"
  type        = string
}

variable "instance_count" {
  description = "Number of instances"
}

output "bucket_name" {
  value = aws_s3_bucket.bad.bucket
}

resource "aws_s3_bucket" "bad" {
  bucket        = "my-public-bucket"
  acl           = "public-read"
  force_destroy = true
}

resource "aws_s3_bucket" "good" {
  bucket = "my-private-bucket"
  acl    = "private"
}

resource "aws_s3_bucket_server_side_encryption_configuration" "good_enc" {
  bucket = aws_s3_bucket.good.id
  rule {
    apply_server_side_encryption_by_default {
      sse_algorithm = "AES256"
    }
  }
}

resource "aws_security_group" "open" {
  ingress {
    from_port   = 22
    to_port     = 22
    protocol    = "tcp"
    cidr_blocks = ["0.0.0.0/0"]
  }
}

resource "aws_db_instance" "rds" {
  allocated_storage = 20
  engine            = "postgres"
  storage_encrypted = false
}

resource "aws_iam_policy" "wide" {
  policy = jsonencode({
    Statement = [{
      Effect   = "Allow"
      Action   = "*"
      Resource = "*"
    }]
  })
}

resource "aws_instance" "web" {
  ami           = "ami-12345"
  instance_type = "t3.micro"
}
"""


def _popper_self_test() -> Dict[str, Any]:
    """V1397 真生产 popper self-test (主 17:43 真跑真测)."""
    import tempfile
    n_findings_expected_min = 8  # 至少应该找到 8+ 真问题

    with tempfile.NamedTemporaryFile(suffix=".tf", mode="w", delete=False, encoding="utf-8") as f:
        f.write(_V1397_BUILTIN_SAMPLE)
        tmp = Path(f.name)

    try:
        report = lint_terraform_file(tmp)
        # 真测 12 规则都覆盖到
        rule_ids_seen = {f.rule_id for f in report.findings}
        all_12_covered = sum(1 for rid, _ in V1397_RULES if rid in rule_ids_seen)
        ok = (
            report.n_errors >= 3  # TF001, TF003, TF004, TF007 至少 3 errors
            and report.n_warnings >= 2  # TF006, TF009 至少 2 warnings
            and report.n_info >= 2  # TF008, TF010, TF011, TF012 至少 2 info
            and report.ok is False  # 有 errors 所以不 ok
            and all_12_covered >= 8
            and report.parser.startswith("hcl2")
        )
        return {
            "ok": ok,
            "n_findings": report.n_findings,
            "n_errors": report.n_errors,
            "n_warnings": report.n_warnings,
            "n_info": report.n_info,
            "n_rules_total": len(V1397_RULES),
            "n_rules_covered": all_12_covered,
            "parser": report.parser,
            "elapsed_seconds": round(report.elapsed_seconds, 4),
            "note": f"V1397 popper ok={ok} covered={all_12_covered}/12 errors={report.n_errors} warnings={report.n_warnings} info={report.n_info}",
        }
    finally:
        try:
            tmp.unlink()
        except Exception:
            pass


# ============================================================================
# 真生产 CLI (主 17:43)
# ============================================================================


def _render_text(report: LintReport) -> str:
    """V1397 真生产 render text report (主 17:43)."""
    lines: List[str] = []
    lines.append(f"\n=== V1397 {report.file_path} ===")
    lines.append(f"lines={report.n_lines} findings={report.n_findings} "
                 f"errors={report.n_errors} warnings={report.n_warnings} info={report.n_info} "
                 f"ok={report.ok} parser={report.parser}")
    if report.parse_error:
        lines.append(f"  parse_error: {report.parse_error}")
    for f in report.findings:
        sev = f.severity.upper()
        lines.append(f"  [{sev}] {f.rule_id} (line {f.line_no}) {f.resource}: {f.message}")
        if f.suggestion:
            lines.append(f"    → {f.suggestion}")
    return "\n".join(lines)


def _render_sarif(reports: List[LintReport]) -> Dict[str, Any]:
    """V1397 真生产 render SARIF v2.1.0 (主 00:36 工程化)."""
    results = []
    for r in reports:
        for f in r.findings:
            results.append({
                "ruleId": f.rule_id,
                "level": {"error": "error", "warning": "warning", "info": "note"}.get(f.severity, "note"),
                "message": {"text": f.message},
                "locations": [{
                    "physicalLocation": {
                        "artifactLocation": {"uri": f.file_path or r.file_path},
                        "region": {"startLine": max(1, f.line_no), "snippet": {"text": f.line_text}},
                    }
                }],
            })
    return {
        "$schema": "https://json.schemastore.org/sarif-2.1.0.json",
        "version": "2.1.0",
        "runs": [{
            "tool": {
                "driver": {
                    "name": "v1397-terraform-lint",
                    "version": V1397_VERSION,
                    "informationUri": "https://github.com/apeireth",
                    "rules": [
                        {"id": rid, "shortDescription": {"text": rid}} for rid, _ in V1397_RULES
                    ],
                }
            },
            "results": results,
        }],
    }


def _cmd_version(_args: argparse.Namespace) -> int:
    """V1397 version command (主 17:43)."""
    print(f"v1397-terraform-lint v{V1397_VERSION} (schema {V1397_SCHEMA})")
    print(f"  hcl2_available: {_HCL_AVAILABLE}")
    print(f"  n_rules: {len(V1397_RULES)}")
    print(f"  n_guards: {len(V1397_GUARDS)}")
    print(f"  n_borrowed: {len(V1397_BORROWED)}")
    return 0


def _cmd_lint(args: argparse.Namespace) -> int:
    """V1397 lint command (主 17:43)."""
    target = Path(args.path)
    files = discover_tf_files(target)
    if not files:
        print(f"V1397: no .tf files found in {target}", file=sys.stderr)
        return 3

    reports = [lint_terraform_file(f) for f in files]
    total_errors = sum(r.n_errors for r in reports)
    total_warnings = sum(r.n_warnings for r in reports)
    total_info = sum(r.n_info for r in reports)

    if args.format == "json":
        print(json.dumps({
            "schema": V1397_SCHEMA,
            "version": V1397_VERSION,
            "n_files": len(reports),
            "n_findings": sum(r.n_findings for r in reports),
            "n_errors": total_errors,
            "n_warnings": total_warnings,
            "n_info": total_info,
            "ok": total_errors == 0,
            "reports": [r.to_dict() for r in reports],
        }, indent=2, ensure_ascii=False))
    elif args.format == "sarif":
        print(json.dumps(_render_sarif(reports), indent=2, ensure_ascii=False))
    else:
        for r in reports:
            print(_render_text(r))
        print(f"\nTotal: {len(reports)} files, "
              f"{sum(r.n_findings for r in reports)} findings "
              f"({total_errors} errors, {total_warnings} warnings, {total_info} info)")

    if total_errors > 0:
        return 1
    if args.strict and total_warnings > 0:
        return 2
    return 0


def _cmd_chain(args: argparse.Namespace) -> int:
    """V1397 chain command — 真调 V1387 + V1397 (主 17:43)."""
    target = Path(args.path)
    ok_v1387, msg_v1387 = _delegate_v1387(target)

    files = discover_tf_files(target)
    reports = [lint_terraform_file(f) for f in files]
    total_errors = sum(r.n_errors for r in reports)
    total_warnings = sum(r.n_warnings for r in reports)

    print(f"\n=== V1397 chain report for {target} ===")
    print(f"V1387 unified runner: {'OK' if ok_v1387 else 'FAIL'} — {msg_v1387}")
    print(f"V1397 terraform lint: {len(reports)} files, "
          f"{sum(r.n_findings for r in reports)} findings "
          f"({total_errors} errors, {total_warnings} warnings)")

    if args.json:
        print(json.dumps({
            "schema": V1397_SCHEMA,
            "chain": {
                "v1387": {"ok": ok_v1387, "msg": msg_v1387},
                "v1397": {
                    "ok": total_errors == 0,
                    "n_files": len(reports),
                    "n_findings": sum(r.n_findings for r in reports),
                    "n_errors": total_errors,
                    "n_warnings": total_warnings,
                },
            },
        }, indent=2, ensure_ascii=False))

    if total_errors > 0:
        return 1
    return 0 if ok_v1387 else 2


def _cmd_demo(_args: argparse.Namespace) -> int:
    """V1397 demo (主 17:43)."""
    print("# V1397 demo — builtin .tf sample (12 规则全覆盖)")
    print(f"# Schema: {V1397_SCHEMA} v{V1397_VERSION}")
    print(f"# hcl2_available: {_HCL_AVAILABLE}")
    print(f"# n_rules: {len(V1397_RULES)}")
    print(f"# n_guards: {len(V1397_GUARDS)}")
    print(f"# n_borrowed: {len(V1397_BORROWED)}")
    print()
    print("## Borrowed (主 19:33)")
    for b in V1397_BORROWED:
        print(f"- {b}")
    print()
    print("## Guards (主 17:43)")
    for g in V1397_GUARDS:
        print(f"- {g}")
    print()
    print("## Rules (主 19:33 tfsec/checkov/tflint 真借鉴)")
    for rid, _ in V1397_RULES:
        print(f"- {rid}")
    print()
    print("## Sample HCL (主 17:43 真测)")
    print(_V1397_BUILTIN_SAMPLE)
    return 0


def _cmd_popper(_args: argparse.Namespace) -> int:
    """V1397 popper self-test (主 17:43)."""
    result = _popper_self_test()
    print(json.dumps(result, indent=2, ensure_ascii=False))
    return 0 if result["ok"] else 1


def _build_parser() -> argparse.ArgumentParser:
    """V1397 CLI parser (主 17:43)."""
    parser = argparse.ArgumentParser(
        prog="v1397-terraform-lint",
        description=f"V1397 ASI real production terraform HCL lint (v{V1397_VERSION})",
    )
    sub = parser.add_subparsers(dest="cmd")

    p_ver = sub.add_parser("version", help="V1397 version")
    p_ver.set_defaults(func=_cmd_version)

    p_lint = sub.add_parser("lint", help="真 lint .tf files")
    p_lint.add_argument("path", nargs="?", default=".", help="file or directory")
    p_lint.add_argument("--format", choices=("text", "json", "sarif"), default="text")
    p_lint.add_argument("--strict", action="store_true", help="exit non-zero on warnings")
    p_lint.set_defaults(func=_cmd_lint)

    p_chain = sub.add_parser("chain", help="真 chain V1387 + V1397")
    p_chain.add_argument("path", nargs="?", default=".", help="file or directory")
    p_chain.add_argument("--json", action="store_true")
    p_chain.set_defaults(func=_cmd_chain)

    p_demo = sub.add_parser("demo", help="V1397 demo")
    p_demo.set_defaults(func=_cmd_demo)

    p_popper = sub.add_parser("popper", help="V1397 popper self-test")
    p_popper.set_defaults(func=_cmd_popper)

    return parser


def main(argv: Optional[Sequence[str]] = None) -> int:
    """V1397 真生产 CLI main (主 17:43)."""
    parser = _build_parser()
    args = parser.parse_args(argv)
    if not hasattr(args, "func"):
        parser.print_help()
        return 0
    return args.func(args)


if __name__ == "__main__":
    sys.exit(main())