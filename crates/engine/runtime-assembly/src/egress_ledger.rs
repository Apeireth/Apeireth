//! 出域账本（Egress Ledger）—— 承诺措辞里「记录已知流向」的载体。
//!
//! 上位：ADR-0013 §2.2（动态红线）、ADR-0014 §2 / §3（承诺分级：R1 保证、R2/R4 不保证
//! 但**必须记录已知流向、提供请求路径、在发现影响时通知**）。
//!
//! # 纪律
//!
//! - **零真值**：只记「哪个对象类型、什么用途、去了哪、什么判定」，**不记内容原文**；
//!   对象以**类型 + 指纹**表示（指纹是 HMAC 截断，不可逆）。
//! - **append-only**：坏行读作不存在（与 `governance-audit.jsonl` 同纪律）。
//! - **账本本身不是泄漏面**：不写真值、不写 prompt、不写响应内容。
//!
//! # 为什么现在做
//!
//! R2/R4 不承诺（provider 侧不可控），但「不能控制」必须翻译成「可见的限制」——
//! 而「可见」的第一件事就是**有账可查**。

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// 一次出域判定的一行记录（**零真值**）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EgressLedgerEntry {
    /// 记录时刻（epoch 毫秒）。
    pub ts_unix_ms: i64,
    /// 会话 id（可空：没有会话上下文的直连请求）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session: Option<String>,
    /// 判定：`pass`（原样发出）/ `redacted`（已替换占位）/ `deny`（拒绝即帧）。
    pub verdict: String,
    /// 命中的对象类型（如 `email` / `person_name`），**不含真值**。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub objects: Vec<String>,
    /// 命中的对象指纹（HMAC 截断，不可逆），**不含真值**。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fingerprints: Vec<String>,
    /// 目的地（如 `provider:anthropic`）。
    pub destination: String,
    /// 用途（当前固定 `chat`；预留未来按用途授权）。
    pub usage: String,
    /// 判定理由（人类可读，无敏感值）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// 出域账本（append-only JSONL）。
///
/// 写不进去**不阻断出网**（账本是观测面，不是闸）—— 但会在返回值里带出
/// 「记录失败」的事实，由调用方决定是否降级告警。
#[derive(Debug, Clone)]
pub struct EgressLedger {
    path: Option<PathBuf>,
    /// 内存兜底（账本开不出时仍能记，诚实标注「不持久」）。
    entries: std::sync::Arc<std::sync::Mutex<Vec<EgressLedgerEntry>>>,
}

impl EgressLedger {
    /// 新建账本（`path` 为 `None` = 纯内存，不持久）。
    pub fn new(path: Option<PathBuf>) -> Self {
        Self {
            path,
            entries: std::sync::Arc::new(std::sync::Mutex::new(Vec::new())),
        }
    }

    /// 账本路径（`None` = 不持久）。
    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    /// 追加一行（append-only）。返回是否落盘成功。
    pub fn append(&self, entry: EgressLedgerEntry) -> bool {
        let Ok(mut guard) = self.entries.lock() else {
            return false;
        };
        guard.push(entry.clone());
        let Some(path) = &self.path else {
            // 内存账本：无持久化 = 诚实返回 false（调用方据此知道「不持久」）。
            return false;
        };
        match serde_json::to_string(&entry) {
            Ok(line) => append_line(path, &line).is_ok(),
            Err(_) => false,
        }
    }

    /// 读回全部条目（坏行读作不存在）。
    ///
    /// 有账本文件时**以文件为准**（它是 append-only 真源）；无文件时用内存兜底。
    /// 两者不叠加 —— 否则同一条会被数两次。
    pub fn entries(&self) -> Vec<EgressLedgerEntry> {
        if let Some(path) = &self.path {
            return read_entries(path);
        }
        self.entries
            .lock()
            .map(|guard| guard.clone())
            .unwrap_or_default()
    }
}

/// 追加一行（带换行）。父目录不存在时先建。
fn append_line(path: &Path, line: &str) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    file.write_all(line.as_bytes())?;
    file.write_all(b"\n")?;
    Ok(())
}

/// 读回条目（坏行读作不存在 —— 与仓内其它账本同纪律）。
fn read_entries(path: &Path) -> Vec<EgressLedgerEntry> {
    let Ok(text) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    text.lines()
        .filter_map(|line| serde_json::from_str::<EgressLedgerEntry>(line).ok())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_path(tag: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "apeireth-egress-ledger-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ))
    }

    fn entry() -> EgressLedgerEntry {
        EgressLedgerEntry {
            ts_unix_ms: 1_700_000_000_000,
            session: Some("s1".into()),
            verdict: "redacted".into(),
            objects: vec!["email".into()],
            fingerprints: vec!["ef79ffc52d25".into()],
            destination: "provider:anthropic".into(),
            usage: "chat".into(),
            reason: Some("检出 1 个受保护对象".into()),
        }
    }

    #[test]
    fn appends_and_reads_back() {
        let path = temp_path("basic");
        let ledger = EgressLedger::new(Some(path.clone()));
        assert!(ledger.append(entry()));
        let entries = ledger.entries();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].verdict, "redacted");
        assert_eq!(entries[0].objects, vec!["email".to_string()]);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn bad_lines_are_read_as_absent() {
        let path = temp_path("bad");
        std::fs::create_dir_all(path.parent().unwrap()).expect("dir");
        std::fs::write(
            &path,
            "{\"verdict\":\"pass\"}\nNOT_JSON\n{\"garbage\":true}\n",
        )
        .expect("write");
        let ledger = EgressLedger::new(Some(path.clone()));
        // 坏行读作不存在 → 0 条（而不是崩）。
        assert!(ledger.entries().is_empty());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn memory_only_ledger_reports_not_persisted() {
        let ledger = EgressLedger::new(None);
        assert!(ledger.path().is_none());
        // 内存账本：记下了，但如实说「不持久」。
        assert!(!ledger.append(entry()));
        assert_eq!(ledger.entries().len(), 1);
    }

    #[test]
    fn ledger_never_stores_raw_values() {
        let path = temp_path("zerotrue");
        let ledger = EgressLedger::new(Some(path.clone()));
        ledger.append(entry());
        let raw = std::fs::read_to_string(&path).expect("read");
        // 零真值：只有类型与指纹，没有内容。
        assert!(raw.contains("email"));
        assert!(!raw.contains("alice@example.com"));
        let _ = std::fs::remove_file(&path);
    }
}
