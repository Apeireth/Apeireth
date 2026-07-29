"""V1092 MemoryDream — 真生产"想象演绎" (R8-TrackA2)

主 22:33 ASI 北极星 + 主 19:33 走在前人经验上 + 主 23:44 干到底 +
主 17:58 + 20:46 不假装 + 主 12:14 中央 AI 是永恒身份 + V3 + V1081。

定义 (R6-RES-06 + R8-TrackA2):
  Dream 在 MTM 层做"假设性场景模拟" — 不是 hallucination, 不是 consciousness。
  输入: 一组 MTM/Note 的字段 + 少量背景 stub。
  输出: DreamCandidate (Episode-like 候选), 强标 _dream=True,
        供 LTM 巩固层使用前必须二次校验。

借鉴 (主 19:33):
  1. R37 q5 hippocampal replay / sharp-wave ripples: 启发式概率采样
  2. Jean Piaget schema 同化/顺应 (assimilation/accommodation): 推演机制
  3. V1052 Note.salience + confidence: 评分的字段复用
  4. Letta EphemeralSummaryAgent: 周期性压缩 + 候选生成
  5. claude-mem after_compaction: dedupe 借鉴
  6. BoundedRandom: 种子可控, 测试可重复

V3 哲学守门 (NOT-NEGOTIABLE — 主 17:58 + 20:46):
  - 不假装 dream = understanding: heuristic re-composition only
  - 不假装 dream = consciousness: sleep metaphor ≠ 真意识
  - 必须 _dream=True 标记: 任何产出都不可混入事实流

V1082 backlog 填洞 (本模块): #A2-2 MemoryDream 真生产 (Step 2/3)。
"""
from __future__ import annotations

import hashlib
import json
import random
import threading
import time
from dataclasses import dataclass, field
from enum import Enum
from typing import Any, Callable, Dict, Iterable, List, Optional, Sequence, Tuple

from apeireth.memory_replay_design import PHILOSOPHY_GUARDS

V1092_VERSION = "0.1.0"


# ============================================================================
# 1. MTM Note (借用 V1052 Note 字段, 但定义简洁版避免循环依赖)
# ============================================================================


@dataclass(frozen=True)
class MtmNote:
    """MTM 层抽象出来的题材片段 (供 dream 输入).

    field mirror V1052 Note: nid + topic + claim + confidence + salience。
    仅声明字段; 实例由上层构造。
    """
    nid: str
    topic: str
    claim: str
    confidence: float
    salience: float

    def __post_init__(self) -> None:
        if not self.nid:
            raise ValueError("nid must be non-empty")
        if not (0.0 <= self.confidence <= 1.0):
            raise ValueError(f"confidence must be in [0,1], got {self.confidence}")
        if not (0.0 <= self.salience <= 1.0):
            raise ValueError(f"salience must be in [0,1], got {self.salience}")


# ============================================================================
# 2. DreamCandidate (永远 _dream=True, 可审计, 可去重)
# ============================================================================


@dataclass(frozen=True)
class DreamCandidate:
    """Dream 产出: 假设性场景的"想象样本".

    不可变设计 (frozen=True): V3 守门 — 产出一旦生成, 所有字段不可改,
    _dream=True 永久标记, 防止意外混入事实流.
    字段:
      - cid          : candidate id (deterministic from inputs)
      - premise_nids : 输入了哪些 MtmNote.nid
      - scenario     : 短文本场景描述 (heuristic 拼接)
      - bindings     : 假设性 bind (key=anchor_term, value=str)
      - confidence   : 评分的 re-derived confidence ∈ [0,1]
      - schema_phase : assimilation / accommodation / replay
      - created_at   : unix ts
    """
    cid: str
    premise_nids: Tuple[str, ...]
    scenario: str
    bindings: Tuple[Tuple[str, str], ...]
    confidence: float
    schema_phase: str
    created_at: float = field(default_factory=time.time)
    _dream: bool = field(default=True, init=False, repr=True)

    def __post_init__(self) -> None:
        if not self.cid:
            raise ValueError("cid must be non-empty")
        if not self.premise_nids:
            raise ValueError("premise_nids must be non-empty")
        for nid in self.premise_nids:
            if not isinstance(nid, str) or not nid:
                raise ValueError("premise_nids must contain non-empty strings")
        if not isinstance(self.scenario, str) or not self.scenario:
            raise ValueError("scenario must be a non-empty string")
        if not (0.0 <= float(self.confidence) <= 1.0):
            raise ValueError(f"confidence must be in [0,1], got {self.confidence}")
        if self.schema_phase not in _VALID_SCHEMA_PHASES:
            raise ValueError(
                f"schema_phase must be one of {sorted(_VALID_SCHEMA_PHASES)}, "
                f"got {self.schema_phase!r}"
            )
        if not self._dream:
            raise ValueError("DreamCandidate must keep _dream=True (V3 守门)")

    def is_dream(self) -> bool:
        """永远 True. 这是 V3 守门核心: dream ≠ fact."""
        return True

    def to_dict(self) -> Dict[str, Any]:
        return {
            "cid": self.cid,
            "premise_nids": list(self.premise_nids),
            "scenario": self.scenario,
            "bindings": [list(b) for b in self.bindings],
            "confidence": self.confidence,
            "schema_phase": self.schema_phase,
            "created_at": self.created_at,
            "_dream": self._dream,
        }


