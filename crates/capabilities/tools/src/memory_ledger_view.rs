//! 记忆账本视图层：`MemoryLedgerStats` 类型与它的 JSON 渲染。
//!
//! 从 `self_status.rs` 拆出（该文件在 arch_guard 尺寸基线内，只减不增；
//! 视图层独立成文件后，账本字段的增删不再被 `self_status.rs` 的行数卡住）。
//!
//! 语义边界不变：**永不回内容原文**，只有计数与调权数字；缺失计数显式 `null`
//! 并给原因，不编数。
//!
//! 另含教训对象（Lesson）的两个只读计数：`verified_lessons` / `candidate_lessons`
//! —— 与既有 `lessons`（反思沉淀计数）是**两个东西**，不互相冒充。

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::self_status::{SelfStatusSnapshot, DATA_PROBE_NOT_WIRED};

/// 记忆账本计数 (纯计数元数据; 缺失的计数显式 `None` → JSON `null`)。
///
/// **永不回内容原文**: 只有数字, 没有记忆文本。
///
/// (V4-c 起含浮点前后权重, 不再派生 `Eq`; 比较用 `PartialEq`。)
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct MemoryLedgerStats {
    /// 会话数 (episode 流里的不同会话计数)。
    pub sessions: Option<u64>,
    /// 记忆件数 (episode 计数)。
    pub memories: Option<u64>,
    /// 保护件数 (治理层标记为保护的记忆计数)。
    pub protected: Option<u64>,
    /// 教训数 (反思沉淀的教训计数)。
    pub lessons: Option<u64>,
    /// 待摄取数 (V4-b 保鲜回路: 摄取队列 `pending` 真值; 缺源 = `None` 显式 null)。
    pub pending_ingest: Option<u64>,
    /// 最近折入真值 (V4-b: epoch 毫秒; 尚未折入 = `None`, 显式 null 不编时间)。
    pub last_fold_at_ms: Option<i64>,
    /// 去重计数 (V4-b: 散列增量门跳过重复摄取的累计真值; 缺源 = `None`)。
    pub dedup_skipped: Option<u64>,
    /// 进化次数 (V4-c 检索维度自进化: 自进化账 `retrieval_tuning.jsonl` 的真实
    /// 调权移动行数; 缺源 = `None` 显式 null, 0 = 有账未调权)。
    pub retrieval_tuning_count: Option<u64>,
    /// 最近调权真值 (V4-c: 何时/因何证据/何维度/前后权重; 尚未调权 = `None`
    /// 显式 null, 不编移动)。
    pub last_retrieval_tuning: Option<RetrievalTuningMove>,
    /// 已验证教训数（教训对象 Lesson：`verified` 状态计数；与 `lessons`
    /// 的「反思沉淀计数」是两个东西，不互相冒充）。
    pub verified_lessons: Option<u64>,
    /// 候选教训数（尚未在新场景验证；**不注入上下文**，此处仅供如实自述）。
    pub candidate_lessons: Option<u64>,
    /// 局部缺失原因 (某计数探测失败时给出; 与字段级 null 配对)。
    pub reason: Option<String>,
}

/// 最近一次检索调权真值 (记忆节「最近调权」的一行; 只回数字与原因摘要)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RetrievalTuningMove {
    /// 何维度: `dimension:<id>` (层权重) 或 `entry:<memory_id>` (条目显著性)。
    pub target: String,
    /// 判定信号 (`positive` / `negative`)。
    pub signal: String,
    /// 调整前权重。
    pub previous: f64,
    /// 调整后权重。
    pub next: f64,
    /// 调权时刻 (epoch 毫秒)。
    pub at_epoch_ms: i64,
    /// 因何证据 (回执摘要, 与自进化账同一行同源)。
    pub evidence: String,
}

/// 渲染记忆账本节；未接线时逐字段显式 `null` + 原因（不编数）。
pub fn render_memory_ledger(snapshot: &SelfStatusSnapshot) -> Value {
    match &snapshot.memory_ledger {
        Some(stats) => json!({
            "sessions": stats.sessions,
            "memories": stats.memories,
            "protected": stats.protected,
            "lessons": stats.lessons,
            // V4-b 保鲜回路真值 (待摄取数 / 最近折入 / 去重计数); 缺源显式 null。
            "pending_ingest": stats.pending_ingest,
            "last_fold_at_ms": stats.last_fold_at_ms,
            "dedup_skipped": stats.dedup_skipped,
            // V4-c 检索维度自进化真值 (进化次数 / 最近调权); 尚未调权 = 显式 null 不编移动。
            "retrieval_tuning_count": stats.retrieval_tuning_count,
            "last_retrieval_tuning": stats.last_retrieval_tuning,
            // 教训对象（可验证）：已验证 / 候选分列 —— 候选不算「学到」。
            "verified_lessons": stats.verified_lessons,
            "candidate_lessons": stats.candidate_lessons,
            "reason": snapshot
                .memory_ledger_reason
                .clone()
                .or_else(|| stats.reason.clone()),
        }),
        None => json!({
            "sessions": null,
            "memories": null,
            "protected": null,
            "lessons": null,
            "pending_ingest": null,
            "last_fold_at_ms": null,
            "dedup_skipped": null,
            "retrieval_tuning_count": null,
            "last_retrieval_tuning": null,
            "verified_lessons": null,
            "candidate_lessons": null,
            "reason": snapshot
                .memory_ledger_reason
                .clone()
                .unwrap_or_else(|| DATA_PROBE_NOT_WIRED.to_string()),
        }),
    }
}
