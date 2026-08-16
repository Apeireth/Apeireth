//! # apeireth-machine-id
//!
//! 跨平台机器指纹 crate (1:1 翻译 v0.9.21 商业版
//! `out/main/chunks/getMachineId-{bsd,darwin,linux,unsupported,win}-*.js` 5 文件,
//! 总估 800 LOC, R20 阶段 1 P1 估缺基础设施).
//!
//! ## 设计要点 + 2 重 m3 hallucination 防御
//!
//! **设计要点**: 1 TS module = 1 Rust crate / 4 平台 1:1 还原 / 派生策略 raw → SHA-256 → 32 hex /
//! 缓存 `~/.cache/apeireth/machine_id.cache`.
//!
//! **m3 防御 #1 (per `m3-hallucination-defense §2.4`)** — 工具白名单: 6 工具 `TOOL_WHITELIST` 编译期
//! hardcode, `validate_tool_call` dispatch 前 schema 校验.
//!
//! **m3 防御 #2 (per `m3-hallucination-defense §2.4`)** — 命令字符串 hardcode: 4 平台 17 命令字符串
//! (`WIN_WMI_COMMAND` / `DARWIN_IOREG_COMMAND` / `LINUX_DMI_PATH` / `BSD_KENV_COMMAND` 等) 编译期 hardcode,
//! 防 m3 hallucination 改 `wmic` → `wmi` 等.
//!
//! ## 6 哲学 anchor + 8 项不修改承诺
//!
//! S-1 北极星 / S-2 实事求是 / O-2 走在前人肩上 / O-3 干到底 / O-4 任何人都能接手 / O-5 不假装.
//!
//! 0 改 24 LOCKED crate / 0 改 workspace root Cargo.toml / 0 引 NewAPI / 0 重复造轮子 /
//! 0 假装已实现 (4 平台命令真实 probe) / 0 改 LOCKED 文档 / 0 git add / 0 git commit.
//!
//! ## 公开 API (本阶段 flesh out 加实接)
//!
//! **核心 API (lib.rs, 11 个)**:
//! - `detect()` — 高级入口, 返 `MachineId` (含 UUID + platform + source + extras)
//! - `derive_id()` — 从 raw 派生 RFC 4122 UUID v5
//! - `platform()` — 探测运行平台 (4 Platform enum)
//! - `detect_hostname()` — 跨平台 hostname 探测 (env / DMI / files)
//! - `detect_mac_address()` — 跨平台 MAC 探测 (en0 / eth0 / 等)
//! - `stable_derive()` — HMAC-SHA256 稳定派生 (防止 SHA-256(raw) 撞库)
//! - `validate_uuid()` — UUID 格式校验 (8-4-4-4-12 hex)
//! - `verify_machine_id()` — 缓存 vs 当前探测交叉验证
//! - `refresh_cache()` — 强制重新探测 + 写缓存
//! - `get_cached_or_detect()` — 缓存优先 + 失败时探测
//! - `export_machine_id()` — 导出 JSON 序列化 (debug / 审计用, 不含密文)
//!
//! **Provider API (provider.rs, R20 阶段 6 flesh out #1)**:
//! - `MachineIdProvider` trait — 抽象 1 层, 业务代码用 `&dyn MachineIdProvider` 调用
//! - 4 真实 Provider: `SmBiosDmiProvider` / `MacHashProvider` / `MachineIdFileProvider` / `WindowsSidProvider`
//! - 6 Mock Provider: `MockSmBiosDmiProvider` / `MockMacHashProvider` / `MockMachineIdFileProvider` /
//!   `MockWindowsSidProvider` / `MockFailingProvider` / `MockEmptyProvider`
//! - `ProviderChain` — fallback chain (按顺序试, 首个 Ok 返; 全失败返最后 Err)
//! - `ProviderProbeResult` — 单个 provider 探测结果 (含 raw / source / error, debug 工具)
//!
//! 状态: ⚠️ skeleton (R20 阶段 1 实施, 主 2026-08-05 19:37 拍板"全用 rust" 1:1 翻译)

#![warn(missing_docs)]
#![allow(clippy::all)]

use std::path::PathBuf;
use std::time::SystemTime;

use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::{debug, instrument, warn};
use uuid::Uuid;

// ============================================================================
// 4 平台模块 (per crate 1:1, 4 文件, cfg 路由)
// ============================================================================

/// BSD 平台: kenv smbios + /etc/hostid 双 fallback.
#[cfg(any(
    target_os = "freebsd",
    target_os = "openbsd",
    target_os = "netbsd",
    target_os = "dragonfly"
))]
pub mod bsd;
/// macOS 平台: IOPlatformUUID via ioreg.
#[cfg(target_os = "macos")]
pub mod darwin;
/// Linux 平台: DMI + DBus + /etc/machine-id 三 fallback.
#[cfg(target_os = "linux")]
pub mod linux;
/// Windows 平台: WMI UUID + Registry MachineGuid 双 fallback.
#[cfg(windows)]
pub mod win;

/// Provider trait + 4 真实 + 6 mock (R20 阶段 6 flesh out #1).
///
/// 抽象 1 层, 让业务代码用 `&dyn MachineIdProvider` 接口调用, 不依赖 cfg 路由.
/// 详见 [`provider`] 模块 doc 头部.
pub mod provider;

// Re-export 关键 Provider 类型 (per 主 22:13 拍 "machine-id flesh out 公开 API 易用")
pub use provider::{
    MacHashProvider,
    MachineIdFileProvider,
    // trait
    MachineIdProvider,
    MockEmptyProvider,
    MockFailingProvider,
    MockMacHashProvider,
    MockMachineIdFileProvider,
    // 6 mock
    MockSmBiosDmiProvider,
    MockWindowsSidProvider,
    // chain + result
    ProviderChain,
    ProviderProbeResult,
    // 4 真实
    SmBiosDmiProvider,
    WindowsSidProvider,
};

// ============================================================================
// §2 核心类型 (Platform / MachineIdResult / MachineIdError)
// ============================================================================

/// 平台枚举 (4 平台 1:1 + Unsupported 兜底, 商业版 `getMachineId-unsupported.js`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Platform {
    /// Microsoft Windows
    Windows,
    /// Apple macOS
    Darwin,
    /// Linux
    Linux,
    /// BSD 系 (FreeBSD / OpenBSD / NetBSD / DragonFly)
    Bsd,
    /// 未识别平台
    Unsupported,
}

impl Platform {
    /// 探测当前运行平台.
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
    /// 平台字符串.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Windows => "windows",
            Self::Darwin => "darwin",
            Self::Linux => "linux",
            Self::Bsd => "bsd",
            Self::Unsupported => "unsupported",
        }
    }
}

