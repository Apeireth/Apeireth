//! Integration tests for apeireth-memory (service-based, real SQLite in-memory)
//!
//! **R18 第 2 阶段第 10 项**: 测 SqliteMemoryStore 真实 SQLite (in-memory 模式)

use apeireth_memory::SqliteMemoryStore;

fn store() -> SqliteMemoryStore {
    SqliteMemoryStore::open_in_memory().expect("open in-memory")
}

#[test]
fn store_opens_in_memory() {
    let _store = store();
}

#[test]
fn store_runs_at_least_one_migration() {
    let s = store();
    let applied = s.applied_migrations().expect("applied_migrations");
    assert!(!applied.is_empty(), "expected at least 1 migration");
}

#[test]
fn store_export_streams_jsonl_empty() {
    let s = store();
    let entries = s.export_streams_jsonl().expect("export_streams_jsonl");
    assert!(
        entries.is_empty(),
        "expected 0 history entries in fresh DB, got {}",
        entries.len()
    );
}

#[test]
fn store_open_in_memory_creates_fresh_db() {
    let s1 = store();
    let applied1 = s1.applied_migrations().expect("applied 1");
    drop(s1);
    let s2 = store();
    let applied2 = s2.applied_migrations().expect("applied 2");
    assert_eq!(applied1.len(), applied2.len());
}

#[test]
fn store_migration_ids_are_positive() {
    let s = store();
    let applied = s.applied_migrations().expect("applied");
    for id in &applied {
        assert!(*id > 0, "migration id should be positive, got {}", id);
    }
}

#[test]
fn store_migration_ids_sorted_ascending() {
    let s = store();
    let mut applied = s.applied_migrations().expect("applied");
    let mut sorted = applied.clone();
    sorted.sort();
    assert_eq!(applied, sorted, "migrations should be in ascending order");
}
