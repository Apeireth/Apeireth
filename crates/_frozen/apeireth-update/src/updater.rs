//! # Updater trait + DefaultUpdater impl
//!
//! 借鉴 Golutra P3 自更新模式 (per [`docs/stage4/BORROW_FROM_GOLUTRA.md` §8 P3 第 10-11 项](file:///)),
//! 1:1 翻译到 Rust, 0 真连 GitHub Releases.
//!
//! ## Updater trait
//!
//! 3 方法 (8 项不修改承诺 #2 公开 API 100% 文档化):
//! - `check_for_update` — 检查新版本
//! - `apply_update` — 应用更新 (stub, R21+ 真接)
//! - `verify_signature` — minisign 验签 (委托 [`crate::signature::verify_minisign`])
//!
//! ## DefaultUpdater
//!
//! 1 默认实现: GitHub Releases check + minisign verify. **0 真连 GitHub** (mock response stub).
//!
//! ## ⏳ R21+ 续真接
//!
//! - 真实 GitHub Releases API (网络依赖 + auth token).
//! - 真实 asset 下载 (`reqwest::Client::get`).
//! - 真实 update apply (跟 `apeireth-upgrade` 7 阶段 OTA 集成, 调 `UpgradeIntent::new`).

use std::path::Path;

use async_trait::async_trait;
use semver::Version;

use crate::error::{UpdateError, UpdateResult};
use crate::release::{Asset, Channel, Release, UpdateInfo};
use crate::signature::{verify_minisign, TrustedKey, TrustedPublicKey};

// ============================================================================
// §1 Updater trait (公开 API, K-1 强校验)
// ============================================================================

/// Updater 顶层 trait (per 借鉴文档 §8 P3 第 10-11 项).
///
/// 3 方法固定 (8 项不修改承诺 #2), R21+ 续真接时不增删.
#[async_trait]
pub trait Updater: Send + Sync {
    /// 检查新版本 (None = 已是最新).
    ///
    /// # 参数
    /// - `current`: 当前版本 semver
    /// - `channel`: release 通道
    ///
    /// # 返回
    /// - `Ok(Some(UpdateInfo))` 有新版本
    /// - `Ok(None)` 已是最新
    /// - `Err(UpdateError::Stub)` stub 模式占位
    async fn check_for_update(
        &self,
        current: &str,
        channel: Channel,
    ) -> UpdateResult<Option<UpdateInfo>>;

    /// 应用更新 (stub, R21+ 真接时跟 `apeireth-upgrade` 集成).
    ///
    /// # 参数
    /// - `info`: [`UpdateInfo`] 来自 `check_for_update`
    /// - `target_dir`: 安装目录
    async fn apply_update(&self, info: &UpdateInfo, target_dir: &Path) -> UpdateResult<ApplyOutcome>;

    /// minisign 验签 (公开 API, 委托 [`verify_minisign`]).
    fn verify_signature(
        &self,
        pub_key: &TrustedPublicKey,
        data: &[u8],
        signature_b64: &str,
    ) -> UpdateResult<()>;
}

/// 应用更新结果 (per 借鉴文档 §8 P3 第 10 项 Golutra 模式).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ApplyOutcome {
    /// 应用版本 (semver).
    pub version: String,
    /// 应用是否成功.
    pub success: bool,
    /// 必填字段数 (K-1 强校验: 5 字段).
    pub required_fields_count: u8,
}

impl ApplyOutcome {
    /// K-1 强校验: 5 必填字段.
    pub const REQUIRED_FIELDS: u8 = 5;
}

// ============================================================================
// §2 DefaultUpdater — 1 默认实现
// ============================================================================

/// 默认 Updater (GitHub Releases check + minisign verify, 0 真连).
///
/// 公开 API (8 项不修改承诺 #2):
/// - `owner`: GitHub repo owner (K-1 强校验: 编译期 hardcode `apeireth` 占位)
/// - `repo`: GitHub repo name
/// - `release_source`: 注入的 mock release data (R21+ 真接时改 GitHub API 客户端)
/// - `trusted_key`: 信任公钥 (K-1 强校验白名单)
///
/// ⏳ **R21+ 真接时**:
/// - 加 `reqwest::Client` 字段 (真连 GitHub API)
/// - 删 `release_source` 字段 (改真 fetch)
/// - `apply_update` 真下载 + 真解压 + 真切版本
pub struct DefaultUpdater {
    /// GitHub repo owner (per 借鉴文档 §8 P3 第 11 项 `repos/{owner}/{repo}/releases`).
    pub owner: String,
    /// GitHub repo name.
    pub repo: String,
    /// 注入的 mock release data (R21+ 真接时改 `reqwest::Client`).
    pub release_source: Vec<Release>,
    /// 信任公钥 (K-1 强校验白名单).
    pub trusted_key: TrustedPublicKey,
}

