//! 模型画像表 (`ModelProfile`) —— 模型面轴 (H-P1) 的依据数据面。
//!
//! 画像回答「这个模型能干什么、怎么计价」——路由与成本分级都以它为依据。
//! 一行画像 = 模型 id + 能力事实 (上下文窗 / 输出上限 / 工具支持 / 流式支持 /
//! 额外能力标记) + 单价 (微元/千 token)。**缺省即 `None` = 未提供, 绝不编造**:
//! 未知模型回显式 unknown 行 (见 [`ModelProfile::unknown`]), 单价缺省永不硬造。
//!
//! # 配置面: 三层覆盖合并, env 最上
//!
//! 1. **内置基线** ([`ModelProfileTable::baseline`]): 本库提供商默认模型表已声明
//!    的 id (接口事实)。能力布尔取族声明 ([`BASELINE_OPENAI_COMPATIBLE_MODELS`] 等
//!    三个族常量, 与 `ModelDescriptor` 的能力声明同源); 上下文窗/输出上限是
//!    **保守下界** ([`BASELINE_CONTEXT_WINDOW_FLOOR`] / [`BASELINE_MAX_OUTPUT_FLOOR`],
//!    routing floor, 不是规格书); **单价一律未提供** (绝不编价)。
//! 2. **数据目录 `model-profiles.json`** (见 [`MODEL_PROFILES_FILE`]): 存储文档
//!    拒开语义 ([`crate::stored_doc`]) —— 坏 JSON / 身份不符 / 版本不符**拒绝
//!    打开**, 绝不静默回退基线。
//! 3. **env [`MODEL_PROFILES_ENV`]** (JSON 覆盖): 顶层 JSON 数组或
//!    `{"profiles": [...]}` 对象; 解析失败同样 fail-closed。
//!
//! 三层**逐模型整行覆盖**: 同一 model id 的上层行整行替换下层行 (不是字段级
//! 混血), 合并结果可预测、口径唯一。上层优先级: env > 数据目录 > 基线。
//!
//! # 无密钥面 (脱敏)
//!
//! 画像面**没有任何凭据字段** (结构上无密钥可泄)。配置解析是值自由报错:
//! 未知字段拒绝时只报**字段名**, 类型不符只报**期望类型** —— 任何被误粘贴进
//! 配置的密钥值都不会被错误消息回显 (见 [`ModelProfileError`])。

use std::borrow::Cow;
use std::fmt;
use std::path::{Path, PathBuf};

use serde::de::Error as _;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::stored_doc::{self, DocCompat, StoredDocError};

/// env 入口: `APEIRETH_MODEL_PROFILES` (JSON 覆盖层)。
pub const MODEL_PROFILES_ENV: &str = "APEIRETH_MODEL_PROFILES";

/// 数据目录里的画像表文件名。
pub const MODEL_PROFILES_FILE: &str = "model-profiles.json";

/// 画像表存储文档的身份名。
pub const MODEL_PROFILES_DOC_NAME: &str = "model-profiles";

/// 画像表存储文档的格式版本。
pub const MODEL_PROFILES_DOC_VERSION: u32 = 1;

/// 基线保守下界: 上下文窗 (token)。routing floor, **不是**模型规格书。
pub const BASELINE_CONTEXT_WINDOW_FLOOR: u64 = 32_768;

/// 基线保守下界: 单响应输出上限 (token)。routing floor, **不是**模型规格书。
pub const BASELINE_MAX_OUTPUT_FLOOR: u64 = 8_192;

/// 基线能力布尔的族来源: openai-compatible 族默认模型表 (接口事实)。
///
/// 该族的 `ModelDescriptor` 声明 `system_prompt` + `tool_calls` + `streaming`。
pub const BASELINE_OPENAI_COMPATIBLE_MODELS: &[&str] = &[
    "gpt-4o",
    "gpt-4o-mini",
    "gpt-4-turbo",
    "gpt-4",
    "gpt-3.5-turbo",
    "o1",
    "o1-mini",
    "o3-mini",
    "deepseek-chat",
    "deepseek-reasoner",
    "qwen-turbo",
    "qwen-plus",
    "qwen-max",
];

