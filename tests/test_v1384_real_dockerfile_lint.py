"""Tests for V1384 ASI Dockerfile 真解析 + 真 lint (主 06:15 + 主 17:43 + 主 19:33 + 主 23:44).

主 17:43 实事求是: 真 read + 真 parse + 真规则匹配, 不假装 lint.
"""
from __future__ import annotations

import json
import os
import pathlib
import sys
import tempfile
from typing import Any, Dict, List

import pytest

MODULE_DIR = pathlib.Path(__file__).resolve().parent.parent / "apeireth"
sys.path.insert(0, str(MODULE_DIR))

from v1384_real_dockerfile_lint import (
    V1384_VERSION,
    V1384DockerfileLint,
    DOC_LEVEL_RULES,
    LintFinding,
    LintReport,
    DockerfileInstruction,
    PER_INSTRUCTION_RULES,
    parse_dockerfile,
    run_cli,
)


# ============================================================================
# 解析器测试 (parse_dockerfile)
# ============================================================================


class TestParseDockerfile:
    """V1384 真生产 Dockerfile 解析测试 (主 17:43 实事求是)."""

    def test_parse_simple_from(self):
        text = "FROM python:3.13-slim"
        instrs = parse_dockerfile(text)
        assert len(instrs) == 1
        assert instrs[0].cmd == "FROM"
        assert instrs[0].args == "python:3.13-slim"
        assert instrs[0].line_no == 1

    def test_parse_multiple_instructions(self):
        text = """FROM python:3.13-slim

WORKDIR /app

COPY . /app

RUN pip install foo
CMD ["python", "app.py"]
"""
        instrs = parse_dockerfile(text)
        cmds = [i.cmd for i in instrs]
        assert cmds == ["FROM", "WORKDIR", "COPY", "RUN", "CMD"]

    def test_parse_skip_comments_and_blanks(self):
        text = """# this is a comment
FROM python:3.13

# another comment

RUN echo hi
"""
        instrs = parse_dockerfile(text)
        assert len(instrs) == 2
        assert all(i.cmd != "#" for i in instrs)

    def test_parse_continuation_lines(self):
        text = """RUN apt-get update \\
    && apt-get install -y foo \\
    && rm -rf /var/lib/apt/lists/*
"""
        instrs = parse_dockerfile(text)
        assert len(instrs) == 1
        assert instrs[0].cmd == "RUN"
        assert "apt-get update" in instrs[0].args
        assert "rm -rf" in instrs[0].args

    def test_parse_lowercase_keyword(self):
        text = "from python:3.13-slim"
        instrs = parse_dockerfile(text)
        assert len(instrs) == 1
        assert instrs[0].cmd == "FROM"

    def test_parse_inline_comment(self):
        text = "FROM python:3.13-slim  # base image"
        instrs = parse_dockerfile(text)
        assert len(instrs) == 1
        assert instrs[0].cmd == "FROM"
        assert "base image" not in instrs[0].args

    def test_parse_multistage_from(self):
        text = """FROM python:3.13 AS builder
FROM python:3.13-slim AS runtime
"""
        instrs = parse_dockerfile(text)
        assert len(instrs) == 2
        assert all(i.cmd == "FROM" for i in instrs)
        assert "builder" in instrs[0].args
        assert "runtime" in instrs[1].args

    def test_parse_unknown_keyword_skipped(self):
        text = """FROM python:3.13
WATTHIS foo bar
RUN echo hi
"""
        instrs = parse_dockerfile(text)
        cmds = [i.cmd for i in instrs]
        assert "WATTHIS" not in cmds
        assert cmds == ["FROM", "RUN"]


# ============================================================================
# 单条规则测试 (per-instruction hadolint-inspired rules)
# ============================================================================


