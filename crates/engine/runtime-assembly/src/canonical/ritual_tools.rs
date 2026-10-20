//! 生物机制族 W3.7 · 仪式完成面 (`tool.ritual_done` / `tool.ritual_skip`)。
//!
//! # 补的是哪半程
//!
//! organ [`apeireth_organ::ritual`] 是纯内核 (只给候选与到期判定), 状态由调用方
//! 持有; 存储面 [`crate::canonical::care_reminder::ritual_state_store`] (W3.5)
//! 已把 `<care_dir>/ritual_state.json` 读写接上, 但**生产没有「仪式完成」事件源**:
//! 候选被发起 ≠ 仪式真的发生过, 从「已发起」推定「已完成」就是编数据。
//!
//! 本模块补上**显式事件源**: 两个模型可调的低风险工具, 只在用户**明确说**
//! 某场仪式已经发生 (或明确拒绝) 时由模型调用, 把 `last_done_ms` /
//! `consecutive_done` / `skipped_count` 写回同一本 `ritual_state.json`。
//!
//! | 工具 | 语义 | 副作用 |
//! |---|---|---|
//! | `tool.ritual_done` | 用户明确确认「某仪式已完成」→ 记一次完成 | 本地统计 +1 行 (`last_done_ms` 前移 / `consecutive_done` 计数) |
//! | `tool.ritual_skip` | 用户明确说「这次跳过」→ 记一次跳过 | 本地统计 (`skipped_count`), **不动**完成时刻与静默 |
//!
//! # 注册 (默认关, 复用族总闸)
//!
//! 装配门 = env [`BIO_CARE_CANDIDATES_ENABLE_ENV`] (`APEIRETH_ENABLE_BIO_CARE_CANDIDATES`,
//! **只认 `"1"`**, 未设置/空/`0`/`true`/其它一律关 = fail-closed) **且** 已注入
//! 关怀数据目录 ([`ProductionModulesConfig::care_schedule`], 与 care 心跳同一目录)。
//! 全关 = 不构造存储、不注册 (零读取零写入, 不创建 `ritual_state.json`)。
//! 缺目录 = 无处落盘 = **不注册** (fail-closed, 与 play 工具面「缺根不注册」同口径)。
//!
//! # 0 装诚实 (四级口径)
//!
//! **IMPLEMENTED + PRODUCTION WIRED (默认关)** —— 装配根在开关打开时注册两个工具;
//! 不是 DEFAULT ENABLED, 不是 HARDWARE VALIDATED, **未做 live E2E**。
//!
//! - **事件源是「模型判定 + 用户显式确认」, 不是自动监听**: 没有从消息回执、发起
//!   记录或通道送达自动推定完成 —— 那会把「提醒过」洗成「做到了」。模型误调
//!   (用户没确认却调了) 会前移完成时刻, 这是本面已知风险, 参数描述里写明「只在
//!   用户明确确认时调用」; 内核仍按正常到期节奏走, 不存在「补课/亏欠」文案。
//! - **只写本地统计**: 不发消息、不推进 care 管道、不改排程、不写主体记忆
//!   (无 memory / episode / wiki 写入调用), 不产生 streak 惩罚 —— 跳过只是计数,
//!   完成只是时刻前移, 二者都不触发任何补偿动作。
//! - **单写者假设**: 文件落盘走 [`RitualStateStore`] 的进程内串行锁 (读 → 改 → 写
//!   原子写, 坏文件拒写不覆盖); 同进程**另一实例** (如 cli 的显式注入口) 与本工具
//!   面并发写时互不加锁, 可能丢失一次更新 —— 部署保证同一数据目录单写者, 本批不做
//!   跨实例/跨进程锁。
//! - **消费侧在心跳宿主**: 写下的状态由 care 心跳 (gateway serve 的
//!   `CareHeartbeatSource`) 每次轮询现读; 纯 CLI chat 无心跳宿主时, 写下的状态
//!   暂时没有候选消费方 —— 不冒充「已生效于所有入口」。
//! - **不提供打卡 UI / 仪式清单**: 工具只记发生, 不展示进度、不催办、不排行。
//!
//! # 治理
//!
//! 两个工具是**执行档** (本地写, 非只读), 但风险只到本地统计文件: 装配根不注册
//! 审批钩子 (与 play 四件套不同 —— 后者推进练习状态机); 是否要求审批由适配器
//! 治理层决定。permission preset 里归入写/执行档 (`WRITE_OR_EXECUTE`), 只读档会被
//! 拒掉 (不靠工具自述「我很安全」)。
//!
//! [`ProductionModulesConfig::care_schedule`]: crate::ProductionModulesConfig::care_schedule

