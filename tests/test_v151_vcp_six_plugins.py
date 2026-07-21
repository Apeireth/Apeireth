"""v151_vcp_six_plugins_production.py 真生产回归测试 (主 22:27 不空壳)."""
from __future__ import annotations

import sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).parent.parent))
import pytest
from apeireth.v151_vcp_six_plugins_production import (
    V151_VERSION, VCPPluginType, VCPContextType, VCPPlugin,
    VCPSyncResult, VCPAsyncTask, V151VCPSixPluginsProduction,
)


class TestV151VCPSixPluginsProduction:
    def test_init(self):
        vcp = V151VCPSixPluginsProduction()
        assert vcp.plugins == {}
        assert vcp.n_plugins() == 0

    def test_register_plugin_single_type(self):
        vcp = V151VCPSixPluginsProduction()
        pid = vcp.register_plugin("test", [VCPPluginType.SYNC])
        assert pid in vcp.plugins
        assert VCPPluginType.SYNC in vcp.plugins[pid].types

    def test_register_plugin_multi_type(self):
        vcp = V151VCPSixPluginsProduction()
        pid = vcp.register_plugin(
            "multi",
            [VCPPluginType.SYNC, VCPPluginType.ASYNC, VCPPluginType.HYBRID],
            description="test multi-type",
            capabilities=["c1", "c2"],
        )
        assert len(vcp.plugins[pid].types) == 3

    def test_register_plugin_all_six_types(self):
        vcp = V151VCPSixPluginsProduction()
        all_types = list(VCPPluginType)
        pid = vcp.register_plugin("all", all_types)
        assert len(vcp.plugins[pid].types) == 6

    def test_execute_sync(self):
        vcp = V151VCPSixPluginsProduction()
        pid = vcp.register_plugin("p", [VCPPluginType.SYNC])
        rid = vcp.execute_sync(pid, query="test")
        assert rid in [r.sync_id for r in vcp.sync_results]
        assert vcp.sync_results[-1].result is not None
        assert vcp.sync_results[-1].error == ""

    def test_execute_sync_unknown(self):
        vcp = V151VCPSixPluginsProduction()
        rid = vcp.execute_sync("nonexistent")
        assert vcp.sync_results[-1].error != ""

    def test_submit_async(self):
        vcp = V151VCPSixPluginsProduction()
        pid = vcp.register_plugin("p", [VCPPluginType.ASYNC])
        tid = vcp.submit_async(pid, query="async_test")
        assert tid in vcp.async_tasks
        assert vcp.async_tasks[tid].status == "pending"

    def test_complete_async(self):
        vcp = V151VCPSixPluginsProduction()
        pid = vcp.register_plugin("p", [VCPPluginType.ASYNC])
        tid = vcp.submit_async(pid)
        assert vcp.complete_async(tid, "done") is True
        assert vcp.async_tasks[tid].status == "success"

    def test_complete_async_unknown(self):
        vcp = V151VCPSixPluginsProduction()
        assert vcp.complete_async("nonexistent", "x") is False

    def test_add_context(self):
        vcp = V151VCPSixPluginsProduction()
        vcp.add_context(VCPContextType.SYNC_USER, "msg1")
        vcp.add_context(VCPContextType.ASYNC_USER, "msg2")
        vcp.add_context(VCPContextType.SUMMARY_USER, "msg3")
        vcp.add_context(VCPContextType.NOTIFICATION, "msg4")
        assert len(vcp.contexts[VCPContextType.SYNC_USER]) == 1
        assert len(vcp.contexts[VCPContextType.ASYNC_USER]) == 1
        assert len(vcp.contexts[VCPContextType.SUMMARY_USER]) == 1
        assert len(vcp.contexts[VCPContextType.NOTIFICATION]) == 1

    def test_notify(self):
        vcp = V151VCPSixPluginsProduction()
        vcp.notify("msg1", "AI")
        vcp.notify("msg2", "VCPLog")
        vcp.notify("msg3", "VCPInfo")
        assert vcp.n_notifications() == 3

    def test_stats(self):
        vcp = V151VCPSixPluginsProduction()
        pid = vcp.register_plugin("p", [VCPPluginType.SYNC])
        vcp.execute_sync(pid)
        stats = vcp.stats()
        assert stats["n_plugins"] == 1
        assert stats["version"] == V151_VERSION
        assert "philosophy" in stats


if __name__ == "__main__":
    pytest.main([__file__, "-v"])