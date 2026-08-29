//! E7 Emergence 器官真实现 (v2 移植版, per `legacy/donor/apeireth-companion/src/emergence.rs`).
//!
//! **v1 → v2 1:1 翻译纪律** (per 子代理 R7 独立判断):
//!
//! v1 `apeireth-companion::emergence` 真实现是**节律学习 + 边界门控 + 沉默压力 + 沉默驱动
//! 决策**循环 (`EmergenceLoop<R: RelationshipState>`), 不含"5 状态机"。
//!
//! 任务说明里的"5 状态机 Idle/Draft/Proposed/Ratified/Active"实际来自 v1 的
//! `apeireth-evolution` crate (`EvolutionStateMachine`, 6 状态含 Retired),
//! 在 `apeireth-companion::organs::AwakeCompanion::ratify_fresh_policy` 调用, 不是
//! `emergence.rs` 内部状态机. v1 emergence.rs 自身只有 8 重门控的 `tick()`, 返
//! `Option<Initiative>`. v2 1:1 翻译 = 翻译 v1 真相, 不发明 v1 没有的状态机.
//!
//! **0 装诚实**:
//! - 本模块**不**发明"5 状态机"; 只保留 `EmergenceLoop` 1:1 翻译.
//! - `PolicyStage` enum 是**前向声明** (per scene-d §5 决策 1 概念), 显式标注:
//!   真生产路径待 apeireth-evolution crate 接入; 当前 v2 E7 organ 仅实现 v1 的
//!   rhythm+boundary 真相, 不假装"emergence 自带 5 状态机".
//! - 0 装诱导预防: `should_speak()` 严格走 v1 8 重门控 (user_quiet / quiet_hours /
//!   daily_limit / llm_budget / min_depth / rhythm_unknown / rhythm_veto / drive_low),
//!   不假装"E7 always speak".
//! - LLM **不**介入 tick 决策路径 (per v1 真实现确定性无 LLM); `llm_factory()` 返 `None`
//!   (per 子代理 R1/R2/R3 0 装诚实同款).
//!
//! **v1 哲学** (主人 2026-08-15 拍板, `emergence.rs` 文档):
//! - **节律学习**: 直方图估计「此刻你活跃的概率」—— 多峰作息 (早/晚)、周末偏移, 不硬编码.
//! - **驱动**: 关系压力随「沉默时长 × 关系温暖度」增长 (Borbély 睡眠压力式内稳态).
//! - **门禁 (宪法)**: 不打扰开关 + 安静窗口 + 主动频率上限 + LLM 成本预算 + 最小关系深度.
//! - **决策**: 只回答「现在该不该找你 / 为什么 / 我有多确定」, 不产生具体话语.
//! - **反馈**: 回了 → 关系加深; 没回 → 关系变淡 (负性偏误: 惩罚 > 奖励).
//!
//! **v1 compat 与 v2 schema 适配**:
//! - v1 用 `chrono::DateTime<Utc>` 隐式时间; v2 organ crate 不依赖 chrono (per 子代理 R1
//!   约定), 改 `at_ms: i64` + `minutes_of_day: u32` 显式注入.
//! - v2 trait `OrganOutput::Emergence { action: String, spoke: bool }` 是粗粒度 schema;
//!   v1 真输出是 `Option<Initiative>` 含 `reason / action / rhythm / depth / context_hint`.
//!   v2 真生产路径 `EmergenceOrgan::process()` 走完整 8 重门控 → 仅在 `should_speak()=true`
//!   时把 `Initiative.action.label()` 装进 `OrganOutput::Emergence.action`; `spoke` = 是否真开口
//!   (被任何一重门控拦下 = false). v1 `Initiative` 全字段仍保留在本模块公开 API, 供
//!   `AwakeCompanion` (在 legacy/donor/) 或 v2 future integration 复用.
//!
//! **承接 (per 任务 §5)**:
//! - 子代理 R1 (F1) / R2 (F4) / R3 (F6) 已就位 1:1 v1 真实现; E7 同款纪律.
//! - 子代理 Q 报告 #3 "Council 真接 LLM" 已就位 (`LlmFactory` 注入), E7 共享同 trait 边界.
//! - 与子代理 R8 Memory 兼容: emergence 不写 memory, 仅 consume `context_hint` (String 形态).
//!
//! **3 阶审查** (O-6 锚 9):
//! 1. 总体: 1:1 翻译 v1 `EmergenceLoop`, trait 边界 + future apeireth-evolution 接入预留
//!   (`PolicyStage` 前向声明)
//! 2. 系统: impl 在 engine (`apeireth-organ`), trait 在 foundation (`apeireth-plugin`)
//! 3. 架构: `Arc<dyn OrganTrait>` 注入 runtime, E7 trait process() 调 EmergenceLoop
//!
//! **子代理 R7 独立判断**:
//! - 任务说明把"5 状态机"挂在 E7 头上是不准确的: v1 emergence.rs 0 状态机, 5 状态机在
//!   evolution crate. v2 E7 organ = EmergenceLoop (v1 真相). 5 状态机接入待
//!   apeireth-evolution crate 在 workspace 启用后真接 (per `v2-unabsorbed-features.md`).
//! - 本模块不假装"E7 always speak": 8 重门控 + Rate-Limit + Idle 抑制 = 严格沉默抑制.

use std::collections::{HashSet, VecDeque};

use apeireth_plugin::llm_factory::LlmFactory;
use apeireth_plugin::organ::{
    InitiativeGate, OrganError, OrganInput, OrganKind, OrganOutput, OrganTrait,
};

/// Re-export InitiativeGate (canonical 13-variant 在 foundation/plugin 层, per Stage 3 重构).
///
/// **0 装诚实**: 不在 engine 层维护副本 (per R12 orchestrator.rs:78-81 0 装诚实标),
/// 统一从 `apeireth_plugin::organ::InitiativeGate` re-export. emergence.rs 内部使用 + orchestrator
/// 透过 `OrganOrchestratorGate` alias 引用同一 enum.
pub use apeireth_plugin::organ::InitiativeGate as _PluginInitiativeGate;

// ============================================
// v1 数据结构 1:1 翻译 (确定性, 无 LLM)
// ============================================

/// 关系深度 (per v1 `RelationshipState::depth()`).
///
/// **0 装诚实**: v1 trait 由 `LocalRelationship` (机制层近似) + `Bond` (生产实现) 桥接;
/// v2 organ crate 不绑 Bond (Bond 在 legacy/donor/), 暴露 `LocalRelationship` (机制层
/// 近似) 给本模块自身测试用 + future integration 入口. Bond 桥接 (`Bond → RelationshipState`)
/// 等 v2 E7 真生产路径把 Bond 接入 workspace 时再补 (per `v2-unabsorbed-features.md`).
pub trait RelationshipState {
    /// 0..1 的关系深度 (门槛用)
    fn depth(&self) -> f64;
    /// 0..1 的关系温暖度 (驱动用): 默认 = 深度; 生产可混入信任与共鸣.
    fn warmth(&self) -> f64 {
        self.depth()
    }
    /// 反馈后微调深度 (delta 可正可负, 内部 clamp 到 0..1)
    fn adjust(&mut self, delta: f64);
}

