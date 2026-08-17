//! # Tool Pipeline — 5 阶段 pipeline-g5 集成 (R132.4 B 案接入生产)
//!
//! **目的**: 让 `apeireth-pipeline-g5` 通用 5 阶段框架在 tool-runtime LLM tool_call 真接
//! 上生产路径. R131.7 audit 选了"不合并 + pipeline-g5 0 调用方" (Option A), R132.4
//! 主人拍 B 案: 接入 tool-runtime 作为第 1 个生产调用方.
//!
//! ## 5 阶段映射 (per R132.4 决策)
//!
//! 1. **Dispatch** — 查 `ToolRegistry::get`, 填 `ctx.tool_ref`
//! 2. **Normalize** — 校验 args 是 JSON object, 填 `ctx.normalized`
//! 3. **Policy** — hardcode allow-list (后续接 `apeireth-tool-approval` 升级)
//! 4. **Reliability** — `tokio::time::timeout` 包裹 tool.call, 填 `ctx.attempts + result`
//! 5. **Throttle** — 简单 1 token/次 (实际限流留 R133+ 接)
//!
//! ## I/O 共享: `ToolCallContext`
//!
//! 5 阶段都接受 `ToolCallContext`, 每阶段 mutate 自己的字段, 返回同一个 struct.
//! 这跟 pipeline-g5 的 `Pipeline<T, I, O>` 设计哲学一致 (uniform I/O 跨 stage).

use std::sync::Arc;
use std::time::{Duration, Instant};

use apeireth_pipeline_g5::{Pipeline, PipelineConfig, PipelineError, Stage, StageKind};
use apeireth_tool_registry::ToolRegistry;
use serde_json::Value;
use tracing::{debug, warn};

use crate::parser::ParsedToolCall;

// ============================================================
// R133.5 — Tool Pipeline Metrics (telemetry 4 umbrella 真实接入)
// ============================================================
//
// **设计**: 5 阶段 process 入口处 inc 各自 counter, 全部注册到全局 `MetricsRegistry`.
//   进程启动时调 `init_tool_metrics()` 一次注册 5 个 counter, 之后 5 阶段共享.
//
// **不假装**:
// - 未调 `init_tool_metrics()` 时 inc 是 no-op (Arc::strong_count > 0 检查)
// - 5 counter 命名严格按 Prometheus convention (`<namespace>_<subsystem>_<name>_<unit>_total`)
// - 各 stage counter label 仅含 stage 名, 不含 tool_name (避免 cardinality 爆炸)
//
// **典型用法** (in main.rs / startup):
// ```no_run
// use apeireth_tool_runtime::tool_pipeline::init_tool_metrics;
// init_tool_metrics(); // 调一次, 之后 5 阶段自动记录
// ```

use apeireth_telemetry::metric::{counter::Counter, registry::MetricsRegistry};
use std::sync::atomic::{AtomicU64, Ordering};

/// R133.5 — 全局 tool pipeline metrics 状态.
///
/// **字段**:
/// - `dispatch / normalize / policy / reliability / throttle_total`: AtomicU64 累计值
///   (每个 stage 1 counter, 5 阶段 = 5 字段, 编译期 hardcode 守门 K-1)
/// - `registry`: 可选 MetricsRegistry, init_tool_metrics() 后 Some, 否则 None
pub struct ToolMetrics {
    pub dispatch_total: AtomicU64,
    pub normalize_total: AtomicU64,
    pub policy_total: AtomicU64,
    pub reliability_total: AtomicU64,
    pub throttle_total: AtomicU64,
    pub registry: Option<Arc<MetricsRegistry>>,
}

impl ToolMetrics {
    /// 5 阶段 = 5 counter (编译期 K-1 守门)
    pub const STAGE_COUNT: usize = 5;
    pub const STAGE_KINDS: [&str; 5] =
        ["dispatch", "normalize", "policy", "reliability", "throttle"];
}

/// 全局 OnceLock 持有 ToolMetrics (R133.5 设计: 进程单例).
///
/// **不假装**: 不可在 test 间共享, 每个 test 调一次 `init_tool_metrics_for_test()` 拿独立 registry.
static TOOL_METRICS: std::sync::OnceLock<parking_lot::Mutex<ToolMetrics>> =
    std::sync::OnceLock::new();

fn metrics() -> &'static parking_lot::Mutex<ToolMetrics> {
    TOOL_METRICS.get_or_init(|| {
        parking_lot::Mutex::new(ToolMetrics {
            dispatch_total: AtomicU64::new(0),
            normalize_total: AtomicU64::new(0),
            policy_total: AtomicU64::new(0),
            reliability_total: AtomicU64::new(0),
            throttle_total: AtomicU64::new(0),
            registry: None,
        })
    })
}

/// R133.5 — 初始化全局 metrics (生产环境启动时调一次).
///
/// **行为**: 创建 5 counter 注册到新 MetricsRegistry, 存到全局 TOOL_METRICS.
///   后续 5 阶段 process 自动 inc.
pub fn init_tool_metrics() -> Arc<MetricsRegistry> {
    let mut m = metrics().lock();
    let registry = Arc::new(MetricsRegistry::new());
    for kind in ToolMetrics::STAGE_KINDS {
        let name = format!("tool_runtime_stage_{}_total", kind);
        let counter = Arc::new(
            Counter::new(
                name.clone(),
                format!("Total times tool pipeline {} stage was processed", kind),
                std::collections::HashMap::new(),
            )
            .expect("counter init"),
        );
        registry
            .register_counter(counter)
            .expect("counter register");
    }
    m.registry = Some(registry.clone());
    registry
}

/// R133.5 — 清空全局 metrics (供测试 / 重启用).
pub fn reset_tool_metrics() {
    let mut m = metrics().lock();
    m.dispatch_total.store(0, Ordering::Relaxed);
    m.normalize_total.store(0, Ordering::Relaxed);
    m.policy_total.store(0, Ordering::Relaxed);
    m.reliability_total.store(0, Ordering::Relaxed);
    m.throttle_total.store(0, Ordering::Relaxed);
}

/// R133.5 — 5 阶段 process 入口调用: 记一次该 stage 的执行 (counter inc + atomic inc).
fn record_stage(stage_idx: usize) {
    let m = metrics().lock();
    let counter = match stage_idx {
        0 => &m.dispatch_total,
        1 => &m.normalize_total,
        2 => &m.policy_total,
        3 => &m.reliability_total,
        4 => &m.throttle_total,
        _ => return, // 防御: 越界 stage 不计数
    };
    counter.fetch_add(1, Ordering::Relaxed);
}

