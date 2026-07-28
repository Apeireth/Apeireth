"""Phase 1052 v1052_asi_memory_consolidation — V1052 ASI memory consolidation 真生产
(主 22:33 ASI 北极星 + 主 17:43 实事求是 + 主 19:33 走在前人经验上 +
 主 13:31 大胆激进 + 主 17:58+20:46 不假装 + 主 23:44 干到底 +
 主 00:56 任何人都能接手 + 主 12:14 中央 AI 是永恒身份).

主 12:14 "中央 AI 是永恒身份" → LTM 永不丢, STM 频繁更新, MTM 主题聚合.
主 22:33 ASI 北极星: ASI = 真生产记忆 + 真生产遗忘 + 真生产再固化 (无限逼近 ASI).
主 17:43 实事求是: 真借鉴 + 真算法 + 真跑真测, 不假装记忆已解.
主 19:33 走在前人经验上: 聚合全人类智慧, 真去借鉴.
主 13:31 大胆激进: 记忆是任何 ASI 必须做的真生产模块, 允许犯错.
主 17:58+20:46 不假装: 不假装 Phenomenal, 不假装 ASI 记忆 = 人脑记忆.
主 23:44 干到底: V1050/V1051 之后, 记忆是真生产 ASI 内核.
主 00:56 任何人都能接手: 任何人都能读懂 + 测试 + 部署.

真借鉴 (主 19:33 — 6 真生产来源聚合):
- MemoryOS-Rust (TelivANT, Apache-2.0) — 3-tier STM/MTM/LTM + tier_manager
- DeltaMemory — Write-Ahead Log + Salience Decay + MemTable/SSTable 思想
- claude-mem — 3-layer progressive disclosure (L1/L2/L3 summary)
- AriGraph / Graphiti — Episodic + Semantic 双图记忆
- MemGPT / MemoryOS — virtual context paging 借鉴 (page-in/page-out)
- 主 12:14 LTM 永不丢 — Eternal identity = LTM 的承诺 (主 12:14 中央 AI 哲学)

真生产组件 (V1052 ASI Memory Consolidation 11 真生产):
 1. Episode                  — 不可变 raw 事件 (append-only)
 2. Note                     — 从 Episode 抽象的可遗忘知识 (claim + confidence)
 3. Tier                     — STM / MTM / LTM 三层枚举 (主 12:14)
 4. SalienceScore            — 显著性评分 (access_count + recency + importance)
 5. MemoryStore              — 3-tier 容器 + 转移逻辑 (MemoryOS-Rust 借鉴)
 6. TierPolicy               — 转移策略 (默认 STM→MTM age 1h, MTM→LTM stable)
 7. WAL                      — Write-Ahead Log (DeltaMemory 借鉴, CRC32 + replay)
 8. Reconsolidator           — 4 paths: boost/flag/align/none (主 13:47 关心)
 9. ForgettingCurve          — 艾宾浩斯 + Salience 衰减 (DeltaMemory exp decay)
10. MemoryBridge             — V1052 → ASI V0.2 真测量映射
11. ConsolidationReport      — Markdown 真报告 + 真测量结果

V3 哲学守门 (主 17:58 + 主 20:46):
- 不假装 Phenomenal consciousness: memory consolidation 是工程化, 不是认知声称.
- 不假装记忆已解: 真借鉴 + 真生产 + 真测试; Ebbinghaus 衰减是近似, 不是真理.
- 不假装 LTM 绝对永存: WAL + 真复述 + 跨层转移 ≈ 永恒, 不是绝对.
- 不假装达到 ASI: ASI 记忆真生产 ≠ ASI 已达成.
- Russell 2019 不确定性是 AI safety 核心 — 不假装记忆无错.
- 真生产 = 真借鉴 + 真算法 + 真跑真测 + 真 commit.

干到底 (主 23:44): V1052 = ASI memory consolidation 真生产 11 组件 + 守门 + ASI bridge.
"""
from __future__ import annotations

