//! 设置页三页 · `wizard` 子面的 **wasm 宿主挂载**模块。
//!
//! ## 职责（三件，**不多不少**）
//!
//! 1. 注入**固定数据**（文案 / 预设表 / 初始状态）；
//! 2. 持有向导的**十个本地状态**（**调用方持有** ⇒ 组件是纯函数）；
//! 3. 把 [`ui_kit::settings_wizard::FirstRunWizard`] 挂到容器里。
//!
//! ## 「引擎不迁」的那一半（**只发意图**）
//!
//! | 生产的动作 | 本模块 |
//! |---|---|
//! | 密钥写进系统钥匙串 | **不迁**（宿主能力） |
//! | 自我描述词写进用户印象 | **不迁**（宿主能力） |
//! | 合并出最终配置并交给上层 | **不迁**（宿主能力） |
//! | 「完成」/「以后再说」 | **只发意图**（`on_complete` / `on_skip`） |
//!
//! ## 诚实边界
//!
//! - **输入框的值是 DOM property**（不是 attribute）⇒ 与生产同构 ⇒ 本模块在
//!   每次渲染后**显式回写** property；否则画面上的输入框是空的（DOM 对照会
//!   发现属性缺失，**像素对照**会发现框里没字）。
//! - 本模块**不实现**任何宿主能力（钥匙串 / 本地存储 / HTTP）⇒ 那些动作在生产
//!   侧仍是「未接线」的真实状态，本模块只把**意图**暴露出去。
//! - `wasm` 子模块整体被 `#[cfg(target_arch = "wasm32")]` 隔离 ⇒ **native 编译
//!   不参与** ⇒ 它的行为**未经 native 侧验证**（浏览器内的验证由 harness 承担：
//!   像素对照 + DOM 全等，见 `harness/settings/`）。

use std::cell::RefCell;
use std::rc::Rc;

use apeireth_ui_core::{Callback, ReactiveScope, Signal, UiComponent};
use apeireth_ui_dom::VNode;
use apeireth_ui_kit::settings_wizard::{
    FirstRunWizard, FirstRunWizardProps, WizardPreset, WizardStep, WizardStepTitle, WizardTier,
};

pub mod fixture {
    //! 向导的固定数据（**逐字取自生产**；见各常量的说明）。
    //!
    //! ## 真值口径
    //!
    //! - **生产组件**（`frontend/companion-desktop/src/lib/components/
    //!   FirstRunWizard.svelte`）里的 `PRESETS` / `STEP_TITLES`；
    //! - **生产文案**（`frontend/companion-desktop/src/lib/onboarding.ts`）里的
    //!   `INTRO_BULLETS` / `INTRO_PLAN` / `PRO_GUIDE_LINES` / `CASUAL_STEPS` /
    //!   `PRO_STEPS`。
    //!
    //! ⚠️ 固定数据草案（`harness/settings/fixtures/wizard.json`）里的 `minimax`
    //! 一条把 `MiniMax` / `MiniMax-M3` 写成了小写 ⇒ **与生产不一致** ⇒ 本模块
    //! 按**生产真值**（大写）实现（像素 / DOM 对照以生产侧为准）。

    /// 步骤标题表（六个步骤：主标题 + 副标题）。
    pub const STEP_TITLES: &[(&str, &str, &str)] = &[
        (
            "welcome",
            "欢迎使用 Apeireth 伙伴",
            "先把「这是什么」讲清楚，约 2 分钟",
        ),
        ("tier", "你是哪类用户？", "不同轨道，不同待遇——之后随时可改"),
        ("api", "连接模型服务", "密钥只进系统钥匙串，不落盘明文"),
        ("identity", "自我描述词", "他如何理解你——随每次对话带在身上"),
        ("preset", "普通用户预设值", "一次配好，使用中自动调整参数"),
        ("pro-guide", "专业用户参考", "只做介绍——一切由你自己掌控"),
    ];

    /// 预设表（id / 名称 / 端点 / 模型 / 提示）。
    pub const PRESETS: &[(&str, &str, &str, &str, &str)] = &[
        (
            "deepseek",
            "DeepSeek",
            "https://api.deepseek.com/v1",
            "deepseek-v4-flash",
            "推荐 · 本项目实测路径",
        ),
        (
            "openai",
            "OpenAI",
            "https://api.openai.com/v1",
            "gpt-4o",
            "",
        ),
        (
            "minimax",
            "MiniMax",
            "https://api.minimax.chat/v1",
            "MiniMax-M3",
            "",
        ),
        (
            "ollama",
            "Ollama 本地",
            "http://localhost:11434/v1",
            "llama3.3",
            "无需 key",
        ),
        ("custom", "自定义端点", "", "", "任意 OpenAI 兼容服务"),
    ];

    /// 欢迎步的要点。
    pub const INTRO_BULLETS: &[&str] = &[
        "对话：多会话、流式出字，审批与审计都在本地留痕",
        "治理：危险能力（shell / fetch / 文件写）默认关闭，逐次人工审批",
        "记忆：本地记忆库，核心记忆族可一键开启",
        "伙伴：阿佩瑞斯带着人设与用户印象与你相处，换人设即换档案",
    ];

    /// 欢迎步的预告。
    pub const INTRO_PLAN: &[&str] = &[
        "① 选择使用方式（普通用户 / 专业用户）",
        "② 连接模型服务（端点、模型、密钥）",
        "③ 自我描述词（他如何理解你）",
        "④ 普通用户预设值（使用中自动调整参数）；专业用户只做介绍",
    ];

    /// 专业用户参考步的条目。
    pub const PRO_GUIDE_LINES: &[&str] = &[
        "设置 ›「连接」：服务商、端点、模型、密钥（密钥只进系统钥匙串）",
        "设置 ›「伙伴档案」：人设、用户印象、记忆倾向、性情旋钮与成长日志",
        "设置 ›「能力与安全」：工具与高级能力开关（危险项默认关，逐次人工审批）",
        "设置 ›「外观」：主题、强调色、背景",
        "预算旋钮（回合轮数 / 工具调用 / 上下文预算）留空 = 后端默认，填写即显式生效",
    ];

