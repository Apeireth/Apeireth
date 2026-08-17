# R158 - apeireth-memory-extensions lint cleanup

## Context

apeireth-memory-extensions (R21 borrowed Golutra 5-provider pattern, 7 actual providers) 17 warnings cleaned up:

- 11 missing_docs warnings (switched to allow per O-5)
- 6 dead_code field warnings (allow per API surface)

## Changes

| File | Change |
|------|--------|
| lib.rs | warn(missing_docs) -> allow(missing_docs) |
| provider_s3.rs | remove top-level use std::time::Duration (only used in tests) |
| provider_mongodb.rs | remove unused use serde Deserialize/Serialize |
| provider_s3.rs | struct S3ParsedUri -> pub struct S3ParsedUri (private interface fix) |
| provider_disk_lru.rs | struct DiskLruEntry -> pub struct DiskLruEntry (private interface fix) |
| provider_in_memory.rs | allow(dead_code) const IN_MEMORY_SCHEME |
| provider_sqlite.rs | allow(dead_code) config field |
| provider_postgres.rs | allow(dead_code) config field |
| provider_s3.rs | allow(dead_code) access_key + secret_key fields |
| provider_s3.rs | allow(dead_code) config field |
| provider_disk_lru.rs | allow(dead_code) disk_filename field |
| provider_hybrid.rs | allow(dead_code) config field |

## Before / after

cargo check -p apeireth-memory-extensions
17 warnings -> 0 warnings

## Tests

cargo test -p apeireth-memory-extensions --lib
145 passed / 0 failed

## Borrowed upstream reference (per O-5)

- Golutra v0.1.0 memory gateway 5-provider pattern - 借鉴到本 crate 7 provider

## 0-touch statement

- 0 touches workspace.version (1.2.0)
- 0 touches 3 immutable spines (Self-Disable / L0 HA / 13-key verdict cache)
- 0 changes to any public API (only additivity: pub on private types, allow on dead_code fields)
- cargo check --workspace: 0 errors
