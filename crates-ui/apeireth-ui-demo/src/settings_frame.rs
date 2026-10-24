//! 页面框架与导航（**设置页 · 框架面**）的 wasm 宿主装配。
//!
//! ## 本模块是什么
//!
//! **宿主侧**：持有导航三态（`activeSection` / `expandedGroups` /
//! `lastSectionByGroup`）+ **固定数据**（侧栏的大类 / 分区表），把
//! `ui_kit::settings_frame` 的组件挂到页面的 `#app` 容器里。
//!
//! ## 诚实边界
//!
//! | 事项 | 口径 |
//! |---|---|
//! | 数据 | **固定 fixture**（宿主注入）⇒ 引擎不碰 HTTP / SSE 契约（设计 §1.3 第 6 条） |
//! | 页头位 | `PageHeader` 是**外部组件** ⇒ 本面**只留空位**（不迁）⇒ 引擎侧页头区高度为 0（生产 117.53px） |
//! | 错误横幅位 | `ErrorSolutionBanner` 是**外部组件** ⇒ 本面**只留空位**（不迁） |
//! | 内容列 | 本面**不拥有**任何内容区（那是 M4–M7 各面）⇒ 内容列在本面**为空** |
//! | `selectSection('runtime')` 触发深度诊断 | **宿主能力**（HTTP）⇒ **不迁** ⇒ 引擎侧只改分区 |
//! | 键串 / 存储 / 剪贴板 | **宿主能力** ⇒ **不迁** |
//!
//! ## `lastSectionByGroup`：**保持非响应式**（与生产同构）
//!
//! 生产里它是**普通 `Map`** ⇒ 不触发重渲染；且**只被 `toggleGroup` 读**
//! ⇒ 本模块把它留成 [`std::cell::RefCell`] + `HashMap`（**普通映射**，不是
//! [`Signal`]）⇒ **零重排**，与生产**同构**。
//!
//! ⇒ 组件（`ui_kit::settings_frame`）**根本不读它** ⇒ 迁移成信号也不会带来
//! 「多一次重排」；这里选择的是**与生产逐字同构**的那条路（保持非响应式）。
//!
//! ## 固定数据的来源
//!
//! 侧栏的大类 / 分区表来自 `harness/settings/fixtures/nav-groups.json`
//! （**顺序与文案逐字取自生产**）⇒ 见 [`data`] 模块（**生成物**）。

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use apeireth_ui_core::{Runtime, Signal};
use apeireth_ui_dom::VNode;
use apeireth_ui_kit::settings_frame::{NavGroup, SettingsFrameProps};
use apeireth_ui_macro::view;

/// 侧栏的固定数据（**生成物** —— 勿手改）。
pub mod data {
    use apeireth_ui_kit::settings_frame::{icons, NavGroup, NavSection};

