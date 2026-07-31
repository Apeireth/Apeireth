"""V1136/V1128 ASI V0.5 truth adapter for dashboard and UI.

The adapter deliberately keeps measurement separate from presentation: V1136 owns the
3-Dim score, V1128 owns the compatible 18-Dim breakdown, and this module never invents
a score when either producer is unavailable.
"""
from __future__ import annotations

import json
from pathlib import Path
from typing import Any, Callable, Dict, Mapping, Optional

from apeireth.v1128_r10_multi_agent_integration import (
    VERSION as V1128_VERSION,
    V1128MultiAgentIntegrationProtocol,
)
from apeireth.v1136_asi_v05_3dim_real_measurement import (
    VERSION as V1136_VERSION,
    measure_v05_3dims,
)

SCHEMA_VERSION = "1.0"
DEFAULT_SNAPSHOT_PATH = Path(__file__).resolve().parent.parent / "artifacts" / "asi_snapshot.json"


def _legacy(snapshot_path: Path, v04_score: float) -> Dict[str, Any]:
    legacy: Dict[str, Any] = {
        "v03": {"status": "empty", "value": None, "source": str(snapshot_path)},
        "v04": {"status": "ok", "value": round(float(v04_score), 4), "source": "V1136 input"},
    }
    try:
        raw = json.loads(snapshot_path.read_text(encoding="utf-8"))
        value = raw.get("v03_score")
        if isinstance(value, (int, float)):
            legacy["v03"] = {
                "status": "ok",
                "value": round(float(value), 4),
                "source": f"{snapshot_path}#{raw.get('snapshot_id', 'unknown')}",
            }
    except (OSError, ValueError, TypeError):
        pass
    return legacy


def _empty_state(legacy: Mapping[str, Any], message: str = "No V1136 measurement data") -> Dict[str, Any]:
    return {
        "schema_version": SCHEMA_VERSION,
        "status": "empty",
        "message": message,
        "legacy": dict(legacy),
        "v05": {"status": "empty", "value": None},
        "dimensions_3": {"status": "empty", "count": 0, "values": {}, "details": {}},
        "dimensions_18": {"status": "empty", "count": 0, "values": {}, "source": "V1128"},
        "source": {"engine": "V1136", "version": None, "measured_at": None},
    }


def build_dashboard_state(
    v1136_payload: Optional[Mapping[str, Any]],
    v1128_payload: Optional[Mapping[str, Any]] = None,
    *,
    legacy: Optional[Mapping[str, Any]] = None,
) -> Dict[str, Any]:
    """Validate producer payloads and create the public dashboard contract."""
    legacy_data = dict(legacy or {})
    if not v1136_payload:
        return _empty_state(legacy_data)

    producer_version = v1136_payload.get("version")
    if producer_version != V1136_VERSION:
        state = _empty_state(legacy_data, f"V1136 version mismatch: expected {V1136_VERSION}, got {producer_version!r}")
        state["status"] = "version_mismatch"
        state["v05"]["status"] = "version_mismatch"
        state["dimensions_3"]["status"] = "version_mismatch"
        state["source"]["version"] = producer_version
        return state

    required = ("v05_total_v1136", "v04_score", "continuity", "autonomy", "transferability")
    missing = [key for key in required if not isinstance(v1136_payload.get(key), (int, float))]
    if missing:
        state = _empty_state(legacy_data, f"V1136 measurement missing numeric fields: {', '.join(missing)}")
        state["status"] = "measurement_failed"
        state["v05"]["status"] = "measurement_failed"
        state["dimensions_3"]["status"] = "measurement_failed"
        state["source"]["version"] = producer_version
        return state

    dim3 = {
        "continuity": float(v1136_payload["continuity"]),
        "autonomy": float(v1136_payload["autonomy"]),
        "transferability": float(v1136_payload["transferability"]),
    }
    details = {
        name: dict(v1136_payload.get(f"{name}_detail") or {})
        for name in dim3
    }
    state: Dict[str, Any] = {
        "schema_version": SCHEMA_VERSION,
        "status": "ok",
        "message": "V1136 live measurement loaded",
        "legacy": legacy_data,
        "v05": {
            "status": "ok",
            "value": float(v1136_payload["v05_total_v1136"]),
            "v04_input": float(v1136_payload["v04_score"]),
            "formula": "v04*0.85 + continuity*0.05 + autonomy*0.05 + transferability*0.05",
        },
        "dimensions_3": {"status": "ok", "count": 3, "values": dim3, "details": details},
        "dimensions_18": {"status": "empty", "count": 0, "values": {}, "source": "V1128"},
        "source": {
            "engine": "V1136",
            "version": producer_version,
            "measured_at": v1136_payload.get("timestamp"),
        },
    }

    if not v1128_payload:
        state["status"] = "measurement_failed"
        state["message"] = "V1136 succeeded; V1128 18-Dim measurement unavailable"
        state["dimensions_18"]["status"] = "measurement_failed"
        return state
    if v1128_payload.get("version") != V1128_VERSION:
        state["status"] = "version_mismatch"
        state["message"] = f"V1128 version mismatch: expected {V1128_VERSION}, got {v1128_payload.get('version')!r}"
        state["dimensions_18"]["status"] = "version_mismatch"
        return state

    form = v1128_payload.get("v05_18_form") or {}
    dims = form.get("dims") if isinstance(form, Mapping) else None
    if not isinstance(dims, Mapping) or len(dims) != 18 or not all(isinstance(v, (int, float)) for v in dims.values()):
        state["status"] = "measurement_failed"
        state["message"] = f"V1128 returned {len(dims) if isinstance(dims, Mapping) else 0}/18 numeric dimensions"
        state["dimensions_18"]["status"] = "measurement_failed"
        return state
    state["dimensions_18"] = {
        "status": "ok",
        "count": 18,
        "values": {str(k): float(v) for k, v in dims.items()},
        "total": form.get("v05_18_total"),
        "source": "V1128 compatibility measurement",
    }
    return state


