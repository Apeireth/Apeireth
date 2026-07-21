"""v35_4paradigms_integration.py 真生产回归测试."""
from __future__ import annotations

import sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).parent.parent))
import pytest
from apeireth.v35_4paradigms_integration import (
    V35_VERSION, VCP_4_PARADIGMS, ParadigmIntegration,
    PARADIGM_MAPPINGS, V354ParadigmsIntegration,
)


class TestV35:
    def test_vcp_paradigms(self):
        assert len(VCP_4_PARADIGMS) == 4
        assert "continuous_existence" in VCP_4_PARADIGMS
        assert "natural_perception" in VCP_4_PARADIGMS
        assert "autonomous_life" in VCP_4_PARADIGMS
        assert "integrated_ecosystem" in VCP_4_PARADIGMS

    def test_paradigm_mappings(self):
        assert len(PARADIGM_MAPPINGS) == 4
        for m in PARADIGM_MAPPINGS:
            assert m["paradigm"] in VCP_4_PARADIGMS
            assert len(m["modules"]) > 0

    def test_init(self):
        s = V354ParadigmsIntegration()
        assert len(s.integrations) == 4

    def test_total_modules(self):
        s = V354ParadigmsIntegration()
        n = s.total_modules_used()
        assert n >= 10

    def test_stats(self):
        s = V354ParadigmsIntegration()
        stats = s.stats()
        assert stats["n_paradigms"] == 4


if __name__ == "__main__":
    pytest.main([__file__, "-v"])