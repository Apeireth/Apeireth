//! 语句账本 (`spoken.jsonl`) —— 说了什么 / 何时 / 哪把嗓 / 时长, append-only 可查。
//!
//! # 口径 (脱敏真账)
//!
//! - **全文唯一落点就是本账本**: 合成文本只进 `spoken.jsonl` (它的职责是
//!   「说了什么可查」), **不进日志、不进错误帧、不进审批载荷** —— 那些面
//!   只带字符数 / 嗓名 / 时长 / 状态;
//! - **append-only**: 逐行一个 [`StoredDoc`] 信封, 追加写 + 落盘 (与技能卡
//!   版本演进审计同一姿势), 不重写、不删行;
//! - **坏行读作不存在**: 读回走 [`scan_records`] 逐记录语义 (坏行跳过并
//!   如实计入 `skipped`), 不砖化整本;
//! - **记的是真说过的话**: 合成失败 = 没说话 = 不入账 (错误即帧); 被打断 =
//!   说了半句 = 如实入账 (状态 `interrupted` + 已发音时长)。

use std::path::{Path, PathBuf};

use apeireth_core::stored_doc::{scan_records, DocCompat, RecordScan, StoredDoc};
use serde::{Deserialize, Serialize};

use crate::self_status::VoiceStatus;

/// 语句账本文件 (数据目录落点: `<data_dir>/spoken.jsonl`)。
pub const SPOKEN_LEDGER_FILE: &str = "spoken.jsonl";

/// 语句账本 StoredDoc 的身份与格式版本戳。
pub const SPOKEN_DOC_NAME: &str = "voice-spoken-ledger";

/// 语句账本 StoredDoc 的格式版本戳。
pub const SPOKEN_DOC_VERSION: u32 = 1;

/// 自述「最近说话」的摘录上限 (字符; 超出截断并标注)。
pub const VOICE_EXCERPT_MAX_CHARS: usize = 80;

/// 一条语句的账本状态 (真说过 vs 说了半句)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpokenStatus {
    /// 说完 (合成即播走完全程)。
    Completed,
    /// 说了半句 (被新话打断 / 被取消真停)。
    Interrupted,
}

impl SpokenStatus {
    /// 稳定标签 (`completed` / `interrupted`)。
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Completed => "completed",
            Self::Interrupted => "interrupted",
        }
    }
}

/// 一条语句账本记录 (append-only 的一行主体)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpokenUtterance {
    /// 语句稳定身份 (`spk-<时刻毫秒>-<进程内序号>`)。
    pub id: String,
    /// 发声时刻 (epoch 毫秒)。
    pub spoken_at_ms: i64,
    /// 说了什么 (全文; 只在本账本出现)。
    pub text: String,
    /// 哪把嗓 (系统默认嗓记 [`DEFAULT_VOICE_LABEL`])。
    pub voice: String,
    /// 实际发音时长 (毫秒; 被打断 = 已发音部分)。
    pub duration_ms: u64,
    /// 账本状态 ([`SpokenStatus`])。
    pub status: SpokenStatus,
    /// 生效语速 (SAPI 面 `[-10, 10]`)。
    pub rate: i32,
    /// 生效音量 (SAPI 面 `[0, 100]`)。
    pub volume: u32,
    /// 承载引擎标识。
    pub engine: String,
}

/// 系统默认嗓在账本 / 自述里的稳定标签。
pub const DEFAULT_VOICE_LABEL: &str = "system-default";

/// 语句身份 (`spk-<时刻毫秒>-<进程内序号>`)。
pub fn spoken_utterance_id(spoken_at_ms: i64, ordinal: u64) -> String {
    format!("spk-{spoken_at_ms}-{ordinal}")
}

/// 「最近说话」摘录 (超 [`VOICE_EXCERPT_MAX_CHARS`] 截断并加省略标)。
pub fn voice_excerpt(text: &str) -> String {
    let mut excerpt: String = text.chars().take(VOICE_EXCERPT_MAX_CHARS).collect();
    if text.chars().count() > VOICE_EXCERPT_MAX_CHARS {
        excerpt.push('…');
    }
    excerpt
}

/// 语句账本 (数据目录 `spoken.jsonl`)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpokenLedger {
    path: PathBuf,
}

