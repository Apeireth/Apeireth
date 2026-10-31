//! 双向绑定：信号 ⇄ DOM property。
//!
//! 设计文档 §5.4.3（易错点，写死）：
//! - **信号 → DOM** 的写回发生在渲染阶段，且**只在 DOM 当前值 ≠ 信号值时**才写
//!   （无条件回写会打断输入法组合态与光标位置）；
//! - **DOM → 信号** 的写回发生在事件处理器里（即写回路径），随后由调度器统一
//!   触发一次渲染 ⇒ 输入框内容不会被「回写再被信号覆盖」地抖动。
//!
//! `bind:this`（元素 / 实例引用绑定）见 [`ThisBinding`]：它**不是**双向数据
//! 绑定，而是「挂载 ⇒ 写引用 / 卸载前 ⇒ 清引用」的生命周期通道。

use std::any::Any;
use std::rc::Rc;

use apeireth_ui_core::{InstanceRef, Signal};

use crate::dom::{Dom, DomError};

/// 双向绑定的槽位。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BindingSlot {
    /// `bind:value`（文本 / 数字输入）。
    Value,
    /// `bind:checked`（勾选框）。
    Checked,
    /// `bind:this`（元素 / 实例引用；**不是** property 绑定）。
    This,
}

impl BindingSlot {
    /// 槽位对应的 DOM property 名。
    ///
    /// `bind:this` 不是 property ⇒ 返回 `None`。
    pub fn property_name(&self) -> Option<&'static str> {
        match self {
            BindingSlot::Value => Some("value"),
            BindingSlot::Checked => Some("checked"),
            BindingSlot::This => None,
        }
    }
}

/// 绑定描述：信号与 DOM property 的双向通道。
#[derive(Debug, Clone)]
pub enum Binding {
    /// 文本绑定。
    Text {
        /// 绑定槽位。
        slot: BindingSlot,
        /// 绑定的信号。
        signal: Signal<String>,
    },
    /// 布尔绑定。
    Checked {
        /// 绑定槽位。
        slot: BindingSlot,
        /// 绑定的信号。
        signal: Signal<bool>,
    },
}

/// ⭐ `Binding` 的相等语义：**同槽位 + 同信号**才算同一个绑定。
///
/// ⇒ 理由：`Binding` 持有 `Signal`（响应式状态），它的**身份**是「哪个槽位接哪个信号」
/// ⇒ ⇒ 差分时才能判断「绑定换了没有」（换了 ⇒ 需要重新写回/重新接线）。
impl PartialEq for Binding {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (
                Self::Text {
                    slot: a,
                    signal: a_signal,
                },
                Self::Text {
                    slot: b,
                    signal: b_signal,
                },
            ) => a == b && a_signal.id() == b_signal.id(),
            (
                Self::Checked {
                    slot: a,
                    signal: a_signal,
                },
                Self::Checked {
                    slot: b,
                    signal: b_signal,
                },
            ) => a == b && a_signal.id() == b_signal.id(),
            _ => false,
        }
    }
}

impl Eq for Binding {}

impl Binding {
    /// 建一个文本绑定（`bind:value`）。
    pub fn text(signal: Signal<String>) -> Self {
        Binding::Text {
            slot: BindingSlot::Value,
            signal,
        }
    }

    /// 建一个布尔绑定（`bind:checked`）。
    pub fn checked(signal: Signal<bool>) -> Self {
        Binding::Checked {
            slot: BindingSlot::Checked,
            signal,
        }
    }

    /// 绑定槽位。
    pub fn slot(&self) -> BindingSlot {
        match self {
            Binding::Text { slot, .. } | Binding::Checked { slot, .. } => *slot,
        }
    }

    /// DOM → 信号：事件回调里直接写信号；返回是否真的变了。
    pub fn write_back(&self, value: &str) -> bool {
        match self {
            Binding::Text { signal, .. } => signal.set_if_changed(value.to_owned()),
            Binding::Checked { signal, .. } => signal.set_if_changed(value == "true"),
        }
    }

    /// 信号 → DOM：判断是否需要写回（DOM 当前值 ≠ 信号值才写）。
    ///
    /// 读取信号会**登记依赖** ⇒ 渲染 effect 会因绑定信号变化而重跑（设计 §5.4.3）。
    pub fn needs_write(&self, current: &str) -> bool {
        self.value() != current
    }

