//! 设置页三页 · `character` 子面（**人格与记忆**）的组件模块。
//!
//! ## 本面是什么
//!
//! 生产 `SettingsView.svelte` 的**人格与记忆**四个分区（markup 2364–2862，
//! **496 行** = 设置页里最大的子面）：
//!
//! | 分区 | 内容 | 生产行区间 |
//! |---|---|---|
//! | `personality` | 人设卡列表（名称 / 固定模型 / 人设文本 / 设为当前 / 删除） | 2364–2454 |
//! | `impression` | 当前伙伴 + 立即总结 / 自动总结 / 清空 + 印象全文 | 2456–2520 |
//! | `cognition` | 摘要条 + 推荐配置 + **三张能力开关卡**（5 + 6 + 2 行） | 2522–2662 |
//! | `disposition` | 体验参数（4 个滑杆）+ 预设芯片 + 自学习开关 + 学习日志 | 2664–2862 |
//!
//! ## 本面是**全批技术风险最高点**（迁移序 M6 的原文）
//!
//! ⇒ 「能力开关卡」组件在这里**成型** ⇒ 下一面（能力与治理）**复用**它。
//! ⇒ 四个高危点全部落在本面 ⇒ 逐条钉死：
//!
//! ### ① `invert` **反向旋钮**（判反 = 静默错误）
//!
//! 生产的 `typedRecall` 判的是 **env 变量名**（`APEIRETH_DISABLE_TYPED_RECALL`）
//! ⇒ 「旋钮开」对应 **env 被禁用** ⇒ 判定口径**逐字**是：
//!
//! ```text
//! isCapOn(def) = def.invert ? value === false : value === true
//! ```
//!
//! ⚠️ 若按普通布尔处理 ⇒ 画面**完全正常**但**语义相反** ⇒ **静默错误**。
//! ⇒ 本模块把它做成 [`is_cap_on`]（**唯一**判定入口）并有单测钉住。
//!
//! ### ② `requires` **依赖行**
//!
//! `fileWriteAutoPass` 依赖 `fileWrite` ⇒ 父行状态驱动子行的**禁用态**
//! ⇒ 判定口径：`capDisabled(def) = def.requires !== undefined &&
//! capabilities[def.requires] !== true` ⇒ 见 [`cap_disabled`]。
//!
//! ### ③ **嵌套行插在父行旁**（同层，不是子层）
//!
//! 生产的 `{#if def.key === 'morphologyRecall' && …}` 把检索温度滑杆行插在
//! **父行的兄弟位置** ⇒ **一项产出两个节点** ⇒ 列表渲染必须支持这种形态
//! ⇒ 见 [`render_capability_rows`]（每个定义可带一个 [`KnobSpec`] 嵌套行）。
//!
//! ### ④ 「**一个组件 + 一张表**」
//!
//! 生产的 8 个 `CapDef` 注册表（74 行**数据**）撑起 944 行 markup ⇒
//! **不要**按 8 段模板各写一遍 ⇒ 本模块的 [`render_capability_card`] 吃
//! **一张定义表** ⇒ 表由**宿主**喂（固定数据 ⇒ `fixtures/capabilities.json`）
//! ⇒ **组件里没有一行能力定义**。
//!
//! ## 逐字照抄的**真值口径**（全部来自生产 DOM 的**浏览器实测 dump**）
//!
//! | 事项 | 口径 |
//! |---|---|
//! | 空白 | 元素之间的换行折叠成**恰好一个空格**；`{#if}` / `{#each}` 边界两侧的空白**被裁掉** ⇒ 人设卡之间**没有**空白文本节点 |
//! | 段落内部换行 | **原样保留**（文本节点逐字）⇒ 标题 / 说明里的 `\n` + 缩进是**数据** |
//! | 能力行 | `<button role="switch" class="cap-row">` + `aria-checked` + `aria-label` + 条件 class `dim` / `pending` |
//! | 静态 `disabled` | DOM 上是 **`disabled=""`**（**不是**属性缺失，**也不是** `disabled="true"`）⇒ 用 `.attr("disabled", "")` |
//! | 开关的「开」 | `aria-checked="true"` / `"false"`（**文本**）⇒ 不是布尔属性 |
//! | 原生控件 | `input[type=range]` **必须**是原生元素 ⇒ **不写** `value` 属性（生产用 **property**）⇒ 由宿主在渲染后回写 |
//! | 徽标 | `StatusBadge` 是**外部组件** ⇒ 本面只产出它的**结构与 class**（`.status-badge green small`）⇒ 样式由外部组件的样式表提供 |
//!
//! ## 状态归属（**引擎不迁**的部分）
//!
//! 人设增删、印象落盘、能力开关的 apply、预设、学习日志的读与撤销 ⇒ 全部是
//! **宿主侧的事**（HTTP / 本地存储 / 钥匙串）⇒ 本组件**只承接展示面 +
//! 受控输入** ⇒ 点击 ⇒ **发意图**（[`CharacterPanelsProps`] 的各个出口）。
//!
//! ## 图标数据
//!
//! 本面用到的图标几何数据是**构建期抽取**的**生成物**（上游图标包 ⇒ 真实
//! `.svelte` ⇒ `iconNode`）⇒ 见 [`icons`] 模块。class 后缀取**真实文件名**
//! （`Layers3` ⇒ `lucide-layers`；`CheckCircle2` ⇒ `lucide-circle-check`）。

use std::collections::BTreeMap;
use std::rc::Rc;

use crate::icon::{Icon, IconData, IconProps};
use crate::settings_frame::{FieldFlag, FieldFlagProps, FieldFlagState};
use apeireth_ui_core::{Signal, UiComponent};
use apeireth_ui_dom::{DomEvent, EventKind, ElementHandler, VNode};
use apeireth_ui_macro::view;

