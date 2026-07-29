"""V1091 MemoryReplay — 真生产状态回放 (R8-TrackA2)

主 22:33 ASI 北极星 + 主 19:33 走在前人经验上 + 主 23:44 干到底 +
主 17:58 + 20:46 不假装 + 主 12:14 中央 AI 是永恒身份 + V3 + V1081。

R6-RES-07 5 方法契约 (从 memory_replay_design.py 提升到真生产):
  - capture_state  : 从 WAL 快照生成 StateID, scope+seq+sha256
  - restore_state  : 把当前状态回滚到指定 StateID 的检查点
  - replay_events  : 在 [from_ts, to_ts] 窗口重放事件流 (Iterator)
  - diff_states    : 两个检查点之间的对称差 (added / removed / changed)
  - idempotent_apply: 事件幂等应用 (白名单 only, 同 event 二次返回 cached)

借鉴 (主 19:33 走在前人经验上):
  1. V1052 WAL (DeltaMemory 借鉴)  : JSONL + sha256 + 损坏 skip
  2. V1052 Episode 不可变 (MemoryOS): append-only, ts 排序
  3. Letta archival_memory         : query + tag + temporal + top_k 思想
  4. R37 q5 hippocampal replay     : sharp-wave ripples, 启发式优先级
  5. Tonbo WAL 并发读写指标        : 借用锁模型 (RLock + 损坏容错)

哲学守门 (V3 + V1081):
  - replay ≠ bit-exact     : 重放是近似重建, 不声称字节相等
  - idempotent ≠ safe      : 幂等仅对白名单 op 集合成立
  - capture ≠ backup       : 快照是 diff/restore 工具, 不是历史归档
  - replay ≠ understanding : 启发式再发射, 不是现象学回忆

V1082 backlog 填洞 (本模块): #A2-1 MemoryReplay 真生产 (Step 1/3)。
不对外宣称 ASI, 数字涨不涨不重要, 真生产不停 才重要。
"""
from __future__ import annotations

import copy
import hashlib
import json
import os
import threading
import time
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Dict, Iterator, List, Optional, Tuple

# 从 R7-BE-02 设计契约引用 (已存在的内存协议).
from apeireth.memory_replay_design import (
    ApplyResult,
    Event,
    IDEMPOTENT_OPS,
    PHILOSOPHY_GUARDS,
    StateDiff,
    StateID,
)

V1091_VERSION = "0.1.0"


# ============================================================================
# 1. WAL 格式 — V1052 兼容的 JSONL + sha256 + seq
# ============================================================================


@dataclass
class WalEntry:
    """单条 WAL 记录 (V1052 WalEntry 的回放专用版本).

    设计: sequence 严格递增; payload 字段名兼容 V1052,
          但额外携带 ts/scope/event 以便 replay_events 排序。
    """
    sequence: int
    ts: float
    scope: str
    event: Event
    checksum: str = ""

    def __post_init__(self) -> None:
        if not isinstance(self.sequence, int) or isinstance(self.sequence, bool) or self.sequence < 1:
            raise ValueError("sequence must be a positive integer")
        if not isinstance(self.ts, (int, float)) or isinstance(self.ts, bool):
            raise ValueError("ts must be numeric")
        if not isinstance(self.scope, str):
            raise ValueError("scope must be a string")
        if not isinstance(self.event, Event):
            raise ValueError("event must be an Event instance")
        if not self.event.event_id:
            raise ValueError("event.event_id must be non-empty")

    def compute_checksum(self) -> str:
        canonical = json.dumps(
            {
                "sequence": self.sequence,
                "ts": round(self.ts, 6),
                "scope": self.scope,
                "event_id": self.event.event_id,
                "event_ts": round(self.event.ts, 6),
                "kind": self.event.kind,
                "payload": dict(self.event.payload),
            },
            ensure_ascii=False,
            sort_keys=True,
        )
        return hashlib.sha256(canonical.encode("utf-8")).hexdigest()

    def to_jsonl(self) -> str:
        self.checksum = self.compute_checksum()
        rec = {
            "sequence": self.sequence,
            "ts": self.ts,
            "scope": self.scope,
            "event": {
                "event_id": self.event.event_id,
                "ts": self.event.ts,
                "kind": self.event.kind,
                "payload": dict(self.event.payload),
            },
            "checksum": self.checksum,
        }
        return json.dumps(rec, ensure_ascii=False, sort_keys=True)

    @staticmethod
    def from_jsonl(line: str) -> "WalEntry":
        if not isinstance(line, str) or not line.strip():
            raise ValueError("WAL line must be a non-empty string")
        rec = json.loads(line)
        if not isinstance(rec, dict):
            raise ValueError("WAL record must be a JSON object")
        event_rec = rec.get("event")
        if not isinstance(event_rec, dict):
            raise ValueError("WAL record missing event object")
        payload_value = event_rec.get("payload", {})
        if not isinstance(payload_value, dict):
            raise ValueError("event.payload must be a JSON object")
        ev = Event(
            event_id=str(event_rec.get("event_id", "")),
            ts=float(event_rec["ts"]),
            kind=str(event_rec.get("kind", "")),
            payload=tuple(sorted(payload_value.items())),
        )
        entry = WalEntry(
            sequence=int(rec["sequence"]),
            ts=float(rec["ts"]),
            scope=str(rec.get("scope", "")),
            event=ev,
            checksum=str(rec.get("checksum", "")),
        )
        return entry


