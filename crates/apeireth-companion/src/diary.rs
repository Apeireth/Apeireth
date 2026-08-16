//! 日记本中心 (§5.1 记忆域深化包机制⑤; VCP RAGDiaryPlugin 精神吸收).
//!
//! **职责**: 日粒度叙事记忆 —— 按日归档 (root 下 `{YYYY-MM-DD}.json` 一天一文件) +
//! 确定性检索 (日期范围/关键词子串) + 注入块生成 (近 N 日摘要, 字符预算内截断).
//!
//! **与 episodes 记忆区分**: episodes = 条目级事实流; 日记 = 日粒度叙事归档,
//! 不重复存条目级事实 (调用方决定写入时机, 本模块只承载归档机制).
//!
//! **真实机制 (0 假装)**:
//! - [`DiaryStore`]: root + clock 注入 (VirtualClock 可快进, 0 真等待)
//! - append (同日追加/跨日分文件) / read_day / list_days (字典序=时间序) /
//!   search (大小写不敏感子串匹配, 全确定性)
//! - [`DiaryInjector`]: 注入块 trait 口 (infallible, 失败/空 → 空串诚实降级)
//!
//! **0 装 PASS 标注 (诚实)**:
//! - 注入实接线 (assemble.rs/context.rs 渲染链挂接) 延后: companion crate 当前被
//!   N14 (他人未提交 WIP 编译失败) 阻塞, 且两文件已有主人 — 本模块只提供机制口
//! - 检索为确定性子串匹配 (0 向量/embedding); RAG 式语义检索可后续接 semantic 索引
//! - 写入侧 (何时归档一天) 由调用方驱动, 不做自动生成

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use apeireth_core::clock::Clock;
use chrono::Datelike;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// 注入块截断标记 (预算不足时尾行提示, 注入侧诚实声明"有省略").
pub const TRUNCATION_MARK: &str = "…(已截断)";

/// 日记错误 (非法输入显式拒绝, 不 panic).
#[derive(Debug, Error)]
pub enum DiaryError {
    #[error("非法日期: {0} (须 YYYY-MM-DD)")]
    InvalidDate(String),
    #[error("内容为空")]
    EmptyContent,
    #[error("IO 失败: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON 失败: {0}")]
    Json(#[from] serde_json::Error),
}

/// 一条日记条目 (source = 来源标注, 如 reflection/user/extractor).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiaryEntry {
    pub source: String,
    pub body: String,
}

/// 一日日记页 (date = 归档日 YYYY-MM-DD).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DayPage {
    pub date: String,
    #[serde(default)]
    pub entries: Vec<DiaryEntry>,
}

/// 检索命中 (date + 命中条目; 顺序确定性: 日期升序, 日内存储序).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiaryHit {
    pub date: String,
    pub entry: DiaryEntry,
}

/// 日期字符串校验: 严格 YYYY-MM-DD (含月 1-12/日 1-31 范围), 兼防路径注入.
fn valid_date(d: &str) -> bool {
    let b = d.as_bytes();
    b.len() == 10
        && b[4] == b'-'
        && b[7] == b'-'
        && b[..4].iter().all(u8::is_ascii_digit)
        && b[5..7].iter().all(u8::is_ascii_digit)
        && b[8..10].iter().all(u8::is_ascii_digit)
        && {
            let m = &d[5..7].parse::<u32>();
            let day = &d[8..10].parse::<u32>();
            matches!(m, Ok(1..=12)) && matches!(day, Ok(1..=31))
        }
}

/// 日记本存储: 按日归档的文件形态 (一天一 JSON 文件, 文件名 = 日期).
pub struct DiaryStore {
    root: PathBuf,
    clock: Arc<dyn Clock>,
}

impl DiaryStore {
    /// root = 日记根目录 (注入, 如 `<memory_path>/diary`); clock 注入 → 可测.
    pub fn new(root: impl Into<PathBuf>, clock: Arc<dyn Clock>) -> Self {
        Self { root: root.into(), clock }
    }