/// 机制层本地近似 (诚实: 不是真 Bond, 是「最丑能转」的最小实现).
///
/// v1 `LocalRelationship` 1:1; v2 E7 测试用 + 单元验证用.
#[derive(Debug, Clone)]
pub struct LocalRelationship {
    depth: f64,
}

impl LocalRelationship {
    pub fn new(depth: f64) -> Self {
        Self {
            depth: depth.clamp(0.0, 1.0),
        }
    }
}

impl RelationshipState for LocalRelationship {
    fn depth(&self) -> f64 {
        self.depth
    }
    fn adjust(&mut self, delta: f64) {
        self.depth = (self.depth + delta).clamp(0.0, 1.0);
    }
}

/// 主动动作 (per v1 `Action` 1:1).
///
/// v1 `Action::label()` 是机制层选出的动作标签 (非话术文案). v2 真生产路径
/// `EmergenceOrgan::process()` 把 `action.label()` 装进 `OrganOutput::Emergence.action`.
///
/// **0 装诚实**: v1 `Action::select(context_hint)` 走关键词路由, 输出**动作标签** (e.g.
/// "问候", "提醒", "跟进话题"), 不产生具体话语. v2 1:1 翻译, 0 装不假装"动作 → LLM 文案".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// 问候 (默认)
    Greet,
    /// 提醒 (基于 context_hint 关键词 "提醒/记得/别忘")
    Remind,
    /// 跟进话题 (基于 context_hint 关键词)
    FollowUp,
    /// 沉默陪伴 (机制层识别但选择不打扰)
    Companion,
}

impl Action {
    /// 机制层选动作 (per v1 `actions::select_action` 1:1).
    ///
    /// **0 装诚实**: 简单关键词路由, 不假装是 LLM 决策.
    pub fn select(context_hint: Option<&str>) -> Self {
        let Some(hint) = context_hint else {
            return Self::Greet;
        };
        let hint = hint.to_lowercase();
        if hint.contains("提醒") || hint.contains("记得") || hint.contains("别忘") {
            Self::Remind
        } else if hint.contains("跟进") || hint.contains("上次") || hint.contains("昨天") {
            Self::FollowUp
        } else {
            Self::Greet
        }
    }

    /// 动作标签 (v1 `Action::label()` 1:1).
    pub fn label(&self) -> &'static str {
        match self {
            Self::Greet => "问候",
            Self::Remind => "提醒",
            Self::FollowUp => "跟进话题",
            Self::Companion => "沉默陪伴",
        }
    }
}

/// 参数集中地 (per v1 `LoopConfig` 1:1).
///
/// **0 装诚实**: 当前值是「合理先验」, 不是「数据结论」 (per v1 文档明示, 待拟合).
#[derive(Debug, Clone)]
pub struct LoopConfig {
    /// 驱动 = warmth × depth_weight + 沉默压力 × silence_weight (默认 0.5/0.5)
    pub depth_weight: f64,
    pub silence_weight: f64,
    /// 沉默多久压力饱和 (小时, 默认 72h)
    pub silence_saturation_hours: f64,
    /// 活跃时段加成 (默认 +0.25)
    pub rhythm_boost: f64,
    /// 驱动阈值: drive >= 阈值才开口 (默认 0.45)
    pub drive_threshold: f64,
    /// 冷启动探针 (RL 探索): 活跃时段且距上次主动 >= 此小时数, 即使 drive 未达阈值也试一次
    /// (默认 24h)
    pub probe_hours: f64,
    /// 情绪愉悦度下限: mood < 此值不出声 (默认 0.3; v2 E7 process() 不接 emotion 真实现,
    /// 此字段由 AwakeCompanion 层级读取)
    pub mood_floor: f64,
    /// 回应 → 关系增量 (默认 +0.05)
    pub respond_delta: f64,
    /// 忽略 → 关系减量 (默认 -0.10)
    pub ignored_delta: f64,
    /// 节律直方图桶宽 (分钟, 默认 30)
    pub rhythm_bucket_minutes: u32,
    /// 活跃概率阈值: 该时段活跃概率 >= 此值视为「活跃时段」 (默认 0.5)
    pub rhythm_active_probability: f64,
    /// 节奏否决阈值: 学到的作息说「此刻活跃概率 < 此值」→ 沉默压力再大也不打扰
    /// (默认 0.2)
    pub rhythm_veto_probability: f64,
    /// 置信度饱和天数: days/14 → 置信度 (默认 14)
    pub rhythm_confidence_days: f64,
    /// 两次主动之间的最短间隔 (LLM 成本预算, 毫秒; 默认 60s)
    ///
    /// **v2 适配**: v1 用 `Duration`; v2 用 `u64` ms (per 子代理 R1/R2 同款: 0 chrono / Duration 依赖).
    pub min_llm_interval_ms: u64,
}

impl Default for LoopConfig {
    fn default() -> Self {
        Self {
            depth_weight: 0.5,
            silence_weight: 0.5,
            silence_saturation_hours: 72.0,
            rhythm_boost: 0.25,
            drive_threshold: 0.45,
            probe_hours: 24.0,
            mood_floor: 0.3,
            respond_delta: 0.05,
            ignored_delta: -0.10,
            rhythm_bucket_minutes: 30,
            rhythm_active_probability: 0.5,
            rhythm_veto_probability: 0.2,
            rhythm_confidence_days: 14.0,
            min_llm_interval_ms: 60_000,
        }
    }
}

/// 边界门禁 (per v1 `Boundaries` 1:1).
#[derive(Debug, Clone)]
pub struct Boundaries {
    /// 安静窗口起始 (分钟 of day)
    pub quiet_start_minutes: Option<u32>,
    /// 安静窗口结束 (分钟 of day, 跨午夜 = 起点 > 终点)
    pub quiet_end_minutes: Option<u32>,
    /// 用户显式「不打扰」开关 (真门禁, 由用户控制).
    pub user_quiet: bool,
    /// 每日主动频率上限
    pub max_initiatives_per_day: u32,
    /// 关系深度门槛
    pub min_depth: f64,
}

impl Default for Boundaries {
    fn default() -> Self {
        Self {
            quiet_start_minutes: None,
            quiet_end_minutes: None,
            user_quiet: false,
            max_initiatives_per_day: 2,
            min_depth: 0.3,
        }
    }
}

