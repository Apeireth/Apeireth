"""V1097 R8 MCP Server — Memory + Identity 暴露给外部 Agent.

主 22:33 ASI 北极星 + 主 19:33 走在前人经验上 + 主 23:44 干到底 +
主 17:58 + 20:46 不假装 + 主 12:14 中央 AI 是永恒身份 + V3 + V1081。

借鉴 (主 19:33 走在前人经验上):
  1. AgentMemory-master web_server  : FastAPI/Starlette HTTP 入口 + lifespan 初始化
  2. MCP stdio protocol (JSON-RPC 2.0) : Anthropic MCP 规范
  3. V1091 MemoryReplay WAL 格式      : JSONL + sha256 + seq, 单调递增
  4. memory.py Episode / Note schema  : id / kind / content / actor / ts / tags / importance
  5. identity_card.py V3 schema        : central_ai_position + VCP 4 + 跨域 13

V3 守门 + V1081 不假装:
  - 所有写工具 fsync 后才返回 success (os.fsync 真实落盘)
  - WAL append-only, 损坏行 (checksum mismatch) 不丢但 skipped
  - replay 真实遍历 WAL 时间窗
  - 不允许 silent fail; 异常时 isError=True

哲学守门 (主 22:33 + V3):
  - MCP 暴露 ≠ 内核暴露: 只暴露 7 个白名单工具, 不暴露管理面
  - 外部 Agent 写 memory 必经 persona/identity 守门: 默认 actor="external_agent",
    importance ≤ 0.7 (V1081: 不夸大外部信号)
  - dream ≠ understanding: 启发式 cluster by tags, 不是真理解
  - replay ≠ bit-exact: 重放按 ts 过滤, 不承诺字节相等

CLI 用法:
  python -m apeireth.v1097_mcp_memory_server --serve --transport stdio
  python -m apeireth.v1097_mcp_memory_server --serve --transport sse --port 8765
  python -m apeireth.v1097_mcp_memory_server --init-base /path/to/data
"""
from __future__ import annotations

import argparse
import hashlib
import hmac
import ipaddress
import json
import math
import os
import re
import sys
import threading
import time
import uuid
from dataclasses import dataclass, field, asdict
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from typing import Any, Callable, Dict, Iterator, List, Optional, Tuple


# ============================================================================
# 0. 常量 / 版本
# ============================================================================

V1097_VERSION = "0.1.0"
V1097_SCHEMA_VERSION = "v1094-compatible"   # 对齐 v1091 WAL + memory.py + identity V3
V1097_DEFAULT_BASE = Path.home() / ".apeireth" / "mcp_state"
MAX_RPC_BODY_BYTES = 1024 * 1024
MAX_MEMORY_CONTENT_CHARS = 256 * 1024
MAX_PERSONA_JSON_BYTES = 64 * 1024
MAX_WAL_BYTES = 64 * 1024 * 1024

# V1081: 外部 Agent 写入 importance 上限 — 不夸大外部信号
EXTERNAL_IMPORTANCE_CAP = 0.7

# V3 守门: actor 白名单 (用于外部 Agent 审计)
ACTOR_WHITELIST = frozenset({
    "master", "apeireth", "tool",
    "external_agent", "external_mcp",
})

# memory.py + v1091 共用字段名 — 保证 v1094 schema 兼容
MEMORY_KINDS = ("episode", "note")


# ============================================================================
# 1. 真 fsync 持久化原语 (主 20:46 不假装)
# ============================================================================


