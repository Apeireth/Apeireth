//! Passphrase KDF: PBKDF2-HMAC-SHA-256 (仓内实现, 0 新外部包).
//!
//! **构造是标准构造** (HMAC-SHA-256 per RFC 2104 两段 ipad/opad + PBKDF2
//! 每块 U1||U2||… 链式异或), 实现钉**标准测试向量** (见测试: HMAC-SHA-256
//! 公开向量 + PBKDF2-HMAC-SHA-256 公开向量), 不做自创密码学。
//!
//! 为什么不引外部 KDF 包: 批口径「0 引新外部 dep」——workspace 加密族
//! (aes-gcm/sha2/rand) 已在锁内, KDF 包不在; 引新包列报告待拍板不动手,
//! 本模块按标准构造复用 `sha2` 自实现并以公开向量钉死正确性。
//!
//! **0 装诚实**: 默认迭代数已按拍板取 **600_000** (对齐公开口令硬化建议
//! 口径, 2026-10-21 B6-DEEP 调参: 210k → 600k); **解锁一次的开销未实测**
//! (600k 全速路径未在测试里跑满, 见 [`DEFAULT_PBKDF2_ITERATIONS`] 注)。

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use zeroize::Zeroizing;

use crate::crypto::error::CryptoError;

/// KDF 标识 (写进包裹槽位, 恢复时按 id 选派生路径)。
pub const KDF_KIND: &str = "pbkdf2-hmac-sha256";

/// 默认 PBKDF2 迭代数: **600_000** (2026-10-21 拍板值, 对齐公开口令硬化
/// 建议口径; 由测试钉死字面值防漂移)。
///
/// **性能未实测**: 解锁一次的开销 (600k × PBKDF2-HMAC-SHA-256) 没跑过
/// 实测数字, 如实登记; 测试面刻意用小迭代数 (算法正确性由公开标准向量钉,
/// 与迭代数无关)。
pub const DEFAULT_PBKDF2_ITERATIONS: u32 = 600_000;

/// 口令盐长度 (16 字节, 随机生成)。
pub const SALT_LEN: usize = 16;

/// HMAC-SHA-256 分组长度 (SHA-256 block = 64 字节)。
const HMAC_BLOCK: usize = 64;

/// 派生输出长度 (KEK = 32 字节)。
const KEK_LEN: usize = 32;

/// PBKDF2 参数 (随包裹槽位落盘, 恢复时照参数重派生)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KdfParams {
    /// KDF 标识; 现仅 [`KDF_KIND`] 一种, 未知值 = 显式拒绝。
    pub kind: String,
    /// 迭代数 (≥ 1)。
    pub iterations: u32,
    /// 盐 (hex)。
    pub salt_hex: String,
}

impl KdfParams {
    /// 构造 PBKDF2-HMAC-SHA-256 参数 (盐由调用方给定字节)。
    pub fn pbkdf2_hmac_sha256(iterations: u32, salt: &[u8]) -> Result<Self, CryptoError> {
        if iterations == 0 {
            return Err(CryptoError::InvalidKdf("iterations 必须 ≥ 1".to_string()));
        }
        Ok(Self {
            kind: KDF_KIND.to_string(),
            iterations,
            salt_hex: hex::encode(salt),
        })
    }

    /// 随机盐 + 默认迭代数 (600k 拍板值, 见 [`DEFAULT_PBKDF2_ITERATIONS`])。
    pub fn with_random_salt_default() -> Result<Self, CryptoError> {
        use rand::RngCore;
        let mut salt = [0u8; SALT_LEN];
        rand::rngs::OsRng.fill_bytes(&mut salt);
        Self::pbkdf2_hmac_sha256(DEFAULT_PBKDF2_ITERATIONS, &salt)
    }

    fn salt_bytes(&self) -> Result<Vec<u8>, CryptoError> {
        hex::decode(&self.salt_hex)
            .map_err(|error| CryptoError::InvalidKdf(format!("salt 不是合法 hex: {error}")))
    }
}