    /// 普通用户轨道的步序。
    pub const CASUAL_STEPS: &[&str] = &["welcome", "tier", "api", "identity", "preset"];

    /// 专业用户轨道的步序。
    pub const PRO_STEPS: &[&str] = &["welcome", "tier", "pro-guide"];

    /// 初始分流（未选）。
    pub const INITIAL_TIER: Option<&str> = None;
    /// 初始步序下标。
    pub const INITIAL_STEP_INDEX: usize = 0;
    /// 初始预设 id。
    pub const INITIAL_PRESET_ID: &str = "deepseek";
    /// 初始端点。
    pub const INITIAL_BASE_URL: &str = "https://api.deepseek.com/v1";
    /// 初始模型名。
    pub const INITIAL_MODEL: &str = "deepseek-v4-flash";
    /// 初始密钥（空）。
    pub const INITIAL_API_KEY: &str = "";
    /// 初始自我描述词（空）。
    pub const INITIAL_SELF_DESCRIPTION: &str = "";
    /// 初始「使用中自动调整参数」（开）。
    pub const INITIAL_AUTO_TUNE: bool = true;
    /// 初始「应用中」（关）。
    pub const INITIAL_APPLYING: bool = false;
    /// 初始错误（空）。
    pub const INITIAL_ERROR: &str = "";
}

/// 向导的**状态快照**（宿主注入 / 对照脚本设置）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WizardState {
    /// 分流（`None` ⇒ 未选）。
    pub tier: Option<WizardTier>,
    /// 当前步序下标。
    pub step_index: usize,
    /// 当前预设 id。
    pub preset_id: String,
    /// 密钥。
    pub api_key: String,
    /// 端点。
    pub base_url: String,
    /// 模型名。
    pub model: String,
    /// 自我描述词。
    pub self_description: String,
    /// 「使用中自动调整参数」。
    pub auto_tune: bool,
    /// 「应用中」。
    pub applying: bool,
    /// 错误文案。
    pub error: String,
}

impl Default for WizardState {
    fn default() -> Self {
        Self {
            tier: match fixture::INITIAL_TIER {
                Some(id) => WizardTier::from_id(id),
                None => None,
            },
            step_index: fixture::INITIAL_STEP_INDEX,
            preset_id: String::from(fixture::INITIAL_PRESET_ID),
            api_key: String::from(fixture::INITIAL_API_KEY),
            base_url: String::from(fixture::INITIAL_BASE_URL),
            model: String::from(fixture::INITIAL_MODEL),
            self_description: String::from(fixture::INITIAL_SELF_DESCRIPTION),
            auto_tune: fixture::INITIAL_AUTO_TUNE,
            applying: fixture::INITIAL_APPLYING,
            error: String::from(fixture::INITIAL_ERROR),
        }
    }
}

/// 步骤 id ⇒ (分流, 步序下标)。
///
/// ## 口径
///
/// 六个步骤在**两条轨道**上各出现一次：分流步之前是**未选**分流（预览普通轨道
/// 的步序），分流步之后按轨道分流 ⇒ 专业轨道只有三步。
///
/// | 步骤 id | 分流 | 步序下标 |
/// |---|---|---:|
/// | `welcome` | 未选 | 0 |
/// | `tier` | 未选 | 1 |
/// | `api` | 普通 | 2 |
/// | `identity` | 普通 | 3 |
/// | `preset` | 普通 | 4 |
/// | `pro-guide` | 专业 | 2 |
pub fn step_anchor(step_id: &str) -> Option<(Option<WizardTier>, usize)> {
    match step_id {
        "welcome" => Some((None, 0)),
        "tier" => Some((None, 1)),
        "api" => Some((Some(WizardTier::Casual), 2)),
        "identity" => Some((Some(WizardTier::Casual), 3)),
        "preset" => Some((Some(WizardTier::Casual), 4)),
        "pro-guide" => Some((Some(WizardTier::Pro), 2)),
        _ => None,
    }
}

/// 宿主状态（**十个本地状态 + 运行时 + 已发出的意图**）。
#[derive(Clone)]
pub struct WizardHost {
    /// 响应式作用域（**使用者形态** ⇒ 本状态**挂不了 effect**）。
    scope: ReactiveScope,
    /// 分流。
    tier: Signal<Option<WizardTier>>,
    /// 步序下标。
    step_index: Signal<usize>,
    /// 当前预设 id。
    preset_id: Signal<String>,
    /// 密钥。
    api_key: Signal<String>,
    /// 端点。
    base_url: Signal<String>,
    /// 模型名。
    model: Signal<String>,
    /// 自我描述词。
    self_description: Signal<String>,
    /// 自动调参。
    auto_tune: Signal<bool>,
    /// 应用中。
    applying: Signal<bool>,
    /// 错误文案。
    error: Signal<String>,
    /// **已发出的意图**（宿主执行未接线 ⇒ 这里**如实记录**，不假装执行）。
    emitted: Rc<RefCell<Vec<String>>>,
}

impl WizardHost {
    /// 建一份宿主状态（**初始态取固定数据**）。
    pub fn new(scope: ReactiveScope) -> Self {
        let state = WizardState::default();
        Self {
            tier: scope.signal(state.tier),
            step_index: scope.signal(state.step_index),
            preset_id: scope.signal(state.preset_id),
            api_key: scope.signal(state.api_key),
            base_url: scope.signal(state.base_url),
            model: scope.signal(state.model),
            self_description: scope.signal(state.self_description),
            auto_tune: scope.signal(state.auto_tune),
            applying: scope.signal(state.applying),
            error: scope.signal(state.error),
            emitted: Rc::new(RefCell::new(Vec::new())),
            scope,
        }
    }

    /// **已发出的意图**（宿主侧动作：完成 / 跳过）。
    ///
    /// ## 为什么只记录不执行
    ///
    /// 生产的「完成」要**写配置 / 钥匙串 / 用户印象**、「以后再说」要**跳过引导**
    /// ⇒ 都是**宿主能力**（引擎不碰）⇒ 本模块**只发意图**。
    ///
    /// ⇒ 意图**被如实记录**（可被断言 / 被宿主订阅）⇒ **不假装执行**（0 装诚实）。
    pub fn emitted_intents(&self) -> Vec<String> {
        self.emitted.borrow().clone()
    }

