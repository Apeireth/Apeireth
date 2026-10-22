//! 远程访问面 (W2-②, F 轴 P3): 绑定地址旋钮 + 非回环绑定的 fail-closed 闸 +
//! TLS 面真接线 (安全加固批 B5)。
//!
//! 三条承诺:
//!
//! 1. **默认仅本机**: 绑定地址由 `APEIRETH_LISTEN` (缺省 `127.0.0.1`) 或
//!    `--bind` 显式给出; 回环绑定永远不需要令牌;
//! 2. **非回环绑定 fail-closed**: 没有任何令牌可校验 (env 单令牌面与客户端
//!    令牌表两面皆空) 时**拒绝启动** —— 不裸奔、不警告后照跑; 逃生阀
//!    (`APEIRETH_GATEWAY_TOKEN=none`) 是操作者的显式自担声明, 放行但**留审计**;
//! 3. **TLS 面真接线** (B5 起接线, 原 "接口桩" 已退役): `APEIRETH_TLS_CERT` /
//!    `APEIRETH_TLS_KEY` 两旋钮指向 PEM 文件时**真加载** (证书链 + 私钥, 含配对
//!    校验), 服务面走 TLS listener (能力口径: PRODUCTION WIRED —— 接线真、默认
//!    不启用; 不设旋钮 = 明文回环缺省不变)。材料坏 (文件缺失 / PEM 损坏 /
//!    证书私钥不配对 / 半套配置) = **拒绝启动**并如实报错, 绝不静默降级明文
//!    (fail-closed)。"设了旋钮只警告、流量照旧明文"是旧接口桩语义, 已随接线
//!    退役 (见测试模块的演进说明)。
//!
//! 0 装口径: 三个状态 (未配置 / 已启用 / 加载失败) 全部是实测行为; live TLS
//! 握手 E2E 见 `tests/tls_serve.rs`。

use thiserror::Error;

use rustls_pki_types::pem::PemObject;
use rustls_pki_types::{CertificateDer, PrivateKeyDer};

use crate::canonical_entry::GatewayTokenGate;
use crate::client_tokens::ClientTokenTable;

/// 绑定地址旋钮的环境变量名 (host, 可含方括号 IPv6 字面量)。
pub const LISTEN_ENV: &str = "APEIRETH_LISTEN";

/// 未配置时的绑定地址: 仅本机回环 (fail-closed 默认)。
pub const DEFAULT_BIND: &str = "127.0.0.1";

/// TLS 证书链旋钮 (真接线): 指向 PEM 文件 (可含多张证书, 叶证书在前);
/// 与 [`TLS_KEY_ENV`] 成对设置 —— 半套配置拒启动。
pub const TLS_CERT_ENV: &str = "APEIRETH_TLS_CERT";

/// TLS 私钥旋钮 (真接线): 指向 PEM 文件 (PKCS#8 / PKCS#1 / SEC1 皆收);
/// 与 [`TLS_CERT_ENV`] 成对设置 —— 半套配置拒启动。
pub const TLS_KEY_ENV: &str = "APEIRETH_TLS_KEY";

/// 绑定地址解析: `--bind` 显式旗标 > `APEIRETH_LISTEN` > 回环缺省。
///
/// 空白旗标值 = 显式错误 (不静默回退); 空白 env 值按未配置处理。
pub fn resolve_listen_bind_from(
    flag: Option<&str>,
    env_value: Option<&str>,
) -> Result<String, RemoteBindError> {
    if let Some(flag_value) = flag {
        let trimmed = flag_value.trim();
        if trimmed.is_empty() {
            return Err(RemoteBindError::EmptyBind);
        }
        return Ok(trimmed.to_string());
    }
    if let Some(env_bind) = env_value.map(str::trim) {
        if !env_bind.is_empty() {
            return Ok(env_bind.to_string());
        }
    }
    Ok(DEFAULT_BIND.to_string())
}

