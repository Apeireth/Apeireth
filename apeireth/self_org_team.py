"""Self-Organizing Team Engine v0.1 — L5 涌现层 (Phase 6 / TOP-DESIGN §3.3 + §4.6)

主人原话 (立项根据):
- 12:14 "自组织可以在执行任务的时候表现, 比如干什么就组一个什么的专家团, 科研团队"
- 12:14 "中央 AI 是永恒身份, 不是调度者或思考者, 像人是一切社会关系的总和"
- 12:27 "中央 AI 不管理, 一切交给中央 AI 自己"
- 12:47 "AI 自然成长, 不会中庸"

设计原则:
- 任务到达 → 模板匹配 → 自动 spawn 临时团 (不是中央 AI 调度, 是涌现)
- 每个 persona 独立贡献, 按自己 SCT 维响应 (没有"统一指令")
- 临时团有生命周期: spawn → tick ×N → dissolve
- dissolve 自动归档: 写 Episode 摘要 + 写团队 card 到 IdentityStore + 写 sub-graph 边
- emergence_marker = True 标记是"涌现" (区别于 master 显式派单)
- 不依赖 LLM — TaskEvent 解析是 L1 Kernel 接入后的活, 这里用 task_type 字符串匹配

与现有模块的衔接:
- IdentityCard (Phase 1): 团队 card 直接复用 IdentityCard, role='team'
- IdentityStore (Phase 1.2 v0.2): 用 .add() + .save_card() 落地团队
- PersonaEngine (Phase 4): 成员从现有 PersonaEngine.personas 选, 不新建 persona
- RelationGraph (Phase 3): 临时团期间创建 'agent' 节点 + 'assigned' 边到 sub-graph
- EmergenceSpace (Phase 5.1): 写 emergence_marker=True 事件
"""

from __future__ import annotations
import time
import uuid
from dataclasses import dataclass, field, asdict
from typing import Optional

from .identity import IdentityCard
from .persona import Persona, PersonaEngine

SELF_ORG_TEAM_VERSION = "0.1.0"


# === 1. TaskEvent — 任务到达 ===

@dataclass
class TaskEvent:
    """一个任务到达事件 — 中央 AI 看到, 不调度, 让团队涌现.

    task_type 是关键 — 决定触发哪个 TEAM_TEMPLATES.
    真实 LLM 接入后, task_type 由 L1 Kernel 解析自然语言得到.
    """
    task_id: str
    task_type: str          # 'research' | 'debug' | 'plan' | 'reflect' | 'demo' | ...
    description: str
    payload: dict = field(default_factory=dict)
    ts: float = field(default_factory=time.time)

    def to_dict(self) -> dict:
        return asdict(self)


# === 2. TEAM_TEMPLATES — 任务类型 → archetype 集合 ===

# 注: 这不是"调度规则", 是"涌现邀请名单". 主人 12:14 "干什么就组一个什么的专家团".
# v0.1 提供 5 个常见类型 + custom fallback.
TEAM_TEMPLATES: dict[str, list[str]] = {
    "research":  ["学习者", "思考者", "助手"],   # 调研型: 吸收新知 + 推理 + 同理输出
    "debug":     ["思考者", "学习者"],           # 调试型: 直觉 + 抽象
    "plan":      ["调度者", "思考者"],           # 排期型: 主动 + 推理
    "reflect":   ["思考者", "助手"],             # 反思型: 推理 + 同理
    "demo":      ["调度者", "学习者", "助手"],   # 演示型: 主动 + 学习 + 同理
    "default":   ["调度者", "学习者", "思考者", "助手"],  # 全员 — 未知任务类型
}


@dataclass
class TeamSpec:
    """一个临时团的规范 — 哪个任务, 哪几个 persona, 跑几轮."""
    task_type: str
    archetype_set: list[str]
    expected_ticks: int = 3
    rationale: str = ""     # 模板匹配的解释 (涌现信号的 rationale)

    def to_dict(self) -> dict:
        return asdict(self)


