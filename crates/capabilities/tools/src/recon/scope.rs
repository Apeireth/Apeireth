//! 授权范围（Scope）：一次分析允许触碰的路径、域名与网段。
//!
//! Fail-closed：未显式授权、清单为空、目标越界都直接拒绝；不存在「默认放开」。

use std::net::IpAddr;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// 一次 recon 会话的授权清单。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Scope {
    /// 范围名（写入 Evidence 的 `scope` 字段）。
    pub name: String,
    /// 操作员是否显式授权；`false` 时所有检查都拒绝。
    #[serde(default)]
    pub authorized: bool,
    /// 允许读取的路径前缀（目录或文件）。
    #[serde(default)]
    pub paths: Vec<PathBuf>,
    /// 允许访问的域名（精确匹配，或子域匹配）。
    #[serde(default)]
    pub domains: Vec<String>,
    /// 允许访问的 IP 网段（`a.b.c.d/len` 或 IPv6 CIDR）。
    #[serde(default)]
    pub cidrs: Vec<String>,
}

/// Scope 校验失败。
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ScopeError {
    /// 未授权。
    #[error("scope {0:?} is not authorized by the operator")]
    NotAuthorized(String),
    /// 目标越界。
    #[error("target {0:?} is outside the authorized scope")]
    Outside(String),
    /// 清单条目畸形。
    #[error("scope entry {0:?} is malformed")]
    Malformed(String),
    /// 目标路径无法解析（不存在或不可读）。
    #[error("target path {0:?} cannot be resolved")]
    Unresolvable(String),
}

impl Scope {
    /// 构造一个未授权空范围。
    pub fn empty(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            authorized: false,
            paths: Vec::new(),
            domains: Vec::new(),
            cidrs: Vec::new(),
        }
    }

    /// 结构校验：必须显式授权，且每个 CIDR 条目可解析。
    pub fn validate(&self) -> Result<(), ScopeError> {
        if !self.authorized {
            return Err(ScopeError::NotAuthorized(self.name.clone()));
        }
        if self.paths.is_empty() && self.domains.is_empty() && self.cidrs.is_empty() {
            return Err(ScopeError::NotAuthorized(self.name.clone()));
        }
        for cidr in &self.cidrs {
            if parse_cidr(cidr).is_none() {
                return Err(ScopeError::Malformed(cidr.clone()));
            }
        }
        for domain in &self.domains {
            if domain.trim().trim_start_matches("*.").is_empty() {
                return Err(ScopeError::Malformed(domain.clone()));
            }
        }
        Ok(())
    }

    /// 目标路径必须落在某个授权路径前缀下（两者都先 canonicalize）。
    pub fn check_path(&self, target: &Path) -> Result<PathBuf, ScopeError> {
        self.validate()?;
        let target_canonical = target
            .canonicalize()
            .map_err(|_| ScopeError::Unresolvable(target.display().to_string()))?;
        for allowed in &self.paths {
            if let Ok(allowed_canonical) = allowed.canonicalize() {
                if target_canonical.starts_with(&allowed_canonical) {
                    return Ok(target_canonical);
                }
            }
        }
        Err(ScopeError::Outside(target.display().to_string()))
    }

    /// 域名必须精确匹配，或是某个授权域名的子域。
    pub fn check_domain(&self, domain: &str) -> Result<(), ScopeError> {
        self.validate()?;
        let target = domain.trim().trim_end_matches('.').to_ascii_lowercase();
        for allowed in &self.domains {
            let allowed = allowed
                .trim()
                .trim_start_matches("*.")
                .trim_end_matches('.')
                .to_ascii_lowercase();
            if target == allowed || target.ends_with(&format!(".{allowed}")) {
                return Ok(());
            }
        }
        Err(ScopeError::Outside(domain.to_string()))
    }

    /// IP 必须落在某个授权网段内。
    pub fn check_address(&self, address: IpAddr) -> Result<(), ScopeError> {
        self.validate()?;
        for cidr in &self.cidrs {
            if let Some((network, prefix)) = parse_cidr(cidr) {
                if cidr_contains(network, prefix, address) {
                    return Ok(());
                }
            }
        }
        Err(ScopeError::Outside(address.to_string()))
    }
}

