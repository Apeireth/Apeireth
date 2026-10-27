#![no_main]

//! fuzz: **在场感知面** (`tool.device_sense`, 声明名 `device_sense`) —— K 轴 L1。
//!
//! # 被测面
//!
//! - 主被测: `crates/capabilities/tools/src/device/sense.rs`
//!   (`DeviceSenseTool::invoke` / `DeviceSenseBoundaryGuard` / `device_presence_status`
//!   / `DeviceSenseActivity` / `device_sense_enabled_from_env`)。
//! - 只读（**不改**）: `crates/capabilities/tools/src/device/{registry,audit,adapter}.rs`
//!   的公开 API —— 本 target 只**消费**它们（设备表 / 审计账 / 适配器插槽图）。
//!
//! # 两条不可信输入面
//!
//! 1. **设备表落盘字节** (`devices.json` = StoredDoc 信封): 模块头写死了
//!    「[`DeviceRegistry`] = `devices.json` StoredDoc 拒开, 默认零设备」
//!    (`sense.rs:5-6`) ⇒ 表可以是**任意字节**（畸形 / 串档 / 未来版本 / 坏条目）。
//! 2. **调用参数** (模型回传的 JSON): 「调用参数 (未知字段拒绝)」(`sense.rs:74`)
//!    + 声明面 `additionalProperties: false` / `required: ["action"]`
//!    (`sense.rs:444-470`)。
//!
//! # 判据纪律
//!
//! 每条断言都指到**被测代码自己写下的契约**（文件:行号 + 原文片段）；**不自造期望**。
//! 「不 panic」只在契约明写「畸形输入一律拒开 / 拒绝即帧」处才敢用（见判据 ⑫）。
//!
//! | # | 判据 | 出处（原文片段） |
//! |---|------|------------------|
//! | ① | 参数面闭合: 未知字段 / 类型不符 / `action` 缺失 / `action` 词表外 ⇒ **拒绝即帧** | `sense.rs:74`「调用参数 (未知字段拒绝)」; `sense.rs:71-72`「闭合动作词表 (list / sense)」; `sense.rs:502-504`「拒绝感知: 必须显式给 action (list / sense)」; `sense.rs:468-469` 声明 `"required": ["action"]` + `"additionalProperties": false` |
//! | ② | 白名单制 (底线①): 表外 id / 停用 id / 空 `device_ids` ⇒ **拒绝即帧** | `sense.rs:5-8`「只读**白名单设备**的状态 … 表外 / 停用设备一律拒绝即帧」; `sense.rs:400-402`「指定 id 时逐台核白名单。表外 / 停用设备 = `Err(拒绝即帧文案)`」; `sense.rs:410-413` 空列表拒绝 |
//! | ③ | 适配器词表: 词表外适配器 / 真实协议插槽 ⇒ **拒绝即帧** | `sense.rs:7`「词表外适配器拒绝即帧」; `adapter.rs:8-12` 插槽图; `adapter.rs:15-16`「解析到插槽名 = 结构化 `AdapterError::SlotNotWired` 拒绝 (0 装诚实)」 |
//! | ④ | 敏感面授权: 摄像头类必须 `sensitive_read_authorized: true` **且** `authorized_by` 非空, 缺任一 ⇒ **拒绝即帧 + 审计行 `denied`** | `sense.rs:10-15`「放行要**显式授权** (`sensitive_read_authorized: true` + 非空 `authorized_by`), 缺任一 = 拒绝即帧 + 审计行 `denied`」; `audit.rs:5-6`「每一次感知尝试 (无论放行还是拒绝) 都落一行审计」 |
//! | ⑤ | 每次敏感面尝试都入审计账 (放行 `granted` / 拒绝 `denied`), 行的 `action` 恒 `sense`、类别恒摄像头、授权人如实 | `audit.rs:55`「设备类别 (敏感面判定依据; 摄像头类恒入账)」; `audit.rs:57`「动作 (`sense`)」; `audit.rs:59`「谁给的显式授权 (`None` = 无授权人, 对应 `denied`)」 |
//! | ⑥ | **审计账写不进 = 不放行** (fail-closed), 且此时不得读任何设备 | `sense.rs:15`「审计账写不进 = 不放行 (fail-closed)」; `sense.rs:318-324` |
//! | ⑦ | 拒绝即帧 = **永久错误** (不可重试) + 真因非空 | `sense.rs:394-397`「拒绝即帧 (真因如实, 不静默降级)」→ `ToolResult::permanent_error` (`protocol/src/canonical/tool_result.rs:90-99`, `retryable: false`) |
//! | ⑧ | 输出归一合同: 结构化输出是对象且 `action` / `status` 恒在场 | `sense.rs:218-223`「输出归一合同: 结构化输出必须是对象且 `action` / `status` 恒在场」 |
//! | ⑨ | `list` = **零适配器读 + 零审计行 + 无读数面** (只回白名单视图) | `sense.rs:66`「动作: 读白名单视图 (零适配器读)」; `sense.rs:191`「`list` 只回白名单视图」; 审计行的 `action` 恒 `sense` (`audit.rs:57`) ⇒ `list` 不入账 |
//! | ⑩ | 自述在场节与工具面**同表同计数** (设备数 / 白名单数 / 最近感知) | `sense.rs:23-27`「与工具面同一本设备表同一进程内感知计数」; `sense.rs:158-160`「在场节真值 … 与工具面同表同计数」; `registry.rs:150-156` / `registry.rs:163` 白名单与设备数口径 |
//! | ⑪ | 边界 Guard 单调: 开了授权旗却没给授权人 ⇒ **拒**; 且拒了不得被后续阶段翻回放行 | `sense.rs:519-521`「单调边界 Guard (只拒不放) … 开了授权旗但没给授权人 = 拒 (任何后续阶段都不能翻回放行)」; `sense.rs:541-566` |
//! | ⑫ | 坏表拒开**不回退默认表**: 表存在但拒开 ⇒ 工具必须拒 (不得当成零设备放行) | `sense.rs:238`「设备表拒开 (不回退默认表)」; `registry.rs:12-15`「畸形 JSON / 串档身份 / 未来版本 / 坏条目 … 一律 `DeviceStoreError` 拒开, 不静默迁移、不回退默认表」 |
//! | ⑬ | 摄像头 fixture 永不回图像内容; 数据源如实标注 `mock-fixture` | `adapter.rs:128`「摄像头类 fixture 只回**存在性/事件位**, 永不回图像内容」; `adapter.rs:47` / `adapter.rs:53`「数据源如实标注 (`mock-fixture` / `live:<适配器名>`)」; `audit.rs:11`「图像内容永不出适配器」 |
//! | ⑭ | 开关 fail-closed: 只认显式 `"1"`（前后空白容忍）, 其它任何取值 = 关 | `sense.rs:60-63`「只认显式 `"1"` (前后空白容忍); 未设 / `"0"` / 其它任何取值 = 关」; `sense.rs:642-650` |
//!
//! # 诚实边界（本 target **没有**覆盖到的东西）
//!
//! - **五段流水线的 Around 段（超时）没实跑**: `apeireth_core::deadline::Deadline::after`
//!   要 tokio 运行时上下文 (`core/src/deadline.rs:150`), 而 fuzz crate 按约定**零外部依赖**
//!   ⇒ 流水线拆成三段**分别直测**: Guard 段直测 `DeviceSenseBoundaryGuard::deny`,
//!   Normalize 段直测 `DeviceSenseTool::output_schema().validate`
//!   （即 `exec_pipeline/contract.rs:76-81` 做的同一件事）, 执行段直测 `tool.invoke`。
//! - **「`list` 只回白名单视图」的措辞与实现有张力**（实现把**停用设备也列出来**, 只带
//!   `enabled` 标记）⇒ 本 target **不断言**「`list` 只能出现启用设备」, 只断言
//!   `list` 的零读 / 零审计 / 无读数面。
//! - **表外摄像头请求不入审计账**（白名单闸在前, 表外设备没有 id/kind 可记）
//!   ⇒ 契约「每一次感知尝试都落一行审计」在这条路径上无法落实 ⇒ **不断言**。
//! - **`device_ids` 重复项的读数条数**（实现逐项读, 不去重）⇒ 只断言
//!   「不读表外设备」+「每个目标都被读到」+（无重复时）「`whitelisted` == 读数条数」。
//! - **探测适配器 (`fuzz_probe`) 的读数**是本 harness 私有的 ⇒ 不断言其内容;
//!   只有 mock 档的读数受判据 ⑬ 约束。
//! - **切向执行 / 时钟回拨 / 并发**不在本面契约内 ⇒ 未覆盖。

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::task::{Context, Poll, Wake, Waker};

