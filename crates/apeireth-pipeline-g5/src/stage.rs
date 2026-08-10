//! # Stage — 5 阶段 pipeline stage trait
//!
//! 借鉴 Golutra v0.1.0 `chat_db` 5 阶段 pipeline 思想 (per
//! `analysis\golutra\BORROW_FROM_GOLUTRA.md` §8 P2):
//! 1. **Dispatch** (路由 / 分发) — 决定走哪条路径
//! 2. **Normalize** (归一化) — 清洗输入 (trim / lowercase / dedup)
//! 3. **Policy** (策略) — 拒绝非法输入 (deny-list / quota / scope)
//! 4. **Reliability** (可靠性) — 重试 / 幂等 / 限频重连
//! 5. **Throttle** (限流) — rate limit / back-pressure
//!
//! ## 通用化 (vs LOCKED `apeireth-pipeline` R17 chat 专用)
//!
//! LOCKED `apeireth-pipeline` 是 R17 chat 专用, 处理 4 LLM 协议 (Anthropic / OpenAI / Gemini / ...)
//! 5 步: resolve_placeholders → token_budget → force_translate → protocol_normalize → http_call.
//! 本 crate 借鉴的是 Golutra 5 阶段分类思想, **不**抄 VCP chatCompletionHandler 真业务代码,
//! 通用化后 0 业务耦合, 任何模块 (chat / task / memory / MCP / ...) 都能包成 `Pipeline<T, I, O>`.
//!
//! ## 编译期 hardcode (5 阶段 enum, K-1 强校验)
//!
//! - `STAGE_KIND_COUNT == 5` 守门
//! - `STAGE_ORDER` 编译期数组, 防御 stage 顺序错
//! - 5 阶段 enum 编译期 exhaustive match (`match stage { ... }` 必须覆盖全部 5 variant)

use std::any::Any;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::error::PipelineError;

/// **K-1 强校验 #1**: 5 阶段 pipeline 编译期 hardcode (Dispatch / Normalize / Policy / Reliability / Throttle).
pub const STAGE_KIND_COUNT: usize = 5;

/// 5 阶段 pipeline 顺序 (编译期数组, 防御顺序错).
///
/// 借鉴 Golutra v0.1.0 `chat_db/pipeline/mod.rs` 5 阶段顺序, 简化通用化.
/// 调用方应按 `STAGE_ORDER` 顺序 add stage, 违反时 `Pipeline::run` 返回 `InvalidStageOrder`.
pub const STAGE_ORDER: [StageKind; STAGE_KIND_COUNT] = [
    StageKind::Dispatch,
    StageKind::Normalize,
    StageKind::Policy,
    StageKind::Reliability,
    StageKind::Throttle,
];

/// 5 阶段 pipeline stage 分类 (编译期 enum exhaustive match 守门).
///
/// 顺序固定: Dispatch → Normalize → Policy → Reliability → Throttle.
/// 变体索引 = STAGE_ORDER 数组索引.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StageKind {
    /// 阶段 0: 路由 / 分发. 决定输入走哪条 sub-path.
    Dispatch,
    /// 阶段 1: 归一化. 清洗输入 (trim / lowercase / dedup / format).
    Normalize,
    /// 阶段 2: 策略. 拒绝非法输入 (deny-list / quota / scope check).
    Policy,
    /// 阶段 3: 可靠性. 重试 / 幂等 / 限频重连.
    Reliability,
    /// 阶段 4: 限流. rate limit / back-pressure.
    Throttle,
}

impl fmt::Display for StageKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl StageKind {
    /// 编译期字符串表示 (per 借鉴 Golutra `StageKind::as_str()` 模式).
    pub const fn as_str(&self) -> &'static str {
        match self {
            StageKind::Dispatch => "dispatch",
            StageKind::Normalize => "normalize",
            StageKind::Policy => "policy",
            StageKind::Reliability => "reliability",
            StageKind::Throttle => "throttle",
        }
    }

    /// 在 5 阶段顺序中的索引 (0..5).
    pub const fn order_index(&self) -> usize {
        match self {
            StageKind::Dispatch => 0,
            StageKind::Normalize => 1,
            StageKind::Policy => 2,
            StageKind::Reliability => 3,
            StageKind::Throttle => 4,
        }
    }

    /// 下一阶段 (链尾返回 `None`).
    pub const fn next(&self) -> Option<StageKind> {
        match self {
            StageKind::Dispatch => Some(StageKind::Normalize),
            StageKind::Normalize => Some(StageKind::Policy),
            StageKind::Policy => Some(StageKind::Reliability),
            StageKind::Reliability => Some(StageKind::Throttle),
            StageKind::Throttle => None,
        }
    }
}

/// 编译期守门: STAGE_KIND_COUNT == 5 (K-1 强校验 #1).
const _: () = assert!(STAGE_KIND_COUNT == 5);
/// 编译期守门: STAGE_ORDER 长度 == 5 (K-1 强校验 #1).
const _: () = assert!(STAGE_ORDER.len() == 5);
/// 编译期守门: STAGE_ORDER[0] == Dispatch.
const _: () = assert!(matches!(STAGE_ORDER[0], StageKind::Dispatch));
/// 编译期守门: STAGE_ORDER[4] == Throttle.
const _: () = assert!(matches!(STAGE_ORDER[4], StageKind::Throttle));

