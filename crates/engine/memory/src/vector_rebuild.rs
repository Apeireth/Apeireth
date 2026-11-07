//! `apeireth-memory::vector_rebuild` — 派生索引的**重建**与**换代检测**。
//!
//! # 为什么需要它
//!
//! D2 的语义索引是**派生的**（P1：原始证据永久保留，向量只是派生）
//! ⇒ 派生的东西**必须能删了重建**，否则它就成了「事实」⇒ 保真原则破了。
//!
//! # 但「重建后结果一致」有前提
//!
//! 生产 embedding 来自**外部 API** ⇒ ① 模型可能换版 ⇒ ② 浮点不保证逐位相同
//! ⇒ ⇒ **一致 = 同模型同输入下的一致**；模型变了 ⇒ 那是「**换代**」（D2 明确区分）
//! ⇒ ⇒ 本模块**显式记录建索引时的模型标识**，重建时**先比对**：
//! * **同模型** ⇒ 重建 ⇒ 逐条比对向量 ⇒ 报「一致 / 不一致 + 差多少」；
//! * **模型不同** ⇒ **不假装一致** ⇒ 报「**换代**」（可以重建，但那是换代，不是重建）。
//!
//! # 模型标识存在哪
//!
//! 存在**索引文件自己**的 `vec_meta` 表（键 `embed_model_id`）⇒ 索引搬到哪，
//! 标识就跟到哪 ⇒ 重建时能读回「上次是哪个模型建的」；索引被删 ⇒ 标识一起没了
//! ⇒ 下一次就是「首次建索引」（`NoBaseline`），**不会**拿旧模型冒充。
//!
//! # 能力口径
//!
//! **NOT IMPLEMENTED（新增未接线）**：库级实现（代码存在），但**生产 0 调用点**
//! ⇒ 不是 PRODUCTION WIRED，也不是 DEFAULT ENABLED。

use std::collections::BTreeMap;
use std::path::Path;
use std::time::Instant;

use rusqlite::{params, Connection, OpenFlags, OptionalExtension};

use crate::persistent_vector::PersistentVectorIndex;
use crate::scope::EmbeddingProvider;

/// 向量逐分量比对的**默认容差**（`compare_vectors` 的建议值）。
pub const DEFAULT_TOLERANCE: f64 = 1e-6;

/// `vec_meta` 里存模型标识的键名。
const META_MODEL_ID: &str = "embed_model_id";

/// 一条**源记录**（重建的输入）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceRecord {
    /// 记录 id。
    pub id: String,
    /// 原文（**要 embed 的文本**）。
    pub text: String,
}

/// 重建时用的**模型标识**（`EmbeddingProvider::model_id()` 的快照）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelStamp {
    /// 模型 id。
    pub model_id: String,
}

/// 重建**前**的索引状态（**从索引文件里读出来** ⇒ 用来比对）。
#[derive(Debug, Clone, PartialEq)]
pub struct IndexSnapshot {
    /// 建索引时用的模型标识。
    pub stamp: ModelStamp,
    /// 索引里的条目数。
    pub entries: usize,
}

/// 重建的**结果**。
#[derive(Debug, Clone, PartialEq)]
pub struct RebuildOutcome {
    /// 重建了多久（毫秒）—— **不参与判定**（非确定量 ⇒ 只做记录）。
    pub elapsed_ms: u64,
    /// 重建后的条目数。
    pub entries: usize,
    /// **判定**（见 [`RebuildVerdict`]）。
    pub verdict: RebuildVerdict,
    /// 人话说明。
    pub detail: String,
}

