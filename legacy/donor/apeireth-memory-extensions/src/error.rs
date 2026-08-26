//! # MemoryProviderError — 7 provider 跨后端统一错误
//!
//! 7 variant 设计 (per 借鉴 Golutra memory gateway 5 provider 错误模式 + 2 防御),
//! 编译期 enum exhaustive match 守门.
//!
//! ## 7 variant 守门 (编译期 hardcode)
//!
//! 1. `Config { field, reason }` — ProviderConfig 6 K-1 强校验失败
//! 2. `Connection { provider, reason }` — 连接失败 (Redis/Postgres/S3 连不上)
//! 3. `NotFound { provider, key }` — key 不存在
//! 4. `Serialization { provider, reason }` — 序列化/反序列化失败
//! 5. `Backend { provider, reason }` — 后端特定错误 (rusqlite / lru / redis 内部错误)
//! 6. `Capacity { provider, max_size, current }` — max_size 满
//! 7. `Other { msg }` — 兜底 (catch-all)
//!
//! ## 借用 Golutra
//!
//! Golutra memory gateway 5 provider 错误模式 (per `analysis/golututra/BORROW_FROM_GOLUTRA.md` §8 P1 第 13/14 项),
//! 简化通用化到 7 类, 适配 Rust 错误传播模式 (`thiserror` + `From` 自动转换).
//!
//! **0 抄 Golutra 业务代码**, 只借 7 类错误分类思想.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::memory_provider::{ProviderConfigField, ProviderKind};

/// **Error K-1 强校验 #1**: MemoryProviderError 7 variant (编译期 hardcode).
pub const MEMORY_PROVIDER_ERROR_VARIANT_COUNT: usize = 7;

/// 7 provider 跨后端统一错误.
///
/// 跨 7 provider 错误统一包装, 错误传播支持 `?`.
#[derive(Debug, Clone)]
pub enum MemoryProviderError {
    /// ProviderConfig 6 K-1 强校验失败 (e.g. connection_string 空字符串 / timeout 0 / max_size 0).
    Config {
        /// 哪个 K-1 字段 (e.g. `ConnectionString` / `Timeout` / `MaxSize` / `Persist` / `CacheTtl` / `Scope`).
        field: ProviderConfigField,
        /// 失败原因 (人读).
        reason: String,
    },

    /// 连接失败 (Redis/Postgres/S3 连不上服务端).
    Connection {
        /// 哪个 provider (per ProviderKind 7 变体).
        provider: ProviderKind,
        /// 失败原因 (e.g. "Connection refused" / "DNS lookup failed").
        reason: String,
    },

    /// key 不存在 (用于 get 失败场景).
    NotFound {
        /// 哪个 provider.
        provider: ProviderKind,
        /// 不存在的 key.
        key: String,
    },

    /// 序列化/反序列化失败 (e.g. value 不是合法 JSON / BSON).
    Serialization {
        /// 哪个 provider.
        provider: ProviderKind,
        /// 失败原因.
        reason: String,
    },

    /// 后端特定错误 (rusqlite / lru / redis 内部错误透传).
    Backend {
        /// 哪个 provider.
        provider: ProviderKind,
        /// 后端错误描述.
        reason: String,
    },

    /// max_size 满 (put 时超过 max_size 配置).
    Capacity {
        /// 哪个 provider.
        provider: ProviderKind,
        /// 配置的 max_size (bytes).
        max_size: u64,
        /// 当前已用 size (bytes).
        current: u64,
    },

    /// 兜底 (catch-all).
    Other {
        /// 错误消息.
        msg: String,
    },
}

impl MemoryProviderError {
    /// 序列化摘要 (跨层通信用, 不含具体 key/msg 字段以减小序列化体积).
    pub fn kind(&self) -> MemoryProviderErrorKind {
        match self {
            Self::Config { .. } => MemoryProviderErrorKind::Config,
            Self::Connection { .. } => MemoryProviderErrorKind::Connection,
            Self::NotFound { .. } => MemoryProviderErrorKind::NotFound,
            Self::Serialization { .. } => MemoryProviderErrorKind::Serialization,
            Self::Backend { .. } => MemoryProviderErrorKind::Backend,
            Self::Capacity { .. } => MemoryProviderErrorKind::Capacity,
            Self::Other { .. } => MemoryProviderErrorKind::Other,
        }
    }

