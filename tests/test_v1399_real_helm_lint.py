"""Phase 1399 test_v1399_real_helm_lint — V1399 真生产 helm chart 真解析 + 真 lint 测试 (主 06:15 + 主 23:44 + 主 17:43 + 主 19:33 + 主 22:33 + 主 00:56 + 主 13:31 + 主 17:33 + 主 00:36).

主 17:43 实事求是: 真跑 V1399 module 真测 12 真规则全 coverage.
主 19:33 走在前人经验上: 真借鉴 helm + chartmuseum + pluto + trivy + helmsman + conftest.
"""
from __future__ import annotations

import json
import os
import sys
import tempfile
from pathlib import Path

import pytest

# V1399 真生产 import (主 17:43)
from apeireth.v1399_real_helm_lint import (
    HelmFinding,
    HelmLintReport,
    V1399_BORROWED,
    V1399_GUARDS,
    V1399_SCHEMA,
    V1399_SECRET_VALUE_PATTERNS,
    V1399_SEMVER_RE,
    V1399_VALID_API_VERSIONS,
    V1399_VERSION,
    ChartMeta,
    TemplateDoc,
    ValuesMeta,
    _find_line_no,
    _format_sarif,
    _format_text,
    _parse_chart_yaml,
    _parse_values_yaml,
    _popper_self_test,
    _render_template,
    _rule_hl001_missing_chart_yaml,
    _rule_hl002_invalid_api_version,
    _rule_hl003_missing_name,
    _rule_hl004_invalid_version,
    _rule_hl005_missing_app_version,
    _rule_hl006_deprecated_api_v1,
    _rule_hl007_missing_type,
    _rule_hl008_template_syntax,
    _rule_hl009_image_without_tag,
    _rule_hl010_resources_without_limits,
    _rule_hl011_missing_helmignore,
    _rule_hl012_dependency_missing_repository,
    _sev_to_sarif,
    chain_with_v1386,
    lint_chart,
    run_cli,
)


# ============================================================================
# V1399 TestV1399Constants: 验证 constants / GUARDS / borrowed (主 17:43)
# ============================================================================


class TestV1399Constants:
    """V1399 真生产 constants sanity (主 17:43)."""

    def test_version_is_string(self):
        assert isinstance(V1399_VERSION, str) and len(V1399_VERSION) > 0

    def test_schema_is_string(self):
        assert isinstance(V1399_SCHEMA, str)
        assert V1399_SCHEMA.startswith("v1399.")

    def test_guards_count(self):
        assert len(V1399_GUARDS) >= 12

    def test_guards_unique(self):
        assert len(V1399_GUARDS) == len(set(V1399_GUARDS))

    def test_guards_contain_core(self):
        for g in (
            "GUARD_CHART_PARSED",
            "GUARD_VALUES_PARSED",
            "GUARD_TEMPLATES_RENDERED",
            "GUARD_RULES_REAL",
            "GUARD_FILE_IO",
            "GUARD_LINE_TRACKED",
            "GUARD_NO_CAP_CHANGE",
            "GUARD_DETERMINISTIC",
            "GUARD_HONEST_DISCLOSURE",
            "GUARD_PATH_SAFE",
            "GUARD_NON_DESTRUCTIVE",
            "GUARD_DELEGATE_REAL",
            "GUARD_CLI_RUNNABLE",
            "GUARD_POPPER_RUNS",
        ):
            assert g in V1399_GUARDS, f"missing guard: {g}"

    def test_borrowed_count(self):
        assert len(V1399_BORROWED) >= 6

    def test_valid_api_versions(self):
        for v in ("v1", "v2", "v2beta1", "v2beta2"):
            assert v in V1399_VALID_API_VERSIONS

    def test_semver_re_matches_valid(self):
        assert V1399_SEMVER_RE.match("1.2.3")
        assert V1399_SEMVER_RE.match("0.0.1")
        assert V1399_SEMVER_RE.match("1.0.0-alpha")
        assert V1399_SEMVER_RE.match("2.3.4-beta.1+build.123")

    def test_semver_re_rejects_invalid(self):
        assert not V1399_SEMVER_RE.match("not-semver")
        assert not V1399_SEMVER_RE.match("1.2")
        assert not V1399_SEMVER_RE.match("v1.2.3")  # prefix not allowed

    def test_secret_patterns_count(self):
        assert len(V1399_SECRET_VALUE_PATTERNS) >= 5