#![forbid(unsafe_code)]

use std::path::Path;
use std::sync::Arc;

use apeireth_core::kernel::{CapabilityId, Clock};
use apeireth_organ::ritual::{RitualKind, RitualState};
use apeireth_plugin::ToolCapability;
use apeireth_protocol::canonical::{NormalizedTool, ToolCall, ToolResult};
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{json, Value};

use super::capability::CapabilityProvider;
use super::care_reminder::{
    bio_care_candidates_enabled_from_env_value, RitualStateStore, RitualStateStoreError,
    BIO_CARE_CANDIDATES_ENABLE_ENV,
};

/// 仪式完成能力标识。
pub const RITUAL_DONE_CAPABILITY_ID: &str = "tool.ritual_done";
/// 仪式跳过能力标识。
pub const RITUAL_SKIP_CAPABILITY_ID: &str = "tool.ritual_skip";

/// 仪式完成模型侧声明名。
pub const RITUAL_DONE_TOOL_NAME: &str = "ritual_done";
/// 仪式跳过模型侧声明名。
pub const RITUAL_SKIP_TOOL_NAME: &str = "ritual_skip";

/// 两件套能力标识 (闭合词表; 只读档门与测试断言共用)。
pub const RITUAL_CAPABILITY_IDS: &[&str] = &[RITUAL_DONE_CAPABILITY_ID, RITUAL_SKIP_CAPABILITY_ID];

/// 允许的仪式种类标签 (与内核 serde 标签同值; 拒绝原因与参数枚举共用)。
#[must_use]
pub fn ritual_kind_labels() -> Vec<&'static str> {
    RitualKind::ALL.iter().map(|kind| kind.as_str()).collect()
}

/// 共享仪式完成状态 (两个工具共持**同一本**存储与注入时钟)。
///
/// 存储是文件账 (无进程内缓存): 每次调用现读现写, 重启不丢; 时钟由装配注入
/// (内核不读时钟, `now_ms` 一律调用方给)。
pub struct BioRitualState {
    store: RitualStateStore,
    clock: Arc<dyn Clock>,
}

impl BioRitualState {
    /// 绑定仪式状态存储与注入时钟。
    pub fn new(store: RitualStateStore, clock: Arc<dyn Clock>) -> Self {
        Self { store, clock }
    }

    /// 底层存储 (测试/自述只读核对口)。
    #[must_use]
    pub fn store(&self) -> &RitualStateStore {
        &self.store
    }

    /// 注入时钟 → `u64` 毫秒; 负值按 `0` 处理 (不 panic, 不倒流)。
    fn now_ms(&self) -> u64 {
        u64::try_from(self.clock.now().timestamp_millis()).unwrap_or(0)
    }

    /// 记一次完成 (时刻单调: 时钟回拨不回退)。
    fn record_done(&self, kind: RitualKind) -> Result<RitualState, RitualStateStoreError> {
        self.store.record_done(kind, self.now_ms())
    }

    /// 记一次跳过 (只加计数, 不动完成时刻与静默)。
    fn record_skip(&self, kind: RitualKind) -> Result<RitualState, RitualStateStoreError> {
        self.store.record_skip(kind)
    }
}

/// 两件套装配结果: 共享状态 + 两个模块 (每个模块贡献一个能力)。
pub struct BioRitualWiring {
    state: Arc<BioRitualState>,
    modules: Vec<Arc<dyn CapabilityProvider>>,
}