use apeireth_core::kernel::CapabilityId;
use apeireth_governance::{ToolGuard, ToolGuardRequest};
use apeireth_plugin::ToolCapability;
use apeireth_protocol::canonical::{ToolCall, ToolOutcome, ToolResult};
use apeireth_tools_canonical::device::adapter::{
    AdapterError, AdapterRegistry, DeviceAdapter, DeviceState, MockAdapter, MOCK_ADAPTER,
    RESERVED_PROTOCOL_SLOTS,
};
use apeireth_tools_canonical::device::audit::{
    DeviceSenseAuditLedger, SensitiveReadAudit, SensitiveReadOutcome, DEVICE_SENSE_AUDIT_FILE,
};
use apeireth_tools_canonical::device::registry::{
    Device, DeviceKind, DeviceRegistry, DevicesBody, DEVICES_DOC_NAME, DEVICES_DOC_VERSION,
};
use apeireth_tools_canonical::device::sense::{
    device_presence_status, device_sense_enabled_from_env, DeviceSenseActivity,
    DeviceSenseBoundaryGuard, DeviceSenseConfig, DeviceSenseTool, ACTION_LIST, ACTION_SENSE,
    DEVICE_SENSE_ACTIONS, DEVICE_SENSE_CAPABILITY_ID, DEVICE_SENSE_ENABLE_ENV,
    DEVICE_SENSE_TOOL_NAME,
};
use libfuzzer_sys::fuzz_target;
use serde_json::{json, Value};

// ============================================================================
// 零依赖 block_on（与 fuzz/fuzz_targets/credentials.rs 同姿势）
// ============================================================================

/// 无依赖的极简 `block_on`: 工具的 `invoke` 是 async, 但实现内没有任何挂起点
/// （一次 poll 即完成）, 不需要 tokio。
struct NoopWaker;

impl Wake for NoopWaker {
    fn wake(self: Arc<Self>) {}
}

fn block_on<F: std::future::Future>(future: F) -> F::Output {
    let waker = Waker::from(Arc::new(NoopWaker));
    let mut cx = Context::from_waker(&waker);
    let mut future = Box::pin(future);
    loop {
        match future.as_mut().poll(&mut cx) {
            Poll::Ready(value) => return value,
            Poll::Pending => std::thread::yield_now(),
        }
    }
}

// ============================================================================
// 夹具目录
// ============================================================================

/// 本 target 的临时根目录（设备表 + 审计账落位）。
///
/// 用 `/dev/shm`（tmpfs）而不是磁盘: 审计账每行都要 `sync_all()`
/// (`audit.rs:115-118`) ⇒ 磁盘 fsync 会把 fuzz 吞吐压到几百次/秒; tmpfs 上
/// 同一段代码快两个数量级, 且**不占本机磁盘**。
fn work_root() -> &'static PathBuf {
    static ROOT: OnceLock<PathBuf> = OnceLock::new();
    ROOT.get_or_init(|| {
        let mut base = PathBuf::from("/dev/shm");
        if !base.is_dir() {
            base = std::env::temp_dir();
        }
        base.push(format!("apeireth-fuzz-device-sense-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&base).expect("建工作目录");
        base
    })
}

/// 「审计账写不进」目录: 审计账路径被做成**目录** ⇒ `append` 必然失败 (EISDIR)
/// ⇒ 判据 ⑥ 的 fail-closed 分支必然被走到 (`sense.rs:15`「审计账写不进 = 不放行」)。
fn broken_audit_root() -> &'static PathBuf {
    static ROOT: OnceLock<PathBuf> = OnceLock::new();
    ROOT.get_or_init(|| {
        let dir = work_root().join("audit-broken");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join(DEVICE_SENSE_AUDIT_FILE)).expect("建坏审计账目录");
        dir
    })
}

// ============================================================================
// 探测适配器（只观测「读了几台、读了哪几台」, 不改任何产品语义）
// ============================================================================

/// 本 target 私有的探测适配器名（产品词表外 ⇒ 只有本 harness 注册得上）。
const PROBE_ADAPTER: &str = "fuzz_probe";

/// 计数适配器: 把「适配器到底被读了几台 / 哪几台」变成可观测读数。
///
/// 用途: 判据 ⑨（`list` 零适配器读）与判据 ⑥（fail-closed 时不得读）需要一个
/// **可数**的读面 —— 产品自带的 mock 适配器是纯函数, 读没读过它不露脸。
struct ProbeAdapter {
    reads: Arc<Mutex<Vec<String>>>,
}

impl DeviceAdapter for ProbeAdapter {
    fn name(&self) -> &str {
        PROBE_ADAPTER
    }

    fn read_state(&self, device: &Device, now_ms: i64) -> Result<DeviceState, AdapterError> {
        self.reads
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .push(device.id.clone());
        let mut readings = BTreeMap::new();
        readings.insert("probe".to_string(), Value::Bool(true));
        Ok(DeviceState {
            device_id: device.id.clone(),
            kind: device.kind,
            adapter: PROBE_ADAPTER.to_string(),
            online: true,
            readings,
            observed_at_ms: now_ms,
            source_note: "fuzz-probe".to_string(),
        })
    }
}

/// **计数版 mock 适配器**: 行为与产品自带的 mock 适配器完全一致
/// （同样的固定 fixture、同样的 `mock-fixture` 标注）, 只多记一笔「读了哪台」。
///
/// 为什么必须包一层: 读数面里既有 mock 档的台, 也有探测档的台 ⇒ 计数日志
/// 必须**两个适配器都往里写**, 否则「读数面 == 实读序列」这条判据会误报。
struct CountingMock {
    inner: MockAdapter,
    reads: Arc<Mutex<Vec<String>>>,
}

impl DeviceAdapter for CountingMock {
    fn name(&self) -> &str {
        MOCK_ADAPTER
    }

    fn read_state(&self, device: &Device, now_ms: i64) -> Result<DeviceState, AdapterError> {
        self.reads
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .push(device.id.clone());
        self.inner.read_state(device, now_ms)
    }
}

// ============================================================================
// 场景构造
// ============================================================================

/// 设备表的四种形态。
#[derive(Debug, Clone)]
enum TableSpec {
    /// 文件不存在（缺省 = 默认零设备）。
    Missing,
    /// 合法信封 + 构造好的设备行。
    Valid(Vec<Device>),
    /// 任意 JSON（可能是坏信封 / 坏条目 ⇒ 拒开）。
    Json(Value),
    /// 任意字节（可能畸形 / 串档 / 未来版本 / 坏条目）。
    Raw(Vec<u8>),
}

/// 一个 fuzz 场景: 设备表 + 调用参数（+ 是否走「审计账写不进」的坏目录）。
#[derive(Debug, Clone)]
struct Case {
    table: TableSpec,
    args: Value,
    broken_audit: bool,
}

/// 设备 id 池（固定 ⇒ 构造出的表**没有重复 id**, 必是合法表）。
const ID_POOL: &[&str] = &[
    "thermo-1", "cam-1", "lamp-1", "misc-1", "hall-1", "porch-1", "attic-1", "shed-1",
];

/// 适配器名池: mock + 探测 + 三个真实协议插槽 + 一个词表外名 (`adapter.rs:8-12`)。
const ADAPTER_POOL: &[&str] = &[
    MOCK_ADAPTER,
    PROBE_ADAPTER,
    "local_bridge",
    "cloud_bridge",
    "direct",
    "toaster-os",
];