/// [`resolve_listen_bind_from`] 的环境读取包装 (组合根用)。
pub fn resolve_listen_bind(flag: Option<&str>) -> Result<String, RemoteBindError> {
    resolve_listen_bind_from(flag, std::env::var(LISTEN_ENV).ok().as_deref())
}

/// 绑定地址的作用域: 回环 = 只有本机可达; 非回环 = 网络可达 (远程面)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BindScope {
    /// 只有本机可连 (127.0.0.0/8 / `::1` / `localhost`)。
    Loopback,
    /// 至少一个非本机接口可达 (含 `0.0.0.0` / `::` / LAN 地址 / 具名主机)。
    NonLoopback,
}

/// 判定绑定作用域。**认不出一律按非回环处置** (fail-closed: 宁可多要令牌,
/// 不可把远程绑定当本机放行)。
pub fn bind_scope(bind: &str) -> BindScope {
    if is_loopback_bind(bind) {
        BindScope::Loopback
    } else {
        BindScope::NonLoopback
    }
}

/// 回环判定: host 段命中本机回环字面量才认 (剥方括号 / 剥尾随 `:端口`)。
pub fn is_loopback_bind(bind: &str) -> bool {
    let host = bind_host(bind);
    if host.is_empty() {
        return false;
    }
    let lower = host.to_ascii_lowercase();
    if lower == "localhost" {
        return true;
    }
    if lower == "::1" || lower == "0:0:0:0:0:0:0:1" {
        return true;
    }
    // IPv4 回环 = 127.0.0.0/8。
    lower.starts_with("127.")
        && lower.split('.').count() == 4
        && lower
            .split('.')
            .all(|octet| !octet.is_empty() && octet.chars().all(|c| c.is_ascii_digit()))
}

/// host 段提取: 剥 IPv6 方括号 (端口在括号外), 剥尾随 `:<纯数字端口>`
/// (容忍 `host:port` 输入; 裸 IPv6 字面量含冒号, 不做端口剥离)。
fn bind_host(bind: &str) -> &str {
    let trimmed = bind.trim();
    if let Some(rest) = trimmed.strip_prefix('[') {
        return rest.split(']').next().unwrap_or("");
    }
    match trimmed.rsplit_once(':') {
        Some((host, port))
            if !host.is_empty()
                && !host.contains(':')
                && !port.is_empty()
                && port.chars().all(|c| c.is_ascii_digit()) =>
        {
            host
        }
        _ => trimmed,
    }
}

/// 非回环绑定闸放行后的清场凭据: 需要落审计的事实行。
#[derive(Debug, Clone)]
pub struct RemoteBindClearance {
    /// 绑定作用域 (回环 = 本机面; 非回环 = 远程面)。
    pub scope: BindScope,
    /// 组合根应写入审计面的事实 (事件名, 明细)。
    pub audit_facts: Vec<(String, Option<String>)>,
}

/// 远程绑定闸的拒绝与输入缺陷。
#[derive(Debug, Error, PartialEq, Eq)]
pub enum RemoteBindError {
    /// 绑定地址显式给了空白值。
    #[error("bind address must not be empty")]
    EmptyBind,
    /// 非回环绑定但令牌族两面皆空: fail-closed 拒绝启动。
    #[error(
        "refusing to bind non-loopback address {bind} without an enforceable token: set \
         {token_env} (or configure {table_env}) so every API call must authenticate, or bind a \
         loopback address; declaring {token_env}={sentinel} is the explicit unprotected escape \
         valve (audit-traced)",
        token_env = GatewayTokenGate::ENV,
        table_env = crate::client_tokens::CLIENT_TOKENS_ENV,
        sentinel = GatewayTokenGate::UNPROTECTED_SENTINEL,
    )]
    NonLoopbackWithoutToken {
        /// 被拒的绑定地址。
        bind: String,
    },
}