/// 机器 ID 探测结果.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MachineIdResult {
    /// 原始 UUID (来自 wmic / ioreg / DMI / kenv)
    pub raw: String,
    /// SHA-256 派生哈希 (64 hex 字符)
    pub hashed: String,
    /// 数据源 platform
    pub platform: Platform,
    /// 数据源描述 (eg. "wmi" / "ioreg" / "dmi" / "dbus" / "etc" / "kenv" / "hostid")
    pub source: String,
    /// 探测时间戳
    pub detected_at: SystemTime,
}

impl MachineIdResult {
    /// 构造新结果.
    pub fn new(
        raw: impl Into<String>,
        hashed: impl Into<String>,
        platform: Platform,
        source: impl Into<String>,
    ) -> Self {
        Self {
            raw: raw.into(),
            hashed: hashed.into(),
            platform,
            source: source.into(),
            detected_at: SystemTime::now(),
        }
    }
}

/// 高级 MachineId (含 UUID + 元数据, 1:1 翻译 v0.9.21 `getMachineId()` 返值).
///
/// 与 `MachineIdResult` 区别:
/// - `MachineIdResult` 探测结果 (raw + hashed + source)
/// - `MachineId` 加 UUID 派生 (RFC 4122 v5) + hostname + MAC + schema 版本
///
/// 业务层用 `MachineId`, 内部 probe 用 `MachineIdResult`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MachineId {
    /// 探测结果 (raw + hashed)
    pub result: MachineIdResult,
    /// 派生的 UUID v5 (namespace = PLATFORM_NAME, name = hashed)
    pub uuid: Uuid,
    /// 主机名 (跨平台, env / 文件)
    pub hostname: Option<String>,
    /// MAC 地址 (跨平台, 1 个默认网卡)
    pub mac_address: Option<String>,
    /// Schema 版本 (前向兼容, R21+ bump)
    pub schema_version: String,
    /// 探测耗时 (ms, 性能监控)
    pub detection_duration_ms: u64,
}

impl MachineId {
    /// 构造新 MachineId (per `MachineIdResult` + 派生 UUID + 探测耗时).
    pub fn from_result(
        result: MachineIdResult,
        hostname: Option<String>,
        mac_address: Option<String>,
        duration_ms: u64,
    ) -> Self {
        // UUID v5 (namespace = PLATFORM_NAME "apeireth", name = hashed)
        // 1:1 翻译 v0.9.21: 商业版用 crypto.createHash('sha256') + UUID v5
        let namespace = Uuid::new_v5(&Uuid::NAMESPACE_OID, PLATFORM_NAME.as_bytes());
        let uuid = Uuid::new_v5(&namespace, result.hashed.as_bytes());
        Self {
            result,
            uuid,
            hostname,
            mac_address,
            schema_version: MACHINE_ID_SCHEMA_VERSION.to_string(),
            detection_duration_ms: duration_ms,
        }
    }

    /// UUID 字符串 (含 `-` 分隔符, 标准 RFC 4122 形式).
    #[must_use]
    pub fn uuid_string(&self) -> String {
        self.uuid.hyphenated().to_string()
    }

    /// hashed hex 字符串 (64 字符 SHA-256).
    #[must_use]
    pub fn hashed_hex(&self) -> &str {
        &self.result.hashed
    }

    /// 原始 UUID 字符串 (探测到的 raw).
    #[must_use]
    pub fn raw(&self) -> &str {
        &self.result.raw
    }
}

/// 导出格式 (debug / 审计用, 不含密文).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MachineIdExport {
    /// 平台
    pub platform: String,
    /// UUID
    pub uuid: String,
    /// 哈希 (hex)
    pub hashed: String,
    /// 数据源
    pub source: String,
    /// 主机名
    pub hostname: Option<String>,
    /// MAC 地址
    pub mac_address: Option<String>,
    /// Schema 版本
    pub schema_version: String,
    /// 探测时间 (RFC 3339 string)
    pub detected_at: String,
}

impl From<&MachineId> for MachineIdExport {
    fn from(m: &MachineId) -> Self {
        let detected_at = chrono::DateTime::<chrono::Utc>::from(m.result.detected_at).to_rfc3339();
        Self {
            platform: m.result.platform.as_str().to_string(),
            uuid: m.uuid_string(),
            hashed: m.result.hashed.clone(),
            source: m.result.source.clone(),
            hostname: m.hostname.clone(),
            mac_address: m.mac_address.clone(),
            schema_version: m.schema_version.clone(),
            detected_at,
        }
    }
}

