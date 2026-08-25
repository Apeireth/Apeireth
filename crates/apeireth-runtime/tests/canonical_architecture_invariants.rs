//! Architecture freeze guards for `apeireth-runtime::canonical`.
//!
//! These are deliberately small source-level guards. The semantic proofs live
//! in `canonical_agent_loop.rs`; here we assert only the invariants that cannot
//! be exercised semantically without importing the forbidden concrete vendor
//! crates into this test.

/// Remove Rust line and block comments so the guard reads code, not prose.
fn strip_comments(source: &str) -> String {
    let mut out = String::with_capacity(source.len());
    let mut chars = source.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '/' if chars.peek() == Some(&'/') => {
                for c in chars.by_ref() {
                    if c == '\n' {
                        break;
                    }
                }
            }
            '/' if chars.peek() == Some(&'*') => {
                chars.next();
                while let Some(c) = chars.next() {
                    if c == '*' && chars.peek() == Some(&'/') {
                        chars.next();
                        break;
                    }
                }
            }
            _ => out.push(ch),
        }
    }
    out
}

fn canonical_sources() -> Vec<(&'static str, &'static str)> {
    vec![
        ("mod", include_str!("../src/canonical/mod.rs")),
        ("error", include_str!("../src/canonical/error.rs")),
        ("execute", include_str!("../src/canonical/execute.rs")),
        ("provider", include_str!("../src/canonical/provider.rs")),
        ("runtime", include_str!("../src/canonical/runtime.rs")),
        ("session", include_str!("../src/canonical/session.rs")),
        ("trace", include_str!("../src/canonical/trace.rs")),
    ]
}

fn canonical_code() -> String {
    canonical_sources()
        .into_iter()
        .map(|(_, source)| strip_comments(source))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn canonical_runtime_code_does_not_branch_on_vendor_identity() {
    let code = canonical_code();

    for vendor in ["MiniMax", "Anthropic", "OpenAI", "Claude", "Gemini"] {
        assert!(
            !code.contains(vendor),
            "canonical runtime code must not name vendor {vendor:?}"
        );
    }

    for transport in ["Bearer", "x-api-key", "/chat/completions", "/messages"] {
        assert!(
            !code.contains(transport),
            "canonical runtime code must not own vendor transport token {transport:?}"
        );
    }
}

#[test]
fn canonical_runtime_code_does_not_construct_concrete_providers_or_http_clients() {
    let code = canonical_code();

    for forbidden in [
        "reqwest::",
        "MinimaxProvider",
        "AnthropicProvider",
        "OpenAiCompatibleProvider",
        "LlmWorker",
    ] {
        assert!(
            !code.contains(forbidden),
            "canonical runtime code must not construct {forbidden:?}; providers are injected capabilities"
        );
    }
}

#[test]
fn canonical_runtime_code_does_not_reference_process_containment_internals() {
    let code = canonical_code();

    for forbidden in [
        "JobObject",
        "RestrictedToken",
        "PlatformSandbox",
        "ProcessExecutor",
        "AssignProcessToJobObject",
    ] {
        assert!(
            !code.contains(forbidden),
            "canonical runtime code must not reach into OS process containment: {forbidden:?}"
        );
    }
}
