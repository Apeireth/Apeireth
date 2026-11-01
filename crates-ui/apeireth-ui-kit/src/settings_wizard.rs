//! 首次运行向导（设置页 · **W1 面**）。
//!
//! ## 边界（与生产的 `FirstRunWizard.svelte` 一一对应）
//!
//! | 项 | 生产 | 本组件 |
//! |---|---|---|
//! | 根节点 | `<div class="wizard-backdrop">`（`position: fixed` ⇒ 整视口） | 同（class 名逐字一致） |
//! | 步骤条 | `<ol class="step-dots">` + `{#if i < stepIndex}` 的**结构分支** | 同（`Check` 图标 ⇄ 序号文本） |
//! | 步骤体 | 六个互斥分支（`welcome` / `tier` / `api` / `identity` / `preset` / `pro-guide`） | 同（**结构分支**，不是 class 分支） |
//! | 错误行 | `{#if error}` ⇒ `<p class="wizard-error">` | 同（空串 ⇒ **不渲染**） |
//! | 页脚 | 五种互斥的按钮形态（见 [`FooterKind`]） | 同 |
//!
//! ## 「引擎不迁」的那一半（**只发意图**）
//!
//! 生产在向导里还会做三件**宿主侧**的事：把密钥写进系统钥匙串、把自我描述词
//! 写进用户印象、按分流合并出最终配置。⇒ 本组件**只渲染 + 只发意图**：
//! 十个本地状态（分流 / 步序 / 预设 / 端点 / 模型 / 密钥 / 描述词 / 自动调参 /
//! 应用中 / 错误）由**调用方注入**，完成与跳过是**意图出口**（[`Callback`]）。
//!
//! ## 空白（照抄生产的折叠规则）
//!
//! 生产模板里**相邻元素之间**的换行会折叠成**恰好一个空格**的文本节点；本组件
//! 只在**同一个元素内**产出生产**逐字**需要的空白：
//!
//! | 位置 | 生产 | 本组件 |
//! |---|---|---|
//! | 「上一步」 | ` 上一步`（**前导**空格） | 同 |
//! | 「下一步 」 | `下一步 `（**尾随**空格） | 同 |
//! | 其它相邻元素 | 折叠出一个空格 | **不产出**（元素间的空白不可见 ⇒ 不影响画面） |
//!
//! ## 图标（**真实 class 后缀**，不是生产 import 名）
//!
//! | 生产 import | 真实 class |
//! |---|---|
//! | `Sparkles` | `lucide-sparkles` |
//! | `ChevronLeft` | `lucide-chevron-left` |
//! | `ChevronRight` | `lucide-chevron-right` |
//! | `Check` | `lucide-check` |
//!
//! 几何数据来自上游图标包（**抽取产物** ⇒ 见 [`icons`]）。

use apeireth_ui_core::{Callback, UiComponent};
use apeireth_ui_dom::{AttrValue, VNode};
use apeireth_ui_macro::view;

use crate::icon::{Icon, IconProps};
use crate::icon_data::{ICON_CHECK, ICON_CHEVRON_RIGHT};

/// 本面的图标数据（**抽取产物** —— 上游图标包里的同名图标文件）。
///
/// ## 为什么单独一个模块
///
/// 图标几何的**共享**真源是 [`crate::icon_data`]（批一抽取器产物）；本面需要的
/// `sparkles` / `chevron-left` 两枚**不在**那份清单里 ⇒ 本面自带一份（**同一份
/// 上游源头**，同一套许可）。
///
/// - 来源：`frontend/companion-desktop/node_modules/lucide-svelte/dist/icons/
///   {sparkles,chevron-left}.svelte`；
/// - 许可：ISC（其中 Feather 部分为 MIT）—— 原文见 [`crate::icon_data`] 的头部。
pub mod icons {
    use crate::icon::{IconData, IconNode};

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

    /// 图标 `chevron-left`。
    #[rustfmt::skip]
    pub const ICON_CHEVRON_LEFT: IconData = IconData {
        name: "chevron-left",
        class: "lucide-icon lucide lucide-chevron-left",
        nodes: &[
            IconNode { tag: "path", attrs: &[("d", "m15 18-6-6 6-6")] },
        ],
    };
}

/// 向导的分流（**两条轨道**：普通 / 专业）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WizardTier {
    /// 普通用户轨道（写预设值）。
    Casual,
    /// 专业用户轨道（只做介绍）。
    Pro,
}

impl WizardTier {
    /// 分流 id（**与固定数据的 id 逐字一致**）。
    pub fn id(self) -> &'static str {
        match self {
            WizardTier::Casual => "casual",
            WizardTier::Pro => "pro",
        }
    }

    /// 按 id 取分流（未知 id ⇒ `None`）。
    pub fn from_id(id: &str) -> Option<Self> {
        match id {
            "casual" => Some(WizardTier::Casual),
            "pro" => Some(WizardTier::Pro),
            _ => None,
        }
    }
}

/// 向导的一个步骤（**结构名**；顺序由调用方的步序表决定）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WizardStep {
    /// 欢迎 / 介绍。
    Welcome,
    /// 分流。
    Tier,
    /// 连接模型服务。
    Api,
    /// 自我描述词。
    Identity,
    /// 普通用户预设值。
    Preset,
    /// 专业用户参考。
    ProGuide,
}

impl WizardStep {
    /// 步骤 id（**与固定数据的 id 逐字一致**）。
    pub fn id(self) -> &'static str {
        match self {
            WizardStep::Welcome => "welcome",
            WizardStep::Tier => "tier",
            WizardStep::Api => "api",
            WizardStep::Identity => "identity",
            WizardStep::Preset => "preset",
            WizardStep::ProGuide => "pro-guide",
        }
    }

    /// 按 id 取步骤（未知 id ⇒ `None`）。
    pub fn from_id(id: &str) -> Option<Self> {
        match id {
            "welcome" => Some(WizardStep::Welcome),
            "tier" => Some(WizardStep::Tier),
            "api" => Some(WizardStep::Api),
            "identity" => Some(WizardStep::Identity),
            "preset" => Some(WizardStep::Preset),
            "pro-guide" => Some(WizardStep::ProGuide),
            _ => None,
        }
    }
}