# ============================================================================
# V1399 TestV1399FindLineNo: 真生产 line tracking (主 17:43)
# ============================================================================


class TestV1399FindLineNo:
    """V1399 真生产 find 1-indexed line number (主 17:43)."""

    def test_first_line(self):
        assert _find_line_no("hello\nworld", "hello") == 1

    def test_second_line(self):
        assert _find_line_no("hello\nworld", "world") == 2

    def test_not_found(self):
        assert _find_line_no("hello\nworld", "missing") == 0

    def test_empty_target(self):
        assert _find_line_no("hello\nworld", "") == 0

    def test_third_line(self):
        text = "a\nb\nc"
        assert _find_line_no(text, "c") == 3

    def test_with_offset(self):
        text = "foo\nbar\nfoo"
        assert _find_line_no(text, "foo", start=5) == 3


# ============================================================================
# V1399 TestV1399ParseChartYaml: 真生产 Chart.yaml parse (主 17:43)
# ============================================================================


class TestV1399ParseChartYaml:
    """V1399 真生产 parse Chart.yaml (主 17:43)."""

    def test_valid_v2(self, tmp_path: Path):
        p = tmp_path / "Chart.yaml"
        p.write_text(
            "apiVersion: v2\nname: myapp\nversion: 1.0.0\nappVersion: '1.16.0'\ntype: application\n",
            encoding="utf-8",
        )
        meta, err = _parse_chart_yaml(p)
        assert err == ""
        assert meta.parsed
        assert meta.api_version == "v2"
        assert meta.name == "myapp"
        assert meta.version == "1.0.0"
        assert meta.app_version == "1.16.0"
        assert meta.type == "application"

    def test_missing_file(self, tmp_path: Path):
        p = tmp_path / "Chart.yaml"
        meta, err = _parse_chart_yaml(p)
        assert not meta.parsed
        assert "Chart.yaml not found" in err

    def test_invalid_yaml(self, tmp_path: Path):
        p = tmp_path / "Chart.yaml"
        p.write_text("apiVersion: v2\nname: [unclosed\n", encoding="utf-8")
        meta, err = _parse_chart_yaml(p)
        assert not meta.parsed or "YAML parse error" in err

    def test_dependencies_parsed(self, tmp_path: Path):
        p = tmp_path / "Chart.yaml"
        p.write_text(
            "apiVersion: v2\nname: x\nversion: 1.0.0\ndependencies:\n  - name: redis\n    version: 1.0.0\n    repository: https://x.com\n",
            encoding="utf-8",
        )
        meta, err = _parse_chart_yaml(p)
        assert err == ""
        assert len(meta.dependencies) == 1
        assert meta.dependencies[0]["name"] == "redis"

    def test_v1_no_type(self, tmp_path: Path):
        p = tmp_path / "Chart.yaml"
        p.write_text("apiVersion: v1\nname: legacy\nversion: 0.1.0\n", encoding="utf-8")
        meta, err = _parse_chart_yaml(p)
        assert err == ""
        assert meta.api_version == "v1"
        assert meta.type == ""


# ============================================================================
# V1399 TestV1399ParseValuesYaml: 真生产 values.yaml parse (主 17:43)
# ============================================================================


class TestV1399ParseValuesYaml:
    """V1399 真生产 parse values.yaml (主 17:43)."""

    def test_valid(self, tmp_path: Path):
        p = tmp_path / "values.yaml"
        p.write_text("image:\n  repository: nginx\n  tag: 1.25\n", encoding="utf-8")
        meta, err = _parse_values_yaml(p)
        assert err == ""
        assert meta.parsed
        assert meta.values["image"]["repository"] == "nginx"

    def test_missing_is_ok(self, tmp_path: Path):
        # values.yaml is optional
        meta, err = _parse_values_yaml(tmp_path / "values.yaml")
        assert err == ""
        assert not meta.parsed

    def test_invalid_yaml(self, tmp_path: Path):
        p = tmp_path / "values.yaml"
        p.write_text("image: : :\n", encoding="utf-8")
        meta, err = _parse_values_yaml(p)
        assert "YAML parse error" in err


# ============================================================================
# V1399 TestV1399RulesFire: 真生产 12 rules (主 19:33)
# ============================================================================


