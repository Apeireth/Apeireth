//! 设置页三页 · `provider` 子面（**服务商与连接**）的组件模块。
//!
//! ## 本面是什么
//!
//! 生产 `SettingsView.svelte` 的「服务商与连接」面（markup 2019–2362 的
//! `models` 分区 + 3687–3749 的 **API Key 模态**，共 407 行）：
//!
//! | 区块 | 内容 | 生产行区间 |
//! |---|---|---|
//! | 摘要条 | 协议徽标 + 预设名 + 模型 + 状态药品 + 端点 | 2023–2033 |
//! | 生效回执 | 警告框（`.warnings-box`）+ 网关生效配置（`.effective-config`） | 2035–2055 |
//! | 步骤 1 | 协议页签（OpenAI 兼容 / Anthropic） | 2065–2105 |
//! | 步骤 2 | 预设芯片（按协议取 5 / 3 条） | 2107–2129 |
//! | 步骤 3 | 端点 / 模型 / 模型选择器占位 / 密钥（**明文切换**）/ 协议头 / 测连接 | 2131–2250 |
//! | 测连接回执 | 成功 / 失败 + 延迟 + 远端模型目录（搜索 + 前 24 项） | 2252–2302 |
//! | 高级网关 | 折叠头 + 网关地址 + 网关认证密钥（**打开模态**） | 2305–2359 |
//! | API Key 模态 | 钥匙串状态 + 新密钥输入 + 删除 / 取消 / 写入 | 3687–3749 |
//!
//! ## 本面成型的**四条硬口径**（判反 = 静默错误 / 判据必败）
//!
//! ### ① 明文切换是**结构分支**（只产出一个图标节点）
//!
//! 生产的
//!
//! ```text
//! {#if showApiKey}<EyeOff size={14} />{:else}<Eye size={14} />{/if}
//! ```
//!
//! ⇒ **一次只产出一个**图标节点（不是两个、不是零个）⇒ 见 [`render_key_toggle`]。
//! ⚠️ 同时 `type` 属性**跟着变**（`password` ⇔ `text`）⇒ 那是**属性分支**，
//! 不是 property（见 [`key_input_type`]）。
//!
//! ### ② 整组提交的**原子性**
//!
//! 生产的整组草稿（端点 / 模型 / 密钥 / 协议头）**一次提交**：
//! `submitProviderGroup()` 取 `providerGroupDraft()` 的**整组**快照 ⇒ 失败整组
//! 回填 ⇒ 「半截配置」永远到不了运行时（`settings-live-apply.ts` 的
//! `configWithProviderGroup` 也是整组写回）。
//!
//! ⇒ 本组件的提交出口是 [`GroupSubmitIntent`]（**入参就是整组草稿**）⇒
//! **引擎只发「提交这组」这一个意图**、且意图**自带整组** ⇒ 见 [`draft_of`]。
//!
//! ### ③ 密钥**打码口径**可迁、密钥本身**不迁**
//!
//! 生产 `maskApiKey`（`SettingsView.svelte:1430`）：
//!
//! ```text
//! const value = key.trim();
//! if (value.length <= 8) return '****';
//! return `${value.slice(0, 3)}****${value.slice(-3)}`;
//! ```
//!
//! ⇒ 逐字翻译为 [`mask_api_key`]（**长度按 UTF-16 计** ⇒ 用
//! [`crate::num::string_length`] / [`crate::num::string_slice`]）。
//! ⚠️ 本组件**只承接打码后的展示串**（[`ProviderGroupProps::stored_key_masked`]）
//! ⇒ **明文密钥不进虚拟树**。
//!
//! ### ④ `SessionModelPicker`（241 行**外部组件**）**不迁**
//!
//! ⇒ 本模块只产出它的**闭合态占位**（`div.model-picker > button.trigger` +
//! `span.value` + `span.chevron`，逐字照生产**闭合态** DOM）⇒ 样式由面级
//! manifest 抽出的 `model-picker.css` 提供（外部组件的 `<style>` 不在
//! `SettingsView.svelte` 里）。
//! ⇒ **下拉弹层 / 搜索 / 键选**未迁（见 [`render_model_picker_placeholder`]）。
//!
//! ## 诚实边界（**引擎只发意图，不假装已接线**）
//!
//! | 事项 | 口径 |
//! |---|---|
//! | 测连接（`testProviderConnection`） | **HTTP** ⇒ 不迁 ⇒ 只发 [`ProviderIntent`] |
//! | 模型发现（`listModels`） | **HTTP** ⇒ 不迁 ⇒ 只发意图 |
//! | 网关热应用（`applyAdminConfig` / `getAdminConfig`） | **HTTP** ⇒ 不迁；本组件只渲染宿主给的**回执** |
//! | 钥匙串（读 / 写 / 删） | **宿主能力**（系统钥匙串）⇒ 不迁 ⇒ 只发意图 |
//! | 整组落盘（`onSave`） | **宿主** ⇒ 只发「提交这组」 |
//! | 打码 | **展示口径**可迁（[`mask_api_key`]）；**明文密钥不迁** |
//! | `SessionModelPicker` | **不迁**（闭合态占位；下拉未迁移） |
//! | 「未保存 / ✓」字段旗标 | 复用 S1 的 [`FieldFlag`]（状态归宿主） |
//! | 原生控件的**取值** | 生产用 **property**（`bind:value`）⇒ 组件**不写** `value` 属性，由组件用 `prop("value", …)` **声明**、引擎写入 |
//!
//! ## 图标数据
//!
//! 14 个图标的几何数据是**构建期抽取**的生成物（上游图标包 ⇒ 真实 `.svelte`
//! ⇒ `iconNode`）⇒ 见 [`icons`] 模块（生成器：面级 manifest
//! `harness/settings/web/engine/provider/face.manifest.json`）。
//! class 后缀取**真实文件名**：`CheckCircle2` ⇒ `lucide-circle-check`、
//! `XCircle` ⇒ `lucide-circle-x`（生产 import 名 `XCircle` 是**别名**）。

use std::rc::Rc;

use crate::icon::{Icon, IconData, IconProps};
use crate::settings_frame::{FieldFlag, FieldFlagProps, FieldFlagState};
use apeireth_ui_core::{Signal, UiComponent};
use apeireth_ui_dom::{DomEvent, ElementHandler, EventKind, VNode};
use apeireth_ui_macro::view;

/// 本面用到的图标数据（**自动生成** —— 勿手改）。
///
/// 生成器：`crates-ui/apeireth-ui-demo/harness/scripts/extract-assets.mjs`
/// （**面 manifest 模式** ⇒ 清单：`harness/settings/web/engine/provider/face.manifest.json`）
/// ⇒ 来源：`frontend/companion-desktop/node_modules/lucide-svelte/dist/icons/*.svelte`。
///
/// 上游许可（ISC；其中 Feather 部分为 MIT）—— 原文如下：
///
/// ISC License
///
/// Copyright (c) for portions of Lucide are held by Cole Bemis 2013-2022 as part of Feather (MIT). All other copyright (c) for Lucide are held by Lucide Contributors 2022.
///
/// Permission to use, copy, modify, and/or distribute this software for any
/// purpose with or without fee is hereby granted, provided that the above
/// copyright notice and this permission notice appear in all copies.
///
/// THE SOFTWARE IS PROVIDED "AS IS" AND THE AUTHOR DISCLAIMS ALL WARRANTIES
/// WITH REGARD TO THIS SOFTWARE INCLUDING ALL IMPLIED WARRANTIES OF
/// MERCHANTABILITY AND FITNESS. IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR ANY
/// SPECIAL, DIRECT, INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES
/// WHATSOEVER RESULTING FROM LOSS OF USE, DATA OR PROFITS, WHETHER IN AN
/// ACTION OF CONTRACT, NEGLIGENCE OR OTHER TORTIOUS ACTION, ARISING OUT OF OR
/// IN CONNECTION WITH THE USE OR PERFORMANCE OF THIS SOFTWARE.
pub mod icons {
    use crate::icon::{IconData, IconNode};

    /// 图标 `server`。
    #[rustfmt::skip]
    pub const ICON_SERVER: IconData = IconData {
        name: "server",
        class: "lucide-icon lucide lucide-server",
        nodes: &[
            IconNode { tag: "rect", attrs: &[("width", "20"), ("height", "8"), ("x", "2"), ("y", "2"), ("rx", "2"), ("ry", "2")] },
            IconNode { tag: "rect", attrs: &[("width", "20"), ("height", "8"), ("x", "2"), ("y", "14"), ("rx", "2"), ("ry", "2")] },
            IconNode { tag: "line", attrs: &[("x1", "6"), ("x2", "6.01"), ("y1", "6"), ("y2", "6")] },
            IconNode { tag: "line", attrs: &[("x1", "6"), ("x2", "6.01"), ("y1", "18"), ("y2", "18")] },
        ],
    };

    /// 图标 `globe`。
    #[rustfmt::skip]
    pub const ICON_GLOBE: IconData = IconData {
        name: "globe",
        class: "lucide-icon lucide lucide-globe",
        nodes: &[
            IconNode { tag: "circle", attrs: &[("cx", "12"), ("cy", "12"), ("r", "10")] },
            IconNode { tag: "path", attrs: &[("d", "M12 2a14.5 14.5 0 0 0 0 20 14.5 14.5 0 0 0 0-20")] },
            IconNode { tag: "path", attrs: &[("d", "M2 12h20")] },
        ],
    };

    /// 图标 `sparkles`。
    #[rustfmt::skip]
    pub const ICON_SPARKLES: IconData = IconData {
        name: "sparkles",
        class: "lucide-icon lucide lucide-sparkles",
        nodes: &[
            IconNode { tag: "path", attrs: &[("d", "M9.937 15.5A2 2 0 0 0 8.5 14.063l-6.135-1.582a.5.5 0 0 1 0-.962L8.5 9.936A2 2 0 0 0 9.937 8.5l1.582-6.135a.5.5 0 0 1 .963 0L14.063 8.5A2 2 0 0 0 15.5 9.937l6.135 1.581a.5.5 0 0 1 0 .964L15.5 14.063a2 2 0 0 0-1.437 1.437l-1.582 6.135a.5.5 0 0 1-.963 0z")] },
            IconNode { tag: "path", attrs: &[("d", "M20 3v4")] },
            IconNode { tag: "path", attrs: &[("d", "M22 5h-4")] },
            IconNode { tag: "path", attrs: &[("d", "M4 17v2")] },
            IconNode { tag: "path", attrs: &[("d", "M5 18H3")] },
        ],
    };

    /// 图标 `eye`。
    #[rustfmt::skip]
    pub const ICON_EYE: IconData = IconData {
        name: "eye",
        class: "lucide-icon lucide lucide-eye",
        nodes: &[
            IconNode { tag: "path", attrs: &[("d", "M2.062 12.348a1 1 0 0 1 0-.696 10.75 10.75 0 0 1 19.876 0 1 1 0 0 1 0 .696 10.75 10.75 0 0 1-19.876 0")] },
            IconNode { tag: "circle", attrs: &[("cx", "12"), ("cy", "12"), ("r", "3")] },
        ],
    };

    /// 图标 `eye-off`。
    #[rustfmt::skip]
    pub const ICON_EYE_OFF: IconData = IconData {
        name: "eye-off",
        class: "lucide-icon lucide lucide-eye-off",
        nodes: &[
            IconNode { tag: "path", attrs: &[("d", "M10.733 5.076a10.744 10.744 0 0 1 11.205 6.575 1 1 0 0 1 0 .696 10.747 10.747 0 0 1-1.444 2.49")] },
            IconNode { tag: "path", attrs: &[("d", "M14.084 14.158a3 3 0 0 1-4.242-4.242")] },
            IconNode { tag: "path", attrs: &[("d", "M17.479 17.499a10.75 10.75 0 0 1-15.417-5.151 1 1 0 0 1 0-.696 10.75 10.75 0 0 1 4.446-5.143")] },
            IconNode { tag: "path", attrs: &[("d", "m2 2 20 20")] },
        ],
    };

    /// 图标 `lock`。
    #[rustfmt::skip]
    pub const ICON_LOCK: IconData = IconData {
        name: "lock",
        class: "lucide-icon lucide lucide-lock",
        nodes: &[
            IconNode { tag: "rect", attrs: &[("width", "18"), ("height", "11"), ("x", "3"), ("y", "11"), ("rx", "2"), ("ry", "2")] },
            IconNode { tag: "path", attrs: &[("d", "M7 11V7a5 5 0 0 1 10 0v4")] },
        ],
    };

    /// 图标 `check`。
    #[rustfmt::skip]
        pub const ICON_CHECK: IconData = IconData {
        name: "check",
        class: "lucide-icon lucide lucide-check",
        nodes: &[
            IconNode { tag: "path", attrs: &[("d", "M20 6 9 17l-5-5")] },
        ],
    };

    /// 图标 `circle-check`（生产 import 名 `CheckCircle2`）。
    #[rustfmt::skip]
    pub const ICON_CIRCLE_CHECK: IconData = IconData {
        name: "circle-check",
        class: "lucide-icon lucide lucide-circle-check",
        nodes: &[
            IconNode { tag: "circle", attrs: &[("cx", "12"), ("cy", "12"), ("r", "10")] },
            IconNode { tag: "path", attrs: &[("d", "m9 12 2 2 4-4")] },
        ],
    };

    /// 图标 `circle-x`（生产 import 名 `XCircle` ⇒ **别名** `x-circle`）。
    #[rustfmt::skip]
    pub const ICON_CIRCLE_X: IconData = IconData {
        name: "circle-x",
        class: "lucide-icon lucide lucide-circle-x",
        nodes: &[
            IconNode { tag: "circle", attrs: &[("cx", "12"), ("cy", "12"), ("r", "10")] },
            IconNode { tag: "path", attrs: &[("d", "m15 9-6 6")] },
            IconNode { tag: "path", attrs: &[("d", "m9 9 6 6")] },
        ],
    };

    /// 图标 `chevron-down`。
    #[rustfmt::skip]
        pub const ICON_CHEVRON_DOWN: IconData = IconData {
        name: "chevron-down",
        class: "lucide-icon lucide lucide-chevron-down",
        nodes: &[
            IconNode { tag: "path", attrs: &[("d", "m6 9 6 6 6-6")] },
        ],
    };

    /// 图标 `chevron-up`。
    #[rustfmt::skip]
        pub const ICON_CHEVRON_UP: IconData = IconData {
        name: "chevron-up",
        class: "lucide-icon lucide lucide-chevron-up",
        nodes: &[
            IconNode { tag: "path", attrs: &[("d", "m18 15-6-6-6 6")] },
        ],
    };

    /// 图标 `search`。
    #[rustfmt::skip]
    pub const ICON_SEARCH: IconData = IconData {
        name: "search",
        class: "lucide-icon lucide lucide-search",
        nodes: &[
            IconNode { tag: "circle", attrs: &[("cx", "11"), ("cy", "11"), ("r", "8")] },
            IconNode { tag: "path", attrs: &[("d", "m21 21-4.3-4.3")] },
        ],
    };

    /// 图标 `rotate-ccw`。
    #[rustfmt::skip]
    pub const ICON_ROTATE_CCW: IconData = IconData {
        name: "rotate-ccw",
        class: "lucide-icon lucide lucide-rotate-ccw",
        nodes: &[
            IconNode { tag: "path", attrs: &[("d", "M3 12a9 9 0 1 0 9-9 9.75 9.75 0 0 0-6.74 2.74L3 8")] },
            IconNode { tag: "path", attrs: &[("d", "M3 3v5h5")] },
        ],
    };

    /// 图标 `trash-2`。
    #[rustfmt::skip]
    pub const ICON_TRASH_2: IconData = IconData {
        name: "trash-2",
        class: "lucide-icon lucide lucide-trash-2",
        nodes: &[
            IconNode { tag: "path", attrs: &[("d", "M3 6h18")] },
            IconNode { tag: "path", attrs: &[("d", "M19 6v14c0 1-1 2-2 2H7c-1 0-2-1-2-2V6")] },
            IconNode { tag: "path", attrs: &[("d", "M8 6V4c0-1 1-2 2-2h4c1 0 2 1 2 2v2")] },
            IconNode { tag: "line", attrs: &[("x1", "10"), ("x2", "10"), ("y1", "11"), ("y2", "17")] },
            IconNode { tag: "line", attrs: &[("x1", "14"), ("x2", "14"), ("y1", "11"), ("y2", "17")] },
        ],
    };