/// 重建的**判定**（**四档** ⇒ 不许含糊）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RebuildVerdict {
    /// **同模型 + 向量逐条一致** ⇒ 真正的「重建」（可安全删了重建）。
    ///
    /// ⚠️ 例外：旧索引**没有模型标识**时「同模型」不可验证 ⇒ 此时结论**只由向量逐条一致支撑**
    /// （向量一致是最强的证据）⇒ `detail` 会写明「模型标识未知」。
    RebuiltIdentical,
    /// **同模型 + 向量有差异** ⇒ 重建了，但**不一致**（⇒ 要查：embed 非确定？数据变过？）。
    ///
    /// ⚠️ 模型标识未知时的差异 ⇒ 无法区分「换代」与「embed 非确定」⇒ `detail` 会写明。
    RebuiltDifferent,
    /// **模型不同** ⇒ 这是「**换代**」，不是「重建」⇒ **不假装一致**。
    ModelChanged,
    /// **没有旧索引可比**（首次建索引）⇒ 无从比对。
    NoBaseline,
}

/// 读出索引的快照（模型标识 + 条目数）。
///
/// ⚠️ 若索引文件**不存在** ⇒ 返回 `Ok(None)`（**不是错误** ⇒ 首次建索引是正常路径）。
///
/// ⚠️ 索引存在但**没有模型标识**（别的路径直接建过索引）⇒ 模型标识留空
/// ⇒ 「模型身份未知」**不等于**「模型变了」⇒ 重建时不会误报 `ModelChanged`。
pub fn read_snapshot(index_path: &std::path::Path) -> Result<Option<IndexSnapshot>, String> {
    if !index_path.exists() {
        return Ok(None);
    }
    let conn = open_existing(index_path)?;
    // 不是向量索引文件（缺 vec_items 表）⇒ 无基线可比 ⇒ Ok(None)（不是错误）。
    if !table_exists(&conn, "vec_items")? {
        return Ok(None);
    }
    let entries: i64 = conn
        .query_row("SELECT COUNT(*) FROM vec_items", [], |row| row.get(0))
        .map_err(|e| format!("统计索引条目失败 ({}): {e}", index_path.display()))?;
    let model_id = if table_exists(&conn, "vec_meta")? {
        read_meta(&conn, META_MODEL_ID)?
    } else {
        None
    };
    Ok(Some(IndexSnapshot {
        stamp: ModelStamp {
            model_id: model_id.unwrap_or_default(),
        },
        entries: entries as usize,
    }))
}

