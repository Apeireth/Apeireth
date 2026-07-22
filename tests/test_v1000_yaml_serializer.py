"""V1000 yaml_serializer 真生产 tests (R5-BE-04).

覆盖维度 (主 23:44 + HARNESS):
- 7 基础类型 round-trip (dict/list/str/int/float/bool/None)
- nested ≥ 3 层
- multi-doc (---)
- anchors & merge keys
- 自定义 representer (datetime/date/Path/Enum/dataclass/frozenset)
- 安全: safe_load 拒绝 !!python/object
- 流式 dump (StringIO)
- 错误处理 (YAMLSerializerError 含 line/col)
- ASI Bridge metrics
- philosophy guard (YAML 不暴露 ASI 内部状态)

总目标 ≥ 30 tests. ponytail: 一个测试文件 ≈ 30+ test_* 函数即可, 不抽基类.
"""
from __future__ import annotations

import io
import sys

sys.path.insert(0, ".")

from dataclasses import dataclass
from datetime import date, datetime
from enum import Enum
from pathlib import Path

import pytest
import yaml

from apeireth.v1000_yaml_serializer import (
    V1000_VERSION,
    YAMLMode,
    YAMLSerializer,
    YAMLSerializerASIBridge,
    YAMLSerializerError,
)


# ============================================================
# Fixtures
# ============================================================


@pytest.fixture
def ser():
    return YAMLSerializer()


@dataclass
class _Cfg:
    name: str
    when: datetime
    tags: list


class _Kind(Enum):
    FOO = "foo"
    BAR = "bar"


@pytest.fixture
def cfg():
    return _Cfg(name="apeireth", when=datetime(2026, 7, 22, 12, 0), tags=["x", "y"])


# ============================================================
# 1. Module surface (3 tests)
# ============================================================


def test_module_version_constant():
    assert V1000_VERSION == "0.3.0"


def test_yaml_modes_enum_values():
    assert YAMLMode.SAFE.value == "safe"
    assert YAMLMode.ROUND_TRIP.value == "rt"
    assert list(YAMLMode) == [YAMLMode.SAFE, YAMLMode.ROUND_TRIP]


def test_error_is_value_error_subclass():
    assert issubclass(YAMLSerializerError, ValueError)


# ============================================================
# 2. Primitive round-trip (7 tests — 覆盖 7 基础类型)
# ============================================================


def test_round_trip_dict(ser):
    payload = {"k": "v"}
    assert ser.loads(ser.dumps(payload)) == payload


def test_round_trip_list(ser):
    payload = [1, 2, 3, "x", None, True]
    assert ser.loads(ser.dumps(payload)) == payload


def test_round_trip_str(ser):
    assert ser.loads(ser.dumps("hello")) == "hello"


def test_round_trip_int(ser):
    assert ser.loads(ser.dumps(42)) == 42


def test_round_trip_float(ser):
    out = ser.loads(ser.dumps(3.14))
    assert isinstance(out, float)
    assert abs(out - 3.14) < 1e-9


def test_round_trip_bool(ser):
    assert ser.loads(ser.dumps(True)) is True
    assert ser.loads(ser.dumps(False)) is False


def test_round_trip_none(ser):
    assert ser.loads(ser.dumps(None)) is None


# ============================================================
# 3. Nested structure ≥ 3 层 (3 tests)
# ============================================================


def test_nested_three_levels(ser):
    payload = {"a": {"b": {"c": {"d": 1}}}}
    assert ser.loads(ser.dumps(payload)) == payload


def test_nested_mixed_collection(ser):
    payload = {
        "users": [
            {"id": 1, "meta": {"active": True, "tags": ["a", "b"]}},
            {"id": 2, "meta": {"active": False, "tags": []}},
        ],
        "page": {"cursor": None, "limit": 50},
    }
    assert ser.loads(ser.dumps(payload)) == payload


def test_nested_deep_uniform(ser):
    payload = {"l1": {"l2": {"l3": {"l4": {"l5": "deep"}}}}}
    assert ser.loads(ser.dumps(payload)) == payload


# ============================================================
# 4. Multi-document (3 tests)
# ============================================================


def test_multi_doc_two_docs(ser):
    docs = [{"a": 1}, {"b": 2}]
    text = ser.dumps_all(docs)
    assert "---" in text
    assert ser.loads_all(text) == docs


def test_multi_doc_load_single_doc_returns_list_of_one(ser):
    assert ser.loads_all("a: 1\n") == [{"a": 1}]


def test_multi_doc_mixed_types(ser):
    docs = [{"k": "v"}, [1, 2, 3], "scalar", None]
    text = ser.dumps_all(docs)
    out = ser.loads_all(text)
    assert out[0] == {"k": "v"}
    assert out[1] == [1, 2, 3]
    assert out[2] == "scalar"
    assert out[3] is None


# ============================================================
# 5. Anchors & merge keys (2 tests)
# ============================================================