/// 基线能力布尔的族来源: minimax 族默认模型表 (接口事实)。
///
/// 该族的 `ModelDescriptor` 声明 `system_prompt` + `tool_calls` (不声明流式)。
pub const BASELINE_MINIMAX_MODELS: &[&str] = &["MiniMax-M3", "MiniMax-M3-thinking"];

/// 基线能力布尔的族来源: anthropic 族默认模型表 (接口事实)。
///
/// 该族的 wire 适配面拒收工具/图像, `ModelDescriptor` 如实只声明 `system_prompt`
/// —— 基线照实镜像, 不拔高。配置层可按真实通路覆盖。
pub const BASELINE_ANTHROPIC_MODELS: &[&str] = &[
    "MiniMax-M3",
    "claude-3-7-sonnet-20250219",
    "claude-3-7-sonnet",
    "claude-3-5-sonnet-20241022",
    "claude-3-5-sonnet",
    "claude-3-5-haiku-20241022",
    "claude-3-5-haiku",
    "claude-3-opus-20240229",
    "claude-3-opus",
    "claude-sonnet-4-5",
    "claude-sonnet-5",
];

/// 额外能力标记的保留词: `tool_calls` / `streaming` 走显式布尔字段,
/// 不允许再进 [`ModelProfile::capabilities_flags`] (禁止同义重复与矛盾)。
pub const RESERVED_CAPABILITY_FLAGS: &[&str] = &["tool_calls", "streaming"];

/// 一行模型画像: 能力事实 + 单价 (微元/千 token)。
///
/// **缺省 = `None` = 未提供**: 上下文窗/输出上限/两布尔/三单价都可缺省,
/// 缺省时如实回 `null`, 不编造任何数字 (单价尤其: 绝不编价)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ModelProfile {
    /// 模型 id (匹配大小写不敏感, 见 [`normalize_model_id`]; 行内保留原拼写)。
    pub model_id: String,
    /// 上下文窗 (token); `None` = 未提供。
    pub context_window: Option<u64>,
    /// 单响应输出上限 (token); `None` = 未提供。
    pub max_output: Option<u64>,
    /// 工具调用支持; `None` = 未提供 (未知不等于不支持)。
    pub supports_tools: Option<bool>,
    /// 流式支持; `None` = 未提供 (未知不等于不支持)。
    pub supports_streaming: Option<bool>,
    /// 输入单价 (微元/千 token); `None` = 未提供, **绝不编价**。
    pub input_price_micros: Option<u64>,
    /// 输出单价 (微元/千 token); `None` = 未提供, **绝不编价**。
    pub output_price_micros: Option<u64>,
    /// 缓存读单价 (微元/千 token); `None` = 未提供, **绝不编价**。
    pub cache_read_price_micros: Option<u64>,
    /// 额外能力标记 (snake_case 开放词表, 保留词见 [`RESERVED_CAPABILITY_FLAGS`])。
    pub capabilities_flags: Vec<String>,
}

impl ModelProfile {
    /// 显式 unknown 行: 除 id 外全部未提供 —— 未知模型不编造任何事实。
    pub fn unknown(model_id: &str) -> Self {
        Self {
            model_id: model_id.to_string(),
            context_window: None,
            max_output: None,
            supports_tools: None,
            supports_streaming: None,
            input_price_micros: None,
            output_price_micros: None,
            cache_read_price_micros: None,
            capabilities_flags: Vec::new(),
        }
    }

    /// 是否是「事实全未提供」的行 (显式 unknown)。部分画像 (哪怕只有一项事实)
    /// 不算 unknown。
    pub fn is_unknown(&self) -> bool {
        self.context_window.is_none()
            && self.max_output.is_none()
            && self.supports_tools.is_none()
            && self.supports_streaming.is_none()
            && self.input_price_micros.is_none()
            && self.output_price_micros.is_none()
            && self.cache_read_price_micros.is_none()
            && self.capabilities_flags.is_empty()
    }

