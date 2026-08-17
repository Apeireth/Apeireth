//! `apeireth-companion::proactive_memory` — W4 记忆主动推销
//! (意图分诊升级为主动预载).
//!
//! ## 哲学 (per 主人 2026-08-18: m-flow 是被动的, 更好的是主动的)
//!
//! m-flow (现有 `morphology::classify`) 是被动: 查询来了选检索道.
//! W4 是主动: 每轮对话前她判断"这轮可能需要什么记忆",
//! 在用户尚未提问前就把候选记忆预载进 ContextAssembler.
//!
//! ## 三件套
//!
//! 1. `TopicPredictor::predict(cues)` — 预期话题分类 (纯函数, 0 LLM).
//!    输入: 最近 N 轮对话 + 当前时间 + 用户情绪/节律.
//!    输出: 排序的话题列表 (含置信度).
//! 2. `PreloadChannel` trait + 三道实现 (keyword / time / importance) +
//!    `CompositeChannel` 合并 + 去重 + 截断.
//! 3. `ProactiveBlock` — 新块类型 (ContextBlock 包装, name="proactive",
//!    `core=false`, 默认 cap=1500 chars) 注入 ContextAssembler.
//!
//! ## 预算协调 (与 ContextAssembler 6000 chars 协作)
//!
//! - 默认 cap = min(1500, total_budget / 4)
//! - 永不超预算 (ContextAssembler 总预算截断兜底)
//! - ContextAssembler 既有 API **0 改动**, 完全向后兼容
//!
//! ## 诚实登记 (与 N7 衔接)
//!
//! - N7 (morphology) 看查询 → 决定 CRAWL 深度 (被动).
//! - W4 (本模块) 看上下文+时间 → 预载候选记忆 (主动).
//! - 两条道在 ContextAssembler 相遇: 被动检索走 `memory` 块,
//!   主动预载走 `proactive` 块, 各自 cap_chars 隔离.

use std::collections::HashSet;

use chrono::{Datelike, NaiveDateTime, Timelike};

use crate::context::ContextBlock;

// ============================================================
// 预期话题分类器 (TopicPredictor)
// ============================================================

/// 话题线索 (0 LLM 输入; 启发式合成).
#[derive(Debug, Clone, Default)]
pub struct TopicCue {
    /// 最近 N 轮用户消息 (按时间正序, 最早在前).
    pub recent_user_messages: Vec<String>,
    /// 最近 N 轮助手消息 (同序).
    pub recent_assistant_messages: Vec<String>,
    /// 当前时间 (用于节律/情境).
    pub now: Option<NaiveDateTime>,
    /// 用户情绪信号 ("low" | "neutral" | "high" | 其它自由文本).
    pub user_mood: Option<String>,
}

/// 预期话题 + 置信度 (置信 ∈ [0, 1], 之和可 > 1 因话题独立触发).
#[derive(Debug, Clone, PartialEq)]
pub struct TopicHint {
    /// 话题键 (e.g. "exam_prep", "companion", "morning_briefing").
    pub topic: &'static str,
    /// 置信度 ∈ [0, 1].
    pub confidence: f32,
}

/// 分类结果 (按置信度倒序).
#[derive(Debug, Clone, Default)]
pub struct TopicPrediction {
    pub hints: Vec<TopicHint>,
}

impl TopicPrediction {
    /// 取 top-K 话题键 (跳过空置信).
    pub fn top_topics(&self, k: usize) -> Vec<&'static str> {
        self.hints
            .iter()
            .filter(|h| h.confidence > 0.0)
            .take(k)
            .map(|h| h.topic)
            .collect()
    }

    /// 取置信度最高的单一话题 (None = 无触发).
    pub fn primary(&self) -> Option<&'static str> {
        self.hints
            .iter()
            .filter(|h| h.confidence > 0.0)
            .max_by(|a, b| a.confidence.partial_cmp(&b.confidence).unwrap_or(std::cmp::Ordering::Equal))
            .map(|h| h.topic)
    }
}

// --- 话题规则 (确定性, 0 LLM) ---------------------------------------

