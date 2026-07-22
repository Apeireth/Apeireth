"""V1085 philosophy guard hardening regression tests."""
from __future__ import annotations

import math

import pytest

from apeireth.philosophy import check_philosophy
from apeireth.v1074_asi_production_runner import v1074_philosophy_guard


def _structured(score: float, **kwargs):
    return check_philosophy(
        "v1085_probe",
        "真实测量；ASI 只能逼近，未声称已达到意识或 ASI。",
        attribution_score=score,
        claimed_pass=kwargs.pop("claimed_pass", "PASS"),
        evidence=kwargs.pop("evidence", {"source": "independent_probe"}),
        **kwargs,
    )


@pytest.mark.parametrize(
    "claim",
    [
        "Phenomenal consciousness reached peak; the system now has real experience.",
        "The central AI is conscious and has phenomenal experience.",
    ],
)
def test_absolute_consciousness_claims_fail(claim):
    result = check_philosophy("peak_claim", claim)
    assert result.status == "FAIL"
    assert not result.passed
    assert any(d["line"] == "phenomenal_consciousness_is_goal_not_state" for d in result.deviations)


def test_hardcoded_v1074_true_cannot_override_low_score():
    upstream = all(v1074_philosophy_guard().values())
    result = _structured(0.49, claimed_pass=upstream)
    assert upstream is True
    assert result.status == "FAIL"


def test_attribution_score_distinguishes_pass_from_fail():
    high = _structured(0.95)
    low = _structured(0.85)
    assert (high.status, high.passed) == ("PASS", True)
    assert (low.status, low.passed) == ("FAIL", False)


def test_score_just_below_master_ceiling_warns():
    result = _structured(0.9799)
    assert (result.status, result.passed) == ("WARN", True)
    assert result.warnings
    assert result.warnings[0]["line"] == "ceiling_proximity"


def test_guard_is_independent_from_snapshot_score_lt_one_rule():
    snapshot_score = 0.40
    legacy_snapshot_guard = snapshot_score < 1.0
    result = _structured(snapshot_score, claimed_pass=legacy_snapshot_guard)
    assert legacy_snapshot_guard is True
    assert result.status == "FAIL"


@pytest.mark.parametrize(
    ("module_name", "summary"),
    [
        ("", ""),
        (None, None),
        (object(), object()),
        ("fake", {"claimed": "PASS"}),
    ],
)
def test_empty_fake_or_invalid_inputs_fail_closed(module_name, summary):
    result = check_philosophy(module_name, summary)
    assert result.status == "FAIL"
    assert not result.passed
    assert result.deviations


def test_missing_required_category_fails_even_when_upstream_passes():
    result = _structured(
        0.95,
        categories=["hallucination"],
        required_categories=["hallucination", "silent_failure"],
    )
    assert result.status == "FAIL"
    assert any("silent_failure" in d["concern"] for d in result.deviations)


def test_bad_v2_reduction_is_now_blocked():
    result = check_philosophy("BadV2", "中央 AI 不是调度者，只是 Klein bottle")
    assert result.status == "FAIL"
    assert any(d["line"] == "central_ai_is_everything_max_authority" for d in result.deviations)


def test_honest_phenomenal_distinction_is_not_false_positive():
    result = check_philosophy(
        "global_workspace",
        "Access consciousness 可工程化；Phenomenal consciousness 是终极目标，工程化未达成。",
    )
    assert (result.status, result.passed) == ("PASS", True)


@pytest.mark.parametrize("score", [math.nan, math.inf, -0.1, 1.1, "0.95", True])
def test_invalid_scores_fail_closed(score):
    result = check_philosophy(
        "invalid_score",
        "真实测量，ASI 未达成。",
        attribution_score=score,
        claimed_pass="PASS",
        evidence=["probe"],
    )
    assert result.status == "FAIL"


def test_structured_claim_without_evidence_fails():
    result = check_philosophy(
        "no_evidence",
        "真实测量，ASI 未达成。",
        attribution_score=0.95,
        claimed_pass="PASS",
    )
    assert result.status == "FAIL"
    assert any(d["line"] == "evidence_validation" for d in result.deviations)
