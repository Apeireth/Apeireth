//! 情绪→语气微调 (拍板 ⑱) —— `voice_speak` 读当前情绪态 (PAD 情感三维),
//! 按**冻结映射表**微调语速/音量。情绪只调「味道」, 不改说话内容。
//!
//! # 情绪源 (读码情绪面)
//!
//! 只认 **PAD 三维读数** ([`PadReading`]: pleasure / arousal / dominance 各
//! ∈ [-1, 1], 与 F1 organ `OrganOutput::Emotion`、presence PAD 同口径);
//! 情感环离散点按 PAD 就近映射后同走本表 (一条推导面, 不搞两套判档)。
//! 读取面是注入的 [`EmotionReader`]: 组装根把实时情绪态接进来, `voice_speak`
//! 在**每次发声时**现读一次。
//!
//! # 映射表 (拍板 ⑱, 冻结可测)
//!
//! | 档 [`EmotionBand`] | 判定 (PAD 读数) | 语速微调 | 音量微调 |
//! |---|---|---|---|
//! | 愉悦高 `HighPleasure` | `pleasure ≥ 0.5` | **+1** | **+3** |
//! | 张力高 `HighTension` | `arousal ≥ 0.5` 且 `pleasure ≤ 0.0` | **-1** | 0 |
//! | 中性 `Neutral` | 其余 | 0 (原参数) | 0 (原参数) |
//!
//! # 三条硬口径 (测试逐条钉死)
//!
//! 1. **无情绪数据 = 原参数零变化**: 读取面缺省或返回 `None` 时, 生效参数 =
//!    显式传参/缺省常量, 零微调 (不猜、不按默认情绪调);
//! 2. **显式传参覆盖自动微调 (用户给的优先)**: 调用方给了的参数原值直通,
//!    只有缺省参数才吃情绪微调;
//! 3. **微调幅度有限 (防夸张)**: 语速增量**±2 内** (拍板幅度上限, 语速刻度
//!    ±10); 音量增量按冻结表上限 **±3 格** (响度刻度 0-100, 3 格 = 3% 响度,
//!    比例上远小于语速 ±2)。增量先过 [`TuneDelta::clamped`] 钳位, 落地值再过
//!    SAPI 刻度钳位 —— 缺省音量已是满格 100, 愉悦高档的 +3 在满格基线上
//!    如实钳在上界 (不假装变大声)。
//!
//! # 接线状态 (0 装诚实)
//!
//! 机制面 (判档/映射表/钳位/显式覆盖) 已实现且测试钉死; **生产情绪源未接线**
//! —— 组装根尚未注入 [`EmotionReader`], 现状 = 无情绪数据 = 零微调 (原参数),
//! 不把「已实现未接线」说成已生效。

use std::sync::Arc;

use super::speak::{
    VOICE_RATE_DEFAULT, VOICE_RATE_MAX, VOICE_RATE_MIN, VOICE_VOLUME_DEFAULT, VOICE_VOLUME_MAX,
    VOICE_VOLUME_MIN,
};

/// 愉悦高档判档下限 (`pleasure ≥ 本值` = 愉悦高)。
pub const HIGH_PLEASURE_MIN: f32 = 0.5;

/// 张力高档判档下限 (`arousal ≥ 本值` 才算高唤起)。
pub const HIGH_TENSION_AROUSAL_MIN: f32 = 0.5;

/// 张力高档愉悦上限 (`pleasure ≤ 本值` 才算不悦侧 —— 高唤起 + 不悦 = 张力)。
pub const HIGH_TENSION_PLEASURE_MAX: f32 = 0.0;

/// 语速微调幅度上限 (拍板「微调幅度有限, ±2 内, 防夸张」落语速面)。
pub const TUNE_RATE_DELTA_CAP: i32 = 2;

/// 音量微调幅度上限 (冻结表最大 +3 格; 0-100 响度刻度, 同一防夸张纪律)。
pub const TUNE_VOLUME_DELTA_CAP: i32 = 3;

/// 情绪态读数 (PAD 情感三维, 与 F1 organ `OrganOutput::Emotion` 同口径:
/// pleasure / arousal / dominance 各 ∈ [-1, 1])。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PadReading {
    /// 愉悦度 (负=不悦, 正=愉悦)。
    pub pleasure: f32,
    /// 唤起度 (负=低落平静, 正=高唤起紧张)。
    pub arousal: f32,
    /// 掌控度 (本表不参与判档, 保留 PAD 契约完整)。
    pub dominance: f32,
}

/// 当前情绪态读取面 (注入: 组装根把实时 PAD 读数接进来)。
///
/// 缺省不接 / 返回 `None` = 无情绪数据 = 原参数零变化。
pub type EmotionReader = Arc<dyn Fn() -> Option<PadReading> + Send + Sync>;

