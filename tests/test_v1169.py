"""Test V1169 — ASI reinforcement_learning V0.6 follow-up (5 sub-dim 真补).

主 17:43 实事求是: 测试覆盖 constants / dataclasses / helpers / _measure_*
with monkeypatched underlying modules (V1069) (不实际真 instantiate 14 RL agents).
"""

from __future__ import annotations

import sys
import types
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT))

import pytest


class TestV1169Constants:
    def test_version_present(self):
        from apeireth.v1169_asi_reinforcement_learning_v06_real_measure import V1169_VERSION
        assert V1169_VERSION == "0.1.0"

    def test_dim_version(self):
        from apeireth.v1169_asi_reinforcement_learning_v06_real_measure import V1169_DIM_VERSION
        assert V1169_DIM_VERSION == "0.6"

    def test_subdim_names_locked(self):
        from apeireth.v1169_asi_reinforcement_learning_v06_real_measure import V1169_SUBDIM_NAMES
        assert V1169_SUBDIM_NAMES == (
            "agents_real",
            "references_real",
            "v3_guards_real",
            "metrics_real",
            "v02_bridge_real",
        )

    def test_baseline_target_constants(self):
        from apeireth.v1169_asi_reinforcement_learning_v06_real_measure import (
            V1155_BASELINE_REINFORCEMENT_LEARNING,
            TARGET_REINFORCEMENT_LEARNING_V06,
        )
        assert V1155_BASELINE_REINFORCEMENT_LEARNING == 0.7272
        assert TARGET_REINFORCEMENT_LEARNING_V06 == 0.85

    def test_artifact_dir_default(self):
        from apeireth.v1169_asi_reinforcement_learning_v06_real_measure import DEFAULT_ARTIFACT_DIR
        assert DEFAULT_ARTIFACT_DIR == "artifacts"

    def test_expected_agents_locked(self):
        from apeireth.v1169_asi_reinforcement_learning_v06_real_measure import EXPECTED_AGENTS
        assert "QValue" in EXPECTED_AGENTS
        assert "ReplayBuffer" in EXPECTED_AGENTS
        assert "DQN" in EXPECTED_AGENTS
        assert "PPO" in EXPECTED_AGENTS
        assert "SAC" in EXPECTED_AGENTS
        assert "A3C" in EXPECTED_AGENTS
        assert "RainbowConfig" in EXPECTED_AGENTS
        assert len(EXPECTED_AGENTS) >= 8

    def test_expected_references_locked(self):
        from apeireth.v1169_asi_reinforcement_learning_v06_real_measure import EXPECTED_REFERENCES
        assert "Mnih" in EXPECTED_REFERENCES
        assert "Schulman" in EXPECTED_REFERENCES
        assert "Haarnoja" in EXPECTED_REFERENCES
        assert "Hessel" in EXPECTED_REFERENCES
        assert "Hafner" in EXPECTED_REFERENCES
        assert len(EXPECTED_REFERENCES) >= 14


class TestSafeHelpers:
    def test_safe_import_returns_none_on_missing(self):
        from apeireth.v1169_asi_reinforcement_learning_v06_real_measure import _safe_import
        assert _safe_import("nonexistent.module.xyz") is None

    def test_safe_import_returns_module_on_present(self):
        from apeireth.v1169_asi_reinforcement_learning_v06_real_measure import _safe_import
        mod = _safe_import("apeireth.v1169_asi_reinforcement_learning_v06_real_measure")
        assert mod is not None

    def test_attr_returns_none_on_none_mod(self):
        from apeireth.v1169_asi_reinforcement_learning_v06_real_measure import _attr
        assert _attr(None, "X") is None

    def test_class_signature_non_class(self):
        from apeireth.v1169_asi_reinforcement_learning_v06_real_measure import _class_signature
        assert _class_signature("notaclass") == ()

    def test_has_method_present(self):
        class T:
            def f(self): pass
        from apeireth.v1169_asi_reinforcement_learning_v06_real_measure import _has_method
        assert _has_method(T, "f") is True

    def test_has_method_missing(self):
        from apeireth.v1169_asi_reinforcement_learning_v06_real_measure import _has_method
        assert _has_method(int, "nonexistent_method") is False