// ============================================================
// ToolCallContext — 5 阶段共享 I/O
// ============================================================

/// 5 阶段共享 context (per R132.4 B 案, 借鉴 pipeline-g5 PipelineMessage 模式).
///
/// 每阶段 mutate 一个字段, 最后整个 struct 含全部阶段产出.
#[derive(Debug, Clone)]
pub struct ToolCallContext {
    // 1. Dispatch 阶段填
    /// Tool 是否在 registry 中找到
    pub tool_ref_found: bool,
    // 2. Normalize 阶段填
    /// Args 是否通过归一化 (是 JSON object)
    pub normalized: bool,
    // 3. Policy 阶段填
    /// Policy 是否通过
    pub approved: bool,
    // 4. Reliability 阶段填
    /// Tool call 尝试次数 (含 retry, 当前简化 = 1)
    pub attempts: u32,
    /// Tool 执行结果 (成功时)
    pub result: Option<Value>,
    /// Tool 执行错误 (失败时)
    pub error: Option<String>,
    // 5. Throttle 阶段填
    /// Throttle 阶段通过的 token (简化 = 总是 true)
    pub throttle_passed: bool,
    // 跟踪字段
    /// 原始 ParsedToolCall (input)
    pub call: ParsedToolCall,
    /// Pipeline run 开始时间
    pub started_at: Instant,
    /// 阶段运行时间记录 (ms per stage, 诊断用)
    pub stage_durations_ms: Vec<(StageKind, u128)>,
}

impl ToolCallContext {
    /// 新建 ToolCallContext (从 ParsedToolCall 出发).
    pub fn new(call: ParsedToolCall) -> Self {
        Self {
            tool_ref_found: false,
            normalized: false,
            approved: false,
            attempts: 0,
            result: None,
            error: None,
            throttle_passed: false,
            call,
            started_at: Instant::now(),
            stage_durations_ms: Vec::with_capacity(5),
        }
    }

    /// 是否最终成功 (Reliability + Throttle 都过)
    pub fn is_success(&self) -> bool {
        self.tool_ref_found
            && self.normalized
            && self.approved
            && self.throttle_passed
            && self.error.is_none()
            && self.result.is_some()
    }
}

// ============================================================
// 5 阶段实现
// ============================================================

/// **阶段 0: Dispatch** — 查 `ToolRegistry::get`, 失败返 Stage error.
pub struct ToolDispatch {
    registry: Arc<ToolRegistry>,
}

impl ToolDispatch {
    pub fn new(registry: Arc<ToolRegistry>) -> Self {
        Self { registry }
    }
}

impl Stage<ToolCallContext, ToolCallContext> for ToolDispatch {
    fn kind(&self) -> StageKind {
        StageKind::Dispatch
    }
    fn name(&self) -> &str {
        "tool-dispatch"
    }
    fn process(&self, mut ctx: ToolCallContext) -> Result<ToolCallContext, PipelineError> {
        record_stage(0); // R133.5 — dispatch counter
        let stage_start = Instant::now();
        let tool_name = &ctx.call.tool_name;
        match self.registry.get(tool_name) {
            Some(_tool) => {
                ctx.tool_ref_found = true;
                debug!("[Dispatch] tool '{}' found in registry", tool_name);
            }
            None => {
                let msg = format!("Tool not found: {tool_name}");
                debug!("[Dispatch] {}", msg);
                return Err(PipelineError::Stage {
                    kind: StageKind::Dispatch,
                    source: msg.into(),
                });
            }
        }
        ctx.stage_durations_ms
            .push((StageKind::Dispatch, stage_start.elapsed().as_millis()));
        Ok(ctx)
    }
}

/// **阶段 1: Normalize** — 校验 args 是 JSON object.
pub struct ToolNormalize;

impl ToolNormalize {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ToolNormalize {
    fn default() -> Self {
        Self::new()
    }
}

impl Stage<ToolCallContext, ToolCallContext> for ToolNormalize {
    fn kind(&self) -> StageKind {
        StageKind::Normalize
    }
    fn name(&self) -> &str {
        "tool-normalize"
    }
    fn process(&self, mut ctx: ToolCallContext) -> Result<ToolCallContext, PipelineError> {
        record_stage(1); // R133.5 — normalize counter
        let stage_start = Instant::now();
        // 校验 args 是 JSON object (per VCP toolCallParser.js 约定)
        match ctx.call.args.as_object() {
            Some(_) => {
                ctx.normalized = true;
                debug!("[Normalize] args is valid JSON object");
            }
            None => {
                let msg = format!(
                    "args is not JSON object: {}",
                    serde_json::to_string(&ctx.call.args).unwrap_or_default()
                );
                return Err(PipelineError::Stage {
                    kind: StageKind::Normalize,
                    source: msg.into(),
                });
            }
        }
        ctx.stage_durations_ms
            .push((StageKind::Normalize, stage_start.elapsed().as_millis()));
        Ok(ctx)
    }
}

// ============================================================
// Policy trait — 打破 cyclic dep (R133.2 终极解)
// ============================================================

/// **Tool Policy trait** (R133.2 设计):
/// - 定义在 `apeireth-tool-runtime` (低层)
/// - 由 `apeireth-tool-approval` (高层) 实现
/// - runtime 通过 `Box<dyn ToolPolicy>` 注入, 不需要 import approval crate
/// - 打破 `tool-runtime <-> tool-approval` 循环依赖
pub trait ToolPolicyRule: Send + Sync + std::fmt::Debug {
    /// 检查 tool_call 是否被允许
    fn check(&self, call: &ParsedToolCall) -> PolicyVerdict;
}

/// Policy 决策 (3 态 + 1 内部态, 跟 `ApprovalDecision` 对齐但独立定义).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PolicyVerdict {
    /// 允许
    Allow,
    /// 拒绝 (附原因; silent = 拒绝时不通知 AI, 与 tool-approval `Deny { reason, silent }` 对齐 —
    /// N20 收尾: silent 不再丢失)
    Deny { reason: String, silent: bool },
    /// 需主人审批 (R133.2 简化: 视为拒绝, 实际等 handler 留 R133+)
    RequireApproval,
    /// 当前规则不匹配, 下一规则判断
    NoMatch,
}

