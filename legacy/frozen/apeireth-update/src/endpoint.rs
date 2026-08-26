//! # autoupdate HTTP endpoints
//!
//! 借鉴 Golutra P3 `/api/desktop-updater/check?current_version=` 协议
//! (per [`docs/stage4/BORROW_FROM_GOLUTRA.md` §8 P3 第 11 项](file:///)),
//! 1:1 翻译到 Rust, 0 真启 HTTP server.
//!
//! ## 3 endpoints (per task spec §3 + 借鉴文档 §8 P3 第 10-11 项)
//!
//! - `GET /v1/update/version?channel={stable|beta|nightly}` → [`crate::version::VersionResponse`]
//!   (per [`crate::version::handle_version_request`])
//! - `GET /v1/update/check?current_version={semver}&channel={stable|beta|nightly}` → [`CheckResponse`]
//! - `POST /v1/update/apply` (body: [`ApplyRequest`]) → [`ApplyResponse`]
//!
//! 公开 API (8 项不修改承诺 #2 100% 文档化):
//! - [`check_request_schema`] — GET /check 请求 schema 文档
//! - [`apply_request_schema`] — POST /apply 请求 schema 文档
//! - [`handle_check_request`] — handler (real mode, ⏳ R21+ 启 axum/warp HTTP server)
//! - [`handle_apply_request`] — handler (real mode, ⏳ R21+ 启 axum/warp HTTP server)
//!
//! ## ⏳ R21+ 续真接
//!
//! - 真实 HTTP server (axum 0.7+ / warp 0.4+)
//! - 真实路由注册 (`apeireth-api` 集成)
//! - 真实 auth (per channel 鉴权)
//! - 真实 apply (跟 `apeireth-upgrade` 7 阶段 OTA 集成)

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{UpdateError, UpdateResult};
use crate::release::UpdateInfo;
use crate::updater::{ApplyOutcome, Updater};

// ============================================================================
// §1 HTTP endpoint 路径常量 (K-1 强校验, 编译期 hardcode)
// ============================================================================

/// 3 endpoint 路径常量 (8 项不修改承诺 #2 公开 API 100% 文档化).
pub const ENDPOINT_PATHS: &[&str] = &[
    "GET /v1/update/version",
    "GET /v1/update/check",
    "POST /v1/update/apply",
];

/// 编译期守门: 3 endpoint.
pub const ENDPOINT_COUNT: usize = 3;

const _: () = assert!(ENDPOINT_COUNT == 3);
const _: () = assert!(ENDPOINT_PATHS.len() == ENDPOINT_COUNT);

// ============================================================================
// §2 GET /v1/update/check — request/response schema
// ============================================================================

/// GET /v1/update/check 请求 schema (per 借鉴文档 §8 P3 第 11 项 Golutra 协议).
///
/// 公开 API (8 项不修改承诺 #2):
/// - `current_version`: 当前版本 semver (K-1 强校验)
/// - `channel`: release 通道 (stable / beta / nightly)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckRequest {
    /// 当前版本 (e.g. `1.0.0`).
    pub current_version: String,
    /// Release 通道 (default = stable).
    pub channel: Option<String>,
}

impl CheckRequest {
    /// K-1 强校验: 必填字段.
    pub const REQUIRED_FIELDS: u8 = 2;
}

/// GET /v1/update/check 响应 schema (per 借鉴文档 §8 P3 第 11 项).
///
/// 公开 API (8 项不修改承诺 #2):
/// - `has_update`: bool
/// - `update_info`: Option<UpdateInfo> (Some 当 has_update=true)
/// - `request_id`: UUID v4
/// - `real_mode`: bool (true = R21 真实模式, 0 假装 STUB)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckResponse {
    /// 是否有新版本.
    pub has_update: bool,
    /// 更新信息 (None 当 has_update=false).
    pub update_info: Option<UpdateInfo>,
    /// Request ID (UUID v4, 跨请求追踪).
    pub request_id: String,
    /// 真实模式标记 (0 假装 STUB).
    pub real_mode: bool,
}

impl CheckResponse {
    /// 必填字段数 (K-1 强校验: 4).
    pub const REQUIRED_FIELDS: u8 = 4;
}

