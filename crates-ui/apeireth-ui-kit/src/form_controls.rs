//! # 表单控件组：单行文本 / 多行文本 / 复选框 / 下拉
//!
//! ## 职责边界（设计文档 §3.3.5 / §2.2 / §9.1）
//!
//! - 组件是**纯函数** `fn(&Props) -> VNode`：同样的 props ⇒ 同样的虚拟树
//!   ⇒ ⇒ native 下**直接断言虚拟树**即可，不需要浏览器、不需要 wasm；
//! - **class 名由调用方给**（class 名是**视觉契约** ⇒ 组件库**不发明**
//!   class 名）⇒ ⇒ 同一个控件能对齐不同的既有实现；
//! - **组件层不做布局**（ADR-0015）：不算宽高、不写内联样式；
//! - 只出现 **class 名与结构** ⇒ ⇒ **不出现颜色字面量与时长字面量**。
//!
//! ## ⭐ 双向绑定怎么接（**不是**「mount 时写一次 value」）
//!
//! 引擎的**双向绑定**机制是 [`Binding`]（`apeireth_ui_dom::binding`）：
//!
//! | 方向 | 载体 | 时机 |
//! |---|---|---|
//! | 信号 → DOM | [`apeireth_ui_dom::ElementVNode::bindings`] | 渲染阶段（**只在** DOM 当前值 ≠ 信号值时才写） |
//! | DOM → 信号 | 同一个绑定 | 事件处理器里（`input` / `change`） |
//!
//! ⇒ ⇒ 组件把**调用方持有的 `Signal`** 交给 [`Binding::text`] /
//! [`Binding::checked`] ⇒ ⇒ 「用户敲进去」与「程序改值」两个方向都通。
//!
//! ⚠️ **不用 `view!` 宏的 `bind:`**：`apeireth-ui-macro` 目前的 `bind:value`
//! 只生成**单向** property 写入（`.prop("value", …)`）⇒ ⇒ 组件层直接用
//! [`Binding`]（引擎真正的双向机制）。
//!
//! ⚠️ **信号要在渲染期读一次**：core 的 effect 是**依赖追踪式**的
//! ⇒ ⇒ 只把信号放进 [`Binding`]（不读它的值）⇒ ⇒ 视图**不会订阅**它
//! ⇒ ⇒ 「状态变了界面不动」且**没有任何报错**（引擎 `VNode::touch_bindings`
//! 的文档里点名了这个坑）⇒ ⇒ 本模块的每个组件都在渲染期读一次信号
//! ⇒ ⇒ **组件自己把依赖建起来**，不依赖宿主记得调 `touch_bindings`。
//!
//! ## ⭐ IME 组合期不误提交（中文输入法的关键判据）
//!
//! 中文输入法**组合期**里，用户按 `Enter` 是「**确认候选**」⇒ ⇒ **不是**
//! 「提交」⇒ ⇒ 不判就会把半成品拼音当成提交发出去（**误提交**）。
//!
//! ⇒ ⇒ 判据抽成**纯函数** [`should_submit`] / [`should_submit_multiline`]
//! ⇒ ⇒ native 单测可以**直接断言**（不依赖浏览器、不依赖真实 IME）。
//!
//! ## ⭐ `select` 的取值落在哪（**property，不是属性**）
//!
//! | 元素 | 取值通道 | 理由 |
//! |---|---|---|
//! | `<option>` | **属性** `value="…"` | 它的 `value` 是**静态内容属性**（每个选项一个） |
//! | `<select>` | **property** `value` | 它**没有** `value` 内容属性 ⇒ 写属性**无效** ⇒ 只能写 property |
//!
//! ⇒ ⇒ 组件对 `<select>` 用 [`ElementBuilder::prop`]（`prop("value", …)`）
//! ⇒ ⇒ **同时**给每个 `<option>` 标 `selected`（布尔 property）⇒ ⇒
//! 覆盖「选项列表被重建、但取值没变」这种差分场景（此时 `<select>` 的
//! `value` property **不会**被重写 ⇒ 只有 `selected` 能保住选中项）。
//!
//! ⚠️ `selected` **每项都写**（命中写 `true`、其余写 `false`）⇒ ⇒
//! 差分时是「翻转布尔」而不是「property 消失」⇒ ⇒ 不会触发后端的
//! `remove_property` 路径。

use apeireth_ui_core::{Signal, UiComponent};
use apeireth_ui_dom::{Binding, DomEvent, ElementBuilder, ElementHandler, EventKind, VNode};

// ═══════════════════════════════════════════════════════════════════════════
// 纯函数：按键判据（IME / Enter / Shift+Enter）
// ═══════════════════════════════════════════════════════════════════════════

/// ⭐⭐⭐ **纯函数**：这次按键**是不是「提交键」**（`Enter`）？
///
/// 只看按键名 ⇒ ⇒ **不看** IME 组合期 ⇒ ⇒ **不能单独用它判提交**
/// （组合期的 Enter 是「选词」）。
pub fn is_enter_key(event: &DomEvent) -> bool {
    matches!(event.key(), Some("Enter"))
}