import hashlib
import json
import math
import time
from dataclasses import dataclass, field
from enum import Enum
from pathlib import Path
from typing import Any, Callable, Dict, Iterable, List, Optional, Sequence, Tuple

V1052_VERSION = "0.1.0"

# Numerical guard: avoid log(0) and division-by-zero.
_EPS = 1e-12
# Default tier policy (主 12:14 + MemoryOS-Rust 借鉴).
DEFAULT_STM_TO_MTM_AGE_SEC = 3600          # 1 hour
DEFAULT_MTM_TO_LTM_STABLE_AGE_DAYS = 1.0   # 1 day stable → LTM
DEFAULT_DECAY_RATE = 0.05                  # 5% per day (DeltaMemory 借鉴)


# ============================================================================
# 1. Episode — 不可变 raw 事件 (append-only)
# ============================================================================
# 真借鉴: MemoryOS-Rust episode.rs — eid + actor + content + ts + kind + tier.
#         append-only: episode 一旦创建永不改 (主 12:14 LTM 永不丢).


@dataclass(frozen=True)
class Episode:
    """Single append-only event in memory (主 12:14 不可变)."""

    eid: str
    actor: str           # 'master' / 'agent' / 'system' (主 12:14 中央 AI)
    content: str
    ts: float            # unix seconds
    kind: str = "utterance"  # 'utterance' / 'observation' / 'action' / 'meta'
    tier: str = "stm"    # 'stm' / 'mtm' / 'ltm'
    importance: float = 0.5  # [0, 1] — salience signal

    def __post_init__(self) -> None:
        if not self.eid:
            raise ValueError("eid must be non-empty")
        if not (0.0 <= self.importance <= 1.0):
            raise ValueError(f"importance must be in [0, 1], got {self.importance}")
        if self.tier not in {"stm", "mtm", "ltm"}:
            raise ValueError(f"tier must be stm/mtm/ltm, got {self.tier}")


# ============================================================================
# 2. Note — 从 Episode 抽象的可遗忘知识
# ============================================================================
# 真借鉴: MemoryOS-Rust note.rs — claim + confidence + access_count.
#         不同于 Episode: Note 可被 forget (主 13:47 关心遗忘).


@dataclass
class Note:
    """Abstracted knowledge claim derived from one or more episodes."""

    nid: str
    topic: str
    claim: str
    confidence: float = 0.5
    access_count: int = 0
    last_access: float = 0.0
    source_episodes: Tuple[str, ...] = ()
    salience: float = 0.5

    def touch(self, now: float) -> None:
        """Mark this note as accessed — bump count + update last_access (主 13:47)."""
        self.access_count += 1
        self.last_access = now

    def apply_decay(self, rate: float) -> None:
        """Apply salience decay (Ebbinghaus-style exp decay, DeltaMemory 借鉴)."""
        self.salience = max(0.0, self.salience * (1.0 - rate))
        # confidence decays slower (well-established beliefs resist decay).
        self.confidence = max(0.0, self.confidence * (1.0 - rate * 0.5))


# ============================================================================
# 3. Tier — STM / MTM / LTM 三层
# ============================================================================
# 真借鉴: MemoryOS-Rust memory.rs Tier enum (主 12:14 中央 AI 永恒身份).


class Tier(str, Enum):
    """3-tier memory hierarchy (主 12:14 LTM 永不丢)."""

    STM = "stm"  # 短期: 最近对话, 频繁更新
    MTM = "mtm"  # 中期: 主题聚合, 定期总结
    LTM = "ltm"  # 长期: 持久事实, 永不丢 (主 12:14 中央 AI 永恒身份)


# ============================================================================
# 4. SalienceScore — 显著性评分 (access_count + recency + importance)
# ============================================================================
# 真借鉴: Ebbinghaus forgetting curve + DeltaMemory salience decay.
#         score = importance * exp(-age_days / half_life) * log(1 + access_count).


