//! # GitHub Release data types
//!
//! 借鉴 Golutra P3 `/api/desktop-updater/check?current_version=` 协议思想
//! (per [`docs/stage4/BORROW_FROM_GOLUTRA.md` §8 P3 第 11 项](file:///)),
//! 1:1 翻译到 Rust, 0 真连 GitHub Releases API.
//!
//! ## Channel 枚举
//!
//! K-1 强校验: [`Channel`] 编译期 hardcode 3 值 (Stable / Beta / Nightly).
//! R21+ 真接时不变 (8 项不修改承诺 #5).
//!
//! ## ⏳ R21+ 续真接
//!
//! - 真实 GitHub Releases API (网络依赖, 当前 stub).
//! - Channel 4 (估加 `Lts` / `Edge`).

use serde::{Deserialize, Serialize};

use crate::error::{validate_sha256_hex, validate_signature_b64, validate_version_string, UpdateError, UpdateResult};
use crate::signature::SignatureAlgorithm;

// ============================================================================
// §1 Channel 枚举 (K-1 强校验, 编译期 hardcode)
// ============================================================================

/// Release 通道 (3 值固定, 8 项不修改承诺 #5).
///
/// 顺序固定: `Stable` → `Beta` → `Nightly`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Channel {
    /// Stable release (生产).
    Stable,
    /// Beta release (RC, 1.0 release 前置).
    Beta,
    /// Nightly release (每日构建, 仅 dev).
    Nightly,
}

impl Channel {
    /// 全部 3 通道.
    pub const ALL: &'static [Channel] = &[Channel::Stable, Channel::Beta, Channel::Nightly];

    /// 编译期守门: 3 通道.
    pub const COUNT: usize = 3;

    /// 通道字符串名 (per 借鉴文档 §8 P3 第 11 项 `current_version=1.0.0&channel=stable`).
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Channel::Stable => "stable",
            Channel::Beta => "beta",
            Channel::Nightly => "nightly",
        }
    }

    /// 从字符串解析 (URL query / JSON 反序列化).
    pub fn parse(s: &str) -> UpdateResult<Self> {
        match s.to_ascii_lowercase().as_str() {
            "stable" => Ok(Channel::Stable),
            "beta" => Ok(Channel::Beta),
            "nightly" => Ok(Channel::Nightly),
            other => Err(UpdateError::InvalidRequest(format!(
                "unknown channel: {}",
                other
            ))),
        }
    }
}

const _: () = assert!(Channel::COUNT == 3);

// ============================================================================
// §2 Asset — 单个 release 资产
// ============================================================================

/// 单个 release 资产 (binary, tarball, ...).
///
/// K-1 强校验:
/// - `sha256`: 64 hex 字符 (K-1 校验函数 [`validate_sha256_hex`])
/// - `size_bytes`: > 0
/// - `algorithm`: [`SignatureAlgorithm::Ed25519`] (编译期 hardcode)
///
/// 借鉴文档 §8 P3 第 10 项 Golutra 模式: minisign signature + SHA-256 双校验.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Asset {
    /// Asset 名 (e.g. `apeireth-v1.0.0-x86_64-unknown-linux-gnu.tar.gz`).
    pub name: String,
    /// 下载 URL (per Golutra `/api/desktop-updater/check?current_version=` 协议).
    pub url: String,
    /// 字节大小.
    pub size_bytes: u64,
    /// SHA-256 hex (64 chars).
    pub sha256: String,
    /// minisign 签名 base64 (8 项不修改承诺 #7 用 minisign crate 验签).
    pub signature_b64: String,
    /// 签名算法 (K-1 强校验: 编译期 hardcode Ed25519).
    pub algorithm: SignatureAlgorithm,
}

impl Asset {
    /// 校验 Asset 完整性 (5 K-1 强校验).
    pub fn validate(&self) -> UpdateResult<()> {
        validate_sha256_hex(&self.sha256)?;
        if self.size_bytes == 0 {
            return Err(UpdateError::InvalidRequest(format!(
                "asset {} has zero size",
                self.name
            )));
        }
        validate_signature_b64(&self.signature_b64)?;
        if self.algorithm != SignatureAlgorithm::Ed25519 {
            return Err(UpdateError::InvalidRequest(format!(
                "asset {} algorithm mismatch: expected ed25519, got {}",
                self.name,
                self.algorithm.as_str()
            )));
        }
        Ok(())
    }
}

// ============================================================================
// §3 Release — GitHub Release 完整结构
// ============================================================================

/// GitHub Release 完整结构 (mock data + future real fetch).
///
/// 借鉴文档 §8 P3 第 11 项 协议: 当前 stub, R21+ 真接时调
/// `https://api.github.com/repos/{owner}/{repo}/releases/latest` 拉数据.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Release {
    /// Tag (e.g. `v1.0.0`).
    pub tag: String,
    /// Semver 版本 (从 tag 解析, 去掉 `v` 前缀).
    pub version: String,
    /// 通道.
    pub channel: Channel,
    /// Release notes (markdown).
    pub notes: String,
    /// 发布时间 (RFC 3339, 8 项不修改承诺 #4 编译期 hardcode chrono::DateTime format).
    pub published_at: String,
    /// 资产列表 (binary + signature + checksum).
    pub assets: Vec<Asset>,
    /// 是否预发布.
    pub prerelease: bool,
}

