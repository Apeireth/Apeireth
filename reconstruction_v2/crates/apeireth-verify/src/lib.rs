//! apeireth-verify: 跨 22+ crate 的回归验证机制 (P28 阶段 6).
//!
//! 提供:
//! - [`VerdictTrace`] — 8 字段可审计 verdict trace.
//! - [`RegressionAssertion`] — 4 类结构化断言 (InRange / Monotonic / Idempotent / Equivalent).
//! - `regression_assert!` 宏: 调用方提供表达式, 编译期 + 运行期一致.
//! - `trace_init!` 宏: 一次性初始化每个 crate 的 VERIFY_TRACE.
//! - `register_all_in_crate!` 宏: 显式列出本 crate 全部 AssertionRef ID, 触发运行期注册.
//! - `verify_all` / `run_all` 一键执行全部 crate 注册的断言.

pub mod const_proofs; // R217: 编译期形式化证明 (Kani-style const proof demo)
                      // R177: organ invariants (5 tests + 2 Kani)
mod organ_kani_proofs;

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;

static REGISTRY: OnceLock<std::sync::Mutex<Vec<RegisteredAssertion>>> = OnceLock::new();
static TRACE_COUNTER: AtomicU64 = AtomicU64::new(0);

/// 全局断言注册表项.
#[allow(dead_code)] // fields are registered for future assertion metadata readback
struct RegisteredAssertion {
    crate_name: &'static str,
    description: &'static str,
    assertion: RegressionAssertion,
}

fn registry() -> &'static std::sync::Mutex<Vec<RegisteredAssertion>> {
    REGISTRY.get_or_init(|| std::sync::Mutex::new(Vec::new()))
}

/// 8 字段 verdict trace — 真正可审计.
#[derive(Debug, Clone)]
pub struct VerdictTrace {
    /// 源 crate (例: "apeireth-core")
    pub source: &'static str,
    /// 关联 session id
    pub session_id: String,
    /// V1 (哲学守门) 输出
    pub v1: String,
    /// V2 (权限洋葱) 输出
    pub v2: String,
    /// V3 (默认守门) 输出
    pub v3: String,
    /// 最终 verdict: "allow" / "block" / "pending"
    pub final_verdict: String,
    /// 推理过程 (人读可读)
    pub reasoning: String,
    /// 时间戳 (unix nanoseconds since epoch)
    pub ts: u128,
}

impl VerdictTrace {
    /// 构造一个 verdict trace.
    pub fn new(
        source: &'static str,
        session_id: String,
        v1: String,
        v2: String,
        v3: String,
        final_verdict: String,
        reasoning: String,
    ) -> Self {
        Self {
            source,
            session_id,
            v1,
            v2,
            v3,
            final_verdict,
            reasoning,
            ts: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0),
        }
    }

    /// 所有关键字段都非空.
    pub fn is_complete(&self) -> bool {
        !self.source.is_empty()
            && !self.session_id.is_empty()
            && !self.v1.is_empty()
            && !self.v2.is_empty()
            && !self.v3.is_empty()
            && !self.final_verdict.is_empty()
            && !self.reasoning.is_empty()
    }
}

/// 创建一个空的 `OnceLock<VerdictTrace>` — 给每个 crate 顶层使用.
pub const fn new_trace_slot() -> OnceLock<VerdictTrace> {
    OnceLock::new()
}

/// 4 类结构化断言 (P28 阶段 6 要求 InRange/Monotonic/Idempotent/Equivalent).
#[derive(Debug, Clone)]
pub enum RegressionAssertion {
    /// 数值在 [min, max] 闭区间内.
    InRange {
        /// 断言名 (例: "apeireth-core::invariant-a")
        name: &'static str,
        /// 实测值
        value: f64,
        /// 区间下界
        min: f64,
        /// 区间上界
        max: f64,
    },
    /// 序列满足单调性.
    Monotonic {
        /// 断言名
        name: &'static str,
        /// 序列引用 (Ponytail: 用 slice; A5+ 可换 iterator)
        values: Vec<f64>,
        /// true = 递增, false = 递减
        increasing: bool,
    },
    /// 两次运行结果一致 (幂等).
    Idempotent {
        /// 断言名
        name: &'static str,
        /// 第一次结果
        first: String,
        /// 第二次结果
        second: String,
    },
    /// 两个表达式结果等价.
    Equivalent {
        /// 断言名
        name: &'static str,
        /// 左表达式结果
        left: String,
        /// 右表达式结果
        right: String,
    },
}