    /// ⭐⭐ **从事件载荷写回信号**（DOM → 信号的上行路径）。
    ///
    /// ## 为什么用「事件载荷」而不是回读 DOM
    ///
    /// 事件处理器**拿不到 `&Dom`** ⇒ ⇒ 回读 DOM 做不到
    /// ⇒ ⇒ 浏览器侧在派发时把 `input.value` / `change.checked` **填进事件载荷**
    /// （见 `WebDom` 的载荷填充）⇒ ⇒ 处理器直接用载荷即可 ⇒ ⇒ **不产生第二份真相**。
    ///
    /// ## 返回值
    ///
    /// `true` = 信号确实被更新；`false` = 载荷里没有对应值（**不写**，
    /// 避免用「空字符串」覆盖用户输入）。
    pub fn write_back_from_event(&self, event: &crate::events::DomEvent) -> bool {
        match self {
            Self::Text { signal, .. } => match event.target_value() {
                Some(value) => {
                    let changed = signal.get() != value;
                    signal.set(value.to_owned());
                    changed
                }
                None => false,
            },
            Self::Checked { signal, .. } => match event.checked() {
                Some(flag) => {
                    let changed = signal.get() != flag;
                    signal.set(flag);
                    changed
                }
                None => false,
            },
        }
    }

    /// 信号当前值（写回 DOM 时用）。
    pub fn value(&self) -> String {
        match self {
            Binding::Text { signal, .. } => signal.get(),
            Binding::Checked { signal, .. } => {
                if signal.get() {
                    String::from("true")
                } else {
                    String::from("false")
                }
            }
        }
    }
}

/// `bind:this` 绑定：元素 / 组件实例的**引用**通道（设计文档 §5.4.1）。
///
/// **不是双向数据绑定** —— 它只承载一条生命周期通道：
///
/// | 时机 | 动作 |
/// |---|---|
/// | [`ThisBinding::mount`] | 把元素句柄**写入**引用槽位 |
/// | [`ThisBinding::unmount`] | 把引用槽位**清空** |
///
/// 「卸载前清空」是硬要求：槽位里若留着已销毁的节点句柄，调用方会在
/// 「看起来还在」的节点上读出过期状态。
///
/// 槽位本体是 [`apeireth_ui_core::InstanceRef`]（响应式核心里的通用形态），
/// 本类型只是它对 DOM 后端的特化。
#[derive(Debug)]
pub struct ThisBinding<D: Dom> {
    slot: Rc<InstanceRef<D::Node>>,
    /// 同一槽位的**类型擦除视图**（虚拟树侧持有；见 [`ThisRef`]）。
    erased: ThisRef,
}

impl<D: Dom> Clone for ThisBinding<D> {
    fn clone(&self) -> Self {
        Self {
            slot: self.slot.clone(),
            erased: self.erased.clone(),
        }
    }
}

impl<D: Dom> ThisBinding<D> {
    /// 建一个未绑定的 `bind:this`。
    pub fn new() -> Self {
        let slot = Rc::new(InstanceRef::new());
        let erased = ThisRef {
            // 类型参数必须显式标注：擦除视图的 `D` 只出现在 trait object 里，
            // 编译器无法从 `Rc<dyn ErasedInstance>` 反推（wasm 目标下报错 E0283）。
            inner: Rc::new(TypedInstance::<D> { slot: slot.clone() }),
        };
        Self { slot, erased }
    }

    /// 读取引用的元素句柄（未挂载 ⇒ `None`）。
    pub fn get(&self) -> Option<D::Node>
    where
        D::Node: Clone,
    {
        self.slot.get()
    }

    /// 当前是否已绑定。
    pub fn is_bound(&self) -> bool {
        self.slot.is_bound()
    }

    /// 挂载：写入元素句柄。
    pub fn mount(&self, node: D::Node) {
        self.slot.bind(node);
    }

    /// 卸载：**清空**引用（幂等）。
    pub fn unmount(&self) {
        self.slot.unbind();
    }

    /// 取出底层的通用引用槽位（供上层组合用）。
    pub fn slot(&self) -> Rc<InstanceRef<D::Node>> {
        self.slot.clone()
    }

    /// 导出**类型擦除**的引用句柄（放进 `VNode` 用）。
    ///
    /// 与本绑定**共享同一个槽位** ⇒ 渲染器写入虚拟树侧的句柄时，
    /// [`ThisBinding::get`] 立刻能读到节点。
    pub fn erased(&self) -> ThisRef {
        self.erased.clone()
    }
}

