//! # 凭证 Scope 权限 (5 级别)
//!
//! 1:1 翻译 v0.9.21 商业版 RBAC 权限模型. 5 Scope 级别按从低到高排列:
//! `Read` < `Write` < `Admin` < `Owner` < `Root`.
//!
//! ## 5 Scope 级别 (K-1 强校验 #1: 编译期 hardcode, 不可运行时增删)
//!
//! | 级别 | 数值 | 权限 | 典型操作 |
//! |------|------|------|---------|
//! | `Read` | 1 | 只读 | list / get |
//! | `Write` | 2 | 写 | create / update |
//! | `Admin` | 3 | 管理 | delete / config |
//! | `Owner` | 4 | 所有者 | transfer / billing |
//! | `Root` | 5 | 全权 | account delete / root credential revoke |
//!
//! ## 设计原则 (per S-2 实事求是 + O-5 不假装)
//!
//! 1. **5 级别编译期 hardcode**: 不可运行时增删, 防 m3 hallucination 注入新 scope
//! 2. **顺序固定**: Read < Write < Admin < Owner < Root, 不可重排
//! 3. **越权检查**: `Scope::can_perform(current, required)` 返 bool, 失败返 `CredentialsError::InsufficientScope`
//! 4. **1:1 翻译 RBAC**: 借鉴 Kubernetes RBAC / AWS IAM policy 5 级标准
//!
//! ## 6 哲学 anchor 穿透
//!
//! - **S-1 北极星导向**: 5 Scope 1:1 翻译 RBAC 行业标准, 0 业务重设计
//! - **S-2 实事求是**: 5 级别够用, 不发明 `ReadWrite` / `SuperRoot` 等花哨组合
//! - **O-2 走在前人肩上**: 借鉴 Kubernetes RBAC `get/list/watch` 隐式 Read 模式
//! - **O-3 干到底**: 5 枚举 + can_perform 守门 + 5 fixture 测试
//! - **O-4 任何人都能接手**: 跟 keyring / i18n 同模式 (enum + Display + Default)
//! - **O-5 不假装**: 5 级别穷举 match, 0 任何 `unknown_scope` 漏防

use serde::{Deserialize, Serialize};

use crate::error::{CredentialsError, CredentialsResult};

// ============================================================================
// §1 Scope 枚举 (5 级别, 编译期 hardcode)
// ============================================================================

/// 凭证 scope 权限级别 (5 级别, K-1 强校验 #1).
///
/// 顺序固定: `Read` < `Write` < `Admin` < `Owner` < `Root`.
/// 不可运行时增删, 不可重排.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Scope {
    /// **默认**: `Read` 只读 (e.g. list / get).
    #[default]
    Read = 1,
    /// `Write` 写 (e.g. create / update).
    Write = 2,
    /// `Admin` 管理 (e.g. delete / config).
    Admin = 3,
    /// `Owner` 所有者 (e.g. transfer / billing).
    Owner = 4,
    /// `Root` 全权 (e.g. account delete / root credential revoke).
    Root = 5,
}

impl Scope {
    /// scope 字符串 (snake_case, 跟 serde rename_all 对齐).
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::Write => "write",
            Self::Admin => "admin",
            Self::Owner => "owner",
            Self::Root => "root",
        }
    }

    /// scope 数值 (1-5, 跟 enum 显式赋值对齐).
    #[must_use]
    pub fn level(self) -> u8 {
        match self {
            Self::Read => 1,
            Self::Write => 2,
            Self::Admin => 3,
            Self::Owner => 4,
            Self::Root => 5,
        }
    }

    /// 从字符串解析 (snake_case, 区分大小写).
    pub fn parse(s: &str) -> CredentialsResult<Self> {
        match s {
            "read" => Ok(Self::Read),
            "write" => Ok(Self::Write),
            "admin" => Ok(Self::Admin),
            "owner" => Ok(Self::Owner),
            "root" => Ok(Self::Root),
            other => Err(CredentialsError::UnknownScope(other.to_string())),
        }
    }

    /// **越权检查**: 当前 scope 是否能执行 required scope 操作.
    ///
    /// - `current.level() >= required.level()` → 允许
    /// - 否则 → `Err(CredentialsError::InsufficientScope)`
    pub fn can_perform(current: Scope, required: Scope) -> CredentialsResult<()> {
        if current.level() >= required.level() {
            Ok(())
        } else {
            Err(CredentialsError::InsufficientScope {
                current: current.as_str().to_string(),
                required: required.as_str().to_string(),
            })
        }
    }

    /// 5 Scope 全部 (按从低到高顺序, 编译期 hardcode).
    #[must_use]
    pub const fn all() -> [Scope; 5] {
        [Self::Read, Self::Write, Self::Admin, Self::Owner, Self::Root]
    }
}

