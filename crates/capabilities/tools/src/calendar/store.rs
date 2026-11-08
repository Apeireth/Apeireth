//! 日程账存储面: 数据模型 + 字段校验 + StoredDoc 账本 (append-only)。
//!
//! # 存储口径 (与既有族同姿势)
//!
//! - **逐行一个 [`StoredDoc`] 信封**: 条目账与审计账都是 append-only 的 JSONL,
//!   行的追加复用仓内**唯一**的追加手续
//!   ([`crate::proposal::append_jsonl_row`]: 行体与行尾换行同在一次写 ⇒ 并发
//!   追加不挤行; 末行缺换行的撕裂半行先补换行封口 ⇒ 不会吞掉下一行)。
//! - **坏行按不存在处理**: 单行坏 JSON / 身份不符 / 版本不符 ⇒ **跳过并计数**
//!   ([`apeireth_core::stored_doc::scan_records`] 的 `skipped` 口径), **不砖化
//!   整本**; 好行照读。计数由上层如实上报到结果帧 ⇒ 不静默吞错。
//! - **状态折算**: 条目当前态 = 账上按顺序折叠的结果 (`created` → `updated` →
//!   `cancelled`); 取消是**终态** (取消过的条目不可改期)。
//!
//! # 边界 (拒绝即帧 · 宁拒不截)
//!
//! 字段校验与上限校验都在本层做 (装载与写出同口径): 空标题 / 倒挂或零时长 /
//! 超长时段 / 超长标题 / 超长备注 / 非法 id / 重复 id / 条数上限 / 账本行数上限
//! ⇒ 全部**拒绝**, 不截断、不静默丢条目。

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use apeireth_core::stored_doc::{scan_records, DocCompat, RecordScan, StoredDoc};
use serde::{Deserialize, Serialize};

use super::{
    ACTION_CANCEL, ACTION_CREATE, ACTION_UPDATE, CALENDAR_AUDIT_DOC_NAME,
    CALENDAR_AUDIT_DOC_VERSION, CALENDAR_AUDIT_FILE, CALENDAR_DIR_NAME,
    CALENDAR_ENTRY_ID_MAX_BYTES, CALENDAR_LEDGER_DOC_NAME, CALENDAR_LEDGER_DOC_VERSION,
    CALENDAR_LEDGER_FILE, CALENDAR_MAX_ENTRIES, CALENDAR_MAX_LEDGER_ROWS, CALENDAR_MAX_SPAN_MS,
    CALENDAR_NOTE_MAX_BYTES, CALENDAR_TITLE_MAX_BYTES,
};

// ---------------------------------------------------------------------------
// 数据模型
// ---------------------------------------------------------------------------

/// 日程条目状态 (闭合词表: 已排期 / 已取消)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CalendarStatus {
    /// 已排期 (有效条目)。
    Scheduled,
    /// 已取消 (终态: 保留行, 默认列表不显示, 不可再改期)。
    Cancelled,
}

impl CalendarStatus {
    /// 稳定标签 (`scheduled` / `cancelled`)。
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Scheduled => "scheduled",
            Self::Cancelled => "cancelled",
        }
    }
}

/// 一条日程条目 (主人自己的一个安排)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CalendarEntry {
    /// 条目 id (闭合字符集 `[A-Za-z0-9._-]`)。
    pub id: String,
    /// 标题 (非空; 超长即拒)。
    pub title: String,
    /// 开始时刻 (epoch 毫秒)。
    pub start_ms: i64,
    /// 结束时刻 (epoch 毫秒; 必须晚于开始)。
    pub end_ms: i64,
    /// 备注 (可空; 超长即拒)。
    #[serde(default)]
    pub note: Option<String>,
    /// 状态 (已排期 / 已取消)。
    pub status: CalendarStatus,
    /// 入账时刻 (epoch 毫秒)。
    pub created_at_ms: i64,
    /// 最近一次改动时刻 (epoch 毫秒)。
    pub updated_at_ms: i64,
}

impl CalendarEntry {
    /// 审计备注文本 (如实: 标题 + 起止; 本面无凭据面 ⇒ 不涉敏感面)。
    pub fn note_text(&self) -> String {
        format!("{} / {}..{}", self.title, self.start_ms, self.end_ms)
    }
}

