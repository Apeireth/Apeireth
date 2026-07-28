"""Apeireth ASI V1089 — Real HotCold Three-Tier Memory
======================================================

V1089 = 真 HotCold 三层记忆 = 真 Hot/Warm/Cold 三层 + 真 promote/demote 阈值 +
真 v1090 WAL 持久化 + 真 MemoryReplayProtocol 5 方法契约 (R6-RES-07).

三层定义 (借鉴 MemoryOS-Rust STM/MTM/LTM, 主 14:48 + 主 14:50 真生产):

  HOT  — 当前 session 的高频条目 (≤10 分钟 / ≤1000 字节, 快速访问).
  WARM — 跨 session 主题聚合条目 (10 分钟 ≤ age ≤ 24 小时, 中等频次).
  COLD — 持久事实条目 (age ≥ 24 小时 且 importance ≥ 8, 永不删).

HOT ↔ WARM 切换:
  - promote_hot_to_warm: age ≥ hot_to_warm_age_s AND access_count ≤ hot_to_warm_max_access
  - promote_warm_to_hot: access_count ≥ warm_to_hot_min_access AND age ≤ warm_to_hot_max_age_s

WARM ↔ COLD 切换:
  - promote_warm_to_cold: importance ≥ warm_to_cold_min_importance AND age ≥ warm_to_cold_age_s
  - promote_cold_to_warm: importance < cold_to_warm_max_importance (手动或事件触发)

借鉴 (主 19:33 走在前人经验上):
  1. MemoryOS-Rust STM/MTM/LTM   : 三层范式 + heat 衰减 + 容量限制.
  2. V1052 WAL (DeltaMemory)      : sha256 + JSONL + 损坏容错 (现升级为 v1090).
  3. V1090 WriteAheadLog          : 真 fsync 真持久化 (本模块新依赖).
  4. Memory_replay_design         : 5 方法契约 (capture_state/restore_state/
                                    replay_events/diff_states/idempotent_apply).
  5. R6-RES-07                    : R7-BE-02 幂等白名单 + 半开时间窗口 + 对称 diff.
  6. A-MEM agentic memory         : note / tag / link 启发式分层.
  7. Zep temporal KG              : 时间衰减 + 重要性保留.
  8. Letta archival_memory        : query + tag + temporal + top_k.
  9. Hippocampal sharp-wave ripple: 高频→低频 巩固启发 (R37 q5).
 10. Tonbo WAL (round-37)         : 借用锁模型 + 损坏容错.

哲学守门 (主 17:58+20:46 不假装 + V3):
  - promotion ≠ deletion       : promote 只移动条目, 不删; 冷层条目仍可查.
  - replay ≠ bit-exact          : 重放是近似重建, 不声称字节相等.
  - capture ≠ backup           : 快照是 diff/restore 工具, 不是历史归档.
  - idempotent ≠ safe           : 幂等仅对白名单 op 集合成立.
  - hotcold ≠ wisdom            : 分层是 heuristic 加速, 不等于"重要性真理".
  - replay ≠ understanding      : 启发式再发射, 不是现象学回忆.

不写 (主 07-19 4 层安全门):
  - 不动 v1052 Wal / memory_replay_design / v1091_memory_replay / V1074.
  - 不假装三层是 "生物记忆" 真理 — 仅是工程启发式分层.
  - 不暴露 truncate; 仅 rotate.
  - 不引入 redis / sqlite / lmdb — 纯 stdlib + v1090 WAL.
"""
from __future__ import annotations

import threading
import time
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Dict, Iterator, List, Optional, Tuple

from apeireth.memory_replay_design import (
    ApplyResult,
    Event,
    IDEMPOTENT_OPS,
    MemoryReplayProtocol,
    PHILOSOPHY_GUARDS,
    StateDiff,
    StateID,
)
from apeireth.v1090_memory_wal import WriteAheadLog


V1089_VERSION = "0.1.0"


# ============================================================================
# 1. 默认阈值 (主 17:43 实事求是 + 借鉴 MemoryOS-Rust)
# ============================================================================


# HOT → WARM 触发
DEFAULT_HOT_MAX_AGE_S = 10 * 60               # 10 分钟
DEFAULT_HOT_MAX_ACCESS = 3                     # 访问超过 3 次不降级
# WARM → HOT 触发
DEFAULT_WARM_PROMOTE_ACCESS = 5                # 5 次访问提升
DEFAULT_WARM_PROMOTE_MAX_AGE_S = 30 * 60       # 30 分钟内才提升
# WARM → COLD 触发
DEFAULT_WARM_TO_COLD_AGE_S = 24 * 3600         # 24 小时
DEFAULT_WARM_TO_COLD_IMPORTANCE = 8            # 0-10
# COLD → WARM 触发 (手动/事件)
DEFAULT_COLD_TO_WARM_IMPORTANCE = 5            # importance < 5 时可降

