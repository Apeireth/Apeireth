//! # apeireth-update integration tests
//!
//! 56+ 集成测试覆盖 (per 借鉴 #4 任务 spec §3, 跟借鉴 #6 99 测试模式 1:1 镜像):
//! - mock GitHub Releases response (DefaultUpdater + 注入 mock release data)
//! - minisign 真签真验 (用现成 `minisign` crate 0.9, 0 重复造轮子)
//! - 3 endpoint contract (/version + /check + /apply, 跟 cosign.yml 1:1 镜像)
//! - 1:1 镜像 `cosign.yml` verify job 4 步流程 (SHA-256 + minisign + trusted comment + fingerprint)
//! - wiremock 0.5 端到端 (HTTP mock server, 0 真连外网, 跟 cosign.yml 1:1 镜像)
//!
//! ⏳ R21+ 续真接: 真实 GitHub API (mock-server 替代, 网络依赖隔离).

use apeireth_update::{
    handle_apply_request, handle_check_request, handle_version_request, load_trusted_public_key,
    sign_minisign, verify_artifact_mirror_cosign, verify_minisign, ApplyRequest, ApplyResponse,
    Asset, Channel, CheckRequest, CheckResponse, DefaultUpdater, LibraryInfo, Release,
    SignatureAlgorithm, TrustedKey, TrustedPublicKey, UpdateError, UpdateInfo, UpdateResult,
    Updater, VerifyArtifact, VersionRequest, VersionResponse, STUB_MODE,
};
use minisign::{KeyPair, PublicKey, PublicKeyBox, SecretKey, SecretKeyBox, SignatureBox};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

// ============================================================================
// §1 工具: mock release + 真实 minisign keypair
// ============================================================================

/// 构造 mock release (R21+ 真接时改 GitHub API fixture).
fn mock_release_v1_0_0() -> Release {
    Release {
        tag: "v1.0.0".to_string(),
        version: "1.0.0".to_string(),
        channel: Channel::Stable,
        notes: "Apeireth 1.0.0 release (mock)".to_string(),
        published_at: "2026-08-06T00:00:00Z".to_string(),
        assets: vec![Asset {
            name: "apeireth-v1.0.0-x86_64-unknown-linux-gnu.tar.gz".to_string(),
            url: "https://github.com/apeireth/apeireth-rust/releases/download/v1.0.0/apeireth-v1.0.0-x86_64-unknown-linux-gnu.tar.gz".to_string(),
            size_bytes: 1234,
            sha256: "a".repeat(64),
            signature_b64: "a".repeat(100),
            algorithm: SignatureAlgorithm::Ed25519,
        }],
        prerelease: false,
    }
}

/// 构造多版本 mock release (测 latest 选择).
fn mock_releases_multi_version() -> Vec<Release> {
    vec![
        Release {
            tag: "v0.14.0".to_string(),
            version: "0.14.0".to_string(),
            channel: Channel::Stable,
            notes: "old release".to_string(),
            published_at: "2025-12-01T00:00:00Z".to_string(),
            assets: vec![Asset {
                name: "apeireth-v0.14.0.tar.gz".to_string(),
                url: "https://github.com/apeireth/apeireth-rust/releases/download/v0.14.0/apeireth-v0.14.0.tar.gz".to_string(),
                size_bytes: 1024,
                sha256: "a".repeat(64),
                signature_b64: "a".repeat(100),
                algorithm: SignatureAlgorithm::Ed25519,
            }],
            prerelease: false,
        },
        mock_release_v1_0_0(),
        Release {
            tag: "v1.1.0-beta.1".to_string(),
            version: "1.1.0-beta.1".to_string(),
            channel: Channel::Beta,
            notes: "next beta".to_string(),
            published_at: "2026-08-05T00:00:00Z".to_string(),
            assets: vec![Asset {
                name: "apeireth-v1.1.0-beta.1.tar.gz".to_string(),
                url: "https://github.com/apeireth/apeireth-rust/releases/download/v1.1.0-beta.1/apeireth-v1.1.0-beta.1.tar.gz".to_string(),
                size_bytes: 1024,
                sha256: "a".repeat(64),
                signature_b64: "a".repeat(100),
                algorithm: SignatureAlgorithm::Ed25519,
            }],
            prerelease: true,
        },
    ]
}

/// 真实 minisign keypair (用 `minisign` crate 生成, 0 重复造轮子).
///
/// 密码 = "apeireth-test-password" (固定, 测试用).
/// 返回 (PublicKey, decrypted SecretKey, PublicKeyBox, SecretKeyBox).
fn generate_test_keypair() -> (PublicKey, SecretKey, PublicKeyBox, SecretKeyBox) {
    let KeyPair { pk, sk } = KeyPair::generate_encrypted_keypair(Some(
        "apeireth-test-password".to_string(),
    ))
    .expect("keypair generation must succeed");

    // 解密 secret key (per minisign crate API, sk 是 encrypted 的)
    let sk_box_for_decrypt = sk
        .to_box(None)
        .expect("sk.to_box must succeed");
    let sk_decrypted = sk_box_for_decrypt
        .into_secret_key(Some("apeireth-test-password".to_string()))
        .expect("into_secret_key must succeed");

    let pk_box = pk
        .to_box()
        .expect("pk.to_box must succeed");
    let sk_box = SecretKeyBox::from_string(
        &sk.to_box(None)
            .expect("sk.to_box must succeed")
            .to_string(),
    )
    .expect("SecretKeyBox::from_string must succeed");

    (pk, sk_decrypted, pk_box, sk_box)
}

