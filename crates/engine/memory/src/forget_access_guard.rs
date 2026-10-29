//! `apeireth-memory::forget_access_guard` — 「**先记访问史，再忘记**」的顺序纪律。
//!
//! # 为什么（这是 D0 保真的底线）
//!
//! D0 要求永久保留「**访问史**」⇒ 降权/归档/销毁的决策**必须**有访问史作依据
//! （设计稿 §3 D5 的权重公式里就有「访问史」这一项）。
//!
//! ⚠️ **危险顺序**：先销毁 ⇒ 访问史就没了 ⇒ **事后无法判断**「这条当初为什么被降权」
//! ⇒ 而且**不可逆**（append-only）⇒ 补不回来。
//!
//! ⇒ 本模块把这条顺序变成**类型层面的保证**：不先交访问事件，就**拿不到** forgets 的令牌。
//!
//! # 状态口径（四级）
//!
//! **NOT IMPLEMENTED（新增未接线）**：本模块只提供**类型 + 判定**，
//! 生产 forgets 入口（`universal_forget` / `memory_governance` / `forget_coordinator`）
//! **尚未接线**到本守卫 ⇒ 顺序纪律目前**只在调用本模块的代码里成立**，
//! 既有 forgets 路径仍可被直接调用（见「现状核实」）。
//!
//! # 现状核实（2026-10-26, W-18 实读代码）
//!
//! * **访问史怎么写的**：`access_history.rs` 的 `SqliteAccessHistoryStore::record_event`
//!   —— append-only 表 `memory_access_events`（带 `BEFORE DELETE` 触发器 ⇒ 物理删除被拒）
//!   + 同事务写小时聚合 `memory_access_aggregates`；读取走 `latest_accesses` / `latest_aggregates`。
//! * **forget 路径长什么样**：`universal_forget.rs`（`forget_episode` / `forget_principal` /
//!   `forget_principal_on_pool`，一个事务清 5 账 + 墓碑）、
//!   `memory_governance.rs`（`MemoryGovernanceStore::forget_episode`，governance sidecar 软删）、
//!   `forget_coordinator.rs`（三段式协调遗忘 P1 明文 / P2 派生标记 / P3 执行态清理）。
//! * **现在有没有「先记访问史」的保证**：**没有**。三条 forgets 路径都**不**写访问事件
//!   （`record_event` 的调用点全仓只有 `access_history.rs` 自身与 `context_compiler` 的
//!   检索观测面，forget 侧 0 调用）⇒ 本模块就是补这条缺口的第一块砖。
//!
//! # 与访问史存储的关系（不要误读）
//!
//! 本模块**不落库**。[`AccessEvent`] 必须是 `access_history::SqliteAccessHistoryStore::record_event`
//! **落库后**取回的那一条；守卫只对**已经记好的**事件签发回执 ⇒ 它是**顺序闸门**，
//! 不是**记录器**。⚠️ 因此本模块保证的是**顺序**（先交访问事件才拿得到令牌），
//! **不**保证**事件真实性**（调用方仍可能手工构造 `AccessEvent`）。
//! 「真实性」要靠接 `SqliteAccessHistoryStore` 反查来补 —— 属后续批次裁决项。

use crate::access_history::AccessEvent;

/// 守卫私有的「已签发」标记（零尺寸）。
///
/// # 为什么要有它
///
/// 顺序纪律要**不可绕过**，就不能让调用方用结构体字面量手工造一个
/// [`AccessReceipt`] / [`ForgetPermit`] 塞给 forgets 入口。
/// 零尺寸**私有**类型 + **私有**字段 ⇒ 模块外**无法**用字面量构造这两个结构
/// ⇒ 「先记访问史」这条路径成了**唯一**入口（`AccessGuard::record` → `AccessGuard::permit`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct GuardIssued;

/// **「可以执行忘记」的令牌**。
///
/// # 为什么用令牌（而不是布尔标志）
///
/// 布尔标志可以被**随手传 true** 绕过 ⇒ 令牌只能由
/// [`AccessReceipt`] 构造 ⇒ 而 [`AccessReceipt`] **只能**由本模块的
/// [`AccessGuard::record`] 产生 ⇒ **顺序不可绕过**。
#[derive(Debug, Clone, PartialEq)]
pub struct ForgetPermit {
    /// 本次记录的访问事件（**留痕** ⇒ 事后可查「销毁前记了什么」）。
    pub events: Vec<AccessEvent>,
    /// 记录时刻（毫秒）。
    pub recorded_at_ms: i64,
    /// 私有签发标记 ⇒ 外部**不能**手工伪造令牌（顺序纪律不可绕过的类型根据）。
    issued: GuardIssued,
}

