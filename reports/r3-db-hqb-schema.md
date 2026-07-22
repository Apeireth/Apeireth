# R3-DB-01 — HQB DB Schema 草案

> 数据库工程师 | 2026-07-22 | schema v0.1.0

## 4 表结构

- `hqb_decisions` — id TEXT PK · task_id · decision · score REAL · philosophy_guard_status · snapshot_score · ts
- `hqb_guard_events` — id TEXT PK · decision_id FK→decisions(id) CASCADE · guard_type · passed INT(0/1) · reason · ts
- `hqb_asi_deltas` — id TEXT PK · decision_id FK·CASCADE · asiv0_before · asiv0_after · lift_value · ts
- `hqb_trace` — id TEXT PK · parent_id FK→trace(id) SET NULL · action · rationale · ts
- `hqb_meta` — k TEXT PK · v TEXT (schema_version=0.1.0)

Indexes: `idx_hqb_decisions_task/ts`, `idx_hqb_guard_decision/type`, `idx_hqb_delta_decision/ts`, `idx_hqb_trace_parent/ts`.

## 现仓 DB 现状

SQLite stdlib + `CREATE TABLE IF NOT EXISTS` + `<name>_meta.schema_version`，**无 alembic**；3 个独立 db (memory/graph/identity).

## Migration 兼容性

HQB 沿用现仓模式 (hqb_meta v0.1.0 + 独立 hqb.db). 表前缀 `hqb_` 与 `episodes/graph_*/identity_*` 不重名 — 跨 db 命名空间检查通过. **零破坏**: 未碰 `asi_snapshot.json` / `philosophy_guard`; 仅 additive 新增 `apeireth/hqb/`.

## smoke_load.py 输出

```
HQB smoke (R3-DB-01) | schema_version=0.1.0
T1: [OK] :memory: store, decision insert/read (t-001, guard=pass)
T1: [OK] guard_event + asi_delta lift_value=0.0045 (auto)
T1: [OK] trace chain parent→child linked
T1: [OK] delete_decision CASCADE: 1+1+1 removed
T2: [OK] wrote/reopened hqb_smoke.db, decision survived
T3: [OK] init_schema x3 — no duplicate table errors
=== ALL SMOKE TESTS PASSED ===
```

## V1088 嵌入 HQB 计划

1. V1088 backend-engineer 每次评分后 `HqbStore.record_decision(...)`.
2. V1086 runner harness edit 前 `record_trace(action, parent_id)`, edit 后 `record_delta(...)` 自动算 `lift_value`.
3. V1074 末尾 export `hqb.db` → `artifacts/hqb_snapshot.json`.

## 交付

`apeireth/hqb/__init__.py` (7 行) · `schema.py` (184 行) · `smoke_load.py` (3 用例全过). 0 行触及现仓真生产.