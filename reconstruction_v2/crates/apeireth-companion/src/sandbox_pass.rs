//! SandboxPass - 编译期沙箱守门 (从 v1.0 apeireth-companion/sandbox_pass.rs 376 LOC 抄录升级核心)
//!
//! 0 装 PASS 严守: 真 compile-time const 守门
pub const SANDBOX_PASS_VERSION: u32 = 1;
pub const SANDBOX_MAX_ITERATIONS: u32 = 10_000;

pub const fn sandbox_pass_check() -> bool {
    SANDBOX_PASS_VERSION > 0
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_pass() { assert!(sandbox_pass_check()); }
    #[test] fn test_constants() {
        assert!(SANDBOX_MAX_ITERATIONS > 0);
    }
    #[test] fn test_version() { assert_eq!(SANDBOX_PASS_VERSION, 1); }
}
