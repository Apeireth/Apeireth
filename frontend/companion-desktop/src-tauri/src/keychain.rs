//! Provider API-key persistence via the OS keychain.
//!
//! Keys live only in the platform credential store — Windows Credential
//! Manager, macOS Keychain, Linux Secret Service — never in the app-data JSON
//! config or any other plaintext file. The service name is fixed so every
//! provider entry shares one keyring "application"; the account name carries
//! the provider identity so keys never collide across providers.

use keyring::Entry;

/// Fixed keyring service name shared by every provider entry.
pub const KEYRING_SERVICE: &str = "apeireth.companion";

/// Canonical provider identifiers used for the sidecar's automatic key
/// injection at spawn. OpenAI-compatible presets (openai/deepseek/ollama/
/// custom) all share [`PROVIDER_OPENAI`] because the sidecar reads them from
/// the single `OPENAI_API_KEY` variable.
pub const PROVIDER_OPENAI: &str = "openai";
pub const PROVIDER_MINIMAX: &str = "minimax";
pub const PROVIDER_ANTHROPIC: &str = "anthropic";

/// Keychain account name for a provider.
pub fn account_for(provider: &str) -> String {
    format!("provider:{}", provider.trim())
}

fn entry_for(provider: &str) -> Result<Entry, String> {
    let provider = provider.trim();
    if provider.is_empty() {
        return Err("provider must not be empty".to_string());
    }
    Entry::new(KEYRING_SERVICE, &account_for(provider))
        .map_err(|e| format!("failed to open keychain entry: {e}"))
}

/// Fetch a provider key from the OS keychain, if one is stored.
///
/// A missing entry (or an unreadable store) resolves to `None` so callers can
/// treat the keychain as an optional fallback rather than a hard dependency.
pub fn get_provider_key(provider: &str) -> Option<String> {
    let entry = entry_for(provider).ok()?;
    match entry.get_password() {
        Ok(key) if !key.is_empty() => Some(key),
        Ok(_) => None,
        Err(keyring::Error::NoEntry) => None,
        Err(_) => None,
    }
}

/// Store a provider key in the OS keychain (value is trimmed first).
pub fn set_provider_key(provider: &str, key: &str) -> Result<(), String> {
    let key = key.trim();
    if key.is_empty() {
        return Err("key must not be empty".to_string());
    }
    let entry = entry_for(provider)?;
    entry
        .set_password(key)
        .map_err(|e| format!("failed to store key in keychain: {e}"))
}

/// Remove a provider key from the OS keychain. Deleting a missing entry is a
/// successful no-op.
pub fn delete_provider_key(provider: &str) -> Result<(), String> {
    let entry = entry_for(provider)?;
    match entry.delete_credential() {
        Ok(()) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(format!("failed to delete key from keychain: {e}")),
    }
}

/// Whether a provider key is currently stored.
pub fn has_provider_key(provider: &str) -> bool {
    get_provider_key(provider).is_some()
}