def measure_dashboard_state(
    *,
    v04_score: float = 0.8538,
    snapshot_path: Path = DEFAULT_SNAPSHOT_PATH,
    v1136_measure: Callable[..., Any] = measure_v05_3dims,
    v1128_measure: Optional[Callable[..., Mapping[str, Any]]] = None,
) -> Dict[str, Any]:
    """Run real producers and return honest failure states instead of fallback scores."""
    legacy = _legacy(Path(snapshot_path), v04_score)
    try:
        measured = v1136_measure(v04_score=v04_score)
        payload = measured.to_dict() if hasattr(measured, "to_dict") else dict(measured)
        payload["version"] = V1136_VERSION
    except Exception as exc:  # measurement boundary: expose failure, never fabricate a score
        state = _empty_state(legacy, f"V1136 measurement failed: {type(exc).__name__}: {exc}")
        state["status"] = "measurement_failed"
        state["v05"]["status"] = "measurement_failed"
        state["dimensions_3"]["status"] = "measurement_failed"
        return state

    try:
        if v1128_measure is None:
            protocol = V1128MultiAgentIntegrationProtocol()
            detail18 = protocol.evaluate_r10_week(v04_score=v04_score)
        else:
            detail18 = v1128_measure(v04_score=v04_score)
    except Exception as exc:
        state = build_dashboard_state(payload, legacy=legacy)
        state["message"] = f"V1136 succeeded; V1128 measurement failed: {type(exc).__name__}: {exc}"
        return state
    return build_dashboard_state(payload, detail18, legacy=legacy)


def render_streamlit_v05(st: Any, state: Mapping[str, Any]) -> None:
    """Render the view-model with duck-typed Streamlit (also usable by E2E tests)."""
    st.subheader("ASI V0.5 真测 — V1136")
    status = state.get("status")
    if status != "ok":
        getattr(st, "warning" if status == "empty" else "error")(state.get("message", status))
    legacy = state.get("legacy", {})
    for version in ("v03", "v04"):
        item = legacy.get(version, {})
        st.metric(version.upper(), "N/A" if item.get("value") is None else f"{item['value']:.4f}", item.get("status", "empty"))
    v05 = state.get("v05", {})
    st.metric("V0.5 (V1136 live)", "N/A" if v05.get("value") is None else f"{v05['value']:.4f}", v05.get("status", "empty"))
    st.write("3-Dim detail", state.get("dimensions_3", {}))
    st.write("18-Dim compatibility detail", state.get("dimensions_18", {}))


__all__ = [
    "SCHEMA_VERSION", "build_dashboard_state", "measure_dashboard_state", "render_streamlit_v05"
]