    /// 侧栏的固定数据（**生成物** —— 勿手改）。
    ///
    /// 来源：`crates-ui/apeireth-ui-demo/harness/settings/fixtures/nav-groups.json`
    /// （**顺序与文案逐字取自生产**：`settings-nav-groups.ts` +
    /// `SettingsView.svelte:1309-1336`）。
    ///
    /// ⚠️ **大类说明（blurb）是数据**（不进导航列 ⇒ 落位在内容列顶部的
    /// `.category-context`）⇒ 本块只是**把 fixture 喂进来**，不是把它写死在
    /// 组件里。
    pub fn nav_groups() -> Vec<NavGroup> {
        vec![
            NavGroup {
                id: String::from("account"),
                title: String::from("账户与服务商"),
                blurb: String::from("你是谁、它靠谁供给智能：用户资料、服务商、密钥、模型与连接。"),
                icon: &icons::ICON_CPU,
                items: vec![
                    NavSection {
                        id: String::from("user"),
                        label: String::from("用户中心"),
                        icon: &icons::ICON_CIRCLE_USER_ROUND,
                    },
                    NavSection {
                        id: String::from("models"),
                        label: String::from("模型与提供商"),
                        icon: &icons::ICON_CPU,
                    },
                ],
            },
            NavGroup {
                id: String::from("character"),
                title: String::from("它的性格"),
                blurb: String::from(
                    "它是谁、它怎么看你：人设、用户印象、记忆倾向、性情旋钮与成长日志。",
                ),
                icon: &icons::ICON_SPARKLES,
                items: vec![
                    NavSection {
                        id: String::from("personality"),
                        label: String::from("伙伴人设与行为"),
                        icon: &icons::ICON_USER,
                    },
                    NavSection {
                        id: String::from("impression"),
                        label: String::from("用户印象"),
                        icon: &icons::ICON_EYE,
                    },
                    NavSection {
                        id: String::from("cognition"),
                        label: String::from("记忆与认知"),
                        icon: &icons::ICON_BRAIN,
                    },
                    NavSection {
                        id: String::from("disposition"),
                        label: String::from("性格与记忆"),
                        icon: &icons::ICON_SPARKLES,
                    },
                ],
            },
            NavGroup {
                id: String::from("capability"),
                title: String::from("能力与安全"),
                blurb: String::from("它能做什么、要经谁允许：能力开关、预算与治理。"),
                icon: &icons::ICON_SHIELD_CHECK,
                items: vec![
                    NavSection {
                        id: String::from("tools"),
                        label: String::from("工具与安全"),
                        icon: &icons::ICON_WRENCH,
                    },
                    NavSection {
                        id: String::from("budget"),
                        label: String::from("预算与配额"),
                        icon: &icons::ICON_GAUGE,
                    },
                    NavSection {
                        id: String::from("governance"),
                        label: String::from("决策与治理"),
                        icon: &icons::ICON_SCALE,
                    },
                    NavSection {
                        id: String::from("security"),
                        label: String::from("安全与治理"),
                        icon: &icons::ICON_SHIELD_CHECK,
                    },
                ],
            },
            NavGroup {
                id: String::from("appearance"),
                title: String::from("外观"),
                blurb: String::from("它住的房间怎么布置：主题、背景、形象与动效。"),
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
            NavGroup {
                id: String::from("data"),
                title: String::from("数据与工作区"),
                blurb: String::from("记忆与数据落在哪：工作区、存储、诊断与开发者项。"),
                icon: &icons::ICON_ARCHIVE,
                items: vec![
                    NavSection {
                        id: String::from("data"),
                        label: String::from("数据与存储"),
                        icon: &icons::ICON_TRASH_2,
                    },
                    NavSection {
                        id: String::from("runtime"),
                        label: String::from("运行时与诊断"),
                        icon: &icons::ICON_ACTIVITY,
                    },
                    NavSection {
                        id: String::from("developer"),
                        label: String::from("开发者选项"),
                        icon: &icons::ICON_CODE,
                    },
                ],
            },
        ]
    }
}

/// 宿主注入的**页头节点**（**同形**，只为**几何对齐**）。
///
/// ## 为什么注入页头（而不是留空）
///
/// 生产里 `PageHeader` 是**外部组件**（23 行）⇒ 本面**不迁**它 ⇒ 按切分线
/// 的口径，框架面「只放占位」。
///
/// ⚠️ 但**页头决定整页的纵向几何**：生产的 `.page-head` 高 **117.53px**
/// （`base.css:277` 的 `padding: 26px 42px 16px` + 三行文字）⇒ 侧栏的
/// 导航行因此落在**小数相位**上（y = 117.53 + 16 + …）。
///
/// ⇒ 引擎侧若把页头留空 ⇒ 导航行落在**整数相位**上 ⇒ 两侧**同一批字形**
/// 的**抗锯齿**不同 ⇒ 逐像素对照会出现**约 2%**的**假差异**（与字形无关）。
///
/// ⇒ 宿主按生产的**同形节点**注入页头（`PageHeader` 组件本身**仍未迁**）
/// ⇒ 引擎侧的整页纵向几何与生产**一致** ⇒ 其它内容面（M4–M7）也能逐像素
/// 对照。
///
/// ## 文案
///
/// 页头的三行文字是**生产 markup 里的固定串**（`SettingsView.svelte:1922-1927`
/// ⇒ `PageHeader` 的 props）⇒ 逐字照抄。
fn page_header() -> VNode {
    view! {
        div(class = "page-head") {
            div {
                p(class = "eyebrow") { "首偏好" }
                h1 { "系统设置" }
                p(class = "subtitle") {
                    "配置模型提供商、记忆与认知、决策治理、工具安全与数据存储。改动即点/失焦即生效。"
                }
            }
        }
    }
}

/// 宿主侧的导航三态。
///
/// ## 三个状态的口径（**逐字照抄生产**）
///
/// | 状态 | 类型 | 生产对应 | 写入者 |
/// |---|---|---|---|
/// | 当前分区 | [`Signal`] | `activeSection`（`$state`） | 本模块（宿主） |
/// | 展开的大类集合 | [`Signal`] | `expandedGroups`（`$state<string[]>`） | 本模块（宿主） |
/// | 每大类最近访问的分区页 | [`RefCell`] + `HashMap` | `lastSectionByGroup`（**普通 `Map`** ⇒ **非响应式**） | 本模块（宿主） |
#[derive(Clone)]
pub struct FrameState {
    /// 运行时（信号宿主）。
    runtime: Runtime,
    /// 当前分区 id。
    active_section: Signal<String>,
    /// 展开的大类集合（**真值口径** = 集合里有它）。
    expanded_groups: Signal<Vec<String>>,
    /// 「应用进行中」（**宿主写，框架面只读**）。
    live_apply_pending: Signal<bool>,
    /// 每个大类记住最近访问的分区页（**普通映射 ⇒ 非响应式** ⇒ 零重排）。
    last_section_by_group: Rc<RefCell<HashMap<String, String>>>,
    /// 大类表（固定数据）。
    groups: Vec<NavGroup>,
}

impl FrameState {
    /// 建一份初始状态（**生产同口径**：`activeSection = 'appearance'`，
    /// `expandedGroups = [分组(activeSection)]`）。
    pub fn new() -> Self {
        let runtime = Runtime::new();
        Self {
            active_section: runtime.signal(String::from("appearance")),
            expanded_groups: runtime.signal(vec![String::from("appearance")]),
            live_apply_pending: runtime.signal(false),
            last_section_by_group: Rc::new(RefCell::new(HashMap::new())),
            groups: data::nav_groups(),
            runtime,
        }
    }

