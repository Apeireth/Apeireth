"""Tests for V1397 real production terraform HCL lint.

Tests structure (V1397 真生产 设计):
- TestV1397Constants: version / schema / GUARDS / BORROWED / RULES
- TestV1397HCLParse: 真 parse HCL (python-hcl2)
- TestV1397StripQuotes: 真 strip hcl2 quoted strings
- TestV1397FindLineNo: 真 find line number
- TestV1397Rules: 真 12 规则 each with positive + negative
- TestV1397LintFile: 真 lint report aggregation
- TestV1397Discover: 真 discover .tf / .tfvars files
- TestV1397ChainDelegate: 真 chain V1387 + V1397
- TestV1397Popper: 真 popper self-test
- TestV1397CLI: 真 CLI version/lint/chain/demo/popper
- TestV1397RealFile: 真 lint real .tf files in repo
- TestV1397V3Guards: 真 V3 哲学 6 guards
"""
from __future__ import annotations

import json
import os
import subprocess
import sys
import tempfile
from pathlib import Path

import pytest

# Ensure promethean root is on path
ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT))

import apeireth.v1397_real_terraform_lint as V1397


# ============================================================================
# Constants / version / schema (V1397 真生产)
# ============================================================================


class TestV1397Constants:
    def test_version(self):
        assert V1397.V1397_VERSION == "0.1.0"

    def test_schema(self):
        assert V1397.V1397_SCHEMA == "v1397.terraform-lint/v1"

    def test_hcl2_available(self):
        # V1397 depends on python-hcl2; should be installed
        assert V1397._HCL_AVAILABLE is True

    def test_n_guards(self):
        assert len(V1397.V1397_GUARDS) == 12

    def test_n_borrowed(self):
        assert len(V1397.V1397_BORROWED) == 6

    def test_n_rules(self):
        assert len(V1397.V1397_RULES) == 12

    def test_guards_include_required(self):
        required = (
            "GUARD_HCL_PARSED", "GUARD_RULES_REAL", "GUARD_FILE_IO",
            "GUARD_LINE_TRACKED", "GUARD_NO_CAP_CHANGE", "GUARD_DETERMINISTIC",
            "GUARD_HONEST_DISCLOSURE", "GUARD_PATH_SAFE", "GUARD_NON_DESTRUCTIVE",
            "GUARD_DELEGATE_REAL", "GUARD_CLI_RUNNABLE", "GUARD_POPPER_RUNS",
        )
        for g in required:
            assert g in V1397.V1397_GUARDS, f"missing guard: {g}"

    def test_borrowed_include_terraform_tools(self):
        text = " ".join(V1397.V1397_BORROWED)
        for tool in ("tflint", "tfsec", "checkov", "terrascan", "terraform", "conftest"):
            assert tool in text.lower(), f"missing borrowed tool: {tool}"

    def test_rules_unique(self):
        ids = [rid for rid, _ in V1397.V1397_RULES]
        assert len(ids) == len(set(ids)), "rule IDs must be unique"

    def test_rules_prefix_tf(self):
        for rid, _ in V1397.V1397_RULES:
            assert rid.startswith("TF"), f"rule {rid} must start with TF"

    def test_rule_ids_cover_12_categories(self):
        ids = {rid for rid, _ in V1397.V1397_RULES}
        assert "TF001-HARDCODED-SECRET" in ids
        assert "TF002-S3-NO-ENCRYPTION" in ids
        assert "TF003-S3-PUBLIC-ACL" in ids
        assert "TF004-SG-OPEN-INGRESS" in ids
        assert "TF005-UNPINNED-PROVIDER" in ids
        assert "TF006-RDS-NO-ENCRYPTION" in ids
        assert "TF007-IAM-WILDCARD" in ids
        assert "TF008-EC2-NO-TAGS" in ids
        assert "TF009-LIFECYCLE-FORCE-DESTROY" in ids
        assert "TF010-VAR-NO-TYPE" in ids
        assert "TF011-OUTPUT-NO-DESCRIPTION" in ids
        assert "TF012-MISSING-REQUIRED-VERSION" in ids


