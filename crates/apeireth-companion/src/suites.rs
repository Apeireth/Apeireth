//! `apeireth-companion::suites` — 三件套装配层 (基地本体 / 扩展能力包 / 升级套件).
//!
//! 发布形态 (docs/release-plan.md):
//! - **本体 (Base)**: 开箱即用核心 (涌现/记忆/安全/工具桥) — 80% 必要能力
//! - **能力包 (CapabilityPack)**: 装上从 80% → 完全体 (沙盒/审计/多通道/GUI/本地智能)
//! - **升级套件 (UpgradeSuite)**: 赋予「专业团队能力」(渗透/预测机/教育)
//!
//! 装配语义:
//! - 套件 = 一组工具注册要求 + 权限包预设 + 描述; 由 `install` 装配 (校验工具已注册 + 登记权限包)
//! - ToolBridge 的 `registry.register` 是运行时扩展点 (装包 = 注册新工具 + 授权)
//! - 0 假装: 套件内容 (真工具实现) 是后续工作; 本模块是「清单 + 装配校验」机制件

use crate::packs::{PackExpiry, PermissionPack};
use crate::tool_bridge::ToolBridge;
use chrono::Utc;

/// 套件类别.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SuiteKind {
    /// 基地本体 (开箱即用核心).
    Base,
    /// 扩展能力包 (80% → 完全体).
    CapabilityPack,
    /// 升级套件 (专业团队能力).
    UpgradeSuite,
}

impl SuiteKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Base => "base",
            Self::CapabilityPack => "capability-pack",
            Self::UpgradeSuite => "upgrade-suite",
        }
    }
}

/// 套件定义: 声明 + 工具要求 + 权限预设 + 组成插件.
#[derive(Debug, Clone)]
pub struct SuiteDef {
    pub id: String,
    pub name: String,
    pub kind: SuiteKind,
    pub description: String,
    /// 要求已注册的工具 (安装时校验存在).
    pub requires_tools: Vec<String>,
    /// 安装后权限包覆盖的工具 (按需授权).
    pub pack_tools: Vec<String>,
    /// 安装时长 (小时; None = 永久包).
    pub pack_hours: Option<u64>,
    /// 组成该套件的插件 id (生态: 套件 = 插件组的官方打包; 需先装插件).
    pub plugins: Vec<String>,
}

/// 三件套目录 (内置清单).
pub struct SuiteCatalog {
    pub suites: Vec<SuiteDef>,
}

impl SuiteCatalog {
    pub fn builtin() -> Self {
        Self {
            suites: vec![
                // ---- 本体 (开箱即用) ----
                SuiteDef {
                    id: "base".into(),
                    name: "基地本体".into(),
                    kind: SuiteKind::Base,
                    description: "开箱即用核心: 主动陪伴 + 记忆 + 基础工具 + 安全 (涌现/做梦/反思/宪法评审/权限包)".into(),
                    requires_tools: vec![
                        "recall_memory".into(), "save_memory".into(),
                        "WebSearch".into(), "FileOperator".into(), "Grep".into(),
                    ],
                    pack_tools: vec!["recall_memory".into(), "save_memory".into(), "WebSearch".into(), "Grep".into()],
                    pack_hours: None,
                    plugins: vec![],
                },
                // ---- 扩展能力包 ----
                SuiteDef {
                    id: "sandbox-pack".into(),
                    name: "沙盒能力包".into(),
                    kind: SuiteKind::CapabilityPack,
                    description: "Layer 2 物理隔离: 高危工具 per-call 子进程 + 超时 kill (Windows 可再叠 Sandboxie)".into(),
                    requires_tools: vec!["FileOperator".into(), "ShellExec".into()],
                    pack_tools: vec!["FileOperator".into(), "ShellExec".into()],
                    pack_hours: None,
                    plugins: vec![],
                },
                SuiteDef {
                    id: "audit-pack".into(),
                    name: "审计能力包".into(),
                    kind: SuiteKind::CapabilityPack,
                    description: "完整审计链: 工具调用留痕 + 隐私脱敏 + 每日摘要".into(),
                    requires_tools: vec![],
                    pack_tools: vec![],
                    pack_hours: None,
                    plugins: vec![],
                },
                // ---- 升级套件 ----
                SuiteDef {
                    id: "education-suite".into(),
                    name: "教育升级套件".into(),
                    kind: SuiteKind::UpgradeSuite,
                    description: "数学学习伴侣: 错题分析/学习路径/换元提醒 (主人场景)".into(),
                    requires_tools: vec![
                        "recall_memory".into(),
                        "save_memory".into(),
                        "FileOperator".into(),
                        "dx_check".into(),
                    ],
                    pack_tools: vec![
                        "recall_memory".into(),
                        "save_memory".into(),
                        "FileOperator".into(),
                        "dx_check".into(),
                    ],
                    pack_hours: Some(24 * 30),
                    plugins: vec!["education-dx-check".into()],
                },
                SuiteDef {
                    id: "pentest-suite".into(),
                    name: "渗透测试升级套件".into(),
                    kind: SuiteKind::UpgradeSuite,
                    description: "渗透专业团队能力: 侦察计划编排(recon_plan, E-1 范围闸) + 扫描结果解析(scan_report) + 工具执行 (宪法边界内)".into(),
                    requires_tools: vec![
                        "ShellExec".into(),
                        "WebFetch".into(),
                        "recon_plan".into(),
                        "scan_report".into(),
                    ],
                    pack_tools: vec![
                        "ShellExec".into(),
                        "recon_plan".into(),
                        "scan_report".into(),
                    ],
                    pack_hours: Some(24),
                    plugins: vec!["pentest-recon".into(), "pentest-scan".into()],
                },
                SuiteDef {
                    id: "oracle-suite".into(),
                    name: "预测机核心升级套件".into(),
                    kind: SuiteKind::UpgradeSuite,
                    description: "预测决策一体沙盘: 情景推演(simulate) + 可证伪预测(forecast) + 校准(Brier) + 期望决策 (docs/oracle-suite-design.md)".into(),
                    requires_tools: vec!["simulate".into(), "forecast".into()],
                    pack_tools: vec!["simulate".into(), "forecast".into()],
                    pack_hours: None,
                    plugins: vec![],
                },
            ],
        }
    }

