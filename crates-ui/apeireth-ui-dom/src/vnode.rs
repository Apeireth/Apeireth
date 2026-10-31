//! 虚拟节点（VNode）：DOM 的**纯数据结构**表示。
//!
//! 设计文档 §5.1.1：VNode 是普通 Rust 类型（可 `PartialEq`、可 `Debug`）⇒ 断言
//! 写成 `assert_eq!(vnode, expected)`，而不是「读源码文本断言」——这是消灭 38 处
//! 源码镜像测试的结构性前提。
//!
//! ## 四个分区（设计文档 §5.1.1）
//!
//! | 分区 | 载体 | 差分策略 |
//! |---|---|---|
//! | `class` | `attrs` 里名为 `class` 的项 | **按 token 差分**（增删单个 class） |
//! | `style` | `attrs` 里名为 `style` 的项 | 整体字符串比对，不同才写 |
//! | `attrs` | 其余属性 | 键级差分（新增 / 改值 / 消失） |
//! | `bindings` | [`ElementVNode::this_ref`]（`bind:this`） | 键级差分（槽位换了才重新接线） |

use crate::binding::ThisRef;
use crate::dom::EventKind;
use crate::events::ElementHandler;

/// 列表项身份（key）：不透明的可比较值（设计文档 §5.2.3）。
///
/// 由 `view!` 宏从 `key = expr` 推出（字符串 / 整数）。
/// **不支持复合 key**（本仓 0 处需要）。
///
/// > **与设计文档的一处刻意差异**：设计文档写的是 `Key::Str(&'static str)`；
/// > 本实现用**拥有所有权**的 `String`。理由：key 是**运行期**值（列表项的
/// > 标识），不可能是 `'static`；`'static str` 只够描述「编译期字面量 key」
/// > 这一种用法。`String` 是 `&'static str` 的严格超集 ⇒ 不丢能力。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Key {
    /// 字符串 key。
    Str(String),
    /// 有符号整数 key。
    Int(i64),
    /// 无符号整数 key。
    Uint(u64),
}

impl Key {
    /// 从字符串构造 key。
    pub fn str(value: impl Into<String>) -> Self {
        Key::Str(value.into())
    }

    /// 从有符号整数构造 key。
    pub fn int(value: i64) -> Self {
        Key::Int(value)
    }

    /// 从无符号整数构造 key。
    pub fn uint(value: u64) -> Self {
        Key::Uint(value)
    }
}

impl std::fmt::Display for Key {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Key::Str(text) => write!(f, "{text}"),
            Key::Int(value) => write!(f, "{value}"),
            Key::Uint(value) => write!(f, "{value}"),
        }
    }
}

impl From<&str> for Key {
    fn from(value: &str) -> Self {
        Key::Str(value.to_owned())
    }
}

impl From<String> for Key {
    fn from(value: String) -> Self {
        Key::Str(value)
    }
}

impl From<i64> for Key {
    fn from(value: i64) -> Self {
        Key::Int(value)
    }
}

impl From<u64> for Key {
    fn from(value: u64) -> Self {
        Key::Uint(value)
    }
}

/// 属性值。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AttrValue {
    /// 文本值。
    Text(String),
    /// 布尔值（`false` ⇒ **不写该属性**，而非写 `false`）。
    Bool(bool),
}

impl AttrValue {
    /// 转成「要写入 DOM 的属性值」；布尔 `false` 返回 `None`（表示不写）。
    pub fn to_attr_string(&self) -> Option<String> {
        match self {
            AttrValue::Text(text) => Some(text.clone()),
            AttrValue::Bool(flag) => {
                if *flag {
                    Some(String::from("true"))
                } else {
                    None
                }
            }
        }
    }
}

impl From<&str> for AttrValue {
    fn from(value: &str) -> Self {
        AttrValue::Text(value.to_owned())
    }
}

impl From<String> for AttrValue {
    fn from(value: String) -> Self {
        AttrValue::Text(value)
    }
}

impl From<bool> for AttrValue {
    fn from(value: bool) -> Self {
        AttrValue::Bool(value)
    }
}

/// 虚拟节点。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VNode {
    /// 元素节点。
    Element(ElementVNode),
    /// 文本节点。
    Text(TextVNode),
    /// 分组节点（无宿主节点，对应列表 / 条件块容器）。
    Fragment(Vec<VNode>),
}

