//! 自动校准引擎 (Self-Tuning Engine) — 「性格养成」第一铲的核心机制件。
//!
//! **定位** (设计契约 2026-11 性格养成批):
//! - **重设计·轻默认**: 自学习默认关 (`APEIRETH_ENABLE_SELF_TUNING` 未设/非 "1" = 关),
//!   引擎本体不读开关 —— 开关由接线层把关。
//! - **不对称三件套**: 每次自动调整产生一条 [`TuningRecord`] (可见日志), 可
//!   [`SelfTuningEngine::revert`] 撤销, 记录整体透明给主人 (调参面板学习日志)。
//! - **治理分级**: 引擎**只准**动体验参数 —— [`TunableParam`] 恰四个体验级变体,
//!   治理/内核参数在类型系统里**根本不存在**, 不是靠注释约定。
//! - **0 装诚实**: 未接真实信号源的事件变体一律注释「接口已备待接」,
//!   引擎不制造"学习了"的假象; 无信号时 [`SelfTuningEngine::observe`] 不调参。
//!
//! **确定性**: 引擎全确定性、无 IO、无时钟 —— 时间戳一律由事件携带
//! (`at_epoch_ms`), 持久化 (tuning-log.jsonl 追加) 由调用方/接线层负责。
//!
//! **四个体验参数的单位与推导** (baseline 即现状硬编码常量, 未设 = 零变化):
//!
//! | 参数 | 单位 | baseline | min | max | max_step | 推导 |
//! |---|---|---|---|---|---|---|
//! | [`TunableParam::MemoryFade`] | 遗忘衰减强度倍率 (无量纲) | 1.0 | 0.25 | 4.0 | 0.25 | 有效艾宾浩斯半衰期 = 24h / 倍率 |
//! | [`TunableParam::CuriosityStrength`] | 好奇强度倍率 (无量纲) | 1.0 | 0.25 | 4.0 | 0.25 | 每日好奇预算 = 2000 × 倍率 |
//! | [`TunableParam::ToneSaturation`] | 语气情绪饱和倍率 (无量纲) | 1.0 | 0.0 | 2.0 | 0.2 | 情绪注入 EMA 混合项 (0.1/0.2) × 倍率 |
//! | [`TunableParam::ConsolidationCadence`] | 整合间隔 (回合数) | 1.0 | 1.0 | 10.0 | 1.0 | 每 N 回合触发一次记忆整合 |
//!
//! env 旋钮 (与调参面板/侧车注入严格同名):
//! `APEIRETH_TUNE_MEMORY_FADE` / `APEIRETH_TUNE_CURIOSITY_STRENGTH` /
//! `APEIRETH_TUNE_TONE_SATURATION` / `APEIRETH_TUNE_CONSOLIDATION_CADENCE`。
//! 值语义 = 对应参数值本身; 缺省或非法值 (非数值 / NaN / 越界) 一律回 baseline
//! (回默认而不是静默钳制, 如实回现状)。

use std::collections::HashMap;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

/// 一个"自然日"的毫秒数 (日累计漂移上限的窗口单位, 以事件时间戳整除切日, 确定性)。
pub const DAY_MS: i64 = 86_400_000;

/// 浮点相等判据 (判定"调整实际为 0"= 被边界挡住)。
const ZERO_DELTA_EPS: f64 = 1e-9;

/// 体验级可调参数 —— **恰四个**, 全部属于体验层。
///
/// 治理/内核参数 (权限、审批、洋葱门、守卫阈值、内核不变量等) 在设计上
/// **不可表达**: 本枚举没有它们的变体, `SelfTuningEngine` 的出入参类型只能
/// 拿到这四个 —— 这是编译期收窄, 不是注释承诺。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TunableParam {
    /// 遗忘衰减强度 (倍率): 作用于记忆艾宾浩斯衰减, 有效半衰期 = 24h / 倍率。
    MemoryFade,
    /// 好奇强度 (倍率): 作用于 CuriosityEngine 每日好奇预算 (2000 × 倍率)。
    CuriosityStrength,
    /// 语气情绪饱和度 (倍率): 作用于语调合成的情绪注入混合项 (0.1/0.2 × 倍率)。
    ToneSaturation,
    /// 整合节奏 (回合数): 每 N 回合触发一次记忆整合 (1 = 每回合 = 现行为)。
    ConsolidationCadence,
}

impl TunableParam {
    /// 全部体验参数 (固定四元; 新增变体必须同步本表与全部映射测试)。
    pub const ALL: [TunableParam; 4] = [
        TunableParam::MemoryFade,
        TunableParam::CuriosityStrength,
        TunableParam::ToneSaturation,
        TunableParam::ConsolidationCadence,
    ];

    /// 基线值 (= 各常量现状硬编码值的归一化语义; 未设 = 现行为, 零变化)。
    pub const fn baseline(self) -> f64 {
        match self {
            TunableParam::MemoryFade => 1.0,
            TunableParam::CuriosityStrength => 1.0,
            TunableParam::ToneSaturation => 1.0,
            TunableParam::ConsolidationCadence => 1.0,
        }
    }

    /// 参数下界 (夹紧下限)。
    pub const fn min(self) -> f64 {
        match self {
            TunableParam::MemoryFade => 0.25,
            TunableParam::CuriosityStrength => 0.25,
            TunableParam::ToneSaturation => 0.0,
            TunableParam::ConsolidationCadence => 1.0,
        }
    }

