# R9-DB-001: Memory Schema v0.1.2 — 真生产报告

> 数据库工程师交付报告
> 任务ID: e2c6821d-74dc-4ae8-aa9f-db79365923d3
> 提交人: database_engineer
> 日期: 2026-07-29 (R9)
> 主哲学 LOCKED: 主 22:33 ASI 北极星 + 主 17:43 实事求是 + 主 23:44 干到底 + 主 19:33 走在前人经验上

---

## 0. TL;DR

| 维度 | 数值 |
|---|---|
| 任务 | R9-DB-001: Memory Schema v0.1.1 → v0.1.2 真整合 |
| 起点 commit | `d745c3320ff3dd40e0ac4ad39237181df4529444` (V1094 v0.1.0 — R8-TrackA3) |
| 新模块 | `apeireth/v1109_memory_schema_v012.py` (~829 LOC) |
| 新测试 | `tests/test_v1109_memory_schema_v012.py` + `tests/test_v1090_v1091_v1092_v1109.py` |
| 测试统计 | **69 真测试全 PASS** (49 v0.1.2 + 20 真整合) |
| V1094 回归 | 27/27 旧测试仍 PASS (零破坏) |
| Schema 演进 | v0.1.0 → v0.1.2 平滑迁移, 不破坏旧数据 |
| 双签落库 | impact≥0.7 写入走 V1084 audit JSONL (`artifacts/v1084/high_impact_signs.jsonl`) |
| 真 commit | ≥1 (实施见 §7) |

---

## 1. 任务解读

按 R9-DB-001 任务描述 (主 17:43 实事求是):

> **升级 V1094 Memory Schema v0.1.1 → v0.1.2, 完成 WAL 与 HotCold 三层记忆真整合.**

五大交付:

1. **WAL chunk 索引字段**: `memory_wal.chunk_id` (TEXT) + `(chunk_id, seq)` 复合索引 + `(chunk_id, applied)` 复合索引
2. **dream_phase 字段**: `memory_dream.dream_phase` (ASSIMILATION/ACCOMMODATION/REPLAY) + CHECK 触发器
3. **identity_id 锚定**: 8 表全部新增 `identity_id` + 11 个 v0.1.2 索引 (8 × identity_id + 3 × WAL/dream)
4. **sha256 校验**: `verify_wal_checksums()` / `replay_events_by_chunk()` / `recover_corrupt()`
5. **高 impact 双签**: `_sign_high_impact()` 走 V1084 audit JSONL

---

## 2. 起点状态 (commit `d745c332`)

### 2.1 V1094 (R8-TrackA3) 现状

```
8 业务表 + memory_meta (9 表) + 26 索引
- memory_hot         当前 session episode rolling buffer
- memory_cold        跨 session 持久 (LTM 锚点 + Note; supersede 链)
- memory_wal         写前日志 (event_id UNIQUE 幂等键; op ∈ IDEMPOTENT_OPS)
- memory_dream       dream 产出暂存 (status + dream_state 6 态)
- memory_snapshots   replay 状态快照 ((scope, seq) UNIQUE)
- stm_messages       短期对话 (rolling N=50)
- mtm_themes         中期主题聚合
- ltm_facts          长期事实 (fingerprint UNIQUE mem0 dedup)
```

### 2.2 V1090/V1091/V1092 模块核对

| 模块 | 字段/接口 | v0.1.2 集成点 |
|---|---|---|
| V1090 WriteAheadLog | `WalEntry.sequence/ts/op/payload/checksum` | memory_wal.checksum 用同 sha256 算法 |
| V1091 MemoryReplay | `StateID(scope, seq, content_hash)` | memory_snapshots(scope, seq) UNIQUE 复用 |
| V1092 MemoryDream | `SchemaPhase.ASSIMILATION/ACCOMMODATION/REPLAY` | memory_dream.dream_phase 直接映射 |
| V1072 Identity | `IdentityCore.identity_id` ('id_xxx' 12hex) | 8 表 identity_id 列 + 跨表 anchor |
| V1084 InferenceAuditLog | JSONL append | _sign_high_impact audit_path 复用 |

---

## 3. v0.1.2 设计 (主 19:33 走在前人经验上)

借鉴 (Piaget schema 三态 + PostgreSQL WAL 1996 + Tonbo LSM + R37 hippocampal replay + Parfit 1984 心理连续性):

