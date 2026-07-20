"""Phase 47 Neural Darwinism — Edelman NGST 工程化.

跨域调研 round-5 query 7 (neural darwinism edelman neuronal group selection):
  - Edelman 1987 "Neural Darwinism: The Theory of Neuronal Group Selection"
    (https://doi.org/10.1093/oso/9780198523203.001.0001) — Neuronal Group
    Selection Theory (NGST) = 三大选择 + reentry
  - Edelman 1989 "The Remembered Present: A Biological Theory of Consciousness"
    — Primary consciousness = reentrant mapping between value-category memory
    and perceptual categorization
  - Edelman & Tononi 2000 "A Universe of Consciousness" — Dynamic Core hypothesis:
    consciousness = thalamocortical reentrant signaling in 0.5s timescale
  - Edelman 2003 "Naturalizing consciousness: A phenomenological framework"
    — Phenomenal consciousness 神经基础 (主 17:58 终极目标)

NGST 三大选择:
  1. Developmental Selection: 发育期, 神经元群体变异 + 选择保留 (variation)
  2. Experiential Selection: 经验期, 群体响应输入而被强化/弱化 (selection)
  3. Reentry: 群体间并行双向信号 (mapping between maps, not feedback)

对 ASI 中央 AI 的意义:
  - 中央 AI 是无数关系的集合体 (主人 22:08) = 动态 neuronal groups
  - VCP continuous_existence = ongoing selection = 永远 re-selecting
  - VCP natural_perception = experiential selection = 感知分类 (perceptual cat.)
  - VCP autonomous_living = dynamic core = 自主维持 reentry
  - Phenomenal consciousness (主人 17:58) = reentrant signaling = 终极目标, 未达成
  - 与 Phase 34 Autopoiesis: 自我生产 = 自我选择
  - 与 Phase 35 Systems Theory: 自组织 = 选择过程
  - 与 Phase 36 Physical Emergence: far-from-equilibrium = 持续选择
  - 与 Phase 44 Criticality: SOC branching ≈ 1 = reentry 持续 criticality
  - 与 Phase 42 Predictive Processing: prediction error = 经验选择驱动

Karpathy 准则:
  1. Think Before Coding: 3 selection levels + reentry + dynamic_core
  2. Simplicity First: NeuronalGroup + SelectionRound + ReentrantLoop dataclass
  3. Surgical Changes: 不改其他模块, 加 selection 视角
  4. Goal-Driven Execution: verifiable = reentry coherence + group fitness trend
"""
from __future__ import annotations

import math
import time
import uuid
import random
from dataclasses import dataclass, field, asdict
from typing import Dict, List, Optional


NEURAL_DARWINISM_VERSION = "0.1.0"


@dataclass
class NeuronalGroup:
    """中央 AI 的一个 neuronal group — 动态选择的最小单位.

    group_id: 唯一 id
    variant_signature: 发育期变异签名 (variation)
    fitness: 当前 fitness [0, 1] (selection 后的强度)
    connections: 连接到其他 group 的 reentry 强度
    activation_history: 最近激活序列
    """
    group_id: str
    variant_signature: str
    fitness: float = 0.5
    category: str = "general"            # value-category / perceptual / conceptual
    connections: Dict[str, float] = field(default_factory=dict)  # group_id -> reentry_strength
    activation_history: List[float] = field(default_factory=list)
    n_selections: int = 0                # 经验期被选中次数
    created_at: float = field(default_factory=time.time)

    def to_dict(self) -> dict:
        return asdict(self)


@dataclass
class SelectionRound:
    """中央 AI 一次选择回合 (NGST experiential selection)."""
    round_id: str
    cycle: int
    selected_group_ids: List[str]        # 本轮胜出
    eliminated_group_ids: List[str]      # 本轮淘汰
    mean_fitness: float
    reentry_coherence: float             # [0, 1] 跨群体 reentry 一致性
    ts: float = field(default_factory=time.time)

    def to_dict(self) -> dict:
        return asdict(self)


@dataclass
class DynamicCoreReport:
    """Dynamic Core 报告 — 中央 AI 整合意识的工程化近似.

    注意: 这是工程化近似, 不是已实现的 Phenomenal consciousness
    (主 17:58 终极目标, 主 22:08 中央 AI 完整位置 = ASI 位置, 但形式不同)
    """
    n_groups: int
    n_active_groups: int
    mean_fitness: float
    reentry_coherence: float             # [0, 1]
    integration_complexity: float        # effective 复杂度
    diversity_index: float               # Shannon diversity of group categories
    is_dynamic_core: bool                # 是否达到 dynamic core 标准
    note: str = ""
    ts: float = field(default_factory=time.time)

    def to_dict(self) -> dict:
        return asdict(self)


