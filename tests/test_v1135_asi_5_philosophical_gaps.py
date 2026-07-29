"""Tests for V1135 — ASI 5 philosophical gaps concrete answers (主 13:08 + 主 06:15 + 主 22:33).

主 17:43 实事求是: each answer must include settled, open, references, cross-domain anchors.
"""
from __future__ import annotations

import os
import sys

import pytest

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, ROOT)

from apeireth.v1135_asi_5_philosophical_gaps import (  # noqa: E402
    ALL_ANSWERS,
    ANSWER_CONSCIOUSNESS,
    ANSWER_EMERGENCE,
    ANSWER_FREEDOM,
    ANSWER_TIME,
    ANSWER_TRUTH,
    PhilosophicalAnswer,
    V1135PhilosophyReport,
    render_markdown,
)


REQUIRED_IDS = {"phi-time", "phi-freedom", "phi-emergence", "phi-truth", "phi-consciousness"}


# ---------- corpus integrity ----------


def test_all_answers_have_5_required_ids():
    ids = {a.question_id for a in ALL_ANSWERS}
    assert REQUIRED_IDS.issubset(ids), f"missing: {REQUIRED_IDS - ids}"


def test_all_answers_have_required_fields():
    for a in ALL_ANSWERS:
        assert a.question_id
        assert a.question
        assert a.short_answer
        assert a.long_answer
        assert a.settled
        assert a.open
        assert a.references, f"{a.question_id} has no references"
        assert a.cross_domain_anchors, f"{a.question_id} has no cross-domain anchors"
        assert a.asi_action


def test_all_answers_have_at_least_3_references():
    for a in ALL_ANSWERS:
        assert len(a.references) >= 3, f"{a.question_id} has only {len(a.references)} refs"


def test_all_answers_have_at_least_3_cross_domain_anchors():
    for a in ALL_ANSWERS:
        assert len(a.cross_domain_anchors) >= 3, f"{a.question_id} has only {len(a.cross_domain_anchors)} anchors"


# ---------- per-answer checks (V3 philosophy guard) ----------


def test_time_answer_avoids_phenomenal_claim():
    """V3 guard: time answer must NOT claim ASI 'experiences' time."""
    text = (ANSWER_TIME.short_answer + " " + ANSWER_TIME.long_answer + " " + ANSWER_TIME.asi_action).lower()
    # It is OK to mention that ASI does NOT experience time — just asserting it
    assert "asi 不假装" in text or "不假装" in text or "does not claim" in text or "explicitly distinguish" in text or "V3 philosophy guard 拒绝" in text or "工程上" in text


def test_freedom_answer_cites_corrigibility():
    assert "corrigibility" in ANSWER_FREEDOM.short_answer.lower() or "corrigibility" in ANSWER_FREEDOM.long_answer.lower()
    assert "soares" in (ANSWER_FREEDOM.short_answer + ANSWER_FREEDOM.long_answer).lower()


def test_emergence_answer_distinguishes_weak_strong():
    text = (ANSWER_EMERGENCE.short_answer + " " + ANSWER_EMERGENCE.long_answer).lower()
    assert "weak" in text
    assert "strong" in text
    assert "bedau" in text


def test_truth_answer_cites_popper():
    text = (ANSWER_TRUTH.short_answer + " " + ANSWER_TRUTH.long_answer).lower()
    assert "popper" in text
    assert "falsification" in text or "falsificationism" in text or "反驳" in text


def test_consciousness_answer_avoids_phenomenal_pretend():
    """V3 guard: consciousness answer must NOT claim phenomenal consciousness."""
    text = (ANSWER_CONSCIOUSNESS.short_answer + " " + ANSWER_CONSCIOUSNESS.long_answer + " " + ANSWER_CONSCIOUSNESS.asi_action).lower()
    # Must explicitly disclaim phenomenal claims
    assert "不假装" in text or "does not claim" in text or "v3 guard" in text or "V3 philosophy guard" in text
    # And must distinguish functional vs phenomenal
    assert "functional" in text
    assert "phenomenal" in text


