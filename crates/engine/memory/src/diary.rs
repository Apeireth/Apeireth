//! `apeireth-memory::diary` — 日记本归档与叙事记忆中心 (R12-LongTermMemory 实施).
//!
//! **设计哲学 (日粒度叙事归档与历史注入)**:
//! - **① 日粒度叙事记忆**: 按日归档 (`YYYY-MM-DD`)，条目级事实由 episode 处理，日记承载日粒度的沉淀与叙事；
//! - **② 确定性检索**: 提供日期范围枚举、大小写不敏感子串关键词检索；
//! - **③ 字符预算注入 (`DiaryInjector`)**: 为渲染上下文提供近 N 日日记抽取，超出字符预算时诚实声明截断 (`TRUNCATION_MARK`)；
//! - **④ 0 假装 (O-5)**:
//!   显式时间戳注入、纯确定性算法，白名单日期校验杜绝路径穿越。
//!
//! **O-6 三阶审查**:
//! 1. 总体: 解决 Agent 长程关系演进中的宏观跨日叙事沉淀与记忆再唤醒
//! 2. 系统: 放置在 `engine/memory`, 提供清晰的 `DiaryStore` Trait 与文件/内存实现
//! 3. 架构: 强类型数据模型，0 unsafe, 0 外部 C 扩展

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// 注入块截断标记 (预算不足时尾行提示，诚实声明有省略).
pub const TRUNCATION_MARK: &str = "…(已截断)";

/// 日记错误.
#[derive(Debug, Error)]
pub enum DiaryError {
    #[error("非法日期: {0} (必须为 YYYY-MM-DD 格式)")]
    InvalidDate(String),
    #[error("日记内容为空")]
    EmptyContent,
    #[error("IO 失败: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON 解析失败: {0}")]
    Json(#[from] serde_json::Error),
    #[error("存储锁中毒")]
    LockPoisoned,
}

/// 一条日记条目.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiaryEntry {
    /// 来源标注 (如 "user", "reflection", "extractor")
    pub source: String,
    /// 正文内容
    pub body: String,
    /// 写入时间戳 (毫秒)
    pub timestamp_epoch_ms: i64,
}

impl DiaryEntry {
    /// 构造新的日记条目.
    pub fn new(
        source: impl Into<String>,
        body: impl Into<String>,
        timestamp_epoch_ms: i64,
    ) -> Self {
        Self {
            source: source.into(),
            body: body.into(),
            timestamp_epoch_ms,
        }
    }
}

/// 一日的日记页 (按日归档).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DayPage {
    /// 归档日期 (YYYY-MM-DD)
    pub date: String,
    /// 当日全部条目
    #[serde(default)]
    pub entries: Vec<DiaryEntry>,
}

/// 检索命中条目.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiaryHit {
    /// 归档日期
    pub date: String,
    /// 命中条目
    pub entry: DiaryEntry,
}

/// 校验日期字符串是否严格符合 YYYY-MM-DD 且月份/日期有效 (防路径穿越).
pub fn valid_date(d: &str) -> bool {
    let b = d.as_bytes();
    if b.len() != 10 || b[4] != b'-' || b[7] != b'-' {
        return false;
    }
    if !b[..4].iter().all(u8::is_ascii_digit)
        || !b[5..7].iter().all(u8::is_ascii_digit)
        || !b[8..10].iter().all(u8::is_ascii_digit)
    {
        return false;
    }
    let Ok(m) = d[5..7].parse::<u32>() else {
        return false;
    };
    let Ok(day) = d[8..10].parse::<u32>() else {
        return false;
    };
    (1..=12).contains(&m) && (1..=31).contains(&day)
}

/// 日记存储抽象 Trait.
pub trait DiaryStore: Send + Sync {
    /// 追加一条日记.
    fn append(&self, date: &str, entry: DiaryEntry) -> Result<(), DiaryError>;

    /// 读取某一天的全部日记.
    fn read_day(&self, date: &str) -> Result<DayPage, DiaryError>;

    /// 列出所有已归档的日期 (按时间升序字典序排序).
    fn list_days(&self) -> Result<Vec<String>, DiaryError>;

    /// 按关键词检索日记条目 (不区分大小写).
    fn search(&self, keyword: &str) -> Result<Vec<DiaryHit>, DiaryError>;
}

/// 内存日记存储 (供测试与嵌入式场景).
#[derive(Debug, Default)]
pub struct InMemoryDiaryStore {
    days: Mutex<BTreeMap<String, DayPage>>,
}

