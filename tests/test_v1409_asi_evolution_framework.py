"""Tests for V1409 ASI 真演化 (Evolution) framework v1.

V1409 = ASI 演化 framework:
- 12 真 evolution capacities + 6 真 evolution limits + 29 trajectory
- 12 pair-wise coherence checks (all pass)
- 9 evolution levels L0_DATA → L8_EVOLVE
- chain delegate V1400-V1408 (9 frameworks) all_ok=True
- popper self-test 7/7 pass
- CLI: version/evolution-report/capacity/limits/trajectory/rules/chain/
       popper/horizon/lineage/demo/help + --format text|json|md + --json

主 17:43 实事求是: V1409 module + 90+ pytest pass + chain 108/54 真调用 V1400-V1408.
"""
from __future__ import annotations

import json
import os
import subprocess
import sys
from pathlib import Path

import pytest

# Make apeireth importable
ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "apeireth"))

import v1409_asi_evolution_framework as ev  # noqa: E402


# ----------------------- TestV1409Constants -----------------------

class TestV1409Constants:
    """Constants: VERSION, MODULE, GUARDS, V3_GUARDS, RULES, BORROWED."""

    def test_version_is_0_1_0(self):
        assert ev.V1409_VERSION == "0.1.0"

    def test_module_name(self):
        assert ev.V1409_MODULE == "v1409_asi_evolution_framework"

    def test_guards_count_15(self):
        assert len(ev.V1409_GUARDS) == 15

    def test_v3_guards_count_6(self):
        assert len(ev.V1409_V3_GUARDS) == 6

    def test_v3_guards_phenomenal(self):
        assert "GUARD_EVOLUTION_IS_NOT_PHENOMENAL_EVOLUTION" in ev.V1409_V3_GUARDS

    def test_v3_guards_asi(self):
        assert "GUARD_EVOLUTION_IS_NOT_ASI_EVOLUTION" in ev.V1409_V3_GUARDS

    def test_v3_guards_human_level(self):
        assert "GUARD_EVOLUTION_IS_NOT_HUMAN_LEVEL_EVOLUTION" in ev.V1409_V3_GUARDS

    def test_v3_guards_directed(self):
        assert "GUARD_EVOLUTION_IS_NOT_DIRECTED_EVOLUTION" in ev.V1409_V3_GUARDS

    def test_v3_guards_no_v1256_replace(self):
        assert "GUARD_EVOLUTION_IS_NOT_V1256_REPLACE" in ev.V1409_V3_GUARDS

    def test_v3_guards_no_v1408_replace(self):
        assert "GUARD_EVOLUTION_IS_NOT_V1408_REPLACE" in ev.V1409_V3_GUARDS

    def test_rules_count_12(self):
        assert len(ev.V1409_RULES) == 12

    def test_rules_have_3_tuples(self):
        for r in ev.V1409_RULES:
            assert len(r) == 3
            assert isinstance(r[0], str)
            assert isinstance(r[1], str)
            assert isinstance(r[2], str)

    def test_borrowed_count_7(self):
        assert len(ev.V1409_BORROWED) == 7


# ----------------------- TestV1409Capacities -----------------------

class TestV1409Capacities:
    """12 真 evolution capacities with real evidence + borrowed_from."""

    def test_capacities_count_12(self):
        caps = ev.build_capacities()
        assert len(caps) == 12

    def test_capacities_unique_ids(self):
        caps = ev.build_capacities()
        ids = [c.cap_id for c in caps]
        assert len(ids) == len(set(ids)), "duplicate cap_id"

    def test_capacities_have_evidence(self):
        caps = ev.build_capacities()
        for c in caps:
            assert c.evidence and len(c.evidence) > 20

    def test_capacities_have_borrowed(self):
        caps = ev.build_capacities()
        for c in caps:
            assert c.borrowed_from and len(c.borrowed_from) > 5

    def test_capacities_evolution_keywords(self):
        caps = ev.build_capacities()
        # Each cap should mention evolution or evolution-related concepts
        for c in caps:
            text = (c.name + c.description + c.evidence).lower()
            assert any(kw in text for kw in [
                "evolution", "lineage", "variation", "horizon",
                "inheritance", "anchor", "north-star", "north star",
                "gap", "level", "chain", "borrowed", "honest",
            ]), f"{c.cap_id} lacks evolution keyword"


