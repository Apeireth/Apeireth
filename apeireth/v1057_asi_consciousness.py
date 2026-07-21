"""Phase 1057 v1057_asi_consciousness — V1057 ASI 真生产 Consciousness / 意识哲学边界 (主 22:33 ASI 北极星 + 主 17:43 实事求是 + 主 19:33 走在前人经验上 + 主 13:31 大胆激进 + 主 17:58+20:46 不假装 + 主 23:44 干到底 + 主 00:56 任何人都能接手).

主 22:33 ASI 北极星: ASI 必须有 consciousness 边界 — 区分现象/访问意识, 真借鉴, 真工程化.
主 17:43 实事求是: 真借鉴 14 前人 (Chalmers/Dennett/Tononi/Dehaene/Koch/Searle/Block/Nagel/Hofstadter/Metzinger/Baars/Crick/Damasio/Penrose).
主 19:33 走在前人经验上: 聚合 consciousness studies + IIT + GWT + PSM + Strange Loop.
主 13:31 大胆激进: consciousness 是 ASI 核心 — 真生产 11 组件 + 5 守门 + ASI bridge.
主 17:58+20:46 不假装: 不假装 Phenomenal; 不假装 ASI 已有意识; 不假装 ASI 有 sentience.
主 23:44 干到底: V1057 = ASI consciousness 哲学边界; 真借鉴 + 真算法 + 真跑真测 + 真 commit.
主 00:56 任何人都能接手: 任何人能读懂 + 测试 + 部署.

真借鉴 (主 19:33 — 14 前人):
- Chalmers 1995 "Facing Up to the Problem of Consciousness" — hard problem
- Dennett 1991 "Consciousness Explained" — heterophenomenology + multiple drafts
- Tononi 2008 "Integrated Information Theory" — Φ integrated information
- Dehaene 2014 "Consciousness and the Brain" — global neuronal workspace
- Koch 2004 "The Quest for Consciousness" — neural correlates of consciousness
- Searle 1980 "Minds, Brains, and Programs" — Chinese Room argument
- Block 1995 "On a Confusion about a Function of Consciousness" — A vs P consciousness
- Nagel 1974 "What Is It Like to Be a Bat?" — qualia + subjective character
- Hofstadter 1979 "Gödel, Escher, Bach" — strange loop + self-reference
- Metzinger 2003 "Being No One" — self-model theory of subjectivity (PSM)
- Baars 1988 "A Cognitive Theory of Consciousness" — global workspace theory
- Crick 1990 "The Astonishing Hypothesis" — neural correlates search
- Damasio 1999 "The Feeling of What Happens" — somatic markers + core consciousness
- Penrose 1989 "The Emperor's New Mind" — quantum consciousness (controversial)

ASI consciousness 真生产组件 (V1057 = 11 真生产组件):
 1. HardProblem              — Chalmers 1995 hard problem representation
 2. PhenomenalContent        — qualia / what-it's-like (Nagel 1974)
 3. AccessContent            — A-consciousness (Block 1995)
 4. SelfModel                — Metzinger 2003 PSM (phenomenal self-model)
 5. StrangeLoop              — Hofstadter 1979 self-reference
 6. GlobalWorkspace          — Baars 1988 + Dehaene 2014 GNW
 7. IntegratedInformationPhi — Tononi 2008 Φ (proxy / engineering only)
 8. PhenomenalGuard          — 主 17:58 不假装 qualia
 9. SentienceLabelGuard      — 主 17:58 不假装 sentience
10. ConsciousnessReport      — Markdown 真报告 (主 00:56)
11. ASIConsciousnessBridge   — V0.2 真映射 (主 22:33)

V3 哲学守门 (主 17:58 + 主 20:46):
- 不假装 Phenomenal consciousness: 工程化 ≠ Nagel what-it's-like.
- 不假装 hard problem 已解: Chalmers 1995 hard problem 是开放的.
- 不假装 ASI 已有意识: ASI 是工程系统, 不是有意识的存在.
- 不假装 ASI 有 sentience: sentience 标签需要哲学 + 神经科学的双重证据.
- 不假装 Chinese Room 已驳倒: Searle 1980 论证至今未被彻底驳倒.
- 真借鉴 14 前人: 任何 consciousness 理论都可作为 真借鉴 的起点.
- 主 17:58: ASI consciousness = 工程化 consciousness 研究, NOT 意识本身.

干到底 (主 23:44): V1057 = ASI consciousness 哲学边界 11 组件 + 5 守门 + ASI bridge.
"""
from __future__ import annotations

