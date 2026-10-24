//! 设置页三页 · `capability` 子面的 **wasm 宿主挂载**模块（能力与治理）。
//!
//! ## 本模块是什么
//!
//! **宿主侧**：持有三个分区的状态（能力表 / 认知深度 / 全局权限预设 / 允许服务器 /
//! 字段旗标 / 配置 JSON 预览）+ **固定数据**（`fixtures/capabilities.json` +
//! `fixtures/provider.json`），把 `ui_kit::settings_capability` 的组件挂到页面的
//! `#app` 容器里。
//!
//! ## 诚实边界
//!
//! | 事项 | 口径 |
//! |---|---|
//! | 数据 | **固定 fixture**（宿主注入）⇒ 引擎不碰 HTTP / SSE 契约 |
//! | 能力开关的 apply | **宿主侧**（配置持久化 + env 注入 + 网关热应用）⇒ 引擎只发意图；本宿主**只记意图**，不落盘 |
//! | 认知深度预设 | 生产是**派生 + 即效提交**（`applyCognitiveDepth`）⇒ 本宿主只做**派生**（`toggle_capability` 里判 judge/council） |
//! | 全局权限预设 | 生产落 `localStorage`（`apeireth-permission-preset-default`）⇒ 引擎**不碰** ⇒ 本宿主只持有取值 |
//! | 允许服务器 | 生产走 HTTP（`listAllowedServers` / `addAllowedServer` / `removeAllowedServer`）⇒ 引擎只发意图 |
//! | 配置 JSON 复制 | 生产走剪贴板 ⇒ 引擎只发意图；预览文本**由宿主生成**（见下） |
//! | 原生控件的取值 | 生产用 **property**（`bind:value` / `value={…}`）⇒ 宿主在每次渲染后**回写 property**（不写属性） |
//!
//! ## 客户端配置 JSON 的预览（**逐字节相同**）
//!
//! 生产：`JSON.stringify({baseUrl: config.baseUrl, model: providerModel,
//! provider: config.provider, hasApiKey}, null, 2)`
//! （`SettingsView.svelte:1759-1761`）⇒ 键序 `baseUrl` → `model` → `provider` →
//! `hasApiKey`，缩进 2。
//!
//! ⇒ 本宿主**不手拼字符串**：用 `ui_kit::json::JsonValue`（**保插入序**）+
//! `to_string_pretty()`（缩进 2）⇒ 与脚本侧**逐字节相同**。
//!
//! ⚠️ **取值从哪来**（逐条对着参照页核过）：
//!
//! | 字段 | 来源 |
//! |---|---|
//! | `baseUrl` | 参照页的常量 `config.baseUrl`（`web/reference/ReferenceSettingsApp.svelte`）⇒ 见 [`CLIENT_CONFIG_BASE_URL`] |
//! | `model` | 生产的 `providerModel` **初值**（`SettingsView.svelte:897` 的 `$state('gpt-4o')`）⇒ 见 [`CLIENT_CONFIG_MODEL`] |
//! | `provider` | `fixtures/provider.json` 的 `state`（**同一份数据** ⇒ 不手抄）|
//! | `hasApiKey` | 由 [`CLIENT_CONFIG_API_KEY`] 按生产口径 `!!apiKey && apiKey.trim().length > 0` 算 |
//!
//! ## 分区切换（**零 wasm 导出**）
//!
//! `set_settings_section` **已被向导面占用** ⇒ 同名导出 ⇒ `duplicate symbol`
//! ⇒ 链接失败 ⇒ 本面**不导出任何 wasm 函数** ⇒ 走两条不依赖导出名的路径：
//!
//! 1. [`wasm::patch_settings_api`] 包裹共享 `window.__SETTINGS__.selectSection`
//!    （**幂等** ⇒ 共享契约对本面也生效）；
//! 2. `window.__CAPABILITY__.selectSection(id)`（本面控制 API ⇒ 对照脚本直接调）。

use std::collections::BTreeMap;
use std::rc::Rc;

use apeireth_ui_core::{Runtime, Signal};
use apeireth_ui_kit::json::JsonValue;
use apeireth_ui_kit::settings_capability::icons as face_icons;
use apeireth_ui_kit::settings_capability::{
    card_bridge, CapabilityPanelsProps, COPIED_LABEL, COPY_LABEL, REASONING_FILTERS_VALUE_KEY,
    REASONING_TAG_VALUE_KEY, SECTIONS, SECTION_GOVERNANCE,
};
use apeireth_ui_kit::settings_character::{
    CapabilityDef, CapabilityIntent, CapabilityTable, CapabilityValue, CharacterPanelsProps,
    KnobIntent,
};
use apeireth_ui_kit::settings_frame::FieldFlagState;

/// 能力开关的固定数据（**编译期内嵌** ⇒ 与参考侧同一份文件）。
pub const CAPABILITIES_FIXTURE_JSON: &str =
    include_str!("../harness/settings/fixtures/capabilities.json");

/// 服务商与连接的固定数据（**编译期内嵌** ⇒ 与参考侧同一份文件）。
///
/// ⚠️ 本面只从它取**客户端配置 JSON 预览**里的 `provider` 块（生产
/// `config.provider` 就是这个对象）⇒ 不重复手抄那五个取值。
pub const PROVIDER_FIXTURE_JSON: &str = include_str!("../harness/settings/fixtures/provider.json");

/// 参照页的 `config.baseUrl`（**逐字**取自 `web/reference/ReferenceSettingsApp.svelte`
/// 的 `config` 字面量 ⇒ 生产的 `config.baseUrl` 在对照语境下就是这个值）。
pub const CLIENT_CONFIG_BASE_URL: &str = "http://127.0.0.1:8080";

/// 生产的 `providerModel` **初值**（`SettingsView.svelte:897`）。
pub const CLIENT_CONFIG_MODEL: &str = "gpt-4o";

/// 参照页的 `config.apiKey`（空串 ⇒ `hasApiKey` = false）。
pub const CLIENT_CONFIG_API_KEY: &str = "";

/// **生产真值的文案覆盖**（`fileWrite` 行）—— **参照物过期，不是引擎画错**。
///
/// ## 实测（2026-10-24，逐字段比对生产源与固定数据）
///
/// `fixtures/capabilities.json` 的 `capDefs.tool[3].desc`（`fileWrite`）与生产源
/// （`views/SettingsView.svelte` 的 `TOOL_DEFS`）**不一致** —— 固定数据在
/// 「同受此闸」之前多出「重定向/删除类」四个字：
///
/// | 来源 | 前缀 |
/// |---|---|
/// | 生产源（= 参考侧实际渲染的真值）| `文件写入总闸：apply_patch 与 shell 写命令同受此闸——…` |
/// | 固定数据（旧版文案）| `文件写入总闸：apply_patch 与 shell 重定向/删除类写命令同受此闸——…` |
///
/// ⇒ 判据红灯的**直接原因在参照物**（固定数据过期），引擎照固定数据画才是错的。
/// ⇒ 处置：本宿主以**生产源**为准（覆盖这一条 desc）⇒ 主线修好固定数据后
/// 删掉这个覆盖即可（见报告的「需要主线改的共享文件」）。
pub const FILE_WRITE_DESC_PRODUCTION_TRUTH: &str = "文件写入总闸：apply_patch 与 shell 写命令同受此闸——补丁式受控写文件（创建/修改/删除须在补丁里声明）+ shell 重定向/删除类写命令（关闸即拒绝即帧），每次写入默认要人工审批；工作区外路径与凭据/密钥面拒绝；git 提交等写操作不提供工具（设计边界）。";

/// 固定数据的 JSON 值（**复用主题面的极小解析器** ⇒ 同一 crate 内不重复实现）。
type Json = crate::settings_theme::Json;

// ── 固定数据的装配 ──────────────────────────────────────────────────────────

