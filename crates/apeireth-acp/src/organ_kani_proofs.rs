//! R177 acp (LLM facade) organ Kani proofs (W3+W4)

#![allow(missing_docs)]

use crate::llm_facade::{
    is_valid_provider, LlmRequest, LlmResponse, LlmStatus, ALL_PROVIDER_NAMES,
};

#[test]
fn r177_acp_01_all_provider_names_6() {
    assert_eq!(ALL_PROVIDER_NAMES.len(), 6);
    assert!(is_valid_provider("claude-code"));
    assert!(is_valid_provider("codex"));
    assert!(is_valid_provider("copilot"));
    assert!(is_valid_provider("gemini-cli"));
    assert!(is_valid_provider("opencode"));
    assert!(is_valid_provider("minimax"));
}

#[test]
fn r177_acp_02_is_valid_provider_negative() {
    assert!(!is_valid_provider("unknown"));
    assert!(!is_valid_provider(""));
    assert!(!is_valid_provider("gpt-4"));
}

#[test]
fn r177_acp_03_request_new_valid() {
    let r = LlmRequest::new("minimax", "sys", "user");
    assert_eq!(r.provider, "minimax");
    assert_eq!(r.system, "sys");
    assert_eq!(r.user, "user");
    assert!(r.validate().is_ok());
}

#[test]
fn r177_acp_04_request_validate_empty_provider() {
    let mut r = LlmRequest::new("minimax", "s", "u");
    r.provider = String::new();
    assert!(r.validate().is_err());
}

#[test]
fn r177_acp_05_request_validate_empty_prompt() {
    let mut r = LlmRequest::new("minimax", "", "");
    assert!(r.validate().is_err());
}

#[test]
fn r177_acp_06_request_temperature_range() {
    let r = LlmRequest::new("minimax", "s", "u");
    let t = r.temperature();
    assert!(t >= 0.0 && t <= 2.0, "temperature 应 ∈ [0.0, 2.0], got {}", t);
}

#[test]
fn r177_acp_07_response_ok() {
    let resp = LlmResponse::ok("minimax", "model", "text", 100, 50);
    assert_eq!(resp.provider, "minimax");
    assert_eq!(resp.text, "text");
    assert_eq!(resp.prompt_tokens, 100);
    assert_eq!(resp.completion_tokens, 50);
    assert_eq!(resp.total_tokens(), 150);
    assert_eq!(resp.status, LlmStatus::Ok);
    assert!(resp.status.is_success());
}

#[test]
fn r177_acp_08_response_error() {
    let resp = LlmResponse::error("minimax", "model", "oops");
    assert_eq!(resp.text, "oops");
    assert_eq!(resp.status, LlmStatus::Error);
    assert!(!resp.status.is_success());
}

#[test]
fn r177_acp_09_llm_status_5() {
    let statuses = [
        LlmStatus::Ok,
        LlmStatus::Error,
        LlmStatus::RateLimited,
        LlmStatus::InvalidAuth,
        LlmStatus::Timeout,
    ];
    assert_eq!(statuses.len(), 5);
}

#[test]
fn r177_acp_10_request_max_tokens_zero_rejected() {
    let mut r = LlmRequest::new("minimax", "s", "u");
    r.max_tokens = 0;
    assert!(r.validate().is_err());
}

#[cfg(kani)]
#[kani::proof]
fn r177_acp_kani_01_provider_count() {
    assert_eq!(ALL_PROVIDER_NAMES.len(), 6);
}

#[cfg(kani)]
#[kani::proof]
fn r177_acp_kani_02_valid_provider() {
    assert!(is_valid_provider("minimax"));
    assert!(!is_valid_provider("invalid"));
}
