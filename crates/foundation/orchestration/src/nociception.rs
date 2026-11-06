//! `apeireth-orchestration::nociception` — 痛觉学习 (SG-16): 把失败转成保护性学习信号.
//!
//! # 生物原型
//! - **痛是保护性学习信号，不是伤害本身**: 伤害感受器把组织损伤编码成「该避开的证据」，
//!   驱动撤退/休息/求诊；它不负责持续输出同等强度的警报。
//! - **习惯化 (habituation)**: 同类、可自愈的重复刺激 → 反应逐次减弱，避免过反应。
//! - **慢性化 (chronification)**: 持续或跨会话反复的异常输入让回路自己「学会痛」
//!   （阈值下调 = 敏化）；此时正确动作是诊断根因，而不是继续重试。
//! - **脱敏 (desensitization)**: 损伤修复后急性敏化回落；「这里受过伤」的痕迹保留在
//!   `recovery_protocol::ScarLedger`，不等于把记忆删掉。
//!
//! # 工程边界
//! - **纯内存 + 注入时钟**: 时间一律 `u64` 毫秒由调用方注入；本模块 0 系统时钟、0 `chrono`。
//! - **固定容量**: 按 `(capability, cause_key)` 归组，最多 `capacity` 组；超限时按
//!   「最久未触达 → capability 字典序 → cause_key 字典序」淘汰（确定性，可复核）。
//! - **只给判定，不做动作**: 只产出 [`PainAssessment`]（失败分类 + 习惯化 + 慢性 + 建议），
//!   重试/退避/熔断的实际执行在调用方；本模块不持有 provider、不发请求、不写日志。
//! - **会话边界用时间间隙近似**: 同组相邻信号间隔 > `session_gap_ms` 记为一个新会话，
//!   用于「跨会话反复」判据；它不是真实会话 ID（W2 若拿到真会话 id 可替换该近似）。
//! - **外部类不入账**: [`FailureClass::External`] 不是「我们可学可修的痛」，只回
//!   [`Recommendation::Backoff`]，不写账本、不累计慢性。
//! - **0 unsafe / 0 新外部依赖 / 0 系统时间依赖**。
//!
//! # IMPLEMENTED；装配面已接（默认关，2026-10-25 核对）
//!
//! 生产经 `RuntimeBuilder::with_bio_pain` 接上（开关 `APEIRETH_ENABLE_BIO_PAIN` /
//! `bio-mechanisms.json`，默认关）；开启后痛觉评估进 runtime 快照与自述 SG-16 格。
//! 原文「未接线」已过期 —— 有装配面，只是默认不开。
//! 本模块是机制内核 v0：类型、判定规则、固定容量与单元测试齐备，落 **IMPLEMENTED** 级；
//! **未接 `harness_patch`、未进失败处理链、无 `self_status` 自述节**，
//! 生产路径 0 调用点（缺口见
//! `docs/04-internal/backend-enhancement/system-gap-ledger-2026-10-18.md` SG-16）。

use serde::{Deserialize, Serialize};

/// 默认账本容量（可区分的 `(capability, cause_key)` 组数上限）。
pub const DEFAULT_CAPACITY: usize = 64;
/// 慢性化默认观察窗（毫秒）: 10 分钟。
pub const DEFAULT_CHRONIC_WINDOW_MS: u64 = 10 * 60 * 1000;
/// 慢性化默认窗口内次数阈值（含临界值）。
pub const DEFAULT_CHRONIC_THRESHOLD: u32 = 4;
/// 慢性化默认跨会话次数阈值（窗口外反复）。
pub const DEFAULT_CHRONIC_SESSIONS: u32 = 3;
/// 习惯化默认窗口内次数阈值（低于慢性阈值，先习惯化再慢性化）。
pub const DEFAULT_HABITUATION_THRESHOLD: u32 = 3;
/// 会话切分默认间隙（毫秒）: 30 分钟无信号即视为新会话。
pub const DEFAULT_SESSION_GAP_MS: u64 = 30 * 60 * 1000;
/// 「修复后仍复发」默认次数阈值（达到即建议放弃该路径）。
pub const DEFAULT_ABANDON_REPAIR_FAILURES: u32 = 2;
/// 单组窗口内样本保留上限（防无界增长）。
pub const DEFAULT_MAX_SAMPLES: usize = 64;
/// 急性敏化权重上限（仅供 W2 估算退避幅度，本批不参与判定）。
pub const SENSITIVITY_MAX: u32 = 4;

