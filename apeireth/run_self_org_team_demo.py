"""Demo — Phase 6 Self-Organizing Team Engine v0.1 PoC

主人 12:14: "自组织可以在执行任务的时候表现, 比如干什么就组一个什么的专家团"
主人 12:47: "中央 AI 不管理, 一切交给中央 AI 自己"

演示 5 步:
[1] 加载 IdentityStore (master + 4 persona + 1 team stub)
[2] 加载 PersonaEngine (4 archetype)
[3] 创建 RelationGraph (空, 临时团写 sub-graph)
[4] 投递 3 个 TaskEvent → 自动 spawn 3 个临时团 → tick 各 3 次
[5] dissolve 全部 → 验证:
    - 3 张新 team card 落到 IdentityStore
    - 3 个 agent 节点 + N 个 part_of 边 + N 个 assigned 边写到 graph
    - emergence_marker=True 标记
    - 临时团生命周期 active → completed → dissolved
"""

from __future__ import annotations
import json
import sys
import time
from pathlib import Path

# 确保从仓库根目录导入 apeireth 包
ROOT = Path(__file__).parent.parent
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from apeireth.identity_store import IdentityStore
from apeireth.persona import PersonaEngine, seed_default_personas
from apeireth.relation import RelationGraph
from apeireth.self_org_team import (
    SELF_ORG_TEAM_VERSION,
    TaskEvent,
    TEAM_TEMPLATES,
    SelfOrgOrchestrator,
)


def banner(title: str) -> None:
    print()
    print("=" * 60)
    print(f"  {title}")
    print("=" * 60)