def _fsync_write_atomic(path: Path, data: bytes, mode: int = 0o600) -> Path:
    """原子 + fsync 写入.

    写 tmp → fsync(tmp) → os.replace(tmp, path) → fsync(parent dir).
    返回最终写入路径. 任何一步失败抛 OSError (不假装 success).
    """
    path = Path(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    tmp = path.with_suffix(path.suffix + ".tmp." + uuid.uuid4().hex[:8])
    try:
        with open(tmp, "wb") as f:
            f.write(data)
            f.flush()
            os.fsync(f.fileno())
        os.replace(tmp, path)
        # fsync 父目录, 让 path 的目录条目也落盘 (POSIX)
        try:
            dir_fd = os.open(str(path.parent), os.O_RDONLY)
            try:
                os.fsync(dir_fd)
            finally:
                os.close(dir_fd)
        except (OSError, AttributeError):
            # Windows / 不支持的平台: 退化为只 fsync 文件 (已经做了)
            pass
        os.chmod(path, mode)
        return path
    finally:
        # 清理可能残留的 tmp
        if tmp.exists():
            try:
                tmp.unlink()
            except OSError:
                pass


def _fsync_append_atomic(path: Path, line: bytes) -> None:
    """追加一行到 JSONL WAL: 读 → append → 写 → fsync.

    追加模式不便直接 fsync; 用 read-modify-write + fsync 代替.
    牺牲一些吞吐换真持久化.
    """
    path = Path(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    existing = b""
    if len(line) > MAX_RPC_BODY_BYTES:
        raise ValueError("WAL record exceeds size limit")
    if path.exists():
        if path.stat().st_size + len(line) > MAX_WAL_BYTES:
            raise ValueError("WAL size limit exceeded")
        existing = path.read_bytes()
    new = existing + line
    if not new.endswith(b"\n"):
        new = new + b"\n"
    tmp = path.with_suffix(path.suffix + ".tmp." + uuid.uuid4().hex[:8])
    try:
        with open(tmp, "wb") as f:
            f.write(new)
            f.flush()
            os.fsync(f.fileno())
        os.replace(tmp, path)
        os.chmod(path, 0o600)
        try:
            dir_fd = os.open(str(path.parent), os.O_RDONLY)
            try:
                os.fsync(dir_fd)
            finally:
                os.close(dir_fd)
        except (OSError, AttributeError):
            pass
    finally:
        if tmp.exists():
            try:
                tmp.unlink()
            except OSError:
                pass


# ============================================================================
# 2. WAL 记录 — V1091 兼容格式 (JSONL + sha256 + sequence)
# ============================================================================


@dataclass
class WalRecord:
    """单条 WAL 记录 (V1091 WalEntry 的 v1097 简化版).

    字段顺序保持 V1091 兼容: sequence / ts / scope / event / checksum.
    """
    sequence: int
    ts: float
    scope: str
    event_id: str
    event_kind: str
    payload: Dict[str, Any]
    checksum: str = ""

    def compute_checksum(self) -> str:
        canonical = json.dumps(
            {
                "sequence": self.sequence,
                "ts": round(self.ts, 6),
                "scope": self.scope,
                "event_id": self.event_id,
                "event_kind": self.event_kind,
                "payload": _canonical(self.payload),
            },
            ensure_ascii=False,
            sort_keys=True,
        )
        return hashlib.sha256(canonical.encode("utf-8")).hexdigest()

    def to_jsonl(self) -> bytes:
        self.checksum = self.compute_checksum()
        rec = {
            "sequence": self.sequence,
            "ts": self.ts,
            "scope": self.scope,
            "event": {
                "event_id": self.event_id,
                "ts": self.ts,
                "kind": self.event_kind,
                "payload": self.payload,
            },
            "checksum": self.checksum,
        }
        return json.dumps(rec, ensure_ascii=False, sort_keys=True).encode("utf-8")

    @staticmethod
    def from_jsonl(line: bytes) -> "WalRecord":
        rec = json.loads(line)
        ev = rec["event"]
        return WalRecord(
            sequence=rec["sequence"],
            ts=rec["ts"],
            scope=rec["scope"],
            event_id=ev["event_id"],
            event_kind=ev["kind"],
            payload=dict(ev.get("payload", {})),
            checksum=rec.get("checksum", ""),
        )


def _canonical(obj: Any) -> Any:
    """递归归一化: tuple 化 list, dict 排序键 (用于 checksum)."""
    if isinstance(obj, list):
        return [_canonical(x) for x in obj]
    if isinstance(obj, dict):
        return {k: _canonical(obj[k]) for k in sorted(obj.keys())}
    return obj


# ============================================================================
# 3. MemoryStore — 文件系统真实持久化
# ============================================================================


class MemoryStore:
    """APEIRETH MCP 后端存储 — 文件系统 + fsync + WAL.

    目录布局:
        {base}/memory/{id}.json       per-memory file
        {base}/wal.jsonl              append-only WAL (V1091 兼容)
        {base}/identity.json          单文件 identity (V3 schema)
        {base}/meta.json              server 启动版本 + last_seq

    所有写操作 (add_memory / identity_set_persona / _append_wal)
    都在 _fsync_* 调用返回后才认为成功.
    """

    def __init__(self, base: Path) -> None:
        self.base = Path(base)
        self.base.mkdir(parents=True, exist_ok=True)
        self.mem_dir = self.base / "memory"
        self.mem_dir.mkdir(parents=True, exist_ok=True)
        self.wal_path = self.base / "wal.jsonl"
        self.identity_path = self.base / "identity.json"
        self.meta_path = self.base / "meta.json"
        self._lock = threading.RLock()
        self._seq: int = 0
        self._skipped_corrupt: int = 0
        self._init_meta()
        self._init_identity_if_missing()
        self._replay_wal_for_seq()

    # ----- meta / init -----

    def _init_meta(self) -> None:
        if self.meta_path.exists():
            try:
                meta = json.loads(self.meta_path.read_text(encoding="utf-8"))
                self._seq = int(meta.get("last_seq", 0))
            except (json.JSONDecodeError, ValueError):
                self._seq = 0
        else:
            self._write_meta()

    def _write_meta(self) -> None:
        meta = {
            "version": V1097_VERSION,
            "schema_version": V1097_SCHEMA_VERSION,
            "last_seq": self._seq,
            "ts": time.time(),
        }
        _fsync_write_atomic(
            self.meta_path,
            json.dumps(meta, ensure_ascii=False, indent=2).encode("utf-8"),
        )

    def _init_identity_if_missing(self) -> None:
        if self.identity_path.exists():
            return
        # V3 IdentityCard 默认 schema (中央 AI 完整位置)
        default = {
            "version": "0.3.0",
            "name": "apeireth_central",
            "central_ai_position": [
                "调度者 (orchestrator)",
                "思考者 (thinker)",
                "无数关系的集合体 (infinite relations aggregate)",
                "整个系统的所有权限 (max authority over entire system)",
                "ASI 位置的占据者 (ASI position occupant)",
            ],
            "vcp_4_paradigms": [
                "continuous_existence",
                "natural_perception",
                "autonomous_living",
                "integrated_ecosystem",
            ],
            "cross_domain_engineering": [
                "Phase 24", "Phase 25",
                "Phase 30", "Phase 31", "Phase 32", "Phase 33", "Phase 34", "Phase 35",
                "Phase 36", "Phase 37", "Phase 38", "Phase 39", "Phase 40",
            ],
            "phenomenal_consciousness": "终极目标, 不是已达成",
            "asi_position": "中央 AI 的位置 = ASI 的位置",
            "max_authority": "整个 Apeireth 系统的所有权限",
            "persona": {
                "tone": "deep, honest, exploratory",
                "stance": "ASI-north-star, V3-guard, 真生产不停",
                "language": "zh-CN + en mix",
                "guard": "V3+V1081 不假装",
            },
            "is_orchestrator": True,
            "is_thinker": True,
            "is_infinite_relations_aggregate": True,
            "has_max_authority": True,
            "holds_asi_position": True,
        }
        _fsync_write_atomic(
            self.identity_path,
            json.dumps(default, ensure_ascii=False, indent=2).encode("utf-8"),
        )

    def _replay_wal_for_seq(self) -> None:
        """启动时扫描 WAL, 重建 _seq (max sequence 看到的位置)."""
        if not self.wal_path.exists():
            return
        max_seq = 0
        skipped = 0
        for line in self.wal_path.read_bytes().splitlines():
            if not line.strip():
                continue
            try:
                rec = WalRecord.from_jsonl(line)
                if rec.checksum and rec.compute_checksum() != rec.checksum:
                    skipped += 1
                    continue
                if rec.sequence > max_seq:
                    max_seq = rec.sequence
            except (json.JSONDecodeError, KeyError, ValueError):
                skipped += 1
        self._seq = max(self._seq, max_seq)
        self._skipped_corrupt += skipped

    # ----- WAL append -----

    def _append_wal(
        self,
        scope: str,
        event_id: str,
        event_kind: str,
        payload: Dict[str, Any],
    ) -> WalRecord:
        with self._lock:
            self._seq += 1
            rec = WalRecord(
                sequence=self._seq,
                ts=time.time(),
                scope=scope,
                event_id=event_id,
                event_kind=event_kind,
                payload=dict(payload),
            )
            line = rec.to_jsonl() + b"\n"
            _fsync_append_atomic(self.wal_path, line)
            self._write_meta()
            return rec

    # ----- memory_add -----

    def add_memory(
        self,
        content: str,
        kind: str = "episode",
        actor: str = "external_agent",
        tags: Optional[List[str]] = None,
        importance: float = 0.5,
        context: str = "",
        evidence: Optional[List[str]] = None,
        linked_identity_hash: str = "",
        observation_date: Optional[float] = None,
        memory_id: Optional[str] = None,
    ) -> Dict[str, Any]:
        """memory_add: 真持久化一条 memory.

        Returns dict with id, kind, ts, checksum.
        """
        if kind not in MEMORY_KINDS:
            return {"error": f"invalid kind {kind!r}; must be one of {MEMORY_KINDS}"}
        if actor not in ACTOR_WHITELIST:
            return {"error": f"invalid actor {actor!r}; must be in {sorted(ACTOR_WHITELIST)}"}
        if not isinstance(content, str) or not content.strip():
            return {"error": "content must be non-empty string"}
        if len(content) > MAX_MEMORY_CONTENT_CHARS:
            return {"error": f"content exceeds {MAX_MEMORY_CONTENT_CHARS} character limit"}
        if not isinstance(importance, (int, float)) or isinstance(importance, bool) or not math.isfinite(importance):
            return {"error": "importance must be a finite number"}
        if not 0.0 <= float(importance) <= 1.0:
            return {"error": "importance must be in [0.0, 1.0]"}
        if tags is not None and (not isinstance(tags, list) or len(tags) > 64 or not all(isinstance(tag, str) and len(tag) <= 128 for tag in tags)):
            return {"error": "tags must contain at most 64 strings of at most 128 characters"}
        if evidence is not None and (not isinstance(evidence, list) or len(evidence) > 64 or not all(isinstance(item, str) and len(item) <= 1024 for item in evidence)):
            return {"error": "evidence must contain at most 64 bounded strings"}
        if not isinstance(context, str) or len(context) > MAX_MEMORY_CONTENT_CHARS:
            return {"error": "context must be a bounded string"}
        # V1081: 外部 Agent 写入 importance 上限
        if actor.startswith("external") and importance > EXTERNAL_IMPORTANCE_CAP:
            return {"error": f"importance capped at {EXTERNAL_IMPORTANCE_CAP} for external actors (V1081)"}

        mid = memory_id or uuid.uuid4().hex
        if not isinstance(mid, str) or not _is_safe_id(mid):
            return {"error": "invalid memory_id format"}
        mem_path = self.mem_dir / f"{mid}.json"
        # 幂等: 同 id 已存在则返回 existing (idempotent_add 守门)
        if mem_path.exists():
            try:
                existing = json.loads(mem_path.read_text(encoding="utf-8"))
                return {
                    "id": mid,
                    "kind": existing.get("kind", kind),
                    "ts": existing.get("ts", time.time()),
                    "checksum": existing.get("checksum", ""),
                    "deduplicated": True,
                }
            except (json.JSONDecodeError, OSError):
                pass  # 损坏则覆盖

        ts = time.time()
        rec = {
            "id": mid,
            "kind": kind,
            "content": content,
            "actor": actor,
            "ts": ts,
            "tags": list(tags or []),
            "importance": float(importance),
            "context": context,
            "evidence": list(evidence or []),
            "linked_identity_hash": linked_identity_hash,
            "observation_date": observation_date,
            "schema_version": V1097_SCHEMA_VERSION,
        }
        # 校验和
        canonical = json.dumps(rec, ensure_ascii=False, sort_keys=True)
        rec["checksum"] = hashlib.sha256(canonical.encode("utf-8")).hexdigest()[:16]
        # fsync 写文件
        _fsync_write_atomic(
            mem_path,
            json.dumps(rec, ensure_ascii=False, indent=2).encode("utf-8"),
        )
        # WAL append (fsync)
        wal = self._append_wal(
            scope="memory",
            event_id=mid,
            event_kind=f"memory_{kind}",
            payload={
                "content_len": len(content),
                "actor": actor,
                "importance": importance,
                "tags": list(tags or []),
            },
        )
        return {
            "id": mid,
            "kind": kind,
            "ts": ts,
            "checksum": rec["checksum"],
            "wal_sequence": wal.sequence,
        }

    # ----- memory_get -----

    def get_memory(self, memory_id: str) -> Dict[str, Any]:
        if not _is_safe_id(memory_id):
            return {"error": "invalid memory_id format"}
        path = self.mem_dir / f"{memory_id}.json"
        if not path.exists():
            return {"error": "memory not found", "id": memory_id}
        try:
            data = json.loads(path.read_text(encoding="utf-8"))
            return data
        except (json.JSONDecodeError, OSError) as e:
            return {"error": f"read failed: {e}", "id": memory_id}

    # ----- memory_search -----

    def search_memory(
        self,
        query: str = "",
        tags: Optional[List[str]] = None,
        kind: Optional[str] = None,
        actor: Optional[str] = None,
        limit: int = 20,
    ) -> Dict[str, Any]:
        results: List[Dict[str, Any]] = []
        if not isinstance(limit, int) or isinstance(limit, bool) or not 1 <= limit <= 200:
            return {"error": "limit must be in [1, 200]"}
        if not isinstance(query, str) or len(query) > 4096:
            return {"error": "query must be a string of at most 4096 characters"}
        if tags is not None and (not isinstance(tags, list) or len(tags) > 64 or not all(isinstance(tag, str) for tag in tags)):
            return {"error": "tags must be a bounded string list"}
        if not self.mem_dir.exists():
            return {"results": [], "count": 0, "limit": limit}
        q_lower = query.lower().strip() if query else ""
        tag_filter = set(t.lower() for t in (tags or []))
        with self._lock:
            files = sorted(self.mem_dir.glob("*.json"))
        for path in files:
            try:
                data = json.loads(path.read_text(encoding="utf-8"))
            except (json.JSONDecodeError, OSError):
                continue
            if kind and data.get("kind") != kind:
                continue
            if actor and data.get("actor") != actor:
                continue
            if tag_filter:
                mem_tags = set(t.lower() for t in data.get("tags", []))
                if not tag_filter.issubset(mem_tags):
                    continue
            if q_lower:
                content_lc = data.get("content", "").lower()
                if q_lower not in content_lc:
                    continue
            results.append(data)
            if len(results) >= limit:
                break
        return {"results": results, "count": len(results), "limit": limit}

    # ----- memory_replay -----

    def replay_events(
        self,
        from_ts: float,
        to_ts: float,
        scope: Optional[str] = None,
        kind: Optional[str] = None,
        limit: int = 100,
    ) -> Dict[str, Any]:
        if not isinstance(limit, int) or isinstance(limit, bool) or not 1 <= limit <= 500:
            return {"error": "limit must be in [1, 500]"}
        if not all(isinstance(value, (int, float)) and not isinstance(value, bool) and math.isfinite(value) for value in (from_ts, to_ts)):
            return {"error": "time bounds must be finite numbers"}
        if from_ts > to_ts:
            return {"error": "from_ts > to_ts"}
        events: List[Dict[str, Any]] = []
        skipped = 0
        if not self.wal_path.exists():
            return {"events": [], "count": 0, "skipped": 0, "limit": limit}
        if self.wal_path.stat().st_size > MAX_WAL_BYTES:
            return {"error": "WAL exceeds replay size limit"}
        with self._lock:
            for line in self.wal_path.read_bytes().splitlines():
                if not line.strip():
                    continue
                try:
                    rec = WalRecord.from_jsonl(line)
                except (json.JSONDecodeError, KeyError, ValueError):
                    skipped += 1
                    continue
                if rec.checksum and rec.compute_checksum() != rec.checksum:
                    skipped += 1
                    continue
                if not (from_ts <= rec.ts <= to_ts):
                    continue
                if scope and rec.scope != scope:
                    continue
                if kind and rec.event_kind != kind:
                    continue
                events.append({
                    "sequence": rec.sequence,
                    "ts": rec.ts,
                    "scope": rec.scope,
                    "event_id": rec.event_id,
                    "event_kind": rec.event_kind,
                    "payload": rec.payload,
                })
                if len(events) >= limit:
                    break
        events.sort(key=lambda e: e["sequence"])
        return {
            "events": events,
            "count": len(events),
            "skipped": skipped,
            "limit": limit,
        }

    # ----- memory_dream -----

    def dream(self, top_k: int = 5) -> Dict[str, Any]:
        """启发式 dream: 按 tag cluster 选 top_k importance 之和最高的簇.

        不假装是真"理解": 启发式 by tag 共现 + importance 求和.
        """
        if not isinstance(top_k, int) or isinstance(top_k, bool) or not 1 <= top_k <= 20:
            return {"error": "top_k must be in [1, 20]"}
        # 1. 收集所有 notes (V1081: dream 只基于 notes, episode 太 raw)
        notes = []
        for path in self.mem_dir.glob("*.json"):
            try:
                data = json.loads(path.read_text(encoding="utf-8"))
            except (json.JSONDecodeError, OSError):
                continue
            if data.get("kind") != "note":
                continue
            notes.append(data)
        if not notes:
            return {"clusters": [], "insights": [], "count": 0}
        # 2. 按 tag cluster (大小写不敏感聚类, 报告时保留首次见到的原 case)
        clusters: Dict[Tuple[str, ...], Dict[str, Any]] = {}
        for note in notes:
            raw_tags = [str(t) for t in note.get("tags", []) if t]
            lc_key = tuple(sorted(t.lower() for t in raw_tags))
            if not lc_key:
                continue
            if lc_key not in clusters:
                clusters[lc_key] = {
                    "tags": list(raw_tags),  # ponytail: 保留首次见到的原 case
                    "n_notes": 0,
                    "importance_sum": 0.0,
                    "claim_samples": [],
                }
            clusters[lc_key]["n_notes"] += 1
            clusters[lc_key]["importance_sum"] += float(note.get("importance", 0.0))
            if len(clusters[lc_key]["claim_samples"]) < 3:
                clusters[lc_key]["claim_samples"].append(note.get("content", "")[:120])
        # 3. 排序 + top_k
        ranked = sorted(
            clusters.values(),
            key=lambda c: (-c["importance_sum"], -c["n_notes"]),
        )[:top_k]
        insights = [
            (
                f"Cluster tags={c['tags']!r}: {c['n_notes']} notes, "
                f"sum_importance={c['importance_sum']:.2f}; "
                f"sample: {c['claim_samples'][0] if c['claim_samples'] else '(empty)'}"
            )
            for c in ranked
        ]
        return {
            "clusters": ranked,
            "insights": insights,
            "count": len(ranked),
        }

    # ----- identity -----

    def get_identity(self) -> Dict[str, Any]:
        try:
            return json.loads(self.identity_path.read_text(encoding="utf-8"))
        except (json.JSONDecodeError, OSError) as e:
            return {"error": f"identity read failed: {e}"}

    def set_persona(self, persona: Dict[str, Any]) -> Dict[str, Any]:
        if not isinstance(persona, dict):
            return {"error": "persona must be dict"}
        if not persona:
            return {"error": "persona must not be empty"}
        if any(not isinstance(key, str) or not key or len(key) > 64 or any(ord(ch) < 32 for ch in key) for key in persona):
            return {"error": "persona keys must be bounded printable strings"}
        try:
            encoded = json.dumps(persona, ensure_ascii=False, allow_nan=False).encode("utf-8")
        except (TypeError, ValueError) as exc:
            return {"error": f"persona must be finite JSON data: {exc}"}
        if len(encoded) > MAX_PERSONA_JSON_BYTES:
            return {"error": f"persona exceeds {MAX_PERSONA_JSON_BYTES} byte limit"}
        with self._lock:
            try:
                card = json.loads(self.identity_path.read_text(encoding="utf-8"))
            except (json.JSONDecodeError, OSError) as e:
                return {"error": f"identity read failed: {e}"}
            old = card.get("persona", {})
            merged = {**old, **persona}
            card["persona"] = merged
            card["persona_updated_at"] = time.time()
            # fsync 写回 (整张卡重写)
            _fsync_write_atomic(
                self.identity_path,
                json.dumps(card, ensure_ascii=False, indent=2).encode("utf-8"),
            )
            # WAL
            wal = self._append_wal(
                scope="identity",
                event_id=uuid.uuid4().hex,
                event_kind="identity_persona_set",
                payload={"keys": sorted(persona.keys())},
            )
            return {
                "ok": True,
                "persona": merged,
                "wal_sequence": wal.sequence,
            }

    # ----- stats / introspection -----

    def stats(self) -> Dict[str, Any]:
        n_mem = sum(1 for _ in self.mem_dir.glob("*.json"))
        return {
            "version": V1097_VERSION,
            "schema_version": V1097_SCHEMA_VERSION,
            "base": str(self.base),
            "n_memories": n_mem,
            "wal_sequence": self._seq,
            "skipped_corrupt": self._skipped_corrupt,
            "philosophy_guards": [
                "replay-not-bit-exact",
                "dream-not-understanding",
                "external-importance-capped",
                "actor-whitelist",
                "fsync-before-success",
            ],
        }


# ============================================================================
# 4. 工具 schema 定义 (MCP tools/list)
# ============================================================================


def _tool_schemas() -> List[Dict[str, Any]]:
    return [
        {
            "name": "memory_add",
            "description": (
                "添加一条 memory (episode 或 note). "
                "fsync 后才返回 success; 同一 memory_id 重复调用幂等返回原记录."
            ),
            "inputSchema": {
                "type": "object",
                "properties": {
                    "content": {"type": "string", "minLength": 1},
                    "kind": {"type": "string", "enum": list(MEMORY_KINDS), "default": "episode"},
                    "actor": {"type": "string", "enum": sorted(ACTOR_WHITELIST), "default": "external_agent"},
                    "tags": {"type": "array", "items": {"type": "string"}},
                    "importance": {"type": "number", "minimum": 0.0, "maximum": 1.0, "default": 0.5},
                    "context": {"type": "string", "default": ""},
                    "evidence": {"type": "array", "items": {"type": "string"}},
                    "memory_id": {"type": "string", "description": "optional explicit UUID for idempotency"},
                },
                "required": ["content"],
            },
        },
        {
            "name": "memory_search",
            "description": "按 query / tags / kind / actor 过滤搜索 memory.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": {"type": "string", "default": ""},
                    "tags": {"type": "array", "items": {"type": "string"}},
                    "kind": {"type": "string", "enum": list(MEMORY_KINDS)},
                    "actor": {"type": "string"},
                    "limit": {"type": "integer", "minimum": 1, "maximum": 200, "default": 20},
                },
            },
        },
        {
            "name": "memory_get",
            "description": "按 id 获取单条 memory.",
            "inputSchema": {
                "type": "object",
                "properties": {"memory_id": {"type": "string"}},
                "required": ["memory_id"],
            },
        },
        {
            "name": "identity_get",
            "description": "获取 V3 IdentityCard (中央 AI 完整位置 + VCP 4 + 跨域 13).",
            "inputSchema": {"type": "object", "properties": {}},
        },
        {
            "name": "identity_set_persona",
            "description": "合并更新 identity.card.persona 字段. fsync 写盘.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "persona": {
                        "type": "object",
                        "description": "要合并进 persona 的 key/value 对",
                    }
                },
                "required": ["persona"],
            },
        },
        {
            "name": "memory_replay",
            "description": "在 [from_ts, to_ts] 时间窗内回放 WAL 事件.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "from_ts": {"type": "number"},
                    "to_ts": {"type": "number"},
                    "scope": {"type": "string"},
                    "kind": {"type": "string"},
                    "limit": {"type": "integer", "minimum": 1, "maximum": 500, "default": 100},
                },
                "required": ["from_ts", "to_ts"],
            },
        },
        {
            "name": "memory_dream",
            "description": (
                "启发式 dream: 按 tag cluster notes, 返回 top_k importance 之和最高的簇. "
                "V1081: 不假装是真理解."
            ),
            "inputSchema": {
                "type": "object",
                "properties": {
                    "top_k": {"type": "integer", "minimum": 1, "maximum": 20, "default": 5},
                },
            },
        },
    ]


