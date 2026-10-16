//! IM 快捷接入: 共享秘密持有 + 入站签名校验 (日志红线)。
//!
//! - [`ImSecret`] 持有渠道共享秘密: `Debug` / `Display` 恒脱敏 `[redacted]`,
//!   0 明文进日志;
//! - 入站签名契约 (适配器口径): `sha256=<hex(sha256(secret || "\n" || body))>`,
//!   比较恒定时间, 失败只报事实类别 [`crate::im::error::ImError::Verify`]。

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::im::error::{ImError, ImResult};
use crate::redact::REDACTED_PLACEHOLDER;

/// 签名前缀 (`sha256=<hex>`)。
pub const IM_SIGNATURE_PREFIX: &str = "sha256=";

/// 渠道共享秘密 (内存持有, Debug 脱敏; serde 落盘为明文配置值)。
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ImSecret(String);

impl ImSecret {
    /// 持有一个秘密值。空秘密非法 (fail-closed)。
    pub fn new(value: impl Into<String>) -> ImResult<Self> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err(ImError::InvalidConfig {
                reason: "channel secret must not be empty".to_string(),
            });
        }
        Ok(Self(value))
    }

    /// 明文读取 (仅用于签名计算, 不得进日志)。
    pub fn reveal(&self) -> &str {
        &self.0
    }

    /// 秘密长度 (用于容量统计, 不泄露内容)。
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// 是否为空 (恒 `false`: 构造期已拒空)。
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// 计算请求体签名: `sha256=<hex(sha256(secret || "\n" || body))>`。
    pub fn sign_body(&self, body: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(self.0.as_bytes());
        hasher.update(b"\n");
        hasher.update(body);
        format!("{IM_SIGNATURE_PREFIX}{:x}", hasher.finalize())
    }

    /// 校验请求体签名 (恒定时间比较; 失败只报 `mismatch`)。
    pub fn verify_body(&self, body: &[u8], provided: &str) -> ImResult<()> {
        let expected = self.sign_body(body);
        if constant_time_eq(expected.as_bytes(), provided.as_bytes()) {
            Ok(())
        } else {
            Err(ImError::Verify {
                reason: "signature mismatch".to_string(),
            })
        }
    }
}

impl std::fmt::Debug for ImSecret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(REDACTED_PLACEHOLDER)
    }
}

impl std::fmt::Display for ImSecret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(REDACTED_PLACEHOLDER)
    }
}

/// 恒定时间字节比较 (长度不同也跑满短的一侧, 不早退)。
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    let max = a.len().max(b.len());
    let mut diff = a.len() ^ b.len();
    for i in 0..max {
        let left = a.get(i).copied().unwrap_or(0);
        let right = b.get(i).copied().unwrap_or(0);
        diff |= usize::from(left ^ right);
    }
    diff == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secret_is_redacted_in_debug_and_display() {
        let secret = ImSecret::new("super-secret-value").unwrap();
        assert_eq!(format!("{secret:?}"), REDACTED_PLACEHOLDER);
        assert_eq!(format!("{secret}"), REDACTED_PLACEHOLDER);
        assert_eq!(secret.reveal(), "super-secret-value");
    }

    #[test]
    fn empty_secret_is_rejected() {
        assert!(matches!(
            ImSecret::new("   "),
            Err(ImError::InvalidConfig { .. })
        ));
    }

    #[test]
    fn body_signature_round_trips_and_rejects_tampering() {
        let secret = ImSecret::new("shared-key").unwrap();
        let body = br#"{"text":"hello"}"#;
        let signature = secret.sign_body(body);
        assert!(signature.starts_with(IM_SIGNATURE_PREFIX));
        secret.verify_body(body, &signature).unwrap();

        assert!(matches!(
            secret.verify_body(br#"{"text":"tampered"}"#, &signature),
            Err(ImError::Verify { .. })
        ));
        let mut wrong = signature.clone();
        wrong.pop();
        assert!(matches!(
            secret.verify_body(body, &wrong),
            Err(ImError::Verify { .. })
        ));
    }

    #[test]
    fn distinct_secrets_produce_distinct_signatures() {
        let a = ImSecret::new("key-a").unwrap();
        let b = ImSecret::new("key-b").unwrap();
        assert_ne!(a.sign_body(b"x"), b.sign_body(b"x"));
    }
}