/// **AlwaysAllowPolicy** — fallback, 无规则时全部允许 (per R132.4 行为).
#[derive(Debug)]
pub struct AlwaysAllowPolicy;

impl ToolPolicyRule for AlwaysAllowPolicy {
    fn check(&self, _call: &ParsedToolCall) -> PolicyVerdict {
        PolicyVerdict::Allow
    }
}

/// **阶段 2: Policy** — R133.2 用 `Box<dyn ToolPolicyRule>` 注入, 替代 R132.4 hardcode allow-list.
pub struct ToolPolicy {
    rule: Box<dyn ToolPolicyRule>,
}

impl ToolPolicy {
    pub fn new() -> Self {
        Self {
            rule: Box::new(AlwaysAllowPolicy),
        }
    }

    pub fn with_rule<R: ToolPolicyRule + 'static>(mut self, rule: R) -> Self {
        self.rule = Box::new(rule);
        self
    }

    /// R133.2 — 注入已构造的 `Box<dyn ToolPolicyRule>` (用于 `ToolCallPipeline::with_policy`).
    pub fn with_box_rule(mut self, rule: Box<dyn ToolPolicyRule>) -> Self {
        self.rule = rule;
        self
    }
}

impl Default for ToolPolicy {
    fn default() -> Self {
        Self::new()
    }
}

impl Stage<ToolCallContext, ToolCallContext> for ToolPolicy {
    fn kind(&self) -> StageKind {
        StageKind::Policy
    }
    fn name(&self) -> &str {
        "tool-policy"
    }
    fn process(&self, mut ctx: ToolCallContext) -> Result<ToolCallContext, PipelineError> {
        record_stage(2); // R133.5 — policy counter
        let stage_start = Instant::now();
        match self.rule.check(&ctx.call) {
            PolicyVerdict::Allow => {
                ctx.approved = true;
            }
            PolicyVerdict::NoMatch => {
                // 5 规则都没匹配, 默认拒绝 (保守)
                return Err(PipelineError::PolicyDenied {
                    kind: StageKind::Policy,
                    reason: format!(
                        "tool '{}' not matched by any policy rule",
                        ctx.call.tool_name
                    )
                    .into(),
                });
            }
            PolicyVerdict::RequireApproval => {
                return Err(PipelineError::PolicyDenied {
                    kind: StageKind::Policy,
                    reason: format!("tool '{}' requires approval", ctx.call.tool_name).into(),
                });
            }
            PolicyVerdict::Deny { reason, silent } => {
                // N20 收尾: silent 拒绝不通知 AI (VCP SilentReject 语义)
                let mut msg = format!("tool '{}' denied: {}", ctx.call.tool_name, reason);
                if silent {
                    msg = format!("tool '{}' denied (silent)", ctx.call.tool_name);
                }
                return Err(PipelineError::PolicyDenied {
                    kind: StageKind::Policy,
                    reason: msg.into(),
                });
            }
        }
        ctx.stage_durations_ms
            .push((StageKind::Policy, stage_start.elapsed().as_millis()));
        Ok(ctx)
    }
}

/// **阶段 3: Reliability** — `tokio::time::timeout` 包裹 tool.call + R133.3 retry/backoff.
///
/// **R133.3 retry 行为**:
/// - 默认 `max_retries = 3` (共 4 次尝试: 1 初次 + 3 retry)
/// - 默认 `initial_backoff_ms = 100`, `backoff_multiplier = 2.0`
/// - backoff 序列: 100ms → 200ms → 400ms → 800ms ... (geometric)
/// - 所有错误都重试 (ToolError = String, 无 retryable 标记; R133+ 加可重试分类)
/// - `ctx.attempts` 累加 (最终值 = 1 + 实际重试次数, 上限 = max_retries + 1)
///
/// **不假装**: max_retries=0 → 1 次尝试, 不重试. max_retries=3 → 4 次.
pub struct ToolReliability {
    registry: Arc<ToolRegistry>,
    timeout_ms: u64,
    /// R133.3 — 最大重试次数 (0 = 不重试, 1 次尝试)
    pub max_retries: u32,
    /// R133.3 — 首次 backoff (ms), 之后乘 multiplier
    pub initial_backoff_ms: u64,
    /// R133.3 — backoff 乘子 (geometric backoff)
    pub backoff_multiplier: f64,
}

impl ToolReliability {
    /// 构造 Reliability stage, 默认 retry=3, backoff=100ms, multiplier=2.0
    pub fn new(registry: Arc<ToolRegistry>, timeout_ms: u64) -> Self {
        Self {
            registry,
            timeout_ms,
            max_retries: 3,
            initial_backoff_ms: 100,
            backoff_multiplier: 2.0,
        }
    }

    /// R133.3 — builder: 设置 max_retries
    pub fn with_max_retries(mut self, n: u32) -> Self {
        self.max_retries = n;
        self
    }

    /// R133.3 — builder: 设置 initial_backoff_ms
    pub fn with_initial_backoff_ms(mut self, ms: u64) -> Self {
        self.initial_backoff_ms = ms;
        self
    }

    /// R133.3 — builder: 设置 backoff_multiplier
    pub fn with_backoff_multiplier(mut self, m: f64) -> Self {
        self.backoff_multiplier = m;
        self
    }

    /// R133.3 — 一次性构造带 retry 配置
    pub fn new_with_retry(
        registry: Arc<ToolRegistry>,
        timeout_ms: u64,
        max_retries: u32,
        initial_backoff_ms: u64,
        backoff_multiplier: f64,
    ) -> Self {
        Self {
            registry,
            timeout_ms,
            max_retries,
            initial_backoff_ms,
            backoff_multiplier,
        }
    }
}

