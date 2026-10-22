//! Recon / 外部工具的生产接线开关（2026-10-22 接线批）。
//!
//! 三个环境变量，默认全关（fail-closed）：
//!
//! * [`ENABLE_RECON_ENV`] —— 注册 9 个 `tool.recon.*` 原生侦察工具；
//! * [`ENABLE_EXTERNAL_ENV`] —— 注册 10 个 `ext.*` 外部适配器；
//! * [`TOOL_SCOPE_ENV`] —— 授权范围 JSON（路径 / 域名 / CIDR）。缺省 = 空范围，
//!   网络类工具（`net.recon` / `web.probe` / `re.tls` / `ext.nmap` / `ext.openssl`）
//!   一律拒绝；文件类工具仍受 workspace 根限域。
//!
//! # 为什么开关语义放在本模块
//!
//! 「开关名 = 开关权」：装配侧注册与治理侧 grant 读**同一个函数**，不各自解析
//! env，避免出现「注册了但权力没到手」或「授了权但没注册」的两张皮。
//!
//! # 边界
//!
//! * 只认显式 `"1"`（前后空白容忍）；`0` / `true` / `on` / `11` 都不算开。
//! * scope 文件解析失败 = 启动失败并给出可读错误，**绝不静默降级为空范围**
//!   （静默失败会让人误以为「授权生效」，实际全拒绝或全放行，两种误判都不可接受）。
//! * scope 结构校验复用 [`Scope::validate`]：未显式 `authorized: true`、清单全空、
//!   CIDR 畸形都算坏配置。

use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::external::ExternalToolsConfig;
use crate::plugin::ReconConfig;
use crate::recon::Scope;

/// 注册 9 个 `tool.recon.*` 原生侦察工具（默认关）。
pub const ENABLE_RECON_ENV: &str = "APEIRETH_ENABLE_RECON";
/// 注册 10 个 `ext.*` 外部适配器（默认关）。
pub const ENABLE_EXTERNAL_ENV: &str = "APEIRETH_ENABLE_EXTERNAL";
/// 授权范围文件（JSON）。缺省 = 空范围（网络目标全拒绝）。
pub const TOOL_SCOPE_ENV: &str = "APEIRETH_TOOL_SCOPE";

/// 空范围的固定名（写进 Evidence 的 `scope` 字段，便于事后追查）。
const EMPTY_SCOPE_NAME: &str = "未配置 APEIRETH_TOOL_SCOPE（默认空范围，网络目标全拒绝）";

fn switch_on(key: &str) -> bool {
    std::env::var(key)
        .ok()
        .is_some_and(|value| value.trim() == "1")
}

/// `APEIRETH_ENABLE_RECON` 是否为显式开启。
pub fn recon_enabled_from_env() -> bool {
    switch_on(ENABLE_RECON_ENV)
}

/// `APEIRETH_ENABLE_EXTERNAL` 是否为显式开启。
pub fn external_enabled_from_env() -> bool {
    switch_on(ENABLE_EXTERNAL_ENV)
}

/// 读取授权范围：设了 `APEIRETH_TOOL_SCOPE` 就解析并校验该 JSON；未设（或空白）
/// 返回空范围 —— 空范围在 [`Scope::validate`] 处即失败，网络目标全拒绝。
pub fn scope_from_env() -> Result<Arc<Scope>, String> {
    match std::env::var(TOOL_SCOPE_ENV) {
        Ok(value) if !value.trim().is_empty() => load_scope(Path::new(value.trim())),
        _ => Ok(Arc::new(Scope::empty(EMPTY_SCOPE_NAME))),
    }
}

/// 从文件加载并校验 scope（启动期失败即报错，不降级）。
pub fn load_scope(path: &Path) -> Result<Arc<Scope>, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|error| format!("scope 文件读取失败 {}: {error}", path.display()))?;
    let scope: Scope = serde_json::from_str(&text)
        .map_err(|error| format!("scope JSON 解析失败 {}: {error}", path.display()))?;
    scope
        .validate()
        .map_err(|error| format!("scope 校验失败 {}: {error}", path.display()))?;
    Ok(Arc::new(scope))
}

/// recon 族配置：开关关 = `None`（不注册，零行为变化）。
pub fn recon_config_from_env() -> Result<Option<ReconConfig>, String> {
    if !recon_enabled_from_env() {
        return Ok(None);
    }
    Ok(Some(ReconConfig {
        scope: scope_from_env()?,
    }))
}

