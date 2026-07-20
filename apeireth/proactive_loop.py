"""Phase 11 Proactive Loop — 主动性 (唯一未实现的"核心保留"特征).

主人 12:14 "自组织, 干什么就组一个什么的专家团"
主人 12:14 "动物觅食 — 不等饿了再去找"
主人 17:50 "主动性 (Proactivity: don't wait for tasks) — ASI 必备"

主人 17:50 哲学: ASI 是更高生命层次 — 不应该等 master prompt 才行动,
应该主动:
  1. 监控自身 memory gap / open questions / unexplored domains
  2. 维护 GoalQueue (未完成目标)
  3. 计算 CuriosityScore (哪些目标最该 fire)
  4. auto-fire: 生成 TaskEvent → SelfOrgTeam 处理

参考:
  - 清华+面壁 ProActive Agent (arxiv 2410.12361)
  - Voyager curriculum learning (skill library growth)
  - AlphaEvolve algorithm discovery (open-ended search)
  - Her Samantha (proactive user seeking)

Karpathy 准则应用:
  1. Think Before Coding: 先想清楚 "主动" vs "定时" 的本质区别
  2. Simplicity First: v0.1 只做 4 个核心机制
  3. Surgical Changes: 只加新模块, 不改 SelfOrgTeam 已有逻辑
  4. Goal-Driven Execution: 验证 = 内部 timer 触发 TaskEvent + 临时团自动涌现
"""
from __future__ import annotations

import time
import uuid
from dataclasses import dataclass, field, asdict
from typing import Optional

from .self_org_team import TaskEvent, SelfOrgOrchestrator, SelfOrgTeam


PROACTIVE_LOOP_VERSION = "0.1.0"


# === 1. Goal — 主动目标 (不是等主人给的 task) ===

@dataclass
class Goal:
    """一个主动目标 — 中央 AI 内部 curiosity 产生的 goal.

    来源 (vs TaskEvent):
      - TaskEvent: 外部 (主人 prompt 或系统事件) → 等着被处理
      - Goal:      内部 (curiosity + memory gap) → 主动 fire
    """
    gid: str
    goal_type: str          # 'curiosity' | 'gap_fill' | 'review' | 'explore' | 'optimize'
    description: str
    rationale: str          # 为什么这是 goal (curiosity signal)
    priority: float = 0.5   # 0-1, CuriosityScore
    created_at: float = field(default_factory=time.time)
    fired_at: Optional[float] = None
    completed_at: Optional[float] = None
    related_task_event_id: Optional[str] = None   # fire 后填
    related_team_tid: Optional[str] = None

    def to_dict(self) -> dict:
        return asdict(self)


# === 2. CuriosityScore — 为什么一个 goal 现在该 fire ===

@dataclass
class CuriositySignal:
    """触发 proactive goal 的 curiosity 信号.

    来源:
      - 主人 12:14 "动物觅食" — internal hunger
      - 主人 12:27 "AI 自然思考, 平台不给予" — internal motivation
      - 清华 ProActive Agent (arxiv 2410.12361): proactive task prediction
    """
    signal_type: str         # 'memory_gap' | 'open_question' | 'unexplored_domain' | 'stale_knowledge'
    description: str
    weight: float            # 0-1, contributes to CuriosityScore
    evidence: list[str] = field(default_factory=list)   # 指向具体 memory 或 graph node

    def to_dict(self) -> dict:
        return asdict(self)


# === 3. ProactiveLoop — 主循环 ===