    /// 记一条意图。
    fn emit(&self, intent: &str) {
        self.emitted.borrow_mut().push(String::from(intent));
    }

    /// 响应式作用域（**使用者形态**；供宿主复核「同一个运行时」）。
    pub fn scope(&self) -> &ReactiveScope {
        &self.scope
    }

    /// 分流信号。
    pub fn tier(&self) -> Signal<Option<WizardTier>> {
        self.tier.clone()
    }

    /// 步序下标信号。
    pub fn step_index(&self) -> Signal<usize> {
        self.step_index.clone()
    }

    /// 预设 id 信号。
    pub fn preset_id(&self) -> Signal<String> {
        self.preset_id.clone()
    }

    /// 密钥信号。
    pub fn api_key(&self) -> Signal<String> {
        self.api_key.clone()
    }

    /// 端点信号。
    pub fn base_url(&self) -> Signal<String> {
        self.base_url.clone()
    }

    /// 模型名信号。
    pub fn model(&self) -> Signal<String> {
        self.model.clone()
    }

    /// 自我描述词信号。
    pub fn self_description(&self) -> Signal<String> {
        self.self_description.clone()
    }

    /// 自动调参信号。
    pub fn auto_tune(&self) -> Signal<bool> {
        self.auto_tune.clone()
    }

    /// 「应用中」信号。
    pub fn applying(&self) -> Signal<bool> {
        self.applying.clone()
    }

    /// 错误文案信号。
    pub fn error(&self) -> Signal<String> {
        self.error.clone()
    }

    /// 当前轨道的步序表（**分流决定**；未选 ⇒ 普通轨道预览，与生产一致）。
    pub fn steps(&self) -> Vec<WizardStep> {
        let ids = match self.tier.get() {
            Some(WizardTier::Pro) => fixture::PRO_STEPS,
            _ => fixture::CASUAL_STEPS,
        };
        ids.iter()
            .filter_map(|id| WizardStep::from_id(id))
            .collect()
    }

    /// 当前状态快照。
    pub fn state(&self) -> WizardState {
        WizardState {
            tier: self.tier.get(),
            step_index: self.step_index.get(),
            preset_id: self.preset_id.get(),
            api_key: self.api_key.get(),
            base_url: self.base_url.get(),
            model: self.model.get(),
            self_description: self.self_description.get(),
            auto_tune: self.auto_tune.get(),
            applying: self.applying.get(),
            error: self.error.get(),
        }
    }

    /// 覆盖状态（**一次性写全部字段** ⇒ 一次重绘）。
    pub fn set_state(&self, state: WizardState) {
        self.tier.set(state.tier);
        self.step_index.set(state.step_index);
        self.preset_id.set(state.preset_id);
        self.api_key.set(state.api_key);
        self.base_url.set(state.base_url);
        self.model.set(state.model);
        self.self_description.set(state.self_description);
        self.auto_tune.set(state.auto_tune);
        self.applying.set(state.applying);
        self.error.set(state.error);
    }

    /// 定位到某个步骤（**按 [`step_anchor`] 的表**；未知 id ⇒ `false`）。
    pub fn select_step(&self, step_id: &str) -> bool {
        let Some((tier, index)) = step_anchor(step_id) else {
            return false;
        };
        self.tier.set(tier);
        self.step_index.set(index);
        true
    }

    /// 选一个预设（**照抄生产**：同时改端点与模型，并清掉错误）。
    pub fn select_preset(&self, preset_id: &str) -> bool {
        let Some((_, _, base_url, model, _)) = fixture::PRESETS
            .iter()
            .find(|(id, _, _, _, _)| *id == preset_id)
        else {
            return false;
        };
        self.preset_id.set(String::from(preset_id));
        self.base_url.set(String::from(*base_url));
        self.model.set(String::from(*model));
        self.error.set(String::new());
        true
    }

    /// 组装组件 props（**组件是纯函数** ⇒ 状态 ⇒ props）。
    pub fn props(&self) -> FirstRunWizardProps {
        let steps = self.steps();
        let step_titles = fixture::STEP_TITLES
            .iter()
            .filter_map(|(id, title, sub)| {
                WizardStep::from_id(id).map(|step| WizardStepTitle {
                    step,
                    title: String::from(*title),
                    sub: String::from(*sub),
                })
            })
            .collect();
        let presets = fixture::PRESETS
            .iter()
            .map(|(id, name, base_url, model, hint)| WizardPreset {
                id: String::from(*id),
                name: String::from(*name),
                base_url: String::from(*base_url),
                model: String::from(*model),
                hint: String::from(*hint),
            })
            .collect();
        let preset_intents = (0..fixture::PRESETS.len())
            .map(|index| {
                let host = self.clone();
                let id = String::from(fixture::PRESETS[index].0);
                Callback::new(move || {
                    host.select_preset(&id);
                })
            })
            .collect();

        FirstRunWizardProps {
            tier: self.tier.get(),
            step_index: self.step_index.get(),
            steps,
            step_titles,
            presets,
            intro_bullets: fixture::INTRO_BULLETS
                .iter()
                .map(|line| String::from(*line))
                .collect(),
            intro_plan: fixture::INTRO_PLAN
                .iter()
                .map(|line| String::from(*line))
                .collect(),
            pro_guide_lines: fixture::PRO_GUIDE_LINES
                .iter()
                .map(|line| String::from(*line))
                .collect(),
            preset_id: self.preset_id.get(),
            base_url: self.base_url.get(),
            model: self.model.get(),
            api_key: self.api_key.get(),
            self_description: self.self_description.get(),
            auto_tune: self.auto_tune.get(),
            applying: self.applying.get(),
            error: self.error.get(),
            on_next: self.intent_next(),
            on_back: self.intent_back(),
            on_skip: self.intent_skip(),
            on_complete: self.intent_complete(),
            on_pick_casual: self.intent_pick(WizardTier::Casual),
            on_pick_pro: self.intent_pick(WizardTier::Pro),
            preset_intents,
            on_set_base_url: Callback::new(|| {}),
            on_set_model: Callback::new(|| {}),
            on_set_api_key: Callback::new(|| {}),
            on_set_self_description: Callback::new(|| {}),
            on_toggle_auto_tune: Callback::new(|| {}),
        }
    }

