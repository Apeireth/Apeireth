//! 设备注册表 (`devices.json`, StoredDoc 拒开语义) —— 白名单制, 默认零设备。
//!
//! # 四条安全底线之①: 设备白名单
//!
//! 能碰的设备**显式登记 + 显式启用**, 默认零设备: `devices.json` 缺省 = 空表
//! (没有任何设备可感知), 不是"全都能碰"。读面只认白名单
//! ([`DevicesBody::whitelist`] = 表内且 `enabled`), 表外/停用设备一律
//! 拒绝即帧 (见 [`crate::device::sense`])。
//!
//! # StoredDoc 拒开语义
//!
//! `devices.json` 落盘为 [`stored_doc::StoredDoc`] 信封 JSON: 身份 / 版本戳 /
//! 主体。读端**严格打开**: 畸形 JSON / 串档身份 / 未来版本 / 坏条目
//! (重复 id / 空 id / 词表外 kind) 一律 [`DeviceStoreError`] 拒开, 不静默
//! 迁移、不回退默认表 —— 坏表拒开也拒写 (不带病落盘)。
//!
//! # 设备表 schema (稳定合同)
//!
//! 见 [`devices_json_schema`] (JSON Schema 形状的单一真源, 测试钉住字段)。

use std::path::{Path, PathBuf};

use apeireth_core::stored_doc::{self, DocCompat, StoredDocError};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// 设备表文件 (数据目录落点: `<data_dir>/devices.json`)。
pub const DEVICES_FILE: &str = "devices.json";

/// 设备表 StoredDoc 的身份戳 (串档打开即拒)。
pub const DEVICES_DOC_NAME: &str = "device-registry";

/// 设备表 StoredDoc 的格式版本戳 (读端只认当前版本)。
pub const DEVICES_DOC_VERSION: u32 = 1;

/// 设备类别闭合词表: `sensor` / `camera` / `appliance` / `other`。
///
/// [`DeviceKind::Camera`] 是**敏感读面** (谁在家类目), 读取要显式授权 + 审计
/// (见 [`crate::device::audit`] 与 [`crate::device::sense`])。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeviceKind {
    /// 传感器 (温度/湿度/光照等环境读数)。
    Sensor,
    /// 摄像头类 (视觉读面, 敏感)。
    Camera,
    /// 家电类 (灯/空调/微波炉等; 本波只感知不执行)。
    Appliance,
    /// 其它 (词表兜底, 非敏感读面)。
    Other,
}

impl DeviceKind {
    /// 稳定标签 (`sensor` / `camera` / `appliance` / `other`)。
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Sensor => "sensor",
            Self::Camera => "camera",
            Self::Appliance => "appliance",
            Self::Other => "other",
        }
    }

    /// 闭合词表解析 (词表外 = `None`; 落表时词表外 = 拒开)。
    pub fn parse(text: &str) -> Option<Self> {
        match text.trim() {
            "sensor" => Some(Self::Sensor),
            "camera" => Some(Self::Camera),
            "appliance" => Some(Self::Appliance),
            "other" => Some(Self::Other),
            _ => None,
        }
    }

    /// 敏感读面判定: 摄像头类 = 敏感 (显式授权 + 审计)。
    pub const fn is_sensitive(self) -> bool {
        matches!(self, Self::Camera)
    }
}

/// 一台登记设备 (设备表的一行)。
///
/// 字段合同 (稳定): `{id, kind, name, adapter, enabled}` —— 未知字段拒开
/// (`deny_unknown_fields`), 不做宽容解析。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Device {
    /// 设备稳定 id (表内唯一, 非空)。
    pub id: String,
    /// 设备类别 ([`DeviceKind`] 闭合词表)。
    pub kind: DeviceKind,
    /// 人读名 (非空)。
    pub name: String,
    /// 适配器名 (解析见 [`crate::device::adapter::AdapterRegistry`]; 非空)。
    pub adapter: String,
    /// 白名单启用位: `true` = 在白名单内可感知, `false` = 登记但不碰。
    pub enabled: bool,
}

