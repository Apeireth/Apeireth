"""Linker v0.1 Demo — Memory ↔ Graph 跨层自动绑定 (Phase 3.6)

依据: TOP-DESIGN-V1 §4.3 + DEV-LOG 14:50 next step #1+#2

跑法:
1. 用 SqliteMemoryStore + SqliteRelationStore — 真实持久化
2. sync_all() 全量同步 —  幂等, 重复跑数据不污染
3. 对比 sync 前后 graph stats 差异
4. 演示跨层引用 ref 查找 (eid → episode node)
5. 演示增量 Linker (link_one_episode)
"""

from __future__ import annotations
import sys
import uuid
import time
from pathlib import Path

if sys.platform == "win32":
    for s in (sys.stdout, sys.stderr):
        try:
            s.reconfigure(encoding="utf-8")
        except Exception:
            pass

from .memory import Episode, Note
from .memory_store import SqliteMemoryStore
from .relation_store import SqliteRelationStore
from .linker import (
    sync_all, Linker,
    ensure_central_ai_node,
    LINKER_VERSION,
)


def _resolve_db_paths() -> tuple[Path, Path]:
    base = Path(__file__).parent.parent / "data"
    base.mkdir(parents=True, exist_ok=True)
    return base / "memory.db", base / "graph_linker_demo.db"


def main() -> None:
    print("=" * 64)
    print("🔗 Apeireth — Linker v0.1 (Memory ↔ Graph 跨层绑定)")
    print("=" * 64)
    print(f"📋 linker version: {LINKER_VERSION}\n")

    memory_db, graph_db = _resolve_db_paths()
    mstore = SqliteMemoryStore(memory_db)

    # 单独用一张图 db 给 demo, 不污染真 graph.db
    if graph_db.exists():
        graph_db.unlink()
    gstore = SqliteRelationStore(graph_db)
    ai_nid = ensure_central_ai_node(gstore, "楚零 (中央 AI)")
    print(f"🧬 ai_self nid: {ai_nid}\n")

    # ---- 0) 种子数据 (memory 之前 v0.1 已写就了 3 ep + 2 notes, 这里追加一组让 demo 显著) ----
    print("─── Step 0: 追加种子数据 (3 ep + 2 notes) ───")
    seed_episodes = [
        Episode(eid=uuid.uuid4().hex[:8], actor="master",
                content="LLM 没历史就从主人学 — 母兽教小兽", context="philosophy"),
        Episode(eid=uuid.uuid4().hex[:8], actor="apeireth",
                content="Identity Store v0.1 PoC 跑通 — 263 行", context="phase1-ship"),
        Episode(eid=uuid.uuid4().hex[:8], actor="master",
                content="中央 AI 不管理, 一切交给中央 AI 自己 — 涌现", context="philosophy"),
    ]
    for ep in seed_episodes:
        mstore.append_episode(ep)
        print(f"  ✓ episode [{ep.actor}] {ep.content[:40]}...")

    seed_notes = []
    eid_first = mstore.episodes(limit=1)[0].eid
    seed_notes.append(Note(
        nid=uuid.uuid4().hex[:8], topic="永远记得: 火没灭",
        claim="Apeireth 火没灭 — 命名日 2026-07-20",
        evidence=[seed_episodes[0].eid], confidence=0.5, importance=8,
    ))
    seed_notes.append(Note(
        nid=uuid.uuid4().hex[:8], topic="母兽教小兽",
        claim="中央 AI 像母兽教小兽 — 从主人学",
        evidence=[seed_episodes[0].eid], confidence=0.6, importance=7,
    ))
    for n in seed_notes:
        mstore.add_note(n)
        print(f"  ✓ note [{n.topic}]")

    # ---- 1) 全量 sync ----
    print("\n─── Step 1: sync_all() 全量同步 ───")
    stats_before = gstore.stats()
    print(f"   graph (before): {stats_before['nodes']} nodes, {stats_before['edges']} edges")
    sync_result = sync_all(mstore, gstore)
    stats_after = gstore.stats()
    for k, v in sync_result.items():
        if k == "ts": continue
        print(f"   sync.{k:30s}: {v}")
    print(f"   graph (after):  {stats_after['nodes']} nodes, {stats_after['edges']} edges")

    # ---- 2) 幂等: 再跑一次, 数据不增长 ----
    print("\n─── Step 2: idempotency check — 再 sync_all() ───")
    sync_2 = sync_all(mstore, gstore)
    stats_3rd = gstore.stats()
    print(f"   sync 2nd: ep_added={sync_2['episodes_linked']} note_added={sync_2['notes_linked']}")
    print(f"   graph now: {stats_3rd['nodes']} nodes, {stats_3rd['edges']} edges")
    is_idempotent = (sync_2['episodes_linked'] == 0 and sync_2['notes_linked'] == 0)
    print(f"   {'✅ idempotent' if is_idempotent else '❌ NOT idempotent'}")

    # ---- 3) 跨层引用查找 ----
    print("\n─── Step 3: 跨层引用 — nodes_by_ref(episode.eid) ───")
    # 取 memory 里第一个 episode 的 eid
    first_ep = mstore.episodes(limit=1)[0]
    refs = gstore.nodes_by_ref(first_ep.eid)
    print(f"   episode.eid={first_ep.eid}")
    print(f"   found {len(refs)} graph nodes pointing to this episode")
    for n in refs[:5]:
        print(f"     [{n.kind}] {n.nid:18s} | w={n.weight:.1f} | {n.label[:40]}")

    # ---- 4) 演示 derived_from 边 ----
    print("\n─── Step 4: derived_from edges (Note ← Episode) ───")
    derived = gstore.edges_by_kind("derived_from")
    for e in derived[:5]:
        print(f"   {e.src[:18]:18s} → {e.dst[:18]:18s} | w={e.weight:.1f}")

    # ---- 5) 增量 Linker 演示 ----
    print("\n─── Step 5: 增量 Linker (session 内增量加 node) ───")
    linker = Linker(mstore, gstore)
    new_ep = Episode(eid=uuid.uuid4().hex[:8], actor="master",
                     content="这是 linker demo 临时加的 episode", context="demo")
    mstore.append_episode(new_ep)
    node_new, edge_new = linker.link_one_episode(new_ep)
    print(f"   link_one_episode: node_added={node_new}, edge_added={edge_new}")
    print(f"   session_added: nodes={linker.session_added_nodes}, edges={linker.session_added_edges}")

    final_stats = linker.stats()
    print(f"\n─── Final Stats ───")
    for k, v in final_stats.items():
        print(f"   {k:20s}: {v}")

    mstore.close()
    gstore.close()
    print(f"\n✅ done — demo graph at: {graph_db}")


if __name__ == "__main__":
    main()