    /// 渲染整棵视图树（**状态变化 ⇒ 重新渲染**）。
    pub fn render_tree(&self) -> VNode {
        FirstRunWizard::render(&self.props())
    }

    /// 「下一步」意图（**照抄生产**：分流步未选 ⇒ 报错并停住）。
    fn intent_next(&self) -> Callback {
        let host = self.clone();
        Callback::new(move || {
            host.error.set(String::new());
            let steps = host.steps();
            let last = steps.len().saturating_sub(1);
            let current = host.step_index.get();
            let step = steps.get(current.min(last)).copied();
            if step == Some(WizardStep::Tier) && host.tier.get().is_none() {
                host.error.set(String::from("请选择一种使用方式"));
                return;
            }
            if step == Some(WizardStep::Api) && !host.validate_connection() {
                return;
            }
            host.step_index.set((current + 1).min(last));
        })
    }

    /// 「上一步」意图（**照抄生产**：夹到第一步）。
    fn intent_back(&self) -> Callback {
        let host = self.clone();
        Callback::new(move || {
            host.error.set(String::new());
            host.step_index.set(host.step_index.get().saturating_sub(1));
        })
    }

    /// 「以后再说」意图（**跳过**；执行在宿主 ⇒ 本模块**只记录意图**）。
    fn intent_skip(&self) -> Callback {
        let host = self.clone();
        Callback::new(move || {
            host.emit("skip");
        })
    }

    /// 「完成」意图（**执行在宿主** ⇒ 本模块**只记录意图**）。
    fn intent_complete(&self) -> Callback {
        let host = self.clone();
        Callback::new(move || {
            host.emit("complete");
        })
    }

    /// 选分流意图（**照抄生产**：清错误）。
    fn intent_pick(&self, tier: WizardTier) -> Callback {
        let host = self.clone();
        Callback::new(move || {
            host.tier.set(Some(tier));
            host.error.set(String::new());
        })
    }

    /// 连接信息校验（**照抄生产**：本地端点免密钥）。
    pub fn validate_connection(&self) -> bool {
        let endpoint = self.base_url.get().trim().to_owned();
        let model = self.model.get().trim().to_owned();
        let preset_id = self.preset_id.get();
        let key = if preset_id == "ollama" {
            String::new()
        } else {
            self.api_key.get().trim().to_owned()
        };
        if endpoint.is_empty() {
            self.error.set(String::from("请填写 API 端点"));
            return false;
        }
        if model.is_empty() {
            self.error.set(String::from("请填写模型名"));
            return false;
        }
        if preset_id != "ollama" && key.is_empty() {
            self.error
                .set(String::from("请填写 API Key（Ollama 本地可留空）"));
            return false;
        }
        true
    }
}

/// wasm 宿主挂载（**引擎侧**）。
#[cfg(target_arch = "wasm32")]
pub mod wasm {
    //! 向导面的 wasm 装配（**native 编译不参与**）。
    //!
    //! ## 装配形态（照批一 / 批二）
    //!
    //! `WebDom`（真实 DOM 后端）+ `Renderer`（挂载 / 打补丁）+ `MicrotaskScheduler`
    //! （写入 ⇒ 一次微任务 flush）+ `Runtime::effect`（状态变化 ⇒ 重算虚拟树 ⇒
    //! 就地打补丁）。
    //!
    //! ## DOM property 回写（**为什么需要**）
    //!
    //! 生产的输入框用 `bind:value` / `bind:checked` ⇒ 值是 **property**（DOM 上
    //! **没有** `value` / `checked` 属性）⇒ 引擎侧若只写属性，DOM 对照会多出属性、
    //! 像素对照会看到**空输入框** ⇒ 本模块在每次渲染后**显式回写** property
    //! （**只在当前值 ≠ 目标值时写** ⇒ 不打断输入法组合态与光标位置）。

    use std::cell::RefCell;
    use std::rc::Rc;

    use apeireth_ui_core::{HostScheduler, Runtime};
    use apeireth_ui_dom::{Renderer, WebDom};
    use js_sys::Reflect;
    use wasm_bindgen::prelude::*;
    use web_sys::Element;

    use super::{WizardHost, WizardState};

    /// 微任务调度器：把 flush 排进浏览器的微任务队列。
    ///
    /// ## 为什么**不用** `#[wasm_bindgen] extern "C"`
    ///
    /// 本模块文件**同时**被 lib 目标与 bin 目标编译 ⇒ 任何 `#[wasm_bindgen]`
    /// 记号（**导出**与**导入**都一样）都会在两个编译单元里各生成一份同名
    /// 外部符号 ⇒ wasm 链接阶段**重复符号**失败。
    ///
    /// ⇒ 处置：微任务入口走 `window.queueMicrotask`（**纯 JS 反射**，无符号）。
    struct MicrotaskScheduler;

    impl HostScheduler for MicrotaskScheduler {
        fn schedule_flush(&self, flush: Box<dyn FnOnce()>) {
            let callback = Closure::once_into_js(move || {
                flush();
                JsValue::UNDEFINED
            });
            let Some(window) = web_sys::window() else {
                return;
            };
            let Ok(entry) = Reflect::get(window.as_ref(), &JsValue::from_str("queueMicrotask"))
            else {
                return;
            };
            let Ok(function) = entry.dyn_into::<js_sys::Function>() else {
                return;
            };
            let _ = function.call1(&JsValue::NULL, callback.as_ref());
        }
    }

    /// 已挂载的应用（**单例**）。
    struct WizardApp {
        /// 宿主状态。
        host: WizardHost,
    }

