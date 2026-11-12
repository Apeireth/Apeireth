//! Follow durable derivation/correction links to independent Runtime input markers.
use super::audit::SourceAudit;
use apeireth_memory::{EpisodeStore, MemoryGovernanceStore, SqliteMemoryStore};
use rusqlite::OptionalExtension;
use std::collections::BTreeSet;
use std::path::Path;

fn applied(db: &Path, operation: &str) -> bool {
    rusqlite::Connection::open_with_flags(db,rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY).ok().and_then(|connection|connection.query_row("SELECT payload_hash FROM memory_recovery_operation_receipts WHERE operation_id=?1",[operation],|row|row.get::<_,String>(0)).optional().ok().flatten()).is_some_and(|hash|hash.len()==64 && hash.bytes().all(|b|b.is_ascii_hexdigit()))
}

fn raw(
    store: &SqliteMemoryStore,
    db: &Path,
    id: &str,
    audit: &SourceAudit,
    seen: &mut BTreeSet<String>,
) -> bool {
    if seen.len() >= 16 || !seen.insert(id.into()) {
        return false;
    }
    let result = (|| {
        let episode = EpisodeStore::get_episode(store, id).ok().flatten()?;
        let governed = store.get_governed(id).ok().flatten()?;
        if governed.status.as_str() != "active" {
            return Some(false);
        }
        let meta = apeireth_plugin::memory_backend::MemoryBackend::get_episode_metadata(store, id)
            .ok()
            .flatten()?;
        if episode.role == "fact_history" {
            let parent = meta["derived_from_episode_id"].as_str()?;
            let evidence = meta["correction_evidence_id"].as_str()?;
            let operation = meta["correction_operation"].as_str()?;
            return Some(
                applied(db, operation)
                    && raw(store, db, parent, audit, seen)
                    && raw(store, db, evidence, audit, seen),
            );
        }
        if episode.role == "memory" {
            if let Some(parent) = meta["derived_from_episode_id"].as_str() {
                return Some(raw(store, db, parent, audit, seen));
            }
        }
        let request = meta["provenance"]["source_request"].as_str()?;
        Some(audit.verify(
            &episode.session_id,
            request,
            &episode.role,
            &episode.content,
        ))
    })()
    .unwrap_or(false);
    seen.remove(id);
    result
}

pub fn verify(store: &SqliteMemoryStore, db: &Path, id: &str, audit: &SourceAudit) -> bool {
    if !raw(store, db, id, audit, &mut BTreeSet::new()) {
        return false;
    }
    let Some(governed) = store.get_governed(id).ok().flatten() else {
        return false;
    };
    if governed.content_override.is_none() {
        return true;
    }
    let Some(meta) =
        apeireth_plugin::memory_backend::MemoryBackend::get_episode_metadata(store, id)
            .ok()
            .flatten()
    else {
        return false;
    };
    let Some(operation) = meta["correction"]["operation"].as_str() else {
        return false;
    };
    let Some(evidence) = meta["correction"]["evidence"].as_str() else {
        return false;
    };
    let Some(source) = EpisodeStore::get_episode(store, evidence).ok().flatten() else {
        return false;
    };
    source.role == "user"
        && source.session_id == governed.episode.session_id
        && applied(db, operation)
        && raw(store, db, evidence, audit, &mut BTreeSet::new())
}
