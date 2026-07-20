//! PyO3 binding — Python 调 Rust
//!
//! 主人 14:47 "多语言混合, 核心 Rust" — 这就是桥

use apeireth_core::{Episode, EpisodeKind, Actor, Note, IdentityCard};
use apeireth_adapters::SqliteEpisodeRepository;
use apeireth_ports::EpisodeRepository;
use pyo3::prelude::*;

#[pyclass]
pub struct PyEpisodeRepo {
    inner: SqliteEpisodeRepository,
}

#[pymethods]
impl PyEpisodeRepo {
    #[new]
    fn new(path: &str) -> PyResult<Self> {
        let repo = SqliteEpisodeRepository::open(path)
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
        Ok(Self { inner: repo })
    }

    fn append(&self, py: Python, actor: &str, content: &str, context: &str, kind: &str, identity_hash: &str, tier: &str) -> PyResult<bool> {
        let episode = Episode::new(
            Actor::from_str(actor),
            content,
            context,
            EpisodeKind::from_str(kind),
            identity_hash,
            tier,
        );
        let rt = tokio::runtime::Runtime::new().map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
        let result = rt.block_on(self.inner.append(&episode))
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
        // tell py we're not actually waiting
        let _ = py.allow_threads(|| {});
        Ok(result)
    }

    fn count(&self) -> PyResult<u64> {
        let rt = tokio::runtime::Runtime::new().map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
        rt.block_on(self.inner.count())
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))
    }

    fn list_by_tier(&self, tier: &str, limit: usize) -> PyResult<Vec<PyDict>> {
        let rt = tokio::runtime::Runtime::new().map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
        let eps = rt.block_on(self.inner.list_by_tier(tier, limit))
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
        let mut out = Vec::with_capacity(eps.len());
        for ep in eps {
            let d = PyDict::new_bound(py);
            d.set_item("eid", &ep.eid)?;
            d.set_item("actor", format!("{:?}", ep.actor))?;
            d.set_item("content", &ep.content)?;
            d.set_item("context", &ep.context)?;
            d.set_item("kind", format!("{:?}", ep.kind))?;
            d.set_item("ts", ep.ts.to_rfc3339())?;
            d.set_item("linked_identity_hash", &ep.linked_identity_hash)?;
            d.set_item("tier", &ep.tier)?;
            d.set_item("fingerprint", &ep.fingerprint)?;
            out.push(d.unbind());
        }
        Ok(out)
    }
}

#[pyfunction]
fn py_forget_sweep(notes_json: &str, threshold: f64) -> PyResult<String> {
    use apeireth_core::forget;
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

#[pyfunction]
fn py_reconsolidate(notes_json: &str, card_json: &str) -> PyResult<String> {
    use apeireth_core::reconsolidate;
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

#[pymodule]
fn apeireth_py(_py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyEpisodeRepo>()?;
    m.add_function(wrap_pyfunction!(py_forget_sweep, m)?)?;
    m.add_function(wrap_pyfunction!(py_reconsolidate, m)?)?;
    Ok(())
}