impl RegressionAssertion {
    /// 判定断言是否通过.
    pub fn check(&self) -> Result<(), String> {
        match self {
            Self::InRange {
                name,
                value,
                min,
                max,
            } => {
                if *value < *min || *value > *max {
                    Err(format!(
                        "[InRange:{name}] value={value} not in [{min}, {max}]"
                    ))
                } else {
                    Ok(())
                }
            }
            Self::Monotonic {
                name,
                values,
                increasing,
            } => {
                for w in values.windows(2) {
                    let good = if *increasing {
                        w[0] <= w[1]
                    } else {
                        w[0] >= w[1]
                    };
                    if !good {
                        return Err(format!(
                            "[Monotonic:{name}] values={values:?} not {}monotonic",
                            if *increasing { "" } else { "de" }
                        ));
                    }
                }
                Ok(())
            }
            Self::Idempotent {
                name,
                first,
                second,
            } => {
                if first != second {
                    Err(format!("[Idempotent:{name}] {first:?} != {second:?}"))
                } else {
                    Ok(())
                }
            }
            Self::Equivalent { name, left, right } => {
                if left != right {
                    Err(format!("[Equivalent:{name}] {left:?} != {right:?}"))
                } else {
                    Ok(())
                }
            }
        }
    }

    /// 断言名.
    pub fn name(&self) -> &'static str {
        match self {
            Self::InRange { name, .. }
            | Self::Monotonic { name, .. }
            | Self::Idempotent { name, .. }
            | Self::Equivalent { name, .. } => name,
        }
    }
}

/// 4 类断言 (P28 阶段 6 要求 InRange/Monotonic/Idempotent/Equivalent).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AssertionKind {
    /// 数值在 [min, max] 闭区间内.
    InRange,
    /// 序列满足单调性.
    Monotonic,
    /// 两次运行结果一致 (幂等).
    Idempotent,
    /// 两个表达式结果等价.
    Equivalent,
}

/// 编译期可存储的断言描述 (Ponytail: 不依赖 ctor/inventory, 静态 const 数据).
#[derive(Debug, Clone, Copy)]
pub struct AssertionRef {
    /// 所属 crate 名
    pub crate_name: &'static str,
    /// 断言描述
    pub description: &'static str,
    /// 断言类别
    pub kind: AssertionKind,
    /// 断言名
    pub name: &'static str,
    /// 数值字段 A
    pub num_a: f64,
    /// 数值字段 B
    pub num_b: f64,
    /// 数值字段 C
    pub num_c: f64,
    /// 文本字段 A
    pub str_a: &'static str,
    /// 文本字段 B
    pub str_b: &'static str,
    /// Monotonic: true=递增, false=递减
    pub increasing: bool,
}

impl AssertionRef {
    /// 转换为可执行的 `RegressionAssertion` (Ponytail: &self, 不消耗原数据, 允许多次调用).
    pub fn into_assertion(&self) -> RegressionAssertion {
        match self.kind {
            AssertionKind::InRange => RegressionAssertion::InRange {
                name: self.name,
                value: self.num_a,
                min: self.num_b,
                max: self.num_c,
            },
            AssertionKind::Monotonic => RegressionAssertion::Monotonic {
                name: self.name,
                values: vec![self.num_a, self.num_b, self.num_c],
                increasing: self.increasing,
            },
            AssertionKind::Idempotent => RegressionAssertion::Idempotent {
                name: self.name,
                first: self.str_a.to_string(),
                second: self.str_b.to_string(),
            },
            AssertionKind::Equivalent => RegressionAssertion::Equivalent {
                name: self.name,
                left: self.str_a.to_string(),
                right: self.str_b.to_string(),
            },
        }
    }
}

// ============ 宏 ============
//
// ponytail: 宏体只 emit `pub static AssertionRef` (item-context safe, 字段全 const-able).
// 运行期注册需调用方在 crate 顶层用 `register_all_in_crate!(A, B, C);` 显式列出 ID.
// 升级路径: 接入 ctor / inventory crate 后改为真自动注册.