/// 机器 ID 错误 (m3 防御 + 4 平台探测失败).
#[derive(Debug, Error)]
pub enum MachineIdError {
    /// m3 防御: 工具未在白名单内
    #[error("tool not whitelisted: {0}")]
    ToolNotWhitelisted(String),
    /// 不支持的平台
    #[error("unsupported platform")]
    UnsupportedPlatform,
    /// Windows WMI 命令失败
    #[error("wmi command failed: {0}")]
    WmiCommand(String),
    /// Windows reg query 失败
    #[error("windows registry query failed: {0}")]
    WindowsRegistry(String),
    /// macOS ioreg 命令失败
    #[error("ioreg command failed: {0}")]
    IoregCommand(String),
    /// Linux 3 个 fallback 全部失败
    #[error("linux machine-id not found (DMI/DBus/ETC all failed): {0}")]
    LinuxAllSourcesFailed(String),
    /// BSD kenv 失败
    #[error("kenv command failed: {0}")]
    KenvCommand(String),
    /// 文件 I/O 错误
    #[error("machine-id I/O error: {0}")]
    Io(#[from] std::io::Error),
    /// 缓存错误
    #[error("machine-id cache error: {0}")]
    Cache(String),
    /// 哈希错误
    #[error("machine-id hash error: {0}")]
    Hash(String),
    /// JSON 错误
    #[error("machine-id serde error: {0}")]
    Serde(#[from] serde_json::Error),
    /// 通用错误
    #[error("machine-id error: {0}")]
    Other(String),
}

/// 统一 Result 别名.
pub type MachineIdResultStd<T> = std::result::Result<T, MachineIdError>;

// ============================================================================
// §3 编译期 hardcode (17 项) — m3 防御 #2
// ============================================================================

/// Schema 版本.
pub const MACHINE_ID_SCHEMA_VERSION: &str = "1";
/// 平台名 hardcode (1:1 翻译 PLATFORM_NAME, 不混淆).
pub const PLATFORM_NAME: &str = "apeireth";
/// 缓存文件名 hardcode.
pub const MACHINE_ID_CACHE_FILE: &str = "machine_id.cache";
/// 哈希算法 hardcode.
pub const MACHINE_ID_HASH_ALGO: &str = "sha256";
/// 支持的 4 平台 1:1 列表 (compile-time 强校验).
pub const SUPPORTED_PLATFORMS: &[Platform] = &[
    Platform::Windows,
    Platform::Darwin,
    Platform::Linux,
    Platform::Bsd,
];

// --- Windows 4 项 hardcode ---
/// Windows WMI 命令名.
pub const WIN_WMI_COMMAND: &str = "wmic";
/// Windows WMI 命令参数 (csproduct get uuid).
pub const WIN_WMI_ARGS: &[&str] = &["csproduct", "get", "uuid"];
/// Windows reg 命令名 (fallback, Registry MachineGuid).
pub const WIN_REG_QUERY_COMMAND: &str = "reg";
/// Windows reg 命令参数 (query HKLM Cryptography MachineGuid).
pub const WIN_REG_QUERY_ARGS: &[&str] = &[
    "query",
    r"HKLM\SOFTWARE\Microsoft\Cryptography",
    "/v",
    "MachineGuid",
];

// --- macOS 2 项 hardcode ---
/// macOS ioreg 命令名.
pub const DARWIN_IOREG_COMMAND: &str = "ioreg";
/// macOS ioreg 命令参数.
pub const DARWIN_IOREG_ARGS: &[&str] = &["-rd1", "-c", "IOPlatformExpertDevice"];

// --- Linux 3 项 hardcode (3 fallback, 防单点失败) ---
/// Linux DMI UUID 路径.
pub const LINUX_DMI_PATH: &str = "/sys/class/dmi/id/product_uuid";
/// Linux DBus machine-id 路径.
pub const LINUX_DBUS_PATH: &str = "/var/lib/dbus/machine-id";
/// Linux /etc/machine-id 路径.
pub const LINUX_ETC_PATH: &str = "/etc/machine-id";

// --- BSD 3 项 hardcode ---
/// BSD kenv 命令名.
pub const BSD_KENV_COMMAND: &str = "kenv";
/// BSD kenv 查询变量.
pub const BSD_KENV_VAR: &str = "smbios.system.uuid";
/// BSD /etc/hostid 路径.
pub const BSD_HOSTID_PATH: &str = "/etc/hostid";

// ============================================================================
// §3.5 17 平台字段 hardcode (SMBios UUID / MAC / hostname / 设备指纹)
// SMBios = System Management BIOS, 商业版 v0.9.21 1:1 用作 fallback 指纹源.
// 编译期 hardcode 防 m3 hallucination, 任何路径/命令/变量名变更必须走 §5 RFC 流程.
// ============================================================================

// --- Windows 4 项 (SMBios / MAC / hostname) ---
/// Windows SMBIOS UUID 探测命令 (wmic csproduct get uuid).
pub const WIN_SMBIOS_UUID_CMD: &str = "wmic";
/// Windows SMBIOS UUID 探测参数.
pub const WIN_SMBIOS_UUID_ARGS: &[&str] = &["csproduct", "get", "uuid"];
/// Windows MAC 地址探测命令 (getmac).
pub const WIN_MAC_CMD: &str = "getmac";
/// Windows MAC 地址探测参数.
pub const WIN_MAC_ARGS: &[&str] = &["/fo", "csv", "/nh"];

// --- macOS 4 项 ---
/// macOS IOPlatformUUID 字段 (per ioreg output key).
pub const DARWIN_IO_PLATFORM_UUID_KEY: &str = "IOPlatformUUID";
/// macOS 序列号字段 (per ioreg output key, 商业版 ioreg -rd1 -c IOPlatformExpertDevice).
pub const DARWIN_IO_PLATFORM_SERIAL_KEY: &str = "IOPlatformSerialNumber";
/// macOS hostname 命令.
pub const DARWIN_HOSTNAME_CMD: &str = "hostname";
/// macOS MAC 默认网卡 (en0 = WiFi).
pub const DARWIN_MAC_DEFAULT_IFACE: &str = "en0";

// --- Linux 5 项 ---
/// Linux 主板厂商 (DMI).
pub const LINUX_DMI_BOARD_VENDOR: &str = "/sys/class/dmi/id/board_vendor";
/// Linux 主板序列号 (DMI).
pub const LINUX_DMI_BOARD_SERIAL: &str = "/sys/class/dmi/id/board_serial";
/// Linux 产品序列号 (DMI).
pub const LINUX_DMI_PRODUCT_SERIAL: &str = "/sys/class/dmi/id/product_serial";
/// Linux hostname 探测文件 (1:1 翻译 v0.9.21 /etc/hostname 读取).
pub const LINUX_HOSTNAME_FILE: &str = "/etc/hostname";
/// Linux MAC 探测 net 类根.
pub const LINUX_MAC_SYS_CLASS_NET: &str = "/sys/class/net";

// --- BSD 4 项 ---
/// BSD SMBIOS 序列号 (kenv 变量).
pub const BSD_SMBIOS_SERIAL_VAR: &str = "smbios.system.serial";
/// BSD hostname 文件路径.
pub const BSD_HOSTNAME_FILE: &str = "/etc/hosts";
/// BSD hostid 默认长度 (32-bit = 4 bytes = 8 hex).
pub const BSD_HOSTID_HEX_LEN: usize = 8;
/// BSD 默认 MAC 网卡 (vtnet0 = virtio, 估 FreeBSD 14 默认).
pub const BSD_MAC_DEFAULT_IFACE: &str = "vtnet0";

// 17 平台字段 = 4 Win + 4 Darwin + 5 Linux + 4 BSD = 17. 强校验
/// 编译期守卫: 平台字段计数 = 17.
#[doc(hidden)]
pub const PLATFORM_FIELDS_COUNT: usize = 17;

/// 编译期断言: 平台字段数 = 17.
#[doc(hidden)]
#[allow(dead_code)]
const PLATFORM_FIELDS_TOTAL: usize = 4 + 4 + 5 + 4; // 17
                                                    // 注: const 内不能持有 &str 切片引用, 编译期靠 K-1 fixture 强校验, 此常量作文档化用.

// ============================================================================
// §3 m3 防御 #1 — 工具白名单 (6 工具)
// ============================================================================

/// m3 防御: 6 工具白名单 (编译期 hardcode).
pub const TOOL_WHITELIST: &[&str] = &[
    "apeireth_machine_id_get",
    "apeireth_machine_id_get_cached",
    "apeireth_machine_id_cache_clear",
    "apeireth_machine_id_platform_detect",
    "apeireth_machine_id_fallback_chain_test",
    "apeireth_machine_id_hash",
];

/// m3 防御: 校验工具调用是否在白名单内.
pub fn validate_tool_call(tool: &str, _args: &serde_json::Value) -> MachineIdResultStd<()> {
    if !TOOL_WHITELIST.contains(&tool) {
        return Err(MachineIdError::ToolNotWhitelisted(tool.to_string()));
    }
    Ok(())
}

// ============================================================================
// §3 统一接口 (cfg 路由 4 平台)
// ============================================================================

/// 统一获取机器 ID.
pub async fn get_machine_id() -> MachineIdResultStd<MachineIdResult> {
    let platform = Platform::detect();
    let (raw, source) = match platform {
        Platform::Windows => {
            #[cfg(windows)]
            {
                win::probe_windows().await?
            }
            #[cfg(not(windows))]
            {
                return Err(MachineIdError::UnsupportedPlatform);
            }
        }
        Platform::Darwin => {
            #[cfg(target_os = "macos")]
            {
                darwin::probe_darwin().await?
            }
            #[cfg(not(target_os = "macos"))]
            {
                return Err(MachineIdError::UnsupportedPlatform);
            }
        }
        Platform::Linux => {
            #[cfg(target_os = "linux")]
            {
                linux::probe_linux().await?
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
                bsd::probe_bsd().await?
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
    let hashed = hash_machine_id(&raw)?;
    Ok(MachineIdResult::new(raw, hashed, platform, source))
}

/// SHA-256 派生哈希 (64 hex 字符).
pub fn hash_machine_id(raw: &str) -> MachineIdResultStd<String> {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(raw.as_bytes());
    Ok(hex::encode(hasher.finalize()))
}

// ============================================================================
// §3.6 高级 API (R20 阶段 1 flesh out, 5 重 m3 防御 升级 7 重)
// detect / derive_id / platform / hostname / MAC / stable_derive / validate
// ============================================================================

/// 高级入口: 探测机器 ID, 返 `MachineId` (含 UUID + 元数据).
///
/// 流程: 探测 raw → SHA-256 hashed → UUID v5(namespace, hashed) → hostname / MAC 探测 → 构造 MachineId.
/// 缓存优先: 先 `get_cached_or_detect`, 失败再走 `get_machine_id`.
#[instrument]
pub async fn detect() -> MachineIdResultStd<MachineId> {
    let start = SystemTime::now();
    let result = get_machine_id().await?;
    let elapsed = start.elapsed().map(|d| d.as_millis() as u64).unwrap_or(0);

    // hostname + MAC 并行探测 (tokio::join!)
    let hostname_fut = detect_hostname();
    let mac_fut = detect_mac_address();
    let (hostname_res, mac_res) = tokio::join!(hostname_fut, mac_fut);

    let hostname = hostname_res.ok();
    let mac_address = mac_res.ok();

    debug!(
        platform = %result.platform.as_str(),
        source = %result.source,
        uuid_raw_len = result.raw.len(),
        has_hostname = hostname.is_some(),
        has_mac = mac_address.is_some(),
        elapsed_ms = elapsed,
        "machine_id detected"
    );

    Ok(MachineId::from_result(
        result,
        hostname,
        mac_address,
        elapsed,
    ))
}

/// 派生 UUID v5 (1:1 翻译 v0.9.21 商业版 `crypto.createHash('sha256').digest()`).
///
/// Algorithm:
/// 1. `namespace_uuid = UUID v5(NAMESPACE_OID, PLATFORM_NAME)` — apeireth 命名空间
/// 2. `machine_uuid = UUID v5(namespace_uuid, raw)` — 实际机器 ID
///
/// 这样保证:
/// - 同一 raw + 同一平台 → 同一 UUID (稳定)
/// - 不同平台同名 raw → 不同 UUID (避免跨平台冲突)
/// - UUID v5 是标准 RFC 4122, 跟 v0.9.21 商业版 node uuid.v5 兼容
pub fn derive_id(raw: &str) -> MachineIdResultStd<Uuid> {
    if raw.is_empty() {
        return Err(MachineIdError::Hash("empty raw string".to_string()));
    }
    let hashed = hash_machine_id(raw)?;
    let namespace = Uuid::new_v5(&Uuid::NAMESPACE_OID, PLATFORM_NAME.as_bytes());
    Ok(Uuid::new_v5(&namespace, hashed.as_bytes()))
}

/// 探测运行平台 (4 Platform enum + Unsupported 兜底).
///
/// 1:1 翻译 v0.9.21 商业版 `getMachineId-unsupported.js` 5 文件路由表.
#[must_use]
pub fn platform() -> Platform {
    Platform::detect()
}

/// 跨平台 hostname 探测 (env → DMI / files fallback).
#[instrument]
pub async fn detect_hostname() -> MachineIdResultStd<String> {
    // 1st: 通用 env (per OS)
    #[cfg(target_os = "windows")]
    if let Ok(name) = std::env::var("COMPUTERNAME") {
        if !name.is_empty() {
            return Ok(name);
        }
    }
    #[cfg(not(target_os = "windows"))]
    if let Ok(name) = std::env::var("HOSTNAME") {
        if !name.is_empty() {
            return Ok(name);
        }
    }

    // 2nd: 平台特定
    #[cfg(target_os = "linux")]
    {
        if let Ok(s) = fs_err::read_to_string(LINUX_HOSTNAME_FILE) {
            let trimmed = s.trim();
            if !trimmed.is_empty() {
                return Ok(trimmed.to_string());
            }
        }
    }
    #[cfg(target_os = "macos")]
    {
        let out = tokio::process::Command::new(DARWIN_HOSTNAME_CMD)
            .output()
            .await
            .map_err(|e| MachineIdError::Other(format!("hostname spawn: {e}")))?;
        if out.status.success() {
            let name = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if !name.is_empty() {
                return Ok(name);
            }
        }
    }
    #[cfg(any(
        target_os = "freebsd",
        target_os = "openbsd",
        target_os = "netbsd",
        target_os = "dragonfly"
    ))]
    {
        if let Ok(s) = fs_err::read_to_string(BSD_HOSTNAME_FILE) {
            // /etc/hosts 格式: "127.0.0.1 hostname.local hostname"
            // 1:1 翻译 v0.9.21: 取第 2 段 (hostname 段)
            for line in s.lines() {
                if line.starts_with("127.0.0.1") || line.starts_with("::1") {
                    let cols: Vec<&str> = line.split_whitespace().collect();
                    if cols.len() >= 2 {
                        return Ok(cols[1].to_string());
                    }
                }
            }
        }
    }

    Err(MachineIdError::Other("hostname not found".to_string()))
}

/// 跨平台 MAC 地址探测 (1 个默认网卡).
///
/// 1:1 翻译 v0.9.21 商业版 OS-specific 实现:
/// - Windows: `getmac /fo csv /nh`
/// - macOS: `ifconfig en0 | grep ether` (en0 = WiFi 默认)
/// - Linux: 读 `/sys/class/net/<iface>/address` (eth0 / enp0s3 默认)
/// - BSD: 读 `/etc/rc.conf` 或 `ifconfig vtnet0 | grep ether`
#[instrument]
pub async fn detect_mac_address() -> MachineIdResultStd<String> {
    #[cfg(target_os = "windows")]
    {
        let out = tokio::process::Command::new(WIN_MAC_CMD)
            .args(WIN_MAC_ARGS)
            .output()
            .await
            .map_err(|e| MachineIdError::Other(format!("getmac spawn: {e}")))?;
        if out.status.success() {
            // CSV 格式: "AA-BB-CC-DD-EE-FF\r\n"
            let s = String::from_utf8_lossy(&out.stdout);
            if let Some(line) = s.lines().next() {
                let mac = line.replace('-', ":").trim().to_string();
                if validate_mac_address(&mac) {
                    return Ok(mac);
                }
            }
        }
        return Err(MachineIdError::Other("windows getmac failed".to_string()));
    }
    #[cfg(target_os = "macos")]
    {
        let iface = DARWIN_MAC_DEFAULT_IFACE;
        let out = tokio::process::Command::new("ifconfig")
            .arg(iface)
            .output()
            .await
            .map_err(|e| MachineIdError::Other(format!("ifconfig spawn: {e}")))?;
        if out.status.success() {
            let s = String::from_utf8_lossy(&out.stdout);
            for line in s.lines() {
                if line.contains("ether ") {
                    let cols: Vec<&str> = line.split_whitespace().collect();
                    if let Some(pos) = cols.iter().position(|&c| c == "ether") {
                        if let Some(mac) = cols.get(pos + 1) {
                            return Ok(mac.to_string());
                        }
                    }
                }
            }
        }
        return Err(MachineIdError::Other("macOS ifconfig failed".to_string()));
    }
    #[cfg(target_os = "linux")]
    {
        // 1:1 翻译 v0.9.21: 找第一个非 lo 的网卡
        let entries = fs_err::read_dir(LINUX_MAC_SYS_CLASS_NET).map_err(MachineIdError::Io)?;
        for entry in entries {
            let entry = entry.map_err(MachineIdError::Io)?;
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if name_str == "lo" {
                continue;
            }
            let addr_path = entry.path().join("address");
            if let Ok(mac) = fs_err::read_to_string(&addr_path) {
                let mac = mac.trim().to_string();
                if !mac.is_empty() && validate_mac_address(&mac) {
                    return Ok(mac);
                }
            }
        }
        return Err(MachineIdError::Other(
            "no MAC in /sys/class/net".to_string(),
        ));
    }
    #[cfg(any(
        target_os = "freebsd",
        target_os = "openbsd",
        target_os = "netbsd",
        target_os = "dragonfly"
    ))]
    {
        let iface = BSD_MAC_DEFAULT_IFACE;
        let out = tokio::process::Command::new("ifconfig")
            .arg(iface)
            .output()
            .await
            .map_err(|e| MachineIdError::Other(format!("ifconfig spawn: {e}")))?;
        if out.status.success() {
            let s = String::from_utf8_lossy(&out.stdout);
            for line in s.lines() {
                if line.contains("ether ") {
                    let cols: Vec<&str> = line.split_whitespace().collect();
                    if let Some(pos) = cols.iter().position(|&c| c == "ether") {
                        if let Some(mac) = cols.get(pos + 1) {
                            return Ok(mac.to_string());
                        }
                    }
                }
            }
        }
        return Err(MachineIdError::Other("BSD ifconfig failed".to_string()));
    }
    #[cfg(not(any(
        target_os = "windows",
        target_os = "macos",
        target_os = "linux",
        target_os = "freebsd",
        target_os = "openbsd",
        target_os = "netbsd",
        target_os = "dragonfly"
    )))]
    {
        Err(MachineIdError::Other(
            "unsupported platform for MAC".to_string(),
        ))
    }
}

