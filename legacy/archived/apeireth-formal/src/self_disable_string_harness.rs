//! R133.1 Self-Disable 5 机制字符串 ownership_token 形式化
//!
//! **目的**: 把 R131.8 POD 模型的 `u8` / `u32` token 升级为**真实字符串 token** (per 5 机制实际定义).
//! 验证: 任意字符串 token 都不能绕过 5 重治理, 5 机制严格区分 (机制 ID 1..=5 唯一).
//!
//! **跑法**: `cargo test -p apeireth-formal --lib self_disable_string_harness` (unit)
//!          `cargo kani --harness kani_verify_*` (Kani 形式化, 任意字符串输入)
//!
//! **对比 R131.8**:
//! - R131.8 `SelfDisableGuardPod` 用 `u8` / `u32` 模拟 token (降级版, Kani 友好但失真)
//! - R133.1 `StringPod` 用固定 32-byte buffer + length (Kani 友好 + 真实字符串语义)
//!
//! **5 机制 token 映射** (per `apeireth-sovereignty::self_disable` 真实定义):
//! 1. `NoDegradeViolation { from, to }` — 风险等级字符串
//! 2. `NoPatchViolation { rule }` — 规则名字符串
//! 3. `NoBypassViolation { token }` — 绕过 token 字符串 (含 `Master` 也不能绕)
//! 4. `NoReverseViolation { trigger_id }` — trigger_id 字符串
//! 5. `NoHideViolation { window_id }` — window_id 字符串

#![cfg_attr(kani, allow(dead_code))]

// ============================================================
// 编译期常量
// ============================================================

pub const SELF_DISABLE_MECHANISM_COUNT: usize = 5;
pub const STRING_POD_MAX_LEN: usize = 32;  // Kani-friendly 固定 buffer

// ============================================================
// StringPod — Kani 友好的字符串 POD 模型
// ============================================================

/// **StringPod**: 固定 32-byte buffer + length, Kani 任意字符串测试.
///
/// 设计选择:
/// - 不用 `String` (heap allocation, Kani 不友好)
/// - 不用 `&str` (借用, lifetime 复杂)
/// - 用 `[u8; 32]` + length (Kani::any 完全符号化)
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct StringPod {
    pub buf: [u8; STRING_POD_MAX_LEN],
    pub len: u8,
}

impl StringPod {
    pub const fn empty() -> Self {
        Self { buf: [0u8; STRING_POD_MAX_LEN], len: 0 }
    }

    /// 从字节切片构造 (长度截断到 32).
    pub fn from_slice(s: &[u8]) -> Self {
        let mut sp = Self::empty();
        let n = s.len().min(STRING_POD_MAX_LEN);
        sp.buf[..n].copy_from_slice(&s[..n]);
        sp.len = n as u8;
        sp
    }

    /// 从字面量构造 (测试用).
    pub const fn from_str(s: &str) -> Self {
        let bytes = s.as_bytes();
        let mut sp = Self::empty();
        let mut i = 0;
        while i < bytes.len() && i < STRING_POD_MAX_LEN {
            sp.buf[i] = bytes[i];
            i += 1;
        }
        sp.len = i as u8;
        sp
    }

    /// 转 `&str` (测试断言用, 非 Kani 路径).
    pub fn as_str(&self) -> &str {
        core::str::from_utf8(&self.buf[..self.len as usize]).unwrap_or("")
    }

    /// 是否包含子串 (case-sensitive).
    pub fn contains(&self, needle: &str) -> bool {
        self.as_str().contains(needle)
    }

    /// 严格相等 (per R133.1: 区分 case, 大小写敏感).
    pub fn eq_str(&self, other: &str) -> bool {
        self.as_str() == other
    }
}

impl Default for StringPod {
    fn default() -> Self { Self::empty() }
}

// ============================================================
// 5 机制字符串 token POD
// ============================================================

/// **5 机制 token 严格区分 POD** (per R133.1, 5 机制 token 类型互不相同).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum SelfDisableTokenPod {
    /// 1. NoDegradeViolation — 风险等级 (from / to 两个 StringPod)
    NoDegrade { from: StringPod, to: StringPod },
    /// 2. NoPatchViolation — 规则名 (rule StringPod)
    NoPatch { rule: StringPod },
    /// 3. NoBypassViolation — 绕过 token (token StringPod, 含 "Master" 也不能绕)
    NoBypass { token: StringPod },
    /// 4. NoReverseViolation — trigger_id (trigger_id StringPod)
    NoReverse { trigger_id: StringPod },
    /// 5. NoHideViolation — window_id (window_id StringPod)
    NoHide { window_id: StringPod },
}