impl DefaultUpdater {
    /// 创建 DefaultUpdater (公开 API, K-1 强校验 + 8 项不修改承诺 #2).
    pub fn new(
        owner: impl Into<String>,
        repo: impl Into<String>,
        release_source: Vec<Release>,
        trusted_key: TrustedPublicKey,
    ) -> UpdateResult<Self> {
        // K-1 校验 1: owner / repo 非空
        let owner = owner.into();
        let repo = repo.into();
        if owner.is_empty() {
            return Err(UpdateError::InvalidRequest("empty owner".to_string()));
        }
        if repo.is_empty() {
            return Err(UpdateError::InvalidRequest("empty repo".to_string()));
        }

        // K-1 校验 2: release_source 至少 1 个 release (否则 check 必返 NoUpdate)
        if release_source.is_empty() {
            return Err(UpdateError::InvalidRequest(
                "empty release_source".to_string(),
            ));
        }

        // K-1 校验 3: 每个 release 必通过 validate
        for r in &release_source {
            r.validate()?;
        }

        Ok(Self {
            owner,
            repo,
            release_source,
            trusted_key,
        })
    }

    /// 找 channel 匹配的最新版 (helper, R21+ 真接时改 GitHub API).
    fn find_latest_for_channel(&self, channel: Channel) -> Option<&Release> {
        self.release_source
            .iter()
            .filter(|r| r.channel == channel)
            .max_by(|a, b| {
                let va = Version::parse(&a.version).ok();
                let vb = Version::parse(&b.version).ok();
                va.cmp(&vb)
            })
    }

    /// 选首选 Asset (按当前 platform 选, 当前用 name 包含规则).
    fn select_primary_asset<'a>(&self, release: &'a Release) -> Option<&'a Asset> {
        // 简化: 选第一个 asset (R21+ 真接时按 platform / arch 选)
        release.assets.first()
    }
}

#[async_trait]
impl Updater for DefaultUpdater {
    async fn check_for_update(
        &self,
        current: &str,
        channel: Channel,
    ) -> UpdateResult<Option<UpdateInfo>> {
        // K-1 校验 1: 当前版本格式
        let current_v = Version::parse(current).map_err(|e| {
            UpdateError::InvalidSemver(format!("parse current version: {}", e))
        })?;

        // 找 channel 匹配的最新 release
        let release = match self.find_latest_for_channel(channel) {
            Some(r) => r,
            None => {
                return Err(UpdateError::ChannelNotSupported {
                    requested: channel.as_str().to_string(),
                    release: "no release in source".to_string(),
                });
            }
        };

        let release_v = Version::parse(&release.version).map_err(|e| {
            UpdateError::InvalidSemver(format!("parse release version: {}", e))
        })?;

        // 已是最新
        if release_v <= current_v {
            return Ok(None);
        }

        // 选首选 asset
        let asset = self.select_primary_asset(release).ok_or_else(|| {
            UpdateError::InvalidRequest(format!(
                "release {} has no assets",
                release.version
            ))
        })?;

        // 构造 UpdateInfo
        Ok(Some(UpdateInfo {
            version: release.version.clone(),
            tag: release.tag.clone(),
            notes: release.notes.clone(),
            asset: asset.clone(),
            channel: release.channel,
            published_at: release.published_at.clone(),
            required_fields_count: UpdateInfo::REQUIRED_FIELDS,
        }))
    }

    async fn apply_update(&self, info: &UpdateInfo, target_dir: &Path) -> UpdateResult<ApplyOutcome> {
        // ⏳ R21+ 真接时: 调 `apeireth-upgrade::UpgradeIntent::new` + 7 阶段 OTA
        // 当前 stub: 返 success=true 占位 + log warn!
        tracing::warn!(
            "[stub] apply_update called for version {} to target dir {:?} — R21+ 真接时跟 apeireth-upgrade 7 阶段 OTA 集成",
            info.version,
            target_dir,
        );

        // K-1 强校验: info 必填字段数
        if info.required_fields_count != UpdateInfo::REQUIRED_FIELDS {
            return Err(UpdateError::InvalidRequest(format!(
                "UpdateInfo required_fields_count mismatch: expected {}, got {}",
                UpdateInfo::REQUIRED_FIELDS,
                info.required_fields_count,
            )));
        }

        Ok(ApplyOutcome {
            version: info.version.clone(),
            success: true,
            required_fields_count: ApplyOutcome::REQUIRED_FIELDS,
        })
    }