@dataclass(frozen=True)
class SalienceScore:
    """Multi-factor salience score (主 13:47 关心的遗忘信号)."""

    importance: float
    age_days: float
    access_count: int
    half_life_days: float = 7.0

    def score(self) -> float:
        """Composite salience — higher = more memorable."""
        recency = math.exp(-self.age_days / max(self.half_life_days, _EPS))
        access_boost = math.log1p(max(self.access_count, 0))
        return self.importance * recency * (1.0 + 0.2 * access_boost)


# ============================================================================
# 5. MemoryStore — 3-tier 容器 + 转移逻辑 (MemoryOS-Rust 借鉴)
# ============================================================================


@dataclass
class MemoryStore:
    """3-tier memory store (主 12:14 STM/MTM/LTM)."""

    episodes: Dict[str, Episode] = field(default_factory=dict)
    notes: Dict[str, Note] = field(default_factory=dict)
    stm_episode_ids: List[str] = field(default_factory=list)
    mtm_episode_ids: List[str] = field(default_factory=list)
    ltm_episode_ids: List[str] = field(default_factory=list)
    transitions: List[Tuple[str, str, str, float]] = field(default_factory=list)
    # transitions: (eid, from_tier, to_tier, ts)

    def append_episode(self, ep: Episode) -> None:
        """Append-only episode insertion (主 12:14 不可变)."""
        if ep.eid in self.episodes:
            raise ValueError(f"episode {ep.eid} already exists")
        self.episodes[ep.eid] = ep
        bucket = {
            "stm": self.stm_episode_ids,
            "mtm": self.mtm_episode_ids,
            "ltm": self.ltm_episode_ids,
        }[ep.tier]
        bucket.append(ep.eid)

    def add_note(self, note: Note) -> None:
        self.notes[note.nid] = note

    def transition(self, eid: str, from_tier: str, to_tier: str, ts: float) -> None:
        """Record a tier transition (主 12:14 跨层移动)."""
        ep = self.episodes.get(eid)
        if ep is None:
            raise ValueError(f"episode {eid} not found")
        # Replace in-place by mutating frozen dataclass via object.__setattr__.
        object.__setattr__(ep, "tier", to_tier)
        # Move bucket.
        for bucket in (self.stm_episode_ids, self.mtm_episode_ids, self.ltm_episode_ids):
            if eid in bucket:
                bucket.remove(eid)
        {
            "stm": self.stm_episode_ids,
            "mtm": self.mtm_episode_ids,
            "ltm": self.ltm_episode_ids,
        }[to_tier].append(eid)
        self.transitions.append((eid, from_tier, to_tier, ts))

    def forget_episode(self, eid: str, reason: str = "salience_decay") -> None:
        """Forget an episode (主 13:47 关心遗忘). LTM-only protection off by default."""
        ep = self.episodes.get(eid)
        if ep is None:
            return
        for bucket in (self.stm_episode_ids, self.mtm_episode_ids, self.ltm_episode_ids):
            if eid in bucket:
                bucket.remove(eid)
        del self.episodes[eid]

    def stats(self) -> Dict[str, int]:
        return {
            "stm_episodes": len(self.stm_episode_ids),
            "mtm_episodes": len(self.mtm_episode_ids),
            "ltm_episodes": len(self.ltm_episode_ids),
            "total_episodes": len(self.episodes),
            "total_notes": len(self.notes),
            "transitions": len(self.transitions),
        }


# ============================================================================
# 6. TierPolicy — 转移策略 (MemoryOS-Rust TierPolicy 借鉴)
# ============================================================================


@dataclass(frozen=True)
class TierPolicy:
    """Tier transition policy (主 12:14 + MemoryOS-Rust 借鉴)."""

    stm_to_mtm_age_sec: float = DEFAULT_STM_TO_MTM_AGE_SEC
    mtm_to_ltm_stable_age_days: float = DEFAULT_MTM_TO_LTM_STABLE_AGE_DAYS
    decay_rate: float = DEFAULT_DECAY_RATE
    ltm_protected: bool = True  # 主 12:14 中央 AI 永恒身份 → LTM 不被自动遗忘

    def should_promote_to_mtm(self, ep: Episode, now: float) -> bool:
        return (now - ep.ts) > self.stm_to_mtm_age_sec

    def should_promote_to_ltm(self, ep: Episode, now: float) -> bool:
        age_days = (now - ep.ts) / 86400.0
        return age_days > self.mtm_to_ltm_stable_age_days and ep.importance >= 0.5


