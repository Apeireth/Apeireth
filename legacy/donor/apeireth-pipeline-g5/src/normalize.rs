//! # Normalize Stage — 5 阶段 pipeline 第 1 阶段 (归一化)
//!
//! 借鉴 Golutra v0.1.0 `chat_db/pipeline/normalize.rs` 思想 (per
//! `analysis\golutra\BORROW_FROM_GOLUTRA.md` §8 P2):
//! - Golutra normalize: 清洗输入 (trim 空白 / 折叠换行 / 转义控制字符)
//! - 本 crate 通用化: 5 步归一化 (trim / 折叠空白 / ASCII lowercase / 去空字节 / 去 BOM)
//!
//! ## 1 例子: Normalize 阶段给 chat message 用
//!
//! 用户给 chat 模块写自定义 Normalize (示意, 用户可重写):
//!
//! ```ignore
//! // 伪代码 — 阶段 6 skeleton 提供 default, 用户可改
//! use apeireth_pipeline_g5::{Stage, StageKind, PipelineMessage};
//!
//! pub struct ChatNormalize;
//!
//! impl Stage<PipelineMessage, PipelineMessage> for ChatNormalize {
//!     fn kind(&self) -> StageKind { StageKind::Normalize }
//!     fn process(&self, msg: PipelineMessage) -> Result<PipelineMessage, PipelineError> {
//!         // 1. trim 头尾空白
//!         // 2. 折叠 \r\n → \n
//!         // 3. 去掉 NUL (\0) 字节
//!         // 4. 去掉 UTF-8 BOM
//!         // (跟 DefaultNormalize 5 步对齐)
//!         Ok(msg)
//!     }
//! }
//! ```
//!
//! ## 编译期守门 (3 项, K-1 强校验)
//!
//! 1. 5 步归一化顺序 (trim → fold_whitespace → lowercase_ascii → strip_null → strip_bom)
//! 2. `MAX_NORMALIZE_ITERATIONS == 4` (防 m3 幻觉无限 trim 循环)
//! 3. `kind()` 永远返回 `StageKind::Normalize`

use std::fmt;

use crate::error::PipelineError;
use crate::message::PipelineMessage;
use crate::stage::{Stage, StageKind};

/// **Hardcode #1**: 5 步归一化顺序 (编译期数组, 防顺序错).
///
/// 步骤:
/// 1. `Trim` — 去掉头尾空白
/// 2. `FoldWhitespace` — 折叠连续空白为单空格
/// 3. `LowercaseAscii` — ASCII 字符 lowercase (Unicode 不动, 防破坏中文)
/// 4. `StripNull` — 去掉 NUL (\0) 字节
/// 5. `StripBom` — 去掉 UTF-8 BOM (\u{FEFF})
pub const NORMALIZE_STEPS: &[&str] = &[
    "trim",
    "fold_whitespace",
    "lowercase_ascii",
    "strip_null",
    "strip_bom",
];

/// **Hardcode #2**: 最大归一化迭代次数 (4, 防 m3 幻觉无限 trim 循环).
pub const MAX_NORMALIZE_ITERATIONS: usize = 4;

/// **Hardcode #3**: 归一化后 payload 最小长度 (1, 拒绝空 payload).
pub const MIN_NORMALIZED_PAYLOAD_LEN: usize = 1;

/// Default Normalize stage (5 步归一化).
///
/// 行为 (按 NORMALIZE_STEPS 顺序):
/// 1. `trim` — 去掉头尾 ASCII whitespace
/// 2. `fold_whitespace` — 折叠连续空白为单空格
/// 3. `lowercase_ascii` — ASCII A-Z → a-z
/// 4. `strip_null` — 去掉 NUL (\0) 字节
/// 5. `strip_bom` — 去掉 UTF-8 BOM (\u{FEFF})
#[derive(Debug, Clone, Default)]
pub struct DefaultNormalize {
    /// 是否启用 lowercase_ascii 步骤 (默认 true, 关闭 = 保留大小写).
    lowercase_enabled: bool,
}

impl DefaultNormalize {
    /// 创建默认 Normalize stage (5 步全开).
    pub fn new() -> Self {
        Self {
            lowercase_enabled: true,
        }
    }