    /// 行级校验: id 非空; 能力标记 snake_case 语法、不重复、不占保留词。
    pub fn validate(&self) -> Result<(), ModelProfileError> {
        if self.model_id.trim().is_empty() {
            return Err(ModelProfileError::InvalidProfile {
                model_id: String::new(),
                reason: "model_id must not be empty".to_string(),
            });
        }
        let mut seen = std::collections::BTreeSet::new();
        for flag in &self.capabilities_flags {
            if !is_valid_capability_flag(flag) {
                return Err(ModelProfileError::InvalidProfile {
                    model_id: self.model_id.clone(),
                    reason: format!(
                        "capability flag `{flag}` must match [a-z][a-z0-9_]* and must not be a reserved word"
                    ),
                });
            }
            if !seen.insert(flag.as_str()) {
                return Err(ModelProfileError::InvalidProfile {
                    model_id: self.model_id.clone(),
                    reason: format!("capability flag `{flag}` is listed twice"),
                });
            }
        }
        Ok(())
    }

    /// 从 JSON 行解析 (值自由报错: 只报字段名与期望类型, 不回显任何值)。
    pub fn from_json_value(value: &Value) -> Result<Self, ModelProfileError> {
        let map = value
            .as_object()
            .ok_or_else(|| ModelProfileError::InvalidShape {
                reason: format!(
                    "a profile row must be a JSON object, found {}",
                    value_kind(value)
                ),
            })?;
        const FIELDS: &[&str] = &[
            "model_id",
            "context_window",
            "max_output",
            "supports_tools",
            "supports_streaming",
            "input_price_micros",
            "output_price_micros",
            "cache_read_price_micros",
            "capabilities_flags",
        ];
        for key in map.keys() {
            if !FIELDS.contains(&key.as_str()) {
                return Err(ModelProfileError::InvalidShape {
                    reason: format!(
                        "unknown field `{key}` in a model profile row (accepted fields: {})",
                        FIELDS.join(", ")
                    ),
                });
            }
        }
        let model_id = required_string(map, "model_id")?;
        let profile = Self {
            model_id,
            context_window: optional_u64(map, "context_window")?,
            max_output: optional_u64(map, "max_output")?,
            supports_tools: optional_bool(map, "supports_tools")?,
            supports_streaming: optional_bool(map, "supports_streaming")?,
            input_price_micros: optional_u64(map, "input_price_micros")?,
            output_price_micros: optional_u64(map, "output_price_micros")?,
            cache_read_price_micros: optional_u64(map, "cache_read_price_micros")?,
            capabilities_flags: optional_flags(map, "capabilities_flags")?,
        };
        profile.validate()?;
        Ok(profile)
    }
}

impl<'de> Deserialize<'de> for ModelProfile {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = Value::deserialize(deserializer)?;
        Self::from_json_value(&value).map_err(D::Error::custom)
    }
}

/// 画像表: 一层配置的全部画像行 (多层叠加见 [`ModelProfileTable::with_overlay`])。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct ModelProfileTable {
    /// 画像行 (保持配置顺序; model id 层内唯一, 大小写不敏感)。
    pub profiles: Vec<ModelProfile>,
}

impl ModelProfileTable {
    /// 一张显式行表。
    pub fn new(profiles: Vec<ModelProfile>) -> Self {
        Self { profiles }
    }

    /// 内置基线表: 本库默认模型表已声明的 id + 族能力布尔 + 保守下界窗口,
    /// 单价一律未提供 (绝不编价)。
    ///
    /// 同一 id 出现在多族时能力布尔取 **OR** (任一通路可即为支持), 不做减法。
    pub fn baseline() -> Self {
        let mut rows: Vec<ModelProfile> = Vec::new();
        declare_family(&mut rows, BASELINE_OPENAI_COMPATIBLE_MODELS, true, true);
        declare_family(&mut rows, BASELINE_MINIMAX_MODELS, true, false);
        declare_family(&mut rows, BASELINE_ANTHROPIC_MODELS, false, false);
        Self { profiles: rows }
    }

    /// 全部行。
    pub fn profiles(&self) -> &[ModelProfile] {
        &self.profiles
    }

    /// 精确查表 (model id 匹配大小写不敏感); 无行回 `None`。
    pub fn get(&self, model_id: &str) -> Option<&ModelProfile> {
        let key = normalize_model_id(model_id);
        self.profiles
            .iter()
            .find(|profile| normalize_model_id(&profile.model_id) == key)
    }

