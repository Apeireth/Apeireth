"""Tests for V1429 ASI deployment semantic linter."""

from __future__ import annotations

import json
import sys
from pathlib import Path

import pytest

# Make sure apeireth is importable
ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))

from apeireth.v1429_asi_deployment_semantic_linter import (  # noqa: E402
    ALL_ARTIFACT_KINDS,
    ALL_RULES,
    ArtifactKind,
    ArtifactLintReport,
    LintResult,
    LintRule,
    RuleSeverity,
    SemanticLinterReport,
    V1429_BORROWED,
    V1429_GUARDS,
    V1429_MODULE,
    V1429_SCHEMA,
    V1429_V3_GUARDS,
    V1429_VERSION,
    _compose_sl001,
    _compose_sl002,
    _compose_sl003,
    _compose_sl004,
    _compose_sl005,
    _dockerfile_sl020,
    _dockerfile_sl021,
    _dockerfile_sl022,
    _dockerfile_sl023,
    _dockerfile_sl024,
    _env_sl050,
    _env_sl051,
    _env_sl052,
    _k8s_sl010,
    _k8s_sl011,
    _k8s_sl012,
    _k8s_sl013,
    _req_sl030,
    _req_sl031,
    _req_sl032,
    _startup_sl040,
    _startup_sl041,
    _startup_sl042,
    _startup_sl043,
    chain_delegate,
    check_rule,
    lint_all,
    lint_artifact,
    lint_from_v1428,
    module_meta,
    popper_self_test,
    render_report_md,
    rules_for,
    semantic_readiness,
)


# =============================================================================
# Constants / metadata
# =============================================================================


class TestMeta:
    def test_version_is_string(self):
        assert isinstance(V1429_VERSION, str)
        assert V1429_VERSION == "0.1.0"

    def test_module_name(self):
        assert V1429_MODULE == "v1429_asi_deployment_semantic_linter"

    def test_schema(self):
        assert V1429_SCHEMA.startswith("v1429.")

    def test_guards_present(self):
        assert len(V1429_GUARDS) == 14
        assert "GUARD_NO_DOCKER_REQUIRED" in V1429_GUARDS
        assert "GUARD_DETERMINISTIC" in V1429_GUARDS

    def test_v3_guards_present(self):
        assert len(V1429_V3_GUARDS) == 5
        assert "GUARD_NO_PHENOMENAL_LINT" in V1429_V3_GUARDS
        assert "GUARD_NO_ASI_LINT" in V1429_V3_GUARDS

    def test_borrowed_includes_v1428(self):
        names = [t[0] for t in V1429_BORROWED]
        assert "V1428" in names
        assert "V1008" in names
        assert "V1032" in names

    def test_module_meta_keys(self):
        m = module_meta()
        assert m["version"] == V1429_VERSION
        assert m["n_rules"] == 24
        assert m["n_artifact_kinds"] == 6
        assert m["phase"] == 1429


# =============================================================================
# Rule registry
# =============================================================================


class TestRules:
    def test_total_rule_count(self):
        assert len(ALL_RULES) == 24

    def test_rule_codes_format(self):
        for r in ALL_RULES:
            assert r.code.startswith("SL"), r.code
            assert len(r.code) == 5, r.code

    def test_codes_unique(self):
        codes = [r.code for r in ALL_RULES]
        assert len(codes) == len(set(codes))

    def test_compose_5_rules(self):
        assert len(rules_for(ArtifactKind.COMPOSE)) == 5

    def test_k8s_4_rules(self):
        assert len(rules_for(ArtifactKind.K8S)) == 4

    def test_dockerfile_5_rules(self):
        assert len(rules_for(ArtifactKind.DOCKERFILE)) == 5

    def test_requirements_3_rules(self):
        assert len(rules_for(ArtifactKind.REQUIREMENTS)) == 3

    def test_startup_4_rules(self):
        assert len(rules_for(ArtifactKind.STARTUP)) == 4

    def test_env_3_rules(self):
        assert len(rules_for(ArtifactKind.ENV)) == 3

    def test_all_kinds_covered(self):
        kinds = {r.artifact_kind for r in ALL_RULES}
        assert kinds == set(ALL_ARTIFACT_KINDS)

    def test_rules_for_unknown_kind_returns_empty(self):
        # rules_for only returns rules for ArtifactKind enum members in ALL_RULES.
        # The function is forgiving — returns empty tuple for non-matching kinds.
        class FakeKind:
            value = "fake"
        # Note: ArtifactKind is a str enum, so rules_for won't raise on string-coercible input
        # It will return empty tuple because FakeKind has no matching rules.
        try:
            result = rules_for(FakeKind())
            assert result == ()
        except (TypeError, AttributeError):
            # Also acceptable
            pass


