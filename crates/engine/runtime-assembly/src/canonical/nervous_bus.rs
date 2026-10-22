//! R1b-1a · 神经系统同步信号总线（ADR-0009 D1，内部私有）。
//!
//! 设计边界（与 `docs/adr/0009-r1b-nervous-bus.md` 一一对应）：
//! - **同步派发**：`publish` 返回前完成全部订阅者调用；无 channel、无异步任务、
//!   无第二套生命周期。订阅者只许做轻量状态推进（机制状态在既有内核里）。
//! - **内部私有**：不进入 `apeireth_runtime` 内核公共 ABI，不登记
//!   `contract.toml [extension_points]`；只在本 crate 与适配器装配面使用。
//! - **R1b-1a 不迁移任何机制**：默认零订阅者 = 发布是零副作用计数操作，
//!   行为与接线前逐字节一致。机制迁移在 R1b-1b（生物线）。
//! - **计数只读**：`published` / `delivered` / `subscribers` 供 R1b-4 台账
//!   additive v2 使用；本批**不写入** `tool.self_status`（schema v1 不动）。
//! - **不编值**：信号只承载既有事实（工具结果成败、错误文本截断），不携带
//!   推测的强度/情绪/时间；错误文本上限 [`MAX_SIGNAL_TEXT_CHARS`]。

use std::collections::BTreeMap;
use std::fmt;
use std::sync::Arc;

use parking_lot::{Mutex, RwLock};

/// 错误文本进入总线前的截断上限（字符数；防无界内存与超长日志）。
pub const MAX_SIGNAL_TEXT_CHARS: usize = 256;

/// 信号类别：机制可订阅的稳定词表。
///
/// 类别只在**生产者已接**时新增（R1b-1a: `HookPoint::AfterToolResult` /
/// `HookPoint::OnError` 两类；R1b-2c: 审批人决定三类），不得复用现有类别
/// 假装已接；其余缺失信号源（tone_feedback / sleep_pressure /
/// scar_ledger_learn / session_event_source）由 R1b-3 逐源定性。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum NervousSignalKind {
    /// 一次工具调用已产生结果（成功与失败都是观测事实）。
    ToolOutcome,
    /// 一次可观测失败（运行时或模块错误文本）。
    ErrorObserved,
    /// 一次人工审批决定（人批/人拒/人取消；R1b-2c 起由 CLI 审批路径发布）。
    ApprovalSocial,
}

impl NervousSignalKind {
    /// 全部类别（顺序稳定；台账 v2 与测试按此遍历）。
    pub const ALL: [Self; 3] = [Self::ToolOutcome, Self::ErrorObserved, Self::ApprovalSocial];

    /// 稳定短标签（台账/日志用，随 schema 冻结）。
    pub const fn label(self) -> &'static str {
        match self {
            Self::ToolOutcome => "tool_outcome",
            Self::ErrorObserved => "error_observed",
            Self::ApprovalSocial => "approval_social",
        }
    }
}

/// 人工审批决定的三态事实（R1b-2c）。
///
/// 只表达"人做了什么决定"，不携带理由文本/权限判定/情绪强度——
/// 理由属于审批面审计记录，不进总线（避免 PII 与二次真源）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ApprovalSocialOutcome {
    /// 人批准。
    Approved,
    /// 人拒绝。
    Rejected,
    /// 人取消（未形成批/拒结论）。
    Cancelled,
}

impl ApprovalSocialOutcome {
    /// 稳定短标签。
    pub const fn label(self) -> &'static str {
        match self {
            Self::Approved => "approved",
            Self::Rejected => "rejected",
            Self::Cancelled => "cancelled",
        }
    }
}

impl fmt::Display for NervousSignalKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

fn truncate_text(text: &str) -> String {
    if text.chars().count() <= MAX_SIGNAL_TEXT_CHARS {
        return text.to_string();
    }
    let mut truncated: String = text.chars().take(MAX_SIGNAL_TEXT_CHARS).collect();
    truncated.push('…');
    truncated
}