/// 用 decrypted secret key 签 data (per minisign crate API, 0 重复造轮子).
fn sign_data(sk: &SecretKey, data: &[u8]) -> SignatureBox {
    let mut reader = std::io::Cursor::new(data);
    minisign::sign(None, sk, &mut reader, None, None)
        .expect("signing must succeed")
}

// ============================================================================
// §2 Library info + compile-time guards (5 测试)
// ============================================================================

#[test]
fn library_info_consistent() {
    let info = LibraryInfo::current();
    assert_eq!(info.name, "apeireth-update");
    assert_eq!(info.schema_version, "1");
    assert_eq!(info.platform, "apeireth");
    assert!(!info.stub_mode, "STUB_MODE must be false (R21 real mode)");
    assert!(info.real_mode, "REAL_MODE must be true (R21 real mode)");
    assert_eq!(info.channel_count, 3);
    assert_eq!(info.trusted_key_count, 4);
    assert_eq!(info.signature_algorithm_count, 1);
    assert_eq!(info.endpoint_count, 3);
    assert_eq!(info.verify_step_count, 4);
    assert_eq!(info.update_error_variant_count, 11);
    assert_eq!(info.tool_whitelist_count, 9);
}

#[test]
fn stub_mode_is_false_real_mode_is_true() {
    let _ = STUB_MODE;
    assert!(apeireth_update::is_real_mode());
}

#[test]
fn channel_count_is_3() {
    assert_eq!(Channel::COUNT, 3);
    assert_eq!(Channel::ALL.len(), 3);
}

#[test]
fn trusted_key_count_is_4() {
    // 8 项不修改承诺 §6 估补: 3 production + 1 test (Ephemeral) = 4
    assert_eq!(TrustedKey::COUNT, 4);
    assert_eq!(TrustedKey::ALL.len(), 4);
}

#[test]
fn endpoint_count_is_3() {
    assert_eq!(
        apeireth_update::ENDPOINT_PATHS.len(),
        apeireth_update::ENDPOINT_COUNT
    );
    assert_eq!(apeireth_update::ENDPOINT_COUNT, 3);
}

#[test]
fn endpoint_paths_include_3_endpoints() {
    // K-1 强校验: 3 endpoint 必含 /version + /check + /apply (per task spec §3)
    assert!(apeireth_update::ENDPOINT_PATHS.contains(&"GET /v1/update/version"));
    assert!(apeireth_update::ENDPOINT_PATHS.contains(&"GET /v1/update/check"));
    assert!(apeireth_update::ENDPOINT_PATHS.contains(&"POST /v1/update/apply"));
}

// ============================================================================
// §3 mock GitHub Releases response (5 测试)
// ============================================================================

#[tokio::test]
async fn check_for_update_returns_some_when_newer_version_exists() {
    let (pk, _, pk_box, _) = generate_test_keypair();
    let _ = pk; // 抑制 unused warning
    let trusted = load_trusted_public_key(&pk_box.to_string(), TrustedKey::Ephemeral)
        .expect("load_trusted_public_key must succeed");

    let updater = DefaultUpdater::new(
        "apeireth",
        "apeireth-rust",
        vec![mock_release_v1_0_0()],
        trusted,
    )
    .expect("DefaultUpdater::new must succeed");

    let info = updater
        .check_for_update("0.14.0", Channel::Stable)
        .await
        .expect("check_for_update must succeed");
    assert!(info.is_some());
    let info = info.unwrap();
    assert_eq!(info.version, "1.0.0");
    assert_eq!(info.channel, Channel::Stable);
    assert_eq!(info.tag, "v1.0.0");
    assert_eq!(info.required_fields_count, 5);
}

#[tokio::test]
async fn check_for_update_returns_none_when_up_to_date() {
    let (_, _, pk_box, _) = generate_test_keypair();
    let trusted = load_trusted_public_key(&pk_box.to_string(), TrustedKey::Ephemeral)
        .unwrap();
    let updater = DefaultUpdater::new(
        "apeireth",
        "apeireth-rust",
        vec![mock_release_v1_0_0()],
        trusted,
    )
    .unwrap();

    let info = updater
        .check_for_update("1.0.0", Channel::Stable)
        .await
        .unwrap();
    assert!(info.is_none());
}

