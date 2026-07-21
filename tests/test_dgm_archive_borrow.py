"""dgm_archive.py DGM borrow regression tests.

主 9:41 round-19 source-deep-read 推荐:
  下周 (高价值): DGM diagnose->fix 分离 + score_child_prop (重塑 self_evolving.py + dgm_archive.py 灵魂)

借鉴自 DGM (Darwin Gödel Machine, arxiv 2505.22954):
  1. score_child_prop 反收敛核心: selection_score = eval_score * 1/(1+children_count)
     显式鼓励探索低子代节点, 避免 best_score 局部最优
  2. diagnose->fix 分离: reviewer LLM 输出 structured proposed_fixes JSON,
     validator apply, 不让 LLM 直接改代码 (GitHub PR 模式)

本测试锁住:
  1. Generation dataclass 有 selection_score + proposed_fixes + applied_fixes 字段
  2. branch() 计算 selection_score = eval_score / (1+parent_child_count_before)
  3. select_parent_for_branching() 选 selection_score 最高的非 root
  4. propose_fix() + apply_fix() diagnose->fix 分离 (无 validator signature 拒绝)
  5. best_gen_id 仍然按 raw eval_score (selection_score 是探索指标, best 是质量指标)
  6. V2 哲学守门: 不假装 Phenomenal
"""
from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent.parent))

import pytest

from apeireth.dgm_archive import DGMArchive, Generation
from apeireth.self_evolving import Harness


# === 1. Generation dataclass 新字段测试 ===

class TestGenerationNewFields:
    """借鉴 DGM (主 9:41 round-19): Generation 新增 selection_score + proposed/applied_fixes."""

    def test_generation_has_selection_score(self):
        g = Generation(gen_id="g1", parent_gen_id=None, harness=Harness())
        assert hasattr(g, "selection_score")
        assert isinstance(g.selection_score, float)

    def test_generation_has_proposed_fixes(self):
        g = Generation(gen_id="g1", parent_gen_id=None, harness=Harness())
        assert hasattr(g, "proposed_fixes")
        assert isinstance(g.proposed_fixes, list)
        assert len(g.proposed_fixes) == 0

    def test_generation_has_applied_fixes(self):
        g = Generation(gen_id="g1", parent_gen_id=None, harness=Harness())
        assert hasattr(g, "applied_fixes")
        assert isinstance(g.applied_fixes, list)
        assert len(g.applied_fixes) == 0


# === 2. branch() selection_score 计算测试 (反收敛核心) ===

class TestBranchSelectionScore:
    """借鉴 DGM (主 9:41 round-19): branch() 计算 selection_score = eval_score / (1+parent_child_count_before)."""

    def test_first_child_selection_score_equals_eval_score(self, tmp_path):
        """parent 没孩子时, 第 1 个 child selection_score = eval_score."""
        archive = DGMArchive()
        h = Harness()
        root_id = archive.init_root(h)
        new_h = Harness()
        # root 没孩子, parent_child_count_before = 0, 分母 = 1
        new_id = archive.branch(root_id, new_h, [], 0.5, {})
        gen = archive.generations[new_id]
        assert gen.selection_score == pytest.approx(0.5)

    def test_second_child_selection_score_decays(self, tmp_path):
        """parent 有 1 个孩子时, 第 2 个 child selection_score = eval_score / 2."""
        archive = DGMArchive()
        h = Harness()
        root_id = archive.init_root(h)
        # 第 1 个 child: parent_child_count_before = 0 → 1.0 * eval
        c1 = archive.branch(root_id, Harness(), [], 0.8, {})
        # 第 2 个 child: parent_child_count_before = 1 → 0.5 * eval
        c2 = archive.branch(root_id, Harness(), [], 0.8, {})
        gen_c2 = archive.generations[c2]
        assert gen_c2.selection_score == pytest.approx(0.4)

    def test_third_child_selection_score_third(self, tmp_path):
        """parent 有 2 个孩子时, 第 3 个 child selection_score = eval_score / 3."""
        archive = DGMArchive()
        h = Harness()
        root_id = archive.init_root(h)
        archive.branch(root_id, Harness(), [], 0.6, {})  # child 1
        archive.branch(root_id, Harness(), [], 0.6, {})  # child 2
        c3 = archive.branch(root_id, Harness(), [], 0.6, {})  # child 3
        gen_c3 = archive.generations[c3]
        assert gen_c3.selection_score == pytest.approx(0.2)

    def test_best_gen_id_still_uses_eval_score(self, tmp_path):
        """best_gen_id 仍按 raw eval_score, 不受 selection_score 影响 (反收敛 vs 质量)."""
        archive = DGMArchive()
        h = Harness()
        root_id = archive.init_root(h)
        # 两个 child eval_score 都 0.5, 但 selection_score 不同
        c1 = archive.branch(root_id, Harness(), [], 0.5, {})
        c2 = archive.branch(root_id, Harness(), [], 0.5, {})
        # best_gen_id 应该是 eval_score 最高的, 两者并列
        # selection_score: c1 = 0.5, c2 = 0.25
        # 验证 best_gen_id 仍然是 c1 或 c2, 不是按 selection_score 排序
        best = archive.generations[archive.best_gen_id]
        assert best.eval_score == 0.5

    def test_high_eval_score_child_chosen_as_best_even_if_low_selection(self, tmp_path):
        """raw eval_score 高的 child 即使 selection_score 低 (作为第 N 个 child) 仍是 best."""
        archive = DGMArchive()
        h = Harness()
        root_id = archive.init_root(h)
        # 第 1 个 child eval_score 0.5 (selection_score 0.5)
        archive.branch(root_id, Harness(), [], 0.5, {})
        # 第 2 个 child eval_score 0.9 (selection_score 0.45)  ← raw 更高
        c2 = archive.branch(root_id, Harness(), [], 0.9, {})
        # best 应该是 c2
        assert archive.best_gen_id == c2