/// 注册一个回归断言 — 顶层调用, 编译期登记 + 运行期可被 `verify_all` 收集.
#[macro_export]
macro_rules! regression_assert {
    ($id:ident, $crate_name:expr, $desc:expr, InRange { name: $n:expr, value: $v:expr, min: $mn:expr, max: $mx:expr }) => {
        #[allow(dead_code, non_camel_case_types)]
        pub static $id: $crate::AssertionRef = $crate::AssertionRef {
            crate_name: $crate_name,
            description: $desc,
            kind: $crate::AssertionKind::InRange,
            name: $n,
            num_a: $v,
            num_b: $mn,
            num_c: $mx,
            str_a: "",
            str_b: "",
            increasing: false,
        };
    };
    ($id:ident, $crate_name:expr, $desc:expr, Monotonic { name: $n:expr, value: $v:expr, second: $b:expr, third: $c:expr, increasing: $i:expr }) => {
        #[allow(dead_code, non_camel_case_types)]
        pub static $id: $crate::AssertionRef = $crate::AssertionRef {
            crate_name: $crate_name,
            description: $desc,
            kind: $crate::AssertionKind::Monotonic,
            name: $n,
            num_a: $v,
            num_b: $b,
            num_c: $c,
            str_a: "",
            str_b: "",
            increasing: $i,
        };
    };
    ($id:ident, $crate_name:expr, $desc:expr, Idempotent { name: $n:expr, first: $f:expr, second: $s:expr }) => {
        #[allow(dead_code, non_camel_case_types)]
        pub static $id: $crate::AssertionRef = $crate::AssertionRef {
            crate_name: $crate_name,
            description: $desc,
            kind: $crate::AssertionKind::Idempotent,
            name: $n,
            num_a: 0.0,
            num_b: 0.0,
            num_c: 0.0,
            str_a: $f,
            str_b: $s,
            increasing: false,
        };
    };
    ($id:ident, $crate_name:expr, $desc:expr, Equivalent { name: $n:expr, left: $l:expr, right: $r:expr }) => {
        #[allow(dead_code, non_camel_case_types)]
        pub static $id: $crate::AssertionRef = $crate::AssertionRef {
            crate_name: $crate_name,
            description: $desc,
            kind: $crate::AssertionKind::Equivalent,
            name: $n,
            num_a: 0.0,
            num_b: 0.0,
            num_c: 0.0,
            str_a: $l,
            str_b: $r,
            increasing: false,
        };
    };
}

/// trace_init! — 顶层声明 VERIFY_TRACE 静态槽.
#[macro_export]
macro_rules! trace_init {
    ($slot:ident) => {
        #[allow(dead_code)]
        pub static $slot: ::std::sync::OnceLock<$crate::VerdictTrace> = $crate::new_trace_slot();
    };
}

/// 便捷宏: 注册一个 Monotonic 断言 (3 采样点 MVP).
#[macro_export]
macro_rules! assert_monotonic {
    ($id:ident, $crate_name:expr, $name:expr, $v1:expr, $v2:expr, $v3:expr, $increasing:expr) => {
        $crate::regression_assert!(
            $id,
            $crate_name,
            concat!("Monotonic ", $name),
            Monotonic {
                name: $name,
                value: $v1 as f64,
                second: $v2 as f64,
                third: $v3 as f64,
                increasing: $increasing
            }
        );
    };
}

/// 便捷宏: 注册一个 Idempotent 断言.
#[macro_export]
macro_rules! assert_idempotent {
    ($id:ident, $crate_name:expr, $name:expr, $first:expr, $second:expr) => {
        $crate::regression_assert!(
            $id,
            $crate_name,
            concat!("Idempotent ", $name),
            Idempotent {
                name: $name,
                first: $first,
                second: $second
            }
        );
    };
}

/// 便捷宏: 注册一个 Equivalent 断言.
#[macro_export]
macro_rules! assert_equivalent {
    ($id:ident, $crate_name:expr, $name:expr, $left:expr, $right:expr) => {
        $crate::regression_assert!(
            $id,
            $crate_name,
            concat!("Equivalent ", $name),
            Equivalent {
                name: $name,
                left: $left,
                right: $right
            }
        );
    };
}

/// register_all_in_crate! — 把当前 crate 顶层声明的全部 AssertionRef 静态注册到全局表.
/// ponytail: 没有 proc-macro / inventory, 需调用方显式列出 ID. 升级路径: 接 inventory / ctor 后改为自动发现.
#[macro_export]
macro_rules! register_all_in_crate {
    ($($id:ident),+ $(,)?) => {
        #[allow(dead_code)]
        pub fn __register_all_asserts() {
            $(
                let r: &$crate::AssertionRef = &$id;
                $crate::do_register(
                    r.crate_name,
                    r.description,
                    r.into_assertion(),
                );
            )+
        }
    };
}

/// 注册一个断言到全局表 (宏内部用).
#[doc(hidden)]
pub fn do_register(crate_name: &'static str, desc: &'static str, a: RegressionAssertion) {
    let _ = TRACE_COUNTER.fetch_add(1, Ordering::Relaxed);
    registry().lock().unwrap().push(RegisteredAssertion {
        crate_name,
        description: desc,
        assertion: a,
    });
}

/// 收集并执行全部注册断言, 返回 (passed, total).
pub fn verify_all() -> Result<(usize, usize), Vec<String>> {
    let reg = registry().lock().unwrap();
    let total = reg.len();
    let mut errs = Vec::new();
    let mut passed = 0;
    for r in reg.iter() {
        match r.assertion.check() {
            Ok(()) => passed += 1,
            Err(e) => errs.push(e),
        }
    }
    if errs.is_empty() {
        Ok((passed, total))
    } else {
        Err(errs)
    }
}

