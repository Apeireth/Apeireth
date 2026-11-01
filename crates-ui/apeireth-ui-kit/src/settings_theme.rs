//! 主题外观面板（**主题子面**）—— 生产组件的 Rust 重写。
//!
//! ## 面边界（这一面只回答「长什么样」）
//!
//! | 在本模块 | 不在本模块（**宿主侧**） |
//! |---|---|
//! | 主题卡 / 配色卡的结构 / class 名 / 文本 | 主题与配色的取值**写 `<html>`**（`data-theme` / `data-accent`） |
//! | 自定义背景区的展示（按钮 / 错误行 / 预览块） | **文件选择**（`<input type=file>` 的取文件） |
//! | 二次确认弹层的**意图出口** | 二次确认弹层本体（**共享覆盖层组件**，不属任一面） |
//! | 内联样式的**取值**（固定数据喂进来的原文） | 内联样式的**浏览器序列化**（宿主侧 CSSOM 归一化） |
//!
//! ⇒ 数据（主题目录 / 配色目录 / 选中态 / 忙碌态 / 错误文案）由**宿主注入**
//! （`Props`）；引擎不碰数据面与宿主能力（设计文档 §1.3 第 6 条）。
//!
//! ## class 名是视觉契约（设计文档 §9.1）
//!
//! 生产组件的 class 名与本模块**逐字一致**（`.theme-panel` / `.theme-card` /
//! `.accent-card` / `.bg-actions` / …）⇒ **不改名**（改名 ⇒ 像素不一致）。
//!
//! ## 空白
//!
//! 生产模板里相邻元素之间的换行会折叠成**一个空格**（文本节点）⇒ 本模块
//! **逐个复现**这些空白文本节点。
//!
//! ⚠️ **图标与文字之间的那个空格与文字同属一个文本节点**（生产如此：图标是
//! 元素、文字是文本 ⇒ 中间的换行折叠成空格后**并进**文字那个节点）⇒
//! 按钮标签常量**自带前导空格**（`" 上传图片"`）⇒ 拆成两个文本节点会让 DOM
//! 逐字对照出现差异。
//!
//! ## 内联样式（设计文档 §9.2 C3）
//!
//! 主题样品是**内联渐变**、配色样品是**内联 CSS 变量覆写** ⇒ 两者的取值
//! **逐字**来自固定数据（本模块**不拼渐变**、**不重算颜色**、**不裁剪串**）。
//!
//! ⚠️ 浏览器的 CSSOM 会把取值**序列化**成自己的口径（默认角度省略 / 颜色转
//! `rgb()` / `url()` 加引号 / 位置补全 …）⇒ 引擎侧由**宿主侧**在挂载后用
//! CSSOM 归一化（见 `apeireth-ui-demo` 的主题宿主模块）⇒ 本模块只负责
//! **原样给出取值**（不重实现浏览器的序列化规则）。
//!
//! ## 诚实边界
//!
//! - **选中态**由宿主喂（`active_theme` / `active_accent`）⇒ 本模块只渲染
//!   选中态，**不写** `<html>`（写 `<html>` 是宿主侧的事）。
//! - **二次确认弹层**不属本面 ⇒ 本模块只发「请求二次确认」的意图。

use crate::icon::{Icon, IconData, IconNode, IconProps};
use apeireth_ui_core::{Callback, UiComponent};
use apeireth_ui_dom::{AttrValue, VNode};
use apeireth_ui_macro::view;

// ── 图标数据（**生成物** —— 勿手改） ───────────────────────────────────────
//
// 生成器：`crates-ui/apeireth-ui-demo/harness/scripts/extract-assets.mjs`
//   （`--manifest` 模式 ⇒ 按面输出，避免与其它面共用同一个生成物文件）
// 来源：`frontend/companion-desktop/node_modules/lucide-svelte/dist/icons/*.svelte`
// ⚠️ 生产 import 名 ⇒ **真实 class 后缀**：`Upload` ⇒ `lucide-upload`；
//   `ImageOff` ⇒ `lucide-image-off`。
// ⚠️ `Check` 已在本 crate 的共享图标数据里（`crate::icon_data::ICON_CHECK`）
//   ⇒ 本模块**直接复用**，不重复定义。
//
// 上游许可（ISC；其中 Feather 部分为 MIT）—— 原文如下：
//
// ISC License
//
// Copyright (c) for portions of Lucide are held by Cole Bemis 2013-2022 as part of Feather (MIT). All other copyright (c) for Lucide are held by Lucide Contributors 2022.
//
// Permission to use, copy, modify, and/or distribute this software for any
// purpose with or without fee is hereby granted, provided that the above
// copyright notice and this permission notice appear in all copies.
//
// THE SOFTWARE IS PROVIDED "AS IS" AND THE AUTHOR DISCLAIMS ALL WARRANTIES
// WITH REGARD TO THIS SOFTWARE INCLUDING ALL IMPLIED WARRANTIES OF
// MERCHANTABILITY AND FITNESS. IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR
// ANY SPECIAL, DIRECT, INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES
// WHATSOEVER RESULTING FROM LOSS OF USE, DATA OR PROFITS, WHETHER IN AN
// ACTION OF CONTRACT, NEGLIGENCE OR OTHER TORTIOUS ACTION, ARISING OUT OF
// OR IN CONNECTION WITH THE USE OR PERFORMANCE OF THIS SOFTWARE.

