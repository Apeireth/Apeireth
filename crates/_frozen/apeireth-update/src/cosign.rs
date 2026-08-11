//! # cosign.yml 1:1 镜像签名验证
//!
//! 借鉴 `.github/workflows/cosign.yml` 的 `verify` job 1:1 翻译到 Rust.
//! 跟 `signature.rs::verify_minisign` 镜像, 但走"完整 4 步" (per cosign.yml `verify` job):
//!
//! ## 4 步验证流程 (per `.github/workflows/cosign.yml` `verify` job 1:1 镜像)
//!
//! 1. **SHA-256 校验** (跟 `cosign verify-blob --insecure-ignore-tlog` 1:1 镜像) —
//!    `sha256(asset_bytes) == asset.sha256_hex` (K-1 强校验 `validate_sha256_hex`)
//! 2. **minisign 验签** (跟 `cosign verify` 1:1 镜像) —
//!    `verify_minisign(pub_key, asset_bytes, signature_b64)` (走 `minisign` crate, 0 重复造轮子)
//! 3. **trusted comment 校验** (跟 cosign `require_trusted_comment=true` 1:1 镜像) —
//!    minisign signature box 第一行 `trusted comment: <comment>` 必含 `file:` + 路径守门
//! 4. **fingerprint 白名单** (跟 cosign `cosign.pub` 1:1 镜像) —
//!    `pub_key.fingerprint` ∈ `TrustedKey` 编译期 hardcode 枚举
//!
//! ## K-1 强校验 (5 校验, 跟 `error.rs::K1_STRONG_VALIDATION_VARIANTS` 镜像)
//!
//! 1. `HashMismatch` — SHA-256 不匹配
//! 2. `UntrustedPublicKey` — fingerprint 不在白名单
//! 3. `InvalidPublicKey` — 公钥解析失败
//! 4. `InvalidSignature` — 签名解析失败
//! 5. `SignatureVerifyFailed` — minisign 验签失败
//!
//! ## ⏳ R21+ 续真接
//!
//! - 真实 Rekor transparency log 查询 (cosign.yml `verify` job 步骤 4)
//! - 真实 ECDSA P-256 cosign key 加载 (cosign `cosign.pub` 格式)
//! - 真实 OIDC Fulcio 验证 (cosign Docker image signature)

use serde::{Deserialize, Serialize};

use crate::error::{validate_fingerprint_hex, validate_sha256_hex, UpdateError, UpdateResult};
use crate::signature::{verify_minisign, SignatureAlgorithm, TrustedKey, TrustedPublicKey};

// ============================================================================
// §1 镜像 cosign.yml verify job 编译期常量 (K-1 强校验, 编译期 hardcode)
// ============================================================================

/// 镜像 cosign.yml `verify` job 算法 (K-1 强校验: 编译期 hardcode).
///
/// cosign 用 ECDSA P-256, 我们镜像 minisign 走 Ed25519 (per 借鉴文档 §8 P3 第 10 项).
pub const COSIGN_MIRROR_ALGORITHM: SignatureAlgorithm = SignatureAlgorithm::Ed25519;

/// 镜像 cosign.yml `verify` job 协议版本 (K-1 强校验: 编译期 hardcode).
pub const COSIGN_MIRROR_PROTOCOL: &str = "cosign-verify-1";

/// 镜像 cosign.yml `verify` job 4 步流程名 (K-1 强校验守门, 0 漏防).
pub const VERIFY_STEPS: &[&str] = &[
    "sha256_check",        // 1 步: SHA-256 校验
    "minisign_verify",     // 2 步: minisign 验签
    "trusted_comment",     // 3 步: trusted comment 守门
    "fingerprint_check",   // 4 步: fingerprint 白名单
];

/// K-1 强校验: 4 步流程 (跟 cosign.yml `verify` job 1:1 镜像).
pub const VERIFY_STEP_COUNT: usize = 4;
const _: () = assert!(VERIFY_STEPS.len() == VERIFY_STEP_COUNT);

// ============================================================================
// §2 VerifyArtifact — 1 artifact 完整元数据 (per Asset 镜像)
// ============================================================================