impl std::fmt::Display for Scope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for Scope {
    type Err = CredentialsError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

// ============================================================================
// §2 编译期守门 (K-1 强校验 + 5 Scope 对齐)
// ============================================================================

/// 5 Scope 编译期常量 (K-1 强校验 #1).
pub const ALL_SCOPES: &[Scope] = &[
    Scope::Read,
    Scope::Write,
    Scope::Admin,
    Scope::Owner,
    Scope::Root,
];

/// 编译期守门: ALL_SCOPES 长度 == 5 (K-1 强校验 #1).
pub const SCOPE_COUNT: usize = 5;
const _: () = assert!(ALL_SCOPES.len() == SCOPE_COUNT);
const _: () = assert!(Scope::all().len() == SCOPE_COUNT);

/// 编译期守门: Scope enum 5 变体.
pub const SCOPE_VARIANT_NAMES: &[&str] = &["Read", "Write", "Admin", "Owner", "Root"];
const _: () = assert!(SCOPE_VARIANT_NAMES.len() == SCOPE_COUNT);

// ============================================================================
// §3 单元测试 (K-1 强校验 + 越权检查)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scope_5_levels() {
        // 5 Scope 全部存在, 顺序固定
        assert_eq!(Scope::all().len(), 5);
        assert_eq!(Scope::Read.level(), 1);
        assert_eq!(Scope::Write.level(), 2);
        assert_eq!(Scope::Admin.level(), 3);
        assert_eq!(Scope::Owner.level(), 4);
        assert_eq!(Scope::Root.level(), 5);
    }

    #[test]
    fn test_scope_ordering() {
        // Read < Write < Admin < Owner < Root
        assert!(Scope::Read < Scope::Write);
        assert!(Scope::Write < Scope::Admin);
        assert!(Scope::Admin < Scope::Owner);
        assert!(Scope::Owner < Scope::Root);
    }

    #[test]
    fn test_scope_default_is_read() {
        assert_eq!(Scope::default(), Scope::Read);
    }

    #[test]
    fn test_scope_as_str_snake_case() {
        assert_eq!(Scope::Read.as_str(), "read");
        assert_eq!(Scope::Write.as_str(), "write");
        assert_eq!(Scope::Admin.as_str(), "admin");
        assert_eq!(Scope::Owner.as_str(), "owner");
        assert_eq!(Scope::Root.as_str(), "root");
    }

    #[test]
    fn test_scope_parse_roundtrip() {
        for scope in Scope::all() {
            let s = scope.as_str();
            let parsed = Scope::parse(s).expect("parse must succeed");
            assert_eq!(parsed, scope);
        }
    }

    #[test]
    fn test_scope_parse_unknown_returns_error() {
        // 5 Scope 外的值返 UnknownScope
        let err = Scope::parse("super_root").unwrap_err();
        assert!(matches!(err, CredentialsError::UnknownScope(_)));
        assert!(err.to_string().contains("super_root"));
    }

    #[test]
    fn test_scope_can_perform_higher_can_do_lower() {
        // Root 可以做任何事
        for required in Scope::all() {
            Scope::can_perform(Scope::Root, required).expect("Root can do anything");
        }
        // Owner 可以 Read/Write/Admin/Owner (不能 Root)
        for required in &[Scope::Read, Scope::Write, Scope::Admin, Scope::Owner] {
            Scope::can_perform(Scope::Owner, *required).expect("Owner can do lower scopes");
        }
        assert!(Scope::can_perform(Scope::Owner, Scope::Root).is_err());
    }

    #[test]
    fn test_scope_can_perform_lower_cannot_do_higher() {
        // Read 不能 Write
        let err = Scope::can_perform(Scope::Read, Scope::Write).unwrap_err();
        assert!(matches!(err, CredentialsError::InsufficientScope { .. }));
        // Read 不能 Admin
        let err = Scope::can_perform(Scope::Read, Scope::Admin).unwrap_err();
        assert!(matches!(err, CredentialsError::InsufficientScope { .. }));
        // Write 不能 Admin
        assert!(Scope::can_perform(Scope::Write, Scope::Admin).is_err());
        // Write 不能 Root
        assert!(Scope::can_perform(Scope::Write, Scope::Root).is_err());
    }

    #[test]
    fn test_scope_can_perform_same_is_ok() {
        // 同级别允许 (e.g. Write 试图 Write)
        for scope in Scope::all() {
            Scope::can_perform(scope, scope).expect("same scope must be allowed");
        }
    }

    #[test]
    fn test_scope_serde_roundtrip() {
        for scope in Scope::all() {
            let json = serde_json::to_string(&scope).expect("serialize");
            let parsed: Scope = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(parsed, scope);
        }
    }
}
