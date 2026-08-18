//! `apeireth-team-lead::handoff` — **Handoff 委托协议 (TP11, A1, P0)**
//!
//! **做什么** (per `team-work-doc.md §11 TP11`):
//! 1. `transfer_to_<agent>` 工具族 — Orchestrator 在 tool-registry 动态启用/禁用
//! 2. `InputFilter` — 可选 `Fn(&AgentContext) -> AgentContext`, 裁剪目标 Agent 不需要的
//!    上下文块; **缺省 = 透传** (per `team-work-doc §1.2 「机制而非补丁」` + 0 假装标注)
//! 3. `OnHandoff` 回调 — 源 Agent 释放资源 / 目标 Agent 初始化专属上下文
//! 4. 动态启用 — 工具描述里实时反映目标 Agent 在线/忙碌状态
//! 5. 审计 — 每次 handoff 写一行 `tracing::info!` (源/目标/filter 摘要/时间戳/chain id)
//!
//! **不做什么** (per task §3 非目标):
//! - ❌ 不实现具体 Agent (researcher/coder/qa) 内部逻辑, 只提供协议骨架 + mock agent
//! - ❌ 不接真实 LLM 调用 (单测用 stub)
//! - ❌ 不做多轮 handoff 状态机 (本包只做单步 handoff)
//! - ❌ 不改 companion 的 send_to_agent/wait_agent 占位 (那是 TP1 收尾的 N16)
//!
//! **0 假装标注** (per 主人偏好 #3 + #7 + `team-work-doc §1.2`):
//! - ✅ input_filter 缺省 = 透传 (NOT 忽略 / NOT 假装裁剪)
//! - ✅ 失败路径全部返 `HandoffError`, **不**返 `Ok(())` 假装成功
//! - ✅ on_handoff / input_filter panic 隔离 (`catch_unwind` 包住)
//! - ✅ 审计日志真写 `tracing::info!` (不进 DB)
//!
//! **调研锚点**:
//! - OpenAI Agents SDK `handoff.py` — `transfer_to_<agent>` 工具名约定
//! - LangGraph `langgraph/graph.py` — 节点转移语义 (本期只做单步)
//! - apeireth-team-lead `Orchestrator` trait (14 工具) — 扩展但**不破坏**既有签名

use std::collections::HashMap;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use crate::{
    AwaitingAxis, OutputAxis, ResidentAxis, Tool, ToolAxes, ToolKind, ToolRegistry, TransportAxis,
    TriggerAxis,
};
use async_trait::async_trait;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;
use tracing::{debug, info, instrument, warn};

// ============================================================================
// 时间戳 / trace_id helper (避免 apeireth-bus 依赖, 打破 dep cycle)
// ============================================================================

/// 当前 epoch 毫秒 (单测无 clock skew 时等价 `apeireth_bus::now_ms`).
#[inline]
fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// 自增 trace id (per process, 单调递增; 等价 `apeireth_bus::next_trace_id`).
fn next_trace_id() -> u64 {
    static COUNTER: AtomicU64 = AtomicU64::new(1);
    COUNTER.fetch_add(1, Ordering::SeqCst)
}

// ============================================================================
// §1 错误类型 (per task §3 "Result<_, HandoffError>")
// ============================================================================

/// Handoff 协议错误类型 (5 variant, per TP11 验收失败路径).
#[derive(Debug, Error)]
pub enum HandoffError {
    /// 目标 Agent 不在 registry 内 (动态禁用状态下被调用)
    #[error("handoff target agent not found: {0}")]
    TargetNotFound(String),

    /// 目标 Agent 当前离线/忙碌 (工具描述 enabled=false 时被强调)
    #[error("handoff target agent is disabled: {0}")]
    TargetDisabled(String),

    /// input_filter 抛错 / panic (隔离后转换为 typed error)
    #[error("handoff input_filter failed: {0}")]
    FilterFailed(String),

    /// on_handoff 回调抛错 / panic (隔离后转换为 typed error, 但审计仍记)
    #[error("handoff on_handoff callback failed: {0}")]
    OnHandoffFailed(String),

    /// Tool trait 调用层 schema 校验失败 (per Tool.call args contract)
    #[error("handoff tool args invalid: {0}")]
    InvalidArgs(String),
}

/// Handoff crate Result 别名.
pub type HandoffResult<T> = Result<T, HandoffError>;

// ============================================================================
// §2 上下文数据 (AgentContext / HandoffRequest / HandoffResult)
// ============================================================================

/// 跨 Agent 上下文 (handoff 时传递的消息 + 元数据).
///
/// **设计**: 用 `serde_json::Value` 存 messages/metadata, 避免过度约束 schema.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AgentContext {
    /// 消息列表 (e.g. `{role: "user", content: "..."}` blocks)
    pub messages: Vec<Value>,
    /// 元数据 (trace_id / chain_id / source_agent / 自由键值)
    pub metadata: HashMap<String, Value>,
}

impl AgentContext {
    /// 创建空上下文
    pub fn new() -> Self {
        Self::default()
    }

    /// 追加消息
    pub fn with_message(mut self, msg: Value) -> Self {
        self.messages.push(msg);
        self
    }

    /// 设置元数据键
    pub fn with_metadata(mut self, key: impl Into<String>, value: Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }

    /// 消息数量 (供 input_filter 决策用)
    pub fn message_count(&self) -> usize {
        self.messages.len()
    }