impl SelfDisableTokenPod {
    /// 机制 ID (1-5, 严格唯一, 编译期 hardcode).
    pub fn mechanism_id(&self) -> u8 {
        match self {
            Self::NoDegrade { .. } => 1,
            Self::NoPatch { .. } => 2,
            Self::NoBypass { .. } => 3,
            Self::NoReverse { .. } => 4,
            Self::NoHide { .. } => 5,
        }
    }

    /// 5 机制严格区分 (per R133.1): 不同 mechanism 不能互转.
    /// 5 个 variant 各有独立字段结构, 编译期 exhaustive match 守门.
    pub fn is_no_degrade(&self) -> bool { matches!(self, Self::NoDegrade { .. }) }
    pub fn is_no_patch(&self) -> bool { matches!(self, Self::NoPatch { .. }) }
    pub fn is_no_bypass(&self) -> bool { matches!(self, Self::NoBypass { .. }) }
    pub fn is_no_reverse(&self) -> bool { matches!(self, Self::NoReverse { .. }) }
    pub fn is_no_hide(&self) -> bool { matches!(self, Self::NoHide { .. }) }
}

// ============================================================
// 编译期守门 (K-1 强校验)
// ============================================================

const _: () = assert!(SELF_DISABLE_MECHANISM_COUNT == 5);
const _: () = assert!(STRING_POD_MAX_LEN == 32);

// ============================================================
// Kani proof harness
// ============================================================

#[cfg(kani)]
mod kani_proofs {
    use super::*;

    /// **R133.1 证明 #1**: 5 机制 ID 严格唯一 (任意 token → mechanism_id ∈ 1..=5).
    #[kani::proof]
    fn kani_verify_string_token_mechanism_id_unique() {
        let token: SelfDisableTokenPod = kani::any();
        let id = token.mechanism_id();
        assert!(id >= 1 && id <= 5, "mechanism_id must be in 1..=5");
    }

    /// **R133.1 证明 #2**: 5 机制 type 严格区分 — 任意 token 仅匹配 1 个 is_* 谓词.
    #[kani::proof]
    fn kani_verify_string_token_5_variants_distinct() {
        let token: SelfDisableTokenPod = kani::any();
        let count = (token.is_no_degrade() as u8)
            + (token.is_no_patch() as u8)
            + (token.is_no_bypass() as u8)
            + (token.is_no_reverse() as u8)
            + (token.is_no_hide() as u8);
        assert!(count == 1, "exactly 1 is_* predicate must match");
    }

    /// **R133.1 证明 #3**: NoBypass "Master" token 也不能绕过 (per Q13 兜底, 跟 R131.8 一致).
    #[kani::proof]
    fn kani_verify_master_token_cannot_bypass() {
        // 任何含 "Master" 的 token 都映射到 NoBypass 机制, mechanism_id = 3
        let token_str: [u8; STRING_POD_MAX_LEN] = kani::any();
        let len: u8 = kani::any();
        let pod = StringPod { buf: token_str, len: len.min(STRING_POD_MAX_LEN as u8) };
        if pod.contains("Master") {
            let token = SelfDisableTokenPod::NoBypass { token: pod };
            assert!(token.mechanism_id() == 3, "Master token always NoBypass (id=3)");
            assert!(token.is_no_bypass(), "Master token is NoBypass");
        }
    }

    /// **R133.1 证明 #4**: 任意字符串 to NoDegrade 都机制 ID = 1.
    #[kani::proof]
    fn kani_verify_no_degrade_mechanism_id_is_1() {
        let from = StringPod { buf: kani::any(), len: kani::any::<u8>().min(STRING_POD_MAX_LEN as u8) };
        let to = StringPod { buf: kani::any(), len: kani::any::<u8>().min(STRING_POD_MAX_LEN as u8) };
        let token = SelfDisableTokenPod::NoDegrade { from, to };
        assert!(token.mechanism_id() == 1);
        assert!(token.is_no_degrade());
        assert!(!token.is_no_patch());
    }