    /// 解析一个 model id 的画像: 有行回该行, 无行回显式 unknown 行
    /// (事实全未提供, 不编造)。
    pub fn resolve(&self, model_id: &str) -> Cow<'_, ModelProfile> {
        match self.get(model_id) {
            Some(profile) => Cow::Borrowed(profile),
            None => Cow::Owned(ModelProfile::unknown(model_id)),
        }
    }

    /// 覆盖合并: `overrides` 的行按 model id **整行替换**本表同 id 行
    /// (大小写不敏感), 本表没有的 id 追加在后。不是字段级混血。
    pub fn with_overlay(&self, overrides: &Self) -> Self {
        let mut merged = self.clone();
        for row in &overrides.profiles {
            let key = normalize_model_id(&row.model_id);
            match merged
                .profiles
                .iter_mut()
                .find(|profile| normalize_model_id(&profile.model_id) == key)
            {
                Some(existing) => *existing = row.clone(),
                None => merged.profiles.push(row.clone()),
            }
        }
        merged
    }

    /// 解析一层配置的 JSON 值: 顶层 JSON 数组或 `{"profiles": [...]}` 对象。
    /// fail-closed: 形状/字段/行校验任一不符都是错误, 值自由报错。
    pub fn from_json_value(value: Value) -> Result<Self, ModelProfileError> {
        let rows = match &value {
            Value::Array(items) => items,
            Value::Object(map) => {
                for key in map.keys() {
                    if key != "profiles" {
                        return Err(ModelProfileError::InvalidShape {
                            reason: format!(
                                "unknown field `{key}` in the model profile table (accepted: profiles)"
                            ),
                        });
                    }
                }
                match map.get("profiles") {
                    Some(Value::Array(items)) => items,
                    Some(other) => {
                        return Err(ModelProfileError::InvalidShape {
                            reason: format!(
                                "field `profiles` must be an array, found {}",
                                value_kind(other)
                            ),
                        })
                    }
                    None => {
                        return Err(ModelProfileError::InvalidShape {
                            reason: "missing field `profiles` in the model profile table".to_string(),
                        })
                    }
                }
            }
            other => {
                return Err(ModelProfileError::InvalidShape {
                    reason: format!(
                        "the model profile table must be a JSON array or an object with `profiles`, found {}",
                        value_kind(other)
                    ),
                })
            }
        };
        let mut profiles = Vec::with_capacity(rows.len());
        for row in rows {
            profiles.push(ModelProfile::from_json_value(row)?);
        }
        let table = Self { profiles };
        table.validate()?;
        Ok(table)
    }

    /// 解析 env [`MODEL_PROFILES_ENV`] 的原始值。fail-closed。
    pub fn from_env_value(raw: &str) -> Result<Self, ModelProfileError> {
        let value: Value =
            serde_json::from_str(raw).map_err(|error| ModelProfileError::InvalidShape {
                reason: format!(
                    "{MODEL_PROFILES_ENV}: JSON parse failed at line {}",
                    error.line()
                ),
            })?;
        Self::from_json_value(value)
    }

    /// 读数据目录次入口 ([`MODEL_PROFILES_FILE`], 存储文档拒开语义)。
    /// `Ok(None)` = 文件不存在; 存在但拒开 = 错误 (绝不静默回退)。
    pub fn load_from_data_dir(data_dir: &Path) -> Result<Option<Self>, ModelProfileError> {
        let path = model_profiles_path(data_dir);
        if !path.exists() {
            return Ok(None);
        }
        let doc = stored_doc::open_single::<Self>(&path, &doc_compat())?;
        Ok(Some(doc.body))
    }

    /// 三层折叠 (纯函数, env 原始值显式传入): 基线 ← 数据目录 ← env,
    /// 逐模型整行覆盖, env 最上; 任一层坏 = 整体拒开。
    pub fn load_with(
        data_dir: Option<&Path>,
        env_raw: Option<&str>,
    ) -> Result<Self, ModelProfileError> {
        let mut table = Self::baseline();
        if let Some(dir) = data_dir {
            if let Some(file_table) = Self::load_from_data_dir(dir)? {
                table = table.with_overlay(&file_table);
            }
        }
        if let Some(raw) = env_raw.map(str::trim).filter(|raw| !raw.is_empty()) {
            let env_table = Self::from_env_value(raw)?;
            table = table.with_overlay(&env_table);
        }
        Ok(table)
    }

    /// 生产入口: env [`MODEL_PROFILES_ENV`] 现读, 三层折叠 (见
    /// [`Self::load_with`])。
    pub fn load(data_dir: Option<&Path>) -> Result<Self, ModelProfileError> {
        let raw = std::env::var(MODEL_PROFILES_ENV).ok();
        Self::load_with(data_dir, raw.as_deref())
    }

    /// 表级校验: 每行行级校验 + model id 层内唯一 (大小写不敏感)。
    pub fn validate(&self) -> Result<(), ModelProfileError> {
        let mut seen = std::collections::BTreeSet::new();
        for profile in &self.profiles {
            profile.validate()?;
            if !seen.insert(normalize_model_id(&profile.model_id)) {
                return Err(ModelProfileError::DuplicateModel {
                    model_id: profile.model_id.clone(),
                });
            }
        }
        Ok(())
    }
}

