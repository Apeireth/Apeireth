//! bio_social_repair —— SG-20 交互修复观测面（Wave 4 收口批；默认关）。
//!
//! # 生物原型
//!
//! 对话质量的大头不是「一次说对」，而是**错了之后能不能修**：误解一旦出现，
//! 先对齐对象与理解、再继续堆信息才划算（心智理论 / 共同注意 / 修复动作）。
//! 机制内核见 `apeireth_organ::social_repair`（W1 批，纯函数、固定规则表）。
//!
//! # 本模块接的**真实事件源**（不是模拟、不是编造）
//!
//! SG-20 的行为面仍未接线（不进 prompt 组装），本波只接**观测面**：
//!
//! - 事件源 = 人工审批被拒：`NervousSignalKind::ApprovalSocial` 且
//!   `ApprovalSocialOutcome::Rejected`（由 `bio_approval_social` 在人工介入后发布）。
//!   语义 = 「用户明确否定我方提案」——属内核规则表里的
//!   [`UncertaintySignal::UserCorrection`]（rank 1），因此修复计划可确定性算出；
//! - 订阅者把每次拒绝折算成一份 [`RepairPlan`]（至多 2 个动作、至多 1 个问句），
//!   只做**只读观测**：不投递、不写 prompt、不改审批结论、不落盘；
//! - 关闭（修复指引族开关未开）= 不构造订阅者 = 零订阅、零计数、零计划。
//!
//! # 工程边界（明确不做什么）
//!
//! - 不模拟心智理论、不推断动机或情绪（内核已声明；本模块不新增推理）；
//! - 不生成对话文本：观测值里只有动作名与内核 `rationale`（不含用户原话）；
//! - 不把拒绝理由写进信号/观测（理由属 PII，`bio_approval_social` 发布时已丢弃）；
//! - 不读系统时钟、不落盘、无副作用（帧内 `now_ms` 不参与计算）。
//!
//! # 能力级别（四级口径）
//!
//! **PRODUCTION WIRED（仅观测面，默认关）**：装配根在修复指引族开关开时构造本源并
//! 注册总线订阅者；自述 `capabilities.bio_mechanisms` 的 `social_repair` 格读取本源。
//! 行为面（把 `moves` 写成 prompt 修复指令 / 据此决定是否提问）**仍未接线**，
//! 不得写成已生效。

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use apeireth_organ::social_repair::{plan_repair, RepairPlan, UncertaintySignal};
use apeireth_tools_canonical::self_status::{BioMechanismProbe, BioMechanismReading};
use serde_json::json;

use crate::canonical::cognitive::BioAssemblySwitches;
use crate::canonical::nervous_bus::{ApprovalSocialOutcome, NervousBus, NervousSignal};

/// 观测事件来源的稳定标签（写进自述值，便于人工复核）。
pub const SOCIAL_REPAIR_EVENT_BASIS: &str = "human_rejected_approval";

/// 观测值里 `basis` 的中文说明（只描述事实，不夸大）。
pub const SOCIAL_REPAIR_BASIS: &str =
    "人工拒绝审批 = 用户明确否定我方提案（内核规则表 UserCorrection；只读观测，不改行为）";

/// 一次拒绝折算出的观测记录（纯数据）。
#[derive(Debug, Clone, PartialEq)]
pub struct SocialRepairObservation {
    /// 累计观测到的人工拒绝次数。
    pub rejections_seen: u64,
    /// 最近一次拒绝由内核规则表算出的修复计划。
    pub last_plan: RepairPlan,
}

/// SG-20 观测源：保存计数与最近计划，供自述 probe 只读读取。
///
/// 唯一写入口是 [`Self::handle_signal`]（总线订阅回调），锁毒化时保守忽略
/// （观测失败不 panic、不污染运行期）；计数用原子量，与最近计划分开存。
pub struct SocialRepairObservationSource {
    switches: BioAssemblySwitches,
    rejections: AtomicU64,
    last: Mutex<Option<SocialRepairObservation>>,
}