impl VNode {
    /// 建一个元素节点（返回构造器）。
    pub fn element(tag: impl Into<String>) -> ElementBuilder {
        ElementBuilder::new(tag)
    }

    /// 建一个文本节点。
    pub fn text(data: impl Into<String>) -> Self {
        VNode::Text(TextVNode { data: data.into() })
    }

    /// ⭐⭐⭐ **收集「绑定信号 → 元素路径」的映射**（细粒度更新的地基）。
    ///
    /// ## 它解决什么
    ///
    /// 实测（E-137）：4.7k 元素的树里，**改一个叶子**的「差分 patch」要 **89 ms**
    /// ⇒ ⇒ 瓶颈是 **patch 的逐节点比对范围**，不是视图构造（仅 2 ms）。
    /// ⇒ ⇒ 要收窄比对范围，就必须知道「**哪个信号变 ⇒ 哪些 DOM 子树要重画**」
    /// ⇒ ⇒ 本方法给出那个映射的**第一半**（信号 → 路径）。
    ///
    /// ## ⭐ 路径可直接喂给 `Renderer::node_for_path`（**同根定义**）
    ///
    /// 实测校准（2026-10-31）：`node_for_path` 的路径根 = **tree 根** = **挂载的那棵
    /// VNode 的根** ⇒ ⇒ 与本方法返回的路径**同一个根定义** ⇒ ⇒ **无需任何前缀换算**。
    ///
    /// ## 路径语义（与 `Renderer::node_for_path` **严格一致**）
    ///
    /// 路径 = 从根到目标元素的**子节点索引**序列；索引 = 该节点 `children` 里的**序号**
    /// ⇒ ⇒ **`VNode::Fragment` 自己占一个索引**（它没有 DOM 节点 ⇒ ⇒ 定位它会
    /// `NodeNotFound`）⇒ ⇒ Fragment 的子节点路径**多一层**。
    /// （⚠️ 第一版按「展平」实现 ⇒ ⇒ 与引擎不一致 ⇒ ⇒ 被 native 测试抓出后校准。）
    ///
    /// ## 用法
    ///
    /// ```ignore
    /// let map = view.collect_binding_paths(); // (信号 id → 元素路径)
    /// ```
    pub fn collect_binding_paths(&self) -> Vec<BindingPath> {
        let mut out = Vec::new();
        let mut path = Vec::new();
        collect_paths(self, &mut path, &mut out);
        out
    }

    /// ⭐⭐ **读一遍树里所有绑定用到的信号**（⇒ 自动建立响应式依赖）。
    ///
    /// ## 为什么需要它（这是我自己踩过的坑）
    ///
    /// core 的 effect 是**依赖追踪式**的：只有在 effect 体里**读到**的信号才会被订阅。
    /// 而 `Binding::text(signal)` 只是**把信号放进 VNode**，**不读它的值**
    /// ⇒ ⇒ 只写 `Binding::text(sig)` 的视图**不会**订阅 `sig`
    /// ⇒ ⇒ 状态变了界面不动 ⇒ ⇒ 必须由屏幕作者**手动** `sig.get()` 一下
    /// ⇒ ⇒ **极易漏**（漏了就是「界面不更新」，且没有任何报错）。
    ///
    /// ⇒ 这个方法把「读一遍」**自动化**了：遍历树 ⇒ 对每个绑定读一次信号值
    /// ⇒ ⇒ 依赖被 core 收集 ⇒ ⇒ 屏幕作者**不需要**手动声明。
    ///
    /// ## 使用位置
    ///
    /// 必须在 **effect 体里**调用（依赖收集只在 effect 执行期间生效）：
    /// ```ignore
    /// runtime.effect(move || {
    ///     let view = screen.view();
    ///     view.touch_bindings();   // ← 这一行建立依赖
    ///     renderer.patch(view);
    /// });
    /// ```
    ///
    /// ## 为什么「读值」是安全的
    ///
    /// [`crate::binding::Binding::value`] 只是**读**信号（不写）⇒ ⇒
    /// 不会造成信号写回风暴，也不会把 DOM 值同步回信号（那是事件路径的事）。
    pub fn touch_bindings(&self) {
        match self {
            VNode::Element(element) => {
                for binding in &element.bindings {
                    // 只读值（建立依赖）⇒ ⇒ 丢弃结果。
                    let _ = binding.value();
                }
                for child in &element.children {
                    child.touch_bindings();
                }
            }
            VNode::Fragment(children) => {
                for child in children {
                    child.touch_bindings();
                }
            }
            VNode::Text(_) => {}
        }
    }

