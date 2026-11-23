//! Persist desktop-only startup settings and project live registration facts.

use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use apeireth_core::storage_atomic::{self, OWNER_ONLY_MODE};
use apeireth_gateway::desktop_capabilities::{
    DesktopCapabilityError as Error, DesktopCapabilityPatch, DesktopCapabilityService,
    DesktopCapabilitySettings, DesktopCapabilityView,
};
use apeireth_runtime::canonical::{behavior_adapter::registered_behavior_capability, Runtime};
use serde::{Deserialize, Serialize};

const FILE: &str = "desktop-capabilities.json";
const SCHEMA_VERSION: u32 = 1;
const PROFILE_MAX_BYTES: u64 = 4096;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Profile {
    schema_version: u32,
    revision: u64,
    configured: DesktopCapabilitySettings,
}

impl Default for Profile {
    fn default() -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            revision: 0,
            configured: DesktopCapabilitySettings::default(),
        }
    }
}

fn load(path: &Path) -> Result<Profile, Error> {
    match fs_err::symlink_metadata(path) {
        Ok(metadata) if metadata.is_file() && metadata.len() <= PROFILE_MAX_BYTES => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Profile::default()),
        _ => return Err(Error::Storage),
    }
    let mut bytes = Vec::new();
    fs_err::File::open(path)
        .map_err(|_| Error::Storage)?
        .take(PROFILE_MAX_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| Error::Storage)?;
    if bytes.len() as u64 > PROFILE_MAX_BYTES {
        return Err(Error::Storage);
    }
    let profile: Profile = serde_json::from_slice(&bytes).map_err(|_| Error::Storage)?;
    if profile.schema_version != SCHEMA_VERSION {
        return Err(Error::Storage);
    }
    Ok(profile)
}

pub(crate) struct Prepared {
    path: PathBuf,
    boot: DesktopCapabilitySettings,
}

impl Prepared {
    pub(crate) fn load(root: &Path) -> Result<Self, String> {
        let path = root.join(FILE);
        let profile = load(&path).map_err(|_| "managed desktop capability profile unavailable")?;
        Ok(Self {
            path,
            boot: profile.configured,
        })
    }

    pub(crate) fn settings(&self) -> DesktopCapabilitySettings {
        self.boot
    }

    pub(crate) fn attach(self, runtime: &Runtime) -> Arc<dyn DesktopCapabilityService> {
        let learning = registered_behavior_capability(
            apeireth_runtime_assembly::canonical::PREFERENCE_LEARNING_MODULE_ID,
        )
        .expect("static preference learning capability");
        let capabilities = runtime.capability_registry().capabilities();
        let installed = |id: &str| capabilities.iter().any(|tool| tool.id().as_str() == id);
        Arc::new(Service {
            path: self.path,
            effective: DesktopCapabilitySettings {
                project_file_tools: installed("tool.project_files_read")
                    && installed("tool.project_files_write"),
                preference_learning: installed(learning.as_str()),
                skill_search: installed(apeireth_tools_canonical::SKILL_SEARCH_CAPABILITY_ID),
            },
        })
    }
}

struct Service {
    path: PathBuf,
    effective: DesktopCapabilitySettings,
}

impl Service {
    fn view(&self, profile: Profile) -> DesktopCapabilityView {
        DesktopCapabilityView {
            schema_version: profile.schema_version,
            revision: profile.revision,
            configured: profile.configured,
            effective: self.effective,
            restart_required: profile.configured != self.effective,
        }
    }
}

impl DesktopCapabilityService for Service {
    fn read(&self) -> Result<DesktopCapabilityView, Error> {
        Ok(self.view(load(&self.path)?))
    }

    fn save(&self, patch: DesktopCapabilityPatch) -> Result<DesktopCapabilityView, Error> {
        storage_atomic::with_file_lock(&storage_atomic::lock_path_for(&self.path), || {
            let current = load(&self.path)?;
            if current.revision != patch.expected_revision {
                return Err(Error::Conflict);
            }
            if current.configured == patch.configured {
                return Ok(self.view(current));
            }
            let next = Profile {
                revision: current.revision.checked_add(1).ok_or(Error::Storage)?,
                configured: patch.configured,
                ..current
            };
            let bytes = serde_json::to_vec_pretty(&next).map_err(|_| Error::Storage)?;
            storage_atomic::write_atomic_durable(&self.path, &bytes, OWNER_ONLY_MODE)
                .map_err(|_| Error::Storage)?;
            Ok(self.view(next))
        })
        .map_err(|_| Error::Storage)?
    }
}

#[cfg(test)]
mod tests;