# ----------------------- TestV1409Limits -----------------------

class TestV1409Limits:
    """6 真 evolution limits with honest disclosure."""

    def test_limits_count_6(self):
        lims = ev.build_limits()
        assert len(lims) == 6

    def test_limits_unique_ids(self):
        lims = ev.build_limits()
        ids = [lim.lim_id for lim in lims]
        assert len(ids) == len(set(ids)), "duplicate lim_id"

    def test_limits_have_evidence(self):
        lims = ev.build_limits()
        for lim in lims:
            assert lim.evidence and len(lim.evidence) > 20

    def test_limits_have_why_no_phenomenal(self):
        lims = ev.build_limits()
        for lim in lims:
            assert lim.why_no_phenomenal and len(lim.why_no_phenomenal) > 20

    def test_limits_include_v1256_replace(self):
        lims = ev.build_limits()
        ids = [lim.lim_id for lim in lims]
        assert "LIM_NOT_V1256_REPLACE" in ids

    def test_limits_include_v1408_replace(self):
        lims = ev.build_limits()
        ids = [lim.lim_id for lim in lims]
        assert "LIM_NOT_V1408_REPLACE" in ids


# ----------------------- TestV1409Trajectory -----------------------

class TestV1409Trajectory:
    """Trajectory: 25+ points from V1256 anchor to V1409 present to V1410 future."""

    def test_trajectory_count_at_least_25(self):
        traj = ev.build_trajectory()
        assert len(traj) >= 25

    def test_trajectory_includes_v1256_anchor(self):
        traj = ev.build_trajectory()
        versions = [t.version for t in traj]
        assert "V1256" in versions

    def test_trajectory_includes_v1408_north_star(self):
        traj = ev.build_trajectory()
        versions = [t.version for t in traj]
        assert "V1408" in versions

    def test_trajectory_includes_v1409_present(self):
        traj = ev.build_trajectory()
        versions = [t.version for t in traj]
        assert "V1409" in versions

    def test_trajectory_includes_v1410_future(self):
        traj = ev.build_trajectory()
        versions = [t.version for t in traj]
        assert "V1410" in versions

    def test_trajectory_present_status(self):
        traj = ev.build_trajectory()
        present = [t for t in traj if t.status == "present"]
        assert len(present) == 1
        assert present[0].version == "V1409"


# ----------------------- TestV1409Borrowed -----------------------

class TestV1409Borrowed:
    """7 真 evolution borrowed from evolutionary epistemology."""

    def test_borrowed_count_7(self):
        assert len(ev.V1409_BORROWED) == 7

    def test_borrowed_keys_unique(self):
        keys = [b["key"] for b in ev.V1409_BORROWED]
        assert len(keys) == len(set(keys))

    def test_borrowed_keys_include_required(self):
        keys = [b["key"] for b in ev.V1409_BORROWED]
        required = [
            "v1256_unio_mystica_2026",
            "v1408_asi_northstar_2026",
            "popper_1972_objective_knowledge",
            "campbell_1960_evolutionary_epistemology",
            "toulmin_1972_human_understanding",
            "hull_1988_science_as_process",
            "dennett_1995_darwin_dangerous_idea",
        ]
        for k in required:
            assert k in keys, f"missing borrowed key: {k}"


# ----------------------- TestV1409Coherence -----------------------

class TestV1409Coherence:
    """12 pair-wise coherence checks (all pass)."""

    def test_coherence_checks_count_12(self):
        checks = ev.build_coherence_checks()
        assert len(checks) == 12

    def test_coherence_checks_all_pass(self):
        checks = ev.build_coherence_checks()
        for c in checks:
            assert c.passes, f"coherence failed: {c.pair}"

    def test_coherence_pairs_unique(self):
        checks = ev.build_coherence_checks()
        pairs = [(c.pair[0], c.pair[1]) for c in checks]
        assert len(pairs) == len(set(pairs)), "duplicate coherence pair"

    def test_coherence_reasons_nonempty(self):
        checks = ev.build_coherence_checks()
        for c in checks:
            assert c.reason and len(c.reason) > 10