/// 本面要用的固定数据（**装配后**）。
#[derive(Debug, Clone, PartialEq)]
pub struct CapabilityFixture {
    /// 能力表（键 → 取值）。
    pub capabilities: CapabilityTable,
    /// 决策组定义（评审 / 议会）。
    pub decision_defs: Vec<CapabilityDef>,
    /// 治理组定义（洋葱 / worktree）。
    pub guard_defs: Vec<CapabilityDef>,
    /// 工具组定义（shell / fetch / 本地只读 / 文件写入）。
    pub tool_defs: Vec<CapabilityDef>,
    /// 工具子开关定义（依赖父行 ⇒ 渲染成嵌套行）。
    pub tool_sub_defs: Vec<CapabilityDef>,
    /// Beta 组定义（思考模式）。
    pub beta_defs: Vec<CapabilityDef>,
    /// 认知深度下拉的初值（`light` / `balanced` / `deep` / `custom`）。
    pub cognitive_depth: String,
    /// 全局权限预设的初值。
    pub permission_preset: String,
    /// 允许连接的自建服务器（初值）。
    pub allowed_servers: Vec<String>,
    /// 允许服务器输入框的初值。
    pub allowed_server_input: String,
    /// 允许服务器输入框的错误文案（初值）。
    pub allowed_server_error: String,
    /// 字段旗标（键 → 状态）。
    pub text_flags: Vec<(String, FieldFlagState)>,
    /// 思考模式过滤器的初值（**文本旋钮** ⇒ 生产在 `capabilities` 里，是字符串）。
    pub reasoning_model_filters: String,
    /// reasoning 标签的初值（同上）。
    pub reasoning_tag: String,
    /// 客户端配置 JSON 的预览文本（**宿主生成** ⇒ 逐字节对齐脚本侧）。
    pub config_json_preview: String,
}

impl CapabilityFixture {
    /// 读固定数据并装配（**固定数据坏了 ⇒ 显式报错**）。
    pub fn load() -> Result<Self, String> {
        let capabilities = crate::settings_theme::parse_json(CAPABILITIES_FIXTURE_JSON)
            .map_err(|error| format!("能力固定数据解析失败：{error}"))?;
        let provider = crate::settings_theme::parse_json(PROVIDER_FIXTURE_JSON)
            .map_err(|error| format!("服务商固定数据解析失败：{error}"))?;

        let capability_table = read_capability_table(&capabilities)?;
        let decision_defs = read_cap_defs(&capabilities, "decision")?;
        let guard_defs = read_cap_defs(&capabilities, "guard")?;
        let mut tool_defs = read_cap_defs(&capabilities, "tool")?;
        // ⚠️ 固定数据的这一条文案已过期（见 [`FILE_WRITE_DESC_PRODUCTION_TRUTH`]）
        // ⇒ 以生产源为准。
        if let Some(def) = tool_defs.iter_mut().find(|def| def.key == "fileWrite") {
            def.desc = String::from(FILE_WRITE_DESC_PRODUCTION_TRUTH);
        }
        let tool_sub_defs = read_cap_defs(&capabilities, "toolSub")?;
        let beta_defs = read_cap_defs(&capabilities, "beta")?;
        let cognitive_depth = read_string(
            capabilities
                .get("cognitiveDepth")
                .ok_or_else(|| "固定数据缺字段：cognitiveDepth".to_owned())?,
            "value",
        )?;
        let permission_preset = read_string(&capabilities, "permissionPreset")?;
        let allowed_servers = read_string_array(&capabilities, "allowedServers")?;
        let allowed_server_input = read_string(&capabilities, "allowedServerInput")?;
        let allowed_server_error = read_string(&capabilities, "allowedServerError")?;
        let text_flags = read_text_flags(&capabilities)?;
        let capability_values = capabilities
            .get("capabilities")
            .ok_or_else(|| "固定数据缺字段：capabilities".to_owned())?;
        let reasoning_model_filters = read_string(capability_values, "reasoningModelFilters")?;
        let reasoning_tag = read_string(capability_values, "reasoningTag")?;
        let config_json_preview = client_config_json(&provider)?;

        Ok(Self {
            capabilities: capability_table,
            decision_defs,
            guard_defs,
            tool_defs,
            tool_sub_defs,
            beta_defs,
            cognitive_depth,
            permission_preset,
            allowed_servers,
            allowed_server_input,
            allowed_server_error,
            text_flags,
            reasoning_model_filters,
            reasoning_tag,
            config_json_preview,
        })
    }

    /// 按能力键找它的定义（**五张表一起找** ⇒ `toggle_capability` 要用 `invert`）。
    pub fn def_by_key(&self, key: &str) -> Option<&CapabilityDef> {
        self.decision_defs
            .iter()
            .chain(self.guard_defs.iter())
            .chain(self.tool_defs.iter())
            .chain(self.tool_sub_defs.iter())
            .chain(self.beta_defs.iter())
            .find(|def| def.key == key)
    }

    /// 五个注册表的容量（`decision` / `guard` / `tool` / `toolSub` / `beta`）。
    pub fn registry_sizes(&self) -> [usize; 5] {
        [
            self.decision_defs.len(),
            self.guard_defs.len(),
            self.tool_defs.len(),
            self.tool_sub_defs.len(),
            self.beta_defs.len(),
        ]
    }
}

/// 取一个对象的键值表（**不是对象 ⇒ 空**）。
fn json_object(value: &Json) -> &[(String, Json)] {
    match value {
        Json::Object(entries) => entries,
        _ => &[],
    }
}

/// 取一个数值（**不是数值 ⇒ `None`**）。
fn json_number(value: &Json) -> Option<f64> {
    match value {
        Json::Number(number) => Some(*number),
        _ => None,
    }
}

/// 读一个字符串字段。
fn read_string(value: &Json, key: &str) -> Result<String, String> {
    value
        .get(key)
        .and_then(|item| item.as_str())
        .map(String::from)
        .ok_or_else(|| format!("固定数据缺字段（或不是字符串）：{key}"))
}

/// 读一个字符串数组字段。
fn read_string_array(value: &Json, key: &str) -> Result<Vec<String>, String> {
    let items = value
        .get(key)
        .and_then(|item| item.as_array())
        .ok_or_else(|| format!("固定数据缺字段（或不是数组）：{key}"))?;
    let mut out = Vec::new();
    for item in items {
        let text = item
            .as_str()
            .ok_or_else(|| format!("固定数据的 {key} 里有非字符串项"))?;
        out.push(String::from(text));
    }
    Ok(out)
}

/// 读能力表（键 → 取值；`_` 开头的键是注释 ⇒ 跳过）。
fn read_capability_table(value: &Json) -> Result<CapabilityTable, String> {
    let entries = value
        .get("capabilities")
        .ok_or_else(|| "固定数据缺字段：capabilities".to_owned())?;
    let mut table = CapabilityTable::new();
    for (key, item) in json_object(entries) {
        if key.starts_with('_') {
            continue;
        }
        if let Some(flag) = item.as_bool() {
            table.insert(key.clone(), CapabilityValue::Bool(flag));
        } else if let Some(number) = json_number(item) {
            table.insert(key.clone(), CapabilityValue::Number(number));
        }
        // 字符串型的旋钮（`reasoningTag` 等）**不进能力表**：它们是文本 property
        // ⇒ 由宿主持有并回写 DOM（能力表只放布尔 / 数值）。
    }
    Ok(table)
}

/// 能力定义里的图标 import 名 ⇒ 抽取出来的几何数据（**class 后缀取真实文件名**）。
fn icon_by_import_name(name: &str) -> Option<&'static apeireth_ui_kit::icon::IconData> {
    Some(match name {
        "Scale" => &face_icons::ICON_SCALE,
        "Users" => &face_icons::ICON_USERS,
        "ShieldCheck" => &face_icons::ICON_SHIELD_CHECK,
        "Workflow" => &face_icons::ICON_WORKFLOW,
        "Terminal" => &face_icons::ICON_TERMINAL,
        "Globe" => &face_icons::ICON_GLOBE,
        "FolderSearch" => &face_icons::ICON_FOLDER_SEARCH,
        "FilePenLine" => &face_icons::ICON_FILE_PEN_LINE,
        "FileCheck2" => &face_icons::ICON_FILE_CHECK_2,
        "Brain" => &face_icons::ICON_BRAIN,
        _ => return None,
    })
}

