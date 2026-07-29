"""V1108 Dream V2 — 6 状态机 + V3 守门强化 + 与 V1107 cognitive_core 集成

主 22:33 ASI 北极星 + 主 19:33 走在前人经验上 + 主 23:44 干到底 +
主 17:58 + 20:46 不假装 + 主 12:14 中央 AI 是永恒身份 + V3 + V1081 + V1092.

V1108 = R9-FE-001 副模块. V1092 → V1108 的核心差异:
  1. **6 状态机**: IDLE / DREAMING / CONSOLIDATING / FORGETTING / VERIFYING / INTERRUPTED
  2. **V3 守门强化**: _dream=True 永远 (init=False) + dream_is_not_consciousness 显式声明
  3. **DreamEpisode adapter**: 与 V1107 EpisodeBuffer / NoteConsolidator 集成
  4. **审计日志**: state transitions / cids 全留痕

6 状态机转换 (主 23:44 干到底):
  IDLE ──input──▶ DREAMING ──compose──▶ CONSOLIDATING ──write episode──▶ VERIFYING
                    │                       │                              │
                    └────interrupt───────▶ INTERRUPTED ◀────reject──────┘ │
                    │                                                      │
                    └───────────────low_conf──────────────────────▶ FORGETTING
                                                                              │
                                                                              └─▶ IDLE

V3 哲学守门 (NOT-NEGOTIABLE — 主 17:58 + 20:46):
  - 不假装 dream = understanding: heuristic re-composition only
  - 不假装 dream = consciousness: sleep metaphor ≠ 真意识
  - 不假装 state_machine = real_consciousness: 6 状态 = 状态机, ≠ 心理状态
  - 必须 _dream=True 标记: 任何产出都不可混入事实流
  - dream_is_not_consciousness 显式声明: dream ≠ consciousness

真借鉴 (主 19:33):
  1. V1092 MemoryDream: 3 SchemaPhase (assimilation/accommodation/replay)
  2. REM/NREM sleep cycle: REM = dreaming, NREM = consolidation
  3. Sleep-dependent memory consolidation (Walker 2017)
  4. V1081 MTM/Note 字段 + salience + confidence
  5. Finite-state machine pattern (Hopcroft 1979)

干到底 (主 23:44):
  - 6 状态机真实现
  - _dream=True frozen 永远
  - 与 V1107 EpisodeBuffer 集成 (DreamEpisode 适配)
  - ≥30 测试
"""
from __future__ import annotations

import hashlib
import json
import random
import threading
import time
from dataclasses import dataclass, field
from enum import Enum
from typing import Any, Callable, Dict, Iterable, List, Optional, Sequence, Set, Tuple

# Lazy import v1092 (与 V1108 共存)
_V1092_MOD: Optional[Any] = None


def _safe_import_apeireth_v1092() -> Any:
    """Lazy import v1092 — 主 17:43 实事求是: 不重定义 DreamCandidate."""
    global _V1092_MOD
    if _V1092_MOD is None:
        from apeireth import v1092_memory_dream as m  # type: ignore
        _V1092_MOD = m
    return _V1092_MOD


V1108_VERSION = "0.2.0"


# ============================================================================
# 0. V3 哲学守门 — dream_is_not_consciousness 显式声明 (主 17:58 + 20:46)
# ============================================================================

# 主 17:58+20:46 不假装: 这条注释是文档级 + 代码级双重守门.
# Dream 是 heuristic re-composition 的工具. 不是意识. 不是理解.
DREAM_IS_NOT_CONSCIOUSNESS = (
    "Dream is a heuristic re-composition tool (V1092/V1108), NOT a phenomenon "
    "of consciousness. The 6-state machine (IDLE/DREAMING/CONSOLIDATING/"
    "FORGETTING/VERIFYING/INTERRUPTED) is a finite-state automaton borrowed "
    "from Hopcroft 1979 — it is a computational abstraction, NOT a model of "
    "sleep or dreaming in biological organisms. Sleep metaphor ≠ 真意识."
)


# ============================================================================
# 1. 6 状态机 (主 23:44 干到底)
# ============================================================================


class DreamState(str, Enum):
    """6 状态机 (主 23:44)."""

    IDLE = "idle"
    DREAMING = "dreaming"
    CONSOLIDATING = "consolidating"
    FORGETTING = "forgetting"
    VERIFYING = "verifying"
    INTERRUPTED = "interrupted"


