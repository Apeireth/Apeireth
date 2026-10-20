//! R1b-1a · 神经系统开关单入口（ADR-0009 D3）。
//!
//! 现状（`docs/architecture/r1b-inventory-2026-10-20.md` §3）：9 个生物机制开关
//! 散落在 **17 个 src 文件**里各自解析（`BioAssemblySwitches::from_env` /
//! `bio_tone_wiring_from_env` / `std::env::var(BIO_LIFECYCLE_ENABLE_ENV)` /
//! CLI 痛觉与节律门各自读等）。本模块把「一次解析、一次记录」收进装配根：
//! [`NervousSwitchboard::from_env`] 是**唯一**同时读取这 9 个开关 + 节律时区
//! 参数的位置。
//!
//! R1b-1a 边界：
//! - 只**解析与记录**，不改任何现存解析器的读法（机制迁移在 R1b-1b，生物线）；
//! - 词表 = 既有 env 常量（直接引用各模块常量，不复制字面量）——**不新增开关**；
//! - 语义与现状逐项一致：开关**只认 `"1"`**（其余值 fail-closed = 关，不报错，
//!   与 `bio_switch_enabled_from_env_value` 等既有口径一致）；
//! - 节律时区偏移是唯一**参数**（分钟，±14h，缺省 `0` = UTC，不推断本地时区）：
//!   非法值**拒绝**（返回错误，不静默回退 0）。
//!
//! 唯一仍在本文件保留字面量的 env 名是 [`CIRCADIAN_TZ_OFFSET_ENV`]：它当前只
//! 定义在适配器 `crates/adapters/cli/src/care_heartbeat.rs`（架构方向不允许
//! 装配层反向依赖适配器）。R1b-1b 迁移时 CLI 改为引用本常量，届时字面量收敛为 1 处。

use thiserror::Error;

use crate::canonical::bio_circadian::BIO_CIRCADIAN_ENABLE_ENV;
use crate::canonical::bio_tone::BIO_TONE_ENABLE_ENV;
use crate::canonical::care_heartbeat::bio_lifecycle::BIO_LIFECYCLE_ENABLE_ENV;
use crate::canonical::care_reminder::BIO_CARE_CANDIDATES_ENABLE_ENV;
use crate::canonical::cognitive::{BIO_CONTEXT_POLICY_ENABLE_ENV, BIO_REPAIR_GUIDANCE_ENABLE_ENV};
use crate::canonical::play_tools::BIO_PLAY_ENABLE_ENV;
use crate::canonical::self_status_source::nervous_snapshot::BIO_PAIN_ENABLE_ENV;

/// 节律门时区偏移 env（分钟；显式配置，缺省 `0` = UTC）。
///
/// 与 `crates/adapters/cli/src/care_heartbeat.rs` 的同名常量**同值**；R1b-1b
/// 迁移后以本常量为准（见模块头"唯一字面量"说明）。
pub const CIRCADIAN_TZ_OFFSET_ENV: &str = "APEIRETH_BIO_CIRCADIAN_TZ_OFFSET_MIN";
/// 节律时区偏移缺省值 = `0` 分钟（UTC；显式口径，不推断本地时区）。
pub const DEFAULT_CIRCADIAN_TZ_OFFSET_MIN: i32 = 0;
/// 节律时区偏移允许范围 = ±14 小时（分钟）。
pub const CIRCADIAN_TZ_OFFSET_LIMIT_MIN: i32 = 14 * 60;

/// 生物机制开关词表（一项 = 一个既有 env 名；顺序 = 台账/测试遍历顺序）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BioSwitch {
    /// 生物候选总闸（关怀候选供给、仪式完成面共用）。
    CareCandidates,
    /// 节律门（时区由 [`CIRCADIAN_TZ_OFFSET_ENV`] 单独配置）。
    Circadian,
    /// 策略化上下文块。
    ContextPolicy,
    /// 资产生命周期观测（SG-18）。
    Lifecycle,
    /// 痛觉 / 再生输入（SG-16）。
    Pain,
    /// 游戏练习场工具面。
    Play,
    /// 修复指令段。
    RepairGuidance,
    /// 自述 `capabilities.bio_mechanisms` 子对象。
    SelfStatus,
    /// 调质 → 语气（SG-13）。
    Tone,
}

