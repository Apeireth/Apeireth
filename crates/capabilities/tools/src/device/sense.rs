//! 感知工具面 (`tool.device_sense`, 声明名 `device_sense`) —— K 轴 L1 在场感知。
//!
//! # 白名单制读面 (四条安全底线之①)
//!
//! 只读**白名单设备**的状态 ([`DeviceRegistry`] = `devices.json` StoredDoc 拒开,
//! 默认零设备): 表外 / 停用设备一律拒绝即帧, 词表外适配器拒绝即帧。适配器
//! 插槽图见 [`crate::device::adapter`] (mock 已接固定 fixture, 真实协议族留
//! 插槽不接)。
//!
//! # 敏感面: 摄像头类 = 宿主拍板 + 审计
//!
//! 摄像头类 (谁在家类目) 是**敏感读面**: 每次感知尝试都落审计账
//! ([`crate::device::audit`], append-only); 放行要**宿主拍板** —— 模型面
//! **无权提供授权人** (授权旗 `sensitive_read_authorized` 只是模型的*请求*,
//! 真正放行看**宿主决定账**的批准记录, 见 [`crate::device::approval`]),
//! 缺拍板 = 拒绝即帧 + 审计行 `denied`。审计账写不进 = 不放行 (fail-closed)。
//!
//! **同族缺陷 (2026-11-01 安全审计)**: 原实现把 `authorized_by` 直接取自
//! **模型参数** ⇒ 模型可以自称「已获授权」从而读摄像头, 且审计账里的授权人
//! 不可信。现由 [`DeviceSenseApprovalHook`] (宿主审批门, 与执行面
//! `DeviceControlApprovalHook` 同形态) 承载宿主人批, 工具面只认决定账里的
//! 宿主署名。
//!
//! # 治理 (风险档 = 低-中, 读面)
//!
//! 权力面 = **只读 + append 敏感读审计账**: 零执行、零外联、零设备控制
//! (L3 受控执行不在本波)。注册即闸 (`APEIRETH_ENABLE_DEVICE_SENSE`, 默认关
//! fail-closed, 见 [`DeviceSenseConfig::from_env`]), grant 即用 (读面)。
//!
//! # 自述随行
//!
//! 在场节 (设备数 / 白名单数 / 最近感知) 随 `tool.self_status` 的 `presence`
//! 节上报 ([`device_presence_status`] → [`crate::self_status::DevicePresenceStatus`]),
//! 与工具面同一本设备表同一进程内感知计数。

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use apeireth_core::kernel::CapabilityId;
use apeireth_governance::{
    Action, Decision, GovernanceHook, GovernanceRequest, ToolGuard, ToolGuardRequest,
};
use apeireth_plugin::ToolCapability;
use apeireth_protocol::canonical::{NormalizedTool, ToolCall, ToolParameters, ToolResult};
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{json, Value};

use super::adapter::{AdapterRegistry, DeviceState};
use super::approval::{host_approval, DeviceApprovalDecisionLedger};
use super::audit::{DeviceSenseAuditLedger, SensitiveReadAudit, SensitiveReadOutcome};
use super::registry::{Device, DeviceKind, DeviceRegistry, DevicesBody};
use crate::exec_pipeline::{AroundPolicy, OutputSchema, SchemaKind, DEFAULT_MAX_TIMEOUT_MS};
use crate::self_status::DevicePresenceStatus;

/// 稳定能力标识 (注册表 / 治理授权 / 风险映射共用)。
pub const DEVICE_SENSE_CAPABILITY_ID: &str = "tool.device_sense";

/// 模型侧声明名 (短名, 每次请求都进上下文)。
pub const DEVICE_SENSE_TOOL_NAME: &str = "device_sense";

/// 生产执行时限 (毫秒): 感知是设备表读 + 适配器读 + 审计追加, 到期即归
/// `timeout.*`。
pub const DEVICE_SENSE_TIMEOUT_MS: u64 = 5_000;

/// 在场感知面总开关 (`APEIRETH_ENABLE_DEVICE_SENSE`): 注册即闸, 默认关
/// fail-closed。
///
/// 只认显式 `"1"` (前后空白容忍); 未设 / `"0"` / 其它任何取值 = 关。本开关是
/// K 轴 L1 **在场感知族**的一闸: `tool.device_sense` 与 `tool.weather`
/// (同属 L1 感知: 温度/天气/谁在家) 同族注册即闸, 不另立旋钮。
pub const DEVICE_SENSE_ENABLE_ENV: &str = "APEIRETH_ENABLE_DEVICE_SENSE";