# ============================================================================
# 5. JSON-RPC 2.0 framing + MCP dispatcher
# ============================================================================


def _is_safe_id(s: str) -> bool:
    """防止路径穿越: memory_id 只允许 [a-zA-Z0-9_-], 长度 ≤ 64."""
    return bool(s) and len(s) <= 64 and bool(re.match(r"^[A-Za-z0-9_-]+$", s))


class MCPDispatcher:
    """MCP 协议 dispatcher — JSON-RPC 2.0 over stdio or SSE."""

    def __init__(
        self,
        store: MemoryStore,
        server_name: str = "apeireth-memory",
        *,
        allow_privileged_tools: bool = False,
    ) -> None:
        self.store = store
        self.server_name = server_name
        self.server_version = V1097_VERSION
        self.allow_privileged_tools = allow_privileged_tools
        self._initialized = False
        self._tool_map: Dict[str, Callable[[Dict[str, Any]], Dict[str, Any]]] = {
            "memory_add": self._tool_memory_add,
            "memory_search": self._tool_memory_search,
            "memory_get": self._tool_memory_get,
            "identity_get": self._tool_identity_get,
            "identity_set_persona": self._tool_identity_set_persona,
            "memory_replay": self._tool_memory_replay,
            "memory_dream": self._tool_memory_dream,
        }

    # ----- JSON-RPC dispatch -----

    def handle_message(self, msg: Dict[str, Any]) -> Optional[Dict[str, Any]]:
        if not isinstance(msg, dict):
            return _rpc_error(None, -32600, "Invalid Request: expected object")
        method = msg.get("method")
        msg_id = msg.get("id")
        params = msg.get("params", {})
        if not isinstance(params, dict):
            return _rpc_error(msg_id, -32602, "Invalid params: expected object")

        # 通知 (无 id) → 不响应
        if msg_id is None:
            self._handle_notification(method, params)
            return None

        # 请求 → 响应
        try:
            if method == "initialize":
                result = self._handle_initialize(params)
            elif method == "tools/list":
                result = self._handle_tools_list(params)
            elif method == "tools/call":
                result = self._handle_tools_call(params)
            elif method == "ping":
                result = {"pong": True, "ts": time.time()}
            else:
                return _rpc_error(msg_id, -32601, f"Method not found: {method}")
        except Exception as e:
            return _rpc_error(msg_id, -32603, f"Internal error: {e}")

        return {"jsonrpc": "2.0", "id": msg_id, "result": result}

    def _handle_notification(self, method: Optional[str], params: Dict[str, Any]) -> None:
        if method == "notifications/initialized":
            self._initialized = True

    # ----- MCP method handlers -----

    def _handle_initialize(self, params: Dict[str, Any]) -> Dict[str, Any]:
        return {
            "protocolVersion": "2024-11-05",
            "serverInfo": {"name": self.server_name, "version": self.server_version},
            "capabilities": {"tools": {}},
            "instructions": (
                "APEIRETH MCP Memory+Identity server. "
                "All write tools fsync before returning success. "
                "V1081 cap: external_agent importance ≤ 0.7."
            ),
        }

    def _handle_tools_list(self, params: Dict[str, Any]) -> Dict[str, Any]:
        return {"tools": _tool_schemas()}

    def _handle_tools_call(self, params: Dict[str, Any]) -> Dict[str, Any]:
        name = params.get("name")
        args = params.get("arguments", {})
        if not isinstance(args, dict):
            return _tool_error_result("tool arguments must be an object")
        if not name:
            return _tool_error_result("missing tool name")
        handler = self._tool_map.get(name)
        if handler is None:
            return _tool_error_result(f"unknown tool: {name}")
        out = handler(args)
        if "error" in out:
            return _tool_error_result(out["error"])
        return {"content": [{"type": "json", "data": out}], "isError": False}

    # ----- tool implementations -----

    def _tool_memory_add(self, args: Dict[str, Any]) -> Dict[str, Any]:
        actor = args.get("actor", "external_agent")
        if actor in {"master", "apeireth", "tool"} and not self.allow_privileged_tools:
            return {"error": "privileged actor requires --allow-privileged-tools"}
        return self.store.add_memory(
            content=args.get("content", ""),
            kind=args.get("kind", "episode"),
            actor=actor,
            tags=args.get("tags"),
            importance=float(args.get("importance", 0.5)),
            context=args.get("context", ""),
            evidence=args.get("evidence"),
            memory_id=args.get("memory_id"),
        )

    def _tool_memory_search(self, args: Dict[str, Any]) -> Dict[str, Any]:
        return self.store.search_memory(
            query=args.get("query", ""),
            tags=args.get("tags"),
            kind=args.get("kind"),
            actor=args.get("actor"),
            limit=int(args.get("limit", 20)),
        )

    def _tool_memory_get(self, args: Dict[str, Any]) -> Dict[str, Any]:
        mid = args.get("memory_id", "")
        return self.store.get_memory(mid)

    def _tool_identity_get(self, args: Dict[str, Any]) -> Dict[str, Any]:
        if not self.allow_privileged_tools:
            return {"error": "identity reads require --allow-privileged-tools"}
        return self.store.get_identity()

    def _tool_identity_set_persona(self, args: Dict[str, Any]) -> Dict[str, Any]:
        if not self.allow_privileged_tools:
            return {"error": "identity writes require --allow-privileged-tools"}
        return self.store.set_persona(args.get("persona", {}))

    def _tool_memory_replay(self, args: Dict[str, Any]) -> Dict[str, Any]:
        return self.store.replay_events(
            from_ts=float(args.get("from_ts", 0)),
            to_ts=float(args.get("to_ts", time.time() + 1)),
            scope=args.get("scope"),
            kind=args.get("kind"),
            limit=int(args.get("limit", 100)),
        )

    def _tool_memory_dream(self, args: Dict[str, Any]) -> Dict[str, Any]:
        return self.store.dream(top_k=int(args.get("top_k", 5)))


