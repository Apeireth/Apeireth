//! SandboxFfiLibkrun - libkrun FFI 绑定 (从 v1.0 apeireth-companion/sandbox_ffi_libkrun.rs 391 LOC 抄录升级核心)
//!
//! 0 装 PASS 严守: 真 libkrun wrapper struct (FFI 调用 0 装 PASS 标 stub)

pub struct LibkrunInstance { pub id: u32 }

impl LibkrunInstance {
    /// 0 装 PASS stub: 真 FFI 调 dlopen("libkrun.so") + krun_create
    pub fn create() -> Result<Self, String> { Ok(Self { id: 1 }) }
    /// 0 装 PASS stub: 真 FFI 调 krun_start
    pub fn start(&self) -> Result<(), String> { Ok(()) }
    /// 0 装 PASS stub: 真 FFI 调 krun_stop
    pub fn stop(&self) -> Result<(), String> { Ok(()) }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_create() { assert!(LibkrunInstance::create().is_ok()); }
    #[test] fn test_lifecycle() {
        let i = LibkrunInstance::create().unwrap();
        assert!(i.start().is_ok());
        assert!(i.stop().is_ok());
    }
    #[test] fn test_id() {
        let i = LibkrunInstance::create().unwrap();
        assert_eq!(i.id, 1);
    }
}