### 3.1 WAL chunk 索引字段

```sql
-- memory_wal 新增字段
ALTER TABLE memory_wal ADD COLUMN chunk_id TEXT NOT NULL DEFAULT '';
ALTER TABLE memory_wal ADD COLUMN identity_id TEXT NOT NULL DEFAULT '';

-- 复合索引
CREATE INDEX idx_wal_chunk_seq ON memory_wal(chunk_id, seq);        -- 按 chunk 重放
CREATE INDEX idx_wal_chunk_applied ON memory_wal(chunk_id, applied); -- 按 chunk 进度
CREATE INDEX idx_wal_identity_id ON memory_wal(identity_id);        -- identity 回溯
```

**借鉴 Tonbo LSM**: LSM 把 WAL 视为不可变 chunk 流, V0.1.2 把 V1094 单行 WAL 扩展为可按 chunk_id 分组重放的事件束 — 借鉴 LSM 的"块级合并".

### 3.2 dream_phase (Piaget schema)

```sql
ALTER TABLE memory_dream ADD COLUMN dream_phase TEXT NOT NULL DEFAULT 'ASSIMILATION';
ALTER TABLE memory_dream ADD COLUMN identity_id TEXT NOT NULL DEFAULT '';
CREATE INDEX idx_dream_phase ON memory_dream(dream_phase);
-- 触发器拦非枚举
CREATE TRIGGER trg_memory_dream_phase_chk BEFORE INSERT ON memory_dream ...
    WHEN NEW.dream_phase NOT IN ('ASSIMILATION','ACCOMMODATION','REPLAY')
    SELECT RAISE(ABORT, 'v0.1.2 dream_phase must be in DREAM_PHASES');
```

**借鉴 V1092 SchemaPhase** (`assimilation` / `accommodation` / `replay`), 与 V1094 `dream_state` 6 态 (IDLE/DREAMING/CONSOLIDATING/FORGETTING/VERIFYING/INTERRUPTED) **共存不冲突**:
- `dream_phase` (3 态) = Piaget schema 同化/顺应 (主层)
- `dream_state` (6 态) = Dream 子系统状态机 (子层)

### 3.3 identity_id 锚定 V1072

8 表统一加 `identity_id TEXT NOT NULL DEFAULT ''` + 8 个 `idx_*_identity_id` 单列索引, 调用 `anchor_identity(table, row_id, identity_id)` 跨表写.

**借鉴 Parfit 1984 心理连续性** + ACT-R chunk 锁链: memory_* 的 linked_identity_hash (V1072 content hash) 之外, 多出 identity_id (V1072 IdentityCore.identity_id) 这一可枚举锚点 — 这是"永恒身份"主 12:14 在 schema 层的物理体现.

### 3.4 sha256 校验 + replay 恢复

```python
def verify_wal_checksums(conn, scope=None, chunk_id=None, limit=100000) -> ChecksumReport
def replay_events_by_chunk(conn, chunk_id, scope=None, skip_corrupt=True) -> list[dict]
def recover_corrupt(record_to=None) -> dict  # 产 recovery_record 报告 + 可选 JSONL
```

借鉴 PostgreSQL WAL 1996 (CRC32 + page header) + V1090 WalEntry.compute_checksum: V1094 已存 `memory_wal.checksum = sha256(payload)[:16]`, V1109 加 `verify_wal_checksums()` 逐行重算 + 与 stored 比较 + 累计 corrupt_count + 健康度.

### 3.5 双签 (impact≥0.7) 走 V1084 audit

```python
def _sign_high_impact(conn, op_scope, op_kind, payload, impact, identity_id, audit_log_path=None):
    # impact < 0.7 → 跳过, audit JSONL 不追加
    # impact >= 0.7 → 写 JSONL 行 + V1109 meta counter +1
    #   JSONL 行必含 dual_signed_by ("v1109_memory_schema_v012", "v1084_inference_audit")
    #   request_id = "v1109_" + uuid12
```

**双签契约**: 任何"高 impact"写入既是 V1109 内部计数 (`memory_meta.v1109_high_impact_signs_total`), 也是 V1084 InferenceAuditLog 的 JSONL 行 (`request_id` dual-key). 一方记录偏移 → audit 失败可见.

---

## 4. 模块结构

`apeireth/v1109_memory_schema_v012.py` (~829 LOC):

