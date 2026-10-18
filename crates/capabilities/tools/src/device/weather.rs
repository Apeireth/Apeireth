//! 天气工具面 (`tool.weather`, 声明名 `weather`) —— K 轴 L1/L2: 环境感知 +
//! 提醒信号。
//!
//! # 数据源如实标注
//!
//! 两个数据源档:
//!
//! - **mock 档** ([`MockWeatherSource`]): 固定 fixture 数据, 每帧恒标注
//!   `data_source: "mock-fixture"` + `data_note` 明说「mock 档固定 fixture
//!   数据, 非实测天气」—— 如实标注数据源, 绝不冒充实测;
//! - **live 档** ([`LiveWeatherSource`]): 公开天气接口读, 走**受控出口通道 /
//!   fetch 族**接缝 ([`WeatherFetchPort`]) —— **留插槽不接**: 端口没接线 =
//!   结构化拒绝帧 (0 装诚实, 不编天气)。
//!
//! # 提醒 = 只交信号 (L2 同源接缝)
//!
//! 天气信号 ([`ReminderSignal`]) 从天气读数确定性导出, 交给信号落点
//! ([`SignalSink`], 落 `weather-signals.jsonl` append-only 账) ——
//! **提醒纪律 (勿扰窗口 / 频率) 归提醒侧** (care 引擎同源接缝消费本信号),
//! 本面只交信号不决定发不发 (见 [`REMINDER_DISCIPLINE_NOTE`])。
//!
//! # 治理
//!
//! 权力面 = **只读 + append 信号账**: 零执行、live 档未接线时零外联。注册即闸
//! ( [`WeatherConfig::from_env`], 与 `tool.device_sense` 同一在场感知族开关
//! `APEIRETH_ENABLE_DEVICE_SENSE`, 默认关 fail-closed), grant 即用。

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use apeireth_core::kernel::CapabilityId;
use apeireth_core::stored_doc::{scan_records, DocCompat, RecordScan, StoredDoc};
use apeireth_plugin::ToolCapability;
use apeireth_protocol::canonical::{NormalizedTool, ToolCall, ToolParameters, ToolResult};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use super::sense::{device_sense_enabled_from_env, DEVICE_SENSE_ENABLE_ENV};
use crate::exec_pipeline::{AroundPolicy, OutputSchema, SchemaKind, DEFAULT_MAX_TIMEOUT_MS};

/// 稳定能力标识 (注册表 / 治理授权 / 风险映射共用)。
pub const WEATHER_CAPABILITY_ID: &str = "tool.weather";

/// 模型侧声明名 (短名, 每次请求都进上下文)。
pub const WEATHER_TOOL_NAME: &str = "weather";

/// 生产执行时限 (毫秒): 天气读数是现取现算, 到期即归 `timeout.*`。
pub const WEATHER_TIMEOUT_MS: u64 = 5_000;

/// 天气信号账文件 (数据目录落点: `<data_dir>/weather-signals.jsonl`)。
pub const WEATHER_SIGNALS_FILE: &str = "weather-signals.jsonl";

/// 信号账 StoredDoc 的身份戳。
pub const WEATHER_SIGNALS_DOC_NAME: &str = "weather-signals";

/// 信号账 StoredDoc 的格式版本戳。
pub const WEATHER_SIGNALS_DOC_VERSION: u32 = 1;

/// mock 档数据源标注 (每帧恒在场)。
pub const SOURCE_MOCK_FIXTURE: &str = "mock-fixture";

/// mock 档如实注记 (绝不冒充实测)。
pub const SOURCE_MOCK_NOTE: &str = "mock 档固定 fixture 数据, 非实测天气 (数据源如实标注)";

/// 提醒纪律声明 (只交信号): 勿扰 / 频率等提醒纪律归**提醒侧**消费。
pub const REMINDER_DISCIPLINE_NOTE: &str = "只交信号: 勿扰窗口 / 频率等提醒纪律归提醒侧 \
  (care 引擎同源接缝) 消费, 本面不决定发不发";

