//! Architecture freeze guards for the canonical provider capabilities.
//!
//! The semantic tests in `canonical_*.rs` exercise transport and credential
//! behaviour. These guards assert the ownership invariants that cannot be
//! tested semantically without giving a provider a tool registry and seeing
//! whether it uses it.

fn canonical_provider_sources() -> Vec<(&'static str, &'static str)> {
    vec![
        ("minimax", include_str!("../src/canonical_minimax.rs")),
        ("anthropic", include_str!("../src/canonical_anthropic.rs")),
        (
            "openai_compatible",
            include_str!("../src/canonical_openai_compatible.rs"),
        ),
    ]
}

#[test]
fn canonical_providers_own_transport_and_credentials_only() {
    for (name, source) in canonical_provider_sources() {
        assert!(
            source.contains("reqwest::Client"),
            "{name}: a canonical provider must own its vendor HTTP transport"
        );
        assert!(
            source.contains("CredentialResolver"),
            "{name}: a canonical provider must obtain secrets through CredentialResolver"
        );
        assert!(
            source.contains("fn resolve_key"),
            "{name}: a canonical provider must resolve its key per turn"
        );
    }
}

#[test]
fn canonical_providers_do_not_store_or_execute_anything_else() {
    for (name, source) in canonical_provider_sources() {
        assert!(
            !source.contains("api_key: String"),
            "{name}: a canonical provider must never store a plaintext API key"
        );
        assert!(
            !source.contains("ToolCapability"),
            "{name}: a canonical provider must never import or implement tool execution"
        );
        assert!(
            !source.contains("ToolRegistry"),
            "{name}: a canonical provider must never use a tool registry"
        );
        assert!(
            !source.contains(".invoke("),
            "{name}: a canonical provider must never invoke a tool"
        );
        assert!(
            !source.contains("execute_tool"),
            "{name}: a canonical provider must never execute a tool"
        );
        assert!(
            !source.contains("ProcessExecutor") && !source.contains("ProcessRequest"),
            "{name}: a canonical provider must never own or use the process execution boundary"
        );
    }
}
