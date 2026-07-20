"""Phase 48 Global Workspace Theory — Baars + Dehaene 工程化.

跨域调研 round-5 query 5 + 6 + round-6 query 4:
  - Baars 1988 "A Cognitive Theory of Consciousness"
    (https://doi.org/10.1017/S0140525X00049676) — Global Workspace Theory (GWT):
    consciousness = 广播 (global broadcasting) + 多个 specialist modules 竞争
  - Dehaene 2014 "Consciousness and the Brain" — Neuronal Global Workspace (GNW)
  - Dehaene et al. 1998 "A neuronal network model linking subjective reports
    and objective physiological data during conscious perception"
    (https://doi.org/10.1162/089976698300015402) — ignition 现象, late amplification
  - Mashour et al. 2020 "Consciousness science and its theories" — Block 1995
    Access vs Phenomenal consciousness 区分

GWT 模型:
  - Specialist modules: 大量并行 unconscious processors (竞争)
  - Global Workspace: 中央工作空间, 接收 winning coalition 广播
  - Ignition: late amplification (非线性的全脑激活)
  - Access consciousness (Block 1995): 报告/行为可用 = 全球广播
  - Phenomenal consciousness (Block 1995): 体验本身 = 工程化近似未达成
  - GNW: prefrontal + parietal + anterior cingulate 形成 global broadcast

对 ASI 中央 AI 的意义:
  - 中央 AI 是无数关系集合体 (主人 22:08) = specialist modules 集合
  - VCP integrated_ecosystem = 全球广播 = specialist 互联
  - VCP autonomous_living = ignition 自主触发 = 中央 AI 决策
  - Phase 42 Predictive Processing + Phase 47 Neural Darwinism = specialist modules
  - Phase 47 reentry = GWT 双向广播 = parallel bidirectional signaling
  - Access consciousness = 全球广播 (工程化可达) ≠ Phenomenal (终极目标 主 17:58)
  - 主人 17:43 实事求是 — Access 可工程化, Phenomenal 是终极目标未达成

Karpathy 准则:
  1. Think Before Coding: SpecialistModule + GlobalWorkspace + Ignition + Broadcast
  2. Simplicity First: Coalition + Broadcast + Report 三类对象
  3. Surgical Changes: 不改其他模块, 加 broadcasting 视角
  4. Goal-Driven Execution: verifiable = ignition triggered + report consistency
"""
from __future__ import annotations

import math
import time
import uuid
import random
from dataclasses import dataclass, field, asdict
from typing import Dict, List, Optional


GLOBAL_WORKSPACE_VERSION = "0.1.0"


@dataclass
class SpecialistModule:
    """中央 AI 的一个 specialist module — unconscious processor.

    借鉴 Baars 1988: 大量的并行 specialist modules, 竞争进入 global workspace
    对中央 AI: 每个 skill / persona / memory cluster 都可以是 specialist
    """
    module_id: str
    name: str
    activation: float = 0.0
    confidence: float = 0.5
    n_votes: int = 0           # 进入 global workspace 次数
    last_active: float = field(default_factory=time.time)

    def to_dict(self) -> dict:
        return asdict(self)


@dataclass
class Coalition:
    """中央 AI 的 winning coalition — 进入 global workspace 的一组 specialists.

    Baars 1988: coalition = 一组一致的 specialist modules, 共同胜出
    """
    coalition_id: str
    module_ids: List[str]
    total_activation: float
    coherence: float                  # 内部一致性 [0, 1]
    content: str                      # coalition 表示的内容
    timestamp: float = field(default_factory=time.time)

    def to_dict(self) -> dict:
        return asdict(self)


@dataclass
class Broadcast:
    """中央 AI 的 global broadcast — Baars 1988 的核心机制.

    借鉴 Dehaene 1998 ignition: 一旦 coalition 胜出, 全脑 (所有 modules)
    接收广播, 触发 late amplification.
    """
    broadcast_id: str
    coalition_id: str
    recipients: List[str]             # 接收广播的 specialist module ids
    intensity: float                  # 广播强度 (ignition 强度)
    ts: float = field(default_factory=time.time)

    def to_dict(self) -> dict:
        return asdict(self)