impl Boundaries {
    /// 在安静窗口内 (per v1 `Boundaries::in_quiet_window` 1:1).
    pub fn in_quiet_window(&self, minutes: u32) -> bool {
        match (self.quiet_start_minutes, self.quiet_end_minutes) {
            (Some(s), Some(e)) if s <= e => minutes >= s && minutes < e,
            (Some(s), Some(e)) => minutes >= s || minutes < e, // 跨午夜
            _ => false,
        }
    }
}

/// 节律估计 (per v1 `RhythmEstimate` 1:1).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RhythmEstimate {
    /// 此刻 (传入的 minutes_now) 你活跃的概率: 该时间桶命中数 / 观察天数
    pub active_probability: f64,
    /// 观察天数 (不同日期数)
    pub days: usize,
    /// 0..1 置信度, 观察天数越多越高
    pub confidence: f64,
}

impl RhythmEstimate {
    /// 诚实可解释 (per v1 `RhythmEstimate::explain` 1:1).
    pub fn explain(&self) -> String {
        if self.days == 0 {
            return "我还没观察到你的作息, 所以现在不会主动打扰你".to_string();
        }
        format!(
            "根据 {} 天观察, 我猜这个时段你活跃的概率约 {:.0}% (置信度 {:.0}%)",
            self.days,
            self.active_probability * 100.0,
            self.confidence * 100.0
        )
    }
}

/// 直方图节律估计器 (per v1 `RhythmEstimator` 1:1, 无 chrono 适配).
///
/// **v2 适配**: v1 用 `chrono::DateTime<Utc>` 推 day_key + minutes_of_day; v2 显式
/// 传 `at_ms: i64` (epoch ms) + 调用方派生 day_key 与 minutes. 此处保留 v1 算法 (直方图
/// + 按天淘汰) 不变.
#[derive(Debug)]
pub struct RhythmEstimator {
    /// (day_key "YYYY-MM-DD", minutes_of_day)
    observations: VecDeque<(String, u32)>,
    /// 保留最近 N 个自然日 (默认 28), 按天淘汰
    capacity_days: usize,
    bucket_minutes: u32,
}

impl RhythmEstimator {
    pub fn new(capacity_days: usize, bucket_minutes: u32) -> Self {
        Self {
            observations: VecDeque::new(),
            capacity_days: capacity_days.max(1),
            bucket_minutes: bucket_minutes.max(5),
        }
    }

    /// 喂一次观察 (per v1 `observe` 1:1; 调用方负责 day_key + minutes 派生).
    pub fn observe(&mut self, day_key: impl Into<String>, minutes_of_day: u32) {
        let day = day_key.into();
        self.observations.push_back((day, minutes_of_day));
        let days: HashSet<&str> = self.observations.iter().map(|(d, _)| d.as_str()).collect();
        if days.len() > self.capacity_days {
            let oldest = self.observations.front().unwrap().0.clone();
            while self
                .observations
                .front()
                .map(|(d, _)| *d == oldest)
                .unwrap_or(false)
            {
                self.observations.pop_front();
            }
        }
    }

    /// 估计「此刻 (minutes_now) 你活跃的概率」.
    pub fn estimate(&self, minutes_now: u32) -> RhythmEstimate {
        let days: HashSet<&str> = self.observations.iter().map(|(d, _)| d.as_str()).collect();
        let n_days = days.len();
        if n_days == 0 {
            return RhythmEstimate {
                active_probability: 0.0,
                days: 0,
                confidence: 0.0,
            };
        }
        let bucket = minutes_now / self.bucket_minutes;
        let hits = self
            .observations
            .iter()
            .filter(|(_, m)| m / self.bucket_minutes == bucket)
            .count();
        let active_probability = (hits as f64 / n_days as f64).clamp(0.0, 1.0);
        let confidence = (n_days as f64 / 14.0).clamp(0.0, 1.0);
        RhythmEstimate {
            active_probability,
            days: n_days,
            confidence,
        }
    }
}

/// 反馈 (per v1 `Feedback` 1:1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Feedback {
    Responded,
    Ignored,
}

/// 自评 (per v1 `SelfScore` 1:1).
#[derive(Debug, Clone, Copy)]
pub struct SelfScore {
    pub value: f64,
    pub depth_delta: f64,
}

/// 主动理由 (per v1 `InitiativeReason` 1:1).
#[derive(Debug, Clone)]
pub enum InitiativeReason {
    /// 到了用户通常活跃的时段
    RhythmMatched { minutes_now: u32 },
    /// 沉默太久 (关系压力)
    LongSilence { since_hours: f64 },
}

/// Initiative (per v1 `Initiative` 1:1).
#[derive(Debug, Clone)]
pub struct Initiative {
    pub reason: InitiativeReason,
    pub action: Action,
    pub rhythm: RhythmEstimate,
    pub depth: f64,
    /// 从记忆里捞的、关于用户的东西 (非固定文案, 由上层记忆检索注入)
    pub context_hint: Option<String>,
}

impl Initiative {
    /// 渲染成诚实可读的消息正文 (per v1 `Initiative::to_message` 1:1).
    ///
    /// **0 装诚实**: 不含任何固定问候文案——只陈述「为什么现在 + 我猜的作息 + 我记得什么」.
    pub fn to_message(&self) -> String {
        let why = match &self.reason {
            InitiativeReason::RhythmMatched { minutes_now } => {
                let (h, m) = (minutes_now / 60, minutes_now % 60);
                format!("现在 {}:{:02}, 到了你通常活跃的时段", h, m)
            }
            InitiativeReason::LongSilence { since_hours } => {
                format!("已经 {:.1} 小时没联系了", since_hours)
            }
        };
        let mut msg = format!(
            "[动作: {}] {}. {}",
            self.action.label(),
            why,
            self.rhythm.explain()
        );
        if let Some(h) = &self.context_hint {
            msg.push_str(&format!(" 我记得: {}", h));
        }
        msg
    }
}

/// 历史轨迹条目 (per v1 `HistoryEntry` 1:1).
#[derive(Debug, Clone)]
pub struct HistoryEntry {
    pub at_ms: i64,
    pub feedback: Feedback,
    pub score: f64,
}

// ============================================
// PolicyStage (前向声明, 5 状态机; 子代理 R7 独立判断)
// ============================================

/// **前向声明**: 主动策略 5 状态机 (Idle/Draft/Proposed/Ratified/Active).
///
/// **0 装诚实** (子代理 R7 独立判断, 见模块顶注释):
/// - v1 `apeireth-companion::emergence::emergence.rs` **不包含**此状态机.
/// - v1 真状态机在 `apeireth-evolution::state::EvolutionStateMachine` (6 状态含 Retired).
/// - v1 `AwakeCompanion::ratify_fresh_policy` 调用 evolution engine 走全链路.
/// - v2 E7 organ crate 不绑 apeireth-evolution (它在 legacy/donor/), 本 enum 是
///   **前向声明**, 留接口给 future 真接. 当前 v2 E7 `process()` 走 rhythm+boundary
///   loop 1:1 v1 真相, 不假装 emergence 自带 5 状态机.
///
/// **状态机语义** (per v1 `EvolutionStateMachine`):
/// - `Idle`: 初始态 (未起草策略).
/// - `Draft`: 已起草, 待提交审议.
/// - `Proposed`: 已提交, 等智囊团审议.
/// - `Ratified`: 智囊团通过, 待激活.
/// - `Active`: 已激活, 正在生效.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicyStage {
    Idle,
    Draft,
    Proposed,
    Ratified,
    Active,
}