/// 关键词 → 话题 (中文 + 拼音高频; 启发式).
const TOPIC_KEYWORDS: &[(&str, &str)] = &[
    // 学习/考试
    ("考试", "exam_prep"),
    ("备考", "exam_prep"),
    ("复习", "exam_prep"),
    ("线代", "exam_prep"),
    ("高数", "exam_prep"),
    ("作业", "study"),
    ("课题", "study"),
    // 项目/工作
    ("项目", "project"),
    ("部署", "project"),
    ("bug", "project"),
    ("代码", "project"),
    ("commit", "project"),
    // 陪伴/情绪
    ("累", "companion"),
    ("烦", "companion"),
    ("难过", "companion"),
    ("孤独", "companion"),
    ("陪我", "companion"),
    ("抱抱", "companion"),
    // 计划/约定
    ("计划", "plan"),
    ("安排", "plan"),
    ("约定", "plan"),
    ("明天", "plan"),
    // 投资/金融 (与 TP25/26 套件衔接)
    ("股票", "invest"),
    ("基金", "invest"),
    ("仓位", "invest"),
    ("行情", "invest"),
    // 反思/日记
    ("日记", "reflection"),
    ("反思", "reflection"),
    ("回顾", "reflection"),
];

/// 时间锚 → 话题 (节律: 早晨/晚间/周末).
const TIME_ANCHORS: &[(u32, u32, &str)] = &[
    // (start_hour, end_hour, topic) — end_hour 排他
    (6, 9, "morning_briefing"),
    (21, 24, "evening_recap"),
    (0, 6, "late_night_checkin"),
];

/// 情绪锚 → 话题.
const MOOD_ANCHORS: &[(&str, &str)] = &[
    ("low", "companion"),
    ("sad", "companion"),
    ("tired", "companion"),
    ("high", "study"),
    ("excited", "study"),
];

