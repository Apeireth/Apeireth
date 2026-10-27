//! `organ::nociception` — 痛觉学习 (SG-16) + 再生案例记账 (SG-17) 的领域账本.
//!
//! # 它是什么
//! 领域级的**只读观测 + 记账**面: runtime 内核在每个工具调用完成时, 通过通用
//! [`ToolCompletionObserver`] 端口发出一次事实; 本模块把失败压成一个稳定的
//! [`FailureSignal`] 交给 `apeireth_orchestration::nociception::PainLedger`,
//! 成功则对同一能力的已记账原因执行 `record_repair`。账本挂在
//! [`NociceptionObserver`] 上 (跨会话), 因此"跨会话反复"由账本按
//! `session_gap_ms` 的时间间隙近似, 与 `nociception` 内核口径一致。
//!
//! 内核不持有痛觉状态、不识别 `bio_pain` 开关、不按领域名分支; 装配层决定是否
//! 把本 observer 装进 runtime。未安装 = 内核零接线、零取锁、零写入。
//!
//! 痛觉建议升级时, 同一失败链再把 `(capability, cause_key)` 记入一张**再生案例账**
//! (`apeireth_orchestration::recovery_protocol::RecoveryCase`, 四期状态机), 成功
//! 时把该能力的案例收尾到 `Resolved` 并移出活跃账。见下文"再生案例账"节。
//!
//! # 接线点 (唯一失败事实出口)
//! runtime 的 `record_call_completed` 是三个调用点的共同收口:
//! 串行派发 (`execute.rs` 的 `dispatch_one_tool`)、并行窗口正常提交、并行窗口
//! 取消排水。内核在该函数**原有事件/轨迹落账之后**向已安装的
//! [`ToolCompletionObserver`] 发出**一次**事实 —— 事实先落, 观测后追加。
//! 取消槽位与人工拒绝的伪结果不经过该收口, 因此不会产生评估 (与原语义一致)。
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
//!   ([`NociceptionObserver::snapshot`]), 不是事件流。
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
//! # 再生案例账 (SG-17, 同开关)
//! 开启时, [`BioPainState`] 在**同一把锁内**追加一张
//! `(capability, cause_key) → RecoveryCase` 的活跃案例账 (`BTreeMap`, 遍历确定)。
//! 只有痛觉建议升级到 `Backoff` / `Diagnose` / `Abandon` 才开案或推进; `Retry`
//! 不动账 (瞬时失败尚未升级)。开案相位由 `recommended` 决定 (纯函数
//! [`recovery_target_phase`]):
//!
//! | recommended | 开案相位 (沿合法前向事件补齐) | 语义类比 |
//! |---|---|---|
//! | `Retry` | 不开案 | 正常重试/习惯化: 伤口尚未形成 |
//! | `Backoff` | `Hemostasis` | 先止损/隔离, 等外部环境恢复 |
//! | `Diagnose` | `Inflammation` | 停重试, 诊断取证 |
//! | `Abandon` | `Proliferation` | 放弃自愈, 隔离该路径 (处置动作) |
//!
//! 开案用 [`RecoveryCase::new`] (从 `Hemostasis` 开始), 再沿内核合法前向事件补齐
//! 到目标相位: `Contained` → `Diagnosed` → `Repaired`。此后**同一
//! `(capability, cause_key)` 的每次升级失败只推进一步**, 不重复开新案例:
//! `Hemostasis --Contained--> Inflammation --Diagnosed--> Proliferation
//! --Repaired--> Remodeling`; 已在 `Remodeling` 的案例再失败投 `Relapse` 回到
//! `Inflammation` (复发, 重新走)。失败驱动**永远不会**把案例推到 `Resolved`:
//! 终态只能由成功 (验证通过) 收尾 —— 成功分支对该能力的每个活跃案例沿
//! `Contained`/`Diagnosed`/`Repaired`/`Verified` 补齐到 `Resolved`, 然后从活跃账
//! 移除并累加 `closed_cases` (不泄漏、不重复关闭)。
//!
//! 这些事件是**观测类比**, 不是"运行时真的执行了修复动作"的声明: 本层只记账,
//! 不执行回滚、不调 harness、不写瘢痕 (`ScarLedger::learn` 仍未接)。案例只暴露
//! 在 [`BioRecoverySnapshot`] (只读快照), 不产生 session/trace 事件。
//!
//! # 默认关
//! 装配层开关默认 `false`: 关闭时不构造本 observer, runtime 的
//! `record_call_completed` 走原路径, 不取锁、不写账本, session 事件与 trace
//! 逐字节不变。开启只通过装配层把 [`NociceptionObserver`] 装进 runtime,
//! 痛觉与再生案例共用这一个开关。
//!
//! # 诚实边界
//! - 本批**只接** `nociception` 与 `recovery_protocol` 的
//!   `RecoveryCase`/`transition` 状态机; `ScarLedger::learn` 仍未接线。
//! - 再生案例的四期事件是观测记账, 不是真实修复动作的执行证据; 不做回滚、
//!   不产生事件、不参与错误处理。
//! - 无真实网络/文件工具的 E2E 手测; 全回合证据用脚本化 provider + 脚本化
//!   失败工具 + 固定虚拟时钟 (`tests/bio_pain_wiring.rs`)。
//! - 评估不参与任何控制流, 因此"建议 Retry 但运行时仍按原逻辑处理"是预期行为。

