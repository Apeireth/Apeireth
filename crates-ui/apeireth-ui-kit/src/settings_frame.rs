//! 页面框架与导航（**设置页 · 框架面**）。
//!
//! ## 本面是什么
//!
//! 生产 `SettingsView.svelte` 的**框架区**（markup 1922–1993，72 行）：
//! 页头位 + `.settings-layout` 网格 + `.settings-subnav` 侧栏（5 大类 → 15 分区
//! 两级导航）+ `.settings-content` 内容列（应用态条 / 错误横幅位 / 大类说明条）。
//!
//! 本面是**几何基准**：其它内容面的对照舞台宽度（内容盒 **828px**）由本面的
//! `.settings-layout`（`grid-template-columns: 200px 1fr`）与 `.settings-content`
//! （`max-width: 900px` + `padding: 24px 36px 48px`）决定 ⇒ **侧栏恒 200px、
//! 内容列 border-box 恒 900px、内容盒恒 828px**。
//!
//! ## 逐字照抄的**真值口径**（全部来自生产 DOM 的**浏览器实测 dump**）
//!
//! | 事项 | 口径 |
//! |---|---|
//! | 侧栏根 | `<aside class="settings-subnav">` |
//! | 大类行根 | `<div class="subnav-group">` + 条件 class `expanded` |
//! | 大类行按钮 | `<button class="subnav-group-btn">` + 条件 class `active` + `aria-expanded`（**恒存在**；取值是字符串 `"true"` / `"false"`，**不是布尔属性**） |
//! | 大类行图标 | `<group.icon size={15} />`（**15px**） |
//! | 大类标题 | `<span class="subnav-group-name">` |
//! | 展开指示 | `<span class="subnav-chevron">` + 条件 class `open` + `<ChevronDown size={13} />`（**13px**） |
//! | 分区容器 | `<div class="subnav-group-pages">`（**只在展开时出现**） |
//! | 分区行 | `<button class="subnav-btn">` + 条件 class `active` + `<sec.icon size={14} />`（**14px**）+ `<span>{label}</span>` |
//! | 展开判定 | `expandedGroups.includes(group.id)`（**真值口径**：集合里有它） |
//! | 当前大类 | `activeGroup?.id === group.id`（**当前分区所属的大类**，不是「点过的大类」） |
//! | 空白 | 元素之间的换行**折叠成恰好一个空格**（文本节点）；块边界（`{#if}` / `{#each}`）两侧的空白**被裁掉** ⇒ 大类行之间**没有**空白文本节点 |
//!
//! ## 结构分支 vs class 分支（**别搞混**）
//!
//! - **结构分支**：`{#if open}` ⇒ 分区容器 `<div class="subnav-group-pages">`
//!   **整段出现 / 消失**（展开与否 ⇒ 不同的子节点）⇒ 本组件用**真正的条件
//!   渲染**，不是「永远渲染 + 改 class」。
//! - **class 分支**：`class:expanded` / `class:active` / `class:open` ⇒ 只是
//!   **多一个 class token**（元素始终在）。
//!
//! ## 状态归属（**引擎不迁**的部分）
//!
//! `activeSection` / `expandedGroups` / `lastSectionByGroup` 三个状态**由宿主
//! 注入** ⇒ 本组件只承接**展示面 + 受控输入**：点击 ⇒ **发意图**，**不自己
//! 改状态**（意图出口 = [`SettingsFrameProps::on_select_section`] /
//! [`SettingsFrameProps::on_toggle_group`]）。
//!
//! ### `lastSectionByGroup`：**保持非响应式**（与生产同构）
//!
//! 生产里它是**普通 `Map`**（不是 `$state`）⇒ 它**不触发重渲染**；而且它
//! **只被 `toggleGroup` 读**（「展开一个大类 ⇒ 落到该大类最近访问的分区页」）
//! ⇒ **它根本不进渲染**。
//!
//! ⇒ 本组件**不读它**（props 里也没有这一项）⇒ 迁移成信号**不会**带来
//! 「多一次重排」；宿主侧按生产口径把它留成**普通映射**（非响应式）⇒ 行为
//! 与生产**同构**，且**零重排**。
//!
//! ## 大类说明（blurb）是**数据**，不写死
//!
//! 生产里 `blurb` 来自 `settings-nav-groups.ts` ⇒ 本组件只**渲染**它
//! （[`NavGroup::blurb`]），取值由宿主从固定数据喂进来 ⇒ 组件里**没有**
//! 任何一句面向用户的说明文案。
//!
//! ## 图标数据
//!
//! 本面用到的图标几何数据是**构建期抽取**的**生成物**（上游图标包 ⇒ 真实
//! `.svelte` ⇒ `iconNode`）⇒ 见 [`icons`] 模块。class 后缀取**真实文件名**
//! （`Trash2` ⇒ `lucide-trash-2`；`CircleUserRound` ⇒ `lucide-circle-user-round`）。

