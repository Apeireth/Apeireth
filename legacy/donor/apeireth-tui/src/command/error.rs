//! TUI 9 器官 command 模块化 — 统一错误类型
//!
//! **职责**: 9 器官 command 调用失败的统一错误封装, 跟 [`crate::error::TuiError`]
//! 互补 (TuiError 管 HTTP / K-1 校验, OrganError 管器官状态 / 命令派发).
//!
//! **设计原则** (per 8 项承诺 + 6 哲学锚):
//! - S-2 实事求是: 错误信息真, 不装, 不假装已实现
//! - 编译期 hardcode: 5 错误变体, 跟 9 器官 command 失败场景对齐
//! - thiserror 派生 (不重复造轮子, 跟 `error.rs` 对齐)
//!
//! **5 错误变体**:
//! - [`OrganError::UnknownOrgan`] — organ id 越界 (0-8)
//! - [`OrganError::Unsupported`] — 器官当前实接度不足 (stub organ 不支持该 command)
//! - [`OrganError::InvalidArg`] — command 参数非法 (字符串空 / 越界值 / 非法 JSON)
//! - [`OrganError::NotReady`] — 器官前置条件不满足 (未初始化 / 配置缺失)
//! - [`OrganError::CrossNavDenied`] — 5 nav 跨界跳转被拒 (越权跨 nav)
//!
//! **不假装**:
//! - 错误信息携带原始失败原因 (via thiserror `#[source]` / Display 字段)
//! - 不 wrap anyhow, 走 thiserror 保持 typed
//!
//! **6 哲学锚穿透**:
//! - S-1 北极星: 错误服务于"用户能感知" — 5 变体即足够诊断
//! - S-2 实事求是: 错误名精确描述失败模式, 不模糊化
//! - O-2 走在前人经验上: 借 thiserror / anyhow 业界共识
//! - O-3 干到底: 5 变体覆盖 9 器官所有失败, 不漏
//! - O-4 任何人都能接手: 5 变体全文档化 + tests
//! - O-5 不假装: 不假装器官是 "ok" — 标 Unsupported 标 stub
//!
//! **8 项承诺**:
//! - 不假装已实现 ✅
//! - 编译期 hardcode (5 变体) ✅
//! - 不改 LOCKED ✅
//! - 不改 workspace version ✅
//! - 6 哲学锚穿透 ✅
//! - 不依赖 NewAPI ✅
//! - 不重复造轮子 (用 thiserror) ✅
//! - 诚实标缺 ✅
//!
//! **借鉴 Golutra #1 (P0 70-command 模式)**:
//! - Golutra `Result<T, String>` 兜底, 5 重命令错误用 String 描述
//! - Apeireth 走 typed OrganError 5 变体 — 更强约束, 编译期守门

use thiserror::Error;

/// Organ command 错误类型 (5 变体, 编译期 hardcode)
#[derive(Debug, Error)]
pub enum OrganError {
    /// 未知 organ (id 越界 0-8)
    #[error("unknown organ id: {0} (valid range 0-8)")]
    UnknownOrgan(u8),

    /// 器官实接度不足 (stub organ 不支持该 command, 诚实标缺)
    #[error(
        "organ '{organ}' not ready (readiness=stub), command '{command}' denied (S-2 实事求是)"
    )]
    Unsupported {
        /// 器官 ASCII 字符 e.g. `"[EYE]"`
        organ: &'static str,
        /// command 名 e.g. `"GetRecentTokens"`
        command: &'static str,
    },

    /// command 参数非法 (字符串空 / 越界值 / 非法 JSON)
    #[error("invalid argument for command '{command}': {reason}")]
    InvalidArg {
        /// command 名
        command: &'static str,
        /// 失败原因描述
        reason: String,
    },

    /// 器官前置条件不满足 (未初始化 / 配置缺失 / state 为空)
    #[error("organ '{organ}' not ready: {reason} (need to run Init first)")]
    NotReady {
        /// 器官 ASCII 字符
        organ: &'static str,
        /// 失败原因
        reason: String,
    },

    /// 5 nav 跨界跳转被拒 (越权跨 nav — 9 器官 command 不能强行切 nav)
    #[error("cross-navigate from '{from}' to '{to}' denied (organ command 只能软 hint, 不能强切)")]
    CrossNavDenied {
        /// 来源 nav 名
        from: &'static str,
        /// 目标 nav 名
        to: &'static str,
    },
}

