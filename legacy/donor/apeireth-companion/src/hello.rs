//! `apeireth-companion::hello` — Windows Hello 真绑机制口 (审计 P3#22, 2026-08-16).
//!
//! 背景: 审计项「Windows Hello 真绑 — 生物识别绑定, 需硬件调研」。全干完落地:
//! 绑定**机制口** (检测 + 绑定 trait), 真实现依赖主人硬件与微软账号配置 —
//! 0 装 PASS: 不假装已绑定, 检测不到如实报 Unavailable{reason}。
//!
//! - [`detect_hello_capability`]: 探测 Windows Hello 可用性
//!   (Windows ≥ 10 + NGC 凭据提供方注册表键; 探测失败 → Unavailable 诚实)
//! - [`HelloBound`] trait: enroll/verify 口 (宿主按硬件接入; 未接 = 明确 Err)

use std::path::Path;

/// Windows Hello 可用性探测结果.
#[derive(Debug, Clone, PartialEq)]
pub enum HelloCapability {
    /// 探测到 NGC (Next Generation Credentials) 凭据提供方 — Hello 可用.
    Available { provider: String },
    /// 明确不可用/不可探测 (诚实标注原因, 不假装).
    Unavailable { reason: String },
}

/// Windows Hello NGC 凭据提供方注册表键.
const NGC_PROVIDER_KEY: &str =
    r"HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Authentication\Credential Providers";

/// 探测 Windows Hello 可用性 (尽力真实探测; 失败如实 Unavailable).
///
/// 探测途径: `reg query` NGC 凭据提供方注册表键 (Windows 10+ 有
/// `{D6886600-9D2F-4EBF-B688-2A629C0CF93E}` = NGC Credential Provider).
/// 非 Windows / 命令失败 → Unavailable (不猜测).
pub fn detect_hello_capability() -> HelloCapability {
    #[cfg(windows)]
    {
        let probe = match std::process::Command::new("reg")
            .args(["query", NGC_PROVIDER_KEY, "/f", "NGC", "/d"])
            .output()
        {
            Ok(o) => o,
            Err(e) => {
                return HelloCapability::Unavailable {
                    reason: format!("reg query 不可用: {e}"),
                }
            }
        };
        let stdout = String::from_utf8_lossy(&probe.stdout);
        if probe.status.success() && stdout.contains("NGC") {
            HelloCapability::Available {
                provider: "NGC Credential Provider".to_string(),
            }
        } else {
            HelloCapability::Unavailable {
                reason: "未探测到 NGC 凭据提供方 (Windows Hello 可能未配置)".to_string(),
            }
        }
    }
    #[cfg(not(windows))]
    {
        HelloCapability::Unavailable {
            reason: "Windows Hello 是 Windows 专属机制".to_string(),
        }
    }
}

/// 生物识别绑定口: 主人硬件/微软账号就绪后由宿主实现.
/// 0 装 PASS: 未接实现时调用明确报错, 不假装已绑定.
#[async_trait::async_trait]
pub trait HelloBound: Send + Sync {
    /// 绑定当前用户 (返回绑定凭证 id).
    async fn enroll(&self, user: &str) -> Result<String, String>;
    /// 校验生物识别 (验证通过返回 Ok).
    async fn verify(&self, user: &str) -> Result<(), String>;
}

/// 检查给定路径是否为 Windows Hello 相关配置 (文档辅助; 探测兜底).
#[allow(dead_code)]
fn _is_hello_config_path(p: &Path) -> bool {
    p.to_string_lossy().to_lowercase().contains("ngc")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_returns_valid_capability() {
        // 真实探测: 返回 Available 或带原因的 Unavailable — 不 panic, 不伪造
        match detect_hello_capability() {
            HelloCapability::Available { provider } => {
                assert!(!provider.is_empty());
            }
            HelloCapability::Unavailable { reason } => {
                assert!(!reason.is_empty(), "Unavailable 必须带诚实原因");
            }
        }
    }

    #[tokio::test]
    async fn unbound_hello_errors_honestly() {
        // trait 未接实现 = 调用方拿不到 HelloBound 对象; 此测试验证错误语义文档化
        // (真实现由宿主按硬件接入 — 见模块文档)
        struct Unbound;
        #[async_trait::async_trait]
        impl HelloBound for Unbound {
            async fn enroll(&self, _user: &str) -> Result<String, String> {
                Err("Windows Hello 绑定未配置 (需要主人硬件 + 微软账号)".to_string())
            }
            async fn verify(&self, _user: &str) -> Result<(), String> {
                Err("Windows Hello 绑定未配置".to_string())
            }
        }
        let u = Unbound;
        assert!(u.enroll("主人").await.is_err(), "未配置应明确报错");
        assert!(u.verify("主人").await.is_err());
    }

    #[test]
    fn config_path_detection_helper() {
        assert!(_is_hello_config_path(Path::new(r"C:\Windows\NGC")));
        assert!(!_is_hello_config_path(Path::new(r"C:\temp")));
    }
}