/// 日程账的一行 (append-only 的行主体)。
///
/// - `created`: 新建条目 (全量快照);
/// - `updated`: 改期 / 改标题 / 改备注后的**全量快照**;
/// - `cancelled`: 取消标记 (点名条目 id + 时刻)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum CalendarLedgerEntry {
    /// 新建条目行。
    Created(CalendarEntry),
    /// 改期 / 改内容行 (全量快照)。
    Updated(CalendarEntry),
    /// 取消行 (取消是终态)。
    Cancelled {
        /// 被取消的条目 id。
        entry_id: String,
        /// 取消时刻 (epoch 毫秒)。
        at_ms: i64,
    },
}

/// 写动作审计行 (append-only 的行主体)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CalendarAuditRow {
    /// 写动作 (`create` / `update` / `cancel`)。
    pub action: String,
    /// 目标条目 id。
    pub entry_id: String,
    /// 结果标签 (恒 `ok`: 失败路径在落账前就拒了, 不落审计行)。
    pub outcome: String,
    /// 人读备注 (如实: 改期前后 / 取消时刻)。
    pub note: String,
    /// 时刻 (epoch 毫秒)。
    pub at_ms: i64,
}

// ---------------------------------------------------------------------------
// 字段校验 (拒绝即帧 · 宁拒不截)
// ---------------------------------------------------------------------------

/// 条目 id 校验: 闭合字符集 `[A-Za-z0-9._-]`, 1..=[`CALENDAR_ENTRY_ID_MAX_BYTES`] 字节。
pub fn validate_calendar_entry_id(raw: &str) -> Result<(), String> {
    if raw.is_empty() {
        return Err("条目 id 不得为空".to_string());
    }
    if raw.len() > CALENDAR_ENTRY_ID_MAX_BYTES {
        return Err(format!(
            "条目 id 是 {} 字节; 上限 {CALENDAR_ENTRY_ID_MAX_BYTES}",
            raw.len()
        ));
    }
    if !raw
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
    {
        return Err(
            "条目 id 只允许 [A-Za-z0-9._-] (它会进账本行与审计行, 不接受空白与控制符)".to_string(),
        );
    }
    Ok(())
}

/// 标题校验: 去空白后非空, 字节数 ≤ [`CALENDAR_TITLE_MAX_BYTES`] (**不截断**)。
pub fn validate_calendar_title(raw: &str) -> Result<(), String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err("标题不得为空 (去空白后仍为空)".to_string());
    }
    if raw.len() > CALENDAR_TITLE_MAX_BYTES {
        return Err(format!(
            "标题是 {} 字节; 上限 {CALENDAR_TITLE_MAX_BYTES} (超长即拒, 不截断)",
            raw.len()
        ));
    }
    Ok(())
}

/// 备注校验: 字节数 ≤ [`CALENDAR_NOTE_MAX_BYTES`] (**不截断**); 空白备注归一为无。
pub fn validate_calendar_note(raw: &str) -> Result<Option<String>, String> {
    if raw.len() > CALENDAR_NOTE_MAX_BYTES {
        return Err(format!(
            "备注是 {} 字节; 上限 {CALENDAR_NOTE_MAX_BYTES} (超长即拒, 不截断)",
            raw.len()
        ));
    }
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    Ok(Some(trimmed.to_string()))
}

/// 起止时间校验: 时刻为正、结束晚于开始 (零时长 / 倒挂即拒)、跨度不超上限。
pub fn validate_calendar_span(start_ms: i64, end_ms: i64) -> Result<(), String> {
    if start_ms <= 0 || end_ms <= 0 {
        return Err(format!(
            "起止时刻必须为正 (epoch 毫秒); 收到 start={start_ms} end={end_ms}"
        ));
    }
    if end_ms <= start_ms {
        return Err(format!(
            "结束时刻必须晚于开始时刻 (零时长 / 倒挂即拒); 收到 start={start_ms} end={end_ms}"
        ));
    }
    if end_ms - start_ms > CALENDAR_MAX_SPAN_MS {
        return Err(format!(
            "日程跨度 {} 毫秒超上限 {CALENDAR_MAX_SPAN_MS} (超长时段即拒)",
            end_ms - start_ms
        ));
    }
    Ok(())
}

/// 当前时刻 (epoch 毫秒); 时钟回拨不编造, 如实取系统读数。
pub fn now_epoch_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|value| value.as_millis() as i64)
        .unwrap_or(0)
}

/// 进程内条目序号 (身份唯一性用, 非安全边界)。
static CALENDAR_ID_SEQ: AtomicU64 = AtomicU64::new(0);

/// 生成下一个条目身份 (`cal-<时刻毫秒>-<进程内序号>`)。
pub fn next_calendar_entry_id(at_ms: i64) -> String {
    let seq = CALENDAR_ID_SEQ.fetch_add(1, Ordering::Relaxed);
    format!("cal-{at_ms}-{seq}")
}