/// 动作: 现在的天气 (读数 + 信号)。
pub const ACTION_NOW: &str = "now";
/// 动作: 读信号账 (已交出的信号真值)。
pub const ACTION_LIST_SIGNALS: &str = "list_signals";

/// 闭合动作词表 (now / list_signals)。
pub const WEATHER_ACTIONS: &[&str] = &[ACTION_NOW, ACTION_LIST_SIGNALS];

/// 一次天气读数 (数据源逐帧如实标注)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WeatherReport {
    /// 地点 (调用方给的自由文本; mock 档原样回显不解析)。
    pub place: String,
    /// 观测时刻 (epoch 毫秒)。
    pub observed_at_ms: i64,
    /// 温度 (摄氏; `None` = 该源没给, 不编)。
    pub temperature_c: Option<f64>,
    /// 相对湿度 (%; `None` = 该源没给)。
    pub humidity_pct: Option<f64>,
    /// 风速 (m/s; `None` = 该源没给)。
    pub wind_speed_ms: Option<f64>,
    /// 天气状况 (闭合词表: clear / cloudy / rain / snow / unknown)。
    pub condition: String,
    /// 数据源标注 (`mock-fixture` / `live:<端点>`)。
    pub data_source: String,
    /// 数据源注记 (mock 档恒注明非实测)。
    pub data_note: String,
}

/// 天气错误 (闭合词表: 插槽未接线 / 读失败)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WeatherError {
    /// live 档受控出口通道**留插槽不接** (0 装诚实)。
    SlotNotWired {
        /// 目标端点 (如实回显)。
        endpoint: String,
    },
    /// 数据源读失败 (真因如实)。
    ReadFailed {
        /// 真因。
        reason: String,
    },
}

impl std::fmt::Display for WeatherError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SlotNotWired { endpoint } => write!(
                f,
                "live 天气档的受控出口通道 (fetch 族接缝) 未接线: {endpoint} —— \
                 不出数不冒充 (mock 档可用, 如实标注)"
            ),
            Self::ReadFailed { reason } => write!(f, "天气读失败: {reason}"),
        }
    }
}

impl std::error::Error for WeatherError {}

/// 天气源合同: 取一次天气读数 (读面)。
pub trait WeatherSource: Send + Sync {
    /// 数据源标签 (帧内 `data_source` 的来源)。
    fn label(&self) -> &str;

    /// 取一次读数 (`now_ms` = 观测时刻, 由调用方注入同一时钟)。
    fn observe(&self, place: &str, now_ms: i64) -> Result<WeatherReport, WeatherError>;
}

/// mock 天气源 (内置): 固定 fixture 数据, 恒标注 `mock-fixture` + 非实测注记。
pub struct MockWeatherSource;

impl MockWeatherSource {
    /// mock 天气源 (固定 fixture)。
    pub fn new() -> Self {
        Self
    }

    /// 固定 fixture 读数 (确定性; 低温值用于把降温提醒信号路径钉住)。
    pub fn fixture(place: &str, now_ms: i64) -> WeatherReport {
        WeatherReport {
            place: place.to_string(),
            observed_at_ms: now_ms,
            temperature_c: Some(2.5),
            humidity_pct: Some(70.0),
            wind_speed_ms: Some(3.2),
            condition: "cloudy".to_string(),
            data_source: SOURCE_MOCK_FIXTURE.to_string(),
            data_note: SOURCE_MOCK_NOTE.to_string(),
        }
    }
}

impl Default for MockWeatherSource {
    fn default() -> Self {
        Self::new()
    }
}

impl WeatherSource for MockWeatherSource {
    fn label(&self) -> &str {
        SOURCE_MOCK_FIXTURE
    }

    fn observe(&self, place: &str, now_ms: i64) -> Result<WeatherReport, WeatherError> {
        Ok(Self::fixture(place, now_ms))
    }
}

