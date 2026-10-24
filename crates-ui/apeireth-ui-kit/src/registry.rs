//! 组件注册表：UI 组件扩展点（契约登记路径）。
//!
//! 契约登记：`docs/architecture/contract.toml` `[extension_points]`
//! `ui_component = "crates-ui/apeireth-ui-kit/src/registry.rs::ComponentRegistry"`。
//!
//! 与主仓既有扩展点的关系（契约 §[extension_points] 注释）：
//!
//! - plugin 是**运行时能力**扩展（注册进 `CapabilityRegistry` / `BehaviorRegistry`）；
//! - `ui_component` 是**前端呈现**扩展（注册进 UI 组件表）。
//!
//! 两者不合并、不互为替代。
//!
//! 命名约束：`ComponentRegistry` 这个名字全仓唯一（arch_guard 扩展点唯一性检查）。

use std::any::Any;
use std::collections::BTreeMap;
use std::marker::PhantomData;
use std::rc::Rc;

use apeireth_ui_core::UiComponent;
use apeireth_ui_dom::VNode;

/// 组件注册错误。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegistryError {
    /// 同名组件重复注册。
    Duplicate(String),
    /// 组件不存在。
    NotFound(String),
    /// props 类型与注册组件不匹配。
    PropsTypeMismatch {
        /// 组件注册名。
        component: String,
        /// 期望的 props 类型名。
        expected: String,
    },
}

impl std::fmt::Display for RegistryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RegistryError::Duplicate(name) => write!(f, "Duplicate: 组件已注册（{name}）"),
            RegistryError::NotFound(name) => write!(f, "NotFound: 组件未注册（{name}）"),
            RegistryError::PropsTypeMismatch {
                component,
                expected,
            } => {
                write!(
                    f,
                    "PropsTypeMismatch: {component} 期望 props 类型 {expected}"
                )
            }
        }
    }
}

impl std::error::Error for RegistryError {}

/// 已注册组件的擦除视图（类型擦除：注册表按名字索引异构组件）。
pub trait RegisteredComponent: 'static {
    /// 注册名。
    fn name(&self) -> &str;

    /// 以擦除后的 props 渲染；props 类型不匹配时返回错误。
    fn render(&self, props: &dyn Any) -> Result<VNode, RegistryError>;
}

/// 类型擦除包装：把具体组件类型藏进 `Rc<dyn RegisteredComponent>`。
struct ErasedComponent<C>(PhantomData<C>);

impl<C> RegisteredComponent for ErasedComponent<C>
where
    C: UiComponent<Output = VNode> + 'static,
    C::Props: 'static,
{
    fn name(&self) -> &str {
        std::any::type_name::<C>()
    }

    fn render(&self, props: &dyn Any) -> Result<VNode, RegistryError> {
        let typed =
            props
                .downcast_ref::<C::Props>()
                .ok_or_else(|| RegistryError::PropsTypeMismatch {
                    component: self.name().to_owned(),
                    expected: std::any::type_name::<C::Props>().to_owned(),
                })?;
        Ok(C::render(typed))
    }
}

/// UI 组件注册表。
///
/// 用法：注册组件 → 按名取组件 → 用具体 props 渲染。
/// 注册表只索引**名字**与**类型擦除后的渲染入口**；真正的类型安全由调用方
/// 在编译期保证（`render::<C>` 要求 props 类型与 `C` 一致）。
#[derive(Default)]
pub struct ComponentRegistry {
    entries: BTreeMap<String, Rc<dyn RegisteredComponent>>,
}

impl std::fmt::Debug for ComponentRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ComponentRegistry")
            .field("components", &self.names())
            .finish()
    }
}

impl ComponentRegistry {
    /// 建一个空注册表。
    pub fn new() -> Self {
        Self::default()
    }

