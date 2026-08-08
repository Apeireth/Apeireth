"""Tests for V1356 ASI Pole-Star V0.2 honest re-measurement."""
import json
import sys
from pathlib import Path

import pytest

REPO = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(REPO))

from apeireth.v1356_asi_pole_star_v02 import (
    V1356_VERSION, V1356_ASI_HONEST_CAP, V1356_V01_REFERENCE_SCORE,
    V02_COMPONENTS,
    ComponentScore, PoleStarV02Report,
    measure_v02, render_report,
    _popper_self_tests,
    _measure_phi_proxy, _measure_capabilities, _measure_engineering,
    _measure_v2_philosophy, _measure_v3_philosophy_addendum,
    _measure_real_production, _measure_vcp_toolchain, _measure_cross_domain,
    _measure_approach_margin,
)


# -----------------------------------------------------------------------------
# Constants
# -----------------------------------------------------------------------------

class TestConstants:
    def test_version_semver(self):
        assert V1356_VERSION.count(".") == 2

    def test_honest_cap_090(self):
        assert abs(V1356_ASI_HONEST_CAP - 0.90) < 1e-9

    def test_v01_baseline_reference(self):
        assert abs(V1356_V01_REFERENCE_SCORE - 0.7905) < 1e-9


# -----------------------------------------------------------------------------
# Component weight table
# -----------------------------------------------------------------------------

class TestComponentTable:
    def test_components_nonempty(self):
        assert len(V02_COMPONENTS) >= 5

    def test_weights_sum_below_cap(self):
        """Component weights sum to ~0.85 (approach margin fills to cap)."""
        total = sum(c["weight"] for c in V02_COMPONENTS)
        assert 0.80 <= total <= 0.90, f"got {total}"

    def test_each_weight_in_range(self):
        for c in V02_COMPONENTS:
            assert 0.0 < c["weight"] < 0.30, f"{c['name']}: weight={c['weight']}"


# -----------------------------------------------------------------------------
# Each individual measurement function
# -----------------------------------------------------------------------------

class TestIndividualMeasures:
    @pytest.mark.parametrize("comp", V02_COMPONENTS, ids=lambda c: c["name"])
    def test_measure_returns_01(self, comp):
        v, ev = comp["measure"]()
        assert 0.0 <= v <= 1.0, f"{comp['name']} out of range: {v}"
        assert isinstance(ev, str) and len(ev) > 0

    def test_phi_proxy_default(self, monkeypatch):
        """If no phi file exists, default 0.50 (主 17:43)."""
        # Monkeypatch the paths v1356 looks at to non-existent ones
        from apeireth import v1356_asi_pole_star_v02 as m
        monkeypatch.setattr(m, "APEIRETH_DIR", m.REPO_ROOT / "_nonexistent_dir")
        v, ev = _measure_phi_proxy()
        assert v == 0.50
        assert "default 0.50" in ev

    def test_capabilities_uses_real_disk(self):
        v, ev = _measure_capabilities()
        assert 0.0 <= v <= 1.0
        assert "tests=" in ev and "modules=" in ev

    def test_engineering_uses_real_disk(self):
        v, ev = _measure_engineering()
        assert 0.0 <= v <= 1.0
        assert "commits=" in ev


# -----------------------------------------------------------------------------
# Approach margin mechanic
# -----------------------------------------------------------------------------

class TestApproachMargin:
    def test_margin_is_remainder(self):
        """approach_margin = max(0, cap - weighted_subtotal)."""
        from apeireth.v1356_asi_pole_star_v02 import V1356_ASI_HONEST_CAP
        v, _ = _measure_approach_margin(weighted_subtotal=0.30)
        assert abs(v - (V1356_ASI_HONEST_CAP - 0.30)) < 1e-9

    def test_margin_zero_when_over(self):
        """If subtotal already > cap, margin = 0 (total capped structurally)."""
        v, ev = _measure_approach_margin(weighted_subtotal=1.50)
        assert v == 0.0
        assert "structural" in ev

    def test_margin_no_subsume_cap(self):
        from apeireth.v1356_asi_pole_star_v02 import V1356_ASI_HONEST_CAP
        v, _ = _measure_approach_margin(weighted_subtotal=0.0)
        assert v == pytest.approx(V1356_ASI_HONEST_CAP)


# -----------------------------------------------------------------------------
# measure_v02() end-to-end
# -----------------------------------------------------------------------------

class TestMeasureV02:
    def test_total_under_cap(self):
        """Hard structural invariant: total <= honest_cap."""
        report = measure_v02()
        assert report.total <= V1356_ASI_HONEST_CAP + 1e-9

    def test_total_non_negative(self):
        report = measure_v02()
        assert report.total >= 0.0

    def test_delta_finite(self):
        report = measure_v02()
        assert abs(report.delta_vs_v01) < 1.0

    def test_components_present(self):
        report = measure_v02()
        assert len(report.components) >= 5

    def test_each_component_weight_in_table(self):
        report = measure_v02()
        for c in report.components:
            assert c.weight > 0
            assert 0.0 <= c.raw_value <= 1.0

    def test_proximity_string_set(self):
        report = measure_v02()
        assert report.asi_proximity in ("approach", "near", "far")


# -----------------------------------------------------------------------------
# Rendering
# -----------------------------------------------------------------------------

class TestRendering:
    def test_render_text_mode(self):
        report = measure_v02()
        text = render_report(report)
        assert isinstance(text, str)
        assert "V1356 ASI Pole-Star V0.2" in text
        assert "0.7905" in text  # baseline
        assert "Honest cap" in text

    def test_render_includes_component_evidence(self):
        report = measure_v02()
        text = render_report(report)
        for c in report.components:
            assert c.name in text


# -----------------------------------------------------------------------------
# Self-tests
# -----------------------------------------------------------------------------

class TestPopperSelfTests:
    def test_self_tests_pass(self):
        passed, total, failures = _popper_self_tests(verbose=False)
        assert passed == total, f"failed: {failures}"

    def test_self_tests_count_24_plus(self):
        _, total, _ = _popper_self_tests(verbose=False)
        assert total >= 24, f"only {total} self-tests"


# -----------------------------------------------------------------------------
# CLI smoke
# -----------------------------------------------------------------------------

class TestCLI:
    def test_cli_measure_text(self, capsys):
        from apeireth.v1356_asi_pole_star_v02 import main
        rc = main(["measure"])
        assert rc == 0
        out = capsys.readouterr().out
        assert "V0.2" in out

    def test_cli_measure_json(self, capsys):
        from apeireth.v1356_asi_pole_star_v02 import main
        rc = main(["measure", "--json"])
        assert rc == 0
        out = capsys.readouterr().out
        data = json.loads(out)
        assert "total" in data
        assert "components" in data

    def test_cli_delta(self, capsys):
        from apeireth.v1356_asi_pole_star_v02 import main
        rc = main(["delta"])
        assert rc == 0
        out = capsys.readouterr().out
        assert "Delta" in out

    def test_cli_self_test(self, capsys):
        from apeireth.v1356_asi_pole_star_v02 import main
        rc = main(["self-test"])
        assert rc == 0
        out = capsys.readouterr().out
        assert "passed" in out

    def test_cli_version(self, capsys):
        from apeireth.v1356_asi_pole_star_v02 import main
        rc = main(["version"])
        assert rc == 0
        out = capsys.readouterr().out
        assert V1356_VERSION in out


if __name__ == "__main__":
    pytest.main([__file__, "-v", "--tb=short"])