impl SocialRepairObservationSource {
    /// 构造观测源（`switches` 由装配根一次性解析后传入，本模块不读 env）。
    #[must_use]
    pub fn new(switches: BioAssemblySwitches) -> Self {
        Self {
            switches,
            rejections: AtomicU64::new(0),
            last: Mutex::new(None),
        }
    }

    /// 装配面开关快照（观测格据此区分「未接线」与「已接线但零事件」）。
    #[must_use]
    pub const fn switches(&self) -> BioAssemblySwitches {
        self.switches
    }

    /// 累计观测到的人工拒绝次数。
    #[must_use]
    pub fn rejections_seen(&self) -> u64 {
        self.rejections.load(Ordering::Relaxed)
    }

    /// 最近一次拒绝的观测记录（无拒绝 = `None`）。
    #[must_use]
    pub fn last(&self) -> Option<SocialRepairObservation> {
        self.last
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }

    /// 把一次「人工拒绝」折算为修复计划并记账（返回值便于测试与日志断言）。
    ///
    /// 输入固定为 [`UncertaintySignal::UserCorrection`]：拒绝 = 用户明确否定我方
    /// 提案；`detail` 只写来源标签（不含用户原话/理由），权重取 1.0（明确否定）。
    pub fn record_rejection(&self) -> RepairPlan {
        let signals = [UncertaintySignal::UserCorrection {
            detail: SOCIAL_REPAIR_EVENT_BASIS.to_owned(),
            weight: 1.0,
        }];
        let plan = plan_repair(&signals, None);
        let rejections = self.rejections.fetch_add(1, Ordering::Relaxed) + 1;
        let mut last = self
            .last
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        *last = Some(SocialRepairObservation {
            rejections_seen: rejections,
            last_plan: plan.clone(),
        });
        plan
    }

    /// 处理一条总线信号；仅 `ApprovalSocial{Rejected}` 被计入。
    ///
    /// 返回 `true` 表示本信号被本观测源消费（用于订阅测试断言）。
    pub fn handle_signal(&self, signal: &NervousSignal) -> bool {
        match signal {
            NervousSignal::ApprovalSocial {
                outcome: ApprovalSocialOutcome::Rejected,
                ..
            } => {
                let _ = self.record_rejection();
                true
            }
            _ => false,
        }
    }

    /// 订阅总线上的审批社交信号（同样只受一次），返回该类订阅者数。
    ///
    /// 装配根只在开关开时调用；关 = 零订阅 = 零计数。回调捕获的是同一个
    /// `Arc`（计数与最近计划在调用方与订阅者之间共享，不是拷贝）。
    pub fn subscribe(self: &Arc<Self>, bus: &NervousBus) -> usize {
        let source = Arc::clone(self);
        bus.subscribe(
            crate::canonical::nervous_bus::NervousSignalKind::ApprovalSocial,
            move |signal| {
                let _ = source.handle_signal(signal);
            },
        )
    }
}

/// 自述 probe：把观测源渲染成单格读数。
pub struct SocialRepairProbe {
    source: Arc<SocialRepairObservationSource>,
}

impl SocialRepairProbe {
    /// 从观测源构造 probe（装配根迟绑定槽用）。
    #[must_use]
    pub fn new(source: Arc<SocialRepairObservationSource>) -> Self {
        Self { source }
    }
}

impl BioMechanismProbe for SocialRepairProbe {
    fn snapshot(&self, _now_ms: i64) -> Result<BioMechanismReading, String> {
        if !self.source.switches().repair_guidance() {
            return Ok(BioMechanismReading::not_wired(
                "修复指引族开关未开：社会修复观测默认关（开 = 订阅人工拒绝信号并只读记账）",
            ));
        }
        let last = self.source.last();
        let last_plan = last.as_ref().map(|observation| {
            let plan = &observation.last_plan;
            json!({
                "moves": plan.moves.iter().map(|movement| movement.as_str()).collect::<Vec<_>>(),
                "ask_user": plan.ask_user,
                "max_moves": plan.max_moves,
                "rationale": plan.rationale,
            })
        });
        let value = json!({
            "repair_guidance_enabled": true,
            "event_basis": SOCIAL_REPAIR_EVENT_BASIS,
            "rejections_seen": self.source.rejections_seen(),
            "last_plan": last_plan,
            "basis": SOCIAL_REPAIR_BASIS,
        });
        Ok(BioMechanismReading::wired(Some(value), None, None))
    }
}

