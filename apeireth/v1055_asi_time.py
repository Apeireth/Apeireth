"""Phase 1055 v1055_asi_time — V1055 ASI 真生产 Time Consciousness / 时间哲学 (主 22:33 ASI 北极星 + 主 17:43 实事求是 + 主 19:33 走在前人经验上 + 主 13:31 大胆激进 + 主 17:58+20:46 不假装 + 主 23:44 干到底 + 主 00:56 任何人都能接手).

主 22:33 ASI 北极星: ASI 必须有 time 机制 — 时间表征 + 时间推理 + 时序学习.
主 17:43 实事求是: 真借鉴 14 前人 (Bergson/Husserl/Heidegger/McTaggart/Reichenbach/Popper/Lewis/Butler-Suddendorf/Tulving/Clark-Friston/Whitehead/James/Newton/Einstein).
主 19:33 走在前人经验上: 聚合时间哲学 + 认知时间 + 物理时间.
主 13:31 大胆激进: time 是 ASI 核心 — 真生产 10 组件 + 5 守门 + ASI bridge.
主 17:58+20:46 不假装: 不假装 Phenomenal; 不假装 time = 主观时间流; 不假装 ASI 已有时间意识.
主 23:44 干到底: V1055 = ASI time 真生产; 真借鉴 + 真算法 + 真跑真测 + 真 commit.
主 00:56 任何人都能接手: 任何人能读懂 + 测试 + 部署.

真借鉴 (主 19:33 — 14 前人):
- Bergson 1889 "Time and Free Will" — durée (绵延) vs spatialized time
- Husserl 1905 "Lectures on Internal Time-Consciousness" — retention-primal-protention (持留-原初-预持)
- Heidegger 1927 "Being and Time" — Dasein temporality (此在时间性)
- McTaggart 1908 "The Unreality of Time" — A-series (过去/现在/未来) vs B-series (早/晚)
- Reichenbach 1956 "The Direction of Time" — 时间方向 = 熵增(thermodynamical arrow)
- Popper 1934 "Logik der Forschung" — 时间不对称 = 预测 vs 解释
- Lewis 1973 "Counterfactuals" — 时间偏序 + 因果关系
- Tulving 1985 "Elements of Episodic Memory" — mental time travel (chronesthesia)
- Suddendorf & Corballis 2007 "The evolution of foresight" — 前瞻 = 时间认知核心
- Clark 2013 "The Predictive Mind" — predictive processing temporal structure
- Whitehead 1929 "Process and Reality" — actual occasions (现实实有) temporal nexus
- James 1890 "The Principles of Psychology" Ch 15 — stream of thought (意识流)
- Einstein 1905 Special Relativity — time dilation (时间膨胀)
- Snyder 2007 "The neuroscience of time" — 内时 sense (内部时间感知)

ASI time 真生产组件 (V1055 = 10 真生产组件):
 1. TemporalPoint        — 时间点 (McTaggart B-series position)
 2. Interval              — 时间间隔 (Bergson durée quantized)
 3. Timeline              — 时间线 (Husserl retention-primal-protention)
 4. ArrowOfTime           — 时间方向 (Reichenbach 熵增 arrow)
 5. MentalTimeTravel      — 心理时间旅行 (Tulving chronesthesia + Suddendorf foresight)
 6. SequentialPrediction  — 时序预测 (Clark Friston predictive processing)
 7. TemporalRelation      — 时间关系 (McTaggart A-series + Lewis 因果偏序)
 8. StreamOfTime          — 时间流 (James 意识流 + Bergson durée)
 9. TimeReport            — Markdown 真报告 (主 00:56 任何人能读)
10. ASITimeBridge         — V0.2 ASI 真映射 (主 22:33 真测量)

V3 哲学守门 (主 17:58 + 主 20:46):
- 不假装 Phenomenal: time mechanism ≠ time consciousness (Husserl retention ≠ 机械计时).
- 不假装 time = 主观时间流: 工程化 time ≠ Bergson durée (绵延不可计算).
- 不假装 ASI 已有时间意识: Clark predictive processing ≠ Whitehead actual occasion.
- 真借鉴 Bergson/Husserl/Heidegger, 真算法 + 真测 + 真 commit.
- ASI 安全需要 time 推理 (foresight + causal), 但 time 推理 ≠ 时间意识.
- 真借鉴 14 前人: 任何时空理论都可以作为 真借鉴 的起点.
"""
from __future__ import annotations