/// 一个步骤的标题（主标题 + 副标题）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WizardStepTitle {
    /// 步骤。
    pub step: WizardStep,
    /// 主标题。
    pub title: String,
    /// 副标题。
    pub sub: String,
}

/// 一个预设项（**展示口径**；不含任何凭据）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WizardPreset {
    /// 预设 id。
    pub id: String,
    /// 预设名。
    pub name: String,
    /// 端点。
    pub base_url: String,
    /// 模型名。
    pub model: String,
    /// 提示（**空串 ⇒ 不渲染** `<small>` ⇒ 与生产同构的结构分支）。
    pub hint: String,
}

/// 页脚的按钮形态（**互斥**；照抄生产的分支链）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FooterKind {
    /// 欢迎步：跳过 + 开始。
    Start,
    /// 分流步且**未选**分流：只有「下一步」，且**禁用**。
    Blocked,
    /// 仍有下一步。
    Next,
    /// 专业用户参考步：完成（不写任何预设）。
    ProDone,
    /// 普通用户收尾：完成。
    Finish,
}

/// 向导组件的 props。
///
/// ## 纪律
///
/// - **状态归调用方**：本组件是**纯函数**（`Props` ⇒ `VNode`），十个本地状态
///   全部由调用方注入；
/// - **意图出口**：完成 / 跳过 / 下一步 / 上一步 / 选预设 / 选分流 / 改输入
///   都是 [`Callback`]（**无参** ⇒ 调用方自己知道是哪个缝）。
#[derive(Clone, Debug)]
pub struct FirstRunWizardProps {
    /// 分流（`None` ⇒ 未选；此时步序按**普通用户轨道**预览，与生产一致）。
    pub tier: Option<WizardTier>,
    /// 当前步序下标（**越界会被夹到末步**，与生产的取小一致）。
    pub step_index: usize,
    /// 当前轨道的步序表（**调用方按分流选好**）。
    pub steps: Vec<WizardStep>,
    /// 步骤标题表（六个步骤各一条）。
    pub step_titles: Vec<WizardStepTitle>,
    /// 预设表（**展示顺序即渲染顺序**）。
    pub presets: Vec<WizardPreset>,
    /// 欢迎步的要点。
    pub intro_bullets: Vec<String>,
    /// 欢迎步的预告。
    pub intro_plan: Vec<String>,
    /// 专业用户参考步的条目。
    pub pro_guide_lines: Vec<String>,
    /// 当前预设 id。
    pub preset_id: String,
    /// 端点（**受控输入**的当前值）。
    pub base_url: String,
    /// 模型名（**受控输入**的当前值）。
    pub model: String,
    /// 密钥（**受控输入**的当前值；**组件不渲染它** ⇒ 只进 DOM property）。
    pub api_key: String,
    /// 自我描述词（**受控输入**的当前值）。
    pub self_description: String,
    /// 「使用中自动调整参数」。
    pub auto_tune: bool,
    /// 应用中（**禁用页脚按钮**）。
    pub applying: bool,
    /// 错误文案（**空串 ⇒ 不渲染**错误行）。
    pub error: String,
    /// 「下一步」意图。
    pub on_next: Callback,
    /// 「上一步」意图。
    pub on_back: Callback,
    /// 「以后再说」意图（**跳过**）。
    pub on_skip: Callback,
    /// 「完成」意图（**宿主**负责写配置 / 钥匙串 / 印象）。
    pub on_complete: Callback,
    /// 选「普通用户」意图。
    pub on_pick_casual: Callback,
    /// 选「专业用户」意图。
    pub on_pick_pro: Callback,
    /// 每个预设一项的意图（**与 [`FirstRunWizardProps::presets`] 同序**）。
    pub preset_intents: Vec<Callback>,
    /// 端点输入变更意图（**宿主**从 DOM 取值）。
    pub on_set_base_url: Callback,
    /// 模型输入变更意图。
    pub on_set_model: Callback,
    /// 密钥输入变更意图。
    pub on_set_api_key: Callback,
    /// 自我描述词变更意图。
    pub on_set_self_description: Callback,
    /// 自动调参开关变更意图。
    pub on_toggle_auto_tune: Callback,
}

impl Default for FirstRunWizardProps {
    fn default() -> Self {
        Self {
            tier: None,
            step_index: 0,
            steps: Vec::new(),
            step_titles: Vec::new(),
            presets: Vec::new(),
            intro_bullets: Vec::new(),
            intro_plan: Vec::new(),
            pro_guide_lines: Vec::new(),
            preset_id: String::new(),
            base_url: String::new(),
            model: String::new(),
            api_key: String::new(),
            self_description: String::new(),
            auto_tune: true,
            applying: false,
            error: String::new(),
            on_next: Callback::new(|| {}),
            on_back: Callback::new(|| {}),
            on_skip: Callback::new(|| {}),
            on_complete: Callback::new(|| {}),
            on_pick_casual: Callback::new(|| {}),
            on_pick_pro: Callback::new(|| {}),
            preset_intents: Vec::new(),
            on_set_base_url: Callback::new(|| {}),
            on_set_model: Callback::new(|| {}),
            on_set_api_key: Callback::new(|| {}),
            on_set_self_description: Callback::new(|| {}),
            on_toggle_auto_tune: Callback::new(|| {}),
        }
    }
}