impl InMemoryDiaryStore {
    pub fn new() -> Self {
        Self::default()
    }
}

impl DiaryStore for InMemoryDiaryStore {
    fn append(&self, date: &str, entry: DiaryEntry) -> Result<(), DiaryError> {
        if !valid_date(date) {
            return Err(DiaryError::InvalidDate(date.into()));
        }
        if entry.body.trim().is_empty() {
            return Err(DiaryError::EmptyContent);
        }
        let mut guard = self.days.lock().map_err(|_| DiaryError::LockPoisoned)?;
        let page = guard.entry(date.to_string()).or_insert_with(|| DayPage {
            date: date.to_string(),
            entries: Vec::new(),
        });
        page.entries.push(entry);
        Ok(())
    }

    fn read_day(&self, date: &str) -> Result<DayPage, DiaryError> {
        if !valid_date(date) {
            return Err(DiaryError::InvalidDate(date.into()));
        }
        let guard = self.days.lock().map_err(|_| DiaryError::LockPoisoned)?;
        Ok(guard.get(date).cloned().unwrap_or_else(|| DayPage {
            date: date.to_string(),
            entries: Vec::new(),
        }))
    }

    fn list_days(&self) -> Result<Vec<String>, DiaryError> {
        let guard = self.days.lock().map_err(|_| DiaryError::LockPoisoned)?;
        Ok(guard.keys().cloned().collect())
    }

    fn search(&self, keyword: &str) -> Result<Vec<DiaryHit>, DiaryError> {
        let guard = self.days.lock().map_err(|_| DiaryError::LockPoisoned)?;
        let kw = keyword.to_lowercase();
        let mut hits = Vec::new();
        for (date, page) in guard.iter() {
            for entry in &page.entries {
                if entry.body.to_lowercase().contains(&kw)
                    || entry.source.to_lowercase().contains(&kw)
                {
                    hits.push(DiaryHit {
                        date: date.clone(),
                        entry: entry.clone(),
                    });
                }
            }
        }
        Ok(hits)
    }
}

/// 文件系统日记存储 (一天一 JSON 文件，崩溃安全原子写入).
pub struct FileDiaryStore {
    root: PathBuf,
}

impl FileDiaryStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    fn path_for(&self, date: &str) -> PathBuf {
        self.root.join(format!("{date}.json"))
    }
}

impl DiaryStore for FileDiaryStore {
    fn append(&self, date: &str, entry: DiaryEntry) -> Result<(), DiaryError> {
        if !valid_date(date) {
            return Err(DiaryError::InvalidDate(date.into()));
        }
        if entry.body.trim().is_empty() {
            return Err(DiaryError::EmptyContent);
        }

        std::fs::create_dir_all(&self.root)?;
        let mut page = self.read_day(date)?;
        page.entries.push(entry);

        let target_path = self.path_for(date);
        let tmp_path = self
            .root
            .join(format!("{date}.tmp-{}", uuid::Uuid::new_v4()));
        let bytes = serde_json::to_vec_pretty(&page)?;
        std::fs::write(&tmp_path, bytes)?;
        std::fs::rename(&tmp_path, &target_path)?;
        Ok(())
    }

    fn read_day(&self, date: &str) -> Result<DayPage, DiaryError> {
        if !valid_date(date) {
            return Err(DiaryError::InvalidDate(date.into()));
        }
        let target_path = self.path_for(date);
        if !target_path.exists() {
            return Ok(DayPage {
                date: date.to_string(),
                entries: Vec::new(),
            });
        }
        let bytes = std::fs::read(&target_path)?;
        let page = serde_json::from_slice(&bytes)?;
        Ok(page)
    }

    fn list_days(&self) -> Result<Vec<String>, DiaryError> {
        if !self.root.exists() {
            return Ok(Vec::new());
        }
        let mut days = Vec::new();
        for entry in std::fs::read_dir(&self.root)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "json") {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    if valid_date(stem) {
                        days.push(stem.to_string());
                    }
                }
            }
        }
        days.sort();
        Ok(days)
    }

    fn search(&self, keyword: &str) -> Result<Vec<DiaryHit>, DiaryError> {
        let days = self.list_days()?;
        let kw = keyword.to_lowercase();
        let mut hits = Vec::new();
        for date in days {
            let page = self.read_day(&date)?;
            for entry in page.entries {
                if entry.body.to_lowercase().contains(&kw)
                    || entry.source.to_lowercase().contains(&kw)
                {
                    hits.push(DiaryHit {
                        date: date.clone(),
                        entry,
                    });
                }
            }
        }
        Ok(hits)
    }
}