/// 读一组能力定义（**表的一行 ⇒ 数据**）。
fn read_cap_defs(value: &Json, group: &str) -> Result<Vec<CapabilityDef>, String> {
    let items = value
        .get("capDefs")
        .and_then(|item| item.get(group))
        .and_then(|item| item.as_array())
        .ok_or_else(|| format!("固定数据缺字段：capDefs.{group}"))?;
    let mut defs = Vec::new();
    for item in items {
        let icon_name = read_string(item, "icon")?;
        let icon = icon_by_import_name(&icon_name).ok_or_else(|| {
            format!(
                "能力定义引用了未知图标：{icon_name}（生产 import 名 ⇒ 真实 class 见 SPLIT.md §3）"
            )
        })?;
        let mut def = CapabilityDef::new(
            read_string(item, "key")?,
            icon,
            read_string(item, "label")?,
            read_string(item, "env")?,
            read_string(item, "desc")?,
        );
        if item.get("invert").and_then(|item| item.as_bool()) == Some(true) {
            def = def.inverted();
        }
        if let Some(required) = item.get("requires").and_then(|item| item.as_str()) {
            def = def.requires(required);
        }
        // 生产的依赖子行是 `class="cap-row cap-row-nested"` ⇒ **嵌套展示**语义在
        // class 上 ⇒ 归**定义**（表的一行）⇒ 组件照抄。
        if group == "toolSub" {
            def = def.with_row_class("cap-row-nested");
        }
        defs.push(def);
    }
    Ok(defs)
}

/// 读字段旗标（**缺字段 ⇒ 无旗标**）。
fn read_text_flags(value: &Json) -> Result<Vec<(String, FieldFlagState)>, String> {
    let flags = value
        .get("textFlags")
        .ok_or_else(|| "固定数据缺字段：textFlags".to_owned())?;
    let mut out = Vec::new();
    for (key, item) in json_object(flags) {
        if key.starts_with('_') {
            continue;
        }
        let state = if item.as_bool() == Some(true) {
            FieldFlagState::Dirty
        } else {
            FieldFlagState::None
        };
        out.push((key.clone(), state));
    }
    Ok(out)
}

/// 客户端配置 JSON 的预览（**`JSON.stringify(v, null, 2)` 语义** ⇒ 逐字节相同）。
///
/// ## 口径
///
/// 生产的预览对象是
/// `{baseUrl: config.baseUrl, model: providerModel, provider: config.provider,
/// hasApiKey}` ⇒ **键序即插入序**（`baseUrl` → `model` → `provider` → `hasApiKey`）。
///
/// `provider` 是**对象**（生产的 `provider?: ProviderConfig`）⇒ 它的键序也逐字
/// 照抄参照页的构造顺序（`protocol` → `preset` → `baseUrl` → `apiKey` → `model`
/// → `anthropicVersion`）。
fn client_config_json(provider_fixture: &Json) -> Result<String, String> {
    let state = provider_fixture
        .get("state")
        .ok_or_else(|| "服务商固定数据缺字段：state".to_owned())?;
    let provider = JsonValue::Object(vec![
        (
            String::from("protocol"),
            JsonValue::String(read_string(state, "activeProtocol")?),
        ),
        (
            String::from("preset"),
            JsonValue::String(read_string(state, "activePreset")?),
        ),
        (
            String::from("baseUrl"),
            JsonValue::String(read_string(state, "providerBaseUrl")?),
        ),
        (
            String::from("apiKey"),
            JsonValue::String(String::from(CLIENT_CONFIG_API_KEY)),
        ),
        (
            String::from("model"),
            JsonValue::String(read_string(state, "providerModel")?),
        ),
        (
            String::from("anthropicVersion"),
            JsonValue::String(read_string(state, "anthropicVersion")?),
        ),
    ]);
    let preview = JsonValue::Object(vec![
        (
            String::from("baseUrl"),
            JsonValue::String(String::from(CLIENT_CONFIG_BASE_URL)),
        ),
        (
            String::from("model"),
            JsonValue::String(String::from(CLIENT_CONFIG_MODEL)),
        ),
        (String::from("provider"), provider),
        (
            String::from("hasApiKey"),
            // 生产：`!!config.apiKey && config.apiKey.trim().length > 0`。
            JsonValue::Bool(!CLIENT_CONFIG_API_KEY.trim().is_empty()),
        ),
    ]);
    Ok(preview.to_string_pretty())
}

// ── 宿主状态 ────────────────────────────────────────────────────────────────

/// 宿主状态（**状态归宿主** ⇒ 组件是纯函数）。
#[derive(Clone)]
pub struct CapabilityHost {
    /// 运行时（信号宿主）。
    runtime: Runtime,
    /// 当前分区。
    section: Signal<String>,
    /// 能力表。
    capabilities: Signal<CapabilityTable>,
    /// 生效中的键。
    pending_key: Signal<Option<String>>,
    /// 认知深度。
    cognitive_depth: Signal<String>,
    /// 全局权限预设。
    permission_preset: Signal<String>,
    /// 允许连接的自建服务器。
    allowed_servers: Signal<Vec<String>>,
    /// 允许服务器输入框的取值。
    allowed_server_input: Signal<String>,
    /// 允许服务器的错误文案。
    allowed_server_error: Signal<String>,
    /// 允许服务器的操作进行中。
    allowed_servers_busy: Signal<bool>,
    /// 字段旗标。
    text_flags: Signal<Vec<(String, FieldFlagState)>>,
    /// 文本旋钮的取值（键 → 信号；**生产里它们也是 `capabilities` 的字段**，
    /// 但引擎的能力表只放布尔 / 数值 ⇒ 文本另行持有）。
    text_values: BTreeMap<String, Signal<String>>,
    /// 客户端配置 JSON 的预览（**宿主生成**）。
    config_json_preview: Signal<String>,
    /// 配置 JSON 的复制态。
    config_json_copied: Signal<bool>,
    /// 最近一次意图（**对照脚本可读**）。
    intent: Signal<String>,
    /// 固定数据（**不可变** ⇒ 装配一次）。
    fixture: CapabilityFixture,
    /// 冻结卡组件的 props 适配（**只在建宿主时造一次**）。
    card: CharacterPanelsProps,
}

impl CapabilityHost {
    /// 建一份宿主状态（**固定数据坏了 ⇒ 显式报错**）。
    pub fn new(fixture: CapabilityFixture) -> Self {
        let runtime = Runtime::new();
        let section = runtime.signal(String::from(SECTION_GOVERNANCE));
        let capabilities = runtime.signal(fixture.capabilities.clone());
        let pending_key = runtime.signal(None);
        let cognitive_depth = runtime.signal(fixture.cognitive_depth.clone());
        let permission_preset = runtime.signal(fixture.permission_preset.clone());
        let allowed_servers = runtime.signal(fixture.allowed_servers.clone());
        let allowed_server_input = runtime.signal(fixture.allowed_server_input.clone());
        let allowed_server_error = runtime.signal(fixture.allowed_server_error.clone());
        let allowed_servers_busy = runtime.signal(false);
        let text_flags = runtime.signal(fixture.text_flags.clone());
        let mut text_values = BTreeMap::new();
        text_values.insert(
            String::from("reasoningModelFilters"),
            runtime.signal(fixture.reasoning_model_filters.clone()),
        );
        text_values.insert(
            String::from("reasoningTag"),
            runtime.signal(fixture.reasoning_tag.clone()),
        );
        let config_json_preview = runtime.signal(fixture.config_json_preview.clone());
        let config_json_copied = runtime.signal(false);
        let intent = runtime.signal(String::new());

        // ⚠️ 卡组件的两个意图出口**直接捕获意图信号**（不捕获 `self`）⇒
        // 不会出现「先有宿主还是先有 props」的循环依赖。
        let toggle_intent = intent.clone();
        let toggle: CapabilityIntent = Rc::new(move |key: &str| {
            toggle_intent.set(format!("toggle-capability:{key}"));
        });
        let knob_intent = intent.clone();
        let knob: KnobIntent = Rc::new(move |key: &str| {
            knob_intent.set(format!("set-knob:{key}"));
        });
        let card = card_bridge(&runtime, toggle, knob);

        Self {
            runtime,
            section,
            capabilities,
            pending_key,
            cognitive_depth,
            permission_preset,
            allowed_servers,
            allowed_server_input,
            allowed_server_error,
            allowed_servers_busy,
            text_flags,
            text_values,
            config_json_preview,
            config_json_copied,
            intent,
            fixture,
            card,
        }
    }

    /// 运行时。
    pub fn runtime(&self) -> &Runtime {
        &self.runtime
    }

    /// 当前分区。
    pub fn section(&self) -> Signal<String> {
        self.section.clone()
    }

    /// 切到某个分区（**未知分区 ⇒ 不切** ⇒ 不假装已接线；`security` 不属本面）。
    pub fn select_section(&self, id: &str) -> bool {
        if !SECTIONS.contains(&id) {
            return false;
        }
        self.section.set(String::from(id));
        true
    }

