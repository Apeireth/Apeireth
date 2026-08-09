"""Test suite for V1407 ASI 真生产 (Production) framework v1.

Target: 110+ tests covering constants, capacities, limits, trajectory, rules,
borrowed, coherence, chain delegate (real V1400-V1406 invocation),
popper self-test, report, CLI, compose, deploy-check, northstar, format.

主 17:43 实事求是: 真生产真调真测; 主 00:36 质量工程化 popper + 4 exit codes;
主 17:58 不假装 Phenomenal; 主 20:46 不假装达到 ASI;
honest 0.90 cap preserved (V1256 LOCKED).
"""
from __future__ import annotations

import json
import os
import subprocess
import sys

import pytest

# Ensure apeireth package on path
_HERE = os.path.dirname(os.path.abspath(__file__))
_PROMETHEAN_ROOT = os.path.dirname(os.path.dirname(_HERE))
_APEIRETH_DIR = os.path.dirname(_HERE)
if _APEIRETH_DIR not in sys.path:
    sys.path.insert(0, _APEIRETH_DIR)
if _PROMETHEAN_ROOT not in sys.path:
    sys.path.insert(0, _PROMETHEAN_ROOT)

import v1407_asi_production_framework as v1407


# ----------------------- TestV1407Constants -----------------------

class TestV1407Constants:
    def test_version_format(self):
        parts = v1407.V1407_VERSION.split(".")
        assert len(parts) == 3, "version must be semver X.Y.Z"

    def test_module_name(self):
        assert v1407.V1407_MODULE == "v1407_asi_production_framework"

    def test_guards_count(self):
        assert len(v1407.V1407_GUARDS) == 15

    def test_v3_guards_count(self):
        assert len(v1407.V1407_V3_GUARDS) == 6

    def test_v3_guards_unique(self):
        assert len(set(v1407.V1407_V3_GUARDS)) == 6

    def test_rules_count(self):
        assert len(v1407.V1407_RULES) == 12

    def test_rules_unique(self):
        assert len(set(r[0] for r in v1407.V1407_RULES)) == 12

    def test_rules_triple_structure(self):
        for r in v1407.V1407_RULES:
            assert len(r) == 3
            assert r[1] in ("info", "warn", "error")

    def test_borrowed_count(self):
        assert len(v1407.V1407_BORROWED) == 7


# ----------------------- TestV1407Capacities -----------------------

class TestV1407Capacities:
    def test_capacities_count(self):
        assert len(v1407.build_capacities()) == 12

    def test_capacities_unique_ids(self):
        caps = v1407.build_capacities()
        ids = [c.cap_id for c in caps]
        assert len(set(ids)) == 12

    def test_capacities_all_have_evidence(self):
        for c in v1407.build_capacities():
            assert c.evidence, f"{c.cap_id} missing evidence"
            assert len(c.evidence) > 20

    def test_capacities_all_have_borrowed(self):
        for c in v1407.build_capacities():
            assert c.borrowed_from, f"{c.cap_id} missing borrowed_from"

    def test_capacity_required_ids(self):
        required = [
            "CAP_PRODUCTION_LINEAGE", "CAP_PRODUCTION_TRAJECTORY",
            "CAP_PRODUCTION_COMPOSE", "CAP_PRODUCTION_DELEGATE",
            "CAP_PRODUCTION_LEVEL", "CAP_PRODUCTION_VERDICT",
            "CAP_PRODUCTION_GUARD", "CAP_PRODUCTION_NORTHSTAR",
            "CAP_PRODUCTION_CROSS_DOMAIN", "CAP_PRODUCTION_EVIDENCE",
            "CAP_PRODUCTION_BORROW", "CAP_PRODUCTION_HONEST",
        ]
        ids = {c.cap_id for c in v1407.build_capacities()}
        for r in required:
            assert r in ids, f"missing capacity {r}"


# ----------------------- TestV1407Limits -----------------------