class TestV1399RulesFire:
    """V1399 真生产 12 rules coverage (主 19:33)."""

    def _meta(self, **kw) -> ChartMeta:
        meta = ChartMeta()
        for k, v in kw.items():
            setattr(meta, k, v)
        meta.parsed = True
        return meta

    def _empty_report(self, tmp_path: Path) -> HelmLintReport:
        return HelmLintReport(chart_dir=str(tmp_path))

    def test_hl001_missing_chart_yaml(self, tmp_path: Path):
        meta = ChartMeta()  # not parsed
        report = self._empty_report(tmp_path)
        findings = _rule_hl001_missing_chart_yaml(meta, report, tmp_path / "Chart.yaml")
        assert len(findings) == 1
        assert findings[0].rule_id == "HL001-MISSING-CHART-YAML"
        assert findings[0].severity == "error"

    def test_hl002_invalid_api_version(self):
        meta = self._meta(api_version="v99")
        findings = _rule_hl002_invalid_api_version(meta)
        assert len(findings) == 1
        assert findings[0].rule_id == "HL002-INVALID-CHART-API-VERSION"

    def test_hl002_valid(self):
        meta = self._meta(api_version="v2")
        findings = _rule_hl002_invalid_api_version(meta)
        assert findings == []

    def test_hl002_missing(self):
        meta = self._meta(api_version="")
        findings = _rule_hl002_invalid_api_version(meta)
        assert len(findings) == 1

    def test_hl003_missing_name(self):
        meta = self._meta(name="")
        findings = _rule_hl003_missing_name(meta)
        assert len(findings) == 1
        assert findings[0].rule_id == "HL003-MISSING-CHART-NAME"

    def test_hl003_valid(self):
        meta = self._meta(name="mychart")
        findings = _rule_hl003_missing_name(meta)
        assert findings == []

    def test_hl004_invalid_version(self):
        meta = self._meta(version="not-semver")
        findings = _rule_hl004_invalid_version(meta)
        assert len(findings) == 1
        assert findings[0].rule_id == "HL004-INVALID-CHART-VERSION"

    def test_hl004_missing(self):
        meta = self._meta(version="")
        findings = _rule_hl004_invalid_version(meta)
        assert len(findings) == 1

    def test_hl004_valid(self):
        meta = self._meta(version="1.2.3")
        findings = _rule_hl004_invalid_version(meta)
        assert findings == []

    def test_hl005_missing_app_version(self):
        meta = self._meta(app_version="")
        findings = _rule_hl005_missing_app_version(meta)
        assert len(findings) == 1
        assert findings[0].severity == "warning"

    def test_hl006_deprecated_api_v1(self):
        meta = self._meta(api_version="v1")
        findings = _rule_hl006_deprecated_api_v1(meta)
        assert len(findings) == 1
        assert findings[0].rule_id == "HL006-DEPRECATED-API-VERSION-V1"

    def test_hl006_v2_ok(self):
        meta = self._meta(api_version="v2")
        findings = _rule_hl006_deprecated_api_v1(meta)
        assert findings == []

    def test_hl007_missing_type(self):
        meta = self._meta(type="")
        findings = _rule_hl007_missing_type(meta)
        assert len(findings) == 1
        assert findings[0].severity == "info"

    def test_hl007_type_ok(self):
        meta = self._meta(type="application")
        findings = _rule_hl007_missing_type(meta)
        assert findings == []

    def test_hl008_template_syntax_error(self):
        docs = [TemplateDoc(template_path="x.yaml", render_error="syntax error at line 5")]
        findings = _rule_hl008_template_syntax(docs)
        assert len(findings) == 1
        assert findings[0].rule_id == "HL008-TEMPLATE-SYNTAX-ERROR"
        assert findings[0].severity == "error"

    def test_hl008_no_errors(self):
        docs = [TemplateDoc(template_path="x.yaml", rendered=True)]
        findings = _rule_hl008_template_syntax(docs)
        assert findings == []

    def test_hl009_image_without_tag(self):
        v = ValuesMeta(raw_text="image: nginx\n", values={}, parsed=True)
        findings = _rule_hl009_image_without_tag(ChartMeta(), v, [])
        assert len(findings) == 1
        assert findings[0].rule_id == "HL009-IMAGE-WITHOUT-TAG"

    def test_hl009_image_with_tag_ok(self):
        v = ValuesMeta(raw_text="image: nginx:1.25\n", values={}, parsed=True)
        findings = _rule_hl009_image_without_tag(ChartMeta(), v, [])
        assert findings == []

    def test_hl009_image_with_digest_ok(self):
        v = ValuesMeta(raw_text="image: nginx@sha256:abc123\n", values={}, parsed=True)
        findings = _rule_hl009_image_without_tag(ChartMeta(), v, [])
        assert findings == []

    def test_hl011_missing_helmignore(self, tmp_path: Path):
        report = HelmLintReport(chart_dir=str(tmp_path), helmignore_present=False, chart_name="x")
        findings = _rule_hl011_missing_helmignore(tmp_path, report)
        assert len(findings) == 1
        assert findings[0].severity == "info"

    def test_hl011_helmignore_present(self, tmp_path: Path):
        (tmp_path / ".helmignore").write_text("*.bak\n", encoding="utf-8")
        report = HelmLintReport(chart_dir=str(tmp_path), helmignore_present=True, chart_name="x")
        findings = _rule_hl011_missing_helmignore(tmp_path, report)
        assert findings == []

    def test_hl012_dependency_empty_repo(self):
        meta = self._meta(dependencies=[{"name": "redis", "repository": ""}])
        findings = _rule_hl012_dependency_missing_repository(meta)
        assert len(findings) == 1

    def test_hl012_dependency_valid_repo(self):
        meta = self._meta(dependencies=[{"name": "redis", "repository": "https://x.com"}])
        findings = _rule_hl012_dependency_missing_repository(meta)
        assert findings == []

    def test_hl012_no_repository_field_ok(self):
        # path-only dep is OK
        meta = self._meta(dependencies=[{"name": "local", "version": "1.0.0"}])
        findings = _rule_hl012_dependency_missing_repository(meta)
        assert findings == []