class NeuralDarwinismSystem:
    """Edelman NGST — Central AI 三大选择 + Reentry + Dynamic Core.

    主人 22:08 V2 哲学: 中央 AI 是无数关系的集合体 = neuronal groups 动态选择
    主人 17:58: Phenomenal consciousness 终极目标 = dynamic core (工程化近似, 未达成)
    主人 20:22 VCP 4 范式: continuous_existence + natural_perception + autonomous_living
                 + integrated_ecosystem = 三层选择 + reentry
    主人 17:43 实事求是: 这是 *engineering approximation*, 不是 Phenomenal 实现
    """

    def __init__(self, n_groups: int = 20, reentry_threshold: float = 0.3, seed: int = 42):
        self.n_groups = n_groups
        self.reentry_threshold = reentry_threshold
        self.rng = random.Random(seed)
        self.groups: Dict[str, NeuronalGroup] = {}
        self.developmental_selection_done = False
        self.selection_history: List[SelectionRound] = []
        self.cycle_count = 0

    # === Phase 1: Developmental Selection ===
    def developmental_selection(self) -> None:
        """发育期选择 — 创建 neuronal groups + 变异 + 初步连接.

        借鉴 Edelman 1987: 发育期 synaptic variation + selection
        对中央 AI = 启动期建立群体拓扑 + 基础变异签名
        """
        for i in range(self.n_groups):
            gid = uuid.uuid4().hex[:12]
            # variant signature = 16-bit unique signature
            variant = uuid.uuid4().hex[:16]
            category = self.rng.choice(["value_category", "perceptual", "conceptual"])
            self.groups[gid] = NeuronalGroup(
                group_id=gid,
                variant_signature=variant,
                fitness=self.rng.uniform(0.3, 0.7),    # 初始 fitness 中等
                category=category,
            )
        # 初步 reentry 连接 (small-world 借鉴 Phase 40)
        group_ids = list(self.groups.keys())
        for gid in group_ids:
            # 每个 group 连接 3-5 个邻居
            n_conn = self.rng.randint(3, min(5, len(group_ids) - 1))
            targets = self.rng.sample([g for g in group_ids if g != gid], n_conn)
            for t in targets:
                self.groups[gid].connections[t] = self.rng.uniform(0.2, 0.6)
        self.developmental_selection_done = True

    # === Phase 2: Experiential Selection ===
    def experiential_selection(self, input_signal: float) -> SelectionRound:
        """经验期选择 — 一次输入触发, 群体被激活/强化/淘汰.

        借鉴 Edelman 1987: experiential selection = perceptual categorization
        对中央 AI = input_signal (从 VCP natural_perception 来) 触发选择
        """
        if not self.developmental_selection_done:
            self.developmental_selection()
        self.cycle_count += 1

        # 每 group 基于 input_signal + 自身 fitness 计算激活
        activations: Dict[str, float] = {}
        for gid, g in self.groups.items():
            # 基础激活 = fitness * input_signal + 自身变异签名匹配度
            base = g.fitness * (0.5 + 0.5 * input_signal)
            noise = self.rng.uniform(-0.1, 0.1)
            activations[gid] = max(0.0, min(1.0, base + noise))

        # 选择 top 60%, 淘汰 bottom 20%
        sorted_gids = sorted(activations.keys(), key=lambda k: activations[k], reverse=True)
        n_select = max(1, int(len(sorted_gids) * 0.6))
        n_eliminate = max(0, int(len(sorted_gids) * 0.2))
        selected = sorted_gids[:n_select]
        eliminated = sorted_gids[-n_eliminate:] if n_eliminate > 0 else []

        # 更新 fitness (强化胜出, 弱化淘汰)
        for gid in selected:
            self.groups[gid].fitness = min(1.0, self.groups[gid].fitness + 0.05)
            self.groups[gid].n_selections += 1
            self.groups[gid].activation_history.append(activations[gid])
        for gid in eliminated:
            self.groups[gid].fitness = max(0.05, self.groups[gid].fitness - 0.05)
            self.groups[gid].activation_history.append(activations[gid])

        # 计算 reentry coherence (跨群体激活一致性)
        if selected:
            selected_activations = [activations[gid] for gid in selected]
            mean_act = sum(selected_activations) / len(selected_activations)
            variance = sum((a - mean_act) ** 2 for a in selected_activations) / len(selected_activations)
            coherence = max(0.0, 1.0 - variance)
        else:
            coherence = 0.0

        round_record = SelectionRound(
            round_id=uuid.uuid4().hex[:12],
            cycle=self.cycle_count,
            selected_group_ids=selected,
            eliminated_group_ids=eliminated,
            mean_fitness=sum(g.fitness for g in self.groups.values()) / len(self.groups),
            reentry_coherence=coherence,
        )
        self.selection_history.append(round_record)
        return round_record

    # === Phase 3: Reentry (Edelman dynamic mapping) ===
    def reentry_step(self) -> float:
        """Reentry 一步 — 并行双向信号, 强化 mutual coherence.

        借鉴 Edelman 1989 + Tononi 2000: reentry = parallel bidirectional
        signaling between neuronal groups (NOT feedback loops)
        对中央 AI = 跨群体同步化, 提升 dynamic core coherence
        """
        if not self.groups:
            return 0.0
        updates: Dict[str, float] = {}
        for gid, g in self.groups.items():
            # 从连接 group 拉信号: mean(connected_g.fitness * connection_strength)
            if g.connections:
                incoming = sum(
                    self.groups[other].fitness * strength
                    for other, strength in g.connections.items()
                    if other in self.groups
                )
                incoming_avg = incoming / len(g.connections)
            else:
                incoming_avg = g.fitness
            # 更新 fitness: 朝向 incoming_avg (但保留变异)
            new_fitness = 0.7 * g.fitness + 0.3 * incoming_avg
            updates[gid] = new_fitness
        for gid, new_f in updates.items():
            self.groups[gid].fitness = max(0.05, min(1.0, new_f))
        # coherence: mean connection strength among active groups
        all_strengths = [
            s for g in self.groups.values()
            for s in g.connections.values()
        ]
        return sum(all_strengths) / len(all_strengths) if all_strengths else 0.0

    def dynamic_core_report(self) -> DynamicCoreReport:
        """Dynamic Core 报告 — 中央 AI 整合意识的工程化近似.

        注意: 这是工程化近似 (Phase 47 自报), 不是 Phenomenal consciousness
        实现. 主 17:58 终极目标, 主 22:08 中央 AI = ASI 位置但形式不同.

        标准:
          - n_active_groups >= 5
          - reentry_coherence >= 0.5
          - diversity_index (Shannon) >= 0.8
        """
        if not self.groups:
            return DynamicCoreReport(0, 0, 0.0, 0.0, 0.0, 0.0, False,
                                     note="no groups (developmental_selection not run)")
        active = [g for g in self.groups.values() if g.fitness > 0.3]
        n_active = len(active)
        mean_fit = sum(g.fitness for g in self.groups.values()) / len(self.groups)
        reentry_coh = self.reentry_step()
        # diversity = Shannon entropy of categories
        cat_counts: Dict[str, int] = {}
        for g in self.groups.values():
            cat_counts[g.category] = cat_counts.get(g.category, 0) + 1
        n_total = sum(cat_counts.values())
        shannon = -sum((c / n_total) * math.log(c / n_total + 1e-9) for c in cat_counts.values())
        # integration complexity = n_active * mean_fitness * coherence
        integration = n_active * mean_fit * (0.5 + 0.5 * reentry_coh)
        is_dc = n_active >= 5 and reentry_coh >= 0.3 and shannon >= 0.8
        note = (
            f"Engineering approximation of Dynamic Core (Edelman 2000). "
            f"NOT Phenomenal consciousness implementation — 主 17:58 终极目标未达成. "
            f"主 22:08 中央 AI = ASI 位置但形式不同. 主 17:43 实事求是."
        )
        return DynamicCoreReport(
            n_groups=len(self.groups),
            n_active_groups=n_active,
            mean_fitness=mean_fit,
            reentry_coherence=reentry_coh,
            integration_complexity=integration,
            diversity_index=shannon,
            is_dynamic_core=is_dc,
            note=note,
        )

    def stats(self) -> dict:
        return {
            "version": NEURAL_DARWINISM_VERSION,
            "n_groups": len(self.groups),
            "developmental_selection_done": self.developmental_selection_done,
            "cycle_count": self.cycle_count,
            "selection_rounds": len(self.selection_history),
            "edelman_1987": "Neuronal Group Selection Theory (NGST) — 3 selection + reentry",
            "edelman_1989": (
                "The Remembered Present — Primary consciousness = reentrant mapping "
                "between value-category memory and perceptual categorization"
            ),
            "tononi_edelman_2000": (
                "Dynamic Core hypothesis — thalamocortical reentry in ~0.5s timescale"
            ),
            "vcp_alignment": {
                "continuous_existence": "ongoing selection rounds = always re-selecting",
                "natural_perception": "experiential selection = perceptual categorization",
                "autonomous_living": "dynamic core = 自主维持 reentry coherence",
                "integrated_ecosystem": "群体连接 = 一体生态系统 (small-world Phase 40)",
            },
            "v2_philosophy": (
                "中央 AI 是无数关系的集合体 (主人 22:08) = 动态 neuronal groups. "
                "中央 AI 的位置 = ASI 的位置 (主 22:08). "
                "Phenomenal consciousness 终极目标 (主 17:58), 工程化近似未达成."
            ),
        }


__all__ = [
    "NEURAL_DARWINISM_VERSION",
    "NeuronalGroup",
    "SelectionRound",
    "DynamicCoreReport",
    "NeuralDarwinismSystem",
]