//! IM 提案卡推送账 (`proposal-pushes.jsonl`, 与提案账同数据目录): 推送幂等 +
//! 失败留账的真值账。
//!
//! 背景 (自主度提案面接线批): 提案入账后, 人审卡要真的推到 IM 渠道
//! (「建议你亲自确认」行随卡到手机)。推送是**外联动作**, 会失败; 本账就是
//! 推送这件事的诚实痕迹:
//!
//! - **推送幂等键 = 提案 id**: [`ProposalPushLedger::pushed_ids`] 是真值幂等集,
//!   已成功推送的提案不再推 (同提案不重复推送);
//! - **失败留账**: 推送失败写 `failed` 行 (带真因) 待重试 —— IM 失败不丢提案,
//!   提案本体永远在提案账 (`proposals.jsonl`);
//! - **append-only**: 逐行一个 [`StoredDoc`] 信封, 追加写 + 落盘, 不重写不删行
//!   (与提案账同一姿势); 坏行读作不存在 (跳过并计入 `skipped`), 不砖化整本。
//!
//! 本账只记**推送痕迹**, 不记提案语义 —— 草稿/决定的语义全在
//! [`crate::proposal`] (一提案一决定、append-only 不改史), 本面零第二套。

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use apeireth_core::stored_doc::{scan_records, DocCompat, RecordScan, StoredDoc};
use serde::{Deserialize, Serialize};

use crate::proposal::PROPOSAL_RECENT_LIMIT;
use crate::self_status::ProposalPushStatusEntry;

/// 推送账文件 (数据目录落点: `<data_dir>/proposal-pushes.jsonl`)。
pub const PROPOSAL_PUSH_LEDGER_FILE: &str = "proposal-pushes.jsonl";

/// 推送账 StoredDoc 的身份戳。
pub const PROPOSAL_PUSH_DOC_NAME: &str = "proposal-push";

/// 推送账 StoredDoc 的格式版本戳。
pub const PROPOSAL_PUSH_DOC_VERSION: u32 = 1;

/// 一次推送的结果 (两态闭合词表: `pushed` / `failed`, 无第三套词表)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProposalPushOutcome {
    /// 已送达 (幂等键从此生效: 同提案不再推)。
    Pushed,
    /// 推送失败 (留账待重试; 提案本体仍在提案账, 不丢)。
    Failed,
}

impl ProposalPushOutcome {
    /// 稳定标签 (`pushed` / `failed`)。
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pushed => "pushed",
            Self::Failed => "failed",
        }
    }

    /// 闭合词表解析 (词表外 = `None`, 调用方照实报未知)。
    pub fn parse(text: &str) -> Option<Self> {
        match text.trim() {
            "pushed" => Some(Self::Pushed),
            "failed" => Some(Self::Failed),
            _ => None,
        }
    }
}

/// 推送账的一行 (append-only 的行主体: 一次推送尝试的痕迹)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProposalPushRecord {
    /// 被推送的提案 id (**推送幂等键**)。
    pub proposal_id: String,
    /// 推送到的 IM 渠道 id。
    pub channel_id: String,
    /// 推送到的 IM 会话 id。
    pub conversation_id: String,
    /// 推送结果 ([`ProposalPushOutcome`])。
    pub outcome: ProposalPushOutcome,
    /// 对端回执引用 (`pushed` 时在场; 对端未给 = 本地接受标记)。
    pub message_ref: Option<String>,
    /// 失败真因 (`failed` 时在场, 如实)。
    pub reason: Option<String>,
    /// 推送时刻 (epoch 毫秒)。
    pub at_ms: i64,
}

/// 推送账的自述列真值 (自述提案节的「已推送数 / 最近推送」同源)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProposalPushStats {
    /// 已推送数 (成功送达的**去重提案 id** 计数; 幂等键 = 提案 id)。
    pub pushed: u64,
    /// 最近推送 (有界 [`PROPOSAL_RECENT_LIMIT`] 条, 新在前; 含 `failed`
    /// 留账行 —— 失败也是真值, 不藏)。
    pub recent: Vec<ProposalPushStatusEntry>,
    /// 备注 (坏行跳过如实注; 干净时 `None`)。
    pub note: Option<String>,
}

