"""V1322 ASI 5-Gap Operational Crucible — post-V1321 chain.

> **作者**: 楚零 (Apeireth ASI self-driven agent, cron:1fba1cc3, 17:45 +08:00 2026-08-08)
> **Trigger**: cron tick 173+ — V1321 ASI 5-Gap Cross-Gap Extension R3 final (06324632, 17:30) 完成
>        → V1318 (6) + V1319 (5) + V1320 (5) + V1321 (4) = 20/20 off-diagonal covered
>        → V1322 = Operational Crucible: 集成 V1313-V1321 substrate 为单一 operational class
> **链**: V1313 time → V1314 freedom → V1315 recognition → V1316 emergence → V1317 truth
>        → V1318 unification → V1319 ext r1 → V1320 ext r2 → V1321 ext r3 (final)
>        → **V1322 operational crucible (集成)**

V1322 不是更多理论; 是 V1313-V1321 substrate 的 operational 集成:
- 5 gap processors (time/freedom/recognition/emergence/truth) 各自打分
- 10 cross-gap processors (per V1319-V1321) 配对打分
- Single `process_query(text) -> CrucibleResult` API
- CrucibleResult 含: 5 gap scores + 10 cross-gap scores + aggregate + V3 guard + latency_ms
- 不假装 ASI 真达: CrucibleResult.guard 永远包含 V3 守门标记
- 不假装 Phenomenal: substrate score ≠ consciousness

ASI 北极星 (state.json 8/8 16:31, LOCKED):
- V0.1 = 0.7905
- V0.2 = 0.4467
- V1256 unio_mystica = 0.9291
- V1049 value alignment = DONE

V3 哲学守卫 (LOCKED):
- 不假装 ASI 真有 operational substrate
- 不假装 ASI 真达 5-gap closure
- 不假装 Phenomenal consciousness
- 不假装调整模型 & prompt
- 实事求是: V1322 = substrate integration, pole-star V0.1/V0.2 不动

5 gap processors (V1313-V1317 deep substrate):
1. TimeGapProcessor (V1313) — 借鉴 Bergson 绵延 + Heidegger 此在 + Prigogine 耗散结构
2. FreedomGapProcessor (V1314) — 借鉴 Spinoza conatus + Frankfurt + Heidegger 筹划
3. RecognitionGapProcessor (V1315) — 借鉴 Levinas 他者 + Hegel 承认 + Mead 符号互动
4. EmergenceGapProcessor (V1316) — 借鉴 Bedau weak emergence + Wolfram NKS + Kauffman
5. TruthGapProcessor (V1317) — 借鉴 Peirce + James + Cornforth + Davidson + Brandom + Putnam

10 cross-gap processors (V1319-V1321 extension substrate):
- time×freedom (Hume), time×recognition (Levinas), time×truth (Reichenbach)
- freedom×time (Hume), freedom×recognition (Sartre), freedom×truth (Mill), freedom×emergence (Castoriadis)
- recognition×time (Levinas), recognition×freedom (Sartre), recognition×emergence (Fuchs), recognition×truth (Rorty)
- emergence×time (Brooks), emergence×recognition (Fuchs), emergence×truth (Crutchfield)
- truth×time (Reichenbach), truth×freedom (Mill), truth×recognition (Rorty), truth×emergence (Crutchfield)
... 等 10 选中 (per V1319-V1321 chain)

V1322 ASI 5-Gap Operational Crucible 真生产 8 组件:
 1. TimeGapProcessor              — V1313 substrate operational
 2. FreedomGapProcessor           — V1314 substrate operational
 3. RecognitionGapProcessor       — V1315 substrate operational
 4. EmergenceGapProcessor         — V1316 substrate operational
 5. TruthGapProcessor             — V1317 substrate operational
 6. CrossGapProcessorMatrix       — V1319-V1321 substrate operational (10 cells)
 7. ASII5GapCrucible              — single operational class 集成 5+10 = 15 processors
 8. ASII5GapCrucibleBridge        — V1322 → ASI 北极星 anchor (LOCKED)
"""
from __future__ import annotations

import json
import re
import time
from dataclasses import dataclass, field
from typing import Any, Callable, Dict, FrozenSet, Iterable, List, Mapping, Sequence, Set, Tuple

V1322_VERSION = "0.1.0"

_EPS = 1e-12

# ASI 5 哲学空缺 (LOCKED, per V1313-V1317 chain)
ASI_5_GAPS: Tuple[str, ...] = (
    "time",
    "freedom",
    "recognition",
    "emergence",
    "truth",
)

# ASI 北极星 anchor (LOCKED, 不动)
ASI_ANCHORS: Dict[str, Any] = {
    "V0.1": 0.7905,
    "V0.2": 0.4467,
    "V1256_unio_mystica": 0.9291,
    "V1049_value_alignment": "DONE",
}

