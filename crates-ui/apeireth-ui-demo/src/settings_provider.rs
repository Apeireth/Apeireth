//! 设置页三页 · `provider` 子面的 **wasm 宿主挂载**模块（服务商与连接）。
//!
//! ## 本模块是什么
//!
//! **宿主侧**：持有本面的全部状态（协议 / 预设 / 端点 / 模型 / 密钥 / 测连接
//! 回执 / 热应用回执 / 钥匙串回显 / 字段旗标）+ **固定数据**
//! （`fixtures/provider.json`），把 `ui_kit::settings_provider` 的组件挂到页面的
//! `#app` 容器里。
//!
//! ## 诚实边界（**引擎侧只发意图、不假装已接线**）
//!
//! | 事项 | 口径 |
//! |---|---|
//! | 测连接（`testProviderConnection`） | **HTTP** ⇒ 不迁 ⇒ 组件只发意图；宿主用**固定回执**推进画面（**不是**"已接线"） |
//! | 模型发现（`listModels`） | **HTTP** ⇒ 同上（固定数据里的 3 条） |
//! | 网关热应用（`applyAdminConfig` / `getAdminConfig`） | **HTTP** ⇒ 同上（固定回执） |
//! | 钥匙串（读 / 写 / 删） | **宿主能力** ⇒ 不迁 ⇒ 组件只发意图；宿主只动**内存里的回显串** |
//! | 整组落盘（`onSave`） | **宿主** ⇒ 组件发**一次**整组意图；宿主记下快照 |
//! | 打码 | `ui_kit::settings_provider::mask_api_key`（展示口径）；**明文密钥不落虚拟树** |
//! | `SessionModelPicker` | **不迁**（闭合态占位）；下拉未接线 |
//! | 原生控件的取值 | 生产用 **property**（`bind:value`）⇒ 宿主在每次渲染后**回写 property**；`input` 事件时**从 DOM 读回** |
//!
//! ## 固定数据的**键名映射**（面内完成 ⇒ 不改共享 fixture）
//!
//! | 固定数据的键 | 本模块的落点 | 说明 |
//! |---|---|---|
//! | `presets.{openai,anthropic}[]` | [`ProviderFixture::presets`] | 逐字喂组件 |
//! | `defaultModelId` | [`DEFAULT_MODEL_ID`]（组件常量） | 固定数据里的值必须与生产常量**逐字相同**（本模块有断言） |
//! | `state.*` | 宿主信号初值 | 组状态 |
//! | `discoveredModels[]` | [`ProviderFixture::discovered_models`] | 模型选择器占位的 `models` |
//! | `testResult` | 宿主信号（**初值 `None`**） | 点「测试连接」后**灌入**（固定回执） |
//! | `applyResult` / `effectiveConfig` | 宿主信号（**初值 `None`**） | 整组提交后**灌入**（固定回执） |
//! | `keychain.*` | 宿主信号 | 模态的钥匙串回显（**只存打码串**） |
//! | `textFlags{}` | 宿主信号 | `true` ⇒ 未保存；本模块按事件推进 |
//!
//! ⚠️ **为什么 `testResult` / `applyResult` / `effectiveConfig` 的初值是 `None`**：
//! 参考侧是**生产初始态**（`testResult = null` / `applyResult = null` /
//! `effectiveConfig = null`）⇒ 引擎侧若一上来就把固定数据里的**回执**渲染出来
//! ⇒ 两侧首屏必然不等。⇒ 这三份固定数据是**宿主按意图灌入的回执素材**
//! （对应生产里 HTTP 回来的那一刻），\u{800c}不是首屏状态。

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use apeireth_ui_core::{Runtime, Signal};
use apeireth_ui_kit::settings_frame::FieldFlagState;
use apeireth_ui_kit::settings_provider::{
    mask_api_key, ApplyResult, DiscoveredModel, EffectiveConfig, ProviderGroupDraft,
    ProviderGroupProps, ProviderPreset, ProviderPresets, TestResult, PROVIDER_TEXT_KEYS,
};

/// 固定数据的 JSON 值（**复用主题面的极小解析器** ⇒ 同一 crate 内不重复实现）。
type Json = crate::settings_theme::Json;

/// 服务商与连接的固定数据（**编译期内嵌** ⇒ 与参考侧同一份文件）。
pub const PROVIDER_FIXTURE_JSON: &str = include_str!("../harness/settings/fixtures/provider.json");

/// 协议的稳定取值（`openai`）。
pub const PROTOCOL_OPENAI: &str = "openai";

/// 协议的稳定取值（`anthropic`）。
pub const PROTOCOL_ANTHROPIC: &str = "anthropic";

/// 分区的稳定 id：`models`。
pub const SECTION_MODELS: &str = "models";

/// 分区的稳定 id：API Key 模态（**整视口**截图）。
pub const SECTION_API_KEY_MODAL: &str = "api-key-modal";

/// 「未保存」旗标的清除延迟（**生产 1.6s** ⇒ 字段旗标闪一下就收）。
pub const ACK_HOLD_MS: i32 = 1600;

/// 模态里「请输入 API Key」的错误文案（生产 `saveNewApiKey` 的早退分支）。
pub const EMPTY_KEY_ERROR: &str = "请输入 API Key";

// ── 固定数据的装配 ──────────────────────────────────────────────────────────

/// `state` 里的组状态（固定数据 ⇒ 宿主信号初值）。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FixtureState {
    /// 协议。
    pub protocol: String,
    /// 预设 id。
    pub preset: String,
    /// 端点。
    pub base_url: String,
    /// 密钥（**固定数据里必须是空或占位串**）。
    pub api_key: String,
    /// 活动模型。
    pub model: String,
    /// 协议头。
    pub anthropic_version: String,
    /// 是否明文展示密钥。
    pub show_api_key: bool,
    /// 模型目录搜索词。
    pub model_search_query: String,
    /// 模型列表读取中。
    pub models_loading: bool,
    /// 模型列表错误。
    pub models_error: String,
    /// 网关服务地址。
    pub edit_base_url: String,
    /// 高级网关是否展开。
    pub show_advanced_gateway: bool,
    /// 网关是否已配置认证密钥（生产 `hasApiKey`）。
    pub has_api_key: bool,
}

/// `keychain` 里的钥匙串回显（**只存打码串**）。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct KeychainFixture {
    /// 已存密钥的**打码**串。
    pub stored_key_masked: String,
    /// 是否有已存密钥。
    pub stored_key_exists: bool,
    /// 读取中。
    pub loading: bool,
    /// 写入中。
    pub saving: bool,
    /// 删除中。
    pub deleting: bool,
    /// 动作错误。
    pub action_error: String,
    /// 模态里的新密钥草稿。
    pub temp_api_key: String,
}

/// 本面要用的固定数据（**装配后**）。
#[derive(Debug, Clone, PartialEq)]
pub struct ProviderFixture {
    /// 两组预设表。
    pub presets: ProviderPresets,
    /// 组状态初值。
    pub state: FixtureState,
    /// 模型发现列表。
    pub discovered_models: Vec<DiscoveredModel>,
    /// 测连接的**回执素材**（**不是**首屏状态 ⇒ 见模块头的说明）。
    pub test_result: Option<TestResult>,
    /// 热应用的**回执素材**。
    pub apply_result: Option<ApplyResult>,
    /// 生效配置的**回执素材**。
    pub effective_config: Option<EffectiveConfig>,
    /// 钥匙串回显。
    pub keychain: KeychainFixture,
    /// 字段旗标（键 → 状态）。
    pub text_flags: Vec<(String, FieldFlagState)>,
    /// 固定数据里的默认模型 id（**必须等于组件常量** ⇒ 有断言）。
    pub default_model_id: String,
}

impl ProviderFixture {
    /// 读固定数据并装配（**固定数据坏了 ⇒ 显式报错**）。
    pub fn load() -> Result<Self, String> {
        let root = crate::settings_theme::parse_json(PROVIDER_FIXTURE_JSON)
            .map_err(|error| format!("服务商固定数据解析失败：{error}"))?;
        let presets = read_presets(&root)?;
        let state = read_state(&root)?;
        let discovered_models = read_discovered_models(&root)?;
        let test_result = read_test_result(&root)?;
        let apply_result = read_apply_result(&root)?;
        let effective_config = read_effective_config(&root)?;
        let keychain = read_keychain(&root)?;
        let text_flags = read_text_flags(&root)?;
        let default_model_id = read_string(&root, "defaultModelId")?;
        Ok(Self {
            presets,
            state,
            discovered_models,
            test_result,
            apply_result,
            effective_config,
            keychain,
            text_flags,
            default_model_id,
        })
    }
}

/// 读一个字符串字段（**缺字段 ⇒ 显式报错**）。
fn read_string(value: &Json, key: &str) -> Result<String, String> {
    value
        .get(key)
        .and_then(|item| item.as_str())
        .map(str::to_owned)
        .ok_or_else(|| format!("固定数据缺字段：{key}"))
}

/// 读一个布尔字段（**缺字段 ⇒ 显式报错**）。
fn read_bool(value: &Json, key: &str) -> Result<bool, String> {
    value
        .get(key)
        .and_then(|item| item.as_bool())
        .ok_or_else(|| format!("固定数据缺字段：{key}"))
}