/// 本面用到的图标数据（**自动生成** —— 勿手改）。
///
/// 生成器：`crates-ui/apeireth-ui-demo/harness/scripts/extract-assets.mjs`
/// （**面 manifest 模式** ⇒ 清单：`harness/settings/web/engine/character/face.manifest.json`）
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
/// MERCHANTABILITY AND FITNESS. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT
/// HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN
/// ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN
/// CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
pub mod icons {
    use crate::icon::{IconData, IconNode};

/// 图标 `layers`。
#[rustfmt::skip]
pub const ICON_LAYERS: IconData = IconData {
    name: "layers",
    class: "lucide-icon lucide lucide-layers",
    nodes: &[
        IconNode { tag: "path", attrs: &[("d", "M12.83 2.18a2 2 0 0 0-1.66 0L2.6 6.08a1 1 0 0 0 0 1.83l8.58 3.91a2 2 0 0 0 1.66 0l8.58-3.9a1 1 0 0 0 0-1.83z")] },
        IconNode { tag: "path", attrs: &[("d", "M2 12a1 1 0 0 0 .58.91l8.6 3.91a2 2 0 0 0 1.65 0l8.58-3.9A1 1 0 0 0 22 12")] },
        IconNode { tag: "path", attrs: &[("d", "M2 17a1 1 0 0 0 .58.91l8.6 3.91a2 2 0 0 0 1.65 0l8.58-3.9A1 1 0 0 0 22 17")] },
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

/// 图标 `plus`。
#[rustfmt::skip]
pub const ICON_PLUS: IconData = IconData {
    name: "plus",
    class: "lucide-icon lucide lucide-plus",
    nodes: &[
        IconNode { tag: "path", attrs: &[("d", "M5 12h14")] },
        IconNode { tag: "path", attrs: &[("d", "M12 5v14")] },
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

/// 图标 `heart-handshake`。
#[rustfmt::skip]
pub const ICON_HEART_HANDSHAKE: IconData = IconData {
    name: "heart-handshake",
    class: "lucide-icon lucide lucide-heart-handshake",
    nodes: &[
        IconNode { tag: "path", attrs: &[("d", "M19 14c1.49-1.46 3-3.21 3-5.5A5.5 5.5 0 0 0 16.5 3c-1.76 0-3 .5-4.5 2-1.5-1.5-2.74-2-4.5-2A5.5 5.5 0 0 0 2 8.5c0 2.3 1.5 4.05 3 5.5l7 7Z")] },
        IconNode { tag: "path", attrs: &[("d", "M12 5 9.04 7.96a2.17 2.17 0 0 0 0 3.08c.82.82 2.13.85 3 .07l2.07-1.9a2.82 2.82 0 0 1 3.79 0l2.96 2.66")] },
        IconNode { tag: "path", attrs: &[("d", "m18 15-2-2")] },
        IconNode { tag: "path", attrs: &[("d", "m15 18-2-2")] },
    ],
};

/// 图标 `git-branch`。
#[rustfmt::skip]
pub const ICON_GIT_BRANCH: IconData = IconData {
    name: "git-branch",
    class: "lucide-icon lucide lucide-git-branch",
    nodes: &[
        IconNode { tag: "line", attrs: &[("x1", "6"), ("x2", "6"), ("y1", "3"), ("y2", "15")] },
        IconNode { tag: "circle", attrs: &[("cx", "18"), ("cy", "6"), ("r", "3")] },
        IconNode { tag: "circle", attrs: &[("cx", "6"), ("cy", "18"), ("r", "3")] },
        IconNode { tag: "path", attrs: &[("d", "M18 9a9 9 0 0 1-9 9")] },
    ],
};

/// 图标 `sliders-horizontal`。
#[rustfmt::skip]
pub const ICON_SLIDERS_HORIZONTAL: IconData = IconData {
    name: "sliders-horizontal",
    class: "lucide-icon lucide lucide-sliders-horizontal",
    nodes: &[
        IconNode { tag: "line", attrs: &[("x1", "21"), ("x2", "14"), ("y1", "4"), ("y2", "4")] },
        IconNode { tag: "line", attrs: &[("x1", "10"), ("x2", "3"), ("y1", "4"), ("y2", "4")] },
        IconNode { tag: "line", attrs: &[("x1", "21"), ("x2", "12"), ("y1", "12"), ("y2", "12")] },
        IconNode { tag: "line", attrs: &[("x1", "8"), ("x2", "3"), ("y1", "12"), ("y2", "12")] },
        IconNode { tag: "line", attrs: &[("x1", "21"), ("x2", "16"), ("y1", "20"), ("y2", "20")] },
        IconNode { tag: "line", attrs: &[("x1", "12"), ("x2", "3"), ("y1", "20"), ("y2", "20")] },
        IconNode { tag: "line", attrs: &[("x1", "14"), ("x2", "14"), ("y1", "2"), ("y2", "6")] },
        IconNode { tag: "line", attrs: &[("x1", "8"), ("x2", "8"), ("y1", "10"), ("y2", "14")] },
        IconNode { tag: "line", attrs: &[("x1", "16"), ("x2", "16"), ("y1", "18"), ("y2", "22")] },
    ],
};

/// 图标 `refresh-ccw`。
#[rustfmt::skip]
pub const ICON_REFRESH_CCW: IconData = IconData {
    name: "refresh-ccw",
    class: "lucide-icon lucide lucide-refresh-ccw",
    nodes: &[
        IconNode { tag: "path", attrs: &[("d", "M21 12a9 9 0 0 0-9-9 9.75 9.75 0 0 0-6.74 2.74L3 8")] },
        IconNode { tag: "path", attrs: &[("d", "M3 3v5h5")] },
        IconNode { tag: "path", attrs: &[("d", "M3 12a9 9 0 0 0 9 9 9.75 9.75 0 0 0 6.74-2.74L21 16")] },
        IconNode { tag: "path", attrs: &[("d", "M16 16h5v5")] },
    ],
};

/// 图标 `radar`。
#[rustfmt::skip]
pub const ICON_RADAR: IconData = IconData {
    name: "radar",
    class: "lucide-icon lucide lucide-radar",
    nodes: &[
        IconNode { tag: "path", attrs: &[("d", "M19.07 4.93A10 10 0 0 0 6.99 3.34")] },
        IconNode { tag: "path", attrs: &[("d", "M4 6h.01")] },
        IconNode { tag: "path", attrs: &[("d", "M2.29 9.62A10 10 0 1 0 21.31 8.35")] },
        IconNode { tag: "path", attrs: &[("d", "M16.24 7.76A6 6 0 1 0 8.23 16.67")] },
        IconNode { tag: "path", attrs: &[("d", "M12 18h.01")] },
        IconNode { tag: "path", attrs: &[("d", "M17.99 11.66A6 6 0 0 1 15.77 16.67")] },
        IconNode { tag: "circle", attrs: &[("cx", "12"), ("cy", "12"), ("r", "2")] },
        IconNode { tag: "path", attrs: &[("d", "m13.41 10.59 5.66-5.66")] },
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

/// 图标 `message-square-plus`。
#[rustfmt::skip]
pub const ICON_MESSAGE_SQUARE_PLUS: IconData = IconData {
    name: "message-square-plus",
    class: "lucide-icon lucide lucide-message-square-plus",
    nodes: &[
        IconNode { tag: "path", attrs: &[("d", "M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z")] },
        IconNode { tag: "path", attrs: &[("d", "M12 7v6")] },
        IconNode { tag: "path", attrs: &[("d", "M9 10h6")] },
    ],
};

/// 图标 `network`。
#[rustfmt::skip]
pub const ICON_NETWORK: IconData = IconData {
    name: "network",
    class: "lucide-icon lucide lucide-network",
    nodes: &[
        IconNode { tag: "rect", attrs: &[("x", "16"), ("y", "16"), ("width", "6"), ("height", "6"), ("rx", "1")] },
        IconNode { tag: "rect", attrs: &[("x", "2"), ("y", "16"), ("width", "6"), ("height", "6"), ("rx", "1")] },
        IconNode { tag: "rect", attrs: &[("x", "9"), ("y", "2"), ("width", "6"), ("height", "6"), ("rx", "1")] },
        IconNode { tag: "path", attrs: &[("d", "M5 16v-3a1 1 0 0 1 1-1h12a1 1 0 0 1 1 1v3")] },
        IconNode { tag: "path", attrs: &[("d", "M12 12V8")] },
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

/// 图标 `book-open`。
#[rustfmt::skip]
pub const ICON_BOOK_OPEN: IconData = IconData {
    name: "book-open",
    class: "lucide-icon lucide lucide-book-open",
    nodes: &[
        IconNode { tag: "path", attrs: &[("d", "M12 7v14")] },
        IconNode { tag: "path", attrs: &[("d", "M3 18a1 1 0 0 1-1-1V4a1 1 0 0 1 1-1h5a4 4 0 0 1 4 4 4 4 0 0 1 4-4h5a1 1 0 0 1 1 1v13a1 1 0 0 1-1 1h-6a3 3 0 0 0-3 3 3 3 0 0 0-3-3z")] },
    ],
};

/// 图标 `dumbbell`。
#[rustfmt::skip]
pub const ICON_DUMBBELL: IconData = IconData {
    name: "dumbbell",
    class: "lucide-icon lucide lucide-dumbbell",
    nodes: &[
        IconNode { tag: "path", attrs: &[("d", "M14.4 14.4 9.6 9.6")] },
        IconNode { tag: "path", attrs: &[("d", "M18.657 21.485a2 2 0 1 1-2.829-2.828l-1.767 1.768a2 2 0 1 1-2.829-2.829l6.364-6.364a2 2 0 1 1 2.829 2.829l-1.768 1.767a2 2 0 1 1 2.828 2.829z")] },
        IconNode { tag: "path", attrs: &[("d", "m21.5 21.5-1.4-1.4")] },
        IconNode { tag: "path", attrs: &[("d", "M3.9 3.9 2.5 2.5")] },
        IconNode { tag: "path", attrs: &[("d", "M6.404 12.768a2 2 0 1 1-2.829-2.829l1.768-1.767a2 2 0 1 1-2.828-2.829l2.828-2.828a2 2 0 1 1 2.829 2.828l1.767-1.768a2 2 0 1 1 2.829 2.829z")] },
    ],
};

/// 图标 `landmark`。
#[rustfmt::skip]
pub const ICON_LANDMARK: IconData = IconData {
    name: "landmark",
    class: "lucide-icon lucide lucide-landmark",
    nodes: &[
        IconNode { tag: "line", attrs: &[("x1", "3"), ("x2", "21"), ("y1", "22"), ("y2", "22")] },
        IconNode { tag: "line", attrs: &[("x1", "6"), ("x2", "6"), ("y1", "18"), ("y2", "11")] },
        IconNode { tag: "line", attrs: &[("x1", "10"), ("x2", "10"), ("y1", "18"), ("y2", "11")] },
        IconNode { tag: "line", attrs: &[("x1", "14"), ("x2", "14"), ("y1", "18"), ("y2", "11")] },
        IconNode { tag: "line", attrs: &[("x1", "18"), ("x2", "18"), ("y1", "18"), ("y2", "11")] },
        IconNode { tag: "polygon", attrs: &[("points", "12 2 20 7 4 7")] },
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

    /// 按**生产的 import 名**取图标数据（**表驱动** ⇒ 组件里没有逐行硬编码）。
    ///
    /// ## 为什么需要这张映射
    ///
    /// 固定数据（`fixtures/capabilities.json`）里存的是**生产的 import 名**
    /// （`MessageSquarePlus` / `Layers3` / `CheckCircle2` / …）⇒ 宿主按名取
    /// 图标 ⇒ 映射表是**数据**，不是逐行代码。
    ///
    /// ⚠️ **已知改写**（生产 import 名 ⇒ 真实文件 ⇒ 真实 class）：
    ///
    /// | 生产 import 名 | 真实文件 | 渲染出的 class |
    /// |---|---|---|
    /// | `Layers3` | `layers.svelte` | `lucide-layers` |
    /// | `CheckCircle2` | `circle-check.svelte` | `lucide-circle-check` |
    /// | `AlertTriangle` | `triangle-alert.svelte` | `lucide-triangle-alert` |
    /// | `XCircle` | `circle-x.svelte` | `lucide-circle-x` |
    /// | `FileCheck2` | `file-check-2.svelte` | `lucide-file-check-2` |
    ///
    /// 未知名字 ⇒ `None`（**显式失败** ⇒ 宿主据此报错，不静默画空图标）。
    pub fn icon_by_import_name(name: &str) -> Option<&'static IconData> {
        match name {
            "Archive" => Some(&ICON_ARCHIVE),
            "BookOpen" => Some(&ICON_BOOK_OPEN),
            "Brain" => Some(&ICON_BRAIN),
            "CheckCircle2" => Some(&ICON_CIRCLE_CHECK),
            "Dumbbell" => Some(&ICON_DUMBBELL),
            "FileCheck2" => Some(&ICON_FILE_CHECK_2),
            "GitBranch" => Some(&ICON_GIT_BRANCH),
            "HeartHandshake" => Some(&ICON_HEART_HANDSHAKE),
            "Info" => Some(&ICON_INFO),
            "Landmark" => Some(&ICON_LANDMARK),
            "Layers3" => Some(&ICON_LAYERS),
            "MessageSquarePlus" => Some(&ICON_MESSAGE_SQUARE_PLUS),
            "Network" => Some(&ICON_NETWORK),
            "Plus" => Some(&ICON_PLUS),
            "Radar" => Some(&ICON_RADAR),
            "RefreshCcw" => Some(&ICON_REFRESH_CCW),
            "RotateCcw" => Some(&ICON_ROTATE_CCW),
            "SlidersHorizontal" => Some(&ICON_SLIDERS_HORIZONTAL),
            "Sparkles" => Some(&ICON_SPARKLES),
            "Timer" => Some(&ICON_TIMER),
            "Trash2" => Some(&ICON_TRASH_2),
            "Users" => Some(&ICON_USERS),
            _ => None,
        }
    }
}

// ── 文案常量（**生产 markup 逐字**） ────────────────────────────────────────
//
// ## 为什么文案在组件里
//
// 本面的文案全部是**生产 markup 的原文**（标题 / 说明 / 提示 / 按钮标签）。
// 它们**不是**可配置数据（没有第二份来源）⇒ 放成常量（**逐字**）⇒
// 「文案漂移」这件事在**代码评审**里就能看见，而不是等到像素对照才发现。
//
// ⚠️ **空白逐字**：生产模板里段落**内部**的换行 + 缩进是**原样保留**的
// （文本节点逐字）⇒ 这里的常量里带着真实的 `\n` 与缩进 ⇒ **不要**用别的
// 工具"整理"它们。

/// 人设区的标题。
pub const PERSONALITY_TITLE: &str = "伙伴人设与行为 (Persona)";

/// 人设区的说明（**含生产模板里的换行与缩进**）。
pub const PERSONALITY_DESC: &str = "数据驱动的多 Agent 身份：可随时增删改。文本改动失焦或回车整组提交（避免半截配置）；\n            「设为当前 / 新增伙伴」即点即生效，删除需二次确认（人设作为 system 消息注入每次对话），无需重编译。";

/// 人设区底部提示条。
pub const PERSONALITY_NOTICE: &str = "人设由客户端作为 system 消息注入每次请求；失焦或回车提交后立即生效，重启应用仍保留（不含任何密钥）。";

/// 人设名标签（**尾随空格**：字段旗标跟在后面）。
pub const PERSONA_NAME_LABEL: &str = "名称 ";

/// 人设固定模型标签（**尾随空格**）。
pub const PERSONA_MODEL_LABEL: &str = "固定模型（可选，留空跟随全局设置） ";

/// 人设正文标签（**尾随空格**）。
pub const PERSONA_TEXT_LABEL: &str = "人设文本（system 消息；留空 = 该伙伴不注入人设） ";

/// 人设名的占位文本。
pub const PERSONA_NAME_PLACEHOLDER: &str = "伙伴名称";

/// 人设正文的占位文本。
pub const PERSONA_TEXT_PLACEHOLDER: &str = "你是「阿佩瑞斯」——Apeireth 基地的主管…";

/// 「设为当前」按钮的**当前**态文案。
pub const PERSONA_CURRENT_LABEL: &str = "✓ 当前伙伴";

/// 「设为当前」按钮的**未选中**态文案。
pub const PERSONA_SELECT_LABEL: &str = "设为当前";

/// 「设为当前」按钮的悬浮提示。
pub const PERSONA_SELECT_TITLE: &str = "设为当前伙伴";

/// 「删除伙伴」按钮的悬浮提示。
pub const PERSONA_REMOVE_TITLE: &str = "删除该伙伴（需二次确认）";

/// 「新增伙伴」按钮文案。
pub const PERSONA_ADD_LABEL: &str = "新增伙伴";

/// 徽标文案（「实时生效」徽标；**外部组件** ⇒ 样式另带一份）。
pub const BADGE_LIVE_LABEL: &str = "实时生效";

/// 徽标的 class（生产 `StatusBadge` 的 class 组合 ⇒ **逐字**）。
pub const BADGE_LIVE_CLASS: &str = "status-badge green small";

/// 印象区的标题。
pub const IMPRESSION_TITLE: &str = "用户印象";

/// 印象区的说明（**含换行与缩进**）。
pub const IMPRESSION_DESC: &str = "它眼中的你：每个伙伴在对话里慢慢攒出的用户印象。双重身份——给你看的阅读项（可读、可改、可清空），\n            也是它的自我参考项（随每次对话注入它的 system 前缀，让它带着既有印象继续相处）。";

/// 印象区提示条的前缀（**紧跟 `<strong>` 之前**）。
pub const IMPRESSION_NOTICE_PREFIX: &str = "当前伙伴：";

/// 印象区提示条的后缀（**紧跟 `<strong>` 之后**）。
pub const IMPRESSION_NOTICE_SUFFIX: &str = "——印象按伙伴分档，换伙伴即换它自己的档案。";

/// 没有当前伙伴时的占位名。
pub const IMPRESSION_NO_PERSONA: &str = "无人设（默认）";

/// 「立即总结」按钮文案。
pub const IMPRESSION_SUMMARIZE_LABEL: &str = "立即总结";

/// 「立即总结」按钮的**进行中**文案。
pub const IMPRESSION_SUMMARIZING_LABEL: &str = "正在总结…";

/// 「自动总结」按钮的前缀。
pub const IMPRESSION_AUTO_PREFIX: &str = "自动总结：";

/// 「自动总结」按钮的「开」。
pub const IMPRESSION_AUTO_ON: &str = "开";

/// 「自动总结」按钮的「关」。
pub const IMPRESSION_AUTO_OFF: &str = "关";

/// 「自动总结」按钮的悬浮提示。
pub const IMPRESSION_AUTO_TITLE: &str = "开启后它会在对话后自动更新印象（每攒够几条新消息追一次，成本可控）";

/// 「清空印象」按钮文案。
pub const IMPRESSION_CLEAR_LABEL: &str = "清空印象";

/// 「清空印象」按钮的悬浮提示。
pub const IMPRESSION_CLEAR_TITLE: &str = "清空印象正文（自动总结开关保留）";

/// 印象正文的标签（**尾随空格**）。
pub const IMPRESSION_TEXT_LABEL: &str = "印象全文（可直接改写，失焦或回车即存） ";

/// 印象正文的占位文本。
pub const IMPRESSION_TEXT_PLACEHOLDER: &str =
    "还没有印象。聊过几句后点「立即总结」，或等它在对话里自动攒出对你的印象。";

/// 印象区的「没有记录」分支。
pub const IMPRESSION_NONE_TEXT: &str = "还没有印象记录。";

/// 印象区的「最近更新」前缀。
pub const IMPRESSION_UPDATED_PREFIX: &str = "最近更新：";

/// 印象区的来源文案（自动总结）。
pub const IMPRESSION_SOURCE_AUTO: &str = "对话中自动总结";

/// 印象区的来源文案（手动）。
pub const IMPRESSION_SOURCE_MANUAL: &str = "手动总结或手写";

/// 印象区提示的尾部（**首字符是空格**：紧跟前面的分支之后**）。
pub const IMPRESSION_HINT_TAIL: &str = " 只落这台机器的应用存储（与会话本地账同一口径），不上传；总结只带对话正文，不带密钥与配置。";

/// 认知区的标题。
pub const COGNITION_TITLE: &str = "记忆与认知";

/// 认知区的说明（**含换行与缩进**）。
pub const COGNITION_DESC: &str = "记忆流运行态 + W2/W3 收官批落地的认知增强件。记忆核心族（记忆注入/前瞻召回/偏好学习）\n            默认开启、可随时关；其余默认关闭、逐项显式开启。拨动即注入侧车环境并生效（配置没变不会重启网关）。";

/// 认知区摘要条的前缀。
pub const COGNITION_SUMMARY_PREFIX: &str = "本页已启用 ";

/// 认知区摘要条的后缀（**总数是动态的** ⇒ 由组件拼）。
pub const COGNITION_SUMMARY_SUFFIX: &str = " 项认知能力";

/// 「应用推荐配置」按钮文案。
pub const COGNITION_PRESET_LABEL: &str = "应用推荐配置";

/// 「应用推荐配置」的旁注。
pub const COGNITION_PRESET_HINT: &str = "一键开启记忆核心族 + 记忆固化/反思沉淀/器官链，不含 shell/fetch；点击即生效（失败回填 + 横幅）。";

/// 认知区提示条的第一段（**尾随空格**：紧跟 `<strong>` 之前**）。
pub const COGNITION_NOTICE_HEAD: &str = "将开启 6 项：前瞻召回、偏好学习、记忆注入、记忆固化、反思沉淀、器官链。 ";

/// 认知区提示条里的加粗段。
pub const COGNITION_NOTICE_STRONG: &str = "不包含";

/// 认知区提示条的尾段（**首字符是空格**）。
pub const COGNITION_NOTICE_TAIL: &str = " Shell / 网络读取等危险工具（它们保持现状、仍需逐项开启与审批），其余开关不动。";

/// 记忆流卡的标题（**首字符是空格**：紧跟图标之后**）。
pub const MEMORY_FLOW_CARD_TITLE: &str = " 记忆流";

/// 认知增强卡的标题（**首字符是空格**）。
pub const COGNITION_CARD_TITLE: &str = " 认知增强 · W2 接线批";

/// 社区与账本卡的标题（**首字符是空格**）。
pub const COMMUNITY_CARD_TITLE: &str = " 社区与账本 · W3 落地批";

/// 「默认能力」徽标的文案（**首字符是空格**）。
pub const DEFAULT_BADGE_LABEL: &str = " 默认能力 · 显式触发";

/// 「默认能力」的标题。
pub const DEFAULT_TITLE: &str = "做梦与反思 (Dream & Reflection)";

/// 「默认能力」的说明（**首段**；后面夹一个 `<code>`）。
pub const DEFAULT_TEXT_HEAD: &str = "无需开关、永远可用：命令行 ";

/// 「默认能力」的命令名（`<code>` 里）。
pub const DEFAULT_TEXT_CODE: &str = "apeireth dream";

/// 「默认能力」的说明尾段（**含换行与缩进**）。
pub const DEFAULT_TEXT_TAIL: &str = " 即授权一次做梦周期，\n              苏醒后把提炼结果写日记（source=dream），日记失败不伪装成功。";

/// 认知区的脚注（**含换行与缩进**）。
pub const COGNITION_FOOTNOTE: &str = "fail-closed 语义：以上旋钮关闭（默认）时不注入任何环境变量，后端能力完全不存在；\n            开启只注入 \"1\"。唯一的例外是沙箱（后端默认开），见「工具与安全」。";

/// 性情区的标题。
pub const DISPOSITION_TITLE: &str = "性格与记忆";

/// 性情区的说明（**含换行与缩进**）。
pub const DISPOSITION_DESC: &str = "4 个体验参数可手动调校（未设 = 基线行为，零变化）；「从使用中学习」默认关。\n            参数只覆盖体验层（记忆遗忘/好奇/语气/整合节奏），治理与内核参数不可调。";

/// 体验参数卡的标题（**首字符是空格**）。
pub const DISPOSITION_KNOB_CARD_TITLE: &str = " 体验参数 · 手动调校";

/// 体验参数卡的计数文案。
pub const DISPOSITION_KNOB_CARD_COUNT: &str = "未设 = 基线行为";

/// 性情区预设条的旁注。
pub const DISPOSITION_PRESET_HINT: &str = "预设只动这 4 个体验参数（「恢复基线」另把「从使用中学习」关回默认关），点击即生效；失败回填旧值 + 横幅。";

/// 「从使用中学习」卡的标题（**首字符是空格**）。
pub const SELF_TUNING_CARD_TITLE: &str = " 从使用中学习";

/// 「从使用中学习」卡的计数文案。
pub const SELF_TUNING_CARD_COUNT: &str = "默认关";

/// 学习日志卡的标题（**首字符是空格**）。
pub const TUNING_LOG_CARD_TITLE: &str = " 学习日志（自动调整记录）";

/// 学习日志的「刷新」按钮文案。
pub const TUNING_LOG_REFRESH_LABEL: &str = "刷新";

/// 学习日志的「读取中」按钮文案。
pub const TUNING_LOG_LOADING_LABEL: &str = "读取中…";

/// 学习日志「不可读」分支的文案。
pub const TUNING_LOG_UNREADABLE: &str = "学习日志仅桌面版可读（接口已备）。";

/// 学习日志「空」分支的文案。
pub const TUNING_LOG_EMPTY: &str = "暂无自动调整记录。";

/// 学习日志行尾的说明。
pub const TUNING_LOG_UNDO_NOTE: &str = "「撤销」= 把该参数拨回记录原值并立即生效（写回值走同一条 apply 路径，不做独立撤销请求）。";

/// 性情区的脚注。
pub const DISPOSITION_FOOTNOTE: &str = "自动调整记录由后端写入数据目录的 tuning-log.jsonl，此处只读；未接的信号不会产生记录（0 装诚实）。";

/// 「撤销」按钮文案。
pub const TUNING_LOG_UNDO_LABEL: &str = "撤销";

// ── 数据形状（**宿主注入** ⇒ 组件不碰数据面） ───────────────────────────────

/// 一个能力开关的**定义**（**表的一行** ⇒ 数据，不是代码）。
#[derive(Debug, Clone, PartialEq)]
pub struct CapabilityDef {
    /// 能力键（`CapabilityToggles` 的键）。
    pub key: String,
    /// 行首图标（**几何数据来自上游包** ⇒ 两侧逐字一致）。
    pub icon: &'static IconData,
    /// 行标题（中文）。
    pub label: String,
    /// 后端 env 名（等宽芯片里显示）。
    pub env: String,
    /// 行副文案。
    pub desc: String,
    /// **反向旋钮**：开启 = 不注入（如 `APEIRETH_DISABLE_*` ⇒ 默认开）。
    pub invert: bool,
    /// **前置能力**：未开启时本行不可用（`requires`）。
    pub requires: Option<String>,
    /// 行的**附加 class**（如嵌套行的 `cap-row-nested`；`None` ⇒ 无）。
    ///
    /// ⚠️ 生产里依赖子行是 `class="cap-row cap-row-nested"` ⇒ 它的**嵌套展示**
    /// 语义在 class 上 ⇒ 归**定义**（表的一行）⇒ 组件照抄。
    pub row_class: Option<String>,
    /// **嵌套行**（插在**父行之后、同层**）：父行「开」时才渲染。
    pub nested: Option<KnobSpec>,
}

impl CapabilityDef {
    /// 建一条定义（**不带** `invert` / `requires` / 嵌套行）。
    pub fn new(
        key: impl Into<String>,
        icon: &'static IconData,
        label: impl Into<String>,
        env: impl Into<String>,
        desc: impl Into<String>,
    ) -> Self {
        Self {
            key: key.into(),
            icon,
            label: label.into(),
            env: env.into(),
            desc: desc.into(),
            invert: false,
            requires: None,
            row_class: None,
            nested: None,
        }
    }

    /// 标成**反向旋钮**（builder 风格）。
    pub fn inverted(mut self) -> Self {
        self.invert = true;
        self
    }

    /// 挂上**前置能力**（builder 风格）。
    pub fn requires(mut self, key: impl Into<String>) -> Self {
        self.requires = Some(key.into());
        self
    }

    /// 挂上**行的附加 class**（builder 风格）。
    pub fn with_row_class(mut self, row_class: impl Into<String>) -> Self {
        self.row_class = Some(row_class.into());
        self
    }

    /// 挂上**嵌套行**（builder 风格）。
    pub fn with_nested(mut self, nested: KnobSpec) -> Self {
        self.nested = Some(nested);
        self
    }
}

/// 一个**旋钮行**（滑杆）的规格（**数据** ⇒ 组件只渲染）。
///
/// ⚠️ 字段是 `&'static str`（**不是** `String`）⇒ 规格表可以是 **`const`**
/// ⇒ 宿主侧直接写常量，**不复制第二份**。
#[derive(Debug, Clone, PartialEq)]
pub struct KnobSpec {
    /// 取值所在的键（宿主喂）。
    pub value_key: &'static str,
    /// 行首图标。
    pub icon: &'static IconData,
    /// 行标题。
    pub label: &'static str,
    /// 后端 env 名。
    pub env: &'static str,
    /// 行副文案。
    pub desc: &'static str,
    /// 滑杆下界（**逐字**：生产写的是字符串）。
    pub min: &'static str,
    /// 滑杆上界。
    pub max: &'static str,
    /// 滑杆步进。
    pub step: &'static str,
    /// 取值展示的小数位（`toFixed` 位数）。
    pub digits: usize,
    /// 基线提示（如「基线 1.0」；`None` ⇒ 不渲染）。
    pub baseline: Option<&'static str>,
    /// 取值展示的前缀（如「×」）。
    pub prefix: &'static str,
    /// 取值展示的后缀（如「 回合」）。
    pub suffix: &'static str,
}


/// 能力取值（布尔 / 数值）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CapabilityValue {
    /// 开关型。
    Bool(bool),
    /// 数值型（旋钮）。
    Number(f64),
}

impl CapabilityValue {
    /// 取布尔值（非布尔 ⇒ `false`）。
    pub fn as_bool(self) -> bool {
        match self {
            CapabilityValue::Bool(flag) => flag,
            CapabilityValue::Number(_) => false,
        }
    }

    /// 取数值（非数值 ⇒ `0.0`）。
    pub fn as_number(self) -> f64 {
        match self {
            CapabilityValue::Bool(_) => 0.0,
            CapabilityValue::Number(value) => value,
        }
    }
}

/// 能力表（键 → 取值；**宿主喂**）。
pub type CapabilityTable = BTreeMap<String, CapabilityValue>;

/// 一个伙伴人设。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Persona {
    /// 人设 id（**深链与表单 id 的来源** ⇒ 逐字）。
    pub id: String,
    /// 名称。
    pub name: String,
    /// 固定模型（空 ⇒ 跟随全局设置）。
    pub model: String,
    /// 人设正文（system 消息）。
    pub persona: String,
}

/// 用户印象档案。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Impression {
    /// 归属伙伴 id。
    pub persona_id: String,
    /// 归属伙伴名。
    pub persona_name: String,
    /// 印象正文。
    pub summary: String,
    /// 最近更新时刻（**epoch 毫秒**；`0` ⇒ 没有记录）。
    pub updated_at: i64,
    /// 来源（`auto` / `manual`）。
    pub source: String,
    /// 自动总结开关。
    pub auto_refresh: bool,
    /// **宿主格式化好的**时刻文本（生产用 `toLocaleString()` ⇒ **宿主算**，组件只渲染）。
    pub updated_text: String,
}

/// 学习日志的一条记录。
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TuningLogEntry {
    /// 序号（**倒序展示** ⇒ 宿主排序）。
    pub seq: i64,
    /// 参数名（snake_case）。
    pub param: String,
    /// 参数的**中文标签**（宿主按映射表给；缺省 ⇒ 用参数名）。
    pub param_label: String,
    /// 调整前的值。
    pub previous: f64,
    /// 调整后的值。
    pub next: f64,
    /// 调整原因。
    pub reason: String,
    /// 时刻（epoch 毫秒）。
    pub at_epoch_ms: i64,
    /// **宿主格式化好的**时刻文本（`toLocaleString('zh-CN')` ⇒ 宿主算）。
    pub at_text: String,
}

// ── 判定（**本批最高危的两条口径**） ─────────────────────────────────────────

/// 旋钮的「开」判定（**`invert` 反向旋钮** ⇒ 判反 = 静默错误）。
///
/// ## 口径（生产 `isCapOn` 的逐字翻译）
///
/// ```text
/// value = capabilities[def.key];
/// def.invert ? value === false : value === true
/// ```
///
/// ⚠️ `typedRecall` 的 env 是 `APEIRETH_DISABLE_TYPED_RECALL` ⇒ 「开」= `false`
/// ⇒ 若按普通布尔处理 ⇒ 画面正常但**语义相反** ⇒ **静默错误**。
pub fn is_cap_on(def: &CapabilityDef, table: &CapabilityTable) -> bool {
    match table.get(&def.key) {
        Some(CapabilityValue::Bool(value)) => {
            if def.invert {
                !*value
            } else {
                *value
            }
        }
        Some(CapabilityValue::Number(value)) => {
            if def.invert {
                *value == 0.0
            } else {
                *value != 0.0
            }
        }
        None => false,
    }
}

/// 依赖行的禁用判定（**`requires`** ⇒ 父行状态驱动子行）。
///
/// ## 口径（生产 `capDisabled` 的逐字翻译）
///
/// ```text
/// def.requires !== undefined && capabilities[def.requires] !== true
/// ```
pub fn cap_disabled(def: &CapabilityDef, table: &CapabilityTable) -> bool {
    let Some(required) = def.requires.as_ref() else {
        return false;
    };
    !matches!(
        table.get(required),
        Some(CapabilityValue::Bool(true))
    )
}

/// 已启用的行数（**已开且未禁用** ⇒ 生产的 `enabledCount`）。
pub fn enabled_count(defs: &[CapabilityDef], table: &CapabilityTable) -> usize {
    defs.iter()
        .filter(|def| is_cap_on(def, table) && !cap_disabled(def, table))
        .count()
}

/// 读一个布尔能力（**缺省 ⇒ `false`**）。
pub fn capability_bool(table: &CapabilityTable, key: &str) -> bool {
    matches!(
        table.get(key),
        Some(CapabilityValue::Bool(true))
    )
}

/// 读一个数值能力（**缺省 ⇒ `0.0`**）。
pub fn capability_number(table: &CapabilityTable, key: &str) -> f64 {
    table.get(key).map(|value| value.as_number()).unwrap_or(0.0)
}

// ── 意图出口（**宿主注入** ⇒ 组件不自己改状态） ─────────────────────────────

/// 能力开关的「切换」意图（入参 = 能力键）。
pub type CapabilityIntent = Rc<dyn Fn(&str)>;

/// 旋钮的「拨动」意图（入参 = 能力键）。
///
/// ⚠️ **取值不在事件载荷里**：浏览器的 DOM 后端构造事件时**不回填目标取值**
/// ⇒ 引擎只发**键**；宿主自行回读 DOM（`input[type=range]` 的取值是
/// **property**，不是属性）。
pub type KnobIntent = Rc<dyn Fn(&str)>;

/// 人设的「选中」意图（入参 = 人设 id）。
pub type PersonaIntent = Rc<dyn Fn(&str)>;

/// 一般的「动作」意图（入参 = 动作名）。
pub type ActionIntent = Rc<dyn Fn(&str)>;

// ── 组件 ────────────────────────────────────────────────────────────────────

/// 人格与记忆面的 props（**宿主注入** ⇒ 组件是纯函数）。
#[derive(Clone)]
pub struct CharacterPanelsProps {
    /// 当前分区（`personality` / `impression` / `cognition` / `disposition`）。
    pub section: Signal<String>,
    /// 人设列表（**顺序即渲染顺序**）。
    pub personas: Signal<Vec<Persona>>,
    /// 当前伙伴 id。
    pub active_persona_id: Signal<String>,
    /// 字段旗标（键 → 状态；**宿主持有** ⇒ 组件只渲染）。
    pub text_flags: Signal<Vec<(String, FieldFlagState)>>,
    /// 印象档案。
    pub impression: Signal<Impression>,
    /// 印象草稿（**失焦 / 回车才落盘** ⇒ 宿主持有）。
    pub impression_draft: Signal<String>,
    /// 印象操作的错误文案（空 ⇒ 不渲染错误行）。
    pub impression_error: Signal<String>,
    /// 「立即总结」进行中。
    pub impression_busy: Signal<bool>,
    /// 能力表（**键 → 取值**）。
    pub capabilities: Signal<CapabilityTable>,
    /// 当前生效中的键（`pending` 态；`None` ⇒ 无）。
    pub pending_key: Signal<Option<String>>,
    /// 学习日志（`None` ⇒ **不可读** ⇒ 走「仅桌面版可读」分支）。
    pub tuning_log: Signal<Option<Vec<TuningLogEntry>>>,
    /// 学习日志读取中。
    pub tuning_log_loading: Signal<bool>,
    /// 全局模型（人设卡的占位；**宿主喂** ⇒ 与参考侧同一份）。
    pub global_model: String,
    /// 记忆流的能力定义表（**数据**）。
    pub memory_flow_defs: Vec<CapabilityDef>,
    /// 认知增强的能力定义表（**数据**）。
    pub cognition_defs: Vec<CapabilityDef>,
    /// 社区与账本的能力定义表（**数据**）。
    pub community_defs: Vec<CapabilityDef>,
    /// 「从使用中学习」的能力定义（**数据**）。
    pub self_tuning_def: CapabilityDef,
    /// 体验参数的旋钮规格表（**数据**）。
    pub disposition_knobs: Vec<KnobSpec>,
    /// 性情区的预设名（**顺序即渲染顺序**；含「恢复基线」⇒ 图标不同）。
    pub disposition_presets: Vec<String>,
    /// 「切换能力开关」意图出口。
    pub on_toggle_capability: CapabilityIntent,
    /// 「拨动旋钮」意图出口。
    pub on_set_knob: KnobIntent,
    /// 「选中伙伴」意图出口。
    pub on_select_persona: PersonaIntent,
    /// 「新增伙伴」意图出口。
    pub on_add_persona: ActionIntent,
    /// 「删除伙伴」意图出口（**宿主弹二次确认** ⇒ 引擎只发意图）。
    pub on_remove_persona: PersonaIntent,
    /// 印象区的动作意图（`summarize` / `toggle-auto` / `clear`）。
    pub on_impression: ActionIntent,
    /// 「应用预设」意图出口（入参 = 预设名）。
    pub on_apply_preset: ActionIntent,
    /// 「撤销」意图出口（入参 = 日志序号）。
    pub on_undo_tuning: ActionIntent,
    /// 「整组提交」意图出口（入参 = 组名；失焦 ⇒ 提交）。
    pub on_submit_group: ActionIntent,
    /// 「字段改动」意图出口（入参 = 字段键；失焦 / 回车 ⇒ 落盘）。
    ///
    /// ⚠️ **取值不在事件载荷里** ⇒ 引擎只发**键**；宿主回读 DOM（人设输入框
    /// 有 `id` ⇒ 可直接定位）。
    pub on_field_input: ActionIntent,
}

impl std::fmt::Debug for CharacterPanelsProps {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CharacterPanelsProps")
            .field("section", &self.section.get())
            .field("personas", &self.personas.get().len())
            .field("active_persona_id", &self.active_persona_id.get())
            .field("text_flags", &self.text_flags.get().len())
            .field("impression", &self.impression.get())
            .field("capabilities", &self.capabilities.get().len())
            .field("pending_key", &self.pending_key.get())
            .field("tuning_log", &self.tuning_log.get().map(|log| log.len()))
            .finish_non_exhaustive()
    }
}

/// 人格与记忆面组件（**四个分区由宿主的 `section` 决定**）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CharacterPanels;

impl UiComponent for CharacterPanels {
    type Props = CharacterPanelsProps;
    type Output = VNode;

    fn render(props: &Self::Props) -> Self::Output {
        match props.section.get().as_str() {
            "impression" => render_impression(props),
            "cognition" => render_cognition(props),
            "disposition" => render_disposition(props),
            _ => render_personality(props),
        }
    }
}

// ── 分区渲染 ────────────────────────────────────────────────────────────────

/// 人设分区。
fn render_personality(props: &CharacterPanelsProps) -> VNode {
    let personas = props.personas.get();
    let active = props.active_persona_id.get();
    let flags = props.text_flags.get();
    let cards: Vec<VNode> = personas
        .iter()
        .enumerate()
        .map(|(index, persona)| render_persona_card(persona, index, &active, &flags, props))
        .collect();
    let add = props.on_add_persona.clone();
    let submit_personas = props.on_submit_group.clone();
    let submit_personas_key = props.on_submit_group.clone();
    let title = PERSONALITY_TITLE;
    let desc = PERSONALITY_DESC;
    let notice = PERSONALITY_NOTICE;
    let badge_label = BADGE_LIVE_LABEL;
    let badge_class = BADGE_LIVE_CLASS;
    let add_label = PERSONA_ADD_LABEL;

    view! {
        div(class = "setting-block") {
            h3(class = "block-title") { title }
            " "
            p(class = "block-desc") { desc }
            " "
            div(class = "text-group", role = "group", aria-label = "伙伴人设", onfocusout = move |_| { (submit_personas)("personas"); }, onkeydown = move |_| { (submit_personas_key)("personas"); }) {
                for card in cards {
                    { card }
                }
            }
            " "
            button(class = "quiet-button", onclick = move |_| { (add)("add-persona"); }) {
                { Icon::render(&IconProps::new(&icons::ICON_PLUS, 14)) }
                " "
                span { add_label }
            }
            " "
            div(class = "notice-box") {
                span(class = badge_class) { badge_label }
                " "
                span { notice }
            }
        }
    }
}

/// 一张人设卡。
fn render_persona_card(
    persona: &Persona,
    index: usize,
    active: &str,
    flags: &[(String, FieldFlagState)],
    props: &CharacterPanelsProps,
) -> VNode {
    let is_active = persona.id == active;
    let index_label = format!("伙伴 {}", index + 1);
    let current_label = if is_active {
        PERSONA_CURRENT_LABEL
    } else {
        PERSONA_SELECT_LABEL
    };
    let select_title = PERSONA_SELECT_TITLE;
    let remove_title = PERSONA_REMOVE_TITLE;
    let name_label = PERSONA_NAME_LABEL;
    let model_label = PERSONA_MODEL_LABEL;
    let text_label = PERSONA_TEXT_LABEL;
    let name_placeholder = PERSONA_NAME_PLACEHOLDER;
    let text_placeholder = PERSONA_TEXT_PLACEHOLDER;
    let model_placeholder = props.global_model.clone();

    let name_id = format!("persona-name-{}", persona.id);
    let model_id = format!("persona-model-{}", persona.id);
    let text_id = format!("persona-text-{}", persona.id);

    let name_flag = FieldFlagProps::new(flag_of(
        flags,
        &format!("persona.name:{}", persona.id),
    ));
    let model_flag = FieldFlagProps::new(flag_of(
        flags,
        &format!("persona.model:{}", persona.id),
    ));
    let text_flag = FieldFlagProps::new(flag_of(
        flags,
        &format!("persona.text:{}", persona.id),
    ));

    let name_key = format!("persona.name:{}", persona.id);
    let model_key = format!("persona.model:{}", persona.id);
    let text_key = format!("persona.text:{}", persona.id);
    let field_input = props.on_field_input.clone();
    let name_input = field_input.clone();
    let model_input = field_input.clone();
    let text_input = field_input.clone();

    let select_id = persona.id.clone();
    let select = props.on_select_persona.clone();
    let remove_id = persona.id.clone();
    let remove = props.on_remove_persona.clone();
    // 生产：`disabled={personas.length <= 1}` ⇒ 只剩一个伙伴时不可删。
    let remove_disabled = props.personas.get().len() <= 1;

    let mut select_button = VNode::element("button")
        .attr("title", select_title)
        .attr(
            "class",
            if is_active {
                "quiet-button selected"
            } else {
                "quiet-button"
            },
        );
    select_button = select_button.handler(
        EventKind::Click,
        ElementHandler::new(move |_event: &mut DomEvent| { (select)(&select_id); }),
    );
    let select_node = select_button.child(VNode::text(current_label)).build();

    let mut remove_button = VNode::element("button")
        .attr("class", "quiet-button danger-text")
        .attr("title", remove_title);
    if remove_disabled {
        remove_button = remove_button.attr("disabled", "");
    }
    remove_button = remove_button.handler(
        EventKind::Click,
        ElementHandler::new(move |_event: &mut DomEvent| { (remove)(&remove_id); }),
    );
    let remove_node = remove_button
        .child(Icon::render(&IconProps::new(&icons::ICON_TRASH_2, 13)))
        .build();

    // ⚠️ `label` 的子节点顺序**逐字照抄生产**：文本在前、字段旗标在后 ⇒
    // ⇒ 用**构造器 API**（`view!` 里「裸标识符 + 紧跟 `{`」会被当成元素）。
    let name_label_node = VNode::element("label")
        .attr("for", name_id.clone())
        .child(VNode::text(name_label))
        .child(FieldFlag::render(&name_flag))
        .build();
    let model_label_node = VNode::element("label")
        .attr("for", model_id.clone())
        .child(VNode::text(model_label))
        .child(FieldFlag::render(&model_flag))
        .build();
    let text_label_node = VNode::element("label")
        .attr("for", text_id.clone())
        .child(VNode::text(text_label))
        .child(FieldFlag::render(&text_flag))
        .build();

    view! {
        div(class = "persona-card") {
            div(class = "persona-card-head") {
                span(class = "persona-index") { index_label }
                " "
                div(class = "persona-card-actions") {
                    { select_node }
                    " "
                    { remove_node }
                }
            }
            " "
            div(class = "form-group") {
                { name_label_node }
                " "
                input(id = name_id.clone(), type = "text", placeholder = name_placeholder, oninput = move |_| { (name_input)(&name_key); })
                " "
            }
            " "
            div(class = "form-group") {
                { model_label_node }
                " "
                input(id = model_id.clone(), type = "text", placeholder = model_placeholder, oninput = move |_| { (model_input)(&model_key); })
                " "
            }
            " "
            div(class = "form-group") {
                { text_label_node }
                " "
                textarea(id = text_id.clone(), rows = "5", placeholder = text_placeholder, oninput = move |_| { (text_input)(&text_key); })
                " "
            }
        }
    }
}

/// 字段旗标的状态（**查不到 ⇒ 无旗标**）。
fn flag_of(flags: &[(String, FieldFlagState)], key: &str) -> FieldFlagState {
    flags
        .iter()
        .find(|(name, _)| name == key)
        .map(|(_, state)| *state)
        .unwrap_or(FieldFlagState::None)
}

/// 印象分区。
fn render_impression(props: &CharacterPanelsProps) -> VNode {
    let impression = props.impression.get();
    let personas = props.personas.get();
    let active = props.active_persona_id.get();
    let busy = props.impression_busy.get();
    let error = props.impression_error.get();
    let flags = props.text_flags.get();

    let persona_name = personas
        .iter()
        .find(|persona| persona.id == active)
        .map(|persona| persona.name.clone())
        .unwrap_or_default();
    let shown_name = if persona_name.is_empty() {
        IMPRESSION_NO_PERSONA.to_owned()
    } else {
        persona_name
    };

    let title = IMPRESSION_TITLE;
    let desc = IMPRESSION_DESC;
    let badge_label = BADGE_LIVE_LABEL;
    let badge_class = BADGE_LIVE_CLASS;
    let notice_prefix = IMPRESSION_NOTICE_PREFIX;
    let notice_suffix = IMPRESSION_NOTICE_SUFFIX;
    let summarize_label = if busy {
        IMPRESSION_SUMMARIZING_LABEL
    } else {
        IMPRESSION_SUMMARIZE_LABEL
    };
    let auto_prefix = IMPRESSION_AUTO_PREFIX;
    let auto_state = if impression.auto_refresh {
        IMPRESSION_AUTO_ON
    } else {
        IMPRESSION_AUTO_OFF
    };
    let auto_label = format!("{auto_prefix}{auto_state}");
    let auto_title = IMPRESSION_AUTO_TITLE;
    let clear_label = IMPRESSION_CLEAR_LABEL;
    let clear_title = IMPRESSION_CLEAR_TITLE;
    let text_label = IMPRESSION_TEXT_LABEL;
    let text_placeholder = IMPRESSION_TEXT_PLACEHOLDER;
    let hint_tail = IMPRESSION_HINT_TAIL;
    let impression_flag = FieldFlagProps::new(flag_of(&flags, "user.impression"));

    // 「最近更新」分支（`updatedAt > 0` ⇒ 有记录）。
    let hint_head = if impression.updated_at > 0 {
        let source = if impression.source == "auto" {
            IMPRESSION_SOURCE_AUTO
        } else {
            IMPRESSION_SOURCE_MANUAL
        };
        format!(
            "{}{}（{}）。",
            IMPRESSION_UPDATED_PREFIX, impression.updated_text, source
        )
    } else {
        IMPRESSION_NONE_TEXT.to_owned()
    };

    let summarize = props.on_impression.clone();
    let toggle_auto = props.on_impression.clone();
    let clear = props.on_impression.clone();
    let submit_impression = props.on_submit_group.clone();
    let submit_impression_key = props.on_submit_group.clone();
    let impression_input = props.on_field_input.clone();

    let mut summarize_button = VNode::element("button").attr("class", "quiet-button");
    if busy {
        summarize_button = summarize_button.attr("disabled", "");
    }
    summarize_button = summarize_button.handler(
        EventKind::Click,
        ElementHandler::new(move |_event: &mut DomEvent| { (summarize)("summarize"); }),
    );
    let summarize_node = summarize_button
        .child(Icon::render(
            &IconProps::new(&icons::ICON_REFRESH_CCW, 13)
                .extra_class(if busy { "spin" } else { "" }),
        ))
        .child(VNode::text(" "))
        .child(view! { span { summarize_label } })
        .build();

    let auto_node = VNode::element("button")
        .attr("title", auto_title)
        .attr(
            "class",
            if impression.auto_refresh {
                "quiet-button selected"
            } else {
                "quiet-button"
            },
        )
        .handler(
            EventKind::Click,
            ElementHandler::new(move |_event: &mut DomEvent| { (toggle_auto)("toggle-auto"); }),
        )
        .child(VNode::text(auto_label))
        .build();

    let clear_node = VNode::element("button")
        .attr("class", "quiet-button danger-text")
        .attr("title", clear_title)
        .handler(
            EventKind::Click,
            ElementHandler::new(move |_event: &mut DomEvent| { (clear)("clear"); }),
        )
        .child(Icon::render(&IconProps::new(&icons::ICON_TRASH_2, 13)))
        .child(VNode::text(" "))
        .child(view! { span { clear_label } })
        .build();

    let error_node = if error.is_empty() {
        VNode::fragment(Vec::new())
    } else {
        view! { p(class = "field-hint error-hint") { error.clone() } }
    };

    // 印象正文的标签（文本在前、旗标在后 ⇒ **构造器 API**）。
    let impression_label_node = VNode::element("label")
        .attr("for", "impression-text")
        .child(VNode::text(text_label))
        .child(FieldFlag::render(&impression_flag))
        .build();

    view! {
        div(class = "setting-block") {
            h3(class = "block-title") { title }
            " "
            p(class = "block-desc") { desc }
            " "
            div(class = "notice-box") {
                span(class = badge_class) { badge_label }
                " "
                span {
                    notice_prefix
                    strong { shown_name }
                    notice_suffix
                }
            }
            " "
            div(class = "impression-actions") {
                { summarize_node }
                " "
                { auto_node }
                " "
                { clear_node }
            }
            " "
            div(class = "text-group", role = "group", aria-label = "用户印象", onfocusout = move |_| { (submit_impression)("user.impression"); }, onkeydown = move |_| { (submit_impression_key)("user.impression"); }) {
                div(class = "form-group") {
                    { impression_label_node }
                    " "
                    textarea(id = "impression-text", rows = "8", maxlength = "2000", placeholder = text_placeholder, oninput = move |_| { (impression_input)("user.impression"); })
                    " "
                }
            }
            " "
            { error_node }
            " "
            p(class = "field-hint") {
                hint_head
                hint_tail
            }
        }
    }
}

/// 认知分区。
fn render_cognition(props: &CharacterPanelsProps) -> VNode {
    let table = props.capabilities.get();
    let pending = props.pending_key.get();
    let total =
        props.memory_flow_defs.len() + props.cognition_defs.len() + props.community_defs.len();
    let enabled = enabled_count(&props.memory_flow_defs, &table)
        + enabled_count(&props.cognition_defs, &table)
        + enabled_count(&props.community_defs, &table);

    let title = COGNITION_TITLE;
    let desc = COGNITION_DESC;
    let summary_prefix = COGNITION_SUMMARY_PREFIX;
    let summary_suffix = format!(" / {total}{COGNITION_SUMMARY_SUFFIX}");
    let preset_label = COGNITION_PRESET_LABEL;
    let preset_hint = COGNITION_PRESET_HINT;
    let notice_head = COGNITION_NOTICE_HEAD;
    let notice_strong = COGNITION_NOTICE_STRONG;
    let notice_tail = COGNITION_NOTICE_TAIL;
    let footnote = COGNITION_FOOTNOTE;
    let enabled_text = enabled.to_string();
    // ⚠️ `view!` 里**大写开头的标识符**被当成**组件名** ⇒ 文案常量先落到
    // 小写局部变量再用（否则宏会把常量当组件去 `builder()`）。
    let default_badge = DEFAULT_BADGE_LABEL;
    let default_title = DEFAULT_TITLE;
    let default_text_head = DEFAULT_TEXT_HEAD;
    let default_text_code = DEFAULT_TEXT_CODE;
    let default_text_tail = DEFAULT_TEXT_TAIL;

    let memory_flow_card = render_capability_card(
        &icons::ICON_LAYERS,
        MEMORY_FLOW_CARD_TITLE,
        &format!(
            "{}/{}",
            enabled_count(&props.memory_flow_defs, &table),
            props.memory_flow_defs.len()
        ),
        &props.memory_flow_defs,
        &table,
        pending.as_deref(),
        props,
    );
    let cognition_card = render_capability_card(
        &icons::ICON_SPARKLES,
        COGNITION_CARD_TITLE,
        &format!(
            "{}/{}",
            enabled_count(&props.cognition_defs, &table),
            props.cognition_defs.len()
        ),
        &props.cognition_defs,
        &table,
        pending.as_deref(),
        props,
    );
    let community_card = render_capability_card(
        &icons::ICON_GIT_BRANCH,
        COMMUNITY_CARD_TITLE,
        &format!(
            "{}/{}",
            enabled_count(&props.community_defs, &table),
            props.community_defs.len()
        ),
        &props.community_defs,
        &table,
        pending.as_deref(),
        props,
    );

    let preset = props.on_apply_preset.clone();

    view! {
        div(class = "setting-block") {
            h3(class = "block-title") { title }
            " "
            p(class = "block-desc") { desc }
            " "
            div(class = "cap-summary") {
                { Icon::render(&IconProps::new(&icons::ICON_BRAIN, 14)) }
                " "
                span {
                    summary_prefix
                    b { enabled_text }
                    summary_suffix
                }
            }
            " "
            div(class = "preset-bar") {
                button(class = "quiet-button", onclick = move |_| { (preset)("recommended"); }) {
                    { Icon::render(&IconProps::new(&icons::ICON_SPARKLES, 14)) }
                    " "
                    span { preset_label }
                }
                " "
                span(class = "preset-hint") { preset_hint }
            }
            " "
            div(class = "notice-box") {
                span {
                    notice_head
                    strong { notice_strong }
                    notice_tail
                }
            }
            " "
            { memory_flow_card }
            " "
            { cognition_card }
            " "
            { community_card }
            " "
            div(class = "cap-default-card") {
                span(class = "cap-default-badge on") {
                    { Icon::render(&IconProps::new(&icons::ICON_CIRCLE_CHECK, 11)) }
                    default_badge
                }
                " "
                strong(class = "cap-default-title") { default_title }
                " "
                p(class = "cap-default-text") {
                    default_text_head
                    code { default_text_code }
                    default_text_tail
                }
            }
            " "
            p(class = "cap-footnote") { footnote }
        }
    }
}

/// 性情分区。
fn render_disposition(props: &CharacterPanelsProps) -> VNode {
    let table = props.capabilities.get();
    let pending = props.pending_key.get();
    let log = props.tuning_log.get();
    let loading = props.tuning_log_loading.get();

    let title = DISPOSITION_TITLE;
    let desc = DISPOSITION_DESC;
    let knob_card_title = DISPOSITION_KNOB_CARD_TITLE;
    let knob_card_count = DISPOSITION_KNOB_CARD_COUNT;
    let preset_hint = DISPOSITION_PRESET_HINT;
    let self_tuning_title = SELF_TUNING_CARD_TITLE;
    let self_tuning_count = SELF_TUNING_CARD_COUNT;
    let log_title = TUNING_LOG_CARD_TITLE;
    let refresh_label = if loading {
        TUNING_LOG_LOADING_LABEL
    } else {
        TUNING_LOG_REFRESH_LABEL
    };
    let unreadable = TUNING_LOG_UNREADABLE;
    let empty_text = TUNING_LOG_EMPTY;
    let undo_note = TUNING_LOG_UNDO_NOTE;
    let footnote = DISPOSITION_FOOTNOTE;

    // 体验参数卡（4 个滑杆 ⇒ **数据驱动**）。
    let knob_rows: Vec<VNode> = props
        .disposition_knobs
        .iter()
        .map(|spec| render_knob_row(spec, &table, &props.on_set_knob))
        .collect();
    let knob_card = render_card(
        &icons::ICON_SLIDERS_HORIZONTAL,
        knob_card_title,
        knob_card_count,
        knob_rows,
    );

    // 自学习卡（**一张只有一行的能力卡** ⇒ 复用同一个组件）。
    let self_tuning_card = render_capability_card(
        &icons::ICON_SPARKLES,
        self_tuning_title,
        self_tuning_count,
        std::slice::from_ref(&props.self_tuning_def),
        &table,
        pending.as_deref(),
        props,
    );

    // 学习日志卡（**三个分支**：不可读 / 空 / 有记录）。
    let refresh = props.on_undo_tuning.clone();
    let mut refresh_button = VNode::element("button").attr("class", "quiet-button");
    if loading {
        refresh_button = refresh_button.attr("disabled", "");
    }
    refresh_button = refresh_button.handler(
        EventKind::Click,
        ElementHandler::new(move |_event: &mut DomEvent| { (refresh)("refresh-log"); }),
    );
    let refresh_node = refresh_button
        .child(Icon::render(&IconProps::new(&icons::ICON_REFRESH_CCW, 13)))
        .child(VNode::text(" "))
        .child(view! { span { refresh_label } })
        .build();

    let log_body: Vec<VNode> = match log.as_ref() {
        None => vec![view! {
            div(class = "notice-box") {
                { Icon::render(&IconProps::new(&icons::ICON_INFO, 14)) }
                " "
                span { unreadable }
            }
        }],
        Some(entries) if entries.is_empty() => vec![view! {
            div(class = "notice-box") {
                { Icon::render(&IconProps::new(&icons::ICON_INFO, 14)) }
                " "
                span { empty_text }
            }
        }],
        Some(entries) => {
            let mut rows: Vec<VNode> = entries
                .iter()
                .map(|entry| render_tuning_row(entry, &props.on_undo_tuning))
                .collect();
            rows.push(view! {
                div(class = "cap-row cap-row-static") {
                    span(class = "cap-text") {
                        small { undo_note }
                    }
                }
            });
            rows
        }
    };

    let log_card = render_card_with_head(
        &icons::ICON_ARCHIVE,
        log_title,
        view! {
            span(class = "preset-actions") {
                { refresh_node }
            }
        },
        log_body,
    );

    // 预设芯片（4 枚 ⇒ **数据**：预设名由宿主给）。
    let preset_buttons: Vec<VNode> = props
        .disposition_presets
        .iter()
        .map(|name| {
            let preset = props.on_apply_preset.clone();
            let preset_name = name.clone();
            let icon = if name == PRESET_RESET_NAME {
                &icons::ICON_ROTATE_CCW
            } else {
                &icons::ICON_SPARKLES
            };
            view! {
                button(class = "quiet-button", onclick = move |_| { (preset)(&preset_name); }) {
                    { Icon::render(&IconProps::new(icon, 14)) }
                    " "
                    span { name.clone() }
                }
            }
        })
        .collect();

    view! {
        div(class = "setting-block") {
            h3(class = "block-title") { title }
            " "
            p(class = "block-desc") { desc }
            " "
            { knob_card }
            " "
            div(class = "preset-bar") {
                for button in preset_buttons {
                    { button }
                    " "
                }
                span(class = "preset-hint") { preset_hint }
            }
            " "
            { self_tuning_card }
            " "
            { log_card }
            " "
            p(class = "cap-footnote") { footnote }
        }
    }
}

/// 「恢复基线」预设名（生产里它是第四枚芯片 ⇒ 图标不同）。
pub const PRESET_RESET_NAME: &str = "恢复基线";

// ── 能力卡（**一个组件 + 一张表** ⇒ 供能力与治理面复用） ───────────────────

/// 渲染一张能力卡（**表驱动**）。
///
/// ## 为什么是「一个组件 + 一张表」
///
/// 生产的 8 个 `CapDef` 注册表（**74 行数据**）撑起 944 行 markup ⇒
///
/// 迁移的正确形态是**一个组件吃一张表** ⇒ 本函数**不认识任何具体能力**
/// ⇒ 表由宿主喂（固定数据）⇒ 组件里**没有一行能力定义**。
///
/// ## 嵌套行（**一项产出两个节点**）
///
/// 定义带 [`CapabilityDef::nested`] 且**父行「开」** ⇒ 在父行**之后**
/// （**同层**，不是子层）插入一行旋钮 ⇒ 与生产的
/// `{#if def.key === 'morphologyRecall' && …}` 同构。
pub fn render_capability_card(
    title_icon: &'static IconData,
    title: &str,
    count: &str,
    defs: &[CapabilityDef],
    table: &CapabilityTable,
    pending_key: Option<&str>,
    props: &CharacterPanelsProps,
) -> VNode {
    let rows = render_capability_rows(defs, table, pending_key, props);
    render_card(title_icon, title, count, rows)
}

/// 渲染能力卡的**行**（**表驱动** ⇒ 含嵌套行的插入点**）。
pub fn render_capability_rows(
    defs: &[CapabilityDef],
    table: &CapabilityTable,
    pending_key: Option<&str>,
    props: &CharacterPanelsProps,
) -> Vec<VNode> {
    let mut rows: Vec<VNode> = Vec::new();
    for def in defs {
        rows.push(render_capability_row(def, table, pending_key, props));
        // 嵌套行：**插在父行之后、同层** ⇒ 一项产出两个节点。
        if let Some(nested) = def.nested.as_ref() {
            if is_cap_on(def, table) {
                rows.push(render_knob_row(nested, table, &props.on_set_knob));
            }
        }
    }
    rows
}

/// 渲染一行能力开关（**`role="switch"`**）。
fn render_capability_row(
    def: &CapabilityDef,
    table: &CapabilityTable,
    pending_key: Option<&str>,
    props: &CharacterPanelsProps,
) -> VNode {
    let on = is_cap_on(def, table);
    let dim = cap_disabled(def, table);
    let pending = pending_key == Some(def.key.as_str());
    // 生产的两种写法在这里**等价**：
    // - 主行：`disabled={liveApplyPendingKey !== null}`；
    // - 依赖子行：`disabled={capDisabled(subDef) || liveApplyPendingKey !== null}`。
    // ⇒ 主行的定义**没有** `requires` ⇒ `cap_disabled` 恒 `false` ⇒ 两条口径合一。
    let disabled = dim || pending_key.is_some();

    let mut classes = vec!["cap-row"];
    if let Some(extra) = def.row_class.as_deref() {
        classes.push(extra);
    }
    if dim {
        classes.push("dim");
    }
    if pending {
        classes.push("pending");
    }

    let key = def.key.clone();
    let toggle = props.on_toggle_capability.clone();
    let mut button = VNode::element("button")
        .attr("role", "switch")
        .attr("class", classes.join(" "))
        .attr("aria-checked", if on { "true" } else { "false" })
        .attr("aria-label", def.label.clone());
    if disabled {
        // 静态 `disabled` 在 DOM 上是 `disabled=""` ⇒ **不是** `disabled="true"`。
        button = button.attr("disabled", "");
    }
    button = button.handler(
        EventKind::Click,
        ElementHandler::new(move |_event: &mut DomEvent| { (toggle)(&key); }),
    );

    let icon = Icon::render(&IconProps::new(def.icon, 15));
    let label_text = def.label.clone();
    let env = def.env.clone();
    let desc = def.desc.clone();
    let text = view! {
        span(class = "cap-text") {
            strong {
                label_text
                code(class = "cap-env") { env }
            }
            " "
            small { desc }
        }
    };
    let switch = view! {
        span(class = "cap-switch", class:on = on) {
            span(class = "cap-knob") { }
        }
    };

    button
        .child(view! { span(class = "cap-icon") { { icon } } })
        .child(text)
        .child(switch)
        .build()
}

/// 渲染一张能力卡的**壳**（标题 + 计数 + 行）。
fn render_card(
    title_icon: &'static IconData,
    title: &str,
    count: &str,
    rows: Vec<VNode>,
) -> VNode {
    let head = view! {
        div(class = "cap-card-head") {
            span(class = "cap-card-title") {
                { Icon::render(&IconProps::new(title_icon, 13)) }
                title
            }
            " "
            span(class = "cap-card-count") { count }
        }
    };
    render_card_shell(head, rows)
}

/// 渲染一张能力卡的**壳**（**卡头自定义** ⇒ 学习日志卡的卡头右侧是按钮）。
fn render_card_with_head(
    title_icon: &'static IconData,
    title: &str,
    head_right: VNode,
    rows: Vec<VNode>,
) -> VNode {
    let head = view! {
        div(class = "cap-card-head") {
            span(class = "cap-card-title") {
                { Icon::render(&IconProps::new(title_icon, 13)) }
                title
            }
            " "
            { head_right }
        }
    };
    render_card_shell(head, rows)
}

/// 能力卡壳的最终拼装（**行之间有空白** ⇒ 与生产的模板换行一致）。
fn render_card_shell(head: VNode, rows: Vec<VNode>) -> VNode {
    let mut card = VNode::element("div").attr("class", "cap-card");
    card = card.child(head);
    for row in rows {
        card = card.child(VNode::text(" ")).child(row);
    }
    card.build()
}

/// 渲染一行旋钮（**原生 `input[type=range]`** ⇒ 值是 **property** ⇒ 不写属性）。
fn render_knob_row(
    spec: &KnobSpec,
    table: &CapabilityTable,
    on_set_knob: &KnobIntent,
) -> VNode {
    let value = capability_number(table, spec.value_key);
    let shown = format!(
        "{}{}{}",
        spec.prefix,
        crate::num::to_fixed(value, spec.digits),
        spec.suffix
    );
    let label_text = spec.label.to_owned();
    let env = spec.env.to_owned();
    let desc = spec.desc.to_owned();
    let min = spec.min.to_owned();
    let max = spec.max.to_owned();
    let step = spec.step.to_owned();
    let icon = Icon::render(&IconProps::new(spec.icon, 15));
    let aria_label = spec.label.to_owned();
    let baseline = spec.baseline.map(str::to_owned);

    let knob_key = spec.value_key.to_owned();
    let knob_intent = on_set_knob.clone();
    let mut slider_wrap = VNode::element("span").attr("class", "cap-slider-wrap");
    slider_wrap = slider_wrap.child(
        VNode::element("input")
            .attr("type", "range")
            .attr("min", min)
            .attr("max", max)
            .attr("step", step)
            .attr("aria-label", aria_label)
            .handler(
                EventKind::Input,
                ElementHandler::new(move |_event: &mut DomEvent| { (knob_intent)(&knob_key); }),
            )
            .build(),
    );
    slider_wrap = slider_wrap.child(VNode::text(" "));
    if let Some(baseline) = baseline {
        slider_wrap = slider_wrap.child(view! { span(class = "preset-hint") { baseline } });
        slider_wrap = slider_wrap.child(VNode::text(" "));
    }
    slider_wrap = slider_wrap.child(view! { b { shown } });

    view! {
        div(class = "cap-row cap-row-static") {
            span(class = "cap-icon") { { icon } }
            " "
            span(class = "cap-text") {
                strong {
                    label_text
                    code(class = "cap-env") { env }
                }
                " "
                small { desc }
            }
            " "
            { slider_wrap.build() }
        }
    }
}

/// 渲染一行学习日志。
/// 学习日志行 `<small>` 里 `·` 之后的**换行 + 缩进**（**逐字照抄生产**）。
///
/// ⚠️ **22 个空格** = 生产源文件 `views/SettingsView.svelte:2845` 该行的**前导缩进**
/// ⇒ Svelte 的空白规则（实测）：**内部**空白**逐字保留** ⇒ 这段换行 + 缩进**不可省**。
const TUNING_LOG_DETAIL_INDENT: &str = "                      ";

fn render_tuning_row(entry: &TuningLogEntry, on_undo: &ActionIntent) -> VNode {
    let param_label = if entry.param_label.is_empty() {
        entry.param.clone()
    } else {
        entry.param_label.clone()
    };
    let param = entry.param.clone();
    let previous = crate::num::to_fixed(entry.previous, 2);
    let next = crate::num::to_fixed(entry.next, 2);
    let reason = entry.reason.clone();
    let at_text = entry.at_text.clone();
    // ⚠️ **空白逐字照抄生产**（2026-10-24 主代理实测）：
    // 生产的 `<small>` 内容是
    //   `\n{22 空格}{prev} → {next} ·\n{22 空格}{reason} · {time}\n{20 空格}`
    // ⇒ **Svelte 的空白规则（实测）**：
    //   ① 文本节点**首尾**的空白 ⇒ **裁掉**；
    //   ② 与**元素**相邻的空白 ⇒ **折叠成一个空格**（`{label}` 与 `<code>` 之间 ⇒ 一个空格）；
    //   ③ **内部**、且**不与元素相邻**的空白 ⇒ **逐字保留**（含换行 + 缩进）。
    // ⇒ 所以 `·` 之后的那个换行 + 22 个空格**必须保留**（`·` 后面接的是**表达式**不是元素）。
    // ⇒ 那 22 个空格 = 生产源文件该行的**前导缩进**（`views/SettingsView.svelte:2845`）。
    let detail = format!(
        "{previous} → {next} ·\n{TUNING_LOG_DETAIL_INDENT}{reason} · {at_text}"
    );
    // ⚠️ `{label}` 与 `<code>` 之间是**元素边界** ⇒ 折叠成**一个空格**（不是零个）。
    let head = format!("#{} {param_label} ", entry.seq);
    let undo_key = entry.seq.to_string();
    let undo_label = TUNING_LOG_UNDO_LABEL;
    let undo = on_undo.clone();

    view! {
        div(class = "cap-row cap-row-static") {
            span(class = "cap-icon") {
                { Icon::render(&IconProps::new(&icons::ICON_SLIDERS_HORIZONTAL, 15)) }
            }
            " "
            span(class = "cap-text") {
                strong {
                    head
                    code(class = "cap-env") { param }
                }
                " "
                small { detail }
            }
            " "
            button(class = "quiet-button", onclick = move |_| { (undo)(&undo_key); }) {
                undo_label
            }
        }
    }
}

