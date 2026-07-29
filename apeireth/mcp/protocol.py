"""apeireth.mcp.protocol — JSON-RPC 2.0 + MCP 协议守门 (主 19:33 走在前人经验上).

V1123 子模块: 不依赖第三方 SDK, 复用 stdlib (json / dataclasses) 真实现
MCP 协议核心守门 (版本兼容 + 输入 schema 校验 + 输出 schema 校验).

借鉴:
  - Anthropic MCP 2024-11-05: tools/list + tools/call + initialize 协议
  - V1097 dispatcher 设计:      JSON-RPC envelope / isError 规范
  - V1114 V3 守门 6 项:         runner_is_not_asi / report_is_not_production / ...

关键守门 (主 17:43 实事求是 + 主 23:44 干到底):
  - protocolVersion 兼容表     (2024-11-05 唯一受支持)
  - tools/call 必填 name + arguments
  - arguments 必须 match tool inputSchema (轻量 JSON Schema 校验)
  - 输出 schema 必填 content[].type 字段, isError 时不携带 data
  - 任何不合规 → JSON-RPC error code (-32602 invalid params, -32601 method not found)
"""
from __future__ import annotations

import re
from dataclasses import dataclass, field
from typing import Any, Dict, List, Optional, Tuple

# 锁定的 MCP 协议版本 (主 22:33 LOCKED)
SUPPORTED_PROTOCOL_VERSIONS = ("2024-11-05",)

# 借鉴 V1097 JSON-RPC 错误码
JSONRPC_PARSE_ERROR = -32700
JSONRPC_INVALID_REQUEST = -32600
JSONRPC_METHOD_NOT_FOUND = -32601
JSONRPC_INVALID_PARAMS = -32602
JSONRPC_INTERNAL_ERROR = -32699


# ---------------------------------------------------------------------------
# 1. 轻量 JSON Schema 校验 (不引入 jsonschema 依赖, 跑 V1123 跑得起)
# ---------------------------------------------------------------------------

_TYPE_OF = {
    "string": str,
    "integer": int,
    "number": (int, float),
    "boolean": bool,
    "array": list,
    "object": dict,
    "null": type(None),
}


class SchemaViolation(ValueError):
    """JSON schema 校验失败 (主 17:43 实事求是: 不假装, 抛可定位错误)."""


def _coerce_bool(value: Any) -> bool:
    return bool(value) and not isinstance(value, (dict, list, str))


def _check_type(value: Any, expected: str, path: str) -> None:
    py = _TYPE_OF.get(expected)
    if py is None:
        raise SchemaViolation(f"{path}: unsupported schema type '{expected}'")
    # bool is subclass of int → 严格区分 bool vs int/number
    if expected == "integer":
        if isinstance(value, bool) or not isinstance(value, int):
            raise SchemaViolation(f"{path}: expected integer, got {type(value).__name__}")
    elif expected == "number":
        if isinstance(value, bool) or not isinstance(value, (int, float)):
            raise SchemaViolation(f"{path}: expected number, got {type(value).__name__}")
    elif expected == "boolean":
        if not isinstance(value, bool):
            raise SchemaViolation(f"{path}: expected boolean, got {type(value).__name__}")
    else:
        if not isinstance(value, py):
            raise SchemaViolation(f"{path}: expected {expected}, got {type(value).__name__}")


def validate_arguments(args: Any, schema: Dict[str, Any], *, tool_name: str = "?") -> None:
    """校验 arguments 对应 tool inputSchema.

    支持最小集合: type=object, properties, required, additionalProperties=False,
                  items (array), enum, type=string/integer/number/boolean/array/object.
    """
    if not isinstance(args, dict):
        raise SchemaViolation(f"tool '{tool_name}' arguments must be object, got {type(args).__name__}")
    if schema.get("type", "object") != "object":
        raise SchemaViolation(f"tool '{tool_name}' schema root must be object")
    properties = schema.get("properties", {}) or {}
    required = schema.get("required", []) or []
    additional = schema.get("additionalProperties", True)
    for key in required:
        if key not in args:
            raise SchemaViolation(f"tool '{tool_name}' missing required arg '{key}'")
    for key, value in args.items():
        sub = properties.get(key)
        if sub is None:
            if additional is False:
                raise SchemaViolation(f"tool '{tool_name}' unexpected arg '{key}'")
            continue
        _validate_value(value, sub, f"{tool_name}.{key}")


