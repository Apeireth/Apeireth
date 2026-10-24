//! 设置页三页 · `capability` 子面（**能力与治理**）的组件模块。
//!
//! ## 本面是什么
//!
//! 生产 `SettingsView.svelte` 的**能力与治理**三个分区（markup 2864–3021 /
//! 3023–3199 / 3567–3680，**448 行**，可迁 432 行）：
//!
//! | 分区 | 内容 | 生产行区间 |
//! |---|---|---|
//! | `governance` | 决策与治理：认知深度预设 + 2 张能力卡（评审/议会、权限与子代理）+ 设计说明卡 | 2864–3021 |
//! | `tools` | 工具与安全：允许连接的自建服务器 + 全局权限预设 + 工具授予卡（含**嵌套行**与 **MCP 静态行**）+ 2 张常驻说明卡 | 3023–3199 |
//! | `developer` | 开发者选项：Beta 能力卡（含**思考模式文本旋钮组**）+ 契约说明卡 + 客户端配置 JSON | 3567–3680 |
//!
//! ⚠️ **`security` 分区（3006–3021）不迁**：那一段整块委派给 `GovernanceView`
//! （145 行、独立面）⇒ 不属本面 ⇒ 本模块**没有** `security` 分支。
//!
//! ## 复用（**不重造**）
//!
//! 开关行（`button.cap-row` + `role="switch"` + `aria-checked` + `dim` /
//! `pending` 条件 class + `disabled=""`）**全部**由人格与记忆面（M6）冻结的
//! 行组件 [`crate::settings_character::render_capability_rows`] 渲染 ⇒ 本模块
//! **没有**第二份行渲染代码。
//!
//! 卡壳（`div.cap-card` + `cap-card-head` + 行之间一个空格）由本模块**自建**
//! （十几行）：本面四张卡的**行组合**都是生产特有的（认知深度行 + 定义行 +
//! 条件行 / 定义行 + 嵌套行 + 告警 + MCP 静态行 / 定义行 + 文本旋钮组）⇒
//! M6 的 `render_capability_card` 只覆盖「纯定义表」那一种形态 ⇒ 套用会**漏行**。
//!
//! ⚠️ 冻结的行组件吃的是**它自己的** props 类型（[`CharacterPanelsProps`]）⇒
//! 本模块用 [`card_bridge`] 做一层**只填两个意图出口**的适配（其余字段空值，
//! 行组件不读它们）⇒ **不改冻结文件**。
//!
//! ## 逐字照抄的**真值口径**（全部来自生产 DOM 的**浏览器实测 dump**）
//!
//! | 事项 | 口径 |
//! |---|---|
//! | `select` / `option` | **原生元素**；`option` 的 `value` 是**静态属性**；`select` 的取值是 **property**（生产 `bind:value`）⇒ 引擎**不写** `value` 属性，由组件用 `prop("value", …)` **声明**、引擎写入 |
//! | `input[type=range]` / `[type=number]` | 取值是 **property** ⇒ 不写 `value` 属性；`number` 行**没有 `max`**（生产 `max={… ?? undefined}` ⇒ **属性不出现**，不是 `max=""`） |
//! | `input[type=text]` | 取值是 **property**；`placeholder` 是静态属性（第二个文本旋钮**没有** placeholder） |
//! | 静态 `disabled` | DOM 上是 **`disabled=""`**（不是属性缺失、也不是 `disabled="true"`） |
//! | 开关的「开」 | `aria-checked="true"` / `"false"`（**文本**，不是布尔属性） |
//! | `class:on` | `<span class="cap-switch on">`（`on` 是**追加** token，不是替换） |
//! | 沙箱嵌套行 | 沙箱**关** ⇒ `class="cap-row cap-row-nested sandbox-off"` + 图标换成 `shield-off`；沙箱**开** ⇒ `class="cap-row cap-row-nested"` + `shield-check`。**两个态都没有 `dim`** |
//! | 依赖子行 | `fileWriteAutoPass` ← `fileWrite`：父行开 ⇒ `class="cap-row cap-row-nested"`；父行关 ⇒ `class="cap-row cap-row-nested dim"` + `disabled=""` |
//! | **MCP 静态行** | `{#each TOOL_DEFS}` **之后**的 `div.cap-row.cap-row-static` ⇒ **必须渲染**：否则依赖子行成了 `:last-child` ⇒ 生产 `.cap-row:last-child{border-bottom:0}` ⇒ 引擎少 1px 下边框 ⇒ **高度差 1px** |
//! | 空白 | 元素之间的换行**折叠成恰好一个空格**；**文本节点内部**的换行 + 源文件缩进**逐字保留**（`\n` + 缩进是**数据**，见各 `*_DESC` / `*_TEXT` 常量） |
//! | 卡头标题 | 图标与标题之间**有一个空格**（生产 `<Users size={13} /> 评审与议会`）⇒ 标题常量带**前导空格** |
//! | 徽标 | `cap-default-badge note` / `cap-default-badge on` 内部的文案**带前导空格**（同上） |
//! | 摘要行 | 常量与表达式**必须在同一个文本节点里**（生产 `<span>本页已授予 <b>…</b> / 4 类工具 · 沙箱{…}</span>` ⇒ 后半截是**一个**文本节点）⇒ 渲染时**先拼成整串**再落一个文本节点（否则归一化后是两个相邻文本节点 ⇒ 判据报差异） |
//!
//! ## 状态归属（**引擎不迁**的部分）
//!
//! 能力开关的 apply、认知深度预设的落盘、全局权限预设的 `localStorage`、
//! 允许服务器的增删（HTTP）、配置 JSON 的复制（剪贴板）⇒ 全部是**宿主侧**的事
//! ⇒ 本组件只承接**展示面 + 受控输入** ⇒ 点击 / 输入 ⇒ **发意图**
//! （见 [`CapabilityPanelsProps`] 的各出口），**不自己改状态**。
//!
//! ## 图标数据
//!
//! 本面用的图标几何数据是**构建期抽取**的**生成物**（上游图标包 ⇒ 真实
//! `.svelte` ⇒ `iconNode`）⇒ 见 [`icons`] 模块。class 后缀取**真实文件名**
//! （`CheckCircle2` ⇒ `circle-check`；`AlertTriangle` ⇒ `triangle-alert`）。

use std::rc::Rc;

use crate::icon::{Icon, IconData, IconProps};
use crate::settings_character::{
    capability_bool, capability_number, enabled_count, render_capability_rows, ActionIntent,
    CapabilityDef, CapabilityIntent, CapabilityTable, CharacterPanelsProps, Impression, KnobIntent,
    TuningLogEntry,
};
use crate::settings_frame::{FieldFlag, FieldFlagProps, FieldFlagState};
use apeireth_ui_core::{Runtime, Signal, UiComponent};
use apeireth_ui_dom::{DomEvent, ElementHandler, EventKind, VNode};
use apeireth_ui_macro::view;

/// 本面用到的图标数据（**自动生成** —— 勿手改）。
///
/// 来源：`frontend/companion-desktop/node_modules/lucide-svelte/dist/icons/*.svelte`
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
/// MERCHANTABILITY AND FITNESS. IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR
/// ANY SPECIAL, DIRECT, INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES
/// WHATSOEVER RESULTING FROM LOSS OF USE, DATA OR PROFITS, WHETHER IN AN
/// ACTION OF CONTRACT, NEGLIGENCE OR OTHER TORTIOUS ACTION, ARISING OUT OF
/// OR IN CONNECTION WITH THE USE OR PERFORMANCE OF THIS SOFTWARE.
///
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

    /// 图标 `triangle-alert`。
#[rustfmt::skip]
pub const ICON_TRIANGLE_ALERT: IconData = IconData {
    name: "triangle-alert",
    class: "lucide-icon lucide lucide-triangle-alert",
    nodes: &[
        IconNode { tag: "path", attrs: &[("d", "m21.73 18-8-14a2 2 0 0 0-3.48 0l-8 14A2 2 0 0 0 4 21h16a2 2 0 0 0 1.73-3")] },
        IconNode { tag: "path", attrs: &[("d", "M12 9v4")] },
        IconNode { tag: "path", attrs: &[("d", "M12 17h.01")] },
    ],
};

    /// 图标 `circle-check`。
#[rustfmt::skip]
pub const ICON_CIRCLE_CHECK: IconData = IconData {
    name: "circle-check",
    class: "lucide-icon lucide lucide-circle-check",
    nodes: &[
        IconNode { tag: "circle", attrs: &[("cx", "12"), ("cy", "12"), ("r", "10")] },
        IconNode { tag: "path", attrs: &[("d", "m9 12 2 2 4-4")] },
    ],
};

    /// 图标 `info`。
#[rustfmt::skip]
pub const ICON_INFO: IconData = IconData {
    name: "info",
    class: "lucide-icon lucide lucide-info",
    nodes: &[
        IconNode { tag: "circle", attrs: &[("cx", "12"), ("cy", "12"), ("r", "10")] },
        IconNode { tag: "path", attrs: &[("d", "M12 16v-4")] },
        IconNode { tag: "path", attrs: &[("d", "M12 8h.01")] },
    ],
};

    /// 图标 `brain`。
#[rustfmt::skip]
pub const ICON_BRAIN: IconData = IconData {
    name: "brain",
    class: "lucide-icon lucide lucide-brain",
    nodes: &[
        IconNode { tag: "path", attrs: &[("d", "M12 5a3 3 0 1 0-5.997.125 4 4 0 0 0-2.526 5.77 4 4 0 0 0 .556 6.588A4 4 0 1 0 12 18Z")] },
        IconNode { tag: "path", attrs: &[("d", "M12 5a3 3 0 1 1 5.997.125 4 4 0 0 1 2.526 5.77 4 4 0 0 1-.556 6.588A4 4 0 1 1 12 18Z")] },
        IconNode { tag: "path", attrs: &[("d", "M15 13a4.5 4.5 0 0 1-3-4 4.5 4.5 0 0 1-3 4")] },
        IconNode { tag: "path", attrs: &[("d", "M17.599 6.5a3 3 0 0 0 .399-1.375")] },
        IconNode { tag: "path", attrs: &[("d", "M6.003 5.125A3 3 0 0 0 6.401 6.5")] },
        IconNode { tag: "path", attrs: &[("d", "M3.477 10.896a4 4 0 0 1 .585-.396")] },
        IconNode { tag: "path", attrs: &[("d", "M19.938 10.5a4 4 0 0 1 .585.396")] },
        IconNode { tag: "path", attrs: &[("d", "M6 18a4 4 0 0 1-1.967-.516")] },
        IconNode { tag: "path", attrs: &[("d", "M19.967 17.484A4 4 0 0 1 18 18")] },
    ],
};

    /// 图标 `scale`。
#[rustfmt::skip]
pub const ICON_SCALE: IconData = IconData {
    name: "scale",
    class: "lucide-icon lucide lucide-scale",
    nodes: &[
        IconNode { tag: "path", attrs: &[("d", "m16 16 3-8 3 8c-.87.65-1.92 1-3 1s-2.13-.35-3-1Z")] },
        IconNode { tag: "path", attrs: &[("d", "m2 16 3-8 3 8c-.87.65-1.92 1-3 1s-2.13-.35-3-1Z")] },
        IconNode { tag: "path", attrs: &[("d", "M7 21h10")] },
        IconNode { tag: "path", attrs: &[("d", "M12 3v18")] },
        IconNode { tag: "path", attrs: &[("d", "M3 7h2c2 0 5-1 7-2 2 1 5 2 7 2h2")] },
    ],
};

    /// 图标 `wrench`。
#[rustfmt::skip]
pub const ICON_WRENCH: IconData = IconData {
    name: "wrench",
    class: "lucide-icon lucide lucide-wrench",
    nodes: &[
        IconNode { tag: "path", attrs: &[("d", "M14.7 6.3a1 1 0 0 0 0 1.4l1.6 1.6a1 1 0 0 0 1.4 0l3.77-3.77a6 6 0 0 1-7.94 7.94l-6.91 6.91a2.12 2.12 0 0 1-3-3l6.91-6.91a6 6 0 0 1 7.94-7.94l-3.76 3.76z")] },
    ],
};

    /// 图标 `users`。