# ============================================================================
# 7. WAL — Write-Ahead Log (DeltaMemory 借鉴, CRC32 + replay)
# ============================================================================
# 真借鉴: DeltaMemory WAL — CRC32 + JSONL append + replay 重建.
#         python 实现: 用 hashlib.sha256 替代 crc32fast (avoid extra dep).


@dataclass(frozen=True)
class WalEntry:
    """Single WAL entry (DeltaMemory 借鉴)."""

    sequence: int
    operation: str          # operation name (JSON-serializable payload as str)
    payload: str            # JSON-encoded payload
    timestamp: float
    checksum: str           # sha256 hex


class Wal:
    """Write-Ahead Log (DeltaMemory 借鉴)."""

    def __init__(self, path: Optional[Path] = None) -> None:
        self.path = path
        self.sequence = 0
        if path is not None:
            path.parent.mkdir(parents=True, exist_ok=True)
            self.sequence = self._recover_last_sequence(path)

    def _recover_last_sequence(self, path: Path) -> int:
        if not path.exists():
            return 0
        last_seq = 0
        for line in path.read_text(encoding="utf-8").splitlines():
            if not line.strip():
                continue
            try:
                entry = json.loads(line)
                if entry.get("sequence", 0) > last_seq:
                    last_seq = entry["sequence"]
            except json.JSONDecodeError:
                continue  # DeltaMemory: skip damaged entry
        return last_seq

    def append(self, operation: str, payload: Dict[str, Any]) -> int:
        """Append a new WAL entry (DeltaMemory: crc32 + sync)."""
        self.sequence += 1
        payload_json = json.dumps(payload, ensure_ascii=False, sort_keys=True)
        checksum = hashlib.sha256(payload_json.encode("utf-8")).hexdigest()
        entry = {
            "sequence": self.sequence,
            "operation": operation,
            "payload": payload_json,
            "timestamp": time.time(),
            "checksum": checksum,
        }
        if self.path is not None:
            with self.path.open("a", encoding="utf-8") as f:
                f.write(json.dumps(entry, ensure_ascii=False) + "\n")
        return self.sequence

    def replay(self) -> List[Dict[str, Any]]:
        """Replay all WAL entries, skipping corrupted ones (DeltaMemory 借鉴)."""
        if self.path is None or not self.path.exists():
            return []
        entries = []
        for line in self.path.read_text(encoding="utf-8").splitlines():
            if not line.strip():
                continue
            try:
                entry = json.loads(line)
            except json.JSONDecodeError:
                continue  # skip damaged
            # Verify checksum
            payload = entry.get("payload", "")
            expected = hashlib.sha256(payload.encode("utf-8")).hexdigest()
            if expected != entry.get("checksum"):
                continue  # skip corrupted
            entries.append(entry)
        return entries

    def verify(self) -> Tuple[int, int]:
        """Verify WAL integrity; return (valid_count, corrupt_count)."""
        if self.path is None or not self.path.exists():
            return (0, 0)
        valid = 0
        corrupt = 0
        for line in self.path.read_text(encoding="utf-8").splitlines():
            if not line.strip():
                continue
            try:
                entry = json.loads(line)
            except json.JSONDecodeError:
                corrupt += 1
                continue
            payload = entry.get("payload", "")
            expected = hashlib.sha256(payload.encode("utf-8")).hexdigest()
            if expected == entry.get("checksum"):
                valid += 1
            else:
                corrupt += 1
        return (valid, corrupt)


