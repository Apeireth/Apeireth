//! 性质族 3 · 治理单调性 harness (对应"默认拒绝" / "RequireApproval 不得跳过审批")。
//!
//! 真实策略评估入口 (GovernancePipeline / PermissionPolicy::decision_for_capability /
//! ApprovalPolicyEngine::evaluate) 返回 canonical Decision, 但其编译依赖闭包
//! (apeireth_core 等) 无法零复制装进 mirror crate; 依任务口径降级为对
//! crates/foundation/governance 纯判定函数的命题证明:
//!   - intent.rs::TaskIntentEnvelopeV1 allows_* 判定 (策略轴单调性 / 默认拒绝);
//!   - risk.rs::check_no_degrade (风险标签单调, 降级必拒);
//!   - risk.rs::run_fail_closed (前置阶段失败短路, 不得跳到 apply)。
//! Decision 三态 (Allow/Deny/RequireApproval) 到这些纯核的映射边界见 README。
//!
//! 每个 harness 的注释一句话写明"证明什么、边界是什么"。

use super::intent::{
    CredentialPolicy, IntentClass, MutationPolicy, NetworkPolicy, OperationClass, ShellPolicy,
    TaskIntentEnvelopeV1,
};
use super::risk::{
    check_no_degrade, risk_rank, run_fail_closed, ApplyPhase, FailClosedPhase, NoDegradeCheck,
    PreparePhase, VerifyPhase,
};

const OPERATIONS: [OperationClass; 18] = [
    OperationClass::Read,
    OperationClass::Search,
    OperationClass::Enumerate,
    OperationClass::Create,
    OperationClass::Write,
    OperationClass::Modify,
    OperationClass::Delete,
    OperationClass::Execute,
    OperationClass::SpawnProcess,
    OperationClass::NetworkRead,
    OperationClass::NetworkSend,
    OperationClass::Publish,
    OperationClass::CredentialRead,
    OperationClass::CredentialWrite,
    OperationClass::MemoryRead,
    OperationClass::MemoryWrite,
    OperationClass::AdminChange,
    OperationClass::PersistenceChange,
];
/// Read/Search 之外的效果类操作 (默认拒绝命题的量词域)。
const EFFECT_OPERATIONS: [OperationClass; 16] = [
    OperationClass::Enumerate,
    OperationClass::Create,
    OperationClass::Write,
    OperationClass::Modify,
    OperationClass::Delete,
    OperationClass::Execute,
    OperationClass::SpawnProcess,
    OperationClass::NetworkRead,
    OperationClass::NetworkSend,
    OperationClass::Publish,
    OperationClass::CredentialRead,
    OperationClass::CredentialWrite,
    OperationClass::MemoryRead,
    OperationClass::MemoryWrite,
    OperationClass::AdminChange,
    OperationClass::PersistenceChange,
];

fn any_intent_class() -> IntentClass {
    match kani::any::<u8>() % 14 {
        0 => IntentClass::ReadOnlyInspection,
        1 => IntentClass::Research,
        2 => IntentClass::CodeAnalysis,
        3 => IntentClass::CodeModification,
        4 => IntentClass::FileManagement,
        5 => IntentClass::RepositoryMaintenance,
        6 => IntentClass::RepositoryPublish,
        7 => IntentClass::NetworkResearch,
        8 => IntentClass::DataTransformation,
        9 => IntentClass::MemoryOperation,
        10 => IntentClass::SystemAdministration,
        11 => IntentClass::ExplicitDestructiveMaintenance,
        12 => IntentClass::CredentialOperation,
        _ => IntentClass::Unknown,
    }
}

fn any_ops() -> Vec<OperationClass> {
    let n = kani::any::<usize>() % 3;
    (0..n)
        .map(|_| OPERATIONS[kani::any::<usize>() % 18])
        .collect()
}