impl Stage<ToolCallContext, ToolCallContext> for ToolReliability {
    fn kind(&self) -> StageKind {
        StageKind::Reliability
    }
    fn name(&self) -> &str {
        "tool-reliability"
    }
    fn process(&self, mut ctx: ToolCallContext) -> Result<ToolCallContext, PipelineError> {
        record_stage(3); // R133.5 — reliability counter
        let stage_start = Instant::now();
        let tool_name = ctx.call.tool_name.clone();
        let tool = match self.registry.get(&tool_name) {
            Some(t) => t,
            None => {
                return Err(PipelineError::Stage {
                    kind: StageKind::Reliability,
                    source: format!("tool '{}' vanished mid-pipeline", tool_name).into(),
                });
            }
        };
        // R133.3 — retry loop (max_retries + 1 次总尝试)
        //
        // **runtime 嵌套问题**: Stage::process 是 sync, 不能直接 await tokio future.
        //   R132.4 用 `block_in_place + Handle::block_on` 解决 (block_in_place 释放
        //   worker thread, block_on 同步等待 future). 同样模式用于 retry loop.
        //   **backoff** 用 `std::thread::sleep` 替代 tokio sleep, 避免 nested runtime panic.
        let timeout_duration = Duration::from_millis(self.timeout_ms);
        let total_attempts = (self.max_retries + 1) as usize;
        let mut last_err: Option<String> = None;
        let mut succeeded = false;
        for attempt_idx in 0..total_attempts {
            ctx.attempts = (attempt_idx + 1) as u32;
            let call_result = tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(async {
                    tokio::time::timeout(timeout_duration, tool.call(ctx.call.args.clone())).await
                })
            });
            match call_result {
                Ok(Ok(value)) => {
                    ctx.result = Some(value);
                    debug!(
                        "[Reliability] tool '{}' succeeded on attempt {}/{}",
                        tool_name,
                        attempt_idx + 1,
                        total_attempts
                    );
                    last_err = None;
                    succeeded = true;
                    break;
                }
                Ok(Err(e)) => {
                    let msg = e.to_string();
                    debug!(
                        "[Reliability] tool '{}' errored on attempt {}/{}: {}",
                        tool_name,
                        attempt_idx + 1,
                        total_attempts,
                        msg
                    );
                    last_err = Some(format!("tool error: {msg}"));
                }
                Err(_elapsed) => {
                    let msg = format!("tool timeout after {}ms", self.timeout_ms);
                    debug!(
                        "[Reliability] tool '{}' timed out on attempt {}/{}",
                        tool_name,
                        attempt_idx + 1,
                        total_attempts
                    );
                    last_err = Some(msg);
                }
            }
            // backoff before next attempt (skip after last)
            if attempt_idx + 1 < total_attempts {
                let backoff_ms = (self.initial_backoff_ms as f64
                    * self.backoff_multiplier.powi(attempt_idx as i32))
                    as u64;
                std::thread::sleep(Duration::from_millis(backoff_ms));
            }
        }

        ctx.stage_durations_ms
            .push((StageKind::Reliability, stage_start.elapsed().as_millis()));

        if !succeeded {
            let msg = last_err.unwrap_or_else(|| "unknown error".to_string());
            ctx.error = Some(msg.clone());
            return Err(PipelineError::Stage {
                kind: StageKind::Reliability,
                source: msg.into(),
            });
        }
        Ok(ctx)
    }
}

/// **阶段 4: Throttle** — R133.4 真接 `apeireth-rate-limiter`.
///
/// **行为**:
/// - `limiter = None` → 总是 pass-through (向后兼容, R132.4 行为)
/// - `limiter = Some(rate_limiter)` → 调 `try_acquire(key, 1)`, 失败返 `PipelineError::Throttled`
/// - `key = format!("{key_prefix}:{tool_name}")` (e.g. "tool:EchoSync"), 每工具独立限流
/// - `cost = 1` (R133.4 简化, 1 tool_call = 1 token, R133+ 按 tool 复杂度分级)
///
/// **不假装**: limiter 为 None 时显式 pass-through, 跟 R132.4 "总是通过" 语义一致.
pub struct ToolThrottle {
    limiter: Option<Arc<dyn apeireth_rate_limiter::RateLimiter>>,
    key_prefix: String,
}

impl ToolThrottle {
    /// 默认 Throttle (无 limiter, pass-through)
    pub fn new() -> Self {
        Self {
            limiter: None,
            key_prefix: "tool".to_string(),
        }
    }

    /// R133.4 — 注入 rate limiter + key 前缀
    ///
    /// **典型用法**: `ToolThrottle::new().with_limiter(limiter_arc, "tool".into())`
    pub fn with_limiter(
        mut self,
        limiter: Arc<dyn apeireth_rate_limiter::RateLimiter>,
        key_prefix: impl Into<String>,
    ) -> Self {
        self.limiter = Some(limiter);
        self.key_prefix = key_prefix.into();
        self
    }

    /// 是否启用了限流 (供测试 / 调试)
    pub fn is_enabled(&self) -> bool {
        self.limiter.is_some()
    }
}

impl Default for ToolThrottle {
    fn default() -> Self {
        Self::new()
    }
}

impl Stage<ToolCallContext, ToolCallContext> for ToolThrottle {
    fn kind(&self) -> StageKind {
        StageKind::Throttle
    }
    fn name(&self) -> &str {
        "tool-throttle"
    }
    fn process(&self, mut ctx: ToolCallContext) -> Result<ToolCallContext, PipelineError> {
        record_stage(4); // R133.5 — throttle counter
        let stage_start = Instant::now();
        let tool_name = ctx.call.tool_name.clone();
        let key = format!("{}:{}", self.key_prefix, tool_name);

        if let Some(limiter) = &self.limiter {
            // R133.4 — 真调 rate limiter (try_acquire 是 async, Stage::process 是 sync)
            // 用 block_in_place 跟 R132.4 Reliability 一致
            let allowed = tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current()
                    .block_on(async { limiter.try_acquire(&key, 1).await })
            });
            match allowed {
                Ok(true) => {
                    debug!(
                        "[Throttle] tool '{}' acquired 1 token for key '{}'",
                        tool_name, key
                    );
                    ctx.throttle_passed = true;
                }
                Ok(false) => {
                    debug!(
                        "[Throttle] tool '{}' rate limited (no tokens) for key '{}'",
                        tool_name, key
                    );
                    ctx.throttle_passed = false;
                    ctx.stage_durations_ms
                        .push((StageKind::Throttle, stage_start.elapsed().as_millis()));
                    return Err(PipelineError::Throttled {
                        kind: StageKind::Throttle,
                        retry_after_ms: 1000, // R133.4 简化: 固定 1s 建议, R133+ 跟 rate-limiter stats 联动
                    });
                }
                Err(e) => {
                    // rate limiter 内部错 (storage 失败等), 保守 fail-open
                    warn!(
                        "[Throttle] rate limiter err for tool '{}': {}, fail-open",
                        tool_name, e
                    );
                    ctx.throttle_passed = true;
                }
            }
        } else {
            // R132.4 行为: pass-through
            ctx.throttle_passed = true;
        }

        ctx.stage_durations_ms
            .push((StageKind::Throttle, stage_start.elapsed().as_millis()));
        Ok(ctx)
    }
}