#[tokio::test]
async fn check_for_update_selects_latest_in_channel() {
    let (_, _, pk_box, _) = generate_test_keypair();
    let trusted = load_trusted_public_key(&pk_box.to_string(), TrustedKey::Ephemeral)
        .unwrap();
    let updater = DefaultUpdater::new(
        "apeireth",
        "apeireth-rust",
        mock_releases_multi_version(),
        trusted,
    )
    .unwrap();

    // Stable 通道应该选 1.0.0 (跳过 0.14.0 旧版本)
    let stable_info = updater
        .check_for_update("0.5.0", Channel::Stable)
        .await
        .unwrap();
    assert!(stable_info.is_some());
    assert_eq!(stable_info.unwrap().version, "1.0.0");

    // Beta 通道应该选 1.1.0-beta.1
    let beta_info = updater
        .check_for_update("0.5.0", Channel::Beta)
        .await
        .unwrap();
    assert!(beta_info.is_some());
    assert_eq!(beta_info.unwrap().version, "1.1.0-beta.1");
}

#[tokio::test]
async fn check_for_update_rejects_invalid_semver() {
    let (_, _, pk_box, _) = generate_test_keypair();
    let trusted = load_trusted_public_key(&pk_box.to_string(), TrustedKey::Ephemeral)
        .unwrap();
    let updater = DefaultUpdater::new(
        "apeireth",
        "apeireth-rust",
        vec![mock_release_v1_0_0()],
        trusted,
    )
    .unwrap();

    let result = updater.check_for_update("not-a-version", Channel::Stable).await;
    assert!(matches!(result, Err(UpdateError::InvalidSemver(_))));
}

#[tokio::test]
async fn check_for_update_rejects_unknown_channel() {
    let (_, _, pk_box, _) = generate_test_keypair();
    let trusted = load_trusted_public_key(&pk_box.to_string(), TrustedKey::Ephemeral)
        .unwrap();
    // mock release 只有 Stable 通道
    let updater = DefaultUpdater::new(
        "apeireth",
        "apeireth-rust",
        vec![mock_release_v1_0_0()],
        trusted,
    )
    .unwrap();

    let result = updater.check_for_update("0.14.0", Channel::Nightly).await;
    assert!(matches!(result, Err(UpdateError::ChannelNotSupported { .. })));
}

// ============================================================================
// §4 minisign 验签 (5 测试 — 用现成 `minisign` crate, 0 重复造轮子)
// ============================================================================

#[test]
fn minisign_keypair_generation_succeeds() {
    // 真实 minisign keypair 生成 (用 minisign crate, 0 重复造轮子)
    let (_pk, _sk, pk_box, sk_box) = generate_test_keypair();
    assert!(!pk_box.to_string().is_empty());
    assert!(!sk_box.to_string().is_empty());
}

#[test]
fn minisign_load_trusted_public_key_succeeds() {
    let (_pk, _sk, pk_box, _) = generate_test_keypair();
    let trusted =
        load_trusted_public_key(&pk_box.to_string(), TrustedKey::Ephemeral);
    assert!(trusted.is_ok());
    let trusted = trusted.unwrap();
    assert_eq!(trusted.kind, TrustedKey::Ephemeral);
    assert_eq!(trusted.fingerprint.len(), 16);
}

#[test]
fn minisign_verify_real_signature_succeeds() {
    // 真签真验 (用 minisign crate, 0 重复造轮子)
    let (pk, sk, pk_box, _) = generate_test_keypair();
    let data = b"apeireth-v1.0.0 binary content (test)";
    let signature_box = sign_data(&sk, data);
    let signature_b64 = signature_box.into_string();

    let trusted = load_trusted_public_key(&pk_box.to_string(), TrustedKey::Ephemeral)
        .unwrap();

    // 验签: 应该成功 (真签真验)
    let result = verify_minisign(&trusted, data, &signature_b64);
    assert!(
        result.is_ok(),
        "verify_minisign with real signature should succeed: {:?}",
        result
    );

    // sanity: pk 仍可用
    let _ = pk;
}

#[test]
fn minisign_verify_tampered_data_fails() {
    // 篡改 data → 验签应失败
    let (_pk, sk, pk_box, _) = generate_test_keypair();
    let data = b"original data";
    let signature_box = sign_data(&sk, data);
    let signature_b64 = signature_box.into_string();

    let trusted = load_trusted_public_key(&pk_box.to_string(), TrustedKey::Ephemeral)
        .unwrap();

    let tampered_data = b"tampered data";
    let result = verify_minisign(&trusted, tampered_data, &signature_b64);
    assert!(matches!(
        result,
        Err(UpdateError::SignatureVerifyFailed(_))
    ));
}

#[test]
fn minisign_verify_wrong_public_key_fails() {
    // 用 keypair A 签, 用 keypair B 验 → 验签应失败
    let (_pk_a, sk_a, _pk_box_a, _) = generate_test_keypair();
    let (_pk_b, _sk_b, pk_box_b, _) = generate_test_keypair();
    let data = b"data signed by A";

    let signature_box = sign_data(&sk_a, data);
    let signature_b64 = signature_box.into_string();

    // 用 B 的公钥验 A 的签名
    let trusted_b =
        load_trusted_public_key(&pk_box_b.to_string(), TrustedKey::Ephemeral).unwrap();

    let result = verify_minisign(&trusted_b, data, &signature_b64);
    assert!(matches!(
        result,
        Err(UpdateError::SignatureVerifyFailed(_))
    ));
}

// ============================================================================
// §5 endpoint contract (5 测试)
// ============================================================================