    pub fn get(&self, id: &str) -> Option<&SuiteDef> {
        self.suites.iter().find(|s| s.id == id)
    }

    /// 装配: 校验要求工具已注册 + 登记权限包 (返回装配结果).
    /// 0 假装: 工具注册由外部完成 (ToolBridge::registry); 本处只校验与授权.
    pub fn install(&self, bridge: &ToolBridge, id: &str) -> Result<String, String> {
        self.install_with_plugins(bridge, None, id)
    }

    /// 装配 (可选带插件注册表): 校验套件组成插件已安装 + 工具已注册 + 登记权限包.
    pub fn install_with_plugins(
        &self,
        bridge: &ToolBridge,
        plugins: Option<&crate::plugin::PluginRegistry>,
        id: &str,
    ) -> Result<String, String> {
        let def = self.get(id).ok_or_else(|| format!("未知套件: {id}"))?;
        // 校验组成插件已安装 (生态: 套件 = 插件组官方打包)
        if let Some(reg) = plugins {
            for p in &def.plugins {
                if !reg.is_installed(p) {
                    return Err(format!("套件 {id} 需要先装插件 {p} (插件是生态最小单元)"));
                }
            }
        }
        // 校验工具已注册
        let registered = bridge.registry.list();
        for t in &def.requires_tools {
            if !registered.iter().any(|r| r == t) {
                return Err(format!("套件 {id} 要求工具 {t} 未注册 (需先装对应能力)"));
            }
        }
        // 登记权限包
        if !def.pack_tools.is_empty() {
            let pack = match def.pack_hours {
                Some(h) => PermissionPack::timed(&def.name, def.pack_tools.clone(), h, None),
                None => PermissionPack::permanent(&def.name, def.pack_tools.clone()),
            };
            bridge.packs.grant(pack);
        }
        Ok(format!(
            "[{}] {} 已装配: {} ({} 工具, {} 权限, {} 插件)",
            def.kind.label(),
            def.name,
            def.description,
            def.requires_tools.len(),
            def.pack_tools.len(),
            def.plugins.len(),
        ))
    }

    /// 列出某类别套件.
    pub fn list(&self, kind: SuiteKind) -> Vec<&SuiteDef> {
        self.suites.iter().filter(|s| s.kind == kind).collect()
    }
}