# === 3. select_parent_for_branching() 反收敛核心测试 ===

class TestSelectParentForBranching:
    """借鉴 DGM (主 9:41 round-19): 选 selection_score 最高的非 root 作为 parent."""

    def test_only_root_returns_root(self, tmp_path):
        archive = DGMArchive()
        h = Harness()
        root_id = archive.init_root(h)
        assert archive.select_parent_for_branching() == root_id

    def test_picks_highest_selection_score(self, tmp_path):
        archive = DGMArchive()
        h = Harness()
        root_id = archive.init_root(h)
        # 第 1 个 child: selection_score = 0.8
        c1 = archive.branch(root_id, Harness(), [], 0.8, {})
        # 第 2 个 child of root: selection_score = 0.5 / 2 = 0.25
        c2 = archive.branch(root_id, Harness(), [], 0.5, {})
        # 第 3 个 child of root: selection_score = 0.5 / 3 ≈ 0.167
        c3 = archive.branch(root_id, Harness(), [], 0.5, {})
        # c1 selection_score 最高 (0.8), 应该选 c1
        assert archive.select_parent_for_branching() == c1

    def test_after_branching_more_picks_low_child_count(self, tmp_path):
        """反收敛核心: 已分支的节点 selection_score 衰减, 探索低子代节点."""
        archive = DGMArchive()
        h = Harness()
        root_id = archive.init_root(h)
        # root 有 3 个 children: selection_scores 0.5, 0.25, 0.167
        c1 = archive.branch(root_id, Harness(), [], 0.5, {})
        c2 = archive.branch(root_id, Harness(), [], 0.5, {})
        c3 = archive.branch(root_id, Harness(), [], 0.5, {})
        # c1 自己还没有 children → 加一个 child
        grandchild = archive.branch(c1, Harness(), [], 0.3, {})
        # 现在 c1 的 selection_score 还是 0.5 (创建时 calc, 不更新)
        # grandchild selection_score = 0.3 / (1+0) = 0.3
        # 但 c2 还没孩子, 如果 c2 加一个 child, 它的 selection_score 会 = eval_score / 1 = eval_score
        # 反收敛效果: 已经分支的 c1 后续 children 衰减, 探索还没分支的 c2/c3 更有价值
        assert archive.select_parent_for_branching() == c1  # c1 selection_score 0.5 仍最高


# === 4. diagnose→fix 分离测试 (Round-19 核心借鉴) ===