#[rustfmt::skip]
pub const ICON_USERS: IconData = IconData {
    name: "users",
    class: "lucide-icon lucide lucide-users",
    nodes: &[
        IconNode { tag: "path", attrs: &[("d", "M16 21v-2a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v2")] },
        IconNode { tag: "circle", attrs: &[("cx", "9"), ("cy", "7"), ("r", "4")] },
        IconNode { tag: "path", attrs: &[("d", "M22 21v-2a4 4 0 0 0-3-3.87")] },
        IconNode { tag: "path", attrs: &[("d", "M16 3.13a4 4 0 0 1 0 7.75")] },
    ],
};

    /// 图标 `shield-check`。
#[rustfmt::skip]
pub const ICON_SHIELD_CHECK: IconData = IconData {
    name: "shield-check",
    class: "lucide-icon lucide lucide-shield-check",
    nodes: &[
        IconNode { tag: "path", attrs: &[("d", "M20 13c0 5-3.5 7.5-7.66 8.95a1 1 0 0 1-.67-.01C7.5 20.5 4 18 4 13V6a1 1 0 0 1 1-1c2 0 4.5-1.2 6.24-2.72a1.17 1.17 0 0 1 1.52 0C14.51 3.81 17 5 19 5a1 1 0 0 1 1 1z")] },
        IconNode { tag: "path", attrs: &[("d", "m9 12 2 2 4-4")] },
    ],
};

    /// 图标 `shield-off`。
#[rustfmt::skip]
pub const ICON_SHIELD_OFF: IconData = IconData {
    name: "shield-off",
    class: "lucide-icon lucide lucide-shield-off",
    nodes: &[
        IconNode { tag: "path", attrs: &[("d", "m2 2 20 20")] },
        IconNode { tag: "path", attrs: &[("d", "M5 5a1 1 0 0 0-1 1v7c0 5 3.5 7.5 7.67 8.94a1 1 0 0 0 .67.01c2.35-.82 4.48-1.97 5.9-3.71")] },
        IconNode { tag: "path", attrs: &[("d", "M9.309 3.652A12.252 12.252 0 0 0 11.24 2.28a1.17 1.17 0 0 1 1.52 0C14.51 3.81 17 5 19 5a1 1 0 0 1 1 1v7a9.784 9.784 0 0 1-.08 1.264")] },
    ],
};

    /// 图标 `terminal`。
#[rustfmt::skip]
pub const ICON_TERMINAL: IconData = IconData {
    name: "terminal",
    class: "lucide-icon lucide lucide-terminal",
    nodes: &[
        IconNode { tag: "polyline", attrs: &[("points", "4 17 10 11 4 5")] },
        IconNode { tag: "line", attrs: &[("x1", "12"), ("x2", "20"), ("y1", "19"), ("y2", "19")] },
    ],
};

    /// 图标 `timer`。