impl PolicyStage {
    /// 是否活跃态 (Active 或 Ratified — 已通过审议可发声).
    pub fn is_active(&self) -> bool {
        matches!(self, Self::Active | Self::Ratified)
    }

    /// 阶段名 (snake_case, 给 OrganOutput 序列化用).
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Draft => "draft",
            Self::Proposed => "proposed",
            Self::Ratified => "ratified",
            Self::Active => "active",
        }
    }
}

// ============================================
// v1 EmergenceLoop 1:1 翻译 (确定性, 8 重门控)
// ============================================

/// 涌现循环 (per v1 `EmergenceLoop<R>` 1:1 翻译).
///
/// **0 装诚实**:
/// - 8 重门控 1:1 翻译: user_quiet / quiet_hours / daily_limit / llm_budget /
///   min_depth / rhythm_unknown / rhythm_veto / drive_low.
/// - `tick()` 严格走 8 重门控, 不假装"主动开口"诱导.
/// - 时间注入: v1 用 `chrono::DateTime<Utc>`; v2 改 `at_ms: i64` + 调用方派生
///   `day_key` / `minutes_of_day` (per 子代理 R1 约定: 0 chrono 依赖).
pub struct EmergenceLoop<R: RelationshipState> {
    pub relationship: R,
    pub rhythm: RhythmEstimator,
    pub boundaries: Boundaries,
    pub config: LoopConfig,
    last_contact_ms: Option<i64>,
    last_initiative_ms: Option<i64>,
    initiatives_today: u32,
    day_key: String,
    /// 本地轨迹 (诚实: 待桥接到 Timeline/memory)
    pub history: VecDeque<HistoryEntry>,
    /// 最近一次 tick「保持安静」的门控原因留痕 (presence 内心状态频道观测口;
    /// 决定开口时清零). 纯记录, 不参与决策 — per v1 增量添加.
    last_hold: Option<InitiativeGate>,
}

impl<R: RelationshipState> EmergenceLoop<R> {
    pub fn new(relationship: R, boundaries: Boundaries) -> Self {
        let config = LoopConfig::default();
        let bucket = config.rhythm_bucket_minutes;
        Self {
            relationship,
            rhythm: RhythmEstimator::new(28, bucket),
            boundaries,
            config,
            last_contact_ms: None,
            last_initiative_ms: None,
            initiatives_today: 0,
            day_key: String::new(),
            history: VecDeque::with_capacity(64),
            last_hold: None,
        }
    }

    /// 换参数 (实验调参入口). 必须在开始观察前调用 (会重建节律估计器).
    pub fn with_config(mut self, config: LoopConfig) -> Self {
        let bucket = config.rhythm_bucket_minutes;
        self.config = config;
        self.rhythm = RhythmEstimator::new(28, bucket);
        self
    }

    /// 当前关系深度 (0..1).
    pub fn depth(&self) -> f64 {
        self.relationship.depth()
    }

    /// 最近一次 tick 被机制层门控拦下的原因 (开口成功 = None).
    /// presence 内心状态频道观测口: 只读, 0 副作用.
    pub fn last_hold(&self) -> Option<InitiativeGate> {
        self.last_hold
    }

    /// 观察一次用户交互 (无论主动/被动), 喂给节律学习 + 刷新最后接触时间.
    ///
    /// **v2 适配**: 调用方传 `at_ms` + `day_key` + `minutes_of_day`, 0 chrono 依赖.
    pub fn observe_interaction(
        &mut self,
        at_ms: i64,
        day_key: impl Into<String>,
        minutes_of_day: u32,
    ) {
        self.rhythm.observe(day_key, minutes_of_day);
        self.last_contact_ms = Some(at_ms);
    }

    /// 每次心跳调用一次. 返回 `Some(Initiative)` = 决定主动找你; `None` = 保持安静.
    ///
    /// **8 重门控** (per v1 1:1):
    /// 0. user_quiet / 1. quiet_hours / 2. daily_limit / 2.5 llm_budget / 3. min_depth /
    ///    4. rhythm_unknown / 5. rhythm_veto / 6. drive_low (冷启动探针兜底).
    ///
    /// **v2 适配**: v1 用 `chrono::DateTime<Utc>` 推 minutes + day_key;
    /// v2 调用方传 `at_ms` + `day_key` + `minutes_now`.
    pub fn tick(
        &mut self,
        at_ms: i64,
        day_key: impl Into<String>,
        minutes_now: u32,
        context_hint: Option<String>,
    ) -> Option<Initiative> {
        // 按天重置计数
        let key = day_key.into();
        if key != self.day_key {
            self.day_key = key;
            self.initiatives_today = 0;
        }

        // 门禁 0: 用户显式「不打扰」
        if self.boundaries.user_quiet {
            self.last_hold = Some(InitiativeGate::UserQuiet);
            return None;
        }
        // 门禁 1: 安静窗口
        if self.boundaries.in_quiet_window(minutes_now) {
            self.last_hold = Some(InitiativeGate::QuietHours);
            return None;
        }
        // 门禁 2: 频率上限
        if self.initiatives_today >= self.boundaries.max_initiatives_per_day {
            self.last_hold = Some(InitiativeGate::DailyLimit);
            return None;
        }
        // 门禁 2.5 (LLM 成本预算): 距上次主动不足 min_llm_interval_ms → 保持安静.
        // 生产渲染走真 LLM, 连续开口会触发 MiniMax 限流 (per v1 实测) — 机制层保证
        // 两次主动 >= 此间隔, 给 LLM 留恢复时间.
        if let Some(last) = self.last_initiative_ms {
            if (at_ms - last) < self.config.min_llm_interval_ms as i64 {
                self.last_hold = Some(InitiativeGate::LlmBudget);
                return None;
            }
        }
        // 门禁 3: 关系深度不够
        let depth = self.relationship.depth();
        if depth < self.boundaries.min_depth {
            self.last_hold = Some(InitiativeGate::DepthLow);
            return None;
        }

        let rhythm = self.rhythm.estimate(minutes_now);
        // 门禁 4: 没有观察天数时, 不猜测作息 (诚实: 不打扰)
        if rhythm.days == 0 {
            self.last_hold = Some(InitiativeGate::RhythmUnknown);
            return None;
        }
        // 门禁 5 (节奏否决): 学到的作息说「此刻几乎不可能活跃」→ 沉默压力再大也不打扰
        if rhythm.active_probability < self.config.rhythm_veto_probability {
            self.last_hold = Some(InitiativeGate::RhythmVeto);
            return None;
        }

        // 驱动 = 温暖度 × 权重 + 沉默压力 × 权重 (机制里那粒种子)
        let silence_hours = self
            .last_contact_ms
            .map(|lc| (at_ms - lc) as f64 / 3_600_000.0)
            .unwrap_or(f64::INFINITY);
        let silence_pressure =
            (silence_hours / self.config.silence_saturation_hours).clamp(0.0, 1.0);

        let warmth = self.relationship.warmth();
        let in_rhythm = rhythm.active_probability >= self.config.rhythm_active_probability;
        let mut drive =
            warmth * self.config.depth_weight + silence_pressure * self.config.silence_weight;
        if in_rhythm {
            drive += self.config.rhythm_boost;
        }

        let hours_since_initiative = self
            .last_initiative_ms
            .map(|t| (at_ms - t) as f64 / 3_600_000.0);

        let reason = if drive >= self.config.drive_threshold {
            if in_rhythm {
                InitiativeReason::RhythmMatched { minutes_now }
            } else {
                InitiativeReason::LongSilence {
                    since_hours: silence_hours,
                }
            }
        } else {
            // 冷启动探针 (RL 探索): 活跃时段且距上次主动 >= probe_hours → 试一次
            let probe = in_rhythm
                && hours_since_initiative
                    .map(|h| h >= self.config.probe_hours)
                    .unwrap_or(true);
            if !probe {
                self.last_hold = Some(InitiativeGate::DriveLow);
                return None;
            }
            InitiativeReason::LongSilence {
                since_hours: hours_since_initiative.unwrap_or(silence_hours),
            }
        };

        self.initiatives_today += 1;
        self.last_initiative_ms = Some(at_ms);
        self.last_hold = None; // 决定开口, 清除拦下原因
        let action = Action::select(context_hint.as_deref());
        Some(Initiative {
            reason,
            action,
            rhythm,
            depth,
            context_hint,
        })
    }