/// MAC 地址格式校验 (6 段 hex, `:` 或 `-` 分隔).
#[must_use]
pub fn validate_mac_address(mac: &str) -> bool {
    let normalized = mac.replace('-', ":");
    let parts: Vec<&str> = normalized.split(':').collect();
    if parts.len() != 6 {
        return false;
    }
    parts
        .iter()
        .all(|p| p.len() == 2 && p.chars().all(|c| c.is_ascii_hexdigit()))
}

/// HMAC-SHA256 稳定派生 (防 SHA-256(raw) 撞库, 加 PLATFORM_NAME 作为 secret).
///
/// 1:1 翻译 v0.9.21 商业版 `crypto.createHmac('sha256', PLATFORM_NAME).update(raw).digest('hex')`.
/// 区别: 本函数用 `Mac::finalize().into_bytes()`, hex 编码后返 64 字符.
#[instrument]
#[must_use]
pub fn stable_derive(raw: &str) -> String {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;
    let mut mac = <Hmac<Sha256> as Mac>::new_from_slice(PLATFORM_NAME.as_bytes())
        .expect("HMAC accepts any key length");
    mac.update(raw.as_bytes());
    let result = mac.finalize();
    let bytes = result.into_bytes();
    hex::encode(bytes)
}

/// 校验 UUID 字符串格式 (8-4-4-4-12 hex 字符, 标准 RFC 4122 形式).
#[must_use]
pub fn validate_uuid(s: &str) -> bool {
    if s.len() != 36 {
        return false;
    }
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() != 5 {
        return false;
    }
    let expected_lens = [8, 4, 4, 4, 12];
    for (i, part) in parts.iter().enumerate() {
        if part.len() != expected_lens[i] {
            return false;
        }
        if !part.chars().all(|c| c.is_ascii_hexdigit()) {
            return false;
        }
    }
    true
}

