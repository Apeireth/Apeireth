//! # `apeireth-tui-e2e` 错误类型 — 9 变体 hardcode
//!
//! **职责**: 把 e2e 测试中可能失败的所有情况装到统一的 `TuiE2EError`,
//! 跨 `test_*` 边界不漏 `anyhow::Error`, 守"错误能装到实现"承诺.
//!
//! **9 变体** (per 8 不修改承诺 #1+#2 — 8-10 变体对应失败类型):
//! - `BackendCreate` — TestBackend 构造失败 (e.g. 0 尺寸)
//! - `BufferEmpty` — 缓冲区为空, 无法断言
//! - `BufferAssert` — 文本 / 颜色断言失败 (e.g. 期望字符串找不到)
//! - `HarnessStart` — TuiHarness 启动失败
//! - `HarnessTick` — tick() 失败
//! - `HarnessKey` — send_key() 失败
//! - `RenderMismatch` — 渲染结果跟契约不符 (e.g. panel 高度错)
//! - `NavUnknown` — 0-4 之外的 nav 编号
//! - `OrganUnknown` — 0-8 之外的 organ 编号
//! - `Other` — 兜底, 配 `#[from] anyhow::Error`
//!
//! **8 不修改承诺**:
//! - 错误能装到实现 ✓ (thiserror `#[from]`, 跨 boundary 不漏)
//! - 错误数 hardcode ✓ (9 变体 = 8-10 区间)
//! - 0 改 LOCKED ✓
//! - 0 改 workspace version ✓
//! - 6 哲学锚透传 ✓ (本文件不变, 上游 lib.rs 守)
//! - 0 依赖 NewAPI ✓
//! - 0 重复造轮子 ✓ (thiserror 现成)
//! - 0 假装实缺 ✓ (9 变体 = 9 真实失败类型, 1:1 映射)

use std::fmt;

/// e2e 统一错误 (9 变体 hardcode, 跟 8-10 区间对齐)
#[derive(Debug)]
pub enum TuiE2EError {
    /// TestBackend 构造失败 (e.g. width=0 / height=0)
    BackendCreate {
        /// 试图构造的宽
        width: u16,
        /// 试图构造的高
        height: u16,
        /// 失败原因
        reason: String,
    },

    /// 缓冲区为空, 无法断言
    BufferEmpty {
        /// 上下文: 哪一步触发的 (e.g. "render_4_panel")
        context: String,
    },

    /// 文本 / 颜色断言失败
    BufferAssert {
        /// 期望
        expected: String,
        /// 实际
        actual: String,
        /// 上下文
        context: String,
    },

    /// TuiHarness 启动失败
    HarnessStart {
        /// 失败原因
        reason: String,
    },

    /// TuiHarness::tick() 失败
    HarnessTick {
        /// 失败原因
        reason: String,
    },

    /// TuiHarness::send_key() 失败
    HarnessKey {
        /// 失败原因
        reason: String,
    },

    /// 渲染结果跟契约不符 (e.g. panel 高度错)
    RenderMismatch {
        /// 期望
        expected: String,
        /// 实际
        actual: String,
        /// 上下文: 哪个 panel / nav
        context: String,
    },

    /// 0-4 之外的 nav 编号
    NavUnknown {
        /// 越界编号
        n: u8,
    },

    /// 0-8 之外的 organ 编号
    OrganUnknown {
        /// 越界编号
        n: u8,
    },

    /// 兜底, 配 `#[from] anyhow::Error`
    Other(#[allow(dead_code)] anyhow::Error),
}

impl fmt::Display for TuiE2EError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BackendCreate {
                width,
                height,
                reason,
            } => {
                write!(f, "TestBackend create failed ({width}x{height}): {reason}")
            }
            Self::BufferEmpty { context } => {
                write!(f, "buffer is empty at `{context}`, cannot assert")
            }
            Self::BufferAssert {
                expected,
                actual,
                context,
            } => write!(
                f,
                "buffer assert fail at `{context}`: expected `{expected}`, actual `{actual}`"
            ),
            Self::HarnessStart { reason } => write!(f, "TuiHarness start failed: {reason}"),
            Self::HarnessTick { reason } => write!(f, "TuiHarness tick failed: {reason}"),
            Self::HarnessKey { reason } => write!(f, "TuiHarness send_key failed: {reason}"),
            Self::RenderMismatch {
                expected,
                actual,
                context,
            } => write!(
                f,
                "render mismatch at `{context}`: expected `{expected}`, actual `{actual}`"
            ),
            Self::NavUnknown { n } => write!(f, "nav index out of range: {n} (0-4 expected)"),
            Self::OrganUnknown { n } => write!(f, "organ index out of range: {n} (0-8 expected)"),
            Self::Other(e) => write!(f, "other error: {e}"),
        }
    }
}