    /// 建一个分组节点。
    pub fn fragment(children: impl IntoIterator<Item = VNode>) -> Self {
        VNode::Fragment(children.into_iter().collect())
    }

    /// 本节点的标签（元素节点才有）。
    pub fn tag(&self) -> Option<&str> {
        match self {
            VNode::Element(element) => Some(element.tag.as_str()),
            _ => None,
        }
    }

    /// 本节点的子节点（元素节点才有）。
    pub fn children(&self) -> &[VNode] {
        match self {
            VNode::Element(element) => element.children.as_slice(),
            VNode::Fragment(children) => children.as_slice(),
            _ => &[],
        }
    }

    /// 本节点的 key（只有带 key 的元素节点有）。
    pub fn key(&self) -> Option<&Key> {
        match self {
            VNode::Element(element) => element.key.as_ref(),
            _ => None,
        }
    }

    /// 本节点的属性值（非元素节点 ⇒ `None`）。
    pub fn attr_value(&self, name: &str) -> Option<&AttrValue> {
        match self {
            VNode::Element(element) => element.attr_value(name),
            _ => None,
        }
    }

    /// 本节点的 **property** 取值（非元素节点 ⇒ `None`）。
    pub fn prop_value(&self, name: &str) -> Option<&PropValue> {
        match self {
            VNode::Element(element) => element.prop_value(name),
            _ => None,
        }
    }

    /// 本节点的 class token 集合（保序；非元素节点 ⇒ 空）。
    pub fn class_tokens(&self) -> Vec<String> {
        match self.attr_value("class") {
            Some(AttrValue::Text(text)) => text.split_whitespace().map(str::to_owned).collect(),
            _ => Vec::new(),
        }
    }
}

/// 要写进 DOM 的 **property**（**不是属性**）。
///
/// ## 为什么必须有这一层
///
/// `bind:value` / `value={expr}` 在宿主模板里编译成 **property 赋值**
/// ⇒ DOM 上**没有** `value` 属性；而写 attribute 对 `input.value` **无效**，
/// 且会破坏浏览器的受控输入语义（设计 §5.4.1）。
///
/// ⇒ 元素的取值有**两条独立通道**：属性和 property。缺后者 ⇒ 原生控件
/// （`input` / `select` / `textarea` / `input[type=range]`）会按**浏览器默认值**渲染
/// （`range` 是区间中点、文本框是空、`select` 是第一个 option）
/// ⇒ 画面与宿主模板不一致，而**属性对照看不出来**（它只比属性）。
///
/// ## 为什么两种变体
///
/// - [`PropValue::Text`] ⇒ 按字符串写（`value` / `title` / …）；
/// - [`PropValue::Bool`] ⇒ 按**布尔**写（`checked` / `selected` / …）。
///
/// ⚠️ **布尔 property 不能用字符串写**：`Reflect::set(el, "checked", "false")`
/// 写进去是**非空字符串** ⇒ 真值判断为**真** ⇒ 语义**反了**。
/// 所以布尔单独一个变体，后端走 `set_property_bool`。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PropValue {
    /// 字符串 property（`value` 等）。
    Text(String),
    /// 布尔 property（`checked` 等）。
    Bool(bool),
}

impl PropValue {
    /// 文本 property。
    pub fn text(value: impl Into<String>) -> Self {
        PropValue::Text(value.into())
    }

    /// 布尔 property。
    pub fn bool(value: bool) -> Self {
        PropValue::Bool(value)
    }

    /// 是否与另一个取值相等（差分用；变体不同 ⇒ 不相等）。
    pub fn same_as(&self, other: &PropValue) -> bool {
        self == other
    }
}

impl From<String> for PropValue {
    fn from(value: String) -> Self {
        PropValue::Text(value)
    }
}

impl From<&str> for PropValue {
    fn from(value: &str) -> Self {
        PropValue::Text(value.to_owned())
    }
}

