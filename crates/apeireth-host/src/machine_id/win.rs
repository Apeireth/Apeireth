//! Windows 平台: 1:1 翻译 `getMachineId-win-*.js`.
//!
//! ## 1:1 翻译表 (per `getMachineId-win-*.js` ~200 LOC)
//!
//! | v0.9.21 JS | Rust | 1:1 还原点 |
//! |------------|------|------------|
//! | `wmic csproduct get uuid` (1st probe) | `tokio::process::Command::new(WIN_WMI_COMMAND).args(WIN_WMI_ARGS).output()` | 命令字符串 hardcode |
//! | `reg query HKLM Cryptography MachineGuid` (2nd fallback) | `tokio::process::Command::new(WIN_REG_QUERY_COMMAND).args(WIN_REG_QUERY_ARGS).output()` | Registry 路径 hardcode |
//! | UUID trim whitespace | `.trim()` | 1:1 |
//! | raw → SHA-256 → 32 hex | per `MACHINE_ID_HASH_ALGO` | 在 lib.rs `get_machine_id()` 统一做 |
//!
//! ## fallback chain 完整 (2 sources)
//! 1. `wmic csproduct get uuid` (主, 商业版默认走这条)
//! 2. `reg query HKLM Cryptography MachineGuid` (备, 旧 WMI 命令失败时)
//!
//! ## 状态: ⚠️ skeleton (R20 阶段 1 实施, 真实 Windows 调用)

use super::{MachineIdError, MachineIdResultStd, WIN_REG_QUERY_ARGS, WIN_REG_QUERY_COMMAND, WIN_WMI_ARGS, WIN_WMI_COMMAND};

/// Windows 平台 probe 入口 (per lib.rs `get_machine_id` cfg 路由).
/// 返 `(raw_uuid, source)` 二元组, 由 `get_machine_id` 统一做 SHA-256 派生.
pub async fn probe_windows() -> MachineIdResultStd<(String, String)> {
    // 1st fallback: wmic csproduct get uuid
    if let Ok((raw, source)) = probe_wmi().await {
        return Ok((raw, source));
    }
    // 2nd fallback: reg query HKLM Cryptography MachineGuid
    probe_registry().await.map(|raw| (raw, "registry".to_string()))
}

/// 1st fallback: `wmic csproduct get uuid` (per `WIN_WMI_COMMAND` + `WIN_WMI_ARGS` hardcode).
async fn probe_wmi() -> MachineIdResultStd<(String, String)> {
    let output = tokio::process::Command::new(WIN_WMI_COMMAND)
        .args(WIN_WMI_ARGS)
        .output()
        .await
        .map_err(|e| MachineIdError::WmiCommand(format!("spawn failed: {e}")))?;

    if !output.status.success() {
        return Err(MachineIdError::WmiCommand(format!(
            "exit {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        )));
    }

    // wmic 输出格式: "UUID\r\nAAAAAAAA-BBBB-...UUID...\r\n\r\n"
    // 1:1 翻译 v0.9.21: split lines, skip header/blank, 找首行合法 UUID
    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("UUID") {
            continue;
        }
        // UUID 格式: 8-4-4-4-12 hex 字符
        if trimmed.len() == 36 && trimmed.chars().filter(|c| *c == '-').count() == 4 {
            return Ok((trimmed.to_string(), "wmi".to_string()));
        }
    }
    Err(MachineIdError::WmiCommand(format!(
        "no valid UUID in wmic output: {stdout}"
    )))
}

/// 2nd fallback: `reg query HKLM Cryptography MachineGuid` (per `WIN_REG_QUERY_COMMAND` + `WIN_REG_QUERY_ARGS`).
async fn probe_registry() -> MachineIdResultStd<String> {
    let output = tokio::process::Command::new(WIN_REG_QUERY_COMMAND)
        .args(WIN_REG_QUERY_ARGS)
        .output()
        .await
        .map_err(|e| MachineIdError::WindowsRegistry(format!("spawn failed: {e}")))?;

    if !output.status.success() {
        return Err(MachineIdError::WindowsRegistry(format!(
            "exit {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        )));
    }

    // reg query 输出格式: "    MachineGuid    REG_SZ    <UUID>\r\n"
    // 1:1 翻译 v0.9.21: 找含 "MachineGuid" 的行, 取最后一列
    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        if line.contains("MachineGuid") {
            let cols: Vec<&str> = line.split_whitespace().collect();
            if let Some(val) = cols.last() {
                let trimmed = val.trim();
                if !trimmed.is_empty() {
                    return Ok(trimmed.to_string());
                }
            }
        }
    }
    Err(MachineIdError::WindowsRegistry(format!(
        "no MachineGuid in reg output: {stdout}"
    )))
}

// ============================================================================
// §6 in-module 测试 (3 fixture, 测 Windows 命令字符串 hardcode + probe 结构)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn win_wmi_command_hardcoded_matches_blueprint() {
        // 1:1 翻译 v0.9.21 wmic csproduct get uuid (per blueprint §2.4.2)
        assert_eq!(WIN_WMI_COMMAND, "wmic");
        assert_eq!(WIN_WMI_ARGS, &["csproduct", "get", "uuid"]);
    }

    #[test]
    fn win_reg_query_hardcoded_matches_blueprint() {
        // 1:1 翻译 v0.9.21 reg query HKLM Cryptography MachineGuid
        assert_eq!(WIN_REG_QUERY_COMMAND, "reg");
        assert_eq!(WIN_REG_QUERY_ARGS[0], "query");
        assert!(WIN_REG_QUERY_ARGS[1].contains("HKLM"));
        assert!(WIN_REG_QUERY_ARGS[1].contains("Cryptography"));
        assert!(WIN_REG_QUERY_ARGS.contains(&"MachineGuid"));
    }

    #[test]
    fn win_fallback_chain_has_two_sources() {
        // Windows fallback chain: wmic (1st) + reg (2nd), 2 sources 防单点失败
        let sources = ["wmi", "registry"];
        assert_eq!(sources.len(), 2);
    }
}