use std::collections::{BTreeMap, VecDeque};
use std::sync::Mutex;

use apeireth_orchestration::nociception::{
    FailureClass, FailureSignal, Habituation, PainAssessment, PainGroup, PainLedger, Recommendation,
};
use apeireth_orchestration::recovery_protocol::{
    transition, HealingPhase, RecoveryCase, RecoveryEvent,
};
use apeireth_plugin::observation::{ToolCompletionFact, ToolCompletionObserver};
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

/// 痛觉建议 → 再生案例开案的**目标相位** (`None` = 不开案).
///
/// 这是文档化的观测映射 (表见模块文档"再生案例账"节), 不是"运行时真的做了
/// 该期动作"的声明:
/// - [`Recommendation::Retry`] → `None` (痛觉未升级, 不建案例);
/// - [`Recommendation::Backoff`] → `Hemostasis` (止损/隔离, 等环境恢复);
/// - [`Recommendation::Diagnose`] → `Inflammation` (停重试, 诊断取证);
/// - [`Recommendation::Abandon`] → `Proliferation` (放弃自愈, 落隔离动作)。
///
/// 开案的 `RecoveryCase` 从 `Hemostasis` 开始, 调用方沿内核合法前向事件
/// (`Contained`/`Diagnosed`/`Repaired`) 补齐到该相位; 后续同因升级失败每次只
/// 推进一期。
pub fn recovery_target_phase(recommended: Recommendation) -> Option<HealingPhase> {
    match recommended {
        Recommendation::Retry => None,
        Recommendation::Backoff => Some(HealingPhase::Hemostasis),
        Recommendation::Diagnose => Some(HealingPhase::Inflammation),
        Recommendation::Abandon => Some(HealingPhase::Proliferation),
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

/// 一个活跃再生案例的只读投影 (纯数据, 可序列化).
///
/// `evidence` 来自内核 [`RecoveryCase::evidence`] (有界, 只含事件名与注入毫秒),
/// 不含错误原文, 因此可安全暴露在观测面。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BioRecoveryCaseSnapshot {
    /// 受影响能力标识。
    pub capability: String,
    /// 归一化的原因键 (只含类别与摘要)。
    pub cause_key: String,
    /// 当前愈合相位。
    pub phase: HealingPhase,
    /// 开案时间 (毫秒, 运行时时钟注入)。
    pub started_ms: u64,
    /// 最近迁移时间 (内核保证单调不回退)。
    pub updated_ms: u64,
    /// 有界证据串 (旧→新; 只含 `事件@毫秒`)。
    pub evidence: Vec<String>,
}

/// 活跃案例的相位计数 (只统计活跃账里的案例; 已收尾案例只进 `closed_cases`)。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct BioRecoveryPhaseCounts {
    /// 止血期案例数。
    pub hemostasis: usize,
    /// 炎症期案例数。
    pub inflammation: usize,
    /// 增生期案例数。
    pub proliferation: usize,
    /// 重塑期案例数。
    pub remodeling: usize,
    /// 已解决期案例数 (按设计恒为 0: 收尾即移出活跃账)。
    pub resolved: usize,
}

/// 再生案例账的只读快照 (SG-17, 与痛觉共用开关).
///
/// 关闭时 [`BioRecoverySnapshot::default`] (全零/空)。只读访问器
/// [`NociceptionObserver::recovery_snapshot`] 与 [`BioPainSnapshot::recovery`]
/// 都返回它的副本, 不改变状态。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct BioRecoverySnapshot {
    /// 活跃案例数 (= `cases.len()`)。
    pub active_cases: usize,
    /// 累计收尾 (到达 `Resolved` 并移出活跃账) 的案例数。
    pub closed_cases: u64,
    /// 防御性计数: 内核拒绝的迁移次数。正常路径恒 0 (迁移事件由本模块按相位
    /// 构造, 不可能非法); 非 0 说明状态机假设被破坏, 观测面如实暴露。
    pub rejected_transitions: u64,
    /// 活跃案例的相位分布。
    pub phases: BioRecoveryPhaseCounts,
    /// 活跃案例 (按 `(capability, cause_key)` 字典序, 确定性)。
    pub cases: Vec<BioRecoveryCaseSnapshot>,
}

/// 痛觉观测面的只读快照.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BioPainSnapshot {
    /// 机制是否开启 (observer 是否已装入 runtime)。关闭时其余字段为空/零。
    pub enabled: bool,
    /// 累计评估次数。
    pub assessed: u64,
    /// 累计生效的修复次数 (成功调用命中已记账组的次数)。
    pub repaired: u64,
    /// 最近的评估记录 (最多 [`DEFAULT_BIO_PAIN_OBSERVATIONS`] 条, 旧→新)。
    pub observations: Vec<BioPainObservation>,
    /// 账本当前的全部组 (插入序; 只读副本)。
    pub groups: Vec<PainGroup>,
    /// 再生案例账快照 (SG-17, 同一开关; 关闭时全零/空)。
    pub recovery: BioRecoverySnapshot,
}

/// 再生案例账的键: `(capability, cause_key)`。用 `BTreeMap` 保证遍历顺序确定。
type RecoveryKey = (String, String);