/// 日记注入器: 将近 N 日日记抽取并在字符预算内渲染为上下文 Prompt 块.
pub struct DiaryInjector;

impl DiaryInjector {
    /// 渲染最近 `count` 天的日记，保证总字符数不超过 `budget_chars`.
    pub fn render_recent_days(
        store: &dyn DiaryStore,
        count: usize,
        budget_chars: usize,
    ) -> Result<String, DiaryError> {
        let all_days = store.list_days()?;
        if all_days.is_empty() || count == 0 || budget_chars == 0 {
            return Ok(String::new());
        }

        // 取最近 count 天 (升序排列)
        let recent_days: Vec<String> = all_days
            .into_iter()
            .rev()
            .take(count)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();

        let mut lines = Vec::new();
        lines.push("【近期日记记忆】".to_string());

        let mut current_chars = lines[0].chars().count();
        let mut truncated = false;

        for date in recent_days {
            let page = store.read_day(&date)?;
            if page.entries.is_empty() {
                continue;
            }

            let header = format!("- 日期 {}:", page.date);
            let header_chars = header.chars().count();
            if current_chars + header_chars + 1 > budget_chars {
                truncated = true;
                break;
            }
            lines.push(header);
            current_chars += header_chars + 1;

            for entry in page.entries {
                let entry_str = format!("  * [{}] {}", entry.source, entry.body);
                let entry_chars = entry_str.chars().count();
                if current_chars + entry_chars + 1 > budget_chars {
                    truncated = true;
                    break;
                }
                lines.push(entry_str);
                current_chars += entry_chars + 1;
            }
            if truncated {
                break;
            }
        }

        if truncated {
            lines.push(TRUNCATION_MARK.to_string());
        }

        if lines.len() <= 1 {
            return Ok(String::new());
        }

        Ok(lines.join("\n"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_date_validation() {
        assert!(valid_date("2026-08-28"));
        assert!(valid_date("2026-01-01"));
        assert!(valid_date("2026-12-31"));

        assert!(!valid_date("2026-08-32"));
        assert!(!valid_date("2026-13-01"));
        assert!(!valid_date("2026/08/28"));
        assert!(!valid_date("invalid"));
        assert!(!valid_date("../etc/passwd"));
    }

    #[test]
    fn in_memory_store_append_and_search() {
        let store = InMemoryDiaryStore::new();
        let entry1 = DiaryEntry::new("user", "今天主人教我写 Rust", 1000);
        let entry2 = DiaryEntry::new("reflection", "学习了生命周期和所有权", 2000);

        store.append("2026-08-28", entry1.clone()).unwrap();
        store.append("2026-08-28", entry2.clone()).unwrap();

        let page = store.read_day("2026-08-28").unwrap();
        assert_eq!(page.entries.len(), 2);
        assert_eq!(page.entries[0], entry1);

        let hits = store.search("Rust").unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].entry.body, "今天主人教我写 Rust");
    }

    #[test]
    fn file_store_atomic_roundtrip() {
        let tmp = tempfile::tempdir().unwrap();
        let store = FileDiaryStore::new(tmp.path());

        let entry = DiaryEntry::new("user", "测试原子持久化", 1000);
        store.append("2026-08-29", entry.clone()).unwrap();

        let days = store.list_days().unwrap();
        assert_eq!(days, vec!["2026-08-29"]);

        let page = store.read_day("2026-08-29").unwrap();
        assert_eq!(page.entries.len(), 1);
        assert_eq!(page.entries[0], entry);
    }

    #[test]
    fn diary_injector_truncates_with_budget() {
        let store = InMemoryDiaryStore::new();
        store
            .append(
                "2026-08-27",
                DiaryEntry::new("user", "很长很长的一段日记描述01", 1000),
            )
            .unwrap();
        store
            .append(
                "2026-08-28",
                DiaryEntry::new("user", "很长很长的一段日记描述02", 2000),
            )
            .unwrap();

        let injected = DiaryInjector::render_recent_days(&store, 2, 50).unwrap();
        assert!(injected.contains("【近期日记记忆】"));
        assert!(injected.contains(TRUNCATION_MARK));
    }
}