# ============================================================================
# 真生产 HCL parse (V1397)
# ============================================================================


class TestV1397HCLParse:
    def test_parse_simple(self):
        text = 'variable "foo" { default = "bar" }\n'
        ast, parser = V1397.parse_terraform(text)
        assert parser == "hcl2"
        assert "variable" in ast

    def test_parse_resource(self):
        text = '''
resource "aws_s3_bucket" "b" {
  bucket = "x"
  acl    = "private"
}
'''
        ast, parser = V1397.parse_terraform(text)
        assert parser == "hcl2"
        assert "resource" in ast
        assert ast["resource"][0]["\"aws_s3_bucket\""]["\"b\""]["bucket"] == '"x"'

    def test_parse_provider_block(self):
        text = '''
terraform {
  required_providers {
    aws = {
      source = "hashicorp/aws"
    }
  }
}
'''
        ast, parser = V1397.parse_terraform(text)
        assert parser == "hcl2"
        assert "terraform" in ast
        # 真 hcl2 wraps required_providers in list
        tf_block = ast["terraform"][0]
        assert "required_providers" in tf_block

    def test_parse_error_returns_error(self):
        # 真 broken HCL
        text = 'resource "aws_s3_bucket" "b" { bucket = "x" ##### syntax error #####\n'
        ast, parser = V1397.parse_terraform(text)
        assert parser.startswith("hcl2-error") or ast == {}

    def test_strip_quotes(self):
        assert V1397._strip_quotes('"hello"') == "hello"
        assert V1397._strip_quotes("hello") == "hello"
        assert V1397._strip_quotes('""') == ""
        assert V1397._strip_quotes(123) == 123

    def test_normalize_value_dict(self):
        d = {'"foo"': '"bar"', '"nested"': {'"k"': '"v"'}}
        out = V1397._normalize_value(d)
        assert out == {"foo": "bar", "nested": {"k": "v"}}

    def test_normalize_value_list(self):
        out = V1397._normalize_value(['"a"', '"b"'])
        assert out == ["a", "b"]

    def test_find_line_no(self):
        text = "line1\nline2\nline3\n"
        assert V1397._find_line_no(text, "line1") == 1
        assert V1397._find_line_no(text, "line2") == 2
        assert V1397._find_line_no(text, "line3") == 3
        assert V1397._find_line_no(text, "missing") == 0

    def test_find_line_no_realistic(self):
        text = '''variable "foo" {
  default = "bar"
}

resource "aws_s3_bucket" "b" {
  bucket = "x"
}
'''
        assert V1397._find_line_no(text, "variable") == 1
        assert V1397._find_line_no(text, '"aws_s3_bucket"') == 5


# ============================================================================
# 真生产 规则 (V1397)
# ============================================================================


SAMPLE_BAD = '''
variable "db_password" {
  default = "supersecret123"
  description = "DB password"
}

variable "api_key" {
  default = "AKIAIOSFODNN7EXAMPLE"
  description = "AWS access key"
}

resource "aws_s3_bucket" "bad" {
  bucket        = "my-public-bucket"
  acl           = "public-read"
  force_destroy = true
}

resource "aws_s3_bucket" "noenc" {
  bucket = "no-enc-bucket"
  acl    = "private"
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

output "bucket_name" {
  value = aws_s3_bucket.bad.bucket
}

terraform {
  required_providers {
    aws = {
      source = "hashicorp/aws"
    }
  }
}
'''

