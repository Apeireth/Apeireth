//! `canonical::bio_pain` — 痛觉学习 (SG-16) 在失败处理链上的观测接线 (W3.3).
//!
//! # 它是什么
//! 运行时级的**只读观测 + 记账**面: 每次工具调用完成时, 把失败压成一个稳定的
//! [`FailureSignal`] 交给 `apeireth_orchestration::nociception::PainLedger`,
//! 成功则对同一能力的已记账原因执行 `record_repair`。账本挂在 [`Runtime`] 级
//! (跨会话), 因此"跨会话反复"由账本按 `session_gap_ms` 的时间间隙近似, 与
//! `nociception` 内核口径一致。
//!
//! # 接线点 (唯一失败入口)
//! `execute::record_call_completed` 是三个调用点的共同收口:
//! 串行派发 (`execute.rs` 的 `dispatch_one_tool`)、并行窗口正常提交、并行窗口
//! 取消排水。本模块的评估在该函数**原有事件/轨迹落账之后**执行 —— 事实先落,
//! 观测后追加。
//!
//! # 修复键的口径
//! 成功结果不携带"修好了哪一个失败原因", 所以成功分支不能只指认一个 cause_key:
//! 它对**该能力**在账本里的每个 cause_key 各执行一次 `record_repair` (脱敏)。
//! 语义是"这个能力现在可用"这一条证据, 而不是对某条历史失败的精确归因。
//!
//! # 只观测, 不决策 (本批硬边界)
//! 评估结果 (`recommended` / `habituation` / `chronic`) **只记录**:
//! - 不改写 [`ToolResult`]、不改写 `ToolFailed` 的错误文本;
//! - 不改变重试、退避、熔断或错误处理决策; 不引入第 4 套 RetryPolicy;
//! - 不伪造任何 session 事件或 trace 事件 —— 观测面是 [`BioPainSnapshot`]
//!   (运行时只读访问器), 不是事件流。
//!
//! # 失败类别的近似口径 (不冒充精确分类)
//! protocol 层唯一的失败判别位是 `ToolOutcome::Error::retryable` (没有独立的
//! `permanent` 字段; `ToolResult::permanent_error` 的语义就是 `retryable = false`)。
//! 因此本层做的是**近似映射**:
//! - `retryable == true` → [`FailureClass::Transient`];
//! - `retryable == false` → [`FailureClass::Systematic`];
//! - 类别未知 (`None`) → [`FailureClass::External`] (当前工具面不产生; 保留为
//!   文档化的兜底, 测试覆盖)。
//!
//! 这条近似会把"上游 503 但被包装成 permanent"读成系统性, 也会把"本地超时但被
//! 标成 retryable"读成瞬时。它只是**证据, 不是判决**。
//!
//! # 原因键 (cause_key) 的隐私口径
//! [`failure_cause_key`] 只输出 `类别:16 位十六进制摘要`, 摘要输入是
//! `capability ␀ class ␀ rendered_failure` 的 FNV-1a 64 位散列。键里**不含**
//! 文件路径、URL、用户原文或错误原文; 原文只作为散列输入。FNV-1a 是稳定散列,
//! 不是密码学承诺 (短消息可被穷举), 这里的目标是"可归组、不泄漏", 不是保密。
//! 错误文本若含动态值 (时间戳 / 端口 / 请求 id), 同因会被拆成不同键 —— 这是
//! 可归组的近似, 不是语义分类; 需要精确归因时应在上游把错误归一后再交给本层。
//!
//! # 默认关
//! [`RuntimeConfig::bio_pain_enabled`](super::runtime::RuntimeConfig) 默认
//! `false`: 关闭时 `record_call_completed` 走原路径, 不取锁、不写账本, session
//! 事件与 trace 逐字节不变。开启只通过 builder 的
//! [`RuntimeBuilder::with_bio_pain`](super::runtime::RuntimeBuilder::with_bio_pain)。
//!
//! # 诚实边界
//! - 本批**只接** `nociception`; `recovery_protocol` (`RecoveryCase` /
//!   `ScarLedger`) 仍未接线。
//! - 无真实网络/文件工具的 E2E 手测; 全回合证据用脚本化 provider + 总是失败的
//!   工具 + 固定虚拟时钟 (`tests/bio_pain_wiring.rs`)。
//! - 评估不参与任何控制流, 因此"建议 Retry 但运行时仍按原逻辑处理"是预期行为。

