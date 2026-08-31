//! F1 EmotionMemory 器官真实现 (v2 移植版, per `legacy/canonical/apeireth-companion/src/emotion_memory.rs`).
//!
//! **v1 → v2 1:1 翻译纪律**:
//! - v1 真实现是**确定性无 LLM** (per `legacy/canonical/apeireth-companion/src/emotion_memory.rs:11-17`
//!   文档明示: "## 机制 (确定性, 无 LLM)") — v2 trait 保留 `llm_factory()` 接口但**真实现不用**.
//! - v1 数据模型: `MoodRecord { valence [-1,1], arousal [0,1], source, note, at_ms }` +
//!   `MoodSnapshot { valence, arousal, sample_count, last_source }` (2D valence/arousal).
//! - v2 trait `OrganOutput::Emotion` schema 是 {pleasure, arousal, dominance, trend} (3D PAD 提案).
//!   v1 schema 真实只有 2D valence/arousal — **0 装诚实**: 把 `valence` 映射 `pleasure`,
//!   `dominance` 填 `0.0` 并在注释里显式标: v1 无 dominance 概念, 这是 schema 扩展字段
//!   (R1.4 inspired, per `apeireth-plugin::organ:169-175` 注释) 不假装 v1 有.
//!
//! **v1 哲学** (主人 2026-08-18 拍板, docs/design-intent.md):
//! - "她可以像一个情绪障碍患者一样, 极其理性但一直在尝试理解主人的情感".
//! - "我没有心。我只是一直在算, 怎么才能让你在这个晚上好过一点点."
//! - 边界 (0 装 PASS): **不是"她的情感"** (LLM 无情感, 模拟即假装) — 是**主人的情绪
//!   作为数据维度**: 记录/检索/趋势, 供她"算怎么让你好过".
//!
//! **v1 机制 1:1 翻译**:
//! - `MoodRecord`: 主人情绪时间线 (valence/arousal + 来源 + 备注 + 时间戳).
//! - `current_mood`: 最近记录按时间衰减加权 (半衰期 4h).
//! - `mood_trend`: 窗口内首尾 valence 差 (趋势斜率 — "她注意到你在好转").
//! - `recall_by_mood`: 给定目标情绪返回历史相似时段 ("记得你上次烦的时候" — 伙伴行为的机制).
//!
//! **与 v1 真实现的 2 个差异 (子代理 R1 独立判断, 见模块顶注释)**:
//!
//! 1. **时间戳**: v1 用 `chrono::Utc::now()` 隐式; v2 organ crate 不依赖 chrono (保持
//!    依赖最小, 与子代理 R2 hypothesis 同模式), 改 `MoodRecord::at_ms: i64` 由调用方显式注入.
//!    `MoodRecord::new` 默认 `0` (per 子代理 R2 兜底约定); `EmotionOrgan::process()` 在
//!    入口用 `std::time::SystemTime` 派生 epoch ms (无 chrono 依赖, 0 装诚实).
//! 2. **trait schema 适配**: v2 `OrganOutput::Emotion` 是 3D PAD {pleasure, arousal, dominance} +
//!    trend enum; v1 schema 只有 2D valence/arousal + 数值趋势. 映射在 `process()` 注释
//!    显式标注. dominance = 0.0 (v1 无此概念).
//!
//! **承接 (per 任务 §5)**:
//! - 子代理 Q 报告 #3 "Council 真接 LLM" 已就位 (`LlmFactory` 注入).
//! - EmotionMemory 与 Council 共享 `LlmFactory` trait 边界; **真实现是确定性无 LLM**,
//!   `llm_factory()` 返 None (0 装诚实).
//!
//! **3 阶审查** (O-6 锚 9):
//! 1. 总体: 1:1 翻译 v1 `EmotionMemory`, trait 边界 + v2 schema 适配
//!   (valence→pleasure, dominance 显式标缺 0.0)
//! 2. 系统: impl 在 engine (`apeireth-organ`), trait 在 foundation (`apeireth-plugin`)
//! 3. 架构: `Arc<dyn OrganTrait>` 注入 runtime, F1 trait process() 调 EmotionMemoryEngine

use std::time::{SystemTime, UNIX_EPOCH};

use apeireth_plugin::organ::{
    EmotionTrend, OrganError, OrganInput, OrganKind, OrganOutput, OrganTrait,
};