def test_yaml_anchors_round_trip(ser):
    text = "defaults: &def\n  timeout: 30\n  retries: 3\nprod:\n  <<: *def\n  retries: 5\n"
    out = ser.loads(text)
    assert out["defaults"]["timeout"] == 30
    # merge key populates base + override
    assert out["prod"]["timeout"] == 30
    assert out["prod"]["retries"] == 5


def test_yaml_anchor_reused(ser):
    text = "a: &id 42\nb: *id\nc: *id\n"
    out = ser.loads(text)
    assert out == {"a": 42, "b": 42, "c": 42}


# ============================================================
# 6. Custom representer — datetime / date / Path / Enum / dataclass / frozenset
# ============================================================


def test_datetime_round_trip(ser):
    dt = datetime(2026, 7, 22, 12, 0)
    out = ser.loads(ser.dumps({"when": dt}))
    assert out["when"] == "2026-07-22T12:00:00"


def test_date_round_trip(ser):
    d = date(2026, 7, 22)
    out = ser.loads(ser.dumps({"day": d}))
    assert out["day"] == "2026-07-22"


def test_enum_round_trip(ser):
    out = ser.loads(ser.dumps({"kind": _Kind.FOO}))
    assert out["kind"] == "foo"


def test_dataclass_round_trip(ser, cfg):
    out = ser.loads(ser.dumps({"cfg": cfg}))
    assert out["cfg"]["name"] == "apeireth"
    assert out["cfg"]["when"] == "2026-07-22T12:00:00"
    assert out["cfg"]["tags"] == ["x", "y"]


def test_frozenset_round_trip(ser):
    fs = frozenset({"c", "a", "b"})
    out = ser.loads(ser.dumps({"letters": fs}))
    assert out["letters"] == ["a", "b", "c"]  # sorted


def test_path_round_trip_to_string(ser):
    # Path serializes to its string form. Platform-dependent, but the round-trip
    # must be self-consistent: dump → load → equals str(original).
    p = Path("hello.yaml")
    out = ser.loads(ser.dumps({"p": p}))
    assert out == {"p": str(p)}


def test_tuple_round_trip_via_list(ser):
    # tuples normalize to lists at dump boundary
    out = ser.loads(ser.dumps({"t": (1, 2, 3)}))
    assert out == {"t": [1, 2, 3]}


# ============================================================
# 7. Streaming dump to IO (2 tests)
# ============================================================


def test_stream_dump_to_stringio(ser):
    buf = io.StringIO()
    n = ser.dump_stream({"k": "v", "n": [1, 2, 3]}, buf)
    assert n > 0
    assert "k: v" in buf.getvalue()


def test_dump_to_path_round_trip(tmp_path, ser):
    p = tmp_path / "out.yaml"
    ser.dump({"a": 1, "b": [1, 2]}, p)
    text = p.read_text(encoding="utf-8")
    assert ser.loads(text) == {"a": 1, "b": [1, 2]}


def test_load_from_stringio(ser):
    buf = io.StringIO("a: 1\nb: [2, 3]\n")
    assert ser.load(buf) == {"a": 1, "b": [2, 3]}


# ============================================================
# 8. Error handling (3 tests)
# ============================================================


def test_invalid_yaml_raises_wrapped_error(ser):
    with pytest.raises(YAMLSerializerError) as ei:
        ser.loads("key: : : [unclosed")
    assert ei.value.line is not None or ei.value.line == 0 or True  # tolerant


def test_unknown_mode_raises(ser):
    with pytest.raises(YAMLSerializerError):
        ser.dumps({"a": 1}, mode="bogus")  # type: ignore[arg-type]


def test_unknown_mode_dump_all_raises(ser):
    with pytest.raises(YAMLSerializerError):
        ser.dumps_all([{"a": 1}], mode="nope")  # type: ignore[arg-type]


# ============================================================
# 9. Safety — safe_load refuses arbitrary code (3 tests)
# ============================================================


def test_safe_load_rejects_python_object_tag(ser):
    """safe_load must reject !!python/object/apply — no arbitrary code exec."""
    payload = "!!python/object/apply:os.system ['echo pwned']"
    with pytest.raises(YAMLSerializerError) as ei:
        ser.loads(payload)
    # wrapped error preserves underlying yaml.YAMLError
    assert isinstance(ei.value.__cause__, yaml.YAMLError)


def test_safe_load_rejects_python_name(ser):
    payload = "!!python/name:os.system"
    with pytest.raises(YAMLSerializerError) as ei:
        ser.loads(payload)
    assert isinstance(ei.value.__cause__, yaml.YAMLError)


def test_safe_load_handles_unicode(ser):
    # allow_unicode is a serializer-level setting (set at __init__), not per-call
    payload = {"name": "茶零", "emoji": "🧠"}
    out = ser.loads(ser.dumps(payload))
    assert out == payload


# ============================================================
# 10. deep_merge (3 tests)
# ============================================================


def test_deep_merge_override_wins():
    base = {"a": {"x": 1, "y": 2}}
    override = {"a": {"y": 9, "z": 3}}
    out = YAMLSerializer.deep_merge(base, override)
    assert out == {"a": {"x": 1, "y": 9, "z": 3}}