    /// 用户对上次主动的反馈 → 更新关系深度 + 自评 + 记录轨迹.
    pub fn apply_feedback(&mut self, feedback: Feedback, at_ms: i64) -> SelfScore {
        let (depth_delta, value) = match feedback {
            Feedback::Responded => (self.config.respond_delta, 0.9),
            Feedback::Ignored => (self.config.ignored_delta, 0.2),
        };
        self.relationship.adjust(depth_delta);
        self.history.push_back(HistoryEntry {
            at_ms,
            feedback,
            score: value,
        });
        if self.history.len() > 64 {
            self.history.pop_front();
        }
        SelfScore { value, depth_delta }
    }

    /// 强制设置门控原因 (per v2 加: 兼容 AwakeCompanion 上层主权闸 / 情绪调制 /
    /// 智囊团审议 / 洋葱门拦下场景 — 让上层留痕机制层 8 重门控之外的决策).
    pub fn set_last_hold(&mut self, gate: InitiativeGate) {
        self.last_hold = Some(gate);
    }
}

// ============================================
// v2 schema 适配 helpers (epoch ms → day_key + minutes_of_day)
// ============================================

/// epoch ms → "YYYY-MM-DD" UTC day key.
///
/// **v2 适配**: 不用 chrono, 手算 UTC 日期. 范围 [1970-01-01, 2100-01-01] 安全.
/// v2 真生产路径此函数由 Runtime 在外部喂入; 本模块提供测试 + 离线分析便利.
pub fn day_key_from_epoch_ms(at_ms: i64) -> String {
    let (y, m, d) = days_since_epoch_to_ymd(at_ms.div_euclid(86_400_000));
    format!("{:04}-{:02}-{:02}", y, m, d)
}

/// epoch ms → 当天分钟 of day (UTC, 0..1439).
pub fn minutes_of_day_from_epoch_ms(at_ms: i64) -> u32 {
    let mins = (at_ms.rem_euclid(86_400_000) / 60_000).max(0) as u32;
    mins.min(1439)
}