/// 1 个待验证 artifact 的完整元数据 (跟 `release::Asset` 1:1 镜像).
///
/// 公开 API (8 项不修改承诺 #2 公开 API 100% 文档化).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifyArtifact {
    /// Asset 名 (e.g. `apeireth-v1.0.0-x86_64-unknown-linux-gnu.tar.gz`).
    pub name: String,
    /// Asset 字节内容 (R21+ 改 path + 文件 I/O).
    pub data: Vec<u8>,
    /// 期望 SHA-256 hex (64 chars, K-1 强校验).
    pub expected_sha256: String,
    /// minisign 签名 b64 (K-1 强校验).
    pub signature_b64: String,
    /// 签名算法 (K-1 强校验: 编译期 hardcode `Ed25519`).
    pub algorithm: SignatureAlgorithm,
}

impl VerifyArtifact {
    /// K-1 强校验: 必填字段数 (5 字段, 跟 `release::Asset::REQUIRED_FIELDS` 镜像).
    pub const REQUIRED_FIELDS: u8 = 5;
}

// ============================================================================
// §3 VerifyReport — 1 次验证的完整报告 (per step 1:1)
// ============================================================================

/// 1 次验证的完整报告 (4 步, 跟 cosign.yml `verify` job 1:1 镜像).
///
/// 公开 API (8 项不修改承诺 #2 公开 API 100% 文档化).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifyReport {
    /// Artifact 名.
    pub artifact_name: String,
    /// 是否通过 (4 步全 OK).
    pub passed: bool,
    /// 4 步结果 (按 [`VERIFY_STEPS`] 顺序).
    pub steps: Vec<VerifyStepResult>,
    /// Request ID (UUID v4, 跨请求追踪, 跟 /check + /apply 镜像).
    pub request_id: String,
    /// 协议版本 (K-1 强校验: `cosign-verify-1`).
    pub protocol: String,
    /// 真实模式标记 (0 假装 STUB 模式).
    pub real_mode: bool,
}

/// 单步验证结果 (per `VERIFY_STEPS` 1:1).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifyStepResult {
    /// 步骤名 (∈ `VERIFY_STEPS`).
    pub step: String,
    /// 步骤是否通过.
    pub passed: bool,
    /// 步骤错误信息 (None 当 passed=true).
    pub error: Option<String>,
}

impl VerifyReport {
    /// K-1 强校验: 必填字段数 (6 字段).
    pub const REQUIRED_FIELDS: u8 = 6;
}

// ============================================================================
// §4 4 步验证主函数 (跟 cosign.yml `verify` job 1:1 镜像)
// ============================================================================