/// ⭐⭐⭐ **纯函数**：这次按键**要不要触发提交**（**单行**输入框口径）。
///
/// ## 判据（两条同时成立）
///
/// 1. 按键是 `Enter`（[`is_enter_key`]）；
/// 2. **不在 IME 组合期**（`event.is_composing()` 为假）。
///
/// ## ⭐ 为什么必须判 IME 组合期
///
/// 中文输入法组合期里，用户按 `Enter` 是「**确认候选**」⇒ ⇒ **不是**「提交」
/// ⇒ ⇒ 不判 ⇒ ⇒ **误提交**（把半成品拼音发出去）。
///
/// ⇒ ⇒ 判据抽成**纯函数** ⇒ ⇒ 「组合期的 Enter 不提交」在 native 下
/// **可测**（断言直接打在这条函数上 ⇒ ⇒ 不需要浏览器、不需要真实 IME）。
///
/// ## 口径：单行输入框
///
/// 单行 `<input>` 里 `Enter` 的默认行为就是提交 ⇒ ⇒ `Shift` **不**参与判定
/// （浏览器里单行输入框的 `Shift+Enter` 同样提交）⇒ ⇒ 多行口径见
/// [`should_submit_multiline`]。
pub fn should_submit(event: &DomEvent) -> bool {
    is_enter_key(event) && !event.is_composing()
}

/// ⭐⭐ **纯函数**：这次按键**要不要触发提交**（**多行**文本框口径）。
///
/// = [`should_submit`] **且** **没有**按 `Shift`
/// ⇒ ⇒ `Shift+Enter` 是「**换行**」⇒ ⇒ 不提交（多行文本框里 Enter 才提交）。
pub fn should_submit_multiline(event: &DomEvent) -> bool {
    should_submit(event) && !event.shift_key()
}

// ═══════════════════════════════════════════════════════════════════════════
// 公共辅助
// ═══════════════════════════════════════════════════════════════════════════

/// 把**通用控件属性**写进构建器（`id` / `placeholder` / `aria-label` / `disabled`）。
///
/// ⚠️ `None` ⇒ **不写该属性**（不写 `id=""`、不写 `disabled="false"`）。
fn apply_common_attrs(
    mut node: ElementBuilder,
    id: Option<&str>,
    placeholder: Option<&str>,
    aria_label: Option<&str>,
    disabled: bool,
) -> ElementBuilder {
    if let Some(id) = id {
        node = node.attr("id", id);
    }
    if let Some(placeholder) = placeholder {
        node = node.attr("placeholder", placeholder);
    }
    if let Some(aria_label) = aria_label {
        node = node.attr("aria-label", aria_label);
    }
    if disabled {
        // 布尔 false ⇒ 不写该属性（不写 `disabled="false"`）。
        node = node.attr("disabled", true);
    }
    node
}

// ═══════════════════════════════════════════════════════════════════════════
// 单行文本框
// ═══════════════════════════════════════════════════════════════════════════

/// 单行文本框（`<input type="text">`）的 props。
///
/// ⚠️ props **不能** `PartialEq`：[`Signal`] 没有结构相等（同一个信号有
/// 多个克隆）⇒ ⇒ 只派生 `Debug` / `Clone`（与仓内既有含信号 props 一致）。
#[derive(Debug, Clone)]
pub struct TextFieldProps {
    /// class 名（可含多个 token）—— **视觉契约由调用方给**。
    pub class_name: String,
    /// 元素 id（`None` ⇒ 不写）。
    pub id: Option<String>,
    /// 占位提示（`None` ⇒ 不写）。
    pub placeholder: Option<String>,
    /// ⭐ **取值信号**（**调用方持有**）⇒ ⇒ 组件保持纯函数。
    pub value: Signal<String>,
    /// `Enter` 提交（`None` ⇒ 不挂提交处理器）。
    pub on_submit: Option<ElementHandler>,
    /// 输入事件（`None` ⇒ 不挂）。
    pub on_input: Option<ElementHandler>,
    /// 无障碍标签（`None` ⇒ 不写）。
    pub aria_label: Option<String>,
    /// 是否禁用（`false` ⇒ 不写 `disabled` 属性）。
    pub disabled: bool,
}

impl TextFieldProps {
    /// 建一组单行文本框 props（class 名与取值信号必给）。
    pub fn new(class_name: impl Into<String>, value: Signal<String>) -> Self {
        Self {
            class_name: class_name.into(),
            id: None,
            placeholder: None,
            value,
            on_submit: None,
            on_input: None,
            aria_label: None,
            disabled: false,
        }
    }

    /// 设元素 id（builder 风格）。
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// 设占位提示（builder 风格）。
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = Some(placeholder.into());
        self
    }

    /// 设 `Enter` 提交处理器（builder 风格）。
    ///
    /// ⚠️ 提交**必过** [`should_submit`] ⇒ ⇒ IME 组合期的 Enter **不**提交。
    pub fn on_submit(mut self, on_submit: ElementHandler) -> Self {
        self.on_submit = Some(on_submit);
        self
    }

    /// 设输入事件处理器（builder 风格）。
    ///
    /// ⚠️ 引擎会把**绑定写回**与**同类型的用户处理器**合成一个监听器
    /// ⇒ ⇒ 处理器里读到的信号**已经是**新值。
    pub fn on_input(mut self, on_input: ElementHandler) -> Self {
        self.on_input = Some(on_input);
        self
    }

    /// 设无障碍标签（builder 风格）。
    pub fn aria_label(mut self, aria_label: impl Into<String>) -> Self {
        self.aria_label = Some(aria_label.into());
        self
    }

    /// 设禁用态（builder 风格）。
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

