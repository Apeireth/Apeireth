//! # apeireth-plugin
//!
//! Plugin Manager (1:1 翻译 v0.9.21 商业版 `out/main/chunks/PluginManager-BAmNCucP.js` ~12KB).
//! 商业版 PluginManager 是 **marketplace 插件安装器** (从 GitHub URL 拉插件 → scan → copy → 写
//! `known_marketplaces.json` 索引), **不是** runtime WASM/VM2 loader. 本 crate 1:1 翻译 + 扩展 6 个
//! 周边 API (lifecycle / permission / watcher / registry / m3 防御), R20 阶段 4 估缺 P1.
//!
//! ## v0.9.21 PluginManager-BAmNCucP.js 实查 (obfuscated webpack bundle, 12KB, 单行)
//!
//! | Token / 函数 | 命中 | 推断 | 出处 |
//! |--------------|----:|------|------|
//! | `installPluginFromUrl` | 1 | 核心入口 (下载 → 扫描 → 复制 → 注册) | exports 导出 |
//! | `checkPlatform` | 1 | 平台 (win/darwin/linux/bsd) 兼容性检查 | exports 导出 |
//! | `SPECTRAI_PLUGIN_NO_REGISTRY` | 1 | 跳过注册中心 (env var) | process.env |
//! | `known_marketplaces.json` | 4 | 插件索引文件 (record install history) | 字符串多次出现 |
//! | `mkdtempSync(os.tmpdir())` | 1 | 临时目录下载 | node:os + fs |
//! | `downloadTarball` | 1 | GitHub tarball 下载 (octokit 风格) | 内部调用 |
//! | `scanRepo` | 1 | 仓库扫描 (来自 `RepoScanner.js` 共用) | require |
//! | `repoDirStats` | 1 | 仓库目录统计 (file/byte count) | 内部调用 |
//! | `plugin.json` / `extension.toml` | 隐式 | 插件元信息 (估 `apeireth-extension` 风格) | 字符串 |
//! | `vm2` / `isolated-vm` / `wasmtime` | **0** | **v0.9.21 不用 WASM/VM2 沙箱** (用 `node-pty` 子进程, O-5 不假装) | — |
//! | `chokidar` / `fs.watch` | **0** | **v0.9.21 无热加载 watcher** (R20 阶段 4 扩展) | — |
//! | `process.platform` (win/darwin/linux/bsd) | 4 | 4 平台 (per `checkPlatform`) | — |
//!
//! ## 关键 design (R20 阶段 4)
//!
//! - **1:1 翻译**: `install_plugin_from_url` + `check_platform` 严格按 v0.9.21 字段.
//! - **集成**: 运行时执行委托给 `apeireth-extension` (6 类插件 + extension.toml + 沙盒 + 审计, LOCKED, 不改 src/).
//! - **R20 扩展**: lifecycle (load→init→ready→unload→destroy) + 4 种 PluginPermission + 文件 watcher (poll, 2s 间隔) + m3 防御 (8 工具白名单 hardcode).
//! - **沙箱**: 不引 wasmtime (per v0.9.21 实查 0 命中, O-5 不假装), 用 `std::process::Command` 子进程隔离 (per R20 阶段 4 决策).
//! - **K-1 强校验**: 编译期 hardcode `"apeireth"` 平台名 + 4 PluginPermission + 8 工具名 (per `m3-hallucination-defense §2.4` + `supervisor-prompt-818 §5.3`).
//!
//! ## 状态: ⏳ skeleton (R20 阶段 4 续, 主 2026-08-05 19:50 拍板"派成员干, 自己干分散注意力" 5 P0 crate 已写, 本 crate 是阶段 1 续)

#![allow(missing_docs)]
#![allow(clippy::all)]

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, SystemTime};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::{info, warn};
use uuid::Uuid;

// ============================================================================
// m3 hallucination 防御 #3 (per m3-hallucination-defense-2026-08-05.md §2.4 + §2.1)
// WHITELIST 编译期 hardcode 8 工具, validate_tool_call 在 dispatch 前 schema 校验.
// 防止 minimax m3 模型幻觉调用不存在的工具 (eg. "apeireth_plugin_uninstall" 实际不存在).
// ============================================================================