class TestSubDim1AgentsReal:
    def test_agents_real_real_v1069(self):
        """With real V1069 module loaded — should find all 8+ agents."""
        from apeireth.v1169_asi_reinforcement_learning_v06_real_measure import (
            _measure_agents_real, _safe_import,
        )
        v1069 = _safe_import("apeireth.v1069_asi_reinforcement_learning_core")
        if v1069 is None:
            pytest.skip("V1069 not importable")
        sc, raw = _measure_agents_real(v1069)
        assert sc >= 0.8
        assert raw["n_found"] >= 8

    def test_agents_real_missing_returns_zero(self):
        from apeireth.v1169_asi_reinforcement_learning_v06_real_measure import _measure_agents_real
        sc, raw = _measure_agents_real(None)
        assert sc == 0.0
        assert raw["n_found"] == 0

    def test_agents_real_partial_via_fake(self):
        # Simulate a partial module — only 3 classes
        fake = types.SimpleNamespace()
        class QValue: pass
        class DQN: pass
        class PPO: pass
        fake.QValue = QValue
        fake.DQN = DQN
        fake.PPO = PPO
        from apeireth.v1169_asi_reinforcement_learning_v06_real_measure import _measure_agents_real
        sc, raw = _measure_agents_real(fake)
        assert 0.2 <= sc < 0.5
        assert raw["n_found"] == 3


class TestSubDim2ReferencesReal:
    def test_references_real_missing_returns_zero(self):
        from apeireth.v1169_asi_reinforcement_learning_v06_real_measure import _measure_references_real
        sc, raw = _measure_references_real(None)
        assert sc == 0.0
        assert raw["n_found"] == 0


class TestSubDim3V3GuardsReal:
    def test_v3_guards_real_real_v1069(self):
        from apeireth.v1169_asi_reinforcement_learning_v06_real_measure import (
            _measure_v3_guards_real, _safe_import,
        )
        v1069 = _safe_import("apeireth.v1069_asi_reinforcement_learning_core")
        if v1069 is None:
            pytest.skip("V1069 not importable")
        sc, raw = _measure_v3_guards_real(v1069)
        assert sc >= 0.8
        assert raw["n_valid"] >= 5

    def test_v3_guards_real_missing_v1069(self):
        from apeireth.v1169_asi_reinforcement_learning_v06_real_measure import _measure_v3_guards_real
        sc, raw = _measure_v3_guards_real(None)
        # None won't have V3_GUARDS attr, so score=0
        assert sc == 0.0

    def test_v3_guards_real_partial_fake(self):
        fake = types.SimpleNamespace()
        fake.V3_GUARDS = {
            "guard1": "module is not ASI",
            "guard2": "RL is decision tool",
            "guard3": "Q-value is engineering not value",
        }
        from apeireth.v1169_asi_reinforcement_learning_v06_real_measure import _measure_v3_guards_real
        sc, raw = _measure_v3_guards_real(fake)
        assert 0.4 <= sc < 0.7
        assert raw["n_valid"] == 3

    def test_v3_guards_real_not_dict(self):
        fake = types.SimpleNamespace()
        fake.V3_GUARDS = "not a dict"
        from apeireth.v1169_asi_reinforcement_learning_v06_real_measure import _measure_v3_guards_real
        sc, raw = _measure_v3_guards_real(fake)
        assert sc == 0.0


class TestSubDim4MetricsReal:
    def test_metrics_real_real_v1069(self):
        from apeireth.v1169_asi_reinforcement_learning_v06_real_measure import (
            _measure_metrics_real, _safe_import,
        )
        v1069 = _safe_import("apeireth.v1069_asi_reinforcement_learning_core")
        if v1069 is None:
            pytest.skip("V1069 not importable")
        sc, raw = _measure_metrics_real(v1069)
        assert sc >= 0.7
        assert raw["n_found"] >= 4


