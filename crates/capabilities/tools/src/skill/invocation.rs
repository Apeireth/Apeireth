//! 调用面 —— 技能卡进入模型上下文的唯一方式是 **L2 封套披露** (检索式注入)。
//!
//! 上下文预算纪律 (红线): **绝不无差别全量注入**。技能库是长期资产, 只有
//! 两类卡进入上下文:
//!
//! 1. **触发词命中**: 对话文本命中卡的触发词 ( [`trigger_hit_count`] );
//! 2. **按需拉取**: 调用方点名卡名 ( [`name_pull`] )。
//!
//! 命中的卡经 [`build_skill_disclosure`] 披露 —— 与记忆检索披露同一套
//! **共享引用封套** ([`UntrustedEnvelope`]: 固定警告头 + 显式边界 + 边界伪造
//! 转义) 与**同一套预算换算** ([`EnvelopeBudget::from_total_budget_chars`]:
//! 每源 `max(400, 总预算/4)` + 分级省略措辞), 一个预算系统管到底。
//! 未命中的卡**零注入** (正文连一个字都不进上下文)。

use apeireth_orchestration::untrusted_envelope::{
    EnvelopeBudget, EnvelopeCompleteness, UntrustedEnvelope,
};

use super::card::{name_key, SkillCard};

/// 披露预算缺省值 (字符): 与调用方的上下文预算同参换算, 缺省给 4000。
pub const DEFAULT_SKILL_DISCLOSURE_BUDGET_CHARS: usize = 4_000;

/// 按名拉取的最短查询键 (字符): 过短的点名不做子串拉取 (防单字符误命中)。
pub const SKILL_NAME_QUERY_MIN_CHARS: usize = 2;

/// L2 披露头 (技能卡专属一行, 封套本体与记忆检索披露同款)。
pub const SKILL_DISCLOSURE_HEADER: &str =
    "【技能卡 · L2 检索披露】(仅触发词命中 / 按需拉取的技能卡进入上下文; 每卡按共享预算披露)";

/// 触发词命中数: 查询文本命中卡片触发词的个数 (大小写折叠子串命中)。
pub fn trigger_hit_count(card: &SkillCard, query: &str) -> usize {
    let query = query.to_lowercase();
    card.triggers
        .iter()
        .filter(|trigger| !trigger.trim().is_empty() && query.contains(&trigger.to_lowercase()))
        .count()
}

/// 按需拉取判定: 查询点名了这张卡 (卡名键互为子串, 查询键至少
/// [`SKILL_NAME_QUERY_MIN_CHARS`] 字符)。
pub fn name_pull(card: &SkillCard, query: &str) -> bool {
    let query_key = name_key(query);
    if query_key.chars().count() < SKILL_NAME_QUERY_MIN_CHARS {
        return false;
    }
    let card_key = name_key(&card.name);
    card_key.contains(&query_key) || query_key.contains(&card_key)
}

/// 检索式选择 (调用面第一拍): 只取**命中**的卡 (触发词命中 / 按需拉取),
/// 命中数降序 + 卡名键升序 + id 升序 (确定性), 截 `limit`。未命中的卡
/// 不出现在结果里 —— 全量注入在此结构性不可能。
pub fn select_for_context<'a>(
    cards: &'a [SkillCard],
    query: &str,
    limit: usize,
) -> Vec<&'a SkillCard> {
    let mut matched: Vec<(usize, &SkillCard)> = cards
        .iter()
        .filter_map(|card| {
            let hits = trigger_hit_count(card, query);
            if hits > 0 || name_pull(card, query) {
                Some((hits, card))
            } else {
                None
            }
        })
        .collect();
    matched.sort_by(|a, b| {
        b.0.cmp(&a.0)
            .then_with(|| name_key(&a.1.name).cmp(&name_key(&b.1.name)))
            .then_with(|| a.1.id.cmp(&b.1.id))
    });
    matched
        .into_iter()
        .take(limit)
        .map(|(_, card)| card)
        .collect()
}

/// L2 封套披露 (调用面第二拍): 命中卡逐卡进共享引用封套, 按共享预算的
/// 每源份额披露 (超额走分级省略措辞)。空输入空串 (不注入)。
pub fn build_skill_disclosure(cards: &[&SkillCard], total_budget_chars: usize) -> String {
    if cards.is_empty() {
        return String::new();
    }
    let budget = EnvelopeBudget::from_total_budget_chars(total_budget_chars);
    let mut out = String::from(SKILL_DISCLOSURE_HEADER);
    out.push('\n');
    for card in cards {
        let envelope = UntrustedEnvelope::new(
            card.id.clone(),
            card.disclosure_payload(),
            EnvelopeCompleteness::Complete,
        );
        out.push_str(&envelope.disclose(budget, None).text);
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skill::card::{SkillSource, SkillSourceKind};

    fn card(name: &str, triggers: &[&str], body: &str) -> SkillCard {
        SkillCard {
            id: crate::skill::card::skill_card_id(name),
            name: name.into(),
            summary: "摘要".into(),
            triggers: triggers.iter().map(|t| (*t).to_string()).collect(),
            body: body.into(),
            source: SkillSource {
                kind: SkillSourceKind::Book,
                title: "讲义".into(),
            },
            created_at: 0,
        }
    }

    #[test]
    fn trigger_hit_counts_each_trigger_once() {
        let c = card("换元法", &["换元", "dx"], "正文");
        assert_eq!(trigger_hit_count(&c, "这题要用换元, 还要换 dx"), 2);
        assert_eq!(trigger_hit_count(&c, "这题要用换元"), 1);
        assert_eq!(trigger_hit_count(&c, "与本卡无关"), 0);
    }

    #[test]
    fn selection_keeps_only_hit_cards() {
        let cards = vec![
            card("换元法", &["换元"], "换元正文"),
            card("分部积分", &["分部"], "分部正文"),
        ];
        let picked = select_for_context(&cards, "换元怎么做", 8);
        assert_eq!(picked.len(), 1);
        assert_eq!(picked[0].name, "换元法");
    }

    #[test]
    fn name_pull_fetches_a_named_card_on_demand() {
        let c = card("分部积分法", &["分部"], "正文");
        assert!(name_pull(&c, "分部积分法怎么用"));
        assert!(name_pull(&c, "讲讲分部积分法"));
        assert!(!name_pull(&c, "x"), "过短查询键不拉取");
    }
}