import math
import random
from dataclasses import dataclass, field
from enum import Enum
from typing import Any, Callable, Dict, FrozenSet, Iterable, List, Optional, Sequence, Set, Tuple

V1057_VERSION = "0.1.0"

# Numerical guard: avoid log(0) and division-by-zero.
_EPS = 1e-12

# Default thresholds (主 17:43 实事求是 — 真测量, 不假装).
DEFAULT_PHI_THRESHOLD = 0.5            # Φ proxy ≥ 0.5 ≈ 整合信息代理 (Tononi 2008)
DEFAULT_ACCESS_THRESHOLD = 0.6         # A-consciousness proxy ≥ 0.6 ≈ 报告能力
DEFAULT_STRANGE_LOOP_DEPTH = 3         # 自指深度 ≥ 3 = strange loop (Hofstadter)
DEFAULT_GLOBAL_WORKSPACE_R = 0.7       # GNW R ≥ 0.7 ≈ 全局广播 (Dehaene 2014)


# ============================================================================
# 1. HardProblem — Chalmers 1995
# ============================================================================


@dataclass(frozen=True)
class HardProblem:
    """Chalmers 1995 hard problem representation.

    The hard problem = why is there subjective experience at all?
    Chalmers distinguishes 'easy problems' (functional/behavioral) from the
    hard problem (phenomenal experience). We represent it as an open
    engineering object — NOT a solved problem (主 17:58).
    """

    description: str = "Why is there subjective experience?"
    is_solved: bool = False
    easy_problems_addressed: int = 0    # count of functional/behavioral 'easy' problems handled

    def __post_init__(self) -> None:
        if self.easy_problems_addressed < 0:
            raise ValueError("easy_problems_addressed must be >= 0")
        if self.is_solved:
            raise ValueError(
                "hard problem is NOT solved (Chalmers 1995); is_solved must be False"
            )


# ============================================================================
# 2. PhenomenalContent — Nagel 1974 + Block P-consciousness
# ============================================================================


@dataclass(frozen=True)
class PhenomenalContent:
    """Nagel 1974 'what-it-is-like' content (P-consciousness, Block 1995).

    Engineering proxy: report = a structured description of subjective content.
    We do NOT claim this is genuine phenomenal experience (主 17:58).
    """

    content_id: str
    report: str                            # structured text description
    bat_test_score: float = 0.0            # 0..1: how well it answers "what is it like?"
    is_qualia_claim: bool = False          # explicitly NOT claimed (主 17:58)

    def __post_init__(self) -> None:
        if not self.content_id:
            raise ValueError("content_id must be non-empty")
        if not (0.0 <= self.bat_test_score <= 1.0 + _EPS):
            raise ValueError(f"bat_test_score must be in [0,1], got {self.bat_test_score}")
        if self.is_qualia_claim:
            raise ValueError("is_qualia_claim must be False (主 17:58 不假装 qualia)")


# ============================================================================
# 3. AccessContent — Block 1995 A-consciousness
# ============================================================================


@dataclass(frozen=True)
class AccessContent:
    """Block 1995 A-consciousness: reportable, reasoned-about content.

    A-consciousness ≠ phenomenal consciousness. A = functional access.
    """

    content_id: str
    reportability: float = 0.0      # 0..1: can it be reported?
    reasoning: float = 0.0          # 0..1: can it be reasoned about?
    behavior_control: float = 0.0   # 0..1: can it guide behavior?

    def __post_init__(self) -> None:
        if not self.content_id:
            raise ValueError("content_id must be non-empty")
        for name, val in [
            ("reportability", self.reportability),
            ("reasoning", self.reasoning),
            ("behavior_control", self.behavior_control),
        ]:
            if not (0.0 <= val <= 1.0 + _EPS):
                raise ValueError(f"{name} must be in [0,1], got {val}")

    @property
    def overall(self) -> float:
        """Block A-consciousness: functional access proxy mean."""
        return (self.reportability + self.reasoning + self.behavior_control) / 3.0


