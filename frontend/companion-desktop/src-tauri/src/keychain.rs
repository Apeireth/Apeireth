//! Provider API-key persistence via the OS credential store.
//!
//! Keys live only in the platform credential store — Windows Credential
//! Manager, macOS Keychain, Linux Secret Service — never in the app-data JSON
//! config or any other plaintext file. The service name is fixed so every
//! provider entry shares one keyring "application"; the account name carries
//! the provider identity so keys never collide across providers.
//!
//! # Windows: why not the `keyring` crate
//!
//! `keyring` v3.6 writes with `CRED_PERSIST_ENTERPRISE`, which on Microsoft-
//! account / non-domain machines makes the credential invisible to OTHER
//! processes (same-process read works, fresh-process read returns NoEntry).
//! That broke the core P0-1 promise "save key → relaunch app → key still
//! there" on the real 2026-09-12 machine. This module therefore speaks to
//! Credential Manager directly with `CRED_PERSIST_LOCAL_MACHINE`, which
//! survives process restarts and interoperates with `cmdkey /generic`
//! (target = our target name, username = the provider account).

/// Fixed credential-store service name shared by every provider entry.
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

/// Fetch a provider key from the OS credential store, if one is stored.
///
/// A missing entry (or an unreadable store) resolves to `None` so callers can
/// treat the store as an optional fallback rather than a hard dependency.
pub fn get_provider_key(provider: &str) -> Option<String> {
    let provider = provider.trim();
    if provider.is_empty() {
        return None;
    }
    platform::get(&account_for(provider)).ok().flatten()
}

/// Store a provider key in the OS credential store (value is trimmed first).
pub fn set_provider_key(provider: &str, key: &str) -> Result<(), String> {
    let provider = provider.trim();
    let key = key.trim();
    if provider.is_empty() {
        return Err("provider must not be empty".to_string());
    }
    if key.is_empty() {
        return Err("key must not be empty".to_string());
    }
    platform::set(&account_for(provider), key)
}

/// Remove a provider key from the OS credential store. Deleting a missing
/// entry is a successful no-op.
pub fn delete_provider_key(provider: &str) -> Result<(), String> {
    let provider = provider.trim();
    if provider.is_empty() {
        return Err("provider must not be empty".to_string());
    }
    platform::delete(&account_for(provider))
}

/// Whether a provider key is currently stored.
pub fn has_provider_key(provider: &str) -> bool {
    get_provider_key(provider).is_some()
}

// ---------------------------------------------------------------------------
// Platform backends
// ---------------------------------------------------------------------------

#[cfg(windows)]
mod platform {
    use windows_sys::Win32::Foundation::ERROR_NOT_FOUND;
    use windows_sys::Win32::Security::Credentials::{
        CredDeleteW, CredFree, CredReadW, CredWriteW, CREDENTIALW, CRED_PERSIST_LOCAL_MACHINE,
        CRED_TYPE_GENERIC,
    };

    /// Credential Manager target for a provider account.
    fn target(account: &str) -> Vec<u16> {
        let mut v: Vec<u16> = format!("{}:{}", super::KEYRING_SERVICE, account)
            .encode_utf16()
            .collect();
        v.push(0);
        v
    }

    fn wide(s: &str) -> Vec<u16> {
        let mut v: Vec<u16> = s.encode_utf16().collect();
        v.push(0);
        v
    }

    /// Credential blobs in Credential Manager are raw bytes; we store the key
    /// as UTF-16LE (the native Windows string charset, same as `cmdkey`) and
    /// accept UTF-8 on read for tolerance of anything else that wrote it.
    fn to_blob(key: &str) -> Vec<u8> {
        let units: Vec<u16> = key.encode_utf16().collect();
        let mut blob = Vec::with_capacity(units.len() * 2);
        for unit in units {
            blob.extend_from_slice(&unit.to_le_bytes());
        }
        blob
    }

    fn from_blob(blob: &[u8]) -> Option<String> {
        if blob.len() % 2 == 0 {
            let mut units = Vec::with_capacity(blob.len() / 2);
            for chunk in blob.chunks_exact(2) {
                units.push(u16::from_le_bytes([chunk[0], chunk[1]]));
            }
            if let Ok(s) = String::from_utf16(&units) {
                if !s.is_empty() {
                    return Some(s);
                }
            }
        }
        String::from_utf8(blob.to_vec()).ok().filter(|s| !s.is_empty())
    }

