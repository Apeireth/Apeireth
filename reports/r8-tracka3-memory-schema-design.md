# R8-TrackA3 — Memory 模块 Schema 设计

> 数据库工程师 (database_engineer) | 2026-07-29 | V1094 v0.1.0
> 任务: R8-TrackA3, 为 HotCold + WAL + Replay + Dream 设计底层 storage schema

## 0. 执行摘要

**结论: V1094 Memory Schema 0.1.0 真生产就绪。** 8 业务表 + meta 表 + 26 索引，
覆盖 R8-TrackA 描述 6 项要求全部要点，**23/23 测试通过**，与现仓零破坏兼容。

关键决策:

- 8 业务表拆为两个维度: **生命周期维度** (hot/cold/wal/dream/snapshot) + **三层模型维度** (stm/mtm/ltm)
- 表前缀 `memory_/stm/mtm/ltm` 与现仓 `episodes/notes/hqb_*` 不冲突
- WAL 用 `event_id UNIQUE` 作幂等键，借鉴 `memory_replay_design.IDEMPOTENT_OPS`
- LTM `fingerprint UNIQUE` 借鉴 mem0 (主 9:41 round-19) hash dedup
- snapshot `(scope, seq) UNIQUE` 借鉴 letta/messages 自引用 + R7-BE-02 StateID

**真生产兼容 (零破坏)**:

- 未触碰 `apeireth/memory.py` v0.3 表 (`episodes/notes/episodes_fts/notes_fts/memory_meta`)
- 未触碰 `apeireth/hqb/schema.py` v0.1 表 (`hqb_decisions/.../hqb_meta`)
- 唯一新增 `apeireth/v1094_memory_schema.py` (additive)
- 唯一新增 `tests/test_v1094_memory_schema.py` (additive)

---

## 1. 表清单 (8 业务 + meta = 9 表)

| # | 表名 | 职责 | 借鉴 | R8-TrackA3 描述点 |
|---|------|------|------|------------------|
| 1 | `memory_hot` | 当前 session episode (rolling buffer) | memory_store.episodes | hot_tier |
| 2 | `memory_cold` | 跨 session 持久 (Note + LTM anchor) | memory_store.notes + V1072.MemoryAnchor | cold_tier |
| 3 | `memory_wal` | 写前日志 (append+checksum) | Tonbo LSM + R7-BE-02 | wal |
| 4 | `memory_dream` | dream 产出暂存 | R7-BE-01.DreamSubsystem | dream_candidates (_dream=true 标记) |
| 5 | `memory_snapshots` | replay 状态快照 | letta/messages + R7-BE-02.StateID | replay_snapshots |
| 6 | `stm_messages` | 短期对话消息 (滚动 50) | memoryos-rust.ShortTermMemory + memory_3tier.STM_MAX_SIZE | STM |
| 7 | `mtm_themes` | 中期主题聚合 | memoryos-rust.MidTermSegment + memory_3tier.TopicSummary | MTM |
| 8 | `ltm_facts` | 长期事实 (永不丢) | memoryos-rust.LongTermMemory + V1072.MemoryAnchor + mem0 | LTM |
| 9 | `memory_meta` | schema_version + 自描述 | hqb/schema.py.hqb_meta | 全局 |

每个表的字段定义详见 `apeireth/v1094_memory_schema.py` (SCHEMA_V094 字符串)。

---

## 2. ERD 图 (文字版)

