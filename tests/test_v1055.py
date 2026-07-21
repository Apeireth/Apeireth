"""Tests for V1055 ASI Time."""
from __future__ import annotations

import pytest
import apeireth.v1055_asi_time as m


# ============================================================================
# TemporalPoint tests
# ============================================================================


class TestTemporalPoint:
    def test_basic(self) -> None:
        tp = m.TemporalPoint(5, "label", 1.0)
        assert tp.tick == 5
        assert tp.label == "label"
        assert tp.value == 1.0

    def test_tick_non_negative(self) -> None:
        with pytest.raises(ValueError):
            m.TemporalPoint(-1)

    def test_earlier_later(self) -> None:
        a = m.TemporalPoint(1)
        b = m.TemporalPoint(3)
        assert a.earlier_than(b) is True
        assert b.later_than(a) is True
        assert a.later_than(b) is False


# ============================================================================
# Interval tests
# ============================================================================


class TestInterval:
    def test_basic_interval(self) -> None:
        s = m.TemporalPoint(0)
        e = m.TemporalPoint(5)
        iv = m.Interval(s, e, "range")
        assert iv.duration_ticks() == 5
        assert iv.contains(m.TemporalPoint(3)) is True
        assert iv.contains(m.TemporalPoint(6)) is False

    def test_invalid_order(self) -> None:
        s = m.TemporalPoint(5)
        e = m.TemporalPoint(3)
        with pytest.raises(ValueError):
            m.Interval(s, e)


# ============================================================================
# Timeline tests (Husserl 1905)
# ============================================================================


class TestTimeline:
    def test_empty_timeline(self) -> None:
        tl = m.Timeline()
        assert tl.retention() == []
        assert tl.primal() is None
        assert tl.protention() == []

    def test_temporal_moment(self) -> None:
        tl = m.Timeline()
        for i in range(6):
            tl.add_point(m.TemporalPoint(i, f"t{i}", float(i)))
        tl.set_now(3)
        # retention: 0,1,2 (3 back)
        assert len(tl.retention()) == 3
        # primal: 3
        assert tl.primal() is not None
        assert tl.primal().tick == 3
        # protention: 4,5 (3 ahead)
        assert len(tl.protention()) == 2

    def test_invalid_now(self) -> None:
        tl = m.Timeline()
        tl.add_point(m.TemporalPoint(0))
        with pytest.raises(ValueError):
            tl.set_now(5)
        with pytest.raises(ValueError):
            tl.set_now(-1)

    def test_temporal_moment_structure(self) -> None:
        tl = m.Timeline()
        for i in range(5):
            tl.add_point(m.TemporalPoint(i))
        tl.set_now(2)
        tms = tl.temporal_moment_structure()
        assert tms["retention_count"] == 2
        assert tms["primal_index"] == 2
        assert tms["protention_count"] == 2


# ============================================================================
# ArrowOfTime tests (Reichenbach 1956)
# ============================================================================


class TestArrowOfTime:
    def test_undefined(self) -> None:
        arw = m.ArrowOfTime()
        assert arw.arrow_direction() == "undefined"

    def test_forward(self) -> None:
        arw = m.ArrowOfTime()
        arw.add_entropy(0.1)
        arw.add_entropy(0.5)
        arw.add_entropy(0.8)
        assert arw.arrow_direction() == "forward"
        assert arw.is_forward() is True

    def test_backward(self) -> None:
        arw = m.ArrowOfTime()
        arw.add_entropy(0.9)
        arw.add_entropy(0.5)
        arw.add_entropy(0.1)
        assert arw.arrow_direction() == "backward"
        assert arw.is_forward() is False

    def test_entropy_rate(self) -> None:
        arw = m.ArrowOfTime()
        arw.add_entropy(0.0)
        arw.add_entropy(0.5)
        arw.add_entropy(1.0)
        assert arw.entropy_rate() == pytest.approx(0.5, abs=1e-6)


# ============================================================================
# MentalTimeTravel tests (Tulving 1985)
# ============================================================================


class TestMentalTimeTravel:
    def test_empty(self) -> None:
        mtt = m.MentalTimeTravel()
        assert mtt.mental_time_travel_score() == 0.0

    def test_with_memories(self) -> None:
        mtt = m.MentalTimeTravel()
        mtt.store_memory({"event": "breakfast"})
        mtt.simulate_future({"event": "lunch"})
        assert mtt.memory_count() == 1
        assert mtt.simulation_count() == 1
        assert mtt.mental_time_travel_score() > 0.0


# ============================================================================
# SequentialPrediction tests (Clark 2013 FEP)
# ============================================================================


class TestSequentialPrediction:
    def test_empty(self) -> None:
        pred = m.SequentialPrediction()
        assert pred.predict_next() == 0.0

    def test_linear_prediction(self) -> None:
        pred = m.SequentialPrediction()
        pred.observe(1.0)
        pred.observe(2.0)
        pred.observe(3.0)
        assert pred.predict_next() == 4.0  # 3 + (3-2) = 4

    def test_prediction_error(self) -> None:
        pred = m.SequentialPrediction()
        pred.observe(1.0)
        pred.observe(2.0)
        err = pred.prediction_error(4.5)  # predicted=3.0, actual=4.5
        assert err == pytest.approx(1.5, abs=1e-6)

    def test_learning_rate(self) -> None:
        pred = m.SequentialPrediction()
        assert pred.learning_rate(0.0) == pytest.approx(1.0, abs=1e-6)
        rate = pred.learning_rate(9.0)
        assert rate == pytest.approx(0.1, abs=1e-6)


