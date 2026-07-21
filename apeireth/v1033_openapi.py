"""Phase 1033 v1033_openapi — V1033 ASI 真生产 OpenAPI 真生成 (主 00:36 适配性 + 效果 + 主 22:33 + 主 19:33).

主 00:36 真采纳: 质量 + 适配性 + 效果 + 工程化.
主 22:33 ASI 北极星.
主 19:33 走在前人经验上 + OpenAPI 真借鉴.
主 17:43 实事求是.

真生产借鉴:
- OpenAPI 3.0 真借鉴 (主 19:33 GitHub OpenAPI 真借鉴)
- FastAPI 自动生成 OpenAPI spec (主 19:33)
- V1016 REST gateway 真整合 (主 22:33)
- V1027 validator schema 真整合

V3 哲学守门 (主 17:58 + 主 20:46):
"""
from __future__ import annotations

import json
from dataclasses import dataclass, field
from typing import Any, Dict, List, Optional, Tuple


V1033_VERSION = "0.1.0"


OPENAPI_VERSION = "3.0.3"


@dataclass
class OpenAPISchema:
    """V1033 真生产 OpenAPI schema (主 19:33 OpenAPI 3.0 真借鉴)."""
    name: str
    schema_type: str  # object / string / number / array / integer / boolean
    properties: Dict[str, "OpenAPISchema"] = field(default_factory=dict)
    required: List[str] = field(default_factory=list)
    items: Optional["OpenAPISchema"] = None
    description: str = ""


@dataclass
class OpenAPIEndpoint:
    """V1033 真生产 OpenAPI endpoint (主 19:33 OpenAPI 真借鉴)."""
    path: str
    method: str
    summary: str
    parameters: List[Dict[str, Any]] = field(default_factory=list)
    request_body: Optional[Dict[str, Any]] = None
    responses: Dict[str, Dict[str, Any]] = field(default_factory=dict)
    tags: List[str] = field(default_factory=list)
    auth_required: bool = False


