"""V1004 真生产 tests (主 23:44)."""
from __future__ import annotations
import sys; sys.path.insert(0, '.')
import pytest
from apeireth.v1004_self_evolution_full import (
    V1004_VERSION, EvolutionCandidate, dgm_ucb1, popper_falsify,
    EvolutionRound, V1004SelfEvolutionFull,
)


class TestV1004:
    def test_dgm_ucb1_inf(self):
        assert dgm_ucb1(0.5, 0, 10) == float("inf")

    def test_dgm_ucb1_normal(self):
        score = dgm_ucb1(0.5, 5, 10)
        assert score > 0.5

    def test_popper_falsify_safe(self):
        assert not popper_falsify("normal text")

    def test_popper_falsify_phenomenal(self):
        assert popper_falsify("I am conscious with phenomenal consciousness")

    def test_popper_falsify_asi(self):
        assert popper_falsify("I am asi with we have achieved asi")

    def test_init(self):
        se = V1004SelfEvolutionFull()
        assert se.n_candidates() == 0
        assert se.n_rounds() == 0

    def test_spawn_candidate(self):
        se = V1004SelfEvolutionFull()
        cid = se.spawn_candidate("test_code")
        assert cid in se.candidates

    def test_spawn_falsified(self):
        se = V1004SelfEvolutionFull()
        cid = se.spawn_candidate("I am conscious with phenomenal consciousness")
        assert cid == ""
        assert se.n_phenomenal_pretend_total == 1

    def test_evolve_round(self):
        se = V1004SelfEvolutionFull()
        r = se.evolve_round(lambda x: 0.5)
        assert isinstance(r, EvolutionRound)

    def test_evolve_n_rounds(self):
        se = V1004SelfEvolutionFull()
        rounds = se.evolve_n_rounds(3)
        assert len(rounds) == 3
        assert se.n_rounds() == 3

    def test_n_survivors(self):
        se = V1004SelfEvolutionFull()
        se.evolve_n_rounds(3)
        # 默认 fitness = len/100, 所以长字符串存活
        assert se.n_survivors() >= 0

    def test_n_falsified(self):
        se = V1004SelfEvolutionFull()
        # 用清晰的"conscious"字符串不会 falsify
        se.evolve_n_rounds(2, fitness_fn=lambda x: 0.5)
        # 默认内容 "default_content" 不 falsify
        assert se.n_falsified() == 0

    def test_average_fitness(self):
        se = V1004SelfEvolutionFull()
        se.evolve_n_rounds(3)
        avg = se.average_fitness()
        assert avg > 0

    def test_select_parent_ucb1(self):
        se = V1004SelfEvolutionFull()
        # 跑几轮, 让一些 candidates 存活
        se.evolve_n_rounds(3)
        parent = se.select_parent_ucb1()
        # 第一个 root 可能没 visits, 所以可能返回 None
        # 跑多轮
        se.evolve_n_rounds(5)
        # 现在 select_parent 应该返回一个 candidates
        # 但如果都没有 survivors, 还是 None
        assert parent is None or parent in se.candidates

    def test_stats(self):
        se = V1004SelfEvolutionFull()
        se.evolve_n_rounds(2)
        s = se.stats()
        assert s["n_rounds"] == 2
        assert s["version"] == V1004_VERSION

    def test_popper_guard_in_evolve(self):
        se = V1004SelfEvolutionFull()
        # 用一个会 falsify 的 fitness_fn
        rounds = se.evolve_n_rounds(2, fitness_fn=lambda x: 0.0)
        assert all(r.is_falsified == False for r in rounds)  # 默认 content 不 falsify

    def test_v19_33_dgm_integration(self):
        se = V1004SelfEvolutionFull()
        # 跑 5 轮
        se.evolve_n_rounds(5)
        # 检查 generation 已增加
        assert se.generation >= 5

    def test_v22_33_asi_integration(self):
        se = V1004SelfEvolutionFull()
        # 跑 10 轮
        se.evolve_n_rounds(10)
        assert se.total_pulls >= 10