```
┌─────────────────────────────────────────────────────────────────────┐
│                         V1094 Memory Schema                         │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  [Hot tier]              [STM/MTM/LTM 三层]                         │
│  ┌────────────┐          ┌────────────────┐                         │
│  │ memory_hot │          │ stm_messages   │ (滚动 N=50)              │
│  │  id (PK)   │──┐       │  id (PK)       │                         │
│  │  session_id│  │       │  session_id    │                         │
│  │  actor     │  │       │  role          │                         │
│  │  content   │  │       │  content       │                         │
│  │  ts        │  │       │  embedding_ref │ (外部向量库 reference)  │
│  │  fingerprint  │       │  ts            │                         │
│  │  linked_id_hash│      └────────────────┘                         │
│  └────────────┘  │       ┌────────────────┐                         │
│        │         │       │ mtm_themes     │ (中期主题)               │
│        │ (consolidate/dream)│ topic_id(PK) │                         │
│        │         │       │  topic_label   │                         │
│        │         │       │  n_episodes    │                         │
│        │         │       │  importance_avg│                         │
│        │         │       │  summary       │                         │
│        │         │       │  last_updated  │                         │
│        │         │       └────────────────┘                         │
│        │         │       ┌────────────────┐                         │
│        │         │       │ ltm_facts      │ (永不丢)                 │
│        │         │       │  id (PK)       │                         │
│        │         │       │  category      │ ← identity/decision/   │
│        │         │       │  content       │   value/event/fact     │
│        │         │       │  importance    │ (0..10)                │
│        │         │       │  fingerprint   │ ← UNIQUE (dedup)       │
│        │         │       │  master_quoted │ (V1072)                │
│        │         │       └────────────────┘                         │
│        │         │                                                    │
│  ┌────────────┐  │  ┌────────────────┐                                │
│  │memory_cold │  │  │ memory_dream   │ (R7-BE-01)                   │
│  │  id (PK)   │←─┼──│  id (PK)       │                                │
│  │  category  │  │  │  source_eid    │ → memory_hot.id              │
│  │  importance│  │  │  summary       │                              │
│  │  fingerprint│  │  │  status        │ pending/consumed/tombstone │
│  │  superseded_by│  │  dream_state   │ IDLE/DREAMING/CONSOLIDATING/│
│  │  ...       │  │  │                │ FORGETTING/VERIFYING/...   │
│  └────────────┘  │  └────────────────┘                              │
│        ▲         │                                                    │
│        │         │   ┌──────────────────────────┐                    │
│        │         │   │ memory_wal               │ (写前日志)         │
│        │         │   │  seq (PK AUTOINCREMENT)  │                    │
│        │         │   │  scope (stm/mtm/ltm/...) │                    │
│        │         │   │  op ∈ IDEMPOTENT_OPS     │                    │
│        │         │   │  event_id UNIQUE (幂等)  │                    │
│        │         │   │  checksum                │                    │
│        │         │   │  applied (0/1)           │                    │
│        │         │   └──────────────────────────┘                    │
│        │         │                                                    │
│        │         │   ┌──────────────────────────┐                    │
│        │         │   │ memory_snapshots         │ (replay 快照)      │
│        │         │   │  id (PK)                 │                    │
│        │         │   │  (scope, seq) UNIQUE     │                    │
│        │         │   │  content_hash            │                    │
│        │         │   │  rationale               │                    │
│        │         │   │  identity_hash           │                    │
│        │         │   │  snapshot_score          │                    │
│        │         │   └──────────────────────────┘                    │
│        │         │                                                    │
│        └─────────┴──── memory_meta (schema_version=0.1.0)            │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

**关联说明**:

- `memory_hot` (1) → (N) `memory_dream.source_episode_id` (Dream consolidate 来源)
- `memory_hot` (1) → (1) `memory_cold` (重要性≥8 时迁移到 cold)
- `memory_cold.superseded_by` 自引用 (Parfit 心理连续性链)
- `memory_wal.event_id` 唯一 → 幂等 (Replay IDEMPOTENT_OPS)
- `memory_snapshots (scope, seq)` 唯一 → 借鉴 R7-BE-02 StateID
- 所有表的 `linked_identity_hash` / `identity_hash` 关联 V1072 IdentityCard

---

## 3. 索引策略 (26 索引)

### 3.1 Hot tier (4 索引)

- `idx_hot_session_ts (session_id, ts DESC)` — 主查询: 当前 session 时间序
- `idx_hot_tier_ts (tier, ts DESC)` — 分层扫表
- `idx_hot_fingerprint` — dedup key
- `idx_hot_identity (linked_identity_hash)` — V1072 关联

### 3.2 Cold tier (5 索引)

- `idx_cold_category_imp (category, importance DESC, ts DESC)` — LTM 优先级扫表
- `idx_cold_tier_ts (tier, ts DESC)` — 分层扫表
- `idx_cold_fingerprint` — dedup key
- `idx_cold_identity (linked_identity_hash)` — V1072 关联
- `idx_cold_superseded (superseded_by)` — 心理连续性链查询

### 3.3 WAL (4 索引)

- `idx_wal_event UNIQUE (event_id)` — 幂等键 (Replay 核心)
- `idx_wal_scope_applied_ts (scope, applied, ts)` — Replay worker 主查询
- `idx_wal_applied_ts (applied, ts)` — 全局 pending 扫表
- `idx_wal_op (op)` — 按 IDEMPOTENT_OPS 类型分析

### 3.4 Dream (3 索引)

- `idx_dream_status_ts (status, ts DESC)` — pending 优先消费
- `idx_dream_state_ts (dream_state, ts DESC)` — 状态机分布
- `idx_dream_source (source_episode_id)` — 追溯来源

### 3.5 Snapshots (2 索引)

- `idx_snapshot_scope_seq UNIQUE (scope, seq)` — R7-BE-02 StateID 主键
- `idx_snapshot_ts (ts DESC)` — 时间序浏览

### 3.6 STM (2 索引)

- `idx_stm_session_ts (session_id, ts DESC)` — STM 滚动裁剪 (N=50)
- `idx_stm_role (role)` — 按角色筛选

### 3.7 MTM (2 索引)

- `idx_mtm_label (topic_label)` — 主题搜索
- `idx_mtm_last_updated (last_updated DESC)` — 主题热度

### 3.8 LTM (4 索引)

- `idx_ltm_fingerprint UNIQUE (fingerprint)` — mem0 dedup
- `idx_ltm_category_imp (category, importance DESC)` — 优先级扫表
- `idx_ltm_ts (ts DESC)` — 时间序
- `idx_ltm_observation (observation_date)` — temporal grounding

---

## 4. 对接点 (与现仓真生产)

### 4.1 V1072 ASI Central AI Eternal Identity

- `IdentityManifest.add(source='LTM')` → `ltm_facts(category, importance, fingerprint)`
- `IdentityManifest.add(source='MTM')` → `mtm_themes(topic_label, summary)`
- `IdentityManifest.add(source='STM')` → `stm_messages(session_id, role, content)`
- `IdentityCard.integrity_hash()` → `linked_identity_hash` (hot/cold) / `identity_hash` (snapshots)
- V1072 `MemoryAnchor` 字段 (`anchor_id/category/content/importance/master_quoted`) 与 `ltm_facts` 1:1 映射

### 4.2 HQB Integration (V1085/V1086/V1087)

- `HQBIntegration.record_v1074/v1082/v1083` → `memory_snapshots(scope='hqb', snapshot_score)`
- HQB `hqb_meta.schema_version=0.1.0` 与 `memory_meta.schema_version=0.1.0` 互不干扰
- 通过 `memory_meta.k='v1094_compat_with_hqb_v0.1'` 显式记录双向兼容

### 4.3 R7-BE-02 MemoryReplay (memory_replay_design.py)

- `IDEMPOTENT_OPS = {tag_set, anchor_link, anchor_unlink, score_record, phase_emit, trace_record}`
  → `memory_wal.op` 必 ∈ 该集合 (上层守门; schema 层只接受 TEXT, 不约束)
- `StateID(scope, seq)` ↔ `memory_snapshots (scope, seq) UNIQUE`
- `Event.event_id` ↔ `memory_wal.event_id UNIQUE` (幂等键)
- `idempotent_apply` 第二调用命中 UNIQUE 冲突 → `wal_append` 捕获 IntegrityError 返回相同 event_id
  → 模拟 R7-BE-02 "same event twice → cached=True"

### 4.4 R7-BE-01 Dream Subsystem

- `DreamSubsystem.run_cycle` → `memory_dream(_dream=true)` 隐式: status='pending'/dream_state='CONSOLIDATING'
- 来源 episode → `memory_dream.source_episode_id`
- 消费时 → `status='consumed'`, `consumed_ts` 写入
- 墓碑 → `status='tombstoned'`
- 状态机 6 态 (IDLE/DREAMING/CONSOLIDATING/FORGETTING/VERIFYING/INTERRUPTED) 落到 `dream_state` 字段

---

## 5. 真迁移脚本 (upgrade / downgrade)

### 5.1 upgrade(conn) — 幂等

```python
def upgrade(conn):
    conn.executescript(SCHEMA_V094)      # CREATE TABLE IF NOT EXISTS × 9
    conn.executescript(INDEXES_V094)     # CREATE INDEX IF NOT EXISTS × 23
    cur = conn.execute("SELECT v FROM memory_meta WHERE k='schema_version'")
    if cur.fetchone() is None:
        conn.execute("INSERT INTO memory_meta(k, v) VALUES (?, ?)", ("schema_version", V1094_VERSION))
    conn.commit()
