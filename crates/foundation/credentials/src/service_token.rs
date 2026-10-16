//! 常驻服务令牌面 (W2-① G10 协同)。
//!
//! 常驻服务安装面 (`packaging/service/apeireth-service.ps1` 的 install 动作)
//! 生成一枚持久令牌存入平台凭据库 —— **明文不落盘**; 服务进程只以 env
//! (`APEIRETH_GATEWAY_TOKEN`) 拿到它, 网关令牌门 (`GatewayTokenGate`)
//! 按既有三态口径执行。桌面壳 / TUI 连常驻服务时从**同一凭据条目**取同源
//! 令牌 (探测到常驻 = 读服务令牌面); 取不到 = 无令牌, 三态门警告帧语义照旧。
//!
//! 凭据条目身份是一份跨工具契约 (写入方 = 服务脚本, 读取方 = 本模块与
//! 桌面壳), 由 [`SERVICE_TOKEN_SERVICE`] / [`SERVICE_TOKEN_USER`] /
//! [`SERVICE_TOKEN_WINDOWS_TARGET`] 三常量钉住; 抽取自读取面的纯函数
//! [`read_from`] 让取值裁决 (空白 = 未配置) 可注入测试。

use crate::secret::SecretString;

/// 凭据条目「服务名」 (keyring 语义)。
pub const SERVICE_TOKEN_SERVICE: &str = "gateway-token";

/// 凭据条目「用户名」 (keyring 语义)。
pub const SERVICE_TOKEN_USER: &str = "apeireth";

/// Windows 凭据管理器目标名。
///
/// keyring 的 Windows 默认映射把 `Entry::new(service, user)` 写成目标名
/// `{user}.{service}` —— 服务脚本 (P/Invoke 写凭据) 与桌面壳 (直接读凭据)
/// 都按此名对表, 三方同源。
pub const SERVICE_TOKEN_WINDOWS_TARGET: &str = "apeireth.gateway-token";

/// 从平台凭据库读常驻服务令牌。
///
/// 未安装 / 读不到 / 空白值一律 `None` (调用方按"无令牌"回落, 三态门口径
/// 照旧); 令牌值只走 [`SecretString`] (Debug/Display 恒打码, 不入日志)。
pub fn read() -> Option<SecretString> {
    read_from(&mut |service, user| {
        keyring::Entry::new(service, user)
            .ok()
            .and_then(|entry| entry.get_password().ok())
    })
}

/// 取值裁决纯函数 (注入读取面): 只认非空白值, 其余读作未配置。
fn read_from(lookup: &mut dyn FnMut(&str, &str) -> Option<String>) -> Option<SecretString> {
    let value = lookup(SERVICE_TOKEN_SERVICE, SERVICE_TOKEN_USER)?;
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(SecretString::new(trimmed.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 三方同源的锚: Windows 目标名必须等于 keyring 默认映射
    /// `{user}.{service}` (服务脚本写入名 / 桌面壳读取名与此一致)。
    #[test]
    fn windows_target_matches_the_keyring_default_mapping() {
        assert_eq!(
            SERVICE_TOKEN_WINDOWS_TARGET,
            format!("{}.{}", SERVICE_TOKEN_USER, SERVICE_TOKEN_SERVICE),
        );
        assert_eq!(SERVICE_TOKEN_WINDOWS_TARGET, "apeireth.gateway-token");
    }

    /// 取值裁决: 缺失 / 空白 = 无令牌 (三态门照旧); 非空白 = 令牌面给出,
    /// 且呈现恒打码 (脱敏红线)。
    #[test]
    fn lookup_outcomes_map_to_the_token_face_without_leaking() {
        let none = read_from(&mut |_, _| None);
        assert!(none.is_none(), "凭据库读不到 = 无令牌");

        for blank in ["", "   ", "\t"] {
            let value = read_from(&mut |_, _| Some(blank.to_string()));
            assert!(value.is_none(), "空白值读作未配置 ({blank:?})");
        }

        let secret = read_from(&mut |_, _| Some("loopback-token-0123456789abcdef".to_string()))
            .expect("non-blank value is the token face");
        assert_eq!(secret.expose(), "loopback-token-0123456789abcdef");

        // 脱敏红线: Debug/Display 不得回显令牌值。
        let rendered = format!("{secret:?} / {secret}");
        assert!(
            !rendered.contains("loopback-token-0123456789abcdef"),
            "令牌不得进呈现: {rendered}"
        );
    }

    /// 读取面只对"服务令牌"这一个身份取值 (命名即承诺: 不读别的条目)。
    #[test]
    fn lookup_reads_exactly_the_service_token_identity() {
        let mut probed: Vec<(String, String)> = Vec::new();
        let _ = read_from(&mut |service, user| {
            probed.push((service.to_string(), user.to_string()));
            None
        });
        assert_eq!(
            probed,
            vec![(
                SERVICE_TOKEN_SERVICE.to_string(),
                SERVICE_TOKEN_USER.to_string()
            )]
        );
    }

    /// 真机探针 (默认忽略): 跨工具同源契约 —— 服务安装面按 keyring 默认目标
    /// 映射写入的凭据条目, 本读取面必须拿得到 (逐字节一致, 设
    /// `SERVICE_TOKEN_PROBE_EXPECTED` 时强校验)。探针只读不写。
    ///
    ///   SERVICE_TOKEN_PROBE_EXPECTED=<token> cargo test -p apeireth-credentials \
    ///     service_token_face_live_probe -- --ignored --nocapture
    #[test]
    #[ignore]
    fn service_token_face_live_probe() {
        match read() {
            Some(secret) => {
                if let Ok(expected) = std::env::var("SERVICE_TOKEN_PROBE_EXPECTED") {
                    assert_eq!(secret.expose(), expected, "同源令牌必须逐字节一致");
                }
                println!("SERVICE TOKEN FACE READ OK ({} bytes)", secret.len());
            }
            None => println!("SERVICE TOKEN FACE EMPTY"),
        }
    }
}