// ============================================
// v1 数据结构 1:1 翻译 (确定性, 无 LLM)
// ============================================

/// 情绪来源 (per v1 `MoodSource` 1:1)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoodSource {
    /// 文本信号 (对话内容 → 情绪推断, 输入侧).
    TextSignal,
    /// 时段节律 (深夜/清晨的基线状态).
    TimeOfDay,
    /// 主人显式反馈 ("我今天很烦" / 反馈标注).
    ExplicitFeedback,
}

/// 一条主人情绪记录 (per v1 `MoodRecord` 1:1).
///
/// **0 装诚实**: v1 字段是 `valence ∈ [-1.0, 1.0]` + `arousal ∈ [0.0, 1.0]` (2D).
/// v2 trait `OrganOutput::Emotion` 是 3D PAD {pleasure, arousal, dominance}; v1 没有
/// `dominance` 字段 — 不假装 v1 有. 转换在 `EmotionOrgan::process()` 注释里显式标.
#[derive(Debug, Clone)]
pub struct MoodRecord {
    /// 效价 ∈ [-1.0, 1.0] (-1 很差, 0 中性, +1 很好).
    pub valence: f64,
    /// 唤醒度 ∈ [0.0, 1.0] (0 平静, 1 激动).
    pub arousal: f64,
    pub source: MoodSource,
    pub note: String,
    /// epoch 毫秒. v1 隐式 `chrono::Utc::now()`; v2 显式注入 (0 chrono 依赖).
    pub at_ms: i64,
}

impl MoodRecord {
    /// 构造 + clamp (无 chrono 依赖, `at_ms = 0` 兜底 — per 子代理 R2 约定).
    ///
    /// **外部调用**: 如要 now-时间戳, 显式传 `system_time_ms()`.
    pub fn new(valence: f64, arousal: f64, source: MoodSource, note: impl Into<String>) -> Self {
        Self {
            valence: valence.clamp(-1.0, 1.0),
            arousal: arousal.clamp(0.0, 1.0),
            source,
            note: note.into(),
            at_ms: 0,
        }
    }

    /// 构造 + clamp + 显式时间戳.
    pub fn with_timestamp(
        valence: f64,
        arousal: f64,
        source: MoodSource,
        note: impl Into<String>,
        at_ms: i64,
    ) -> Self {
        Self {
            valence: valence.clamp(-1.0, 1.0),
            arousal: arousal.clamp(0.0, 1.0),
            source,
            note: note.into(),
            at_ms,
        }
    }
}

/// 当前情绪快照 (per v1 `MoodSnapshot` 1:1).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MoodSnapshot {
    pub valence: f64,
    pub arousal: f64,
    /// 采样的记录数 (0 = 无数据).
    pub sample_count: usize,
    /// 最近一条的来源 (诊断用).
    pub last_source: Option<MoodSource>,
}

impl MoodSnapshot {
    /// 空快照 (无数据时返).
    pub const EMPTY: Self = Self {
        valence: 0.0,
        arousal: 0.0,
        sample_count: 0,
        last_source: None,
    };
}

/// 情感记忆配置 (per v1 `EmotionMemory` 公共字段 1:1).
#[derive(Debug, Clone, Copy)]
pub struct EmotionConfig {
    /// 最近记录半衰期 (ms): 越近权重越高. 默认 4h (per v1).
    pub decay_half_life_ms: i64,
    /// 检索窗口 (ms): `recall_by_mood` 只看这个窗口内的记录. 默认 30 天 (per v1).
    pub recall_window_ms: i64,
}

impl Default for EmotionConfig {
    fn default() -> Self {
        Self {
            decay_half_life_ms: 4 * 3600 * 1000,     // 4h
            recall_window_ms: 30 * 24 * 3600 * 1000, // 30 天
        }
    }
}

// ============================================
// v1 EmotionMemory 1:1 翻译 (确定性, 无 LLM)
// ============================================

/// 情感记忆引擎 (per v1 `EmotionMemory` 1:1 翻译, 保留确定性算法).
///
/// **0 装 PASS**: 无 LLM 依赖. 全部状态可测. 时间戳由调用方注入 (per 子代理 R2 约定).
#[derive(Debug, Default)]
pub struct EmotionMemoryEngine {
    records: Vec<MoodRecord>,
    config: EmotionConfig,
}

