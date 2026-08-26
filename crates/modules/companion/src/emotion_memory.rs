//! `apeireth-companion::emotion_memory` — F1 情感记忆 (主人的情绪作为输入侧数据维度).
//!
//! ## 哲学 (主人 2026-08-18 拍板)
//!
//! 主人原话: "她可以像一个情绪障碍患者一样, 极其理性但一直在尝试理解主人的情感".
//! 主人小说第五节: "我没有心。我只是一直在算, 怎么才能让你在这个晚上好过一点点."
//!
//! 边界 (0 装 PASS): **不是"她的情感"** (LLM 无情感, 模拟即假装) — 是**主人的情绪
//! 作为数据维度**: 记录/检索/趋势, 供她"算怎么让你好过".
//!
//! ## 机制 (确定性, 无 LLM)
//!
//! - **MoodRecord**: 主人情绪时间线 (valence -1..1 + arousal 0..1 + 来源 + 备注).
//! - **current_mood**: 最近记录加权 (越近权重越高, 半衰期 4h).
//! - **mood_trend**: 趋势斜率 (变好/变坏 — "她注意到你在好转").
//! - **recall_by_mood**: 情绪上下文检索 — 给定当前情绪, 返回历史相似时段
//!   ("记得你上次烦的时候" — 伙伴行为的机制, 非拟人).
//!
//! ## 挂接 (集成而非分立)
//!
//! - 输入: `emergence::LoopConfig.mood_floor` 同源 (门控已用 mood, 本模块是它的数据源);
//!   `tone` 情绪风格 (文本信号→valence); 主人显式反馈.
//! - 输出: 记忆注入 (情绪上下文调用), 开口策略 (mood 低 → 少开口/陪伴性开口).

/// 情绪来源 (诚实标注).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoodSource {
    /// 文本信号 (对话内容 → 情绪推断, 输入侧).
    TextSignal,
    /// 时段节律 (深夜/清晨的基线状态).
    TimeOfDay,
    /// 主人显式反馈 ("我今天很烦" / 反馈标注).
    ExplicitFeedback,
}

/// 一条主人情绪记录.
#[derive(Debug, Clone)]
pub struct MoodRecord {
    /// 效价 ∈ [-1.0, 1.0] (-1 很差, 0 中性, +1 很好).
    pub valence: f64,
    /// 唤醒度 ∈ [0.0, 1.0] (0 平静, 1 激动).
    pub arousal: f64,
    pub source: MoodSource,
    pub note: String,
    pub at_ms: i64,
}

impl MoodRecord {
    pub fn new(valence: f64, arousal: f64, source: MoodSource, note: impl Into<String>) -> Self {
        Self {
            valence: valence.clamp(-1.0, 1.0),
            arousal: arousal.clamp(0.0, 1.0),
            source,
            note: note.into(),
            at_ms: chrono::Utc::now().timestamp_millis(),
        }
    }
}

/// 当前情绪快照 (加权).
#[derive(Debug, Clone)]
pub struct MoodSnapshot {
    pub valence: f64,
    pub arousal: f64,
    /// 采样的记录数 (0 = 无数据).
    pub sample_count: usize,
    /// 最近一条的来源 (诊断用).
    pub last_source: Option<MoodSource>,
}

/// 情感记忆 (内存时间线; 持久化 = 调用方接 SQLite, 本模块不假装).
#[derive(Debug, Default)]
pub struct EmotionMemory {
    records: Vec<MoodRecord>,
    /// 最近记录半衰期 (ms): 越近权重越高.
    pub decay_half_life_ms: i64,
    /// 检索窗口 (ms): recall_by_mood 只看这个窗口内的记录.
    pub recall_window_ms: i64,
}

impl EmotionMemory {
    pub fn new() -> Self {
        Self {
            records: Vec::new(),
            decay_half_life_ms: 4 * 3600 * 1000,     // 4h
            recall_window_ms: 30 * 24 * 3600 * 1000, // 30 天
        }
    }

    pub fn record(&mut self, r: MoodRecord) {
        self.records.push(r);
    }

    /// 当前情绪: 最近记录按时间衰减加权 (半衰期 decay_half_life_ms).
    pub fn current_mood(&self) -> MoodSnapshot {
        let now = chrono::Utc::now().timestamp_millis();
        let mut w_sum = 0.0;
        let mut v = 0.0;
        let mut a = 0.0;
        let mut count = 0;
        let mut last_source = None;
        for r in self.records.iter().rev().take(50) {
            let age = (now - r.at_ms).max(0) as f64;
            let w = 0.5f64.powf(age / self.decay_half_life_ms as f64);
            v += r.valence * w;
            a += r.arousal * w;
            w_sum += w;
            count += 1;
            if last_source.is_none() {
                last_source = Some(r.source);
            }
        }
        if w_sum <= 0.0 || count == 0 {
            return MoodSnapshot {
                valence: 0.0,
                arousal: 0.0,
                sample_count: 0,
                last_source: None,
            };
        }
        MoodSnapshot {
            valence: (v / w_sum).clamp(-1.0, 1.0),
            arousal: (a / w_sum).clamp(0.0, 1.0),
            sample_count: count,
            last_source,
        }
    }

