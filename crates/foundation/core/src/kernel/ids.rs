//! Canonical identifier primitives.
//!
//! Two families, deliberately different:
//!
//! - **Generated** ids ([`SessionId`], [`TaskId`], [`TraceId`], [`RequestId`]) wrap
//!   a UUID. They are minted at runtime and are meaningless to humans.
//! - **Stable** ids ([`PluginId`], [`CapabilityId`], [`ModelId`]) wrap a validated
//!   string. They appear in manifests, configuration, and logs, so they are part
//!   of the public contract and must not change between releases.
//!
//! Stable ids are validated on construction. An unvalidated `String` flowing into
//! a registry key is how a system ends up with `tool.shell`, `Tool.Shell`, and
//! `tool_shell` all naming the same thing.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::error::{CoreError, CoreResult};

/// Declares a UUID-backed generated identifier.
macro_rules! generated_id {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(Uuid);

        impl $name {
            /// Mint a fresh, random identifier.
            pub fn new() -> Self {
                Self(Uuid::new_v4())
            }

            /// Wrap an existing UUID (for rehydration from storage or a wire format).
            pub const fn from_uuid(uuid: Uuid) -> Self {
                Self(uuid)
            }

            /// The underlying UUID.
            pub const fn as_uuid(&self) -> Uuid {
                self.0
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl FromStr for $name {
            type Err = CoreError;

            fn from_str(s: &str) -> CoreResult<Self> {
                Uuid::parse_str(s)
                    .map(Self)
                    .map_err(|e| CoreError::invalid_id(stringify!($name), s, e.to_string()))
            }
        }

        impl From<Uuid> for $name {
            fn from(uuid: Uuid) -> Self {
                Self(uuid)
            }
        }
    };
}

generated_id! {
    /// Identifies one conversation: an ordered message history plus its state.
    ///
    /// A session outlives the individual requests made against it.
    SessionId
}

generated_id! {
    /// Identifies one unit of asynchronous work owned by the runtime.
    TaskId
}

generated_id! {
    /// Identifies one causally-related span of activity across subsystems.
    ///
    /// A trace crosses provider calls, capability dispatches, and governance
    /// decisions. It is the join key for anything that needs to answer "what
    /// happened during this turn".
    TraceId
}

generated_id! {
    /// Identifies one inbound request.
    ///
    /// One session has many requests; one request may fan out into many provider
    /// round-trips, all of which share the request's [`TraceId`].
    RequestId
}

generated_id! {
    /// Identifies one pending human approval.
    ///
    /// Minted when governance returns `RequireApproval` and stored with the
    /// frozen operation. It is the stable handle callers use to approve or
    /// reject the pending operation later.
    ApprovalId
}

/// Longest permitted stable identifier.
///
/// Generous enough for `provider.some-vendor.some-long-model-name`, short enough
/// that an id can never be mistaken for a payload.
const MAX_STABLE_ID_LEN: usize = 128;

/// Validates the shared grammar for stable identifiers.
///
/// Permitted: ASCII lowercase letters, digits, `.`, `-`, `_`. Must start with a
/// letter, must not be empty, must not exceed [`MAX_STABLE_ID_LEN`], and must not
/// contain an empty dot-separated segment.
fn validate_stable_id(kind: &'static str, raw: &str) -> CoreResult<()> {
    if raw.is_empty() {
        return Err(CoreError::invalid_id(kind, raw, "must not be empty"));
    }
    if raw.len() > MAX_STABLE_ID_LEN {
        return Err(CoreError::invalid_id(
            kind,
            raw,
            format!("must be at most {MAX_STABLE_ID_LEN} bytes"),
        ));
    }
    if !raw.starts_with(|c: char| c.is_ascii_lowercase()) {
        return Err(CoreError::invalid_id(
            kind,
            raw,
            "must start with an ASCII lowercase letter",
        ));
    }
    for c in raw.chars() {
        let ok = c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '.' | '-' | '_');
        if !ok {
            return Err(CoreError::invalid_id(
                kind,
                raw,
                format!("illegal character {c:?}; allowed: a-z 0-9 . - _"),
            ));
        }
    }
    if raw.split('.').any(str::is_empty) {
        return Err(CoreError::invalid_id(
            kind,
            raw,
            "must not contain an empty dot-separated segment",
        ));
    }
    Ok(())
}