/// 设备表主体 (`devices.json` 的 StoredDoc body)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DevicesBody {
    /// 设备行 (可空 = 默认零设备)。
    pub devices: Vec<Device>,
}

impl Default for DevicesBody {
    fn default() -> Self {
        Self::new()
    }
}

impl DevicesBody {
    /// 空表 (默认零设备)。
    pub fn new() -> Self {
        Self {
            devices: Vec::new(),
        }
    }

    /// 表体校验 (纯判定): 重复 id / 空 id / 空名 / 空适配器名 = 拒开拒写。
    pub fn validate(&self) -> Result<(), DeviceStoreError> {
        let mut seen = std::collections::BTreeSet::new();
        for device in &self.devices {
            if device.id.trim().is_empty() {
                return Err(DeviceStoreError::Invalid {
                    reason: "设备 id 不能为空".to_string(),
                });
            }
            if device.name.trim().is_empty() {
                return Err(DeviceStoreError::Invalid {
                    reason: format!("设备 {} 的 name 不能为空", device.id),
                });
            }
            if device.adapter.trim().is_empty() {
                return Err(DeviceStoreError::Invalid {
                    reason: format!("设备 {} 的 adapter 不能为空", device.id),
                });
            }
            if !seen.insert(device.id.as_str()) {
                return Err(DeviceStoreError::Invalid {
                    reason: format!("设备 id 重复: {}", device.id),
                });
            }
        }
        Ok(())
    }

    /// 白名单 = 表内且 `enabled` (底线①: 显式启用才可感知)。
    pub fn whitelist(&self) -> Vec<&Device> {
        self.devices
            .iter()
            .filter(|device| device.enabled)
            .collect()
    }

    /// 按 id 查设备 (`None` = 表上没这台)。
    pub fn find(&self, id: &str) -> Option<&Device> {
        self.devices.iter().find(|device| device.id == id)
    }

    /// 设备数 (表内行数, 含停用)。
    pub fn len(&self) -> usize {
        self.devices.len()
    }

    /// 空表判定 (默认零设备)。
    pub fn is_empty(&self) -> bool {
        self.devices.is_empty()
    }
}

/// 设备表存储面错误 (闭合词表: 拒开 / 坏表 / 写失败)。
#[derive(Debug)]
pub enum DeviceStoreError {
    /// `devices.json` 存在但拒开 (StoredDoc 语义: 不静默修复)。
    Stored(StoredDocError),
    /// 表体坏 (重复 id / 空字段) —— 拒开也拒写。
    Invalid {
        /// 具体原因 (人读)。
        reason: String,
    },
    /// IO 失败 (读写路径 + 真因)。
    Io {
        /// 相关路径。
        path: PathBuf,
        /// 底层错误。
        source: std::io::Error,
    },
}

impl std::fmt::Display for DeviceStoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Stored(error) => write!(f, "设备表拒开: {error}"),
            Self::Invalid { reason } => write!(f, "设备表坏 (拒开也拒写): {reason}"),
            Self::Io { path, source } => write!(f, "设备表 IO 失败 {}: {source}", path.display()),
        }
    }
}

impl std::error::Error for DeviceStoreError {}

impl From<StoredDocError> for DeviceStoreError {
    fn from(error: StoredDocError) -> Self {
        Self::Stored(error)
    }
}

/// 设备表读端契约 (仅当前格式版本可读, 其余拒开)。
fn devices_compat() -> DocCompat {
    DocCompat::exact(DEVICES_DOC_NAME, DEVICES_DOC_VERSION)
}

/// 设备注册表: `<data_dir>/devices.json` 的读写面 (StoredDoc 拒开语义)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceRegistry {
    path: PathBuf,
}

impl DeviceRegistry {
    /// 数据目录落点 (`<data_dir>/devices.json`)。
    pub fn under_data_dir(data_dir: impl AsRef<Path>) -> Self {
        Self {
            path: data_dir.as_ref().join(DEVICES_FILE),
        }
    }