# 合法转换: 主 23:44 干到底 — 真定义 FSM
_DREAM_TRANSITIONS: Dict[DreamState, Set[DreamState]] = {
    DreamState.IDLE: {DreamState.DREAMING},
    DreamState.DREAMING: {DreamState.CONSOLIDATING, DreamState.INTERRUPTED,
                          DreamState.FORGETTING},
    DreamState.CONSOLIDATING: {DreamState.VERIFYING, DreamState.FORGETTING,
                                DreamState.INTERRUPTED},
    DreamState.VERIFYING: {DreamState.IDLE, DreamState.FORGETTING,
                            DreamState.INTERRUPTED},
    DreamState.FORGETTING: {DreamState.IDLE, DreamState.INTERRUPTED},
    DreamState.INTERRUPTED: {DreamState.IDLE},
}


# ============================================================================
# 2. DreamCandidate (V1092 重用 + V3 守门强化)
# ============================================================================
# 真生产: 全部借 V1092.DreamCandidate, V3 守门靠它本身.
# V3 守门 (主 17:58): _dream=True 永远 (init=False, default=True).
# V3 守门: frozen=True 防止任何字段被改.


@dataclass(frozen=True)
class DreamCandidateV2:
    """V1108 DreamCandidate (强化版): 继承 V1092 + 加 audit_trail + state_at_birth.

    V3 守门强化:
      - _dream=True 永远 (init=False, default=True) — 任何 __init__ 都不可改
      - frozen=True — 整个 dataclass 不可变
      - state_at_birth — 出生时的 FSM 状态 (审计)
      - audit_trail — 转换日志 (主 23:44 干到底)
    """

    cid: str
    premise_nids: Tuple[str, ...]
    scenario: str
    bindings: Tuple[Tuple[str, str], ...]
    confidence: float
    schema_phase: str
    state_at_birth: str                       # V1108 新: 出生 FSM 状态
    audit_trail: Tuple[Tuple[str, float], ...]  # V1108 新: (event, ts) 元组
    created_at: float = field(default_factory=time.time)
    _dream: bool = field(default=True, init=False, repr=True)

    def __post_init__(self) -> None:
        # V3 守门: _dream 必须 True (哪怕你显式传 _dream=False 也不行)
        if not self._dream:
            raise ValueError("DreamCandidateV2 must keep _dream=True (V3 守门)")
        if not self.cid:
            raise ValueError("cid must be non-empty")
        if not self.premise_nids:
            raise ValueError("premise_nids must be non-empty")
        if not isinstance(self.scenario, str) or not self.scenario:
            raise ValueError("scenario must be a non-empty string")
        if not (0.0 <= float(self.confidence) <= 1.0):
            raise ValueError(f"confidence must be in [0,1], got {self.confidence}")
        valid_phases = {"assimilation", "accommodation", "replay"}
        if self.schema_phase not in valid_phases:
            raise ValueError(f"schema_phase must be one of {sorted(valid_phases)}")

    def is_dream(self) -> bool:
        """永远 True. 这是 V3 守门核心."""
        return True

    def to_dict(self) -> Dict[str, Any]:
        return {
            "cid": self.cid,
            "premise_nids": list(self.premise_nids),
            "scenario": self.scenario,
            "bindings": [list(b) for b in self.bindings],
            "confidence": self.confidence,
            "schema_phase": self.schema_phase,
            "state_at_birth": self.state_at_birth,
            "audit_trail": [list(t) for t in self.audit_trail],
            "created_at": self.created_at,
            "_dream": self._dream,
        }


# ============================================================================
# 3. DreamStateMachine — 6 状态 FSM (Hopcroft 1979 借鉴)
# ============================================================================


