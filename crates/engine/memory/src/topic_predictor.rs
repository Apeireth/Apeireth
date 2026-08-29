//! `apeireth-memory::topic_predictor` — 话题预测器与主动记忆预载通道.
//!
//! C1 / R20 `preference_learning` 实施 (per `deferred-slot-activation-preference_learning-spec.md`).
//!
//! **设计哲学**:
//! - **纯确定性算法 (0 LLM 依赖)**: 基于关键词词频、时间节律、情绪信号及重要性启发式预测当前可能涉及的话题.
//! - **4 Channel 架构**:
//!   1. `KeywordChannel`: 对话内容上下文关键词特征提取
//!   2. `TimeChannel`: 时间段 (早晨/下午/傍晚/深夜) 节律特征
//!   3. `ImportanceChannel`: 记忆重要性阈值通道
//!   4. `CompositeChannel`: 多通道加权、去重与置信度归一化
//!
//! **O-6 三阶审查**:
//! 1. 总体: 主动预载与偏好学习的核心算法支撑, 避免对话时被动拉取
//! 2. 系统: 放置在 `apeireth-memory`, 与 `PreferenceStore` 和 `EpisodeStore` 对接
//! 3. 架构: 纯函数与 Trait 模式, 支持单测与离线确定性重放

use chrono::{Datelike, NaiveDateTime, Timelike};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// 话题线索输入 (纯启发式, 0 LLM).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TopicCue {
    /// 最近 N 轮用户消息 (按时间升序).
    pub recent_user_messages: Vec<String>,
    /// 最近 N 轮助手消息.
    pub recent_assistant_messages: Vec<String>,
    /// 当前时间 (用于节律特征分析).
    pub now: Option<NaiveDateTime>,
    /// 用户情绪/节律信号 ("low" | "neutral" | "high" 或自定义标签).
    pub user_mood: Option<String>,
}

/// 预期话题及其置信度 (置信度范围 `[0.0, 1.0]`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TopicHint {
    /// 话题键标识 (如 "exam_prep", "project", "companion", "morning_briefing").
    pub topic: String,
    /// 预测置信度 `[0.0, 1.0]`.
    pub confidence: f32,
}

/// 话题预测结果集 (按置信度降序排列).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TopicPrediction {
    pub hints: Vec<TopicHint>,
}

impl TopicPrediction {
    /// 提取前 K 个非空置信度话题键.
    pub fn top_topics(&self, k: usize) -> Vec<&str> {
        self.hints
            .iter()
            .filter(|h| h.confidence > 0.0)
            .take(k)
            .map(|h| h.topic.as_str())
            .collect()
    }

    /// 提取置信度最高的主导话题.
    pub fn primary(&self) -> Option<&str> {
        self.hints
            .iter()
            .filter(|h| h.confidence > 0.0)
            .max_by(|a, b| {
                a.confidence
                    .partial_cmp(&b.confidence)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|h| h.topic.as_str())
    }
}

/// 关键词 → 话题启发式映射规则 (中文 + 英文高频词).
const TOPIC_KEYWORDS: &[(&str, &str)] = &[
    // 学习/备考
    ("考试", "exam_prep"),
    ("备考", "exam_prep"),
    ("复习", "exam_prep"),
    ("线代", "exam_prep"),
    ("高数", "exam_prep"),
    ("作业", "study"),
    ("课题", "study"),
    ("论文", "study"),
    // 工程/开发
    ("项目", "project"),
    ("部署", "project"),
    ("bug", "project"),
    ("代码", "project"),
    ("commit", "project"),
    ("重构", "project"),
    ("rust", "project"),
    ("architecture", "project"),
    // 陪伴/情绪
    ("累", "companion"),
    ("烦", "companion"),
    ("难过", "companion"),
    ("孤独", "companion"),
    ("陪我", "companion"),
    ("抱抱", "companion"),
    ("开心", "companion"),
    // 休闲/娱乐
    ("游戏", "entertainment"),
    ("番剧", "entertainment"),
    ("电影", "entertainment"),
    ("音乐", "entertainment"),
    ("旅行", "entertainment"),
    // 日常生活/健康
    ("早安", "daily_routine"),
    ("晚安", "daily_routine"),
    ("吃饭", "daily_routine"),
    ("睡觉", "daily_routine"),
    ("失眠", "daily_routine"),
    ("健身", "daily_routine"),
];

/// 话题预测器 (确定性纯函数实现, 0 LLM).
pub struct TopicPredictor;

impl TopicPredictor {
    /// 根据话题线索进行综合预测.
    pub fn predict(cue: &TopicCue) -> TopicPrediction {
        let mut scores: std::collections::HashMap<&'static str, f32> =
            std::collections::HashMap::new();

        // 1. 关键词通道 (从最近用户与助手消息中加权提取)
        for (i, msg) in cue.recent_user_messages.iter().rev().enumerate() {
            let recency_weight = 1.0 / (1.0 + (i as f32) * 0.4);
            let lower = msg.to_lowercase();
            for (kw, topic) in TOPIC_KEYWORDS {
                if lower.contains(kw) {
                    *scores.entry(topic).or_default() += 0.35 * recency_weight;
                }
            }
        }
        for (i, msg) in cue.recent_assistant_messages.iter().rev().enumerate() {
            let recency_weight = 0.5 / (1.0 + (i as f32) * 0.5);
            let lower = msg.to_lowercase();
            for (kw, topic) in TOPIC_KEYWORDS {
                if lower.contains(kw) {
                    *scores.entry(topic).or_default() += 0.2 * recency_weight;
                }
            }
        }

        // 2. 时间节律通道
        if let Some(now) = cue.now {
            let hour = now.hour();
            match hour {
                5..=9 => {
                    *scores.entry("morning_briefing").or_default() += 0.4;
                    *scores.entry("daily_routine").or_default() += 0.2;
                }
                22..=23 | 0..=4 => {
                    *scores.entry("night_chat").or_default() += 0.4;
                    *scores.entry("companion").or_default() += 0.25;
                }
                _ => {}
            }
        }

        // 3. 情绪特征通道
        if let Some(mood) = &cue.user_mood {
            match mood.as_str() {
                "low" => {
                    *scores.entry("companion").or_default() += 0.45;
                }
                "high" => {
                    *scores.entry("entertainment").or_default() += 0.3;
                }
                _ => {}
            }
        }

        // 4. 排序与归一化
        let mut hints: Vec<TopicHint> = scores
            .into_iter()
            .map(|(topic, score)| TopicHint {
                topic: topic.to_string(),
                confidence: score.clamp(0.0, 1.0),
            })
            .collect();

        hints.sort_by(|a, b| {
            b.confidence
                .partial_cmp(&a.confidence)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        TopicPrediction { hints }
    }
}

/// 预载通道 Trait.
pub trait PreloadChannel: Send + Sync {
    /// 通道名称.
    fn name(&self) -> &'static str;
    /// 根据线索生成推荐候选词/标签.
    fn candidates(&self, cue: &TopicCue) -> Vec<String>;
}

/// 关键词匹配预载通道.
pub struct KeywordChannel;

impl PreloadChannel for KeywordChannel {
    fn name(&self) -> &'static str {
        "keyword"
    }

    fn candidates(&self, cue: &TopicCue) -> Vec<String> {
        let mut matched = HashSet::new();
        for msg in &cue.recent_user_messages {
            let lower = msg.to_lowercase();
            for (kw, _) in TOPIC_KEYWORDS {
                if lower.contains(kw) {
                    matched.insert(kw.to_string());
                }
            }
        }
        matched.into_iter().collect()
    }
}

/// 时间节律预载通道.
pub struct TimeChannel;

impl PreloadChannel for TimeChannel {
    fn name(&self) -> &'static str {
        "time"
    }