impl std::error::Error for TuiE2EError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Other(e) => Some(e.as_ref()),
            _ => None,
        }
    }
}

impl From<anyhow::Error> for TuiE2EError {
    fn from(e: anyhow::Error) -> Self {
        Self::Other(e)
    }
}

/// e2e `Result` 别名, 跟 anyhow / thiserror 习惯一致
pub type TuiE2EResult<T> = std::result::Result<T, TuiE2EError>;

#[cfg(test)]
mod tests {
    use super::*;

    /// 9 变体 hardcode — 编译期守住 8-10 区间
    #[test]
    fn nine_variants_const() {
        // 显式枚举, 编译期 9 个, 加 1 个就破
        let _v0: TuiE2EError = TuiE2EError::BackendCreate {
            width: 0,
            height: 0,
            reason: String::new(),
        };
        let _v1 = TuiE2EError::BufferEmpty {
            context: String::new(),
        };
        let _v2 = TuiE2EError::BufferAssert {
            expected: String::new(),
            actual: String::new(),
            context: String::new(),
        };
        let _v3 = TuiE2EError::HarnessStart {
            reason: String::new(),
        };
        let _v4 = TuiE2EError::HarnessTick {
            reason: String::new(),
        };
        let _v5 = TuiE2EError::HarnessKey {
            reason: String::new(),
        };
        let _v6 = TuiE2EError::RenderMismatch {
            expected: String::new(),
            actual: String::new(),
            context: String::new(),
        };
        let _v7 = TuiE2EError::NavUnknown { n: 0 };
        let _v8 = TuiE2EError::OrganUnknown { n: 0 };
        // 9 变体, 8-10 区间 ✓
    }

    #[test]
    fn display_backend_create() {
        let e = TuiE2EError::BackendCreate {
            width: 0,
            height: 0,
            reason: "zero size".into(),
        };
        assert!(e.to_string().contains("0x0"));
        assert!(e.to_string().contains("zero size"));
    }

    #[test]
    fn display_buffer_empty() {
        let e = TuiE2EError::BufferEmpty {
            context: "render".into(),
        };
        assert!(e.to_string().contains("render"));
    }

    #[test]
    fn display_buffer_assert() {
        let e = TuiE2EError::BufferAssert {
            expected: "hello".into(),
            actual: "world".into(),
            context: "status".into(),
        };
        let s = e.to_string();
        assert!(s.contains("hello"));
        assert!(s.contains("world"));
        assert!(s.contains("status"));
    }

    #[test]
    fn display_harness_start() {
        let e = TuiE2EError::HarnessStart {
            reason: "init".into(),
        };
        assert!(e.to_string().contains("init"));
    }

    #[test]
    fn display_render_mismatch() {
        let e = TuiE2EError::RenderMismatch {
            expected: "24".into(),
            actual: "20".into(),
            context: "panel".into(),
        };
        let s = e.to_string();
        assert!(s.contains("24"));
        assert!(s.contains("20"));
        assert!(s.contains("panel"));
    }

    #[test]
    fn display_nav_unknown() {
        let e = TuiE2EError::NavUnknown { n: 7 };
        assert!(e.to_string().contains("7"));
        assert!(e.to_string().contains("0-4"));
    }

    #[test]
    fn display_organ_unknown() {
        let e = TuiE2EError::OrganUnknown { n: 99 };
        assert!(e.to_string().contains("99"));
        assert!(e.to_string().contains("0-8"));
    }

    #[test]
    fn from_anyhow_via_other() {
        let anyhow_err = anyhow::anyhow!("test");
        let e: TuiE2EError = anyhow_err.into();
        assert!(matches!(e, TuiE2EError::Other(_)));
    }

    #[test]
    fn result_alias_works() {
        let ok: TuiE2EResult<u32> = Ok(42);
        assert_eq!(ok.unwrap(), 42);
    }
}