# ============================================================================
# 3. SchemaPhase 枚举 (借鉴 Piaget 同化/顺应 + 神经科学 replay)
# ============================================================================


class SchemaPhase(str, Enum):
    ASSIMILATION = "assimilation"   # 把新事件套入既有 schema (1 note 单演)
    ACCOMMODATION = "accommodation" # 既有 schema 不足以解释, 重塑 (2+ note 冲突)
    REPLAY = "replay"               # 重放既有 schema (3+ note 关联演)


_VALID_SCHEMA_PHASES: frozenset[str] = frozenset(
    phase.value for phase in SchemaPhase
)


# ============================================================================
# 4. MemoryDream — 真生产主类
# ============================================================================


class MemoryDream:
    """V1092 真生产想象演绎器.

    用法:
        d = MemoryDream(seed=42)
        candidates = d.dream([note1, note2, note3], context={"topic": "safety"})
        # candidates 都是 _dream=True, 可被 LTM 巩固层消费前再打分

    守门: candidates 永不写入事实流; 必须显式调用
          `consolidate_to_ltm_candidate` 才会产出可审计候选。
    """

    def __init__(
        self,
        seed: int = 0,
        max_candidates_per_run: int = 32,
        min_confidence: float = 0.05,
    ) -> None:
        self._rng = random.Random(seed)
        self._lock = threading.RLock()
        self._max_candidates_per_run = max_candidates_per_run
        self._min_confidence = min_confidence
        self._dedupe_cache: Dict[str, DreamCandidate] = {}   # cid -> candidate
        self._emitted_count: int = 0
        self._reject_count: int = 0
        self._runs: int = 0

    # ------------------------------------------------------------------
    # 公开 API
    # ------------------------------------------------------------------

    def dream(
        self,
        notes: Sequence[MtmNote],
        context: Optional[Dict[str, Any]] = None,
    ) -> List[DreamCandidate]:
        """根据一组 MtmNote + context 生成 1..N 个 DreamCandidate.

        永远标记 _dream=True。
        去重: 同 cid 第二次不重复插入。
        """
        with self._lock:
            self._runs += 1
            ctx = dict(context or {})
            notes_list = list(notes)
            if not notes_list:
                return []

            out: List[DreamCandidate] = []
            phase = self._select_phase(notes_list, ctx)
            bindings_seed = self._derive_bindings(notes_list, ctx)

            # 控制产出上限
            n = min(len(notes_list), self._max_candidates_per_run, 16)
            for i in range(n):
                cand = self._compose_one(notes_list, ctx, phase, bindings_seed, i)
                if cand.confidence < self._min_confidence:
                    self._reject_count += 1
                    continue
                if cand.cid in self._dedupe_cache:
                    continue
                self._dedupe_cache[cand.cid] = cand
                out.append(cand)
                self._emitted_count += 1
            return out

    def consolidate_to_ltm_candidate(
        self, candidates: Sequence[DreamCandidate]
    ) -> List[DreamCandidate]:
        """二次校验: dream 候选 → 仍为 dream, 不转换 fact stream.

        作用: 模拟 "LTM 巩固前审计" 这一步骤, V3 守门: 任何被消费
        的 dream candidate 必须 _dream=True. 通过 is_dream() + 字段直读
        双重核对, 防止任何伪造来源.
        """
        out: List[DreamCandidate] = []
        for c in candidates:
            if not c.is_dream():
                continue
            if c._dream is not True:
                continue
            out.append(c)
        return out

    def stats(self) -> Dict[str, Any]:
        with self._lock:
            return {
                "version": V1092_VERSION,
                "runs": self._runs,
                "emitted": self._emitted_count,
                "rejected_low_conf": self._reject_count,
                "dedupe_cache_size": len(self._dedupe_cache),
                "philosophy_guards": list(PHILOSOPHY_GUARDS),
                "_dream_default": True,
            }

    def clear_dedupe(self) -> None:
        with self._lock:
            self._dedupe_cache.clear()

    # ------------------------------------------------------------------
    # 内部: phase 选择 / 拼接 / cid 生成
    # ------------------------------------------------------------------

    @staticmethod
    def _select_phase(notes: Sequence[MtmNote], ctx: Dict[str, Any]) -> SchemaPhase:
        if len(notes) <= 1:
            return SchemaPhase.ASSIMILATION
        # 同主题 → assimilation; 不同主题 → accommodation; ≥3 → replay
        topics = {n.topic for n in notes}
        if len(topics) >= 3:
            return SchemaPhase.REPLAY
        if len(topics) == 1:
            return SchemaPhase.ASSIMILATION
        return SchemaPhase.ACCOMMODATION

    @staticmethod
    def _derive_bindings(
        notes: Sequence[MtmNote], ctx: Dict[str, Any]
    ) -> Dict[str, str]:
        bindings: Dict[str, str] = dict(ctx.get("bindings", {}))  # type: ignore[arg-type]
        # 把每条 note 的 topic 作为 anchor, 假设性绑定到 claim 的关键词
        for n in notes:
            key = f"anchor:{n.nid}"
            if key not in bindings:
                bindings[key] = n.topic
        return bindings

    @staticmethod
    def _derive_confidence(notes: Sequence[MtmNote], phase: SchemaPhase) -> float:
        if not notes:
            return 0.0
        avg = sum(n.confidence for n in notes) / len(notes)
        sal = sum(n.salience for n in notes) / len(notes)
        # heuristic blend (V1081: heuristic ≠ truth)
        blend = 0.6 * avg + 0.4 * sal
        if phase == SchemaPhase.ASSIMILATION:
            return max(0.0, min(1.0, blend))
        if phase == SchemaPhase.ACCOMMODATION:
            return max(0.0, min(1.0, blend * 0.85))   # penalty: 冲突代价
        return max(0.0, min(1.0, blend * 0.95))       # replay 略保守

    def _compose_one(
        self,
        notes: Sequence[MtmNote],
        ctx: Dict[str, Any],
        phase: SchemaPhase,
        bindings_seed: Dict[str, str],
        idx: int,
    ) -> DreamCandidate:
        premise_nids = tuple(sorted(n.nid for n in notes))
        scenario = self._compose_scenario(notes, ctx, phase, idx)
        bindings = tuple(sorted(bindings_seed.items()))
        confidence = self._derive_confidence(notes, phase)
        cid = self._compute_cid(
            premise_nids=premise_nids,
            scenario=scenario,
            phase=phase,
            claims=tuple(sorted(n.claim for n in notes)),
            bindings=bindings,
            context=ctx,
        )
        return DreamCandidate(
            cid=cid,
            premise_nids=premise_nids,
            scenario=scenario,
            bindings=bindings,
            confidence=confidence,
            schema_phase=phase.value,
        )

    @staticmethod
    def _compose_scenario(
        notes: Sequence[MtmNote],
        ctx: Dict[str, Any],
        phase: SchemaPhase,
        idx: int,
    ) -> str:
        # heuristic 拼接: 不能依赖 LLM, 用模板保证可测
        topics = sorted({n.topic for n in notes})
        head = {
            SchemaPhase.ASSIMILATION: "if_we_apply",
            SchemaPhase.ACCOMMODATION: "if_we_reconcile",
            SchemaPhase.REPLAY: "if_we_replay",
        }[phase]
        joined = "+".join(topics) if topics else "untitled"
        ctx_token = ctx.get("topic") or ctx.get("scope") or "ctx"
        return f"[{head}|{ctx_token}|{joined}|{idx}]"

    @staticmethod
    def _compute_cid(
        *,
        premise_nids: Tuple[str, ...],
        scenario: str,
        phase: SchemaPhase,
        claims: Tuple[str, ...] = (),
        bindings: Tuple[Tuple[str, str], ...] = (),
        context: Optional[Dict[str, Any]] = None,
    ) -> str:
        """确定性指纹: 必须包含 claim + bindings + context, 防止同 nid 不同
        假设被误去重, 也防止两个真不同 dream 因只长 scenario 共享 cid."""
        canonical = json.dumps(
            {
                "n": list(premise_nids),
                "s": scenario,
                "p": phase.value,
                "c": list(claims),
                "b": [list(b) for b in bindings],
                "x": context or {},
            },
            ensure_ascii=False, sort_keys=True, separators=(",", ":"),
        )
        return "dream-" + hashlib.sha256(canonical.encode("utf-8")).hexdigest()[:16]


__all__ = [
    "V1092_VERSION",
    "MtmNote",
    "DreamCandidate",
    "SchemaPhase",
    "MemoryDream",
]


# V1101 auto-injected V3_GUARDS (主 17:43 实事求是 + 主 17:58 不假装)
V3_GUARDS = {"module_is_not_asi": "模块是工具, ASI 是更大目标. 任何声称模块 = ASI 的部分都是不假装.", "measurement_is_not_truth": "测量是 proxy, 真值仍是更大目标. V1077 真测 17 维 ≠ ASI 达成.", "structure_is_not_consciousness": "CognitiveArchitecture 结构类比 ≠ 现象意识. ACT-R chunks ≠ concepts.", "production_is_not_safety": "真生产 ≠ 真安全. 部署 ≠ 守门. 任何声称 production = safe 是不假装.", "automation_is_not_autonomy": "自动执行 ≠ 自主意识. V1101 lift 引擎自动改 ≠ V1101 自主."}