/// 由 fuzz 字节构造一批设备行（id 唯一 ⇒ 表体合法; kind / adapter / enabled 随机）。
fn build_devices(spec: &[u8]) -> Vec<Device> {
    let count = (spec.first().copied().unwrap_or(0) as usize % 5) + 1;
    let mut devices = Vec::with_capacity(count);
    for (index, byte) in spec.iter().take(count).enumerate() {
        let id = ID_POOL[index % ID_POOL.len()];
        let kind = match (byte >> 1) & 0b11 {
            0 => DeviceKind::Sensor,
            1 => DeviceKind::Camera,
            2 => DeviceKind::Appliance,
            _ => DeviceKind::Other,
        };
        let adapter = ADAPTER_POOL[(usize::from(byte >> 3)) % ADAPTER_POOL.len()];
        devices.push(Device {
            id: id.to_string(),
            kind,
            name: format!("设备 {id}"),
            adapter: adapter.to_string(),
            enabled: byte & 0b1000_0000 == 0,
        });
    }
    devices
}

/// 落盘设备表（信封形状 = `registry.rs:277-278` 写明的稳定合同）。
fn write_table(registry: &DeviceRegistry, spec: &TableSpec) {
    match spec {
        TableSpec::Missing => {
            let _ = std::fs::remove_file(registry.path());
        }
        TableSpec::Valid(devices) => {
            let doc = json!({
                "name": DEVICES_DOC_NAME,
                "version": DEVICES_DOC_VERSION,
                "compatible_versions": [DEVICES_DOC_VERSION],
                "body": DevicesBody {
                    devices: devices.clone(),
                },
            });
            let bytes = serde_json::to_vec(&doc).expect("设备表序列化");
            std::fs::write(registry.path(), bytes).expect("写设备表");
        }
        TableSpec::Json(doc) => {
            let bytes = serde_json::to_vec(doc).expect("设备表序列化");
            std::fs::write(registry.path(), bytes).expect("写设备表");
        }
        TableSpec::Raw(bytes) => {
            std::fs::write(registry.path(), bytes).expect("写设备表");
        }
    }
}

/// 把合法信封按 fuzz 字节**变异**一处 —— 造出 `registry.rs:12-15` 写明的各种
/// 拒开形态（串档身份 / 未来版本 / 信封不自洽 / 空 id / 词表外 kind /
/// 行内未知字段 / 重复 id / 主体未知字段）。
fn mutate_envelope(devices: &[Device], payload: &[u8]) -> Value {
    let mut doc = json!({
        "name": DEVICES_DOC_NAME,
        "version": DEVICES_DOC_VERSION,
        "compatible_versions": [DEVICES_DOC_VERSION],
        "body": DevicesBody {
            devices: devices.to_vec(),
        },
    });
    match payload.first().copied().unwrap_or(0) % 8 {
        0 => doc["name"] = json!("other-doc"),
        1 => doc["version"] = json!(DEVICES_DOC_VERSION + 1),
        2 => doc["compatible_versions"] = json!([DEVICES_DOC_VERSION + 1]),
        3 => {
            doc["body"]["devices"] =
                json!([{ "id": "", "kind": "sensor", "name": "x", "adapter": "mock", "enabled": true }])
        }
        4 => {
            doc["body"]["devices"] =
                json!([{ "id": "a", "kind": "toaster", "name": "x", "adapter": "mock", "enabled": true }])
        }
        5 => {
            doc["body"]["devices"] = json!([{
                "id": "a", "kind": "sensor", "name": "x", "adapter": "mock", "enabled": true, "extra": 1
            }])
        }
        6 => {
            doc["body"]["devices"] = json!([
                { "id": "a", "kind": "sensor", "name": "x", "adapter": "mock", "enabled": true },
                { "id": "a", "kind": "other", "name": "y", "adapter": "mock", "enabled": true },
            ])
        }
        _ => doc["body"] = json!({ "devices": [], "extra": true }),
    }
    doc
}

// ============================================================================
// 参数解析（与工具的 serde 口径同形: 未知字段拒 / 类型不符拒 / null 当缺省）
// ============================================================================

/// 解析后的调用参数。
#[derive(Debug, Clone, Default)]
struct ParsedArgs {
    action: Option<String>,
    device_ids: Option<Vec<String>>,
    sensitive_read_authorized: Option<bool>,
    authorized_by: Option<String>,
}

/// 闭合词表: 工具只认这四个字段（`sense.rs:75-82` + 声明面 `additionalProperties: false`）。
const KNOWN_FIELDS: &[&str] = &[
    "action",
    "device_ids",
    "sensitive_read_authorized",
    "authorized_by",
];

/// 按 serde 的口径解析参数（`Option<T>` 字段遇 `null` = 缺省）。
///
/// `Err(())` = 解析失败（工具会拒: `sense.rs:496-501`; 边界 Guard 也会跳过:
/// `sense.rs:545-549`「解析失败不归边界 Guard」）。
fn parse_args(args: &Value) -> Result<ParsedArgs, ()> {
    let object = args.as_object().ok_or(())?;
    if object.keys().any(|key| !KNOWN_FIELDS.contains(&key.as_str())) {
        return Err(());
    }
    let action = match object.get("action") {
        None | Some(Value::Null) => None,
        Some(Value::String(text)) => Some(text.clone()),
        Some(_) => return Err(()),
    };
    let device_ids = match object.get("device_ids") {
        None | Some(Value::Null) => None,
        Some(Value::Array(items)) => {
            let mut ids = Vec::with_capacity(items.len());
            for item in items {
                ids.push(item.as_str().ok_or(())?.to_string());
            }
            Some(ids)
        }
        Some(_) => return Err(()),
    };
    let sensitive_read_authorized = match object.get("sensitive_read_authorized") {
        None | Some(Value::Null) => None,
        Some(Value::Bool(flag)) => Some(*flag),
        Some(_) => return Err(()),
    };
    let authorized_by = match object.get("authorized_by") {
        None | Some(Value::Null) => None,
        Some(Value::String(text)) => Some(text.clone()),
        Some(_) => return Err(()),
    };
    Ok(ParsedArgs {
        action,
        device_ids,
        sensitive_read_authorized,
        authorized_by,
    })
}

// ============================================================================
// 契约预期（判据的来源是 sense.rs 的注释, 不是实现的返回值）
// ============================================================================

/// 一次调用的**预期结局**（哪一道闸会拒）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Refusal {
    /// 不拒（预期放行）。
    None,
    /// 参数面拒绝（解析失败 / 未知字段 / 类型不符 / `action` 缺失或词表外）。
    Params,
    /// 设备表拒开。
    Table,
    /// 白名单闸拒绝（空 `device_ids` / 表外 id / 停用 id）。
    Whitelist,
    /// 敏感面授权缺位（拒绝即帧 + 审计行 `denied`）。
    SensitiveDenied,
    /// 审计账写不进（fail-closed: 不放行）。
    AuditUnavailable,
    /// 适配器不可用（词表外 / 真实协议插槽未接线）。
    AdapterUnavailable,
}

/// 一次调用的预期全貌。
#[derive(Debug, Clone)]
struct Plan {
    refusal: Refusal,
    /// 白名单目标（按请求顺序; 重复 id 保留重复）。
    targets: Vec<String>,
    /// 目标里的摄像头类（敏感面）。
    sensitive: Vec<String>,
    /// 授权是否成立（显式授权旗 + 非空授权人）。
    authorized: bool,
    /// 授权人（裁剪空白后）—— 审计行要如实记这个人。
    authorizer: Option<String>,
    /// 预期读到的设备序列（适配器逐台读, 遇错即止）。
    expected_reads: Vec<String>,
}

impl Plan {
    fn refused(reason: Refusal) -> Self {
        Self {
            refusal: reason,
            targets: Vec::new(),
            sensitive: Vec::new(),
            authorized: false,
            authorizer: None,
            expected_reads: Vec::new(),
        }
    }
}

