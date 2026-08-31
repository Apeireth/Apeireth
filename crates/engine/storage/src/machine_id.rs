//! Cross-platform machine fingerprint recovered from
//! `legacy/archived/apeireth-machine-id`.
//!
//! Probe chains (donor 1:1):
//! - Windows: `wmic csproduct get uuid` then `reg query … MachineGuid`
//! - macOS: `ioreg -rd1 -c IOPlatformExpertDevice` → `IOPlatformUUID`
//! - Linux: DMI product_uuid → D-Bus machine-id → `/etc/machine-id`
//! - BSD: `kenv smbios.system.uuid` then `/etc/hostid`
//!
//! SHA-256 of the raw id is **not** computed here (would add `sha2`). Callers
//! that already depend on `sha2` (credentials) can hash `MachineIdProbe::raw`.

use std::process::Command;

/// Detected platform.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Platform {
    /// Microsoft Windows.
    Windows,
    /// Apple macOS.
    Darwin,
    /// Linux.
    Linux,
    /// BSD family.
    Bsd,
    /// Anything else.
    Unsupported,
}

impl Platform {
    /// Compile-time platform.
    pub fn detect() -> Self {
        if cfg!(windows) {
            Platform::Windows
        } else if cfg!(target_os = "macos") {
            Platform::Darwin
        } else if cfg!(target_os = "linux") {
            Platform::Linux
        } else if cfg!(any(
            target_os = "freebsd",
            target_os = "openbsd",
            target_os = "netbsd",
            target_os = "dragonfly"
        )) {
            Platform::Bsd
        } else {
            Platform::Unsupported
        }
    }

    /// Stable name.
    pub fn as_str(&self) -> &'static str {
        match self {
            Platform::Windows => "windows",
            Platform::Darwin => "darwin",
            Platform::Linux => "linux",
            Platform::Bsd => "bsd",
            Platform::Unsupported => "unsupported",
        }
    }
}

/// Probe result: raw identifier + source label + platform.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MachineIdProbe {
    /// Raw platform identifier (UUID / machine-id / hostid hex).
    pub raw: String,
    /// Source tag (`wmi`, `registry`, `ioreg`, `dmi`, `dbus`, `etc`, `kenv`, `hostid`).
    pub source: String,
    /// Platform the probe ran on.
    pub platform: Platform,
}

/// Probe failures.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum MachineIdError {
    /// Current OS is not one of the four supported families.
    #[error("unsupported platform")]
    UnsupportedPlatform,
    /// Windows WMI command failed.
    #[error("wmi command failed: {0}")]
    WmiCommand(String),
    /// Windows registry query failed.
    #[error("windows registry query failed: {0}")]
    WindowsRegistry(String),
    /// macOS ioreg failed.
    #[error("ioreg command failed: {0}")]
    IoregCommand(String),
    /// All three Linux sources failed.
    #[error("linux machine-id not found (DMI/DBus/ETC all failed): {0}")]
    LinuxAllSourcesFailed(String),
    /// BSD kenv failed (and hostid also failed when reported here).
    #[error("kenv command failed: {0}")]
    KenvCommand(String),
    /// File I/O.
    #[error("machine-id I/O error: {0}")]
    Io(String),
}

/// Windows WMI command.
pub const WIN_WMI_COMMAND: &str = "wmic";
/// Windows WMI args.
pub const WIN_WMI_ARGS: &[&str] = &["csproduct", "get", "uuid"];
/// Windows `reg` command.
pub const WIN_REG_QUERY_COMMAND: &str = "reg";
/// Windows `reg` args for MachineGuid.
pub const WIN_REG_QUERY_ARGS: &[&str] = &[
    "query",
    r"HKLM\SOFTWARE\Microsoft\Cryptography",
    "/v",
    "MachineGuid",
];
/// macOS ioreg command.
pub const DARWIN_IOREG_COMMAND: &str = "ioreg";
/// macOS ioreg args.
pub const DARWIN_IOREG_ARGS: &[&str] = &["-rd1", "-c", "IOPlatformExpertDevice"];
/// Linux DMI UUID path.
pub const LINUX_DMI_PATH: &str = "/sys/class/dmi/id/product_uuid";
/// Linux D-Bus machine-id path.
pub const LINUX_DBUS_PATH: &str = "/var/lib/dbus/machine-id";
/// Linux systemd machine-id path.
pub const LINUX_ETC_PATH: &str = "/etc/machine-id";
/// BSD kenv command.
pub const BSD_KENV_COMMAND: &str = "kenv";
/// BSD kenv SMBIOS variable.
pub const BSD_KENV_VAR: &str = "smbios.system.uuid";
/// BSD hostid path.
pub const BSD_HOSTID_PATH: &str = "/etc/hostid";

