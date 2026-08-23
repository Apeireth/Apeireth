use sha2::{Digest, Sha256};
use std::net::IpAddr;
use std::sync::RwLock;

#[derive(Debug, Clone)]
pub struct EgressRule {
    pub allowed_hosts: Vec<String>,
    pub allowed_ports: Vec<u16>,
    pub block_private_ips: bool,
}

impl Default for EgressRule {
    fn default() -> Self {
        Self {
            allowed_hosts: vec![
                "api.openai.com".into(),
                "api.anthropic.com".into(),
                "generativelanguage.googleapis.com".into(),
                "api.minimax.chat".into(),
                "allowed.com".into(),
            ],
            allowed_ports: vec![80, 443],
            block_private_ips: true,
        }
    }
}

pub struct EgressFilter {
    rules: RwLock<EgressRule>,
}

impl Default for EgressFilter {
    fn default() -> Self {
        Self::new()
    }
}

impl EgressFilter {
    pub fn new() -> Self {
        Self {
            rules: RwLock::new(EgressRule::default()),
        }
    }

    pub fn with_rules(rules: EgressRule) -> Self {
        Self {
            rules: RwLock::new(rules),
        }
    }

    pub fn add_allowed_host(&self, host: impl Into<String>) {
        if let Ok(mut r) = self.rules.write() {
            r.allowed_hosts.push(host.into());
        }
    }

    /// Check if target host and port are permitted under S4 Default-Deny policy.
    pub fn validate_target(&self, host: &str, port: u16) -> Result<String, String> {
        let rules = self.rules.read().map_err(|e| e.to_string())?;

        // 1. Port restriction
        if !rules.allowed_ports.contains(&port) {
            return Err(format!("forbidden_egress_port: Port {} is not permitted", port));
        }

        // 2. Private IP block (SSRF protection across all platforms)
        if rules.block_private_ips {
            if let Ok(ip) = host.parse::<IpAddr>() {
                if is_private_or_loopback(&ip) {
                    return Err("forbidden_egress_private_ip: Access to private/loopback network is blocked".into());
                }
            }
        }

        // 3. Host / Domain whitelist check
        let is_allowed = rules.allowed_hosts.iter().any(|allowed| {
            if allowed.starts_with("*.") {
                let suffix = &allowed[1..];
                host.ends_with(suffix)
            } else {
                host.eq_ignore_ascii_case(allowed)
            }
        });

        if !is_allowed {
            return Err(format!("forbidden_egress_domain: {} is not in allowed egress whitelist", host));
        }

        // 4. Generate SHA-256 hash of outbound target for immutable audit logging
        let mut hasher = Sha256::new();
        hasher.update(format!("{}:{}", host, port).as_bytes());
        let egress_hash = format!("{:x}", hasher.finalize());

        Ok(egress_hash)
    }

    /// Static backward-compatible helper
    pub fn check_domain(domain: &str) -> Result<(), String> {
        let filter = Self::new();
        filter.validate_target(domain, 443).map(|_| ())
    }
}

fn is_private_or_loopback(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            v4.is_loopback()
                || v4.is_private()
                || v4.is_link_local()
                || v4.is_broadcast()
                || v4.is_unspecified()
        }
        IpAddr::V6(v6) => {
            v6.is_loopback() || v6.is_unspecified()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_egress_whitelist_and_audit_hash() {
        let filter = EgressFilter::new();
        
        // Allowed hosts pass and generate SHA-256 hash
        let hash = filter.validate_target("api.openai.com", 443);
        assert!(hash.is_ok());
        assert_eq!(hash.unwrap().len(), 64);

        // Disallowed hosts fail-closed
        let err = filter.validate_target("malicious-exfiltration.net", 443);
        assert!(err.is_err());
        assert!(err.unwrap_err().contains("forbidden_egress_domain"));

        // Private loopback IP blocked
        let ip_err = filter.validate_target("127.0.0.1", 80);
        assert!(ip_err.is_err());

        // Disallowed port blocked
        let port_err = filter.validate_target("api.openai.com", 22);
        assert!(port_err.is_err());
    }
}
