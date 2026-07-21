"""V1001 真生产 tests (主 23:44 真采纳)."""
from __future__ import annotations
import sys; sys.path.insert(0, '.')
import pytest
from apeireth.v1001_vcp_six_plugins_full import (
    V1001_VERSION, VCPPluginType, VCPContextType, V1001VCPSixPluginsFull,
)


class TestV1001:
    def test_init(self):
        vcp = V1001VCPSixPluginsFull()
        assert vcp.n_plugins() == 0
        assert vcp.n_context_total() == 0

    def test_register_plugin(self):
        vcp = V1001VCPSixPluginsFull()
        pid = vcp.register_plugin("test", [VCPPluginType.SYNC])
        assert pid in vcp.plugins

    def test_register_all_6_types(self):
        vcp = V1001VCPSixPluginsFull()
        pid = vcp.register_plugin(
            "all6", list(VCPPluginType),
            capabilities=["c1", "c2", "c3", "c4", "c5", "c6"],
        )
        assert len(vcp.plugins[pid].types) == 6

    def test_execute_sync(self):
        vcp = V1001VCPSixPluginsFull()
        pid = vcp.register_plugin("t", [VCPPluginType.SYNC], fn=lambda x: x * 2)
        r = vcp.execute_sync(pid, 5)
        assert r.result == 10
        assert r.error == ""

    def test_execute_sync_unknown(self):
        vcp = V1001VCPSixPluginsFull()
        r = vcp.execute_sync("nonexistent")
        assert r.error != ""

    def test_execute_sync_wrong_type(self):
        vcp = V1001VCPSixPluginsFull()
        pid = vcp.register_plugin("t", [VCPPluginType.ASYNC])
        r = vcp.execute_sync(pid, "x")
        assert "not support sync" in r.error

    def test_execute_sync_uses_sync_user_context(self):
        vcp = V1001VCPSixPluginsFull()
        pid = vcp.register_plugin("t", [VCPPluginType.SYNC])
        vcp.execute_sync(pid, "x")
        assert len(vcp.context_objects[VCPContextType.SYNC_USER]) == 1

    def test_submit_async(self):
        vcp = V1001VCPSixPluginsFull()
        pid = vcp.register_plugin("t", [VCPPluginType.ASYNC])
        tid = vcp.submit_async(pid, args={"q": "x"})
        assert tid in vcp.plugins[pid].async_tasks
        assert len(vcp.context_objects[VCPContextType.ASYNC_USER]) == 1

    def test_submit_async_uses_async_user_context(self):
        vcp = V1001VCPSixPluginsFull()
        pid = vcp.register_plugin("t", [VCPPluginType.ASYNC])
        tid = vcp.submit_async(pid)
        assert vcp.context_objects[VCPContextType.ASYNC_USER][0].ttl_ms == 60000

    def test_complete_async(self):
        vcp = V1001VCPSixPluginsFull()
        pid = vcp.register_plugin("t", [VCPPluginType.ASYNC])
        tid = vcp.submit_async(pid)
        vcp.complete_async_task(tid, result="done", status="success")
        task = vcp.plugins[pid].async_tasks[tid]
        assert task.status == "success"
        assert task.result == "done"
        assert task.progress == 1.0

    def test_complete_async_summary_context(self):
        vcp = V1001VCPSixPluginsFull()
        pid = vcp.register_plugin("t", [VCPPluginType.ASYNC])
        tid = vcp.submit_async(pid)
        vcp.complete_async_task(tid, result="done")
        assert len(vcp.context_objects[VCPContextType.SUMMARY_USER]) == 1

    def test_push_context(self):
        vcp = V1001VCPSixPluginsFull()
        cid = vcp.push_context(VCPContextType.SYNC_USER, "data", is_persistent=True)
        assert cid in [c.ctx_id for c in vcp.context_objects[VCPContextType.SYNC_USER]]

    def test_purge_expired(self):
        vcp = V1001VCPSixPluginsFull()
        vcp.push_context(VCPContextType.ASYNC_USER, "data", ttl_ms=10)
        vcp.push_context(VCPContextType.SYNC_USER, "data2", is_persistent=True)
        import time as t
        t.sleep(0.05)
        purged = vcp.purge_expired_contexts()
        assert purged == 1
        # 持久的保留
        assert len(vcp.context_objects[VCPContextType.SYNC_USER]) == 1

    def test_notify_3_systems(self):
        vcp = V1001VCPSixPluginsFull()
        vcp.notify("ai msg", audience="AI")
        vcp.notify("log msg", audience="VCPLog")
        vcp.notify("info msg", audience="VCPInfo")
        assert vcp.n_notifications == 3
        assert sum(1 for n in vcp.notifications if n.audience == "AI") == 1
        assert sum(1 for n in vcp.notifications if n.audience == "VCPLog") == 1
        assert sum(1 for n in vcp.notifications if n.audience == "VCPInfo") == 1

    def test_stats(self):
        vcp = V1001VCPSixPluginsFull()
        pid = vcp.register_plugin("t", [VCPPluginType.SYNC])
        vcp.execute_sync(pid, "x")
        s = vcp.stats()
        assert s["n_plugins"] == 1
        assert s["n_sync_executions"] == 1
        assert s["version"] == V1001_VERSION

    def test_3_audience_integration(self):
        vcp = V1001VCPSixPluginsFull()
        vcp.notify("Apeireth 真生产借鉴", audience="AI")
        vcp.notify("用户操作审计", audience="VCPLog")
        vcp.notify("frame 30/120", audience="VCPInfo")
        audiences = [n.audience for n in vcp.notifications]
        assert set(audiences) == {"AI", "VCPLog", "VCPInfo"}

    def test_all_4_context_types(self):
        vcp = V1001VCPSixPluginsFull()
        vcp.push_context(VCPContextType.ASYNC_USER, "a")
        vcp.push_context(VCPContextType.SYNC_USER, "b")
        vcp.push_context(VCPContextType.SUMMARY_USER, "c")
        vcp.push_context(VCPContextType.NOTIFICATION, "d")
        assert vcp.n_context_total() == 4

    def test_preprocessor_plugin(self):
        vcp = V1001VCPSixPluginsFull()
        pid = vcp.register_plugin("pre", [VCPPluginType.PREPROCESSOR],
                                  fn=lambda x: f"pre_{x}")
        assert pid in vcp.plugins

    def test_service_plugin(self):
        vcp = V1001VCPSixPluginsFull()
        pid = vcp.register_plugin("ws", [VCPPluginType.SERVICE])
        assert pid in vcp.plugins

    def test_static_plugin(self):
        vcp = V1001VCPSixPluginsFull()
        pid = vcp.register_plugin("weather", [VCPPluginType.STATIC])
        assert pid in vcp.plugins

    def test_hybrid_plugin(self):
        vcp = V1001VCPSixPluginsFull()
        pid = vcp.register_plugin("hybrid",
                                  [VCPPluginType.SYNC, VCPPluginType.ASYNC, VCPPluginType.HYBRID])
        # Hybrid should support both sync and async
        r = vcp.execute_sync(pid, "x")
        assert r.error == ""
        tid = vcp.submit_async(pid)
        assert tid != ""