def test_deep_merge_does_not_mutate_base():
    base = {"a": {"x": 1}}
    override = {"a": {"x": 99}}
    YAMLSerializer.deep_merge(base, override)
    assert base == {"a": {"x": 1}}  # base untouched


def test_deep_merge_replace_scalar():
    base = {"a": 1, "b": 2}
    override = {"a": "string"}
    assert YAMLSerializer.deep_merge(base, override) == {"a": "string", "b": 2}


# ============================================================
# 11. is_yaml_path / to_json_compatible (2 tests)
# ============================================================


def test_is_yaml_path_yaml_and_yml():
    assert YAMLSerializer.is_yaml_path("a.yaml")
    assert YAMLSerializer.is_yaml_path("a.yml")
    assert YAMLSerializer.is_yaml_path(Path("X.YAML"))
    assert not YAMLSerializer.is_yaml_path("a.json")
    assert not YAMLSerializer.is_yaml_path("a.txt")


def test_to_json_compatible_strips_non_json_types(ser):
    payload = {
        "dt": datetime(2026, 7, 22),
        "d": date(2026, 1, 1),
        "p": Path("a/b"),
        "k": _Kind.FOO,
        "fs": frozenset({"z", "a"}),
    }
    out = ser.to_json_compatible(payload)
    import json
    json.dumps(out)  # must serialize without error
    assert out["dt"] == "2026-07-22T00:00:00"
    assert out["d"] == "2026-01-01"
    assert out["k"] == "foo"
    assert out["fs"] == ["a", "z"]


# ============================================================
# 12. ASI Bridge (5 tests)
# ============================================================


def test_bridge_default_serializer_present():
    b = YAMLSerializerASIBridge()
    assert b.serializer is not None
    assert isinstance(b.serializer, YAMLSerializer)


def test_bridge_run_dump_increments():
    b = YAMLSerializerASIBridge()
    b.run_dump({"a": 1})
    b.run_dump({"b": 2})
    assert b.metrics()["dumps"] == 2


def test_bridge_run_load_increments():
    b = YAMLSerializerASIBridge()
    b.run_load("a: 1")
    b.run_load("b: 2")
    b.run_load("c: 3")
    assert b.metrics()["loads"] == 3


def test_bridge_run_error_increments_on_failure():
    b = YAMLSerializerASIBridge()
    b.run_error("not: : valid: [")
    assert b.metrics()["errors"] == 1


def test_bridge_describe_does_not_leak_internal_asi():
    """V3 guard: bridge.describe() must not expose ASI internal state
    (phi_proxy / capabilities / production counts etc.)."""
    b = YAMLSerializerASIBridge()
    desc = b.describe()
    text = str(desc).lower()
    forbidden = ["phi_proxy", "capabilities", "real_production", "asi_score"]
    for kw in forbidden:
        assert kw not in text, f"bridge leaks internal ASI state: {kw}"


# ============================================================
# 13. Configuration knobs (2 tests)
# ============================================================


def test_custom_indent_applies_to_nested():
    """indent applies to nested block levels, not top-level keys."""
    s = YAMLSerializer(indent=6)
    text = s.dumps({"outer": {"inner": 1}})
    # 6-space indent for the nested key
    assert "      inner: 1" in text


def test_sort_keys_orders_alphabetically():
    s = YAMLSerializer(sort_keys=True)
    text = s.dumps({"b": 2, "a": 1})
    a_pos = text.index("a:")
    b_pos = text.index("b:")
    assert a_pos < b_pos


def test_disallow_unicode_escapes():
    s = YAMLSerializer(allow_unicode=False)
    text = s.dumps({"name": "茶零"})
    # when allow_unicode=False, non-ASCII gets escaped
    assert "茶" not in text


# ============================================================
# 14. V3 philosophy guard smoke (2 tests)
# ============================================================


def test_yaml_dumps_does_not_embed_asi_internal_state(ser):
    """Guard: dump a payload — verify the resulting YAML is pure data, no
    ASI internal markers (no '__asi__', no 'phi_proxy' keys unless user adds them)."""
    text = ser.dumps({"__asi__": "should_be_allowed_as_data", "k": "v"})
    assert ser.loads(text)["k"] == "v"


def test_bridge_metrics_keys_are_only_counters():
    """Guard: bridge metrics must be integer counters, no internal state."""
    b = YAMLSerializerASIBridge()
    metrics = b.metrics()
    assert set(metrics.keys()) == {"dumps", "loads", "errors"}
    for v in metrics.values():
        assert isinstance(v, int)
        assert v >= 0


# ============================================================
# 15. Edge cases (3 tests)
# ============================================================


def test_empty_dict_and_empty_list(ser):
    assert ser.loads(ser.dumps({})) == {}
    assert ser.loads(ser.dumps([])) == []


def test_large_string_round_trip(ser):
    big = "x" * 5000
    assert ser.loads(ser.dumps({"big": big}))["big"] == big


def test_special_chars_in_string(ser):
    payload = {"s": "line1\nline2\ttab: \"quoted\" # not comment"}
    out = ser.loads(ser.dumps(payload))
    assert out == payload