/// 活跃再生案例账 (SG-17): 只由 [`BioPainState`] 在同一把锁内写入.
///
/// 一个 `(capability, cause_key)` 同时最多一个活跃案例; 案例到达 `Resolved`
/// 后移出活跃账并累加 `closed`。不持有任何外部资源, 不 panic。
#[derive(Debug, Default)]
struct BioRecoveryLedger {
    cases: BTreeMap<RecoveryKey, RecoveryCase>,
    closed: u64,
    rejected_transitions: u64,
}

/// 失败观测驱动案例推进时, 当前相位的合法事件.
///
/// 前向三期按内核迁移表推进; 已在 `Remodeling` 的案例再失败是**复发**
/// (`Relapse` → `Inflammation`), 终态仅在防御路径上出现 (正常不会留在活跃账)。
fn failure_advance_event(phase: HealingPhase) -> RecoveryEvent {
    match phase {
        HealingPhase::Hemostasis => RecoveryEvent::Contained,
        HealingPhase::Inflammation => RecoveryEvent::Diagnosed,
        HealingPhase::Proliferation => RecoveryEvent::Repaired,
        HealingPhase::Remodeling | HealingPhase::Resolved => RecoveryEvent::Relapse,
    }
}

/// 成功收尾 / 开案补齐时, 沿内核合法前向路径走一步 (终态无事件).
fn healing_forward_event(phase: HealingPhase) -> Option<RecoveryEvent> {
    match phase {
        HealingPhase::Hemostasis => Some(RecoveryEvent::Contained),
        HealingPhase::Inflammation => Some(RecoveryEvent::Diagnosed),
        HealingPhase::Proliferation => Some(RecoveryEvent::Repaired),
        HealingPhase::Remodeling => Some(RecoveryEvent::Verified),
        HealingPhase::Resolved => None,
    }
}

/// 案例标识 (确定性、不含原文): `recovery:{capability}:{cause_key}`。
fn recovery_case_id(capability: &str, cause_key: &str) -> String {
    format!("recovery:{capability}:{cause_key}")
}

/// 案例触发描述 (确定性、不含原文): 只含类别、能力与归一化原因键。
fn recovery_trigger(class: FailureClass, capability: &str, cause_key: &str) -> String {
    format!(
        "pain escalation [{}] capability={capability} cause={cause_key}",
        class.as_str()
    )
}

impl BioRecoveryLedger {
    /// 记一次升级失败: 无案例则按 [`recovery_target_phase`] 开案, 有案例则推进一期.
    ///
    /// `Retry` 直接返回 (不开案、不推进); 同因多次升级失败**不会**重复开案例。
    fn observe_failure(
        &mut self,
        capability: &str,
        cause_key: &str,
        class: FailureClass,
        recommended: Recommendation,
        at_ms: u64,
    ) {
        let Some(target) = recovery_target_phase(recommended) else {
            return;
        };
        let key: RecoveryKey = (capability.to_string(), cause_key.to_string());

        if let Some(existing) = self.cases.get(&key) {
            let event = failure_advance_event(existing.phase);
            match transition(existing, event, at_ms) {
                Ok(next) => {
                    self.cases.insert(key, next);
                }
                // 事件按当前相位构造, 正常不可达; 拒绝次数如实暴露而不是吞掉。
                Err(_) => {
                    self.rejected_transitions = self.rejected_transitions.saturating_add(1);
                }
            }
            return;
        }

        // 开案: 从 Hemostasis 起, 沿合法前向事件补齐到目标相位
        // (Backoff 0 步 / Diagnose 1 步 / Abandon 2 步)。
        let mut case = RecoveryCase::new(
            recovery_case_id(capability, cause_key),
            recovery_trigger(class, capability, cause_key),
            at_ms,
        );
        while case.phase < target {
            let Some(event) = healing_forward_event(case.phase) else {
                break;
            };
            match transition(&case, event, at_ms) {
                Ok(next) => case = next,
                Err(_) => {
                    self.rejected_transitions = self.rejected_transitions.saturating_add(1);
                    break;
                }
            }
        }
        self.cases.insert(key, case);
    }

    /// 记一次成功: 该能力的每个活跃案例沿合法前向路径补齐到 `Resolved` 后移出.
    ///
    /// 失败驱动永远不会把案例推到终态, 所以"成功 = 修复证据 + 验证通过"是唯一
    /// 的收尾入口。移出后累加 `closed`, 不泄漏、不重复关闭。
    fn observe_success(&mut self, capability: &str, at_ms: u64) {
        let keys: Vec<RecoveryKey> = self
            .cases
            .keys()
            .filter(|(case_capability, _)| case_capability == capability)
            .cloned()
            .collect();
        let mut closed = 0u64;
        for key in keys {
            let Some(existing) = self.cases.get(&key) else {
                continue;
            };
            let mut current = existing.clone();
            while !current.is_resolved() {
                let Some(event) = healing_forward_event(current.phase) else {
                    break;
                };
                match transition(&current, event, at_ms) {
                    Ok(next) => current = next,
                    Err(_) => {
                        self.rejected_transitions = self.rejected_transitions.saturating_add(1);
                        break;
                    }
                }
            }
            if current.is_resolved() {
                self.cases.remove(&key);
                closed = closed.saturating_add(1);
            }
        }
        self.closed = self.closed.saturating_add(closed);
    }

