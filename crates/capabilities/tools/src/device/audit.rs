//! 敏感读面审计账 (`device-sense-audit.jsonl`, append-only)。
//!
//! # 显式授权 + 审计 (摄像头类敏感读面)
//!
//! 摄像头类 (谁在家类目) = **敏感读面**: 每一次感知尝试 (无论放行还是拒绝) 都
//! 落一行审计 —— 谁授权 (`authorized_by`)、哪台设备、什么结果、什么时刻。
//! 审计账是 append-only 的 [`StoredDoc`] 信封行 (与提案账 / 语句账本同姿势),
//! 不重写、不删行; 坏行读作不存在 (跳过并计入 `skipped`), 不砖化整本。
//!
//! 零明文留痕: 审计行只留设备 id / 类别 / 授权人标识 / 结果 / 时刻, 不留任何
//! 读数内容 (图像内容永不出适配器, 见 [`crate::device::adapter`])。

use std::path::{Path, PathBuf};

use apeireth_core::stored_doc::{scan_records, DocCompat, RecordScan, StoredDoc};
use serde::{Deserialize, Serialize};

use super::registry::DeviceKind;

/// 敏感读面审计账文件 (数据目录落点: `<data_dir>/device-sense-audit.jsonl`)。
pub const DEVICE_SENSE_AUDIT_FILE: &str = "device-sense-audit.jsonl";

/// 审计账 StoredDoc 的身份戳。
pub const DEVICE_SENSE_AUDIT_DOC_NAME: &str = "device-sense-audit";

/// 审计账 StoredDoc 的格式版本戳。
pub const DEVICE_SENSE_AUDIT_DOC_VERSION: u32 = 1;

/// 敏感读结果 (闭合词表: 放行 / 拒绝)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SensitiveReadOutcome {
    /// 显式授权齐备, 读面放行。
    Granted,
    /// 显式授权缺位/不诚实, 拒绝即帧。
    Denied,
}

impl SensitiveReadOutcome {
    /// 稳定标签 (`granted` / `denied`)。
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Granted => "granted",
            Self::Denied => "denied",
        }
    }
}

/// 敏感读面审计一行 (append-only 的行主体)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SensitiveReadAudit {
    /// 被读设备 id。
    pub device_id: String,
    /// 设备类别 (敏感面判定依据; 摄像头类恒入账)。
    pub kind: DeviceKind,
    /// 动作 (`sense`)。
    pub action: String,
    /// 谁给的显式授权 (`None` = 无授权人, 对应 `denied`)。
    pub authorized_by: Option<String>,
    /// 放行还是拒绝。
    pub outcome: SensitiveReadOutcome,
    /// 判定原因 (人读, 如实)。
    pub reason: String,
    /// 审计时刻 (epoch 毫秒)。
    pub at_ms: i64,
}

/// 敏感读面审计账 (append-only, 逐行一个 [`StoredDoc`] 信封)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceSenseAuditLedger {
    path: PathBuf,
}

impl DeviceSenseAuditLedger {
    /// 数据目录落点 (`<data_dir>/device-sense-audit.jsonl`)。
    pub fn under_data_dir(data_dir: impl AsRef<Path>) -> Self {
        Self {
            path: data_dir.as_ref().join(DEVICE_SENSE_AUDIT_FILE),
        }
    }

    /// 显式路径落点。
    pub fn at(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// 审计账路径。
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// 追加一行审计 (JSONL 单行 + 落盘; append-only, 不重写不删行)。
    pub fn append(&self, entry: &SensitiveReadAudit) -> Result<(), String> {
        use std::io::Write;
        if let Some(parent) = self.path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)
                    .map_err(|error| format!("审计账目录创建失败 {parent:?}: {error}"))?;
            }
        }
        let doc = StoredDoc::new(
            DEVICE_SENSE_AUDIT_DOC_NAME,
            DEVICE_SENSE_AUDIT_DOC_VERSION,
            vec![DEVICE_SENSE_AUDIT_DOC_VERSION],
            entry,
        );
        // ⚠️ **2026-10-27 修同族缺陷（安全记录丢失）**：原写法是**两次写**
        // （行体一次、换行一次）⇒ **并发写者**可在两次写之间插进来
        // ⇒ ⇒ **两行被挤成一行** ⇒ **审计记录丢失**（读回时那行解析失败 ⇒
        // 计入 `skipped` ⇒ **看着像「坏行」，实则是「丢记录」**）。
        // ⇒ 改用**共用原语**（行体与行尾换行同在一次写 ⇒ O_APPEND 原子）。
        crate::proposal::append_jsonl_row(&self.path, "审计账", &doc)
    }

    /// 读回整本 (逐记录语义: 坏行读作不存在并计入 `skipped`)。
    pub fn scan(&self) -> Result<RecordScan<SensitiveReadAudit>, String> {
        scan_records::<SensitiveReadAudit>(&self.path, &ledger_compat())
            .map_err(|error| format!("审计账读取失败 {}: {error}", self.path.display()))
    }
}