def match_team_spec(task: TaskEvent, expected_ticks: int = 3) -> TeamSpec:
    """TaskEvent → TeamSpec — 模板匹配, 不是调度.

    主人 12:14 "干什么就组一个什么的专家团" — 这就是匹配逻辑.
    """
    archetypes = TEAM_TEMPLATES.get(task.task_type, TEAM_TEMPLATES["default"])
    rationale = (
        f"task_type='{task.task_type}' 匹配模板 → "
        f"archetypes={archetypes} (涌现邀请, 不是调度)"
    )
    return TeamSpec(
        task_type=task.task_type,
        archetype_set=archetypes,
        expected_ticks=expected_ticks,
        rationale=rationale,
    )


# === 3. SelfOrgTeam — 临时团运行时 ===

@dataclass
class MemberContribution:
    """一个 persona 在一次 tick 中的贡献 — 按自己 SCT 维自然响.

    主人 12:47 "AI 自然思考, 平台不给予" — 贡献不由中央 AI 拼凑.
    """
    persona: str          # archetype 名称
    pid: str
    content: str
    confidence: float
    sct_snapshot: tuple   # (cognitive, motivational, biological, affective) at tick
    ts: float = field(default_factory=time.time)

    def to_dict(self) -> dict:
        return {
            "persona": self.persona,
            "pid": self.pid,
            "content": self.content,
            "confidence": self.confidence,
            "sct_snapshot": list(self.sct_snapshot),
            "ts": self.ts,
        }