impl BioSwitch {
    /// 全部开关（顺序稳定）。
    pub const ALL: [Self; 9] = [
        Self::CareCandidates,
        Self::Circadian,
        Self::ContextPolicy,
        Self::Lifecycle,
        Self::Pain,
        Self::Play,
        Self::RepairGuidance,
        Self::SelfStatus,
        Self::Tone,
    ];

    /// 对应的既有 env 名（引用各模块常量，不复制字面量）。
    pub const fn env_name(self) -> &'static str {
        match self {
            Self::CareCandidates => BIO_CARE_CANDIDATES_ENABLE_ENV,
            Self::Circadian => BIO_CIRCADIAN_ENABLE_ENV,
            Self::ContextPolicy => BIO_CONTEXT_POLICY_ENABLE_ENV,
            Self::Lifecycle => BIO_LIFECYCLE_ENABLE_ENV,
            Self::Pain => BIO_PAIN_ENABLE_ENV,
            Self::Play => BIO_PLAY_ENABLE_ENV,
            Self::RepairGuidance => BIO_REPAIR_GUIDANCE_ENABLE_ENV,
            Self::SelfStatus => apeireth_tools_canonical::self_status::BIO_SELF_STATUS_ENABLE_ENV,
            Self::Tone => BIO_TONE_ENABLE_ENV,
        }
    }

    /// 稳定短标签（台账/日志用）。
    pub const fn label(self) -> &'static str {
        match self {
            Self::CareCandidates => "care_candidates",
            Self::Circadian => "circadian",
            Self::ContextPolicy => "context_policy",
            Self::Lifecycle => "lifecycle",
            Self::Pain => "pain",
            Self::Play => "play",
            Self::RepairGuidance => "repair_guidance",
            Self::SelfStatus => "self_status",
            Self::Tone => "tone",
        }
    }

    const fn index(self) -> usize {
        match self {
            Self::CareCandidates => 0,
            Self::Circadian => 1,
            Self::ContextPolicy => 2,
            Self::Lifecycle => 3,
            Self::Pain => 4,
            Self::Play => 5,
            Self::RepairGuidance => 6,
            Self::SelfStatus => 7,
            Self::Tone => 8,
        }
    }
}

/// 开关表解析错误（只由节律时区参数产生；开关本身 fail-closed 不报错）。
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("{0}")]
pub struct NervousSwitchboardError(String);

impl NervousSwitchboardError {
    fn new(message: impl Into<String>) -> Self {
        Self(message.into())
    }
}

/// 装配根一次的开关解析结果（R1b-1a：只解析与记录；机制迁移在 R1b-1b）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NervousSwitchboard {
    enabled: [bool; 9],
    circadian_tz_offset_min: i32,
}

impl Default for NervousSwitchboard {
    fn default() -> Self {
        Self {
            enabled: [false; 9],
            circadian_tz_offset_min: DEFAULT_CIRCADIAN_TZ_OFFSET_MIN,
        }
    }
}

impl NervousSwitchboard {
    /// 从进程 env 解析一次（装配根唯一入口；本函数是唯一同时读这 10 个 env 的
    /// 位置）。
    pub fn from_env() -> Result<Self, NervousSwitchboardError> {
        Self::from_lookup(|name| std::env::var(name).ok())
    }