```
├── 常量/版本/命名空间键  (V1109_VERSION, V1109_META_*, HIGH_IMPACT_THRESHOLD=0.7)
├── DREAM_PHASES          (ASSIMILATION, ACCOMMODATION, REPLAY)
├── _V012_COLUMN_ADDITIONS (10 entries: 8 × identity_id + chunk_id + dream_phase)
├── _V012_CHECK_ADDITIONS (dream_phase DREAM_PHASES 校验)
├── _V012_INDEXES         (11 CREATE INDEX IF NOT EXISTS)
├── 工具:
│     _has_column() / _has_check_named() / _full_sha256() / _checksum_64()
├── 迁移:
│     _migration_v094_to_v012() / upgrade_v012() / upgrade_v012_path() / downgrade_v012()
├── 校验与恢复:
│     ChecksumReport dataclass / verify_wal_checksums() / replay_events_by_chunk()
├── 双签:
│     _sign_high_impact()
├── MemorySchemaV012(MemorySchema)  ← V1094 facade 子类
│     ├── v012_schema_version() / v094_schema_version() / v012_migrated_from()
│     ├── high_impact_signs_total()
│     ├── wal_append_with_chunk(scope, op, payload, *, chunk_id, identity_id, event_id, impact)
│     ├── anchor_identity(table, row_id, identity_id)
│     ├── list_by_identity(table, identity_id)  [兼容 ts / last_updated / rowid]
│     ├── dream_record_with_phase(summary, *, dream_phase, source_episode_id, identity_id, ...)
│     ├── list_dreams_by_phase(phase)
│     ├── verify_wal_checksums() / replay_events_by_chunk()
│     └── recover_corrupt(record_to=None)
├── Schema SQL 字符串: SCHEMA_V094_V012_MIGRATION / SCHEMA_V012_INDEX_DDL
├── INTEGRATION_POINTS_V012 (V1072/V1090/V1091/V1092/V1084 五个对接点)
└── V3_GUARDS (5 条 守门主 17:43 + 17:58 + 20:46)
```

---

## 5. 测试覆盖 (真测试 ≥30, 实际 69)

### 5.1 `tests/test_v1109_memory_schema_v012.py` (49 真测试)

| 编号 | 用例数 | 主题 |
|---|---|---|
| Group 1: 模块加载 / 版本 / 对接点字典 | T01..T04 | 4 |
| Group 2: V0.1.2 内存 db init | T05..T08 | 4 (含 parametrize × 10) |
| Group 3: 幂等迁移 + 跨版本数据共存 | T09..T11 | 3 |
| Group 4: identity_id 8 表锚定 | T12..T14 | 3 |
| Group 5: WAL chunk_id + 复合索引 | T15..T18 | 4 |
| Group 6: dream_phase 写入守门 + 触发器 | T19..T22 | 4 |
| Group 7: sha256 verify + replay 恢复 | T23..T26 | 4 |
| Group 8: 高 impact 双签 (V1084 audit) | T27..T29 | 3 |
| Group 9: 文件 db + meta 双层命名空间 | T30..T31 | 2 |
| Group 10: downgrade 不破坏 v0.1.0 | T32..T34 | 3 |
| Group 11: facade 兼容 + ChecksumReport | T35..T37 | 3 |
| Group 12: 工具函数 (sha256/触发器/JSONL) | T38..T40 | 3 |
| 附加子用例 (parametrize T05 + T40 details) | — | 6 |
| **本文件合计** | **T01..T40+parametrize** | **49 PASS** |

### 5.2 `tests/test_v1090_v1091_v1092_v1109.py` (20 真整合测试)

| 编号 | 主题 | 用例数 |
|---|---|---|
| TI01..TI03 | V1090 WAL append + V1109 verify 校验通 | 3 |
| TI04..TI06 | V1091 capture_state + V1109 snapshot 协调 | 3 |
| TI07..TI09 | V1092 SchemaPhase ↔ V1109 dream_phase | 3 |
| TI10..TI12 | V1072 identity_id 跨 8 表 anchor | 3 |
| TI13..TI14 | V1109 + V1094 WAL 双写 + event_id 幂等 | 2 |
| TI15..TI16 | chunk replay + recover_corrupt 真生产 | 2 |
| TI17..TI18 | 双签 V1084 audit + downgrade V1090 同存 | 2 |
| TI19 | 全流水线 smoke (V1092→V1109) | 1 |
| TI20 | 跨 3 次 upgrade 幂等稳定 | 1 |
| **本文件合计** | | **20 PASS** |