    /// 按**生产 import 名**取图标数据（表驱动 ⇒ 组件里没有逐行硬编码）。
    ///
    /// ⚠️ **已知改写**（生产 import 名 ⇒ 真实文件 ⇒ 真实 class）：
    ///
    /// | 生产 import 名 | 真实文件 | 渲染出的 class |
    /// |---|---|---|
    /// | `CheckCircle2` | `circle-check.svelte` | `lucide-circle-check` |
    /// | `XCircle` | `circle-x.svelte` | `lucide-circle-x` |
    /// | `Trash2` | `trash-2.svelte` | `lucide-trash-2` |
    /// | `RotateCcw` | `rotate-ccw.svelte` | `lucide-rotate-ccw` |
    ///
    /// 未知名字 ⇒ `None`（**显式失败** ⇒ 宿主据此报错，不静默画空图标）。
    pub fn icon_by_import_name(name: &str) -> Option<&'static IconData> {
        match name {
            "Server" => Some(&ICON_SERVER),
            "Globe" => Some(&ICON_GLOBE),
            "Sparkles" => Some(&ICON_SPARKLES),
            "Eye" => Some(&ICON_EYE),
            "EyeOff" => Some(&ICON_EYE_OFF),
            "Lock" => Some(&ICON_LOCK),
            "Check" => Some(&ICON_CHECK),
            "CheckCircle2" => Some(&ICON_CIRCLE_CHECK),
            "XCircle" => Some(&ICON_CIRCLE_X),
            "ChevronDown" => Some(&ICON_CHEVRON_DOWN),
            "ChevronUp" => Some(&ICON_CHEVRON_UP),
            "Search" => Some(&ICON_SEARCH),
            "RotateCcw" => Some(&ICON_ROTATE_CCW),
            "Trash2" => Some(&ICON_TRASH_2),
            _ => None,
        }
    }
}

// ── 文案常量（**生产 markup 逐字**） ────────────────────────────────────────
//
// 每条都对着生产 markup 抄（`SettingsView.svelte` 的 2019–2362 / 3687–3749）。
// ⚠️ **尾随空格是数据**：`{@render fieldFlag(...)}` 之前的那个空格属于
// **文本节点**（生产 DOM 实测：`"API 端点 (Base URL) "`）⇒ 不可省。

/// 本面 `models` 分区的标题。
pub const MODELS_TITLE: &str = "模型与提供商";

/// 本面 `models` 分区的说明。
pub const MODELS_DESC: &str =
    "配置 LLM 协议、服务商端点与活动模型；与 Dock 模型选择器共用同一配置源。";

/// 协议徽标的 OpenAI 文案。
pub const PROTOCOL_LABEL_OPENAI: &str = "OpenAI 兼容";

/// 协议徽标的 Anthropic 文案。
pub const PROTOCOL_LABEL_ANTHROPIC: &str = "Anthropic";

/// 当前预设查不到时的名字。
pub const CUSTOM_PRESET_NAME: &str = "自定义";

/// 模型为空时的摘要文案。
pub const NO_MODEL_TEXT: &str = "未指定模型";

/// 端点为空的摘要文案（**生产是一个 em dash**）。
pub const NO_URL_TEXT: &str = "—";

/// 连接状态的「已连通」。
pub const STATUS_CONNECTED: &str = "已连通";

/// 连接状态的「连接失败」。
pub const STATUS_FAILED: &str = "连接失败";

/// 连接状态的「待测试」。
pub const STATUS_UNTESTED: &str = "待测试";

/// 连接状态的「未配置密钥」。
pub const STATUS_NO_KEY: &str = "未配置密钥";

/// 步骤 1 的序号。
pub const STEP_ONE: &str = "1";

/// 步骤 2 的序号。
pub const STEP_TWO: &str = "2";

/// 步骤 3 的序号。
pub const STEP_THREE: &str = "3";

/// 步骤 1 的标题。
pub const PROTOCOL_STEP_TITLE: &str = "选择协议";

/// 步骤 2 的标题。
pub const PRESET_STEP_TITLE: &str = "选择服务商预设";

/// 步骤 3 的标题。
pub const FIELDS_STEP_TITLE: &str = "端点、密钥与模型";

/// OpenAI 协议的页签标题。
pub const PROTO_TITLE_OPENAI: &str = "OpenAI 兼容协议";

/// OpenAI 协议的页签副文案。
pub const PROTO_SUB_OPENAI: &str = "OpenAI · DeepSeek · MiniMax · Ollama · vLLM";

/// Anthropic 协议的页签标题。
pub const PROTO_TITLE_ANTHROPIC: &str = "Anthropic Claude 协议";

/// Anthropic 协议的页签副文案。
pub const PROTO_SUB_ANTHROPIC: &str = "Messages API · Claude 3.5 / 3.7";

/// 端点标签（**尾随空格**：字段旗标跟在后面）。
pub const URL_LABEL: &str = "API 端点 (Base URL) ";

/// 模型标签（**尾随空格**）。
pub const MODEL_LABEL: &str = "活动模型 ID ";

/// OpenAI 的端点占位。
pub const URL_PLACEHOLDER_OPENAI: &str = "https://api.openai.com/v1";

/// Anthropic 的端点占位。
pub const URL_PLACEHOLDER_ANTHROPIC: &str = "https://api.anthropic.com";

/// OpenAI 的模型占位。
pub const MODEL_PLACEHOLDER_OPENAI: &str = "gpt-4o";

/// Anthropic 的模型占位。
pub const MODEL_PLACEHOLDER_ANTHROPIC: &str = "claude-3-7-sonnet-20250219";

/// 模型选择器一组的分组标签。
pub const GROUP_LABEL_MODELS: &str = "从 /v1/models 选择模型";

/// 「重新加载」按钮文案。
pub const RELOAD_LABEL: &str = "重新加载";

/// 「加载中」按钮文案。
pub const LOADING_LABEL: &str = "加载中…";

/// 模型选择器的默认模型（生产 `DEFAULT_MODEL_ID`）。
pub const DEFAULT_MODEL_ID: &str = "deepseek-v4-flash";

/// 模型选择器的提示（**生产文案里的默认模型名是写死的**）。
pub const DEFAULT_MODEL_HINT: &str = "默认模型 deepseek-v4-flash；列表来自网关 /v1/models。";

/// 密钥标签（OpenAI 协议；**尾随空格**）。
pub const KEY_LABEL_OPENAI: &str = "API Key (Bearer) ";

/// 密钥标签（Anthropic 协议；**尾随空格**）。
pub const KEY_LABEL_ANTHROPIC: &str = "API Key (x-api-key) ";

/// OpenAI 的密钥占位。
pub const KEY_PLACEHOLDER_OPENAI: &str = "sk-...";

/// Anthropic 的密钥占位。
pub const KEY_PLACEHOLDER_ANTHROPIC: &str = "sk-ant-...";

/// 明文切换按钮的「显示」提示。
pub const SHOW_KEY_TITLE: &str = "显示密钥";

/// 明文切换按钮的「隐藏」提示。
pub const HIDE_KEY_TITLE: &str = "隐藏密钥";

/// 协议头标签（**尾随空格**）。
pub const ANTHROPIC_VERSION_LABEL: &str = "anthropic-version ";

/// 协议头占位。
pub const ANTHROPIC_VERSION_PLACEHOLDER: &str = "2023-06-01";

/// 测连接按钮文案。
pub const TEST_CONNECTION_LABEL: &str = "测试连接并拉取模型";

/// 测连接进行中的文案。
pub const TESTING_CONNECTION_LABEL: &str = "探测中…";

/// 预设推荐的旁注。
pub const RECOMMENDED_LABEL: &str = "预设推荐";

/// 测连接成功标题。
pub const TEST_OK_TITLE: &str = "连接成功";

/// 测连接失败标题。
pub const TEST_FAIL_TITLE: &str = "连接失败";

/// 远端模型计数的前缀（`远端模型 (`）。
pub const REMOTE_MODELS_PREFIX: &str = "远端模型 (";

/// 远端模型计数的后缀（`)`）。
pub const REMOTE_MODELS_SUFFIX: &str = ")";

/// 远端模型搜索的占位。
pub const MODEL_SEARCH_PLACEHOLDER: &str = "筛选模型…";

/// 模型目录「无匹配」的文案。
pub const CATALOG_EMPTY_TEXT: &str = "无匹配模型";

/// 模型目录截断的提示。
pub const MORE_MODELS_TEXT: &str = "仅显示前 24 项，请用搜索缩小范围";

/// 模型目录一次最多展示的条数（生产 `.slice(0, 24)`）。
pub const MODEL_CATALOG_LIMIT: usize = 24;

/// 高级网关折叠头文案。
pub const ADVANCED_GATEWAY_LABEL: &str = "Apeireth 核心网关与守护进程 (Advanced Gateway)";

/// 网关地址标签（**尾随空格**）。
pub const GATEWAY_URL_LABEL: &str = "网关服务地址 (Gateway URL) ";

/// 网关地址占位。
pub const GATEWAY_URL_PLACEHOLDER: &str = "http://127.0.0.1:8080";

/// 网关地址的提示。
pub const GATEWAY_URL_HINT: &str =
    "默认为 Apeireth 核心网关端口 (:8080)；失焦或回车提交，失败回填旧值。";

/// 网关认证密钥的标签。
pub const GATEWAY_KEY_LABEL: &str = "网关认证密钥 (Gateway Auth Key)";

/// 网关密钥「已配置」。
pub const GATEWAY_KEY_CONFIGURED: &str = "已配置 (Configured)";

/// 网关密钥「未配置」。
pub const GATEWAY_KEY_NOT_CONFIGURED: &str = "未配置 (Not configured)";

/// 网关密钥按钮的「更换」文案。
pub const CHANGE_KEY_LABEL: &str = "更换 Key";

/// 网关密钥按钮的「配置」文案。
pub const CONFIGURE_KEY_LABEL: &str = "配置 Key";

/// 网关密钥的提示。
pub const GATEWAY_KEY_HINT: &str = "Apeireth 网关管理认证密钥（可选）。";

/// 警告框标题的前缀（后面接条数 + ` 条警告：`）。
pub const WARNINGS_PREFIX: &str = "配置已应用，但有 ";

/// 警告框标题的后缀。
pub const WARNINGS_SUFFIX: &str = " 条警告：";

/// 生效配置卡的标题。
pub const EFFECTIVE_TITLE: &str = "网关生效配置 (无重启热应用)";

/// 生效配置的行标签（Provider）。
pub const EFFECTIVE_PROVIDER_LABEL: &str = "Provider: ";

/// 生效配置的行标签（Base URL）。
pub const EFFECTIVE_BASE_URL_LABEL: &str = "Base URL: ";

/// 生效配置的行标签（Model）。
pub const EFFECTIVE_MODEL_LABEL: &str = "Model: ";

/// 生效配置的行标签（API Key）。
pub const EFFECTIVE_API_KEY_LABEL: &str = "API Key: ";

/// 生效配置缺字段时的占位。
pub const EFFECTIVE_MISSING: &str = "—";

/// 生效配置的密钥缺省文案。
pub const EFFECTIVE_NO_KEY: &str = "未配置";

/// 模态标题。
pub const MODAL_TITLE: &str = "配置模型 API 密钥";

/// 模态说明（生产的文本**含字面的 `**` 强调** ⇒ 逐字照抄）。
pub const MODAL_DESC: &str =
    "这是**模型提供商**的密钥（如 DeepSeek）。密钥存入系统钥匙串，不落盘明文；写入后热更新，无需重启。";

/// 钥匙串读取中的文案。
pub const KEYCHAIN_LOADING_TEXT: &str = "正在读取系统钥匙串…";

/// 钥匙串为空的文案。
pub const KEYCHAIN_EMPTY_TEXT: &str = "系统钥匙串中暂无该提供商的密钥";

/// 钥匙串已存密钥的后缀。
pub const KEYCHAIN_STORED_SUFFIX: &str = " (已存钥匙串)";

/// 模态里新密钥输入的占位。
pub const MODAL_KEY_PLACEHOLDER: &str = "输入新的 API Key（例如 sk-…）";

/// 「删除已存密钥」按钮文案。
pub const DELETE_KEY_LABEL: &str = "删除已存密钥";

/// 「删除中」文案。
pub const DELETING_KEY_LABEL: &str = "删除中…";

/// 「删除已存密钥」的悬浮提示。
pub const DELETE_KEY_TITLE: &str = "删除已存密钥（需二次确认）";

/// 「取消」按钮文案。
pub const CANCEL_LABEL: &str = "取消";

/// 「写入并应用」按钮文案。
pub const SAVE_KEY_LABEL: &str = "写入并应用";

/// 「写入中」文案。
pub const SAVING_KEY_LABEL: &str = "写入中…";

/// 服务商组的**文本键表**（生产 `PROVIDER_TEXT_KEYS` ⇒ 逗号分隔的载荷）。
///
/// ⚠️ 它是**一个字段键列表**，不是结构：生产用它做 `markTextDirty` 的批量标记
/// （`protocol` 切换 ⇒ 整组四个键都标「未保存」）。
pub const PROVIDER_TEXT_KEYS: &str =
    "provider.baseUrl,provider.model,provider.apiKey,provider.anthropicVersion";

/// 选预设时要标「未保存」的键（生产 `markProviderDirty(['provider.baseUrl', 'provider.model'])`）。
pub const PRESET_DIRTY_KEYS: &str = "provider.baseUrl,provider.model";

/// 分区的稳定 id：`models`。
pub const SECTION_MODELS: &str = "models";

/// 分区的稳定 id：API Key 模态（**整视口**截图）。
pub const SECTION_API_KEY_MODAL: &str = "api-key-modal";

// ── 数据形状（**宿主注入** ⇒ 组件不碰数据面） ───────────────────────────────

/// 一条服务商预设（**表的一行** ⇒ 数据，不是代码）。
///
/// 生产是 `OPENAI_PRESETS` / `ANTHROPIC_PRESETS` 两组常量
/// （`SettingsView.svelte:782-842`）⇒ 本组件**不认识任何具体预设** ⇒ 表由宿主喂。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ProviderPreset {
    /// 预设 id（`openai` / `deepseek` / `minimax` / `ollama` / `custom` …）。
    pub id: String,
    /// 展示名。
    pub name: String,
    /// 端点。
    pub base_url: String,
    /// 默认模型。
    pub default_model: String,
    /// 预设推荐的模型（**顺序即渲染顺序**）。
    pub models: Vec<String>,
}

/// 按协议分组的预设表（生产两份常量 ⇒ 宿主持有）。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ProviderPresets {
    /// OpenAI 兼容协议的预设（生产 5 条）。
    pub openai: Vec<ProviderPreset>,
    /// Anthropic 协议的预设（生产 3 条）。
    pub anthropic: Vec<ProviderPreset>,
}

impl ProviderPresets {
    /// 按协议取预设表。
    ///
    /// ⚠️ **未知协议 ⇒ 取 OpenAI 那份**（生产 `activeProtocol === 'openai' ? … : …`
    /// 只有两条分支 ⇒ 非 `openai` 即 Anthropic；本方法按「显式 `anthropic` ⇒
    /// Anthropic，其余 ⇒ OpenAI」判定 ⇒ 与生产的两分法同构）。
    pub fn for_protocol(&self, protocol: &str) -> &[ProviderPreset] {
        if protocol == "anthropic" {
            &self.anthropic
        } else {
            &self.openai
        }
    }

    /// 取一条预设。
    pub fn find(&self, protocol: &str, preset_id: &str) -> Option<&ProviderPreset> {
        self.for_protocol(protocol)
            .iter()
            .find(|item| item.id == preset_id)
    }

    /// 当前生效的预设对象（生产 `currentPresetObj`）。
    ///
    /// ## 口径（逐字照抄生产）
    ///
    /// ```text
    /// currentPresets.find((p) => p.id === activePreset) || currentPresets[currentPresets.length - 1]
    /// ```
    ///
    /// ⇒ **查不到 ⇒ 取最后一条**（不是 `None`）⇒ 摘要条因此总能显示一个名字。
    pub fn active(&self, protocol: &str, preset_id: &str) -> Option<&ProviderPreset> {
        let table = self.for_protocol(protocol);
        table
            .iter()
            .find(|item| item.id == preset_id)
            .or_else(|| table.last())
    }

    /// 预设推荐的模型（生产 `recommendedModels`）。
    pub fn recommended(&self, protocol: &str, preset_id: &str) -> &[String] {
        self.active(protocol, preset_id)
            .map(|item| item.models.as_slice())
            .unwrap_or(&[])
    }
}

/// 一条模型发现结果（`discoveredModels[]`）。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DiscoveredModel {
    /// 模型 id。
    pub id: String,
    /// 归属方（`ownedBy`；空 ⇒ 不展示）。
    pub owned_by: String,
}