    /// 只读快照 (按 `(capability, cause_key)` 字典序克隆活跃案例)。
    fn snapshot(&self) -> BioRecoverySnapshot {
        let mut phases = BioRecoveryPhaseCounts::default();
        let mut cases = Vec::with_capacity(self.cases.len());
        for ((capability, cause_key), case) in &self.cases {
            match case.phase {
                HealingPhase::Hemostasis => phases.hemostasis += 1,
                HealingPhase::Inflammation => phases.inflammation += 1,
                HealingPhase::Proliferation => phases.proliferation += 1,
                HealingPhase::Remodeling => phases.remodeling += 1,
                HealingPhase::Resolved => phases.resolved += 1,
            }
            cases.push(BioRecoveryCaseSnapshot {
                capability: capability.clone(),
                cause_key: cause_key.clone(),
                phase: case.phase,
                started_ms: case.started_ms,
                updated_ms: case.updated_ms,
                evidence: case.evidence.clone(),
            });
        }
        BioRecoverySnapshot {
            active_cases: self.cases.len(),
            closed_cases: self.closed,
            rejected_transitions: self.rejected_transitions,
            phases,
            cases,
        }
    }
}

/// 痛觉账本 + 观测环 + 再生案例账.
///
/// [`NociceptionObserver`] 以 `Mutex` 持有本结构; 只由 observer 收到内核完成
/// 事实时写入。所有方法都不 panic、不做 I/O。痛觉与再生案例共用**同一把锁**,
/// 因此不存在嵌套锁与锁顺序问题。
#[derive(Debug)]
pub(crate) struct BioPainState {
    ledger: PainLedger,
    observations: VecDeque<BioPainObservation>,
    observation_capacity: usize,
    assessed: u64,
    repaired: u64,
    recovery: BioRecoveryLedger,
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
            recovery: BioRecoveryLedger::default(),
        }
    }

    /// 测试专用: 以自定义痛觉判定参数构造状态 (用于构造 `Abandon` 边界序列)。
    #[cfg(test)]
    pub(crate) fn with_pain_config(
        config: apeireth_orchestration::nociception::PainConfig,
    ) -> Self {
        Self {
            ledger: PainLedger::with_config(config),
            ..Self::new()
        }
    }

    /// 记录一次失败: 归一键 → 评估 → 再生案例记账 → 追加观测.
    ///
    /// 外部类由内核判定 (不写痛觉组, 建议 `Backoff`), 但**仍会**在再生案例账里
    /// 开案/推进 (退避类比止血期)。
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
        // 再生案例 (SG-17): 评估之后、观测落环之前记账; 同一把锁, 无嵌套。
        self.recovery.observe_failure(
            &signal.capability,
            &signal.cause_key,
            class,
            assessment.recommended,
            at_ms,
        );
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

    /// 记录一次成功: 对该能力每个已记账原因执行 `record_repair` (脱敏), 并收尾
    /// 该能力在再生案例账里的全部活跃案例.
    ///
    /// 成功只能证明"这个能力现在可用", 不能指认修好了哪一个原因, 所以修复与
    /// 案例收尾都覆盖该能力的全部已记账原因。返回实际生效的痛觉修复组数
    /// (未知能力为 0; 再生案例另计 `closed_cases`)。
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
        self.recovery.observe_success(capability, at_ms);
        repaired
    }

    /// 只读快照 (克隆账本、观测环与再生案例账, 调用方拿到后不受后续写入影响)。
    pub(crate) fn snapshot(&self, enabled: bool) -> BioPainSnapshot {
        BioPainSnapshot {
            enabled,
            assessed: self.assessed,
            repaired: self.repaired,
            observations: self.observations.iter().cloned().collect(),
            groups: self.ledger.groups().to_vec(),
            recovery: self.recovery.snapshot(),
        }
    }

    /// 再生案例账的只读快照 (与 [`Self::snapshot`] 的 `recovery` 字段同源)。
    pub(crate) fn recovery_snapshot(&self) -> BioRecoverySnapshot {
        self.recovery.snapshot()
    }
}

impl Default for BioPainSnapshot {
    /// 机制未开启时的空快照 (与观测面既有口径一致)。
    fn default() -> Self {
        Self {
            enabled: false,
            assessed: 0,
            repaired: 0,
            observations: Vec::new(),
            groups: Vec::new(),
            recovery: BioRecoverySnapshot::default(),
        }
    }
}

/// 领域观察者: 把内核交出的通用工具完成事实记入痛觉账本.
///
/// 由装配层构造并装入 runtime; 内核只认识
/// [`ToolCompletionObserver`] 端口, 不认识痛觉、再生案例或本类型。
/// 账本由 `Mutex` 串行化, 因此串行/并行/取消排水的提交顺序不会造成重复记账:
/// 每个 `record_call_completed` 只发出一次事实。
#[derive(Debug)]
pub struct NociceptionObserver {
    state: Mutex<BioPainState>,
    enabled: bool,
}

impl NociceptionObserver {
    /// 开启状态的观察者 (装进 runtime 后生效)。
    #[must_use]
    pub fn new() -> Self {
        Self {
            state: Mutex::new(BioPainState::new()),
            enabled: true,
        }
    }

    /// 关闭状态的观察者: 快照恒为空, 事实被忽略。
    ///
    /// 供装配层在开关关闭时仍能提供"同口径空快照"读取口 (不安装进 runtime)。
    #[must_use]
    pub fn disabled() -> Self {
        Self {
            state: Mutex::new(BioPainState::new()),
            enabled: false,
        }
    }