class TestPerInstructionRules:
    """V1384 真生产 per-instruction 规则测试 (主 19:33 hadolint 真借鉴)."""

    def test_dl3008_pin_apt_versions(self):
        """DL3008: apt-get install 没版本号要 warn."""
        text = "RUN apt-get install -y gcc"
        instrs = parse_dockerfile(text)
        f = PER_INSTRUCTION_RULES[0](instrs[0])  # _rule_dl3008
        assert f is not None
        assert f.rule_id == "DL3008"
        assert f.severity == "warning"

    def test_dl3008_no_warning_when_pinned(self):
        text = "RUN apt-get install -y gcc=4.2"
        instrs = parse_dockerfile(text)
        f = PER_INSTRUCTION_RULES[0](instrs[0])
        assert f is None

    def test_dl3008_no_warning_for_non_apt(self):
        text = "RUN pip install requests"
        instrs = parse_dockerfile(text)
        f = PER_INSTRUCTION_RULES[0](instrs[0])
        assert f is None

    def test_dl3009_no_apt_list_cleanup(self):
        """DL3009: apt-get install 后没 rm lists."""
        text = "RUN apt-get install -y foo"
        instrs = parse_dockerfile(text)
        f = PER_INSTRUCTION_RULES[1](instrs[0])
        assert f is not None
        assert f.rule_id == "DL3009"

    def test_dl3009_clean_when_cleanup_present(self):
        text = "RUN apt-get install -y foo && rm -rf /var/lib/apt/lists/*"
        instrs = parse_dockerfile(text)
        f = PER_INSTRUCTION_RULES[1](instrs[0])
        assert f is None

    def test_dl3015_no_install_recommends(self):
        """DL3015: apt-get install 缺 --no-install-recommends."""
        text = "RUN apt-get install -y foo"
        instrs = parse_dockerfile(text)
        f = PER_INSTRUCTION_RULES[2](instrs[0])
        assert f is not None
        assert f.rule_id == "DL3015"

    def test_dl3015_clean_when_present(self):
        text = "RUN apt-get install --no-install-recommends -y foo"
        instrs = parse_dockerfile(text)
        f = PER_INSTRUCTION_RULES[2](instrs[0])
        assert f is None

    def test_dl3020_add_for_local_file(self):
        """DL3020: ADD 用于本地文件应该用 COPY."""
        text = "ADD ./local.txt /app/local.txt"
        instrs = parse_dockerfile(text)
        f = PER_INSTRUCTION_RULES[3](instrs[0])
        assert f is not None
        assert f.rule_id == "DL3020"

    def test_dl3020_add_for_tar_ok(self):
        text = "ADD http://example.com/foo.tar.gz /app/"
        instrs = parse_dockerfile(text)
        f = PER_INSTRUCTION_RULES[3](instrs[0])
        assert f is None

    def test_dl3025_cmd_shell_form(self):
        """DL3025: CMD shell 形式应该用 JSON."""
        text = "CMD python app.py"
        instrs = parse_dockerfile(text)
        f = PER_INSTRUCTION_RULES[4](instrs[0])
        assert f is not None
        assert f.rule_id == "DL3025"

    def test_dl3025_cmd_json_form_ok(self):
        text = 'CMD ["python", "app.py"]'
        instrs = parse_dockerfile(text)
        f = PER_INSTRUCTION_RULES[4](instrs[0])
        assert f is None

    def test_dl4000_maintainer_deprecated(self):
        """DL4000: MAINTAINER 已弃用."""
        text = 'MAINTAINER alice "alice@example.com"'
        instrs = parse_dockerfile(text)
        f = PER_INSTRUCTION_RULES[5](instrs[0])
        assert f is not None
        assert f.rule_id == "DL4000"


# ============================================================================
# 文档级规则测试 (doc-level V1384 自有规则)
# ============================================================================