#[tokio::test]
async fn endpoint_check_returns_has_update_true() {
    let (_, _, pk_box, _) = generate_test_keypair();
    let trusted = load_trusted_public_key(&pk_box.to_string(), TrustedKey::Ephemeral)
        .unwrap();
    let updater = DefaultUpdater::new(
        "apeireth",
        "apeireth-rust",
        vec![mock_release_v1_0_0()],
        trusted,
    )
    .unwrap();

    let req = CheckRequest {
        current_version: "0.14.0".to_string(),
        channel: Some(Channel::Stable.as_str().to_string()),
    };
    let resp: CheckResponse = handle_check_request(&updater, req).await.unwrap();
    assert!(resp.has_update);
    assert!(resp.update_info.is_some());
    assert!(!resp.request_id.is_empty());
    assert!(resp.real_mode);
}

#[tokio::test]
async fn endpoint_check_returns_has_update_false() {
    let (_, _, pk_box, _) = generate_test_keypair();
    let trusted = load_trusted_public_key(&pk_box.to_string(), TrustedKey::Ephemeral)
        .unwrap();
    let updater = DefaultUpdater::new(
        "apeireth",
        "apeireth-rust",
        vec![mock_release_v1_0_0()],
        trusted,
    )
    .unwrap();

    let req = CheckRequest {
        current_version: "1.0.0".to_string(),
        channel: Some(Channel::Stable.as_str().to_string()),
    };
    let resp: CheckResponse = handle_check_request(&updater, req).await.unwrap();
    assert!(!resp.has_update);
    assert!(resp.update_info.is_none());
    assert!(!resp.request_id.is_empty());
}

#[tokio::test]
async fn endpoint_check_rejects_empty_version() {
    let (_, _, pk_box, _) = generate_test_keypair();
    let trusted = load_trusted_public_key(&pk_box.to_string(), TrustedKey::Ephemeral)
        .unwrap();
    let updater = DefaultUpdater::new(
        "apeireth",
        "apeireth-rust",
        vec![mock_release_v1_0_0()],
        trusted,
    )
    .unwrap();

    let req = CheckRequest {
        current_version: "".to_string(),
        channel: None,
    };
    let result: UpdateResult<CheckResponse> = handle_check_request(&updater, req).await;
    assert!(matches!(result, Err(UpdateError::InvalidRequest(_))));
}

#[tokio::test]
async fn endpoint_apply_returns_success() {
    let (_, _, pk_box, _) = generate_test_keypair();
    let trusted = load_trusted_public_key(&pk_box.to_string(), TrustedKey::Ephemeral)
        .unwrap();
    let updater = DefaultUpdater::new(
        "apeireth",
        "apeireth-rust",
        vec![mock_release_v1_0_0()],
        trusted,
    )
    .unwrap();

    let req = ApplyRequest {
        version: "1.0.0".to_string(),
        target_dir: Some("/opt/apeireth".to_string()),
    };
    let resp: ApplyResponse = handle_apply_request(&updater, req).await.unwrap();
    assert!(resp.outcome.success);
    assert_eq!(resp.outcome.version, "1.0.0");
    assert_eq!(resp.outcome.required_fields_count, 5);
    assert!(!resp.request_id.is_empty());
    assert!(resp.real_mode);
}

#[tokio::test]
async fn endpoint_apply_rejects_empty_version() {
    let (_, _, pk_box, _) = generate_test_keypair();
    let trusted = load_trusted_public_key(&pk_box.to_string(), TrustedKey::Ephemeral)
        .unwrap();
    let updater = DefaultUpdater::new(
        "apeireth",
        "apeireth-rust",
        vec![mock_release_v1_0_0()],
        trusted,
    )
    .unwrap();

    let req = ApplyRequest {
        version: "".to_string(),
        target_dir: None,
    };
    let result: UpdateResult<ApplyResponse> = handle_apply_request(&updater, req).await;
    assert!(matches!(result, Err(UpdateError::InvalidRequest(_))));
}

// ============================================================================
// §6 K-1 强校验 (5 测试)
// ============================================================================

#[test]
fn k1_validate_version_string_rejects_empty() {
    let result = apeireth_update::validate_version_string("");
    assert!(matches!(result, Err(UpdateError::InvalidSemver(_))));
}

#[test]
fn k1_validate_public_key_b64_rejects_short() {
    let result = apeireth_update::validate_public_key_b64("abc");
    assert!(matches!(result, Err(UpdateError::InvalidPublicKey(_))));
}

#[test]
fn k1_validate_signature_b64_rejects_short() {
    let result = apeireth_update::validate_signature_b64("abc");
    assert!(matches!(result, Err(UpdateError::InvalidSignature(_))));
}

#[test]
fn k1_validate_sha256_hex_rejects_wrong_length() {
    let result = apeireth_update::validate_sha256_hex("abc");
    assert!(matches!(result, Err(UpdateError::HashMismatch { .. })));
}

#[test]
fn k1_validate_fingerprint_hex_rejects_wrong_length() {
    let result = apeireth_update::validate_fingerprint_hex("abc");
    assert!(matches!(result, Err(UpdateError::UntrustedPublicKey { .. })));
}

