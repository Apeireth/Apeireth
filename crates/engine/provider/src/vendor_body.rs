//! Vendor-private request fields: the **single source of truth** for the
//! environment mapping that feeds the provider's opt-in extra-body passthrough.
//!
//! ⭐ **Why this exists.** Some endpoints in the OpenAI-compatible protocol
//! family add request fields the canonical protocol does not model. DeepSeek's
//! OpenAI-compatible endpoint is one of them: its reasoning mode refuses a
//! request whose history contains an assistant turn that carried tool calls
//! unless the caller replays `reasoning_content` for that turn — and the only
//! other door is the vendor-private switch `{"thinking": {"type": "enabled" |
//! "disabled"}}`. The provider already owns the passthrough itself
//! ([`crate::openai_chat::merge_extra_body`]); this module owns the *naming,
//! parsing and validation* of the environment knobs that feed it.
//!
//! ⭐ **The switch only applies to endpoints that support the vendor-private
//! field.** Sending `{"thinking": ...}` to an endpoint that does not know it
//! (a standard OpenAI-compatible endpoint, for instance) makes the vendor
//! answer `400 Bad Request`. ⇒ every knob here is **opt-in** and the default
//! is "send nothing": with none of them set, the request body stays
//! byte-identical to what it was before this module existed.
//!
//! ⭐ **Fail fast.** An unrecognised value is an error **at construction
//! time** — never a silently ignored switch, never "treat it as disabled".
//! A typo in a deployment variable must surface where the variable is read,
//! not at the first real request (which would look like a vendor outage).

use crate::openai_chat::RESERVED_EXTRA_BODY_KEYS;

/// Environment variable: the vendor's reasoning-mode switch.
///
/// Accepted values (case-insensitive, surrounding blanks tolerated):
/// `enabled` / `on` / `true` / `1` ⇒ reasoning mode on;
/// `disabled` / `off` / `false` / `0` ⇒ reasoning mode off.
/// Any other value is refused at construction time.
///
/// ⭐ **This is the only vendor knob the production factory owns.** The
/// environment budget in the architecture contract counts one name per
/// capability switch, and the contract forbids "adding one while at hand"
/// (only what a capability genuinely needs may take a slot). The reasoning
/// mode is the one the deployment needs; further vendor-private fields can
/// still be expressed through [`vendor_body_from_parts`] by a caller that
/// owns its own configuration surface (the long-range memory evaluation does
/// exactly that).
pub const THINKING_ENV: &str = "APEIRETH_OPENAI_THINKING";

/// Why a vendor-private request body could not be built.
///
/// Every variant is a **construction-time** failure: the caller learns about
/// a misconfigured switch where the switch is read, not at the first real
/// request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VendorBodyError {
    /// The reasoning-mode switch carries a value that is not in the accepted
    /// set.
    InvalidThinking {
        /// The normalised (trimmed, lower-cased) value that was refused.
        value: String,
    },
    /// The extra-body payload does not carry valid JSON.
    InvalidExtraBodyJson {
        /// The parser's complaint.
        detail: String,
    },
    /// The extra-body payload carries valid JSON that is not an object.
    ExtraBodyNotObject,
    /// The extra body tries to override a key that carries protocol meaning.
    ReservedKey {
        /// The offending key.
        key: String,
    },
}

impl std::fmt::Display for VendorBodyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidThinking { value } => write!(
                f,
                "环境变量 {THINKING_ENV} 取值无法识别：{value}（可用：enabled/disabled）"
            ),
            Self::InvalidExtraBodyJson { detail } => {
                write!(f, "厂商额外请求字段不是合法 JSON：{detail}")
            }
            Self::ExtraBodyNotObject => {
                write!(f, "厂商额外请求字段必须是 JSON 对象")
            }
            Self::ReservedKey { key } => write!(
                f,
                "厂商额外请求字段不得覆盖协议保留键 '{key}'（保留键：{RESERVED_EXTRA_BODY_KEYS:?}）"
            ),
        }
    }
}