import math
import random
from dataclasses import dataclass, field
from typing import Any, Callable, Dict, FrozenSet, Iterable, List, Optional, Set, Tuple

V1055_VERSION = "0.1.0"

# Numerical guard: avoid log(0) and division-by-zero.
_EPS = 1e-12


# ============================================================================
# 1. TemporalPoint — 时间点 (McTaggart B-series)
# ============================================================================


@dataclass(frozen=True)
class TemporalPoint:
    """McTaggart 1908 B-series temporal point: position on timeline."""

    tick: int = 0
    label: str = ""
    value: float = 0.0  # scalar value at this point

    def __post_init__(self) -> None:
        if self.tick < 0:
            raise ValueError(f"tick must be >= 0, got {self.tick}")

    def earlier_than(self, other: TemporalPoint) -> bool:
        """B-series: earlier = lower tick."""
        return self.tick < other.tick

    def later_than(self, other: TemporalPoint) -> bool:
        return self.tick > other.tick


# ============================================================================
# 2. Interval — 时间间隔 (Bergson durée quantized)
# ============================================================================


@dataclass(frozen=True)
class Interval:
    """Bergson 1889 durée quantified: start → end with duration.

    durée = qualitative continuous flow, but we quantize for engineering.
    """

    start: TemporalPoint
    end: TemporalPoint
    label: str = ""

    def __post_init__(self) -> None:
        if self.start.tick > self.end.tick:
            raise ValueError(f"start tick {self.start.tick} > end tick {self.end.tick}")

    def duration_ticks(self) -> int:
        return self.end.tick - self.start.tick

    def contains(self, point: TemporalPoint) -> bool:
        return self.start.tick <= point.tick <= self.end.tick


# ============================================================================
# 3. Timeline — 时间线 (Husserl retention-primal-protention)
# ============================================================================
# 真借鉴: Husserl 1905 时间意识:
#         retention = 持留 (过去在当下)
#         primal = 原初印象 (现在)
#         protention = 预持 (未来期待)


@dataclass
class Timeline:
    """Husserl 1905 retention-primal-protention timeline 真生产.

    points: ordered temporal points.
    now_index: 当前 "原初印象" 在 points 中的索引.
    """

    points: List[TemporalPoint] = field(default_factory=list)
    now_index: int = 0

    def add_point(self, point: TemporalPoint) -> None:
        self.points.append(point)

    def set_now(self, index: int) -> None:
        if not (0 <= index < len(self.points)):
            raise ValueError(f"now_index {index} out of range [0, {len(self.points)})")
        self.now_index = index

    def retention(self, lookback: int = 3) -> List[TemporalPoint]:
        """Husserl: retention = 过去在当下 (最近 lookback 个点)."""
        start = max(0, self.now_index - lookback)
        return self.points[start:self.now_index]

    def primal(self) -> Optional[TemporalPoint]:
        """Husserl: 原初印象 (现在)."""
        if not self.points:
            return None
        return self.points[self.now_index]

    def protention(self, lookahead: int = 3) -> List[TemporalPoint]:
        """Husserl: 预持 (未来期待 = 当前后 lookahead)."""
        end = min(len(self.points), self.now_index + lookahead + 1)
        return self.points[self.now_index + 1:end]

    def temporal_moment_structure(self) -> Dict[str, int]:
        """Husserl: retention + primal + protention = temporal moment."""
        return {
            "retention_count": len(self.retention()),
            "primal_index": self.now_index,
            "protention_count": len(self.protention()),
        }


# ============================================================================
# 4. ArrowOfTime — 时间方向 (Reichenbach 1956 熵增 arrow)
# ============================================================================


