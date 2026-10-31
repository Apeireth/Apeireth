//! 组件回调：类型擦除的「无参闭包」。
//!
//! 组件 props 里的「点击后要做的事」用本类型表达（例：允许 / 拒绝 / 关闭）。
//!
//! ## 为什么要有这个类型
//!
//! props 结构体由 `#[component]` 宏生成，而宏生成的结构体带
//! `#[derive(Clone, Debug)]` ⇒ **字段类型必须 `Clone` + `Debug`**。
//! `Rc<dyn Fn()>` 既不 `Debug` 也不 `PartialEq` ⇒ 直接用它做字段会让宏
//! 生成的类型编译不过。
//!
//! 本类型是那层「可 `Clone` + 可 `Debug` + 可比较」的包装：
//! - `Clone`：共享同一个闭包（浅拷贝）—— 回调语义上正确（多个视图共享
//!   同一个回调是常态）；
//! - `Debug`：手写实现，**不打印闭包内容**（闭包不可打印）；
//! - `PartialEq`：**按指针身份**比较（同一个闭包实例 ⇒ 相等）—— 与
//!   `ui-dom` 的 `ElementHandler` 同一口径。
//!
//! ## 纪律
//!
//! 回调**不认识业务语义**（允许 / 拒绝 / 关闭这些含义归 `ui-kit` / `ui-demo`
//! 层）；本类型只表达「有个可调用的东西」（设计文档 §1.3 第 12 条）。

use std::rc::Rc;

/// 组件回调（类型擦除的 `Fn()`）。
#[derive(Clone)]
pub struct Callback {
    inner: Rc<dyn Fn()>,
}

impl Callback {
    /// 包装一个闭包。
    pub fn new(handler: impl Fn() + 'static) -> Self {
        Self {
            inner: Rc::new(handler),
        }
    }

    /// 调用回调。
    pub fn call(&self) {
        (self.inner)();
    }
}

impl std::fmt::Debug for Callback {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Callback(..)")
    }
}

impl PartialEq for Callback {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.inner, &other.inner)
    }
}

impl Eq for Callback {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    #[test]
    fn callback_invokes_the_wrapped_closure() {
        let hits = Rc::new(Cell::new(0u32));
        let counter = hits.clone();
        let callback = Callback::new(move || counter.set(counter.get() + 1));

        callback.call();
        callback.call();

        assert_eq!(hits.get(), 2);
    }

    #[test]
    fn cloned_callback_shares_the_same_closure() {
        let hits = Rc::new(Cell::new(0u32));
        let counter = hits.clone();
        let callback = Callback::new(move || counter.set(counter.get() + 1));
        let clone = callback.clone();

        clone.call();

        assert_eq!(hits.get(), 1, "克隆共享同一个闭包");
    }

    #[test]
    fn equality_is_pointer_identity() {
        let first = Callback::new(|| {});
        let second = Callback::new(|| {});
        assert_eq!(first, first.clone(), "同一实例 ⇒ 相等");
        assert_ne!(first, second, "不同实例 ⇒ 不等");
    }

    #[test]
    fn debug_does_not_leak_the_closure_body() {
        let callback = Callback::new(|| {});
        assert_eq!(format!("{callback:?}"), "Callback(..)");
    }
}