/// 动作: 读白名单视图 (零适配器读)。
pub const ACTION_LIST: &str = "list";
/// 动作: 感知白名单设备状态 (摄像头类要显式授权 + 审计)。
pub const ACTION_SENSE: &str = "sense";

/// 闭合动作词表 (list / sense)。
pub const DEVICE_SENSE_ACTIONS: &[&str] = &[ACTION_LIST, ACTION_SENSE];

/// 调用参数 (未知字段拒绝)。
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct DeviceSenseParams {
    action: Option<String>,
    device_ids: Option<Vec<String>>,
    /// 模型的**请求**旗 (摄像头类敏感读要显式请求) —— **不是授权**: 真正的
    /// 放行看宿主决定账的批准记录 (模型的授权旗/授权人一律不作数)。
    sensitive_read_authorized: Option<bool>,
    /// 模型自称的授权人 —— **一律不作数** (2026-11-01 安全审计: 原实现直接
    /// 取这个模型参数当授权人 ⇒ 模型可自称「已获授权」读摄像头, 且审计账里的
    /// 授权人不可信)。保留解析只为旧提示词不硬失败; 真授权只认宿主决定账。
    authorized_by: Option<String>,
}

/// 当前时刻 (epoch 毫秒); 时钟回拨不编造, 如实取系统读数。
fn now_epoch_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or(0)
}

/// 在场感知的进程内计数 (自述在场节「最近感知」的真值源)。
///
/// **进程内计数**: 本进程生命周期内的感知次数/台数/敏感面授权计数, 重启清零,
/// 不落账不造账 (0 装诚实, 不拿磁盘账冒充) —— 与创作节
/// ([`crate::self_status::ImageOpsStatus`]) 同口径。
#[derive(Debug, Default)]
pub struct DeviceSenseActivity {
    sense_calls: AtomicU64,
    sensed_devices: AtomicU64,
    sensitive_granted: AtomicU64,
    sensitive_denied: AtomicU64,
    last: Mutex<Option<(i64, String)>>,
}

impl DeviceSenseActivity {
    /// 计数器 (进程内)。
    pub fn new() -> Self {
        Self::default()
    }

    /// 记一次感知 (时刻 + 有界摘要)。
    pub fn record_sense(&self, at_ms: i64, note: impl Into<String>) {
        self.sense_calls.fetch_add(1, Ordering::Relaxed);
        let mut last = self
            .last
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        *last = Some((at_ms, note.into()));
    }

