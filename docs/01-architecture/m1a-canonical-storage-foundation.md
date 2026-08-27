# M1A Canonical Storage Foundation — Port Note

> **现状 (2026-08-27)**：本文是 v1 时代（master 线/86-crate）或 reconstruct_v2 过程中的历史快照，正文保留原样。当前基线：默认分支 `main`、13-crate 工作区（`crates/foundation|engine|capabilities|adapters`，见根 `ARCHITECTURE.md` 与 `docs/01-architecture/architecture.md`）、tag `v2.0.0-alpha.1` @ `d6910cf7`；旧 86-crate 代码整体在 `legacy/`（workspace exclude）；v2 下一步见根 `ROADMAP.md` §4。

## Source

```text
origin/master:reconstruction_v2/crates/apeireth-storage
```

Files read:

```text
Cargo.toml
src/pool.rs
src/migrations.rs
src/lib.rs
```

## Ported semantics

```text
- Single serialized writer + reader pool architecture (verified, not just type names)
- SQLite WAL, synchronous=NORMAL, foreign_keys=ON, busy_timeout=5000
- Per-connection PRAGMA initialization through r2d2_sqlite with_init
- Write path ordered through an mpsc channel; result returned via oneshot
- Shared in-memory database support (SQLite :memory: is per-connection, so
  canonical uses r2d2_sqlite::SqliteConnectionManager::memory instead)
- StorageError contract: Open / Db / Pool / WriteQueue / Migration / InvalidConfiguration
- Versioned migrations using PRAGMA user_version, transaction-per-migration,
  rollback on failure, idempotent v1 for donor on-disk compatibility
```

## Implementation changed

```text
- r2d2_sqlite: donor 0.24 (rusqlite 0.31) -> 0.25 (rusqlite 0.32) to obey the
  workspace rusqlite 0.32 hard lock and avoid a libsqlite3-sys links conflict
- Writer connection is created directly from the manager before the reader pool
  is built, so the writer does not consume a reader pool slot
- Reader access is a short-lived closure (`read`) rather than a returned
  PooledConnection; mutations must go through `write`
- Migration engine is versioned (user_version); donor had none
- Config is explicit (`SqliteConfig`) with validation; donor hardcoded PRAGMAs
- StorageError separates Open/Migration/InvalidConfiguration instead of donor's
  narrower variants
```

## Not ported

```text
MemoryStore
MemoryItem
ACT-R activation
VectorIndex
Graph
Context folding
Session persistence
Runtime/Gateway/CLI storage integration
Memory_* support modules
```

## On-disk compatibility

Migration version 1 preserves the donor table set:

```text
episodes, notes, sessions, agent_traces, facts, links, topic_groups, provenance
```

and the `idx_facts_id` index. The SQL is intentionally `CREATE IF NOT EXISTS`
so a donor-created database with `user_version = 0` is upgraded idempotently
without data loss.