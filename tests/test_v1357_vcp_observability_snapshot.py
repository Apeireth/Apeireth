"""Tests for V1357 VCP observability snapshot."""
import json
import sys
from pathlib import Path

import pytest

REPO = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(REPO))

from apeireth.v1357_vcp_observability_snapshot import (
    V1357_VERSION, V1357_ASI_CAP,
    VCP_INFRA_FILES, VCP_TOOLCHAIN_MODULES,
    ProjectSnapshot,
    build_snapshot,
    render_summary, render_recipe, render_pretty,
    _popper_self_tests,
    _module_counts, _infra_state, _recent_commits,
    _measure_pole_star, _measure_toolchain_health, _measure_close_loop,
)


# -----------------------------------------------------------------------------
# Constants
# -----------------------------------------------------------------------------

class TestConstants:
    def test_version_semver(self):
        assert V1357_VERSION.count(".") == 2

    def test_asi_cap_honest(self):
        assert V1357_ASI_CAP <= 0.01

    def test_infra_files_3(self):
        assert len(VCP_INFRA_FILES) == 3

    def test_toolchain_modules_11(self):
        """V1345-V1355 = 11 modules (V1345 was the seed)."""
        assert len(VCP_TOOLCHAIN_MODULES) == 11


# -----------------------------------------------------------------------------
# Module counts
# -----------------------------------------------------------------------------

class TestModuleCounts:
    def test_counts_have_keys(self):
        c = _module_counts()
        assert "apeireth_v_modules" in c
        assert "test_files" in c

    def test_counts_positive(self):
        c = _module_counts()
        assert c["apeireth_v_modules"] > 0
        assert c["test_files"] > 0


# -----------------------------------------------------------------------------
# Infra state
# -----------------------------------------------------------------------------

class TestInfraState:
    def test_three_keys(self):
        state = _infra_state()
        assert set(state.keys()) == {"ledger", "migration_audit", "remediation_history"}

    def test_values_are_bool(self):
        state = _infra_state()
        for v in state.values():
            assert isinstance(v, bool)


# -----------------------------------------------------------------------------
# Recent commits (delegates to git)
# -----------------------------------------------------------------------------

class TestRecentCommits:
    def test_commits_have_required_keys(self):
        commits, _ = _recent_commits(limit=3)
        if commits:
            for c in commits:
                assert "hash" in c
                assert "date" in c
                assert "subject" in c

    def test_limit_respected(self):
        commits, _ = _recent_commits(limit=2)
        assert len(commits) <= 2


# -----------------------------------------------------------------------------
# Toolchain health
# -----------------------------------------------------------------------------

class TestToolchainHealth:
    def test_returns_required_keys(self):
        tc, _ = _measure_toolchain_health()
        assert "n_modules_present" in tc
        assert "n_modules_total" in tc
        assert "presence_ratio" in tc
        assert "modules_present" in tc
        assert "modules_absent" in tc

    def test_presence_below_or_equal_total(self):
        tc, _ = _measure_toolchain_health()
        assert tc["n_modules_present"] <= tc["n_modules_total"]

    def test_sum_invariant(self):
        tc, _ = _measure_toolchain_health()
        assert tc["n_modules_present"] + len(tc["modules_absent"]) == tc["n_modules_total"]


# -----------------------------------------------------------------------------
# Close-loop (V1355)
# -----------------------------------------------------------------------------

class TestCloseLoop:
    def test_close_loop_returns_scenarios(self):
        cl, _ = _measure_close_loop()
        assert "scenarios" in cl
        assert isinstance(cl["scenarios"], list)


# -----------------------------------------------------------------------------
# Pole-star (V1356)
# -----------------------------------------------------------------------------

class TestPoleStar:
    def test_pole_star_returns_dict(self):
        pole, _ = _measure_pole_star()
        assert "total" in pole
        assert "v01_baseline" in pole

    def test_pole_star_baseline_07905(self):
        pole, _ = _measure_pole_star()
        assert pole.get("v01_baseline") == 0.7905


# -----------------------------------------------------------------------------
# Full snapshot
# -----------------------------------------------------------------------------

