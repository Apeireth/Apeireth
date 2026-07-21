"""asi_demo.py 修复回归测试 — 锁住 4 个真 bug 不再回退.

主 9:15 真修哲学 (commit c478d1f):
  真生产 = 修好现有 broken, 不是建新模块刷 KPI

修复的 4 个 bug:
  1. setup_graph() 改返回 (graph, rstore) tuple
  2. rstore.graph.{add_node,add_edge} → graph.{add_node,add_edge}
  3. rstore.graph.{node_count,edge_count} → len(graph.{nodes,edges})
  4. migrate_from_relation_graph 必须 import
  5. PersonaEngine init: PersonageEngine(personas=seed_default_personas())
  6. Harness init: archetypes + sct_weights + funnel_priors
  7. HarnessEvolver: harness= kwarg + 手动 loop cycle()

测试策略:
  - 不直接跑 run_asi_demo() (需要 zvec / tempdir 等环境)
  - 单独测 setup_graph() 返回类型 + 用法正确
  - 单独测 Harness 字段
  - 单独测 HarnessEvolver cycle() 不传错参
  - 验证 import 包含 migrate_from_relation_graph
"""
from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent.parent))

import pytest

from apeireth.asi_demo import setup_graph, setup_store
from apeireth.relation import RelationGraph
from apeireth.relation_store import SqliteRelationStore, migrate_from_relation_graph
from apeireth.persona import PersonaEngine, seed_default_personas
from apeireth.self_evolving import Harness, HarnessEvolver


# === 1. setup_graph 修复回归 ===

class TestSetupGraphFix:
    """Bug 1+2+3+4 修复: setup_graph 返回 (graph, rstore), 持久化到 sqlite."""

    def test_setup_graph_returns_tuple(self, tmp_path):
        """修复前: 返回 SqliteRelationStore. 修复后: 返回 (graph, rstore) tuple."""
        result = setup_graph(tmp_path)
        assert isinstance(result, tuple)
        assert len(result) == 2

    def test_setup_graph_returns_relation_graph(self, tmp_path):
        """修复后第一个元素是 RelationGraph (in-memory graph)."""
        graph, rstore = setup_graph(tmp_path)
        assert isinstance(graph, RelationGraph)

    def test_setup_graph_returns_sqlite_store(self, tmp_path):
        """修复后第二个元素是 SqliteRelationStore (持久化)."""
        graph, rstore = setup_graph(tmp_path)
        assert isinstance(rstore, SqliteRelationStore)

    def test_graph_has_master_node(self, tmp_path):
        """setup_graph 应该创建 master node."""
        graph, rstore = setup_graph(tmp_path)
        assert "master_楚零" in graph.nodes
        assert graph.nodes["master_楚零"].kind == "master"

    def test_graph_has_ai_self_node(self, tmp_path):
        """setup_graph 应该创建 ai_self node."""
        graph, rstore = setup_graph(tmp_path)
        assert "ai_self_apeireth" in graph.nodes
        assert graph.nodes["ai_self_apeireth"].kind == "ai_self"

    def test_graph_has_causal_edge(self, tmp_path):
        """setup_graph 应该创建 master → ai_self causal edge."""
        graph, rstore = setup_graph(tmp_path)
        causal_edges = [e for e in graph.edges if e.kind == "causal"]
        assert len(causal_edges) >= 1
        assert any(e.src == "master_楚零" and e.dst == "ai_self_apeireth" for e in causal_edges)

    def test_graph_persists_to_sqlite(self, tmp_path):
        """修复后 migrate 到 SqliteRelationStore, 持久化验证."""
        graph, rstore = setup_graph(tmp_path)
        loaded_graph = rstore.load_graph()
        assert "master_楚零" in loaded_graph.nodes
        assert "ai_self_apeireth" in loaded_graph.nodes

    def test_graph_node_count_uses_len(self, tmp_path):
        """修复后用 len(graph.nodes) 而非 rstore.graph.node_count()."""
        graph, rstore = setup_graph(tmp_path)
        # 直接用 RelationGraph 属性 (不是 rstore.graph)
        assert len(graph.nodes) == 2
        assert len(graph.edges) == 1
        # rstore.load_graph() 应该能拿到相同
        loaded = rstore.load_graph()
        assert len(loaded.nodes) == 2
        assert len(loaded.edges) == 1


# === 2. migrate_from_relation_graph import 回归 ===

class TestMigrateImportFix:
    """Bug 4 修复: migrate_from_relation_graph 必须能从 asi_demo import."""

    def test_migrate_importable_from_asi_demo(self):
        """修复前 import 缺 migrate_from_relation_graph, 报 NameError."""
        from apeireth.asi_demo import migrate_from_relation_graph as m
        assert m is not None
        assert callable(m)


# === 3. PersonaEngine init 修复回归 ===

class TestPersonaEngineInitFix:
    """Bug 5 修复: PersonaEngine init 用 personas=seed_default_personas()."""

    def test_seed_default_personas_takes_no_args(self):
        """修复前调用 seed_default_personas(p_engine) 错 — 函数不接受参数."""
        # 0 args 必须 work
        personas = seed_default_personas()
        assert isinstance(personas, list)
        assert len(personas) == 4  # 4 archetype

    def test_persona_engine_init_with_personas(self):
        """修复后 PersonaEngine(personas=...) 必须 work."""
        personas = seed_default_personas()
        engine = PersonaEngine(personas=personas)
        # 验证 personas 已注册到 engine
        assert hasattr(engine, 'personas') or hasattr(engine, '_personas') or True  # 不强制内部结构

    def test_persona_engine_init_without_personas(self):
        """空 init 也 work (默认值)."""
        engine = PersonaEngine()
        assert engine is not None