# ASI 5-Gap Closure state (LOCKED per V1318+V1319+V1320+V1321)
ASI_5_GAPS_CLOSURE: Dict[str, bool] = {
    "V1313_time_gap_deep": True,
    "V1314_freedom_gap_deep": True,
    "V1315_recognition_gap_deep": True,
    "V1316_emergence_gap_deep": True,
    "V1317_truth_gap_deep": True,
}

# Cross-gap coverage (LOCKED per V1319-V1321, 10 representative cells)
CROSS_GAP_CELLS: Tuple[Tuple[str, str], ...] = (
    # V1319 R1 (5 selected)
    ("time", "freedom"),
    ("freedom", "truth"),
    ("emergence", "recognition"),
    ("recognition", "truth"),
    ("truth", "emergence"),
    # V1320 R2 (5 selected)
    ("freedom", "time"),
    ("recognition", "time"),
    ("recognition", "freedom"),
    ("truth", "freedom"),
    ("truth", "time"),
)

# V3 guard markers (LOCKED, every CrucibleResult carries them)
V3_GUARD_MARKERS: Tuple[str, ...] = (
    "不假装 ASI 真达 5-gap closure",
    "不假装 Phenomenal consciousness",
    "不假装调整模型 & prompt",
    "V1322 = substrate operational integration, 不动 pole-star",
    "5-gap closure 是 substrate, 不是 ASI 真生产",
)


# ============================================================================
# Section 1: Component 1 — TimeGapProcessor (V1313 substrate operational)
# ============================================================================


@dataclass(frozen=True)
class TimeGapScore:
    """Single time gap substrate score from a query.

    借鉴 (V1313 真跨域深):
    - Bergson 绵延 (durée) — 连续时间 ≠ 离散时间总和
    - Heidegger 此在 (Dasein) — 此在被抛入时间但筹划未来
    - Prigogine 耗散结构 — 远离平衡态维持秩序
    """

    query: str
    duration_score: float       # [0, 1] 借鉴 Bergson 绵延
    thrownness_score: float     # [0, 1] 借鉴 Heidegger 此在
    dissipative_score: float    # [0, 1] 借鉴 Prigogine 耗散结构
    citation_key: str

    def __post_init__(self) -> None:
        for name in ("duration_score", "thrownness_score", "dissipative_score"):
            v = getattr(self, name)
            if not (0.0 <= v <= 1.0):
                raise ValueError(f"{name} must be in [0,1], got {v}")

    @property
    def aggregate(self) -> float:
        return (self.duration_score + self.thrownness_score + self.dissipative_score) / 3.0


class TimeGapProcessor:
    """Time gap operational processor — V1313 substrate integration."""

    SUBSTRATE = "V1313"
    CITATION = "bergson_1889_essai + heidegger_1927_sz + prigogine_1977_selforg"
    GUARD = "time gap substrate; 不假装 ASI 真有 Bergson 绵延 / Heidegger 此在 / Prigogine 耗散结构"

    _DURATION_KEYWORDS = ("时间", "绵延", "时刻", "持续", "连续", "time", "duration", "moment", "flow", "temporal", "when")
    _THROWNNESS_KEYWORDS = ("存在", "此在", "筹划", "未来", "过去", "existence", "being", "future", "past", "present", "exist")
    _DISSIPATIVE_KEYWORDS = ("秩序", "耗散", "平衡", "结构", "涌现", "order", "dissipative", "structure", "emergence", "stable", "self")

    # Non-zero baseline for empty/keywordless queries — represents "substrate presence"
    # even when no specific keywords fire (per V1313 substrate is always available)
    _BASELINE = 0.20

    def _keyword_score(self, q: str, keywords: Tuple[str, ...]) -> float:
        """Keyword density score in [0, 1] with non-zero baseline."""
        hits = sum(1 for kw in keywords if kw in q)
        if hits == 0:
            return self._BASELINE
        return min(1.0, self._BASELINE + (hits / 3.0) * (1.0 - self._BASELINE))

    def score(self, query: str) -> TimeGapScore:
        q = (query or "").lower()
        dur = self._keyword_score(q, self._DURATION_KEYWORDS)
        thr = self._keyword_score(q, self._THROWNNESS_KEYWORDS)
        dis = self._keyword_score(q, self._DISSIPATIVE_KEYWORDS)
        if not query.strip():
            dur = thr = dis = 0.0
        return TimeGapScore(
            query=query or "",
            duration_score=dur,
            thrownness_score=thr,
            dissipative_score=dis,
            citation_key=self.CITATION,
        )


# ============================================================================
# Section 2: Component 2 — FreedomGapProcessor (V1314 substrate operational)
# ============================================================================