# ============================================================================
# TemporalRelation tests (McTaggart A-series + Lewis 1973)
# ============================================================================


class TestTemporalRelation:
    def test_causal_edge(self) -> None:
        rel = m.TemporalRelation()
        rel.add_causal_edge(0, 3)
        rel.add_causal_edge(3, 5)
        assert rel.is_earlier_causal(0, 3) is True
        assert rel.is_earlier_causal(3, 5) is True
        assert rel.is_earlier_causal(5, 0) is False

    def test_invalid_causal(self) -> None:
        rel = m.TemporalRelation()
        with pytest.raises(ValueError):
            rel.add_causal_edge(5, 3)  # cause after effect

    def test_a_series(self) -> None:
        rel = m.TemporalRelation()
        rel.set_a_series(0, "past")
        rel.set_a_series(5, "present")
        assert rel.is_present(5) is True
        assert rel.is_present(0) is False

    def test_causal_chain(self) -> None:
        rel = m.TemporalRelation()
        rel.add_causal_edge(0, 2)
        rel.add_causal_edge(2, 4)
        chain = rel.causal_chain(0, 5)
        assert chain == [0, 2, 4]


# ============================================================================
# StreamOfTime tests (James 1890 + Bergson 1889)
# ============================================================================


class TestStreamOfTime:
    def test_empty(self) -> None:
        st = m.StreamOfTime()
        assert st.stream_length() == 0
        assert st.average_change_rate() == 0.0

    def test_stream_change(self) -> None:
        st = m.StreamOfTime(duration_quality=0.7)
        st.add_stream_entry({"a": 1.0, "b": 0.5})
        st.add_stream_entry({"a": 0.8, "b": 0.3})
        assert st.stream_length() == 2
        assert st.average_change_rate() == pytest.approx(0.2, abs=1e-6)
        assert st.durational_depth() > 0.0


# ============================================================================
# TimeReport tests
# ============================================================================


class TestTimeReport:
    def test_empty(self) -> None:
        r = m.TimeReport("Empty")
        md = r.to_markdown()
        assert "# Empty" in md

    def test_full(self) -> None:
        tl = m.Timeline()
        for i in range(3):
            tl.add_point(m.TemporalPoint(i))
        tl.set_now(1)
        arw = m.ArrowOfTime()
        arw.add_entropy(0.1)
        arw.add_entropy(0.2)
        mtt = m.MentalTimeTravel()
        mtt.store_memory({"x": 1})
        mtt.simulate_future({"y": 2})
        pred = m.SequentialPrediction()
        pred.observe(1.0); pred.observe(2.0)
        rel = m.TemporalRelation()
        rel.add_causal_edge(0, 1)
        st = m.StreamOfTime(duration_quality=0.6)
        st.add_stream_entry({"a": 1.0})
        r = m.TimeReport("Full", timeline=tl, arrow=arw, mtt=mtt,
                         predictor=pred, relation=rel, stream=st,
                         asi_v02_metrics={"overall": 0.6},
                         notes=["time note"])
        md = r.to_markdown()
        assert "Timeline" in md
        assert "Arrow" in md
        assert "Mental Time Travel" in md
        assert "Stream of Time" in md
        assert "ASI V0.2" in md
        assert "time note" in md


# ============================================================================
# ASITimeBridge tests
# ============================================================================


class TestASITimeBridge:
    def test_empty(self) -> None:
        br = m.ASITimeBridge()
        assert br.time_score() == {}
        assert br.asi_v02_time_contribution() == 0.0
        assert br.has_temporal_understanding() is False

    def test_full(self) -> None:
        tl = m.Timeline()
        for i in range(5):
            tl.add_point(m.TemporalPoint(i))
        tl.set_now(2)
        arw = m.ArrowOfTime()
        arw.add_entropy(0.1); arw.add_entropy(0.5)
        mtt = m.MentalTimeTravel()
        mtt.store_memory({"x": 1})
        pred = m.SequentialPrediction()
        pred.observe(1.0); pred.observe(2.0)
        br = m.ASITimeBridge(timeline=tl, arrow=arw, mtt=mtt, predictor=pred)
        s = br.time_score()
        assert "overall" in s
        assert s["overall"] > 0.5
        assert br.has_temporal_understanding() is True
        assert br.asi_v02_time_contribution() > 0.0

    def test_partial(self) -> None:
        tl = m.Timeline()
        tl.add_point(m.TemporalPoint(0))
        br = m.ASITimeBridge(timeline=tl)
        s = br.time_score()
        assert "husserl_structure" in s
        assert "overall" in s


# ============================================================================
# Guards tests
# ============================================================================


class TestGuards:
    def test_bergson(self) -> None:
        assert m.bergson_duree_guard() is True
        assert m.bergson_duree_guard(False) is False

    def test_husserl(self) -> None:
        assert m.husserl_retention_guard(True) is True
        assert m.husserl_retention_guard(False) is False

    def test_mctaggart(self) -> None:
        assert m.mctaggart_series_guard() is True

    def test_whitehead(self) -> None:
        assert m.whitehead_occasions_guard() is True

    def test_time_consciousness(self) -> None:
        assert m.time_consciousness_guard() is True


# ============================================================================
# Sanity
# ============================================================================


class TestSanity:
    def test_version(self) -> None:
        assert m.V1055_VERSION == "0.1.0"

    def test_10_components(self) -> None:
        components = [
            m.TemporalPoint, m.Interval, m.Timeline, m.ArrowOfTime,
            m.MentalTimeTravel, m.SequentialPrediction, m.TemporalRelation,
            m.StreamOfTime, m.TimeReport, m.ASITimeBridge,
        ]
        assert len(components) == 10