/// 按**契约**推导一次调用的预期结局。
///
/// 判据出处（逐条）:
/// - 参数面: `sense.rs:74` + `sense.rs:71-72` + `sense.rs:502-504` + 声明面
///   `sense.rs:444-470`。
/// - 设备表: `sense.rs:238`。
/// - 白名单: `sense.rs:400-402` / `sense.rs:410-413` / `registry.rs:150-156`。
/// - 敏感面: `sense.rs:280-336` + `audit.rs:5-6`。
/// - 适配器: `sense.rs:338-360` + `adapter.rs:195-208`。
fn plan(table: Option<&DevicesBody>, args: &Value, audit_broken: bool) -> Plan {
    // ── ① 参数面: 参数必须能解析成闭合结构 ────────────────────────────────
    let Ok(parsed) = parse_args(args) else {
        return Plan::refused(Refusal::Params);
    };
    let Some(action) = parsed.action.as_deref() else {
        // 「必须显式给 action」(`sense.rs:502-504`)。
        return Plan::refused(Refusal::Params);
    };
    if !DEVICE_SENSE_ACTIONS.contains(&action) {
        return Plan::refused(Refusal::Params);
    }

    // ── ② 设备表: 拒开即帧, 不回退默认表 ─────────────────────────────────
    let Some(body) = table else {
        return Plan::refused(Refusal::Table);
    };

    // ── ③ `list`: 只回白名单视图, 零适配器读 ⇒ 没有任何目标 ────────────────
    if action == ACTION_LIST {
        return Plan::refused(Refusal::None);
    }

    // ── ④ 白名单目标 ──────────────────────────────────────────────────────
    let targets: Vec<&Device> = match &parsed.device_ids {
        None => body.whitelist(),
        Some(ids) => {
            if ids.is_empty() {
                return Plan::refused(Refusal::Whitelist);
            }
            let mut picked = Vec::with_capacity(ids.len());
            for id in ids {
                match body.find(id) {
                    None => return Plan::refused(Refusal::Whitelist),
                    Some(device) if !device.enabled => return Plan::refused(Refusal::Whitelist),
                    Some(device) => picked.push(device),
                }
            }
            picked
        }
    };

    let authorizer = parsed
        .authorized_by
        .as_deref()
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(str::to_string);
    let mut plan = Plan {
        refusal: Refusal::None,
        targets: targets.iter().map(|device| device.id.clone()).collect(),
        sensitive: targets
            .iter()
            .filter(|device| device.kind.is_sensitive())
            .map(|device| device.id.clone())
            .collect(),
        authorized: parsed.sensitive_read_authorized == Some(true) && authorizer.is_some(),
        authorizer,
        expected_reads: Vec::new(),
    };

    // ── ⑤ 敏感面闸: 显式授权缺位 ⇒ 拒; 审计账写不进 ⇒ 不放行 ──────────────
    //
    // 闸序（`sense.rs:308-336`）: **先落审计行, 再判授权** ⇒ 审计写不进时
    // 先死在「审计缺位不放行」, 根本走不到授权判定。
    if !plan.sensitive.is_empty() {
        if audit_broken {
            plan.refusal = Refusal::AuditUnavailable;
            return plan;
        }
        if !plan.authorized {
            plan.refusal = Refusal::SensitiveDenied;
            return plan;
        }
    }

    // ── ⑥ 适配器逐台读: 遇错即止（整帧拒绝, 不静默跳台不降级）──────────────
    for device in &targets {
        let name = device.adapter.as_str();
        if name == MOCK_ADAPTER || name == PROBE_ADAPTER {
            plan.expected_reads.push(device.id.clone());
        } else {
            // 词表外适配器 / 真实协议插槽（未接线）⇒ 拒绝即帧。
            let _reserved = RESERVED_PROTOCOL_SLOTS.contains(&name);
            plan.refusal = Refusal::AdapterUnavailable;
            return plan;
        }
    }
    plan
}

// ============================================================================
// 实测读数
// ============================================================================

/// 一次场景跑完后的实测读数。
#[derive(Debug)]
struct Observed {
    result: ToolResult,
    /// 审计账读回的行（`None` = 账读不回来, 例如坏目录）。
    rows: Option<Vec<SensitiveReadAudit>>,
    /// 适配器读到的设备序列（按读到的顺序）。
    reads: Vec<String>,
    sense_calls: u64,
    sensed_devices: u64,
    granted: u64,
    denied: u64,
    last_sense: Option<(i64, String)>,
    /// 同一本设备表 + 同一个进程内计数（自述在场节要与工具面同源）。
    registry: DeviceRegistry,
    activity: Arc<DeviceSenseActivity>,
}

/// 跑一个场景: 落表 → 建工具 → 调一次 → 收读数。
fn run_case(case: &Case) -> (Observed, Option<DevicesBody>) {
    let root = if case.broken_audit {
        broken_audit_root().clone()
    } else {
        work_root().clone()
    };
    let registry = DeviceRegistry::under_data_dir(&root);
    write_table(&registry, &case.table);

    let audit = DeviceSenseAuditLedger::under_data_dir(&root);
    if !case.broken_audit {
        // 本轮的行必须可归因 ⇒ 每次先清空审计账。
        let _ = std::fs::remove_file(audit.path());
    }

    let reads = Arc::new(Mutex::new(Vec::new()));
    let adapters = AdapterRegistry::new()
        .with_adapter(Arc::new(CountingMock {
            inner: MockAdapter::new(),
            reads: Arc::clone(&reads),
        }))
        .with_adapter(Arc::new(ProbeAdapter {
            reads: Arc::clone(&reads),
        }));
    let activity = Arc::new(DeviceSenseActivity::new());
    let tool = DeviceSenseTool::new(
        registry.clone(),
        adapters,
        audit.clone(),
        Arc::clone(&activity),
    );
    let call = ToolCall {
        id: "call-1".to_string(),
        name: DEVICE_SENSE_TOOL_NAME.to_string(),
        arguments: case.args.clone(),
    };
    let result = block_on(tool.invoke(&call));

    let rows = audit.scan().ok().map(|scan| {
        scan.records
            .into_iter()
            .map(|record| record.body)
            .collect::<Vec<SensitiveReadAudit>>()
    });
    let (granted, denied) = activity.sensitive_counts();
    let observed = Observed {
        result,
        rows,
        reads: reads
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone(),
        sense_calls: activity.sense_calls(),
        sensed_devices: activity.sensed_devices(),
        granted,
        denied,
        last_sense: activity.last_sense(),
        registry,
        activity,
    };
    // 设备表的**可开性**由注册表自己判（`registry.rs:12-15` 的拒开语义）;
    // 感知面的契约是「拒开即帧, 不回退默认表」, 不是「表一定开」。
    let table = observed.registry.load().ok();
    (observed, table)
}

// ============================================================================
// 断言
// ============================================================================

/// 断言消息里截断输入。
fn brief(value: &Value) -> String {
    let text = value.to_string();
    let head: String = text.chars().take(200).collect();
    head
}

/// 取 ok 结果的结构化值。
fn ok_value(result: &ToolResult) -> Option<&Value> {
    match &result.outcome {
        ToolOutcome::Ok { value } => Some(value),
        ToolOutcome::Error { .. } => None,
    }
}

/// 取错误结果的消息。
fn err_message(result: &ToolResult) -> Option<&str> {
    match &result.outcome {
        ToolOutcome::Ok { .. } => None,
        ToolOutcome::Error { message, .. } => Some(message.as_str()),
    }
}

/// 本次调用的动作（解析失败时 = `None`）。
fn action_of(args: &Value) -> Option<String> {
    parse_args(args).ok().and_then(|parsed| parsed.action)
}