/// HMAC-SHA-256 (RFC 2104 构造; key 超过一个分组先哈希)。
pub fn hmac_sha256(key: &[u8], message: &[u8]) -> [u8; 32] {
    let mut block_key = Zeroizing::new([0u8; HMAC_BLOCK]);
    if key.len() > HMAC_BLOCK {
        block_key[..KEK_LEN].copy_from_slice(&Sha256::digest(key));
    } else {
        block_key[..key.len()].copy_from_slice(key);
    }

    let mut ipad = [0x36u8; HMAC_BLOCK];
    let mut opad = [0x5cu8; HMAC_BLOCK];
    for index in 0..HMAC_BLOCK {
        ipad[index] ^= block_key[index];
        opad[index] ^= block_key[index];
    }

    let mut inner = Sha256::new();
    inner.update(ipad);
    inner.update(message);
    let inner_hash = inner.finalize();

    let mut outer = Sha256::new();
    outer.update(opad);
    outer.update(inner_hash);
    outer.finalize().into()
}

/// PBKDF2-HMAC-SHA-256 (标准构造; `out` 长度任意, 逐 32 字节块派生)。
///
/// `iterations == 0` 拒绝 ([`CryptoError::InvalidKdf`]), 不静默降级。
pub fn pbkdf2_hmac_sha256(
    password: &[u8],
    salt: &[u8],
    iterations: u32,
    out: &mut [u8],
) -> Result<(), CryptoError> {
    if iterations == 0 {
        return Err(CryptoError::InvalidKdf("iterations 必须 ≥ 1".to_string()));
    }
    for (block_index, chunk) in out.chunks_mut(32).enumerate() {
        let counter = u32::try_from(block_index)
            .ok()
            .and_then(|index| index.checked_add(1))
            .ok_or_else(|| CryptoError::InvalidKdf("派生长度超出 PBKDF2 块计数".to_string()))?;
        let mut block_input = Vec::with_capacity(salt.len() + 4);
        block_input.extend_from_slice(salt);
        block_input.extend_from_slice(&counter.to_be_bytes());

        let mut u = hmac_sha256(password, &block_input);
        let mut accumulated = u;
        for _ in 1..iterations {
            u = hmac_sha256(password, &u);
            for (acc_byte, u_byte) in accumulated.iter_mut().zip(u.iter()) {
                *acc_byte ^= u_byte;
            }
        }
        chunk.copy_from_slice(&accumulated[..chunk.len()]);
    }
    Ok(())
}