impl EmotionMemoryEngine {
    pub fn new(config: EmotionConfig) -> Self {
        Self {
            records: Vec::new(),
            config,
        }
    }

    /// 记录一条主人情绪 (per v1 `EmotionMemory::record` 1:1)
    pub fn record(&mut self, r: MoodRecord) {
        self.records.push(r);
    }

    /// 当前情绪: 最近记录按时间衰减加权 (半衰期 `decay_half_life_ms`) (per v1 1:1).
    ///
    /// v1 实现细节: 取最近 50 条 (`.rev().take(50)`), 时间戳衰减加权 (半衰期).
    /// **now**: 由调用方注入 — 当前 emotion 是基于 caller-supplied now 计算的
    /// (process() 内部用 `system_time_ms()` 派生).
    pub fn current_mood_at(&self, now_ms: i64) -> MoodSnapshot {
        let mut w_sum = 0.0;
        let mut v = 0.0;
        let mut a = 0.0;
        let mut count = 0;
        let mut last_source = None;
        for r in self.records.iter().rev().take(50) {
            let age = (now_ms - r.at_ms).max(0) as f64;
            let w = 0.5_f64.powf(age / self.config.decay_half_life_ms as f64);
            v += r.valence * w;
            a += r.arousal * w;
            w_sum += w;
            count += 1;
            if last_source.is_none() {
                last_source = Some(r.source);
            }
        }
        if w_sum <= 0.0 || count == 0 {
            return MoodSnapshot::EMPTY;
        }
        MoodSnapshot {
            valence: (v / w_sum).clamp(-1.0, 1.0),
            arousal: (a / w_sum).clamp(0.0, 1.0),
            sample_count: count,
            last_source,
        }
    }

    /// 当前情绪 (便捷: 用 `system_time_ms()` 派生 now). 等价 v1 行为.
    pub fn current_mood(&self) -> MoodSnapshot {
        self.current_mood_at(system_time_ms())
    }

    /// 情绪趋势: 窗口内首尾 valence 差 (正值 = 在变好) (per v1 1:1).
    ///
    /// 数据不足 (< 2 条) 返回 None.
    pub fn mood_trend_at(&self, now_ms: i64, window_ms: i64) -> Option<f64> {
        let cutoff = now_ms - window_ms;
        let window: Vec<&MoodRecord> = self.records.iter().filter(|r| r.at_ms >= cutoff).collect();
        if window.len() < 2 {
            return None;
        }
        let first = window[0].valence;
        let last = window[window.len() - 1].valence;
        Some(last - first)
    }

    /// 情绪趋势 (便捷: 用 `system_time_ms()` 派生 now).
    pub fn mood_trend(&self, window_ms: i64) -> Option<f64> {
        self.mood_trend_at(system_time_ms(), window_ms)
    }

    /// 情绪上下文检索: 找与目标情绪相似的记录 (valence 差 ≤ tolerance) (per v1 1:1).
    ///
    /// "记得你上次烦的时候" — 伙伴行为的机制, 非拟人.
    ///
    /// **测试便利**: `_at(now_ms)` 显式注入时间, 测试可控 (per 子代理 R2 约定).
    pub fn recall_by_mood_at(
        &self,
        now_ms: i64,
        target_valence: f64,
        tolerance: f64,
        max: usize,
    ) -> Vec<&MoodRecord> {
        let cutoff = now_ms - self.config.recall_window_ms;
        let mut out: Vec<&MoodRecord> = self
            .records
            .iter()
            .filter(|r| r.at_ms >= cutoff && (r.valence - target_valence).abs() <= tolerance)
            .collect();
        out.sort_by_key(|r| std::cmp::Reverse(r.at_ms));
        out.truncate(max);
        out
    }

    /// 情绪上下文检索 (便捷: 用 `system_time_ms()` 派生 now, 等价 v1 行为).
    pub fn recall_by_mood(
        &self,
        target_valence: f64,
        tolerance: f64,
        max: usize,
    ) -> Vec<&MoodRecord> {
        self.recall_by_mood_at(system_time_ms(), target_valence, tolerance, max)
    }

    /// 记录数 (per v1 `len` 1:1).
    pub fn len(&self) -> usize {
        self.records.len()
    }