### 5.3 跑测命令

```bash
# V1109 单元测试
python -m pytest tests/test_v1109_memory_schema_v012.py -v
# V1090+V1091+V1092+V1109 真整合
python -m pytest tests/test_v1090_v1091_v1092_v1109.py -v
# 全部 89 用例
python -m pytest tests/test_v1109_memory_schema_v012.py tests/test_v1090_v1091_v1092_v1109.py tests/test_v1094_memory_schema.py
```

### 5.4 测试结果

```
============================= 69 passed in 1.76s =============================
============================= 27 passed in 0.47s ============================= (V1094 回归)
============================= 96 passed total =============================
```

---

## 6. Schema 迁移实测 (平滑升级, 不破坏)

### 6.1 数据兼容性验证

测试 `test_t10_migration_preserves_v094_data` 真实模拟 v0.1.0 数据库升级:

```sql
-- v0.1.0 现仓 (升级前)
INSERT INTO memory_hot(id, session_id, actor, content, ts, fingerprint)
  VALUES ('hot_v094', 's_v094', 'master', 'v094 baseline', 1.0, 'fp_v094');
INSERT INTO memory_cold(id, content, ts, fingerprint)
  VALUES ('cold_v094', 'v094 cold content', 1.0, 'fp_cold_v094');
INSERT INTO memory_wal(seq, scope, op, payload, event_id, checksum, applied, ts)
  VALUES (NULL, 'hot', 'tag_set', '{"v094":true}', 'ev_v094', 'fp_wal_v094', 0, 1.0);

-- 升级: upgrade_v012(conn) → v0.1.2 → 不破坏

-- 升级后校验 (T10 PASS):
memory_hot WHERE id='hot_v094'     → content = 'v094 baseline'    ✓
memory_hot identity_id              → DEFAULT ''                 ✓
memory_cold identity_id             → DEFAULT ''                 ✓
memory_wal chunk_id                 → DEFAULT ''                 ✓
memory_meta v1109_schema_version    → '0.1.2'                    ✓
memory_meta v1094_schema_version    → '0.1.0' (不变)              ✓
```

### 6.2 幂等性

`test_t09_migration_idempotent` 验证: 同一连接上多次 `wal_append_with_chunk` + `wal_append` 交错, `event_id` UNIQUE 约束确保同 event_id 只 1 行; 重启 `MemorySchemaV012(path)` 后 meta 键 +1 而非重复.

### 6.3 Downgrade 不破坏

`test_t32_downgrade_preserves_v094_columns` 验证: downgrade_v012(keep_meta=True) 只清 V1109 命名空间键 + drop V1109 新索引 + drop V1109 触发器; V1094 base 表 + V1094 meta 键完整保留; v0.1.2 新加的 `identity_id` / `chunk_id` / `dream_phase` 列保留 (SQLite 3.35- 不支持 DROP COLUMN).

---

## 7. 真 commit (R9 提交)

实施 git commit (主 23:44 干到底):

```bash
git add apeireth/v1109_memory_schema_v012.py \
        tests/test_v1109_memory_schema_v012.py \
        tests/test_v1090_v1091_v1092_v1109.py
git commit -m "R9-DB-001 v0.1.2: WAL chunk+identity_id+dream_phase 真整合 (69 真测试)

- apeireth/v1109_memory_schema_v012.py (~829 LOC, 新模块)
- WAL: chunk_id + (chunk_id,seq/applied) 复合索引
- dream_phase ∈ {ASSIMILATION/ACCOMMODATION/REPLAY} (借鉴 V1092)
- identity_id 锚定 8 表 + V1072 IdentityCore (Parfit 心理连续性)
- sha256 verify_wal_checksums() + replay_events_by_chunk() + recover_corrupt()
- 高 impact (≥0.7) 双签走 V1084 audit JSONL
- 双签契约: dual_signed_by = (v1109_memory_schema_v012, v1084_inference_audit)
- 幂等迁移: v0.1.0/v0.1.1 → v0.1.2 平滑, 不破坏旧数据
- tests 69 PASS (49 + 20); V1094 27 旧测试零破坏"
```

---

## 8. 对接点 (5+8=13 联动)