/// 读一个嵌套对象的字段（**缺 ⇒ 走 `default`**：固定数据的可选块）。
fn read_opt_string(value: &Json, object: &str, key: &str) -> String {
    value
        .get(object)
        .and_then(|item| item.get(key))
        .and_then(|item| item.as_str())
        .unwrap_or("")
        .to_owned()
}

/// 读一条预设。
fn read_preset(value: &Json) -> Result<ProviderPreset, String> {
    let models = value
        .get("models")
        .and_then(|item| item.as_array())
        .ok_or_else(|| "固定数据缺字段：presets[].models".to_owned())?
        .iter()
        .map(|item| {
            item.as_str()
                .map(str::to_owned)
                .ok_or_else(|| "固定数据里的 presets[].models 必须是字符串数组".to_owned())
        })
        .collect::<Result<Vec<String>, String>>()?;
    Ok(ProviderPreset {
        id: read_string(value, "id")?,
        name: read_string(value, "name")?,
        base_url: read_string(value, "baseUrl")?,
        default_model: read_string(value, "defaultModel")?,
        models,
    })
}

/// 读一组预设。
fn read_preset_group(value: &Json, key: &str) -> Result<Vec<ProviderPreset>, String> {
    let items = value
        .get("presets")
        .and_then(|item| item.get(key))
        .and_then(|item| item.as_array())
        .ok_or_else(|| format!("固定数据缺字段：presets.{key}"))?;
    items.iter().map(read_preset).collect()
}

/// 读两组预设表。
fn read_presets(value: &Json) -> Result<ProviderPresets, String> {
    Ok(ProviderPresets {
        openai: read_preset_group(value, "openai")?,
        anthropic: read_preset_group(value, "anthropic")?,
    })
}

/// 读组状态。
fn read_state(value: &Json) -> Result<FixtureState, String> {
    let state = value
        .get("state")
        .ok_or_else(|| "固定数据缺字段：state".to_owned())?;
    Ok(FixtureState {
        protocol: read_string(state, "activeProtocol")?,
        preset: read_string(state, "activePreset")?,
        base_url: read_string(state, "providerBaseUrl")?,
        api_key: read_string(state, "providerApiKey")?,
        model: read_string(state, "providerModel")?,
        anthropic_version: read_string(state, "anthropicVersion")?,
        show_api_key: read_bool(state, "showApiKey")?,
        model_search_query: read_string(state, "modelSearchQuery")?,
        models_loading: read_bool(state, "modelsLoading")?,
        models_error: read_string(state, "modelsError")?,
        edit_base_url: read_string(state, "editBaseUrl")?,
        show_advanced_gateway: read_bool(state, "showAdvancedGateway")?,
        has_api_key: read_bool(state, "hasApiKey")?,
    })
}

/// 读模型发现列表。
fn read_discovered_models(value: &Json) -> Result<Vec<DiscoveredModel>, String> {
    let items = value
        .get("discoveredModels")
        .and_then(|item| item.as_array())
        .ok_or_else(|| "固定数据缺字段：discoveredModels".to_owned())?;
    let mut models = Vec::new();
    for item in items {
        models.push(DiscoveredModel {
            id: read_string(item, "id")?,
            owned_by: read_string(item, "ownedBy")?,
        });
    }
    Ok(models)
}

/// 读测连接的回执素材（`null` ⇒ `None`）。
fn read_test_result(value: &Json) -> Result<Option<TestResult>, String> {
    let Some(result) = value.get("testResult") else {
        return Ok(None);
    };
    if matches!(result, Json::Null) {
        return Ok(None);
    }
    let models = result
        .get("models")
        .and_then(|item| item.as_array())
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str().map(str::to_owned))
                .collect::<Vec<String>>()
        })
        .unwrap_or_default();
    let latency_ms = match result.get("latencyMs") {
        Some(Json::Number(number)) => Some(*number),
        _ => None,
    };
    Ok(Some(TestResult {
        ok: read_bool(result, "ok")?,
        message: read_string(result, "message")?,
        latency_ms,
        models,
    }))
}

/// 读热应用回执素材。
fn read_apply_result(value: &Json) -> Result<Option<ApplyResult>, String> {
    let Some(result) = value.get("applyResult") else {
        return Ok(None);
    };
    if matches!(result, Json::Null) {
        return Ok(None);
    }
    let warnings = result
        .get("warnings")
        .and_then(|item| item.as_array())
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str().map(str::to_owned))
                .collect::<Vec<String>>()
        })
        .unwrap_or_default();
    Ok(Some(ApplyResult {
        ok: read_bool(result, "ok")?,
        warnings,
    }))
}

/// 读生效配置回执素材（**字段可缺** ⇒ 缺即 `None` ⇒ 组件展示 `—`）。
fn read_effective_config(value: &Json) -> Result<Option<EffectiveConfig>, String> {
    let Some(config) = value.get("effectiveConfig") else {
        return Ok(None);
    };
    if matches!(config, Json::Null) {
        return Ok(None);
    }
    let field = |key: &str| {
        config
            .get(key)
            .and_then(|item| item.as_str())
            .map(str::to_owned)
    };
    Ok(Some(EffectiveConfig {
        provider: field("provider"),
        base_url: field("base_url"),
        api_key: field("api_key"),
        model: field("model"),
    }))
}

/// 读钥匙串回显。
fn read_keychain(value: &Json) -> Result<KeychainFixture, String> {
    Ok(KeychainFixture {
        stored_key_masked: read_opt_string(value, "keychain", "storedKeyMasked"),
        stored_key_exists: value
            .get("keychain")
            .and_then(|item| item.get("storedKeyExists"))
            .and_then(|item| item.as_bool())
            .unwrap_or(false),
        loading: value
            .get("keychain")
            .and_then(|item| item.get("keychainLoading"))
            .and_then(|item| item.as_bool())
            .unwrap_or(false),
        saving: value
            .get("keychain")
            .and_then(|item| item.get("keychainSaving"))
            .and_then(|item| item.as_bool())
            .unwrap_or(false),
        deleting: value
            .get("keychain")
            .and_then(|item| item.get("keychainDeleting"))
            .and_then(|item| item.as_bool())
            .unwrap_or(false),
        action_error: read_opt_string(value, "keychain", "keychainActionError"),
        temp_api_key: read_opt_string(value, "keychain", "tempApiKey"),
    })
}

/// 读字段旗标（`true` ⇒ 未保存；`_` 开头的键是注释）。
fn read_text_flags(value: &Json) -> Result<Vec<(String, FieldFlagState)>, String> {
    let entries = match value.get("textFlags") {
        Some(Json::Object(entries)) => entries.clone(),
        _ => return Ok(Vec::new()),
    };
    let mut flags = Vec::new();
    for (key, item) in entries {
        if key.starts_with('_') {
            continue;
        }
        let dirty = item.as_bool().unwrap_or(false);
        flags.push((
            key,
            if dirty {
                FieldFlagState::Dirty
            } else {
                FieldFlagState::None
            },
        ));
    }
    Ok(flags)
}

// ── 宿主状态 ────────────────────────────────────────────────────────────────

/// 字段值的**回读钩子**（**DOM 访问只在 wasm 层** ⇒ 宿主保持可 native 测试）。
///
/// 入参 = 字段键（`provider.baseUrl` …）；返回值 = DOM 里该输入框的**当前值**。
pub type FieldReader = Rc<dyn Fn(&str) -> Option<String>>;

/// 宿主状态（**状态归宿主** ⇒ 组件是纯函数）。
#[derive(Clone)]
pub struct ProviderHost {
    /// 运行时（信号宿主）。
    runtime: Runtime,
    /// 当前分区（**只用于共享 API 的语义**；渲染时两个区域同时存在）。
    section: Signal<String>,
    /// 协议。
    protocol: Signal<String>,
    /// 预设 id。
    preset: Signal<String>,
    /// 端点。
    base_url: Signal<String>,
    /// 密钥（**永远不进发布面**）。
    api_key: Signal<String>,
    /// 活动模型。
    model: Signal<String>,
    /// 协议头。
    anthropic_version: Signal<String>,
    /// 明文切换。
    show_api_key: Signal<bool>,
    /// 模型发现列表。
    discovered_models: Signal<Vec<DiscoveredModel>>,
    /// 模型列表读取中。
    models_loading: Signal<bool>,
    /// 模型列表错误。
    models_error: Signal<String>,
    /// 模型目录搜索词。
    model_search_query: Signal<String>,
    /// 测连接回执。
    test_result: Signal<Option<TestResult>>,
    /// 测连接进行中。
    testing: Signal<bool>,
    /// 热应用回执。
    apply_result: Signal<Option<ApplyResult>>,
    /// 生效配置。
    effective_config: Signal<Option<EffectiveConfig>>,
    /// 网关服务地址草稿。
    edit_base_url: Signal<String>,
    /// 高级网关展开。
    show_advanced_gateway: Signal<bool>,
    /// 网关是否已配置认证密钥。
    has_api_key: Signal<bool>,
    /// 钥匙串打码串。
    stored_key_masked: Signal<String>,
    /// 钥匙串是否有已存密钥。
    stored_key_exists: Signal<bool>,
    /// 钥匙串读取中。
    keychain_loading: Signal<bool>,
    /// 钥匙串写入中。
    keychain_saving: Signal<bool>,
    /// 钥匙串删除中。
    keychain_deleting: Signal<bool>,
    /// 钥匙串动作错误。
    keychain_action_error: Signal<String>,
    /// 模态新密钥草稿。
    temp_api_key: Signal<String>,
    /// 模态是否打开。
    show_api_key_modal: Signal<bool>,
    /// 字段旗标。
    text_flags: Signal<Vec<(String, FieldFlagState)>>,
    /// 最近一次意图。
    intent: Signal<String>,
    /// **整组提交**的次数（T5：一次提交 ⇒ +1）。
    submit_count: Signal<u32>,
    /// 最近一次整组提交的**打码快照**（`apiKey` 已打码 ⇒ 不泄明文）。
    last_submit: Signal<String>,
    /// 字段回读钩子（wasm 层注入）。
    field_reader: Rc<RefCell<Option<FieldReader>>>,
    /// 协议双缓冲（生产 `openaiBuffer` / `anthropicBuffer`）。
    buffers: Rc<RefCell<ProtocolBuffers>>,
    /// 预设表（**不可变**）。
    presets: ProviderPresets,
    /// 固定数据（**不可变** ⇒ 回执素材在这儿）。
    fixture: ProviderFixture,
}