class TestV1407Limits:
    def test_limits_count(self):
        assert len(v1407.build_limits()) == 6

    def test_limits_unique_ids(self):
        lims = v1407.build_limits()
        ids = [l.lim_id for l in lims]
        assert len(set(ids)) == 6

    def test_limits_have_phenomenal_explanation(self):
        for lim in v1407.build_limits():
            assert lim.why_no_phenomenal, f"{lim.lim_id} missing explanation"

    def test_limit_required_ids(self):
        required = [
            "LIM_NOT_PHENOMENAL_PRODUCTION", "LIM_NOT_ASI_PRODUCTION",
            "LIM_NOT_HUMAN_LEVEL_PRODUCTION", "LIM_NOT_SELF_HEALING_PRODUCTION",
            "LIM_NOT_AUTONOMOUS_PRODUCTION", "LIM_NOT_NORTHSTAR_REP_PRODUCTION",
        ]
        ids = {l.lim_id for l in v1407.build_limits()}
        for r in required:
            assert r in ids, f"missing limit {r}"

    def test_v3_guard_match_limits(self):
        lims = {l.lim_id for l in v1407.build_limits()}
        # LIM_NOT_*_PRODUCTION should have corresponding V3 GUARD
        v3 = set(v1407.V1407_V3_GUARDS)
        assert "GUARD_PRODUCTION_IS_NOT_PHENOMENAL_PRODUCTION" in v3
        assert "GUARD_PRODUCTION_IS_NOT_ASI" in v3


# ----------------------- TestV1407Trajectory -----------------------

class TestV1407Trajectory:
    def test_trajectory_count(self):
        assert len(v1407.build_trajectory()) >= 25

    def test_trajectory_present_present(self):
        traj = v1407.build_trajectory()
        present = [t for t in traj if t.status == "present"]
        assert len(present) >= 1

    def test_trajectory_present_is_v1407(self):
        traj = v1407.build_trajectory()
        present = [t for t in traj if t.status == "present"]
        assert any(t.version == "V1407" for t in present)

    def test_trajectory_has_anchor(self):
        traj = v1407.build_trajectory()
        anchors = [t for t in traj if t.status == "anchor"]
        assert any(t.version == "V1256" for t in anchors)

    def test_trajectory_has_future(self):
        traj = v1407.build_trajectory()
        futures = [t for t in traj if t.status == "future"]
        assert len(futures) >= 1

    def test_trajectory_has_8_framework_chain(self):
        traj = v1407.build_trajectory()
        fws = [t.version for t in traj if t.kind == "framework"]
        for fw in ["V1400", "V1401", "V1402", "V1403", "V1404", "V1405",
                   "V1406", "V1407"]:
            assert fw in fws, f"missing framework {fw}"


# ----------------------- TestV1407Borrowed -----------------------

class TestV1407Borrowed:
    def test_borrowed_required_keys(self):
        for b in v1407.build_borrowed():
            assert "key" in b
            assert "use" in b
            assert "applied_to" in b

    def test_borrowed_required_sources(self):
        keys = {b["key"] for b in v1407.build_borrowed()}
        for required in ["12factor_2011_webapp", "kubernetes_2014_patterns",
                         "gitops_2017_weaveworks", "sre_2016_google_beyer",
                         "observability_2017_observable",
                         "iac_2018_hashicorp_terraform",
                         "chaos_2011_netflix_principles"]:
            assert required in keys, f"missing borrowed {required}"


# ----------------------- TestV1407Coherence -----------------------

class TestV1407Coherence:
    def test_coherence_count(self):
        checks = v1407.coherence_check()
        assert len(checks) == 12

    def test_coherence_all_pass(self):
        checks = v1407.coherence_check()
        for c in checks:
            assert c.passes, f"coherence fail: {c.pair}"

    def test_coherence_pairs_unique(self):
        checks = v1407.coherence_check()
        pairs = [c.pair for c in checks]
        assert len(set(pairs)) == 12

    def test_coherence_pair_structure(self):
        for c in v1407.coherence_check():
            assert len(c.pair) == 2
            assert c.reason


# ----------------------- TestV1407ChainDelegate -----------------------

class TestV1407ChainDelegate:
    def test_chain_schema(self):
        cd = v1407.chain_delegate()
        assert "v1407" in cd.schema
        assert "production" in cd.schema

    def test_chain_all_ok(self):
        cd = v1407.chain_delegate()
        assert cd.all_ok is True

    def test_chain_total_capacities(self):
        cd = v1407.chain_delegate()
        assert cd.total_capacities == 84  # 7 frameworks × 12 cap

    def test_chain_total_limits(self):
        cd = v1407.chain_delegate()
        assert cd.total_limits == 42  # 7 frameworks × 6 lim

    def test_chain_7_frameworks(self):
        cd = v1407.chain_delegate()
        assert len(cd.delegated) == 7

    def test_chain_frameworks_v1400_to_v1406(self):
        cd = v1407.chain_delegate()
        fws = [d["framework"] for d in cd.delegated]
        for fw in ["V1400", "V1401", "V1402", "V1403", "V1404", "V1405",
                   "V1406"]:
            assert fw in fws, f"missing {fw}"

    def test_chain_real_invocation(self):
        cd = v1407.chain_delegate()
        # Each delegation should have a non-None result_type
        for d in cd.delegated:
            assert d["ok"] is True
            assert d["result_type"] != "None"