    /// 参数上界 (夹紧上限)。
    pub const fn max(self) -> f64 {
        match self {
            TunableParam::MemoryFade => 4.0,
            TunableParam::CuriosityStrength => 4.0,
            TunableParam::ToneSaturation => 2.0,
            TunableParam::ConsolidationCadence => 10.0,
        }
    }

    /// 单次自动调整的步长上限 (一次 observe 最多移动这么多)。
    pub const fn max_step(self) -> f64 {
        match self {
            TunableParam::MemoryFade => 0.25,
            TunableParam::CuriosityStrength => 0.25,
            TunableParam::ToneSaturation => 0.2,
            TunableParam::ConsolidationCadence => 1.0,
        }
    }

    /// 每日累计漂移上限 (同一自然日同一参数的 |Δ| 累计, 4 个步长):
    /// 限制"一天之内被环境推着走太远", 超限不调、只记「已达上限」。
    pub const fn max_daily_drift(self) -> f64 {
        self.max_step() * 4.0
    }

    /// env 旋钮名 (与调参面板/侧车注入严格同名)。
    pub const fn env_name(self) -> &'static str {
        match self {
            TunableParam::MemoryFade => "APEIRETH_TUNE_MEMORY_FADE",
            TunableParam::CuriosityStrength => "APEIRETH_TUNE_CURIOSITY_STRENGTH",
            TunableParam::ToneSaturation => "APEIRETH_TUNE_TONE_SATURATION",
            TunableParam::ConsolidationCadence => "APEIRETH_TUNE_CONSOLIDATION_CADENCE",
        }
    }

    /// 中文名 (学习日志/调参面板展示用)。
    pub const fn label(self) -> &'static str {
        match self {
            TunableParam::MemoryFade => "遗忘衰减强度",
            TunableParam::CuriosityStrength => "好奇心强度",
            TunableParam::ToneSaturation => "语气情绪饱和度",
            TunableParam::ConsolidationCadence => "整合节奏",
        }
    }

    /// 夹紧到 `[min, max]` (NaN 视为 baseline)。
    pub fn clamp_value(self, value: f64) -> f64 {
        if value.is_finite() {
            value.clamp(self.min(), self.max())
        } else {
            self.baseline()
        }
    }

    /// 解析 env 旋钮原始串: 合法 (有限且在界内) 用值本身;
    /// 缺省/空白/非数值/NaN/越界一律回 baseline —— **非法值回默认**,
    /// 不静默钳制 (钳制会把主人的误操作伪装成有效设置)。
    pub fn parse_env_value(self, raw: Option<&str>) -> f64 {
        match raw.map(str::trim).filter(|s| !s.is_empty()) {
            None => self.baseline(),
            Some(text) => match text.parse::<f64>() {
                Ok(value) if value.is_finite() && value >= self.min() && value <= self.max() => {
                    value
                }
                _ => self.baseline(),
            },
        }
    }

    /// 当前生效值: 进程内 override (自动校准落地) > env 旋钮 > baseline。
    ///
    /// 各常量的使用/构造点调用本方法取值 —— 未设 env 且无 override 时恒等于
    /// baseline (= 现状常量), 行为零变化。
    pub fn effective(self) -> f64 {
        if let Some(values) = effective_override() {
            return values.get(self);
        }
        self.parse_env_value(std::env::var(self.env_name()).ok().as_deref())
    }
}

/// 四个体验参数的一组取值 (纯数据, Copy; 用于引擎状态与生效值传递)。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct TuningValues {
    /// 遗忘衰减强度倍率。
    pub memory_fade: f64,
    /// 好奇强度倍率。
    pub curiosity_strength: f64,
    /// 语气情绪饱和倍率。
    pub tone_saturation: f64,
    /// 整合间隔 (回合数)。
    pub consolidation_cadence: f64,
}

impl Default for TuningValues {
    fn default() -> Self {
        Self::baseline()
    }
}

impl TuningValues {
    /// 全基线 (= 现状常量, 零变化)。
    pub const fn baseline() -> Self {
        Self {
            memory_fade: TunableParam::MemoryFade.baseline(),
            curiosity_strength: TunableParam::CuriosityStrength.baseline(),
            tone_saturation: TunableParam::ToneSaturation.baseline(),
            consolidation_cadence: TunableParam::ConsolidationCadence.baseline(),
        }
    }

    /// 从四个 env 旋钮取值 (非法回 baseline); 未设 = 全基线 = 现行为。
    pub fn from_env() -> Self {
        Self {
            memory_fade: TunableParam::MemoryFade.parse_env_value(
                std::env::var(TunableParam::MemoryFade.env_name())
                    .ok()
                    .as_deref(),
            ),
            curiosity_strength: TunableParam::CuriosityStrength.parse_env_value(
                std::env::var(TunableParam::CuriosityStrength.env_name())
                    .ok()
                    .as_deref(),
            ),
            tone_saturation: TunableParam::ToneSaturation.parse_env_value(
                std::env::var(TunableParam::ToneSaturation.env_name())
                    .ok()
                    .as_deref(),
            ),
            consolidation_cadence: TunableParam::ConsolidationCadence.parse_env_value(
                std::env::var(TunableParam::ConsolidationCadence.env_name())
                    .ok()
                    .as_deref(),
            ),
        }
    }

    /// 取某参数当前值。
    pub fn get(self, param: TunableParam) -> f64 {
        match param {
            TunableParam::MemoryFade => self.memory_fade,
            TunableParam::CuriosityStrength => self.curiosity_strength,
            TunableParam::ToneSaturation => self.tone_saturation,
            TunableParam::ConsolidationCadence => self.consolidation_cadence,
        }
    }