impl std::error::Error for VendorBodyError {}

/// Parse the reasoning-mode switch.
///
/// ⭐ Returns `Ok(true)` for `enabled` / `on` / `true` / `1` and `Ok(false)`
/// for `disabled` / `off` / `false` / `0` (case-insensitive, surrounding
/// blanks tolerated). Every other value is an error — it is **not** silently
/// treated as "off".
pub fn parse_thinking(value: &str) -> Result<bool, VendorBodyError> {
    match value.trim().to_ascii_lowercase().as_str() {
        "enabled" | "on" | "true" | "1" => Ok(true),
        "disabled" | "off" | "false" | "0" => Ok(false),
        other => Err(VendorBodyError::InvalidThinking {
            value: other.to_string(),
        }),
    }
}

/// Build the vendor-private request fields from already-parsed parts.
///
/// Three sources, in this order (later wins):
/// 1. the reasoning-mode switch ([`parse_thinking`]);
/// 2. the reasoning effort (a free-form string);
/// 3. the raw JSON object of the extra body.
///
/// ⭐ The result is always a JSON **object**; with no source at all it is the
/// empty object `{}` (which changes nothing on the wire). Keys that carry
/// protocol meaning are refused here, before any request is sent.
///
/// ⭐ The parts are values, not environment names: a caller that owns its own
/// configuration surface (a stored configuration file, a tool-specific
/// switch) can feed them here without spending an environment name in the
/// architecture contract's budget.
pub fn vendor_body_from_parts(
    thinking: Option<bool>,
    reasoning_effort: Option<&str>,
    extra_body: Option<&str>,
) -> Result<serde_json::Value, VendorBodyError> {
    let mut fields = serde_json::Map::new();
    if let Some(thinking) = thinking {
        let mode = if thinking { "enabled" } else { "disabled" };
        fields.insert("thinking".to_string(), serde_json::json!({ "type": mode }));
    }
    if let Some(effort) = reasoning_effort {
        fields.insert("reasoning_effort".to_string(), serde_json::json!(effort));
    }
    if let Some(raw) = extra_body.map(str::trim).filter(|value| !value.is_empty()) {
        let parsed: serde_json::Value =
            serde_json::from_str(raw).map_err(|error| VendorBodyError::InvalidExtraBodyJson {
                detail: error.to_string(),
            })?;
        let Some(extra_fields) = parsed.as_object() else {
            return Err(VendorBodyError::ExtraBodyNotObject);
        };
        for (key, value) in extra_fields {
            fields.insert(key.clone(), value.clone());
        }
    }
    for key in fields.keys() {
        if RESERVED_EXTRA_BODY_KEYS.contains(&key.as_str()) {
            return Err(VendorBodyError::ReservedKey { key: key.clone() });
        }
    }
    Ok(serde_json::Value::Object(fields))
}

/// Build the vendor-private request fields from the process environment.
///
/// ⭐ Returns `Ok(None)` when the reasoning-mode switch is **not set**: that
/// is the "send nothing" default, and the caller must then leave the
/// provider's extra body unset (the request body then stays byte-identical to
/// the one before this module existed).
///
/// A set-but-unusable value is an error: the switch is read once, at
/// construction time.
pub fn vendor_body_from_env() -> Result<Option<serde_json::Value>, VendorBodyError> {
    vendor_body_from_env_with(|name| std::env::var(name).ok())
}

