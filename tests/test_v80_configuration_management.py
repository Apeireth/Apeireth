"""v80_configuration_management.py 真生产回归测试."""
from __future__ import annotations

import sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).parent.parent))
import pytest
from apeireth.v80_configuration_management import (
    V80_VERSION, ConfigValue, ConfigSnapshot, V80ConfigurationManagement,
)


class TestV80:
    def test_init(self):
        cm = V80ConfigurationManagement()
        assert cm.configs == {}

    def test_set_get(self):
        cm = V80ConfigurationManagement()
        cm.set("key1", "value1")
        assert cm.get("key1") == "value1"

    def test_get_default(self):
        cm = V80ConfigurationManagement()
        assert cm.get("nonexistent", "default") == "default"

    def test_overridable(self):
        cm = V80ConfigurationManagement()
        cm.set("key1", "v", is_overridable=False)
        assert cm.n_overridable() == 0

    def test_snapshot(self):
        cm = V80ConfigurationManagement()
        cm.set("a", 1)
        cm.set("b", 2)
        snap_id = cm.snapshot()
        assert cm.n_snapshots() == 1
        assert snap_id.startswith("snap_")

    def test_multiple_snapshots(self):
        cm = V80ConfigurationManagement()
        cm.snapshot()
        cm.snapshot()
        cm.snapshot()
        assert cm.n_snapshots() == 3

    def test_stats(self):
        cm = V80ConfigurationManagement()
        cm.set("a", 1)
        stats = cm.stats()
        assert stats["n_configs"] == 1


if __name__ == "__main__":
    pytest.main([__file__, "-v"])