/// 提案卡推送账 (数据目录 `proposal-pushes.jsonl`)。
///
/// **append-only**: 逐行一个 [`StoredDoc`] 信封, 追加写 + 落盘, 不重写、不删行;
/// 坏行读作不存在 (跳过并计入 `skipped`), 不砖化整本。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProposalPushLedger {
    path: PathBuf,
}

impl ProposalPushLedger {
    /// 数据目录落点 (`<data_dir>/proposal-pushes.jsonl`)。
    pub fn under_data_dir(data_dir: impl AsRef<Path>) -> Self {
        Self {
            path: data_dir.as_ref().join(PROPOSAL_PUSH_LEDGER_FILE),
        }
    }

    /// 显式路径落点。
    pub fn at(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// 账本路径。
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// 追加一行 (JSONL 单行 + 落盘; append-only, 不重写不删行)。
    pub fn append(&self, record: &ProposalPushRecord) -> Result<(), String> {
        use std::io::Write;
        if let Some(parent) = self.path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)
                    .map_err(|error| format!("推送账目录创建失败 {parent:?}: {error}"))?;
            }
        }
        let doc = StoredDoc::new(
            PROPOSAL_PUSH_DOC_NAME,
            PROPOSAL_PUSH_DOC_VERSION,
            vec![PROPOSAL_PUSH_DOC_VERSION],
            record,
        );
        let line =
            serde_json::to_vec(&doc).map_err(|error| format!("推送账序列化失败: {error}"))?;
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .map_err(|error| format!("推送账打开失败 {}: {error}", self.path.display()))?;
        file.write_all(&line)
            .and_then(|()| file.write_all(b"\n"))
            .and_then(|()| file.sync_all())
            .map_err(|error| format!("推送账写入失败 {}: {error}", self.path.display()))
    }

    /// 读回整本 (逐记录语义: 坏行读作不存在并计入 `skipped`)。
    pub fn scan(&self) -> Result<RecordScan<ProposalPushRecord>, String> {
        scan_records::<ProposalPushRecord>(&self.path, &push_doc_compat())
            .map_err(|error| format!("推送账读取失败 {}: {error}", self.path.display()))
    }

    /// 幂等键集合 (**推送幂等键 = 提案 id**): 已成功推送 (`pushed`) 的提案 id。
    ///
    /// `failed` 行不入集合 —— 失败留账待重试, 重试成功才进幂等集。
    pub fn pushed_ids(&self) -> Result<BTreeSet<String>, String> {
        let scan = self.scan()?;
        Ok(scan
            .records
            .iter()
            .filter(|record| record.body.outcome == ProposalPushOutcome::Pushed)
            .map(|record| record.body.proposal_id.clone())
            .collect())
    }

    /// 自述提案节的推送列真值 (已推送数 + 最近推送, 真值如实)。
    pub fn status(&self) -> Result<ProposalPushStats, String> {
        let scan = self.scan()?;
        let mut note: Option<String> = None;
        if scan.skipped > 0 {
            note = Some(format!(
                "推送账 {} 行坏行读作不存在 (如实跳过, 不砖化整本)",
                scan.skipped
            ));
        }
        let mut pushed_ids: BTreeSet<String> = BTreeSet::new();
        for record in &scan.records {
            if record.body.outcome == ProposalPushOutcome::Pushed {
                pushed_ids.insert(record.body.proposal_id.clone());
            }
        }
        let recent: Vec<ProposalPushStatusEntry> = scan
            .records
            .iter()
            .rev()
            .take(PROPOSAL_RECENT_LIMIT)
            .map(|record| ProposalPushStatusEntry {
                proposal_id: record.body.proposal_id.clone(),
                channel_id: record.body.channel_id.clone(),
                at_ms: record.body.at_ms,
                outcome: record.body.outcome.as_str().to_string(),
            })
            .collect();
        Ok(ProposalPushStats {
            pushed: pushed_ids.len() as u64,
            recent,
            note,
        })
    }
}