class TestDiagnoseFixSeparation:
    """借鉴 DGM (主 9:41 round-19): propose_fix + apply_fix 分离, validator 必须签名."""

    def test_propose_fix_stores_to_proposed_fixes(self, tmp_path):
        archive = DGMArchive()
        h = Harness()
        root_id = archive.init_root(h)
        fix = {
            "type": "modify_harness",
            "target": "sct_weights",
            "key": "调度者",
            "delta": {"motivational": 0.1},
            "rationale": "increase motivation",
            "confidence": 0.8,
        }
        archive.propose_fix(root_id, fix)
        gen = archive.generations[root_id]
        assert len(gen.proposed_fixes) == 1
        assert gen.proposed_fixes[0]["type"] == "modify_harness"
        # applied_fixes 仍空
        assert len(gen.applied_fixes) == 0

    def test_apply_fix_without_validator_rejected(self, tmp_path):
        """主 22:08 V2: 必须 validator signature, 不能直接 apply."""
        archive = DGMArchive()
        h = Harness()
        root_id = archive.init_root(h)
        fix = {"type": "noop"}
        archive.propose_fix(root_id, fix)
        # 没有 validator_signature → 拒绝
        result = archive.apply_fix(root_id, 0, validator_signature="")
        assert result is False
        gen = archive.generations[root_id]
        assert len(gen.applied_fixes) == 0  # 没 apply

    def test_apply_fix_with_validator_succeeds(self, tmp_path):
        archive = DGMArchive()
        h = Harness()
        root_id = archive.init_root(h)
        fix = {"type": "modify_harness", "rationale": "test"}
        archive.propose_fix(root_id, fix)
        result = archive.apply_fix(root_id, 0, validator_signature="test_run_passed_xyz")
        assert result is True
        gen = archive.generations[root_id]
        assert len(gen.applied_fixes) == 1
        assert gen.applied_fixes[0]["validator"] == "test_run_passed_xyz"

    def test_apply_fix_invalid_index_rejected(self, tmp_path):
        archive = DGMArchive()
        h = Harness()
        root_id = archive.init_root(h)
        # 不 propose_fix 直接 apply → 拒绝
        result = archive.apply_fix(root_id, 0, validator_signature="sig")
        assert result is False

    def test_apply_fix_invalid_gen_id_rejected(self, tmp_path):
        archive = DGMArchive()
        result = archive.apply_fix("nonexistent_gen", 0, validator_signature="sig")
        assert result is False


# === 5. stats() 扩展测试 ===

class TestStatsExtended:
    """借鉴 DGM: stats() 暴露 propose/applied fixes 计数."""

    def test_stats_include_fixes_count(self, tmp_path):
        archive = DGMArchive()
        h = Harness()
        root_id = archive.init_root(h)
        archive.propose_fix(root_id, {"type": "a"})
        archive.propose_fix(root_id, {"type": "b"})
        archive.apply_fix(root_id, 0, validator_signature="sig")
        stats = archive.stats()
        assert stats["n_proposed_fixes"] == 2
        assert stats["n_applied_fixes"] == 1


# === 6. 向后兼容测试 ===

class TestBackwardCompatibility:
    """确保 DGM borrow 不破坏现有行为."""

    def test_init_root_still_works(self, tmp_path):
        archive = DGMArchive()
        h = Harness()
        root_id = archive.init_root(h)
        assert archive.root_gen_id == root_id
        assert archive.best_gen_id == root_id

    def test_branch_basic_still_works(self, tmp_path):
        archive = DGMArchive()
        h = Harness()
        root_id = archive.init_root(h)
        new_id = archive.branch(root_id, Harness(), [], 0.5, {})
        assert new_id in archive.generations
        assert root_id in archive.generations[new_id].parent_gen_id

    def test_get_lineage_still_works(self, tmp_path):
        archive = DGMArchive()
        root_id = archive.init_root(Harness())
        c1 = archive.branch(root_id, Harness(), [], 0.5, {})
        c2 = archive.branch(c1, Harness(), [], 0.5, {})
        lineage = archive.get_lineage(c2)
        assert lineage == [root_id, c1, c2]


# === 7. V2 哲学守门测试 ===

class TestV2PhilosophyGuard:
    """V2 哲学守门 (主 22:08): 不假装 Phenomenal."""

    def test_no_consciousness_fields(self, tmp_path):
        g = Generation(gen_id="g1", parent_gen_id=None, harness=Harness())
        forbidden = ["awareness", "consciousness", "qualia", "phenomenal",
                     "self_aware", "subjective_experience"]
        for f in forbidden:
            assert not hasattr(g, f), f"Generation 不应有假装意识字段 {f}"

    def test_selection_score_is_metric_not_consciousness(self, tmp_path):
        """selection_score 是探索指标, 不假装意识."""
        g = Generation(gen_id="g1", parent_gen_id=None, harness=Harness(), eval_score=0.5)
        # selection_score 默认 0 (init_root 会重算为 eval_score)
        assert g.selection_score >= 0
        assert isinstance(g.selection_score, float)


if __name__ == "__main__":
    pytest.main([__file__, "-v"])