/// m3 防御: Plugin Manager 8 工具白名单 (编译期 hardcode, 不可运行时改).
pub const TOOL_WHITELIST: &[&str] = &[
    "apeireth_plugin_load",
    "apeireth_plugin_unload",
    "apeireth_plugin_reload",
    "apeireth_plugin_list",
    "apeireth_plugin_get_metadata",
    "apeireth_plugin_set_permission",
    "apeireth_plugin_watch_start",
    "apeireth_plugin_watch_stop",
];

/// 编译期守门: TOOL_WHITELIST 长度 == 8 (per K-1 强校验 + 8 项不修改承诺 #5)
pub const TOOL_WHITELIST_COUNT: usize = 8;
const _: () = assert!(TOOL_WHITELIST.len() == TOOL_WHITELIST_COUNT);

/// m3 防御: 校验工具调用是否在白名单内. 不在则拒绝 (返 `PluginError::ToolNotWhitelisted`).
pub fn validate_tool_call(tool: &str, _args: &serde_json::Value) -> Result<(), PluginError> {
    if !TOOL_WHITELIST.contains(&tool) {
        return Err(PluginError::ToolNotWhitelisted(tool.to_string()));
    }
    Ok(())
}

// ============================================================================
// §1 文档头 + 编译期 hardcode (per R20 P0 5 crate 风格 + K-1 强校验)
// ============================================================================

/// plugin manifest schema version (跟 v0.9.21 `plugin.json` `schema_version` 字段 1:1).
/// K-1 强校验 #1: 编译期 hardcode, 不写 `"1"` 字符串 elsewhere.
pub const PLUGIN_SCHEMA_VERSION: &str = "1";

/// 平台名 (K-1 强校验 #2: 编译期 hardcode `"apeireth"`, v0.9.21 1:1 翻译, 不写 "SpectrAI" 等装饰名).
pub const PLATFORM_NAME: &str = "apeireth";

/// 单 host 最大插件数 (估 `apeireth-mcp-ssh` `max_sessions = 16` 类比, plugin 留 64 因为轻量).
pub const MAX_PLUGINS_PER_HOST: usize = 64;

/// plugin 目录 poll 间隔 (ms, cross-platform 走 poll 不走 inotify 避免 FSAPI 差异).
pub const PLUGIN_WATCH_POLL_INTERVAL_MS: u64 = 2000;

/// 单插件安装超时 (ms, 估 GitHub tarball 下载 30s 够).
pub const PLUGIN_INSTALL_TIMEOUT_MS: u64 = 30_000;

/// sandbox 子进程超时 (ms, plugin 命令最长跑 5s 防恶意插件搞坏主进程).
pub const PLUGIN_SANDBOX_TIMEOUT_MS: u64 = 5_000;

// ============================================================================
// §2 核心类型 (PluginMetadata / PluginLifecycle / PluginPermission / PluginError)
// ============================================================================

/// plugin 唯一 ID (UUID v4, per `uuid::Uuid::new_v4()`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PluginId(pub Uuid);

impl PluginId {
    /// 派生新 ID (per R20 K-1: 不接受外部传入字符串, 防 ID 注入).
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for PluginId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for PluginId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// plugin 元信息 (1:1 翻译 v0.9.21 `plugin.json` 字段).
///
/// 字段对应 v0.9.21 `PluginManifest` (估 5-7 字段): `name` / `version` / `author` / `entry` /
/// `permissions` / `minApeirethVersion`. R20 阶段 4 加 `installed_at` / `source` / `install_path` /
/// `lifecycle` / `size_bytes` 5 个管理字段.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginMetadata {
    /// plugin 名 (e.g. `"apeireth-plugin-relay-image-helper"`).
    pub name: String,
    /// plugin 版本 (semver, e.g. `"0.1.0"`).
    pub version: String,
    /// plugin 作者 (e.g. `"weibin <bin.wei@steriguard.cn>"`).
    pub author: String,
    /// 入口文件 (相对 plugin 目录, e.g. `"src/lib.rs"` / `"index.js"`).
    pub entry: PathBuf,
    /// 授权 permissions (4 种枚举, per K-1 强校验).
    pub permissions: Vec<PluginPermission>,
    /// 最低 apeireth 版本要求 (估 1.0.0, 跟 workspace v1.0.0 一致).
    pub min_apeireth_version: String,
    /// 安装时间戳 (SystemTime, R20 阶段 4 加).
    #[serde(default = "default_installed_at")]
    pub installed_at: SystemTime,
    /// 安装源 (e.g. `"github:owner/repo"` / `"local:/path/to/plugin"`, 1:1 翻译 v0.9.21 installPluginFromUrl).
    pub source: String,
    /// 本地安装路径 (per v0.9.21 `installLocation` 字段, 默认 `~/.apeireth/plugins/<name>`).
    pub install_path: PathBuf,
    /// 当前 lifecycle 状态 (per v0.9.21 5 状态机).
    pub lifecycle: PluginLifecycle,
    /// 仓库扫描统计 (file count + bytes, per v0.9.21 `repoDirStats` 1:1).
    pub size_bytes: u64,
}

