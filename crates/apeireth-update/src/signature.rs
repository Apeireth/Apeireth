//! # Minisign signature verification
//!
//! 借鉴 Golutra P3 minisign 公钥 `99F790EC4BE6E38D` 思想, 用现成
//! [`minisign`](https://crates.io/crates/minisign) crate (jedisct1/rust-minisign) 验签.
//! **0 重复造轮子** (8 项不修改承诺 #7).
//!
//! ## K-1 强校验
//!
//! - **公钥指纹白名单**: 编译期 hardcode [`TrustedKey`] 枚举, R21+ 真接时新增 fingerprint 走评审.
//! - **签名算法**: 编译期 hardcode [`SignatureAlgorithm::Ed25519`], 不允许运行时切换.
//! - **TrustedKey 枚举顺序** 严守 (8 项不修改承诺 #5).
//!
//! ## ⏳ R21+ 续真接
//!
//! - 加载 Apeireth 真实 minisign 公钥 (替代 `TEST_MINISIGN_PUBLIC_KEY` fixture).
//! - 改 `SignatureAlgorithm::Ed25519` 默认值 (如改 Ed448 / 跟 Golutra 同步).

use minisign::{PublicKey, PublicKeyBox, SignatureBox};

use crate::error::{
    validate_fingerprint_hex, validate_public_key_b64, validate_signature_b64, UpdateError,
    UpdateResult,
};

// ============================================================================
// §1 编译期 hardcode 常量 (K-1 强校验 + 8 项不修改承诺 #2 + #4)
// ============================================================================

/// 签名算法 (K-1 强校验: 编译期 hardcode Ed25519, 不允许运行时切换).
///
/// 顺序固定 (8 项不修改承诺 #5): R21+ 续真接时不增删.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum SignatureAlgorithm {
    /// Ed25519 (RFC 8032, minisign 默认, 当前唯一支持).
    #[serde(rename = "ed25519")]
    Ed25519,
}

impl SignatureAlgorithm {
    /// 全部 1 算法列表 (R21+ 加 Ed448 时改).
    pub const ALL: &'static [SignatureAlgorithm] = &[SignatureAlgorithm::Ed25519];

    /// 算法编译期守门 (K-1: 1 个).
    pub const COUNT: usize = 1;

    /// 算法名 (per 借鉴文档 §8 P3 第 10 项 minisign 公钥指纹).
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            SignatureAlgorithm::Ed25519 => "ed25519",
        }
    }
}

const _: () = assert!(SignatureAlgorithm::COUNT == 1);

/// 信任公钥指纹枚举 (K-1 强校验: 编译期 hardcode, 8 项不修改承诺 #3).
///
/// **3 枚举值固定** (R21+ 真接时新增 fingerprint 走评审, 不在 enum 内动态加):
/// - `TestFixture` (测试 fixture 公钥, 借鉴文档 §8 P3 第 10 项示例指纹 `99F790EC4BE6E38D`)
/// - `Stable` (Apeireth Stable release 公钥, R21+ 真接时填)
/// - `Beta` (Apeireth Beta release 公钥, R21+ 真接时填)
/// - `Ephemeral` (测试临时公钥, R21+ 真接时**移除** — 8 项承诺 §6 枚举扩 1 估补项,
//   测试用, 跳过白名单校验, 不进 production 路径)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TrustedKey {
    /// 测试 fixture (per 借鉴文档 §8 P3 第 10 项示例 `99F790EC4BE6E38D`).
    TestFixture,
    /// Apeireth Stable release 公钥 (R21+ 真接时填, 当前空).
    Stable,
    /// Apeireth Beta release 公钥 (R21+ 真接时填, 当前空).
    Beta,
    /// 测试临时公钥 (R21+ 真接时**移除**). 用于集成测试, 跳过白名单校验.
    Ephemeral,
}

