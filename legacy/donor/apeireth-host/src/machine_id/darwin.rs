//! macOS 平台: 1:1 翻译 `getMachineId-darwin-*.js`.
//!
//! ## 1:1 翻译表 (per `getMachineId-darwin-*.js` ~200 LOC)
//!
//! | v0.9.21 JS | Rust | 1:1 还原点 |
//! |------------|------|------------|
//! | `ioreg -rd1 -c IOPlatformExpertDevice` | `tokio::process::Command::new(DARWIN_IOREG_COMMAND).args(DARWIN_IOREG_ARGS).output()` | 命令字符串 hardcode |
//! | `grep IOPlatformUUID` parse | 行扫描 + `contains("IOPlatformUUID")` | 1:1 |
//! | raw → SHA-256 → 32 hex | per `MACHINE_ID_HASH_ALGO` | lib.rs `get_machine_id()` 统一做 |
//!
//! ## fallback chain 完整 (1 source + 1 兜底)
//! 1. `ioreg -rd1 -c IOPlatformExpertDevice` 抓 `IOPlatformUUID` (主)
//! 2. 系统调用失败 → 返 `MachineIdError::IoregCommand` (兜底, 由 lib.rs fallback 到 next platform 或报 Unsupported)
//!
//! macOS 商业版 1:1 是单源 (ioreg), 本 crate 保留 1:1, 兜底由 lib.rs `Platform::Unsupported` 路径处理.
//!
//! ## 状态: ⚠️ skeleton (R20 阶段 1 实施)

use super::{MachineIdError, MachineIdResultStd, DARWIN_IOREG_ARGS, DARWIN_IOREG_COMMAND};

/// macOS 平台 probe 入口 (per lib.rs `get_machine_id` cfg 路由).
/// 返 `(raw_uuid, source)` 二元组, 由 `get_machine_id` 统一做 SHA-256 派生.
pub async fn probe_darwin() -> MachineIdResultStd<(String, String)> {
    probe_ioreg().await
}

/// `ioreg -rd1 -c IOPlatformExpertDevice` 抓 `IOPlatformUUID` (per `DARWIN_IOREG_COMMAND` + `DARWIN_IOREG_ARGS` hardcode).
async fn probe_ioreg() -> MachineIdResultStd<(String, String)> {
    let output = tokio::process::Command::new(DARWIN_IOREG_COMMAND)
        .args(DARWIN_IOREG_ARGS)
        .output()
        .await
        .map_err(|e| MachineIdError::IoregCommand(format!("spawn failed: {e}")))?;

    if !output.status.success() {
        return Err(MachineIdError::IoregCommand(format!(
            "exit {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        )));
    }

    // ioreg 输出 (plist-like):
    //   {
    //     ...
    //     "IOPlatformUUID" = "AAAAAAAA-BBBB-CCCC-DDDD-EEEEEEEEEEEE"
    //     ...
    //   }
    // 1:1 翻译 v0.9.21: 找含 "IOPlatformUUID" 的行, 提取引号内字符串
    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        if line.contains("IOPlatformUUID") {
            if let Some(uuid) = extract_quoted_value(line) {
                let trimmed = uuid.trim();
                if !trimmed.is_empty() {
                    return Ok((trimmed.to_string(), "ioreg".to_string()));
                }
            }
        }
    }
    Err(MachineIdError::IoregCommand(format!(
        "no IOPlatformUUID in ioreg output: {} bytes",
        stdout.len()
    )))
}

/// 从 `key = "value"` 形式提取引号内 value (1:1 翻译 v0.9.21 正则).
///
/// CI fix 2026-08: 原实现取第一个引号对 → 拿到的是 key ("IOPlatformUUID")
/// 而非 value (ioreg 行: `"IOPlatformUUID" = "UUID..."`), macOS CI 必挂.
/// 正确: 找 `= "` 之后的引号对 (key 的引号在 `=` 之前).
fn extract_quoted_value(line: &str) -> Option<&str> {
    let eq = line.find('=')?;
    let rest = &line[eq + 1..];
    let start = rest.find('"')?;
    let rest = &rest[start + 1..];
    let end = rest.find('"')?;
    Some(&rest[..end])
}

// ============================================================================
// §6 in-module 测试 (2 fixture, 测 macOS 命令字符串 hardcode + extract 函数)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn darwin_ioreg_command_hardcoded_matches_blueprint() {
        // 1:1 翻译 v0.9.21 ioreg -rd1 -c IOPlatformExpertDevice
        assert_eq!(DARWIN_IOREG_COMMAND, "ioreg");
        assert_eq!(DARWIN_IOREG_ARGS, &["-rd1", "-c", "IOPlatformExpertDevice"]);
    }

    #[test]
    fn extract_quoted_value_parses_ioreg_output() {
        // 模拟 ioreg 输出行: "    \"IOPlatformUUID\" = \"AAAAAAAA-BBBB-CCCC-DDDD-EEEEEEEEEEEE\""
        let line = r#"    "IOPlatformUUID" = "AAAAAAAA-BBBB-CCCC-DDDD-EEEEEEEEEEEE""#;
        let val = extract_quoted_value(line);
        assert_eq!(val, Some("AAAAAAAA-BBBB-CCCC-DDDD-EEEEEEEEEEEE"));
    }
}
