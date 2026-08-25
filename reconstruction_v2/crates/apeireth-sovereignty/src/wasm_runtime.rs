//! WASM sandbox runtime

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum WasmError {
    #[error("wasm: module bytes empty")]
    EmptyModule,
    #[error("wasm: module bytes exceed limit {limit}, got {actual}")]
    ModuleTooLarge { limit: usize, actual: usize },
    #[error("wasm: magic number mismatch")]
    BadMagic([u8; 4]),
    #[error("wasm: unknown version {0}")]
    BadVersion(u32),
    #[error("wasm: policy denied: {0}")]
    PolicyDenied(String),
    #[error("wasm: fuel exhausted (spent {spent} of {budget})")]
    FuelExhausted { spent: u64, budget: u64 },
    #[error("wasm: memory limit exceeded (peak {peak} bytes of {limit})")]
    MemoryExceeded { peak: usize, limit: usize },
    #[error("wasm: capability not granted: {0}")]
    CapabilityMissing(String),
    #[error("wasm: execution fault: {0}")]
    Fault(String),
}

pub type WasmResult<T> = Result<T, WasmError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WasmCapability { ReadFs, WriteScratch, Network, Process, EnvRead, Trace }

impl WasmCapability {
    pub const ALL: [WasmCapability; 6] = [Self::ReadFs, Self::WriteScratch, Self::Network, Self::Process, Self::EnvRead, Self::Trace];
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmPolicy {
    pub memory_limit_bytes: usize,
    pub fuel_budget: u64,
    pub max_module_bytes: usize,
    pub capabilities: Vec<WasmCapability>,
    pub name: String,
}

impl WasmPolicy {
    pub fn strict(name: impl Into<String>) -> Self {
        Self { memory_limit_bytes: 16 * 1024 * 1024, fuel_budget: 1_000_000, max_module_bytes: 256 * 1024, capabilities: vec![WasmCapability::Trace], name: name.into() }
    }
    pub fn allow(&mut self, cap: WasmCapability) { if !self.capabilities.contains(&cap) { self.capabilities.push(cap); } }
    pub fn deny(&mut self, cap: WasmCapability) { self.capabilities.retain(|c| *c != cap); }
    pub fn has(&self, cap: WasmCapability) -> bool { self.capabilities.contains(&cap) }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmModule {
    pub name: String,
    pub bytes: Vec<u8>,
    pub content_hash: String,
}

impl WasmModule {
    pub fn new(name: impl Into<String>, bytes: Vec<u8>) -> WasmResult<Self> {
        if bytes.is_empty() { return Err(WasmError::EmptyModule); }
        let content_hash = hex_sha256(&bytes);
        Ok(Self { name: name.into(), bytes, content_hash })
    }
    pub fn validate_magic(&self) -> WasmResult<()> {
        if self.bytes.len() < 8 { return Err(WasmError::BadMagic([0; 4])); }
        let mut magic = [0u8; 4]; magic.copy_from_slice(&self.bytes[0..4]);
        if &magic != b"\0asm" { return Err(WasmError::BadMagic(magic)); }
        let version = u32::from_le_bytes([self.bytes[4], self.bytes[5], self.bytes[6], self.bytes[7]]);
        if version != 1 { return Err(WasmError::BadVersion(version)); }
        Ok(())
    }
    pub fn len(&self) -> usize { self.bytes.len() }
}

fn hex_sha256(bytes: &[u8]) -> String {
    let mut h = Sha256::new(); h.update(bytes); let digest = h.finalize();
    let mut s = String::with_capacity(64);
    for b in digest { s.push_str(&format!("{:02x}", b)); }
    s
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmExecution {
    pub module_hash: String,
    pub policy_name: String,
    pub fuel_spent: u64,
    pub memory_peak_bytes: usize,
    pub capabilities_used: Vec<WasmCapability>,
    pub fault: Option<String>,
    pub return_code: i32,
    pub duration_ms: u64,
}

impl WasmExecution {
    pub fn is_ok(&self) -> bool { self.fault.is_none() && self.return_code == 0 }
}

pub trait WasmRuntime: Send + Sync {
    fn name(&self) -> &str;
    fn validate(&self, module: &WasmModule, policy: &WasmPolicy) -> WasmResult<()>;
    fn execute(&self, module: &WasmModule, policy: &WasmPolicy, entry: &str, args: &[String], now_ms: u64) -> WasmResult<WasmExecution>;
}

pub struct StubWasmRuntime;
impl WasmRuntime for StubWasmRuntime {
    fn name(&self) -> &str { "stub" }
    fn validate(&self, module: &WasmModule, policy: &WasmPolicy) -> WasmResult<()> {
        if module.len() > policy.max_module_bytes { return Err(WasmError::ModuleTooLarge { limit: policy.max_module_bytes, actual: module.len() }); }
        module.validate_magic()?;
        Ok(())
    }
    fn execute(&self, module: &WasmModule, policy: &WasmPolicy, entry: &str, args: &[String], now_ms: u64) -> WasmResult<WasmExecution> {
        if entry.is_empty() { return Err(WasmError::Fault("entry is empty".into())); }
        let fuel_spent = (module.len() as u64) + (args.len() as u64) * 7;
        if fuel_spent > policy.fuel_budget { return Err(WasmError::FuelExhausted { spent: fuel_spent, budget: policy.fuel_budget }); }
        let memory_peak = (module.len() / 4).max(1024);
        if memory_peak > policy.memory_limit_bytes { return Err(WasmError::MemoryExceeded { peak: memory_peak, limit: policy.memory_limit_bytes }); }
        Ok(WasmExecution { module_hash: module.content_hash.clone(), policy_name: policy.name.clone(), fuel_spent, memory_peak_bytes: memory_peak, capabilities_used: vec![], fault: None, return_code: 0, duration_ms: now_ms })
    }
}

pub struct WasmRegistry { runtimes: Vec<Box<dyn WasmRuntime>> }
impl Default for WasmRegistry { fn default() -> Self { Self::new() } }

impl WasmRegistry {
    pub fn new() -> Self { Self { runtimes: Vec::new() } }
    pub fn with_stub() -> Self { let mut r = Self::new(); r.register(Box::new(StubWasmRuntime)); r }
    pub fn register(&mut self, rt: Box<dyn WasmRuntime>) { self.runtimes.push(rt); }
    pub fn get(&self, name: &str) -> Option<&dyn WasmRuntime> { self.runtimes.iter().find(|r| r.name() == name).map(|r| r.as_ref()) }
    pub fn names(&self) -> Vec<&str> { self.runtimes.iter().map(|r| r.name()).collect() }
}

#[cfg(test)]
mod tests {
    use super::*;
    const DUMMY: &[u8] = b"\0asm\x01\x00\x00\x00rest";
    fn tiny() -> WasmModule { WasmModule::new("tiny", DUMMY.to_vec()).unwrap() }

    #[test] fn module_hash_len_64() { assert_eq!(tiny().content_hash.len(), 64); }
    #[test] fn module_empty_rejected() { assert!(matches!(WasmModule::new("x", vec![]), Err(WasmError::EmptyModule))); }
    #[test] fn validate_magic_ok() { tiny().validate_magic().unwrap(); }
    #[test] fn validate_magic_bad() {
        let mut b = DUMMY.to_vec(); b[0] = b'X';
        let m = WasmModule::new("b", b).unwrap();
        assert!(matches!(m.validate_magic(), Err(WasmError::BadMagic(_))));
    }
    #[test] fn validate_magic_bad_version() {
        let mut b = DUMMY.to_vec(); b[4] = 99;
        let m = WasmModule::new("v", b).unwrap();
        assert!(matches!(m.validate_magic(), Err(WasmError::BadVersion(99))));
    }
    #[test] fn validate_magic_short() {
        let m = WasmModule::new("s", vec![0, 1]).unwrap();
        assert!(matches!(m.validate_magic(), Err(WasmError::BadMagic(_))));
    }
    #[test] fn policy_strict_defaults() {
        let p = WasmPolicy::strict("t");
        assert_eq!(p.name, "t");
        assert!(p.memory_limit_bytes > 0);
        assert!(p.fuel_budget > 0);
        assert!(p.has(WasmCapability::Trace));
        assert!(!p.has(WasmCapability::Network));
    }
    #[test] fn policy_allow_deny() {
        let mut p = WasmPolicy::strict("t");
        p.allow(WasmCapability::Network); assert!(p.has(WasmCapability::Network));
        p.deny(WasmCapability::Network); assert!(!p.has(WasmCapability::Network));
    }
    #[test] fn policy_allow_idempotent() {
        let mut p = WasmPolicy::strict("t");
        p.allow(WasmCapability::Network); p.allow(WasmCapability::Network);
        assert_eq!(p.capabilities.iter().filter(|c| **c == WasmCapability::Network).count(), 1);
    }
    #[test] fn stub_validate_oversize() {
        let big = vec![0u8; 1024 * 1024];
        let m = WasmModule::new("big", big).unwrap();
        let p = WasmPolicy::strict("t");
        assert!(matches!(StubWasmRuntime.validate(&m, &p), Err(WasmError::ModuleTooLarge { .. })));
    }
    #[test] fn stub_validate_in_bounds() {
        let m = tiny(); let p = WasmPolicy::strict("t");
        assert!(StubWasmRuntime.validate(&m, &p).is_ok());
    }
    #[test] fn stub_execute_ok() {
        let m = tiny(); let p = WasmPolicy::strict("t");
        let e = StubWasmRuntime.execute(&m, &p, "main", &["a".into(), "b".into()], 100).unwrap();
        assert!(e.is_ok());
        assert_eq!(e.module_hash, m.content_hash);
        assert!(e.fuel_spent > 0);
    }
    #[test] fn stub_execute_empty_entry() {
        let m = tiny(); let p = WasmPolicy::strict("t");
        assert!(matches!(StubWasmRuntime.execute(&m, &p, "", &[], 0), Err(WasmError::Fault(_))));
    }
    #[test] fn stub_execute_fuel() {
        let mut p = WasmPolicy::strict("t"); p.fuel_budget = 5;
        let m = tiny();
        assert!(matches!(StubWasmRuntime.execute(&m, &p, "main", &[], 0), Err(WasmError::FuelExhausted { .. })));
    }
    #[test] fn stub_execute_memory() {
        let mut p = WasmPolicy::strict("t"); p.memory_limit_bytes = 4;
        let m = tiny();
        assert!(matches!(StubWasmRuntime.execute(&m, &p, "main", &[], 0), Err(WasmError::MemoryExceeded { .. })));
    }
    #[test] fn registry_with_stub() {
        let r = WasmRegistry::with_stub();
        assert_eq!(r.names().len(), 1);
        assert_eq!(r.names()[0], "stub");
        assert!(r.get("stub").is_some());
        assert!(r.get("wasmtime").is_none());
    }
    #[test] fn execution_is_ok_semantics() {
        let mut e = WasmExecution { module_hash: "h".into(), policy_name: "p".into(), fuel_spent: 1, memory_peak_bytes: 1, capabilities_used: vec![], fault: None, return_code: 0, duration_ms: 0 };
        assert!(e.is_ok());
        e.return_code = 1; assert!(!e.is_ok());
        e.return_code = 0; e.fault = Some("b".into()); assert!(!e.is_ok());
    }
    #[test] fn sha256_deterministic() {
        let a = hex_sha256(b"hello"); let b = hex_sha256(b"hello");
        assert_eq!(a, b); assert_ne!(a, hex_sha256(b"hellp"));
    }
    #[test] fn capability_all_count() {
        assert_eq!(WasmCapability::ALL.len(), 6);
    }
}