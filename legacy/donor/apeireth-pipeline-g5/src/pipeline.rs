//! # Pipeline<T, I, O> — 通用 5 阶段 pipeline 链 (核心)
//!
//! 借鉴 Golutra v0.1.0 `chat_db/pipeline/mod.rs` 主链架构 (per
//! `analysis\golutra\BORROW_FROM_GOLUTRA.md` §8 P2), 通用化:
//! - **T** = pipeline 类型 marker (e.g. `ChatPipeline` / `TaskPipeline` / `MemoryPipeline`)
//! - **I** = 入口输入类型 (uniform across stages, Send + 'static)
//! - **O** = 出口输出类型 (同 I, Send + 'static)
//!
//! ## 链行为
//!
//! ```text
//!     ┌────────────────────────────────────────────────────┐
//!     │ Pipeline<T, I, O>                                  │
//!     │ ┌──────────┐  ┌───────────┐  ┌────────┐  ┌─────┐  │
//!  I →│ │ Dispatch │→ │ Normalize │→ │ Policy │→ │ ... │→ │  O
//!     │ └──────────┘  └───────────┘  └────────┘  └─────┘  │
//!     └────────────────────────────────────────────────────┘
//! ```
//!
//! 5 阶段按 STAGE_ORDER 顺序执行, 任意 stage 失败立即返回 `PipelineError`,
//! 不继续后续 stage (fail-fast, 借鉴 Golutra `chat_db/pipeline` 默认行为).
//!
//! ## 编译期守门 (3 项, K-1 强校验)
//!
//! 1. `STAGE_ORDER.len() == 5` 守门 (跟 stage.rs 共享)
//! 2. `Pipeline::new` 拒绝添加 0 stage (返回 `EmptyPipeline` at run time)
//! 3. `Pipeline::run` 验证 stage 顺序 = STAGE_ORDER (返回 `InvalidStageOrder` if mismatch)

use std::any::Any;
use std::fmt;
use std::marker::PhantomData;

use serde::{Deserialize, Serialize};

use crate::error::PipelineError;
use crate::stage::{Stage, StageEntry, StageKind, StageOp, STAGE_KIND_COUNT, STAGE_ORDER};

/// **Hardcode #1**: Pipeline 至少需要 1 stage (空 chain 跑会返回 EmptyPipeline).
pub const PIPELINE_MIN_STAGES: usize = 1;

/// **Hardcode #2**: Pipeline 最多 5 stage (跟 STAGE_KIND_COUNT 对齐).
pub const PIPELINE_MAX_STAGES: usize = 5;

/// **Hardcode #3**: Pipeline 阶段名最大长度 (32 字符, 给日志/调试用).
pub const PIPELINE_STAGE_NAME_MAX_LEN: usize = 32;

/// Pipeline 配置 (跟 stage chain 无关的元数据).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineConfig {
    /// Pipeline 名称 (e.g. "chat-default", "task-priority-router").
    pub name: String,
    /// Pipeline 类型 marker 的字符串表示 (e.g. "ChatPipeline", "TaskPipeline").
    pub type_marker: String,
    /// 是否启用 stage 顺序严格校验 (默认 true; 关闭 = 跳过 STAGE_ORDER 检查).
    pub strict_order: bool,
    /// 是否在 stage 失败时收集诊断信息 (默认 false; 启用会带 trace_id / attempt).
    pub diagnostics: bool,
}

impl PipelineConfig {
    /// 创建默认 config (strict_order=true, diagnostics=false).
    pub fn new(name: impl Into<String>, type_marker: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            type_marker: type_marker.into(),
            strict_order: true,
            diagnostics: false,
        }
    }

    /// 关闭 stage 顺序严格校验 (允许乱序, 留给用户自定义 pipeline).
    pub fn with_strict_order_disabled(mut self) -> Self {
        self.strict_order = false;
        self
    }

    /// 启用 diagnostics (在 stage 失败时记录 attempt / trace_id).
    pub fn with_diagnostics(mut self) -> Self {
        self.diagnostics = true;
        self
    }
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self::new("default", "Unknown")
    }
}

/// Pipeline 链记录 (单次 run 的诊断, 启用 diagnostics 时填充).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineTrace {
    /// Pipeline 名称.
    pub pipeline_name: String,
    /// 实际跑的 stage 序列 (按顺序).
    pub stages_run: Vec<String>,
    /// 失败的 stage 索引 (None = 全部成功).
    pub failed_at: Option<usize>,
}

impl PipelineTrace {
    /// 创建空 trace.
    pub fn new(pipeline_name: impl Into<String>) -> Self {
        Self {
            pipeline_name: pipeline_name.into(),
            stages_run: Vec::new(),
            failed_at: None,
        }
    }