    /// 运行时（信号宿主）。
    pub fn runtime(&self) -> &Runtime {
        &self.runtime
    }

    /// 当前分区 id。
    pub fn active_section(&self) -> String {
        self.active_section.get()
    }

    /// 展开的大类集合。
    pub fn expanded_groups(&self) -> Vec<String> {
        self.expanded_groups.get()
    }

    /// 每大类最近访问的分区页（**非响应式的普通映射** ⇒ 写它**不触发重绘**）。
    pub fn last_section_by_group(&self) -> Rc<RefCell<HashMap<String, String>>> {
        self.last_section_by_group.clone()
    }

    /// 大类表（固定数据）。
    pub fn groups(&self) -> &[NavGroup] {
        &self.groups
    }

    /// 某个分区所属的大类 id（**逐字照抄生产**：分区 → 大类的映射由大类表
    /// 的 `sectionIds` 派生 ⇒ 生产里是 `SETTINGS_SECTION_GROUP`）。
    pub fn group_of(&self, section_id: &str) -> Option<String> {
        self.groups
            .iter()
            .find(|group| group.items.iter().any(|item| item.id == section_id))
            .map(|group| group.id.clone())
    }

    /// 选中某个分区（**逐字照抄生产的 `selectSection`**）。
    ///
    /// ⚠️ 生产里 `selectSection('runtime')` 会顺带触发深度诊断（**HTTP**）
    /// ⇒ 那是**宿主能力** ⇒ **不迁** ⇒ 引擎侧只改分区。
    pub fn select_section(&self, section_id: &str) {
        if let Some(group_id) = self.group_of(section_id) {
            self.last_section_by_group
                .borrow_mut()
                .insert(group_id, String::from(section_id));
        }
        self.active_section.set(String::from(section_id));
    }