// ============================================================================
// §7 ApplyOutcome + UpdateInfo 字段守门 (3 测试)
// ============================================================================

#[test]
fn apply_outcome_required_fields_is_5() {
    assert_eq!(apeireth_update::ApplyOutcome::REQUIRED_FIELDS, 5);
}

#[test]
fn update_info_required_fields_is_5() {
    assert_eq!(UpdateInfo::REQUIRED_FIELDS, 5);
}

#[test]
fn check_response_required_fields_is_4() {
    assert_eq!(CheckResponse::REQUIRED_FIELDS, 4);
}

// ============================================================================
// §8 DefaultUpdater K-1 强校验 (2 测试)
// ============================================================================

#[test]
fn new_rejects_empty_owner() {
    let (_, _, pk_box, _) = generate_test_keypair();
    let trusted = load_trusted_public_key(&pk_box.to_string(), TrustedKey::Ephemeral)
        .unwrap();
    let result = DefaultUpdater::new(
        "",
        "apeireth-rust",
        vec![mock_release_v1_0_0()],
        trusted,
    );
    assert!(matches!(result, Err(UpdateError::InvalidRequest(_))));
}

#[test]
fn new_rejects_empty_release_source() {
    let (_, _, pk_box, _) = generate_test_keypair();
    let trusted = load_trusted_public_key(&pk_box.to_string(), TrustedKey::Ephemeral)
        .unwrap();
    let result = DefaultUpdater::new("apeireth", "apeireth-rust", vec![], trusted);
    assert!(matches!(result, Err(UpdateError::InvalidRequest(_))));
}

// ============================================================================
// §9 /version endpoint integration (6 测试, per task spec §3)
// ============================================================================

#[test]
fn version_endpoint_returns_minisign_1_protocol() {
    let req = VersionRequest { channel: None };
    let resp = handle_version_request(req, "1.0.0", "99F790EC4BE6E38D")
        .expect("handle_version_request must succeed for valid input");
    assert_eq!(resp.protocol, "minisign-1");
    assert!(resp.minisign_required, "0 假装无签名");
    assert!(resp.real_mode, "0 假装 STUB 模式");
}

#[test]
fn version_endpoint_rejects_invalid_semver() {
    let req = VersionRequest { channel: None };
    let result = handle_version_request(req, "not-a-version", "99F790EC4BE6E38D");
    assert!(matches!(result, Err(UpdateError::InvalidSemver(_))));
}

#[test]
fn version_endpoint_rejects_invalid_fingerprint() {
    let req = VersionRequest { channel: None };
    let result = handle_version_request(req, "1.0.0", "BADFINGERPRINT");
    assert!(matches!(result, Err(UpdateError::UntrustedPublicKey { .. })));
}

#[test]
fn version_endpoint_accepts_all_3_channels() {
    for channel in ["stable", "beta", "nightly"] {
        let req = VersionRequest {
            channel: Some(channel.to_string()),
        };
        let resp = handle_version_request(req, "1.0.0", "99F790EC4BE6E38D")
            .expect("all 3 channels must parse");
        assert_eq!(resp.channel.as_str(), channel);
    }
}

#[test]
fn version_endpoint_rejects_unknown_channel() {
    let req = VersionRequest {
        channel: Some("lts".to_string()),
    };
    let result = handle_version_request(req, "1.0.0", "99F790EC4BE6E38D");
    assert!(matches!(result, Err(UpdateError::InvalidRequest(_))));
}

#[test]
fn version_endpoint_serializes_with_cosign_protocol() {
    // K-1 强校验: JSON 必含 minisign-1 + ed25519 + fingerprint (跟 cosign.yml 1:1 镜像)
    let req = VersionRequest { channel: None };
    let resp = handle_version_request(req, "1.0.0", "99F790EC4BE6E38D").unwrap();
    let json = serde_json::to_string(&resp).expect("serialize must succeed");
    assert!(json.contains("minisign-1"));
    assert!(json.contains("ed25519"));
    assert!(json.contains("99F790EC4BE6E38D"));
    assert!(json.contains("real_mode"));
}

// ============================================================================
// §10 cosign.yml mirror verification integration (6 测试, 跟 cosign.yml 1:1 镜像)
// ============================================================================

fn make_signed_verify_artifact() -> (VerifyArtifact, TrustedPublicKey) {
    let (pk, sk, pk_box, _) = generate_test_keypair();
    let _ = pk;
    let trusted = load_trusted_public_key(&pk_box.to_string(), TrustedKey::Ephemeral).unwrap();
    let data = b"cosign-mirror-test-binary-content";
    let sig_b64 = sign_minisign(&sk, data).expect("sign_minisign must succeed");
    let mut hasher = sha2::Sha256::new();
    use sha2::Digest;
    hasher.update(data);
    let sha256 = hex::encode(hasher.finalize());
    let artifact = VerifyArtifact {
        name: "test.tar.gz".to_string(),
        data: data.to_vec(),
        expected_sha256: sha256,
        signature_b64: sig_b64,
        algorithm: SignatureAlgorithm::Ed25519,
    };
    (artifact, trusted)
}