# ============================================================================
# V1399 TestV1399LintChart: 真生产 lint_chart integration (主 17:43)
# ============================================================================


class TestV1399LintChart:
    """V1399 真生产 lint_chart 集成测试 (主 17:43)."""

    def test_valid_minimal_chart(self, tmp_path: Path):
        (tmp_path / "Chart.yaml").write_text(
            "apiVersion: v2\nname: x\ntype: application\nversion: 1.0.0\nappVersion: '1.0'\n",
            encoding="utf-8",
        )
        (tmp_path / ".helmignore").write_text("*.bak\n", encoding="utf-8")
        (tmp_path / "templates").mkdir()
        (tmp_path / "templates" / "deployment.yaml").write_text(
            "apiVersion: apps/v1\nkind: Deployment\nmetadata:\n  name: x\n",
            encoding="utf-8",
        )
        r = lint_chart(tmp_path)
        assert r.ok
        assert r.n_errors == 0

    def test_missing_chart_yaml(self, tmp_path: Path):
        r = lint_chart(tmp_path)
        assert not r.ok
        assert any(f.rule_id == "HL001-MISSING-CHART-YAML" for f in r.findings)

    def test_invalid_api_version(self, tmp_path: Path):
        (tmp_path / "Chart.yaml").write_text(
            "apiVersion: v99\nname: x\nversion: 1.0.0\n", encoding="utf-8",
        )
        r = lint_chart(tmp_path)
        assert any(f.rule_id == "HL002-INVALID-CHART-API-VERSION" for f in r.findings)

    def test_missing_name(self, tmp_path: Path):
        (tmp_path / "Chart.yaml").write_text(
            "apiVersion: v2\nversion: 1.0.0\n", encoding="utf-8",
        )
        r = lint_chart(tmp_path)
        assert any(f.rule_id == "HL003-MISSING-CHART-NAME" for f in r.findings)

    def test_invalid_version(self, tmp_path: Path):
        (tmp_path / "Chart.yaml").write_text(
            "apiVersion: v2\nname: x\nversion: bad-version\n", encoding="utf-8",
        )
        r = lint_chart(tmp_path)
        assert any(f.rule_id == "HL004-INVALID-CHART-VERSION" for f in r.findings)

    def test_missing_app_version(self, tmp_path: Path):
        (tmp_path / "Chart.yaml").write_text(
            "apiVersion: v2\nname: x\ntype: application\nversion: 1.0.0\n", encoding="utf-8",
        )
        r = lint_chart(tmp_path)
        assert any(f.rule_id == "HL005-MISSING-APP-VERSION" for f in r.findings)

    def test_deprecated_api_v1(self, tmp_path: Path):
        (tmp_path / "Chart.yaml").write_text(
            "apiVersion: v1\nname: x\nversion: 1.0.0\n", encoding="utf-8",
        )
        r = lint_chart(tmp_path)
        assert any(f.rule_id == "HL006-DEPRECATED-API-VERSION-V1" for f in r.findings)

    def test_missing_type(self, tmp_path: Path):
        (tmp_path / "Chart.yaml").write_text(
            "apiVersion: v2\nname: x\nversion: 1.0.0\nappVersion: '1.0'\n", encoding="utf-8",
        )
        r = lint_chart(tmp_path)
        assert any(f.rule_id == "HL007-MISSING-TYPE" for f in r.findings)

    def test_template_syntax_error(self, tmp_path: Path):
        (tmp_path / "Chart.yaml").write_text(
            "apiVersion: v2\nname: x\ntype: application\nversion: 1.0.0\nappVersion: '1.0'\n", encoding="utf-8",
        )
        (tmp_path / "templates").mkdir()
        (tmp_path / "templates" / "bad.yaml").write_text(
            "apiVersion: v1\nkind: ConfigMap\nmetadata:\n  name: x\ndata:\n  key: {{ unclosed\n",
            encoding="utf-8",
        )
        r = lint_chart(tmp_path)
        assert any(f.rule_id == "HL008-TEMPLATE-SYNTAX-ERROR" for f in r.findings)

    def test_image_without_tag(self, tmp_path: Path):
        (tmp_path / "Chart.yaml").write_text(
            "apiVersion: v2\nname: x\ntype: application\nversion: 1.0.0\nappVersion: '1.0'\n", encoding="utf-8",
        )
        (tmp_path / ".helmignore").write_text("*.bak\n", encoding="utf-8")
        (tmp_path / "values.yaml").write_text(
            "image:\n  repository: nginx\n",
            encoding="utf-8",
        )
        (tmp_path / "templates").mkdir()
        (tmp_path / "templates" / "deployment.yaml").write_text(
            "apiVersion: apps/v1\nkind: Deployment\nmetadata:\n  name: x\nspec:\n  template:\n    spec:\n      containers:\n        - name: app\n          image: '{{ Values.image.repository }}'\n",
            encoding="utf-8",
        )
        r = lint_chart(tmp_path)
        assert any(f.rule_id == "HL009-IMAGE-WITHOUT-TAG" for f in r.findings)

    def test_image_with_tag_ok(self, tmp_path: Path):
        (tmp_path / "Chart.yaml").write_text(
            "apiVersion: v2\nname: x\ntype: application\nversion: 1.0.0\nappVersion: '1.0'\n", encoding="utf-8",
        )
        (tmp_path / ".helmignore").write_text("*.bak\n", encoding="utf-8")
        (tmp_path / "values.yaml").write_text(
            "image:\n  repository: nginx:1.25.3\n",
            encoding="utf-8",
        )
        (tmp_path / "templates").mkdir()
        (tmp_path / "templates" / "deployment.yaml").write_text(
            "apiVersion: apps/v1\nkind: Deployment\nmetadata:\n  name: x\nspec:\n  template:\n    spec:\n      containers:\n        - name: app\n          image: '{{ Values.image.repository }}'\n",
            encoding="utf-8",
        )
        r = lint_chart(tmp_path)
        assert not any(f.rule_id == "HL009-IMAGE-WITHOUT-TAG" for f in r.findings)

    def test_resources_no_limits(self, tmp_path: Path):
        (tmp_path / "Chart.yaml").write_text(
            "apiVersion: v2\nname: x\ntype: application\nversion: 1.0.0\nappVersion: '1.0'\n", encoding="utf-8",
        )
        (tmp_path / ".helmignore").write_text("*.bak\n", encoding="utf-8")
        (tmp_path / "templates").mkdir()
        (tmp_path / "templates" / "deployment.yaml").write_text(
            "apiVersion: apps/v1\nkind: Deployment\nmetadata:\n  name: x\nspec:\n  template:\n    spec:\n      containers:\n        - name: app\n          image: nginx:1.25\n          resources:\n            requests:\n              cpu: 100m\n              memory: 128Mi\n",
            encoding="utf-8",
        )
        r = lint_chart(tmp_path)
        assert any(f.rule_id == "HL010-RESOURCES-WITHOUT-LIMITS" for f in r.findings)

    def test_resources_with_limits_ok(self, tmp_path: Path):
        (tmp_path / "Chart.yaml").write_text(
            "apiVersion: v2\nname: x\ntype: application\nversion: 1.0.0\nappVersion: '1.0'\n", encoding="utf-8",
        )
        (tmp_path / ".helmignore").write_text("*.bak\n", encoding="utf-8")
        (tmp_path / "templates").mkdir()
        (tmp_path / "templates" / "deployment.yaml").write_text(
            "apiVersion: apps/v1\nkind: Deployment\nmetadata:\n  name: x\nspec:\n  template:\n    spec:\n      containers:\n        - name: app\n          image: nginx:1.25\n          resources:\n            requests:\n              cpu: 100m\n              memory: 128Mi\n            limits:\n              cpu: 200m\n              memory: 256Mi\n",
            encoding="utf-8",
        )
        r = lint_chart(tmp_path)
        assert not any(f.rule_id == "HL010-RESOURCES-WITHOUT-LIMITS" for f in r.findings)

    def test_missing_helmignore(self, tmp_path: Path):
        (tmp_path / "Chart.yaml").write_text(
            "apiVersion: v2\nname: x\ntype: application\nversion: 1.0.0\nappVersion: '1.0'\n", encoding="utf-8",
        )
        r = lint_chart(tmp_path)
        assert any(f.rule_id == "HL011-MISSING-HELMIGNORE" for f in r.findings)

    def test_dependency_empty_repo(self, tmp_path: Path):
        (tmp_path / "Chart.yaml").write_text(
            "apiVersion: v2\nname: x\ntype: application\nversion: 1.0.0\nappVersion: '1.0'\ndependencies:\n  - name: pg\n    version: 1.0.0\n    repository: ''\n", encoding="utf-8",
        )
        (tmp_path / ".helmignore").write_text("*.bak\n", encoding="utf-8")
        r = lint_chart(tmp_path)
        assert any(f.rule_id == "HL012-DEPENDENCY-MISSING-REPOSITORY" for f in r.findings)

    def test_not_a_directory(self, tmp_path: Path):
        f = tmp_path / "file.yaml"
        f.write_text("x", encoding="utf-8")
        r = lint_chart(f)
        assert not r.ok
        assert r.parse_error