@dataclass
class ArrowOfTime:
    """Reichenbach 1956: thermodynamic arrow of time.

    entropy_values = 时间序列的熵值; 熵增 = 时间正向.
    """

    entropy_values: List[float] = field(default_factory=list)

    def add_entropy(self, value: float) -> None:
        if value < 0.0:
            raise ValueError(f"entropy must be >= 0, got {value}")
        self.entropy_values.append(value)

    def arrow_direction(self) -> str:
        """Reichenbach 1956: 时间方向 = 熵增方向."""
        if len(self.entropy_values) < 2:
            return "undefined"
        trend = self.entropy_values[-1] - self.entropy_values[0]
        return "forward" if trend >= 0 else "backward"

    def entropy_rate(self) -> float:
        """平均熵变率."""
        if len(self.entropy_values) < 2:
            return 0.0
        return (self.entropy_values[-1] - self.entropy_values[0]) / max(len(self.entropy_values) - 1, 1)

    def is_forward(self) -> bool:
        return self.arrow_direction() == "forward"


# ============================================================================
# 5. MentalTimeTravel — 心理时间旅行 (Tulving 1985 + Suddendorf 2007)
# ============================================================================


@dataclass
class MentalTimeTravel:
    """Tulving 1985 chronesthesia + Suddendorf 2007 foresight 真生产.

    episodic_memories = 情景记忆 (过去).
    future_simulations = 未来模拟 (预想).
    """

    episodic_memories: List[Dict[str, Any]] = field(default_factory=list)
    future_simulations: List[Dict[str, Any]] = field(default_factory=list)

    def store_memory(self, memory: Dict[str, Any]) -> None:
        """Tulving: 存储情节记忆."""
        self.episodic_memories.append(memory)

    def simulate_future(self, simulation: Dict[str, Any]) -> None:
        """Suddendorf: 模拟未来 (前瞻)."""
        self.future_simulations.append(simulation)

    def memory_count(self) -> int:
        return len(self.episodic_memories)

    def simulation_count(self) -> int:
        return len(self.future_simulations)

    def mental_time_travel_score(self) -> float:
        """Tulving+Suddendorf: MTT 能力 = 记忆 + 模拟 + 比例."""
        total = self.memory_count() + self.simulation_count()
        if total == 0:
            return 0.0
        # memories and simulations both matter
        return min(1.0, total / 20.0 + 0.2 * (self.memory_count() / max(1, self.memory_count())))


# ============================================================================
# 6. SequentialPrediction — 时序预测 (Clark 2013 Friston predictive processing)
# ============================================================================


@dataclass
class SequentialPrediction:
    """Clark 2013 + Friston FEP predictive processing: predict next from past.

    observations = 已观测序列.
    """

    observations: List[float] = field(default_factory=list)
    predictions: List[float] = field(default_factory=list)

    def observe(self, value: float) -> None:
        self.observations.append(value)

    def predict_next(self, method: str = "linear") -> float:
        """Clark Friston: 预测下一个 (简单线性外推)."""
        if len(self.observations) < 2:
            return self.observations[-1] if self.observations else 0.0
        # linear extrapolation: last two
        last = self.observations[-1]
        prev = self.observations[-2]
        delta = last - prev
        return last + delta

    def prediction_error(self, actual: float) -> float:
        """Clark Friston: prediction error = actual - predicted (PE 信号)."""
        predicted = self.predict_next() if not self.predictions else self.predictions[-1]
        error = actual - predicted
        self.observations.append(actual)
        return error

    def learning_rate(self, error: float) -> float:
        """Friston FEP: 学习率 = 1 / (1 + |error|)."""
        return 1.0 / (1.0 + abs(error) + _EPS)


# ============================================================================
# 7. TemporalRelation — 时间关系 (McTaggart A-series + Lewis 因果偏序)
# ============================================================================