/// Probe the current machine. Sync; uses `std::process::Command` / `std::fs`.
pub fn probe_machine_id() -> Result<MachineIdProbe, MachineIdError> {
    let platform = Platform::detect();
    let (raw, source) = match platform {
        Platform::Windows => {
            #[cfg(windows)]
            {
                probe_windows()?
            }
            #[cfg(not(windows))]
            {
                return Err(MachineIdError::UnsupportedPlatform);
            }
        }
        Platform::Darwin => {
            #[cfg(target_os = "macos")]
            {
                probe_darwin()?
            }
            #[cfg(not(target_os = "macos"))]
            {
                return Err(MachineIdError::UnsupportedPlatform);
            }
        }
        Platform::Linux => {
            #[cfg(target_os = "linux")]
            {
                probe_linux()?
            }
            #[cfg(not(target_os = "linux"))]
            {
                return Err(MachineIdError::UnsupportedPlatform);
            }
        }
        Platform::Bsd => {
            #[cfg(any(
                target_os = "freebsd",
                target_os = "openbsd",
                target_os = "netbsd",
                target_os = "dragonfly"
            ))]
            {
                probe_bsd()?
            }
            #[cfg(not(any(
                target_os = "freebsd",
                target_os = "openbsd",
                target_os = "netbsd",
                target_os = "dragonfly"
            )))]
            {
                return Err(MachineIdError::UnsupportedPlatform);
            }
        }
        Platform::Unsupported => return Err(MachineIdError::UnsupportedPlatform),
    };
    Ok(MachineIdProbe {
        raw,
        source,
        platform,
    })
}

/// Parse `wmic csproduct get uuid` stdout. Public so tests can exercise the
/// parser without spawning WMI.
pub fn parse_wmi_uuid(stdout: &str) -> Option<String> {
    for line in stdout.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("UUID") {
            continue;
        }
        if looks_like_uuid(trimmed) {
            return Some(trimmed.to_string());
        }
    }
    None
}

/// Parse `reg query … MachineGuid` stdout.
pub fn parse_registry_machine_guid(stdout: &str) -> Option<String> {
    for line in stdout.lines() {
        if line.contains("MachineGuid") {
            if let Some(val) = line.split_whitespace().last() {
                let trimmed = val.trim();
                if !trimmed.is_empty() {
                    return Some(trimmed.to_string());
                }
            }
        }
    }
    None
}

/// Extract the quoted value from an ioreg `IOPlatformUUID` line.
///
/// Donor format: `"IOPlatformUUID" = "AAAAAAAA-BBBB-..."` — the UUID is the
/// **second** quoted token, not the key.
pub fn extract_quoted_value(line: &str) -> Option<&str> {
    let first = line.find('"')?;
    let after_first = &line[first + 1..];
    let end_first = after_first.find('"')?;
    let rest = &after_first[end_first + 1..];
    let second = rest.find('"')?;
    let after_second = &rest[second + 1..];
    let end_second = after_second.find('"')?;
    Some(&after_second[..end_second])
}

/// Parse ioreg stdout for IOPlatformUUID.
pub fn parse_ioreg_uuid(stdout: &str) -> Option<String> {
    for line in stdout.lines() {
        if line.contains("IOPlatformUUID") {
            if let Some(uuid) = extract_quoted_value(line) {
                let trimmed = uuid.trim();
                if !trimmed.is_empty() {
                    return Some(trimmed.to_string());
                }
            }
        }
    }
    None
}

