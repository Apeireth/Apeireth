//! # Policy Stage — 5 阶段 pipeline 第 2 阶段 (策略)
//!
//! 借鉴 Golutra v0.1.0 `chat_db/pipeline/policy.rs` 思想 (per
//! `analysis\golutra\BORROW_FROM_GOLUTRA.md` §8 P2):
//! - Golutra policy: deny-list / quota / scope check
//! - 本 crate 通用化: 5 重策略 (deny_kind / max_attempt / max_payload_size / scope_check / kind_required)
//!
//! ## 1 例子: Policy 阶段给 `apeireth-sovereignty` MEWG 治理用
//!
//! 用户给 sovereignty 治理写自定义 Policy (示意, **不**引 `apeireth-sovereignty` crate dep):
//!
//! ```ignore
//! // 伪代码 — 阶段 6 skeleton 故意不引 workspace dep, 留 R21 续真接
//! use apeireth_sovereignty::{MewgGate, MEWG_RULES};
//! use apeireth_pipeline_g5::{Stage, StageKind, PipelineError, PipelineMessage};
//!
//! pub struct SovereigntyPolicy {
//!     mewg: MewgGate,
//! }
//!
//! impl Stage<PipelineMessage, PipelineMessage> for SovereigntyPolicy {
//!     fn kind(&self) -> StageKind { StageKind::Policy }
//!     fn name(&self) -> &str { "sovereignty-policy" }
//!
//!     fn process(&self, msg: PipelineMessage) -> Result<PipelineMessage, PipelineError> {
//!         // 1. 调 MEWG 5 重治理检查
//!         self.mewg.check(&msg).map_err(|denial| PipelineError::PolicyDenied {
//!             kind: StageKind::Policy,
//!             reason: format!("MEWG denied: {}", denial),
//!         })?;
//!         Ok(msg)
//!     }
//! }
//! ```
//!
//! ## 编译期守门 (4 项, K-1 强校验)
//!
//! 1. `POLICY_DENY_KINDS` 编译期数组 (4 类: "spam" / "phishing" / "exploit" / "leak")
//! 2. `MAX_POLICY_ATTEMPTS == 3` (重试超过 3 次拒绝, 防无限重试 DoS)
//! 3. `MAX_POLICY_PAYLOAD_SIZE == 16 * 1024` (16 KiB policy body 上限, 防 m3 幻觉大 payload)
//! 4. `kind()` 永远返回 `StageKind::Policy`

use std::collections::HashSet;
use std::fmt;

use crate::error::PipelineError;
use crate::message::{PipelineMessage, MAX_PAYLOAD_LEN};
use crate::stage::{Stage, StageKind};

/// **Hardcode #1**: Policy 默认 deny-list (4 类危险 kind).
///
/// 借鉴 Golutra `chat_db/policy.rs` deny-list (4 类), Apeireth 通用化对齐 4 类.
pub const POLICY_DENY_KINDS: &[&str] = &["spam", "phishing", "exploit", "leak"];

/// **Hardcode #2**: Policy 最大 attempt 次数 (3, 防无限重试 DoS).
pub const MAX_POLICY_ATTEMPTS: u32 = 3;

/// **Hardcode #3**: Policy 检查 payload 上限 (16 KiB, 比通用 MAX_PAYLOAD_LEN 64 KiB 严格).
pub const MAX_POLICY_PAYLOAD_SIZE: usize = 16 * 1024;

/// **Hardcode #4**: Policy require kind 非空 (默认 true).
pub const POLICY_REQUIRE_KIND: bool = true;

/// Default Policy stage (5 重策略: deny_kind / max_attempt / payload_size / scope / kind_required).
#[derive(Debug, Clone)]
pub struct DefaultPolicy {
    /// 额外的 deny-list (跟 POLICY_DENY_KINDS 合并).
    extra_deny_kinds: Vec<String>,
}

impl DefaultPolicy {
    /// 创建默认 Policy stage.
    pub fn new() -> Self {
        Self {
            extra_deny_kinds: Vec::new(),
        }
    }

    /// 添加额外 deny kind (e.g. 用户自定义 "blocked_user_<id>").
    pub fn with_extra_deny_kind(mut self, kind: impl Into<String>) -> Self {
        self.extra_deny_kinds.push(kind.into());
        self
    }

    /// 获取完整 deny list (POLICY_DENY_KINDS + extra).
    pub fn deny_list(&self) -> Vec<&str> {
        let mut list: Vec<&str> = POLICY_DENY_KINDS.to_vec();
        for k in &self.extra_deny_kinds {
            list.push(k.as_str());
        }
        list
    }