impl From<bool> for PropValue {
    fn from(value: bool) -> Self {
        PropValue::Bool(value)
    }
}

/// 元素节点。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ElementVNode {
    /// 标签名。
    pub tag: String,
    /// 列表项身份（keyed 差分用；`None` ⇒ 身份由位置决定）。
    pub key: Option<Key>,
    /// 属性（保持写入顺序 ⇒ 差分可逐字对比）。
    pub attrs: Vec<(String, AttrValue)>,
    /// **property**（与属性是两条独立通道；见 [`PropValue`]）。
    ///
    /// 写入时机：**子节点挂完之后**（`select` 的 `value` 依赖 `option` 已存在）、
    /// **节点进文档树之前** ⇒ 用户看不到中间态。
    pub props: Vec<(String, PropValue)>,
    /// `bind:this` 槽位（类型擦除；见 [`ThisRef`]）。
    ///
    /// `None` ⇒ 该元素没有元素引用绑定。
    pub this_ref: Option<ThisRef>,
    /// **property 双向绑定**（信号 ⇄ DOM property；见 [`crate::binding::Binding`]）。
    ///
    /// 与 [`Self::props`] 的区别：`props` 是**单向**的（我们给值 ⇒ 写进 DOM）；
    /// 这里的 `bindings` 是**双向**的：
    /// - **信号 → DOM**：渲染阶段写回，且**只在 DOM 当前值 ≠ 信号值时**才写
    ///   （无条件回写会打断输入法组合态与光标位置）；
    /// - **DOM → 信号**：事件处理器里写回（见 `Binding::write_back`）。
    pub bindings: Vec<crate::binding::Binding>,
    /// 事件处理器（按事件类型索引）。
    ///
    /// 处理器**按指针身份**比较（见 [`ElementHandler`]）⇒ 差分能判断
    /// 「处理器换了没有」（换了 ⇒ 重挂监听器）。
    pub handlers: Vec<(EventKind, ElementHandler)>,
    /// 子节点。
    pub children: Vec<VNode>,
}

impl ElementVNode {
    /// 建一个元素节点。
    pub fn new(tag: impl Into<String>) -> Self {
        Self {
            tag: tag.into(),
            key: None,
            attrs: Vec::new(),
            props: Vec::new(),
            this_ref: None,
            bindings: Vec::new(),
            handlers: Vec::new(),
            children: Vec::new(),
        }
    }

    /// ⭐ 追加一个 **property 双向绑定**（信号 ⇄ DOM property）。
    pub fn bind(mut self, binding: crate::binding::Binding) -> Self {
        self.bindings.push(binding);
        self
    }

    /// 设置列表项身份（keyed 差分用）。
    pub fn key(mut self, key: impl Into<Key>) -> Self {
        self.key = Some(key.into());
        self
    }

    /// 挂上 `bind:this` 槽位（元素引用绑定）。
    ///
    /// 槽位是**类型擦除**的（[`ThisRef`]）⇒ 虚拟树不必知道后端的节点类型。
    pub fn this(mut self, this_ref: ThisRef) -> Self {
        self.this_ref = Some(this_ref);
        self
    }

    /// 读 key。
    pub fn key_value(&self) -> Option<&Key> {
        self.key.as_ref()
    }

    /// 追加一个属性（同名属性覆盖，保持原位置）。
    pub fn attr(mut self, name: impl Into<String>, value: impl Into<AttrValue>) -> Self {
        let name = name.into();
        let value = value.into();
        match self.attrs.iter_mut().find(|(key, _)| *key == name) {
            Some(slot) => slot.1 = value,
            None => self.attrs.push((name, value)),
        }
        self
    }

    /// 追加一个 **property**（同名覆盖，保原位置）。
    ///
    /// 与 [`Self::attr`] 是**两条独立通道**：`attr` 写 attribute，`prop` 写 property。
    /// 原生控件的取值（`value` / `checked`）必须走本方法 —— 见 [`PropValue`]。
    pub fn prop(mut self, name: impl Into<String>, value: impl Into<PropValue>) -> Self {
        let name = name.into();
        let value = value.into();
        match self.props.iter_mut().find(|(key, _)| *key == name) {
            Some(slot) => slot.1 = value,
            None => self.props.push((name, value)),
        }
        self
    }