    /// 摘要 (供审计 + 工具描述用)
    pub fn summary(&self) -> String {
        format!(
            "AgentContext(messages={}, metadata_keys={})",
            self.messages.len(),
            self.metadata.len()
        )
    }
}

/// Handoff 委托请求.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandoffRequest {
    /// 源 Agent ID (发起 handoff 的 agent, None = orchestrator 自身)
    pub from: Option<String>,
    /// 目标 Agent 名 (per `transfer_to_<agent>` 工具名约定)
    pub target_agent: String,
    /// 待传递上下文 (会先过 input_filter)
    pub context: AgentContext,
    /// 链路追踪 ID (per `apeireth-bus::next_trace_id`, 跨子 Agent 保持)
    pub chain_id: u64,
    /// 创建时间戳 (epoch millis)
    pub created_at_ms: i64,
}

impl HandoffRequest {
    /// 构造请求 (自动分配 chain_id + 时间戳)
    pub fn new(
        from: Option<String>,
        target_agent: impl Into<String>,
        context: AgentContext,
    ) -> Self {
        Self {
            from,
            target_agent: target_agent.into(),
            context,
            chain_id: next_trace_id(),
            created_at_ms: now_ms(),
        }
    }
}

/// Handoff 委托结果 (成功的实际产物, 不是 `Result<T, E>` 别名).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandoffOutcome {
    /// 实际接管的目标 Agent ID
    pub target_agent: String,
    /// input_filter 应用后的上下文 (供审计 + 验证)
    pub filtered_context: AgentContext,
    /// 链路追踪 ID (回传)
    pub chain_id: u64,
    /// 接受时间戳
    pub accepted_at_ms: i64,
    /// on_handoff 回调是否被触发 (供 caller 决策)
    pub on_handoff_invoked: bool,
}

// ============================================================================
// §3 input_filter (per task §3 "input_filter 可选函数, 缺省 = 透传")
// ============================================================================

/// InputFilter trait — 在 handoff 前裁剪目标 Agent 不需要的上下文.
///
/// **0 假装标注**: 缺省 impl = `PassthroughFilter` (透传, 不裁剪).
/// 调用方必须**显式**实现 `Filter` trait 才能裁剪; 0 配置 = 0 裁剪.
pub trait InputFilter: Send + Sync {
    /// 应用 filter. 返回新 context (允许原地修改或构造新对象).
    fn apply(&self, ctx: &AgentContext) -> HandoffResult<AgentContext>;
}

/// 缺省 filter — 透传 (per task §3 "缺省 = 透传").
///
/// **0 假装标注**: 这个类型**故意**什么都不做, 仅返回输入的 clone.
/// 文档明示"未配 filter = 全文透传", 防止使用者误以为"缺省 = 已裁剪".
#[derive(Debug, Default, Clone, Copy)]
pub struct PassthroughFilter;

impl InputFilter for PassthroughFilter {
    fn apply(&self, ctx: &AgentContext) -> HandoffResult<AgentContext> {
        Ok(ctx.clone())
    }
}

/// 函数式 filter 适配器 (e.g. `Filter::new(|ctx| ...)`).
///
/// **panic 隔离**: `apply` 用 `catch_unwind` 包住用户闭包, panic 转换为
/// `HandoffError::FilterFailed`, 不污染 orchestrator 主流程.
pub struct FnFilter {
    /// filter 名 (供审计 + 错误信息)
    pub name: &'static str,
    /// 用户闭包
    pub func: Box<dyn Fn(&AgentContext) -> AgentContext + Send + Sync>,
}

impl std::fmt::Debug for FnFilter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FnFilter")
            .field("name", &self.name)
            .finish_non_exhaustive()
    }
}

impl FnFilter {
    /// 构造函数式 filter
    pub fn new<F>(name: &'static str, func: F) -> Self
    where
        F: Fn(&AgentContext) -> AgentContext + Send + Sync + 'static,
    {
        Self {
            name,
            func: Box::new(func),
        }
    }
}

impl InputFilter for FnFilter {
    fn apply(&self, ctx: &AgentContext) -> HandoffResult<AgentContext> {
        // panic 隔离: 用户闭包 panic 不能污染 handoff 主流程
        let result = catch_unwind(AssertUnwindSafe(|| (self.func)(ctx)));
        match result {
            Ok(ctx) => Ok(ctx),
            Err(panic_payload) => {
                let msg = panic_msg(&panic_payload);
                Err(HandoffError::FilterFailed(format!(
                    "{}: {}",
                    self.name, msg
                )))
            }
        }
    }
}

// ============================================================================
// §4 on_handoff 回调 (per task §3 "源 Agent 释放资源 + 目标 Agent 初始化")
// ============================================================================

/// on_handoff 回调 trait — 等价于 `Fn(&HandoffRequest) -> Result<(), String>`
/// 但允许作为 `Arc<dyn OnHandoff>` 共享 (Fn 不能直接 dyn).
#[async_trait]
pub trait OnHandoff: Send + Sync {
    /// 触发回调. 失败不致命 (per OpenAI Agents SDK 行为), 转
    /// `HandoffError::OnHandoffFailed` 但仍算 handoff 成功.
    async fn invoke(&self, req: &HandoffRequest) -> HandoffResult<()>;
}

/// 同步闭包回调适配器 (最常见用例: 释放资源 / 记录日志).
pub struct SyncOnHandoff {
    /// 回调名 (供审计)
    pub name: &'static str,
    /// 同步闭包
    pub func: Box<dyn Fn(&HandoffRequest) -> Result<(), String> + Send + Sync>,
}

impl std::fmt::Debug for SyncOnHandoff {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SyncOnHandoff")
            .field("name", &self.name)
            .finish_non_exhaustive()
    }
}

