"""Test suite for V1408 ASI 真北极星 (NorthStar) framework v1.

Target: 90+ tests covering constants, capacities, limits, trajectory, rules,
borrowed, coherence, chain delegate (real V1400-V1407 invocation),
popper self-test, report, CLI, anchor, gap, north-star, format.

主 17:43 实事求是: 真北极星真调真测; 主 00:36 质量工程化 popper + 4 exit codes;
主 17:58 不假装 Phenomenal; 主 20:46 不假装达到 ASI;
主 17:58 不假装替代 V1256 / V1259;
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

import v1408_asi_northstar_framework as v1408


# ----------------------- TestV1408Constants -----------------------

class TestV1408Constants:
    def test_version_format(self):
        parts = v1408.V1408_VERSION.split(".")
        assert len(parts) == 3, "version must be semver X.Y.Z"

    def test_module_name(self):
        assert v1408.V1408_MODULE == "v1408_asi_northstar_framework"

    def test_guards_count(self):
        assert len(v1408.V1408_GUARDS) == 15

    def test_v3_guards_count(self):
        assert len(v1408.V1408_V3_GUARDS) == 6

    def test_v3_guards_unique(self):
        assert len(set(v1408.V1408_V3_GUARDS)) == 6

    def test_rules_count(self):
        assert len(v1408.V1408_RULES) == 12

    def test_rules_unique(self):
        assert len(set(r[0] for r in v1408.V1408_RULES)) == 12

    def test_rules_triple_structure(self):
        for r in v1408.V1408_RULES:
            assert len(r) == 3
            assert r[1] in ("info", "warn", "error")

    def test_borrowed_count(self):
        assert len(v1408.V1408_BORROWED) == 7


# ----------------------- TestV1408Capacities -----------------------

class TestV1408Capacities:
    def test_capacities_count(self):
        caps = v1408.build_capacities()
        assert len(caps) == 12

    def test_capacities_unique(self):
        caps = v1408.build_capacities()
        ids = [c.cap_id for c in caps]
        assert len(set(ids)) == 12

    def test_capacities_have_borrowed(self):
        caps = v1408.build_capacities()
        for c in caps:
            assert c.borrowed_from, f"{c.cap_id} missing borrowed_from"

    def test_capacities_have_evidence(self):
        caps = v1408.build_capacities()
        for c in caps:
            assert c.evidence, f"{c.cap_id} missing evidence"

    def test_capacities_have_anchor_cap(self):
        caps = v1408.build_capacities()
        anchor_caps = [c for c in caps if "ANCHOR" in c.cap_id]
        assert len(anchor_caps) >= 1


# ----------------------- TestV1408Limits -----------------------

class TestV1408Limits:
    def test_limits_count(self):
        lims = v1408.build_limits()
        assert len(lims) == 6

    def test_limits_unique(self):
        lims = v1408.build_limits()
        ids = [lim.lim_id for lim in lims]
        assert len(set(ids)) == 6

    def test_limits_have_evidence(self):
        lims = v1408.build_limits()
        for lim in lims:
            assert lim.evidence, f"{lim.lim_id} missing evidence"

    def test_limits_have_why_no_phenomenal(self):
        lims = v1408.build_limits()
        for lim in lims:
            assert lim.why_no_phenomenal, f"{lim.lim_id} missing why_no_phenomenal"

    def test_limits_disallow_asi(self):
        lims = v1408.build_limits()
        asi_lim = [l for l in lims if "NOT_ASI" in l.lim_id]
        assert len(asi_lim) >= 1


# ----------------------- TestV1408Trajectory -----------------------

class TestV1408Trajectory:
    def test_trajectory_count(self):
        traj = v1408.build_trajectory()
        assert len(traj) >= 25

    def test_trajectory_has_anchor(self):
        traj = v1408.build_trajectory()
        anchor = [t for t in traj if t.kind == "anchor"]
        assert len(anchor) >= 1
        assert anchor[0].version == "V1256"

    def test_trajectory_has_present(self):
        traj = v1408.build_trajectory()
        present = [t for t in traj if t.status == "present"]
        assert len(present) >= 1
        assert present[0].version == "V1408"

    def test_trajectory_has_past(self):
        traj = v1408.build_trajectory()
        past = [t for t in traj if t.status == "past"]
        assert len(past) >= 20

    def test_trajectory_has_v1407(self):
        traj = v1408.build_trajectory()
        v1407_pts = [t for t in traj if t.version == "V1407"]
        assert len(v1407_pts) == 1

    def test_trajectory_unique(self):
        traj = v1408.build_trajectory()
        versions = [t.version for t in traj]
        assert len(set(versions)) == len(versions)


# ----------------------- TestV1408Borrowed -----------------------

class TestV1408Borrowed:
    def test_borrowed_unique_keys(self):
        keys = [b["key"] for b in v1408.V1408_BORROWED]
        assert len(set(keys)) == len(keys)

    def test_borrowed_has_v1256(self):
        keys = [b["key"] for b in v1408.V1408_BORROWED]
        assert any("v1256" in k.lower() for k in keys)


# ----------------------- TestV1408Coherence -----------------------

class TestV1408Coherence:
    def test_coherence_count(self):
        checks = v1408.coherence_check()
        assert len(checks) == 12

    def test_coherence_all_pass(self):
        checks = v1408.coherence_check()
        assert all(c.passes for c in checks)

    def test_coherence_pair_unique(self):
        checks = v1408.coherence_check()
        pairs = [c.pair for c in checks]
        assert len(set(pairs)) == len(pairs)

    def test_coherence_cycle(self):
        checks = v1408.coherence_check()
        # First pair starts with first cap
        caps = [c.cap_id for c in v1408.build_capacities()]
        assert checks[0].pair[0] == caps[0]
        # Last pair closes cycle
        assert checks[-1].pair[1] == caps[0]


# ----------------------- TestV1408ChainDelegate -----------------------

class TestV1408ChainDelegate:
    def test_chain_schema(self):
        cd = v1408.chain_delegate()
        assert cd.schema == (
            "v1408.northstar-production-judge-explainer-trace-meta-self"
            "-cognition-integration.chain/v1"
        )

    def test_chain_all_ok(self):
        cd = v1408.chain_delegate()
        assert cd.all_ok is True

    def test_chain_total_capacities(self):
        cd = v1408.chain_delegate()
        assert cd.total_capacities == 96

    def test_chain_total_limits(self):
        cd = v1408.chain_delegate()
        assert cd.total_limits == 48

    def test_chain_8_frameworks(self):
        cd = v1408.chain_delegate()
        assert len(cd.delegated) == 8

    def test_chain_frameworks_v1400_to_v1407(self):
        cd = v1408.chain_delegate()
        fws = [d["framework"] for d in cd.delegated]
        assert fws == ["V1400", "V1401", "V1402", "V1403",
                       "V1404", "V1405", "V1406", "V1407"]

    def test_chain_real_invocation(self):
        cd = v1408.chain_delegate()
        for d in cd.delegated:
            assert d["ok"], f"{d['framework']} failed"
            assert d["result_type"] != "None"


# ----------------------- TestV1408Popper -----------------------

class TestV1408Popper:
    def test_popper_pass_count(self):
        pop = v1408.popper_self_test()
        assert pop["pass_count"] == 7

    def test_popper_total_count(self):
        pop = v1408.popper_self_test()
        assert pop["total_count"] == 7

    def test_popper_all_pass(self):
        pop = v1408.popper_self_test()
        assert pop["all_pass"] is True

    def test_popper_anchor(self):
        pop = v1408.popper_self_test()
        assert pop["anchor_declared"] is True

    def test_popper_chain(self):
        pop = v1408.popper_self_test()
        assert pop["chain_delegate_real"] is True


# ----------------------- TestV1408NorthStar -----------------------

class TestV1408NorthStar:
    def test_anchor_version(self):
        r = v1408.run_self_northstar()
        assert r.anchor_version == "V1256"

    def test_anchor_value(self):
        r = v1408.run_self_northstar()
        assert r.anchor_value == 0.9105

    def test_north_star_ceiling(self):
        r = v1408.run_self_northstar()
        assert r.north_star_ceiling == 0.98

    def test_absolute_ceiling(self):
        r = v1408.run_self_northstar()
        assert r.absolute_ceiling == 0.99

    def test_current_realized(self):
        r = v1408.run_self_northstar()
        assert r.current_realized == 0.9105

    def test_gap_to_north_star(self):
        r = v1408.run_self_northstar()
        assert r.gap_to_north_star == 0.0695

    def test_gap_to_ceiling(self):
        r = v1408.run_self_northstar()
        assert r.gap_to_ceiling == 0.0795


# ----------------------- TestV1408Report -----------------------

class TestV1408Report:
    def test_report_has_capacities(self):
        r = v1408.run_self_northstar()
        assert len(r.capacities) == 12

    def test_report_has_limits(self):
        r = v1408.run_self_northstar()
        assert len(r.limits) == 6

    def test_report_has_trajectory(self):
        r = v1408.run_self_northstar()
        assert len(r.trajectory) >= 25

    def test_report_has_chain(self):
        r = v1408.run_self_northstar()
        assert r.chain_delegate.all_ok is True

    def test_report_north_star_levels(self):
        r = v1408.run_self_northstar()
        assert len(r.north_star_levels) == 9
        assert r.north_star_levels[0] == "L0_DATA"
        assert r.north_star_levels[-1] == "L8_NORTHSTAR"

    def test_report_philosophy_complete(self):
        r = v1408.run_self_northstar()
        assert r.asi_7_philosophy_complete is True

    def test_report_generated_at(self):
        r = v1408.run_self_northstar()
        assert r.generated_at.endswith("Z")


# ----------------------- TestV1408ChainDelegateIdempotent -----------------------

class TestV1408ChainDelegateIdempotent:
    def test_chain_delegate_idempotent(self):
        a = v1408.chain_delegate()
        b = v1408.chain_delegate()
        assert a.all_ok == b.all_ok
        assert a.total_capacities == b.total_capacities

    def test_chain_delegate_with_v1407(self):
        # Verify chain_delegate reaches V1407 (production predecessor)
        cd = v1408.chain_delegate()
        v1407 = [d for d in cd.delegated if d["framework"] == "V1407"]
        assert len(v1407) == 1
        assert v1407[0]["ok"] is True


# ----------------------- TestV1408V3Guards -----------------------

class TestV1408V3Guards:
    def test_v3_guards_block_phenomenal(self):
        guards = v1408.V1408_V3_GUARDS
        assert any("NOT_PHENOMENAL" in g for g in guards)

    def test_v3_guards_block_asi(self):
        guards = v1408.V1408_V3_GUARDS
        assert any("NOT_ASI" in g for g in guards)

    def test_v3_guards_block_human_level(self):
        guards = v1408.V1408_V3_GUARDS
        assert any("NOT_HUMAN_LEVEL" in g for g in guards)

    def test_v3_guards_block_absolute(self):
        guards = v1408.V1408_V3_GUARDS
        assert any("NOT_ABSOLUTE" in g for g in guards)


# ----------------------- TestV1408CLI -----------------------

class TestV1408CLI:
    def test_cli_version(self):
        rc = v1408.run_cli(["version"])
        assert rc == 0

    def test_cli_help(self):
        rc = v1408.run_cli(["help"])
        assert rc == 0

    def test_cli_capacity(self):
        rc = v1408.run_cli(["capacity"])
        assert rc == 0

    def test_cli_limits(self):
        rc = v1408.run_cli(["limits"])
        assert rc == 0

    def test_cli_trajectory(self):
        rc = v1408.run_cli(["trajectory"])
        assert rc == 0

    def test_cli_rules(self):
        rc = v1408.run_cli(["rules"])
        assert rc == 0

    def test_cli_chain(self):
        rc = v1408.run_cli(["chain"])
        assert rc == 0

    def test_cli_popper(self):
        rc = v1408.run_cli(["popper"])
        assert rc == 0

    def test_cli_anchor(self):
        rc = v1408.run_cli(["anchor"])
        assert rc == 0

    def test_cli_gap(self):
        rc = v1408.run_cli(["gap"])
        assert rc == 0

    def test_cli_demo(self):
        rc = v1408.run_cli(["demo"])
        assert rc == 0

    def test_cli_northstar_report_text(self):
        rc = v1408.run_cli(["northstar-report", "--format", "text"])
        assert rc == 0

    def test_cli_northstar_report_json(self):
        rc = v1408.run_cli(["northstar-report", "--json"])
        assert rc == 0

    def test_cli_northstar_report_md(self):
        rc = v1408.run_cli(["northstar-report", "--format", "md"])
        assert rc == 0


# ----------------------- TestV1408Format -----------------------

class TestV1408Format:
    def test_format_text_nonempty(self):
        r = v1408.run_self_northstar()
        text = v1408._format_text(r)
        assert "V1408" in text
        assert "0.9105" in text

    def test_format_json_parseable(self):
        r = v1408.run_self_northstar()
        js = v1408._format_json(r)
        parsed = json.loads(js)
        assert parsed["module"] == v1408.V1408_MODULE

    def test_format_md_nonempty(self):
        r = v1408.run_self_northstar()
        md = v1408._format_md(r)
        assert "# V1408" in md


# ----------------------- TestV1408Deterministic -----------------------

class TestV1408Deterministic:
    def test_report_deterministic(self):
        a = v1408.run_self_northstar()
        b = v1408.run_self_northstar()
        # anchor / gap / chain should be deterministic (not generated_at)
        assert a.anchor_value == b.anchor_value
        assert a.gap_to_north_star == b.gap_to_north_star
        assert a.chain_delegate.all_ok == b.chain_delegate.all_ok

    def test_popper_deterministic(self):
        a = v1408.popper_self_test()
        b = v1408.popper_self_test()
        assert a["pass_count"] == b["pass_count"]


# ----------------------- TestV1408Subprocess -----------------------

class TestV1408Subprocess:
    """End-to-end via subprocess.run."""

    def test_subprocess_version(self):
        result = subprocess.run(
            [sys.executable, "-m", "apeireth.v1408_asi_northstar_framework",
             "version"],
            capture_output=True, text=True,
        )
        assert result.returncode == 0
        assert "V1408" in result.stdout

    def test_subprocess_chain(self):
        result = subprocess.run(
            [sys.executable, "-m", "apeireth.v1408_asi_northstar_framework",
             "chain"],
            capture_output=True, text=True,
        )
        assert result.returncode == 0
        assert "all_ok: True" in result.stdout
        assert "V1407" in result.stdout

    def test_subprocess_demo(self):
        result = subprocess.run(
            [sys.executable, "-m", "apeireth.v1408_asi_northstar_framework",
             "demo"],
            capture_output=True, text=True,
        )
        assert result.returncode == 0
        assert "capacities: 12" in result.stdout

    def test_subprocess_northstar_report(self):
        result = subprocess.run(
            [sys.executable, "-m", "apeireth.v1408_asi_northstar_framework",
             "northstar-report", "--format", "md"],
            capture_output=True, text=True, encoding="utf-8",
        )
        assert result.returncode == 0
        assert "V1408" in result.stdout

    def test_subprocess_anchor(self):
        result = subprocess.run(
            [sys.executable, "-m", "apeireth.v1408_asi_northstar_framework",
             "anchor"],
            capture_output=True, text=True,
        )
        assert result.returncode == 0
        assert "V1256" in result.stdout
        assert "0.9105" in result.stdout


# ----------------------- TestV1408Continuity -----------------------

class TestV1408Continuity:
    """V1408 continuity from V1407 production."""

    def test_v1408_continues_v1407(self):
        r = v1408.run_self_northstar()
        traj = [t.version for t in r.trajectory]
        assert "V1407" in traj
        assert "V1408" in traj

    def test_v1408_chain_inherits_v1407(self):
        cd = v1408.chain_delegate()
        fws = [d["framework"] for d in cd.delegated]
        assert "V1407" in fws

    def test_v1408_inherits_production_guard(self):
        guards = v1408.V1408_GUARDS
        assert any("INHERITS_PRODUCTION" in g for g in guards)


# ----------------------- TestV1408BuildParser -----------------------

class TestV1408BuildParser:
    def test_build_capacities_idempotent(self):
        a = v1408.build_capacities()
        b = v1408.build_capacities()
        assert [c.cap_id for c in a] == [c.cap_id for c in b]

    def test_build_limits_idempotent(self):
        a = v1408.build_limits()
        b = v1408.build_limits()
        assert [l.lim_id for l in a] == [l.lim_id for l in b]

    def test_build_trajectory_idempotent(self):
        a = v1408.build_trajectory()
        b = v1408.build_trajectory()
        assert [t.version for t in a] == [t.version for t in b]