/// 用户口令 → KEK (32 字节, [`Zeroizing`] 落地零化)。
pub fn derive_kek_from_passphrase(
    passphrase: &[u8],
    params: &KdfParams,
) -> Result<Zeroizing<[u8; 32]>, CryptoError> {
    if params.kind != KDF_KIND {
        return Err(CryptoError::InvalidKdf(format!(
            "未知 KDF kind `{}` (仅支持 `{KDF_KIND}`)",
            params.kind
        )));
    }
    let salt = params.salt_bytes()?;
    let mut kek = Zeroizing::new([0u8; KEK_LEN]);
    pbkdf2_hmac_sha256(passphrase, &salt, params.iterations, &mut *kek)?;
    Ok(kek)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decode_hex(text: &str) -> Vec<u8> {
        hex::decode(text).expect("hex")
    }

    #[test]
    fn hmac_sha256_matches_the_published_short_key_vector() {
        // 公开标准测试向量 (HMAC-SHA-256, 短密钥 "Jefe" / "what do ya want for nothing?")。
        let tag = hmac_sha256(b"Jefe", b"what do ya want for nothing?");
        assert_eq!(
            hex::encode(tag),
            "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843"
        );
    }

    #[test]
    fn hmac_sha256_matches_the_published_long_key_vector() {
        // 公开标准测试向量 (key = 20 × 0x0b, data = "Hi There")。
        let tag = hmac_sha256(&[0x0bu8; 20], b"Hi There");
        assert_eq!(
            hex::encode(tag),
            "b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7"
        );
    }

    #[test]
    fn hmac_sha256_hashes_over_long_keys() {
        // 公开标准测试向量 (key = 131 × 0xaa)。
        let tag = hmac_sha256(
            &[0xaau8; 131],
            b"Test Using Larger Than Block-Size Key - Hash Key First",
        );
        assert_eq!(
            hex::encode(tag),
            "60e431591ee0b67f0d8a26aacbf5b77f8e0bc6213728c5140546040f0ee37f54"
        );
    }

    #[test]
    fn pbkdf2_hmac_sha256_matches_the_published_vectors() {
        // 公开标准测试向量 (PBKDF2-HMAC-SHA-256, P="password", S="salt")。
        let mut out = [0u8; 32];
        pbkdf2_hmac_sha256(b"password", b"salt", 1, &mut out).expect("derive");
        assert_eq!(
            hex::encode(out),
            "120fb6cffcf8b32c43e7225256c4f837a86548c92ccc35480805987cb70be17b"
        );

        pbkdf2_hmac_sha256(b"password", b"salt", 2, &mut out).expect("derive");
        assert_eq!(
            hex::encode(out),
            "ae4d0c95af6b46d32d0adff928f06dd02a303f8ef3c251dfd6e2d85a95474c43"
        );

        pbkdf2_hmac_sha256(b"password", b"salt", 4096, &mut out).expect("derive");
        assert_eq!(
            hex::encode(out),
            "c5e478d59288c841aa530db6845c4c8d962893a001ce4e11a4963873aa98134a"
        );
    }

    #[test]
    fn pbkdf2_rejects_zero_iterations() {
        let mut out = [0u8; 32];
        let error =
            pbkdf2_hmac_sha256(b"password", b"salt", 0, &mut out).expect_err("0 迭代必须拒");
        assert!(matches!(error, CryptoError::InvalidKdf(_)));
    }

    #[test]
    fn derive_kek_rejects_unknown_kdf_kind() {
        let params = KdfParams {
            kind: "unknown-kdf".to_string(),
            iterations: 1,
            salt_hex: "00".to_string(),
        };
        let error = derive_kek_from_passphrase(b"pw", &params).expect_err("未知 kind 必须拒");
        assert!(matches!(error, CryptoError::InvalidKdf(_)));
    }

    #[test]
    fn derive_kek_is_deterministic_per_params_and_sensitive_to_salt() {
        let base = KdfParams::pbkdf2_hmac_sha256(8, b"salt-a").expect("params");
        let other = KdfParams::pbkdf2_hmac_sha256(8, b"salt-b").expect("params");
        let first = derive_kek_from_passphrase(b"pw", &base).expect("kek");
        let second = derive_kek_from_passphrase(b"pw", &base).expect("kek");
        let third = derive_kek_from_passphrase(b"pw", &other).expect("kek");
        assert_eq!(*first, *second, "同参数同口令必须同 KEK");
        assert_ne!(*first, *third, "换盐必须换 KEK");
    }

    #[test]
    fn random_salt_params_are_well_formed() {
        let params = KdfParams::with_random_salt_default().expect("params");
        assert_eq!(params.kind, KDF_KIND);
        assert_eq!(params.iterations, DEFAULT_PBKDF2_ITERATIONS);
        assert_eq!(decode_hex(&params.salt_hex).len(), SALT_LEN);
    }

    #[test]
    fn default_iterations_are_pinned_to_the_approved_600k_value() {
        // 拍板值字面钉死 (2026-10-21: 210k → 600k, 对齐公开口令硬化建议口径);
        // 改这个数 = 改安全语义, 必须走拍板 + 同步本测试。
        assert_eq!(DEFAULT_PBKDF2_ITERATIONS, 600_000);
    }
}