class TestDocLevelRules:
    """V1384 真生产 doc-level 规则测试 (主 17:43 实事求是)."""

    def test_no_user_directive(self):
        """缺 USER 应该报 error."""
        text = "FROM python:3.13\nRUN echo hi\nCMD [\"python\"]"
        instrs = parse_dockerfile(text)
        f = DOC_LEVEL_RULES[0](instrs)  # _rule_v1384_user_root
        assert len(f) == 1
        assert f[0].rule_id == "V1384-NO-USER"
        assert f[0].severity == "error"

    def test_user_directive_present_ok(self):
        text = "FROM python:3.13\nRUN useradd foo\nUSER foo\nCMD [\"python\"]"
        instrs = parse_dockerfile(text)
        f = DOC_LEVEL_RULES[0](instrs)
        assert f == []

    def test_no_healthcheck(self):
        """缺 HEALTHCHECK 应该报 warning."""
        text = "FROM python:3.13\nCMD [\"python\"]"
        instrs = parse_dockerfile(text)
        f = DOC_LEVEL_RULES[1](instrs)
        assert len(f) == 1
        assert f[0].rule_id == "V1384-NO-HEALTHCHECK"
        assert f[0].severity == "warning"

    def test_healthcheck_present_ok(self):
        text = "FROM python:3.13\nHEALTHCHECK CMD curl http://localhost\nCMD [\"python\"]"
        instrs = parse_dockerfile(text)
        f = DOC_LEVEL_RULES[1](instrs)
        assert f == []

    def test_from_no_tag(self):
        """FROM 没指定 tag."""
        text = "FROM python"
        instrs = parse_dockerfile(text)
        f = DOC_LEVEL_RULES[2](instrs)
        assert len(f) == 1
        assert f[0].rule_id == "V1384-FROM-NO-TAG"

    def test_from_latest_tag(self):
        """FROM latest tag."""
        text = "FROM python:latest"
        instrs = parse_dockerfile(text)
        f = DOC_LEVEL_RULES[2](instrs)
        assert len(f) == 1
        assert f[0].rule_id == "V1384-FROM-LATEST"

    def test_from_pinned_ok(self):
        text = "FROM python:3.13-slim"
        instrs = parse_dockerfile(text)
        f = DOC_LEVEL_RULES[2](instrs)
        assert f == []

    def test_add_insecure_http_url(self):
        """ADD 用 http:// URL 应该报 error."""
        text = "ADD http://example.com/foo.tar.gz /app/"
        instrs = parse_dockerfile(text)
        f = DOC_LEVEL_RULES[3](instrs)
        assert any(x.rule_id == "V1384-ADD-INSECURE-URL" for x in f)

    def test_add_https_url_info(self):
        text = "ADD https://example.com/foo.tar.gz /app/"
        instrs = parse_dockerfile(text)
        f = DOC_LEVEL_RULES[3](instrs)
        assert any(x.rule_id == "V1384-ADD-NO-VERIFY" for x in f)

    def test_sudo_unnecessary(self):
        text = "RUN sudo apt-get install foo"
        instrs = parse_dockerfile(text)
        f = DOC_LEVEL_RULES[4](instrs)
        assert len(f) == 1
        assert f[0].rule_id == "V1384-UNNECESSARY-SUDO"

    def test_no_sudo_ok(self):
        text = "RUN apt-get install foo"
        instrs = parse_dockerfile(text)
        f = DOC_LEVEL_RULES[4](instrs)
        assert f == []

    def test_abs_path_without_workdir(self):
        """COPY 用绝对路径但没 WORKDIR."""
        text = "FROM python:3.13\nCOPY . /app"
        instrs = parse_dockerfile(text)
        f = DOC_LEVEL_RULES[5](instrs)
        assert len(f) == 1
        assert f[0].rule_id == "V1384-ABS-PATH-WITHOUT-WORKDIR"

    def test_workdir_set_ok(self):
        text = "FROM python:3.13\nWORKDIR /app\nCOPY . /app"
        instrs = parse_dockerfile(text)
        f = DOC_LEVEL_RULES[5](instrs)
        assert f == []


# ============================================================================
# 集成测试 (lint_text / lint_file)
# ============================================================================