impl BioRitualWiring {
    /// 共享状态 (测试/自述只读核对口)。
    #[must_use]
    pub fn state(&self) -> &Arc<BioRitualState> {
        &self.state
    }

    /// 两个模块 (顺序: done / skip)。
    #[must_use]
    pub fn modules(&self) -> &[Arc<dyn CapabilityProvider>] {
        &self.modules
    }

    /// 消费为模块列表 (装配根注册用)。
    #[must_use]
    pub fn into_modules(self) -> Vec<Arc<dyn CapabilityProvider>> {
        self.modules
    }
}

/// 装配门 (纯函数, 测试可注入): env 原始值 = `"1"` **且** 关怀数据目录已注入时,
/// 构造两件套; 其余一律 `None` (不构造 = 不注册, 零变化)。
///
/// 缺数据目录 = 无处落盘 = **拒绝注册** (fail-closed; 不在临时目录/当前目录乱写)。
pub fn bio_ritual_wiring_from_env_value(
    raw: Option<&str>,
    care_dir: Option<&Path>,
    clock: Arc<dyn Clock>,
) -> Option<BioRitualWiring> {
    if !bio_care_candidates_enabled_from_env_value(raw) {
        return None;
    }
    let care_dir = care_dir?;
    let state = Arc::new(BioRitualState::new(
        RitualStateStore::under_data_dir(care_dir),
        clock,
    ));
    let modules: Vec<Arc<dyn CapabilityProvider>> = vec![
        Arc::new(RitualDoneModule::new(Arc::clone(&state))),
        Arc::new(RitualSkipModule::new(Arc::clone(&state))),
    ];
    Some(BioRitualWiring { state, modules })
}

/// 从装配根已解析的布尔开关装配 (R1b-1b: env 读取收敛到 `NervousSwitchboard`;
/// 本 helper 不读 env, 开关关 = `None`)。仪式完成面复用关怀候选族总闸, 由装配根
/// 以 `BioSwitch::CareCandidates` 的解析结果显式传入。
pub fn bio_ritual_wiring_from_enabled(
    enabled: bool,
    care_dir: Option<&Path>,
    clock: Arc<dyn Clock>,
) -> Option<BioRitualWiring> {
    bio_ritual_wiring_from_env_value(enabled.then_some("1"), care_dir, clock)
}

// ============================================
// 参数面
// ============================================

/// 完成面参数 (`kind` 必填; 多余字段拒绝)。
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RitualDoneParams {
    kind: String,
}

/// 跳过面参数 (`kind` 必填; 多余字段拒绝)。
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RitualSkipParams {
    kind: String,
}

fn parse_params<T>(call: &ToolCall, tool: &str) -> Result<T, ToolResult>
where
    T: serde::de::DeserializeOwned,
{
    if !call.arguments.is_object() {
        return Err(refusal(call, tool, "arguments must be a JSON object"));
    }
    serde_json::from_value(call.arguments.clone())
        .map_err(|error| refusal(call, tool, format!("invalid parameters: {error}")))
}

/// 解析仪式种类: 只认内核稳定标签 (snake_case); 未知一律结构化拒绝 (不猜、不近似匹配)。
fn resolve_kind(call: &ToolCall, tool: &str, raw: &str) -> Result<RitualKind, ToolResult> {
    let trimmed = raw.trim();
    RitualKind::ALL
        .iter()
        .copied()
        .find(|kind| kind.as_str() == trimmed)
        .ok_or_else(|| {
            refusal(
                call,
                tool,
                format!(
                    "unknown ritual kind {raw:?}; allowed: {}",
                    ritual_kind_labels().join(", ")
                ),
            )
        })
}

fn refusal(call: &ToolCall, tool: &str, message: impl Into<String>) -> ToolResult {
    let message = message.into();
    ToolResult::permanent_error(&call.id, format!("{tool}: {message}")).with_name(tool)
}