    /// 追加一个 class token（保序去重；class 必须**按 token** 差分，见设计 §5.2.1）。
    pub fn class(mut self, token: impl Into<String>) -> Self {
        let token = token.into();
        let existing = self
            .attrs
            .iter()
            .find(|(key, _)| key == "class")
            .map(|(_, value)| value.clone());
        match existing {
            Some(AttrValue::Text(mut text)) => {
                if text.split_whitespace().any(|item| item == token) {
                    return self;
                }
                if !text.is_empty() {
                    text.push(' ');
                }
                text.push_str(&token);
                let attrs = std::mem::take(&mut self.attrs);
                self.attrs = attrs
                    .into_iter()
                    .map(|(key, value)| {
                        if key == "class" {
                            (key, AttrValue::Text(text.clone()))
                        } else {
                            (key, value)
                        }
                    })
                    .collect();
                self
            }
            Some(_) => self,
            None => {
                self.attrs
                    .push((String::from("class"), AttrValue::Text(token)));
                self
            }
        }
    }

    /// 挂一个事件处理器（同一事件类型挂第二个 ⇒ 覆盖第一个）。
    pub fn handler(mut self, kind: EventKind, handler: ElementHandler) -> Self {
        match self.handlers.iter_mut().find(|(key, _)| *key == kind) {
            Some(slot) => slot.1 = handler,
            None => self.handlers.push((kind, handler)),
        }
        self
    }

    /// 追加子节点。
    pub fn child(mut self, child: VNode) -> Self {
        self.children.push(child);
        self
    }
    /// 追加多个子节点。
    pub fn children(mut self, children: impl IntoIterator<Item = VNode>) -> Self {
        self.children.extend(children);
        self
    }

    /// 读属性值。
    pub fn attr_value(&self, name: &str) -> Option<&AttrValue> {
        self.attrs
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value)
    }

    /// 读 **property** 取值。
    pub fn prop_value(&self, name: &str) -> Option<&PropValue> {
        self.props
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value)
    }

    /// 产出虚拟节点。
    pub fn build(self) -> VNode {
        VNode::Element(self)
    }
}

/// 文本节点。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextVNode {
    /// 文本内容。
    pub data: String,
}

/// ⭐⭐⭐ **一个绑定在树中的位置**（信号 id + 元素路径）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BindingPath {
    /// 绑定用的信号 id（`apeireth_ui_core::graph::NodeId`）。
    pub signal_id: apeireth_ui_core::graph::NodeId,
    /// 元素路径（从根到该元素的**子节点索引**序列；与 `Renderer::node_for_path` 同语义）。
    pub path: Vec<usize>,
}

/// 递归收集绑定路径。
///
/// ## ⭐ 路径语义（**必须与引擎一致**，2026-10-31 实测校准）
///
/// 引擎的路径是**挂载树路径**（`Renderer::node_for_path`）：每一层的索引 =
/// 该节点 `children` 里的**序号** ⇒ ⇒ **`VNode::Fragment` 自己占一个索引**
/// （虽然它**没有** DOM 节点 ⇒ ⇒ 定位到它会 `NodeNotFound`）
/// ⇒ ⇒ Fragment 的子节点路径要**多一层**（先到 Fragment，再到它的子节点）。
///
/// ⚠️ 第一版实现**展平**了 Fragment ⇒ ⇒ 与引擎**不一致**
/// ⇒ ⇒ 被 native 测试当场抓出（`node_for_path` 报 `NodeNotFound`）⇒ ⇒ 已按实测校准。
fn collect_paths(node: &VNode, path: &mut Vec<usize>, out: &mut Vec<BindingPath>) {
    match node {
        VNode::Element(element) => {
            for binding in &element.bindings {
                out.push(BindingPath {
                    signal_id: binding.signal_id(),
                    path: path.clone(),
                });
            }
            for (index, child) in element.children.iter().enumerate() {
                path.push(index);
                collect_paths(child, path, out);
                path.pop();
            }
        }
        VNode::Fragment(children) => {
            for (index, child) in children.iter().enumerate() {
                path.push(index);
                collect_paths(child, path, out);
                path.pop();
            }
        }
        VNode::Text(_) => {}
    }
}

/// 元素节点构造器。
#[derive(Debug, Clone)]
pub struct ElementBuilder {
    node: ElementVNode,
}