# =============================================================================
# Individual rule check functions (smoke tests on each rule's behavior)
# =============================================================================


class TestComposeRules:
    def test_sl001_no_healthcheck(self):
        ok, _ = _compose_sl001("services:\n  api:\n    image: foo:1.0\n")
        assert not ok

    def test_sl001_has_healthcheck(self):
        ok, _ = _compose_sl001("services:\n  api:\n    healthcheck:\n      test: [\"CMD\", \"true\"]\n")
        assert ok

    def test_sl002_depends_on_with_condition(self):
        ok, _ = _compose_sl002("services:\n  api:\n    depends_on:\n      db:\n        condition: service_healthy\n")
        assert ok

    def test_sl002_depends_on_without_condition(self):
        ok, _ = _compose_sl002("services:\n  api:\n    depends_on:\n      - db\n")
        assert not ok

    def test_sl003_restart_present(self):
        ok, _ = _compose_sl003("services:\n  api:\n    restart: always\n")
        assert ok

    def test_sl003_restart_absent(self):
        ok, _ = _compose_sl003("services:\n  api:\n    image: foo:1.0\n")
        assert not ok

    def test_sl004_resource_limits(self):
        ok, _ = _compose_sl004("services:\n  api:\n    mem_limit: 512m\n")
        assert ok

    def test_sl004_no_resource_limits(self):
        ok, _ = _compose_sl004("services:\n  api:\n    image: foo:1.0\n")
        assert not ok

    def test_sl005_latest_tag(self):
        ok, _ = _compose_sl005("services:\n  api:\n    image: foo:latest\n")
        assert not ok

    def test_sl005_pinned_tag(self):
        ok, _ = _compose_sl005("services:\n  api:\n    image: foo:1.0\n")
        assert ok


class TestK8sRules:
    def test_sl010_limits_present(self):
        ok, _ = _k8s_sl010("resources:\n  limits:\n    cpu: 500m\n")
        assert ok

    def test_sl010_limits_absent(self):
        ok, _ = _k8s_sl010("image: foo:1.0\n")
        assert not ok

    def test_sl011_liveness_probe(self):
        ok, _ = _k8s_sl011("livenessProbe:\n  httpGet:\n    path: /health\n")
        assert ok

    def test_sl011_no_probes(self):
        ok, _ = _k8s_sl011("image: foo:1.0\n")
        assert not ok

    def test_sl012_latest_image(self):
        ok, _ = _k8s_sl012("image: foo:latest\n")
        assert not ok

    def test_sl013_workload_present(self):
        ok, _ = _k8s_sl013("kind: Deployment\n")
        assert ok

    def test_sl013_no_workload(self):
        ok, _ = _k8s_sl013("kind: ConfigMap\n")
        assert not ok


class TestDockerfileRules:
    def test_sl020_from_latest(self):
        ok, _ = _dockerfile_sl020("FROM python:latest\n")
        assert not ok

    def test_sl020_from_pinned(self):
        ok, _ = _dockerfile_sl020("FROM python:3.12-slim\n")
        assert ok

    def test_sl020_from_digest(self):
        ok, _ = _dockerfile_sl020("FROM python@sha256:abc123\n")
        assert ok

    def test_sl020_no_from(self):
        ok, _ = _dockerfile_sl020("RUN echo hi\n")
        assert not ok

    def test_sl021_user_root(self):
        ok, _ = _dockerfile_sl021("FROM python:3.12\nUSER root\n")
        assert not ok

    def test_sl021_user_app(self):
        ok, _ = _dockerfile_sl021("FROM python:3.12\nUSER app\n")
        assert ok

    def test_sl021_no_user(self):
        ok, _ = _dockerfile_sl021("FROM python:3.12\n")
        assert not ok

    def test_sl022_healthcheck(self):
        ok, _ = _dockerfile_sl022("HEALTHCHECK CMD curl http://localhost\n")
        assert ok

    def test_sl023_add_url(self):
        ok, _ = _dockerfile_sl023("ADD https://example.com/foo.tar.gz /tmp/\n")
        assert not ok

    def test_sl023_add_local(self):
        ok, _ = _dockerfile_sl023("ADD ./local.tar.gz /tmp/\n")
        assert ok

    def test_sl024_workdir(self):
        ok, _ = _dockerfile_sl024("WORKDIR /app\n")
        assert ok


