//! Explicit single-operator composition shared by tools and HTTP.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use apeireth_governance::{Permission, PermissionPolicy};
use apeireth_protocol::artifacts::ArtifactService;
use apeireth_runtime::RuntimeBuilder;
use apeireth_runtime_assembly::artifact_tools::{
    ArtifactTool, ARTIFACT_READ_CAPABILITY, ARTIFACT_WRITE_CAPABILITY,
};

pub(crate) type ArtifactHandle = Option<Arc<dyn ArtifactService>>;

/// No hidden global handle registry and no implicit fallback from user domains.
pub(crate) async fn open(root: PathBuf) -> Result<ArtifactHandle, String> {
    match std::fs::symlink_metadata(root.join("users.json")) {
        Ok(_) => return Ok(None),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(format!("artifact operator scope unavailable: {error}")),
    }
    let service = apeireth_runtime_assembly::sqlite_artifacts::SqliteArtifactService::open(root)
        .await
        .map_err(|error| format!("managed artifact service failed to open: {error}"))?;
    Ok(Some(Arc::new(service)))
}

/// The same Arc is returned to the caller and injected into both tool faces.
/// The ordinary file-write switch grants no permission to this managed store.
pub(crate) fn attach(
    builder: RuntimeBuilder,
    policy: &Mutex<PermissionPolicy>,
    artifacts: ArtifactHandle,
) -> RuntimeBuilder {
    let Some(service) = artifacts else {
        return builder;
    };
    {
        let mut permissions = policy.lock().expect("permission policy lock poisoned");
        permissions.grant(Permission::ExecuteTool(ARTIFACT_READ_CAPABILITY.into()));
        permissions.grant(Permission::ExecuteTool(ARTIFACT_WRITE_CAPABILITY.into()));
        permissions.require_approval_for(ARTIFACT_WRITE_CAPABILITY);
    }
    builder
        .with_capability(Arc::new(ArtifactTool::read(service.clone())))
        .with_capability(Arc::new(ArtifactTool::write(service)))
}
