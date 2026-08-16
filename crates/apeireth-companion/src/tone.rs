//! `apeireth-companion::tone` — 语调桥: Bond 关系 → 语调提示 (渲染层用).
//!
//! 对接哲学「陪伴 = 关系可能性 + voice」: 关系 (Bond.character) 调制表达语调。
//! voice crate 的 `bond_to_tone` 产出真实 `Tone` 枚举 (音频层); 本模块是
//! companion 侧的轻量桥 — 把信任/共鸣映射成**中文语调提示**, 注入 LLM 渲染 prompt,
//! 让「他的话」自然带上关系温度 (不依赖 voice crate, 避免循环依赖)。
//!
//! **A3 人格化深化 (2026-08-16)**: 三层器官语调 —
//! 1. 关系基线 (`tone_hint`, Bond) — 已有
//! 2. 情绪调制 (`emotion_tone`, consciousness ResponseStyle → 确定性语气措辞)
//! 3. 审议强度 (`deliberation_intensity`, council 加权分/置信度 → 措辞强度)
//! `organ_tone` 把三层合成一句提示; `ToneRefiner` 是 LLM 措辞注入 trait 口
//! (0 装 PASS: 口已留, 本 crate 无 LLM 依赖, 实现由部署层注入, 未接)。

use std::fmt;

use apeireth_consciousness::emotion::ResponseStyle;

use crate::Bond;

/// 关系 → 语调提示 (注入渲染层).
pub fn tone_hint(bond: &Bond) -> &'static str {
    let c = bond.character();
    if c.trust >= 0.6 && c.resonance >= 0.6 {
        "轻松亲切, 像老朋友一样自然"
    } else if c.trust >= 0.4 {
        "温暖自然, 带一点熟稔"
    } else if c.resonance >= 0.4 {
        "温和关切, 保持合适的分寸"
    } else {
        "礼貌克制, 谨慎而友好"
    }
}

// ============================================================
// A3 第 2 层: 情绪状态 → 语气 (确定性映射)
// ============================================================

/// 情绪风格 → 语气措辞 (确定性映射, 7 档全覆盖).
///
/// 输入是 consciousness `EmotionEngine::response_style()` 的产出
/// (基于最近一次情绪快照的 dominant + intensity)。
/// 纯函数: 同一 style 永远产出同一措辞 (可测、可审计)。
pub fn emotion_tone(style: ResponseStyle) -> &'static str {
    match style {
        ResponseStyle::Warm => "明朗温暖, 情绪自然流动",
        ResponseStyle::Friendly => "轻松友好, 像近邻般随和",
        ResponseStyle::Gentle => "轻柔舒缓, 带着关照",
        ResponseStyle::Cautious => "沉稳谨慎, 字斟句酌",
        ResponseStyle::Diplomatic => "平稳客观, 不走极端",
        ResponseStyle::Curious => "好奇探索, 喜欢追问",
        ResponseStyle::Professional => "简洁专业, 情绪收敛",
    }
}

// ============================================================
// A3 第 3 层: 审议结果 → 措辞强度 (确定性映射)
// ============================================================

/// 审议结果的回声 — organs.rs 在每次 council 审议后捕获, 供语调合成用。
/// 字段来自 `CouncilVerdict.report` (加权分归一化到 [-1, +1], 置信度 [0, 1])。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DeliberationEcho {
    /// 智囊团加权总分 (归一化 [-1, +1])
    pub weighted_score: f64,
    /// 综合置信度 [0, 1]
    pub confidence: f64,
}

/// 语调层的输入校验错误 (0 装 PASS: 非法输入明确报错, 不静默兜底)。
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ToneError {
    /// weighted_score 为 NaN 或超出 [-1, 1]
    InvalidScore(f64),
    /// confidence 为 NaN 或超出 [0, 1]
    InvalidConfidence(f64),
}

