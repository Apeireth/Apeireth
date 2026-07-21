"""V1033 真生产 tests (主 00:36 适配性 + 效果)."""
from __future__ import annotations
import sys; sys.path.insert(0, '.')
import json
import pytest
from apeireth.v1033_openapi import (
    V1033_VERSION, OPENAPI_VERSION, OpenAPISchema, OpenAPIEndpoint,
    V1033OpenAPIGenerator,
)


class TestV1033:
    def test_init(self):
        gen = V1033OpenAPIGenerator()
        assert gen.n_endpoints() == 0
        assert gen.n_schemas() == 2  # Memory + Error

    def test_default_schemas(self):
        """V1033 真测 OpenAPI default schemas (主 19:33 真借鉴)."""
        gen = V1033OpenAPIGenerator()
        assert "Memory" in gen.schemas
        assert "Error" in gen.schemas

    def test_register_schema(self):
        gen = V1033OpenAPIGenerator()
        gen.register_schema(OpenAPISchema(
            name="User", schema_type="object",
            properties={"id": OpenAPISchema(name="id", schema_type="string")},
            required=["id"],
        ))
        assert gen.n_schemas() == 3

    def test_add_endpoint(self):
        """V1033 真测 OpenAPI endpoint 真借鉴 (主 19:33)."""
        gen = V1033OpenAPIGenerator()
        gen.add_endpoint("/test", "GET", "Test endpoint")
        assert gen.n_endpoints() == 1

    def test_generate_basic(self):
        gen = V1033OpenAPIGenerator()
        gen.add_endpoint("/test", "GET", "Test")
        spec = gen.generate()
        assert spec["openapi"] == OPENAPI_VERSION
        assert spec["info"]["title"] == "Apeireth ASI API"
        assert "/test" in spec["paths"]

    def test_generate_with_post(self):
        gen = V1033OpenAPIGenerator()
        gen.add_endpoint("/memories", "POST", "Create memory", auth_required=True)
        spec = gen.generate()
        assert "post" in spec["paths"]["/memories"]
        # 真 auth_required → security 字段
        assert spec["paths"]["/memories"]["post"]["security"] == [{"bearerAuth": []}]

    def test_generate_with_parameters(self):
        """V1033 真测 path parameters 真借鉴 (主 19:33 OpenAPI 真借鉴)."""
        gen = V1033OpenAPIGenerator()
        gen.add_endpoint(
            "/memories/{id}", "GET", "Get memory",
            parameters=[{
                "name": "id", "in": "path", "required": True,
                "schema": {"type": "string"},
            }],
        )
        spec = gen.generate()
        params = spec["paths"]["/memories/{id}"]["get"]["parameters"]
        assert params[0]["name"] == "id"

    def test_generate_with_request_body(self):
        gen = V1033OpenAPIGenerator()
        body = {
            "required": True,
            "content": {"application/json": {"schema": {"$ref": "#/components/schemas/Memory"}}},
        }
        gen.add_endpoint("/memories", "POST", "Create", request_body=body)
        spec = gen.generate()
        assert "requestBody" in spec["paths"]["/memories"]["post"]

    def test_generate_with_tags(self):
        gen = V1033OpenAPIGenerator()
        gen.add_endpoint("/test", "GET", "T", tags=["v1001"])
        spec = gen.generate()
        assert "v1001" in spec["paths"]["/test"]["get"]["tags"]

    def test_components_schemas(self):
        """V1033 真测 components.schemas 真借鉴 (主 19:33 OpenAPI 3.0)."""
        gen = V1033OpenAPIGenerator()
        spec = gen.generate()
        assert "schemas" in spec["components"]
        assert "Memory" in spec["components"]["schemas"]
        mem = spec["components"]["schemas"]["Memory"]
        assert mem["type"] == "object"
        assert "id" in mem["properties"]
        assert "id" in mem["required"]

    def test_components_security_schemes(self):
        """V1033 真测 JWT security scheme 真借鉴 (主 19:33 + V1028 整合)."""
        gen = V1033OpenAPIGenerator()
        spec = gen.generate()
        assert "bearerAuth" in spec["components"]["securitySchemes"]
        scheme = spec["components"]["securitySchemes"]["bearerAuth"]
        assert scheme["type"] == "http"
        assert scheme["scheme"] == "bearer"

    def test_to_json(self):
        gen = V1033OpenAPIGenerator()
        gen.add_endpoint("/test", "GET", "T")
        js = gen.to_json()
        # 真 JSON
        parsed = json.loads(js)
        assert parsed["openapi"] == OPENAPI_VERSION

    def test_to_yaml(self):
        gen = V1033OpenAPIGenerator()
        gen.add_endpoint("/test", "GET", "T")
        y = gen.to_yaml()
        # YAML or JSON fallback
        assert "openapi" in y

    def test_integrate_with_rest_gateway(self):
        """V1033 真测 V1016 REST gateway 整合 (主 22:33)."""
        gen = V1033OpenAPIGenerator()
        from apeireth.v1016_rest_gateway import V1016RESTGateway
        g = V1016RESTGateway()
        g.add_route("/memories", ["GET"], "list_memories", auth_required=True)
        g.add_route("/memories/:id", ["GET", "DELETE"], "memory_ops")
        added = gen.integrate_with_rest_gateway(g)
        assert len(added) == 3
        # 真 spec 包含 routes
        spec = gen.generate()
        assert "/memories" in spec["paths"]

    def test_stats(self):
        gen = V1033OpenAPIGenerator()
        s = gen.stats()
        assert s["n_schemas"] == 2
        assert s["version"] == V1033_VERSION

    def test_v22_33_asi_integration(self):
        """V1033 真测主 22:33 ASI 北极星."""
        gen = V1033OpenAPIGenerator()
        s = gen.stats()
        assert "ASI" in s["philosophy"]

    def test_v00_36_adaptability(self):
        """V1033 真测主 00:36 适配性 — 真能 import Postman."""
        gen = V1033OpenAPIGenerator()
        gen.add_endpoint("/test", "GET", "T", auth_required=True)
        spec = gen.generate()
        # Postman 必需字段
        assert "openapi" in spec
        assert "info" in spec
        assert "paths" in spec
        assert "components" in spec
        # JWT auth
        assert spec["components"]["securitySchemes"]["bearerAuth"]["scheme"] == "bearer"

    def test_v19_33_openapi_3(self):
        """V1033 真测主 19:33 OpenAPI 3.0.3 真借鉴."""
        gen = V1033OpenAPIGenerator()
        spec = gen.generate()
        assert spec["openapi"] == "3.0.3"
        assert "servers" in spec

    def test_v17_43_truth(self):
        """V1033 真测主 17:43 实事求是 — 真生成, 不假装."""
        gen = V1033OpenAPIGenerator()
        gen.add_endpoint("/test", "GET", "T")
        spec = gen.generate()
        # 真 JSON 可解析
        js = json.dumps(spec)
        parsed = json.loads(js)
        assert parsed["paths"]["/test"]["get"]["summary"] == "T"

    def test_complete_integration(self):
        """V1033 真测完整 OpenAPI (主 00:36 + 主 22:33 + 主 19:33 + 主 17:43)."""
        gen = V1033OpenAPIGenerator()
        gen.add_endpoint("/memories", "GET", "List memories")
        gen.add_endpoint("/memories", "POST", "Create memory", auth_required=True)
        gen.add_endpoint("/memories/{id}", "GET", "Get memory",
                        parameters=[{"name": "id", "in": "path", "required": True, "schema": {"type": "string"}}])
        spec = gen.generate()
        # 3 个 endpoint 全部生成
        assert len(spec["paths"]) == 2  # 2 unique paths
        # 真 JSON
        js = gen.to_json()
        parsed = json.loads(js)
        assert "Memory" in parsed["components"]["schemas"]