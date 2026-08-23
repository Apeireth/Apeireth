use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditRecord {
    pub index: usize,
    pub timestamp_epoch_sec: i64,
    pub action: String,
    pub actor: String,
    pub previous_hash: String,
    pub current_hash: String,
}

#[derive(Default)]
pub struct AuditHashChain {
    records: Vec<AuditRecord>,
}

impl AuditHashChain {
    pub fn new() -> Self {
        Self {
            records: Vec::new(),
        }
    }

    pub fn append(&mut self, action: impl Into<String>, actor: impl Into<String>) -> &AuditRecord {
        let index = self.records.len();
        let prev_hash = if index == 0 {
            "GENESIS_HASH_APEIRETH_V2_00000000000000000000000000000000".to_string()
        } else {
            self.records[index - 1].current_hash.clone()
        };

        let action = action.into();
        let actor = actor.into();
        let timestamp = chrono::Utc::now().timestamp();

        let mut hasher = Sha256::new();
        hasher.update(format!("{}:{}:{}:{}:{}", index, timestamp, action, actor, prev_hash).as_bytes());
        let current_hash = format!("{:x}", hasher.finalize());

        let record = AuditRecord {
            index,
            timestamp_epoch_sec: timestamp,
            action,
            actor,
            previous_hash: prev_hash,
            current_hash,
        };

        self.records.push(record);
        self.records.last().unwrap()
    }

    pub fn verify_chain(&self) -> Result<(), (usize, &'static str)> {
        for (i, record) in self.records.iter().enumerate() {
            if i == 0 {
                if record.previous_hash != "GENESIS_HASH_APEIRETH_V2_00000000000000000000000000000000" {
                    return Err((0, "Genesis hash mismatch"));
                }
            } else {
                if record.previous_hash != self.records[i - 1].current_hash {
                    return Err((i, "Previous hash pointer broken"));
                }
            }

            let mut hasher = Sha256::new();
            hasher.update(format!("{}:{}:{}:{}:{}", record.index, record.timestamp_epoch_sec, record.action, record.actor, record.previous_hash).as_bytes());
            let expected_hash = format!("{:x}", hasher.finalize());

            if record.current_hash != expected_hash {
                return Err((i, "Record current hash corrupted (tamper detected)"));
            }
        }
        Ok(())
    }

    pub fn records(&self) -> &[AuditRecord] {
        &self.records
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audit_hash_chain_integrity() {
        let mut chain = AuditHashChain::new();
        chain.append("boot_system", "system");
        chain.append("user_login", "alice");
        chain.append("execute_tool_shell", "alice");

        assert_eq!(chain.records().len(), 3);
        assert!(chain.verify_chain().is_ok());

        // Tamper test
        let mut corrupted_chain = chain;
        corrupted_chain.records[1].action = "tampered_action".into();
        assert!(corrupted_chain.verify_chain().is_err());
    }
}