/// 存储错误 → 拒绝帧 (真因透传; 坏文件拒写, 不静默回退空表)。
fn store_refusal(call: &ToolCall, tool: &str, error: RitualStateStoreError) -> ToolResult {
    refusal(
        call,
        tool,
        format!("ritual state refused (nothing was written): {error}"),
    )
}

fn done_payload(state: &RitualState) -> Value {
    json!({
        "ritual": state.kind.as_str(),
        "label": state.kind.label(),
        "last_done_ms": state.last_done_ms,
        "consecutive_done": state.consecutive_done,
        "persisted": true,
        "memory_write": false,
        "message_sent": false,
        "effects": "local ritual statistics only (last_done_ms / consecutive_done); no message, \
                    no candidate, no scheduling change, no memory write",
    })
}

fn skip_payload(state: &RitualState) -> Value {
    json!({
        "ritual": state.kind.as_str(),
        "label": state.kind.label(),
        "skipped_count": state.skipped_count,
        "last_done_ms": state.last_done_ms,
        "persisted": true,
        "memory_write": false,
        "message_sent": false,
        "effects": "local ritual statistics only (skipped_count); the due decision and \
                    completion time are untouched",
    })
}

fn declaration_params(tool: &str, kind_hint: &str) -> Value {
    json!({
        "type": "object",
        "properties": {
            "kind": {
                "type": "string",
                "enum": ritual_kind_labels(),
                "description": kind_hint
            }
        },
        "required": ["kind"],
        "additionalProperties": false,
        "description": tool
    })
}

// ============================================
// tool.ritual_done
// ============================================

/// `tool.ritual_done`: 记一次**明确确认**的仪式完成。
pub struct RitualDoneTool {
    id: CapabilityId,
    state: Arc<BioRitualState>,
}

impl RitualDoneTool {
    /// 绑定共享状态。
    pub fn new(state: Arc<BioRitualState>) -> Self {
        Self {
            id: CapabilityId::new(RITUAL_DONE_CAPABILITY_ID)
                .expect("static ritual_done capability id is valid"),
            state,
        }
    }
}

#[async_trait]
impl ToolCapability for RitualDoneTool {
    fn id(&self) -> &CapabilityId {
        &self.id
    }

    fn declaration(&self) -> NormalizedTool {
        let params = declaration_params(
            RITUAL_DONE_TOOL_NAME,
            "Ritual kind that actually happened. Call this only when the user explicitly \
             confirms in this conversation that the ritual was done; never infer completion \
             from the assistant having proposed or sent a ritual message.",
        )
        .as_object()
        .cloned()
        .unwrap_or_default();
        NormalizedTool::new(RITUAL_DONE_TOOL_NAME)
            .with_description(
                "Record that one ritual (morning check-in / evening wrap / weekly review / \
                 anniversary / shared project) actually happened, after the user explicitly \
                 says so. Writes only local ritual statistics (last_done_ms moves forward, \
                 consecutive_done +1) into the same care data file the heartbeat reads. It \
                 sends no message, creates no candidate, changes no schedule, writes no \
                 memory, and applies no penalty or catch-up. Unknown kinds are refused; if \
                 the state file is unreadable the call fails closed and the file is left \
                 untouched.",
            )
            .with_parameters(params)
    }

    async fn invoke(&self, call: &ToolCall) -> ToolResult {
        let params: RitualDoneParams = match parse_params(call, RITUAL_DONE_TOOL_NAME) {
            Ok(params) => params,
            Err(result) => return result,
        };
        let kind = match resolve_kind(call, RITUAL_DONE_TOOL_NAME, &params.kind) {
            Ok(kind) => kind,
            Err(result) => return result,
        };
        match self.state.record_done(kind) {
            Ok(state) => {
                ToolResult::ok(&call.id, done_payload(&state)).with_name(RITUAL_DONE_TOOL_NAME)
            }
            Err(error) => store_refusal(call, RITUAL_DONE_TOOL_NAME, error),
        }
    }
}