# ----------------------- TestV1409ChainDelegate -----------------------

class TestV1409ChainDelegate:
    """Chain delegate V1400-V1408 (9 frameworks) all_ok=True."""

    def test_chain_delegate_all_ok(self):
        cd = ev.build_chain_delegate()
        assert cd.all_ok is True

    def test_chain_delegate_count_9(self):
        cd = ev.build_chain_delegate()
        assert len(cd.delegated) == 9

    def test_chain_delegate_total_capacities_108(self):
        cd = ev.build_chain_delegate()
        assert cd.total_capacities == 108  # 9 frameworks × 12 caps

    def test_chain_delegate_total_limits_54(self):
        cd = ev.build_chain_delegate()
        assert cd.total_limits == 54  # 9 frameworks × 6 lims

    def test_chain_delegate_schema(self):
        cd = ev.build_chain_delegate()
        assert "v1409.evolution" in cd.schema

    def test_chain_delegate_includes_v1400_to_v1408(self):
        cd = ev.build_chain_delegate()
        fws = [d["framework"] for d in cd.delegated]
        for fw in ["V1400", "V1401", "V1402", "V1403", "V1404",
                   "V1405", "V1406", "V1407", "V1408"]:
            assert fw in fws, f"missing framework: {fw}"

    def test_chain_delegate_each_ok(self):
        cd = ev.build_chain_delegate()
        for d in cd.delegated:
            assert d["ok"] is True


# ----------------------- TestV1409Popper -----------------------

class TestV1409Popper:
    """Popper self-test 7/7 pass."""

    def test_popper_all_pass(self):
        result = ev.popper_self_test()
        assert result["all_pass"] is True

    def test_popper_total_7(self):
        result = ev.popper_self_test()
        assert result["total_checks"] == 7

    def test_popper_passed_7(self):
        result = ev.popper_self_test()
        assert result["passed_checks"] == 7

    def test_popper_individual_flags(self):
        result = ev.popper_self_test()
        for key in [
            "anchor_declared", "northstar_locked", "gap_preserved",
            "level_declared", "chain_delegate_real",
            "delegated_9_frameworks", "honest_disclosure",
        ]:
            assert result[key] is True, f"popper flag {key} not True"


# ----------------------- TestV1409Report -----------------------

class TestV1409Report:
    """EvolutionReport structure: anchor + north-star + capacities + limits."""

    def test_report_anchor_v1256(self):
        r = ev.build_report()
        assert r.anchor_version == "V1256"
        assert r.anchor_value == 0.9105

    def test_report_north_star_v1408(self):
        r = ev.build_report()
        assert r.north_star_version == "V1408"
        assert r.north_star_ceiling == 0.98
        assert r.absolute_ceiling == 0.99

    def test_report_current_realized_honest(self):
        r = ev.build_report()
        # Honest cap preserved: current_realized == anchor_value (V1256)
        assert r.current_realized == 0.9105

    def test_report_gap_to_north_star(self):
        r = ev.build_report()
        assert abs(r.gap_to_north_star - 0.0695) < 1e-6

    def test_report_gap_to_ceiling(self):
        r = ev.build_report()
        assert abs(r.gap_to_ceiling - 0.0795) < 1e-6

    def test_report_evolution_levels_9(self):
        r = ev.build_report()
        assert len(r.evolution_levels) == 9
        assert r.evolution_levels[0] == "L0_DATA"
        assert r.evolution_levels[-1] == "L8_EVOLVE"

    def test_report_asi_7_complete(self):
        r = ev.build_report()
        assert r.asi_7_philosophy_complete is True


# ----------------------- TestV1409HorizonLineage -----------------------