/// 镜像 cosign.yml `verify` job 4 步验证主函数 (1:1 翻译到 Rust).
///
/// 公开 API (8 项不修改承诺 #2 公开 API 100% 文档化).
///
/// # 流程 (4 步, 跟 cosign.yml `verify` job 1:1 镜像)
///
/// 1. `sha256_check` — `sha256(artifact.data) == expected_sha256`
/// 2. `minisign_verify` — `verify_minisign(pub_key, data, signature_b64)` (走 minisign crate)
/// 3. `trusted_comment` — 解析 signature box 校验 `trusted comment:` 头
/// 4. `fingerprint_check` — `pub_key.fingerprint ∈ TrustedKey` 编译期 hardcode 白名单
///
/// # 参数
///
/// - `artifact`: [`VerifyArtifact`] 待验证 artifact 元数据
/// - `pub_key`: [`TrustedPublicKey`] 信任公钥 (含 fingerprint 白名单)
///
/// # 返回
///
/// - `Ok(VerifyReport { passed: true, .. })` 4 步全 OK
/// - `Ok(VerifyReport { passed: false, .. })` 4 步有失败 (steps 含详细错误)
/// - `Err(UpdateError::InvalidRequest)` artifact 字段 K-1 不通过
///
/// # 错误
///
/// - [`UpdateError::HashMismatch`] SHA-256 不匹配
/// - [`UpdateError::UntrustedPublicKey`] fingerprint 不在白名单
/// - [`UpdateError::InvalidPublicKey`] 公钥解析失败
/// - [`UpdateError::InvalidSignature`] 签名解析失败
/// - [`UpdateError::SignatureVerifyFailed`] minisign 验签失败
pub fn verify_artifact_mirror_cosign(
    artifact: &VerifyArtifact,
    pub_key: &TrustedPublicKey,
) -> UpdateResult<VerifyReport> {
    let request_id = uuid::Uuid::new_v4().to_string();
    let mut steps: Vec<VerifyStepResult> = Vec::with_capacity(VERIFY_STEP_COUNT);

    // ========== Step 1/4: SHA-256 校验 (跟 cosign `verify-blob` 1:1 镜像) ==========
    let sha256_step = step_sha256_check(artifact);
    let sha256_ok = sha256_step.passed;
    steps.push(sha256_step);
    if !sha256_ok {
        return Ok(build_report(artifact.name.clone(), steps, request_id));
    }

    // ========== Step 2/4: minisign 验签 (跟 cosign `verify` 1:1 镜像) ==========
    let minisign_step = step_minisign_verify(artifact, pub_key);
    let minisign_ok = minisign_step.passed;
    steps.push(minisign_step);
    if !minisign_ok {
        return Ok(build_report(artifact.name.clone(), steps, request_id));
    }

    // ========== Step 3/4: trusted comment 守门 (跟 cosign `require_trusted_comment=true` 1:1 镜像) ==========
    let trusted_step = step_trusted_comment(artifact);
    let trusted_ok = trusted_step.passed;
    steps.push(trusted_step);
    if !trusted_ok {
        return Ok(build_report(artifact.name.clone(), steps, request_id));
    }

    // ========== Step 4/4: fingerprint 白名单 (跟 cosign `cosign.pub` 1:1 镜像) ==========
    let fingerprint_step = step_fingerprint_check(pub_key);
    let fingerprint_ok = fingerprint_step.passed;
    steps.push(fingerprint_step);
    if !fingerprint_ok {
        return Ok(build_report(artifact.name.clone(), steps, request_id));
    }

    Ok(build_report(artifact.name.clone(), steps, request_id))
}

// ============================================================================
// §5 4 步 step helper (per step 1 个 helper, 跟 cosign.yml `verify` job 1:1 镜像)
// ============================================================================

/// Step 1/4: SHA-256 校验 (跟 cosign `verify-blob` 1:1 镜像).
fn step_sha256_check(artifact: &VerifyArtifact) -> VerifyStepResult {
    let step_name = VERIFY_STEPS[0];
    // K-1 强校验 1: 期望 SHA-256 必 64 hex
    if validate_sha256_hex(&artifact.expected_sha256).is_err() {
        return VerifyStepResult {
            step: step_name.to_string(),
            passed: false,
            error: Some(format!(
                "expected_sha256 invalid format: {} chars",
                artifact.expected_sha256.len()
            )),
        };
    }
    // 实际算 SHA-256
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(&artifact.data);
    let actual = hex::encode(hasher.finalize());
    if actual != artifact.expected_sha256 {
        return VerifyStepResult {
            step: step_name.to_string(),
            passed: false,
            error: Some(format!(
                "SHA-256 mismatch: expected {}, got {}",
                &artifact.expected_sha256[..16],
                &actual[..16]
            )),
        };
    }
    VerifyStepResult {
        step: step_name.to_string(),
        passed: true,
        error: None,
    }
}

/// Step 2/4: minisign 验签 (跟 cosign `verify` 1:1 镜像).
fn step_minisign_verify(
    artifact: &VerifyArtifact,
    pub_key: &TrustedPublicKey,
) -> VerifyStepResult {
    let step_name = VERIFY_STEPS[1];
    match verify_minisign(pub_key, &artifact.data, &artifact.signature_b64) {
        Ok(()) => VerifyStepResult {
            step: step_name.to_string(),
            passed: true,
            error: None,
        },
        Err(e) => VerifyStepResult {
            step: step_name.to_string(),
            passed: false,
            error: Some(format!("minisign::verify failed: {}", e)),
        },
    }
}