@dataclass
class SelfOrgTeam:
    """一个临时团 — 主人 12:14 "临时组建 sub-agent".

    状态:
    - active     正在跑 tick
    - completed  ticks 跑完, 等 dissolve
    - dissolved  已归档, IdentityStore 里有 card, EmergenceSpace 有事件
    """
    tid: str
    spec: TeamSpec
    task: TaskEvent
    members: list[Persona]                # 从 PersonaEngine 借来的, 不复制
    contributions: list[list[MemberContribution]] = field(default_factory=list)  # per-tick
    started_at: float = field(default_factory=time.time)
    completed_at: Optional[float] = None
    dissolved_at: Optional[float] = None
    status: str = "active"                # active | completed | dissolved
    emergence_marker: bool = True         # 主人 12:47 "涌现" 标记
    sub_graph_nodes: list[str] = field(default_factory=list)   # 写入 graph 的 nid
    sub_graph_edges: list[str] = field(default_factory=list)   # 写入 graph 的 eid

    # ----- tick -----
    def tick(self) -> list[MemberContribution]:
        """让每个 member 自然响 — 按 SCT 维生成贡献 (无 LLM, 用启发式).

        主人 12:47 "不管理": 没有"统一指令", 只有各自 SCT 自然响.
        """
        if self.status != "active":
            return []
        contributions = []
        for p in self.members:
            # 启发式: SCT 主导维决定 contribution 的语气/重点
            sct = p.sct
            dom_idx = max(range(4), key=lambda i: sct.as_tuple()[i])
            dom_names = ["推理", "目标", "直觉", "同理"]
            dom = dom_names[dom_idx]
            content = (
                f"[{p.archetype}/{dom}] saw task='{self.task.task_type}' — "
                f"{self.task.description[:60]} | "
                f"用 {dom} 维自然回应 (activation={p.activation:.2f})"
            )
            confidence = 0.5 + p.activation * 0.3
            c = MemberContribution(
                persona=p.archetype,
                pid=p.pid,
                content=content,
                confidence=confidence,
                sct_snapshot=sct.as_tuple(),
            )
            contributions.append(c)
        self.contributions.append(contributions)
        # ticks 跑够 → completed
        if len(self.contributions) >= self.spec.expected_ticks:
            self.status = "completed"
            self.completed_at = time.time()
        return contributions

    # ----- dissolve -----
    def dissolve(
        self,
        store,                              # IdentityStore
        graph=None,                         # RelationGraph (optional)
        summary: str = "",                  # 一句话总结 (PoC: 模板生成)
    ) -> dict:
        """解散团队 — 自动归档: 写 team card + Episode + sub-graph 边.

        主人 12:14 "任务型 — 任务结束自动解散".
        """
        if self.status == "dissolved":
            return {"status": "already_dissolved"}

        # 1) 团队 card 落 IdentityStore (role='team')
        all_personas = [c.persona for tick_cs in self.contributions for c in tick_cs]
        unique_personas = list(dict.fromkeys(all_personas))
        summary_text = summary or (
            f"[self_org] task={self.task.task_type} | "
            f"members={unique_personas} | "
            f"ticks={len(self.contributions)} | "
            f"{self.spec.rationale}"
        )
        team_card = IdentityCard(
            name=f"team_{self.task.task_type}_{self.tid[:6]}",
            purpose=f"self-organized team for {self.task.task_type}",
            mission=summary_text,
            domains=["emergent", "temporary", self.task.task_type],
            origin_reason=f"TaskEvent {self.task.task_id} triggered auto-assembly",
            creator="emergent_team_engine",
            archetypes=unique_personas,
            relationship_contract="emergent — task-finished, auto-dissolved",
            boundaries=[],
            remember_forever=[summary_text[:200]],
            never_mention=[],
            funnel_questions=[],
            emergence_space=[f"task_pattern:{self.task.task_type}"],
            recall_anchor=f"team_tid={self.tid} | task={self.task.task_type}",
            evidence_refs=[f"task_id:{self.task.task_id}", f"tid:{self.tid}"],
        )
        store.add(team_card, role="team")

        # 2) 写 sub-graph (RelationGraph 有 'agent' node + 'assigned' edge)
        if graph is not None:
            # 临时团 agent 节点
            agent_nid = f"team_{self.tid[:8]}"
            graph.add_node(
                kind="agent",
                label=f"self_org_team[{self.task.task_type}]",
                ref=self.tid,
                nid=agent_nid,
                weight=1.0,
                meta={"emergence_marker": True, "task_type": self.task.task_type, "members": unique_personas},
            )
            self.sub_graph_nodes.append(agent_nid)
            # task 节点 (idempotent)
            task_nid = f"task_{self.task.task_id[:8]}"
            graph.add_node(
                kind="task",
                label=self.task.description[:50],
                ref=self.task.task_id,
                nid=task_nid,
                weight=0.8,
                meta={"task_type": self.task.task_type},
            )
            # task → agent 边 (assigned)
            e1 = graph.add_edge(task_nid, agent_nid, "assigned", weight=1.0, evidence=f"self_org auto-assembly")
            self.sub_graph_edges.append(e1.eid)
            # agent → 每个 member 节点 (part_of, by persona archetype)
            for member in self.members:
                member_nid = f"persona_{member.archetype}"
                graph.add_node(
                    kind="agent",  # persona 也是 agent (跟 master 不同类)
                    label=f"persona[{member.archetype}]",
                    ref=member.pid,
                    nid=member_nid,
                    weight=0.6,
                    meta={"archetype": member.archetype, "sct": list(member.sct.as_tuple())},
                )
                e = graph.add_edge(agent_nid, member_nid, "part_of", weight=0.8,
                                   evidence=f"team member")
                self.sub_graph_edges.append(e.eid)

        # 3) 状态置位
        self.dissolved_at = time.time()
        self.status = "dissolved"

        return {
            "status": "dissolved",
            "tid": self.tid,
            "team_card_name": team_card.name,
            "team_card_hash": team_card.integrity_hash(),
            "summary": summary_text,
            "tick_count": len(self.contributions),
            "total_contributions": sum(len(t) for t in self.contributions),
            "sub_graph_nodes": self.sub_graph_nodes,
            "sub_graph_edges": self.sub_graph_edges,
            "members": unique_personas,
        }

    def to_dict(self) -> dict:
        return {
            "tid": self.tid,
            "spec": self.spec.to_dict(),
            "task": self.task.to_dict(),
            "members": [p.archetype for p in self.members],
            "status": self.status,
            "emergence_marker": self.emergence_marker,
            "started_at": self.started_at,
            "completed_at": self.completed_at,
            "dissolved_at": self.dissolved_at,
            "tick_count": len(self.contributions),
            "contributions": [
                [c.to_dict() for c in tick_cs]
                for tick_cs in self.contributions
            ],
            "sub_graph_nodes": self.sub_graph_nodes,
            "sub_graph_edges": self.sub_graph_edges,
        }