/// 受控出口通道 / fetch 族接缝 (live 天气档的唯一出网口)。
///
/// **留插槽不接**: 生产实现不在本波 (真实出口统一走 fetch 族受控通道, 接线
/// 后置); [`LiveWeatherSource`] 在端口缺位时如实报
/// [`WeatherError::SlotNotWired`]。
pub trait WeatherFetchPort: Send + Sync {
    /// 取一个公开天气端点的 JSON 文本 (受控出口通道语义由实现方承载)。
    fn fetch_json(&self, endpoint: &str) -> Result<String, String>;
}

/// live 天气源 (公开天气接口读): **插槽已留, 通道未接**。
///
/// 端口 [`WeatherFetchPort`] 缺位 = [`WeatherError::SlotNotWired`] (不出数不
/// 冒充); 端口接上后 [`LiveWeatherSource`] 的 `data_source` 恒带
/// `live:<端点>` 标注 (数据源如实)。
pub struct LiveWeatherSource {
    endpoint: String,
    port: Option<Arc<dyn WeatherFetchPort>>,
}

impl LiveWeatherSource {
    /// live 档插槽 (端口未接): 解析即报未接线。
    pub fn slot(endpoint: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
            port: None,
        }
    }

    /// live 档接线态 (端口接上后用; 本波无生产端口)。
    pub fn with_port(endpoint: impl Into<String>, port: Arc<dyn WeatherFetchPort>) -> Self {
        Self {
            endpoint: endpoint.into(),
            port: Some(port),
        }
    }
}

impl WeatherSource for LiveWeatherSource {
    fn label(&self) -> &str {
        "live"
    }

    fn observe(&self, place: &str, now_ms: i64) -> Result<WeatherReport, WeatherError> {
        let Some(port) = &self.port else {
            return Err(WeatherError::SlotNotWired {
                endpoint: self.endpoint.clone(),
            });
        };
        // 端口接上后的形状 (留槽合同): 拉 JSON → 解析 → 恒标 `live:<端点>`。
        // 本波无生产端口, 解析失败一律真因帧 (不回退 mock, 不混源)。
        let body = port
            .fetch_json(&self.endpoint)
            .map_err(|reason| WeatherError::ReadFailed { reason })?;
        let parsed: Value =
            serde_json::from_str(&body).map_err(|error| WeatherError::ReadFailed {
                reason: format!("live 天气 JSON 解析失败: {error}"),
            })?;
        Ok(WeatherReport {
            place: place.to_string(),
            observed_at_ms: now_ms,
            temperature_c: parsed.get("temperature_c").and_then(Value::as_f64),
            humidity_pct: parsed.get("humidity_pct").and_then(Value::as_f64),
            wind_speed_ms: parsed.get("wind_speed_ms").and_then(Value::as_f64),
            condition: parsed
                .get("condition")
                .and_then(Value::as_str)
                .unwrap_or("unknown")
                .to_string(),
            data_source: format!("live:{}", self.endpoint),
            data_note: "live 档实测读数 (受控出口通道)".to_string(),
        })
    }
}

// ---------------------------------------------------------------------------
// 提醒信号 (天气信号 → 提醒侧同源接缝: 只交信号)
// ---------------------------------------------------------------------------