/// 失败类别: 决定「值不值得学」与「该不该升级」.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FailureClass {
    /// 瞬时: 可自愈的抖动（超时/限流/短暂不可用）；重复出现先习惯化。
    Transient,
    /// 系统性: 我们自己的确定性缺陷（逻辑错/契约违背/配置错）；不可习惯化。
    Systematic,
    /// 外部: 上游/网络/第三方故障；只能退避，不记慢性（不是可修的学习信号）。
    External,
}

impl FailureClass {
    /// 稳定短名（日志/证据串用，不随 `Debug` 派生漂移）。
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Transient => "transient",
            Self::Systematic => "systematic",
            Self::External => "external",
        }
    }
}

/// 一次失败事件（调用方注入；本模块不感知具体子系统）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FailureSignal {
    /// 受影响能力标识（如 `tool.web_fetch` / `provider.chat`）。
    pub capability: String,
    /// 原因归一键（由调用方归一化，如 `http_403` / `schema_mismatch`）。
    pub cause_key: String,
    /// 失败类别。
    pub class: FailureClass,
    /// 事件时间（毫秒，调用方注入）。
    pub at_ms: u64,
}

/// 同类失败的反应方向: 正常 / 敏化 / 习惯化.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Habituation {
    /// 正常: 尚未形成习惯化或敏化。
    Normal,
    /// 敏化: 慢性化压过习惯化，回路对该组合更敏感（不可继续无脑重试）。
    Sensitized,
    /// 习惯化: 同类瞬时失败重复后反应降级、不再升级。
    Habituated,
}

impl Habituation {
    /// 稳定短名。
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::Sensitized => "sensitized",
            Self::Habituated => "habituated",
        }
    }
}

/// 处置建议（本模块只建议，不执行）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Recommendation {
    /// 重试: 瞬时失败且未慢性化（含习惯化后的低敏感重试）。
    Retry,
    /// 退避: 外部故障，等环境恢复；不学习、不升级。
    Backoff,
    /// 诊断: 慢性化或系统性缺陷，停止重试、定位根因。
    Diagnose,
    /// 放弃: 慢性化且多次修复后仍复发，隔离该路径并保留瘢痕。
    Abandon,
}

impl Recommendation {
    /// 稳定短名。
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Retry => "retry",
            Self::Backoff => "backoff",
            Self::Diagnose => "diagnose",
            Self::Abandon => "abandon",
        }
    }
}

/// 单次评估结果（纯数据，可审计、可序列化）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PainAssessment {
    /// 本次信号类别（与 [`FailureSignal::class`] 一致）。
    pub class: FailureClass,
    /// 反应方向。
    pub habituation: Habituation,
    /// 是否已慢性化（窗口内重复或跨会话反复）。
    pub chronic: bool,
    /// 处置建议。
    pub recommended: Recommendation,
    /// 判定理由（含阈值与计数，确定性可复核）。
    pub rationale: String,
}

/// 痛觉账本的判定参数.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PainConfig {
    /// 组数上限（固定容量；至少 1）。
    pub capacity: usize,
    /// 慢性观察窗（毫秒，边界含等号）。
    pub chronic_window_ms: u64,
    /// 窗口内次数达到该值 → 慢性（至少 1）。
    pub chronic_threshold: u32,
    /// 未修复会话数达到该值 → 慢性（至少 1）。
    pub chronic_sessions: u32,
    /// 窗口内次数达到该值 → 习惯化（至少 1）。
    pub habituation_threshold: u32,
    /// 会话切分间隙（毫秒）。
    pub session_gap_ms: u64,
    /// 修复后复发次数达到该值 → 建议放弃（至少 1）。
    pub abandon_repair_failures: u32,
    /// 单组窗口样本保留上限（至少 1）。
    pub max_samples: usize,
}