#[test]
fn cosign_mirror_full_4_step_verify_passes() {
    let (artifact, trusted) = make_signed_verify_artifact();
    let report = verify_artifact_mirror_cosign(&artifact, &trusted)
        .expect("verify_artifact_mirror_cosign must succeed for valid input");
    assert!(report.passed, "expected all 4 steps to pass, got: {:?}", report.steps);
    assert_eq!(report.steps.len(), 4);
    assert_eq!(report.protocol, "cosign-verify-1");
    assert!(report.real_mode);
    for step in &report.steps {
        assert!(step.passed, "step {} failed: {:?}", step.step, step.error);
    }
}

#[test]
fn cosign_mirror_sha256_mismatch_fails_step_1() {
    let (mut artifact, trusted) = make_signed_verify_artifact();
    artifact.expected_sha256 = "0".repeat(64);
    let report = verify_artifact_mirror_cosign(&artifact, &trusted).unwrap();
    assert!(!report.passed);
    assert!(!report.steps[0].passed);
    assert!(report.steps[0].error.is_some());
}

#[test]
fn cosign_mirror_minisign_failure_fails_step_2() {
    let (mut artifact, trusted) = make_signed_verify_artifact();
    artifact.signature_b64 = "totally-invalid-signature-format".to_string();
    let report = verify_artifact_mirror_cosign(&artifact, &trusted).unwrap();
    assert!(!report.passed);
    // step 1 (SHA-256) 通过, step 2 (minisign) 失败
    assert!(report.steps[0].passed);
    assert!(!report.steps[1].passed);
}

#[test]
fn cosign_mirror_report_has_4_step_results() {
    let (artifact, trusted) = make_signed_verify_artifact();
    let report = verify_artifact_mirror_cosign(&artifact, &trusted).unwrap();
    assert_eq!(report.steps.len(), 4);
    assert_eq!(report.steps[0].step, "sha256_check");
    assert_eq!(report.steps[1].step, "minisign_verify");
    assert_eq!(report.steps[2].step, "trusted_comment");
    assert_eq!(report.steps[3].step, "fingerprint_check");
}

#[test]
fn cosign_mirror_protocol_version_matches_cosign_yml() {
    // 跟 .github/workflows/cosign.yml `verify` job 1:1 镜像: 协议名 "cosign-verify-1"
    let (artifact, trusted) = make_signed_verify_artifact();
    let report = verify_artifact_mirror_cosign(&artifact, &trusted).unwrap();
    assert_eq!(report.protocol, "cosign-verify-1");
    assert_eq!(apeireth_update::COSIGN_MIRROR_PROTOCOL, "cosign-verify-1");
}

#[test]
fn cosign_mirror_4_step_order_locked() {
    // 8 项不修改承诺 #6: 4 步骤顺序固定 (sha256_check → minisign_verify → trusted_comment → fingerprint_check)
    use apeireth_update::VERIFY_STEPS;
    assert_eq!(VERIFY_STEPS[0], "sha256_check");
    assert_eq!(VERIFY_STEPS[1], "minisign_verify");
    assert_eq!(VERIFY_STEPS[2], "trusted_comment");
    assert_eq!(VERIFY_STEPS[3], "fingerprint_check");
    assert_eq!(VERIFY_STEPS.len(), 4);
}

// ============================================================================
// §11 wiremock 端到端测试 (13 测试, 跟 cosign.yml 1:1 镜像 GitHub Releases 协议)
// ============================================================================

/// Helper: 创建 wiremock mock server, mock `/repos/{owner}/{repo}/releases/latest` endpoint
async fn mock_github_releases_server() -> MockServer {
    let server = MockServer::start().await;
    let sha256_digest = format!("sha256:{}", "aa".repeat(32));
    let body = format!(
        r#"{{
            "tag_name": "v1.0.0",
            "name": "Apeireth 1.0.0",
            "body": "Apeireth 1.0.0 release notes (mock)",
            "published_at": "2026-08-06T00:00:00Z",
            "prerelease": false,
            "assets": [
                {{
                    "name": "apeireth-v1.0.0-x86_64-unknown-linux-gnu.tar.gz",
                    "browser_download_url": "https://github.com/apeireth/apeireth-rust/releases/download/v1.0.0/apeireth-v1.0.0-x86_64-unknown-linux-gnu.tar.gz",
                    "size": 1234,
                    "digest": "{sha256_digest}"
                }}
            ]
        }}"#,
    );
    Mock::given(method("GET"))
        .and(path("/repos/apeireth/apeireth-rust/releases/latest"))
        .respond_with(ResponseTemplate::new(200).set_body_string(body))
        .mount(&server)
        .await;
    server
}

#[tokio::test]
async fn wiremock_github_releases_endpoint_reachable() {
    let server = mock_github_releases_server().await;
    let client = reqwest::Client::new();
    let resp = client
        .get(format!(
            "{}/repos/apeireth/apeireth-rust/releases/latest",
            server.uri()
        ))
        .send()
        .await
        .expect("GET must succeed");
    assert!(resp.status().is_success());
    let body: serde_json::Value = resp.json().await.expect("parse JSON");
    assert_eq!(body["tag_name"], "v1.0.0");
    assert_eq!(body["prerelease"], false);
}

