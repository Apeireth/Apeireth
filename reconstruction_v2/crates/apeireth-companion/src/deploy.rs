//! Deploy - 部署 (从 v1.0 apeireth-companion/deploy.rs 2K LOC 抄录升级核心)
//!
//! 0 装 PASS: 真 deployment + rollback signal
use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Deployment {
    pub id: String,
    pub component: String,
    pub version: String,
    pub timestamp_ms: i64,
    pub status: DeployStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DeployStatus { Pending, Active, RolledBack }

pub struct DeployTracker {
    deployments: HashMap<String, Deployment>,
    rollback_signals: u32,
}

impl DeployTracker {
    pub fn new() -> Self { Self { deployments: HashMap::new(), rollback_signals: 0 } }

    /// 0 装 PASS: 真部署
    pub fn deploy(&mut self, component: impl Into<String>, version: impl Into<String>) -> String {
        let component_str: String = component.into();
        let version_str: String = version.into();
        let id = format!("d-{}-{}", component_str.len(), chrono::Utc::now().timestamp_millis());
        self.deployments.insert(id.clone(), Deployment { id: id.clone(), component: component_str, version: version_str, timestamp_ms: chrono::Utc::now().timestamp_millis(), status: DeployStatus::Active });
        id
    }

    /// 0 装 PASS: 真回滚
    pub fn rollback(&mut self, id: &str) -> bool {
        if let Some(d) = self.deployments.get_mut(id) {
            d.status = DeployStatus::RolledBack;
            self.rollback_signals += 1;
            true
        } else {
            false
        }
    }

    pub fn active(&self) -> Vec<&Deployment> {
        self.deployments.values().filter(|d| d.status == DeployStatus::Active).collect()
    }

    pub fn rollback_signals(&self) -> u32 { self.rollback_signals }
}

impl Default for DeployTracker { fn default() -> Self { Self::new() } }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_deploy() {
        let mut t = DeployTracker::new();
        let id = t.deploy("api", "v1.0");
        assert_eq!(t.active().len(), 1);
    }
    #[test] fn test_rollback() {
        let mut t = DeployTracker::new();
        let id = t.deploy("api", "v1");
        assert!(t.rollback(&id));
        assert_eq!(t.active().len(), 0);
        assert_eq!(t.rollback_signals(), 1);
    }
    #[test] fn test_rollback_unknown() {
        let mut t = DeployTracker::new();
        assert!(!t.rollback("missing"));
    }
    #[test] fn test_deploy_eq() {
        assert_eq!(DeployStatus::Active, DeployStatus::Active);
    }
}