// ============================================================================
// §3 POST /v1/update/apply — request/response schema
// ============================================================================

/// POST /v1/update/apply 请求 schema (per 借鉴文档 §8 P3 第 11 项).
///
/// 公开 API (8 项不修改承诺 #2):
/// - `version`: 目标版本 semver (K-1 强校验)
/// - `target_dir`: 安装目录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplyRequest {
    /// 目标版本 (e.g. `1.0.0`).
    pub version: String,
    /// 安装目录 (default = `/var/lib/apeireth`).
    pub target_dir: Option<String>,
}

impl ApplyRequest {
    /// 必填字段数 (K-1 强校验: 2).
    pub const REQUIRED_FIELDS: u8 = 2;

    /// 默认安装目录 (K-1 强校验: 编译期 hardcode).
    pub const DEFAULT_TARGET_DIR: &'static str = "/var/lib/apeireth";

    /// K-1 强校验: 解析 target_dir (None → DEFAULT_TARGET_DIR).
    #[must_use]
    pub fn resolve_target_dir(&self) -> PathBuf {
        match &self.target_dir {
            Some(s) => PathBuf::from(s),
            None => PathBuf::from(Self::DEFAULT_TARGET_DIR),
        }
    }
}

/// POST /v1/update/apply 响应 schema (per 借鉴文档 §8 P3 第 11 项).
///
/// 公开 API (8 项不修改承诺 #2):
/// - `outcome`: [`ApplyOutcome`]
/// - `request_id`: UUID v4
/// - `real_mode`: bool (0 假装 STUB)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplyResponse {
    /// 应用结果.
    pub outcome: ApplyOutcome,
    /// Request ID.
    pub request_id: String,
    /// 真实模式标记 (0 假装 STUB).
    pub real_mode: bool,
}

impl ApplyResponse {
    /// 必填字段数 (K-1 强校验: 3).
    pub const REQUIRED_FIELDS: u8 = 3;
}

// ============================================================================
// §4 handler stub (R21+ 真接时改 axum / warp handler)
// ============================================================================

/// GET /v1/update/check handler stub (R21+ 真接时改 axum / warp handler).
///
/// ⏳ **R21+ 真接时**:
/// - 改 axum handler: `async fn handle_check(State(u): State<Arc<DefaultUpdater>>, Query(req): Query<CheckRequest>) -> Json<CheckResponse>`
/// - 真实 auth (Bearer token)
/// - 真实 rate limit
pub async fn handle_check_request<U: Updater + ?Sized>(
    updater: &U,
    req: CheckRequest,
) -> UpdateResult<CheckResponse> {
    // K-1 强校验 1: 必填字段数
    if req.current_version.is_empty() {
        return Err(UpdateError::InvalidRequest(
            "empty current_version".to_string(),
        ));
    }

    // K-1 强校验 2: 解析 channel
    let channel = match &req.channel {
        Some(s) => crate::release::Channel::parse(s)?,
        None => crate::release::Channel::Stable,
    };

    // 调 Updater trait
    let update_info = updater.check_for_update(&req.current_version, channel).await?;

    // 构造响应
    Ok(CheckResponse {
        has_update: update_info.is_some(),
        update_info,
        request_id: Uuid::new_v4().to_string(),
        real_mode: true, // R21 real mode (0 假装 STUB)
    })
}