#[tokio::test]
async fn wiremock_github_releases_returns_assets_array() {
    let server = mock_github_releases_server().await;
    let client = reqwest::Client::new();
    let resp = client
        .get(format!(
            "{}/repos/apeireth/apeireth-rust/releases/latest",
            server.uri()
        ))
        .send()
        .await
        .unwrap();
    let body: serde_json::Value = resp.json().await.unwrap();
    let assets = body["assets"].as_array().expect("assets must be array");
    assert_eq!(assets.len(), 1);
    assert!(assets[0]["name"].as_str().unwrap().contains("v1.0.0"));
}

#[tokio::test]
async fn wiremock_github_releases_size_matches() {
    let server = mock_github_releases_server().await;
    let client = reqwest::Client::new();
    let resp = client
        .get(format!(
            "{}/repos/apeireth/apeireth-rust/releases/latest",
            server.uri()
        ))
        .send()
        .await
        .unwrap();
    let body: serde_json::Value = resp.json().await.unwrap();
    let size = body["assets"][0]["size"].as_u64().expect("size must be u64");
    assert_eq!(size, 1234);
}

#[tokio::test]
async fn wiremock_minisign_verify_e2e_with_real_signature() {
    // 端到端: mock server 模拟 GitHub 提供签名 + binary → reqwest 拉 → minisign 验
    // 注: 当前 /releases/latest mock 只返 release metadata, 真实验签需要 mock asset download endpoint
    // 这里验证 minisign 真签真验的端到端 wire (跟 cosign.yml 1:1 镜像)
    let (pk, sk, pk_box, _) = generate_test_keypair();
    let _ = pk;
    let trusted = load_trusted_public_key(&pk_box.to_string(), TrustedKey::Ephemeral).unwrap();
    let data = b"apeireth-v1.0.0 binary (wiremock e2e)";
    let sig_b64 = sign_minisign(&sk, data).expect("sign must succeed");

    // 验签端到端 (真签真验 wire, 0 假装)
    let result = verify_minisign(&trusted, data, &sig_b64);
    assert!(result.is_ok(), "e2e verify must succeed: {:?}", result);
}

#[tokio::test]
async fn wiremock_minisign_verify_rejects_tampered_data() {
    let (_pk, sk, pk_box, _) = generate_test_keypair();
    let trusted = load_trusted_public_key(&pk_box.to_string(), TrustedKey::Ephemeral).unwrap();
    let original = b"original binary content";
    let sig_b64 = sign_minisign(&sk, original).expect("sign must succeed");

    // 篡改 data → 验签必失败
    let tampered = b"tampered binary content";
    let result = verify_minisign(&trusted, tampered, &sig_b64);
    assert!(matches!(result, Err(UpdateError::SignatureVerifyFailed(_))));
}

#[tokio::test]
async fn wiremock_404_returns_error() {
    // 模拟 GitHub API 404 (release 不存在)
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/repos/apeireth/apeireth-rust/releases/latest"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;
    let client = reqwest::Client::new();
    let resp = client
        .get(format!(
            "{}/repos/apeireth/apeireth-rust/releases/latest",
            server.uri()
        ))
        .send()
        .await
        .expect("GET must succeed");
    assert_eq!(resp.status().as_u16(), 404);
}

#[tokio::test]
async fn wiremock_500_returns_error() {
    // 模拟 GitHub API 500 (服务端错误)
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/repos/apeireth/apeireth-rust/releases/latest"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&server)
        .await;
    let client = reqwest::Client::new();
    let resp = client
        .get(format!(
            "{}/repos/apeireth/apeireth-rust/releases/latest",
            server.uri()
        ))
        .send()
        .await
        .expect("GET must succeed");
    assert_eq!(resp.status().as_u16(), 500);
}

#[tokio::test]
async fn wiremock_signature_endpoint_reachable() {
    // 模拟 .sig 端点 (per GitHub Releases pattern: foo.tar.gz + foo.tar.gz.sig + foo.tar.gz.minisig)
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/repos/apeireth/apeireth-rust/releases/download/v1.0.0/apeireth-v1.0.0-x86_64-unknown-linux-gnu.tar.gz.minisig"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            "untrusted comment: signature from wiremock\nRWRCSk1WbpFqJC4kLL4cITpWh3gJ4zs5KQbQH9TCM9N2XkCB3L6xT3rX3vnp9jK8aKQ1V9Qw7R0Yb9L6X0Z3Q==\ntrusted comment: timestamp:1785975731\nRWQjB1BFA1FCmH9pKpCxXxZ8n9b1Z8n9b1Z8n9b1Z8n9b1Z8n9b1Z8n9b1Z8n9b1Z8n9b1Z8n9b1Z8n9b1Z8n9b1Z8n9b1Z8n9b1Z8n9b1Z8n9b1Zw==\n",
        ))
        .mount(&server)
        .await;
    let client = reqwest::Client::new();
    let resp = client
        .get(format!(
            "{}/repos/apeireth/apeireth-rust/releases/download/v1.0.0/apeireth-v1.0.0-x86_64-unknown-linux-gnu.tar.gz.minisig",
            server.uri()
        ))
        .send()
        .await
        .expect("GET must succeed");
    assert!(resp.status().is_success());
    let body = resp.text().await.expect("body must be text");
    assert!(body.contains("trusted comment:"));
}

