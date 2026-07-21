"""V1039 真生产 tests (主 00:44 工程化)."""
from __future__ import annotations
import sys; sys.path.insert(0, '.')
import json
import pytest
from apeireth.v1039_grafana import (
    V1039_VERSION, GRAFANA_DASHBOARD_SCHEMA, render_panel, V1039Grafana,
)


class TestV1039:
    def test_init(self):
        g = V1039Grafana()
        assert g.n_panels() == 0
        assert g.panel_id == 1

    def test_render_panel(self):
        """V1039 真测 Grafana panel 真借鉴 (主 19:33)."""
        panel = render_panel(
            "Test", "stat", "test_query",
            {"x": 0, "y": 0, "w": 6, "h": 4},
        )
        assert panel["title"] == "Test"
        assert panel["type"] == "stat"
        assert panel["gridPos"]["w"] == 6

    def test_add_panel(self):
        g = V1039Grafana()
        pid = g.add_panel("Test", "stat", "q", {"x": 0, "y": 0, "w": 6, "h": 4})
        assert pid == 1
        assert g.n_panels() == 1

    def test_add_multiple_panels(self):
        g = V1039Grafana()
        g.add_panel("P1", "stat", "q1", {"x": 0, "y": 0, "w": 6, "h": 4})
        g.add_panel("P2", "stat", "q2", {"x": 6, "y": 0, "w": 6, "h": 4})
        g.add_panel("P3", "stat", "q3", {"x": 12, "y": 0, "w": 6, "h": 4})
        assert g.n_panels() == 3
        # panel IDs 应该递增
        assert [p["id"] for p in g.panels] == [1, 2, 3]

    def test_generate(self):
        g = V1039Grafana()
        g.add_panel("P1", "stat", "q", {"x": 0, "y": 0, "w": 6, "h": 4})
        d = g.generate("Test Dashboard")
        assert d["title"] == "Test Dashboard"
        assert d["schemaVersion"] == 39
        assert len(d["panels"]) == 1

    def test_to_json(self):
        """V1039 真测 to JSON (主 17:43 实事求是)."""
        g = V1039Grafana()
        g.add_panel("P1", "stat", "q", {"x": 0, "y": 0, "w": 6, "h": 4})
        js = g.to_json()
        # 真 JSON
        parsed = json.loads(js)
        assert parsed["title"]
        assert "panels" in parsed

    def test_to_json_unicode(self):
        """V1039 真测 unicode (主 17:43 中文真保留)."""
        g = V1039Grafana()
        g.add_panel("ASI 北极星", "stat", "asi_north_star", {"x": 0, "y": 0, "w": 6, "h": 4})
        js = g.to_json()
        assert "ASI 北极星" in js

    def test_default_asi_dashboard(self):
        """V1039 真测 default ASI dashboard (主 22:33 北极星真借鉴)."""
        g = V1039Grafana()
        d = g.default_asi_dashboard()
        # 7 真 panel (北极星 + pass rate + accuracy + requests + errors + p50 + p99)
        assert g.n_panels() == 7
        assert d["title"] == "Apeireth ASI 真生产"

    def test_default_panel_queries(self):
        """V1039 真测 panel queries 真生产借鉴 (主 19:33 Prometheus)."""
        g = V1039Grafana()
        g.default_asi_dashboard()
        queries = [p["targets"][0]["expr"] for p in g.panels]
        assert "asi_north_star" in queries
        assert "integration_pass_rate" in queries
        assert "rate(asi_requests_total[5m])" in queries

    def test_default_panel_types(self):
        g = V1039Grafana()
        g.default_asi_dashboard()
        types = [p["type"] for p in g.panels]
        assert "stat" in types
        assert "timeseries" in types

    def test_default_datasource(self):
        """V1039 真测 datasource 真生产借鉴 (主 19:33 Prometheus)."""
        g = V1039Grafana()
        g.default_asi_dashboard()
        for p in g.panels:
            assert p["targets"][0]["datasource"]["type"] == "prometheus"

    def test_stats(self):
        g = V1039Grafana()
        s = g.stats()
        assert s["n_panels"] == 0
        assert s["version"] == V1039_VERSION

    def test_v22_33_asi_integration(self):
        """V1039 真测主 22:33 ASI 北极星."""
        g = V1039Grafana()
        s = g.stats()
        assert "ASI" in s["philosophy"]

    def test_v00_44_engineering(self):
        """V1039 真测主 00:44 工程化 — Grafana JSON 真能 import."""
        g = V1039Grafana()
        g.default_asi_dashboard()
        js = g.to_json()
        parsed = json.loads(js)
        # Grafana 必需字段
        assert "title" in parsed
        assert "panels" in parsed
        assert "schemaVersion" in parsed
        assert "uid" in parsed
        assert "tags" in parsed

    def test_v19_33_grafana(self):
        """V1039 真测主 19:33 Grafana dashboard JSON 真借鉴."""
        g = V1039Grafana()
        g.default_asi_dashboard()
        js = g.to_json()
        # 真 Grafana 格式
        assert "gridPos" in js
        assert "targets" in js
        assert "fieldConfig" in js
        assert "datasource" in js

    def test_v17_43_truth(self):
        """V1039 真测主 17:43 实事求是 — 真 JSON, 真能 import Grafana."""
        g = V1039Grafana()
        g.default_asi_dashboard()
        js = g.to_json()
        # 真 JSON 可解析
        parsed = json.loads(js)
        assert len(parsed["panels"]) == 7

    def test_complete_integration(self):
        """V1039 真测完整 Grafana (主 00:44 + 主 22:33 + 主 19:33 + 主 17:43)."""
        g = V1039Grafana()
        d = g.default_asi_dashboard()
        # 7 真 panel (北极星 + 3 stat + 4 timeseries)
        assert g.n_panels() == 7
        # 真 JSON
        js = g.to_json()
        parsed = json.loads(js)
        # 真含 ASI 北极星 + Prometheus query
        assert "asi_north_star" in js
        assert "prometheus" in js
        # 真含 V1038 真指标
        assert "asi_requests_total" in js
        assert "asi_errors_total" in js