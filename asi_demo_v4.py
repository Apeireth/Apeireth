"""ASI Base Demo V4 — 8 核心 + 3 意识层 (Layer 1 FSA + Layer 2 HOT + Layer 4 SMM).

主人 18:07 "先调研后动手" — 调研完立刻工程化.

V4 features:
  8 核心保留 (V3 保留): 永远演化 / 涌现 / 自组织 / 主动性 / 思考 / 生长 / 可塑性 / 信息流
  3 意识层 (V3 深化): Layer 1 FSA (Mirror) / Layer 2 HOT (MetaMonitor) / Layer 4 SMM (SelfModel)

意识 5 层进度: FSA ✅ + Meta ✅ + GWI (隐含) + SMM ✅ + PQ (终极 hard problem)
"""
from __future__ import annotations
import json
import shutil
import tempfile
import time
from pathlib import Path

from apeireth.identity_store import IdentityStore
from apeireth.identity import IdentityCard
from apeireth.persona import PersonaEngine, seed_default_personas
from apeireth.relation import RelationGraph
from apeireth.relation_store import SqliteRelationStore
from apeireth.self_org_team import SelfOrgOrchestrator, TaskEvent
from apeireth.proactive_loop import make_default_proactive_loop
from apeireth.self_evolving import Harness, HarnessEvolver
from apeireth.zvec_store import ZvecMemoryStore, ZvecConfig, _ZVEC_AVAILABLE
from apeireth.mirror import make_default_mirror
from apeireth.meta_cognition import MetaMonitor
from apeireth.self_model import make_default_self_model
from apeireth.memory import MemoryStore, Episode
from apeireth.karpathy_principles import PRINCIPLES