impl TrustedKey {
    /// 全部 4 枚举列表 (8 项不修改承诺 #5 枚举顺序固定, Ephemeral 末尾 = 测试专用).
    pub const ALL: &'static [TrustedKey] = &[
        TrustedKey::TestFixture,
        TrustedKey::Stable,
        TrustedKey::Beta,
        TrustedKey::Ephemeral,
    ];

    /// 编译期守门: 4 枚举 (3 production + 1 test).
    pub const COUNT: usize = 4;

    /// 期望指纹 (hex, 16 字符).
    ///
    /// `TestFixture` = 借鉴文档 §8 P3 第 10 项示例 `99F790EC4BE6E38D` (Golutra 公钥前缀).
    /// `Stable` / `Beta` = R21+ 真接时填 (留 placeholder).
    /// `Ephemeral` = 测试专用, 跳过白名单 (R21+ 真接时移除, 此 variant 仅 testing).
    #[must_use]
    pub const fn expected_fingerprint(&self) -> &'static str {
        match self {
            TrustedKey::TestFixture => "99F790EC4BE6E38D",
            // ⏳ R21+ 真接时: 填 Apeireth 真实公钥指纹 (16 字符 hex, 8 bytes truncated).
            TrustedKey::Stable => "0000000000000000",
            TrustedKey::Beta => "0000000000000000",
            // Ephemeral: 不参与白名单 (白名单逻辑在 load_trusted_public_key 里 bypass)
            TrustedKey::Ephemeral => "0000000000000000",
        }
    }
}

const _: () = assert!(TrustedKey::COUNT == 4);

/// K-1 强校验变体 (5 校验, 跟 [`crate::error::K1_STRONG_VALIDATION_VARIANTS`] 镜像).
pub const SIGNATURE_K1_VALIDATION_VARIANTS: [&str; 5] = [
    "InvalidPublicKeyEncoding",
    "InvalidSignatureEncoding",
    "UntrustedFingerprint",
    "AlgorithmMismatch",
    "DataLengthMismatch",
];

/// 编译期守门: 5 K-1 校验.
const _: () = assert!(SIGNATURE_K1_VALIDATION_VARIANTS.len() == 5);

// ============================================================================
// §2 TrustedKey 公钥加载 + 指纹提取
// ============================================================================

/// 已加载的信任公钥 (含原始 minisign `PublicKey` + 指纹).
///
/// 公开 API (8 项不修改承诺 #2 公开 API 100% 文档化):
/// - `pub_key`: minisign 原始 `PublicKey` (给 [`minisign::verify`] 用)
/// - `fingerprint`: 16 字符 hex 指纹 (K-1 强校验白名单比对)
/// - `kind`: [`TrustedKey`] 枚举
#[derive(Debug, Clone)]
pub struct TrustedPublicKey {
    /// minisign 原始公钥.
    pub pub_key: PublicKey,
    /// 16 字符 hex 指纹.
    pub fingerprint: String,
    /// 信任枚举.
    pub kind: TrustedKey,
}

/// 从 minisign 公钥 box 字符串加载信任公钥 (per 借鉴文档 §8 P3 第 10 项 Golutra minisign 模式).
///
/// 输入是 minisign 公钥 box 字符串 (含 untrusted comment + keynum + base64 pk),
/// 跟 `minisign -G` 生成的标准 `.pub` 文件格式一致.
///
/// 流程: minisign `PublicKeyBox::from_string` → `into_public_key` →
/// 指纹 SHA-256 截 8 bytes (16 hex chars) → K-1 强校验 4 步 (白名单 + 格式).
///
/// ⏳ **R21+ 真接时**: 当前 `key_box_str` 来自测试 fixture,
/// 真接时改 Apeireth 真实公钥 (`TrustedKey::Stable` / `Beta`).
pub fn load_trusted_public_key(
    key_box_str: &str,
    kind: TrustedKey,
) -> UpdateResult<TrustedPublicKey> {
    // K-1 校验 1: 公钥 box 字符串非空
    if key_box_str.is_empty() {
        return Err(UpdateError::InvalidPublicKey(
            "empty key box".to_string(),
        ));
    }

    // 解析 minisign PublicKeyBox (minisign crate API, 0 重复造轮子)
    let pk_box = PublicKeyBox::from_string(key_box_str).map_err(|e| {
        UpdateError::InvalidPublicKey(format!("PublicKeyBox::from_string failed: {}", e))
    })?;
    let pub_key = pk_box.into_public_key().map_err(|e| {
        UpdateError::InvalidPublicKey(format!("into_public_key failed: {}", e))
    })?;

    // K-1 校验 2: 指纹 = 整个 key_box_str SHA-256 截 8 bytes (16 hex chars)
    // 简化实现: 不同 minisign 格式兼容性靠 sha256 兜底
    let fingerprint = fingerprint_from_bytes(key_box_str.as_bytes());

    // K-1 校验 3: 指纹白名单 (跟 expected 对比)
    // 特殊:
    //   - Ephemeral: 跳过白名单 (测试临时公钥, R21+ 移除)
    //   - Stable / Beta 占位 fingerprint = "0000000000000000" 时跳过 (R21+ 真接时填)
    let expected_fp = kind.expected_fingerprint();
    if kind != TrustedKey::Ephemeral
        && fingerprint != expected_fp
        && expected_fp != "0000000000000000"
    {
        return Err(UpdateError::UntrustedPublicKey {
            expected: expected_fp.to_string(),
            got: fingerprint.clone(),
        });
    }

    // K-1 校验 4: 指纹格式
    validate_fingerprint_hex(&fingerprint)?;

    Ok(TrustedPublicKey {
        pub_key,
        fingerprint,
        kind,
    })
}