/// 情绪档 (映射表的行键; 判档推导见 [`EmotionBand::derive`])。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EmotionBand {
    /// 愉悦高 (语速 +1 / 音量 +3)。
    HighPleasure,
    /// 张力高 (语速 -1 / 音量 0)。
    HighTension,
    /// 中性 (原参数, 零微调)。
    Neutral,
}

impl EmotionBand {
    /// Stable label for reports and ledgers.
    pub const fn label(self) -> &'static str {
        match self {
            Self::HighPleasure => "high_pleasure",
            Self::HighTension => "high_tension",
            Self::Neutral => "neutral",
        }
    }

    /// PAD 读数 → 情绪档 (判档阈值冻结, 见模块文档映射表)。
    ///
    /// 读数按 PAD 契约钳在 [-1, 1] 后判档; 愉悦高优先于张力高 (愉悦侧的高
    /// 唤起是「来劲」不是「紧张」)。
    pub fn derive(reading: &PadReading) -> Self {
        let pleasure = reading.pleasure.clamp(-1.0, 1.0);
        let arousal = reading.arousal.clamp(-1.0, 1.0);
        if pleasure >= HIGH_PLEASURE_MIN {
            Self::HighPleasure
        } else if arousal >= HIGH_TENSION_AROUSAL_MIN && pleasure <= HIGH_TENSION_PLEASURE_MAX {
            Self::HighTension
        } else {
            Self::Neutral
        }
    }
}

/// 冻结映射表的一行 (数值即承诺)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TuneRow {
    /// 情绪档。
    pub band: EmotionBand,
    /// 语速增量 (SAPI 刻度)。
    pub rate_delta: i32,
    /// 音量增量 (SAPI 刻度)。
    pub volume_delta: i32,
}

/// 情绪→语气微调映射表 (拍板 ⑱ 冻结: 愉悦高 +1/+3, 张力高 -1/0, 中性 0/0)。
pub const EMOTION_TUNE_TABLE: [TuneRow; 3] = [
    TuneRow {
        band: EmotionBand::HighPleasure,
        rate_delta: 1,
        volume_delta: 3,
    },
    TuneRow {
        band: EmotionBand::HighTension,
        rate_delta: -1,
        volume_delta: 0,
    },
    TuneRow {
        band: EmotionBand::Neutral,
        rate_delta: 0,
        volume_delta: 0,
    },
];

/// 一次微调增量 (语速/音量; 过 [`TuneDelta::clamped`] 才落地)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TuneDelta {
    /// 语速增量 (SAPI 刻度)。
    pub rate: i32,
    /// 音量增量 (SAPI 刻度)。
    pub volume: i32,
}

impl TuneDelta {
    /// 零微调 (无情绪数据 / 中性档)。
    pub const ZERO: Self = Self { rate: 0, volume: 0 };

    /// 冻结映射表查表 (每档一行, 覆盖全档)。
    pub fn for_band(band: EmotionBand) -> Self {
        let row = EMOTION_TUNE_TABLE
            .iter()
            .find(|row| row.band == band)
            .expect("frozen tune table covers every band");
        Self {
            rate: row.rate_delta,
            volume: row.volume_delta,
        }
        .clamped()
    }

    /// 防夸张钳位: 语速 ±[`TUNE_RATE_DELTA_CAP`] 内, 音量 ±[`TUNE_VOLUME_DELTA_CAP`] 内。
    #[must_use]
    pub fn clamped(self) -> Self {
        Self {
            rate: self.rate.clamp(-TUNE_RATE_DELTA_CAP, TUNE_RATE_DELTA_CAP),
            volume: self
                .volume
                .clamp(-TUNE_VOLUME_DELTA_CAP, TUNE_VOLUME_DELTA_CAP),
        }
    }
}

/// 一次发声的生效参数 (显式直通 + 缺省参数吃情绪微调)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TunedVoiceParams {
    /// 生效语速。
    pub rate: i32,
    /// 生效音量。
    pub volume: u32,
    /// 实际命中的情绪档 (无情绪数据 = `None` = 零微调)。
    pub band: Option<EmotionBand>,
}

/// 基线参数 + 微调增量 → 落地参数 (SAPI 刻度钳位; 显式覆盖前的最后一步)。
pub fn apply_tune(base_rate: i32, base_volume: u32, delta: TuneDelta) -> (i32, u32) {
    let delta = delta.clamped();
    let rate = (base_rate + delta.rate).clamp(VOICE_RATE_MIN, VOICE_RATE_MAX);
    let volume = base_volume
        .saturating_add_signed(delta.volume)
        .clamp(VOICE_VOLUME_MIN, VOICE_VOLUME_MAX);
    (rate, volume)
}