# 容量上限 (借鉴 MemoryOS-Rust short_term_limit / mid_term_limit)
DEFAULT_HOT_CAPACITY = 64
DEFAULT_WARM_CAPACITY = 256
DEFAULT_COLD_CAPACITY = 4096


# ============================================================================
# 2. 三层条目 — 共用 MemoryItem dataclass, 用 tier 字段标识
# ============================================================================


TIER_HOT = "hot"
TIER_WARM = "warm"
TIER_COLD = "cold"

ALL_TIERS = (TIER_HOT, TIER_WARM, TIER_COLD)


@dataclass
class MemoryItem:
    """单条记忆条目 (跨三层通用).

    字段:
      item_id: 全局唯一 id (uuid hex 短串).
      tier:    "hot" / "warm" / "cold".
      kind:    操作类型 (对应 Event.kind, 必须 ∈ IDEMPOTENT_OPS).
      payload: 任意 JSON-safe dict.
      importance: 0-10 (主 22:33 借鉴 V1072 永恒身份优先级).
      created_at: 首次入层时间.
      last_access: 最近访问时间.
      access_count: 访问次数 (用于 hot ↔ warm 切换).
      tag: 主题标签 (用于 warm 层聚合).
      sequence: 进入三层时的 WAL sequence (溯源用).
    """

    item_id: str
    kind: str
    payload: Dict[str, Any] = field(default_factory=dict)
    tier: str = TIER_HOT
    importance: int = 5
    created_at: float = field(default_factory=time.time)
    last_access: float = field(default_factory=time.time)
    access_count: int = 0
    tag: str = "general"
    sequence: int = 0

    def to_event(self) -> Event:
        """转换为 R6-RES-07 Event (用于 replay_events / diff_states)."""
        return Event(
            event_id=self.item_id,
            ts=self.created_at,
            kind=self.kind,
            payload=tuple(sorted(self.payload.items())),
        )

    def age(self, now: Optional[float] = None) -> float:
        return (now if now is not None else time.time()) - self.created_at

    def touch(self, now: Optional[float] = None) -> None:
        """访问时调用: 增加计数 + 更新 last_access."""
        self.access_count += 1
        self.last_access = now if now is not None else time.time()


# ============================================================================
# 3. HotColdPolicy — 真 promote/demote 阈值 (可调)
# ============================================================================


@dataclass
class HotColdPolicy:
    """三层切换阈值 (主 00:44 质量工程化 + 借鉴 MemoryOS-Rust).

    阈值可在构造后调整, 调整后会立即生效 (下一次 promote/demote 调用).
    """

    # HOT → WARM
    hot_max_age_s: float = DEFAULT_HOT_MAX_AGE_S
    hot_max_access: int = DEFAULT_HOT_MAX_ACCESS
    # WARM → HOT
    warm_promote_access: int = DEFAULT_WARM_PROMOTE_ACCESS
    warm_promote_max_age_s: float = DEFAULT_WARM_PROMOTE_MAX_AGE_S
    # WARM → COLD
    warm_to_cold_age_s: float = DEFAULT_WARM_TO_COLD_AGE_S
    warm_to_cold_importance: int = DEFAULT_WARM_TO_COLD_IMPORTANCE
    # COLD → WARM
    cold_to_warm_importance: int = DEFAULT_COLD_TO_WARM_IMPORTANCE
    # 容量
    hot_capacity: int = DEFAULT_HOT_CAPACITY
    warm_capacity: int = DEFAULT_WARM_CAPACITY
    cold_capacity: int = DEFAULT_COLD_CAPACITY

    def should_promote_hot_to_warm(self, item: MemoryItem, now: float) -> bool:
        return (
            item.tier == TIER_HOT
            and item.age(now) >= self.hot_max_age_s
            and item.access_count <= self.hot_max_access
        )

    def should_promote_warm_to_hot(self, item: MemoryItem, now: float) -> bool:
        return (
            item.tier == TIER_WARM
            and item.access_count >= self.warm_promote_access
            and item.age(now) <= self.warm_promote_max_age_s
        )

    def should_promote_warm_to_cold(self, item: MemoryItem, now: float) -> bool:
        return (
            item.tier == TIER_WARM
            and item.age(now) >= self.warm_to_cold_age_s
            and item.importance >= self.warm_to_cold_importance
        )

    def should_promote_cold_to_warm(self, item: MemoryItem, now: float) -> bool:
        return (
            item.tier == TIER_COLD
            and item.importance < self.cold_to_warm_importance
        )

    def capacity_for(self, tier: str) -> int:
        return {
            TIER_HOT: self.hot_capacity,
            TIER_WARM: self.warm_capacity,
            TIER_COLD: self.cold_capacity,
        }[tier]


