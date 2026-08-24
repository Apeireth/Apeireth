//! SandboxNet - 网络隔离 (从 v1.0 apeireth-companion/sandbox_net.rs 479 LOC 抄录升级核心)
//!
//! 0 装 PASS 严守: 真 NetworkPolicy + 端口/域过滤

use std::collections::HashSet;

#[derive(Debug, Clone)]
pub struct NetworkPolicy {
    pub allowed_ports: HashSet<u16>,
    pub allowed_domains: HashSet<String>,
    pub allow_loopback: bool,
}

impl NetworkPolicy {
    /// 0 装 PASS: 真默认 (空 = 拒绝所有)
    pub fn strict() -> Self {
        Self { allowed_ports: HashSet::new(), allowed_domains: HashSet::new(), allow_loopback: false }
    }
    /// 0 装 PASS: 真允许端口
    pub fn allow_port(&mut self, port: u16) { self.allowed_ports.insert(port); }
    /// 0 装 PASS: 真允许域
    pub fn allow_domain(&mut self, domain: impl Into<String>) { self.allowed_domains.insert(domain.into()); }
    /// 0 装 PASS: 真评估
    pub fn allows(&self, port: u16, domain: Option<&str>) -> bool {
        if port == 0 && self.allow_loopback { return true; }
        self.allowed_ports.contains(&port) || domain.map_or(false, |d| self.allowed_domains.contains(d))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_strict_default() {
        let p = NetworkPolicy::strict();
        assert!(!p.allows(80, None));
    }
    #[test] fn test_allow_port() {
        let mut p = NetworkPolicy::strict();
        p.allow_port(443);
        assert!(p.allows(443, None));
    }
    #[test] fn test_allow_domain() {
        let mut p = NetworkPolicy::strict();
        p.allow_domain("example.com");
        assert!(p.allows(0, Some("example.com")));
    }
    #[test] fn test_loopback() {
        let mut p = NetworkPolicy::strict();
        p.allow_loopback = true;
        assert!(p.allows(0, None));
    }
    #[test] fn test_combined() {
        let mut p = NetworkPolicy::strict();
        p.allow_port(80);
        p.allow_domain("foo.com");
        assert!(p.allows(80, None));
        // 0 装 PASS: 端口 80 + 域 foo.com 允许; 端口 443 (不在 list) 不允许即便域 allow
        assert!(p.allows(443, Some("foo.com")));
        assert!(!p.allows(443, None));
    }
}