impl Default for PainConfig {
    fn default() -> Self {
        Self {
            capacity: DEFAULT_CAPACITY,
            chronic_window_ms: DEFAULT_CHRONIC_WINDOW_MS,
            chronic_threshold: DEFAULT_CHRONIC_THRESHOLD,
            chronic_sessions: DEFAULT_CHRONIC_SESSIONS,
            habituation_threshold: DEFAULT_HABITUATION_THRESHOLD,
            session_gap_ms: DEFAULT_SESSION_GAP_MS,
            abandon_repair_failures: DEFAULT_ABANDON_REPAIR_FAILURES,
            max_samples: DEFAULT_MAX_SAMPLES,
        }
    }
}

impl PainConfig {
    /// 夹紧退化配置（0 容量/0 阈值/0 样本上限一律抬到 1），保证判定有定义、无 panic。
    fn normalized(mut self) -> Self {
        self.capacity = self.capacity.max(1);
        self.chronic_threshold = self.chronic_threshold.max(1);
        self.chronic_sessions = self.chronic_sessions.max(1);
        self.habituation_threshold = self.habituation_threshold.max(1);
        self.abandon_repair_failures = self.abandon_repair_failures.max(1);
        self.max_samples = self.max_samples.max(1);
        self
    }
}

/// 单个 `(capability, cause_key)` 组的可观测快照.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PainGroup {
    /// 能力标识。
    pub capability: String,
    /// 原因归一键。
    pub cause_key: String,
    /// 最近一次信号的类别（同类组内类别可被更新覆盖）。
    pub class: FailureClass,
    /// 累计信号次数（含已修复前历史，只增不减）。
    pub hits: u64,
    /// 未修复的连续会话数（修复后下一次信号从 1 重新计）。
    pub sessions: u32,
    /// 修复后仍复发的次数（瘢痕式记忆，不随脱敏清零）。
    pub repair_failures: u32,
    /// 急性敏化权重（1..=[`SENSITIVITY_MAX`]；修复后回落到 1）。
    pub sensitivity: u32,
    /// 最近一次信号时间（单调不回退）。
    pub last_ms: u64,
    /// 最近一次修复时间（0 = 从未修复）。
    pub last_repair_ms: u64,
    /// 窗口内样本时间（已裁剪；私有，仅供判定与复核）。
    samples: Vec<u64>,
    /// 是否处于「刚修复、等待验证未复发」状态。
    pending_repair: bool,
}

impl PainGroup {
    fn new(signal: &FailureSignal) -> Self {
        Self {
            capability: signal.capability.clone(),
            cause_key: signal.cause_key.clone(),
            class: signal.class,
            hits: 0,
            sessions: 0,
            repair_failures: 0,
            sensitivity: 1,
            last_ms: signal.at_ms,
            last_repair_ms: 0,
            samples: Vec::new(),
            pending_repair: false,
        }
    }

    /// 窗口内样本数（`samples` 已按最新信号时刻裁剪）。
    fn window_hits(&self) -> u64 {
        u64::try_from(self.samples.len()).unwrap_or(u64::MAX)
    }
}

/// 固定容量痛觉账本: 按 `(capability, cause_key)` 归组学习，可脱敏、可淘汰.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PainLedger {
    groups: Vec<PainGroup>,
    config: PainConfig,
}

impl Default for PainLedger {
    fn default() -> Self {
        Self::new(DEFAULT_CAPACITY)
    }
}

impl PainLedger {
    /// 以容量构造账本（容量 0 抬到 1）。
    pub fn new(capacity: usize) -> Self {
        Self::with_config(PainConfig {
            capacity,
            ..PainConfig::default()
        })
    }

    /// 以完整参数构造账本（退化参数在构造时夹紧）。
    pub fn with_config(config: PainConfig) -> Self {
        Self {
            groups: Vec::new(),
            config: config.normalized(),
        }
    }