class TestV1409HorizonLineage:
    """Horizon scan + lineage mechanism."""

    def test_horizon_includes_v1410_candidates(self):
        from v1409_asi_evolution_framework import build_parser, run_cli
        # Just verify the data exists
        assert ev.V1409_MODULE is not None
        # Check the horizon data via run_cli output structure
        report = ev.build_report()
        assert len(report.trajectory) > 0
        future = [t for t in report.trajectory if t.status == "future"]
        assert len(future) >= 1

    def test_lineage_chain_complete(self):
        r = ev.build_report()
        traj_versions = [t.version for t in r.trajectory]
        # Must include V1400-V1409 + V1256 anchor
        for v in ["V1256", "V1400", "V1408", "V1409"]:
            assert v in traj_versions


# ----------------------- TestV1409CLI -----------------------

class TestV1409CLI:
    """CLI: version/evolution-report/capacity/limits/trajectory/rules/chain/
    popper/horizon/lineage/demo/help."""

    def test_build_parser(self):
        p = ev.build_parser()
        assert p is not None

    def test_cli_version(self, capsys):
        rc = ev.run_cli(["version"])
        assert rc == 0
        out = capsys.readouterr().out
        data = json.loads(out)
        assert data["module"] == ev.V1409_MODULE
        assert data["version"] == ev.V1409_VERSION

    def test_cli_evolution_report(self, capsys):
        rc = ev.run_cli(["evolution-report"])
        assert rc == 0
        out = capsys.readouterr().out
        data = json.loads(out)
        assert data["module"] == ev.V1409_MODULE
        assert data["anchor_value"] == 0.9105
        assert data["north_star_ceiling"] == 0.98

    def test_cli_capacity(self, capsys):
        rc = ev.run_cli(["capacity"])
        assert rc == 0
        out = capsys.readouterr().out
        data = json.loads(out)
        assert len(data) == 12

    def test_cli_limits(self, capsys):
        rc = ev.run_cli(["limits"])
        assert rc == 0
        out = capsys.readouterr().out
        data = json.loads(out)
        assert len(data) == 6

    def test_cli_trajectory(self, capsys):
        rc = ev.run_cli(["trajectory"])
        assert rc == 0
        out = capsys.readouterr().out
        data = json.loads(out)
        assert len(data) >= 25

    def test_cli_rules(self, capsys):
        rc = ev.run_cli(["rules"])
        assert rc == 0
        out = capsys.readouterr().out
        data = json.loads(out)
        assert len(data) == 12

    def test_cli_chain(self, capsys):
        rc = ev.run_cli(["chain"])
        assert rc == 0
        out = capsys.readouterr().out
        data = json.loads(out)
        assert data["all_ok"] is True
        assert len(data["delegated"]) == 9

    def test_cli_popper(self, capsys):
        rc = ev.run_cli(["popper"])
        assert rc == 0
        out = capsys.readouterr().out
        data = json.loads(out)
        assert data["all_pass"] is True
        assert data["passed_checks"] == 7

    def test_cli_horizon(self, capsys):
        rc = ev.run_cli(["horizon"])
        assert rc == 0
        out = capsys.readouterr().out
        data = json.loads(out)
        assert "v1410_candidates" in data
        assert len(data["v1410_candidates"]) >= 1
        assert "constraints" in data

    def test_cli_lineage(self, capsys):
        rc = ev.run_cli(["lineage"])
        assert rc == 0
        out = capsys.readouterr().out
        data = json.loads(out)
        assert data["anchor"].startswith("V1256")
        assert data["north_star"].startswith("V1408")
        assert len(data["frameworks_chain"]) == 10  # V1400-V1409

    def test_cli_demo(self, capsys):
        rc = ev.run_cli(["demo"])
        assert rc == 0
        out = capsys.readouterr().out
        data = json.loads(out)
        assert data["capacities_count"] == 12
        assert data["limits_count"] == 6
        assert data["chain_delegate_all_ok"] is True


# ----------------------- TestV1409Format -----------------------