    /// 切换某个大类的展开态（**逐字照抄生产的 `toggleGroup`**）。
    ///
    /// - 展开着 ⇒ **收起**（内容列不动）；
    /// - 收着 ⇒ **展开**并落到该大类**最近访问**的分区页（无则落首格）。
    pub fn toggle_group(&self, group_id: &str) {
        let expanded = self.expanded_groups.get();
        if expanded.iter().any(|id| id == group_id) {
            // 展开着 ⇒ 收起（**内容列不动** ⇒ 不改当前分区）。
            let kept: Vec<String> = expanded.into_iter().filter(|id| id != group_id).collect();
            self.expanded_groups.set(kept);
            return;
        }
        let mut next = expanded;
        next.push(String::from(group_id));
        self.expanded_groups.set(next);
        // 落到最近访问的分区页（**非响应式映射** ⇒ 读它不触发重绘）。
        let remembered = self
            .last_section_by_group
            .borrow()
            .get(group_id)
            .cloned()
            .or_else(|| {
                self.groups
                    .iter()
                    .find(|group| group.id == group_id)
                    .and_then(|group| group.items.first().map(|item| item.id.clone()))
            });
        if let Some(section_id) = remembered {
            self.select_section(&section_id);
        }
    }

    /// 建一份组件 props（**状态归宿主** ⇒ 组件是纯函数）。
    pub fn props(&self) -> SettingsFrameProps {
        let select_state = self.clone();
        let toggle_state = self.clone();
        SettingsFrameProps {
            groups: self.groups.clone(),
            active_section: self.active_section.clone(),
            expanded_groups: self.expanded_groups.clone(),
            live_apply_pending: self.live_apply_pending.clone(),
            // `PageHeader` / `ErrorSolutionBanner` 都是**外部组件** ⇒ 本面只留
            // 空位（不迁）⇒ 错误横幅位**空**。
            //
            // ⚠️ 页头位**注入一个同形节点**（理由见 `page_header()` 的文档）。
            page_header: Some(page_header()),
            error_banner: None,
            // 内容列归**各内容面** ⇒ 本面不拥有。
            content: None,
            on_select_section: Rc::new(move |id: &str| select_state.select_section(id)),
            on_toggle_group: Rc::new(move |id: &str| toggle_state.toggle_group(id)),
        }
    }
}

impl Default for FrameState {
    fn default() -> Self {
        Self::new()
    }
}

/// wasm32 侧的装配（**native 编译不参与** ⇒ 行为由浏览器内对照钉住）。
#[cfg(target_arch = "wasm32")]
pub mod wasm {
    //! ## 诚实边界
    //!
    //! - 本模块整体被 `#[cfg(target_arch = "wasm32")]` 隔离 ⇒ **native 编译不参与**
    //!   ⇒ 它的行为**未经 native 侧验证**（浏览器内的验证由设置页 harness 承担：
    //!   像素对照 + DOM 全等 + 宽度同源断言）。
    //! - **真后端未接线**：数据面是**固定 fixture**。
    //!
    //! ## 调度
    //!
    //! 写入 ⇒ 一次**微任务** flush ⇒ 同一轮事件处理里的多次写合并为一次
    //! DOM 写入 ⇒ 界面在同一帧响应。
    //!
    //! ⚠️ 微任务调度器在本模块内**自带一份**（与 `main.rs` 里的同名实现同构）
    //! ⇒ 因为 `main.rs` 是**共享文件**（并行期不许改）。

    use std::cell::RefCell;
    use std::rc::Rc;

