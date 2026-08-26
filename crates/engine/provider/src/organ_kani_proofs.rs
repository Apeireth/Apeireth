//! R177 provider organ Kani proofs (W3) — VCP 模式 7+8 model_router

#![allow(missing_docs)]

use crate::http_dispatch::{
    config_for_claude_code, config_for_codex, config_for_copilot, config_for_gemini_cli,
    config_for_minimax, config_for_opencode, configs_for_all, ProviderConfig,
};

#[test]
fn r177_prv_01_configs_for_all_6() {
    let configs = configs_for_all("test-key");
    assert_eq!(configs.len(), 6, "configs_for_all 应给 6 Provider configs");
}

#[test]
fn r177_prv_02_configs_unique_providers() {
    let configs = configs_for_all("test-key");
    let names: Vec<&str> = configs.iter().map(|c| c.provider_name).collect();
    let mut seen = std::collections::HashSet::new();
    for n in &names {
        assert!(seen.insert(*n), "provider_name 重复: {}", n);
    }
    assert_eq!(seen.len(), 6);
}

#[test]
fn r177_prv_03_config_new_fields() {
    let c = ProviderConfig::new("test", "https://api.test", "key123", "model-x");
    assert_eq!(c.provider_name, "test");
    assert_eq!(c.base_url, "https://api.test");
    assert_eq!(c.api_key, "key123");
    assert_eq!(c.default_model, "model-x");
}

#[test]
fn r177_prv_04_config_for_claude_code() {
    let c = config_for_claude_code("k");
    assert_eq!(c.provider_name, "claude-code");
    assert_eq!(c.default_model, "claude-sonnet-4-5");
    assert!(c.base_url.contains("anthropic"));
}

#[test]
fn r177_prv_05_config_for_codex() {
    let c = config_for_codex("k");
    assert_eq!(c.provider_name, "codex");
    assert!(c.base_url.contains("openai"));
}

#[test]
fn r177_prv_06_config_for_copilot() {
    let c = config_for_copilot("k");
    assert_eq!(c.provider_name, "copilot");
    assert_eq!(c.default_model, "gpt-4o");
    assert!(c.base_url.contains("github"));
}

#[test]
fn r177_prv_07_config_for_gemini_cli() {
    let c = config_for_gemini_cli("k");
    assert_eq!(c.provider_name, "gemini-cli");
    assert!(c.base_url.contains("googleapis"));
}

#[test]
fn r177_prv_08_config_for_opencode() {
    let c = config_for_opencode("k");
    assert_eq!(c.provider_name, "opencode");
}

#[test]
fn r177_prv_09_config_for_minimax() {
    let c = config_for_minimax("k");
    assert_eq!(c.provider_name, "minimax");
    assert!(c.base_url.contains("minimaxi"));
}

#[test]
fn r177_prv_10_all_configs_have_non_empty_fields() {
    let configs = configs_for_all("test-key");
    for c in &configs {
        assert!(!c.provider_name.is_empty());
        assert!(!c.base_url.is_empty());
        assert!(!c.api_key.is_empty());
        assert!(!c.default_model.is_empty());
    }
}

#[cfg(kani)]
#[kani::proof]
fn r177_prv_kani_01_configs_count() {
    let configs = configs_for_all("k");
    assert_eq!(configs.len(), 6);
}

#[cfg(kani)]
#[kani::proof]
fn r177_prv_kani_02_configs_distinct() {
    let configs = configs_for_all("k");
    let names: Vec<&str> = configs.iter().map(|c| c.provider_name).collect();
    let mut seen = std::collections::HashSet::new();
    for n in &names {
        assert!(seen.insert(*n), "provider name 重复");
    }
}