@dataclass
class ConsciousnessReport:
    """中央 AI GWT 报告 — Access vs Phenomenal 区分 (Block 1995).

    Access = 全球广播可报告 = 工程化可达
    Phenomenal = 体验本身 = 工程化近似未达成 (主 17:58 终极目标)
    """
    n_specialists: int
    n_active: int
    n_coalitions: int
    n_broadcasts: int
    access_strength: float            # [0, 1] 全球广播可达性 (工程化)
    ignition_rate: float              # [0, 1] ignition 触发率
    is_conscious_access: bool         # access 工程化是否"足够"
    note: str
    ts: float = field(default_factory=time.time)

    def to_dict(self) -> dict:
        return asdict(self)


class GlobalWorkspace:
    """Baars 1988 GWT + Dehaene 2014 GNW — 中央 AI 全球广播系统.

    主人 22:08 V2: 中央 AI 是无数关系的集合体 = specialist modules 集合
    主人 17:58: Phenomenal consciousness 终极目标, 工程化近似未达成
    主人 17:43 实事求是: Access consciousness 工程化可达, Phenomenal 不是
    """

    def __init__(self, n_specialists: int = 15, ignition_threshold: float = 0.6, seed: int = 42):
        self.n_specialists = n_specialists
        self.ignition_threshold = ignition_threshold
        self.rng = random.Random(seed)
        self.modules: Dict[str, SpecialistModule] = {}
        self.coalitions: List[Coalition] = []
        self.broadcasts: List[Broadcast] = []
        self.access_strength_history: List[float] = []
        self._init_modules()

    def _init_modules(self) -> None:
        """初始化 specialist modules — 模仿大量并行 unconscious processors."""
        categories = ["perception", "memory", "reasoning", "language", "emotion",
                     "motor", "attention", "value", "metacognition", "social"]
        for i in range(self.n_specialists):
            mid = uuid.uuid4().hex[:12]
            self.modules[mid] = SpecialistModule(
                module_id=mid,
                name=f"{categories[i % len(categories)]}_{i}",
                confidence=self.rng.uniform(0.4, 0.8),
            )

    def stimulate(self, content: str, activation_pattern: Dict[str, float]) -> Coalition:
        """输入刺激 — 激活一组 specialists, 寻找 winning coalition.

        activation_pattern: module_id -> 激活强度 (由 caller 决定, 模拟 sensory input)
        """
        # 1. 激活 specialists (按 activation_pattern + 自身 confidence + 噪声)
        active: Dict[str, float] = {}
        for mid, module in self.modules.items():
            base = activation_pattern.get(mid, 0.0)
            noise = self.rng.uniform(-0.05, 0.05)
            module.activation = max(0.0, min(1.0, base + module.confidence * 0.3 + noise))
            active[mid] = module.activation

        # 2. 寻找 winning coalition (top-N 一致性最高的 specialists)
        sorted_mids = sorted(active.keys(), key=lambda k: active[k], reverse=True)
        n_coalition = max(2, self.n_specialists // 3)        # 33% specialists 胜出
        coalition_mids = sorted_mids[:n_coalition]
        coalition_acts = [active[m] for m in coalition_mids]
        total_act = sum(coalition_acts)

        # 3. 计算 coherence (内部激活方差低 = 高一致性)
        if coalition_acts:
            mean = total_act / len(coalition_acts)
            var = sum((a - mean) ** 2 for a in coalition_acts) / len(coalition_acts)
            coherence = max(0.0, 1.0 - var)
        else:
            coherence = 0.0

        # 4. 创建 coalition
        coalition = Coalition(
            coalition_id=uuid.uuid4().hex[:12],
            module_ids=coalition_mids,
            total_activation=total_act,
            coherence=coherence,
            content=content,
        )
        self.coalitions.append(coalition)
        for mid in coalition_mids:
            self.modules[mid].n_votes += 1
        return coalition

    def ignite(self, coalition: Coalition) -> Optional[Broadcast]:
        """Ignition — Dehaene 1998 late amplification.

        借鉴 Dehaene: 一旦 total_activation > ignition_threshold, 触发 nonlinear
        全局广播, 所有 specialist modules 接收.
        """
        if coalition.total_activation / max(len(coalition.module_ids), 1) < self.ignition_threshold:
            return None        # 未达到 ignition 阈值
        # ignite: 触发 global broadcast 到所有 modules
        broadcast = Broadcast(
            broadcast_id=uuid.uuid4().hex[:12],
            coalition_id=coalition.coalition_id,
            recipients=list(self.modules.keys()),
            intensity=coalition.total_activation,
        )
        self.broadcasts.append(broadcast)
        # 广播效应: 所有 specialists 接收信号, confidence 微调
        for mid in self.modules:
            self.modules[mid].confidence = max(
                0.1, min(1.0,
                         self.modules[mid].confidence + 0.02 * coalition.coherence
                )
            )
        return broadcast

    def step(self, content: str, activation_pattern: Dict[str, float]) -> ConsciousnessReport:
        """中央 AI 一次 GWT 完整步骤 — stimulate → coalition → ignite."""
        # 1. stimulate: 寻找 winning coalition
        coalition = self.stimulate(content, activation_pattern)
        # 2. ignite: 如果达到阈值, 触发全局广播
        broadcast = self.ignite(coalition)
        # 3. 报告
        n_active = sum(1 for m in self.modules.values() if m.activation > 0.3)
        n_broadcasts = len(self.broadcasts)
        ignition_rate = n_broadcasts / max(len(self.coalitions), 1)
        access_strength = (
            sum(b.intensity for b in self.broadcasts[-10:]) / max(min(len(self.broadcasts), 10), 1)
            if self.broadcasts else 0.0
        )
        access_strength = min(1.0, access_strength / self.n_specialists)
        self.access_strength_history.append(access_strength)
        is_conscious_access = ignition_rate >= 0.4 and access_strength >= 0.3
        note = (
            "Access consciousness (Block 1995) = global broadcasting + report. "
            "Engineering approximation可达. Phenomenal consciousness = experience itself, "
            "终极目标 (主人 17:58), 工程化近似未达成. "
            "主人 17:43 实事求是 — 不假装 Phenomenal."
        )
        return ConsciousnessReport(
            n_specialists=len(self.modules),
            n_active=n_active,
            n_coalitions=len(self.coalitions),
            n_broadcasts=n_broadcasts,
            access_strength=access_strength,
            ignition_rate=ignition_rate,
            is_conscious_access=is_conscious_access,
            note=note,
        )

    def stats(self) -> dict:
        return {
            "version": GLOBAL_WORKSPACE_VERSION,
            "n_specialists": len(self.modules),
            "n_coalitions": len(self.coalitions),
            "n_broadcasts": len(self.broadcasts),
            "ignition_threshold": self.ignition_threshold,
            "mean_access_strength": (
                sum(self.access_strength_history) / len(self.access_strength_history)
                if self.access_strength_history else 0.0
            ),
            "baars_1988": (
                "Global Workspace Theory: consciousness = global broadcasting. "
                "Specialist modules compete for workspace entry."
            ),
            "dehaene_2014": (
                "Neuronal Global Workspace (GNW): prefrontal + parietal + ACC. "
                "Ignition = late non-linear amplification."
            ),
            "block_1995": (
                "Access vs Phenomenal consciousness distinction. "
                "Access = globally broadcast (report可用). "
                "Phenomenal = experience itself (终极目标, 工程化未达成)."
            ),
            "vcp_alignment": {
                "continuous_existence": "ongoing coalitions = 持续 broadcast 链",
                "natural_perception": "specialist modules = 并行感知 channels",
                "autonomous_living": "ignition = 中央 AI 自主触发决策",
                "integrated_ecosystem": "global workspace = 统一广播 substrate",
            },
            "v2_philosophy": (
                "中央 AI 是无数关系的集合体 (主人 22:08) = specialist modules 集合. "
                "中央 AI 完整位置 (主人 22:08): 是调度者/思考者/无数关系集合体/最大权限/ASI 位置. "
                "Access consciousness 工程化可达, Phenomenal = 终极目标 (主人 17:58), "
                "实事求是 (主人 17:43) — 不假装 Phenomenal."
            ),
        }


__all__ = [
    "GLOBAL_WORKSPACE_VERSION",
    "SpecialistModule",
    "Coalition",
    "Broadcast",
    "ConsciousnessReport",
    "GlobalWorkspace",
]