    /// 情绪趋势: 窗口内首尾 valence 差 (正值 = 在变好). 数据不足返回 None.
    pub fn mood_trend(&self, window_ms: i64) -> Option<f64> {
        let now = chrono::Utc::now().timestamp_millis();
        let cutoff = now - window_ms;
        let window: Vec<&MoodRecord> = self.records.iter().filter(|r| r.at_ms >= cutoff).collect();
        if window.len() < 2 {
            return None;
        }
        let first = window[0].valence;
        let last = window[window.len() - 1].valence;
        Some(last - first)
    }

    /// 情绪上下文检索: 找与目标情绪相似的记录 (valence 差 ≤ tolerance).
    /// "记得你上次烦的时候" — 伙伴行为的机制.
    pub fn recall_by_mood(
        &self,
        target_valence: f64,
        tolerance: f64,
        max: usize,
    ) -> Vec<&MoodRecord> {
        let now = chrono::Utc::now().timestamp_millis();
        let cutoff = now - self.recall_window_ms;
        let mut out: Vec<&MoodRecord> = self
            .records
            .iter()
            .filter(|r| r.at_ms >= cutoff && (r.valence - target_valence).abs() <= tolerance)
            .collect();
        out.sort_by_key(|r| std::cmp::Reverse(r.at_ms));
        out.truncate(max);
        out
    }

    pub fn len(&self) -> usize {
        self.records.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_and_current_mood_weighted() {
        let mut mem = EmotionMemory::new();
        assert_eq!(mem.current_mood().sample_count, 0, "无数据 → 空快照");
        mem.record(MoodRecord::new(
            0.5,
            0.3,
            MoodSource::TextSignal,
            "主人聊得开心",
        ));
        mem.record(MoodRecord::new(
            -0.8,
            0.6,
            MoodSource::ExplicitFeedback,
            "主人说今天很烦",
        ));
        // 最近的记录权重更高 → 当前情绪偏负
        let mood = mem.current_mood();
        assert!(mood.valence < 0.0, "最近记录(烦)应主导: {:?}", mood);
        assert!(mood.sample_count >= 2);
    }

    #[test]
    fn trend_improving_detected() {
        let mut mem = EmotionMemory::new();
        let now = chrono::Utc::now().timestamp_millis();
        mem.records.push(MoodRecord {
            valence: -0.7,
            arousal: 0.5,
            source: MoodSource::TextSignal,
            note: "早".into(),
            at_ms: now - 8 * 3600 * 1000,
        });
        mem.records.push(MoodRecord {
            valence: 0.4,
            arousal: 0.3,
            source: MoodSource::TextSignal,
            note: "晚".into(),
            at_ms: now,
        });
        let trend = mem.mood_trend(24 * 3600 * 1000).unwrap();
        assert!(trend > 0.0, "情绪在变好: trend={trend}");
        // 窗口太短 → None
        assert!(mem.mood_trend(60 * 1000).is_none());
    }

    #[test]
    fn recall_by_mood_finds_similar_periods() {
        let mut mem = EmotionMemory::new();
        let now = chrono::Utc::now().timestamp_millis();
        mem.records.push(MoodRecord {
            valence: -0.9,
            arousal: 0.7,
            source: MoodSource::ExplicitFeedback,
            note: "上次项目黄了".into(),
            at_ms: now - 3 * 24 * 3600 * 1000,
        });
        mem.records.push(MoodRecord {
            valence: 0.8,
            arousal: 0.2,
            source: MoodSource::TextSignal,
            note: "拿到投资那天".into(),
            at_ms: now - 5 * 24 * 3600 * 1000,
        });
        // 现在主人很低落 → 检索相似低落时段 (tolerance 0.2)
        let low = mem.recall_by_mood(-0.8, 0.2, 5);
        assert_eq!(low.len(), 1);
        assert!(
            low[0].note.contains("项目黄了"),
            "应找回低落时段: {:?}",
            low[0].note
        );
        // 高情绪检索
        let high = mem.recall_by_mood(0.8, 0.2, 5);
        assert_eq!(high.len(), 1);
        assert!(high[0].note.contains("投资"));
        // 超窗口的旧记录不被检索
        mem.records.push(MoodRecord {
            valence: -0.8,
            arousal: 0.5,
            source: MoodSource::TextSignal,
            note: "很久以前".into(),
            at_ms: now - 90 * 24 * 3600 * 1000,
        });
        let low2 = mem.recall_by_mood(-0.8, 0.2, 5);
        assert_eq!(low2.len(), 1, "90 天前记录应被窗口排除");
    }

    #[test]
    fn mood_floor_integration_shape() {
        // 与 emergence::LoopConfig.mood_floor (默认 0.3) 语义衔接:
        // 当前 mood < floor → 门控可读 (emergence 决定不出声/陪伴性开口).
        let mut mem = EmotionMemory::new();
        mem.record(MoodRecord::new(
            -0.5,
            0.6,
            MoodSource::ExplicitFeedback,
            "低落",
        ));
        let mood = mem.current_mood();
        assert!(mood.valence < 0.3, "低落期 valence 低于 mood_floor 0.3");
    }

    #[test]
    fn valence_clamped() {
        let r = MoodRecord::new(5.0, 2.0, MoodSource::TimeOfDay, "clamp");
        assert_eq!(r.valence, 1.0);
        assert_eq!(r.arousal, 1.0);
    }
}