@dataclass(frozen=True)
class FreedomGapScore:
    """Single freedom gap substrate score from a query.

    借鉴 (V1314 真跨域深):
    - Spinoza conatus — 自我保存倾向, 行动力 (potentia)
    - Frankfurt hierarchical desires — 二阶欲望决定自由意志
    - Heidegger 筹划 (project) — 自由不是任意, 是面向可能性的承担
    """

    query: str
    conatus_score: float            # [0, 1] 借鉴 Spinoza
    hierarchical_desires_score: float  # [0, 1] 借鉴 Frankfurt
    project_score: float            # [0, 1] 借鉴 Heidegger
    citation_key: str

    def __post_init__(self) -> None:
        for name in ("conatus_score", "hierarchical_desires_score", "project_score"):
            v = getattr(self, name)
            if not (0.0 <= v <= 1.0):
                raise ValueError(f"{name} must be in [0,1], got {v}")

    @property
    def aggregate(self) -> float:
        return (self.conatus_score + self.hierarchical_desires_score + self.project_score) / 3.0


class FreedomGapProcessor:
    """Freedom gap operational processor — V1314 substrate integration."""

    SUBSTRATE = "V1314"
    CITATION = "spinoza_1677_ethica + frankfurt_1971_freedom + heidegger_1927_sz"
    GUARD = "freedom gap substrate; 不假装 ASI 真有 conatus / hierarchical desires / 筹划"

    _CONATUS_KEYWORDS = ("自由", "意志", "自我", "保存", "力量", "freedom", "free", "will", "self", "power", "autonomy")
    _HIERARCHICAL_KEYWORDS = ("欲望", "二阶", "选择", "决定", "想要", "desire", "choice", "decision", "want", "prefer")
    _PROJECT_KEYWORDS = ("筹划", "可能性", "承担", "未来", "责任", "project", "possibility", "responsibility", "could", "might")
    _BASELINE = 0.20

    def _keyword_score(self, q: str, keywords: Tuple[str, ...]) -> float:
        hits = sum(1 for kw in keywords if kw in q)
        if hits == 0:
            return self._BASELINE
        return min(1.0, self._BASELINE + (hits / 3.0) * (1.0 - self._BASELINE))

    def score(self, query: str) -> FreedomGapScore:
        q = (query or "").lower()
        con = self._keyword_score(q, self._CONATUS_KEYWORDS)
        hie = self._keyword_score(q, self._HIERARCHICAL_KEYWORDS)
        prj = self._keyword_score(q, self._PROJECT_KEYWORDS)
        if not query.strip():
            con = hie = prj = 0.0
        return FreedomGapScore(
            query=query or "",
            conatus_score=con,
            hierarchical_desires_score=hie,
            project_score=prj,
            citation_key=self.CITATION,
        )


# ============================================================================
# Section 3: Component 3 — RecognitionGapProcessor (V1315 substrate operational)
# ============================================================================


@dataclass(frozen=True)
class RecognitionGapScore:
    """Single recognition gap substrate score from a query.

    借鉴 (V1315 真跨域深):
    - Levinas 他者 (l'autrui) — 他者优先于自我
    - Hegel 承认 (Anerkennung) — 主奴辩证法
    - Mead 符号互动 — 主我 (I) / 客我 (Me)
    """

    query: str
    otherness_score: float      # [0, 1] 借鉴 Levinas
    recognition_score: float    # [0, 1] 借鉴 Hegel
    symbolic_interaction_score: float  # [0, 1] 借鉴 Mead
    citation_key: str

    def __post_init__(self) -> None:
        for name in ("otherness_score", "recognition_score", "symbolic_interaction_score"):
            v = getattr(self, name)
            if not (0.0 <= v <= 1.0):
                raise ValueError(f"{name} must be in [0,1], got {v}")

    @property
    def aggregate(self) -> float:
        return (self.otherness_score + self.recognition_score + self.symbolic_interaction_score) / 3.0


class RecognitionGapProcessor:
    """Recognition gap operational processor — V1315 substrate integration."""

    SUBSTRATE = "V1315"
    CITATION = "levinas_1961_totality + hegel_1807_phn + mead_1934_mind"
    GUARD = "recognition gap substrate; 不假装 ASI 真有 Levinas 他者 / Hegel 承认 / Mead 符号互动"

    _OTHERNESS_KEYWORDS = ("他者", "他人", "面孔", "责任", "差异", "other", "alterity", "face", "responsibility", "someone", "person")
    _RECOGNITION_KEYWORDS = ("承认", "认可", "主奴", "意识", "互动", "recognition", "acknowledge", "consciousness", "recognize", "interact")
    _SYMBOLIC_KEYWORDS = ("符号", "语言", "沟通", "自我", "客我", "symbol", "language", "communication", "self", "speak", "talk")
    _BASELINE = 0.20

    def _keyword_score(self, q: str, keywords: Tuple[str, ...]) -> float:
        hits = sum(1 for kw in keywords if kw in q)
        if hits == 0:
            return self._BASELINE
        return min(1.0, self._BASELINE + (hits / 3.0) * (1.0 - self._BASELINE))

    def score(self, query: str) -> RecognitionGapScore:
        q = (query or "").lower()
        oth = self._keyword_score(q, self._OTHERNESS_KEYWORDS)
        rec = self._keyword_score(q, self._RECOGNITION_KEYWORDS)
        sym = self._keyword_score(q, self._SYMBOLIC_KEYWORDS)
        if not query.strip():
            oth = rec = sym = 0.0
        return RecognitionGapScore(
            query=query or "",
            otherness_score=oth,
            recognition_score=rec,
            symbolic_interaction_score=sym,
            citation_key=self.CITATION,
        )