impl ProviderHost {
    /// 建一份宿主状态（**固定数据坏了 ⇒ 显式报错**）。
    pub fn new(fixture: ProviderFixture) -> Self {
        let runtime = Runtime::new();
        let state = fixture.state.clone();
        let keychain = fixture.keychain.clone();
        let section = runtime.signal(String::from(SECTION_MODELS));
        let protocol = runtime.signal(state.protocol.clone());
        let preset = runtime.signal(state.preset.clone());
        let base_url = runtime.signal(state.base_url.clone());
        let api_key = runtime.signal(state.api_key.clone());
        let model = runtime.signal(state.model.clone());
        let anthropic_version = runtime.signal(state.anthropic_version.clone());
        let show_api_key = runtime.signal(state.show_api_key);
        let discovered_models = runtime.signal(fixture.discovered_models.clone());
        let models_loading = runtime.signal(state.models_loading);
        let models_error = runtime.signal(state.models_error.clone());
        let model_search_query = runtime.signal(state.model_search_query.clone());
        // ⚠️ **回执类信号初值 `None`**（参考侧是生产初始态 ⇒ 见模块头）。
        let test_result = runtime.signal(None);
        let testing = runtime.signal(false);
        let apply_result = runtime.signal(None);
        let effective_config = runtime.signal(None);
        let edit_base_url = runtime.signal(state.edit_base_url.clone());
        let show_advanced_gateway = runtime.signal(state.show_advanced_gateway);
        let has_api_key = runtime.signal(state.has_api_key);
        let stored_key_masked = runtime.signal(keychain.stored_key_masked.clone());
        let stored_key_exists = runtime.signal(keychain.stored_key_exists);
        let keychain_loading = runtime.signal(keychain.loading);
        let keychain_saving = runtime.signal(keychain.saving);
        let keychain_deleting = runtime.signal(keychain.deleting);
        let keychain_action_error = runtime.signal(keychain.action_error.clone());
        let temp_api_key = runtime.signal(keychain.temp_api_key.clone());
        let show_api_key_modal = runtime.signal(false);
        let text_flags = runtime.signal(fixture.text_flags.clone());
        let intent = runtime.signal(String::new());
        let submit_count = runtime.signal(0u32);
        let last_submit = runtime.signal(String::new());
        let presets = fixture.presets.clone();
        Self {
            runtime,
            section,
            protocol,
            preset,
            base_url,
            api_key,
            model,
            anthropic_version,
            show_api_key,
            discovered_models,
            models_loading,
            models_error,
            model_search_query,
            test_result,
            testing,
            apply_result,
            effective_config,
            edit_base_url,
            show_advanced_gateway,
            has_api_key,
            stored_key_masked,
            stored_key_exists,
            keychain_loading,
            keychain_saving,
            keychain_deleting,
            keychain_action_error,
            temp_api_key,
            show_api_key_modal,
            text_flags,
            intent,
            submit_count,
            last_submit,
            field_reader: Rc::new(RefCell::new(None)),
            buffers: Rc::new(RefCell::new(initial_buffers(&presets))),
            presets,
            fixture,
        }
    }

    /// 运行时。
    pub fn runtime(&self) -> &Runtime {
        &self.runtime
    }

    /// 当前分区。
    pub fn section(&self) -> String {
        self.section.get()
    }

    /// 切到某个分区（**未知分区 ⇒ 不切** ⇒ 不假装已接线）。
    ///
    /// ⚠️ 渲染上两个区域**同时存在**（见组件的 `render` 文档）⇒ 本方法只影响
    /// 共享 API 的 `section()` 语义与裁剪选择器的选择。
    pub fn select_section(&self, id: &str) -> bool {
        if !matches!(id, SECTION_MODELS | SECTION_API_KEY_MODAL) {
            return false;
        }
        self.section.set(String::from(id));
        true
    }

    /// 装字段回读钩子（**wasm 层注入**；native 测试不装 ⇒ 只推进旗标）。
    pub fn set_field_reader(&self, reader: FieldReader) {
        *self.field_reader.borrow_mut() = Some(reader);
    }

    /// 记录一条意图（**意图出口 ⇒ 宿主留痕**）。
    pub fn record_intent(&self, intent: &str) {
        self.intent.set(String::from(intent));
    }

    /// 最近一次意图。
    pub fn last_intent(&self) -> String {
        self.intent.get()
    }

    /// 整组提交次数。
    pub fn submit_count(&self) -> u32 {
        self.submit_count.get()
    }

    /// 最近一次整组提交的打码快照。
    pub fn last_submit(&self) -> String {
        self.last_submit.get()
    }

    /// 对照装置入口：**喂一串原始密钥** ⇒ 本侧用自己的 [`mask_api_key`] 打码后回显。
    ///
    /// ## 为什么需要
    ///
    /// 生产 `openApiKeyModal()` 的显示串是 `maskApiKey(getProviderKey())` 算出来的
    /// ⇒ 参考侧的显示串**就是生产打码函数的输出**。对照驱动把同一串喂给本方法
    /// ⇒ 本侧显示的是**引擎打码函数的输出** ⇒ 两个串逐字相等 ⇒ 判据 T6 有跨侧证据。
    ///
    /// ⚠️ 这不是"假接线"：真实链路里宿主拿到的是**已打码串**（钥匙串只回显打码值）
    /// ⇒ 本方法**只用于对照装置**（`window.__PROVIDER__.applyStoredKey`）。
    pub fn apply_stored_key(&self, raw: &str) {
        self.stored_key_exists.set(true);
        self.stored_key_masked.set(mask_api_key(raw));
    }

    /// 取一个字段的当前宿主值（**不经过发布面** ⇒ 密钥不回显到全局）。
    ///
    /// 键集与组件侧一致：`provider.baseUrl` / `provider.model` /
    /// `provider.apiKey` / `provider.anthropicVersion` / `gateway.baseUrl` /
    /// `model.searchQuery` / `modal.tempApiKey`；未知键 ⇒ `None`。
    pub fn field_value(&self, key: &str) -> Option<String> {
        match key {
            "provider.baseUrl" => Some(self.base_url.get()),
            "provider.model" => Some(self.model.get()),
            "provider.apiKey" => Some(self.api_key.get()),
            "provider.anthropicVersion" => Some(self.anthropic_version.get()),
            "gateway.baseUrl" => Some(self.edit_base_url.get()),
            "model.searchQuery" => Some(self.model_search_query.get()),
            "modal.tempApiKey" => Some(self.temp_api_key.get()),
            _ => None,
        }
    }

    /// 建一份组件 props（**状态归宿主** ⇒ 组件是纯函数）。
    pub fn props(&self) -> ProviderGroupProps {
        let action = self.clone();
        let field = self.clone();
        let dirty = self.clone();
        let submit = self.clone();
        ProviderGroupProps {
            protocol: self.protocol.clone(),
            preset: self.preset.clone(),
            base_url: self.base_url.clone(),
            api_key: self.api_key.clone(),
            model: self.model.clone(),
            anthropic_version: self.anthropic_version.clone(),
            show_api_key: self.show_api_key.clone(),
            discovered_models: self.discovered_models.clone(),
            models_loading: self.models_loading.clone(),
            models_error: self.models_error.clone(),
            model_search_query: self.model_search_query.clone(),
            test_result: self.test_result.clone(),
            testing: self.testing.clone(),
            apply_result: self.apply_result.clone(),
            effective_config: self.effective_config.clone(),
            edit_base_url: self.edit_base_url.clone(),
            show_advanced_gateway: self.show_advanced_gateway.clone(),
            has_api_key: self.has_api_key.get(),
            stored_key_masked: self.stored_key_masked.clone(),
            stored_key_exists: self.stored_key_exists.clone(),
            keychain_loading: self.keychain_loading.clone(),
            keychain_saving: self.keychain_saving.clone(),
            keychain_deleting: self.keychain_deleting.clone(),
            keychain_action_error: self.keychain_action_error.clone(),
            temp_api_key: self.temp_api_key.clone(),
            show_api_key_modal: self.show_api_key_modal.clone(),
            text_flags: self.text_flags.clone(),
            presets: self.presets.clone(),
            on_action: Rc::new(move |name: &str| {
                action.record_intent(name);
                action.handle_action(name);
            }),
            on_field_input: Rc::new(move |keys: &str| {
                field.record_intent(&format!("field:{keys}"));
                field.mark_dirty(keys);
            }),
            on_mark_dirty: Rc::new(move |keys: &str| {
                dirty.record_intent(&format!("dirty:{keys}"));
                dirty.mark_flags_only(keys);
            }),
            on_submit_group: Rc::new(move |group: &str, draft: &ProviderGroupDraft| {
                submit.submit_group(group, draft);
            }),
        }
    }