use std::collections::VecDeque;

use apeireth_orchestration::nociception::{
    FailureClass, FailureSignal, Habituation, PainAssessment, PainGroup, PainLedger, Recommendation,
};
use apeireth_protocol::canonical::{ToolOutcome, ToolResult};
use serde::Serialize;

/// 进程内保留的最近评估条数上限 (环形, 防无界增长)。
pub const DEFAULT_BIO_PAIN_OBSERVATIONS: usize = 64;

const FNV1A_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
const FNV1A_PRIME: u64 = 0x0000_0100_0000_01b3;

/// FNV-1a 64 位散列 (稳定、无依赖、跨进程一致)。
fn fnv1a64(bytes: impl Iterator<Item = u8>) -> u64 {
    bytes.fold(FNV1A_OFFSET_BASIS, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(FNV1A_PRIME)
    })
}

/// 归一化一次失败的原因键: `类别:16 位十六进制`.
///
/// 输入只参与散列, 不进入输出: `capability`、`class` 与失败渲染文本之间用
/// `\0` 分隔, 避免拼接歧义。渲染文本只作为散列输入, 因此键里不含文件路径、
/// URL 或用户原文 (见模块文档的隐私口径)。
pub fn failure_cause_key(capability: &str, class: FailureClass, rendered_failure: &str) -> String {
    let hash = fnv1a64(
        capability
            .as_bytes()
            .iter()
            .copied()
            .chain(std::iter::once(0))
            .chain(class.as_str().as_bytes().iter().copied())
            .chain(std::iter::once(0))
            .chain(rendered_failure.as_bytes().iter().copied()),
    );
    format!("{}:{hash:016x}", class.as_str())
}

/// 把 `retryable` 三态近似成内核的三类失败.
///
/// `Some(true)` → 瞬时, `Some(false)` → 系统性 (即 `permanent_error` 的语义),
/// `None` (未知) → 外部。这是文档化的近似, 不是精确分类。
pub fn classify_retryability(retryable: Option<bool>) -> FailureClass {
    match retryable {
        Some(true) => FailureClass::Transient,
        Some(false) => FailureClass::Systematic,
        None => FailureClass::External,
    }
}

/// 工具结果的失败类别; `None` 表示这次调用成功 (不构成失败信号)。
///
/// 近似口径见 [`classify_retryability`]: `ToolOutcome::Error { retryable }` 是
/// 协议层唯一的失败判别位。
pub fn failure_class_for_result(result: &ToolResult) -> Option<FailureClass> {
    match &result.outcome {
        ToolOutcome::Ok { .. } => None,
        ToolOutcome::Error { retryable, .. } => Some(classify_retryability(Some(*retryable))),
    }
}

/// 一次痛觉评估的只读观测记录 (纯数据, 可序列化).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BioPainObservation {
    /// 受影响能力标识 (如 `tool.web_fetch`)。
    pub capability: String,
    /// 归一化的原因键 (只含类别与摘要)。
    pub cause_key: String,
    /// 本次信号类别 (近似口径)。
    pub class: FailureClass,
    /// 事件时间 (毫秒, 运行时时钟注入)。
    pub at_ms: u64,
    /// 反应方向。
    pub habituation: Habituation,
    /// 是否已慢性化。
    pub chronic: bool,
    /// 处置建议 (只记录, 不执行)。
    pub recommended: Recommendation,
    /// 判定理由 (阈值与计数, 确定性可复核)。
    pub rationale: String,
}

/// 运行时痛觉观测面的只读快照.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BioPainSnapshot {
    /// 机制是否开启 (`with_bio_pain(true)`)。关闭时其余字段为空/零。
    pub enabled: bool,
    /// 累计评估次数。
    pub assessed: u64,
    /// 累计生效的修复次数 (成功调用命中已记账组的次数)。
    pub repaired: u64,
    /// 最近的评估记录 (最多 [`DEFAULT_BIO_PAIN_OBSERVATIONS`] 条, 旧→新)。
    pub observations: Vec<BioPainObservation>,
    /// 账本当前的全部组 (插入序; 只读副本)。
    pub groups: Vec<PainGroup>,
}