    /// 记录数是否为 0 (标准 trait method).
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }
}

// ============================================
// EmotionOrgan (v2 trait 真实现)
// ============================================

/// F1 情感记忆器官 (per v2 OrganTrait 1:1 翻译 v1 EmotionMemory).
///
/// **0 装诚实**:
/// - v1 emotion_memory 是确定性无 LLM (per v1 doc 11-17 行).
/// - `llm_factory()` 返 None — 不假装能调 LLM.
/// - trait schema 适配: `valence→pleasure`, `dominance=0.0` (v1 无此概念, 显式标缺).
pub struct EmotionOrgan {
    engine: std::sync::Mutex<EmotionMemoryEngine>,
}

impl EmotionOrgan {
    /// 构造 F1 emotion_memory organ (默认配置: 4h 半衰期, 30 天检索窗口).
    pub fn new() -> Self {
        Self {
            engine: std::sync::Mutex::new(EmotionMemoryEngine::new(EmotionConfig::default())),
        }
    }

    /// 构造 + 自定义 config.
    pub fn with_config(config: EmotionConfig) -> Self {
        Self {
            engine: std::sync::Mutex::new(EmotionMemoryEngine::new(config)),
        }
    }

    /// 暴露底层 engine (per v1 API 1:1, 外部可直接调 record / current_mood / mood_trend).
    ///
    /// **为何**: 1:1 翻译 v1 公开 API (record / current_mood / mood_trend / recall_by_mood),
    /// runtime 集成层可能要直接调 (e.g. 喂入 mood_floor 门控). trait `process()` 是聚合入口.
    pub fn engine(&self) -> std::sync::MutexGuard<'_, EmotionMemoryEngine> {
        self.engine
            .lock()
            .expect("EmotionOrgan mutex poisoned (0 装诚实)")
    }

    /// 便捷 record (per v1 1:1, 简化外部调用, 时间戳 = now).
    pub fn record(&self, valence: f64, arousal: f64, source: MoodSource, note: impl Into<String>) {
        self.engine().record(MoodRecord::with_timestamp(
            valence,
            arousal,
            source,
            note,
            system_time_ms(),
        ));
    }
}

impl Default for EmotionOrgan {
    fn default() -> Self {
        Self::new()
    }
}

/// 从 `OrganInput` 解析 valence/arousal (0 装诚实: 不发明 LLM 推断, 仅从 hints 解析).
///
/// **解析规则** (per task spec §3 + 子代理 Q1 R1.4):
/// - `context_hints` 是 `Vec<String>`. 期望格式:
///   - `"valence=<f64>"` 显式 valence
///   - `"arousal=<f64>"` 显式 arousal
///   - `"source=<text_signal|time_of_day|explicit_feedback>"` 来源
///   - 其他 (e.g. topic tags, 主人原话) 视为 note 片段
/// - 缺 `valence` / `arousal` 默认 `0.0` (中性).
/// - 缺 `source` 默认 `TextSignal` (per v1 doc 暗示).
///
/// **0 装诚实**: 不发明 LLM 推断. 仅确定性解析 hint 字段.
fn parse_hints(input: &OrganInput) -> (f64, f64, MoodSource, String) {
    let mut valence = 0.0_f64;
    let mut arousal = 0.0_f64;
    let mut source = MoodSource::TextSignal;
    let mut note_parts: Vec<String> = Vec::new();

    for hint in &input.context_hints {
        let hint = hint.trim();
        if let Some(rest) = hint.strip_prefix("valence=") {
            if let Ok(v) = rest.parse::<f64>() {
                valence = v.clamp(-1.0, 1.0);
            }
        } else if let Some(rest) = hint.strip_prefix("arousal=") {
            if let Ok(v) = rest.parse::<f64>() {
                arousal = v.clamp(0.0, 1.0);
            }
        } else if let Some(rest) = hint.strip_prefix("source=") {
            source = match rest.trim() {
                "text_signal" => MoodSource::TextSignal,
                "time_of_day" => MoodSource::TimeOfDay,
                "explicit_feedback" => MoodSource::ExplicitFeedback,
                _ => MoodSource::TextSignal,
            };
        } else {
            // 其他 hint → note 片段
            note_parts.push(hint.to_string());
        }
    }

    // episode 内容作为 note 后缀 (per v1 "文本信号" 含义: 主人原话)
    if !input.episode.content.is_empty() {
        note_parts.push(input.episode.content.clone());
    }

    let note = if note_parts.is_empty() {
        format!("session={}", input.session_id)
    } else {
        note_parts.join(" | ")
    };

    (valence, arousal, source, note)
}