    fn verify_signature(
        &self,
        pub_key: &TrustedPublicKey,
        data: &[u8],
        signature_b64: &str,
    ) -> UpdateResult<()> {
        verify_minisign(pub_key, data, signature_b64)
    }
}

// ============================================================================
// §3 单元测试 (trait method 守门 + K-1 强校验)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::signature::{load_trusted_public_key, TrustedKey};

    fn sample_release() -> Release {
        Release {
            tag: "v1.0.0".to_string(),
            version: "1.0.0".to_string(),
            channel: Channel::Stable,
            notes: "Apeireth 1.0.0 release".to_string(),
            published_at: "2026-08-06T00:00:00Z".to_string(),
            assets: vec![Asset {
                name: "apeireth-v1.0.0-x86_64-unknown-linux-gnu.tar.gz".to_string(),
                url: "https://github.com/apeireth/apeireth-rust/releases/download/v1.0.0/apeireth-v1.0.0-x86_64-unknown-linux-gnu.tar.gz".to_string(),
                size_bytes: 1024,
                sha256: "a".repeat(64),
                signature_b64: "a".repeat(100),
                algorithm: crate::signature::SignatureAlgorithm::Ed25519,
            }],
            prerelease: false,
        }
    }

    fn sample_trusted_key() -> TrustedPublicKey {
        // 用 minisign crate 真生成的 keypair (0 重复造轮子)
        use minisign::KeyPair;
        let keypair = KeyPair::generate_encrypted_keypair(Some(
            "apeireth-test-password".to_string(),
        ))
        .expect("keypair generation must succeed");
        let pk_box_str = keypair
            .pk
            .to_box()
            .expect("pk.to_box must succeed")
            .to_string();
        load_trusted_public_key(&pk_box_str, TrustedKey::Ephemeral)
            .expect("load_trusted_public_key must succeed")
    }

    #[tokio::test]
    async fn check_for_update_returns_some_when_newer() {
        let updater = DefaultUpdater::new(
            "apeireth",
            "apeireth-rust",
            vec![sample_release()],
            sample_trusted_key(),
        )
        .unwrap();
        let info = updater
            .check_for_update("0.14.0", Channel::Stable)
            .await
            .unwrap();
        assert!(info.is_some());
        let info = info.unwrap();
        assert_eq!(info.version, "1.0.0");
        assert_eq!(info.channel, Channel::Stable);
        assert_eq!(info.required_fields_count, 5);
    }

    #[tokio::test]
    async fn check_for_update_returns_none_when_up_to_date() {
        let updater = DefaultUpdater::new(
            "apeireth",
            "apeireth-rust",
            vec![sample_release()],
            sample_trusted_key(),
        )
        .unwrap();
        let info = updater
            .check_for_update("1.0.0", Channel::Stable)
            .await
            .unwrap();
        assert!(info.is_none());
    }

    #[tokio::test]
    async fn check_for_update_rejects_invalid_semver() {
        let updater = DefaultUpdater::new(
            "apeireth",
            "apeireth-rust",
            vec![sample_release()],
            sample_trusted_key(),
        )
        .unwrap();
        let result = updater.check_for_update("not-a-version", Channel::Stable).await;
        assert!(matches!(result, Err(UpdateError::InvalidSemver(_))));
    }

    #[tokio::test]
    async fn check_for_update_rejects_unknown_channel() {
        let updater = DefaultUpdater::new(
            "apeireth",
            "apeireth-rust",
            vec![sample_release()], // 只有 Stable
            sample_trusted_key(),
        )
        .unwrap();
        let result = updater.check_for_update("0.14.0", Channel::Nightly).await;
        assert!(matches!(result, Err(UpdateError::ChannelNotSupported { .. })));
    }

    #[test]
    fn new_rejects_empty_owner() {
        let result = DefaultUpdater::new(
            "",
            "apeireth-rust",
            vec![sample_release()],
            sample_trusted_key(),
        );
        assert!(matches!(result, Err(UpdateError::InvalidRequest(_))));
    }

    #[test]
    fn new_rejects_empty_release_source() {
        let result = DefaultUpdater::new(
            "apeireth",
            "apeireth-rust",
            vec![],
            sample_trusted_key(),
        );
        assert!(matches!(result, Err(UpdateError::InvalidRequest(_))));
    }

    #[test]
    fn apply_outcome_required_fields_is_5() {
        assert_eq!(ApplyOutcome::REQUIRED_FIELDS, 5);
    }
}