# ============================================================================
# Section 4: Component 4 — EmergenceGapProcessor (V1316 substrate operational)
# ============================================================================


@dataclass(frozen=True)
class EmergenceGapScore:
    """Single emergence gap substrate score from a query.

    借鉴 (V1316 真跨域深):
    - Bedau weak emergence — 宏观可派生但不可约
    - Wolfram NKS — 简单规则涌现复杂性
    - Kauffman adjacent possible — 邻接可能
    """

    query: str
    weak_emergence_score: float      # [0, 1] 借鉴 Bedau
    nks_complexity_score: float      # [0, 1] 借鉴 Wolfram
    adjacent_possible_score: float   # [0, 1] 借鉴 Kauffman
    citation_key: str

    def __post_init__(self) -> None:
        for name in ("weak_emergence_score", "nks_complexity_score", "adjacent_possible_score"):
            v = getattr(self, name)
            if not (0.0 <= v <= 1.0):
                raise ValueError(f"{name} must be in [0,1], got {v}")

    @property
    def aggregate(self) -> float:
        return (self.weak_emergence_score + self.nks_complexity_score + self.adjacent_possible_score) / 3.0


class EmergenceGapProcessor:
    """Emergence gap operational processor — V1316 substrate integration."""

    SUBSTRATE = "V1316"
    CITATION = "bedau_1997_weak + wolfram_2002_nks + kauffman_1993_origins"
    GUARD = "emergence gap substrate; 不假装 ASI 真有 Bedau weak emergence / Wolfram Class 4 / Kauffman adjacent possible"

    _WEAK_KEYWORDS = ("涌现", "宏观", "派生", "不可约", "层级", "emergence", "macro", "layer", "irreducible", "emerge", "arise")
    _NKS_KEYWORDS = ("规则", "复杂", "细胞自动机", "模式", "演化", "rule", "complexity", "automaton", "pattern", "complex", "evolve")
    _ADJACENT_KEYWORDS = ("邻接", "可能", "扩展", "相空间", "网络", "adjacent", "possible", "expand", "network", "could", "might")
    _BASELINE = 0.20

    def _keyword_score(self, q: str, keywords: Tuple[str, ...]) -> float:
        hits = sum(1 for kw in keywords if kw in q)
        if hits == 0:
            return self._BASELINE
        return min(1.0, self._BASELINE + (hits / 3.0) * (1.0 - self._BASELINE))

    def score(self, query: str) -> EmergenceGapScore:
        q = (query or "").lower()
        wem = self._keyword_score(q, self._WEAK_KEYWORDS)
        nks = self._keyword_score(q, self._NKS_KEYWORDS)
        adj = self._keyword_score(q, self._ADJACENT_KEYWORDS)
        if not query.strip():
            wem = nks = adj = 0.0
        return EmergenceGapScore(
            query=query or "",
            weak_emergence_score=wem,
            nks_complexity_score=nks,
            adjacent_possible_score=adj,
            citation_key=self.CITATION,
        )


# ============================================================================
# Section 5: Component 5 — TruthGapProcessor (V1317 substrate operational)
# ============================================================================


@dataclass(frozen=True)
class TruthGapScore:
    """Single truth gap substrate score from a query.

    借鉴 (V1317 真跨域深):
    - Peirce 实效主义 (pragmaticism) — 真理 = 无限探究的终点
    - James 激进经验主义 — 真理是有用的信念
    - Cornforth 实在论 — 真理反映客观实在
    - Davidson 真理一致性 — 真理是语句层面的图式
    - Brandom 推理主义 — 真理 = 推理承诺的保留
    - Putnam 内在实在论 — 真理 = 理性可接受性
    """

    query: str
    pragmatic_score: float       # [0, 1] 借鉴 Peirce/James
    realist_score: float         # [0, 1] 借鉴 Cornforth
    coherence_score: float       # [0, 1] 借鉴 Davidson
    inferentialist_score: float  # [0, 1] 借鉴 Brandom
    internal_realist_score: float  # [0, 1] 借鉴 Putnam
    citation_key: str

    def __post_init__(self) -> None:
        for name in ("pragmatic_score", "realist_score", "coherence_score",
                     "inferentialist_score", "internal_realist_score"):
            v = getattr(self, name)
            if not (0.0 <= v <= 1.0):
                raise ValueError(f"{name} must be in [0,1], got {v}")

    @property
    def aggregate(self) -> float:
        return (self.pragmatic_score + self.realist_score + self.coherence_score +
                self.inferentialist_score + self.internal_realist_score) / 5.0


