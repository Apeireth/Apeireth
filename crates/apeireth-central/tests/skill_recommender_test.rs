//! `apeireth-central` SkillRecommender 借鉴集成测试 (R125-16 升级)
//!
//! 借鉴 ID: `R125-16-BORROW-obra/superpowers-2026-05-2026-08-10` (per 决策 #36 §1.1 +
//! 决策 #51 §1.1 + 决策 #52). 借鉴源码 ✅ cloned (234 files, per 决策 #41 §1).
//!
//! # 8 集成测试 (per R125-16 spec)
//!
//! 1. `test_skill_recommender_tdd_for_test_keywords` — TDD skill 排第 1
//! 2. `test_skill_recommender_brainstorming_for_spec_keywords` — Brainstorming 排第 1
//! 3. `test_skill_recommender_no_match_returns_empty` — 0 匹配 → 空
//! 4. `test_skill_recommender_top_n_limits` — top N 限制
//! 5. `test_skill_recommender_sorted_by_score` — 排序从高到低
//! 6. `test_skill_recommender_case_insensitive` — case-insensitive 匹配
//! 7. `test_skill_recommender_multiple_keywords_score_higher` — 多关键词分数更高
//! 8. `test_skill_recommender_uses_registry_1to1` — 14 entry 严守

#![deny(unsafe_code)]

use apeireth_central::skill_recommender::{ScoredSkill, SkillRecommender};
use apeireth_central::skill_registry::SkillRegistry;
use apeireth_central::skill_trait::SkillId;

#[test]
fn test_skill_recommender_tdd_for_test_keywords() {
    let registry = SkillRegistry::new();
    let rec = SkillRecommender::new(&registry);
    let recs = rec.recommend("I need to write a failing test first", 3);
    assert!(!recs.is_empty());
    assert_eq!(recs[0].skill_id, SkillId::TestDrivenDevelopment);
    assert!(recs[0].score > 0);
    assert!(recs[0].matched_keywords.contains(&"test"));
    assert!(recs[0].matched_keywords.contains(&"failing"));
    assert!(recs[0].matched_keywords.contains(&"first"));
}

#[test]
fn test_skill_recommender_brainstorming_for_spec_keywords() {
    let registry = SkillRegistry::new();
    let rec = SkillRecommender::new(&registry);
    let recs = rec.recommend("Let me brainstorm the spec design alternatives", 3);
    assert!(!recs.is_empty());
    assert_eq!(recs[0].skill_id, SkillId::Brainstorming);
    assert!(recs[0].score > 0);
    assert!(recs[0].matched_keywords.contains(&"brainstorm"));
    assert!(recs[0].matched_keywords.contains(&"spec"));
}

#[test]
fn test_skill_recommender_no_match_returns_empty() {
    let registry = SkillRegistry::new();
    let rec = SkillRecommender::new(&registry);
    let recs = rec.recommend("xyzqwertynonsense", 3);
    assert!(recs.is_empty());
}

#[test]
fn test_skill_recommender_top_n_limits() {
    let registry = SkillRegistry::new();
    let rec = SkillRecommender::new(&registry);
    let recs = rec.recommend("test plan verify review", 2);
    assert!(recs.len() <= 2);
    for r in &recs {
        assert!(r.score > 0);
    }
}

#[test]
fn test_skill_recommender_sorted_by_score() {
    let registry = SkillRegistry::new();
    let rec = SkillRecommender::new(&registry);
    let recs = rec.recommend("debug bug fix root cause", 0);
    for i in 1..recs.len() {
        assert!(recs[i - 1].score >= recs[i].score);
    }
}

#[test]
fn test_skill_recommender_case_insensitive() {
    let registry = SkillRegistry::new();
    let rec = SkillRecommender::new(&registry);
    let recs1 = rec.recommend("I need to TEST this code", 3);
    let recs2 = rec.recommend("i need to test this code", 3);
    assert_eq!(recs1.len(), recs2.len());
    if !recs1.is_empty() {
        assert_eq!(recs1[0].skill_id, recs2[0].skill_id);
        assert_eq!(recs1[0].score, recs2[0].score);
    }
}

#[test]
fn test_skill_recommender_multiple_keywords_score_higher() {
    let registry = SkillRegistry::new();
    let rec = SkillRecommender::new(&registry);
    let recs1 = rec.recommend("test tdd", 1);
    let recs2 = rec.recommend("test tdd red failing first green refactor", 1);
    assert!(!recs1.is_empty() && !recs2.is_empty());
    assert!(recs2[0].score > recs1[0].score);
}

#[test]
fn test_skill_recommender_uses_registry_1to1() {
    let registry = SkillRegistry::new();
    let rec = SkillRecommender::new(&registry);
    assert_eq!(rec.registry().count(), 14);
    let total_kw = SkillRecommender::total_keywords();
    assert!(total_kw > 0);
    // 14 skill 各自 ≥ 1 关键词 (跟 R125-15e 14 entry 1:1 配合)
    for id in SkillId::ALL {
        assert!(
            !SkillRecommender::skill_keywords(id).is_empty(),
            "{id:?} must have ≥1 keyword"
        );
    }
}

#[test]
fn test_skill_recommender_threshold_filters_low_scores() {
    // 验证 threshold 过滤
    let registry = SkillRegistry::new();
    let rec = SkillRecommender::new(&registry);
    let all = rec.recommend("test debug review", 0);
    let high = rec.recommend_with_threshold("test debug review", 30);
    // 过滤后 0 多于过滤前
    assert!(high.len() <= all.len());
    for r in &high {
        assert!(r.score >= 30);
    }
}