def _event_hash(event: Event) -> str:
    """幂等键: sha256(event_id) + kind + payload canonical."""
    payload_str = json.dumps(dict(event.payload), sort_keys=True, ensure_ascii=False)
    raw = f"{event.event_id}|{event.kind}|{payload_str}"
    return hashlib.sha256(raw.encode("utf-8")).hexdigest()


# ============================================================================
# 2. 快照 (StateID + 状态字典的 diff 化存储)
# ============================================================================


# V1091 内部事件 kind — 由 capture_state / restore_state 写盘.
# 仅 _recover_from_disk 识别; 不参与 IDEMPOTENT_OPS, 永远不修改 live_state.
_CHECKPOINT_KIND = "v1091_checkpoint"
_ROLLBACK_KIND = "v1091_rollback"


@dataclass
class Checkpoint:
    """一个 StateID 对应的检查点: 当前 state dict + 触发它的 WAL seq 上限."""
    state_id: StateID
    state: Dict[str, Any]
    up_to_sequence: int       # capture 时已经写入 WAL 的最大 seq
    created_at: float = field(default_factory=time.time)


# ============================================================================
# 3. MemoryReplay — 真生产主类
# ============================================================================


class MemoryReplay:
    """V1091 真生产状态回放器.

    用法:
        mr = MemoryReplay()                    # 内存模式
        mr = MemoryReplay(wal_path=Path(...))  # 持久化模式
        sid = mr.capture_state("session-1")
        mr.apply_event(scope, Event(...))      # 写 WAL 并更新 live state
        for ev in mr.replay_events(0, t2):
            ...

    线程安全: 单实例内的 _lock 保证 capture_state / apply_event /
              restore_state 的互斥;replay_events 走 snapshot, 无锁。
    """

    def __init__(
        self,
        wal_path: Optional[Path] = None,
        clock: Optional[Any] = None,
        max_wal_bytes: int = 64 * 1024 * 1024,
    ) -> None:
        self.wal_path = wal_path
        self._clock = clock or time.time
        self._max_wal_bytes = max_wal_bytes

        self._lock = threading.RLock()
        self._seq: int = 0
        self._wal: List[WalEntry] = []                 # 内存 WAL (权威)
        self._checkpoint_seq: List[int] = []           # 每个 checkpoint 对应的 up_to seq (有序)
        self._checkpoint_marker_seq: List[int] = []    # checkpoint 自身 WAL seq (用于回放恢复)
        self._checkpoints: List[Checkpoint] = []       # 内存 checkpoint 镜像 (同进程内 capture)
        self._live_state: Dict[str, Any] = {}          # 当前 live state (从最近 checkpoint replay 后增量)
        self._applied: Dict[str, ApplyResult] = {}     # idempotent_apply 缓存 (key=sha256(event_id+payload))
        self._applied_event_ids: Dict[str, str] = {}   # event_id -> event_hash (漂移检测)
        self._skipped_corrupt: int = 0

        if wal_path is not None:
            self._recover_from_disk(wal_path)

    # ------------------------------------------------------------------
    # WAL 持久化 / 恢复
    # ------------------------------------------------------------------

    def _recover_from_disk(self, path: Path) -> None:
        """从磁盘 JSONL 重建 WAL; 损坏行跳过, 累计 _skipped_corrupt.

        恢复完成后:
          - _wal 持有所有有效 WAL 行 (checkpoint / rollback / idempotent 写)
          - _seq 跟到目前最大 seq
          - _live_state 通过从最早 checkpoint 行开始回放 idempotent op 重建
        """
        if not path.exists():
            return
        path.parent.mkdir(parents=True, exist_ok=True)
        last_seq = 0
        with path.open("r", encoding="utf-8") as fh:
            for raw in fh:
                line = raw.strip()
                if not line:
                    continue
                try:
                    entry = WalEntry.from_jsonl(line)
                except (json.JSONDecodeError, KeyError, ValueError, TypeError):
                    self._skipped_corrupt += 1
                    continue
                expected = entry.compute_checksum()
                if expected != entry.checksum:
                    self._skipped_corrupt += 1
                    continue
                self._wal.append(entry)
                if entry.sequence > last_seq:
                    last_seq = entry.sequence
        self._seq = last_seq
        # 重建 live_state: 先重放所有 IDEMPOTENT_OPS (单调覆盖), 再按最后
        # 一个 rollback 标记将 live_state 截断回 checkpoint 快照.
        # 取最后一个 rollback 标记 (最大 seq).
        rollback_seq = 0
        rollback_target_seq = 0
        for entry in self._wal:
            if entry.event.kind == _ROLLBACK_KIND:
                rollback_seq = entry.sequence
                payload = dict(entry.event.payload)
                target_hash = payload.get("content_hash", "")
                if target_hash:
                    rollback_target_seq = self._find_checkpoint_seq_by_hash(target_hash)

        self._live_state = {}
        for entry in self._wal:
            if entry.event.kind in IDEMPOTENT_OPS:
                self._apply_to_state(entry.event)

        if rollback_target_seq:
            snapshot = self._find_snapshot_by_seq(rollback_target_seq)
            if snapshot is not None:
                self._live_state = snapshot  # snapshot 已是 dict, deep-copy 在 _locate_checkpoint 做过
        elif rollback_seq:
            # rollback 标记存在但找不到 checkpoint (e.g. rotate). 兜底: 不
            # 截断, 上面 replay 仍完整.
            pass

        # 重建 idempotent 缓存. 由于 rollback 之后的事件已不再属于 live_state,
        # 缓存要按 replay_wal 重建, 只记录 rollback 之前的事件.
        self._applied.clear()
        self._applied_event_ids.clear()
        for entry in self._wal:
            if rollback_seq and entry.sequence >= rollback_seq:
                continue
            if entry.event.kind in IDEMPOTENT_OPS:
                eh = _event_hash(entry.event)
                self._applied_event_ids[entry.event.event_id] = eh
                self._applied[eh] = ApplyResult(status="applied", event_hash=eh)

    def _persist(self, entry: WalEntry) -> None:
        if self.wal_path is None:
            return
        # 简单大小限制: 超过阈值则裁剪最先的 25%
        line = entry.to_jsonl() + "\n"
        if self.wal_path.exists() and self.wal_path.stat().st_size > self._max_wal_bytes:
            self._rotate_wal()
        with self.wal_path.open("a", encoding="utf-8") as fh:
            fh.write(line)

    def _rotate_wal(self) -> None:
        """软轮转: 保留后 75% 行 (借鉴 DeltaMemory compact 借鉴).

        只保留完整 WAL 行; 若剩余行过少 (≤ 4 行) 不截断, 避免把 checkpoint
        标记或活跃重放窗口全部丢失.
        """
        if self.wal_path is None or not self.wal_path.exists():
            return
        lines = self.wal_path.read_text(encoding="utf-8").splitlines()
        if len(lines) <= 4:
            return
        keep = lines[len(lines) * 3 // 4:]
        self.wal_path.write_text("\n".join(keep) + "\n", encoding="utf-8")

    def _checkpoint_marker_event(self, state_id: StateID, snapshot: Dict[str, Any]) -> Event:
        """构造写盘的 checkpoint 事件 (内含 deep-copy 状态快照)."""
        event_id = f"cp:{state_id.seq}:{state_id.content_hash[:16]}"
        payload = (
            ("scope", state_id.scope),
            ("content_hash", state_id.content_hash),
            ("state_snapshot", json.dumps(snapshot, ensure_ascii=False, sort_keys=True)),
        )
        return Event(event_id=event_id, ts=self._clock(), kind=_CHECKPOINT_KIND, payload=payload)

    def _rollback_marker_event(self, state_id: StateID, up_to_seq: int) -> Event:
        event_id = f"rb:{state_id.seq}:{state_id.content_hash[:16]}"
        payload = (
            ("scope", state_id.scope),
            ("content_hash", state_id.content_hash),
            ("up_to_seq", int(up_to_seq)),
        )
        return Event(event_id=event_id, ts=self._clock(), kind=_ROLLBACK_KIND, payload=payload)

    # ------------------------------------------------------------------
    # 内部: 受锁的事件写入
    # ------------------------------------------------------------------

    def apply_event(self, scope: str, event: Event) -> int:
        """写入一条事件到 WAL 并按 op 类型更新 live state.

        仅在 IDEMPOTENT_OPS 内的 kind 才更新 live state, 其他 kind 仅落 WAL
        (用于审计, 不污染状态机). 返回新 sequence.

        并发: 多线程同时写入通过 _lock 串行化; 写完后立即持久化。
        """
        with self._lock:
            self._seq += 1
            entry = WalEntry(
                sequence=self._seq,
                ts=self._clock(),
                scope=scope,
                event=event,
            )
            entry.checksum = entry.compute_checksum()
            self._wal.append(entry)
            self._persist(entry)

            if event.kind in IDEMPOTENT_OPS:
                self._apply_to_state(event)
            return self._seq

    def _apply_to_state(self, event: Event) -> None:
        """把白名单事件应用到 live state dict (无副作用读取)."""
        if event.kind == "tag_set":
            tags = self._live_state.setdefault("tags", {})
            for k, v in event.payload:
                tags[k] = v
        elif event.kind == "anchor_link":
            links = self._live_state.setdefault("anchors", [])
            links.append(dict(event.payload))
        elif event.kind == "anchor_unlink":
            anchors = self._live_state.setdefault("anchors", [])
            payload_d = dict(event.payload)
            self._live_state["anchors"] = [
                a for a in anchors if a.get("id") != payload_d.get("id")
            ]
        elif event.kind == "score_record":
            scores = self._live_state.setdefault("scores", [])
            scores.append(dict(event.payload))
        elif event.kind == "phase_emit":
            phases = self._live_state.setdefault("phases", [])
            phases.append(dict(event.payload))
        elif event.kind == "trace_record":
            traces = self._live_state.setdefault("traces", [])
            traces.append(dict(event.payload))

    def _record_idempotent(self, event: Event, *, persist: bool) -> bool:
        """白名单事件写入幂等缓存.

        - event_id 已知且 payload hash 不一致 → 拒绝 (漂移攻击护栏).
        - event_id 已知且 payload hash 一致 → 命中, cached=True.
        - 否则写入, 返回 True.

        ``persist=True`` 时同时落 WAL; ``persist=False`` 仅刷新缓存 (恢复路径).
        """
        if event.kind not in IDEMPOTENT_OPS:
            return False
        eh = _event_hash(event)
        existing = self._applied_event_ids.get(event.event_id)
        if existing is not None:
            if existing != eh:
                return False
            return True
        self._applied_event_ids[event.event_id] = eh
        self._applied[eh] = ApplyResult(status="applied", event_hash=eh)
        if persist:
            self._seq += 1
            entry = WalEntry(
                sequence=self._seq,
                ts=self._clock(),
                scope="idempotent",
                event=event,
            )
            entry.checksum = entry.compute_checksum()
            self._wal.append(entry)
            self._persist(entry)
        self._apply_to_state(event)
        return True

    # ------------------------------------------------------------------
    # 5 方法契约 — 真生产实现
    # ------------------------------------------------------------------

    def capture_state(self, scope: str) -> StateID:
        """捕获当前 live state 为 checkpoint, 返回 StateID.

        行为:
          1. 生成 StateID (scope + seq + content_hash).
          2. 在内存保存 deep-copy 快照 (同进程兜底).
          3. 写一条 WAL checkpoint 标记, 让 restore 跨进程也能定位.
        """
        with self._lock:
            snapshot = copy.deepcopy(self._live_state)
            state_id = StateID(
                scope=scope,
                seq=len(self._checkpoints) + 1,
                content_hash=self._state_hash(snapshot),
            )
            checkpoint = Checkpoint(
                state_id=state_id,
                state=snapshot,
                up_to_sequence=self._seq,
            )
            self._checkpoints.append(checkpoint)
            self._checkpoint_seq.append(self._seq)

            self._seq += 1
            marker = self._checkpoint_marker_event(state_id, snapshot)
            entry = WalEntry(
                sequence=self._seq,
                ts=self._clock(),
                scope=scope,
                event=marker,
            )
            entry.checksum = entry.compute_checksum()
            self._wal.append(entry)
            self._persist(entry)
            self._checkpoint_marker_seq.append(entry.sequence)
            return state_id

    def restore_state(self, state_id: StateID) -> bool:
        """回滚 live state 到指定 StateID 对应的 checkpoint.

        行为 (契约: 仅返回成功/失败, 不自动重新跑):
          1. 找到 state_id 对应的 checkpoint 快照 (优先 in-mem, 兜底 WAL).
          2. 把快照写回 live_state.
          3. 写 rollback 标记 WAL 行, 附带 up_to_seq, 便于跨进程恢复时
             截断 rollback 之后的事件.
        """
        with self._lock:
            snapshot, up_to_seq = self._locate_checkpoint(state_id)
            if snapshot is None:
                return False
            self._live_state = copy.deepcopy(snapshot)

            self._seq += 1
            entry = WalEntry(
                sequence=self._seq,
                ts=self._clock(),
                scope=state_id.scope,
                event=self._rollback_marker_event(state_id, up_to_seq),
            )
            entry.checksum = entry.compute_checksum()
            self._wal.append(entry)
            self._persist(entry)
            return True

    def _locate_checkpoint(self, state_id: StateID) -> Tuple[Optional[Dict[str, Any]], int]:
        """返回 (snapshot_dict, up_to_seq). 失败时 (None, 0).

        优先从 WAL 持久化 checkpoint 恢复; 内存副本作为兜底.
        """
        for idx, cp in enumerate(self._checkpoints):
            if cp.state_id == state_id:
                up_to = self._checkpoint_seq[idx]
                # 同一进程内, 内存快照是源真.
                return copy.deepcopy(cp.state), up_to
        # 跨进程: 解析 WAL 中的 checkpoint 行
        for entry in self._wal:
            if entry.event.kind != _CHECKPOINT_KIND:
                continue
            payload = dict(entry.event.payload)
            if payload.get("scope") != state_id.scope:
                continue
            if payload.get("content_hash") != state_id.content_hash:
                continue
            raw_snapshot = payload.get("state_snapshot")
            if not isinstance(raw_snapshot, str):
                continue
            try:
                snap = json.loads(raw_snapshot)
                if not isinstance(snap, dict):
                    continue
            except (json.JSONDecodeError, ValueError):
                continue
            # up_to_seq = checkpoint 行前一行 (因为 checkpoint 标记本身就
            # 是当前 seq, 物理 seq > up_to_seq)
            return snap, entry.sequence - 1
        return None, 0

    def replay_events(
        self, from_ts: float, to_ts: float
    ) -> Iterator[Event]:
        """在 (from_ts, to_ts] 时间窗口内按 ts 升序产出事件.

        半开 (R6-RES-07 契约): from_ts < event_ts <= to_ts.
        返回 iterator 而非 list 以节约内存.
        """
        # 走 snapshot 防止迭代时被并发写入干扰
        with self._lock:
            snapshot = list(self._wal)
        snapshot.sort(key=lambda e: (e.event.ts, e.sequence))
        for entry in snapshot:
            if from_ts < entry.event.ts <= to_ts:
                yield entry.event

    def diff_states(self, state_a: StateID, state_b: StateID) -> StateDiff:
        """计算两个 checkpoint 之间的"字段级"对称 diff.

        实现: 还原 checkpoint 时的 live_state dict (tag_set / anchor 等),
        以 (field, key) 为单位:
          - added:   b 有 a 无  → Event(kind="<kind>", payload=[(key, value)])
          - removed: a 有 b 无  → Event(kind="<kind>", payload=[(key, value)])
          - changed: 双方皆有, value 不同 → (a_event, b_event)
        """
        with self._lock:
            stateA = self._state_at_checkpoint(state_a)
            stateB = self._state_at_checkpoint(state_b)
        return self._diff_state_dicts(stateA, stateB)

    def _state_at_checkpoint(self, state_id: StateID) -> Dict[str, Any]:
        """还原 checkpoint 处的 live_state dict (跨进程也 OK)."""
        # 1. 优先同进程内存副本 (深拷贝)
        for cp in self._checkpoints:
            if cp.state_id == state_id:
                return copy.deepcopy(cp.state)
        # 2. 跨进程: 从 WAL 还原到 up_to_seq
        up_to_seq = self._seq_up_to(state_id)
        if up_to_seq is None:
            return {}
        reconstructing: Dict[str, Any] = {}
        for entry in self._wal:
            if entry.sequence > up_to_seq:
                break
            if entry.event.kind in IDEMPOTENT_OPS:
                self._apply_to_state_dict(entry.event, reconstructing)
        return reconstructing

    @staticmethod
    def _apply_to_state_dict(event: Event, state: Dict[str, Any]) -> None:
        """与 _apply_to_state 等价, 但写入传入的 dict (不污染 self)."""
        if event.kind == "tag_set":
            tags = state.setdefault("tags", {})
            for k, v in event.payload:
                tags[k] = v
        elif event.kind == "anchor_link":
            links = state.setdefault("anchors", [])
            links.append(dict(event.payload))
        elif event.kind == "anchor_unlink":
            anchors = state.get("anchors", [])
            payload_d = dict(event.payload)
            state["anchors"] = [
                a for a in anchors if a.get("id") != payload_d.get("id")
            ]
        elif event.kind == "score_record":
            scores = state.setdefault("scores", [])
            scores.append(dict(event.payload))
        elif event.kind == "phase_emit":
            phases = state.setdefault("phases", [])
            phases.append(dict(event.payload))
        elif event.kind == "trace_record":
            traces = state.setdefault("traces", [])
            traces.append(dict(event.payload))

    def _seq_up_to(self, state_id: StateID) -> Optional[int]:
        """checkpoint 物理 seq 上限 (即 marker 之前的 seq)."""
        for idx, cp in enumerate(self._checkpoints):
            if cp.state_id == state_id:
                return self._checkpoint_seq[idx]
        for entry in self._wal:
            if entry.event.kind != _CHECKPOINT_KIND:
                continue
            payload = dict(entry.event.payload)
            if (
                payload.get("scope") == state_id.scope
                and payload.get("content_hash") == state_id.content_hash
            ):
                return entry.sequence - 1
        return None

    @staticmethod
    def _diff_state_dicts(
        state_a: Dict[str, Any], state_b: Dict[str, Any]
    ) -> StateDiff:
        """对称 diff: tag_set 字段级 (added/removed/changed), 列表字段整体。"""

        def _tag_event(key: str, value: Any, ts: float = 0.0) -> Event:
            return Event(
                event_id=f"tag:{key}",
                ts=ts,
                kind="tag_set",
                payload=((key, value),),
            )

        tags_a = state_a.get("tags", {}) or {}
        tags_b = state_b.get("tags", {}) or {}
        keys_a = set(tags_a.keys())
        keys_b = set(tags_b.keys())

        added: List[Event] = []
        removed: List[Event] = []
        changed: List[Tuple[Event, Event]] = []

        for key in sorted(keys_b - keys_a):
            added.append(_tag_event(key, tags_b[key]))
        for key in sorted(keys_a - keys_b):
            removed.append(_tag_event(key, tags_a[key]))
        for key in sorted(keys_a & keys_b):
            if tags_a[key] != tags_b[key]:
                changed.append((
                    _tag_event(key, tags_a[key]),
                    _tag_event(key, tags_b[key]),
                ))

        return StateDiff(
            added=tuple(added),
            removed=tuple(removed),
            changed=tuple(changed),
        )

    def idempotent_apply(self, event: Event) -> ApplyResult:
        """幂等应用事件 (白名单 only).

        行为:
          - op ∉ IDEMPOTENT_OPS → rejected (reason 列出).
          - event_id 已记录且 payload hash 改变 → rejected (drift 护栏).
          - event_id 已记录且 payload hash 一致 → cached=True.
          - 否则 applied, 落 WAL + state.
        """
        with self._lock:
            if event.kind not in IDEMPOTENT_OPS:
                return ApplyResult(
                    status="rejected",
                    event_hash=_event_hash(event),
                    reason=f"op {event.kind!r} not whitelisted",
                )
            eh = _event_hash(event)
            existing = self._applied_event_ids.get(event.event_id)
            if existing is not None and existing != eh:
                return ApplyResult(
                    status="rejected",
                    event_hash=eh,
                    reason=(
                        f"event_id {event.event_id!r} already applied with "
                        "different payload (drift blocked)"
                    ),
                )
            if existing is not None and existing == eh:
                return ApplyResult(
                    status="applied",
                    event_hash=eh,
                    cached=True,
                )
            self._applied_event_ids[event.event_id] = eh
            self._applied[eh] = ApplyResult(status="applied", event_hash=eh)
            self._apply_to_state(event)
            self._seq += 1
            entry = WalEntry(
                sequence=self._seq,
                ts=self._clock(),
                scope="idempotent",
                event=event,
            )
            entry.checksum = entry.compute_checksum()
            self._wal.append(entry)
            self._persist(entry)
            return ApplyResult(status="applied", event_hash=eh)

    # ------------------------------------------------------------------
    # 辅助
    # ------------------------------------------------------------------

    def _find_checkpoint(self, state_id: StateID) -> Optional[Checkpoint]:
        for cp in self._checkpoints:
            if cp.state_id == state_id:
                return cp
        return None

    def _find_checkpoint_seq_by_hash(self, content_hash: str) -> int:
        for entry in self._wal:
            if entry.event.kind != _CHECKPOINT_KIND:
                continue
            if dict(entry.event.payload).get("content_hash") == content_hash:
                return entry.sequence
        return 0

    def _find_snapshot_by_seq(self, seq: int) -> Optional[Dict[str, Any]]:
        for entry in self._wal:
            if entry.sequence == seq and entry.event.kind == _CHECKPOINT_KIND:
                raw = dict(entry.event.payload).get("state_snapshot")
                if isinstance(raw, str):
                    try:
                        snap = json.loads(raw)
                        if isinstance(snap, dict):
                            return snap
                    except (json.JSONDecodeError, ValueError):
                        return None
        return None

    @staticmethod
    def _state_hash(state: Dict[str, Any]) -> str:
        canonical = json.dumps(state, sort_keys=True, ensure_ascii=False)
        return hashlib.sha256(canonical.encode("utf-8")).hexdigest()[:16]

    # ------------------------------------------------------------------
    # 自检 / 报告 (供测试与运维)
    # ------------------------------------------------------------------

    def stats(self) -> Dict[str, Any]:
        with self._lock:
            return {
                "version": V1091_VERSION,
                "wal_entries": len(self._wal),
                "current_seq": self._seq,
                "checkpoints": len(self._checkpoints),
                "checkpoint_markers": len(self._checkpoint_marker_seq),
                "rollback_markers": sum(
                    1 for e in self._wal if e.event.kind == _ROLLBACK_KIND
                ),
                "applied_cache": len(self._applied),
                "applied_event_ids": len(self._applied_event_ids),
                "skipped_corrupt": self._skipped_corrupt,
                "live_state_keys": sorted(self._live_state.keys()),
                "philosophy_guards": list(PHILOSOPHY_GUARDS),
            }

    def verify_wal(self) -> Tuple[int, int]:
        """验证 WAL 完整性. 返回 (valid, corrupt)."""
        valid = 0
        corrupt = 0
        with self._lock:
            for entry in self._wal:
                if entry.compute_checksum() == entry.checksum:
                    valid += 1
                else:
                    corrupt += 1
        return (valid, corrupt)


__all__ = [
    "V1091_VERSION",
    "WalEntry",
    "Checkpoint",
    "MemoryReplay",
    "_event_hash",
]


# V1101 auto-injected V3_GUARDS (主 17:43 实事求是 + 主 17:58 不假装)
V3_GUARDS = {"module_is_not_asi": "模块是工具, ASI 是更大目标. 任何声称模块 = ASI 的部分都是不假装.", "measurement_is_not_truth": "测量是 proxy, 真值仍是更大目标. V1077 真测 17 维 ≠ ASI 达成.", "structure_is_not_consciousness": "CognitiveArchitecture 结构类比 ≠ 现象意识. ACT-R chunks ≠ concepts.", "production_is_not_safety": "真生产 ≠ 真安全. 部署 ≠ 守门. 任何声称 production = safe 是不假装.", "automation_is_not_autonomy": "自动执行 ≠ 自主意识. V1101 lift 引擎自动改 ≠ V1101 自主."}