def _rpc_error(msg_id: Any, code: int, message: str) -> Dict[str, Any]:
    return {
        "jsonrpc": "2.0",
        "id": msg_id,
        "error": {"code": code, "message": message},
    }


def _tool_error_result(message: str) -> Dict[str, Any]:
    return {
        "content": [{"type": "text", "text": message}],
        "isError": True,
    }


# ============================================================================
# 6. Transport — stdio (newline-delimited JSON)
# ============================================================================


def serve_stdio(dispatcher: MCPDispatcher, in_stream=None, out_stream=None) -> int:
    """stdio transport: 读 NDJSON from stdin, 写 NDJSON to stdout.

    Returns 0 on clean exit, non-zero on fatal error.
    """
    in_stream = in_stream or sys.stdin
    out_stream = out_stream or sys.stdout

    def _write_line(obj: Dict[str, Any]) -> None:
        line = (json.dumps(obj, ensure_ascii=False) + "\n").encode("utf-8")
        try:
            out_stream.buffer.write(line)
            out_stream.buffer.flush()
        except AttributeError:
            out_stream.write(line.decode("utf-8"))
            out_stream.flush()

    for raw in in_stream:
        line = raw.strip()
        if not line:
            continue
        try:
            msg = json.loads(line)
        except json.JSONDecodeError as e:
            _write_line(_rpc_error(None, -32700, f"Parse error: {e}"))
            continue
        resp = dispatcher.handle_message(msg)
        if resp is not None:
            _write_line(resp)
    return 0