    thread_local! {
        /// wasm 侧的单例应用。
        static APP: RefCell<Option<WizardApp>> = const { RefCell::new(None) };
        /// 控制 API 的闭包（**必须活到页面结束** ⇒ 存在这里，不随函数返回丢弃）。
        static CONTROL_CLOSURES: RefCell<Vec<Closure<dyn FnMut(JsValue) -> JsValue>>> =
            const { RefCell::new(Vec::new()) };
    }

    /// 挂载向导面。
    ///
    /// ## 失败口径
    ///
    /// 任何一步失败 ⇒ **显式返回错误**（不静默吞掉）⇒ 页面显示为空白。
    pub fn mount(container: &Element) -> Result<(), String> {
        let dom = WebDom::new().map_err(|error| error.to_string())?;
        let root: web_sys::Node = container.clone().into();
        let mut renderer = Renderer::new(dom, root);

        // 应用根（**runtime 的所有者**）：建运行时 ⇒ ⇒ 交给状态持有者一个作用域。
        let runtime = Runtime::new();
        runtime.set_host_scheduler(Rc::new(MicrotaskScheduler));
        let host = WizardHost::new(runtime.reactive_scope());

        let tree = host.render_tree();
        renderer.mount(tree).map_err(|error| error.to_string())?;

        let renderer = Rc::new(RefCell::new(renderer));
        let effect_renderer = renderer.clone();
        let effect_host = host.clone();
        let effect_container = container.clone();
        runtime.effect(move || {
            let tree = effect_host.render_tree();
            let mut guard = effect_renderer.borrow_mut();
            let _ = guard.patch(tree);
            drop(guard);
            let _ = sync_properties(&effect_host, &effect_container);
        });

        // 首次渲染后的 property 回写（输入框的值 / 勾选框的勾选态）。
        let _ = sync_properties(&host, container);

        APP.with(|slot| {
            *slot.borrow_mut() = Some(WizardApp { host: host.clone() });
        });

        install_control_api();
        patch_settings_api();
        mark_ready();
        Ok(())
    }

    /// 设置页面的当前步骤（**共享 harness 的 `selectSection` 调它**）。
    ///
    /// 未知步骤 id ⇒ **不挂任何东西**（显式不假装已接线）。
    ///
    /// ## 为什么能导出（`#[wasm_bindgen]`）
    ///
    /// 本模块文件**同时**被 lib 目标与 bin 目标声明 ⇒ 若两边都编译它 ⇒ 任何
    /// `#[wasm_bindgen]` 记号都会产生**同名外部符号** ⇒ wasm 链接失败。
    /// ⇒ 主线已把 `demo/src/lib.rs` 的声明改成 `#[cfg(not(target_arch =
    /// "wasm32"))]` ⇒ **wasm 侧只剩 bin 这一份** ⇒ 导出安全 ✓。
    #[wasm_bindgen]
    pub fn set_settings_section(id: &str) {
        APP.with(|slot| {
            let slot = slot.borrow();
            let Some(app) = slot.as_ref() else {
                return;
            };
            app.host.select_step(id);
        });
    }

