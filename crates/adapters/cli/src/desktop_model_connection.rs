//! Managed desktop startup profile; secrets stay in the existing OS keyring.
//! Writes configure the next start, never mutate an in-flight runtime.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use apeireth_core::hash_chain::{hash_linked, Chunk};
use apeireth_core::storage_atomic::{self, OWNER_ONLY_MODE};
use apeireth_credentials::keyring::{CountingAudit, KeyringBackend, KeyringError, PlatformKeyring};
use apeireth_credentials::SecretBuf;
use apeireth_gateway::model_connection::{
    CredentialState, ModelConnectionError as Error, ModelConnectionPatch as Patch,
    ModelConnectionService, ModelConnectionView as View,
};
use serde::{Deserialize, Serialize};

const FILE: &str = "desktop-model-connection.json";
const CREDENTIAL: &str = apeireth_provider::credentials::OPENAI_COMPATIBLE_API_KEY;
const DEFAULT_URL: &str = "https://api.deepseek.com/v1";
const DEFAULT_MODEL: &str = "deepseek-flash";

pub(crate) fn managed() -> bool {
    std::env::var("APEIRETH_DESKTOP_MANAGED").as_deref() == Ok("1")
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Profile {
    schema_version: u32,
    base_url: String,
    model: String,
    credential_service: Option<String>,
    credential_endpoint: Option<String>,
}

impl Default for Profile {
    fn default() -> Self {
        Self {
            schema_version: 1,
            base_url: DEFAULT_URL.into(),
            model: DEFAULT_MODEL.into(),
            credential_service: None,
            credential_endpoint: None,
        }
    }
}

impl Profile {
    fn valid(&self) -> bool {
        self.schema_version == 1
            && Patch {
                base_url: self.base_url.clone(),
                model: self.model.clone(),
                api_key: None,
            }
            .valid()
            && match (&self.credential_service, &self.credential_endpoint) {
                (None, None) => true,
                (Some(service), Some(endpoint)) => {
                    service
                        .strip_prefix("desktop-model-")
                        .and_then(|id| uuid::Uuid::parse_str(id).ok())
                        .is_some_and(|id| service == &format!("desktop-model-{id}"))
                        && endpoint == &self.base_url
                }
                _ => false,
            }
    }
}

struct ManagedConnection {
    root: PathBuf,
    backend: Arc<dyn KeyringBackend>,
    boot: Profile,
    writes: Mutex<()>,
    persist: fn(&Path, &[u8], u32) -> std::io::Result<()>,
}

fn namespace(root: &Path) -> Result<String, Error> {
    let canonical = fs_err::canonicalize(root).map_err(|_| Error::Storage)?;
    let mut identity = canonical.to_str().ok_or(Error::Storage)?.to_string();
    if cfg!(windows) {
        identity.make_ascii_lowercase();
    }
    let digest = hash_linked(
        "desktop-model-keyring-v1",
        &[Chunk::LenPrefixed(identity.as_bytes())],
    );
    Ok(format!("apeireth.desktop.{}", &digest[..32]))
}

fn scope(root: &Path) -> Result<(), Error> {
    for file in ["users.json", "client-tokens.json"] {
        match fs_err::symlink_metadata(root.join(file)) {
            Ok(_) => return Err(Error::Scope),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => return Err(Error::Scope),
        }
    }
    Ok(())
}

fn load(root: &Path) -> Result<Profile, Error> {
    let path = root.join(FILE);
    let metadata = match fs_err::symlink_metadata(&path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Profile::default()),
        Err(_) => return Err(Error::Storage),
    };
    if !metadata.is_file() || metadata.len() > 8192 {
        return Err(Error::Storage);
    }
    let bytes = fs_err::read(path).map_err(|_| Error::Storage)?;
    let profile: Profile = serde_json::from_slice(&bytes).map_err(|_| Error::Storage)?;
    if !profile.valid() {
        return Err(Error::Storage);
    }
    Ok(profile)
}

impl ManagedConnection {
    fn new(root: PathBuf, backend: Arc<dyn KeyringBackend>) -> Result<Self, Error> {
        scope(&root)?;
        let boot = load(&root)?;
        Ok(Self {
            root,
            backend,
            boot,
            writes: Mutex::new(()),
            persist: storage_atomic::write_atomic_durable,
        })
    }

    fn view(&self, profile: &Profile) -> View {
        let credential_state = match &profile.credential_service {
            None => CredentialState::Missing,
            Some(service) => match self.backend.get(service) {
                Ok(secret)
                    if std::str::from_utf8(secret.expose()).is_ok_and(|value| {
                        !value.trim().is_empty() && !value.chars().any(char::is_control)
                    }) =>
                {
                    CredentialState::Available
                }
                Ok(_) | Err(KeyringError::UnknownService { .. }) => CredentialState::Missing,
                Err(_) => CredentialState::Unavailable,
            },
        };
        View {
            schema_version: 1,
            provider: "openai-compatible",
            base_url: profile.base_url.clone(),
            model: profile.model.clone(),
            credential_state,
            storage: "windows-credential-manager",
            endpoint_bound: profile.credential_service.is_some()
                && profile.credential_endpoint.as_deref() == Some(profile.base_url.as_str()),
            restart_required: profile != &self.boot,
        }
    }