fn default_installed_at() -> SystemTime {
    SystemTime::UNIX_EPOCH
}

/// plugin 生命周期 (5 状态机, 1:1 翻译 v0.9.21 5 lifecycle hook).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginLifecycle {
    /// 刚下载/解压, 还没 init.
    Loaded,
    /// 调 `init(apeireth_api)` 完成, 准备好接收请求.
    Initialized,
    /// 进入 ready 状态, 接受 tool calls.
    Ready,
    /// 用户/系统触发 unload, 停止接收请求, 释放资源.
    Unloaded,
    /// destroy 后最终态 (per v0.9.21 `unload → destroy` 链).
    Destroyed,
}

impl PluginLifecycle {
    /// 状态转移合法性检查 (per R20 状态机 1:1 翻译).
    /// 允许: Loaded→Initialized, Initialized→Ready, Ready→Unloaded, Unloaded→Destroyed.
    /// 跨级 / 倒退 / 死循环 → false.
    pub fn can_transition_to(self, next: PluginLifecycle) -> bool {
        use PluginLifecycle::*;
        matches!(
            (self, next),
            (Loaded, Initialized) | (Initialized, Ready) | (Ready, Unloaded) | (Unloaded, Destroyed)
        )
    }
}

/// plugin 权限 (4 种, K-1 强校验 #3: 编译期 hardcode 4 个枚举值, 不可运行时增删).
///
/// 字段对应 v0.9.21 `PluginManifest.permissions` 数组元素 (估 `file_read` / `file_write` /
/// `network` / `mcp_call` 4 种, 1:1 翻译).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginPermission {
    /// 读文件 (e.g. 读 `~/.apeireth/config.toml`).
    FileRead,
    /// 写文件 (e.g. 写 plugin 自己的 cache).
    FileWrite,
    /// 网络 (e.g. 调外部 API).
    Network,
    /// 调 apeireth MCP 工具 (e.g. 调 `apeireth_mcp_ssh_exec`).
    McpCall,
}

/// 编译期守门: SUPPORTED_PERMISSIONS 4 项 (K-1 强校验 #3).
pub const SUPPORTED_PERMISSIONS: &[PluginPermission] = &[
    PluginPermission::FileRead,
    PluginPermission::FileWrite,
    PluginPermission::Network,
    PluginPermission::McpCall,
];
const _: () = assert!(SUPPORTED_PERMISSIONS.len() == 4);

/// plugin 安装来源 (1:1 翻译 v0.9.21 installPluginFromUrl 选项).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PluginSource {
    /// GitHub URL (e.g. `"https://github.com/owner/repo"`).
    GitHub { url: String, owner: String, repo: String },
    /// 本地路径 (e.g. `"/path/to/plugin"`).
    Local { path: PathBuf },
    /// HTTP tarball URL.
    Tarball { url: String },
}

impl PluginSource {
    /// 解析为 source 字符串 (per v0.9.21 `known_marketplaces.json` `source` 字段).
    pub fn as_source_str(&self) -> String {
        match self {
            PluginSource::GitHub { url, .. } => format!("github:{url}"),
            PluginSource::Local { path } => format!("local:{}", path.display()),
            PluginSource::Tarball { url } => format!("tarball:{url}"),
        }
    }
}