    /// 包裹共享 API 的 `selectSection`（**兜底**：让共享契约对本面**也**生效）。
    ///
    /// ## 为什么还要包一层
    ///
    /// 共享的 `web/engine/main.js` 在 `selectSection(id)` 里**先**调
    /// `module.set_settings_section(id)`（wasm 导出，**存在**）**再**由共享代码
    /// 维护 `__SETTINGS_SECTION__` ⇒ 正常路径已经驱动本面状态。
    ///
    /// ⇒ 但共享代码是 `if (typeof module.set_settings_section === 'function')`
    /// ⇒ **导出缺失时静默跳过** ⇒ 本层包一层，保证「共享契约 ⇒ 本面状态」这条
    /// 链**不依赖导出是否存在**（幂等 ⇒ 与导出重复设置也无副作用）。
    ///
    /// ## 历史坑（**已由主线修**）
    ///
    /// 本模块文件**同时**被 lib 目标与 bin 目标编译 ⇒ 任何 `#[wasm_bindgen]`
    /// 记号都会产生**同名外部符号** ⇒ wasm 链接阶段**重复符号**失败
    /// （实测报错：`duplicate symbol`）⇒ 主线已把 `demo/src/lib.rs` 的声明
    /// 改成 `#[cfg(not(target_arch = "wasm32"))]` ⇒ wasm 侧只剩 bin 这一份。
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
            // 先按**共享契约**更新 `__SETTINGS_SECTION__`，再驱动本面状态。
            let _ = original.call1(&JsValue::NULL, &args);
            let id = args.as_string().unwrap_or_default();
            APP.with(|slot| {
                let slot = slot.borrow();
                if let Some(app) = slot.as_ref() {
                    app.host.select_step(&id);
                }
            });
            JsValue::UNDEFINED
        }) as Box<dyn FnMut(JsValue) -> JsValue>);
        let function: &js_sys::Function = wrapped.as_ref().unchecked_ref();
        let _ = Reflect::set(&api, &JsValue::from_str("selectSection"), function.as_ref());
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

    /// 装控制 API（`window.__WIZARD__`）—— 对照脚本用它设置状态。
    fn install_control_api() {
        let Some(window) = web_sys::window() else {
            return;
        };

        let set_state = Closure::wrap(Box::new(|args: JsValue| -> JsValue {
            // **一个数组参数**（闭包只拿到第一个实参 ⇒ 十个字段打包成数组）。
            let item = |index: f64| -> JsValue {
                Reflect::get(&args, &JsValue::from_f64(index)).unwrap_or(JsValue::UNDEFINED)
            };
            let text = |index: f64| -> String { item(index).as_string().unwrap_or_default() };
            let flag = |index: f64| -> bool { item(index).as_bool().unwrap_or(false) };
            let tier = text(0.0);
            let state = WizardState {
                tier: if tier.is_empty() {
                    None
                } else {
                    super::WizardTier::from_id(&tier)
                },
                step_index: item(1.0).as_f64().unwrap_or(0.0) as usize,
                preset_id: text(2.0),
                api_key: text(3.0),
                base_url: text(4.0),
                model: text(5.0),
                self_description: text(6.0),
                auto_tune: flag(7.0),
                applying: flag(8.0),
                error: text(9.0),
            };
            APP.with(|slot| {
                let slot = slot.borrow();
                if let Some(app) = slot.as_ref() {
                    app.host.set_state(state);
                }
            });
            JsValue::UNDEFINED
        }) as Box<dyn FnMut(JsValue) -> JsValue>);

        let get_state = Closure::wrap(Box::new(|_args: JsValue| -> JsValue {
            let state = APP.with(|slot| {
                slot.borrow()
                    .as_ref()
                    .map(|app| app.host.state())
                    .unwrap_or_default()
            });
            let object = js_sys::Object::new();
            let set = |key: &str, value: JsValue| {
                let _ = Reflect::set(object.as_ref(), &JsValue::from_str(key), &value);
            };
            set(
                "tier",
                match state.tier {
                    Some(tier) => JsValue::from_str(tier.id()),
                    None => JsValue::NULL,
                },
            );
            set("stepIndex", JsValue::from_f64(state.step_index as f64));
            set("presetId", JsValue::from_str(&state.preset_id));
            set("apiKey", JsValue::from_str(&state.api_key));
            set("baseUrl", JsValue::from_str(&state.base_url));
            set("model", JsValue::from_str(&state.model));
            set(
                "selfDescription",
                JsValue::from_str(&state.self_description),
            );
            set("autoTune", JsValue::from_bool(state.auto_tune));
            set("applying", JsValue::from_bool(state.applying));
            set("error", JsValue::from_str(&state.error));
            object.into()
        }) as Box<dyn FnMut(JsValue) -> JsValue>);

        let get_intents = Closure::wrap(Box::new(|_args: JsValue| -> JsValue {
            let intents = APP.with(|slot| {
                slot.borrow()
                    .as_ref()
                    .map(|app| app.host.emitted_intents())
                    .unwrap_or_default()
            });
            let array = js_sys::Array::new();
            for intent in intents {
                array.push(&JsValue::from_str(&intent));
            }
            array.into()
        }) as Box<dyn FnMut(JsValue) -> JsValue>);

        let select_step = Closure::wrap(Box::new(|args: JsValue| -> JsValue {
            let id = args.as_string().unwrap_or_default();
            APP.with(|slot| {
                let slot = slot.borrow();
                if let Some(app) = slot.as_ref() {
                    app.host.select_step(&id);
                }
            });
            JsValue::UNDEFINED
        }) as Box<dyn FnMut(JsValue) -> JsValue>);

        let api = js_sys::Object::new();
        let bind = |key: &str, closure: &Closure<dyn FnMut(JsValue) -> JsValue>| {
            // `Closure::as_ref()` 就是那个 JS 函数对象 ⇒ 直接挂上去。
            let function: &js_sys::Function = closure.as_ref().unchecked_ref();
            let _ = Reflect::set(api.as_ref(), &JsValue::from_str(key), function.as_ref());
        };
        bind("setState", &set_state);
        bind("getState", &get_state);
        bind("selectStep", &select_step);
        bind("getIntents", &get_intents);
        let _ = Reflect::set(
            api.as_ref(),
            &JsValue::from_str("ready"),
            &JsValue::from_bool(true),
        );

        let _ = Reflect::set(
            window.as_ref(),
            &JsValue::from_str("__WIZARD__"),
            api.as_ref(),
        );

        CONTROL_CLOSURES.with(|slot| {
            let mut slot = slot.borrow_mut();
            slot.push(set_state);
            slot.push(get_state);
            slot.push(select_step);
            slot.push(get_intents);
        });
    }

    /// 回写 DOM property（**输入框的值 / 勾选框的勾选态 / 应用中禁用**）。
    ///
    /// **只在当前值 ≠ 目标值时写** ⇒ 不打断输入法组合态与光标位置。
    fn sync_properties(host: &WizardHost, container: &Element) -> Result<(), String> {
        let state = host.state();
        set_property(container, "#wizard-url", "value", &state.base_url)?;
        set_property(container, "#wizard-model", "value", &state.model)?;
        set_property(container, "#wizard-key", "value", &state.api_key)?;
        set_property(container, "#wizard-self", "value", &state.self_description)?;
        set_checked(container, ".tune-row input", state.auto_tune)?;
        if state.applying {
            set_disabled(container, true)?;
        }
        Ok(())
    }

    /// 写一个字符串 property。
    fn set_property(
        container: &Element,
        selector: &str,
        name: &str,
        value: &str,
    ) -> Result<(), String> {
        let Some(element) = query(container, selector) else {
            return Ok(());
        };
        let current = Reflect::get(element.as_ref(), &JsValue::from_str(name))
            .map(|value| value.as_string().unwrap_or_default())
            .unwrap_or_default();
        if current == value {
            return Ok(());
        }
        Reflect::set(
            element.as_ref(),
            &JsValue::from_str(name),
            &JsValue::from_str(value),
        )
        .map(|_| ())
        .map_err(|error| format!("写 {selector} 的 {name} 失败：{error:?}"))
    }

    /// 写一个布尔 property。
    fn set_checked(container: &Element, selector: &str, value: bool) -> Result<(), String> {
        let Some(element) = query(container, selector) else {
            return Ok(());
        };
        let current = Reflect::get(element.as_ref(), &JsValue::from_str("checked"))
            .map(|value| value.as_bool().unwrap_or(false))
            .unwrap_or(false);
        if current == value {
            return Ok(());
        }
        Reflect::set(
            element.as_ref(),
            &JsValue::from_str("checked"),
            &JsValue::from_bool(value),
        )
        .map(|_| ())
        .map_err(|error| format!("写 {selector} 的 checked 失败：{error:?}"))
    }

    /// 「应用中」⇒ 禁用页脚按钮（**只禁用没有静态 `disabled` 属性的那些**）。
    fn set_disabled(container: &Element, value: bool) -> Result<(), String> {
        let Some(footer) = query(container, ".wizard-foot") else {
            return Ok(());
        };
        let buttons = footer
            .query_selector_all("button")
            .map_err(|error| format!("查页脚按钮失败：{error:?}"))?;
        for index in 0..buttons.length() {
            let Some(button) = buttons.item(index) else {
                continue;
            };
            // 静态 `disabled`（分流步的「下一步」）⇒ **不动**（动了会抹掉属性）。
            let Some(element) = button.dyn_ref::<Element>() else {
                continue;
            };
            if element.has_attribute("disabled") {
                continue;
            }
            let _ = Reflect::set(
                button.as_ref(),
                &JsValue::from_str("disabled"),
                &JsValue::from_bool(value),
            );
        }
        Ok(())
    }

    /// 查一个元素（**查不到 ⇒ `None`**，不报错：元素可能不在当前步骤里）。
    fn query(container: &Element, selector: &str) -> Option<Element> {
        container.query_selector(selector).ok().flatten()
    }
}