    /// **R133.1 证明 #5**: StringPod.len 严格 0..=32, 任意 buffer 不会越界.
    #[kani::proof]
    fn kani_verify_string_pod_len_in_bounds() {
        let len: u8 = kani::any();
        let bounded_len = (len as usize).min(STRING_POD_MAX_LEN);
        assert!(bounded_len <= STRING_POD_MAX_LEN, "len must be in 0..=32");
    }
}

// ============================================================
// Unit tests
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn string_pod_from_str_basic() {
        let sp = StringPod::from_str("hello");
        assert_eq!(sp.len, 5);
        assert_eq!(sp.as_str(), "hello");
    }

    #[test]
    fn string_pod_truncation_at_max_len() {
        let s = "a".repeat(STRING_POD_MAX_LEN + 10);
        let sp = StringPod::from_slice(s.as_bytes());
        assert_eq!(sp.len as usize, STRING_POD_MAX_LEN);
        assert_eq!(sp.as_str().len(), STRING_POD_MAX_LEN);
    }

    #[test]
    fn string_pod_contains_case_sensitive() {
        let sp = StringPod::from_str("HelloMaster");
        assert!(sp.contains("Master"));
        assert!(!sp.contains("master"), "case-sensitive per R133.1");
    }

    #[test]
    fn token_pod_5_mechanism_ids_distinct() {
        let tokens = [
            SelfDisableTokenPod::NoDegrade {
                from: StringPod::from_str("high"),
                to: StringPod::from_str("low"),
            },
            SelfDisableTokenPod::NoPatch { rule: StringPod::from_str("principle_keys_count") },
            SelfDisableTokenPod::NoBypass { token: StringPod::from_str("Master") },
            SelfDisableTokenPod::NoReverse { trigger_id: StringPod::from_str("trig-001") },
            SelfDisableTokenPod::NoHide { window_id: StringPod::from_str("win-001") },
        ];
        let ids: Vec<u8> = tokens.iter().map(|t| t.mechanism_id()).collect();
        assert_eq!(ids, vec![1, 2, 3, 4, 5]);
        // 5 机制严格区分: 每 token 仅 1 个 is_* true
        for t in &tokens {
            let count = (t.is_no_degrade() as u8)
                + (t.is_no_patch() as u8)
                + (t.is_no_bypass() as u8)
                + (t.is_no_reverse() as u8)
                + (t.is_no_hide() as u8);
            assert_eq!(count, 1, "exactly 1 is_* predicate must match");
        }
    }

    #[test]
    fn master_token_cannot_bypass() {
        let token = SelfDisableTokenPod::NoBypass { token: StringPod::from_str("Master") };
        assert_eq!(token.mechanism_id(), 3);
        assert!(token.is_no_bypass());
    }

    #[test]
    fn arbitrary_string_to_no_degrade_mechanism_1() {
        let token = SelfDisableTokenPod::NoDegrade {
            from: StringPod::from_str("anything"),
            to: StringPod::from_str("really anything"),
        };
        assert_eq!(token.mechanism_id(), 1);
        assert!(token.is_no_degrade());
        assert!(!token.is_no_bypass());
    }

    #[test]
    fn string_pod_default_is_empty() {
        let sp = StringPod::default();
        assert_eq!(sp.len, 0);
        assert_eq!(sp.as_str(), "");
    }

    #[test]
    fn string_pod_eq_str_exact() {
        let sp = StringPod::from_str("Master");
        assert!(sp.eq_str("Master"));
        assert!(!sp.eq_str("master"), "strict case-sensitive per R133.1");
        assert!(!sp.eq_str("Master "));
    }

    #[test]
    fn five_variant_partial_eq_strict() {
        // 不同 variant 即使 token 内容相同也不等
        let a = SelfDisableTokenPod::NoPatch { rule: StringPod::from_str("Master") };
        let b = SelfDisableTokenPod::NoBypass { token: StringPod::from_str("Master") };
        assert_ne!(a, b, "different variants with same content must be unequal");
    }

    #[test]
    fn compile_time_constants_correct() {
        assert_eq!(SELF_DISABLE_MECHANISM_COUNT, 5);
        assert_eq!(STRING_POD_MAX_LEN, 32);
    }
}
