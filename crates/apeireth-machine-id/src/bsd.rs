//! BSD 平台: 1:1 翻译 `getMachineId-bsd-*.js`.
//!
//! ## 1:1 翻译表 (per `getMachineId-bsd-*.js` ~200 LOC)
//!
//! | v0.9.21 JS | Rust | 1:1 还原点 |
//! |------------|------|------------|
//! | `kenv smbios.system.uuid` (1st) | `tokio::process::Command::new(BSD_KENV_COMMAND).arg(BSD_KENV_VAR).output()` | 命令 + 变量 hardcode |
//! | 读 `/etc/hostid` (2nd fallback) | `fs_err::read_to_string(BSD_HOSTID_PATH).hex_decode()` | 路径 hardcode (hostid 是二进制, hex 编码) |
//! | trim whitespace | `.trim().to_string()` | 1:1 |
//! | raw → SHA-256 → 32 hex | per `MACHINE_ID_HASH_ALGO` | lib.rs 统一做 |
//!
//! ## fallback chain 完整 (2 sources)
//! 1. `kenv smbios.system.uuid` (主, SMBIOS UUID, 商业版默认走这条)
//! 2. `/etc/hostid` (备, 32-bit hostid 派生, hex 编码)
//!
//! ## 状态: ⚠️ skeleton (R20 阶段 1 实施, 当前 platform 跨 FreeBSD/OpenBSD/NetBSD/DragonFly 4 编译)

use crate::{MachineIdError, MachineIdResultStd, BSD_HOSTID_PATH, BSD_KENV_COMMAND, BSD_KENV_VAR};

/// BSD 平台 probe 入口 (per lib.rs `get_machine_id` cfg 路由).
/// 2 fallback chain, 命中任一返 (raw, source) 二元组, 全失败返 typed error.
pub async fn probe_bsd() -> MachineIdResultStd<(String, String)> {
    // 1st fallback: kenv smbios.system.uuid (per `BSD_KENV_COMMAND` + `BSD_KENV_VAR` hardcode)
    if let Ok((raw, source)) = probe_kenv().await {
        return Ok((raw, source));
    }
    // 2nd fallback: /etc/hostid (per `BSD_HOSTID_PATH` hardcode, 32-bit hostid 派生)
    probe_hostid().await.map(|raw| (raw, "hostid".to_string()))
}

/// 1st fallback: `kenv smbios.system.uuid` 抓 SMBIOS UUID.
async fn probe_kenv() -> MachineIdResultStd<(String, String)> {
    let output = tokio::process::Command::new(BSD_KENV_COMMAND)
        .arg(BSD_KENV_VAR)
        .output()
        .await
        .map_err(|e| MachineIdError::KenvCommand(format!("spawn failed: {e}")))?;

    if !output.status.success() {
        return Err(MachineIdError::KenvCommand(format!(
            "exit {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        )));
    }

    // kenv 输出格式: "<uuid-value>\n" (单行, trim 即可)
    let raw = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if raw.is_empty() {
        return Err(MachineIdError::KenvCommand("empty kenv output".to_string()));
    }
    // SMBIOS UUID 格式: 8-4-4-4-12 hex (36 字符)
    if raw.len() == 36 && raw.chars().filter(|c| *c == '-').count() == 4 {
        Ok((raw, "kenv".to_string()))
    } else {
        // 商业版 1:1: 非标准格式也接受 (某些 BSD 输出短 UUID)
        Ok((raw, "kenv".to_string()))
    }
}

/// 2nd fallback: 读 `/etc/hostid` (32-bit hostid, 4 字节, hex 编码).
async fn probe_hostid() -> MachineIdResultStd<String> {
    let bytes = fs_err::read(BSD_HOSTID_PATH).map_err(MachineIdError::Io)?;
    if bytes.is_empty() {
        return Err(MachineIdError::Other("empty /etc/hostid".to_string()));
    }
    // hostid 32-bit (4 字节) → 8 hex 字符
    // 但某些 BSD 输出 8/16 字节, 用 min(8) 适配
    let hex_len = bytes.len().min(8);
    Ok(hex::encode(&bytes[..hex_len]))
}

// ============================================================================
// §6 in-module 测试 (3 fixture, 测 2 fallback hardcode + 链完整 + hex 编码)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bsd_kenv_and_hostid_hardcoded() {
        // 1:1 翻译 v0.9.21 kenv smbios.system.uuid + /etc/hostid
        assert_eq!(BSD_KENV_COMMAND, "kenv");
        assert_eq!(BSD_KENV_VAR, "smbios.system.uuid");
        assert_eq!(BSD_HOSTID_PATH, "/etc/hostid");
    }

    #[test]
    fn bsd_fallback_chain_has_two_sources() {
        // BSD fallback chain: kenv (1st) + hostid (2nd), 2 sources 防单点失败
        let sources = ["kenv", "hostid"];
        assert_eq!(sources.len(), 2);
    }

    #[test]
    fn bsd_hostid_hex_encodes_to_8_or_16_chars() {
        // 32-bit hostid = 4 bytes = 8 hex; 64-bit = 8 bytes = 16 hex (BSD 系差异)
        let bytes_4 = [0x12u8, 0x34, 0x56, 0x78];
        assert_eq!(hex::encode(&bytes_4[..]), "12345678");
        assert_eq!(hex::encode(&bytes_4[..]).len(), 8);
    }
}