/// 与 `verify_all` 等价 (Q22 兼容名).
pub fn run_all() -> Result<(usize, usize), Vec<String>> {
    verify_all()
}

/// 当前已注册的断言数 (测试用).
pub fn assertion_count() -> usize {
    registry().lock().unwrap().len()
}

/// 重置注册表 (测试用).
#[doc(hidden)]
pub fn reset_for_tests() {
    if let Some(reg) = REGISTRY.get() {
        reg.lock().unwrap().clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verdict_trace_is_complete() {
        let t = VerdictTrace::new(
            "test",
            "s-1".into(),
            "v1".into(),
            "v2".into(),
            "v3".into(),
            "allow".into(),
            "ok".into(),
        );
        assert!(t.is_complete());
        assert!(t.ts > 0);
    }

    #[test]
    fn in_range_check() {
        let a = RegressionAssertion::InRange {
            name: "t::a",
            value: 0.5,
            min: 0.0,
            max: 1.0,
        };
        assert!(a.check().is_ok());
        let b = RegressionAssertion::InRange {
            name: "t::a",
            value: 2.0,
            min: 0.0,
            max: 1.0,
        };
        assert!(b.check().is_err());
    }

    #[test]
    fn monotonic_check() {
        let a = RegressionAssertion::Monotonic {
            name: "t::m",
            values: vec![1.0, 2.0, 3.0],
            increasing: true,
        };
        assert!(a.check().is_ok());
    }

    #[test]
    fn idempotent_check() {
        let a = RegressionAssertion::Idempotent {
            name: "t::i",
            first: "x".into(),
            second: "x".into(),
        };
        assert!(a.check().is_ok());
    }

    #[test]
    fn equivalent_check() {
        let a = RegressionAssertion::Equivalent {
            name: "t::e",
            left: "x".into(),
            right: "x".into(),
        };
        assert!(a.check().is_ok());
    }

    #[test]
    fn assertion_ref_in_range_roundtrip() {
        let r = AssertionRef {
            crate_name: "t",
            description: "d",
            kind: AssertionKind::InRange,
            name: "t::a",
            num_a: 0.5,
            num_b: 0.0,
            num_c: 1.0,
            str_a: "",
            str_b: "",
            increasing: false,
        };
        match r.into_assertion() {
            RegressionAssertion::InRange {
                value, min, max, ..
            } => {
                assert_eq!(value, 0.5);
                assert_eq!(min, 0.0);
                assert_eq!(max, 1.0);
            }
            _ => panic!("expected InRange"),
        }
    }
}

// ============================================================================
// 阶段 6 — 22 trait 互锁 (round8-08)
// ============================================================================
//
// 依据 docs/stage6/22-trait-interlock.md (round8-02 深化, 不修改 stage1-5 LOCKED).
// 本模块: InterlockedTraitKind 22 变体 enum + INTERLOCKED_TRAIT_COUNT + interlock_matrix
// 静态查询 + interlock_assert! 编译期宏.
//
// 守门: 仅在本 crate 新增, 不修改任何其他 crate (council/sovereignty/constraint 不动).

// V-Measure 24 维 + 9 子测度 重导出 (阶段 6 验证基石, round8-08).
//
// 依据 docs/stage6/V-measure-design.md. 实装位于 apeireth-asi (round10-12 qa_engineer).
// apeireth-verify 作为阶段 6 验证锚点, 重导出供跨 crate 集成测试统一引用.
//
// 守门: 仅 re-export, 不二次实装, 不修改 apeireth-asi 源 (7 项不修改承诺).
pub use apeireth_asi::{AsiV05Scores, DimensionTrace, V1136Submeasures};
//
// 依据 docs/stage6/22-trait-interlock.md (round8-02 深化, 不修改 stage1-5 LOCKED).
// 本模块: InterlockedTraitKind 22 变体 enum + INTERLOCKED_TRAIT_COUNT + interlock_matrix
// 静态查询 + interlock_assert! 编译期宏.
//
// 守门: 仅在本 crate 新增, 不修改任何其他 crate (council/sovereignty/constraint 不动).

/// 22 个互锁 trait 的真实身份枚举 (阶段 6 验证基石).
///
/// 依据 docs/stage6/22-trait-interlock.md §1 推导的 22 个核心 trait.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InterlockedTraitKind {
    /// #1 §3.1 感知
    Perception,
    /// #2 §3.1 信号
    Signal,
    /// #3 §3.2 认知
    Cognition,
    /// #4 §3.2 直觉 (PHL-05)
    Intuition,
    /// #5 §3.2 推理
    Reasoning,
    /// #6 §3.2 元认知 (v4.1 §13.2)
    MetaCognition,
    /// #7 §3.3 行动
    Action,
    /// #8 §3.3 执行 (PHL-02b)
    Execution,
    /// #9 §3.3 表达
    Expression,
    /// #10 §3.4 记忆 + §14 子测度 8
    Memory,
    /// #11 §3.4 回忆
    Recall,
    /// #12 §3.4 巩固 + §14 子测度 8
    Consolidation,
    /// #13 §3.5 演化 + 主人修正 #4
    Evolution,
    /// #14 §3.5 学习
    Learning,
    /// #15 §3.5 自我修改 + OTA 守门
    SelfModification,
    /// #16 §3.6 动机
    Motivation,
    /// #17 §3.6 驱动
    Drive,
    /// #18 §3.7 价值
    Value,
    /// #19 §3.8 意识 (v4.1 §13.2)
    Consciousness,
    /// #20 §3.8 自觉 (v4.1 §13.2)
    SelfAwareness,
    /// #21 §3.9 人类权威 + L0 守门
    HumanAuthority,
    /// #22 §3.10 反思 + §14 子测度 9
    Reflection,
}

