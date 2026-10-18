//! 设备适配器面: 读状态的插槽合同 + mock 适配器 (固定 fixture) + 真实协议插槽。
//!
//! # 适配器插槽图 (留插槽不接)
//!
//! ```text
//! tool.device_sense ──▶ AdapterRegistry::resolve(adapter 名)
//!                          │
//!                          ├─ "mock"          → MockAdapter       ✅ 已接 (固定 fixture)
//!                          ├─ "local_bridge"  → 本地桥协议族插槽  ⛔ 留插槽不接
//!                          ├─ "cloud_bridge"  → 云桥协议族插槽    ⛔ 留插槽不接
//!                          ├─ "direct"        → 直连协议族插槽    ⛔ 留插槽不接
//!                          └─ 其它名          → 拒绝即帧 (未知适配器)
//!
//! 真实协议接入后置 (K 轴 L1 骨架波): 协议族三形态 (本地桥 / 云桥 / 直连) 只留
//! 插槽名与 [`DeviceAdapter`] 合同, **不接实现** —— 解析到插槽名 = 结构化
//! [`AdapterError::SlotNotWired`] 拒绝 (0 装诚实: 未接线就说未接线)。
//! 未来接线走 [`AdapterRegistry::with_adapter`] 注册真实适配器, 白名单语义
//! 与敏感面审计不变。

use std::collections::BTreeMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::registry::{Device, DeviceKind};

/// mock 适配器名 (内置, 固定 fixture 数据)。
pub const MOCK_ADAPTER: &str = "mock";

/// 真实协议族三形态的**留位插槽名** (家居协议族: 本地桥 / 云桥 / 直连)。
///
/// 这些名字在 [`AdapterRegistry::resolve`] 里显式报
/// [`AdapterError::SlotNotWired`] —— 插槽留着, 实现不接 (真实协议接入后置)。
pub const RESERVED_PROTOCOL_SLOTS: &[&str] = &["local_bridge", "cloud_bridge", "direct"];

/// 一次设备感知读数 (适配器输出)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeviceState {
    /// 设备 id (与设备表行配对)。
    pub device_id: String,
    /// 设备类别 (来自设备表)。
    pub kind: DeviceKind,
    /// 实际出数的适配器名。
    pub adapter: String,
    /// 在线态 (适配器如实报; mock 档恒 `true` 并注明 fixture)。
    pub online: bool,
    /// 读数 (键值; 空 = 该类目 mock fixture 无读数)。
    pub readings: BTreeMap<String, Value>,
    /// 观测时刻 (epoch 毫秒)。
    pub observed_at_ms: i64,
    /// 数据源如实标注 (`mock-fixture` / `live:<适配器名>`)。
    pub source_note: String,
}

/// 适配器错误 (闭合词表: 插槽未接线 / 未知适配器 / 读失败)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdapterError {
    /// 真实协议插槽**留位不接** (0 装诚实: 显式未接线, 不冒充)。
    SlotNotWired {
        /// 请求的适配器名 (插槽名)。
        adapter: String,
    },
    /// 未知适配器名 (词表外)。
    UnknownAdapter {
        /// 请求的适配器名。
        adapter: String,
    },
    /// 适配器读失败 (真因如实)。
    ReadFailed {
        /// 出数的适配器名。
        adapter: String,
        /// 真因。
        reason: String,
    },
}

impl std::fmt::Display for AdapterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SlotNotWired { adapter } => write!(
                f,
                "适配器 {adapter} 是真实协议插槽 (留插槽不接): 未接线, 不出数不冒充"
            ),
            Self::UnknownAdapter { adapter } => write!(
                f,
                "未知适配器 {adapter}; 已接词表 = mock (真实协议插槽未接线)"
            ),
            Self::ReadFailed { adapter, reason } => write!(f, "适配器 {adapter} 读失败: {reason}"),
        }
    }
}

impl std::error::Error for AdapterError {}

/// 设备适配器合同: 按设备读一次状态 (读面, 零写零执行)。
pub trait DeviceAdapter: Send + Sync {
    /// 适配器稳定名 (与设备表 `adapter` 列配对)。
    fn name(&self) -> &str;

    /// 读一台设备的状态 (`now_ms` = 观测时刻, 由调用方注入同一时钟)。
    fn read_state(&self, device: &Device, now_ms: i64) -> Result<DeviceState, AdapterError>;
}

/// mock 适配器 (内置): **固定 fixture 数据**, 确定性读数, 恒标注 `mock-fixture`。
///
/// fixture 按设备类别给固定值 (见 [`MockAdapter::fixture_readings`]) —— 同一台
/// 设备每次读到同样的数, 测试可钉真值; 这不是实测世界, 是骨架的可测替身。
pub struct MockAdapter;

impl MockAdapter {
    /// mock 适配器 (固定 fixture)。
    pub fn new() -> Self {
        Self
    }

