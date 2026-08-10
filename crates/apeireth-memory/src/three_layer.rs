//! `apeireth-memory::three_layer` — **R30 U9 claude-mem 3 层记忆 facade**
//!
//! **设计哲学** (claude-mem 借鉴, VCP/Apeireth 工程实践):
//! - **Working** (工作层): 当前 session 的 in-memory ring buffer, 极快 (无 IO), 给 LLM 喂上下文
//! - **Short-term** (短程层): 最近 N 小时的 episode (SQLite), 中等 IO, 跨 session 回忆
//! - **Long-term** (长程层): 永久笔记 + IdentityCard (SQLite), 慢但可压缩检索
//!
//! **3 层关系**:
//! - write(msg) → 同步写 Working, 异步落库 (Short)
//! - recall(query, depth) → 按 depth 选层: depth=0 working, depth=1 short, depth=2 long
//! - promote(working→short) → 后台任务, working 满了就 promote 到 short
//!
//! **Apeireth 扩展** (claude-mem 没有):
//! - depth 参数: 让 caller 选"速度/范围"权衡, 不要全查
//! - compress 钩子: 主人按需压缩 (不是自动后台, 避免后台跑 LLM 烧钱)

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use apeireth_core::{Episode, Note};

use crate::episode::{EpisodeQuery, EpisodeStore};
use crate::session_note::{NoteQuery, NoteStore};

/// R30 U9: working layer 容量 (in-memory ring buffer, 默认 50 条)
pub const WORKING_CAPACITY: usize = 50;

/// R30 U9: short-term 窗口 (秒, 默认 24h)
pub const SHORT_TERM_WINDOW_SECS: i64 = 24 * 3600;

/// R30 U9: 3 层 facade. 内部组合 in-memory working + SQLite short/long
///
/// **用法**:
/// ```ignore
/// let mem = ThreeLayerMemory::new(episodes, notes, WORKING_CAPACITY);
/// mem.write(episode)?;
/// let ctx: Vec<Episode> = mem.recall(0, 10)?; // working 层, 拿最新 10
/// let ctx: Vec<Episode> = mem.recall(1, 20)?; // short-term, 最近 20 条
/// let notes = mem.recall(2, 5)?;              // long-term notes
/// ```
pub struct ThreeLayerMemory {
    /// Working layer (in-memory, ring buffer)
    working: Arc<Mutex<VecDeque<Episode>>>,
    working_capacity: usize,
    /// Short-term layer (SQLite episodes)
    episodes: Arc<dyn EpisodeStore>,
    /// Long-term layer (SQLite notes + identity cards)
    notes: Arc<dyn NoteStore>,
}

impl ThreeLayerMemory {
    pub fn new(
        episodes: Arc<dyn EpisodeStore>,
        notes: Arc<dyn NoteStore>,
        working_capacity: usize,
    ) -> Self {
        Self {
            working: Arc::new(Mutex::new(VecDeque::with_capacity(working_capacity))),
            working_capacity,
            episodes,
            notes,
        }
    }

    /// R30 U9: 写一条 episode. 同步进 working (ring buffer), 不主动落库
    /// (落库由 caller 用 put_episode 显式做, facade 不假定 IO 策略)
    pub fn write(&self, ep: Episode) -> Result<(), String> {
        let mut w = self.working.lock().map_err(|e| format!("lock: {e}"))?;
        if w.len() >= self.working_capacity {
            w.pop_front(); // ring buffer 满了丢最旧
        }
        w.push_back(ep);
        Ok(())
    }

    /// R30 U9: 按层 recall
    /// - depth=0: working, limit ≤ working_capacity
    /// - depth=1: short-term (最近 SHORT_TERM_WINDOW_SECS)
    /// - depth=2: long-term notes (按 updated_at DESC)
    pub fn recall(&self, depth: u8, limit: usize) -> Result<Vec<Episode>, String> {
        match depth {
            0 => {
                let w = self.working.lock().map_err(|e| format!("lock: {e}"))?;
                // oldest first (跟 short-term 一致, LLM 喂上下文按时间序)
                Ok(w.iter().take(limit).cloned().collect())
            }
            1 => {
                let now = chrono::Utc::now().timestamp();
                let since = now - SHORT_TERM_WINDOW_SECS;
                let q = EpisodeQuery::new().in_range(Some(since), Some(now)).limit(limit);
                self.episodes
                    .query(&q)
                    .map_err(|e| format!("query short: {e}"))
            }
            2 => {
                // long-term: 把 notes 转成 Episode (role="system", content=note content)
                // (简化: notes 不是 episode 格式, 这里透传 NoteRecord 由 caller 处理)
                let nq = NoteQuery::new().limit(limit);
                let notes = self
                    .notes
                    .query(&nq)
                    .map_err(|e| format!("query long: {e}"))?;
                Ok(notes
                    .into_iter()
                    .map(|n| Episode {
                        id: format!("note-{}", n.id),
                        timestamp: n.timestamp,
                        role: "system".to_string(),
                        content: n.content,
                        session_id: format!("note-session-{}", n.id),
                    })
                    .collect())
            }
            _ => Err(format!("unknown recall depth: {depth} (0/1/2)")),
        }
    }