    /// 记一次敏感读面判定 (true = 显式授权放行)。
    pub fn record_sensitive(&self, granted: bool) {
        if granted {
            self.sensitive_granted.fetch_add(1, Ordering::Relaxed);
        } else {
            self.sensitive_denied.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// 感知台数累计 (进程内)。
    pub fn sensed_devices(&self) -> u64 {
        self.sensed_devices.load(Ordering::Relaxed)
    }

    /// 感知调用累计 (进程内)。
    pub fn sense_calls(&self) -> u64 {
        self.sense_calls.load(Ordering::Relaxed)
    }

    /// 敏感读面放行/拒绝累计 (进程内)。
    pub fn sensitive_counts(&self) -> (u64, u64) {
        (
            self.sensitive_granted.load(Ordering::Relaxed),
            self.sensitive_denied.load(Ordering::Relaxed),
        )
    }

    /// 最近感知 (时刻 + 摘要; `None` = 尚无感知)。
    pub fn last_sense(&self) -> Option<(i64, String)> {
        self.last
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }
}

/// 在场节真值 (设备数 / 白名单数 / 最近感知), 与工具面同表同计数。
///
/// 设备表拒开 = `Err(真因)` (自述节显式 null + 原因, 不编数)。
pub fn device_presence_status(
    registry: &DeviceRegistry,
    activity: &DeviceSenseActivity,
) -> Result<DevicePresenceStatus, String> {
    let body = registry.load().map_err(|error| error.to_string())?;
    // 跨线机械补位 (device_control 批报备): `last_sense()` 回 `Option<(i64, String)>`,
    // 本节字段是 `Option` 对 —— 无记录 = 双 None (不编时间不编备注), 语义不变。
    let (last_sense_at_ms, last_sense_note) = match activity.last_sense() {
        Some((at_ms, note)) => (Some(at_ms), Some(note)),
        None => (None, None),
    };
    let table_present = registry.path().exists();
    let mut note = if table_present {
        "设备表已载入 (devices.json)".to_string()
    } else {
        "devices.json 缺省: 白名单默认零设备 (显式登记 + 启用才可感知)".to_string()
    };
    note.push_str("; 最近感知为进程内计数 (重启清零)");
    Ok(DevicePresenceStatus {
        devices: Some(body.len() as u64),
        whitelisted: Some(body.whitelist().len() as u64),
        last_sense_at_ms,
        last_sense_note,
        note: Some(note),
        reason: None,
    })
}

/// 设备感知工具 (`tool.device_sense`, 声明名 `device_sense`)。
///
/// 读面零执行: `list` 只回白名单视图, `sense` 走适配器读状态 (白名单 +
/// 敏感面宿主拍板 + 审计)。
pub struct DeviceSenseTool {
    id: CapabilityId,
    registry: DeviceRegistry,
    adapters: AdapterRegistry,
    audit: DeviceSenseAuditLedger,
    /// 宿主决定账 (敏感读的「谁批」唯一来源; 模型参数不作数)。
    decisions: DeviceApprovalDecisionLedger,
    activity: Arc<DeviceSenseActivity>,
}

impl DeviceSenseTool {
    /// 绑定设备表 / 适配器注册表 / 敏感读审计账 / 宿主决定账 / 进程内计数。
    pub fn new(
        registry: DeviceRegistry,
        adapters: AdapterRegistry,
        audit: DeviceSenseAuditLedger,
        decisions: DeviceApprovalDecisionLedger,
        activity: Arc<DeviceSenseActivity>,
    ) -> Self {
        Self {
            id: CapabilityId::new(DEVICE_SENSE_CAPABILITY_ID).expect("valid capability id"),
            registry,
            adapters,
            audit,
            decisions,
            activity,
        }
    }

    /// 输出归一合同: 结构化输出必须是对象且 `action` / `status` 恒在场。
    pub fn output_schema() -> OutputSchema {
        OutputSchema::new()
            .require("action", SchemaKind::String)
            .require("status", SchemaKind::String)
    }

    /// 进程内感知计数 (自述在场节同源同账)。
    pub fn activity(&self) -> Arc<DeviceSenseActivity> {
        Arc::clone(&self.activity)
    }

    /// 设备表 (自述在场节同表同源)。
    pub fn registry(&self) -> &DeviceRegistry {
        &self.registry
    }

    fn list(&self, call: &ToolCall) -> ToolResult {
        let body = match self.registry.load() {
            Ok(body) => body,
            Err(error) => return self.refuse(call, format!("设备表拒开 (不回退默认表): {error}")),
        };
        let rows: Vec<Value> = body
            .devices
            .iter()
            .map(|device| {
                json!({
                    "id": device.id,
                    "kind": device.kind.as_str(),
                    "name": device.name,
                    "adapter": device.adapter,
                    "enabled": device.enabled,
                    "sensitive": device.kind.is_sensitive(),
                })
            })
            .collect();
        let whitelisted = body.whitelist().len();
        ToolResult::ok(
            &call.id,
            json!({
                "action": ACTION_LIST,
                "status": "ok",
                "devices": rows,
                "devices_total": body.len(),
                "whitelisted": whitelisted,
                "table_present": self.registry.path().exists(),
                "note": "白名单制: enabled=true 的设备才可感知; 默认零设备 (devices.json 缺省 = 空表)",
            }),
        )
        .with_name(DEVICE_SENSE_TOOL_NAME)
    }

    fn sense(&self, call: &ToolCall, params: &DeviceSenseParams) -> ToolResult {
        let body = match self.registry.load() {
            Ok(body) => body,
            Err(error) => return self.refuse(call, format!("设备表拒开 (不回退默认表): {error}")),
        };
        let targets = match select_targets(&body, params.device_ids.as_deref()) {
            Ok(targets) => targets,
            Err(refusal) => return self.refuse(call, refusal),
        };

        // 敏感面 (摄像头类): 宿主拍板 + 审计 —— 每次尝试都入账, 写不进 = 不放行。
        let sensitive_targets: Vec<&Device> = targets
            .iter()
            .copied()
            .filter(|device| device.kind.is_sensitive())
            .collect();
        if !sensitive_targets.is_empty() {
            // **原先错在哪**: 授权人直接取模型参数 (`authorized_by`) ⇒ 模型自称
            // 「已获授权」即可读摄像头, 审计账里的授权人也不可信。
            // **现在为什么这样**: 放行只认**宿主写的决定账** (宿主拍板记录);
            // 模型自称的授权人/授权旗一律不作数 (不采信、不落账) —— 审计账里
            // 的 `authorized_by` 是宿主的署名 (决定账 `decided_by` 原样)。
            let plan_id = sense_plan_id(&sensitive_targets);
            let host = host_approval(&self.decisions, None, &plan_id, false);
            let (outcome, reason, authorizer) = match &host {
                Ok(approval) => (
                    SensitiveReadOutcome::Granted,
                    format!("宿主已拍板 (决定账批准记录: {})", approval.approved_by),
                    Some(approval.approved_by.clone()),
                ),
                Err(error) => (
                    SensitiveReadOutcome::Denied,
                    format!("摄像头类是敏感读面: {error}"),
                    None,
                ),
            };
            // 模型的请求旗仍是必要条件 (模型得显式要读摄像头类): 但它**不是**
            // 授权 —— 授权只认宿主决定账 (上面那一步)。
            let granted = outcome == SensitiveReadOutcome::Granted
                && params.sensitive_read_authorized == Some(true);
            let (outcome, reason, authorizer) = if granted {
                (outcome, reason, authorizer)
            } else {
                (
                    SensitiveReadOutcome::Denied,
                    format!("{reason}; 且模型未显式请求敏感读 (sensitive_read_authorized=true)"),
                    None,
                )
            };
            let at_ms = now_epoch_ms();
            for device in &sensitive_targets {
                let row = SensitiveReadAudit {
                    device_id: device.id.clone(),
                    kind: device.kind,
                    action: ACTION_SENSE.to_string(),
                    authorized_by: authorizer.clone(),
                    outcome,
                    reason: reason.clone(),
                    at_ms,
                };
                if let Err(error) = self.audit.append(&row) {
                    // 审计账写不进 = 不放行 (fail-closed: 敏感读必须留痕)。
                    return self.refuse(
                        call,
                        format!("拒绝感知 (审计缺位不放行): 敏感读面审计入账失败: {error}"),
                    );
                }
            }
            self.activity.record_sensitive(granted);
            if !granted {
                return self.refuse(
                    call,
                    format!(
                        "拒绝感知: {reason}; 本次尝试已入审计账 ({})",
                        self.audit.path().display()
                    ),
                );
            }
        }

        // 适配器读 (白名单设备逐台): 插槽未接线 / 读失败 = 整帧拒绝即帧 (真因
        // 如实, 不静默跳台不降级)。
        let mut states: Vec<DeviceState> = Vec::new();
        for device in &targets {
            let adapter = match self.adapters.resolve(&device.adapter) {
                Ok(adapter) => adapter,
                Err(error) => {
                    return self.refuse(
                        call,
                        format!("拒绝感知: 设备 {} 的适配器不可用: {error}", device.id),
                    );
                }
            };
            match adapter.read_state(device, now_epoch_ms()) {
                Ok(state) => states.push(state),
                Err(error) => {
                    return self.refuse(
                        call,
                        format!("拒绝感知: 设备 {} 读失败: {error}", device.id),
                    );
                }
            }
        }

        let note = if states.is_empty() {
            "白名单零设备: 无设备可感知 (默认空态, 不编造读数)".to_string()
        } else {
            format!(
                "{} 台白名单设备已感知 (mock 档 = 固定 fixture 数据, 非实测世界)",
                states.len()
            )
        };
        let at_ms = now_epoch_ms();
        self.activity.record_sense(at_ms, note.clone());
        for _ in &states {
            self.activity.sensed_devices.fetch_add(1, Ordering::Relaxed);
        }
        let (sensitive_granted, sensitive_denied) = self.activity.sensitive_counts();
        ToolResult::ok(
            &call.id,
            json!({
                "action": ACTION_SENSE,
                "status": "ok",
                "states": states,
                "whitelisted": targets.len(),
                "sensitive_audit": {
                    "granted_total": sensitive_granted,
                    "denied_total": sensitive_denied,
                    "ledger": self.audit.path().to_string_lossy(),
                },
                "note": note,
            }),
        )
        .with_name(DEVICE_SENSE_TOOL_NAME)
    }

    /// 拒绝即帧 (真因如实, 不静默降级)。
    fn refuse(&self, call: &ToolCall, reason: impl Into<String>) -> ToolResult {
        ToolResult::permanent_error(&call.id, reason.into()).with_name(DEVICE_SENSE_TOOL_NAME)
    }
}

/// 敏感读计划身份 (宿主拍板的闭合身份): 同一批敏感设备的一次读 = 一个身份。
///
/// 与动作面的 `plan_id_for` 同口径 (fnv1a64 规范摘要) ⇒ 宿主批的是「这一批
/// 设备的这一次读」; 换一批设备 = 换身份 (不得借批)。
pub fn sense_plan_id(devices: &[&Device]) -> String {
    let mut ids: Vec<&str> = devices.iter().map(|device| device.id.as_str()).collect();
    ids.sort_unstable();
    let canonical = format!("device_sense\nsense\n{}", ids.join("\n"));
    format!(
        "{:016x}",
        crate::worktree_plan::fnv1a64(canonical.as_bytes())
    )
}

/// 目标选择 (纯判定): 白名单制 —— 默认全白名单; 指定 id 时逐台核白名单。
///
/// 表外 / 停用设备 = `Err(拒绝即帧文案)` (底线①: 显式登记 + 启用才可感知)。
fn select_targets<'a>(
    body: &'a DevicesBody,
    device_ids: Option<&[String]>,
) -> Result<Vec<&'a Device>, String> {
    match device_ids {
        None => Ok(body.whitelist()),
        Some(ids) => {
            if ids.is_empty() {
                return Err("拒绝感知: device_ids 给了空列表 (要么省略读全白名单, \
                    要么给非空 id 列表)"
                    .to_string());
            }
            let mut targets = Vec::new();
            for id in ids {
                match body.find(id) {
                    None => {
                        return Err(format!(
                            "拒绝感知: 设备 {id} 不在设备表上 (白名单制: 默认零设备, \
                             显式登记 + 启用才可感知)"
                        ));
                    }
                    Some(device) if !device.enabled => {
                        return Err(format!(
                            "拒绝感知: 设备 {id} 已登记但未启用 (白名单外, enabled=false)"
                        ));
                    }
                    Some(device) => targets.push(device),
                }
            }
            Ok(targets)
        }
    }
}

