//! CapabilityCatalog — 装配能力清单 (N17/TP2).
//!
//! **职责**: 从 `ToolRegistry` 生成"当前装配了哪些工具对外可见"的清单.
//! 供 serve 端点 / 自审 / 卸载核对用. **只读快照**, 不持 registry 引用.
//!
//! **确定性**: 条目按名排序 (全 API 确定性排序纪律).

use crate::registry::ToolRegistry;
use crate::types::{ToolAxes, ToolKind};

/// 清单条目: 一个已装配工具的可公开元数据.
#[derive(Debug, Clone)]
pub struct CatalogEntry {
    /// 工具注册名
    pub name: String,
    /// 6 类 (VCP pluginType)
    pub kind: ToolKind,
    /// 5 轴正交属性
    pub axes: ToolAxes,
}

/// 装配能力清单 (只读快照).
#[derive(Debug, Clone, Default)]
pub struct CapabilityCatalog {
    /// 条目 (按名升序)
    pub entries: Vec<CatalogEntry>,
}

impl CapabilityCatalog {
    /// 从 registry 快照生成 (按名排序, 确定性).
    pub fn from_registry(registry: &ToolRegistry) -> Self {
        let mut entries: Vec<CatalogEntry> = registry
            .list()
            .into_iter()
            .filter_map(|name| {
                let tool = registry.get(&name)?;
                Some(CatalogEntry {
                    name,
                    kind: tool.kind(),
                    axes: tool.axes(),
                })
            })
            .collect();
        entries.sort_by(|a, b| a.name.cmp(&b.name));
        Self { entries }
    }

    /// 工具名列表 (已排序).
    pub fn names(&self) -> Vec<&str> {
        self.entries.iter().map(|e| e.name.as_str()).collect()
    }

    /// 条目数.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// 是否为空.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// 是否包含某工具名.
    pub fn contains(&self, name: &str) -> bool {
        self.entries.iter().any(|e| e.name == name)
    }

    /// Markdown 渲染 (清单对外展示用).
    pub fn render_markdown(&self) -> String {
        let mut out = String::from("| 工具 | 类型 |\n|---|---|\n");
        for e in &self.entries {
            out.push_str(&format!("| {} | {} |\n", e.name, e.kind.as_legacy_str()));
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::{MockSyncTool, MOCK_NAMES};

    #[test]
    fn catalog_snapshots_sorted_names() {
        let registry = ToolRegistry::new();
        registry.register("zeta".to_string(), std::sync::Arc::new(MockSyncTool { name: "zeta".to_string() }));
        registry.register("alpha".to_string(), std::sync::Arc::new(MockSyncTool { name: "alpha".to_string() }));
        let cat = CapabilityCatalog::from_registry(&registry);
        assert_eq!(cat.len(), 2);
        assert_eq!(cat.names(), vec!["alpha", "zeta"], "必须按名排序");
        assert!(cat.contains("alpha"));
        assert!(!cat.contains("missing"));
    }

    #[test]
    fn catalog_tracks_unregister() {
        let registry = ToolRegistry::new();
        registry.register("tmp".to_string(), std::sync::Arc::new(MockSyncTool { name: "tmp".to_string() }));
        assert!(CapabilityCatalog::from_registry(&registry).contains("tmp"));
        registry.unregister("tmp");
        assert!(!CapabilityCatalog::from_registry(&registry).contains("tmp"), "卸载后清单不得残留");
    }

    #[test]
    fn render_markdown_lists_rows() {
        let registry = ToolRegistry::new();
        registry.register("alpha".to_string(), std::sync::Arc::new(MockSyncTool { name: "alpha".to_string() }));
        let md = CapabilityCatalog::from_registry(&registry).render_markdown();
        assert!(md.contains("| alpha | synchronous |"), "markdown 应含工具行: {md}");
    }

    #[test]
    fn mock_tools_registered_via_names() {
        // MOCK_NAMES 存在性守护 (防止 mock 体系变更破坏本模块测试依赖)
        assert!(!MOCK_NAMES.is_empty());
    }
}