    // ── 状态转移（**宿主侧**；对齐生产脚本的行为） ──────────────────────────

    /// 处理一条「动作」意图（**HTTP / 钥匙串一律是固定回执 ⇒ 见模块头**）。
    fn handle_action(&self, action: &str) {
        if let Some(protocol) = action.strip_prefix("protocol:") {
            self.switch_protocol(protocol);
            return;
        }
        if let Some(preset) = action.strip_prefix("preset:") {
            self.select_preset(preset);
            return;
        }
        if let Some(model) = action.strip_prefix("model:") {
            self.model.set(String::from(model));
            return;
        }
        match action {
            "toggle-show-key" => {
                self.show_api_key.set(!self.show_api_key.get());
            }
            "toggle-advanced" => {
                self.show_advanced_gateway
                    .set(!self.show_advanced_gateway.get());
            }
            "reload-models" => {
                // ⚠️ 生产是 HTTP（`listModels`）⇒ 这里是**固定数据回执**（未接线）。
                self.models_error.set(String::new());
                self.discovered_models
                    .set(self.fixture.discovered_models.clone());
            }
            "test-connection" => {
                // ⚠️ 生产是 HTTP（`testProviderConnection`）⇒ 固定回执。
                self.testing.set(true);
                self.test_result.set(self.fixture.test_result.clone());
                self.testing.set(false);
            }
            "open-api-key-modal" => {
                // 生产 `openApiKeyModal()`：清空草稿与错误 ⇒ 读钥匙串 ⇒ 回显。
                self.temp_api_key.set(String::new());
                self.keychain_action_error.set(String::new());
                self.stored_key_exists.set(false);
                self.stored_key_masked.set(String::new());
                self.keychain_loading.set(true);
                self.show_api_key_modal.set(true);
                // ⚠️ 生产是钥匙串读（宿主能力）⇒ 固定回显（**只给打码串**）。
                self.stored_key_exists
                    .set(self.fixture.keychain.stored_key_exists);
                self.stored_key_masked
                    .set(self.fixture.keychain.stored_key_masked.clone());
                self.keychain_loading.set(false);
            }
            "close-api-key-modal" => {
                self.show_api_key_modal.set(false);
            }
            "save-new-api-key" => {
                let key = self.temp_api_key.get().trim().to_owned();
                if key.is_empty() {
                    self.keychain_action_error
                        .set(String::from(EMPTY_KEY_ERROR));
                    return;
                }
                // ⚠️ 生产写系统钥匙串 + 热应用（宿主能力 / HTTP）⇒ 这里只推进回显。
                self.keychain_saving.set(true);
                self.keychain_action_error.set(String::new());
                self.api_key.set(key.clone());
                self.stored_key_exists.set(true);
                self.stored_key_masked.set(mask_api_key(&key));
                self.temp_api_key.set(String::new());
                self.show_api_key_modal.set(false);
                self.keychain_saving.set(false);
            }
            "delete-stored-key" => {
                // ⚠️ 生产先弹二次确认（危险动作）⇒ 引擎只发意图；这里直接推进回显。
                self.keychain_deleting.set(true);
                self.keychain_action_error.set(String::new());
                self.stored_key_exists.set(false);
                self.stored_key_masked.set(String::new());
                // 生产：删完把组内密钥清空（避免测试连接带旧 key）。
                self.api_key.set(String::new());
                self.keychain_deleting.set(false);
            }
            // `modal-stop`：对话框自己吞事件（**不改状态**）。
            _ => {}
        }
    }

    /// 标「未保存」并从 DOM **回读**字段值（生产 `bind:value` + `markTextDirty`）。
    fn mark_dirty(&self, keys: &str) {
        for key in keys.split(',') {
            let key = key.trim();
            if key.is_empty() {
                continue;
            }
            if let Some(value) = self.read_field(key) {
                self.set_field(key, &value);
            }
            self.set_flag(key, FieldFlagState::Dirty);
        }
    }

    /// **只**标「未保存」（点选类改动 ⇒ 不回读 DOM）。
    fn mark_flags_only(&self, keys: &str) {
        self.set_flags(keys, FieldFlagState::Dirty);
    }

    /// 从回读钩子取一个字段的当前值。
    fn read_field(&self, key: &str) -> Option<String> {
        let reader = self.field_reader.borrow();
        let reader = reader.as_ref()?;
        (reader)(key)
    }

    /// 把一个字段值写进对应信号。
    fn set_field(&self, key: &str, value: &str) {
        match key {
            "provider.baseUrl" => self.base_url.set(String::from(value)),
            "provider.model" => self.model.set(String::from(value)),
            "provider.apiKey" => self.api_key.set(String::from(value)),
            "provider.anthropicVersion" => self.anthropic_version.set(String::from(value)),
            "gateway.baseUrl" => self.edit_base_url.set(String::from(value)),
            "model.searchQuery" => self.model_search_query.set(String::from(value)),
            "modal.tempApiKey" => self.temp_api_key.set(String::from(value)),
            _ => {}
        }
    }

    /// 设一个字段旗标。
    fn set_flag(&self, key: &str, state: FieldFlagState) {
        let mut flags = self.text_flags.get();
        match flags.iter_mut().find(|(name, _)| name == key) {
            Some(entry) => entry.1 = state,
            None => flags.push((String::from(key), state)),
        }
        self.text_flags.set(flags);
    }

    /// 批量设旗标。
    fn set_flags(&self, keys: &str, state: FieldFlagState) {
        for key in keys.split(',') {
            let key = key.trim();
            if !key.is_empty() {
                self.set_flag(key, state);
            }
        }
    }

    /// **整组提交**（生产 `submitProviderGroup` 的宿主落点）。
    ///
    /// ## 口径
    ///
    /// - `provider-group` ⇒ 记录**一次**提交 + 把整组草稿落进状态信号 +
    ///   灌入**固定回执**（`applyResult` / `effectiveConfig`）+ 闪 ✓ 旗标；
    /// - `gateway-url` ⇒ 只记录（网关地址的落盘是宿主的事）。
    ///
    /// ⚠️ **引擎不执行**：组件只发意图；本方法只推进宿主状态（**HTTP 未接线**）。
    fn submit_group(&self, group: &str, draft: &ProviderGroupDraft) {
        self.record_intent(&format!("submit:{group}"));
        if group != "provider-group" {
            return;
        }
        // 整组落位（生产 `providerGroupDraft()` 的六个字段）。
        self.protocol.set(draft.protocol.clone());
        self.preset.set(draft.preset.clone());
        self.base_url.set(draft.base_url.clone());
        self.api_key.set(draft.api_key.clone());
        self.model.set(draft.model.clone());
        self.anthropic_version.set(draft.anthropic_version.clone());
        // 提交快照（**打码后发布** ⇒ 不泄明文）。
        self.submit_count.set(self.submit_count.get() + 1);
        self.last_submit.set(masked_snapshot(draft));
        // 固定回执（**HTTP 未接线** ⇒ 只推进画面）。
        self.apply_result.set(self.fixture.apply_result.clone());
        self.effective_config
            .set(self.fixture.effective_config.clone());
        // 生产 `ackTextKeys(PROVIDER_TEXT_KEYS)`：闪 ✓（1.6s 后收起）。
        self.set_flags(PROVIDER_TEXT_KEYS, FieldFlagState::Ack);
        self.schedule_ack_clear();
    }

    /// 闪 ✓ 之后 **1.6s** 收起（生产用 `setTimeout`；wasm 层用浏览器计时器）。
    fn schedule_ack_clear(&self) {
        #[cfg(target_arch = "wasm32")]
        {
            let host = self.clone();
            schedule_timeout(ACK_HOLD_MS, move || {
                host.set_flags(PROVIDER_TEXT_KEYS, FieldFlagState::None);
            });
        }
        // native：没有计时器（测试断言的是「提交后立刻是 ✓」那一态）。
    }

    // ── 生产脚本的纯状态转移（**逐字对齐**） ────────────────────────────────