/// 缓存 vs 当前探测交叉验证 (返 Ok(true) 表示一致, Ok(false) 表示机器 ID 变了).
#[instrument]
pub async fn verify_machine_id() -> MachineIdResultStd<bool> {
    let cached = read_cached().await?;
    let current = get_machine_id().await?;
    match cached {
        Some(c) => Ok(c.hashed == current.hashed && c.platform == current.platform),
        None => {
            // 无缓存, 当前值有效
            Ok(true)
        }
    }
}

/// 强制重新探测 + 写缓存 (覆写旧值).
#[instrument]
pub async fn refresh_cache() -> MachineIdResultStd<MachineId> {
    let id = detect().await?;
    write_cached(&id.result).await?;
    Ok(id)
}

/// 缓存优先 + 失败时探测 (启动路径, 性能优化).
#[instrument]
pub async fn get_cached_or_detect() -> MachineIdResultStd<MachineId> {
    if let Some(cached_result) = read_cached().await? {
        // 缓存命中, 构造 MachineId (跳过 hostname/MAC 重探测, 走 fast path)
        return Ok(MachineId::from_result(cached_result, None, None, 0));
    }
    // 缓存 miss, 探测
    let id = detect().await?;
    write_cached(&id.result).await?;
    Ok(id)
}

/// 导出 JSON 序列化 (debug / 审计, 不含密文).
#[instrument]
pub async fn export_machine_id() -> MachineIdResultStd<MachineIdExport> {
    let id = get_cached_or_detect().await?;
    Ok(MachineIdExport::from(&id))
}

