"""Relation Graph v0.2 Demo — SQLite 持久化 + 跨 session 存活

Phase 3 续:
1. 跑 v0.1 build_graph (造图)
2. 迁到 SQLite (data/graph.db)
3. 关闭连接, 重开 → load_graph → 校验 integrity_hash 一致
4. 演示 nodes_by_kind / nodes_by_ref / edges_by_kind 查询
5. 演示 remove_node 级联删除

依据: TOP-DESIGN-V1 §3.3 + §4.3
"""

from __future__ import annotations
import sys
from pathlib import Path

if sys.platform == "win32":
    for s in (sys.stdout, sys.stderr):
        try:
            s.reconfigure(encoding="utf-8")
        except Exception:
            pass

from .run_relation_demo import build_graph, _load_master_card, _load_memory_episodes, _load_memory_notes
from .relation import RelationGraph
from .relation_store import SqliteRelationStore, migrate_from_relation_graph


def main() -> None:
    print("=" * 60)
    print("🗄️  Apeireth — Relation Graph v0.2 (SQLite) Demo")
    print("=" * 60)

    # 1) 构造图 (复用 v0.1 demo)
    card = _load_master_card()
    episodes = _load_memory_episodes(limit=50)
    notes = _load_memory_notes(limit=5)
    g_v1 = build_graph(card, episodes, notes)
    hash_before = g_v1.integrity_hash()
    print(f"📇 master: {card['name']} (hash {card['hash']})")
    print(f"🕸️  built v0.1 graph: {len(g_v1.nodes)} nodes, {len(g_v1.edges)} edges")
    print(f"🔐 integrity hash (before save): {hash_before}")

    # 2) 迁到 SQLite
    db_path = Path(__file__).parent.parent / "data" / "graph.db"
    if db_path.exists():
        db_path.unlink()  # 干净起见
    store = SqliteRelationStore(db_path)
    migrate_stats = migrate_from_relation_graph(g_v1, store)
    print(f"\n💾 migrated → {db_path}")
    for k, v in migrate_stats.items():
        print(f"   {k:20s}: {v}")

    # 3) 关闭 + 重开 + 校验
    store.close()
    store2 = SqliteRelationStore(db_path)
    g_v2 = store2.load_graph()
    hash_after = g_v2.integrity_hash()
    print(f"\n🔄 reloaded from SQLite: {len(g_v2.nodes)} nodes, {len(g_v2.edges)} edges")
    print(f"🔐 integrity hash (after load):  {hash_after}")
    print(f"{'✅' if hash_before == hash_after else '❌'} hash match: {hash_before == hash_after}")

    # 4) 查询演示
    print(f"\n─── nodes_by_kind('value') ───")
    values = store2.nodes_by_kind("value")
    for n in values[:5]:
        print(f"  {n.nid:18s} | w={n.weight:.1f} | {n.label[:50]}")

    print(f"\n─── nodes_by_ref('demo_e1') — 跨层引用查找 ───")
    refs = store2.nodes_by_ref("demo_e1")
    for n in refs:
        print(f"  {n.kind:10s} | {n.nid} | {n.label[:50]}")

    print(f"\n─── edges_by_kind('causal') — 因果边 ───")
    casuals = store2.edges_by_kind("causal")
    for e in casuals[:5]:
        print(f"  {e.src[:14]:14s} → {e.dst[:14]:14s} | w={e.weight:.1f} | {e.evidence[:30]}")

    print(f"\n─── stats ───")
    stats = store2.stats()
    for k, v in stats.items():
        print(f"  {k:18s}: {v}")

    # 5) 级联删除演示
    print(f"\n─── remove_node('mas_63fc0dca' [if exists]) — 级联删边 ───")
    # 我们不知道 master node 的 nid, 先查
    masters = store2.nodes_by_kind("master")
    if masters:
        m = masters[0]
        removed_edges = store2.remove_node(m.nid)
        print(f"  removed: {m.nid} (master) + {removed_edges} cascaded edges")
        print(f"  remaining nodes: {store2.stats()['nodes']}")
        print(f"  remaining edges: {store2.stats()['edges']}")
    else:
        print("  (no master node in graph — skipped)")

    store2.close()
    print(f"\n✅ done — graph.db persisted at {db_path}")


if __name__ == "__main__":
    main()