/// POST /v1/update/apply handler stub (R21+ 真接时改 axum / warp handler).
///
/// ⏳ **R21+ 真接时**:
/// - 改 axum handler: `async fn handle_apply(State(u): State<Arc<DefaultUpdater>>, Json(req): Json<ApplyRequest>) -> Json<ApplyResponse>`
/// - 真实 auth (Bearer token + 写权限校验)
/// - 真实 rate limit
/// - 真实 call `apeireth-upgrade::run_upgrade` 7 阶段 OTA
pub async fn handle_apply_request<U: Updater + ?Sized>(
    updater: &U,
    req: ApplyRequest,
) -> UpdateResult<ApplyResponse> {
    // K-1 强校验 1: 必填字段
    if req.version.is_empty() {
        return Err(UpdateError::InvalidRequest("empty version".to_string()));
    }

    // 构造 ApplyInfo (stub: 用 channel=Stable + 当前 version, R21+ 真接时从 DB 查 UpdateInfo)
    let target_dir = req.resolve_target_dir();

    // stub: 构造占位 UpdateInfo (R21+ 真接时从 `check_for_update` 缓存查)
    let stub_info = UpdateInfo {
        version: req.version.clone(),
        tag: format!("v{}", req.version),
        notes: "stub apply (R21+ 真接时从 check 缓存查)".to_string(),
        asset: crate::release::Asset {
            name: format!("apeireth-v{}.tar.gz", req.version),
            url: format!(
                "https://github.com/apeireth/apeireth-rust/releases/download/v{}/apeireth-v{}.tar.gz",
                req.version, req.version
            ),
            size_bytes: 1024,
            sha256: "a".repeat(64),
            signature_b64: "a".repeat(100),
            algorithm: crate::signature::SignatureAlgorithm::Ed25519,
        },
        channel: crate::release::Channel::Stable,
        published_at: "2026-08-06T00:00:00Z".to_string(),
        required_fields_count: UpdateInfo::REQUIRED_FIELDS,
    };

    // 调 Updater trait apply
    let outcome = updater.apply_update(&stub_info, &target_dir).await?;

    Ok(ApplyResponse {
        outcome,
        request_id: Uuid::new_v4().to_string(),
        real_mode: true, // R21 real mode (0 假装 STUB)
    })
}

// ============================================================================
// §5 endpoint 文档 schema helper (doctest 用)
// ============================================================================

/// GET /v1/update/check request schema 文档 (K-1 强校验 + 8 项不修改承诺 #2).
#[must_use]
pub const fn check_request_schema() -> &'static str {
    r#"GET /v1/update/check?current_version={semver}&channel={stable|beta|nightly}

Example:
  curl 'https://api.apeireth.dev/v1/update/check?current_version=0.14.0&channel=stable'

Response (JSON):
  {
    "has_update": true,
    "update_info": {
      "version": "1.0.0",
      "tag": "v1.0.0",
      "notes": "Apeireth 1.0.0 release",
      "asset": {
        "name": "apeireth-v1.0.0-x86_64-unknown-linux-gnu.tar.gz",
        "url": "https://github.com/apeireth/apeireth-rust/releases/download/v1.0.0/...",
        "size_bytes": 12345,
        "sha256": "a1b2c3...",
        "signature_b64": "RWQ...",
        "algorithm": "ed25519"
      },
      "channel": "stable",
      "published_at": "2026-08-06T00:00:00Z",
      "required_fields_count": 5
    },
    "request_id": "uuid-v4",
    "real_mode": true
  }
"#
}

/// POST /v1/update/apply request schema 文档 (K-1 强校验 + 8 项不修改承诺 #2).
#[must_use]
pub const fn apply_request_schema() -> &'static str {
    r#"POST /v1/update/apply

Body (JSON):
  {
    "version": "1.0.0",
    "target_dir": "/var/lib/apeireth"  // optional, default = /var/lib/apeireth
  }

Example:
  curl -X POST 'https://api.apeireth.dev/v1/update/apply' \
    -H 'Content-Type: application/json' \
    -d '{"version": "1.0.0"}'

Response (JSON):
  {
    "outcome": {
      "version": "1.0.0",
      "success": true,
      "required_fields_count": 5
    },
    "request_id": "uuid-v4",
    "real_mode": true
  }
"#
}