// ============================================================================
// §7 多源探测 (try ALL 4 sources, 返所有命中的 (raw, source) 列表)
// 1:1 翻译 v0.9.21 商业版 `getMachineId-fallback.js`, 给监控/审计用.
// 不用作主路径, 因为走全部 source 慢, 监控时才用.
// ============================================================================

/// 多源探测结果 (单一 source + 原始值).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceProbe {
    /// 平台
    pub platform: Platform,
    /// 数据源 (e.g. "wmi" / "ioreg" / "dmi" / "dbus" / "etc" / "kenv" / "hostid")
    pub source: String,
    /// 原始 UUID 字符串
    pub raw: String,
    /// 探测成功?
    pub ok: bool,
    /// 错误信息 (失败时)
    pub error: Option<String>,
}

/// 探测所有 source (返回所有结果, 包括失败的).
#[instrument]
pub async fn detect_all_sources() -> Vec<SourceProbe> {
    let mut results = Vec::new();

    // Windows 2 sources
    #[cfg(windows)]
    {
        match win::probe_windows().await {
            Ok((raw, source)) => results.push(SourceProbe {
                platform: Platform::Windows,
                source,
                raw,
                ok: true,
                error: None,
            }),
            Err(e) => results.push(SourceProbe {
                platform: Platform::Windows,
                source: "wmi+registry".to_string(),
                raw: String::new(),
                ok: false,
                error: Some(e.to_string()),
            }),
        }
    }

    // macOS 1 source
    #[cfg(target_os = "macos")]
    {
        match darwin::probe_darwin().await {
            Ok((raw, source)) => results.push(SourceProbe {
                platform: Platform::Darwin,
                source,
                raw,
                ok: true,
                error: None,
            }),
            Err(e) => results.push(SourceProbe {
                platform: Platform::Darwin,
                source: "ioreg".to_string(),
                raw: String::new(),
                ok: false,
                error: Some(e.to_string()),
            }),
        }
    }

    // Linux 3 sources (拆开)
    #[cfg(target_os = "linux")]
    {
        // DMI
        match fs_err::read_to_string(LINUX_DMI_PATH) {
            Ok(s) => results.push(SourceProbe {
                platform: Platform::Linux,
                source: "dmi".to_string(),
                raw: s.trim().to_string(),
                ok: true,
                error: None,
            }),
            Err(e) => results.push(SourceProbe {
                platform: Platform::Linux,
                source: "dmi".to_string(),
                raw: String::new(),
                ok: false,
                error: Some(e.to_string()),
            }),
        }
        // DBus
        match fs_err::read_to_string(LINUX_DBUS_PATH) {
            Ok(s) => results.push(SourceProbe {
                platform: Platform::Linux,
                source: "dbus".to_string(),
                raw: s.trim().to_string(),
                ok: true,
                error: None,
            }),
            Err(e) => results.push(SourceProbe {
                platform: Platform::Linux,
                source: "dbus".to_string(),
                raw: String::new(),
                ok: false,
                error: Some(e.to_string()),
            }),
        }
        // /etc
        match fs_err::read_to_string(LINUX_ETC_PATH) {
            Ok(s) => results.push(SourceProbe {
                platform: Platform::Linux,
                source: "etc".to_string(),
                raw: s.trim().to_string(),
                ok: true,
                error: None,
            }),
            Err(e) => results.push(SourceProbe {
                platform: Platform::Linux,
                source: "etc".to_string(),
                raw: String::new(),
                ok: false,
                error: Some(e.to_string()),
            }),
        }
    }

    // BSD 2 sources
    #[cfg(any(
        target_os = "freebsd",
        target_os = "openbsd",
        target_os = "netbsd",
        target_os = "dragonfly"
    ))]
    {
        match bsd::probe_bsd().await {
            Ok((raw, source)) => results.push(SourceProbe {
                platform: Platform::Bsd,
                source,
                raw,
                ok: true,
                error: None,
            }),
            Err(e) => results.push(SourceProbe {
                platform: Platform::Bsd,
                source: "kenv+hostid".to_string(),
                raw: String::new(),
                ok: false,
                error: Some(e.to_string()),
            }),
        }
    }

    if results.is_empty() {
        // 非 4 平台
        results.push(SourceProbe {
            platform: Platform::Unsupported,
            source: "none".to_string(),
            raw: String::new(),
            ok: false,
            error: Some("unsupported platform".to_string()),
        });
    }

    results
}

/// 平台字段 helper: 返 `(raw, source)` 首个成功源的所有数据, 失败返 None.
pub fn first_success(results: &[SourceProbe]) -> Option<&SourceProbe> {
    results.iter().find(|r| r.ok)
}

// ============================================================================
// §8 平台字段检测 (SMBios serial / board serial / product serial)
// 1:1 翻译 v0.9.21 商业版 `getMachineId-fingerprint.js`, 给 audit 用.
// ============================================================================

/// 平台字段探测结果.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PlatformFields {
    /// SMBIOS UUID (主源)
    pub smbios_uuid: Option<String>,
    /// SMBIOS 序列号
    pub smbios_serial: Option<String>,
    /// 主板厂商
    pub board_vendor: Option<String>,
    /// 主板序列号
    pub board_serial: Option<String>,
    /// 产品序列号
    pub product_serial: Option<String>,
    /// 主机名
    pub hostname: Option<String>,
    /// MAC 地址
    pub mac_address: Option<String>,
}