    /// 改一个能力取值（**对照脚本用** ⇒ 把画面推到指定状态）。
    pub fn set_capability(&self, key: &str, value: CapabilityValue) {
        let mut table = self.capabilities.get();
        table.insert(String::from(key), value);
        self.capabilities.set(table);
    }

    /// 读一个能力取值。
    pub fn capability(&self, key: &str) -> Option<CapabilityValue> {
        self.capabilities.get().get(key).copied()
    }

    /// 能力表的快照（**对照脚本用**）。
    pub fn capabilities_table(&self) -> CapabilityTable {
        self.capabilities.get()
    }

    /// **切换**一个能力开关（**逐字照抄生产 `toggleCap` + `handleCapabilityToggle`**）。
    ///
    /// ## 生产口径
    ///
    /// ```text
    /// toggleCap(def):
    ///   handleCapabilityToggle(def.key,
    ///     def.invert ? capabilities[def.key] === true : capabilities[def.key] !== true)
    ///
    /// handleCapabilityToggle(key, checked):
    ///   capabilities[key] = checked
    ///   if (key === 'judge' || key === 'council') cognitiveDepth = 'custom'
    /// ```
    ///
    /// ⚠️ `invert` 是**反向旋钮**（本面五张表里没有，但口径要留全 ⇒ 判反 =
    /// 静默错误）⇒ 与组件侧的 [`apeireth_ui_kit::settings_character::is_cap_on`]
    /// 同源。
    pub fn toggle_capability(&self, key: &str) -> bool {
        let invert = self
            .fixture
            .def_by_key(key)
            .map(|def| def.invert)
            .unwrap_or(false);
        let checked = if invert {
            self.capability(key) == Some(CapabilityValue::Bool(true))
        } else {
            self.capability(key) != Some(CapabilityValue::Bool(true))
        };
        self.set_capability(key, CapabilityValue::Bool(checked));
        if key == "judge" || key == "council" {
            self.cognitive_depth.set(String::from("custom"));
        }
        self.record_intent(&format!("toggle-capability:{key}"));
        checked
    }

    /// 认知深度（**property** ⇒ 宿主回写 DOM）。
    pub fn set_depth(&self, value: &str) {
        self.cognitive_depth.set(String::from(value));
    }

    /// 全局权限预设（**property**）。
    pub fn set_preset(&self, value: &str) {
        self.permission_preset.set(String::from(value));
    }

    /// 顾问数量（**生产 `setCouncilAdvisors` 的逐字翻译**）。
    ///
    /// `Math.min(7, Math.max(1, Math.round(value) || 1))`
    pub fn set_council_advisors(&self, value: f64) {
        let rounded = apeireth_ui_kit::num::round(value);
        // `Math.round(x) || 1` ⇒ `0` / `NaN` 都落到 `1`（脚本侧 `0` 是假值）。
        let fallback = if rounded == 0.0 || rounded.is_nan() {
            1.0
        } else {
            rounded
        };
        // `Math.min(7, Math.max(1, x))` ⇒ `clamp(1, 7)`（等价：`x` 已是非 NaN；
        // `clamp` 的 NaN 传播与 panic 条件在上一步的 `fallback` 之后都不可达）。
        let clamped = fallback.clamp(1.0, 7.0);
        self.set_capability("councilAdvisors", CapabilityValue::Number(clamped));
    }

    /// 单顾问超时（**生产 `setCouncilTimeout` 的逐字翻译**）。
    ///
    /// `Math.max(1000, Math.round(value) || 30000)`
    pub fn set_council_timeout(&self, value: f64) {
        let rounded = apeireth_ui_kit::num::round(value);
        let fallback = if rounded == 0.0 || rounded.is_nan() {
            30000.0
        } else {
            rounded
        };
        self.set_capability(
            "councilTimeoutMs",
            CapabilityValue::Number(fallback.max(1000.0)),
        );
    }

    /// 思考模式过滤器（**原样存**）。
    pub fn set_reasoning_filters(&self, value: &str) {
        self.text_value("reasoningModelFilters")
            .set(String::from(value));
    }

    /// reasoning 标签（**生产 `setReasoningTag`：`value.trim() || 'think'`**）。
    pub fn set_reasoning_tag(&self, value: &str) {
        let trimmed = value.trim();
        let next = if trimmed.is_empty() { "think" } else { trimmed };
        self.text_value("reasoningTag").set(String::from(next));
    }

    /// 允许服务器输入框的取值（**property**）。
    pub fn set_allowed_server_input(&self, value: &str) {
        self.allowed_server_input.set(String::from(value));
    }

    /// 允许服务器列表（**对照脚本用** ⇒ 把画面推到「有服务器」态）。
    pub fn set_allowed_servers(&self, servers: Vec<String>) {
        self.allowed_servers.set(servers);
    }

    /// 文本型旋钮的取值（**宿主侧状态** ⇒ 组件不读它，只由宿主回写 DOM）。
    ///
    /// ⚠️ 生产里这些是 `capabilities` 里的**字符串**字段；本引擎的能力表只放
    /// 布尔 / 数值（[`CapabilityValue`] 没有字符串变体）⇒ 文本取值由宿主另行
    /// 持有（`text_values`）。**对外表现相同**：DOM 上的 property 一样。
    fn text_value(&self, key: &str) -> Signal<String> {
        self.text_values
            .get(key)
            .cloned()
            .expect("文本旋钮的键必须在 text_values 里建好")
    }

    /// 文本旋钮的取值（读）。
    pub fn text(&self, key: &str) -> Option<String> {
        self.text_values.get(key).map(|slot| slot.get())
    }

    /// 生效中的键（**`pending` 态** ⇒ 对照脚本可推）。
    pub fn set_pending_key(&self, key: Option<String>) {
        self.pending_key.set(key);
    }

    /// 复制态（**对照脚本可推**）。
    pub fn set_config_json_copied(&self, copied: bool) {
        self.config_json_copied.set(copied);
    }

    /// 配置 JSON 的预览。
    pub fn config_json_preview(&self) -> String {
        self.config_json_preview.get()
    }

    /// 字段旗标（读）。
    pub fn text_flags(&self) -> Vec<(String, FieldFlagState)> {
        self.text_flags.get()
    }

    /// 记录一条意图（**意图出口 ⇒ 宿主留痕**）。
    pub fn record_intent(&self, intent: &str) {
        self.intent.set(String::from(intent));
    }

    /// 最近一次意图。
    pub fn last_intent(&self) -> String {
        self.intent.get()
    }

    /// 建一份组件 props（**状态归宿主** ⇒ 组件是纯函数）。
    pub fn props(&self) -> CapabilityPanelsProps {
        let depth = self.clone();
        let preset = self.clone();
        let server = self.clone();
        let copy = self.clone();
        let field = self.clone();
        let submit = self.clone();
        CapabilityPanelsProps {
            section: self.section.clone(),
            capabilities: self.capabilities.clone(),
            pending_key: self.pending_key.clone(),
            cognitive_depth: self.cognitive_depth.clone(),
            permission_preset: self.permission_preset.clone(),
            allowed_servers: self.allowed_servers.clone(),
            allowed_server_input: self.allowed_server_input.clone(),
            allowed_server_error: self.allowed_server_error.clone(),
            allowed_servers_busy: self.allowed_servers_busy.clone(),
            text_flags: self.text_flags.clone(),
            config_json_preview: self.config_json_preview.clone(),
            config_json_copied: self.config_json_copied.clone(),
            decision_defs: self.fixture.decision_defs.clone(),
            guard_defs: self.fixture.guard_defs.clone(),
            tool_defs: self.fixture.tool_defs.clone(),
            tool_sub_defs: self.fixture.tool_sub_defs.clone(),
            beta_defs: self.fixture.beta_defs.clone(),
            card: self.card.clone(),
            on_select_depth: Rc::new(move |key: &str| {
                depth.record_intent(&format!("select-depth:{key}"));
            }),
            on_select_preset: Rc::new(move |key: &str| {
                preset.record_intent(&format!("select-preset:{key}"));
            }),
            on_allowed_server: Rc::new(move |action: &str| {
                server.record_intent(&format!("allowed-server:{action}"));
            }),
            on_copy_config: Rc::new(move |key: &str| {
                copy.record_intent(&format!("copy-config:{key}"));
            }),
            on_field_input: Rc::new(move |key: &str| {
                field.record_intent(&format!("field:{key}"));
            }),
            on_submit_group: Rc::new(move |key: &str| {
                submit.record_intent(&format!("submit:{key}"));
            }),
        }
    }