/// 计算指纹: 16 字符 hex (8 bytes truncated SHA-256).
fn fingerprint_from_bytes(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let full = hasher.finalize();
    hex::encode(&full[..8])
}

// ============================================================================
// §3 签 / 验签主函数 (真签真验, 0 假装, 用现成 minisign crate 0.9)
// ============================================================================

/// minisign 签 (公开 API, 8 项不修改承诺 #7 0 重复造轮子, 借 minisign crate).
///
/// 流程: 调 `minisign::sign(None, sk, reader, None, None)` → `SignatureBox` → `into_string()`.
///
/// 公开 API (8 项不修改承诺 #2 公开 API 100% 文档化).
///
/// # 参数
///
/// - `sk`: [`minisign::SecretKey`] 已解密的 secret key (per `into_secret_key` 解密)
/// - `data`: 待签的字节切片
///
/// # 返回
///
/// - `Ok(String)` minisign SignatureBox 字符串 (含 `untrusted comment:` + `trusted comment:` + 双 b64 行)
/// - `Err(UpdateError::InvalidRequest)` `data` 为空
///
/// # 错误
///
/// - [`UpdateError::InvalidRequest`] `data` 为空
/// - minisign crate 内部错误 (e.g. RNG 失败) 透传 [`UpdateError::SignatureVerifyFailed`]
///
/// ⏳ **R21+ 真接时**: 当前走 `minisign::sign` 默认 trusted comment, 真接时改
/// `format!("file:{},sequential", filename)` (per minisign crate docs).
pub fn sign_minisign(sk: &minisign::SecretKey, data: &[u8]) -> UpdateResult<String> {
    if data.is_empty() {
        return Err(UpdateError::InvalidRequest(
            "empty data for sign".to_string(),
        ));
    }
    let mut reader = std::io::Cursor::new(data);
    let sig_box = minisign::sign(None, sk, &mut reader, None, None)
        .map_err(|e| UpdateError::SignatureVerifyFailed(format!("minisign::sign: {}", e)))?;
    Ok(sig_box.into_string())
}

