"""V1027 真生产 tests (主 23:44 干到底)."""
from __future__ import annotations
import sys; sys.path.insert(0, '.')
import pytest
from apeireth.v1027_validator import (
    V1027_VERSION, ValidationError, validate, V1027Validator,
)


class TestV1027:
    def test_validate_string_success(self):
        errs = validate("hello", {"type": "string"})
        assert errs == []

    def test_validate_string_type_error(self):
        errs = validate(123, {"type": "string"})
        assert len(errs) == 1
        assert errs[0].message == "must be string"

    def test_validate_string_min_length(self):
        errs = validate("ab", {"type": "string", "minLength": 5})
        assert len(errs) == 1
        assert "minLength" in errs[0].message

    def test_validate_string_max_length(self):
        errs = validate("abcdef", {"type": "string", "maxLength": 3})
        assert len(errs) == 1

    def test_validate_string_pattern(self):
        errs = validate("abc123", {"type": "string", "pattern": r"^[a-z]+$"})
        assert len(errs) == 1

    def test_validate_number_success(self):
        errs = validate(42, {"type": "integer"})
        assert errs == []

    def test_validate_number_type_error(self):
        errs = validate("x", {"type": "number"})
        assert len(errs) == 1

    def test_validate_number_min(self):
        errs = validate(5, {"type": "integer", "minimum": 10})
        assert len(errs) == 1

    def test_validate_number_max(self):
        errs = validate(100, {"type": "integer", "maximum": 50})
        assert len(errs) == 1

    def test_validate_array_success(self):
        errs = validate([1, 2, 3], {"type": "array"})
        assert errs == []

    def test_validate_array_min_items(self):
        errs = validate([1], {"type": "array", "minItems": 3})
        assert len(errs) == 1

    def test_validate_array_max_items(self):
        errs = validate([1, 2, 3, 4], {"type": "array", "maxItems": 2})
        assert len(errs) == 1

    def test_validate_object_success(self):
        errs = validate({"a": 1}, {"type": "object"})
        assert errs == []

    def test_validate_object_required(self):
        errs = validate({}, {"type": "object", "required": ["id"]})
        assert len(errs) == 1
        assert "required" in errs[0].message

    def test_validate_object_nested(self):
        """V1027 真测 Pydantic 嵌套 真借鉴 (主 19:33)."""
        schema = {
            "type": "object",
            "properties": {
                "user": {
                    "type": "object",
                    "required": ["id"],
                    "properties": {"id": {"type": "string"}},
                },
            },
        }
        errs = validate({"user": {}}, schema)
        assert len(errs) == 1
        assert errs[0].path == "user.id"

    def test_validate_boolean(self):
        errs = validate(True, {"type": "boolean"})
        assert errs == []
        errs = validate("yes", {"type": "boolean"})
        assert len(errs) == 1

    def test_init(self):
        v = V1027Validator()
        assert v.n_schemas() == 0

    def test_register_schema(self):
        """V1027 真测 JSON Schema 真借鉴 (主 19:33)."""
        v = V1027Validator()
        v.register_schema("user", {"type": "object"})
        assert v.n_schemas() == 1

    def test_validate_success(self):
        v = V1027Validator()
        v.register_schema("user", {
            "type": "object",
            "required": ["id"],
            "properties": {"id": {"type": "string"}},
        })
        ok, errs = v.validate("user", {"id": "u1"})
        assert ok is True
        assert errs == []

    def test_validate_failure(self):
        v = V1027Validator()
        v.register_schema("user", {
            "type": "object",
            "required": ["id"],
            "properties": {"id": {"type": "string"}},
        })
        ok, errs = v.validate("user", {})
        assert ok is False
        assert len(errs) == 1

    def test_validate_unknown_schema(self):
        v = V1027Validator()
        ok, errs = v.validate("missing", "x")
        assert ok is False
        assert "unknown" in errs[0].message

    def test_stats(self):
        v = V1027Validator()
        v.register_schema("a", {})
        v.register_schema("b", {})
        s = v.stats()
        assert s["n_schemas"] == 2

    def test_v22_33_asi_integration(self):
        """V1027 真测主 22:33 ASI 北极星."""
        v = V1027Validator()
        s = v.stats()
        assert "ASI" in s["philosophy"]

    def test_v19_33_jsonschema_pydantic(self):
        """V1027 真测主 19:33 JSON Schema + Pydantic 真借鉴."""
        v = V1027Validator()
        schema = {
            "type": "object",
            "required": ["id", "name"],
            "properties": {
                "id": {"type": "string", "minLength": 1},
                "name": {"type": "string"},
                "tags": {"type": "array", "items": {"type": "string"}},
            },
        }
        v.register_schema("user", schema)
        ok, errs = v.validate("user", {
            "id": "u1",
            "name": "Apeireth",
            "tags": ["v1001", "v1002"],
        })
        assert ok is True

    def test_v17_43_truth(self):
        """V1027 真测主 17:43 实事求是 — 真校验, 不假装."""
        v = V1027Validator()
        v.register_schema("memory", {
            "type": "object",
            "required": ["id", "content"],
            "properties": {
                "id": {"type": "string", "minLength": 1},
                "content": {"type": "string"},
            },
        })
        # 真 invalid
        ok, errs = v.validate("memory", {"content": "x"})
        assert ok is False
        assert any("id" in e.path for e in errs)
        # 真 valid
        ok2, errs2 = v.validate("memory", {"id": "m1", "content": "ASI 真生产"})
        assert ok2 is True

    def test_complete_integration(self):
        """V1027 真测完整 validator (主 23:44 + 主 22:33 + 主 19:33 + 主 17:43)."""
        v = V1027Validator()
        v.register_schema("memory", {
            "type": "object",
            "required": ["id", "content", "importance"],
            "properties": {
                "id": {"type": "string", "minLength": 1, "maxLength": 100},
                "content": {"type": "string", "minLength": 1},
                "importance": {"type": "number", "minimum": 0.0, "maximum": 1.0},
                "tags": {"type": "array", "maxItems": 10},
            },
        })
        # 真 valid
        ok, _ = v.validate("memory", {
            "id": "mem_1",
            "content": "Apeireth ASI",
            "importance": 0.79,
            "tags": ["v1001", "v1002"],
        })
        assert ok is True
        # 真 invalid (importance out of range)
        ok2, errs2 = v.validate("memory", {
            "id": "mem_2",
            "content": "x",
            "importance": 2.0,
        })
        assert ok2 is False
        assert any("importance" in e.path for e in errs2)