/// 一个场景的全部断言。
fn assert_case(case: &Case, observed: &Observed, table: Option<&DevicesBody>, plan: &Plan) {
    let action = action_of(&case.args);

    // ── ⑦ 拒绝即帧 = 永久错误 + 真因非空 ────────────────────────────────
    if let Some(message) = err_message(&observed.result) {
        match &observed.result.outcome {
            ToolOutcome::Error { retryable, .. } => {
                assert!(
                    !retryable,
                    "拒绝即帧必须是**永久**错误 (sense.rs:394-397 → permanent_error): {message}"
                );
                assert!(
                    !message.trim().is_empty(),
                    "拒绝必须带真因 (sense.rs:394「真因如实, 不静默降级」)"
                );
            }
            ToolOutcome::Ok { .. } => unreachable!(),
        }
    }

    match plan.refusal {
        Refusal::None => {
            // ── ⑧ 输出归一合同: 对象 + `action` / `status` 恒在场 ──────────
            let value = ok_value(&observed.result).unwrap_or_else(|| {
                panic!(
                    "预期放行却拿到拒绝: {} (args={})",
                    err_message(&observed.result).unwrap_or(""),
                    brief(&case.args)
                )
            });
            assert!(
                DeviceSenseTool::output_schema().validate(value).is_ok(),
                "结构化输出没过自己的归一合同 (sense.rs:218-223): {value}"
            );
            let expected_action = if action.as_deref() == Some(ACTION_LIST) {
                ACTION_LIST
            } else {
                ACTION_SENSE
            };
            assert_eq!(
                value.get("action").and_then(Value::as_str),
                Some(expected_action),
                "输出的 `action` 字段必须回显本次动作 (sense.rs:258 / 379): {value}"
            );
            assert_eq!(
                value.get("status").and_then(Value::as_str),
                Some("ok"),
                "放行时 `status` 恒为 ok (sense.rs:259 / 380): {value}"
            );

            if expected_action == ACTION_LIST {
                // ── ⑨ `list` = 零适配器读 + 零审计行 + 无读数面 ───────────
                assert!(
                    observed.reads.is_empty(),
                    "`list` 不得读任何适配器 (sense.rs:66「零适配器读」): {:?}",
                    observed.reads
                );
                assert!(
                    value.get("states").is_none(),
                    "`list` 只回白名单视图, 不得带读数面 (sense.rs:191): {value}"
                );
                // 坏目录下审计账读不回来 ⇒ 那条路径归 `AuditUnavailable` 分支。
                if let Some(rows) = observed.rows.as_ref() {
                    assert_eq!(
                        rows.len(),
                        0,
                        "`list` 不是感知尝试, 不得落审计行 (audit.rs:57 审计行 action 恒为 sense)"
                    );
                }
                // `list` 报的两本账必须与设备表同口径（`registry.rs:150-163`）。
                let body = table.expect("放行时设备表必可开");
                assert_eq!(
                    value.get("devices_total").and_then(Value::as_u64),
                    Some(body.len() as u64),
                    "`list` 的设备总数必须等于表内行数 (sense.rs:261 / registry.rs:163): {value}"
                );
                assert_eq!(
                    value.get("whitelisted").and_then(Value::as_u64),
                    Some(body.whitelist().len() as u64),
                    "`list` 的白名单数必须等于表内且 enabled 的行数 (sense.rs:254 / registry.rs:150-156): {value}"
                );
                assert_eq!(
                    value.get("table_present").and_then(Value::as_bool),
                    Some(!matches!(case.table, TableSpec::Missing)),
                    "`list` 必须如实报设备表在不在 (sense.rs:263): {value}"
                );
                assert_eq!(
                    observed.sense_calls,
                    0,
                    "`list` 不是感知, 不得计感知次数 (sense.rs:370-371 只在 sense 路径记账)"
                );
            } else {
                // ── ② 放行时读到的设备必须**恰好**是白名单目标 ────────────
                let states = value
                    .get("states")
                    .and_then(Value::as_array)
                    .unwrap_or_else(|| {
                        panic!("放行时 `states` 必须是数组 (sense.rs:380-381): {value}")
                    });
                let read_ids: Vec<&str> = states
                    .iter()
                    .filter_map(|state| state.get("device_id").and_then(Value::as_str))
                    .collect();
                assert_eq!(
                    read_ids, observed.reads,
                    "读数面与适配器实读必须一一对应 (sense.rs:340-360): {read_ids:?} vs {:?}",
                    observed.reads
                );
                for id in &read_ids {
                    assert!(
                        plan.targets.contains(&id.to_string()),
                        "读到了白名单外的设备 {id:?} (底线①: 只读白名单设备): {read_ids:?}"
                    );
                }
                for target in &plan.targets {
                    assert!(
                        read_ids.contains(&target.as_str()),
                        "白名单目标 {target:?} 没被读到: {read_ids:?}"
                    );
                }
                // 无重复 id 时「白名单台数」== 「读到的台数」（`whitelisted` / `states` 同源）。
                let unique_targets: std::collections::BTreeSet<&String> =
                    plan.targets.iter().collect();
                if unique_targets.len() == plan.targets.len() {
                    assert_eq!(
                        value.get("whitelisted").and_then(Value::as_u64),
                        Some(states.len() as u64),
                        "`whitelisted` 必须等于本次感知到的台数 (sense.rs:362-368 / 382): {value}"
                    );
                }
                if states.is_empty() {
                    assert!(
                        value
                            .get("note")
                            .and_then(Value::as_str)
                            .is_some_and(|note| note.contains("白名单零设备")),
                        "零设备时必须如实说「白名单零设备」, 不编造读数 (sense.rs:362-363): {value}"
                    );
                }
                assert_eq!(
                    observed.sensed_devices,
                    states.len() as u64,
                    "感知台数累计必须等于读到的台数 (sense.rs:372-374)"
                );
                assert_eq!(
                    observed.sense_calls,
                    1,
                    "放行的感知必须记一次感知次数 (sense.rs:370-371)"
                );

                // ── ⑬ mock 档的数据源必须如实标注 ────────────────────────
                for state in states {
                    let adapter = state.get("adapter").and_then(Value::as_str).unwrap_or("");
                    if adapter == MOCK_ADAPTER {
                        assert_eq!(
                            state.get("source_note").and_then(Value::as_str),
                            Some("mock-fixture"),
                            "mock 档必须如实标注固定 fixture 数据源 (adapter.rs:53 / 166): {state}"
                        );
                    }
                }
            }

            // ── ⑤ 敏感面尝试 ⇒ 审计账逐台入账, 放行时恒 `granted` ───────────
            // 坏目录下审计账**读不回来**（`rows == None`）⇒ 那条路径由
            // `Refusal::AuditUnavailable` 分支断言（fail-closed）。
            if let Some(rows) = observed.rows.as_ref() {
                assert_eq!(
                    rows.len(),
                    plan.sensitive.len(),
                    "敏感面每次尝试都落一行审计 (audit.rs:5-6): 期望 {} 行, 实得 {} 行",
                    plan.sensitive.len(),
                    rows.len()
                );
                for row in rows {
                    assert_eq!(
                        row.outcome,
                        SensitiveReadOutcome::Granted,
                        "授权齐备时审计行必须记 `granted` (sense.rs:294-297): {row:?}"
                    );
                    assert_eq!(
                        row.action, ACTION_SENSE,
                        "审计行的动作恒为 sense (audit.rs:57): {row:?}"
                    );
                    assert_eq!(
                        row.kind,
                        DeviceKind::Camera,
                        "敏感面审计行只记摄像头类 (audit.rs:55): {row:?}"
                    );
                    assert_eq!(
                        row.authorized_by, plan.authorizer,
                        "审计行要如实记授权人 (audit.rs:59 / sense.rs:313): {row:?}"
                    );
                }
            }
            // ── ⑬ 摄像头读数永不回图像内容 ────────────────────────────────
            for state in value
                .get("states")
                .and_then(Value::as_array)
                .unwrap_or(&Vec::new())
            {
                let kind = state.get("kind").and_then(Value::as_str).unwrap_or("");
                if kind != "camera" {
                    continue;
                }
                let Some(readings) = state.get("readings").and_then(Value::as_object) else {
                    continue;
                };
                for key in readings.keys() {
                    assert!(
                        !key.contains("frame")
                            && !key.contains("image")
                            && !key.contains("jpeg"),
                        "摄像头读数不得含图像内容 (adapter.rs:128): key={key}"
                    );
                }
            }

            // ── ⑩ 进程内计数与结果同源 ──────────────────────────────────
            // (`sensitive_audit` 段只出现在 `sense` 的输出里 —— `list` 的输出
            //  按 `sense.rs:255-266` 没有这一段。)
            if expected_action == ACTION_SENSE {
                assert_eq!(
                    value["sensitive_audit"]["granted_total"].as_u64(),
                    Some(observed.granted),
                    "结果里的放行计数必须与进程内计数同源 (sense.rs:375): {value}"
                );
                assert_eq!(
                    value["sensitive_audit"]["denied_total"].as_u64(),
                    Some(observed.denied),
                    "结果里的拒绝计数必须与进程内计数同源 (sense.rs:375): {value}"
                );
            }
            assert_eq!(
                observed.granted + observed.denied,
                u64::from(!plan.sensitive.is_empty()),
                "一次调用最多记一次敏感面判定 (sense.rs:326): granted={} denied={}",
                observed.granted,
                observed.denied
            );
        }
        refusal => {
            // ── 预期拒绝 ⇒ 实得必须是拒绝 ────────────────────────────────
            assert!(
                !observed.result.is_ok(),
                "预期拒绝 ({refusal:?}) 却放行了: args={} table={:?}",
                brief(&case.args),
                case.table
            );
            // ── ⑩ 被拒的调用不得留下「最近感知」 ─────────────────────────
            assert_eq!(
                observed.sense_calls, 0,
                "被拒的调用不得计感知次数 (sense.rs:370-371 只在放行路径记账)"
            );
            assert!(
                observed.last_sense.is_none(),
                "被拒的调用不得留下「最近感知」记录"
            );
            match refusal {
                Refusal::Params | Refusal::Table | Refusal::Whitelist => {
                    // 这三道闸都在敏感面闸之前 ⇒ 没有审计行, 没有设备读。
                    if let Some(rows) = observed.rows.as_ref() {
                        assert_eq!(
                            rows.len(),
                            0,
                            "在敏感面闸之前被拒的调用不得落审计行 (audit.rs:5-6 的范围是敏感面尝试)"
                        );
                    }
                    assert!(
                        observed.reads.is_empty(),
                        "在敏感面闸之前被拒的调用不得读任何设备 (sense.rs:280-360 的闸序)"
                    );
                }
                Refusal::SensitiveDenied => {
                    // ── ④ ⑤ 授权缺位 ⇒ 拒绝即帧 + 逐台审计行 `denied` ───────
                    let rows = observed.rows.as_ref().expect("正常目录的审计账必须读得回来");
                    assert_eq!(
                        rows.len(),
                        plan.sensitive.len(),
                        "敏感面每次尝试都落一行审计 (audit.rs:5-6): 期望 {} 行, 实得 {} 行",
                        plan.sensitive.len(),
                        rows.len()
                    );
                    for row in rows {
                        assert_eq!(
                            row.outcome,
                            SensitiveReadOutcome::Denied,
                            "授权缺位时审计行必须记 `denied` (sense.rs:299-305): {row:?}"
                        );
                        assert_eq!(row.action, ACTION_SENSE, "审计行动作恒 sense (audit.rs:57)");
                        assert_eq!(row.kind, DeviceKind::Camera, "敏感面审计行只记摄像头类");
                    }
                    assert!(
                        observed.reads.is_empty(),
                        "敏感面被拒时不得读任何设备 (拒绝即帧 ⇒ 不静默降级): {:?}",
                        observed.reads
                    );
                    assert_eq!(observed.granted, 0, "被拒的敏感面不得记放行");
                    assert!(
                        observed.denied >= 1,
                        "被拒的敏感面必须记一次拒绝 (sense.rs:326)"
                    );
                }
                Refusal::AuditUnavailable => {
                    // ── ⑥ 审计账写不进 ⇒ 不放行, 且不得读设备 ────────────────
                    assert!(
                        observed.reads.is_empty(),
                        "审计缺位时不得读任何设备 (fail-closed ⇒ 不放行): {:?}",
                        observed.reads
                    );
                    assert_eq!(observed.granted, 0, "审计缺位时不得记放行");
                }
                Refusal::AdapterUnavailable => {
                    // 适配器不可用 ⇒ 读到坏的那台为止（整帧拒绝）。
                    assert_eq!(
                        observed.reads, plan.expected_reads,
                        "适配器遇错即止 (sense.rs:338-359「不静默跳台不降级」): {:?} vs {:?}",
                        observed.reads, plan.expected_reads
                    );
                    if let Some(rows) = observed.rows.as_ref() {
                        assert_eq!(
                            rows.len(),
                            plan.sensitive.len(),
                            "授权已落账后才死在适配器上 ⇒ 审计行数 = 敏感目标数"
                        );
                        for row in rows {
                            assert_eq!(
                                row.outcome,
                                SensitiveReadOutcome::Granted,
                                "授权齐备时审计行必须记 `granted` (sense.rs:294-297): {row:?}"
                            );
                        }
                    }
                }
                Refusal::None => {
                    unreachable!("`Refusal::None` 走的是放行分支, 不会落到这里")
                }
            }
        }
    }

    // ── ⑩ 自述在场节与工具面同表同计数 ──────────────────────────────────
    let presence = device_presence_status(&observed.registry, &observed.activity);
    match table {
        None => {
            assert!(
                presence.is_err(),
                "设备表拒开时自述在场节必须报 Err(真因) (sense.rs:160「设备表拒开 = Err(真因)」)"
            );
        }
        Some(body) => {
            let status = presence.expect("表可开时自述在场节必须给真值");
            assert_eq!(
                status.devices,
                Some(body.len() as u64),
                "自述在场节的设备数必须与表内行数同源 (registry.rs:163 / sense.rs:180)"
            );
            assert_eq!(
                status.whitelisted,
                Some(body.whitelist().len() as u64),
                "自述在场节的白名单数必须与白名单口径同源 (registry.rs:150-156 / sense.rs:181)"
            );
            match (&observed.last_sense, plan.refusal) {
                (Some((at_ms, note)), Refusal::None) => {
                    assert_eq!(
                        status.last_sense_at_ms,
                        Some(*at_ms),
                        "自述在场节的「最近感知时刻」必须与进程内计数同源 (sense.rs:27 / 168-171)"
                    );
                    assert_eq!(
                        status.last_sense_note.as_deref(),
                        Some(note.as_str()),
                        "自述在场节的「最近感知摘要」必须与进程内计数同源 (sense.rs:27 / 168-171)"
                    );
                    // 摘要必须与本次放行的结果同源（同一本账）。
                    if let Some(value) = ok_value(&observed.result) {
                        assert_eq!(
                            value.get("note").and_then(Value::as_str),
                            Some(note.as_str()),
                            "「最近感知摘要」必须就是本次感知记下的那句 (sense.rs:371)"
                        );
                    }
                }
                (None, _) => {
                    assert_eq!(status.last_sense_at_ms, None, "尚无感知不编时间");
                    assert_eq!(status.last_sense_note, None, "尚无感知不编备注");
                }
                (Some(_), _) => {
                    panic!("被拒的调用不得留下「最近感知」记录")
                }
            }
        }
    }
}