    /// 协议切换（生产 `switchProtocol`：**双缓冲**）。
    ///
    /// ## 口径（逐字照抄生产的两个 buffer）
    ///
    /// - 先把当前值存进**当前协议**的 buffer；
    /// - 再把**目标协议**的 buffer 恢复成当前值；
    /// - `anthropicVersion` 只在 Anthropic 侧存在（空 ⇒ 回 `2023-06-01`）；
    /// - 切协议**清掉测连接回执**（`testResult = null`）。
    fn switch_protocol(&self, protocol: &str) {
        let current = self.protocol.get();
        if current == protocol {
            return;
        }
        let draft = BufferDraft {
            preset: self.preset.get(),
            base_url: self.base_url.get(),
            api_key: self.api_key.get(),
            model: self.model.get(),
            anthropic_version: None,
        };
        let mut buffers = self.buffers.borrow_mut();
        if current == PROTOCOL_ANTHROPIC {
            buffers.anthropic = Some(BufferDraft {
                anthropic_version: Some(self.anthropic_version.get()),
                ..draft
            });
        } else {
            buffers.openai = Some(draft);
        }
        self.protocol.set(String::from(protocol));
        self.test_result.set(None);
        // ⚠️ 目标协议**没有**缓冲时 ⇒ **保持当前值**（生产的 buffer 恒有初值
        // ⇒ 这条只是防御；**绝不清空状态**）。
        let mut restored = if protocol == PROTOCOL_ANTHROPIC {
            buffers.anthropic.clone()
        } else {
            buffers.openai.clone()
        }
        .unwrap_or_else(|| BufferDraft {
            preset: self.preset.get(),
            base_url: self.base_url.get(),
            api_key: self.api_key.get(),
            model: self.model.get(),
            anthropic_version: Some(self.anthropic_version.get()),
        });
        self.preset.set(std::mem::take(&mut restored.preset));
        self.base_url.set(std::mem::take(&mut restored.base_url));
        self.api_key.set(std::mem::take(&mut restored.api_key));
        self.model.set(std::mem::take(&mut restored.model));
        if protocol == PROTOCOL_ANTHROPIC {
            let version = restored
                .anthropic_version
                .filter(|value| !value.is_empty())
                .unwrap_or_else(|| String::from("2023-06-01"));
            self.anthropic_version.set(version);
        }
    }

    /// 选预设（生产 `selectPreset`：设 id + 清测连接回执 + 端点与模型联动）。
    fn select_preset(&self, preset_id: &str) {
        self.preset.set(String::from(preset_id));
        self.test_result.set(None);
        let protocol = self.protocol.get();
        let Some(preset) = self.presets.find(&protocol, preset_id) else {
            return;
        };
        if preset.id == "custom" {
            return;
        }
        self.base_url.set(preset.base_url.clone());
        let current = self.model.get();
        if let Some(model) =
            apeireth_ui_kit::settings_provider::preset_model_after_select(preset, &current)
        {
            self.model.set(model);
        }
    }

    /// 当前状态的快照（**对照脚本可读**；**不含明文密钥**）。
    pub fn state(&self) -> BTreeMap<String, String> {
        let mut state = BTreeMap::new();
        state.insert(String::from("section"), self.section.get());
        state.insert(String::from("protocol"), self.protocol.get());
        state.insert(String::from("preset"), self.preset.get());
        state.insert(String::from("baseUrl"), self.base_url.get());
        state.insert(String::from("model"), self.model.get());
        state.insert(
            String::from("anthropicVersion"),
            self.anthropic_version.get(),
        );
        state.insert(
            String::from("showApiKey"),
            bool_text(self.show_api_key.get()),
        );
        // ⚠️ **密钥不进发布面**：只给「有没有」与打码串。
        state.insert(
            String::from("hasGroupApiKey"),
            bool_text(!self.api_key.get().trim().is_empty()),
        );
        state.insert(
            String::from("apiKeyMasked"),
            mask_api_key(&self.api_key.get()),
        );
        state.insert(
            String::from("testResult"),
            match self.test_result.get() {
                Some(result) => {
                    let latency = match result.latency_ms {
                        Some(value) => apeireth_ui_kit::num::number_to_string(value),
                        None => String::from(""),
                    };
                    format!(
                        "ok={};latencyMs={latency};models={}",
                        bool_text(result.ok),
                        result.models.len()
                    )
                }
                None => String::from("null"),
            },
        );
        state.insert(
            String::from("applyResult"),
            match self.apply_result.get() {
                Some(result) => format!(
                    "ok={};warnings={}",
                    bool_text(result.ok),
                    result.warnings.len()
                ),
                None => String::from("null"),
            },
        );
        state.insert(
            String::from("effectiveConfig"),
            match self.effective_config.get() {
                Some(config) => format!(
                    "provider={};baseUrl={};model={}",
                    config.provider.unwrap_or_default(),
                    config.base_url.unwrap_or_default(),
                    config.model.unwrap_or_default()
                ),
                None => String::from("null"),
            },
        );
        state.insert(
            String::from("showApiKeyModal"),
            bool_text(self.show_api_key_modal.get()),
        );
        state.insert(
            String::from("showAdvancedGateway"),
            bool_text(self.show_advanced_gateway.get()),
        );
        state.insert(
            String::from("keychain"),
            format!(
                "loading={};exists={};masked={};saving={};deleting={}",
                bool_text(self.keychain_loading.get()),
                bool_text(self.stored_key_exists.get()),
                self.stored_key_masked.get(),
                bool_text(self.keychain_saving.get()),
                bool_text(self.keychain_deleting.get())
            ),
        );
        state.insert(
            String::from("submitCount"),
            self.submit_count.get().to_string(),
        );
        state.insert(String::from("lastSubmit"), self.last_submit.get());
        state.insert(String::from("intent"), self.intent.get());
        for (key, flag) in self.text_flags.get() {
            let text = match flag {
                FieldFlagState::None => "none",
                FieldFlagState::Dirty => "dirty",
                FieldFlagState::Ack => "ack",
            };
            state.insert(format!("flag:{key}"), String::from(text));
        }
        state
    }
}

/// 一个协议的缓冲（生产 `openaiBuffer` / `anthropicBuffer`）。
#[derive(Debug, Clone, Default)]
struct BufferDraft {
    preset: String,
    base_url: String,
    api_key: String,
    model: String,
    anthropic_version: Option<String>,
}

/// 双缓冲（**生产的两份 `$state` 对象** ⇒ 宿主持有）。
#[derive(Debug, Clone, Default)]
struct ProtocolBuffers {
    openai: Option<BufferDraft>,
    anthropic: Option<BufferDraft>,
}

/// 双缓冲的**初值**（生产的两份字面量）。
///
/// ## 口径（逐字照抄生产）
///
/// ```text
/// openaiBuffer    = {preset: 'openai',    baseUrl: 'https://api.openai.com/v1',     apiKey: '', model: 'gpt-4o'}
/// anthropicBuffer = {preset: 'anthropic', baseUrl: 'https://api.anthropic.com',     apiKey: '', model: 'claude-3-7-sonnet-20250219',
///                    anthropicVersion: '2023-06-01'}
/// ```
///
/// ⚠️ 它们**不是空对象**：首次切到 Anthropic 协议时生产会**恢复**这份初值
/// （端点 / 模型 / 协议头一起换）⇒ 若宿主把它当空缓冲 ⇒ 切协议后端点与模型
/// 变成空串 ⇒ 与参考侧立刻不一致（实测：切协议后 27,229 px 差异 + DOM 不等）。
fn initial_buffers(presets: &ProviderPresets) -> ProtocolBuffers {
    ProtocolBuffers {
        openai: presets
            .for_protocol(PROTOCOL_OPENAI)
            .first()
            .map(|preset| BufferDraft {
                preset: preset.id.clone(),
                base_url: preset.base_url.clone(),
                api_key: String::new(),
                model: preset.default_model.clone(),
                anthropic_version: None,
            }),
        anthropic: presets
            .for_protocol(PROTOCOL_ANTHROPIC)
            .first()
            .map(|preset| BufferDraft {
                preset: preset.id.clone(),
                base_url: preset.base_url.clone(),
                api_key: String::new(),
                model: preset.default_model.clone(),
                anthropic_version: Some(String::from("2023-06-01")),
            }),
    }
}

/// 布尔 ⇒ 发布面文本。
fn bool_text(value: bool) -> String {
    if value {
        String::from("true")
    } else {
        String::from("false")
    }
}

/// 整组草稿 ⇒ **打码后**的一行快照（T5 用；**不泄明文**）。
fn masked_snapshot(draft: &ProviderGroupDraft) -> String {
    format!(
        "protocol={};preset={};baseUrl={};apiKey={};model={};anthropicVersion={}",
        draft.protocol,
        draft.preset,
        draft.base_url,
        mask_api_key(&draft.api_key),
        draft.model,
        draft.anthropic_version
    )
}

/// 整组提交意图里**必须都在**的六个字段（T5 的字段清单）。
pub const SUBMIT_FIELDS: [&str; 6] = [
    "protocol",
    "preset",
    "baseUrl",
    "apiKey",
    "model",
    "anthropicVersion",
];

// ── 浏览器的计时器（**零新增 web-sys feature** ⇒ 走全局函数反射） ───────────