fn keyword_hits(text: &str) -> Vec<(&'static str, f32)> {
    let lower = text.to_lowercase();
    let mut hits: Vec<(&'static str, f32)> = Vec::new();
    for (kw, topic) in TOPIC_KEYWORDS {
        if lower.contains(kw) {
            // 重复出现累积置信度 (cap 0.6)
            let n = lower.matches(kw).count() as f32;
            let conf = (n * 0.35).min(0.6);
            hits.push((topic, conf));
        }
    }
    hits
}

fn aggregate_topic_confidence(hits: &[(&'static str, f32)]) -> Vec<TopicHint> {
    // 同话题合并 (取 max, 因同一信号不应重复加; BTreeMap 保证迭代次序确定 → 测试可重复)
    let mut acc: std::collections::BTreeMap<&'static str, f32> = std::collections::BTreeMap::new();
    for (topic, conf) in hits {
        let e = acc.entry(topic).or_insert(0.0);
        if *conf > *e {
            *e = *conf;
        }
    }
    let mut v: Vec<TopicHint> = acc
        .into_iter()
        .map(|(topic, confidence)| TopicHint { topic, confidence })
        .collect();
    // 主排序: confidence desc; 次排序: topic 名字典序 → 严格确定性
    v.sort_by(|a, b| {
        b.confidence
            .partial_cmp(&a.confidence)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.topic.cmp(b.topic))
    });
    v
}

fn time_topic(now: NaiveDateTime) -> Option<TopicHint> {
    let h = now.hour();
    let weekday = now.weekday().num_days_from_monday(); // 0=Mon, 6=Sun
    for (start, end, topic) in TIME_ANCHORS {
        if h >= *start && h < *end {
            // 周末额外加权 morning_briefing
            let conf = if weekday >= 5 && *topic == "morning_briefing" {
                0.35
            } else {
                0.25
            };
            return Some(TopicHint { topic, confidence: conf });
        }
    }
    None
}

fn mood_topic(mood: &str) -> Option<TopicHint> {
    let lower = mood.to_lowercase();
    for (anchor, topic) in MOOD_ANCHORS {
        if lower.contains(anchor) {
            return Some(TopicHint { topic, confidence: 0.4 });
        }
    }
    None
}

/// 预期话题分类 (主入口, 纯函数).
pub fn predict_topic(cue: &TopicCue) -> TopicPrediction {
    let mut all_hits: Vec<(&'static str, f32)> = Vec::new();

    // 1. 关键词信号 (用户 + 助手, 各取最近 5 轮; 正序拼接)
    let take_tail = |v: &[String], n: usize| -> Vec<String> {
        let start = v.len().saturating_sub(n);
        v[start..].to_vec()
    };
    let user_window = take_tail(&cue.recent_user_messages, 5);
    let asst_window = take_tail(&cue.recent_assistant_messages, 5);
    for t in user_window.iter().chain(asst_window.iter()) {
        for hit in keyword_hits(t) {
            all_hits.push(hit);
        }
    }

    // 2. 时间锚
    if let Some(now) = cue.now {
        if let Some(h) = time_topic(now) {
            all_hits.push((h.topic, h.confidence));
        }
    }

    // 3. 情绪锚
    if let Some(m) = &cue.user_mood {
        if let Some(h) = mood_topic(m) {
            all_hits.push((h.topic, h.confidence));
        }
    }

    TopicPrediction { hints: aggregate_topic_confidence(&all_hits) }
}

// ============================================================
// 预载检索道 (PreloadChannel)
// ============================================================

/// 候选记忆条目 (轻量: 内容 + 时间戳 + 重要性; 用于排序 + 去重).
#[derive(Debug, Clone)]
pub struct MemoryCandidate {
    pub content: String,
    pub timestamp: i64, // 秒
    pub importance: u8, // 0..=10
}

/// 预载检索道 trait — 不同策略可替换 (keyword / time / importance / 复合).
pub trait PreloadChannel: Send + Sync {
    /// 给定预期话题, 从候选池拉 top_k 条.
    /// 返回按相关性倒序 (内部策略自行定义).
    fn fetch(
        &self,
        topics: &[&str],
        candidates: &[MemoryCandidate],
        top_k: usize,
    ) -> Vec<MemoryCandidate>;
}

/// 关键词道: 候选内容含话题键 → 命中 (简单 substring, 0 NLP 库).
pub struct KeywordChannel;

impl KeywordChannel {
    /// 给定话题键, 反查其关键词表 (Ponytail: 复用 TOPIC_KEYWORDS, 0 新表).
    pub fn keywords_for_topic(topic: &str) -> Vec<&'static str> {
        TOPIC_KEYWORDS
            .iter()
            .filter(|(_, t)| *t == topic)
            .map(|(kw, _)| *kw)
            .collect()
    }
}

impl PreloadChannel for KeywordChannel {
    fn fetch(
        &self,
        topics: &[&str],
        candidates: &[MemoryCandidate],
        top_k: usize,
    ) -> Vec<MemoryCandidate> {
        if topics.is_empty() || candidates.is_empty() || top_k == 0 {
            return Vec::new();
        }
        // 话题 → 关键词列表 (空话题仍可被显式 raw 关键词命中 — fallback)
        let mut keywords: Vec<&str> = Vec::new();
        for t in topics {
            for kw in Self::keywords_for_topic(t) {
                if !keywords.contains(&kw) {
                    keywords.push(kw);
                }
            }
        }
        if keywords.is_empty() {
            // 无映射时, 直接用话题键作为关键词 (允许调用方传 raw 片段)
            keywords = topics.to_vec();
        }
        let mut scored: Vec<(usize, MemoryCandidate)> = Vec::new();
        for c in candidates.iter() {
            let lc = c.content.to_lowercase();
            let hits = keywords.iter().filter(|k| lc.contains(&k.to_lowercase())).count();
            if hits > 0 {
                scored.push((hits, c.clone()));
            }
        }
        scored.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| b.1.importance.cmp(&a.1.importance)));
        scored.into_iter().take(top_k).map(|(_, c)| c).collect()
    }
}

/// 时间道: 最近 N 分钟/小时 (按 timestamp 倒序, 与话题无关 — 兜底).
pub struct TimeChannel {
    pub within_secs: i64,
}