/// 显式传参 + 当前情绪态 → 生效语速/音量 (三条硬口径的唯一入口)。
///
/// * 显式传参原值直通 (用户给的优先, 不吃微调也不再校验 —— 校验在参数面);
/// * 缺省参数 = 缺省常量 + 情绪微调 (钳位两道: 增量上限 + SAPI 刻度);
/// * 无情绪数据 = 缺省参数 = 缺省常量 (零微调)。
pub fn tune_voice_params(
    explicit_rate: Option<i32>,
    explicit_volume: Option<u32>,
    emotion: Option<PadReading>,
) -> TunedVoiceParams {
    let band = emotion.as_ref().map(EmotionBand::derive);
    let delta = band.map(TuneDelta::for_band).unwrap_or(TuneDelta::ZERO);
    let (auto_rate, auto_volume) = apply_tune(VOICE_RATE_DEFAULT, VOICE_VOLUME_DEFAULT, delta);
    TunedVoiceParams {
        rate: explicit_rate.unwrap_or(auto_rate),
        volume: explicit_volume.unwrap_or(auto_volume),
        band,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reading(pleasure: f32, arousal: f32, dominance: f32) -> PadReading {
        PadReading {
            pleasure,
            arousal,
            dominance,
        }
    }

    /// 映射表逐档 (拍板 ⑱ 冻结值): 愉悦高 +1/+3, 张力高 -1/0, 中性 0/0。
    #[test]
    fn tune_table_is_frozen_band_by_band() {
        assert_eq!(
            TuneDelta::for_band(EmotionBand::HighPleasure),
            TuneDelta { rate: 1, volume: 3 },
            "愉悦高 = 语速 +1 / 音量 +3"
        );
        assert_eq!(
            TuneDelta::for_band(EmotionBand::HighTension),
            TuneDelta {
                rate: -1,
                volume: 0
            },
            "张力高 = 语速 -1"
        );
        assert_eq!(
            TuneDelta::for_band(EmotionBand::Neutral),
            TuneDelta::ZERO,
            "中性 = 原参数"
        );
        // 表本体逐档一一对应 (查表口径与表冻结值不许分叉)。
        assert_eq!(EMOTION_TUNE_TABLE.len(), 3);
        for row in EMOTION_TUNE_TABLE {
            assert_eq!(
                TuneDelta::for_band(row.band),
                TuneDelta {
                    rate: row.rate_delta,
                    volume: row.volume_delta
                }
            );
        }
    }

    /// 判档阈值冻结: 愉悦高 / 张力高 / 中性 三档边界逐个钉。
    #[test]
    fn band_derivation_thresholds_are_frozen() {
        // 愉悦高: pleasure ≥ 0.5 (愉悦侧的高唤起仍是愉悦)。
        assert_eq!(
            EmotionBand::derive(&reading(0.5, 0.9, 0.0)),
            EmotionBand::HighPleasure
        );
        assert_eq!(
            EmotionBand::derive(&reading(0.49, 0.0, 0.0)),
            EmotionBand::Neutral
        );
        // 张力高: arousal ≥ 0.5 且 pleasure ≤ 0.0。
        assert_eq!(
            EmotionBand::derive(&reading(0.0, 0.5, 0.0)),
            EmotionBand::HighTension
        );
        assert_eq!(
            EmotionBand::derive(&reading(-1.0, 1.0, -0.5)),
            EmotionBand::HighTension
        );
        assert_eq!(
            EmotionBand::derive(&reading(0.1, 0.9, 0.0)),
            EmotionBand::Neutral,
            "有愉悦不判张力"
        );
        assert_eq!(
            EmotionBand::derive(&reading(-0.5, 0.49, 0.0)),
            EmotionBand::Neutral,
            "唤起不够不判张力"
        );
        // 中性: 基线 / 低落但低唤起。
        assert_eq!(
            EmotionBand::derive(&reading(0.0, 0.0, 0.0)),
            EmotionBand::Neutral
        );
        assert_eq!(
            EmotionBand::derive(&reading(-0.7, -0.2, -0.4)),
            EmotionBand::Neutral
        );
    }

    /// 微调上限 (防夸张): 表内每档在幅度上限内, 表外超幅增量钳回上限。
    #[test]
    fn tune_amplitude_never_exceeds_the_caps() {
        for row in EMOTION_TUNE_TABLE {
            assert!(
                row.rate_delta.abs() <= TUNE_RATE_DELTA_CAP,
                "语速增量须在 ±{TUNE_RATE_DELTA_CAP} 内: {row:?}"
            );
            assert!(
                row.volume_delta.abs() <= TUNE_VOLUME_DELTA_CAP,
                "音量增量须在 ±{TUNE_VOLUME_DELTA_CAP} 内: {row:?}"
            );
        }
        // 超幅增量一律钳回上限。
        let oversized = TuneDelta {
            rate: 9,
            volume: -9,
        }
        .clamped();
        assert_eq!(
            oversized,
            TuneDelta {
                rate: TUNE_RATE_DELTA_CAP,
                volume: -TUNE_VOLUME_DELTA_CAP
            }
        );
        // 任意极端读数落地后与基线差都在幅度上限内。
        for extreme in [reading(1.0, 1.0, 1.0), reading(-1.0, -1.0, -1.0)] {
            let tuned = tune_voice_params(None, None, Some(extreme));
            assert!((tuned.rate - VOICE_RATE_DEFAULT).abs() <= TUNE_RATE_DELTA_CAP);
            assert!(
                tuned.volume.abs_diff(VOICE_VOLUME_DEFAULT) <= TUNE_VOLUME_DELTA_CAP.unsigned_abs(),
                "volume={}",
                tuned.volume
            );
        }
    }

    /// 显式传参覆盖自动微调 (用户给的优先): 给了的参数原值直通。
    #[test]
    fn explicit_parameters_override_the_auto_tune() {
        let joy = Some(reading(0.9, 0.1, 0.0));
        // 显式语速直通; 音量缺省仍吃微调 (满格基线如实钳在上界)。
        let tuned = tune_voice_params(Some(4), None, joy);
        assert_eq!(tuned.rate, 4, "显式语速不吃微调");
        assert_eq!(tuned.volume, VOICE_VOLUME_MAX);
        assert_eq!(tuned.band, Some(EmotionBand::HighPleasure));
        // 显式音量直通; 语速缺省吃微调 (+1)。
        let tuned = tune_voice_params(None, Some(55), joy);
        assert_eq!(tuned.rate, VOICE_RATE_DEFAULT + 1);
        assert_eq!(tuned.volume, 55, "显式音量不吃微调");
        // 双显式 = 双直通 (即便情绪档在也一样)。
        let tuned = tune_voice_params(Some(-3), Some(20), joy);
        assert_eq!((tuned.rate, tuned.volume), (-3, 20));
    }

    /// 无情绪数据 = 原参数零变化 (不猜、不按默认情绪调)。
    #[test]
    fn no_emotion_data_keeps_original_parameters() {
        let tuned = tune_voice_params(None, None, None);
        assert_eq!(
            (tuned.rate, tuned.volume),
            (VOICE_RATE_DEFAULT, VOICE_VOLUME_DEFAULT),
            "无情绪数据 = 缺省原参数, 零微调"
        );
        assert_eq!(tuned.band, None);
        // 显式参数 + 无情绪 = 原值直通。
        let tuned = tune_voice_params(Some(-7), Some(30), None);
        assert_eq!((tuned.rate, tuned.volume), (-7, 30));
    }

    /// 刻度钳位如实: 缺省满格音量上的 +3 钳在上界; 低基线上微调显形。
    #[test]
    fn apply_tune_clamps_to_scale_bounds_honestly() {
        // 满格基线 (缺省音量 100): 愉悦高档 +3 如实钳在 100 (不假装变大声)。
        let (rate, volume) = apply_tune(
            VOICE_RATE_DEFAULT,
            VOICE_VOLUME_DEFAULT,
            TuneDelta::for_band(EmotionBand::HighPleasure),
        );
        assert_eq!((rate, volume), (1, VOICE_VOLUME_MAX));
        // 低基线音量: 微调显形 (80 + 3 = 83)。
        let (rate, volume) = apply_tune(0, 80, TuneDelta::for_band(EmotionBand::HighPleasure));
        assert_eq!((rate, volume), (1, 83));
        // 张力高档: 语速 -1, 音量不动。
        let (rate, volume) = apply_tune(0, 80, TuneDelta::for_band(EmotionBand::HighTension));
        assert_eq!((rate, volume), (-1, 80));
        // 上下界钳位。
        assert_eq!(
            apply_tune(
                VOICE_RATE_MAX,
                1,
                TuneDelta::for_band(EmotionBand::HighPleasure)
            )
            .0,
            VOICE_RATE_MAX
        );
        assert_eq!(
            apply_tune(
                VOICE_RATE_MIN,
                1,
                TuneDelta::for_band(EmotionBand::HighTension)
            )
            .0,
            VOICE_RATE_MIN
        );
        assert_eq!(
            apply_tune(
                0,
                1,
                TuneDelta {
                    rate: 0,
                    volume: -TUNE_VOLUME_DELTA_CAP
                }
            )
            .1,
            VOICE_VOLUME_MIN
        );
    }
}