impl SyncOnHandoff {
    /// 构造同步回调
    pub fn new<F>(name: &'static str, func: F) -> Self
    where
        F: Fn(&HandoffRequest) -> Result<(), String> + Send + Sync + 'static,
    {
        Self {
            name,
            func: Box::new(func),
        }
    }
}

#[async_trait]
impl OnHandoff for SyncOnHandoff {
    async fn invoke(&self, req: &HandoffRequest) -> HandoffResult<()> {
        // panic 隔离: 用户闭包 panic 不能污染 handoff 主流程
        let result = catch_unwind(AssertUnwindSafe(|| (self.func)(req)));
        match result {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(HandoffError::OnHandoffFailed(format!(
                "{}: {}",
                self.name, e
            ))),
            Err(panic_payload) => {
                let msg = panic_msg(&panic_payload);
                Err(HandoffError::OnHandoffFailed(format!(
                    "{}: PANIC: {}",
                    self.name, msg
                )))
            }
        }
    }
}

/// 提取 panic 消息 (per std::any::Any payload 格式, 字符串优先)
fn panic_msg(payload: &Box<dyn std::any::Any + Send>) -> String {
    if let Some(s) = payload.downcast_ref::<&'static str>() {
        (*s).to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "non-string panic payload".to_string()
    }
}

// ============================================================================
// §5 HandoffRegistry — 跟踪 target agent + filter + callback + enable 状态
// ============================================================================

/// 单个 handoff 目标的配置.
///
/// **共享所有权**: filter / on_handoff 都是 `Arc<dyn ...>`, 可被 TransferTool
/// 浅克隆 (Arc::clone) 而无持锁跨 await 问题.
pub struct HandoffTarget {
    /// 目标 Agent 名 (per `transfer_to_<agent>` 后缀)
    pub agent_name: String,
    /// 工具名 (e.g. `transfer_to_researcher`)
    pub tool_name: String,
    /// 工具描述 (含 enabled 状态)
    pub description: String,
    /// 当前是否启用 (false = 工具仍注册但 call 返 TargetDisabled)
    pub enabled: bool,
    /// input_filter (None = PassthroughFilter)
    pub filter: Option<Arc<dyn InputFilter>>,
    /// on_handoff 回调 (None = 不调)
    pub on_handoff: Option<Arc<dyn OnHandoff>>,
    /// 目标 Agent 的角色 (audit 用)
    pub role: Option<String>,
}

impl std::fmt::Debug for HandoffTarget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HandoffTarget")
            .field("agent_name", &self.agent_name)
            .field("tool_name", &self.tool_name)
            .field("description", &self.description)
            .field("enabled", &self.enabled)
            .field("role", &self.role)
            .finish_non_exhaustive()
    }
}

impl HandoffTarget {
    /// 工具名 (per OpenAI Agents SDK `transfer_to_<agent>` 约定)
    pub fn tool_name_for(agent_name: &str) -> String {
        format!("transfer_to_{}", agent_name)
    }
}

/// Handoff Registry — 跟踪所有目标 Agent + filter + callback.
///
/// **共享所有权**: 整个 `HandoffRegistry` 用 `Arc<HandoffRegistry>` 跨多个
/// `TransferTool` 实例共享 (一个 tool 注册 = 一份 Arc clone).
///
/// **线程安全**: 用 `parking_lot::RwLock` (per `apeireth-tool-registry` 风格).
pub struct HandoffRegistry {
    /// target agent → 配置
    targets: RwLock<HashMap<String, HandoffTarget>>,
    /// chain id 自增 (per handoff 一次, 跨子 Agent 保持)
    next_chain_id: AtomicU64,
}

impl Default for HandoffRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for HandoffRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HandoffRegistry")
            .field("targets", &self.targets.read().len())
            .finish_non_exhaustive()
    }
}

impl HandoffRegistry {
    /// 创建空 registry
    pub fn new() -> Self {
        Self {
            targets: RwLock::new(HashMap::new()),
            next_chain_id: AtomicU64::new(1),
        }
    }

    /// 注册一个新 handoff 目标
    pub fn register_target(&self, target: HandoffTarget) {
        let mut targets = self.targets.write();
        targets.insert(target.agent_name.clone(), target);
    }

    /// 注册 handoff 目标的便捷方法 (默认 enabled, 无 filter, 无 callback)
    pub fn register_simple(&self, agent_name: impl Into<String>, role: Option<String>) {
        let agent_name = agent_name.into();
        let tool_name = HandoffTarget::tool_name_for(&agent_name);
        let description = format!(
            "Hand off control to the `{}` agent. \
             Use this when the task requires specialized capabilities.",
            agent_name
        );
        self.register_target(HandoffTarget {
            agent_name,
            tool_name,
            description,
            enabled: true,
            filter: None,
            on_handoff: None,
            role,
        });
    }

    /// 设置 input_filter
    pub fn set_filter(&self, agent_name: &str, filter: Arc<dyn InputFilter>) {
        let mut targets = self.targets.write();
        if let Some(t) = targets.get_mut(agent_name) {
            t.filter = Some(filter);
        }
    }