class TestFullSnapshot:
    def test_snapshot_has_six_sections(self):
        snap = build_snapshot()
        d = snap.to_dict()
        for k in ["pole_star", "toolchain_health", "recent_commits",
                  "infra_state", "close_loop_state", "module_counts"]:
            assert k in d, f"missing {k}"

    def test_snapshot_to_dict_json_round_trip(self):
        snap = build_snapshot()
        text = json.dumps(snap.to_dict(), indent=2)
        loaded = json.loads(text)
        assert loaded["version"] == V1357_VERSION

    def test_snapshot_known_unknowns_field(self):
        snap = build_snapshot()
        assert "known_unknowns" in snap.to_dict()
        assert isinstance(snap.known_unknowns, tuple)

    def test_snapshot_philosophy_guards(self):
        snap = build_snapshot()
        assert any("NOT_ASI" in g for g in snap.philosophy_guards)


# -----------------------------------------------------------------------------
# Renderers
# -----------------------------------------------------------------------------

class TestRenderers:
    def test_summary_is_one_line(self):
        snap = build_snapshot()
        s = render_summary(snap)
        assert isinstance(s, str)
        assert s.count("\n") <= 0
        assert "pole_star" in s

    def test_recipe_has_steps(self):
        snap = build_snapshot()
        r = render_recipe(snap)
        assert "RECIPE" in r
        assert "1." in r
        assert "2." in r
        assert "v1354" in r
        assert "v1355" in r
        assert "v1356" in r

    def test_pretty_has_all_sections(self):
        snap = build_snapshot()
        p = render_pretty(snap)
        assert "Pole-Star" in p
        assert "Toolchain" in p
        assert "Close-Loop" in p
        assert "Infra" in p
        assert "Counts" in p
        assert "Commits" in p


# -----------------------------------------------------------------------------
# Self-tests
# -----------------------------------------------------------------------------

class TestPopperSelfTests:
    def test_self_tests_pass(self):
        passed, total, failures = _popper_self_tests(verbose=False)
        assert passed == total, f"failed: {failures}"

    def test_self_tests_18_plus(self):
        _, total, _ = _popper_self_tests(verbose=False)
        assert total >= 18, f"only {total}"


# -----------------------------------------------------------------------------
# CLI smoke
# -----------------------------------------------------------------------------

class TestCLI:
    def test_cli_snapshot_default(self, capsys):
        from apeireth.v1357_vcp_observability_snapshot import main
        rc = main(["snapshot"])
        # exit 0 if no unknowns, 1 otherwise. Both OK
        assert rc in (0, 1)
        out = capsys.readouterr().out
        data = json.loads(out)
        assert "version" in data

    def test_cli_snapshot_pretty(self, capsys):
        from apeireth.v1357_vcp_observability_snapshot import main
        rc = main(["snapshot", "--pretty"])
        assert rc in (0, 1)
        out = capsys.readouterr().out
        assert "Apeireth Snapshot" in out

    def test_cli_summary(self, capsys):
        from apeireth.v1357_vcp_observability_snapshot import main
        rc = main(["summary"])
        assert rc == 0
        out = capsys.readouterr().out
        assert "pole_star" in out

    def test_cli_recipe(self, capsys):
        from apeireth.v1357_vcp_observability_snapshot import main
        rc = main(["recipe"])
        assert rc == 0
        out = capsys.readouterr().out
        assert "RECIPE" in out

    def test_cli_self_test(self, capsys):
        from apeireth.v1357_vcp_observability_snapshot import main
        rc = main(["self-test"])
        assert rc == 0
        out = capsys.readouterr().out
        assert "self-test:" in out

    def test_cli_version(self, capsys):
        from apeireth.v1357_vcp_observability_snapshot import main
        rc = main(["version"])
        assert rc == 0
        out = capsys.readouterr().out
        assert V1357_VERSION in out


# -----------------------------------------------------------------------------
# V1364: --record flag (opt-in auto-append to V1362 history ledger)
# -----------------------------------------------------------------------------