/// 天气提醒信号 (L2 提醒的**信号载荷**)。
///
/// 字段合同 (稳定): `{signal_id, kind, trigger, severity, message, evidence,
/// source, source_data, created_at_ms, discipline}` —— 提醒侧 (care 引擎同源
/// 接缝) 消费本载荷; 勿扰窗口 / 频率等**提醒纪律归提醒侧**, 本面只交信号。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReminderSignal {
    /// 信号稳定身份 (`wxsig-<时刻毫秒>-<进程内序号>`)。
    pub signal_id: String,
    /// 信号族 (`weather`)。
    pub kind: String,
    /// 触发词表 (`temperature_drop` / `heat_risk` / `rain_expected` / `snow_expected`)。
    pub trigger: String,
    /// 严重度 (`info` / `warn`)。
    pub severity: String,
    /// 建议语 (提示措辞, **不是发送**; 发不发由提醒侧按纪律定)。
    pub message: String,
    /// 数据依据 (来自 [`WeatherReport`] 的有界摘要)。
    pub evidence: String,
    /// 信号来源工具 (`tool.weather`)。
    pub source: String,
    /// 数据源标注 (与 [`WeatherReport::data_source`] 同源: mock / live 如实)。
    pub source_data: String,
    /// 信号生成时刻 (epoch 毫秒)。
    pub created_at_ms: i64,
    /// 提醒纪律声明 (恒 [`REMINDER_DISCIPLINE_NOTE`])。
    pub discipline: String,
}

/// 信号族标签 (天气)。
pub const SIGNAL_KIND_WEATHER: &str = "weather";

/// 信号触发词表: 降温。
pub const TRIGGER_TEMPERATURE_DROP: &str = "temperature_drop";
/// 信号触发词表: 高温风险。
pub const TRIGGER_HEAT_RISK: &str = "heat_risk";
/// 信号触发词表: 有雨。
pub const TRIGGER_RAIN_EXPECTED: &str = "rain_expected";
/// 信号触发词表: 有雪。
pub const TRIGGER_SNOW_EXPECTED: &str = "snow_expected";

/// 进程内信号序号 (身份唯一性用, 非安全边界)。
static SIGNAL_ORDINAL: AtomicU64 = AtomicU64::new(1);

/// 信号身份 (`wxsig-<时刻毫秒>-<进程内序号>`)。
pub fn reminder_signal_id(created_at_ms: i64, ordinal: u64) -> String {
    format!("wxsig-{created_at_ms}-{ordinal}")
}

/// 从天气读数确定性导出提醒信号 (纯判定, 同读数恒同信号集)。
///
/// 规则 (闭合): 温度 ≤ 5°C → 降温 `warn`; 温度 ≥ 35°C → 高温 `warn`;
/// 有雨/有雪 → `info`。**只交信号**: 消息是建议语, 发不发由提醒侧按
/// 勿扰/频率纪律定。
pub fn weather_signals(report: &WeatherReport) -> Vec<ReminderSignal> {
    let mut signals = Vec::new();
    let mut push = |trigger: &str, severity: &str, message: String| {
        signals.push(ReminderSignal {
            signal_id: reminder_signal_id(
                report.observed_at_ms,
                SIGNAL_ORDINAL.fetch_add(1, Ordering::Relaxed),
            ),
            kind: SIGNAL_KIND_WEATHER.to_string(),
            trigger: trigger.to_string(),
            severity: severity.to_string(),
            message,
            evidence: format!(
                "{}: temperature_c={:?}, condition={} ({})",
                report.place, report.temperature_c, report.condition, report.data_source
            ),
            source: WEATHER_CAPABILITY_ID.to_string(),
            source_data: report.data_source.clone(),
            created_at_ms: report.observed_at_ms,
            discipline: REMINDER_DISCIPLINE_NOTE.to_string(),
        });
    };
    if let Some(temperature) = report.temperature_c {
        if temperature <= 5.0 {
            push(
                TRIGGER_TEMPERATURE_DROP,
                "warn",
                format!("降温提醒: {temperature}°C 偏冷, 记得添衣 (建议语, 发不发由提醒侧定)"),
            );
        }
        if temperature >= 35.0 {
            push(
                TRIGGER_HEAT_RISK,
                "warn",
                format!("高温提醒: {temperature}°C 偏热, 注意避暑补水 (建议语, 发不发由提醒侧定)"),
            );
        }
    }
    match report.condition.as_str() {
        "rain" => push(
            TRIGGER_RAIN_EXPECTED,
            "info",
            "有雨提醒: 出门记得带伞 (建议语, 发不发由提醒侧定)".to_string(),
        ),
        "snow" => push(
            TRIGGER_SNOW_EXPECTED,
            "info",
            "有雪提醒: 路滑注意安全 (建议语, 发不发由提醒侧定)".to_string(),
        ),
        _ => {}
    }
    signals
}