/// 测连接的回执（宿主填；`None` ⇒ 还没测过）。
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TestResult {
    /// 是否连通。
    pub ok: bool,
    /// 回执文案。
    pub message: String,
    /// 延迟（毫秒；`None` ⇒ 不渲染延迟徽标）。
    pub latency_ms: Option<f64>,
    /// 远端模型 id 列表（**字符串数组** ⇒ 与生产的 `testResult.models` 同形）。
    pub models: Vec<String>,
}

/// 网关热应用的回执（宿主填；`None` ⇒ 不渲染任何回执区块）。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ApplyResult {
    /// 是否成功。
    pub ok: bool,
    /// 警告（**非空才渲染 `.warnings-box`**）。
    pub warnings: Vec<String>,
}

/// 网关生效配置的回显（宿主填；字段缺 ⇒ 展示 `—`）。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EffectiveConfig {
    /// `provider`。
    pub provider: Option<String>,
    /// `base_url`。
    pub base_url: Option<String>,
    /// `api_key`（**打码后的展示串** ⇒ 明文密钥永不进虚拟树）。
    pub api_key: Option<String>,
    /// `model`。
    pub model: Option<String>,
}

impl EffectiveConfig {
    /// 按生产的 fallback 口径取一个字段（缺 ⇒ `fallback`）。
    fn shown(value: &Option<String>, fallback: &'static str) -> String {
        value.clone().unwrap_or_else(|| String::from(fallback))
    }
}

/// 服务商组的**整组草稿**（端点 / 模型 / 密钥 / 协议头 ⇒ **一个提交原子**）。
///
/// 生产 `ProviderGroupDraft`（`settings-live-apply.ts:47`）的逐字对应。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ProviderGroupDraft {
    /// 协议。
    pub protocol: String,
    /// 预设 id。
    pub preset: String,
    /// 端点。
    pub base_url: String,
    /// 密钥（**明文只活在宿主里**；本结构体只作为**提交载荷**由组件交还宿主）。
    ///
    /// ⚠️ 组件**从不渲染**这个字段 ⇒ 打码串走
    /// [`ProviderGroupProps::stored_key_masked`]。
    pub api_key: String,
    /// 模型。
    pub model: String,
    /// 协议头（`anthropic-version`）。
    pub anthropic_version: String,
}

impl ProviderGroupDraft {
    /// 整组快照（生产 `providerGroupSnapshot`：**逐字段 trim 后比较**）。
    ///
    /// 用途：宿主判「空提交」（值没变 ⇒ 只清「未保存」标记、不打扰运行时）。
    /// ⇒ **组件不判**（提交与否是宿主的事）⇒ 本方法只提供同口径的快照文本。
    pub fn snapshot(&self) -> String {
        format!(
            "{{\"protocol\":{},\"preset\":{},\"baseUrl\":{},\"apiKey\":{},\"model\":{},\"anthropicVersion\":{}}}",
            json_string(&self.protocol),
            json_string(&self.preset),
            json_string(self.base_url.trim()),
            json_string(self.api_key.trim()),
            json_string(self.model.trim()),
            json_string(self.anthropic_version.trim()),
        )
    }

    /// 端点（trim 后）。
    pub fn trimmed_base_url(&self) -> &str {
        self.base_url.trim()
    }

    /// 模型（trim 后）。
    pub fn trimmed_model(&self) -> &str {
        self.model.trim()
    }

    /// 协议头（trim 后）。
    pub fn trimmed_anthropic_version(&self) -> &str {
        self.anthropic_version.trim()
    }
}

/// 一个字符串字面量的 JSON 转义（**只处理引号 / 反斜杠 / 控制符**）。
fn json_string(text: &str) -> String {
    let mut out = String::from("\"");
    for ch in text.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            other => out.push(other),
        }
    }
    out.push('"');
    out
}

// ── 判定（**逐字照抄生产的纯函数**） ────────────────────────────────────────

/// 密钥打码（生产 `maskApiKey` 的逐字翻译）。
///
/// ```text
/// const value = key.trim();
/// if (value.length <= 8) return '****';
/// return `${value.slice(0, 3)}****${value.slice(-3)}`;
/// ```
///
/// ⚠️ **长度与切片都按 UTF-16 计**（脚本侧语义）⇒ 用
/// [`crate::num::string_length`] / [`crate::num::string_slice`]，
/// **不要**用 `str::len()` / 字节切片（含 emoji 的密钥会错）。
///
/// ## 例
///
/// | 入参 | 出参 |
/// |---|---|
/// | `""` | `****` |
/// | `"12345678"`（8 位） | `****` |
/// | `"123456789"`（9 位） | `123****789` |
/// | `"  abcdefghij  "` | `abc****hij`（先 trim） |
pub fn mask_api_key(key: &str) -> String {
    // JS 的 `String.prototype.trim()` 去的是 Unicode 空白 ⇒ `str::trim` 同口径
    // （Rust 的 `trim` 按 `char::is_whitespace` ⇒ 与 ECMAScript 的 WhiteSpace +
    // LineTerminator 集合在本仓数据下等价）。
    let value = key.trim();
    let length = crate::num::string_length(value);
    if length <= 8 {
        return String::from("****");
    }
    let head = crate::num::string_slice(value, 0, 3);
    let tail = crate::num::string_slice(value, length - 3, length);
    format!("{head}****{tail}")
}

/// 协议徽标文案（生产 `activeProtocol === 'openai' ? 'OpenAI 兼容' : 'Anthropic'`）。
pub fn protocol_badge(protocol: &str) -> &'static str {
    if protocol == "openai" {
        PROTOCOL_LABEL_OPENAI
    } else {
        PROTOCOL_LABEL_ANTHROPIC
    }
}

/// 端点占位（随协议变）。
pub fn url_placeholder(protocol: &str) -> &'static str {
    if protocol == "openai" {
        URL_PLACEHOLDER_OPENAI
    } else {
        URL_PLACEHOLDER_ANTHROPIC
    }
}

/// 模型占位（随协议变）。
pub fn model_placeholder(protocol: &str) -> &'static str {
    if protocol == "openai" {
        MODEL_PLACEHOLDER_OPENAI
    } else {
        MODEL_PLACEHOLDER_ANTHROPIC
    }
}

/// 密钥标签（随协议变；**尾随空格**）。
pub fn key_label(protocol: &str) -> &'static str {
    if protocol == "openai" {
        KEY_LABEL_OPENAI
    } else {
        KEY_LABEL_ANTHROPIC
    }
}

/// 密钥占位（随协议变）。
pub fn key_placeholder(protocol: &str) -> &'static str {
    if protocol == "openai" {
        KEY_PLACEHOLDER_OPENAI
    } else {
        KEY_PLACEHOLDER_ANTHROPIC
    }
}

/// 密钥输入的 `type` 属性（**明文切换 = 属性分支**）。
///
/// 生产 `type={showApiKey ? 'text' : 'password'}` ⇒ 本函数是**唯一**判定入口。
pub fn key_input_type(show_api_key: bool) -> &'static str {
    if show_api_key {
        "text"
    } else {
        "password"
    }
}

/// 连接状态药品的文案（生产 `providerStatusLabel` 的四条分支）。
///
/// ```text
/// testResult?.ok ? '已连通'
///   : testResult && !testResult.ok ? '连接失败'
///   : providerApiKey.trim() ? '待测试'
///   : '未配置密钥'
/// ```
pub fn status_label(test_result: Option<&TestResult>, api_key: &str) -> &'static str {
    match test_result {
        Some(result) if result.ok => STATUS_CONNECTED,
        Some(_) => STATUS_FAILED,
        None if !api_key.trim().is_empty() => STATUS_UNTESTED,
        None => STATUS_NO_KEY,
    }
}

/// 状态药品的**条件 class**（生产 `class:ok` / `class:warn` / `class:idle`）。
///
/// ## 口径（三条互斥且**必须逐条照抄**）
///
/// ```text
/// class:ok   = testResult?.ok            // null ⇒ false
/// class:warn = !testResult?.ok && testResult   // null ⇒ false（且值不是对象）
/// class:idle = !testResult               // null ⇒ true
/// ```
pub fn status_classes(test_result: Option<&TestResult>) -> Vec<&'static str> {
    let mut classes = vec!["status-pill"];
    match test_result {
        Some(result) if result.ok => classes.push("ok"),
        Some(_) => classes.push("warn"),
        None => classes.push("idle"),
    }
    classes
}

/// 钥匙串 / env family 标识（生产 `providerFamily`）。
///
/// ```text
/// activeProtocol === 'anthropic' ? 'anthropic'
///   : activePreset === 'minimax' ? 'minimax'
///   : 'openai'
/// ```
pub fn provider_family(protocol: &str, preset: &str) -> &'static str {
    if protocol == "anthropic" {
        "anthropic"
    } else if preset == "minimax" {
        "minimax"
    } else {
        "openai"
    }
}

/// 模型搜索过滤（生产 `filteredDiscoveredModels`）。
///
/// ```text
/// const q = modelSearchQuery.trim().toLowerCase();
/// if (!q) return list;
/// return list.filter((m) => m.toLowerCase().includes(q));
/// ```
///
/// ⚠️ **`toLowerCase` 是语言无关的简单折叠**（不是 `toLocaleLowerCase`）⇒
/// Rust 侧用 `to_lowercase`（对 ASCII / 常用拉丁字母同结果）。
pub fn filter_models<'a>(models: &'a [String], query: &str) -> Vec<&'a str> {
    let needle = query.trim().to_lowercase();
    if needle.is_empty() {
        return models.iter().map(String::as_str).collect();
    }
    models
        .iter()
        .filter(|item| item.to_lowercase().contains(&needle))
        .map(String::as_str)
        .collect()
}

/// 模型目录的**展示切片**（生产 `.slice(0, 24)`）。
pub fn catalog_slice(models: &[&str]) -> Vec<String> {
    models
        .iter()
        .take(MODEL_CATALOG_LIMIT)
        .map(|item| String::from(*item))
        .collect()
}

/// 模型目录是否**被截断**（生产 `filteredDiscoveredModels.length > 24`）。
pub fn catalog_truncated(models: &[&str]) -> bool {
    models.len() > MODEL_CATALOG_LIMIT
}

/// 选预设时的模型联动（生产 `selectPreset` 的模型分支）。
///
/// ## 口径（逐字照抄生产，**含一个反直觉的「写死模型名」判据**）
///
/// ```text
/// if (p && p.id !== 'custom') {
///   providerBaseUrl = p.baseUrl;
///   if (p.defaultModel && (!providerModel || p.models.includes(providerModel)
///       || providerModel === 'gpt-4o' || providerModel === 'MiniMax-M3'
///       || providerModel === 'claude-3-7-sonnet-20250219')) {
///     providerModel = p.defaultModel;
///   }
/// }
/// ```
///
/// ⇒ 返回值 = **新的模型取值**（`None` ⇒ 模型不动）。
///
/// ⚠️ 那三个**写死的模型名**是生产原样（跨协议的「自家默认值」）⇒ 本函数照抄
/// ⇒ **不要**"优化"成「当前模型是否属于旧预设」。
pub fn preset_model_after_select(preset: &ProviderPreset, current_model: &str) -> Option<String> {
    if preset.id == "custom" {
        return None;
    }
    if preset.default_model.is_empty() {
        return None;
    }
    let current_is_known_default = current_model.is_empty()
        || preset.models.iter().any(|item| item == current_model)
        || current_model == "gpt-4o"
        || current_model == "MiniMax-M3"
        || current_model == "claude-3-7-sonnet-20250219";
    if current_is_known_default {
        Some(preset.default_model.clone())
    } else {
        None
    }
}

// ── 意图出口（**宿主注入** ⇒ 组件不自己改状态、不发请求） ───────────────────

/// 一般的「动作」意图（入参 = 动作名）。
pub type ProviderIntent = Rc<dyn Fn(&str)>;

/// **整组提交**的意图（入参 = 组名 + **整组草稿**）。
///
/// ## 为什么载荷是整组
///
/// 生产 `submitProviderGroup()` 一次取**整组**草稿（端点 / 模型 / 密钥 /
/// 协议头）⇒ 失败整组回填 ⇒ 「半截配置」到不了运行时。
/// ⇒ 引擎侧把「整组」放进**一个**意图里（判据 T5：一次意图含整组）。
pub type GroupSubmitIntent = Rc<dyn Fn(&str, &ProviderGroupDraft)>;

/// 读一个字段旗标（**查不到 ⇒ 无旗标**）。
fn flag_of(flags: &[(String, FieldFlagState)], key: &str) -> FieldFlagState {
    flags
        .iter()
        .find(|(name, _)| name == key)
        .map(|(_, state)| *state)
        .unwrap_or(FieldFlagState::None)
}

// ── 组件 ────────────────────────────────────────────────────────────────────

/// 服务商与连接面的 props（**宿主注入** ⇒ 组件是纯函数）。
#[derive(Clone)]
pub struct ProviderGroupProps {
    /// 协议（`openai` / `anthropic`）。
    pub protocol: Signal<String>,
    /// 预设 id。
    pub preset: Signal<String>,
    /// 端点。
    pub base_url: Signal<String>,
    /// 密钥（**明文只用于提交载荷与输入框回显** ⇒ 打码串走 `stored_key_masked`）。
    pub api_key: Signal<String>,
    /// 活动模型。
    pub model: Signal<String>,
    /// 协议头（`anthropic-version`）。
    pub anthropic_version: Signal<String>,
    /// 是否明文展示密钥（**结构分支** ⇒ 一个图标节点 + `type` 属性）。
    pub show_api_key: Signal<bool>,
    /// 模型发现列表（喂模型选择器占位）。
    pub discovered_models: Signal<Vec<DiscoveredModel>>,
    /// 模型列表读取中。
    pub models_loading: Signal<bool>,
    /// 模型列表的错误文案（空 ⇒ 不渲染错误行）。
    pub models_error: Signal<String>,
    /// 模型目录的搜索词（**受控输入** ⇒ 宿主回读 DOM）。
    pub model_search_query: Signal<String>,
    /// 测连接的回执（`None` ⇒ 还没测过）。
    pub test_result: Signal<Option<TestResult>>,
    /// 测连接进行中。
    pub testing: Signal<bool>,
    /// 网关热应用的回执（`None` ⇒ 不渲染警告框）。
    pub apply_result: Signal<Option<ApplyResult>>,
    /// 网关生效配置（`None` ⇒ 不渲染生效配置卡）。
    pub effective_config: Signal<Option<EffectiveConfig>>,
    /// 网关服务地址的草稿。
    pub edit_base_url: Signal<String>,
    /// 高级网关是否展开。
    pub show_advanced_gateway: Signal<bool>,
    /// 网关是否已配置认证密钥（**生产 `hasApiKey`**）。
    pub has_api_key: bool,
    /// 钥匙串里的密钥（**已打码** ⇒ 明文永不进虚拟树）。
    pub stored_key_masked: Signal<String>,
    /// 钥匙串里是否有该提供商的密钥。
    pub stored_key_exists: Signal<bool>,
    /// 钥匙串读取中。
    pub keychain_loading: Signal<bool>,
    /// 钥匙串写入中。
    pub keychain_saving: Signal<bool>,
    /// 钥匙串删除中。
    pub keychain_deleting: Signal<bool>,
    /// 钥匙串动作的错误文案（空 ⇒ 不渲染）。
    pub keychain_action_error: Signal<String>,
    /// 模态里的新密钥草稿（**persistent 输入** ⇒ 宿主回读 DOM）。
    pub temp_api_key: Signal<String>,
    /// 模态是否打开。
    pub show_api_key_modal: Signal<bool>,
    /// 字段旗标（键 → 状态；**宿主持有** ⇒ 组件只渲染）。
    pub text_flags: Signal<Vec<(String, FieldFlagState)>>,
    /// 预设表（**数据** ⇒ 两组）。
    pub presets: ProviderPresets,
    /// 一般动作的意图出口。
    pub on_action: ProviderIntent,
    /// 「字段改动」意图出口（入参 = 字段键；**宿主回读 DOM** 后标「未保存」）。
    pub on_field_input: ProviderIntent,
    /// 「只标未保存」意图出口（入参 = 字段键表）。
    ///
    /// ## 为什么要与 [`Self::on_field_input`] 分开
    ///
    /// 生产的**点选类**改动（切协议 / 选预设 / 选模型）只调 `markProviderDirty`
    /// ⇒ **不回读输入框**（值由代码写进 `bind:value` 的绑定变量）。
    /// ⇒ 若点选也走入「回读 DOM」那条 ⇒ 宿主会用**旧的 DOM 值覆盖**刚写进去的
    /// 新值（实测：选预设后端点/模型被打回旧值）。
    pub on_mark_dirty: ProviderIntent,
    /// 「整组提交」意图出口（入参 = 组名 + **整组草稿**）。
    pub on_submit_group: GroupSubmitIntent,
}

