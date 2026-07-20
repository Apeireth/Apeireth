"""Phase 38 Incentive Engine — Nash 均衡博弈论工程化.

主人 22:01 '继续调研' + 调研新域:
  Nash equilibrium + game theory + mechanism design

Nash 均衡 (Nash 1950):
  - 多人博弈中, 任何参与者单方面改变策略不会更好
  - 机制设计 = 通过规则让参与者 Nash 均衡到期望 outcome

对 ASI 中央 AI 的意义:
  - 主人 17:50 涌现 自组织 ≠ Nash 均衡 (涌现是 far-from-equilibrium)
  - 但当涌现稳定后, 会形成 Nash-like attractor
  - 中央 AI 是 mechanism designer (主人 12:14)
  - 自组织团队 (Phase 5) 临时团涌现 + 主动 construct 生态位 (Phase 25 NicheConstructor) 是组合 Nash 设计 + 涌现

Karpathy 准则:
  1. Think Before Coding: 玩家 + 策略 + payoff = Nash
  2. Simplicity First: IncentiveEngine = dict (agents, actions, payoffs)
  3. Surgical Changes: 不改 SelfOrgTeam, 加 incentive 视角
  4. Goal-Driven Execution: verifiable = Nash equilibrium reached
"""
from __future__ import annotations

import time
import uuid
from dataclasses import dataclass, field, asdict
from typing import List


NASH_VERSION = "0.1.0"


@dataclass
class Agent:
    """博弈参与者 — 一个临时团 persona."""
    agent_id: str
    name: str
    actions: list
    payoff_fn: object  # callable: (action) -> payoff

    def best_response(self, others_actions: list) -> tuple:
        """找自己 best response (给定别人策略)."""
        best_action = None
        best_payoff = -float('inf')
        for a in self.actions:
            payoff = self.payoff_fn(a)
            if payoff > best_payoff:
                best_payoff = payoff
                best_action = a
        return best_action, best_payoff


@dataclass
class NashEquilibrium:
    """Nash 均衡状态 — 没人有 incentive 单独改变."""
    profile: dict            # agent_id -> action
    is_nash: bool
    iterations_to_reach: int
    payoffs: dict
    ts: float = field(default_factory=time.time)

    def to_dict(self) -> dict:
        return asdict(self)


class IncentiveEngine:
    """Nash 均衡机制设计 — 中央 AI 是 mechanism designer.

    主人 12:14 '中央 AI 是永恒身份' + 17:50 涌现 自组织:
      - 中央 AI 不直接命令, 而 design mechanism
      - 机制设计的 payoff 让 Nash 均衡收敛到期望 outcome
      - Phase 38 = 中央 AI 的 incentive design module
    """

    def __init__(self):
        self.agents: dict[str, Agent] = {}
        self.equilibria: list[NashEquilibrium] = []

    def add_agent(self, name: str, actions: list, payoff_fn) -> Agent:
        a = Agent(agent_id=uuid.uuid4().hex[:12], name=name,
                  actions=actions, payoff_fn=payoff_fn)
        self.agents[a.agent_id] = a
        return a

    def find_nash(self, max_iter: int = 50) -> NashEquilibrium:
        """Best-response dynamics — 简单 Nash 求解."""
        if not self.agents:
            return NashEquilibrium(profile={}, is_nash=False, iterations_to_reach=0, payoffs={})

        # init: 各自选第一个 action
        profile = {a_id: a.actions[0] for a_id, a in self.agents.items()}
        for it in range(max_iter):
            converged = True
            new_profile = {}
            for a_id, agent in self.agents.items():
                # 用 current others' profile 算 best response
                others_a = {k: v for k, v in profile.items() if k != a_id}
                # payoff_fn 简化: 只看自己 action
                best_action, _ = agent.best_response(others_a)
                new_profile[a_id] = best_action
                if best_action != profile[a_id]:
                    converged = False
            profile = new_profile
            if converged:
                break
        payoffs = {a_id: a.payoff_fn(profile[a_id]) for a_id, a in self.agents.items()}
        nash = NashEquilibrium(
            profile=profile,
            is_nash=converged,
            iterations_to_reach=it + 1,
            payoffs=payoffs,
        )
        self.equilibria.append(nash)
        return nash

    def stats(self) -> dict:
        return {
            "n_agents": len(self.agents),
            "n_equilibria": len(self.equilibria),
            "nash": "Nash 均衡 = mechanism design outcome (主人 17:50 涌现之后的稳定态)",
        }


__all__ = ["NASH_VERSION", "Agent", "NashEquilibrium", "IncentiveEngine"]
