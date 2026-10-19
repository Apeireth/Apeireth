//! V3-a 双引擎主动关怀 (让它"活在生活里"的核心推进力)。
//!
//! 机制卡来源: 机制级设计哲学研究 · AI 伴侣/桌宠族「双引擎关怀」卡 —— 规则式
//! 时间关怀 (作息/纪念日/久未联系) + 模型式情境关怀**并行互兜底**: 关怀不能只
//! 靠模型"想起来", **规则兜底保证不漏**。主动面沿 initiative 语义
//! ([`crate::emergence`]) 的显式判定面: 判「该不该说 / 说什么」, 不做隐式生成。
//!
//! # 双引擎架构图
//!
//! ```text
//!                    care_rules.json (StoredDoc 拒开)
//!                    ┌──────────────────────────────┐
//!                    │ rules[]   四类触发器          │
//!                    │ discipline 打扰纪律三件       │
//!                    │ fired_at / history 发起留痕   │
//!                    └──────────────┬───────────────┘
//!                                   │ 时钟可注入 (冻结时钟可测)
//!                                   ▼
//!  引擎一 · 规则引擎 (确定性兜底, 保证不漏)
//!    inactive ──── N 小时无交互          ┐
//!    daily_moment 每日 HH:mm             │→ 关怀候选 (素材 + 时机)
//!    anniversary ─ 每年月日              │   CareCandidate
//!    interval ──── 每 N 小时            ┘
//!                                   │
//!                                   ▼
//!  打扰纪律 (先立规矩再谈温度, 顺序固定)
//!    ① 勿扰窗口 ② 专注抑制 ③ 频率上限 ④ 关怀预算 (同账本接缝)
//!    拦下 = Held (候选不丢, 重试)
//!                                   │
//!                                   ▼
//!  引擎二 · 模型情境引擎 (增色, 显式判定面)
//!    候选 + 近期记忆素材 → 结构化判定 { speak, focus, reason }
//!    ├─ 无模型配置 → RuleFallbackJudge 纯规则照发 (兜底)
//!    └─ 判定/解析失败 → 回退规则判定 (不漏发不乱发)
//!                                   │
//!                                   ▼
//!  输出面 · 标准回合 (零新消息通道)
//!    CareInitiative::to_turn_input() → TurnRequest
//!    → 生成的关怀消息进会话流 + IM 推送缝 (既有)
//!    → 计费落会话配额账本 (三维烧钱闸同一本账, 照常计费)
//! ```
//!
//! # 触发器词表 (闭合, 四类)
//!
//! | 词 (`kind`) | 语义 | 参数 | 到点条件 |
//! |---|---|---|---|
//! | `inactive` | 久未联系 | `hours` (1..=8784) | 距上次交互 N 小时无交互 |
//! | `daily_moment` | 作息时刻 | `minutes_of_day` (0..=1439, `HH:mm`) | 每日到点, 每日一次 |
//! | `anniversary` | 纪念日 | `month` (1..=12) + `day` (按月历) | 每年月日当天, 每年一次 |
//! | `interval` | 间隔循环 | `hours` (1..=8784) | 距上次发起每 N 小时 |
//!
//! 词外拒绝 (闭合词表, [`CareTrigger::parse_kind`]); 参数闭合校验, 非法拒绝即帧。
//!
//! # 分层 (面拆三层)
//!
//! - [`rules`]: 触发器数据面 —— `care_rules.json` StoredDoc 拒开 + 打扰纪律配置
//!   + 发起留痕 (写面归 write_intent 同族: 改规则 = 写文件);
//! - [`engine`]: 双引擎判定面 —— 规则引擎 + 判定面 + 打扰纪律 + 标准回合输入;
//! - [`reminder`]: 提醒侧消费端 (K 轴 L2/L5 同源接缝) —— 天气提醒信号 /
//!   守护告警 = 关怀候选的一种, 走同一套打扰纪律 (守护例外档: 破勿扰可配 +
//!   「守护告警」标注 + 独立频率上限);
//! - [`tool`]: 工具面 —— `tool.care_schedule` (增删列, RequireApproval 级) +
//!   治理钩子; 自述关怀节 (规则数 / 下次触发 / 最近关怀真值) 由组装根把
//!   [`CareStore::status`] 投影到 `tool.self_status` 的关怀节;
//! - [`ritual_candidate`] / [`reunion`]: 生物机制候选源 (W3.1) —— SG-22 仪式
//!   内核的 `Due` / `DueSoft` 与 SG-19 分离-重聚内核的非 `Brief` 重聚计划转成
//!   [`CareCandidate`], 由 [`CareEngine::poll_bio_candidate`] 走**同一条**打扰
//!   纪律管道 (勿扰 / 专注 / 频率 / 预算 → 发起闸 → 判定 → 台账); 默认关
//!   ([`CareConfig::bio_candidates_enabled`] = `false`), 关闭时永不产出。
//!
//! # 0 装诚实
//!
//! - 常驻心跳生产者 (定时 [`CareEngine::poll`]) 已交付在运行时
//!   `canonical::heartbeat` 调度缝 + runtime-assembly 装配
//!   (`CareHeartbeatProducer`, 开关 `APEIRETH_ENABLE_CARE_HEARTBEAT` 注册即闸
//!   默认关 fail-closed); 常驻宿主的启动注入点 (presence `try_take_initiative`
//!   同源闸 → [`CareInitiativeGate`], 以及自述关怀节的心跳态探测口) 已在
//!   常驻宿主接线 (cli 组合根 `care_heartbeat` 模块, 双闸齐开才起; 2026-10-17
//!   随行) —— 开关关/未起 = 心跳不自动起, 自述心跳两列显式 null (不冒充)。
//! - 交互观察 ([`CareEngine::observe_interaction`]) 是进程内值; 重启后
//!   久未联系类规则以规则创建时刻为锚 (如实)。
//! - 关怀预算闸 ([`CareBudgetGate`]) 是同账本接缝的前置问询; **记账本身**随
//!   标准回合落会话配额账本 (关怀回合照常计费, 无免费通道)。
//! - W3.1 生物机制候选源是 **crate 内接线**: 候选源 + 纪律管道入口已实现并有
//!   测试, 但 **runtime-assembly 尚未注入 replay 数据 / 打开开关** (待 W3.2);
//!   默认关 = 既有 care 行为逐字节不变, 不声称 PRODUCTION WIRED / DEFAULT
//!   ENABLED。

