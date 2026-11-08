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
fn fetch_code() -> String {
    strip_comments(include_str!("../src/fetch.rs"))
}

fn fetch_code_raw() -> String {
    include_str!("../src/fetch.rs").to_string()
}

fn http_write_code() -> String {
    strip_comments(include_str!("../src/http_write.rs"))
}

fn http_write_code_raw() -> String {
    include_str!("../src/http_write.rs").to_string()
}

fn egress_code_raw() -> String {
    include_str!("../src/egress.rs").to_string()
}

#[test]
fn fetch_tool_uses_controlled_egress_and_no_second_http_client() {
    let code = fetch_code();

    for forbidden in [
        "reqwest::",
        "tokio::net::lookup_host",
        "Command::new(\"curl\")",
        "Command::new(\"wget\")",
        "ProcessExecutor",
        "is_private_host",
        "is_ssrf",
        "validate_localhost",
    ] {
        assert!(
            !code.contains(forbidden),
            "fetch tool must not bypass controlled egress with {forbidden:?}"
        );
    }

    assert!(
        fetch_code_raw().contains("get_with_headers"),
        "fetch tool must execute through ControlledEgress::get_with_headers"
    );
}

/// C-B2（能力创造线 W1）：**带鉴权的 HTTP 写面**。
///
/// 两条钉死：
///
/// ① 新面文件内**零** HTTP 客户端痕迹 ⇒ 写请求只能走受控出网通道；
/// ② 写方法**必须长在** `ControlledEgress` 上（否则写面就会自带第二套
///    白名单 / 私网判定 —— 那是仓内既有不变量要拦的东西）。
#[test]
fn http_write_tool_uses_controlled_egress_and_no_second_http_client() {
    let code = http_write_code();

    for forbidden in [
        "reqwest::",
        "tokio::net::lookup_host",
        "Command::new(\"curl\")",
        "Command::new(\"wget\")",
        "ProcessExecutor",
        "is_private_host",
        "is_ssrf",
        "validate_localhost",
    ] {
        assert!(
            !code.contains(forbidden),
            "http write tool must not bypass controlled egress with {forbidden:?}"
        );
    }

    // ⚠️ 连**注释里**都不许出现 HTTP 客户端库名（新文件零命中）。
    let raw = http_write_code_raw();
    assert!(
        !raw.contains("reqwest"),
        "http write tool source must not mention an HTTP client library at all"
    );

    assert!(
        raw.contains("ControlledEgress"),
        "http write tool must execute through the controlled egress transport"
    );
    assert!(
        raw.contains(".write("),
        "http write tool must call the controlled egress write method"
    );
    assert!(
        !raw.contains("get_with_headers"),
        "http write tool must not reach the network through the read path"
    );

    // 写方法**长在**受控通道上（同一套白名单 / 私网判定 / 闸）。
    let egress_raw = egress_code_raw();
    assert!(
        egress_raw.contains("pub async fn write("),
        "the controlled egress transport must own the write method"
    );
    assert!(
        egress_raw.contains("EgressMethod"),
        "the controlled egress transport must own the write method vocabulary"
    );
}