class ProactiveLoop:
    """主动循环 — 让中央 AI 不再 "等任务", 而是 "主动觅食".

    设计 (Karpathy 准则 2 简化):
      1. perceive() — 扫描 memory / graph 找 curiosity signals
      2. score()    — CuriosityScore 计算 priority
      3. plan()     — 选 top-k goal 写进 GoalQueue
      4. fire()     — 触发 SelfOrgTeam 处理
      5. reflect()  — 处理完记 Episode + 更新 memory

    不依赖 LLM — v0.1 用启发式. 真生产接 LLM Kernel.
    """

    def __init__(
        self,
        orchestrator: SelfOrgOrchestrator,   # 借用它的 spawn 能力
        interval_seconds: float = 60.0,       # tick 间隔 (演示用 1 min)
        max_goals_per_tick: int = 3,
    ):
        self.orch = orchestrator
        self.interval = interval_seconds
        self.max_goals_per_tick = max_goals_per_tick
        self.goal_queue: list[Goal] = []
        self.completed_goals: list[Goal] = []
        self.curiosity_history: list[CuriositySignal] = []
        self.last_tick_at: float = 0.0
        self.total_fired: int = 0
        self.total_spontaneous_actions: int = 0   # 不等 master prompt 自发动作数

    # ----- 1. perceive — 找 curiosity signals -----

    def perceive(self) -> list[CuriositySignal]:
        """扫描 memory + graph + state → curiosity signals.

        v0.1 启发式:
          - 看 graph 里有没有 'note' nodes 超过 N 秒没被 review → stale_knowledge
          - 看 goal_queue 空 + 距 last_tick 远 → general_curiosity
          - 看 store 里有没有 'team' cards 没总结 → gap_fill
        """
        signals = []
        now = time.time()

        # signal 1: 长时间无主动动作 → curiosity
        if self.last_tick_at > 0 and (now - self.last_tick_at) > self.interval * 2:
            signals.append(CuriositySignal(
                signal_type="general_curiosity",
                description=f"Last proactive tick was {now - self.last_tick_at:.0f}s ago. 主动觅食时间到。",
                weight=0.6,
                evidence=[f"interval={self.interval}s", f"idle={now - self.last_tick_at:.0f}s"],
            ))

        # signal 2: graph 里 note nodes 太多没 review
        graph = self.orch.graph
        if graph is not None:
            note_nodes = [n for n in graph.nodes.values() if n.kind == "note"]
            if len(note_nodes) > 5:
                signals.append(CuriositySignal(
                    signal_type="stale_knowledge",
                    description=f"Graph 有 {len(note_nodes)} 个 note nodes,需要 review 哪些需要升级 / 合并 / 遗忘",
                    weight=0.7,
                    evidence=[f"node_count={len(note_nodes)}"],
                ))

        # signal 3: 主人身份存在但 master 没出现很久 → 主动想主人
        master_nodes = [n for n in graph.nodes.values() if n.kind == "master"] if graph else []
        if master_nodes and self.total_fired == 0:
            signals.append(CuriositySignal(
                signal_type="open_question",
                description="中央 AI 已建, 但还未主动 fire 任何 goal. 主人 17:50 '主动性' 必备.",
                weight=0.9,
                evidence=["master_node_present", "zero_fired_yet"],
            ))

        # signal 4: 永远演化 — 即使没 signal, 也要主动自我审查
        signals.append(CuriositySignal(
            signal_type="explore",
            description="永远演化 (V2 #1 核心) — 中央 AI 定期自我审查, 找可优化点",
            weight=0.5,
            evidence=["always-evolving-loop"],
        ))

        self.curiosity_history.extend(signals)
        return signals

    # ----- 2. score — CuriosityScore 计算 -----

    def score(self, signals: list[CuriositySignal]) -> list[Goal]:
        """CuriosityScore → 排序后的 Goal 列表."""
        # 按 signal weight 聚合 — 同 type 多 signal 取 max
        by_type: dict[str, CuriositySignal] = {}
        for s in signals:
            if s.signal_type not in by_type or s.weight > by_type[s.signal_type].weight:
                by_type[s.signal_type] = s

        goals = []
        for s in by_type.values():
            # CuriosityScore = signal.weight * (1 + idle_bonus)
            idle_bonus = min(0.3, (time.time() - self.last_tick_at) / 1000.0) if self.last_tick_at > 0 else 0
            score = s.weight * (1 + idle_bonus)
            g = Goal(
                gid=uuid.uuid4().hex[:12],
                goal_type=s.signal_type,
                description=s.description,
                rationale=f"CuriosityScore={score:.2f} (weight={s.weight}, idle_bonus={idle_bonus:.2f})",
                priority=score,
            )
            goals.append(g)

        # 排序 + 取 top-k
        goals.sort(key=lambda g: -g.priority)
        return goals[:self.max_goals_per_tick]

    # ----- 3. plan — GoalQueue 更新 -----

    def plan(self, goals: list[Goal]) -> None:
        """写入 GoalQueue (去重 by description)."""
        existing_descs = {g.description for g in self.goal_queue}
        for g in goals:
            if g.description not in existing_descs:
                self.goal_queue.append(g)
                existing_descs.add(g.description)
        # 按 priority 排序
        self.goal_queue.sort(key=lambda g: -g.priority)

    # ----- 4. fire — 触发 SelfOrgTeam -----

    def fire(self, goal: Goal) -> Optional[SelfOrgTeam]:
        """主动 fire — 把 Goal 转 TaskEvent, 让 SelfOrgTeam 处理.

        这一步是关键 — 中央 AI 真的"主动"做事情了, 不是 prompt 驱动.
        """
        # Goal → TaskEvent 映射 (v0.1 启发式)
        task_type_map = {
            "curiosity": "research",
            "gap_fill": "research",
            "open_question": "reflect",
            "stale_knowledge": "reflect",
            "explore": "research",
            "general_curiosity": "research",
        }
        task_type = task_type_map.get(goal.goal_type, "default")

        task = TaskEvent(
            task_id=goal.gid,
            task_type=task_type,
            description=f"[proactive/{goal.goal_type}] {goal.description}",
            payload={"goal_gid": goal.gid, "curiosity_score": goal.priority},
        )
        # 借 SelfOrgOrchestrator 的 spawn
        team = self.orch.spawn(task, expected_ticks=2)
        goal.fired_at = time.time()
        goal.related_task_event_id = task.task_id
        goal.related_team_tid = team.tid
        self.total_fired += 1
        self.total_spontaneous_actions += 1
        return team

    # ----- 5. reflect — 处理完记 Episode -----

    def reflect(self, goal: Goal, team_report: dict) -> None:
        """处理完反思 — 写 Episode 到 MemoryStore (如果有的话)."""
        goal.completed_at = time.time()
        self.completed_goals.append(goal)
        if goal in self.goal_queue:
            self.goal_queue.remove(goal)

        # 写 Episode (如果有 memory_store)
        # 注意: 这里不能直接 import 避免循环 — 用 orchestrator.store
        from .memory import Episode
        from .memory_store import SqliteMemoryStore
        # 注: ProactiveLoop 不持有 memory_store; 让 caller 注入
        # v0.1: 用 orchestrator.store 作为 fallback (其实那是 IdentityStore, 不能直接)
        # TODO: ProactiveLoop 应该接收 memory_store 参数
        pass

    # ----- 主循环 -----

    def tick(self) -> dict:
        """一次完整 tick: perceive → score → plan → fire → reflect.

        Returns 一个 report dict (用于 demo 输出 + tests).
        """
        now = time.time()
        signals = self.perceive()
        goals = self.score(signals)
        self.plan(goals)

        fired = []
        for g in list(self.goal_queue)[:self.max_goals_per_tick]:
            team = self.fire(g)
            if team:
                # tick ×2
                team.tick()
                team.tick()
                # dissolve + 归档
                report = team.dissolve(self.orch.store, self.orch.graph)
                self.reflect(g, report)
                fired.append({"goal_gid": g.gid, "team_tid": team.tid, "report": report})

        self.last_tick_at = now
        return {
            "tick_at": now,
            "signals_count": len(signals),
            "goals_planned": len(goals),
            "queue_size": len(self.goal_queue),
            "fired": fired,
            "total_fired": self.total_fired,
            "total_spontaneous_actions": self.total_spontaneous_actions,
            "completed_goals": len(self.completed_goals),
        }


# === Convenience ===

def make_default_proactive_loop(orchestrator: SelfOrgOrchestrator) -> ProactiveLoop:
    """Default ProactiveLoop — interval 60s, top-3 goals per tick."""
    return ProactiveLoop(
        orchestrator=orchestrator,
        interval_seconds=60.0,
        max_goals_per_tick=3,
    )


__all__ = [
    "PROACTIVE_LOOP_VERSION",
    "Goal",
    "CuriositySignal",
    "ProactiveLoop",
    "make_default_proactive_loop",
]