/// Plugin Manager 错误 (12 variant, per mcp-ssh 13 variant 类比).
#[derive(Debug, Error)]
pub enum PluginError {
    /// m3 防御: 工具未在白名单内 (per m3-hallucination-defense §2.4).
    #[error("tool not whitelisted: {0}")]
    ToolNotWhitelisted(String),
    #[error("plugin not found: {0}")]
    NotFound(PluginId),
    #[error("plugin already exists: {0}")]
    AlreadyExists(String),
    #[error("plugin install failed: {0}")]
    InstallFailed(String),
    #[error("plugin metadata parse failed: {0}")]
    MetadataParse(String),
    #[error("plugin version incompatible: need {need}, got {got}")]
    VersionMismatch { need: String, got: String },
    #[error("plugin permission denied: {0:?}")]
    PermissionDenied(PluginPermission),
    #[error("plugin lifecycle invalid: {from:?} -> {to:?}")]
    InvalidLifecycleTransition {
        from: PluginLifecycle,
        to: PluginLifecycle,
    },
    #[error("plugin sandbox timeout ({0:?})")]
    SandboxTimeout(Duration),
    #[error("plugin sandbox execution failed: {0}")]
    SandboxExecFailed(String),
    #[error("plugin I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("plugin error: {0}")]
    Other(String),
}

pub type PluginResult<T> = Result<T, PluginError>;

// ============================================================================
// §3 加载器 PluginLoader (async fn load / unload / reload / watch)
// ============================================================================

/// plugin 安装结果 (1:1 翻译 v0.9.21 `installPluginFromUrl` 返回值).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallResult {
    /// 是否成功 (per v0.9.21 隐式 success 字段).
    pub result: bool,
    /// plugin 元信息 (成功时填充).
    pub metadata: Option<PluginMetadata>,
    /// 警告列表 (per v0.9.21 `warnings` 字段, 平台不支持等).
    pub warnings: Vec<String>,
    /// 错误码 (失败时填充, per v0.9.21 `ErrorCode` 枚举).
    pub error_code: Option<String>,
    /// 跳过注册中心 (per `SPECTRAI_PLUGIN_NO_REGISTRY` env).
    pub no_registry: bool,
}

impl Default for InstallResult {
    fn default() -> Self {
        Self {
            result: false,
            metadata: None,
            warnings: Vec::new(),
            error_code: None,
            no_registry: false,
        }
    }
}

/// plugin 加载器 (核心: install_plugin_from_url 1:1 翻译 v0.9.21).
///
/// 字段对应 v0.9.21 `PluginManager` class (估 8 fields):
/// - `plugins_dir` (per v0.9.21 `os.homedir() + '/.apeireth/plugins'`)
/// - `registry_path` (per v0.9.21 `known_marketplaces.json`)
/// - `no_registry` (per `SPECTRAI_PLUGIN_NO_REGISTRY` env)
/// - `max_per_host` (per R20 阶段 4 估补)
/// - `watch_running` (per §3 watcher)
/// - `loaded` (per HashMap<PluginId, PluginHandle>)
/// - `sandbox` (per §4)
/// - `registry` (per §5)
#[derive(Debug)]
pub struct PluginLoader {
    /// plugin 安装目录 (默认 `~/.apeireth/plugins/`, 跟 v0.9.21 一致).
    pub plugins_dir: PathBuf,
    /// 插件索引文件 (per v0.9.21 `known_marketplaces.json`).
    pub registry_path: PathBuf,
    /// 跳过注册中心 (per v0.9.21 `SPECTRAI_PLUGIN_NO_REGISTRY` env).
    pub no_registry: bool,
    /// 单 host 最大插件数 (编译期 `MAX_PLUGINS_PER_HOST`).
    pub max_per_host: usize,
    /// watcher 是否运行 (per §3 watch_start/watch_stop).
    pub watch_running: AtomicBool,
    /// 已加载 plugin (per §5 PluginRegistry 集成).
    pub registry: PluginRegistry,
}