#[rustfmt::skip]
pub const ICON_TIMER: IconData = IconData {
    name: "timer",
    class: "lucide-icon lucide lucide-timer",
    nodes: &[
        IconNode { tag: "line", attrs: &[("x1", "10"), ("x2", "14"), ("y1", "2"), ("y2", "2")] },
        IconNode { tag: "line", attrs: &[("x1", "12"), ("x2", "15"), ("y1", "14"), ("y2", "11")] },
        IconNode { tag: "circle", attrs: &[("cx", "12"), ("cy", "14"), ("r", "8")] },
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

    /// 图标 `filter`。
#[rustfmt::skip]
pub const ICON_FILTER: IconData = IconData {
    name: "filter",
    class: "lucide-icon lucide lucide-filter",
    nodes: &[
        IconNode { tag: "polygon", attrs: &[("points", "22 3 2 3 10 12.46 10 19 14 21 14 12.46 22 3")] },
    ],
};

    /// 图标 `tag`。
#[rustfmt::skip]
pub const ICON_TAG: IconData = IconData {
    name: "tag",
    class: "lucide-icon lucide lucide-tag",
    nodes: &[
        IconNode { tag: "path", attrs: &[("d", "M12.586 2.586A2 2 0 0 0 11.172 2H4a2 2 0 0 0-2 2v7.172a2 2 0 0 0 .586 1.414l8.704 8.704a2.426 2.426 0 0 0 3.42 0l6.58-6.58a2.426 2.426 0 0 0 0-3.42z")] },
        IconNode { tag: "circle", attrs: &[("cx", "7.5"), ("cy", "7.5"), ("r", ".5"), ("fill", "currentColor")] },
    ],
};

    /// 图标 `copy`。
#[rustfmt::skip]
pub const ICON_COPY: IconData = IconData {
    name: "copy",
    class: "lucide-icon lucide lucide-copy",
    nodes: &[
        IconNode { tag: "rect", attrs: &[("width", "14"), ("height", "14"), ("x", "8"), ("y", "8"), ("rx", "2"), ("ry", "2")] },
        IconNode { tag: "path", attrs: &[("d", "M4 16c-1.1 0-2-.9-2-2V4c0-1.1.9-2 2-2h10c1.1 0 2 .9 2 2")] },
    ],
};

    /// 图标 `workflow`。
#[rustfmt::skip]
pub const ICON_WORKFLOW: IconData = IconData {
    name: "workflow",
    class: "lucide-icon lucide lucide-workflow",
    nodes: &[
        IconNode { tag: "rect", attrs: &[("width", "8"), ("height", "8"), ("x", "3"), ("y", "3"), ("rx", "2")] },
        IconNode { tag: "path", attrs: &[("d", "M7 11v4a2 2 0 0 0 2 2h4")] },
        IconNode { tag: "rect", attrs: &[("width", "8"), ("height", "8"), ("x", "13"), ("y", "13"), ("rx", "2")] },
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

    /// 图标 `folder-search`。
#[rustfmt::skip]
pub const ICON_FOLDER_SEARCH: IconData = IconData {
    name: "folder-search",
    class: "lucide-icon lucide lucide-folder-search",
    nodes: &[
        IconNode { tag: "path", attrs: &[("d", "M10.7 20H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h3.9a2 2 0 0 1 1.69.9l.81 1.2a2 2 0 0 0 1.67.9H20a2 2 0 0 1 2 2v4.1")] },
        IconNode { tag: "path", attrs: &[("d", "m21 21-1.9-1.9")] },
        IconNode { tag: "circle", attrs: &[("cx", "17"), ("cy", "17"), ("r", "3")] },
    ],
};

    /// 图标 `file-pen-line`。
#[rustfmt::skip]
pub const ICON_FILE_PEN_LINE: IconData = IconData {
    name: "file-pen-line",
    class: "lucide-icon lucide lucide-file-pen-line",
    nodes: &[
        IconNode { tag: "path", attrs: &[("d", "m18 5-2.414-2.414A2 2 0 0 0 14.172 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2")] },
        IconNode { tag: "path", attrs: &[("d", "M21.378 12.626a1 1 0 0 0-3.004-3.004l-4.01 4.012a2 2 0 0 0-.506.854l-.837 2.87a.5.5 0 0 0 .62.62l2.87-.837a2 2 0 0 0 .854-.506z")] },
        IconNode { tag: "path", attrs: &[("d", "M8 18h1")] },
    ],
};

    /// 图标 `file-check-2`。
#[rustfmt::skip]
pub const ICON_FILE_CHECK_2: IconData = IconData {
    name: "file-check-2",
    class: "lucide-icon lucide lucide-file-check-2",
    nodes: &[
        IconNode { tag: "path", attrs: &[("d", "M4 22h14a2 2 0 0 0 2-2V7l-5-5H6a2 2 0 0 0-2 2v4")] },
        IconNode { tag: "path", attrs: &[("d", "M14 2v4a2 2 0 0 0 2 2h4")] },
        IconNode { tag: "path", attrs: &[("d", "m3 15 2 2 4-4")] },
    ],
};
}

// ── 文案常量（**逐字取自生产** ⇒ 一个字符都不改） ───────────────────────────

/// 治理分区标题。
pub const GOVERNANCE_TITLE: &str = "决策与治理";

/// 治理分区说明（**含内部的换行 + 12 空格缩进** ⇒ 生产模板空白逐字保留）。
pub const GOVERNANCE_DESC: &str = "评审/议会深度、三洋葱权限门与子代理隔离。议会默认 3 位顾问、日常轻量运行，\n            重要决策再加深。";

/// 治理摘要的前缀（`<b>` 之前）。
pub const GOVERNANCE_SUMMARY_PREFIX: &str = "本页已启用 ";

/// 治理摘要的后缀（`<b>` 之后；**不含**总数 ⇒ 总数在渲染时拼进整串）。
pub const GOVERNANCE_SUMMARY_SUFFIX: &str = " 项治理件";

/// 「评审与议会」卡的标题（**带前导空格** ⇒ 图标与标题之间有一个空格）。
pub const DECISION_CARD_TITLE: &str = " 评审与议会";

/// 「权限与子代理」卡的标题（带前导空格）。
pub const GUARD_CARD_TITLE: &str = " 权限与子代理";

/// 认知深度行的标题。
pub const DEPTH_LABEL: &str = "认知深度";

/// 认知深度行的说明。
pub const DEPTH_DESC: &str =
    "快捷档位：轻量 ≈1.1s / 平衡 ≈2.6s / 深度 ≈13s 每轮（实测 2026-10-06）。";

/// 认知深度下拉的 `id`（**也是取值 property 的定位点**）。
pub const DEPTH_SELECT_ID: &str = "cognitive-depth";

/// 认知深度下拉的四个选项（`值` → `文案`）。
pub const DEPTH_OPTIONS: [(&str, &str); 4] = [
    ("light", "轻量"),
    ("balanced", "平衡"),
    ("deep", "深度"),
    ("custom", "自定义"),
];

/// 认知深度下拉的意图载荷（**取值不在事件里** ⇒ 宿主回读 DOM）。
pub const DEPTH_CONTROL_KEY: &str = "cognitive-depth";

/// 「顾问数量」行的标题。
pub const ADVISORS_LABEL: &str = "顾问数量";

/// 「顾问数量」行的 env 芯片。
pub const ADVISORS_ENV: &str = "APEIRETH_COUNCIL_ADVISORS";

/// 「顾问数量」行的说明。
pub const ADVISORS_DESC: &str = "1-7 位，Safety 恒首位；默认 3（7→3 收敛批）。";

/// 顾问数量滑杆的 `aria-label`（**宿主回写 property 的定位点**）。
pub const ADVISORS_ARIA_LABEL: &str = "顾问数量";

/// 顾问数量滑杆的值键。
pub const ADVISORS_VALUE_KEY: &str = "councilAdvisors";

/// 「单顾问超时」行的标题。
pub const TIMEOUT_LABEL: &str = "单顾问超时 (ms)";

/// 「单顾问超时」行的 env 芯片。
pub const TIMEOUT_ENV: &str = "APEIRETH_COUNCIL_TIMEOUT_MS";

/// 「单顾问超时」行的说明。
pub const TIMEOUT_DESC: &str =
    "默认 30000；思考型模型延迟高时可放宽。失焦或回车提交，失败回填旧值。";

/// 单顾问超时输入框的 `aria-label`（宿主回写 property 的定位点）。
pub const TIMEOUT_ARIA_LABEL: &str = "单顾问超时毫秒";

/// 单顾问超时输入框的值键。
pub const TIMEOUT_VALUE_KEY: &str = "councilTimeoutMs";

/// 治理分区末尾「设计说明」卡的徽标文案（**带前导空格**）。
pub const GOVERNANCE_NOTE_BADGE: &str = " 设计说明 · 非开关";

/// 「设计说明」卡的标题。
pub const GOVERNANCE_NOTE_TITLE: &str = "议会语义（2026-10-10 改造批）";

/// 「设计说明」卡的正文（**含内部的换行 + 14 空格缩进**）。
pub const GOVERNANCE_NOTE_TEXT: &str = "议会是决策环节的顾问团，不是每轮评审器：升级/部署批准、高危操作、\n              待裁冲突批量裁决时启用，日常对话保持轻量。未配 LLM 时 fail-loud 不造假裁决。";

/// 工具分区标题。
pub const TOOLS_TITLE: &str = "工具与安全";

/// 工具分区说明。
pub const TOOLS_DESC: &str =
    "工具权限授予 + 沙箱边界。开启工具 ≠ 无审批执行——shell 每次调用仍走审批卡。";

/// 「允许连接的自建服务器」分组标题。
pub const SERVERS_GROUP_LABEL: &str = "允许连接的自建服务器";

/// 「允许连接的自建服务器」分组说明。
pub const SERVERS_HINT: &str = "允许桌面端连接这些服务器；只加你信任的地址（http(s) 基址或 主机:端口）。本机服务不由这里管理，归本地端点注册面。";

/// 允许服务器输入框的 `id`（宿主回写 property 的定位点）。
pub const SERVER_INPUT_ID: &str = "allowed-server-input";

/// 允许服务器输入框的值键。
pub const SERVER_INPUT_KEY: &str = "allowedServerInput";

/// 允许服务器输入框的占位文案。
pub const SERVER_INPUT_PLACEHOLDER: &str =
    "https://relay.example.com:8443 或 relay.example.com:8443";

/// 「移除」按钮的文案。
pub const SERVER_REMOVE_LABEL: &str = "移除";

/// 「添加」按钮的文案。
pub const SERVER_ADD_LABEL: &str = "添加";

/// 工具摘要的前缀（`<b>` 之前）。
pub const TOOLS_SUMMARY_PREFIX: &str = "本页已授予 ";

/// 工具摘要的后缀（`<b>` 之后、**沙箱状态之前**）。
pub const TOOLS_SUMMARY_SUFFIX: &str = " 类工具 · 沙箱";

/// 沙箱打开时的摘要词。
pub const SANDBOX_ON_LABEL: &str = "开";

/// 沙箱关闭时的摘要词。
pub const SANDBOX_OFF_LABEL: &str = "关";

/// 「全局权限预设」的分组标题。
pub const PRESET_LABEL: &str = "全局权限预设";

/// 全局权限预设下拉的 `id`（宿主回写 property 的定位点）。
pub const PRESET_SELECT_ID: &str = "permission-preset";

/// 全局权限预设的意图载荷。
pub const PRESET_CONTROL_KEY: &str = "permission-preset";

/// 全局权限预设的三个选项（`值` → `文案`）。
pub const PRESET_OPTIONS: [(&str, &str); 3] = [
    ("read_only", "read_only — 只读"),
    ("standard", "standard — 标准 (推荐)"),
    ("full", "full — 完全权限"),
];

/// 全局权限预设的说明。
pub const PRESET_HINT: &str = "作为新会话默认 (已生效)：仅对新会话生效，已有会话请在会话内改。";

/// 「工具授予」卡的标题（带前导空格）。
pub const TOOL_CARD_TITLE: &str = " 工具授予";

/// 沙箱嵌套行的标题。
pub const SANDBOX_LABEL: &str = "AppContainer 沙箱执行";

/// 沙箱嵌套行的 env 芯片。
pub const SANDBOX_ENV: &str = "APEIRETH_SHELL_SANDBOX";

/// 沙箱嵌套行的值键。
pub const SANDBOX_VALUE_KEY: &str = "shellSandbox";

/// 沙箱嵌套行的说明（**含内部的换行 + 22 空格缩进**）。
pub const SANDBOX_DESC: &str = "断网 + 用户空间零访问（双探针实证，默认开）。关闭 = 显式裸跑，\n                      命令获得你的完整用户权限——危险操作会红标提醒。";

/// 沙箱嵌套行的 `aria-label`。
pub const SANDBOX_ARIA_LABEL: &str = "AppContainer 沙箱执行";

/// 沙箱关闭时的告警文案。
pub const SANDBOX_WARNING: &str =
    "沙箱已关闭：shell 命令将以你的完整用户权限裸跑，请确认你信任正在运行的任务。";

/// 工具分组里「shell」的键（**沙箱嵌套行的挂点**）。
pub const SHELL_KEY: &str = "shell";

/// 工具分组里「fileWrite」的键（**依赖子行的挂点**）。
pub const FILE_WRITE_KEY: &str = "fileWrite";

/// MCP 静态行的标题。
pub const MCP_LABEL: &str = "外部工具桥（MCP）";

/// MCP 静态行的 env 芯片。
pub const MCP_ENV: &str = "APEIRETH_ENABLE_MCP";

/// MCP 静态行的说明（**含内部的换行 + 18 空格缩进**）。
pub const MCP_DESC: &str = "无开关（本页不设开关，服务端 CLI 旋钮）——需另行配置服务器列表\n                  （APEIRETH_MCP_SERVERS 或数据目录 mcp-servers.json）才真正有外部工具；\n                  已启用但无服务器配置 = 无外部工具（自报照实）。";

/// 「常驻 · 默认运行」徽标文案（**带前导空格**）。
pub const RESIDENT_BADGE: &str = " 常驻 · 默认运行";

/// 权限洋葱说明卡的标题。
pub const ONION_TITLE: &str = "权限洋葱与即时授权 (On-demand Permission Pack)";

/// 权限洋葱说明卡的正文（**含内部的换行 + 14 空格缩进**）。
pub const ONION_TEXT: &str = "这是始终在线的安全机制，不是可关的开关：Master Token 绝不持久化保存在客户端存储中；\n              特权工具被拒绝产生待批请求时，在「工具管理」页面输入 Token 即时完成时效性签发。";

/// 宪法评审说明卡的标题。
pub const CONSTITUTION_TITLE: &str = "宪法评审 (MiniMaxConstitutionLlm)";

/// 宪法评审说明卡的正文（**含内部的换行 + 14 空格缩进**）。
pub const CONSTITUTION_TEXT: &str =
    "高危工具执行前自动按 E 层进行安全判案——默认运行、不可关闭，\n              杜绝越权或有害操作。";

/// 开发者分区标题。
pub const DEVELOPER_TITLE: &str = "开发者选项";

/// 开发者分区说明（**含内部的换行 + 12 空格缩进**）。
pub const DEVELOPER_DESC: &str = "Beta 功能试验场与运行时契约。Beta 件默认关、随时可撤，验证稳定后晋升正式设置页；\n            改动即点/失焦即注入侧车环境生效。";

/// Beta 功能卡的标题（带前导空格）。
pub const BETA_CARD_TITLE: &str = " Beta 功能";

/// 思考模式文本旋钮组的 `aria-label`。
pub const REASONING_GROUP_ARIA_LABEL: &str = "思考模式文本旋钮";

/// 「生效模型过滤器」行的标题。
pub const REASONING_FILTERS_LABEL: &str = "生效模型过滤器";

/// 「生效模型过滤器」行的 env 芯片。
pub const REASONING_FILTERS_ENV: &str = "APEIRETH_REASONING_MODEL_FILTERS";

/// 「生效模型过滤器」行的说明。
pub const REASONING_FILTERS_DESC: &str =
    "逗号分隔子串白名单（如 deepseek,o3）；留空 = 全部模型生效。";

/// 过滤器输入框的 `aria-label`（宿主回写 property 的定位点）。
pub const REASONING_FILTERS_ARIA_LABEL: &str = "思考模式模型过滤器";

/// 过滤器输入框的占位文案。
pub const REASONING_FILTERS_PLACEHOLDER: &str = "全部模型";

/// 过滤器输入框的值键。
pub const REASONING_FILTERS_VALUE_KEY: &str = "reasoningModelFilters";

/// 「reasoning 标签」行的标题。
pub const REASONING_TAG_LABEL: &str = "reasoning 标签";

/// 「reasoning 标签」行的 env 芯片。
pub const REASONING_TAG_ENV: &str = "APEIRETH_REASONING_TAG";

/// 「reasoning 标签」行的说明。
pub const REASONING_TAG_DESC: &str = "reasoning_content 的字段标签名，默认 think。";

/// 标签输入框的 `aria-label`（**本行没有 placeholder**）。
pub const REASONING_TAG_ARIA_LABEL: &str = "reasoning 标签";

/// 标签输入框的值键。
pub const REASONING_TAG_VALUE_KEY: &str = "reasoningTag";

/// Beta 分组的「思考模式」键（**文本旋钮组的开关**）。
pub const REASONING_ENABLED_KEY: &str = "reasoningEnabled";

/// 思考模式文本旋钮组的组名（**整组提交** ⇒ 意图载荷）。
pub const REASONING_GROUP_KEY: &str = "reasoning";

/// 「契约 · 恒成立」徽标文案（**带前导空格**）。
pub const CONTRACT_BADGE: &str = " 契约 · 恒成立";

/// 运行时契约说明卡的标题。
pub const CONTRACT_TITLE: &str = "Agent Runtime Contract (§15)";

/// 运行时契约说明卡的正文（**含内部的换行 + 14 空格缩进**）。
pub const CONTRACT_TEXT: &str = "UI 仅面对标准事件流 (run-start, text-delta, reasoning-delta, tool-call, tool-result,\n              message-end)，不裸碰底层 HTTP/SSE 协议。";

/// 「客户端配置 (JSON)」的分组标题。
pub const CONFIG_GROUP_LABEL: &str = "客户端配置 (JSON)";

/// 「客户端配置 (JSON)」标签的 `for`（生产写死，**没有对应 id 的元素**）。
pub const CONFIG_LABEL_FOR: &str = "raw-config-json";

/// 复制按钮的意图载荷。
pub const COPY_CONTROL_KEY: &str = "copy-config";

/// 复制按钮的文案（未复制）。
pub const COPY_LABEL: &str = "复制";

/// 复制按钮的文案（已复制）。
pub const COPIED_LABEL: &str = "已复制";

/// 字段旗标的键：单顾问超时。
pub const FLAG_COUNCIL_TIMEOUT: &str = "council.timeoutMs";
/// 字段旗标的键：过滤器。
pub const FLAG_REASONING_FILTERS: &str = "reasoning.filters";
/// 字段旗标的键：标签。
pub const FLAG_REASONING_TAG: &str = "reasoning.tag";

// ── 分区 ────────────────────────────────────────────────────────────────────

/// 分区 id：治理。
pub const SECTION_GOVERNANCE: &str = "governance";
/// 分区 id：工具。
pub const SECTION_TOOLS: &str = "tools";
/// 分区 id：开发者。
pub const SECTION_DEVELOPER: &str = "developer";

/// 本面覆盖的三个分区（**顺序即生产的分支顺序**）。
pub const SECTIONS: [&str; 3] = [SECTION_GOVERNANCE, SECTION_TOOLS, SECTION_DEVELOPER];

/// 判定分区 id 是否属本面（**`security` 不在内** ⇒ 该分区不迁）。
pub fn is_known_section(id: &str) -> bool {
    SECTIONS.contains(&id)
}

// ── props（**宿主注入** ⇒ 组件是纯函数） ────────────────────────────────────

/// 能力与治理面的 props（**宿主注入** ⇒ 组件只渲染）。
#[derive(Clone)]
pub struct CapabilityPanelsProps {
    /// 当前分区（`governance` / `tools` / `developer`）。
    pub section: Signal<String>,
    /// 能力表（**键 → 取值**）。
    pub capabilities: Signal<CapabilityTable>,
    /// 生效中的键（`pending` 态；`None` ⇒ 无）。
    pub pending_key: Signal<Option<String>>,
    /// 认知深度下拉的取值（**property** ⇒ 宿主回写 DOM）。
    pub cognitive_depth: Signal<String>,
    /// 全局权限预设的取值（**property** ⇒ 宿主回写 DOM）。
    pub permission_preset: Signal<String>,
    /// 允许连接的自建服务器（**顺序即渲染顺序**）。
    pub allowed_servers: Signal<Vec<String>>,
    /// 允许服务器输入框的取值（**property** ⇒ 宿主回写 DOM）。
    pub allowed_server_input: Signal<String>,
    /// 允许服务器输入框的错误文案（空 ⇒ 不渲染错误行）。
    pub allowed_server_error: Signal<String>,
    /// 允许服务器的操作进行中（`true` ⇒ 输入框与按钮 `disabled=""`）。
    pub allowed_servers_busy: Signal<bool>,
    /// 字段旗标（键 → 状态；**宿主持有** ⇒ 组件只渲染）。
    pub text_flags: Signal<Vec<(String, FieldFlagState)>>,
    /// 客户端配置 JSON 的**预览文本**（**宿主按 `JSON.stringify(v, null, 2)` 的
    /// 语义生成** ⇒ 组件只渲染，**不自己拼字符串**）。
    pub config_json_preview: Signal<String>,
    /// 配置 JSON 的复制态（`true` ⇒ 按钮显示「已复制」）。
    pub config_json_copied: Signal<bool>,
    /// 决策组的能力定义表（**数据**）。
    pub decision_defs: Vec<CapabilityDef>,
    /// 治理组的能力定义表（**数据**）。
    pub guard_defs: Vec<CapabilityDef>,
    /// 工具组的能力定义表（**数据**）。
    pub tool_defs: Vec<CapabilityDef>,
    /// 工具子开关的能力定义表（**数据**；依赖父行）。
    pub tool_sub_defs: Vec<CapabilityDef>,
    /// Beta 组的能力定义表（**数据**）。
    pub beta_defs: Vec<CapabilityDef>,
    /// **冻结卡组件的 props 适配**（见 [`card_bridge`]）⇒ 行组件从这里取两个意图出口。
    pub card: CharacterPanelsProps,
    /// 「切换认知深度」意图出口（入参 = 控件键；**取值不在事件载荷里**）。
    pub on_select_depth: ActionIntent,
    /// 「切换全局权限预设」意图出口。
    pub on_select_preset: ActionIntent,
    /// 「允许服务器」的动作意图（`add` / `remove:<基址>`）。
    pub on_allowed_server: ActionIntent,
    /// 「复制配置 JSON」意图出口。
    pub on_copy_config: ActionIntent,
    /// 「文本旋钮改动」意图出口（入参 = 字段键；宿主回读 DOM）。
    pub on_field_input: ActionIntent,
    /// 「整组提交」意图出口（失焦 / 回车 ⇒ 提交）。
    pub on_submit_group: ActionIntent,
}

impl std::fmt::Debug for CapabilityPanelsProps {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CapabilityPanelsProps")
            .field("section", &self.section.get())
            .field("capabilities", &self.capabilities.get().len())
            .field("pending_key", &self.pending_key.get())
            .field("cognitive_depth", &self.cognitive_depth.get())
            .field("permission_preset", &self.permission_preset.get())
            .field("config_json_copied", &self.config_json_copied.get())
            .finish_non_exhaustive()
    }
}

/// 能力与治理面组件（**三个分区由宿主的 `section` 决定**）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CapabilityPanels;

impl UiComponent for CapabilityPanels {
    type Props = CapabilityPanelsProps;
    type Output = VNode;

    fn render(props: &Self::Props) -> Self::Output {
        match props.section.get().as_str() {
            SECTION_GOVERNANCE => render_governance(props),
            SECTION_TOOLS => render_tools(props),
            // ⚠️ 生产的分支顺序是 `{#if governance} … {:else if tools} … {:else}`
            // ⇒ **兜底分支 = developer**（不是 governance）⇒ 这里逐字照抄。
            _ => render_developer(props),
        }
    }
}

/// 冻结卡组件的 props 适配（**只填两个意图出口** ⇒ 其余字段空值）。
///
/// ## 为什么需要
///
/// 人格与记忆面（M6）冻结的行组件
/// [`crate::settings_character::render_capability_rows`] 吃的是**它自己的**
/// props 类型 [`CharacterPanelsProps`]（19 个字段 + 11 个意图出口）⇒
/// 而它对行渲染**只读**两个出口（`on_toggle_capability` / `on_set_knob`）。
///
/// ⇒ 本适配把**其余字段填空值**，只把两个出口接到本面的意图上 ⇒ 行渲染
/// **逐字复用**，且**不改冻结文件**（正解：把行组件的 props 解耦成
/// 「两个意图 + 表」⇒ 见报告的「需要主线改的共享文件」）。
pub fn card_bridge(
    runtime: &Runtime,
    toggle: CapabilityIntent,
    knob: KnobIntent,
) -> CharacterPanelsProps {
    let noop: ActionIntent = Rc::new(|_: &str| {});
    CharacterPanelsProps {
        section: runtime.signal(String::new()),
        personas: runtime.signal(Vec::new()),
        active_persona_id: runtime.signal(String::new()),
        text_flags: runtime.signal(Vec::new()),
        impression: runtime.signal(Impression::default()),
        impression_draft: runtime.signal(String::new()),
        impression_error: runtime.signal(String::new()),
        impression_busy: runtime.signal(false),
        capabilities: runtime.signal(CapabilityTable::new()),
        pending_key: runtime.signal(None),
        tuning_log: runtime.signal(None::<Vec<TuningLogEntry>>),
        tuning_log_loading: runtime.signal(false),
        global_model: String::new(),
        memory_flow_defs: Vec::new(),
        cognition_defs: Vec::new(),
        community_defs: Vec::new(),
        // 本面**不渲染**「从使用中学习」那一行 ⇒ 这里的定义只为把字段填满。
        self_tuning_def: CapabilityDef::new("", &icons::ICON_INFO, "", "", ""),
        disposition_knobs: Vec::new(),
        disposition_presets: Vec::new(),
        on_toggle_capability: toggle,
        on_set_knob: knob,
        on_select_persona: noop.clone(),
        on_add_persona: noop.clone(),
        on_remove_persona: noop.clone(),
        on_impression: noop.clone(),
        on_apply_preset: noop.clone(),
        on_undo_tuning: noop.clone(),
        on_submit_group: noop.clone(),
        on_field_input: noop,
    }
}

// ── 小工具 ──────────────────────────────────────────────────────────────────

/// 取字段旗标的状态（**查不到 ⇒ 无旗标**）。
fn flag_of(flags: &[(String, FieldFlagState)], key: &str) -> FieldFlagState {
    flags
        .iter()
        .find(|(name, _)| name == key)
        .map(|(_, state)| *state)
        .unwrap_or(FieldFlagState::None)
}

/// 渲染一张能力卡的**壳**（标题 + 计数 + 行；行之间一个空格）。
fn render_card(
    title_icon: &'static IconData,
    heading: &str,
    count: &str,
    rows: Vec<VNode>,
) -> VNode {
    let head = view! {
        div(class = "cap-card-head") {
            span(class = "cap-card-title") {
                { Icon::render(&IconProps::new(title_icon, 13)) }
                heading
            }
            " "
            span(class = "cap-card-count") { count }
        }
    };
    let mut card = VNode::element("div").attr("class", "cap-card");
    card = card.child(head);
    for row in rows {
        card = card.child(VNode::text(" ")).child(row);
    }
    card.build()
}

/// 渲染一张**静态行**（`div.cap-row.cap-row-static` ⇒ 图标 + 文本 + 右侧控件）。
fn render_static_row(
    icon: &'static IconData,
    label: &str,
    env: Option<&str>,
    desc: &str,
    flag: FieldFlagState,
    right: VNode,
) -> VNode {
    let mut strong = VNode::element("strong").child(VNode::text(label));
    if let Some(env) = env {
        strong = strong.child(view! { code(class = "cap-env") { env } });
    }
    if flag != FieldFlagState::None {
        strong = strong
            .child(VNode::text(" "))
            .child(FieldFlag::render(&FieldFlagProps::new(flag)));
    }
    let strong = strong.build();
    view! {
        div(class = "cap-row cap-row-static") {
            span(class = "cap-icon") { { Icon::render(&IconProps::new(icon, 15)) } }
            " "
            span(class = "cap-text") {
                { strong }
                " "
                small { desc }
            }
            " "
            { right }
        }
    }
}

/// 渲染一张**说明卡**（`div.cap-default-card` ⇒ 徽标 + 标题 + 正文）。
///
/// `resident` = `true` ⇒ 徽标是 `cap-default-badge on`（常驻 · 默认运行）；
/// `false` ⇒ `cap-default-badge note`（设计说明 / 契约）。
fn render_default_card(
    badge_icon: &'static IconData,
    resident: bool,
    badge: &str,
    heading: &str,
    text: &str,
) -> VNode {
    view! {
        div(class = "cap-default-card") {
            span(class = "cap-default-badge", class:on = resident, class:note = !resident) {
                { Icon::render(&IconProps::new(badge_icon, 11)) }
                badge
            }
            " "
            strong(class = "cap-default-title") { heading }
            " "
            p(class = "cap-default-text") { text }
        }
    }
}

/// 渲染**开关行**（**复用冻结的行组件** ⇒ 一项一行 ⇒ 不重造）。
fn render_switch_rows(
    defs: &[CapabilityDef],
    table: &CapabilityTable,
    pending_key: Option<&str>,
    card: &CharacterPanelsProps,
) -> Vec<VNode> {
    render_capability_rows(defs, table, pending_key, card)
}

/// 渲染**一行**开关（**复用冻结的行组件** ⇒ 一项也走同一条路径）。
fn render_one_switch_row(
    def: &CapabilityDef,
    table: &CapabilityTable,
    pending_key: Option<&str>,
    card: &CharacterPanelsProps,
) -> VNode {
    let mut rows = render_capability_rows(std::slice::from_ref(def), table, pending_key, card);
    if rows.is_empty() {
        return VNode::fragment(Vec::new());
    }
    rows.remove(0)
}

// ── 治理分区 ────────────────────────────────────────────────────────────────

/// 治理分区的渲染。
fn render_governance(props: &CapabilityPanelsProps) -> VNode {
    let table = props.capabilities.get();
    let pending = props.pending_key.get();
    let flags = props.text_flags.get();

    let total = props.decision_defs.len() + props.guard_defs.len();
    let enabled =
        enabled_count(&props.decision_defs, &table) + enabled_count(&props.guard_defs, &table);

    let heading = GOVERNANCE_TITLE;
    let desc = GOVERNANCE_DESC;
    let summary_prefix = GOVERNANCE_SUMMARY_PREFIX;
    let summary_suffix = format!(" / {total}{GOVERNANCE_SUMMARY_SUFFIX}");
    let enabled_text = enabled.to_string();
    let note_badge = GOVERNANCE_NOTE_BADGE;
    let note_title = GOVERNANCE_NOTE_TITLE;
    let note_text = GOVERNANCE_NOTE_TEXT;

    let decision_card = render_card(
        &icons::ICON_USERS,
        DECISION_CARD_TITLE,
        &format!(
            "{}/{}",
            enabled_count(&props.decision_defs, &table),
            props.decision_defs.len()
        ),
        render_decision_card_rows(props, &table, pending.as_deref(), &flags),
    );
    let guard_card = render_card(
        &icons::ICON_SHIELD_CHECK,
        GUARD_CARD_TITLE,
        &format!(
            "{}/{}",
            enabled_count(&props.guard_defs, &table),
            props.guard_defs.len()
        ),
        render_switch_rows(&props.guard_defs, &table, pending.as_deref(), &props.card),
    );
    let note_card =
        render_default_card(&icons::ICON_INFO, false, note_badge, note_title, note_text);

    view! {
        div(class = "setting-block") {
            h3(class = "block-title") { heading }
            " "
            p(class = "block-desc") { desc }
            " "
            div(class = "cap-summary") {
                { Icon::render(&IconProps::new(&icons::ICON_SCALE, 14)) }
                " "
                span {
                    summary_prefix
                    b { enabled_text }
                    summary_suffix
                }
            }
            " "
            { decision_card }
            " "
            { guard_card }
            " "
            { note_card }
        }
    }
}

/// 「评审与议会」卡的**行序列**（认知深度行 + 决策行 + 议会条件行）。
fn render_decision_card_rows(
    props: &CapabilityPanelsProps,
    table: &CapabilityTable,
    pending_key: Option<&str>,
    flags: &[(String, FieldFlagState)],
) -> Vec<VNode> {
    let mut rows: Vec<VNode> = vec![render_depth_row(props)];
    rows.extend(render_switch_rows(
        &props.decision_defs,
        table,
        pending_key,
        &props.card,
    ));
    // ⚠️ 生产：`{#if capabilities.council}` ⇒ 顾问数量行 + 单顾问超时行
    // **只在议会打开时出现**（两行都是 `cap-row cap-row-static`）。
    if capability_bool(table, "council") {
        rows.push(render_advisors_row(props, table));
        rows.push(render_timeout_row(
            props,
            flag_of(flags, FLAG_COUNCIL_TIMEOUT),
        ));
    }
    rows
}

/// 认知深度行（**原生 `select`** ⇒ 取值是 property ⇒ 不写 `value` 属性）。
fn render_depth_row(props: &CapabilityPanelsProps) -> VNode {
    let intent = props.on_select_depth.clone();
    let key = DEPTH_CONTROL_KEY;
    let options: Vec<VNode> = DEPTH_OPTIONS
        .iter()
        .map(|item| {
            let (value, option_text) = *item;
            view! { option(value = value) { option_text } }
        })
        .collect();
    // ⚠️ **取值由组件声明**（`prop("value", …)` ⇒ **property**，不写属性）：
    // 生产是 `bind:value` ⇒ Svelte 写 property、DOM 上**没有** `value` 属性
    // ⇒ 组件自己带值 ⇒ 宿主不必再按选择器回写。
    let depth_value = props.cognitive_depth.get();
    let mut select = VNode::element("select")
        .attr("id", DEPTH_SELECT_ID)
        .prop("value", depth_value)
        .handler(
            EventKind::Change,
            ElementHandler::new(move |_event: &mut DomEvent| {
                (intent)(key);
            }),
        );
    for option in options {
        select = select.child(option);
    }
    let wrap = VNode::element("span")
        .attr("class", "cap-select-wrap")
        .child(select.build())
        .build();
    render_static_row(
        &icons::ICON_BRAIN,
        DEPTH_LABEL,
        None,
        DEPTH_DESC,
        FieldFlagState::None,
        wrap,
    )
}

/// 顾问数量行（**原生 `input[type=range]`** ⇒ 取值是 property）。
fn render_advisors_row(props: &CapabilityPanelsProps, table: &CapabilityTable) -> VNode {
    let advisors = capability_number(table, ADVISORS_VALUE_KEY);
    let knob = props.card.on_set_knob.clone();
    let input_key = ADVISORS_VALUE_KEY;
    let submit = props.on_submit_group.clone();
    let submit_key = ADVISORS_VALUE_KEY;
    // ⚠️ **取值由组件声明**（`prop("value", …)` ⇒ **property**，不写属性）：
    // 生产是 `bind:value` ⇒ Svelte 写 property、DOM 上**没有** `value` 属性
    // ⇒ 组件自己带值 ⇒ 宿主不必再按选择器回写。
    let mut slider = VNode::element("input")
        .attr("type", "range")
        .prop("value", crate::num::number_to_string(advisors))
        .attr("min", "1")
        .attr("max", "7")
        .attr("step", "1")
        .attr("aria-label", ADVISORS_ARIA_LABEL)
        .handler(
            EventKind::Input,
            ElementHandler::new(move |_event: &mut DomEvent| {
                (knob)(input_key);
            }),
        );
    slider = slider.handler(
        EventKind::Change,
        ElementHandler::new(move |_event: &mut DomEvent| {
            (submit)(submit_key);
        }),
    );
    let shown = crate::num::number_to_string(advisors);
    let wrap = VNode::element("span")
        .attr("class", "cap-slider-wrap")
        .child(slider.build())
        .child(VNode::text(" "))
        .child(view! { b { shown } })
        .build();
    render_static_row(
        &icons::ICON_USERS,
        ADVISORS_LABEL,
        Some(ADVISORS_ENV),
        ADVISORS_DESC,
        FieldFlagState::None,
        wrap,
    )
}

/// 单顾问超时行（**原生 `input[type=number]`** ⇒ 无 `max` 属性、取值是 property）。
fn render_timeout_row(props: &CapabilityPanelsProps, flag: FieldFlagState) -> VNode {
    let field_input = props.on_field_input.clone();
    let field_key = TIMEOUT_VALUE_KEY;
    let focus_out = props.on_submit_group.clone();
    let focus_key = FLAG_COUNCIL_TIMEOUT;
    let keydown = props.on_submit_group.clone();
    let keydown_key = FLAG_COUNCIL_TIMEOUT;
    // ⚠️ **取值由组件声明**（`prop("value", …)` ⇒ **property**，不写属性）：
    // 生产是 `bind:value` ⇒ Svelte 写 property、DOM 上**没有** `value` 属性
    // ⇒ 组件自己带值 ⇒ 宿主不必再按选择器回写。
    let timeout_value = capability_number(&props.capabilities.get(), TIMEOUT_VALUE_KEY);
    let mut number = VNode::element("input")
        .attr("type", "number")
        .prop("value", crate::num::number_to_string(timeout_value))
        .attr("min", "1000")
        .attr("step", "5000")
        .attr("aria-label", TIMEOUT_ARIA_LABEL)
        .handler(
            EventKind::Input,
            ElementHandler::new(move |_event: &mut DomEvent| {
                (field_input)(field_key);
            }),
        );
    number = number.handler(
        EventKind::FocusOut,
        ElementHandler::new(move |_event: &mut DomEvent| {
            (focus_out)(focus_key);
        }),
    );
    number = number.handler(
        EventKind::KeyDown,
        ElementHandler::new(move |event: &mut DomEvent| {
            // 生产：`if (isTextCommitKey(e.key)) { … submit … }` ⇒ 只在回车 / Esc
            // 提交（其它键不打扰运行时）。
            if matches!(event.key(), Some("Enter") | Some("Escape")) {
                (keydown)(keydown_key);
            }
        }),
    );
    let wrap = VNode::element("span")
        .attr("class", "cap-slider-wrap")
        .child(number.build())
        .build();
    render_static_row(
        &icons::ICON_TIMER,
        TIMEOUT_LABEL,
        Some(TIMEOUT_ENV),
        TIMEOUT_DESC,
        flag,
        wrap,
    )
}

// ── 工具分区 ────────────────────────────────────────────────────────────────

/// 工具分区的渲染。
fn render_tools(props: &CapabilityPanelsProps) -> VNode {
    let table = props.capabilities.get();
    let pending = props.pending_key.get();
    let busy = props.allowed_servers_busy.get();
    let servers = props.allowed_servers.get();
    let error = props.allowed_server_error.get();
    let sandbox_on = capability_bool(&table, SANDBOX_VALUE_KEY);
    let enabled = enabled_count(&props.tool_defs, &table);

    let heading = TOOLS_TITLE;
    let desc = TOOLS_DESC;
    let group_label = SERVERS_GROUP_LABEL;
    let group_hint = SERVERS_HINT;
    let preset_label = PRESET_LABEL;
    let preset_hint = PRESET_HINT;
    let summary_prefix = TOOLS_SUMMARY_PREFIX;
    let enabled_text = enabled.to_string();
    let sandbox_word = if sandbox_on {
        SANDBOX_ON_LABEL
    } else {
        SANDBOX_OFF_LABEL
    };
    // ⚠️ 常量 + 表达式**必须落成同一个文本节点**（生产里它们是同一段文本）
    // ⇒ 先拼成整串。
    let summary_suffix = format!(
        " / {}{TOOLS_SUMMARY_SUFFIX}{sandbox_word}",
        props.tool_defs.len()
    );

    let server_rows: Vec<VNode> = servers
        .iter()
        .map(|base| render_server_row(props, base, busy))
        .collect();
    let add_row = render_server_add_row(props, busy);
    let error_node = if error.is_empty() {
        VNode::fragment(Vec::new())
    } else {
        view! { small(class = "field-hint allowed-server-error") { error.clone() } }
    };

    let tool_card = render_card(
        &icons::ICON_TERMINAL,
        TOOL_CARD_TITLE,
        &format!("{}/{}", enabled, props.tool_defs.len()),
        render_tool_card_rows(props, &table, pending.as_deref()),
    );

    let preset_options: Vec<VNode> = PRESET_OPTIONS
        .iter()
        .map(|item| {
            let (value, option_text) = *item;
            view! { option(value = value) { option_text } }
        })
        .collect();
    let preset_intent = props.on_select_preset.clone();
    let preset_key = PRESET_CONTROL_KEY;
    // ⚠️ **取值由组件声明**（`prop("value", …)` ⇒ **property**，不写属性）：
    // 生产是 `bind:value` ⇒ Svelte 写 property、DOM 上**没有** `value` 属性
    // ⇒ 组件自己带值 ⇒ 宿主不必再按选择器回写。
    let preset_value = props.permission_preset.get();
    let mut preset_select = VNode::element("select")
        .attr("id", PRESET_SELECT_ID)
        .prop("value", preset_value)
        .handler(
            EventKind::Change,
            ElementHandler::new(move |_event: &mut DomEvent| {
                (preset_intent)(preset_key);
            }),
        );
    for option in preset_options {
        preset_select = preset_select.child(option);
    }
    let preset_select = preset_select.build();

    let resident_badge = RESIDENT_BADGE;
    let onion_card = render_default_card(
        &icons::ICON_CIRCLE_CHECK,
        true,
        resident_badge,
        ONION_TITLE,
        ONION_TEXT,
    );
    let constitution_card = render_default_card(
        &icons::ICON_CIRCLE_CHECK,
        true,
        resident_badge,
        CONSTITUTION_TITLE,
        CONSTITUTION_TEXT,
    );

    view! {
        div(class = "setting-block") {
            h3(class = "block-title") { heading }
            " "
            p(class = "block-desc") { desc }
            " "
            div(class = "form-group") {
                span(class = "group-label") { group_label }
                " "
                small(class = "field-hint") { group_hint }
                " "
                for row in server_rows {
                    { row }
                    " "
                }
                { add_row }
                " "
                { error_node }
            }
            " "
            div(class = "cap-summary") {
                { Icon::render(&IconProps::new(&icons::ICON_WRENCH, 14)) }
                " "
                span {
                    summary_prefix
                    b { enabled_text }
                    summary_suffix
                }
            }
            " "
            div(class = "form-group") {
                label(for = PRESET_SELECT_ID) { preset_label }
                " "
                { preset_select }
                " "
                small(class = "field-hint") { preset_hint }
            }
            " "
            { tool_card }
            " "
            { onion_card }
            " "
            { constitution_card }
        }
    }
}

/// 一行「允许连接的自建服务器」。
fn render_server_row(props: &CapabilityPanelsProps, base: &str, busy: bool) -> VNode {
    let intent = props.on_allowed_server.clone();
    let action = format!("remove:{base}");
    let mut button = VNode::element("button")
        .attr("type", "button")
        .attr("class", "allowed-server-remove");
    if busy {
        button = button.attr("disabled", "");
    }
    button = button.handler(
        EventKind::Click,
        ElementHandler::new(move |_event: &mut DomEvent| {
            (intent)(&action);
        }),
    );
    let remove_node = button.child(VNode::text(SERVER_REMOVE_LABEL)).build();
    view! {
        div(class = "allowed-server-row") {
            code(class = "allowed-server-base") { base }
            " "
            { remove_node }
        }
    }
}

/// 「允许连接的自建服务器」的输入 + 添加按钮。
fn render_server_add_row(props: &CapabilityPanelsProps, busy: bool) -> VNode {
    let input_text = props.allowed_server_input.get();
    let field_input = props.on_field_input.clone();
    let field_key = SERVER_INPUT_KEY;
    // ⚠️ **取值由组件声明**（`prop("value", …)` ⇒ **property**，不写属性）：
    // 生产是 `bind:value` ⇒ Svelte 写 property、DOM 上**没有** `value` 属性
    // ⇒ 组件自己带值 ⇒ 宿主不必再按选择器回写。
    let mut input = VNode::element("input")
        .attr("id", SERVER_INPUT_ID)
        .prop("value", props.allowed_server_input.get())
        .attr("type", "text")
        .attr("placeholder", SERVER_INPUT_PLACEHOLDER);
    if busy {
        input = input.attr("disabled", "");
    }
    input = input.handler(
        EventKind::Input,
        ElementHandler::new(move |_event: &mut DomEvent| {
            (field_input)(field_key);
        }),
    );
    let keydown = props.on_allowed_server.clone();
    input = input.handler(
        EventKind::KeyDown,
        ElementHandler::new(move |event: &mut DomEvent| {
            // 生产：`if (e.key === 'Enter') handleAddAllowedServer()`。
            if event.key() == Some("Enter") {
                (keydown)("add");
            }
        }),
    );

    let submit = props.on_allowed_server.clone();
    let mut button = VNode::element("button")
        .attr("type", "button")
        .attr("class", "allowed-server-add-btn");
    // 生产：`disabled={allowedServersBusy || !allowedServerInput.trim()}`
    // ⇒ **属性存在性**是两个条件之一为真（`disabled=""`）。
    if busy || input_text.trim().is_empty() {
        button = button.attr("disabled", "");
    }
    button = button.handler(
        EventKind::Click,
        ElementHandler::new(move |_event: &mut DomEvent| {
            (submit)("add");
        }),
    );
    let add_node = button.child(VNode::text(SERVER_ADD_LABEL)).build();
    view! {
        div(class = "allowed-server-add") {
            { input.build() }
            " "
            { add_node }
        }
    }
}

/// 「工具授予」卡的**行序列**（工具行 + 沙箱嵌套行 + 依赖子行 + **MCP 静态行**）。
fn render_tool_card_rows(
    props: &CapabilityPanelsProps,
    table: &CapabilityTable,
    pending_key: Option<&str>,
) -> Vec<VNode> {
    let mut rows: Vec<VNode> = Vec::new();
    for def in props.tool_defs.iter() {
        rows.push(render_one_switch_row(def, table, pending_key, &props.card));
        if def.key.as_str() == SHELL_KEY && capability_bool(table, SHELL_KEY) {
            rows.push(render_sandbox_row(props, table, pending_key));
            if !capability_bool(table, SANDBOX_VALUE_KEY) {
                rows.push(render_sandbox_warning());
            }
        }
        if def.key.as_str() == FILE_WRITE_KEY {
            rows.extend(render_switch_rows(
                &props.tool_sub_defs,
                table,
                pending_key,
                &props.card,
            ));
        }
    }
    // ⚠️ **MCP 静态行必须渲染**：生产的它是 `{#each}` 之后的最后一个元素
    // ⇒ 少了它，依赖子行会成为 `:last-child` ⇒ 生产 `.cap-row:last-child`
    // 去掉下边框 ⇒ 引擎多 1px ⇒ **高度差 1px**。
    rows.push(render_mcp_row());
    rows
}

/// 沙箱嵌套行（**`shell` 打开时才出现**；关 ⇒ `sandbox-off` + 图标换成 `shield-off`）。
fn render_sandbox_row(
    props: &CapabilityPanelsProps,
    table: &CapabilityTable,
    pending_key: Option<&str>,
) -> VNode {
    let on = capability_bool(table, SANDBOX_VALUE_KEY);
    let pending = pending_key == Some(SANDBOX_VALUE_KEY);
    let mut classes: Vec<&str> = vec!["cap-row", "cap-row-nested"];
    if !on {
        classes.push("sandbox-off");
    }
    if pending {
        classes.push("pending");
    }
    let mut button = VNode::element("button")
        .attr("role", "switch")
        .attr("class", classes.join(" "))
        .attr("aria-checked", if on { "true" } else { "false" })
        .attr("aria-label", SANDBOX_ARIA_LABEL);
    // 生产：`disabled={liveApplyPendingKey !== null}`（**没有** `capDisabled`
    // ⇒ 沙箱行**不因父行而禁用**）。
    if pending_key.is_some() {
        button = button.attr("disabled", "");
    }
    let toggle = props.card.on_toggle_capability.clone();
    let key = SANDBOX_VALUE_KEY;
    button = button.handler(
        EventKind::Click,
        ElementHandler::new(move |_event: &mut DomEvent| {
            (toggle)(key);
        }),
    );
    let icon = if on {
        &icons::ICON_SHIELD_CHECK
    } else {
        &icons::ICON_SHIELD_OFF
    };
    // ⚠️ `view!` 里**首字母大写**的裸标识符会被判成**组件** ⇒ 常量先落小写局部变量。
    let sandbox_label = SANDBOX_LABEL;
    let sandbox_env = SANDBOX_ENV;
    let sandbox_desc = SANDBOX_DESC;
    let switch = view! {
        span(class = "cap-switch", class:on = on) {
            span(class = "cap-knob") { }
        }
    };
    button
        .child(view! { span(class = "cap-icon") { { Icon::render(&IconProps::new(icon, 15)) } } })
        .child(view! {
            span(class = "cap-text") {
                strong {
                    sandbox_label
                    code(class = "cap-env") { sandbox_env }
                }
                " "
                small { sandbox_desc }
            }
        })
        .child(switch)
        .build()
}

/// 沙箱关闭时的告警框（`div.cap-warning`）。
fn render_sandbox_warning() -> VNode {
    let warning = SANDBOX_WARNING;
    view! {
        div(class = "cap-warning") {
            { Icon::render(&IconProps::new(&icons::ICON_TRIANGLE_ALERT, 13)) }
            " "
            span { warning }
        }
    }
}

/// MCP 静态行（**无开关** ⇒ `div.cap-row.cap-row-static`）。
fn render_mcp_row() -> VNode {
    render_static_row(
        &icons::ICON_SERVER,
        MCP_LABEL,
        Some(MCP_ENV),
        MCP_DESC,
        FieldFlagState::None,
        VNode::fragment(Vec::new()),
    )
}

// ── 开发者分区 ──────────────────────────────────────────────────────────────

/// 开发者分区的渲染。
fn render_developer(props: &CapabilityPanelsProps) -> VNode {
    let table = props.capabilities.get();
    let pending = props.pending_key.get();
    let flags = props.text_flags.get();
    let copied = props.config_json_copied.get();
    let preview = props.config_json_preview.get();

    let heading = DEVELOPER_TITLE;
    let desc = DEVELOPER_DESC;
    let contract_badge = CONTRACT_BADGE;
    let contract_title = CONTRACT_TITLE;
    let contract_text = CONTRACT_TEXT;
    let config_label = CONFIG_GROUP_LABEL;
    let copy_label = if copied { COPIED_LABEL } else { COPY_LABEL };

    let mut beta_rows =
        render_switch_rows(&props.beta_defs, &table, pending.as_deref(), &props.card);
    if capability_bool(&table, REASONING_ENABLED_KEY) {
        beta_rows.push(render_reasoning_group(props, &flags));
    }
    let beta_card = render_card(
        &icons::ICON_SPARKLES,
        BETA_CARD_TITLE,
        &format!(
            "{}/{}",
            enabled_count(&props.beta_defs, &table),
            props.beta_defs.len()
        ),
        beta_rows,
    );
    let contract_card = render_default_card(
        &icons::ICON_INFO,
        false,
        contract_badge,
        contract_title,
        contract_text,
    );

    let copy_intent = props.on_copy_config.clone();
    let copy_key = COPY_CONTROL_KEY;
    let copy_icon = if copied {
        &icons::ICON_CHECK
    } else {
        &icons::ICON_COPY
    };
    let copy_button = VNode::element("button")
        .attr("class", "quiet-button copy-btn")
        .handler(
            EventKind::Click,
            ElementHandler::new(move |_event: &mut DomEvent| {
                (copy_intent)(copy_key);
            }),
        )
        .child(Icon::render(&IconProps::new(copy_icon, 12)))
        .child(VNode::text(" "))
        .child(view! { span { copy_label } })
        .build();
    let code_box = view! { pre(class = "code-box") { preview } };

    view! {
        div(class = "setting-block") {
            h3(class = "block-title") { heading }
            " "
            p(class = "block-desc") { desc }
            " "
            { beta_card }
            " "
            { contract_card }
            " "
            div(class = "form-group") {
                label(for = CONFIG_LABEL_FOR) { config_label }
                " "
                div(class = "code-box-wrap") {
                    { copy_button }
                    " "
                    { code_box }
                }
            }
        }
    }
}

/// 思考模式文本旋钮组（**`reasoningEnabled` 打开时才出现**）。
fn render_reasoning_group(
    props: &CapabilityPanelsProps,
    flags: &[(String, FieldFlagState)],
) -> VNode {
    let filters_row = render_reasoning_filters_row(props, flag_of(flags, FLAG_REASONING_FILTERS));
    let tag_row = render_reasoning_tag_row(props, flag_of(flags, FLAG_REASONING_TAG));
    let focus_out = props.on_submit_group.clone();
    let focus_key = REASONING_GROUP_KEY;
    let keydown = props.on_submit_group.clone();
    let keydown_key = REASONING_GROUP_KEY;
    view! {
        div(
            class = "text-group",
            role = "group",
            aria-label = REASONING_GROUP_ARIA_LABEL,
            onfocusout = move |_event: &mut DomEvent| { (focus_out)(focus_key); },
            onkeydown = move |_event: &mut DomEvent| { (keydown)(keydown_key); }
        ) {
            { filters_row }
            " "
            { tag_row }
        }
    }
}

/// 「生效模型过滤器」行（**原生 `input[type=text]`** ⇒ 取值是 property）。
fn render_reasoning_filters_row(props: &CapabilityPanelsProps, flag: FieldFlagState) -> VNode {
    let field_input = props.on_field_input.clone();
    let field_key = REASONING_FILTERS_VALUE_KEY;
    let input = VNode::element("input")
        .attr("type", "text")
        .attr("placeholder", REASONING_FILTERS_PLACEHOLDER)
        .attr("aria-label", REASONING_FILTERS_ARIA_LABEL)
        .handler(
            EventKind::Input,
            ElementHandler::new(move |_event: &mut DomEvent| {
                (field_input)(field_key);
            }),
        )
        .build();
    let wrap = VNode::element("span")
        .attr("class", "cap-input-wrap")
        .child(input)
        .build();
    render_static_row(
        &icons::ICON_FILTER,
        REASONING_FILTERS_LABEL,
        Some(REASONING_FILTERS_ENV),
        REASONING_FILTERS_DESC,
        flag,
        wrap,
    )
}

/// 「reasoning 标签」行（**没有 placeholder**）。
fn render_reasoning_tag_row(props: &CapabilityPanelsProps, flag: FieldFlagState) -> VNode {
    let field_input = props.on_field_input.clone();
    let field_key = REASONING_TAG_VALUE_KEY;
    let input = VNode::element("input")
        .attr("type", "text")
        .attr("aria-label", REASONING_TAG_ARIA_LABEL)
        .handler(
            EventKind::Input,
            ElementHandler::new(move |_event: &mut DomEvent| {
                (field_input)(field_key);
            }),
        )
        .build();
    let wrap = VNode::element("span")
        .attr("class", "cap-input-wrap")
        .child(input)
        .build();
    render_static_row(
        &icons::ICON_TAG,
        REASONING_TAG_LABEL,
        Some(REASONING_TAG_ENV),
        REASONING_TAG_DESC,
        flag,
        wrap,
    )
}

#[cfg(test)]
mod tests {
    //! 能力与治理面的**行为**断言（渲染结果 ⇒ 结构 / class / 文本 / 属性），
    //! **不读生产源码**（把生产行为钉在渲染结果上）。

    use std::cell::RefCell;

    use super::*;
    use crate::settings_character::CapabilityValue;

    /// 收集全部 class token（去重、保序）。
    fn classes(node: &VNode, out: &mut Vec<String>) {
        for token in node.class_tokens() {
            if !out.contains(&token) {
                out.push(token);
            }
        }
        for child in node.children() {
            classes(child, out);
        }
    }

    /// 数某个 class token 出现了多少次（**不去重**）。
    fn count_class(node: &VNode, token: &str) -> usize {
        let mut count = 0usize;
        if node
            .class_tokens()
            .iter()
            .any(|candidate| candidate == token)
        {
            count += 1;
        }
        for child in node.children() {
            count += count_class(child, token);
        }
        count
    }

    /// 收集文本节点（**逐个**，含空白）。
    fn text_nodes(node: &VNode, out: &mut Vec<String>) {
        match node {
            VNode::Element(element) => {
                for child in &element.children {
                    text_nodes(child, out);
                }
            }
            VNode::Text(text) => out.push(text.data.clone()),
            VNode::Fragment(children) => {
                for child in children {
                    text_nodes(child, out);
                }
            }
        }
    }

    /// 收集文本（同一棵树 ⇒ 拼成一个字符串，便于「含某段文案」断言）。
    fn all_text(node: &VNode) -> String {
        let mut texts = Vec::new();
        text_nodes(node, &mut texts);
        texts.join("")
    }

    /// 找某个 class token 的元素（**保序**）。
    fn find_by_class<'a>(node: &'a VNode, token: &str, out: &mut Vec<&'a VNode>) {
        if node
            .class_tokens()
            .iter()
            .any(|candidate| candidate == token)
        {
            out.push(node);
        }
        for child in node.children() {
            find_by_class(child, token, out);
        }
    }

    /// 找某个标签的元素（**保序**）。
    fn find_by_tag<'a>(node: &'a VNode, tag: &str, out: &mut Vec<&'a VNode>) {
        if let VNode::Element(element) = node {
            if element.tag == tag {
                out.push(node);
            }
        }
        for child in node.children() {
            find_by_tag(child, tag, out);
        }
    }

    /// 读一个属性（缺失 ⇒ `None`）。
    fn attr_of(node: &VNode, name: &str) -> Option<String> {
        match node {
            VNode::Element(element) => element
                .attrs
                .iter()
                .find(|(key, _)| key == name)
                .and_then(|(_, value)| value.to_attr_string()),
            _ => None,
        }
    }

    /// 建一份能力定义（**测试用** ⇒ 文案与真实表无关，结构才重要）。
    fn def(key: &str, icon: &'static IconData, label: &str, env: &str) -> CapabilityDef {
        CapabilityDef::new(key, icon, label, env, "说明")
    }

    /// 建一份能力表。
    fn table(entries: &[(&str, CapabilityValue)]) -> CapabilityTable {
        let mut table = CapabilityTable::new();
        for (key, value) in entries {
            table.insert(String::from(*key), *value);
        }
        table
    }

    /// 五个注册表（decision 2 / guard 2 / tool 4 / toolSub 1 / beta 1 ⇒ 与生产同形）。
    type Registries = (
        Vec<CapabilityDef>,
        Vec<CapabilityDef>,
        Vec<CapabilityDef>,
        Vec<CapabilityDef>,
        Vec<CapabilityDef>,
    );

    fn registries() -> Registries {
        let decision = vec![
            def(
                "judge",
                &icons::ICON_SCALE,
                "评审",
                "APEIRETH_COGNITIVE_JUDGE",
            ),
            def(
                "council",
                &icons::ICON_USERS,
                "议会",
                "APEIRETH_COGNITIVE_COUNCIL",
            ),
        ];
        let guard = vec![
            def(
                "onionLayer",
                &icons::ICON_SHIELD_CHECK,
                "三洋葱治理层",
                "APEIRETH_ENABLE_ONION_LAYER",
            ),
            def(
                "worktreeSandbox",
                &icons::ICON_WORKFLOW,
                "子代理 worktree 隔离",
                "APEIRETH_ENABLE_WORKTREE_SANDBOX",
            ),
        ];
        let tool = vec![
            def(
                "shell",
                &icons::ICON_TERMINAL,
                "Shell 命令工具",
                "APEIRETH_ENABLE_SHELL",
            ),
            def(
                "fetch",
                &icons::ICON_GLOBE,
                "网络读取工具",
                "APEIRETH_ENABLE_FETCH",
            ),
            def(
                "localReadTools",
                &icons::ICON_FOLDER_SEARCH,
                "本地只读工具",
                "APEIRETH_ENABLE_LOCAL_READ_TOOLS",
            ),
            def(
                "fileWrite",
                &icons::ICON_FILE_PEN_LINE,
                "文件写入",
                "APEIRETH_ENABLE_FILE_WRITE",
            ),
        ];
        let tool_sub = vec![def(
            "fileWriteAutoPass",
            &icons::ICON_FILE_CHECK_2,
            "自动放行已读文件修改",
            "APEIRETH_ENABLE_FILE_WRITE_AUTO_PASS",
        )
        .requires("fileWrite")
        .with_row_class("cap-row-nested")];
        let beta = vec![def(
            "reasoningEnabled",
            &icons::ICON_BRAIN,
            "思考模式",
            "APEIRETH_REASONING_ENABLED",
        )];
        (decision, guard, tool, tool_sub, beta)
    }

    /// 建一份 props（意图记进一个共享的 `Vec`）。
    ///
    /// ⚠️ 返回值带上 `Runtime`：**改某个信号的值**要新建一个信号 ⇒ 测试里
    /// 需要 Runtime 来建它。
    fn props(
        section: &str,
        entries: &[(&str, CapabilityValue)],
    ) -> (Runtime, CapabilityPanelsProps) {
        let runtime = Runtime::new();
        let intents: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));
        let toggle_intents = intents.clone();
        let knob_intents = intents.clone();
        let toggle: CapabilityIntent = Rc::new(move |key: &str| {
            toggle_intents.borrow_mut().push(format!("toggle:{key}"));
        });
        let knob: KnobIntent = Rc::new(move |key: &str| {
            knob_intents.borrow_mut().push(format!("knob:{key}"));
        });
        let card = card_bridge(&runtime, toggle, knob);
        let (decision, guard, tool, tool_sub, beta) = registries();
        let record = |name: &'static str, intents: Rc<RefCell<Vec<String>>>| -> ActionIntent {
            Rc::new(move |key: &str| {
                intents.borrow_mut().push(format!("{name}:{key}"));
            })
        };
        let capabilities = runtime.signal(table(entries));
        let section_signal = runtime.signal(String::from(section));
        let props = CapabilityPanelsProps {
            section: section_signal,
            capabilities,
            pending_key: runtime.signal(None),
            cognitive_depth: runtime.signal(String::from("light")),
            permission_preset: runtime.signal(String::from("standard")),
            allowed_servers: runtime.signal(Vec::new()),
            allowed_server_input: runtime.signal(String::new()),
            allowed_server_error: runtime.signal(String::new()),
            allowed_servers_busy: runtime.signal(false),
            text_flags: runtime.signal(Vec::new()),
            config_json_preview: runtime.signal(String::from("{}")),
            config_json_copied: runtime.signal(false),
            decision_defs: decision,
            guard_defs: guard,
            tool_defs: tool,
            tool_sub_defs: tool_sub,
            beta_defs: beta,
            card,
            on_select_depth: record("depth", intents.clone()),
            on_select_preset: record("preset", intents.clone()),
            on_allowed_server: record("server", intents.clone()),
            on_copy_config: record("copy", intents.clone()),
            on_field_input: record("field", intents.clone()),
            on_submit_group: record("submit", intents.clone()),
        };
        (runtime, props)
    }

    /// 常用形态：建 props ⇒ 直接渲染。
    fn render(section: &str, entries: &[(&str, CapabilityValue)]) -> VNode {
        let (_runtime, props) = props(section, entries);
        CapabilityPanels::render(&props)
    }

    #[test]
    fn the_governance_section_renders_the_production_class_names() {
        let node = render(SECTION_GOVERNANCE, &[]);
        let mut found = Vec::new();
        classes(&node, &mut found);
        for class in [
            "setting-block",
            "block-title",
            "block-desc",
            "cap-summary",
            "cap-card",
            "cap-card-head",
            "cap-card-title",
            "cap-card-count",
            "cap-row",
            "cap-row-static",
            "cap-icon",
            "cap-text",
            "cap-env",
            "cap-select-wrap",
            "cap-switch",
            "cap-knob",
            "cap-default-card",
            "cap-default-badge",
            "cap-default-title",
            "cap-default-text",
            "note",
        ] {
            assert!(
                found.iter().any(|token| token == class),
                "缺 class：{class}"
            );
        }
        // 分区标题与说明**逐字**来自生产（含说明里的换行 + 缩进）。
        let text = all_text(&node);
        assert!(text.contains(GOVERNANCE_TITLE));
        assert!(
            text.contains(GOVERNANCE_DESC),
            "说明必须逐字（含内部换行 + 缩进）"
        );
    }

    #[test]
    fn the_summary_counts_only_rows_that_are_on_and_available() {
        // judge 开 / council 关 / onionLayer 开 / worktreeSandbox 关 ⇒ 2/4。
        let node = render(
            SECTION_GOVERNANCE,
            &[
                ("judge", CapabilityValue::Bool(true)),
                ("council", CapabilityValue::Bool(false)),
                ("onionLayer", CapabilityValue::Bool(true)),
                ("worktreeSandbox", CapabilityValue::Bool(false)),
            ],
        );
        let mut texts = Vec::new();
        text_nodes(&node, &mut texts);
        assert!(
            texts.iter().any(|text| text == " / 4 项治理件"),
            "摘要后缀必须是**一个**含总数的文本节点：{texts:?}"
        );
        assert!(
            texts.iter().any(|text| text == "2"),
            "已启用计数应为 2：{texts:?}"
        );
        assert!(
            texts.iter().any(|text| text == "1/2"),
            "决策卡计数应为 1/2：{texts:?}"
        );
    }

    #[test]
    fn the_council_rows_appear_only_when_the_council_is_on() {
        let off = render(
            SECTION_GOVERNANCE,
            &[("council", CapabilityValue::Bool(false))],
        );
        let text = all_text(&off);
        assert!(!text.contains(ADVISORS_LABEL), "议会关 ⇒ 顾问数量行不渲染");
        assert!(!text.contains(TIMEOUT_LABEL), "议会关 ⇒ 超时行不渲染");

        let on = render(
            SECTION_GOVERNANCE,
            &[
                ("council", CapabilityValue::Bool(true)),
                ("councilAdvisors", CapabilityValue::Number(3.0)),
                ("councilTimeoutMs", CapabilityValue::Number(30000.0)),
            ],
        );
        let text = all_text(&on);
        assert!(text.contains(ADVISORS_LABEL));
        assert!(text.contains(TIMEOUT_LABEL));
        assert!(text.contains(ADVISORS_ENV));
        assert!(text.contains(TIMEOUT_ENV));
        // 顾问数量的取值展示（`<b>3</b>`）。
        let mut texts = Vec::new();
        text_nodes(&on, &mut texts);
        assert!(texts.iter().any(|item| item == "3"));
        // 两个原生控件的 aria-label 必须逐字来自生产。
        let mut inputs = Vec::new();
        find_by_tag(&on, "input", &mut inputs);
        let labels: Vec<String> = inputs
            .iter()
            .filter_map(|node| attr_of(node, "aria-label"))
            .collect();
        assert!(labels.iter().any(|label| label == ADVISORS_ARIA_LABEL));
        assert!(labels.iter().any(|label| label == TIMEOUT_ARIA_LABEL));
    }

    #[test]
    fn a_switch_row_carries_the_production_attributes() {
        let node = render(
            SECTION_GOVERNANCE,
            &[("judge", CapabilityValue::Bool(true))],
        );
        let mut buttons = Vec::new();
        find_by_tag(&node, "button", &mut buttons);
        let judge = buttons
            .iter()
            .find(|node| attr_of(node, "aria-label").as_deref() == Some("评审"))
            .expect("找不到评审批次行");
        assert_eq!(attr_of(judge, "role").as_deref(), Some("switch"));
        assert_eq!(attr_of(judge, "aria-checked").as_deref(), Some("true"));
        assert_eq!(attr_of(judge, "class").as_deref(), Some("cap-row"));
        // 未生效 ⇒ `aria-checked="false"`；**不是**布尔属性。
        let other = buttons
            .iter()
            .find(|node| attr_of(node, "aria-label").as_deref() == Some("议会"))
            .expect("找不到议会行");
        assert_eq!(attr_of(other, "aria-checked").as_deref(), Some("false"));
    }

    #[test]
    fn the_dependent_row_is_dim_and_disabled_as_an_empty_string() {
        let off = render(
            SECTION_TOOLS,
            &[("fileWrite", CapabilityValue::Bool(false))],
        );
        let mut rows = Vec::new();
        find_by_class(&off, "cap-row-nested", &mut rows);
        assert_eq!(rows.len(), 1, "父行关 ⇒ 依赖子行仍在（**dim** 不是消失）");
        let class = attr_of(rows[0], "class").unwrap_or_default();
        assert!(class.contains("dim"), "父行关 ⇒ 必须带 dim：{class}");
        // ⚠️ 静态 disabled 是**空串**（不是缺失、也不是 "true"）。
        assert_eq!(attr_of(rows[0], "disabled").as_deref(), Some(""));

        let on = render(SECTION_TOOLS, &[("fileWrite", CapabilityValue::Bool(true))]);
        let mut rows = Vec::new();
        find_by_class(&on, "cap-row-nested", &mut rows);
        assert_eq!(rows.len(), 1);
        let class = attr_of(rows[0], "class").unwrap_or_default();
        assert!(!class.contains("dim"), "父行开 ⇒ 不能带 dim：{class}");
        assert_eq!(
            attr_of(rows[0], "disabled"),
            None,
            "父行开 ⇒ 不能有 disabled 属性"
        );
    }

    #[test]
    fn the_mcp_row_stays_the_last_row_and_the_dependent_row_is_a_sibling() {
        let node = render(
            SECTION_TOOLS,
            &[("fileWrite", CapabilityValue::Bool(false))],
        );
        let mut cards = Vec::new();
        find_by_class(&node, "cap-card", &mut cards);
        assert_eq!(cards.len(), 1, "工具分区只有一张能力卡");
        let elements: Vec<&VNode> = cards[0]
            .children()
            .iter()
            .filter(|child| matches!(child, VNode::Element(_)))
            .collect();
        // 最后一行 = MCP 静态行（**必须渲染** ⇒ 否则依赖子行成 `:last-child`）。
        let last = elements.last().expect("能力卡至少有一行");
        assert!(
            last.class_tokens()
                .iter()
                .any(|token| token == "cap-row-static"),
            "最后一行必须是静态行（MCP）"
        );
        let last_text = all_text(last);
        assert!(last_text.contains(MCP_LABEL));
        assert!(last_text.contains(MCP_ENV));

        // 依赖子行是**父行的下一个兄弟**（同层），**不是**父行的子节点。
        let position = elements
            .iter()
            .position(|row| all_text(row).contains("APEIRETH_ENABLE_FILE_WRITE"))
            .expect("找不到 fileWrite 父行");
        let nested = elements[position + 1];
        let nested_class = attr_of(nested, "class").unwrap_or_default();
        assert!(
            nested_class.contains("cap-row-nested"),
            "父行之后紧跟的必须是嵌套行（同层）：{nested_class}"
        );
        let parent_children: Vec<String> = match elements[position] {
            VNode::Element(element) => element.children.iter().map(all_text).collect(),
            _ => Vec::new(),
        };
        assert!(
            !parent_children
                .iter()
                .any(|text| text.contains("APEIRETH_ENABLE_FILE_WRITE_AUTO_PASS")),
            "依赖子行不能是父行的子节点"
        );
    }

    #[test]
    fn the_sandbox_row_swaps_its_icon_and_adds_sandbox_off() {
        let off = render(
            SECTION_TOOLS,
            &[
                ("shell", CapabilityValue::Bool(true)),
                ("shellSandbox", CapabilityValue::Bool(false)),
            ],
        );
        let mut rows = Vec::new();
        find_by_class(&off, "cap-row-nested", &mut rows);
        let sandbox = rows
            .iter()
            .find(|row| attr_of(row, "aria-label").as_deref() == Some(SANDBOX_ARIA_LABEL))
            .expect("沙箱行没渲染");
        let class = attr_of(sandbox, "class").unwrap_or_default();
        assert!(
            class.contains("sandbox-off"),
            "沙箱关 ⇒ sandbox-off：{class}"
        );
        assert!(!class.contains("dim"), "沙箱行没有 dim：{class}");
        assert_eq!(attr_of(sandbox, "aria-checked").as_deref(), Some("false"));
        let mut icons = Vec::new();
        find_by_class(sandbox, "lucide-shield-off", &mut icons);
        assert_eq!(icons.len(), 1, "沙箱关 ⇒ shield-off 图标");

        let on = render(
            SECTION_TOOLS,
            &[
                ("shell", CapabilityValue::Bool(true)),
                ("shellSandbox", CapabilityValue::Bool(true)),
            ],
        );
        let mut rows = Vec::new();
        find_by_class(&on, "cap-row-nested", &mut rows);
        let sandbox = rows
            .iter()
            .find(|row| attr_of(row, "aria-label").as_deref() == Some(SANDBOX_ARIA_LABEL))
            .expect("沙箱行没渲染");
        assert_eq!(
            attr_of(sandbox, "class").as_deref(),
            Some("cap-row cap-row-nested")
        );
        let mut icons = Vec::new();
        find_by_class(sandbox, "lucide-shield-check", &mut icons);
        assert_eq!(icons.len(), 1, "沙箱开 ⇒ shield-check 图标");
    }

    #[test]
    fn the_warning_box_appears_only_when_the_sandbox_is_off() {
        let off = render(
            SECTION_TOOLS,
            &[
                ("shell", CapabilityValue::Bool(true)),
                ("shellSandbox", CapabilityValue::Bool(false)),
            ],
        );
        assert_eq!(count_class(&off, "cap-warning"), 1);
        assert!(all_text(&off).contains(SANDBOX_WARNING));
        let on = render(
            SECTION_TOOLS,
            &[
                ("shell", CapabilityValue::Bool(true)),
                ("shellSandbox", CapabilityValue::Bool(true)),
            ],
        );
        assert_eq!(count_class(&on, "cap-warning"), 0);
        // shell 关 ⇒ 沙箱行与告警都不出现。
        let no_shell = render(SECTION_TOOLS, &[("shell", CapabilityValue::Bool(false))]);
        assert_eq!(count_class(&no_shell, "cap-warning"), 0);
        let mut rows = Vec::new();
        find_by_class(&no_shell, "cap-row-nested", &mut rows);
        assert!(
            rows.iter()
                .all(|row| attr_of(row, "aria-label").as_deref() != Some(SANDBOX_ARIA_LABEL)),
            "shell 关 ⇒ 沙箱行不渲染"
        );
    }

    #[test]
    fn the_developer_section_shows_the_text_group_only_when_reasoning_is_on() {
        let off = render(
            SECTION_DEVELOPER,
            &[("reasoningEnabled", CapabilityValue::Bool(false))],
        );
        assert_eq!(count_class(&off, "text-group"), 0);
        let on = render(
            SECTION_DEVELOPER,
            &[("reasoningEnabled", CapabilityValue::Bool(true))],
        );
        assert_eq!(count_class(&on, "text-group"), 1);
        let mut groups = Vec::new();
        find_by_class(&on, "text-group", &mut groups);
        assert_eq!(attr_of(groups[0], "role").as_deref(), Some("group"));
        assert_eq!(
            attr_of(groups[0], "aria-label").as_deref(),
            Some(REASONING_GROUP_ARIA_LABEL)
        );
        let mut inputs = Vec::new();
        find_by_tag(&on, "input", &mut inputs);
        let labels: Vec<String> = inputs
            .iter()
            .filter_map(|node| attr_of(node, "aria-label"))
            .collect();
        assert!(labels
            .iter()
            .any(|label| label == REASONING_FILTERS_ARIA_LABEL));
        assert!(labels.iter().any(|label| label == REASONING_TAG_ARIA_LABEL));
        // 过滤器有 placeholder，标签**没有**。
        let filters = inputs
            .iter()
            .find(|node| {
                attr_of(node, "aria-label").as_deref() == Some(REASONING_FILTERS_ARIA_LABEL)
            })
            .expect("找不到过滤器输入框");
        assert_eq!(
            attr_of(filters, "placeholder").as_deref(),
            Some(REASONING_FILTERS_PLACEHOLDER)
        );
        let tag = inputs
            .iter()
            .find(|node| attr_of(node, "aria-label").as_deref() == Some(REASONING_TAG_ARIA_LABEL))
            .expect("找不到标签输入框");
        assert_eq!(attr_of(tag, "placeholder"), None);
    }

    #[test]
    fn the_config_preview_is_rendered_verbatim() {
        let preview = "{\n  \"baseUrl\": \"http://127.0.0.1:8080\",\n  \"hasApiKey\": false\n}";
        let (runtime, mut props) = props(SECTION_DEVELOPER, &[]);
        props.config_json_preview = runtime.signal(String::from(preview));
        let node = CapabilityPanels::render(&props);
        let mut pres = Vec::new();
        find_by_tag(&node, "pre", &mut pres);
        assert_eq!(pres.len(), 1);
        assert_eq!(attr_of(pres[0], "class").as_deref(), Some("code-box"));
        let mut texts = Vec::new();
        text_nodes(pres[0], &mut texts);
        assert_eq!(
            texts,
            vec![String::from(preview)],
            "预览必须逐字落**一个**文本节点"
        );
    }

    #[test]
    fn the_native_controls_do_not_carry_value_attributes() {
        // T4：取值是 **property** ⇒ DOM 上**没有** `value` 属性（存在性也要对）。
        let node = render(
            SECTION_GOVERNANCE,
            &[("council", CapabilityValue::Bool(true))],
        );
        let mut selects = Vec::new();
        find_by_tag(&node, "select", &mut selects);
        assert_eq!(selects.len(), 1);
        assert_eq!(attr_of(selects[0], "id").as_deref(), Some(DEPTH_SELECT_ID));
        assert_eq!(
            attr_of(selects[0], "value"),
            None,
            "select 不能有 value 属性"
        );
        let mut inputs = Vec::new();
        find_by_tag(&node, "input", &mut inputs);
        for input in &inputs {
            assert_eq!(attr_of(input, "value"), None, "原生控件不能有 value 属性");
        }
        // `number` 行**没有 max**（生产 `max={… ?? undefined}` ⇒ 属性不出现）。
        let number = inputs
            .iter()
            .find(|node| attr_of(node, "type").as_deref() == Some("number"))
            .expect("找不到超时输入框");
        assert_eq!(attr_of(number, "min").as_deref(), Some("1000"));
        assert_eq!(attr_of(number, "step").as_deref(), Some("5000"));
        assert_eq!(attr_of(number, "max"), None, "max 不出现 ≠ max=\"\"");
    }

    #[test]
    fn the_preset_select_carries_the_three_production_options() {
        let node = render(SECTION_TOOLS, &[]);
        let mut selects = Vec::new();
        find_by_tag(&node, "select", &mut selects);
        let preset = selects
            .iter()
            .find(|node| attr_of(node, "id").as_deref() == Some(PRESET_SELECT_ID))
            .expect("找不到全局权限预设下拉");
        let mut options = Vec::new();
        find_by_tag(preset, "option", &mut options);
        assert_eq!(options.len(), PRESET_OPTIONS.len());
        let text = all_text(preset);
        for (value, caption) in PRESET_OPTIONS {
            assert!(options
                .iter()
                .any(|option| attr_of(option, "value").as_deref() == Some(value)));
            assert!(text.contains(caption), "缺选项文案：{caption}");
        }
    }

    #[test]
    fn the_copy_button_switches_between_two_labels() {
        let off = render(SECTION_DEVELOPER, &[]);
        let mut buttons = Vec::new();
        find_by_class(&off, "copy-btn", &mut buttons);
        assert_eq!(buttons.len(), 1);
        assert!(all_text(buttons[0]).contains(COPY_LABEL));

        let (runtime, mut props) = props(SECTION_DEVELOPER, &[]);
        props.config_json_copied = runtime.signal(true);
        let on = CapabilityPanels::render(&props);
        let mut buttons = Vec::new();
        find_by_class(&on, "copy-btn", &mut buttons);
        assert_eq!(buttons.len(), 1);
        assert!(all_text(buttons[0]).contains(COPIED_LABEL));
        let mut icons = Vec::new();
        find_by_class(buttons[0], "lucide-check", &mut icons);
        assert_eq!(icons.len(), 1, "已复制 ⇒ check 图标");
    }

    #[test]
    fn the_pending_key_disables_every_switch_row_and_marks_one() {
        let (runtime, mut props) = props(SECTION_GOVERNANCE, &[]);
        props.pending_key = runtime.signal(Some(String::from("judge")));
        let node = CapabilityPanels::render(&props);
        let mut buttons = Vec::new();
        find_by_tag(&node, "button", &mut buttons);
        assert!(!buttons.is_empty());
        for button in buttons {
            assert_eq!(
                attr_of(button, "disabled").as_deref(),
                Some(""),
                "pending ⇒ 所有开关行禁用（disabled=\"\"）"
            );
        }
        assert_eq!(count_class(&node, "pending"), 1, "只有目标行带 pending");
    }

    #[test]
    fn the_fallback_section_renders_the_developer_branch() {
        // 生产的分支顺序：`{:else}` 兜底 = developer ⇒ 未知分区也走它。
        let node = render("unknown-section", &[]);
        let text = all_text(&node);
        assert!(text.contains(DEVELOPER_TITLE));
        assert!(!text.contains(GOVERNANCE_TITLE));
        assert!(is_known_section(SECTION_DEVELOPER));
        assert!(!is_known_section("security"), "security 分区不属本面");
    }
}