impl std::fmt::Debug for ProviderGroupProps {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ProviderGroupProps")
            .field("protocol", &self.protocol.get())
            .field("preset", &self.preset.get())
            .field("base_url", &self.base_url.get())
            .field("model", &self.model.get())
            .field("show_api_key", &self.show_api_key.get())
            .field("test_result", &self.test_result.get().is_some())
            .field("apply_result", &self.apply_result.get().is_some())
            .field("effective_config", &self.effective_config.get().is_some())
            .field("show_advanced_gateway", &self.show_advanced_gateway.get())
            .field("show_api_key_modal", &self.show_api_key_modal.get())
            .field("presets_openai", &self.presets.openai.len())
            .field("presets_anthropic", &self.presets.anthropic.len())
            .finish_non_exhaustive()
    }
}

impl ProviderGroupProps {
    /// 取**整组草稿**（生产 `providerGroupDraft()`）。
    ///
    /// ⚠️ 取值顺序 = 生产字段顺序（`protocol` / `preset` / `baseUrl` / `apiKey` /
    /// `model` / `anthropicVersion`）。
    pub fn draft(&self) -> ProviderGroupDraft {
        ProviderGroupDraft {
            protocol: self.protocol.get(),
            preset: self.preset.get(),
            base_url: self.base_url.get(),
            api_key: self.api_key.get(),
            model: self.model.get(),
            anthropic_version: self.anthropic_version.get(),
        }
    }
}

/// 取整组草稿的独立函数（`view!` 闭包里用 ⇒ 不借 `&self` 的临时生命周期）。
fn draft_of(props: &ProviderGroupProps) -> ProviderGroupDraft {
    props.draft()
}

/// 服务商与连接面组件（两个分区由宿主的 `section` 决定）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ProviderGroup;

impl UiComponent for ProviderGroup {
    type Props = ProviderGroupProps;
    type Output = VNode;

    /// 渲染**两个区域**（与生产结构同构）：
    ///
    /// - `models` 分区的 `.setting-block`（**始终存在**）；
    /// - API Key 模态（`{#if showApiKeyModal}` ⇒ 关着时**一个节点都不产出**）。
    ///
    /// ⚠️ 为什么不做「按分区二选一」：生产的模态是 `position: fixed` 的**覆盖层**
    /// ⇒ 它打开时底下的 `.setting-block` **仍然在**（遮罩会透出它）⇒ 引擎侧若按
    /// 分区二选一 ⇒ 模态对照的遮罩底下什么都没有 ⇒ 与参考侧的合成结果不同。
    fn render(props: &Self::Props) -> Self::Output {
        VNode::fragment(vec![render_models(props), render_api_key_modal(props)])
    }
}

// ── 渲染工具 ────────────────────────────────────────────────────────────────

/// 渲染一个图标（**统一入口** ⇒ 尺寸与 class 只在这里定）。
fn icon(data: &'static IconData, size: u32) -> VNode {
    Icon::render(&IconProps::new(data, size))
}

/// 渲染一个带附加 class 的图标（生产 `<RotateCcw class="spin" />` 的形态）。
fn icon_with_class(data: &'static IconData, size: u32, extra: &str) -> VNode {
    Icon::render(&IconProps::new(data, size).extra_class(extra))
}

/// 空片段（`None` 分支 ⇒ **不产出任何节点**）。
fn nothing() -> VNode {
    VNode::fragment(Vec::new())
}

/// 把 `Vec<VNode>` 用**单空格文本节点**连起来（生产模板里元素之间的空白）。
///
/// ## 为什么要单空格
///
/// 生产模板里相邻元素之间的换行 ⇒ Svelte 编译成**一个空格文本节点**
/// （空白规则第 ② 档）。像素对照时这是**真实的排版输入**（行内元素之间的
/// 间隙）⇒ 不能省。
fn spaced(nodes: Vec<VNode>) -> Vec<VNode> {
    let mut out = Vec::new();
    for (index, node) in nodes.into_iter().enumerate() {
        if index > 0 {
            out.push(VNode::text(" "));
        }
        out.push(node);
    }
    out
}

/// 渲染一个「标签 + 字段旗标」的 `<label>`（**尾随空格逐字**）。
fn render_label(for_id: &str, text: &str, flag: FieldFlagState) -> VNode {
    VNode::element("label")
        .attr("for", for_id)
        .child(VNode::text(text))
        .child(FieldFlag::render(&FieldFlagProps::new(flag)))
        .build()
}

/// 渲染一个文本输入框（**不写 `value` 属性** ⇒ 生产用 property）。
fn render_text_input(
    id: &str,
    input_type: &str,
    placeholder: &str,
    value: &str,
    on_input: impl Fn(&mut DomEvent) + 'static,
) -> VNode {
    VNode::element("input")
        .attr("id", id)
        .attr("type", input_type)
        .attr("placeholder", placeholder)
        // ⚠️ **取值由组件声明**（`prop("value", …)` ⇒ **property**，不写属性）：
        // 生产是 `bind:value` ⇒ Svelte 写 property、DOM 上**没有** `value` 属性
        // ⇒ 组件自己带值 ⇒ 宿主不必再按选择器回写（写 attribute 对 `input.value` 无效）。
        .prop("value", value)
        .handler(EventKind::Input, ElementHandler::new(on_input))
        .build()
}

// ── 分区：models ────────────────────────────────────────────────────────────

/// `models` 分区。
///
/// ## 结构（逐字照生产 markup 的两层）
///
/// ```text
/// div.setting-block
///   h3.block-title / p.block-desc
///   div.provider-summary
///   {#if applyResult && warnings.length > 0} div.warnings-box
///   {#if effectiveConfig} div.effective-config.info-card
///   div.text-group[role=group][aria-label]      ← **整组提交的容器**
///     div.config-step × 3（协议 / 预设 / 端点密钥模型）
///     {#if testResult} div.test-result-box
///   div.advanced-box（折叠头 + {#if showAdvancedGateway} 展开体）
/// ```
///
/// ⚠️ **三个 config-step 都在 `.text-group` 里**（整组失焦提交覆盖它们）
/// ⇒ 不要把它们拆成 `.setting-block` 的直接子节点。
fn render_models(props: &ProviderGroupProps) -> VNode {
    let protocol = props.protocol.get();
    let preset_id = props.preset.get();
    let base_url = props.base_url.get();
    let model = props.model.get();
    let api_key = props.api_key.get();
    let title = MODELS_TITLE;
    let desc = MODELS_DESC;

    let summary = render_summary(props, &protocol, &base_url, &model, &api_key);
    let warnings = render_warnings(props);
    let effective = render_effective_config(props);
    let step_protocol = render_protocol_step(props, &protocol);
    let step_presets = render_presets_step(props, &protocol, &preset_id);
    let step_fields = render_fields_step(props, &protocol);
    let test_box = render_test_result(props);
    let advanced = render_advanced_gateway(props);

    // `.text-group` 的四个孩子（三个步骤 + 测连接回执）⇒ **元素之间一个空格**
    // （生产模板的换行折叠口径；`{#if}` 锚点不产出元素）。
    let mut steps: Vec<VNode> = vec![step_protocol];
    steps.push(step_presets);
    steps.push(step_fields);
    if props.test_result.get().is_some() {
        steps.push(test_box);
    }
    // 整组提交的两个入口（失焦 / 回车）⇒ **同一个**整组意图。
    // ⚠️ 载荷是 [`ProviderGroupDraft`]（**整组**）⇒ 判据 T5。
    let submit_focus = props.on_submit_group.clone();
    let draft_focus = draft_of(props);
    let submit_key = props.on_submit_group.clone();
    let draft_key = draft_of(props);
    let text_group = VNode::element("div")
        .attr("class", "text-group")
        .attr("role", "group")
        .attr("aria-label", "服务商端点、密钥与模型")
        .handler(
            EventKind::FocusOut,
            ElementHandler::new(move |_event: &mut DomEvent| {
                (submit_focus)("provider-group", &draft_focus);
            }),
        )
        .handler(
            EventKind::KeyDown,
            ElementHandler::new(move |_event: &mut DomEvent| {
                (submit_key)("provider-group", &draft_key);
            }),
        )
        .children(spaced(steps))
        .build();

    view! {
        div(class = "setting-block") {
            h3(class = "block-title") { title }
            " "
            p(class = "block-desc") { desc }
            " "
            { summary }
            " "
            { warnings }
            " "
            { effective }
            " "
            { text_group }
            " "
            { advanced }
        }
    }
}

/// 摘要条（协议徽标 / 预设名 / 模型 / 状态药品 / 端点）。
fn render_summary(
    props: &ProviderGroupProps,
    protocol: &str,
    base_url: &str,
    model: &str,
    api_key: &str,
) -> VNode {
    let badge = protocol_badge(protocol);
    let preset_name = props
        .presets
        .active(protocol, &props.preset.get())
        .map(|item| item.name.clone())
        .unwrap_or_else(|| String::from(CUSTOM_PRESET_NAME));
    let model_text = if model.is_empty() {
        String::from(NO_MODEL_TEXT)
    } else {
        model.to_owned()
    };
    let result = props.test_result.get();
    let status = status_label(result.as_ref(), api_key);
    let status_class = status_classes(result.as_ref()).join(" ");
    let url_text = if base_url.is_empty() {
        String::from(NO_URL_TEXT)
    } else {
        base_url.to_owned()
    };

    view! {
        div(class = "provider-summary") {
            div(class = "summary-main") {
                span(class = "summary-badge") { badge }
                " "
                strong { preset_name }
                " "
                span(class = "summary-model") { model_text }
            }
            " "
            div(class = "summary-side") {
                span(class = status_class.as_str()) { status }
                " "
                span(class = "summary-url") { url_text }
            }
        }
    }
}

/// 警告框（**`applyResult` 存在且 `warnings` 非空**才渲染）。
fn render_warnings(props: &ProviderGroupProps) -> VNode {
    let Some(result) = props.apply_result.get() else {
        return nothing();
    };
    if result.warnings.is_empty() {
        return nothing();
    }
    let count = result.warnings.len().to_string();
    let head = format!("{WARNINGS_PREFIX}{count}{WARNINGS_SUFFIX}");
    let items: Vec<VNode> = result
        .warnings
        .iter()
        .map(|warning| view! { li { warning.clone() } })
        .collect();
    let list = VNode::element("ul").children(spaced(items)).build();
    view! {
        div(class = "warnings-box") {
            strong { head }
            " "
            { list }
        }
    }
}

/// 网关生效配置卡（`effectiveConfig` 存在才渲染）。
fn render_effective_config(props: &ProviderGroupProps) -> VNode {
    let Some(config) = props.effective_config.get() else {
        return nothing();
    };
    let title = EFFECTIVE_TITLE;
    let provider = EffectiveConfig::shown(&config.provider, EFFECTIVE_MISSING);
    let base_url = EffectiveConfig::shown(&config.base_url, EFFECTIVE_MISSING);
    let model = EffectiveConfig::shown(&config.model, EFFECTIVE_MISSING);
    let api_key = EffectiveConfig::shown(&config.api_key, EFFECTIVE_NO_KEY);
    let provider_label = EFFECTIVE_PROVIDER_LABEL;
    let base_url_label = EFFECTIVE_BASE_URL_LABEL;
    let model_label = EFFECTIVE_MODEL_LABEL;
    let api_key_label = EFFECTIVE_API_KEY_LABEL;

    view! {
        div(class = "effective-config info-card") {
            strong(class = "info-title") { title }
            " "
            div(class = "effective-grid") {
                span {
                    provider_label
                    code { provider }
                }
                " "
                span {
                    base_url_label
                    code { base_url }
                }
                " "
                span {
                    model_label
                    code { model }
                }
                " "
                span {
                    api_key_label
                    code { api_key }
                }
            }
        }
    }
}

/// 步骤 1：协议页签（**两个按钮**，选中态是条件 class）。
fn render_protocol_step(props: &ProviderGroupProps, protocol: &str) -> VNode {
    let title = PROTOCOL_STEP_TITLE;
    let number = STEP_ONE;
    let openai_selected = protocol == "openai";
    let anthropic_selected = protocol == "anthropic";

    // 生产：`switchProtocol(...)` + `markProviderDirty()`（**默认 = 四个文本键**）。
    let openai_protocol = props.on_action.clone();
    let anthropic_protocol = props.on_action.clone();
    let openai_dirty = props.on_mark_dirty.clone();
    let anthropic_dirty = props.on_mark_dirty.clone();

    let openai_class = if openai_selected {
        "protocol-tab selected"
    } else {
        "protocol-tab"
    };
    let anthropic_class = if anthropic_selected {
        "protocol-tab selected"
    } else {
        "protocol-tab"
    };
    let openai_title = PROTO_TITLE_OPENAI;
    let openai_sub = PROTO_SUB_OPENAI;
    let anthropic_title = PROTO_TITLE_ANTHROPIC;
    let anthropic_sub = PROTO_SUB_ANTHROPIC;

    view! {
        div(class = "config-step") {
            span(class = "step-num") { number }
            " "
            div(class = "step-body") {
                h4(class = "step-title") { title }
                " "
                div(class = "protocol-tabs") {
                    button(class = openai_class, onclick = move |_| {
                        (openai_protocol)("protocol:openai");
                        (openai_dirty)(PROVIDER_TEXT_KEYS);
                    }) {
                        { icon(&icons::ICON_GLOBE, 15) }
                        " "
                        div(class = "proto-text") {
                            span(class = "proto-title") { openai_title }
                            " "
                            span(class = "proto-sub") { openai_sub }
                        }
                    }
                    " "
                    button(class = anthropic_class, onclick = move |_| {
                        (anthropic_protocol)("protocol:anthropic");
                        (anthropic_dirty)(PROVIDER_TEXT_KEYS);
                    }) {
                        { icon(&icons::ICON_SPARKLES, 15) }
                        " "
                        div(class = "proto-text") {
                            span(class = "proto-title") { anthropic_title }
                            " "
                            span(class = "proto-sub") { anthropic_sub }
                        }
                    }
                }
            }
        }
    }
}

/// 步骤 2：预设芯片（**表驱动** ⇒ 组件里没有一条预设）。
fn render_presets_step(props: &ProviderGroupProps, protocol: &str, preset_id: &str) -> VNode {
    let title = PRESET_STEP_TITLE;
    let number = STEP_TWO;
    let mut chips: Vec<VNode> = Vec::new();
    for item in props.presets.for_protocol(protocol) {
        let id = item.id.clone();
        let name = item.name.clone();
        let selected = item.id == preset_id;
        let class = if selected {
            "preset-chip selected"
        } else {
            "preset-chip"
        };
        let select = props.on_action.clone();
        let dirty = props.on_mark_dirty.clone();
        // 生产：`selectPreset(p.id)` + `markProviderDirty(['provider.baseUrl', 'provider.model'])`
        // ⇒ 两条意图各自带**自己的**载荷（id / 脏键表）。
        let action = format!("preset:{id}");
        chips.push(view! {
            button(class = class, onclick = move |_| {
                (select)(&action);
                (dirty)(PRESET_DIRTY_KEYS);
            }) {
                span { name.clone() }
            }
        });
    }

    view! {
        div(class = "config-step") {
            span(class = "step-num") { number }
            " "
            div(class = "step-body") {
                h4(class = "step-title") { title }
                " "
                div(class = "presets-row") {
                    for chip in chips {
                        { chip }
                    }
                }
            }
        }
    }
}

/// 步骤 3 的一个关键点：**密钥输入的明文切换是属性分支**（`password` ⇔ `text`）。
fn render_key_toggle(props: &ProviderGroupProps, show: bool) -> VNode {
    let title = if show { HIDE_KEY_TITLE } else { SHOW_KEY_TITLE };
    // ⚠️ **只产出一个**图标节点（T4 判据：不是两个、不是零个）。
    let glyph = if show {
        icon(&icons::ICON_EYE_OFF, 14)
    } else {
        icon(&icons::ICON_EYE, 14)
    };
    let toggle = props.on_action.clone();
    VNode::element("button")
        .attr("class", "key-toggle-btn")
        .attr("type", "button")
        .attr("title", title)
        .handler(
            EventKind::Click,
            ElementHandler::new(move |_event: &mut DomEvent| {
                (toggle)("toggle-show-key");
            }),
        )
        .child(glyph)
        .build()
}