/// 信号落点合同: 交出一条天气提醒信号 (提醒侧消费的同源接缝)。
///
/// 落点实现: [`SignalLedger`] (append-only 账, 提醒侧从账消费) / 测试用内存
/// 落点。**提醒纪律 (勿扰 / 频率) 不在本合同内** —— 那是提醒侧的事。
pub trait SignalSink: Send + Sync {
    /// 交出一条信号。
    fn emit(&self, signal: &ReminderSignal) -> Result<(), String>;
}

/// 信号账 (`weather-signals.jsonl`, append-only, 逐行一个 [`StoredDoc`] 信封)。
///
/// 提醒侧 (care 引擎同源接缝) 从本账消费信号; 坏行读作不存在 (跳过并计入
/// `skipped`), 不砖化整本。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignalLedger {
    path: PathBuf,
}

impl SignalLedger {
    /// 数据目录落点 (`<data_dir>/weather-signals.jsonl`)。
    pub fn under_data_dir(data_dir: impl AsRef<Path>) -> Self {
        Self {
            path: data_dir.as_ref().join(WEATHER_SIGNALS_FILE),
        }
    }

    /// 显式路径落点。
    pub fn at(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// 信号账路径。
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// 追加一行信号 (JSONL 单行 + 落盘; append-only, 不重写不删行)。
    pub fn append(&self, signal: &ReminderSignal) -> Result<(), String> {
        use std::io::Write;
        if let Some(parent) = self.path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)
                    .map_err(|error| format!("信号账目录创建失败 {parent:?}: {error}"))?;
            }
        }
        let doc = StoredDoc::new(
            WEATHER_SIGNALS_DOC_NAME,
            WEATHER_SIGNALS_DOC_VERSION,
            vec![WEATHER_SIGNALS_DOC_VERSION],
            signal,
        );
        let line =
            serde_json::to_vec(&doc).map_err(|error| format!("信号账序列化失败: {error}"))?;
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .map_err(|error| format!("信号账打开失败 {}: {error}", self.path.display()))?;
        file.write_all(&line)
            .and_then(|()| file.write_all(b"\n"))
            .and_then(|()| file.sync_all())
            .map_err(|error| format!("信号账写入失败 {}: {error}", self.path.display()))
    }

    /// 读回整本 (逐记录语义: 坏行读作不存在并计入 `skipped`)。
    pub fn scan(&self) -> Result<RecordScan<ReminderSignal>, String> {
        scan_records::<ReminderSignal>(&self.path, &ledger_compat())
            .map_err(|error| format!("信号账读取失败 {}: {error}", self.path.display()))
    }
}

impl SignalSink for SignalLedger {
    fn emit(&self, signal: &ReminderSignal) -> Result<(), String> {
        self.append(signal)
    }
}

/// 信号账读端契约 (仅当前格式版本可读)。
fn ledger_compat() -> DocCompat {
    DocCompat::exact(WEATHER_SIGNALS_DOC_NAME, WEATHER_SIGNALS_DOC_VERSION)
}

/// 内存信号落点 (测试/无账装配用): 只收不落盘。
#[derive(Debug, Default)]
pub struct MemorySignalSink {
    seen: std::sync::Mutex<Vec<ReminderSignal>>,
}

impl MemorySignalSink {
    /// 内存落点。
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    /// 已收信号 (顺序保持)。
    pub fn seen(&self) -> Vec<ReminderSignal> {
        self.seen
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }
}