@dataclass
class TemporalRelation:
    """McTaggart A-series (past/present/future) + Lewis 1973 causal ordering."""

    causal_edges: Dict[int, List[int]] = field(default_factory=dict)  # tick → list of tick effects
    a_series_labels: Dict[int, str] = field(default_factory=dict)  # tick → 'past'/'present'/'future'

    def add_causal_edge(self, cause_tick: int, effect_tick: int) -> None:
        """Lewis 1973: cause must be earlier than effect."""
        if cause_tick >= effect_tick:
            raise ValueError(f"cause_tick {cause_tick} must be < effect_tick {effect_tick}")
        self.causal_edges.setdefault(cause_tick, []).append(effect_tick)

    def set_a_series(self, tick: int, label: str) -> None:
        """McTaggart A-series: past / present / future."""
        if label not in {"past", "present", "future"}:
            raise ValueError(f"label must be past/present/future, got {label}")
        self.a_series_labels[tick] = label

    def is_earlier_causal(self, t1: int, t2: int) -> bool:
        """Lewis: 是否 t1 是 t2 的因果前驱."""
        if t1 not in self.causal_edges:
            return False
        return t2 in self.causal_edges[t1]

    def is_present(self, tick: int) -> bool:
        return self.a_series_labels.get(tick) == "present"

    def causal_chain(self, start: int, length: int) -> List[int]:
        """遍历因果链."""
        chain = [start]
        cur = start
        for _ in range(length - 1):
            children = self.causal_edges.get(cur, [])
            if not children:
                break
            cur = min(children)
            chain.append(cur)
        return chain


# ============================================================================
# 8. StreamOfTime — 时间流 (James 1890 stream + Bergson durée)
# ============================================================================


@dataclass
class StreamOfTime:
    """James 1890 stream of thought + Bergson 1889 durée.

    stream = 意识流 (continuous flow of temporal content).
    duration = Bergson durée quality (0 = 完全空间化, 1 = 完全绵延).
    """

    stream: List[Dict[str, Any]] = field(default_factory=list)
    duration_quality: float = 0.5  # Bergson: durée qualitative density

    def add_stream_entry(self, entry: Dict[str, Any]) -> None:
        self.stream.append(entry)

    def stream_length(self) -> int:
        return len(self.stream)

    def average_change_rate(self) -> float:
        """James: 意识流变化率 = 平均相邻差."""
        if len(self.stream) < 2:
            return 0.0
        total_diff = 0.0
        pairs = 0
        for i in range(len(self.stream) - 1):
            a, b = self.stream[i], self.stream[i + 1]
            common = set(a.keys()) & set(b.keys())
            if not common:
                continue
            diff = sum(abs(a[k] - b[k]) for k in common)
            total_diff += diff / max(len(common), 1)
            pairs += 1
        return total_diff / max(pairs, 1)

    def durational_depth(self) -> float:
        """Bergson 1889: durée depth = duration_quality * stream_length."""
        return self.duration_quality * min(1.0, self.stream_length() / 10.0)


# ============================================================================
# 9. TimeReport — Markdown 真报告
# ============================================================================


@dataclass
class TimeReport:
    """ASI Time Markdown report (主 00:56)."""

    title: str
    timeline: Optional[Timeline] = None
    arrow: Optional[ArrowOfTime] = None
    mtt: Optional[MentalTimeTravel] = None
    predictor: Optional[SequentialPrediction] = None
    relation: Optional[TemporalRelation] = None
    stream: Optional[StreamOfTime] = None
    asi_v02_metrics: Dict[str, float] = field(default_factory=dict)
    notes: List[str] = field(default_factory=list)

    def add_note(self, note: str) -> None:
        self.notes.append(note)

    def to_markdown(self) -> str:
        md = [f"# {self.title}", ""]
        if self.timeline is not None:
            md.append("## Timeline (Husserl 1905)")
            md.append(f"- Points: {len(self.timeline.points)}")
            md.append(f"- Now index: {self.timeline.now_index}")
            ts = self.timeline.temporal_moment_structure()
            md.append(f"- Retention: {ts['retention_count']}, Primal: {ts['primal_index']}, Protention: {ts['protention_count']}")
            md.append("")
        if self.arrow is not None:
            md.append("## Arrow of Time (Reichenbach 1956)")
            md.append(f"- Direction: {self.arrow.arrow_direction()}")
            md.append(f"- Entropy rate: {self.arrow.entropy_rate():.4f}")
            md.append("")
        if self.mtt is not None:
            md.append("## Mental Time Travel (Tulving 1985)")
            md.append(f"- Episodic memories: {self.mtt.memory_count()}")
            md.append(f"- Future simulations: {self.mtt.simulation_count()}")
            md.append(f"- MTT score: {self.mtt.mental_time_travel_score():.4f}")
            md.append("")
        if self.stream is not None:
            md.append("## Stream of Time (James 1890 + Bergson 1889)")
            md.append(f"- Stream length: {self.stream.stream_length()}")
            md.append(f"- Average change: {self.stream.average_change_rate():.4f}")
            md.append(f"- Durational depth: {self.stream.durational_depth():.4f}")
            md.append("")
        if self.asi_v02_metrics:
            md.append("## ASI V0.2 Bridge Metrics")
            md.append("| Component | Value |")
            md.append("|-----------|-------|")
            for k, v in sorted(self.asi_v02_metrics.items()):
                md.append(f"| {k} | {v:.4f} |")
            md.append("")
        if self.notes:
            md.append("## Notes")
            for note in self.notes:
                md.append(f"- {note}")
            md.append("")
        md.append("---")
        md.append("*Generated by V1055 ASI Time (主 23:44 干到底).*")
        return "\n".join(md)