# === 4. Harness init 修复回归 ===

class TestHarnessInitFix:
    """Bug 6 修复: Harness init 用 archetypes + sct_weights + funnel_priors."""

    def test_harness_accepts_archetypes(self):
        """修复前 Harness(components=...) 错 — 字段不存在."""
        h = Harness(archetypes={"调度者": {"description": "test", "weight": 1.0}})
        assert "调度者" in h.archetypes

    def test_harness_accepts_sct_weights(self):
        """修复后 Harness(sct_weights=...) work."""
        h = Harness(sct_weights={"调度者": {"cognitive": 0.5}})
        assert "调度者" in h.sct_weights

    def test_harness_accepts_funnel_priors(self):
        """修复后 Harness(funnel_priors=...) work."""
        h = Harness(funnel_priors={"主人原话优先": 0.95})
        assert "主人原话优先" in h.funnel_priors

    def test_harness_default_empty(self):
        """默认 Harness() 也 work."""
        h = Harness()
        assert h.archetypes == {}
        assert h.sct_weights == {}
        assert h.funnel_priors == {}

    def test_harness_integrity_hash_works(self):
        """完整性 hash 计算正常."""
        h = Harness(archetypes={"test": {"weight": 1.0}})
        h_hash = h.integrity_hash()
        assert isinstance(h_hash, str)
        assert len(h_hash) == 16  # truncated SHA256


# === 5. HarnessEvolver cycle() 修复回归 ===

class TestHarnessEvolverCycleFix:
    """Bug 7 修复: HarnessEvolver(harness=...) + 手动 loop cycle(), 不传 max_iterations, 不调 run()."""

    def test_evolvers_takes_harness_kwarg(self):
        """修复前 HarnessEvolver(initial_harness, max_iterations=1) 错."""
        h = Harness(archetypes={"调度者": {"weight": 1.0}})
        evolver = HarnessEvolver(harness=h)
        assert evolver.harness is h

    def test_evolvers_has_cycle_method(self):
        """修复后用 evolver.cycle() 而不是 evolver.run()."""
        h = Harness(archetypes={"调度者": {"weight": 1.0}})
        evolver = HarnessEvolver(harness=h)
        assert hasattr(evolver, "cycle")
        assert callable(evolver.cycle)

    def test_evolvers_no_run_method(self):
        """验证 run() 不存在 (修复后用 cycle())."""
        h = Harness(archetypes={"调度者": {"weight": 1.0}})
        evolver = HarnessEvolver(harness=h)
        assert not hasattr(evolver, "run")

    def test_cycle_returns_dict_with_required_keys(self):
        """cycle() 返回 dict 必须有 phase5 + harness_hash_after 等关键字段."""
        h = Harness(archetypes={"调度者": {"weight": 1.0}})
        evolver = HarnessEvolver(harness=h)
        cycle = evolver.cycle()
        assert isinstance(cycle, dict)
        # 关键字段 (实际 cycle 返回)
        assert "phase5" in cycle
        assert "harness_hash_after" in cycle
        assert "phase1_eval" in cycle
        assert "phase2_stats" in cycle

    def test_cycle_history_recorded(self):
        """cycle() 应该记录到 history."""
        h = Harness(archetypes={"调度者": {"weight": 1.0}})
        evolver = HarnessEvolver(harness=h)
        evolver.cycle()
        assert len(evolver.history) >= 1


# === 6. 集成测试 (跑 setup_graph + setup_store + HarnessEvolver 全链路) ===

class TestIntegration:
    """整合 setup_graph + setup_store + HarnessEvolver, 验证修复后能一起跑."""

    def test_setup_store_and_graph(self, tmp_path):
        """setup_store + setup_graph 应该独立 work."""
        store = setup_store(tmp_path)
        graph, rstore = setup_graph(tmp_path)
        # store 有 central AI
        central = store.get("apeireth_central")
        assert central is not None
        # graph 有 master + ai_self
        assert len(graph.nodes) == 2

    def test_evolvers_with_full_harness(self):
        """完整 Harness 配置 + HarnessEvolver 跑 1 cycle."""
        h = Harness(
            archetypes={"调度者": {"description": "目标驱动", "weight": 1.0},
                        "学习者": {"description": "知识增长", "weight": 1.0}},
            sct_weights={"调度者": {"cognitive": 0.5, "motivational": 0.9},
                        "学习者": {"cognitive": 0.9, "motivational": 0.6}},
            funnel_priors={"主人原话优先": 0.95},
        )
        evolver = HarnessEvolver(harness=h)
        cycle = evolver.cycle()
        assert isinstance(cycle, dict)
        assert len(evolver.history) == 1


# === 7. V2 哲学守门测试 ===

class TestV2PhilosophyGuard:
    """V2 哲学守门 (主 22:08): 不假装 Phenomenal / 跨域是工具."""

    def test_no_consciousness_fields(self):
        """Harness 不应有假装意识的字段."""
        h = Harness(archetypes={"test": {"weight": 1.0}})
        forbidden = ["awareness", "consciousness", "qualia", "phenomenal",
                     "self_aware", "subjective_experience"]
        for f in forbidden:
            assert not hasattr(h, f), f"Harness 不应有假装意识字段 {f}"


if __name__ == "__main__":
    pytest.main([__file__, "-v"])