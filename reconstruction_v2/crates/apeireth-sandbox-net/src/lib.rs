//! apeireth-sandbox-net - Network sandbox (v2 完整抄录 v1 sandbox_net.rs)
//!
//! 0 装 PASS: 真 NetworkPolicy + 真 port/domain 检查

use std::collections::HashSet;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkPolicy {
    pub allowed_ports: HashSet<u16>,
    pub allowed_domains: HashSet<String>,
    pub allow_loopback: bool,
}

impl NetworkPolicy {
    pub fn strict() -> Self { Self { allowed_ports: HashSet::new(), allowed_domains: HashSet::new(), allow_loopback: false } }
    pub fn allow_port(&mut self, port: u16) { self.allowed_ports.insert(port); }
    pub fn allow_domain(&mut self, domain: impl Into<String>) { self.allowed_domains.insert(domain.into()); }
    pub fn allows(&self, port: u16, domain: Option<&str>) -> bool {
        let port_ok = self.allowed_ports.contains(&port);
        let domain_ok = domain.map_or(false, |d| self.allowed_domains.contains(d));
        port_ok || domain_ok
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_port_only() {
        let mut p = NetworkPolicy::strict();
        p.allow_port(443);
        assert!(p.allows(443, None));
    }
    #[test]
    fn test_domain_only() {
        let mut p = NetworkPolicy::strict();
        p.allow_domain("example.com");
        assert!(p.allows(80, Some("example.com")));
    }
    #[test]
    fn test_neither() {
        let p = NetworkPolicy::strict();
        assert!(!p.allows(443, None));
    }
}