/// 非回环绑定强制令牌闸 (fail-closed 即帧):
///
/// * 回环绑定 —— 恒放行 (本机面, 桌面/TUI 零回归);
/// * 非回环 + 令牌族在场 (env 单令牌或客户端令牌表任一) —— 放行, 留审计事实;
/// * 非回环 + 逃生阀 (`APEIRETH_GATEWAY_TOKEN=none`) —— 放行, **明示可裸奔,
///   留审计**;
/// * 非回环 + 令牌族两面皆空 —— **拒绝启动**。
///
/// 逃生阀与令牌表互斥的配置矛盾由
/// [`crate::client_tokens::validate_gate_table_pair`] 另闸拒开, 本函数假定
/// 配置已过该闸。
pub fn enforce_remote_bind(
    bind: &str,
    gate: &GatewayTokenGate,
    clients: &ClientTokenTable,
) -> Result<RemoteBindClearance, RemoteBindError> {
    let scope = bind_scope(bind);
    if scope == BindScope::Loopback {
        return Ok(RemoteBindClearance {
            scope,
            audit_facts: Vec::new(),
        });
    }
    let bind = bind.trim();
    match gate {
        // 逃生阀: 操作者明示不设防 —— 可裸奔但每次装配留痕。
        GatewayTokenGate::UnprotectedDeclared => Ok(RemoteBindClearance {
            scope,
            audit_facts: vec![(
                "gateway.remote_bind".to_string(),
                Some(format!(
                    "bind={bind} mode=declared_none detail=operator explicitly declared an \
                     unprotected gateway on a non-loopback bind (escape valve)"
                )),
            )],
        }),
        // 令牌族在场 (env 单令牌 / 客户端令牌表任一) = 全 API 面强制校验。
        GatewayTokenGate::Enforced(_) => Ok(RemoteBindClearance {
            scope,
            audit_facts: vec![(
                "gateway.remote_bind".to_string(),
                Some(format!("bind={bind} mode=enforced")),
            )],
        }),
        GatewayTokenGate::UnprotectedDefault if !clients.is_empty() => Ok(RemoteBindClearance {
            scope,
            audit_facts: vec![(
                "gateway.remote_bind".to_string(),
                Some(format!(
                    "bind={bind} mode=enforced detail=client token table ({} client token(s))",
                    clients.len()
                )),
            )],
        }),
        // 令牌族两面皆空: fail-closed 拒绝启动。
        GatewayTokenGate::UnprotectedDefault => Err(RemoteBindError::NonLoopbackWithoutToken {
            bind: bind.to_string(),
        }),
    }
}

/// TLS 材料加载错误 (fail-closed: 任何一条都意味着**拒绝启动**, 不降级明文)。
#[derive(Debug, Error, PartialEq, Eq)]
pub enum TlsConfigError {
    /// 只设了一半旋钮: 半套配置是配置矛盾, 拒启动。
    #[error(
        "TLS knobs are half-configured: set both {cert_env} (certificate chain) and {key_env} \
         (private key) or neither — the gateway refuses to start rather than silently serving \
         plaintext",
        cert_env = TLS_CERT_ENV,
        key_env = TLS_KEY_ENV,
    )]
    IncompleteConfig {
        /// 证书旋钮是否在场。
        cert_env_set: bool,
        /// 私钥旋钮是否在场。
        key_env_set: bool,
    },
    /// 证书链文件缺失或不可读。
    #[error("TLS certificate chain file {path} could not be read: {detail}")]
    CertFileUnreadable {
        /// 证书链文件路径。
        path: String,
        /// 具体病因。
        detail: String,
    },
    /// 证书链 PEM 不可用 (解析失败 / 无 CERTIFICATE 段)。
    #[error("TLS certificate chain file {path} is not a usable PEM certificate chain: {detail}")]
    CertChainInvalid {
        /// 证书链文件路径。
        path: String,
        /// 具体病因。
        detail: String,
    },
    /// 私钥文件缺失或不可读。
    #[error("TLS private key file {path} could not be read: {detail}")]
    KeyFileUnreadable {
        /// 私钥文件路径。
        path: String,
        /// 具体病因。
        detail: String,
    },
    /// 私钥 PEM 不可用。
    #[error("TLS private key file {path} is not a usable PEM private key: {detail}")]
    KeyInvalid {
        /// 私钥文件路径。
        path: String,
        /// 具体病因。
        detail: String,
    },
    /// 证书与私钥都读出来了但拒绝配对 (密钥不匹配等)。
    #[error("TLS certificate chain and private key refuse to pair: {detail}")]
    PairingRefused {
        /// 具体病因。
        detail: String,
    },
}