/// 图标 `upload`（生产 import 名 `Upload`）。
#[rustfmt::skip]
pub const ICON_UPLOAD: IconData = IconData {
    name: "upload",
    class: "lucide-icon lucide lucide-upload",
    nodes: &[
        IconNode { tag: "path", attrs: &[("d", "M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4")] },
        IconNode { tag: "polyline", attrs: &[("points", "17 8 12 3 7 8")] },
        IconNode { tag: "line", attrs: &[("x1", "12"), ("x2", "12"), ("y1", "3"), ("y2", "15")] },
    ],
};

/// 图标 `image-off`（生产 import 名 `ImageOff`）。
#[rustfmt::skip]
pub const ICON_IMAGE_OFF: IconData = IconData {
    name: "image-off",
    class: "lucide-icon lucide lucide-image-off",
    nodes: &[
        IconNode { tag: "line", attrs: &[("x1", "2"), ("x2", "22"), ("y1", "2"), ("y2", "22")] },
        IconNode { tag: "path", attrs: &[("d", "M10.41 10.41a2 2 0 1 1-2.83-2.83")] },
        IconNode { tag: "line", attrs: &[("x1", "13.5"), ("x2", "6"), ("y1", "13.5"), ("y2", "21")] },
        IconNode { tag: "line", attrs: &[("x1", "18"), ("x2", "21"), ("y1", "12"), ("y2", "15")] },
        IconNode { tag: "path", attrs: &[("d", "M3.59 3.59A1.99 1.99 0 0 0 3 5v14a2 2 0 0 0 2 2h14c.55 0 1.052-.22 1.41-.59")] },
        IconNode { tag: "path", attrs: &[("d", "M21 15V5a2 2 0 0 0-2-2H9")] },
    ],
};

// ── 文案常量（**逐字**取自生产模板） ───────────────────────────────────────

/// 分区块的标题（生产 `SettingsView` 的 `appearance` 分支外壳）。
pub const BLOCK_TITLE: &str = "外观与主题";

/// 分区块的说明（同上）。
pub const BLOCK_DESC: &str = "选择界面照明档位与背景风格，立即生效并写入本地配置。";

/// 面板顶部的说明。
pub const THEME_LEDE: &str = "切换界面照明与背景。默认遗产星空为静态图；深空舰桥保留 WebGL 实时场景与金色存在纪律；Essence 使用星空山脉浅色雾面；星舰为科幻 HUD 档——深空黑底 + 发光数据（网格/扫描线/角标记），正文保持可读性优先。";

/// 自定义背景小节的标题。
pub const BG_TITLE: &str = "自定义背景";

/// 自定义背景小节的说明。
///
/// ⚠️ 生产模板里这段文字**跨两行**（换行 + 缩进**留在文本节点里**）⇒ 本常量
/// **逐字**保留那个换行与缩进（否则换行位置不同 ⇒ 像素不一致）。
pub const BG_LEDE: &str = "上传一张图片盖住主题默认背景（未选中会话时的右侧显影区与全局底）。\n      图片只存在本机浏览器数据库里——开发模式换端口或换浏览器后需要重传。";

/// 配色小节的标题。
pub const ACCENT_TITLE: &str = "UI 配色";

/// 配色小节的说明。
pub const ACCENT_LEDE: &str =
    "配色只染界面高亮（导航激活、开关这类 UI 元素）。金色是他的存在色——可选方案里没有金色系。";

/// 主题列表的无障碍名。
pub const THEME_LIST_LABEL: &str = "界面主题";

/// 配色列表的无障碍名。
pub const ACCENT_LIST_LABEL: &str = "UI 配色方案";

/// 上传按钮的文案（**自带前导空格**：与图标同属一个文本节点）。
pub const UPLOAD_LABEL: &str = " 上传图片";

/// 重新上传按钮的文案（**自带前导空格**：同上）。
pub const REUPLOAD_LABEL: &str = " 重新上传";

/// 「恢复主题默认」按钮的文案（**无**前导空格：按钮里没有图标）。
pub const USE_DEFAULT_LABEL: &str = "恢复主题默认";

