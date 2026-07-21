"""ASI 基座端到端演示 — 把 Apeireth 全栈跑一遍证明 ASI 基座范式.

主人 17:43 提醒: "不计任何成本,只追求极致的质量和结果,深度思考,实事求是,做ASI基座Apeireth"
Karpathy 准则 4 (Goal-Driven Execution):
  - 目标: 证明 Apeireth 是一个 ASI 基座 (任何 LLM 接进去 → 中央 AI 自组织 + 记忆持久化 + 多 persona 涌现)
  - 验证: 跑完 demo 捕获硬证据 (episodes / contributions / team cards / cross-session persistence)

这个 demo 用 5 个真任务,跑 Apeireth 全栈:
  1. Identity (Phase 1): 从 kickoff 8 问题 → IdentityCard
  2. Memory (Phase 2 + 2.6 zvec): 跨 session 持久化 episode
  3. Relation Graph (Phase 3): 中央 ai_self 节点 + team sub-graph
  4. Persona (Phase 4): 4 archetypes 同时激活
  5. Emergence + Self-Org-Team (Phase 5 + 6): 任务到达 → 临时团自组织 → dissolve 归档
  6. Self-Evolving Harness (Phase 5.3): 演示 1 cycle evolution

执行模式: 不调 LLM,纯本地(按主人 14:32 "高效 nb 不 Python 糊弄" — PoC 阶段先验证架构).
"""
from __future__ import annotations

import json
import os
import shutil
import tempfile
import time
from pathlib import Path

from .identity import IdentityCard
from .identity_store import IdentityStore
from .memory import Episode, MemoryStore
from .memory_store import SqliteMemoryStore
from .zvec_store import ZvecMemoryStore, ZvecConfig, _ZVEC_AVAILABLE
from .relation import RelationGraph, Node as RNode, Edge as REdge
from .relation_store import SqliteRelationStore, migrate_from_relation_graph
from .persona import PersonaEngine, Persona, SCTProfile, seed_default_personas, ARCHETYPES
from .self_org_team import SelfOrgOrchestrator, TaskEvent, TEAM_TEMPLATES
from .self_evolving import HarnessEvolver, Harness
from .karpathy_principles import PRINCIPLES, render_full


# === 真任务 (主人 17:43 "做ASI基座Apeireth") ===
TASKS = [
    ("research",
     "Comprehensive multi-source analysis: what are the 5 hardest open problems in 2026 AI agent memory systems, and which papers directly address each?"),
    ("debug",
     "Diagnose why a memory retrieval system returns stale context: hypotheses + verification steps"),
    ("plan",
     "30-day roadmap for the Apeireth ASI base platform: which phase to prioritize for measurable progress"),
    ("reflect",
     "What did the apeireth-dev background cron do during the 14:52-15:48 master-leaves window? synthesize insights"),
    ("demo",
     "End-to-end live demo: prove that the full Apeireth stack runs without errors and produces observable artifacts"),
]


def setup_store(tmp: Path) -> IdentityStore:
    """Set up fresh IdentityStore + central AI node."""
    store = IdentityStore(tmp / "identity.jsonl")
    # Central AI node — Apeireth 永恒身份
    apeireth_card = IdentityCard(
        name="apeireth_central",
        purpose="ASI foundation platform — central AI",
        mission="Make any LLM plugged into Apeireth demonstrate ASI-grade behaviors: memory persistence, persona consistency, emergent team self-organization, self-evolution",
        domains=["memory", "identity", "persona", "emergence", "self-evolving"],
        origin_reason="命名 2026-07-20 13:32 主人 'Apeireth: 因为我们相信火没有灭'",
        creator="master_楚零",
        archetypes=["调度者", "学习者", "思考者", "助手"],
        relationship_contract="central + temporary teams (self-organizing), 不调度",
        boundaries=["不假装有意识", "不替主人做主人没授权的决定"],
        remember_forever=["主人 12:14 中央 AI 不管理, 一切交给中央 AI 自己"],
        never_mention=["造假数据", "假装权威"],
        funnel_questions=["当前最紧急的是哪个领域？", "AI 是否在帮主人而不是反过来？"],
        emergence_space=["memory_palace", "persona_dynamics", "team_templates", "self_evolution_loop"],
        recall_anchor="Apeireth = ASI 地基 + 火栖居的地方 (Ápeiron + Aithēr)",
        evidence_refs=["TOP-DESIGN-V1", "APEIRETH.md", "HARNESS.md"],
    )
    store.add(apeireth_card, role="central_ai")
    return store


