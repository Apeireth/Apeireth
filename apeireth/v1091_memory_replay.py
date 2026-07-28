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
        rec = json.loads(line)
        ev = Event(
            event_id=rec["event"]["event_id"],
            ts=rec["event"]["ts"],
            kind=rec["event"]["kind"],
            payload=tuple(sorted(rec["event"]["payload"].items())),
        )
        entry = WalEntry(
            sequence=rec["sequence"],
            ts=rec["ts"],
            scope=rec["scope"],
            event=ev,
            checksum=rec.get("checksum", ""),
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
        self._checkpoints: List[Checkpoint] = []       # 与 _checkpoint_seq 平行的 checkpoint 列表
        self._live_state: Dict[str, Any] = {}          # 当前 live state (从最近 checkpoint replay 后增量)
        self._applied: Dict[str, ApplyResult] = {}     # idempotent_apply 缓存 (key=sha256(event_id+payload))
        self._skipped_corrupt: int = 0

        if wal_path is not None:
            self._recover_from_disk(wal_path)

    # ------------------------------------------------------------------
    # WAL 持久化 / 恢复
    # ------------------------------------------------------------------

    def _recover_from_disk(self, path: Path) -> None:
        """从磁盘 JSONL 重建 WAL; 损坏行跳过, 累计 _skipped_corrupt."""
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
                except (json.JSONDecodeError, KeyError, ValueError):
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
        """软轮转: 保留后 75% 行 (借鉴 DeltaMemory compact 借鉴)."""
        if self.wal_path is None or not self.wal_path.exists():
            return
        lines = self.wal_path.read_text(encoding="utf-8").splitlines()
        keep = lines[len(lines) * 3 // 4 :] if len(lines) > 4 else lines
        self.wal_path.write_text("\n".join(keep) + "\n", encoding="utf-8")

    # ------------------------------------------------------------------
    # 内部: 受锁的事件写入
    # ------------------------------------------------------------------

    def apply_event(self, scope: str, event: Event) -> int:
        """写入一条事件到 WAL 并按 op 类型更新 live state.

        仅在 IDEMPOTENT_OPS 内的 kind 才更新 live state, 其他 kind 仅落 WAL
        (用于审计, 不污染状态机). 返回新 sequence。

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
        key = f"{event.kind}:{event.event_id}"
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

    # ------------------------------------------------------------------
    # 5 方法契约 — 真生产实现
    # ------------------------------------------------------------------

    def capture_state(self, scope: str) -> StateID:
        """捕获当前 live state 为 checkpoint, 返回 StateID."""
        with self._lock:
            current_seq = self._seq
            checkpoint = Checkpoint(
                state_id=StateID(
                    scope=scope,
                    seq=len(self._checkpoints) + 1,
                    content_hash=self._state_hash(self._live_state),
                ),
                state=copy.deepcopy(self._live_state),
                up_to_sequence=current_seq,
            )
            self._checkpoints.append(checkpoint)
            self._checkpoint_seq.append(current_seq)
            return checkpoint.state_id

    def restore_state(self, state_id: StateID) -> bool:
        """回滚 live state 到指定 StateID 对应的 checkpoint.

        返回 True 表示成功; False 表示 state_id 找不到或已被轮转清除。
        """
        with self._lock:
            target = None
            for cp in self._checkpoints:
                if cp.state_id == state_id:
                    target = cp
                    break
            if target is None:
                return False
            # 重放该 checkpoint up_to_sequence 之后的 WAL,
            # 而非从空开始 (模拟 "回滚到此点然后继续跑").
            self._live_state = copy.deepcopy(target.state)
            for entry in self._wal:
                if entry.sequence > target.up_to_sequence:
                    if entry.event.kind in IDEMPOTENT_OPS:
                        self._apply_to_state(entry.event)
            return True

    def replay_events(
        self, from_ts: float, to_ts: float
    ) -> Iterator[Event]:
        """在 [from_ts, to_ts] 时间窗口内按 seq 顺序产出事件.

        闭区间语义: from_ts <= event_ts <= to_ts。
        返回 iterator 而非 list 以节约内存。
        """
        # 走 snapshot 防止迭代时被并发写入干扰
        with self._lock:
            snapshot = list(self._wal)
        for entry in snapshot:
            if from_ts <= entry.event.ts <= to_ts:
                yield entry.event

    def diff_states(self, state_a: StateID, state_b: StateID) -> StateDiff:
        """计算两个检查点之间的对称事件差.

        思路: 在 [seq_a+1, seq_b] 窗口里提取事件;
              - 在 state_a state dict 里出现过但不在 window 内的事件 = removed (来自 a 视角)
              - 在 window 内的事件 = added (来自 b 视角)
              - 改动的事件 (event_id 同, payload 不同) = changed
        """
        with self._lock:
            cp_a = self._find_checkpoint(state_a)
            cp_b = self._find_checkpoint(state_b)
            if cp_a is None or cp_b is None:
                return StateDiff()
            seq_a = cp_a.up_to_sequence
            seq_b = cp_b.up_to_sequence
            events_in_window = [
                entry.event
                for entry in self._wal
                if seq_a < entry.sequence <= seq_b
            ]

        # changed: 同 event_id 但 payload 不同
        seen: Dict[str, Event] = {}
        changed_pairs: List[Tuple[Event, Event]] = []
        for ev in events_in_window:
            if ev.event_id in seen:
                prev = seen[ev.event_id]
                if prev.payload != ev.payload:
                    changed_pairs.append((prev, ev))
            else:
                seen[ev.event_id] = ev

        # added (b 视角新增) / removed (a 视角去除)
        # 简化: 拿 a 的 state 字典 keys 当基线, 比对事件级别
        a_event_keys = set(self._state_event_keys(cp_a.state))
        b_event_keys = set()
        for ev in events_in_window:
            b_event_keys.add(f"{ev.kind}:{ev.event_id}")

        added = tuple(ev for ev in events_in_window if f"{ev.kind}:{ev.event_id}" not in a_event_keys)
        removed_keys = a_event_keys - b_event_keys
        # 不在 window 里的 a 事件 = removed (构造镜像)
        removed: List[Event] = []
        for k in sorted(removed_keys):
            kind, eid = k.split(":", 1)
            removed.append(Event(event_id=eid, ts=0.0, kind=kind, payload=()))

        return StateDiff(
            added=tuple(added),
            removed=tuple(removed),
            changed=tuple(changed_pairs),
        )

    def idempotent_apply(self, event: Event) -> ApplyResult:
        """幂等应用事件 (白名单 only).

        相同 event 重复调用 → 第二次返回 cached=True 且 status/event_hash 不变。
        非白名单 kind → rejected, 绝不静默通过 (V3 哲学守门).
        """
        with self._lock:
            if event.kind not in IDEMPOTENT_OPS:
                return ApplyResult(
                    status="rejected",
                    event_hash=_event_hash(event),
                    reason=f"op {event.kind!r} not whitelisted",
                )
            # 幂等 key: kind + event_id + payload hash (同 eid 不同 payload = 不同 key)
            payload_digest = hashlib.sha256(
                json.dumps(dict(event.payload), sort_keys=True, ensure_ascii=False).encode("utf-8")
            ).hexdigest()[:16]
            key = f"{event.kind}|{event.event_id}|{payload_digest}"
            prev = self._applied.get(key)
            if prev is not None:
                return ApplyResult(
                    status=prev.status,
                    event_hash=prev.event_hash,
                    cached=True,
                )
            result = ApplyResult(
                status="applied",
                event_hash=_event_hash(event),
            )
            self._applied[key] = result
            self._apply_to_state(event)
            # 同时落 WAL (幂等调用也只写一次)
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
            return result

    # ------------------------------------------------------------------
    # 辅助
    # ------------------------------------------------------------------

    def _find_checkpoint(self, state_id: StateID) -> Optional[Checkpoint]:
        for cp in self._checkpoints:
            if cp.state_id == state_id:
                return cp
        return None

    @staticmethod
    def _state_hash(state: Dict[str, Any]) -> str:
        canonical = json.dumps(state, sort_keys=True, ensure_ascii=False)
        return hashlib.sha256(canonical.encode("utf-8")).hexdigest()[:16]

    @staticmethod
    def _state_event_keys(state: Dict[str, Any]) -> Iterator[str]:
        for tag_k in state.get("tags", {}):
            yield f"tag_set:{tag_k}"
        for i, _ in enumerate(state.get("anchors", [])):
            yield f"anchor_link:{i}"
        for i in range(len(state.get("scores", []))):
            yield f"score_record:{i}"
        for i in range(len(state.get("phases", []))):
            yield f"phase_emit:{i}"
        for i in range(len(state.get("traces", []))):
            yield f"trace_record:{i}"

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
                "applied_cache": len(self._applied),
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
