//! Architecture guards for the canonical process execution boundary.
//!
//! The process module is physical containment, not governance and not runtime
//! orchestration. These source-level guards keep it that way.

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

fn process_sources() -> Vec<(&'static str, &'static str)> {
    vec![
        ("mod", include_str!("../src/process/mod.rs")),
        ("windows", include_str!("../src/process/windows.rs")),
        ("linux", include_str!("../src/process/linux.rs")),
        ("macos", include_str!("../src/process/macos.rs")),
        ("platform", include_str!("../src/process/platform.rs")),
    ]
}

fn egress_code() -> String {
    strip_comments(include_str!("../src/egress.rs"))
}

fn process_code() -> String {
    process_sources()
        .into_iter()
        .map(|(_, source)| strip_comments(source))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn process_infrastructure_does_not_depend_on_runtime_gateway_or_provider() {
    let code = process_code();

    for forbidden in [
        "apeireth_runtime",
        "apeireth_gateway",
        "apeireth_provider",
        "ToolRegistry",
        "SandboxRegistry",
        "ProcessRegistry",
        "ExecutionManager",
        "GovernancePipeline",
        "PlatformManager",
        "BackendRegistry",
        "SecurityFacade",
    ] {
        assert!(
            !code.contains(forbidden),
            "process execution infrastructure must not depend on or recreate {forbidden:?}"
        );
    }
}

#[test]
fn process_infrastructure_does_not_create_a_shell_backdoor() {
    let code = process_code();

    for forbidden in ["cmd /c", "sh -c", "shell_words", "command: String"] {
        assert!(
            !code.contains(forbidden),
            "process execution boundary must not accept shell command strings: {forbidden:?}"
        );
    }
}

#[test]
fn egress_infrastructure_does_not_depend_on_runtime_gateway_or_provider() {
    let code = egress_code();

    for forbidden in [
        "apeireth_runtime",
        "apeireth_gateway",
        "apeireth_provider",
        "apeireth_companion",
        "FetchTool",
        "BrowserTool",
        "ToolRegistry",
        "GovernancePipeline",
    ] {
        assert!(
            !code.contains(forbidden),
            "controlled egress infrastructure must not depend on {forbidden:?}"
        );
    }
}

#[test]
fn public_contract_does_not_name_platform_sandbox_mechanisms() {
    let public_contract = strip_comments(include_str!("../src/process/mod.rs"));

    // The public canonical contract only names platform-agnostic capabilities.
    // Platform mechanism names belong to backend modules.
    for forbidden in [
        "JobObject",
        "RestrictedToken",
        "CreateProcessW",
        "cgroup",
        "seccomp",
        "Landlock",
        "namespace",
        "setrlimit",
        "Seatbelt",
        "sandbox-exec",
    ] {
        assert!(
            !public_contract.contains(forbidden),
            "public canonical process contract must not expose {forbidden:?}"
        );
    }
}