/// 把源记录**重建**进向量索引。
///
/// # 口径
///
/// * 对每条源记录调 `embedder.embed(&text)` ⇒ 向量 ⇒ `upsert` 进索引；
/// * **模型标识**：用 `embedder.model_id()`（**快照**）⇒ 与旧快照比对 ⇒ 判定换代；
/// * **确定性**：给同一个 `embedder` 与同一批源记录 ⇒ 重建结果一致
///   （⚠️ 外部 embedder **不保证** ⇒ 那是 `RebuiltDifferent` 要报告的情形）；
/// * **不许 panic**：embed 失败 ⇒ 返回 `Err`（**如实**，不跳过）。
///
/// # 写入次序（为什么是这个次序）
///
/// ① 先读旧快照 + **旧向量**（重建会覆盖旧向量 ⇒ 不先读出来就没法比对）；
/// ② 再 embed（失败 ⇒ `Err`，此时**索引一个字节都没动** ⇒ 不留半拉索引）；
/// ③ 再清空 + 重写（派生索引可删 ⇒ 重建 = 清空后重新派生）；
/// ④ 再落**模型标识**（换代判定的唯一依据）；
/// ⑤ 最后**回读落盘结果**再比对（比的是**真正写进磁盘的**向量，不是内存里的）。
pub async fn rebuild_vector_index(
    index_path: &std::path::Path,
    records: &[SourceRecord],
    embedder: &dyn EmbeddingProvider,
) -> Result<RebuildOutcome, String> {
    let started = Instant::now();

    // ① 旧基线: 快照 + 旧向量（重建会覆盖 ⇒ 必须先读出来）。
    let baseline = read_snapshot(index_path)?;
    let old_vectors = match &baseline {
        Some(_) => read_index_vectors(index_path)?,
        None => BTreeMap::new(),
    };

    // ② 新向量: embed 失败 ⇒ Err（如实，不跳过）；失败发生在任何写入之前。
    let mut new_vectors: Vec<(String, Vec<f32>)> = Vec::with_capacity(records.len());
    for record in records {
        if record.id.is_empty() {
            return Err("源记录 id 不能为空".to_string());
        }
        let vector = embedder
            .embed(&record.text)
            .await
            .map_err(|e| format!("embed 失败 (id={}): {e}", record.id))?;
        if vector.is_empty() {
            return Err(format!("embed 返回空向量 (id={})", record.id));
        }
        new_vectors.push((record.id.clone(), vector));
    }
    // 维度必须一致（索引只存一种维度）⇒ 先验后写：不合格就别碰索引。
    if let Some((_, first)) = new_vectors.first() {
        let dim = first.len();
        if new_vectors.iter().any(|(_, v)| v.len() != dim) {
            return Err(format!("源记录向量维度不一致（期望 {dim}）"));
        }
    }

    // ③ 落盘: 派生索引可删 ⇒ 重建 = 清空后重新派生。
    {
        let mut index = PersistentVectorIndex::open(index_path)
            .map_err(|e| format!("打开向量索引失败 ({}): {e}", index_path.display()))?;
        index
            .clear()
            .map_err(|e| format!("清空向量索引失败 ({}): {e}", index_path.display()))?;
        if let Some((_, first)) = new_vectors.first() {
            index
                .set_dimension(first.len())
                .map_err(|e| format!("设置向量维度失败 ({}): {e}", index_path.display()))?;
            let items: Vec<(String, Vec<f32>, Option<serde_json::Value>)> = new_vectors
                .iter()
                .map(|(id, vector)| (id.clone(), vector.clone(), None))
                .collect();
            index
                .upsert_batch(&items)
                .map_err(|e| format!("写入向量失败 ({}): {e}", index_path.display()))?;
        }
    } // drop ⇒ 提交

    // ④ 记录模型标识（换代判定的唯一依据 = `model_id` 快照）。
    write_model_id(index_path, embedder.model_id())?;

    // ⑤ 回读落盘结果（比对的是真正落盘的向量）。
    let rebuilt = read_index_vectors(index_path)?;
    let entries = rebuilt.len();
    let new_model = embedder.model_id().to_string();
    let old_model = baseline
        .as_ref()
        .map(|s| s.stamp.model_id.clone())
        .unwrap_or_default();
    // 「模型身份未知」（任一侧为空）**不等于**「模型变了」⇒ 只有两侧都非空才判换代。
    let model_known = !old_model.is_empty() && !new_model.is_empty();
    let model_changed = model_known && old_model != new_model;
    let diff = compare_maps(&old_vectors, &rebuilt, DEFAULT_TOLERANCE);

    let (verdict, detail) = match &baseline {
        None => (
            RebuildVerdict::NoBaseline,
            format!("首次建索引: 无旧索引可比 (模型={new_model}, 条目={entries})"),
        ),
        Some(_) if model_changed => (
            RebuildVerdict::ModelChanged,
            format!(
                "换代检测: 旧模型={old_model} ≠ 新模型={new_model} ⇒ 这是换代不是重建; \
                 向量差异 {} 条 (未声称一致)",
                diff.different + diff.only_one_side
            ),
        ),
        Some(_) if !model_known => {
            // 模型身份未知 ⇒ 不能声称「同模型」，也不能声称「换代」⇒ 如实说明。
            if diff.different == 0 && diff.only_one_side == 0 {
                (
                    RebuildVerdict::RebuiltIdentical,
                    format!(
                        "向量逐条一致 (旧模型标识未知=「{old_model}」, 新模型={new_model}): \
                         {} 条全部一致 (容差 {DEFAULT_TOLERANCE})",
                        diff.identical
                    ),
                )
            } else {
                (
                    RebuildVerdict::RebuiltDifferent,
                    format!(
                        "向量逐条不一致且模型标识未知 (旧=「{old_model}」, 新={new_model}): \
                         一致 {} 条 / 不一致 {} 条 / 只在一边 {} 条; 最大分量差 {:.3e} \
                         ⇒ 无法区分「换代」与「embed 非确定」",
                        diff.identical, diff.different, diff.only_one_side, diff.max_abs_diff
                    ),
                )
            }
        }
        Some(_) => {
            if diff.different == 0 && diff.only_one_side == 0 {
                (
                    RebuildVerdict::RebuiltIdentical,
                    format!(
                        "同模型重建一致: 模型={new_model}, {} 条逐条比对全部一致 (容差 {DEFAULT_TOLERANCE})",
                        diff.identical
                    ),
                )
            } else {
                (
                    RebuildVerdict::RebuiltDifferent,
                    format!(
                        "同模型重建不一致: 模型={new_model}, 一致 {} 条 / 不一致 {} 条 / \
                         只在一边 {} 条; 最大分量差 {:.3e}",
                        diff.identical, diff.different, diff.only_one_side, diff.max_abs_diff
                    ),
                )
            }
        }
    };

    Ok(RebuildOutcome {
        elapsed_ms: started.elapsed().as_millis() as u64,
        entries,
        verdict,
        detail,
    })
}