impl<'de> Deserialize<'de> for ModelProfileTable {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = Value::deserialize(deserializer)?;
        Self::from_json_value(value).map_err(D::Error::custom)
    }
}

/// 数据目录里画像表的路径。
pub fn model_profiles_path(data_dir: &Path) -> PathBuf {
    data_dir.join(MODEL_PROFILES_FILE)
}

/// 画像表存储文档的读写契约 (身份 + 版本拒开)。
pub fn doc_compat() -> DocCompat {
    DocCompat::exact(MODEL_PROFILES_DOC_NAME, MODEL_PROFILES_DOC_VERSION)
}

/// model id 的匹配归一: ASCII 大小写不敏感 (与提供商面规范 id 的小写化同口径)。
pub fn normalize_model_id(model_id: &str) -> String {
    model_id.to_ascii_lowercase()
}

/// 画像配置面的缺陷。所有错误**值自由**: 只报字段名 / 期望类型 / 行位置,
/// 从不回显被解析的值 (误粘贴的密钥绝不进错误帧)。
#[derive(Debug)]
pub enum ModelProfileError {
    /// JSON 形状或字段不符 (值自由报错)。
    InvalidShape {
        /// 只含字段名与期望类型的说明。
        reason: String,
    },
    /// 单行画像字段非法 (只报字段名与语法要求)。
    InvalidProfile {
        /// 出错的 model id (id 是接口事实, 不是密钥)。
        model_id: String,
        /// 语法要求说明 (不含值)。
        reason: String,
    },
    /// 同一 model id 在一层配置里出现两次 (大小写不敏感)。
    DuplicateModel {
        /// 重复的 model id。
        model_id: String,
    },
    /// 数据目录的存储文档拒绝打开 (拒开语义)。
    Stored(StoredDocError),
}

impl From<StoredDocError> for ModelProfileError {
    fn from(source: StoredDocError) -> Self {
        Self::Stored(source)
    }
}

impl fmt::Display for ModelProfileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidShape { reason } => {
                write!(f, "model profile configuration is malformed: {reason}")
            }
            Self::InvalidProfile { model_id, reason } => {
                write!(f, "model profile {model_id:?} is invalid: {reason}")
            }
            Self::DuplicateModel { model_id } => {
                write!(f, "model profile {model_id:?} is configured twice")
            }
            Self::Stored(source) => {
                write!(f, "model profile file refused to open: {source}")
            }
        }
    }
}

impl std::error::Error for ModelProfileError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Stored(source) => Some(source),
            _ => None,
        }
    }
}

/// 基线行: 保守下界窗口 + 族能力布尔 + 族能力标记 + 单价未提供。
fn baseline_row(model_id: &str, supports_tools: bool, supports_streaming: bool) -> ModelProfile {
    ModelProfile {
        model_id: model_id.to_string(),
        context_window: Some(BASELINE_CONTEXT_WINDOW_FLOOR),
        max_output: Some(BASELINE_MAX_OUTPUT_FLOOR),
        supports_tools: Some(supports_tools),
        supports_streaming: Some(supports_streaming),
        input_price_micros: None,
        output_price_micros: None,
        cache_read_price_micros: None,
        capabilities_flags: vec!["system_prompt".to_string()],
    }
}