    pub fn set(account: &str, key: &str) -> Result<(), String> {
        let target = target(account);
        let username = wide(account);
        let mut blob = to_blob(key);

        let mut credential = CREDENTIALW {
            Flags: 0,
            Type: CRED_TYPE_GENERIC,
            TargetName: target.as_ptr() as *mut u16,
            Comment: std::ptr::null_mut(),
            LastWritten: windows_sys::Win32::Foundation::FILETIME {
                dwLowDateTime: 0,
                dwHighDateTime: 0,
            },
            CredentialBlobSize: blob.len() as u32,
            CredentialBlob: blob.as_mut_ptr(),
            Persist: CRED_PERSIST_LOCAL_MACHINE,
            AttributeCount: 0,
            Attributes: std::ptr::null_mut(),
            TargetAlias: std::ptr::null_mut(),
            UserName: username.as_ptr() as *mut u16,
        };

        // SAFETY: all pointer fields reference the buffers above, which stay
        // alive for the duration of the call.
        let ok = unsafe { CredWriteW(&mut credential, 0) };
        if ok == 0 {
            let code = std::io::Error::last_os_error();
            return Err(format!("CredWriteW failed: {code}"));
        }
        Ok(())
    }

    pub fn get(account: &str) -> Result<Option<String>, String> {
        let target = target(account);
        let mut credential: *mut CREDENTIALW = std::ptr::null_mut();
        // SAFETY: target is a NUL-terminated UTF-16 buffer alive for the call;
        // the returned pointer is freed via CredFree below.
        let ok = unsafe { CredReadW(target.as_ptr(), CRED_TYPE_GENERIC, 0, &mut credential) };
        if ok == 0 {
            let code = std::io::Error::last_os_error();
            if code.raw_os_error() == Some(ERROR_NOT_FOUND as i32) {
                return Ok(None);
            }
            return Err(format!("CredReadW failed: {code}"));
        }
        // SAFETY: credential is a valid CREDENTIALW pointer on success.
        let result = unsafe {
            let blob = std::slice::from_raw_parts(
                (*credential).CredentialBlob,
                (*credential).CredentialBlobSize as usize,
            );
            from_blob(blob)
        };
        // SAFETY: pointer came from CredReadW; CredFree releases it.
        unsafe { CredFree(credential as *mut _) };
        Ok(result)
    }

    pub fn delete(account: &str) -> Result<(), String> {
        let target = target(account);
        // SAFETY: target is a NUL-terminated UTF-16 buffer alive for the call.
        let ok = unsafe { CredDeleteW(target.as_ptr(), CRED_TYPE_GENERIC, 0) };
        if ok == 0 {
            let code = std::io::Error::last_os_error();
            if code.raw_os_error() == Some(ERROR_NOT_FOUND as i32) {
                return Ok(());
            }
            return Err(format!("CredDeleteW failed: {code}"));
        }
        Ok(())
    }

    /// Live probe (ignored by default): write a throwaway credential and read
    /// it back. Run in TWO separate invocations to prove cross-process
    /// visibility — the exact property the `keyring` v3 crate broke on
    /// Microsoft-account machines:
    ///   cargo test -p companion-desktop --lib keychain_cross_process -- --ignored --nocapture
    /// First run writes, second run reads; the second run MUST succeed.
    #[test]
    #[ignore]
    fn keychain_cross_process_probe() {
        const PROBE_ACCOUNT: &str = "provider:probe";
        match super::get_provider_key("probe") {
            Some(key) => {
                let tail: String = key
                    .chars()
                    .rev()
                    .take(4)
                    .collect::<Vec<_>>()
                    .into_iter()
                    .rev()
                    .collect();
                println!("PROBE READ OK (ends ****{tail})");
            }
            None => {
                println!("PROBE EMPTY — writing");
                super::set_provider_key("probe", "sk-probe-1234567890abcd").unwrap();
            }
        }
        let _ = PROBE_ACCOUNT;
    }
}

#[cfg(not(windows))]
mod platform {
    use keyring::Entry;

    fn entry(account: &str) -> Result<Entry, String> {
        Entry::new(super::KEYRING_SERVICE, account)
            .map_err(|e| format!("failed to open keychain entry: {e}"))
    }

    pub fn set(account: &str, key: &str) -> Result<(), String> {
        entry(account)?
            .set_password(key)
            .map_err(|e| format!("failed to store key in keychain: {e}"))
    }

    pub fn get(account: &str) -> Result<Option<String>, String> {
        match entry(account).and_then(|e| e.get_password()) {
            Ok(key) if !key.is_empty() => Ok(Some(key)),
            Ok(_) => Ok(None),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(format!("failed to read keychain: {e}")),
        }
    }

    pub fn delete(account: &str) -> Result<(), String> {
        match entry(account)?.delete_credential() {
            Ok(()) => Ok(()),
            Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(format!("failed to delete keychain entry: {e}")),
        }
    }
}