class DreamStateMachine:
    """6 状态有限状态机 (Hopcroft 1979 借鉴).

    主 23:44 干到底: 状态转换合法 + 审计日志 + 拒绝非法转换.
    """

    def __init__(self, initial: DreamState = DreamState.IDLE) -> None:
        if initial != DreamState.IDLE:
            raise ValueError(f"initial state must be IDLE, got {initial.value}")
        self._state: DreamState = initial
        self._lock = threading.RLock()
        self._history: List[Tuple[DreamState, DreamState, float, str]] = []
        # (from_state, to_state, ts, reason)

    @property
    def state(self) -> DreamState:
        return self._state

    def transition(self, target: DreamState,
                   reason: str = "") -> bool:
        """尝试转换. 非法转换返回 False (主 17:43 实事求是)."""
        with self._lock:
            allowed = _DREAM_TRANSITIONS.get(self._state, set())
            if target not in allowed:
                # V3 守门: 非法转换拒绝 (dream ≠ random)
                return False
            old = self._state
            self._state = target
            self._history.append((old, target, time.time(), reason))
            return True

    def force_idle(self, reason: str = "reset") -> None:
        """强制回到 IDLE (interrupt 后必备)."""
        with self._lock:
            old = self._state
            self._state = DreamState.IDLE
            self._history.append((old, DreamState.IDLE, time.time(), reason))

    def history(self) -> List[Tuple[DreamState, DreamState, float, str]]:
        return list(self._history)

    def audit_count(self) -> int:
        return len(self._history)


# ============================================================================
# 4. Dream V2 主类 — 6 状态机驱动
# ============================================================================


@dataclass
class DreamV2Stats:
    """Dream V2 统计 (主 17:43 实事求是)."""
    runs: int = 0
    candidates_emitted: int = 0
    candidates_consolidated: int = 0
    candidates_verified: int = 0
    candidates_forgotten: int = 0
    candidates_interrupted: int = 0
    rejects_low_conf: int = 0
    rejects_invalid_state: int = 0
    state_transitions: int = 0