    fn day_path(&self, date: &str) -> PathBuf {
        self.root.join(format!("{date}.json"))
    }

    fn today(&self) -> String {
        let dt = self.clock.now();
        format!("{:04}-{:02}-{:02}", dt.year(), dt.month(), dt.day())
    }

    /// 归档一条到 clock 当前日 (同日追加, 跨日自动分文件).
    pub fn append(&self, source: &str, body: &str) -> Result<(), DiaryError> {
        let date = self.today();
        self.append_to(&date, source, body)
    }

    /// 归档一条到指定日 (显式日期入口; 非法日期显式拒绝).
    pub fn append_to(&self, date: &str, source: &str, body: &str) -> Result<(), DiaryError> {
        if !valid_date(date) {
            return Err(DiaryError::InvalidDate(date.to_string()));
        }
        if body.trim().is_empty() {
            return Err(DiaryError::EmptyContent);
        }
        let path = self.day_path(date);
        let mut page = read_page(&path, date);
        page.entries.push(DiaryEntry { source: source.to_string(), body: body.to_string() });
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, serde_json::to_vec_pretty(&page)?)?;
        Ok(())
    }

    /// 读一日日记; 无此日/读取失败 → None (诚实降级).
    pub fn read_day(&self, date: &str) -> Option<DayPage> {
        if !valid_date(date) {
            return None;
        }
        let p = read_page(&self.day_path(date), date);
        (!p.entries.is_empty()).then_some(p)
    }

    /// 全部有日记的日期 (字典序 = 时间序, 确定性); IO 失败 → 空.
    pub fn list_days(&self) -> Vec<String> {
        let Ok(rd) = std::fs::read_dir(&self.root) else { return Vec::new() };
        rd.flatten()
            .filter_map(|e| e.file_name().into_string().ok())
            .filter(|n| n.ends_with(".json") && valid_date(n.trim_end_matches(".json")))
            .map(|n| n.trim_end_matches(".json").to_string())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    /// 关键词检索 (大小写不敏感子串匹配); from/to 为可选日期范围 (闭区间).
    /// 空关键词 → 空结果 (防全量误召回); 非法范围日期 → 空结果.
    /// 顺序确定性: 日期升序, 日内存储序.
    pub fn search(&self, keyword: &str, from: Option<&str>, to: Option<&str>) -> Vec<DiaryHit> {
        if keyword.is_empty()
            || from.is_some_and(|f| !valid_date(f))
            || to.is_some_and(|t| !valid_date(t))
        {
            return Vec::new();
        }
        let kw = keyword.to_lowercase();
        self.list_days()
            .into_iter()
            .filter(|d| from.map_or(true, |f| d.as_str() >= f))
            .filter(|d| to.map_or(true, |t| d.as_str() <= t))
            .flat_map(|d| {
                let page = self.read_day(&d).unwrap_or_default();
                page.entries
                    .into_iter()
                    .filter(|e| e.body.to_lowercase().contains(&kw))
                    .map(move |e| DiaryHit { date: d.clone(), entry: e })
            })
            .collect()
    }

    /// 注入块: 近 n_days 个有日记的日 (最新优先), 字符预算内截断.
    /// 无日记/预算放不下头部+至少一条 → 空串 (诚实: 不注入半残块).
    pub fn recent_injection(&self, n_days: usize, budget_chars: usize) -> String {
        let days: Vec<String> = self.list_days().into_iter().rev().take(n_days).collect();
        if days.is_empty() {
            return String::new();
        }
        let newest = days[0].clone();
        let oldest = days.last().cloned().unwrap_or_default();
        let mut items: Vec<(String, DiaryEntry)> = Vec::new();
        for d in &days {
            if let Some(p) = self.read_day(d) {
                for e in p.entries {
                    items.push((d.clone(), e));
                }
            }
        }
        if items.is_empty() {
            return String::new();
        }
        items.reverse(); // 最新在前
        let header = format!("【日记】近段叙事 ({} ~ {})", oldest, newest);
        // 行格式 "\n· {date} ({source}): {body}" → 固定 8 字符 + 三个变长段
        let line_len = |d: &str, e: &DiaryEntry| {
            8 + d.chars().count() + e.source.chars().count() + e.body.chars().count()
        };
        let mark_len = 1 + TRUNCATION_MARK.chars().count(); // "\n…(已截断)"
        let mut kept: Vec<(String, DiaryEntry)> = Vec::new();
        let mut used = header.chars().count();
        let mut truncated = false;
        for (d, e) in items {
            if used + line_len(&d, &e) > budget_chars {
                truncated = !kept.is_empty();
                break;
            }
            used += line_len(&d, &e);
            kept.push((d, e));
        }
        // 截断标记也要在预算内: 不足则回退一条 (至少保一条, 超限则整块弃)
        while truncated && used + mark_len > budget_chars && !kept.is_empty() {
            let (d, e) = kept.pop().expect("kept 非空");
            used -= line_len(&d, &e);
            if kept.is_empty() {
                return String::new();
            }
        }
        if kept.is_empty() {
            return String::new();
        }
        let mut out = header;
        for (d, e) in &kept {
            out.push_str(&format!("\n· {} ({}): {}", d, e.source, e.body));
        }
        if truncated {
            out.push_str(&format!("\n{}", TRUNCATION_MARK));
        }
        out
    }
}

