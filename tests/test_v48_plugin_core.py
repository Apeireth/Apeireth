"""v48_plugin_core.py 真生产回归测试."""
from __future__ import annotations

import sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).parent.parent))
import pytest
from apeireth.v48_plugin_core import (
    V48_VERSION, CapabilityType, Capability, PluginManifest, V48PluginCore,
)


class TestV48:
    def test_init(self):
        c = V48PluginCore()
        assert c.capabilities == {}

    def test_create_capability(self):
        c = V48PluginCore()
        cap_id = c.create_capability("test", CapabilityType.READ)
        assert cap_id in c.capabilities

    def test_register_plugin(self):
        c = V48PluginCore()
        plugin_id = c.register_plugin(name="test")
        assert plugin_id in c.plugins

    def test_grant_capability(self):
        c = V48PluginCore()
        cap_id = c.create_capability("r", CapabilityType.READ)
        plugin_id = c.register_plugin(name="p")
        result = c.grant_capability(plugin_id, cap_id)
        assert result is True

    def test_grant_unknown(self):
        c = V48PluginCore()
        plugin_id = c.register_plugin(name="p")
        result = c.grant_capability(plugin_id, "unknown_cap")
        assert result is False

    def test_check_capability_granted(self):
        c = V48PluginCore()
        cap_id = c.create_capability("r", CapabilityType.READ, resource="memory")
        plugin_id = c.register_plugin(name="p")
        c.grant_capability(plugin_id, cap_id)
        assert c.check_capability(plugin_id, CapabilityType.READ, "memory") is True

    def test_check_capability_wildcard(self):
        c = V48PluginCore()
        cap_id = c.create_capability("r", CapabilityType.READ, resource="*")
        plugin_id = c.register_plugin(name="p")
        c.grant_capability(plugin_id, cap_id)
        assert c.check_capability(plugin_id, CapabilityType.READ, "anywhere") is True

    def test_check_capability_not_granted(self):
        c = V48PluginCore()
        plugin_id = c.register_plugin(name="p")
        assert c.check_capability(plugin_id, CapabilityType.NETWORK, "*") is False

    def test_check_capability_inactive(self):
        c = V48PluginCore()
        cap_id = c.create_capability("r", CapabilityType.READ)
        c.capabilities[cap_id].is_active = False
        plugin_id = c.register_plugin(name="p")
        c.grant_capability(plugin_id, cap_id)
        assert c.check_capability(plugin_id, CapabilityType.READ, "*") is False

    def test_stats(self):
        c = V48PluginCore()
        c.create_capability("r", CapabilityType.READ)
        stats = c.stats()
        assert stats["n_capabilities"] == 1


if __name__ == "__main__":
    pytest.main([__file__, "-v"])