#[async_trait]
impl ToolCapability for DeviceSenseTool {
    fn id(&self) -> &CapabilityId {
        &self.id
    }

    fn declaration(&self) -> NormalizedTool {
        let mut parameters = ToolParameters::new();
        parameters.extend(
            json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["list", "sense"],
                        "description": "list reads the whitelist view (no device read); sense reads the state of whitelisted devices through their adapters."
                    },
                    "device_ids": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "sense only: specific whitelisted device ids; omit to sense every whitelisted device. Off-list or disabled ids are refused."
                    },
                    "sensitive_read_authorized": {
                        "type": "boolean",
                        "description": "sense of camera-class devices only: the model's REQUEST to read a camera-class device. This flag is NOT an authorization: the host must have approved the read (host decision ledger), otherwise the read is refused and the attempt lands in the audit ledger. You cannot name the authorizer - the host owns that."
                    }
                },
                "required": ["action"],
                "additionalProperties": false
            })
            .as_object()
            .cloned()
            .unwrap_or_default(),
        );
        NormalizedTool::new(DEVICE_SENSE_TOOL_NAME)
            .with_description(
                "Read the state of whitelisted physical devices (K-axis L1 presence sensing). \
                 The device table (devices.json) is a strict stored document with zero devices \
                 by default: only explicitly registered AND enabled devices can be sensed, and \
                 off-list ids are refused. The mock adapter returns fixed fixture data and says \
                 so in source_note; real protocol adapters (local_bridge / cloud_bridge / \
                 direct) are reserved slots that are honestly not wired yet. Camera-class \
                 devices are a sensitive read face: each attempt is audited (append-only \
                 device-sense-audit.jsonl) and a read needs the HOST's approval (a host decision \
                 record) - you cannot grant it yourself and you cannot name the authorizer. \
                 This face only senses - it never controls a device.",
            )
            .with_parameters(parameters)
    }

    async fn invoke(&self, call: &ToolCall) -> ToolResult {
        let arguments = match &call.arguments {
            Value::Null => json!({}),
            other => other.clone(),
        };
        let params = match serde_json::from_value::<DeviceSenseParams>(arguments) {
            Ok(params) => params,
            Err(error) => {
                return self.refuse(call, format!("invalid device_sense parameters: {error}"));
            }
        };
        let Some(action) = params.action.as_deref() else {
            return self.refuse(call, "拒绝感知: 必须显式给 action (list / sense)");
        };
        match action {
            ACTION_LIST => self.list(call),
            ACTION_SENSE => self.sense(call, &params),
            other => self.refuse(
                call,
                format!(
                    "unknown device_sense action {other:?}; the closed vocabulary is \
                     list / sense"
                ),
            ),
        }
    }
}

