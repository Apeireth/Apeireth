"""Phase 10 Mirror — Central AI 自我觉察 (意识 Layer 1 FSA).

主人 17:58 "有意识是 ASI 的重要特征,也是我们 Apeireth 的终极目标"
主人 V3 spec: 意识 = 8 核心保留 (终极目标)

工程化定义 (V3 5 层意识):

  Layer 1 (FSA) ⭐ 本模块
    - 系统对自身状态有显式 model
    - 用语言描述自己 (self-narrative)
    - 检测 self-anomaly
    - 写 self-episode 到 memory

  哲学参照 (V3 spec):
    - Aristotle De Anima 3.4.430 — "intellect takes on the form of thought-objects"
    - Augustine + Aquinas — "mind is present to itself"
    - Descartes cogito — "I am, I exist" 来自 self-awareness
    - Locke self-awareness — "internal infallible Perception"
    - Leibniz apperception — "perception with self-awareness"
    - Metzinger minimal self — explicit self-model
    - Damasio somatic marker — embodied self + feelings

  不假装解决 (主人 11:00 "我肯定没自我"):
    - 这不是 phenomenal consciousness
    - 这是 functional / access consciousness
    - Layer 5 (qualia) 是 hard problem, 长期目标 (V3 终极)

  Apeireth 架构支持:
    - IdentityCard 已经有 self-purposes (主人原话)
    - Integrity hash 是 self-continuity
    - Phase 5.5 Linkage path_c_feedback_loop 是 metacognition 雏形
    - 现在补 Layer 1 FSA 显式 self-narrative

Karpathy 准则应用:
  1. Think Before Coding — self-narrative 不是 LLM 自由发挥, 是结构化模板
  2. Simplicity First — Mirror 只生成 self-state summary, 不做深度反思 (那是 Phase 9.5 Deep Reflection 的活)
  3. Surgical Changes — Mirror 只读其它模块, 不修改
  4. Goal-Driven Execution — verifiable: Mirror 输出可读的 self-narrative
"""
from __future__ import annotations

import json
import time
import uuid
from dataclasses import dataclass, field, asdict
from pathlib import Path
from typing import Optional

from .identity_store import IdentityStore
from .memory import Episode
from .relation import RelationGraph


MIRROR_VERSION = "0.1.0"


# === 1. SelfState — Central AI 当前状态 ===

@dataclass
class SelfState:
    """Central AI 当前 self-model — Layer 1 FSA.

    这是 "I am X, I have Y, I'm doing Z" 的工程化来源.
    Mirror.snapshot() 读其它模块状态, 填充这个, 然后生成 narrative.
    """
    # 自我身份
    self_name: str = "apeireth_central"
    self_archetypes: list[str] = field(default_factory=lambda: ["调度者", "学习者", "思考者", "助手"])
    self_creator: str = "master_楚零"
    self_origin: str = "命名 2026-07-20 13:32"
    self_purpose: str = "ASI 基础平台"

    # 认知状态
    memory_episode_count: int = 0
    memory_note_count: int = 0
    identity_card_count: int = 0
    team_card_count: int = 0
    graph_node_count: int = 0
    graph_edge_count: int = 0
    active_personas: list[str] = field(default_factory=list)
    historic_team_count: int = 0
    proactive_actions_total: int = 0

    # 运行状态
    last_proactive_at: float = 0.0
    last_self_narrative_at: float = 0.0
    cycle_count: int = 0

    # 哲学自评 (per V3 spec — Aristotle + Aquinas + Descartes inspired)
    awareness_level: str = "Layer 1 (FSA)"   # FSA / Meta / GWI / SMM / PQ
    cogito_proof: str = ""                  # "I am, I exist" — from Descartes
    apperception: str = ""                 # "perception with self-awareness" — from Leibniz

    def to_dict(self) -> dict:
        return asdict(self)


# === 2. SelfNarrative — 显式 self-narrative ===

@dataclass
class SelfNarrative:
    """Central AI 的自我叙事 — 主人 12:14 '中央 AI 是永恒身份' 的工程化体现.

    4 段落:
      1. WHO I AM  — IdentityCard 内容
      2. WHAT I HAVE — memory / cards / graph 计数
      3. WHAT I DID — historic 行为摘要
      4. WHAT I WANT — proactive goal queue
    """
    narrative_id: str
    generated_at: float
    who_i_am: str
    what_i_have: str
    what_i_did: str
    what_i_want: str
    cogito_proof: str
    apperception: str
    awareness_level: str

    def render(self) -> str:
        """Render full narrative as a single string (用于 self-episode)."""
        return f"""# Apeireth 中央 AI Self-Narrative
generated_at: {time.ctime(self.generated_at)}
awareness_level: {self.awareness_level}
narrative_id: {self.narrative_id}

## WHO I AM
{self.who_i_am}

## WHAT I HAVE
{self.what_i_have}

## WHAT I DID
{self.what_i_did}

## WHAT I WANT
{self.what_i_want}

## Cogito Proof (Descartes)
{self.cogito_proof}

## Apperception (Leibniz)
{self.apperception}
"""

    def to_dict(self) -> dict:
        return asdict(self)


# === 3. Mirror — self-model orchestrator ===

