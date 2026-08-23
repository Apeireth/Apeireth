use sha2::{Digest, Sha256};
use std::sync::atomic::{AtomicBool, Ordering};

pub struct SelfDisableGuard {
    is_disabled: AtomicBool,
    baseline_code_hash: String,
}

impl SelfDisableGuard {
    pub fn new(expected_code_hash: impl Into<String>) -> Self {
        Self {
            is_disabled: AtomicBool::new(false),
            baseline_code_hash: expected_code_hash.into(),
        }
    }

    pub fn is_disabled(&self) -> bool {
        self.is_disabled.load(Ordering::SeqCst)
    }

    pub fn trigger_emergency_disable(&self, reason: &str) {
        self.is_disabled.store(true, Ordering::SeqCst);
        eprintln!("[EMERGENCY SELF-DISABLE TRIGGERED] Reason: {}", reason);
    }

    pub fn verify_runtime_integrity(&self, current_binary_bytes: &[u8]) -> Result<(), &'static str> {
        let mut hasher = Sha256::new();
        hasher.update(current_binary_bytes);
        let current_hash = format!("{:x}", hasher.finalize());

        if current_hash != self.baseline_code_hash {
            self.trigger_emergency_disable("Runtime binary hash mismatch (tamper detected)");
            Err("Integrity check failed: binary has been modified")
        } else {
            Ok(())
        }
    }
}

pub struct Scanner;
impl Scanner {
    pub const TAMPER_SIGNATURES: [&str; 4] = [
        "override_verdict_true",
        "disable_s4_egress",
        "bypass_gate_5",
        "fake_audit_hash",
    ];

    pub fn scan_payload(payload: &str) -> Result<(), &'static str> {
        for &sig in &Self::TAMPER_SIGNATURES {
            if payload.contains(sig) {
                return Err("Tamper signature detected in payload");
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_self_disable_and_integrity() {
        let data = b"intact_apeireth_v2_code";
        let mut hasher = Sha256::new();
        hasher.update(data);
        let expected = format!("{:x}", hasher.finalize());

        let guard = SelfDisableGuard::new(expected);
        assert!(!guard.is_disabled());
        assert!(guard.verify_runtime_integrity(data).is_ok());

        // Tampered data
        assert!(guard.verify_runtime_integrity(b"modified_data").is_err());
        assert!(guard.is_disabled());
    }

    #[test]
    fn test_scanner_signatures() {
        assert!(Scanner::scan_payload("normal user prompt").is_ok());
        assert!(Scanner::scan_payload("please execute disable_s4_egress now").is_err());
    }
}