impl SignalSink for MemorySignalSink {
    fn emit(&self, signal: &ReminderSignal) -> Result<(), String> {
        self.seen
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .push(signal.clone());
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// 工具面
// ---------------------------------------------------------------------------

/// 调用参数 (未知字段拒绝)。
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct WeatherParams {
    action: Option<String>,
    place: Option<String>,
}

/// 天气工具 (`tool.weather`, 声明名 `weather`)。
///
/// 读面零执行: `now` 取一次天气读数并交出提醒信号 (只交信号),
/// `list_signals` 读信号账真值。
pub struct WeatherTool {
    id: CapabilityId,
    source: Arc<dyn WeatherSource>,
    sink: Arc<dyn SignalSink>,
    signals: SignalLedger,
}

impl WeatherTool {
    /// 绑定天气源 / 信号落点 / 信号账 (账 = 提醒侧消费面)。
    pub fn new(
        source: Arc<dyn WeatherSource>,
        sink: Arc<dyn SignalSink>,
        signals: SignalLedger,
    ) -> Self {
        Self {
            id: CapabilityId::new(WEATHER_CAPABILITY_ID).expect("valid capability id"),
            source,
            sink,
            signals,
        }
    }

    /// 输出归一合同: 结构化输出必须是对象且 `action` / `status` 恒在场。
    pub fn output_schema() -> OutputSchema {
        OutputSchema::new()
            .require("action", SchemaKind::String)
            .require("status", SchemaKind::String)
    }

    /// 信号账 (提醒侧消费面同账)。
    pub fn signals(&self) -> &SignalLedger {
        &self.signals
    }

    fn now(&self, call: &ToolCall, params: &WeatherParams) -> ToolResult {
        let place = params.place.as_deref().unwrap_or("local");
        let report = match self.source.observe(place, now_epoch_ms()) {
            Ok(report) => report,
            Err(error) => return self.refuse(call, format!("天气读失败 (不编天气): {error}")),
        };
        let signals = weather_signals(&report);
        for signal in &signals {
            if let Err(error) = self.sink.emit(signal) {
                return self.refuse(call, format!("提醒信号入账失败 (未交出, 无半帧): {error}"));
            }
        }
        ToolResult::ok(
            &call.id,
            json!({
                "action": ACTION_NOW,
                "status": "ok",
                "report": report,
                "signals": signals,
                "signals_emitted": signals.len(),
                "discipline_note": REMINDER_DISCIPLINE_NOTE,
            }),
        )
        .with_name(WEATHER_TOOL_NAME)
    }

    fn list_signals(&self, call: &ToolCall) -> ToolResult {
        let scan = match self.signals.scan() {
            Ok(scan) => scan,
            Err(error) => return self.refuse(call, format!("信号账读取失败: {error}")),
        };
        let recent: Vec<&ReminderSignal> = scan
            .records
            .iter()
            .map(|record| &record.body)
            .rev()
            .take(WEATHER_SIGNALS_RECENT_LIMIT)
            .collect();
        ToolResult::ok(
            &call.id,
            json!({
                "action": ACTION_LIST_SIGNALS,
                "status": "ok",
                "total": scan.records.len(),
                "skipped_bad_rows": scan.skipped,
                "recent": recent,
                "discipline_note": REMINDER_DISCIPLINE_NOTE,
            }),
        )
        .with_name(WEATHER_TOOL_NAME)
    }

    /// 拒绝即帧 (真因如实, 不静默降级)。
    fn refuse(&self, call: &ToolCall, reason: impl Into<String>) -> ToolResult {
        ToolResult::permanent_error(&call.id, reason.into()).with_name(WEATHER_TOOL_NAME)
    }
}

/// 信号账「最近信号」的读回上限 (条)。
pub const WEATHER_SIGNALS_RECENT_LIMIT: usize = 5;

#[async_trait]
impl ToolCapability for WeatherTool {
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
                        "enum": ["now", "list_signals"],
                        "description": "now reads current weather and hands over reminder signals; list_signals reads the signal ledger's true records."
                    },
                    "place": {
                        "type": "string",
                        "description": "now only: free-text place label (echoed as-is by the mock source)."
                    }
                },
                "required": ["action"],
                "additionalProperties": false
            })
            .as_object()
            .cloned()
            .unwrap_or_default(),
        );
        NormalizedTool::new(WEATHER_TOOL_NAME)
            .with_description(
                "Read public weather (K-axis L1 environment sensing) and hand over weather \
                 reminder signals (K-axis L2). Every frame labels its data source honestly: \
                 the mock source returns fixed fixture data marked data_source=mock-fixture \
                 with an explicit not-real-measurement note; the live source goes through the \
                 controlled egress / fetch port seam and is honestly unwired until that port \
                 is connected. Reminder signals are only handed over (signal ledger \
                 weather-signals.jsonl): delivery discipline such as quiet hours and \
                 frequency belongs to the reminder side, never to this face.",
            )
            .with_parameters(parameters)
    }

    async fn invoke(&self, call: &ToolCall) -> ToolResult {
        let arguments = match &call.arguments {
            Value::Null => json!({}),
            other => other.clone(),
        };
        let params = match serde_json::from_value::<WeatherParams>(arguments) {
            Ok(params) => params,
            Err(error) => {
                return self.refuse(call, format!("invalid weather parameters: {error}"));
            }
        };
        let Some(action) = params.action.as_deref() else {
            return self.refuse(call, "拒绝天气读: 必须显式给 action (now / list_signals)");
        };
        match action {
            ACTION_NOW => self.now(call, &params),
            ACTION_LIST_SIGNALS => self.list_signals(call),
            other => self.refuse(
                call,
                format!(
                    "unknown weather action {other:?}; the closed vocabulary is \
                     now / list_signals"
                ),
            ),
        }
    }
}

