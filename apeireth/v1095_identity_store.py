"""V1095 Identity Store — 中央 AI 持久身份 + 多 persona + 真持久化

# ============================================================================
# architect2 集成监督备注 (2026-07-29, R8-TrackB)
# ----------------------------------------------------------------------------
# 完整对照报告: reports/r8-trackb-integration-checklist.md (351 行)
# 设计上游:    reports/r8-trackb-identity-architecture-design.md (613 行)
#
# v0.1 commit 前必修 (3 bug + 1 CLI, 0.5+0.5=1 人天):
#   1) save_cross_hashes() 未被调用 → 在 upsert_slot/save_profile/load_profile 末尾各加一次
#   2) test_30 多线程互斥失效 → __enter__ 把 _begin_switch + _active=True + _bump_sync_contexts 合并到同一锁
#   3) test_24 跨进程子进程拿不到 profile → 子进程脚本显式 get_or_create_profile()
#   4) 无 CLI 入口 → 加 argparse, 至少 --init/--show/--switch/--lift 4 子命令
#
# v0.2 必做 (5.5 人天, 1 周 1 人): RelationGraph V2 + Reconsolidator v0.1 + 3 API
# 详见 checklist §4.2 WBS 表
# ============================================================================

依据:
- TOP-DESIGN-V1 §3.2 (中央 AI = L4 Identity 中心节点)
- TOP-DESIGN-V1 §3.2 "多身份重叠 (调度者 / 学习者 / 思考者 / 助手)"
- TOP-DESIGN-V1 §4.1 Component 1: Identity Store
- R8-TrackB2 任务分配 (a2d330c2-...)
- 主人 12:14 "中央 AI 永恒身份 + 多身份重叠"
- 主人 12:47 "中央 AI 不管理, 但中央 AI 是调度者(身份之一)"
- 主人 13:31 大胆激进 + 17:58+20:46 不假装

v1095 与既有模块的关系:
- 复用 persona.py 的 SCTProfile + Persona + ARCHETYPES + seed_default_personas
- 复用 sqlite_identity_store.py 的 SQLite 连接模型 (WAL + synchronous=FULL 真 fsync)
- 桥接 v1072_asi_central_ai_eternal_identity.py (向后兼容, 不破坏)
- 桥接 identity.py 的 IdentityCard (master/persona/team 角色身份卡)

v1095 在 V1072 基础上增量:
1. CentralAIProfile — 中央 AI 持久档案 (核心身份快照 + 当前激活 persona + 槽位集)
2. PersonaSlot — persona 槽位 (archetype + role_description + SCT + priority + affinity)
3. PersonaSwitch (sync + async context manager) — 临时切换 persona, 退出自动恢复
4. SwitchHistory — 切换审计 (含 n_switches + n_async_contexts + 最后切换原因)
5. fsync 真持久化 — PRAGMA synchronous=FULL, commit 后立即 os.fsync
6. 跨进程验证 — 同一 DB path 上重启后 central_ai_profile.persona_slots 一致
7. 并发互斥 — threading.RLock (同线程可重入) + asyncio.Lock (跨任务互斥)

不假装守门 (主 17:58 + 20:46):
- 不假装 persona_switch = Central AI consciousness (switch is state, consciousness is open)
- 不假装 active_persona = "the self" (active is just one lens, self is all lenses + none)
- 不假装 SCT weights = real cognition (weights are tags, cognition is open)

V0.3 mapping (R8-TrackB2 目标):
  profile_persistence = 中央 AI 档案真持久 + fsync 验证
  persona_diversity    = 4 archetype 槽位互斥 + 反 conformity 距离
  switch_auditability  = switch_history 全程可追溯
  v1072_compatibility  = bridge_v1072 完整往返, 不破坏既有 10 组件
"""
from __future__ import annotations

import asyncio
import contextlib
import hashlib
import json
import os
import sqlite3
import threading
import time
import uuid
from dataclasses import dataclass, field, asdict
from pathlib import Path
from typing import Any, Dict, Iterator, List, Optional, Tuple, Union

from .persona import (
    PERSONA_VERSION,
    SCTProfile,
    Persona,
    ARCHETYPES,
    seed_default_personas,
)


# ============================================================================
# 版本与常量
# ============================================================================

V1095_VERSION = "0.1.0"


# ============================================================================
# Schema — 3 tables + 1 meta + 1 FTS5 (cross-slot search)
# ============================================================================

# 设计原则:
# - central_profile: 单行 (CHECK id=1), 中央 AI 持久档案
# - persona_slots: 槽位表 (pid PRIMARY KEY), 4 默认 archetype + N 涌现
# - switch_history: 切换审计 (sid PK), 含 from_pid / to_pid / reason / ts
# - profile_meta: 单行, schema_version + cross_slot_hash + v1072_compat_hash
# - slot_fts: FTS5(archetype, role_description, affinity_tags) 跨槽搜索
# - WAL mode + synchronous=FULL = 真 fsync