/// `bind:this` 的**类型擦除**引用句柄（虚拟树侧持有）。
///
/// ## 为什么需要擦除
///
/// [`crate::vnode::VNode`] 不是 `D: Dom` 的泛型（它是**纯数据结构**，设计文档
/// §5.1.1），而节点句柄的类型是 `D::Node` ⇒ 在虚拟树边界必须把节点类型
/// 擦除。擦除后 `VNode` 仍是 `Clone + PartialEq + Eq`（身份按**槽位**比较：
/// 同一个槽位 = 同一个身份）。
///
/// ## 擦除不是降级
///
/// 类型不符（槽位与后端节点类型不一致）⇒ 显式报 [`DomError::Unsupported`]，
/// **不静默丢弃**（0 装纪律）。
#[derive(Clone)]
pub struct ThisRef {
    inner: Rc<dyn ErasedInstance>,
}

impl std::fmt::Debug for ThisRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ThisRef").finish_non_exhaustive()
    }
}

impl PartialEq for ThisRef {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.inner, &other.inner)
    }
}

impl Eq for ThisRef {}

impl ThisRef {
    /// 挂载：写入节点句柄（类型擦除）。
    ///
    /// 由渲染器调用（crate 内部）—— 类型不符 ⇒ 显式报错。
    pub(crate) fn bind_node(&self, node: Box<dyn Any>) -> Result<(), DomError> {
        self.inner.bind_any(node)
    }

    /// 卸载：清空引用（幂等）。
    pub(crate) fn unbind(&self) {
        self.inner.unbind_any();
    }
}

/// 类型擦除的引用槽（内部契约）。
trait ErasedInstance {
    /// 挂载：写入节点句柄（类型擦除）。类型不符 ⇒ 显式报错。
    fn bind_any(&self, node: Box<dyn Any>) -> Result<(), DomError>;

    /// 卸载：清空引用。
    fn unbind_any(&self);
}

/// 类型化的引用槽视图：把 `InstanceRef<D::Node>` 擦除后暴露给虚拟树。
struct TypedInstance<D: Dom> {
    slot: Rc<InstanceRef<D::Node>>,
}

impl<D: Dom> ErasedInstance for TypedInstance<D> {
    fn bind_any(&self, node: Box<dyn Any>) -> Result<(), DomError> {
        let handle = node.downcast::<D::Node>().map_err(|_| {
            DomError::Unsupported(String::from(
                "bind:this 槽位与后端节点类型不符（擦除后无法还原）",
            ))
        })?;
        self.slot.bind(*handle);
        Ok(())
    }

    fn unbind_any(&self) {
        self.slot.unbind();
    }
}

