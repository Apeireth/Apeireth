//! Sandbox validator — 物理隔离守门 (v6 §4 重守门之守门 3).

use crate::manifest::UpgradeManifest;

/// Sandbox 校验结果.
#[derive(Debug, Clone)]
pub enum SandboxVerdict {
    /// 接受升级.
    Accept,
    /// 拒绝升级 (含原因).
    Reject(String),
}

/// Sandbox validator trait — 物理隔离守门 3 的接口.
///
/// 实现方可以是 WASM sandbox / 进程隔离 / 文件权限隔离等 (v6 §1350).
pub trait SandboxValidator {
    /// 校验 manifest 是否可在沙盒中安全执行.
    fn validate(&self, manifest: &UpgradeManifest) -> SandboxVerdict;
}

/// 默认 sandbox validator — 仅校验 version 非空.
pub struct DefaultSandbox;

impl SandboxValidator for DefaultSandbox {
    fn validate(&self, manifest: &UpgradeManifest) -> SandboxVerdict {
        if manifest.version.is_empty() {
            SandboxVerdict::Reject("empty version".to_string())
        } else if manifest.kind == crate::manifest::UpgradeKind::ELayerMutation {
            // E 层修改需更严格的隔离 (v6 §1350)
            SandboxVerdict::Reject("E-layer mutation requires explicit sandbox".to_string())
        } else {
            SandboxVerdict::Accept
        }
    }
}

/// 严格 sandbox validator — 拒绝所有 E 层修改.
pub struct StrictSandbox;

impl SandboxValidator for StrictSandbox {
    fn validate(&self, manifest: &UpgradeManifest) -> SandboxVerdict {
        if manifest.version.is_empty() {
            return SandboxVerdict::Reject("empty version".to_string());
        }
        if manifest.kind == crate::manifest::UpgradeKind::ELayerMutation {
            return SandboxVerdict::Reject("E-layer mutation forbidden in strict mode".to_string());
        }
        SandboxVerdict::Accept
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::{ManifestBuilder, UpgradeKind};

    fn patch_manifest() -> UpgradeManifest {
        ManifestBuilder::new("v1.0.0", UpgradeKind::Patch)
            .with_description("sandbox test")
            .with_content_hash("hash")
            .build()
    }

    #[test]
    fn default_sandbox_accepts_patch() {
        let m = patch_manifest();
        let v = DefaultSandbox.validate(&m);
        assert!(matches!(v, SandboxVerdict::Accept));
    }

    #[test]
    fn default_sandbox_rejects_e_layer() {
        let m = ManifestBuilder::new("v2.0.0", UpgradeKind::ELayerMutation).build();
        let v = DefaultSandbox.validate(&m);
        assert!(matches!(v, SandboxVerdict::Reject(_)));
    }

    #[test]
    fn default_sandbox_rejects_empty_version() {
        let m = ManifestBuilder::new("", UpgradeKind::Patch).build();
        let v = DefaultSandbox.validate(&m);
        assert!(matches!(v, SandboxVerdict::Reject(_)));
    }

    #[test]
    fn strict_sandbox_rejects_e_layer() {
        let m = ManifestBuilder::new("v1.0.0", UpgradeKind::ELayerMutation).build();
        let v = StrictSandbox.validate(&m);
        assert!(matches!(v, SandboxVerdict::Reject(_)));
    }

    #[test]
    fn strict_sandbox_accepts_patch() {
        let m = patch_manifest();
        let v = StrictSandbox.validate(&m);
        assert!(matches!(v, SandboxVerdict::Accept));
    }
}