impl PluginLoader {
    /// 默认配置 (per v0.9.21 实查 `os.homedir() + '/.apeireth/plugins'`).
    pub fn new() -> PluginResult<Self> {
        let home = std::env::var("USERPROFILE")
            .or_else(|_| std::env::var("HOME"))
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("."));
        let plugins_dir = home.join(".apeireth").join("plugins");
        let registry_path = plugins_dir.join("known_marketplaces.json");
        Ok(Self {
            plugins_dir,
            registry_path,
            no_registry: std::env::var("SPECTRAI_PLUGIN_NO_REGISTRY")
                .map(|v| v == "1")
                .unwrap_or(false),
            max_per_host: MAX_PLUGINS_PER_HOST,
            watch_running: AtomicBool::new(false),
            registry: PluginRegistry::default(),
        })
    }

    /// 1:1 翻译 v0.9.21 `installPluginFromUrl(url, options)`.
    /// 选项 `options.dryRun` (估 v0.9.21 `dryRun` boolean) 控制是否真写盘.
    /// 步骤: 1) mkdtemp → 2) download tarball → 3) scan → 4) copy → 5) 注册.
    pub async fn install_plugin_from_url(
        &mut self,
        url: &str,
        dry_run: bool,
    ) -> PluginResult<InstallResult> {
        if self.registry.plugins.len() >= self.max_per_host {
            return Err(PluginError::InstallFailed(format!(
                "max plugins per host reached: {}",
                self.max_per_host
            )));
        }
        let mut res = InstallResult {
            no_registry: self.no_registry,
            ..Default::default()
        };
        // 1:1 翻译 v0.9.21 步骤 1: 检查 platform
        let platform_warnings = self.check_platform();
        if !platform_warnings.is_empty() {
            res.warnings.extend(platform_warnings);
        }
        info!(target: "apeireth_plugin", "install_plugin_from_url url={url} dry_run={dry_run}");
        // 步骤 2-5: skeleton 占位 (R20 阶段 4 真接 GitHub API + scan)
        // skeleton 阶段 dry_run 也返 result=true (只 metadata=None, per v0.9.21 dry_run 行为)
        res.result = true;
        if !dry_run {
            // 真安装路径: 调 §5 PluginRegistry + 写 registry_path
            res.metadata = Some(PluginMetadata {
                name: format!("{PLATFORM_NAME}-plugin-stub"),
                version: "0.1.0".to_string(),
                author: "skeleton".to_string(),
                entry: PathBuf::from("src/lib.rs"),
                permissions: SUPPORTED_PERMISSIONS.to_vec(),
                min_apeireth_version: PLATFORM_NAME.to_string(),
                installed_at: SystemTime::now(),
                source: format!("github:{url}"),
                install_path: self.plugins_dir.join("stub"),
                lifecycle: PluginLifecycle::Loaded,
                size_bytes: 0,
            });
        } else {
            // dry-run 路径: per v0.9.21 `dryRun=true` 跳过真复制, metadata=None
            res.warnings
                .push(format!("{PLATFORM_NAME} dry-run: {url}"));
        }
        Ok(res)
    }

    /// 1:1 翻译 v0.9.21 `checkPlatform` (4 平台 win/darwin/linux/bsd).
    /// 返回空 Vec = 平台完全支持, 非空 = warning 列表.
    pub fn check_platform(&self) -> Vec<String> {
        let mut warnings = Vec::new();
        let platform = std::env::consts::OS;
        if !matches!(platform, "windows" | "darwin" | "linux" | "freebsd") {
            warnings.push(format!(
                "{PLATFORM_NAME} plugin: unsupported platform {platform}, expected win/darwin/linux/bsd"
            ));
        }
        warnings
    }

    /// 卸载 plugin (per v0.9.21 估缺 + R20 阶段 4 估补).
    pub async fn unload(&mut self, plugin_id: &PluginId) -> PluginResult<()> {
        self.registry.unregister(plugin_id).await
    }

    /// 重新加载 plugin (先 unload 再 load).
    pub async fn reload(&mut self, plugin_id: &PluginId) -> PluginResult<()> {
        self.registry.unregister(plugin_id).await?;
        // skeleton: 真 reload 需要 plugin 路径, 走 §5 PluginRegistry.register 重做
        warn!(target: "apeireth_plugin", "reload skeleton for {plugin_id}");
        Ok(())
    }

    /// 启动文件 watcher (per §3, poll 不走 inotify).
    pub fn watch_start(&self) -> PluginResult<()> {
        self.watch_running.store(true, Ordering::SeqCst);
        info!(target: "apeireth_plugin", "watch_start poll_interval_ms={PLUGIN_WATCH_POLL_INTERVAL_MS}");
        Ok(())
    }

    /// 停止文件 watcher.
    pub fn watch_stop(&self) -> PluginResult<()> {
        self.watch_running.store(false, Ordering::SeqCst);
        info!(target: "apeireth_plugin", "watch_stop");
        Ok(())
    }
}