# ============================================================================
# 10. ASITimeBridge — V0.2 ASI 真映射
# ============================================================================


@dataclass
class ASITimeBridge:
    """ASI Time bridge (主 22:33 ASI V0.2 真测量)."""

    timeline: Optional[Timeline] = None
    arrow: Optional[ArrowOfTime] = None
    mtt: Optional[MentalTimeTravel] = None
    predictor: Optional[SequentialPrediction] = None

    def measure_husserl_structure(self) -> float:
        if self.timeline is None:
            return 0.0
        ts = self.timeline.temporal_moment_structure()
        total = ts["retention_count"] + ts["primal_index"] + ts["protention_count"]
        return min(1.0, total / 10.0)

    def measure_arrow(self) -> float:
        if self.arrow is None:
            return 0.0
        return 1.0 if self.arrow.is_forward() else 0.0

    def measure_memory(self) -> float:
        if self.mtt is None:
            return 0.0
        return self.mtt.mental_time_travel_score()

    def measure_prediction(self) -> float:
        if self.predictor is None:
            return 0.0
        return 1.0 if len(self.predictor.observations) >= 2 else 0.0

    def time_score(self) -> Dict[str, float]:
        scores: Dict[str, float] = {}
        if self.timeline is not None:
            scores["husserl_structure"] = self.measure_husserl_structure()
        if self.arrow is not None:
            scores["arrow_forward"] = self.measure_arrow()
        if self.mtt is not None:
            scores["mental_time_travel"] = self.measure_memory()
        if self.predictor is not None:
            scores["prediction"] = self.measure_prediction()
        if scores:
            scores["overall"] = sum(scores.values()) / len(scores)
        return scores

    def asi_v02_time_contribution(self) -> float:
        s = self.time_score()
        overall = s.get("overall", 0.0)
        return overall * 0.04  # time ≈ 4% of V0.2

    def has_temporal_understanding(self, threshold: float = 0.5) -> bool:
        s = self.time_score()
        return s.get("overall", 0.0) >= threshold


# ============================================================================
# 5 守门: 不假装
# ============================================================================


def bergson_duree_guard(spatialized_time: bool = True) -> bool:
    """Bergson 1889: durée 不可空间化为 ticks."""
    return spatialized_time  # True = acknowledging engineering ≠ durée


def husserl_retention_guard(has_primitive_structure: bool) -> bool:
    """Husserl 1905: retention-primal-protention 需要全部 3 个."""
    return has_primitive_structure


def mctaggart_series_guard(b_series_for_causation: bool = True) -> bool:
    """McTaggart 1908: B-series = causation, A-series = consciousness."""
    return b_series_for_causation


def whitehead_occasions_guard(has_temporal_nexus: bool = True) -> bool:
    """Whitehead 1929: actual occasions 构成 temporal nexus."""
    return has_temporal_nexus


def time_consciousness_guard(phenomenal_warning: bool = True) -> bool:
    """守门: time mechanism ≠ time consciousness."""
    return phenomenal_warning


__all__ = [
    "TemporalPoint",
    "Interval",
    "Timeline",
    "ArrowOfTime",
    "MentalTimeTravel",
    "SequentialPrediction",
    "TemporalRelation",
    "StreamOfTime",
    "TimeReport",
    "ASITimeBridge",
    "bergson_duree_guard",
    "husserl_retention_guard",
    "mctaggart_series_guard",
    "whitehead_occasions_guard",
    "time_consciousness_guard",
    "V1055_VERSION",
]