/// v2 schema 适配: 趋势值 → `EmotionTrend` enum (per `apeireth-plugin::organ:208-213`).
///
/// **0 装诚实**: v1 emotion_memory 0 显式提供 trend enum. v2 schema 是 R1.4 inspired 提案
/// (per `apeireth-plugin::organ:207` 注释). 翻译规则:
/// - `trend = None` (数据不足) → Stable (中性)
/// - `|trend| < 0.05` → Stable (微变视为稳定)
/// - `trend > 0` → Rising (在变好)
/// - `trend < 0` → Falling (在变坏)
fn trend_to_enum(trend: Option<f64>) -> EmotionTrend {
    match trend {
        Some(t) if t > 0.05 => EmotionTrend::Rising,
        Some(t) if t < -0.05 => EmotionTrend::Falling,
        _ => EmotionTrend::Stable,
    }
}

#[async_trait::async_trait]
impl OrganTrait for EmotionOrgan {
    fn name(&self) -> &'static str {
        "F1 Emotion Memory"
    }

    fn organ_id(&self) -> OrganKind {
        OrganKind::F1
    }

    async fn process(&self, input: OrganInput) -> Result<OrganOutput, OrganError> {
        // 1:1 翻译 v1 emotion_memory 路径:
        // 1) 解析 input → 构造 MoodRecord (valence/arousal from hints, source from hint,
        //    note from episode content)
        // 2) 记录一条 MoodRecord (per v1 `record`)
        // 3) 取当前情绪快照 (per v1 `current_mood`) + 趋势 (per v1 `mood_trend`)
        // 4) 翻译成 v2 trait schema (OrganOutput::Emotion { pleasure, arousal, dominance, trend })
        //
        // **0 装诚实 schema 适配**:
        // - `pleasure` ← `valence` (v1 真有, 1:1 翻译)
        // - `arousal` ← `arousal` (v1 真有, 1:1 翻译)
        // - `dominance` ← `0.0` (v1 **无此概念**, 显式标缺; R1.4 inspired 提案字段)
        // - `trend` ← `mood_trend` 转 enum (v1 真有 trend 概念, 仅翻译为 enum)
        //
        // **dry_run 模式**: 不真记录 MoodRecord (per v1 curiosity dry_run 同模式 —
        // 不扣/不改状态). 此处 v1 emotion_memory 无显式 dry_run, 但 v2 trait 接口要求.
        let (valence, arousal, source, note) = parse_hints(&input);
        let now_ms = system_time_ms();

        let (snapshot, trend_opt) = {
            let mut engine = self
                .engine
                .lock()
                .map_err(|e| OrganError::Internal(format!("mutex poisoned: {e}")))?;

            if !input.dry_run {
                engine.record(MoodRecord {
                    valence,
                    arousal,
                    source,
                    note,
                    at_ms: now_ms,
                });
            }

            // 趋势窗口 = 24h (per v1 tests 24 * 3600 * 1000)
            let snap = engine.current_mood_at(now_ms);
            let trend = engine.mood_trend_at(now_ms, 24 * 3600 * 1000);
            (snap, trend)
        };

        // 0 装诚实: dominance = 0.0 (v1 无此概念, schema 字段保留给 R1.4 扩展)
        Ok(OrganOutput::Emotion {
            pleasure: snapshot.valence as f32,
            arousal: snapshot.arousal as f32,
            dominance: 0.0,
            trend: trend_to_enum(trend_opt),
        })
    }

    /// 0 装诚实: v1 emotion_memory 是确定性无 LLM, 返 None 不假装.
    fn llm_factory(&self) -> Option<std::sync::Arc<dyn apeireth_plugin::llm_factory::LlmFactory>> {
        None
    }
}

// ============================================
// 时间戳 helper (无 chrono 依赖, 0 装诚实)
// ============================================

/// 当前 epoch 毫秒 (per v1 `chrono::Utc::now().timestamp_millis()` 1:1 行为,
/// 用 std `SystemTime` 实现避免引入 chrono 依赖).
fn system_time_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

