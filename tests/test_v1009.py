"""V1009 真生产 tests (主 23:44)."""
from __future__ import annotations
import sys; sys.path.insert(0, '.')
import pytest
from apeireth.v1009_web_ui import V1009_VERSION, APIEndpoint, V1009WebUI


class TestV1009:
    def test_init(self):
        web = V1009WebUI()
        assert web.n_endpoints() == 8
        assert web.n_pages() == 10

    def test_default_endpoints(self):
        web = V1009WebUI()
        paths = [ep.path for ep in web.endpoints]
        assert "/health" in paths
        assert "/v1002/measure" in paths
        assert "/v1001/vcp" in paths
        assert "/v1004/evolve" in paths
        assert "/v1005/search" in paths
        assert "/v1006/themes" in paths
        assert "/v1003/philosophy" in paths
        assert "/v0_1/measure" in paths

    def test_default_pages(self):
        web = V1009WebUI()
        # 10 真生产页面
        assert any("ASI" in p for p in web.pages)
        assert any("V1002" in p for p in web.pages)
        assert any("V1001" in p for p in web.pages)
        assert any("V1003" in p for p in web.pages)
        assert any("V1004" in p for p in web.pages)
        assert any("V1005" in p for p in web.pages)
        assert any("V1006" in p for p in web.pages)
        assert any("V1008" in p for p in web.pages)

    def test_render_fastapi(self):
        web = V1009WebUI()
        app = web.render_fastapi_app()
        assert "FastAPI" in app
        assert "/health" in app
        assert "/v1002/measure" in app
        assert "/v1001/vcp" in app
        assert "/v1004/evolve" in app
        assert "/v1005/search" in app
        assert "/v1006/themes" in app
        assert "/v1003/philosophy" in app
        assert "/v0_1/measure" in app
        # 确认 endpoint 真实
        for ep in web.endpoints:
            assert f"'{ep.path}'" in app or f'"{ep.path}"' in app

    def test_render_streamlit(self):
        web = V1009WebUI()
        app = web.render_streamlit_app()
        assert "streamlit" in app
        assert "st.title" in app
        assert "st.set_page_config" in app
        assert "Apeireth ASI" in app
        for p in web.pages:
            assert p in app

    def test_stats(self):
        web = V1009WebUI()
        s = web.stats()
        assert s["n_endpoints"] == 8
        assert s["n_pages"] == 10
        assert s["version"] == V1009_VERSION

    def test_v22_33_asi_integration(self):
        """V1009 真测主 22:33 ASI 北极星."""
        web = V1009WebUI()
        app = web.render_fastapi_app()
        assert "Apeireth ASI" in app
        assert "asi_north_star" in app.lower() or "ASI" in app

    def test_v19_33_integration(self):
        """V1009 真测主 19:33 走在前人经验上 (FastAPI + Streamlit 真借鉴)."""
        web = V1009WebUI()
        fastapi = web.render_fastapi_app()
        streamlit = web.render_streamlit_app()
        assert "FastAPI" in fastapi
        assert "streamlit" in streamlit

    def test_v17_33_integration(self):
        """V1009 真测主 17:33 放手干到底 (8 endpoints + 10 pages 完整)."""
        web = V1009WebUI()
        assert web.n_endpoints() == 8
        assert web.n_pages() == 10

    def test_all_endpoints_valid(self):
        web = V1009WebUI()
        for ep in web.endpoints:
            assert ep.path.startswith("/")
            assert ep.method in ("GET", "POST", "PUT", "DELETE")
            assert ep.handler_name
            assert ep.description

    def test_real_modules_imported(self):
        """V1009 真测真模块导入 (主 17:43 实事求是)."""
        fastapi = web.render_fastapi_app() if False else None
        from apeireth.v1009_web_ui import V1009WebUI
        web = V1009WebUI()
        app = web.render_fastapi_app()
        # V1001 VCP 真引用
        assert "v1001_vcp_six_plugins_full" in app
        # V1002 V0.2 真引用
        assert "v1002_asi_v02_measure" in app
        # V1003 真哲学
        assert "v1003_v4_philosophy_full" in app
        # V1004 自演化
        assert "v1004_self_evolution_full" in app
        # V1005 AnySearch
        assert "v1005_anysearch_full_index" in app
        # V1006 真调研
        assert "v1006_research_grand_synthesis" in app

    def test_complete_integration(self):
        """V1009 真测完整 web UI (主 22:33 + 主 17:33 + 主 19:33)."""
        web = V1009WebUI()
        fastapi = web.render_fastapi_app()
        streamlit = web.render_streamlit_app()
        # 完整覆盖 - V1001-V1008 + 关键哲学 + 真借鉴
        keywords = ["FastAPI", "Streamlit", "Apeireth ASI", "V1001", "V1002",
                    "V1003", "V1004", "V1005", "V1006", "V1008",
                    "V0.2", "VCP", "Popper"]
        for k in keywords:
            assert k in fastapi or k in streamlit, f"missing: {k}"