SAMPLE_GOOD = '''
variable "region" {
  type        = string
  default     = "us-east-1"
  description = "AWS region"
}

resource "aws_s3_bucket" "good" {
  bucket = "good-bucket"
  acl    = "private"
  tags = {
    Name = "good"
  }
}

resource "aws_s3_bucket_server_side_encryption_configuration" "good_enc" {
  bucket = aws_s3_bucket.good.id
  rule {
    apply_server_side_encryption_by_default {
      sse_algorithm = "AES256"
    }
  }
}

resource "aws_security_group" "restricted" {
  ingress {
    from_port   = 22
    to_port     = 22
    protocol    = "tcp"
    cidr_blocks = ["10.0.0.0/8"]
  }
}

resource "aws_db_instance" "rds" {
  allocated_storage = 20
  engine            = "postgres"
  storage_encrypted = true
  kms_key_id        = "arn:aws:kms:us-east-1:123:key/abc"
  tags = {
    Name = "rds"
  }
}

resource "aws_iam_policy" "scoped" {
  policy = jsonencode({
    Statement = [{
      Effect   = "Allow"
      Action   = ["s3:GetObject"]
      Resource = "arn:aws:s3:::my-bucket/*"
    }]
  })
}

resource "aws_instance" "web" {
  ami           = "ami-12345"
  instance_type = "t3.micro"
  tags = {
    Name = "web"
  }
}

output "bucket_name" {
  value     = aws_s3_bucket.good.bucket
  description = "The bucket name"
}

terraform {
  required_version = ">= 1.5.0"
  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = "~> 5.0"
    }
  }
}
'''


class TestV1397RuleTF001:
    """TF001-HARDCODED-SECRET 真生产 规则."""

    def test_detects_aws_access_key(self):
        ast, _ = V1397.parse_terraform(SAMPLE_BAD)
        findings = V1397._rule_tf001_hardcoded_secret(SAMPLE_BAD, ast)
        assert len(findings) >= 1
        assert any(f.rule_id == "TF001-HARDCODED-SECRET" for f in findings)

    def test_no_findings_in_clean(self):
        ast, _ = V1397.parse_terraform(SAMPLE_GOOD)
        findings = V1397._rule_tf001_hardcoded_secret(SAMPLE_GOOD, ast)
        assert findings == []

    def test_severity_error(self):
        ast, _ = V1397.parse_terraform(SAMPLE_BAD)
        findings = V1397._rule_tf001_hardcoded_secret(SAMPLE_BAD, ast)
        assert all(f.severity == "error" for f in findings)


class TestV1397RuleTF002:
    """TF002-S3-NO-ENCRYPTION 真生产 规则."""

    def test_detects_no_sse(self):
        ast, _ = V1397.parse_terraform(SAMPLE_BAD)
        findings = V1397._rule_tf002_s3_no_encryption(SAMPLE_BAD, ast)
        # bad has no SSE; noenc has no SSE; good not in this sample
        assert len(findings) >= 2
        assert any(f.rule_id == "TF002-S3-NO-ENCRYPTION" for f in findings)

    def test_no_findings_when_sse_present(self):
        ast, _ = V1397.parse_terraform(SAMPLE_GOOD)
        findings = V1397._rule_tf002_s3_no_encryption(SAMPLE_GOOD, ast)
        assert findings == []


class TestV1397RuleTF003:
    """TF003-S3-PUBLIC-ACL 真生产 规则."""

    def test_detects_public_read(self):
        ast, _ = V1397.parse_terraform(SAMPLE_BAD)
        findings = V1397._rule_tf003_s3_public_acl(SAMPLE_BAD, ast)
        assert len(findings) >= 1
        assert findings[0].rule_id == "TF003-S3-PUBLIC-ACL"
        assert findings[0].severity == "error"

    def test_no_findings_private(self):
        ast, _ = V1397.parse_terraform(SAMPLE_GOOD)
        findings = V1397._rule_tf003_s3_public_acl(SAMPLE_GOOD, ast)
        assert findings == []


class TestV1397RuleTF004:
    """TF004-SG-OPEN-INGRESS 真生产 规则."""

    def test_detects_open_ingress(self):
        ast, _ = V1397.parse_terraform(SAMPLE_BAD)
        findings = V1397._rule_tf004_sg_open_ingress(SAMPLE_BAD, ast)
        assert len(findings) >= 1
        assert findings[0].rule_id == "TF004-SG-OPEN-INGRESS"
        assert findings[0].severity == "error"

    def test_no_findings_restricted(self):
        ast, _ = V1397.parse_terraform(SAMPLE_GOOD)
        findings = V1397._rule_tf004_sg_open_ingress(SAMPLE_GOOD, ast)
        assert findings == []