    /// 观察面是否开启。
    #[must_use]
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// 痛觉观测面的只读快照 (克隆账本、观测环与再生案例账)。
    #[must_use]
    pub fn snapshot(&self) -> BioPainSnapshot {
        self.lock_state().snapshot(self.enabled)
    }

    /// 再生案例账的只读快照 (与 [`Self::snapshot`] 的 `recovery` 字段同源)。
    #[must_use]
    pub fn recovery_snapshot(&self) -> BioRecoverySnapshot {
        self.lock_state().recovery_snapshot()
    }

    /// poison 容错读锁: 持锁线程 panic 后账本数据并未损坏, 按 L 组口径恢复。
    fn lock_state(&self) -> std::sync::MutexGuard<'_, BioPainState> {
        self.state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

impl Default for NociceptionObserver {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolCompletionObserver for NociceptionObserver {
    fn observe(&self, fact: &ToolCompletionFact<'_>) {
        if !self.enabled {
            return;
        }
        let now_ms = u64::try_from(fact.at.epoch_millis()).unwrap_or(0);
        let capability = fact.capability.as_str();
        let rendered = fact.result.render();
        let class = failure_class_for_result(fact.result);
        let mut state = self.lock_state();
        match class {
            Some(class) => state.observe_failure(capability, class, &rendered, now_ms),
            // `None` 只表示这次调用成功 (失败信号不成立) → 记修复。
            None => {
                state.observe_success(capability, now_ms);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_core::kernel::{CapabilityId, RequestId, SessionId, Timestamp, TraceId};
    use apeireth_orchestration::nociception::{
        DEFAULT_HABITUATION_THRESHOLD, DEFAULT_SESSION_GAP_MS,
    };
    use apeireth_protocol::canonical::ToolCall;

    const FACT_NOW_MS: i64 = 1_700_000_000_000;

    fn completion_fact<'a>(
        call: &'a ToolCall,
        capability: &'a CapabilityId,
        result: &'a ToolResult,
    ) -> ToolCompletionFact<'a> {
        ToolCompletionFact {
            session: SessionId::new(),
            request: RequestId::new(),
            trace: TraceId::new(),
            round: 1,
            call,
            capability,
            result,
            at: Timestamp::from_epoch_millis(FACT_NOW_MS).expect("timestamp"),
        }
    }

    fn call() -> ToolCall {
        ToolCall {
            id: "call_1".to_string(),
            name: "echo".to_string(),
            arguments: serde_json::json!({}),
        }
    }

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
        assert_eq!(snapshot.recovery, BioRecoverySnapshot::default());
        assert_eq!(state.recovery_snapshot(), BioRecoverySnapshot::default());
    }

    // ------------------------------------------------------------------
    // 再生案例记账 (W3.4 SG-17): 开案/推进/收尾/计数/确定性/并发
    // ------------------------------------------------------------------

    use apeireth_orchestration::nociception::PainConfig;
    use std::sync::{mpsc, Arc, Mutex};

    fn recovery_cases(snapshot: &BioPainSnapshot) -> &[BioRecoveryCaseSnapshot] {
        &snapshot.recovery.cases
    }

    #[test]
    fn the_recommendation_to_phase_mapping_is_the_documented_one() {
        assert_eq!(recovery_target_phase(Recommendation::Retry), None);
        assert_eq!(
            recovery_target_phase(Recommendation::Backoff),
            Some(HealingPhase::Hemostasis)
        );
        assert_eq!(
            recovery_target_phase(Recommendation::Diagnose),
            Some(HealingPhase::Inflammation)
        );
        assert_eq!(
            recovery_target_phase(Recommendation::Abandon),
            Some(HealingPhase::Proliferation)
        );
    }

    /// Retry 不建案例: 首次/习惯化阶段的瞬时失败只走痛觉账。
    #[test]
    fn a_retry_verdict_never_opens_a_recovery_case() {
        let mut state = BioPainState::new();
        for at_ms in 0u64..3 {
            state.observe_failure(
                "tool.echo",
                FailureClass::Transient,
                "error: timeout",
                at_ms,
            );
        }
        let snapshot = state.snapshot(true);
        assert_eq!(snapshot.observations.len(), 3);
        assert!(snapshot
            .observations
            .iter()
            .all(|observation| observation.recommended == Recommendation::Retry));
        assert_eq!(snapshot.recovery.active_cases, 0);
        assert_eq!(snapshot.recovery.closed_cases, 0);
        assert!(recovery_cases(&snapshot).is_empty());
        assert_eq!(snapshot.recovery.phases, BioRecoveryPhaseCounts::default());
        assert_eq!(snapshot.recovery.rejected_transitions, 0);
    }