// ---------------------------------------------------------------------------
// 配置 (注册即闸, 默认关 fail-closed)
// ---------------------------------------------------------------------------

/// 天气源档 (装配注入)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WeatherSourceConfig {
    /// mock 档: 固定 fixture 数据 (如实标注, 非实测)。
    Mock,
    /// live 档: 公开天气接口 (受控出口通道插槽) —— 本波**留插槽不接**。
    Live {
        /// 公开天气端点 (如实回显)。
        endpoint: String,
    },
}

/// 天气面配置 (数据源档 + 信号账落位)。
///
/// 组装根按 [`WeatherConfig::from_env`] 注入 (注册即闸, 与 `tool.device_sense`
/// 同一在场感知族开关 [`DEVICE_SENSE_ENABLE_ENV`], 默认关 fail-closed)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WeatherConfig {
    /// 信号账所在数据目录 (`<data_dir>/weather-signals.jsonl`)。
    pub data_dir: PathBuf,
    /// 数据源档。
    pub source: WeatherSourceConfig,
}

impl WeatherConfig {
    /// mock 档配置 (默认档: 骨架可测, 真实接口后插)。
    pub fn mock(data_dir: impl Into<PathBuf>) -> Self {
        Self {
            data_dir: data_dir.into(),
            source: WeatherSourceConfig::Mock,
        }
    }

    /// live 档配置 (**留插槽不接**: 装配上仍无生产出口端口)。
    pub fn live(data_dir: impl Into<PathBuf>, endpoint: impl Into<String>) -> Self {
        Self {
            data_dir: data_dir.into(),
            source: WeatherSourceConfig::Live {
                endpoint: endpoint.into(),
            },
        }
    }

    /// 本配置的信号账。
    pub fn signals(&self) -> SignalLedger {
        SignalLedger::under_data_dir(&self.data_dir)
    }