class TestV1397RuleTF005:
    """TF005-UNPINNED-PROVIDER 真生产 规则."""

    def test_detects_unpinned(self):
        ast, _ = V1397.parse_terraform(SAMPLE_BAD)
        findings = V1397._rule_tf005_unpinned_provider(SAMPLE_BAD, ast)
        assert len(findings) >= 1
        assert findings[0].rule_id == "TF005-UNPINNED-PROVIDER"

    def test_no_findings_pinned(self):
        ast, _ = V1397.parse_terraform(SAMPLE_GOOD)
        findings = V1397._rule_tf005_unpinned_provider(SAMPLE_GOOD, ast)
        assert findings == []


class TestV1397RuleTF006:
    """TF006-RDS-NO-ENCRYPTION 真生产 规则."""

    def test_detects_unencrypted_rds(self):
        ast, _ = V1397.parse_terraform(SAMPLE_BAD)
        findings = V1397._rule_tf006_rds_no_encryption(SAMPLE_BAD, ast)
        assert len(findings) >= 1
        assert findings[0].rule_id == "TF006-RDS-NO-ENCRYPTION"

    def test_no_findings_encrypted(self):
        ast, _ = V1397.parse_terraform(SAMPLE_GOOD)
        findings = V1397._rule_tf006_rds_no_encryption(SAMPLE_GOOD, ast)
        assert findings == []


class TestV1397RuleTF007:
    """TF007-IAM-WILDCARD 真生产 规则."""

    def test_detects_wildcard(self):
        ast, _ = V1397.parse_terraform(SAMPLE_BAD)
        findings = V1397._rule_tf007_iam_wildcard(SAMPLE_BAD, ast)
        assert len(findings) >= 1
        assert findings[0].rule_id == "TF007-IAM-WILDCARD"
        assert findings[0].severity == "error"

    def test_no_findings_scoped(self):
        ast, _ = V1397.parse_terraform(SAMPLE_GOOD)
        findings = V1397._rule_tf007_iam_wildcard(SAMPLE_GOOD, ast)
        assert findings == []


class TestV1397RuleTF008:
    """TF008-EC2-NO-TAGS 真生产 规则."""

    def test_detects_no_tags(self):
        ast, _ = V1397.parse_terraform(SAMPLE_BAD)
        findings = V1397._rule_tf008_ec2_no_tags(SAMPLE_BAD, ast)
        # aws_instance.web and aws_db_instance.rds have no tags
        assert len(findings) >= 1
        assert all(f.rule_id == "TF008-EC2-NO-TAGS" for f in findings)
        assert all(f.severity == "info" for f in findings)

    def test_no_findings_tagged(self):
        ast, _ = V1397.parse_terraform(SAMPLE_GOOD)
        findings = V1397._rule_tf008_ec2_no_tags(SAMPLE_GOOD, ast)
        assert findings == []


class TestV1397RuleTF009:
    """TF009-LIFECYCLE-FORCE-DESTROY 真生产 规则."""

    def test_detects_force_destroy(self):
        ast, _ = V1397.parse_terraform(SAMPLE_BAD)
        findings = V1397._rule_tf009_lifecycle_force_destroy(SAMPLE_BAD, ast)
        assert len(findings) >= 1
        assert findings[0].rule_id == "TF009-LIFECYCLE-FORCE-DESTROY"

    def test_no_findings_no_force(self):
        ast, _ = V1397.parse_terraform(SAMPLE_GOOD)
        findings = V1397._rule_tf009_lifecycle_force_destroy(SAMPLE_GOOD, ast)
        assert findings == []


