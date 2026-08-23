//! The provider capability: something that can serve a completion.
//!
//! A provider is a capability like any other, so `provider.anthropic` lives in
//! the same registry as `tool.shell`. That is what keeps one source of truth for
//! "what can this runtime do".
//!
//! The provider owns exactly what the protocol layer must not: credentials, the
//! HTTP client and its lifetime, and per-vendor transport behaviour. It does
//! *not* own routing, fallback, or health policy — those are one level up, in the
//! runtime's router, because they are decisions *between* providers.

use apeireth_core::kernel::CapabilityId;
use apeireth_protocol::canonical::{ModelDescriptor, NormalizedRequest, NormalizedResponse};
use async_trait::async_trait;
use thiserror::Error;

/// A failure serving a completion.
///
/// The retryable/permanent split is the whole point of this type: it is what
/// lets a router decide between falling back to another provider and failing
/// fast. Classification is ported from the mature `MultiLlmRouter` in
/// `apeireth-api`, whose fallback loop this preserves.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum ProviderError {
    /// The provider is rate limiting. Transient.
    #[error("{provider}: rate limited, retry after {retry_after_ms}ms")]
    RateLimited {
        /// Which provider.
        provider: String,
        /// Suggested wait.
        retry_after_ms: u64,
    },

    /// The request timed out. Transient.
    #[error("{provider}: timed out after {timeout_ms}ms")]
    Timeout {
        /// Which provider.
        provider: String,
        /// The budget that elapsed.
        timeout_ms: u64,
    },

    /// A transport failure. Transient.
    #[error("{provider}: network error: {detail}")]
    Network {
        /// Which provider.
        provider: String,
        /// What happened.
        detail: String,
    },

    /// Credentials were missing or rejected. Permanent: retrying with the same
    /// key against another provider will not help, and silently falling back
    /// would mask a misconfiguration.
    #[error("{provider}: authentication failed: {detail}")]
    AuthFailed {
        /// Which provider.
        provider: String,
        /// What happened.
        detail: String,
    },

    /// The provider answered, but not in a shape the adapter could read.
    /// Permanent.
    #[error("{provider}: unusable response: {detail}")]
    BadResponse {
        /// Which provider.
        provider: String,
        /// What was wrong.
        detail: String,
    },

    /// The provider declined the request on policy grounds. Permanent.
    #[error("{provider}: request refused: {detail}")]
    Refused {
        /// Which provider.
        provider: String,
        /// The stated reason.
        detail: String,
    },
}

impl ProviderError {
    /// Whether trying a different provider could plausibly succeed.
    pub const fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::RateLimited { .. } | Self::Timeout { .. } | Self::Network { .. }
        )
    }

    /// Which provider produced this error.
    pub fn provider(&self) -> &str {
        match self {
            Self::RateLimited { provider, .. }
            | Self::Timeout { provider, .. }
            | Self::Network { provider, .. }
            | Self::AuthFailed { provider, .. }
            | Self::BadResponse { provider, .. }
            | Self::Refused { provider, .. } => provider,
        }
    }
}

/// Something that can serve a completion.
#[async_trait]
pub trait ProviderCapability: Send + Sync {
    /// Stable identity, e.g. `provider.anthropic`. Must match the manifest.
    fn id(&self) -> &CapabilityId;

    /// Models this provider can serve.
    fn models(&self) -> Vec<ModelDescriptor>;

    /// Whether this provider can serve `model`.
    ///
    /// Defaults to a lookup over [`ProviderCapability::models`]. Override when a
    /// provider accepts a family of names it cannot enumerate.
    fn supports_model(&self, model: &str) -> bool {
        self.models().iter().any(|m| m.id.as_str() == model)
    }

    /// Serve one completion.
    ///
    /// Takes a canonical [`NormalizedRequest`] and returns a canonical
    /// [`NormalizedResponse`]; translating to and from the vendor's wire format
    /// is the protocol layer's job, which this implementation calls into.
    async fn complete(
        &self,
        request: &NormalizedRequest,
    ) -> Result<NormalizedResponse, ProviderError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transient_failures_are_retryable_and_permanent_ones_are_not() {
        let transient = [
            ProviderError::RateLimited {
                provider: "p".into(),
                retry_after_ms: 100,
            },
            ProviderError::Timeout {
                provider: "p".into(),
                timeout_ms: 5000,
            },
            ProviderError::Network {
                provider: "p".into(),
                detail: "connection reset".into(),
            },
        ];
        for e in &transient {
            assert!(e.is_retryable(), "{e} should be retryable");
        }

        let permanent = [
            ProviderError::AuthFailed {
                provider: "p".into(),
                detail: "bad key".into(),
            },
            ProviderError::BadResponse {
                provider: "p".into(),
                detail: "missing choices".into(),
            },
            ProviderError::Refused {
                provider: "p".into(),
                detail: "policy".into(),
            },
        ];
        for e in &permanent {
            assert!(!e.is_retryable(), "{e} should not be retryable");
        }
    }

    #[test]
    fn every_error_names_its_provider() {
        let e = ProviderError::Timeout {
            provider: "provider.fake".into(),
            timeout_ms: 1,
        };
        assert_eq!(e.provider(), "provider.fake");
        assert!(e.to_string().contains("provider.fake"), "{e}");
    }
}