class TestSubDim5V02BridgeReal:
    def test_v02_bridge_real_real_v1069(self):
        from apeireth.v1169_asi_reinforcement_learning_v06_real_measure import (
            _measure_v02_bridge_real, _safe_import,
        )
        v1069 = _safe_import("apeireth.v1069_asi_reinforcement_learning_core")
        if v1069 is None:
            pytest.skip("V1069 not importable")
        sc, raw = _measure_v02_bridge_real(v1069)
        assert sc >= 0.5

    def test_v02_bridge_real_missing_v1069(self):
        from apeireth.v1169_asi_reinforcement_learning_v06_real_measure import _measure_v02_bridge_real
        sc, raw = _measure_v02_bridge_real(None)
        assert sc == 0.0


class TestDataclass:
    def test_subdim_evidence_default(self):
        from apeireth.v1169_asi_reinforcement_learning_v06_real_measure import SubDimEvidence
        ev = SubDimEvidence(name="x")
        assert ev.name == "x"
        assert ev.score == 0.0
        d = ev.to_dict()
        assert d["name"] == "x"

    def test_rl_report_default(self):
        from apeireth.v1169_asi_reinforcement_learning_v06_real_measure import RLReport
        r = RLReport()
        assert r.version == "0.1.0"
        assert r.n_subdims_total == 5
        d = r.to_dict()
        assert "snapshot_id" in d
        assert "sub_dim_evidence" in d


class TestMeasureEndToEnd:
    def test_measure_returns_float(self):
        from apeireth.v1169_asi_reinforcement_learning_v06_real_measure import (
            measure_reinforcement_learning_v06,
        )
        sc = measure_reinforcement_learning_v06()
        assert 0.0 <= sc <= 1.0

    def test_measure_full_returns_report(self):
        from apeireth.v1169_asi_reinforcement_learning_v06_real_measure import (
            measure_reinforcement_learning_full,
        )
        r = measure_reinforcement_learning_full(write_artifact=False)
        assert 0.0 <= r.total <= 1.0
        assert set(r.sub_dim_scores.keys()) == set([
            "agents_real", "references_real", "v3_guards_real", "metrics_real", "v02_bridge_real",
        ])
        assert r.v1155_baseline == 0.7272

    def test_artifact_written(self, tmp_path):
        from apeireth.v1169_asi_reinforcement_learning_v06_real_measure import (
            measure_reinforcement_learning_full,
        )
        r = measure_reinforcement_learning_full(
            artifact_dir=str(tmp_path), write_artifact=True,
        )
        assert r.artifact_path
        assert Path(r.artifact_path).exists()
        # Re-read JSON
        import json
        d = json.loads(Path(r.artifact_path).read_text(encoding="utf-8"))
        assert "total" in d
        assert "sub_dim_scores" in d


class TestCLI:
    def test_cli_json(self, capsys):
        import subprocess
        r = subprocess.run(
            [sys.executable, "-m", "apeireth.v1169_asi_reinforcement_learning_v06_real_measure", "--json", "--no-write"],
            capture_output=True, text=True, cwd=str(ROOT), encoding="utf-8", errors="replace",
        )
        assert r.returncode == 0, f"stderr={r.stderr}"
        assert "total" in (r.stdout or "")

    def test_cli_default(self, capsys):
        import subprocess
        r = subprocess.run(
            [sys.executable, "-m", "apeireth.v1169_asi_reinforcement_learning_v06_real_measure", "--no-write"],
            capture_output=True, text=True, cwd=str(ROOT), encoding="utf-8", errors="replace",
        )
        assert r.returncode == 0, f"stderr={r.stderr}"
        assert "V1169" in (r.stdout or "")

    def test_cli_report(self, capsys):
        import subprocess
        r = subprocess.run(
            [sys.executable, "-m", "apeireth.v1169_asi_reinforcement_learning_v06_real_measure", "--report", "--no-write"],
            capture_output=True, text=True, cwd=str(ROOT), encoding="utf-8", errors="replace",
        )
        assert r.returncode == 0, f"stderr={r.stderr}"
        assert "V1169" in (r.stdout or "")
        assert "5 sub-dim" in (r.stdout or "")