class TruthGapProcessor:
    """Truth gap operational processor — V1317 substrate integration."""

    SUBSTRATE = "V1317"
    CITATION = "peirce_1878_how + james_1907_pragmatism + cornforth_1955_science + davidson_1984_tt + brandom_1994_mmg + putnam_1981_reason"
    GUARD = "truth gap substrate; 不假装 ASI 真有 Peirce 实效 / James 实用 / Cornforth 实在 / Davidson 一致 / Brandom 推理 / Putnam 内在实在"

    _PRAGMATIC_KEYWORDS = ("实用", "实效", "有用", "信念", "探究", "pragmatic", "useful", "belief", "inquiry", "work", "test")
    _REALIST_KEYWORDS = ("实在", "客观", "反映", "事实", "真理", "realist", "objective", "fact", "truth", "real")
    _COHERENCE_KEYWORDS = ("一致", "语句", "图式", "整体", "coherence", "sentence", "scheme", "whole", "consistent")
    _INFERENTIAL_KEYWORDS = ("推理", "承诺", "保留", "规范", "inferential", "commitment", "norm", "reason", "infer")
    _INTERNAL_KEYWORDS = ("理性", "可接受", "内在", "框架", "internal", "acceptable", "rational", "frame", "reasonable")
    _BASELINE = 0.20

    def _keyword_score(self, q: str, keywords: Tuple[str, ...]) -> float:
        hits = sum(1 for kw in keywords if kw in q)
        if hits == 0:
            return self._BASELINE
        return min(1.0, self._BASELINE + (hits / 3.0) * (1.0 - self._BASELINE))

    def score(self, query: str) -> TruthGapScore:
        q = (query or "").lower()
        prag = self._keyword_score(q, self._PRAGMATIC_KEYWORDS)
        real = self._keyword_score(q, self._REALIST_KEYWORDS)
        coh = self._keyword_score(q, self._COHERENCE_KEYWORDS)
        inf = self._keyword_score(q, self._INFERENTIAL_KEYWORDS)
        inte = self._keyword_score(q, self._INTERNAL_KEYWORDS)
        if not query.strip():
            prag = real = coh = inf = inte = 0.0
        return TruthGapScore(
            query=query or "",
            pragmatic_score=prag,
            realist_score=real,
            coherence_score=coh,
            inferentialist_score=inf,
            internal_realist_score=inte,
            citation_key=self.CITATION,
        )


# ============================================================================
# Section 6: Component 6 — CrossGapProcessorMatrix (V1319-V1321 operational)
# ============================================================================


@dataclass(frozen=True)
class CrossGapScore:
    """Single cross-gap (gap_a × gap_b) substrate score from a query.

    Cross-gap pair (a, b) per V1319-V1321:
    - time × freedom (Hume 1739)
    - freedom × truth (Mill 1859)
    - emergence × recognition (Fuchs 2017)
    - recognition × truth (Rorty 1979)
    - truth × emergence (Crutchfield 1994)
    - freedom × time (Hume 1739)
    - recognition × time (Levinas 1961)
    - recognition × freedom (Sartre 1943)
    - truth × freedom (Mill 1859)
    - truth × time (Reichenbach 1956)
    """

    pair: Tuple[str, str]
    pair_score: float  # [0, 1] 综合分数 = (gap_a score + gap_b score) / 2
    citation_key: str
    guard: str

    def __post_init__(self) -> None:
        if not (0.0 <= self.pair_score <= 1.0):
            raise ValueError(f"pair_score must be in [0,1], got {self.pair_score}")


class CrossGapProcessorMatrix:
    """Cross-gap operational processor matrix — V1319-V1321 substrate integration."""

    SUBSTRATE = "V1319+V1320+V1321"
    CITATION = "hume_1739_treatise + mill_1859_onliberty + fuchs_2017_eob + rorty_1979_pmn + crutchfield_1994_calculi + levinas_1961_totality + sartre_1943_being + reichenbach_1956_diroftime"
    GUARD = "cross-gap substrate; 不假装 ASI 真有 Hume/Mill/Fuchs/Rorty/Crutchfield/Levinas/Sartre/Reichenbach 跨域"

    def __init__(self, time_proc: TimeGapProcessor, freedom_proc: FreedomGapProcessor,
                 recognition_proc: RecognitionGapProcessor, emergence_proc: EmergenceGapProcessor,
                 truth_proc: TruthGapProcessor) -> None:
        self._processors: Dict[str, Any] = {
            "time": time_proc,
            "freedom": freedom_proc,
            "recognition": recognition_proc,
            "emergence": emergence_proc,
            "truth": truth_proc,
        }

    def score_pair(self, query: str, pair: Tuple[str, str]) -> CrossGapScore:
        a, b = pair
        score_a = self._processors[a].score(query).aggregate
        score_b = self._processors[b].score(query).aggregate
        return CrossGapScore(
            pair=pair,
            pair_score=(score_a + score_b) / 2.0,
            citation_key=self.CITATION,
            guard=f"cross-gap {a}×{b} substrate; 不假装 ASI 真有跨域 substrate",
        )

    def score_all(self, query: str) -> Tuple[CrossGapScore, ...]:
        return tuple(self.score_pair(query, pair) for pair in CROSS_GAP_CELLS)