impl<D: Dom> Default for ThisBinding<D> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dom::{DomError, InMemoryDom};
    use apeireth_ui_core::Runtime;

    #[test]
    fn write_back_updates_the_signal_only_on_change() {
        let runtime = Runtime::new();
        let signal = runtime.signal(String::from("a"));
        let binding = Binding::text(signal.clone());

        assert!(binding.write_back("b"), "值变了 ⇒ 写信号");
        assert_eq!(signal.get(), "b");
        assert!(!binding.write_back("b"), "值没变 ⇒ 不写（避免抖动）");
    }

    #[test]
    fn needs_write_is_false_when_dom_already_matches() {
        let runtime = Runtime::new();
        let signal = runtime.signal(String::from("a"));
        let binding = Binding::text(signal.clone());

        assert!(!binding.needs_write("a"), "DOM 当前值已等于信号 ⇒ 不写回");
        assert!(binding.needs_write("stale"), "DOM 当前值过期 ⇒ 写回");

        signal.set(String::from("z"));
        assert!(binding.needs_write("a"));
        assert_eq!(binding.value(), "z");
    }

    #[test]
    fn checked_binding_round_trips_booleans() {
        let runtime = Runtime::new();
        let signal = runtime.signal(false);
        let binding = Binding::checked(signal.clone());

        assert!(binding.write_back("true"));
        assert!(signal.get());
        assert_eq!(binding.value(), "true");
        assert_eq!(binding.slot(), BindingSlot::Checked);
        assert_eq!(binding.slot().property_name(), Some("checked"));
    }

    #[test]
    fn value_binding_slot_reports_its_property_name() {
        assert_eq!(BindingSlot::Value.property_name(), Some("value"));
        assert_eq!(
            BindingSlot::This.property_name(),
            None,
            "bind:this 不是 property 绑定"
        );
    }

    // ── bind:this ─────────────────────────────────────────────────────────

    #[test]
    fn this_binding_is_empty_before_mount() {
        let binding: ThisBinding<InMemoryDom> = ThisBinding::new();
        assert!(!binding.is_bound());
        assert_eq!(binding.get(), None);
    }

    #[test]
    fn this_binding_writes_the_node_on_mount() {
        let dom = InMemoryDom::new();
        let node = dom.create_element("canvas").expect("建节点必须成功");
        let binding: ThisBinding<InMemoryDom> = ThisBinding::new();

        binding.mount(node);
        assert!(binding.is_bound());
        assert_eq!(binding.get(), Some(node));
    }

    #[test]
    fn this_binding_clears_the_reference_before_unmount() {
        let dom = InMemoryDom::new();
        let node = dom.create_element("canvas").expect("建节点必须成功");
        let binding: ThisBinding<InMemoryDom> = ThisBinding::new();

        binding.mount(node);
        binding.unmount();
        assert!(!binding.is_bound(), "卸载前必须清空引用");
        assert_eq!(binding.get(), None);
    }

    #[test]
    fn this_binding_unmount_is_idempotent() {
        let dom = InMemoryDom::new();
        let node = dom.create_element("canvas").expect("建节点必须成功");
        let binding: ThisBinding<InMemoryDom> = ThisBinding::new();

        binding.mount(node);
        binding.unmount();
        binding.unmount();
        assert!(!binding.is_bound());
    }

    #[test]
    fn this_binding_clones_share_one_reference() {
        let dom = InMemoryDom::new();
        let node = dom.create_element("canvas").expect("建节点必须成功");
        let binding: ThisBinding<InMemoryDom> = ThisBinding::new();
        let clone = binding.clone();

        binding.mount(node);
        assert_eq!(clone.get(), Some(node), "克隆共享同一引用槽位");
        clone.unmount();
        assert!(!binding.is_bound());
    }

    // ── P2：类型擦除的引用句柄 ────────────────────────────────────────────

    #[test]
    fn this_ref_identity_follows_the_slot_not_the_content() {
        let first: ThisBinding<InMemoryDom> = ThisBinding::new();
        let second: ThisBinding<InMemoryDom> = ThisBinding::new();

        assert_eq!(first.erased(), first.erased(), "同一槽位 = 同一身份");
        assert_ne!(first.erased(), second.erased(), "不同槽位 = 不同身份");
    }

    #[test]
    fn the_erased_view_and_the_typed_binding_share_one_slot() {
        let dom = InMemoryDom::new();
        let node = dom.create_element("canvas").expect("建节点必须成功");
        let binding: ThisBinding<InMemoryDom> = ThisBinding::new();
        let erased = binding.erased();

        erased
            .bind_node(Box::new(node))
            .expect("擦除视图接线必须成功");

        assert_eq!(
            binding.get(),
            Some(node),
            "擦除视图写进去的节点，类型化绑定立刻能读到"
        );
        assert!(binding.is_bound());
    }

    #[test]
    fn binding_a_node_of_the_wrong_type_is_refused_outright() {
        let binding: ThisBinding<InMemoryDom> = ThisBinding::new();
        let erased = binding.erased();

        let err = erased
            .bind_node(Box::new(42usize))
            .expect_err("类型不符必须显式报错");
        assert!(matches!(err, DomError::Unsupported(_)), "实测：{err:?}");
        assert!(!binding.is_bound(), "被拒绝的写不得留下痕迹");
    }

    #[test]
    fn unbinding_the_erased_view_clears_the_typed_binding() {
        let dom = InMemoryDom::new();
        let node = dom.create_element("canvas").expect("建节点必须成功");
        let binding: ThisBinding<InMemoryDom> = ThisBinding::new();
        let erased = binding.erased();

        erased.bind_node(Box::new(node)).expect("接线必须成功");
        assert!(binding.is_bound());

        erased.unbind();
        assert!(!binding.is_bound(), "擦除视图清引用 ⇒ 类型化绑定同步清空");
        assert_eq!(binding.get(), None);
    }

    #[test]
    fn this_binding_remount_replaces_the_reference() {
        let dom = InMemoryDom::new();
        let first = dom.create_element("canvas").expect("建节点必须成功");
        let second = dom.create_element("canvas").expect("建节点必须成功");
        let binding: ThisBinding<InMemoryDom> = ThisBinding::new();

        binding.mount(first);
        binding.mount(second);
        assert_eq!(binding.get(), Some(second), "后挂载者拥有该引用");
    }
}