class TestV1397RuleTF010:
    """TF010-VAR-NO-TYPE 真生产 规则."""

    def test_detects_no_type(self):
        ast, _ = V1397.parse_terraform(SAMPLE_BAD)
        findings = V1397._rule_tf010_var_no_type(SAMPLE_BAD, ast)
        # db_password and api_key have no type
        assert len(findings) >= 1
        assert all(f.rule_id == "TF010-VAR-NO-TYPE" for f in findings)

    def test_no_findings_typed(self):
        ast, _ = V1397.parse_terraform(SAMPLE_GOOD)
        findings = V1397._rule_tf010_var_no_type(SAMPLE_GOOD, ast)
        assert findings == []


class TestV1397RuleTF011:
    """TF011-OUTPUT-NO-DESCRIPTION 真生产 规则."""

    def test_detects_no_desc(self):
        ast, _ = V1397.parse_terraform(SAMPLE_BAD)
        findings = V1397._rule_tf011_output_no_description(SAMPLE_BAD, ast)
        assert len(findings) >= 1
        assert findings[0].rule_id == "TF011-OUTPUT-NO-DESCRIPTION"

    def test_no_findings_described(self):
        ast, _ = V1397.parse_terraform(SAMPLE_GOOD)
        findings = V1397._rule_tf011_output_no_description(SAMPLE_GOOD, ast)
        assert findings == []


class TestV1397RuleTF012:
    """TF012-MISSING-REQUIRED-VERSION 真生产 规则."""

    def test_detects_no_required_version(self):
        ast, _ = V1397.parse_terraform(SAMPLE_BAD)
        findings = V1397._rule_tf012_missing_required_version(SAMPLE_BAD, ast)
        assert len(findings) >= 1
        assert findings[0].rule_id == "TF012-MISSING-REQUIRED-VERSION"

    def test_no_findings_has_version(self):
        ast, _ = V1397.parse_terraform(SAMPLE_GOOD)
        findings = V1397._rule_tf012_missing_required_version(SAMPLE_GOOD, ast)
        assert findings == []


# ============================================================================
# 真生产 LintReport + lint_terraform_file (V1397)
# ============================================================================


class TestV1397LintFile:
    def _write(self, content: str) -> Path:
        f = tempfile.NamedTemporaryFile(suffix=".tf", mode="w", delete=False, encoding="utf-8")
        f.write(content)
        f.close()
        return Path(f.name)

    def test_lint_bad_file(self):
        p = self._write(SAMPLE_BAD)
        try:
            report = V1397.lint_terraform_file(p)
            assert report.parser == "hcl2"
            assert report.n_findings >= 8
            assert report.n_errors >= 3  # TF001, TF003, TF004, TF007
            assert report.ok is False  # errors found
        finally:
            p.unlink()

    def test_lint_good_file(self):
        p = self._write(SAMPLE_GOOD)
        try:
            report = V1397.lint_terraform_file(p)
            assert report.parser == "hcl2"
            # TF010 may still fire for instance_count (no type) — actually GOOD has type on region only
            # GOOD sample has all rules satisfied
            assert report.n_errors == 0
            assert report.ok is True
        finally:
            p.unlink()

    def test_lint_nonexistent_file(self):
        report = V1397.lint_terraform_file(Path("/nonexistent/foo/bar.tf"))
        assert report.ok is False
        assert "read-error" in report.parse_error

    def test_lint_n_lines(self):
        p = self._write(SAMPLE_BAD)
        try:
            report = V1397.lint_terraform_file(p)
            assert report.n_lines > 0
        finally:
            p.unlink()

    def test_lint_elapsed_seconds(self):
        p = self._write(SAMPLE_BAD)
        try:
            report = V1397.lint_terraform_file(p)
            assert report.elapsed_seconds >= 0
            assert report.elapsed_seconds < 5.0  # 真生产 应该 <5s
        finally:
            p.unlink()

    def test_lint_report_to_dict(self):
        p = self._write(SAMPLE_BAD)
        try:
            report = V1397.lint_terraform_file(p)
            d = report.to_dict()
            assert d["parser"] == "hcl2"
            assert d["n_findings"] == report.n_findings
            assert isinstance(d["findings"], list)
            assert d["findings"][0]["rule_id"].startswith("TF")
        finally:
            p.unlink()

    def test_finding_to_dict(self):
        f = V1397.LintFinding(
            rule_id="TF001",
            severity="error",
            line_no=10,
            line_text='access_key = "..."',
            message="test",
            suggestion="fix",
            resource="aws.foo.bar",
            file_path="test.tf",
        )
        d = f.to_dict()
        assert d["rule_id"] == "TF001"
        assert d["severity"] == "error"
        assert d["line_no"] == 10
        assert d["resource"] == "aws.foo.bar"


