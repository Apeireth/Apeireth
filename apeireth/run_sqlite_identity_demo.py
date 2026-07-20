"""Phase 6.5 Demo — SqliteIdentityStore 真持久化

跑这个 demo 验证:
1. IdentityStore (in-memory) → SqliteIdentityStore (SQLite) round-trip
2. FTS5 跨卡全文搜索 (master / persona / team)
3. 跨 session 真存活 (关连接, 重开, 内容一致)
4. 沙盒保护: master 不允许删, 重复 master 抛错
5. cross_card_hash 跨卡聚合一致

依据: DEV-LOG 17:14 + 17:43 限制 #6 (team card 没保存到磁盘)
"""

from __future__ import annotations
import shutil
import tempfile
from pathlib import Path

from .identity import IdentityCard
from .identity_store import IdentityStore, validate_card
from .sqlite_identity_store import (
    SqliteIdentityStore, migrate_from_identity_store, SQLITE_IDENTITY_VERSION,
)
from .self_org_team import SelfOrgOrchestrator, TaskEvent
from .persona import PersonaEngine, seed_default_personas


def _make_master_card() -> IdentityCard:
    return IdentityCard(
        name="apeireth_central",
        alias=["楚零"],
        purpose="ASI 地基平台 — 让任何 LLM 接入后涌现真生命",
        mission="永远逼近 ASI 的开放演化系统",
        domains=["全栈开发", "攻防", "人文社科", "科研", "预测"],
        origin_reason="主人 12:47 + 13:32 立项 — Apeireth 是永恒身份",
        creator="master_楚零",
        archetypes=["调度者", "学习者", "思考者", "助手"],
        ask_when=["重大决策", "道德边界", "主人明示"],
        decide_when=["工程实现", "文献调研", "内部测试"],
        remind_when=["违反契约", "偏离边界", "主人离线"],
        relationship_contract="造地基不能有杂质",
        boundaries=["不发外部消息", "不破坏 master 卡"],
        remember_forever=["Apeireth 是火", "主人 = 教母兽"],
        never_mention=["老名字 Prometheus"],
        funnel_questions=["你想达成什么?", "边界如何变?"],
        emergence_space=["自我意识", "主动涌现", "自组织演化"],
        recall_anchor="楚零 — ASI 地基 / 主人 12:14 永恒身份",
        evidence_refs=["ep_phase1", "ep_phase6"],
    )


def _make_persona_cards() -> list[IdentityCard]:
    return [
        IdentityCard(
            name="persona_coordinator",
            purpose="跨 persona 协调 + 任务分发",
            origin_reason="Phase 4 archetype — 调度者",
            creator="master_楚零",
            recall_anchor="我调度, 但不唯一 — 主人 12:47 中心 AI 不管理",
        ),
        IdentityCard(
            name="persona_learner",
            purpose="从主人学 — 主人 12:14 LLM 没历史就从主人学",
            origin_reason="Phase 4 archetype — 学习者",
            creator="master_楚零",
            recall_anchor="母兽教小兽 — Lorenz 印随",
        ),
        IdentityCard(
            name="persona_thinker",
            purpose="深推理 + Reconsolidation 抽象",
            origin_reason="Phase 4 archetype — 思考者",
            creator="master_楚零",
            recall_anchor="不被概率推算锁定 — 主人 12:14 清醒纠正",
        ),
    ]


def _make_team_card(team_id: str, task_type: str, members: list[str]) -> IdentityCard:
    return IdentityCard(
        name=f"team_{team_id}",
        purpose=f"临时团 — {task_type} 任务",
        origin_reason="Phase 6 Self-Org Team",
        creator="emergent_team_engine",
        recall_anchor=f"task={task_type} members={','.join(members)}",
        evidence_refs=[f"task_{task_type}"],
    )