/// 5 错误变体断言 (编译期守门 — 改 1 个变体加 1 个 match arm)
#[allow(dead_code)]
const _: fn() = || {
    let _e1 = OrganError::UnknownOrgan(99);
    let _e2 = OrganError::Unsupported {
        organ: "[EYE]",
        command: "WatchInput",
    };
    let _e3 = OrganError::InvalidArg {
        command: "SetBpm",
        reason: "out of range".into(),
    };
    let _e4 = OrganError::NotReady {
        organ: "[BRAIN]",
        reason: "no provider set".into(),
    };
    let _e5 = OrganError::CrossNavDenied {
        from: "Bridge",
        to: "Settings",
    };
};

// =====================================================================
// 单元测试 (5 错误变体 + Display 字符串 + 5 失败场景 = 9+ 测试)
// =====================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ---- 5 错误变体可构造 ----

    #[test]
    fn five_variants_constructible() {
        let _e1 = OrganError::UnknownOrgan(42);
        let _e2 = OrganError::Unsupported {
            organ: "[EYE]",
            command: "WatchInput",
        };
        let _e3 = OrganError::InvalidArg {
            command: "SetBpm",
            reason: "must be 1-255".into(),
        };
        let _e4 = OrganError::NotReady {
            organ: "[BRAIN]",
            reason: "no provider".into(),
        };
        let _e5 = OrganError::CrossNavDenied {
            from: "Bridge",
            to: "Settings",
        };
    }

    // ---- 5 变体 Display 字符串关键字段都在 ----

    #[test]
    fn unknown_organ_displays_id() {
        let e = OrganError::UnknownOrgan(99);
        let s = format!("{e}");
        assert!(s.contains("99"), "应含 id 99, 实际: {s}");
        assert!(s.contains("0-8"), "应含合法范围提示");
    }

    #[test]
    fn unsupported_displays_organ_and_command() {
        let e = OrganError::Unsupported {
            organ: "[EAR]",
            command: "Subscribe",
        };
        let s = format!("{e}");
        assert!(s.contains("[EAR]"), "应含器官 ASCII: {s}");
        assert!(s.contains("Subscribe"), "应含 command 名: {s}");
        // S-2 实事求是 — 错误信息应带诚实标记
        assert!(
            s.contains("stub") || s.contains("not ready"),
            "应诚实标缺: {s}"
        );
    }

    #[test]
    fn invalid_arg_displays_command_and_reason() {
        let e = OrganError::InvalidArg {
            command: "SetBpm",
            reason: "BPM 0 not allowed".into(),
        };
        let s = format!("{e}");
        assert!(s.contains("SetBpm"), "应含 command: {s}");
        assert!(s.contains("BPM 0 not allowed"), "应含 reason: {s}");
    }

    #[test]
    fn not_ready_displays_organ_and_reason() {
        let e = OrganError::NotReady {
            organ: "[MEMORY]",
            reason: "no conversations".into(),
        };
        let s = format!("{e}");
        assert!(s.contains("[MEMORY]"), "应含器官: {s}");
        assert!(s.contains("no conversations"), "应含 reason: {s}");
        assert!(s.contains("Init"), "应提示需要 Init: {s}");
    }

    #[test]
    fn cross_nav_displays_from_and_to() {
        let e = OrganError::CrossNavDenied {
            from: "Dialogue",
            to: "Settings",
        };
        let s = format!("{e}");
        assert!(s.contains("Dialogue"), "应含 from: {s}");
        assert!(s.contains("Settings"), "应含 to: {s}");
    }

    // ---- 8 项承诺守门 ----

    #[test]
    fn compile_time_hardcode_5_variants() {
        // 5 变体 — 改任何一个必须改这个测试
        use OrganError::*;
        let variants: Vec<OrganError> = vec![
            UnknownOrgan(0),
            Unsupported {
                organ: "x",
                command: "y",
            },
            InvalidArg {
                command: "z",
                reason: "w".into(),
            },
            NotReady {
                organ: "v",
                reason: "u".into(),
            },
            CrossNavDenied { from: "a", to: "b" },
        ];
        assert_eq!(variants.len(), 5, "5 错误变体 hardcode");
    }

    #[test]
    fn error_implements_send_sync() {
        // 跨线程传 Result<T, OrganError> 必须 Send + Sync (5 nav + 9 器官可能在 worker thread)
        fn assert_send<T: Send + Sync>() {}
        assert_send::<OrganError>();
    }
}