/// Hex-encode up to 8 bytes of `/etc/hostid`.
pub fn encode_hostid(bytes: &[u8]) -> Option<String> {
    if bytes.is_empty() {
        return None;
    }
    let n = bytes.len().min(8);
    Some(bytes[..n].iter().map(|b| format!("{b:02x}")).collect())
}

fn looks_like_uuid(s: &str) -> bool {
    s.len() == 36 && s.chars().filter(|c| *c == '-').count() == 4
}

#[cfg(windows)]
fn probe_windows() -> Result<(String, String), MachineIdError> {
    match run_command(WIN_WMI_COMMAND, WIN_WMI_ARGS) {
        Ok(stdout) => {
            if let Some(raw) = parse_wmi_uuid(&stdout) {
                return Ok((raw, "wmi".to_string()));
            }
        }
        Err(e) => {
            let _ = e;
        }
    }
    let stdout = run_command(WIN_REG_QUERY_COMMAND, WIN_REG_QUERY_ARGS)
        .map_err(|e| MachineIdError::WindowsRegistry(e))?;
    parse_registry_machine_guid(&stdout)
        .map(|raw| (raw, "registry".to_string()))
        .ok_or_else(|| {
            MachineIdError::WindowsRegistry(format!("no MachineGuid in reg output: {stdout}"))
        })
}

#[cfg(target_os = "macos")]
fn probe_darwin() -> Result<(String, String), MachineIdError> {
    let stdout = run_command(DARWIN_IOREG_COMMAND, DARWIN_IOREG_ARGS)
        .map_err(MachineIdError::IoregCommand)?;
    parse_ioreg_uuid(&stdout)
        .map(|raw| (raw, "ioreg".to_string()))
        .ok_or_else(|| {
            MachineIdError::IoregCommand(format!(
                "no IOPlatformUUID in ioreg output: {} bytes",
                stdout.len()
            ))
        })
}

#[cfg(target_os = "linux")]
fn probe_linux() -> Result<(String, String), MachineIdError> {
    let mut last_err: Option<String> = None;
    match read_trimmed(LINUX_DMI_PATH) {
        Ok(raw) if !raw.is_empty() && !raw.contains("None") && !raw.contains("To Be Filled") => {
            return Ok((raw, "dmi".to_string()));
        }
        Ok(_) => last_err = Some("DMI empty/placeholder".to_string()),
        Err(e) => last_err = Some(format!("DMI: {e}")),
    }
    match read_trimmed(LINUX_DBUS_PATH) {
        Ok(raw) if !raw.is_empty() => return Ok((raw, "dbus".to_string())),
        Ok(_) => last_err = Some("DBus empty".to_string()),
        Err(e) => last_err = Some(format!("DBus: {e}")),
    }
    match read_trimmed(LINUX_ETC_PATH) {
        Ok(raw) if !raw.is_empty() => return Ok((raw, "etc".to_string())),
        Ok(_) => last_err = Some("ETC empty".to_string()),
        Err(e) => last_err = Some(format!("ETC: {e}")),
    }
    Err(MachineIdError::LinuxAllSourcesFailed(
        last_err.unwrap_or_else(|| "all 3 sources unavailable".to_string()),
    ))
}

#[cfg(any(
    target_os = "freebsd",
    target_os = "openbsd",
    target_os = "netbsd",
    target_os = "dragonfly"
))]
fn probe_bsd() -> Result<(String, String), MachineIdError> {
    if let Ok(stdout) = run_command(BSD_KENV_COMMAND, &[BSD_KENV_VAR]) {
        let raw = stdout.trim().to_string();
        if !raw.is_empty() {
            return Ok((raw, "kenv".to_string()));
        }
    }
    let bytes = std::fs::read(BSD_HOSTID_PATH).map_err(|e| MachineIdError::Io(e.to_string()))?;
    encode_hostid(&bytes)
        .map(|raw| (raw, "hostid".to_string()))
        .ok_or_else(|| MachineIdError::KenvCommand("empty /etc/hostid".to_string()))
}

