//! # GET /v1/update/version endpoint
//!
//! 借鉴 Golutra `/api/desktop-updater/version?channel=` 协议 (per
//! [`docs/stage4/BORROW_FROM_GOLUTRA.md` §8 P3 第 10-11 项](file:///))
//! 1:1 翻译到 Rust, 跟 `endpoint.rs::/check` + `/apply` 3 端点 schema 镜像.
//!
//! ## 协议 (per 借鉴文档 §8 P3 第 11 项 + cosign.yml 1:1 镜像)
//!
//! - `GET /v1/update/version?channel={stable|beta|nightly}` → [`VersionResponse`]
//! - 响应: `version` + `channel` + `fingerprint` (信任公钥指纹) + `minisign_required` (K-1 强校验) +
//!   `request_id` + `real_mode: true` (0 假装 STUB 模式)
//!
//! ## K-1 强校验 (5 校验, 跟 `endpoint.rs` / `signature.rs` 镜像)
//!
//! 1. `channel` 必为 `Stable` / `Beta` / `Nightly` 之一
//! 2. `current_version` 必 semver 格式 (K-1 强校验 version_string)
//! 3. `minisign_required = true` (0 假装无签名)
//! 4. `fingerprint` 必 16 字符 hex (K-1 强校验 fingerprint_hex)
//! 5. `request_id` 必 UUID v4 格式 (跨请求追踪)
//!
//! ## ⏳ R21+ 续真接
//!
//! - 真实 GitHub Releases `/repos/{owner}/{repo}/releases/latest` fetch
//! - 真实 trust public key 加载 (替换 TestFixture)
//! - 真实 Rekor transparency log 查询 (per cosign.yml verify job)

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{validate_fingerprint_hex, validate_version_string, UpdateError, UpdateResult};
use crate::release::Channel;
use crate::signature::SignatureAlgorithm;

// ============================================================================
// §1 Version 协议常量 (K-1 强校验, 编译期 hardcode)
// ============================================================================

/// 协议版本 (per 借鉴文档 §8 P3 第 10 项 cosign + minisign 协议).
///
/// K-1 强校验: 编译期 hardcode, 8 项不修改承诺 #2 公开 API 100% 文档化.
pub const VERSION_PROTOCOL: &str = "minisign-1";

/// 协议要求的算法 (K-1 强校验: 编译期 hardcode `Ed25519`).
///
/// 8 项不修改承诺 #7: 0 重复造轮子, 借 minisign crate 0.9.
pub const VERSION_ALGORITHM: SignatureAlgorithm = SignatureAlgorithm::Ed25519;

/// K-1 强校验: 协议版本字符串 = 10 字符.
pub const VERSION_PROTOCOL_LEN: usize = 10;

const _: () = assert!(VERSION_PROTOCOL.len() == VERSION_PROTOCOL_LEN);
const _: () = assert!(
    VERSION_PROTOCOL.eq_ignore_ascii_case("minisign-1"),
    "VERSION_PROTOCOL must be minisign-1"
);

// ============================================================================
// §2 GET /v1/update/version — request/response schema
// ============================================================================

/// GET /v1/update/version 请求 schema (per 借鉴文档 §8 P3 第 11 项 + cosign.yml 1:1 镜像).
///
/// 公开 API (8 项不修改承诺 #2):
/// - `channel`: Release 通道 (default = `stable`, K-1 强校验 enum 守门)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionRequest {
    /// Release 通道 (default = `stable`).
    pub channel: Option<String>,
}

impl VersionRequest {
    /// K-1 强校验: 必填字段数.
    pub const REQUIRED_FIELDS: u8 = 1;
}