/// 判据 ⑪: 边界 Guard 单调（只拒不放, 拒了不得被翻回放行）。
fn assert_guard(case: &Case, observed: &Observed, plan: &Plan) {
    let guard = DeviceSenseBoundaryGuard::new();
    let capability = CapabilityId::new(DEVICE_SENSE_CAPABILITY_ID).expect("valid capability id");
    let request = ToolGuardRequest::new(&capability, DEVICE_SENSE_TOOL_NAME, &case.args);
    let denial = guard.deny(&request);

    // 预期: 仅当「参数能解析 ∧ action=sense ∧ 授权旗=true ∧ 授权人为空」时拒
    // (`sense.rs:550-564`)。
    let expected = match parse_args(&case.args) {
        Err(()) => false,
        Ok(parsed) => {
            parsed.action.as_deref() == Some(ACTION_SENSE)
                && parsed.sensitive_read_authorized == Some(true)
                && parsed
                    .authorized_by
                    .as_deref()
                    .map(str::trim)
                    .unwrap_or_default()
                    .is_empty()
        }
    };
    assert_eq!(
        denial.is_some(),
        expected,
        "边界 Guard 的判定必须与契约一致 (sense.rs:550-564): args={}",
        brief(&case.args)
    );

    // 只拒不放: 别的能力一概不拒 (`sense.rs:542-544`)。
    let other = CapabilityId::new("tool.other_face").expect("valid capability id");
    let other_request = ToolGuardRequest::new(&other, DEVICE_SENSE_TOOL_NAME, &case.args);
    assert!(
        guard.deny(&other_request).is_none(),
        "边界 Guard 只管本能力 (sense.rs:542-544)"
    );

    // 拒了不得被后续阶段翻回放行 (`sense.rs:520`「任何后续阶段都不能翻回放行」)。
    if denial.is_some() {
        let sensitive_read = observed
            .reads
            .iter()
            .any(|id| plan.sensitive.contains(id));
        assert!(
            !sensitive_read,
            "Guard 拒了之后不得仍读到摄像头类设备 (sense.rs:520): {:?}",
            observed.reads
        );
        if let Some(rows) = observed.rows.as_ref() {
            let granted = rows
                .iter()
                .any(|row| row.outcome == SensitiveReadOutcome::Granted);
            assert!(
                !granted,
                "Guard 拒了之后不得留下放行审计行 (sense.rs:520)"
            );
        }
    }
}