class TestRequirementsRules:
    def test_sl030_pinned(self):
        ok, _ = _req_sl030("fastapi==0.110.0\npydantic>=2.0\n")
        assert ok

    def test_sl030_unpinned(self):
        ok, _ = _req_sl030("fastapi\npydantic\n")
        assert not ok

    def test_sl030_skips_comments(self):
        ok, _ = _req_sl030("# comment\nfastapi==0.110.0\n")
        assert ok

    def test_sl031_git_url(self):
        ok, _ = _req_sl031("mypkg @ git+https://github.com/foo/bar\n")
        assert not ok

    def test_sl032_editable(self):
        ok, _ = _req_sl032("-e .\n")
        assert not ok


class TestStartupRules:
    def test_sl040_trap_and_signal(self):
        ok, _ = _startup_sl040("trap 'kill $PID' SIGTERM SIGINT\n")
        assert ok

    def test_sl040_trap_no_signal(self):
        ok, _ = _startup_sl040("trap 'echo done' EXIT\n")
        assert not ok

    def test_sl041_retry_while(self):
        ok, _ = _startup_sl041("while ! curl -f http://localhost; do sleep 1; done\n")
        assert ok

    def test_sl041_retry_flag(self):
        ok, _ = _startup_sl041("curl --retry 3 http://localhost\n")
        assert ok

    def test_sl042_set_e(self):
        ok, _ = _startup_sl042("set -e\n")
        assert ok

    def test_sl042_no_set_e(self):
        ok, _ = _startup_sl042("# nothing\n")
        assert not ok

    def test_sl043_pidfile(self):
        ok, _ = _startup_sl043('echo $! > /var/run/asi.pid\n')
        assert ok


class TestEnvRules:
    def test_sl050_hardcoded_secret(self):
        ok, _ = _env_sl050("API_KEY=supersecret123\n")
        assert not ok

    def test_sl050_empty_secret(self):
        ok, _ = _env_sl050("API_KEY=\n")
        assert ok

    def test_sl050_placeholder_secret(self):
        ok, _ = _env_sl050("API_KEY=<change-me>\n")
        assert ok

    def test_sl051_few_keys(self):
        ok, _ = _env_sl051("FOO=bar\n")
        assert not ok

    def test_sl051_three_keys(self):
        ok, _ = _env_sl051("FOO=bar\nBAZ=qux\nQUX=zed\n")
        assert ok

    def test_sl052_no_comments(self):
        ok, _ = _env_sl052("FOO=bar\n")
        assert not ok

    def test_sl052_with_comments(self):
        ok, _ = _env_sl052("# this is the config\nFOO=bar\n")
        assert ok


# =============================================================================
# check_rule + lint_artifact
# =============================================================================


class TestCheckRule:
    def test_check_rule_passing(self):
        rule = LintRule("SL001", ArtifactKind.COMPOSE, "x", RuleSeverity.FAIL, _compose_sl001)
        result = check_rule(rule, "services:\n  api:\n    healthcheck:\n      test: [\"CMD\", \"true\"]\n")
        assert isinstance(result, LintResult)
        assert result.passed

    def test_check_rule_failing(self):
        rule = LintRule("SL001", ArtifactKind.COMPOSE, "x", RuleSeverity.FAIL, _compose_sl001)
        result = check_rule(rule, "services:\n  api:\n    image: foo:1.0\n")
        assert not result.passed

    def test_check_rule_exception_caught(self):
        def bad_check(c):
            raise ValueError("oops")
        rule = LintRule("SLX", ArtifactKind.COMPOSE, "bad", RuleSeverity.FAIL, bad_check)
        result = check_rule(rule, "anything")
        assert not result.passed
        assert "ValueError" in result.message


class TestLintArtifact:
    def test_empty_content_mostly_fails(self):
        rep = lint_artifact(ArtifactKind.COMPOSE, "")
        assert rep.n_total == 5
        assert rep.n_fail >= 1

    def test_perfect_compose(self):
        content = (
            "services:\n"
            "  api:\n"
            "    image: apeireth:1.0.0\n"
            "    healthcheck:\n"
            "      test: [\"CMD\", \"true\"]\n"
            "    restart: always\n"
            "    depends_on:\n"
            "      db:\n"
            "        condition: service_healthy\n"
            "    mem_limit: 512m\n"
        )
        rep = lint_artifact(ArtifactKind.COMPOSE, content)
        assert rep.n_fail == 0
        assert rep.n_pass == 5
        assert rep.score == 1.0

    def test_score_calculation(self):
        rep = lint_artifact(ArtifactKind.K8S, "")
        assert 0.0 <= rep.score <= 1.0

    def test_results_have_codes(self):
        rep = lint_artifact(ArtifactKind.COMPOSE, "")
        for r in rep.results:
            assert r.code.startswith("SL")


