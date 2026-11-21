//! Versioned keyring encoding for a credential and its binding.
//!
//! Legacy keyring values remain raw UTF-8 secrets and therefore unmanaged.
//! New bound values store the secret and endpoint declarations in one companion
//! keyring record, so a backend `get` cannot observe a value from one revision
//! and a binding from another. The public service contains only a non-secret
//! marker. Older binaries therefore fail authentication instead of sending an
//! envelope containing the real key as a bearer token.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;
use zeroize::Zeroizing;

use crate::SecretBuf;

const ENVELOPE_PREFIX: &[u8] = b"apeireth:bound-credential:v1:";
const REFERENCE_PREFIX: &[u8] = b"apeireth:bound-reference:";
const REFERENCE_V1: &[u8] = b"apeireth:bound-reference:v1";

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum CredentialRecordError {
    /// A recognized bound record is truncated, malformed, or has unknown fields.
    #[error("bound credential record is malformed")]
    Malformed,
    /// The in-memory record could not be encoded.
    #[error("bound credential record could not be encoded")]
    Encode,
}

/// Deterministic, non-secret companion service for one logical credential.
///
/// The full SHA-256 digest keeps the result below every backend's service-name
/// limit without exposing the logical credential name in a second keyring row.
pub fn bound_record_service(service: &str) -> String {
    let digest = Sha256::digest(service.as_bytes());
    format!("apeireth.bound.v1.{}", hex::encode(digest))
}

/// Non-secret marker stored under the legacy service name for a bound record.
pub fn bound_credential_reference() -> SecretBuf {
    SecretBuf::new(REFERENCE_V1.to_vec())
}

/// Classify the legacy service record without accepting corrupt future markers.
pub fn is_bound_credential_reference(stored: &SecretBuf) -> Result<bool, CredentialRecordError> {
    if stored.expose() == REFERENCE_V1 {
        return Ok(true);
    }
    if stored.expose().starts_with(REFERENCE_PREFIX) {
        return Err(CredentialRecordError::Malformed);
    }
    Ok(false)
}

/// A decoded keyring record. `None` endpoints identify a legacy raw value.
pub struct CredentialRecord {
    secret: SecretBuf,
    endpoints: Option<Vec<String>>,
}

impl CredentialRecord {
    /// Consume the record into the secret and its binding declaration.
    pub fn into_parts(self) -> (SecretBuf, Option<Vec<String>>) {
        (self.secret, self.endpoints)
    }

    /// Borrow only the non-secret endpoint metadata.
    pub fn endpoints(&self) -> Option<&[String]> {
        self.endpoints.as_deref()
    }
}

#[derive(Serialize)]
#[serde(deny_unknown_fields)]
struct StoredCredentialRef<'a> {
    secret: &'a str,
    endpoints: &'a [String],
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredCredential {
    secret: String,
    endpoints: Vec<String>,
}

/// Encode a credential for one keyring `set` operation.
///
/// `None` preserves the legacy raw format. `Some`, including an empty slice,
/// uses the versioned envelope and is therefore an explicit binding declaration.
pub fn encode_credential_record(
    secret: &str,
    endpoints: Option<&[String]>,
) -> Result<SecretBuf, CredentialRecordError> {
    let Some(endpoints) = endpoints else {
        return Ok(SecretBuf::from_str(secret));
    };
    let serialized = Zeroizing::new(
        serde_json::to_string(&StoredCredentialRef { secret, endpoints })
            .map_err(|_| CredentialRecordError::Encode)?,
    );
    let mut bytes = Vec::with_capacity(ENVELOPE_PREFIX.len() + serialized.len());
    bytes.extend_from_slice(ENVELOPE_PREFIX);
    bytes.extend_from_slice(serialized.as_bytes());
    Ok(SecretBuf::new(bytes))
}

/// Decode one keyring `get` result.
///
/// A value without the prefix is a backward-compatible unmanaged secret. Once
/// the prefix is present, malformed data fails closed instead of silently
/// downgrading the credential to the legacy unbound state.
pub fn decode_credential_record(
    stored: &SecretBuf,
) -> Result<CredentialRecord, CredentialRecordError> {
    if stored.expose().starts_with(REFERENCE_PREFIX) {
        return Err(CredentialRecordError::Malformed);
    }
    let Some(encoded) = stored.expose().strip_prefix(ENVELOPE_PREFIX) else {
        return Ok(CredentialRecord {
            secret: SecretBuf::new(stored.expose().to_vec()),
            endpoints: None,
        });
    };
    let encoded = std::str::from_utf8(encoded).map_err(|_| CredentialRecordError::Malformed)?;
    let StoredCredential { secret, endpoints } =
        serde_json::from_str(encoded).map_err(|_| CredentialRecordError::Malformed)?;
    let secret = Zeroizing::new(secret);
    Ok(CredentialRecord {
        secret: SecretBuf::from_str(&secret),
        endpoints: Some(endpoints),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_values_remain_unmanaged() {
        let stored = SecretBuf::from_str("synthetic-legacy-value");
        let (secret, endpoints) = decode_credential_record(&stored).unwrap().into_parts();
        assert_eq!(secret.expose(), b"synthetic-legacy-value");
        assert_eq!(endpoints, None);
    }

    #[test]
    fn bound_values_round_trip_without_debug_disclosure() {
        let stored = encode_credential_record(
            "synthetic-bound-value",
            Some(&["https://model.example/v1".to_string()]),
        )
        .unwrap();
        assert!(!format!("{stored:?}").contains("synthetic-bound-value"));
        let (secret, endpoints) = decode_credential_record(&stored).unwrap().into_parts();
        assert_eq!(secret.expose(), b"synthetic-bound-value");
        assert_eq!(
            endpoints,
            Some(vec!["https://model.example/v1".to_string()])
        );
    }

    #[test]
    fn malformed_versioned_values_do_not_downgrade_to_legacy() {
        let mut bytes = ENVELOPE_PREFIX.to_vec();
        bytes.extend_from_slice(br#"{"secret":"synthetic"}"#);
        let Err(error) = decode_credential_record(&SecretBuf::new(bytes)) else {
            panic!("malformed record unexpectedly decoded");
        };
        assert_eq!(error, CredentialRecordError::Malformed);
        assert!(!format!("{error}").contains("synthetic"));
    }

    #[test]
    fn reference_is_non_secret_and_has_a_bounded_companion_name() {
        let service = "provider.synthetic.api_key";
        let marker = bound_credential_reference();
        assert!(is_bound_credential_reference(&marker).unwrap());
        assert!(!marker.expose().windows(2).any(|window| window == b"sk"));
        let companion = bound_record_service(service);
        assert!(companion.len() <= crate::keyring::MAX_SERVICE_NAME_LEN);
        assert!(!companion.contains(service));
    }

    #[test]
    fn malformed_reference_does_not_become_a_legacy_secret() {
        let stored = SecretBuf::from_str("apeireth:bound-reference:v2");
        assert_eq!(
            is_bound_credential_reference(&stored),
            Err(CredentialRecordError::Malformed)
        );
        assert!(decode_credential_record(&stored).is_err());
    }
}