/// 已加载的 TLS 材料: 证书链 + 私钥已过 PEM 解析与配对校验, 服务面可直接启用。
///
/// `Debug` 只报路径与链长 —— **私钥字节永不进诊断输出**。
#[derive(Clone)]
pub struct TlsMaterial {
    cert_path: String,
    key_path: String,
    chain_len: usize,
    config: std::sync::Arc<rustls::ServerConfig>,
}

impl std::fmt::Debug for TlsMaterial {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TlsMaterial")
            .field("cert_path", &self.cert_path)
            .field("key_path", &self.key_path)
            .field("chain_len", &self.chain_len)
            .finish_non_exhaustive()
    }
}

impl TlsMaterial {
    /// 证书链文件路径 (诊断/状态面用)。
    pub fn cert_path(&self) -> &str {
        &self.cert_path
    }

    /// 私钥文件路径 (诊断/状态面用)。
    pub fn key_path(&self) -> &str {
        &self.key_path
    }

    /// 证书链张数 (≥ 1)。
    pub fn chain_len(&self) -> usize {
        self.chain_len
    }

    /// 服务面用的 TLS 服务端配置 (证书链 + 私钥已配对; 不要求客户端证书)。
    pub fn server_config(&self) -> std::sync::Arc<rustls::ServerConfig> {
        std::sync::Arc::clone(&self.config)
    }
}

/// 真加载 TLS 材料 (fail-closed): 证书链 + 私钥都解析成功且**配对成功**才算数;
/// 任何一步失败都以 [`TlsConfigError`] 如实帧化 —— 调用方据此**拒绝启动**。
///
/// 加载面覆盖: 文件缺失/不可读、证书链 PEM 损坏或无段、私钥 PEM 损坏、
/// 证书私钥不配对 (配对校验在加载时做, 不留到握手时才爆)。
pub fn load_tls_material(cert_path: &str, key_path: &str) -> Result<TlsMaterial, TlsConfigError> {
    let cert_bytes =
        fs_err::read(cert_path).map_err(|error| TlsConfigError::CertFileUnreadable {
            path: cert_path.to_string(),
            detail: error.to_string(),
        })?;
    let key_bytes = fs_err::read(key_path).map_err(|error| TlsConfigError::KeyFileUnreadable {
        path: key_path.to_string(),
        detail: error.to_string(),
    })?;
    let certs: Vec<CertificateDer<'static>> = CertificateDer::pem_slice_iter(&cert_bytes)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| TlsConfigError::CertChainInvalid {
            path: cert_path.to_string(),
            detail: error.to_string(),
        })?;
    if certs.is_empty() {
        return Err(TlsConfigError::CertChainInvalid {
            path: cert_path.to_string(),
            detail: "no CERTIFICATE section found".to_string(),
        });
    }
    let chain_len = certs.len();
    let key =
        PrivateKeyDer::from_pem_slice(&key_bytes).map_err(|error| TlsConfigError::KeyInvalid {
            path: key_path.to_string(),
            detail: error.to_string(),
        })?;
    // 配对校验是加载的一部分: 密钥不匹配/密钥不可用在这里就拒, 不留到握手时爆。
    let config = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .map_err(|error| TlsConfigError::PairingRefused {
            detail: error.to_string(),
        })?;
    Ok(TlsMaterial {
        cert_path: cert_path.to_string(),
        key_path: key_path.to_string(),
        chain_len,
        config: std::sync::Arc::new(config),
    })
}