    /// 改某参数值 (调用方负责夹紧; 引擎侧统一走 [`TunableParam::clamp_value`])。
    pub fn set(&mut self, param: TunableParam, value: f64) {
        match param {
            TunableParam::MemoryFade => self.memory_fade = value,
            TunableParam::CuriosityStrength => self.curiosity_strength = value,
            TunableParam::ToneSaturation => self.tone_saturation = value,
            TunableParam::ConsolidationCadence => self.consolidation_cadence = value,
        }
    }

    /// 全参数夹紧 (越界/NaN 回各参数 baseline 语义上的界内值)。
    pub fn clamped(self) -> Self {
        let mut out = self;
        for param in TunableParam::ALL {
            out.set(param, param.clamp_value(self.get(param)));
        }
        out
    }
}

/// 进程内生效值 override (自动校准落地点)。
///
/// 接线层在每次 [`TuningRecord`] 产生后 [`install_effective_values`],
/// 各常量使用点经 [`TunableParam::effective`] 实时取值。
/// 无 override 时各常量回落 env 旋钮 / baseline —— 进程重启即回到配置值。
static OVERRIDES: Mutex<Option<TuningValues>> = Mutex::new(None);

/// 安装当前生效值 (接线层在自动调整后调用; 越界值先夹紧)。
pub fn install_effective_values(values: TuningValues) {
    let mut slot = OVERRIDES
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    *slot = Some(values.clamped());
}

/// 清除生效值 override (回到 env 旋钮 / baseline; 测试与重置用)。
pub fn clear_effective_overrides() {
    let mut slot = OVERRIDES
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    *slot = None;
}

/// 读当前 override (无则 None)。
pub fn effective_override() -> Option<TuningValues> {
    OVERRIDES
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .as_ref()
        .copied()
}

/// 一条自动调整记录 —— 「记录透明」的载体 (可见日志/学习日志的行格式)。
///
/// 序列化即 tuning-log.jsonl 的一行 (snake_case)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TuningRecord {
    /// 单调递增序号 (引擎内全局, 也用于撤销定位)。
    pub seq: u64,
    /// 被调的体验参数。
    pub param: TunableParam,
    /// 调整前值。
    pub previous: f64,
    /// 调整后值 (被上限挡住时 == previous)。
    pub next: f64,
    /// 调整原因 (人可读, 记录透明)。
    pub reason: String,
    /// 事件时间戳 (epoch ms, 由信号携带, 引擎不读时钟)。
    pub at_epoch_ms: i64,
}

/// 使用信号事件 —— 引擎的观察输入。
///
/// **信号接入状态 (0 装诚实, 逐条如实标注)**:
/// - [`TuningEvent::RetrievalOutcome`] —— **真接**: MemoryRecallModule TurnStart
///   召回结果 (每 4 轮召回一批: 选中 ≥1 候选 = 命中轮, 空 = 未命中轮)。
/// - [`TuningEvent::ReflexionOutcome`] —— **接口已备待接** (reflexion 教训注入
///   后回合成败; 信号源尚未接入引擎)。
/// - [`TuningEvent::ToneFeedback`] —— **接口已备待接** (主人对语气情绪饱和度的
///   显式反馈; 尚无采集入口)。
/// - [`TuningEvent::ConsolidationOutcome`] —— **接口已备待接** (记忆整合产出;
///   run_consolidation 结果尚未回灌)。
#[derive(Debug, Clone, PartialEq)]
pub enum TuningEvent {
    /// 检索命中/未命中 (真接信号): 一批召回轮的计数。
    RetrievalOutcome {
        /// 命中轮数 (召回结果选中了至少 1 个候选的轮次)。
        hits: u32,
        /// 未命中轮数 (召回结果为空的轮次)。
        misses: u32,
        /// 事件时间戳 (epoch ms)。
        at_epoch_ms: i64,
    },
    /// 教训注入后回合成败 (接口已备待接)。
    ReflexionOutcome {
        /// 注入教训后的回合是否成功。
        turn_succeeded: bool,
        /// 事件时间戳 (epoch ms)。
        at_epoch_ms: i64,
    },
    /// 语气情绪饱和度的显式反馈 (接口已备待接)。
    ToneFeedback {
        /// 主人觉得情绪过饱和。
        felt_too_much: bool,
        /// 主人觉得情绪过于平淡。
        felt_too_flat: bool,
        /// 事件时间戳 (epoch ms)。
        at_epoch_ms: i64,
    },
    /// 记忆整合产出 (接口已备待接)。
    ConsolidationOutcome {
        /// 本次整合提炼出的洞察条数 (0 = 无产出)。
        extracted_insights: u32,
        /// 事件时间戳 (epoch ms)。
        at_epoch_ms: i64,
    },
}

impl TuningEvent {
    /// 事件时间戳 (epoch ms)。
    pub fn at_epoch_ms(&self) -> i64 {
        match self {
            TuningEvent::RetrievalOutcome { at_epoch_ms, .. }
            | TuningEvent::ReflexionOutcome { at_epoch_ms, .. }
            | TuningEvent::ToneFeedback { at_epoch_ms, .. }
            | TuningEvent::ConsolidationOutcome { at_epoch_ms, .. } => *at_epoch_ms,
        }
    }