// ============================================
// 单元测试 (1:1 翻译 v1 emotion_memory.rs 测试)
// ============================================

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_core::kernel::memory::Episode;

    fn make_ep(content: &str) -> Episode {
        Episode {
            id: "test-ep".into(),
            session_id: "test-session".into(),
            role: "user".into(),
            content: content.into(),
            timestamp: 0,
        }
    }

    /// v1 1:1: record + current_mood 加权 (最近记录主导).
    #[test]
    fn record_and_current_mood_weighted() {
        let mut mem = EmotionMemoryEngine::new(EmotionConfig::default());
        assert_eq!(mem.current_mood().sample_count, 0, "无数据 → 空快照");
        mem.record(MoodRecord::with_timestamp(
            0.5,
            0.3,
            MoodSource::TextSignal,
            "主人聊得开心",
            0,
        ));
        mem.record(MoodRecord::with_timestamp(
            -0.8,
            0.6,
            MoodSource::ExplicitFeedback,
            "主人说今天很烦",
            0,
        ));
        let mood = mem.current_mood_at(0);
        assert!(mood.valence < 0.0, "最近记录(烦)应主导: {:?}", mood);
        assert!(mood.sample_count >= 2);
    }

    /// v1 1:1: 趋势检测 (val - val > 0 → 在变好).
    #[test]
    fn trend_improving_detected() {
        let mut mem = EmotionMemoryEngine::new(EmotionConfig::default());
        let now = 1_000_000_000_i64; // 固定时间戳 (0 chrono 依赖)
        mem.record(MoodRecord::with_timestamp(
            -0.7,
            0.5,
            MoodSource::TextSignal,
            "早",
            now - 8 * 3600 * 1000,
        ));
        mem.record(MoodRecord::with_timestamp(
            0.4,
            0.3,
            MoodSource::TextSignal,
            "晚",
            now,
        ));
        let trend = mem.mood_trend_at(now, 24 * 3600 * 1000).unwrap();
        assert!(trend > 0.0, "情绪在变好: trend={trend}");
        assert!(mem.mood_trend_at(now, 60 * 1000).is_none());
    }

    /// v1 1:1: recall_by_mood 找相似情绪时段.
    #[test]
    fn recall_by_mood_finds_similar_periods() {
        let mut mem = EmotionMemoryEngine::new(EmotionConfig::default());
        let now = 1_000_000_000_i64;
        mem.record(MoodRecord::with_timestamp(
            -0.9,
            0.7,
            MoodSource::ExplicitFeedback,
            "上次项目黄了",
            now - 3 * 24 * 3600 * 1000,
        ));
        mem.record(MoodRecord::with_timestamp(
            0.8,
            0.2,
            MoodSource::TextSignal,
            "拿到投资那天",
            now - 5 * 24 * 3600 * 1000,
        ));
        let low = mem.recall_by_mood_at(now, -0.8, 0.2, 5);
        assert_eq!(low.len(), 1);
        assert!(low[0].note.contains("项目黄了"));
        let high = mem.recall_by_mood_at(now, 0.8, 0.2, 5);
        assert_eq!(high.len(), 1);
        assert!(high[0].note.contains("投资"));
        mem.record(MoodRecord::with_timestamp(
            -0.8,
            0.5,
            MoodSource::TextSignal,
            "很久以前",
            now - 90 * 24 * 3600 * 1000,
        ));
        let low2 = mem.recall_by_mood_at(now, -0.8, 0.2, 5);
        assert_eq!(low2.len(), 1, "90 天前记录应被窗口排除");
    }

    /// v1 1:1: valence / arousal clamp.
    #[test]
    fn valence_clamped() {
        let r = MoodRecord::new(5.0, 2.0, MoodSource::TimeOfDay, "clamp");
        assert_eq!(r.valence, 1.0);
        assert_eq!(r.arousal, 1.0);
    }

    /// v2 新增: EmotionOrgan 构造 + 0 装诚实标 (llm_factory None + organ_id F1).
    #[test]
    fn emotion_organ_trait_metadata() {
        let organ = EmotionOrgan::new();
        assert_eq!(organ.name(), "F1 Emotion Memory");
        assert_eq!(organ.organ_id(), OrganKind::F1);
        assert!(
            organ.llm_factory().is_none(),
            "v1 emotion_memory 是确定性无 LLM, trait 必须返 None (0 装诚实)"
        );
    }

    /// v2 新增: EmotionOrgan 便捷 record (per v1 `record` 1:1 暴露).
    #[test]
    fn emotion_organ_record_via_convenience_method() {
        let organ = EmotionOrgan::new();
        organ.record(0.5, 0.3, MoodSource::TextSignal, "主人聊得开心");
        organ.record(-0.8, 0.6, MoodSource::ExplicitFeedback, "主人说今天很烦");
        let engine = organ.engine();
        let mood = engine.current_mood();
        assert!(mood.valence < 0.0, "最近记录主导");
        assert!(mood.sample_count >= 2);
    }

    /// v2 schema 适配: dominance = 0.0 (v1 无此概念, 显式标缺).
    #[tokio::test]
    async fn emotion_organ_process_dominance_is_zero_v1_truth() {
        let organ = EmotionOrgan::new();
        // 不喂任何 record → 空快照
        let input = OrganInput {
            episode: make_ep(""),
            session_id: "test-session".into(),
            context_hints: vec![],
            dry_run: true, // dry_run → 不真记录, 验空快照路径
        };
        let output = organ.process(input).await.expect("process ok");
        match output {
            OrganOutput::Emotion {
                pleasure,
                arousal,
                dominance,
                trend,
            } => {
                assert_eq!(pleasure, 0.0, "空快照 valence=0");
                assert_eq!(arousal, 0.0, "空快照 arousal=0");
                assert_eq!(
                    dominance, 0.0,
                    "0 装诚实: v1 无 dominance 概念, schema 字段填 0.0"
                );
                // 空快照 trend=None → Stable
                assert!(matches!(trend, EmotionTrend::Stable));
            }
            other => panic!("expected Emotion output, got {other:?}"),
        }
    }

    /// v2 schema 适配: parse_hints 解析 valence/arousal/source (确定性, 无 LLM).
    #[tokio::test]
    async fn emotion_organ_process_parses_hints() {
        let organ = EmotionOrgan::new();
        let input = OrganInput {
            episode: make_ep("主人说今天很累"),
            session_id: "test-session".into(),
            context_hints: vec![
                "valence=-0.5".into(),
                "arousal=0.6".into(),
                "source=explicit_feedback".into(),
                "low_energy".into(),
            ],
            dry_run: false,
        };
        let output = organ.process(input).await.expect("process ok");
        match output {
            OrganOutput::Emotion {
                pleasure,
                arousal,
                dominance,
                ..
            } => {
                assert!((pleasure - (-0.5)).abs() < 1e-4, "valence 解析");
                assert!((arousal - 0.6).abs() < 1e-4, "arousal 解析");
                assert_eq!(dominance, 0.0, "v1 无 dominance");
            }
            other => panic!("expected Emotion output, got {other:?}"),
        }
        // 验证 record 真入库
        let engine = organ.engine();
        assert_eq!(engine.len(), 1);
        assert_eq!(engine.current_mood().sample_count, 1);
    }

    /// v2 新增: dry_run 模式不真记录 (per task spec §3 + 子代理 Q1 dry_run 同模式).
    #[tokio::test]
    async fn emotion_organ_dry_run_does_not_record() {
        let organ = EmotionOrgan::new();
        let input = OrganInput {
            episode: make_ep("test"),
            session_id: "test-session".into(),
            context_hints: vec!["valence=0.5".into()],
            dry_run: true,
        };
        let _ = organ.process(input).await.expect("process ok");
        let engine = organ.engine();
        assert_eq!(engine.len(), 0, "dry_run 不真记录");
    }

    /// v2 新增: 趋势翻译 None → Stable (per v1 数据不足行为).
    #[test]
    fn trend_to_enum_none_is_stable() {
        assert!(matches!(trend_to_enum(None), EmotionTrend::Stable));
        assert!(matches!(trend_to_enum(Some(0.01)), EmotionTrend::Stable));
        assert!(matches!(trend_to_enum(Some(0.5)), EmotionTrend::Rising));
        assert!(matches!(trend_to_enum(Some(-0.5)), EmotionTrend::Falling));
    }
}