use std::rc::Rc;

use crate::icon::{Icon, IconData, IconProps};
use apeireth_ui_core::{Signal, UiComponent};
use apeireth_ui_dom::VNode;
use apeireth_ui_macro::view;

/// 本面用到的图标数据（**自动生成** —— 勿手改）。
///
/// 生成器：`crates-ui/apeireth-ui-demo/harness/scripts/extract-assets.mjs`
/// （面 manifest 模式）⇒ 来源：`frontend/companion-desktop/node_modules/
/// lucide-svelte/dist/icons/*.svelte`。
///
/// 上游许可（ISC；其中 Feather 部分为 MIT）：
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
/// WHATSOEVER RESULTING FROM LOSS OF USE, DATA OR PROFITS, WHETHER IN AN ACTION
/// OF CONTRACT, NEGLIGENCE OR OTHER TORTIOUS ACTION, ARISING OUT OF OR IN
/// CONNECTION WITH THE USE OR PERFORMANCE OF THIS SOFTWARE.
pub mod icons {
    use crate::icon::{IconData, IconNode};

    /// 图标 `activity`。
    #[rustfmt::skip]
    pub const ICON_ACTIVITY: IconData = IconData {
        name: "activity",
        class: "lucide-icon lucide lucide-activity",
        nodes: &[
            IconNode { tag: "path", attrs: &[("d", "M22 12h-2.48a2 2 0 0 0-1.93 1.46l-2.35 8.36a.25.25 0 0 1-.48 0L9.24 2.18a.25.25 0 0 0-.48 0l-2.35 8.36A2 2 0 0 1 4.49 12H2")] },
        ],
    };