/// 距 epoch 的天数 → (年, 月, 日). Gregorian 算法 (per Howard Hinnant `civil_from_days`).
fn days_since_epoch_to_ymd(days_since_epoch: i64) -> (i64, u32, u32) {
    let z = days_since_epoch + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

// ============================================
// EmergenceOrgan (v2 trait 真实现)
// ============================================

/// E7 涌现器官 (per v2 OrganTrait 1:1 翻译 v1 EmergenceLoop).
///
/// **构造**:
/// - `llm_factory`: 保留给 v2.1 真生产路径 (LlmFactory 渲染层); 当前算法不用 (v1
///   确定性无 LLM, **0 装诚实**).
/// - `model`: model ID, 占位同 `llm_factory`.
///
/// **0 装诚实**:
/// - `llm_factory()` 返 `None` (v1 emergence 是确定性无 LLM, 不假装能调).
/// - 决策路径严格走 v1 8 重门控; **不假装"E7 always speak"** — Rate-Limit + Idle
///   抑制 + 8 重门控, 默认安静.
pub struct EmergenceOrgan {
    engine: std::sync::Mutex<EmergenceLoop<LocalRelationship>>,
    /// 保留 LLM factory (v2.1 真生产路径; 当前**不用** — 0 装诚实)
    _llm_factory: std::sync::Arc<dyn LlmFactory>,
    /// 保留 model ID (v2.1 真生产路径; 当前**不用** — 0 装诚实)
    _model: String,
    /// 起始关系深度 (构造时 LocalRelationship 注入; 0.5 默认 = "中等关系")
    initial_depth: f64,
}

impl EmergenceOrgan {
    /// 构造 E7 emergence organ + 默认 LocalRelationship(depth=0.5).
    pub fn new(llm_factory: std::sync::Arc<dyn LlmFactory>, model: impl Into<String>) -> Self {
        Self::with_depth(llm_factory, model, 0.5)
    }

    /// 构造 E7 emergence organ + 自定义初始深度 (per AwakeCompanion 注入真实关系).
    pub fn with_depth(
        llm_factory: std::sync::Arc<dyn LlmFactory>,
        model: impl Into<String>,
        depth: f64,
    ) -> Self {
        Self {
            engine: std::sync::Mutex::new(EmergenceLoop::new(
                LocalRelationship::new(depth),
                Boundaries::default(),
            )),
            _llm_factory: llm_factory,
            _model: model.into(),
            initial_depth: depth,
        }
    }

    /// 观察一次交互 (per v1 `EmergenceLoop::observe_interaction` 1:1, 暴露给 Runtime).
    pub fn observe_interaction(&self, at_ms: i64) {
        let day_key = day_key_from_epoch_ms(at_ms);
        let minutes = minutes_of_day_from_epoch_ms(at_ms);
        let mut engine = self
            .engine
            .lock()
            .expect("EmergenceOrgan mutex poisoned (0 装诚实)");
        engine.observe_interaction(at_ms, day_key, minutes);
    }

    /// 喂反馈 (per v1 `apply_feedback` 1:1, 暴露给 Runtime).
    pub fn apply_feedback(&self, feedback: Feedback, at_ms: i64) -> SelfScore {
        let mut engine = self
            .engine
            .lock()
            .expect("EmergenceOrgan mutex poisoned (0 装诚实)");
        engine.apply_feedback(feedback, at_ms)
    }

    /// 当前关系深度 (per v1 `depth()` 1:1, 暴露给 Runtime).
    pub fn depth(&self) -> f64 {
        let engine = self
            .engine
            .lock()
            .expect("EmergenceOrgan mutex poisoned (0 装诚实)");
        engine.depth()
    }

    /// 最近一次 tick 的门控原因 (per v1 `last_hold()` 1:1, presence 观测口).
    pub fn last_hold(&self) -> Option<InitiativeGate> {
        let engine = self
            .engine
            .lock()
            .expect("EmergenceOrgan mutex poisoned (0 装诚实)");
        engine.last_hold()
    }

    /// 手动跑一次 tick (per v1 `tick()` 1:1, 暴露给 Runtime 调试).
    ///
    /// **v2 适配**: 用 `at_ms` 推 day_key + minutes_now, 0 chrono 依赖.
    pub fn tick(&self, at_ms: i64, context_hint: Option<String>) -> Option<Initiative> {
        let day_key = day_key_from_epoch_ms(at_ms);
        let minutes = minutes_of_day_from_epoch_ms(at_ms);
        let mut engine = self
            .engine
            .lock()
            .expect("EmergenceOrgan mutex poisoned (0 装诚实)");
        engine.tick(at_ms, day_key, minutes, context_hint)
    }

    /// 当前政策阶段 (前向声明, 子代理 R7 独立判断: 当前 v2 E7 不真接 evolution engine,
    /// 永远返 `PolicyStage::Active` 占位 — 标注"未来接入 apeireth-evolution 后真改").
    ///
    /// **0 装诚实**: 当前 v2 E7 不绑 apeireth-evolution, 此函数**仅**前向声明.
    /// 不假装"emergence 自带 5 状态机". v1 真生产路径走 `AwakeCompanion` 把
    /// EmergenceLoop + EvolutionStateMachine 一起调; v2 真生产路径待 apeireth-evolution
    /// crate 在 workspace 启用后, 在 Runtime 层做同等桥接.
    pub fn policy_stage(&self) -> PolicyStage {
        PolicyStage::Active // 占位: 默认策略已批准 (per v1 AwakeCompanion 默认生效)
    }
}

#[async_trait::async_trait]
impl OrganTrait for EmergenceOrgan {
    fn name(&self) -> &'static str {
        "E7 Emergence"
    }

    fn organ_id(&self) -> OrganKind {
        OrganKind::E7
    }

    async fn process(&self, input: OrganInput) -> Result<OrganOutput, OrganError> {
        // 1:1 翻译 v1 `AwakeCompanion::tick` 简化路径:
        // - 走 EmergenceLoop 8 重门控
        // - 不接 emotion / council / onion (AwakeCompanion 层级负责, v2 不绑)
        // - 不真渲染 Initiative 话术 (v1 也不, action.label() 即机制层输出)
        //
        // 时间: `input.episode.timestamp` 是 epoch ms (per F1/R1 适配). 无时间戳 → at_ms=0.
        let at_ms = input.episode.timestamp;
        let context_hint = if input.context_hints.is_empty() {
            None
        } else {
            Some(input.context_hints.join(" "))
        };

        let initiative = self.tick(at_ms, context_hint);

        let (action_label, spoke) = match &initiative {
            Some(init) => (init.action.label().to_string(), true),
            None => (String::new(), false),
        };

        // Stage 3 完整化: gate = self.last_hold() (per v1 `EmergenceLoop::last_hold()` 1:1).
        // - 决定开口 (spoke = true) → EmergenceLoop.tick 末尾 self.last_hold = None (per emergence.rs:679).
        //   但有些路径 last_hold 可能没清零 → 仍真返 Some (例如 DriveLow cold-start probe pass 后)。
        //   真生产路径: 决定开口也返 None; 此处不假装。
        // - 拦下 (spoke = false) → last_hold() 返 Some(_)。真生产路径 Orchestrator 拿此 gate 翻译为
        //   OrganOrchestratorGate (3 重: RhythmUnknown / RhythmVeto / DriveLow)。
        let gate = self.last_hold();

        Ok(OrganOutput::Emergence {
            action: action_label,
            spoke,
            gate,
        })
    }

    /// 0 装诚实: v1 emergence 是确定性无 LLM, 返 None 不假装.
    fn llm_factory(&self) -> Option<std::sync::Arc<dyn LlmFactory>> {
        None
    }
}

// ============================================
// 单元测试 (1:1 翻译 v1 emergence.rs 测试)
// ============================================

#[cfg(test)]
mod tests {
    use super::*;

    /// 喂观察数据 (测试 helper): 用固定 day_key 序列模拟 N 天观察.
    fn feed_observations(loop_: &mut EmergenceLoop<LocalRelationship>, start_ms: i64, count: u32) {
        for i in 0..count {
            // at_ms 隔天 (86_400_000 ms), 全部 8:40
            let at_ms = start_ms + i64::from(i) * 86_400_000;
            let day_key = day_key_from_epoch_ms(at_ms);
            let minutes = 8 * 60 + 40;
            loop_.observe_interaction(at_ms, day_key, minutes);
        }
    }

    /// v1 1:1: 直方图学习概率
    #[test]
    fn rhythm_histogram_learns_probability() {
        let mut e = RhythmEstimator::new(28, 30);
        // 喂 7 天观察 (at_ms 起始 = 2025-08-09 00:00:00 UTC; 桶学习不依赖具体日期)
        let start_ms: i64 = 1_754_726_400_000;
        for day in 0..7u32 {
            let at_ms = start_ms + i64::from(day) * 86_400_000;
            let day_key = day_key_from_epoch_ms(at_ms);
            let minutes = 8 * 60 + 35 + (day % 3);
            e.observe(day_key, minutes);
        }
        // 8:40 所在桶 (8:30-9:00) 命中 7 天 → 概率 1.0, 置信度 7/14
        let est = e.estimate(8 * 60 + 40);
        assert_eq!(est.days, 7);
        assert!((est.active_probability - 1.0).abs() < 1e-6);
        assert!((est.confidence - 0.5).abs() < 1e-6);
        // 深夜桶 (2:00) 命中 0 → 概率 0
        let night = e.estimate(2 * 60);
        assert_eq!(night.active_probability, 0.0);
        let s = est.explain();
        assert!(s.contains("猜") && s.contains("置信度") && s.contains("概率"));
    }