/// 总线上流动的一条信号。
///
/// 字段全部来自既有事实：工具结果的成功位/可重试位/工具名，或失败文本。
/// **不携带**推断出的强度、情绪方向或时间戳（时间由订阅机制用自己的时钟
/// 记录，避免总线成为第二个时间真源）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NervousSignal {
    /// 工具调用结果。`ok == false` 时 `retryable` 才可能为 `true`。
    ToolOutcome {
        /// 调用是否成功（`ToolResult::is_ok`）。
        ok: bool,
        /// 工具名（提供方未回显时为 `None`，不编造）。
        tool: Option<String>,
        /// 失败是否可重试（成功时恒 `false`）。
        retryable: bool,
    },
    /// 可观测失败（错误文本截断到 [`MAX_SIGNAL_TEXT_CHARS`]）。
    ErrorObserved {
        /// 失败文本（已截断）。
        message: String,
    },
    /// 人工审批决定（R1b-2c；来源标签截断规则同 [`MAX_SIGNAL_TEXT_CHARS`]）。
    ApprovalSocial {
        /// 人做出的决定三态。
        outcome: ApprovalSocialOutcome,
        /// 决定来源（如 `cli.approval`；截断，仅用于归因，不含理由文本）。
        source: String,
    },
}

impl NervousSignal {
    /// 工具结果信号（工具名截断规则同 [`MAX_SIGNAL_TEXT_CHARS`]）。
    ///
    /// `retryable` 只在失败时保留（成功结果不存在"可重试"语义，不编造）。
    pub fn tool_outcome(ok: bool, tool: Option<&str>, retryable: bool) -> Self {
        Self::ToolOutcome {
            ok,
            tool: tool.map(truncate_text),
            retryable: !ok && retryable,
        }
    }

    /// 错误观测信号（文本截断到 [`MAX_SIGNAL_TEXT_CHARS`]）。
    pub fn error_observed(message: &str) -> Self {
        Self::ErrorObserved {
            message: truncate_text(message),
        }
    }

    /// 人工审批决定信号（来源标签截断规则同 [`MAX_SIGNAL_TEXT_CHARS`]）。
    pub fn approval_social(outcome: ApprovalSocialOutcome, source: &str) -> Self {
        Self::ApprovalSocial {
            outcome,
            source: truncate_text(source),
        }
    }

    /// 本信号所属类别。
    pub const fn kind(&self) -> NervousSignalKind {
        match self {
            Self::ToolOutcome { .. } => NervousSignalKind::ToolOutcome,
            Self::ErrorObserved { .. } => NervousSignalKind::ErrorObserved,
            Self::ApprovalSocial { .. } => NervousSignalKind::ApprovalSocial,
        }
    }
}

type Subscriber = Arc<dyn Fn(&NervousSignal) + Send + Sync>;

/// 总线内部状态（只经 [`NervousBus`] 句柄访问）。
struct BusInner {
    subscribers: RwLock<BTreeMap<NervousSignalKind, Vec<Subscriber>>>,
    published: Mutex<BTreeMap<NervousSignalKind, u64>>,
    delivered: Mutex<u64>,
}

impl Default for BusInner {
    fn default() -> Self {
        Self {
            subscribers: RwLock::new(BTreeMap::new()),
            published: Mutex::new(BTreeMap::new()),
            delivered: Mutex::new(0),
        }
    }
}

/// 同步信号总线句柄（`clone` 共享同一总线；进程内私有）。
#[derive(Clone, Default)]
pub struct NervousBus {
    inner: Arc<BusInner>,
}

impl fmt::Debug for NervousBus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let snapshot = self.snapshot();
        f.debug_struct("NervousBus")
            .field("subscribers", &snapshot.subscribers)
            .field("published", &snapshot.published)
            .field("delivered", &snapshot.delivered)
            .finish()
    }
}

impl NervousBus {
    /// 一条空总线（零订阅者 = 发布零副作用）。
    pub fn new() -> Self {
        Self::default()
    }

    /// 订阅一类信号；回调在 [`Self::publish`] 的调用线程上**同步**执行。
    ///
    /// 返回订阅后该类别的订阅者总数（从 1 起）。回调必须是轻量、不阻塞；
    /// 回调内再次 `publish` 是允许的（锁只在收集/记账时短持有）。
    pub fn subscribe<F>(&self, kind: NervousSignalKind, callback: F) -> usize
    where
        F: Fn(&NervousSignal) + Send + Sync + 'static,
    {
        let mut subscribers = self.inner.subscribers.write();
        let list = subscribers.entry(kind).or_default();
        list.push(Arc::new(callback));
        list.len()
    }