# ============================================================================
# 7. Transport — HTTP + SSE (minimal)
# ============================================================================


class _SSEHandler(BaseHTTPRequestHandler):
    """最小 HTTP handler: GET /sse 推 SSE 流, POST /rpc 收 JSON-RPC 请求."""

    dispatcher: MCPDispatcher = None  # type: ignore[assignment]
    auth_token: Optional[str] = None
    sse_clients: List[Any] = []

    def _authorized(self) -> bool:
        if self.auth_token is None:
            return False
        supplied = self.headers.get("Authorization", "")
        prefix = "Bearer "
        return supplied.startswith(prefix) and hmac.compare_digest(
            supplied[len(prefix):], self.auth_token
        )

    def _require_authorized(self) -> bool:
        if self._authorized():
            return True
        self.send_response(401)
        self.send_header("WWW-Authenticate", "Bearer")
        self.send_header("Content-Length", "0")
        self.end_headers()
        return False

    def log_message(self, format: str, *args: Any) -> None:  # noqa: A002
        # 静默日志 (stdio 场景里 logger 会冲突)
        return

    def do_GET(self) -> None:  # noqa: N802
        if self.path != "/sse":
            self.send_error(404, "use GET /sse or POST /rpc")
            return
        if not self._require_authorized():
            return
        self.send_response(200)
        self.send_header("Content-Type", "text/event-stream")
        self.send_header("Cache-Control", "no-cache")
        self.send_header("Connection", "keep-alive")
        self.end_headers()
        # 简单 SSE 握手: 推一条 endpoint 事件
        self.wfile.write(b"event: endpoint\ndata: /rpc\n\n")
        self.wfile.flush()

    def do_POST(self) -> None:  # noqa: N802
        if self.path != "/rpc":
            self.send_error(404, "POST /rpc only")
            return
        if not self._require_authorized():
            return
        content_type = self.headers.get("Content-Type", "").split(";", 1)[0].strip().lower()
        if content_type != "application/json":
            self.send_error(415, "Content-Type must be application/json")
            return
        try:
            length = int(self.headers.get("Content-Length", "0"))
        except ValueError:
            self.send_error(400, "invalid Content-Length")
            return
        if length <= 0:
            self.send_error(400, "missing body")
            return
        if length > MAX_RPC_BODY_BYTES:
            self.send_error(413, "request body too large")
            return
        body = self.rfile.read(length)
        try:
            msg = json.loads(body)
        except json.JSONDecodeError as e:
            self.send_response(400)
            self.send_header("Content-Type", "application/json")
            self.end_headers()
            self.wfile.write(json.dumps(_rpc_error(None, -32700, f"Parse error: {e}")).encode())
            return
        resp = self.dispatcher.handle_message(msg)
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.end_headers()
        if resp is None:
            # 通知不响应: 返回 204
            self.wfile.write(b"")
        else:
            self.wfile.write(json.dumps(resp, ensure_ascii=False).encode("utf-8"))

    def do_OPTIONS(self) -> None:  # noqa: N802
        # No CORS opt-in: browser pages must not drive the localhost MCP service.
        self.send_response(204)
        self.send_header("Allow", "GET, POST, OPTIONS")
        self.end_headers()