/// Declares a validated, human-meaningful stable identifier.
macro_rules! stable_id {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            /// Validate and wrap a stable identifier.
            pub fn new(raw: impl Into<String>) -> CoreResult<Self> {
                let raw = raw.into();
                validate_stable_id(stringify!($name), &raw)?;
                Ok(Self(raw))
            }

            /// The identifier as a string slice.
            pub fn as_str(&self) -> &str {
                &self.0
            }

            /// Consume into the underlying `String`.
            pub fn into_inner(self) -> String {
                self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl FromStr for $name {
            type Err = CoreError;

            fn from_str(s: &str) -> CoreResult<Self> {
                Self::new(s)
            }
        }

        impl AsRef<str> for $name {
            fn as_ref(&self) -> &str {
                &self.0
            }
        }

        // Deserialization goes through the same validation as construction, so a
        // malformed manifest fails at parse time rather than at dispatch time.
        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                let raw = String::deserialize(deserializer)?;
                Self::new(raw).map_err(serde::de::Error::custom)
            }
        }
    };
}

stable_id! {
    /// Identifies a plugin across restarts and releases, e.g. `builtin.calculator`.
    PluginId
}

stable_id! {
    /// Identifies a capability across restarts and releases, e.g. `tool.shell`.
    ///
    /// By convention the first dot-separated segment names the capability kind
    /// (`tool`, `provider`, `memory`, `transport`, `observer`, `scheduler`,
    /// `extension`) and the remainder names the instance. The typed
    /// `CapabilityKind` that pairs with this id is owned by `apeireth-plugin`;
    /// core deliberately holds only the identifier, so that adding a kind does not
    /// require touching core.
    CapabilityId
}

impl CapabilityId {
    /// The leading dot-separated segment, by convention the capability kind.
    ///
    /// Returns the whole id when it has no dot.
    pub fn kind_segment(&self) -> &str {
        self.0.split('.').next().unwrap_or(&self.0)
    }
}

stable_id! {
    /// Identifies a model offered by a provider, e.g. `claude-opus-5`.
    ModelId
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_ids_are_unique_and_round_trip_through_strings() {
        let a = SessionId::new();
        let b = SessionId::new();
        assert_ne!(a, b, "fresh ids must not collide");

        let parsed: SessionId = a.to_string().parse().unwrap();
        assert_eq!(a, parsed);
    }

    #[test]
    fn generated_id_rejects_non_uuid() {
        let err = "not-a-uuid".parse::<TraceId>().unwrap_err();
        assert!(matches!(err, CoreError::InvalidId { .. }));
    }

    #[test]
    fn stable_id_accepts_conventional_forms() {
        for raw in [
            "tool.shell",
            "provider.anthropic",
            "memory.sqlite",
            "transport.mcp",
            "observer.tracing",
            "builtin.calculator",
            "some-vendor_v2.model-01",
        ] {
            assert!(CapabilityId::new(raw).is_ok(), "{raw} should be accepted");
        }
    }

    #[test]
    fn stable_id_rejects_malformed_forms() {
        for raw in [
            "",             // empty
            "Tool.Shell",   // uppercase
            "1tool.shell",  // leading digit
            "tool shell",   // space
            "tool..shell",  // empty segment
            ".tool",        // leading dot
            "tool.shell!",  // punctuation
            "tool.shell\n", // control character
        ] {
            assert!(
                CapabilityId::new(raw).is_err(),
                "{raw:?} should be rejected"
            );
        }
    }

    #[test]
    fn stable_id_rejects_overlong_input() {
        let raw = format!("tool.{}", "x".repeat(MAX_STABLE_ID_LEN));
        assert!(CapabilityId::new(raw).is_err());
    }

    #[test]
    fn capability_id_exposes_its_kind_segment() {
        assert_eq!(
            CapabilityId::new("tool.shell").unwrap().kind_segment(),
            "tool"
        );
        assert_eq!(
            CapabilityId::new("provider.anthropic")
                .unwrap()
                .kind_segment(),
            "provider"
        );
        assert_eq!(CapabilityId::new("bare").unwrap().kind_segment(), "bare");
    }

    #[test]
    fn stable_id_deserialization_enforces_the_same_grammar() {
        let ok: CapabilityId = serde_json::from_str(r#""tool.shell""#).unwrap();
        assert_eq!(ok.as_str(), "tool.shell");

        // A malformed manifest must fail at parse time, not at dispatch time.
        let err = serde_json::from_str::<CapabilityId>(r#""Tool.Shell""#);
        assert!(err.is_err());
    }
}