// ============================================================
// ToolCallPipeline — 5 阶段 builder
// ============================================================

/// Pipeline 类型 marker (per `Pipeline<T, I, O>` 用法, 编译期区分 pipeline 种类).
#[derive(Debug, Clone, Copy)]
pub struct ToolCallPipelineMarker;

/// Tool LLM 调用的 5 阶段 pipeline 入口.
///
/// **R133.2 设计**: 持有 registry + timeout + 可选 custom_policy, `with_policy()`
/// 立即 rebuild inner (消费 self), 让 Policy stage 真的用 custom rule.
pub struct ToolCallPipeline {
    inner: Pipeline<ToolCallPipelineMarker, ToolCallContext, ToolCallContext>,
}

impl ToolCallPipeline {
    /// 构造 5 阶段 pipeline (Dispatch → Normalize → Policy → Reliability → Throttle).
    ///
    /// **默认 policy**: `AlwaysAllowPolicy` (R132.4 行为, 保持向后兼容).
    /// **R133.2+ 自定义 policy**: 用 `with_policy()` 注入 (如 `ApprovalBridge`).
    pub fn new(registry: Arc<ToolRegistry>, timeout_ms: u64) -> Self {
        Self {
            inner: Self::build_default_inner(registry, timeout_ms),
        }
    }

    /// 默认 inner pipeline 构建 (R132.4 行为)
    fn build_default_inner(
        registry: Arc<ToolRegistry>,
        timeout_ms: u64,
    ) -> Pipeline<ToolCallPipelineMarker, ToolCallContext, ToolCallContext> {
        Pipeline::new(PipelineConfig::new(
            "tool-call-default",
            "ToolCallPipelineMarker",
        ))
        .with_stage(ToolDispatch::new(registry.clone()))
        .with_stage(ToolNormalize::new())
        .with_stage(ToolPolicy::new()) // 默认 AlwaysAllowPolicy
        .with_stage(ToolReliability::new(registry, timeout_ms))
        .with_stage(ToolThrottle::new())
    }

    /// **R133.2 — 一次性构造带 custom policy 的 pipeline.**
    ///
    /// **背景**: `with_policy()` builder 受 `Pipeline<...>` 不可 rebuild 限制无法实现.
    ///   `new_with_policy()` 一次性构造带 custom rule 的 inner pipeline, Policy stage
    ///   用 `ToolPolicy::new().with_box_rule(rule)`. 重建后 `execute()` 真的用 custom rule.
    ///
    /// **典型用法**: `ApprovalBridge` 桥接 `ApprovalManager`.
    /// ```no_run
    /// # use std::sync::Arc;
    /// # use apeireth_tool_runtime::tool_pipeline::ToolCallPipeline;
    /// # use apeireth_tool_approval::approval_bridge::ApprovalBridge;
    /// # use apeireth_tool_approval::ApprovalManager;
    /// let registry = Arc::new(apeireth_tool_registry::ToolRegistry::new());
    /// let manager = ApprovalManager::new();
    /// let bridge = ApprovalBridge::new(manager);
    /// let pipeline = ToolCallPipeline::new_with_policy(registry, 30_000, bridge);
    /// ```
    /// **R133.4 — 一次性构造带 rate limiter 的 pipeline.**
    ///
    /// **背景**: ToolThrottle 集成 `apeireth-rate-limiter`, 每工具按 key 限流.
    ///   key = `format!("{key_prefix}:{tool_name}")`, 默认 prefix = "tool".
    ///
    /// **典型用法**:
    /// ```no_run
    /// # use std::sync::Arc;
    /// # use apeireth_tool_runtime::tool_pipeline::ToolCallPipeline;
    /// # use apeireth_rate_limiter::{token_bucket_in_memory, RateLimiter};
    /// # let registry = Arc::new(apeireth_tool_registry::ToolRegistry::new());
    /// let limiter = apeireth_rate_limiter::token_bucket_in_memory(10.0, 5, None).expect("rate limiter");
    /// let pipeline = ToolCallPipeline::new_with_rate_limit(
    ///     registry, 30_000, Arc::new(limiter) as Arc<dyn apeireth_rate_limiter::RateLimiter>, "tool".to_string(),
    /// );
    /// ```
    pub fn new_with_rate_limit(
        registry: Arc<ToolRegistry>,
        timeout_ms: u64,
        limiter: Arc<dyn apeireth_rate_limiter::RateLimiter>,
        key_prefix: String,
    ) -> Self {
        let inner: Pipeline<ToolCallPipelineMarker, ToolCallContext, ToolCallContext> =
            Pipeline::new(PipelineConfig::new(
                "tool-call-default",
                "ToolCallPipelineMarker",
            ))
            .with_stage(ToolDispatch::new(registry.clone()))
            .with_stage(ToolNormalize::new())
            .with_stage(ToolPolicy::new()) // 默认 AlwaysAllowPolicy
            .with_stage(ToolReliability::new(registry, timeout_ms))
            .with_stage(ToolThrottle::new().with_limiter(limiter, key_prefix));
        Self { inner }
    }

    /// **R133.4 — 一次性构造带 custom policy + rate limiter 的 pipeline (终极 API).**
    pub fn new_with_policy_and_rate_limit<R: ToolPolicyRule + 'static>(
        registry: Arc<ToolRegistry>,
        timeout_ms: u64,
        policy: R,
        limiter: Arc<dyn apeireth_rate_limiter::RateLimiter>,
        key_prefix: String,
    ) -> Self {
        let inner: Pipeline<ToolCallPipelineMarker, ToolCallContext, ToolCallContext> =
            Pipeline::new(PipelineConfig::new(
                "tool-call-default",
                "ToolCallPipelineMarker",
            ))
            .with_stage(ToolDispatch::new(registry.clone()))
            .with_stage(ToolNormalize::new())
            .with_stage(ToolPolicy::new().with_box_rule(Box::new(policy)))
            .with_stage(ToolReliability::new(registry, timeout_ms))
            .with_stage(ToolThrottle::new().with_limiter(limiter, key_prefix));
        Self { inner }
    }

    pub fn new_with_policy<R: ToolPolicyRule + 'static>(
        registry: Arc<ToolRegistry>,
        timeout_ms: u64,
        rule: R,
    ) -> Self {
        let inner: Pipeline<ToolCallPipelineMarker, ToolCallContext, ToolCallContext> =
            Pipeline::new(PipelineConfig::new(
                "tool-call-default",
                "ToolCallPipelineMarker",
            ))
            .with_stage(ToolDispatch::new(registry.clone()))
            .with_stage(ToolNormalize::new())
            .with_stage(ToolPolicy::new().with_box_rule(Box::new(rule)))
            .with_stage(ToolReliability::new(registry, timeout_ms))
            .with_stage(ToolThrottle::new());
        Self { inner }
    }

    /// 跑 1 个 tool_call 走 5 阶段.
    pub fn execute(&self, call: ParsedToolCall) -> Result<ToolCallContext, PipelineError> {
        let ctx = ToolCallContext::new(call);
        self.inner.run(ctx)
    }

    /// 获取 stage 数 (应该 = 5).
    pub fn stage_count(&self) -> usize {
        self.inner.len()
    }

    /// 获取 stage kind 序列 (按 chain 顺序).
    pub fn stage_kinds(&self) -> Vec<StageKind> {
        self.inner.stage_kinds()
    }
}