    /// 同步发布一条信号；返回实际投递的订阅者数。
    ///
    /// 顺序：先无锁收集该类别订阅者快照 → 逐个回调（锁外）→ 记账。
    /// 某个订阅者 panic 会传播给调用方（与既有 `on_hook` 派发口径一致，
    /// 不吞异常）；投递计数在记账前完成，panic 时该次信号未计入 `delivered`。
    pub fn publish(&self, signal: &NervousSignal) -> usize {
        let kind = signal.kind();
        let subscribers: Vec<Subscriber> = {
            let guard = self.inner.subscribers.read();
            guard.get(&kind).cloned().unwrap_or_default()
        };
        for callback in &subscribers {
            callback(signal);
        }
        let delivered = subscribers.len() as u64;
        *self.inner.published.lock().entry(kind).or_insert(0) += 1;
        if delivered > 0 {
            *self.inner.delivered.lock() += delivered;
        }
        subscribers.len()
    }

    /// 某类别的订阅者数。
    pub fn subscriber_count(&self, kind: NervousSignalKind) -> usize {
        self.inner.subscribers.read().get(&kind).map_or(0, Vec::len)
    }

    /// 某类别已发布次数（订阅者数为 0 时也计数 = 台账可证"发过但没人听"）。
    pub fn published_count(&self, kind: NervousSignalKind) -> u64 {
        self.inner.published.lock().get(&kind).copied().unwrap_or(0)
    }

    /// 全部类别累计投递次数。
    pub fn delivered_total(&self) -> u64 {
        *self.inner.delivered.lock()
    }

    /// 是否零订阅者（默认装配态；R1b-1a 恒为 `true`）。
    pub fn is_idle(&self) -> bool {
        self.inner
            .subscribers
            .read()
            .values()
            .all(|list| list.is_empty())
    }

    /// 只读计数快照（R1b-4 台账 additive v2 的数据面）。
    pub fn snapshot(&self) -> NervousBusSnapshot {
        let subscribers = self.inner.subscribers.read();
        let published = self.inner.published.lock();
        NervousBusSnapshot {
            subscribers: NervousSignalKind::ALL
                .into_iter()
                .map(|kind| (kind, subscribers.get(&kind).map_or(0, Vec::len)))
                .collect(),
            published: NervousSignalKind::ALL
                .into_iter()
                .map(|kind| (kind, published.get(&kind).copied().unwrap_or(0)))
                .collect(),
            delivered: *self.inner.delivered.lock(),
        }
    }
}