    /// 显式路径落点。
    pub fn at(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// 设备表路径。
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// 装载设备表:
    ///
    /// - 文件缺失 = [`DevicesBody::new`] (默认零设备, 白名单制的空态);
    /// - 文件存在但坏 (畸形 / 串档 / 未来版本 / 坏条目) = `Err` (拒开, 不回退)。
    pub fn load(&self) -> Result<DevicesBody, DeviceStoreError> {
        if !self.path.exists() {
            return Ok(DevicesBody::new());
        }
        let body = self.load_strict()?;
        Ok(body)
    }

    /// 严格装载 (文件必须存在且可开): 与 [`Self::load`] 同拒开语义, 缺文件报
    /// IO 错误 —— 供"区分缺省与实载"的调用方使用。
    pub fn load_strict(&self) -> Result<DevicesBody, DeviceStoreError> {
        let doc = stored_doc::open_single::<DevicesBody>(&self.path, &devices_compat())?;
        doc.body.validate()?;
        Ok(doc.body)
    }

    /// 落盘设备表 (持久档原子写): 先校验后落盘, 坏表拒写 (不带病落盘)。
    pub fn save(&self, body: &DevicesBody) -> Result<(), DeviceStoreError> {
        body.validate()?;
        stored_doc::save_single(
            &self.path,
            &devices_compat(),
            body,
            stored_doc::DEFAULT_DOC_MODE,
        )?;
        Ok(())
    }
}

/// 设备表 schema (JSON Schema 形状的单一真源)。
///
/// 稳定合同: 信封 `{name, version, compatible_versions, body}` + 主体
/// `{devices: [{id, kind, name, adapter, enabled}]}`; `kind` 闭合词表,
/// 行内未知字段拒开。产出交付与测试都引用这一份, 不另抄。
pub fn devices_json_schema() -> Value {
    json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "title": DEVICES_DOC_NAME,
        "type": "object",
        "description": "设备注册表 (StoredDoc 信封): 白名单制, 默认零设备; 坏表拒开也拒写。",
        "required": ["name", "version", "compatible_versions", "body"],
        "additionalProperties": false,
        "properties": {
            "name": { "const": DEVICES_DOC_NAME },
            "version": { "const": DEVICES_DOC_VERSION },
            "compatible_versions": {
                "type": "array",
                "items": { "type": "integer" },
                "contains": { "const": DEVICES_DOC_VERSION }
            },
            "body": {
                "type": "object",
                "required": ["devices"],
                "additionalProperties": false,
                "properties": {
                    "devices": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "required": ["id", "kind", "name", "adapter", "enabled"],
                            "additionalProperties": false,
                            "properties": {
                                "id": { "type": "string", "minLength": 1 },
                                "kind": {
                                    "type": "string",
                                    "enum": ["sensor", "camera", "appliance", "other"]
                                },
                                "name": { "type": "string", "minLength": 1 },
                                "adapter": { "type": "string", "minLength": 1 },
                                "enabled": {
                                    "type": "boolean",
                                    "description": "白名单启用位: true = 可感知, false = 登记不碰"
                                }
                            }
                        }
                    }
                }
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "apeireth-device-registry-{tag}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("创建测试目录");
        dir
    }

    fn device(id: &str, kind: DeviceKind, enabled: bool) -> Device {
        Device {
            id: id.to_string(),
            kind,
            name: format!("设备 {id}"),
            adapter: "mock".to_string(),
            enabled,
        }
    }

    #[test]
    fn a_missing_table_is_the_default_zero_device_whitelist() {
        let registry = DeviceRegistry::under_data_dir(root("missing"));
        let body = registry.load().expect("缺省表不是错误");
        assert!(body.is_empty(), "默认零设备");
        assert!(body.whitelist().is_empty(), "默认白名单为空");
    }

    #[test]
    fn saving_and_loading_round_trips_the_whitelist() {
        let registry = DeviceRegistry::under_data_dir(root("roundtrip"));
        let body = DevicesBody {
            devices: vec![
                device("thermo-1", DeviceKind::Sensor, true),
                device("cam-1", DeviceKind::Camera, false),
            ],
        };
        registry.save(&body).expect("落盘");
        let loaded = registry.load().expect("可开");
        assert_eq!(loaded, body);
        let whitelist = loaded.whitelist();
        assert_eq!(whitelist.len(), 1);
        assert_eq!(whitelist[0].id, "thermo-1");
    }

