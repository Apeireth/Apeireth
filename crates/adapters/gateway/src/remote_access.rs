//! 远程访问面 (W2-②, F 轴 P3): 绑定地址旋钮 + 非回环绑定的 fail-closed 闸 +
//! TLS 面接口桩。
//!
//! 三条承诺:
//!
//! 1. **默认仅本机**: 绑定地址由 `APEIRETH_LISTEN` (缺省 `127.0.0.1`) 或
//!    `--bind` 显式给出; 回环绑定永远不需要令牌;
//! 2. **非回环绑定 fail-closed**: 没有任何令牌可校验 (env 单令牌面与客户端
//!    令牌表两面皆空) 时**拒绝启动** —— 不裸奔、不警告后照跑; 逃生阀
//!    (`APEIRETH_GATEWAY_TOKEN=none`) 是操作者的显式自担声明, 放行但**留审计**;
//! 3. **TLS 面是接口桩** (显式未接线): `APEIRETH_TLS_CERT` / `APEIRETH_TLS_KEY`
//!    两个占位旋钮只被识别与报告, 不接线、不硬造 —— 设了它们**不会**得到 TLS,
//!    只会得到一句"仍未接线, 流量是明文"的显式警告 (假旋钮纪律: 旋钮要么有权
//!    要么喊破, 不许静默装死)。

use thiserror::Error;

use crate::canonical_entry::GatewayTokenGate;
use crate::client_tokens::ClientTokenTable;

/// 绑定地址旋钮的环境变量名 (host, 可含方括号 IPv6 字面量)。
pub const LISTEN_ENV: &str = "APEIRETH_LISTEN";

/// 未配置时的绑定地址: 仅本机回环 (fail-closed 默认)。
pub const DEFAULT_BIND: &str = "127.0.0.1";

/// TLS 证书链占位旋钮 (**接口桩, 未接线**): 设值只触发显式警告, 不启用 TLS。
pub const TLS_CERT_ENV: &str = "APEIRETH_TLS_CERT";

/// TLS 私钥占位旋钮 (**接口桩, 未接线**): 设值只触发显式警告, 不启用 TLS。
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

/// TLS 面状态: 显式未接线的接口桩 (不硬造 TLS 实现)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TlsSurface {
    /// `APEIRETH_TLS_CERT` / `APEIRETH_TLS_KEY` 只是占位旋钮: 未接线。
    Unwired {
        /// 证书占位旋钮是否被设置。
        cert_env_set: bool,
        /// 私钥占位旋钮是否被设置。
        key_env_set: bool,
    },
}

impl TlsSurface {
    /// 占位旋钮被设置时给出显式警告 (设了≠启用); 未设置 = 无话可说。
    pub fn warning(&self) -> Option<String> {
        let Self::Unwired {
            cert_env_set,
            key_env_set,
        } = *self;
        if !cert_env_set && !key_env_set {
            return None;
        }
        let mut set = Vec::new();
        if cert_env_set {
            set.push(TLS_CERT_ENV);
        }
        if key_env_set {
            set.push(TLS_KEY_ENV);
        }
        Some(format!(
            "{} is set but the TLS surface is an unwired placeholder: the gateway still serves \
             plaintext HTTP (the knob is recognized and reported, never silently ignored)",
            set.join(" and ")
        ))
    }
}

/// TLS 面状态 (环境读取包装)。
pub fn tls_surface_status() -> TlsSurface {
    tls_surface_status_from(
        std::env::var(TLS_CERT_ENV).ok().as_deref(),
        std::env::var(TLS_KEY_ENV).ok().as_deref(),
    )
}

/// TLS 面状态 (纯函数形态, 供测试钉行为)。
pub fn tls_surface_status_from(cert_env: Option<&str>, key_env: Option<&str>) -> TlsSurface {
    TlsSurface::Unwired {
        cert_env_set: cert_env
            .map(str::trim)
            .is_some_and(|value| !value.is_empty()),
        key_env_set: key_env
            .map(str::trim)
            .is_some_and(|value| !value.is_empty()),
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

    #[test]
    fn tls_placeholder_knobs_are_reported_as_unwired() {
        // 两个占位旋钮都未设: 无话可说。
        assert_eq!(tls_surface_status_from(None, None).warning(), None);
        assert_eq!(
            tls_surface_status_from(Some("  "), Some("")).warning(),
            None,
            "空白按未设置处理"
        );

        // 任一被设置: 显式喊破"未接线", 且点名旋钮。
        let cert_only = tls_surface_status_from(Some("/etc/cert.pem"), None)
            .warning()
            .expect("设了证书占位旋钮必须显式警告");
        assert!(cert_only.contains(TLS_CERT_ENV), "{cert_only}");
        assert!(cert_only.contains("unwired placeholder"), "{cert_only}");
        assert!(cert_only.contains("plaintext"), "{cert_only}");

        let key_only = tls_surface_status_from(None, Some("/etc/key.pem"))
            .warning()
            .expect("设了私钥占位旋钮必须显式警告");
        assert!(key_only.contains(TLS_KEY_ENV), "{key_only}");

        let both = tls_surface_status_from(Some("c"), Some("k"))
            .warning()
            .expect("双旋钮也要警告");
        assert!(
            both.contains(TLS_CERT_ENV) && both.contains(TLS_KEY_ENV),
            "{both}"
        );
    }
}
