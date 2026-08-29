//! Architecture freeze guard for the canonical CLI entry.
//!
//! The semantic proof is `canonical_cli_bootstrap.rs`: the real bootstrap
//! builds a runtime and `execute_canonical_cli_turn` serves a turn. This guard
//! keeps the production chat path from drifting back into an agent loop.

const CLI_LIB: &str = include_str!("../src/lib.rs");

fn function_body(name: &str, next_marker: &str) -> &'static str {
    let start = CLI_LIB
        .find(name)
        .unwrap_or_else(|| panic!("missing {name} in cli lib"));
    let rest = &CLI_LIB[start..];
    let end = rest
        .find(next_marker)
        .unwrap_or_else(|| panic!("missing {next_marker} after {name}"));
    &rest[..end]
}

fn without_whitespace(source: &str) -> String {
    source.chars().filter(|c| !c.is_whitespace()).collect()
}

#[test]
fn canonical_cli_chat_path_enters_runtime_execute() {
    let body = function_body(
        "pub async fn execute_canonical_cli_turn",
        "/// Resolve a pending approval through the canonical runtime API.",
    );
    let compact = without_whitespace(body);

    assert!(
        compact.contains("runtime.execute_outcome(request)"),
        "the CLI chat path must enter Runtime::execute_outcome, not an agent loop:\n{body}"
    );
    assert!(
        body.contains("TurnRequest::new"),
        "the CLI chat path must build a canonical TurnRequest:\n{body}"
    );
}

#[test]
fn canonical_cli_bootstrap_uses_the_runtime_builder_and_its_entry_adapter() {
    let body = function_body(
        "pub async fn dispatch_canonical_chat",
        "/// Bootstrap and resolve a pending approval on the production session store.",
    );

    assert!(
        body.contains("build_canonical_runtime_from_env"),
        "the CLI chat path must bootstrap through the canonical runtime builder:\n{body}"
    );
    assert!(
        body.contains("execute_canonical_cli_turn"),
        "the CLI chat path must dispatch through the canonical CLI turn entry:\n{body}"
    );
}