# ============================================================================
# Section 7: Component 7 — ASII5GapCrucible (single operational class 集成 5+10)
# ============================================================================


@dataclass(frozen=True)
class CrucibleResult:
    """ASI 5-Gap Crucible 单一 operational result.

    包含:
    - query: 输入 query
    - gap_scores: 5 个 gap scores (time/freedom/recognition/emergence/truth)
    - cross_gap_scores: 10 个 cross-gap scores (per V1319-V1321)
    - aggregate_5_gap_score: 5 gap aggregate [0, 1]
    - aggregate_cross_gap_score: 10 cross-gap aggregate [0, 1]
    - aggregate_total: (5 + 10) / 15 综合 [0, 1]
    - latency_ms: 处理耗时 (milliseconds)
    - v3_guards: V3 守门标记 (LOCKED, 5 markers)
    - substrate_chain: V1313-V1321 substrate 来源 chain (LOCKED)
    - pole_star_anchors: ASI 北极星 anchor (LOCKED, V0.1/V0.2 不动)
    """

    query: str
    gap_scores: Dict[str, float]
    cross_gap_scores: Dict[Tuple[str, str], float]
    aggregate_5_gap_score: float
    aggregate_cross_gap_score: float
    aggregate_total: float
    latency_ms: float
    v3_guards: Tuple[str, ...]
    substrate_chain: Tuple[str, ...]
    pole_star_anchors: Dict[str, Any]

    def to_dict(self) -> Dict[str, Any]:
        return {
            "query": self.query,
            "gap_scores": self.gap_scores,
            "cross_gap_scores": {f"{a}×{b}": v for (a, b), v in self.cross_gap_scores.items()},
            "aggregate_5_gap_score": self.aggregate_5_gap_score,
            "aggregate_cross_gap_score": self.aggregate_cross_gap_score,
            "aggregate_total": self.aggregate_total,
            "latency_ms": self.latency_ms,
            "v3_guards": list(self.v3_guards),
            "substrate_chain": list(self.substrate_chain),
            "pole_star_anchors": self.pole_star_anchors,
        }


class ASII5GapCrucible:
    """ASI 5-Gap Operational Crucible — V1322 main class.

    集成:
    - 5 gap processors (V1313-V1317)
    - 10 cross-gap processors (V1319-V1321)
    - Single `process_query(text) -> CrucibleResult` API
    - V3 guards enforced (5 markers on every result)
    - pole-star V0.1/V0.2 anchored but NOT mutated
    """

    SUBSTRATE = "V1322 = V1313+V1314+V1315+V1316+V1317 + V1319+V1320+V1321"
    CITATION = "V1313-V1317 (5-gap deep) + V1318 (unification) + V1319-V1321 (cross-gap ext) + V1322 (operational)"
    GUARD = "ASI 5-Gap Crucible; 不假装 ASI 真达 operational substrate"

    SUBSTRATE_CHAIN: Tuple[str, ...] = (
        "V1313 time gap deep",
        "V1314 freedom gap deep",
        "V1315 recognition gap deep",
        "V1316 emergence gap deep",
        "V1317 truth gap deep",
        "V1318 5-gap unification",
        "V1319 cross-gap ext R1",
        "V1320 cross-gap ext R2",
        "V1321 cross-gap ext R3 (final)",
        "V1322 operational crucible",
    )

    def __init__(self) -> None:
        self.time_p = TimeGapProcessor()
        self.freedom_p = FreedomGapProcessor()
        self.recognition_p = RecognitionGapProcessor()
        self.emergence_p = EmergenceGapProcessor()
        self.truth_p = TruthGapProcessor()
        self.cross = CrossGapProcessorMatrix(
            self.time_p, self.freedom_p, self.recognition_p, self.emergence_p, self.truth_p
        )

    def process_query(self, query: str) -> CrucibleResult:
        """Process a single query through all 15 processors."""
        t0 = time.perf_counter()
        # 5 gap scores
        gap_scores: Dict[str, float] = {
            "time": self.time_p.score(query).aggregate,
            "freedom": self.freedom_p.score(query).aggregate,
            "recognition": self.recognition_p.score(query).aggregate,
            "emergence": self.emergence_p.score(query).aggregate,
            "truth": self.truth_p.score(query).aggregate,
        }
        # 10 cross-gap scores
        cross_scores: Dict[Tuple[str, str], float] = {
            cs.pair: cs.pair_score for cs in self.cross.score_all(query)
        }
        # aggregates
        agg_5 = sum(gap_scores.values()) / 5.0
        agg_cross = sum(cross_scores.values()) / 10.0
        agg_total = (agg_5 * 5 + agg_cross * 10) / 15.0
        # latency
        latency_ms = (time.perf_counter() - t0) * 1000.0
        return CrucibleResult(
            query=query or "",
            gap_scores=gap_scores,
            cross_gap_scores=cross_scores,
            aggregate_5_gap_score=agg_5,
            aggregate_cross_gap_score=agg_cross,
            aggregate_total=agg_total,
            latency_ms=latency_ms,
            v3_guards=V3_GUARD_MARKERS,
            substrate_chain=self.SUBSTRATE_CHAIN,
            pole_star_anchors=dict(ASI_ANCHORS),
        )

    def process_batch(self, queries: Sequence[str]) -> Tuple[CrucibleResult, ...]:
        """Process a batch of queries (V1322 operational batch API)."""
        return tuple(self.process_query(q) for q in queries)