    /// 组数上限。
    pub fn capacity(&self) -> usize {
        self.config.capacity
    }

    /// 当前组数。
    pub fn len(&self) -> usize {
        self.groups.len()
    }

    /// 账本是否为空。
    pub fn is_empty(&self) -> bool {
        self.groups.is_empty()
    }

    /// 只读配置（已夹紧）。
    pub fn config(&self) -> &PainConfig {
        &self.config
    }

    /// 全部组快照（插入序）。
    pub fn groups(&self) -> &[PainGroup] {
        &self.groups
    }

    /// 按 `(capability, cause_key)` 查组。
    pub fn get(&self, capability: &str, cause_key: &str) -> Option<&PainGroup> {
        self.groups
            .iter()
            .find(|g| g.capability == capability && g.cause_key == cause_key)
    }

    /// 评估一次失败信号: 学习（外部类除外）并返回处置建议.
    ///
    /// 判定优先级（文档化规则，测试覆盖边界）:
    /// 1. [`FailureClass::External`] → `Backoff`，不写账本；
    /// 2. 慢性化（窗口内次数 ≥ `chronic_threshold` 或未修复会话数 ≥ `chronic_sessions`）
    ///    → `Sensitized`；
    /// 3. 慢性化且修复后复发 ≥ `abandon_repair_failures` → `Abandon`；
    /// 4. 慢性化（其余）→ `Diagnose`；
    /// 5. 瞬时 + 窗口内次数 ≥ `habituation_threshold` → `Habituated` + `Retry`；
    /// 6. 瞬时未达习惯化阈值 → `Normal` + `Retry`；
    /// 7. 系统性 → `Sensitized` + `Diagnose`（系统性缺陷不可习惯化）。
    pub fn assess(&mut self, signal: &FailureSignal) -> PainAssessment {
        if signal.class == FailureClass::External {
            return external_assessment(&signal.cause_key);
        }

        let idx = self.record_signal(signal);
        let cfg = &self.config;
        let group = &self.groups[idx];
        let window_hits = group.window_hits();
        let chronic = window_hits >= u64::from(cfg.chronic_threshold)
            || group.sessions >= cfg.chronic_sessions;
        let abandon = chronic && group.repair_failures >= cfg.abandon_repair_failures;

        let (habituation, recommended, rationale) = match signal.class {
            FailureClass::External => {
                // 入口已早退；此臂仅为穷尽性，语义与入口保持一致（不 panic）。
                let a = external_assessment(&signal.cause_key);
                (a.habituation, a.recommended, a.rationale)
            }
            FailureClass::Systematic => {
                if abandon {
                    (
                        Habituation::Sensitized,
                        Recommendation::Abandon,
                        format!(
                            "系统性缺陷在 {} 次修复后仍复发（窗口 {} 次）: 隔离该路径并保留瘢痕",
                            group.repair_failures, window_hits
                        ),
                    )
                } else if chronic {
                    (
                        Habituation::Sensitized,
                        Recommendation::Diagnose,
                        format!(
                            "系统性缺陷慢性化（窗口 {} 次 ≥ {} 或跨 {} 会话）: 停重试，诊断根因",
                            window_hits, cfg.chronic_threshold, group.sessions
                        ),
                    )
                } else {
                    (
                        Habituation::Sensitized,
                        Recommendation::Diagnose,
                        "系统性缺陷不可习惯化: 直接诊断根因，重试只会放大成本".to_string(),
                    )
                }
            }
            FailureClass::Transient => {
                if abandon {
                    (
                        Habituation::Sensitized,
                        Recommendation::Abandon,
                        format!(
                            "瞬时失败在 {} 次修复后仍复发（窗口 {} 次）: 视为不可自愈，隔离该路径",
                            group.repair_failures, window_hits
                        ),
                    )
                } else if chronic {
                    (
                        Habituation::Sensitized,
                        Recommendation::Diagnose,
                        format!(
                            "慢性化压过习惯化（窗口 {} 次 ≥ {} 或跨 {} 会话）: 停重试，诊断根因",
                            window_hits, cfg.chronic_threshold, group.sessions
                        ),
                    )
                } else if window_hits >= u64::from(cfg.habituation_threshold) {
                    (
                        Habituation::Habituated,
                        Recommendation::Retry,
                        format!(
                            "同类瞬时失败窗口内 {} 次 ≥ 习惯化阈值 {}: 反应降级，不升级、继续重试",
                            window_hits, cfg.habituation_threshold
                        ),
                    )
                } else {
                    (
                        Habituation::Normal,
                        Recommendation::Retry,
                        format!(
                            "瞬时失败窗口内 {} 次 < 习惯化阈值 {}: 正常重试，未敏化",
                            window_hits, cfg.habituation_threshold
                        ),
                    )
                }
            }
        };

        PainAssessment {
            class: signal.class,
            habituation,
            chronic,
            recommended,
            rationale,
        }
    }