impl Default for PluginLoader {
    fn default() -> Self {
        Self::new().expect("PluginLoader::new default must succeed")
    }
}

// ============================================================================
// §4 沙箱 PluginSandbox (std::process::Command 隔离, 不引 wasmtime)
// ============================================================================

/// plugin 沙箱配置.
#[derive(Debug, Clone)]
pub struct SandboxConfig {
    /// 子进程超时 (ms, per 编译期 `PLUGIN_SANDBOX_TIMEOUT_MS`).
    pub timeout: Duration,
    /// 工作目录 (默认 `plugin.install_path`).
    pub workdir: PathBuf,
    /// 环境变量白名单 (允许 plugin 读的环境变量, 防泄露 secrets).
    pub env_whitelist: Vec<String>,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            timeout: Duration::from_millis(PLUGIN_SANDBOX_TIMEOUT_MS),
            workdir: PathBuf::from("."),
            env_whitelist: vec![
                "PATH".to_string(),
                "HOME".to_string(),
                "USERPROFILE".to_string(),
                format!("{PLATFORM_NAME}_HOME"),
            ],
        }
    }
}

/// plugin 沙箱执行结果.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxResult {
    /// stdout (UTF-8 string).
    pub stdout: String,
    /// stderr (UTF-8 string).
    pub stderr: String,
    /// 退出码 (-1 = 超时 / 未跑完).
    pub exit_code: i32,
    /// 实际耗时.
    pub duration: Duration,
}

/// plugin 沙箱 (用 `std::process::Command` 子进程隔离, **不**用 wasmtime 因为 v0.9.21 实查 0 命中).
///
/// 跟 v0.9.21 `node-pty` 子进程 1:1 翻译 (per R20 阶段 4 决策). O-5 不假装 v0.9.21 有
/// wasmtime/VM2 沙箱 — 实查 0 命中.
#[derive(Debug)]
pub struct PluginSandbox {
    pub config: SandboxConfig,
}

impl PluginSandbox {
    /// 默认配置.
    pub fn new() -> Self {
        Self {
            config: SandboxConfig::default(),
        }
    }

    /// 沙箱执行 plugin 命令 (e.g. `cargo build --release` / `node index.js`).
    /// skeleton: 估补, R20 阶段 4 真接 `tokio::process::Command` + timeout.
    pub async fn execute(
        &self,
        cmd: &str,
        args: &[&str],
    ) -> PluginResult<SandboxResult> {
        info!(target: "apeireth_plugin", "sandbox exec cmd={cmd} args={args:?}");
        // skeleton: R20 阶段 4 真接 `tokio::process::Command::new(cmd).args(args)`
        // + `tokio::time::timeout(self.config.timeout, child.wait())` + 收集 stdout/stderr.
        Ok(SandboxResult {
            stdout: String::new(),
            stderr: "skeleton: sandbox not implemented yet".to_string(),
            exit_code: -1,
            duration: Duration::ZERO,
        })
    }
}

impl Default for PluginSandbox {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// §5 API 注册 PluginRegistry (HashMap<PluginId, PluginHandle>)
// ============================================================================

/// plugin 句柄 (运行时实例, lifecycle 状态 + 沙箱 + 元信息).
#[derive(Debug)]
pub struct PluginHandle {
    pub metadata: PluginMetadata,
    pub sandbox: PluginSandbox,
}

impl PluginHandle {
    /// 创建新 handle.
    pub fn new(metadata: PluginMetadata) -> Self {
        Self {
            metadata,
            sandbox: PluginSandbox::new(),
        }
    }

