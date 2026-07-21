"""v64_rust_preparation.py 真生产回归测试."""
from __future__ import annotations

import sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).parent.parent))
import pytest
from apeireth.v64_rust_preparation import (
    V64_VERSION, RustCrate, RustCrateSpec, RUST_CRATES,
    RustMigrationPlan, RUST_MIGRATION_PLANS, V64RustPreparation,
)


class TestV64:
    def test_init(self):
        rp = V64RustPreparation()
        assert rp.n_crates() == 6

    def test_6_crates(self):
        rp = V64RustPreparation()
        for expected in ["tokio", "sqlx", "sled", "arrow-rs", "tantivy", "delta-rs"]:
            assert expected in rp.crate_specs

    def test_6_plans(self):
        rp = V64RustPreparation()
        assert rp.n_plans() == 6

    def test_n_poc(self):
        rp = V64RustPreparation()
        assert rp.n_poc() >= 3

    def test_n_production(self):
        rp = V64RustPreparation()
        assert rp.n_production() >= 1

    def test_stats(self):
        rp = V64RustPreparation()
        stats = rp.stats()
        assert stats["n_crates"] == 6
        assert stats["n_plans"] == 6


if __name__ == "__main__":
    pytest.main([__file__, "-v"])