# ----------------------- TestV1407Popper -----------------------

class TestV1407Popper:
    def test_popper_7_pass(self):
        pop = v1407.popper_self_test()
        assert pop["pass_count"] == 7
        assert pop["total_count"] == 7

    def test_popper_required_keys(self):
        pop = v1407.popper_self_test()
        required = [
            "artifact_declared", "level_declared", "verdict_anchored",
            "northstar_locked", "chain_delegate_real", "delegated_7_frameworks",
            "honest_disclosure",
        ]
        for k in required:
            assert pop[k] is True, f"popper {k} not True"

    def test_popper_all_pass(self):
        pop = v1407.popper_self_test()
        assert pop["all_pass"] is True


# ----------------------- TestV1407NorthStar -----------------------

class TestV1407NorthStar:
    def test_northstar_anchor_v1256(self):
        ns = v1407.build_northstar_alignment()
        assert ns["anchor_version"] == "V1256"

    def test_northstar_ceiling(self):
        ns = v1407.build_northstar_alignment()
        assert ns["ceiling"] == 0.9105

    def test_northstar_locked(self):
        ns = v1407.build_northstar_alignment()
        assert ns["locked"] is True

    def test_northstar_honest_cap(self):
        ns = v1407.build_northstar_alignment()
        assert ns["honest_cap"] == 0.90

    def test_northstar_8_frameworks(self):
        ns = v1407.build_northstar_alignment()
        assert len(ns["frameworks_chain"]) == 8

    def test_northstar_philosophy_complete(self):
        ns = v1407.build_northstar_alignment()
        assert ns["philosophy_complete"] is True


# ----------------------- TestV1407Report -----------------------

class TestV1407Report:
    def test_report_module(self):
        r = v1407.run_self_production()
        assert r.module == "v1407_asi_production_framework"

    def test_report_version(self):
        r = v1407.run_self_production()
        assert r.version == "0.1.0"

    def test_report_generated_at_iso(self):
        r = v1407.run_self_production()
        assert "T" in r.generated_at_iso

    def test_report_capacities_count(self):
        r = v1407.run_self_production()
        assert len(r.capacities) == 12

    def test_report_limits_count(self):
        r = v1407.run_self_production()
        assert len(r.limits) == 6

    def test_report_asi_7_complete(self):
        r = v1407.run_self_production()
        assert r.asi_7_philosophy_complete is True

    def test_report_8_production_levels(self):
        r = v1407.run_self_production()
        assert len(r.production_levels) == 8
        assert r.production_levels[0] == "L0_DATA"
        assert r.production_levels[-1] == "L7_PRODUCTION"


# ----------------------- TestV1407Compose -----------------------

class TestV1407Compose:
    def test_compose_is_string(self):
        c = v1407.generate_docker_compose()
        assert isinstance(c, str)
        assert len(c) > 1000

    def test_compose_has_8_services(self):
        c = v1407.generate_docker_compose()
        for svc in ["postgres:", "redis:", "prometheus:", "grafana:",
                    "apeireth-self:", "apeireth-judge:",
                    "apeireth-explainer:", "apeireth-trace:"]:
            assert svc in c, f"missing service {svc}"

    def test_compose_has_volumes(self):
        c = v1407.generate_docker_compose()
        assert "volumes:" in c
        for vol in ["apeireth_pg_data:", "apeireth_redis_data:",
                    "apeireth_prom_data:", "apeireth_grafana_data:"]:
            assert vol in c

    def test_compose_has_healthchecks(self):
        c = v1407.generate_docker_compose()
        # Should have multiple healthcheck blocks
        assert c.count("healthcheck:") >= 8

    def test_compose_yaml_parseable(self):
        try:
            import yaml
        except ImportError:
            pytest.skip("PyYAML not installed")
        c = v1407.generate_docker_compose()
        parsed = yaml.safe_load(c)
        assert "services" in parsed
        assert "volumes" in parsed
        assert len(parsed["services"]) == 8

    def test_compose_services_have_image_or_build(self):
        try:
            import yaml
        except ImportError:
            pytest.skip("PyYAML not installed")
        c = v1407.generate_docker_compose()
        parsed = yaml.safe_load(c)
        for name, svc in parsed["services"].items():
            assert ("image" in svc or "build" in svc), \
                f"{name} missing image/build"


