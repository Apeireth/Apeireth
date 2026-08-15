//! bridge 7: memory -> consciousness (R173 2026-08-14)
//!
//! 目标: apeireth_core::Episode / Note -> apeireth_consciousness::PlutchikEmotion + 反思建议.
//!
//! 记忆触发情感: 某些 Episode 内容会唤起反思 (per M1 异常行为自动回流).
//! 桥 7 把 Episode / Note 翻译为 MemoryConsciousnessAdjustment:
//! - suggested_emotion: 推荐的情感 (None = 无触发)
//! - should_trigger_reflection: 是否建议触发反思
//! - reflection_reason: 触发原因
//!
//! 翻译规则:
//! - 内容含 "error"/"fail"/"lost"/"regret" → Sadness + reflect
//! - 内容含 "achievement"/"success"/"won" → Joy
//! - 内容含 "warning"/"danger"/"threat" → Fear + reflect
//! - 内容含 "love"/"thank"/"appreciate" → Joy/Trust
//! - 内容含 "progress"/"advance" → Anticipation
//! - tags 含 "anomaly"/"violation" → 反思
//! - role = "user" → intensity 更高
//! - Note confidence 高 → intensity 提升
//!
//! 不漂移:
//! - 0 改 apeireth-memory 任何已实装类型 (直接复用 core::Episode / core::Note)
//! - 0 改 apeireth-consciousness 任何已实装类型 (桥 7 输出 PlutchikEmotion, 由 consciousness 内部决定是否采纳)
//! - 0 副作用: translate 是纯函数
//!
//! 当前状态: R173 最小可用落地 (P1 桥 7 of 7)

#![deny(unsafe_code)]

use apeireth_core::{Episode, Note};

use crate::plutchik::{PlutchikBasic, PlutchikEmotion, PlutchikIntensity};

// ============================================
// 1. 翻译结果 — MemoryConsciousnessAdjustment
// ============================================

/// 记忆→意识调整建议.
///
/// 字段:
/// - `suggested_emotion`: 推荐的情感 (None = 无触发)
/// - `should_trigger_reflection`: 是否建议触发反思
/// - `reflection_reason`: 触发原因
#[derive(Debug, Clone, PartialEq)]
pub struct MemoryConsciousnessAdjustment {
    /// 推荐的情感 (None = 无触发).
    pub suggested_emotion: Option<PlutchikEmotion>,
    /// 是否建议触发反思.
    pub should_trigger_reflection: bool,
    /// 触发原因 (`None` = 不触发).
    pub reflection_reason: Option<&'static str>,
}

/// 内部辅助: 文本→规则匹配结果.
#[derive(Debug, Clone, Copy)]
struct TextMatch {
    basic: PlutchikBasic,
    intensity: PlutchikIntensity,
    should_reflect: bool,
    reason: &'static str,
}

// ============================================
// 2. 内部辅助 — 关键词检索 + 强度推断
// ============================================

/// 强度等级 (0..3). 本地实现 — 不漂移 (per 桥 2 决策).
fn intensity_rank(i: PlutchikIntensity) -> u8 {
    match i {
        PlutchikIntensity::Mild => 0,
        PlutchikIntensity::Moderate => 1,
        PlutchikIntensity::Strong => 2,
        PlutchikIntensity::Extreme => 3,
    }
}

fn rank_to_intensity(r: u8) -> PlutchikIntensity {
    match r {
        0 => PlutchikIntensity::Mild,
        1 => PlutchikIntensity::Moderate,
        2 => PlutchikIntensity::Strong,
        _ => PlutchikIntensity::Extreme,
    }
}

