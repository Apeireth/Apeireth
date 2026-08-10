//! # plugin_demo
//!
//! apeireth-plugin 端到端 demo (per R20 阶段 4 估补).
//!
//! 跑通 5 步:
//! 1. 创建 PluginLoader (默认 `~/.apeireth/plugins/`)
//! 2. 调 `install_plugin_from_url` (dry_run=true 不真写盘)
//! 3. 调 `check_platform` 拿 platform warnings
//! 4. 调 m3 防御 sanity check (8 工具 + 4 权限 + K-1 字样)
//! 5. 打印 InstallResult + 列出已加载 plugin
//!
//! 跑法: `cargo run --example plugin_demo --manifest-path crates/apeireth-plugin/Cargo.toml`

use apeireth_plugin::{
    m3_defense_sanity_check, validate_tool_call, PluginLifecycle, PluginLoader, PluginPermission,
    PluginSource, SUPPORTED_PERMISSIONS, TOOL_WHITELIST, TOOL_WHITELIST_COUNT,
    PLATFORM_NAME, PLUGIN_SCHEMA_VERSION, MAX_PLUGINS_PER_HOST,
};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== apeireth-plugin demo ===\n");

    // §1 编译期 hardcode sanity (K-1 强校验)
    println!("[K-1] platform_name = {PLATFORM_NAME}");
    println!("[K-1] schema_version = {PLUGIN_SCHEMA_VERSION}");
    println!("[K-1] max_plugins_per_host = {MAX_PLUGINS_PER_HOST}");
    println!("[K-1] supported_permissions.len() = {}", SUPPORTED_PERMISSIONS.len());
    println!("[K-1] tool_whitelist.len() = {TOOL_WHITELIST_COUNT}\n");

    // §2 创建 PluginLoader
    let mut loader = PluginLoader::new()?;
    println!("[§3 loader] plugins_dir = {}", loader.plugins_dir.display());
    println!("[§3 loader] registry_path = {}", loader.registry_path.display());
    println!("[§3 loader] no_registry = {}\n", loader.no_registry);

    // §3 check_platform (4 平台 win/darwin/linux/bsd, 不在则 warning)
    let platform = std::env::consts::OS;
    let warnings = loader.check_platform();
    println!("[§3 check_platform] current = {platform}, warnings = {warnings:?}\n");

    // §4 install_plugin_from_url (dry_run=true 不真写盘)
    let res = loader
        .install_plugin_from_url("https://github.com/apeireth/plugin-stub", true)
        .await?;
    println!("[§3 install_plugin_from_url]");
    println!("  result        = {}", res.result);
    println!("  no_registry   = {}", res.no_registry);
    println!("  warnings.len  = {}", res.warnings.len());
    println!("  metadata      = {:?}\n", res.metadata.as_ref().map(|m| &m.name));

    // §5 PluginSource 1:1 翻译
    let src = PluginSource::GitHub {
        url: "https://github.com/apeireth/plugin-stub".to_string(),
        owner: "apeireth".to_string(),
        repo: "plugin-stub".to_string(),
    };
    println!("[§2 PluginSource] as_source_str = {}\n", src.as_source_str());

    // §6 m3 防御 sanity check (8 工具 + 4 权限 + K-1 字样)
    println!("[§6 m3 defense] sanity_check = {}", m3_defense_sanity_check());
    println!("[§6 m3 defense] whitelist = {TOOL_WHITELIST:#?}");
    let bad = serde_json::json!({});
    println!(
        "[§6 m3 defense] validate_tool_call('apeireth_plugin_uninstall', _) = {:?}",
        validate_tool_call("apeireth_plugin_uninstall", &bad)
    );
    println!(
        "[§6 m3 defense] validate_tool_call('apeireth_plugin_load', _) = {:?}",
        validate_tool_call("apeireth_plugin_load", &bad)
    );
    println!();

    // §7 PluginPermission 4 种
    println!("[§2 PluginPermission] SUPPORTED_PERMISSITIONS = {SUPPORTED_PERMISSIONS:#?}\n");

    // §8 PluginLifecycle 5 状态 + 状态机
    println!("[§2 PluginLifecycle] can_transition_to tests:");
    println!(
        "  Loaded    -> Initialized = {}",
        PluginLifecycle::Loaded.can_transition_to(PluginLifecycle::Initialized)
    );
    println!(
        "  Initialized -> Ready    = {}",
        PluginLifecycle::Initialized.can_transition_to(PluginLifecycle::Ready)
    );
    println!(
        "  Ready     -> Unloaded   = {}",
        PluginLifecycle::Ready.can_transition_to(PluginLifecycle::Unloaded)
    );
    println!(
        "  Unloaded  -> Destroyed  = {}",
        PluginLifecycle::Unloaded.can_transition_to(PluginLifecycle::Destroyed)
    );
    println!(
        "  Loaded    -> Ready (非法) = {}",
        PluginLifecycle::Loaded.can_transition_to(PluginLifecycle::Ready)
    );
    println!();

    // §9 FileRead perm sanity
    let _perm = PluginPermission::FileRead;
    println!("[§2 PluginPermission] FileRead copy check OK\n");

    // §10 临时 PathBuf 演示
    let pb = PathBuf::from("src/lib.rs");
    println!("[§3 install] entry path = {}", pb.display());

    println!("\n=== demo done ===");
    Ok(())
}