impl FirstRunWizardProps {
    /// 当前步骤（**越界夹到末步**；空步序 ⇒ 欢迎步）。
    pub fn current_step(&self) -> WizardStep {
        let last = self.steps.len().saturating_sub(1);
        self.steps
            .get(self.step_index.min(last))
            .copied()
            .unwrap_or(WizardStep::Welcome)
    }

    /// 步骤标题（**缺项 ⇒ 空串**，不静默编造）。
    pub fn title_of(&self, step: WizardStep) -> (&str, &str) {
        self.step_titles
            .iter()
            .find(|item| item.step == step)
            .map(|item| (item.title.as_str(), item.sub.as_str()))
            .unwrap_or(("", ""))
    }

    /// 页脚形态（**照抄生产的分支链**）。
    ///
    /// | 生产 | 本方法 |
    /// |---|---|
    /// | 分流步且未选分流 | [`FooterKind::Blocked`] |
    /// | 还有下一步 | [`FooterKind::Next`] |
    /// | 专业用户参考步 | [`FooterKind::ProDone`] |
    /// | 其余 | [`FooterKind::Finish`] |
    ///
    /// （欢迎步由调用方先判 ⇒ [`FooterKind::Start`]；生产里欢迎步是页脚的**第一
    /// 个分支**，与其它形态互斥。）
    pub fn footer_kind(&self) -> FooterKind {
        if self.current_step() == WizardStep::Welcome {
            return FooterKind::Start;
        }
        if self.current_step() == WizardStep::Tier && self.tier.is_none() {
            return FooterKind::Blocked;
        }
        if self.step_index < self.steps.len().saturating_sub(1) {
            return FooterKind::Next;
        }
        if self.current_step() == WizardStep::ProGuide {
            return FooterKind::ProDone;
        }
        FooterKind::Finish
    }

    /// 当前预设（**缺项 ⇒ `None`**）。
    pub fn current_preset(&self) -> Option<&WizardPreset> {
        self.presets
            .iter()
            .find(|preset| preset.id == self.preset_id)
    }
}

/// 首次运行向导组件。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FirstRunWizard;

impl UiComponent for FirstRunWizard {
    type Props = FirstRunWizardProps;
    type Output = VNode;

    fn render(props: &Self::Props) -> Self::Output {
        let step = props.current_step();
        let (title, sub) = props.title_of(step);
        let title = title.to_owned();
        let sub_text = sub.to_owned();

        // 步骤条：序号文本 + 两个互斥的 class 标记（**结构分支**：已完成 ⇒ 图标）。
        let dots: Vec<(String, bool, bool)> = props
            .steps
            .iter()
            .enumerate()
            .map(|(index, _)| {
                (
                    (index + 1).to_string(),
                    index == props.step_index,
                    index < props.step_index,
                )
            })
            .collect();

        let sparkles = Icon::render(&IconProps::new(&icons::ICON_SPARKLES, 16));
        let check = Icon::render(&IconProps::new(&ICON_CHECK, 11));

        let body = render_step_body(props, step);
        let footer = render_footer(props);

        let error = props.error.clone();
        let has_error = !error.is_empty();

        view! {
            div(class = "wizard-backdrop") {
                div(class = "wizard-card") {
                    header(class = "wizard-head") {
                        span(class = "wizard-icon") {
                            { sparkles }
                        }
                        div {
                            h2 { title }
                            p { sub_text }
                        }
                    }
                    ol(class = "step-dots", aria-label = "引导进度") {
                        for (number_text, is_active, is_done) in dots {
                            li(class:active = is_active, class:done = is_done) {
                                if is_done {
                                    { check.clone() }
                                } else {
                                    number_text
                                }
                            }
                        }
                    }
                    { body }
                    if has_error {
                        p(class = "wizard-error") { error }
                    }
                    { footer }
                }
            }
        }
    }
}

