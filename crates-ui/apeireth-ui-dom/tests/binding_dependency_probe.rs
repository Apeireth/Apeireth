//! ⭐⭐ **绑定的依赖自动收集**（独立回归测试；不读生产源码文本）。
//!
//! ## 它钉住什么
//!
//! core 的 effect 是**依赖追踪式**的 ⇒ 只有 effect 体里**读到**的信号才被订阅。
//! 而 `Binding::text(signal)` 只把信号放进 VNode、**不读值**
//! ⇒ ⇒ 必须有一条**自动**路径（`VNode::touch_bindings`）把依赖收集起来
//! ⇒ ⇒ 否则屏幕作者得手动 `signal.get()` ⇒ ⇒ **极易漏**（漏了 = 界面不更新，且无报错）。

use std::cell::Cell;
use std::rc::Rc;

use apeireth_ui_core::Runtime;
use apeireth_ui_dom::binding::Binding;
use apeireth_ui_dom::vnode::VNode;

/// ⭐ `touch_bindings` 让 effect **订阅到**绑定里的信号。
#[test]
fn touch_bindings_establishes_reactive_dependency() {
    let runtime = Runtime::new();
    let signal = runtime.signal(String::from("初始"));
    // 视图**只**用绑定表达依赖（**不**手动 `signal.get()`）
    let view = VNode::element("input")
        .bind(Binding::text(signal.clone()))
        .build();

    let ran = Rc::new(Cell::new(0usize));
    let counter = ran.clone();
    runtime.effect(move || {
        view.touch_bindings();
        counter.set(counter.get() + 1);
    });
    runtime.flush();
    assert_eq!(ran.get(), 1, "首次执行发生在 flush");

    signal.set(String::from("变了"));
    assert_eq!(
        ran.get(),
        2,
        "⭐ 依赖必须被**自动**收集（否则屏幕得手动 get() ⇒ 极易漏 ⇒ 界面不更新）"
    );
}

/// ⭐ 嵌套子树里的绑定**也会**被收集（递归）。
#[test]
fn touch_bindings_walks_nested_children() {
    let runtime = Runtime::new();
    let signal = runtime.signal(String::from("初始"));
    let view = VNode::element("section")
        .child(
            VNode::element("div")
                .child(
                    VNode::element("input")
                        .bind(Binding::text(signal.clone()))
                        .build(),
                )
                .build(),
        )
        .build();

    let ran = Rc::new(Cell::new(0usize));
    let counter = ran.clone();
    runtime.effect(move || {
        view.touch_bindings();
        counter.set(counter.get() + 1);
    });
    runtime.flush();
    assert_eq!(ran.get(), 1);

    signal.set(String::from("变了"));
    assert_eq!(ran.get(), 2, "⭐ 嵌套子树里的绑定也必须被收集");
}

/// ⭐ `checked` 绑定同样被收集（不只是文本绑定）。
#[test]
fn touch_bindings_covers_checked_binding() {
    let runtime = Runtime::new();
    let flag = runtime.signal(false);
    let view = VNode::element("input")
        .bind(Binding::checked(flag.clone()))
        .build();

    let ran = Rc::new(Cell::new(0usize));
    let counter = ran.clone();
    runtime.effect(move || {
        view.touch_bindings();
        counter.set(counter.get() + 1);
    });
    runtime.flush();
    assert_eq!(ran.get(), 1);

    flag.set(true);
    assert_eq!(ran.get(), 2, "⭐ checked 绑定也必须被收集");
}
