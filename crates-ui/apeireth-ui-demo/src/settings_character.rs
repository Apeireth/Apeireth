//! 设置页三页 · `character` 子面的 **wasm 宿主挂载**模块（人格与记忆）。
//!
//! ## 本模块是什么
//!
//! **宿主侧**：持有四个分区的状态（人设 / 印象 / 能力表 / 学习日志）+ **固定
//! 数据**（`fixtures/character.json` + `fixtures/capabilities.json`），把
//! `ui_kit::settings_character` 的组件挂到页面的 `#app` 容器里。
//!
//! ## 诚实边界
//!
//! | 事项 | 口径 |
//! |---|---|
//! | 数据 | **固定 fixture**（宿主注入）⇒ 引擎不碰 HTTP / SSE 契约（设计 §1.3 第 6 条） |
//! | 人设增删 / 落盘 | **宿主侧**（本地存储）⇒ 引擎只发意图 |
//! | 印象落盘 / 立即总结 | **宿主侧**（本地存储 + 模型调用）⇒ 引擎只发意图 |
//! | 能力开关的 apply | **宿主侧**（配置持久化 + env 注入 + 网关热应用）⇒ 引擎只发意图 |
//! | 学习日志的读 / 撤销 | **宿主侧**（后端读面）⇒ 引擎只发意图 |
//! | **时刻的格式化** | 生产用 `toLocaleString()` ⇒ **宿主用浏览器自己的 `Date` 算**（与生产**同口径**）⇒ 组件只渲染文本 |
//! | 原生控件的取值 | 生产用 **property**（`bind:value`）⇒ 宿主在每次渲染后**回写 property**（不写属性） |
//!
//! ## 「一个组件 + 一张表」的落点
//!
//! 能力开关的**定义表**来自 `fixtures/capabilities.json` 的 `capDefs`
//! （**逐字取自生产**）⇒ 宿主把它装配成 [`CapabilityDef`] 表 ⇒
//! 组件 [`render_capability_card`] 只吃表 ⇒ **组件里没有一行能力定义**。
//!
//! ⚠️ **图标名**：固定数据里存的是**生产的 import 名**（`MessageSquarePlus`）
//! ⇒ 宿主经 [`icons::icon_by_import_name`] 映射到**抽取出来的几何数据**
//! （class 后缀取**真实文件名** ⇒ `MessageSquarePlus` ⇒ `lucide-message-square-plus`）。

use std::collections::BTreeMap;
use std::rc::Rc;

use apeireth_ui_core::{Runtime, Signal};
use apeireth_ui_kit::settings_character::{
    icons, CapabilityDef, CapabilityTable, CapabilityValue, CharacterPanelsProps, Impression,
    KnobSpec, Persona, TuningLogEntry,
};
use apeireth_ui_kit::settings_frame::FieldFlagState;

/// 固定数据的 JSON 值（**复用主题面的极小解析器** ⇒ 同一 crate 内不重复实现）。
///
/// ⚠️ 主题面的解析器只暴露了 `get` / `as_str` / `as_bool` / `as_array`
/// ⇒ 数值与对象用**本模块的 helper** 取（直接匹配枚举变体 ⇒ 不改共享文件）。
type Json = crate::settings_theme::Json;

/// 人格与记忆的固定数据（**编译期内嵌** ⇒ 与参考侧同一份文件）。
pub const CHARACTER_FIXTURE_JSON: &str =
    include_str!("../harness/settings/fixtures/character.json");

/// 能力开关的固定数据（**编译期内嵌** ⇒ 与参考侧同一份文件）。
pub const CAPABILITIES_FIXTURE_JSON: &str =
    include_str!("../harness/settings/fixtures/capabilities.json");

/// 全局模型（人设卡的占位；**与参考侧同一份**：参考侧的 `config.model`）。
pub const GLOBAL_MODEL: &str = "gpt-4o";

/// 检索温度的旋钮规格（**生产 markup 的嵌套行** ⇒ 不在能力注册表里 ⇒ 宿主补齐）。
///
/// ⚠️ 生产把它插在 `morphologyRecall` **父行的旁边（同层）** ⇒ 组件的
/// 「一项产出两个节点」由本规格驱动。
pub const MORPHOLOGY_TEMPERATURE_KNOB: KnobSpec = KnobSpec {
    value_key: "morphologyTemperature",
    icon: &icons::ICON_SLIDERS_HORIZONTAL,
    label: "检索温度",
    env: "APEIRETH_MORPHOLOGY_TEMPERATURE",
    desc: "形态学读数活跃度，默认 1.0；越高检索面越宽。",
    min: "0.1",
    max: "2",
    step: "0.1",
    digits: 1,
    baseline: None,
    prefix: "",
    suffix: "",
};

/// 「从使用中学习」的能力定义（**生产 markup 的单行开关** ⇒ 宿主补齐）。
///
/// ⚠️ **不是 `const`**：能力定义的字段是 `String` ⇒ `const` 里放不了 ⇒
/// 这里是**函数**（每次调用建一份 ⇒ 成本可忽略）。
pub fn self_tuning_def() -> CapabilityDef {
    CapabilityDef::new(
        "selfTuning",
        &icons::ICON_SPARKLES,
        "从使用中学习",
        "APEIRETH_ENABLE_SELF_TUNING",
        "打开后引擎按真实使用信号自动微调体验参数（当前真接信号 = 记忆检索命中/未命中）；\n                  每次自动调整可见、可撤销，日志见下方学习日志。默认关。\n                  挂在记忆检索信号链上：记忆召回关闭时此项不生效（自报照实）。",
    )
}