    /// 确定性调整意图: (参数, 期望位移, 原因); 无方向证据 → None (不乱调)。
    ///
    /// **推导方向 (事件 → 调整)**:
    /// - 检索未命中占多 → [`TunableParam::MemoryFade`] 下调 (忘太快留不住 →
    ///   放缓遗忘); 检索全命中且 ≥4 轮 → 小步上调 (召回供给充裕, 遗忘可回收)。
    ///   持平/证据不足 → 不调。位移 = `max_step × 2 × |命中-未命中| / 轮数`
    ///   (可能超过 max_step, 由引擎按步长上限截断)。
    /// - 教训后回合成功 → [`TunableParam::CuriosityStrength`] 半步上调
    ///   (探索配比有效); 失败 → 半步下调 (先收敛好奇预算)。
    /// - 语气过饱和 → [`TunableParam::ToneSaturation`] 下调; 过平淡 → 上调;
    ///   同时/都不 → 不调。
    /// - 整合无产出 → [`TunableParam::ConsolidationCadence`] 上调 (间隔拉长,
    ///   少做无用功); 有产出 → 下调 (间隔缩短, 有用就勤一点)。
    fn intent(&self) -> Option<TuningIntent> {
        match *self {
            TuningEvent::RetrievalOutcome { hits, misses, .. } => {
                let total = hits.saturating_add(misses);
                if total == 0 {
                    return None;
                }
                let diff = f64::from(hits) - f64::from(misses);
                if diff.abs() <= ZERO_DELTA_EPS {
                    return None;
                }
                let total = f64::from(total);
                let param = TunableParam::MemoryFade;
                if diff > 0.0 {
                    // 上调遗忘强度需要"全命中"的强证据 (轻默认: 不敢轻易忘快)。
                    if misses != 0 || hits < 4 {
                        return None;
                    }
                    let strength = (diff / total) * 2.0;
                    Some(TuningIntent {
                        param,
                        desired_delta: param.max_step() * strength,
                        reason: format!(
                            "检索命中 {hits}/未命中 {misses}: 召回供给充裕, 遗忘衰减上调一档"
                        ),
                    })
                } else {
                    // 下调 (忘慢一点) 需要未命中占多且证据窗口 ≥ 4 轮。
                    if misses <= hits || total < 4.0 {
                        return None;
                    }
                    let strength = (-diff / total) * 2.0;
                    Some(TuningIntent {
                        param,
                        desired_delta: -param.max_step() * strength,
                        reason: format!(
                            "检索命中 {hits}/未命中 {misses}: 未命中占多, 放缓遗忘衰减以留住记忆"
                        ),
                    })
                }
            }
            TuningEvent::ReflexionOutcome { turn_succeeded, .. } => {
                let param = TunableParam::CuriosityStrength;
                let (desired_delta, reason) = if turn_succeeded {
                    (
                        param.max_step() * 0.5,
                        "教训注入后回合成功: 探索配比有效, 好奇强度半步上调".to_string(),
                    )
                } else {
                    (
                        -param.max_step() * 0.5,
                        "教训注入后回合失败: 先收敛好奇预算, 好奇强度半步下调".to_string(),
                    )
                };
                Some(TuningIntent {
                    param,
                    desired_delta,
                    reason,
                })
            }
            TuningEvent::ToneFeedback {
                felt_too_much,
                felt_too_flat,
                ..
            } => {
                let param = TunableParam::ToneSaturation;
                if felt_too_much && !felt_too_flat {
                    Some(TuningIntent {
                        param,
                        desired_delta: -param.max_step(),
                        reason: "语气反馈: 情绪过饱和, 饱和度下调".to_string(),
                    })
                } else if felt_too_flat && !felt_too_much {
                    Some(TuningIntent {
                        param,
                        desired_delta: param.max_step(),
                        reason: "语气反馈: 过于平淡, 饱和度上调".to_string(),
                    })
                } else {
                    None
                }
            }
            TuningEvent::ConsolidationOutcome {
                extracted_insights, ..
            } => {
                let param = TunableParam::ConsolidationCadence;
                if extracted_insights == 0 {
                    Some(TuningIntent {
                        param,
                        desired_delta: param.max_step(),
                        reason: "本次记忆整合无产出: 拉长整合间隔".to_string(),
                    })
                } else {
                    Some(TuningIntent {
                        param,
                        desired_delta: -param.max_step(),
                        reason: format!("本次记忆整合产出 {extracted_insights} 条: 缩短整合间隔"),
                    })
                }
            }
        }
    }
}

/// 单次观察产生的调整意图 (引擎负责夹紧/限步/限漂移)。
#[derive(Debug, Clone, PartialEq)]
struct TuningIntent {
    param: TunableParam,
    desired_delta: f64,
    reason: String,
}

/// 信号注入点: 真实使用信号进入引擎的唯一入口。
///
/// 各信号源 (检索路径 / reflexion 路径 / ... ) 实现本 trait, 接线层轮询
/// [`TuningSignal::poll`] 并把事件喂给 [`SelfTuningEngine::observe`];
/// `None` = 没有新信号, 引擎不乱调。
pub trait TuningSignal {
    /// 拉取下一条待观察信号; 无新信号返回 None。
    fn poll(&self) -> Option<TuningEvent>;
}

/// 检索命中/未命中计数器 —— **真接**信号源 (MemoryRecallModule 召回路径写入)。
///
/// 每凑满一批 (`batch` 轮, 默认 4) 可 [`TuningSignal::poll`] 出一条
/// [`TuningEvent::RetrievalOutcome`]; 不满一批不发 (信号按证据窗口成批)。
#[derive(Debug)]
pub struct RetrievalHitMissSignal {
    batch: u32,
    state: Mutex<RetrievalTally>,
}