/// 模型选择器的**闭合态占位**（`SessionModelPicker` 不迁）。
///
/// ## 为什么不迁
///
/// 生产 `SessionModelPicker.svelte` 是 **241 行的外部组件**（下拉 / 过滤 /
/// 键盘 / 键选 / 向上翻）⇒ 它**不属于本面**（切分方案 §2.9 ④）。
///
/// ## 为什么占位也要**逐字照闭合态 DOM**
///
/// 像素对照的裁剪区**包含**这个选择器（它在 `.setting-block` 里）⇒ 占位的
/// 结构 / class / 文本 / 图标必须与生产**闭合态**一致，否则 T1 必败。
///
/// ⇒ 产出（逐字照生产闭合态）：
///
/// ```text
/// div.model-picker > button.trigger[aria-haspopup=listbox][aria-expanded=false][title]
///   > span.value + span.chevron > svg
/// ```
///
/// ⚠️ **下拉未迁移** ⇒ 点击**只发意图**（`model-picker`）⇒ 宿主可弹原生选择器；
/// 本组件不假装有下拉。
fn render_model_picker_placeholder(props: &ProviderGroupProps, model: &str) -> VNode {
    let loading = props.models_loading.get();
    let value = if model.is_empty() {
        String::from(DEFAULT_MODEL_ID)
    } else {
        model.to_owned()
    };
    let mut root = VNode::element("div").attr("class", "model-picker");
    if loading {
        root = root.attr("class", "model-picker disabled");
    }
    let intent = props.on_action.clone();
    let mut trigger = VNode::element("button")
        .attr("class", "trigger")
        // ⚠️ 连字符属性在 `view!` 里不支持 ⇒ 构造器写。
        .attr("aria-haspopup", "listbox")
        .attr("aria-expanded", "false")
        .attr("title", value.clone())
        .handler(
            EventKind::Click,
            ElementHandler::new(move |_event: &mut DomEvent| {
                (intent)("model-picker");
            }),
        );
    if loading {
        trigger = trigger.attr("disabled", "");
    }
    let trigger = trigger
        .child(view! { span(class = "value") { value.clone() } })
        .child(VNode::text(" "))
        .child(view! {
            span(class = "chevron") {
                { icon(&icons::ICON_CHEVRON_DOWN, 14) }
            }
        })
        .build();
    root.child(trigger).build()
}

/// 步骤 3：端点 / 模型 / 模型选择器 / 密钥 / 协议头 / 测连接。
///
/// ⚠️ 只返回**一个 `config-step`**（它的父容器 `.text-group` 由
/// [`render_models`] 拼，因为三个步骤共享同一个整组提交容器）。
fn render_fields_step(props: &ProviderGroupProps, protocol: &str) -> VNode {
    let title = FIELDS_STEP_TITLE;
    let number = STEP_THREE;
    let flags = props.text_flags.get();
    let url_flag = flag_of(&flags, "provider.baseUrl");
    let model_flag = flag_of(&flags, "provider.model");
    let key_flag = flag_of(&flags, "provider.apiKey");
    let version_flag = flag_of(&flags, "provider.anthropicVersion");
    let model = props.model.get();

    let url_label = render_label("provider-url-input", URL_LABEL, url_flag);
    let model_label = render_label("provider-model-input", MODEL_LABEL, model_flag);
    let key_label_node = render_label("provider-key-input", key_label(protocol), key_flag);
    let version_label = render_label("anthropic-ver-input", ANTHROPIC_VERSION_LABEL, version_flag);

    let url_input = {
        let field = props.on_field_input.clone();
        render_text_input(
            "provider-url-input",
            "text",
            url_placeholder(protocol),
            &props.base_url.get(),
            move |_event| (field)("provider.baseUrl"),
        )
    };
    let model_input = {
        let field = props.on_field_input.clone();
        render_text_input(
            "provider-model-input",
            "text",
            model_placeholder(protocol),
            &props.model.get(),
            move |_event| (field)("provider.model"),
        )
    };
    let key_input = {
        let field = props.on_field_input.clone();
        let input_type = key_input_type(props.show_api_key.get());
        VNode::element("input")
            .attr("id", "provider-key-input")
            .attr("autocomplete", "off")
            // ⚠️ `type` 是**属性分支**（T4：`password` ⇔ `text`）。
            .attr("type", input_type)
            .attr("placeholder", key_placeholder(protocol))
            // 取值由组件声明（property）；密钥**明文不进属性**。
            .prop("value", props.api_key.get())
            .handler(
                EventKind::Input,
                ElementHandler::new(move |_event: &mut DomEvent| {
                    (field)("provider.apiKey");
                }),
            )
            .build()
    };
    let version_input = {
        let field = props.on_field_input.clone();
        render_text_input(
            "anthropic-ver-input",
            "text",
            ANTHROPIC_VERSION_PLACEHOLDER,
            &props.anthropic_version.get(),
            move |_event| (field)("provider.anthropicVersion"),
        )
    };

    let picker = render_model_picker_placeholder(props, &model);
    let reload = {
        let action = props.on_action.clone();
        let loading = props.models_loading.get();
        let mut button = VNode::element("button").attr("class", "quiet-button");
        if loading {
            button = button.attr("disabled", "");
        }
        let spin = if loading { "spin" } else { "" };
        let button_text = if loading { LOADING_LABEL } else { RELOAD_LABEL };
        button
            .handler(
                EventKind::Click,
                ElementHandler::new(move |_event: &mut DomEvent| {
                    (action)("reload-models");
                }),
            )
            .child(icon_with_class(&icons::ICON_ROTATE_CCW, 13, spin))
            .child(VNode::text(" "))
            .child(view! { span { button_text } })
            .build()
    };
    let models_error = {
        let error = props.models_error.get();
        if error.is_empty() {
            nothing()
        } else {
            view! { small(class = "field-hint error-hint") { error.clone() } }
        }
    };
    let hint = DEFAULT_MODEL_HINT;
    let group_label = GROUP_LABEL_MODELS;

    let key_toggle = render_key_toggle(props, props.show_api_key.get());
    let key_wrapper = VNode::element("div")
        .attr("class", "key-input-wrapper")
        .child(key_input)
        .child(VNode::text(" "))
        .child(key_toggle)
        .build();

    // 协议头那一组（**只在 Anthropic 协议下出现**）。
    let version_group = if protocol == "anthropic" {
        view! {
            div(class = "form-group") {
                { version_label }
                " "
                { version_input }
            }
        }
    } else {
        nothing()
    };

    // 测连接按钮（**HTTP 未接线** ⇒ 只发意图）。
    let testing = props.testing.get();
    let test_button = {
        let action = props.on_action.clone();
        let mut button = VNode::element("button").attr("class", "primary-button");
        // 生产：`disabled={isTestingConnection || !providerBaseUrl}`
        // ⇒ 未测连接且端点为非空 ⇒ 属性**不出现**。
        if testing || props.base_url.get().is_empty() {
            button = button.attr("disabled", "");
        }
        let button_text = if testing {
            TESTING_CONNECTION_LABEL
        } else {
            TEST_CONNECTION_LABEL
        };
        let spin = if testing { "spin" } else { "" };
        button
            .handler(
                EventKind::Click,
                ElementHandler::new(move |_event: &mut DomEvent| {
                    (action)("test-connection");
                }),
            )
            .child(icon_with_class(&icons::ICON_ROTATE_CCW, 13, spin))
            .child(VNode::text(" "))
            .child(view! { span { button_text } })
            .build()
    };

    // 预设推荐的芯片（**数据驱动** ⇒ 来自当前预设的 `models`）。
    let recommended = props
        .presets
        .recommended(protocol, &props.preset.get())
        .to_vec();
    let mut actions: Vec<VNode> = vec![test_button];
    if !recommended.is_empty() {
        let chip_label = RECOMMENDED_LABEL;
        actions.push(view! { span(class = "chip-label") { chip_label } });
        let current = props.model.get();
        for item in recommended {
            let selected = item == current;
            let class = if selected {
                "model-chip selected"
            } else {
                "model-chip"
            };
            let action = props.on_action.clone();
            let dirty = props.on_mark_dirty.clone();
            let action_name = format!("model:{item}");
            actions.push(view! {
                button(class = class, onclick = move |_| {
                    (action)(&action_name);
                    (dirty)("provider.model");
                }) {
                    item.clone()
                }
            });
        }
    }
    let actions_node = VNode::element("div")
        .attr("class", "model-actions")
        .children(spaced(actions))
        .build();

    // 端点 + 模型（两列）。
    let url_group = VNode::element("div")
        .attr("class", "form-group")
        .child(url_label)
        .child(VNode::text(" "))
        .child(url_input)
        .build();
    let model_group = VNode::element("div")
        .attr("class", "form-group")
        .child(model_label)
        .child(VNode::text(" "))
        .child(model_input)
        .build();
    let row = VNode::element("div")
        .attr("class", "form-row-2")
        .child(url_group)
        .child(VNode::text(" "))
        .child(model_group)
        .build();

    let picker_row = VNode::element("div")
        .attr("class", "model-picker-row")
        .child(picker)
        .child(VNode::text(" "))
        .child(reload)
        .build();
    let picker_group = VNode::element("div")
        .attr("class", "form-group")
        .child(view! { span(class = "group-label") { group_label } })
        .child(VNode::text(" "))
        .child(picker_row)
        .child(VNode::text(" "))
        .child(models_error)
        .child(VNode::text(" "))
        .child(view! { small(class = "field-hint") { hint } })
        .build();

    let key_group = VNode::element("div")
        .attr("class", "form-group")
        .child(key_label_node)
        .child(VNode::text(" "))
        .child(key_wrapper)
        .build();

    view! {
        div(class = "config-step") {
            span(class = "step-num") { number }
            " "
            div(class = "step-body") {
                h4(class = "step-title") { title }
                " "
                { row }
                " "
                { picker_group }
                " "
                { key_group }
                " "
                { version_group }
                " "
                { actions_node }
            }
        }
    }
}

/// 测连接回执（成功 / 失败 + 延迟 + 远端模型目录）。
fn render_test_result(props: &ProviderGroupProps) -> VNode {
    let Some(result) = props.test_result.get() else {
        return nothing();
    };
    let class = if result.ok {
        "test-result-box success"
    } else {
        "test-result-box failed"
    };
    let (head_icon, head_title) = if result.ok {
        (
            icon_with_class(&icons::ICON_CIRCLE_CHECK, 16, "head-icon success-icon"),
            TEST_OK_TITLE,
        )
    } else {
        (
            icon_with_class(&icons::ICON_CIRCLE_X, 16, "head-icon failed-icon"),
            TEST_FAIL_TITLE,
        )
    };

    let latency = match result.latency_ms {
        Some(value) => {
            let text = format!("{} ms", crate::num::number_to_string(value));
            view! { span(class = "latency-badge") { text } }
        }
        None => nothing(),
    };

    let mut head_children = vec![head_icon, view! { strong { head_title } }];
    if result.latency_ms.is_some() {
        head_children.push(latency);
    }
    let head = VNode::element("div")
        .attr("class", "test-result-head")
        .children(spaced(head_children))
        .build();

    let message = result.message.clone();
    let message_node = view! { p(class = "test-result-msg") { message.clone() } };

    let discovered = render_discovered_models(props, &result);

    let mut children = vec![head, message_node];
    if result.models.is_empty() {
        // 生产：`{#if testResult.models && testResult.models.length > 0}` ⇒ 空 ⇒ 不渲染。
    } else {
        children.push(discovered);
    }
    VNode::element("div")
        .attr("class", class)
        .children(spaced(children))
        .build()
}

/// 远端模型目录（搜索 + 前 24 项 + 截断提示）。
fn render_discovered_models(props: &ProviderGroupProps, result: &TestResult) -> VNode {
    let query = props.model_search_query.get();
    let filtered = filter_models(&result.models, &query);
    let shown = catalog_slice(&filtered);
    let truncated = catalog_truncated(&filtered);
    let count = result.models.len().to_string();
    let count_label = format!("{REMOTE_MODELS_PREFIX}{count}{REMOTE_MODELS_SUFFIX}");
    let placeholder = MODEL_SEARCH_PLACEHOLDER;

    let search_input = {
        // ⚠️ **走「字段输入」通道**，不是「动作」通道：
        // 生产是 `bind:value={modelSearchQuery}` ⇒ 输入即过滤
        // （`filteredDiscoveredModels` 是 `$derived.by` ⇒ 依赖 `modelSearchQuery`）
        // ⇒ 引擎侧必须让**宿主回读 DOM** 并更新 `model.searchQuery` 信号
        // ⇒ 否则列表**不会被过滤**（实测：键入后引擎侧仍显示 3 个模型、
        // 参考侧只剩 2 个 ⇒ 裁剪高度差 158 px ⇒ 像素对照无法进行）。
        // ⇒ 动作通道（`on_action`）**不**用于此处：它**不携带取值**。
        let field = props.on_field_input.clone();
        VNode::element("input")
            .attr("type", "search")
            .attr("placeholder", placeholder)
            // 取值由组件声明（property）⇒ 宿主不再回写。
            .prop("value", props.model_search_query.get())
            .handler(
                EventKind::Input,
                ElementHandler::new(move |_event: &mut DomEvent| {
                    (field)("model.searchQuery");
                }),
            )
            .build()
    };
    let search = VNode::element("div")
        .attr("class", "disc-search")
        .child(icon(&icons::ICON_SEARCH, 13))
        .child(VNode::text(" "))
        .child(search_input)
        .build();
    let head = VNode::element("div")
        .attr("class", "disc-head")
        .child(view! { span(class = "disc-label") { count_label } })
        .child(VNode::text(" "))
        .child(search)
        .build();

    let current = props.model.get();
    let mut items: Vec<VNode> = Vec::new();
    if shown.is_empty() {
        let empty = CATALOG_EMPTY_TEXT;
        items.push(view! { p(class = "catalog-empty") { empty } });
    } else {
        for item in &shown {
            let selected = item == &current;
            let class = if selected {
                "catalog-item selected"
            } else {
                "catalog-item"
            };
            let action = props.on_action.clone();
            let dirty = props.on_mark_dirty.clone();
            let id = item.clone();
            let action_name = format!("model:{id}");
            let check = if selected {
                icon(&icons::ICON_CHECK, 12)
            } else {
                nothing()
            };
            items.push(view! {
                // ⭐ `key = id`（列表身份）：模型列表会随**搜索词过滤**而变长变短
                // ⇒ 无 key 时按位置配对 ⇒ 列表长度一变，**后面的兄弟就错位**
                // ⇒ ⇒ 搜索框（列表的兄弟）被换掉 ⇒ **用户输入与焦点一起丢**
                // ⇒ ⇒ 有 key 时走 keyed 差分（按身份配对 + 移动）⇒ 兄弟位置不变。
                button(class = class, key = id.clone(), onclick = move |_| {
                    (action)(&action_name);
                    (dirty)("provider.model");
                }) {
                    span(class = "catalog-name") { id.clone() }
                    " "
                    { check }
                }
            });
        }
    }
    // ⚠️ **不插空格文本节点**（2026-10-25 改）：
    // - 生产的 `.model-catalog` 是 `display: flex; flex-direction: column; gap: 2px`
    //   ⇒ **空白文本节点在 flex 容器里不生成 flex item** ⇒ 插空格**无视觉影响**；
    // - 生产用 `{#each}` ⇒ **Svelte 也不产生**元素间空白 ⇒ 引擎侧插空格反而**不像生产**；
    // - ⚠️ 更关键：空格文本节点**没有 key** ⇒ `list_diff_mode` 的「全部有 key」
    //   ⇒ **keyed 差分永远不成立** ⇒ 列表长度一变，**后面的兄弟就错位**
    //   ⇒ ⇒ 搜索框（列表的兄弟）被换掉 ⇒ **用户输入与焦点一起丢**。
    let catalog = VNode::element("div")
        .attr("class", "model-catalog")
        .children(items)
        .build();

    let more = if truncated {
        let text = MORE_MODELS_TEXT;
        view! { p(class = "more-models") { text } }
    } else {
        nothing()
    };

    let mut children = vec![head, catalog];
    if truncated {
        children.push(more);
    }
    VNode::element("div")
        .attr("class", "discovered-models")
        .children(spaced(children))
        .build()
}