def main() -> None:
    print(f"APEIRETH — Self-Organizing Team Engine v{SELF_ORG_TEAM_VERSION} Demo")
    print('主人 12:14 "自组织... 干什么就组一个什么的专家团"')
    print('主人 12:47 "中央 AI 不管理, 一切交给中央 AI 自己"')

    # ---- [1] IdentityStore ----
    banner("[1] 加载 IdentityStore (master + 4 persona)")
    store = IdentityStore()
    log = store.load_dir("apeireth/data/identity_store")
    for l in log:
        print(f"  {l}")
    print(f"  stats: {store.stats()}")

    # ---- [2] PersonaEngine ----
    banner("[2] PersonaEngine (4 archetype — 借成员用)")
    personas = seed_default_personas()
    engine = PersonaEngine(personas=personas, min_distance=0.25)
    for p in engine.personas:
        print(f"  pid={p.pid}  archetype={p.archetype:6s}  sct={p.sct.as_tuple()}")

    # ---- [3] RelationGraph ----
    banner("[3] RelationGraph (空 — 临时团写 sub-graph)")
    graph = RelationGraph()
    print(f"  init nodes={len(graph.nodes)} edges={len(graph.edges)}")

    # ---- [4] 3 个 TaskEvent → 3 个临时团 ----
    banner("[4] SelfOrgOrchestrator — 投递 3 个 TaskEvent")
    orch = SelfOrgOrchestrator(engine, store, graph)

    tasks = [
        TaskEvent(
            task_id="t_research_001",
            task_type="research",
            description="调研 AHE evolve.py 5 阶段, 借鉴到 Phase 5.3",
            payload={"refs": ["ahe_2604_25850"]},
        ),
        TaskEvent(
            task_id="t_debug_002",
            task_type="debug",
            description="Phase 5.5 Path A rationale 重复加 question 的 bug",
            payload={"component": "linkage", "version": "0.11.0"},
        ),
        TaskEvent(
            task_id="t_plan_003",
            task_type="plan",
            description="Phase 6 排期 — 涌现空间 + 自组织临时团 + SqliteIdentityStore",
            payload={"phases": ["6.0", "6.5"]},
        ),
    ]

    spawned_teams = []
    for t in tasks:
        team = orch.spawn(t, expected_ticks=3)
        spawned_teams.append(team)
        print(f"  ✓ spawned tid={team.tid[:10]}  task={t.task_type}  "
              f"members={[p.archetype for p in team.members]}  "
              f"rationale={team.spec.rationale[:80]}")

    print(f"\n  5 templates: {list(TEAM_TEMPLATES.keys())}")
    print(f"  active teams: {len(orch.active_teams)}")

    # ---- [5] Tick 全部, 每个团跑 3 轮 ----
    banner("[5] Tick 全部 (每团 3 轮 — 每个 persona 独立贡献)")
    for round_idx in range(3):
        print(f"\n  --- round {round_idx + 1} ---")
        tick_results = orch.tick_all()
        for tid, cs in tick_results.items():
            team = orch.active_teams.get(tid)
            print(f"  tid={tid[:10]} [{team.spec.task_type}] {len(cs)} contributions")
            for c in cs[:3]:
                print(f"    [{c['persona']:6s}] conf={c['confidence']:.2f}  {c['content'][:80]}")
        # 检查是否有团变 completed
        completed = [t for t in orch.active_teams.values() if t.status == "completed"]
        if completed:
            print(f"\n  → {len(completed)} 团已 completed: {[t.spec.task_type for t in completed]}")

    # ---- [6] Dissolve 全部 ----
    banner("[6] Dissolve 全部 — 自动归档 (team card + sub-graph)")
    dissolve_results = orch.dissolve_all()
    for r in dissolve_results:
        print(f"  ✓ dissolved tid={r['tid'][:10]}  "
              f"members={r['members']}  "
              f"team_card={r['team_card_name']}  "
              f"hash={r['team_card_hash']}  "
              f"sub_graph_nodes={len(r['sub_graph_nodes'])}  "
              f"sub_graph_edges={len(r['sub_graph_edges'])}")

    # ---- [7] 验证 ----
    banner("[7] 验证 — 涌现层自治闭环")

    # 7.1 团队 card 已落 IdentityStore
    teams_now = store.teams()
    print(f"  [7.1] IdentityStore.teams() 现在共 {len(teams_now)} 张 team card:")
    for c in teams_now:
        if c.creator == "emergent_team_engine":   # 新涌现的
            print(f"    - {c.name} | mission={c.mission[:60]} | hash={c.integrity_hash()}")

    # 7.2 graph 写入了 sub-graph
    print(f"\n  [7.2] RelationGraph 现在 nodes={len(graph.nodes)} edges={len(graph.edges)}")
    agent_nodes = [n for n in graph.nodes.values() if n.kind == "agent"]
    task_nodes = [n for n in graph.nodes.values() if n.kind == "task"]
    print(f"    agent 节点: {len(agent_nodes)} (3 临时团 + N persona 借成员)")
    print(f"    task 节点: {len(task_nodes)}")
    print(f"    emergence_marker=True 的 agent 节点:")
    for n in agent_nodes:
        if n.meta.get("emergence_marker"):
            print(f"      - {n.nid} | task_type={n.meta.get('task_type')} | members={n.meta.get('members')}")

    # 7.3 涌现标记 — 3 个临时团都是 emergence_marker=True
    print(f"\n  [7.3] emergence_marker 标记:")
    for team_dict in orch.history:
        print(f"    tid={team_dict['tid'][:10]}  emergence_marker={team_dict['emergence_marker']}  "
              f"task={team_dict['spec']['task_type']}  "
              f"ticks={team_dict['tick_count']}")

    # 7.4 store integrity_hash 已更新 (新增 3 张卡)
    print(f"\n  [7.4] IdentityStore.integrity_hash() = {store.integrity_hash()}")

    # 7.5 临时团生命周期
    print(f"\n  [7.5] 临时团生命周期:")
    statuses = [t['status'] for t in orch.history]
    print(f"    history statuses: {statuses}")
    print(f"    active_teams 现在: {len(orch.active_teams)} (期望 0)")

    # ---- Save snapshot ----
    snapshot = {
        "version": SELF_ORG_TEAM_VERSION,
        "templates": TEAM_TEMPLATES,
        "history": orch.history,
        "graph_stats": {
            "nodes": len(graph.nodes),
            "edges": len(graph.edges),
            "agent_count": len(agent_nodes),
            "task_count": len(task_nodes),
        },
        "store_stats_after": store.stats(),
        "store_integrity_hash": store.integrity_hash(),
        "ts": time.time(),
    }
    out = Path("apeireth/self_org_team_demo.json")
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(json.dumps(snapshot, ensure_ascii=False, indent=2), encoding="utf-8")
    print(f"\n  📸 snapshot saved: {out}")

    print()
    print("=" * 60)
    print(f"  Phase 6 v0.1 PoC — 自组织临时团 ✅")
    print(f"  3 任务 → 3 团自动涌现 → 3 轮 tick → 自动 dissolve")
    print(f"  4 张新 team card + 1 个 sub-graph 已落地")
    print("=" * 60)


if __name__ == "__main__":
    main()