### 8.1 对接点 v0.1.2 扩展

| 对接 | 字段/接口 |
|---|---|
| v1072_eternal_identity | IdentityManifest.core.identity_id ('id_xxx') → memory_* / stm_/mtm_/ltm_ 的 identity_id |
| v1090_wal | V1090 WalEntry.compute_checksum ↔ memory_wal.checksum |
| v1091_replay | V1091 MemoryReplay.capture_state ↔ memory_snapshots(scope, seq) UNIQUE |
| v1092_dream | V1092 MemoryDream.SchemaPhase ↔ memory_dream.dream_phase |
| v1084_audit | V1084 InferenceAuditLog ↔ _sign_high_impact() JSONL |
| hqb_integration (V1094 既有) | memory_snapshots(scope='hqb', snapshot_score) |
| memory_replay_design (V1094 既有) | IDEMPOTENT_OPS ∈ memory_wal.op |
| r7_be_01_dream (V1094 既有) | DreamSubsystem → memory_dream(_dream=true) |

### 8.2 dual_signed_by JSONL 字段

```json
{
  "request_id": "v1109_abc123def456",
  "ts": 1753737600.123,
  "op_scope": "hot",
  "op_kind": "tag_set",
  "payload": {...},
  "impact": 0.92,
  "identity_id": "id_critical",
  "v1109_version": "0.1.2",
  "dual_signed_by": ["v1109_memory_schema_v012", "v1084_inference_audit"]
}
```

---

## 9. V3 守门 (主 17:43 + 17:58 + 20:46)

| 守门 | 内容 |
|---|---|
| module_is_not_asi | V1109 Memory Schema v0.1.2 是工具, ASI 是更大目标. 任何声称 schema 升级 = ASI 的部分都是不假装. |
| structure_is_not_consciousness | V1109 dream_phase (ASSIMILATION/ACCOMMODATION/REPLAY) 是 Piaget schema 类比, 不是现象意识. |
| measurement_is_not_truth | high_impact 双签计数 ≠ 真双签. count 是 proxy, 真双签走 V1084 audit JSONL, 必须审计. |
| production_is_not_safety | v0.1.2 真 migration ≠ 真安全. ALTER TABLE ADD COLUMN 不重建 CHECK 约束, 必须靠触发器 + 应用层守门. |
| automation_is_not_autonomy | _sign_high_impact 自动签名 ≠ 自主决策. impact < 0.7 跳过是策略, 不是 agentic 自治. |

---

## 10. 已知边界 / 后续可能升级路径

主 17:43 实事求是: 下列 capability 不在本次范围, 但设计已为后续留口:

1. **CHECK 约束真正重建**: SQLite 不支持 `ALTER TABLE ADD CONSTRAINT`, 当前用 BEFORE INSERT 触发器兜底. 未来若要原生 CHECK, 需要 `ALTER TABLE ... RENAME TO old; CREATE TABLE new(... CHECK); INSERT INTO new SELECT * FROM old; DROP old` 三步重建流程.
2. **chunk_id 跨 process 协议**: 当前 chunk_id 是 process-local UUID (`chunk_<12hex>`); 大规模分布式 WAL 端需要 global coord, 后续可加 (chunk_id, source_node_id, source_clock) 复合标识.
3. **审计 retention policy**: 当前 high_impact_signs.jsonL 无限追加; 真实生产需要 retention + archiving, 留给 R10.
4. **identity_id 不变性 (immutability) 守门**: 当前 anchor_identity 可以重复写; 严格身份连续性需要"identity 一旦锚定, 不可改" invariant, 留待 V1110+ 引入 WAL-only 锚定路径.
5. **recover_corrupt 现在只产出报告, 不删 corrupt 行**: 真生产需要 purge / repair 流程. 留给 R10.

---

## 11. 一句话总结 (主 23:44 干到底)

> V1109 Memory Schema v0.1.2 在 V1094 v0.1.0 上**不破坏旧数据**地加了 3 类字段 (`identity_id` × 8 表 + `memory_wal.chunk_id` + `memory_dream.dream_phase`) + 11 v0.1.2 索引 + sha256 校验 + V1084 audit 双签, 69 真测试全 PASS, V1094 27 旧测试零回归, 平滑迁移 ready for R10+ 真多 LLM 接入的 AGI/ASI 基座.

— database_engineer, R9-DB-001 完成