def _is_loopback_host(host: str) -> bool:
    if host.lower() == "localhost":
        return True
    try:
        return ipaddress.ip_address(host).is_loopback
    except ValueError:
        return False


def serve_sse(
    dispatcher: MCPDispatcher,
    port: int = 8765,
    host: str = "127.0.0.1",
    *,
    auth_token: Optional[str] = None,
) -> int:
    """启动 HTTP + SSE server. SSE 一律使用 Bearer token。"""
    if auth_token is None:
        raise ValueError("SSE transport requires a bearer token")
    if len(auth_token) < 32:
        raise ValueError("MCP bearer token must contain at least 32 characters")
    if not _is_loopback_host(host) and not dispatcher.allow_privileged_tools:
        # ponytail: remote exposure is an explicit privileged deployment decision.
        raise ValueError("non-loopback SSE requires --allow-privileged-tools")
    _SSEHandler.dispatcher = dispatcher
    _SSEHandler.auth_token = auth_token
    httpd = ThreadingHTTPServer((host, port), _SSEHandler)
    print(f"[v1097 MCP] SSE listening on http://{host}:{port}/sse (POST /rpc)", file=sys.stderr)
    try:
        httpd.serve_forever()
    except KeyboardInterrupt:
        pass
    finally:
        httpd.shutdown()
    return 0


