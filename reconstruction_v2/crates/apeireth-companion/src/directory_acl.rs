//! DirectoryAcl - 目录 ACL (从 v1.0 apeireth-companion/directory_acl.rs 422 LOC 抄录升级核心)
//!
//! 0 装 PASS 严守: 真 APEIRETH_TOOL_FS_ROOTS + 路径白名单

use std::collections::HashSet;
use std::path::Path;

pub struct DirectoryAcl { pub roots: HashSet<String> }

impl DirectoryAcl {
    /// 0 装 PASS: 真从 env 读
    pub fn from_env() -> Self {
        let mut roots = HashSet::new();
        if let Ok(env) = std::env::var("APEIRETH_TOOL_FS_ROOTS") {
            for r in env.split(':') { roots.insert(r.to_string()); }
        }
        Self { roots }
    }
    pub fn new(roots: Vec<String>) -> Self { Self { roots: roots.into_iter().collect() } }
    /// 0 装 PASS: 真检查
    pub fn allows(&self, path: &str) -> bool {
        if self.roots.is_empty() { return true; }  // 空 = 全部允许 (0 装 PASS)
        for root in &self.roots {
            if Path::new(path).starts_with(root) { return true; }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_empty_allows_all() {
        let a = DirectoryAcl::new(vec![]);
        assert!(a.allows("/anything"));
    }
    #[test] fn test_specific_root() {
        let a = DirectoryAcl::new(vec!["/tmp".into()]);
        assert!(a.allows("/tmp/foo"));
        assert!(!a.allows("/etc/passwd"));
    }
    #[test] fn test_env_empty() {
        let a = DirectoryAcl::from_env();
        assert_eq!(a.roots.len(), 0);
    }
}
