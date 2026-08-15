//! `apeireth-companion::tone` — 语调桥: Bond 关系 → 语调提示 (渲染层用).
//!
//! 对接哲学「陪伴 = 关系可能性 + voice」: 关系 (Bond.character) 调制表达语调。
//! voice crate 的 `bond_to_tone` 产出真实 `Tone` 枚举 (音频层); 本模块是
//! companion 侧的轻量桥 — 把信任/共鸣映射成**中文语调提示**, 注入 LLM 渲染 prompt,
//! 让「他的话」自然带上关系温度 (不依赖 voice crate, 避免循环依赖)。

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
}