// ---------------------------------------------------------------------------
// 账本
// ---------------------------------------------------------------------------

/// 日程账读取 / 写入的失败 (闭合词表: 读失败 / 拒收 / 不存在 / 重复 / 已取消 /
/// 满 / 超上限 / 审计落账失败)。
#[derive(Debug, thiserror::Error)]
pub enum CalendarError {
    /// 账本读取失败 (IO 等)。
    #[error("日程账读取失败: {0}")]
    Ledger(String),
    /// 条目被拒收 (字段边界不过)。
    #[error("日程条目被拒收: {0}")]
    Refused(String),
    /// 条目不在账上。
    #[error("日程条目不存在: {0}")]
    UnknownEntry(String),
    /// 条目 id 重复。
    #[error("日程条目 id 重复: {0}")]
    DuplicateId(String),
    /// 条目已取消 (取消是终态, 不可再改期 / 再取消)。
    #[error("日程条目已取消: {0}")]
    AlreadyCancelled(String),
    /// 有效条目数超上限。
    #[error("有效日程条目数已满 (上限 {0})")]
    Full(usize),
    /// 账本行数超上限 (防炸弹账本)。
    #[error("日程账行数超上限 {0} (防炸弹账本)")]
    LedgerTooLarge(usize),
    /// 审计行写入失败 (账已落, 如实报 —— 不假装已审计)。
    #[error("日程已入账但审计行写入失败: {0} (账已落, 如实报)")]
    AuditFailed(String),
}

/// 账本折算出的当前态 (条目 + 坏行计数 + 总行数)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CalendarSnapshot {
    /// 当前条目态 (按开始时刻排序, 同刻按 id 排序)。
    pub entries: Vec<CalendarEntry>,
    /// 坏行数 (坏行读作不存在: 跳过并计数, 不砖化整本)。
    pub skipped: usize,
    /// 账上总行数 (好行 + 坏行)。
    pub rows: usize,
}

/// 日程账 (数据目录 `calendar/` 子目录: 条目账 + 审计账)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CalendarStore {
    dir: PathBuf,
    ledger_path: PathBuf,
    audit_path: PathBuf,
}

impl CalendarStore {
    /// 数据目录落点 (`<data_dir>/calendar/`)。
    pub fn under_data_dir(data_dir: impl AsRef<Path>) -> Self {
        Self::at_dir(data_dir.as_ref().join(CALENDAR_DIR_NAME))
    }

    /// 显式日程面目录落点。
    pub fn at_dir(dir: impl Into<PathBuf>) -> Self {
        let dir = dir.into();
        Self {
            ledger_path: dir.join(CALENDAR_LEDGER_FILE),
            audit_path: dir.join(CALENDAR_AUDIT_FILE),
            dir,
        }
    }

    /// 日程面目录。
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// 条目账路径 (`<dir>/entries.jsonl`)。
    pub fn ledger_path(&self) -> &Path {
        &self.ledger_path
    }

    /// 审计账路径 (`<dir>/audit.jsonl`).
    pub fn audit_path(&self) -> &Path {
        &self.audit_path
    }

    /// 条目账读端契约 (仅当前格式版本可读)。
    fn ledger_compat() -> DocCompat {
        DocCompat::exact(CALENDAR_LEDGER_DOC_NAME, CALENDAR_LEDGER_DOC_VERSION)
    }

    /// 审计账读端契约 (仅当前格式版本可读)。
    fn audit_compat() -> DocCompat {
        DocCompat::exact(CALENDAR_AUDIT_DOC_NAME, CALENDAR_AUDIT_DOC_VERSION)
    }

    /// 读条目账 (逐记录语义: **坏行读作不存在**, 跳过并计数, 不砖化整本)。
    pub fn scan(&self) -> Result<RecordScan<CalendarLedgerEntry>, String> {
        scan_records::<CalendarLedgerEntry>(&self.ledger_path, &Self::ledger_compat())
            .map_err(|error| format!("日程账读取失败 {}: {error}", self.ledger_path.display()))
    }

    /// 读审计账 (逐记录语义: 坏行读作不存在, 跳过并计数)。
    pub fn scan_audit(&self) -> Result<RecordScan<CalendarAuditRow>, String> {
        scan_records::<CalendarAuditRow>(&self.audit_path, &Self::audit_compat())
            .map_err(|error| format!("日程审计账读取失败 {}: {error}", self.audit_path.display()))
    }