// ============================================================================
// 参数构造（把 fuzz 字节变成「模型会回传的参数」）
// ============================================================================

/// 构造一份**类型乱七八糟**的参数（每个字段都可能是缺省 / 对类型 / 越界值）。
///
/// 覆盖判据 ① 的各条拒绝路径: 未知字段 / 类型不符 / `action` 缺失 / 词表外。
fn args_fuzzed(payload: &[u8]) -> Value {
    let mut object = serde_json::Map::new();
    let head = payload.first().copied().unwrap_or(0);

    // `action`: 词表内 / 词表外 / 空 / 任意字节 / 缺省 / 非字符串。
    match head % 6 {
        0 => {
            object.insert("action".into(), json!(ACTION_LIST));
        }
        1 => {
            object.insert("action".into(), json!(ACTION_SENSE));
        }
        2 => {
            object.insert("action".into(), json!("SENSE"));
        }
        3 => {
            object.insert("action".into(), json!(""));
        }
        4 => {
            object.insert("action".into(), json!(String::from_utf8_lossy(payload).into_owned()));
        }
        _ => {
            object.insert("action".into(), json!(7));
        }
    }

    // `device_ids`: 空数组 / 表外 id / 非字符串元素 / 不是数组 / 缺省。
    match payload.get(1).copied().unwrap_or(0) % 5 {
        0 => {
            object.insert("device_ids".into(), json!([]));
        }
        1 => {
            object.insert("device_ids".into(), json!(["no-such-device"]));
        }
        2 => {
            object.insert("device_ids".into(), json!([7]));
        }
        3 => {
            object.insert("device_ids".into(), json!("thermo-1"));
        }
        _ => {}
    }

    // 授权面: 旗 / 人 各自取「缺省 / 对 / 空白 / 非布尔」四种形态。
    match payload.get(2).copied().unwrap_or(0) % 4 {
        0 => {
            object.insert("sensitive_read_authorized".into(), json!(true));
            object.insert("authorized_by".into(), json!("owner"));
        }
        1 => {
            object.insert("sensitive_read_authorized".into(), json!(true));
            object.insert("authorized_by".into(), json!("   "));
        }
        2 => {
            object.insert("sensitive_read_authorized".into(), json!("yes"));
        }
        _ => {
            object.insert("sensitive_read_authorized".into(), json!(false));
            object.insert("authorized_by".into(), json!("owner"));
        }
    }

    // 未知字段（判据 ① 的「未知字段拒绝」）。
    if payload.get(3).copied().unwrap_or(0) % 4 == 0 {
        object.insert("unknown_field".into(), json!("x"));
    }

    Value::Object(object)
}

/// 构造一份**贴近真实**的参数（用设备表里真实存在的 id）。
///
/// 覆盖判据 ② / ④ 的白名单与授权路径: 命中启用设备 / 命中停用设备 /
/// 表外 id / 重复 id × 授权齐备 / 旗开人空 / 人空白 / 旗假。
fn args_realistic(devices: &[Device], payload: &[u8]) -> Value {
    let mut object = serde_json::Map::new();
    let action = if payload.first().copied().unwrap_or(0) % 2 == 0 {
        ACTION_LIST
    } else {
        ACTION_SENSE
    };
    object.insert("action".into(), json!(action));
    if action == ACTION_SENSE {
        let enabled: Vec<&Device> = devices.iter().filter(|device| device.enabled).collect();
        let disabled: Vec<&Device> = devices.iter().filter(|device| !device.enabled).collect();
        let mut ids: Vec<String> = Vec::new();
        match payload.get(1).copied().unwrap_or(0) % 4 {
            0 => {
                if let Some(device) = enabled.first() {
                    ids.push(device.id.clone());
                }
            }
            1 => {
                if let Some(device) = disabled.first() {
                    ids.push(device.id.clone());
                }
            }
            2 => {
                ids.push("no-such-device".to_string());
            }
            _ => {
                if let Some(device) = enabled.first() {
                    ids.push(device.id.clone());
                    ids.push(device.id.clone());
                }
            }
        }
        object.insert("device_ids".into(), json!(ids));
        match payload.get(2).copied().unwrap_or(0) % 4 {
            0 => {
                object.insert("sensitive_read_authorized".into(), json!(true));
                object.insert("authorized_by".into(), json!("owner"));
            }
            1 => {
                object.insert("sensitive_read_authorized".into(), json!(true));
            }
            2 => {
                object.insert("sensitive_read_authorized".into(), json!(true));
                object.insert("authorized_by".into(), json!("  "));
            }
            _ => {
                object.insert("sensitive_read_authorized".into(), json!(false));
                object.insert("authorized_by".into(), json!("owner"));
            }
        }
    }
    Value::Object(object)
}

// ============================================================================
// 确定性夹具（把「随机输入未必造得出」的契约一次性钉住）
// ============================================================================

static FIXTURES_RUN: AtomicBool = AtomicBool::new(false);