impl ElementBuilder {
    /// 建一个元素节点构造器。
    pub fn new(tag: impl Into<String>) -> Self {
        Self {
            node: ElementVNode::new(tag),
        }
    }

    /// ⭐ 追加一个 **property 双向绑定**（信号 ⇄ DOM property）。
    pub fn bind(mut self, binding: crate::binding::Binding) -> Self {
        self.node = self.node.bind(binding);
        self
    }

    /// 设置列表项身份（keyed 差分用）。
    pub fn key(mut self, key: impl Into<Key>) -> Self {
        self.node = self.node.key(key);
        self
    }

    /// 挂上 `bind:this` 槽位（元素引用绑定）。
    pub fn this(mut self, this_ref: ThisRef) -> Self {
        self.node = self.node.this(this_ref);
        self
    }

    /// 追加属性。
    pub fn attr(mut self, name: impl Into<String>, value: impl Into<AttrValue>) -> Self {
        self.node = self.node.attr(name, value);
        self
    }

    /// 追加 **property**（与属性两条独立通道；见 [`PropValue`]）。
    ///
    /// 原生控件的取值（`value` / `checked`）必须走本方法 —— 写 attribute 对
    /// `input.value` **无效**（设计 §5.4.1）。
    pub fn prop(mut self, name: impl Into<String>, value: impl Into<PropValue>) -> Self {
        self.node = self.node.prop(name, value);
        self
    }

    /// 追加 class token。
    pub fn class(mut self, token: impl Into<String>) -> Self {
        self.node = self.node.class(token);
        self
    }

    /// 挂一个事件处理器。
    pub fn handler(mut self, kind: EventKind, handler: ElementHandler) -> Self {
        self.node = self.node.handler(kind, handler);
        self
    }

    /// 追加子节点。
    pub fn child(mut self, child: VNode) -> Self {
        self.node = self.node.child(child);
        self
    }

    /// 追加多个子节点。
    pub fn children(mut self, children: impl IntoIterator<Item = VNode>) -> Self {
        self.node = self.node.children(children);
        self
    }

    /// 产出虚拟节点。
    pub fn build(self) -> VNode {
        VNode::Element(self.node)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn class_tokens_are_merged_and_deduplicated() {
        let node = ElementVNode::new("div")
            .class("a")
            .class("b")
            .class("a")
            .build();

        match node {
            VNode::Element(element) => {
                assert_eq!(
                    element.attr_value("class"),
                    Some(&AttrValue::Text(String::from("a b")))
                );
            }
            other => panic!("实测节点形态：{other:?}"),
        }
    }

    #[test]
    fn bool_false_attribute_is_not_written() {
        assert_eq!(AttrValue::Bool(false).to_attr_string(), None);
        assert_eq!(
            AttrValue::Bool(true).to_attr_string(),
            Some(String::from("true"))
        );
        assert_eq!(
            AttrValue::Text(String::from("x")).to_attr_string(),
            Some(String::from("x"))
        );
    }

    #[test]
    fn element_handlers_compare_by_pointer_identity() {
        let handler = ElementHandler::new(|_event| {});
        let same = handler.clone();
        let other = ElementHandler::new(|_event| {});

        assert_eq!(handler, same, "同一个闭包 ⇒ 同一个处理器");
        assert_ne!(handler, other, "不同闭包 ⇒ 不同处理器");
    }

    #[test]
    fn a_second_handler_of_the_same_kind_replaces_the_first() {
        let node = ElementVNode::new("button")
            .handler(EventKind::Click, ElementHandler::new(|_event| {}))
            .handler(EventKind::Click, ElementHandler::new(|_event| {}));
        assert_eq!(node.handlers.len(), 1, "同一事件类型只留一个处理器");
    }

    #[test]
    fn same_name_attribute_is_overwritten_in_place() {
        let element = ElementVNode::new("input")
            .attr("type", "text")
            .attr("value", "a")
            .attr("type", "email")
            .build();
        match element {
            VNode::Element(element) => {
                assert_eq!(element.attrs.len(), 2);
                assert_eq!(
                    element.attr_value("type"),
                    Some(&AttrValue::Text(String::from("email")))
                );
            }
            other => panic!("实测节点形态：{other:?}"),
        }
    }
}