    /// 记录跑过 1 个 stage.
    pub fn record_stage(&mut self, name: impl Into<String>) {
        self.stages_run.push(name.into());
    }

    /// 记录失败位置.
    pub fn record_failure(&mut self, idx: usize) {
        self.failed_at = Some(idx);
    }
}

/// 通用 5 阶段 pipeline 链 (核心类型).
///
/// ## 类型参数
///
/// - `T` — pipeline 类型 marker (用 PhantomData 占位, 编译期区分 pipeline "种类")
/// - `I` — 入口输入类型 (uniform across stages)
/// - `O` — 出口输出类型 (uniform across stages)
///
/// ## Builder 模式
///
/// ```ignore
/// use apeireth_pipeline_g5::{Pipeline, PipelineConfig};
/// use apeireth_pipeline_g5::{DefaultDispatch, DefaultNormalize, DefaultPolicy, DefaultReliability, DefaultThrottle};
///
/// let pipeline: Pipeline<ChatPipeline, PipelineMessage, PipelineMessage> = Pipeline::new(
///     PipelineConfig::new("chat-default", "ChatPipeline")
/// )
/// .with_stage(DefaultDispatch::new())
/// .with_stage(DefaultNormalize::new())
/// .with_stage(DefaultPolicy::new())
/// .with_stage(DefaultReliability::new())
/// .with_stage(DefaultThrottle::new());
/// ```
pub struct Pipeline<T, I, O>
where
    I: Send + 'static,
    O: Send + 'static,
{
    _marker_t: PhantomData<T>,
    _marker_io: PhantomData<fn(I) -> O>,
    config: PipelineConfig,
    stages: Vec<Box<dyn StageOp>>,
}