/// 把一族声明折进基线行表; 同 id 再遇 = 能力布尔 OR (任一通路可即为支持)。
fn declare_family(
    rows: &mut Vec<ModelProfile>,
    ids: &[&str],
    supports_tools: bool,
    supports_streaming: bool,
) {
    for id in ids {
        let key = normalize_model_id(id);
        match rows
            .iter_mut()
            .find(|profile| normalize_model_id(&profile.model_id) == key)
        {
            Some(existing) => {
                existing.supports_tools =
                    Some(existing.supports_tools.unwrap_or(false) || supports_tools);
                existing.supports_streaming =
                    Some(existing.supports_streaming.unwrap_or(false) || supports_streaming);
            }
            None => rows.push(baseline_row(id, supports_tools, supports_streaming)),
        }
    }
}

/// 能力标记语法: `[a-z][a-z0-9_]*` 且不占保留词。
fn is_valid_capability_flag(flag: &str) -> bool {
    if RESERVED_CAPABILITY_FLAGS.contains(&flag) {
        return false;
    }
    let mut chars = flag.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    first.is_ascii_lowercase()
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

/// JSON 值的类型名 (报错用; 只报类型, 绝不报值)。
fn value_kind(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

fn invalid_field(field: &str, expected: &str, found: &Value) -> ModelProfileError {
    ModelProfileError::InvalidShape {
        reason: format!(
            "field `{field}` must be {expected}, found {}",
            value_kind(found)
        ),
    }
}

fn required_string(
    map: &serde_json::Map<String, Value>,
    field: &str,
) -> Result<String, ModelProfileError> {
    match map.get(field) {
        Some(Value::String(text)) => Ok(text.clone()),
        Some(other) => Err(invalid_field(field, "a string", other)),
        None => Err(ModelProfileError::InvalidShape {
            reason: format!("missing field `{field}`"),
        }),
    }
}

fn optional_u64(
    map: &serde_json::Map<String, Value>,
    field: &str,
) -> Result<Option<u64>, ModelProfileError> {
    match map.get(field) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Number(number)) => {
            number
                .as_u64()
                .map(Some)
                .ok_or_else(|| ModelProfileError::InvalidShape {
                    reason: format!(
                        "field `{field}` must be a non-negative integer within u64 range"
                    ),
                })
        }
        Some(other) => Err(invalid_field(field, "a non-negative integer", other)),
    }
}

fn optional_bool(
    map: &serde_json::Map<String, Value>,
    field: &str,
) -> Result<Option<bool>, ModelProfileError> {
    match map.get(field) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Bool(flag)) => Ok(Some(*flag)),
        Some(other) => Err(invalid_field(field, "a boolean", other)),
    }
}