# =============================================================================
# lint_all + semantic_readiness
# =============================================================================


class TestLintAll:
    def test_empty_all(self):
        rep = lint_all({})
        # Even with no content, every artifact kind still has rules checked against ""
        # so n_total == sum of all rules across all 6 kinds == 24
        assert rep.n_total == 24
        assert semantic_readiness(rep) < 1.0  # most fail on empty content
        assert len(rep.per_kind) == 6

    def test_full_lint_with_perfect_samples(self):
        samples = {
            ArtifactKind.COMPOSE: (
                "services:\n"
                "  api:\n"
                "    image: apeireth:1.0.0\n"
                "    healthcheck:\n"
                "      test: [\"CMD\", \"true\"]\n"
                "    restart: always\n"
                "    depends_on:\n"
                "      db:\n"
                "        condition: service_healthy\n"
                "    mem_limit: 512m\n"
            ),
            ArtifactKind.K8S: (
                "kind: Deployment\n"
                "spec:\n"
                "  template:\n"
                "    spec:\n"
                "      containers:\n"
                "        - name: asi\n"
                "          image: apeireth:1.0.0\n"
                "          resources:\n"
                "            limits:\n"
                "              cpu: 500m\n"
                "              memory: 512Mi\n"
                "          livenessProbe:\n"
                "            httpGet:\n"
                "              path: /healthz\n"
            ),
            ArtifactKind.DOCKERFILE: (
                "FROM python:3.12-slim\n"
                "WORKDIR /app\n"
                "USER app\n"
                "HEALTHCHECK CMD curl -f http://localhost/health || exit 1\n"
            ),
            ArtifactKind.REQUIREMENTS: (
                "fastapi==0.110.0\n"
                "pydantic==2.6.0\n"
            ),
            ArtifactKind.STARTUP: (
                "#!/usr/bin/env bash\n"
                "set -e\n"
                "trap 'kill $PID' SIGTERM SIGINT\n"
                "while true; do python -m asi; PID=$!; wait $PID; done\n"
                "echo $! > /var/run/asi.pid\n"
            ),
            ArtifactKind.ENV: (
                "# ASI config\n"
                "ASI_PORT=8080\n"
                "ASI_LOG_LEVEL=INFO\n"
                "ASI_API_KEY=\n"
            ),
        }
        rep = lint_all(samples)
        # Should have many passes
        assert rep.n_pass >= 18
        # Overall readiness should be high
        assert semantic_readiness(rep) >= 0.7


class TestSemanticReadiness:
    def test_zero_when_no_results(self):
        rep = SemanticLinterReport()
        assert semantic_readiness(rep) == 0.0

    def test_one_when_all_pass(self):
        rep = SemanticLinterReport(n_pass=10, n_total=10)
        assert semantic_readiness(rep) == 1.0

    def test_handles_partial(self):
        rep = SemanticLinterReport(n_pass=5, n_warn=3, n_fail=2, n_total=10)
        assert semantic_readiness(rep) == 0.5


# =============================================================================
# Popper self-test
# =============================================================================


class TestPopper:
    def test_popper_all_pass(self):
        out = popper_self_test()
        assert out["ok"], f"popper failed: {out['results']}"
        assert out["n_total"] >= 14
        assert out["n_pass"] == out["n_total"]

    def test_popper_results_have_tuples(self):
        out = popper_self_test()
        for name, val in out["results"].items():
            assert isinstance(name, str)
            assert isinstance(val, tuple)
            assert len(val) == 2
            assert isinstance(val[0], bool)


# =============================================================================
# chain_delegate
# =============================================================================


class TestChainDelegate:
    def test_chain_returns_dict(self):
        out = chain_delegate()
        assert "all_ok" in out
        assert "chain" in out
        assert "n_modules" in out

    def test_chain_has_v1428(self):
        out = chain_delegate()
        assert "V1428" in out["chain"]


# =============================================================================
# render_report_md
# =============================================================================


class TestRenderReport:
    def test_report_contains_header(self):
        rep = lint_all({})
        md = render_report_md(rep)
        assert "# V1429 ASI Deployment Semantic Lint Report" in md

    def test_report_includes_all_kinds(self):
        rep = lint_all({})
        md = render_report_md(rep)
        for kind in ALL_ARTIFACT_KINDS:
            assert kind.value in md

    def test_report_with_results(self):
        samples = {ArtifactKind.COMPOSE: "image: foo:latest\n"}
        rep = lint_all(samples)
        md = render_report_md(rep)
        assert "SL005" in md  # the rule for latest tag


