# apeireth-storage

> Engine layer: the canonical storage foundation — SQLite pool management, configuration, schema migrations and storage-level errors.

`apeireth-storage` sits in the **engine** layer of the Apeireth engine workspace. It
owns the low-level, architecture-neutral persistence primitives that the memory, session
and credential layers are built on.

## What is inside

- `pool.rs` — `SqliteConnectionPool`, `SqliteConfig`, `JournalMode`, `SynchronousMode`
- `migrations.rs` — `Migration`, `run_migrations`, `run_migrations_report`,
  `current_version`, `LATEST_SCHEMA_VERSION` (currently `3`)
- `error.rs` — `StorageError`
- `crypto.rs` — the at-rest crypto primitives used by the sealed-column and sealed-backup
  surfaces (AES-256-GCM + HMAC-SHA-256 / PBKDF2, zeroizing key material)
- `cache.rs` — LRU + TTL cache primitives
- `quota.rs` — snapshot quota accounting
- `rate_limit.rs` — in-process rate-limiter algorithms
- `machine_id.rs` — portable machine-id probe

## What it deliberately does not own

It does **not** own memory, vector, graph, session, runtime, gateway, provider or
governance logic. Those layers build on top of this foundation. In particular, there is
no second credential owner here: credential storage lives in `apeireth-credentials`.

## Honest limits

- `pool.rs` and `migrations.rs` carry the implemented baseline; the on-disk schema
  version is tracked by `LATEST_SCHEMA_VERSION`.
- Crypto helpers here are primitives; the backup/export/verify workflow itself lives in
  `apeireth-tools-canonical` (`tool.backup_export`).

## Install

```toml
[dependencies]
apeireth-storage = "2.0.0-rc.4"
```

## License

Apache-2.0 OR MIT.

## Links

- Source: <https://github.com/Apeireth/Apeireth/tree/main/crates/engine/storage>
- Workspace manifest: <https://github.com/Apeireth/Apeireth/tree/main/crates/engine/storage/Cargo.toml>
- Architecture docs: <https://github.com/Apeireth/Apeireth/tree/main/docs/01-architecture>
