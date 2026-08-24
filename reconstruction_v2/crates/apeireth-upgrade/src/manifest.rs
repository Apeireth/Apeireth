//! UpgradeManifest — 升级包元数据 (版本 + 内容 hash + 守门签名).

use chrono::Utc;
use uuid::Uuid;

use crate::UpgradeError;

/// 升级种类.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpgradeKind {
    /// 普通补丁 (非 E 层).
    Patch,
    /// 次版本升级 (非 E 层).
    Minor,
    /// 主版本升级 (非 E 层).
    Major,
    /// **E 层修改** (需 5 重治理 + 物理隔离, v6 §177 LOCKED).
    ELayerMutation,
}

/// 升级 manifest.
#[derive(Debug, Clone)]
pub struct UpgradeManifest {
    /// 唯一 ID.
    pub id: Uuid,
    /// 版本号 (semver-like 字符串).
    pub version: String,
    /// 升级种类.
    pub kind: UpgradeKind,
    /// 描述.
    pub description: String,
    /// 内容 hash (SHA-256 hex).
    pub content_hash: String,
    /// 创建时间 (Unix seconds).
    pub created_at: i64,
}

impl UpgradeManifest {
    /// 校验 manifest 合法性.
    pub fn validate(&self) -> Result<(), UpgradeError> {
        if self.version.is_empty() {
            return Err(UpgradeError::InvalidManifest("empty version".to_string()));
        }
        if self.content_hash.is_empty() {
            return Err(UpgradeError::InvalidManifest(
                "empty content_hash".to_string(),
            ));
        }
        Ok(())
    }
}

/// Manifest builder — 流式 API.
pub struct ManifestBuilder {
    id: Uuid,
    version: String,
    kind: UpgradeKind,
    description: String,
    content_hash: String,
    created_at: i64,
}

impl ManifestBuilder {
    /// 构造 builder.
    pub fn new(version: impl Into<String>, kind: UpgradeKind) -> Self {
        Self {
            id: Uuid::new_v4(),
            version: version.into(),
            kind,
            description: String::new(),
            content_hash: String::new(),
            created_at: Utc::now().timestamp(),
        }
    }

    /// 添加描述.
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    /// 设置内容 hash.
    pub fn with_content_hash(mut self, hash: impl Into<String>) -> Self {
        self.content_hash = hash.into();
        self
    }

    /// 设置 ID (测试用).
    pub fn with_id(mut self, id: Uuid) -> Self {
        self.id = id;
        self
    }

    /// 构建 manifest.
    pub fn build(self) -> UpgradeManifest {
        UpgradeManifest {
            id: self.id,
            version: self.version,
            kind: self.kind,
            description: self.description,
            content_hash: self.content_hash,
            created_at: self.created_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_builder_basic() {
        let m = ManifestBuilder::new("v1.0.0", UpgradeKind::Patch).build();
        assert_eq!(m.version, "v1.0.0");
        assert_eq!(m.kind, UpgradeKind::Patch);
        assert!(m.validate().is_err()); // content_hash 空
    }

    #[test]
    fn manifest_validate_rejects_empty_version() {
        let m = ManifestBuilder::new("", UpgradeKind::Patch)
            .with_content_hash("hash")
            .build();
        assert!(m.validate().is_err());
    }

    #[test]
    fn manifest_validate_rejects_empty_hash() {
        let m = ManifestBuilder::new("v1.0.0", UpgradeKind::Patch).build();
        assert!(m.validate().is_err());
    }

    #[test]
    fn manifest_validate_accepts_complete_manifest() {
        let m = ManifestBuilder::new("v1.0.0", UpgradeKind::Patch)
            .with_content_hash("abc123")
            .with_description("test")
            .build();
        assert!(m.validate().is_ok());
    }

    #[test]
    fn upgrade_kind_variants_count() {
        let kinds = [
            UpgradeKind::Patch,
            UpgradeKind::Minor,
            UpgradeKind::Major,
            UpgradeKind::ELayerMutation,
        ];
        assert_eq!(kinds.len(), 4);
    }

    #[test]
    fn manifest_with_description_and_id() {
        let id = Uuid::new_v4();
        let m = ManifestBuilder::new("v1.0.0", UpgradeKind::Minor)
            .with_description("with desc")
            .with_content_hash("hash")
            .with_id(id)
            .build();
        assert_eq!(m.id, id);
        assert_eq!(m.description, "with desc");
        assert_eq!(m.kind, UpgradeKind::Minor);
    }
}