/// 单调边界 Guard (只拒不放): 模型自称授权的**诚实性** —— 开了授权旗却给不出
/// 授权人 = 拒 (任何后续阶段都不能翻回放行)。
///
/// **口径说明 (2026-11-01 安全审计)**: 模型的授权旗/授权人**一律不作数**
/// (放行只认宿主决定账, 见 [`host_approval`]); 本 Guard 只拦最不诚实的一档
/// (开了旗却给不出授权人), 免得审计账里出现无主的授权声明。真正的放行判定
/// 在工具面 (宿主拍板 + 审计留痕)。
pub struct DeviceSenseBoundaryGuard;

impl DeviceSenseBoundaryGuard {
    /// 感知面边界 Guard (无状态)。
    pub fn new() -> Self {
        Self
    }
}

impl Default for DeviceSenseBoundaryGuard {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolGuard for DeviceSenseBoundaryGuard {
    fn name(&self) -> &str {
        "device_sense_boundary"
    }

    fn deny(&self, request: &ToolGuardRequest<'_>) -> Option<String> {
        if request.capability.as_str() != DEVICE_SENSE_CAPABILITY_ID {
            return None;
        }
        // 解析失败不归边界 Guard (由工具即帧报解析错误), 只判授权参数诚实性。
        let Ok(params) = serde_json::from_value::<DeviceSenseParams>(request.arguments.clone())
        else {
            return None;
        };
        if params.action.as_deref() == Some(ACTION_SENSE)
            && params.sensitive_read_authorized == Some(true)
            && params
                .authorized_by
                .as_deref()
                .map(str::trim)
                .unwrap_or_default()
                .is_empty()
        {
            return Some(
                "拒绝感知: sensitive_read_authorized=true 必须配非空 authorized_by \
                 (模型自称的授权人不作数, 放行只认宿主拍板)"
                    .to_string(),
            );
        }
        None
    }
}

/// 敏感读面的**宿主审批门** (治理链, 与执行面 `DeviceControlApprovalHook`
/// 同形态): 摄像头类敏感读 = [`Decision::require_approval`] (宿主人批, 无
/// 自动放行档); 其余读面 (白名单视图 / 非敏感设备) 放行。
///
/// **原先错在哪**: 敏感读没有任何宿主审批门 —— 授权人直接取模型参数, 模型
/// 自称「已获授权」即可读摄像头 (注册即 grant, 零审批)。
/// **现在为什么这样**: 敏感读先停宿主人批 (无自动档), 且真正的放行判定在
/// 工具面查**宿主决定账** (模型的授权旗/授权人一律不作数)。
#[derive(Debug, Clone)]
pub struct DeviceSenseApprovalHook {
    registry: DeviceRegistry,
}

impl DeviceSenseApprovalHook {
    /// 绑定设备表 (判定本次读是否命中摄像头类敏感面)。
    pub fn new(registry: DeviceRegistry) -> Self {
        Self { registry }
    }