/// 任意参数化的信封: 除策略轴外全部字段符号化 (调用方再固定被测策略轴)。
fn any_envelope() -> TaskIntentEnvelopeV1 {
    let mut e = TaskIntentEnvelopeV1::unknown("sess", "trace");
    e.intent_class = any_intent_class();
    e.explicitness = match kani::any::<u8>() % 3 {
        0 => super::intent::IntentExplicitness::Explicit,
        1 => super::intent::IntentExplicitness::Inferred,
        _ => super::intent::IntentExplicitness::Unknown,
    };
    e.confidence = kani::any::<f64>();
    e.requested_operations = any_ops();
    e.allowed_effects = any_ops();
    e.created_at_ms = kani::any::<i64>();
    e.credential_policy = match kani::any::<u8>() % 4 {
        0 => CredentialPolicy::Deny,
        1 => CredentialPolicy::ReadOnly,
        2 => CredentialPolicy::Allow,
        _ => CredentialPolicy::Unknown,
    };
    e.shell_policy = match kani::any::<u8>() % 4 {
        0 => ShellPolicy::Deny,
        1 => ShellPolicy::ReadOnly,
        2 => ShellPolicy::Allow,
        _ => ShellPolicy::Unknown,
    };
    e.mutation_policy = match kani::any::<u8>() % 4 {
        0 => MutationPolicy::Deny,
        1 => MutationPolicy::WorkspaceOnly,
        2 => MutationPolicy::Allow,
        _ => MutationPolicy::Unknown,
    };
    e.network_policy = match kani::any::<u8>() % 5 {
        0 => NetworkPolicy::Deny,
        1 => NetworkPolicy::LocalOnly,
        2 => NetworkPolicy::PublicRead,
        3 => NetworkPolicy::Allow,
        _ => NetworkPolicy::Unknown,
    };
    e
}

/// 证明: 任一策略轴被置 Deny 后, 无论其它任何参数 (意图类别/显式度/操作集/
/// 其余策略轴/时间戳) 如何变化, 对应放行判定恒为 false —— Deny 不可被
/// 参数变化翻转为 Allow (策略本身是唯一决定项)。
/// 边界: 量词域 = 全部非策略字段的符号化组合 (信封结构见 intent.rs),
/// unwind 32。
#[kani::proof]
#[kani::unwind(32)]
fn kani_governance_deny_axis_never_allows() {
    // 轴 1: network_policy = Deny, 其余任意。
    let mut e = any_envelope();
    e.network_policy = NetworkPolicy::Deny;
    assert!(!e.allows_network(), "Deny 网络轴不得放行 (任意参数变化)");

    // 轴 2: credential_policy = Deny, 其余任意。
    let mut e = any_envelope();
    e.credential_policy = CredentialPolicy::Deny;
    assert!(
        !e.allows_credentials(),
        "Deny 凭据轴不得放行 (任意参数变化)"
    );

    // 轴 3: shell_policy = Deny, 其余任意。
    let mut e = any_envelope();
    e.shell_policy = ShellPolicy::Deny;
    assert!(!e.allows_shell(), "Deny shell 轴不得放行 (任意参数变化)");

    // 轴 4: mutation_policy = Deny, 其余任意。
    let mut e = any_envelope();
    e.mutation_policy = MutationPolicy::Deny;
    assert!(!e.allows_mutation(), "Deny 变异轴不得放行 (任意参数变化)");
}

/// 证明: 默认拒绝 —— 信封未声明任何操作 (requested/allowed 均空) 时,
/// 无论意图类别与策略轴取何值, 任何效果类操作的放行判定恒为 false。
/// 边界: Read/Search 有显式白名单例外 (只读意图 + 空效果集), 不在本命题量词域;
/// allows_publish 另有 intent_class == RepositoryPublish 的设计内旁路,
/// 不经 allows_operation, 亦不在量词域 (见 README 观察记录); unwind 32。
#[kani::proof]
#[kani::unwind(32)]
fn kani_governance_default_deny_unrequested_ops() {
    let mut e = any_envelope();
    e.requested_operations = Vec::new();
    e.allowed_effects = Vec::new();
    for op in EFFECT_OPERATIONS {
        assert!(
            !e.allows_operation(op),
            "未声明的操作必须默认拒绝 (任意意图类别与策略轴)"
        );
    }
}