# ----------------------- TestV1407CLI -----------------------

class TestV1407CLI:
    def test_cli_version(self, capsys):
        rc = v1407.run_cli(["version"])
        assert rc == 0
        out = capsys.readouterr().out
        assert "V1407" in out

    def test_cli_help(self, capsys):
        rc = v1407.run_cli(["help"])
        assert rc == 0
        out = capsys.readouterr().out
        assert "v1407" in out.lower() or "production" in out.lower()

    def test_cli_demo(self, capsys):
        rc = v1407.run_cli(["demo"])
        assert rc == 0
        out = capsys.readouterr().out
        assert "capacities: 12" in out
        assert "limits: 6" in out

    def test_cli_chain(self, capsys):
        rc = v1407.run_cli(["chain"])
        assert rc == 0
        out = capsys.readouterr().out
        assert "V1400" in out
        assert "V1406" in out

    def test_cli_popper(self, capsys):
        rc = v1407.run_cli(["popper"])
        assert rc == 0
        out = capsys.readouterr().out
        assert "all_pass" in out

    def test_cli_capacity(self, capsys):
        rc = v1407.run_cli(["capacity"])
        assert rc == 0
        out = capsys.readouterr().out
        assert "CAP_PRODUCTION_LINEAGE" in out

    def test_cli_limits(self, capsys):
        rc = v1407.run_cli(["limits"])
        assert rc == 0
        out = capsys.readouterr().out
        assert "LIM_NOT_PHENOMENAL_PRODUCTION" in out

    def test_cli_trajectory(self, capsys):
        rc = v1407.run_cli(["trajectory"])
        assert rc == 0
        out = capsys.readouterr().out
        assert "V1256" in out

    def test_cli_rules(self, capsys):
        rc = v1407.run_cli(["rules"])
        assert rc == 0
        out = capsys.readouterr().out
        assert "PRD001" in out

    def test_cli_compose_stdout(self, capsys):
        rc = v1407.run_cli(["compose"])
        assert rc == 0
        out = capsys.readouterr().out
        assert "version:" in out
        assert "services:" in out

    def test_cli_compose_to_file(self, tmp_path, capsys):
        out_file = tmp_path / "docker-compose.yml"
        rc = v1407.run_cli(["compose", "--compose-out", str(out_file)])
        assert rc == 0
        assert out_file.exists()
        content = out_file.read_text(encoding="utf-8")
        # Must have actual YAML "services:" key (not just in header comment)
        assert content.count("\nservices:\n") == 1

    def test_cli_deploy_check(self, capsys):
        rc = v1407.run_cli(["deploy-check"])
        out = capsys.readouterr().out
        if "PyYAML not installed" in out:
            pytest.skip("PyYAML not installed")
        assert rc == 0
        assert "services: 8" in out

    def test_cli_production_report_text(self, capsys):
        rc = v1407.run_cli(["production-report"])
        assert rc == 0
        out = capsys.readouterr().out
        assert "Production Levels" in out
        assert "Capacities" in out

    def test_cli_production_report_json(self, capsys):
        rc = v1407.run_cli(["production-report", "--json"])
        assert rc == 0
        out = capsys.readouterr().out
        # Should be valid JSON
        parsed = json.loads(out)
        assert parsed["module"] == "v1407_asi_production_framework"

    def test_cli_production_report_md(self, capsys):
        rc = v1407.run_cli(["production-report", "--format", "md"])
        assert rc == 0
        out = capsys.readouterr().out
        assert "# V1407" in out


# ----------------------- TestV1407V3Guards -----------------------

class TestV1407V3Guards:
    def test_v3_guards_complete(self):
        required = [
            "GUARD_PRODUCTION_IS_NOT_PHENOMENAL_PRODUCTION",
            "GUARD_PRODUCTION_IS_NOT_ASI",
            "GUARD_PRODUCTION_IS_NOT_HUMAN_LEVEL",
            "GUARD_PRODUCTION_IS_NOT_SELF_HEALING",
            "GUARD_PRODUCTION_IS_NOT_AUTONOMOUS",
            "GUARD_PRODUCTION_IS_NOT_NORTHSTAR_REP",
        ]
        for g in required:
            assert g in v1407.V1407_V3_GUARDS

    def test_v3_guard_substrings(self):
        # All V3 guards should reference "PRODUCTION" in name
        for g in v1407.V1407_V3_GUARDS:
            assert "PRODUCTION" in g