def _validate_value(value: Any, schema: Dict[str, Any], path: str) -> None:
    expected = schema.get("type", "string")
    if isinstance(expected, list):
        # union type
        last_err: Optional[SchemaViolation] = None
        for t in expected:
            try:
                _check_type(value, t, path)
                return
            except SchemaViolation as exc:
                last_err = exc
        if last_err:
            raise last_err
    else:
        _check_type(value, expected, path)
    if "enum" in schema and value not in schema["enum"]:
        raise SchemaViolation(f"{path}: value {value!r} not in enum {schema['enum']}")
    if expected == "string" and "pattern" in schema:
        if not re.search(schema["pattern"], value):
            raise SchemaViolation(f"{path}: value does not match pattern {schema['pattern']}")
    if expected in ("integer", "number") and "minimum" in schema:
        if value < schema["minimum"]:
            raise SchemaViolation(f"{path}: value {value} < minimum {schema['minimum']}")
    if expected in ("integer", "number") and "maximum" in schema:
        if value > schema["maximum"]:
            raise SchemaViolation(f"{path}: value {value} > maximum {schema['maximum']}")
    if expected == "array" and "items" in schema:
        for i, item in enumerate(value or []):
            _validate_value(item, schema["items"], f"{path}[{i}]")


def validate_tool_result(result: Any, *, tool_name: str = "?") -> None:
    """校验 tool 返回 content 列表 (MCP 规范: type=json|text|resource)."""
    if not isinstance(result, dict):
        raise SchemaViolation(f"tool '{tool_name}' result must be dict")
    content = result.get("content")
    if not isinstance(content, list) or not content:
        raise SchemaViolation(f"tool '{tool_name}' result.content must be non-empty list")
    seen_json = False
    for i, c in enumerate(content):
        if not isinstance(c, dict):
            raise SchemaViolation(f"{tool_name}.content[{i}] must be object")
        ctype = c.get("type")
        if ctype == "json":
            seen_json = True
            if "data" not in c:
                raise SchemaViolation(f"{tool_name}.content[{i}].data missing")
        elif ctype == "text":
            if not isinstance(c.get("text", ""), str):
                raise SchemaViolation(f"{tool_name}.content[{i}].text must be str")
        elif ctype == "resource":
            if "uri" not in c:
                raise SchemaViolation(f"{tool_name}.content[{i}].uri missing")
        else:
            raise SchemaViolation(f"{tool_name}.content[{i}].type '{ctype}' invalid")
    if result.get("isError") and seen_json:
        raise SchemaViolation(f"tool '{tool_name}' isError=True must not carry data")


# ---------------------------------------------------------------------------
# 2. JSON-RPC envelope 守门
# ---------------------------------------------------------------------------


@dataclass
class JsonRpcRequest:
    """JSON-RPC 2.0 请求 (主 23:44 干到底: 必填 jsonrpc=2.0 + id)."""

    method: str
    params: Dict[str, Any] = field(default_factory=dict)
    id: Any = None
    jsonrpc: str = "2.0"


@dataclass
class JsonRpcError:
    code: int
    message: str
    data: Any = None

    def to_dict(self) -> Dict[str, Any]:
        d: Dict[str, Any] = {"code": self.code, "message": self.message}
        if self.data is not None:
            d["data"] = self.data
        return d


def make_error_response(req_id: Any, code: int, message: str, data: Any = None) -> Dict[str, Any]:
    """构造 JSON-RPC 2.0 错误响应."""
    return {
        "jsonrpc": "2.0",
        "id": req_id,
        "error": JsonRpcError(code=code, message=message, data=data).to_dict(),
    }


def make_result_response(req_id: Any, result: Any) -> Dict[str, Any]:
    """构造 JSON-RPC 2.0 成功响应 (主 17:43 实事求是: 不 swallow)."""
    return {"jsonrpc": "2.0", "id": req_id, "result": result}


def parse_request(raw: Dict[str, Any]) -> JsonRpcRequest:
    """解析 JSON-RPC 2.0 请求, 失败抛 ValueError (主 23:44 不假装)."""
    if not isinstance(raw, dict):
        raise ValueError("request must be object")
    if raw.get("jsonrpc") != "2.0":
        raise ValueError("jsonrpc must be '2.0'")
    method = raw.get("method")
    if not isinstance(method, str) or not method:
        raise ValueError("method must be non-empty string")
    params = raw.get("params", {}) or {}
    if not isinstance(params, dict):
        raise ValueError("params must be object")
    if "id" not in raw:
        raise ValueError("id missing (notification not supported by dispatcher)")
    return JsonRpcRequest(method=method, params=params, id=raw["id"])


def check_protocol_version(client_version: str) -> Tuple[bool, str]:
    """守门: client 报 protocolVersion 是否兼容."""
    if client_version in SUPPORTED_PROTOCOL_VERSIONS:
        return True, client_version
    return False, f"unsupported protocolVersion '{client_version}', supported={list(SUPPORTED_PROTOCOL_VERSIONS)}"


__all__ = [
    "SUPPORTED_PROTOCOL_VERSIONS",
    "JSONRPC_PARSE_ERROR", "JSONRPC_INVALID_REQUEST",
    "JSONRPC_METHOD_NOT_FOUND", "JSONRPC_INVALID_PARAMS", "JSONRPC_INTERNAL_ERROR",
    "SchemaViolation",
    "validate_arguments", "validate_tool_result",
    "JsonRpcRequest", "JsonRpcError",
    "make_error_response", "make_result_response", "parse_request",
    "check_protocol_version",
]