/// 用页面的 `setTimeout` 排一个回调（**没有 `setTimeout` ⇒ 立刻执行**）。
///
/// ## 为什么不直接用 `web-sys` 的 `Window::set_timeout_*`
///
/// 那需要额外的 `web-sys` feature（本 crate 的 feature 面**只开了四个**，见
/// `Cargo.toml`）⇒ 为了一个 1.6s 的旗标收起去扩 feature 面不划算。
/// ⇒ 用 `Reflect::get(globalThis, "setTimeout")` 取函数（与 character 面取
/// `queueMicrotask` 同一手法）。
#[cfg(target_arch = "wasm32")]
fn schedule_timeout(milliseconds: i32, callback: impl FnOnce() + 'static) {
    use wasm_bindgen::prelude::Closure;
    use wasm_bindgen::JsCast;

    let Some(window) = web_sys::window() else {
        callback();
        return;
    };
    let Ok(timer) = js_sys::Reflect::get(&window, &wasm_bindgen::JsValue::from_str("setTimeout"))
    else {
        callback();
        return;
    };
    let Ok(timer) = timer.dyn_into::<js_sys::Function>() else {
        callback();
        return;
    };
    // `setTimeout(fn, ms)`：闭包是**第一个实参**（`once_into_js` 交出所有权）。
    let closure = Closure::once_into_js(move || {
        callback();
    });
    let arguments = js_sys::Array::new();
    arguments.push(&closure);
    arguments.push(&wasm_bindgen::JsValue::from_f64(f64::from(milliseconds)));
    let _ = timer.apply(&window, &arguments);
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
    //! - **真后端未接线**：数据面是**固定数据**（编译期内嵌的那一份）。
    //! - **原生控件的取值是 property**：生产用 `bind:value` ⇒ 引擎侧**不写**
    //!   `value` 属性 ⇒ 宿主在每次渲染后**回写 property**（否则像素对照会看到
    //!   空输入框）；`input` 事件时**从 property 读回**（生产 binding 的等价物）。

    use std::cell::RefCell;
    use std::rc::Rc;

    use apeireth_ui_core::{HostScheduler, UiComponent};
    use apeireth_ui_dom::{Renderer, VNode, WebDom};
    use apeireth_ui_kit::settings_provider::ProviderGroup;
    use apeireth_ui_macro::view;
    use js_sys::Reflect;
    use wasm_bindgen::prelude::*;
    use web_sys::Element;

    use super::{ProviderFixture, ProviderHost};

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
        /// wasm 侧的单例应用（服务商与连接面形态）。
        static PROVIDER_APP: RefCell<Option<ProviderHost>> = const { RefCell::new(None) };
        /// 控制 API 的闭包（**必须活到页面结束**）。
        static CONTROL_CLOSURES: RefCell<Vec<Closure<dyn FnMut(JsValue) -> JsValue>>> =
            const { RefCell::new(Vec::new()) };
    }

    /// 渲染整棵视图树（**舞台 + 作用域锚点**）。
    ///
    /// ⚠️ 两个区域**同时存在**（`models` 区块 + 条件模态）⇒ 与生产结构同构；
    /// 模态是 `position: fixed` ⇒ 裁剪它对**整视口**取框。
    fn render_tree(host: &ProviderHost) -> VNode {
        let panel = ProviderGroup::render(&host.props());
        view! {
            div(class = "settings-stage") {
                div(class = "sp-provider") {
                    { panel }
                }
            }
        }
    }

    /// 挂载服务商与连接面。
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

        let fixture = ProviderFixture::load()?;
        let host = ProviderHost::new(fixture);
        host.runtime()
            .set_host_scheduler(Rc::new(MicrotaskScheduler));

        // 字段回读钩子（**DOM 访问只在这里** ⇒ 宿主保持可 native 测试）。
        host.set_field_reader(Rc::new(|key: &str| read_field_from_dom(key)));

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

        PROVIDER_APP.with(|slot| {
            *slot.borrow_mut() = Some(host.clone());
        });
        install_control_api();
        patch_settings_api();
        mark_ready();
        publish_state(&host);
        Ok(())
    }

    // ⚠️ **不导出 wasm 函数**（`set_settings_section` 已被向导面占用）。
    //
    // ⇒ wasm 的**导出名是全局的** ⇒ 同一个名字导出两次 ⇒ **链接失败**。
    // ⇒ 本面的分区切换走**共享 API 的包装**：`patch_settings_api()`（幂等）
    //    把 `window.__SETTINGS__.selectSection` 包一层 ⇒ 先按共享契约更新
    //    `__SETTINGS_SECTION__`，再驱动本面状态。

    /// 包装共享 API 的 `selectSection`（**幂等** ⇒ 重复包装无害）。
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
        // 幂等哨兵：已经包过 ⇒ 不再包（否则每次挂载都会叠一层）。
        let flag = Reflect::get(&api, &JsValue::from_str("__providerPatched"))
            .ok()
            .and_then(|value| value.as_bool())
            .unwrap_or(false);
        if flag {
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
            // 先按**共享契约**更新 `__SETTINGS_SECTION__`，再驱动本面状态。
            let _ = original.call1(&JsValue::NULL, &args);
            let id = args.as_string().unwrap_or_default();
            PROVIDER_APP.with(|slot| {
                let borrowed = slot.borrow();
                if let Some(app) = borrowed.as_ref() {
                    app.select_section(&id);
                }
            });
            publish_state_any();
            JsValue::UNDEFINED
        }) as Box<dyn FnMut(JsValue) -> JsValue>);
        let function: &js_sys::Function = wrapped.as_ref().unchecked_ref();
        let _ = Reflect::set(&api, &JsValue::from_str("selectSection"), function.as_ref());
        let _ = Reflect::set(
            &api,
            &JsValue::from_str("__providerPatched"),
            &JsValue::TRUE,
        );
        CONTROL_CLOSURES.with(|slot| slot.borrow_mut().push(wrapped));
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
    fn publish_state(host: &ProviderHost) {
        let Some(window) = web_sys::window() else {
            return;
        };
        let state = host.state();
        let json = serde_free_json(&state);
        let _ = Reflect::set(
            window.as_ref(),
            &JsValue::from_str("__PROVIDER_STATE__"),
            &JsValue::from_str(&json),
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

    /// 装控制 API（`window.__PROVIDER__`）—— 对照脚本用它读状态。
    fn install_control_api() {
        let Some(window) = web_sys::window() else {
            return;
        };

        let get_state = Closure::wrap(Box::new(|_args: JsValue| -> JsValue {
            let state = PROVIDER_APP.with(|slot| {
                slot.borrow()
                    .as_ref()
                    .map(|host| host.state())
                    .unwrap_or_default()
            });
            JsValue::from_str(&serde_free_json(&state))
        }) as Box<dyn FnMut(JsValue) -> JsValue>);

        let select_section = Closure::wrap(Box::new(|args: JsValue| -> JsValue {
            let id = args.as_string().unwrap_or_default();
            PROVIDER_APP.with(|slot| {
                let slot = slot.borrow();
                let Some(host) = slot.as_ref() else {
                    return;
                };
                host.select_section(&id);
            });
            publish_state_any();
            JsValue::UNDEFINED
        }) as Box<dyn FnMut(JsValue) -> JsValue>);

        let apply_stored_key = Closure::wrap(Box::new(|args: JsValue| -> JsValue {
            let raw = args.as_string().unwrap_or_default();
            PROVIDER_APP.with(|slot| {
                let slot = slot.borrow();
                let Some(host) = slot.as_ref() else {
                    return;
                };
                host.apply_stored_key(&raw);
            });
            publish_state_any();
            JsValue::UNDEFINED
        }) as Box<dyn FnMut(JsValue) -> JsValue>);

        let get_last_submit = Closure::wrap(Box::new(|_args: JsValue| -> JsValue {
            let value = PROVIDER_APP.with(|slot| {
                slot.borrow()
                    .as_ref()
                    .map(|host| host.last_submit())
                    .unwrap_or_default()
            });
            JsValue::from_str(&value)
        }) as Box<dyn FnMut(JsValue) -> JsValue>);

        let api = js_sys::Object::new();
        let bind = |key: &str, closure: &Closure<dyn FnMut(JsValue) -> JsValue>| {
            let function: &js_sys::Function = closure.as_ref().unchecked_ref();
            let _ = Reflect::set(api.as_ref(), &JsValue::from_str(key), function.as_ref());
        };
        bind("getState", &get_state);
        bind("selectSection", &select_section);
        bind("applyStoredKey", &apply_stored_key);
        bind("getLastSubmit", &get_last_submit);
        let _ = Reflect::set(
            api.as_ref(),
            &JsValue::from_str("ready"),
            &JsValue::from_bool(true),
        );
        let _ = Reflect::set(
            window.as_ref(),
            &JsValue::from_str("__PROVIDER__"),
            api.as_ref(),
        );

        CONTROL_CLOSURES.with(|slot| {
            let mut slot = slot.borrow_mut();
            slot.push(get_state);
            slot.push(select_section);
            slot.push(apply_stored_key);
            slot.push(get_last_submit);
        });
    }

    /// 发布状态（无参版本 ⇒ 闭包里用）。
    fn publish_state_any() {
        PROVIDER_APP.with(|slot| {
            let borrowed = slot.borrow();
            if let Some(host) = borrowed.as_ref() {
                publish_state(host);
            }
        });
    }
    fn read_dom_value(selector: &str) -> Option<String> {
        let window = web_sys::window()?;
        let document = window.document()?;
        let element = document.query_selector(selector).ok().flatten()?;
        Reflect::get(element.as_ref(), &JsValue::from_str("value"))
            .ok()
            .and_then(|value| value.as_string())
    }

    /// 字段键 ⇒ DOM 选择器（**唯一映射点**）。
    fn field_selector(key: &str) -> Option<&'static str> {
        match key {
            "provider.baseUrl" => Some("#provider-url-input"),
            "provider.model" => Some("#provider-model-input"),
            "provider.apiKey" => Some("#provider-key-input"),
            "provider.anthropicVersion" => Some("#anthropic-ver-input"),
            "gateway.baseUrl" => Some("#endpoint-input"),
            "model.searchQuery" => Some(".disc-search input"),
            "modal.tempApiKey" => Some(".modal-body .form-group input"),
            _ => None,
        }
    }

    /// 按字段键读回当前值（字段回读钩子的实现）。
    fn read_field_from_dom(key: &str) -> Option<String> {
        let selector = field_selector(key)?;
        read_dom_value(selector)
    }
}