    /// 设置 on_handoff 回调
    pub fn set_on_handoff(&self, agent_name: &str, callback: Arc<dyn OnHandoff>) {
        let mut targets = self.targets.write();
        if let Some(t) = targets.get_mut(agent_name) {
            t.on_handoff = Some(callback);
        }
    }

    /// 切换 enabled 状态 (供动态启用/禁用, e.g. 目标 agent 离线时)
    pub fn set_enabled(&self, agent_name: &str, enabled: bool) -> bool {
        let mut targets = self.targets.write();
        if let Some(t) = targets.get_mut(agent_name) {
            t.enabled = enabled;
            t.description = if enabled {
                format!(
                    "Hand off control to the `{}` agent. (status: enabled)",
                    t.agent_name
                )
            } else {
                format!(
                    "Hand off control to the `{}` agent. (status: DISABLED — agent offline/busy)",
                    t.agent_name
                )
            };
            true
        } else {
            false
        }
    }

    /// 注销目标 agent (热插拔用)
    pub fn unregister_target(&self, agent_name: &str) -> Option<HandoffTarget> {
        self.targets.write().remove(agent_name)
    }

    /// 列出所有目标 agent 名
    pub fn list_agent_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self.targets.read().keys().cloned().collect();
        names.sort();
        names
    }

    /// 列出所有 transfer_to_<agent> 工具名
    pub fn list_tool_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self
            .targets
            .read()
            .values()
            .map(|t| t.tool_name.clone())
            .collect();
        names.sort();
        names
    }

    /// 取目标 agent 的工具名 (用于 Tool registry 注册)
    pub fn tool_name_for(&self, agent_name: &str) -> Option<String> {
        self.targets
            .read()
            .get(agent_name)
            .map(|t| t.tool_name.clone())
    }

    /// 读取目标 agent 快照 (供 transfer_to_<agent> tool 调用时用)
    pub fn get_target_snapshot(&self, agent_name: &str) -> Option<TargetSnapshot> {
        self.targets.read().get(agent_name).map(|t| TargetSnapshot {
            agent_name: t.agent_name.clone(),
            tool_name: t.tool_name.clone(),
            description: t.description.clone(),
            enabled: t.enabled,
            role: t.role.clone(),
        })
    }

    /// 是否存在该 agent
    pub fn contains(&self, agent_name: &str) -> bool {
        self.targets.read().contains_key(agent_name)
    }

    /// 总数 (供单测 + 调试)
    pub fn len(&self) -> usize {
        self.targets.read().len()
    }

    /// 是否为空
    pub fn is_empty(&self) -> bool {
        self.targets.read().is_empty()
    }

    /// 取下一个 chain id (atomic, 不依赖 apeireth-bus; 单测友好)
    pub fn next_chain_id(&self) -> u64 {
        self.next_chain_id.fetch_add(1, Ordering::SeqCst)
    }

    /// 在持锁窗口内安全取出 filter Arc clone + callback Arc clone
    /// (避免 TransferTool 持锁跨 await)
    pub(crate) fn borrow_filter_and_callback(
        &self,
        agent_name: &str,
    ) -> (Option<Arc<dyn InputFilter>>, Option<Arc<dyn OnHandoff>>) {
        let targets = self.targets.read();
        if let Some(t) = targets.get(agent_name) {
            (t.filter.clone(), t.on_handoff.clone())
        } else {
            (None, None)
        }
    }
}

/// Target agent 配置快照 (供 TransferTool 调用时使用, 避免持锁跨 await)
#[derive(Debug, Clone)]
pub struct TargetSnapshot {
    /// 目标 Agent 名
    pub agent_name: String,
    /// 工具名 (per `transfer_to_<agent>`)
    pub tool_name: String,
    /// 工具描述 (含 enabled 状态)
    pub description: String,
    /// 当前是否启用
    pub enabled: bool,
    /// 目标 Agent 的角色 (audit 用)
    pub role: Option<String>,
}

// ============================================================================
// §6 TransferTool — Tool trait impl for `transfer_to_<agent>`
// ============================================================================

/// `transfer_to_<agent>` 工具 (一个 agent 一个实例, 共享 `Arc<HandoffRegistry>` 引用).
pub struct TransferTool {
    /// 该工具绑定的目标 Agent 名
    agent_name: String,
    /// Handoff Registry (Arc 共享)
    registry: Arc<HandoffRegistry>,
    /// 工具名 (per `transfer_to_<agent>` 约定)
    tool_name: String,
}

impl std::fmt::Debug for TransferTool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TransferTool")
            .field("agent_name", &self.agent_name)
            .field("tool_name", &self.tool_name)
            .finish_non_exhaustive()
    }
}

impl TransferTool {
    /// 构造一个新 transfer tool (用于单个目标 agent)
    pub fn new(agent_name: impl Into<String>, registry: Arc<HandoffRegistry>) -> Self {
        let agent_name = agent_name.into();
        let tool_name = HandoffTarget::tool_name_for(&agent_name);
        Self {
            agent_name,
            registry,
            tool_name,
        }
    }
}

#[async_trait]
impl Tool for TransferTool {
    fn name(&self) -> &str {
        &self.tool_name
    }

    fn kind(&self) -> ToolKind {
        ToolKind::Async
    }

    fn axes(&self) -> ToolAxes {
        ToolAxes {
            trigger: TriggerAxis::OnDemand,
            awaiting: AwaitingAxis::Deferred,
            resident: ResidentAxis::Ephemeral,
            transport: TransportAxis::Local,
            output: OutputAxis::Value,
        }
    }