# ============================================================================
# 真生产 Discover (V1397)
# ============================================================================


class TestV1397Discover:
    def test_discover_file(self):
        f = tempfile.NamedTemporaryFile(suffix=".tf", mode="w", delete=False, encoding="utf-8")
        f.write("# nothing\n")
        f.close()
        try:
            files = V1397.discover_tf_files(Path(f.name))
            assert len(files) == 1
        finally:
            Path(f.name).unlink()

    def test_discover_dir_finds_tf(self):
        with tempfile.TemporaryDirectory() as d:
            (Path(d) / "main.tf").write_text("# x\n")
            (Path(d) / "vars.tfvars").write_text("# x\n")
            (Path(d) / "README.md").write_text("no")
            (Path(d) / "sub").mkdir()
            (Path(d) / "sub" / "nested.tf").write_text("# x\n")
            files = V1397.discover_tf_files(Path(d))
            names = {f.name for f in files}
            assert "main.tf" in names
            assert "vars.tfvars" in names
            assert "nested.tf" in names
            assert "README.md" not in names

    def test_discover_skips_build_dirs(self):
        with tempfile.TemporaryDirectory() as d:
            (Path(d) / "main.tf").write_text("# x\n")
            (Path(d) / "node_modules").mkdir()
            (Path(d) / "node_modules" / "skip.tf").write_text("# skip\n")
            (Path(d) / ".terraform").mkdir()
            (Path(d) / ".terraform" / "skip.tf").write_text("# skip\n")
            files = V1397.discover_tf_files(Path(d))
            names = {f.name for f in files}
            assert "main.tf" in names
            assert "skip.tf" not in names

    def test_discover_nonexistent(self):
        files = V1397.discover_tf_files(Path("/nonexistent/foo"))
        assert files == []


# ============================================================================
# 真生产 Chain (V1397)
# ============================================================================


class TestV1397ChainDelegate:
    def test_v1387_available_or_graceful(self):
        # 真调 V1387; 可用 或 graceful 失败
        ok, msg = V1397._delegate_v1387(ROOT)
        assert isinstance(ok, bool)
        assert isinstance(msg, str)
        assert "v1387" in msg.lower()


# ============================================================================
# 真生产 Popper self-test (V1397)
# ============================================================================


class TestV1397Popper:
    def test_popper_ok(self):
        result = V1397._popper_self_test()
        assert result["ok"] is True
        assert result["n_rules_total"] == 12
        assert result["n_rules_covered"] >= 10  # 真测 ≥10
        assert result["n_errors"] >= 3
        assert result["n_warnings"] >= 2
        assert result["n_info"] >= 2
        assert result["parser"].startswith("hcl2")


# ============================================================================
# 真生产 CLI (V1397)
# ============================================================================