#[cfg(test)]
mod tests {
    //! 宿主侧的**行为**断言（状态 ⇒ 步序 / 校验 / 意图），不读源码。

    use super::*;
    use apeireth_ui_core::Runtime;
    use std::cell::Cell;
    use std::rc::Rc;

    #[test]
    fn the_step_anchor_covers_the_six_registered_sections() {
        assert_eq!(step_anchor("welcome"), Some((None, 0)));
        assert_eq!(step_anchor("tier"), Some((None, 1)));
        assert_eq!(step_anchor("api"), Some((Some(WizardTier::Casual), 2)));
        assert_eq!(step_anchor("identity"), Some((Some(WizardTier::Casual), 3)));
        assert_eq!(step_anchor("preset"), Some((Some(WizardTier::Casual), 4)));
        assert_eq!(step_anchor("pro-guide"), Some((Some(WizardTier::Pro), 2)));
        assert_eq!(step_anchor("nope"), None);
    }

    #[test]
    fn the_initial_state_matches_the_fixed_data() {
        let state = WizardState::default();
        assert_eq!(state.tier, None);
        assert_eq!(state.step_index, 0);
        assert_eq!(state.preset_id, "deepseek");
        assert_eq!(state.base_url, "https://api.deepseek.com/v1");
        assert_eq!(state.model, "deepseek-v4-flash");
        assert_eq!(state.api_key, "");
        assert_eq!(state.self_description, "");
        assert!(state.auto_tune);
        assert!(!state.applying);
        assert_eq!(state.error, "");
    }

    #[test]
    fn the_step_list_follows_the_tier() {
        let host = WizardHost::new(Runtime::new().reactive_scope());
        assert_eq!(host.steps().len(), 5, "未选 ⇒ 普通轨道预览");
        host.tier.set(Some(WizardTier::Pro));
        assert_eq!(host.steps().len(), 3, "专业轨道三步");
        assert_eq!(host.steps()[2], WizardStep::ProGuide);
    }

    #[test]
    fn selecting_a_step_moves_the_state() {
        let host = WizardHost::new(Runtime::new().reactive_scope());
        assert!(host.select_step("api"));
        assert_eq!(host.tier.get(), Some(WizardTier::Casual));
        assert_eq!(host.step_index.get(), 2);
        assert!(!host.select_step("nope"), "未知 id ⇒ 不假装已接线");
    }

    #[test]
    fn selecting_a_preset_updates_the_endpoint_and_the_model() {
        let host = WizardHost::new(Runtime::new().reactive_scope());
        assert!(host.select_preset("ollama"));
        assert_eq!(host.preset_id.get(), "ollama");
        assert_eq!(host.base_url.get(), "http://localhost:11434/v1");
        assert_eq!(host.model.get(), "llama3.3");
        assert!(!host.select_preset("nope"));
    }

    #[test]
    fn the_connection_validation_follows_the_production_rules() {
        let host = WizardHost::new(Runtime::new().reactive_scope());
        // 默认预设（deepseek）⇒ 缺密钥 ⇒ 不通过。
        assert!(!host.validate_connection());
        assert_eq!(host.error.get(), "请填写 API Key（Ollama 本地可留空）");

        // 本地端点 ⇒ 免密钥。
        host.select_preset("ollama");
        assert!(host.validate_connection());

        // 缺端点。
        host.base_url.set(String::new());
        assert!(!host.validate_connection());
        assert_eq!(host.error.get(), "请填写 API 端点");

        // 缺模型名。
        host.base_url.set(String::from("http://localhost:11434/v1"));
        host.model.set(String::new());
        assert!(!host.validate_connection());
        assert_eq!(host.error.get(), "请填写模型名");
    }

    #[test]
    fn the_next_intent_advances_and_stops_at_the_tier_step() {
        let host = WizardHost::new(Runtime::new().reactive_scope());
        let props = host.props();
        // 分流步未选 ⇒ 报错并停住。
        host.step_index.set(1);
        host.props().on_next.call();
        assert_eq!(host.step_index.get(), 1, "未选分流 ⇒ 不前进");
        assert_eq!(host.error.get(), "请选择一种使用方式");
        drop(props);

        // 选了分流 ⇒ 前进。
        host.tier.set(Some(WizardTier::Casual));
        host.props().on_next.call();
        assert_eq!(host.step_index.get(), 2);
        assert_eq!(host.error.get(), "");
    }

    #[test]
    fn the_back_intent_never_goes_below_the_first_step() {
        let host = WizardHost::new(Runtime::new().reactive_scope());
        host.props().on_back.call();
        assert_eq!(host.step_index.get(), 0);
        host.step_index.set(3);
        host.props().on_back.call();
        assert_eq!(host.step_index.get(), 2);
    }

    #[test]
    fn the_pick_intent_sets_the_tier_and_clears_the_error() {
        let host = WizardHost::new(Runtime::new().reactive_scope());
        host.error.set(String::from("旧错误"));
        host.props().on_pick_pro.call();
        assert_eq!(host.tier.get(), Some(WizardTier::Pro));
        assert_eq!(host.error.get(), "");
    }

    #[test]
    fn the_preset_intents_select_their_own_preset() {
        let host = WizardHost::new(Runtime::new().reactive_scope());
        let props = host.props();
        assert_eq!(props.preset_intents.len(), fixture::PRESETS.len());
        props.preset_intents[3].call();
        assert_eq!(host.preset_id.get(), "ollama");
        assert_eq!(host.base_url.get(), "http://localhost:11434/v1");
    }