    #[test]
    fn a_defective_table_refuses_to_open() {
        let root = root("defect");
        let registry = DeviceRegistry::under_data_dir(&root);
        registry
            .save(&DevicesBody {
                devices: vec![device("a", DeviceKind::Sensor, true)],
            })
            .unwrap();

        // 畸形 JSON = 拒开。
        std::fs::write(registry.path(), b"{ not json").unwrap();
        assert!(registry.load().is_err(), "畸形 JSON 必须拒开");

        // 串档身份 = 拒开。
        let wrong_name = serde_json::to_string(&stored_doc::StoredDoc::new(
            "other-doc",
            DEVICES_DOC_VERSION,
            vec![DEVICES_DOC_VERSION],
            DevicesBody::new(),
        ))
        .unwrap();
        std::fs::write(registry.path(), wrong_name).unwrap();
        assert!(registry.load().is_err(), "串档必须拒开");

        // 未来版本 = 拒开 (不静默迁移)。
        let future = serde_json::to_string(&stored_doc::StoredDoc::new(
            DEVICES_DOC_NAME,
            DEVICES_DOC_VERSION + 1,
            vec![DEVICES_DOC_VERSION + 1],
            DevicesBody::new(),
        ))
        .unwrap();
        std::fs::write(registry.path(), future).unwrap();
        assert!(registry.load().is_err(), "未来版本必须拒开");

        // 词表外 kind = 坏条目 = 拒开。
        std::fs::write(
            registry.path(),
            format!(
                r#"{{"name":"{DEVICES_DOC_NAME}","version":{DEVICES_DOC_VERSION},
                     "compatible_versions":[{DEVICES_DOC_VERSION}],
                     "body":{{"devices":[{{"id":"a","kind":"toaster",
                     "name":"A","adapter":"mock","enabled":true}}]}}}}"#
            ),
        )
        .unwrap();
        assert!(registry.load().is_err(), "词表外 kind 必须拒开");

        // 重复 id = 坏条目 = 拒开。
        let dup = DevicesBody {
            devices: vec![
                device("a", DeviceKind::Sensor, true),
                device("a", DeviceKind::Other, true),
            ],
        };
        let doc = stored_doc::StoredDoc::new(
            DEVICES_DOC_NAME,
            DEVICES_DOC_VERSION,
            vec![DEVICES_DOC_VERSION],
            dup,
        );
        std::fs::write(registry.path(), serde_json::to_string(&doc).unwrap()).unwrap();
        assert!(registry.load().is_err(), "重复 id 必须拒开");
    }

    #[test]
    fn a_defective_body_is_refused_on_save_too() {
        let registry = DeviceRegistry::under_data_dir(root("refuse-save"));
        let bad = DevicesBody {
            devices: vec![
                device("a", DeviceKind::Sensor, true),
                device("a", DeviceKind::Other, true),
            ],
        };
        assert!(registry.save(&bad).is_err(), "坏表拒写 (不带病落盘)");
        assert!(!registry.path().exists(), "拒写不得落盘");
    }

    #[test]
    fn the_schema_pins_the_stable_table_shape() {
        let schema = devices_json_schema();
        assert_eq!(schema["properties"]["name"]["const"], DEVICES_DOC_NAME);
        assert_eq!(
            schema["properties"]["version"]["const"],
            DEVICES_DOC_VERSION
        );
        let kind = &schema["properties"]["body"]["properties"]["devices"]["items"]["properties"]
            ["kind"]["enum"];
        // 跨线机械补位 (device_control 批报备): `&Value` 与 `Value` 不可比, 补 `&` 同义。
        assert_eq!(kind, &json!(["sensor", "camera", "appliance", "other"]));
        let required = &schema["properties"]["body"]["properties"]["devices"]["items"]["required"];
        assert_eq!(
            required,
            &json!(["id", "kind", "name", "adapter", "enabled"])
        );
    }
}
