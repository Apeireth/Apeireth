//! 组件模型边界：UI 组件的统一 trait。
//!
//! 设计文档 §3.3.1 / §6.1：组件 = **纯函数** `fn(&Props) -> Output`，
//! 不碰真实 DOM、不含业务语义。渲染层（`ui-dom`）负责把 `Output` 写成 DOM。
//!
//! **命名约束（契约登记硬要求）**：本 trait 名固定为 `UiComponent`，**不得**叫
//! `Component` —— 全仓已有两处 `Component` 同名定义（图像家族），改名会触发
//! `scripts/arch_guard.py` 的扩展点唯一性检查（W10 结构变更违规）。
//! 登记位置：`docs/architecture/contract.toml` `[extension_points]`
//! `ui_component_trait`。

/// UI 组件的统一 trait。
///
/// 实现方约定：
/// - [`UiComponent::render`] 是**纯函数**：同样的 props 产出同样的视图树；
/// - 组件**不含业务语义**（不认识会话 / 审批 / presence 等概念），
///   业务语义归 `ui-kit` / `ui-demo` 层（设计文档 §1.3 第 12 条）。
pub trait UiComponent {
    /// 组件属性（props）类型：由编译期检查的结构体承载，杜绝手写类型漂移。
    type Props;

    /// 渲染产物类型：通常是被渲染层消费的虚拟节点树。
    type Output;

    /// 纯函数渲染：由 props 产出视图树。
    ///
    /// # Panics
    ///
    /// 本函数不应 panic；实现方若遇到不可渲染的输入，应返回可判定的错误值
    /// 而不是让渲染栈崩掉。
    fn render(props: &Self::Props) -> Self::Output;
}

#[cfg(test)]
mod tests {
    use super::*;

    struct DemoProps {
        title: String,
    }

    struct Demo;

    impl UiComponent for Demo {
        type Props = DemoProps;
        type Output = String;

        fn render(props: &Self::Props) -> Self::Output {
            format!("demo:{}", props.title)
        }
    }

    #[test]
    fn ui_component_renders_purely_from_props() {
        let props = DemoProps {
            title: "审批".to_owned(),
        };
        assert_eq!(Demo::render(&props), "demo:审批");
        // 纯函数：同样的 props 产出同样的结果。
        assert_eq!(Demo::render(&props), Demo::render(&props));
    }
}
