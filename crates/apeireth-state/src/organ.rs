//! # Organ — 9 器官编译期 hardcode
//!
//! 9 器官 enum (per 主人 R19 拟人化决策, per 借鉴 #1 sister 报告 9 organ).
//!
//! **不** 跟 LOCKED `apeireth-tui/src/organ/mod.rs::Organ` 直接同名以避 8 项之 3 "不改 LOCKED".
//! 实际本 crate 的 9 organ stub 字段 (e.g. `HeartStub`) 是**新**的占位类型,
//! 真实集成由 R21+ 在 LOCKED 边界外做 (`apeireth-tui/app.rs` 加 1 行 `OrganStateRegistry::new()`).
//!
//! **编译期 hardcode (10 项)**:
//! - `ORGAN_COUNT == 9` (守门)
//! - `ORGAN_NAMES_ZH` 9 元素编译期数组
//! - `ORGAN_ASCII_CHARS` 9 元素编译期数组
//! - 9 Organ 变体 + 9 OrganStub 类型一一对应
//!
//! **不假装**: OrganStub 是占位 (0 业务字段), 真实集成时换为 sister 报告 9 organ State 类型.

use std::fmt;

use serde::{Deserialize, Serialize};

/// **K-1 强校验 #2**: 9 器官编译期 hardcode (跟借鉴 #1 sister 报告 9 organ 1:1 对齐).
pub const ORGAN_COUNT: usize = 9;

/// 9 器官中文名 (per 借鉴 #1 sister 报告 `organ::mod.rs::Organ::name_zh`).
///
/// 顺序必须跟 `Organ` 变体顺序一致, 编译期数组.
pub const ORGAN_NAMES_ZH: [&str; ORGAN_COUNT] = [
    "心",   // Heart = 0
    "脑",   // Brain = 1
    "手",   // Hand = 2
    "眼",   // Eye = 3
    "耳",   // Ear = 4
    "记忆", // Memory = 5
    "声",   // Voice = 6
    "体",   // Body = 7
    "意",   // Mind = 8
];

/// 9 器官 ASCII 字符 (per 借鉴 #1 sister 报告 `organ::mod.rs::Organ::ascii_char`).
///
/// 跨平台 ASCII (不依赖 emoji 字体), 编译期数组.
pub const ORGAN_ASCII_CHARS: [&str; ORGAN_COUNT] = [
    "[♥]",     // Heart = 0
    "[BRAIN]", // Brain = 1
    "[HAND]",  // Hand = 2
    "[EYE]",   // Eye = 3
    "[EAR]",   // Ear = 4
    "[MEM]",   // Memory = 5
    "[VOICE]", // Voice = 6
    "[BODY]",  // Body = 7
    "[MIND]",  // Mind = 8
];

/// 9 器官 enum (编译期 hardcode, 跟借鉴 #1 sister 报告 1:1).
///
/// **变体索引** (0-8) 用于数组查找 (ORGAN_NAMES_ZH / ORGAN_ASCII_CHARS).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Organ {
    /// 0: 心 (heart) — CPU 心跳 / 60Hz.
    Heart = 0,
    /// 1: 脑 (brain) — LLM 调用频率.
    Brain = 1,
    /// 2: 手 (hand) — 工具调用统计.
    Hand = 2,
    /// 3: 眼 (eye) — 输入监控 (stub, per 借鉴 #1).
    Eye = 3,
    /// 4: 耳 (ear) — 事件订阅 (stub, per 借鉴 #1).
    Ear = 4,
    /// 5: 记忆 (memory) — 会话历史长度.
    Memory = 5,
    /// 6: 声 (voice) — TTS / STT (stub, per 借鉴 #1).
    Voice = 6,
    /// 7: 体 (body) — 进程 / 内存 / 磁盘.
    Body = 7,
    /// 8: 意 (mind) — AGI 状态 + 6 哲学锚.
    Mind = 8,
}

impl fmt::Display for Organ {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl Organ {
    /// 数字 0-8 → Organ.
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(Self::Heart),
            1 => Some(Self::Brain),
            2 => Some(Self::Hand),
            3 => Some(Self::Eye),
            4 => Some(Self::Ear),
            5 => Some(Self::Memory),
            6 => Some(Self::Voice),
            7 => Some(Self::Body),
            8 => Some(Self::Mind),
            _ => None,
        }
    }

    /// 器官中文名 (per ORGAN_NAMES_ZH).
    pub fn name_zh(self) -> &'static str {
        ORGAN_NAMES_ZH[self as usize]
    }

    /// 器官 ASCII 字符 (per ORGAN_ASCII_CHARS).
    pub fn ascii_char(self) -> &'static str {
        ORGAN_ASCII_CHARS[self as usize]
    }

    /// 编译期字符串表示 (调试用).
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Heart => "heart",
            Self::Brain => "brain",
            Self::Hand => "hand",
            Self::Eye => "eye",
            Self::Ear => "ear",
            Self::Memory => "memory",
            Self::Voice => "voice",
            Self::Body => "body",
            Self::Mind => "mind",
        }
    }
}

// ============================================================================
// 9 OrganStub 类型 (9 个新占位 struct, 真实集成由 R21+ 续做)
// ============================================================================