    /// 数据目录落位 (设备表 = `<data_dir>/devices.json`, 与感知面同一本)。
    pub fn under_data_dir(data_dir: impl AsRef<std::path::Path>) -> Self {
        Self::new(DeviceRegistry::under_data_dir(data_dir))
    }
}

#[async_trait]
impl GovernanceHook for DeviceSenseApprovalHook {
    fn name(&self) -> &str {
        "device_sense_sensitive_read"
    }

    async fn evaluate(&self, request: &GovernanceRequest<'_>) -> Decision {
        let Action::CapabilityDispatch {
            capability,
            arguments,
        } = &request.action
        else {
            return Decision::Allow;
        };
        if capability.as_str() != DEVICE_SENSE_CAPABILITY_ID {
            return Decision::Allow;
        }
        let verb = arguments
            .get("action")
            .and_then(Value::as_str)
            .unwrap_or_default();
        match verb {
            // 白名单视图读 = 零副作用读面: 零审批。
            ACTION_LIST => Decision::Allow,
            ACTION_SENSE => {
                // 设备表拒开 = fail-closed (不放行未知面)。
                let body = match self.registry.load() {
                    Ok(body) => body,
                    Err(error) => {
                        return Decision::deny(format!(
                            "设备表拒开, 拒绝感知 (fail-closed, 不回退默认表): {error}"
                        ))
                    }
                };
                let device_ids = arguments
                    .get("device_ids")
                    .and_then(Value::as_array)
                    .map(|ids| {
                        ids.iter()
                            .filter_map(Value::as_str)
                            .map(str::to_string)
                            .collect::<Vec<String>>()
                    });
                let targets = match select_targets(&body, device_ids.as_deref()) {
                    Ok(targets) => targets,
                    Err(refusal) => return Decision::deny(refusal),
                };
                let sensitive: Vec<&str> = targets
                    .iter()
                    .filter(|device| device.kind.is_sensitive())
                    .map(|device| device.id.as_str())
                    .collect();
                if sensitive.is_empty() {
                    return Decision::Allow;
                }
                Decision::require_approval(format!(
                    "摄像头类是敏感读面 (设备: {}): 宿主必须拍板才放行 \
                     (工具面查宿主决定账的批准记录; 模型的授权旗/授权人不作数)",
                    sensitive.join(", ")
                ))
            }
            other => Decision::deny(format!(
                "设备感知拍 {other:?} 不在闭合拍词表 (list/sense): fail-closed 拒绝即帧"
            )),
        }
    }
}

/// 组装感知面五段流水线: 读面 (低风险档) → 单调边界 Guard → 超时 → 输出归一合同。
pub fn device_sense_pipeline() -> Arc<crate::exec_pipeline::ToolExecutionPipeline> {
    Arc::new(
        crate::exec_pipeline::ToolExecutionPipeline::new()
            .with_guard(Arc::new(DeviceSenseBoundaryGuard::new()))
            .with_around(
                AroundPolicy::new().with_timeout(DEVICE_SENSE_TIMEOUT_MS, DEFAULT_MAX_TIMEOUT_MS),
            )
            .with_output_schema(DeviceSenseTool::output_schema()),
    )
}

/// 组装设备感知工具面 (模型侧一个能力)。
pub fn device_sense_capability(
    config: DeviceSenseConfig,
    activity: Arc<DeviceSenseActivity>,
) -> Arc<dyn ToolCapability> {
    let tool: Arc<dyn ToolCapability> = Arc::new(DeviceSenseTool::new(
        config.registry(),
        AdapterRegistry::builtin(),
        config.audit(),
        config.decisions(),
        activity,
    ));
    Arc::new(crate::exec_pipeline::PipelinedCapability::new(
        tool,
        device_sense_pipeline(),
    ))
}

// ---------------------------------------------------------------------------
// 开关 (注册即闸, 默认关 fail-closed)
// ---------------------------------------------------------------------------

/// 在场感知面配置 (设备表 + 敏感读审计账落位)。
///
/// 组装根按 [`DeviceSenseConfig::from_env`] 注入 (注册即闸): 缺配置 =
/// `tool.device_sense` / `tool.weather` 都不注册, 自述名册 `device_sense` /
/// `weather` 行照实 false。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceSenseConfig {
    /// 设备表与审计账所在数据目录 (`<data_dir>/devices.json` 等)。
    pub data_dir: PathBuf,
}