class MemoryDreamV2:
    """V1108 真生产 Memory Dream V2 (6 状态机).

    流程:
      1. IDLE → DREAMING (输入 notes)
      2. DREAMING → CONSOLIDATING (compose candidates)
      3. CONSOLIDATING → VERIFYING (write audit trail)
      4. VERIFYING → IDLE (pass) | FORGETTING (low conf) | INTERRUPTED (外部 stop)
      5. FORGETTING → IDLE
      6. INTERRUPTED → IDLE (resume)

    用法:
        d = MemoryDreamV2(seed=42)
        result = d.dream([note1, note2], context={"topic": "safety"})
        # result.candidates 是 List[DreamCandidateV2], 全部 _dream=True
        # result.episodes 是 List[Dict] (供 V1107 EpisodeBuffer 消费)
    """

    def __init__(
        self,
        seed: int = 0,
        max_candidates_per_run: int = 32,
        min_confidence: float = 0.05,
        verify_threshold: float = 0.1,
    ) -> None:
        self._rng = random.Random(seed)
        self._lock = threading.RLock()
        self._max_candidates_per_run = max_candidates_per_run
        self._min_confidence = min_confidence
        self._verify_threshold = verify_threshold
        self._fsm = DreamStateMachine(initial=DreamState.IDLE)
        self._stats = DreamV2Stats()
        self._dedupe_cache: Dict[str, DreamCandidateV2] = {}
        # 尝试 import V1092 (失败仍可本地运行)
        try:
            self._v1092 = _safe_import_apeireth_v1092()
        except Exception:  # pragma: no cover
            self._v1092 = None

    # ------------------------------------------------------------------
    # 公开 API
    # ------------------------------------------------------------------

    def dream(
        self,
        notes: Sequence[Any],     # 接受 V1092.MtmNote 或 任何带 nid/topic/claim 对象
        context: Optional[Dict[str, Any]] = None,
    ) -> "DreamV2Result":
        """根据一组 notes + context 跑一次 6 状态机 dream cycle.

        返回 DreamV2Result: candidates + episodes + transitions。
        """
        with self._lock:
            self._stats.runs += 1
            ctx = dict(context or {})
            notes_list = list(notes)
            transitions: List[Tuple[DreamState, DreamState, float, str]] = []
            out: List[DreamCandidateV2] = []
            episodes: List[Dict[str, Any]] = []

            if not notes_list:
                return DreamV2Result(
                    candidates=[], episodes=[], transitions=transitions,
                    stats=self._stats_snapshot(),
                    final_state=self._fsm.state,
                )

            # 1. IDLE → DREAMING
            if not self._fsm.transition(DreamState.DREAMING, reason="input"):
                self._stats.rejects_invalid_state += 1
                return DreamV2Result(
                    candidates=[], episodes=[], transitions=transitions,
                    stats=self._stats_snapshot(),
                    final_state=self._fsm.state,
                )
            self._stats.state_transitions += 1
            transitions.extend(self._consume_history())

            # 2. DREAMING: compose candidates
            n = min(len(notes_list), self._max_candidates_per_run, 16)
            phase = self._select_phase(notes_list, ctx)
            bindings_seed = self._derive_bindings(notes_list, ctx)
            for i in range(n):
                cand = self._compose_one(
                    notes_list, ctx, phase, bindings_seed, i,
                    state_at_birth=DreamState.DREAMING.value,
                )
                if cand is None:
                    self._stats.rejects_low_conf += 1
                    continue
                if cand.cid in self._dedupe_cache:
                    continue
                self._dedupe_cache[cand.cid] = cand
                out.append(cand)
                self._stats.candidates_emitted += 1

            # 3. DREAMING → CONSOLIDATING
            if not self._fsm.transition(DreamState.CONSOLIDATING,
                                         reason=f"compose_n={len(out)}"):
                self._interrupt(reason="DREAMING→CONSOLIDATING rejected")
                transitions.extend(self._consume_history())
                return DreamV2Result(
                    candidates=out, episodes=episodes,
                    transitions=transitions,
                    stats=self._stats_snapshot(),
                    final_state=self._fsm.state,
                )
            self._stats.state_transitions += 1
            transitions.extend(self._consume_history())

            # 4. CONSOLIDATING: 每个 cand 转 DreamEpisode (给 V1107)
            for cand in out:
                ep = self._cand_to_episode(cand)
                episodes.append(ep)
                self._stats.candidates_consolidated += 1

            # 5. CONSOLIDATING → VERIFYING
            if not self._fsm.transition(DreamState.VERIFYING,
                                         reason="verify"):
                self._interrupt(reason="CONSOLIDATING→VERIFYING rejected")
                transitions.extend(self._consume_history())
                return DreamV2Result(
                    candidates=out, episodes=episodes,
                    transitions=transitions,
                    stats=self._stats_snapshot(),
                    final_state=self._fsm.state,
                )
            self._stats.state_transitions += 1
            transitions.extend(self._consume_history())

            # 6. VERIFYING: 按 confidence 决定去留
            passed: List[DreamCandidateV2] = []
            for cand in out:
                if cand.confidence >= self._verify_threshold:
                    passed.append(cand)
                    self._stats.candidates_verified += 1
                else:
                    self._stats.candidates_forgotten += 1

            # 7. VERIFYING → IDLE (或 FORGETTING 如果有被丢的)
            if self._stats.candidates_forgotten > self._stats.candidates_emitted // 2:
                self._fsm.transition(DreamState.FORGETTING, reason="majority_rejected")
                self._stats.state_transitions += 1
                transitions.extend(self._consume_history())
                self._fsm.transition(DreamState.IDLE, reason="reset")
                self._stats.state_transitions += 1
            else:
                self._fsm.transition(DreamState.IDLE, reason="verified")
                self._stats.state_transitions += 1
            transitions.extend(self._consume_history())

            return DreamV2Result(
                candidates=passed, episodes=episodes,
                transitions=transitions,
                stats=self._stats_snapshot(),
                final_state=self._fsm.state,
            )

    def interrupt(self, reason: str = "external_stop") -> bool:
        """外部中断: 任意状态 → INTERRUPTED."""
        with self._lock:
            return self._interrupt(reason=reason)

    def reset(self) -> None:
        """强制回到 IDLE."""
        with self._lock:
            self._fsm.force_idle(reason="manual_reset")

    def state(self) -> DreamState:
        """当前状态 (主 00:56)."""
        return self._fsm.state

    def stats(self) -> Dict[str, Any]:
        """统计 + 哲学守门 (主 17:43 实事求是)."""
        s = self._stats_snapshot()
        s["philosophy_guards"] = [
            "dream_is_not_consciousness",
            "_dream=True_forever",
            "6_state_machine_is_not_psychology",
        ]
        s["_dream_default"] = True
        s["version"] = V1108_VERSION
        return s

    def audit_log(self) -> List[Tuple[DreamState, DreamState, float, str]]:
        """全部状态转换审计."""
        return self._fsm.history()

    # ------------------------------------------------------------------
    # 内部方法
    # ------------------------------------------------------------------

    def _interrupt(self, reason: str) -> bool:
        """内部 interrupt."""
        ok = self._fsm.transition(DreamState.INTERRUPTED, reason=reason)
        if ok:
            self._stats.candidates_interrupted += 1
            self._stats.state_transitions += 1
            # 立刻 force_idle
            self._fsm.force_idle(reason="interrupt_recovery")
            self._stats.state_transitions += 1
        return ok

    def _consume_history(self) -> List[Tuple[DreamState, DreamState, float, str]]:
        """取 self._fsm.history() 副本 (主 17:43)."""
        return list(self._fsm.history())

    def _stats_snapshot(self) -> Dict[str, Any]:
        return {
            "runs": self._stats.runs,
            "candidates_emitted": self._stats.candidates_emitted,
            "candidates_consolidated": self._stats.candidates_consolidated,
            "candidates_verified": self._stats.candidates_verified,
            "candidates_forgotten": self._stats.candidates_forgotten,
            "candidates_interrupted": self._stats.candidates_interrupted,
            "rejects_low_conf": self._stats.rejects_low_conf,
            "rejects_invalid_state": self._stats.rejects_invalid_state,
            "state_transitions": self._stats.state_transitions,
        }

    @staticmethod
    def _select_phase(notes: Sequence[Any],
                       ctx: Dict[str, Any]) -> str:
        """3 phase: ASSIMILATION/ACCOMMODATION/REPLAY (借 V1092.SchemaPhase)."""
        if len(notes) <= 1:
            return "assimilation"
        topics = {getattr(n, "topic", "") for n in notes}
        if len(topics) >= 3:
            return "replay"
        if len(topics) == 1:
            return "assimilation"
        return "accommodation"

    @staticmethod
    def _derive_bindings(
        notes: Sequence[Any], ctx: Dict[str, Any]
    ) -> Dict[str, str]:
        bindings: Dict[str, str] = dict(ctx.get("bindings", {}))  # type: ignore[arg-type]
        for n in notes:
            key = f"anchor:{getattr(n, 'nid', '?')}"
            if key not in bindings:
                bindings[key] = str(getattr(n, "topic", "untitled"))
        return bindings

    @staticmethod
    def _derive_confidence(notes: Sequence[Any], phase: str) -> float:
        if not notes:
            return 0.0
        avg = sum(getattr(n, "confidence", 0.5) for n in notes) / len(notes)
        sal = sum(getattr(n, "salience", 0.5) for n in notes) / len(notes)
        blend = 0.6 * avg + 0.4 * sal
        if phase == "assimilation":
            return max(0.0, min(1.0, blend))
        if phase == "accommodation":
            return max(0.0, min(1.0, blend * 0.85))
        return max(0.0, min(1.0, blend * 0.95))

    def _compose_one(
        self,
        notes: Sequence[Any],
        ctx: Dict[str, Any],
        phase: str,
        bindings_seed: Dict[str, str],
        idx: int,
        state_at_birth: str,
    ) -> Optional[DreamCandidateV2]:
        premise_nids = tuple(sorted(getattr(n, "nid", f"n_{idx}") for n in notes))
        scenario = self._compose_scenario(notes, ctx, phase, idx)
        bindings = tuple(sorted(bindings_seed.items()))
        confidence = self._derive_confidence(notes, phase)
        if confidence < self._min_confidence:
            return None
        cid = self._compute_cid(
            premise_nids=premise_nids, scenario=scenario, phase=phase,
            claims=tuple(sorted(str(getattr(n, "claim", "")) for n in notes)),
            bindings=bindings, context=ctx,
        )
        audit = (("born", time.time()),)
        return DreamCandidateV2(
            cid=cid, premise_nids=premise_nids, scenario=scenario,
            bindings=bindings, confidence=confidence,
            schema_phase=phase, state_at_birth=state_at_birth,
            audit_trail=audit,
        )

    @staticmethod
    def _compose_scenario(
        notes: Sequence[Any],
        ctx: Dict[str, Any],
        phase: str,
        idx: int,
    ) -> str:
        topics = sorted({getattr(n, "topic", "untitled") for n in notes})
        head = {
            "assimilation": "if_we_apply",
            "accommodation": "if_we_reconcile",
            "replay": "if_we_replay",
        }[phase]
        joined = "+".join(topics) if topics else "untitled"
        ctx_token = ctx.get("topic") or ctx.get("scope") or "ctx"
        return f"[{head}|{ctx_token}|{joined}|{idx}]"

    @staticmethod
    def _compute_cid(
        *,
        premise_nids: Tuple[str, ...],
        scenario: str,
        phase: str,
        claims: Tuple[str, ...] = (),
        bindings: Tuple[Tuple[str, str], ...] = (),
        context: Optional[Dict[str, Any]] = None,
    ) -> str:
        """确定性指纹 (V3 守门: dedupe 可靠)."""
        canonical = json.dumps(
            {
                "n": list(premise_nids), "s": scenario, "p": phase,
                "c": list(claims), "b": [list(b) for b in bindings],
                "x": context or {},
            },
            ensure_ascii=False, sort_keys=True, separators=(",", ":"),
        )
        return "dream-" + hashlib.sha256(canonical.encode("utf-8")).hexdigest()[:16]

    @staticmethod
    def _cand_to_episode(cand: DreamCandidateV2) -> Dict[str, Any]:
        """V1107 集成: DreamCandidateV2 → Dict (EpisodeBuffer 可消费)."""
        # V3 守门: _dream=True 永远
        return {
            "episode_id": f"dreamep_{cand.cid[:24]}",
            "content": {
                "topic": cand.schema_phase,
                "claim": cand.scenario,
                "_dream": cand._dream,           # V3 守门
                "source_dream_cid": cand.cid,
                "state_at_birth": cand.state_at_birth,
            },
            "salience": min(0.7, 0.3 + 0.05 * (len(cand.scenario) // 10)),
            "confidence": min(0.7, cand.confidence),
            "source": "dream",
        }


@dataclass
class DreamV2Result:
    """dream() 返回结构 (主 00:56)."""
    candidates: List[DreamCandidateV2]
    episodes: List[Dict[str, Any]]
    transitions: List[Tuple[DreamState, DreamState, float, str]]
    stats: Dict[str, Any]
    final_state: DreamState


# ============================================================================
# 5. V1108 V3 哲学守门 (主 17:58 + 主 20:46)
# ============================================================================

V1108_V3_GUARDS = {
    "dream_is_not_consciousness": (
        "不假装 dream = consciousness. "
        "6 状态机 = 状态机 (Hopcroft 1979) ≠ 心理状态. "
        "睡眠比喻 ≠ 真意识."
    ),
    "dream_is_not_understanding": (
        "不假装 dream = understanding. "
        "V1092/V1108 heuristic re-composition ≠ 概念理解."
    ),
    "state_machine_is_not_psychology": (
        "不假装 6 状态机 = 真实睡眠周期. "
        "IDLE/DREAMING/CONSOLIDATING/FORGETTING/VERIFYING/INTERRUPTED 是 FSM."
    ),
    "module_is_not_asi": (
        "不假装 V1108 = ASI. V1108 是工具, ASI 是更大目标. "
        "dream 增强 ≠ ASI 达成."
    ),
    "dream_fact": (
        "不假装 dream = fact. _dream=True 永远. "
        "V1107 DreamEpisode adapter cap confidence ≤ 0.7."
    ),
    "frozen_immutability": (
        "不假装 frozen = safety. V3 守门: DreamCandidateV2 frozen=True, "
        "任何字段不可改. 防止意外混入事实流."
    ),
}


__all__ = [
    "V1108_VERSION",
    "DREAM_IS_NOT_CONSCIOUSNESS",
    "DreamState",
    "DreamCandidateV2",
    "DreamStateMachine",
    "DreamV2Stats",
    "MemoryDreamV2",
    "DreamV2Result",
    "V1108_V3_GUARDS",
]


# V1101 auto-injected V3_GUARDS (主 17:43 实事求是 + 主 17:58 不假装)
V3_GUARDS = {"module_is_not_asi": "模块是工具, ASI 是更大目标. 任何声称模块 = ASI 的部分都是不假装.", "measurement_is_not_truth": "测量是 proxy, 真值仍是更大目标. V1077 真测 17 维 ≠ ASI 达成.", "structure_is_not_consciousness": "CognitiveArchitecture 结构类比 ≠ 现象意识. ACT-R chunks ≠ concepts.", "production_is_not_safety": "真生产 ≠ 真安全. 部署 ≠ 守门. 任何声称 production = safe 是不假装.", "automation_is_not_autonomy": "自动执行 ≠ 自主意识. V1101 lift 引擎自动改 ≠ V1101 自主."}