/// Build the vendor-private request fields from an injected environment
/// lookup.
///
/// ⭐ The lookup indirection exists so the mapping can be exercised without
/// touching the process-wide environment (which is global state that tests
/// must not race on). The production path uses [`vendor_body_from_env`].
pub fn vendor_body_from_env_with<F>(lookup: F) -> Result<Option<serde_json::Value>, VendorBodyError>
where
    F: Fn(&str) -> Option<String>,
{
    let thinking = lookup(THINKING_ENV).filter(|value| !value.trim().is_empty());
    let Some(thinking) = thinking else {
        return Ok(None);
    };
    let thinking = parse_thinking(&thinking)?;
    Ok(Some(vendor_body_from_parts(Some(thinking), None, None)?))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env_of(pairs: &[(&str, &str)]) -> Box<dyn Fn(&str) -> Option<String>> {
        let pairs: Vec<(String, String)> = pairs
            .iter()
            .map(|(key, value)| ((*key).to_string(), (*value).to_string()))
            .collect();
        Box::new(move |name: &str| {
            pairs
                .iter()
                .find(|(key, _)| key == name)
                .map(|(_, value)| value.clone())
        })
    }

    #[test]
    fn thinking_switch_accepts_every_documented_spelling() {
        for value in ["enabled", "on", "true", "1", "ENABLED", " On "] {
            assert!(parse_thinking(value).expect("accepted"), "{value} ⇒ on");
        }
        for value in ["disabled", "off", "false", "0", "DISABLED", " Off "] {
            assert!(!parse_thinking(value).expect("accepted"), "{value} ⇒ off");
        }
    }

    #[test]
    fn thinking_switch_refuses_an_unknown_value() {
        let error = parse_thinking("maybe").expect_err("must refuse");
        assert_eq!(
            error,
            VendorBodyError::InvalidThinking {
                value: "maybe".into()
            }
        );
        assert!(error.to_string().contains(THINKING_ENV), "{error}");
        assert!(error.to_string().contains("maybe"), "{error}");
    }

    #[test]
    fn no_switch_set_yields_no_body_at_all() {
        let body = vendor_body_from_env_with(env_of(&[])).expect("no error");
        assert!(body.is_none(), "no switch ⇒ nothing to send");
    }

    #[test]
    fn a_blank_switch_counts_as_unset() {
        let body = vendor_body_from_env_with(env_of(&[(THINKING_ENV, "  ")])).expect("no error");
        assert!(body.is_none(), "blank switch ⇒ nothing to send");
    }

    #[test]
    fn the_reasoning_switch_alone_produces_the_vendor_field() {
        let body = vendor_body_from_env_with(env_of(&[(THINKING_ENV, "disabled")]))
            .expect("no error")
            .expect("a body");
        assert_eq!(body, serde_json::json!({"thinking": {"type": "disabled"}}));
    }

    #[test]
    fn the_parts_merge_into_one_object() {
        let body =
            vendor_body_from_parts(Some(true), Some("low"), Some(r#"{"vendor_flag": true}"#))
                .expect("no error");
        assert_eq!(
            body,
            serde_json::json!({
                "thinking": {"type": "enabled"},
                "reasoning_effort": "low",
                "vendor_flag": true,
            })
        );
    }

    #[test]
    fn a_malformed_extra_body_is_refused() {
        let error = vendor_body_from_parts(None, None, Some("not json")).expect_err("must refuse");
        assert!(matches!(
            error,
            VendorBodyError::InvalidExtraBodyJson { .. }
        ));
        assert!(error.to_string().contains("JSON"), "{error}");
    }

    #[test]
    fn a_non_object_extra_body_is_refused() {
        let error = vendor_body_from_parts(None, None, Some("[1, 2]")).expect_err("must refuse");
        assert_eq!(error, VendorBodyError::ExtraBodyNotObject);
        assert!(error.to_string().contains("对象"), "{error}");
    }

    #[test]
    fn a_reserved_protocol_key_is_refused() {
        let error = vendor_body_from_parts(None, None, Some(r#"{"messages": []}"#))
            .expect_err("must refuse");
        assert_eq!(
            error,
            VendorBodyError::ReservedKey {
                key: "messages".into()
            }
        );
        assert!(error.to_string().contains("messages"), "{error}");
    }
}