impl DeviceSenseConfig {
    /// 落位 = 数据目录。
    pub fn new(data_dir: impl Into<PathBuf>) -> Self {
        Self {
            data_dir: data_dir.into(),
        }
    }

    /// 本配置的设备表。
    pub fn registry(&self) -> DeviceRegistry {
        DeviceRegistry::under_data_dir(&self.data_dir)
    }

    /// 本配置的敏感读审计账。
    pub fn audit(&self) -> DeviceSenseAuditLedger {
        DeviceSenseAuditLedger::under_data_dir(&self.data_dir)
    }

    /// 本配置的**宿主决定账** (敏感读放行的「谁批」唯一来源)。
    ///
    /// 与执行面的审批决定账同一本 (`device-approval-decisions.jsonl`, 同数据
    /// 目录) ⇒ 两个设备面 (读面/执行面) 的「谁批」同源同账。
    pub fn decisions(&self) -> DeviceApprovalDecisionLedger {
        DeviceApprovalDecisionLedger::under_data_dir(&self.data_dir)
    }

    /// 开关实况: `APEIRETH_ENABLE_DEVICE_SENSE` 只认显式 `"1"` (默认关 fail-closed)。
    pub fn enabled_from_env() -> bool {
        device_sense_enabled_from_env()
    }