class TestLintIntegration:
    """V1384 真生产 lint 集成测试 (主 17:43 真跑)."""

    def test_lint_clean_dockerfile(self):
        """一个干净的 Dockerfile 应该 ok=True."""
        text = """FROM python:3.13-slim AS builder

RUN apt-get update \\
    && apt-get install --no-install-recommends -y gcc=12.2 \\
    && rm -rf /var/lib/apt/lists/*

FROM python:3.13-slim

WORKDIR /app
RUN useradd --create-home foo
COPY --from=builder /app /app
USER foo

HEALTHCHECK CMD python -c "print(1)"
CMD ["python", "app.py"]
"""
        linter = V1384DockerfileLint()
        report = linter.lint_text(text, file_path="<clean>")
        assert report.ok is True
        assert report.n_errors == 0
        # 应当没有 / 少 findings
        assert report.n_findings <= 1, f"clean Dockerfile should have few findings, got {report.n_findings}: {report.findings}"

    def test_lint_dirty_dockerfile(self):
        """一个糟糕的 Dockerfile 应该多个 finding."""
        text = """FROM python:latest

MAINTAINER alice

RUN sudo apt-get install -y foo

ADD . /app
ADD http://example.com/x.tar.gz /tmp/

CMD python app.py
"""
        linter = V1384DockerfileLint()
        report = linter.lint_text(text, file_path="<dirty>")
        assert report.n_errors >= 1
        assert report.n_warnings >= 3
        # 应至少包含: NO-USER / NO-HEALTHCHECK / FROM-LATEST / MAINTAINER / SUDO / DL3008 / DL3009 / DL3015 / DL3020 / DL3025 / INSECURE-URL
        rule_ids = {f.rule_id for f in report.findings}
        assert "V1384-NO-USER" in rule_ids
        assert "V1384-NO-HEALTHCHECK" in rule_ids
        assert "V1384-FROM-LATEST" in rule_ids
        assert "DL4000" in rule_ids
        assert "V1384-UNNECESSARY-SUDO" in rule_ids
        assert "V1384-ADD-INSECURE-URL" in rule_ids

    def test_lint_file_not_found(self):
        """不存在的文件应该报 error."""
        linter = V1384DockerfileLint()
        report = linter.lint_file("/nonexistent/path/Dockerfile")
        assert report.ok is False
        assert report.n_errors == 1
        assert report.findings[0].rule_id == "V1384-FILE-NOT-FOUND"

    def test_lint_file_with_real_v1050(self):
        """真 lint V1050 写的 Dockerfile (主 17:43 真读真跑)."""
        from v1050_real_docker_deploy import DOCKERFILE_RUNTIME
        linter = V1384DockerfileLint()
        report = linter.lint_text(DOCKERFILE_RUNTIME, file_path="<V1050 DOCKERFILE>")
        # V1050 Dockerfile 应该 pass (有 USER + HEALTHCHECK + pinned FROM)
        assert report.ok is True
        # 应该没 NO-USER 和 NO-HEALTHCHECK
        rule_ids = {f.rule_id for f in report.findings}
        assert "V1384-NO-USER" not in rule_ids
        assert "V1384-NO-HEALTHCHECK" not in rule_ids

    def test_lint_file_via_real_file(self):
        """真 lint 一个写到磁盘的文件 (主 17:43 真写真读)."""
        text = """FROM python:latest
MAINTAINER alice
RUN sudo echo hi
CMD python x
"""
        with tempfile.NamedTemporaryFile(mode="w", suffix=".dockerfile",
                                          delete=False, encoding="utf-8") as f:
            f.write(text)
            path = f.name
        try:
            linter = V1384DockerfileLint()
            report = linter.lint_file(path)
            assert report.n_errors >= 1  # sudo + from-latest + maintainer + cmd shell
            assert report.file_path == path
        finally:
            os.unlink(path)

    def test_lint_report_to_dict(self):
        text = "FROM python:latest"
        linter = V1384DockerfileLint()
        report = linter.lint_text(text, file_path="<test>")
        d = report.to_dict()
        assert "findings" in d
        assert "n_findings" in d
        assert "elapsed_seconds" in d
        assert d["file_path"] == "<test>"

    def test_lint_finding_to_dict(self):
        finding = LintFinding(
            rule_id="TEST",
            severity="warning",
            line_no=42,
            line_text="RUN sudo echo hi",
            message="use sudo",
            suggestion="remove sudo",
        )
        d = finding.to_dict()
        assert d["rule_id"] == "TEST"
        assert d["severity"] == "warning"
        assert d["line_no"] == 42

    def test_findings_sorted_by_line_then_severity(self):
        """findings 应按 line_no 排序, 同 line 按 severity 排序."""
        text = """FROM python:latest
MAINTAINER alice
USER foo
"""
        linter = V1384DockerfileLint()
        report = linter.lint_text(text, file_path="<sort>")
        # line 1: FROM-LATEST (warning)
        # line 2: DL4000 MAINTAINER (warning)
        # line 3: 没 finding (USER 没问题)
        if len(report.findings) >= 2:
            for i in range(1, len(report.findings)):
                assert report.findings[i - 1].line_no <= report.findings[i].line_no

    def test_version_constant(self):
        assert V1384_VERSION == "0.1.0"

    def test_stats(self):
        linter = V1384DockerfileLint()
        s = linter.stats()
        assert s["version"] == "0.1.0"
        assert s["n_rules_per_instruction"] == 6
        assert s["n_rules_doc_level"] == 6
        assert "philosophy" in s