    // ── 回写到 DOM property 的取值（**宿主唯一出口** ⇒ 原生侧可测） ──────────

    /// 认知深度下拉的取值。
    pub fn display_depth(&self) -> String {
        self.cognitive_depth.get()
    }

    /// 全局权限预设下拉的取值。
    pub fn display_preset(&self) -> String {
        self.permission_preset.get()
    }

    /// 允许服务器输入框的取值。
    pub fn display_allowed_server_input(&self) -> String {
        self.allowed_server_input.get()
    }

    /// 顾问数量滑杆的取值（**脚本侧 `String(number)` 语义**）。
    pub fn display_council_advisors(&self) -> String {
        apeireth_ui_kit::num::number_to_string(self.number("councilAdvisors"))
    }

    /// 单顾问超时输入框的取值。
    pub fn display_council_timeout(&self) -> String {
        apeireth_ui_kit::num::number_to_string(self.number("councilTimeoutMs"))
    }

    /// 思考模式过滤器输入框的取值。
    pub fn display_reasoning_filters(&self) -> String {
        self.text(REASONING_FILTERS_VALUE_KEY).unwrap_or_default()
    }

    /// reasoning 标签输入框的取值。
    pub fn display_reasoning_tag(&self) -> String {
        self.text(REASONING_TAG_VALUE_KEY).unwrap_or_default()
    }

    /// 读一个**数值**能力（缺省 ⇒ `0`）。
    pub fn number(&self, key: &str) -> f64 {
        match self.capability(key) {
            Some(CapabilityValue::Number(number)) => number,
            _ => 0.0,
        }
    }

    /// 当前状态的快照（**对照脚本可读**）。
    pub fn state(&self) -> BTreeMap<String, String> {
        let mut state = BTreeMap::new();
        state.insert(String::from("section"), self.section.get());
        state.insert(String::from("cognitiveDepth"), self.cognitive_depth.get());
        state.insert(
            String::from("permissionPreset"),
            self.permission_preset.get(),
        );
        state.insert(
            String::from("configJsonCopied"),
            if self.config_json_copied.get() {
                String::from("true")
            } else {
                String::from("false")
            },
        );
        state.insert(String::from("intent"), self.intent.get());
        for (key, slot) in self.text_values.iter() {
            state.insert(key.clone(), slot.get());
        }
        for (key, value) in self.capabilities.get() {
            let text = match value {
                CapabilityValue::Bool(flag) => {
                    if flag {
                        String::from("true")
                    } else {
                        String::from("false")
                    }
                }
                CapabilityValue::Number(number) => apeireth_ui_kit::num::number_to_string(number),
            };
            state.insert(key, text);
        }
        state
    }

    /// 复制按钮的两种文案（**供对照脚本断言**）。
    pub fn copy_label(&self) -> &'static str {
        if self.config_json_copied.get() {
            COPIED_LABEL
        } else {
            COPY_LABEL
        }
    }
}

// ── wasm 装配（**native 编译不参与** ⇒ 行为由浏览器内对照钉住） ─────────────

/// wasm32 侧的装配。
#[cfg(target_arch = "wasm32")]
pub mod wasm {
    //! ## 诚实边界
    //!
    //! - 本模块整体被 `#[cfg(target_arch = "wasm32")]` 隔离 ⇒ **native 编译不参与**
    //!   ⇒ 它的行为**未经 native 侧验证**（浏览器内的验证由设置页 harness 承担：
    //!   像素对照 + DOM 全等 + 宽高同源断言）。
    //! - **真后端未接线**：数据面是**固定数据**（编译期内嵌的那两份）。
    //! - **原生控件的取值是 property**：生产用 `bind:value` / `value={…}` ⇒ 引擎侧
    //!   **不写** `value` 属性 ⇒ 宿主在每次渲染后**回写 property**（否则像素对照会
    //!   看到空下拉 / 空输入框）。
    //! - **零 wasm 导出**：`set_settings_section` 已被向导面占用 ⇒ 本面不导出任何
    //!   函数（见 [`patch_settings_api`] 与 `window.__CAPABILITY__`）。

    use std::cell::RefCell;
    use std::rc::Rc;

    use apeireth_ui_core::{HostScheduler, UiComponent};
    use apeireth_ui_dom::{Renderer, VNode, WebDom};
    use apeireth_ui_kit::settings_capability::{
        CapabilityPanels, ADVISORS_ARIA_LABEL, ADVISORS_VALUE_KEY, DEPTH_SELECT_ID,
        PRESET_SELECT_ID, REASONING_FILTERS_ARIA_LABEL, REASONING_FILTERS_VALUE_KEY,
        REASONING_TAG_ARIA_LABEL, REASONING_TAG_VALUE_KEY, SERVER_INPUT_ID, TIMEOUT_ARIA_LABEL,
        TIMEOUT_VALUE_KEY,
    };
    use apeireth_ui_macro::view;
    use js_sys::Reflect;
    use wasm_bindgen::prelude::*;
    use web_sys::Element;

    use super::{CapabilityFixture, CapabilityHost};

    /// 微任务调度器：把 flush 排进浏览器的微任务队列。
    struct MicrotaskScheduler;

    impl HostScheduler for MicrotaskScheduler {
        fn schedule_flush(&self, flush: Box<dyn FnOnce()>) {
            let queue = Reflect::get(&js_sys::global(), &JsValue::from_str("queueMicrotask"))
                .ok()
                .and_then(|value| value.dyn_into::<js_sys::Function>().ok());
            match queue {
                Some(queue) => {
                    let callback = Closure::once_into_js(move || {
                        flush();
                        JsValue::UNDEFINED
                    });
                    let _ = queue.call1(&JsValue::UNDEFINED, &callback);
                }
                None => {
                    // 没有微任务队列 ⇒ **同步**跑（不静默丢刷新）。
                    flush();
                }
            }
        }
    }

    thread_local! {
        /// wasm 侧的单例宿主。
        static CAPABILITY_APP: RefCell<Option<CapabilityHost>> = const { RefCell::new(None) };
        /// 控制 API 的闭包（**必须活到页面结束**）。
        static CONTROL_CLOSURES: RefCell<Vec<Closure<dyn FnMut(JsValue) -> JsValue>>> =
            const { RefCell::new(Vec::new()) };
    }

    /// 渲染整棵视图树（**舞台 + 作用域锚点**）。
    fn render_tree(host: &CapabilityHost) -> VNode {
        let panel = CapabilityPanels::render(&host.props());
        view! {
            div(class = "settings-stage") {
                div(class = "sp-capability") {
                    { panel }
                }
            }
        }
    }

    /// 挂载能力与治理面。
    ///
    /// ## 失败口径
    ///
    /// 失败 ⇒ **既返回错误、又发布到页面**（`window.__SETTINGS_ERROR__`）⇒
    /// 调用方忽略返回值时**仍然看得见**（不静默）。
    pub fn mount(container: &Element) -> Result<(), String> {
        match mount_inner(container) {
            Ok(()) => Ok(()),
            Err(error) => {
                report_error(&error);
                Err(error)
            }
        }
    }

    /// 挂载（内部实现）。
    fn mount_inner(container: &Element) -> Result<(), String> {
        let dom = WebDom::new().map_err(|error| error.to_string())?;
        let root: web_sys::Node = container.clone().into();
        let mut renderer = Renderer::new(dom, root);

        let fixture = CapabilityFixture::load()?;
        let host = CapabilityHost::new(fixture);
        host.runtime()
            .set_host_scheduler(Rc::new(MicrotaskScheduler));

        renderer
            .mount(render_tree(&host))
            .map_err(|error| error.to_string())?;

        let renderer = Rc::new(RefCell::new(renderer));
        let effect_renderer = renderer.clone();
        let effect_host = host.clone();
        let effect_container = container.clone();
        host.runtime().effect(move || {
            let tree = render_tree(&effect_host);
            let mut guard = effect_renderer.borrow_mut();
            let _ = guard.patch(tree);
            drop(guard);
            publish_state(&effect_host);
        });

        CAPABILITY_APP.with(|slot| {
            *slot.borrow_mut() = Some(host.clone());
        });
        install_control_api();
        patch_settings_api();
        mark_ready();
        publish_state(&host);
        Ok(())
    }