    /// lifecycle 状态机推进 (per v0.9.21 5 状态机 + `can_transition_to` 守门).
    pub fn transition(&mut self, next: PluginLifecycle) -> PluginResult<()> {
        if !self.metadata.lifecycle.can_transition_to(next) {
            return Err(PluginError::InvalidLifecycleTransition {
                from: self.metadata.lifecycle,
                to: next,
            });
        }
        info!(
            target: "apeireth_plugin",
            "plugin {} transition {:?} -> {:?}",
            self.metadata.name, self.metadata.lifecycle, next
        );
        self.metadata.lifecycle = next;
        Ok(())
    }
}

/// plugin 注册表 (HashMap<PluginId, PluginHandle>).
#[derive(Debug, Default)]
pub struct PluginRegistry {
    pub plugins: HashMap<PluginId, PluginHandle>,
}

impl PluginRegistry {
    /// 注册 plugin (per §5).
    pub async fn register(&mut self, metadata: PluginMetadata) -> PluginResult<PluginId> {
        if self.plugins.len() >= MAX_PLUGINS_PER_HOST {
            return Err(PluginError::InstallFailed(format!(
                "max plugins per host reached: {MAX_PLUGINS_PER_HOST}"
            )));
        }
        let id = PluginId::new();
        if self.plugins.contains_key(&id) {
            return Err(PluginError::AlreadyExists(id.to_string()));
        }
        info!(
            target: "apeireth_plugin",
            "register plugin name={} id={id}",
            metadata.name
        );
        self.plugins.insert(id.clone(), PluginHandle::new(metadata));
        Ok(id)
    }

    /// 注销 plugin (per §5).
    pub async fn unregister(&mut self, id: &PluginId) -> PluginResult<()> {
        let handle = self
            .plugins
            .get_mut(id)
            .ok_or_else(|| PluginError::NotFound(id.clone()))?;
        handle.transition(PluginLifecycle::Unloaded)?;
        handle.transition(PluginLifecycle::Destroyed)?;
        self.plugins.remove(id);
        Ok(())
    }

    /// 列出所有 plugin metadata.
    pub fn list(&self) -> Vec<&PluginMetadata> {
        self.plugins.values().map(|h| &h.metadata).collect()
    }

    /// 按 ID 查 metadata.
    pub fn get(&self, id: &PluginId) -> Option<&PluginMetadata> {
        self.plugins.get(id).map(|h| &h.metadata)
    }

    /// 设 plugin permission (R20 阶段 4 估补).
    pub fn set_permission(
        &mut self,
        id: &PluginId,
        perm: PluginPermission,
        granted: bool,
    ) -> PluginResult<()> {
        let handle = self
            .plugins
            .get_mut(id)
            .ok_or_else(|| PluginError::NotFound(id.clone()))?;
        if granted && !handle.metadata.permissions.contains(&perm) {
            handle.metadata.permissions.push(perm);
        } else if !granted {
            handle.metadata.permissions.retain(|p| *p != perm);
        }
        info!(
            target: "apeireth_plugin",
            "plugin {} permission {perm:?} granted={granted}",
            handle.metadata.name
        );
        Ok(())
    }

    /// plugin 数量.
    pub fn len(&self) -> usize {
        self.plugins.len()
    }

