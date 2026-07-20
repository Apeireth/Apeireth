"""Relation Graph v0.1 Demo — Phase 3 PoC

干什么:
  1. 加载 master IdentityCard
  2. 加载 memory.db (SQLite FTS5)
  3. 构造 RelationGraph:
     - ai_self (中心节点)
     - master → ai_self (causal)
     - values (from archetypes + remember_forever) → supports ← ai_self
     - tasks → assigned ← ai_self
     - episodes → linked (causal) ← ai_self
     - notes → derived_from episode + supports value
     - tools (AnySearch) → part_of ai_self
  4. 跑查询: neighbors / traverse / find_path
  5. 存盘 + 报告

依据: TOP-DESIGN-V1 §4.3 + AriGraph (2407.04363)
"""

from __future__ import annotations
import sys
import time
from pathlib import Path

if sys.platform == "win32":
    for s in (sys.stdout, sys.stderr):
        try:
            s.reconfigure(encoding="utf-8")
        except Exception:
            pass

from .identity import load_card
from .relation import (
    RelationGraph, Node, Edge, save_graph,
    NODE_KINDS, EDGE_KINDS,
)


def _load_master_card() -> dict:
    """加载 master IdentityCard — 返回 dict (含 hash)"""
    card_path = Path(__file__).parent / "identity_card.master.json"
    card = load_card(card_path)
    return {
        "name": card.name,
        "archetypes": card.archetypes,
        "remember_forever": card.remember_forever,
        "never_mention": card.never_mention,
        "hash": card.integrity_hash(),
    }


def _load_memory_episodes(limit: int = 50) -> list[dict]:
    """从 memory.db 读最近 N 个 episodes — demo 过滤掉 benchmark test 数据。

    若 DB 里全是 benchmark 假数据 (空), 则返回少量 demo 假 episode 让图丰富。
    """
    try:
        from .memory_store import SqliteMemoryStore
        store = SqliteMemoryStore(Path(__file__).parent.parent / "data" / "memory.db")
        eps = store.episodes(limit=limit)
        real = [{"eid": e.eid, "actor": e.actor, "content": e.content, "context": e.context}
                for e in eps if "benchmark" not in e.content]
        if not real:
            return [
                {"eid": "demo_e1", "actor": "master",   "content": "Apeireth 命名日 2026-07-20 13:32 — 火没灭", "context": "naming-ceremony"},
                {"eid": "demo_e2", "actor": "apeireth", "content": "Identity Store v0.1 跑通, 8 Kickoff 问题 → JSON 身份卡", "context": "phase1-ship"},
                {"eid": "demo_e3", "actor": "master",   "content": "中央 AI 必须有 Memory — 没记忆就不是同一只", "context": "philosophy"},
            ]
        return real
    except Exception as exc:
        print(f"[warn] memory.db load skipped: {exc}")
        return []


def _load_memory_notes(limit: int = 3) -> list[dict]:
    try:
        from .memory_store import SqliteMemoryStore
        store = SqliteMemoryStore(Path(__file__).parent.parent / "data" / "memory.db")
        notes = store.notes(limit=limit)
        return [{"nid": n.nid, "topic": n.topic, "claim": n.claim, "importance": n.importance} for n in notes]
    except Exception:
        return []