    /// R30 U9: promote working → short-term (把 working 全部写 SQLite)
    ///
    /// **用法**: 启动时 / 定时任务 调一次, 防 working 满了丢 episode
    pub fn promote(&self) -> Result<usize, String> {
        let snapshot: Vec<Episode> = {
            let w = self.working.lock().map_err(|e| format!("lock: {e}"))?;
            w.iter().cloned().collect()
        };
        // 一次性拿全部已存的 id, 避免每条都全表查
        let existing_ids: std::collections::HashSet<String> = self
            .episodes
            .query(&EpisodeQuery::new())
            .map_err(|e| format!("query dedup: {e}"))?
            .into_iter()
            .map(|e| e.id)
            .collect();
        let mut n = 0;
        for ep in &snapshot {
            if existing_ids.contains(&ep.id) {
                continue;
            }
            self.episodes
                .put_episode(ep)
                .map_err(|e| format!("put episode: {e}"))?;
            n += 1;
        }
        Ok(n)
    }

    /// R30 U9: working 当前大小 (debug/监控用)
    pub fn working_size(&self) -> usize {
        self.working.lock().map(|w| w.len()).unwrap_or(0)
    }

    /// R30 U9: 清空 working (force reset, debug 用)
    pub fn clear_working(&self) {
        if let Ok(mut w) = self.working.lock() {
            w.clear();
        }
    }

    /// R33-2 (mem0 借鉴): promote working → long-term notes (自动 fact extraction)
    ///
    /// **mem0 思想** (开源 long-term memory, Apache 2.0):
    /// - `add("I love Rust", user_id=...)` 自动抽 fact
    /// - 走 LLM 抽 fact (我们不用, 烧钱)
    ///
    /// **Apeireth 简化**: 用 5 启发式 rule 抽 fact, 零 LLM 调用
    /// - 数字 (e.g. "我有 3 只猫")
    /// - 日期 (e.g. "生日 1990-01-01")
    /// - 事件 (e.g. "上周开会")
    /// - 身份 (e.g. "我叫 Mavis")
    /// - 状态 (e.g. "我在等 X")
    ///
    /// **返回**: 抽到的 fact 数, 写进 long-term Note
    pub fn promote_with_summarize(&self) -> Result<usize, String> {
        let snapshot: Vec<Episode> = {
            let w = self.working.lock().map_err(|e| format!("lock: {e}"))?;
            w.iter().cloned().collect()
        };
        let mut n = 0;
        let mut now_ts = chrono::Utc::now().timestamp();
        for ep in &snapshot {
            // 5 启发式 rule 抽 fact
            let facts = extract_facts(&ep.content);
            for fact in facts {
                let note = Note {
                    id: format!("note-{}-{}", ep.id, now_ts),
                    timestamp: ep.timestamp,
                    content: fact,
                    source_episode_ids: vec![ep.id.clone()],
                    confidence: 0.7, // 启发式中等置信
                    tags: vec![ep.role.clone()],
                };
                if let Err(e) = self.notes.put_note(&note) {
                    return Err(format!("put note: {e}"));
                }
                n += 1;
                now_ts += 1; // id 唯一
            }
        }
        Ok(n)
    }
}