/// 22 个 trait 的真实计数 (编译期 hardcode).
pub const INTERLOCKED_TRAIT_COUNT: usize = 22;

/// 全部 22 个 trait 列表 (编译期 hardcode 顺序).
pub const INTERLOCKED_TRAITS: [InterlockedTraitKind; INTERLOCKED_TRAIT_COUNT] = [
    InterlockedTraitKind::Perception,
    InterlockedTraitKind::Signal,
    InterlockedTraitKind::Cognition,
    InterlockedTraitKind::Intuition,
    InterlockedTraitKind::Reasoning,
    InterlockedTraitKind::MetaCognition,
    InterlockedTraitKind::Action,
    InterlockedTraitKind::Execution,
    InterlockedTraitKind::Expression,
    InterlockedTraitKind::Memory,
    InterlockedTraitKind::Recall,
    InterlockedTraitKind::Consolidation,
    InterlockedTraitKind::Evolution,
    InterlockedTraitKind::Learning,
    InterlockedTraitKind::SelfModification,
    InterlockedTraitKind::Motivation,
    InterlockedTraitKind::Drive,
    InterlockedTraitKind::Value,
    InterlockedTraitKind::Consciousness,
    InterlockedTraitKind::SelfAwareness,
    InterlockedTraitKind::HumanAuthority,
    InterlockedTraitKind::Reflection,
];

/// 互锁矩阵错误.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InterlockError {
    /// 互锁关系不存在
    NotInMatrix {
        from: &'static str,
        to: &'static str,
    },
    /// 未识别的 trait kind
    UnknownTrait { name: &'static str },
}

impl std::fmt::Display for InterlockError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InterlockError::NotInMatrix { from, to } => {
                write!(f, "互锁矩阵中不存在该依赖关系: {from} → {to}")
            }
            InterlockError::UnknownTrait { name } => write!(f, "未知 trait: {name}"),
        }
    }
}

impl std::error::Error for InterlockError {}

/// trait kind → 名称 (用于错误信息, const fn 供宏使用).
pub const fn trait_name(t: InterlockedTraitKind) -> &'static str {
    match t {
        InterlockedTraitKind::Perception => "Perception",
        InterlockedTraitKind::Signal => "Signal",
        InterlockedTraitKind::Cognition => "Cognition",
        InterlockedTraitKind::Intuition => "Intuition",
        InterlockedTraitKind::Reasoning => "Reasoning",
        InterlockedTraitKind::MetaCognition => "MetaCognition",
        InterlockedTraitKind::Action => "Action",
        InterlockedTraitKind::Execution => "Execution",
        InterlockedTraitKind::Expression => "Expression",
        InterlockedTraitKind::Memory => "Memory",
        InterlockedTraitKind::Recall => "Recall",
        InterlockedTraitKind::Consolidation => "Consolidation",
        InterlockedTraitKind::Evolution => "Evolution",
        InterlockedTraitKind::Learning => "Learning",
        InterlockedTraitKind::SelfModification => "SelfModification",
        InterlockedTraitKind::Motivation => "Motivation",
        InterlockedTraitKind::Drive => "Drive",
        InterlockedTraitKind::Value => "Value",
        InterlockedTraitKind::Consciousness => "Consciousness",
        InterlockedTraitKind::SelfAwareness => "SelfAwareness",
        InterlockedTraitKind::HumanAuthority => "HumanAuthority",
        InterlockedTraitKind::Reflection => "Reflection",
    }
}