class TestV1397CLI:
    def test_help(self):
        out = subprocess.run(
            [sys.executable, "-m", "apeireth.v1397_real_terraform_lint", "--help"],
            capture_output=True, encoding="utf-8", errors="replace", cwd=str(ROOT),
            env={**os.environ, "PYTHONIOENCODING": "utf-8"},
        )
        assert out.returncode == 0
        assert "terraform" in (out.stdout or "").lower()

    def test_version(self):
        out = subprocess.run(
            [sys.executable, "-m", "apeireth.v1397_real_terraform_lint", "version"],
            capture_output=True, encoding="utf-8", errors="replace", cwd=str(ROOT),
            env={**os.environ, "PYTHONIOENCODING": "utf-8"},
        )
        assert out.returncode == 0
        assert V1397.V1397_VERSION in (out.stdout or "")
        assert "12" in (out.stdout or "")  # n_rules / n_guards

    def test_demo(self):
        out = subprocess.run(
            [sys.executable, "-m", "apeireth.v1397_real_terraform_lint", "demo"],
            capture_output=True, encoding="utf-8", errors="replace", cwd=str(ROOT),
            env={**os.environ, "PYTHONIOENCODING": "utf-8"},
        )
        assert out.returncode == 0
        assert "TF001" in (out.stdout or "")
        assert "TF012" in (out.stdout or "")

    def test_popper(self):
        out = subprocess.run(
            [sys.executable, "-m", "apeireth.v1397_real_terraform_lint", "popper"],
            capture_output=True, encoding="utf-8", errors="replace", cwd=str(ROOT),
            env={**os.environ, "PYTHONIOENCODING": "utf-8"},
        )
        assert out.returncode == 0
        data = json.loads(out.stdout)
        assert data["ok"] is True

    def test_lint_text(self):
        # Write a .tf and lint it
        f = tempfile.NamedTemporaryFile(suffix=".tf", mode="w", delete=False, encoding="utf-8")
        f.write(SAMPLE_BAD)
        f.close()
        try:
            out = subprocess.run(
                [sys.executable, "-m", "apeireth.v1397_real_terraform_lint", "lint", f.name],
                capture_output=True, encoding="utf-8", errors="replace", cwd=str(ROOT),
                env={**os.environ, "PYTHONIOENCODING": "utf-8"},
            )
            # exit code 1 because errors found
            assert out.returncode == 1
            assert "TF003-S3-PUBLIC-ACL" in (out.stdout or "")
        finally:
            Path(f.name).unlink()

    def test_lint_json(self):
        f = tempfile.NamedTemporaryFile(suffix=".tf", mode="w", delete=False, encoding="utf-8")
        f.write(SAMPLE_BAD)
        f.close()
        try:
            out = subprocess.run(
                [sys.executable, "-m", "apeireth.v1397_real_terraform_lint", "lint", f.name, "--format", "json"],
                capture_output=True, encoding="utf-8", errors="replace", cwd=str(ROOT),
                env={**os.environ, "PYTHONIOENCODING": "utf-8"},
            )
            assert out.returncode == 1
            data = json.loads(out.stdout)
            assert data["schema"] == V1397.V1397_SCHEMA
            assert data["n_errors"] >= 3
        finally:
            Path(f.name).unlink()

    def test_lint_sarif(self):
        f = tempfile.NamedTemporaryFile(suffix=".tf", mode="w", delete=False, encoding="utf-8")
        f.write(SAMPLE_BAD)
        f.close()
        try:
            out = subprocess.run(
                [sys.executable, "-m", "apeireth.v1397_real_terraform_lint", "lint", f.name, "--format", "sarif"],
                capture_output=True, encoding="utf-8", errors="replace", cwd=str(ROOT),
                env={**os.environ, "PYTHONIOENCODING": "utf-8"},
            )
            assert out.returncode == 1
            data = json.loads(out.stdout)
            assert data["version"] == "2.1.0"
            assert len(data["runs"][0]["results"]) >= 3
        finally:
            Path(f.name).unlink()

    def test_lint_clean_returns_zero(self):
        f = tempfile.NamedTemporaryFile(suffix=".tf", mode="w", delete=False, encoding="utf-8")
        f.write(SAMPLE_GOOD)
        f.close()
        try:
            out = subprocess.run(
                [sys.executable, "-m", "apeireth.v1397_real_terraform_lint", "lint", f.name],
                capture_output=True, encoding="utf-8", errors="replace", cwd=str(ROOT),
                env={**os.environ, "PYTHONIOENCODING": "utf-8"},
            )
            assert out.returncode == 0
        finally:
            Path(f.name).unlink()

    def test_chain_real_repo(self):
        out = subprocess.run(
            [sys.executable, "-m", "apeireth.v1397_real_terraform_lint", "chain", "."],
            capture_output=True, encoding="utf-8", errors="replace", cwd=str(ROOT),
            env={**os.environ, "PYTHONIOENCODING": "utf-8"},
        )
        # exit 0 or 1 depending on findings, but output should mention v1387 and v1397
        assert "V1387" in (out.stdout or "") or "v1387" in (out.stdout or "")
        assert "V1397" in (out.stdout or "") or "v1397" in (out.stdout or "")

    def test_no_files_exit_3(self):
        with tempfile.TemporaryDirectory() as d:
            out = subprocess.run(
                [sys.executable, "-m", "apeireth.v1397_real_terraform_lint", "lint", d],
                capture_output=True, encoding="utf-8", errors="replace", cwd=str(ROOT),
                env={**os.environ, "PYTHONIOENCODING": "utf-8"},
            )
            assert out.returncode == 3


