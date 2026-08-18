//! `apeireth-companion::app_container` — AppContainer 档 backend (S1 高危档, 0 装 PASS).
//!
//! **为什么独立于 B3 SandboxBackend**: B3 是物理隔离后端 (Sandboxie/landlock), 与 OS
//! 虚拟化或第三方软件栈相关; AppContainer 是 Windows 8+ 内建"UWP-style"沙盒 — 需要
//! 注册 AppContainer profile + 申请 capability, 复杂程度高 (`CreateAppContainerProfile` +
//! `GetAppContainerFolderPath` + `DeriveAppContainerSidFromAppContainerName` 等),
//! 属下个迭代范畴. **本模块只留 trait 口, 0 装PASS**:
//! - `available()` 永远返回 `false`
//! - `status()` 诚实返回"未接"原因
//! - `render_params()` 返回一份启动参数模板 (供"真接"时使用)
//!
//! **何时升级**: 真接点见 [`crate::restricted_token::win_imp`] (CreateRestrictedToken
//! 已支持 AppContainer), 需要补: (1) Profile 注册 (2) Capability 清单 (3) SID 转换.
//! 属后续高危档任务, 与 team-work-doc §3 后续 N 行一致.

use crate::sandbox::SandboxConfig;

/// AppContainer 档 backend 接口 (S1 高危档).
///
/// 复用 B3 [`crate::sandbox::SandboxBackend`] 风格 — **同一组语义**:
/// `available()` / `status()` / `render_params()` 一致. 之所以独立:
/// AppContainer 接收的是 AppContainer SID + capability 清单, 不是 sandbox 后端参数;
/// 上下游处理路径不同 (例如需要 `DeriveAppContainerSidFromAppContainerName`).
pub trait AppContainerBackend {
    /// 后端名.
    fn name(&self) -> &'static str;
    /// 当前平台是否已接 (0 装 PASS: 未接如实返回 false).
    fn available(&self) -> bool;
    /// 接入状态说明 (诚实标注: 未接原因 + 真接路径).
    fn status(&self) -> &'static str;
    /// 启动参数模板 (真接时用于 CreateProcess + AppContainer SID 包).
    fn render_params(&self, cfg: &SandboxConfig) -> Vec<String>;
}

/// Windows AppContainer 档 (高危档, 0 装 PASS).
///
/// 真接路径 (后续任务):
/// 1. `CreateAppContainerProfile(name, displayName, description, capabilities)` — 注册 profile.
/// 2. `DeriveAppContainerSidFromAppContainerName(name)` → SID for CreateProcess.
/// 3. `GetAppContainerFolderPath(profile)` → 容器家目录 (可选).
/// 4. `CreateProcess` 用 `PROC_THREAD_ATTRIBUTE_LIST` + `PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES`
///    注入 SID + capability.
/// 5. capability 清单 (internetClient / privateNetworkClientServer / documentsLibrary /
///    picturesLibrary / videosLibrary / musicLibrary / location 等) 由 upper layer 决定.
///
/// 现状: 全部 0 装 — `available()` 永远 false.
#[derive(Debug, Clone, Copy)]
pub struct AppContainerProfile;

impl AppContainerBackend for AppContainerProfile {
    fn name(&self) -> &'static str {
        "AppContainer"
    }
    fn available(&self) -> bool {
        false
    }
    fn status(&self) -> &'static str {
        "trait 口已备, 未接: AppContainer 档为高危档 (UWP-style 容器) — 需注册 \
         CreateAppContainerProfile + capability 清单 + DeriveAppContainerSidFromAppContainerName, \
         属下个迭代任务. 当前 use_app_container=true 仅记录不强制启用"
    }
    fn render_params(&self, cfg: &SandboxConfig) -> Vec<String> {
        // 真接时: CreateProcess PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES 包装参数模板.
        // 这里的字符串仅文档 / 配置面板参考, 实际进程创建走 win32 API.
        vec![
            "AppContainerProfile=<to-be-registered>".to_string(),
            format!("AppContainerSid=deriving-pending (RID={})", "S-1-5-93"),
            format!("CapabilityCount=2 (internetClient, documentsLibrary)"),
            format!("AppContainer_Folder=per-profile"),
            format!("JobObject_TimeoutSecs={}", cfg.timeout_secs),
        ]
    }
}

/// Windows 平台后端清单 (诚实口径: 当前 0 装 PASS).
pub fn windows_backends() -> Vec<Box<dyn AppContainerBackend>> {
    vec![Box::new(AppContainerProfile)]
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_container_0_install_honest() {
        // 0 装 PASS: available 必须 false, status 必须诚实标注未接.
        let b = AppContainerProfile;
        assert!(
            !b.available(),
            "AppContainer 0 装 PASS: available 必须 false"
        );
        assert!(
            b.status().contains("未接"),
            "status 必须诚实标注未接: {}",
            b.status()
        );
    }

    #[test]
    fn app_container_name_stable() {
        assert_eq!(AppContainerProfile.name(), "AppContainer");
    }

    #[test]
    fn app_container_render_params_contains_timeout() {
        let cfg = SandboxConfig {
            timeout_secs: 90,
            ..Default::default()
        };
        let p = AppContainerProfile.render_params(&cfg);
        assert!(
            p.iter().any(|s| s.contains("90")),
            "参数模板应含 timeout=90: {p:?}"
        );
    }

    #[test]
    fn windows_backends_contains_app_container() {
        for b in windows_backends() {
            assert_eq!(b.name(), "AppContainer");
            assert!(!b.available());
        }
    }
}