impl SpokenLedger {
    /// 数据目录落点 (`<data_dir>/spoken.jsonl`)。
    pub fn under_data_dir(data_dir: impl AsRef<Path>) -> Self {
        Self {
            path: data_dir.as_ref().join(SPOKEN_LEDGER_FILE),
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

    /// 追加一条语句 (JSONL 单行 + 落盘; append-only, 不重写不删行)。
    pub fn append(&self, utterance: &SpokenUtterance) -> Result<(), String> {
        use std::io::Write;
        if let Some(parent) = self.path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)
                    .map_err(|error| format!("语句账本目录创建失败 {parent:?}: {error}"))?;
            }
        }
        let doc = StoredDoc::new(
            SPOKEN_DOC_NAME,
            SPOKEN_DOC_VERSION,
            vec![SPOKEN_DOC_VERSION],
            utterance,
        );
        let line =
            serde_json::to_vec(&doc).map_err(|error| format!("语句账本序列化失败: {error}"))?;
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .map_err(|error| format!("语句账本打开失败 {}: {error}", self.path.display()))?;
        file.write_all(&line)
            .and_then(|()| file.write_all(b"\n"))
            .and_then(|()| file.sync_all())
            .map_err(|error| format!("语句账本写入失败 {}: {error}", self.path.display()))
    }

    /// 读回整本 (逐记录语义: 坏行读作不存在并计入 `skipped`)。
    pub fn scan(&self) -> Result<RecordScan<SpokenUtterance>, String> {
        scan_records::<SpokenUtterance>(&self.path, &ledger_compat())
            .map_err(|error| format!("语句账本读取失败 {}: {error}", self.path.display()))
    }

    /// 自述语音节的现探测 (账本数 / 最近说话 / 嗓名, 真值如实)。
    pub fn status(&self) -> Result<VoiceStatus, String> {
        let scan = self.scan()?;
        let utterances = scan.records.len() as u64;
        let mut note: Option<String> = None;
        if scan.skipped > 0 {
            note = Some(format!(
                "账本 {} 行坏行读作不存在 (如实跳过, 不砖化整本)",
                scan.skipped
            ));
        }
        let last = scan.records.last();
        if last.is_none() && note.is_none() {
            note = Some("尚无语句记录".to_string());
        }
        Ok(VoiceStatus {
            utterances: Some(utterances),
            last_spoken_at_ms: last.map(|record| record.body.spoken_at_ms),
            last_text_excerpt: last.map(|record| voice_excerpt(&record.body.text)),
            voice: last.map(|record| record.body.voice.clone()),
            note,
            reason: None,
        })
    }
}

/// 账本读端契约 (仅当前格式版本可读)。
fn ledger_compat() -> DocCompat {
    DocCompat::exact(SPOKEN_DOC_NAME, SPOKEN_DOC_VERSION)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn utterance(text: &str) -> SpokenUtterance {
        SpokenUtterance {
            id: spoken_utterance_id(1_700_000_000_000, 1),
            spoken_at_ms: 1_700_000_000_000,
            text: text.to_string(),
            voice: DEFAULT_VOICE_LABEL.to_string(),
            duration_ms: 320,
            status: SpokenStatus::Completed,
            rate: 0,
            volume: 100,
            engine: "mock".to_string(),
        }
    }

    #[test]
    fn append_then_read_back_roundtrips_in_order() {
        let dir = tempfile::tempdir().expect("tempdir");
        let ledger = SpokenLedger::under_data_dir(dir.path());
        ledger.append(&utterance("第一句")).expect("append 1");
        ledger.append(&utterance("第二句")).expect("append 2");
        let scan = ledger.scan().expect("scan");
        assert_eq!(scan.skipped, 0);
        let texts: Vec<&str> = scan
            .records
            .iter()
            .map(|doc| doc.body.text.as_str())
            .collect();
        assert_eq!(texts, vec!["第一句", "第二句"], "append-only 读回保序");
    }

    #[test]
    fn bad_lines_are_skipped_and_counted_honestly() {
        let dir = tempfile::tempdir().expect("tempdir");
        let ledger = SpokenLedger::under_data_dir(dir.path());
        ledger.append(&utterance("好行")).expect("append");
        use std::io::Write;
        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .open(ledger.path())
            .expect("open");
        writeln!(file, "{{not json").expect("bad line");
        drop(file);
        let scan = ledger.scan().expect("scan");
        assert_eq!(scan.records.len(), 1, "好行照读");
        assert_eq!(scan.skipped, 1, "坏行读作不存在并如实计数");
        let status = ledger.status().expect("status");
        assert_eq!(status.utterances, Some(1));
    }

    #[test]
    fn status_reports_the_last_utterance_and_the_voice() {
        let dir = tempfile::tempdir().expect("tempdir");
        let ledger = SpokenLedger::under_data_dir(dir.path());

        let empty = ledger.status().expect("empty status");
        assert_eq!(empty.utterances, Some(0));
        assert_eq!(empty.note.as_deref(), Some("尚无语句记录"));
        assert_eq!(empty.last_spoken_at_ms, None, "尚无语句不编时间");

        let mut spoken = utterance("你好, 这是一句较长的话");
        spoken.voice = "测试嗓".to_string();
        ledger.append(&spoken).expect("append");
        let status = ledger.status().expect("status");
        assert_eq!(status.utterances, Some(1));
        assert_eq!(status.last_spoken_at_ms, Some(1_700_000_000_000));
        assert_eq!(status.voice.as_deref(), Some("测试嗓"));
        assert_eq!(
            status.last_text_excerpt.as_deref(),
            Some("你好, 这是一句较长的话")
        );
    }

    #[test]
    fn excerpt_is_bounded() {
        let text: String = "语".repeat(VOICE_EXCERPT_MAX_CHARS + 10);
        let excerpt = voice_excerpt(&text);
        assert_eq!(
            excerpt.chars().count(),
            VOICE_EXCERPT_MAX_CHARS + 1,
            "截断 + 省略标"
        );
        assert!(excerpt.ends_with('…'));
        assert_eq!(voice_excerpt("短句"), "短句");
    }
}