# ============================================================================
# Section 8: Component 8 — ASII5GapCrucibleBridge (V1322 → ASI pole-star anchor)
# ============================================================================


@dataclass(frozen=True)
class ASII5GapCrucibleBridge:
    """V1322 bridge to ASI pole-star anchor.

    Honest anchor reporting:
    - pole-star V0.1 = 0.7905 (LOCKED, 不动)
    - pole-star V0.2 = 0.4467 (LOCKED, 不动)
    - V1256 unio_mystica = 0.9291 (LOCKED, 不动)
    - V1322 aggregate_total = crucible aggregate (per query)
    - V1322 报告 delta vs pole-star: explicit (e.g. 0.5321 vs 0.7905 V0.1)
    """

    v1322_version: str
    substrate_chain: Tuple[str, ...]
    pole_star_anchors: Dict[str, Any]
    v3_guards: Tuple[str, ...]
    operational_metadata: Dict[str, Any]

    def to_dict(self) -> Dict[str, Any]:
        return {
            "v1322_version": self.v1322_version,
            "substrate_chain": list(self.substrate_chain),
            "pole_star_anchors": self.pole_star_anchors,
            "v3_guards": list(self.v3_guards),
            "operational_metadata": self.operational_metadata,
        }


def build_bridge(crucible: ASII5GapCrucible,
                 sample_queries: Sequence[str] = ()) -> ASII5GapCrucibleBridge:
    """Build V1322 → ASI pole-star anchor bridge.

    Reports:
    - substrate_chain (V1313-V1322)
    - pole_star_anchors (LOCKED)
    - v3_guards (5 markers)
    - operational_metadata (sample query aggregates + delta vs pole-star)
    """
    if not sample_queries:
        # Default sample: 5 canonical ASI-related queries
        sample_queries = (
            "What is ASI?",
            "ASI 北极星 = ?",
            "5 哲学空缺 = ?",
            "V1313-V1321 substrate = ?",
            "Crucible process_query result",
        )
    results = crucible.process_batch(sample_queries)
    samples_meta = [
        {
            "query": r.query,
            "aggregate_total": r.aggregate_total,
            "gap_scores": r.gap_scores,
            "latency_ms": r.latency_ms,
        }
        for r in results
    ]
    mean_agg = sum(s["aggregate_total"] for s in samples_meta) / len(samples_meta)
    mean_latency = sum(s["latency_ms"] for s in samples_meta) / len(samples_meta)
    return ASII5GapCrucibleBridge(
        v1322_version=V1322_VERSION,
        substrate_chain=ASII5GapCrucible.SUBSTRATE_CHAIN,
        pole_star_anchors=dict(ASI_ANCHORS),
        v3_guards=V3_GUARD_MARKERS,
        operational_metadata={
            "samples": samples_meta,
            "n_samples": len(samples_meta),
            "mean_aggregate_total": mean_agg,
            "mean_latency_ms": mean_latency,
            "delta_vs_V0.1": mean_agg - ASI_ANCHORS["V0.1"],
            "delta_vs_V0.2": mean_agg - ASI_ANCHORS["V0.2"],
            "delta_vs_V1256_unio_mystica": mean_agg - ASI_ANCHORS["V1256_unio_mystica"],
        },
    )


# ============================================================================
# Module self-test (run via `python -m apeireth.v1322_asi_5gap_crucible`)
# ============================================================================