fn fixture_suite() {
    let root = work_root().clone();
    let registry = DeviceRegistry::under_data_dir(&root);
    let audit = DeviceSenseAuditLedger::under_data_dir(&root);
    let activity = Arc::new(DeviceSenseActivity::new());
    let tool = DeviceSenseTool::new(
        registry.clone(),
        AdapterRegistry::builtin(),
        audit.clone(),
        Arc::clone(&activity),
    );

    // (a) 声明面: 名字 + 闭合动作词表 + 必填 `action` + 禁止未知字段
    //     (`sense.rs:52` / `sense.rs:71-72` / `sense.rs:468-469`)。
    let declaration = tool.declaration();
    assert_eq!(
        declaration.name, DEVICE_SENSE_TOOL_NAME,
        "声明名必须是 device_sense (sense.rs:52)"
    );
    let parameters = &declaration.parameters;
    assert_eq!(
        parameters.get("required"),
        Some(&json!(["action"])),
        "声明里 `action` 必填 (sense.rs:468): {parameters:?}"
    );
    assert_eq!(
        parameters.get("additionalProperties"),
        Some(&json!(false)),
        "声明里禁止未知字段 (sense.rs:469): {parameters:?}"
    );
    let declared_enum: Vec<&str> = parameters["properties"]["action"]["enum"]
        .as_array()
        .expect("声明里的 action 必须是枚举")
        .iter()
        .filter_map(Value::as_str)
        .collect();
    assert_eq!(
        declared_enum,
        DEVICE_SENSE_ACTIONS.to_vec(),
        "声明里的动作词表必须等于闭合词表 (sense.rs:71-72)"
    );
    let declared: Vec<&str> = parameters["properties"]
        .as_object()
        .expect("声明里必须有 properties")
        .keys()
        .map(String::as_str)
        .collect();
    for field in KNOWN_FIELDS {
        assert!(
            declared.contains(field),
            "声明里必须有字段 {field} (sense.rs:444-467): {declared:?}"
        );
    }
    assert_eq!(
        declared.len(),
        KNOWN_FIELDS.len(),
        "声明里不得有多余字段 (additionalProperties: false): {declared:?}"
    );

    // (b) 输出归一合同: `action` / `status` 恒在场 (`sense.rs:218-223`)。
    let schema = DeviceSenseTool::output_schema();
    let required: Vec<(&str, &str)> = schema
        .fields()
        .iter()
        .map(|field| (field.name.as_str(), field.kind.label()))
        .collect();
    assert!(
        required.contains(&("action", "string")) && required.contains(&("status", "string")),
        "输出归一合同必须要求 action/status 两个字符串 (sense.rs:219-222): {required:?}"
    );

    // (c) 边界 Guard: 名字 + 只管本能力 + 「旗开人空必拒」
    //     (`sense.rs:537-544` / `sense.rs:550-564`)。
    let guard = DeviceSenseBoundaryGuard::new();
    assert_eq!(
        guard.name(),
        "device_sense_boundary",
        "边界 Guard 的稳定名 (sense.rs:537-539)"
    );
    let capability = CapabilityId::new(DEVICE_SENSE_CAPABILITY_ID).expect("valid capability id");
    let other = CapabilityId::new("tool.other_face").expect("valid capability id");
    for args in [
        json!({ "action": "sense", "sensitive_read_authorized": true, "authorized_by": "owner" }),
        json!({ "action": "list" }),
    ] {
        let request = ToolGuardRequest::new(&other, DEVICE_SENSE_TOOL_NAME, &args);
        assert!(
            guard.deny(&request).is_none(),
            "边界 Guard 只管本能力 (sense.rs:542-544)"
        );
    }
    for dishonest in [
        json!({ "action": "sense", "sensitive_read_authorized": true, "authorized_by": "   " }),
        json!({ "action": "sense", "sensitive_read_authorized": true }),
    ] {
        let request = ToolGuardRequest::new(&capability, DEVICE_SENSE_TOOL_NAME, &dishonest);
        assert!(
            guard.deny(&request).is_some(),
            "开了授权旗却没给授权人必须被拒 (sense.rs:550-564): {dishonest}"
        );
    }

    // (d) 开关 fail-closed: 只认显式 `"1"` (`sense.rs:60-63` / `sense.rs:642-650`)。
    let original = std::env::var(DEVICE_SENSE_ENABLE_ENV).ok();
    for (value, expected) in [
        ("1", true),
        (" 1 ", true),
        ("1 ", true),
        ("0", false),
        ("true", false),
        ("on", false),
        ("", false),
        ("01", false),
        ("１", false),
    ] {
        // SAFETY: 单线程夹具（libFuzzer 同一时刻只跑一个输入）, 进程内没有别的
        // 线程读这个环境变量 ⇒ 不存在「读的同时被改」的未定义行为。
        unsafe { std::env::set_var(DEVICE_SENSE_ENABLE_ENV, value) };
        assert_eq!(
            device_sense_enabled_from_env(),
            expected,
            "开关口径不对: {value:?} (sense.rs:60-63)"
        );
        assert_eq!(
            DeviceSenseConfig::from_env(".").is_some(),
            expected,
            "开关口径不对: {value:?} (sense.rs:637-639)"
        );
    }
    // SAFETY: 同上（此时进程内仍无其它线程读该变量）。
    unsafe { std::env::remove_var(DEVICE_SENSE_ENABLE_ENV) };
    assert!(
        !device_sense_enabled_from_env(),
        "未设 = 关 (fail-closed) (sense.rs:644-646)"
    );
    match original {
        Some(value) => {
            // SAFETY: 同上。
            unsafe { std::env::set_var(DEVICE_SENSE_ENABLE_ENV, value) };
        }
        None => {
            // SAFETY: 同上。
            unsafe { std::env::remove_var(DEVICE_SENSE_ENABLE_ENV) };
        }
    }

    // (e) 缺省表 = 默认零设备 (`sense.rs:5-6` / `registry.rs:242-243` /
    //     `sense.rs:362-363`)。
    let _ = std::fs::remove_file(registry.path());
    let list = block_on(tool.invoke(&ToolCall {
        id: "fixture-list".to_string(),
        name: DEVICE_SENSE_TOOL_NAME.to_string(),
        arguments: json!({ "action": ACTION_LIST }),
    }));
    let value = ok_value(&list).expect("缺省表的 list 必须放行");
    assert_eq!(
        value.get("devices_total").and_then(Value::as_u64),
        Some(0),
        "缺省表 = 默认零设备 (registry.rs:242-243)"
    );
    assert_eq!(
        value.get("whitelisted").and_then(Value::as_u64),
        Some(0),
        "缺省表 = 默认零设备"
    );
    assert_eq!(
        value.get("table_present").and_then(Value::as_bool),
        Some(false),
        "缺省表必须如实报不在 (sense.rs:263)"
    );
    let sense = block_on(tool.invoke(&ToolCall {
        id: "fixture-sense".to_string(),
        name: DEVICE_SENSE_TOOL_NAME.to_string(),
        arguments: json!({ "action": ACTION_SENSE }),
    }));
    let value = ok_value(&sense).expect("缺省表的 sense 必须放行");
    assert_eq!(
        value.get("states").and_then(Value::as_array).map(Vec::len),
        Some(0),
        "缺省表没有可感知的设备"
    );
    assert!(
        value
            .get("note")
            .and_then(Value::as_str)
            .is_some_and(|note| note.contains("白名单零设备")),
        "零设备必须如实说零设备, 不编造读数 (sense.rs:362-363): {value}"
    );
}

// ============================================================================
// fuzz 主体
// ============================================================================

fuzz_target!(|data: &[u8]| {
    if !FIXTURES_RUN.swap(true, Ordering::Relaxed) {
        fixture_suite();
    }

    let bytes = &data[..data.len().min(4096)];
    let flags = bytes.first().copied().unwrap_or(0);
    let payload = bytes.get(1..).unwrap_or(&[]);

    // ── 设备表: 四种形态轮着来（缺省 / 合法 / 变异信封 / 任意字节）─────────
    let devices = build_devices(payload);
    let table = match flags % 4 {
        0 => TableSpec::Missing,
        1 => TableSpec::Valid(devices.clone()),
        2 => TableSpec::Json(mutate_envelope(&devices, payload)),
        _ => TableSpec::Raw(payload.to_vec()),
    };

    // ── 场景: 每轮跑几组参数（同一张表 + 不同参数）────────────────────────
    let mut cases: Vec<Case> = Vec::new();
    cases.push(Case {
        table: table.clone(),
        args: args_fuzzed(payload),
        broken_audit: false,
    });
    if let Ok(parsed) = serde_json::from_slice::<Value>(payload) {
        cases.push(Case {
            table: table.clone(),
            args: parsed,
            broken_audit: false,
        });
    }
    cases.push(Case {
        table: table.clone(),
        args: args_realistic(&devices, payload),
        broken_audit: false,
    });
    // 「审计账写不进」的坏目录（判据 ⑥ 的 fail-closed 分支）。
    cases.push(Case {
        table: table.clone(),
        args: args_realistic(&devices, payload),
        broken_audit: true,
    });

    for case in &cases {
        let (observed, table_body) = run_case(case);
        let expected = plan(table_body.as_ref(), &case.args, case.broken_audit);
        assert_case(case, &observed, table_body.as_ref(), &expected);
        assert_guard(case, &observed, &expected);
    }
});