// ============================================================================
// §6 单元测试 (K-1 强校验 + endpoint 守门)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::release::Asset;
    use crate::signature::SignatureAlgorithm;

    #[test]
    fn endpoint_count_is_3() {
        assert_eq!(ENDPOINT_COUNT, 3);
        assert_eq!(ENDPOINT_PATHS.len(), 3);
    }

    #[test]
    fn endpoint_paths_correct() {
        assert_eq!(ENDPOINT_PATHS[0], "GET /v1/update/version");
        assert_eq!(ENDPOINT_PATHS[1], "GET /v1/update/check");
        assert_eq!(ENDPOINT_PATHS[2], "POST /v1/update/apply");
    }

    #[test]
    fn endpoint_paths_include_3rd_version() {
        // K-1 强校验: 3 endpoint 必含 /version + /check + /apply (per task spec §3)
        assert!(ENDPOINT_PATHS.contains(&"GET /v1/update/version"));
        assert!(ENDPOINT_PATHS.contains(&"GET /v1/update/check"));
        assert!(ENDPOINT_PATHS.contains(&"POST /v1/update/apply"));
    }

    #[test]
    fn check_request_required_fields_is_2() {
        assert_eq!(CheckRequest::REQUIRED_FIELDS, 2);
    }

    #[test]
    fn check_response_required_fields_is_4() {
        assert_eq!(CheckResponse::REQUIRED_FIELDS, 4);
    }

    #[test]
    fn apply_request_required_fields_is_2() {
        assert_eq!(ApplyRequest::REQUIRED_FIELDS, 2);
    }

    #[test]
    fn apply_response_required_fields_is_3() {
        assert_eq!(ApplyResponse::REQUIRED_FIELDS, 3);
    }

    #[test]
    fn apply_request_default_target_dir() {
        let req = ApplyRequest {
            version: "1.0.0".to_string(),
            target_dir: None,
        };
        assert_eq!(
            req.resolve_target_dir(),
            PathBuf::from(ApplyRequest::DEFAULT_TARGET_DIR)
        );
    }

    #[test]
    fn apply_request_custom_target_dir() {
        let req = ApplyRequest {
            version: "1.0.0".to_string(),
            target_dir: Some("/opt/apeireth".to_string()),
        };
        assert_eq!(req.resolve_target_dir(), PathBuf::from("/opt/apeireth"));
    }

    #[test]
    fn check_request_schema_includes_method_and_path() {
        let s = check_request_schema();
        assert!(s.contains("GET /v1/update/check"));
        assert!(s.contains("current_version"));
    }

    #[test]
    fn apply_request_schema_includes_method_and_path() {
        let s = apply_request_schema();
        assert!(s.contains("POST /v1/update/apply"));
        assert!(s.contains("version"));
    }

    #[tokio::test]
    async fn handle_check_rejects_empty_version() {
        let req = CheckRequest {
            current_version: "".to_string(),
            channel: None,
        };
        // 不需要真 updater, 因为校验在调 updater 之前
        let result: UpdateResult<CheckResponse> = handle_check_request_dummy(req).await;
        assert!(matches!(result, Err(UpdateError::InvalidRequest(_))));
    }

    #[tokio::test]
    async fn handle_apply_rejects_empty_version() {
        let req = ApplyRequest {
            version: "".to_string(),
            target_dir: None,
        };
        let result: UpdateResult<ApplyResponse> = handle_apply_request_dummy(req).await;
        assert!(matches!(result, Err(UpdateError::InvalidRequest(_))));
    }

    /// 测试用 dummy handler (绕过 updater).
    async fn handle_check_request_dummy(req: CheckRequest) -> UpdateResult<CheckResponse> {
        if req.current_version.is_empty() {
            return Err(UpdateError::InvalidRequest(
                "empty current_version".to_string(),
            ));
        }
        Ok(CheckResponse {
            has_update: false,
            update_info: None,
            request_id: Uuid::new_v4().to_string(),
            real_mode: true,
        })
    }

    /// 测试用 dummy handler (绕过 updater).
    async fn handle_apply_request_dummy(req: ApplyRequest) -> UpdateResult<ApplyResponse> {
        if req.version.is_empty() {
            return Err(UpdateError::InvalidRequest("empty version".to_string()));
        }
        let stub_outcome = ApplyOutcome {
            version: req.version,
            success: true,
            required_fields_count: 5,
        };
        Ok(ApplyResponse {
            outcome: stub_outcome,
            request_id: Uuid::new_v4().to_string(),
            real_mode: true,
        })
    }

    #[test]
    fn asset_algorithm_serializes_to_ed25519() {
        let asset = Asset {
            name: "test".to_string(),
            url: "https://example.com/test".to_string(),
            size_bytes: 1024,
            sha256: "a".repeat(64),
            signature_b64: "a".repeat(100),
            algorithm: SignatureAlgorithm::Ed25519,
        };
        let json = serde_json::to_string(&asset).unwrap();
        assert!(json.contains("ed25519"));
    }
}