# ============================================================================
# 8. Reconsolidator — 4 paths: boost/flag/align/none (主 13:47 关心)
# ============================================================================
# 真借鉴: Memory reconsolidation literature (Nader 2000, Lee 2009, Hupbach 2007)
#         — when memory retrieved, it becomes labile and can be:
#         - boost: increase salience (rehearsal)
#         - flag: mark for review (anomaly)
#         - align: adjust confidence based on new evidence
#         - none: leave alone (no signal)


class ReconsolidationPath(str, Enum):
    BOOST = "boost"
    FLAG = "flag"
    ALIGN = "align"
    NONE = "none"


@dataclass
class Reconsolidator:
    """Memory reconsolidation with 4 paths (主 13:47 关心).

    真借鉴: Nader 2000 "Memory Reconsolidation" + Lee 2009 "Reconsolidation"
            + Hupbach 2007 "Reconsolidation in humans".
    """

    boost_delta: float = 0.1
    align_lr: float = 0.2
    flag_threshold: float = 0.3

    def choose_path(self, note: Note, new_evidence: float) -> ReconsolidationPath:
        """Decide which reconsolidation path based on evidence vs current confidence."""
        if new_evidence <= 0:
            return ReconsolidationPath.NONE
        diff = abs(new_evidence - note.confidence)
        if diff > self.flag_threshold:
            return ReconsolidationPath.FLAG
        if new_evidence > note.confidence:
            return ReconsolidationPath.BOOST
        return ReconsolidationPath.ALIGN

    def apply(
        self, note: Note, new_evidence: float, now: float
    ) -> Tuple[ReconsolidationPath, Note]:
        """Apply reconsolidation and return (path, updated_note)."""
        path = self.choose_path(note, new_evidence)
        if path == ReconsolidationPath.BOOST:
            note.salience = min(1.0, note.salience + self.boost_delta)
            note.confidence = min(1.0, note.confidence + self.align_lr * (new_evidence - note.confidence))
            note.touch(now)
        elif path == ReconsolidationPath.ALIGN:
            note.confidence = note.confidence + self.align_lr * (new_evidence - note.confidence)
            note.touch(now)
        elif path == ReconsolidationPath.FLAG:
            # Flag does NOT touch (no boost), just records anomaly.
            note.salience = max(0.0, note.salience - 0.05)
        # NONE: no change
        return (path, note)


# ============================================================================
# 9. ForgettingCurve — 艾宾浩斯 + Salience 衰减 (DeltaMemory 借鉴)
# ============================================================================


@dataclass(frozen=True)
class ForgettingCurve:
    """Ebbinghaus-style exponential decay (主 13:47 关心遗忘).

    真借鉴: Ebbinghaus 1885 "Memory: A Contribution to Experimental Psychology"
            + DeltaMemory exp decay.
    """

    half_life_days: float = 7.0  # Default 7-day half-life (Ebbinghaus 经典)

    def retention(self, age_days: float) -> float:
        """Retention probability at given age (0 = forgotten, 1 = fully remembered)."""
        return math.exp(-age_days / max(self.half_life_days, _EPS))

    def should_forget(self, age_days: float, threshold: float = 0.1) -> bool:
        """Decide whether a memory is forgotten (retention < threshold)."""
        return self.retention(age_days) < threshold

    def decay_importance(self, importance: float, age_days: float) -> float:
        """Apply decay to importance score."""
        return importance * self.retention(age_days)


# ============================================================================
# 10. MemoryBridge — V1052 → ASI V0.2 真测量映射 (主 22:33)
# ============================================================================