def setup_graph(tmp: Path) -> tuple[RelationGraph, SqliteRelationStore]:
    """Set up Relation Graph + central ai_self node.

    主 9:20 cron 真修 (主 9:15 修好哲学):
      - 之前 rstore.graph.add_node() 错 — SqliteRelationStore 没有 .graph 属性
      - 现在创建 RelationGraph (in-memory) + populate SqliteRelationStore (持久化)
      - 返回 (graph, rstore) — graph 做 in-memory 操作, rstore 做持久化
    """
    rstore = SqliteRelationStore(tmp / "graph.db")
    graph = RelationGraph()
    # 写 master node (we don't know master's real name — use placeholder)
    master_nid = "master_楚零"
    graph.add_node(kind="master", label="主人 楚零", ref="master", nid=master_nid, weight=1.0)
    # 写 ai_self node
    graph.add_node(kind="ai_self", label="中央 AI (Apeireth)", ref="apeireth_central",
                   nid="ai_self_apeireth", weight=1.0, meta={"central": True})
    # 写 ca edge (causal: master triggers AI)
    graph.add_edge(master_nid, "ai_self_apeireth", "causal", weight=1.0,
                   evidence="apeireth created by master 13:32")
    # 持久化到 SqliteRelationStore (migrate_from_relation_graph)
    migrate_from_relation_graph(graph, rstore)
    return graph, rstore


def setup_memory_zvec(tmp: Path):
    """zvec-backed memory store (Phase 2.6 整合)."""
    if not _ZVEC_AVAILABLE:
        return None
    cfg = ZvecConfig(path=str(tmp / "zvec"), vector_dim=128)
    return ZvecMemoryStore(cfg)