    /// 折出当前条目态 (按账上顺序折叠: 新建 → 改期 → 取消; 取消是终态)。
    pub fn snapshot(&self) -> Result<CalendarSnapshot, String> {
        let scan = self.scan()?;
        let mut order: Vec<String> = Vec::new();
        let mut map: std::collections::BTreeMap<String, CalendarEntry> =
            std::collections::BTreeMap::new();
        for record in &scan.records {
            match &record.body {
                CalendarLedgerEntry::Created(entry) => {
                    if !map.contains_key(&entry.id) {
                        order.push(entry.id.clone());
                    }
                    map.insert(entry.id.clone(), entry.clone());
                }
                CalendarLedgerEntry::Updated(entry) => {
                    let mut next = entry.clone();
                    // 取消是终态: 已取消的条目即使账上出现改期行, 也保持取消态。
                    if let Some(previous) = map.get(&entry.id) {
                        if previous.status == CalendarStatus::Cancelled {
                            next.status = CalendarStatus::Cancelled;
                        }
                    }
                    if !map.contains_key(&entry.id) {
                        order.push(entry.id.clone());
                    }
                    map.insert(next.id.clone(), next);
                }
                CalendarLedgerEntry::Cancelled { entry_id, at_ms } => {
                    if let Some(entry) = map.get_mut(entry_id) {
                        entry.status = CalendarStatus::Cancelled;
                        entry.updated_at_ms = *at_ms;
                    }
                }
            }
        }
        let mut entries: Vec<CalendarEntry> = order
            .into_iter()
            .filter_map(|id| map.get(&id).cloned())
            .collect();
        entries.sort_by(|left, right| {
            left.start_ms
                .cmp(&right.start_ms)
                .then_with(|| left.id.cmp(&right.id))
        });
        Ok(CalendarSnapshot {
            entries,
            skipped: scan.skipped,
            rows: scan.records.len() + scan.skipped,
        })
    }

    /// 追加一行到条目账 (append-only; 复用仓内唯一追加手续)。
    fn append_ledger(&self, entry: &CalendarLedgerEntry) -> Result<(), String> {
        std::fs::create_dir_all(&self.dir)
            .map_err(|error| format!("日程面目录创建失败 {:?}: {error}", self.dir))?;
        let doc = StoredDoc::new(
            CALENDAR_LEDGER_DOC_NAME,
            CALENDAR_LEDGER_DOC_VERSION,
            vec![CALENDAR_LEDGER_DOC_VERSION],
            entry,
        );
        crate::proposal::append_jsonl_row(&self.ledger_path, "日程账", &doc)
    }

    /// 追加一行到审计账 (append-only; 复用仓内唯一追加手续)。
    fn append_audit(&self, row: &CalendarAuditRow) -> Result<(), String> {
        std::fs::create_dir_all(&self.dir)
            .map_err(|error| format!("日程面目录创建失败 {:?}: {error}", self.dir))?;
        let doc = StoredDoc::new(
            CALENDAR_AUDIT_DOC_NAME,
            CALENDAR_AUDIT_DOC_VERSION,
            vec![CALENDAR_AUDIT_DOC_VERSION],
            row,
        );
        crate::proposal::append_jsonl_row(&self.audit_path, "日程审计账", &doc)
    }

    /// 新建条目 (写账本 + 写审计行)。
    pub fn create(&self, entry: CalendarEntry) -> Result<CalendarEntry, CalendarError> {
        let snapshot = self.snapshot().map_err(CalendarError::Ledger)?;
        if snapshot.rows + 1 > CALENDAR_MAX_LEDGER_ROWS {
            return Err(CalendarError::LedgerTooLarge(CALENDAR_MAX_LEDGER_ROWS));
        }
        if snapshot.entries.iter().any(|item| item.id == entry.id) {
            return Err(CalendarError::DuplicateId(entry.id));
        }
        let active = snapshot
            .entries
            .iter()
            .filter(|item| item.status == CalendarStatus::Scheduled)
            .count();
        if active >= CALENDAR_MAX_ENTRIES {
            return Err(CalendarError::Full(CALENDAR_MAX_ENTRIES));
        }
        self.append_ledger(&CalendarLedgerEntry::Created(entry.clone()))
            .map_err(CalendarError::Ledger)?;
        self.write_audit(ACTION_CREATE, &entry.id, &entry.note_text())?;
        Ok(entry)
    }