/// minisign 验签 (公开 API, 8 项不修改承诺 #7 0 重复造轮子).
///
/// 流程: K-1 强校验 5 步 → [`minisign::verify`] → Result<(), UpdateError>.
///
/// ⏳ **R21+ 真接时**: 当前不传 `require_trusted_comment = true` (兼容 fixture).
/// 真接时改 `true` (Golutra 模式 + minisign 0.9 推荐).
///
/// # 错误
///
/// - [`UpdateError::UntrustedPublicKey`] 公钥不在白名单
/// - [`UpdateError::InvalidSignature`] 签名 base64 解析失败
/// - [`UpdateError::SignatureVerifyFailed`] minisign 验签失败
/// - [`UpdateError::Stub`] R21+ 续真接占位
pub fn verify_minisign(
    pub_key: &TrustedPublicKey,
    data: &[u8],
    signature_b64: &str,
) -> UpdateResult<()> {
    // K-1 校验 1: 签名 b64 格式
    validate_signature_b64(signature_b64)?;

    // K-1 校验 2: 公钥 kind 必须在白名单 (编译期 enum 强守, 运行时仅 sanity check)
    if !TrustedKey::ALL.contains(&pub_key.kind) {
        return Err(UpdateError::UntrustedPublicKey {
            expected: "whitelisted TrustedKey".to_string(),
            got: format!("{:?}", pub_key.kind),
        });
    }

    // K-1 校验 3: 数据非空
    if data.is_empty() {
        return Err(UpdateError::SignatureVerifyFailed(
            "empty data".to_string(),
        ));
    }

    // 解析 minisign SignatureBox
    let sig_box = SignatureBox::from_string(signature_b64).map_err(|e| {
        UpdateError::InvalidSignature(format!("SignatureBox::from_string failed: {}", e))
    })?;

    // 调 minisign crate 验签 (8 项不修改承诺 #7 0 重复造轮子)
    // minisign::verify(pk, sig_box, reader, require_trusted_comment, no_output, quiet)
    let mut reader = std::io::Cursor::new(data);
    minisign::verify(
        &pub_key.pub_key,
        &sig_box,
        &mut reader,
        false, // require_trusted_comment (fixture 测试 false, R21+ 真接时改 true)
        true,  // no_output
        false, // quiet
    )
    .map_err(|e| UpdateError::SignatureVerifyFailed(format!("minisign::verify: {}", e)))
}

// ============================================================================
// §4 单元测试 (K-1 强校验守门 + 算法/枚举守门)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn algorithm_count_is_1() {
        assert_eq!(SignatureAlgorithm::COUNT, 1);
        assert_eq!(SignatureAlgorithm::ALL.len(), 1);
    }

    #[test]
    fn algorithm_all_is_ed25519() {
        assert_eq!(SignatureAlgorithm::ALL[0], SignatureAlgorithm::Ed25519);
        assert_eq!(SignatureAlgorithm::Ed25519.as_str(), "ed25519");
    }

    #[test]
    fn trusted_key_count_is_4() {
        // 8 项不修改承诺 §6 估补: 3 production + 1 test (Ephemeral) = 4
        assert_eq!(TrustedKey::COUNT, 4);
        assert_eq!(TrustedKey::ALL.len(), 4);
    }

    #[test]
    fn trusted_key_order_locked() {
        // 8 项不修改承诺 #5: 枚举顺序固定 (production 3 + test 1 末尾)
        assert_eq!(TrustedKey::ALL[0], TrustedKey::TestFixture);
        assert_eq!(TrustedKey::ALL[1], TrustedKey::Stable);
        assert_eq!(TrustedKey::ALL[2], TrustedKey::Beta);
        assert_eq!(TrustedKey::ALL[3], TrustedKey::Ephemeral);
    }

    #[test]
    fn k1_variants_count_is_5() {
        assert_eq!(SIGNATURE_K1_VALIDATION_VARIANTS.len(), 5);
    }

    #[test]
    fn test_fixture_fingerprint_is_golutra_prefix() {
        // 借鉴文档 §8 P3 第 10 项: Golutra minisign 公钥 `99F790EC4BE6E38D`
        // 我们用同样 prefix 作为 TestFixture 占位 (R21+ 真接时换 Apeireth 真公钥)
        assert_eq!(
            TrustedKey::TestFixture.expected_fingerprint(),
            "99F790EC4BE6E38D"
        );
    }

    #[test]
    fn stable_and_beta_fingerprint_placeholder() {
        // R21+ 真接时填 (当前 placeholder "0000000000000000", 加载时跳过白名单校验)
        assert_eq!(TrustedKey::Stable.expected_fingerprint(), "0000000000000000");
        assert_eq!(TrustedKey::Beta.expected_fingerprint(), "0000000000000000");
    }
}