/// 运行时级痛觉账本 + 观测环.
///
/// [`Runtime`](super::runtime::Runtime) 以 `Mutex` 持有本结构; 只在开关开启时
/// 由 `record_call_completed` 写入。所有方法都不 panic、不做 I/O。
#[derive(Debug)]
pub(crate) struct BioPainState {
    ledger: PainLedger,
    observations: VecDeque<BioPainObservation>,
    observation_capacity: usize,
    assessed: u64,
    repaired: u64,
}

impl BioPainState {
    /// 空账本 (固定容量与默认判定参数来自 `nociception` 内核)。
    pub(crate) fn new() -> Self {
        Self {
            ledger: PainLedger::default(),
            observations: VecDeque::new(),
            observation_capacity: DEFAULT_BIO_PAIN_OBSERVATIONS,
            assessed: 0,
            repaired: 0,
        }
    }

    /// 记录一次失败: 归一键 → 评估 → 追加观测 (外部类由内核判定, 不写组)。
    pub(crate) fn observe_failure(
        &mut self,
        capability: &str,
        class: FailureClass,
        rendered_failure: &str,
        at_ms: u64,
    ) {
        let cause_key = failure_cause_key(capability, class, rendered_failure);
        let signal = FailureSignal {
            capability: capability.to_string(),
            cause_key,
            class,
            at_ms,
        };
        let assessment: PainAssessment = self.ledger.assess(&signal);
        self.assessed = self.assessed.saturating_add(1);
        self.observations.push_back(BioPainObservation {
            capability: signal.capability,
            cause_key: signal.cause_key,
            class: assessment.class,
            at_ms,
            habituation: assessment.habituation,
            chronic: assessment.chronic,
            recommended: assessment.recommended,
            rationale: assessment.rationale,
        });
        while self.observations.len() > self.observation_capacity {
            self.observations.pop_front();
        }
    }

    /// 记录一次成功: 对该能力每个已记账原因执行 `record_repair` (脱敏).
    ///
    /// 成功只能证明"这个能力现在可用", 不能指认修好了哪一个原因, 所以修复
    /// 覆盖该能力的全部已记账组。返回实际生效的修复组数 (未知能力为 0)。
    pub(crate) fn observe_success(&mut self, capability: &str, at_ms: u64) -> u32 {
        let cause_keys: Vec<String> = self
            .ledger
            .groups()
            .iter()
            .filter(|group| group.capability == capability)
            .map(|group| group.cause_key.clone())
            .collect();
        let mut repaired = 0u32;
        for cause_key in cause_keys {
            if self.ledger.record_repair(capability, &cause_key, at_ms) {
                repaired = repaired.saturating_add(1);
            }
        }
        self.repaired = self.repaired.saturating_add(u64::from(repaired));
        repaired
    }

