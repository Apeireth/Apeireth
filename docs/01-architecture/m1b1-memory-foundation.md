# M1B1 — Canonical Memory Foundation

> **现状 (2026-08-27)**：本文是 v1 时代（master 线/86-crate）或 reconstruct_v2 过程中的历史快照，正文保留原样。当前基线：默认分支 `main`、13-crate 工作区（`crates/foundation|engine|capabilities|adapters`，见根 `ARCHITECTURE.md` 与 `docs/01-architecture/architecture.md`）、tag `v2.0.0-alpha.1` @ `d6910cf7`；旧 86-crate 代码整体在 `legacy/`（workspace exclude）；v2 下一步见根 `ROADMAP.md` §4。

## Status

Implemented on `reconstruct_v2`. Migration version after this phase: `2`.

## Boundary decision

- Canonical memory owner: `apeireth-memory::canonical`
- Storage owner: `apeireth-storage` (SQLite pool + migrations)
- Runtime integration: none

## Donor source

- `origin/master:reconstruction_v2/crates/apeireth-storage/src/memory_v2.rs`

## Semantic classification

| Donor concept | Classification | Disposition |
| --- | --- | --- |
| `MemoryItem` fields (`id`, `data`, `importance`, `access_count`, `access_times`, `created_at`, `valid_from`, `valid_until`, `is_tombstone`, `artifact_sig`) | DOMAIN | Ported as `canonical::MemoryItem` with core `Timestamp` time fields and a typed `MemoryId` |
| `MemoryOperation` | DOMAIN operation | Adapted into explicit repository methods (`insert`, `update`, `tombstone`) instead of a catch-all apply op |
| `QueryMode` | RETRIEVAL | Deferred to M1B2 (temporal filtering is available as `MemoryFilter`; no ranking) |
| ACT-R activation | RETRIEVAL | Deferred to M1B2 |
| `facts` JSON table | PERSISTENCE DETAIL | Rejected for canonical use. v1 `facts` remains untouched for on-disk compatibility, but canonical memory uses a typed `memory_items` table from migration v2 |
| Automatic SHA-256 `artifact_sig` | DOMAIN policy | Adapted: `artifact_sig` is persisted as an optional caller-supplied fingerprint; automatic hashing is not performed in M1B1 |

## Domain model

`MemoryItem` fields: `id` (`MemoryId`), `data`, `importance`, `access_count`, `access_times`, `created_at`, `valid_from`, `valid_until`, `is_tombstone`, `artifact_sig`.

Temporal validity is deterministic and explicit:

- `valid_from` inclusive, `valid_until` exclusive
- `valid_until = None` means valid indefinitely
- boundary tests cover all five required cases

## Repository contract

`MemoryRepository` exposes `insert`, `get`, `update`, `query`, `tombstone`.

Normal reads exclude tombstones. Historical/tombstoned reads require `MemoryFilter::with_include_tombstones(true)`.

## SQLite backend

`SqliteMemoryRepository` wraps `apeireth_storage::SqliteConnectionPool`. It never opens a raw `rusqlite::Connection`, never creates an `r2d2` pool, and never spawns a writer thread.

## Schema

- v1: unchanged
- v2: `memory_items` typed table with indexes on temporal and tombstone columns
- Upgrade tested: fresh `0 -> 2`, existing `1 -> 2`, idempotent `2 -> 2`, failed migration not marked complete

## Architecture gate

- Memory -> Runtime: NO
- Memory -> Gateway: NO
- Memory -> Provider: NO
- Memory -> Companion: NO
- Storage -> Memory: NO
- Memory uses canonical storage pool: YES
- ACT-R/vector/graph imported: NO