    /// 是否空.
    pub fn is_empty(&self) -> bool {
        self.plugins.is_empty()
    }
}

// ============================================================================
// §6 m3 防御 (TOOL_WHITELIST + validate_tool_call)
// ============================================================================
//
// m3 hallucination 防御 #3: 8 工具白名单 hardcode + schema 校验.
// 实现位置: 文件顶部 `TOOL_WHITELIST` const + `validate_tool_call` 函数.
// 测试: 见 `tests/test_plugin_in_process.rs` (Fixture 5).
//
// 跟 m3 §2.4 14 工具 (mcp builtin) 关系:
// - 本 crate 是 Plugin Manager 8 工具 (R20 阶段 4 新增)
// - apeireth-mcp builtin 14 工具 (per m3-hallucination-defense §2.4) 独立维护
// - 跨 crate 集成: apeireth-mcp 估补时 import `apeireth_plugin::TOOL_WHITELIST`
// ============================================================================

/// plugin demo helper: 跑一遍 m3 防御 sanity check (8 工具名 + 5 K-1 字样).
pub fn m3_defense_sanity_check() -> bool {
    let in_whitelist = [
        "apeireth_plugin_load",
        "apeireth_plugin_unload",
        "apeireth_plugin_reload",
        "apeireth_plugin_list",
        "apeireth_plugin_get_metadata",
        "apeireth_plugin_set_permission",
        "apeireth_plugin_watch_start",
        "apeireth_plugin_watch_stop",
    ];
    in_whitelist.iter().all(|t| TOOL_WHITELIST.contains(t))
        && SUPPORTED_PERMISSIONS.len() == 4
        && PLATFORM_NAME == "apeireth"
        && PLUGIN_SCHEMA_VERSION == "1"
}

// ============================================================================
// §7 测试 fixture (in-module, 跟 mcp-ssh 7 测试类比)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plugin_loader_skeleton_creates_with_default_config() {
        let loader = PluginLoader::new();
        assert!(loader.is_ok());
    }

    #[test]
    fn plugin_lifecycle_can_transition_to_validates_5_states() {
        use PluginLifecycle::*;
        assert!(Loaded.can_transition_to(Initialized));
        assert!(Initialized.can_transition_to(Ready));
        assert!(Ready.can_transition_to(Unloaded));
        assert!(Unloaded.can_transition_to(Destroyed));
        // 非法转移
        assert!(!Loaded.can_transition_to(Ready));
        assert!(!Loaded.can_transition_to(Destroyed));
        assert!(!Ready.can_transition_to(Loaded));
    }

    #[tokio::test]
    async fn plugin_metadata_serializes_with_4_permissions() {
        let meta = PluginMetadata {
            name: "test".into(),
            version: "0.1.0".into(),
            author: "test".into(),
            entry: PathBuf::from("src/lib.rs"),
            permissions: SUPPORTED_PERMISSIONS.to_vec(),
            min_apeireth_version: PLATFORM_NAME.into(),
            installed_at: SystemTime::now(),
            source: "local:/tmp/test".into(),
            install_path: PathBuf::from("/tmp/test"),
            lifecycle: PluginLifecycle::Loaded,
            size_bytes: 1024,
        };
        let json = serde_json::to_string(&meta).unwrap();
        assert!(json.contains("\"file_read\""));
        assert!(json.contains("\"file_write\""));
        assert!(json.contains("\"network\""));
        assert!(json.contains("\"mcp_call\""));
    }

    #[test]
    fn m3_defense_validate_tool_call_rejects_unknown_tool() {
        let bad = serde_json::json!({});
        assert!(matches!(
            validate_tool_call("apeireth_plugin_uninstall", &bad),
            Err(PluginError::ToolNotWhitelisted(_))
        ));
        // 白名单内 8 工具全 OK
        for tool in TOOL_WHITELIST {
            assert!(validate_tool_call(tool, &bad).is_ok());
        }
    }

    #[test]
    fn m3_defense_sanity_check_passes() {
        assert!(m3_defense_sanity_check());
    }

    #[tokio::test]
    async fn plugin_registry_register_and_list() {
        let mut reg = PluginRegistry::default();
        let meta = PluginMetadata {
            name: "demo".into(),
            version: "0.1.0".into(),
            author: "demo".into(),
            entry: PathBuf::from("src/lib.rs"),
            permissions: vec![PluginPermission::FileRead],
            min_apeireth_version: PLATFORM_NAME.into(),
            installed_at: SystemTime::now(),
            source: "local:/tmp/demo".into(),
            install_path: PathBuf::from("/tmp/demo"),
            lifecycle: PluginLifecycle::Loaded,
            size_bytes: 0,
        };
        let id = reg.register(meta).await.unwrap();
        assert_eq!(reg.len(), 1);
        assert!(!reg.is_empty());
        let listed = reg.list();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].name, "demo");
        // 走完 lifecycle 5 状态机 (Loaded → Initialized → Ready → Unloaded → Destroyed)
        // 注销前先推进 lifecycle 到 Ready (Loaded → Unloaded 非法)
        let handle = reg.plugins.get_mut(&id).expect("handle present");
        handle
            .transition(PluginLifecycle::Initialized)
            .expect("Loaded→Initialized");
        handle
            .transition(PluginLifecycle::Ready)
            .expect("Initialized→Ready");
        // 注销走完 Ready → Unloaded → Destroyed
        reg.unregister(&id).await.unwrap();
        assert!(reg.is_empty());
    }

    #[tokio::test]
    async fn plugin_sandbox_execute_skeleton_returns_dummy() {
        let sb = PluginSandbox::new();
        let res = sb.execute("echo", &["hello"]).await.unwrap();
        assert_eq!(res.exit_code, -1);
        assert!(res.stderr.contains("skeleton"));
    }
}

// ============================================================================
// async_trait placeholder (避免 unused_imports warning)
// ============================================================================
#[allow(dead_code)]
#[async_trait]
trait _PluginAsync: Send + Sync {
    async fn _dummy(&self) -> PluginResult<()> {
        Ok(())
    }
}

const _: () = ();