/// 体验参数的四个旋钮（**生产 markup 的四行** ⇒ 宿主补齐）。
pub const DISPOSITION_KNOBS: [KnobSpec; 4] = [
    KnobSpec {
        value_key: "memoryFade",
        icon: &icons::ICON_ARCHIVE,
        label: "遗忘衰减强度",
        env: "APEIRETH_TUNE_MEMORY_FADE",
        desc: "遗忘衰减强度倍率（有效遗忘半衰期 = 24h / 值；1.0 = 现行为）。",
        min: "0.25",
        max: "4",
        step: "0.25",
        digits: 2,
        baseline: Some("基线 1.0"),
        prefix: "×",
        suffix: "",
    },
    KnobSpec {
        value_key: "curiosityStrength",
        icon: &icons::ICON_RADAR,
        label: "好奇心强度",
        env: "APEIRETH_TUNE_CURIOSITY_STRENGTH",
        desc: "好奇强度倍率（好奇日预算 = 2000 × 值）。",
        min: "0.25",
        max: "4",
        step: "0.25",
        digits: 2,
        baseline: Some("基线 1.0"),
        prefix: "×",
        suffix: "",
    },
    KnobSpec {
        value_key: "toneSaturation",
        icon: &icons::ICON_HEART_HANDSHAKE,
        label: "语气情绪饱和度",
        env: "APEIRETH_TUNE_TONE_SATURATION",
        desc: "语气情绪饱和度倍率（情绪注入混合 × 值；0 = 纯关系基线）。",
        min: "0",
        max: "2",
        step: "0.2",
        digits: 2,
        baseline: Some("基线 1.0"),
        prefix: "×",
        suffix: "",
    },
    KnobSpec {
        value_key: "consolidationCadence",
        icon: &icons::ICON_TIMER,
        label: "整合节奏",
        env: "APEIRETH_TUNE_CONSOLIDATION_CADENCE",
        desc: "整合节奏：每 N 回合触发一次记忆整合（1 = 每回合 = 现行为）。",
        min: "1",
        max: "10",
        step: "1",
        digits: 0,
        baseline: Some("基线 1（每回合）"),
        prefix: "每 ",
        suffix: " 回合",
    },
];

// ── 固定数据的装配 ──────────────────────────────────────────────────────────

/// 本面要用的固定数据（**装配后**）。
#[derive(Debug, Clone, PartialEq)]
pub struct CharacterFixture {
    /// 人设列表。
    pub personas: Vec<Persona>,
    /// 当前伙伴 id。
    pub active_persona_id: String,
    /// 印象档案。
    pub impression: Impression,
    /// 印象草稿。
    pub impression_draft: String,
    /// 印象操作的错误文案。
    pub impression_error: String,
    /// 「立即总结」进行中。
    pub impression_busy: bool,
    /// 学习日志（`None` ⇒ **不可读** ⇒ 走「仅桌面版可读」分支）。
    pub tuning_log: Option<Vec<TuningLogEntry>>,
    /// 字段旗标。
    pub text_flags: Vec<(String, FieldFlagState)>,
    /// 能力表（键 → 取值）。
    pub capabilities: CapabilityTable,
    /// 记忆流的能力定义。
    pub memory_flow_defs: Vec<CapabilityDef>,
    /// 认知增强的能力定义。
    pub cognition_defs: Vec<CapabilityDef>,
    /// 社区与账本的能力定义。
    pub community_defs: Vec<CapabilityDef>,
    /// **依赖子行**的能力定义（生产的 `TOOL_SUB_DEFS` ⇒ 渲染在「工具与安全」分区）。
    ///
    /// ⚠️ 本面（人格与记忆）**不渲染**它们 ⇒ 它们只喂给**组件探针**
    /// （`requires` 机制的**两态**对照 ⇒ 见 [`wasm::mount_probe`]）。
    pub tool_sub_defs: Vec<CapabilityDef>,
    /// 体验参数的旋钮规格。
    pub disposition_knobs: Vec<KnobSpec>,
    /// 性情区的预设名。
    pub disposition_presets: Vec<String>,
}

impl CharacterFixture {
    /// 读固定数据并装配（**固定数据坏了 ⇒ 显式报错**）。
    pub fn load() -> Result<Self, String> {
        let character = crate::settings_theme::parse_json(CHARACTER_FIXTURE_JSON)
            .map_err(|error| format!("人设固定数据解析失败：{error}"))?;
        let capabilities = crate::settings_theme::parse_json(CAPABILITIES_FIXTURE_JSON)
            .map_err(|error| format!("能力固定数据解析失败：{error}"))?;

        let personas = read_personas(&character)?;
        let active_persona_id = read_string(&character, "activePersonaId")?;
        let impression = read_impression(&character)?;
        let impression_draft = read_string(&character, "impressionDraft")?;
        let impression_error = read_string(&character, "impressionError")?;
        let impression_busy = read_bool(&character, "impressionBusy")?;
        let tuning_log = read_tuning_log(&character)?;
        let text_flags = read_text_flags(&character)?;
        let capability_table = read_capability_table(&capabilities)?;
        let memory_flow_defs = read_cap_defs(&capabilities, "memoryFlow")?;
        let mut cognition_defs = read_cap_defs(&capabilities, "cognition")?;
        // 嵌套行：**挂在 `morphologyRecall` 这一行上** ⇒ 组件在父行**之后**
        // （同层）插入它。
        if let Some(def) = cognition_defs
            .iter_mut()
            .find(|def| def.key == "morphologyRecall")
        {
            def.nested = Some(MORPHOLOGY_TEMPERATURE_KNOB.clone());
        }
        let community_defs = read_cap_defs(&capabilities, "community")?;
        let tool_sub_defs = read_cap_defs(&capabilities, "toolSub")?;
        let disposition_knobs = DISPOSITION_KNOBS.to_vec();
        let disposition_presets = read_preset_names(&character)?;

        Ok(Self {
            personas,
            active_persona_id,
            impression,
            impression_draft,
            impression_error,
            impression_busy,
            tuning_log,
            text_flags,
            capabilities: capability_table,
            memory_flow_defs,
            cognition_defs,
            community_defs,
            tool_sub_defs,
            disposition_knobs,
            disposition_presets,
        })
    }
}