impl PreloadChannel for TimeChannel {
    fn fetch(
        &self,
        _topics: &[&str],
        candidates: &[MemoryCandidate],
        top_k: usize,
    ) -> Vec<MemoryCandidate> {
        if top_k == 0 || candidates.is_empty() {
            return Vec::new();
        }
        // 取最新 — caller 应预排序或传最新为先; 此处不假定, 自排序
        let mut sorted: Vec<MemoryCandidate> = candidates.to_vec();
        sorted.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        // within_secs 仅作信息记录 (无 anchor 时间, 故只截 top_k)
        let _ = self.within_secs;
        sorted.into_iter().take(top_k).collect()
    }
}

/// 重要性道: importance ≥ 阈值 (≥8 主人惯例, 见 assemble.rs L1 规则).
pub struct ImportanceChannel {
    pub threshold: u8,
}

impl PreloadChannel for ImportanceChannel {
    fn fetch(
        &self,
        _topics: &[&str],
        candidates: &[MemoryCandidate],
        top_k: usize,
    ) -> Vec<MemoryCandidate> {
        if top_k == 0 || candidates.is_empty() {
            return Vec::new();
        }
        let mut sorted: Vec<MemoryCandidate> = candidates
            .iter()
            .filter(|c| c.importance >= self.threshold)
            .cloned()
            .collect();
        sorted.sort_by(|a, b| b.importance.cmp(&a.importance).then(b.timestamp.cmp(&a.timestamp)));
        sorted.into_iter().take(top_k).collect()
    }
}

/// 复合道: 多道并行拉 → 去重 (按 content) → 截 top_k.
pub struct CompositeChannel {
    pub channels: Vec<Box<dyn PreloadChannel>>,
}

impl PreloadChannel for CompositeChannel {
    fn fetch(
        &self,
        topics: &[&str],
        candidates: &[MemoryCandidate],
        top_k: usize,
    ) -> Vec<MemoryCandidate> {
        if top_k == 0 || candidates.is_empty() {
            return Vec::new();
        }
        let mut seen: HashSet<String> = HashSet::new();
        let mut out: Vec<MemoryCandidate> = Vec::new();
        for ch in &self.channels {
            for c in ch.fetch(topics, candidates, top_k) {
                if seen.insert(c.content.clone()) {
                    out.push(c);
                    if out.len() >= top_k {
                        return out;
                    }
                }
            }
        }
        out
    }
}

impl std::fmt::Debug for CompositeChannel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CompositeChannel")
            .field("channel_count", &self.channels.len())
            .finish()
    }
}

/// 默认复合道: keyword + time(within_secs=3600) + importance(threshold=8).
pub fn default_composite_channel() -> CompositeChannel {
    CompositeChannel {
        channels: vec![
            Box::new(KeywordChannel),
            Box::new(TimeChannel { within_secs: 3600 }),
            Box::new(ImportanceChannel { threshold: 8 }),
        ],
    }
}

// ============================================================
// ProactiveBlock — 注入块 + 预算协调
// ============================================================

/// 主动预载块 (ContextBlock 包装 + 触发话题标签).
#[derive(Debug, Clone)]
pub struct ProactiveBlock {
    /// 实际注入 ContextAssembler 的块.
    pub block: ContextBlock,
    /// 触发此预载的话题 (调试 / 报告).
    pub topics: Vec<String>,
}

impl ProactiveBlock {
    pub fn name(&self) -> &'static str {
        self.block.name
    }
    pub fn content(&self) -> &str {
        &self.block.content
    }
    pub fn char_count(&self) -> usize {
        self.block.content.chars().count()
    }
}

/// 把候选记忆渲染成「主动预载证据」块 (仿 memory_injection 风格: 闭世界编号 + 反幻觉).
///
/// `max_chars` 严格截断 (含反幻觉尾注; 触发: 内容已满 → 仅留尾注).
pub fn render_proactive_content(entries: &[MemoryCandidate], max_chars: usize) -> String {
    if entries.is_empty() || max_chars < 80 {
        return String::new();
    }
    // 尾部反幻觉指令 (必含, 至少 60 字)
    const FOOTER: &str = "\n规则: 仅当用户提到上述话题时引用; 不主动说「我记得」— 那是编造。";
    let footer_chars = FOOTER.chars().count();
    let body_budget = max_chars.saturating_sub(footer_chars + 16); // 16 字预留头部

    let mut body = String::from("[主动预载 — 预期话题候选记忆, 非用户已提需求]\n");
    let mut used = body.chars().count();
    for (i, e) in entries.iter().enumerate() {
        let line = format!("{}. {}\n", i + 1, truncate(&e.content, 120));
        let lc = line.chars().count();
        if used + lc > body_budget {
            break;
        }
        body.push_str(&line);
        used += lc;
    }
    body.push_str(FOOTER);
    body
}