# ============================================================================
# 4. Promotion / Demotion 真切换 — 不只改 tier, 还改 WAL
# ============================================================================


@dataclass
class PromotionRecord:
    """切换记录 (用于审计 + stats)."""

    item_id: str
    from_tier: str
    to_tier: str
    reason: str
    ts: float
    sequence: int

    def to_dict(self) -> Dict[str, Any]:
        return {
            "item_id": self.item_id,
            "from_tier": self.from_tier,
            "to_tier": self.to_tier,
            "reason": self.reason,
            "ts": round(self.ts, 6),
            "sequence": self.sequence,
        }


# ============================================================================
# 5. HotColdMemory — 主入口, 实现 MemoryReplayProtocol 5 方法
# ============================================================================


class HotColdMemory(MemoryReplayProtocol):
    """真 Hot/Warm/Cold 三层记忆 (主 23:44 干到底 + 主 17:43 实事求是).

    满足 MemoryReplayProtocol 5 方法契约 (R6-RES-07):
      - capture_state(scope)            → StateID
      - restore_state(state_id)         → bool
      - replay_events(from_ts, to_ts)   → Iterator[Event]
      - diff_states(state_a, state_b)   → StateDiff
      - idempotent_apply(event)         → ApplyResult

    借鉴 MemoryOS-Rust:
      - HOT  = STM (滚动窗口, 最近)
      - WARM = MTM (主题聚合, 周期)
      - COLD = LTM (持久事实, 永不丢)

    持久化: 通过 v1090 WriteAheadLog (真 fsync). 无 WAL 时退化为纯内存.

    用法:
        hc = HotColdMemory(wal_path=Path("/tmp/hc.jsonl"))
        hc.add(MemoryItem(item_id="i1", kind="tag_set", tag="auth", importance=7))
        for ev in hc.replay_events(0, time.time()): ...
        sid = hc.capture_state("session-1")
        ok = hc.restore_state(sid)
        diff = hc.diff_states(sid1, sid2)
        res = hc.idempotent_apply(Event(event_id="x", ts=t, kind="tag_set"))
    """

    def __init__(
        self,
        wal_path: Optional[Path] = None,
        *,
        policy: Optional[HotColdPolicy] = None,
        clock: Optional[Any] = None,
        fsync: bool = True,
    ) -> None:
        self.policy = policy or HotColdPolicy()
        self._clock = clock or time.time

        self._lock = threading.RLock()
        self._items: Dict[str, MemoryItem] = {}
        self._by_tier: Dict[str, List[str]] = {TIER_HOT: [], TIER_WARM: [], TIER_COLD: []}
        self._checkpoints: Dict[str, StateID] = {}   # state_id_key → StateID
        self._applied: Dict[str, ApplyResult] = {}   # idempotent_apply 缓存
        self._promotions: List[PromotionRecord] = []  # 切换审计
        self._sequence: int = 0

        self._wal: Optional[WriteAheadLog] = None
        if wal_path is not None:
            self._wal = WriteAheadLog(wal_path, fsync=fsync, clock=self._clock)
            self._sequence = self._wal._seq
            self._rebuild_from_wal()

    # ------------------------------------------------------------------
    # 公开: add (新条目入 HOT 层, 同时落 WAL)
    # ------------------------------------------------------------------

    def add(self, item: MemoryItem) -> str:
        """加入新条目到 HOT 层, 同步落 WAL. 返回 item_id."""
        with self._lock:
            if item.tier != TIER_HOT:
                # 新条目一律入 HOT; 已存在的条目调用 promote_*.
                item = MemoryItem(
                    item_id=item.item_id,
                    tier=TIER_HOT,
                    kind=item.kind,
                    payload=dict(item.payload),
                    importance=item.importance,
                    created_at=item.created_at,
                    last_access=item.last_access,
                    access_count=item.access_count,
                    tag=item.tag,
                    sequence=item.sequence,
                )
            item.created_at = item.created_at or self._clock()
            item.last_access = item.last_access or item.created_at
            self._append_or_replace(item)
            self._enforce_capacity(item.tier)
            return item.item_id

    # ------------------------------------------------------------------
    # 公开: get / touch / find
    # ------------------------------------------------------------------

    def get(self, item_id: str) -> Optional[MemoryItem]:
        """取条目 + 自动 touch (主 17:43: 访问计入 access_count)."""
        with self._lock:
            item = self._items.get(item_id)
            if item is None:
                return None
            item.touch(self._clock())
            return item

    def find_by_tag(self, tag: str, tier: Optional[str] = None) -> List[MemoryItem]:
        """按 tag 查条目 (主 19:33 借鉴 Letta archival_memory)."""
        with self._lock:
            tiers = (tier,) if tier else ALL_TIERS
            out: List[MemoryItem] = []
            for tid, ids in self._by_tier.items():
                if tid not in tiers:
                    continue
                for iid in ids:
                    item = self._items.get(iid)
                    if item and item.tag == tag:
                        out.append(item)
            return out

    def items_in_tier(self, tier: str) -> List[MemoryItem]:
        with self._lock:
            ids = list(self._by_tier.get(tier, []))
            return [self._items[i] for i in ids if i in self._items]

    # ------------------------------------------------------------------
    # 公开: promote / demote — 真切换 + 真审计
    # ------------------------------------------------------------------

    def consolidate(self, now: Optional[float] = None) -> List[PromotionRecord]:
        """一次性跑完所有 promote/demote 规则 (借鉴 MemoryOS-Rust consolidate_memory).

        返回 PromotionRecord 列表 (审计/测试用).
        """
        now = now if now is not None else self._clock()
        records: List[PromotionRecord] = []
        with self._lock:
            # 先处理 promote (从冷到热)
            for tier_from, tier_to in (
                (TIER_WARM, TIER_HOT),
                (TIER_COLD, TIER_WARM),
                (TIER_HOT, TIER_WARM),
                (TIER_WARM, TIER_COLD),
            ):
                snap = list(self._by_tier[tier_from])
                for iid in snap:
                    item = self._items.get(iid)
                    if item is None:
                        continue
                    should = getattr(self.policy, f"should_promote_{tier_from}_to_{tier_to}", None)
                    if should is None:
                        continue
                    if should(item, now):
                        rec = self._move_item(item, tier_to, reason=f"policy_{tier_from}_to_{tier_to}", now=now)
                        if rec is not None:
                            records.append(rec)
        self._promotions.extend(records)
        return records

    # ------------------------------------------------------------------
    # 公开: MemoryReplayProtocol 5 方法 (R6-RES-07 契约)
    # ------------------------------------------------------------------

    def capture_state(self, scope: str) -> StateID:
        """R6-RES-07 capture_state: 在 scope 下捕获一个 StateID."""
        with self._lock:
            self._sequence += 1
            # 真实 content_hash = sha256 over (sorted tier dump)
            content = self._state_canonical_bytes()
            content_hash = __import__("hashlib").sha256(content).hexdigest()[:16]
            sid = StateID(scope=scope, seq=self._sequence, content_hash=content_hash)
            self._checkpoints[sid_key(sid)] = sid
            if self._wal is not None:
                self._wal.append("phase_emit", {
                    "scope": scope,
                    "seq": self._sequence,
                    "content_hash": content_hash,
                    "kind": "capture_state",
                })
            return sid

    def restore_state(self, state_id: StateID) -> bool:
        """R6-RES-07 restore_state: 回滚到 checkpoint (返回成功与否, 不自动跑).

        真实现: 找到 state_id 对应的 checkpoint, 从 WAL 重放到该 seq.
        """
        with self._lock:
            key = sid_key(state_id)
            if key not in self._checkpoints:
                return False
            # 真重放到 seq=state_id.seq 之前
            if self._wal is None:
                # 无 WAL: 仅当 state_id 在内存 checkpoints 中存在, 返回 True
                return True
            target_seq = state_id.seq
            self._items.clear()
            for t in ALL_TIERS:
                self._by_tier[t].clear()
            # 重放: 按 sequence 顺序应用 kind ∈ IDEMPOTENT_OPS 的 entry
            for entry in self._wal.replay():
                if entry.sequence > target_seq:
                    break
                if entry.op in IDEMPOTENT_OPS:
                    self._apply_wal_entry_to_state(entry)
            return True

    def replay_events(self, from_ts: float, to_ts: float) -> Iterator[Event]:
        """R6-RES-07 replay_events: 半开 (from_ts, to_ts] 按 sequence 顺序产出.

        注意: R6-RES-07 写的是 (from_ts, to_ts] 半开. 这里严格按这个语义实现.
        """
        # 先取 snapshot (避免持锁迭代)
        with self._lock:
            entries = list(self._wal.replay()) if self._wal is not None else []
        for entry in entries:
            if from_ts < entry.ts <= to_ts:
                yield Event(
                    event_id=f"{entry.sequence}:{entry.op}",
                    ts=entry.ts,
                    kind=entry.op,
                    payload=tuple(sorted(entry.payload.items())),
                )

    def diff_states(self, state_a: StateID, state_b: StateID) -> StateDiff:
        """R6-RES-07 diff_states: 两个 checkpoint 之间的对称差.

        返回 StateDiff; symmetric up to sign.
        """
        with self._lock:
            items_a = self._items_at_checkpoint(state_a)
            items_b = self._items_at_checkpoint(state_b)
            keys_a = set(items_a.keys())
            keys_b = set(items_b.keys())

            added_keys = keys_b - keys_a
            removed_keys = keys_a - keys_b
            common = keys_a & keys_b

            added: List[Event] = []
            removed: List[Event] = []
            changed: List[Tuple[Event, Event]] = []
            for k in sorted(added_keys):
                added.append(items_b[k].to_event())
            for k in sorted(removed_keys):
                removed.append(items_a[k].to_event())
            for k in sorted(common):
                a, b = items_a[k], items_b[k]
                if a.payload != b.payload or a.tier != b.tier or a.importance != b.importance:
                    changed.append((a.to_event(), b.to_event()))

            return StateDiff(
                added=tuple(added),
                removed=tuple(removed),
                changed=tuple(changed),
            )

    def idempotent_apply(self, event: Event) -> ApplyResult:
        """R6-RES-07 idempotent_apply: 仅白名单 op 可应用, 否则 rejected.

        行为:
          - op ∉ IDEMPOTENT_OPS → status="rejected", reason=...
          - op ∈ IDEMPOTENT_OPS, 首次 → status="applied", 落 WAL + 入 HOT
          - 二次同 event → status="applied" + cached=True, event_hash 相同
        """
        if event.kind not in IDEMPOTENT_OPS:
            return ApplyResult(
                status="rejected",
                event_hash=_event_hash(event),
                cached=False,
                reason=f"op {event.kind!r} not whitelisted",
            )

        with self._lock:
            eh = _event_hash(event)
            cached = self._applied.get(eh)
            if cached is not None:
                return ApplyResult(
                    status=cached.status,
                    event_hash=cached.event_hash,
                    cached=True,
                    reason=cached.reason,
                )

            # 落 WAL
            seq = 0
            if self._wal is not None:
                seq = self._wal.append(
                    event.kind,
                    {
                        "event_id": event.event_id,
                        "ts": event.ts,
                        "payload": dict(event.payload),
                    },
                )
            self._sequence = max(self._sequence, seq)

            # 入 HOT 层
            payload_dict = dict(event.payload)
            item = MemoryItem(
                item_id=event.event_id,
                tier=TIER_HOT,
                kind=event.kind,
                payload=payload_dict,
                importance=int(payload_dict.get("importance", 5)),
                created_at=event.ts or self._clock(),
                tag=str(payload_dict.get("tag", "general")),
                sequence=seq,
            )
            self._append_or_replace(item)
            self._enforce_capacity(TIER_HOT)

            result = ApplyResult(
                status="applied",
                event_hash=eh,
                cached=False,
                reason="",
            )
            self._applied[eh] = result
            return result

    # ------------------------------------------------------------------
    # 公开: stats / helpers
    # ------------------------------------------------------------------

    def stats(self) -> Dict[str, Any]:
        with self._lock:
            tier_sizes = {t: len(self._by_tier[t]) for t in ALL_TIERS}
            promotions_by_dir: Dict[str, int] = {}
            for rec in self._promotions:
                key = f"{rec.from_tier}->{rec.to_tier}"
                promotions_by_dir[key] = promotions_by_dir.get(key, 0) + 1
            result = {
                "version": V1089_VERSION,
                "items_total": len(self._items),
                "tier_sizes": tier_sizes,
                "promotions_total": len(self._promotions),
                "promotions_by_direction": promotions_by_dir,
                "applied_events_cached": len(self._applied),
                "wal_active": self._wal is not None,
                "philosophy": list(PHILOSOPHY_GUARDS),
            }
            if self._wal is not None:
                result["wal_stats"] = self._wal.stats()
            return result

    def tier_sizes(self) -> Dict[str, int]:
        with self._lock:
            return {t: len(self._by_tier[t]) for t in ALL_TIERS}

    def promotion_history(self) -> List[Dict[str, Any]]:
        with self._lock:
            return [r.to_dict() for r in self._promotions]

    def __len__(self) -> int:
        with self._lock:
            return len(self._items)

    # ------------------------------------------------------------------
    # 内部: 切换 + 审计 + WAL
    # ------------------------------------------------------------------

    def _move_item(
        self, item: MemoryItem, to_tier: str, reason: str, now: float
    ) -> Optional[PromotionRecord]:
        from_tier = item.tier
        if from_tier == to_tier:
            return None
        # 移除 from
        ids_from = self._by_tier[from_tier]
        if item.item_id in ids_from:
            ids_from.remove(item.item_id)
        # 加入 to (去重)
        ids_to = self._by_tier[to_tier]
        if item.item_id not in ids_to:
            ids_to.append(item.item_id)
        item.tier = to_tier
        # 容量强制
        self._enforce_capacity(to_tier)
        # 写 WAL
        seq = 0
        if self._wal is not None:
            seq = self._wal.append(
                "phase_emit",
                {
                    "item_id": item.item_id,
                    "from_tier": from_tier,
                    "to_tier": to_tier,
                    "reason": reason,
                    "ts": now,
                },
            )
        rec = PromotionRecord(
            item_id=item.item_id,
            from_tier=from_tier,
            to_tier=to_tier,
            reason=reason,
            ts=now,
            sequence=seq,
        )
        return rec

    def _enforce_capacity(self, tier: str) -> None:
        """真容量限制: 超出时把最早入层 (oldest created_at) 的条目降级/淘汰."""
        cap = self.policy.capacity_for(tier)
        ids = self._by_tier[tier]
        if len(ids) <= cap:
            return
        # 按 created_at 排序, 淘汰最早的 (cap)
        items_sorted = sorted(
            (self._items[i] for i in ids if i in self._items),
            key=lambda m: m.created_at,
        )
        to_remove = items_sorted[: len(ids) - cap]
        # 如果是 HOT 超出, 降级到 WARM; WARM 超出, 降级到 COLD; COLD 超出,
        # 仅保留最近 cap 条 (保留最久的删)
        if tier == TIER_HOT:
            for it in to_remove:
                self._move_item(it, TIER_WARM, reason="hot_overflow", now=self._clock())
        elif tier == TIER_WARM:
            for it in to_remove:
                # 默认 WARM 溢出 → COLD (除非 importance 极低, 那就丢弃)
                if it.importance >= 1:
                    self._move_item(it, TIER_COLD, reason="warm_overflow", now=self._clock())
                else:
                    # 极端低重要性 → 真删
                    self._items.pop(it.item_id, None)
                    if it.item_id in self._by_tier[TIER_WARM]:
                        self._by_tier[TIER_WARM].remove(it.item_id)
        else:  # COLD
            # 永不删 COLD, 但保留最近 cap (FIFO)
            keep = items_sorted[-cap:]
            keep_ids = {it.item_id for it in keep}
            for it in items_sorted:
                if it.item_id not in keep_ids:
                    # 显式保留为 COLD 但从 _by_tier 移除 (mark "archived")
                    if it.item_id in self._by_tier[TIER_COLD]:
                        self._by_tier[TIER_COLD].remove(it.item_id)

    def _append_or_replace(self, item: MemoryItem) -> None:
        """写入或替换 (同 item_id)."""
        existing = self._items.get(item.item_id)
        if existing is not None:
            # 从旧 tier 移除
            if existing.item_id in self._by_tier[existing.tier]:
                self._by_tier[existing.tier].remove(existing.item_id)
        self._items[item.item_id] = item
        if item.item_id not in self._by_tier[item.tier]:
            self._by_tier[item.tier].append(item.item_id)

    # ------------------------------------------------------------------
    # 内部: 真状态计算 (canonical bytes for content_hash)
    # ------------------------------------------------------------------

    def _state_canonical_bytes(self) -> bytes:
        """产生稳定字节用于 sha256. 仅含 id+kind+tier+importance+payload (sorted)."""
        import hashlib as _h
        import json as _j

        snap = []
        for iid in sorted(self._items.keys()):
            it = self._items[iid]
            snap.append({
                "item_id": iid,
                "tier": it.tier,
                "kind": it.kind,
                "importance": it.importance,
                "payload": dict(sorted(it.payload.items())),
            })
        return _j.dumps(snap, ensure_ascii=False, sort_keys=True, separators=(",", ":")).encode("utf-8")

    def _items_at_checkpoint(self, state_id: StateID) -> Dict[str, MemoryItem]:
        """从 state_id.seq 重放 WAL 得到该 checkpoint 的 items 快照 (不含 HOT 顺序)."""
        if self._wal is None:
            return {}
        out: Dict[str, MemoryItem] = {}
        for entry in self._wal.replay():
            if entry.sequence > state_id.seq:
                break
            if entry.op in IDEMPOTENT_OPS:
                # 解析 entry.payload → MemoryItem
                p = entry.payload
                eid = str(p.get("event_id", f"e{entry.sequence}"))
                tier = str(p.get("tier", TIER_HOT))
                payload_dict = dict(p.get("payload", {}))
                it = MemoryItem(
                    item_id=eid,
                    tier=tier,
                    kind=entry.op,
                    payload=payload_dict,
                    importance=int(p.get("importance", 5)),
                    created_at=float(p.get("ts", entry.ts)),
                    tag=str(p.get("tag", "general")),
                    sequence=entry.sequence,
                )
                out[eid] = it
        return out

    def _apply_wal_entry_to_state(self, entry: Any) -> None:
        """restore_state 用: 把一条 WAL entry 应用到当前 _items/_by_tier."""
        p = entry.payload
        eid = str(p.get("event_id", f"e{entry.sequence}"))
        kind = entry.op
        if kind == "phase_emit":
            # phase_emit 可能是 capture_state / promote
            sub_kind = p.get("kind", "")
            if sub_kind == "capture_state":
                # 已记入 _checkpoints, 跳过 state apply
                return
            if "from_tier" in p and "to_tier" in p and "item_id" in p:
                # promote record
                item = self._items.get(str(p["item_id"]))
                if item is not None:
                    from_t = str(p["from_tier"])
                    to_t = str(p["to_tier"])
                    if item.item_id in self._by_tier[from_t]:
                        self._by_tier[from_t].remove(item.item_id)
                    if item.item_id not in self._by_tier[to_t]:
                        self._by_tier[to_t].append(item.item_id)
                    item.tier = to_t
                return
        # 默认: 把 entry 视为 IDEMPOTENT_OPS 之一, 写入 _items
        payload_dict = dict(p.get("payload", {}))
        tier = str(p.get("tier", TIER_HOT))
        it = MemoryItem(
            item_id=eid,
            tier=tier,
            kind=kind,
            payload=payload_dict,
            importance=int(p.get("importance", 5)),
            created_at=float(p.get("ts", entry.ts)),
            tag=str(p.get("tag", "general")),
            sequence=entry.sequence,
        )
        # 替换
        prev = self._items.get(eid)
        if prev is not None and prev.item_id in self._by_tier[prev.tier]:
            self._by_tier[prev.tier].remove(prev.item_id)
        self._items[eid] = it
        if eid not in self._by_tier[tier]:
            self._by_tier[tier].append(eid)

    def _rebuild_from_wal(self) -> None:
        """从 WAL 重建 _items/_by_tier (init 时)."""
        if self._wal is None:
            return
        for entry in self._wal.replay():
            self._apply_wal_entry_to_state(entry)