    /// 记录一次成功修复 → 脱敏该组: 清空窗口样本、敏化权重回落、开始新会话.
    ///
    /// 返回 `true` 表示账本里有该组且已脱敏；未知组返回 `false`（外部类从不入账）。
    /// 注意: `hits`（历史次数）与 `repair_failures`（修复后复发）**不**清零——
    /// 脱敏降的是急性反应，不是删掉「这里受过伤」的记忆。
    pub fn record_repair(&mut self, capability: &str, cause_key: &str, now_ms: u64) -> bool {
        let Some(group) = self
            .groups
            .iter_mut()
            .find(|g| g.capability == capability && g.cause_key == cause_key)
        else {
            return false;
        };
        group.samples.clear();
        group.sessions = 0;
        group.sensitivity = 1;
        group.last_repair_ms = now_ms;
        group.pending_repair = true;
        true
    }

    /// 写入一次信号（外部类已在调用方排除），返回组下标.
    fn record_signal(&mut self, signal: &FailureSignal) -> usize {
        let idx = match self
            .groups
            .iter()
            .position(|g| g.capability == signal.capability && g.cause_key == signal.cause_key)
        {
            Some(i) => i,
            None => {
                self.evict_if_full();
                self.groups.push(PainGroup::new(signal));
                self.groups.len() - 1
            }
        };

        let cfg = &self.config;
        let group = &mut self.groups[idx];
        let mut new_episode = false;
        if group.pending_repair {
            // 修复后第一次失败: 记一次「修复没治好」，并开始新会话。
            group.repair_failures = group.repair_failures.saturating_add(1);
            group.pending_repair = false;
            group.sessions = 1;
            new_episode = true;
        }
        if !new_episode
            && (group.hits == 0 || signal.at_ms.saturating_sub(group.last_ms) > cfg.session_gap_ms)
        {
            group.sessions = group.sessions.saturating_add(1);
        }

        group.hits = group.hits.saturating_add(1);
        group.last_ms = group.last_ms.max(signal.at_ms);
        group.class = signal.class;
        group.sensitivity = group.sensitivity.saturating_add(1).min(SENSITIVITY_MAX);
        group.samples.push(signal.at_ms);

        // 窗口裁剪: 边界含等号（差值 == 窗口仍计入）; 乱序注入按到达值判定，确定性。
        let window = cfg.chronic_window_ms;
        group
            .samples
            .retain(|t| signal.at_ms.saturating_sub(*t) <= window);
        while group.samples.len() > cfg.max_samples {
            group.samples.remove(0);
        }
        idx
    }

    /// 容量满时淘汰「最久未触达 → capability → cause_key」最小的一组。
    fn evict_if_full(&mut self) {
        let capacity = self.config.capacity;
        while self.groups.len() >= capacity {
            let mut victim = 0usize;
            for i in 1..self.groups.len() {
                if is_weaker(&self.groups[i], &self.groups[victim]) {
                    victim = i;
                }
            }
            self.groups.remove(victim);
        }
    }
}