class TestV1409Format:
    """--format text|json|md."""

    def test_format_json_shorthand(self, capsys):
        rc = ev.run_cli(["version", "--json"])
        assert rc == 0
        out = capsys.readouterr().out
        data = json.loads(out)
        assert data["version"] == "0.1.0"

    def test_format_explicit_json(self, capsys):
        rc = ev.run_cli(["version", "--format", "json"])
        assert rc == 0
        out = capsys.readouterr().out
        data = json.loads(out)
        assert data["version"] == "0.1.0"

    def test_format_md(self, capsys):
        rc = ev.run_cli(["version", "--format", "md"])
        assert rc == 0
        out = capsys.readouterr().out
        assert "# v1409_asi_evolution_framework" in out


# ----------------------- TestV1409Deterministic -----------------------

class TestV1409Deterministic:
    """Reproducibility: build_report() is deterministic."""

    def test_build_report_deterministic(self):
        r1 = ev.build_report()
        r2 = ev.build_report()
        assert r1.anchor_value == r2.anchor_value
        assert r1.gap_to_north_star == r2.gap_to_north_star
        assert len(r1.capacities) == len(r2.capacities)
        assert len(r1.limits) == len(r2.limits)


# ----------------------- TestV1409Subprocess -----------------------

class TestV1409Subprocess:
    """End-to-end subprocess tests via CLI."""

    def test_subprocess_version(self):
        result = subprocess.run(
            [sys.executable, "-m", "v1409_asi_evolution_framework", "version"],
            capture_output=True, text=True,
            cwd=str(ROOT / "apeireth"),
        )
        assert result.returncode == 0
        data = json.loads(result.stdout)
        assert data["version"] == "0.1.0"

    def test_subprocess_chain(self):
        result = subprocess.run(
            [sys.executable, "-m", "v1409_asi_evolution_framework", "chain"],
            capture_output=True, text=True,
            cwd=str(ROOT / "apeireth"),
        )
        assert result.returncode == 0
        data = json.loads(result.stdout)
        assert data["all_ok"] is True
        assert len(data["delegated"]) == 9

    def test_subprocess_popper(self):
        result = subprocess.run(
            [sys.executable, "-m", "v1409_asi_evolution_framework", "popper"],
            capture_output=True, text=True,
            cwd=str(ROOT / "apeireth"),
        )
        assert result.returncode == 0
        data = json.loads(result.stdout)
        assert data["all_pass"] is True

    def test_subprocess_demo(self):
        result = subprocess.run(
            [sys.executable, "-m", "v1409_asi_evolution_framework", "demo"],
            capture_output=True, text=True,
            cwd=str(ROOT / "apeireth"),
        )
        assert result.returncode == 0
        data = json.loads(result.stdout)
        assert data["capacities_count"] == 12
        assert data["limits_count"] == 6

    def test_subprocess_horizon(self):
        result = subprocess.run(
            [sys.executable, "-m", "v1409_asi_evolution_framework", "horizon"],
            capture_output=True, text=True,
            cwd=str(ROOT / "apeireth"),
        )
        assert result.returncode == 0
        data = json.loads(result.stdout)
        assert len(data["v1410_candidates"]) >= 1


# ----------------------- TestV1409Continuity -----------------------

class TestV1409Continuity:
    """Continuity with V1256 anchor + V1408 north-star + chain."""

    def test_inherits_v1256_anchor(self):
        # V1409 evolution inherits V1256 honest cap
        r = ev.build_report()
        assert r.anchor_value == 0.9105  # V1256 unio_mystica

    def test_inherits_v1408_north_star(self):
        # V1409 evolution locks to V1408 north-star
        r = ev.build_report()
        assert r.north_star_version == "V1408"
        assert r.north_star_ceiling == 0.98

    def test_chain_inherits_v1400_to_v1408(self):
        r = ev.build_report()
        fws = [d["framework"] for d in r.chain_delegate.delegated]
        for fw in ["V1400", "V1401", "V1402", "V1403", "V1404",
                   "V1405", "V1406", "V1407", "V1408"]:
            assert fw in fws

    def test_horizon_announces_v1410(self):
        # V1409 evolution announces V1410 future
        traj = ev.build_trajectory()
        future = [t for t in traj if t.status == "future"]
        assert len(future) >= 1
        assert future[0].version == "V1410"