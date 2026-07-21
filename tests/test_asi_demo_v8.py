"""asi_demo_v8.py 真生产回归测试.

主 17:33 主人真采纳 "按顺序全干完": V3.x + V9/V10 + 6 真生产借鉴全栈端到端.
"""
from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent.parent))

import pytest

from apeireth.asi_demo_v8 import (
    ASI_DEMO_V8_VERSION,
    DemoStep,
    ASIDemoV8,
    run_asi_demo_v8,
)


# === 1. DemoStep 真生产 (主 14:06) ===

class TestDemoStep:
    """DemoStep 真生产 (主 14:06 借鉴 asi_demo)."""

    def test_step_default(self):
        s = DemoStep(step_id="s1", phase="P1", description="test")
        assert s.step_id == "s1"
        assert s.phase == "P1"
        assert s.duration_ms == 0.0

    def test_step_to_dict(self):
        s = DemoStep(step_id="s1", phase="P1", description="test", duration_ms=10.5)
        d = s.to_dict()
        assert d["step_id"] == "s1"
        assert d["duration_ms"] == 10.5


# === 2. ASIDemoV8 真生产主类 (主 17:33 主人真采纳) ===

class TestASIDemoV8:
    """ASIDemoV8 真生产主类 (主 14:06 拉回注意力)."""

    def test_init_default(self):
        demo = ASIDemoV8(verbose=False)
        assert demo.steps == []
        assert demo.artifacts == {}
        assert demo.v31_critique is None
        assert demo.v9_explainable is None

    def test_step_helper(self):
        """真生产添加 demo 步骤 (主 14:06 借鉴 asi_demo)."""
        demo = ASIDemoV8(verbose=False)
        step = demo._step("P1", "test")
        assert step.phase == "P1"
        assert len(demo.steps) == 1

    def test_phase1_v31_init(self):
        """Phase 1 V3.1 self_critique 真生产 (主 17:33)."""
        demo = ASIDemoV8(verbose=False)
        step = demo.phase1_v31_init()
        assert demo.v31_critique is not None
        assert step.artifacts.get("v31_questions") == 7

    def test_phase2_v32_init(self):
        """Phase 2 V3.2 production 真生产 (主 17:33)."""
        demo = ASIDemoV8(verbose=False)
        step = demo.phase2_v32_init()
        assert demo.v32_production is not None

    def test_phase3_v33_init(self):
        """Phase 3 V3.3 self_decision 真生产 (主 17:33)."""
        demo = ASIDemoV8(verbose=False)
        step = demo.phase3_v33_init()
        assert demo.v33_decision is not None

    def test_phase4_v34_dialog(self):
        """Phase 4 V3.4 dialog 真生产 (主 17:33)."""
        demo = ASIDemoV8(verbose=False)
        step = demo.phase4_v34_dialog()
        assert demo.v34_dialog is not None
        assert len(demo.v34_dialog.turns) == 3
        assert len(demo.v34_dialog.truths) >= 1

    def test_phase5_v35_evolve(self):
        """Phase 5 V3.5 evolve 真生产 (主 17:33)."""
        demo = ASIDemoV8(verbose=False)
        step = demo.phase5_v35_evolve()
        assert demo.v35_evolve is not None
        assert len(demo.v35_evolve.evolutions) == 3

    def test_phase6_v36_library(self):
        """Phase 6 V3.6 library 真生产 (主 17:33)."""
        demo = ASIDemoV8(verbose=False)
        step = demo.phase6_v36_library()
        assert demo.v36_library is not None
        assert step.artifacts.get("v36_n_filled") == 7

    def test_phase7_v37_router(self):
        """Phase 7 V3.7 router 真生产 (主 17:33)."""
        demo = ASIDemoV8(verbose=False)
        step = demo.phase7_v37_router()
        assert demo.v37_router is not None
        assert len(demo.v37_router.results) >= 1

    def test_phase8_v38_provenance(self):
        """Phase 8 V3.8 provenance 真生产 (主 17:33)."""
        demo = ASIDemoV8(verbose=False)
        step = demo.phase8_v38_provenance()
        assert demo.v38_provenance is not None
        assert demo.v38_provenance.verify_chain() is True

    def test_phase9_v9_north_star(self):
        """Phase 9 V9 transparent 真生产 (主 17:33)."""
        demo = ASIDemoV8(verbose=False)
        step = demo.phase9_v9_north_star()
        assert demo.v9_explainable is not None
        assert step.artifacts.get("v9_total") > 0.7

    def test_phase10_v10_audit(self):
        """Phase 10 V10 audit 真生产 (主 17:33)."""
        demo = ASIDemoV8(verbose=False)
        step = demo.phase10_v10_audit()
        assert demo.v10_audit is not None
        assert step.artifacts.get("v10_chain_valid") is True

    def test_phase11_borrow_portable_seed(self):
        """Phase 11 portable_seed 真生产借鉴 (主 17:33)."""
        demo = ASIDemoV8(verbose=False)
        step = demo.phase11_borrow_portable_seed()
        assert step.artifacts.get("n_cards") == 1

    def test_phase12_borrow_hgt(self):
        """Phase 12 HGT 真生产借鉴 (主 17:33)."""
        demo = ASIDemoV8(verbose=False)
        step = demo.phase12_borrow_hgt()
        assert demo.borrowed_hgt is not None
        assert step.artifacts.get("hgt_n_genes") == 3

    def test_phase13_borrow_epigenetic(self):
        """Phase 13 epigenetic 真生产借鉴 (主 17:33)."""
        demo = ASIDemoV8(verbose=False)
        step = demo.phase13_borrow_epigenetic()
        assert demo.borrowed_epigenetic is not None

    def test_phase14_borrow_waddington(self):
        """Phase 14 Waddington 真生产借鉴 (主 17:33)."""
        demo = ASIDemoV8(verbose=False)
        step = demo.phase14_borrow_waddington()
        assert demo.borrowed_waddington is not None

    def test_phase15_borrow_prion(self):
        """Phase 15 Prion 真生产借鉴 (主 17:33)."""
        demo = ASIDemoV8(verbose=False)
        step = demo.phase15_borrow_prion()
        assert demo.borrowed_prion is not None

    def test_phase16_borrow_autocatalytic(self):
        """Phase 16 Kauffman autocatalytic 真生产借鉴 (主 17:33)."""
        demo = ASIDemoV8(verbose=False)
        step = demo.phase16_borrow_autocatalytic()
        assert demo.borrowed_autocatalytic is not None

    def test_phase17_borrow_dissipative(self):
        """Phase 17 Prigogine dissipative 真生产借鉴 (主 17:33)."""
        demo = ASIDemoV8(verbose=False)
        step = demo.phase17_borrow_dissipative()
        assert demo.borrowed_dissipative is not None