impl fmt::Display for ToneError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidScore(v) => write!(
                f,
                "审议加权分非法: {v} (应在 [-1, 1] 且非 NaN; 检查 CouncilVerdict.report.weighted_score 归一化)"
            ),
            Self::InvalidConfidence(v) => write!(
                f,
                "审议置信度非法: {v} (应在 [0, 1] 且非 NaN; 检查 CouncilVerdict.report.confidence)"
            ),
        }
    }
}

impl std::error::Error for ToneError {}

/// 审议加权分 → 措辞强度 (确定性映射).
///
/// 分档: 强共识 (≥0.5 且置信 ≥0.6) → 坚定; 倾向同意 (≥0) → 从容;
/// 有异议 (>-0.5) → 留余地; 强烈反对 (≤-0.5) → 克制。
/// 非法输入 (NaN/越界) → Err (0 装 PASS, 由调用方决定降级路径)。
pub fn deliberation_intensity(weighted_score: f64, confidence: f64) -> Result<&'static str, ToneError> {
    if weighted_score.is_nan() || !(-1.0..=1.0).contains(&weighted_score) {
        return Err(ToneError::InvalidScore(weighted_score));
    }
    if confidence.is_nan() || !(0.0..=1.0).contains(&confidence) {
        return Err(ToneError::InvalidConfidence(confidence));
    }
    Ok(match (weighted_score, confidence) {
        (s, c) if s >= 0.5 && c >= 0.6 => "智囊团高度一致, 措辞可以明确坚定",
        (s, _) if s >= 0.0 => "智囊团倾向同意, 措辞自然从容",
        (s, _) if s > -0.5 => "智囊团有异议, 措辞收敛留余地",
        _ => "智囊团强烈反对, 措辞务必克制",
    })
}

// ============================================================
// A3 合成: 三层器官语调 + LLM 措辞注入 trait 口
// ============================================================

/// LLM 措辞注入 trait 口 (机制留口, 实现未接).
///
/// 确定性映射 ([`organ_tone`]) 永远可用且先行; 部署层若想让 LLM
/// 对措辞做个性化润色, 实现本 trait 并传入 [`organ_tone_refined`]。
/// 返回 `None` = 保留确定性结果 (降级路径明确)。
pub trait ToneRefiner: Send + Sync {
    /// 对确定性合成的语调提示做 LLM 润色; 返回 None = 采用确定性结果。
    fn refine(&self, base_hint: &str) -> Option<String>;
}

/// 三层器官语调合成: 关系基线 (Bond) × 情绪风格 (consciousness) × 审议强度 (council)。
///
/// 确定性、纯函数: 同一输入永远产出同一提示。
/// - `deliberation` 为 None (尚未发生过审议) → 只合成前两层。
/// - 审议回声分值非法 → 不静默丢弃: 显式降级为「保守克制」档并留痕 (0 装 PASS)。
pub fn organ_tone(bond: &Bond, style: ResponseStyle, deliberation: Option<&DeliberationEcho>) -> String {
    let mut parts: Vec<String> = vec![
        tone_hint(bond).to_string(),
        emotion_tone(style).to_string(),
    ];
    if let Some(d) = deliberation {
        match deliberation_intensity(d.weighted_score, d.confidence) {
            Ok(hint) => parts.push(hint.to_string()),
            Err(e) => {
                eprintln!("[tone] 审议强度降级: {e}");
                parts.push("审议分值异常, 措辞保守克制".to_string());
            }
        }
    }
    parts.join("; ")
}

