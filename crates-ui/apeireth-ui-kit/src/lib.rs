//! # apeireth-ui-kit — 组件库（共享组件的 Rust 重写）
//!
//! 职责边界（设计文档 §3.3.5）：只回答「**这块界面长什么样**」，不含业务数据
//! 获取。组件是**纯函数** `fn(&Props) -> VNode` ⇒ native 下直接调用即可断言
//! 虚拟树，不需要浏览器、不需要 wasm（这是 D3 债「38 处源码镜像测试」的根
//! 解法：断言从「读源码文本」升级为「钉行为」）。
//!
//! ## 本批（P0 骨架）交付口径
//! - ✅ **已实现**：组件注册表（[`ComponentRegistry`]，契约登记的 UI 组件扩展点）、
//!   类型擦除的组件索引与按名渲染、props 类型不匹配的显式报错、按钮组件
//!   （[`Button`]）与 props builder。
//! - 🔵 **未接线**：其余共享组件（列表 / 卡片 / 审批卡等）与真实 class 名对齐
//!   —— P5 迁移批逐面完成（class 名须与被替换实现**逐字一致**，设计 §9.1）。
//!
//! ## 纪律
//! - 只出现 **class 名与结构**，不出现颜色字面量与时长字面量（设计 §2.2）；
//! - 组件里不出现业务数据获取（业务语义归 `ui-demo` 与后端契约）。

#![forbid(unsafe_code)]

pub mod button;
pub mod registry;

pub use button::{Button, ButtonProps};
pub use registry::{ComponentRegistry, RegisteredComponent, RegistryError};

#[cfg(test)]
mod tests {
    //! crate 级断言：跨模块组合行为（组件注册 → 按名渲染）。

    use super::*;
    use apeireth_ui_core::UiComponent;

    #[test]
    fn registered_button_renders_through_the_registry() {
        let mut registry = ComponentRegistry::new();
        registry
            .register_named::<Button>("button")
            .expect("首次注册成功");

        let props = ButtonProps::new("提交");
        let node = registry
            .render::<Button>("button", &props)
            .expect("渲染成功");
        assert_eq!(node.tag(), Some("button"));

        // 组件是纯函数：同样 props ⇒ 同样产物。
        assert_eq!(node, Button::render(&props));
    }
}