/// 高级网关折叠块（折叠头 + 展开后的网关地址 / 认证密钥）。
fn render_advanced_gateway(props: &ProviderGroupProps) -> VNode {
    let gateway_label = ADVANCED_GATEWAY_LABEL;
    let open = props.show_advanced_gateway.get();
    let toggle = props.on_action.clone();
    let chevron = if open {
        icon(&icons::ICON_CHEVRON_UP, 14)
    } else {
        icon(&icons::ICON_CHEVRON_DOWN, 14)
    };
    let head = VNode::element("button")
        .attr("class", "advanced-toggle")
        .handler(
            EventKind::Click,
            ElementHandler::new(move |_event: &mut DomEvent| {
                (toggle)("toggle-advanced");
            }),
        )
        .child(view! {
            div(class = "adv-left") {
                { icon(&icons::ICON_SERVER, 14) }
                " "
                span { gateway_label }
            }
        })
        .child(VNode::text(" "))
        .child(chevron)
        .build();

    let body = if open {
        render_advanced_body(props)
    } else {
        nothing()
    };

    VNode::element("div")
        .attr("class", "advanced-box")
        .child(head)
        .child(VNode::text(" "))
        .child(body)
        .build()
}

/// 高级网关展开后的两组（网关地址 + 网关认证密钥）。
fn render_advanced_body(props: &ProviderGroupProps) -> VNode {
    let flags = props.text_flags.get();
    let url_flag = flag_of(&flags, "gateway.baseUrl");
    let url_label = render_label("endpoint-input", GATEWAY_URL_LABEL, url_flag);
    let placeholder = GATEWAY_URL_PLACEHOLDER;
    let field = props.on_field_input.clone();
    let submit = props.on_submit_group.clone();
    let submit_key = props.on_submit_group.clone();
    let draft_focus = draft_of(props);
    let draft_key = draft_of(props);

    let url_input = VNode::element("input")
        .attr("id", "endpoint-input")
        .attr("type", "text")
        .attr("placeholder", placeholder)
        // 取值由组件声明（property）⇒ 宿主不再回写。
        .prop("value", props.edit_base_url.get())
        .handler(
            EventKind::Input,
            ElementHandler::new(move |_event| (field)("gateway.baseUrl")),
        )
        .handler(
            EventKind::FocusOut,
            ElementHandler::new(move |_event| {
                (submit)("gateway-url", &draft_focus);
            }),
        )
        .handler(
            EventKind::KeyDown,
            ElementHandler::new(move |_event| {
                (submit_key)("gateway-url", &draft_key);
            }),
        )
        .build();

    let hint = GATEWAY_URL_HINT;
    let url_group = VNode::element("div")
        .attr("class", "form-group")
        .child(url_label)
        .child(VNode::text(" "))
        .child(url_input)
        .child(VNode::text(" "))
        .child(view! { small(class = "field-hint") { hint } })
        .build();

    let key_label = GATEWAY_KEY_LABEL;
    let configured = if props.has_api_key {
        GATEWAY_KEY_CONFIGURED
    } else {
        GATEWAY_KEY_NOT_CONFIGURED
    };
    let button_label = if props.has_api_key {
        CHANGE_KEY_LABEL
    } else {
        CONFIGURE_KEY_LABEL
    };
    let open_modal = props.on_action.clone();
    let status = VNode::element("div")
        .attr("class", "cred-status")
        .child(icon(&icons::ICON_LOCK, 14))
        .child(VNode::text(" "))
        .child(view! { span { configured } })
        .build();
    let open_button = VNode::element("button")
        .attr("class", "quiet-button")
        .handler(
            EventKind::Click,
            ElementHandler::new(move |_event: &mut DomEvent| {
                (open_modal)("open-api-key-modal");
            }),
        )
        .child(VNode::text(button_label))
        .build();
    let credential_row = VNode::element("div")
        .attr("class", "credential-row")
        .child(status)
        .child(VNode::text(" "))
        .child(open_button)
        .build();
    let key_hint = GATEWAY_KEY_HINT;
    let key_group = VNode::element("div")
        .attr("class", "form-group")
        .child(render_label(
            "api-key-status",
            key_label,
            FieldFlagState::None,
        ))
        .child(VNode::text(" "))
        .child(credential_row)
        .child(VNode::text(" "))
        .child(view! { small(class = "field-hint") { key_hint } })
        .build();

    VNode::element("div")
        .attr("class", "advanced-body")
        .child(url_group)
        .child(VNode::text(" "))
        .child(key_group)
        .build()
}

// ── 分区：API Key 模态（**整视口**） ───────────────────────────────────────

/// API Key 模态（`position: fixed` ⇒ **整视口**截图）。
fn render_api_key_modal(props: &ProviderGroupProps) -> VNode {
    if !props.show_api_key_modal.get() {
        // 生产的模态是 `{#if showApiKeyModal}` ⇒ 关着 ⇒ **一个节点都不产出**。
        return nothing();
    }
    let title = MODAL_TITLE;
    let desc = MODAL_DESC;
    let loading = props.keychain_loading.get();
    let exists = props.stored_key_exists.get();
    let masked = props.stored_key_masked.get();
    let error = props.keychain_action_error.get();
    let saving = props.keychain_saving.get();
    let deleting = props.keychain_deleting.get();

    // 钥匙串状态：读取中 / 已存 / 暂无（**三支互斥**）。
    let keychain = if loading {
        let text = KEYCHAIN_LOADING_TEXT;
        view! {
            div(class = "keychain-status") {
                span(class = "keychain-muted") { text }
            }
        }
    } else if exists {
        let text = format!("{masked}{KEYCHAIN_STORED_SUFFIX}");
        view! {
            div(class = "keychain-status") {
                span(class = "keychain-stored") { text }
            }
        }
    } else {
        let text = KEYCHAIN_EMPTY_TEXT;
        view! {
            div(class = "keychain-status") {
                span(class = "keychain-muted") { text }
            }
        }
    };

    let placeholder = MODAL_KEY_PLACEHOLDER;
    let temp_field = props.on_field_input.clone();
    let temp_input = VNode::element("input")
        .attr("type", "password")
        .attr("placeholder", placeholder)
        .attr("autocomplete", "off")
        // 取值由组件声明（property）；临时密钥**同样只走 property**。
        .prop("value", props.temp_api_key.get())
        .handler(
            EventKind::Input,
            ElementHandler::new(move |_event: &mut DomEvent| {
                (temp_field)("modal.tempApiKey");
            }),
        )
        .build();
    let temp_group = VNode::element("div")
        .attr("class", "form-group")
        .child(temp_input)
        .build();

    let error_node = if error.is_empty() {
        nothing()
    } else {
        view! { p(class = "modal-error") { error.clone() } }
    };

    let body = view! {
        div(class = "modal-body") {
            p(class = "modal-desc") { desc }
            " "
            { keychain }
            " "
            { temp_group }
            " "
            { error_node }
        }
    };

    let header = view! {
        div(class = "modal-header") {
            h3(id = "api-key-dialog-title") { title }
        }
    };

    // 页脚：删除（**仅有已存密钥时**）+ 取消 + 写入并应用。
    let mut footer_children: Vec<VNode> = Vec::new();
    if exists {
        let remove = props.on_action.clone();
        let button_text = if deleting {
            DELETING_KEY_LABEL
        } else {
            DELETE_KEY_LABEL
        };
        let mut delete_button = VNode::element("button")
            .attr("class", "quiet-button danger-text delete-key-btn")
            .attr("title", DELETE_KEY_TITLE);
        if deleting {
            delete_button = delete_button.attr("disabled", "");
        }
        let node = delete_button
            .handler(
                EventKind::Click,
                ElementHandler::new(move |_event: &mut DomEvent| {
                    (remove)("delete-stored-key");
                }),
            )
            .child(icon(&icons::ICON_TRASH_2, 13))
            .child(VNode::text(" "))
            .child(view! { span { button_text } })
            .build();
        footer_children.push(node);
    }
    let cancel = props.on_action.clone();
    footer_children.push(
        VNode::element("button")
            .attr("class", "quiet-button")
            .handler(
                EventKind::Click,
                ElementHandler::new(move |_event: &mut DomEvent| {
                    (cancel)("close-api-key-modal");
                }),
            )
            .child(VNode::text(CANCEL_LABEL))
            .build(),
    );
    let save = props.on_action.clone();
    let save_label = if saving {
        SAVING_KEY_LABEL
    } else {
        SAVE_KEY_LABEL
    };
    let mut save_button = VNode::element("button").attr("class", "primary-button");
    if saving {
        save_button = save_button.attr("disabled", "");
    }
    footer_children.push(
        save_button
            .handler(
                EventKind::Click,
                ElementHandler::new(move |_event: &mut DomEvent| {
                    (save)("save-new-api-key");
                }),
            )
            .child(VNode::text(save_label))
            .build(),
    );
    let footer = VNode::element("div")
        .attr("class", "modal-footer")
        .children(spaced(footer_children))
        .build();

    // 对话框：点自己**不**关（`stopPropagation`）；遮罩点**关**。
    let stop_click = props.on_action.clone();
    let stop_key = props.on_action.clone();
    let dialog = VNode::element("div")
        .attr("class", "modal-dialog")
        .attr("role", "dialog")
        .attr("tabindex", "-1")
        .attr("aria-modal", "true")
        .attr("aria-labelledby", "api-key-dialog-title")
        .handler(
            EventKind::Click,
            ElementHandler::new(move |event: &mut DomEvent| {
                // 生产：`onclick={(e) => e.stopPropagation()}` ⇒ 点对话框**不关**。
                event.stop_propagation();
                (stop_click)("modal-stop");
            }),
        )
        .handler(
            EventKind::KeyDown,
            ElementHandler::new(move |event: &mut DomEvent| {
                event.stop_propagation();
                (stop_key)("modal-stop");
            }),
        )
        .child(header)
        .child(VNode::text(" "))
        .child(body)
        .child(VNode::text(" "))
        .child(footer)
        .build();

    let backdrop_close = props.on_action.clone();
    VNode::element("div")
        .attr("class", "modal-backdrop")
        .attr("role", "presentation")
        .handler(
            EventKind::Click,
            ElementHandler::new(move |_event: &mut DomEvent| {
                (backdrop_close)("close-api-key-modal");
            }),
        )
        .child(dialog)
        .build()
}