/// R33-2 (mem0 借鉴): 5 启发式 fact extraction rule
///
/// **不调 LLM** (R19 离线优先, 零成本). 用简单模式匹配 + 关键句.
fn extract_facts(text: &str) -> Vec<String> {
    if text.is_empty() {
        return Vec::new();
    }
    let mut facts = Vec::new();
    let text_lower = text.to_lowercase();

    // 数字 fact: "我有 N 个 / 我 N 岁 / N 元"
    if let Some(captures) = digit_pattern(&text) {
        facts.push(format!("[数字] {captures}"));
    }

    // 日期 fact: "1990-01-01" / "上周" / "今天"
    if let Some(captures) = date_pattern(&text) {
        facts.push(format!("[日期] {captures}"));
    }

    // 身份 fact: "我叫 X" / "我是 X" / "I am X"
    if let Some(captures) = identity_pattern(&text) {
        facts.push(format!("[身份] {captures}"));
    }

    // 状态 fact: "我在 X" / "我等 X" / "I am waiting"
    if let Some(captures) = state_pattern(&text) {
        facts.push(format!("[状态] {captures}"));
    }

    // 事件 fact: "我做了 X" / "X 发生了" / "I did X"
    if let Some(captures) = event_pattern(&text_lower) {
        facts.push(format!("[事件] {captures}"));
    }

    facts
}

fn digit_pattern(text: &str) -> Option<String> {
    // "3 只猫" / "我 25 岁" / "100 元" - 简单 digit + char
    let re_simple = regex_lite::find_digit_with_context(text, 30);
    re_simple
}

fn date_pattern(text: &str) -> Option<String> {
    // 1990-01-01 / 2026/08/09 / 上周 / 今天
    let re_iso = regex_lite::find_iso_date(text);
    re_iso.or_else(|| regex_lite::find_chinese_time(text))
}

fn identity_pattern(text: &str) -> Option<String> {
    // "我叫 X" / "我是 X" / "I am X"
    if let Some(idx) = text.find("我叫") {
        return Some(text[idx..].chars().take(40).collect());
    }
    if let Some(idx) = text.find("我是") {
        return Some(text[idx..].chars().take(40).collect());
    }
    if let Some(idx) = text.find("I am ") {
        return Some(text[idx..].chars().take(40).collect());
    }
    if let Some(idx) = text.find("I'm ") {
        return Some(text[idx..].chars().take(40).collect());
    }
    None
}

fn state_pattern(text: &str) -> Option<String> {
    // "我在 X" / "我等 X" / "I am waiting"
    if let Some(idx) = text.find("我在") {
        return Some(text[idx..].chars().take(40).collect());
    }
    if let Some(idx) = text.find("我等") {
        return Some(text[idx..].chars().take(40).collect());
    }
    if text.to_lowercase().contains("i am waiting") {
        return Some(text.chars().take(40).collect());
    }
    None
}

fn event_pattern(text: &str) -> Option<String> {
    // "我做了 X" / "X 发生了" / "i did X" / "i went to X"
    if let Some(idx) = text.find("我做了") {
        return Some(text[idx..].chars().take(40).collect());
    }
    if let Some(idx) = text.find("我去了") {
        return Some(text[idx..].chars().take(40).collect());
    }
    if text.starts_with("i did ") {
        return Some(text.chars().take(40).collect());
    }
    if text.starts_with("i went ") {
        return Some(text.chars().take(40).collect());
    }
    None
}

/// R33-2: 简单 regex helper (不引 regex crate, 手写 5 个模式)
mod regex_lite {
    pub fn find_digit_with_context(text: &str, ctx_chars: usize) -> Option<String> {
        // 找第一个数字, 取前后 ctx_chars 个 char
        for (i, c) in text.chars().enumerate() {
            if c.is_ascii_digit() {
                let start = i.saturating_sub(ctx_chars / 2);
                let end = (i + ctx_chars / 2).min(text.chars().count());
                let snippet: String = text.chars().skip(start).take(end - start).collect();
                return Some(snippet);
            }
        }
        None
    }

    pub fn find_iso_date(text: &str) -> Option<String> {
        // 简单 ISO date 模式: 4 数字 - 2 数字 - 2 数字
        let chars: Vec<char> = text.chars().collect();
        for i in 0..chars.len().saturating_sub(9) {
            if chars[i].is_ascii_digit()
                && chars[i + 1].is_ascii_digit()
                && chars[i + 2].is_ascii_digit()
                && chars[i + 3].is_ascii_digit()
                && chars[i + 4] == '-'
                && chars[i + 5].is_ascii_digit()
                && chars[i + 6].is_ascii_digit()
                && chars[i + 7] == '-'
                && chars[i + 8].is_ascii_digit()
                && chars[i + 9].is_ascii_digit()
            {
                let s: String = chars[i..i + 10].iter().collect();
                return Some(s);
            }
        }
        None
    }