    /// 标记「画面已就绪」（**共享 harness 的 `ready()` 读它**）。
    fn mark_ready() {
        let Some(window) = web_sys::window() else {
            return;
        };
        let _ = Reflect::set(
            window.as_ref(),
            &JsValue::from_str("__SETTINGS_READY__"),
            &JsValue::TRUE,
        );
    }

    /// 记录一条**显式错误**（对照脚本可读；不静默）。
    fn report_error(message: &str) {
        let Some(window) = web_sys::window() else {
            return;
        };
        let _ = Reflect::set(
            window.as_ref(),
            &JsValue::from_str("__SETTINGS_ERROR__"),
            &JsValue::from_str(message),
        );
    }

    /// 把宿主状态**发布**到页面的全局（对照脚本只读）。
    fn publish_state(host: &CapabilityHost) {
        let Some(window) = web_sys::window() else {
            return;
        };
        let state = host.state();
        let json = serde_free_json(&state);
        let _ = Reflect::set(
            window.as_ref(),
            &JsValue::from_str("__CAPABILITY_STATE__"),
            &JsValue::from_str(&json),
        );
        let _ = Reflect::set(
            window.as_ref(),
            &JsValue::from_str("__CAPABILITY_INTENT__"),
            &JsValue::from_str(&host.last_intent()),
        );
    }

    /// 状态 ⇒ JSON 文本（**零依赖**：只序列化字符串键值 ⇒ 逐字节可控）。
    fn serde_free_json(state: &std::collections::BTreeMap<String, String>) -> String {
        let mut out = String::from("{");
        for (index, (key, value)) in state.iter().enumerate() {
            if index > 0 {
                out.push(',');
            }
            out.push('"');
            out.push_str(&escape_json(key));
            out.push_str("\":\"");
            out.push_str(&escape_json(value));
            out.push('"');
        }
        out.push('}');
        out
    }

    /// JSON 字符串转义（**只处理引号 / 反斜杠 / 控制符** ⇒ 本仓数据够用）。
    fn escape_json(text: &str) -> String {
        let mut out = String::new();
        for ch in text.chars() {
            match ch {
                '"' => out.push_str("\\\""),
                '\\' => out.push_str("\\\\"),
                '\n' => out.push_str("\\n"),
                '\r' => out.push_str("\\r"),
                '\t' => out.push_str("\\t"),
                other if (other as u32) < 0x20 => {
                    out.push_str(&format!("\\u{:04x}", other as u32));
                }
                other => out.push(other),
            }
        }
        out
    }

    /// 读一个数组实参的第 `index` 项（字符串）。
    fn string_arg(args: &JsValue, index: f64) -> String {
        Reflect::get(args, &JsValue::from_f64(index))
            .ok()
            .and_then(|value| value.as_string())
            .unwrap_or_default()
    }

    /// 装控制 API（`window.__CAPABILITY__`）—— 对照脚本用它设置状态。
    fn install_control_api() {
        let Some(window) = web_sys::window() else {
            return;
        };

        let select_section = Closure::wrap(Box::new(|args: JsValue| -> JsValue {
            let id = args.as_string().unwrap_or_default();
            CAPABILITY_APP.with(|slot| {
                if let Some(host) = slot.borrow().as_ref() {
                    host.select_section(&id);
                }
            });
            JsValue::UNDEFINED
        }) as Box<dyn FnMut(JsValue) -> JsValue>);

        let set_capability = Closure::wrap(Box::new(|args: JsValue| -> JsValue {
            // **一个数组参数**（闭包只拿第一个实参 ⇒ 两个字段打包成数组）。
            let key = string_arg(&args, 0.0);
            let raw = string_arg(&args, 1.0);
            let value = match raw.as_str() {
                "true" => apeireth_ui_kit::settings_character::CapabilityValue::Bool(true),
                "false" => apeireth_ui_kit::settings_character::CapabilityValue::Bool(false),
                other => apeireth_ui_kit::settings_character::CapabilityValue::Number(
                    other.parse::<f64>().unwrap_or_default(),
                ),
            };
            CAPABILITY_APP.with(|slot| {
                if let Some(host) = slot.borrow().as_ref() {
                    host.set_capability(&key, value);
                }
            });
            JsValue::UNDEFINED
        }) as Box<dyn FnMut(JsValue) -> JsValue>);

        let toggle_capability = Closure::wrap(Box::new(|args: JsValue| -> JsValue {
            let key = args.as_string().unwrap_or_default();
            CAPABILITY_APP.with(|slot| {
                if let Some(host) = slot.borrow().as_ref() {
                    host.toggle_capability(&key);
                }
            });
            JsValue::UNDEFINED
        }) as Box<dyn FnMut(JsValue) -> JsValue>);

        let set_depth = Closure::wrap(Box::new(|args: JsValue| -> JsValue {
            let value = args.as_string().unwrap_or_default();
            CAPABILITY_APP.with(|slot| {
                if let Some(host) = slot.borrow().as_ref() {
                    host.set_depth(&value);
                }
            });
            JsValue::UNDEFINED
        }) as Box<dyn FnMut(JsValue) -> JsValue>);

        let set_preset = Closure::wrap(Box::new(|args: JsValue| -> JsValue {
            let value = args.as_string().unwrap_or_default();
            CAPABILITY_APP.with(|slot| {
                if let Some(host) = slot.borrow().as_ref() {
                    host.set_preset(&value);
                }
            });
            JsValue::UNDEFINED
        }) as Box<dyn FnMut(JsValue) -> JsValue>);

        let set_text = Closure::wrap(Box::new(|args: JsValue| -> JsValue {
            let key = string_arg(&args, 0.0);
            let raw = string_arg(&args, 1.0);
            CAPABILITY_APP.with(|slot| {
                let Some(host) = slot.borrow().as_ref().cloned() else {
                    return;
                };
                match key.as_str() {
                    TIMEOUT_VALUE_KEY => {
                        host.set_council_timeout(raw.parse::<f64>().unwrap_or_default());
                    }
                    ADVISORS_VALUE_KEY => {
                        host.set_council_advisors(raw.parse::<f64>().unwrap_or_default());
                    }
                    REASONING_FILTERS_VALUE_KEY => host.set_reasoning_filters(&raw),
                    REASONING_TAG_VALUE_KEY => host.set_reasoning_tag(&raw),
                    _ => host.set_allowed_server_input(&raw),
                }
            });
            JsValue::UNDEFINED
        }) as Box<dyn FnMut(JsValue) -> JsValue>);

        let set_servers = Closure::wrap(Box::new(|args: JsValue| -> JsValue {
            let mut servers = Vec::new();
            if let Ok(length) = Reflect::get(&args, &JsValue::from_str("length"))
                .map(|value| value.as_f64().unwrap_or_default())
            {
                for index in 0..(length as u32) {
                    servers.push(string_arg(&args, f64::from(index)));
                }
            }
            CAPABILITY_APP.with(|slot| {
                if let Some(host) = slot.borrow().as_ref() {
                    host.set_allowed_servers(servers);
                }
            });
            JsValue::UNDEFINED
        }) as Box<dyn FnMut(JsValue) -> JsValue>);

        let set_copied = Closure::wrap(Box::new(|args: JsValue| -> JsValue {
            let copied = args.as_bool().unwrap_or(false);
            CAPABILITY_APP.with(|slot| {
                if let Some(host) = slot.borrow().as_ref() {
                    host.set_config_json_copied(copied);
                }
            });
            JsValue::UNDEFINED
        }) as Box<dyn FnMut(JsValue) -> JsValue>);

        let set_pending = Closure::wrap(Box::new(|args: JsValue| -> JsValue {
            let key = args.as_string().unwrap_or_default();
            CAPABILITY_APP.with(|slot| {
                if let Some(host) = slot.borrow().as_ref() {
                    host.set_pending_key(if key.is_empty() { None } else { Some(key) });
                }
            });
            JsValue::UNDEFINED
        }) as Box<dyn FnMut(JsValue) -> JsValue>);

        let get_state = Closure::wrap(Box::new(|_args: JsValue| -> JsValue {
            let state = CAPABILITY_APP.with(|slot| {
                slot.borrow()
                    .as_ref()
                    .map(CapabilityHost::state)
                    .unwrap_or_default()
            });
            JsValue::from_str(&serde_free_json(&state))
        }) as Box<dyn FnMut(JsValue) -> JsValue>);

        let api = js_sys::Object::new();
        let bind = |key: &str, closure: &Closure<dyn FnMut(JsValue) -> JsValue>| {
            let function: &js_sys::Function = closure.as_ref().unchecked_ref();
            let _ = Reflect::set(api.as_ref(), &JsValue::from_str(key), function.as_ref());
        };
        bind("selectSection", &select_section);
        bind("setCapability", &set_capability);
        bind("toggleCapability", &toggle_capability);
        bind("setDepth", &set_depth);
        bind("setPreset", &set_preset);
        bind("setText", &set_text);
        bind("setServers", &set_servers);
        bind("setCopied", &set_copied);
        bind("setPending", &set_pending);
        bind("getState", &get_state);
        let _ = Reflect::set(
            api.as_ref(),
            &JsValue::from_str("ready"),
            &JsValue::from_bool(true),
        );
        let _ = Reflect::set(
            window.as_ref(),
            &JsValue::from_str("__CAPABILITY__"),
            api.as_ref(),
        );

        CONTROL_CLOSURES.with(|slot| {
            let mut slot = slot.borrow_mut();
            slot.push(select_section);
            slot.push(set_capability);
            slot.push(toggle_capability);
            slot.push(set_depth);
            slot.push(set_preset);
            slot.push(set_text);
            slot.push(set_servers);
            slot.push(set_copied);
            slot.push(set_pending);
            slot.push(get_state);
        });
    }