/// 注入块读取口 (渲染链消费侧 trait 口; 实接线延后 N14, 0 装 PASS).
///
/// infallible 约定: 无日记/预算不足/失败 → 空串, 消费方按诚实降级处理.
pub trait DiaryInjector: Send + Sync {
    /// 近 n_days 日日记注入块 (budget_chars 为字符预算上限).
    fn diary_injection(&self, n_days: usize, budget_chars: usize) -> String;
}

impl DiaryInjector for DiaryStore {
    fn diary_injection(&self, n_days: usize, budget_chars: usize) -> String {
        self.recent_injection(n_days, budget_chars)
    }
}

/// 读一日文件; 缺失/损坏 JSON → 空页 (诚实降级, 不 panic).
fn read_page(path: &Path, date: &str) -> DayPage {
    match std::fs::read(path) {
        Ok(bytes) => serde_json::from_slice(&bytes)
            .unwrap_or_else(|_| DayPage { date: date.to_string(), entries: Vec::new() }),
        Err(_) => DayPage { date: date.to_string(), entries: Vec::new() },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_core::clock::VirtualClock;
    use chrono::TimeZone;

    static COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

    fn tmp_root() -> PathBuf {
        let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let p = std::env::temp_dir().join(format!(
            "apeireth-diary-test-{}-{}-{}",
            std::process::id(),
            n,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let _ = std::fs::remove_dir_all(&p);
        p
    }

    fn clock_at(y: i32, m: u32, d: u32) -> Arc<VirtualClock> {
        Arc::new(VirtualClock::new(
            chrono::Utc.with_ymd_and_hms(y, m, d, 6, 0, 0).single().unwrap(),
        ))
    }

    #[test]
    fn append_archives_same_day_single_file() {
        let root = tmp_root();
        let store = DiaryStore::new(&root, clock_at(2026, 8, 16));
        store.append("user", "今天聊了渗透套件").unwrap();
        store.append("reflection", "主人状态不错").unwrap();
        let page = store.read_day("2026-08-16").expect("当日应有日记");
        assert_eq!(page.entries.len(), 2);
        assert_eq!(page.entries[0].source, "user");
        assert_eq!(page.entries[1].body, "主人状态不错");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn cross_day_separate_files_and_range_filter() {
        let root = tmp_root();
        let clock = clock_at(2026, 8, 16);
        let store = DiaryStore::new(&root, clock.clone());
        store.append("user", "第一天叙事").unwrap();
        clock.advance(chrono::Duration::days(1));
        store.append("user", "第二天叙事").unwrap();
        clock.advance(chrono::Duration::days(1));
        store.append("user", "第三天叙事").unwrap();
        assert_eq!(store.list_days(), vec!["2026-08-16", "2026-08-17", "2026-08-18"]);
        // 范围过滤 (闭区间)
        let mid = store.search("叙事", Some("2026-08-17"), Some("2026-08-17"));
        assert_eq!(mid.len(), 1);
        assert_eq!(mid[0].date, "2026-08-17");
        // 全量: 日期升序确定性
        let all = store.search("叙事", None, None);
        assert_eq!(all.iter().map(|h| h.date.as_str()).collect::<Vec<_>>(),
            vec!["2026-08-16", "2026-08-17", "2026-08-18"]);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn empty_day_and_empty_store() {
        let root = tmp_root();
        let store = DiaryStore::new(&root, clock_at(2026, 8, 16));
        assert!(store.read_day("2026-08-16").is_none(), "空日应为 None");
        assert!(store.list_days().is_empty());
        assert!(store.search("任意", None, None).is_empty());
        assert!(store.recent_injection(3, 500).is_empty(), "空库注入应为空串");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn search_case_insensitive_and_range_excludes() {
        let root = tmp_root();
        let clock = clock_at(2026, 8, 16);
        let store = DiaryStore::new(&root, clock.clone());
        store.append("user", "今天研究了 Rust 异步").unwrap();
        clock.advance(chrono::Duration::days(1));
        store.append("extractor", "rust 工具链升级").unwrap();
        assert_eq!(store.search("RUST", None, None).len(), 2, "大小写不敏感");
        assert_eq!(store.search("rust", Some("2026-08-17"), None).len(), 1, "范围排除 16 日");
        assert!(store.search("", None, None).is_empty(), "空关键词不召回");
        assert!(store.search("不存在的词", None, None).is_empty());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn injection_budget_truncation() {
        let root = tmp_root();
        let clock = clock_at(2026, 8, 16);
        let store = DiaryStore::new(&root, clock.clone());
        store.append("user", "第一条叙事内容").unwrap();
        clock.advance(chrono::Duration::days(1));
        store.append("user", "第二条叙事内容").unwrap();
        clock.advance(chrono::Duration::days(1));
        store.append("user", "第三条叙事内容").unwrap();
        let full = store.recent_injection(3, 500);
        assert!(full.starts_with("【日记】"), "块头: {full}");
        assert!(full.contains("第三条"), "最新优先: {full}");
        assert!(!full.contains(TRUNCATION_MARK), "预算充足不截断");
        let cut = store.recent_injection(3, 80);
        assert!(cut.contains(TRUNCATION_MARK), "预算不足应有截断标记: {cut}");
        assert!(cut.chars().count() <= 80, "预算硬上限: {}", cut.chars().count());
        assert!(store.recent_injection(3, 10).is_empty(), "预算过小 → 空串诚实降级");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn invalid_inputs_rejected() {
        let root = tmp_root();
        let store = DiaryStore::new(&root, clock_at(2026, 8, 16));
        for bad in ["2026-8-16", "2026/08/16", "2026-13-01", "2026-00-10", "../evil"] {
            assert!(store.append_to(bad, "x", "y").is_err(), "应拒绝非法日期: {bad}");
            assert!(store.read_day(bad).is_none());
        }
        assert!(store.append("user", "   ").is_err(), "空白正文应拒绝");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn injection_deterministic_same_input_same_output() {
        let root = tmp_root();
        let clock = clock_at(2026, 8, 16);
        let store = DiaryStore::new(&root, clock.clone());
        store.append("user", "确定性叙事").unwrap();
        clock.advance(chrono::Duration::days(1));
        store.append("reflection", "另一日叙事").unwrap();
        let a = store.recent_injection(2, 300);
        for _ in 0..5 {
            assert_eq!(store.recent_injection(2, 300), a, "同输入同输出");
        }
        let _ = std::fs::remove_dir_all(&root);
    }
}