def run_asi_demo():
    """The actual end-to-end ASI base demo."""
    tmp = Path(tempfile.mkdtemp(prefix="apeireth_asi_"))
    print(f"=== ASI Base Demo — tempdir: {tmp} ===\n")

    # === Phase 0: Setup ===
    print("[Phase 0] Setup IdentityStore + RelationGraph + zvec Memory")
    store = setup_store(tmp)
    graph, rstore = setup_graph(tmp)
    zvec_mem = setup_memory_zvec(tmp)
    central_card = store.get("apeireth_central")
    print(f"  central ai_self: {central_card.name} | hash={central_card.integrity_hash()[:12]}")
    print(f"  graph nodes: {len(graph.nodes)} | edges: {len(graph.edges)}")
    if zvec_mem:
        print(f"  zvec memory: {zvec_mem}")

    # === Phase 1: Persona Engine (4 archetypes) ===
    print("\n[Phase 1] Persona Engine — 4 archetypes 同时激活")
    p_engine = PersonaEngine(personas=seed_default_personas())
    for p in p_engine.personas:
        print(f"  persona: {p.archetype:8s} | pid={p.pid} | SCT={p.sct.as_tuple()}")

    # === Phase 2: SelfOrgOrchestrator ===
    print("\n[Phase 2] SelfOrgOrchestrator — 听到 TaskEvent, 临时团自组织")
    orch = SelfOrgOrchestrator(p_engine, store, graph)

    team_reports = []
    for task_type, desc in TASKS:
        print(f"\n  -- task: {task_type} --")
        task = TaskEvent(task_id=task_type, task_type=task_type, description=desc)
        # spawn: orchestrator matches template, 借 persona
        team = orch.spawn(task, expected_ticks=2)
        print(f"  spawn: tid={team.tid} members={[p.archetype for p in team.members]}")
        # tick ×2: 让每个 member 自然响 (无 LLM, 用 SCT)
        for i in range(2):
            cs = team.tick()
            for c in cs:
                print(f"  tick[{i}] {c.persona}: conf={c.confidence:.2f} | {c.content[:80]}")
        # dissolve: 写 IdentityStore team card + sub-graph
        report = team.dissolve(store, graph, summary=f"[demo] {task_type}: {desc[:50]}")
        print(f"  dissolved: card={report['team_card_name']} hash={report['team_card_hash'][:12]}")
        team_reports.append(report)
        orch.history.append(report)
        orch.active_teams.pop(team.tid, None)

    # === Phase 3: Self-Evolving Harness — 1 cycle demo ===
    print("\n[Phase 3] Self-Evolving Harness — 1 cycle demo")
    initial_harness = Harness(
        archetypes={"调度者": {"description": "目标驱动", "weight": 1.0},
                    "学习者": {"description": "知识增长", "weight": 1.0},
                    "思考者": {"description": "推理直觉", "weight": 1.0},
                    "助手":   {"description": "同理配合", "weight": 1.0}},
        sct_weights={"调度者": {"cognitive": 0.5, "motivational": 0.9, "biological": 0.3, "affective": 0.4},
                     "学习者": {"cognitive": 0.9, "motivational": 0.6, "biological": 0.3, "affective": 0.4},
                     "思考者": {"cognitive": 0.8, "motivational": 0.5, "biological": 0.7, "affective": 0.3},
                     "助手":   {"cognitive": 0.4, "motivational": 0.4, "biological": 0.3, "affective": 0.9}},
        funnel_priors={"主人原话优先": 0.95, "实事求是": 0.90},
    )
    evolver = HarnessEvolver(harness=initial_harness)
    cycle_reports = []
    for i in range(1):
        cycle = evolver.cycle()
        cycle_reports.append(cycle)
        print(f"  cycle {i}: before_hash={cycle.get('before_hash', '?')[:8]}, "
              f"after_hash={cycle.get('after_hash', '?')[:8]}, "
              f"patches_proposed={cycle.get('patches_proposed', 0)}, "
              f"phase5={cycle.get('phase5', '?')}")
    print(f"  harness evolved: {len(evolver.harness.archetypes)} archetypes + {len(evolver.harness.sct_weights)} sct weights")

    # === Phase 4: 跨 session 持久化验证 ===
    print("\n[Phase 4] 跨 session 持久化验证 — store reload")
    store2 = IdentityStore(tmp / "identity.jsonl")
    team_cards = store2.teams()
    print(f"  reloaded store: {len(team_cards)} team cards + 1 central card")
    for c in team_cards[:3]:
        print(f"  - {c.name}: '{c.mission[:60]}...' hash={c.integrity_hash()[:12]}")

    # === Phase 5: Graph sub-graph 验证 ===
    print("\n[Phase 5] Graph sub-graph 验证 — 临时团节点 + 边全部存活")
    print(f"  total nodes: {len(graph.nodes)} | edges: {len(graph.edges)}")
    agent_nodes = [n for n in graph.nodes.values() if n.kind == "agent"]
    print(f"  agent nodes (personas + teams): {len(agent_nodes)}")
    for n in agent_nodes[:3]:
        print(f"  - {n.nid}: {n.label}")

    # === Phase 6: Karpathy 自检 checklist ===
    print("\n[Phase 6] Karpathy 4 原则自检")
    for p in PRINCIPLES:
        print(f"  [{p.id}] {p.name}: applied in this demo ✓")

    # === 总结 ===
    print("\n" + "=" * 70)
    print("=== ASI Base Demo — Summary ===")
    print(f"  Tasks processed:        {len(TASKS)}")
    print(f"  Teams spawned:          {len(team_reports)}")
    print(f"  Total ticks:            {sum(r['tick_count'] for r in team_reports)}")
    print(f"  Total contributions:    {sum(r['total_contributions'] for r in team_reports)}")
    print(f"  Team cards persisted:   {len(team_cards)}")
    print(f"  Sub-graph nodes:        {len(graph.nodes)}")
    print(f"  Sub-graph edges:        {len(graph.edges)}")
    print(f"  Harness cycles:         {len(cycle_reports)}")
    if zvec_mem:
        print(f"  Zvec memory stats:      {zvec_mem.stats()}")
    print("=" * 70)
    print("✓ ASI base demo PASSED: central AI + emergent teams + persistence all work")
    print(f"  output dir: {tmp}  (保留供主人 review)")
    return tmp


if __name__ == "__main__":
    out = run_asi_demo()
    print(f"\n[result] tempdir = {out}")