/// TLS 面解析 (纯形态, 供测试钉行为): 两旋钮皆空白/未设 = 未配置 (明文语义不变);
/// 只设一半 = 半套配置拒启动; 双设 = 真加载, 材料坏拒启动 (fail-closed)。
pub fn resolve_tls_material_from(
    cert_env: Option<&str>,
    key_env: Option<&str>,
) -> Result<Option<TlsMaterial>, TlsConfigError> {
    match (knob_path(cert_env), knob_path(key_env)) {
        (None, None) => Ok(None),
        (Some(cert), Some(key)) => load_tls_material(cert, key).map(Some),
        (cert, key) => Err(TlsConfigError::IncompleteConfig {
            cert_env_set: cert.is_some(),
            key_env_set: key.is_some(),
        }),
    }
}

/// [`resolve_tls_material_from`] 的环境读取包装 (组合根/服务面用)。
pub fn resolve_tls_material() -> Result<Option<TlsMaterial>, TlsConfigError> {
    resolve_tls_material_from(
        std::env::var(TLS_CERT_ENV).ok().as_deref(),
        std::env::var(TLS_KEY_ENV).ok().as_deref(),
    )
}

/// 旋钮取值: 空白按未设置处理 (与绑定旋钮同一口径)。
fn knob_path(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

/// TLS 面状态: 真实三态 (B5 起, "unwired placeholder" 桩语义已退役)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TlsSurface {
    /// 未配置: 两个旋钮都未设 —— 明文回环缺省 (语义不变, 不喊破)。
    Unconfigured,
    /// 已启用: 证书链 + 私钥加载成功, 服务面走 TLS listener。
    Enabled {
        /// 证书链文件路径。
        cert_path: String,
        /// 私钥文件路径。
        key_path: String,
        /// 证书链张数。
        chain_len: usize,
    },
    /// 加载失败: 旋钮在场但材料不可用 —— 启动闸据此**拒绝启动** (fail-closed)。
    Failed {
        /// 具体病因 (即帧文本)。
        detail: String,
    },
}

impl TlsSurface {
    /// TLS 面是否真启用 (供组合根定 URL 协议名/审计事件名)。
    pub fn is_enabled(&self) -> bool {
        matches!(self, Self::Enabled { .. })
    }

    /// 操作者提示行: 已启用 = 接线确认; 加载失败 = fail-closed 警示; 未配置 =
    /// 无 (缺省明文回环姿态不喊破)。
    pub fn notice(&self) -> Option<String> {
        match self {
            Self::Unconfigured => None,
            Self::Enabled {
                cert_path,
                key_path,
                chain_len,
            } => Some(format!(
                "TLS enabled: the gateway serves TLS with the {cert_env} certificate chain \
                 ({chain_len} certificate(s) from {cert_path}) and the {key_env} private key \
                 (from {key_path})",
                cert_env = TLS_CERT_ENV,
                key_env = TLS_KEY_ENV,
            )),
            Self::Failed { detail } => Some(format!(
                "TLS material failed to load ({detail}) — the gateway refuses to start instead of \
                 silently downgrading to plaintext"
            )),
        }
    }

    /// 启动拒绝文本: 仅加载失败时在场 (fail-closed 拒启动, 不静默降级明文)。
    pub fn refusal(&self) -> Option<String> {
        match self {
            Self::Failed { detail } => Some(format!(
                "refusing to start: TLS material is configured but unusable ({detail}); fix the \
                 {cert_env}/{key_env} material or unset both knobs — the gateway never silently \
                 falls back to plaintext",
                cert_env = TLS_CERT_ENV,
                key_env = TLS_KEY_ENV,
            )),
            Self::Unconfigured | Self::Enabled { .. } => None,
        }
    }
}

/// TLS 面状态 (环境读取包装)。
pub fn tls_surface_status() -> TlsSurface {
    tls_surface_status_from(
        std::env::var(TLS_CERT_ENV).ok().as_deref(),
        std::env::var(TLS_KEY_ENV).ok().as_deref(),
    )
}