/// 访问事件的**回执**（只能由守卫产生）。
#[derive(Debug, Clone, PartialEq)]
pub struct AccessReceipt {
    /// 已记录的访问事件。
    pub events: Vec<AccessEvent>,
    /// 记录时刻（毫秒）。
    pub recorded_at_ms: i64,
    /// 私有签发标记 ⇒ 外部**不能**手工伪造回执（否则 `permit` 会被绕过）。
    issued: GuardIssued,
}

/// 访问史守卫。
#[derive(Debug, Clone)]
pub struct AccessGuard {
    /// 记录时刻的时钟（**注入** ⇒ 测试可控、确定性）。
    now_ms: i64,
    /// 已记录的事件数（用于回执与审计）。
    recorded: usize,
}

impl AccessGuard {
    /// 构造（`now_ms` 注入 ⇒ 确定性）。
    pub fn new(now_ms: i64) -> Self {
        Self {
            now_ms,
            recorded: 0,
        }
    }

    /// **记录访问事件** ⇒ 产出回执。
    ///
    /// ⚠️ **空事件列表 ⇒ 返回 `None`**（**没访问过就没得记** ⇒ 不许拿空回执当通行证）。
    ///
    /// # 记账口径
    ///
    /// * 非空 ⇒ `recorded` **累加本批事件条数**（审计用，见 [`AccessGuard::recorded_count`]）；
    /// * 空批 ⇒ `None` 且**不记账**（一次空的 `record` 不是一次访问）。
    pub fn record(&mut self, events: Vec<AccessEvent>) -> Option<AccessReceipt> {
        if events.is_empty() {
            return None;
        }
        self.recorded = self.recorded.saturating_add(events.len());
        Some(AccessReceipt {
            events,
            recorded_at_ms: self.now_ms,
            issued: GuardIssued,
        })
    }

    /// 由回执换取「**可以忘记**」的令牌。
    ///
    /// # 为什么 `permit` 不再查库
    ///
    /// 回执本身就是「这批访问事件已经交给守卫」的凭据；`permit` 只做**凭据 → 令牌**的
    /// 转换 ⇒ 保持纯函数语义（不碰存储 ⇒ 可测、可审计）。
    pub fn permit(&self, receipt: AccessReceipt) -> ForgetPermit {
        debug_assert!(
            !receipt.events.is_empty(),
            "回执必须至少含一条访问事件 (record 对空批次返回 None)"
        );
        ForgetPermit {
            events: receipt.events,
            recorded_at_ms: receipt.recorded_at_ms,
            issued: GuardIssued,
        }
    }

    /// 已记录的事件总数（审计用）。
    pub fn recorded_count(&self) -> usize {
        self.recorded
    }
}

impl AccessReceipt {
    /// 自证：**该回执由访问史守卫签发**（恒为 `true`）。
    ///
    /// 外部无法手工构造 [`AccessReceipt`]（私有签发标记）⇒ 这个恒真断言是
    /// 「顺序纪律不可绕过」在类型上的**可机检**投影。
    pub fn is_issued(&self) -> bool {
        self.issued == GuardIssued
    }
}

impl ForgetPermit {
    /// 自证：**该令牌由访问史守卫签发**（恒为 `true`）。
    ///
    /// 外部无法手工构造 [`ForgetPermit`]（私有签发标记）⇒ 拿到令牌的路径**只有**
    /// `AccessGuard::record` → `AccessGuard::permit` ⇒ **顺序不可绕过**。
    pub fn is_issued(&self) -> bool {
        self.issued == GuardIssued
    }
}

/// **判断「访问史是否已覆盖待忘记的对象」**。
///
/// # 口径
///
/// * 对每个待忘记的 id ⇒ 必须有**至少一条**访问事件**提到它**
///   （[`AccessEvent::memory_id`] 与目标 id 相等即算提到）；
/// * ⇒ 返回**缺哪些 id**（**空 = 已覆盖**）⇒ 调用方据此**拒绝**「无访问史的销毁」。
///
/// # 重要
///
/// * **不修改任何状态**（纯判断）⇒ 便于先查后做；
/// * **缺一个就算不覆盖**（**不许「大部分覆盖就算」**）。
///
/// # 重复目标
///
/// 目标列表里的重复 id **只报一次**（保持首次出现顺序）⇒ 报错信息不会被同一个
/// 缺失 id 刷屏；缺失的**集合**语义不变。
pub fn missing_access_cover(events: &[AccessEvent], targets: &[String]) -> Vec<String> {
    let covered: std::collections::HashSet<&str> = events
        .iter()
        .map(|event| event.memory_id.as_str())
        .collect();
    let mut seen: std::collections::HashSet<&str> = std::collections::HashSet::new();
    let mut missing: Vec<String> = Vec::new();
    for target in targets {
        if !seen.insert(target.as_str()) {
            continue;
        }
        if !covered.contains(target.as_str()) {
            missing.push(target.clone());
        }
    }
    missing
}
