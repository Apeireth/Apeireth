"""v30_async_dispatcher.py 真生产回归测试."""
from __future__ import annotations

import sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).parent.parent))
import pytest
from apeireth.v30_async_dispatcher import (
    V30_VERSION, PluginType, ContextType,
    AsyncTask, ContextObject, V30AsyncDispatcher,
)


class TestV30Enums:
    def test_plugin_type_count(self):
        assert len(list(PluginType)) == 6

    def test_context_type_count(self):
        assert len(list(ContextType)) == 4


class TestV30Helpers:
    def test_async_task_init(self):
        t = AsyncTask(task_id="x", name="y")
        assert t.status == "pending"

    def test_context_object_alive(self):
        c = ContextObject(ctx_id="x", ctx_type=ContextType.ASYNC_USER, payload="data")
        assert c.is_alive() is True

    def test_context_object_ttl_expired(self):
        c = ContextObject(ctx_id="x", ctx_type=ContextType.ASYNC_USER, payload="data", ttl_ms=10)
        import time
        time.sleep(0.05)
        assert c.is_alive() is False


class TestV30:
    def test_init(self):
        d = V30AsyncDispatcher()
        assert d.tasks == {}

    def test_register_plugin(self):
        d = V30AsyncDispatcher()
        d.register_plugin("P1", [PluginType.SYNC, PluginType.ASYNC])
        assert "P1" in d.plugin_manifests

    def test_submit_async_task(self):
        d = V30AsyncDispatcher()
        t = d.submit_async_task("video_gen")
        assert t.task_id in d.tasks

    def test_execute_async_task_success(self):
        d = V30AsyncDispatcher()
        t = d.submit_async_task("t", fn=lambda: "ok")
        result = d.execute_async_task(t.task_id)
        assert result.status == "success"
        assert result.result == "ok"

    def test_execute_async_task_failed(self):
        d = V30AsyncDispatcher()
        def bad(): raise ValueError("oops")
        t = d.submit_async_task("t", fn=bad)
        result = d.execute_async_task(t.task_id)
        assert result.status == "failed"
        assert "oops" in result.error

    def test_execute_async_task_unknown(self):
        d = V30AsyncDispatcher()
        with pytest.raises(ValueError):
            d.execute_async_task("nonexistent")

    def test_push_context(self):
        d = V30AsyncDispatcher()
        d.push_context(ContextType.ASYNC_USER, "data")
        assert len(d.context_objects) == 1

    def test_purge_ttl_context(self):
        d = V30AsyncDispatcher()
        d.push_context(ContextType.ASYNC_USER, "d", ttl_ms=10)
        d.push_context(ContextType.SYNC_USER, "d2", is_persistent=True)
        import time
        time.sleep(0.05)
        purged = d.purge_ttl_context()
        assert purged == 1

    def test_stats(self):
        d = V30AsyncDispatcher()
        d.register_plugin("P", [PluginType.SYNC])
        d.submit_async_task("t")
        stats = d.stats()
        assert stats["v3_philosophy_guard"] == "PASS"


if __name__ == "__main__":
    pytest.main([__file__, "-v"])