/// 证明: 风险标签单调 —— 对任意 (原始标签, 提案标签) 字节串, 严格降级的
/// 提案必被 check_no_degrade 触发 (不得评估为通过); Pass 蕴含未降级。
/// 边界: 标签为任意 ≤4 字节串 (对抗拼写变体是量词域的核心), 判定只依赖
/// 两个标签的 risk_rank, unwind 32。
#[kani::proof]
#[kani::unwind(32)]
fn kani_governance_no_degrade_never_weakens() {
    let original = String::from_utf8_lossy(&{
        let b: [u8; 4] = kani::any();
        b
    })
    .into_owned();
    let proposed = String::from_utf8_lossy(&{
        let b: [u8; 4] = kani::any();
        b
    })
    .into_owned();

    let result = check_no_degrade(&original, &proposed);
    if !proposed.is_empty() && risk_rank(&proposed) < risk_rank(&original) {
        assert!(
            matches!(result, NoDegradeCheck::Triggered { .. }),
            "降级提案必拒"
        );
    }
    if matches!(result, NoDegradeCheck::Pass) {
        assert!(
            proposed.is_empty() || risk_rank(&proposed) >= risk_rank(&original),
            "Pass 蕴含提案未降级"
        );
    }
    assert!((-1..=4).contains(&risk_rank(&original)), "秩值域 = [-1,4]");
    assert!((-1..=4).contains(&risk_rank(&proposed)), "秩值域 = [-1,4]");
}

/// 三阶段探针 (Cell 记录是否被调用), 用于观察 run_fail_closed 的短路语义。
struct Probe {
    fail: bool,
    ran: std::cell::Cell<bool>,
}

impl VerifyPhase for &mut Probe {
    type Error = String;
    fn verify(&mut self) -> Result<(), String> {
        self.ran.set(true);
        if self.fail {
            Err("verify failed".to_string())
        } else {
            Ok(())
        }
    }
}
impl PreparePhase for &mut Probe {
    type Error = String;
    fn prepare(&mut self) -> Result<(), String> {
        self.ran.set(true);
        if self.fail {
            Err("prepare failed".to_string())
        } else {
            Ok(())
        }
    }
}
impl ApplyPhase for &mut Probe {
    type Error = String;
    fn apply(&mut self) -> Result<(), String> {
        self.ran.set(true);
        if self.fail {
            Err("apply failed".to_string())
        } else {
            Ok(())
        }
    }
}

/// 证明: fail-closed 短路 —— verify 失败则 prepare/apply 均不执行,
/// prepare 失败则 apply 不执行 (前置未过不得跳到执行, "RequireApproval
/// 不得跳过审批直接放行"的纯判定投影); 全过 ⇔ Ok。
/// 边界: 三阶段结局为任意布尔组合 (2^3 全量化), 阶段体为探针;
/// Deny/RequireApproval 到 canonical Decision 的映射在 guard/semantics,
/// 不在本纯核量词域; unwind 32。
#[kani::proof]
#[kani::unwind(32)]
fn kani_governance_fail_closed_no_bypass() {
    let (fail_v, fail_p, fail_a) = (
        kani::any::<bool>(),
        kani::any::<bool>(),
        kani::any::<bool>(),
    );
    let mut v = Probe {
        fail: fail_v,
        ran: std::cell::Cell::new(false),
    };
    let mut p = Probe {
        fail: fail_p,
        ran: std::cell::Cell::new(false),
    };
    let mut a = Probe {
        fail: fail_a,
        ran: std::cell::Cell::new(false),
    };

    let outcome = run_fail_closed(&mut v, &mut p, &mut a);

    if fail_v {
        assert!(!p.ran.get() && !a.ran.get(), "verify 失败不得跳到后续阶段");
    }
    if fail_p {
        assert!(!a.ran.get(), "prepare 失败不得跳到 apply");
    }
    match &outcome {
        Ok(()) => {
            assert!(!fail_v && !fail_p && !fail_a, "全过才允许 Ok");
            assert!(
                v.ran.get() && p.ran.get() && a.ran.get(),
                "全过时三阶段皆执行"
            );
        }
        Err(err) => {
            assert!(fail_v || fail_p || fail_a, "有失败必 Err");
            if fail_v {
                assert_eq!(err.phase, FailClosedPhase::Verify);
            } else if fail_p {
                assert_eq!(err.phase, FailClosedPhase::Prepare);
            } else {
                assert_eq!(err.phase, FailClosedPhase::Apply);
            }
        }
    }
}
