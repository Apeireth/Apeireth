//! R177 protocol organ smoke / invariant checks (W6)
//!
//! **诚实声明 (O-5, 2026-09-24 审计 M7)**: 本模块**不是** Kani 形式化证明。
//! 原文件以 "Kani proofs" 命名, 内容是对编译期常量的值断言 (protocol 数 == 4、
//! endpoint 路径字面量、keep-alive 常量范围) —— 是有用的常量回归, 但不是
//! 形式化证明。`#[cfg(kani)]` harness 引用的 Kani proof 在本仓不存在,
//! 同样不是证明。
//!
//! 后续真形式化验证应对 domain 不变量书写 (如 wire 编解码 round-trip 的
//! 全 field 集合覆盖), 而不是留在本文件里。

#![allow(missing_docs)]

#[test]
fn r177_pr_smoke_01_protocol_count_4() {
    assert_eq!(crate::PROTOCOL_COUNT, 4);
}

#[test]
fn r177_pr_smoke_02_protocol_version_non_empty() {
    assert!(!crate::PROTOCOL_VERSION.is_empty());
}

#[test]
fn r177_pr_smoke_03_protocol_paths() {
    assert_eq!(crate::OPENAI_CHAT_PATH, "/v1/chat/completions");
    assert_eq!(crate::OPENAI_RESPONSES_PATH, "/v1/responses");
    assert_eq!(crate::ANTHROPIC_MESSAGES_PATH, "/v1/messages");
    assert!(crate::GEMINI_PATH_TEMPLATE.contains("{model}"));
}

#[test]
fn r177_pr_smoke_04_keep_alive_constants() {
    assert!(crate::KEEP_ALIVE_KEEP_ALIVE);
    assert!(crate::KEEP_ALIVE_KEEP_ALIVE_MSECS > 0);
    assert!(crate::KEEP_ALIVE_FREE_SOCKET_TIMEOUT > 0);
    assert!(crate::KEEP_ALIVE_SCHEDULING_LIFO);
    assert!(crate::KEEP_ALIVE_MAX_SOCKETS > 0);
}

#[test]
fn r177_pr_smoke_05_max_tokens_temperature() {
    assert!(crate::DEFAULT_ANTHROPIC_MAX_TOKENS > 0);
    assert!(crate::DEFAULT_ANTHROPIC_MAX_TOKENS <= 200000);
    assert!(crate::OPENAI_MAX_TEMPERATURE > 0.0);
    assert!(crate::OPENAI_MAX_TEMPERATURE <= 2.0);
    assert!(crate::ANTHROPIC_MAX_TEMPERATURE > 0.0);
    assert!(crate::ANTHROPIC_MAX_TEMPERATURE <= 1.0);
}

#[cfg(kani)]
#[kani::proof]
fn r177_pr_kani_01_protocol_count() {
    // 非证明: 见模块 doc 诚实声明 (Kani harness 在本仓不存在)。
    assert_eq!(crate::PROTOCOL_COUNT, 4);
}

#[cfg(kani)]
#[kani::proof]
fn r177_pr_kani_02_constants_positive() {
    // 非证明: 见模块 doc 诚实声明。
    assert!(crate::DEFAULT_ANTHROPIC_MAX_TOKENS > 0);
    assert!(crate::KEEP_ALIVE_KEEP_ALIVE_MSECS > 0);
}