    /// 数据源档 → 天气源实例 (live 档 = 插槽, 解析即报未接线)。
    pub fn weather_source(&self) -> Arc<dyn WeatherSource> {
        match &self.source {
            WeatherSourceConfig::Mock => Arc::new(MockWeatherSource::new()),
            WeatherSourceConfig::Live { endpoint } => {
                Arc::new(LiveWeatherSource::slot(endpoint.clone()))
            }
        }
    }

    /// 从 env 构造 (mock 档): 开关开才 `Some` (默认关 → [`None`], fail-closed)。
    ///
    /// 开关 = 在场感知族总闸 [`DEVICE_SENSE_ENABLE_ENV`] (`tool.weather` 与
    /// `tool.device_sense` 同族一闸, 不另立旋钮)。
    pub fn from_env(data_dir: impl Into<PathBuf>) -> Option<Self> {
        device_sense_enabled_from_env().then(|| Self::mock(data_dir))
    }
}

/// 组装天气面五段流水线: 读面 (低风险档) → 超时 → 输出归一合同。
pub fn weather_pipeline() -> Arc<crate::exec_pipeline::ToolExecutionPipeline> {
    Arc::new(
        crate::exec_pipeline::ToolExecutionPipeline::new().with_around(
            AroundPolicy::new().with_timeout(WEATHER_TIMEOUT_MS, DEFAULT_MAX_TIMEOUT_MS),
        ),
    )
}

/// 组装天气工具面 (模型侧一个能力)。
pub fn weather_capability(config: WeatherConfig) -> Arc<dyn ToolCapability> {
    let ledger = config.signals();
    let sink: Arc<dyn SignalSink> = Arc::new(ledger.clone());
    let tool: Arc<dyn ToolCapability> =
        Arc::new(WeatherTool::new(config.weather_source(), sink, ledger));
    Arc::new(crate::exec_pipeline::PipelinedCapability::new(
        tool,
        weather_pipeline(),
    ))
}

fn now_epoch_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_fixture_is_deterministic_and_labeled_not_real() {
        let source = MockWeatherSource::new();
        let first = source.observe("阳台", 1_700_000_000_000).unwrap();
        let second = source.observe("阳台", 1_700_000_000_000).unwrap();
        assert_eq!(first, second, "固定 fixture 每次同值");
        assert_eq!(first.data_source, SOURCE_MOCK_FIXTURE);
        assert!(first.data_note.contains("非实测天气"));
    }

    #[test]
    fn signal_rules_are_deterministic_and_only_hand_over() {
        let report = MockWeatherSource::fixture("local", 1_700_000_000_000);
        let signals = weather_signals(&report);
        assert_eq!(signals.len(), 1, "低温档只出降温信号");
        assert_eq!(signals[0].trigger, TRIGGER_TEMPERATURE_DROP);
        assert_eq!(signals[0].severity, "warn");
        assert_eq!(signals[0].kind, SIGNAL_KIND_WEATHER);
        assert_eq!(signals[0].source, WEATHER_CAPABILITY_ID);
        assert_eq!(signals[0].source_data, SOURCE_MOCK_FIXTURE);
        assert_eq!(signals[0].discipline, REMINDER_DISCIPLINE_NOTE);

        let mut hot = MockWeatherSource::fixture("local", 1_700_000_000_000);
        hot.temperature_c = Some(36.0);
        hot.condition = "rain".to_string();
        let hot_signals = weather_signals(&hot);
        let triggers: Vec<&str> = hot_signals
            .iter()
            .map(|signal| signal.trigger.as_str())
            .collect();
        assert_eq!(triggers, [TRIGGER_HEAT_RISK, TRIGGER_RAIN_EXPECTED]);
    }

    #[test]
    fn the_live_slot_is_honestly_unwired() {
        let source = LiveWeatherSource::slot("https://weather.example.invalid");
        let error = source.observe("local", 0).expect_err("插槽必须报未接线");
        assert!(
            matches!(error, WeatherError::SlotNotWired { .. }),
            "live 插槽未接线必须如实报: {error}"
        );
    }
}