    /// 固定 fixture 读数 (按类别; 确定性, 不含时变量)。
    pub fn fixture_readings(kind: DeviceKind) -> BTreeMap<String, Value> {
        let mut readings = BTreeMap::new();
        match kind {
            DeviceKind::Sensor => {
                readings.insert("temperature_c".to_string(), Value::from(21.5));
                readings.insert("humidity_pct".to_string(), Value::from(48.0));
                readings.insert("illuminance_lx".to_string(), Value::from(120.0));
            }
            DeviceKind::Camera => {
                // 摄像头类 fixture 只回**存在性/事件位**, 永不回图像内容。
                readings.insert("motion_detected".to_string(), Value::from(false));
                readings.insert(
                    "last_motion_at_ms".to_string(),
                    Value::from(1_700_000_000_000_i64),
                );
            }
            DeviceKind::Appliance => {
                readings.insert("switched_on".to_string(), Value::from(false));
                readings.insert("power_w".to_string(), Value::from(0.0));
            }
            DeviceKind::Other => {
                readings.insert("heartbeat".to_string(), Value::from(true));
            }
        }
        readings
    }
}

impl Default for MockAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl DeviceAdapter for MockAdapter {
    fn name(&self) -> &str {
        MOCK_ADAPTER
    }

    fn read_state(&self, device: &Device, now_ms: i64) -> Result<DeviceState, AdapterError> {
        Ok(DeviceState {
            device_id: device.id.clone(),
            kind: device.kind,
            adapter: MOCK_ADAPTER.to_string(),
            online: true,
            readings: Self::fixture_readings(device.kind),
            observed_at_ms: now_ms,
            source_note: "mock-fixture".to_string(),
        })
    }
}

/// 适配器注册表: `adapter 名 → 实现` 的解析面 (插槽图的唯一实现)。
#[derive(Clone, Default)]
pub struct AdapterRegistry {
    adapters: BTreeMap<String, Arc<dyn DeviceAdapter>>,
}

impl AdapterRegistry {
    /// 空注册表 (只认 [`Self::with_adapter`] 注册过的实现)。
    pub fn new() -> Self {
        Self::default()
    }

    /// 内置注册表: 只接 [`MockAdapter`] (真实协议插槽留位不接)。
    pub fn builtin() -> Self {
        Self::new().with_adapter(Arc::new(MockAdapter::new()))
    }

    /// 注册一个适配器实现 (真实协议接入的**插槽**: 未来接线走这里)。
    #[must_use]
    pub fn with_adapter(mut self, adapter: Arc<dyn DeviceAdapter>) -> Self {
        self.adapters.insert(adapter.name().to_string(), adapter);
        self
    }

    /// 解析适配器: 已接实现直出; 留位插槽名 = [`AdapterError::SlotNotWired`];
    /// 词表外 = [`AdapterError::UnknownAdapter`]。
    pub fn resolve(&self, name: &str) -> Result<Arc<dyn DeviceAdapter>, AdapterError> {
        if let Some(adapter) = self.adapters.get(name) {
            return Ok(Arc::clone(adapter));
        }
        if RESERVED_PROTOCOL_SLOTS.contains(&name) {
            return Err(AdapterError::SlotNotWired {
                adapter: name.to_string(),
            });
        }
        Err(AdapterError::UnknownAdapter {
            adapter: name.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::registry::Device;

    fn device(id: &str, kind: DeviceKind, adapter: &str) -> Device {
        Device {
            id: id.to_string(),
            kind,
            name: format!("设备 {id}"),
            adapter: adapter.to_string(),
            enabled: true,
        }
    }

    #[test]
    fn mock_fixture_is_deterministic_per_kind() {
        let adapter = MockAdapter::new();
        let sensor = device("thermo-1", DeviceKind::Sensor, MOCK_ADAPTER);
        let first = adapter.read_state(&sensor, 1_700_000_000_000).unwrap();
        let second = adapter.read_state(&sensor, 1_700_000_000_123).unwrap();
        assert_eq!(first.readings, second.readings, "固定 fixture 每次同值");
        assert_eq!(first.readings["temperature_c"], Value::from(21.5));
        assert_eq!(first.source_note, "mock-fixture");
        assert_eq!(first.observed_at_ms, 1_700_000_000_000);
        assert_eq!(second.observed_at_ms, 1_700_000_000_123, "观测时刻如实");
    }

    #[test]
    fn camera_fixture_never_returns_image_content() {
        let adapter = MockAdapter::new();
        let camera = device("cam-1", DeviceKind::Camera, MOCK_ADAPTER);
        let state = adapter.read_state(&camera, 0).unwrap();
        for key in state.readings.keys() {
            assert!(
                !key.contains("frame") && !key.contains("image") && !key.contains("jpeg"),
                "摄像头 fixture 只回事件位, 不回图像内容: {key}"
            );
        }
    }

    #[test]
    fn reserved_protocol_slots_refuse_until_wired() {
        let registry = AdapterRegistry::builtin();
        for slot in RESERVED_PROTOCOL_SLOTS {
            // 跨线机械补位 (device_control 批报备): `expect_err` 要求 Ok 侧 Debug,
            // 插槽句柄没有 Debug —— 改 let-else 同义断言 (必须留位不接)。
            let Err(error) = registry.resolve(slot) else {
                panic!("{slot} 插槽必须留位不接");
            };
            assert!(
                matches!(error, AdapterError::SlotNotWired { .. }),
                "{slot} 必须报未接线: {error}"
            );
        }
        assert!(matches!(
            registry.resolve("toaster-os"),
            Err(AdapterError::UnknownAdapter { .. })
        ));
        assert!(registry.resolve(MOCK_ADAPTER).is_ok());
    }
}