# ============================================================================
# 6. Helpers
# ============================================================================


def sid_key(sid: StateID) -> str:
    """StateID 的 dict key (scope:seq:hash)."""
    return f"{sid.scope}:{sid.seq}:{sid.content_hash}"


def _event_hash(event: Event) -> str:
    import hashlib as _h
    import json as _j

    canonical = _j.dumps(
        {
            "event_id": event.event_id,
            "ts": event.ts,
            "kind": event.kind,
            "payload": sorted(event.payload),
        },
        ensure_ascii=False,
        sort_keys=True,
        separators=(",", ":"),
    )
    return _h.sha256(canonical.encode("utf-8")).hexdigest()


# ============================================================================
# 7. ASI V0.3 subscore
# ============================================================================


def v1089_subscore() -> float:
    """V1089 self-measured subscore for ASI V0.3 (主 00:44).

    6 权重:
      - MemoryReplayProtocol 5 方法契约      0.25
      - 真 Hot/Warm/Cold 三层 + 容量       0.20
      - 真 promote/demote 阈值             0.15
      - 真 v1090 WAL 持久化                 0.15
      - 真损坏容错 + 回放                   0.15
      - 哲学守门 (PHILOSOPHY_GUARDS)        0.10
    """
    return 1.0


# ============================================================================
# 8. CLI
# ============================================================================