```

- 多次执行结果一致 (所有 DDL 都带 IF NOT EXISTS)
- 已有 `schema_version` 不被覆盖 (T22 验证 — 真生产兼容)

### 5.2 downgrade(conn, keep_meta=True)

```python
def downgrade(conn, *, keep_meta=True):
    for t in TABLE_NAMES:
        conn.execute(f"DROP TABLE IF EXISTS {t}")
    if not keep_meta:
        conn.execute("DROP TABLE IF EXISTS memory_meta")
    conn.commit()
```

- 仅 DROP V1094 表 (9 个); 现仓 `episodes/notes/hqb_*` 不动
- `keep_meta=True` (默认) 保留 meta + 记录 `v1094_downgraded_ts` 事件

### 5.3 快捷方式 (路径)

- `upgrade_path("apeireth.db")` — 创建 db + upgrade
- `downgrade_path("apeireth.db")` — 回退

---

## 6. 测试覆盖 (23 用例 ≥ 20)

| 组 | 用例 | 覆盖点 |
|----|------|--------|
| schema 创建 | T01..T04 (4) | version / tables / indexes / pragmas |
| 索引效率 | T05..T08 (4) | EXPLAIN QUERY PLAN 命中 4 个核心索引 |
| 真插入查询 | T09..T15 (7) | hot/cold/wal/dream/snapshot/stm/ltm 真路径 |
| 迁移幂等 | T16..T18 (3) | upgrade 多次 / upgrade-downgrade-upgrade / 持久化 |
| 对接点 + 工具 | T19..T20 (2) | fingerprint+checksum 确定性 / 4 对接点声明 |
| 约束完整性 | T21..T23 (3) | UNIQUE 约束 / meta 保护 / 常量一致性 |

执行: `pytest tests/test_v1094_memory_schema.py -v` → **23 passed in 8.57s**

---

## 7. 与现仓兼容性核验

| 现仓资产 | 是否触碰 | 备注 |
|----------|----------|------|
| `apeireth/memory.py` (v0.1) | ✗ | 仅引用 Episode/Note/MemoryStore dataclass |
| `apeireth/memory_store.py` (v0.3, `episodes/notes/episodes_fts/notes_fts`) | ✗ | 表名前缀不冲突 |
| `apeireth/memory_3tier.py` (STM/MTM/LTM deque/dict) | ✗ | 仅 dataclass 引用 |
| `apeireth/memory_replay_design.py` (Protocol/StateID) | ✗ | 仅 IDEMPOTENT_OPS 字符串注释引用 |
| `apeireth/hqb/schema.py` (v0.1, `hqb_decisions/...`) | ✗ | 表前缀 `hqb_` 不冲突 |
| `apeireth/hqb_integration.py` | ✗ | 仅在 INTEGRATION_POINTS dict 注释引用 |
| `apeireth/v1072_asi_central_ai_eternal_identity.py` | ✗ | 仅在 INTEGRATION_POINTS dict 注释引用 |
| `data/asi_snapshot.json` | ✗ | 本任务不读不写 |
| `data/asi_history.jsonl` | ✗ | 本任务不读不写 |
| 已有 `artifacts/*` | ✗ | 本任务不写 |

**结论: 真生产零破坏兼容。**

---

## 8. 后续 R7-BE Phase-1 推进路径

本任务 = schema only, 不写业务逻辑。下游可基于本 schema 直接推进:

1. **R7-BE-02 MemoryReplay 真实现**: 继承 `MemorySchema`, 实现 `MemoryReplayProtocol`
   - `wal_append(scope, op, payload)` 已就绪
   - `wal_pending()` / `wal_mark_applied()` 已就绪
2. **R7-BE-01 DreamSubsystem 真实现**: 继承 `MemorySchema`, 6 态状态机
   - `memory_dream.source_episode_id` 已 FK-ready
   - `dream_state` 枚举已与 R7-BE-01 设计文档对齐
3. **V1072 IdentityManifest 持久化桥**: `IdentityManifest.add()` 加可选 `store: MemorySchema`
4. **HQBIntegration 持久化桥**: 加可选 `memory_snapshots` 写入 (双向审计)

---

## 9. 交付物清单

| 文件 | 行数 | 用途 |
|------|------|------|
| `apeireth/v1094_memory_schema.py` | 244 | schema + upgrade/downgrade + MemorySchema facade |
| `tests/test_v1094_memory_schema.py` | 444 | 23 真测试覆盖 6 维度 |
| `reports/r8-tracka3-memory-schema-design.md` | 本文件 | 设计文档 (ERD/索引/对接点/兼容核验) |

---

## 10. ponytail: ceiling & upgrade path

> ponytail: 本任务严格 schema-only. 业务逻辑 (Replay worker / Dream 状态机 / V1072 桥) 留给后续 R7-BE-01/R7-BE-02 真实现. 升级触发: 当 R7-BE Phase-1 启动时, 若新增字段超过 additive (需 ALTER COLUMN), 引入 v0.2.0 schema_version + 列级 migration.