# ----------------------- TestV1407Format -----------------------

class TestV1407Format:
    def test_format_text_returns_string(self):
        r = v1407.run_self_production()
        text = v1407._format_text(r)
        assert isinstance(text, str)
        assert "V1407" in text

    def test_format_json_returns_valid(self):
        r = v1407.run_self_production()
        js = v1407._format_json(r)
        parsed = json.loads(js)
        assert "module" in parsed
        assert "capacities" in parsed
        assert len(parsed["capacities"]) == 12

    def test_format_md_returns_string(self):
        r = v1407.run_self_production()
        md = v1407._format_md(r)
        assert isinstance(md, str)
        assert "V1407" in md


# ----------------------- TestV1407Deterministic -----------------------

class TestV1407Deterministic:
    def test_build_capacities_idempotent(self):
        a = v1407.build_capacities()
        b = v1407.build_capacities()
        assert len(a) == len(b)
        for x, y in zip(a, b):
            assert x.cap_id == y.cap_id

    def test_build_limits_idempotent(self):
        a = v1407.build_limits()
        b = v1407.build_limits()
        assert len(a) == len(b)
        for x, y in zip(a, b):
            assert x.lim_id == y.lim_id

    def test_chain_delegate_idempotent(self):
        a = v1407.chain_delegate()
        b = v1407.chain_delegate()
        assert a.all_ok == b.all_ok
        assert a.total_capacities == b.total_capacities

    def test_compose_idempotent(self):
        a = v1407.generate_docker_compose()
        b = v1407.generate_docker_compose()
        assert a == b


# ----------------------- TestV1407Continuity -----------------------

class TestV1407Continuity:
    def test_v1407_inherits_v1406_judge(self):
        # V1406 should still be importable and run
        import v1406_asi_judge_framework as v1406
        r1406 = v1406.run_self_judge()
        assert r1406 is not None

    def test_v1407_chain_includes_v1406(self):
        cd = v1407.chain_delegate()
        v1406_d = [d for d in cd.delegated if d["framework"] == "V1406"]
        assert len(v1406_d) == 1
        assert v1406_d[0]["ok"] is True

    def test_v1407_builds_on_v1400_v1406_chain(self):
        # Verify chain_delegate reaches V1400 (start of chain)
        cd = v1407.chain_delegate()
        v1400_d = [d for d in cd.delegated if d["framework"] == "V1400"]
        assert len(v1400_d) == 1
        assert v1400_d[0]["ok"] is True


# ----------------------- TestV1407Subprocess -----------------------

class TestV1407Subprocess:
    """End-to-end test: run V1407 as subprocess like a real operator would."""

    def test_subprocess_version(self):
        result = subprocess.run(
            [sys.executable, "-m", "v1407_asi_production_framework", "version"],
            capture_output=True, text=True, timeout=30,
            cwd=os.path.dirname(_HERE),
        )
        assert result.returncode == 0
        assert "V1407" in result.stdout

    def test_subprocess_demo(self):
        result = subprocess.run(
            [sys.executable, "-m", "v1407_asi_production_framework", "demo"],
            capture_output=True, text=True, timeout=30,
            cwd=os.path.dirname(_HERE),
        )
        assert result.returncode == 0
        assert "capacities: 12" in result.stdout

    def test_subprocess_chain(self):
        result = subprocess.run(
            [sys.executable, "-m", "v1407_asi_production_framework", "chain"],
            capture_output=True, text=True, timeout=30,
            cwd=os.path.dirname(_HERE),
        )
        assert result.returncode == 0
        assert "V1400" in result.stdout
        assert "V1406" in result.stdout

    def test_subprocess_deploy_check(self):
        result = subprocess.run(
            [sys.executable, "-m", "v1407_asi_production_framework",
             "deploy-check"],
            capture_output=True, text=True, timeout=30,
            cwd=os.path.dirname(_HERE),
        )
        if "PyYAML not installed" in result.stdout:
            pytest.skip("PyYAML not installed")
        assert result.returncode == 0
        assert "services: 8" in result.stdout

    def test_subprocess_popper(self):
        result = subprocess.run(
            [sys.executable, "-m", "v1407_asi_production_framework",
             "popper"],
            capture_output=True, text=True, timeout=30,
            cwd=os.path.dirname(_HERE),
        )
        assert result.returncode == 0
        assert "all_pass" in result.stdout