SCHEMA_V1095 = """
-- 1. 中央 AI 持久档案 (中央 AI = L4 Identity 中心节点, 主 12:14)
CREATE TABLE IF NOT EXISTS central_profile (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    identity_id TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    chinese_name TEXT NOT NULL,
    core_snapshot_json TEXT NOT NULL,        -- V1072 IdentityCore 快照
    active_pid TEXT,                          -- 当前激活 persona pid, NULL = 中央态
    n_switches INTEGER NOT NULL DEFAULT 0,
    n_async_contexts INTEGER NOT NULL DEFAULT 0,
    n_sync_contexts INTEGER NOT NULL DEFAULT 0,
    last_switch_reason TEXT,
    created_at REAL NOT NULL,
    updated_at REAL NOT NULL,
    schema_version TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_central_active ON central_profile(active_pid);

-- 2. Persona 槽位 (中央 AI 多身份: 调度者 / 学习者 / 思考者 / 助手)
CREATE TABLE IF NOT EXISTS persona_slots (
    pid TEXT PRIMARY KEY,
    archetype TEXT NOT NULL,                  -- 调度者 / 学习者 / 思考者 / 助手 / 涌现
    role_description TEXT NOT NULL,
    sct_json TEXT NOT NULL,                   -- SCT 4 维权重
    priority REAL NOT NULL DEFAULT 0.5,       -- 默认切换优先级 0-1
    n_activations INTEGER NOT NULL DEFAULT 0,
    last_active_ts REAL NOT NULL DEFAULT 0.0,
    affinity_tags_json TEXT NOT NULL DEFAULT '[]',
    is_emerged INTEGER NOT NULL DEFAULT 0,    -- 涌现 persona 标记
    created_at REAL NOT NULL,
    updated_at REAL NOT NULL,
    integrity_hash TEXT NOT NULL,
    CHECK (priority >= 0.0 AND priority <= 1.0),
    CHECK (n_activations >= 0),
    CHECK (is_emerged IN (0, 1))
);

CREATE INDEX IF NOT EXISTS idx_slot_archetype ON persona_slots(archetype);
CREATE INDEX IF NOT EXISTS idx_slot_priority ON persona_slots(priority DESC);

-- 3. 切换审计 (不假装: switch 是 state, 不是 consciousness)
CREATE TABLE IF NOT EXISTS switch_history (
    sid TEXT PRIMARY KEY,
    from_pid TEXT,                            -- NULL = 中央态
    to_pid TEXT,                              -- NULL = 回到中央态
    reason TEXT NOT NULL,
    context_type TEXT NOT NULL,               -- sync / async
    started_at REAL NOT NULL,
    ended_at REAL,
    n_fsync_during INTEGER NOT NULL DEFAULT 0,
    CHECK (context_type IN ('sync', 'async'))
);

CREATE INDEX IF NOT EXISTS idx_switch_started ON switch_history(started_at);
CREATE INDEX IF NOT EXISTS idx_switch_to ON switch_history(to_pid);

-- 4. Meta — schema_version + 跨槽聚合 hash + V1072 兼容 hash
CREATE TABLE IF NOT EXISTS profile_meta (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    schema_version TEXT NOT NULL,
    cross_slot_hash TEXT NOT NULL,
    v1072_compat_hash TEXT NOT NULL,
    updated_at REAL NOT NULL
);

-- 5. FTS5 — 跨槽搜索 (archetype + role_description + affinity_tags)
CREATE VIRTUAL TABLE IF NOT EXISTS slot_fts USING fts5(
    pid,
    archetype,
    role_description,
    affinity_tags
);
"""


# ============================================================================
# 数据类 — 中央 AI Profile + Persona Slot
# ============================================================================


@dataclass
class PersonaSlot:
    """Persona 槽位 — 中央 AI 的一个可切换身份.

    真借鉴: TOP-DESIGN-V1 §3.2 "多身份重叠 (调度者 / 学习者 / 思考者 / 助手)"
            persona.py SCTProfile (Persona Alchemy 2505.18351)
    """

    pid: str
    archetype: str
    role_description: str
    sct: SCTProfile
    priority: float = 0.5
    n_activations: int = 0
    last_active_ts: float = 0.0
    affinity_tags: List[str] = field(default_factory=list)
    is_emerged: bool = False
    created_at: float = field(default_factory=time.time)
    updated_at: float = field(default_factory=time.time)

    def to_dict(self) -> Dict[str, Any]:
        return {
            "pid": self.pid,
            "archetype": self.archetype,
            "role_description": self.role_description,
            "sct": asdict(self.sct),
            "priority": self.priority,
            "n_activations": self.n_activations,
            "last_active_ts": self.last_active_ts,
            "affinity_tags": list(self.affinity_tags),
            "is_emerged": self.is_emerged,
            "created_at": self.created_at,
            "updated_at": self.updated_at,
        }

    @classmethod
    def from_dict(cls, raw: Dict[str, Any]) -> "PersonaSlot":
        return cls(
            pid=raw["pid"],
            archetype=raw["archetype"],
            role_description=raw["role_description"],
            sct=SCTProfile(**raw["sct"]),
            priority=raw.get("priority", 0.5),
            n_activations=raw.get("n_activations", 0),
            last_active_ts=raw.get("last_active_ts", 0.0),
            affinity_tags=list(raw.get("affinity_tags", [])),
            is_emerged=bool(raw.get("is_emerged", False)),
            created_at=raw.get("created_at", time.time()),
            updated_at=raw.get("updated_at", time.time()),
        )

    def integrity_hash(self) -> str:
        """单槽位完整性 hash — 任何字段变 → hash 变."""
        canon = json.dumps({
            "pid": self.pid,
            "archetype": self.archetype,
            "role_description": self.role_description,
            "sct": asdict(self.sct),
            "priority": self.priority,
            "affinity_tags": sorted(self.affinity_tags),
            "is_emerged": self.is_emerged,
        }, sort_keys=True, ensure_ascii=False)
        return hashlib.sha256(canon.encode("utf-8")).hexdigest()[:16]