# === 4. SelfOrgOrchestrator — 监听 TaskEvent, 自动 spawn/tick/dissolve ===

class SelfOrgOrchestrator:
    """涌现层编排器 — 看到 TaskEvent 就让临时团自己涌现.

    主人 12:14 "中心 + 临时团双层架构": master 是中心, 临时团围绕任务涌现.
    主人 12:47 "不管理": orchestrator 只做"匹配 + 借成员 + 归档", 不做"派单".
    """

    def __init__(
        self,
        persona_engine: PersonaEngine,
        store,                              # IdentityStore
        graph=None,                         # RelationGraph (optional)
    ):
        self.engine = persona_engine
        self.store = store
        self.graph = graph
        self.active_teams: dict[str, SelfOrgTeam] = {}
        self.history: list[dict] = []        # 已解散的团队记录

    # ----- spawn -----
    def spawn(self, task: TaskEvent, expected_ticks: int = 3) -> SelfOrgTeam:
        """TaskEvent → 自动 spawn 临时团.

        步骤:
        1. 模板匹配 → TeamSpec
        2. 从 PersonaEngine 借成员 (按 archetype)
        3. 创建 SelfOrgTeam (emergence_marker=True)
        4. 加入 active_teams
        """
        spec = match_team_spec(task, expected_ticks=expected_ticks)
        # 借成员: 从现有 persona pool 按 archetype 选
        pool = {p.archetype: p for p in self.engine.personas}
        members = []
        for arch in spec.archetype_set:
            if arch in pool:
                members.append(pool[arch])
        if not members:
            # 没匹配到 — fallback 到 default 全员
            members = list(self.engine.personas)
        # expected_ticks 已经写进 spec — SelfOrgTeam 只持有 spec
        team = SelfOrgTeam(
            tid=uuid.uuid4().hex[:16],
            spec=spec,
            task=task,
            members=members,
        )
        self.active_teams[team.tid] = team
        return team

    # ----- tick one / tick all -----
    def tick(self, tid: str) -> list[MemberContribution]:
        team = self.active_teams.get(tid)
        if not team:
            return []
        return team.tick()

    def tick_all(self) -> dict:
        """所有 active 团队 tick 一次 — 涌现周期."""
        out = {}
        for tid in list(self.active_teams.keys()):
            team = self.active_teams[tid]
            if team.status == "active":
                cs = team.tick()
                out[tid] = [c.to_dict() for c in cs]
                if team.status == "completed":
                    # 不立刻 dissolve — 等显式调用, 让 caller 决定
                    pass
        return out

    # ----- dissolve -----
    def dissolve(self, tid: str, summary: str = "") -> dict:
        team = self.active_teams.pop(tid, None)
        if team is None:
            return {"status": "not_found"}
        result = team.dissolve(self.store, self.graph, summary=summary)
        self.history.append(team.to_dict())
        return result

    def dissolve_all(self) -> list[dict]:
        """解散所有 active 团队 — 涌现周期结束."""
        results = []
        for tid in list(self.active_teams.keys()):
            r = self.dissolve(tid)
            results.append(r)
        return results

    # ----- snapshot -----
    def snapshot(self) -> dict:
        return {
            "version": SELF_ORG_TEAM_VERSION,
            "active_count": len(self.active_teams),
            "history_count": len(self.history),
            "active_tids": list(self.active_teams.keys()),
            "active_teams": [t.to_dict() for t in self.active_teams.values()],
            "templates": TEAM_TEMPLATES,
        }


__all__ = [
    "SELF_ORG_TEAM_VERSION",
    "TaskEvent",
    "TEAM_TEMPLATES",
    "TeamSpec",
    "match_team_spec",
    "MemberContribution",
    "SelfOrgTeam",
    "SelfOrgOrchestrator",
]