    /// 改期 / 改标题 / 改备注 (写账本 + 写审计行)。
    ///
    /// 取消是终态 ⇒ 已取消的条目**不可**再改期 (拒绝即帧)。
    pub fn update(&self, entry: CalendarEntry) -> Result<CalendarEntry, CalendarError> {
        let snapshot = self.snapshot().map_err(CalendarError::Ledger)?;
        if snapshot.rows + 1 > CALENDAR_MAX_LEDGER_ROWS {
            return Err(CalendarError::LedgerTooLarge(CALENDAR_MAX_LEDGER_ROWS));
        }
        let previous = snapshot
            .entries
            .iter()
            .find(|item| item.id == entry.id)
            .ok_or_else(|| CalendarError::UnknownEntry(entry.id.clone()))?;
        if previous.status == CalendarStatus::Cancelled {
            return Err(CalendarError::AlreadyCancelled(entry.id));
        }
        self.append_ledger(&CalendarLedgerEntry::Updated(entry.clone()))
            .map_err(CalendarError::Ledger)?;
        self.write_audit(ACTION_UPDATE, &entry.id, &entry.note_text())?;
        Ok(entry)
    }

    /// 取消条目 (写账本 + 写审计行; 取消是终态)。
    pub fn cancel(&self, entry_id: &str, at_ms: i64) -> Result<CalendarEntry, CalendarError> {
        let snapshot = self.snapshot().map_err(CalendarError::Ledger)?;
        if snapshot.rows + 1 > CALENDAR_MAX_LEDGER_ROWS {
            return Err(CalendarError::LedgerTooLarge(CALENDAR_MAX_LEDGER_ROWS));
        }
        let mut entry = snapshot
            .entries
            .iter()
            .find(|item| item.id == entry_id)
            .cloned()
            .ok_or_else(|| CalendarError::UnknownEntry(entry_id.to_string()))?;
        if entry.status == CalendarStatus::Cancelled {
            return Err(CalendarError::AlreadyCancelled(entry_id.to_string()));
        }
        self.append_ledger(&CalendarLedgerEntry::Cancelled {
            entry_id: entry_id.to_string(),
            at_ms,
        })
        .map_err(CalendarError::Ledger)?;
        entry.status = CalendarStatus::Cancelled;
        entry.updated_at_ms = at_ms;
        self.write_audit(ACTION_CANCEL, entry_id, &format!("取消于 {at_ms}"))?;
        Ok(entry)
    }

    /// 落一行写动作审计 (账已落, 审计失败 ⇒ 如实报, 不假装已审计)。
    fn write_audit(&self, action: &str, entry_id: &str, note: &str) -> Result<(), CalendarError> {
        self.append_audit(&CalendarAuditRow {
            action: action.to_string(),
            entry_id: entry_id.to_string(),
            outcome: "ok".to_string(),
            note: note.to_string(),
            at_ms: now_epoch_ms(),
        })
        .map_err(CalendarError::AuditFailed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn boundary_validations_refuse_instead_of_truncating() {
        // 空标题 / 超长标题。
        assert!(validate_calendar_title("   ").is_err());
        assert!(validate_calendar_title(&"x".repeat(CALENDAR_TITLE_MAX_BYTES + 1)).is_err());
        assert!(validate_calendar_title(&"x".repeat(CALENDAR_TITLE_MAX_BYTES)).is_ok());
        // 备注超长。
        assert!(validate_calendar_note(&"x".repeat(CALENDAR_NOTE_MAX_BYTES + 1)).is_err());
        assert_eq!(validate_calendar_note("  ").expect("空白备注归一"), None);
        // 时间: 倒挂 / 零时长 / 非正 / 超长时段。
        assert!(validate_calendar_span(1_000, 999).is_err());
        assert!(validate_calendar_span(1_000, 1_000).is_err());
        assert!(validate_calendar_span(0, 1_000).is_err());
        assert!(validate_calendar_span(1_000, 1_000 + CALENDAR_MAX_SPAN_MS + 1).is_err());
        assert!(validate_calendar_span(1_000, 2_000).is_ok());
        // id 字符集。
        assert!(validate_calendar_entry_id("").is_err());
        assert!(validate_calendar_entry_id("has space").is_err());
        assert!(validate_calendar_entry_id(&"x".repeat(CALENDAR_ENTRY_ID_MAX_BYTES + 1)).is_err());
        assert!(validate_calendar_entry_id("cal-1").is_ok());
    }

    #[test]
    fn generated_entry_ids_stay_inside_the_closed_charset() {
        let id = next_calendar_entry_id(1_700_000_000_000);
        assert!(id.starts_with("cal-"), "{id}");
        assert!(validate_calendar_entry_id(&id).is_ok(), "{id}");
    }
}