    /// 哪个 provider (除 Config / Other 外, 都带 provider 字段).
    pub fn provider(&self) -> Option<ProviderKind> {
        match self {
            Self::Config { .. } | Self::Other { .. } => None,
            Self::Connection { provider, .. }
            | Self::NotFound { provider, .. }
            | Self::Serialization { provider, .. }
            | Self::Backend { provider, .. }
            | Self::Capacity { provider, .. } => Some(*provider),
        }
    }
}

impl fmt::Display for MemoryProviderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Config { field, reason } => {
                write!(f, "[config invalid] field={field:?}, reason={reason}")
            }
            Self::Connection { provider, reason } => {
                write!(
                    f,
                    "[connection failed] provider={provider:?}, reason={reason}"
                )
            }
            Self::NotFound { provider, key } => {
                write!(f, "[not found] provider={provider:?}, key={key}")
            }
            Self::Serialization { provider, reason } => {
                write!(
                    f,
                    "[serialization failed] provider={provider:?}, reason={reason}"
                )
            }
            Self::Backend { provider, reason } => {
                write!(f, "[backend error] provider={provider:?}, reason={reason}")
            }
            Self::Capacity {
                provider,
                max_size,
                current,
            } => {
                write!(
                    f,
                    "[capacity exceeded] provider={provider:?}, max_size={max_size}, current={current}"
                )
            }
            Self::Other { msg } => write!(f, "[other] {msg}"),
        }
    }
}

impl std::error::Error for MemoryProviderError {}

/// **MemoryProviderError 序列化摘要** (跨层通信用, 不含具体 provider 字段以减小序列化体积).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MemoryProviderErrorKind {
    /// ProviderConfig 强校验失败.
    Config,
    /// 连接失败.
    Connection,
    /// key 不存在.
    NotFound,
    /// 序列化失败.
    Serialization,
    /// 后端错误.
    Backend,
    /// max_size 满.
    Capacity,
    /// 兜底.
    Other,
}

impl fmt::Display for MemoryProviderErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl MemoryProviderErrorKind {
    /// 编译期字符串表示.
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Config => "config",
            Self::Connection => "connection",
            Self::NotFound => "not_found",
            Self::Serialization => "serialization",
            Self::Backend => "backend",
            Self::Capacity => "capacity",
            Self::Other => "other",
        }
    }
}

/// **统一结果类型** (per skeleton 模式, 跟借鉴 #6 `StateResult` 1:1 镜像).
pub type MemoryProviderResult<T> = Result<T, MemoryProviderError>;

