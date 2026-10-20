//! 统一扩展 ABI 的基础类型（ADR-0008 R2-2，additive）。
//!
//! 目标：把两套扩展角色（`Plugin` 配置期 provider / `BehaviorModule` 运行期 hook）
//! 统一到一套**描述/目录**契约，使扩展面可枚举、可校验、可对账。
//!
//! 设计边界（见 ADR-0008）：
//! - **本模块只加类型**，不迁移任何实现；`PluginManager` / `BehaviorRegistry` 在 R2-3 分批实现 [`ExtensionCatalog`]；
//! - 两个角色 trait 保留（合并会逼配置期 provider 实现运行期钩子），`ExtensionKind` 负责统一寻址；
//! - foundation 不依赖 engine：本模块必须保持零 crate 内反向依赖（当前仅 std）。

/// 扩展角色类别（一个概念一个名字；新增类别 = 结构变更，必须 ADR）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ExtensionKind {
    /// 配置期 provider（`Plugin` trait 角色，foundation/plugin）。
    Plugin,
    /// 运行期行为模块（`BehaviorModule` trait 角色，engine/runtime）。
    BehaviorModule,
    /// 能力声明（工具/后端能力，注册在 capability 面）。
    Capability,
    /// 子循环派生器（`SubLoopSpawner`）。
    SubLoop,
}

impl ExtensionKind {
    /// 稳定字符串（用于 manifest / telemetry / 测试断言；改名 = 破坏性变更，必须 ADR）。
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Plugin => "plugin",
            Self::BehaviorModule => "behavior-module",
            Self::Capability => "capability",
            Self::SubLoop => "sub-loop",
        }
    }
}

/// 扩展条目描述（统一 ABI 的最小对账面）。
///
/// `id` 全局唯一；`owner_crate` 为声明归属；`order` 仅影响展示/装配顺序（小者先）；
/// `capabilities` 为可选的能力 id 列表（R2-3 起承接 plugin 侧 id→owner 索引语义）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtensionDescriptor {
    /// 扩展 id（全局唯一，建议 `<kind>:<name>`）。
    pub id: String,
    /// 扩展角色类别。
    pub kind: ExtensionKind,
    /// 声明归属 crate（用于归属对账与依赖检查）。
    pub owner_crate: String,
    /// 装配/展示顺序（升序；同值按 id 字典序，保证确定性）。
    pub order: i32,
    /// 关联能力 id（可为空）。
    pub capabilities: Vec<String>,
}

impl ExtensionDescriptor {
    /// 构造一个无能力的扩展描述。
    pub fn new(
        id: impl Into<String>,
        kind: ExtensionKind,
        owner_crate: impl Into<String>,
        order: i32,
    ) -> Self {
        Self {
            id: id.into(),
            kind,
            owner_crate: owner_crate.into(),
            order,
            capabilities: Vec::new(),
        }
    }

    /// 追加一个能力 id（builder 风格）。
    pub fn with_capability(mut self, capability_id: impl Into<String>) -> Self {
        self.capabilities.push(capability_id.into());
        self
    }

    /// 确定性排序键：(order, id)。
    pub fn sort_key(&self) -> (i32, &str) {
        (self.order, self.id.as_str())
    }
}

/// 扩展目录（注册表的统一读取面）。
///
/// R2-3 起由 `PluginManager` 与 `BehaviorRegistry` 分别实现；调用方只依赖本 trait，
/// 不依赖具体注册表类型。**本 trait 不提供写接口**——写语义由各层注册表保持（避免过早统一写入模型）。
pub trait ExtensionCatalog {
    /// 全部扩展 id（实现需保证确定性排序，建议按 [`ExtensionDescriptor::sort_key`]）。
    fn extension_ids(&self) -> Vec<String>;

    /// 按 id 取描述；不存在返回 `None`。
    fn describe(&self, id: &str) -> Option<ExtensionDescriptor>;

    /// 是否存在该扩展（默认实现基于 [`Self::describe`]）。
    fn contains(&self, id: &str) -> bool {
        self.describe(id).is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kind_strings_are_stable() {
        assert_eq!(ExtensionKind::Plugin.as_str(), "plugin");
        assert_eq!(ExtensionKind::BehaviorModule.as_str(), "behavior-module");
        assert_eq!(ExtensionKind::Capability.as_str(), "capability");
        assert_eq!(ExtensionKind::SubLoop.as_str(), "sub-loop");
    }

    #[test]
    fn descriptor_sorts_by_order_then_id() {
        let mut v = vec![
            ExtensionDescriptor::new("b:z", ExtensionKind::Plugin, "crate-b", 10),
            ExtensionDescriptor::new("a:y", ExtensionKind::BehaviorModule, "crate-a", 10),
            ExtensionDescriptor::new("c:x", ExtensionKind::Capability, "crate-c", 5),
        ];
        v.sort_by(|a, b| a.sort_key().cmp(&b.sort_key()));
        let ids: Vec<&str> = v.iter().map(|d| d.id.as_str()).collect();
        assert_eq!(ids, vec!["c:x", "a:y", "b:z"]);
    }

    #[test]
    fn builder_appends_capabilities() {
        let d =
            ExtensionDescriptor::new("plugin:demo", ExtensionKind::Plugin, "apeireth-plugin", 0)
                .with_capability("cap:one")
                .with_capability("cap:two");
        assert_eq!(d.capabilities, vec!["cap:one", "cap:two"]);
    }

    struct EmptyCatalog;

    impl ExtensionCatalog for EmptyCatalog {
        fn extension_ids(&self) -> Vec<String> {
            Vec::new()
        }

        fn describe(&self, _id: &str) -> Option<ExtensionDescriptor> {
            None
        }
    }

    #[test]
    fn contains_defaults_to_describe() {
        let c = EmptyCatalog;
        assert!(!c.contains("nothing"));
        assert!(c.extension_ids().is_empty());
    }
}