fn optional_flags(
    map: &serde_json::Map<String, Value>,
    field: &str,
) -> Result<Vec<String>, ModelProfileError> {
    match map.get(field) {
        None | Some(Value::Null) => Ok(Vec::new()),
        Some(Value::Array(items)) => {
            let mut flags = Vec::with_capacity(items.len());
            for item in items {
                match item {
                    Value::String(flag) => flags.push(flag.clone()),
                    other => return Err(invalid_field(field, "an array of strings", other)),
                }
            }
            Ok(flags)
        }
        Some(other) => Err(invalid_field(field, "an array of strings", other)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(model_id: &str) -> ModelProfile {
        ModelProfile {
            model_id: model_id.to_string(),
            context_window: Some(65_536),
            max_output: Some(8_192),
            supports_tools: Some(true),
            supports_streaming: Some(true),
            input_price_micros: Some(2_000),
            output_price_micros: Some(8_000),
            cache_read_price_micros: Some(500),
            capabilities_flags: vec!["vision".to_string()],
        }
    }

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "apeireth-model-profiles-{tag}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create test dir");
        dir
    }

    #[test]
    fn builtin_baseline_hits_known_models_with_conservative_values() {
        let baseline = ModelProfileTable::baseline();
        let profile = baseline
            .get("deepseek-chat")
            .expect("known model must hit the baseline");
        assert_eq!(
            profile.context_window,
            Some(BASELINE_CONTEXT_WINDOW_FLOOR),
            "窗口是保守下界 (routing floor), 不冒充规格书"
        );
        assert_eq!(profile.max_output, Some(BASELINE_MAX_OUTPUT_FLOOR));
        assert_eq!(profile.supports_tools, Some(true));
        assert_eq!(profile.supports_streaming, Some(true));
        assert_eq!(
            profile.capabilities_flags,
            vec!["system_prompt".to_string()],
            "族能力标记与 ModelDescriptor 声明同源"
        );

        // minimax 族不声明流式: 基线照实 false。
        let minimax = baseline.get("MiniMax-M3").expect("minimax row");
        assert_eq!(minimax.supports_tools, Some(true), "minimax 族声明工具");
        assert_eq!(
            minimax.supports_streaming,
            Some(false),
            "minimax 族不声明流式"
        );
        // anthropic 族 wire 适配面拒收工具: 照实 false, 不拔高。
        let claude = baseline.get("claude-sonnet-4-5").expect("claude row");
        assert_eq!(claude.supports_tools, Some(false));
        // 同 id 多族: 能力布尔 OR (任一通路可即为支持)。
        assert_eq!(minimax.model_id, "MiniMax-M3", "行 id 保留族声明的接口拼写");
    }

    #[test]
    fn unknown_models_resolve_to_explicit_unknown_profiles() {
        let baseline = ModelProfileTable::baseline();
        let resolved = baseline.resolve("never-heard-of-it");
        assert!(resolved.is_unknown(), "未知模型必须显式 unknown");
        assert_eq!(resolved.model_id, "never-heard-of-it", "id 如实保留");
        assert_eq!(resolved.context_window, None);
        assert_eq!(resolved.max_output, None);
        assert_eq!(resolved.supports_tools, None, "未知不等于不支持");
        assert_eq!(resolved.supports_streaming, None);
        assert!(resolved.capabilities_flags.is_empty());
        assert!(baseline.get("never-heard-of-it").is_none());
    }

    #[test]
    fn unit_price_fields_default_to_none_and_are_never_invented() {
        // 基线单价一律未提供 (绝不编价)。
        for profile in ModelProfileTable::baseline().profiles() {
            assert_eq!(profile.input_price_micros, None, "{}", profile.model_id);
            assert_eq!(profile.output_price_micros, None, "{}", profile.model_id);
            assert_eq!(
                profile.cache_read_price_micros, None,
                "{}",
                profile.model_id
            );
        }
        // 只给了输入价的行: 缺的两项保持 None, 不从任何地方补。
        let partial = ModelProfileTable::from_json_value(serde_json::json!([
            {"model_id": "half-priced", "input_price_micros": 100}
        ]))
        .expect("parse");
        let profile = partial.get("half-priced").expect("row");
        assert_eq!(profile.input_price_micros, Some(100));
        assert_eq!(profile.output_price_micros, None);
        assert_eq!(profile.cache_read_price_micros, None);
        // 缺省字段全走 None (未提供)。
        assert_eq!(profile.context_window, None);
        assert_eq!(profile.supports_tools, None);
    }

    #[test]
    fn layered_configuration_overlays_per_model_and_env_wins() {
        let dir = temp_dir("overlay");
        // 数据目录层: 整行替换基线行 + 追加新行。
        let file_table = ModelProfileTable::new(vec![row("gpt-4o"), row("custom-model")]);
        stored_doc::save_single(
            &model_profiles_path(&dir),
            &doc_compat(),
            &file_table,
            stored_doc::DEFAULT_DOC_MODE,
        )
        .expect("save file layer");
        // env 层: 再整行覆盖 gpt-4o (env 最上)。
        let env_raw = r#"[
            {"model_id": "gpt-4o", "context_window": 1, "input_price_micros": 7},
            {"model_id": "env-only", "max_output": 3}
        ]"#;

        let merged = ModelProfileTable::load_with(Some(&dir), Some(env_raw)).expect("load");
        let gpt4o = merged.get("gpt-4o").expect("gpt-4o row");
        assert_eq!(gpt4o.context_window, Some(1), "env 优先: 整行覆盖");
        assert_eq!(gpt4o.input_price_micros, Some(7));
        assert_eq!(gpt4o.supports_tools, None, "整行替换, 不是字段级混血");
        let custom = merged.get("custom-model").expect("file row kept");
        assert_eq!(custom.input_price_micros, Some(2_000), "文件层行保留");
        assert!(merged.get("env-only").is_some(), "env 新行追加");
        // 基线层不受影响的行照旧在场。
        assert!(merged.get("claude-sonnet-4-5").is_some());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_broken_stored_document_refuses_to_open() {
        let dir = temp_dir("refuse");
        let path = model_profiles_path(&dir);
        // 坏 JSON: 拒开, 不回退基线 (哪怕 env 层同时给值)。
        std::fs::write(&path, "{ not json").expect("write broken");
        let error = ModelProfileTable::load_with(Some(&dir), Some("[]"))
            .expect_err("broken file must refuse");
        assert!(matches!(error, ModelProfileError::Stored(_)), "{error:?}");
        // 身份不符 / 未来版本: 同样拒开。
        let wrong_name = serde_json::to_string(&stored_doc::StoredDoc::new(
            "someone-else",
            MODEL_PROFILES_DOC_VERSION,
            vec![MODEL_PROFILES_DOC_VERSION],
            ModelProfileTable::default(),
        ))
        .expect("serialize");
        std::fs::write(&path, wrong_name).expect("write wrong name");
        assert!(ModelProfileTable::load_from_data_dir(&dir).is_err());
        let future = serde_json::to_string(&stored_doc::StoredDoc::new(
            MODEL_PROFILES_DOC_NAME,
            MODEL_PROFILES_DOC_VERSION + 1,
            vec![MODEL_PROFILES_DOC_VERSION + 1],
            ModelProfileTable::default(),
        ))
        .expect("serialize");
        std::fs::write(&path, future).expect("write future");
        assert!(ModelProfileTable::load_from_data_dir(&dir).is_err());
        // 文件缺失 = 显式无此层 (None), 不是错误。
        let _ = std::fs::remove_file(&path);
        assert!(ModelProfileTable::load_from_data_dir(&dir)
            .expect("absent file is not an error")
            .is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn env_and_file_configuration_fail_closed_on_any_defect() {
        // 坏 JSON / 顶层形状错 / 行形状错 / 未知字段 / 保留词标记 / 重复 id:
        // 全部 fail-closed。
        for raw in [
            "not json",
            "\"a string\"",
            r#"{"model_id": "x"}"#,
            r#"[{"model_id": "x", "surprise": 1}]"#,
            r#"[{"model_id": "x", "capabilities_flags": ["tool_calls"]}]"#,
            r#"[{"model_id": "x"}, {"model_id": "X"}]"#,
            r#"[{"model_id": ""}]"#,
            r#"[{"model_id": "x", "capabilities_flags": ["Bad-Flag"]}]"#,
        ] {
            assert!(
                ModelProfileTable::from_env_value(raw).is_err(),
                "must fail closed: {raw}"
            );
        }
        // 正常形状: 数组与对象双语法都收。
        assert!(ModelProfileTable::from_env_value("[]").is_ok());
        assert!(ModelProfileTable::from_env_value(r#"{"profiles": []}"#).is_ok());
    }

    #[test]
    fn the_profile_plane_carries_no_credential_surface() {
        // 结构面: 画像/表序列化后没有任何密钥面字段。
        let rendered =
            serde_json::to_string(&ModelProfileTable::new(vec![row("m")])).expect("serialize");
        for secretish in ["key", "token", "secret", "password", "credential", "auth"] {
            assert!(
                !rendered.to_ascii_lowercase().contains(secretish),
                "画像面必须无密钥面: {rendered}"
            );
        }
        // 解析面: 误粘贴的凭据字段被拒, 且错误帧不回显其值。
        let error = ModelProfileTable::from_env_value(
            r#"[{"model_id": "m", "api_key": "sk-live-never-echo-me"}]"#,
        )
        .expect_err("credential field must be refused");
        let message = error.to_string();
        assert!(message.contains("api_key"), "报字段名: {message}");
        assert!(
            !message.contains("never-echo-me"),
            "值绝不能进错误帧: {message}"
        );
        // 类型不符同样值自由 (报期望类型, 不报值)。
        let error = ModelProfileTable::from_env_value(
            r#"[{"model_id": "m", "input_price_micros": "sk-live-never-echo-me"}]"#,
        )
        .expect_err("type mismatch must be refused");
        assert!(!error.to_string().contains("never-echo-me"));
    }

    #[test]
    fn model_id_matching_is_ascii_case_insensitive() {
        let table = ModelProfileTable::baseline();
        assert!(table.get("MINIMAX-M3").is_some(), "匹配大小写不敏感");
        assert_eq!(normalize_model_id("MiniMax-M3"), "minimax-m3");
        let resolved = table.resolve("GPT-4O");
        assert!(!resolved.is_unknown());
    }
}