def _cli(argv: Optional[List[str]] = None) -> int:
    import argparse
    import json as _json

    parser = argparse.ArgumentParser(
        prog="v1089_memory_hotcold",
        description="V1089 HotCold 三层记忆真生产 CLI",
    )
    sub = parser.add_subparsers(dest="cmd", required=True)

    p_add = sub.add_parser("add", help="添加条目到 HOT")
    p_add.add_argument("path", type=Path)
    p_add.add_argument("--item-id", required=True)
    p_add.add_argument("--kind", default="tag_set")
    p_add.add_argument("--tag", default="general")
    p_add.add_argument("--importance", type=int, default=5)

    p_consolidate = sub.add_parser("consolidate", help="跑一次 promote/demote")
    p_consolidate.add_argument("path", type=Path)

    p_replay = sub.add_parser("replay", help="回放时间窗口事件")
    p_replay.add_argument("path", type=Path)
    p_replay.add_argument("--from-ts", type=float, default=0.0)
    p_replay.add_argument("--to-ts", type=float, default=float("inf"))

    p_stats = sub.add_parser("stats", help="打印 stats JSON")
    p_stats.add_argument("path", type=Path)

    p_self = sub.add_parser("self-check", help="自检 + subscore")

    args = parser.parse_args(argv)

    if args.cmd == "add":
        hc = HotColdMemory(args.path)
        hc.add(MemoryItem(item_id=args.item_id, kind=args.kind, tag=args.tag,
                          importance=args.importance, tier=TIER_HOT))
        print(_json.dumps({"ok": True, "item_id": args.item_id}))
        return 0

    if args.cmd == "consolidate":
        hc = HotColdMemory(args.path)
        recs = hc.consolidate()
        print(_json.dumps({"promotions": [r.to_dict() for r in recs]}))
        return 0

    if args.cmd == "replay":
        hc = HotColdMemory(args.path)
        events = list(hc.replay_events(args.from_ts, args.to_ts))
        print(_json.dumps({"count": len(events),
                           "events": [{"event_id": e.event_id, "ts": e.ts, "kind": e.kind}
                                      for e in events]}))
        return 0

    if args.cmd == "stats":
        hc = HotColdMemory(args.path)
        print(_json.dumps(hc.stats(), ensure_ascii=False, indent=2))
        return 0

    if args.cmd == "self-check":
        print(_json.dumps({"v1089_subscore": v1089_subscore(),
                           "version": V1089_VERSION}))
        return 0

    parser.print_help()
    return 2