/// 渲染当前步骤的**步骤体**（六个互斥分支 ⇒ 结构分支）。
fn render_step_body(props: &FirstRunWizardProps, step: WizardStep) -> VNode {
    match step {
        WizardStep::Welcome => {
            let bullets = props.intro_bullets.clone();
            let plan = props.intro_plan.clone();
            view! {
                section(class = "step-body") {
                    p(class = "lead") {
                        "Apeireth 是一个本地优先的 AI 伙伴运行时——所有对话、审批、审计、记忆都留在你自己的机器上。"
                    }
                    ul(class = "intro-list") {
                        for line in bullets {
                            li { line }
                        }
                    }
                    div(class = "plan-box") {
                        for line in plan {
                            p { line }
                        }
                    }
                }
            }
        }
        WizardStep::Tier => {
            let casual_selected = props.tier == Some(WizardTier::Casual);
            let pro_selected = props.tier == Some(WizardTier::Pro);
            let on_casual = props.on_pick_casual.clone();
            let on_pro = props.on_pick_pro.clone();
            view! {
                section(class = "step-body") {
                    div(class = "tier-cards") {
                        button(
                            class = "tier-card",
                            class:selected = casual_selected,
                            onclick = move |_| { on_casual.call(); }
                        ) {
                            h3 {
                                "普通用户"
                                span(class = "tier-tag") { "推荐" }
                            }
                            p {
                                "引导为你写好预设值（记忆核心族、预算族），使用中"
                                strong { "自动调整参数" }
                                "——上手就用，细节随时可改。"
                            }
                        }
                        button(
                            class = "tier-card",
                            class:selected = pro_selected,
                            onclick = move |_| { on_pro.call(); }
                        ) {
                            h3 { "专业用户" }
                            p {
                                strong { "只做介绍" }
                                "：给你完整配置参考，不写任何预设——一切旋钮由你自己决定。"
                            }
                        }
                    }
                }
            }
        }
        WizardStep::Api => {
            let preset_id = props.preset_id.clone();
            let chips: Vec<(String, String, bool, Option<Callback>)> = props
                .presets
                .iter()
                .enumerate()
                .map(|(index, preset)| {
                    (
                        preset.name.clone(),
                        preset.hint.clone(),
                        preset.id == preset_id,
                        props.preset_intents.get(index).cloned(),
                    )
                })
                .collect();
            let show_key = preset_id != "ollama";
            let on_set_base_url = props.on_set_base_url.clone();
            let on_set_model = props.on_set_model.clone();
            let on_set_api_key = props.on_set_api_key.clone();
            view! {
                section(class = "step-body") {
                    div(class = "preset-row") {
                        for (name, hint, selected, intent) in chips {
                            button(
                                class = "preset-chip",
                                class:selected = selected,
                                onclick = move |_| {
                                    if let Some(intent) = intent.as_ref() {
                                        intent.call();
                                    }
                                }
                            ) {
                                span { name }
                                if !hint.is_empty() {
                                    small { hint }
                                }
                            }
                        }
                    }
                    div(class = "field") {
                        label(for = "wizard-url") { "API 端点" }
                        input(
                            id = "wizard-url",
                            type = "text",
                            placeholder = "https://api.deepseek.com/v1",
                            oninput = move |_| { on_set_base_url.call(); }
                        )
                    }
                    div(class = "field") {
                        label(for = "wizard-model") { "模型" }
                        input(
                            id = "wizard-model",
                            type = "text",
                            placeholder = "deepseek-v4-flash",
                            oninput = move |_| { on_set_model.call(); }
                        )
                    }
                    if show_key {
                        div(class = "field") {
                            label(for = "wizard-key") { "API Key" }
                            input(
                                id = "wizard-key",
                                type = "password",
                                placeholder = "sk-…（保存到系统钥匙串，不落盘明文）",
                                oninput = move |_| { on_set_api_key.call(); }
                            )
                        }
                    }
                }
            }
        }
        WizardStep::Identity => {
            let on_set_self_description = props.on_set_self_description.clone();
            view! {
                section(class = "step-body") {
                    div(class = "field") {
                        label(for = "wizard-self") {
                            "用一两句话描述你自己，以及希望他如何与你相处"
                        }
                        textarea(
                            id = "wizard-self",
                            rows = "4",
                            placeholder = "例：「叫我小明。我是学生，常用中文，回答请简洁可靠。」",
                            oninput = move |_| { on_set_self_description.call(); }
                        )
                    }
                    p(class = "hint") {
                        "它会成为「用户印象」的初始档案——阿佩瑞斯带着这份理解与你相处（可在设置里随时改或清空；留空 = 之后再补）。"
                    }
                }
            }
        }
        WizardStep::Preset => {
            let on_toggle_auto_tune = props.on_toggle_auto_tune.clone();
            view! {
                section(class = "step-body") {
                    p(class = "lead") {
                        "将为普通用户写入以下预设（之后可在「能力与安全」里逐项改）："
                    }
                    ul(class = "intro-list") {
                        li { "记忆核心族六件：主动回忆 / 偏好学习 / 记忆注入 / 记忆固化 / 反思沉淀 / 器官链" }
                        li { "预算族保持后端默认（上下文 24000 字符 / 回合 8 轮 / 工具 16 次），随使用自动伸缩" }
                        li { "危险能力（shell / fetch / 文件写）保持关闭——不进任何预设" }
                    }
                    label(class = "tune-row") {
                        input(type = "checkbox", onchange = move |_| { on_toggle_auto_tune.call(); })
                        span {
                            strong { "使用中自动调整参数" }
                            small { "从使用中学习，自动微调体验参数（selfTuning）；关闭 = 预设值固定不变" }
                        }
                    }
                }
            }
        }
        WizardStep::ProGuide => {
            let lines = props.pro_guide_lines.clone();
            view! {
                section(class = "step-body") {
                    p(class = "lead") {
                        "专业用户通道——只做介绍，不写任何预设 / 配置 / 印象："
                    }
                    ul(class = "intro-list") {
                        for line in lines {
                            li { line }
                        }
                    }
                    p(class = "hint") {
                        "想换到普通用户轨道拿到预设值：设置 ›「能力与安全」有「推荐配置」一键预设。"
                    }
                }
            }
        }
    }
}

