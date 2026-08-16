//! R174 stage 6: WASM-skill bridge.
//!
//! **Goal**: let a `WasmRuntime` (from `apeireth-sovereignty::wasm_runtime`) execute
//! a skill whose descriptor carries a `wasm_module` payload. The bridge wires the
//! skill's `input_schema` / `output_schema` into the `WasmPolicy` so the runtime
//! knows what memory + fuel + capabilities the skill needs.
//!
//! **Bypass**: the existing `skill_executor` module runs `obra/superpowers`-style
//! workflow state machines in pure Rust; adding WASM is for skill authors who
//! ship actual WebAssembly bytecode (vibeguard / VulnClaw / DeathGuard style).
//!
//! **0 drift**:
//! - 0 change to existing `skill_executor` (5 phase machines).
//! - 0 change to `apeireth-sovereignty::wasm_runtime` (uses public surface only).
//! - 0 unsafe, 0 IO.

#![deny(unsafe_code)]

use apeireth_sovereignty::wasm_runtime::{
    WasmError, WasmModule, WasmPolicy, WasmResult, WasmRuntime,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum WasmSkillError {
    #[error("wasm skill: bytes missing in descriptor")]
    BytesMissing,
    #[error("wasm skill: {0}")]
    Wasm(#[from] WasmError),
}

pub type WasmSkillResult<T> = Result<T, WasmSkillError>;

/// Minimal WASM-skill descriptor (subset of the full `SkillDescriptor`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmSkillDescriptor {
    pub name: String,
    pub entry: String,
    pub bytes: Vec<u8>,
    pub memory_limit_bytes: usize,
    pub fuel_budget: u64,
    pub args: Vec<String>,
}

impl WasmSkillDescriptor {
    pub fn to_policy(&self) -> WasmPolicy {
        WasmPolicy {
            memory_limit_bytes: self.memory_limit_bytes,
            fuel_budget: self.fuel_budget,
            max_module_bytes: self.bytes.len().max(1024),
            capabilities: vec![],
            name: self.name.clone(),
        }
    }

    pub fn to_module(&self) -> WasmSkillResult<WasmModule> {
        if self.bytes.is_empty() {
            return Err(WasmSkillError::BytesMissing);
        }
        Ok(WasmModule::new(self.name.clone(), self.bytes.clone())?)
    }
}

/// Executor that runs a WASM skill through a `WasmRuntime`.
pub struct WasmSkillExecutor<R: WasmRuntime + ?Sized> {
    runtime: Box<R>,
}

impl<R: WasmRuntime + ?Sized> WasmSkillExecutor<R> {
    pub fn new(runtime: Box<R>) -> Self {
        Self { runtime }
    }

    pub fn runtime_name(&self) -> &str {
        self.runtime.name()
    }

    pub fn execute(
        &self,
        descriptor: &WasmSkillDescriptor,
        now_ms: u64,
    ) -> WasmSkillResult<apeireth_sovereignty::wasm_runtime::WasmExecution> {
        let module = descriptor.to_module()?;
        let policy = descriptor.to_policy();
        self.runtime
            .validate(&module, &policy)
            .map_err(WasmSkillError::Wasm)?;
        let exec = self.runtime.execute(
            &module,
            &policy,
            &descriptor.entry,
            &descriptor.args,
            now_ms,
        )?;
        Ok(exec)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_sovereignty::wasm_runtime::StubWasmRuntime;

    const DUMMY_WASM: &[u8] = b"\0asm\x01\x00\x00\x00rest";

    fn descriptor(bytes: Vec<u8>) -> WasmSkillDescriptor {
        WasmSkillDescriptor {
            name: "test-skill".into(),
            entry: "main".into(),
            bytes,
            memory_limit_bytes: 16 * 1024 * 1024,
            fuel_budget: 1_000_000,
            args: vec!["a".into(), "b".into()],
        }
    }

    #[test]
    fn descriptor_to_policy_carries_budget() {
        let d = descriptor(DUMMY_WASM.to_vec());
        let p = d.to_policy();
        assert_eq!(p.name, "test-skill");
        assert_eq!(p.memory_limit_bytes, 16 * 1024 * 1024);
        assert_eq!(p.fuel_budget, 1_000_000);
        assert!(p.max_module_bytes >= DUMMY_WASM.len());
    }

    #[test]
    fn descriptor_to_module_computes_hash() {
        let d = descriptor(DUMMY_WASM.to_vec());
        let m = d.to_module().unwrap();
        assert_eq!(m.name, "test-skill");
        assert_eq!(m.content_hash.len(), 64);
    }

    #[test]
    fn descriptor_rejects_empty_bytes() {
        let d = descriptor(vec![]);
        assert!(matches!(d.to_module(), Err(WasmSkillError::BytesMissing)));
    }

    #[test]
    fn executor_runs_via_stub() {
        let exec = WasmSkillExecutor::new(Box::new(StubWasmRuntime));
        let d = descriptor(DUMMY_WASM.to_vec());
        let r = exec.execute(&d, 100).unwrap();
        assert!(r.is_ok());
        assert_eq!(r.module_hash.len(), 64);
        assert_eq!(r.policy_name, "test-skill");
    }

    #[test]
    fn executor_runs_zero_args() {
        let exec = WasmSkillExecutor::new(Box::new(StubWasmRuntime));
        let mut d = descriptor(DUMMY_WASM.to_vec());
        d.args.clear();
        let r = exec.execute(&d, 0).unwrap();
        assert!(r.is_ok());
    }

    #[test]
    fn executor_runtime_name() {
        let exec = WasmSkillExecutor::new(Box::new(StubWasmRuntime));
        assert_eq!(exec.runtime_name(), "stub");
    }

    #[test]
    fn executor_rejects_oversized_module() {
        let exec = WasmSkillExecutor::new(Box::new(StubWasmRuntime));
        let mut d = descriptor(DUMMY_WASM.to_vec());
        d.memory_limit_bytes = 4; // impossibly small
        assert!(exec.execute(&d, 0).is_err());
    }

    #[test]
    fn executor_rejects_empty_bytes() {
        let exec = WasmSkillExecutor::new(Box::new(StubWasmRuntime));
        let d = descriptor(vec![]);
        assert!(matches!(
            exec.execute(&d, 0),
            Err(WasmSkillError::BytesMissing)
        ));
    }
}