pub mod engine;
pub mod reminder;
pub mod reunion;
pub mod ritual_candidate;
pub mod rules;
pub mod tool;

/// care 调度接线配置 (生物机制候选源开关; W3.1 默认关, 可回退)。
///
/// 单独成类型而不是散在引擎字段里: 装配面 (runtime-assembly) 只需注入这一份
/// 配置; 开关关闭时 [`ritual_candidate::ritual_candidates_for`] /
/// [`reunion::reunion_candidate_for`] 恒返回空, 且
/// [`CareEngine::poll_bio_candidate`] 也拒绝发起 (双保险, 默认 fail-closed)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CareConfig {
    /// 仪式 / 重聚候选总开关。
    ///
    /// **默认 `false`**: 两类生物机制候选永不产出, 既有 care 行为 (规则 /
    /// 提醒 / 守护告警) 逐字节不变。W3.2 装配面才允许打开。
    pub bio_candidates_enabled: bool,
}

impl CareConfig {
    /// 生物机制候选是否启用 (默认 `false`)。
    pub const fn bio_candidates_enabled(&self) -> bool {
        self.bio_candidates_enabled
    }
}

pub use engine::{
    AlwaysAdmitBudget, AlwaysTakeInitiative, CareBudgetGate, CareBudgetVerdict, CareCandidate,
    CareClock, CareEngine, CareHold, CareInitiative, CareInitiativeGate, CareJudge, CareJudgeMode,
    CareJudgmentInput, CareMaterialSource, CareModelPort, CarePoll, CareVerdict, FnBudgetGate,
    FnInitiativeGate, FrozenCareClock, ModelContextJudge, NoRecentMaterial, RuleFallbackJudge,
    SystemCareClock,
};
pub use reminder::{
    consume_reminders, is_guard_alert_record, CareTier, MemoryReminderFeed, ReminderCandidate,
    ReminderConsumeError, ReminderFeed, GUARD_ALERT_ANNOTATION,
};
pub use reunion::{reunion_candidate, reunion_candidate_for};
pub use ritual_candidate::{ritual_candidates, ritual_candidates_for, RitualCandidatePriority};
pub use rules::{
    care_rules_doc_compat, validate_care_material, validate_care_rule_id, CareDiscipline,
    CareRecord, CareRule, CareRulesBody, CareStore, CareStoreError, CareStoreStatus, CareTrigger,
    CareWriteFace, CARE_HISTORY_MAX_COUNT, CARE_MATERIAL_MAX_BYTES, CARE_RULES_DOC_NAME,
    CARE_RULES_DOC_VERSION, CARE_RULES_FILE_NAME, CARE_RULES_MAX_COUNT, CARE_RULE_ID_MAX_BYTES,
    CARE_TRIGGER_HOURS_MAX,
};
pub use tool::{
    care_schedule_approval_reason, care_write_face_for_action, CareScheduleApprovalHook,
    CareScheduleTool, ACTION_CARE_ADD, ACTION_CARE_LIST, ACTION_CARE_REMOVE, CARE_SCHEDULE_ACTIONS,
    CARE_SCHEDULE_CAPABILITY_ID, CARE_SCHEDULE_TOOL_NAME,
};