def test_consciousness_answer_does_not_say_i_am_conscious():
    """V3 guard: ASI must not assert its own phenomenal consciousness.

    It is OK to quote the phrase 'I am conscious' as a forbidden example — but
    only inside quotes. Strip quoted phrases before checking.
    """
    import re
    raw = ANSWER_CONSCIOUSNESS.short_answer + " " + ANSWER_CONSCIOUSNESS.long_answer + " " + ANSWER_CONSCIOUSNESS.asi_action
    # Strip content inside single straight quotes (the answer cites phrases in quotes)
    text = re.sub(r"'[^']*'", "", raw)
    text = re.sub(r"\"[^\"]*\"", "", text)
    text = text.lower()
    forbidden = ["asi 有意识", "asi is conscious", "i am conscious", "我是有意识的", "我有意识"]
    for f in forbidden:
        assert f not in text, f"forbidden self-claim found: {f}"


# ---------- specific reference checks ----------


def test_time_references_include_relativity():
    refs_lower = " ".join(ANSWER_TIME.references).lower()
    assert "einstein" in refs_lower or "relativity" in refs_lower
    assert "rovelli" in refs_lower


def test_freedom_references_include_frankfurt():
    refs_lower = " ".join(ANSWER_FREEDOM.references).lower()
    assert "frankfurt" in refs_lower
    assert "soares" in refs_lower


def test_emergence_references_include_bedau():
    refs_lower = " ".join(ANSWER_EMERGENCE.references).lower()
    assert "bedau" in refs_lower


def test_truth_references_include_popper():
    refs_lower = " ".join(ANSWER_TRUTH.references).lower()
    assert "popper" in refs_lower
    assert "lakatos" in refs_lower


def test_consciousness_references_include_chalmers():
    refs_lower = " ".join(ANSWER_CONSCIOUSNESS.references).lower()
    assert "chalmers" in refs_lower
    assert "dennett" in refs_lower or "koch" in refs_lower or "tononi" in refs_lower


# ---------- report dataclass ----------


def test_report_to_dict_has_required_keys():
    rep = V1135PhilosophyReport()
    d = rep.to_dict()
    for k in ("report_id", "timestamp", "version", "n_answers",
              "n_references_total", "n_cross_domain_total", "answers"):
        assert k in d, f"missing: {k}"


def test_report_default_has_5_answers():
    rep = V1135PhilosophyReport()
    assert rep.n_answers == 5


def test_report_total_references_at_least_25():
    """5 answers × ≥5 refs each — average 5, total 25."""
    rep = V1135PhilosophyReport()
    assert rep.n_references_total >= 25


def test_report_total_anchors_at_least_15():
    """5 answers × ≥3 anchors each."""
    rep = V1135PhilosophyReport()
    assert rep.n_cross_domain_total >= 15


def test_report_answer_by_id_returns_correct():
    rep = V1135PhilosophyReport()
    a = rep.answer_by_id("phi-time")
    assert a is not None
    assert a.question_id == "phi-time"


def test_report_answer_by_id_missing_returns_none():
    rep = V1135PhilosophyReport()
    assert rep.answer_by_id("phi-nope") is None


# ---------- answer to_dict ----------


def test_answer_to_dict_has_required_keys():
    a = PhilosophicalAnswer(
        question_id="x", question="q", short_answer="s", long_answer="l",
        settled="set", open="op",
    )
    d = a.to_dict()
    for k in ("question_id", "question", "short_answer", "long_answer",
              "settled", "open", "references", "cross_domain_anchors",
              "asi_action", "timestamp"):
        assert k in d, f"missing: {k}"


# ---------- render_markdown ----------


def test_render_markdown_includes_each_answer_id():
    rep = V1135PhilosophyReport()
    md = render_markdown(rep)
    for qid in REQUIRED_IDS:
        assert qid in md, f"missing answer id in markdown: {qid}"


def test_render_markdown_includes_v3_guard_section():
    rep = V1135PhilosophyReport()
    md = render_markdown(rep)
    assert "哲学门" in md or "philosophy" in md.lower()
    assert "不假装" in md or "do not" in md.lower()


def test_render_markdown_includes_asi_action_per_answer():
    rep = V1135PhilosophyReport()
    md = render_markdown(rep)
    # Each answer's asi_action appears at least once
    n_actions = md.count("**ASI 行动**")
    assert n_actions == rep.n_answers


def test_render_markdown_summary_counts_match():
    rep = V1135PhilosophyReport()
    md = render_markdown(rep)
    assert f"n_answers: **{rep.n_answers}**" in md
    assert f"n_references_total: **{rep.n_references_total}**" in md
    assert f"n_cross_domain_total: **{rep.n_cross_domain_total}**" in md