/// 单行文本框组件。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TextField;

impl UiComponent for TextField {
    type Props = TextFieldProps;
    type Output = VNode;

    fn render(props: &Self::Props) -> Self::Output {
        // ⭐ 读一次信号 ⇒ ⇒ **建立依赖**（否则信号变了界面不动，且无报错）。
        let _ = props.value.get();

        let mut input = VNode::element("input")
            .attr("type", "text")
            .class(props.class_name.clone());
        input = apply_common_attrs(
            input,
            props.id.as_deref(),
            props.placeholder.as_deref(),
            props.aria_label.as_deref(),
            props.disabled,
        );
        // ⭐⭐⭐ **双向绑定**（不是「mount 时写一次 value」）：
        // 信号 ⇄ `input.value` property（渲染期写回 + `input` 事件写回信号）。
        input = input.bind(Binding::text(props.value.clone()));
        if let Some(on_input) = &props.on_input {
            input = input.handler(EventKind::Input, on_input.clone());
        }
        if let Some(on_submit) = &props.on_submit {
            let submit = on_submit.clone();
            input = input.handler(
                EventKind::KeyDown,
                ElementHandler::new(move |event: &mut DomEvent| {
                    // ⭐ IME 组合期的 Enter 是「选词」⇒ ⇒ **不**提交。
                    if should_submit(event) {
                        submit.call(event);
                    }
                }),
            );
        }
        input.build()
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// 多行文本框
// ═══════════════════════════════════════════════════════════════════════════

/// 多行文本框（`<textarea>`）的 props。
#[derive(Debug, Clone)]
pub struct TextAreaProps {
    /// class 名（可含多个 token）—— **视觉契约由调用方给**。
    pub class_name: String,
    /// 元素 id（`None` ⇒ 不写）。
    pub id: Option<String>,
    /// 占位提示（`None` ⇒ 不写）。
    pub placeholder: Option<String>,
    /// 可见行数（`None` ⇒ 不写 `rows` ⇒ 交给宿主 CSS 决定）。
    pub rows: Option<usize>,
    /// ⭐ **取值信号**（**调用方持有**）⇒ ⇒ 组件保持纯函数。
    pub value: Signal<String>,
    /// `Enter` 提交（`None` ⇒ 不挂提交处理器）。
    pub on_submit: Option<ElementHandler>,
    /// 输入事件（`None` ⇒ 不挂）。
    pub on_input: Option<ElementHandler>,
    /// 无障碍标签（`None` ⇒ 不写）。
    pub aria_label: Option<String>,
    /// 是否禁用（`false` ⇒ 不写 `disabled` 属性）。
    pub disabled: bool,
}

impl TextAreaProps {
    /// 建一组多行文本框 props（class 名与取值信号必给）。
    pub fn new(class_name: impl Into<String>, value: Signal<String>) -> Self {
        Self {
            class_name: class_name.into(),
            id: None,
            placeholder: None,
            rows: None,
            value,
            on_submit: None,
            on_input: None,
            aria_label: None,
            disabled: false,
        }
    }

    /// 设元素 id（builder 风格）。
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// 设占位提示（builder 风格）。
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = Some(placeholder.into());
        self
    }

    /// 设可见行数（builder 风格）。
    pub fn rows(mut self, rows: usize) -> Self {
        self.rows = Some(rows);
        self
    }

    /// 设 `Enter` 提交处理器（builder 风格）。
    ///
    /// ⚠️ 提交**必过** [`should_submit_multiline`] ⇒ ⇒ `Shift+Enter`（换行）
    /// **不**提交、IME 组合期的 Enter **不**提交。
    pub fn on_submit(mut self, on_submit: ElementHandler) -> Self {
        self.on_submit = Some(on_submit);
        self
    }

    /// 设输入事件处理器（builder 风格）。
    pub fn on_input(mut self, on_input: ElementHandler) -> Self {
        self.on_input = Some(on_input);
        self
    }

    /// 设无障碍标签（builder 风格）。
    pub fn aria_label(mut self, aria_label: impl Into<String>) -> Self {
        self.aria_label = Some(aria_label.into());
        self
    }

    /// 设禁用态（builder 风格）。
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

/// 多行文本框组件。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TextArea;

impl UiComponent for TextArea {
    type Props = TextAreaProps;
    type Output = VNode;

    fn render(props: &Self::Props) -> Self::Output {
        // ⭐ 读一次信号 ⇒ ⇒ **建立依赖**。
        let _ = props.value.get();

        let mut area = VNode::element("textarea").class(props.class_name.clone());
        area = apply_common_attrs(
            area,
            props.id.as_deref(),
            props.placeholder.as_deref(),
            props.aria_label.as_deref(),
            props.disabled,
        );
        if let Some(rows) = props.rows {
            area = area.attr("rows", rows.to_string());
        }
        // ⭐⭐⭐ **双向绑定**：`textarea.value` 是 **property** ⇒ ⇒ 走绑定通道
        // （`textarea` 的取值**不**放在文本子节点里 ⇒ 避免「静态文本 + 动态
        // property」两份真相）。
        area = area.bind(Binding::text(props.value.clone()));
        if let Some(on_input) = &props.on_input {
            area = area.handler(EventKind::Input, on_input.clone());
        }
        if let Some(on_submit) = &props.on_submit {
            let submit = on_submit.clone();
            area = area.handler(
                EventKind::KeyDown,
                ElementHandler::new(move |event: &mut DomEvent| {
                    // ⭐ IME 组合期不提交；`Shift+Enter` 是换行 ⇒ 也不提交。
                    if should_submit_multiline(event) {
                        submit.call(event);
                    }
                }),
            );
        }
        area.build()
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// 复选框
// ═══════════════════════════════════════════════════════════════════════════

/// 复选框（`<input type="checkbox">`）的 props。
#[derive(Debug, Clone)]
pub struct CheckboxProps {
    /// class 名（可含多个 token）—— **视觉契约由调用方给**。
    pub class_name: String,
    /// 元素 id（`None` ⇒ 不写）。
    pub id: Option<String>,
    /// ⭐ **勾选态信号**（**调用方持有**）⇒ ⇒ 组件保持纯函数。
    pub checked: Signal<bool>,
    /// 勾选变化（`None` ⇒ 不挂）。
    pub on_change: Option<ElementHandler>,
    /// 无障碍标签（`None` ⇒ 不写）。
    pub aria_label: Option<String>,
    /// 是否禁用（`false` ⇒ 不写 `disabled` 属性）。
    pub disabled: bool,
}

impl CheckboxProps {
    /// 建一组复选框 props（class 名与勾选态信号必给）。
    pub fn new(class_name: impl Into<String>, checked: Signal<bool>) -> Self {
        Self {
            class_name: class_name.into(),
            id: None,
            checked,
            on_change: None,
            aria_label: None,
            disabled: false,
        }
    }

    /// 设元素 id（builder 风格）。
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// 设勾选变化处理器（builder 风格）。
    ///
    /// ⚠️ 引擎会把**绑定写回**与**同类型的用户处理器**合成一个监听器
    /// ⇒ ⇒ 处理器里读到的信号**已经是**新值。
    pub fn on_change(mut self, on_change: ElementHandler) -> Self {
        self.on_change = Some(on_change);
        self
    }

    /// 设无障碍标签（builder 风格）。
    pub fn aria_label(mut self, aria_label: impl Into<String>) -> Self {
        self.aria_label = Some(aria_label.into());
        self
    }

    /// 设禁用态（builder 风格）。
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

/// 复选框组件。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Checkbox;

impl UiComponent for Checkbox {
    type Props = CheckboxProps;
    type Output = VNode;

    fn render(props: &Self::Props) -> Self::Output {
        // ⭐ 读一次信号 ⇒ ⇒ **建立依赖**。
        let _ = props.checked.get();

        let mut box_input = VNode::element("input")
            .attr("type", "checkbox")
            .class(props.class_name.clone());
        box_input = apply_common_attrs(
            box_input,
            props.id.as_deref(),
            None,
            props.aria_label.as_deref(),
            props.disabled,
        );
        // ⭐⭐⭐ **双向绑定**：`checked` 是**布尔 property** ⇒ ⇒ 必须走
        // [`Binding::checked`]（写字符串 `"false"` 会让真值判断反过来）。
        box_input = box_input.bind(Binding::checked(props.checked.clone()));
        if let Some(on_change) = &props.on_change {
            box_input = box_input.handler(EventKind::Change, on_change.clone());
        }
        box_input.build()
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// 下拉
// ═══════════════════════════════════════════════════════════════════════════

/// 下拉选项（**取值, 标签**）。
///
/// ⚠️ **顺序固定**：先是 `value`（写进 `<option>` 的 `value` **属性**），
/// 后是 `label`（`<option>` 的文本子节点）。
pub type SelectOption = (String, String);

/// 下拉（`<select>`）的 props。
#[derive(Debug, Clone)]
pub struct SelectProps {
    /// class 名（可含多个 token）—— **视觉契约由调用方给**。
    pub class_name: String,
    /// 元素 id（`None` ⇒ 不写）。
    pub id: Option<String>,
    /// 选项清单（**取值, 标签**）。
    pub options: Vec<SelectOption>,
    /// ⭐ **取值信号**（**调用方持有**）⇒ ⇒ 组件保持纯函数。
    pub value: Signal<String>,
    /// 选择变化（`None` ⇒ 不挂）。
    pub on_change: Option<ElementHandler>,
    /// 无障碍标签（`None` ⇒ 不写）。
    pub aria_label: Option<String>,
    /// 是否禁用（`false` ⇒ 不写 `disabled` 属性）。
    pub disabled: bool,
}

impl SelectProps {
    /// 建一组下拉 props（class 名与取值信号必给）。
    pub fn new(
        class_name: impl Into<String>,
        options: Vec<SelectOption>,
        value: Signal<String>,
    ) -> Self {
        Self {
            class_name: class_name.into(),
            id: None,
            options,
            value,
            on_change: None,
            aria_label: None,
            disabled: false,
        }
    }

    /// 设元素 id（builder 风格）。
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// 设选择变化处理器（builder 风格）。
    ///
    /// ⚠️ 处理器被调用时，**信号已经是新值**（组件先写回信号，再调宿主）。
    pub fn on_change(mut self, on_change: ElementHandler) -> Self {
        self.on_change = Some(on_change);
        self
    }

    /// 设无障碍标签（builder 风格）。
    pub fn aria_label(mut self, aria_label: impl Into<String>) -> Self {
        self.aria_label = Some(aria_label.into());
        self
    }

    /// 设禁用态（builder 风格）。
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

/// 下拉组件。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Select;

impl UiComponent for Select {
    type Props = SelectProps;
    type Output = VNode;

    fn render(props: &Self::Props) -> Self::Output {
        // ⭐ 读一次信号 ⇒ ⇒ **建立依赖**（取值变了 ⇒ 选中项要重标）。
        let current = props.value.get();

        let mut select = VNode::element("select").class(props.class_name.clone());
        select = apply_common_attrs(
            select,
            props.id.as_deref(),
            None,
            props.aria_label.as_deref(),
            props.disabled,
        );
        // ⭐ `<select>` 的取值是 **property**（它**没有** `value` 内容属性
        // ⇒ 写属性无效）⇒ ⇒ 用 `prop("value", …)` 声明。
        select = select.prop("value", current.clone());
        // ⭐⭐⭐ **双向绑定**（与其它三个控件同一条引擎机制）。
        select = select.bind(Binding::text(props.value.clone()));
        // ⭐ `change` 处理器：**先**把 DOM 取值写回信号，**再**调宿主回调
        // ⇒ ⇒ 宿主回调里读到的信号**已经是**新值。
        //
        // ⚠️ 为什么**还要**自己写一次（而不是只靠绑定的 `input` 通道）：
        // 引擎的 `bind:value` 写回挂在 **`input`** 事件上 ⇒ ⇒ 若宿主环境
        // 只派发 `change`（不派发 `input`），绑定通道就**收不到** ⇒ ⇒ 信号
        // 会停在旧值 ⇒ ⇒ 这里补一条 `change` 路径兜底（`set_if_changed`
        // ⇒ ⇒ 重复写**不**产生额外重渲染）。
        {
            let signal = props.value.clone();
            let host = props.on_change.clone();
            select = select.handler(
                EventKind::Change,
                ElementHandler::new(move |event: &mut DomEvent| {
                    if let Some(value) = event.target_value() {
                        signal.set_if_changed(value.to_owned());
                    }
                    if let Some(host) = host.as_ref() {
                        host.call(event);
                    }
                }),
            );
        }
        for (value, label) in &props.options {
            let mut option = VNode::element("option")
                // ⭐ `<option>` 的 `value` 是**静态内容属性** ⇒ 走 `attr`。
                .attr("value", value.clone())
                // ⭐ 命中项标 `selected`（**布尔 property**）⇒ ⇒ 覆盖
                // 「选项被重建、取值没变」时 `<select>.value` **不**被重写
                // 的差分场景 ⇒ ⇒ 选中项不会跑回第一项。
                .prop("selected", value == &current);
            option = option.child(VNode::text(label.clone()));
            select = select.child(option.build());
        }
        select.build()
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// 测试
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_ui_core::Runtime;
    use apeireth_ui_dom::{AttrValue, BindingSlot, ElementVNode, PropValue};
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;

    /// 取虚拟树里的元素节点（非元素 ⇒ 直接失败）。
    fn as_element(node: &VNode) -> &ElementVNode {
        match node {
            VNode::Element(element) => element,
            other => panic!("实测节点形态：{other:?}"),
        }
    }

    /// 取某个事件类型的处理器。
    fn handler_of(node: &VNode, kind: EventKind) -> Option<ElementHandler> {
        as_element(node)
            .handlers
            .iter()
            .find(|(event_kind, _)| *event_kind == kind)
            .map(|(_, handler)| handler.clone())
    }

    /// 文本属性的读取（便于断言）。
    fn attr_text(node: &VNode, name: &str) -> Option<String> {
        match as_element(node).attr_value(name) {
            Some(AttrValue::Text(text)) => Some(text.clone()),
            Some(AttrValue::Bool(flag)) => Some(flag.to_string()),
            None => None,
        }
    }

    // ── IME 判据（⭐ 本批的核心可测判据） ─────────────────────────────────

    /// ⭐⭐⭐ **IME 判据**：组合期的 Enter **不**触发提交。
    ///
    /// ## 它锁住什么
    ///
    /// 中文输入法组合期里用户按 `Enter` 是「**确认候选**」⇒ ⇒ **不是**「提交」
    /// ⇒ ⇒ 不判 ⇒ ⇒ **误提交**（把半成品拼音发出去）。
    ///
    /// ⇒ ⇒ 判据抽成**纯函数** ⇒ ⇒ 这条断言**不需要浏览器、不需要真实 IME**
    /// ⇒ ⇒ 判据被改坏 ⇒ ⇒ 这条测试**立刻红**。
    #[test]
    fn the_enter_of_an_ime_composition_does_not_submit() {
        let composing = DomEvent::new(EventKind::KeyDown, vec![0])
            .with_key("Enter")
            .with_is_composing(true);
        assert!(
            !should_submit(&composing),
            "⭐ 组合期的 Enter 是「选词」，不是「提交」"
        );
        assert!(
            !should_submit_multiline(&composing),
            "⭐ 多行文本框里，组合期的 Enter 同样不提交"
        );

        // ⭐ 普通 Enter ⇒ 提交。
        let plain = DomEvent::new(EventKind::KeyDown, vec![0]).with_key("Enter");
        assert!(should_submit(&plain), "普通 Enter 必须提交");
        assert!(
            should_submit_multiline(&plain),
            "多行文本框里普通 Enter 必须提交"
        );

        // ⭐ 非 Enter 键 ⇒ 不提交。
        let other = DomEvent::new(EventKind::KeyDown, vec![0]).with_key("a");
        assert!(!should_submit(&other), "非 Enter 键不得提交");
        assert!(!is_enter_key(&other));

        // ⭐ 组合期的**其它**键 ⇒ 不提交。
        let composing_other = DomEvent::new(EventKind::KeyDown, vec![0])
            .with_key("a")
            .with_is_composing(true);
        assert!(!should_submit(&composing_other));
    }

    /// ⭐ `Shift+Enter`：**单行**提交、**多行**换行（不提交）。
    #[test]
    fn shift_enter_submits_a_single_line_field_but_only_newlines_a_textarea() {
        let shift_enter = DomEvent::new(EventKind::KeyDown, vec![0])
            .with_key("Enter")
            .with_modifiers(false, true, false);
        assert!(
            should_submit(&shift_enter),
            "单行输入框里 Shift+Enter 同样提交（浏览器口径）"
        );
        assert!(
            !should_submit_multiline(&shift_enter),
            "⭐ 多行文本框里 Shift+Enter 是换行 ⇒ 不得提交"
        );
    }

    // ── 单行文本框 ─────────────────────────────────────────────────────────

    #[test]
    fn text_field_renders_an_input_with_the_given_class_id_and_type() {
        let runtime = Runtime::new();
        let signal = runtime.signal(String::from("初值"));
        let node = TextField::render(
            &TextFieldProps::new("ap-input", signal)
                .id("ap-input-name")
                .placeholder("请输入名称")
                .aria_label("名称"),
        );

        assert_eq!(node.tag(), Some("input"));
        assert_eq!(attr_text(&node, "type").as_deref(), Some("text"));
        assert_eq!(attr_text(&node, "class").as_deref(), Some("ap-input"));
        assert_eq!(attr_text(&node, "id").as_deref(), Some("ap-input-name"));
        assert_eq!(
            attr_text(&node, "placeholder").as_deref(),
            Some("请输入名称")
        );
        assert_eq!(attr_text(&node, "aria-label").as_deref(), Some("名称"));
        assert!(
            attr_text(&node, "disabled").is_none(),
            "未禁用 ⇒ 不写 disabled 属性"
        );
    }

    /// ⭐⭐⭐ **绑定接对了**（且**双向**通）。
    #[test]
    fn text_field_binds_the_callers_signal_in_both_directions() {
        let runtime = Runtime::new();
        let signal = runtime.signal(String::from("初值"));
        let node = TextField::render(&TextFieldProps::new("ap-input", signal.clone()));

        let element = as_element(&node);
        assert_eq!(element.bindings.len(), 1, "必须挂一个绑定");
        let binding = &element.bindings[0];
        assert_eq!(binding.slot(), BindingSlot::Value, "绑定槽位必须是 value");
        assert_eq!(
            binding.signal_id(),
            signal.id(),
            "⭐ 绑的必须是调用方给的那个信号"
        );
        assert_eq!(binding.value(), "初值", "信号 → DOM 的取值来源");

        // DOM → 信号：事件载荷里的取值写回信号。
        let event = DomEvent::new(EventKind::Input, vec![0]).with_target_value("用户敲的");
        assert!(binding.write_back_from_event(&event));
        assert_eq!(signal.get(), "用户敲的", "DOM → 信号方向必须通");
    }

    /// ⭐ Enter 提交：普通 Enter 提交、**组合期的 Enter 不提交**。
    #[test]
    fn text_field_submits_on_enter_but_not_on_an_ime_enter() {
        let runtime = Runtime::new();
        let signal = runtime.signal(String::new());
        let submitted = Rc::new(Cell::new(0usize));
        let counter = submitted.clone();
        let node = TextField::render(&TextFieldProps::new("ap-input", signal).on_submit(
            ElementHandler::new(move |_event| {
                counter.set(counter.get() + 1);
            }),
        ));

        let handler = handler_of(&node, EventKind::KeyDown).expect("必须挂 KeyDown 处理器");

        let mut enter = DomEvent::new(EventKind::KeyDown, vec![0]).with_key("Enter");
        handler.call(&mut enter);
        assert_eq!(submitted.get(), 1, "普通 Enter 必须提交");

        let mut composing = DomEvent::new(EventKind::KeyDown, vec![0])
            .with_key("Enter")
            .with_is_composing(true);
        handler.call(&mut composing);
        assert_eq!(submitted.get(), 1, "⭐ 组合期的 Enter 不得触发提交");

        let mut other = DomEvent::new(EventKind::KeyDown, vec![0]).with_key("a");
        handler.call(&mut other);
        assert_eq!(submitted.get(), 1, "非 Enter 键不得触发提交");
    }

    #[test]
    fn text_field_without_a_submit_handler_attaches_no_key_handler() {
        let runtime = Runtime::new();
        let signal = runtime.signal(String::new());
        let node = TextField::render(&TextFieldProps::new("ap-input", signal));
        assert!(
            handler_of(&node, EventKind::KeyDown).is_none(),
            "没有提交处理器 ⇒ 不挂 KeyDown 处理器"
        );
    }

    // ── 多行文本框 ─────────────────────────────────────────────────────────

    #[test]
    fn text_area_renders_a_textarea_with_rows_and_placeholder() {
        let runtime = Runtime::new();
        let signal = runtime.signal(String::from("初值"));
        let node = TextArea::render(
            &TextAreaProps::new("ap-textarea", signal)
                .id("ap-desc")
                .placeholder("描述")
                .rows(6),
        );

        assert_eq!(node.tag(), Some("textarea"));
        assert_eq!(attr_text(&node, "class").as_deref(), Some("ap-textarea"));
        assert_eq!(attr_text(&node, "id").as_deref(), Some("ap-desc"));
        assert_eq!(attr_text(&node, "placeholder").as_deref(), Some("描述"));
        assert_eq!(attr_text(&node, "rows").as_deref(), Some("6"));
        assert!(
            node.children().is_empty(),
            "⭐ textarea 的取值在 property 上 ⇒ 不得再放静态文本子节点（两份真相）"
        );
    }

    #[test]
    fn text_area_binds_the_callers_signal_in_both_directions() {
        let runtime = Runtime::new();
        let signal = runtime.signal(String::from("初值"));
        let node = TextArea::render(&TextAreaProps::new("ap-textarea", signal.clone()));

        let element = as_element(&node);
        assert_eq!(element.bindings.len(), 1);
        let binding = &element.bindings[0];
        assert_eq!(binding.slot(), BindingSlot::Value);
        assert_eq!(binding.signal_id(), signal.id());

        let event = DomEvent::new(EventKind::Input, vec![0]).with_target_value("多行内容");
        assert!(binding.write_back_from_event(&event));
        assert_eq!(signal.get(), "多行内容");
    }

    /// ⭐ `Shift+Enter` 在多行文本框里是「换行」⇒ ⇒ 不提交。
    #[test]
    fn text_area_does_not_submit_on_shift_enter() {
        let runtime = Runtime::new();
        let signal = runtime.signal(String::new());
        let submitted = Rc::new(Cell::new(0usize));
        let counter = submitted.clone();
        let node = TextArea::render(&TextAreaProps::new("ap-textarea", signal).on_submit(
            ElementHandler::new(move |_event| {
                counter.set(counter.get() + 1);
            }),
        ));

        let handler = handler_of(&node, EventKind::KeyDown).expect("必须挂 KeyDown 处理器");

        let mut shift_enter = DomEvent::new(EventKind::KeyDown, vec![0])
            .with_key("Enter")
            .with_modifiers(false, true, false);
        handler.call(&mut shift_enter);
        assert_eq!(submitted.get(), 0, "⭐ Shift+Enter 是换行 ⇒ 不得提交");

        let mut enter = DomEvent::new(EventKind::KeyDown, vec![0]).with_key("Enter");
        handler.call(&mut enter);
        assert_eq!(submitted.get(), 1, "普通 Enter 必须提交");
    }

    // ── 复选框 ─────────────────────────────────────────────────────────────

    #[test]
    fn checkbox_renders_an_input_of_type_checkbox() {
        let runtime = Runtime::new();
        let signal = runtime.signal(false);
        let node = Checkbox::render(
            &CheckboxProps::new("ap-check", signal)
                .id("ap-check-enabled")
                .aria_label("启用"),
        );

        assert_eq!(node.tag(), Some("input"));
        assert_eq!(attr_text(&node, "type").as_deref(), Some("checkbox"));
        assert_eq!(attr_text(&node, "class").as_deref(), Some("ap-check"));
        assert_eq!(attr_text(&node, "id").as_deref(), Some("ap-check-enabled"));
        assert_eq!(attr_text(&node, "aria-label").as_deref(), Some("启用"));
    }

    /// ⭐⭐⭐ **勾选态绑定接对了**（且**双向**通）。
    #[test]
    fn checkbox_binds_the_callers_checked_signal_in_both_directions() {
        let runtime = Runtime::new();
        let signal = runtime.signal(false);
        let node = Checkbox::render(&CheckboxProps::new("ap-check", signal.clone()));

        let element = as_element(&node);
        assert_eq!(element.bindings.len(), 1);
        let binding = &element.bindings[0];
        assert_eq!(
            binding.slot(),
            BindingSlot::Checked,
            "⭐ 复选框的绑定槽位必须是 checked"
        );
        assert_eq!(binding.signal_id(), signal.id());

        // DOM → 信号：`change` 事件载荷里的 `checked`。
        let event = DomEvent::new(EventKind::Change, vec![0]).with_checked(true);
        assert!(binding.write_back_from_event(&event));
        assert!(signal.get(), "DOM → 信号方向必须通");
    }

    // ── 下拉 ───────────────────────────────────────────────────────────────

    #[test]
    fn select_renders_options_with_value_attribute_and_label_text() {
        let runtime = Runtime::new();
        let signal = runtime.signal(String::from("b"));
        let node = Select::render(&SelectProps::new(
            "ap-select",
            vec![
                (String::from("a"), String::from("甲")),
                (String::from("b"), String::from("乙")),
            ],
            signal,
        ));

        assert_eq!(node.tag(), Some("select"));
        assert_eq!(attr_text(&node, "class").as_deref(), Some("ap-select"));
        let options: Vec<&ElementVNode> = node.children().iter().map(as_element).collect();
        assert_eq!(options.len(), 2);
        assert_eq!(options[0].tag, "option");
        // ⭐ `<option>` 的 `value` 是**静态属性**。
        assert_eq!(
            options[0].attr_value("value"),
            Some(&AttrValue::Text(String::from("a")))
        );
        // ⭐ 标签是文本子节点。
        assert_eq!(options[0].children[0], VNode::text("甲"));
    }

    /// ⭐⭐⭐ **`<select>` 的取值落在 property 上**（不是属性）+ 命中项标 `selected`。
    #[test]
    fn select_declares_the_value_as_a_property_and_marks_the_selected_option() {
        let runtime = Runtime::new();
        let signal = runtime.signal(String::from("b"));
        let node = Select::render(&SelectProps::new(
            "ap-select",
            vec![
                (String::from("a"), String::from("甲")),
                (String::from("b"), String::from("乙")),
            ],
            signal,
        ));

        let element = as_element(&node);
        // ⭐ `<select>` **没有** `value` 内容属性 ⇒ 写属性无效 ⇒ 必须写 property。
        assert!(
            element.attr_value("value").is_none(),
            "⭐ select 上不得出现 value 属性（它不是内容属性）"
        );
        assert_eq!(
            element.prop_value("value"),
            Some(&PropValue::Text(String::from("b"))),
            "⭐ 取值必须落在 property 上"
        );

        let options: Vec<&ElementVNode> = node.children().iter().map(as_element).collect();
        assert_eq!(
            options[1].prop_value("selected"),
            Some(&PropValue::Bool(true)),
            "⭐ 命中项必须标 selected"
        );
        assert_eq!(
            options[0].prop_value("selected"),
            Some(&PropValue::Bool(false)),
            "未命中项必须显式标 false（差分是翻转布尔，不是 property 消失）"
        );
    }

    #[test]
    fn select_binds_the_callers_signal_in_both_directions() {
        let runtime = Runtime::new();
        let signal = runtime.signal(String::from("a"));
        let node = Select::render(&SelectProps::new(
            "ap-select",
            vec![(String::from("a"), String::from("甲"))],
            signal.clone(),
        ));

        let element = as_element(&node);
        assert_eq!(element.bindings.len(), 1);
        let binding = &element.bindings[0];
        assert_eq!(binding.slot(), BindingSlot::Value);
        assert_eq!(binding.signal_id(), signal.id());

        let event = DomEvent::new(EventKind::Input, vec![0]).with_target_value("b");
        assert!(binding.write_back_from_event(&event));
        assert_eq!(signal.get(), "b");
    }

    /// ⭐ `change`：**先**写回信号、**再**调宿主回调（宿主读到的是新值）。
    #[test]
    fn select_change_writes_the_signal_back_before_calling_the_host() {
        let runtime = Runtime::new();
        let signal = runtime.signal(String::from("a"));
        let seen = Rc::new(RefCell::new(String::new()));
        let seen_by_host = seen.clone();
        let signal_by_host = signal.clone();
        let node = Select::render(
            &SelectProps::new(
                "ap-select",
                vec![
                    (String::from("a"), String::from("甲")),
                    (String::from("b"), String::from("乙")),
                ],
                signal.clone(),
            )
            .on_change(ElementHandler::new(move |_event| {
                *seen_by_host.borrow_mut() = signal_by_host.get();
            })),
        );

        let handler = handler_of(&node, EventKind::Change).expect("必须挂 Change 处理器");
        let mut event = DomEvent::new(EventKind::Change, vec![0]).with_target_value("b");
        handler.call(&mut event);

        assert_eq!(signal.get(), "b", "change 必须把 DOM 取值写回信号");
        assert_eq!(
            seen.borrow().as_str(),
            "b",
            "⭐ 宿主回调必须看到已更新的信号"
        );
    }
}
