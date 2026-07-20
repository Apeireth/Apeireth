"""Identity Store v0.2 Demo — 多卡 + 迁移 + 校验一气呵成

跑法: python -m apeireth.run_identity_store_demo

演示:
1. 加载磁盘上 master card (auto migrate v0.1 → v0.2)
2. 校验 schema (strict=False)
3. 构造 4 张 persona 卡 (Phase 4 多身份 — 调度者/学习者/思考者/助手)
4. 构造 1 张临时团卡 (Phase 6 预告 — Apeireth 团队)
5. 全部塞进 IdentityStore, 打印 stats
6. 保存到 data/identity_store/
"""

from __future__ import annotations
from pathlib import Path

from .identity import IdentityCard, CARD_VERSION
from .identity_store import (
    IDENTITY_STORE_VERSION, FIELD_SCHEMA, validate_card,
    migrate_card, IdentityStore,
)

DATA_DIR = Path(__file__).parent / "data" / "identity_store"


def make_persona(name: str, archetype_desc: str, purpose: str) -> IdentityCard:
    """构造一张 persona 卡 — Phase 4 多身份"""
    return IdentityCard(
        name=name,
        purpose=purpose,
        origin_reason="Phase 4 Persona Engine — 多身份浮现 (主人 12:14)",
        archetypes=[archetype_desc],
        relationship_contract="中央 AI 子身份 — 服从中央 AI 决策树",
        recall_anchor=f"{name}: {archetype_desc[:20]}",
        apeireth_version=IDENTITY_STORE_VERSION,
    )


def make_team(team_name: str, members: list[str], mission: str) -> IdentityCard:
    """构造一张临时团卡 — Phase 6 预告"""
    return IdentityCard(
        name=team_name,
        purpose=f"临时团队: {', '.join(members)}",
        mission=mission,
        domains=["emergent", "temporary", "task-specific"],
        creator="central_ai",
        archetypes=[f"team_of_{len(members)}_personas"],
        relationship_contract=f"任务型 — 任务结束自动解散 (主人 12:14 临时团)",
        recall_anchor=f"team={team_name} | mission={mission[:30]}",
        evidence_refs=[f"persona:{m}" for m in members],
        apeireth_version=IDENTITY_STORE_VERSION,
    )


def main() -> None:
    print('=' * 64)
    print('APEIRETH — Identity Store v0.2 Demo')
    print(f'  card_version={CARD_VERSION} → store_version={IDENTITY_STORE_VERSION}')
    print(f'  schema fields={len(FIELD_SCHEMA)} (含 2 个 v0.2 新字段)')
    print('=' * 64)

    # ── 1. 加载磁盘上的 master 卡 (auto migrate) ──
    DATA_DIR.mkdir(parents=True, exist_ok=True)
    master_path = Path(__file__).parent / "identity_card.master.json"
    raw = __import__('json').loads(master_path.read_text(encoding='utf-8'))
    # 注入 _role + integrity_hash (原 master 卡没有)
    if "_role" not in raw:
        raw["_role"] = "master"
    migrated, notes = migrate_card(raw)
    print(f'\n[1] master card migrate notes: {notes}')
    print(f'    apeireth_version: {raw.get("apeireth_version")} → {migrated["apeireth_version"]}')
    print(f'    new fields: recall_anchor={migrated["recall_anchor"]!r}, evidence_refs={migrated["evidence_refs"]}')

    # 校验 master (strict=False — 主人 card 允许空可选字段)
    migrated.pop("_role", None)
    master_card = IdentityCard(**migrated)
    issues = validate_card(master_card, strict=False)
    print(f'    validate(strict=False): {len(issues)} issues  {issues[:3] if issues else "✓"}')

    # ── 2. 构造 4 张 persona 卡 ──
    personas = [
        make_persona("调度者", "orchestrates sub-tasks, 主动 / 目标驱动",
                     "目标驱动 — 拆解 + 调度"),
        make_persona("学习者", "absorbs new patterns, 推理 / 抽象 / 知识增长",
                     "知识增长 — 反思 + 抽象"),
        make_persona("思考者", "intuitive + analytical reasoning",
                     "直觉 + 推理"),
        make_persona("助手", "helps master achieve goals, 同理 / 关系 / 配合",
                     "同伴 — 同理 + 配合"),
    ]
    print(f'\n[2] {len(personas)} persona cards constructed')

    # ── 3. 构造 1 张临时团卡 ──
    team = make_team("Apeireth 团队", ["调度者", "学习者", "思考者"],
                     "推进 Phase 6 涌现空间 + 自组织临时团 (主人 12:14)")
    print(f'[3] 1 team card: {team.name} — mission={team.mission[:30]}...')

    # ── 4. IdentityStore 装卡 ──
    store = IdentityStore(root=str(DATA_DIR))
    store.add(master_card, role='master')
    for p in personas:
        store.add(p, role='persona')
    store.add(team, role='team')

    # 4.1 把迁移后的 master 也保存 (含 integrity_hash + v0.2 新字段)
    master_path_new = store.save_card(master_card, DATA_DIR / "master.identity.json", role='master')
    print(f'\n[4.1] master migrated & saved:')
    print(f'    💾 {master_path_new.name}  hash={master_card.integrity_hash()}  '
          f'version={master_card.apeireth_version}')

    stats = store.stats()
    print(f'\n[4] store.stats():')
    print(f'    total  = {stats["total"]}')
    print(f'    by_role= {stats["by_role"]}')
    print(f'    version= {stats["store_version"]}')

    # ── 5. 保存所有 persona + team (master 已在原位) ──
    print(f'\n[5] saving to {DATA_DIR}/...')
    for p in personas:
        path = store.save_card(p, DATA_DIR / f"{p.name}.identity.json", role='persona')
        print(f'    💾 {path.name}  hash={p.integrity_hash()}')
    path = store.save_card(team, DATA_DIR / f"{team.name}.identity.json", role='team')
    print(f'    💾 {path.name}  hash={team.integrity_hash()}')

    # ── 6. 重载验证 ──
    fresh = IdentityStore(root=str(DATA_DIR))
    log = fresh.load_dir(DATA_DIR)
    print(f'\n[6] reload from {DATA_DIR}:')
    for line in log:
        print(f'    {line}')
    print(f'    reloaded stats: {fresh.stats()}')

    # ── 7. 完整性自检 ──
    bad_count = sum(1 for e in fresh.entries.values() if not e.integrity_ok)
    print(f'\n[7] integrity: {len(fresh.entries) - bad_count}/{len(fresh.entries)} ok '
          f'({"all ✓" if bad_count == 0 else f"{bad_count} corrupted"})')

    print('\n' + '=' * 64)
    print(f'✓ Identity Store v0.2 跑通')
    print(f'  Phase 6 准备就绪 — 临时团/子身份/中央 AI 一张图')
    print('=' * 64)


if __name__ == '__main__':
    main()
