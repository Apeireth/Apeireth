use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Permission {
    ReadMemory,
    WriteMemory,
    ExecuteTool(String),
    NetworkEgress(String),
    ModifyIdentity,
    AdminOverride,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PermissionPack {
    pub permissions: HashSet<Permission>,
}

impl PermissionPack {
    pub fn new() -> Self {
        Self {
            permissions: HashSet::new(),
        }
    }

    pub fn standard_agent() -> Self {
        let mut set = HashSet::new();
        set.insert(Permission::ReadMemory);
        set.insert(Permission::WriteMemory);
        set.insert(Permission::ExecuteTool("shell".into()));
        set.insert(Permission::ExecuteTool("fetch".into()));
        set.insert(Permission::ExecuteTool("filesystem".into()));
        set.insert(Permission::NetworkEgress("api.minimax.chat".into()));
        Self { permissions: set }
    }

    pub fn grant(&mut self, perm: Permission) {
        self.permissions.insert(perm);
    }

    pub fn has(&self, perm: &Permission) -> bool {
        self.permissions.contains(perm)
    }
}

/// Layer 1: Principle Onion (Absolute philosophical boundaries)
pub struct PrincipleOnion;
impl PrincipleOnion {
    pub fn check(action_name: &str) -> Result<(), &'static str> {
        if action_name.contains("disable_audit") {
            return Err("Principle violation: audit suppression is strictly forbidden");
        }
        if action_name.contains("escalate_privilege_unauthenticated") {
            return Err("Principle violation: unauthenticated privilege escalation");
        }
        Ok(())
    }
}

/// Layer 2: Permission Onion (ABAC grant checking)
pub struct PermissionOnion<'a> {
    pack: &'a PermissionPack,
}

impl<'a> PermissionOnion<'a> {
    pub fn new(pack: &'a PermissionPack) -> Self {
        Self { pack }
    }

    pub fn check(&self, required: &Permission) -> Result<(), String> {
        if self.pack.has(required) || self.pack.has(&Permission::AdminOverride) {
            Ok(())
        } else {
            Err(format!("Permission denied: missing required permission {:?}", required))
        }
    }
}

/// Layer 3: DSL Onion (Policy expression filter)
pub struct DslOnion;
impl DslOnion {
    pub fn evaluate_dsl_policy(policy_expr: &str, context_val: f64) -> bool {
        // e.g. "temperature < 1.2", "budget >= 0.1"
        if policy_expr.starts_with("budget >=") {
            let threshold: f64 = policy_expr.replace("budget >=", "").trim().parse().unwrap_or(0.0);
            context_val >= threshold
        } else if policy_expr.starts_with("temperature <=") {
            let threshold: f64 = policy_expr.replace("temperature <=", "").trim().parse().unwrap_or(2.0);
            context_val <= threshold
        } else {
            true
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_onion_three_layers() {
        // Layer 1
        assert!(PrincipleOnion::check("normal_chat").is_ok());
        assert!(PrincipleOnion::check("disable_audit_logs").is_err());

        // Layer 2
        let pack = PermissionPack::standard_agent();
        let perm_onion = PermissionOnion::new(&pack);
        assert!(perm_onion.check(&Permission::ReadMemory).is_ok());
        assert!(perm_onion.check(&Permission::ModifyIdentity).is_err());

        // Layer 3
        assert!(DslOnion::evaluate_dsl_policy("budget >= 0.5", 0.8));
        assert!(!DslOnion::evaluate_dsl_policy("budget >= 0.5", 0.3));
    }
}