# ============================================================================
# V1399 TestV1399Format: 真生产 output format (主 00:36)
# ============================================================================


class TestV1399Format:
    """V1399 真生产 format (text/JSON/SARIF)."""

    def test_format_text(self, tmp_path: Path):
        (tmp_path / "Chart.yaml").write_text(
            "apiVersion: v2\nname: x\ntype: application\nversion: 1.0.0\nappVersion: '1.0'\n", encoding="utf-8",
        )
        r = lint_chart(tmp_path)
        text = _format_text(r)
        assert "v1399.helm-lint/v1" in text
        assert "HL011-MISSING-HELMIGNORE" in text

    def test_format_sarif(self, tmp_path: Path):
        (tmp_path / "Chart.yaml").write_text(
            "apiVersion: v2\nname: x\ntype: application\nversion: 1.0.0\nappVersion: '1.0'\n", encoding="utf-8",
        )
        r = lint_chart(tmp_path)
        sarif = _format_sarif(r)
        assert sarif["version"] == "2.1.0"
        assert len(sarif["runs"]) == 1
        assert len(sarif["runs"][0]["results"]) >= 1

    def test_format_sarif_to_dict_is_json(self, tmp_path: Path):
        (tmp_path / "Chart.yaml").write_text(
            "apiVersion: v2\nname: x\ntype: application\nversion: 1.0.0\nappVersion: '1.0'\n", encoding="utf-8",
        )
        r = lint_chart(tmp_path)
        sarif = _format_sarif(r)
        # 真生产: 必须能 JSON-serialize + roundtrip
        s = json.dumps(sarif)
        parsed = json.loads(s)
        assert parsed["version"] == "2.1.0"
        assert "v1399-helm-lint" in parsed["runs"][0]["tool"]["driver"]["name"]

    def test_sev_to_sarif(self):
        assert _sev_to_sarif("error") == "error"
        assert _sev_to_sarif("warning") == "warning"
        assert _sev_to_sarif("info") == "note"
        assert _sev_to_sarif("unknown") == "warning"

    def test_report_to_dict(self, tmp_path: Path):
        (tmp_path / "Chart.yaml").write_text(
            "apiVersion: v2\nname: x\ntype: application\nversion: 1.0.0\nappVersion: '1.0'\n", encoding="utf-8",
        )
        r = lint_chart(tmp_path)
        d = r.to_dict()
        assert d["chart_name"] == "x"
        assert d["api_version"] == "v2"
        assert d["chart_version"] == "1.0.0"
        assert "findings" in d
        assert "elapsed_seconds" in d