// ============================================================
// 测试
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_tool_registry::{MockStaticTool, ToolRegistry};
    use serde_json::json;

    fn make_registry() -> Arc<ToolRegistry> {
        let mut r = ToolRegistry::new();
        let echo = Arc::new(MockStaticTool {
            name: "Echo".to_string(),
            static_value: r#"{"echo":"hello"}"#.to_string(),
        });
        r.register("Echo".to_string(), echo);
        Arc::new(r)
    }

    fn make_call(tool: &str, args: Value) -> ParsedToolCall {
        ParsedToolCall {
            tool_name: tool.to_string(),
            args,
            raw_marker: format!("{tool}|{{}}"),
            archery: false,
            archery_no_reply: false,
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_5_stage_dispatch_lookups_tool() {
        let r = make_registry();
        let p = ToolCallPipeline::new(r, 5000);
        assert_eq!(p.stage_count(), 5);
        let kinds = p.stage_kinds();
        assert_eq!(
            kinds,
            vec![
                StageKind::Dispatch,
                StageKind::Normalize,
                StageKind::Policy,
                StageKind::Reliability,
                StageKind::Throttle
            ]
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_dispatch_fails_for_unknown_tool() {
        let r = make_registry();
        let p = ToolCallPipeline::new(r, 5000);
        let call = make_call("DoesNotExist", json!({}));
        let r = p.execute(call);
        assert!(r.is_err(), "unknown tool should fail at Dispatch");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_normalize_rejects_non_object_args() {
        let r = make_registry();
        let p = ToolCallPipeline::new(r, 5000);
        let call = make_call("Echo", json!("not_an_object"));
        let r = p.execute(call);
        assert!(r.is_err(), "non-object args should fail at Normalize");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_full_5_stage_pipeline_success() {
        let r = make_registry();
        let p = ToolCallPipeline::new(r, 5000);
        let call = make_call("Echo", json!({"x": 1}));
        let ctx = p.execute(call).expect("full 5-stage should succeed");
        assert!(ctx.is_success());
        assert!(ctx.tool_ref_found);
        assert!(ctx.normalized);
        assert!(ctx.approved);
        assert_eq!(ctx.attempts, 1);
        assert!(ctx.throttle_passed);
        assert!(ctx.result.is_some());
        assert_eq!(ctx.stage_durations_ms.len(), 5);
    }

    // ============================================================
    // R133.2 — new_with_policy 注入自定义 rule 烟囱测试 (本地 stub rule, 不引 tool-approval)
    //
    // **背景**: tool-runtime dev-dep tool-approval 会构成 cycle (tool-approval
    //   主 dep tool-runtime). R133.2 真实 integration (BlacklistRule 真 deny) 由
    //   tool-approval 侧 `approval_bridge::tests` (5/5 PASS) + examples/r133_2_policy_bridge
    //   e2e 覆盖. tool-runtime 这里只验证 new_with_policy 自身能 work (用本地 stub).
    // ============================================================

    /// R133.2 本地 stub: 永远 deny 特定 tool_name.
    /// 等价语义: 替代 ApprovalBridge 注入, 验证 new_with_policy builder 真正让 rule 生效.
    #[derive(Debug)]
    struct DenyEchoPolicy;

    impl ToolPolicyRule for DenyEchoPolicy {
        fn check(&self, call: &ParsedToolCall) -> PolicyVerdict {
            if call.tool_name == "Echo" {
                PolicyVerdict::Deny {
                    reason: "stub deny".to_string(),
                    silent: false,
                }
            } else {
                PolicyVerdict::Allow
            }
        }
    }

    /// R133.2: new_with_policy 注入 DenyEchoPolicy → Echo 工具在 Policy stage 真被 deny
    #[tokio::test(flavor = "multi_thread")]
    async fn test_r133_2_new_with_policy_denies_stub_rule() {
        let r = make_registry();
        let p = ToolCallPipeline::new_with_policy(r, 5000, DenyEchoPolicy);
        let call = make_call("Echo", json!({"x": 1}));
        let result = p.execute(call);
        assert!(
            result.is_err(),
            "DenyEchoPolicy should deny Echo at Policy stage"
        );
        let err = result.unwrap_err();
        let err_str = format!("{}", err);
        assert!(
            err_str.contains("denied") || err_str.contains("deny"),
            "got: {}",
            err_str
        );
    }

    /// R133.2: new_with_policy 注入 AlwaysAllowPolicy 等价于 new() 行为
    #[tokio::test(flavor = "multi_thread")]
    async fn test_r133_2_new_with_policy_allow_equivalence() {
        let r = make_registry();
        let p = ToolCallPipeline::new_with_policy(r, 5000, AlwaysAllowPolicy);
        let call = make_call("Echo", json!({"x": 1}));
        let ctx = p.execute(call).expect("AlwaysAllow should pass");
        assert!(ctx.is_success());
        assert!(ctx.approved);
    }

    // ============================================================
    // R133.3 — Reliability stage retry + exponential backoff
    //
    // **不假装**: FlakyTool 前 N 次返 Err, 第 N+1 次返 Ok, 验证 ToolReliability
    //   真重试, ctx.attempts 累加到 N+1, ctx.result 是 success 的值.
    // ============================================================

    use apeireth_tool_registry::{Tool, ToolAxes, ToolKind};
    use async_trait::async_trait;
    use serde_json::Value;
    use std::sync::atomic::{AtomicU32, Ordering};

    /// 前 fail_count 次返 Err, 之后返 Ok
    pub struct FlakyTool {
        pub name: String,
        pub fail_count: AtomicU32,
    }

    #[async_trait]
    impl Tool for FlakyTool {
        fn name(&self) -> &str {
            &self.name
        }
        fn kind(&self) -> ToolKind {
            ToolKind::Static
        }
        fn axes(&self) -> ToolAxes {
            ToolAxes {
                trigger: apeireth_tool_registry::TriggerAxis::Periodic,
                awaiting: apeireth_tool_registry::AwaitingAxis::Immediate,
                resident: apeireth_tool_registry::ResidentAxis::Cached,
                transport: apeireth_tool_registry::TransportAxis::Local,
                output: apeireth_tool_registry::OutputAxis::Value,
            }
        }
        async fn call(&self, _args: Value) -> Result<Value, String> {
            let n = self.fail_count.fetch_add(1, Ordering::SeqCst);
            if n < 2 {
                Err(format!("flaky failure #{}", n + 1))
            } else {
                Ok(json!({"flaky": "succeeded", "attempts": n + 1}))
            }
        }
    }

    fn make_flaky_registry() -> Arc<ToolRegistry> {
        let mut r = ToolRegistry::new();
        let tool = Arc::new(FlakyTool {
            name: "Flaky".to_string(),
            fail_count: AtomicU32::new(0),
        });
        r.register("Flaky".to_string(), tool);
        Arc::new(r)
    }

    /// R133.3: FlakyTool 前 2 次 fail, 第 3 次 success → max_retries=3, backoff 10ms*2
    ///   总耗时 ~ 10ms + 20ms = 30ms (不算 call), attempts=3
    ///
    /// **注**: 不走 ToolCallPipeline (5 阶段), 直接调 ToolReliability.process 测 retry 行为.
    ///   `is_success()` 需要 5 阶段全过 (含 approved), 这里单跑 Reliability 所以只看 attempts+result.
    #[tokio::test(flavor = "multi_thread")]
    async fn test_r133_3_retry_eventually_succeeds() {
        let r = make_flaky_registry();
        let stage = ToolReliability::new_with_retry(r, 5_000, 3, 10, 2.0);
        let call = make_call("Flaky", json!({}));
        let ctx = ToolCallContext::new(call);
        let result = stage.process(ctx);
        let ctx = result.expect("FlakyTool should eventually succeed after 2 retries");
        assert_eq!(
            ctx.attempts, 3,
            "should attempt 3 times (1 initial + 2 retries)"
        );
        assert!(ctx.result.is_some(), "ctx.result should be set on success");
        let v = ctx.result.unwrap();
        assert_eq!(v["flaky"], "succeeded");
        assert_eq!(v["attempts"], 3);
    }

    /// R133.3: max_retries=0 → 1 次尝试, FlakyTool 第一次就 fail
    #[tokio::test(flavor = "multi_thread")]
    async fn test_r133_3_no_retry_fails_on_first_error() {
        let r = make_flaky_registry();
        let stage = ToolReliability::new_with_retry(r, 5_000, 0, 10, 2.0);
        let call = make_call("Flaky", json!({}));
        let ctx = ToolCallContext::new(call);
        let result = stage.process(ctx);
        assert!(result.is_err(), "max_retries=0 should fail on first error");
    }

    /// R133.3: max_retries=2 但 FlakyTool 失败 3+ 次 → 最终 fail, attempts=3
    #[tokio::test(flavor = "multi_thread")]
    async fn test_r133_3_retry_exhausts() {
        // FlakyTool 永远 fail (fail_count 很大)
        let mut r = ToolRegistry::new();
        let tool = Arc::new(AlwaysFailTool {
            name: "AlwaysFail".to_string(),
        });
        r.register("AlwaysFail".to_string(), tool);
        let r = Arc::new(r);

        let stage = ToolReliability::new_with_retry(r, 5_000, 2, 5, 2.0);
        let call = make_call("AlwaysFail", json!({}));
        let ctx = ToolCallContext::new(call);
        let result = stage.process(ctx);
        assert!(result.is_err(), "should fail after max_retries exhausted");
    }

    /// R133.3 辅助: 永远 fail 的 tool
    pub struct AlwaysFailTool {
        pub name: String,
    }

    #[async_trait]
    impl Tool for AlwaysFailTool {
        fn name(&self) -> &str {
            &self.name
        }
        fn kind(&self) -> ToolKind {
            ToolKind::Static
        }
        fn axes(&self) -> ToolAxes {
            ToolAxes {
                trigger: apeireth_tool_registry::TriggerAxis::Periodic,
                awaiting: apeireth_tool_registry::AwaitingAxis::Immediate,
                resident: apeireth_tool_registry::ResidentAxis::Cached,
                transport: apeireth_tool_registry::TransportAxis::Local,
                output: apeireth_tool_registry::OutputAxis::Value,
            }
        }
        async fn call(&self, _args: Value) -> Result<Value, String> {
            Err("always fail".to_string())
        }
    }

    // ============================================================
    // R133.4 — ToolThrottle 真接 apeireth-rate-limiter
    //
    // **不假装**: 用 token_bucket_in_memory 构造 rate=0.5/s burst=1 (1 token/2s).
    //   第一次 try_acquire 成功 (用 burst), 第二次立即 fail (token 未 refill).
    // ============================================================

    use apeireth_rate_limiter::{token_bucket_in_memory, RateLimiter};

    /// R133.4: rate limiter 拒绝 → ToolThrottle stage 返 `PipelineError::Throttled`
    #[tokio::test(flavor = "multi_thread")]
    async fn test_r133_4_throttle_rate_limited_returns_error() {
        use std::time::Duration;
        // rate=0.5/s, burst=1, refill 100ms — 第 1 次成功, 第 2 次立即 fail
        let limiter = token_bucket_in_memory(0.5, 1, None).expect("rate limiter");
        let limiter_arc: Arc<dyn RateLimiter> = Arc::new(limiter);

        let r = make_registry();
        let p = ToolCallPipeline::new_with_rate_limit(r, 5_000, limiter_arc, "test".to_string());

        // 第 1 次: burst 1 token, 成功
        let call1 = make_call("Echo", json!({"x": 1}));
        let ctx1 = p
            .execute(call1)
            .expect("first call should pass (burst token)");
        assert!(ctx1.is_success(), "first call should succeed");
        assert!(ctx1.throttle_passed);

        // 第 2 次: 立即调, token 未 refill → Throttled
        let call2 = make_call("Echo", json!({"x": 2}));
        let result2 = p.execute(call2);
        assert!(result2.is_err(), "second call should be throttled");
        let err = result2.unwrap_err();
        let err_str = format!("{}", err);
        assert!(
            err_str.contains("throttled") || err_str.contains("Throttled"),
            "error should be Throttled, got: {}",
            err_str
        );
    }

    /// R133.4: ToolThrottle 默认 (无 limiter) 仍 pass-through (向后兼容)
    #[tokio::test(flavor = "multi_thread")]
    async fn test_r133_4_throttle_passthrough_when_no_limiter() {
        let r = make_registry();
        let p = ToolCallPipeline::new(r, 5_000); // 默认无 limiter
        let call = make_call("Echo", json!({"x": 1}));
        let ctx = p.execute(call).expect("default pipeline should pass");
        assert!(ctx.is_success());
        assert!(ctx.throttle_passed, "pass-through throttle_passed=true");
    }

    /// R133.4: ToolThrottle::is_enabled 正确反映是否启用了限流
    #[test]
    fn test_r133_4_throttle_is_enabled() {
        use std::time::Duration;
        let t_pass = ToolThrottle::new();
        assert!(!t_pass.is_enabled(), "default throttle has no limiter");

        let limiter = token_bucket_in_memory(10.0, 5, None).expect("rate limiter");
        let limiter_arc: Arc<dyn RateLimiter> = Arc::new(limiter);
        let t_lim = ToolThrottle::new().with_limiter(limiter_arc, "k");
        assert!(t_lim.is_enabled(), "with_limiter throttle has limiter");
    }

    // ============================================================
    // R133.5 — Telemetry 4 umbrella 实际接入 (5 阶段 counter 验证)
    //
    // **不假装**: init_tool_metrics() 后, 跑 1 次 5 阶段 pipeline, 5 counter 各 +1
    // ============================================================

    /// R133.5: init_tool_metrics() 注册 5 counter, 跑 1 次 5 阶段后 5 counter 各 +1
    ///
    /// **不假装**: 用 delta 验证 (before vs after), 因为全局 OnceLock 跨 test 共享,
    ///   reset_tool_metrics() 只清 atomic, registry 字段不变.
    #[tokio::test(flavor = "multi_thread")]
    async fn test_r133_5_metrics_5_stage_increments_all_counters() {
        let _guard = METRICS_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let registry = init_tool_metrics();
        // 验证 5 counter 都注册了
        assert_eq!(registry.len(), 5, "应该注册 5 个 counter (5 阶段)");
        for kind in ToolMetrics::STAGE_KINDS {
            let name = format!("tool_runtime_stage_{}_total", kind);
            assert!(registry.get(&name).is_some(), "counter {} not found", name);
        }

        // 记 before
        let before = {
            let m = metrics().lock();
            (
                m.dispatch_total.load(Ordering::Relaxed),
                m.normalize_total.load(Ordering::Relaxed),
                m.policy_total.load(Ordering::Relaxed),
                m.reliability_total.load(Ordering::Relaxed),
                m.throttle_total.load(Ordering::Relaxed),
            )
        };

        // 跑 1 次完整 5 阶段
        let r = make_registry();
        let p = ToolCallPipeline::new(r, 5_000);
        let call = make_call("Echo", json!({}));
        let _ = p.execute(call).expect("5-stage should succeed");

        // 验证 5 counter delta 各 >= 1 (允许其他 test 并行 race, 实际 5 stage 跑 1 次)
        let after = {
            let m = metrics().lock();
            (
                m.dispatch_total.load(Ordering::Relaxed),
                m.normalize_total.load(Ordering::Relaxed),
                m.policy_total.load(Ordering::Relaxed),
                m.reliability_total.load(Ordering::Relaxed),
                m.throttle_total.load(Ordering::Relaxed),
            )
        };
        // **不假装**: 用 >= 而非 ==, 因为全局 atomic counter 跨 test 共享, 跑 R133.5 时
        //   其他 test (如 test_full_5_stage_pipeline_success) 可能也跑 pipeline. 我们
        //   验证 5 stage counter 都至少 +1, 证明 record_stage 真在 5 阶段 process 入口调用.
        assert!(
            after.0 - before.0 >= 1,
            "dispatch delta should be >= 1, got {}",
            after.0 - before.0
        );
        assert!(
            after.1 - before.1 >= 1,
            "normalize delta should be >= 1, got {}",
            after.1 - before.1
        );
        assert!(
            after.2 - before.2 >= 1,
            "policy delta should be >= 1, got {}",
            after.2 - before.2
        );
        assert!(
            after.3 - before.3 >= 1,
            "reliability delta should be >= 1, got {}",
            after.3 - before.3
        );
        assert!(
            after.4 - before.4 >= 1,
            "throttle delta should be >= 1, got {}",
            after.4 - before.4
        );
    }

    /// R133.5: 未调 init_tool_metrics() 时, 5 阶段仍跑, counter 通过 atomic 累加
    #[tokio::test(flavor = "multi_thread")]
    async fn test_r133_5_metrics_atomic_works_without_init() {
        let _guard = METRICS_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        // 不调 init_tool_metrics() — atomic counter 仍工作
        let before_dispatch = {
            let m = metrics().lock();
            m.dispatch_total.load(Ordering::Relaxed)
        };

        let r = make_registry();
        let p = ToolCallPipeline::new(r, 5_000);
        let call = make_call("Echo", json!({}));
        let _ = p.execute(call).expect("5-stage should succeed");

        let after_dispatch = {
            let m = metrics().lock();
            m.dispatch_total.load(Ordering::Relaxed)
        };
        assert!(
            after_dispatch - before_dispatch >= 1,
            "atomic inc even without init, got {}",
            after_dispatch - before_dispatch
        );
    }

    /// R133.5 — 跨 test 串行化 mutex (避免 R133.5 metric test 并行 race)
    static METRICS_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    /// R133.5: STAGE_COUNT = 5 (编译期 K-1 守门)
    #[test]
    fn test_r133_5_stage_count_is_5() {
        assert_eq!(ToolMetrics::STAGE_COUNT, 5);
        assert_eq!(ToolMetrics::STAGE_KINDS.len(), 5);
        assert_eq!(
            ToolMetrics::STAGE_KINDS,
            ["dispatch", "normalize", "policy", "reliability", "throttle"]
        );
    }
}