    /// 升级失败开案: 系统性 → 炎症期 (开案先走合法止血事件), 外部 → 止血期。
    #[test]
    fn an_escalated_failure_opens_one_case_at_the_mapped_phase() {
        let mut systematic = BioPainState::new();
        systematic.observe_failure(
            "tool.echo",
            FailureClass::Systematic,
            "error: bad argument",
            10,
        );
        let snapshot = systematic.snapshot(true);
        assert_eq!(snapshot.recovery.active_cases, 1);
        let case = &recovery_cases(&snapshot)[0];
        assert_eq!(case.capability, "tool.echo");
        assert_eq!(case.phase, HealingPhase::Inflammation);
        assert_eq!(case.started_ms, 10);
        assert_eq!(case.updated_ms, 10);
        assert_eq!(case.evidence.len(), 2, "opened + contained");
        assert_eq!(case.evidence[0], "opened@10ms");
        assert_eq!(case.evidence[1], "contained@10ms");
        assert_eq!(snapshot.recovery.phases.inflammation, 1);
        assert_eq!(snapshot.recovery.closed_cases, 0);
        assert_eq!(snapshot.recovery.rejected_transitions, 0);

        let mut external = BioPainState::new();
        external.observe_failure(
            "tool.web_fetch",
            FailureClass::External,
            "error: upstream",
            20,
        );
        let snapshot = external.snapshot(true);
        assert_eq!(snapshot.recovery.active_cases, 1);
        assert_eq!(recovery_cases(&snapshot)[0].phase, HealingPhase::Hemostasis);
        assert_eq!(snapshot.recovery.phases.hemostasis, 1);
        assert_eq!(recovery_cases(&snapshot)[0].evidence.len(), 1, "开案即止血");
        assert_eq!(recovery_cases(&snapshot)[0].evidence[0], "opened@20ms");
    }

    /// 同因连续升级失败: 每次只推进一步, 绝不重复开案例; 重塑期再失败按复发回炎症。
    #[test]
    fn repeated_failures_advance_one_phase_without_duplicating_the_case() {
        let mut state = BioPainState::new();
        let expected = [
            HealingPhase::Inflammation,
            HealingPhase::Proliferation,
            HealingPhase::Remodeling,
            HealingPhase::Inflammation, // Relapse
            HealingPhase::Proliferation,
            HealingPhase::Remodeling,
        ];
        for (step, expected_phase) in expected.iter().enumerate() {
            state.observe_failure(
                "tool.echo",
                FailureClass::Systematic,
                "error: bad argument",
                100 + step as u64,
            );
            let snapshot = state.snapshot(true);
            assert_eq!(
                snapshot.recovery.active_cases, 1,
                "不得重复开新案例 (step {step})"
            );
            assert_eq!(
                recovery_cases(&snapshot)[0].phase,
                *expected_phase,
                "step {step}"
            );
            assert_eq!(
                snapshot.recovery.closed_cases, 0,
                "失败驱动永不收尾 (step {step})"
            );
            assert_eq!(
                snapshot.recovery.rejected_transitions, 0,
                "每次迁移都合法 (step {step})"
            );
        }
        let snapshot = state.snapshot(true);
        assert_eq!(
            recovery_cases(&snapshot)[0].started_ms,
            100,
            "案例身份不因推进变化"
        );
        assert_eq!(
            recovery_cases(&snapshot)[0].evidence.len(),
            7,
            "opened + 6 次推进"
        );
    }

    /// 瞬时失败前 3 次 Retry 不开案, 慢性化 (第 4 次 Diagnose) 才开, 第 5 次推进。
    #[test]
    fn a_transient_streak_opens_a_case_only_after_escalation() {
        let mut state = BioPainState::new();
        for at_ms in 0u64..3 {
            state.observe_failure(
                "tool.echo",
                FailureClass::Transient,
                "error: timeout",
                at_ms,
            );
            assert_eq!(
                state.snapshot(true).recovery.active_cases,
                0,
                "前 3 次 Retry 不开案"
            );
        }
        state.observe_failure("tool.echo", FailureClass::Transient, "error: timeout", 3);
        let snapshot = state.snapshot(true);
        assert_eq!(
            snapshot
                .observations
                .last()
                .map(|observation| observation.recommended),
            Some(Recommendation::Diagnose),
            "第 4 次达慢性阈值"
        );
        assert_eq!(snapshot.recovery.active_cases, 1);
        assert_eq!(
            recovery_cases(&snapshot)[0].phase,
            HealingPhase::Inflammation
        );

        state.observe_failure("tool.echo", FailureClass::Transient, "error: timeout", 4);
        assert_eq!(
            recovery_cases(&state.snapshot(true))[0].phase,
            HealingPhase::Proliferation
        );
    }

    /// 不同 (capability, cause) 各自独立; 成功只收尾该能力, 其余案例不动。
    #[test]
    fn different_causes_and_capabilities_get_independent_cases() {
        let mut state = BioPainState::new();
        state.observe_failure(
            "tool.echo",
            FailureClass::Systematic,
            "error: bad argument",
            0,
        );
        state.observe_failure(
            "tool.echo",
            FailureClass::Systematic,
            "error: schema mismatch",
            1,
        );
        state.observe_failure(
            "tool.other",
            FailureClass::Systematic,
            "error: bad argument",
            2,
        );
        state.observe_failure("tool.echo", FailureClass::External, "error: upstream", 3);

        let snapshot = state.snapshot(true);
        assert_eq!(snapshot.recovery.active_cases, 4, "四个独立键");
        assert_eq!(snapshot.recovery.phases.hemostasis, 1);
        assert_eq!(snapshot.recovery.phases.inflammation, 3);
        let mut keys: Vec<(String, String)> = recovery_cases(&snapshot)
            .iter()
            .map(|case| (case.capability.clone(), case.cause_key.clone()))
            .collect();
        keys.sort();
        keys.dedup();
        assert_eq!(keys.len(), 4, "案例键互不相同");

        assert_eq!(state.observe_success("tool.echo", 10), 2, "痛觉修复两组");
        let snapshot = state.snapshot(true);
        assert_eq!(snapshot.recovery.active_cases, 1, "其余能力不被误收尾");
        assert_eq!(snapshot.recovery.closed_cases, 3);
        assert_eq!(recovery_cases(&snapshot)[0].capability, "tool.other");
        assert_eq!(
            recovery_cases(&snapshot)[0].phase,
            HealingPhase::Inflammation,
            "未收尾案例相位不变"
        );

        assert_eq!(state.observe_success("tool.other", 11), 1);
        let snapshot = state.snapshot(true);
        assert_eq!(snapshot.recovery.active_cases, 0);
        assert_eq!(snapshot.recovery.closed_cases, 4);
        assert_eq!(snapshot.recovery.phases, BioRecoveryPhaseCounts::default());
    }