/// 带 LLM 注入口的三层语调合成: refiner 返回 Some 时采用其措辞, 否则回退确定性结果。
pub fn organ_tone_refined(
    bond: &Bond,
    style: ResponseStyle,
    deliberation: Option<&DeliberationEcho>,
    refiner: Option<&dyn ToneRefiner>,
) -> String {
    let base = organ_tone(bond, style, deliberation);
    refiner.and_then(|r| r.refine(&base)).unwrap_or(base)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Bond, BondStage};

    #[test]
    fn default_bond_is_reserved() {
        let b = Bond::new();
        assert_eq!(tone_hint(&b), "礼貌克制, 谨慎而友好");
    }

    #[test]
    fn trusted_bond_is_friendly() {
        let mut b = Bond::new();
        b.evolve(BondStage::Trusted, 0.6);
        {
            let c = b.character_mut();
            c.trust = 0.8;
            c.resonance = 0.7;
        }
        let hint = tone_hint(&b);
        assert!(hint.contains("老朋友"), "高信任+高共鸣应老朋友: {hint}");
    }

    #[test]
    fn mid_trust_is_warm() {
        let mut b = Bond::new();
        b.evolve(BondStage::Familiar, 0.5);
        {
            let c = b.character_mut();
            c.trust = 0.5;
            c.resonance = 0.2;
        }
        let hint = tone_hint(&b);
        assert!(hint.contains("温暖"), "中段信任应温暖: {hint}");
    }

    // ============ A3: 情绪档 → 语气 (确定性, 7 档全覆盖) ============

    #[test]
    fn emotion_tone_all_seven_styles_deterministic_and_distinct() {
        let styles = [
            ResponseStyle::Warm,
            ResponseStyle::Friendly,
            ResponseStyle::Gentle,
            ResponseStyle::Cautious,
            ResponseStyle::Diplomatic,
            ResponseStyle::Curious,
            ResponseStyle::Professional,
        ];
        let hints: Vec<&str> = styles.iter().map(|s| emotion_tone(*s)).collect();
        // 每档非空
        for h in &hints {
            assert!(!h.is_empty());
        }
        // 7 档互不相同 (映射不塌缩)
        for i in 0..hints.len() {
            for j in (i + 1)..hints.len() {
                assert_ne!(hints[i], hints[j], "档 {} 与 {} 措辞塌缩", i, j);
            }
        }
        // 确定性: 重复调用结果一致
        for s in styles {
            assert_eq!(emotion_tone(s), emotion_tone(s));
        }
        // 抽查关键档的语义锚点
        assert!(emotion_tone(ResponseStyle::Warm).contains("温暖"));
        assert!(emotion_tone(ResponseStyle::Cautious).contains("谨慎"));
        assert!(emotion_tone(ResponseStyle::Diplomatic).contains("客观"));
    }

    // ============ A3: 审议 → 措辞强度 (确定性分档) ============

    #[test]
    fn deliberation_intensity_tiers() {
        // 强共识: 高分 + 高置信 → 坚定
        assert_eq!(
            deliberation_intensity(0.8, 0.9).unwrap(),
            "智囊团高度一致, 措辞可以明确坚定"
        );
        // 高分但低置信 → 不算坚定, 落「从容」档
        assert_eq!(
            deliberation_intensity(0.8, 0.3).unwrap(),
            "智囊团倾向同意, 措辞自然从容"
        );
        // 边界: 0.5/0.6 恰入坚定档
        assert_eq!(
            deliberation_intensity(0.5, 0.6).unwrap(),
            "智囊团高度一致, 措辞可以明确坚定"
        );
        // 中性 → 从容
        assert_eq!(
            deliberation_intensity(0.0, 0.5).unwrap(),
            "智囊团倾向同意, 措辞自然从容"
        );
        // 轻微异议 → 留余地
        assert_eq!(
            deliberation_intensity(-0.2, 0.5).unwrap(),
            "智囊团有异议, 措辞收敛留余地"
        );
        // 边界: -0.5 恰入克制档
        assert_eq!(
            deliberation_intensity(-0.5, 0.5).unwrap(),
            "智囊团强烈反对, 措辞务必克制"
        );
        // 强烈反对 → 克制
        assert_eq!(
            deliberation_intensity(-0.9, 0.8).unwrap(),
            "智囊团强烈反对, 措辞务必克制"
        );
    }

    #[test]
    fn deliberation_intensity_rejects_invalid_input() {
        // NaN 分数 (NaN != NaN, 用 matches! 断言)
        let e = deliberation_intensity(f64::NAN, 0.5).unwrap_err();
        assert!(matches!(e, ToneError::InvalidScore(v) if v.is_nan()));
        assert!(e.to_string().contains("[-1, 1]"), "错误信息可行动: {e}");
        // 越界分数 (>1 / <-1)
        assert!(matches!(
            deliberation_intensity(1.5, 0.5),
            Err(ToneError::InvalidScore(_))
        ));
        assert!(matches!(
            deliberation_intensity(-1.01, 0.5),
            Err(ToneError::InvalidScore(_))
        ));
        // NaN / 越界置信度
        assert!(matches!(
            deliberation_intensity(0.5, f64::NAN),
            Err(ToneError::InvalidConfidence(_))
        ));
        assert!(matches!(
            deliberation_intensity(0.5, 1.2),
            Err(ToneError::InvalidConfidence(_))
        ));
        // 边界合法值不误杀
        assert!(deliberation_intensity(-1.0, 0.0).is_ok());
        assert!(deliberation_intensity(1.0, 1.0).is_ok());
    }

    // ============ A3: 三层合成 + LLM 注入口 ============

    #[test]
    fn organ_tone_without_deliberation_has_two_layers() {
        let b = Bond::new();
        let t = organ_tone(&b, ResponseStyle::Friendly, None);
        // 关系基线 + 情绪层都在
        assert!(t.contains("礼貌克制"), "应含关系基线: {t}");
        assert!(t.contains("轻松友好"), "应含情绪语气: {t}");
        assert!(!t.contains("智囊团"), "无审议时不应有强度层: {t}");
    }

    #[test]
    fn organ_tone_with_deliberation_has_three_layers() {
        let b = Bond::new();
        let echo = DeliberationEcho {
            weighted_score: 0.7,
            confidence: 0.8,
        };
        let t = organ_tone(&b, ResponseStyle::Gentle, Some(&echo));
        assert!(t.contains("轻柔舒缓"), "情绪层: {t}");
        assert!(t.contains("明确坚定"), "审议强度层: {t}");
    }

    #[test]
    fn organ_tone_invalid_echo_degrades_honestly() {
        let b = Bond::new();
        // 非法分值不静默丢弃 → 显式降级留痕 (0 装 PASS)
        let echo = DeliberationEcho {
            weighted_score: 99.0,
            confidence: 0.5,
        };
        let t = organ_tone(&b, ResponseStyle::Professional, Some(&echo));
        assert!(t.contains("审议分值异常"), "非法分值应显式降级: {t}");
    }

    #[test]
    fn organ_tone_refined_uses_refiner_or_falls_back() {
        struct UppercaseRefiner;
        impl ToneRefiner for UppercaseRefiner {
            fn refine(&self, base: &str) -> Option<String> {
                Some(format!("[LLM]{base}"))
            }
        }
        struct NoopRefiner;
        impl ToneRefiner for NoopRefiner {
            fn refine(&self, _base: &str) -> Option<String> {
                None // 保持确定性结果
            }
        }
        let b = Bond::new();
        // 有 refiner 且返回 Some → 采用 LLM 措辞
        let t = organ_tone_refined(&b, ResponseStyle::Curious, None, Some(&UppercaseRefiner));
        assert!(t.starts_with("[LLM]"), "refiner 措辞应被采用: {t}");
        // refiner 返回 None → 回退确定性结果
        let base = organ_tone(&b, ResponseStyle::Curious, None);
        assert_eq!(
            organ_tone_refined(&b, ResponseStyle::Curious, None, Some(&NoopRefiner)),
            base
        );
        // 无 refiner → 确定性结果
        assert_eq!(
            organ_tone_refined(&b, ResponseStyle::Curious, None, None),
            base
        );
    }
}