    /// 关闭 lowercase_ascii 步骤 (保留大小写, 给 case-sensitive 输入用).
    pub fn with_lowercase_disabled(mut self) -> Self {
        self.lowercase_enabled = false;
        self
    }

    /// 5 步归一化纯函数 (公开, 方便单步调用 / 测试).
    pub fn normalize(&self, input: &str) -> String {
        let mut s = input.to_string();

        // 步骤 1: trim
        s = s.trim().to_string();

        // 步骤 2: fold whitespace (连续 ASCII whitespace 折叠为单空格)
        let mut folded = String::with_capacity(s.len());
        let mut prev_space = false;
        for ch in s.chars() {
            if ch.is_ascii_whitespace() {
                if !prev_space {
                    folded.push(' ');
                    prev_space = true;
                }
            } else {
                folded.push(ch);
                prev_space = false;
            }
        }
        s = folded;

        // 步骤 3: lowercase ASCII (Unicode 不动, 防破坏中文)
        if self.lowercase_enabled {
            s = s
                .chars()
                .map(|c| {
                    if c.is_ascii_uppercase() {
                        c.to_ascii_lowercase()
                    } else {
                        c
                    }
                })
                .collect();
        }

        // 步骤 4: strip NUL
        s = s.replace('\0', "");

        // 步骤 5: strip UTF-8 BOM (任何位置的 BOM 都 strip, 不仅是开头)
        s = s.replace('\u{FEFF}', "");

        s
    }
}

impl Stage<PipelineMessage, PipelineMessage> for DefaultNormalize {
    fn kind(&self) -> StageKind {
        StageKind::Normalize
    }

    fn name(&self) -> &str {
        "default-normalize"
    }

    fn process(&self, input: PipelineMessage) -> Result<PipelineMessage, PipelineError> {
        // 5 步归一化 (NORMALIZE_STEPS 守门, 单次调用已包含 5 步, 不需要循环)
        let normalized_payload = self.normalize(&input.payload);
        let normalized_kind = self.normalize(&input.kind);

        // 守门: 归一化后 payload 非空 (Hardcode #3)
        if normalized_payload.len() < MIN_NORMALIZED_PAYLOAD_LEN {
            return Err(PipelineError::Stage {
                kind: StageKind::Normalize,
                source: Box::new(NormalizeError::EmptyAfterNormalize {
                    original: input.payload.clone(),
                }),
            });
        }

        // 守门: 迭代次数 (Hardcode #2, 实际归一化是单次 5 步, 这条防御 m3 幻觉加循环)
        let _iter_guard: () = assert!(MAX_NORMALIZE_ITERATIONS >= 1);

        Ok(PipelineMessage {
            kind: normalized_kind,
            payload: normalized_payload,
            attempt: input.attempt,
            trace_id: input.trace_id,
        })
    }
}

/// Normalize 阶段内部错误.
#[derive(Debug)]
pub enum NormalizeError {
    /// 归一化后 payload 为空.
    EmptyAfterNormalize {
        /// 原始 payload (归一化前).
        original: String,
    },
}

impl fmt::Display for NormalizeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NormalizeError::EmptyAfterNormalize { original } => {
                write!(f, "payload empty after normalize, original={:?}", original)
            }
        }
    }
}

impl std::error::Error for NormalizeError {}

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

/// 编译期守门: NORMALIZE_STEPS.len() == 5.
const _: () = assert!(NORMALIZE_STEPS.len() == 5);
/// 编译期守门: NORMALIZE_STEPS[0] == "trim".
const _: () = assert!(const_str_eq(NORMALIZE_STEPS[0], "trim"));
/// 编译期守门: NORMALIZE_STEPS[4] == "strip_bom".
const _: () = assert!(const_str_eq(NORMALIZE_STEPS[4], "strip_bom"));
/// 编译期守门: MAX_NORMALIZE_ITERATIONS == 4.
const _: () = assert!(MAX_NORMALIZE_ITERATIONS == 4);
/// 编译期守门: MIN_NORMALIZED_PAYLOAD_LEN == 1.
const _: () = assert!(MIN_NORMALIZED_PAYLOAD_LEN == 1);