    /// 只读快照 (克隆账本与观测环, 调用方拿到后不受后续写入影响)。
    pub(crate) fn snapshot(&self, enabled: bool) -> BioPainSnapshot {
        BioPainSnapshot {
            enabled,
            assessed: self.assessed,
            repaired: self.repaired,
            observations: self.observations.iter().cloned().collect(),
            groups: self.ledger.groups().to_vec(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_orchestration::nociception::{
        DEFAULT_HABITUATION_THRESHOLD, DEFAULT_SESSION_GAP_MS,
    };

    fn retryable(message: &str) -> ToolResult {
        ToolResult::retryable_error("call_1", message)
    }

    fn permanent(message: &str) -> ToolResult {
        ToolResult::permanent_error("call_1", message)
    }

    #[test]
    fn cause_key_is_stable_and_distinct_per_input() {
        let base = failure_cause_key("tool.echo", FailureClass::Transient, "error: timeout");
        assert_eq!(
            base,
            failure_cause_key("tool.echo", FailureClass::Transient, "error: timeout"),
            "同因同键"
        );
        assert_ne!(
            base,
            failure_cause_key("tool.other", FailureClass::Transient, "error: timeout"),
            "不同能力异键"
        );
        assert_ne!(
            base,
            failure_cause_key("tool.echo", FailureClass::Systematic, "error: timeout"),
            "不同类别异键"
        );
        assert_ne!(
            base,
            failure_cause_key("tool.echo", FailureClass::Transient, "error: other"),
            "不同原因异键"
        );
        // 分组歧义防护: 分隔符参与散列。
        assert_ne!(
            failure_cause_key("ab", FailureClass::Transient, "c"),
            failure_cause_key("a", FailureClass::Transient, "bc"),
            "拼接不产生同一键"
        );
    }

    #[test]
    fn cause_key_carries_no_raw_detail() {
        let detail =
            "error: GET https://example.invalid/secret?token=abc failed reading C:\\Users\\me\\file.txt";
        let key = failure_cause_key("tool.web_fetch", FailureClass::Transient, detail);
        assert!(key.starts_with("transient:"), "{key}");
        let digest = key.strip_prefix("transient:").expect("类别前缀应存在");
        assert_eq!(digest.len(), 16, "{key}");
        assert!(
            digest.chars().all(|c| c.is_ascii_hexdigit()),
            "摘要只含十六进制: {key}"
        );
        for forbidden in [
            "https", "example", "token", "Users", "file.txt", "GET", "secret",
        ] {
            assert!(
                !key.contains(forbidden),
                "原因键不得含原文片段 {forbidden:?}: {key}"
            );
        }
    }

    #[test]
    fn failure_class_mapping_is_a_documented_approximation() {
        assert_eq!(
            failure_class_for_result(&retryable("timeout")),
            Some(FailureClass::Transient)
        );
        assert_eq!(
            failure_class_for_result(&permanent("bad argument")),
            Some(FailureClass::Systematic)
        );
        assert_eq!(
            failure_class_for_result(&ToolResult::ok("call_1", serde_json::json!(1))),
            None,
            "成功不构成失败信号"
        );
        assert_eq!(
            classify_retryability(None),
            FailureClass::External,
            "未知类别走文档化兜底"
        );
    }

    #[test]
    fn transient_repeats_habituate_before_chronic() {
        let mut state = BioPainState::new();
        for at_ms in 0..u64::from(DEFAULT_HABITUATION_THRESHOLD) {
            state.observe_failure(
                "tool.echo",
                FailureClass::Transient,
                "error: timeout",
                at_ms,
            );
        }
        let snapshot = state.snapshot(true);
        assert_eq!(snapshot.assessed, 3);
        assert_eq!(
            snapshot.observations[0].habituation,
            Habituation::Normal,
            "首次未达习惯化阈值"
        );
        assert_eq!(
            snapshot.observations[2].habituation,
            Habituation::Habituated,
            "第 3 次达习惯化阈值"
        );
        assert_eq!(
            snapshot.observations[2].recommended,
            Recommendation::Retry,
            "习惯化方向是降敏感, 不升级"
        );
        assert!(!snapshot.observations[2].chronic);

        // 第 4 次达慢性阈值: 建议转为诊断, 但账本只记录建议。
        state.observe_failure("tool.echo", FailureClass::Transient, "error: timeout", 3);
        let snapshot = state.snapshot(true);
        assert!(snapshot.observations[3].chronic);
        assert_eq!(
            snapshot.observations[3].habituation,
            Habituation::Sensitized
        );
        assert_eq!(
            snapshot.observations[3].recommended,
            Recommendation::Diagnose
        );
    }

    #[test]
    fn systematic_failure_is_diagnosed_on_the_first_signal() {
        let mut state = BioPainState::new();
        state.observe_failure(
            "tool.echo",
            FailureClass::Systematic,
            "error: bad argument",
            0,
        );
        let snapshot = state.snapshot(true);
        let observation = &snapshot.observations[0];
        assert_eq!(observation.class, FailureClass::Systematic);
        assert_eq!(observation.habituation, Habituation::Sensitized);
        assert_eq!(observation.recommended, Recommendation::Diagnose);
        assert!(!observation.chronic);
    }

    #[test]
    fn success_repairs_every_recorded_cause_of_that_capability_only() {
        let mut state = BioPainState::new();
        state.observe_failure("tool.echo", FailureClass::Transient, "error: timeout", 10);
        state.observe_failure(
            "tool.echo",
            FailureClass::Systematic,
            "error: bad argument",
            20,
        );
        state.observe_failure("tool.other", FailureClass::Transient, "error: timeout", 30);

        assert_eq!(state.observe_success("tool.echo", 40), 2);
        let snapshot = state.snapshot(true);
        assert_eq!(snapshot.repaired, 2);
        let echo_groups: Vec<&PainGroup> = snapshot
            .groups
            .iter()
            .filter(|group| group.capability == "tool.echo")
            .collect();
        assert_eq!(echo_groups.len(), 2);
        for group in echo_groups {
            assert_eq!(group.last_repair_ms, 40, "修复时间已记录");
            assert_eq!(group.sessions, 0, "修复开始新会话");
            assert_eq!(group.sensitivity, 1, "敏化权重回落");
            assert_eq!(group.hits, 1, "历史次数不因脱敏清零");
        }
        let other = snapshot
            .groups
            .iter()
            .find(|group| group.capability == "tool.other")
            .expect("未修复的能力组仍在");
        assert_eq!(other.last_repair_ms, 0, "其它能力不被误修");

        // 修复后同因再失败: 记一次"修复没治好", 但重新从 Normal 开始。
        state.observe_failure("tool.echo", FailureClass::Transient, "error: timeout", 50);
        let snapshot = state.snapshot(true);
        let last = snapshot.observations.last().expect("有观测");
        assert_eq!(last.habituation, Habituation::Normal);
        assert!(!last.chronic);
        let repaired_group = snapshot
            .groups
            .iter()
            .find(|group| group.capability == "tool.echo" && group.cause_key == last.cause_key)
            .expect("组仍在");
        assert_eq!(repaired_group.repair_failures, 1);
    }

    #[test]
    fn cross_session_recurrence_is_observed_as_chronic() {
        let mut state = BioPainState::new();
        let gap = DEFAULT_SESSION_GAP_MS + 1;
        state.observe_failure("tool.echo", FailureClass::Transient, "error: timeout", 0);
        state.observe_failure("tool.echo", FailureClass::Transient, "error: timeout", gap);
        state.observe_failure(
            "tool.echo",
            FailureClass::Transient,
            "error: timeout",
            gap * 2,
        );
        let snapshot = state.snapshot(true);
        let last = snapshot.observations.last().expect("有观测");
        assert!(last.chronic, "跨 3 个会话反复 → 慢性");
        assert_eq!(last.recommended, Recommendation::Diagnose);
    }

    #[test]
    fn external_failure_is_observed_as_backoff_without_a_ledger_entry() {
        let mut state = BioPainState::new();
        state.observe_failure(
            "tool.web_fetch",
            FailureClass::External,
            "error: upstream",
            0,
        );
        let snapshot = state.snapshot(true);
        assert_eq!(snapshot.assessed, 1);
        assert_eq!(
            snapshot.observations[0].recommended,
            Recommendation::Backoff
        );
        assert!(!snapshot.observations[0].chronic);
        assert!(snapshot.groups.is_empty(), "外部类不入账");
        assert_eq!(state.observe_success("tool.web_fetch", 1), 0);
        assert_eq!(state.snapshot(true).repaired, 0);
    }

    #[test]
    fn observation_ring_is_bounded_and_keeps_the_newest() {
        let mut state = BioPainState::new();
        let total = DEFAULT_BIO_PAIN_OBSERVATIONS as u64 + 5;
        for at_ms in 0..total {
            state.observe_failure(
                "tool.echo",
                FailureClass::Transient,
                "error: timeout",
                at_ms,
            );
        }
        let snapshot = state.snapshot(true);
        assert_eq!(snapshot.assessed, total);
        assert_eq!(
            snapshot.observations.len(),
            DEFAULT_BIO_PAIN_OBSERVATIONS,
            "观测环有界"
        );
        assert_eq!(snapshot.observations[0].at_ms, 5, "最旧的 5 条被挤出");
        assert_eq!(
            snapshot
                .observations
                .last()
                .map(|observation| observation.at_ms),
            Some(total - 1)
        );
    }

    #[test]
    fn disabled_snapshot_is_empty_and_enabled_flag_is_reported() {
        let state = BioPainState::new();
        let snapshot = state.snapshot(false);
        assert!(!snapshot.enabled);
        assert_eq!(snapshot.assessed, 0);
        assert_eq!(snapshot.repaired, 0);
        assert!(snapshot.observations.is_empty());
        assert!(snapshot.groups.is_empty());
    }
}