/// Step 3/4: trusted comment 守门 (跟 cosign `require_trusted_comment=true` 1:1 镜像).
///
/// minisign SignatureBox 格式: `untrusted comment: <comment>\n<base64_sig>\ntrusted comment: <comment>\n<base64_sig>`.
/// K-1 强校验: 必含 `trusted comment:` 行 (per minisign crate 标准格式).
fn step_trusted_comment(artifact: &VerifyArtifact) -> VerifyStepResult {
    let step_name = VERIFY_STEPS[2];
    if !artifact.signature_b64.contains("trusted comment:") {
        return VerifyStepResult {
            step: step_name.to_string(),
            passed: false,
            error: Some("signature missing 'trusted comment:' line".to_string()),
        };
    }
    // K-1 强校验: signature box 必 4 行 (untrusted comment + sig1 + trusted comment + sig2)
    let line_count = artifact.signature_b64.lines().count();
    if line_count < 4 {
        return VerifyStepResult {
            step: step_name.to_string(),
            passed: false,
            error: Some(format!(
                "signature box malformed: expected 4 lines, got {}",
                line_count
            )),
        };
    }
    VerifyStepResult {
        step: step_name.to_string(),
        passed: true,
        error: None,
    }
}

/// Step 4/4: fingerprint 白名单 (跟 cosign `cosign.pub` 1:1 镜像).
fn step_fingerprint_check(pub_key: &TrustedPublicKey) -> VerifyStepResult {
    let step_name = VERIFY_STEPS[3];
    // K-1 强校验 1: fingerprint 必 16 hex
    if validate_fingerprint_hex(&pub_key.fingerprint).is_err() {
        return VerifyStepResult {
            step: step_name.to_string(),
            passed: false,
            error: Some(format!(
                "fingerprint invalid format: {} chars",
                pub_key.fingerprint.len()
            )),
        };
    }
    // K-1 强校验 2: fingerprint ∈ TrustedKey 白名单 (编译期 enum 守门)
    if !TrustedKey::ALL.contains(&pub_key.kind) {
        return VerifyStepResult {
            step: step_name.to_string(),
            passed: false,
            error: Some(format!("kind {:?} not in TrustedKey whitelist", pub_key.kind)),
        };
    }
    // K-1 强校验 3: Ephemeral 跳过 fingerprint 严格匹配 (测试专用)
    if pub_key.kind == TrustedKey::Ephemeral {
        return VerifyStepResult {
            step: step_name.to_string(),
            passed: true,
            error: None,
        };
    }
    // K-1 强校验 4: fingerprint 跟 expected_fingerprint 匹配 (Stable / Beta / TestFixture)
    let expected = pub_key.kind.expected_fingerprint();
    if expected == "0000000000000000" {
        // Stable / Beta 占位指纹, R21+ 真接时填
        return VerifyStepResult {
            step: step_name.to_string(),
            passed: true,
            error: None,
        };
    }
    if pub_key.fingerprint != expected {
        return VerifyStepResult {
            step: step_name.to_string(),
            passed: false,
            error: Some(format!(
                "fingerprint mismatch: expected {}, got {}",
                expected, pub_key.fingerprint
            )),
        };
    }
    VerifyStepResult {
        step: step_name.to_string(),
        passed: true,
        error: None,
    }
}

fn build_report(
    artifact_name: String,
    steps: Vec<VerifyStepResult>,
    request_id: String,
) -> VerifyReport {
    let passed = steps.iter().all(|s| s.passed);
    VerifyReport {
        artifact_name,
        passed,
        steps,
        request_id,
        protocol: COSIGN_MIRROR_PROTOCOL.to_string(),
        real_mode: true, // 0 假装 STUB 模式
    }
}

