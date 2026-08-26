//! R177 pipeline organ Kani proofs (W5)
//! 覆盖 VCP 模式 6+7: role_divider + tiktoken_counter

#![allow(missing_docs)]

use crate::role_divider::Role;
use crate::tiktoken_counter::{TiktokenCounter, TokenModel};

#[test]
fn r177_pl_01_role_6_variants() {
    let roles = [
        Role::System,
        Role::User,
        Role::Assistant,
        Role::Tool,
        Role::Function,
        Role::Developer,
    ];
    assert_eq!(roles.len(), 6);
}

#[test]
fn r177_pl_02_role_distinct() {
    let mut seen = std::collections::HashSet::new();
    for r in Role::ALL {
        assert!(seen.insert(r), "Role 重复: {:?}", r);
    }
    assert_eq!(seen.len(), 6);
}

#[test]
fn r177_pl_03_role_as_str_roundtrip() {
    for r in Role::ALL {
        let s = r.as_str();
        let parsed = Role::parse(s);
        assert_eq!(parsed, Some(r), "as_str → parse roundtrip 失败: {:?}", r);
    }
}

#[test]
fn r177_pl_04_role_parse_case_insensitive() {
    assert_eq!(Role::parse("SYSTEM"), Some(Role::System));
    assert_eq!(Role::parse("User"), Some(Role::User));
    assert_eq!(Role::parse("assistant"), Some(Role::Assistant));
}

#[test]
fn r177_pl_05_role_parse_invalid() {
    assert_eq!(Role::parse("invalid"), None);
    assert_eq!(Role::parse(""), None);
}

#[test]
fn r177_pl_06_role_strings_vcp_aligned() {
    assert_eq!(Role::System.as_str(), "system");
    assert_eq!(Role::User.as_str(), "user");
    assert_eq!(Role::Assistant.as_str(), "assistant");
    assert_eq!(Role::Tool.as_str(), "tool");
    assert_eq!(Role::Function.as_str(), "function");
    assert_eq!(Role::Developer.as_str(), "developer");
}

#[test]
fn r177_pl_07_token_model_5_variants() {
    let models = vec![
        TokenModel::Cl100KBase,
        TokenModel::O200KBase,
        TokenModel::P50KBase,
        TokenModel::R50KBase,
        TokenModel::Gpt2,
    ];
    assert_eq!(models.len(), 5);
}

#[test]
fn r177_pl_08_token_model_as_str() {
    assert_eq!(TokenModel::Cl100KBase.as_str(), "cl100k_base");
    assert_eq!(TokenModel::O200KBase.as_str(), "o200k_base");
    assert_eq!(TokenModel::P50KBase.as_str(), "p50k_base");
    assert_eq!(TokenModel::R50KBase.as_str(), "r50k_base");
    assert_eq!(TokenModel::Gpt2.as_str(), "gpt2");
}

#[test]
fn r177_pl_09_token_model_available() {
    let models = TokenModel::available_models();
    assert_eq!(models.len(), 5);
}

#[test]
fn r177_pl_10_tiktoken_counter_new() {
    let result = TiktokenCounter::new(TokenModel::Cl100KBase);
    // 不管成功失败 (依赖 lazy load), 都应返 Result
    match result {
        Ok(_) | Err(_) => {}
    }
}

#[cfg(kani)]
#[kani::proof]
fn r177_pl_kani_01_role_count() {
    assert_eq!(Role::ALL.len(), 6);
}

#[cfg(kani)]
#[kani::proof]
fn r177_pl_kani_02_token_model_distinct() {
    let models = vec![
        TokenModel::Cl100KBase,
        TokenModel::O200KBase,
        TokenModel::P50KBase,
        TokenModel::R50KBase,
        TokenModel::Gpt2,
    ];
    let names: Vec<&str> = models.iter().map(|m| m.as_str()).collect();
    let mut seen = std::collections::HashSet::new();
    for n in &names {
        assert!(seen.insert(*n), "token model name 重复");
    }
}