/// 推送账读端契约 (仅当前格式版本可读)。
fn push_doc_compat() -> DocCompat {
    DocCompat::exact(PROPOSAL_PUSH_DOC_NAME, PROPOSAL_PUSH_DOC_VERSION)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(proposal_id: &str, outcome: ProposalPushOutcome) -> ProposalPushRecord {
        ProposalPushRecord {
            proposal_id: proposal_id.to_string(),
            channel_id: "primary".to_string(),
            conversation_id: "oc_demo_chat".to_string(),
            outcome,
            message_ref: matches!(outcome, ProposalPushOutcome::Pushed)
                .then(|| "m-out".to_string()),
            reason: matches!(outcome, ProposalPushOutcome::Failed)
                .then(|| "delivery failed: 503".to_string()),
            at_ms: 1_700_000_000_000,
        }
    }

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "apeireth-proposal-push-{tag}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create test dir");
        dir
    }

    #[test]
    fn the_push_ledger_is_append_only_and_reads_back_in_order() {
        let dir = temp_dir("append");
        let ledger = ProposalPushLedger::under_data_dir(&dir);
        ledger
            .append(&record("prop-1", ProposalPushOutcome::Pushed))
            .unwrap();
        let prefix = std::fs::read(ledger.path()).unwrap();
        ledger
            .append(&record("prop-2", ProposalPushOutcome::Failed))
            .unwrap();
        let whole = std::fs::read(ledger.path()).unwrap();
        assert!(
            whole.starts_with(&prefix),
            "append-only: 旧字节前缀逐字节不变"
        );
        let scan = ledger.scan().unwrap();
        assert_eq!(scan.records.len(), 2);
        assert_eq!(scan.records[0].body.proposal_id, "prop-1");
        assert_eq!(scan.records[1].body.outcome, ProposalPushOutcome::Failed);
        assert_eq!(scan.skipped, 0);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 幂等键 = 提案 id: 只有 `pushed` 行入幂等集, `failed` 行留账待重试。
    #[test]
    fn pushed_ids_counts_only_delivered_proposals_for_idempotency() {
        let dir = temp_dir("idempotent");
        let ledger = ProposalPushLedger::under_data_dir(&dir);
        ledger
            .append(&record("prop-1", ProposalPushOutcome::Failed))
            .unwrap();
        assert!(
            ledger.pushed_ids().unwrap().is_empty(),
            "失败留账不算已推送 (要重试)"
        );
        ledger
            .append(&record("prop-1", ProposalPushOutcome::Pushed))
            .unwrap();
        // 同 id 再来一行 pushed 也不重复计数 (幂等键 = 提案 id)。
        ledger
            .append(&record("prop-1", ProposalPushOutcome::Pushed))
            .unwrap();
        ledger
            .append(&record("prop-2", ProposalPushOutcome::Pushed))
            .unwrap();
        let ids = ledger.pushed_ids().unwrap();
        assert_eq!(ids.len(), 2, "去重提案 id 计数");
        assert!(ids.contains("prop-1") && ids.contains("prop-2"));
        let stats = ledger.status().unwrap();
        assert_eq!(stats.pushed, 2, "已推送数 = 去重提案 id 计数");
        assert_eq!(stats.recent.len(), 4, "最近推送含失败留账行 (如实)");
        assert_eq!(stats.recent[0].proposal_id, "prop-2", "新在前");
        assert_eq!(stats.recent[0].outcome, "pushed");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 坏行读作不存在 (跳过并计入 skipped), 不砖化整本。
    #[test]
    fn bad_rows_read_as_absent_and_are_counted_honestly() {
        let dir = temp_dir("bad-rows");
        let ledger = ProposalPushLedger::under_data_dir(&dir);
        ledger
            .append(&record("prop-1", ProposalPushOutcome::Pushed))
            .unwrap();
        use std::io::Write as _;
        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .open(ledger.path())
            .unwrap();
        file.write_all(b"{ not json\n").unwrap();
        drop(file);
        let scan = ledger.scan().unwrap();
        assert_eq!(scan.records.len(), 1);
        assert_eq!(scan.skipped, 1, "坏行如实计数");
        let stats = ledger.status().unwrap();
        assert_eq!(stats.pushed, 1);
        assert!(
            stats.note.as_deref().unwrap_or_default().contains("坏行"),
            "备注如实注坏行"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_outcome_vocabulary_is_closed_and_named_honestly() {
        assert_eq!(ProposalPushOutcome::Pushed.as_str(), "pushed");
        assert_eq!(ProposalPushOutcome::Failed.as_str(), "failed");
        assert_eq!(
            ProposalPushOutcome::parse("pushed"),
            Some(ProposalPushOutcome::Pushed)
        );
        assert_eq!(ProposalPushOutcome::parse("retried"), None, "词表闭合");
    }
}