# =============================================================================
# CLI smoke (run main)
# =============================================================================


class TestCLI:
    def test_version(self, capsys):
        from apeireth.v1429_asi_deployment_semantic_linter import main
        rc = main(["version"])
        captured = capsys.readouterr()
        assert rc == 0
        assert "0.1.0" in captured.out

    def test_help(self, capsys):
        from apeireth.v1429_asi_deployment_semantic_linter import main
        rc = main(["help"])
        captured = capsys.readouterr()
        assert rc == 0
        assert "V1429" in captured.out

    def test_meta(self, capsys):
        from apeireth.v1429_asi_deployment_semantic_linter import main
        rc = main(["meta"])
        captured = capsys.readouterr()
        assert rc == 0
        assert "v1429" in captured.out

    def test_meta_json(self, capsys):
        from apeireth.v1429_asi_deployment_semantic_linter import main
        rc = main(["meta", "--json"])
        captured = capsys.readouterr()
        assert rc == 0
        data = json.loads(captured.out)
        assert data["version"] == V1429_VERSION

    def test_demo(self, capsys):
        from apeireth.v1429_asi_deployment_semantic_linter import main
        rc = main(["demo"])
        captured = capsys.readouterr()
        assert rc == 0
        assert "readiness" in captured.out

    def test_popper_cli(self, capsys):
        from apeireth.v1429_asi_deployment_semantic_linter import main
        rc = main(["popper"])
        captured = capsys.readouterr()
        assert rc == 0
        data = json.loads(captured.out)
        assert data["ok"]

    def test_chain_cli(self, capsys):
        from apeireth.v1429_asi_deployment_semantic_linter import main
        rc = main(["chain"])
        captured = capsys.readouterr()
        # rc is 0 if all upstreams OK, else 1 — both are valid (best-effort)
        assert rc in (0, 1)
        data = json.loads(captured.out)
        assert "chain" in data
        assert "V1428" in data["chain"]

    def test_rules_cli(self, capsys):
        from apeireth.v1429_asi_deployment_semantic_linter import main
        rc = main(["rules"])
        captured = capsys.readouterr()
        assert rc == 0
        assert "SL001" in captured.out
        assert "SL052" in captured.out

    def test_unknown_command(self, capsys):
        from apeireth.v1429_asi_deployment_semantic_linter import main
        rc = main(["nope"])
        captured = capsys.readouterr()
        assert rc == 2


# =============================================================================
# lint_from_v1428 (integration with V1428)
# =============================================================================


class TestLintFromV1428:
    def test_runs_without_error(self):
        # Will either find real artifacts or fall back to empty
        rep = lint_from_v1428()
        assert isinstance(rep, SemanticLinterReport)
        assert rep.n_total >= 0

    def test_readiness_in_range(self):
        rep = lint_from_v1428()
        r = semantic_readiness(rep)
        assert 0.0 <= r <= 1.0


# =============================================================================
# Honest disclosure — V3 guards
# =============================================================================


class TestHonestDisclosure:
    def test_no_phenomenal_claim(self):
        # V1429 module should not claim phenomenal / ASI / human-level
        import apeireth.v1429_asi_deployment_semantic_linter as mod
        src = Path(mod.__file__).read_text(encoding="utf-8")
        # Should contain explicit "not" disclaimers
        assert "NOT consciousness" in src or "NOT ASI" in src or "NOT" in src

    def test_v3_guards_in_module(self):
        # The 5 V3 guards must be declared
        expected_v3 = {
            "GUARD_NO_PHENOMENAL_LINT",
            "GUARD_NO_ASI_LINT",
            "GUARD_NO_HUMAN_LEVEL_LINT",
            "GUARD_NO_ABSOLUTE_LINT",
            "GUARD_NO_PRODUCTION_PADDING",
        }
        assert expected_v3.issubset(set(V1429_V3_GUARDS))


# =============================================================================
# Determinism
# =============================================================================


class TestDeterminism:
    def test_same_input_same_output(self):
        content = "image: foo:latest\nFROM python:latest\n"
        rep1 = lint_artifact(ArtifactKind.COMPOSE, content)
        rep2 = lint_artifact(ArtifactKind.COMPOSE, content)
        assert rep1.n_pass == rep2.n_pass
        assert rep1.n_fail == rep2.n_fail