/// 总线只读计数快照（`BTreeMap` 键为类别枚举，顺序由 `Ord` 决定）。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct NervousBusSnapshot {
    /// 每类别订阅者数。
    pub subscribers: BTreeMap<NervousSignalKind, usize>,
    /// 每类别发布次数。
    pub published: BTreeMap<NervousSignalKind, u64>,
    /// 全部类别累计投递次数。
    pub delivered: u64,
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;

    #[test]
    fn empty_bus_publish_is_side_effect_free_but_counted() {
        let bus = NervousBus::new();
        assert!(bus.is_idle());
        let delivered = bus.publish(&NervousSignal::tool_outcome(true, Some("tool.x"), false));
        assert_eq!(delivered, 0);
        assert_eq!(bus.published_count(NervousSignalKind::ToolOutcome), 1);
        assert_eq!(bus.delivered_total(), 0);
        let snapshot = bus.snapshot();
        assert_eq!(snapshot.published[&NervousSignalKind::ToolOutcome], 1);
        assert_eq!(snapshot.delivered, 0);
    }

    #[test]
    fn subscribers_receive_only_their_kind_synchronously() {
        let bus = NervousBus::new();
        let tool_hits = Arc::new(AtomicUsize::new(0));
        let error_hits = Arc::new(AtomicUsize::new(0));
        let tool_probe = Arc::clone(&tool_hits);
        bus.subscribe(NervousSignalKind::ToolOutcome, move |_| {
            tool_probe.fetch_add(1, Ordering::SeqCst);
        });
        let error_probe = Arc::clone(&error_hits);
        bus.subscribe(NervousSignalKind::ErrorObserved, move |_| {
            error_probe.fetch_add(1, Ordering::SeqCst);
        });

        assert_eq!(
            bus.publish(&NervousSignal::tool_outcome(false, None, true)),
            1
        );
        // 同步：publish 返回即已回调。
        assert_eq!(tool_hits.load(Ordering::SeqCst), 1);
        assert_eq!(error_hits.load(Ordering::SeqCst), 0);

        assert_eq!(bus.publish(&NervousSignal::error_observed("boom")), 1);
        assert_eq!(error_hits.load(Ordering::SeqCst), 1);
        assert_eq!(bus.subscriber_count(NervousSignalKind::ToolOutcome), 1);
        assert_eq!(bus.delivered_total(), 2);
        assert!(!bus.is_idle());
    }

    #[test]
    fn multiple_subscribers_share_one_signal_and_count_deliveries() {
        let bus = NervousBus::new();
        let first = Arc::new(AtomicUsize::new(0));
        let second = Arc::new(AtomicUsize::new(0));
        let first_probe = Arc::clone(&first);
        let second_probe = Arc::clone(&second);
        bus.subscribe(NervousSignalKind::ToolOutcome, move |_| {
            first_probe.fetch_add(1, Ordering::SeqCst);
        });
        bus.subscribe(NervousSignalKind::ToolOutcome, move |_| {
            second_probe.fetch_add(1, Ordering::SeqCst);
        });

        let delivered = bus.publish(&NervousSignal::tool_outcome(true, Some("tool.y"), false));
        assert_eq!(delivered, 2);
        assert_eq!(first.load(Ordering::SeqCst), 1);
        assert_eq!(second.load(Ordering::SeqCst), 1);
        assert_eq!(bus.delivered_total(), 2);
    }

    #[test]
    fn signal_fields_come_from_facts_and_truncate_text() {
        let long = "x".repeat(MAX_SIGNAL_TEXT_CHARS + 40);
        let signal = NervousSignal::error_observed(&long);
        let NervousSignal::ErrorObserved { message } = &signal else {
            panic!("必须映射为错误观测");
        };
        assert_eq!(message.chars().count(), MAX_SIGNAL_TEXT_CHARS + 1); // + 省略号
        assert!(message.ends_with('…'));
        assert_eq!(signal.kind(), NervousSignalKind::ErrorObserved);

        // 成功结果不保留可重试位（不编造"成功但可重试"）。
        let signal = NervousSignal::tool_outcome(true, Some("tool.z"), true);
        assert_eq!(
            signal,
            NervousSignal::ToolOutcome {
                ok: true,
                tool: Some("tool.z".to_string()),
                retryable: false,
            }
        );
        assert_eq!(signal.kind(), NervousSignalKind::ToolOutcome);
    }

    #[test]
    fn approval_social_signal_carries_the_decision_outcome_only() {
        let bus = NervousBus::new();
        let seen = Arc::new(Mutex::new(Vec::new()));
        let probe = Arc::clone(&seen);
        bus.subscribe(NervousSignalKind::ApprovalSocial, move |signal| {
            probe.lock().push(signal.clone());
        });

        assert_eq!(
            bus.publish(&NervousSignal::approval_social(
                ApprovalSocialOutcome::Approved,
                "cli.approval",
            )),
            1
        );
        assert_eq!(
            seen.lock().clone(),
            vec![NervousSignal::ApprovalSocial {
                outcome: ApprovalSocialOutcome::Approved,
                source: "cli.approval".to_string(),
            }]
        );
        assert_eq!(bus.published_count(NervousSignalKind::ApprovalSocial), 1);
        // 其它类别零计数: 新类别不与工具结局/错误混流。
        assert_eq!(bus.published_count(NervousSignalKind::ToolOutcome), 0);
        assert_eq!(bus.published_count(NervousSignalKind::ErrorObserved), 0);
        assert_eq!(NervousSignalKind::ALL.len(), 3);
    }

    #[test]
    fn clone_handle_shares_one_bus() {
        let bus = NervousBus::new();
        let observer = Arc::new(AtomicUsize::new(0));
        let probe = Arc::clone(&observer);
        bus.subscribe(NervousSignalKind::ErrorObserved, move |_| {
            probe.fetch_add(1, Ordering::SeqCst);
        });
        let handle = bus.clone();
        handle.publish(&NervousSignal::error_observed("shared"));
        assert_eq!(observer.load(Ordering::SeqCst), 1);
        assert_eq!(bus.delivered_total(), 1);
    }
}