#[derive(Debug, Default)]
struct RetrievalTally {
    hits: u32,
    misses: u32,
    at_epoch_ms: i64,
}

impl Default for RetrievalHitMissSignal {
    fn default() -> Self {
        Self::new()
    }
}

impl RetrievalHitMissSignal {
    /// 默认批次 = 4 轮 (与引擎的证据窗口下限一致)。
    pub fn new() -> Self {
        Self::with_batch(4)
    }

    /// 指定批次大小 (≥1)。
    pub fn with_batch(batch: u32) -> Self {
        Self {
            batch: batch.max(1),
            state: Mutex::new(RetrievalTally::default()),
        }
    }

    /// 记录一轮召回: `hit` = 该轮召回选中了至少 1 个候选。
    pub fn record_round(&self, hit: bool, at_epoch_ms: i64) {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if hit {
            state.hits += 1;
        } else {
            state.misses += 1;
        }
        state.at_epoch_ms = state.at_epoch_ms.max(at_epoch_ms);
    }

    /// 已累计未发的轮数 (诊断/测试用)。
    pub fn pending_rounds(&self) -> u32 {
        let state = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        state.hits + state.misses
    }
}

impl TuningSignal for RetrievalHitMissSignal {
    fn poll(&self) -> Option<TuningEvent> {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if state.hits + state.misses < self.batch {
            return None;
        }
        let event = TuningEvent::RetrievalOutcome {
            hits: state.hits,
            misses: state.misses,
            at_epoch_ms: state.at_epoch_ms,
        };
        state.hits = 0;
        state.misses = 0;
        state.at_epoch_ms = 0;
        Some(event)
    }
}

/// 自动校准引擎: 观察使用信号 → 微调四个体验参数, 每次调整留痕可撤销。
///
/// 全确定性、无 IO (持久化由调用方负责); 时间戳来自事件。
#[derive(Debug)]
pub struct SelfTuningEngine {
    values: TuningValues,
    history: Vec<TuningRecord>,
    next_seq: u64,
    /// (日序号, 参数) → 当日已累计 |Δ| (每日漂移上限账本)。
    daily_drift: HashMap<(i64, TunableParam), f64>,
    /// 见过的最大事件时间戳 (撤销记录的时间锚, 保持确定性)。
    last_at_epoch_ms: i64,
}

impl Default for SelfTuningEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl SelfTuningEngine {
    /// 全基线起步 (= 现状常量)。
    pub fn new() -> Self {
        Self::with_initial(TuningValues::baseline())
    }

    /// 以给定初值起步 (接线层用 env 旋钮值播种: 手动调参是自动校准的起点)。
    pub fn with_initial(values: TuningValues) -> Self {
        Self {
            values: values.clamped(),
            history: Vec::new(),
            next_seq: 1,
            daily_drift: HashMap::new(),
            last_at_epoch_ms: 0,
        }
    }

    /// 观察一条使用信号: 有方向证据 → 产生并返回一条 [`TuningRecord`]
    /// (步长上限 + 参数夹紧 + 每日漂移上限依次把关); 无信号/无方向 → None
    /// (不调也不记)。被上限挡住时**不调、只记**「已达上限」(previous == next)。
    pub fn observe(&mut self, event: TuningEvent) -> Option<TuningRecord> {
        let at = event.at_epoch_ms();
        if at > self.last_at_epoch_ms {
            self.last_at_epoch_ms = at;
        }
        let intent = event.intent()?;
        Some(self.apply(intent, at))
    }

    /// 从信号源拉一条并观察 (信号注入点的轮询式用法)。
    pub fn observe_signal(&mut self, signal: &dyn TuningSignal) -> Option<TuningRecord> {
        let event = signal.poll()?;
        self.observe(event)
    }

    /// 应用一次调整意图, 依次执行: 步长上限截断 → 参数夹紧 → 每日漂移上限把关。
    fn apply(&mut self, intent: TuningIntent, at_epoch_ms: i64) -> TuningRecord {
        let param = intent.param;
        let previous = self.values.get(param);
        let step = intent
            .desired_delta
            .clamp(-param.max_step(), param.max_step());
        let target = param.clamp_value(previous + step);
        let actual = target - previous;
        let day = at_epoch_ms.div_euclid(DAY_MS);
        let drift_used = self.daily_drift.entry((day, param)).or_insert(0.0);

        let (next, reason) = if actual.abs() <= ZERO_DELTA_EPS {
            // 被参数边界挡住: 不调, 只记「已达上限」。
            let bound = if previous <= param.min() + ZERO_DELTA_EPS {
                "已达参数下限"
            } else {
                "已达参数上限"
            };
            (previous, format!("{bound}: {}", intent.reason))
        } else if *drift_used + actual.abs() > param.max_daily_drift() {
            // 当日累计漂移超限: 不调, 只记「已达上限」。
            (previous, format!("已达每日漂移上限: {}", intent.reason))
        } else {
            *drift_used += actual.abs();
            (target, intent.reason)
        };

        let record = TuningRecord {
            seq: self.next_seq,
            param,
            previous,
            next,
            reason,
            at_epoch_ms,
        };
        self.next_seq += 1;
        if (next - previous).abs() > ZERO_DELTA_EPS {
            self.values.set(param, next);
        }
        self.history.push(record.clone());
        record
    }