// ============================================
// tool.ritual_skip
// ============================================

/// `tool.ritual_skip`: 记一次**明确说明**的跳过 (不是完成)。
pub struct RitualSkipTool {
    id: CapabilityId,
    state: Arc<BioRitualState>,
}

impl RitualSkipTool {
    /// 绑定共享状态。
    pub fn new(state: Arc<BioRitualState>) -> Self {
        Self {
            id: CapabilityId::new(RITUAL_SKIP_CAPABILITY_ID)
                .expect("static ritual_skip capability id is valid"),
            state,
        }
    }
}

#[async_trait]
impl ToolCapability for RitualSkipTool {
    fn id(&self) -> &CapabilityId {
        &self.id
    }

    fn declaration(&self) -> NormalizedTool {
        let params = declaration_params(
            RITUAL_SKIP_TOOL_NAME,
            "Ritual kind the user explicitly turns down this time. Call this only on an \
             explicit decline; skipping is not completing and is not a failure.",
        )
        .as_object()
        .cloned()
        .unwrap_or_default();
        NormalizedTool::new(RITUAL_SKIP_TOOL_NAME)
            .with_description(
                "Record that one ritual was explicitly skipped this time (the user said so). \
                 Writes only local ritual statistics (skipped_count +1); the last completion \
                 time, the due decision and any mute state are untouched, and there is no \
                 streak penalty, no catch-up message, no memory write. Unknown kinds are \
                 refused; an unreadable state file fails closed and is never overwritten.",
            )
            .with_parameters(params)
    }

    async fn invoke(&self, call: &ToolCall) -> ToolResult {
        let params: RitualSkipParams = match parse_params(call, RITUAL_SKIP_TOOL_NAME) {
            Ok(params) => params,
            Err(result) => return result,
        };
        let kind = match resolve_kind(call, RITUAL_SKIP_TOOL_NAME, &params.kind) {
            Ok(kind) => kind,
            Err(result) => return result,
        };
        match self.state.record_skip(kind) {
            Ok(state) => {
                ToolResult::ok(&call.id, skip_payload(&state)).with_name(RITUAL_SKIP_TOOL_NAME)
            }
            Err(error) => store_refusal(call, RITUAL_SKIP_TOOL_NAME, error),
        }
    }
}

// ============================================
// 模块包装 (每个模块贡献一个能力, 共享同一本存储)
// ============================================

/// 完成面模块 (`module.tool.ritual_done`)。
pub struct RitualDoneModule {
    tool: Arc<dyn ToolCapability>,
}

impl RitualDoneModule {
    /// 绑定共享状态。
    pub fn new(state: Arc<BioRitualState>) -> Self {
        Self {
            tool: Arc::new(RitualDoneTool::new(state)),
        }
    }

    /// 能力句柄 (测试/自述只读核对口)。
    #[must_use]
    pub fn tool(&self) -> Arc<dyn ToolCapability> {
        Arc::clone(&self.tool)
    }
}

impl CapabilityProvider for RitualDoneModule {
    fn id(&self) -> &str {
        "module.tool.ritual_done"
    }

    fn capabilities(&self) -> Vec<Arc<dyn ToolCapability>> {
        vec![Arc::clone(&self.tool)]
    }
}

/// 跳过面模块 (`module.tool.ritual_skip`)。
pub struct RitualSkipModule {
    tool: Arc<dyn ToolCapability>,
}

impl RitualSkipModule {
    /// 绑定共享状态。
    pub fn new(state: Arc<BioRitualState>) -> Self {
        Self {
            tool: Arc::new(RitualSkipTool::new(state)),
        }
    }

    /// 能力句柄 (测试/自述只读核对口)。
    #[must_use]
    pub fn tool(&self) -> Arc<dyn ToolCapability> {
        Arc::clone(&self.tool)
    }
}

impl CapabilityProvider for RitualSkipModule {
    fn id(&self) -> &str {
        "module.tool.ritual_skip"
    }