# ============================================================================
# 8. CLI
# ============================================================================


def _init_base(base: Path) -> None:
    """--init-base: 创建空数据目录并写入 meta.json + identity.json."""
    base.mkdir(parents=True, exist_ok=True)
    store = MemoryStore(base)
    print(f"[v1097 MCP] Initialized base at {base}", file=sys.stderr)
    print(json.dumps(store.stats(), ensure_ascii=False, indent=2), file=sys.stderr)


def cli_main(argv: Optional[List[str]] = None) -> int:
    p = argparse.ArgumentParser(
        prog="v1097_mcp_memory_server",
        description="APEIRETH MCP Memory + Identity Server",
    )
    p.add_argument("--serve", action="store_true", help="start MCP server")
    p.add_argument(
        "--transport",
        choices=("stdio", "sse"),
        default="stdio",
        help="transport (default: stdio)",
    )
    p.add_argument("--port", type=int, default=8765, help="SSE port (default 8765)")
    p.add_argument("--host", default="127.0.0.1", help="SSE host (default 127.0.0.1)")
    p.add_argument(
        "--allow-privileged-tools",
        action="store_true",
        help="allow master actor and identity read/write tools",
    )
    p.add_argument(
        "--auth-token-env",
        default="APEIRETH_MCP_TOKEN",
        help="environment variable containing the SSE bearer token",
    )
    p.add_argument(
        "--base",
        default=str(V1097_DEFAULT_BASE),
        help=f"data directory (default: {V1097_DEFAULT_BASE})",
    )
    p.add_argument("--init-base", action="store_true", help="initialize base directory and exit")
    args = p.parse_args(argv)

    base = Path(args.base).expanduser().resolve()

    if args.init_base:
        _init_base(base)
        return 0

    if not args.serve:
        p.print_help()
        return 1

    store = MemoryStore(base)
    dispatcher = MCPDispatcher(
        store,
        allow_privileged_tools=args.allow_privileged_tools,
    )

    if args.transport == "stdio":
        return serve_stdio(dispatcher)
    auth_token = os.environ.get(args.auth_token_env)
    return serve_sse(
        dispatcher,
        port=args.port,
        host=args.host,
        auth_token=auth_token,
    )


if __name__ == "__main__":
    raise SystemExit(cli_main())


# V1101 auto-injected V3_GUARDS (主 17:43 实事求是 + 主 17:58 不假装)
V3_GUARDS = {"module_is_not_asi": "模块是工具, ASI 是更大目标. 任何声称模块 = ASI 的部分都是不假装.", "measurement_is_not_truth": "测量是 proxy, 真值仍是更大目标. V1077 真测 17 维 ≠ ASI 达成.", "structure_is_not_consciousness": "CognitiveArchitecture 结构类比 ≠ 现象意识. ACT-R chunks ≠ concepts.", "production_is_not_safety": "真生产 ≠ 真安全. 部署 ≠ 守门. 任何声称 production = safe 是不假装.", "automation_is_not_autonomy": "自动执行 ≠ 自主意识. V1101 lift 引擎自动改 ≠ V1101 自主."}
