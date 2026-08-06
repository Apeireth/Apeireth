"""Contract and real E2E tests for the V1136 dashboard truth adapter."""
from __future__ import annotations

import json
from pathlib import Path

import pytest

from apeireth.v1035_streamlit import V1035Streamlit
from apeireth.v1128_r10_multi_agent_integration import VERSION as V1128_VERSION
from apeireth.v1136_asi_v05_3dim_real_measurement import VERSION as V1136_VERSION
from apeireth.v1136_dashboard import (
    build_dashboard_state,
    measure_dashboard_state,
    render_streamlit_v05,
)


def _v1136_payload() -> dict:
    return {
        "version": V1136_VERSION,
        "timestamp": 123.0,
        "v04_score": 0.8,
        "continuity": 0.7,
        "autonomy": 0.6,
        "transferability": 0.5,
        "v05_total_v1136": 0.77,
        "continuity_detail": {"sub_scores": {"identity": 0.7}},
        "autonomy_detail": {"sub_scores": {"router": 0.6}},
        "transferability_detail": {"sub_scores": {"model": 0.5}},
    }


def _v1128_payload() -> dict:
    return {
        "version": V1128_VERSION,
        "v05_18_form": {
            "dims": {f"dim_{i}": i / 20 for i in range(18)},
            "v05_18_total": 0.75,
        },
    }


def test_empty_data_is_explicit_and_keeps_legacy() -> None:
    state = build_dashboard_state(None, legacy={"v03": {"status": "ok", "value": 0.7}})
    assert state["status"] == "empty"
    assert state["v05"]["value"] is None
    assert state["legacy"]["v03"]["value"] == 0.7


def test_v1136_version_mismatch_has_no_score() -> None:
    payload = _v1136_payload()
    payload["version"] = "99.0"
    state = build_dashboard_state(payload, _v1128_payload())
    assert state["status"] == "version_mismatch"
    assert state["v05"]["value"] is None
    assert "expected" in state["message"]


def test_missing_measurement_fields_fail_honestly() -> None:
    payload = _v1136_payload()
    del payload["autonomy"]
    state = build_dashboard_state(payload, _v1128_payload())
    assert state["status"] == "measurement_failed"
    assert state["dimensions_3"]["values"] == {}


def test_incomplete_18_dim_payload_is_not_padded() -> None:
    detail18 = _v1128_payload()
    detail18["v05_18_form"]["dims"].pop("dim_17")
    state = build_dashboard_state(_v1136_payload(), detail18)
    assert state["status"] == "measurement_failed"
    assert state["dimensions_18"]["count"] == 0
    assert "17/18" in state["message"]


def test_measurement_exception_exposes_failure_without_fake_value(tmp_path: Path) -> None:
    def fail(**_: object) -> object:
        raise RuntimeError("sensor offline")

    state = measure_dashboard_state(snapshot_path=tmp_path / "missing.json", v1136_measure=fail)
    assert state["status"] == "measurement_failed"
    assert state["v05"]["value"] is None
    assert "sensor offline" in state["message"]


def test_v1128_exception_keeps_v1136_value_but_marks_partial_failure() -> None:
    class Measurement:
        def to_dict(self) -> dict:
            payload = _v1136_payload()
            payload.pop("version")
            return payload

    def v1136(**_: object) -> Measurement:
        return Measurement()

    def v1128(**_: object) -> dict:
        raise ConnectionError("backend down")

    state = measure_dashboard_state(v1136_measure=v1136, v1128_measure=v1128)
    assert state["status"] == "measurement_failed"
    assert state["v05"]["value"] == _v1136_payload()["v05_total_v1136"]
    assert state["dimensions_18"]["status"] == "measurement_failed"
    assert "backend down" in state["message"]


def test_real_v1136_to_dashboard_end_to_end() -> None:
    """No mocks: execute V1136 + V1128 and verify the dashboard formula/data flow."""
    state = measure_dashboard_state()
    assert state["status"] == "ok"
    assert state["dimensions_3"]["count"] == 3
    assert state["dimensions_18"]["count"] == 18
    values = state["dimensions_3"]["values"]
    expected = (
        state["v05"]["v04_input"] * 0.85
        + values["continuity"] * 0.05
        + values["autonomy"] * 0.05
        + values["transferability"] * 0.05
    )
    assert state["v05"]["value"] == pytest.approx(expected, abs=5e-5)
    assert state["legacy"]["v03"]["status"] == "ok"
    json.dumps(state, ensure_ascii=False)


class _FakeStreamlit:
    def __init__(self) -> None:
        self.calls: list[tuple] = []

    def __getattr__(self, name: str):
        def call(*args, **kwargs):
            self.calls.append((name, args, kwargs))
        return call


def test_streamlit_renderer_shows_na_for_empty_data() -> None:
    fake = _FakeStreamlit()
    render_streamlit_v05(fake, build_dashboard_state(None))
    metrics = [args for name, args, _ in fake.calls if name == "metric"]
    assert any("N/A" in args for args in metrics)
    assert any(name == "warning" for name, _, _ in fake.calls)


def test_existing_streamlit_ui_calls_live_adapter_and_has_no_static_v05() -> None:
    app = V1035Streamlit().render_app()
    assert "measure_dashboard_state" in app
    assert "render_streamlit_v05" in app
    forbidden_static = "0." + "8595"
    assert forbidden_static not in app


def test_generated_streamlit_app_executes_real_measurement_end_to_end() -> None:
    """Official Streamlit runner executes the page, not merely an HTTP health shell."""
    streamlit_testing = pytest.importorskip("streamlit.testing.v1")
    from apeireth.v1134_streamlit_real_startup import render_streamlit_app

    app = streamlit_testing.AppTest.from_string(render_streamlit_app(["Home"])).run(timeout=60)
    assert not app.exception
    metrics = {metric.label: metric.value for metric in app.metric}
    assert metrics["V03"] != "N/A"
    assert metrics["V04"] != "N/A"
    assert metrics["V0.5 (V1136 live)"] != "N/A"