/// TLS 面状态 (显式旋钮值形态): 真加载材料后如实报 —— 已启用/未配置/加载失败
/// 三态都是实测事实, 不是占位。加载失败时启动闸同时拒启动
/// (见 [`TlsSurface::refusal`]), 状态面只是把同一个事实说出来。
pub fn tls_surface_status_from(cert_env: Option<&str>, key_env: Option<&str>) -> TlsSurface {
    match resolve_tls_material_from(cert_env, key_env) {
        Ok(None) => TlsSurface::Unconfigured,
        Ok(Some(material)) => TlsSurface::Enabled {
            cert_path: material.cert_path().to_string(),
            key_path: material.key_path().to_string(),
            chain_len: material.chain_len(),
        },
        Err(error) => TlsSurface::Failed {
            detail: error.to_string(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table_with_one() -> ClientTokenTable {
        ClientTokenTable::from_config(crate::client_tokens::ClientTokenConfig::new(vec![
            crate::client_tokens::ClientTokenSpec {
                name: "tui".to_string(),
                token: "tok-a".to_string(),
                user_id: None,
            },
        ]))
        .unwrap()
    }

    #[test]
    fn listen_bind_precedence_is_flag_then_env_then_loopback_default() {
        assert_eq!(
            resolve_listen_bind_from(Some("  0.0.0.0  "), Some("10.0.0.5")).unwrap(),
            "0.0.0.0",
            "显式旗标优先"
        );
        assert_eq!(
            resolve_listen_bind_from(None, Some(" 10.0.0.5 ")).unwrap(),
            "10.0.0.5",
            "旗标缺位时 env 上场"
        );
        assert_eq!(
            resolve_listen_bind_from(None, Some("   ")).unwrap(),
            DEFAULT_BIND,
            "空白 env 按未配置处理"
        );
        assert_eq!(
            resolve_listen_bind_from(None, None).unwrap(),
            "127.0.0.1",
            "缺省仅本机"
        );
        assert_eq!(
            resolve_listen_bind_from(Some("  "), None),
            Err(RemoteBindError::EmptyBind),
            "空白旗标值显式报错, 不静默回退"
        );
    }

    #[test]
    fn loopback_literals_are_the_only_thing_treated_as_local() {
        for bind in [
            "127.0.0.1",
            "127.0.0.1:8080",
            "127.5.6.7",
            "localhost",
            "LOCALHOST",
            "[::1]",
            "::1",
            "0:0:0:0:0:0:0:1",
        ] {
            assert_eq!(bind_scope(bind), BindScope::Loopback, "{bind} 应判回环");
        }
        for bind in [
            "0.0.0.0",
            "::",
            "192.168.1.10",
            "10.0.0.5",
            "169.254.1.1",
            "example.internal",
            "127.0.0.1.evil.example",
            "localhost.evil.example",
            "",
        ] {
            assert_eq!(
                bind_scope(bind),
                BindScope::NonLoopback,
                "{bind} 应判非回环 (fail-closed)"
            );
        }
    }

    #[test]
    fn non_loopback_without_any_token_refuses_to_start() {
        let err = enforce_remote_bind(
            "0.0.0.0",
            &GatewayTokenGate::UnprotectedDefault,
            &ClientTokenTable::default(),
        )
        .expect_err("无令牌的非回环绑定必须拒绝启动");
        assert_eq!(
            err,
            RemoteBindError::NonLoopbackWithoutToken {
                bind: "0.0.0.0".to_string()
            }
        );
        let text = err.to_string();
        assert!(text.contains(GatewayTokenGate::ENV), "{text}");
        assert!(
            text.contains("refusing to bind"),
            "fail-closed 即帧: {text}"
        );
    }

    #[test]
    fn non_loopback_with_tokens_is_cleared_and_audit_traced() {
        // env 单令牌面在场。
        let cleared = enforce_remote_bind(
            "0.0.0.0",
            &GatewayTokenGate::Enforced("tok-e".to_string()),
            &ClientTokenTable::default(),
        )
        .expect("带令牌的非回环绑定放行");
        assert_eq!(cleared.scope, BindScope::NonLoopback);
        let fact = cleared.audit_facts[0].clone();
        assert_eq!(fact.0, "gateway.remote_bind");
        assert!(
            fact.1.as_deref().unwrap_or("").contains("mode=enforced"),
            "{:?}",
            fact.1
        );

        // 仅客户端令牌表在场 (env 单令牌未设) 也算令牌族在场。
        let cleared = enforce_remote_bind(
            "0.0.0.0",
            &GatewayTokenGate::UnprotectedDefault,
            &table_with_one(),
        )
        .expect("令牌表在场即强制, 非回环绑定放行");
        let detail = cleared.audit_facts[0].1.clone().unwrap_or_default();
        assert!(detail.contains("mode=enforced"), "{detail}");
        assert!(detail.contains("client token table"), "{detail}");
    }

    #[test]
    fn non_loopback_escape_valve_runs_but_leaves_the_audit_trace() {
        let cleared = enforce_remote_bind(
            "0.0.0.0",
            &GatewayTokenGate::UnprotectedDeclared,
            &ClientTokenTable::default(),
        )
        .expect("逃生阀放行 (操作者明示自担)");
        let (event, detail) = cleared.audit_facts[0].clone();
        assert_eq!(event, "gateway.remote_bind");
        let detail = detail.unwrap_or_default();
        assert!(detail.contains("mode=declared_none"), "{detail}");
        assert!(detail.contains("escape valve"), "{detail}");
    }

    #[test]
    fn loopback_binds_never_need_tokens() {
        for gate in [
            GatewayTokenGate::UnprotectedDefault,
            GatewayTokenGate::UnprotectedDeclared,
            GatewayTokenGate::Enforced("tok-e".to_string()),
        ] {
            let cleared = enforce_remote_bind("127.0.0.1", &gate, &ClientTokenTable::default())
                .expect("回环绑定恒放行");
            assert_eq!(cleared.scope, BindScope::Loopback);
            assert!(
                cleared.audit_facts.is_empty(),
                "回环是缺省安全姿态, 不额外留痕"
            );
        }
    }

    // ── TLS 面 (B5 真接线) ──────────────────────────────────────────────────
    //
    // 断言演进理由 (纪律 §一.4): 原测试 `tls_placeholder_knobs_are_reported_as_unwired`
    // 钉的是"旋钮只警告、不接线"的接口桩语义。B5 真接线后桩语义整体退役 ——
    // 设了旋钮不再是警告面, 而是真加载/真拒绝 —— 故该断言族按 fail-closed
    // 语义重写为下列四测; 桩语义已不存在于系统中, 不再保留旧断言。

    fn fixture(name: &str) -> String {
        format!("{}/tests/fixtures/tls/{name}", env!("CARGO_MANIFEST_DIR"))
    }

    fn garbage_pem(label: &str) -> String {
        let path = std::env::temp_dir().join(format!(
            "apeireth-b5-tls-{label}-{}.pem",
            std::process::id()
        ));
        fs_err::write(
            &path,
            "-----BEGIN CERTIFICATE-----\nnot base64!!\n-----END CERTIFICATE-----\n",
        )
        .expect("临时坏材料写盘");
        path.to_string_lossy().into_owned()
    }

    #[test]
    fn without_tls_knobs_the_plaintext_loopback_default_is_unchanged() {
        let surface = tls_surface_status_from(None, None);
        assert_eq!(surface, TlsSurface::Unconfigured, "未设旋钮 = 未配置");
        assert!(!surface.is_enabled());
        assert_eq!(surface.notice(), None, "缺省明文回环姿态不喊破");
        assert_eq!(surface.refusal(), None, "未配置不是拒绝面");
        assert_eq!(
            tls_surface_status_from(Some("   "), Some("")),
            TlsSurface::Unconfigured,
            "空白按未设置处理"
        );
        assert!(
            resolve_tls_material_from(None, None)
                .expect("未配置不是错误")
                .is_none(),
            "未设旋钮 = 不加载任何材料"
        );
    }

    #[test]
    fn valid_certificate_chain_and_key_enable_the_tls_surface_for_real() {
        let cert = fixture("server-chain.pem");
        let key = fixture("server-key.pem");
        let material = load_tls_material(&cert, &key).expect("合法材料必须真加载成功");
        assert_eq!(material.chain_len(), 2, "证书链两段 (叶 + 根) 都要吃进");
        assert_eq!(material.cert_path(), cert);
        assert_eq!(material.key_path(), key);

        let surface = tls_surface_status_from(Some(&cert), Some(&key));
        assert!(surface.is_enabled(), "{surface:?}");
        assert_eq!(surface.refusal(), None, "已启用没有拒绝文本");
        let notice = surface.notice().expect("已启用要给接线确认行");
        assert!(
            notice.contains(TLS_CERT_ENV) && notice.contains(TLS_KEY_ENV),
            "{notice}"
        );
        assert!(notice.contains("TLS enabled"), "{notice}");

        // 状态面与材料面同源: 同样材料经旋钮解析也得 Some。
        assert!(
            resolve_tls_material_from(Some(&cert), Some(&key))
                .expect("合法材料解析不得报错")
                .is_some(),
            "双旋钮 + 合法材料 = 材料在场"
        );
    }

    #[test]
    fn missing_or_corrupt_tls_material_refuses_to_start() {
        // 文件缺失 → 拒加载 (不是警告、不是降级)。
        let missing = fixture("no-such-file.pem");
        let err = load_tls_material(&missing, &missing).expect_err("缺文件必须拒加载");
        assert!(
            matches!(err, TlsConfigError::CertFileUnreadable { .. }),
            "{err:?}"
        );

        // 状态面如实 = Failed, refusal 在场 (启动闸据此拒启动)。
        let surface = tls_surface_status_from(Some(&missing), Some(&missing));
        assert!(matches!(surface, TlsSurface::Failed { .. }), "{surface:?}");
        assert!(!surface.is_enabled());
        let refusal = surface.refusal().expect("加载失败必须给拒启动文本");
        assert!(refusal.contains("refusing to start"), "{refusal}");
        assert!(refusal.contains("never silently falls back"), "{refusal}");
        let notice = surface.notice().expect("加载失败必须喊破");
        assert!(notice.contains("refuses to start"), "{notice}");

        // PEM 损坏 → 拒加载 (证书面/私钥面各自帧化)。
        let bad_cert = garbage_pem("cert");
        let err =
            load_tls_material(&bad_cert, &fixture("server-key.pem")).expect_err("坏证书必须拒加载");
        assert!(
            matches!(err, TlsConfigError::CertChainInvalid { .. }),
            "{err:?}"
        );
        let bad_key = garbage_pem("key");
        let err = load_tls_material(&fixture("server-chain.pem"), &bad_key)
            .expect_err("坏私钥必须拒加载");
        assert!(matches!(err, TlsConfigError::KeyInvalid { .. }), "{err:?}");

        // 半套配置 → 配置矛盾拒启动 (双向都要钉)。
        let cert = fixture("server-chain.pem");
        let key = fixture("server-key.pem");
        let err = resolve_tls_material_from(Some(&cert), None).expect_err("只给证书拒启动");
        assert!(
            matches!(
                err,
                TlsConfigError::IncompleteConfig {
                    cert_env_set: true,
                    key_env_set: false
                }
            ),
            "{err:?}"
        );
        let err = resolve_tls_material_from(None, Some(&key)).expect_err("只给私钥拒启动");
        assert!(
            matches!(
                err,
                TlsConfigError::IncompleteConfig {
                    cert_env_set: false,
                    key_env_set: true
                }
            ),
            "{err:?}"
        );
        assert!(
            matches!(
                tls_surface_status_from(Some(&cert), None),
                TlsSurface::Failed { .. }
            ),
            "半套配置在状态面也是加载失败"
        );
    }

    #[test]
    fn mismatched_key_and_certificate_refuse_to_pair() {
        let err = load_tls_material(&fixture("server-chain.pem"), &fixture("mismatch-key.pem"))
            .expect_err("证书与私钥不配对必须拒加载");
        assert!(
            matches!(err, TlsConfigError::PairingRefused { .. }),
            "{err:?}"
        );
    }
}