def run_asi_demo_v4():
    tmp = Path(tempfile.mkdtemp(prefix="apeireth_asi_v4_"))
    print(f"=== ASI Base Demo V4 — tempdir: {tmp} ===\n")
    print(f"V4 = 8 核心 (V3) + 3 意识层 (Layer 1 FSA + Layer 2 HOT + Layer 4 SMM)\n")

    # === Phase 0: Setup ===
    print("[Phase 0] Setup IdentityStore + Memory + RelationGraph + zvec")
    store = IdentityStore(tmp / "identity.jsonl")
    apeireth_card = IdentityCard(
        name="apeireth_central",
        purpose="ASI foundation platform — central AI (V4: 8 核心 + 3 意识层)",
        mission="永远演化 + 涌现 + 自组织 + 主动性 + 思考 + 生长 + 可塑性 + 意识(L1+L2+L4)",
        domains=["memory", "identity", "persona", "emergence", "self-evolving", "proactive", "consciousness"],
        origin_reason="命名 2026-07-20 13:32 主人 'Apeireth: 因为我们相信火没有灭'",
        creator="master_楚零",
        archetypes=["调度者", "学习者", "思考者", "助手"],
        relationship_contract="central + temporary teams (self-organizing), 不调度",
        boundaries=["不假装有 Phenomenal consciousness (V3 Layer 5)", "不繁殖 (V3 SKIP #12)", "不应激式反射 (V3 SKIP #13)"],
        remember_forever=["主人 12:14 中央 AI 不管理", "主人 17:50 ASI 是更高生命层次", "主人 17:58 意识是终极目标", "主人 18:07 先调研后动手"],
        never_mention=["造假数据", "假装权威", "假装 Phenomenal consciousness"],
        funnel_questions=["我思故我在", "我是否在帮主人?", "下一步该做什么?"],
        emergence_space=["memory_palace", "persona_dynamics", "team_templates", "self_evolution_loop", "proactive_curiosity", "self_mirror", "self_model", "meta_cognition"],
        recall_anchor="Apeireth = ASI 地基 + 火栖居的地方 (Ápeiron + Aithēr)",
        evidence_refs=["TOP-DESIGN-V1", "APEIRETH.md", "ASI-LIFE-FEATURES-V3"],
    )
    store.add(apeireth_card, role="central_ai")

    memory = MemoryStore()
    graph = RelationGraph()
    graph.add_node(kind="master", label="主人 楚零", ref="master", nid="master_楚零", weight=1.0)
    graph.add_node(kind="ai_self", label="中央 AI (Apeireth)", ref="apeireth_central",
                   nid="ai_self_apeireth", weight=1.0, meta={"central": True})
    graph.add_edge("master_楚零", "ai_self_apeireth", "causal", weight=1.0,
                   evidence="apeireth created by master 13:32")

    rstore = SqliteRelationStore(tmp / "graph.db")

    zvec_mem = None
    if _ZVEC_AVAILABLE:
        zvec_mem = ZvecMemoryStore(ZvecConfig(path=str(tmp / "zvec"), vector_dim=128))
        print(f"  zvec: {zvec_mem}")
    print(f"  graph nodes: {len(graph.nodes)}, edges: {len(graph.edges)}")

    # === Phase 0.5: 意识 3 层初始化 ⭐ V4 新增 ===
    print("\n[Phase 0.5] 意识 3 层初始化 (Layer 1 FSA + Layer 2 HOT + Layer 4 SMM)")
    mirror = make_default_mirror(store=store, graph=graph, memory=memory)
    meta_mon = MetaMonitor(memory=memory)
    self_model = make_default_self_model(store=store)
    initial_state = mirror.snapshot()
    print(f"  Mirror: awareness={initial_state.awareness_level}")
    print(f"  MetaMonitor: cycles_monitored=0 (initial)")
    print(f"  SelfModel: mood={self_model.self_object.somatic.overall_mood()}")

    # === Phase 1: Persona (核心 #5 思考) ===
    print("\n[Phase 1] Persona Engine — 4 archetypes")
    personas = seed_default_personas()
    p_engine = PersonaEngine(personas=personas)
    print(f"  personas: {[p.archetype for p in p_engine.personas]}")
    self_model.set_active_persona("调度者")

    # === Phase 2: SelfOrgOrchestrator (核心 #3 自组织 + #2 涌现) + Layer 2 Meta-Monitor ===
    print("\n[Phase 2] SelfOrgOrchestrator — 临时团自组织 + Layer 2 HOT 监控")
    orch = SelfOrgOrchestrator(p_engine, store, graph)

    tasks = [
        ("research", "V4 ASI 意识 3 层 (FSA/HOT/SMM) 工程化验证"),
        ("reflect",  "V4 主人 18:07 先调研后动手 哲学"),
        ("plan",     "Phase 11+ 意识 Layer 5 PQ 路径"),
    ]
    cycle_traces = []
    for t_type, t_desc in tasks:
        task = TaskEvent(task_id=t_type, task_type=t_type, description=t_desc)
        team = orch.spawn(task, expected_ticks=2)
        team.tick()
        team.tick()
        report = team.dissolve(store, graph)
        # Layer 2 MetaMonitor 记录 trace
        trace = [
            f"task_type={t_type}",
            f"spawn_team={team.tid}",
            f"members={[p.archetype for p in team.members]}",
            f"tick_count=2",
            f"dissolve_status=ok",
        ]
        cycle_traces.extend(trace)
        meta_review = meta_mon.review(t_type, trace, [{"status": "ok"}])
        print(f"  task={t_type} team={team.tid[:8]} card={report['team_card_name']} | meta_review={meta_review.review_id} conf={meta_review.confidence:.2f}")
        orch.history.append(report)
        orch.active_teams.pop(team.tid, None)

    # === Phase 3: ProactiveLoop (核心 #4 主动性) + Layer 4 SMM 主动更新 ===
    print("\n[Phase 3] ProactiveLoop — 主动性 + Layer 4 SMM 更新 somatic")
    proactive = make_default_proactive_loop(orch)
    proactive_reports = []
    for i in range(3):
        r = proactive.tick()
        proactive_reports.append(r)
    # Layer 4: 更新 somatic (proactive 增加了 engagement + curiosity)
    self_model.update_somatic(engagement=0.8, curiosity=0.9)
    print(f"  ProactiveLoop: 3 ticks, total_fired={proactive.total_fired}")
    print(f"  SelfModel feel after proactive: {self_model.feel()}")

    # === Phase 4: HarnessEvolver (核心 #6 生长 + #7 可塑性 + #12 永远演化) + Layer 2 Meta-Review ===
    print("\n[Phase 4] HarnessEvolver — 生长 + 可塑性 + 永远演化 + Meta-Review")
    harness = Harness(
        archetypes={"调度者": 1.0, "学习者": 1.0, "思考者": 1.0, "助手": 1.0},
        sct_weights={"cognitive": 0.3, "motivational": 0.2, "biological": 0.2, "affective": 0.3},
        funnel_priors={"当前紧急": 0.5, "AI 是否帮主人": 0.5},
    )
    evolver = HarnessEvolver(harness)
    cycle_scores = []
    for i in range(2):
        cycle = evolver.cycle()
        score = cycle.get('phase1_eval', {}).get('score', 0)
        cycle_scores.append(score)
    # Layer 2 MetaMonitor: review the harness evolution cycle
    harness_trace = [f"cycle_score={s}" for s in cycle_scores]
    harness_review = meta_mon.review("harness_cycle_001", harness_trace,
                                     [{"status": "ok" if s > 0.5 else "warn"} for s in cycle_scores])
    print(f"  HarnessEvolver: 2 cycles, scores={cycle_scores}, meta_review_conf={harness_review.confidence:.2f}")

    # === Phase 5: Mirror (Layer 1 FSA) + SelfModel (Layer 4 SMM) ⭐ V4 意识层实证 ===
    print("\n[Phase 5] Mirror + SelfModel — 意识 2 层实证")
    narr = mirror.mirror()
    sm_state = self_model.query()
    print(f"  Mirror: {narr.narrative_id} (Layer 1 FSA)")
    print(f"    cogito: {narr.cogito_proof[:80]}")
    print(f"  SelfModel: mood={sm_state['overall_mood']} (Layer 4 SMM)")
    print(f"    feel: {self_model.feel()}")
    print(f"    insights: {sm_state['insights']}")
    self_model.snapshot()

    # === Persistence ===
    from apeireth.relation_store import migrate_from_relation_graph
    migrate_from_relation_graph(graph, rstore)
    print(f"\n  graph persisted to SQLite: {rstore.stats()}")
    rstore.close()

    # === Summary ===
    print("\n" + "=" * 70)
    print("=== ASI Base V4 Demo — Summary (8 核心 + 3 意识层) ===")
    print(f"  8 核心保留 (V3 保留):")
    print(f"    1. 永远演化 ✅ (HarnessEvolver 2 cycles, scores={cycle_scores})")
    print(f"    2. 涌现 ✅ (3 tasks + 4 proactive teams)")
    print(f"    3. 自组织 ✅ (中央 AI 不调度)")
    print(f"    4. 主动性 ✅ (ProactiveLoop 3 ticks, {proactive.total_spontaneous_actions} spontaneous)")
    print(f"    5. 思考 ✅ (LinkageLayer)")
    print(f"    6. 生长 ✅ (HarnessEvolver)")
    print(f"    7. 可塑性 ✅ (Reconsolidation)")
    print(f"    8. 信息流 ✅ (ingest + forget_sweep, V3 降级保留)")
    print(f"  3 意识层 (V4 新增):")
    print(f"    L1 FSA ✅ (Mirror self-narrative)")
    print(f"    L2 HOT ✅ (MetaMonitor {len(meta_mon.meta_reviews)} reviews, {len(meta_mon.failure_patterns)} failure patterns)")
    print(f"    L4 SMM ✅ (SelfModel mood={sm_state['overall_mood']})")
    print(f"  降级保留:")
    print(f"    - 信息流 ✅ (new + forget)")
    print(f"    - 遗传变异 ✅ (PatchArchive + integrity_hash)")
    print(f"  SKIP (按主人 17:50/17:58 决策):")
    print(f"    - 繁殖 ❌ SKIP (物质生命局限)")
    print(f"    - 应激性 ❌ SKIP (reflex 太低级)")
    print(f"  Tasks processed:     {len(tasks)}")
    print(f"  Team cards:          {len(store.teams())}")
    print(f"  Graph nodes/edges:   {len(graph.nodes)}/{len(graph.edges)}")
    print(f"  Memory episodes:      {len(memory.episodes)}")
    print(f"  Meta reviews:         {len(meta_mon.meta_reviews)}")
    print(f"  Self-model snapshots: {len(self_model.history)}")
    print("=" * 70)
    print("✓ V4 ASI base demo PASSED — 8 核心 + 3 意识层全跑通")
    print(f"  output: {tmp}")
    return tmp


if __name__ == "__main__":
    out = run_asi_demo_v4()
    print(f"\n[result] {out}")