def build_graph(card: dict, episodes: list[dict], notes: list[dict]) -> RelationGraph:
    """构造示例图 — 围绕 ai_self 中心"""
    g = RelationGraph()

    # 1) 中心节点
    ai = g.add_node("ai_self", label=card["name"] or "apeireth-central", weight=10.0, meta={"creator_hash": card["hash"]})

    # 2) master
    master = g.add_node("master", label="主人", weight=10.0)
    g.add_edge(master.nid, ai.nid, "causal", evidence="主人触发创世 (12:54)")

    # 3) values — 来自 archetypes + remember_forever
    seen_values: set[str] = set()
    for label in card["archetypes"] + card["remember_forever"]:
        if not label or label in seen_values:
            continue
        seen_values.add(label)
        v = g.add_node("value", label=label[:80], weight=7.0)
        g.add_edge(v.nid, ai.nid, "supports", evidence="IdentityCard value")

    # 4) tasks — 演示用的硬编码 2 个
    t1 = g.add_node("task", label="Memory Layer v0.2 SQLite+FTS5", ref="phase2.5", weight=6.0)
    t2 = g.add_node("task", label="Apeireth 命名 + 顶层设计 v1", ref="phase0", weight=8.0)
    g.add_edge(ai.nid, t1.nid, "assigned", evidence="Phase 2.5 已 commit d597171")
    g.add_edge(ai.nid, t2.nid, "assigned", evidence="Phase 0 已 commit f3736ee")

    # 5) episodes (从 memory.db) — 过滤掉 benchmark test 假数据
    real_eps = [ep for ep in episodes if "benchmark" not in ep["content"]]
    for ep in real_eps[:5]:  # 限 5 个, 避免 graph 太脏
        e_node = g.add_node("episode", label=ep["content"][:60], ref=ep["eid"], weight=3.0, meta={"actor": ep["actor"]})
        g.add_edge(e_node.nid, ai.nid, "causal", evidence=ep["context"])

    # 6) notes (从 memory.db) — linked to a random episode for derived_from
    for i, n in enumerate(notes):
        n_node = g.add_node("note", label=n["topic"][:60], ref=n["nid"], weight=float(n.get("importance", 5)))
        # 找一个 episode 当 source
        if episodes:
            ep_ref = episodes[i % len(episodes)]["eid"]
            ep_node_id = f"episode_{ep_ref[:6]}"
            # 实际上我们的 nid 用了 prefix; 让 derived_from 边连到 ai_self 的最近 episode
            for nb, _ in g.neighbors(ai.nid):
                if nb.kind == "episode":
                    g.add_edge(n_node.nid, nb.nid, "derived_from", evidence="Note 抽象自 Episode")
                    break
        # 让 note 支持 ai_self
        g.add_edge(n_node.nid, ai.nid, "supports", weight=0.5, evidence="memory knowledge")

    # 7) tools — AnySearch 已接入
    tool1 = g.add_node("tool", label="AnySearch (L2 Interaction)", ref="apeireth.skills.anysearch", weight=4.0)
    g.add_edge(tool1.nid, ai.nid, "part_of", evidence="L2 集成 commit 413d7a5")

    # 8) ai_self 出边 — 服务主人、体现价值观、分派任务
    g.add_edge(ai.nid, master.nid, "causal", evidence="中央 AI 服务于主人 (12:14)")
    # (task 的 assigned 边已加)

    return g


def main() -> None:
    print("=" * 60)
    print("🕸️  Apeireth — Relation Graph v0.1 PoC")
    print("=" * 60)

    card = _load_master_card()
    print(f"📇 master card: {card['name']} (hash {card['hash']})")
    print(f"   archetypes:  {len(card['archetypes'])}")
    print(f"   remember:    {len(card['remember_forever'])}")
    print(f"   never:       {len(card['never_mention'])}")

    episodes = _load_memory_episodes(limit=50)
    notes = _load_memory_notes(limit=5)
    print(f"📦 loaded: {len(episodes)} episodes, {len(notes)} notes (from memory.db)")

    g = build_graph(card, episodes, notes)

    print()
    print("─── Stats ───")
    stats = g.stats()
    for k, v in stats.items():
        print(f"  {k:14s}: {v}")

    print()
    print("─── Neighbors of ai_self (kind=causal, outgoing) ───")
    ai = g.central()
    if ai:
        for nb, edge in g.neighbors(ai.nid, edge_kind="causal")[:6]:
            print(f"  -> {nb.kind:10s} | {nb.label[:50]:50s} | w={edge.weight}")

    print()
    print("─── Neighbors of ai_self (any, outgoing) ───")
    if ai:
        for nb, edge in g.neighbors(ai.nid)[:10]:
            print(f"  -> {nb.kind:10s} [{edge.kind:12s}] | {nb.label[:40]:40s} | w={edge.weight}")

    print()
    print("─── Traverse from master (depth=2) — paths ───")
    master_node = next((n for n in g.nodes.values() if n.kind == "master"), None)
    if master_node:
        paths = g.traverse(master_node.nid, depth=2)
        print(f"  total paths: {len(paths)}")
        for p in paths[:8]:
            kinds = [g.nodes[nid].kind for nid in p if nid in g.nodes]
            print(f"    {' -> '.join(kinds)}")

    print()
    print("─── find_path: master -> first note ───")
    if master_node:
        note_node = next((n for n in g.nodes.values() if n.kind == "note"), None)
        if note_node:
            path = g.find_path(master_node.nid, note_node.nid)
            if path:
                seq = [(g.nodes[nid].kind, g.nodes[nid].label[:30]) for nid in path if nid in g.nodes]
                for k, lbl in seq:
                    print(f"    {k:10s} | {lbl}")
            else:
                print(f"  ❌ no path")

    # 存盘
    out = Path(__file__).parent / "relation_graph.demo.json"
    save_graph(g, out)
    print()
    print(f"💾 saved: {out}")
    print(f"📋 version: {g.version}")


if __name__ == "__main__":
    main()