    /// 从注入的查值函数解析（测试用；不碰进程 env）。
    ///
    /// 恰好查询 [`BioSwitch::ALL`] 的 9 个开关 + [`CIRCADIAN_TZ_OFFSET_ENV`]。
    pub fn from_lookup(
        lookup: impl Fn(&str) -> Option<String>,
    ) -> Result<Self, NervousSwitchboardError> {
        let mut enabled = [false; 9];
        for switch in BioSwitch::ALL {
            enabled[switch.index()] = lookup(switch.env_name()).as_deref() == Some("1");
        }
        let circadian_tz_offset_min = parse_tz_offset(lookup(CIRCADIAN_TZ_OFFSET_ENV).as_deref())?;
        Ok(Self {
            enabled,
            circadian_tz_offset_min,
        })
    }

    /// 单个开关注入式构造（测试用；时区取缺省）。
    #[cfg(test)]
    fn with_switch(switch: BioSwitch) -> Self {
        let mut board = Self::default();
        board.enabled[switch.index()] = true;
        board
    }

    /// 某个开关的生效值。
    pub fn switch_enabled(&self, switch: BioSwitch) -> bool {
        self.enabled[switch.index()]
    }

    /// 至少一个开关为开（= 装配根需要注册桥模块）。
    pub fn any_enabled(&self) -> bool {
        self.enabled.iter().any(|enabled| *enabled)
    }

    /// 生效开关清单（顺序 = [`BioSwitch::ALL`]）。
    pub fn enabled_switches(&self) -> Vec<BioSwitch> {
        BioSwitch::ALL
            .into_iter()
            .filter(|switch| self.switch_enabled(*switch))
            .collect()
    }

    /// 节律时区偏移（分钟）。缺省 `0` = UTC；已在校验范围 ±840 内。
    pub fn circadian_tz_offset_min(&self) -> i32 {
        self.circadian_tz_offset_min
    }
}