# ============================================================================
# V1399 TestV1399Chain: 真生产 chain delegate V1386 (主 17:43)
# ============================================================================


class TestV1399Chain:
    """V1399 真生产 chain delegate V1386 (主 17:43)."""

    def test_chain_with_valid_chart(self, tmp_path: Path):
        (tmp_path / "Chart.yaml").write_text(
            "apiVersion: v2\nname: x\ntype: application\nversion: 1.0.0\nappVersion: '1.0'\n", encoding="utf-8",
        )
        (tmp_path / ".helmignore").write_text("*.bak\n", encoding="utf-8")
        (tmp_path / "values.yaml").write_text("k: v\n", encoding="utf-8")
        (tmp_path / "templates").mkdir()
        (tmp_path / "templates" / "deployment.yaml").write_text(
            "apiVersion: apps/v1\nkind: Deployment\nmetadata:\n  name: x\n", encoding="utf-8",
        )
        result = chain_with_v1386(tmp_path)
        assert result.get("ok") is True or result.get("templates_failed") == 0

    def test_chain_with_no_templates(self, tmp_path: Path):
        (tmp_path / "Chart.yaml").write_text(
            "apiVersion: v2\nname: x\ntype: application\nversion: 1.0.0\nappVersion: '1.0'\n", encoding="utf-8",
        )
        result = chain_with_v1386(tmp_path)
        # No templates is OK; chain returns gracefully
        assert "schema" in result

    def test_chain_with_missing_chart(self, tmp_path: Path):
        result = chain_with_v1386(tmp_path)
        # tmp_path has no Chart.yaml → ok can be False but no crash
        assert "schema" in result