    fn capabilities(&self) -> Vec<Arc<dyn ToolCapability>> {
        vec![Arc::clone(&self.tool)]
    }
}

#[cfg(test)]
mod tests {
    use apeireth_core::clock::VirtualClock;
    use apeireth_core::kernel::Timestamp;
    use serde_json::json;

    use super::*;

    const NOW_MS: i64 = 1_700_000_000_123;

    fn clock() -> Arc<dyn Clock> {
        Arc::new(VirtualClock::new(
            Timestamp::from_epoch_millis(NOW_MS)
                .expect("valid fixed timestamp")
                .as_datetime(),
        ))
    }

    fn call(tool: &str, arguments: Value) -> ToolCall {
        ToolCall {
            id: format!("call-{tool}"),
            name: tool.to_string(),
            arguments,
        }
    }

    fn ok_value(frame: &ToolResult) -> Value {
        match &frame.outcome {
            apeireth_protocol::canonical::ToolOutcome::Ok { value } => value.clone(),
            other => panic!("expected an ok frame, got {other:?}"),
        }
    }

    fn done_tool(wiring: &BioRitualWiring) -> Arc<dyn ToolCapability> {
        Arc::clone(&wiring.modules()[0].capabilities()[0])
    }

    fn skip_tool(wiring: &BioRitualWiring) -> Arc<dyn ToolCapability> {
        Arc::clone(&wiring.modules()[1].capabilities()[0])
    }

    #[test]
    fn the_env_switch_is_fail_closed_and_requires_a_data_dir() {
        let dir = std::env::temp_dir();
        assert!(bio_ritual_wiring_from_env_value(Some("1"), Some(&dir), clock()).is_some());
        for closed in [
            None,
            Some(""),
            Some("1 "),
            Some(" 1"),
            Some("0"),
            Some("true"),
            Some("yes"),
            Some("on"),
        ] {
            assert!(
                bio_ritual_wiring_from_env_value(closed, Some(&dir), clock()).is_none(),
                "{closed:?} 必须关 (fail-closed; 只认精确 \"1\")"
            );
        }
        assert!(
            bio_ritual_wiring_from_env_value(Some("1"), None, clock()).is_none(),
            "缺数据目录 = 无处落盘 = 拒绝注册"
        );
    }

    #[test]
    fn the_declarations_are_model_facing_and_the_kind_enum_is_closed() {
        let dir = tempfile::tempdir().expect("tempdir");
        let wiring = bio_ritual_wiring_from_env_value(Some("1"), Some(dir.path()), clock())
            .expect("env 开 + 目录注入必须构造两件套");
        assert_eq!(wiring.modules().len(), 2);

        let done = done_tool(&wiring);
        assert_eq!(done.id().as_str(), RITUAL_DONE_CAPABILITY_ID);
        let declaration = done.declaration();
        assert_eq!(declaration.name, RITUAL_DONE_TOOL_NAME);
        let params = Value::Object(declaration.parameters);
        let allowed: Vec<&str> = params["properties"]["kind"]["enum"]
            .as_array()
            .expect("kind 枚举")
            .iter()
            .map(|value| value.as_str().expect("enum 是字符串"))
            .collect();
        assert_eq!(allowed, ritual_kind_labels());
        assert_eq!(params["additionalProperties"], json!(false));

        let skip = skip_tool(&wiring);
        assert_eq!(skip.id().as_str(), RITUAL_SKIP_CAPABILITY_ID);
        assert_eq!(skip.declaration().name, RITUAL_SKIP_TOOL_NAME);
        assert!(!dir.path().join("ritual_state.json").exists(), "声明不落盘");
    }

    #[tokio::test]
    async fn done_then_skip_write_only_statistics_to_the_care_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let wiring =
            bio_ritual_wiring_from_env_value(Some("1"), Some(dir.path()), clock()).expect("两件套");
        let file = dir.path().join("ritual_state.json");
        assert!(!file.exists(), "构造不创建文件");