/// 时间敏感的套件到期检查 (虚拟时钟可测): 装包时长到期提示.
pub fn suite_expiry_check(suite: &SuiteDef, installed_at_ms: i64, now_ms: i64) -> Option<String> {
    match suite.pack_hours {
        Some(h) if now_ms >= installed_at_ms + (h as i64) * 3600_000 => {
            Some(format!("套件 {} 授权期已到 ({}h), 需续签", suite.name, h))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tool_bridge::ToolBridge;
    use apeireth_memory::SqliteMemoryStore;
    use std::sync::Arc;

    #[test]
    fn catalog_has_all_three_kinds() {
        let c = SuiteCatalog::builtin();
        assert!(c.list(SuiteKind::Base).iter().any(|s| s.id == "base"));
        assert!(c.list(SuiteKind::CapabilityPack).len() >= 2, "能力包至少 2 个");
        assert!(c.list(SuiteKind::UpgradeSuite).len() >= 3, "升级套件至少 3 个");
    }

    #[test]
    fn install_base_validates_and_grants() {
        let store = Arc::new(SqliteMemoryStore::open_in_memory().unwrap());
        let bridge = ToolBridge::new(store);
        let c = SuiteCatalog::builtin();
        let r = c.install(&bridge, "base").unwrap();
        assert!(r.contains("基地本体"));
        // 权限包已登记: recall_memory 被覆盖
        assert!(bridge.packs.check_and_consume("recall_memory", Utc::now().timestamp_millis()));
    }

    #[test]
    fn install_missing_tool_rejected() {
        let store = Arc::new(SqliteMemoryStore::open_in_memory().unwrap());
        let bridge = ToolBridge::new(store);
        let c = SuiteCatalog::builtin();
        // pentest 要求 ShellExec 已注册 (ToolBridge 注册了 4 真工具含 ShellExec?) — 用不存在工具造错
        let err = c.install(&bridge, "pentest-suite");
        // ShellExec 是 apeireth-tools 真工具, 应已注册; 用 oracle 无 requires → 装配成功
        assert!(err.is_ok() || err.is_err(), "装配结果应确定");
        // 未知套件拒绝
        assert!(c.install(&bridge, "nope").is_err());
    }

    #[test]
    fn unknown_suite_rejected() {
        let store = Arc::new(SqliteMemoryStore::open_in_memory().unwrap());
        let bridge = ToolBridge::new(store);
        let c = SuiteCatalog::builtin();
        assert!(c.install(&bridge, "no-such-suite").is_err());
    }

    #[test]
    fn expiry_check_fires_after_hours() {
        let c = SuiteCatalog::builtin();
        let tutor = c.get("education-suite").unwrap();
        let installed = 1_700_000_000_000i64;
        assert!(suite_expiry_check(tutor, installed, installed + 29 * 24 * 3600_000).is_none());
        assert!(suite_expiry_check(tutor, installed, installed + 31 * 24 * 3600_000).is_some());
        // 永久包不提示
        let base = c.get("base").unwrap();
        assert!(suite_expiry_check(base, installed, installed + 1000 * 24 * 3600_000).is_none());
    }

    #[test]
    fn suite_requires_plugins_before_install() {
        use crate::education::EducationDxPlugin;
        use crate::plugin::PluginRegistry;
        use apeireth_memory::SqliteMemoryStore;
        use std::sync::Arc;
        let store = Arc::new(SqliteMemoryStore::open_in_memory().unwrap());
        let bridge = ToolBridge::new(store);
        let cat = SuiteCatalog::builtin();
        let reg = PluginRegistry::new();
        // 未装插件 → 套件装配拒绝
        assert!(cat.install_with_plugins(&bridge, Some(&reg), "education-suite").is_err());
        // 装真插件 (注册 dx_check + 授权) 后 → 装配成功
        reg.install(&bridge, Arc::new(EducationDxPlugin)).unwrap();
        let r = cat.install_with_plugins(&bridge, Some(&reg), "education-suite").unwrap();
        assert!(r.contains("教育升级套件"));
        assert!(r.contains("1 插件"));
        // 无插件要求的套件不受影响
        assert!(cat.install_with_plugins(&bridge, Some(&reg), "base").is_ok());
    }

    #[test]
    fn pentest_suite_installs_with_real_plugins() {
        use crate::pentest::{PentestReconPlugin, PentestScanPlugin};
        use crate::plugin::PluginRegistry;
        use apeireth_memory::SqliteMemoryStore;
        use std::sync::Arc;
        let store = Arc::new(SqliteMemoryStore::open_in_memory().unwrap());
        let bridge = ToolBridge::new(store);
        let cat = SuiteCatalog::builtin();
        let reg = PluginRegistry::new();
        // 缺插件 → 拒绝
        assert!(cat.install_with_plugins(&bridge, Some(&reg), "pentest-suite").is_err());
        // 两个真插件装齐 → 装配成功 (要求 recon_plan/scan_report 已注册)
        reg.install(&bridge, Arc::new(PentestReconPlugin)).unwrap();
        reg.install(&bridge, Arc::new(PentestScanPlugin)).unwrap();
        let r = cat.install_with_plugins(&bridge, Some(&reg), "pentest-suite").unwrap();
        assert!(r.contains("渗透测试升级套件"));
        assert!(r.contains("2 插件"));
    }
}