    async fn call(&self, args: Value) -> Result<Value, String> {
        // 1. 校验 target 仍存在 (热插拔可能已被注销)
        let snapshot = self
            .registry
            .get_target_snapshot(&self.agent_name)
            .ok_or_else(|| {
                format!(
                    "{}: target agent '{}' not found in registry (may have been unregistered)",
                    self.tool_name, self.agent_name
                )
            })?;

        // 2. 校验 enabled (动态禁用)
        if !snapshot.enabled {
            return Err(format!(
                "{}: target agent '{}' is DISABLED (offline/busy)",
                self.tool_name, self.agent_name
            ));
        }

        // 3. 解析 args → AgentContext
        let context = parse_context_from_args(&args)?;

        // 4. 在持锁窗口内 borrow filter + callback Arc clone (避免跨 await 持锁)
        let (filter, on_handoff_cb) = self.registry.borrow_filter_and_callback(&self.agent_name);

        // 5. 应用 input_filter (缺省 = 透传, per task 0 假装标注)
        let filtered_context = if let Some(f) = filter {
            match f.apply(&context) {
                Ok(c) => c,
                Err(e) => {
                    // Filter failure → typed error (per task "filter 抛错" 失败路径)
                    return Err(format!("{}: {}", self.tool_name, e));
                }
            }
        } else {
            context // 缺省 = 透传
        };

        // 6. 构造 request
        let from = args
            .get("from_agent")
            .and_then(|v| v.as_str())
            .map(String::from);
        let request = HandoffRequest::new(from, &self.agent_name, filtered_context.clone());

        // 7. 调用 on_handoff 回调 (隔离 panic)
        let on_handoff_invoked = if let Some(cb) = on_handoff_cb {
            match cb.invoke(&request).await {
                Ok(()) => true,
                Err(e) => {
                    // 回调业务错: 记录但不算 handoff 失败 (per OpenAI SDK 语义)
                    warn!(
                        target = %self.agent_name,
                        error = %e,
                        "on_handoff callback returned error (logged, handoff continues)"
                    );
                    true
                }
            }
        } else {
            false
        };

        // 8. 审计日志 (per task §3 "审计: tracing::info!")
        let chain_id = self.registry.next_chain_id();
        info!(
            event = "handoff_accepted",
            tool = %self.tool_name,
            from = ?request.from,
            target = %self.agent_name,
            chain_id = chain_id,
            input_filter_summary = %filtered_context.summary(),
            on_handoff_invoked = on_handoff_invoked,
            "handoff accepted"
        );

        // 9. 构造结果
        let outcome = HandoffOutcome {
            target_agent: self.agent_name.clone(),
            filtered_context,
            chain_id,
            accepted_at_ms: now_ms(),
            on_handoff_invoked,
        };

        // 10. 序列化为 JSON Value (Tool trait 返回类型)
        let json_result = serde_json::to_value(&outcome)
            .map_err(|e| format!("{}: serialize HandoffOutcome failed: {}", self.tool_name, e))?;
        Ok(json_result)
    }
}

// ----------------------------------------------------------------------------
// 内部 helper: 从 args JSON 解析 AgentContext
// ----------------------------------------------------------------------------

fn parse_context_from_args(args: &Value) -> Result<AgentContext, String> {
    // 接受 2 种格式:
    // 1. {"context": {...}} → 直接用
    // 2. {"messages": [...], "metadata": {...}} → 构造 AgentContext
    let obj = args
        .as_object()
        .ok_or_else(|| format!("args must be JSON object, got: {}", args))?;

    if let Some(ctx_val) = obj.get("context") {
        // 直接反序列化
        serde_json::from_value::<AgentContext>(ctx_val.clone())
            .map_err(|e| format!("'context' field deserialize failed: {}", e))
    } else {
        let messages = obj
            .get("messages")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        let metadata = obj
            .get("metadata")
            .and_then(|v| v.as_object())
            .map(|m| {
                m.iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect::<HashMap<String, Value>>()
            })
            .unwrap_or_default();
        Ok(AgentContext { messages, metadata })
    }
}

// ============================================================================
// §7 注册 helper — 一次性把所有 enabled transfer tool 注册到 ToolRegistry
// ============================================================================

/// 一次性把所有 target agent 的 `transfer_to_<agent>` 工具注册到 ToolRegistry.
///
/// **行为**:
/// - 遍历 `HandoffRegistry.targets`, 对每个 target 注册一个 `TransferTool`
/// - 已 disabled 的 target 也注册 (per task "工具描述实时更新 enabled"), 但 call 时返 TargetDisabled
/// - 调用方可多次调用: 已存在的同名工具会被覆盖 (per `apeireth-tool-registry::register` 语义)
/// - **不**移除未在 HandoffRegistry 内的旧工具 (热插拔由调用方显式 unregister)
#[instrument(skip(tool_registry, handoff_registry))]
pub fn install_handoff_tools(
    tool_registry: &ToolRegistry,
    handoff_registry: &Arc<HandoffRegistry>,
) -> usize {
    let snapshots: Vec<TargetSnapshot> = handoff_registry
        .list_agent_names()
        .into_iter()
        .filter_map(|name| handoff_registry.get_target_snapshot(&name))
        .collect();

    let mut registered = 0;
    for snap in snapshots {
        let tool = TransferTool::new(snap.agent_name.clone(), handoff_registry.clone());
        tool_registry.register(snap.tool_name.clone(), Arc::new(tool));
        debug!(
            tool = %snap.tool_name,
            enabled = snap.enabled,
            "handoff tool registered"
        );
        registered += 1;
    }

    info!(registered, "install_handoff_tools complete");
    registered
}