#[cfg(test)]
mod tests {
    //! 宿主侧的**行为**断言（固定数据装配 + 状态转移 ⇒ 与生产同口径）。

    use super::*;
    use apeireth_ui_kit::settings_provider::{
        mask_api_key, preset_model_after_select, provider_family, DEFAULT_MODEL_ID,
    };

    #[test]
    fn the_fixture_carries_five_openai_and_three_anthropic_presets() {
        // 生产 `OPENAI_PRESETS` 5 条 / `ANTHROPIC_PRESETS` 3 条（逐字取自源文件）。
        let fixture = ProviderFixture::load().expect("固定数据可读");
        assert_eq!(fixture.presets.openai.len(), 5);
        assert_eq!(fixture.presets.anthropic.len(), 3);
        let ids: Vec<&str> = fixture
            .presets
            .openai
            .iter()
            .map(|item| item.id.as_str())
            .collect();
        assert_eq!(ids, ["openai", "deepseek", "minimax", "ollama", "custom"]);
        let ids: Vec<&str> = fixture
            .presets
            .anthropic
            .iter()
            .map(|item| item.id.as_str())
            .collect();
        assert_eq!(ids, ["anthropic", "minimax_anthropic", "custom"]);
    }

    #[test]
    fn the_fixture_default_model_matches_the_component_constant() {
        // ⚠️ 上一批的教训：共享替身把常量写错 ⇒ 判据报差异。
        // ⇒ 这里断言**固定数据里的默认模型 id 与组件常量逐字相同**。
        let fixture = ProviderFixture::load().expect("固定数据可读");
        assert_eq!(fixture.default_model_id, DEFAULT_MODEL_ID);
        assert_eq!(DEFAULT_MODEL_ID, "deepseek-v4-flash");
    }

    #[test]
    fn the_fixture_preset_table_matches_the_production_literal() {
        // 逐条核对生产常量（`SettingsView.svelte:782-842`）：端点 / 默认模型 / 推荐模型。
        let fixture = ProviderFixture::load().expect("固定数据可读");
        let openai = &fixture.presets.openai;
        assert_eq!(openai[0].base_url, "https://api.openai.com/v1");
        assert_eq!(openai[0].default_model, "gpt-4o");
        assert_eq!(openai[0].models.len(), 4);
        assert_eq!(openai[1].base_url, "https://api.deepseek.com/v1");
        assert_eq!(openai[1].default_model, "deepseek-v4-flash");
        // ⚠️ **本断言曾钉的是固定数据的错值**（小写 `minimax-M3`），而同一批的交付报告
        // 已认定「固定数据是参照物缺陷、生产真值是大写 `MiniMax-M3`」
        // （`SettingsView.svelte:801-802`）⇒ **测试与报告自相矛盾**。
        // ⇒ 主代理按生产真值修正本断言（固定数据同步改为大写）⇒ 现在断言 = 生产 = 固定数据。
        assert_eq!(openai[2].default_model, "MiniMax-M3");
        assert_eq!(openai[2].models, ["MiniMax-M3", "MiniMax-Text-01"]);
        assert_eq!(openai[3].base_url, "http://localhost:11434/v1");
        assert_eq!(openai[3].default_model, "llama3.3");
        assert_eq!(openai[4].base_url, "");
        assert!(openai[4].models.is_empty());
        let anthropic = &fixture.presets.anthropic;
        assert_eq!(anthropic[0].base_url, "https://api.anthropic.com");
        assert_eq!(anthropic[0].default_model, "claude-3-7-sonnet-20250219");
        assert_eq!(anthropic[1].base_url, "https://api.minimaxi.com/anthropic");
        assert_eq!(anthropic[1].id, "minimax_anthropic");
    }

    #[test]
    fn the_fixture_state_is_the_production_initial_state() {
        let fixture = ProviderFixture::load().expect("固定数据可读");
        let state = &fixture.state;
        assert_eq!(state.protocol, "openai");
        assert_eq!(state.preset, "openai");
        assert_eq!(state.base_url, "https://api.openai.com/v1");
        assert_eq!(state.model, "gpt-4o");
        assert_eq!(state.anthropic_version, "2023-06-01");
        assert!(state.api_key.is_empty(), "首屏没有组内密钥（与参考侧一致）");
        assert!(!state.show_api_key);
        assert!(!state.show_advanced_gateway);
        assert!(!state.has_api_key);
        assert!(!state.models_loading);
        assert_eq!(state.models_error, "");
        assert_eq!(state.edit_base_url, "http://127.0.0.1:8080");
    }

    #[test]
    fn the_receipt_signals_start_as_none_like_the_production_page() {
        // 参考侧是生产初始态（三份回执全是 null）⇒ 首屏不得渲染回执。
        let fixture = ProviderFixture::load().expect("固定数据可读");
        assert!(fixture.test_result.is_some(), "固定数据里有回执素材");
        assert!(fixture.apply_result.is_some());
        assert!(fixture.effective_config.is_some());
        let host = ProviderHost::new(fixture);
        assert!(host.test_result.get().is_none(), "首屏无测连接回执");
        assert!(host.apply_result.get().is_none(), "首屏无热应用回执");
        assert!(host.effective_config.get().is_none(), "首屏无生效配置");
    }

    #[test]
    fn the_keychain_fixture_only_carries_a_masked_string() {
        let fixture = ProviderFixture::load().expect("固定数据可读");
        assert!(fixture.keychain.stored_key_exists);
        assert_eq!(fixture.keychain.stored_key_masked, "sk-****abc");
        // 固定数据里的串**已经是打码串**（再打码 ⇒ 不变）。
        assert_eq!(
            mask_api_key(&fixture.keychain.stored_key_masked),
            fixture.keychain.stored_key_masked
        );
    }

    #[test]
    fn opening_the_modal_clears_and_then_fills_the_keychain_echo() {
        // 生产 `openApiKeyModal()`：先清空 + 置 reading，再填回显。
        let host = ProviderHost::new(ProviderFixture::load().expect("固定数据可读"));
        let props = host.props();
        (props.on_action)("open-api-key-modal");
        assert!(host.show_api_key_modal.get(), "模态打开");
        assert!(!host.keychain_loading.get(), "读完了");
        assert!(host.stored_key_exists.get(), "有已存密钥");
        assert_eq!(host.stored_key_masked.get(), "sk-****abc");
        assert_eq!(host.temp_api_key.get(), "");
        assert_eq!(host.last_intent(), "open-api-key-modal");
    }

    #[test]
    fn the_show_key_intent_flips_the_structural_branch() {
        let host = ProviderHost::new(ProviderFixture::load().expect("固定数据可读"));
        let props = host.props();
        assert!(!host.show_api_key.get());
        (props.on_action)("toggle-show-key");
        assert!(host.show_api_key.get(), "明文");
        (props.on_action)("toggle-show-key");
        assert!(!host.show_api_key.get(), "回密文");
    }

    #[test]
    fn the_submit_intent_records_one_whole_group_and_fills_the_receipts() {
        // ## 判据 T5（**整组提交原子性**）的宿主侧断言
        let host = ProviderHost::new(ProviderFixture::load().expect("固定数据可读"));
        let props = host.props();
        assert_eq!(host.submit_count(), 0);
        (props.on_submit_group)("provider-group", &props.draft());
        assert_eq!(host.submit_count(), 1, "一次提交");
        let snapshot = host.last_submit();
        for field in SUBMIT_FIELDS {
            assert!(
                snapshot.contains(&format!("{field}=")),
                "整组快照缺字段 {field}：{snapshot}"
            );
        }
        // 密钥在快照里**已打码**（空 ⇒ `****`）。
        assert!(snapshot.contains("apiKey=****"), "{snapshot}");
        // 回执灌入（**HTTP 未接线** ⇒ 那是固定回执）。
        assert!(host.apply_result.get().is_some());
        assert!(host.effective_config.get().is_some());
        // 闪 ✓（提交后立刻是 ack 态）。
        let flags = host.text_flags.get();
        let ack = flags
            .iter()
            .filter(|(_, state)| *state == FieldFlagState::Ack)
            .count();
        assert_eq!(ack, 4, "四个文本键都闪 ✓");
    }