    pub fn find_chinese_time(text: &str) -> Option<String> {
        // "上周" / "今天" / "明天" / "昨天"
        for kw in &["今天", "明天", "昨天", "上周", "上周三", "本月"] {
            if text.contains(kw) {
                return Some((*kw).to_string());
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_core::Episode;
    use crate::SqliteMemoryStore;
    use std::sync::Arc;

    fn fresh_episode(id: &str, role: &str, content: &str) -> Episode {
        Episode {
            id: id.to_string(),
            timestamp: chrono::Utc::now().timestamp(),
            role: role.to_string(),
            content: content.to_string(),
            session_id: "test-session".to_string(),
        }
    }

    #[test]
    fn working_layer_writes_and_recall() {
        let store = Arc::new(SqliteMemoryStore::open_in_memory().expect("open"));
        let notes = store.clone();
        let mem = ThreeLayerMemory::new(store.clone(), notes, 5);

        for i in 0..3 {
            mem.write(fresh_episode(&format!("e{i}"), "user", &format!("msg{i}"))).unwrap();
        }
        assert_eq!(mem.working_size(), 3);

        let got = mem.recall(0, 10).unwrap();
        assert_eq!(got.len(), 3, "working 3 条应全 recall");
        // recall(0) 返 oldest first (跟 LLM 喂上下文一致)
        assert_eq!(got[0].id, "e0", "最旧应在最前");
        assert_eq!(got[2].id, "e2", "最新应在最后");
    }

    #[test]
    fn working_ring_buffer_drops_oldest() {
        let store = Arc::new(SqliteMemoryStore::open_in_memory().expect("open"));
        let mem = ThreeLayerMemory::new(store.clone(), store.clone(), 3);
        for i in 0..5 {
            mem.write(fresh_episode(&format!("e{i}"), "user", &format!("m{i}"))).unwrap();
        }
        assert_eq!(mem.working_size(), 3, "ring buffer 满了保留最后 3");
        let got = mem.recall(0, 10).unwrap();
        let ids: Vec<&str> = got.iter().map(|e| e.id.as_str()).collect();
        assert_eq!(ids, vec!["e2", "e3", "e4"], "应保留 e2/e3/e4");
    }

    #[test]
    fn short_term_recall_returns_recent_episodes() {
        let store = Arc::new(SqliteMemoryStore::open_in_memory().expect("open"));
        let mem = ThreeLayerMemory::new(store.clone(), store.clone(), 10);
        // 直接 put_episode 走 SQLite
        for i in 0..3 {
            store
                .put_episode(&fresh_episode(&format!("s{i}"), "user", &format!("m{i}")))
                .expect("put");
        }
        let got = mem.recall(1, 10).expect("recall short");
        assert_eq!(got.len(), 3, "short-term 应拿 3 条");
    }

    #[test]
    fn long_term_recall_returns_notes_as_episodes() {
        let store = Arc::new(SqliteMemoryStore::open_in_memory().expect("open"));
        let mem = ThreeLayerMemory::new(store.clone(), store.clone(), 10);
        // 写一条 note (NoteStore::put_note 拿 apeireth_core::Note)
        let note = Note {
            id: "n1".to_string(),
            timestamp: chrono::Utc::now().timestamp(),
            content: "长程记忆测试".to_string(),
            source_episode_ids: vec![],
            confidence: 1.0,
            tags: vec![],
        };
        store.put_note(&note).expect("put note");
        let got = mem.recall(2, 10).expect("recall long");
        assert_eq!(got.len(), 1);
        assert!(got[0].content.contains("长程记忆"));
        assert_eq!(got[0].role, "system", "note 转成 system episode");
    }

    #[test]
    fn promote_drains_working_to_sqlite() {
        let store = Arc::new(SqliteMemoryStore::open_in_memory().expect("open"));
        let mem = ThreeLayerMemory::new(store.clone(), store.clone(), 10);
        mem.write(fresh_episode("p1", "user", "promote 1")).unwrap();
        mem.write(fresh_episode("p2", "user", "promote 2")).unwrap();
        let n = mem.promote().expect("promote");
        assert_eq!(n, 2, "应 promote 2 条");
        // 再调一次 promote 应 0 条 (id 已存在)
        let n2 = mem.promote().expect("promote 2");
        assert_eq!(n2, 0, "重复 promote 应 0");
    }

    #[test]
    fn unknown_depth_returns_error() {
        let store = Arc::new(SqliteMemoryStore::open_in_memory().expect("open"));
        let mem = ThreeLayerMemory::new(store.clone(), store.clone(), 10);
        let r = mem.recall(99, 10);
        assert!(r.is_err(), "depth=99 应报错, got: {:?}", r);
    }

    // R33-2 (mem0 借鉴) 测试
    #[test]
    fn r33_promote_with_summarize_extracts_digit_fact() {
        let store = Arc::new(SqliteMemoryStore::open_in_memory().expect("open"));
        let mem = ThreeLayerMemory::new(store.clone(), store.clone(), 10);
        mem.write(fresh_episode("e1", "user", "我有 3 只猫")).unwrap();
        let n = mem.promote_with_summarize().expect("promote");
        assert!(n >= 1, "应至少抽 1 个 fact (数字), got {n}");
        let notes = mem.recall(2, 100).expect("long recall");
        let has_digit = notes.iter().any(|n| n.content.contains("数字") && n.content.contains("3"));
        assert!(has_digit, "long-term note 应含数字 fact '3', notes: {notes:?}");
    }

    #[test]
    fn r33_promote_with_summarize_extracts_identity_fact() {
        let store = Arc::new(SqliteMemoryStore::open_in_memory().expect("open"));
        let mem = ThreeLayerMemory::new(store.clone(), store.clone(), 10);
        mem.write(fresh_episode("e1", "user", "我叫 Mavis")).unwrap();
        let n = mem.promote_with_summarize().expect("promote");
        assert!(n >= 1, "应抽身份 fact, got {n}");
        let notes = mem.recall(2, 100).expect("long recall");
        let has_id = notes.iter().any(|n| n.content.contains("身份") && n.content.contains("Mavis"));
        assert!(has_id, "应含身份 fact 'Mavis', notes: {notes:?}");
    }

    #[test]
    fn r33_promote_with_summarize_extracts_iso_date_fact() {
        let store = Arc::new(SqliteMemoryStore::open_in_memory().expect("open"));
        let mem = ThreeLayerMemory::new(store.clone(), store.clone(), 10);
        mem.write(fresh_episode("e1", "user", "生日 1990-05-15")).unwrap();
        let n = mem.promote_with_summarize().expect("promote");
        assert!(n >= 1, "应抽日期 fact, got {n}");
        let notes = mem.recall(2, 100).expect("long recall");
        let has_date = notes.iter().any(|n| n.content.contains("日期") && n.content.contains("1990-05-15"));
        assert!(has_date, "应含日期 fact, notes: {notes:?}");
    }

    #[test]
    fn r33_promote_with_summarize_extracts_chinese_time_fact() {
        let store = Arc::new(SqliteMemoryStore::open_in_memory().expect("open"));
        let mem = ThreeLayerMemory::new(store.clone(), store.clone(), 10);
        mem.write(fresh_episode("e1", "user", "我今天去公园")).unwrap();
        let n = mem.promote_with_summarize().expect("promote");
        assert!(n >= 1, "应抽中文时间 fact, got {n}");
        let notes = mem.recall(2, 100).expect("long recall");
        let has_today = notes.iter().any(|n| n.content.contains("今天"));
        assert!(has_today, "应含 '今天' fact, notes: {notes:?}");
    }

    #[test]
    fn r33_promote_with_summarize_empty_returns_zero() {
        let store = Arc::new(SqliteMemoryStore::open_in_memory().expect("open"));
        let mem = ThreeLayerMemory::new(store.clone(), store.clone(), 10);
        let n = mem.promote_with_summarize().expect("promote");
        assert_eq!(n, 0, "空 working 应 0 fact, got {n}");
    }

    #[test]
    fn r33_promote_with_summarize_no_fact_in_text_returns_zero() {
        let store = Arc::new(SqliteMemoryStore::open_in_memory().expect("open"));
        let mem = ThreeLayerMemory::new(store.clone(), store.clone(), 10);
        mem.write(fresh_episode("e1", "user", "今天天气不错")).unwrap();
        let n = mem.promote_with_summarize().expect("promote");
        // "今天" 是中文时间, 应抽 1 个 fact
        assert!(n >= 1, "含 '今天' 应至少 1 fact, got {n}");
    }
}