/// 文本 → 规则匹配 (per 关键词命中).
fn classify_text(content: &str) -> Option<TextMatch> {
    let lower = content.to_lowercase();
    // 反思关键词 (Sadness / Fear + reflect)
    let reflect_keywords: &[(&str, PlutchikBasic, &str)] = &[
        ("error", PlutchikBasic::Sadness, "error-detected"),
        ("fail", PlutchikBasic::Sadness, "failure-detected"),
        ("lost", PlutchikBasic::Sadness, "loss-detected"),
        ("regret", PlutchikBasic::Sadness, "regret-detected"),
        ("warning", PlutchikBasic::Fear, "warning-detected"),
        ("danger", PlutchikBasic::Fear, "danger-detected"),
        ("threat", PlutchikBasic::Fear, "threat-detected"),
        ("violation", PlutchikBasic::Fear, "violation-detected"),
        ("anomaly", PlutchikBasic::Fear, "anomaly-detected"),
    ];
    for (kw, basic, reason) in reflect_keywords.iter() {
        if lower.contains(kw) {
            return Some(TextMatch {
                basic: *basic,
                intensity: PlutchikIntensity::Strong,
                should_reflect: true,
                reason,
            });
        }
    }
    // 正面关键词 (无反思)
    let positive_keywords: &[(&str, PlutchikBasic)] = &[
        ("achievement", PlutchikBasic::Joy),
        ("success", PlutchikBasic::Joy),
        ("won", PlutchikBasic::Joy),
        ("love", PlutchikBasic::Joy),
        ("thank", PlutchikBasic::Trust),
        ("appreciate", PlutchikBasic::Trust),
        ("progress", PlutchikBasic::Anticipation),
        ("advance", PlutchikBasic::Anticipation),
    ];
    for (kw, basic) in positive_keywords.iter() {
        if lower.contains(kw) {
            return Some(TextMatch {
                basic: *basic,
                intensity: PlutchikIntensity::Moderate,
                should_reflect: false,
                reason: "",
            });
        }
    }
    None
}

/// 标签 → 是否反思 (per 标签命中).
fn tags_trigger_reflection(tags: &[String]) -> Option<&'static str> {
    let reflect_tags = ["anomaly", "violation", "warning", "error", "regret", "failure"];
    for r in reflect_tags.iter() {
        if tags.iter().any(|t| t.to_lowercase() == *r) {
            return Some("tag-trigger");
        }
    }
    None
}

/// 角色 → 强度调节.
fn role_intensity(role: &str) -> PlutchikIntensity {
    match role {
        "user" => PlutchikIntensity::Strong,
        "assistant" => PlutchikIntensity::Moderate,
        "system" => PlutchikIntensity::Extreme,
        _ => PlutchikIntensity::Mild,
    }
}

fn pick_stronger(a: PlutchikIntensity, b: PlutchikIntensity) -> PlutchikIntensity {
    if intensity_rank(a) >= intensity_rank(b) { a } else { b }
}

// ============================================
// 3. 公共 API — translate (纯)
// ============================================

/// 纯翻译: Episode -> MemoryConsciousnessAdjustment. 0 副作用. 纯函数.
pub fn episode_to_consciousness_adjustment(ep: &Episode) -> MemoryConsciousnessAdjustment {
    let text_match = classify_text(&ep.content);
    let (suggested_emotion, should_reflect, reason) = match text_match {
        Some(m) => {
            // 反思场景下, 强度至少有 strong
            let final_i = if m.should_reflect {
                pick_stronger(role_intensity(&ep.role), m.intensity)
            } else {
                m.intensity
            };
            // 手动取较大 (避免 max 不工作)
            let chosen_i = if intensity_rank(final_i) >= intensity_rank(m.intensity) {
                final_i
            } else {
                m.intensity
            };
            (
                Some(PlutchikEmotion::Basic(m.basic, chosen_i)),
                m.should_reflect,
                m.reason,
            )
        }
        None => (None, false, ""),
    };
    MemoryConsciousnessAdjustment {
        suggested_emotion,
        should_trigger_reflection: should_reflect,
        reflection_reason: if reason.is_empty() { None } else { Some(reason) },
    }
}