#[cfg(any(
    windows,
    target_os = "macos",
    any(
        target_os = "freebsd",
        target_os = "openbsd",
        target_os = "netbsd",
        target_os = "dragonfly"
    )
))]
fn run_command(cmd: &str, args: &[&str]) -> Result<String, String> {
    let output = Command::new(cmd)
        .args(args)
        .output()
        .map_err(|e| format!("spawn failed: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "exit {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

#[cfg(target_os = "linux")]
fn read_trimmed(path: &str) -> Result<String, std::io::Error> {
    std::fs::read_to_string(path).map(|s| s.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn win_commands_hardcoded() {
        assert_eq!(WIN_WMI_COMMAND, "wmic");
        assert_eq!(WIN_WMI_ARGS, &["csproduct", "get", "uuid"]);
        assert_eq!(WIN_REG_QUERY_COMMAND, "reg");
        assert!(WIN_REG_QUERY_ARGS[1].contains("Cryptography"));
        assert!(WIN_REG_QUERY_ARGS.contains(&"MachineGuid"));
    }

    #[test]
    fn darwin_ioreg_hardcoded() {
        assert_eq!(DARWIN_IOREG_COMMAND, "ioreg");
        assert_eq!(DARWIN_IOREG_ARGS, &["-rd1", "-c", "IOPlatformExpertDevice"]);
    }

    #[test]
    fn linux_three_fallback_paths() {
        assert_eq!(LINUX_DMI_PATH, "/sys/class/dmi/id/product_uuid");
        assert_eq!(LINUX_DBUS_PATH, "/var/lib/dbus/machine-id");
        assert_eq!(LINUX_ETC_PATH, "/etc/machine-id");
    }

    #[test]
    fn bsd_kenv_and_hostid_hardcoded() {
        assert_eq!(BSD_KENV_COMMAND, "kenv");
        assert_eq!(BSD_KENV_VAR, "smbios.system.uuid");
        assert_eq!(BSD_HOSTID_PATH, "/etc/hostid");
    }

    #[test]
    fn parse_wmi_skips_header() {
        let stdout = "UUID\r\nAAAAAAAA-BBBB-CCCC-DDDD-EEEEEEEEEEEE\r\n\r\n";
        assert_eq!(
            parse_wmi_uuid(stdout).as_deref(),
            Some("AAAAAAAA-BBBB-CCCC-DDDD-EEEEEEEEEEEE")
        );
    }

    #[test]
    fn parse_registry_takes_last_column() {
        let stdout = r#"    MachineGuid    REG_SZ    AAAAAAAA-BBBB-CCCC-DDDD-EEEEEEEEEEEE"#;
        assert_eq!(
            parse_registry_machine_guid(stdout).as_deref(),
            Some("AAAAAAAA-BBBB-CCCC-DDDD-EEEEEEEEEEEE")
        );
    }

    #[test]
    fn extract_quoted_value_parses_ioreg_output() {
        let line = r#"    "IOPlatformUUID" = "AAAAAAAA-BBBB-CCCC-DDDD-EEEEEEEEEEEE""#;
        assert_eq!(
            extract_quoted_value(line),
            Some("AAAAAAAA-BBBB-CCCC-DDDD-EEEEEEEEEEEE")
        );
        assert_eq!(
            parse_ioreg_uuid(line).as_deref(),
            Some("AAAAAAAA-BBBB-CCCC-DDDD-EEEEEEEEEEEE")
        );
    }

    #[test]
    fn hostid_hex_encodes_up_to_8_bytes() {
        assert_eq!(
            encode_hostid(&[0x12, 0x34, 0x56, 0x78]).as_deref(),
            Some("12345678")
        );
        assert_eq!(encode_hostid(&[]), None);
        let long = [1u8, 2, 3, 4, 5, 6, 7, 8, 9];
        assert_eq!(encode_hostid(&long).unwrap().len(), 16);
    }

    #[test]
    fn platform_detect_is_one_of_five() {
        let p = Platform::detect();
        assert!(matches!(
            p,
            Platform::Windows
                | Platform::Darwin
                | Platform::Linux
                | Platform::Bsd
                | Platform::Unsupported
        ));
    }
}