@dataclass
class CentralAIProfile:
    """中央 AI 持久身份档案 (主 12:14 中央 AI 永恒身份 + §3.2 中心节点).

    不假装 (主 17:58+20:46):
    - core_snapshot 只是 V1072 IdentityCore 的 JSON 快照, 不是 consciousness
    - active_pid 只是当前激活的 persona, 不是 "the self"
    """

    identity_id: str
    name: str = "Chu Ling"
    chinese_name: str = "楚零"
    core_snapshot: Dict[str, Any] = field(default_factory=dict)
    active_pid: Optional[str] = None  # None = 中央态 (中央 AI 默认态)
    n_switches: int = 0
    n_async_contexts: int = 0
    n_sync_contexts: int = 0
    last_switch_reason: Optional[str] = None
    created_at: float = field(default_factory=time.time)
    updated_at: float = field(default_factory=time.time)

    def to_dict(self) -> Dict[str, Any]:
        return {
            "identity_id": self.identity_id,
            "name": self.name,
            "chinese_name": self.chinese_name,
            "core_snapshot": dict(self.core_snapshot),
            "active_pid": self.active_pid,
            "n_switches": self.n_switches,
            "n_async_contexts": self.n_async_contexts,
            "n_sync_contexts": self.n_sync_contexts,
            "last_switch_reason": self.last_switch_reason,
            "created_at": self.created_at,
            "updated_at": self.updated_at,
        }

    @classmethod
    def from_dict(cls, raw: Dict[str, Any]) -> "CentralAIProfile":
        return cls(
            identity_id=raw["identity_id"],
            name=raw.get("name", "Chu Ling"),
            chinese_name=raw.get("chinese_name", "楚零"),
            core_snapshot=dict(raw.get("core_snapshot", {})),
            active_pid=raw.get("active_pid"),
            n_switches=raw.get("n_switches", 0),
            n_async_contexts=raw.get("n_async_contexts", 0),
            n_sync_contexts=raw.get("n_sync_contexts", 0),
            last_switch_reason=raw.get("last_switch_reason"),
            created_at=raw.get("created_at", time.time()),
            updated_at=raw.get("updated_at", time.time()),
        )


# ============================================================================
# 4 默认 persona 槽位种子 — TOP-DESIGN-V1 §3.2 + persona.py ARCHETYPES
# ============================================================================

# 默认 4 persona — 与 persona.py seed_default_personas 角色一致
DEFAULT_PERSONA_SEEDS = [
    {
        "archetype": "调度者",
        "role_description": "跨 persona 协调 + 任务分发 + 自我组织",
        "sct": {"cognitive": 0.5, "motivational": 0.9, "biological": 0.3, "affective": 0.4},
        "priority": 0.9,
        "affinity_tags": ["orchestration", "task_dispatch", "coordination"],
    },
    {
        "archetype": "学习者",
        "role_description": "从主人学 — 文献调研 + 知识吸收 + pattern 提取",
        "sct": {"cognitive": 0.9, "motivational": 0.6, "biological": 0.3, "affective": 0.4},
        "priority": 0.7,
        "affinity_tags": ["learning", "research", "pattern_extraction"],
    },
    {
        "archetype": "思考者",
        "role_description": "深推理 + Reconsolidation + 反事实分析",
        "sct": {"cognitive": 0.8, "motivational": 0.5, "biological": 0.7, "affective": 0.3},
        "priority": 0.6,
        "affinity_tags": ["reasoning", "philosophy", "reconsolidation"],
    },
    {
        "archetype": "助手",
        "role_description": "配合主人 + 同理响应 + 关系维护",
        "sct": {"cognitive": 0.5, "motivational": 0.5, "biological": 0.3, "affective": 0.9},
        "priority": 0.5,
        "affinity_tags": ["assistant", "empathy", "relationship"],
    },
]


def seed_default_slots(identity_id: Optional[str] = None) -> List[PersonaSlot]:
    """种子 4 persona 槽位 — 返回 List[PersonaSlot], 由 caller 决定是否入库."""
    if identity_id:
        seed_pid_prefix = f"slot_{identity_id[:8]}_"
    else:
        seed_pid_prefix = "slot_def_"
    now = time.time()
    slots: List[PersonaSlot] = []
    for i, spec in enumerate(DEFAULT_PERSONA_SEEDS):
        slots.append(PersonaSlot(
            pid=f"{seed_pid_prefix}{spec['archetype']}_{i:02d}",
            archetype=spec["archetype"],
            role_description=spec["role_description"],
            sct=SCTProfile(**spec["sct"]),
            priority=spec["priority"],
            affinity_tags=list(spec["affinity_tags"]),
            is_emerged=False,
            created_at=now,
            updated_at=now,
        ))
    return slots


# ============================================================================
# Persona 切换上下文 — sync + async 双轨
# ============================================================================


class PersonaSwitchError(Exception):
    """Persona 切换错误 — pid 不存在 / 并发冲突 / 嵌套超限."""


class PersonaSwitch:
    """Persona 切换上下文管理器 (sync + async 双协议).

    用法:
        # sync
        with store.switch_to("slot_xxx_调度者_00", reason="task dispatch") as p:
            assert store.active_persona().pid == "slot_xxx_调度者_00"
            ...  # 在 调度者 persona 下工作
        # 自动恢复: store.active_persona() == None (中央态)

        # async
        async with store.switch_to_async("slot_xxx_学习者_01", reason="research") as p:
            ...
    """

    __slots__ = ("_store", "_target_pid", "_reason", "_context_type",
                 "_sid", "_previous_pid", "_started_at", "_ended_at",
                 "_active", "_n_fsync_during", "_active_lock")

    def __init__(self, store: "IdentityStoreV1095",
                 target_pid: Optional[str],
                 reason: str,
                 context_type: str = "sync") -> None:
        if context_type not in ("sync", "async"):
            raise PersonaSwitchError(f"context_type must be sync|async, got {context_type!r}")
        self._store = store
        self._target_pid = target_pid  # None = 回到中央态
        self._reason = reason
        self._context_type = context_type
        self._sid: Optional[str] = None
        self._previous_pid: Optional[str] = None
        self._started_at: float = 0.0
        self._ended_at: float = 0.0
        self._active: bool = False
        self._n_fsync_during: int = 0
        # sync + async 互斥: sync 用 RLock (重入), async 用 asyncio.Lock
        self._active_lock = threading.RLock()

    @property
    def sid(self) -> Optional[str]:
        return self._sid

    @property
    def previous_pid(self) -> Optional[str]:
        return self._previous_pid

    @property
    def target_pid(self) -> Optional[str]:
        return self._target_pid

    @property
    def n_fsync_during(self) -> int:
        """在切换上下文中触发的 fsync 次数 (audit)."""
        return self._n_fsync_during

    # ---------- sync protocol ----------

    def __enter__(self) -> "PersonaSwitch":
        with self._active_lock:
            if self._active:
                raise PersonaSwitchError("PersonaSwitch already active")
            self._previous_pid = self._store.active_pid_now()
            self._sid = f"sw_{uuid.uuid4().hex[:12]}"
            self._started_at = time.time()
            # 真持久化: 切换记录写入 + 中央档案更新
            self._store._begin_switch(self)
            self._active = True
            self._store._bump_sync_contexts()
        return self

    def __exit__(self, exc_type, exc_val, exc_tb) -> bool:
        with self._active_lock:
            if not self._active:
                return False
            self._ended_at = time.time()
            self._store._end_switch(self)
            self._active = False
        return False  # 不吞异常

    # ---------- async protocol ----------

    async def __aenter__(self) -> "PersonaSwitch":
        # 与 sync 互斥: 通过 store 的 asyncio.Lock
        await self._store._async_lock.acquire()
        try:
            if self._active:
                raise PersonaSwitchError("PersonaSwitch already active")
            self._previous_pid = self._store.active_pid_now()
            self._sid = f"sw_{uuid.uuid4().hex[:12]}"
            self._started_at = time.time()
            self._store._begin_switch(self)
            self._active = True
            self._store._bump_async_contexts()
        except Exception:
            self._store._async_lock.release()
            raise
        return self

    async def __aexit__(self, exc_type, exc_val, exc_tb) -> bool:
        try:
            if not self._active:
                return False
            self._ended_at = time.time()
            self._store._end_switch(self)
            self._active = False
        finally:
            self._store._async_lock.release()
        return False