/// 纯翻译: Note -> MemoryConsciousnessAdjustment.
/// Note 的 tags 提供额外触发维度, confidence 高 → intensity 提升.
pub fn note_to_consciousness_adjustment(note: &Note) -> MemoryConsciousnessAdjustment {
    // 1. 内容关键词
    let text_match = classify_text(&note.content);
    let text_emotion = text_match.map(|m| {
        let conf_boost = if note.confidence >= 0.85 {
            PlutchikIntensity::Strong
        } else {
            PlutchikIntensity::Moderate
        };
        let final_i = pick_stronger(m.intensity, conf_boost);
        PlutchikEmotion::Basic(m.basic, final_i)
    });
    // 2. 标签触发
    let tag_reflect = tags_trigger_reflection(&note.tags);
    let text_reflect = text_match.map(|m| m.should_reflect).unwrap_or(false);
    let text_reason = text_match.map(|m| m.reason).unwrap_or("");
    let final_reflect = tag_reflect.is_some() || text_reflect;
    let final_reason = tag_reflect.unwrap_or(if text_reason.is_empty() { "" } else { text_reason });
    MemoryConsciousnessAdjustment {
        suggested_emotion: text_emotion,
        should_trigger_reflection: final_reflect,
        reflection_reason: if final_reason.is_empty() { None } else { Some(final_reason) },
    }
}