/// 取一个数值字段（**缺字段 ⇒ `None`**）。
fn json_number(value: &Json, key: &str) -> Option<f64> {
    match value.get(key) {
        Some(Json::Number(number)) => Some(*number),
        _ => None,
    }
}

/// 取一个数值（**不是数值 ⇒ `None`**）。
fn json_number_value(value: &Json) -> Option<f64> {
    match value {
        Json::Number(number) => Some(*number),
        _ => None,
    }
}

/// 取一个对象的键值表（**不是对象 ⇒ 空**）。
fn json_object(value: &Json) -> &[(String, Json)] {
    match value {
        Json::Object(entries) => entries.as_slice(),
        _ => &[],
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

/// 读人设列表。
fn read_personas(value: &Json) -> Result<Vec<Persona>, String> {
    let items = value
        .get("personas")
        .and_then(|item| item.as_array())
        .ok_or_else(|| "固定数据缺字段：personas".to_owned())?;
    let mut personas = Vec::new();
    for item in items {
        personas.push(Persona {
            id: read_string(item, "id")?,
            name: read_string(item, "name")?,
            model: read_string(item, "model")?,
            persona: read_string(item, "persona")?,
        });
    }
    Ok(personas)
}

/// 读印象档案（**时刻文本由宿主在浏览器里格式化** ⇒ 这里只取原值）。
fn read_impression(value: &Json) -> Result<Impression, String> {
    let entry = value
        .get("impression")
        .ok_or_else(|| "固定数据缺字段：impression".to_owned())?;
    let source = read_string(entry, "source")?;
    Ok(Impression {
        persona_id: read_string(entry, "personaId")?,
        persona_name: read_string(entry, "personaName")?,
        summary: read_string(entry, "summary")?,
        updated_at: json_number(entry, "updatedAt")
            .map(|value| value as i64)
            .ok_or_else(|| "固定数据缺字段：impression.updatedAt".to_owned())?,
        source,
        auto_refresh: entry
            .get("autoRefresh")
            .and_then(|item| item.as_bool())
            .ok_or_else(|| "固定数据缺字段：impression.autoRefresh".to_owned())?,
        updated_text: String::new(),
    })
}

/// 读学习日志（**`None` ⇒ 不可读**；**时刻文本由宿主格式化**）。
fn read_tuning_log(value: &Json) -> Result<Option<Vec<TuningLogEntry>>, String> {
    let Some(items) = value.get("tuningLog").and_then(|item| item.as_array()) else {
        return Ok(None);
    };
    let labels = value
        .get("tuningParamLabels")
        .map(json_object)
        .unwrap_or_default();
    let mut entries = Vec::new();
    for item in items {
        let param = read_string(item, "param")?;
        let label = labels
            .iter()
            .find(|(key, _)| key == &param)
            .and_then(|(_, value)| value.as_str())
            .map(str::to_owned)
            .unwrap_or_else(|| param.clone());
        entries.push(TuningLogEntry {
            seq: json_number(item, "seq")
                .map(|value| value as i64)
                .ok_or_else(|| "固定数据缺字段：tuningLog.seq".to_owned())?,
            param: param.clone(),
            param_label: label,
            previous: json_number(item, "previous").unwrap_or_default(),
            next: json_number(item, "next").unwrap_or_default(),
            reason: read_string(item, "reason")?,
            at_epoch_ms: json_number(item, "at_epoch_ms")
                .map(|value| value as i64)
                .unwrap_or_default(),
            at_text: String::new(),
        });
    }
    // 生产：**按 seq 倒序**（最新在前）。
    // ⚠️ 用 `sort_by_key(Reverse(..))`（**稳定排序** ⇒ 相同 `seq` 保持原序
    // ⇒ 与脚本侧的 `sort` 稳定性同序）⇒ 不要用 `sort_unstable_by`。
    entries.sort_by_key(|entry| std::cmp::Reverse(entry.seq));
    Ok(Some(entries))
}

/// 读字段旗标（**键 → 状态**；`true` ⇒ 未保存）。
fn read_text_flags(value: &Json) -> Result<Vec<(String, FieldFlagState)>, String> {
    let entries: Vec<(String, Json)> = value
        .get("textFlags")
        .map(|item| json_object(item).to_vec())
        .unwrap_or_default();
    let mut flags = Vec::new();
    for (key, item) in entries {
        if key.starts_with('_') {
            continue;
        }
        let dirty = item.as_bool().unwrap_or(false);
        flags.push((
            key.clone(),
            if dirty {
                FieldFlagState::Dirty
            } else {
                FieldFlagState::None
            },
        ));
    }
    Ok(flags)
}

/// 读能力表（键 → 取值）。
fn read_capability_table(value: &Json) -> Result<CapabilityTable, String> {
    let entries: Vec<(String, Json)> = value
        .get("capabilities")
        .map(|item| json_object(item).to_vec())
        .ok_or_else(|| "固定数据缺字段：capabilities".to_owned())?;
    let mut table = CapabilityTable::new();
    for (key, item) in entries {
        if key.starts_with('_') {
            continue;
        }
        if let Some(flag) = item.as_bool() {
            table.insert(key.clone(), CapabilityValue::Bool(flag));
        } else if let Some(number) = json_number_value(&item) {
            table.insert(key.clone(), CapabilityValue::Number(number));
        }
    }
    Ok(table)
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
        let icon = icons::icon_by_import_name(&icon_name).ok_or_else(|| {
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
        if item.get("invert").and_then(|value| value.as_bool()) == Some(true) {
            def = def.inverted();
        }
        if let Some(required) = item.get("requires").and_then(|value| value.as_str()) {
            def = def.requires(required);
        }
        // 生产的依赖子行是 `class="cap-row cap-row-nested"` ⇒ **嵌套展示**
        // ⇒ 归**定义**（表的一行）⇒ 组件照抄。
        if group == "toolSub" {
            def = def.with_row_class("cap-row-nested");
        }
        defs.push(def);
    }
    Ok(defs)
}

/// 读性情区的预设名（**顺序即渲染顺序**）。
fn read_preset_names(value: &Json) -> Result<Vec<String>, String> {
    let items = value
        .get("dispositionPresets")
        .and_then(|item| item.as_array())
        .ok_or_else(|| "固定数据缺字段：dispositionPresets".to_owned())?;
    let mut names = Vec::new();
    for item in items {
        names.push(read_string(item, "id")?);
    }
    // 生产的第四枚芯片是「恢复基线」（**不在固定数据的预设表里** ⇒ 由宿主补）。
    names.push(String::from("恢复基线"));
    Ok(names)
}

// ── 宿主状态 ────────────────────────────────────────────────────────────────

/// 宿主状态（**状态归宿主** ⇒ 组件是纯函数）。
#[derive(Clone)]
pub struct CharacterHost {
    /// 运行时（信号宿主）。
    runtime: Runtime,
    /// 当前分区。
    section: Signal<String>,
    /// 人设列表。
    personas: Signal<Vec<Persona>>,
    /// 当前伙伴 id。
    active_persona_id: Signal<String>,
    /// 字段旗标。
    text_flags: Signal<Vec<(String, FieldFlagState)>>,
    /// 印象档案。
    impression: Signal<Impression>,
    /// 印象草稿。
    impression_draft: Signal<String>,
    /// 印象错误。
    impression_error: Signal<String>,
    /// 总结进行中。
    impression_busy: Signal<bool>,
    /// 能力表。
    capabilities: Signal<CapabilityTable>,
    /// 生效中的键。
    pending_key: Signal<Option<String>>,
    /// 学习日志。
    tuning_log: Signal<Option<Vec<TuningLogEntry>>>,
    /// 学习日志读取中。
    tuning_log_loading: Signal<bool>,
    /// 组件探针是否挂载（**`requires` 机制的两态对照** ⇒ 由对照脚本按需打开）。
    probe_enabled: Signal<bool>,
    /// 最近一次意图（**对照脚本可读**）。
    intent: Signal<String>,
    /// 固定数据（**不可变** ⇒ 装配一次）。
    fixture: CharacterFixture,
}

impl CharacterHost {
    /// 建一份宿主状态（**固定数据坏了 ⇒ 显式报错**）。
    pub fn new(fixture: CharacterFixture) -> Self {
        let runtime = Runtime::new();
        let section = runtime.signal(String::from("personality"));
        let personas = runtime.signal(fixture.personas.clone());
        let active_persona_id = runtime.signal(fixture.active_persona_id.clone());
        let text_flags = runtime.signal(fixture.text_flags.clone());
        let impression = runtime.signal(fixture.impression.clone());
        let impression_draft = runtime.signal(fixture.impression_draft.clone());
        let impression_error = runtime.signal(fixture.impression_error.clone());
        let impression_busy = runtime.signal(fixture.impression_busy);
        let capabilities = runtime.signal(fixture.capabilities.clone());
        let pending_key = runtime.signal(None);
        // ⚠️ **学习日志：喂固定数据**（**不是** `None`）。
        //
        // 生产的读面是 **Tauri IPC**（`readTuningLog`）⇒ 真实浏览器里读不到；
        // 但**对照 harness 的替身明确让它可达**：
        // `mock/real/desktop-bridge.js` 的 `isDesktop()` 返回 **true**、
        // `readTuningLog()` 返回 `fixtures/character.json` 的 `tuningLog`
        // （其注释原话：「替身返回 true ⇒ 学习日志分支可达」）
        // ⇒ **参考侧渲染的是「日志行」那一支**，不是 notice-box。
        //
        // ⇒ 引擎侧必须**渲染同一支** ⇒ 信号初值 = **固定数据的日志**。
        // ⚠️ **此前的实现按「浏览器里读不到」把信号置 `None`** ⇒ 引擎侧渲染
        // notice-box、参考侧渲染日志行 ⇒ **共享入口的 DOM 全等判据报差异**
        // （`character / disposition`：`根/5/1` 参考 `cap-row cap-row-static`
        // vs 引擎 `notice-box`）⇒ **该推理在 harness 语境下不成立**。
        //
        // ⇒ 「不可读」那一支（`None`）**在浏览器里不可对照**（替身恒返回日志）
        // ⇒ 登记为**未覆盖**（native 单测覆盖它的渲染）。
        let tuning_log = runtime.signal(fixture.tuning_log.clone());
        let tuning_log_loading = runtime.signal(false);
        let intent = runtime.signal(String::new());
        let probe_enabled = runtime.signal(false);
        Self {
            runtime,
            section,
            personas,
            active_persona_id,
            text_flags,
            impression,
            impression_draft,
            impression_error,
            impression_busy,
            capabilities,
            pending_key,
            tuning_log,
            tuning_log_loading,
            probe_enabled,
            intent,
            fixture,
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

    /// 切到某个分区（**未知分区 ⇒ 不切** ⇒ 不假装已接线）。
    pub fn select_section(&self, id: &str) -> bool {
        if !matches!(
            id,
            "personality" | "impression" | "cognition" | "disposition"
        ) {
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

    /// 能力表的快照（**探针用**）。
    pub fn capabilities_table(&self) -> CapabilityTable {
        self.capabilities.get()
    }

    /// 依赖子行的定义（**探针用**）。
    pub fn tool_sub_defs(&self) -> Vec<CapabilityDef> {
        self.fixture.tool_sub_defs.clone()
    }

    /// 打开 / 关闭**组件探针**（`requires` 机制的两态对照用）。
    pub fn set_probe_enabled(&self, enabled: bool) {
        self.probe_enabled.set(enabled);
    }

    /// 探针是否打开。
    pub fn probe_enabled(&self) -> bool {
        self.probe_enabled.get()
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
    pub fn props(&self) -> CharacterPanelsProps {
        let toggle = self.clone();
        let knob = self.clone();
        let select = self.clone();
        let add = self.clone();
        let remove = self.clone();
        let impression = self.clone();
        let preset = self.clone();
        let undo = self.clone();
        let submit = self.clone();
        let field = self.clone();
        CharacterPanelsProps {
            section: self.section.clone(),
            personas: self.personas.clone(),
            active_persona_id: self.active_persona_id.clone(),
            text_flags: self.text_flags.clone(),
            impression: self.impression.clone(),
            impression_draft: self.impression_draft.clone(),
            impression_error: self.impression_error.clone(),
            impression_busy: self.impression_busy.clone(),
            capabilities: self.capabilities.clone(),
            pending_key: self.pending_key.clone(),
            tuning_log: self.tuning_log.clone(),
            tuning_log_loading: self.tuning_log_loading.clone(),
            global_model: String::from(GLOBAL_MODEL),
            memory_flow_defs: self.fixture.memory_flow_defs.clone(),
            cognition_defs: self.fixture.cognition_defs.clone(),
            community_defs: self.fixture.community_defs.clone(),
            self_tuning_def: self_tuning_def(),
            disposition_knobs: self.fixture.disposition_knobs.clone(),
            disposition_presets: self.fixture.disposition_presets.clone(),
            on_toggle_capability: Rc::new(move |key: &str| {
                toggle.record_intent(&format!("toggle-capability:{key}"));
            }),
            on_set_knob: Rc::new(move |key: &str| {
                knob.record_intent(&format!("set-knob:{key}"));
            }),
            on_select_persona: Rc::new(move |id: &str| {
                select.record_intent(&format!("select-persona:{id}"));
            }),
            on_add_persona: Rc::new(move |action: &str| {
                add.record_intent(&format!("persona:{action}"));
            }),
            on_remove_persona: Rc::new(move |id: &str| {
                remove.record_intent(&format!("remove-persona:{id}"));
            }),
            on_impression: Rc::new(move |action: &str| {
                impression.record_intent(&format!("impression:{action}"));
            }),
            on_apply_preset: Rc::new(move |name: &str| {
                preset.record_intent(&format!("preset:{name}"));
            }),
            on_undo_tuning: Rc::new(move |action: &str| {
                undo.record_intent(&format!("tuning:{action}"));
            }),
            on_submit_group: Rc::new(move |group: &str| {
                submit.record_intent(&format!("submit:{group}"));
            }),
            on_field_input: Rc::new(move |field_key: &str| {
                field.record_intent(&format!("field:{field_key}"));
            }),
        }
    }

    /// 当前状态的快照（**对照脚本可读**）。
    pub fn state(&self) -> BTreeMap<String, String> {
        let mut state = BTreeMap::new();
        state.insert(String::from("section"), self.section.get());
        state.insert(
            String::from("activePersonaId"),
            self.active_persona_id.get(),
        );
        state.insert(String::from("intent"), self.intent.get());
        let table = self.capabilities.get();
        for (key, value) in table {
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
}

// ── wasm 装配（**native 编译不参与** ⇒ 行为由浏览器内对照钉住） ─────────────

/// wasm32 侧的装配。
#[cfg(target_arch = "wasm32")]
pub mod wasm {
    //! ## 诚实边界
    //!
    //! - 本模块整体被 `#[cfg(target_arch = "wasm32")]` 隔离 ⇒ **native 编译不参与**
    //!   ⇒ 它的行为**未经 native 侧验证**（浏览器内的验证由设置页 harness 承担：
    //!   像素对照 + DOM 全等 + 宽度同源断言）。
    //! - **真后端未接线**：数据面是**固定数据**（编译期内嵌的那两份）。
    //! - **原生控件的取值是 property**：生产用 `bind:value` ⇒ 引擎侧**不写**
    //!   `value` 属性 ⇒ 宿主在每次渲染后**回写 property**（否则像素对照会看到
    //!   空输入框）。

    use std::cell::RefCell;
    use std::rc::Rc;

    use apeireth_ui_core::{HostScheduler, UiComponent};
    use apeireth_ui_dom::{Renderer, VNode, WebDom};
    use apeireth_ui_kit::settings_character::CharacterPanels;
    use apeireth_ui_macro::view;
    use js_sys::Reflect;
    use wasm_bindgen::prelude::*;
    use web_sys::Element;

    use super::{CharacterFixture, CharacterHost};

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
        /// wasm 侧的单例应用（人格与记忆面形态）。
        static CHARACTER_APP: RefCell<Option<CharacterHost>> = const { RefCell::new(None) };
        /// 控制 API 的闭包（**必须活到页面结束**）。
        static CONTROL_CLOSURES: RefCell<Vec<Closure<dyn FnMut(JsValue) -> JsValue>>> =
            const { RefCell::new(Vec::new()) };
    }

    /// 用**浏览器自己的 `Date`** 格式化时刻（**与生产同口径**）。
    ///
    /// ## 为什么宿主算
    ///
    /// 生产用 `new Date(ms).toLocaleString()`（本地化格式 ⇒ 随浏览器 locale
    /// 变化）⇒ 引擎侧**不能自己拼**（拼出来必然与参考侧不同）⇒ 由宿主
    /// **借浏览器的 `Date`** 算 ⇒ **两侧逐字相同**。
    ///
    /// ## 失败口径
    ///
    /// 浏览器不提供 `Date` ⇒ 返回空串（**不静默编一个假时刻** ⇒ 画面上会
    /// 看得出「时刻缺失」）。
    fn format_time(epoch_ms: i64, locale: Option<&str>) -> String {
        let Some(constructor) = Reflect::get(&js_sys::global(), &JsValue::from_str("Date"))
            .ok()
            .and_then(|value| value.dyn_into::<js_sys::Function>().ok())
        else {
            return String::new();
        };
        let arguments = js_sys::Array::new();
        arguments.push(&JsValue::from_f64(epoch_ms as f64));
        let Ok(instance) = js_sys::Reflect::construct(&constructor, &arguments) else {
            return String::new();
        };
        let Some(formatter) = Reflect::get(&instance, &JsValue::from_str("toLocaleString"))
            .ok()
            .and_then(|value| value.dyn_into::<js_sys::Function>().ok())
        else {
            return String::new();
        };
        let call_arguments = js_sys::Array::new();
        if let Some(tag) = locale {
            call_arguments.push(&JsValue::from_str(tag));
        }
        js_sys::Reflect::apply(&formatter, &instance, &call_arguments)
            .ok()
            .and_then(|value| value.as_string())
            .unwrap_or_default()
    }

    /// 渲染整棵视图树（**舞台 + 作用域锚点**）。
    fn render_tree(host: &CharacterHost) -> VNode {
        let panel = CharacterPanels::render(&host.props());
        let probe = if host.probe_enabled() {
            Some(render_probe(host))
        } else {
            None
        };
        view! {
            div(class = "settings-stage") {
                div(class = "sp-character") {
                    { panel }
                    " "
                    { optional(probe) }
                }
            }
        }
    }

    /// `Option<VNode>` ⇒ 虚拟节点（`None` ⇒ **空片段** ⇒ 不产出任何节点）。
    fn optional(node: Option<VNode>) -> VNode {
        node.unwrap_or_else(|| VNode::fragment(Vec::new()))
    }

    /// 渲染**组件探针**（`requires` 机制的两态对照）。
    ///
    /// ## 这个探针是什么
    ///
    /// 生产的**依赖子行**（`fileWriteAutoPass` ← `fileWrite`）渲染在「工具与
    /// 安全」分区（**属能力与治理面**）⇒ 本面（人格与记忆）**不渲染**它。
    ///
    /// ⇒ 但 `requires` 的**判定口径**是**本面成型的组件**（`CapabilityCard`）
    /// ⇒ 本探针用**同一张表**（固定数据里的 `capDefs.toolSub`）渲染**那一行**
    /// ⇒ 对照脚本拿它与**参考侧「工具与安全」分区的同一行**逐像素 / 逐字对照。
    ///
    /// ⚠️ 探针**默认不挂载** ⇒ 面本身的对照画面**不受影响**。
    fn render_probe(host: &CharacterHost) -> VNode {
        let table = host.capabilities_table();
        let defs = host.tool_sub_defs();
        let rows = apeireth_ui_kit::settings_character::render_capability_rows(
            &defs,
            &table,
            None,
            &host.props(),
        );
        view! {
            div(class = "setting-block cap-probe-block") {
                div(class = "cap-card") {
                    for row in rows {
                        { row }
                    }
                }
            }
        }
    }

    /// 挂载人格与记忆面。
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

        let mut fixture = CharacterFixture::load()?;
        // 时刻文本：**借浏览器的 `Date`** 格式化（与生产同口径 ⇒ 两侧逐字相同）。
        fixture.impression.updated_text = format_time(fixture.impression.updated_at, None);
        if let Some(entries) = fixture.tuning_log.as_mut() {
            for entry in entries.iter_mut() {
                entry.at_text = format_time(entry.at_epoch_ms, Some("zh-CN"));
            }
        }
        let host = CharacterHost::new(fixture);
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

        CHARACTER_APP.with(|slot| {
            *slot.borrow_mut() = Some(host.clone());
        });
        install_control_api();
        mark_ready();
        publish_state(&host);
        Ok(())
    }

    // ⚠️ **不导出 wasm 函数**（`set_settings_section` 已被向导面占用）。
    //
    // ⇒ wasm 的**导出名是全局的** ⇒ 同一个名字导出两次 ⇒ **链接失败**
    // （实测报错：`symbol ... is already defined`）⇒ 各面**只能有一个**导出。
    //
    // ⇒ 本面的分区切换走**自己的控制 API**（`window.__CHARACTER__.selectSection`）
    // ⇒ 共享的 `web/engine/main.js` 里那句
    // `module.set_settings_section(id)` **对本面无效**（它调的是向导面的导出）
    // ⇒ **共享分发需要改**（见报告的「需要主线改的共享文件」）。

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
    fn publish_state(host: &CharacterHost) {
        let Some(window) = web_sys::window() else {
            return;
        };
        let state = host.state();
        let json = serde_free_json(&state);
        let _ = Reflect::set(
            window.as_ref(),
            &JsValue::from_str("__CHARACTER_STATE__"),
            &JsValue::from_str(&json),
        );
        let _ = Reflect::set(
            window.as_ref(),
            &JsValue::from_str("__CHARACTER_INTENT__"),
            &JsValue::from_str(&host.last_intent()),
        );
    }

    /// 状态 ⇒ JSON 文本（**零依赖**：只序列化字符串键值 ⇒ 逐字节可控**）。
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

    /// 装控制 API（`window.__CHARACTER__`）—— 对照脚本用它设置状态。
    fn install_control_api() {
        let Some(window) = web_sys::window() else {
            return;
        };

        let set_capability = Closure::wrap(Box::new(|args: JsValue| -> JsValue {
            // **一个数组参数**（闭包只拿第一个实参 ⇒ 两个字段打包成数组）。
            let key = Reflect::get(&args, &JsValue::from_f64(0.0))
                .ok()
                .and_then(|value| value.as_string())
                .unwrap_or_default();
            let raw = Reflect::get(&args, &JsValue::from_f64(1.0))
                .ok()
                .and_then(|value| value.as_string())
                .unwrap_or_default();
            let value = match raw.as_str() {
                "true" => apeireth_ui_kit::settings_character::CapabilityValue::Bool(true),
                "false" => apeireth_ui_kit::settings_character::CapabilityValue::Bool(false),
                other => apeireth_ui_kit::settings_character::CapabilityValue::Number(
                    other.parse::<f64>().unwrap_or_default(),
                ),
            };
            CHARACTER_APP.with(|slot| {
                let slot = slot.borrow();
                let Some(host) = slot.as_ref() else {
                    return;
                };
                host.set_capability(&key, value);
            });
            JsValue::UNDEFINED
        }) as Box<dyn FnMut(JsValue) -> JsValue>);

        let select_section = Closure::wrap(Box::new(|args: JsValue| -> JsValue {
            let id = args.as_string().unwrap_or_default();
            CHARACTER_APP.with(|slot| {
                let slot = slot.borrow();
                let Some(host) = slot.as_ref() else {
                    return;
                };
                host.select_section(&id);
            });
            JsValue::UNDEFINED
        }) as Box<dyn FnMut(JsValue) -> JsValue>);

        let mount_probe = Closure::wrap(Box::new(|args: JsValue| -> JsValue {
            let enabled = args.as_bool().unwrap_or(true);
            CHARACTER_APP.with(|slot| {
                let slot = slot.borrow();
                let Some(host) = slot.as_ref() else {
                    return;
                };
                host.set_probe_enabled(enabled);
            });
            JsValue::UNDEFINED
        }) as Box<dyn FnMut(JsValue) -> JsValue>);

        let get_state = Closure::wrap(Box::new(|_args: JsValue| -> JsValue {
            let state = CHARACTER_APP.with(|slot| {
                slot.borrow()
                    .as_ref()
                    .map(|host| host.state())
                    .unwrap_or_default()
            });
            JsValue::from_str(&serde_free_json(&state))
        }) as Box<dyn FnMut(JsValue) -> JsValue>);

        let api = js_sys::Object::new();
        let bind = |key: &str, closure: &Closure<dyn FnMut(JsValue) -> JsValue>| {
            let function: &js_sys::Function = closure.as_ref().unchecked_ref();
            let _ = Reflect::set(api.as_ref(), &JsValue::from_str(key), function.as_ref());
        };
        bind("setCapability", &set_capability);
        bind("mountProbe", &mount_probe);
        bind("selectSection", &select_section);
        bind("getState", &get_state);
        let _ = Reflect::set(
            api.as_ref(),
            &JsValue::from_str("ready"),
            &JsValue::from_bool(true),
        );
        let _ = Reflect::set(
            window.as_ref(),
            &JsValue::from_str("__CHARACTER__"),
            api.as_ref(),
        );

        CONTROL_CLOSURES.with(|slot| {
            let mut slot = slot.borrow_mut();
            slot.push(set_capability);
            slot.push(select_section);
            slot.push(mount_probe);
            slot.push(get_state);
        });
    }

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
    //! 宿主侧的**行为**断言（固定数据装配 + 状态转移 ⇒ 与生产同口径）。

    use super::*;
    use apeireth_ui_kit::settings_character::{
        cap_disabled, enabled_count, is_cap_on, CapabilityDef,
    };

    #[test]
    fn the_fixture_carries_two_personas_and_the_current_one() {
        let fixture = CharacterFixture::load().expect("固定数据可读");
        assert_eq!(fixture.personas.len(), 2);
        assert_eq!(fixture.personas[0].name, "阿佩瑞斯");
        assert_eq!(fixture.active_persona_id, "p-0001");
    }

    #[test]
    fn the_fixture_carries_three_ability_registries() {
        let fixture = CharacterFixture::load().expect("固定数据可读");
        assert_eq!(fixture.memory_flow_defs.len(), 5);
        assert_eq!(fixture.cognition_defs.len(), 6);
        assert_eq!(fixture.community_defs.len(), 2);
    }

    #[test]
    fn the_inverted_knob_is_marked_inverted() {
        let fixture = CharacterFixture::load().expect("固定数据可读");
        let typed = fixture
            .memory_flow_defs
            .iter()
            .find(|def| def.key == "typedRecall")
            .expect("有类型化召回");
        assert!(typed.invert, "它是**反向旋钮**");
        assert_eq!(typed.env, "APEIRETH_DISABLE_TYPED_RECALL");
    }

    #[test]
    fn the_nested_row_hangs_on_the_morphology_row() {
        let fixture = CharacterFixture::load().expect("固定数据可读");
        let morphology = fixture
            .cognition_defs
            .iter()
            .find(|def| def.key == "morphologyRecall")
            .expect("有检索深度自适应");
        let nested = morphology.nested.as_ref().expect("带嵌套行");
        assert_eq!(nested.value_key, "morphologyTemperature");
        assert_eq!(nested.env, "APEIRETH_MORPHOLOGY_TEMPERATURE");
    }

    #[test]
    fn the_inverted_knob_reads_the_env_semantics() {
        // `typedRecall = true` ⇒ 旋钮**关**（env 未被禁用 ⇒ 能力开着）。
        let fixture = CharacterFixture::load().expect("固定数据可读");
        let typed = fixture
            .memory_flow_defs
            .iter()
            .find(|def| def.key == "typedRecall")
            .expect("有类型化召回");
        assert!(
            !is_cap_on(typed, &fixture.capabilities),
            "反向旋钮：值 true ⇒ 关"
        );
        let mut table = fixture.capabilities.clone();
        table.insert(String::from("typedRecall"), CapabilityValue::Bool(false));
        assert!(is_cap_on(typed, &table), "反向旋钮：值 false ⇒ 开");
    }

    #[test]
    fn the_enabled_count_matches_the_production_number() {
        // 固定数据下：记忆流 2（类型化召回是反向旋钮 ⇒ 不计）+ 认知 1 + 社区 0 = 3。
        let fixture = CharacterFixture::load().expect("固定数据可读");
        let count = enabled_count(&fixture.memory_flow_defs, &fixture.capabilities)
            + enabled_count(&fixture.cognition_defs, &fixture.capabilities)
            + enabled_count(&fixture.community_defs, &fixture.capabilities);
        assert_eq!(count, 3, "与生产 DOM 里「本页已启用 3 / 13」一致");
    }

    #[test]
    fn a_dependent_row_is_disabled_when_the_parent_is_off() {
        let def = CapabilityDef::new(
            "child",
            &icons::ICON_INFO,
            "子行",
            "APEIRETH_CHILD",
            "子行说明",
        )
        .requires("parent");
        let mut table = CapabilityTable::new();
        table.insert(String::from("parent"), CapabilityValue::Bool(false));
        assert!(cap_disabled(&def, &table), "父行关 ⇒ 子行禁用");
        table.insert(String::from("parent"), CapabilityValue::Bool(true));
        assert!(!cap_disabled(&def, &table), "父行开 ⇒ 子行可用");
    }

    #[test]
    fn the_host_switches_sections_and_ignores_unknown_ones() {
        let fixture = CharacterFixture::load().expect("固定数据可读");
        let host = CharacterHost::new(fixture);
        assert_eq!(host.section().get(), "personality");
        assert!(host.select_section("cognition"));
        assert_eq!(host.section().get(), "cognition");
        assert!(!host.select_section("no-such-section"), "未知分区 ⇒ 不切");
        assert_eq!(host.section().get(), "cognition");
    }

    #[test]
    fn the_host_records_intents_instead_of_changing_state() {
        let fixture = CharacterFixture::load().expect("固定数据可读");
        let host = CharacterHost::new(fixture);
        let props = host.props();
        (props.on_toggle_capability)("memoryInjection");
        assert_eq!(host.last_intent(), "toggle-capability:memoryInjection");
        // 意图**不改**状态（提交是宿主侧的事）。
        assert_eq!(
            host.capability("memoryInjection"),
            Some(CapabilityValue::Bool(true))
        );
    }

    #[test]
    fn the_host_can_push_a_capability_to_a_given_state() {
        let fixture = CharacterFixture::load().expect("固定数据可读");
        let host = CharacterHost::new(fixture);
        host.set_capability("typedRecall", CapabilityValue::Bool(false));
        assert_eq!(
            host.capability("typedRecall"),
            Some(CapabilityValue::Bool(false))
        );
    }
}