    use apeireth_ui_core::{HostScheduler, UiComponent};
    use apeireth_ui_dom::{Renderer, VNode, WebDom};
    use apeireth_ui_kit::settings_frame::SettingsFrame;
    use wasm_bindgen::prelude::*;
    use web_sys::Element;

    use super::FrameState;

    #[wasm_bindgen]
    extern "C" {
        /// 浏览器微任务队列（`queueMicrotask`）。
        #[wasm_bindgen(js_name = queueMicrotask)]
        fn queue_microtask(callback: &JsValue);
    }

    /// 微任务调度器：把 flush 排进浏览器的微任务队列。
    struct MicrotaskScheduler;

    impl HostScheduler for MicrotaskScheduler {
        fn schedule_flush(&self, flush: Box<dyn FnOnce()>) {
            let callback = Closure::once_into_js(move || {
                flush();
                JsValue::UNDEFINED
            });
            queue_microtask(&callback);
        }
    }

    thread_local! {
        /// wasm 侧的单例应用（框架面形态）。
        static FRAME_APP: RefCell<Option<FrameState>> = const { RefCell::new(None) };
    }

    /// 渲染整棵视图树。
    fn render_frame(state: &FrameState) -> VNode {
        SettingsFrame::render(&state.props())
    }

    /// 挂载框架面。
    ///
    /// ## 失败口径
    ///
    /// 失败 ⇒ 把错误写进 `window.__SETTINGS_ERROR__`（**共享的分发只 `let _ =
    /// result`** ⇒ 错误不会自动冒泡 ⇒ 本模块**自己**留痕，不静默）。
    pub fn mount(container: &Element) -> Result<(), String> {
        let dom = WebDom::new().map_err(|error| {
            let message = format!("WebDom 初始化失败：{error}");
            set_error(&message);
            message
        })?;
        let root: web_sys::Node = container.clone().into();
        let mut renderer = Renderer::new(dom, root);

        let state = FrameState::new();
        state
            .runtime()
            .set_host_scheduler(Rc::new(MicrotaskScheduler));

        if let Err(error) = renderer.mount(render_frame(&state)) {
            let message = format!("框架面首次挂载失败：{error}");
            set_error(&message);
            return Err(message);
        }

        let renderer = Rc::new(RefCell::new(renderer));
        let effect_renderer = renderer.clone();
        let effect_state = state.clone();
        state.runtime().effect(move || {
            let tree = render_frame(&effect_state);
            let mut guard = effect_renderer.borrow_mut();
            let _ = guard.patch(tree);
        });

        FRAME_APP.with(|slot| {
            *slot.borrow_mut() = Some(state);
        });
        set_ready();
        Ok(())
    }

    /// 标记「画面已渲染完毕」（对照脚本的 `ready()` 判据；两侧同名同义）。
    fn set_ready() {
        let Some(window) = web_sys::window() else {
            return;
        };
        let _ = js_sys::Reflect::set(
            window.as_ref(),
            &JsValue::from_str("__SETTINGS_READY__"),
            &JsValue::TRUE,
        );
    }

    /// 把错误写进全局（**不静默**）。
    fn set_error(message: &str) {
        let Some(window) = web_sys::window() else {
            return;
        };
        let _ = js_sys::Reflect::set(
            window.as_ref(),
            &JsValue::from_str("__SETTINGS_ERROR__"),
            &JsValue::from_str(message),
        );
    }
}

#[cfg(test)]
mod tests {
    //! 宿主侧的**行为**断言（状态转移 ⇒ 与生产同口径）。

    use super::*;
    use apeireth_ui_core::UiComponent;

    #[test]
    fn the_fixture_carries_five_groups_and_fifteen_sections_in_order() {
        let state = FrameState::new();
        let groups = state.groups();
        assert_eq!(groups.len(), 5, "5 个大类");
        let ids: Vec<&str> = groups.iter().map(|group| group.id.as_str()).collect();
        assert_eq!(
            ids,
            ["account", "character", "capability", "appearance", "data"],
            "大类顺序逐字取自生产"
        );
        let total: usize = groups.iter().map(|group| group.items.len()).sum();
        assert_eq!(total, 15, "15 个分区");
        assert_eq!(groups[0].items[0].label, "用户中心");
        assert_eq!(groups[4].items[2].label, "开发者选项");
    }