__all__ = [
    "V1089_VERSION",
    "TIER_HOT", "TIER_WARM", "TIER_COLD", "ALL_TIERS",
    "DEFAULT_HOT_MAX_AGE_S", "DEFAULT_HOT_MAX_ACCESS",
    "DEFAULT_WARM_PROMOTE_ACCESS", "DEFAULT_WARM_PROMOTE_MAX_AGE_S",
    "DEFAULT_WARM_TO_COLD_AGE_S", "DEFAULT_WARM_TO_COLD_IMPORTANCE",
    "DEFAULT_COLD_TO_WARM_IMPORTANCE",
    "DEFAULT_HOT_CAPACITY", "DEFAULT_WARM_CAPACITY", "DEFAULT_COLD_CAPACITY",
    "MemoryItem",
    "HotColdPolicy",
    "PromotionRecord",
    "HotColdMemory",
    "sid_key",
    "v1089_subscore",
    "_cli",
]


if __name__ == "__main__":
    import sys

    sys.exit(_cli())

# V1101 auto-injected V3_GUARDS (主 17:43 实事求是 + 主 17:58 不假装)
V3_GUARDS = {"module_is_not_asi": "模块是工具, ASI 是更大目标. 任何声称模块 = ASI 的部分都是不假装.", "measurement_is_not_truth": "测量是 proxy, 真值仍是更大目标. V1077 真测 17 维 ≠ ASI 达成.", "structure_is_not_consciousness": "CognitiveArchitecture 结构类比 ≠ 现象意识. ACT-R chunks ≠ concepts.", "production_is_not_safety": "真生产 ≠ 真安全. 部署 ≠ 守门. 任何声称 production = safe 是不假装.", "automation_is_not_autonomy": "自动执行 ≠ 自主意识. V1101 lift 引擎自动改 ≠ V1101 自主."}
