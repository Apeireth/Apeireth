//! 双向绑定：信号 ⇄ DOM property。
//!
//! 设计文档 §5.4.3（易错点，写死）：
//! - **信号 → DOM** 的写回发生在渲染阶段，且**只在 DOM 当前值 ≠ 信号值时**才写
//!   （无条件回写会打断输入法组合态与光标位置）；
//! - **DOM → 信号** 的写回发生在事件处理器里（即写回路径），随后由调度器统一
//!   触发一次渲染 ⇒ 输入框内容不会被「回写再被信号覆盖」地抖动。
//!
//! P0 骨架阶段：`bind:this`（元素引用绑定）未实现 —— 真实元素引用要等 DOM 后端
//! 接入后才有意义（设计 §5.4.1）。

use apeireth_ui_core::Signal;

/// 双向绑定的槽位。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BindingSlot {
    /// `bind:value`（文本 / 数字输入）。
    Value,
    /// `bind:checked`（勾选框）。
    Checked,
}

impl BindingSlot {
    /// 槽位对应的 DOM property 名。
    pub fn property_name(&self) -> &'static str {
        match self {
            BindingSlot::Value => "value",
            BindingSlot::Checked => "checked",
        }
    }
}

/// 绑定描述：信号与 DOM property 的双向通道。
#[derive(Debug)]
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

#[cfg(test)]
mod tests {
    use super::*;
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
        assert_eq!(binding.slot().property_name(), "checked");
    }
}