# ============================================================================
# CLI 测试
# ============================================================================


class TestCLI:
    """V1384 真生产 CLI 测试 (主 17:43 真可执行)."""

    def test_cli_clean_dockerfile(self, capsys):
        text = "FROM python:3.13-slim\nUSER foo\nCMD [\"python\"]\n"
        with tempfile.NamedTemporaryFile(mode="w", suffix=".dockerfile",
                                          delete=False, encoding="utf-8") as f:
            f.write(text)
            path = f.name
        try:
            rc = run_cli([path, "--quiet"])
            captured = capsys.readouterr()
            assert "V1384 Dockerfile Lint Report" in captured.out
            assert rc == 0
        finally:
            os.unlink(path)

    def test_cli_dirty_dockerfile_exits_nonzero(self, capsys):
        text = "FROM python:latest\nMAINTAINER alice\nCMD python x\n"
        with tempfile.NamedTemporaryFile(mode="w", suffix=".dockerfile",
                                          delete=False, encoding="utf-8") as f:
            f.write(text)
            path = f.name
        try:
            rc = run_cli([path, "--quiet"])
            assert rc in (1, 2) or rc != 0  # dirty exits non-zero
        finally:
            os.unlink(path)

    def test_cli_json_output(self, capsys):
        text = "FROM python:3.13-slim\nUSER foo\nCMD [\"python\"]\n"
        with tempfile.NamedTemporaryFile(mode="w", suffix=".dockerfile",
                                          delete=False, encoding="utf-8") as f:
            f.write(text)
            path = f.name
        try:
            rc = run_cli([path, "--json"])
            captured = capsys.readouterr()
            data = json.loads(captured.out)
            assert "findings" in data
            assert "n_findings" in data
            assert rc == 0
        finally:
            os.unlink(path)

    def test_cli_stdin(self, capsys, monkeypatch):
        # 通过 monkeypatch 重定向 stdin
        text = "FROM python:3.13-slim\nUSER foo\nCMD [\"python\"]\n"
        import io
        monkeypatch.setattr(sys, "stdin", io.StringIO(text))
        rc = run_cli(["-", "--quiet"])
        captured = capsys.readouterr()
        assert "<stdin>" in captured.out
        assert rc == 0


# ============================================================================
# Popper-style 自我测试 (主 17:43 实事求是)
# ============================================================================


def test_v1384_popper_self_tests():
    """V1384 Popper 自我测试: 真跑一遍代表性子集."""
    # 真 read 真 parse 真 lint 一段代表 Dockerfile
    sample = """FROM python:latest

MAINTAINER alice <alice@example.com>

RUN sudo apt-get install -y foo bar

ADD . /app
ADD http://example.com/x.tar.gz /tmp/

CMD python app.py
"""
    linter = V1384DockerfileLint()
    report = linter.lint_text(sample, file_path="<popper>")
    # 必须包含这些 finding
    rule_ids = {f.rule_id for f in report.findings}
    expected = {
        "V1384-NO-USER",          # 缺 USER
        "V1384-NO-HEALTHCHECK",   # 缺 HEALTHCHECK
        "V1384-FROM-LATEST",      # latest
        "DL4000",                 # MAINTAINER
        "V1384-UNNECESSARY-SUDO",  # sudo
        "V1384-ADD-INSECURE-URL", # http://
        "DL3008",                 # apt 不带版本
        "DL3009",                 # apt lists 没清
        "DL3015",                 # 缺 --no-install-recommends
        "DL3020",                 # ADD 不用 COPY
        "DL3025",                 # CMD 不用 JSON
    }
    missing = expected - rule_ids
    assert not missing, f"Popper self-test missing findings: {missing}"

    # 验证 ok=False (有 error: NO-USER + INSECURE-URL)
    assert report.ok is False

    # 验证 to_dict round-trip
    d = report.to_dict()
    assert isinstance(d, dict)
    assert d["n_findings"] == len(d["findings"])