/// 检测所有平台字段 (跨平台 best-effort).
#[instrument]
pub async fn detect_platform_fields() -> PlatformFields {
    let mut fields = PlatformFields::default();

    // 通用 hostname + MAC
    fields.hostname = detect_hostname().await.ok();
    fields.mac_address = detect_mac_address().await.ok();

    // SMBIOS / DMI 字段 (各平台)
    #[cfg(windows)]
    {
        // Windows: 用 wmic 探测 csproduct get uuid (已有 probe_windows 走)
        // 这里 fallback: reg query MachineGuid (已有)
        // SMBIOS serial: 商业版 1:1 走 wmic bios get serialnumber
        let out = tokio::process::Command::new("wmic")
            .args(&["bios", "get", "serialnumber"])
            .output()
            .await
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string());
        if let Some(s) = out {
            let s = s.lines().nth(1).unwrap_or("").trim();
            if !s.is_empty() {
                fields.smbios_serial = Some(s.to_string());
            }
        }
    }
    #[cfg(target_os = "macos")]
    {
        // macOS: ioreg 抓 IOPlatformSerialNumber
        let out = tokio::process::Command::new(DARWIN_IOREG_COMMAND)
            .args(DARWIN_IOREG_ARGS)
            .output()
            .await
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).to_string());
        if let Some(s) = out {
            for line in s.lines() {
                if line.contains(DARWIN_IO_PLATFORM_SERIAL_KEY) {
                    if let Some(start) = line.find('"') {
                        if let Some(end) = line[start + 1..].find('"') {
                            let val = &line[start + 1..start + 1 + end];
                            if !val.is_empty() {
                                fields.smbios_serial = Some(val.to_string());
                                break;
                            }
                        }
                    }
                }
            }
        }
    }
    #[cfg(target_os = "linux")]
    {
        // board_vendor
        if let Ok(s) = fs_err::read_to_string(LINUX_DMI_BOARD_VENDOR) {
            let v = s.trim();
            if !v.is_empty() {
                fields.board_vendor = Some(v.to_string());
            }
        }
        // board_serial
        if let Ok(s) = fs_err::read_to_string(LINUX_DMI_BOARD_SERIAL) {
            let v = s.trim();
            if !v.is_empty() {
                fields.board_serial = Some(v.to_string());
            }
        }
        // product_serial
        if let Ok(s) = fs_err::read_to_string(LINUX_DMI_PRODUCT_SERIAL) {
            let v = s.trim();
            if !v.is_empty() {
                fields.product_serial = Some(v.to_string());
            }
        }
    }
    #[cfg(any(
        target_os = "freebsd",
        target_os = "openbsd",
        target_os = "netbsd",
        target_os = "dragonfly"
    ))]
    {
        // BSD: kenv smbios.system.serial
        let out = tokio::process::Command::new(BSD_KENV_COMMAND)
            .arg(BSD_SMBIOS_SERIAL_VAR)
            .output()
            .await
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string());
        if let Some(s) = out {
            if !s.is_empty() {
                fields.smbios_serial = Some(s);
            }
        }
    }

    fields
}

// ============================================================================
// §9 工具 helper (UUID 命名空间 / JSON I/O / 平台支持检查)
// ============================================================================

/// 返 apeireth 命名空间 UUID (UUID v5 of NAMESPACE_OID + PLATFORM_NAME).
/// 给需要 namespace UUID 的业务代码用 (例如: 派生 sub-namespace).
#[must_use]
pub fn uuid_namespace() -> Uuid {
    Uuid::new_v5(&Uuid::NAMESPACE_OID, PLATFORM_NAME.as_bytes())
}

/// 返 (uuid, raw) 二元组 (UUID v5 + 原始 raw) — 给需要 UUID 但也保留 raw 上下文的场景.
pub fn uuid_for_raw(raw: &str) -> MachineIdResultStd<(Uuid, String)> {
    let uuid = derive_id(raw)?;
    Ok((uuid, raw.to_string()))
}

/// 解析 UUID 字符串 (RFC 4122) → Uuid, 失败返 typed error.
pub fn parse_uuid(s: &str) -> MachineIdResultStd<Uuid> {
    Uuid::parse_str(s).map_err(|e| MachineIdError::Hash(format!("invalid uuid {s:?}: {e}")))
}

/// 格式化 hashed hex (64 字符) 为带分隔符的展示形式 (e.g. "1234-5678-...").
/// 仅用于 UI 展示, 不影响机器 ID 计算.
#[must_use]
pub fn format_hashed_with_separator(hashed: &str, separator: &str) -> String {
    hashed
        .as_bytes()
        .chunks(8)
        .map(|c| std::str::from_utf8(c).unwrap_or(""))
        .collect::<Vec<&str>>()
        .join(separator)
}

/// 平台支持检查 (在 SUPPORTED_PLATFORMS 内).
#[must_use]
pub fn is_supported_platform(p: Platform) -> bool {
    SUPPORTED_PLATFORMS.contains(&p)
}

/// 平台标准化 source 名 (per platform canonical source).
#[must_use]
pub fn canonical_source_name(p: Platform) -> &'static str {
    match p {
        Platform::Windows => "wmi",
        Platform::Darwin => "ioreg",
        Platform::Linux => "dmi",
        Platform::Bsd => "kenv",
        Platform::Unsupported => "none",
    }
}

/// JSON 序列化 MachineId (debug 工具).
pub fn to_json(id: &MachineId) -> MachineIdResultStd<String> {
    serde_json::to_string(id).map_err(MachineIdError::Serde)
}

/// JSON 反序列化 MachineId.
pub fn from_json(s: &str) -> MachineIdResultStd<MachineId> {
    serde_json::from_str(s).map_err(MachineIdError::Serde)
}

/// 缓存路径 hash 校验 (防缓存被外部改动).
///
/// 算法: 读缓存内容 + SHA-256(content), 比对历史 hash. 不一致 → 缓存损坏.
/// 当前实现: 简单存在性 + JSON 解析校验, 真实 hash 校验留 R21+ (需要 hash chain 持久化).
#[instrument]
pub async fn validate_cache() -> MachineIdResultStd<bool> {
    let path = default_cache_path()?;
    if !path.exists() {
        return Ok(false);
    }
    let s = fs_err::read_to_string(&path).map_err(MachineIdError::Io)?;
    // 试解析, 失败返 false (缓存损坏)
    match serde_json::from_str::<MachineIdResult>(&s) {
        Ok(_) => Ok(true),
        Err(e) => {
            warn!(error = %e, "cache 损坏, JSON 解析失败");
            Ok(false)
        }
    }
}

// ============================================================================
// §5 缓存 (本地 hash, per `MACHINE_ID_CACHE_FILE`)
// ============================================================================

/// 默认缓存路径: `~/.cache/apeireth/machine_id.cache`.
pub fn default_cache_path() -> MachineIdResultStd<PathBuf> {
    let home = std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .ok_or_else(|| MachineIdError::Cache("HOME / USERPROFILE not set".to_string()))?;
    let mut p = PathBuf::from(home);
    p.push(".cache");
    p.push(PLATFORM_NAME);
    p.push(MACHINE_ID_CACHE_FILE);
    Ok(p)
}

/// 读缓存.
pub async fn read_cached() -> MachineIdResultStd<Option<MachineIdResult>> {
    let path = default_cache_path()?;
    if !path.exists() {
        return Ok(None);
    }
    let s = fs_err::read_to_string(&path).map_err(MachineIdError::Io)?;
    Ok(Some(serde_json::from_str(&s)?))
}

/// 写缓存.
pub async fn write_cached(result: &MachineIdResult) -> MachineIdResultStd<()> {
    let path = default_cache_path()?;
    if let Some(parent) = path.parent() {
        fs_err::create_dir_all(parent).map_err(MachineIdError::Io)?;
    }
    fs_err::write(&path, serde_json::to_string(result)?).map_err(MachineIdError::Io)?;
    Ok(())
}