/// 外部适配器配置：开关关 = `None`；开关开 = 带 workspace 根与 scope。
pub fn external_config_from_env(
    workspace_root: impl Into<PathBuf>,
) -> Result<Option<ExternalToolsConfig>, String> {
    if !external_enabled_from_env() {
        return Ok(None);
    }
    Ok(Some(ExternalToolsConfig {
        workspace_root: workspace_root.into(),
        scope: scope_from_env()?,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_scope(name: &str, body: &str) -> PathBuf {
        let path =
            std::env::temp_dir().join(format!("apeireth-scope-{name}-{}.json", std::process::id()));
        std::fs::write(&path, body).expect("write scope");
        path
    }

    #[test]
    fn load_scope_accepts_a_valid_authorization() {
        let path = write_scope(
            "ok",
            r#"{"name":"lab","authorized":true,"paths":["/tmp"],"domains":["example.com"],"cidrs":["127.0.0.1/32"]}"#,
        );
        let scope = load_scope(&path).expect("valid scope");
        assert_eq!(scope.name, "lab");
        assert!(scope.authorized);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn load_scope_refuses_missing_authorization() {
        let path = write_scope("noauth", r#"{"name":"lab","paths":["/tmp"]}"#);
        let error = load_scope(&path).expect_err("未显式授权必须拒绝");
        assert!(error.contains("scope 校验失败"), "{error}");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn load_scope_refuses_empty_lists_and_malformed_cidr() {
        let empty = write_scope("empty", r#"{"name":"lab","authorized":true}"#);
        assert!(load_scope(&empty).is_err(), "清单全空必须拒绝");
        let _ = std::fs::remove_file(&empty);

        let bad = write_scope(
            "badcidr",
            r#"{"name":"lab","authorized":true,"cidrs":["10.0.0.0/99"]}"#,
        );
        assert!(load_scope(&bad).is_err(), "畸形 CIDR 必须拒绝");
        let _ = std::fs::remove_file(&bad);
    }

    #[test]
    fn load_scope_refuses_unreadable_file_and_bad_json() {
        let missing = std::env::temp_dir().join("apeireth-scope-does-not-exist.json");
        assert!(load_scope(&missing).is_err(), "缺文件必须报错");

        let bad = write_scope("badjson", "{not json");
        assert!(load_scope(&bad).is_err(), "坏 JSON 必须报错");
        let _ = std::fs::remove_file(&bad);
    }
}

// ────────────────────────────────────────────────────────────────────────────
// 治理档位：能力 id 清单（真源在此，装配侧与授权侧都读它，禁止各自手抄）
// ────────────────────────────────────────────────────────────────────────────

/// 只读本地分析的 6 个能力 id：**grant 即用，零审批**（与 `tool.repo` 同档）。
pub const RECON_LOCAL_TOOL_IDS: &[&str] = &[
    "tool.recon.pack",
    "tool.recon.binfmt",
    "tool.recon.yara",
    "tool.recon.dotnet",
    "tool.recon.js",
    "tool.recon.pcap",
];

/// 出网 3 个能力 id：**grant + 逐次人工审批**（发报文属执行档，与 fetch 同档）。
pub const RECON_NETWORK_TOOL_IDS: &[&str] = &["tool.recon.net", "tool.recon.web", "tool.recon.tls"];

/// 10 个外部适配器**能力 id**：**grant + 逐次人工审批**（执行第三方二进制属执行档）。
///
/// 注意：模型侧调用名是 `ext.nmap` 这类短名，**能力 id 是 `tool.external.nmap`**。
/// 治理授权按能力 id 判定，按模型名授权等于没授 —— 两者混用是这里最容易踩的坑，
/// 故 `declared_id_lists_match_the_registered_sets` 直接钉死真实注册集。
pub const EXTERNAL_TOOL_IDS: &[&str] = &[
    "tool.external.nmap",
    "tool.external.exiftool",
    "tool.external.tshark",
    "tool.external.yara",
    "tool.external.binwalk",
    "tool.external.radare2",
    "tool.external.jadx",
    "tool.external.apktool",
    "tool.external.ghidra",
    "tool.external.openssl",
];

#[cfg(test)]
mod id_list_tests {
    use super::*;
    use crate::external::external_tool_set;
    use crate::recon_tool::recon_tool_set;

    fn scratch(name: &str) -> PathBuf {
        let path =
            std::env::temp_dir().join(format!("apeireth-recon-{name}-{}", std::process::id()));
        std::fs::create_dir_all(&path).expect("scratch dir");
        path
    }

    fn ids(tools: &[Arc<dyn apeireth_plugin::ToolCapability>]) -> Vec<String> {
        let mut list: Vec<String> = tools.iter().map(|t| t.id().as_str().to_string()).collect();
        list.sort();
        list
    }

    /// 名实相符闸：声明的能力 id 清单必须与真实注册集**逐一相等**。
    /// 任何一侧加/删工具而忘了同步清单，这里直接红。
    #[test]
    fn declared_id_lists_match_the_registered_sets() {
        let root = scratch("ids");
        let scope = Arc::new(Scope::empty("id-check"));

        let recon = ids(&recon_tool_set(root.clone(), Arc::clone(&scope)));
        let mut expected: Vec<String> = RECON_LOCAL_TOOL_IDS
            .iter()
            .chain(RECON_NETWORK_TOOL_IDS)
            .map(|id| (*id).to_string())
            .collect();
        expected.sort();
        assert_eq!(recon, expected, "recon 能力 id 清单与注册集不一致");

        let external = ids(&external_tool_set(&ExternalToolsConfig {
            workspace_root: root.clone(),
            scope,
        }));
        let mut expected_ext: Vec<String> = EXTERNAL_TOOL_IDS
            .iter()
            .map(|id| (*id).to_string())
            .collect();
        expected_ext.sort();
        assert_eq!(external, expected_ext, "外部适配器 id 清单与注册集不一致");

        let _ = std::fs::remove_dir_all(&root);
    }

    /// 清单自身无重复、无错位（本地档与出网档不得交叠）。
    #[test]
    fn declared_id_lists_are_disjoint_and_deduplicated() {
        let mut all: Vec<&str> = Vec::new();
        all.extend_from_slice(RECON_LOCAL_TOOL_IDS);
        all.extend_from_slice(RECON_NETWORK_TOOL_IDS);
        all.extend_from_slice(EXTERNAL_TOOL_IDS);
        let unique: std::collections::BTreeSet<&str> = all.iter().copied().collect();
        assert_eq!(unique.len(), all.len(), "能力 id 清单出现重复条目");

        let local: std::collections::BTreeSet<&str> =
            RECON_LOCAL_TOOL_IDS.iter().copied().collect();
        let network: std::collections::BTreeSet<&str> =
            RECON_NETWORK_TOOL_IDS.iter().copied().collect();
        assert!(
            local.is_disjoint(&network),
            "本地档与出网档不得交叠（审批档位不同）"
        );
    }
}