class TestRecordFlag:
    """V1364: --record flag should opt-in auto-append to V1362 history."""

    def test_snapshot_has_record_in_guards(self):
        """V1364: GUARD_RECORD_IS_OPT_IN and GUARD_DELEGATE_TO_V1362_ON_OPT_IN present."""
        snap = build_snapshot()
        guards = snap.philosophy_guards
        assert "GUARD_RECORD_IS_OPT_IN" in guards
        assert "GUARD_DELEGATE_TO_V1362_ON_OPT_IN" in guards

    def test_default_snapshot_does_not_append(self, tmp_path, monkeypatch, capsys):
        """V1364: default snapshot (no --record) MUST NOT write to history."""
        from apeireth import v1362_pole_star_history as v1362
        # Redirect history file to a temp path so we can detect any writes.
        fake_history = tmp_path / "pole_star_history.jsonl"
        monkeypatch.setattr(v1362, "V1362_HISTORY_FILE", str(fake_history))
        # Also patch the function used by V1357 to ensure it sees the temp path.
        from apeireth import v1357_vcp_observability_snapshot as v1357
        def fake_path() -> Path:
            return Path(str(fake_history))
        monkeypatch.setattr(v1357, "_history_path", fake_path, raising=False)

        # Now run snapshot without --record
        from apeireth.v1357_vcp_observability_snapshot import main
        rc = main(["snapshot", "--json"])
        assert rc in (0, 1)
        # The temp history file MUST NOT exist (no writes happened)
        assert not fake_history.exists(), "default --record must be OFF"

    def test_record_flag_appends_to_history(self, tmp_path, monkeypatch, capsys):
        """V1364: --record flag MUST append exactly one entry."""
        from apeireth import v1362_pole_star_history as v1362
        from apeireth import v1357_vcp_observability_snapshot as v1357
        fake_history = tmp_path / "pole_star_history.jsonl"
        monkeypatch.setattr(v1362, "V1362_HISTORY_FILE", str(fake_history))
        monkeypatch.setattr(v1357, "V1362_HISTORY_FILE", str(fake_history), raising=False)

        from apeireth.v1357_vcp_observability_snapshot import main
        rc = main(["snapshot", "--record", "--tag", "v1364-test-default-off"])
        assert rc in (0, 1)
        assert fake_history.exists(), "--record must create history file"
        lines = [ln for ln in fake_history.read_text(encoding="utf-8").splitlines() if ln.strip()]
        assert len(lines) == 1
        entry = json.loads(lines[0])
        assert entry.get("tag") == "v1364-test-default-off"
        assert entry.get("pole_star_total") == 0.9

    def test_record_flag_stderr_message(self, tmp_path, monkeypatch, capsys):
        """V1364: --record MUST emit confirmation to stderr."""
        from apeireth import v1362_pole_star_history as v1362
        from apeireth import v1357_vcp_observability_snapshot as v1357
        fake_history = tmp_path / "pole_star_history.jsonl"
        monkeypatch.setattr(v1362, "V1362_HISTORY_FILE", str(fake_history))
        monkeypatch.setattr(v1357, "V1362_HISTORY_FILE", str(fake_history), raising=False)

        from apeireth.v1357_vcp_observability_snapshot import main
        main(["snapshot", "--record", "--tag", "v1364-stderr-check"])
        captured = capsys.readouterr()
        assert "[v1364] recorded to history" in captured.err

    def test_record_flag_default_off_when_no_flag(self, tmp_path, monkeypatch, capsys):
        """V1364: even --record without --tag should work; tag is None."""
        from apeireth import v1362_pole_star_history as v1362
        from apeireth import v1357_vcp_observability_snapshot as v1357
        fake_history = tmp_path / "pole_star_history.jsonl"
        monkeypatch.setattr(v1362, "V1362_HISTORY_FILE", str(fake_history))
        monkeypatch.setattr(v1357, "V1362_HISTORY_FILE", str(fake_history), raising=False)

        from apeireth.v1357_vcp_observability_snapshot import main
        main(["snapshot", "--record"])
        assert fake_history.exists()
        entry = json.loads(fake_history.read_text(encoding="utf-8").strip())
        assert "tag" not in entry  # No tag → entry has no 'tag' key

    def test_record_helper_returns_entry_or_none(self):
        """V1364: _record_to_history helper must handle missing v1362 gracefully."""
        from apeireth.v1357_vcp_observability_snapshot import _record_to_history, build_snapshot
        snap = build_snapshot()
        # Normal path: should return a dict
        result = _record_to_history(snap, tag="v1364-helper-test")
        assert result is None or isinstance(result, dict)

    def test_v1362_append_snapshot_with_dict_helper_exists(self):
        """V1364: V1362 must export append_snapshot_with_dict for the no-double-build path."""
        from apeireth import v1362_pole_star_history as v1362
        assert hasattr(v1362, "append_snapshot_with_dict")
        assert callable(v1362.append_snapshot_with_dict)


if __name__ == "__main__":
    pytest.main([__file__, "-v", "--tb=short"])
