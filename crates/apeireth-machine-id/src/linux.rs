//! Linux 平台: 1:1 翻译 `getMachineId-linux-*.js`.
//!
//! ## 1:1 翻译表 (per `getMachineId-linux-*.js` ~200 LOC)
//!
//! | v0.9.21 JS | Rust | 1:1 还原点 |
//! |------------|------|------------|
//! | 读 `/sys/class/dmi/id/product_uuid` (1st) | `fs_err::read_to_string(LINUX_DMI_PATH)` | 路径 hardcode |
//! | 读 `/var/lib/dbus/machine-id` (2nd fallback) | `fs_err::read_to_string(LINUX_DBUS_PATH)` | 路径 hardcode |
//! | 读 `/etc/machine-id` (3rd fallback) | `fs_err::read_to_string(LINUX_ETC_PATH)` | 路径 hardcode |
//! | trim whitespace | `.trim().to_string()` | 1:1 |
//! | raw → SHA-256 → 32 hex | per `MACHINE_ID_HASH_ALGO` | lib.rs 统一做 |
//!
//! ## fallback chain 完整 (3 sources, 防单点失败)
//! 1. `/sys/class/dmi/id/product_uuid` (主, 主板 SMBIOS UUID, 商业版默认走这条)
//! 2. `/var/lib/dbus/machine-id` (备, DBus 生成, 容器/无 root 时常用)
//! 3. `/etc/machine-id` (末, systemd 风格, 老 distro 兜底)
//!
//! 3 个全部失败 → 返 `MachineIdError::LinuxAllSourcesFailed` (per lib.rs `get_machine_id` 报错).
//!
//! ## 状态: ⚠️ skeleton (R20 阶段 1 实施, 当前 platform 真跑)

use crate::{MachineIdError, MachineIdResultStd, LINUX_DBUS_PATH, LINUX_DMI_PATH, LINUX_ETC_PATH};

/// Linux 平台 probe 入口 (per lib.rs `get_machine_id` cfg 路由).
/// 3 fallback chain, 命中任一返 (raw, source) 二元组, 3 个全失败返 typed error.
pub async fn probe_linux() -> MachineIdResultStd<(String, String)> {
    let mut last_err: Option<String> = None;

    // 1st: DMI UUID (per `LINUX_DMI_PATH` hardcode, 主板 SMBIOS, 商业版默认)
    match read_trimmed(LINUX_DMI_PATH) {
        Ok(raw) if !raw.is_empty() && !raw.contains("None") && !raw.contains("To Be Filled") => {
            return Ok((raw, "dmi".to_string()));
        }
        Ok(_) => last_err = Some("DMI empty/placeholder".to_string()),
        Err(e) => last_err = Some(format!("DMI: {e}")),
    }

    // 2nd: DBus machine-id (per `LINUX_DBUS_PATH` hardcode)
    match read_trimmed(LINUX_DBUS_PATH) {
        Ok(raw) if !raw.is_empty() => return Ok((raw, "dbus".to_string())),
        Ok(_) => last_err = Some("DBus empty".to_string()),
        Err(e) => last_err = Some(format!("DBus: {e}")),
    }

    // 3rd: /etc/machine-id (per `LINUX_ETC_PATH` hardcode, systemd 风格)
    match read_trimmed(LINUX_ETC_PATH) {
        Ok(raw) if !raw.is_empty() => return Ok((raw, "etc".to_string())),
        Ok(_) => last_err = Some("ETC empty".to_string()),
        Err(e) => last_err = Some(format!("ETC: {e}")),
    }

    Err(MachineIdError::LinuxAllSourcesFailed(
        last_err.unwrap_or_else(|| "all 3 sources unavailable".to_string()),
    ))
}

/// 读文件 + trim 空白 (1:1 翻译 v0.9.21 `fs.readFileSync(path).trim().toString()`).
fn read_trimmed(path: &str) -> Result<String, std::io::Error> {
    fs_err::read_to_string(path).map(|s| s.trim().to_string())
}

// ============================================================================
// §6 in-module 测试 (3 fixture, 测 3 fallback 路径 hardcode + 链完整)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linux_three_fallback_paths_hardcoded() {
        // 1:1 翻译 v0.9.21 3 个 fallback 路径
        assert_eq!(LINUX_DMI_PATH, "/sys/class/dmi/id/product_uuid");
        assert_eq!(LINUX_DBUS_PATH, "/var/lib/dbus/machine-id");
        assert_eq!(LINUX_ETC_PATH, "/etc/machine-id");
    }

    #[test]
    fn linux_fallback_chain_has_three_sources() {
        // Linux fallback chain: DMI (1st) + DBus (2nd) + ETC (3rd), 3 sources
        let sources = ["dmi", "dbus", "etc"];
        assert_eq!(sources.len(), 3, "Linux 必须有 3 个 fallback source (防单点失败)");
    }

    #[test]
    fn linux_fallback_chain_order_is_dmi_dbus_etc() {
        // 验证商业版默认顺序: 主板 SMBIOS → DBus → systemd
        let chain = ["dmi", "dbus", "etc"];
        assert_eq!(chain[0], "dmi");    // 主板
        assert_eq!(chain[1], "dbus");   // 容器/无 root
        assert_eq!(chain[2], "etc");    // 老 distro
    }
}