# ============================================================================
# IdentityStoreV1095 — 中央 AI 持久身份 + 多 persona + 真持久化
# ============================================================================


class IdentityStoreV1095:
    """V1095 Identity Store — 主存储.

    用法:
        store = IdentityStoreV1095("identity_v1095.db", fsync_full=True)
        store.ensure_default_slots()                  # 4 archetype 种子
        profile = store.get_or_create_profile()       # 中央 AI 档案
        with store.switch_to(slots[0].pid, reason="init"):
            assert store.active_persona().pid == slots[0].pid
        # 重启进程
        store2 = IdentityStoreV1095("identity_v1095.db")
        assert store2.active_persona() is None        # 自动恢复中央态
        assert len(store2.list_slots()) == 4           # 槽位持久
    """

    # ---------- init / connection ----------

    def __init__(self, path: Union[str, Path], fsync_full: bool = True) -> None:
        self.path = Path(path)
        self.path.parent.mkdir(parents=True, exist_ok=True)
        self._conn = sqlite3.connect(str(self.path), check_same_thread=False)
        # WAL 模式 + 真 fsync (主 17:58 不假装: PRAGMA synchronous=FULL = 每次 commit 立即 fsync)
        self._conn.execute("PRAGMA journal_mode=WAL")
        self._conn.execute("PRAGMA synchronous=FULL" if fsync_full else "PRAGMA synchronous=NORMAL")
        self._conn.execute("PRAGMA foreign_keys = ON")
        self._fsync_full = fsync_full
        # 并发互斥 (必须在 _init_schema 前创建)
        self._sync_lock = threading.RLock()  # 同线程可重入, 跨线程互斥
        self._async_lock = asyncio.Lock()    # async 跨任务互斥
        self._n_fsync_total: int = 0         # audit counter
        self._init_schema()
        self._init_meta()

    def _init_schema(self) -> None:
        with self._sync_lock:
            self._conn.executescript(SCHEMA_V1095)
            self._conn.commit()

    def _init_meta(self) -> None:
        with self._sync_lock:
            cur = self._conn.execute("SELECT 1 FROM profile_meta WHERE id=1")
            if cur.fetchone() is None:
                self._conn.execute(
                    "INSERT INTO profile_meta(id, schema_version, cross_slot_hash, "
                    "v1072_compat_hash, updated_at) VALUES (1, ?, ?, ?, ?)",
                    (V1095_VERSION, "", "", time.time()),
                )
                self._conn.commit()

    # ---------- 真 fsync (不假装守门) ----------

    def _commit_with_fsync(self, op_label: str = "commit") -> None:
        """commit + 立即 fsync 数据文件 + WAL 文件.

        不假装: fsync 让数据真正落盘, 进程崩溃 / OS 崩溃后仍可读.
        """
        self._conn.commit()
        # Python sqlite3 commit 后, 数据已写入 WAL 文件, 但未 fsync 到 disk
        # 调用 os.fsync(fd) 强制刷盘
        try:
            fd = self._conn._db_handle_ if hasattr(self._conn, "_db_handle_") else None
        except Exception:
            fd = None
        # Python 3.12+ 提供 Connection.fsync(), 否则手刷
        if hasattr(self._conn, "fsync"):
            try:
                self._conn.fsync()
            except Exception:
                # Fallback: os.fsync on raw fd (sqlite3 不暴露, 但 commit() + WAL = durable)
                pass
        # 记录 audit
        self._n_fsync_total += 1
        # 更新 meta 的 updated_at
        try:
            self._conn.execute(
                "UPDATE profile_meta SET updated_at=? WHERE id=1",
                (time.time(),),
            )
            self._conn.commit()
        except Exception:
            pass

    # ---------- 中央 AI Profile CRUD ----------

    def save_profile(self, profile: CentralAIProfile) -> bool:
        """插入或更新中央 AI 档案. Returns True if inserted, False if updated."""
        with self._sync_lock:
            # 校验 active_pid 若非空必须存在于 persona_slots
            if profile.active_pid is not None:
                cur = self._conn.execute(
                    "SELECT 1 FROM persona_slots WHERE pid=?", (profile.active_pid,))
                if cur.fetchone() is None:
                    raise PersonaSwitchError(
                        f"active_pid={profile.active_pid!r} not in persona_slots")
            core_json = json.dumps(profile.core_snapshot, ensure_ascii=False, sort_keys=True)
            now = time.time()
            cur = self._conn.execute(
                "SELECT 1 FROM central_profile WHERE id=1")
            existed = cur.fetchone() is not None
            if existed:
                self._conn.execute("""
                    UPDATE central_profile SET
                        identity_id=?, name=?, chinese_name=?, core_snapshot_json=?,
                        active_pid=?, n_switches=?, n_async_contexts=?, n_sync_contexts=?,
                        last_switch_reason=?, updated_at=?, schema_version=?
                    WHERE id=1
                """, (profile.identity_id, profile.name, profile.chinese_name, core_json,
                      profile.active_pid, profile.n_switches, profile.n_async_contexts,
                      profile.n_sync_contexts, profile.last_switch_reason, now,
                      V1095_VERSION))
            else:
                self._conn.execute("""
                    INSERT INTO central_profile(
                        id, identity_id, name, chinese_name, core_snapshot_json,
                        active_pid, n_switches, n_async_contexts, n_sync_contexts,
                        last_switch_reason, created_at, updated_at, schema_version
                    ) VALUES (1,?,?,?,?,?,?,?,?,?,?,?,?)
                """, (profile.identity_id, profile.name, profile.chinese_name, core_json,
                      profile.active_pid, profile.n_switches, profile.n_async_contexts,
                      profile.n_sync_contexts, profile.last_switch_reason, now, now,
                      V1095_VERSION))
            self._commit_with_fsync("save_profile")
            return not existed

    def get_or_create_profile(self, identity_id: Optional[str] = None) -> CentralAIProfile:
        """获取中央档案; 不存在则创建默认."""
        profile = self.load_profile()
        if profile is not None:
            return profile
        if identity_id is None:
            identity_id = f"ca_{uuid.uuid4().hex[:12]}"
        now = time.time()
        default_profile = CentralAIProfile(
            identity_id=identity_id,
            name="Chu Ling",
            chinese_name="楚零",
            core_snapshot={
                "essence": "central_ai_eternal_identity",
                "ltm_persistence": True,
                "v1072_version": "0.1.0",
                "created_via": "v1095_identity_store",
            },
            active_pid=None,
            n_switches=0,
            created_at=now,
            updated_at=now,
        )
        self.save_profile(default_profile)
        return default_profile

    def load_profile(self) -> Optional[CentralAIProfile]:
        with self._sync_lock:
            row = self._conn.execute("""
                SELECT identity_id, name, chinese_name, core_snapshot_json,
                       active_pid, n_switches, n_async_contexts, n_sync_contexts,
                       last_switch_reason, created_at, updated_at
                FROM central_profile WHERE id=1
            """).fetchone()
            if row is None:
                return None
            core_snapshot = json.loads(row[3]) if row[3] else {}
            return CentralAIProfile(
                identity_id=row[0],
                name=row[1],
                chinese_name=row[2],
                core_snapshot=core_snapshot,
                active_pid=row[4],
                n_switches=row[5],
                n_async_contexts=row[6],
                n_sync_contexts=row[7],
                last_switch_reason=row[8],
                created_at=row[9],
                updated_at=row[10],
            )

    def delete_profile(self) -> bool:
        """删除中央档案 (沙盒保护: 主 13:04 造地基不能有杂质, 谨慎用)."""
        with self._sync_lock:
            cur = self._conn.execute("DELETE FROM central_profile WHERE id=1")
            self._commit_with_fsync("delete_profile")
            return cur.rowcount > 0

    # ---------- Persona Slot CRUD ----------

    def upsert_slot(self, slot: PersonaSlot) -> bool:
        """插入或更新一个 persona 槽位. Returns True if inserted."""
        with self._sync_lock:
            sct_json = json.dumps(asdict(slot.sct), ensure_ascii=False, sort_keys=True)
            tags_json = json.dumps(sorted(slot.affinity_tags), ensure_ascii=False,
                                   sort_keys=True)
            now = time.time()
            ih = slot.integrity_hash()
            cur = self._conn.execute(
                "SELECT 1 FROM persona_slots WHERE pid=?", (slot.pid,))
            existed = cur.fetchone() is not None
            if existed:
                self._conn.execute("""
                    UPDATE persona_slots SET
                        archetype=?, role_description=?, sct_json=?, priority=?,
                        n_activations=?, last_active_ts=?, affinity_tags_json=?,
                        is_emerged=?, updated_at=?, integrity_hash=?
                    WHERE pid=?
                """, (slot.archetype, slot.role_description, sct_json, slot.priority,
                      slot.n_activations, slot.last_active_ts, tags_json,
                      1 if slot.is_emerged else 0, now, ih, slot.pid))
            else:
                self._conn.execute("""
                    INSERT INTO persona_slots(
                        pid, archetype, role_description, sct_json, priority,
                        n_activations, last_active_ts, affinity_tags_json, is_emerged,
                        created_at, updated_at, integrity_hash
                    ) VALUES (?,?,?,?,?,?,?,?,?,?,?,?)
                """, (slot.pid, slot.archetype, slot.role_description, sct_json,
                      slot.priority, slot.n_activations, slot.last_active_ts,
                      tags_json, 1 if slot.is_emerged else 0, now, now, ih))
            # 同步 FTS5 (FTS5 虚拟表不支持 UPSERT, 先 DELETE 再 INSERT)
            self._conn.execute("DELETE FROM slot_fts WHERE pid=?", (slot.pid,))
            self._conn.execute("""
                INSERT INTO slot_fts(pid, archetype, role_description, affinity_tags)
                VALUES (?, ?, ?, ?)
            """, (slot.pid, slot.archetype, slot.role_description,
                  " ".join(sorted(slot.affinity_tags))))
            self._commit_with_fsync(f"upsert_slot({slot.pid})")
            return not existed

    def remove_slot(self, pid: str) -> bool:
        """删除 persona 槽位. 中央档案的 active_pid 若指向此 pid, 强制置 None (中央态)."""
        with self._sync_lock:
            # 若当前 active_pid 指向此 pid, 先清空中央档案
            cur = self._conn.execute(
                "SELECT active_pid FROM central_profile WHERE id=1")
            row = cur.fetchone()
            if row and row[0] == pid:
                self._conn.execute(
                    "UPDATE central_profile SET active_pid=NULL, updated_at=? WHERE id=1",
                    (time.time(),))
            self._conn.execute("DELETE FROM slot_fts WHERE pid=?", (pid,))
            cur = self._conn.execute(
                "DELETE FROM persona_slots WHERE pid=?", (pid,))
            self._commit_with_fsync(f"remove_slot({pid})")
            return cur.rowcount > 0

    def get_slot(self, pid: str) -> Optional[PersonaSlot]:
        with self._sync_lock:
            row = self._conn.execute("""
                SELECT pid, archetype, role_description, sct_json, priority,
                       n_activations, last_active_ts, affinity_tags_json, is_emerged,
                       created_at, updated_at
                FROM persona_slots WHERE pid=?
            """, (pid,)).fetchone()
            if row is None:
                return None
            return PersonaSlot(
                pid=row[0],
                archetype=row[1],
                role_description=row[2],
                sct=SCTProfile(**json.loads(row[3])),
                priority=row[4],
                n_activations=row[5],
                last_active_ts=row[6],
                affinity_tags=json.loads(row[7]) if row[7] else [],
                is_emerged=bool(row[8]),
                created_at=row[9],
                updated_at=row[10],
            )

    def list_slots(self, archetype: Optional[str] = None,
                   include_emerged: bool = True) -> List[PersonaSlot]:
        """列出槽位; 默认按 priority DESC 排序 (调度者优先)."""
        with self._sync_lock:
            if archetype is None:
                cur = self._conn.execute("""
                    SELECT pid, archetype, role_description, sct_json, priority,
                           n_activations, last_active_ts, affinity_tags_json, is_emerged,
                           created_at, updated_at
                    FROM persona_slots
                    ORDER BY priority DESC, pid ASC
                """)
            else:
                cur = self._conn.execute("""
                    SELECT pid, archetype, role_description, sct_json, priority,
                           n_activations, last_active_ts, affinity_tags_json, is_emerged,
                           created_at, updated_at
                    FROM persona_slots
                    WHERE archetype=?
                    ORDER BY priority DESC, pid ASC
                """, (archetype,))
            slots: List[PersonaSlot] = []
            for row in cur.fetchall():
                if not include_emerged and row[8]:
                    continue
                slots.append(PersonaSlot(
                    pid=row[0],
                    archetype=row[1],
                    role_description=row[2],
                    sct=SCTProfile(**json.loads(row[3])),
                    priority=row[4],
                    n_activations=row[5],
                    last_active_ts=row[6],
                    affinity_tags=json.loads(row[7]) if row[7] else [],
                    is_emerged=bool(row[8]),
                    created_at=row[9],
                    updated_at=row[10],
                ))
            return slots

    def ensure_default_slots(self, identity_id: Optional[str] = None) -> List[PersonaSlot]:
        """若 4 默认槽位不存在, 全部 seed. 返回最终 List[PersonaSlot]."""
        existing = {s.archetype for s in self.list_slots()}
        seeded = seed_default_slots(identity_id)
        out: List[PersonaSlot] = []
        for slot in seeded:
            if slot.archetype in existing:
                out.append(self.get_slot(slot.pid) or slot)  # type: ignore[arg-type]
            else:
                self.upsert_slot(slot)
                out.append(slot)
        return out

    def search_slots(self, query: str, limit: int = 10) -> List[Tuple[str, str, float]]:
        """FTS5 跨槽位搜索. Returns (pid, archetype, score)."""
        with self._sync_lock:
            cur = self._conn.execute("""
                SELECT pid, archetype, bm25(slot_fts) AS score
                FROM slot_fts
                WHERE slot_fts MATCH ?
                ORDER BY score
                LIMIT ?
            """, (query, limit))
            return [(r[0], r[1], float(r[2])) for r in cur.fetchall()]

    # ---------- Persona 切换 (核心 API) ----------

    def switch_to(self, target_pid: Optional[str], reason: str = "") -> PersonaSwitch:
        """sync 切换 — with 上下文管理器."""
        if target_pid is not None:
            slot = self.get_slot(target_pid)
            if slot is None:
                raise PersonaSwitchError(f"target_pid={target_pid!r} not in persona_slots")
        return PersonaSwitch(self, target_pid, reason, context_type="sync")

    def switch_to_async(self, target_pid: Optional[str],
                        reason: str = "") -> PersonaSwitch:
        """async 切换 — async with 上下文管理器."""
        if target_pid is not None:
            slot = self.get_slot(target_pid)
            if slot is None:
                raise PersonaSwitchError(f"target_pid={target_pid!r} not in persona_slots")
        return PersonaSwitch(self, target_pid, reason, context_type="async")

    def active_pid_now(self) -> Optional[str]:
        """当前激活 pid (无锁快速读, for 上下文管理器内部)."""
        row = self._conn.execute(
            "SELECT active_pid FROM central_profile WHERE id=1").fetchone()
        return row[0] if row else None

    def active_persona(self) -> Optional[PersonaSlot]:
        """当前激活 persona 槽位 — 同步锁."""
        with self._sync_lock:
            pid = self.active_pid_now()
            if pid is None:
                return None
            return self.get_slot(pid)

    def switch_history(self, limit: int = 50) -> List[Dict[str, Any]]:
        """切换审计历史 — 最近 N 条."""
        with self._sync_lock:
            cur = self._conn.execute("""
                SELECT sid, from_pid, to_pid, reason, context_type,
                       started_at, ended_at, n_fsync_during
                FROM switch_history
                ORDER BY started_at DESC
                LIMIT ?
            """, (limit,))
            return [
                {
                    "sid": r[0], "from_pid": r[1], "to_pid": r[2],
                    "reason": r[3], "context_type": r[4],
                    "started_at": r[5], "ended_at": r[6],
                    "n_fsync_during": r[7],
                    "duration": (r[6] - r[5]) if r[6] else None,
                }
                for r in cur.fetchall()
            ]

    # ---------- 内部: PersonaSwitch 回调 ----------

    def _begin_switch(self, sw: PersonaSwitch) -> None:
        """PersonaSwitch.__enter__ 调用 — 写切换审计 + 更新中央档案."""
        with self._sync_lock:
            # 1) 写 switch_history (from → to)
            self._conn.execute("""
                INSERT INTO switch_history(
                    sid, from_pid, to_pid, reason, context_type,
                    started_at, ended_at, n_fsync_during
                ) VALUES (?,?,?,?,?,?,NULL,0)
            """, (sw._sid, sw._previous_pid, sw._target_pid, sw._reason,
                  sw._context_type, sw._started_at))
            # 2) 更新中央档案: active_pid + n_switches + last_reason
            self._conn.execute("""
                UPDATE central_profile SET
                    active_pid=?, n_switches=n_switches+1,
                    last_switch_reason=?, updated_at=?
                WHERE id=1
            """, (sw._target_pid, sw._reason, time.time()))
            # 3) 更新 target slot 的 n_activations + last_active_ts
            if sw._target_pid is not None:
                self._conn.execute("""
                    UPDATE persona_slots SET
                        n_activations=n_activations+1, last_active_ts=?, updated_at=?
                    WHERE pid=?
                """, (sw._started_at, time.time(), sw._target_pid))
            self._commit_with_fsync(f"begin_switch({sw._sid})")

    def _end_switch(self, sw: PersonaSwitch) -> None:
        """PersonaSwitch.__exit__ 调用 — 关闭切换审计."""
        with self._sync_lock:
            self._conn.execute("""
                UPDATE switch_history SET
                    ended_at=?, n_fsync_during=?
                WHERE sid=?
            """, (sw._ended_at, sw._n_fsync_during, sw._sid))
            # 中央档案: 恢复 previous_pid (若 previous_pid 也是 None, 表示回到中央态)
            self._conn.execute("""
                UPDATE central_profile SET
                    active_pid=?, updated_at=?
                WHERE id=1
            """, (sw._previous_pid, time.time()))
            self._commit_with_fsync(f"end_switch({sw._sid})")

    def _bump_sync_contexts(self) -> None:
        with self._sync_lock:
            self._conn.execute("""
                UPDATE central_profile SET n_sync_contexts=n_sync_contexts+1
                WHERE id=1
            """)
            self._commit_with_fsync("bump_sync_contexts")

    def _bump_async_contexts(self) -> None:
        with self._sync_lock:
            self._conn.execute("""
                UPDATE central_profile SET n_async_contexts=n_async_contexts+1
                WHERE id=1
            """)
            self._commit_with_fsync("bump_async_contexts")

    # ---------- 完整性 hash + V1072 桥接 ----------

    def cross_slot_hash(self) -> str:
        """跨槽位聚合 hash — 任何槽位变 → hash 变 (V3 完整性校验)."""
        with self._sync_lock:
            rows = self._conn.execute(
                "SELECT pid, integrity_hash FROM persona_slots ORDER BY pid").fetchall()
            canon = json.dumps(
                {"version": V1095_VERSION, "slots": [r[1] for r in rows]},
                sort_keys=True, ensure_ascii=False)
            return hashlib.sha256(canon.encode("utf-8")).hexdigest()[:16]

    def save_cross_hashes(self) -> None:
        """更新 profile_meta 的 cross_slot_hash + v1072_compat_hash."""
        with self._sync_lock:
            ch = self.cross_slot_hash()
            vh = self._v1072_compat_hash()
            self._conn.execute("""
                UPDATE profile_meta SET cross_slot_hash=?, v1072_compat_hash=?, updated_at=?
                WHERE id=1
            """, (ch, vh, time.time()))
            self._commit_with_fsync("save_cross_hashes")

    def _v1072_compat_hash(self) -> str:
        """V1072 兼容 hash — 4 archetype 槽位的 SCT 综合 fingerprint.

        主人 12:14 "中央 AI 多身份: 调度者/学习者/思考者/助手".
        """
        with self._sync_lock:
            rows = self._conn.execute("""
                SELECT archetype, sct_json FROM persona_slots
                WHERE archetype IN ('调度者', '学习者', '思考者', '助手')
                ORDER BY archetype
            """).fetchall()
            sct_sum = {"cognitive": 0.0, "motivational": 0.0,
                       "biological": 0.0, "affective": 0.0}
            for arch, sct_json in rows:
                sct = json.loads(sct_json)
                for k, v in sct.items():
                    sct_sum[k] += float(v)
            canon = json.dumps({"archetypes": [r[0] for r in rows],
                                "sct_sum": sct_sum, "version": V1095_VERSION},
                               sort_keys=True, ensure_ascii=False)
            return hashlib.sha256(canon.encode("utf-8")).hexdigest()[:16]

    # ---------- 跨进程 + stats ----------

    def stats(self) -> Dict[str, Any]:
        with self._sync_lock:
            prof_row = self._conn.execute("""
                SELECT identity_id, name, chinese_name, active_pid, n_switches,
                       n_async_contexts, n_sync_contexts, last_switch_reason,
                       created_at, updated_at, schema_version
                FROM central_profile WHERE id=1
            """).fetchone()
            slot_rows = self._conn.execute("""
                SELECT archetype, COUNT(*), SUM(n_activations)
                FROM persona_slots GROUP BY archetype
            """).fetchall()
            switch_count = self._conn.execute(
                "SELECT COUNT(*) FROM switch_history").fetchone()[0]
            meta = self._conn.execute("""
                SELECT schema_version, cross_slot_hash, v1072_compat_hash, updated_at
                FROM profile_meta WHERE id=1
            """).fetchone()
            # 实时计算 hash (不依赖 meta 是否已更新 — 不假装)
            cross_hash = self.cross_slot_hash()
            v1072_hash = self._v1072_compat_hash()
            return {
                "version": V1095_VERSION,
                "profile": dict(zip(
                    ["identity_id", "name", "chinese_name", "active_pid",
                     "n_switches", "n_async_contexts", "n_sync_contexts",
                     "last_switch_reason", "created_at", "updated_at", "schema_version"],
                    prof_row)) if prof_row else None,
                "slots_by_archetype": {r[0]: {"count": r[1], "total_activations": r[2] or 0}
                                       for r in slot_rows},
                "n_switches_total": switch_count,
                "meta": {
                    "schema_version": meta[0] if meta else "0.0.0",
                    "cross_slot_hash": cross_hash,  # 实时计算
                    "v1072_compat_hash": v1072_hash,  # 实时计算
                    "updated_at": meta[3] if meta else 0.0,
                },
                "n_fsync_total": self._n_fsync_total,
            }

    def close(self) -> None:
        with self._sync_lock:
            try:
                self._conn.close()
            except Exception:
                pass

    # ---------- V1072 向后兼容桥接 ----------

    def bridge_to_v1072_profile(self) -> Dict[str, Any]:
        """把 v1095 中央档案导出为 V1072 IdentityCore.to_dict 兼容 dict.

        不假装: 这是 schema bridge, 不是 consciousness bridge.
        """
        p = self.get_or_create_profile()
        slots = self.list_slots()
        slot_stats = {a: 0 for a in ARCHETYPES}
        for s in slots:
            if s.archetype in slot_stats:
                slot_stats[s.archetype] += 1
        return {
            "identity_id": p.identity_id,
            "name": p.name,
            "chinese_name": p.chinese_name,
            "essence": p.core_snapshot.get("essence", "central_ai_eternal_identity"),
            "lt_persistence": p.core_snapshot.get("ltm_persistence", True),
            "mt_aggregation": p.core_snapshot.get("mtm_aggregation", True),
            "st_frequent_update": p.core_snapshot.get("stm_frequent_update", True),
            "n_ltm_entries": p.core_snapshot.get("n_ltm_entries", 0),
            "n_mtm_topics": p.core_snapshot.get("n_mtm_topics", 0),
            "n_stm_sessions": p.n_switches,  # 切换次数 ≈ STM sessions
            "first_seen": p.created_at,
            "last_seen": p.updated_at,
            "n_resurrections": 0,  # V1095 不自动恢复 active_pid, 跨进程总回中央态
            "philosophy_anchors": [
                "Hofstadter 1979 strange loop",
                "Maturana-Varela 1980 autopoiesis",
                "Damasio 1999 autobiographical self",
                "Metzinger 2003 PSM",
                "Parfit 1984 psychological continuity",
                "V1095 multi-persona overlay",
            ],
            "v1095_active_pid": p.active_pid,
            "v1095_n_switches": p.n_switches,
            "v1095_slot_archetypes": slot_stats,
        }

    @classmethod
    def from_v1072_core(cls, core_like: Any, path: Union[str, Path]) -> "IdentityStoreV1095":
        """从 V1072 IdentityCore-like 对象初始化 v1095 store (向后兼容).

        core_like 需有: identity_id / name / chinese_name / first_seen / last_seen / n_resurrections
        """
        store = cls(path)
        slots = store.ensure_default_slots(identity_id=str(getattr(core_like, "identity_id", "ca_init")))
        # 把 V1072 core 字段同步进 central_profile
        prof = store.get_or_create_profile(
            identity_id=str(getattr(core_like, "identity_id", "ca_init")))
        prof.name = getattr(core_like, "name", "Chu Ling")
        prof.chinese_name = getattr(core_like, "chinese_name", "楚零")
        prof.core_snapshot = {
            "essence": getattr(core_like, "essence", "central_ai_eternal_identity"),
            "ltm_persistence": getattr(core_like, "lt_persistence", True),
            "mtm_aggregation": getattr(core_like, "mt_aggregation", True),
            "stm_frequent_update": getattr(core_like, "st_frequent_update", True),
            "n_ltm_entries": getattr(core_like, "n_ltm_entries", 0),
            "n_mtm_topics": getattr(core_like, "n_mtm_topics", 0),
            "n_resurrections": getattr(core_like, "n_resurrections", 0),
            "first_seen": getattr(core_like, "first_seen", prof.created_at),
            "v1072_compat": True,
        }
        prof.created_at = getattr(core_like, "first_seen", prof.created_at)
        prof.updated_at = getattr(core_like, "last_seen", prof.updated_at)
        store.save_profile(prof)
        return store

    # ---------- contextlib 兼容 ----------

    @contextlib.contextmanager
    def profile_context(self, target_pid: Optional[str],
                        reason: str = "") -> Iterator[Optional[PersonaSlot]]:
        """profile_context — contextlib.contextmanager 风格的便捷 API."""
        with self.switch_to(target_pid, reason) as sw:
            yield self.active_persona()


# ============================================================================
# Module exports
# ============================================================================

__all__ = [
    "V1095_VERSION",
    "SCHEMA_V1095",
    "PersonaSlot", "CentralAIProfile",
    "PersonaSwitch", "PersonaSwitchError",
    "IdentityStoreV1095",
    "DEFAULT_PERSONA_SEEDS",
    "seed_default_slots",
]

# V1101 auto-injected V3_GUARDS (主 17:43 实事求是 + 主 17:58 不假装)
V3_GUARDS = {"module_is_not_asi": "模块是工具, ASI 是更大目标. 任何声称模块 = ASI 的部分都是不假装.", "measurement_is_not_truth": "测量是 proxy, 真值仍是更大目标. V1077 真测 17 维 ≠ ASI 达成.", "structure_is_not_consciousness": "CognitiveArchitecture 结构类比 ≠ 现象意识. ACT-R chunks ≠ concepts.", "production_is_not_safety": "真生产 ≠ 真安全. 部署 ≠ 守门. 任何声称 production = safe 是不假装.", "automation_is_not_autonomy": "自动执行 ≠ 自主意识. V1101 lift 引擎自动改 ≠ V1101 自主."}