/// 互锁矩阵静态查询 (运行期 const fn).
///
/// 依据 docs/stage6/22-trait-interlock.md §3 互锁矩阵 (29 个非对称互锁关系).
/// 返回 true 表示 A → B 互锁关系存在 (实现 A 必须同时实现 B).
pub const fn interlock_matrix(a: InterlockedTraitKind, b: InterlockedTraitKind) -> bool {
    use InterlockedTraitKind::{
        Action, Cognition, Consciousness, Consolidation, Drive, Evolution, Execution, Expression,
        HumanAuthority, Intuition, Learning, Memory, MetaCognition, Motivation, Perception,
        Reasoning, Recall, Reflection, SelfAwareness, SelfModification, Signal, Value,
    };
    matches!(
        (a, b),
        // 感知层
        (Perception, Signal) | (Signal, Perception)
        // 认知层
        | (Cognition, Perception)
        | (Intuition, Cognition) | (Intuition, Reasoning)
        | (Reasoning, Cognition)
        | (MetaCognition, Cognition) | (MetaCognition, Reflection)
        // 行动层
        | (Action, Execution) | (Action, Expression)
        | (Execution, Action) | (Execution, HumanAuthority)
        | (Expression, Action)
        // 记忆层
        | (Memory, Recall) | (Memory, Consolidation)
        | (Recall, Memory)
        | (Consolidation, Memory) | (Consolidation, Evolution)
        // 演化层
        | (Evolution, Learning) | (Evolution, SelfModification)
        | (Learning, Memory) | (Learning, Evolution)
        | (SelfModification, Evolution) | (SelfModification, HumanAuthority)
        // 动机层
        | (Motivation, Drive) | (Motivation, Value)
        | (Drive, Motivation)
        // 价值层 (via PrincipleOnion)
        | (Value, HumanAuthority)
        // 意识层
        | (Consciousness, MetaCognition) | (Consciousness, SelfAwareness)
        | (SelfAwareness, Consciousness)
        // 关系层
        | (Reflection, MetaCognition) | (Reflection, Memory),
    )
}

/// 全部互锁关系计数 (运行期 const fn). 预期 33 (按 docs/stage6/22-trait-interlock.md §3).
pub const INTERLOCK_RELATIONSHIP_COUNT: usize = 33;

/// 互锁断言 macro: 编译期检查 A → B 是否在互锁矩阵中.
///
/// # 用法
/// ```ignore
/// interlock_assert!(Action, Execution);  // ✓ Action impl 必须同时 impl Execution
/// interlock_assert!(Foo, Bar);          // ✗ 编译失败 (Foo/Bar 不在矩阵)
/// ```
#[macro_export]
macro_rules! interlock_assert {
    ($a:expr, $b:expr) => {{
        const _: () = {
            // 编译期检查 a 和 b 都是合法 enum 变体
            let _: $crate::InterlockedTraitKind = $a;
            let _: $crate::InterlockedTraitKind = $b;
            // 编译期检查 a → b 在互锁矩阵中 (const fn)
            assert!(
                $crate::interlock_matrix($a, $b),
                "互锁矩阵中不存在该依赖关系"
            );
        };
    }};
}

/// 编译期断言: 22 个 trait 计数正确 + INTERLOCKED_TRAITS 数组长度匹配.
const _: () = {
    assert!(INTERLOCKED_TRAIT_COUNT == 22, "必须恰好 22 个互锁 trait");
    assert!(
        INTERLOCKED_TRAITS.len() == 22,
        "INTERLOCKED_TRAITS 数组长度必须等于 22"
    );
    // 验证 match 穷尽 (编译期): 添加/删除 InterlockedTraitKind 变体会编译失败
    fn _exhaustive(t: InterlockedTraitKind) -> u8 {
        match t {
            InterlockedTraitKind::Perception => 1,
            InterlockedTraitKind::Signal => 2,
            InterlockedTraitKind::Cognition => 3,
            InterlockedTraitKind::Intuition => 4,
            InterlockedTraitKind::Reasoning => 5,
            InterlockedTraitKind::MetaCognition => 6,
            InterlockedTraitKind::Action => 7,
            InterlockedTraitKind::Execution => 8,
            InterlockedTraitKind::Expression => 9,
            InterlockedTraitKind::Memory => 10,
            InterlockedTraitKind::Recall => 11,
            InterlockedTraitKind::Consolidation => 12,
            InterlockedTraitKind::Evolution => 13,
            InterlockedTraitKind::Learning => 14,
            InterlockedTraitKind::SelfModification => 15,
            InterlockedTraitKind::Motivation => 16,
            InterlockedTraitKind::Drive => 17,
            InterlockedTraitKind::Value => 18,
            InterlockedTraitKind::Consciousness => 19,
            InterlockedTraitKind::SelfAwareness => 20,
            InterlockedTraitKind::HumanAuthority => 21,
            InterlockedTraitKind::Reflection => 22,
        }
    }
};