    /// 编译期守门: POLICY_DENY_KINDS.len() >= 4.
    pub const fn validate_deny_count() -> bool {
        POLICY_DENY_KINDS.len() >= 4
    }
}

impl Default for DefaultPolicy {
    fn default() -> Self {
        Self::new()
    }
}

impl Stage<PipelineMessage, PipelineMessage> for DefaultPolicy {
    fn kind(&self) -> StageKind {
        StageKind::Policy
    }

    fn name(&self) -> &str {
        "default-policy"
    }

    fn process(&self, input: PipelineMessage) -> Result<PipelineMessage, PipelineError> {
        // 策略 1: kind required (Hardcode #4)
        if POLICY_REQUIRE_KIND && input.kind.trim().is_empty() {
            return Err(PipelineError::PolicyDenied {
                kind: StageKind::Policy,
                reason: "kind field is required (empty)".to_string(),
            });
        }

        // 策略 2: deny-list check
        let deny_set: HashSet<&str> = self.deny_list().into_iter().collect();
        if deny_set.contains(input.kind.as_str()) {
            return Err(PipelineError::PolicyDenied {
                kind: StageKind::Policy,
                reason: format!("kind '{}' in deny list", input.kind),
            });
        }

        // 策略 3: max attempts (Hardcode #2)
        if input.attempt > MAX_POLICY_ATTEMPTS {
            return Err(PipelineError::PolicyDenied {
                kind: StageKind::Policy,
                reason: format!(
                    "attempt {} > MAX_POLICY_ATTEMPTS {}",
                    input.attempt, MAX_POLICY_ATTEMPTS
                ),
            });
        }

        // 策略 4: payload size (Hardcode #3, 比通用 MAX_PAYLOAD_LEN 严格)
        if input.payload.len() > MAX_POLICY_PAYLOAD_SIZE {
            return Err(PipelineError::PolicyDenied {
                kind: StageKind::Policy,
                reason: format!(
                    "payload size {} > MAX_POLICY_PAYLOAD_SIZE {}",
                    input.payload.len(),
                    MAX_POLICY_PAYLOAD_SIZE
                ),
            });
        }

        // 策略 5: scope check (占位, R21 续做 — 当前是"通用放行", 不假装)
        // 借鉴 Golutra `policy.rs` scope check, 实际场景:
        //   - chat: 校验 user role
        //   - task: 校验 priority 在 scope 内
        //   - memory: 校验 access scope
        // 阶段 6 skeleton 留口子, 等 R21 + 某个具体模块接入时 flesh out.
        // (不假装: 当前 0 实现, 加注释明示)
        let _scope_placeholder: () = ();

        // 通用放行
        Ok(input)
    }
}

/// Policy 阶段内部错误 (Box<dyn Error> 透传, 仅用于嵌套 error).
#[derive(Debug)]
pub enum PolicyError {
    /// Payload 超过 MAX_PAYLOAD_LEN (比 MAX_POLICY_PAYLOAD_SIZE 更严格, 是兜底).
    PayloadTooLarge {
        /// 实际大小.
        size: usize,
        /// 上限.
        max: usize,
    },
}

impl fmt::Display for PolicyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PolicyError::PayloadTooLarge { size, max } => {
                write!(f, "payload size {} > hard max {}", size, max)
            }
        }
    }
}

impl std::error::Error for PolicyError {}

/// 编译期字符串相等比较 (per std::str::eq 不是 const-stable, 自实现字节比较).
const fn const_str_eq(a: &str, b: &str) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let ab = a.as_bytes();
    let bb = b.as_bytes();
    let mut i = 0;
    while i < ab.len() {
        if ab[i] != bb[i] {
            return false;
        }
        i += 1;
    }
    true
}

/// 编译期守门: POLICY_DENY_KINDS.len() == 4.
const _: () = assert!(POLICY_DENY_KINDS.len() == 4);
/// 编译期守门: POLICY_DENY_KINDS[0] == "spam".
const _: () = assert!(const_str_eq(POLICY_DENY_KINDS[0], "spam"));
/// 编译期守门: MAX_POLICY_ATTEMPTS == 3.
const _: () = assert!(MAX_POLICY_ATTEMPTS == 3);
/// 编译期守门: MAX_POLICY_PAYLOAD_SIZE == 16 * 1024.
const _: () = assert!(MAX_POLICY_PAYLOAD_SIZE == 16 * 1024);
/// 编译期守门: MAX_POLICY_PAYLOAD_SIZE < MAX_PAYLOAD_LEN (Policy 比通用严格).
const _: () = assert!(MAX_POLICY_PAYLOAD_SIZE < MAX_PAYLOAD_LEN);
/// 编译期守门: POLICY_REQUIRE_KIND == true.
const _: () = assert!(POLICY_REQUIRE_KIND);