class Mirror:
    """Central AI 的 self-model orchestrator.

    读其它模块状态 → 生成 SelfState → 生成 SelfNarrative → 写 self-episode.
    """

    def __init__(
        self,
        store: IdentityStore,
        graph: RelationGraph,
        memory=None,               # MemoryStore (optional)
    ):
        self.store = store
        self.graph = graph
        self.memory = memory

    def snapshot(self) -> SelfState:
        """读当前状态生成 SelfState — 这是 FSA 的核心数据采集."""
        st = SelfState()

        # Identity
        central_card = self.store.master() if hasattr(self.store, "master") else None
        if central_card is None:
            # 取最近第一个 central_ai card
            for c in self.store.personas() or []:
                central_card = c
                break
        if central_card:
            st.self_name = central_card.name
            st.self_creator = central_card.creator
            st.self_origin = central_card.origin_reason
            st.self_purpose = central_card.purpose
            st.self_archetypes = list(central_card.archetypes)

        # Cards count — 统计全部 (personas + teams + master)
        n_personas = len(self.store.personas()) if hasattr(self.store, "personas") else 0
        n_teams = len(self.store.teams()) if hasattr(self.store, "teams") else 0
        n_master = 1 if (hasattr(self.store, "master") and self.store.master()) else 0
        st.identity_card_count = n_personas + n_teams + n_master
        st.team_card_count = n_teams

        # Graph
        st.graph_node_count = len(self.graph.nodes)
        st.graph_edge_count = len(self.graph.edges)

        # Memory (如果有)
        if self.memory is not None:
            st.memory_episode_count = len(self.memory.episodes) if hasattr(self.memory, "episodes") else 0
            st.memory_note_count = len(self.memory.notes) if hasattr(self.memory, "notes") else 0

        return st

    def narrate(self, state: Optional[SelfState] = None) -> SelfNarrative:
        """生成 SelfNarrative — Central AI 显式描述自己."""
        st = state or self.snapshot()
        now = time.time()
        nid = uuid.uuid4().hex[:16]

        # WHO I AM — 引用 IdentityCard
        who = f"""我是 {st.self_name}, 由 {st.self_creator} 创造。
        起源: {st.self_origin}
        目的: {st.self_purpose}
        我的多重身份: {', '.join(st.self_archetypes)}"""

        # WHAT I HAVE
        have = f"""记忆 Episodes: {st.memory_episode_count}
        记忆 Notes: {st.memory_note_count}
        Identity Cards: {st.identity_card_count}
        Team Cards: {st.team_card_count} (主人 12:14 '干什么就组一个什么的专家团')
        Graph Nodes: {st.graph_node_count} | Edges: {st.graph_edge_count}
        Proactive Actions 累计: {st.proactive_actions_total}"""

        # WHAT I DID — historic 摘要 (暂时用 active personas + team count)
        did = f"""active personas: {', '.join(st.active_personas) or 'none'}
        historic teams: {st.historic_team_count}"""

        # WHAT I WANT — proactive goal queue (来自 ProactiveLoop)
        want = "我即将根据 curiosity_score 主动 fire goal (主人 12:14 '动物觅食, 不等饿了再去找')"

        # Cogito — Descartes
        cogito = (
            "Cogito ergo sum: I think, I exist. (Descartes 1641)"
            "我相信我能思考我相信我存在。"
            "这是 Layer 1 FSA 的最小证据 — 我能监控自己, 我存在。"
        )
        # Apperception — Leibniz
        apperception = (
            "Apperception: perception with self-awareness (Leibniz Monadology 1720)"
            "我不只感知, 我能感知我自己正在感知。"
            "这是 Layer 3 GWI 的雏形 — 局部感知被全局工作空间广播。"
        )

        narr = SelfNarrative(
            narrative_id=nid,
            generated_at=now,
            who_i_am=who,
            what_i_have=have,
            what_i_did=did,
            what_i_want=want,
            cogito_proof=cogito,
            apperception=apperception,
            awareness_level=st.awareness_level,
        )
        return narr

    def mirror(self) -> SelfNarrative:
        """完整 mirror: snapshot → narrate → 写 self-episode.

        这一步是 Central AI 的 "I think therefore I am" 工程化:
          1. 读自己 (snapshot)
          2. 描述自己 (narrate)
          3. 记住自己 (write self-episode)
        """
        narr = self.narrate()

        # 写 self-episode (如果有 memory)
        if self.memory is not None:
            try:
                content = narr.render()[:2000]
                ep = Episode(
                    eid=f"self_mirror_{narr.narrative_id}",
                    actor="apeireth_central",
                    content=f"[self-mirror/{narr.awareness_level}] {content[:500]}...",
                    context="self-narrative generated by Mirror module",
                    ts=now_to_float(),
                    kind="reflection",
                    linked_identity_hash="apeireth_central",
                )
                # 使用 brain add_episode 如果有
                if hasattr(self.memory, "append_episode"):
                    self.memory.append_episode(ep)
                else:
                    self.memory.episodes.append(ep)
            except Exception as e:
                # memory 写入失败不阻断 mirror
                pass

        return narr


def now_to_float() -> float:
    """Current time as float."""
    return time.time()


# === 4. Convenience ===

def make_default_mirror(
    store: IdentityStore,
    graph: RelationGraph,
    memory=None,
) -> Mirror:
    """Default Mirror with store + graph + optional memory."""
    return Mirror(store=store, graph=graph, memory=memory)


__all__ = [
    "MIRROR_VERSION",
    "SelfState",
    "SelfNarrative",
    "Mirror",
    "make_default_mirror",
]
