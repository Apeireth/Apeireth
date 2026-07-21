"""V1016 真生产 tests (主 23:44 干到底)."""
from __future__ import annotations
import sys; sys.path.insert(0, '.')
import pytest
from apeireth.v1016_rest_gateway import (
    V1016_VERSION, Route, V1016RESTGateway,
)


class TestV1016:
    def test_init(self):
        g = V1016RESTGateway()
        assert g.n_routes() == 0

    def test_add_route(self):
        g = V1016RESTGateway()
        rid = g.add_route("/api/x", ["GET"], "x")
        assert g.n_routes() == 1

    def test_match_route_static(self):
        """V1016 真测静态路径匹配 (主 19:33)."""
        g = V1016RESTGateway()
        g.add_route("/api/memories", ["GET"], "list_memories")
        r = g.match_route("/api/memories", "GET")
        assert r is not None
        assert r.handler_name == "list_memories"

    def test_match_route_param(self):
        """V1016 真测 FastAPI 路径参数化 真借鉴."""
        g = V1016RESTGateway()
        g.add_route("/api/users/:id", ["GET"], "get_user")
        r = g.match_route("/api/users/42", "GET")
        assert r is not None

    def test_match_route_no_match(self):
        g = V1016RESTGateway()
        g.add_route("/api/x", ["GET"], "x")
        assert g.match_route("/api/y", "GET") is None

    def test_match_route_wrong_method(self):
        g = V1016RESTGateway()
        g.add_route("/api/x", ["GET"], "x")
        assert g.match_route("/api/x", "POST") is None

    def test_dispatch_success(self):
        g = V1016RESTGateway()
        g.add_route("/api/x", ["GET"], "x")
        result = g.dispatch("/api/x", "GET")
        assert result["status"] == 200
        assert result["handler"] == "x"

    def test_dispatch_404(self):
        g = V1016RESTGateway()
        result = g.dispatch("/api/missing", "GET")
        assert result["status"] == 404

    def test_dispatch_auth_required(self):
        """V1016 真测 auth_required (主 19:33 Kong 真借鉴)."""
        g = V1016RESTGateway()
        g.add_route("/api/admin", ["GET"], "admin", auth_required=True)
        # 无 token
        result = g.dispatch("/api/admin", "GET")
        assert result["status"] == 401

    def test_dispatch_auth_pass(self):
        g = V1016RESTGateway()
        g.add_route("/api/admin", ["GET"], "admin", auth_required=True)
        result = g.dispatch("/api/admin", "GET", headers={"Authorization": "Bearer xyz"})
        assert result["status"] == 200

    def test_register_plugin(self):
        """V1016 真测 Kong plugin 真借鉴."""
        g = V1016RESTGateway()
        g.register_plugin("logger", lambda: True)
        assert g.n_plugins() == 1 if hasattr(g, 'n_plugins') else len(g.plugins) == 1

    def test_dispatch_with_plugin(self):
        g = V1016RESTGateway()
        g.add_route("/api/x", ["GET"], "x", plugins=["logger"])
        g.register_plugin("logger", lambda: True)
        result = g.dispatch("/api/x", "GET")
        assert result["status"] == 200
        assert any("logger" in str(p) for p in result["plugins"])

    def test_stats(self):
        g = V1016RESTGateway()
        g.add_route("/a", ["GET"], "a")
        s = g.stats()
        assert s["n_routes"] == 1
        assert s["version"] == V1016_VERSION

    def test_v22_33_asi_integration(self):
        """V1016 真测主 22:33 ASI 北极星."""
        g = V1016RESTGateway()
        s = g.stats()
        assert "ASI" in s["philosophy"]

    def test_v19_33_fastapi_kong(self):
        """V1016 真测主 19:33 FastAPI + Kong 真借鉴."""
        g = V1016RESTGateway()
        g.add_route("/api/users/:id", ["GET"], "get_user")
        r = g.match_route("/api/users/123", "GET")
        assert r is not None

    def test_v17_33_real_dispatch(self):
        """V1016 真测主 17:33 放手干到底 — 真 dispatch."""
        g = V1016RESTGateway()
        g.add_route("/api/memories", ["GET", "POST"], "memories")
        g.add_route("/api/users/:id", ["GET"], "users")
        assert g.dispatch("/api/memories", "GET")["status"] == 200
        assert g.dispatch("/api/users/42", "GET")["status"] == 200
        assert g.dispatch("/api/missing", "GET")["status"] == 404

    def test_complete_integration(self):
        """V1016 真测完整 REST gateway (主 23:44 + 主 22:33 + 主 19:33 + 主 17:33)."""
        g = V1016RESTGateway()
        g.add_route("/api/memories", ["GET"], "list_memories", auth_required=True)
        g.add_route("/api/memories/:id", ["DELETE"], "delete_memory", auth_required=True)
        g.add_route("/api/health", ["GET"], "health")
        # 公开
        r1 = g.dispatch("/api/health", "GET")
        assert r1["status"] == 200
        # 需 auth
        r2 = g.dispatch("/api/memories", "GET")
        assert r2["status"] == 401
        # 路径参数
        r3 = g.dispatch("/api/memories/123", "DELETE", headers={"Authorization": "Bearer x"})
        assert r3["status"] == 200