/// GET /v1/update/version 响应 schema (per 借鉴文档 §8 P3 第 11 项 + cosign.yml verify job 1:1 镜像).
///
/// 公开 API (8 项不修改承诺 #2):
/// - `version`: 客户端当前应跑的版本 (semver, K-1 强校验)
/// - `channel`: 响应通道
/// - `fingerprint`: 信任公钥指纹 (16 hex, K-1 强校验)
/// - `minisign_required`: true (协议必走 minisign 验签, 0 假装无签名)
/// - `algorithm`: `ed25519` (编译期 hardcode, K-1 强校验)
/// - `request_id`: UUID v4
/// - `real_mode`: true (0 假装 STUB 模式, 借鉴 #4 真签真验)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionResponse {
    /// 当前应跑版本 (semver, e.g. `1.0.0`).
    pub version: String,
    /// Release 通道 (`stable` / `beta` / `nightly`).
    pub channel: Channel,
    /// 信任公钥指纹 (16 hex, 跟 `TrustedKey` 枚举 1:1 镜像).
    pub fingerprint: String,
    /// 是否要求 minisign 验签 (K-1 强校验: 必 `true`, 0 假装无签名).
    pub minisign_required: bool,
    /// 签名算法 (K-1 强校验: 编译期 hardcode `ed25519`).
    pub algorithm: SignatureAlgorithm,
    /// 协议版本 (K-1 强校验: 编译期 hardcode `minisign-1`).
    pub protocol: String,
    /// Request ID (UUID v4, 跨请求追踪).
    pub request_id: String,
    /// 真实模式标记 (0 假装 STUB 模式).
    pub real_mode: bool,
}

impl VersionResponse {
    /// K-1 强校验: 必填字段数.
    pub const REQUIRED_FIELDS: u8 = 8;
}

// ============================================================================
// §3 handler: GET /v1/update/version (跟 /check + /apply 镜像)
// ============================================================================

/// 构造 GET /v1/update/version 响应 (K-1 强校验 5 步, 跟 cosign.yml 1:1 镜像).
///
/// 公开 API (8 项不修改承诺 #2 公开 API 100% 文档化).
///
/// # 参数
///
/// - `req`: [`VersionRequest`] 客户端请求
/// - `server_version`: 服务端报告的当前版本 (semver, R21+ 改 GitHub API fetch)
/// - `server_fingerprint`: 服务端信任公钥指纹 (16 hex)
///
/// # 错误
///
/// - [`UpdateError::InvalidSemver`] `server_version` 不 semver
/// - [`UpdateError::InvalidRequest`] channel 解析失败
/// - [`UpdateError::UntrustedPublicKey`] `server_fingerprint` 格式错
pub fn handle_version_request(
    req: VersionRequest,
    server_version: &str,
    server_fingerprint: &str,
) -> UpdateResult<VersionResponse> {
    // K-1 强校验 1: server_version 必 semver
    validate_version_string(server_version)?;

    // K-1 强校验 2: server_fingerprint 必 16 hex
    validate_fingerprint_hex(server_fingerprint)?;

    // K-1 强校验 3: 解析 channel (default = Stable)
    let channel = match &req.channel {
        Some(s) => Channel::parse(s)?,
        None => Channel::Stable,
    };

    Ok(VersionResponse {
        version: server_version.to_string(),
        channel,
        fingerprint: server_fingerprint.to_string(),
        minisign_required: true, // K-1 强校验 4: 0 假装无签名
        algorithm: VERSION_ALGORITHM, // K-1 强校验 5: 编译期 hardcode Ed25519
        protocol: VERSION_PROTOCOL.to_string(),
        request_id: Uuid::new_v4().to_string(),
        real_mode: true, // 0 假装 STUB 模式
    })
}

