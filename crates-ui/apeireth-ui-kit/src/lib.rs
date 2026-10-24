//! # apeireth-ui-kit — 组件库（共享组件的 Rust 重写）
//!
//! 职责边界（设计文档 §3.3.5）：只回答「**这块界面长什么样**」，不含业务数据
//! 获取。组件是**纯函数** `fn(&Props) -> VNode` ⇒ native 下直接调用即可断言
//! 虚拟树，不需要浏览器、不需要 wasm（这是 D3 债「38 处源码镜像测试」的根
//! 解法：断言从「读源码文本」升级为「钉行为」）。
//!
//! ## ✅ 已实现（P3）
//!
//! - **按钮**（[`Button`]）：class 名由调用方给（设计文档 §9.1：class 名是
//!   视觉契约，组件库不发明 class 名）。
//! - **徽标**（[`Badge`]）：class 名 + 条件态追加 class + 悬浮提示。
//! - **卡片壳**（[`CardShell`]）：容器 + class 名 + 可选角色 / 无障碍标签。
//! - **图标**（[`Icon`]）：SVG 结构（标签 / 属性 / 属性顺序）+ **构建期抽取**
//!   的几何数据（[`icon_data`]，带上游许可署名）。
//! - 组件注册表（[`ComponentRegistry`]，契约登记的 UI 组件扩展点）、类型擦除
//!   的组件索引与按名渲染、props 类型不匹配的显式报错。
//!
//! ## ✅ 已实现（工具卡面批）
//!
//! - **工具卡**（[`ToolCallCard`]）与**工具调用生命周期卡**
//!   （[`ToolCallLifecycleCard`]）：结构 / class 名 / 文本节点逐条对齐生产组件。
//! - **JSON 值类型与序列化器**（[`json`]）：对象**保插入序**，序列化结果与脚本侧
//!   的序列化**逐字节相同**（浏览器差分测试钉住）。
//! - **脚本侧数值 / 文本语义**（[`num`]）：数值转文本、固定小数位、取整、
//!   UTF-16 长度与切片。
//!
//! ## 🔵 未接线
//!
//! - 其余共享组件（列表 / 输入 / 对话框 / 开关 / 标签页等）：P5 迁移批逐面
//!   完成（class 名须与被替换实现**逐字一致**，设计 §9.1）。
//! - 组件的**具名插槽**：本期不实现（设计 §6.2：普通泛型函数参数即可表达）。
//!
//! ## 纪律
//!
//! - 只出现 **class 名与结构**，不出现颜色字面量与时长字面量（设计 §2.2）；
//! - 组件里不出现业务数据获取（业务语义归 `ui-demo` 与后端契约）；
//! - **不设 `id`**、**不写内联样式**（设计 §9.2 C3 / C4）。

#![forbid(unsafe_code)]

pub mod badge;
pub mod button;
pub mod card;
pub mod icon;
pub mod icon_data;
pub mod json;
pub mod num;
pub mod registry;
pub mod tool_card;
// ── 并行迁移面（模块声明**预置**，由各面的施工线独占填写对应文件）──
// ⚠️ 预置这些声明是为了让**多个面并行施工而互不碰本文件**（`lib.rs` 是共享点）。
// ⇒ 新模块**不**加扁平 re-export（那会要求各面同时改本文件）⇒ 消费方用路径形式
// （`ui_kit::status_bar::StatusBar`）。
pub mod governance_tab;
pub mod icons_governance;
pub mod icons_status_bar;
pub mod status_bar;

pub use badge::{Badge, BadgeProps};
pub use button::{Button, ButtonProps};
pub use card::{CardShell, CardShellProps};
pub use icon::{Icon, IconData, IconNode, IconProps};
pub use json::JsonValue;
pub use registry::{ComponentRegistry, RegisteredComponent, RegistryError};
pub use tool_card::{
    ToolCallCard, ToolCallCardProps, ToolCallDetails, ToolCallLifecycleCall, ToolCallLifecycleCard,
    ToolCallLifecycleCardProps, ToolCallLifecycleStatus, ToolCallStatus,
};

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

        let props = ButtonProps::new("ap-button").label("提交");
        let node = registry
            .render::<Button>("button", &props)
            .expect("渲染成功");
        assert_eq!(node.tag(), Some("button"));

        // 组件是纯函数：同样 props ⇒ 同样产物。
        assert_eq!(node, Button::render(&props));
    }
}