/// 通用 5 阶段 pipeline stage trait (input → output + error).
///
/// 实现方必须保证:
/// - `kind()` 返回的 `StageKind` 跟实际行为一致 (e.g. 一个 Dispatch stage 必须 `kind() == StageKind::Dispatch`)
/// - `process(input)` 是**幂等**的 (Reliability 阶段会重试, 幂等才安全)
/// - `I, O` 都满足 `Send + 'static` (允许 type-erased 跨 stage 传递)
///
/// ## 设计权衡: uniform I/O vs 变化 I/O
///
/// 本 trait 接受泛型 `I, O`, 但 `Pipeline<T, I, O>` 假设**所有 stage 共享同样的 `I, O`**.
/// 这是 "service chain" 模式 (类似 `tower::Service`), 简化类型擦除.
///
/// 想要变化 I/O (e.g. Request → Validated → Authorized → Response) 的场景,
/// 在 R21+ 加 `Stage::process_dyn` + `Any` 容器 (留口子, 不在阶段 6 skeleton 做).
pub trait Stage<I, O>: Send + Sync
where
    I: Send + 'static,
    O: Send + 'static,
{
    /// Stage 分类 (e.g. `StageKind::Dispatch`).
    fn kind(&self) -> StageKind;

    /// Stage 名称 (用于日志 / 调试, 默认 `kind().as_str()`).
    fn name(&self) -> &str {
        self.kind().as_str()
    }

    /// 处理输入, 返回输出或错误.
    ///
    /// ## 错误约定
    ///
    /// - `Err(PipelineError::Stage { kind, source })` — 业务处理失败, source 透传 inner
    /// - `Err(PipelineError::PolicyDenied { ... })` — 仅 `kind() == Policy` 用
    /// - `Err(PipelineError::Throttled { ... })` — 仅 `kind() == Throttle` 用
    fn process(&self, input: I) -> Result<O, PipelineError>;
}

/// Stage 容器, 用于把泛型 `Stage<I, O>` 装入 `Vec<Box<dyn StageOp>>`.
///
/// type-erased, 通过 `Any` downcast 还原 `I, O`. 编译期 hardcode 守门:
/// - `process_op` 的 `input` 必须是上 stage 输出的 `O` (downcast 失败时返回 `StageTypeMismatch`)
///
/// 这层抽象是必要的: `Pipeline<T, I, O>` 的 stages Vec 元素类型必须一致 (都是 `StageOp`),
/// 不然 Rust 类型系统不允许把 `Box<dyn Stage<I, O>>` 装进一个 Vec.
pub trait StageOp: Send + Sync {
    /// Stage 分类.
    fn kind(&self) -> StageKind;

    /// Stage 名称.
    fn name(&self) -> &str;

    /// 类型擦除的 process: 接收 `Box<dyn Any>` (实际是上一 stage 的 `O`),
    /// 返回 `Box<dyn Any>` (本 stage 的 `O`, 也就是下一 stage 的 `I`).
    fn process_op(&self, input: Box<dyn Any + Send>) -> Result<Box<dyn Any + Send>, PipelineError>;
}

/// Stage 容器, 把泛型 `Stage<I, O>` 装入 `Vec<Box<dyn StageOp>>`.
///
/// type-erased, 通过 `Any` downcast 还原 `I, O`. 这层抽象是必要的:
/// `Pipeline<T, I, O>` 的 stages Vec 元素类型必须一致 (都是 `StageOp`),
/// 不然 Rust 类型系统不允许把 `Box<dyn Stage<I, O>>` 装进一个 Vec.
///
/// **为什么用 `StageEntry<I, O>` 而不是 blanket impl**:
/// Rust 不允许 `impl<I, O, S> StageOp for S where S: Stage<I, O>`,
/// 因为 `I, O` 在 `StageOp` trait 自身和 self type `S` 中都没有出现 (unconstrained).
/// 用 `StageEntry<I, O>` 把 `I, O` 显式编码进 self type, 约束满足.
pub struct StageEntry<I, O> {
    /// 内层 stage (泛型 `Stage<I, O>`).
    inner: Box<dyn Stage<I, O>>,
    /// PhantomData 保活 `I, O`, 让 StageEntry 自身不变成 unused type param.
    _phantom: std::marker::PhantomData<fn(I) -> O>,
}

impl<I, O> StageEntry<I, O>
where
    I: Send + 'static,
    O: Send + 'static,
{
    /// 创建一个新的 StageEntry (包装任意 `Stage<I, O>`).
    pub fn new(stage: impl Stage<I, O> + 'static) -> Self {
        Self {
            inner: Box::new(stage),
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<I, O> StageOp for StageEntry<I, O>
where
    I: Send + 'static,
    O: Send + 'static,
{
    fn kind(&self) -> StageKind {
        self.inner.kind()
    }

    fn name(&self) -> &str {
        self.inner.name()
    }

    fn process_op(&self, input: Box<dyn Any + Send>) -> Result<Box<dyn Any + Send>, PipelineError> {
        // downcast 到期望的 I 类型 (上一 stage 的 O)
        let typed_input: Box<I> = input
            .downcast::<I>()
            .map_err(|_| PipelineError::StageTypeMismatch {
                expected: std::any::type_name::<I>(),
                actual: "previous stage output (type-erased)".to_string(),
            })?;
        let result: O = Stage::process(&*self.inner, *typed_input)?;
        Ok(Box::new(result))
    }
}