impl Release {
    /// 校验 Release 完整性 (Asset 列表 5 K-1 强校验 + version 校验).
    pub fn validate(&self) -> UpdateResult<()> {
        validate_version_string(&self.version)?;
        for asset in &self.assets {
            asset.validate()?;
        }
        Ok(())
    }
}

// ============================================================================
// §4 UpdateInfo — Updater.check_for_update 返回值
// ============================================================================

/// `Updater::check_for_update` 返回值 (None = 已是最新).
///
/// 借鉴文档 §8 P3 第 11 项 协议 schema: 当前 `update_info` 比 Golutra 多了
/// `signature_algorithm` 字段 (K-1 强校验: 编译期 hardcode).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateInfo {
    /// 新版本 semver (e.g. `1.0.0`).
    pub version: String,
    /// Release tag (e.g. `v1.0.0`).
    pub tag: String,
    /// Release notes (markdown, 简化版).
    pub notes: String,
    /// 首选 Asset (按 platform 自动选).
    pub asset: Asset,
    /// Channel (stable / beta / nightly).
    pub channel: Channel,
    /// 发布时间 (RFC 3339).
    pub published_at: String,
    /// 必填 (K-1 强校验): 8 项不修改承诺 #2 公开 API 100% 文档化 — 必填字段 5 个.
    pub required_fields_count: u8,
}

impl UpdateInfo {
    /// 公开 API 必填字段数 (5 K-1 强校验 + 8 项不修改承诺 #2).
    pub const REQUIRED_FIELDS: u8 = 5;
}

// ============================================================================
// §5 单元测试 (K-1 强校验 + Channel 守门)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn channel_count_is_3() {
        assert_eq!(Channel::COUNT, 3);
        assert_eq!(Channel::ALL.len(), 3);
    }

    #[test]
    fn channel_order_locked() {
        // 8 项不修改承诺 #5: 枚举顺序固定
        assert_eq!(Channel::ALL[0], Channel::Stable);
        assert_eq!(Channel::ALL[1], Channel::Beta);
        assert_eq!(Channel::ALL[2], Channel::Nightly);
    }

    #[test]
    fn channel_as_str() {
        assert_eq!(Channel::Stable.as_str(), "stable");
        assert_eq!(Channel::Beta.as_str(), "beta");
        assert_eq!(Channel::Nightly.as_str(), "nightly");
    }

    #[test]
    fn channel_parse_accepts_known() {
        assert_eq!(Channel::parse("stable").unwrap(), Channel::Stable);
        assert_eq!(Channel::parse("BETA").unwrap(), Channel::Beta);
        assert_eq!(Channel::parse("Nightly").unwrap(), Channel::Nightly);
    }

    #[test]
    fn channel_parse_rejects_unknown() {
        assert!(Channel::parse("lts").is_err());
        assert!(Channel::parse("edge").is_err());
        assert!(Channel::parse("").is_err());
    }

    #[test]
    fn required_fields_count_is_5() {
        assert_eq!(UpdateInfo::REQUIRED_FIELDS, 5);
    }

    #[test]
    fn asset_validate_rejects_bad_sha256() {
        let asset = Asset {
            name: "test".to_string(),
            url: "https://example.com/test".to_string(),
            size_bytes: 1024,
            sha256: "abc".to_string(), // 太短
            signature_b64: "a".repeat(100),
            algorithm: SignatureAlgorithm::Ed25519,
        };
        assert!(asset.validate().is_err());
    }

    #[test]
    fn asset_validate_rejects_zero_size() {
        let asset = Asset {
            name: "test".to_string(),
            url: "https://example.com/test".to_string(),
            size_bytes: 0, // zero
            sha256: "a".repeat(64),
            signature_b64: "a".repeat(100),
            algorithm: SignatureAlgorithm::Ed25519,
        };
        assert!(asset.validate().is_err());
    }

    #[test]
    fn asset_validate_rejects_short_signature() {
        let asset = Asset {
            name: "test".to_string(),
            url: "https://example.com/test".to_string(),
            size_bytes: 1024,
            sha256: "a".repeat(64),
            signature_b64: "abc".to_string(), // 太短
            algorithm: SignatureAlgorithm::Ed25519,
        };
        assert!(asset.validate().is_err());
    }

    #[test]
    fn asset_validate_accepts_valid() {
        let asset = Asset {
            name: "test".to_string(),
            url: "https://example.com/test".to_string(),
            size_bytes: 1024,
            sha256: "a".repeat(64),
            signature_b64: "a".repeat(100),
            algorithm: SignatureAlgorithm::Ed25519,
        };
        assert!(asset.validate().is_ok());
    }

    #[test]
    fn release_validate_rejects_bad_version() {
        let release = Release {
            tag: "v1.0.0".to_string(),
            version: "".to_string(), // bad
            channel: Channel::Stable,
            notes: "".to_string(),
            published_at: "2026-08-06T00:00:00Z".to_string(),
            assets: vec![],
            prerelease: false,
        };
        assert!(release.validate().is_err());
    }
}
