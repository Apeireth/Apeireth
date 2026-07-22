"""R4 ASI fun score smoke tests."""
from __future__ import annotations

import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[1]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from apeireth.asi_fun_score import (  # noqa: E402
    ASIFunMetadata,
    compute_asi_fun_score,
    explain_asi_fun_score,
)


def _run(**changes):
    data = {
        "task_type": "code",
        "model": "test-model",
        "deliberation": True,
        "reasoning_steps": 6,
        "emergence_index": 0.8,
        "phi_intrinsic": 0.7,
        "hqb_verdict": "accept",
        "hqb_violations": 0,
        "total_decisions": 4,
    }
    data.update(changes)
    return compute_asi_fun_score(data)


def test_score_is_bounded():
    assert 0.0 <= _run(emergence_index=99, phi_intrinsic=-4) <= 1.0


def test_deeper_reflection_scores_higher():
    assert _run(reasoning_steps=12) > _run(reasoning_steps=1)


def test_hqb_violations_lower_score():
    clean = _run(hqb_violations=0)
    violated = _run(hqb_violations=3, hqb_verdict="review")
    assert violated < clean


def test_all_zero_input_is_zero():
    assert compute_asi_fun_score(ASIFunMetadata()) == pytest.approx(0.0)


def test_friendly_components_and_weight_override_are_auditable():
    result = explain_asi_fun_score(
        {"emergence": 1, "phi": 0, "deliberation_depth": 0,
         "total_decisions": 1, "hqb_violations": 0},
        weights={"w1": 1, "w2": 0, "w3": 0, "w4": 0},
    )
    assert result["self_organized"] == 1.0
    assert result["score"] == 1.0
    assert sum(result["weights"].values()) == pytest.approx(1.0)
    assert result["task_type"] == ""


if __name__ == "__main__":
    pytest.main([__file__, "-q"])