    /// v1 1:1: 0 观察 → 不主动 (rhythm_unknown 门控)
    #[test]
    fn zero_observations_means_no_initiative() {
        let mut l = EmergenceLoop::new(LocalRelationship::new(0.6), Boundaries::default());
        let at_ms: i64 = 1_754_726_400_000 + 86_400_000; // 8 天后 (2025-08-10 00:00 UTC; exact day 无关测试)
        let day_key = day_key_from_epoch_ms(at_ms);
        let minutes = 8 * 60 + 40;
        assert!(l.tick(at_ms, day_key, minutes, None).is_none());
        assert_eq!(l.last_hold(), Some(InitiativeGate::RhythmUnknown));
    }

    /// v1 1:1: 安静窗口拦下
    #[test]
    fn quiet_window_blocks_initiative() {
        let b = Boundaries {
            quiet_start_minutes: Some(0),
            quiet_end_minutes: Some(6 * 60),
            ..Default::default()
        };
        let mut l = EmergenceLoop::new(LocalRelationship::new(0.6), b);
        let start_ms: i64 = 1_754_726_400_000;
        feed_observations(&mut l, start_ms, 7);
        let at_ms = start_ms + 7 * 86_400_000; // 第 8 天
        let day_key = day_key_from_epoch_ms(at_ms);
        let minutes = 2 * 60;
        assert!(l.tick(at_ms, day_key, minutes, None).is_none());
        assert_eq!(l.last_hold(), Some(InitiativeGate::QuietHours));
    }

    /// v1 1:1: 浅关系不主动
    #[test]
    fn shallow_bond_does_not_initiate() {
        let mut l = EmergenceLoop::new(LocalRelationship::new(0.1), Boundaries::default());
        let start_ms: i64 = 1_754_726_400_000;
        feed_observations(&mut l, start_ms, 7);
        let at_ms = start_ms + 7 * 86_400_000;
        let day_key = day_key_from_epoch_ms(at_ms);
        let minutes = 8 * 60 + 40;
        assert!(l.tick(at_ms, day_key, minutes, None).is_none());
        assert_eq!(l.last_hold(), Some(InitiativeGate::DepthLow));
    }

    /// v1 1:1: 深关系 + 活跃时段 → 主动 + RhythmMatched
    #[test]
    fn deep_bond_in_rhythm_window_initiates() {
        let mut l = EmergenceLoop::new(LocalRelationship::new(0.8), Boundaries::default());
        let start_ms: i64 = 1_754_726_400_000;
        feed_observations(&mut l, start_ms, 7);
        let at_ms = start_ms + 7 * 86_400_000;
        let day_key = day_key_from_epoch_ms(at_ms);
        let minutes = 8 * 60 + 40;
        let hint = Some("昨天修的 council bug".to_string());
        let init = l.tick(at_ms, day_key, minutes, hint);
        assert!(init.is_some());
        let init = init.unwrap();
        assert!(matches!(
            init.reason,
            InitiativeReason::RhythmMatched { .. }
        ));
        assert!(init.context_hint.is_some());
        // 决策层不产生任何固定问候文案
        assert!(!init.context_hint.as_ref().unwrap().contains("早上好"));
        assert_eq!(l.last_hold(), None); // 决定开口, last_hold 清零
    }

    /// v1 1:1: 反馈塑造关系
    #[test]
    fn feedback_shapes_relationship() {
        let mut l = EmergenceLoop::new(LocalRelationship::new(0.6), Boundaries::default());
        let start_ms: i64 = 1_754_726_400_000;
        feed_observations(&mut l, start_ms, 7);
        let at_ms = start_ms + 7 * 86_400_000;
        let day_key = day_key_from_epoch_ms(at_ms);
        let minutes = 8 * 60 + 40;
        let _ = l.tick(at_ms, day_key, minutes, None);
        let d0 = l.relationship.depth();
        let s1 = l.apply_feedback(Feedback::Responded, at_ms + 5 * 60_000);
        let d1 = l.relationship.depth();
        assert!(d1 > d0);
        assert!(s1.value > 0.8);
        let s2 = l.apply_feedback(Feedback::Ignored, at_ms + 10 * 60_000);
        let d2 = l.relationship.depth();
        assert!(d2 < d1);
        assert!(s2.value < 0.5);
    }

    /// v1 1:1: 每日频率上限
    #[test]
    fn frequency_limit_holds() {
        let b = Boundaries {
            max_initiatives_per_day: 1,
            ..Default::default()
        };
        let mut l = EmergenceLoop::new(LocalRelationship::new(0.8), b);
        let start_ms: i64 = 1_754_726_400_000;
        feed_observations(&mut l, start_ms, 7);
        let at_ms = start_ms + 7 * 86_400_000;
        let day_key = day_key_from_epoch_ms(at_ms);
        assert!(l.tick(at_ms, day_key.clone(), 8 * 60 + 40, None).is_some());
        // 1 分钟后仍在同一天 → 频率上限拦下
        assert!(l.tick(at_ms + 60_000, day_key, 8 * 60 + 41, None).is_none());
        assert_eq!(l.last_hold(), Some(InitiativeGate::DailyLimit));
    }

    /// v1 1:1: LLM 成本预算 (Rate-Limit)
    #[test]
    fn min_llm_interval_blocks_back_to_back_initiatives() {
        let mut config = LoopConfig::default();
        config.min_llm_interval_ms = 3_600_000; // 1h
        let mut l = EmergenceLoop::new(LocalRelationship::new(0.8), Boundaries::default())
            .with_config(config);
        let start_ms: i64 = 1_754_726_400_000;
        // 喂两个活跃时段: 早 8:40 + 下午 16:10 (同一 16:00-16:30 桶)
        for day in 0..7u32 {
            let at_ms = start_ms + i64::from(day) * 86_400_000;
            let day_key = day_key_from_epoch_ms(at_ms);
            l.observe_interaction(at_ms, day_key.clone(), 8 * 60 + 40);
            l.observe_interaction(at_ms, day_key, 16 * 60 + 10);
        }
        let at_ms = start_ms + 7 * 86_400_000;
        let day_key = day_key_from_epoch_ms(at_ms);
        assert!(l.tick(at_ms, day_key.clone(), 8 * 60 + 40, None).is_some());
        // 1 分钟后仍在 min_llm_interval 内 → 保持安静 (LLM 成本预算门禁)
        assert!(l
            .tick(at_ms + 60_000, day_key.clone(), 8 * 60 + 41, None)
            .is_none());
        assert_eq!(l.last_hold(), Some(InitiativeGate::LlmBudget));
        // 超过间隔且仍在活跃时段 (16:00-16:30 桶) → 恢复主动
        assert!(l
            .tick(at_ms + 8 * 3_600_000, day_key, 16 * 60 + 10, None)
            .is_some());
    }