    /// 撤销某次调整: 把参数恢复为该记录的 `previous` 值, 并追加一条可见的
    /// 撤销记录 (记录透明); 撤销是主人的显式动作, 不占自动漂移预算。
    ///
    /// 第一铲语义: 点值恢复 (不做中间历史重放)。未知 seq → false, 不留痕。
    pub fn revert(&mut self, seq: u64) -> bool {
        let Some(index) = self.history.iter().rposition(|record| record.seq == seq) else {
            return false;
        };
        let original = self.history[index].clone();
        let param = original.param;
        let previous = self.values.get(param);
        let next = param.clamp_value(original.previous);
        let at_epoch_ms = self.last_at_epoch_ms.max(original.at_epoch_ms);
        self.values.set(param, next);
        self.history.push(TuningRecord {
            seq: self.next_seq,
            param,
            previous,
            next,
            reason: format!("撤销 #{}: {}", seq, original.reason),
            at_epoch_ms,
        });
        self.next_seq += 1;
        true
    }

    /// 全部记录 (含撤销记录), 按产生顺序 —— 学习日志的数据源。
    pub fn history(&self) -> &[TuningRecord] {
        &self.history
    }

    /// 当前生效的四个体验参数值。
    pub fn effective_values(&self) -> TuningValues {
        self.values
    }