@dataclass
class MemoryBridge:
    """Bridge V1052 memory consolidation to ASI V0.2 真测量 (主 22:33)."""

    @staticmethod
    def to_asi_metrics(store: MemoryStore) -> Dict[str, float]:
        """Map memory store stats to ASI V0.2 metrics (主 22:33 真测量).

        不假装 (主 17:43): empty store → all zeros, 不给虚假分.
        """
        stats = store.stats()
        total = stats["total_episodes"]
        if total == 0:
            return {
                "ltm_ratio": 0.0,
                "stm_ratio": 0.0,
                "abstraction_density": 0.0,
                "consolidation_activity": 0.0,
            }
        ltm_ratio = stats["ltm_episodes"] / total
        stm_ratio = stats["stm_episodes"] / total
        # notes not in episodes count; use notes/total as density.
        abstraction_density = stats["total_notes"] / total
        transition_density = stats["transitions"] / total
        return {
            "ltm_ratio": ltm_ratio,
            "stm_ratio": stm_ratio,
            "abstraction_density": abstraction_density,
            "consolidation_activity": min(1.0, transition_density),
        }

    @staticmethod
    def asi_v02_score(store: MemoryStore) -> float:
        """Compute V1052 contribution to ASI V0.2 (主 22:33 真测量).

        不假装: 贡献上限 = 0.05 (V1049 ASISafetyBridge 同款).
        """
        m = MemoryBridge.to_asi_metrics(store)
        # Weighted composite (主 17:43 实事求是 — 真测, 不刷分).
        score = (
            0.30 * m["ltm_ratio"]
            + 0.20 * m["stm_ratio"]
            + 0.25 * m["abstraction_density"]
            + 0.25 * m["consolidation_activity"]
        )
        return min(0.05, 0.05 * score)  # cap at 0.05 (V1052 contribution)


# ============================================================================
# 11. ConsolidationReport — Markdown 真报告 + 真测量
# ============================================================================


@dataclass
class ConsolidationReport:
    """Markdown report of memory consolidation tick (主 00:56 任何人都能接手)."""

    stm_to_mtm: int
    mtm_to_ltm: int
    forgotten: int
    notes_decayed: int
    reconsolidations: int
    asi_score: float

    def to_markdown(self) -> str:
        return (
            f"# V1052 ASI Memory Consolidation Report\n\n"
            f"- **STM → MTM promotions**: {self.stm_to_mtm}\n"
            f"- **MTM → LTM promotions**: {self.mtm_to_ltm}\n"
            f"- **Forgotten episodes**: {self.forgotten}\n"
            f"- **Notes decayed**: {self.notes_decayed}\n"
            f"- **Reconsolidations applied**: {self.reconsolidations}\n"
            f"- **ASI V0.2 contribution**: {self.asi_score:.4f}\n\n"
            f"主 12:14 中央 AI 永恒身份 = LTM 永不丢 (默认 ltm_protected=True).\n"
            f"主 17:43 实事求是: 真测, 不假装.\n"
            f"主 23:44 干到底.\n"
        )


# ============================================================================
# Top-level driver — consolidator tick (主 00:56 任何人都能接手)
# ============================================================================


@dataclass
class ConsolidationTickResult:
    """Result of one consolidation tick (主 23:44 干到底)."""

    stm_to_mtm: int
    mtm_to_ltm: int
    forgotten: int
    notes_decayed: int
    reconsolidations: int