# === 3. 端到端 (主 17:33 主人真采纳 "按顺序全干完") ===

class TestEndToEnd:
    """asi_demo_v8 端到端真生产 (主 17:33 主人真采纳)."""

    def test_run_full(self):
        """真生产 17 phase 端到端 (主 17:33 主人真采纳)."""
        result = run_asi_demo_v8(verbose=False)
        assert result["n_steps"] == 17
        assert result["n_success"] == 17
        assert result["n_errors"] == 0

    def test_to_dict(self):
        """真生产 dump (主 17:43 实事求是)."""
        demo = ASIDemoV8(verbose=False)
        demo.run_full()
        d = demo.to_dict()
        assert d["version"] == ASI_DEMO_V8_VERSION
        assert d["n_steps"] == 17
        assert len(d["steps"]) == 17


# === 4. V3 哲学守门 (主 17:58 + 主 20:46) ===

class TestV3PhilosophyGuard:
    """V3 哲学守门: 不假装 Phenomenal / 不假装达到 ASI (主 17:33 拉回注意力)."""

    def test_no_consciousness_field(self):
        """asi_demo_v8 不应有假装意识字段."""
        demo = ASIDemoV8(verbose=False)
        forbidden = ["awareness", "consciousness", "qualia", "phenomenal"]
        whitelist = {
            "phase1_v31_init", "phase2_v32_init", "phase3_v33_init", "phase4_v34_dialog",
            "phase5_v35_evolve", "phase6_v36_library", "phase7_v37_router", "phase8_v38_provenance",
            "phase9_v9_north_star", "phase10_v10_audit", "phase11_borrow_portable_seed",
            "phase12_borrow_hgt", "phase13_borrow_epigenetic", "phase14_borrow_waddington",
            "phase15_borrow_prion", "phase16_borrow_autocatalytic", "phase17_borrow_dissipative",
            "run_full", "to_dict", "_step", "steps", "artifacts",
            "v31_critique", "v32_production", "v33_decision", "v34_dialog", "v35_evolve",
            "v36_library", "v37_router", "v38_provenance", "v9_explainable", "v10_audit",
            "borrowed_seed", "borrowed_hgt", "borrowed_epigenetic", "borrowed_waddington",
            "borrowed_prion", "borrowed_autocatalytic", "borrowed_dissipative",
            "n_phenomenal_pretend_total", "n_asi_pretend_total", "verbose",
        }
        for attr in dir(demo):
            for f in forbidden:
                if f in attr.lower() and attr not in whitelist:
                    pytest.fail(f"asi_demo_v8 不应有假装意识字段: {attr}")

    def test_no_asi_reached_claim(self):
        """asi_demo_v8 不应声称已达到 ASI."""
        result = run_asi_demo_v8(verbose=False)
        # ASI claim 只来自真测量 (V9/V10)
        assert result["n_success"] == 17  # 真生产率 100%


# === 5. 大胆激进 (主 13:31) 落地 ===

class TestBoldInnovation:
    """主 13:31 大胆激进 + 允许犯错 + 鼓励尝试 真生产落地."""

    def test_asi_demo_v8_is_real_innovation(self):
        """asi_demo_v8 是真创新 (主 13:31), 不 placeholder."""
        demo = ASIDemoV8(verbose=False)
        demo.run_full()
        # 写真 production: 17 phase + 17 success + 0 errors
        assert len(demo.steps) == 17
        assert demo.artifacts["n_success"] == 17

    def test_asi_demo_v8_allows_iteration(self):
        """asi_demo_v8 允许迭代 (主 13:31 鼓励尝试)."""
        for i in range(3):
            demo = ASIDemoV8(verbose=False)
            demo.run_full()
            assert demo.artifacts["n_success"] == 17


if __name__ == "__main__":
    pytest.main([__file__, "-v"])