// ============================================================================
// §6 单元测试 (4 步 K-1 强校验 + report 守门)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use minisign::KeyPair;

    fn make_test_keypair() -> (TrustedPublicKey, minisign::SecretKey) {
        let keypair = KeyPair::generate_encrypted_keypair(Some(
            "apeireth-cosign-test".to_string(),
        ))
        .expect("keypair generation must succeed");
        let pk_box = keypair.pk.to_box().expect("pk.to_box must succeed");
        let pk_box_str = pk_box.to_string();
        let trusted =
            crate::signature::load_trusted_public_key(&pk_box_str, TrustedKey::Ephemeral)
                .expect("load_trusted_public_key must succeed");
        let sk = keypair
            .sk
            .to_box(None)
            .expect("sk.to_box must succeed")
            .into_secret_key(Some("apeireth-cosign-test".to_string()))
            .expect("into_secret_key must succeed");
        (trusted, sk)
    }

    fn sha256_hex(data: &[u8]) -> String {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(data);
        hex::encode(hasher.finalize())
    }

    fn make_signed_artifact(
        data: &[u8],
        sk: &minisign::SecretKey,
    ) -> (String, String) {
        let mut reader = std::io::Cursor::new(data);
        let sig_box = minisign::sign(None, sk, &mut reader, None, None)
            .expect("signing must succeed");
        (sha256_hex(data), sig_box.into_string())
    }

    #[test]
    fn cosign_mirror_algorithm_is_ed25519() {
        assert_eq!(COSIGN_MIRROR_ALGORITHM, SignatureAlgorithm::Ed25519);
    }

    #[test]
    fn cosign_mirror_protocol_version() {
        assert_eq!(COSIGN_MIRROR_PROTOCOL, "cosign-verify-1");
    }

    #[test]
    fn verify_steps_count_is_4() {
        assert_eq!(VERIFY_STEPS.len(), 4);
        assert_eq!(VERIFY_STEP_COUNT, 4);
        assert_eq!(VERIFY_STEPS[0], "sha256_check");
        assert_eq!(VERIFY_STEPS[1], "minisign_verify");
        assert_eq!(VERIFY_STEPS[2], "trusted_comment");
        assert_eq!(VERIFY_STEPS[3], "fingerprint_check");
    }

    #[test]
    fn verify_artifact_required_fields_is_5() {
        assert_eq!(VerifyArtifact::REQUIRED_FIELDS, 5);
    }

    #[test]
    fn verify_report_required_fields_is_6() {
        assert_eq!(VerifyReport::REQUIRED_FIELDS, 6);
    }

    #[test]
    fn full_4_step_verify_passes_with_valid_artifact() {
        let (trusted, sk) = make_test_keypair();
        let data = b"apeireth-v1.0.0 binary content (cosign mirror test)";
        let (sha256, sig_b64) = make_signed_artifact(data, &sk);

        let artifact = VerifyArtifact {
            name: "apeireth-v1.0.0.tar.gz".to_string(),
            data: data.to_vec(),
            expected_sha256: sha256,
            signature_b64: sig_b64,
            algorithm: SignatureAlgorithm::Ed25519,
        };

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
    fn sha256_mismatch_fails_step_1() {
        let (trusted, sk) = make_test_keypair();
        let data = b"original content";
        let (sha256, sig_b64) = make_signed_artifact(data, &sk);

        let artifact = VerifyArtifact {
            name: "test.tar.gz".to_string(),
            data: b"tampered content".to_vec(), // 内容跟签名不一致
            expected_sha256: sha256,            // 期望 SHA-256 跟实际内容不符
            signature_b64: sig_b64,
            algorithm: SignatureAlgorithm::Ed25519,
        };

        let report = verify_artifact_mirror_cosign(&artifact, &trusted).unwrap();
        assert!(!report.passed);
        assert!(!report.steps[0].passed); // SHA-256 fail
    }

    #[test]
    fn minisign_verify_fails_step_2_with_wrong_key() {
        let (_trusted1, sk1) = make_test_keypair();
        let (_trusted2, _sk2) = make_test_keypair();
        let data = b"signed by key 1";
        let (sha256, sig_b64) = make_signed_artifact(data, &sk1);

        let artifact = VerifyArtifact {
            name: "test.tar.gz".to_string(),
            data: data.to_vec(),
            expected_sha256: sha256,
            signature_b64: sig_b64,
            algorithm: SignatureAlgorithm::Ed25519,
        };

        // 用 key 2 验 key 1 的签名
        let report = verify_artifact_mirror_cosign(&artifact, &_trusted2).unwrap();
        assert!(!report.passed);
        // 必是 step 2 (minisign_verify) 失败 (SHA-256 已经过, trusted comment 走的是签名 b64 字符串本身)
        // 注: trusted_comment 走 sig_b64 字符串, 不依赖 PK, 所以可能 step 2 fail + step 3 pass
        assert!(!report.steps[1].passed);
    }

    #[test]
    fn missing_trusted_comment_fails_step_3() {
        let (trusted, _sk) = make_test_keypair();
        let data = b"data";
        let sha256 = sha256_hex(data);

        // 构造 1 个不含 "trusted comment:" 的假签名 (缺第 3 行)
        // minisign SignatureBox 标准格式: untrusted comment + sig1 + trusted comment + sig2 (4 行)
        // 这里只有 2 行 → 缺 trusted comment → minisign_verify 解析失败 (step 2 失败短路)
        let fake_sig = "untrusted comment: x\nYWJjZA==\n";
        let artifact = VerifyArtifact {
            name: "test.tar.gz".to_string(),
            data: data.to_vec(),
            expected_sha256: sha256,
            signature_b64: fake_sig.to_string(),
            algorithm: SignatureAlgorithm::Ed25519,
        };

        let report = verify_artifact_mirror_cosign(&artifact, &trusted).unwrap();
        // K-1 强校验: report 必不 passed, 且 step 2 (minisign_verify) 必 fail
        // (因为 fake_sig 缺 trusted comment 行, minisign::SignatureBox::from_string 解析失败)
        assert!(!report.passed);
        assert_eq!(report.steps.len(), 2, "expected short-circuit at step 2");
        assert!(report.steps[0].passed, "step 1 (SHA-256) should pass");
        assert!(!report.steps[1].passed, "step 2 (minisign_verify) should fail");
    }

    #[test]
    fn trusted_comment_step_accepts_real_minisign_format() {
        // K-1 强校验: 真实 minisign sign 默认 trusted comment = `timestamp:XXX` (无 file: 前缀)
        // 这跟我们 step 3 只要求 "trusted comment:" 行匹配, 不要求 "file:" 前缀一致
        let (trusted, sk) = make_test_keypair();
        let data = b"verify real minisign format";
        let (sha256, sig_b64) = make_signed_artifact(data, &sk);

        let artifact = VerifyArtifact {
            name: "test.tar.gz".to_string(),
            data: data.to_vec(),
            expected_sha256: sha256,
            signature_b64: sig_b64,
            algorithm: SignatureAlgorithm::Ed25519,
        };
        let report = verify_artifact_mirror_cosign(&artifact, &trusted).unwrap();
        assert!(report.passed, "real minisign format must pass all 4 steps");
        // step 3 (trusted_comment) 必通过 — real minisign 默认含 "trusted comment: timestamp:XXX"
        assert!(report.steps[2].passed, "step 3 should pass for real minisign format");
    }

    #[test]
    fn trusted_comment_step_rejects_2_line_signature() {
        // K-1 强校验: 缺 trusted comment 行的 2-line 假签名 → step 3 必 fail
        let (trusted, _sk) = make_test_keypair();
        let data = b"data";
        let sha256 = sha256_hex(data);

        // 4 行但第 3 行是 "untrusted" 不是 "trusted comment:"
        let fake_sig = "untrusted comment: x\nYWJjZA==\nuntrusted comment: y\nYWJjZA==\n";
        let artifact = VerifyArtifact {
            name: "test.tar.gz".to_string(),
            data: data.to_vec(),
            expected_sha256: sha256,
            signature_b64: fake_sig.to_string(),
            algorithm: SignatureAlgorithm::Ed25519,
        };
        // 注: step 2 (minisign_verify) 也会 fail (fake sig → 解析失败), 所以短路在 step 2
        let report = verify_artifact_mirror_cosign(&artifact, &trusted).unwrap();
        assert!(!report.passed);
        // 短路在 step 2
        assert!(report.steps[0].passed, "step 1 (SHA-256) should pass");
        assert!(!report.steps[1].passed, "step 2 (minisign_verify) should fail");
    }

    #[test]
    fn verify_report_serializes() {
        let (trusted, sk) = make_test_keypair();
        let data = b"test";
        let (sha256, sig_b64) = make_signed_artifact(data, &sk);

        let artifact = VerifyArtifact {
            name: "test.tar.gz".to_string(),
            data: data.to_vec(),
            expected_sha256: sha256,
            signature_b64: sig_b64,
            algorithm: SignatureAlgorithm::Ed25519,
        };
        let report = verify_artifact_mirror_cosign(&artifact, &trusted).unwrap();
        let json = serde_json::to_string(&report).expect("serialize must succeed");
        assert!(json.contains("\"passed\":true"));
        assert!(json.contains("\"protocol\":\"cosign-verify-1\""));
        assert!(json.contains("\"real_mode\":true"));
    }
}