impl<T, I, O> Pipeline<T, I, O>
where
    I: Send + 'static,
    O: Send + 'static,
{
    /// 创建新 Pipeline (空 chain, 0 stage).
    pub fn new(config: PipelineConfig) -> Self {
        Self {
            _marker_t: PhantomData,
            _marker_io: PhantomData,
            config,
            stages: Vec::new(),
        }
    }

    /// 添加 1 个 stage (链式 builder).
    ///
    /// 阶段 6 skeleton 不在 compile time 校验 stage 类型 (用 type erasure),
    /// runtime `run` 时 downcast 失败会返回 `StageTypeMismatch`.
    pub fn with_stage<S>(mut self, stage: S) -> Self
    where
        S: Stage<I, O> + 'static,
    {
        self.stages.push(Box::new(StageEntry::new(stage)));
        self
    }

    /// 获取 stage 数量.
    pub fn len(&self) -> usize {
        self.stages.len()
    }

    /// 是否空 chain (0 stage).
    pub fn is_empty(&self) -> bool {
        self.stages.is_empty()
    }

    /// 获取 pipeline config (引用).
    pub fn config(&self) -> &PipelineConfig {
        &self.config
    }

    /// 获取所有 stage 的 kind 序列 (按 chain 顺序).
    pub fn stage_kinds(&self) -> Vec<StageKind> {
        self.stages.iter().map(|s| s.kind()).collect()
    }

    /// 跑 pipeline (主入口).
    ///
    /// 行为:
    /// 1. 校验 stage 数 >= 1 (PIPELINE_MIN_STAGES), 0 stage 返回 `EmptyPipeline`
    /// 2. 校验 stage 数 <= 5 (PIPELINE_MAX_STAGES), 超出返回 `InvalidStageOrder`
    /// 3. (可选) 校验 stage 顺序 = STAGE_ORDER (config.strict_order=true 时)
    /// 4. 顺序执行, 任何 stage 失败立即返回 (fail-fast)
    /// 5. 最后 downcast 到 `O`, 类型不匹配返回 `StageTypeMismatch`
    ///
    /// ## 错误
    ///
    /// - `EmptyPipeline` — 0 stage
    /// - `InvalidStageOrder` — stage 数超 5 / 顺序错
    /// - `Stage { kind, source }` — 单 stage 失败
    /// - `PolicyDenied` / `Throttled` — Policy / Throttle 阶段特定错误
    /// - `StageTypeMismatch` — 最后 stage 输出类型不匹配 `O`
    pub fn run(&self, input: I) -> Result<O, PipelineError> {
        // 守门 1: 至少 1 stage (Hardcode #1)
        if self.stages.len() < PIPELINE_MIN_STAGES {
            return Err(PipelineError::EmptyPipeline);
        }

        // 守门 2: 最多 5 stage (Hardcode #2, 跟 STAGE_KIND_COUNT 对齐)
        if self.stages.len() > PIPELINE_MAX_STAGES {
            return Err(PipelineError::InvalidStageOrder {
                got: self
                    .stages
                    .last()
                    .map(|s| s.kind())
                    .unwrap_or(StageKind::Dispatch),
                want: STAGE_ORDER[STAGE_KIND_COUNT - 1],
            });
        }

        // 守门 3: stage 顺序 = STAGE_ORDER (config.strict_order=true)
        if self.config.strict_order {
            for (i, stage) in self.stages.iter().enumerate() {
                let expected = STAGE_ORDER[i];
                let actual = stage.kind();
                if actual != expected {
                    return Err(PipelineError::InvalidStageOrder {
                        got: actual,
                        want: expected,
                    });
                }
            }
        }

        // 跑 chain (fail-fast)
        let mut current: Box<dyn Any + Send> = Box::new(input);
        for (i, stage) in self.stages.iter().enumerate() {
            current = stage.process_op(current).map_err(|e| match e {
                // 透传 stage error, 但加 i (stage 索引) 给 debug
                PipelineError::Stage { kind, source } => PipelineError::Stage {
                    kind,
                    source: format!("[stage {}:{}] {}", i, stage.name(), source).into(),
                },
                other => other,
            })?;
        }

        // 守门 4: 最后 downcast 到 O
        let result: Box<O> =
            current
                .downcast::<O>()
                .map_err(|_| PipelineError::StageTypeMismatch {
                    expected: std::any::type_name::<O>(),
                    actual: "last stage output (type-erased)".to_string(),
                })?;
        Ok(*result)
    }

    /// 跑 pipeline 并收集 trace (diagnostics 用).
    ///
    /// 跟 `run` 行为一致, 但额外返回 `PipelineTrace` 记录跑的 stage 序列和失败位置.
    /// 仅当 `config.diagnostics == true` 时 trace 内容才完整; 否则只记 pipeline_name.
    pub fn run_with_trace(&self, input: I) -> (Result<O, PipelineError>, PipelineTrace) {
        let mut trace = PipelineTrace::new(self.config.name.clone());
        if self.stages.is_empty() {
            return (Err(PipelineError::EmptyPipeline), trace);
        }
        if self.stages.len() > PIPELINE_MAX_STAGES {
            return (
                Err(PipelineError::InvalidStageOrder {
                    got: self
                        .stages
                        .last()
                        .map(|s| s.kind())
                        .unwrap_or(StageKind::Dispatch),
                    want: STAGE_ORDER[STAGE_KIND_COUNT - 1],
                }),
                trace,
            );
        }
        if self.config.strict_order {
            for (i, stage) in self.stages.iter().enumerate() {
                let expected = STAGE_ORDER[i];
                let actual = stage.kind();
                if actual != expected {
                    return (
                        Err(PipelineError::InvalidStageOrder {
                            got: actual,
                            want: expected,
                        }),
                        trace,
                    );
                }
            }
        }

        let mut current: Box<dyn Any + Send> = Box::new(input);
        for (i, stage) in self.stages.iter().enumerate() {
            trace.record_stage(stage.name());
            let result = stage.process_op(current);
            match result {
                Ok(output) => {
                    current = output;
                }
                Err(e) => {
                    trace.record_failure(i);
                    return (Err(e), trace);
                }
            }
        }

        // 守门 4: 最后 downcast 到 O
        match current.downcast::<O>() {
            Ok(v) => (Ok(*v), trace),
            Err(_) => (
                Err(PipelineError::StageTypeMismatch {
                    expected: std::any::type_name::<O>(),
                    actual: "last stage output (type-erased)".to_string(),
                }),
                trace,
            ),
        }
    }
}

impl<T, I, O> fmt::Debug for Pipeline<T, I, O>
where
    I: Send + 'static,
    O: Send + 'static,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Pipeline")
            .field("name", &self.config.name)
            .field("type_marker", &self.config.type_marker)
            .field("stages", &self.stage_kinds())
            .field("stage_count", &self.stages.len())
            .finish()
    }
}

/// 编译期守门: STAGE_KIND_COUNT == 5 (跟 stage.rs 共享约束).
const _: () = assert!(STAGE_KIND_COUNT == 5);
/// 编译期守门: PIPELINE_MIN_STAGES == 1.
const _: () = assert!(PIPELINE_MIN_STAGES == 1);
/// 编译期守门: PIPELINE_MAX_STAGES == 5.
const _: () = assert!(PIPELINE_MAX_STAGES == 5);
/// 编译期守门: PIPELINE_MAX_STAGES == STAGE_KIND_COUNT.
const _: () = assert!(PIPELINE_MAX_STAGES == STAGE_KIND_COUNT);
/// 编译期守门: PIPELINE_STAGE_NAME_MAX_LEN == 32.
const _: () = assert!(PIPELINE_STAGE_NAME_MAX_LEN == 32);