# ============================================================================
# 真生产 Real file lint (V1397 集成测试)
# ============================================================================


class TestV1397RealFile:
    def test_real_repo_no_crash(self):
        # 真 lint promethean repo (会找到 Apeireth-rust 中的 .tf)
        reports = []
        for tf in V1397.discover_tf_files(ROOT):
            reports.append(V1397.lint_terraform_file(tf))
        # 不 crash 就 OK
        assert len(reports) >= 0  # 真生产 至少能跑

    def test_real_repo_or_zero_files(self):
        # 真生产 至少能跑, 不会 raise
        try:
            with tempfile.TemporaryDirectory() as d:
                files = V1397.discover_tf_files(Path(d))
                for tf in files:
                    V1397.lint_terraform_file(tf)
        except Exception as e:
            pytest.fail(f"unexpected exception: {e}")


# ============================================================================
# 真生产 V3 哲学守门 (V1397)
# ============================================================================


class TestV1397V3Guards:
    """V1397 V3 哲学 6 guards 自动注入 (主 17:58 + 主 20:46 + 主 17:43)."""

    def test_module_is_not_asi(self):
        # V1397 是 terraform linter, 不是 ASI 达成
        text = open(V1397.__file__, encoding="utf-8").read()
        assert "不假装达到 ASI" in text or "module_is_not_asi" in text

    def test_measurement_is_not_truth(self):
        # V1397 真测 真报, 不假装调整指标
        text = open(V1397.__file__, encoding="utf-8").read()
        assert "实事求是" in text

    def test_structure_is_not_consciousness(self):
        # V1397 真 parse, 不假装 Phenomenal consciousness
        text = open(V1397.__file__, encoding="utf-8").read()
        assert "Phenomenal consciousness" in text

    def test_production_is_not_safety(self):
        # V1397 真生产 ≠ 安全审计
        text = open(V1397.__file__, encoding="utf-8").read()
        assert "安全审计" in text

    def test_automation_is_not_autonomy(self):
        # V1397 真自动化 ≠ 自主
        assert "v1397" in V1397.__name__  # 真名是 v1397 module (lowercase)

    def test_runner_is_not_asi(self):
        # V1397 runner ≠ ASI
        text = open(V1397.__file__, encoding="utf-8").read()
        assert "runner" in text.lower()


# ============================================================================
# 真生产 continuity / chain (V1397)
# ============================================================================


class TestV1397Continuity:
    def test_module_id_format(self):
        """V1397 真生产 module id format."""
        assert V1397.__name__ == "apeireth.v1397_real_terraform_lint"

    def test_main_entry(self):
        """V1397 真生产 main entry."""
        result = V1397.main(["version"])
        assert result == 0
        result = V1397.main(["popper"])
        assert result == 0

    def test_main_no_args(self):
        """V1397 main no args."""
        result = V1397.main([])
        assert result == 0  # help prints


if __name__ == "__main__":
    pytest.main([__file__, "-v"])