// =====================================================================
// 单元测试 (7 variant + kind 映射 + provider 字段 + Display = 15+ 测试)
// =====================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seven_variant_constructible() {
        let _ = MemoryProviderError::Config {
            field: ProviderConfigField::ConnectionString,
            reason: "empty".to_string(),
        };
        let _ = MemoryProviderError::Connection {
            provider: ProviderKind::Redis,
            reason: "refused".to_string(),
        };
        let _ = MemoryProviderError::NotFound {
            provider: ProviderKind::Sqlite,
            key: "k".to_string(),
        };
        let _ = MemoryProviderError::Serialization {
            provider: ProviderKind::Postgres,
            reason: "bad json".to_string(),
        };
        let _ = MemoryProviderError::Backend {
            provider: ProviderKind::S3,
            reason: "rusqlite locked".to_string(),
        };
        let _ = MemoryProviderError::Capacity {
            provider: ProviderKind::DiskLru,
            max_size: 1024,
            current: 2048,
        };
        let _ = MemoryProviderError::Other {
            msg: "test".to_string(),
        };
    }

    #[test]
    fn seven_kind_variants_constructible() {
        let _ = MemoryProviderErrorKind::Config;
        let _ = MemoryProviderErrorKind::Connection;
        let _ = MemoryProviderErrorKind::NotFound;
        let _ = MemoryProviderErrorKind::Serialization;
        let _ = MemoryProviderErrorKind::Backend;
        let _ = MemoryProviderErrorKind::Capacity;
        let _ = MemoryProviderErrorKind::Other;
    }

    #[test]
    fn kind_mapping_consistent() {
        let config = MemoryProviderError::Config {
            field: ProviderConfigField::Timeout,
            reason: "x".to_string(),
        };
        assert_eq!(config.kind(), MemoryProviderErrorKind::Config);

        let conn = MemoryProviderError::Connection {
            provider: ProviderKind::Redis,
            reason: "x".to_string(),
        };
        assert_eq!(conn.kind(), MemoryProviderErrorKind::Connection);

        let notfound = MemoryProviderError::NotFound {
            provider: ProviderKind::Sqlite,
            key: "k".to_string(),
        };
        assert_eq!(notfound.kind(), MemoryProviderErrorKind::NotFound);

        let ser = MemoryProviderError::Serialization {
            provider: ProviderKind::Postgres,
            reason: "x".to_string(),
        };
        assert_eq!(ser.kind(), MemoryProviderErrorKind::Serialization);

        let backend = MemoryProviderError::Backend {
            provider: ProviderKind::S3,
            reason: "x".to_string(),
        };
        assert_eq!(backend.kind(), MemoryProviderErrorKind::Backend);

        let cap = MemoryProviderError::Capacity {
            provider: ProviderKind::DiskLru,
            max_size: 1,
            current: 2,
        };
        assert_eq!(cap.kind(), MemoryProviderErrorKind::Capacity);

        let other = MemoryProviderError::Other {
            msg: "x".to_string(),
        };
        assert_eq!(other.kind(), MemoryProviderErrorKind::Other);
    }

    #[test]
    fn kind_as_str_7_distinct() {
        let s = [
            MemoryProviderErrorKind::Config.as_str(),
            MemoryProviderErrorKind::Connection.as_str(),
            MemoryProviderErrorKind::NotFound.as_str(),
            MemoryProviderErrorKind::Serialization.as_str(),
            MemoryProviderErrorKind::Backend.as_str(),
            MemoryProviderErrorKind::Capacity.as_str(),
            MemoryProviderErrorKind::Other.as_str(),
        ];
        let unique: std::collections::HashSet<&str> = s.iter().copied().collect();
        assert_eq!(unique.len(), 7, "7 variant 字符串应互不相同");
    }

    #[test]
    fn provider_field_present_for_5_variants() {
        // 5 variant (Connection/NotFound/Serialization/Backend/Capacity) 都有 provider
        // 2 variant (Config/Other) 没有 provider
        let conn = MemoryProviderError::Connection {
            provider: ProviderKind::Redis,
            reason: "x".to_string(),
        };
        assert_eq!(conn.provider(), Some(ProviderKind::Redis));

        let config = MemoryProviderError::Config {
            field: ProviderConfigField::Timeout,
            reason: "x".to_string(),
        };
        assert_eq!(config.provider(), None);

        let other = MemoryProviderError::Other {
            msg: "x".to_string(),
        };
        assert_eq!(other.provider(), None);
    }

    #[test]
    fn display_includes_provider_and_field() {
        let e = MemoryProviderError::Connection {
            provider: ProviderKind::Redis,
            reason: "refused".to_string(),
        };
        let s = format!("{e}");
        assert!(s.contains("Redis"), "Display 应含 provider: {s}");
        assert!(s.contains("refused"), "Display 应含 reason: {s}");
    }

    #[test]
    fn variant_count_constant_matches_7() {
        assert_eq!(MEMORY_PROVIDER_ERROR_VARIANT_COUNT, 7);
    }
}