    fn restore(&self) {
        let hot = crate::keyring_bootstrap::hot_credential_store();
        hot.remove(CREDENTIAL);
        if let Some(service) = &self.boot.credential_service {
            if let Ok(secret) = self.backend.get(service) {
                if let Ok(value) = std::str::from_utf8(secret.expose()) {
                    if !value.trim().is_empty() && !value.chars().any(char::is_control) {
                        hot.set_with_endpoints(
                            CREDENTIAL,
                            value.to_string(),
                            [self.boot.base_url.clone()],
                        );
                    }
                }
            }
        }
    }

    fn save_locked(&self, patch: Patch) -> Result<View, Error> {
        scope(&self.root)?;
        if !patch.valid() {
            return Err(Error::Invalid);
        }
        let previous = load(&self.root)?;
        let base_url = patch.base_url.trim_end_matches('/').to_string();
        if patch.api_key.is_none()
            && previous.credential_service.is_some()
            && base_url != previous.base_url
        {
            return Err(Error::EndpointChangeNeedsKey);
        }
        let mut profile = Profile {
            schema_version: 1,
            base_url,
            model: patch.model,
            credential_service: previous.credential_service.clone(),
            credential_endpoint: previous.credential_endpoint.clone(),
        };
        let new_service = patch
            .api_key
            .as_ref()
            .map(|_| format!("desktop-model-{}", uuid::Uuid::new_v4()));
        if let (Some(service), Some(key)) = (&new_service, &patch.api_key) {
            self.backend
                .set(service, &SecretBuf::from_str(key))
                .map_err(|_| Error::CredentialUnavailable)?;
            // A backend that accepts but cannot read back must not claim persistence.
            let confirmed = self
                .backend
                .get(service)
                .is_ok_and(|secret| secret.expose() == key.as_bytes());
            if !confirmed {
                let _ = self.backend.delete(service);
                return Err(Error::CredentialUnavailable);
            }
            profile.credential_service = Some(service.clone());
            profile.credential_endpoint = Some(profile.base_url.clone());
        }
        let bytes = serde_json::to_vec_pretty(&profile).map_err(|_| Error::Storage)?;
        if (self.persist)(&self.root.join(FILE), &bytes, OWNER_ONLY_MODE).is_err() {
            if let Some(service) = new_service {
                let _ = self.backend.delete(&service);
            }
            return Err(Error::Storage);
        }
        // The current runtime keeps the boot value in the shared hot store;
        // it never rereads the retired OS slot after this profile commit.
        if previous.credential_service != profile.credential_service {
            if let Some(service) = previous.credential_service {
                let _ = self.backend.delete(&service);
            }
        }
        Ok(self.view(&profile))
    }
}

impl ModelConnectionService for ManagedConnection {
    fn read(&self) -> Result<View, Error> {
        scope(&self.root)?;
        Ok(self.view(&load(&self.root)?))
    }

    fn save(&self, patch: Patch) -> Result<View, Error> {
        let _guard = self.writes.lock().map_err(|_| Error::Storage)?;
        storage_atomic::with_file_lock(
            &storage_atomic::lock_path_for(&self.root.join(FILE)),
            || self.save_locked(patch),
        )
        .map_err(|_| Error::Storage)?
    }
}

/// Called before runtime assembly. No profile/key is a valid editing-only start.
pub(crate) fn prepare() -> Result<Option<Arc<dyn ModelConnectionService>>, String> {
    if !managed() {
        return Ok(None);
    }
    if !cfg!(windows) {
        return Err(
            "managed desktop model connection requires the Windows credential backend".into(),
        );
    }
    if !apeireth_gateway::GatewayTokenGate::from_env()
        .enforced_token()
        .is_some_and(|token| token.len() >= 32)
        || std::env::var_os("APEIRETH_GATEWAY_TOKENS").is_some()
    {
        return Err("managed desktop requires its own enforced operator token".into());
    }
    let root = crate::default_panel_data_dir();
    fs_err::create_dir_all(&root).map_err(|_| "managed model data root unavailable")?;
    let prefix = namespace(&root).map_err(|_| "managed model namespace unavailable")?;
    let backend = Arc::new(PlatformKeyring::new(prefix, Arc::new(CountingAudit::new())));
    let service = Arc::new(
        ManagedConnection::new(root, backend)
            .map_err(|_| "managed model profile refused to open")?,
    );
    service.restore();
    // Only non-secret process startup configuration is written here.
    std::env::set_var("APEIRETH_OPENAI_URL", &service.boot.base_url);
    std::env::set_var("APEIRETH_OPENAI_MODELS", &service.boot.model);
    let model = apeireth_provider::provider_model::ProviderModel::from_configured(
        service.boot.model.clone(),
        &apeireth_core::kernel::CapabilityId::new("provider.openai-compatible")
            .expect("stable provider identity"),
        [],
    )
    .map_err(|_| "managed model identity is invalid")?;
    std::env::set_var("APEIRETH_MODEL", model.canonical_id().as_str());
    Ok(Some(service))
}

#[cfg(test)]
#[path = "desktop_model_connection_tests.rs"]
mod tests;