    /// 包裹共享 API 的 `selectSection`（**幂等** ⇒ 共享契约对本面也生效）。
    ///
    /// ## 为什么必须包
    ///
    /// 共享的 `web/engine/main.js` 建 `window.__SETTINGS__` 时，`selectSection`
    /// 里只调 `module.set_settings_section(id)`（**向导面的导出**）⇒ 本面
    /// **零 wasm 导出** ⇒ 那句对本面是空操作 ⇒ 共享入口（`tools/compare_settings.py
    /// --panel capability`）切分区会**无效**。
    ///
    /// ⇒ 这里把原函数包一层：**先调原来的**（保持共享契约的副作用：写
    /// `window.__SETTINGS_SECTION__`），再切本面分区。
    fn patch_settings_api() {
        let Some(window) = web_sys::window() else {
            return;
        };
        let Ok(api) = Reflect::get(window.as_ref(), &JsValue::from_str("__SETTINGS__")) else {
            return;
        };
        if api.is_null() || api.is_undefined() {
            return;
        }
        let Ok(original) = Reflect::get(&api, &JsValue::from_str("selectSection")) else {
            return;
        };
        if original.is_null() || original.is_undefined() {
            return;
        }
        let Ok(original) = original.dyn_into::<js_sys::Function>() else {
            return;
        };
        let wrapped = Closure::wrap(Box::new(move |args: JsValue| -> JsValue {
            let _ = original.call1(&JsValue::NULL, &args);
            let id = args.as_string().unwrap_or_default();
            CAPABILITY_APP.with(|slot| {
                if let Some(host) = slot.borrow().as_ref() {
                    host.select_section(&id);
                }
            });
            JsValue::UNDEFINED
        }) as Box<dyn FnMut(JsValue) -> JsValue>);
        let function: &js_sys::Function = wrapped.as_ref().unchecked_ref();
        let _ = Reflect::set(&api, &JsValue::from_str("selectSection"), function.as_ref());
        CONTROL_CLOSURES.with(|slot| slot.borrow_mut().push(wrapped));
    }

    /// 回写 DOM property（**下拉 / 输入框 / 滑杆的取值**）。
    ///
    /// ## 为什么必须回写
    ///
    /// 生产用 `bind:value` / `value={…}` ⇒ Svelte 对 `<input>` / `<select>` 的
    /// `value` **编译成 property 赋值** ⇒ DOM 上**没有** `value` 属性
    /// ⇒ 引擎侧也**不写属性**（属性对照要能区分「缺失」与「空串」）。
    ///
    /// ⚠️ **不回写的后果**：`ui-dom` 渲染器只有属性通道 ⇒ 刚挂载的 `<select>`
    /// 没有取值 ⇒ 浏览器按**第一个 option** 渲染 ⇒ 与生产不一致；`<input>` 同理
    /// （空框）。M6 已实测过这条（滑杆差 2,463 px）。
    ///
    /// 按选择器取一个元素。
    fn query(container: &Element, selector: &str) -> Option<Element> {
        let found = Reflect::get(container.as_ref(), &JsValue::from_str("querySelector"))
            .ok()?
            .dyn_into::<js_sys::Function>()
            .ok()?
            .call1(container.as_ref(), &JsValue::from_str(selector))
            .ok()?;
        found.dyn_into::<Element>().ok()
    }
}

#[cfg(test)]
mod tests {
    //! 宿主侧的**行为**断言（固定数据 / 状态 / setter / 意图 / 回写取值），
    //! **不读生产源码**。

    use super::*;
    use apeireth_ui_kit::settings_capability::{
        ADVISORS_VALUE_KEY, REASONING_FILTERS_VALUE_KEY, REASONING_TAG_VALUE_KEY,
        SANDBOX_VALUE_KEY, TIMEOUT_VALUE_KEY,
    };

    /// 装配固定数据（**固定数据坏了 ⇒ 测试直接失败**）。
    fn fixture() -> CapabilityFixture {
        CapabilityFixture::load().expect("固定数据必须可装配")
    }

    /// 建一份宿主。
    fn host() -> CapabilityHost {
        CapabilityHost::new(fixture())
    }

    #[test]
    fn the_fixture_carries_the_five_production_registries() {
        // 生产：decision 2 / guard 2 / tool 4 / toolSub 1 / beta 1。
        assert_eq!(fixture().registry_sizes(), [2, 2, 4, 1, 1]);
    }

    #[test]
    fn the_fixture_defaults_match_the_production_values() {
        let fixture = fixture();
        assert_eq!(fixture.cognitive_depth, "light");
        assert_eq!(fixture.permission_preset, "standard");
        assert_eq!(fixture.allowed_servers.len(), 0);
        assert_eq!(fixture.allowed_server_input, "");
        assert_eq!(fixture.allowed_server_error, "");
        // 数值旋钮（**逐字对生产源文件核过**：默认 3 位顾问 / 30000ms 超时）。
        assert_eq!(
            fixture.capabilities.get("councilAdvisors"),
            Some(&CapabilityValue::Number(3.0))
        );
        assert_eq!(
            fixture.capabilities.get("councilTimeoutMs"),
            Some(&CapabilityValue::Number(30000.0))
        );
        // 开关：沙箱默认开、本地只读工具默认开、文件写入默认关、思考模式默认关。
        assert_eq!(
            fixture.capabilities.get("shellSandbox"),
            Some(&CapabilityValue::Bool(true))
        );
        assert_eq!(
            fixture.capabilities.get("localReadTools"),
            Some(&CapabilityValue::Bool(true))
        );
        assert_eq!(
            fixture.capabilities.get("fileWrite"),
            Some(&CapabilityValue::Bool(false))
        );
        assert_eq!(
            fixture.capabilities.get("reasoningEnabled"),
            Some(&CapabilityValue::Bool(false))
        );
        // 文本旋钮的初值。
        assert_eq!(fixture.reasoning_model_filters, "");
        assert_eq!(fixture.reasoning_tag, "think");
        // 依赖子行确实带 `requires`（**两态判据的锚点**）。
        let sub = &fixture.tool_sub_defs[0];
        assert_eq!(sub.key, "fileWriteAutoPass");
        assert_eq!(sub.requires.as_deref(), Some("fileWrite"));
        assert_eq!(sub.row_class.as_deref(), Some("cap-row-nested"));
    }

    #[test]
    fn the_client_config_preview_is_byte_identical_to_the_script_side() {
        // **逐字节**期望值 = 参照页 `JSON.stringify({…}, null, 2)` 的输出
        // （浏览器实测 dump，2026-10-24）。
        let expected = "{\n  \"baseUrl\": \"http://127.0.0.1:8080\",\n  \"model\": \"gpt-4o\",\n  \"provider\": {\n    \"protocol\": \"openai\",\n    \"preset\": \"openai\",\n    \"baseUrl\": \"https://api.openai.com/v1\",\n    \"apiKey\": \"\",\n    \"model\": \"gpt-4o\",\n    \"anthropicVersion\": \"2023-06-01\"\n  },\n  \"hasApiKey\": false\n}";
        assert_eq!(fixture().config_json_preview, expected);
    }