    /// 注册一个组件（注册名 = 组件类型名）。
    ///
    /// # Errors
    ///
    /// 同名组件已注册时返回 [`RegistryError::Duplicate`]。
    pub fn register<C>(&mut self) -> Result<(), RegistryError>
    where
        C: UiComponent<Output = VNode> + 'static,
        C::Props: 'static,
    {
        self.register_named::<C>(std::any::type_name::<C>().to_owned())
    }

    /// 注册一个组件（自定义注册名）。
    ///
    /// # Errors
    ///
    /// 同名组件已注册时返回 [`RegistryError::Duplicate`]。
    pub fn register_named<C>(&mut self, name: impl Into<String>) -> Result<(), RegistryError>
    where
        C: UiComponent<Output = VNode> + 'static,
        C::Props: 'static,
    {
        let name = name.into();
        if self.entries.contains_key(&name) {
            return Err(RegistryError::Duplicate(name));
        }
        self.entries
            .insert(name, Rc::new(ErasedComponent::<C>(PhantomData)));
        Ok(())
    }

    /// 按注册名取组件。
    pub fn get(&self, name: &str) -> Option<&Rc<dyn RegisteredComponent>> {
        self.entries.get(name)
    }

    /// 已注册组件名（有序）。
    pub fn names(&self) -> Vec<&str> {
        self.entries.keys().map(String::as_str).collect()
    }

    /// 已注册组件数。
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// 注册表是否为空。
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// 按注册名渲染（props 类型必须与注册组件一致）。
    ///
    /// # Errors
    ///
    /// 组件未注册返回 [`RegistryError::NotFound`]；props 类型不匹配返回
    /// [`RegistryError::PropsTypeMismatch`]。
    pub fn render<C>(&self, name: &str, props: &C::Props) -> Result<VNode, RegistryError>
    where
        C: UiComponent<Output = VNode> + 'static,
        C::Props: 'static,
    {
        let entry = self
            .entries
            .get(name)
            .ok_or_else(|| RegistryError::NotFound(name.to_owned()))?;
        entry.render(props)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_ui_dom::VNode;

    struct CardProps {
        title: String,
    }

    struct Card;

    impl UiComponent for Card {
        type Props = CardProps;
        type Output = VNode;

        fn render(props: &Self::Props) -> Self::Output {
            VNode::element("div")
                .class("ap-card")
                .child(VNode::text(props.title.clone()))
                .build()
        }
    }

    #[test]
    fn register_then_render_by_name() {
        let mut registry = ComponentRegistry::new();
        assert!(registry.is_empty());

        registry
            .register_named::<Card>("card")
            .expect("首次注册成功");
        assert_eq!(registry.len(), 1);
        assert_eq!(registry.names(), vec!["card"]);

        let props = CardProps {
            title: "审批".to_owned(),
        };
        let node = registry.render::<Card>("card", &props).expect("渲染成功");
        assert_eq!(node.tag(), Some("div"));
    }

    #[test]
    fn duplicate_registration_is_rejected() {
        let mut registry = ComponentRegistry::new();
        registry
            .register_named::<Card>("card")
            .expect("首次注册成功");
        let err = registry
            .register_named::<Card>("card")
            .expect_err("同名重复注册必须报错");
        assert_eq!(err, RegistryError::Duplicate(String::from("card")));
        assert_eq!(registry.len(), 1);
    }

    #[test]
    fn unknown_component_and_wrong_props_are_reported() {
        struct Other;

        impl UiComponent for Other {
            type Props = String;
            type Output = VNode;

            fn render(props: &Self::Props) -> Self::Output {
                VNode::text(props.clone())
            }
        }

        let mut registry = ComponentRegistry::new();
        registry
            .register_named::<Card>("card")
            .expect("首次注册成功");

        let props = CardProps {
            title: "x".to_owned(),
        };
        assert_eq!(
            registry.render::<Card>("missing", &props),
            Err(RegistryError::NotFound(String::from("missing")))
        );
        assert!(matches!(
            registry.render::<Other>("card", &String::from("wrong type")),
            Err(RegistryError::PropsTypeMismatch { .. })
        ));
    }
}