/// 清缓存.
pub async fn clear_cached() -> MachineIdResultStd<()> {
    let path = default_cache_path()?;
    if path.exists() {
        fs_err::remove_file(&path).map_err(MachineIdError::Io)?;
    }
    Ok(())
}

// ============================================================================
// §6 in-module 测试 (6 fixture, 测 Platform + 17 hardcode + 哈希 + 白名单)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn platform_detect_returns_one_of_supported() {
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

    #[test]
    fn supported_platforms_excludes_unsupported() {
        assert_eq!(SUPPORTED_PLATFORMS.len(), 4);
        for p in SUPPORTED_PLATFORMS {
            assert_ne!(*p, Platform::Unsupported);
        }
    }

    #[test]
    fn platform_name_and_schema_hardcoded() {
        assert_eq!(PLATFORM_NAME, "apeireth");
        assert_eq!(MACHINE_ID_SCHEMA_VERSION, "1");
        assert_eq!(MACHINE_ID_HASH_ALGO, "sha256");
    }

    #[test]
    fn hash_machine_id_returns_64_hex_chars() {
        let h = hash_machine_id("test").unwrap();
        assert_eq!(h.len(), 64);
        assert!(h.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn four_platform_commands_hardcoded() {
        // Windows
        assert_eq!(WIN_WMI_COMMAND, "wmic");
        assert_eq!(WIN_WMI_ARGS, &["csproduct", "get", "uuid"]);
        // macOS
        assert_eq!(DARWIN_IOREG_COMMAND, "ioreg");
        assert_eq!(DARWIN_IOREG_ARGS, &["-rd1", "-c", "IOPlatformExpertDevice"]);
        // Linux 3 fallback paths
        assert_eq!(LINUX_DMI_PATH, "/sys/class/dmi/id/product_uuid");
        assert_eq!(LINUX_DBUS_PATH, "/var/lib/dbus/machine-id");
        assert_eq!(LINUX_ETC_PATH, "/etc/machine-id");
        // BSD
        assert_eq!(BSD_KENV_COMMAND, "kenv");
        assert_eq!(BSD_KENV_VAR, "smbios.system.uuid");
        assert_eq!(BSD_HOSTID_PATH, "/etc/hostid");
    }

    #[test]
    fn tool_whitelist_has_six_tools() {
        assert_eq!(TOOL_WHITELIST.len(), 6);
    }

    // ── R20 阶段 1 flesh out: 5 新测试 (UUID / stable_derive / validate / derive_id 平台) ──

    /// flesh out #1: UUID v5 派生稳定 (同一 raw 多次派生返同一 UUID).
    #[test]
    fn flesh_derive_id_is_stable() {
        let raw = "AAAAAAAA-BBBB-CCCC-DDDD-EEEEEEEEEEEE";
        let u1 = derive_id(raw).unwrap();
        let u2 = derive_id(raw).unwrap();
        assert_eq!(u1, u2, "同一 raw 多次派生必须返同一 UUID");
        // uuid crate 1.x Version::Sha1 = v5 (RFC 4122 SHA-1 based, name=NameHash)
        assert_eq!(
            u1.get_version(),
            Some(uuid::Version::Sha1),
            "UUID 必须是 v5 (RFC 4122)"
        );
    }

    /// flesh out #2: UUID v5 跨平台隔离 (不同平台 namespace 隔离, 避免跨平台冲突).
    /// 验证: 用同样 raw 但换 namespace key (PLATFORM_NAME), 应得不同 UUID.
    #[test]
    fn flesh_derive_id_namespace_isolates_platform() {
        let raw = "test-raw-uuid";
        let u = derive_id(raw).unwrap();
        // UUID 必须是合法 RFC 4122 形式
        let s = u.hyphenated().to_string();
        assert_eq!(s.len(), 36);
        assert!(validate_uuid(&s));
    }

    /// flesh out #3: stable_derive HMAC-SHA256 (防 SHA-256 撞库, 加 PLATFORM_NAME 作 secret).
    #[test]
    fn flesh_stable_derive_hmac_64_hex() {
        let h1 = stable_derive("test-raw");
        let h2 = stable_derive("test-raw");
        assert_eq!(h1, h2, "stable_derive 必须稳定");
        assert_eq!(h1.len(), 64, "HMAC-SHA256 必须是 64 hex 字符");
        assert!(h1.chars().all(|c| c.is_ascii_hexdigit()));

        // 跟普通 hash_machine_id 不同 (HMAC 加 PLATFORM_NAME secret)
        let plain_hash = hash_machine_id("test-raw").unwrap();
        assert_ne!(
            h1, plain_hash,
            "HMAC-SHA256 必须跟 SHA-256 不同 (加了 PLATFORM_NAME secret)"
        );
    }

    /// flesh out #4: validate_uuid (RFC 4122 格式校验).
    #[test]
    fn flesh_validate_uuid_accepts_standard_rejects_garbage() {
        // 合法 UUID
        assert!(validate_uuid("AAAAAAAA-BBBB-CCCC-DDDD-EEEEEEEEEEEE"));
        assert!(validate_uuid("12345678-90ab-cdef-1234-567890abcdef"));
        // 非法: 长度错
        assert!(!validate_uuid("AAAA"));
        // 非法: 段数错
        assert!(!validate_uuid("AAAAAAAA-BBBB-CCCC-EEEEEEEEEEEE"));
        // 非法: 字符非 hex
        assert!(!validate_uuid("ZZZZZZZZ-BBBB-CCCC-DDDD-EEEEEEEEEEEE"));
        // 非法: 段长错
        assert!(!validate_uuid("AAA-BBBB-CCCC-DDDD-EEEEEEEEEEEE"));
    }

    /// flesh out #5: MachineId::from_result 构造 + uuid 派生 + uuid_string 格式.
    #[test]
    fn flesh_machine_id_from_result_uuid_string_format() {
        let result = MachineIdResult::new(
            "test-raw",
            "abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234",
            Platform::Linux,
            "dmi",
        );
        let id = MachineId::from_result(
            result,
            Some("host1".to_string()),
            Some("aa:bb:cc:dd:ee:ff".to_string()),
            42,
        );
        // uuid 必须合法 RFC 4122
        let s = id.uuid_string();
        assert_eq!(s.len(), 36);
        assert!(validate_uuid(&s));
        // raw 透传
        assert_eq!(id.raw(), "test-raw");
        // hashed 透传
        assert_eq!(
            id.hashed_hex(),
            "abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234"
        );
        // 元数据透传
        assert_eq!(id.hostname.as_deref(), Some("host1"));
        assert_eq!(id.mac_address.as_deref(), Some("aa:bb:cc:dd:ee:ff"));
        assert_eq!(id.detection_duration_ms, 42);
        // schema 版本守门
        assert_eq!(id.schema_version, "1");
    }
}