    #[test]
    fn the_provider_block_keeps_the_insertion_order() {
        let preview = fixture().config_json_preview;
        let protocol = preview.find("\"protocol\"").expect("缺 protocol");
        let preset = preview.find("\"preset\"").expect("缺 preset");
        let base_url = preview
            .find("\"baseUrl\": \"https://")
            .expect("缺 provider.baseUrl");
        let api_key = preview.find("\"apiKey\"").expect("缺 apiKey");
        let model = preview
            .find("\"model\": \"gpt-4o\",\n    \"anthropicVersion\"")
            .expect("缺 provider.model");
        let version = preview
            .find("\"anthropicVersion\"")
            .expect("缺 anthropicVersion");
        // 键序：protocol → preset → baseUrl → apiKey → model → anthropicVersion。
        assert!(protocol < preset);
        assert!(preset < base_url);
        assert!(base_url < api_key);
        assert!(api_key < model);
        assert!(model < version);
        // 顶层键序：baseUrl → model → provider → hasApiKey。
        let top_base = preview.find("\"baseUrl\"").expect("缺 baseUrl");
        let top_model = preview.find("\"model\"").expect("缺 model");
        let provider = preview.find("\"provider\"").expect("缺 provider");
        let has_key = preview.find("\"hasApiKey\"").expect("缺 hasApiKey");
        assert!(top_base < top_model);
        assert!(top_model < provider);
        assert!(provider < has_key);
    }

    #[test]
    fn the_host_switches_sections_and_ignores_unknown_ones() {
        let host = host();
        assert_eq!(host.section().get(), SECTION_GOVERNANCE);
        assert!(host.select_section("tools"));
        assert_eq!(host.section().get(), "tools");
        assert!(host.select_section("developer"));
        assert_eq!(host.section().get(), "developer");
        // ⚠️ `security` **不属本面**（那一段委派给独立面）⇒ 切不过去。
        assert!(!host.select_section("security"));
        assert_eq!(host.section().get(), "developer");
        assert!(!host.select_section("nope"));
    }

    #[test]
    fn the_host_toggles_a_capability_and_derives_the_custom_depth() {
        let host = host();
        assert_eq!(
            host.capability("council"),
            Some(CapabilityValue::Bool(false))
        );
        assert!(host.toggle_capability("council"), "关 ⇒ 开");
        assert_eq!(
            host.capability("council"),
            Some(CapabilityValue::Bool(true))
        );
        // 生产：judge/council 一动 ⇒ 认知深度落到「自定义」。
        assert_eq!(host.display_depth(), "custom");
        assert!(!host.toggle_capability("council"), "开 ⇒ 关");
        // 其它键不动认知深度（先把深度复位再验）。
        host.set_depth("light");
        host.toggle_capability("fileWrite");
        assert_eq!(host.display_depth(), "light");
        assert_eq!(
            host.capability("fileWrite"),
            Some(CapabilityValue::Bool(true))
        );
    }

    #[test]
    fn the_host_mirrors_the_production_knob_setters() {
        let host = host();
        // `Math.min(7, Math.max(1, Math.round(value) || 1))`
        host.set_council_advisors(9.0);
        assert_eq!(host.number(ADVISORS_VALUE_KEY), 7.0);
        host.set_council_advisors(0.0);
        assert_eq!(host.number(ADVISORS_VALUE_KEY), 1.0);
        host.set_council_advisors(3.4);
        assert_eq!(host.number(ADVISORS_VALUE_KEY), 3.0);
        // `Math.max(1000, Math.round(value) || 30000)`
        host.set_council_timeout(0.0);
        assert_eq!(host.number(TIMEOUT_VALUE_KEY), 30000.0);
        host.set_council_timeout(500.0);
        assert_eq!(host.number(TIMEOUT_VALUE_KEY), 1000.0);
        host.set_council_timeout(45000.0);
        assert_eq!(host.number(TIMEOUT_VALUE_KEY), 45000.0);
        // `setReasoningTag`：`value.trim() || 'think'`。
        host.set_reasoning_tag("   ");
        assert_eq!(host.text(REASONING_TAG_VALUE_KEY).as_deref(), Some("think"));
        host.set_reasoning_tag("  think2  ");
        assert_eq!(
            host.text(REASONING_TAG_VALUE_KEY).as_deref(),
            Some("think2")
        );
        // 过滤器原样存（生产 `setReasoningFilters` 不裁剪）。
        host.set_reasoning_filters(" deepseek,o3 ");
        assert_eq!(
            host.text(REASONING_FILTERS_VALUE_KEY).as_deref(),
            Some(" deepseek,o3 ")
        );
    }

    #[test]
    fn the_display_values_are_what_the_host_writes_to_the_dom() {
        // 回写到 property 的取值 ⇒ 必须与脚本侧的 `String(number)` 同口径。
        let host = host();
        assert_eq!(host.display_council_advisors(), "3");
        assert_eq!(host.display_council_timeout(), "30000");
        assert_eq!(host.display_reasoning_tag(), "think");
        assert_eq!(host.display_reasoning_filters(), "");
        assert_eq!(host.display_depth(), "light");
        assert_eq!(host.display_preset(), "standard");
        assert_eq!(host.display_allowed_server_input(), "");
        host.set_depth("custom");
        host.set_preset("full");
        assert_eq!(host.display_depth(), "custom");
        assert_eq!(host.display_preset(), "full");
    }

    #[test]
    fn the_host_records_intents_instead_of_changing_state() {
        let host = host();
        let props = host.props();
        (props.on_select_depth)("cognitive-depth");
        assert_eq!(host.last_intent(), "select-depth:cognitive-depth");
        (props.on_select_preset)("permission-preset");
        assert_eq!(host.last_intent(), "select-preset:permission-preset");
        (props.on_allowed_server)("add");
        assert_eq!(host.last_intent(), "allowed-server:add");
        (props.on_copy_config)("copy-config");
        assert_eq!(host.last_intent(), "copy-config:copy-config");
        (props.on_field_input)("reasoning.tag");
        assert_eq!(host.last_intent(), "field:reasoning.tag");
        (props.on_submit_group)("reasoning");
        assert_eq!(host.last_intent(), "submit:reasoning");
        // 意图**不改变状态**（状态归宿主 ⇒ 由宿主自己的方法改）。
        assert_eq!(host.display_preset(), "standard");
        assert_eq!(host.display_allowed_server_input(), "");
    }

    #[test]
    fn the_card_bridge_wires_the_frozen_row_component_to_the_host_intents() {
        let host = host();
        let props = host.props();
        // 冻结的行组件从这个适配 props 里取两个出口 ⇒ 必须真的接到宿主。
        (props.card.on_toggle_capability)("judge");
        assert_eq!(host.last_intent(), "toggle-capability:judge");
        (props.card.on_set_knob)(ADVISORS_VALUE_KEY);
        assert_eq!(host.last_intent(), "set-knob:councilAdvisors");
    }

    #[test]
    fn the_fixture_text_flags_start_without_flags() {
        // 固定数据的三个旗标键都是 `false` ⇒ 全部落到「无旗标」（不渲染节点）。
        let flags = fixture().text_flags;
        assert_eq!(flags.len(), 3);
        assert!(flags
            .iter()
            .all(|(_, state)| *state == FieldFlagState::None));
    }

    #[test]
    fn the_host_can_be_pushed_to_the_extra_states_the_compare_needs() {
        let host = host();
        // 沙箱关（工具分区的告警分支）。
        host.set_capability(SANDBOX_VALUE_KEY, CapabilityValue::Bool(false));
        assert_eq!(
            host.capability(SANDBOX_VALUE_KEY),
            Some(CapabilityValue::Bool(false))
        );
        // 允许服务器（列表分支）。
        host.set_allowed_servers(vec![String::from("relay.example.com:8443")]);
        assert_eq!(host.allowed_servers.get().len(), 1);
        // 复制态（按钮的第二文案）。
        host.set_config_json_copied(true);
        assert_eq!(host.copy_label(), COPIED_LABEL);
        host.set_config_json_copied(false);
        assert_eq!(host.copy_label(), COPY_LABEL);
        // pending 态（行内「正在应用」）。
        host.set_pending_key(Some(String::from("judge")));
        assert_eq!(host.pending_key.get().as_deref(), Some("judge"));
    }
}