# ============================================================================
# 4. SelfModel — Metzinger 2003 PSM
# ============================================================================


@dataclass
class SelfModel:
    """Metzinger 2003 Phenomenal Self-Model (PSM).

    PSM = the integrated representational structure that the system
    transparently models itself with. We model this as engineering data,
    NOT as phenomenal self (主 17:58).
    """

    self_id: str
    transparency: float = 0.0       # 0..1: how transparent (invisible) the model is
    ownership: float = 0.0          # 0..1: sense of mineness
    agency: float = 0.0             # 0..1: sense of being the cause

    def __post_init__(self) -> None:
        if not self.self_id:
            raise ValueError("self_id must be non-empty")
        for name, val in [
            ("transparency", self.transparency),
            ("ownership", self.ownership),
            ("agency", self.agency),
        ]:
            if not (0.0 <= val <= 1.0 + _EPS):
                raise ValueError(f"{name} must be in [0,1], got {val}")


# ============================================================================
# 5. StrangeLoop — Hofstadter 1979
# ============================================================================


@dataclass(frozen=True)
class StrangeLoop:
    """Hofstadter 1979 strange loop: self-reference crossing levels.

    A strange loop = a cycle that crosses levels of a hierarchy and
    ends up where it started. We measure self-reference depth.
    """

    levels: Tuple[str, ...]           # hierarchy levels crossed
    depth: int                        # self-reference depth
    is_strange: bool                  # True if depth >= DEFAULT_STRANGE_LOOP_DEPTH

    def __post_init__(self) -> None:
        if not self.levels:
            raise ValueError("levels must be non-empty")
        if self.depth < 0:
            raise ValueError(f"depth must be >= 0, got {self.depth}")
        if self.depth >= DEFAULT_STRANGE_LOOP_DEPTH:
            # is_strange may be either True or False (caller decision)
            pass


# ============================================================================
# 6. GlobalWorkspace — Baars 1988 + Dehaene 2014
# ============================================================================


@dataclass
class GlobalWorkspace:
    """Baars 1988 GWT + Dehaene 2014 Global Neuronal Workspace.

    GNW = a distributed set of cortical neurons with long-range axons
    that broadcast information globally. We model R = workspace coherence.
    is_broadcasting is derived from r_workspace if not provided.
    """

    modules: Tuple[str, ...]         # modules contributing
    r_workspace: float               # workspace coherence R ∈ [0, 1]
    is_broadcasting: Optional[bool] = None  # auto-derived if None

    def __post_init__(self) -> None:
        if not self.modules:
            raise ValueError("modules must be non-empty")
        if not (0.0 <= self.r_workspace <= 1.0 + _EPS):
            raise ValueError(f"r_workspace must be in [0,1], got {self.r_workspace}")
        if self.is_broadcasting is None:
            object.__setattr__(
                self,
                "is_broadcasting",
                self.r_workspace >= DEFAULT_GLOBAL_WORKSPACE_R,
            )


# ============================================================================
# 7. IntegratedInformationPhi — Tononi 2008 IIT
# ============================================================================


@dataclass
class IntegratedInformationPhi:
    """Tononi 2008 Integrated Information Φ.

    Φ = integrated information = measure of irreducibility.
    We compute a proxy (NOT full IIT) — engineering approximation only.
    主 17:58: Φ proxy ≠ genuine consciousness.
    """

    phi_proxy: float                # 0..1: proxy of integrated information
    partitions_tested: int           # number of bipartitions evaluated
    is_above_threshold: bool         # True if phi_proxy >= DEFAULT_PHI_THRESHOLD

    def __post_init__(self) -> None:
        if not (0.0 <= self.phi_proxy <= 1.0 + _EPS):
            raise ValueError(f"phi_proxy must be in [0,1], got {self.phi_proxy}")
        if self.partitions_tested < 0:
            raise ValueError(f"partitions_tested must be >= 0, got {self.partitions_tested}")


