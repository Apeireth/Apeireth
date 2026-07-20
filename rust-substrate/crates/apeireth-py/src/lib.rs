//! PyO3 binding — Python 调 Rust (JSON I/O 模式)
//!
//! 主人 14:47 "多语言混合, 核心 Rust" — 这就是桥
//! 主人 14:32 "高效 nb 不 Python 糊弄" — 验证 Rust 真快

use apeireth_core::{Episode, IdentityCard, Note};
use apeireth_core::episode::{Actor, EpisodeKind};
use apeireth_core::forget;
use apeireth_core::reconsolidate;
use pyo3::prelude::*;

/// AppendEpisode — Python 传 JSON 字符串进来, Rust 返回插入状态
#[pyfunction]
fn py_append_episode(
    actor: &str,
    content: &str,
    context: &str,
    kind: &str,
    linked_identity_hash: &str,
    tier: &str,
) -> PyResult<String> {
    let ep = Episode::new(
        Actor::from_str(actor),
        content,
        context,
        EpisodeKind::from_str(kind),
        linked_identity_hash,
        tier,
    );
    let json = serde_json::json!({
        "eid": ep.eid,
        "fingerprint": ep.fingerprint,
        "actor": format!("{:?}", ep.actor),
        "kind": format!("{:?}", ep.kind),
        "tier": ep.tier,
        "ts": ep.ts.to_rfc3339(),
    });
    Ok(json.to_string())
}

/// ForgetSweep — 主人 13:47 关心 (防 sycophancy)
#[pyfunction]
fn py_forget_sweep(notes_json: &str, threshold: f64) -> PyResult<String> {
    let mut notes: Vec<Note> = serde_json::from_str(notes_json)
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
    let stats = forget::forget_sweep(&mut notes, threshold);
    let out = serde_json::json!({
        "scanned": stats.scanned,
        "forgotten": stats.forgotten,
        "kept": stats.kept,
        "avg_salience_kept": stats.avg_salience_kept,
        "remaining_notes": serde_json::to_string(&notes).unwrap_or_default(),
    });
    Ok(out.to_string())
}

/// Reconsolidate — 4 paths (boost / flag / align / none)
#[pyfunction]
fn py_reconsolidate(notes_json: &str, card_json: &str) -> PyResult<String> {
    let mut notes: Vec<Note> = serde_json::from_str(notes_json)
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
    let card: IdentityCard = serde_json::from_str(card_json)
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
    let stats = reconsolidate::reconsolidate(&mut notes, &card);
    let out = serde_json::json!({
        "boost": stats.boost,
        "flag": stats.flag,
        "align": stats.align,
        "none": stats.none,
        "identity_hash": stats.identity_hash,
        "notes": serde_json::to_string(&notes).unwrap_or_default(),
    });
    Ok(out.to_string())
}

/// Version
#[pyfunction]
fn py_version() -> PyResult<String> {
    Ok(env!("CARGO_PKG_VERSION").to_string())
}

#[pymodule]
fn apeireth_py(_py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(py_version, m)?)?;
    m.add_function(wrap_pyfunction!(py_append_episode, m)?)?;
    m.add_function(wrap_pyfunction!(py_forget_sweep, m)?)?;
    m.add_function(wrap_pyfunction!(py_reconsolidate, m)?)?;
    Ok(())
}