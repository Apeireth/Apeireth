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

fn execution_core_sources() -> Vec<(&'static str, &'static str)> {
    vec![
        ("approval", include_str!("../src/canonical/approval.rs")),
        ("error", include_str!("../src/canonical/error.rs")),
        ("execute", include_str!("../src/canonical/execute.rs")),
        ("module", include_str!("../src/canonical/module.rs")),
        ("provider", include_str!("../src/canonical/provider.rs")),
        ("runtime", include_str!("../src/canonical/runtime.rs")),
        ("session", include_str!("../src/canonical/session.rs")),
        ("subloop", include_str!("../src/canonical/subloop.rs")),
        ("trace", include_str!("../src/canonical/trace.rs")),
    ]
}

fn composition_sources() -> Vec<(&'static str, &'static str)> {
    vec![
        ("cognitive", include_str!("../src/canonical/cognitive.rs")),
        ("production", include_str!("../src/canonical/production.rs")),
        (
            "tool_modules",
            include_str!("../src/canonical/tool_modules.rs"),
        ),
    ]
}

fn execution_core_code() -> String {
    execution_core_sources()
        .into_iter()
        .map(|(_, source)| strip_comments(source))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn canonical_runtime_code_does_not_branch_on_vendor_identity() {
    let code = execution_core_code();

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
    let code = execution_core_code();

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
    let code = execution_core_code();

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

#[test]
fn canonical_approval_lifecycle_does_not_create_a_parallel_approval_engine() {
    let code = execution_core_code();

    for forbidden in [
        "ApprovalManager",
        "ApprovalRuntime",
        "ApprovalExecutor",
        "ApprovalEngine",
        "ApprovalRegistry",
        "ApprovalPipeline",
    ] {
        assert!(
            !code.contains(forbidden),
            "canonical runtime must own approval resume directly, not through {forbidden:?}"
        );
    }
}
#[test]
fn canonical_runtime_code_does_not_know_fetch_or_controlled_egress() {
    let code = execution_core_code();

    for forbidden in ["tool.fetch", "FetchTool", "EgressTransport", "EgressPolicy"] {
        assert!(
            !code.contains(forbidden),
            "canonical runtime must remain capability-generic; it must not mention {forbidden:?}"
        );
    }
}

#[test]
fn execution_core_does_not_special_case_standard_modules() {
    let code = execution_core_code();
    for forbidden in [
        "JudgeModule",
        "CouncilModule",
        "MemoryRecallModule",
        "MemoryWritebackModule",
        "PreferenceRecallModule",
        "SelfAssessmentModule",
        "FilesystemModule",
        "SearchModule",
        "RepoModule",
        "ShellModule",
        "FetchModule",
        "McpModule",
        "BuiltinToolsPlugin",
        "tool.filesystem",
        "tool.search",
        "tool.repo",
        "tool.shell",
        "tool.fetch",
    ] {
        assert!(
            !code.contains(forbidden),
            "execution core must not special-case {forbidden:?}"
        );
    }
}

#[test]
fn composition_files_are_scanned_and_remain_outside_execution_core() {
    let composition = composition_sources();
    assert_eq!(composition.len(), 3);
    let core = execution_core_code();
    assert!(
        !core.contains("ProductionModules"),
        "execution core must not own production composition"
    );
}