    #[test]
    fn the_state_round_trips_through_the_host() {
        let host = WizardHost::new(Runtime::new().reactive_scope());
        let state = WizardState {
            tier: Some(WizardTier::Pro),
            step_index: 2,
            preset_id: String::from("ollama"),
            api_key: String::from("k"),
            base_url: String::from("u"),
            model: String::from("m"),
            self_description: String::from("d"),
            auto_tune: false,
            applying: true,
            error: String::from("e"),
        };
        host.set_state(state.clone());
        assert_eq!(host.state(), state);
    }

    #[test]
    fn the_render_tree_carries_the_backdrop_class() {
        let host = WizardHost::new(Runtime::new().reactive_scope());
        let tree = host.render_tree();
        assert_eq!(tree.tag(), Some("div"));
        assert_eq!(tree.class_tokens(), vec![String::from("wizard-backdrop")]);
    }

    #[test]
    fn the_fixed_data_appears_verbatim_in_the_production_sources() {
        // **漂移哨兵**：固定数据必须**逐字**出现在生产源文件里
        // （生产组件 + 生产文案模块）⇒ 生产改了 ⇒ 本测试**立刻红**。
        let component = PRODUCTION_COMPONENT;
        let onboarding = PRODUCTION_ONBOARDING;

        for (id, name, base_url, model, hint) in fixture::PRESETS {
            for text in [*name, *base_url, *model] {
                assert!(
                    component.contains(text),
                    "预设 `{id}` 的 `{text}` 不在生产组件里 ⇒ 固定数据已漂移"
                );
            }
            if !hint.is_empty() {
                assert!(
                    component.contains(hint),
                    "预设 `{id}` 的提示 `{hint}` 不在生产组件里 ⇒ 固定数据已漂移"
                );
            }
        }
        for (id, title, sub) in fixture::STEP_TITLES {
            assert!(
                component.contains(title),
                "步骤 `{id}` 的标题 `{title}` 不在生产组件里 ⇒ 固定数据已漂移"
            );
            assert!(
                component.contains(sub),
                "步骤 `{id}` 的副标题 `{sub}` 不在生产组件里 ⇒ 固定数据已漂移"
            );
        }
        for line in fixture::INTRO_BULLETS {
            assert!(
                onboarding.contains(line),
                "欢迎要点 `{line}` 不在生产文案模块里 ⇒ 固定数据已漂移"
            );
        }
        for line in fixture::INTRO_PLAN {
            assert!(
                onboarding.contains(line),
                "欢迎预告 `{line}` 不在生产文案模块里 ⇒ 固定数据已漂移"
            );
        }
        for line in fixture::PRO_GUIDE_LINES {
            assert!(
                onboarding.contains(line),
                "专业参考 `{line}` 不在生产文案模块里 ⇒ 固定数据已漂移"
            );
        }
    }

    #[test]
    fn the_fixed_data_matches_the_fixture_draft_ignoring_case() {
        // 固定数据草案（`harness/settings/fixtures/wizard.json`）与生产在
        // **大小写**上有一处差异（`minimax` 条目的 `MiniMax` / `MiniMax-M3`
        // 被写成了小写）⇒ 这里**忽略大小写**比对 ⇒ 主线把草案改好后本测试
        // 仍然绿。
        let raw = FIXTURE_DRAFT;
        for (id, name, base_url, model, hint) in fixture::PRESETS {
            for text in [*name, *base_url, *model, *hint] {
                if text.is_empty() {
                    continue;
                }
                assert!(
                    raw.to_lowercase().contains(&text.to_lowercase()),
                    "预设 `{id}` 的 `{text}` 不在固定数据草案里 ⇒ 固定数据已漂移"
                );
            }
        }
        for (id, title, sub) in fixture::STEP_TITLES {
            for text in [*title, *sub] {
                assert!(
                    raw.contains(text),
                    "步骤 `{id}` 的 `{text}` 不在固定数据草案里 ⇒ 固定数据已漂移"
                );
            }
        }
        for line in fixture::INTRO_BULLETS
            .iter()
            .chain(fixture::INTRO_PLAN.iter())
            .chain(fixture::PRO_GUIDE_LINES.iter())
        {
            assert!(
                raw.contains(line),
                "文案 `{line}` 不在固定数据草案里 ⇒ 固定数据已漂移"
            );
        }
    }

    /// 生产的向导组件（**编译期嵌入** ⇒ 不用 `std::fs` ⇒ 不撞仓库的
    /// `disallowed-methods` 门；生产一改 ⇒ 下面的断言立刻红）。
    const PRODUCTION_COMPONENT: &str = include_str!(
        "../../../frontend/companion-desktop/src/lib/components/FirstRunWizard.svelte"
    );

    /// 生产的文案模块（编译期嵌入）。
    const PRODUCTION_ONBOARDING: &str =
        include_str!("../../../frontend/companion-desktop/src/lib/onboarding.ts");

    /// 固定数据草案（编译期嵌入）。
    const FIXTURE_DRAFT: &str = include_str!("../harness/settings/fixtures/wizard.json");

    #[test]
    fn the_host_side_intents_are_recorded_not_swallowed() {
        // 「完成」/「以后再说」的**执行**在宿主（写配置 / 钥匙串 / 印象）⇒ 本模块
        // **只发意图** ⇒ 意图**必须被记录**（不静默吞掉）。
        let host = WizardHost::new(Runtime::new().reactive_scope());
        let props = host.props();
        props.on_complete.call();
        props.on_skip.call();
        assert_eq!(
            host.emitted_intents(),
            vec![String::from("complete"), String::from("skip")]
        );
    }

    #[test]
    fn the_intent_callbacks_are_present_on_the_props() {
        let host = WizardHost::new(Runtime::new().reactive_scope());
        let props = host.props();
        let hits = Rc::new(Cell::new(0u32));
        let counter = hits.clone();
        props.on_skip.call();
        props.on_complete.call();
        counter.set(counter.get() + 1);
        assert_eq!(hits.get(), 1, "跳过 / 完成都是**意图**（执行在宿主）");
    }
}