/// 外部故障的固定判定: 只退避，不入账。
fn external_assessment(cause_key: &str) -> PainAssessment {
    PainAssessment {
        class: FailureClass::External,
        habituation: Habituation::Normal,
        chronic: false,
        recommended: Recommendation::Backoff,
        rationale: format!(
            "外部故障 cause_key={cause_key} 属上游/环境，不是可学可修的痛: 退避等待，不写账本、不计慢性"
        ),
    }
}

/// 淘汰序: `last_ms` 越小越弱；同刻按 capability、cause_key 字典序。
fn is_weaker(a: &PainGroup, b: &PainGroup) -> bool {
    (a.last_ms, &a.capability, &a.cause_key) < (b.last_ms, &b.capability, &b.cause_key)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn signal(capability: &str, cause_key: &str, class: FailureClass, at_ms: u64) -> FailureSignal {
        FailureSignal {
            capability: capability.to_string(),
            cause_key: cause_key.to_string(),
            class,
            at_ms,
        }
    }

    fn transient(at_ms: u64) -> FailureSignal {
        signal("tool.web_fetch", "timeout", FailureClass::Transient, at_ms)
    }

    #[test]
    fn classifies_three_failure_classes() {
        let mut ledger = PainLedger::default();

        let t = ledger.assess(&transient(0));
        assert_eq!(t.class, FailureClass::Transient);
        assert_eq!(t.recommended, Recommendation::Retry);
        assert_eq!(t.habituation, Habituation::Normal);
        assert!(!t.chronic);

        let s = ledger.assess(&signal(
            "provider.chat",
            "schema_mismatch",
            FailureClass::Systematic,
            1,
        ));
        assert_eq!(s.class, FailureClass::Systematic);
        assert_eq!(s.recommended, Recommendation::Diagnose);
        assert_eq!(
            s.habituation,
            Habituation::Sensitized,
            "系统性缺陷不可习惯化"
        );
        assert!(!s.chronic);

        let e = ledger.assess(&signal("tool.web_fetch", "dns", FailureClass::External, 2));
        assert_eq!(e.recommended, Recommendation::Backoff);
        assert!(!e.chronic);
        assert!(
            ledger.get("tool.web_fetch", "dns").is_none(),
            "外部类不入账"
        );
    }

    #[test]
    fn transient_repeats_habituate_without_escalation() {
        let mut ledger = PainLedger::default();
        let a = ledger.assess(&transient(0));
        let b = ledger.assess(&transient(100));
        let c = ledger.assess(&transient(200));
        assert_eq!(a.habituation, Habituation::Normal);
        assert_eq!(b.habituation, Habituation::Normal);
        assert_eq!(
            c.habituation,
            Habituation::Habituated,
            "第 3 次达习惯化阈值"
        );
        for x in [&a, &b, &c] {
            assert_eq!(
                x.recommended,
                Recommendation::Retry,
                "习惯化方向是降敏感，不升级"
            );
            assert!(!x.chronic);
        }
    }

    #[test]
    fn chronic_window_threshold_both_sides() {
        let mut ledger = PainLedger::default();
        for at in [0u64, 10, 20] {
            let a = ledger.assess(&transient(at));
            assert!(!a.chronic, "3 次 < 慢性阈值 4 不应慢性化");
        }
        let a = ledger.assess(&transient(30));
        assert!(a.chronic, "第 4 次达慢性阈值");
        assert_eq!(a.habituation, Habituation::Sensitized, "慢性化压过习惯化");
        assert_eq!(a.recommended, Recommendation::Diagnose);
        assert_eq!(
            ledger.get("tool.web_fetch", "timeout").map(|g| g.hits),
            Some(4)
        );
    }

    #[test]
    fn chronic_window_boundary_is_inclusive() {
        // 临界值内侧: 差值恰好 == 窗口 → 计入（第 4 次达慢性阈值）。
        let mut inside = PainLedger::default();
        for at in [0u64, 1_000, 2_000] {
            inside.assess(&transient(at));
        }
        let fourth = inside.assess(&transient(600_000));
        assert!(fourth.chronic, "差值 600_000 == 窗口，0ms 样本应计入");
        let group = inside.get("tool.web_fetch", "timeout").expect("组应存在");
        assert_eq!(group.window_hits(), 4);

        // 临界值外侧: 差值 == 窗口 + 1 → 剔除（第 4 次仍只有 3 个窗口内样本）。
        let mut outside = PainLedger::default();
        for at in [0u64, 1_000, 2_000] {
            outside.assess(&transient(at));
        }
        let fourth = outside.assess(&transient(600_001));
        assert!(!fourth.chronic, "剔除 0ms 样本后 3 次 < 慢性阈值 4");
        let group = outside.get("tool.web_fetch", "timeout").expect("组应存在");
        assert_eq!(group.window_hits(), 3, "0ms 样本应被窗口剔除");
    }

    #[test]
    fn cross_session_recurrence_becomes_chronic() {
        let mut ledger = PainLedger::default();
        // 每 30 分钟 + 1ms 一次 → 各自是新会话，窗口内各只有 1 次。
        let a = ledger.assess(&transient(0));
        let b = ledger.assess(&transient(1_800_001));
        let c = ledger.assess(&transient(3_600_002));
        assert!(!a.chronic && !b.chronic);
        assert!(c.chronic, "跨 3 个会话反复 → 慢性");
        assert_eq!(c.recommended, Recommendation::Diagnose);
        let group = ledger.get("tool.web_fetch", "timeout").expect("组应存在");
        assert_eq!(group.sessions, 3);
        assert_eq!(group.window_hits(), 1, "每个会话窗口内仅一次");
    }

    #[test]
    fn repair_desensitizes_acute_counts() {
        let mut ledger = PainLedger::default();
        for at in [0u64, 100, 200] {
            ledger.assess(&transient(at));
        }
        let before = ledger.get("tool.web_fetch", "timeout").expect("组应存在");
        assert_eq!(before.sensitivity, SENSITIVITY_MAX, "急性敏化权重已封顶");
        assert!(ledger.record_repair("tool.web_fetch", "timeout", 300));

        let after = ledger
            .get("tool.web_fetch", "timeout")
            .expect("修复后组仍在（不删记忆）");
        assert!(after.samples.is_empty(), "窗口样本清空");
        assert_eq!(after.sensitivity, 1, "敏化权重回落");
        assert_eq!(after.sessions, 0);

        let next = ledger.assess(&transient(400));
        assert_eq!(next.habituation, Habituation::Normal, "修复后重新脱敏");
        assert!(!next.chronic);
        assert_eq!(next.recommended, Recommendation::Retry);
        let after = ledger.get("tool.web_fetch", "timeout").expect("组应存在");
        assert_eq!(after.sessions, 1);
        assert_eq!(after.repair_failures, 1, "修复后复发被记账");
        assert_eq!(after.hits, 4, "历史次数不因脱敏清零");
    }

    #[test]
    fn repair_then_repeated_relapse_escalates_to_abandon() {
        let mut ledger = PainLedger::default();
        for at in [0u64, 10, 20, 30] {
            ledger.assess(&transient(at));
        }
        assert_eq!(
            ledger.assess(&transient(40)).recommended,
            Recommendation::Diagnose,
            "首个慢性 episode 只诊断"
        );
        assert!(ledger.record_repair("tool.web_fetch", "timeout", 50));
        for at in [60u64, 70, 80, 90] {
            ledger.assess(&transient(at));
        }
        assert_eq!(
            ledger
                .get("tool.web_fetch", "timeout")
                .map(|g| g.repair_failures),
            Some(1)
        );
        assert_eq!(
            ledger.assess(&transient(100)).recommended,
            Recommendation::Diagnose,
            "一次修复后复发仍给诊断"
        );
        assert!(ledger.record_repair("tool.web_fetch", "timeout", 110));
        for at in [120u64, 130, 140] {
            ledger.assess(&transient(at));
        }
        let last = ledger.assess(&transient(150));
        assert_eq!(
            ledger
                .get("tool.web_fetch", "timeout")
                .map(|g| g.repair_failures),
            Some(2)
        );
        assert!(last.chronic);
        assert_eq!(
            last.recommended,
            Recommendation::Abandon,
            "两次修复后仍复发 → 放弃"
        );
    }

    #[test]
    fn capacity_eviction_is_deterministic() {
        let mut ledger = PainLedger::new(2);
        ledger.assess(&signal("cap.b", "k", FailureClass::Transient, 100));
        ledger.assess(&signal("cap.a", "k", FailureClass::Transient, 100));
        // 同刻并列 → 字典序最小的 cap.a 被淘汰。
        ledger.assess(&signal("cap.c", "k", FailureClass::Transient, 200));
        assert_eq!(ledger.len(), 2);
        assert!(ledger.get("cap.a", "k").is_none(), "同刻按字典序淘汰");
        assert!(ledger.get("cap.b", "k").is_some());
        assert!(ledger.get("cap.c", "k").is_some());

        // 再进一组 → 最久未触达的 cap.b（100ms）被淘汰。
        ledger.assess(&signal("cap.d", "k", FailureClass::Transient, 300));
        assert_eq!(ledger.len(), 2);
        assert!(ledger.get("cap.b", "k").is_none());
        assert!(ledger.get("cap.d", "k").is_some());
    }

    #[test]
    fn external_signals_never_enter_ledger_or_become_chronic() {
        let mut ledger = PainLedger::default();
        for at in 0..100u64 {
            let a = ledger.assess(&signal(
                "tool.web_fetch",
                "upstream_5xx",
                FailureClass::External,
                at,
            ));
            assert_eq!(a.recommended, Recommendation::Backoff);
            assert!(!a.chronic);
            assert_eq!(a.habituation, Habituation::Normal);
        }
        assert!(ledger.is_empty(), "外部类从不写账本");
        assert!(!ledger.record_repair("tool.web_fetch", "upstream_5xx", 100));
    }

    #[test]
    fn assessments_are_deterministic() {
        let sequence: Vec<FailureSignal> = (0..12u64)
            .map(|i| match i % 4 {
                0 => transient(i * 11),
                1 => signal(
                    "provider.chat",
                    "schema_mismatch",
                    FailureClass::Systematic,
                    i * 11,
                ),
                2 => signal("tool.web_fetch", "dns", FailureClass::External, i * 11),
                _ => signal("tool.web_fetch", "timeout", FailureClass::Transient, i * 11),
            })
            .collect();
        let mut a = PainLedger::default();
        let mut b = PainLedger::default();
        let ra: Vec<PainAssessment> = sequence.iter().map(|s| a.assess(s)).collect();
        let rb: Vec<PainAssessment> = sequence.iter().map(|s| b.assess(s)).collect();
        assert_eq!(ra, rb, "同序列 → 同评估");
        assert_eq!(a, b, "同序列 → 同账本");
    }

    #[test]
    fn degenerate_config_and_inputs_do_not_panic() {
        let mut ledger = PainLedger::with_config(PainConfig {
            capacity: 0,
            chronic_threshold: 0,
            chronic_sessions: 0,
            habituation_threshold: 0,
            abandon_repair_failures: 0,
            max_samples: 0,
            chronic_window_ms: 0,
            session_gap_ms: 0,
        });
        assert_eq!(ledger.capacity(), 1, "容量 0 抬到 1");

        // 乱序 / 超大时间戳 / 空标识都不得 panic。
        for (i, at) in [u64::MAX, 0, 5, 3, u64::MAX / 2].iter().enumerate() {
            let s = signal(
                if i % 2 == 0 { "" } else { "cap" },
                "",
                FailureClass::Transient,
                *at,
            );
            let a = ledger.assess(&s);
            assert!(matches!(
                a.recommended,
                Recommendation::Retry | Recommendation::Diagnose | Recommendation::Abandon
            ));
        }
        assert_eq!(ledger.len(), 1, "空标识被视为同一组");
    }

    #[test]
    fn repair_unknown_group_is_false_and_ledger_unchanged() {
        let mut ledger = PainLedger::default();
        assert!(!ledger.record_repair("nope", "nope", 0));
        assert!(ledger.is_empty());
    }
}