// ---------------------------------------------------------------------------
// 单元测试 (≥22 个, 验证 interlock 模块)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod interlock_tests {
    use super::*;

    /// 测试 1: INTERLOCKED_TRAIT_COUNT 必须是 22
    #[test]
    fn test_01_trait_count_is_22() {
        assert_eq!(INTERLOCKED_TRAIT_COUNT, 22);
    }

    /// 测试 2: INTERLOCKED_TRAITS 数组长度 = 22
    #[test]
    fn test_02_traits_array_length_22() {
        assert_eq!(INTERLOCKED_TRAITS.len(), 22);
    }

    /// 测试 3: 22 个变体均不重复
    #[test]
    fn test_03_no_duplicate_variants() {
        let mut seen = std::collections::HashSet::new();
        for t in INTERLOCKED_TRAITS.iter() {
            assert!(seen.insert(*t), "重复变体: {t:?}");
        }
        assert_eq!(seen.len(), 22);
    }

    /// 测试 4: 感知 ↔ 信号 (双向互锁)
    #[test]
    fn test_04_perception_signal_bidirectional() {
        assert!(interlock_matrix(
            InterlockedTraitKind::Perception,
            InterlockedTraitKind::Signal
        ));
        assert!(interlock_matrix(
            InterlockedTraitKind::Signal,
            InterlockedTraitKind::Perception
        ));
    }

    /// 测试 5: 直觉 → 认知 + 推理
    #[test]
    fn test_05_intuition_depends_on_cognition_reasoning() {
        assert!(interlock_matrix(
            InterlockedTraitKind::Intuition,
            InterlockedTraitKind::Cognition
        ));
        assert!(interlock_matrix(
            InterlockedTraitKind::Intuition,
            InterlockedTraitKind::Reasoning
        ));
    }

    /// 测试 6: 行动 → 执行 + 表达
    #[test]
    fn test_06_action_depends_on_execution_expression() {
        assert!(interlock_matrix(
            InterlockedTraitKind::Action,
            InterlockedTraitKind::Execution
        ));
        assert!(interlock_matrix(
            InterlockedTraitKind::Action,
            InterlockedTraitKind::Expression
        ));
    }

    /// 测试 7: 执行 → 人类权威 (L0 守门)
    #[test]
    fn test_07_execution_requires_human_authority() {
        assert!(interlock_matrix(
            InterlockedTraitKind::Execution,
            InterlockedTraitKind::HumanAuthority
        ));
    }

    /// 测试 8: 记忆 → 回忆 + 巩固
    #[test]
    fn test_08_memory_depends_on_recall_consolidation() {
        assert!(interlock_matrix(
            InterlockedTraitKind::Memory,
            InterlockedTraitKind::Recall
        ));
        assert!(interlock_matrix(
            InterlockedTraitKind::Memory,
            InterlockedTraitKind::Consolidation
        ));
    }

    /// 测试 9: 演化 → 学习 + 自我修改
    #[test]
    fn test_09_evolution_depends_on_learning_selfmod() {
        assert!(interlock_matrix(
            InterlockedTraitKind::Evolution,
            InterlockedTraitKind::Learning
        ));
        assert!(interlock_matrix(
            InterlockedTraitKind::Evolution,
            InterlockedTraitKind::SelfModification
        ));
    }

    /// 测试 10: 自我修改 → 演化 + 人类权威 (OTA 守门)
    #[test]
    fn test_10_selfmod_requires_evolution_and_ha() {
        assert!(interlock_matrix(
            InterlockedTraitKind::SelfModification,
            InterlockedTraitKind::Evolution
        ));
        assert!(interlock_matrix(
            InterlockedTraitKind::SelfModification,
            InterlockedTraitKind::HumanAuthority
        ));
    }

    /// 测试 11: 动机 → 驱动 + 价值
    #[test]
    fn test_11_motivation_depends_on_drive_value() {
        assert!(interlock_matrix(
            InterlockedTraitKind::Motivation,
            InterlockedTraitKind::Drive
        ));
        assert!(interlock_matrix(
            InterlockedTraitKind::Motivation,
            InterlockedTraitKind::Value
        ));
    }

    /// 测试 12: 价值 → 人类权威 (via PrincipleOnion)
    #[test]
    fn test_12_value_aligned_with_human_authority() {
        assert!(interlock_matrix(
            InterlockedTraitKind::Value,
            InterlockedTraitKind::HumanAuthority
        ));
    }

    /// 测试 13: 意识 → 元认知 + 自觉
    #[test]
    fn test_13_consciousness_depends_on_metacog_selfawareness() {
        assert!(interlock_matrix(
            InterlockedTraitKind::Consciousness,
            InterlockedTraitKind::MetaCognition
        ));
        assert!(interlock_matrix(
            InterlockedTraitKind::Consciousness,
            InterlockedTraitKind::SelfAwareness
        ));
    }

    /// 测试 14: 自觉 → 意识
    #[test]
    fn test_14_selfawareness_depends_on_consciousness() {
        assert!(interlock_matrix(
            InterlockedTraitKind::SelfAwareness,
            InterlockedTraitKind::Consciousness
        ));
    }

    /// 测试 15: 反思 → 元认知 + 记忆
    #[test]
    fn test_15_reflection_depends_on_metacog_memory() {
        assert!(interlock_matrix(
            InterlockedTraitKind::Reflection,
            InterlockedTraitKind::MetaCognition
        ));
        assert!(interlock_matrix(
            InterlockedTraitKind::Reflection,
            InterlockedTraitKind::Memory
        ));
    }

    /// 测试 16: 互锁关系总数 = 33
    #[test]
    fn test_16_relationship_count_is_33() {
        let mut count = 0usize;
        for a in INTERLOCKED_TRAITS.iter() {
            for b in INTERLOCKED_TRAITS.iter() {
                if interlock_matrix(*a, *b) {
                    count += 1;
                }
            }
        }
        assert_eq!(count, INTERLOCK_RELATIONSHIP_COUNT);
        assert_eq!(count, 33);
    }

    /// 测试 17: trait_name 返回正确名称
    #[test]
    fn test_17_trait_name_correctness() {
        assert_eq!(trait_name(InterlockedTraitKind::Perception), "Perception");
        assert_eq!(
            trait_name(InterlockedTraitKind::HumanAuthority),
            "HumanAuthority"
        );
        assert_eq!(
            trait_name(InterlockedTraitKind::SelfModification),
            "SelfModification"
        );
    }

    /// 测试 18: interlock_assert! 宏编译期通过 (合法关系)
    #[test]
    fn test_18_interlock_assert_macro_compiles() {
        // 编译期通过: Action → Execution 在矩阵中
        interlock_assert!(
            InterlockedTraitKind::Action,
            InterlockedTraitKind::Execution
        );
        // 编译期通过: Memory → Recall 在矩阵中
        interlock_assert!(InterlockedTraitKind::Memory, InterlockedTraitKind::Recall);
        // 编译期通过: Reflection → MetaCognition 在矩阵中
        interlock_assert!(
            InterlockedTraitKind::Reflection,
            InterlockedTraitKind::MetaCognition
        );
    }

    /// 测试 19: 互锁矩阵非自反 (Perception → Perception 应为 false)
    #[test]
    fn test_19_matrix_not_reflexive() {
        for t in INTERLOCKED_TRAITS.iter() {
            assert!(!interlock_matrix(*t, *t), "{t:?} 不应自反");
        }
    }

    /// 测试 20: 人类权威是 L0 守门 sink (3 个 trait → HA: Execution / SelfModification / Value)
    #[test]
    fn test_20_human_authority_l0_sink() {
        // HA 是 L0 守门 sink — 3 个 trait 依赖 HA
        let mut count = 0usize;
        for t in INTERLOCKED_TRAITS.iter() {
            if interlock_matrix(*t, InterlockedTraitKind::HumanAuthority) {
                count += 1;
            }
        }
        assert_eq!(count, 3, "应有 3 个 trait → HumanAuthority");
        // 验证具体 3 个
        assert!(interlock_matrix(
            InterlockedTraitKind::Execution,
            InterlockedTraitKind::HumanAuthority
        ));
        assert!(interlock_matrix(
            InterlockedTraitKind::SelfModification,
            InterlockedTraitKind::HumanAuthority
        ));
        assert!(interlock_matrix(
            InterlockedTraitKind::Value,
            InterlockedTraitKind::HumanAuthority
        ));
    }

    /// 测试 21: 全 22 个 trait 均可被 trait_name 识别
    #[test]
    fn test_21_all_traits_have_names() {
        for t in INTERLOCKED_TRAITS.iter() {
            let n = trait_name(*t);
            assert!(!n.is_empty());
            assert!(!n.contains("?"));
        }
    }

    /// 测试 22: InterlockError Display 实现
    #[test]
    fn test_22_interlock_error_display() {
        let err = InterlockError::NotInMatrix {
            from: "Foo",
            to: "Bar",
        };
        let s = format!("{err}");
        assert!(s.contains("Foo"));
        assert!(s.contains("Bar"));
        assert!(s.contains("互锁矩阵"));
    }
}