/// 「清除已传图片」按钮的文案（**自带前导空格**：与图标同属一个文本节点）。
pub const CLEAR_LABEL: &str = " 清除已传图片";

/// 「清除已传图片」按钮的悬浮说明。
pub const CLEAR_TITLE: &str = "清除已传图片（需二次确认）";

/// 自定义背景预览块的无障碍名。
pub const THUMB_LABEL: &str = "当前自定义背景预览";

/// 文件输入框的接受类型（**逐字**取自生产）。
pub const FILE_ACCEPT: &str = "image/png,image/jpeg,image/webp,image/avif,image/gif";

/// 配色样品里「导航」标签的文案。
pub const SAMPLE_TAB_LABEL: &str = "导航";

/// 配色样品里「开关」标签的文案。
pub const SAMPLE_PILL_LABEL: &str = "开关";

// ── 数据形状（**中性**；宿主注入） ──────────────────────────────────────────

/// 主题条目（**中性数据形状** + 意图出口）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThemeEntry {
    /// 条目 id（宿主用它判选中态）。
    pub id: String,
    /// 名称。
    pub label: String,
    /// 说明。
    pub desc: String,
    /// 样品（**内联渐变原文** ⇒ 逐字来自固定数据 ⇒ 组件**不拼渐变**）。
    pub swatch: String,
    /// 点这张卡 ⇒ 意图出口（**宿主注入**；`None` ⇒ 点击无副作用，不假装已接线）。
    pub on_pick: Option<Callback>,
}

impl ThemeEntry {
    /// 建一条主题条目（无意图出口）。
    pub fn new(
        id: impl Into<String>,
        label: impl Into<String>,
        desc: impl Into<String>,
        swatch: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            desc: desc.into(),
            swatch: swatch.into(),
            on_pick: None,
        }
    }

    /// 接上「选中这张主题」的意图出口（builder 风格）。
    pub fn on_pick(mut self, on_pick: Callback) -> Self {
        self.on_pick = Some(on_pick);
        self
    }
}

/// 配色条目（**中性数据形状** + 意图出口）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccentEntry {
    /// 条目 id（宿主用它判选中态）。
    pub id: String,
    /// 名称。
    pub label: String,
    /// 说明。
    pub desc: String,
    /// 强调色（**内联 CSS 变量的取值** ⇒ 逐字来自固定数据）。
    pub accent: String,
    /// 强调色的前景色（同上）。
    pub ink: String,
    /// 点这张卡 ⇒ 意图出口（**宿主注入**；`None` ⇒ 点击无副作用）。
    pub on_pick: Option<Callback>,
}

impl AccentEntry {
    /// 建一条配色条目（无意图出口）。
    pub fn new(
        id: impl Into<String>,
        label: impl Into<String>,
        desc: impl Into<String>,
        accent: impl Into<String>,
        ink: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            desc: desc.into(),
            accent: accent.into(),
            ink: ink.into(),
            on_pick: None,
        }
    }

    /// 接上「选中这套配色」的意图出口（builder 风格）。
    pub fn on_pick(mut self, on_pick: Callback) -> Self {
        self.on_pick = Some(on_pick);
        self
    }
}

// ── 组件 ────────────────────────────────────────────────────────────────────

/// 主题外观面板的 props（**宿主注入** ⇒ 组件是纯函数）。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ThemeSettingsPanelProps {
    /// 主题目录（**顺序即渲染顺序**）。
    pub themes: Vec<ThemeEntry>,
    /// 配色目录（**顺序即渲染顺序**）。
    pub accents: Vec<AccentEntry>,
    /// 当前主题 id（宿主喂 ⇒ 组件只渲染选中态，**不写** `<html>`）。
    pub active_theme: String,
    /// 当前配色 id（同上）。
    pub active_accent: String,
    /// 是否已启用自定义背景（宿主喂）。
    pub custom_bg: bool,
    /// 自定义背景的预览地址（宿主喂；**固定值** ⇒ 两侧同一张图）。
    ///
    /// `None` 或空串 ⇒ 无预览（生产的「读不出就保持无预览」分支）。
    pub custom_preview_url: Option<String>,
    /// 背景操作进行中（宿主喂 ⇒ 按钮禁用）。
    pub bg_busy: bool,
    /// 背景操作的错误文案（宿主喂；空串 ⇒ 不渲染错误行）。
    pub bg_error: String,
    /// 「上传图片」按钮 ⇒ 意图出口（**宿主侧**打开文件选择）。
    pub on_request_upload: Option<Callback>,
    /// 「恢复主题默认」按钮 ⇒ 意图出口。
    pub on_use_theme_default: Option<Callback>,
    /// 「清除已传图片」按钮 ⇒ 意图出口（**宿主侧**弹二次确认）。
    pub on_request_clear: Option<Callback>,
    /// 文件输入框选中文件 ⇒ 意图出口（**宿主侧**取文件）。
    pub on_file_changed: Option<Callback>,
}