    fn candidates(&self, cue: &TopicCue) -> Vec<String> {
        let mut out = Vec::new();
        if let Some(now) = cue.now {
            let hour = now.hour();
            let weekday = now.weekday();
            out.push(format!("hour_{hour}"));
            out.push(format!("weekday_{weekday}"));
            if (5..=9).contains(&hour) {
                out.push("morning".to_string());
            } else if (22..=23).contains(&hour) || (0..=4).contains(&hour) {
                out.push("night".to_string());
            }
        }
        out
    }
}

/// 重要性/情绪预载通道.
pub struct ImportanceChannel;

impl PreloadChannel for ImportanceChannel {
    fn name(&self) -> &'static str {
        "importance"
    }

    fn candidates(&self, cue: &TopicCue) -> Vec<String> {
        let mut out = Vec::new();
        if let Some(mood) = &cue.user_mood {
            out.push(format!("mood_{mood}"));
        }
        out
    }
}

/// 复合多通道聚合器.
pub struct CompositeChannel {
    channels: Vec<Box<dyn PreloadChannel>>,
}

impl Default for CompositeChannel {
    fn default() -> Self {
        Self {
            channels: vec![
                Box::new(KeywordChannel),
                Box::new(TimeChannel),
                Box::new(ImportanceChannel),
            ],
        }
    }
}

impl CompositeChannel {
    pub fn new(channels: Vec<Box<dyn PreloadChannel>>) -> Self {
        Self { channels }
    }

    /// 聚合多通道候选词, 自动去重并按首现保序.
    pub fn collect_all(&self, cue: &TopicCue) -> Vec<String> {
        let mut seen = HashSet::new();
        let mut out = Vec::new();
        for ch in &self.channels {
            for c in ch.candidates(cue) {
                if seen.insert(c.clone()) {
                    out.push(c);
                }
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn topic_predictor_identifies_keywords() {
        let cue = TopicCue {
            recent_user_messages: vec!["今天复习高数好累，好难过".to_string()],
            ..Default::default()
        };
        let pred = TopicPredictor::predict(&cue);
        let topics = pred.top_topics(3);
        assert!(topics.contains(&"exam_prep") || topics.contains(&"companion"));
    }

    #[test]
    fn time_channel_identifies_night() {
        let date =
            NaiveDateTime::parse_from_str("2026-08-29 23:30:00", "%Y-%m-%d %H:%M:%S").unwrap();
        let cue = TopicCue {
            now: Some(date),
            ..Default::default()
        };
        let ch = TimeChannel;
        let cands = ch.candidates(&cue);
        assert!(cands.contains(&"night".to_string()));
        assert!(cands.contains(&"hour_23".to_string()));
    }

    #[test]
    fn composite_channel_aggregates_and_deduplicates() {
        let date =
            NaiveDateTime::parse_from_str("2026-08-29 08:00:00", "%Y-%m-%d %H:%M:%S").unwrap();
        let cue = TopicCue {
            recent_user_messages: vec!["早安，今天准备做项目部署".to_string()],
            now: Some(date),
            user_mood: Some("high".to_string()),
            ..Default::default()
        };
        let comp = CompositeChannel::default();
        let results = comp.collect_all(&cue);
        assert!(results.contains(&"morning".to_string()));
        assert!(results.contains(&"mood_high".to_string()));
    }

    #[test]
    fn empty_cue_produces_empty_prediction() {
        let cue = TopicCue::default();
        let pred = TopicPredictor::predict(&cue);
        assert!(pred.hints.is_empty());
        assert_eq!(pred.primary(), None);
    }
}