        let frame = done_tool(&wiring)
            .invoke(&call(
                RITUAL_DONE_TOOL_NAME,
                json!({"kind": "morning_check_in"}),
            ))
            .await;
        assert!(frame.is_ok(), "记完成必须成功: {frame:?}");
        let body = ok_value(&frame);
        assert_eq!(body["ritual"], "morning_check_in");
        assert_eq!(body["last_done_ms"], NOW_MS as u64);
        assert_eq!(body["consecutive_done"], 1);
        assert_eq!(body["persisted"], true);
        assert_eq!(body["memory_write"], false);
        assert_eq!(body["message_sent"], false);

        // 同一 kind 第二次完成: 计数 +1, 时刻由时钟现读。
        done_tool(&wiring)
            .invoke(&call(
                RITUAL_DONE_TOOL_NAME,
                json!({"kind": "morning_check_in"}),
            ))
            .await;
        let again = done_tool(&wiring)
            .invoke(&call(
                RITUAL_DONE_TOOL_NAME,
                json!({"kind": "morning_check_in"}),
            ))
            .await;
        assert_eq!(ok_value(&again)["consecutive_done"], 3);

        // 跳过只加计数, 不动完成时刻。
        let skipped = skip_tool(&wiring)
            .invoke(&call(
                RITUAL_SKIP_TOOL_NAME,
                json!({"kind": "evening_wrap"}),
            ))
            .await;
        assert!(skipped.is_ok(), "记跳过必须成功: {skipped:?}");
        let body = ok_value(&skipped);
        assert_eq!(body["skipped_count"], 1);
        assert_eq!(body["last_done_ms"], 0, "跳过不是完成, 完成时刻不动");

        // 新实例读同一文件 = 一致 (路径即真源, 无内存缓存)。
        let reopened = RitualStateStore::under_data_dir(dir.path());
        let (config, states) = reopened.inputs().expect("装载");
        assert!(config.schedules.is_empty(), "不推断默认排程");
        assert_eq!(states.len(), 2);
    }

    #[tokio::test]
    async fn unknown_kind_is_refused_and_writes_nothing() {
        let dir = tempfile::tempdir().expect("tempdir");
        let wiring =
            bio_ritual_wiring_from_env_value(Some("1"), Some(dir.path()), clock()).expect("两件套");
        let file = dir.path().join("ritual_state.json");

        for (tool, arguments) in [
            (RITUAL_DONE_TOOL_NAME, json!({"kind": "morning"})),
            (RITUAL_SKIP_TOOL_NAME, json!({"kind": 1})),
            (RITUAL_DONE_TOOL_NAME, json!({})),
            (
                RITUAL_DONE_TOOL_NAME,
                json!({"kind": "morning_check_in", "extra": true}),
            ),
        ] {
            let expected = if tool == RITUAL_DONE_TOOL_NAME {
                done_tool(&wiring)
            } else {
                skip_tool(&wiring)
            };
            let frame = expected.invoke(&call(tool, arguments.clone())).await;
            assert!(!frame.is_ok(), "{tool} {arguments} 必须拒绝: {frame:?}");
        }
        assert!(!file.exists(), "拒绝不得创建/写文件");
    }

    #[tokio::test]
    async fn a_corrupt_file_fails_closed_and_is_never_overwritten() {
        let dir = tempfile::tempdir().expect("tempdir");
        let wiring =
            bio_ritual_wiring_from_env_value(Some("1"), Some(dir.path()), clock()).expect("两件套");
        let file = dir.path().join("ritual_state.json");
        std::fs::write(&file, "{ not json").expect("写坏文件");

        let frame = done_tool(&wiring)
            .invoke(&call(
                RITUAL_DONE_TOOL_NAME,
                json!({"kind": "weekly_review"}),
            ))
            .await;
        assert!(!frame.is_ok(), "坏文件必须拒绝写回");
        assert_eq!(
            std::fs::read_to_string(&file).expect("读文件"),
            "{ not json",
            "拒绝写入 = 原文件逐字节不变"
        );
    }
}