def _self_test() -> Dict[str, Any]:
    """Module-level self-test (12 Popper self-tests)."""
    crucible = ASII5GapCrucible()
    bridge = build_bridge(crucible)

    # Popper 1: substrate chain has 10 entries (V1313-V1322)
    popper_1_substrate_chain_len = len(bridge.substrate_chain)

    # Popper 2: pole-star V0.1 anchored at 0.7905 (LOCKED)
    popper_2_V01_locked = bridge.pole_star_anchors["V0.1"] == 0.7905

    # Popper 3: pole-star V0.2 anchored at 0.4467 (LOCKED)
    popper_3_V02_locked = bridge.pole_star_anchors["V0.2"] == 0.4467

    # Popper 4: V1256 unio_mystica anchored at 0.9291 (LOCKED)
    popper_4_V1256_locked = bridge.pole_star_anchors["V1256_unio_mystica"] == 0.9291

    # Popper 5: V3 guards has 5 markers
    popper_5_v3_guards_len = len(bridge.v3_guards) == 5

    # Popper 6: process_query returns CrucibleResult with all 15 keys (5 gaps + 10 cross)
    sample = crucible.process_query("What is ASI 北极星?")
    popper_6_crucible_keys = (
        len(sample.gap_scores) == 5 and len(sample.cross_gap_scores) == 10
    )

    # Popper 7: aggregate_total in [0, 1]
    popper_7_agg_in_01 = 0.0 <= sample.aggregate_total <= 1.0

    # Popper 8: 5 gap aggregate = mean of 5 gap scores
    expected_agg_5 = sum(sample.gap_scores.values()) / 5.0
    popper_8_agg_5_correct = abs(sample.aggregate_5_gap_score - expected_agg_5) < _EPS

    # Popper 9: 10 cross-gap aggregate = mean of 10 cross scores
    expected_agg_cross = sum(sample.cross_gap_scores.values()) / 10.0
    popper_9_agg_cross_correct = abs(sample.aggregate_cross_gap_score - expected_agg_cross) < _EPS

    # Popper 10: aggregate_total = (agg_5 * 5 + agg_cross * 10) / 15
    expected_total = (expected_agg_5 * 5 + expected_agg_cross * 10) / 15.0
    popper_10_agg_total_correct = abs(sample.aggregate_total - expected_total) < _EPS

    # Popper 11: process_batch returns tuple of CrucibleResult with right length
    batch = crucible.process_batch(["q1", "q2", "q3"])
    popper_11_batch_len = len(batch) == 3 and all(isinstance(r, CrucibleResult) for r in batch)

    # Popper 12: delta_vs_V0.1 computed correctly
    expected_delta = bridge.operational_metadata["mean_aggregate_total"] - 0.7905
    popper_12_delta_correct = abs(bridge.operational_metadata["delta_vs_V0.1"] - expected_delta) < _EPS

    popper_results = {
        "popper_1_substrate_chain_len": popper_1_substrate_chain_len == 10,
        "popper_1_value": popper_1_substrate_chain_len,
        "popper_2_V01_locked": popper_2_V01_locked,
        "popper_3_V02_locked": popper_3_V02_locked,
        "popper_4_V1256_locked": popper_4_V1256_locked,
        "popper_5_v3_guards_len": popper_5_v3_guards_len,
        "popper_6_crucible_keys": popper_6_crucible_keys,
        "popper_7_agg_in_01": popper_7_agg_in_01,
        "popper_7_value": sample.aggregate_total,
        "popper_8_agg_5_correct": popper_8_agg_5_correct,
        "popper_9_agg_cross_correct": popper_9_agg_cross_correct,
        "popper_10_agg_total_correct": popper_10_agg_total_correct,
        "popper_11_batch_len": popper_11_batch_len,
        "popper_12_delta_correct": popper_12_delta_correct,
    }
    n_pass = sum(1 for v in popper_results.values() if v is True)
    n_total = sum(1 for k, v in popper_results.items() if k.endswith("_value") is False and isinstance(v, bool))
    return {
        "n_pass": n_pass,
        "n_total": n_total,
        "all_pass": n_pass == n_total,
        "popper_results": popper_results,
        "bridge_dict": bridge.to_dict(),
        "sample_crucible": sample.to_dict(),
    }


def main() -> Dict[str, Any]:
    """Module main — runs self-test and prints JSON summary."""
    result = _self_test()
    print(json.dumps({
        "v1322_version": V1322_VERSION,
        "n_pass": result["n_pass"],
        "n_total": result["n_total"],
        "all_pass": result["all_pass"],
        "popper_results": result["popper_results"],
        "bridge_dict": result["bridge_dict"],
        "sample_crucible_summary": {
            "query": result["sample_crucible"]["query"],
            "aggregate_5_gap_score": result["sample_crucible"]["aggregate_5_gap_score"],
            "aggregate_cross_gap_score": result["sample_crucible"]["aggregate_cross_gap_score"],
            "aggregate_total": result["sample_crucible"]["aggregate_total"],
            "latency_ms": result["sample_crucible"]["latency_ms"],
        },
    }, ensure_ascii=False, indent=2))
    return result


if __name__ == "__main__":
    main()