/// **逐条比对**两个向量索引的向量（**在容差内**才算一致）。
///
/// # 口径
///
/// * 逐条比 id 与向量；
/// * **容差** = `tolerance`（默认给 `1e-6`）⇒ 超过 ⇒ 该条**不一致**；
/// * 返回：`(一致的条数, 不一致的条数, 只在一边存在的条数)`。
///
/// ⚠️ 索引文件**不存在** ⇒ `Err`（不能拿「空索引」冒充「没有差异」）。
pub fn compare_vectors(
    left_path: &std::path::Path,
    right_path: &std::path::Path,
    tolerance: f64,
) -> Result<(usize, usize, usize), String> {
    let left = read_index_vectors(left_path)?;
    let right = read_index_vectors(right_path)?;
    let diff = compare_maps(&left, &right, tolerance);
    Ok((diff.identical, diff.different, diff.only_one_side))
}

/// 两个向量集合的逐条比对结果。
#[derive(Debug, Clone, Copy, PartialEq)]
struct VectorDiff {
    /// 容差内一致的条数。
    identical: usize,
    /// 超出容差（或维度不同）的条数。
    different: usize,
    /// 只在一边存在的条数（id 集合不同）。
    only_one_side: usize,
    /// 观测到的最大分量绝对差。
    max_abs_diff: f64,
}

/// 逐条比对两个 `id -> 向量` 集合。
fn compare_maps(
    left: &BTreeMap<String, Vec<f32>>,
    right: &BTreeMap<String, Vec<f32>>,
    tolerance: f64,
) -> VectorDiff {
    let mut diff = VectorDiff {
        identical: 0,
        different: 0,
        only_one_side: 0,
        max_abs_diff: 0.0,
    };
    for (id, left_vector) in left {
        match right.get(id) {
            None => diff.only_one_side += 1,
            Some(right_vector) => {
                let delta = max_component_diff(left_vector, right_vector);
                diff.max_abs_diff = diff.max_abs_diff.max(delta);
                if delta <= tolerance {
                    diff.identical += 1;
                } else {
                    diff.different += 1;
                }
            }
        }
    }
    for id in right.keys() {
        if !left.contains_key(id) {
            diff.only_one_side += 1;
        }
    }
    diff
}

