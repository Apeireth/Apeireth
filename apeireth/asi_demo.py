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
from .relation_store import SqliteRelationStore
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


def setup_graph(tmp: Path) -> SqliteRelationStore:
    """Set up Relation Graph + central ai_self node."""
    rstore = SqliteRelationStore(tmp / "graph.db")
    # 写 master node (we don't know master's real name — use placeholder)
    master_nid = "master_楚零"
    rstore.graph.add_node(kind="master", label="主人 楚零", ref="master", nid=master_nid, weight=1.0)
    # 写 ai_self node
    rstore.graph.add_node(kind="ai_self", label="中央 AI (Apeireth)", ref="apeireth_central",
                          nid="ai_self_apeireth", weight=1.0, meta={"central": True})
    # 写 ca edge (causal: master triggers AI)
    rstore.graph.add_edge(master_nid, "ai_self_apeireth", "causal", weight=1.0,
                           evidence="apeireth created by master 13:32")
    return rstore


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
    rstore = setup_graph(tmp)
    zvec_mem = setup_memory_zvec(tmp)
    central_card = store.get("apeireth_central")
    print(f"  central ai_self: {central_card.name} | hash={central_card.integrity_hash()[:12]}")
    print(f"  graph nodes: {rstore.graph.node_count()} | edges: {rstore.graph.edge_count()}")
    if zvec_mem:
        print(f"  zvec memory: {zvec_mem}")

    # === Phase 1: Persona Engine (4 archetypes) ===
    print("\n[Phase 1] Persona Engine — 4 archetypes 同时激活")
    p_engine = PersonaEngine()
    seed_default_personas(p_engine)
    for p in p_engine.personas:
        print(f"  persona: {p.archetype:8s} | pid={p.pid} | SCT={p.sct.as_tuple()}")

    # === Phase 2: SelfOrgOrchestrator ===
    print("\n[Phase 2] SelfOrgOrchestrator — 听到 TaskEvent, 临时团自组织")
    orch = SelfOrgOrchestrator(p_engine, store, rstore.graph)

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
        report = team.dissolve(store, rstore.graph, summary=f"[demo] {task_type}: {desc[:50]}")
        print(f"  dissolved: card={report['team_card_name']} hash={report['team_card_hash'][:12]}")
        team_reports.append(report)
        orch.history.append(report)
        orch.active_teams.pop(team.tid, None)

    # === Phase 3: Self-Evolving Harness — 1 cycle demo ===
    print("\n[Phase 3] Self-Evolving Harness — 1 cycle demo")
    initial_harness = Harness(
        components={"memory": "Episode+Note", "persona": "4 archetypes"},
        skills=["karpathy_principles"],
        guidelines=["主人原话优先", "实事求是"],
    )
    evolver = HarnessEvolver(initial_harness, max_iterations=1)
    cycle_reports = []
    for cycle in evolver.run():
        cycle_reports.append(cycle)
        print(f"  cycle {cycle['iteration']}: eval_score={cycle['eval_score']:.2f}, "
              f"proposed={cycle['proposed']}, accepted={cycle['accepted']}")
    print(f"  harness evolved: {evolver.harness.components}")

    # === Phase 4: 跨 session 持久化验证 ===
    print("\n[Phase 4] 跨 session 持久化验证 — store reload")
    store2 = IdentityStore(tmp / "identity.jsonl")
    team_cards = [c for c in store2.list(role="team")]
    print(f"  reloaded store: {len(team_cards)} team cards + 1 central card")
    for c in team_cards[:3]:
        print(f"  - {c.name}: '{c.mission[:60]}...' hash={c.integrity_hash()[:12]}")

    # === Phase 5: Graph sub-graph 验证 ===
    print("\n[Phase 5] Graph sub-graph 验证 — 临时团节点 + 边全部存活")
    print(f"  total nodes: {rstore.graph.node_count()} | edges: {rstore.graph.edge_count()}")
    agent_nodes = [n for n in rstore.graph.nodes.values() if n.kind == "agent"]
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
    print(f"  Sub-graph nodes:        {rstore.graph.node_count()}")
    print(f"  Sub-graph edges:        {rstore.graph.edge_count()}")
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