    #[test]
    fn the_initial_state_matches_the_production_one() {
        let state = FrameState::new();
        assert_eq!(state.active_section(), "appearance");
        assert_eq!(state.expanded_groups(), vec![String::from("appearance")]);
    }

    #[test]
    fn selecting_a_section_remembers_it_for_its_group() {
        let state = FrameState::new();
        state.select_section("models");
        assert_eq!(state.active_section(), "models");
        let remembered = state.last_section_by_group();
        assert_eq!(
            remembered.borrow().get("account").map(String::as_str),
            Some("models"),
            "最近访问的分区页按大类记住"
        );
    }

    #[test]
    fn toggling_an_expanded_group_closes_it_and_keeps_the_section() {
        let state = FrameState::new();
        state.select_section("avatar");
        state.toggle_group("appearance");
        assert_eq!(state.expanded_groups(), Vec::<String>::new(), "收起了");
        assert_eq!(state.active_section(), "avatar", "收起**不动内容列**");
    }

    #[test]
    fn toggling_a_closed_group_opens_it_and_lands_on_the_remembered_section() {
        let state = FrameState::new();
        // 先访问 appearance 大类的第二格 ⇒ 记住它。
        state.select_section("avatar");
        // 收起 appearance，再重新展开 ⇒ 应回到「形象」而不是首格。
        state.toggle_group("appearance");
        state.toggle_group("appearance");
        assert_eq!(
            state.expanded_groups(),
            vec![String::from("appearance")],
            "展开了"
        );
        assert_eq!(
            state.active_section(),
            "avatar",
            "回到**最近访问**的分区页（不是首格）"
        );
    }

    #[test]
    fn toggling_a_never_visited_group_lands_on_its_first_section() {
        let state = FrameState::new();
        state.toggle_group("data");
        assert_eq!(state.active_section(), "data", "落首格");
    }

    #[test]
    fn the_last_section_map_is_not_reactive() {
        // 生产里它是**普通 Map** ⇒ 写它**不触发**重绘。
        // ⇒ 这里用 effect 的运行次数把「非响应式」钉成**可断言的行为**。
        let state = FrameState::new();
        let counter = Rc::new(RefCell::new(0usize));
        let counter_for_effect = counter.clone();
        let state_for_effect = state.clone();
        let handle = state.runtime().effect(move || {
            // 读信号 ⇒ effect 订阅它们。
            let _ = state_for_effect.active_section();
            let _ = state_for_effect.expanded_groups();
            *counter_for_effect.borrow_mut() += 1;
        });
        state.runtime().flush();
        let after_first = handle.runs();

        // 只写**非响应式映射** ⇒ **不重绘**。
        state
            .last_section_by_group()
            .borrow_mut()
            .insert(String::from("account"), String::from("models"));
        state.runtime().flush();
        assert_eq!(
            handle.runs(),
            after_first,
            "写 `lastSectionByGroup` **不触发**重绘（非响应式）"
        );

        // 写**响应式信号** ⇒ 重绘。
        state.select_section("models");
        state.runtime().flush();
        assert!(
            handle.runs() > after_first,
            "写 `activeSection` ⇒ 重绘（响应式）"
        );
    }

    #[test]
    fn the_rendered_tree_carries_the_sidebar_and_the_content_column() {
        let state = FrameState::new();
        let node = apeireth_ui_kit::settings_frame::SettingsFrame::render(&state.props());
        let text = crate::tree_to_text(&node);
        assert!(text.contains("<aside class=\"settings-subnav\""), "{text}");
        assert!(text.contains("<div class=\"settings-content\""), "{text}");
        assert!(text.contains("账户与服务商"), "{text}");
        assert!(text.contains("外观与主题"), "{text}");
        assert!(text.contains("lucide-cpu"), "{text}");
    }
}