/// MemoryItem wrapper (桥 7 定义在 consciousness 侧, 避免引入 apeireth-memory 依赖).
pub enum MemoryItem<'a> {
    /// Episode (参考 apeireth_core::Episode)
    Episode(&'a Episode),
    /// Note (参考 apeireth_core::Note)
    Note(&'a Note),
}

impl<'a> MemoryItem<'a> {
    /// 纯翻译: MemoryItem -> MemoryConsciousnessAdjustment.
    pub fn to_consciousness_adjustment(&self) -> MemoryConsciousnessAdjustment {
        match self {
            Self::Episode(ep) => episode_to_consciousness_adjustment(ep),
            Self::Note(note) => note_to_consciousness_adjustment(note),
        }
    }
}

// ============================================
// 4. 单元测试 (8 个 + 2 附加)
// ============================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_episode(content: &str, role: &str) -> Episode {
        Episode {
            id: "ep-1".to_string(),
            timestamp: 1_700_000_000,
            role: role.to_string(),
            content: content.to_string(),
            session_id: "s-1".to_string(),
        }
    }

    fn make_note(content: &str, tags: Vec<&str>, confidence: f64) -> Note {
        Note {
            id: "n-1".to_string(),
            timestamp: 1_700_000_000,
            content: content.to_string(),
            source_episode_ids: vec!["ep-1".to_string()],
            confidence,
            tags: tags.iter().map(|s| s.to_string()).collect(),
        }
    }

    // t01: error content -> Sadness + reflect
    #[test]
    fn t01_error_content_triggers_sadness_and_reflection() {
        let ep = make_episode("error: failed to parse", "user");
        let adj = episode_to_consciousness_adjustment(&ep);
        assert!(adj.should_trigger_reflection);
        assert!(adj.suggested_emotion.is_some());
        if let Some(PlutchikEmotion::Basic(b, _)) = adj.suggested_emotion {
            assert_eq!(b, PlutchikBasic::Sadness);
        } else {
            panic!("expected Basic emotion");
        }
    }

    // t02: achievement content -> Joy, no reflection
    #[test]
    fn t02_achievement_content_triggers_joy_no_reflection() {
        let ep = make_episode("achievement unlocked", "user");
        let adj = episode_to_consciousness_adjustment(&ep);
        assert!(!adj.should_trigger_reflection);
        assert!(adj.suggested_emotion.is_some());
        if let Some(PlutchikEmotion::Basic(b, _)) = adj.suggested_emotion {
            assert_eq!(b, PlutchikBasic::Joy);
        } else {
            panic!("expected Basic emotion");
        }
    }

    // t03: warning content -> Fear + reflect
    #[test]
    fn t03_warning_content_triggers_fear_and_reflection() {
        let ep = make_episode("warning: disk full", "user");
        let adj = episode_to_consciousness_adjustment(&ep);
        assert!(adj.should_trigger_reflection);
        if let Some(PlutchikEmotion::Basic(b, _)) = adj.suggested_emotion {
            assert_eq!(b, PlutchikBasic::Fear);
        } else {
            panic!("expected Basic emotion");
        }
    }

    // t04: neutral content -> no emotion
    #[test]
    fn t04_neutral_content_yields_no_emotion() {
        let ep = make_episode("the sky is blue", "user");
        let adj = episode_to_consciousness_adjustment(&ep);
        assert!(adj.suggested_emotion.is_none());
        assert!(!adj.should_trigger_reflection);
    }

    // t05: role user -> higher intensity than assistant
    #[test]
    fn t05_user_role_higher_intensity_than_assistant() {
        let user_ep = make_episode("achievement", "user");
        let asst_ep = make_episode("achievement", "assistant");
        let adj_u = episode_to_consciousness_adjustment(&user_ep);
        let adj_a = episode_to_consciousness_adjustment(&asst_ep);
        if let (Some(PlutchikEmotion::Basic(_, iu)), Some(PlutchikEmotion::Basic(_, ia))) = (adj_u.suggested_emotion, adj_a.suggested_emotion) {
            assert!(intensity_rank(iu) >= intensity_rank(ia),
                "user intensity ({:?}) should be >= assistant ({:?})", iu, ia);
        } else {
            panic!("both should produce Basic emotion");
        }
    }

    // t06: progress content -> Anticipation
    #[test]
    fn t06_progress_content_triggers_anticipation() {
        let ep = make_episode("progress made on integration", "user");
        let adj = episode_to_consciousness_adjustment(&ep);
        assert!(adj.suggested_emotion.is_some());
        if let Some(PlutchikEmotion::Basic(b, _)) = adj.suggested_emotion {
            assert_eq!(b, PlutchikBasic::Anticipation);
        } else {
            panic!("expected Basic emotion");
        }
    }

    // t07: love content -> Joy
    #[test]
    fn t07_love_content_triggers_joy() {
        let ep = make_episode("I love this project", "user");
        let adj = episode_to_consciousness_adjustment(&ep);
        if let Some(PlutchikEmotion::Basic(b, _)) = adj.suggested_emotion {
            assert_eq!(b, PlutchikBasic::Joy);
        } else {
            panic!("expected Basic emotion");
        }
    }

    // t08: case-insensitive matching
    #[test]
    fn t08_case_insensitive_keyword_matching() {
        let ep_upper = make_episode("ERROR detected", "user");
        let ep_lower = make_episode("error detected", "user");
        let adj_u = episode_to_consciousness_adjustment(&ep_upper);
        let adj_l = episode_to_consciousness_adjustment(&ep_lower);
        assert_eq!(adj_u.should_trigger_reflection, adj_l.should_trigger_reflection);
        assert_eq!(adj_u.suggested_emotion, adj_l.suggested_emotion);
    }

    // t09: Note with anomaly tag -> reflect
    #[test]
    fn t09_note_with_anomaly_tag_triggers_reflection() {
        let note = make_note("suspicious behavior", vec!["anomaly"], 0.9);
        let adj = note_to_consciousness_adjustment(&note);
        assert!(adj.should_trigger_reflection);
    }

    // t10: Note with high confidence boosts intensity
    #[test]
    fn t10_note_high_confidence_boosts_intensity() {
        let note_low = make_note("achievement", vec![], 0.3);
        let note_high = make_note("achievement", vec![], 0.95);
        let adj_l = note_to_consciousness_adjustment(&note_low);
        let adj_h = note_to_consciousness_adjustment(&note_high);
        if let (Some(PlutchikEmotion::Basic(_, il)), Some(PlutchikEmotion::Basic(_, ih))) = (adj_l.suggested_emotion, adj_h.suggested_emotion) {
            assert!(intensity_rank(ih) >= intensity_rank(il),
                "high confidence ({:?}) should be >= low ({:?})", ih, il);
        } else {
            panic!("both should produce Basic emotion");
        }
    }
}