// ── 单测（**纯函数 + 虚拟树** ⇒ 不需要浏览器 / 不需要 wasm） ─────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    /// 造一组 props（**默认 = 参考侧初始态**：OpenAI / openai 预设 / 无测连接回执）。
    fn props() -> ProviderGroupProps {
        let runtime = apeireth_ui_core::Runtime::new();
        ProviderGroupProps {
            protocol: runtime.signal(String::from("openai")),
            preset: runtime.signal(String::from("openai")),
            base_url: runtime.signal(String::from("https://api.openai.com/v1")),
            api_key: runtime.signal(String::new()),
            model: runtime.signal(String::from("gpt-4o")),
            anthropic_version: runtime.signal(String::from("2023-06-01")),
            show_api_key: runtime.signal(false),
            discovered_models: runtime.signal(Vec::new()),
            models_loading: runtime.signal(false),
            models_error: runtime.signal(String::new()),
            model_search_query: runtime.signal(String::new()),
            test_result: runtime.signal(None),
            testing: runtime.signal(false),
            apply_result: runtime.signal(None),
            effective_config: runtime.signal(None),
            edit_base_url: runtime.signal(String::from("http://127.0.0.1:8080")),
            show_advanced_gateway: runtime.signal(false),
            has_api_key: false,
            stored_key_masked: runtime.signal(String::from("sk-****abc")),
            stored_key_exists: runtime.signal(true),
            keychain_loading: runtime.signal(false),
            keychain_saving: runtime.signal(false),
            keychain_deleting: runtime.signal(false),
            keychain_action_error: runtime.signal(String::new()),
            temp_api_key: runtime.signal(String::new()),
            show_api_key_modal: runtime.signal(false),
            text_flags: runtime.signal(Vec::new()),
            presets: test_presets(),
            on_action: Rc::new(|_action: &str| {}),
            on_field_input: Rc::new(|_key: &str| {}),
            on_mark_dirty: Rc::new(|_keys: &str| {}),
            on_submit_group: Rc::new(|_group: &str, _draft: &ProviderGroupDraft| {}),
        }
    }

    /// 生产的两组预设（**测试数据** ⇒ 与 fixture 同值，逐字取自生产）。
    fn test_presets() -> ProviderPresets {
        ProviderPresets {
            openai: vec![
                ProviderPreset {
                    id: String::from("openai"),
                    name: String::from("OpenAI 官方"),
                    base_url: String::from("https://api.openai.com/v1"),
                    default_model: String::from("gpt-4o"),
                    models: vec![
                        String::from("gpt-4o"),
                        String::from("gpt-4o-mini"),
                        String::from("o3-mini"),
                        String::from("o1"),
                    ],
                },
                ProviderPreset {
                    id: String::from("custom"),
                    name: String::from("自定义 OpenAI 端点"),
                    base_url: String::new(),
                    default_model: String::new(),
                    models: Vec::new(),
                },
            ],
            anthropic: vec![ProviderPreset {
                id: String::from("anthropic"),
                name: String::from("Anthropic 官方"),
                base_url: String::from("https://api.anthropic.com"),
                default_model: String::from("claude-3-7-sonnet-20250219"),
                models: vec![String::from("claude-3-7-sonnet-20250219")],
            }],
        }
    }

    /// 深度优先找一个元素（按 tag + class token；**展开 Fragment**）。
    fn find<'a>(node: &'a VNode, tag: &str, class: &str) -> Option<&'a VNode> {
        if node.tag() == Some(tag) && node.class_tokens().iter().any(|token| token == class) {
            return Some(node);
        }
        for child in node.children() {
            if let Some(found) = find(child, tag, class) {
                return Some(found);
            }
        }
        None
    }

    /// 把一棵虚拟树摊平成元素序列（**Fragment 不产出 DOM 节点** ⇒ 摊平）。
    fn flatten<'a>(node: &'a VNode, out: &mut Vec<&'a VNode>) {
        if node.tag().is_some() {
            out.push(node);
            for child in node.children() {
                flatten(child, out);
            }
            return;
        }
        for child in node.children() {
            flatten(child, out);
        }
    }

    /// 深度优先找一个元素（按 tag + 属性）。
    fn find_attr<'a>(node: &'a VNode, tag: &str, name: &str, value: &str) -> Option<&'a VNode> {
        let matches = node.tag() == Some(tag)
            && node
                .attr_value(name)
                .and_then(|attr| attr.to_attr_string())
                .as_deref()
                == Some(value);
        if matches {
            return Some(node);
        }
        for child in node.children() {
            if let Some(found) = find_attr(child, tag, name, value) {
                return Some(found);
            }
        }
        None
    }

    /// 深度优先统计某标签的节点数。
    fn count_tag(node: &VNode, tag: &str) -> usize {
        let me = usize::from(node.tag() == Some(tag));
        me + node
            .children()
            .iter()
            .map(|child| count_tag(child, tag))
            .sum::<usize>()
    }

    /// 取一个元素的处理器并调用（**判据 T4 / T5 的执行入口**）。
    fn call_handler(node: &VNode, kind: EventKind) {
        let VNode::Element(element) = node else {
            panic!("处理器只挂在元素上");
        };
        let (_, handler) = element
            .handlers
            .iter()
            .find(|(event, _)| *event == kind)
            .unwrap_or_else(|| panic!("该元素没有 {kind:?} 处理器"));
        let mut event = DomEvent::new(kind, vec![0]);
        handler.call(&mut event);
    }

    /// 取文本节点内容（拼接子树里的全部文本）。
    fn text_of(node: &VNode) -> String {
        if let VNode::Text(text) = node {
            return text.data.clone();
        }
        node.children().iter().map(text_of).collect()
    }

    #[test]
    fn mask_api_key_matches_the_production_rule() {
        assert_eq!(mask_api_key(""), "****", "空 ⇒ 整体打码");
        assert_eq!(mask_api_key("   "), "****", "先 trim");
        assert_eq!(mask_api_key("12345678"), "****", "8 位 ⇒ 整体打码");
        assert_eq!(
            mask_api_key("123456789"),
            "123****789",
            "9 位 ⇒ 首 3 + 末 3"
        );
        assert_eq!(
            mask_api_key("  abcdefghij  "),
            "abc****hij",
            "trim 后首 3 + 末 3"
        );
        // 生产 fixture 的钥匙串回显串本身打码后再打码 ⇒ 稳定（幂等）。
        assert_eq!(mask_api_key("sk-****abc"), "sk-****abc");
        // UTF-16 语义：一个 emoji 占 2 个 code unit ⇒ 「6 个字符 + 2 个 emoji」
        // 的长度是 10 > 8 ⇒ 走首 3 + 末 3 分支（末 3 code unit 落在 emoji 中间
        // 会切开代理对 ⇒ 与脚本侧的 `slice` 同口径，**不 panic**）。
        assert_eq!(mask_api_key("abcdef😀😀"), "abc****😀");
    }

    #[test]
    fn the_password_type_follows_the_show_toggle() {
        let props = props();
        let hidden = ProviderGroup::render(&props);
        let input = find_attr(&hidden, "input", "id", "provider-key-input").expect("有密钥输入");
        assert_eq!(
            input
                .attr_value("type")
                .and_then(|value| value.to_attr_string()),
            Some(String::from("password"))
        );

        props.show_api_key.set(true);
        let shown = ProviderGroup::render(&props);
        let input = find_attr(&shown, "input", "id", "provider-key-input").expect("有密钥输入");
        assert_eq!(
            input
                .attr_value("type")
                .and_then(|value| value.to_attr_string()),
            Some(String::from("text"))
        );
    }

    #[test]
    fn the_eye_toggle_emits_exactly_one_icon_node() {
        // ## 判据 T4（**结构断言**）
        //
        // 生产 `{#if showApiKey}<EyeOff/>{:else}<Eye/>{/if}` ⇒ **一次只产出一个**
        // 图标节点 ⇒ 本断言钉住「不是两个、不是零个」。
        let props = props();

        let hidden = ProviderGroup::render(&props);
        let toggle = find(&hidden, "button", "key-toggle-btn").expect("有明文切换按钮");
        assert_eq!(count_tag(toggle, "svg"), 1, "关闭态：恰好一个图标节点");
        assert!(text_of(toggle).is_empty(), "图标按钮里没有文本节点");
        let eye = find(&hidden, "svg", "lucide-eye");
        assert!(eye.is_some(), "关闭态渲染 `Eye`");
        assert!(
            find(&hidden, "svg", "lucide-eye-off").is_none(),
            "关闭态**没有** `EyeOff`"
        );

        props.show_api_key.set(true);
        let shown = ProviderGroup::render(&props);
        let toggle = find(&shown, "button", "key-toggle-btn").expect("有明文切换按钮");
        assert_eq!(count_tag(toggle, "svg"), 1, "明文态：恰好一个图标节点");
        assert!(
            find(&shown, "svg", "lucide-eye-off").is_some(),
            "明文态渲染 `EyeOff`"
        );
        assert!(
            find(&shown, "svg", "lucide-eye").is_none(),
            "明文态**没有** `Eye`"
        );
        assert_eq!(
            find(&shown, "button", "key-toggle-btn")
                .and_then(|node| node.attr_value("title"))
                .and_then(|value| value.to_attr_string()),
            Some(String::from(HIDE_KEY_TITLE))
        );
    }

    #[test]
    fn one_submit_intent_carries_the_whole_group() {
        // ## 判据 T5（**整组提交原子性**）
        //
        // 失焦 / 回车 ⇒ **一次**意图，载荷是**整组**（协议 / 预设 / 端点 /
        // 密钥 / 模型 / 协议头）⇒ **不执行**（宿主才落盘）。
        let props = props();
        props.api_key.set(String::from("provider-placeholder-key"));
        props.model.set(String::from("gpt-4o-mini"));
        let submitted: Rc<RefCell<Vec<(String, ProviderGroupDraft)>>> =
            Rc::new(RefCell::new(Vec::new()));
        let sink = submitted.clone();
        let props = ProviderGroupProps {
            on_submit_group: Rc::new(move |group: &str, draft: &ProviderGroupDraft| {
                sink.borrow_mut().push((String::from(group), draft.clone()));
            }),
            ..props
        };

        let tree = ProviderGroup::render(&props);
        let group = find(&tree, "div", "text-group").expect("有整组容器");
        call_handler(group, EventKind::FocusOut);
        call_handler(group, EventKind::KeyDown);

        let calls = submitted.borrow();
        // 两个入口（失焦 / 回车）各一次 ⇒ 每条都带**整组**。
        assert_eq!(calls.len(), 2, "两个提交入口各发一次整组意图");
        for (name, draft) in calls.iter() {
            assert_eq!(name, "provider-group");
            assert_eq!(draft.protocol, "openai");
            assert_eq!(draft.preset, "openai");
            assert_eq!(draft.base_url, "https://api.openai.com/v1");
            assert_eq!(draft.api_key, "provider-placeholder-key");
            assert_eq!(draft.model, "gpt-4o-mini");
            assert_eq!(draft.anthropic_version, "2023-06-01");
        }
        // 快照口径：**逐字段 trim**（纯空白差异不算改动）。
        let snapshot = calls[0].1.snapshot();
        assert!(
            snapshot.contains("\"baseUrl\":\"https://api.openai.com/v1\""),
            "{snapshot}"
        );
    }

    #[test]
    fn the_native_controls_carry_the_production_attributes() {
        // ## 判据 T7（原生控件属性：**逐字相等**、**存在性也对**）
        let props = props();
        let tree = ProviderGroup::render(&props);

        let url = find_attr(&tree, "input", "id", "provider-url-input").expect("端点输入");
        assert_eq!(
            url.attr_value("type")
                .and_then(|value| value.to_attr_string()),
            Some(String::from("text"))
        );
        assert_eq!(
            url.attr_value("placeholder")
                .and_then(|value| value.to_attr_string()),
            Some(String::from("https://api.openai.com/v1"))
        );
        // ⚠️ **生产用 property** ⇒ DOM 上**没有** `value` 属性。
        assert!(url.attr_value("value").is_none(), "不写 value 属性");

        let key = find_attr(&tree, "input", "id", "provider-key-input").expect("密钥输入");
        assert_eq!(
            key.attr_value("autocomplete")
                .and_then(|value| value.to_attr_string()),
            Some(String::from("off"))
        );
        assert!(key.attr_value("value").is_none(), "不写 value 属性");
        // 生产**没有** maxlength（≠ 印象正文的 2000）⇒ 存在性也要对。
        assert!(
            key.attr_value("maxlength").is_none(),
            "密钥框没有 maxlength"
        );

        // `disabled={false}` ⇒ 属性**不出现**（不是 `disabled=""`）。
        let test_button = find(&tree, "button", "primary-button").expect("测连接按钮");
        assert!(
            test_button.attr_value("disabled").is_none(),
            "未禁用时无 disabled 属性"
        );

        // 端点为空的 `!providerBaseUrl` 分支 ⇒ `disabled=""`（**空串**）。
        props.base_url.set(String::new());
        let tree = ProviderGroup::render(&props);
        let test_button = find(&tree, "button", "primary-button").expect("测连接按钮");
        assert_eq!(
            test_button
                .attr_value("disabled")
                .and_then(|value| value.to_attr_string()),
            Some(String::new()),
            "禁用态是空串"
        );
    }

    #[test]
    fn the_status_pill_follows_the_four_branches() {
        // 生产 `providerStatusLabel` 的四条分支 + 三个条件 class。
        assert_eq!(status_label(None, ""), STATUS_NO_KEY);
        assert_eq!(status_label(None, "  "), STATUS_NO_KEY, "纯空白 ⇒ 未配置");
        assert_eq!(status_label(None, "x"), STATUS_UNTESTED);
        let ok = TestResult {
            ok: true,
            ..TestResult::default()
        };
        assert_eq!(status_label(Some(&ok), "x"), STATUS_CONNECTED);
        let failed = TestResult {
            ok: false,
            ..TestResult::default()
        };
        assert_eq!(status_label(Some(&failed), "x"), STATUS_FAILED);

        assert_eq!(status_classes(None), vec!["status-pill", "idle"]);
        assert_eq!(status_classes(Some(&ok)), vec!["status-pill", "ok"]);
        assert_eq!(status_classes(Some(&failed)), vec!["status-pill", "warn"]);
    }

    #[test]
    fn the_summary_falls_back_like_the_production_template() {
        let props = props();
        let tree = ProviderGroup::render(&props);
        let summary = find(&tree, "div", "provider-summary").expect("有摘要条");
        let text = text_of(summary);
        assert!(text.contains("OpenAI 兼容"), "{text}");
        assert!(text.contains("OpenAI 官方"), "{text}");
        assert!(text.contains("gpt-4o"), "{text}");
        assert!(text.contains("未配置密钥"), "{text}");
        assert!(text.contains("https://api.openai.com/v1"), "{text}");

        // 模型空 ⇒ 「未指定模型」；端点空 ⇒ em dash；预设查不到 ⇒ 「自定义」。
        props.model.set(String::new());
        props.base_url.set(String::new());
        props.preset.set(String::from("no-such-preset"));
        let tree = ProviderGroup::render(&props);
        let text = text_of(find(&tree, "div", "provider-summary").expect("有摘要条"));
        assert!(text.contains(NO_MODEL_TEXT), "{text}");
        assert!(text.contains(NO_URL_TEXT), "{text}");
        assert!(text.contains(CUSTOM_PRESET_NAME), "{text}");
    }

    #[test]
    fn the_protocol_switches_the_labels_and_the_extra_field() {
        let props = props();
        props.protocol.set(String::from("anthropic"));
        props.preset.set(String::from("anthropic"));
        let tree = ProviderGroup::render(&props);
        // 密钥标签随协议变。
        let label = find_attr(&tree, "label", "for", "provider-key-input").expect("密钥标签");
        assert_eq!(text_of(label), KEY_LABEL_ANTHROPIC);
        // `anthropic-version` 那一组只在 Anthropic 协议下出现。
        assert!(
            find_attr(&tree, "input", "id", "anthropic-ver-input").is_some(),
            "Anthropic 协议有协议头输入"
        );
        let placeholder = find_attr(&tree, "input", "id", "provider-url-input")
            .and_then(|node| node.attr_value("placeholder"))
            .and_then(|value| value.to_attr_string());
        assert_eq!(placeholder, Some(String::from(URL_PLACEHOLDER_ANTHROPIC)));

        props.protocol.set(String::from("openai"));
        let tree = ProviderGroup::render(&props);
        assert!(
            find_attr(&tree, "input", "id", "anthropic-ver-input").is_none(),
            "OpenAI 协议没有协议头输入"
        );
    }

    #[test]
    fn the_presets_come_from_the_table_not_from_the_component() {
        // **一个组件 + 一张表**：组件里没有一行预设定义 ⇒ 换表即换芯片。
        let props = props();
        let tree = ProviderGroup::render(&props);
        let row = find(&tree, "div", "presets-row").expect("有预设行");
        let mut elements: Vec<&VNode> = Vec::new();
        flatten(row, &mut elements);
        let chips: Vec<&VNode> = elements
            .into_iter()
            .filter(|node| {
                node.class_tokens()
                    .iter()
                    .any(|token| token == "preset-chip")
            })
            .collect();
        assert_eq!(chips.len(), 2, "测试表两条 ⇒ 两枚芯片");
        assert!(
            chips[0]
                .class_tokens()
                .iter()
                .any(|token| token == "selected"),
            "当前预设选中"
        );
    }

    #[test]
    fn a_preset_click_marks_dirty_without_reading_the_dom_values() {
        // ## 这条是**实测踩出来的回归断言**
        //
        // 生产：选预设 = `selectPreset(p.id)` + `markProviderDirty([...])`
        // ⇒ 只标「未保存」，**不回读输入框**（值由代码写进绑定变量）。
        // ⇒ 若点选也走「回读 DOM」那条，宿主会用**旧的 DOM 值**覆盖刚写进去的
        // 新端点/新模型（实测：选预设后端点与模型被打回旧值 ⇒ 判据报 5 条差异）。
        let props = props();
        let events: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));
        let actions = events.clone();
        let dirty = events.clone();
        let field = events.clone();
        let props = ProviderGroupProps {
            on_action: Rc::new(move |name: &str| {
                actions.borrow_mut().push(format!("action:{name}"))
            }),
            on_mark_dirty: Rc::new(move |keys: &str| {
                dirty.borrow_mut().push(format!("dirty:{keys}"))
            }),
            on_field_input: Rc::new(move |keys: &str| {
                field.borrow_mut().push(format!("field:{keys}"))
            }),
            ..props
        };
        let tree = ProviderGroup::render(&props);
        let row = find(&tree, "div", "presets-row").expect("有预设行");
        let mut elements: Vec<&VNode> = Vec::new();
        flatten(row, &mut elements);
        let chip = elements
            .into_iter()
            .find(|node| {
                node.tag() == Some("button")
                    && node
                        .class_tokens()
                        .iter()
                        .any(|token| token == "preset-chip")
            })
            .expect("有预设芯片");
        call_handler(chip, EventKind::Click);
        assert_eq!(
            events.borrow().as_slice(),
            [
                String::from("action:preset:openai"),
                format!("dirty:{PRESET_DIRTY_KEYS}")
            ],
            "点选 = 一个动作意图 + 一个「只标未保存」意图（**没有**回读意图）"
        );
    }

    #[test]
    fn the_protocol_click_marks_all_four_text_keys() {
        let props = props();
        let events: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));
        let actions = events.clone();
        let dirty = events.clone();
        let props = ProviderGroupProps {
            on_action: Rc::new(move |name: &str| {
                actions.borrow_mut().push(format!("action:{name}"))
            }),
            on_mark_dirty: Rc::new(move |keys: &str| {
                dirty.borrow_mut().push(format!("dirty:{keys}"))
            }),
            ..props
        };
        let tree = ProviderGroup::render(&props);
        let mut elements: Vec<&VNode> = Vec::new();
        flatten(&tree, &mut elements);
        let tab = elements
            .into_iter()
            .find(|node| {
                node.tag() == Some("button")
                    && node
                        .class_tokens()
                        .iter()
                        .any(|token| token == "protocol-tab")
            })
            .expect("有协议页签");
        call_handler(tab, EventKind::Click);
        assert_eq!(
            events.borrow().as_slice(),
            [
                String::from("action:protocol:openai"),
                format!("dirty:{PROVIDER_TEXT_KEYS}")
            ]
        );
    }

    #[test]
    fn the_gateway_key_row_follows_the_configured_flag() {
        // 生产 `hasApiKey ? '已配置 (Configured)' : '未配置 (Not configured)'`
        // 与 `hasApiKey ? '更换 Key' : '配置 Key'`（**两条都要逐字对**）。
        let props = props();
        props.show_advanced_gateway.set(true);
        let tree = ProviderGroup::render(&props);
        let text = text_of(&tree);
        assert!(text.contains(GATEWAY_KEY_NOT_CONFIGURED), "{text}");
        assert!(text.contains(CONFIGURE_KEY_LABEL), "{text}");

        let configured = ProviderGroupProps {
            has_api_key: true,
            ..props
        };
        let tree = ProviderGroup::render(&configured);
        let text = text_of(&tree);
        assert!(text.contains(GATEWAY_KEY_CONFIGURED), "{text}");
        assert!(text.contains(CHANGE_KEY_LABEL), "{text}");
        assert!(
            !text.contains(CONFIGURE_KEY_LABEL),
            "配置好之后不再显示「配置 Key」"
        );
    }

    #[test]
    fn a_failed_test_result_switches_the_icon_and_the_box_class() {
        // 生产失败分支：`<XCircle class="head-icon failed-icon">` + `class:failed`
        // ⇒ **另一个图标节点**（`circle-x`，生产 import 名 `XCircle` 是别名）。
        let props = props();
        props.test_result.set(Some(TestResult {
            ok: false,
            message: String::from("连接失败：401 Unauthorized"),
            latency_ms: None,
            models: Vec::new(),
        }));
        let tree = ProviderGroup::render(&props);
        let result_box = find(&tree, "div", "test-result-box").expect("有回执框");
        assert!(result_box
            .class_tokens()
            .iter()
            .any(|token| token == "failed"));
        assert!(find(&tree, "svg", "lucide-circle-x").is_some(), "失败图标");
        assert!(
            find(&tree, "svg", "lucide-circle-check").is_none(),
            "不是成功图标"
        );
        assert!(text_of(&tree).contains(TEST_FAIL_TITLE));
        // 没有 `latencyMs` ⇒ **不渲染**延迟徽标（属性缺失 ≠ 空串）。
        assert!(find(&tree, "span", "latency-badge").is_none(), "无延迟徽标");
        // 没有远端模型 ⇒ 不渲染模型目录。
        assert!(find(&tree, "div", "discovered-models").is_none());
    }

    #[test]
    fn the_advanced_box_toggles_and_opens_the_modal_path() {
        let props = props();
        let tree = ProviderGroup::render(&props);
        assert!(
            find(&tree, "div", "advanced-body").is_none(),
            "默认折叠 ⇒ 不渲染展开体"
        );
        let toggled: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));
        let sink = toggled.clone();
        let props = ProviderGroupProps {
            on_action: Rc::new(move |action: &str| sink.borrow_mut().push(String::from(action))),
            ..props
        };
        let tree = ProviderGroup::render(&props);
        let toggle = find(&tree, "button", "advanced-toggle").expect("有折叠头");
        call_handler(toggle, EventKind::Click);
        assert_eq!(toggled.borrow().as_slice(), ["toggle-advanced"]);

        // 展开后：网关地址 + 认证密钥 + 「配置 Key」按钮（`hasApiKey=false`）。
        props.show_advanced_gateway.set(true);
        let tree = ProviderGroup::render(&props);
        assert!(
            find(&tree, "div", "advanced-body").is_some(),
            "展开 ⇒ 有展开体"
        );
        assert!(
            find_attr(&tree, "input", "id", "endpoint-input").is_some(),
            "有网关地址输入"
        );
        let body = find(&tree, "div", "advanced-body").expect("有展开体");
        let mut elements: Vec<&VNode> = Vec::new();
        flatten(body, &mut elements);
        let open = elements
            .into_iter()
            .find(|node| {
                node.tag() == Some("button")
                    && node
                        .class_tokens()
                        .iter()
                        .any(|token| token == "quiet-button")
            })
            .expect("有配置 Key 按钮");
        assert_eq!(text_of(open), CONFIGURE_KEY_LABEL);
        call_handler(open, EventKind::Click);
        assert_eq!(
            toggled.borrow().last().map(String::as_str),
            Some("open-api-key-modal")
        );
    }

    #[test]
    fn the_api_key_modal_renders_the_keychain_branches() {
        let props = props();
        // 关着 ⇒ 模态**一个节点都不产出**（块仍在）。
        let closed = ProviderGroup::render(&props);
        assert!(
            find(&closed, "div", "modal-backdrop").is_none(),
            "关着 ⇒ 没有遮罩"
        );
        assert!(find(&closed, "div", "setting-block").is_some(), "块始终在");

        props.show_api_key_modal.set(true);
        let tree = ProviderGroup::render(&props);
        let backdrop = find(&tree, "div", "modal-backdrop").expect("有遮罩");
        assert_eq!(
            backdrop
                .attr_value("role")
                .and_then(|value| value.to_attr_string()),
            Some(String::from("presentation"))
        );
        let dialog = find(&tree, "div", "modal-dialog").expect("有对话框");
        assert_eq!(
            dialog
                .attr_value("aria-modal")
                .and_then(|value| value.to_attr_string()),
            Some(String::from("true"))
        );
        assert_eq!(
            dialog
                .attr_value("tabindex")
                .and_then(|value| value.to_attr_string()),
            Some(String::from("-1"))
        );
        let head = text_of(find_attr(&tree, "h3", "id", "api-key-dialog-title").expect("有标题"));
        assert_eq!(head, MODAL_TITLE);
        // 已存密钥那一支（fixture：storedKeyExists=true ⇒ 打码串 + 删除按钮）。
        assert!(text_of(&tree).contains("sk-****abc (已存钥匙串)"));
        assert!(
            find(&tree, "button", "delete-key-btn").is_some(),
            "有删除按钮"
        );

        // 读取中 ⇒ 「正在读取系统钥匙串…」（生产读之前把 `storedKeyExists`
        // 置 false ⇒ 这一态**没有**删除按钮）。
        props.keychain_loading.set(true);
        props.stored_key_exists.set(false);
        let tree = ProviderGroup::render(&props);
        assert!(text_of(&tree).contains(KEYCHAIN_LOADING_TEXT));
        assert!(
            find(&tree, "button", "delete-key-btn").is_none(),
            "还没读到 ⇒ 不渲染删除"
        );

        // 暂无 ⇒ 空文案；且**不**渲染删除按钮。
        props.keychain_loading.set(false);
        props.stored_key_exists.set(false);
        let tree = ProviderGroup::render(&props);
        assert!(text_of(&tree).contains(KEYCHAIN_EMPTY_TEXT));
        assert!(find(&tree, "button", "delete-key-btn").is_none());
    }

    #[test]
    fn the_modal_footer_disables_while_saving_and_deleting() {
        let props = props();
        props.show_api_key_modal.set(true);
        let tree = ProviderGroup::render(&props);
        let footer = find(&tree, "div", "modal-footer").expect("有页脚");
        let save = find(footer, "button", "primary-button").expect("有写入按钮");
        assert!(save.attr_value("disabled").is_none());

        props.keychain_saving.set(true);
        let tree = ProviderGroup::render(&props);
        let footer = find(&tree, "div", "modal-footer").expect("有页脚");
        let save = find(footer, "button", "primary-button").expect("有写入按钮");
        assert_eq!(
            save.attr_value("disabled")
                .and_then(|value| value.to_attr_string()),
            Some(String::new())
        );
        assert!(text_of(save).contains(SAVING_KEY_LABEL));

        props.keychain_saving.set(false);
        props.keychain_deleting.set(true);
        let tree = ProviderGroup::render(&props);
        let delete = find(&tree, "button", "delete-key-btn").expect("有删除按钮");
        assert_eq!(
            delete
                .attr_value("disabled")
                .and_then(|value| value.to_attr_string()),
            Some(String::new())
        );
        assert!(text_of(delete).contains(DELETING_KEY_LABEL));
    }

    #[test]
    fn the_test_result_renders_the_catalog_with_the_24_limit() {
        let props = props();
        let models: Vec<String> = (0..30).map(|index| format!("model-{index:02}")).collect();
        props.test_result.set(Some(TestResult {
            ok: true,
            message: String::from("连接成功，已拉取 30 个模型。"),
            latency_ms: Some(128.0),
            models,
        }));
        let tree = ProviderGroup::render(&props);
        let result_box = find(&tree, "div", "test-result-box").expect("有回执框");
        assert!(result_box
            .class_tokens()
            .iter()
            .any(|token| token == "success"));
        assert!(text_of(&tree).contains("128 ms"), "延迟徽标");
        let catalog = find(&tree, "div", "model-catalog").expect("有目录");
        let mut elements: Vec<&VNode> = Vec::new();
        flatten(catalog, &mut elements);
        let items = elements
            .into_iter()
            .filter(|node| node.tag() == Some("button"))
            .count();
        assert_eq!(items, MODEL_CATALOG_LIMIT, "只展示前 24 项");
        assert!(text_of(&tree).contains(MORE_MODELS_TEXT), "有截断提示");

        // 搜索词过滤 ⇒ 0 条 ⇒ 「无匹配模型」。
        props.model_search_query.set(String::from("no-such-model"));
        let tree = ProviderGroup::render(&props);
        assert!(text_of(&tree).contains(CATALOG_EMPTY_TEXT));
        assert!(find(&tree, "p", "more-models").is_none(), "过滤后不截断");
    }

    #[test]
    fn the_warnings_box_needs_a_non_empty_warning_list() {
        let props = props();
        props.apply_result.set(Some(ApplyResult {
            ok: true,
            warnings: Vec::new(),
        }));
        let tree = ProviderGroup::render(&props);
        assert!(
            find(&tree, "div", "warnings-box").is_none(),
            "空警告 ⇒ 不渲染"
        );

        props.apply_result.set(Some(ApplyResult {
            ok: true,
            warnings: vec![String::from("网关热应用未确认。")],
        }));
        let tree = ProviderGroup::render(&props);
        let warnings_box = find(&tree, "div", "warnings-box").expect("有警告框");
        assert!(text_of(warnings_box).contains("配置已应用，但有 1 条警告："));
        assert!(text_of(warnings_box).contains("网关热应用未确认。"));
    }

    #[test]
    fn the_effective_config_renders_the_missing_fields_as_dashes() {
        let props = props();
        props.effective_config.set(Some(EffectiveConfig {
            provider: Some(String::from("openai")),
            base_url: None,
            api_key: None,
            model: Some(String::from("gpt-4o")),
        }));
        let tree = ProviderGroup::render(&props);
        let card = find(&tree, "div", "effective-config").expect("有生效配置卡");
        let text = text_of(card);
        assert!(text.contains("Provider: openai"), "{text}");
        assert!(text.contains("Base URL: —"), "{text}");
        assert!(text.contains("API Key: 未配置"), "{text}");
    }

    #[test]
    fn the_model_picker_placeholder_is_the_closed_state() {
        // `SessionModelPicker` **不迁** ⇒ 只产闭合态：trigger + value + chevron。
        let props = props();
        let tree = ProviderGroup::render(&props);
        let picker = find(&tree, "div", "model-picker").expect("有选择器占位");
        let trigger = find(picker, "button", "trigger").expect("有触发器");
        assert_eq!(
            trigger
                .attr_value("aria-haspopup")
                .and_then(|value| value.to_attr_string()),
            Some(String::from("listbox"))
        );
        assert_eq!(
            trigger
                .attr_value("aria-expanded")
                .and_then(|value| value.to_attr_string()),
            Some(String::from("false"))
        );
        assert_eq!(
            trigger
                .attr_value("title")
                .and_then(|value| value.to_attr_string()),
            Some(String::from("gpt-4o"))
        );
        assert!(text_of(trigger).contains("gpt-4o"));
        // 关闭态没有下拉（**未迁** ⇒ 不假装有）。
        assert!(find(picker, "div", "dropdown").is_none(), "没有下拉弹层");
        // 模型空 ⇒ 用默认模型 `deepseek-v4-flash` 顶替（生产 `value || DEFAULT_MODEL_ID`）。
        props.model.set(String::new());
        let tree = ProviderGroup::render(&props);
        let trigger = find(&tree, "button", "trigger").expect("有触发器");
        assert_eq!(
            trigger
                .attr_value("title")
                .and_then(|value| value.to_attr_string()),
            Some(String::from(DEFAULT_MODEL_ID))
        );
        // `modelsLoading` ⇒ `.model-picker.disabled` + `disabled=""`。
        props.models_loading.set(true);
        let tree = ProviderGroup::render(&props);
        let picker = find(&tree, "div", "model-picker").expect("有选择器占位");
        assert!(picker
            .class_tokens()
            .iter()
            .any(|token| token == "disabled"));
        let trigger = find(picker, "button", "trigger").expect("有触发器");
        assert_eq!(
            trigger
                .attr_value("disabled")
                .and_then(|value| value.to_attr_string()),
            Some(String::new())
        );
    }

    #[test]
    fn the_preset_selection_rule_matches_the_production_mirror() {
        // 生产 `selectPreset` 的模型联动（**含三个写死的模型名**）。
        let preset = ProviderPreset {
            id: String::from("deepseek"),
            name: String::from("DeepSeek"),
            base_url: String::from("https://api.deepseek.com/v1"),
            default_model: String::from("deepseek-v4-flash"),
            models: vec![
                String::from("deepseek-v4-flash"),
                String::from("deepseek-chat"),
            ],
        };
        // 空模型 ⇒ 换成默认模型。
        assert_eq!(
            preset_model_after_select(&preset, ""),
            Some(String::from("deepseek-v4-flash"))
        );
        // 旧预设的模型 ⇒ 换成默认模型。
        assert_eq!(
            preset_model_after_select(&preset, "deepseek-chat"),
            Some(String::from("deepseek-v4-flash"))
        );
        // 写死的 `gpt-4o` ⇒ 也换。
        assert_eq!(
            preset_model_after_select(&preset, "gpt-4o"),
            Some(String::from("deepseek-v4-flash"))
        );
        // 用户自定义的模型 ⇒ **不动**。
        assert_eq!(preset_model_after_select(&preset, "my-finetune-001"), None);
        // `custom` 预设 ⇒ 端点与模型都不动（生产 `p.id !== 'custom'`）。
        let custom = ProviderPreset {
            id: String::from("custom"),
            ..preset.clone()
        };
        assert_eq!(preset_model_after_select(&custom, ""), None);
    }

    #[test]
    fn the_family_matches_the_production_three_branches() {
        assert_eq!(provider_family("anthropic", "openai"), "anthropic");
        assert_eq!(provider_family("openai", "minimax"), "minimax");
        assert_eq!(provider_family("openai", "deepseek"), "openai");
        assert_eq!(provider_family("openai", "openai"), "openai");
    }

    #[test]
    fn the_model_search_is_case_insensitive_and_trims() {
        let models = vec![
            String::from("GPT-4o"),
            String::from("gpt-4o-mini"),
            String::from("o3-mini"),
        ];
        assert_eq!(filter_models(&models, "").len(), 3);
        assert_eq!(filter_models(&models, "   ").len(), 3, "纯空白 ⇒ 不过滤");
        assert_eq!(
            filter_models(&models, " mini "),
            vec!["gpt-4o-mini", "o3-mini"]
        );
        assert!(filter_models(&models, "nope").is_empty());
    }

    #[test]
    fn the_component_renders_both_regions_like_the_production_structure() {
        // 生产：块**始终在**；模态是 `{#if}` 覆盖层（`position: fixed`）。
        let props = props();
        let tree = ProviderGroup::render(&props);
        assert!(find(&tree, "div", "setting-block").is_some(), "块始终在");
        assert!(find(&tree, "div", "modal-backdrop").is_none(), "模态关着");
        props.show_api_key_modal.set(true);
        let tree = ProviderGroup::render(&props);
        assert!(
            find(&tree, "div", "setting-block").is_some(),
            "模态打开时块仍在（遮罩透出它）"
        );
        assert!(find(&tree, "div", "modal-backdrop").is_some(), "模态打开");
        // 遮罩与块是**兄弟**（模态不在 `.setting-block` 里面）。
        assert_eq!(tree.children().len(), 2, "块 + 模态两个区域");
    }

    #[test]
    fn the_draft_snapshot_trims_every_field() {
        let draft = ProviderGroupDraft {
            protocol: String::from("openai"),
            preset: String::from("openai"),
            base_url: String::from("  https://api.openai.com/v1  "),
            api_key: String::from("  key  "),
            model: String::from("  gpt-4o  "),
            anthropic_version: String::from("  2023-06-01  "),
        };
        let snapshot = draft.snapshot();
        assert!(
            snapshot.contains("\"baseUrl\":\"https://api.openai.com/v1\""),
            "{snapshot}"
        );
        assert!(snapshot.contains("\"apiKey\":\"key\""), "{snapshot}");
        assert!(snapshot.contains("\"model\":\"gpt-4o\""), "{snapshot}");
        assert!(
            snapshot.contains("\"anthropicVersion\":\"2023-06-01\""),
            "{snapshot}"
        );
        assert_eq!(draft.trimmed_base_url(), "https://api.openai.com/v1");
        assert_eq!(draft.trimmed_model(), "gpt-4o");
        assert_eq!(draft.trimmed_anthropic_version(), "2023-06-01");
    }

    #[test]
    fn the_icon_table_knows_the_fourteen_import_names() {
        // 生产 import 名 ⇒ 真实 class（**别名要跟到真实文件**）。
        let cases = [
            ("Server", "lucide-server"),
            ("Globe", "lucide-globe"),
            ("Sparkles", "lucide-sparkles"),
            ("Eye", "lucide-eye"),
            ("EyeOff", "lucide-eye-off"),
            ("Lock", "lucide-lock"),
            ("Check", "lucide-check"),
            ("CheckCircle2", "lucide-circle-check"),
            ("XCircle", "lucide-circle-x"),
            ("ChevronDown", "lucide-chevron-down"),
            ("ChevronUp", "lucide-chevron-up"),
            ("Search", "lucide-search"),
            ("RotateCcw", "lucide-rotate-ccw"),
            ("Trash2", "lucide-trash-2"),
        ];
        for (import_name, class) in cases {
            let data = icons::icon_by_import_name(import_name)
                .unwrap_or_else(|| panic!("未知图标：{import_name}"));
            assert_eq!(
                data.class,
                format!("lucide-icon lucide {class}"),
                "{import_name}"
            );
        }
        assert!(
            icons::icon_by_import_name("NoSuchIcon").is_none(),
            "未知 ⇒ 显式 None"
        );
    }

    #[test]
    fn every_text_constant_is_the_production_text() {
        // 关键文案逐字（**尾随空格是数据** ⇒ 单独钉住）。
        assert_eq!(URL_LABEL, "API 端点 (Base URL) ");
        assert_eq!(MODEL_LABEL, "活动模型 ID ");
        assert_eq!(KEY_LABEL_OPENAI, "API Key (Bearer) ");
        assert_eq!(KEY_LABEL_ANTHROPIC, "API Key (x-api-key) ");
        assert_eq!(ANTHROPIC_VERSION_LABEL, "anthropic-version ");
        assert!(MODAL_DESC.contains("**模型提供商**"));
        assert_eq!(NO_URL_TEXT, "—");
        assert_eq!(DEFAULT_MODEL_ID, "deepseek-v4-flash");
    }
}