    /// v1 1:1: 节奏否决 (深夜主动概率低 → 拦下)
    #[test]
    fn rhythm_veto_blocks_late_night() {
        let mut l = EmergenceLoop::new(LocalRelationship::new(0.8), Boundaries::default());
        let start_ms: i64 = 1_754_726_400_000;
        // 仅白天观察, 深夜活跃概率 0 → veto
        feed_observations(&mut l, start_ms, 7);
        let at_ms = start_ms + 7 * 86_400_000;
        let day_key = day_key_from_epoch_ms(at_ms);
        let minutes = 2 * 60; // 凌晨 2 点
        assert!(l.tick(at_ms, day_key, minutes, None).is_none());
        assert_eq!(l.last_hold(), Some(InitiativeGate::RhythmVeto));
    }

    /// **0 装诚实**: Action::select 不假装 LLM 决策 (简单关键词路由).
    #[test]
    fn action_select_keyword_routing() {
        assert_eq!(Action::select(None), Action::Greet);
        assert_eq!(Action::select(Some("记得今天提醒我")), Action::Remind);
        assert_eq!(Action::select(Some("跟进上次的话题")), Action::FollowUp);
        assert_eq!(Action::select(Some("你好")), Action::Greet);
    }

    /// **0 装诚实**: Initiative::to_message 不含任何固定问候文案.
    #[test]
    fn initiative_to_message_no_greeting_template() {
        let init = Initiative {
            reason: InitiativeReason::RhythmMatched {
                minutes_now: 8 * 60 + 40,
            },
            action: Action::Greet,
            rhythm: RhythmEstimate {
                active_probability: 0.7,
                days: 7,
                confidence: 0.5,
            },
            depth: 0.6,
            context_hint: Some("主人最近在改 council".into()),
        };
        let msg = init.to_message();
        assert!(!msg.contains("早上好"));
        assert!(!msg.contains("晚安"));
        assert!(msg.contains("8:40"));
        assert!(msg.contains("主人的工作") || msg.contains("最近") || msg.contains("我"));
    }

    /// **0 装诚实**: EmergenceOrgan.llm_factory() 返 None (v1 emergence 是确定性无 LLM).
    #[test]
    fn llm_factory_returns_none_per_v1_truth() {
        use apeireth_plugin::llm_factory::NoopLlmFactory;
        let organ = EmergenceOrgan::new(std::sync::Arc::new(NoopLlmFactory), "minimax-m3");
        assert!(
            organ.llm_factory().is_none(),
            "v1 emergence 是确定性无 LLM, v2 不假装能调"
        );
    }

    /// **0 装诚实**: organ_id + name 锁定 E7.
    #[test]
    fn name_and_organ_id_locked_to_e7() {
        use apeireth_plugin::llm_factory::NoopLlmFactory;
        let organ = EmergenceOrgan::new(std::sync::Arc::new(NoopLlmFactory), "minimax-m3");
        assert_eq!(organ.name(), "E7 Emergence");
        assert_eq!(organ.organ_id(), OrganKind::E7);
    }

    /// **0 装诚实**: PolicyStage 前向声明 5 variant 全可达 + is_active 正确
    /// (per 子代理 R7 独立判断: v1 emergence.rs 不含状态机, 此 enum 是 v2 前向声明,
    /// 当前 v2 E7 organ 不真接 evolution engine).
    #[test]
    fn policy_stage_5_forward_declared_with_active_check() {
        let stages = [
            PolicyStage::Idle,
            PolicyStage::Draft,
            PolicyStage::Proposed,
            PolicyStage::Ratified,
            PolicyStage::Active,
        ];
        assert_eq!(
            stages.len(),
            5,
            "5 状态机 = Idle/Draft/Proposed/Ratified/Active"
        );
        assert!(!PolicyStage::Idle.is_active());
        assert!(!PolicyStage::Draft.is_active());
        assert!(!PolicyStage::Proposed.is_active());
        assert!(PolicyStage::Ratified.is_active(), "已通过审议可发声");
        assert!(PolicyStage::Active.is_active(), "已激活在生效");
        // as_str 序列化稳定
        assert_eq!(PolicyStage::Idle.as_str(), "idle");
        assert_eq!(PolicyStage::Active.as_str(), "active");
    }

    /// **v2 适配**: epoch ms → day_key + minutes_of_day 派生正确
    /// (无 chrono 依赖, 手算 Gregorian).
    #[test]
    fn epoch_ms_to_day_key_and_minutes_deterministic() {
        // 2026-08-16 00:00:00 UTC = 20454 days (1970→2026-01-01) + 227 days (Jan 1→Aug 16) = 20681 days
        // = 20681 * 86_400_000 = 1_786_838_400_000 ms (per manual Gregorian 计算)
        let at_ms: i64 = 1_786_838_400_000;
        assert_eq!(day_key_from_epoch_ms(at_ms), "2026-08-16");
        let at_ms_with_minutes = at_ms + 8 * 3_600_000 + 40 * 60_000;
        assert_eq!(
            minutes_of_day_from_epoch_ms(at_ms_with_minutes),
            8 * 60 + 40
        );
        // 跨午夜: 23:59 UTC
        let at_ms_late = at_ms + 23 * 3_600_000 + 59 * 60_000;
        assert_eq!(minutes_of_day_from_epoch_ms(at_ms_late), 23 * 60 + 59);
        // 第二天 00:01
        let at_ms_next = at_ms + 24 * 3_600_000 + 60_000;
        assert_eq!(day_key_from_epoch_ms(at_ms_next), "2026-08-17");
        assert_eq!(minutes_of_day_from_epoch_ms(at_ms_next), 1);
    }
}

// ============================================
// module-private notes (per 子代理 R7 独立判断)
// ============================================
//
// **0 装诚实**: `HashMap` 已从 imports 删 (子代理 R7 决定: 真不用就删, 减编译噪音).
// Future 真生产路径 (ContextHint 记忆检索 key→value 映射) 真需要时再加.
//
// **0 触碰 LOCKED**: 本模块仅在 `crates/engine/organ/` 内部新增, 0 触碰:
// - 5 项 LOCKED (apeireth_locked_items.rs, baseline 0 改)
// - 8 哲学锚本体 (physics.rs 0 改)
// - 13 键 (gate/keys 0 改)
// - workspace.version (Cargo.toml workspace 0 改)
// - R11 baseline (body.rs 0 改)
// - `crates/engine/runtime/src/canonical/cognitive.rs` 12 slot ledger 0 改
// - Cargo.lock 0 行 diff (本模块不引新外部 dep)