/// 渲染页脚（**五种互斥形态**；照抄生产的分支链）。
fn render_footer(props: &FirstRunWizardProps) -> VNode {
    let kind = props.footer_kind();
    let chevron_left = Icon::render(&IconProps::new(&icons::ICON_CHEVRON_LEFT, 14));
    let chevron_right = Icon::render(&IconProps::new(&ICON_CHEVRON_RIGHT, 14));

    let on_skip = props.on_skip.clone();
    let on_next = props.on_next.clone();
    let on_back = props.on_back.clone();
    let on_complete = props.on_complete.clone();

    match kind {
        FooterKind::Start => view! {
            footer(class = "wizard-foot") {
                button(class = "skip-btn", onclick = move |_| { on_skip.call(); }) {
                    "以后再说"
                }
                button(class = "start-btn", onclick = move |_| { on_next.call(); }) {
                    "开始引导"
                }
            }
        },
        FooterKind::Blocked => {
            // 生产是**静态** `disabled` ⇒ DOM 上是 `disabled=""`（不是属性缺失）。
            let disabled = AttrValue::Text(String::new());
            view! {
                footer(class = "wizard-foot") {
                    button(class = "nav-btn", onclick = move |_| { on_back.call(); }) {
                        { chevron_left }
                        " 上一步"
                    }
                    button(class = "start-btn", disabled = disabled) {
                        "下一步 "
                        { chevron_right }
                    }
                }
            }
        }
        FooterKind::Next => view! {
            footer(class = "wizard-foot") {
                button(class = "nav-btn", onclick = move |_| { on_back.call(); }) {
                    { chevron_left }
                    " 上一步"
                }
                button(class = "start-btn", onclick = move |_| { on_next.call(); }) {
                    "下一步 "
                    { chevron_right }
                }
            }
        },
        FooterKind::ProDone => view! {
            footer(class = "wizard-foot") {
                button(class = "nav-btn", onclick = move |_| { on_back.call(); }) {
                    { chevron_left }
                    " 上一步"
                }
                button(class = "start-btn", onclick = move |_| { on_complete.call(); }) {
                    "完成（不写任何预设）"
                }
            }
        },
        FooterKind::Finish => {
            // 生产：应用中文案与完成文案二选一。
            let finish_text = if props.applying {
                "正在应用（网关重启中）…"
            } else {
                "完成"
            };
            view! {
                footer(class = "wizard-foot") {
                    button(class = "nav-btn", onclick = move |_| { on_back.call(); }) {
                        { chevron_left }
                        " 上一步"
                    }
                    button(class = "start-btn", onclick = move |_| { on_complete.call(); }) {
                        finish_text
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    //! 向导面的**行为**断言（渲染结果 ⇒ 结构 / class / 文本），不读源码。

    use super::*;
    use apeireth_ui_dom::{DomEvent, EventKind};
    use std::cell::Cell;
    use std::rc::Rc;

    /// 建一份完整的 props（**与固定数据同形**）。
    fn sample_props() -> FirstRunWizardProps {
        FirstRunWizardProps {
            tier: None,
            step_index: 0,
            steps: vec![
                WizardStep::Welcome,
                WizardStep::Tier,
                WizardStep::Api,
                WizardStep::Identity,
                WizardStep::Preset,
            ],
            step_titles: vec![
                WizardStepTitle {
                    step: WizardStep::Welcome,
                    title: String::from("欢迎使用 Apeireth 伙伴"),
                    sub: String::from("先把「这是什么」讲清楚，约 2 分钟"),
                },
                WizardStepTitle {
                    step: WizardStep::Tier,
                    title: String::from("你是哪类用户？"),
                    sub: String::from("不同轨道，不同待遇——之后随时可改"),
                },
                WizardStepTitle {
                    step: WizardStep::Api,
                    title: String::from("连接模型服务"),
                    sub: String::from("密钥只进系统钥匙串，不落盘明文"),
                },
                WizardStepTitle {
                    step: WizardStep::Identity,
                    title: String::from("自我描述词"),
                    sub: String::from("他如何理解你——随每次对话带在身上"),
                },
                WizardStepTitle {
                    step: WizardStep::Preset,
                    title: String::from("普通用户预设值"),
                    sub: String::from("一次配好，使用中自动调整参数"),
                },
                WizardStepTitle {
                    step: WizardStep::ProGuide,
                    title: String::from("专业用户参考"),
                    sub: String::from("只做介绍——一切由你自己掌控"),
                },
            ],
            presets: vec![
                WizardPreset {
                    id: String::from("deepseek"),
                    name: String::from("DeepSeek"),
                    base_url: String::from("https://api.deepseek.com/v1"),
                    model: String::from("deepseek-v4-flash"),
                    hint: String::from("推荐 · 本项目实测路径"),
                },
                WizardPreset {
                    id: String::from("openai"),
                    name: String::from("OpenAI"),
                    base_url: String::from("https://api.openai.com/v1"),
                    model: String::from("gpt-4o"),
                    hint: String::new(),
                },
            ],
            intro_bullets: vec![String::from("要点一"), String::from("要点二")],
            intro_plan: vec![String::from("预告一")],
            pro_guide_lines: vec![String::from("参考一")],
            preset_id: String::from("deepseek"),
            base_url: String::from("https://api.deepseek.com/v1"),
            model: String::from("deepseek-v4-flash"),
            api_key: String::new(),
            self_description: String::new(),
            auto_tune: true,
            applying: false,
            error: String::new(),
            ..FirstRunWizardProps::default()
        }
    }

    /// 收集 class token（保序去重）。
    fn classes(node: &VNode, out: &mut Vec<String>) {
        for token in node.class_tokens() {
            if !out.contains(&token) {
                out.push(token.clone());
            }
        }
        for child in node.children() {
            classes(child, out);
        }
    }

    /// 收集文本节点（逐个，含空白）。
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

    /// 收集标签名（保序）。
    fn tags(node: &VNode, out: &mut Vec<String>) {
        if let Some(tag) = node.tag() {
            out.push(tag.to_owned());
        }
        for child in node.children() {
            tags(child, out);
        }
    }

    /// 找第一个给定 class 的元素。
    fn find_by_class<'a>(node: &'a VNode, class: &str) -> Option<&'a VNode> {
        if node.class_tokens().iter().any(|token| token == class) {
            return Some(node);
        }
        node.children()
            .iter()
            .find_map(|child| find_by_class(child, class))
    }

    /// 收集元素的**直接子元素**（**穿透分组节点** ⇒ 与渲染后的 DOM 一致：
    /// 分组节点只是虚拟树里的容器，不产生 DOM 节点）。
    fn element_children(node: &VNode) -> Vec<&VNode> {
        let mut out = Vec::new();
        collect_elements(node, &mut out);
        out
    }

    /// 递归收集元素（跳过文本与分组节点本身）。
    fn collect_elements<'a>(node: &'a VNode, out: &mut Vec<&'a VNode>) {
        for child in node.children() {
            match child {
                VNode::Element(_) => out.push(child),
                VNode::Fragment(_) => collect_elements(child, out),
                VNode::Text(_) => {}
            }
        }
    }

    /// 收集带给定 class 的全部元素。
    fn collect_by_class<'a>(node: &'a VNode, class: &str, out: &mut Vec<&'a VNode>) {
        if node.class_tokens().iter().any(|token| token == class) {
            out.push(node);
        }
        for child in node.children() {
            collect_by_class(child, class, out);
        }
    }

    /// 收集给定标签的全部元素。
    fn collect_by_tag<'a>(node: &'a VNode, tag: &str, out: &mut Vec<&'a VNode>) {
        if node.tag() == Some(tag) {
            out.push(node);
        }
        for child in node.children() {
            collect_by_tag(child, tag, out);
        }
    }

    /// 收集给定属性的全部取值。
    fn collect_attr(node: &VNode, name: &str, out: &mut Vec<String>) {
        if let Some(value) = node
            .attr_value(name)
            .and_then(|value| value.to_attr_string())
        {
            out.push(value);
        }
        for child in node.children() {
            collect_attr(child, name, out);
        }
    }

    /// 渲染一棵树。
    fn render(props: &FirstRunWizardProps) -> VNode {
        FirstRunWizard::render(props)
    }

    /// 模拟一次点击。
    fn click(node: &VNode) {
        let element = match node {
            VNode::Element(element) => element,
            other => panic!("实测节点形态：{other:?}"),
        };
        assert_eq!(element.handlers.len(), 1, "恰好一个监听器");
        element.handlers[0]
            .1
            .call(&mut DomEvent::new(EventKind::Click, Vec::new()));
    }

    #[test]
    fn the_root_is_the_backdrop_with_the_production_class() {
        let node = render(&sample_props());
        assert_eq!(node.tag(), Some("div"));
        assert_eq!(node.class_tokens(), vec![String::from("wizard-backdrop")]);
    }

    #[test]
    fn the_card_carries_the_production_structure() {
        let node = render(&sample_props());
        let card = find_by_class(&node, "wizard-card").expect("卡片");
        let mut found = Vec::new();
        tags(card, &mut found);
        assert_eq!(
            found,
            vec![
                String::from("div"),
                String::from("header"),
                String::from("span"),
                String::from("svg"),
                String::from("path"),
                String::from("path"),
                String::from("path"),
                String::from("path"),
                String::from("path"),
                String::from("div"),
                String::from("h2"),
                String::from("p"),
                String::from("ol"),
                String::from("li"),
                String::from("li"),
                String::from("li"),
                String::from("li"),
                String::from("li"),
                String::from("section"),
                String::from("p"),
                String::from("ul"),
                String::from("li"),
                String::from("li"),
                String::from("div"),
                String::from("p"),
                String::from("footer"),
                String::from("button"),
                String::from("button"),
            ]
        );
    }

    #[test]
    fn the_step_dots_carry_the_aria_label() {
        let node = render(&sample_props());
        let dots = find_by_class(&node, "step-dots").expect("步骤条");
        assert_eq!(
            dots.attr_value("aria-label")
                .and_then(|value| value.to_attr_string()),
            Some(String::from("引导进度"))
        );
    }

    #[test]
    fn the_first_dot_is_active_and_the_rest_are_plain() {
        let node = render(&sample_props());
        let dots = find_by_class(&node, "step-dots").expect("步骤条");
        let items: Vec<&VNode> = element_children(dots)
            .into_iter()
            .filter(|child| child.tag() == Some("li"))
            .collect();
        assert_eq!(items.len(), 5);
        assert_eq!(items[0].class_tokens(), vec![String::from("active")]);
        assert!(items[1].class_tokens().is_empty());
    }

    #[test]
    fn the_done_dots_render_the_check_icon_instead_of_the_number() {
        let mut props = sample_props();
        props.step_index = 2;
        let node = render(&props);
        let dots = find_by_class(&node, "step-dots").expect("步骤条");
        let items: Vec<&VNode> = element_children(dots)
            .into_iter()
            .filter(|child| child.tag() == Some("li"))
            .collect();
        // 前两个：结构分支 ⇒ 图标（**不是**序号文本）。
        for item in &items[..2] {
            assert_eq!(item.class_tokens(), vec![String::from("done")]);
            let mut found = Vec::new();
            tags(item, &mut found);
            assert!(found.contains(&String::from("svg")), "{found:?}");
            let mut texts = Vec::new();
            text_nodes(item, &mut texts);
            assert!(texts.is_empty(), "完成点只有图标：{texts:?}");
        }
        // 第三个：序号文本。
        let mut texts = Vec::new();
        text_nodes(items[2], &mut texts);
        assert_eq!(texts, vec![String::from("3")]);
    }

    #[test]
    fn the_welcome_step_renders_the_intro_blocks() {
        let node = render(&sample_props());
        assert!(find_by_class(&node, "intro-list").is_some());
        assert!(find_by_class(&node, "plan-box").is_some());
        let mut texts = Vec::new();
        text_nodes(&node, &mut texts);
        assert!(texts.iter().any(|text| text == "要点一"), "{texts:?}");
        assert!(texts.iter().any(|text| text == "预告一"), "{texts:?}");
    }

    #[test]
    fn the_tier_step_renders_two_cards_without_a_selection() {
        let mut props = sample_props();
        props.step_index = 1;
        let node = render(&props);
        let mut cards = Vec::new();
        collect_by_class(&node, "tier-card", &mut cards);
        assert_eq!(cards.len(), 2);
        assert_eq!(cards[0].class_tokens(), vec![String::from("tier-card")]);
        assert!(
            !cards[0].class_tokens().contains(&String::from("selected")),
            "未选 ⇒ 无 selected"
        );
        assert!(
            !cards[1].class_tokens().contains(&String::from("selected")),
            "未选 ⇒ 无 selected"
        );
        let mut texts = Vec::new();
        text_nodes(&node, &mut texts);
        assert!(texts.iter().any(|text| text == "普通用户"), "{texts:?}");
        assert!(texts.iter().any(|text| text == "推荐"), "{texts:?}");
    }

    #[test]
    fn the_tier_step_marks_the_selected_card() {
        let mut props = sample_props();
        props.step_index = 1;
        props.tier = Some(WizardTier::Casual);
        let node = render(&props);
        let mut cards = Vec::new();
        collect_by_class(&node, "tier-card", &mut cards);
        assert_eq!(
            cards[0].class_tokens(),
            vec![String::from("tier-card"), String::from("selected")]
        );
        assert_eq!(cards[1].class_tokens(), vec![String::from("tier-card")]);
    }

    #[test]
    fn the_api_step_renders_the_preset_chips_and_the_fields() {
        let mut props = sample_props();
        props.step_index = 2;
        let node = render(&props);
        let mut chips = Vec::new();
        collect_by_class(&node, "preset-chip", &mut chips);
        assert_eq!(chips.len(), 2);
        assert_eq!(
            chips[0].class_tokens(),
            vec![String::from("preset-chip"), String::from("selected")]
        );
        // 空提示 ⇒ **不渲染** `<small>`（结构分支，与生产同构）。
        let mut found = Vec::new();
        tags(chips[1], &mut found);
        assert!(!found.contains(&String::from("small")), "{found:?}");
        let mut found = Vec::new();
        tags(chips[0], &mut found);
        assert!(found.contains(&String::from("small")), "{found:?}");
    }

    #[test]
    fn the_api_step_renders_the_key_field_unless_the_preset_is_local() {
        let mut props = sample_props();
        props.step_index = 2;
        let node = render(&props);
        let mut ids = Vec::new();
        collect_attr(&node, "id", &mut ids);
        assert!(ids.contains(&String::from("wizard-key")), "{ids:?}");

        let mut props = sample_props();
        props.step_index = 2;
        props.preset_id = String::from("ollama");
        let node = render(&props);
        let mut ids = Vec::new();
        collect_attr(&node, "id", &mut ids);
        assert!(!ids.contains(&String::from("wizard-key")), "{ids:?}");
    }

    #[test]
    fn the_identity_step_renders_the_textarea_with_four_rows() {
        let mut props = sample_props();
        props.step_index = 3;
        let node = render(&props);
        let mut areas = Vec::new();
        collect_by_tag(&node, "textarea", &mut areas);
        assert_eq!(areas.len(), 1);
        assert_eq!(
            areas[0]
                .attr_value("rows")
                .and_then(|value| value.to_attr_string()),
            Some(String::from("4"))
        );
        assert!(find_by_class(&node, "hint").is_some());
    }

    #[test]
    fn the_preset_step_renders_the_tune_row_checkbox() {
        let mut props = sample_props();
        props.step_index = 4;
        let node = render(&props);
        let mut boxes = Vec::new();
        collect_by_tag(&node, "input", &mut boxes);
        assert_eq!(boxes.len(), 1);
        assert_eq!(
            boxes[0]
                .attr_value("type")
                .and_then(|value| value.to_attr_string()),
            Some(String::from("checkbox"))
        );
        // 生产是双向绑定 ⇒ **DOM 上没有 checked 属性**（那是 property）。
        assert!(boxes[0].attr_value("checked").is_none());
    }

    #[test]
    fn the_pro_guide_step_renders_the_reference_lines() {
        let mut props = sample_props();
        props.tier = Some(WizardTier::Pro);
        props.steps = vec![WizardStep::Welcome, WizardStep::Tier, WizardStep::ProGuide];
        props.step_index = 2;
        let node = render(&props);
        let mut texts = Vec::new();
        text_nodes(&node, &mut texts);
        assert!(texts.iter().any(|text| text == "参考一"), "{texts:?}");
        assert!(
            texts.iter().any(|text| text == "完成（不写任何预设）"),
            "{texts:?}"
        );
    }

    #[test]
    fn the_welcome_footer_has_skip_and_start() {
        let node = render(&sample_props());
        let mut found = Vec::new();
        classes(&node, &mut found);
        assert!(found.contains(&String::from("skip-btn")), "{found:?}");
        assert!(found.contains(&String::from("start-btn")), "{found:?}");
        assert!(!found.contains(&String::from("nav-btn")), "{found:?}");
    }

    #[test]
    fn the_blocked_footer_carries_the_static_disabled_attribute() {
        let mut props = sample_props();
        props.step_index = 1;
        let node = render(&props);
        let mut buttons = Vec::new();
        collect_by_class(&node, "start-btn", &mut buttons);
        assert_eq!(buttons.len(), 1);
        assert_eq!(
            buttons[0]
                .attr_value("disabled")
                .and_then(|value| value.to_attr_string()),
            Some(String::new()),
            "生产是静态 `disabled` ⇒ DOM 上是 `disabled=\"\"`"
        );
    }

    #[test]
    fn the_next_footer_has_no_disabled_attribute() {
        let mut props = sample_props();
        props.tier = Some(WizardTier::Casual);
        props.step_index = 2;
        let node = render(&props);
        let mut buttons = Vec::new();
        collect_by_class(&node, "start-btn", &mut buttons);
        assert_eq!(buttons.len(), 1);
        assert!(buttons[0].attr_value("disabled").is_none());
    }

    #[test]
    fn the_back_label_carries_a_leading_space_and_next_a_trailing_one() {
        let mut props = sample_props();
        props.tier = Some(WizardTier::Casual);
        props.step_index = 2;
        let node = render(&props);
        let mut texts = Vec::new();
        text_nodes(&node, &mut texts);
        assert!(texts.contains(&String::from(" 上一步")), "{texts:?}");
        assert!(texts.contains(&String::from("下一步 ")), "{texts:?}");
    }

    #[test]
    fn the_error_line_is_absent_when_the_error_is_empty() {
        let node = render(&sample_props());
        assert!(find_by_class(&node, "wizard-error").is_none());

        let mut props = sample_props();
        props.error = String::from("请填写 API 端点");
        let node = render(&props);
        let line = find_by_class(&node, "wizard-error").expect("错误行");
        let mut texts = Vec::new();
        text_nodes(line, &mut texts);
        assert_eq!(texts, vec![String::from("请填写 API 端点")]);
    }

    #[test]
    fn the_step_index_is_clamped_to_the_last_step() {
        let mut props = sample_props();
        props.step_index = 99;
        assert_eq!(props.current_step(), WizardStep::Preset);
        assert_eq!(props.footer_kind(), FooterKind::Finish);
    }

    #[test]
    fn the_footer_kind_follows_the_production_branch_chain() {
        let mut props = sample_props();
        assert_eq!(props.footer_kind(), FooterKind::Start);

        props.step_index = 1;
        assert_eq!(props.footer_kind(), FooterKind::Blocked);

        props.tier = Some(WizardTier::Casual);
        props.step_index = 2;
        assert_eq!(props.footer_kind(), FooterKind::Next);

        props.step_index = 4;
        assert_eq!(props.footer_kind(), FooterKind::Finish);

        let mut pro = props.clone();
        pro.tier = Some(WizardTier::Pro);
        pro.steps = vec![WizardStep::Welcome, WizardStep::Tier, WizardStep::ProGuide];
        pro.step_index = 2;
        assert_eq!(pro.footer_kind(), FooterKind::ProDone);
    }

    #[test]
    fn the_tier_ids_round_trip() {
        for tier in [WizardTier::Casual, WizardTier::Pro] {
            assert_eq!(WizardTier::from_id(tier.id()), Some(tier));
        }
        assert_eq!(WizardTier::from_id("nope"), None);
    }

    #[test]
    fn the_step_ids_round_trip() {
        for step in [
            WizardStep::Welcome,
            WizardStep::Tier,
            WizardStep::Api,
            WizardStep::Identity,
            WizardStep::Preset,
            WizardStep::ProGuide,
        ] {
            assert_eq!(WizardStep::from_id(step.id()), Some(step));
        }
        assert_eq!(WizardStep::from_id("nope"), None);
    }

    #[test]
    fn the_current_preset_is_found_by_id() {
        let props = sample_props();
        assert_eq!(
            props.current_preset().map(|preset| preset.name.as_str()),
            Some("DeepSeek")
        );
        let mut props = sample_props();
        props.preset_id = String::from("nope");
        assert!(props.current_preset().is_none());
    }

    #[test]
    fn the_component_is_a_pure_function_of_its_props() {
        // 组件是纯函数：同样的 props ⇒ 同样的**结构 / class / 文本**。
        // （处理器按指针身份比较 ⇒ 带回调时两次渲染在 `PartialEq` 上必然不等，
        //   那是 `ui-dom` 的既定口径 ⇒ 这里比可比的部分。）
        let first = render(&sample_props());
        let second = render(&sample_props());
        let mut first_tags = Vec::new();
        let mut second_tags = Vec::new();
        tags(&first, &mut first_tags);
        tags(&second, &mut second_tags);
        assert_eq!(first_tags, second_tags);
        let mut first_classes = Vec::new();
        let mut second_classes = Vec::new();
        classes(&first, &mut first_classes);
        classes(&second, &mut second_classes);
        assert_eq!(first_classes, second_classes);
        let mut first_texts = Vec::new();
        let mut second_texts = Vec::new();
        text_nodes(&first, &mut first_texts);
        text_nodes(&second, &mut second_texts);
        assert_eq!(first_texts, second_texts);
    }

    #[test]
    fn the_next_intent_is_emitted_by_the_next_button() {
        let hits = Rc::new(Cell::new(0u32));
        let counter = hits.clone();
        let mut props = sample_props();
        props.tier = Some(WizardTier::Casual);
        props.step_index = 2;
        props.on_next = Callback::new(move || counter.set(counter.get() + 1));
        let node = render(&props);
        let mut buttons = Vec::new();
        collect_by_class(&node, "start-btn", &mut buttons);
        click(buttons[0]);
        assert_eq!(hits.get(), 1, "点「下一步」⇒ 意图发出一次");
    }

    #[test]
    fn the_skip_intent_is_emitted_by_the_skip_button() {
        let hits = Rc::new(Cell::new(0u32));
        let counter = hits.clone();
        let mut props = sample_props();
        props.on_skip = Callback::new(move || counter.set(counter.get() + 1));
        let node = render(&props);
        let mut buttons = Vec::new();
        collect_by_class(&node, "skip-btn", &mut buttons);
        assert_eq!(buttons.len(), 1);
        click(buttons[0]);
        assert_eq!(hits.get(), 1);
    }

    #[test]
    fn the_tier_cards_emit_the_pick_intents() {
        let casual = Rc::new(Cell::new(0u32));
        let pro = Rc::new(Cell::new(0u32));
        let casual_counter = casual.clone();
        let pro_counter = pro.clone();
        let mut props = sample_props();
        props.step_index = 1;
        props.on_pick_casual = Callback::new(move || casual_counter.set(casual_counter.get() + 1));
        props.on_pick_pro = Callback::new(move || pro_counter.set(pro_counter.get() + 1));
        let node = render(&props);
        let mut cards = Vec::new();
        collect_by_class(&node, "tier-card", &mut cards);
        click(cards[0]);
        click(cards[1]);
        assert_eq!(casual.get(), 1);
        assert_eq!(pro.get(), 1);
    }

    #[test]
    fn the_preset_chips_emit_their_own_intents() {
        let first = Rc::new(Cell::new(0u32));
        let second = Rc::new(Cell::new(0u32));
        let first_counter = first.clone();
        let second_counter = second.clone();
        let mut props = sample_props();
        props.step_index = 2;
        props.preset_intents = vec![
            Callback::new(move || first_counter.set(first_counter.get() + 1)),
            Callback::new(move || second_counter.set(second_counter.get() + 1)),
        ];
        let node = render(&props);
        let mut chips = Vec::new();
        collect_by_class(&node, "preset-chip", &mut chips);
        click(chips[0]);
        click(chips[1]);
        assert_eq!(first.get(), 1);
        assert_eq!(second.get(), 1);
    }
}