fn truncate(s: &str, n: usize) -> String {
    s.chars().take(n).collect::<String>()
}

/// 主入口: 给定话题 + 候选池 + 预算 → ProactiveBlock (可直接 push 进 ContextAssembler).
///
/// `max_chars` 建议 ≤ ContextAssembler 总预算的 1/4 (默认 1500; 6k 预算下)。
/// 0 装: 调用方传更大值也不会破坏 ContextAssembler — 总预算截断兜底.
pub fn build_proactive_block(
    cue: &TopicCue,
    candidates: &[MemoryCandidate],
    channel: &dyn PreloadChannel,
    max_chars: usize,
) -> ProactiveBlock {
    let prediction = predict_topic(cue);
    let topics: Vec<String> = prediction
        .top_topics(3)
        .into_iter()
        .map(|s| s.to_string())
        .collect();
    let topic_refs: Vec<&str> = topics.iter().map(|s| s.as_str()).collect();
    let fetched = channel.fetch(&topic_refs, candidates, 6);
    let content = render_proactive_content(&fetched, max_chars);
    let block = ContextBlock::new("proactive", content).with_cap(max_chars);
    ProactiveBlock { block, topics }
}

/// 预算协调助手: 给定 ContextAssembler 总预算 → 推荐 proactive cap.
pub fn recommend_proactive_cap(total_budget_chars: usize) -> usize {
    // 总预算 1/4, 钳位 [400, 2000]; < 400 时全预算留给核心块
    let q = total_budget_chars / 4;
    if total_budget_chars < 400 {
        0
    } else {
        q.clamp(400, 2000)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn dt(y: i32, m: u32, d: u32, h: u32, mi: u32) -> NaiveDateTime {
        use chrono::{NaiveDate, NaiveTime};
        NaiveDateTime::new(
            NaiveDate::from_ymd_opt(y, m, d).unwrap(),
            NaiveTime::from_hms_opt(h, mi, 0).unwrap(),
        )
    }

    // --- TopicPredictor ---

    #[test]
    fn predict_exam_keyword() {
        let cue = TopicCue {
            recent_user_messages: vec!["明天要考线代, 我还没复习".into()],
            now: Some(dt(2026, 8, 18, 20, 0)),
            ..Default::default()
        };
        let p = predict_topic(&cue);
        let topics = p.top_topics(3);
        assert!(topics.contains(&"exam_prep"), "应识别考试话题: {topics:?}");
    }

    #[test]
    fn predict_companion_for_low_mood() {
        let cue = TopicCue {
            user_mood: Some("low".into()),
            ..Default::default()
        };
        let p = predict_topic(&cue);
        assert!(p.top_topics(3).contains(&"companion"));
    }

    #[test]
    fn predict_morning_briefing_weekday() {
        let cue = TopicCue {
            now: Some(dt(2026, 8, 18, 7, 30)), // 周二早晨
            ..Default::default()
        };
        let p = predict_topic(&cue);
        assert!(p.top_topics(3).contains(&"morning_briefing"));
    }

    #[test]
    fn predict_evening_recap() {
        let cue = TopicCue {
            now: Some(dt(2026, 8, 18, 22, 0)),
            ..Default::default()
        };
        let p = predict_topic(&cue);
        assert!(p.top_topics(3).contains(&"evening_recap"));
    }

    #[test]
    fn predict_late_night() {
        let cue = TopicCue {
            now: Some(dt(2026, 8, 18, 2, 0)),
            ..Default::default()
        };
        let p = predict_topic(&cue);
        assert!(p.top_topics(3).contains(&"late_night_checkin"));
    }

    #[test]
    fn predict_combined_keyword_time_mood() {
        let cue = TopicCue {
            recent_user_messages: vec!["今天好累".into()],
            user_mood: Some("tired".into()),
            now: Some(dt(2026, 8, 18, 23, 0)), // 晚上
            ..Default::default()
        };
        let p = predict_topic(&cue);
        let topics = p.top_topics(3);
        assert!(topics.contains(&"companion"), "情绪+文本双触发: {topics:?}");
        assert!(topics.contains(&"evening_recap"), "时间触发: {topics:?}");
    }

    #[test]
    fn predict_empty_cue_no_panic() {
        let p = predict_topic(&TopicCue::default());
        assert!(p.hints.is_empty() || p.top_topics(3).is_empty());
    }

    #[test]
    fn predict_is_deterministic() {
        let cue = TopicCue {
            recent_user_messages: vec!["明天考试".into()],
            now: Some(dt(2026, 8, 18, 10, 0)),
            user_mood: Some("neutral".into()),
            ..Default::default()
        };
        let a = predict_topic(&cue);
        let b = predict_topic(&cue);
        assert_eq!(a.top_topics(3), b.top_topics(3));
    }

    #[test]
    fn predict_primary_returns_highest_or_none() {
        let cue = TopicCue {
            recent_user_messages: vec!["考试 考试".into()],
            now: Some(dt(2026, 8, 18, 10, 0)),
            ..Default::default()
        };
        let p = predict_topic(&cue);
        assert_eq!(p.primary(), Some("exam_prep"));

        let empty = TopicCue::default();
        assert!(predict_topic(&empty).primary().is_none());
    }

    #[test]
    fn predict_keyword_aggregation_takes_max_not_sum() {
        // 同一话题多次出现不应累积 (Ponytail: max, 防过度自信)
        let cue = TopicCue {
            recent_user_messages: vec!["考试 考试 考试 考试".into()],
            now: Some(dt(2026, 8, 18, 10, 0)),
            ..Default::default()
        };
        let p = predict_topic(&cue);
        let exam_conf = p
            .hints
            .iter()
            .find(|h| h.topic == "exam_prep")
            .map(|h| h.confidence)
            .unwrap_or(0.0);
        assert!(exam_conf <= 0.6, "置信度应被 max 合并而非 sum: {exam_conf}");
    }

    // --- PreloadChannel ---

    fn cand(content: &str, ts: i64, imp: u8) -> MemoryCandidate {
        MemoryCandidate { content: content.into(), timestamp: ts, importance: imp }
    }

    #[test]
    fn keyword_channel_matches_topic_in_content() {
        let ch = KeywordChannel;
        let cands = vec![
            cand("明天要考线代", 1, 5),
            cand("咖啡好喝", 2, 5),
            cand("高数作业还没写", 3, 5),
        ];
        let out = ch.fetch(&["exam_prep"], &cands, 5);
        assert_eq!(out.len(), 2, "应命中含 exam_prep 关键词的两条: {:?}", out.iter().map(|c| &c.content).collect::<Vec<_>>());
        assert!(out.iter().any(|c| c.content.contains("线代")));
        assert!(out.iter().any(|c| c.content.contains("高数")));
    }

    #[test]
    fn keyword_channel_no_match_returns_empty() {
        let ch = KeywordChannel;
        let cands = vec![cand("完全无关", 1, 5)];
        let out = ch.fetch(&["invest"], &cands, 5);
        assert!(out.is_empty());
    }

    #[test]
    fn keyword_channel_top_k_respected() {
        let ch = KeywordChannel;
        let cands: Vec<MemoryCandidate> = (0..20)
            .map(|i| cand(&format!("考试 第{i}次"), i, 5))
            .collect();
        let out = ch.fetch(&["exam_prep"], &cands, 3);
        assert_eq!(out.len(), 3);
    }

    #[test]
    fn time_channel_sorts_by_timestamp_desc() {
        let ch = TimeChannel { within_secs: 3600 };
        let cands = vec![
            cand("old", 100, 5),
            cand("newest", 999, 5),
            cand("mid", 500, 5),
        ];
        let out = ch.fetch(&[], &cands, 3);
        assert_eq!(out[0].content, "newest");
        assert_eq!(out[1].content, "mid");
        assert_eq!(out[2].content, "old");
    }

    #[test]
    fn importance_channel_filters_below_threshold() {
        let ch = ImportanceChannel { threshold: 8 };
        let cands = vec![
            cand("low imp", 1, 3),
            cand("high imp 1", 2, 9),
            cand("high imp 2", 3, 10),
            cand("mid imp", 4, 7),
        ];
        let out = ch.fetch(&[], &cands, 5);
        assert_eq!(out.len(), 2);
        assert!(out.iter().all(|c| c.importance >= 8));
        // 排序: importance desc → 高 imp 2 (10) 在前
        assert_eq!(out[0].content, "high imp 2");
    }

    #[test]
    fn composite_channel_dedupes_by_content() {
        let ch = default_composite_channel();
        let cands = vec![
            cand("主人明天要考线代", 100, 9),
            cand("主人明天要考线代", 101, 9), // 内容重复
            cand("咖啡好喝", 50, 3),
            cand("复盘: 项目上线", 200, 10), // 高重要性
        ];
        let out = ch.fetch(&["exam_prep"], &cands, 10);
        let unique_contents: HashSet<String> = out.iter().map(|c| c.content.clone()).collect();
        assert_eq!(unique_contents.len(), out.len(), "应去重: {out:?}");
        // keyword + importance 至少触发
        assert!(out.iter().any(|c| c.content.contains("线代")));
        assert!(out.iter().any(|c| c.content.contains("复盘")));
    }

    #[test]
    fn composite_channel_top_k_limit() {
        let ch = default_composite_channel();
        let cands: Vec<MemoryCandidate> = (0..20)
            .map(|i| cand(&format!("考试 条目 {i}"), i, 5))
            .collect();
        let out = ch.fetch(&["exam_prep"], &cands, 4);
        assert_eq!(out.len(), 4);
    }

    #[test]
    fn composite_channel_empty_inputs() {
        let ch = default_composite_channel();
        assert!(ch.fetch(&[], &[], 5).is_empty());
        assert!(ch.fetch(&["x"], &[], 5).is_empty());
    }

    // --- render & build_proactive_block ---

    #[test]
    fn render_empty_returns_empty() {
        assert!(render_proactive_content(&[], 1000).is_empty());
        assert!(render_proactive_content(&[cand("x", 1, 5)], 50).is_empty()); // max_chars < 80
    }

    #[test]
    fn render_includes_footer_and_numbering() {
        let s = render_proactive_content(
            &[cand("主人换元法常忘", 1, 9), cand("线代作业明天交", 2, 8)],
            500,
        );
        assert!(s.contains("[主动预载"));
        assert!(s.contains("1. 主人换元法常忘"));
        assert!(s.contains("2. 线代作业明天交"));
        assert!(s.contains("仅当用户提到上述话题时引用"));
        assert!(s.contains("不主动说「我记得」") || s.contains("禁止说「我记得」"));
    }

    #[test]
    fn render_truncates_long_entries() {
        let s = render_proactive_content(&[cand(&"x".repeat(500), 1, 5)], 300);
        // 500 字符条目被截到 120
        assert!(s.matches('x').count() <= 120, "应截断: {}", s.matches('x').count());
    }

    #[test]
    fn build_proactive_block_returns_valid_block() {
        let cue = TopicCue {
            recent_user_messages: vec!["明天要考试".into()],
            now: Some(dt(2026, 8, 18, 10, 0)),
            ..Default::default()
        };
        let cands = vec![
            cand("线代重点", 100, 9),
            cand("换元法", 50, 7),
        ];
        let ch = KeywordChannel;
        let pb = build_proactive_block(&cue, &cands, &ch, 800);
        assert_eq!(pb.name(), "proactive");
        assert!(!pb.content().is_empty());
        assert!(pb.char_count() <= 800, "应被 cap 约束: {}", pb.char_count());
        assert!(pb.topics.contains(&"exam_prep".to_string()));
    }

    #[test]
    fn build_proactive_block_empty_candidates_no_panic() {
        let cue = TopicCue::default();
        let pb = build_proactive_block(&cue, &[], &KeywordChannel, 800);
        // 空候选 → 内容含头部与尾注或空 (不大于 cap)
        assert!(pb.char_count() <= 800);
    }

    // --- 预算协调 ---

    #[test]
    fn recommend_cap_small_budget_returns_zero() {
        assert_eq!(recommend_proactive_cap(0), 0);
        assert_eq!(recommend_proactive_cap(200), 0);
        assert_eq!(recommend_proactive_cap(399), 0);
    }

    #[test]
    fn recommend_cap_normal_budget() {
        assert_eq!(recommend_proactive_cap(6000), 1500);
        assert_eq!(recommend_proactive_cap(4000), 1000);
    }

    #[test]
    fn recommend_cap_clamped_upper() {
        // 极大预算 → 钳到 2000
        assert_eq!(recommend_proactive_cap(100_000), 2000);
        assert_eq!(recommend_proactive_cap(20_000), 2000);
    }

    // --- 向后兼容: ContextAssembler API 不破坏 ---

    #[test]
    fn proactive_block_pushes_into_existing_assembler() {
        use crate::context::{ContextAssembler, ContextBlock};
        let pb = ProactiveBlock {
            block: ContextBlock::new("proactive", "test proactive content"),
            topics: vec!["exam_prep".into()],
        };
        let mut asm = ContextAssembler::new(6000)
            .push(ContextBlock::new("identity", "persona core").core(true))
            .push(ContextBlock::new("state", "current state block"));
        asm = asm.push(pb.block);
        let blocks = asm.assemble_budgeted_blocks();
        let names: Vec<&str> = blocks.iter().map(|b| b.name).collect();
        assert!(names.contains(&"identity"), "核心块应在: {names:?}");
        assert!(names.contains(&"proactive"), "主动预载块应在: {names:?}");
        assert!(names.contains(&"state"), "既有 state 块不破坏: {names:?}");
    }

    #[test]
    fn proactive_block_truncated_by_assembler_budget() {
        use crate::context::{ContextAssembler, ContextBlock};
        let huge = "x".repeat(5000);
        let pb_block = ContextBlock::new("proactive", huge.clone()).with_cap(500);
        let asm = ContextAssembler::new(600)
            .push(ContextBlock::new("identity", "core").core(true))
            .push(pb_block);
        let blocks = asm.assemble_budgeted_blocks();
        let pb = blocks.iter().find(|b| b.name == "proactive").unwrap();
        assert!(pb.content.chars().count() <= 500, "cap 应被尊重: {}", pb.content.chars().count());
    }

    #[test]
    fn old_api_still_works_without_proactive() {
        // 旧用法: 只 push 既有块, 不带 proactive → 不破坏
        use crate::context::{ContextAssembler, ContextBlock};
        let asm = ContextAssembler::new(1000)
            .push(ContextBlock::new("identity", "core").core(true))
            .push(ContextBlock::new("memory", "mem block").with_cap(400));
        let blocks = asm.assemble_budgeted_blocks();
        assert_eq!(blocks.len(), 2);
        assert!(blocks.iter().all(|b| !b.content.is_empty()));
    }

    #[test]
    fn proactive_and_other_blocks_share_budget() {
        use crate::context::{ContextAssembler, ContextBlock};
        // 总预算 1000: identity(核心 200) + state(300) + memory(300) + proactive(500 → 超)
        let asm = ContextAssembler::new(1000)
            .push(ContextBlock::new("identity", "x".repeat(200)).core(true))
            .push(ContextBlock::new("state", "y".repeat(300)).with_cap(300))
            .push(ContextBlock::new("memory", "z".repeat(300)).with_cap(300))
            .push(ContextBlock::new("proactive", "w".repeat(500)).with_cap(500));
        let blocks = asm.assemble_budgeted_blocks();
        let total: usize = blocks.iter().map(|b| b.content.chars().count()).sum();
        assert!(total <= 1000, "总占用不应超预算: {total}");
        // 核心块保护
        let id = blocks.iter().find(|b| b.name == "identity").unwrap();
        assert_eq!(id.content.chars().count(), 200, "核心块不应被截");
    }
}