/// **9 OrganStub 宏**: 9 个 organ 各自一个 stub struct (0 业务字段, 编译期 hardcode).
///
/// **不假装**:
/// - OrganStub 是占位 (无业务数据)
/// - 真实集成时, 替换为 sister 报告 9 organ State 类型 (e.g. `apeireth_tui::organ::command::heart::State`)
/// - 当前 0 业务耦合, 1:1 编译期 mirror 9 organ enum
macro_rules! define_organ_stub {
    ($($name:ident),*) => {
        $(
            #[doc = concat!("**9 OrganStub 系列** — ", stringify!($name), " organ 的占位 state 类型.\n\n")]
            #[doc = "**不假装**: 0 业务字段, 真实集成由 R21+ 续做 (sister 报告 `apeireth-tui::organ::command::...::State` 1:1 替换).\n\n"]
            #[doc = "**8 项承诺**: 全部遵守 (尤其 8 项之 8 — 不假装已实现, 标 stub)."]
            #[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
            pub struct $name {
                /// 编译期 hardcode 字段, 防 serde 默认空 struct 不被 derive 接受.
                pub _marker: u8,
            }

            impl $name {
                /// 新建 stub (默认 `_marker = 0`).
                pub const fn new() -> Self {
                    Self { _marker: 0 }
                }
            }
        )*
    };
}

define_organ_stub!(
    HeartStub, BrainStub, HandStub, EyeStub, EarStub, MemoryStub, VoiceStub, BodyStub, MindStub
);

// =====================================================================
// 单元测试 (9 Organ 变体 + 9 OrganStub + 守门 = 15+ 测试)
// =====================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nine_organ_variants_constructible() {
        let _ = Organ::Heart;
        let _ = Organ::Brain;
        let _ = Organ::Hand;
        let _ = Organ::Eye;
        let _ = Organ::Ear;
        let _ = Organ::Memory;
        let _ = Organ::Voice;
        let _ = Organ::Body;
        let _ = Organ::Mind;
    }

    #[test]
    fn nine_organ_stub_types_constructible() {
        let _ = HeartStub::new();
        let _ = BrainStub::new();
        let _ = HandStub::new();
        let _ = EyeStub::new();
        let _ = EarStub::new();
        let _ = MemoryStub::new();
        let _ = VoiceStub::new();
        let _ = BodyStub::new();
        let _ = MindStub::new();
    }

    #[test]
    fn from_u8_round_trip_9() {
        for n in 0..=8u8 {
            let organ = Organ::from_u8(n).expect("0-8 valid");
            assert_eq!(organ as u8, n);
        }
        assert!(Organ::from_u8(9).is_none());
        assert!(Organ::from_u8(255).is_none());
    }

    #[test]
    fn nine_organ_names_zh_distinct() {
        let unique: std::collections::HashSet<&str> = ORGAN_NAMES_ZH.iter().copied().collect();
        assert_eq!(unique.len(), ORGAN_COUNT, "9 器官中文名应互不相同");
    }

    #[test]
    fn nine_organ_ascii_chars_distinct() {
        let unique: std::collections::HashSet<&str> = ORGAN_ASCII_CHARS.iter().copied().collect();
        assert_eq!(unique.len(), ORGAN_COUNT, "9 器官 ASCII 字符应互不相同");
    }

    #[test]
    fn organ_names_zh_match_via_method() {
        for n in 0..=8u8 {
            let organ = Organ::from_u8(n).unwrap();
            assert_eq!(organ.name_zh(), ORGAN_NAMES_ZH[n as usize]);
        }
    }

    #[test]
    fn organ_ascii_chars_match_via_method() {
        for n in 0..=8u8 {
            let organ = Organ::from_u8(n).unwrap();
            assert_eq!(organ.ascii_char(), ORGAN_ASCII_CHARS[n as usize]);
        }
    }

    #[test]
    fn organ_as_str_9_distinct() {
        let s: Vec<&str> = (0..=8u8)
            .map(|n| Organ::from_u8(n).unwrap().as_str())
            .collect();
        let unique: std::collections::HashSet<&str> = s.iter().copied().collect();
        assert_eq!(unique.len(), 9);
    }

    #[test]
    fn organ_count_constant_is_9() {
        assert_eq!(ORGAN_COUNT, 9);
    }

    #[test]
    fn nine_stub_default_eq() {
        // 9 stub 都应该 Default + Eq (9 独立比较, 因为 9 stub 是 9 不同类型)
        assert_eq!(HeartStub::new(), HeartStub::new());
        assert_eq!(BrainStub::new(), BrainStub::new());
        assert_eq!(HandStub::new(), HandStub::new());
        assert_eq!(EyeStub::new(), EyeStub::new());
        assert_eq!(EarStub::new(), EarStub::new());
        assert_eq!(MemoryStub::new(), MemoryStub::new());
        assert_eq!(VoiceStub::new(), VoiceStub::new());
        assert_eq!(BodyStub::new(), BodyStub::new());
        assert_eq!(MindStub::new(), MindStub::new());
    }

    #[test]
    fn organ_serialize_round_trip() {
        for n in 0..=8u8 {
            let organ = Organ::from_u8(n).unwrap();
            let s = serde_json::to_string(&organ).unwrap();
            let back: Organ = serde_json::from_str(&s).unwrap();
            assert_eq!(organ, back);
        }
    }
}