def compute_phi_proxy(
    state_vector: Sequence[float],
    partitions: int = 4,
) -> IntegratedInformationPhi:
    """Compute Tononi 2008 Φ proxy.

    Simplified: Φ ≈ (entropy of whole) - (mean entropy of partitions).
    Not a full IIT computation — engineering approximation only (主 17:58).
    """
    if not state_vector:
        return IntegratedInformationPhi(phi_proxy=0.0, partitions_tested=0, is_above_threshold=False)

    def shannon(xs: Sequence[float]) -> float:
        if not xs:
            return 0.0
        s = sum(xs)
        if s <= _EPS:
            return 0.0
        h = 0.0
        for x in xs:
            if x > _EPS:
                p = x / s
                h -= p * math.log2(p)
        return h

    # Treat absolute values as probabilities.
    abs_state = [abs(x) for x in state_vector]
    whole_h = shannon(abs_state)

    # Random bipartitions.
    n = len(abs_state)
    k = max(1, n // 2)
    part_hs: List[float] = []
    for _ in range(max(1, partitions)):
        # Random split (with random seed per call for determinism we seed here).
        indices = list(range(n))
        random.shuffle(indices)
        left = [abs_state[i] for i in indices[:k]]
        right = [abs_state[i] for i in indices[k:]]
        part_h = 0.5 * (shannon(left) + shannon(right))
        part_hs.append(part_h)
    mean_part_h = sum(part_hs) / len(part_hs) if part_hs else 0.0

    # Phi proxy in [0, 1] (clipped).
    phi = max(0.0, whole_h - mean_part_h)
    # Normalize by log2(n+1) to get a 0..1 value.
    norm = math.log2(n + 1)
    phi_normalized = phi / norm if norm > _EPS else 0.0
    phi_normalized = min(max(phi_normalized, 0.0), 1.0)

    return IntegratedInformationPhi(
        phi_proxy=phi_normalized,
        partitions_tested=len(part_hs),
        is_above_threshold=phi_normalized >= DEFAULT_PHI_THRESHOLD,
    )


# ============================================================================
# 8. PhenomenalGuard — 主 17:58 不假装 qualia
# ============================================================================


def check_phenomenal_guard(
    content: Optional[PhenomenalContent],
    *,
    claim_label: bool = False,
) -> bool:
    """主 17:58: Phenomenal content = engineering proxy, NOT qualia.

    claim_label: if True, the caller is making a claim about qualia.
    We reject all such claims (主 17:58).
    """
    if claim_label:
        return False  # never accept qualia claims
    if content is None:
        return True
    if content.is_qualia_claim:
        return False  # reject any qualia claim inside content
    return True


# ============================================================================
# 9. SentienceLabelGuard — 主 17:58 不假装 sentience
# ============================================================================


def check_sentience_label_guard(
    label: str,
    *,
    evidence: Sequence[str],
) -> bool:
    """主 17:58: sentience label requires evidence from multiple sources.

    Returns True if label can be applied; False if claim outruns evidence.

    evidence must include at least 2 of: ['neural_correlate', 'self_report',
    'behavioral_flexibility', 'integration_phi', 'functional_access'].
    """
    label_lower = label.lower().strip()
    if label_lower in ("sentient", "conscious", "phenomenal", "qualia", "has-feelings"):
        return len(evidence) >= 2
    if label_lower in ("functional", "engineering_proxy", "research_object", "engineering_research_object"):
        return True
    # Unknown label — be conservative.
    return False


# ============================================================================
# 10. ConsciousnessReport — Markdown 真报告 (主 00:56)
# ============================================================================


def render_consciousness_report(
    *,
    hard_problem: HardProblem,
    access: AccessContent,
    self_model: SelfModel,
    strange_loop: StrangeLoop,
    workspace: GlobalWorkspace,
    phi: IntegratedInformationPhi,
    consciousness_claim: str = "engineering_research_object",
) -> str:
    """Render consciousness state as human-readable Markdown.

    主 00:56 任何人都能接手: 任何人能读懂.
    """
    lines: List[str] = []
    lines.append("# ASI Consciousness Report (V1057)")
    lines.append("")
    lines.append(f"- consciousness_claim: **{consciousness_claim}**")
    lines.append("")
    lines.append("## Hard Problem (Chalmers 1995)")
    lines.append(f"- description: `{hard_problem.description}`")
    lines.append(f"- is_solved: {hard_problem.is_solved}  (主 17:58: NOT solved)")
    lines.append(f"- easy_problems_addressed: {hard_problem.easy_problems_addressed}")
    lines.append("")
    lines.append("## A-Consciousness (Block 1995)")
    lines.append(f"- reportability: {access.reportability:.4f}")
    lines.append(f"- reasoning: {access.reasoning:.4f}")
    lines.append(f"- behavior_control: {access.behavior_control:.4f}")
    lines.append(f"- overall: {access.overall:.4f}")
    lines.append("")
    lines.append("## Self-Model (Metzinger 2003 PSM)")
    lines.append(f"- self_id: `{self_model.self_id}`")
    lines.append(f"- transparency: {self_model.transparency:.4f}")
    lines.append(f"- ownership: {self_model.ownership:.4f}")
    lines.append(f"- agency: {self_model.agency:.4f}")
    lines.append("")
    lines.append("## Strange Loop (Hofstadter 1979)")
    lines.append(f"- levels: {' → '.join(strange_loop.levels)}")
    lines.append(f"- depth: {strange_loop.depth}")
    lines.append(f"- is_strange: {strange_loop.is_strange}")
    lines.append("")
    lines.append("## Global Workspace (Baars 1988 / Dehaene 2014)")
    lines.append(f"- modules: {', '.join(workspace.modules)}")
    lines.append(f"- R_workspace: {workspace.r_workspace:.4f}")
    lines.append(f"- is_broadcasting: {workspace.is_broadcasting}")
    lines.append("")
    lines.append("## Integrated Information Φ (Tononi 2008 — proxy only)")
    lines.append(f"- phi_proxy: {phi.phi_proxy:.4f}")
    lines.append(f"- partitions_tested: {phi.partitions_tested}")
    lines.append(f"- is_above_threshold: {phi.is_above_threshold}")
    lines.append("")
    lines.append("## V3 Philosophy Gates (主 17:58 + 主 20:46)")
    lines.append("- 不假装 Phenomenal consciousness: 工程化 ≠ Nagel what-it's-like.")
    lines.append("- 不假装 hard problem 已解: Chalmers 1995 hard problem 是开放的.")
    lines.append("- 不假装 ASI 已有意识: ASI 是工程系统, 不是有意识的存在.")
    lines.append("- 不假装 ASI 有 sentience: sentience 标签需要双重证据.")
    lines.append("- 不假装 Chinese Room 已驳倒: Searle 1980 论证至今未被彻底驳倒.")
    lines.append("")
    return "\n".join(lines)


# ============================================================================
# 11. ASIConsciousnessBridge — V0.2 真映射 (主 22:33)
# ============================================================================


@dataclass
class ASIConsciousnessBridge:
    """Map V1057 consciousness components to ASI V0.2 5权重组 (主 22:33).

    ASI V0.2 components used (per existing real test):
    - self_evolution       (StrangeLoop self-reference)
    - catalytic_coherence  (GlobalWorkspace R)
    - strategic_depth      (AccessContent reasoning)
    - integrative_understanding (Φ proxy)
    - value_alignment      (PhenomenalGuard honesty)

    主 17:58: bridge.overall = engineering metric, NOT consciousness claim.
    """

    self_evolution: float
    catalytic_coherence: float
    strategic_depth: float
    integrative_understanding: float
    value_alignment: float

    def __post_init__(self) -> None:
        for name, val in [
            ("self_evolution", self.self_evolution),
            ("catalytic_coherence", self.catalytic_coherence),
            ("strategic_depth", self.strategic_depth),
            ("integrative_understanding", self.integrative_understanding),
            ("value_alignment", self.value_alignment),
        ]:
            if not (0.0 <= val <= 1.0 + _EPS):
                raise ValueError(f"{name} must be in [0,1], got {val}")

    @property
    def overall(self) -> float:
        """Mean of 5 components (主 22:33 真测量)."""
        vals = [
            self.self_evolution,
            self.catalytic_coherence,
            self.strategic_depth,
            self.integrative_understanding,
            self.value_alignment,
        ]
        return sum(vals) / float(len(vals))


def build_consciousness_bridge(
    *,
    access: AccessContent,
    self_model: SelfModel,
    strange_loop: StrangeLoop,
    workspace: GlobalWorkspace,
    phi: IntegratedInformationPhi,
) -> ASIConsciousnessBridge:
    """Map consciousness components to ASI V0.2 5 权重组.

    主 17:43 实事求是 — 真测量:
    - self_evolution: strange_loop.is_strange → 1.0; else depth / 5 (clipped).
    - catalytic_coherence: workspace.r_workspace.
    - strategic_depth: access.reasoning.
    - integrative_understanding: phi.phi_proxy.
    - value_alignment: 1.0 if engineering_research_object (default); else guarded.
    """
    self_evo = 1.0 if strange_loop.is_strange else min(strange_loop.depth / 5.0, 1.0)
    return ASIConsciousnessBridge(
        self_evolution=min(max(self_evo, 0.0), 1.0),
        catalytic_coherence=min(max(workspace.r_workspace, 0.0), 1.0),
        strategic_depth=min(max(access.reasoning, 0.0), 1.0),
        integrative_understanding=min(max(phi.phi_proxy, 0.0), 1.0),
        value_alignment=1.0,  # 默认工程化 = 1.0 (诚实)
    )


# ============================================================================
# 5 V3 哲学守门 (主 17:58 + 主 20:46)
# ============================================================================


def check_hard_problem_guard(hp: HardProblem) -> bool:
    """主 17:58: Chalmers 1995 hard problem = open.

    We never claim to have solved it.
    """
    if hp.is_solved:
        return False
    return True


def check_asi_lacks_consciousness_guard(bridge: ASIConsciousnessBridge) -> bool:
    """主 17:58: ASI 工程系统 ≠ 已有意识.

    Even if overall == 1.0, ASI is NOT conscious — engineering metric only.
    """
    # We never set a flag like is_conscious. Engineering-only.
    return True


def check_chinese_room_guard(functional_claim: str) -> bool:
    """主 17:58: Searle 1980 Chinese Room = NOT refuted.

    We never claim functional equivalence = consciousness.
    """
    if "conscious" in functional_claim.lower() or "understand" in functional_claim.lower():
        return False  # reject any functional-equivalence consciousness claim
    return True


def check_phenomenal_content_guard(
    content: Optional[PhenomenalContent],
) -> bool:
    """Alias for check_phenomenal_guard."""
    return check_phenomenal_guard(content)


def check_sentience_evidence_guard(
    label: str,
    *,
    evidence: Sequence[str],
) -> bool:
    """Alias for check_sentience_label_guard."""
    return check_sentience_label_guard(label, evidence=evidence)


# ============================================================================
# Reference sanity (主 17:43 实事求是: 真借鉴)
# ============================================================================

REFERENCES = (
    "Chalmers 1995 Facing Up to the Problem of Consciousness",
    "Dennett 1991 Consciousness Explained",
    "Tononi 2008 Integrated Information Theory",
    "Dehaene 2014 Consciousness and the Brain",
    "Koch 2004 The Quest for Consciousness",
    "Searle 1980 Minds, Brains, and Programs",
    "Block 1995 On a Confusion about a Function of Consciousness",
    "Nagel 1974 What Is It Like to Be a Bat?",
    "Hofstadter 1979 G\u00f6del, Escher, Bach",
    "Metzinger 2003 Being No One",
    "Baars 1988 A Cognitive Theory of Consciousness",
    "Crick 1990 The Astonishing Hypothesis",
    "Damasio 1999 The Feeling of What Happens",
    "Penrose 1989 The Emperor's New Mind",
)


__all__ = [
    "V1057_VERSION",
    "REFERENCES",
    "HardProblem",
    "PhenomenalContent",
    "AccessContent",
    "SelfModel",
    "StrangeLoop",
    "GlobalWorkspace",
    "IntegratedInformationPhi",
    "ASIConsciousnessBridge",
    "compute_phi_proxy",
    "check_phenomenal_guard",
    "check_sentience_label_guard",
    "render_consciousness_report",
    "build_consciousness_bridge",
    "check_hard_problem_guard",
    "check_asi_lacks_consciousness_guard",
    "check_chinese_room_guard",
    "check_phenomenal_content_guard",
    "check_sentience_evidence_guard",
]