/// 主题外观面板组件。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ThemeSettingsPanel;

impl UiComponent for ThemeSettingsPanel {
    type Props = ThemeSettingsPanelProps;
    type Output = VNode;

    fn render(props: &Self::Props) -> Self::Output {
        let cards: Vec<VNode> = props
            .themes
            .iter()
            .map(|entry| render_theme_card(entry, entry.id == props.active_theme))
            .collect();
        let accent_cards: Vec<VNode> = props
            .accents
            .iter()
            .map(|entry| render_accent_card(entry, entry.id == props.active_accent))
            .collect();
        // ⚠️ `view!` 里**大写开头的标识符**被当成**组件名** ⇒ 文案常量先落到
        // 小写局部变量再用（否则宏会把常量当组件去 `builder()`）。
        let block_title = BLOCK_TITLE;
        let block_desc = BLOCK_DESC;
        let theme_lede = THEME_LEDE;
        let accent_title = ACCENT_TITLE;
        let accent_lede = ACCENT_LEDE;

        view! {
            div(class = "setting-block") {
                h3(class = "block-title") { block_title }
                " "
                p(class = "block-desc") { block_desc }
                " "
                div(class = "theme-panel") {
                    p(class = "theme-lede") { theme_lede }
                    " "
                    div(class = "theme-grid", role = "listbox", aria-label = THEME_LIST_LABEL) {
                        for card in cards {
                            { card }
                        }
                    }
                    " "
                    { render_bg_section(props) }
                    " "
                    div(class = "accent-section") {
                        h3(class = "bg-title") { accent_title }
                        " "
                        p(class = "bg-lede") { accent_lede }
                        " "
                        div(class = "accent-grid", role = "listbox", aria-label = ACCENT_LIST_LABEL) {
                            for card in accent_cards {
                                { card }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// 渲染一张主题卡。
fn render_theme_card(entry: &ThemeEntry, selected: bool) -> VNode {
    // 内联渐变：取值**逐字**来自固定数据 ⇒ 组件不拼、不裁、不重算。
    let swatch_style = format!("background: {};", entry.swatch);
    let name_text = entry.label.clone();
    let desc = entry.desc.clone();
    let selected_text = if selected { "true" } else { "false" };
    let check_icon = Icon::render(&IconProps::new(&crate::icon_data::ICON_CHECK, 14));

    view! {
        button(
            class = "theme-card",
            class:selected = selected,
            role = "option",
            aria-selected = selected_text,
            onclick = {
                let callback = entry.on_pick.clone();
                move |_| {
                    if let Some(callback) = callback.as_ref() {
                        callback.call();
                    }
                }
            }
        ) {
            div(class = "theme-swatch", style = swatch_style) { }
            " "
            div(class = "theme-meta") {
                span(class = "theme-name") { name_text }
                " "
                span(class = "theme-desc") { desc }
            }
            " "
            if selected {
                span(class = "theme-check", aria-hidden = "true") { { check_icon } }
            }
        }
    }
}

/// 渲染一张配色卡。
fn render_accent_card(entry: &AccentEntry, selected: bool) -> VNode {
    // 内联 CSS 变量覆写：两个自定义属性的取值**逐字**来自固定数据。
    let sample_style = format!(
        "--ap-accent-ui: {}; --ap-accent-ui-ink: {};",
        entry.accent, entry.ink
    );
    let name_text = entry.label.clone();
    let desc = entry.desc.clone();
    let selected_text = if selected { "true" } else { "false" };
    let check_icon = Icon::render(&IconProps::new(&crate::icon_data::ICON_CHECK, 12));
    // ⚠️ `view!` 里**大写开头的标识符**被当成组件名 ⇒ 文案常量落到小写局部变量。
    let sample_tab_label = SAMPLE_TAB_LABEL;
    let sample_pill_label = SAMPLE_PILL_LABEL;

    view! {
        button(
            class = "accent-card",
            class:selected = selected,
            role = "option",
            aria-selected = selected_text,
            onclick = {
                let callback = entry.on_pick.clone();
                move |_| {
                    if let Some(callback) = callback.as_ref() {
                        callback.call();
                    }
                }
            }
        ) {
            span(class = "accent-sample", aria-hidden = "true", style = sample_style) {
                span(class = "sample-tab") { sample_tab_label }
                " "
                span(class = "sample-pill") { sample_pill_label }
            }
            " "
            span(class = "accent-meta") {
                span(class = "accent-name") { name_text }
                " "
                span(class = "accent-desc") { desc }
            }
            " "
            if selected {
                span(class = "accent-check", aria-hidden = "true") { { check_icon } }
            }
        }
    }
}

/// 渲染「自定义背景」小节。
fn render_bg_section(props: &ThemeSettingsPanelProps) -> VNode {
    let busy = props.bg_busy;
    // 忙碌态：按钮禁用。生产用 `disabled={bgBusy}`（**属性由浏览器反射**）⇒
    // 忙碌时属性值为**空串**（不是 `"true"`）。
    let disabled = if busy {
        AttrValue::Text(String::new())
    } else {
        AttrValue::Bool(false)
    };
    let upload_label = if props.custom_bg {
        REUPLOAD_LABEL
    } else {
        UPLOAD_LABEL
    };
    let upload_icon = Icon::render(&IconProps::new(&ICON_UPLOAD, 13));
    let clear_icon = Icon::render(&IconProps::new(&ICON_IMAGE_OFF, 13));
    let error_text = props.bg_error.clone();
    let has_error = !error_text.is_empty();
    // 预览地址：空串 ⇒ 无预览（生产的「读不出就保持无预览」分支）。
    let preview = props
        .custom_preview_url
        .as_deref()
        .filter(|url| !url.is_empty())
        .map(|url| format!("background-image: url({url});"));
    let show_preview = props.custom_bg && preview.is_some();
    // ⚠️ `view!` 里**大写开头的标识符**被当成组件名 ⇒ 文案常量落到小写局部变量。
    let bg_title = BG_TITLE;
    let bg_lede = BG_LEDE;
    let use_default_label = USE_DEFAULT_LABEL;
    let clear_label = CLEAR_LABEL;

    view! {
        div(class = "bg-section") {
            h3(class = "bg-title") { bg_title }
            " "
            p(class = "bg-lede") { bg_lede }
            " "
            div(class = "bg-row") {
                if show_preview {
                    div(
                        class = "bg-thumb",
                        role = "img",
                        aria-label = THUMB_LABEL,
                        style = preview.unwrap_or_default()
                    ) { }
                }
                " "
                div(class = "bg-actions") {
                    button(
                        class = "quiet-btn",
                        disabled = disabled.clone(),
                        onclick = {
                            let callback = props.on_request_upload.clone();
                            move |_| {
                                if let Some(callback) = callback.as_ref() {
                                    callback.call();
                                }
                            }
                        }
                    ) {
                        { upload_icon }
                        upload_label
                    }
                    " "
                    if props.custom_bg {
                        button(
                            class = "quiet-btn",
                            disabled = disabled.clone(),
                            onclick = {
                                let callback = props.on_use_theme_default.clone();
                                move |_| {
                                    if let Some(callback) = callback.as_ref() {
                                        callback.call();
                                    }
                                }
                            }
                        ) {
                            use_default_label
                        }
                        " "
                        button(
                            class = "quiet-btn",
                            disabled = disabled.clone(),
                            title = CLEAR_TITLE,
                            onclick = {
                                let callback = props.on_request_clear.clone();
                                move |_| {
                                    if let Some(callback) = callback.as_ref() {
                                        callback.call();
                                    }
                                }
                            }
                        ) {
                            { clear_icon }
                            clear_label
                        }
                    }
                }
            }
            " "
            if has_error {
                p(class = "bg-error", role = "alert") { error_text }
            }
            " "
            input(
                type = "file",
                accept = FILE_ACCEPT,
                class = "bg-file",
                onchange = {
                    let callback = props.on_file_changed.clone();
                    move |_| {
                        if let Some(callback) = callback.as_ref() {
                            callback.call();
                        }
                    }
                }
            )
        }
    }
}

#[cfg(test)]
mod tests {
    //! 主题外观面板的**行为**断言（渲染结果 ⇒ 结构 / class / 文本 / 内联样式），
    //! 不读源码（设计文档 §10.2 的「钉行为」口径）。

    use super::*;
    use apeireth_ui_core::Runtime;

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
        for candidate in node.class_tokens() {
            if candidate == token {
                count += 1;
            }
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

    /// 收集全部属性（`标签@属性` ⇒ 值）。
    fn attributes(node: &VNode, out: &mut Vec<(String, String)>) {
        if let VNode::Element(element) = node {
            for (name, value) in &element.attrs {
                if let Some(text) = value.to_attr_string() {
                    out.push((format!("{}@{name}", element.tag), text));
                }
            }
        }
        for child in node.children() {
            attributes(child, out);
        }
    }

    /// 一条最小主题条目。
    fn theme(id: &str) -> ThemeEntry {
        ThemeEntry::new(
            id,
            "原初",
            "默认 · 某某风",
            "linear-gradient(180deg, #ffffff 0%, #f2f3f5 100%)",
        )
    }

    /// 一条最小配色条目。
    fn accent(id: &str) -> AccentEntry {
        AccentEntry::new(id, "存在金（默认）", "UI 高亮", "#ffd27a", "#1b1409")
    }

    /// 一组最小 props。
    fn props() -> ThemeSettingsPanelProps {
        ThemeSettingsPanelProps {
            themes: vec![theme("origin"), theme("noir")],
            accents: vec![accent("presence-gold"), accent("deep-space")],
            active_theme: String::from("origin"),
            active_accent: String::from("presence-gold"),
            custom_bg: false,
            custom_preview_url: None,
            bg_busy: false,
            bg_error: String::new(),
            on_request_upload: None,
            on_use_theme_default: None,
            on_request_clear: None,
            on_file_changed: None,
        }
    }

    #[test]
    fn the_panel_renders_the_production_class_names() {
        let node = ThemeSettingsPanel::render(&props());
        let mut found = Vec::new();
        classes(&node, &mut found);
        for class in [
            "setting-block",
            "block-title",
            "block-desc",
            "theme-panel",
            "theme-lede",
            "theme-grid",
            "theme-card",
            "theme-swatch",
            "theme-meta",
            "theme-name",
            "theme-desc",
            "theme-check",
            "bg-section",
            "bg-title",
            "bg-lede",
            "bg-row",
            "bg-actions",
            "quiet-btn",
            "bg-file",
            "accent-section",
            "accent-grid",
            "accent-card",
            "accent-sample",
            "sample-tab",
            "sample-pill",
            "accent-meta",
            "accent-name",
            "accent-desc",
            "accent-check",
        ] {
            assert!(
                found.iter().any(|token| token == class),
                "class `{class}` 未出现在渲染结果里：{found:?}"
            );
        }
    }

    #[test]
    fn the_panel_renders_the_production_text() {
        let node = ThemeSettingsPanel::render(&props());
        let mut texts = Vec::new();
        text_nodes(&node, &mut texts);
        for text in [
            BLOCK_TITLE,
            BLOCK_DESC,
            THEME_LEDE,
            BG_TITLE,
            BG_LEDE,
            ACCENT_TITLE,
            ACCENT_LEDE,
            SAMPLE_TAB_LABEL,
            SAMPLE_PILL_LABEL,
            UPLOAD_LABEL,
        ] {
            assert!(
                texts.iter().any(|value| value == text),
                "文本 {text:?} 未出现在渲染结果里：{texts:?}"
            );
        }
    }

    #[test]
    fn the_upload_label_carries_the_leading_space_of_the_production_text_node() {
        let node = ThemeSettingsPanel::render(&props());
        let mut texts = Vec::new();
        text_nodes(&node, &mut texts);
        // 图标与文字之间的换行折叠成**一个空格**，且与文字**同属一个文本节点**。
        assert!(
            texts.iter().any(|value| value == " 上传图片"),
            "上传按钮的文本节点：{texts:?}"
        );
        // 不允许把它拆成「空格」+「文字」两个节点。
        assert!(
            !texts.iter().any(|value| value == "上传图片"),
            "上传按钮的文字被拆成了独立节点：{texts:?}"
        );
    }

    #[test]
    fn the_swatch_inline_style_keeps_the_gradient_verbatim() {
        let node = ThemeSettingsPanel::render(&props());
        let mut attrs = Vec::new();
        attributes(&node, &mut attrs);
        let styles: Vec<&String> = attrs
            .iter()
            .filter(|(name, _)| name == "div@style")
            .map(|(_, value)| value)
            .collect();
        assert!(
            styles.contains(&&String::from(
                "background: linear-gradient(180deg, #ffffff 0%, #f2f3f5 100%);"
            )),
            "内联渐变必须逐字取自固定数据：{styles:?}"
        );
    }

    #[test]
    fn the_accent_sample_overrides_the_css_variables_inline() {
        let node = ThemeSettingsPanel::render(&props());
        let mut attrs = Vec::new();
        attributes(&node, &mut attrs);
        let styles: Vec<&String> = attrs
            .iter()
            .filter(|(name, _)| name == "span@style")
            .map(|(_, value)| value)
            .collect();
        assert!(
            styles.contains(&&String::from(
                "--ap-accent-ui: #ffd27a; --ap-accent-ui-ink: #1b1409;"
            )),
            "配色样品的 CSS 变量覆写必须逐字：{styles:?}"
        );
    }

    #[test]
    fn only_the_active_entries_are_marked_selected() {
        let node = ThemeSettingsPanel::render(&props());
        let mut attrs = Vec::new();
        attributes(&node, &mut attrs);
        let selected: Vec<&String> = attrs
            .iter()
            .filter(|(name, _)| name == "button@aria-selected")
            .map(|(_, value)| value)
            .collect();
        // 两张主题卡 + 两张配色卡 ⇒ 四个 `aria-selected`。
        assert_eq!(selected.len(), 4, "{selected:?}");
        assert_eq!(
            selected
                .iter()
                .filter(|value| value.as_str() == "true")
                .count(),
            2,
            "一张主题卡 + 一张配色卡被选中：{selected:?}"
        );
    }

    #[test]
    fn the_selected_card_shows_the_check_mark() {
        let node = ThemeSettingsPanel::render(&props());
        let mut found = Vec::new();
        classes(&node, &mut found);
        assert!(found.iter().any(|token| token == "theme-check"));
        assert!(found.iter().any(|token| token == "accent-check"));
    }

    #[test]
    fn the_check_mark_follows_the_active_entry() {
        let mut props = props();
        props.active_theme = String::from("noir");
        props.active_accent = String::from("deep-space");
        let node = ThemeSettingsPanel::render(&props);
        // 选中态换了 ⇒ 仍然各有一个勾（生产的结构是「按选中渲染」）。
        assert_eq!(count_class(&node, "theme-check"), 1);
        assert_eq!(count_class(&node, "accent-check"), 1);
    }

    #[test]
    fn the_panel_renders_one_card_per_entry() {
        let node = ThemeSettingsPanel::render(&props());
        assert_eq!(count_class(&node, "theme-card"), 2);
        assert_eq!(count_class(&node, "accent-card"), 2);
    }

    #[test]
    fn the_background_row_has_no_clear_button_without_a_custom_background() {
        let node = ThemeSettingsPanel::render(&props());
        let mut texts = Vec::new();
        text_nodes(&node, &mut texts);
        assert!(
            !texts.iter().any(|text| text == CLEAR_LABEL),
            "没有自定义背景 ⇒ 不出现「清除已传图片」：{texts:?}"
        );
        assert!(
            !texts.iter().any(|text| text == USE_DEFAULT_LABEL),
            "没有自定义背景 ⇒ 不出现「恢复主题默认」：{texts:?}"
        );
    }

    #[test]
    fn the_background_row_shows_the_clear_buttons_with_a_custom_background() {
        let mut props = props();
        props.custom_bg = true;
        let node = ThemeSettingsPanel::render(&props);
        let mut texts = Vec::new();
        text_nodes(&node, &mut texts);
        assert!(
            texts.iter().any(|text| text == USE_DEFAULT_LABEL),
            "{texts:?}"
        );
        assert!(texts.iter().any(|text| text == CLEAR_LABEL), "{texts:?}");
        assert!(texts.iter().any(|text| text == REUPLOAD_LABEL), "{texts:?}");
    }

    #[test]
    fn the_custom_background_preview_appears_only_with_a_preview_url() {
        let mut props = props();
        props.custom_bg = true;
        props.custom_preview_url = Some(String::from("data:image/png;base64,AAAA"));
        let node = ThemeSettingsPanel::render(&props);
        let mut found = Vec::new();
        classes(&node, &mut found);
        assert!(found.iter().any(|token| token == "bg-thumb"), "{found:?}");
        let mut attrs = Vec::new();
        attributes(&node, &mut attrs);
        assert!(
            attrs.iter().any(|(name, value)| name == "div@style"
                && value == "background-image: url(data:image/png;base64,AAAA);"),
            "{attrs:?}"
        );
    }

    #[test]
    fn the_custom_background_preview_is_absent_without_a_preview_url() {
        let mut props = props();
        props.custom_bg = true;
        props.custom_preview_url = Some(String::new());
        let node = ThemeSettingsPanel::render(&props);
        let mut found = Vec::new();
        classes(&node, &mut found);
        assert!(!found.iter().any(|token| token == "bg-thumb"), "{found:?}");
    }

    #[test]
    fn the_error_row_appears_only_with_an_error_message() {
        let mut props = props();
        props.bg_error = String::from("写入本机浏览器数据库失败");
        let node = ThemeSettingsPanel::render(&props);
        let mut found = Vec::new();
        classes(&node, &mut found);
        assert!(found.iter().any(|token| token == "bg-error"), "{found:?}");
        let mut texts = Vec::new();
        text_nodes(&node, &mut texts);
        assert!(
            texts.iter().any(|text| text == "写入本机浏览器数据库失败"),
            "{texts:?}"
        );
    }

    #[test]
    fn the_error_row_is_absent_without_an_error_message() {
        let node = ThemeSettingsPanel::render(&props());
        let mut found = Vec::new();
        classes(&node, &mut found);
        assert!(!found.iter().any(|token| token == "bg-error"), "{found:?}");
    }

    #[test]
    fn the_busy_state_disables_the_background_buttons() {
        let mut props = props();
        props.bg_busy = true;
        let node = ThemeSettingsPanel::render(&props);
        let mut attrs = Vec::new();
        attributes(&node, &mut attrs);
        let disabled: Vec<&String> = attrs
            .iter()
            .filter(|(name, _)| name == "button@disabled")
            .map(|(_, value)| value)
            .collect();
        assert_eq!(disabled.len(), 1, "{disabled:?}");
        // 生产用 `disabled={bgBusy}` ⇒ 忙碌时属性反射成**空串**（不是 `"true"`）。
        assert_eq!(disabled[0], "", "{disabled:?}");
    }

    #[test]
    fn the_idle_state_has_no_disabled_attribute() {
        let node = ThemeSettingsPanel::render(&props());
        let mut attrs = Vec::new();
        attributes(&node, &mut attrs);
        assert!(
            !attrs.iter().any(|(name, _)| name == "button@disabled"),
            "空闲态 ⇒ 不出现 disabled 属性：{attrs:?}"
        );
    }

    #[test]
    fn the_file_input_keeps_the_production_attributes() {
        let node = ThemeSettingsPanel::render(&props());
        let mut attrs = Vec::new();
        attributes(&node, &mut attrs);
        assert!(
            attrs
                .iter()
                .any(|(name, value)| name == "input@type" && value == "file"),
            "{attrs:?}"
        );
        assert!(
            attrs
                .iter()
                .any(|(name, value)| name == "input@accept" && value == FILE_ACCEPT),
            "{attrs:?}"
        );
    }

    #[test]
    fn the_picking_intent_reaches_the_host() {
        let runtime = Runtime::new();
        let picked = runtime.signal(String::new());
        let sink = picked.clone();
        let entry = theme("origin").on_pick(Callback::new(move || {
            sink.set(String::from("origin"));
        }));
        let mut props = props();
        props.themes = vec![entry];
        let node = ThemeSettingsPanel::render(&props);

        // 组件是纯函数：渲染本身不改状态。
        assert_eq!(picked.get(), "");

        // 点一下（调用挂在卡上的处理器）⇒ 意图到达宿主。
        fn first_handler(node: &VNode) -> Option<apeireth_ui_dom::ElementHandler> {
            if let VNode::Element(element) = node {
                if node
                    .class_tokens()
                    .iter()
                    .any(|token| token == "theme-card")
                {
                    return element.handlers.first().map(|(_, handler)| handler.clone());
                }
            }
            for child in node.children() {
                if let Some(handler) = first_handler(child) {
                    return Some(handler);
                }
            }
            None
        }
        let handler = first_handler(&node).expect("主题卡上挂了处理器");
        let mut event =
            apeireth_ui_dom::DomEvent::new(apeireth_ui_dom::EventKind::Click, Vec::new());
        handler.call(&mut event);
        assert_eq!(picked.get(), "origin");
    }

    #[test]
    fn the_component_is_a_pure_function_of_its_props() {
        // ⚠️ 处理器按**指针身份**比较（`ui-dom` 的既定口径）⇒ 两次渲染的
        // `PartialEq` **必然不等** ⇒ 这里比**画面口径**：class / 文本 / 属性。
        let plain = props();
        let first = ThemeSettingsPanel::render(&plain);
        let second = ThemeSettingsPanel::render(&plain);
        let mut first_classes = Vec::new();
        let mut second_classes = Vec::new();
        classes(&first, &mut first_classes);
        classes(&second, &mut second_classes);
        assert_eq!(first_classes, second_classes, "class 集合相同");
        let mut first_texts = Vec::new();
        let mut second_texts = Vec::new();
        text_nodes(&first, &mut first_texts);
        text_nodes(&second, &mut second_texts);
        assert_eq!(first_texts, second_texts, "文本节点逐字相同");
        let mut first_attrs = Vec::new();
        let mut second_attrs = Vec::new();
        attributes(&first, &mut first_attrs);
        attributes(&second, &mut second_attrs);
        assert_eq!(first_attrs, second_attrs, "属性逐字相同");
    }

    #[test]
    fn two_renders_with_handlers_have_the_same_shape() {
        let build = || {
            let mut props = props();
            props.themes = vec![theme("origin").on_pick(Callback::new(|| {})), theme("noir")];
            ThemeSettingsPanel::render(&props)
        };
        let first = build();
        let second = build();
        let mut first_classes = Vec::new();
        let mut second_classes = Vec::new();
        classes(&first, &mut first_classes);
        classes(&second, &mut second_classes);
        assert_eq!(first_classes, second_classes, "class 集合相同");
        let mut first_texts = Vec::new();
        let mut second_texts = Vec::new();
        text_nodes(&first, &mut first_texts);
        text_nodes(&second, &mut second_texts);
        assert_eq!(first_texts, second_texts, "文本节点逐字相同");
    }
}