    /// 从 env 构造: 开关开才 `Some` (默认关 → [`None`], fail-closed)。
    pub fn from_env(data_dir: impl Into<PathBuf>) -> Option<Self> {
        device_sense_enabled_from_env().then(|| Self::new(data_dir))
    }
}

/// 在场感知面开关 (`APEIRETH_ENABLE_DEVICE_SENSE`) 实况: 只认显式 `"1"`。
///
/// 未设 / `"0"` / `"true"` / 其它任何取值 = 关 (fail-closed, 命名即承诺:
/// 没有这把钥匙就是没有在场感知面)。
pub fn device_sense_enabled_from_env() -> bool {
    std::env::var(DEVICE_SENSE_ENABLE_ENV)
        .ok()
        .is_some_and(|value| value.trim() == "1")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presence_status_counts_the_table_and_says_when_it_is_absent() {
        let root = std::env::temp_dir().join(format!(
            "apeireth-device-sense-status-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let registry = DeviceRegistry::under_data_dir(&root);
        let activity = DeviceSenseActivity::new();

        // 缺省表: 默认零设备。
        let status = device_presence_status(&registry, &activity).expect("缺省不是错误");
        assert_eq!(status.devices, Some(0));
        assert_eq!(status.whitelisted, Some(0));
        assert_eq!(status.last_sense_at_ms, None, "尚无感知不编时间");
        assert!(status.note.as_deref().unwrap().contains("默认零设备"));

        // 有表: 设备数 / 白名单数分列如实。
        registry
            .save(&DevicesBody {
                devices: vec![
                    Device {
                        id: "thermo-1".into(),
                        kind: DeviceKind::Sensor,
                        name: "温湿度".into(),
                        adapter: "mock".into(),
                        enabled: true,
                    },
                    Device {
                        id: "cam-1".into(),
                        kind: DeviceKind::Camera,
                        name: "门口".into(),
                        adapter: "mock".into(),
                        enabled: false,
                    },
                ],
            })
            .unwrap();
        activity.record_sense(1_700_000_000_000, "2 台白名单设备已感知");
        let status = device_presence_status(&registry, &activity).expect("表可开");
        assert_eq!(status.devices, Some(2));
        assert_eq!(status.whitelisted, Some(1));
        assert_eq!(status.last_sense_at_ms, Some(1_700_000_000_000));
        assert_eq!(
            status.last_sense_note.as_deref(),
            Some("2 台白名单设备已感知")
        );
    }

    #[test]
    fn the_switch_is_fail_closed_except_the_exact_opt_in() {
        crate::test_support::assert_switch_fails_closed_except_exact_one(
            DEVICE_SENSE_ENABLE_ENV,
            &["0", "true", "on", " 1 0 ", ""],
            device_sense_enabled_from_env,
            Some(|| DeviceSenseConfig::from_env(".").is_some()),
        );
    }
}