    /// 图标 `archive`。
    #[rustfmt::skip]
    pub const ICON_ARCHIVE: IconData = IconData {
        name: "archive",
        class: "lucide-icon lucide lucide-archive",
        nodes: &[
            IconNode { tag: "rect", attrs: &[("width", "20"), ("height", "5"), ("x", "2"), ("y", "3"), ("rx", "1")] },
            IconNode { tag: "path", attrs: &[("d", "M4 8v11a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8")] },
            IconNode { tag: "path", attrs: &[("d", "M10 12h4")] },
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

    /// 图标 `check`。
    #[rustfmt::skip]
    pub const ICON_CHECK: IconData = IconData {
        name: "check",
        class: "lucide-icon lucide lucide-check",
        nodes: &[
            IconNode { tag: "path", attrs: &[("d", "M20 6 9 17l-5-5")] },
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

    /// 图标 `circle-user-round`。
    #[rustfmt::skip]
    pub const ICON_CIRCLE_USER_ROUND: IconData = IconData {
        name: "circle-user-round",
        class: "lucide-icon lucide lucide-circle-user-round",
        nodes: &[
            IconNode { tag: "path", attrs: &[("d", "M18 20a6 6 0 0 0-12 0")] },
            IconNode { tag: "circle", attrs: &[("cx", "12"), ("cy", "10"), ("r", "4")] },
            IconNode { tag: "circle", attrs: &[("cx", "12"), ("cy", "12"), ("r", "10")] },
        ],
    };

    /// 图标 `code`。
    #[rustfmt::skip]
    pub const ICON_CODE: IconData = IconData {
        name: "code",
        class: "lucide-icon lucide lucide-code",
        nodes: &[
            IconNode { tag: "polyline", attrs: &[("points", "16 18 22 12 16 6")] },
            IconNode { tag: "polyline", attrs: &[("points", "8 6 2 12 8 18")] },
        ],
    };

    /// 图标 `cpu`。
    #[rustfmt::skip]
    pub const ICON_CPU: IconData = IconData {
        name: "cpu",
        class: "lucide-icon lucide lucide-cpu",
        nodes: &[
            IconNode { tag: "rect", attrs: &[("width", "16"), ("height", "16"), ("x", "4"), ("y", "4"), ("rx", "2")] },
            IconNode { tag: "rect", attrs: &[("width", "6"), ("height", "6"), ("x", "9"), ("y", "9"), ("rx", "1")] },
            IconNode { tag: "path", attrs: &[("d", "M15 2v2")] },
            IconNode { tag: "path", attrs: &[("d", "M15 20v2")] },
            IconNode { tag: "path", attrs: &[("d", "M2 15h2")] },
            IconNode { tag: "path", attrs: &[("d", "M2 9h2")] },
            IconNode { tag: "path", attrs: &[("d", "M20 15h2")] },
            IconNode { tag: "path", attrs: &[("d", "M20 9h2")] },
            IconNode { tag: "path", attrs: &[("d", "M9 2v2")] },
            IconNode { tag: "path", attrs: &[("d", "M9 20v2")] },
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

    /// 图标 `gauge`。
    #[rustfmt::skip]
    pub const ICON_GAUGE: IconData = IconData {
        name: "gauge",
        class: "lucide-icon lucide lucide-gauge",
        nodes: &[
            IconNode { tag: "path", attrs: &[("d", "m12 14 4-4")] },
            IconNode { tag: "path", attrs: &[("d", "M3.34 19a10 10 0 1 1 17.32 0")] },
        ],
    };

    /// 图标 `palette`。
    #[rustfmt::skip]
    pub const ICON_PALETTE: IconData = IconData {
        name: "palette",
        class: "lucide-icon lucide lucide-palette",
        nodes: &[
            IconNode { tag: "circle", attrs: &[("cx", "13.5"), ("cy", "6.5"), ("r", ".5"), ("fill", "currentColor")] },
            IconNode { tag: "circle", attrs: &[("cx", "17.5"), ("cy", "10.5"), ("r", ".5"), ("fill", "currentColor")] },
            IconNode { tag: "circle", attrs: &[("cx", "8.5"), ("cy", "7.5"), ("r", ".5"), ("fill", "currentColor")] },
            IconNode { tag: "circle", attrs: &[("cx", "6.5"), ("cy", "12.5"), ("r", ".5"), ("fill", "currentColor")] },
            IconNode { tag: "path", attrs: &[("d", "M12 2C6.5 2 2 6.5 2 12s4.5 10 10 10c.926 0 1.648-.746 1.648-1.688 0-.437-.18-.835-.437-1.125-.29-.289-.438-.652-.438-1.125a1.64 1.64 0 0 1 1.668-1.668h1.996c3.051 0 5.555-2.503 5.555-5.554C21.965 6.012 17.461 2 12 2z")] },
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

    /// 图标 `smile`。
    #[rustfmt::skip]
    pub const ICON_SMILE: IconData = IconData {
        name: "smile",
        class: "lucide-icon lucide lucide-smile",
        nodes: &[
            IconNode { tag: "circle", attrs: &[("cx", "12"), ("cy", "12"), ("r", "10")] },
            IconNode { tag: "path", attrs: &[("d", "M8 14s1.5 2 4 2 4-2 4-2")] },
            IconNode { tag: "line", attrs: &[("x1", "9"), ("x2", "9.01"), ("y1", "9"), ("y2", "9")] },
            IconNode { tag: "line", attrs: &[("x1", "15"), ("x2", "15.01"), ("y1", "9"), ("y2", "9")] },
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

    /// 图标 `user`。
    #[rustfmt::skip]
    pub const ICON_USER: IconData = IconData {
        name: "user",
        class: "lucide-icon lucide lucide-user",
        nodes: &[
            IconNode { tag: "path", attrs: &[("d", "M19 21v-2a4 4 0 0 0-4-4H9a4 4 0 0 0-4 4v2")] },
            IconNode { tag: "circle", attrs: &[("cx", "12"), ("cy", "7"), ("r", "4")] },
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
}

/// 侧栏里的一个**分区**（第二级）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NavSection {
    /// 分区 id（**深链契约** ⇒ 逐字沿用生产 id）。
    pub id: String,
    /// 显示名。
    pub label: String,
    /// 图标（**几何数据来自上游包** ⇒ 两侧逐字一致）。
    pub icon: &'static IconData,
}

/// 侧栏里的一个**大类**（第一级）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NavGroup {
    /// 大类 id。
    pub id: String,
    /// 显示名。
    pub title: String,
    /// 一句话说明（**落位在内容列顶部** ⇒ 不进导航列；**数据**，不写死）。
    pub blurb: String,
    /// 大类图标。
    pub icon: &'static IconData,
    /// 该大类下的分区（**顺序 = 生产顺序**，逐字沿用）。
    pub items: Vec<NavSection>,
}

/// 意图出口（**宿主注入** ⇒ 组件不自己改状态）。
///
/// 语义：入参是**分区 id**（[`SettingsFrameProps::on_select_section`]）或
/// **大类 id**（[`SettingsFrameProps::on_toggle_group`]）。
pub type NavIntent = Rc<dyn Fn(&str)>;

/// 框架面的 props。
///
/// ## 状态都是**调用方持有**的信号
///
/// ⇒ 组件是**纯函数**（同样的 props ⇒ 同样的产物）。
#[derive(Clone)]
pub struct SettingsFrameProps {
    /// 大类表（**顺序即渲染顺序**；与生产逐字同序）。
    pub groups: Vec<NavGroup>,
    /// 当前分区 id（**宿主持有**）。
    pub active_section: Signal<String>,
    /// 展开的大类集合（**宿主持有**；真值口径 = 集合里有它）。
    pub expanded_groups: Signal<Vec<String>>,
    /// 「应用进行中」（**宿主写，本面只读**）。
    pub live_apply_pending: Signal<bool>,
    /// 页头位（外部组件 ⇒ 宿主注入；`None` ⇒ 该位留空）。
    pub page_header: Option<VNode>,
    /// 错误横幅位（外部组件 ⇒ 宿主注入）。
    pub error_banner: Option<VNode>,
    /// 内容列（**宿主注入** ⇒ 本面不拥有任何内容区）。
    pub content: Option<VNode>,
    /// 「选中分区」意图出口。
    pub on_select_section: NavIntent,
    /// 「切换大类展开态」意图出口。
    pub on_toggle_group: NavIntent,
}

impl std::fmt::Debug for SettingsFrameProps {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("SettingsFrameProps")
            .field("groups", &self.groups.len())
            .field("active_section", &self.active_section.get())
            .field("expanded_groups", &self.expanded_groups.get())
            .field("live_apply_pending", &self.live_apply_pending.get())
            .field("page_header", &self.page_header.is_some())
            .field("error_banner", &self.error_banner.is_some())
            .field("content", &self.content.is_some())
            .finish_non_exhaustive()
    }
}

/// 框架面组件。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SettingsFrame;

impl UiComponent for SettingsFrame {
    type Props = SettingsFrameProps;
    type Output = VNode;

    fn render(props: &Self::Props) -> Self::Output {
        let active_section = props.active_section.get();
        let expanded_groups = props.expanded_groups.get();
        let live_apply_pending = props.live_apply_pending.get();

        // 当前分区所属的大类（导航高亮与内容列顶部的类别说明**同一来源**）。
        let active_group = props
            .groups
            .iter()
            .find(|group| group.items.iter().any(|item| item.id == active_section))
            .cloned();

        // 侧栏的每一行（**结构与空白都在这里定** ⇒ 渲染期不再有条件判断）。
        let rows: Vec<VNode> = props
            .groups
            .iter()
            .map(|group| {
                let open = expanded_groups.iter().any(|id| id == &group.id);
                let current = active_group
                    .as_ref()
                    .is_some_and(|active| active.id == group.id);
                render_group(
                    group,
                    open,
                    current,
                    &active_section,
                    &props.on_toggle_group,
                    &props.on_select_section,
                )
            })
            .collect();

        view! {
            section(class = "settings-view") {
                { optional(props.page_header.clone()) }
                div(class = "settings-layout") {
                    aside(class = "settings-subnav") {
                        for row in rows {
                            { row }
                        }
                    }
                    div(class = "settings-content") {
                        if live_apply_pending {
                            div(class = "apply-status") {
                                { Icon::render(&IconProps::new(&icons::ICON_ROTATE_CCW, 13).extra_class("spin")) }
                                span { "正在应用到运行时…" }
                            }
                        }
                        " "
                        { optional(props.error_banner.clone()) }
                        " "
                        if active_group.is_some() {
                            div(class = "category-context") {
                                p(class = "category-context-title") { active_group_title(&active_group) }
                                p(class = "category-context-blurb") { active_group_blurb(&active_group) }
                            }
                        }
                        " "
                        { optional(props.content.clone()) }
                    }
                }
            }
        }
    }
}

/// `Option<VNode>` ⇒ 虚拟节点（`None` ⇒ **空片段** ⇒ 不产出任何节点）。
///
/// ## 为什么需要它
///
/// `view!` 的子节点位只接受 `VNode` ⇒ 宿主注入的可选位（页头 / 错误横幅 /
/// 内容列）缺省时必须落成**空片段**，否则会多出一个空节点（DOM 逐字比对
/// 会出现差异）。
fn optional(node: Option<VNode>) -> VNode {
    node.unwrap_or_else(|| VNode::fragment(Vec::new()))
}

/// 渲染一个大类行（**含它的分区容器**）。
///
/// ## 空白口径（逐字照抄生产 DOM）
///
/// - 大类行**之间没有**空白文本节点（`{#each}` 边界裁掉了）；
/// - 大类按钮**之前没有**空白文本节点（`{@const}` 边界裁掉了）；
/// - 图标与标题之间、标题与展开指示之间**各有一个空格**（换行折叠成恰好
///   一个空格）；
/// - 大类按钮**之后有一个空格**（`{#if}` 边界**之前**的空白保留）。
fn render_group(
    group: &NavGroup,
    open: bool,
    current: bool,
    active_section: &str,
    on_toggle_group: &NavIntent,
    on_select_section: &NavIntent,
) -> VNode {
    // 事件闭包在**函数体内**捕获（不在宏的循环里捕获）⇒ 每次调用各自持有一份。
    let group_id = group.id.clone();
    let toggle = on_toggle_group.clone();
    let aria_expanded = if open { "true" } else { "false" };
    let pages: Vec<VNode> = group
        .items
        .iter()
        .map(|item| render_section(item, item.id == active_section, on_select_section))
        .collect();

    view! {
        div(class = "subnav-group", class:expanded = open) {
            button(
                class = "subnav-group-btn",
                class:active = current,
                aria-expanded = aria_expanded,
                onclick = move |_| { (toggle)(&group_id); }
            ) {
                { Icon::render(&IconProps::new(group.icon, 15)) }
                " "
                span(class = "subnav-group-name") { group.title.clone() }
                " "
                span(class = "subnav-chevron", class:open = open) {
                    { Icon::render(&IconProps::new(&icons::ICON_CHEVRON_DOWN, 13)) }
                }
            }
            " "
            if open {
                div(class = "subnav-group-pages") {
                    for page in pages {
                        { page }
                    }
                }
            }
        }
    }
}

/// 渲染一个分区行（**第二级**）。
fn render_section(item: &NavSection, active: bool, on_select_section: &NavIntent) -> VNode {
    let item_id = item.id.clone();
    let select = on_select_section.clone();

    view! {
        button(
            class = "subnav-btn",
            class:active = active,
            onclick = move |_| { (select)(&item_id); }
        ) {
            { Icon::render(&IconProps::new(item.icon, 14)) }
            " "
            span { item.label.clone() }
        }
    }
}

/// 当前大类名（**无当前大类 ⇒ 空串** ⇒ 该块整体不渲染）。
fn active_group_title(group: &Option<NavGroup>) -> String {
    group
        .as_ref()
        .map(|item| item.title.clone())
        .unwrap_or_default()
}

/// 当前大类说明（**无当前大类 ⇒ 空串**）。
fn active_group_blurb(group: &Option<NavGroup>) -> String {
    group
        .as_ref()
        .map(|item| item.blurb.clone())
        .unwrap_or_default()
}

/// 字段旗标的状态（**宿主持有** ⇒ 组件只渲染）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FieldFlagState {
    /// 无旗标（**不渲染任何节点**）。
    #[default]
    None,
    /// 改动尚未提交。
    Dirty,
    /// 已提交并生效。
    Ack,
}

/// 字段旗标的 props。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FieldFlagProps {
    /// 旗标状态。
    pub state: FieldFlagState,
}

impl FieldFlagProps {
    /// 建一组 props。
    pub fn new(state: FieldFlagState) -> Self {
        Self { state }
    }
}

/// 字段旗标组件（**只渲染，不拥有**状态）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FieldFlag;

impl UiComponent for FieldFlag {
    type Props = FieldFlagProps;
    type Output = VNode;

    fn render(props: &Self::Props) -> Self::Output {
        match props.state {
            FieldFlagState::None => VNode::fragment(Vec::new()),
            FieldFlagState::Dirty => view! {
                span(
                    class = "field-flag field-dirty",
                    title = "改动尚未提交：失焦或回车即提交"
                ) {
                    "未保存"
                }
            },
            FieldFlagState::Ack => view! {
                span(class = "field-flag field-ack", title = "已提交并生效") {
                    { Icon::render(&IconProps::new(&icons::ICON_CHECK, 12)) }
                }
            },
        }
    }
}

#[cfg(test)]
mod tests {
    //! 框架面的**行为**断言（渲染结果 ⇒ 结构 / class / 文本 / 事件），
    //! **不读生产源码**。

    use std::cell::RefCell;

    use super::*;
    use apeireth_ui_core::Runtime;
    use apeireth_ui_dom::{DomEvent, EventKind};

    /// 建一份最小的大类表（两个大类，第二个带两个分区）。
    fn sample_groups() -> Vec<NavGroup> {
        vec![
            NavGroup {
                id: String::from("account"),
                title: String::from("账户与服务商"),
                blurb: String::from("你是谁、它靠谁供给智能。"),
                icon: &icons::ICON_CPU,
                items: vec![NavSection {
                    id: String::from("user"),
                    label: String::from("用户中心"),
                    icon: &icons::ICON_CIRCLE_USER_ROUND,
                }],
            },
            NavGroup {
                id: String::from("appearance"),
                title: String::from("外观"),
                blurb: String::from("它住的房间怎么布置。"),
                icon: &icons::ICON_PALETTE,
                items: vec![
                    NavSection {
                        id: String::from("appearance"),
                        label: String::from("外观与主题"),
                        icon: &icons::ICON_PALETTE,
                    },
                    NavSection {
                        id: String::from("avatar"),
                        label: String::from("形象"),
                        icon: &icons::ICON_SMILE,
                    },
                ],
            },
        ]
    }

    /// 建一份 props（记录意图）。
    fn sample_props(
        active_section: &str,
        expanded: &[&str],
        selected: Rc<RefCell<Vec<String>>>,
        toggled: Rc<RefCell<Vec<String>>>,
    ) -> SettingsFrameProps {
        let runtime = Runtime::new();
        let select_target = selected.clone();
        let toggle_target = toggled.clone();
        SettingsFrameProps {
            groups: sample_groups(),
            active_section: runtime.signal(String::from(active_section)),
            expanded_groups: runtime.signal(expanded.iter().map(|id| String::from(*id)).collect()),
            live_apply_pending: runtime.signal(false),
            page_header: None,
            error_banner: None,
            content: None,
            on_select_section: Rc::new(move |id: &str| {
                select_target.borrow_mut().push(String::from(id));
            }),
            on_toggle_group: Rc::new(move |id: &str| {
                toggle_target.borrow_mut().push(String::from(id));
            }),
        }
    }

    /// 收集 class token（去重、保序）。
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

    /// 收集文本节点（逐个，**含空白**）。
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

    /// 找第一个带指定 class 的元素。
    fn find_by_class(node: &VNode, class: &str) -> Option<VNode> {
        find_all_by_class(node, class).into_iter().next()
    }

    /// 找**全部**带指定 class 的元素（**保序** ⇒ 顺序 = 渲染顺序）。
    fn find_all_by_class(node: &VNode, class: &str) -> Vec<VNode> {
        let mut found = Vec::new();
        if node.class_tokens().iter().any(|token| token == class) {
            found.push(node.clone());
        }
        for child in node.children() {
            found.extend(find_all_by_class(child, class));
        }
        found
    }

    /// 触发某个元素上的点击处理器（返回是否找到处理器）。
    fn click(node: &VNode) -> bool {
        let VNode::Element(element) = node else {
            return false;
        };
        let mut event = DomEvent::new(EventKind::Click, Vec::new());
        let mut fired = false;
        for (kind, handler) in &element.handlers {
            if *kind == EventKind::Click {
                handler.call(&mut event);
                fired = true;
            }
        }
        fired
    }

    /// 读全部大类按钮的 `aria-expanded`（**保序**）。
    fn aria_expanded_values(node: &VNode) -> Vec<String> {
        find_all_by_class(node, "subnav-group-btn")
            .iter()
            .map(|button| match button.attr_value("aria-expanded") {
                Some(apeireth_ui_dom::AttrValue::Text(text)) => text.clone(),
                other => panic!("aria-expanded 必须是文本值：{other:?}"),
            })
            .collect()
    }

    #[test]
    fn the_sidebar_renders_the_two_level_navigation() {
        let props = sample_props(
            "appearance",
            &["appearance"],
            Rc::new(RefCell::new(Vec::new())),
            Rc::new(RefCell::new(Vec::new())),
        );
        let node = SettingsFrame::render(&props);
        let mut found = Vec::new();
        classes(&node, &mut found);
        for class in [
            "settings-view",
            "settings-layout",
            "settings-subnav",
            "subnav-group",
            "subnav-group-btn",
            "subnav-group-name",
            "subnav-chevron",
        ] {
            assert!(
                found.iter().any(|token| token == class),
                "缺 class {class}：{found:?}"
            );
        }
    }

    #[test]
    fn the_group_pages_are_absent_until_the_group_is_expanded() {
        let props = sample_props(
            "appearance",
            &[],
            Rc::new(RefCell::new(Vec::new())),
            Rc::new(RefCell::new(Vec::new())),
        );
        let node = SettingsFrame::render(&props);
        let mut found = Vec::new();
        classes(&node, &mut found);
        assert!(
            !found.iter().any(|token| token == "subnav-group-pages"),
            "未展开 ⇒ 不渲染分区容器：{found:?}"
        );
        assert!(
            !found.iter().any(|token| token == "subnav-btn"),
            "未展开 ⇒ 不渲染分区行：{found:?}"
        );
    }

    #[test]
    fn the_group_pages_appear_when_the_group_is_expanded() {
        let props = sample_props(
            "appearance",
            &["appearance"],
            Rc::new(RefCell::new(Vec::new())),
            Rc::new(RefCell::new(Vec::new())),
        );
        let node = SettingsFrame::render(&props);
        let mut found = Vec::new();
        classes(&node, &mut found);
        assert!(
            found.iter().any(|token| token == "subnav-group-pages"),
            "展开 ⇒ 渲染分区容器：{found:?}"
        );
        assert!(
            found.iter().any(|token| token == "subnav-btn"),
            "展开 ⇒ 渲染分区行：{found:?}"
        );
    }

    #[test]
    fn the_expanded_class_is_a_class_branch_not_a_structural_one() {
        let props = sample_props(
            "appearance",
            &["appearance"],
            Rc::new(RefCell::new(Vec::new())),
            Rc::new(RefCell::new(Vec::new())),
        );
        let node = SettingsFrame::render(&props);
        let groups = find_all_by_class(&node, "subnav-group");
        assert_eq!(groups.len(), 2, "两个大类行：{}", groups.len());
        let expanded: Vec<&VNode> = groups
            .iter()
            .filter(|group| group.class_tokens().iter().any(|token| token == "expanded"))
            .collect();
        assert_eq!(expanded.len(), 1, "只有一个大类行展开");
        assert!(
            expanded[0]
                .class_tokens()
                .iter()
                .any(|token| token == "expanded"),
            "展开的大类行带 `expanded` class：{:?}",
            expanded[0].class_tokens()
        );
        let chevrons = find_all_by_class(&node, "subnav-chevron");
        assert_eq!(chevrons.len(), 2, "两个展开指示");
        let open: Vec<&VNode> = chevrons
            .iter()
            .filter(|chevron| chevron.class_tokens().iter().any(|token| token == "open"))
            .collect();
        assert_eq!(open.len(), 1, "只有一个展开指示带 `open`");
    }

    #[test]
    fn the_aria_expanded_attribute_is_always_present_with_a_string_value() {
        let props = sample_props(
            "appearance",
            &["appearance"],
            Rc::new(RefCell::new(Vec::new())),
            Rc::new(RefCell::new(Vec::new())),
        );
        let node = SettingsFrame::render(&props);
        let values = aria_expanded_values(&node);
        // 渲染顺序：先「账户与服务商」（收起 ⇒ false），后「外观」（展开 ⇒ true）。
        assert_eq!(values, ["false", "true"], "展开的大类 ⇒ true，其余 false");

        let collapsed = sample_props(
            "appearance",
            &[],
            Rc::new(RefCell::new(Vec::new())),
            Rc::new(RefCell::new(Vec::new())),
        );
        let node = SettingsFrame::render(&collapsed);
        let values = aria_expanded_values(&node);
        assert_eq!(values, ["false", "false"], "全收起 ⇒ 全是 false");
    }

    #[test]
    fn the_current_group_gets_the_active_class() {
        let props = sample_props(
            "appearance",
            &["appearance"],
            Rc::new(RefCell::new(Vec::new())),
            Rc::new(RefCell::new(Vec::new())),
        );
        let node = SettingsFrame::render(&props);
        let buttons = find_all_by_class(&node, "subnav-group-btn");
        let active: Vec<&VNode> = buttons
            .iter()
            .filter(|button| button.class_tokens().iter().any(|token| token == "active"))
            .collect();
        assert_eq!(active.len(), 1, "只有一个大类按钮带 `active`");
        assert!(
            active[0]
                .class_tokens()
                .iter()
                .any(|token| token == "active"),
            "当前分区所属的大类 ⇒ 按钮带 `active`：{:?}",
            active[0].class_tokens()
        );
    }

    #[test]
    fn the_group_blurb_lands_in_the_content_column_not_in_the_sidebar() {
        let props = sample_props(
            "appearance",
            &["appearance"],
            Rc::new(RefCell::new(Vec::new())),
            Rc::new(RefCell::new(Vec::new())),
        );
        let node = SettingsFrame::render(&props);
        let mut found = Vec::new();
        classes(&node, &mut found);
        assert!(
            found.iter().any(|token| token == "category-context"),
            "当前大类 ⇒ 内容列渲染大类说明条：{found:?}"
        );
        let mut texts = Vec::new();
        text_nodes(&node, &mut texts);
        assert!(
            texts.iter().any(|text| text == "它住的房间怎么布置。"),
            "大类说明（数据）逐字渲染：{texts:?}"
        );
        let sidebar = find_by_class(&node, "settings-subnav").expect("侧栏存在");
        let mut sidebar_texts = Vec::new();
        text_nodes(&sidebar, &mut sidebar_texts);
        assert!(
            !sidebar_texts
                .iter()
                .any(|text| text == "它住的房间怎么布置。"),
            "大类说明**不进导航列**：{sidebar_texts:?}"
        );
    }

    #[test]
    fn the_click_emits_an_intent_and_does_not_change_the_state() {
        let selected = Rc::new(RefCell::new(Vec::new()));
        let toggled = Rc::new(RefCell::new(Vec::new()));
        let props = sample_props(
            "appearance",
            &["appearance"],
            selected.clone(),
            toggled.clone(),
        );
        let before = props.active_section.get();
        let node = SettingsFrame::render(&props);

        // 分区行 ⇒ 发「选中分区」意图。
        let section = find_by_class(&node, "subnav-btn").expect("分区行存在");
        assert!(click(&section), "分区行带点击处理器");
        assert_eq!(
            selected.borrow().as_slice(),
            ["appearance"],
            "意图带分区 id"
        );

        // 大类按钮 ⇒ 发「切换大类」意图（点**当前大类**那一行 ⇒ 意图带它的 id）。
        let buttons = find_all_by_class(&node, "subnav-group-btn");
        assert_eq!(buttons.len(), 2, "两个大类按钮");
        let current = buttons
            .iter()
            .find(|button| button.class_tokens().iter().any(|token| token == "active"))
            .expect("当前大类按钮存在");
        assert!(click(current), "大类按钮带点击处理器");
        assert_eq!(toggled.borrow().as_slice(), ["appearance"], "意图带大类 id");

        // **组件不改状态**（状态归宿主）。
        assert_eq!(props.active_section.get(), before, "点击不改宿主的状态");
    }

    #[test]
    fn the_blank_text_nodes_are_reproduced_one_space_each() {
        let props = sample_props(
            "appearance",
            &["appearance"],
            Rc::new(RefCell::new(Vec::new())),
            Rc::new(RefCell::new(Vec::new())),
        );
        let node = SettingsFrame::render(&props);
        let sidebar = find_by_class(&node, "settings-subnav").expect("侧栏存在");
        let mut texts = Vec::new();
        text_nodes(&sidebar, &mut texts);
        let spaces = texts.iter().filter(|text| *text == " ").count();
        // 收起的���类（1 个）：图标↔标题 1 + 标题↔展开指示 1 + 按钮之后 1 = 3；
        // 展开的大类（1 个）：同 3；它下面的 2 个分区行：图标↔标题各 1 = 2；
        // ⇒ 合计 8。
        assert_eq!(spaces, 8, "空白文本节点逐个复现：{texts:?}");
        assert!(
            texts.iter().any(|text| text == "外观与主题"),
            "分区标签逐字：{texts:?}"
        );
    }

    #[test]
    fn the_last_section_map_is_not_part_of_the_rendered_tree() {
        // 生产里 `lastSectionByGroup` 是**普通 Map**（非响应式）且**只被
        // `toggleGroup` 读** ⇒ 它不进渲染 ⇒ 本组件的 props 里也没有它。
        let props = sample_props(
            "appearance",
            &["appearance"],
            Rc::new(RefCell::new(Vec::new())),
            Rc::new(RefCell::new(Vec::new())),
        );
        let node = SettingsFrame::render(&props);
        let mut texts = Vec::new();
        text_nodes(&node, &mut texts);
        assert!(
            !texts.iter().any(|text| text.contains("lastSection")),
            "渲染结果里没有 `lastSectionByGroup` 的痕迹：{texts:?}"
        );
    }

    #[test]
    fn the_field_flag_renders_the_two_branches() {
        let dirty = FieldFlag::render(&FieldFlagProps::new(FieldFlagState::Dirty));
        let mut found = Vec::new();
        classes(&dirty, &mut found);
        assert!(
            found.iter().any(|token| token == "field-dirty"),
            "{found:?}"
        );
        let mut texts = Vec::new();
        text_nodes(&dirty, &mut texts);
        assert!(texts.iter().any(|text| text == "未保存"), "{texts:?}");

        let ack = FieldFlag::render(&FieldFlagProps::new(FieldFlagState::Ack));
        let mut found = Vec::new();
        classes(&ack, &mut found);
        assert!(found.iter().any(|token| token == "field-ack"), "{found:?}");

        let none = FieldFlag::render(&FieldFlagProps::new(FieldFlagState::None));
        let mut found = Vec::new();
        classes(&none, &mut found);
        assert!(found.is_empty(), "无旗标 ⇒ 不渲染任何节点：{found:?}");
    }

    #[test]
    fn the_icons_carry_the_real_class_suffixes() {
        // 生产 import 名 ⇒ 真实 class 后缀（抽取器跟随上游包的再导出找到真实文件）。
        let cases: [(&IconData, &str); 6] = [
            (&icons::ICON_TRASH_2, "lucide-trash-2"),
            (&icons::ICON_CIRCLE_USER_ROUND, "lucide-circle-user-round"),
            (&icons::ICON_SHIELD_CHECK, "lucide-shield-check"),
            (&icons::ICON_ROTATE_CCW, "lucide-rotate-ccw"),
            (&icons::ICON_CHEVRON_DOWN, "lucide-chevron-down"),
            (&icons::ICON_ARCHIVE, "lucide-archive"),
        ];
        for (data, suffix) in cases {
            assert!(
                data.class.ends_with(suffix),
                "图标 class 后缀：{} vs {suffix}",
                data.class
            );
        }
    }
}