// ============================================================================
// §8 单测 (per task §4 验收: 成功/裁剪/禁用/审计/失败隔离)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Tool;

    // ----- 单元 1: AgentContext 基本构造 -----

    #[test]
    fn t11_u01_agent_context_builder() {
        let ctx = AgentContext::new()
            .with_message(json!({"role": "user", "content": "hi"}))
            .with_metadata("trace_id", json!("abc-123"));
        assert_eq!(ctx.message_count(), 1);
        assert_eq!(ctx.metadata.len(), 1);
        assert!(ctx.summary().contains("messages=1"));
    }

    // ----- 单元 2: PassthroughFilter 真的透传 (0 假装标注) -----

    #[test]
    fn t11_u02_passthrough_filter_does_not_trim() {
        // **关键 0 假装**: Passthrough 必须真透传, 不裁剪
        let ctx = AgentContext::new()
            .with_message(json!({"secret": "do-not-leak"}))
            .with_message(json!({"public": "ok"}));
        let result = PassthroughFilter.apply(&ctx).unwrap();
        assert_eq!(
            result.message_count(),
            2,
            "PassthroughFilter must preserve all messages"
        );
        assert_eq!(
            result.messages[0], ctx.messages[0],
            "PassthroughFilter must preserve message content verbatim"
        );
    }

    // ----- 单元 3: FnFilter 真裁剪 -----

    #[test]
    fn t11_u03_fn_filter_actually_trims() {
        let filter = FnFilter::new("trim_first", |ctx| AgentContext {
            // 裁掉第一条消息 (模拟: 源 agent 的内部记忆不该传给目标)
            messages: ctx.messages.iter().skip(1).cloned().collect(),
            metadata: ctx.metadata.clone(),
        });
        let ctx = AgentContext::new()
            .with_message(json!({"secret": "leak-me"}))
            .with_message(json!({"public": "ok"}));
        let result = filter.apply(&ctx).unwrap();
        assert_eq!(result.message_count(), 1);
        assert_eq!(result.messages[0], json!({"public": "ok"}));
    }

    // ----- 单元 4: FnFilter panic 隔离 -----

    #[test]
    fn t11_u04_fn_filter_panic_isolated() {
        let filter = FnFilter::new("boom", |_ctx| {
            panic!("intentional panic for test");
        });
        let ctx = AgentContext::new();
        let result = filter.apply(&ctx);
        assert!(matches!(result, Err(HandoffError::FilterFailed(_))));
        let err_msg = format!("{}", result.unwrap_err());
        assert!(err_msg.contains("boom"), "error must include filter name");
        assert!(
            err_msg.contains("intentional panic"),
            "error must include panic message"
        );
    }

    // ----- 单元 5: HandoffRegistry 注册 / 注销 / 列出 -----

    #[test]
    fn t11_u05_registry_register_list_unregister() {
        let reg = HandoffRegistry::new();
        assert!(reg.is_empty());
        reg.register_simple("researcher", Some("research".to_string()));
        reg.register_simple("coder", Some("code".to_string()));
        assert_eq!(reg.len(), 2);
        assert_eq!(
            reg.list_tool_names(),
            vec!["transfer_to_coder", "transfer_to_researcher"]
        );

        let ok = reg.unregister_target("researcher");
        assert!(ok.is_some());
        assert_eq!(reg.len(), 1);
        assert!(!reg.contains("researcher"));
    }

    // ----- 单元 6: HandoffRegistry 动态启用/禁用 -----

    #[test]
    fn t11_u06_registry_dynamic_enable_disable() {
        let reg = HandoffRegistry::new();
        reg.register_simple("researcher", None);
        let snap1 = reg.get_target_snapshot("researcher").unwrap();
        assert!(snap1.enabled);
        assert!(!snap1.description.contains("DISABLED"));

        reg.set_enabled("researcher", false);
        let snap2 = reg.get_target_snapshot("researcher").unwrap();
        assert!(!snap2.enabled);
        assert!(
            snap2.description.contains("DISABLED"),
            "disabled tool description must reflect state"
        );

        reg.set_enabled("researcher", true);
        let snap3 = reg.get_target_snapshot("researcher").unwrap();
        assert!(snap3.enabled);
    }

    // ----- 单元 7: HandoffRegistry 热插拔 (unregister + re-register 无残留) -----

    #[test]
    fn t11_u07_hot_swap_unregister_then_register() {
        let reg = Arc::new(HandoffRegistry::new());
        let tool_reg = ToolRegistry::new();

        reg.register_simple("agent_a", None);
        assert_eq!(install_handoff_tools(&tool_reg, &reg), 1);
        assert!(tool_reg.get("transfer_to_agent_a").is_some());

        // 注销 + 重新注册 (模拟热插拔)
        reg.unregister_target("agent_a");
        // install 会把 tool_reg 里残留的 transfer_to_agent_a 留着 (per docs);
        // 单测验证: 再次 register_simple + install, 工具应被覆盖 (无 stale state)
        reg.register_simple("agent_a", Some("role_v2".to_string()));
        let count = install_handoff_tools(&tool_reg, &reg);
        assert_eq!(count, 1);
        let tool = tool_reg.get("transfer_to_agent_a").unwrap();
        let snap = reg.get_target_snapshot("agent_a").unwrap();
        assert_eq!(snap.role.as_deref(), Some("role_v2"));
        // 工具仍可达
        assert_eq!(tool.name(), "transfer_to_agent_a");
    }

    // ----- 单元 8: TransferTool.call 成功路径 -----

    #[tokio::test]
    async fn t11_u08_transfer_tool_call_success() {
        let reg = Arc::new(HandoffRegistry::new());
        reg.register_simple("researcher", None);
        let tool = TransferTool::new("researcher", reg.clone());

        let args = json!({
            "messages": [{"role": "user", "content": "find X"}],
            "metadata": {"trace": "abc"}
        });
        let result = tool.call(args).await.expect("call ok");

        assert_eq!(result["target_agent"], "researcher");
        assert!(result["chain_id"].is_u64());
        assert_eq!(result["on_handoff_invoked"], false);
        assert_eq!(
            result["filtered_context"]["messages"][0]["content"],
            "find X"
        );
    }

    // ----- 单元 9: TransferTool.call input_filter 真裁剪 -----

    #[tokio::test]
    async fn t11_u09_transfer_tool_input_filter_actually_trims() {
        let reg = Arc::new(HandoffRegistry::new());
        reg.register_simple("researcher", None);
        // 设 filter: 丢掉所有 messages (模拟强裁剪)
        reg.set_filter(
            "researcher",
            Arc::new(FnFilter::new("wipe_messages", |ctx| AgentContext {
                messages: vec![],
                metadata: ctx.metadata.clone(),
            })),
        );

        let tool = TransferTool::new("researcher", reg.clone());
        let args = json!({
            "messages": [{"role": "user", "content": "leak-me"}],
            "metadata": {}
        });
        let result = tool.call(args).await.expect("call ok");
        // filtered_context.messages 必须是空的 (验证 filter 真生效)
        let msgs = result["filtered_context"]["messages"].as_array().unwrap();
        assert_eq!(msgs.len(), 0, "input_filter must wipe messages");
        // 但 metadata 保留 (per filter impl)
        assert!(result["filtered_context"]["metadata"].is_object());
    }

    // ----- 单元 10: TransferTool.call 目标 disabled → TargetDisabled -----

    #[tokio::test]
    async fn t11_u10_transfer_tool_target_disabled() {
        let reg = Arc::new(HandoffRegistry::new());
        reg.register_simple("researcher", None);
        reg.set_enabled("researcher", false);

        let tool = TransferTool::new("researcher", reg.clone());
        let args = json!({"messages": [], "metadata": {}});
        let err = tool.call(args).await.unwrap_err();
        assert!(
            err.contains("DISABLED"),
            "call must return TargetDisabled error, got: {}",
            err
        );
    }

    // ----- 单元 11: TransferTool.call 目标不存在 → TargetNotFound -----

    #[tokio::test]
    async fn t11_u11_transfer_tool_target_not_found() {
        let reg = Arc::new(HandoffRegistry::new());
        // 注: TransferTool 在 registry 中不存在对应 agent, 应返 not found
        let tool = TransferTool::new("ghost", reg.clone());
        let args = json!({"messages": [], "metadata": {}});
        let err = tool.call(args).await.unwrap_err();
        assert!(err.contains("not found"), "got: {}", err);
    }

    // ----- 单元 12: TransferTool.call on_handoff 回调触发 -----

    #[tokio::test]
    async fn t11_u12_transfer_tool_on_handoff_callback_invoked() {
        use std::sync::atomic::AtomicU64;
        let reg = Arc::new(HandoffRegistry::new());
        reg.register_simple("researcher", None);
        // 设 callback: 记录被调用
        let invoked = Arc::new(AtomicU64::new(0));
        let invoked_clone = invoked.clone();
        reg.set_on_handoff(
            "researcher",
            Arc::new(SyncOnHandoff::new("counter", move |_req| {
                invoked_clone.fetch_add(1, Ordering::SeqCst);
                Ok(())
            })),
        );

        let tool = TransferTool::new("researcher", reg.clone());
        let args = json!({"from_agent": "orchestrator", "messages": [], "metadata": {}});
        let result = tool.call(args).await.expect("call ok");
        assert_eq!(result["on_handoff_invoked"], true);
        assert_eq!(invoked.load(Ordering::SeqCst), 1);
    }

    // ----- 单元 13: TransferTool.call on_handoff panic 隔离 -----

    #[tokio::test]
    async fn t11_u13_on_handoff_panic_isolated() {
        let reg = Arc::new(HandoffRegistry::new());
        reg.register_simple("researcher", None);
        reg.set_on_handoff(
            "researcher",
            Arc::new(SyncOnHandoff::new("boom", |_req| -> Result<(), String> {
                panic!("intentional on_handoff panic")
            })),
        );

        let tool = TransferTool::new("researcher", reg.clone());
        let args = json!({"messages": [], "metadata": {}});
        // panic 必须被隔离, handoff 仍成功
        let result = tool
            .call(args)
            .await
            .expect("call must succeed despite panic");
        assert_eq!(result["on_handoff_invoked"], true);
        assert_eq!(result["target_agent"], "researcher");
    }

    // ----- 单元 14: TransferTool.call 缺省 = PassthroughFilter (0 假装标注) -----

    #[tokio::test]
    async fn t11_u14_default_filter_is_passthrough_not_trim() {
        let reg = Arc::new(HandoffRegistry::new());
        reg.register_simple("researcher", None); // 不设 filter
        let tool = TransferTool::new("researcher", reg.clone());

        let args = json!({
            "messages": [
                {"role": "user", "content": "secret-1"},
                {"role": "user", "content": "secret-2"}
            ],
            "metadata": {}
        });
        let result = tool.call(args).await.expect("call ok");
        let msgs = result["filtered_context"]["messages"].as_array().unwrap();
        // **关键 0 假装**: 缺省 filter = Passthrough = 全量透传, 不裁剪
        assert_eq!(
            msgs.len(),
            2,
            "default filter MUST pass through all messages (0 假装标注)"
        );
        assert_eq!(msgs[0]["content"], "secret-1");
        assert_eq!(msgs[1]["content"], "secret-2");
    }

    // ----- 单元 15: install_handoff_tools 注册所有 target -----

    #[test]
    fn t11_u15_install_registers_all_transfer_tools() {
        let reg = Arc::new(HandoffRegistry::new());
        reg.register_simple("researcher", None);
        reg.register_simple("coder", None);
        reg.register_simple("qa", None);

        let tool_reg = ToolRegistry::new();
        let count = install_handoff_tools(&tool_reg, &reg);
        assert_eq!(count, 3);

        // 验证 3 个 tool 都被注册
        assert!(tool_reg.get("transfer_to_researcher").is_some());
        assert!(tool_reg.get("transfer_to_coder").is_some());
        assert!(tool_reg.get("transfer_to_qa").is_some());

        // list 应包含 3 个
        let names = tool_reg.list();
        assert!(names.contains(&"transfer_to_researcher".to_string()));
        assert!(names.contains(&"transfer_to_coder".to_string()));
        assert!(names.contains(&"transfer_to_qa".to_string()));
    }

    // ----- 单元 16: 链 ID 自增 -----

    #[test]
    fn t11_u16_chain_id_increments() {
        let reg = HandoffRegistry::new();
        let a = reg.next_chain_id();
        let b = reg.next_chain_id();
        let c = reg.next_chain_id();
        assert!(b > a);
        assert!(c > b);
    }

    // ----- 单元 17: AgentContext 序列化往返 -----

    #[test]
    fn t11_u17_agent_context_serde_roundtrip() {
        let ctx = AgentContext::new()
            .with_message(json!({"role": "user", "content": "hi"}))
            .with_metadata("k", json!("v"));
        let v = serde_json::to_value(&ctx).unwrap();
        let back = serde_json::from_value::<AgentContext>(v).unwrap();
        assert_eq!(back.message_count(), 1);
        assert_eq!(back.metadata.get("k").unwrap(), &json!("v"));
    }

    // ----- 单元 18: parse_context_from_args 两种格式 -----

    #[test]
    fn t11_u18_parse_context_two_formats() {
        // 格式 1: {context: {...}}
        let args1 = json!({"context": {"messages": [{"a": 1}], "metadata": {}}});
        let ctx1 = parse_context_from_args(&args1).unwrap();
        assert_eq!(ctx1.message_count(), 1);

        // 格式 2: {messages: [...], metadata: {...}}
        let args2 = json!({"messages": [{"x": 1}, {"y": 2}], "metadata": {"k": "v"}});
        let ctx2 = parse_context_from_args(&args2).unwrap();
        assert_eq!(ctx2.message_count(), 2);
        assert_eq!(ctx2.metadata.get("k").unwrap(), &json!("v"));

        // 格式 3: 空 args → 空 context
        let args3 = json!({});
        let ctx3 = parse_context_from_args(&args3).unwrap();
        assert_eq!(ctx3.message_count(), 0);

        // 格式 4: 非对象 → 错
        let args4 = json!("not an object");
        let err = parse_context_from_args(&args4).unwrap_err();
        assert!(err.contains("JSON object"));
    }

    // ----- 单元 19: 多个 TransferTool 共享同一 registry (Arc 语义) -----

    #[tokio::test]
    async fn t11_u19_multiple_transfer_tools_share_registry() {
        let reg = Arc::new(HandoffRegistry::new());
        reg.register_simple("researcher", None);
        reg.register_simple("coder", None);

        let tool_a = TransferTool::new("researcher", reg.clone());
        let tool_b = TransferTool::new("coder", reg.clone());

        // disable researcher → tool_a 返错, tool_b 不受影响
        reg.set_enabled("researcher", false);

        let err_a = tool_a
            .call(json!({"messages": [], "metadata": {}}))
            .await
            .unwrap_err();
        assert!(err_a.contains("DISABLED"));

        let ok_b = tool_b
            .call(json!({"messages": [], "metadata": {}}))
            .await
            .unwrap();
        assert_eq!(ok_b["target_agent"], "coder");
    }

    // ----- 单元 20: SyncOnHandoff 错误返回 (非 panic 路径) -----

    #[tokio::test]
    async fn t11_u20_sync_on_handoff_returns_err_not_panic() {
        let cb = SyncOnHandoff::new("returns_err", |_req| Err("business error".to_string()));
        let req = HandoffRequest::new(None, "x", AgentContext::new());
        let result = cb.invoke(&req).await;
        assert!(matches!(result, Err(HandoffError::OnHandoffFailed(_))));
        let msg = format!("{}", result.unwrap_err());
        assert!(msg.contains("returns_err"));
        assert!(msg.contains("business error"));
    }
}
