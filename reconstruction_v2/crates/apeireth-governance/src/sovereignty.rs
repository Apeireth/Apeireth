use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OwnerTokenRole {
    Master,
    Admin,
    Operator,
    ReadOnly,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SovereignToken {
    pub role: OwnerTokenRole,
    pub token_hash: String,
    pub issued_at_epoch_sec: i64,
}

impl SovereignToken {
    pub fn new(role: OwnerTokenRole, secret: &str) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(secret.as_bytes());
        let token_hash = format!("{:x}", hasher.finalize());

        Self {
            role,
            token_hash,
            issued_at_epoch_sec: chrono::Utc::now().timestamp(),
        }
    }

    pub fn verify(&self, secret: &str) -> bool {
        let mut hasher = Sha256::new();
        hasher.update(secret.as_bytes());
        let computed = format!("{:x}", hasher.finalize());
        self.token_hash == computed
    }
}

pub struct SovereignControl {
    is_paused: AtomicBool,
}

impl Default for SovereignControl {
    fn default() -> Self {
        Self::new()
    }
}

impl SovereignControl {
    pub fn new() -> Self {
        Self {
            is_paused: AtomicBool::new(false),
        }
    }

    pub fn is_paused(&self) -> bool {
        self.is_paused.load(Ordering::SeqCst)
    }

    pub fn pause(&self, token: &SovereignToken, secret: &str) -> Result<(), &'static str> {
        if !token.verify(secret) {
            return Err("Invalid sovereign token secret");
        }
        if token.role == OwnerTokenRole::ReadOnly {
            return Err("ReadOnly token cannot pause system");
        }
        self.is_paused.store(true, Ordering::SeqCst);
        Ok(())
    }

    pub fn resume(&self, token: &SovereignToken, secret: &str) -> Result<(), &'static str> {
        if !token.verify(secret) {
            return Err("Invalid sovereign token secret");
        }
        if token.role == OwnerTokenRole::ReadOnly || token.role == OwnerTokenRole::Operator {
            return Err("Only Master or Admin can resume system");
        }
        self.is_paused.store(false, Ordering::SeqCst);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sovereign_control_pause_resume() {
        let control = SovereignControl::new();
        let master = SovereignToken::new(OwnerTokenRole::Master, "super_secret_master_key");
        let readonly = SovereignToken::new(OwnerTokenRole::ReadOnly, "readonly_key");

        assert!(!control.is_paused());

        // Readonly cannot pause
        assert!(control.pause(&readonly, "readonly_key").is_err());
        assert!(!control.is_paused());

        // Master pauses
        assert!(control.pause(&master, "super_secret_master_key").is_ok());
        assert!(control.is_paused());

        // Master resumes
        assert!(control.resume(&master, "super_secret_master_key").is_ok());
        assert!(!control.is_paused());
    }
}