// ============================================================================
// §4 单元测试 (K-1 强校验守门 + schema 守门)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn protocol_version_is_minisign_1() {
        assert_eq!(VERSION_PROTOCOL, "minisign-1");
        assert_eq!(VERSION_PROTOCOL.len(), 10);
        assert_eq!(VERSION_PROTOCOL_LEN, 10);
    }

    #[test]
    fn version_algorithm_is_ed25519() {
        assert_eq!(VERSION_ALGORITHM, SignatureAlgorithm::Ed25519);
        assert_eq!(VERSION_ALGORITHM.as_str(), "ed25519");
    }

    #[test]
    fn version_request_required_fields_is_1() {
        assert_eq!(VersionRequest::REQUIRED_FIELDS, 1);
    }

    #[test]
    fn version_response_required_fields_is_8() {
        assert_eq!(VersionResponse::REQUIRED_FIELDS, 8);
    }

    #[test]
    fn handle_version_request_returns_valid_response() {
        let req = VersionRequest { channel: None };
        let resp = handle_version_request(req, "1.0.0", "99F790EC4BE6E38D")
            .expect("handle_version_request must succeed for valid input");
        assert_eq!(resp.version, "1.0.0");
        assert_eq!(resp.channel, Channel::Stable);
        assert_eq!(resp.fingerprint, "99F790EC4BE6E38D");
        assert!(resp.minisign_required);
        assert_eq!(resp.algorithm, SignatureAlgorithm::Ed25519);
        assert_eq!(resp.protocol, "minisign-1");
        assert!(!resp.request_id.is_empty());
        assert!(resp.real_mode);
    }

    #[test]
    fn handle_version_request_with_beta_channel() {
        let req = VersionRequest {
            channel: Some("beta".to_string()),
        };
        let resp = handle_version_request(req, "1.1.0-beta.1", "99F790EC4BE6E38D")
            .expect("beta channel must parse");
        assert_eq!(resp.channel, Channel::Beta);
        assert_eq!(resp.version, "1.1.0-beta.1");
    }

    #[test]
    fn handle_version_request_rejects_invalid_version() {
        let req = VersionRequest { channel: None };
        let result = handle_version_request(req, "not-a-version", "99F790EC4BE6E38D");
        assert!(matches!(result, Err(UpdateError::InvalidSemver(_))));
    }

    #[test]
    fn handle_version_request_rejects_empty_version() {
        let req = VersionRequest { channel: None };
        let result = handle_version_request(req, "", "99F790EC4BE6E38D");
        assert!(matches!(result, Err(UpdateError::InvalidSemver(_))));
    }

    #[test]
    fn handle_version_request_rejects_invalid_fingerprint() {
        let req = VersionRequest { channel: None };
        // 15 chars (非 16)
        let result = handle_version_request(req, "1.0.0", "99F790EC4BE6E38");
        assert!(matches!(result, Err(UpdateError::UntrustedPublicKey { .. })));
    }

    #[test]
    fn handle_version_request_rejects_unknown_channel() {
        let req = VersionRequest {
            channel: Some("lts".to_string()),
        };
        let result = handle_version_request(req, "1.0.0", "99F790EC4BE6E38D");
        assert!(matches!(result, Err(UpdateError::InvalidRequest(_))));
    }

    #[test]
    fn version_response_serializes_with_all_fields() {
        let req = VersionRequest { channel: None };
        let resp = handle_version_request(req, "1.0.0", "99F790EC4BE6E38D").unwrap();
        let json = serde_json::to_string(&resp).expect("serialize must succeed");
        // K-1 强校验 5 字段全在
        assert!(json.contains("\"version\":\"1.0.0\""));
        assert!(json.contains("\"channel\":\"stable\""));
        assert!(json.contains("\"fingerprint\":\"99F790EC4BE6E38D\""));
        assert!(json.contains("\"minisign_required\":true"));
        assert!(json.contains("\"algorithm\":\"ed25519\""));
        assert!(json.contains("\"protocol\":\"minisign-1\""));
        assert!(json.contains("\"real_mode\":true"));
        assert!(json.contains("\"request_id\":"));
    }

    #[test]
    fn version_response_deserializes_round_trip() {
        let req = VersionRequest { channel: None };
        let resp = handle_version_request(req, "1.0.0", "99F790EC4BE6E38D").unwrap();
        let json = serde_json::to_string(&resp).unwrap();
        let de: VersionResponse = serde_json::from_str(&json).expect("deserialize must succeed");
        assert_eq!(de.version, resp.version);
        assert_eq!(de.channel, resp.channel);
        assert_eq!(de.fingerprint, resp.fingerprint);
        assert_eq!(de.protocol, resp.protocol);
    }
}