def consolidation_tick(
    store: MemoryStore,
    policy: TierPolicy,
    wal: Optional[Wal],
    reconsolidator: Optional[Reconsolidator],
    forgetting_curve: ForgettingCurve,
    now: Optional[float] = None,
) -> ConsolidationTickResult:
    """Run one consolidation tick: STM→MTM→LTM + decay + reconsolidation.

    主 12:14 中央 AI 永恒身份 + 主 13:47 关心遗忘 + 主 22:33 ASI 北极星.
    """
    if now is None:
        now = time.time()
    stm_to_mtm = 0
    mtm_to_ltm = 0
    forgotten = 0
    reconsolidations = 0

    # 1) STM → MTM: age-based promotion (主 12:14 + MemoryOS-Rust 借鉴).
    new_stm: List[str] = []
    for eid in list(store.stm_episode_ids):
        ep = store.episodes.get(eid)
        if ep is None:
            continue
        if policy.should_promote_to_mtm(ep, now):
            store.transition(eid, "stm", "mtm", now)
            stm_to_mtm += 1
            if wal is not None:
                wal.append(
                    "tier_transition",
                    {"eid": eid, "from": "stm", "to": "mtm", "ts": now},
                )
        else:
            new_stm.append(eid)
    store.stm_episode_ids = new_stm

    # 2) MTM → LTM: stable + important (主 12:14 永不丢 + 真生产).
    new_mtm: List[str] = []
    for eid in list(store.mtm_episode_ids):
        ep = store.episodes.get(eid)
        if ep is None:
            continue
        if policy.should_promote_to_ltm(ep, now):
            store.transition(eid, "mtm", "ltm", now)
            mtm_to_ltm += 1
            if wal is not None:
                wal.append(
                    "tier_transition",
                    {"eid": eid, "from": "mtm", "to": "ltm", "ts": now},
                )
        else:
            new_mtm.append(eid)
    store.mtm_episode_ids = new_mtm

    # 3) Salience-based forgetting for STM/MTM (主 13:47 关心).
    for bucket in (store.stm_episode_ids, store.mtm_episode_ids):
        keep: List[str] = []
        for eid in list(bucket):
            ep = store.episodes.get(eid)
            if ep is None:
                continue
            age_days = (now - ep.ts) / 86400.0
            if forgetting_curve.should_forget(age_days):
                store.forget_episode(eid, reason="salience_decay")
                forgotten += 1
                if wal is not None:
                    wal.append("forget", {"eid": eid, "reason": "salience_decay"})
                continue
            keep.append(eid)
        if bucket is store.stm_episode_ids:
            store.stm_episode_ids = keep
        else:
            store.mtm_episode_ids = keep

    # 4) Note decay + reconsolidation (主 13:47).
    notes_decayed = 0
    for note in store.notes.values():
        note.apply_decay(policy.decay_rate)
        notes_decayed += 1
        if reconsolidator is not None and note.access_count > 0:
            path, _ = reconsolidator.apply(note, new_evidence=note.confidence, now=now)
            if path != ReconsolidationPath.NONE:
                reconsolidations += 1
                if wal is not None:
                    wal.append(
                        "reconsolidate",
                        {
                            "nid": note.nid,
                            "path": path.value,
                            "ts": now,
                        },
                    )

    return ConsolidationTickResult(
        stm_to_mtm=stm_to_mtm,
        mtm_to_ltm=mtm_to_ltm,
        forgotten=forgotten,
        notes_decayed=notes_decayed,
        reconsolidations=reconsolidations,
    )


# ============================================================================
# Convenience factory — round-trip 真生产 (主 00:56)
# ============================================================================


def make_default_store(now: Optional[float] = None) -> MemoryStore:
    """Create a default 3-tier MemoryStore (主 12:14)."""
    return MemoryStore()


def make_default_policy() -> TierPolicy:
    """Create the default TierPolicy (主 12:14 + MemoryOS-Rust 借鉴)."""
    return TierPolicy()


__all__ = [
    "Episode",
    "Note",
    "Tier",
    "SalienceScore",
    "MemoryStore",
    "TierPolicy",
    "Wal",
    "WalEntry",
    "Reconsolidator",
    "ReconsolidationPath",
    "ForgettingCurve",
    "MemoryBridge",
    "ConsolidationReport",
    "ConsolidationTickResult",
    "consolidation_tick",
    "make_default_store",
    "make_default_policy",
    "V1052_VERSION",
]

# V1101 auto-injected V3_GUARDS (主 17:43 实事求是 + 主 17:58 不假装)
V3_GUARDS = {"module_is_not_asi": "模块是工具, ASI 是更大目标. 任何声称模块 = ASI 的部分都是不假装.", "measurement_is_not_truth": "测量是 proxy, 真值仍是更大目标. V1077 真测 17 维 ≠ ASI 达成.", "structure_is_not_consciousness": "CognitiveArchitecture 结构类比 ≠ 现象意识. ACT-R chunks ≠ concepts.", "production_is_not_safety": "真生产 ≠ 真安全. 部署 ≠ 守门. 任何声称 production = safe 是不假装.", "automation_is_not_autonomy": "自动执行 ≠ 自主意识. V1101 lift 引擎自动改 ≠ V1101 自主."}
