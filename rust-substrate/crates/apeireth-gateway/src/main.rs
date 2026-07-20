//! Gateway — HTTP / JSON-RPC server (借鉴 MemoryOS-Rust gateway 13 KB)
//!
//! Routes:
//! - GET  /health → health check
//! - POST /episodes → append episode
//! - GET  /episodes?tier=stm&limit=10 → list episodes
//! - POST /notes/upsert → upsert note
//! - POST /notes/forget-sweep → forget sweep
//! - POST /notes/reconsolidate → reconsolidate against identity

use axum::{
    routing::{get, post},
    Router,
    Json, extract::{State, Query},
};
use std::sync::Arc;
use apeireth_core::{Episode, EpisodeKind, Actor, Note, IdentityCard};
use apeireth_adapters::{SqliteEpisodeRepository, SqliteNoteRepository};
use apeireth_ports::{EpisodeRepository, NoteRepository};
use serde::{Deserialize, Serialize};

#[derive(Clone)]
struct AppState {
    episodes: Arc<SqliteEpisodeRepository>,
    notes: Arc<SqliteNoteRepository>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let episodes = SqliteEpisodeRepository::open("data/apeireth.db")?;
    let notes = SqliteNoteRepository::open("data/apeireth.db")?;
    let state = AppState {
        episodes: Arc::new(episodes),
        notes: Arc::new(notes),
    };

    let app = Router::new()
        .route("/health", get(health))
        .route("/episodes", post(create_episode).get(list_episodes))
        .route("/notes/upsert", post(upsert_note))
        .route("/notes/forget-sweep", post(forget_sweep))
        .route("/notes/reconsolidate", post(reconsolidate))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await?;
    tracing::info!("Apeireth gateway listening on http://0.0.0.0:8080");
    axum::serve(listener, app).await?;
    Ok(())
}

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({"status": "ok", "version": "0.1.0"}))
}

#[derive(Deserialize)]
struct CreateEpisode {
    actor: String,
    content: String,
    context: String,
    kind: String,
    linked_identity_hash: String,
    tier: String,
}

async fn create_episode(
    State(state): State<AppState>,
    Json(req): Json<CreateEpisode>,
) -> Result<Json<serde_json::Value>, String> {
    let ep = Episode::new(
        Actor::from_str(&req.actor),
        req.content,
        req.context,
        EpisodeKind::from_str(&req.kind),
        req.linked_identity_hash,
        req.tier,
    );
    let inserted = state.episodes.append(&ep).await.map_err(|e| e.to_string())?;
    Ok(Json(serde_json::json!({
        "eid": ep.eid,
        "inserted": inserted,
    })))
}

#[derive(Deserialize)]
struct ListQuery {
    tier: Option<String>,
    limit: Option<usize>,
}

async fn list_episodes(
    State(state): State<AppState>,
    Query(q): Query<ListQuery>,
) -> Result<Json<Vec<serde_json::Value>>, String> {
    let tier = q.tier.unwrap_or_else(|| "stm".to_string());
    let limit = q.limit.unwrap_or(10);
    let eps = state.episodes.list_by_tier(&tier, limit).await.map_err(|e| e.to_string())?;
    let out: Vec<_> = eps.iter().map(|e| serde_json::json!({
        "eid": e.eid,
        "actor": format!("{:?}", e.actor),
        "content": e.content,
        "context": e.context,
        "kind": format!("{:?}", e.kind),
        "ts": e.ts.to_rfc3339(),
        "linked_identity_hash": e.linked_identity_hash,
        "tier": e.tier,
    })).collect();
    Ok(Json(out))
}

async fn upsert_note(
    State(state): State<AppState>,
    Json(note): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, String> {
    let note: Note = serde_json::from_value(note).map_err(|e| e.to_string())?;
    let ok = state.notes.upsert(&note).await.map_err(|e| e.to_string())?;
    Ok(Json(serde_json::json!({"upserted": ok})))
}

#[derive(Deserialize)]
struct ForgetReq { threshold: f64 }

async fn forget_sweep(
    State(state): State<AppState>,
    Json(req): Json<ForgetReq>,
) -> Result<Json<serde_json::Value>, String> {
    use apeireth_core::forget;
    let mut notes = state.notes.list(1000).await.map_err(|e| e.to_string())?;
    let stats = forget::forget_sweep(&mut notes, req.threshold);
    Ok(Json(serde_json::json!({
        "scanned": stats.scanned,
        "forgotten": stats.forgotten,
        "kept": stats.kept,
    })))
}

async fn reconsolidate(
    State(state): State<AppState>,
    Json(card_json): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, String> {
    use apeireth_core::reconsolidate;
    let card: IdentityCard = serde_json::from_value(card_json).map_err(|e| e.to_string())?;
    let mut notes = state.notes.list(1000).await.map_err(|e| e.to_string())?;
    let stats = reconsolidate::reconsolidate(&mut notes, &card);
    Ok(Json(serde_json::json!({
        "boost": stats.boost,
        "flag": stats.flag,
        "align": stats.align,
        "none": stats.none,
        "identity_hash": stats.identity_hash,
    })))
}