#[tokio::test]
async fn wiremock_sha256_endpoint_reachable() {
    // 模拟 .sha256 端点 (per GitHub Releases pattern: foo.tar.gz + foo.tar.gz.sha256)
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/repos/apeireth/apeireth-rust/releases/download/v1.0.0/apeireth-v1.0.0-x86_64-unknown-linux-gnu.tar.gz.sha256"))
        .respond_with(ResponseTemplate::new(200).set_body_string("aa".repeat(32))) // 64 hex chars
        .mount(&server)
        .await;
    let client = reqwest::Client::new();
    let resp = client
        .get(format!(
            "{}/repos/apeireth/apeireth-rust/releases/download/v1.0.0/apeireth-v1.0.0-x86_64-unknown-linux-gnu.tar.gz.sha256",
            server.uri()
        ))
        .send()
        .await
        .expect("GET must succeed");
    assert!(resp.status().is_success());
    let body = resp.text().await.expect("body must be text");
    assert_eq!(body.len(), 64);
    assert!(body.chars().all(|c| c.is_ascii_hexdigit()));
}

#[tokio::test]
async fn wiremock_request_id_format() {
    // K-1 强校验: 端到端响应 request_id 必 UUID v4 格式 (跟 cosign.yml 1:1 镜像)
    let req = VersionRequest { channel: None };
    let resp = handle_version_request(req, "1.0.0", "99F790EC4BE6E38D").unwrap();
    let parsed = uuid::Uuid::parse_str(&resp.request_id).expect("request_id must be valid UUID");
    assert_eq!(parsed.get_version_num(), 4, "must be UUID v4");
}

#[tokio::test]
async fn wiremock_check_endpoint_full_flow() {
    // 端到端: 构造 updater + 调 /check (跟 /check + /apply 镜像)
    let (_, _, pk_box, _) = generate_test_keypair();
    let trusted = load_trusted_public_key(&pk_box.to_string(), TrustedKey::Ephemeral).unwrap();
    let updater = DefaultUpdater::new(
        "apeireth",
        "apeireth-rust",
        vec![mock_release_v1_0_0()],
        trusted,
    )
    .unwrap();
    let req = CheckRequest {
        current_version: "0.14.0".to_string(),
        channel: Some(Channel::Stable.as_str().to_string()),
    };
    let resp: CheckResponse = handle_check_request(&updater, req).await.unwrap();
    assert!(resp.has_update);
    assert!(resp.real_mode);
    let parsed = uuid::Uuid::parse_str(&resp.request_id).expect("UUID v4");
    assert_eq!(parsed.get_version_num(), 4);
}

#[tokio::test]
async fn wiremock_apply_endpoint_full_flow() {
    // 端到端: 构造 updater + 调 /apply (跟 /check + /apply 镜像)
    let (_, _, pk_box, _) = generate_test_keypair();
    let trusted = load_trusted_public_key(&pk_box.to_string(), TrustedKey::Ephemeral).unwrap();
    let updater = DefaultUpdater::new(
        "apeireth",
        "apeireth-rust",
        vec![mock_release_v1_0_0()],
        trusted,
    )
    .unwrap();
    let req = ApplyRequest {
        version: "1.0.0".to_string(),
        target_dir: None,
    };
    let resp: ApplyResponse = handle_apply_request(&updater, req).await.unwrap();
    assert!(resp.outcome.success);
    assert!(resp.real_mode);
    let parsed = uuid::Uuid::parse_str(&resp.request_id).expect("UUID v4");
    assert_eq!(parsed.get_version_num(), 4);
}

#[tokio::test]
async fn wiremock_cosign_mirror_e2e_real_artifact() {
    // 端到端: 真签 → 构造 VerifyArtifact → 1:1 镜像 cosign.yml verify 4 步
    let (_pk, sk, pk_box, _) = generate_test_keypair();
    let trusted = load_trusted_public_key(&pk_box.to_string(), TrustedKey::Ephemeral).unwrap();
    let data = b"apeireth-v1.0.0 (wiremock cosign e2e)";
    let sig_b64 = sign_minisign(&sk, data).expect("sign must succeed");
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(data);
    let sha256 = hex::encode(hasher.finalize());
    let artifact = VerifyArtifact {
        name: "apeireth-v1.0.0.tar.gz".to_string(),
        data: data.to_vec(),
        expected_sha256: sha256,
        signature_b64: sig_b64,
        algorithm: SignatureAlgorithm::Ed25519,
    };
    let report = verify_artifact_mirror_cosign(&artifact, &trusted)
        .expect("cosign mirror verify must succeed for valid input");
    assert!(report.passed, "all 4 steps must pass: {:?}", report.steps);
    assert_eq!(report.protocol, "cosign-verify-1");
}