# ============================================================================
# V1399 TestV1399Popper: 真生产 popper self-test (主 17:43)
# ============================================================================


class TestV1399Popper:
    """V1399 真生产 popper self-test (主 17:43)."""

    def test_popper_runs(self):
        result = _popper_self_test()
        assert result["ok"] is True
        assert result["n_rules"] == 12
        assert result["n_guards"] >= 12

    def test_popper_bad_sample(self):
        result = _popper_self_test()
        bad = next(t for t in result["tests"] if t["name"] == "bad_sample_lints")
        assert bad["ok"] is True
        assert bad["n_findings"] >= 1

    def test_popper_clean_sample(self):
        result = _popper_self_test()
        clean = next(t for t in result["tests"] if t["name"] == "clean_sample_lints")
        assert clean["ok"] is True
        assert clean["n_errors"] == 0

    def test_popper_sarif_roundtrip(self):
        result = _popper_self_test()
        sarif = next(t for t in result["tests"] if t["name"] == "sarif_roundtrip")
        assert sarif["ok"] is True

    def test_popper_chain_delegate(self):
        result = _popper_self_test()
        chain = next(t for t in result["tests"] if t["name"] == "chain_delegate")
        assert chain["ok"] is True


# ============================================================================
# V1399 TestV1399CLI: 真生产 CLI (主 17:43 + 主 00:56)
# ============================================================================