/// 便捷构造：把观测源包成 [`BioMechanismProbe`]。
#[must_use]
pub fn social_repair_bio_probe(
    source: Arc<SocialRepairObservationSource>,
) -> Arc<dyn BioMechanismProbe> {
    Arc::new(SocialRepairProbe::new(source))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn on_switches() -> BioAssemblySwitches {
        BioAssemblySwitches::from_flags(true, false)
    }

    fn off_switches() -> BioAssemblySwitches {
        BioAssemblySwitches::disabled()
    }

    fn rejection_signal() -> NervousSignal {
        NervousSignal::approval_social(
            crate::canonical::nervous_bus::ApprovalSocialOutcome::Rejected,
            "cli.approval",
        )
    }

    #[test]
    fn rejection_signal_records_one_plan_and_ignores_other_outcomes() {
        let source = SocialRepairObservationSource::new(on_switches());
        assert!(source.handle_signal(&rejection_signal()));
        assert_eq!(source.rejections_seen(), 1);
        let last = source.last().expect("一次拒绝后必有观测记录");
        assert_eq!(last.rejections_seen, 1);
        assert_eq!(last.last_plan.moves.len(), 2);
        assert_eq!(last.last_plan.moves[0].as_str(), "acknowledge_uncertainty");
        assert_eq!(last.last_plan.moves[1].as_str(), "restate_understanding");
        assert!(!last.last_plan.ask_user, "修复计划不含问句类动作");
        assert_eq!(last.last_plan.max_moves, 2);

        let approved =
            NervousSignal::approval_social(ApprovalSocialOutcome::Approved, "cli.approval");
        let cancelled =
            NervousSignal::approval_social(ApprovalSocialOutcome::Cancelled, "cli.approval");
        assert!(!source.handle_signal(&approved));
        assert!(!source.handle_signal(&cancelled));
        assert_eq!(source.rejections_seen(), 1, "只有拒绝计入社会摩擦");
    }

    #[test]
    fn switch_off_reports_not_wired_with_a_concrete_reason() {
        let source = Arc::new(SocialRepairObservationSource::new(off_switches()));
        let probe = SocialRepairProbe::new(source);
        let reading = probe.snapshot(0).expect("未接线不是错误");
        assert!(!reading.wired);
        assert!(reading.value.is_none());
        let reason = reading.reason.expect("未接线必须带真因");
        assert!(
            reason.contains("修复指引族开关未开"),
            "真因要能让人看懂: {reason}"
        );
    }

    #[test]
    fn switch_on_with_zero_rejections_is_wired_and_honest() {
        let source = Arc::new(SocialRepairObservationSource::new(on_switches()));
        let probe = SocialRepairProbe::new(source);
        let reading = probe.snapshot(0).expect("零事件是合法读数");
        assert!(reading.wired);
        let value = reading.value.expect("开时必带值");
        assert_eq!(value["rejections_seen"], json!(0));
        assert_eq!(value["last_plan"], serde_json::Value::Null);
        assert_eq!(value["event_basis"], json!(SOCIAL_REPAIR_EVENT_BASIS));
    }

    #[test]
    fn subscription_delivers_only_rejections_to_the_shared_source() {
        let source = Arc::new(SocialRepairObservationSource::new(on_switches()));
        let bus = NervousBus::new();
        assert_eq!(source.subscribe(&bus), 1);
        assert_eq!(
            bus.publish(&rejection_signal()),
            1,
            "拒绝信号应投递给订阅者"
        );
        assert_eq!(source.rejections_seen(), 1);
        let tool = NervousSignal::tool_outcome(true, Some("filesystem"), false);
        let _ = bus.publish(&tool);
        assert_eq!(source.rejections_seen(), 1, "工具结局不是社会摩擦");
    }
}