    /// 成功从失败驱动的每个相位都能合法收尾到 Resolved 并移出活跃账 (不泄漏)。
    #[test]
    fn a_success_closes_a_case_from_every_failure_driven_phase() {
        for failures in 0u64..=3 {
            let mut state = BioPainState::new();
            if failures == 0 {
                state.observe_failure("tool.echo", FailureClass::External, "error: upstream", 0);
            } else {
                for at_ms in 0..failures {
                    state.observe_failure(
                        "tool.echo",
                        FailureClass::Systematic,
                        "error: bad argument",
                        at_ms,
                    );
                }
            }
            let before = state.snapshot(true);
            assert_eq!(before.recovery.active_cases, 1);
            assert!(matches!(
                recovery_cases(&before)[0].phase,
                HealingPhase::Hemostasis
                    | HealingPhase::Inflammation
                    | HealingPhase::Proliferation
                    | HealingPhase::Remodeling
            ));

            state.observe_success("tool.echo", 50);
            let after = state.snapshot(true);
            assert_eq!(after.recovery.active_cases, 0, "收尾后移出活跃账");
            assert_eq!(after.recovery.closed_cases, 1);
            assert_eq!(after.recovery.phases, BioRecoveryPhaseCounts::default());
            assert!(recovery_cases(&after).is_empty());

            // 重复成功不重复关闭。
            state.observe_success("tool.echo", 60);
            assert_eq!(state.snapshot(true).recovery.closed_cases, 1);
        }
    }

    /// Abandon 边界: 用自定义阈值构造"无既存案例 + 慢性且修复后复发"的首次升级,
    /// 验证开案沿合法路径落到增生期 (Contained + Diagnosed)。
    #[test]
    fn abandon_opens_at_proliferation_when_no_case_exists_yet() {
        let mut state = BioPainState::with_pain_config(PainConfig {
            chronic_threshold: 2,
            abandon_repair_failures: 1,
            ..PainConfig::default()
        });
        state.observe_failure("tool.echo", FailureClass::Transient, "error: timeout", 0);
        assert_eq!(
            state.snapshot(true).recovery.active_cases,
            0,
            "首次 Retry 不开案"
        );
        state.observe_success("tool.echo", 1);
        state.observe_failure("tool.echo", FailureClass::Transient, "error: timeout", 2);
        assert_eq!(
            state.snapshot(true).recovery.active_cases,
            0,
            "第二次仍 Retry"
        );

        state.observe_failure("tool.echo", FailureClass::Transient, "error: timeout", 3);
        let snapshot = state.snapshot(true);
        assert_eq!(
            snapshot
                .observations
                .last()
                .map(|observation| observation.recommended),
            Some(Recommendation::Abandon)
        );
        assert_eq!(snapshot.recovery.active_cases, 1);
        let case = &recovery_cases(&snapshot)[0];
        assert_eq!(case.phase, HealingPhase::Proliferation);
        assert_eq!(case.evidence.len(), 3, "opened + contained + diagnosed");
        assert_eq!(snapshot.recovery.phases.proliferation, 1);
        assert_eq!(snapshot.recovery.rejected_transitions, 0);
    }

    /// 快照计数: 活跃案例总数 = 各相位计数之和, 按 capability 字典序排列。
    #[test]
    fn the_recovery_snapshot_reports_exact_phase_counts() {
        let mut state = BioPainState::new();
        state.observe_failure("tool.a", FailureClass::External, "error: upstream", 0);
        state.observe_failure("tool.b", FailureClass::Systematic, "error: bad argument", 1);
        state.observe_failure("tool.c", FailureClass::Systematic, "error: bad argument", 2);
        // tool.b 推进两次 → 重塑期; tool.d 推进一次 → 增生期。
        state.observe_failure("tool.b", FailureClass::Systematic, "error: bad argument", 3);
        state.observe_failure("tool.b", FailureClass::Systematic, "error: bad argument", 4);
        state.observe_failure("tool.d", FailureClass::Systematic, "error: bad argument", 5);
        state.observe_failure("tool.d", FailureClass::Systematic, "error: bad argument", 6);

        let snapshot = state.snapshot(true);
        let counts = &snapshot.recovery.phases;
        assert_eq!(snapshot.recovery.active_cases, 4);
        assert_eq!(counts.hemostasis, 1);
        assert_eq!(counts.inflammation, 1);
        assert_eq!(counts.proliferation, 1);
        assert_eq!(counts.remodeling, 1);
        assert_eq!(counts.resolved, 0);
        assert_eq!(
            counts.hemostasis
                + counts.inflammation
                + counts.proliferation
                + counts.remodeling
                + counts.resolved,
            snapshot.recovery.active_cases,
            "相位计数之和 = 活跃案例数"
        );
        let capabilities: Vec<&str> = recovery_cases(&snapshot)
            .iter()
            .map(|case| case.capability.as_str())
            .collect();
        assert_eq!(
            capabilities,
            vec!["tool.a", "tool.b", "tool.c", "tool.d"],
            "按 (capability, cause_key) 字典序"
        );
    }