/// 解析 `ip/prefix`，前缀长度非法时返回 `None`。
fn parse_cidr(raw: &str) -> Option<(IpAddr, u8)> {
    let (address, prefix) = raw.trim().split_once('/')?;
    let address: IpAddr = address.trim().parse().ok()?;
    let prefix: u8 = prefix.trim().parse().ok()?;
    let bits = if address.is_ipv4() { 32 } else { 128 };
    (prefix <= bits).then_some((address, prefix))
}

/// 判断 `address` 是否属于 `network/prefix`。
fn cidr_contains(network: IpAddr, prefix: u8, address: IpAddr) -> bool {
    let (network, address, bits) = match (network, address) {
        (IpAddr::V4(n), IpAddr::V4(a)) => {
            (u128::from(u32::from(n)), u128::from(u32::from(a)), 32u8)
        }
        (IpAddr::V6(n), IpAddr::V6(a)) => (u128::from(n), u128::from(a), 128u8),
        _ => return false,
    };
    if prefix == 0 {
        return true;
    }
    if prefix > bits {
        return false;
    }
    let mask = u128::MAX << (bits - prefix);
    (network & mask) == (address & mask)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{Ipv4Addr, Ipv6Addr};

    fn demo_scope(root: &Path) -> Scope {
        Scope {
            name: "demo".into(),
            authorized: true,
            paths: vec![root.to_path_buf()],
            domains: vec!["example.com".into(), "*.corp.example".into()],
            cidrs: vec!["10.0.0.0/8".into(), "2001:db8::/32".into()],
        }
    }

    #[test]
    fn unauthorized_or_empty_scope_denies_everything() {
        let scope = Scope::empty("demo");
        assert!(matches!(
            scope.check_domain("example.com"),
            Err(ScopeError::NotAuthorized(_))
        ));

        let mut scope = Scope::empty("demo");
        scope.authorized = true;
        assert!(matches!(
            scope.check_path(Path::new("/tmp")),
            Err(ScopeError::NotAuthorized(_))
        ));
    }

    #[test]
    fn paths_domains_and_addresses_are_scoped() {
        let root = std::env::temp_dir().join(format!(
            "apeireth-scope-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&root).expect("temp dir");
        let inside = root.join("sample.bin");
        std::fs::write(&inside, b"demo").expect("write sample");

        // 跨平台：目录外路径必须**真实存在**。
        // 原来写死 Unix 的 `/etc/hostname`：Windows 上它不是绝对路径、macOS 上该文件
        // 也不保证存在 → `canonicalize` 失败会返回 `Unresolvable` 而不是 `Outside`，
        // 于是三平台里只有 Linux 过（2026-10-23 CI 抓到）。
        let outside_root = std::env::temp_dir().join(format!(
            "apeireth-scope-outside-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&outside_root).expect("outside dir");
        let outside = outside_root.join("outside.bin");
        std::fs::write(&outside, b"demo").expect("write outside");

        let scope = demo_scope(&root);
        scope.validate().expect("valid scope");
        assert!(scope.check_path(&inside).is_ok());
        assert!(matches!(
            scope.check_path(&outside),
            Err(ScopeError::Outside(_))
        ));

        assert!(scope.check_domain("example.com").is_ok());
        assert!(scope.check_domain("api.example.com").is_ok());
        assert!(scope.check_domain("team.corp.example").is_ok());
        assert!(matches!(
            scope.check_domain("evil-example.com"),
            Err(ScopeError::Outside(_))
        ));

        assert!(scope
            .check_address(IpAddr::V4(Ipv4Addr::new(10, 1, 2, 3)))
            .is_ok());
        assert!(matches!(
            scope.check_address(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1))),
            Err(ScopeError::Outside(_))
        ));
        assert!(scope
            .check_address(IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1)))
            .is_ok());

        std::fs::remove_dir_all(&root).ok();
        std::fs::remove_dir_all(&outside_root).ok();
    }

    #[test]
    fn malformed_cidrs_are_rejected() {
        let mut scope = Scope::empty("demo");
        scope.authorized = true;
        scope.cidrs = vec!["10.0.0.0/33".into()];
        assert!(matches!(scope.validate(), Err(ScopeError::Malformed(_))));
    }
}