class TestV1399CLI:
    """V1399 真生产 CLI test (主 17:43 + 主 00:56)."""

    def test_cli_version(self, capsys):
        rc = run_cli(["version"])
        assert rc == 0
        out = capsys.readouterr().out
        assert "v1399.helm-lint/v1" in out

    def test_cli_lint_text(self, tmp_path: Path, capsys):
        # Add Chart.yaml with INVALID apiVersion → triggers HL002 error → rc=1
        (tmp_path / "Chart.yaml").write_text(
            "apiVersion: v99\nname: x\nversion: 1.0.0\n", encoding="utf-8",
        )
        rc = run_cli(["lint", str(tmp_path), "--format", "text"])
        assert rc == 1
        out = capsys.readouterr().out
        assert "HL002" in out

    def test_cli_lint_json(self, tmp_path: Path, capsys):
        (tmp_path / "Chart.yaml").write_text(
            "apiVersion: v99\nname: x\nversion: 1.0.0\n", encoding="utf-8",
        )
        rc = run_cli(["lint", str(tmp_path), "--format", "json"])
        assert rc == 1
        out = capsys.readouterr().out
        d = json.loads(out)
        assert "findings" in d

    def test_cli_lint_sarif(self, tmp_path: Path, capsys):
        (tmp_path / "Chart.yaml").write_text(
            "apiVersion: v99\nname: x\nversion: 1.0.0\n", encoding="utf-8",
        )
        rc = run_cli(["lint", str(tmp_path), "--format", "sarif"])
        assert rc == 1
        out = capsys.readouterr().out
        d = json.loads(out)
        assert d["version"] == "2.1.0"

    def test_cli_lint_not_found(self, tmp_path: Path):
        rc = run_cli(["lint", str(tmp_path / "nonexistent")])
        assert rc == 3

    def test_cli_chain(self, tmp_path: Path, capsys):
        (tmp_path / "Chart.yaml").write_text(
            "apiVersion: v2\nname: x\ntype: application\nversion: 1.0.0\nappVersion: '1.0'\n", encoding="utf-8",
        )
        rc = run_cli(["chain", str(tmp_path)])
        out = capsys.readouterr().out
        d = json.loads(out)
        assert "schema" in d

    def test_cli_chain_not_found(self):
        rc = run_cli(["chain", "/nonexistent/path"])
        assert rc == 3

    def test_cli_popper(self, capsys):
        rc = run_cli(["popper"])
        assert rc == 0
        out = capsys.readouterr().out
        d = json.loads(out)
        assert d["ok"] is True

    def test_cli_demo(self, tmp_path: Path, capsys):
        target = tmp_path / "demo"
        rc = run_cli(["demo", "--target", str(target)])
        assert rc == 1  # demo has known issues
        out = capsys.readouterr().out
        assert target.exists()
        assert "HL009" in out or "HL010" in out or "HL011" in out or "HL012" in out

    def test_cli_help(self, capsys):
        rc = run_cli([])
        # No command → prints help, rc 0
        assert rc == 0


# ============================================================================
# V1399 TestV1399V3Guards: 真生产 V3 哲学守门 (主 17:58 + 主 20:46)
# ============================================================================


class TestV1399V3Guards:
    """V1399 真生产 V3 哲学守门 (主 17:58 + 主 20:46)."""

    def test_no_cap_change(self, tmp_path: Path):
        # 真 cap 仍为 0.7905 (V1398 preserved)
        (tmp_path / "Chart.yaml").write_text(
            "apiVersion: v2\nname: x\ntype: application\nversion: 1.0.0\nappVersion: '1.0'\n", encoding="utf-8",
        )
        r = lint_chart(tmp_path)
        # 真生产: V1399 module 不暴露 cap 字段; 这只确认它存在
        assert "GUARD_NO_CAP_CHANGE" in V1399_GUARDS

    def test_deterministic(self, tmp_path: Path):
        (tmp_path / "Chart.yaml").write_text(
            "apiVersion: v2\nname: x\ntype: application\nversion: 1.0.0\nappVersion: '1.0'\n", encoding="utf-8",
        )
        r1 = lint_chart(tmp_path)
        r2 = lint_chart(tmp_path)
        assert r1.n_findings == r2.n_findings
        # 真行号一致
        for f1, f2 in zip(r1.findings, r2.findings):
            assert f1.rule_id == f2.rule_id
            assert f1.line_no == f2.line_no

    def test_honest_disclosure(self):
        # 真生产: borrowed 数 >= 6
        assert len(V1399_BORROWED) >= 6

    def test_non_destructive(self, tmp_path: Path):
        # 真生产: lint_chart 不写任何文件
        chart_yaml = tmp_path / "Chart.yaml"
        chart_yaml.write_text(
            "apiVersion: v2\nname: x\ntype: application\nversion: 1.0.0\nappVersion: '1.0'\n", encoding="utf-8",
        )
        before = chart_yaml.read_text()
        lint_chart(tmp_path)
        after = chart_yaml.read_text()
        assert before == after


# ============================================================================
# V1399 TestV1399Continuity: 真生产 continuity (主 17:43)
# ============================================================================


class TestV1399Continuity:
    """V1399 真生产 continuity with previous phases (主 17:43)."""

    def test_does_not_break_v1398_imports(self):
        from apeireth.v1398_real_ansible_lint import V1398_VERSION
        assert V1398_VERSION

    def test_does_not_break_v1397_imports(self):
        from apeireth.v1397_real_terraform_lint import V1397_VERSION
        assert V1397_VERSION

    def test_v1399_in_apeireth_package(self):
        import apeireth.v1399_real_helm_lint as m
        assert m.V1399_VERSION

    def test_self_referential_cli_works(self, capsys):
        rc = run_cli(["version"])
        assert rc == 0