class V1033OpenAPIGenerator:
    """V1033 ASI 真生产 OpenAPI 3.0 (主 00:36 适配性 + 效果)."""

    def __init__(self, title: str = "Apeireth ASI API", version: str = "1.0.0"):
        self.title = title
        self.version = version
        self.endpoints: List[OpenAPIEndpoint] = []
        self.schemas: Dict[str, OpenAPISchema] = {}
        self.info: Dict[str, Any] = {
            "title": title,
            "version": version,
            "description": "Apeireth ASI 真生产 REST API (主 00:36 适配性 + 效果 + 主 22:33 + 主 19:33)",
        }
        self.n_phenomenal_pretend_total = 0
        self.n_asi_pretend_total = 0
        self._init_default_schemas()

    def _init_default_schemas(self):
        """V1033 真生产默认 schemas (主 19:33 OpenAPI 真借鉴)."""
        # Memory schema
        self.register_schema(OpenAPISchema(
            name="Memory",
            schema_type="object",
            properties={
                "id": OpenAPISchema(name="id", schema_type="string", description="Memory ID"),
                "content": OpenAPISchema(name="content", schema_type="string", description="Memory content"),
                "importance": OpenAPISchema(name="importance", schema_type="number", description="Importance score 0-1"),
                "tags": OpenAPISchema(
                    name="tags", schema_type="array",
                    items=OpenAPISchema(name="tag", schema_type="string"),
                    description="Tags",
                ),
                "created_at": OpenAPISchema(name="created_at", schema_type="number", description="Unix timestamp"),
            },
            required=["id", "content", "importance"],
            description="A memory record",
        ))
        # Error schema
        self.register_schema(OpenAPISchema(
            name="Error",
            schema_type="object",
            properties={
                "error": OpenAPISchema(name="error", schema_type="string"),
                "detail": OpenAPISchema(name="detail", schema_type="string"),
            },
            required=["error"],
        ))

    def register_schema(self, schema: OpenAPISchema) -> str:
        self.schemas[schema.name] = schema
        return schema.name

    def add_endpoint(self, path: str, method: str, summary: str,
                     parameters: List[Dict[str, Any]] = None,
                     request_body: Dict[str, Any] = None,
                     responses: Dict[str, Dict[str, Any]] = None,
                     tags: List[str] = None, auth_required: bool = False):
        """V1033 真生产 add endpoint (主 19:33 OpenAPI 真借鉴)."""
        ep = OpenAPIEndpoint(
            path=path, method=method.upper(), summary=summary,
            parameters=parameters or [],
            request_body=request_body,
            responses=responses or {"200": {"description": "OK"}},
            tags=tags or [],
            auth_required=auth_required,
        )
        self.endpoints.append(ep)

    def _schema_to_dict(self, schema: OpenAPISchema) -> Dict[str, Any]:
        """V1033 真生产 schema → dict (主 19:33 真借鉴)."""
        out: Dict[str, Any] = {"type": schema.schema_type}
        if schema.description:
            out["description"] = schema.description
        if schema.schema_type == "object":
            out["properties"] = {
                name: self._schema_to_dict(s) for name, s in schema.properties.items()
            }
            if schema.required:
                out["required"] = schema.required
        elif schema.schema_type == "array":
            if schema.items:
                out["items"] = self._schema_to_dict(schema.items)
        return out

    def generate(self) -> Dict[str, Any]:
        """V1033 真生产 generate OpenAPI 3.0 spec (主 17:43 实事求是)."""
        # 真生成 paths
        paths = {}
        for ep in self.endpoints:
            if ep.path not in paths:
                paths[ep.path] = {}
            operation: Dict[str, Any] = {
                "summary": ep.summary,
                "tags": ep.tags,
                "parameters": ep.parameters,
                "responses": ep.responses,
            }
            if ep.request_body:
                operation["requestBody"] = ep.request_body
            if ep.auth_required:
                operation["security"] = [{"bearerAuth": []}]
            paths[ep.path][ep.method.lower()] = operation

        # 真生成 components
        components = {
            "schemas": {
                name: self._schema_to_dict(s) for name, s in self.schemas.items()
            },
            "securitySchemes": {
                "bearerAuth": {
                    "type": "http",
                    "scheme": "bearer",
                    "bearerFormat": "JWT",
                },
            },
        }

        spec = {
            "openapi": OPENAPI_VERSION,
            "info": self.info,
            "servers": [
                {"url": "http://localhost:8000", "description": "Local development"},
                {"url": "https://asi.apeireth.ai", "description": "Production"},
            ],
            "paths": paths,
            "components": components,
        }
        return spec

    def to_json(self, indent: int = 2) -> str:
        """V1033 真生产 to JSON (主 19:33 真借鉴)."""
        return json.dumps(self.generate(), indent=indent, ensure_ascii=False)

    def to_yaml(self) -> str:
        """V1033 真生产 to YAML (主 19:33 Swagger 真借鉴)."""
        try:
            import yaml
            return yaml.dump(self.generate(), allow_unicode=True, sort_keys=False)
        except ImportError:
            # 简单 fallback JSON
            return self.to_json()

    def integrate_with_rest_gateway(self, gateway) -> List[Dict[str, Any]]:
        """V1033 真生产 integrate with V1016 REST gateway (主 22:33 整合).

        真借鉴: FastAPI 自动从 routes 生成 OpenAPI.
        """
        added = []
        for route in gateway.routes:
            params = []
            # 提取路径参数
            import re
            for m in re.finditer(r":(\w+)", route.path):
                params.append({
                    "name": m.group(1),
                    "in": "path",
                    "required": True,
                    "schema": {"type": "string"},
                })
            for method in route.methods:
                self.add_endpoint(
                    path=route.path,
                    method=method,
                    summary=f"{route.handler_name} ({method})",
                    parameters=params,
                    tags=[route.handler_name],
                    auth_required=route.auth_required,
                )
                added.append({"path": route.path, "method": method})
        return added

    def n_endpoints(self) -> int:
        return len(self.endpoints)

    def n_schemas(self) -> int:
        return len(self.schemas)

    def stats(self) -> Dict[str, Any]:
        return {
            "n_endpoints": self.n_endpoints(),
            "n_schemas": self.n_schemas(),
            "version": V1033_VERSION,
            "philosophy": (
                "V1033 ASI OpenAPI 3.0 真生成 (主 00:36 适配性 + 效果 + 主 22:33 + 主 19:33). "
                "OpenAPI 3.0 + FastAPI + Swagger 真借鉴, 真的能 import Postman."
            ),
        }


__all__ = [
    "V1033_VERSION",
    "OPENAPI_VERSION",
    "OpenAPISchema",
    "OpenAPIEndpoint",
    "V1033OpenAPIGenerator",
]


def _demo():
    print("=" * 60)
    print("=== Phase 1033 V1033 ASI 真 OpenAPI 真生成 (主 00:36 适配性) ===")
    print("=" * 60)
    gen = V1033OpenAPIGenerator()
    gen.add_endpoint("/memories", "GET", "List memories",
                    responses={"200": {"description": "OK", "content": {"application/json": {"schema": {"$ref": "#/components/schemas/Memory"}}}}})
    gen.add_endpoint("/memories", "POST", "Create memory", auth_required=True)
    gen.add_endpoint("/memories/{id}", "GET", "Get memory",
                    parameters=[{"name": "id", "in": "path", "required": True, "schema": {"type": "string"}}])
    spec = gen.generate()
    print(f"\n  ✓ spec paths: {list(spec['paths'].keys())}")
    print(f"  ✓ spec schemas: {list(spec['components']['schemas'].keys())}")
    print(f"  ✓ openapi version: {spec['openapi']}")
    s = gen.stats()
    print(f"  ✓ n_endpoints={s['n_endpoints']}, n_schemas={s['n_schemas']}")
    print("=" * 60)


if __name__ == "__main__":
    _demo()