    /// 确定性: 同一失败/成功序列 + 同一注入时钟 → 同一份快照 (含案例证据)。
    #[test]
    fn the_recovery_snapshot_is_deterministic_for_the_same_sequence() {
        fn run() -> BioPainSnapshot {
            let mut state = BioPainState::new();
            state.observe_failure("tool.echo", FailureClass::Transient, "error: timeout", 0);
            state.observe_failure("tool.echo", FailureClass::Transient, "error: timeout", 1);
            state.observe_failure("tool.echo", FailureClass::Transient, "error: timeout", 2);
            state.observe_failure("tool.echo", FailureClass::Transient, "error: timeout", 3);
            state.observe_failure(
                "tool.echo",
                FailureClass::Systematic,
                "error: bad argument",
                4,
            );
            state.observe_failure(
                "tool.other",
                FailureClass::Systematic,
                "error: bad argument",
                5,
            );
            state.observe_success("tool.echo", 6);
            state.snapshot(true)
        }
        let first = run();
        let second = run();
        assert_eq!(first, second);
        assert_eq!(first.recovery.active_cases, 1, "只留 tool.other");
        assert_eq!(first.recovery.closed_cases, 2, "tool.echo 两个案例收尾");
        assert_eq!(first.recovery.rejected_transitions, 0);
    }

    /// 并发 smoke: 多线程交错失败/成功不卡死、不产生非法迁移。
    #[test]
    fn concurrent_recovery_updates_do_not_deadlock() {
        let state = Arc::new(Mutex::new(BioPainState::new()));
        let (done_tx, done_rx) = mpsc::channel::<()>();
        let mut handles = Vec::new();
        for worker in 0u64..4 {
            let state = Arc::clone(&state);
            let done_tx = done_tx.clone();
            handles.push(std::thread::spawn(move || {
                for step in 0u64..100 {
                    let at_ms = worker * 1_000 + step;
                    let mut guard = state
                        .lock()
                        .unwrap_or_else(|poisoned| poisoned.into_inner());
                    if step % 3 == 2 {
                        guard.observe_success("tool.echo", at_ms);
                    } else {
                        guard.observe_failure(
                            "tool.echo",
                            FailureClass::Systematic,
                            "error: bad argument",
                            at_ms,
                        );
                    }
                    drop(guard);
                }
                done_tx.send(()).expect("worker reports completion");
            }));
        }
        drop(done_tx);
        for _ in 0..4 {
            done_rx
                .recv_timeout(std::time::Duration::from_secs(10))
                .expect("并发更新在超时内完成 (无死锁)");
        }
        for handle in handles {
            handle.join().expect("worker joins");
        }
        let snapshot = state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .snapshot(true);
        assert_eq!(snapshot.assessed, 4 * 67, "每 worker 67 次失败评估");
        assert_eq!(
            snapshot.recovery.rejected_transitions, 0,
            "并发交错仍只产生合法迁移"
        );
        assert!(snapshot.recovery.active_cases <= 1, "同键最多一个活跃案例");
    }

    #[test]
    fn installed_observer_records_one_assessment_per_completion_fact() {
        let observer = NociceptionObserver::new();
        let call = call();
        let capability = CapabilityId::new("tool.echo").expect("capability id");
        let failure = retryable("upstream timeout");

        observer.observe(&completion_fact(&call, &capability, &failure));
        observer.observe(&completion_fact(&call, &capability, &failure));

        let snapshot = observer.snapshot();
        assert!(snapshot.enabled);
        assert_eq!(snapshot.assessed, 2, "每次完成事实恰一次评估");
        assert_eq!(snapshot.recovery.active_cases, 0, "Retry 不建案例");
        assert_eq!(
            observer.recovery_snapshot(),
            snapshot.recovery,
            "两个读取口同源"
        );
    }

    #[test]
    fn a_success_fact_repairs_without_being_an_assessment() {
        let observer = NociceptionObserver::new();
        let call = call();
        let capability = CapabilityId::new("tool.echo").expect("capability id");
        observer.observe(&completion_fact(
            &call,
            &capability,
            &retryable("upstream timeout"),
        ));

        observer.observe(&completion_fact(
            &call,
            &capability,
            &ToolResult::ok("call_1", serde_json::json!("ok")),
        ));

        let snapshot = observer.snapshot();
        assert_eq!(snapshot.assessed, 1);
        assert_eq!(snapshot.repaired, 1);
        assert_eq!(snapshot.groups[0].last_repair_ms, FACT_NOW_MS as u64);
    }

    #[test]
    fn a_disabled_observer_stays_empty_and_ignores_facts() {
        let observer = NociceptionObserver::disabled();
        let call = call();
        let capability = CapabilityId::new("tool.echo").expect("capability id");
        observer.observe(&completion_fact(
            &call,
            &capability,
            &permanent("bad argument"),
        ));

        assert!(!observer.is_enabled());
        assert_eq!(observer.snapshot(), BioPainSnapshot::default());
        assert_eq!(observer.recovery_snapshot(), BioRecoverySnapshot::default());
    }
}