/// 两个向量的最大分量绝对差（维度不同 ⇒ 无穷大 ⇒ 必然不一致）。
fn max_component_diff(left: &[f32], right: &[f32]) -> f64 {
    if left.len() != right.len() {
        return f64::INFINITY;
    }
    let mut max = 0.0f64;
    for (a, b) in left.iter().zip(right.iter()) {
        let delta = (f64::from(*a) - f64::from(*b)).abs();
        // NaN 永远算不一致（不许 NaN 混进「一致」）。
        if delta.is_nan() {
            return f64::INFINITY;
        }
        if delta > max {
            max = delta;
        }
    }
    max
}

/// 打开一个**已存在**的索引文件（不 CREATE ⇒ 免得把「文件不存在」变成「空索引」）。
///
/// ⚠️ 用 `SQLITE_OPEN_READ_WRITE` 而不是 READ_ONLY：索引库是 WAL 模式，
/// 只读连接需要能写 `-shm` ⇒ 在只读介质上会直接打不开。
fn open_existing(path: &Path) -> Result<Connection, String> {
    if !path.exists() {
        return Err(format!("索引文件不存在: {}", path.display()));
    }
    Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_WRITE)
        .map_err(|e| format!("打开索引文件失败 ({}): {e}", path.display()))
}

/// 表是否存在。
fn table_exists(conn: &Connection, name: &str) -> Result<bool, String> {
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?1",
            params![name],
            |row| row.get(0),
        )
        .map_err(|e| format!("读取 sqlite_master 失败: {e}"))?;
    Ok(count > 0)
}

/// 读 `vec_meta` 的一个键。
fn read_meta(conn: &Connection, key: &str) -> Result<Option<String>, String> {
    conn.query_row(
        "SELECT value FROM vec_meta WHERE key=?1",
        params![key],
        |row| row.get(0),
    )
    .optional()
    .map_err(|e| format!("读取 vec_meta[{key}] 失败: {e}"))
}

/// 把模型标识写进索引文件自己的 `vec_meta`（⇒ 索引搬到哪，标识就跟到哪）。
fn write_model_id(path: &Path, model_id: &str) -> Result<(), String> {
    let conn = open_existing(path)?;
    if !table_exists(&conn, "vec_meta")? {
        return Err(format!(
            "不是向量索引文件 (缺 vec_meta 表): {}",
            path.display()
        ));
    }
    conn.execute(
        "INSERT OR REPLACE INTO vec_meta(key, value) VALUES(?1, ?2)",
        params![META_MODEL_ID, model_id],
    )
    .map_err(|e| format!("写入模型标识失败 ({}): {e}", path.display()))?;
    Ok(())
}

/// 读出索引里的全部向量（`id -> 向量`）。
fn read_index_vectors(path: &Path) -> Result<BTreeMap<String, Vec<f32>>, String> {
    let conn = open_existing(path)?;
    if !table_exists(&conn, "vec_items")? {
        return Err(format!(
            "不是向量索引文件 (缺 vec_items 表): {}",
            path.display()
        ));
    }
    let mut stmt = conn
        .prepare("SELECT id, vec FROM vec_items")
        .map_err(|e| format!("准备读取向量失败 ({}): {e}", path.display()))?;
    let mut rows = stmt
        .query([])
        .map_err(|e| format!("读取向量失败 ({}): {e}", path.display()))?;
    let mut vectors = BTreeMap::new();
    while let Some(row) = rows
        .next()
        .map_err(|e| format!("读取向量行失败 ({}): {e}", path.display()))?
    {
        let id: String = row
            .get(0)
            .map_err(|e| format!("读取向量 id 失败 ({}): {e}", path.display()))?;
        let blob: Vec<u8> = row
            .get(1)
            .map_err(|e| format!("读取向量内容失败 ({}): {e}", path.display()))?;
        vectors.insert(id, unpack_vec(&blob));
    }
    Ok(vectors)
}

/// 解包向量 BLOB（小端 f32 序列；与 `persistent_vector` 的打包格式一致）。
fn unpack_vec(blob: &[u8]) -> Vec<f32> {
    blob.chunks_exact(4)
        .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
        .collect()
}