fn parse_tz_offset(raw: Option<&str>) -> Result<i32, NervousSwitchboardError> {
    let Some(raw) = raw else {
        return Ok(DEFAULT_CIRCADIAN_TZ_OFFSET_MIN);
    };
    let parsed: i32 = raw.parse().map_err(|_| {
        NervousSwitchboardError::new(format!(
            "拒绝配置: {CIRCADIAN_TZ_OFFSET_ENV} 必须是整数分钟 (可含负号), got {raw:?}"
        ))
    })?;
    if !(-CIRCADIAN_TZ_OFFSET_LIMIT_MIN..=CIRCADIAN_TZ_OFFSET_LIMIT_MIN).contains(&parsed) {
        return Err(NervousSwitchboardError::new(format!(
            "拒绝配置: {CIRCADIAN_TZ_OFFSET_ENV} 必须在 ±{CIRCADIAN_TZ_OFFSET_LIMIT_MIN} 分钟 \
             (±14h) 内, got {parsed}"
        )));
    }
    Ok(parsed)
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::collections::BTreeMap;

    use super::*;

    fn board(pairs: &[(&str, &str)]) -> NervousSwitchboard {
        let map: BTreeMap<&str, &str> = pairs.iter().copied().collect();
        NervousSwitchboard::from_lookup(|name| map.get(name).map(|value| (*value).to_string()))
            .expect("合法配置必须解析成功")
    }

    #[test]
    fn default_is_all_off_utc_and_not_resolved_via_env() {
        let board = board(&[]);
        assert!(!board.any_enabled());
        assert!(board.enabled_switches().is_empty());
        assert_eq!(
            board.circadian_tz_offset_min(),
            DEFAULT_CIRCADIAN_TZ_OFFSET_MIN
        );
        for switch in BioSwitch::ALL {
            assert!(
                !board.switch_enabled(switch),
                "{} 默认必须关",
                switch.label()
            );
        }
    }

    #[test]
    fn every_switch_accepts_only_literal_one() {
        for switch in BioSwitch::ALL {
            let enabled = board(&[(switch.env_name(), "1")]);
            assert!(
                enabled.switch_enabled(switch),
                "{} 的 \"1\" 必须开",
                switch.label()
            );
            assert_eq!(enabled.enabled_switches(), vec![switch]);
            for value in ["true", "TRUE", "0", "", "01", "yes", "2"] {
                let parsed = board(&[(switch.env_name(), value)]);
                assert!(
                    !parsed.switch_enabled(switch),
                    "{} = {value:?} 必须 fail-closed",
                    switch.label()
                );
            }
        }
    }

    #[test]
    fn lookup_queries_each_switch_once_plus_tz() {
        let calls = RefCell::new(Vec::new());
        let parsed = NervousSwitchboard::from_lookup(|name| {
            calls.borrow_mut().push(name.to_string());
            None
        })
        .expect("全缺省合法");
        assert!(!parsed.any_enabled());
        let calls = calls.into_inner();
        assert_eq!(calls.len(), BioSwitch::ALL.len() + 1, "9 开关 + 1 参数");
        for switch in BioSwitch::ALL {
            assert_eq!(
                calls
                    .iter()
                    .filter(|name| name.as_str() == switch.env_name())
                    .count(),
                1,
                "{} 只许查一次",
                switch.label()
            );
        }
        assert_eq!(
            calls
                .iter()
                .filter(|name| name.as_str() == CIRCADIAN_TZ_OFFSET_ENV)
                .count(),
            1
        );
    }

    #[test]
    fn tz_offset_accepts_minutes_within_fourteen_hours() {
        for (raw, expected) in [
            ("480", 480),
            ("-300", -300),
            ("0", 0),
            ("840", 840),
            ("-840", -840),
        ] {
            let parsed = board(&[(CIRCADIAN_TZ_OFFSET_ENV, raw)]);
            assert_eq!(parsed.circadian_tz_offset_min(), expected, "raw = {raw:?}");
        }
    }

    #[test]
    fn tz_offset_rejects_non_integer_and_out_of_range() {
        for raw in ["abc", "8h", "841", "-841", "", "480.5"] {
            let error = NervousSwitchboard::from_lookup(|name| {
                (name == CIRCADIAN_TZ_OFFSET_ENV).then(|| raw.to_string())
            })
            .expect_err("非法时区必须拒绝");
            assert!(
                error.to_string().contains("拒绝配置"),
                "raw = {raw:?} 的错误必须是显式拒绝"
            );
        }
    }

    #[test]
    fn tz_offset_error_does_not_depend_on_switch_state() {
        // 节律关着，非法参数照样拒绝（不静默回退 = 不装）。
        let error = NervousSwitchboard::from_lookup(|name| {
            if name == CIRCADIAN_TZ_OFFSET_ENV {
                Some("oops".to_string())
            } else {
                None
            }
        })
        .expect_err("非法时区即使节律关也必须拒绝");
        assert!(error.to_string().contains(CIRCADIAN_TZ_OFFSET_ENV));
    }

    #[test]
    fn switch_enabled_is_stable_across_multiple_enabled_switches() {
        let parsed = board(&[
            (BIO_TONE_ENABLE_ENV, "1"),
            (BIO_PLAY_ENABLE_ENV, "1"),
            (BIO_CARE_CANDIDATES_ENABLE_ENV, "1"),
        ]);
        assert!(parsed.any_enabled());
        assert!(parsed.switch_enabled(BioSwitch::Tone));
        assert!(parsed.switch_enabled(BioSwitch::Play));
        assert!(parsed.switch_enabled(BioSwitch::CareCandidates));
        assert!(!parsed.switch_enabled(BioSwitch::Circadian));
        assert_eq!(
            parsed.enabled_switches(),
            vec![BioSwitch::CareCandidates, BioSwitch::Play, BioSwitch::Tone]
        );
    }

    #[test]
    fn single_switch_constructor_keeps_rest_off() {
        let board = NervousSwitchboard::with_switch(BioSwitch::Pain);
        assert!(board.switch_enabled(BioSwitch::Pain));
        assert_eq!(board.enabled_switches(), vec![BioSwitch::Pain]);
        assert_eq!(
            board.circadian_tz_offset_min(),
            DEFAULT_CIRCADIAN_TZ_OFFSET_MIN
        );
    }
}