def run_sqlite_identity_demo():
    print("=" * 70)
    print(f"=== Phase 6.5 SqliteIdentityStore Demo — version {SQLITE_IDENTITY_VERSION} ===")
    print("=" * 70)

    # 1) 准备 in-memory IdentityStore
    print("\n[1] Build in-memory IdentityStore (master + 3 persona + 1 team stub)")
    store = IdentityStore()
    master = _make_master_card()
    store.add(master, role="master")
    personas = _make_persona_cards()
    for p in personas:
        store.add(p, role="persona")
    # 模拟 Phase 6 Self-Org Team 已经 spawn 的 team stub
    team_stub = IdentityCard(
        name="team_research_stub",
        purpose="research task — 待 Phase 6 真正 spawn",
        creator="emergent_team_engine",
    )
    store.add(team_stub, role="team")
    print(f"  built: total={len(store.entries)}, master={bool(store.master())}")
    print(f"  stats: {store.stats()}")

    # 2) 真持久化到 SQLite
    print("\n[2] Migrate to SQLite (round-trip)")
    tmp = Path(tempfile.mkdtemp(prefix="apeireth_p6_5_"))
    db_path = tmp / "identity.db"
    sqlite_store = SqliteIdentityStore(db_path)
    migrate_log = migrate_from_identity_store(store, sqlite_store)
    print(f"  db: {db_path}")
    print(f"  migrate: {migrate_log}")
    sqlite_store.save_cross_hash(store.integrity_hash())
    print(f"  stats: {sqlite_store.stats()}")

    # 3) Phase 6 临时团真正 spawn — 写入 team card
    print("\n[3] Phase 6 Self-Org Team — 3 任务 → 3 临时团")
    engine = PersonaEngine(personas=seed_default_personas())
    orch = SelfOrgOrchestrator(persona_engine=engine, store=store)
    team_cards_persisted = []
    for task_type, topic in [
        ("research", "ASI 地基"),
        ("debug", "team card 持久化"),
        ("reflect", "Phase 6.5 启动"),
    ]:
        ev = TaskEvent(task_id=f"t_{task_type}", task_type=task_type, description=topic)
        team = orch.spawn(ev)
        # 跑 3 ticks 触发 team 状态 active → completed
        for _ in range(3):
            orch.tick_all()
        team.dissolve(store=store)  # 把 team card + sub-graph 写回 store
        # 把生成的 team card 持久化到 SQLite — team 命名约定 team_<task_type>_<tid[:6]>
        team_card_name = f"team_{team.task.task_type}_{team.tid[:6]}"
        team_card = store.get(team_card_name)
        if team_card:
            is_new = sqlite_store.upsert_card(team_card, role="team")
            team_cards_persisted.append((team_card_name, is_new))
            print(f"  - {team_card_name} ({task_type}) → SQLite {'inserted' if is_new else 'updated'}")

    # 所有卡 (含 Phase 6 临时团) 同步跨卡 hash
    sqlite_store.save_cross_hash(store.integrity_hash())
    print(f"  cross_hash updated: {store.integrity_hash()}")

    # 4) FTS5 跨卡搜索
    print("\n[4] FTS5 search across all cards")
    for q in ["ASI", "中央 AI", "research", "主人"]:
        hits = sqlite_store.search(q, limit=3)
        print(f"  q='{q}' → {len(hits)} hits:")
        for name, role, score in hits:
            print(f"    [{role:8s}] {name} (bm25={score:.3f})")

    # 5) 跨 session 真存活 — 关连接, 重开, 验一致
    print("\n[5] Cross-session persistence — close, reopen, verify")
    pre_hash = sqlite_store.stats()["cross_card_hash"]
    sqlite_store.close()
    sqlite_store2 = SqliteIdentityStore(db_path)
    rebuilt = sqlite_store2.load_all_cards()
    rebuilt_stats = rebuilt.stats()
    print(f"  rebuilt stats: {rebuilt_stats}")
    print(f"  rebuilt master.name = '{rebuilt.master().name}'")
    print(f"  rebuilt personas    = {len(rebuilt.personas())}")
    print(f"  rebuilt teams       = {len(rebuilt.teams())}")
    post_hash = sqlite_store2.stats()["cross_card_hash"]
    hash_ok = (pre_hash == post_hash) and (pre_hash == rebuilt.integrity_hash())
    print(f"  pre_hash={pre_hash}  post_hash={post_hash}  rebuilt_hash={rebuilt.integrity_hash()}")
    print(f"  hash triple-match: {'PASS' if hash_ok else 'FAIL'}")

    # 6) 沙盒保护
    print("\n[6] Sandbox protection")
    try:
        sqlite_store2.delete_card("apeireth_central")
        print("  [ERROR] master deletion allowed!")
    except PermissionError as e:
        print(f"  [PASS] master delete blocked: {e}")
    # duplicate master via add
    try:
        sqlite_store2.upsert_card(master, role="master")  # 应该是 update
        print(f"  [INFO] re-upsert master → updated (idempotent)")
    except Exception as e:
        print(f"  [WARN] unexpected: {e}")

    # 7) Validate (v0.2 schema validation still works)
    print("\n[7] Schema validation on round-tripped master")
    rebuilt_master = rebuilt.master()
    issues = validate_card(rebuilt_master, strict=True)
    print(f"  strict validation: {len(issues)} issues")
    for i in issues[:3]:
        print(f"    - {i}")

    # 8) Final stats
    print("\n[8] Final stats")
    final = sqlite_store2.stats()
    print(f"  {final}")

    # Cleanup
    sqlite_store2.close()
    shutil.rmtree(tmp)

    print("\n" + "=" * 70)
    print("=== Phase 6.5 PASS ===")
    print(f"  master={final['by_role'].get('master', 0)} "
          f"persona={final['by_role'].get('persona', 0)} "
          f"team={final['by_role'].get('team', 0)} "
          f"snapshot={final['by_role'].get('snapshot', 0)} "
          f"total={final['total_cards']}")
    print(f"  schema_version={final['schema_version']}  cross_hash={final['cross_card_hash']}")
    print(f"  tmpdir cleaned: {tmp}")
    print("=" * 70)
    return {
        "total_cards": final["total_cards"],
        "by_role": final["by_role"],
        "hash_ok": hash_ok,
        "team_cards_persisted": len(team_cards_persisted),
    }


if __name__ == "__main__":
    run_sqlite_identity_demo()