//! `apeireth-companion::plugin` — 插件机制 (生态底层, 对齐 DSH cordis 插件思想).
//!
//! 分层 (生态设计):
//! - **插件 = 最小贡献单元**: 一个声明式单元 (工具注册 + 权限预设 + 生命周期)
//! - **套件 = 插件组的官方打包**: 一键装「完整能力」 (见 suites.rs, SuiteDef.plugins)
//!
//! 生命周期: `on_load` (注册工具+授权) / `on_unload` (卸载清理) / 版本 / 依赖.
//! 装配点: ToolBridge.registry.register 是运行时注册口 (装插件 = 注册 + 授权).
//!
//! 与 AI 演化衔接: AI 提案的能力 (capability) 激活后, 可部署为一个插件
//! (工具注册 + 权限预设), 让「AI 自己长出来的能力」成为可分发生态件.

use std::sync::{Arc, Mutex};

use crate::tool_bridge::ToolBridge;

/// 插件: 最小贡献单元.
pub trait Plugin: Send + Sync {
    fn id(&self) -> &str;
    fn version(&self) -> &str;
    fn description(&self) -> &str;
    /// 加载: 注册工具 + 授权. 失败 → 安装拒绝 (0 装 PASS).
    fn on_load(&self, bridge: &ToolBridge) -> Result<(), String>;
    /// 卸载: 清理 (权限撤销等). 幂等.
    fn on_unload(&self, bridge: &ToolBridge) -> Result<(), String>;
}

/// 插件注册表: 安装/卸载/查询/依赖.
pub struct PluginRegistry {
    inner: Mutex<Vec<Arc<dyn Plugin>>>,
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self { inner: Mutex::new(Vec::new()) }
    }

    /// 安装插件: 调 on_load, 成功才登记; 失败回滚 (不登记, 0 装 PASS).
    pub fn install(&self, bridge: &ToolBridge, plugin: Arc<dyn Plugin>) -> Result<(), String> {
        let mut list = self.inner.lock().unwrap();
        if list.iter().any(|p| p.id() == plugin.id()) {
            return Err(format!("插件已安装: {}", plugin.id()));
        }
        plugin.on_load(bridge)?; // 失败 → 不登记
        list.push(plugin);
        Ok(())
    }

    /// 卸载插件 (幂等).
    pub fn uninstall(&self, bridge: &ToolBridge, id: &str) -> Result<(), String> {
        let mut list = self.inner.lock().unwrap();
        let idx = list.iter().position(|p| p.id() == id).ok_or_else(|| format!("插件未安装: {id}"))?;
        let p = list.remove(idx);
        p.on_unload(bridge)?;
        Ok(())
    }

    pub fn is_installed(&self, id: &str) -> bool {
        self.inner.lock().unwrap().iter().any(|p| p.id() == id)
    }

    pub fn list(&self) -> Vec<String> {
        self.inner.lock().unwrap().iter().map(|p| p.id().to_string()).collect()
    }

    pub fn get(&self, id: &str) -> Option<Arc<dyn Plugin>> {
        self.inner.lock().unwrap().iter().find(|p| p.id() == id).cloned()
    }

    /// 批量安装一组插件 (套件装配用): 全部成功才整体生效, 任一失败回滚已装的.
    pub fn install_all(&self, bridge: &ToolBridge, plugins: &[Arc<dyn Plugin>]) -> Result<(), String> {
        let mut installed: Vec<Arc<dyn Plugin>> = Vec::new();
        for p in plugins {
            if let Err(e) = self.install(bridge, Arc::clone(p)) {
                for done in installed.iter().rev() {
                    let _ = self.uninstall(bridge, done.id());
                }
                return Err(format!("插件 {} 安装失败, 已回滚: {e}", p.id()));
            }
            installed.push(Arc::clone(p));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_memory::SqliteMemoryStore;

    /// 测试插件: 加载时给 packs 授权一个工具, 卸载时撤销 (校验生命周期真跑).
    struct TestPlugin;
    impl Plugin for TestPlugin {
        fn id(&self) -> &str { "test-dx-check" }
        fn version(&self) -> &str { "0.1.0" }
        fn description(&self) -> &str { "测试插件: 换元 dx 检查" }
        fn on_load(&self, bridge: &ToolBridge) -> Result<(), String> {
            bridge.packs.grant(crate::packs::PermissionPack::permanent(
                "插件授权", vec!["recall_memory".into(), "save_memory".into()],
            ));
            Ok(())
        }
        fn on_unload(&self, _bridge: &ToolBridge) -> Result<(), String> {
            Ok(())
        }
    }

    /// 加载失败的插件 (on_load 报错 → 安装拒绝).
    struct BadPlugin;
    impl Plugin for BadPlugin {
        fn id(&self) -> &str { "bad-plugin" }
        fn version(&self) -> &str { "0.0.1" }
        fn description(&self) -> &str { "加载必失败" }
        fn on_load(&self, _bridge: &ToolBridge) -> Result<(), String> {
            Err("加载失败: 缺依赖".into())
        }
        fn on_unload(&self, _bridge: &ToolBridge) -> Result<(), String> { Ok(()) }
    }

    #[test]
    fn install_uninstall_lifecycle() {
        let store = Arc::new(SqliteMemoryStore::open_in_memory().unwrap());
        let bridge = ToolBridge::new(store);
        let reg = PluginRegistry::new();
        reg.install(&bridge, Arc::new(TestPlugin)).unwrap();
        assert!(reg.is_installed("test-dx-check"));
        // 权限包已授权
        assert!(bridge.packs.check_and_consume("recall_memory", chrono::Utc::now().timestamp_millis()));
        // 重复安装拒绝
        assert!(reg.install(&bridge, Arc::new(TestPlugin)).is_err());
        reg.uninstall(&bridge, "test-dx-check").unwrap();
        assert!(!reg.is_installed("test-dx-check"));
        // 未安装卸载报错
        assert!(reg.uninstall(&bridge, "test-dx-check").is_err());
    }

    #[test]
    fn failing_plugin_not_registered() {
        let store = Arc::new(SqliteMemoryStore::open_in_memory().unwrap());
        let bridge = ToolBridge::new(store);
        let reg = PluginRegistry::new();
        assert!(reg.install(&bridge, Arc::new(BadPlugin)).is_err());
        assert!(!reg.is_installed("bad-plugin"));
    }

    #[test]
    fn install_all_rolls_back_on_failure() {
        let store = Arc::new(SqliteMemoryStore::open_in_memory().unwrap());
        let bridge = ToolBridge::new(store);
        let reg = PluginRegistry::new();
        // [ok, bad] → bad 失败 → ok 也应回滚
        let err = reg.install_all(&bridge, &[Arc::new(TestPlugin), Arc::new(BadPlugin)]);
        assert!(err.is_err());
        assert!(!reg.is_installed("test-dx-check"), "失败应回滚已装插件");
        assert!(!reg.is_installed("bad-plugin"));
    }

    #[test]
    fn list_and_get() {
        let store = Arc::new(SqliteMemoryStore::open_in_memory().unwrap());
        let bridge = ToolBridge::new(store);
        let reg = PluginRegistry::new();
        reg.install(&bridge, Arc::new(TestPlugin)).unwrap();
        assert_eq!(reg.list(), vec!["test-dx-check".to_string()]);
        assert_eq!(reg.get("test-dx-check").unwrap().version(), "0.1.0");
    }
}
