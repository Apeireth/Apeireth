"""v39_cross_domain_5.py 真生产回归测试."""
from __future__ import annotations

import sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).parent.parent))
import pytest
from apeireth.v39_cross_domain_5 import (
    V39_VERSION, DomainInsight, DOMAIN_INSIGHTS, V39CrossDomain5,
)


class TestV39:
    def test_init(self):
        s = V39CrossDomain5()
        assert len(s.insights) == 5

    def test_5_domains_present(self):
        s = V39CrossDomain5()
        domains = [i.domain for i in s.insights]
        for d in ["全栈开发", "攻防", "人文社科", "科研", "预测"]:
            assert d in domains

    def test_n_modules_total(self):
        s = V39CrossDomain5()
        n = s.n_modules_total()
        assert n >= 15

    def test_average_confidence(self):
        s = V39CrossDomain5()
        avg = s.average_confidence()
        assert 0.7 < avg < 1.0

    def test_render(self):
        s = V39CrossDomain5()
        text = s.render()
        assert "全栈开发" in text
        assert "主 23:12" in text

    def test_stats(self):
        s = V39CrossDomain5()
        stats = s.stats()
        assert stats["n_domains"] == 5
        assert stats["average_confidence"] > 0.7


if __name__ == "__main__":
    pytest.main([__file__, "-v"])