    /// 单参数当前值。
    pub fn value(&self, param: TunableParam) -> f64 {
        self.values.get(param)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(ms: i64) -> i64 {
        ms
    }

    /// 取一条全命中批事件 (上调方向的强证据)。
    fn hit_batch(at_epoch_ms: i64) -> TuningEvent {
        TuningEvent::RetrievalOutcome {
            hits: 4,
            misses: 0,
            at_epoch_ms,
        }
    }

    /// 取一条未命中占多批事件 (下调方向)。
    fn miss_batch(at_epoch_ms: i64) -> TuningEvent {
        TuningEvent::RetrievalOutcome {
            hits: 0,
            misses: 4,
            at_epoch_ms,
        }
    }

    // 1. 参数夹紧: 初值越界被夹紧; 顶到边界后再调不越界、只记「已达上限」。
    #[test]
    fn value_clamp_holds_param_bounds() {
        let mut engine = SelfTuningEngine::with_initial(TuningValues {
            memory_fade: 99.0,
            curiosity_strength: -3.0,
            tone_saturation: f64::NAN,
            consolidation_cadence: 0.0,
        });
        let values = engine.effective_values();
        assert_eq!(values.memory_fade, TunableParam::MemoryFade.max());
        assert_eq!(
            values.curiosity_strength,
            TunableParam::CuriosityStrength.min()
        );
        assert_eq!(
            values.tone_saturation,
            TunableParam::ToneSaturation.baseline()
        );
        assert_eq!(
            values.consolidation_cadence,
            TunableParam::ConsolidationCadence.min()
        );

        // 顶到上界后再要上调: 不越界, 记录 previous == next 且注明已达上限。
        let mut engine = SelfTuningEngine::with_initial(TuningValues {
            memory_fade: TunableParam::MemoryFade.max(),
            ..TuningValues::baseline()
        });
        let record = engine.observe(hit_batch(at(1_000))).expect("边界处仍留痕");
        assert_eq!(record.previous, record.next);
        assert!(record.reason.contains("已达参数上限"), "{}", record.reason);
        assert_eq!(
            engine.value(TunableParam::MemoryFade),
            TunableParam::MemoryFade.max()
        );

        // 夹紧在中途也生效: 0.3 一步向下只到下界 0.25。
        let mut engine = SelfTuningEngine::with_initial(TuningValues {
            memory_fade: 0.3,
            ..TuningValues::baseline()
        });
        let record = engine.observe(miss_batch(at(1_000))).expect("有记录");
        assert_eq!(record.next, TunableParam::MemoryFade.min());
    }

    // 2. 步长上限: 证据再强 (全命中 = 期望 2×max_step) 一次也只动 max_step。
    #[test]
    fn step_cap_limits_single_adjustment() {
        let mut engine = SelfTuningEngine::new();
        let record = engine.observe(hit_batch(at(1_000))).expect("全命中应调整");
        let moved = (record.next - record.previous).abs();
        assert!(
            (moved - TunableParam::MemoryFade.max_step()).abs() < 1e-9,
            "步长上限应截到 max_step: moved={moved}"
        );
    }

    // 3. 日累计漂移上限: 同日 4 步到顶, 第 5 次不调只记「已达每日漂移上限」,
    //    跨日恢复可调。
    #[test]
    fn daily_drift_cap_blocks_then_resets_next_day() {
        let mut engine = SelfTuningEngine::new();
        for i in 0..4_i64 {
            let record = engine
                .observe(hit_batch(at(1_000 + i)))
                .expect("同日前 4 次可调");
            assert!(record.next > record.previous);
        }
        let base = TunableParam::MemoryFade.baseline();
        let step = TunableParam::MemoryFade.max_step();
        assert!((engine.value(TunableParam::MemoryFade) - (base + 4.0 * step)).abs() < 1e-9);

        let blocked = engine
            .observe(hit_batch(at(2_000)))
            .expect("超限也留痕 (不调只记)");
        assert_eq!(blocked.previous, blocked.next);
        assert!(
            blocked.reason.contains("已达每日漂移上限"),
            "{}",
            blocked.reason
        );

        // 次日 (跨 DAY_MS) 恢复可调。
        let next_day = engine
            .observe(hit_batch(at(DAY_MS + 1_000)))
            .expect("次日可调");
        assert!(next_day.next > next_day.previous);
    }

    // 4. revert: 恢复 previous 值、追加可见撤销记录; 未知 seq 返 false 不留痕。
    #[test]
    fn revert_restores_previous_and_logs_transparently() {
        let mut engine = SelfTuningEngine::new();
        let record = engine.observe(miss_batch(at(1_000))).expect("应有调整");
        let before_revert = engine.value(TunableParam::MemoryFade);
        assert_eq!(before_revert, record.next);
        let history_len = engine.history().len();

        assert!(engine.revert(record.seq), "已知 seq 撤销成功");
        assert_eq!(
            engine.value(TunableParam::MemoryFade),
            record.previous,
            "撤销应回到调整前值"
        );
        let revert_record = &engine.history()[history_len];
        assert_eq!(revert_record.previous, record.next);
        assert_eq!(revert_record.next, record.previous);
        assert!(
            revert_record.reason.contains("撤销"),
            "{}",
            revert_record.reason
        );

        let history_len = engine.history().len();
        assert!(!engine.revert(999_999), "未知 seq 返 false");
        assert_eq!(engine.history().len(), history_len, "失败撤销不留痕");
    }

    // 5. 日志顺序: seq 严格递增, history 顺序 = 产生顺序。
    #[test]
    fn history_is_append_ordered_with_monotonic_seq() {
        let mut engine = SelfTuningEngine::new();
        for i in 0..3_i64 {
            engine.observe(miss_batch(at(1_000 + i))).expect("记录");
            engine.observe(hit_batch(at(2_000 + i))).expect("记录");
        }
        let history = engine.history();
        assert_eq!(history.len(), 6);
        // seq 严格递增 (时间戳跟事件走、可与顺序不一致, 日志顺序以 seq 为准)。
        for pair in history.windows(2) {
            assert!(pair[0].seq < pair[1].seq, "seq 必须严格递增");
        }
        let seqs: Vec<u64> = history.iter().map(|record| record.seq).collect();
        assert_eq!(seqs, vec![1, 2, 3, 4, 5, 6]);
    }

    // 6. 无信号不乱调: 空计数/持平批次 → None, 值与日志都不动。
    #[test]
    fn no_signal_leaves_values_untouched() {
        let mut engine = SelfTuningEngine::new();
        let empty = TuningEvent::RetrievalOutcome {
            hits: 0,
            misses: 0,
            at_epoch_ms: 1_000,
        };
        assert!(engine.observe(empty).is_none(), "空计数 = 无信号");
        let balanced = TuningEvent::RetrievalOutcome {
            hits: 2,
            misses: 2,
            at_epoch_ms: 2_000,
        };
        assert!(engine.observe(balanced).is_none(), "持平 = 无方向证据");
        // 未满批次的信号源也不发事件。
        let signal = RetrievalHitMissSignal::new();
        signal.record_round(true, 3_000);
        assert!(engine.observe_signal(&signal).is_none());
        assert_eq!(engine.effective_values(), TuningValues::baseline());
        assert!(engine.history().is_empty());
    }

    // 7. 事件 → 调整方向正确 (四种事件逐一核对方向)。
    #[test]
    fn event_directions_are_correct() {
        // 未命中占多 → MemoryFade 下调 (放缓遗忘)。
        let mut engine = SelfTuningEngine::new();
        let record = engine.observe(miss_batch(at(1_000))).expect("应下调");
        assert!(record.next < record.previous, "未命中占多应下调 MemoryFade");
        assert_eq!(record.param, TunableParam::MemoryFade);

        // 全命中 → MemoryFade 上调。
        let mut engine = SelfTuningEngine::new();
        let record = engine.observe(hit_batch(at(1_000))).expect("应上调");
        assert!(record.next > record.previous, "全命中应上调 MemoryFade");

        // 教训后成功/失败 → CuriosityStrength 上/下。
        let mut engine = SelfTuningEngine::new();
        let up = engine
            .observe(TuningEvent::ReflexionOutcome {
                turn_succeeded: true,
                at_epoch_ms: 1_000,
            })
            .expect("成功应上调好奇");
        assert_eq!(up.param, TunableParam::CuriosityStrength);
        assert!(up.next > up.previous);
        let down = engine
            .observe(TuningEvent::ReflexionOutcome {
                turn_succeeded: false,
                at_epoch_ms: 2_000,
            })
            .expect("失败应下调好奇");
        assert!(down.next < down.previous);

        // 语气过饱和/过平淡 → ToneSaturation 下/上。
        let mut engine = SelfTuningEngine::new();
        let down = engine
            .observe(TuningEvent::ToneFeedback {
                felt_too_much: true,
                felt_too_flat: false,
                at_epoch_ms: 1_000,
            })
            .expect("过饱和应下调");
        assert_eq!(down.param, TunableParam::ToneSaturation);
        assert!(down.next < down.previous);
        let up = engine
            .observe(TuningEvent::ToneFeedback {
                felt_too_much: false,
                felt_too_flat: true,
                at_epoch_ms: 2_000,
            })
            .expect("过平淡应上调");
        assert!(up.next > up.previous);
        // 两个反馈同时给 → 无方向证据, 不调。
        assert!(engine
            .observe(TuningEvent::ToneFeedback {
                felt_too_much: true,
                felt_too_flat: true,
                at_epoch_ms: 3_000,
            })
            .is_none());

        // 整合无产出 → cadence 上调 (间隔拉长); 有产出 → 下调 (间隔缩短)。
        let mut engine = SelfTuningEngine::new();
        let longer = engine
            .observe(TuningEvent::ConsolidationOutcome {
                extracted_insights: 0,
                at_epoch_ms: 1_000,
            })
            .expect("无产出应拉长间隔");
        assert_eq!(longer.param, TunableParam::ConsolidationCadence);
        assert!(longer.next > longer.previous);
        // cadence 下界是 1 (每回合), 缩短间隔要从非基线初值验证方向。
        let mut engine = SelfTuningEngine::with_initial(TuningValues {
            consolidation_cadence: 3.0,
            ..TuningValues::baseline()
        });
        let shorter = engine
            .observe(TuningEvent::ConsolidationOutcome {
                extracted_insights: 3,
                at_epoch_ms: 1_000,
            })
            .expect("有产出应缩短间隔");
        assert!(shorter.next < shorter.previous);
    }

    // 8. 治理分级: 引擎永不动非体验参数 —— 类型收窄 + 穷举锁定。
    #[test]
    fn engine_can_only_touch_experience_params() {
        // 穷举 match 不留通配分支: TunableParam 增加任何变体时本函数编译失败,
        // 强制重新审视"是否仍是体验参数" —— 编译期收窄, 不是注释约定。
        for param in TunableParam::ALL {
            let domain = match param {
                TunableParam::MemoryFade
                | TunableParam::CuriosityStrength
                | TunableParam::ToneSaturation
                | TunableParam::ConsolidationCadence => "experience",
            };
            assert_eq!(domain, "experience", "引擎只准动体验参数");
            assert!(
                param.env_name().starts_with("APEIRETH_TUNE_"),
                "体验旋钮命名前缀锁定: {}",
                param.env_name()
            );
        }
        assert_eq!(TunableParam::ALL.len(), 4, "体验参数恰四个");
    }

    // 9. env 解析: 非法值回默认 (缺省/非数值/越界/NaN), 合法值原样生效。
    #[test]
    fn env_parse_falls_back_to_baseline_on_illegal() {
        let param = TunableParam::MemoryFade;
        assert_eq!(param.parse_env_value(None), param.baseline());
        assert_eq!(param.parse_env_value(Some("")), param.baseline());
        assert_eq!(param.parse_env_value(Some("  ")), param.baseline());
        assert_eq!(param.parse_env_value(Some("abc")), param.baseline());
        assert_eq!(param.parse_env_value(Some("NaN")), param.baseline());
        assert_eq!(
            param.parse_env_value(Some("99")),
            param.baseline(),
            "越界回默认"
        );
        assert_eq!(
            param.parse_env_value(Some("-1")),
            param.baseline(),
            "越界回默认"
        );
        assert_eq!(param.parse_env_value(Some("0.5")), 0.5);
        let cadence = TunableParam::ConsolidationCadence;
        assert_eq!(cadence.parse_env_value(Some("3")), 3.0);
        assert_eq!(
            cadence.parse_env_value(Some("0")),
            cadence.baseline(),
            "低于下界回默认"
        );
    }

    // 10. 生效值优先级: override > env > baseline; 清除后回落。
    #[test]
    fn effective_prefers_override_then_env_baseline() {
        clear_effective_overrides();
        assert_eq!(
            TunableParam::ToneSaturation.effective(),
            TunableParam::ToneSaturation.baseline(),
            "无 env 无 override = 基线 (零变化)"
        );
        install_effective_values(TuningValues {
            memory_fade: 0.5,
            curiosity_strength: 1.5,
            tone_saturation: 0.8,
            consolidation_cadence: 2.0,
        });
        assert_eq!(TunableParam::MemoryFade.effective(), 0.5);
        assert_eq!(TunableParam::ConsolidationCadence.effective(), 2.0);
        clear_effective_overrides();
        assert_eq!(
            TunableParam::MemoryFade.effective(),
            TunableParam::MemoryFade.baseline()
        );
    }

    // 11. 信号注入点: 检索计数器按批发事件, 不满一批不发。
    #[test]
    fn retrieval_signal_batches_rounds() {
        let signal = RetrievalHitMissSignal::new();
        signal.record_round(true, 1_000);
        signal.record_round(false, 2_000);
        signal.record_round(false, 3_000);
        assert!(signal.poll().is_none(), "3 轮不满一批 (4)");
        assert_eq!(signal.pending_rounds(), 3);
        signal.record_round(false, 4_000);
        match signal.poll() {
            Some(TuningEvent::RetrievalOutcome {
                hits,
                misses,
                at_epoch_ms,
            }) => {
                assert_eq!((hits, misses), (1, 3));
                assert_eq!(at_epoch_ms, 4_000);
            }
            other => panic!("应发一批 RetrievalOutcome, got {other:?}"),
        }
        assert!(signal.poll().is_none(), "发完即空");
        assert_eq!(signal.pending_rounds(), 0);
    }

    // 12. 引擎种子: with_initial 从调参配置起步, effective_values 快照一致。
    #[test]
    fn engine_seeds_from_initial_values() {
        let seeded = TuningValues {
            memory_fade: 0.75,
            curiosity_strength: 1.25,
            tone_saturation: 0.6,
            consolidation_cadence: 3.0,
        };
        let engine = SelfTuningEngine::with_initial(seeded);
        assert_eq!(engine.effective_values(), seeded);
        assert_eq!(engine.value(TunableParam::ConsolidationCadence), 3.0);
        assert!(engine.history().is_empty());
    }
}
