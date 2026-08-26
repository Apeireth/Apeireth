//! How a plugin obtains a secret without anyone hardcoding one.
//!
//! Credentials are *resolved at start-up through an injected resolver*, never
//! read from a fixed path and never stored as a plain `String` field on a
//! long-lived struct. A runtime that owns `api_key: String` has already made the
//! secret visible to every `Debug` print, every panic message, and every
//! serializer that touches the struct.
//!
//! This module deliberately holds only the *contract* plus two trivial
//! implementations. The real backends — OS keyring, encrypted file, zeroizing
//! buffers — already exist in `apeireth-credentials`, and wiring that crate in
//! behind [`CredentialResolver`] is tracked as a migration item rather than
//! duplicated here.

use std::collections::BTreeMap;
use std::fmt;

/// A resolved secret.
///
/// `Debug` and `Display` are redacted, so a secret cannot reach a log by
/// accident. Reading the real value requires calling [`Secret::expose`], which
/// is greppable.
#[derive(Clone, PartialEq, Eq)]
pub struct Secret(String);

impl Secret {
    /// Wrap a secret value.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Read the underlying value.
    ///
    /// Every call site is a place a secret could leak; keep them few and short.
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Secret(<redacted>)")
    }
}

impl fmt::Display for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("<redacted>")
    }
}

/// Supplies secrets by name.
///
/// Names are logical (`provider.anthropic.api_key`), not locations. A plugin
/// asking for a *name* can be served from an environment variable in CI, the OS
/// keyring on a workstation, and a secrets manager in production, without the
/// plugin changing.
pub trait CredentialResolver: Send + Sync {
    /// Resolve `name`, or `None` if this resolver has no value for it.
    fn resolve(&self, name: &str) -> Option<Secret>;
}

/// A resolver that has nothing. Useful for tests and for booting without keys.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoCredentials;

impl CredentialResolver for NoCredentials {
    fn resolve(&self, _name: &str) -> Option<Secret> {
        None
    }
}

/// A resolver backed by an explicit map, for tests and for programmatic wiring.
#[derive(Debug, Clone, Default)]
pub struct StaticCredentials(BTreeMap<String, String>);

impl StaticCredentials {
    /// An empty set.
    pub fn new() -> Self {
        Self::default()
    }

    /// Builder-style insert.
    #[must_use]
    pub fn with(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.0.insert(name.into(), value.into());
        self
    }
}

impl CredentialResolver for StaticCredentials {
    fn resolve(&self, name: &str) -> Option<Secret> {
        self.0.get(name).map(Secret::new)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_secret_does_not_print_itself() {
        let s = Secret::new("sk-super-secret-value");
        assert_eq!(format!("{s:?}"), "Secret(<redacted>)");
        assert_eq!(format!("{s}"), "<redacted>");
        assert!(
            !format!("{s:?} {s}").contains("sk-super"),
            "the value must not survive formatting"
        );
        assert_eq!(s.expose(), "sk-super-secret-value");
    }

    #[test]
    fn static_resolver_serves_only_what_it_was_given() {
        let r = StaticCredentials::new().with("provider.fake.api_key", "k");
        assert_eq!(
            r.resolve("provider.fake.api_key")
                .map(|s| s.expose().to_string()),
            Some("k".to_string())
        );
        assert!(r.resolve("provider.other.api_key").is_none());
    }

    #[test]
    fn the_empty_resolver_resolves_nothing() {
        assert!(NoCredentials.resolve("anything").is_none());
    }
}