    #[test]
    fn an_empty_key_is_rejected_before_any_write() {
        let host = ProviderHost::new(ProviderFixture::load().expect("固定数据可读"));
        let props = host.props();
        (props.on_action)("open-api-key-modal");
        (props.on_action)("save-new-api-key");
        assert_eq!(host.keychain_action_error.get(), EMPTY_KEY_ERROR);
        assert!(host.show_api_key_modal.get(), "没写成功 ⇒ 模态不关");
        assert!(!host.stored_key_exists.get() || host.stored_key_masked.get() == "sk-****abc");
    }

    #[test]
    fn saving_a_new_key_masks_it_and_closes_the_modal() {
        let host = ProviderHost::new(ProviderFixture::load().expect("固定数据可读"));
        let props = host.props();
        (props.on_action)("open-api-key-modal");
        host.temp_api_key
            .set(String::from("provider-placeholder-key"));
        (props.on_action)("save-new-api-key");
        assert_eq!(
            host.stored_key_masked.get(),
            mask_api_key("provider-placeholder-key")
        );
        assert!(!host.show_api_key_modal.get(), "写成功 ⇒ 关模态");
        assert_eq!(host.temp_api_key.get(), "");
        // 发布面**没有明文密钥**（只有打码串与「有没有」）。
        let state = host.state();
        for (key, value) in &state {
            assert!(
                !value.contains("provider-placeholder-key"),
                "发布面泄露了明文密钥：{key}={value}"
            );
        }
        assert_eq!(
            state["apiKeyMasked"],
            mask_api_key("provider-placeholder-key")
        );
    }

    #[test]
    fn deleting_the_stored_key_also_clears_the_group_key() {
        // 生产 `deleteStoredKey()`：删钥匙串 + 清组内密钥。
        let host = ProviderHost::new(ProviderFixture::load().expect("固定数据可读"));
        let props = host.props();
        (props.on_action)("open-api-key-modal");
        host.api_key.set(String::from("provider-placeholder-key"));
        (props.on_action)("delete-stored-key");
        assert!(!host.stored_key_exists.get());
        assert_eq!(host.stored_key_masked.get(), "");
        assert_eq!(host.api_key.get(), "", "组内密钥同步清空");
    }

    #[test]
    fn the_test_connection_intent_fills_the_shared_receipt() {
        let host = ProviderHost::new(ProviderFixture::load().expect("固定数据可读"));
        let props = host.props();
        (props.on_action)("test-connection");
        let result = host.test_result.get().expect("有回执");
        assert!(result.ok);
        assert_eq!(result.latency_ms, Some(128.0));
        assert_eq!(result.models.len(), 3);
        assert!(!host.testing.get(), "跑完 ⇒ 不在探测中");
    }

    #[test]
    fn switching_the_protocol_restores_the_other_buffer() {
        // 生产 `switchProtocol`：双缓冲 + 清测连接回执 + Anthropic 侧有协议头。
        let host = ProviderHost::new(ProviderFixture::load().expect("固定数据可读"));
        let props = host.props();
        (props.on_action)("test-connection");
        (props.on_action)("protocol:anthropic");
        assert_eq!(host.protocol.get(), "anthropic");
        assert!(host.test_result.get().is_none(), "切协议清回执");
        // 目标协议没有 buffer ⇒ 状态保持当前值（生产同样如此）。
        (props.on_action)("preset:anthropic");
        assert_eq!(host.base_url.get(), "https://api.anthropic.com");
        assert_eq!(host.model.get(), "claude-3-7-sonnet-20250219");
        // 切回 openai ⇒ 恢复 openai 侧的 buffer（当时是 OpenAI 官方 / gpt-4o）。
        (props.on_action)("protocol:openai");
        assert_eq!(host.protocol.get(), "openai");
        assert_eq!(host.base_url.get(), "https://api.openai.com/v1");
        assert_eq!(host.model.get(), "gpt-4o");
    }

    #[test]
    fn the_first_protocol_switch_restores_the_initial_buffer() {
        // 生产的两份缓冲**有初值**（官方预设）⇒ 首次切到 Anthropic 就恢复它们。
        let host = ProviderHost::new(ProviderFixture::load().expect("固定数据可读"));
        let props = host.props();
        (props.on_action)("protocol:anthropic");
        assert_eq!(host.protocol.get(), "anthropic");
        assert_eq!(host.preset.get(), "anthropic");
        assert_eq!(host.base_url.get(), "https://api.anthropic.com");
        assert_eq!(host.model.get(), "claude-3-7-sonnet-20250219");
        assert_eq!(host.anthropic_version.get(), "2023-06-01");
    }

    #[test]
    fn selecting_a_preset_moves_the_endpoint_and_the_model() {
        let host = ProviderHost::new(ProviderFixture::load().expect("固定数据可读"));
        let props = host.props();
        (props.on_action)("preset:deepseek");
        assert_eq!(host.preset.get(), "deepseek");
        assert_eq!(host.base_url.get(), "https://api.deepseek.com/v1");
        assert_eq!(host.model.get(), "deepseek-v4-flash");
        // `custom` 预设 ⇒ 端点与模型都不动（生产 `p.id !== 'custom'`）。
        (props.on_action)("preset:custom");
        assert_eq!(host.base_url.get(), "https://api.deepseek.com/v1");
    }

    #[test]
    fn the_family_and_the_model_rule_come_from_the_kit() {
        // 宿主持有的两个**口径函数**在 kit 里（单一真源）。
        assert_eq!(provider_family("anthropic", "openai"), "anthropic");
        let fixture = ProviderFixture::load().expect("固定数据可读");
        let deepseek = fixture
            .presets
            .openai
            .iter()
            .find(|item| item.id == "deepseek")
            .expect("有 DeepSeek 预设");
        assert_eq!(
            preset_model_after_select(deepseek, "mystery-model"),
            None,
            "用户自定义的模型不动"
        );
    }

    #[test]
    fn the_host_switches_sections_and_ignores_unknown_ones() {
        let host = ProviderHost::new(ProviderFixture::load().expect("固定数据可读"));
        assert_eq!(host.section(), "models");
        assert!(host.select_section("api-key-modal"));
        assert_eq!(host.section(), "api-key-modal");
        assert!(!host.select_section("no-such-section"), "未知分区 ⇒ 不切");
        assert_eq!(host.section(), "api-key-modal");
    }

    #[test]
    fn the_host_records_the_field_dirty_keys_without_a_dom_reader() {
        let host = ProviderHost::new(ProviderFixture::load().expect("固定数据可读"));
        let props = host.props();
        (props.on_field_input)("provider.baseUrl,provider.model");
        let flags = host.text_flags.get();
        let dirty = flags
            .iter()
            .filter(|(_, state)| *state == FieldFlagState::Dirty)
            .count();
        assert_eq!(dirty, 2);
        assert_eq!(host.last_intent(), "field:provider.baseUrl,provider.model");
    }

    #[test]
    fn the_host_reads_back_field_values_through_the_reader_hook() {
        // wasm 层的 `bind:value` 等价物：回读钩子 ⇒ 宿主信号。
        let host = ProviderHost::new(ProviderFixture::load().expect("固定数据可读"));
        host.set_field_reader(Rc::new(|key: &str| {
            if key == "provider.baseUrl" {
                Some(String::from("http://127.0.0.1:9999/v1"))
            } else {
                None
            }
        }));
        let props = host.props();
        (props.on_field_input)("provider.baseUrl");
        assert_eq!(host.base_url.get(), "http://127.0.0.1:9999/v1");
    }

    #[test]
    fn the_compare_hook_masks_the_raw_key_with_the_local_function() {
        // 对照装置入口 ⇒ **本侧自己的**打码函数（T6 的跨侧证据来源）。
        let host = ProviderHost::new(ProviderFixture::load().expect("固定数据可读"));
        host.apply_stored_key("abcdefghijklmnop");
        assert_eq!(host.stored_key_masked.get(), "abc****nop");
        host.apply_stored_key("12345678");
        assert_eq!(host.stored_key_masked.get(), "****", "8 位 ⇒ 整体打码");
        host.apply_stored_key("  abcdefghij  ");
        assert_eq!(host.stored_key_masked.get(), "abc****hij", "先 trim");
    }

    #[test]
    fn the_published_state_never_carries_a_plaintext_key() {
        let host = ProviderHost::new(ProviderFixture::load().expect("固定数据可读"));
        host.api_key.set(String::from("provider-placeholder-key"));
        let state = host.state();
        assert_eq!(state["hasGroupApiKey"], "true");
        assert_eq!(
            state["apiKeyMasked"],
            mask_api_key("provider-placeholder-key")
        );
        for value in state.values() {
            assert!(!value.contains("provider-placeholder-key"));
        }
    }
}