/// 审计账读端契约 (仅当前格式版本可读)。
fn ledger_compat() -> DocCompat {
    DocCompat::exact(DEVICE_SENSE_AUDIT_DOC_NAME, DEVICE_SENSE_AUDIT_DOC_VERSION)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(outcome: SensitiveReadOutcome) -> SensitiveReadAudit {
        SensitiveReadAudit {
            device_id: "cam-1".to_string(),
            kind: DeviceKind::Camera,
            action: "sense".to_string(),
            authorized_by: Some("owner".to_string()),
            outcome,
            reason: "测试".to_string(),
            at_ms: 1_700_000_000_000,
        }
    }

    /// 回归（2026-10-27）：**并发追加不许丢审计行**。
    ///
    /// 旧实现是**两次写**（行体一次、换行一次）⇒ **并发写者**可在两次写之间插进来
    /// ⇒ ⇒ **两行被挤成一行** ⇒ ⇒ **审计记录丢失**（读回时那行解析失败 ⇒ 计入
    /// `skipped` ⇒ **看着像「坏行」，实则是「丢记录」**）。
    /// ⇒ 审计账是**安全记录**（敏感读面每次尝试都入账）⇒ **丢一行就是丢一次尝试的痕迹**。
    #[test]
    fn concurrent_appends_keep_every_audit_row() {
        let dir = std::env::temp_dir().join(format!(
            "apeireth-device-audit-{}-{}",
            "concurrent",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        let ledger = std::sync::Arc::new(DeviceSenseAuditLedger::under_data_dir(&dir));
        let mut handles = Vec::new();
        for worker in 0..4u32 {
            let ledger = ledger.clone();
            handles.push(std::thread::spawn(move || {
                for index in 0..8u32 {
                    let mut record = entry(SensitiveReadOutcome::Granted);
                    record.device_id = format!("cam-{worker}-{index}");
                    record.at_ms = 1_700_000_000_000 + i64::from(worker * 100 + index);
                    ledger.append(&record).expect("append");
                }
            }));
        }
        for handle in handles {
            handle.join().expect("join");
        }
        let scan = ledger.scan().unwrap();
        assert_eq!(
            scan.records.len(),
            32,
            "并发追加不许丢审计行（读到 {} 行）",
            scan.records.len()
        );
        assert_eq!(scan.skipped, 0, "并发追加不许产生坏行");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 回归（2026-10-27）：**撕裂写后追加不许把旧行吞掉**。
    #[test]
    fn appending_after_a_torn_line_keeps_every_audit_row_readable() {
        let dir = std::env::temp_dir().join(format!(
            "apeireth-device-audit-{}-{}",
            "torn",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        let ledger = DeviceSenseAuditLedger::under_data_dir(&dir);
        ledger
            .append(&entry(SensitiveReadOutcome::Granted))
            .expect("append 1");

        // 制造撕裂写：把末行的换行去掉。
        let path = ledger.path().to_path_buf();
        let mut bytes = std::fs::read(&path).expect("read audit");
        assert!(bytes.ends_with(b"\n"), "正常追加的末行应有换行");
        bytes.pop();
        std::fs::write(&path, &bytes).expect("write torn audit");

        ledger
            .append(&entry(SensitiveReadOutcome::Denied))
            .expect("append 2");
        let scan = ledger.scan().unwrap();
        assert_eq!(
            scan.records.len(),
            2,
            "撕裂写后追加不许把旧行吞掉（读到 {} 行）",
            scan.records.len()
        );
        assert_eq!(scan.skipped, 0, "撕裂写后追加不许产生坏行");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_audit_ledger_is_append_only_and_readable() {
        let dir = std::env::temp_dir().join(format!(
            "apeireth-device-audit-{}-{}",
            "roundtrip",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        let ledger = DeviceSenseAuditLedger::under_data_dir(&dir);
        ledger
            .append(&entry(SensitiveReadOutcome::Granted))
            .unwrap();
        ledger.append(&entry(SensitiveReadOutcome::Denied)).unwrap();
        let scan = ledger.scan().unwrap();
        assert_eq!(scan.records.len(), 2);
        assert_eq!(scan.skipped, 0);
        assert_eq!(scan.records[0].body.outcome, SensitiveReadOutcome::Granted);
        assert_eq!(scan.records[1].body.outcome, SensitiveReadOutcome::Denied);
    }

    #[test]
    fn a_corrupt_row_is_skipped_not_fatal() {
        let dir = std::env::temp_dir().join(format!(
            "apeireth-device-audit-{}-{}",
            "corrupt",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        let ledger = DeviceSenseAuditLedger::under_data_dir(&dir);
        ledger
            .append(&entry(SensitiveReadOutcome::Granted))
            .unwrap();
        use std::io::Write;
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(ledger.path())
            .unwrap();
        writeln!(file, "{{ 坏行").unwrap();
        let scan = ledger.scan().unwrap();
        assert_eq!(scan.records.len(), 1, "坏行读作不存在");
        assert_eq!(scan.skipped, 1, "坏行如实计数");
    }
}
