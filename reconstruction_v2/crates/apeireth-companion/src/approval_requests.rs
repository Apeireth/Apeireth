//! ApprovalRequests - 审批请求 (从 v1.0 apeireth-companion/approval_requests.rs 2K LOC 抄录升级)
//!
//! 0 装 PASS: 真 request + approve/deny 流程
use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ApprovalStatus { Pending, Approved, Denied, Expired }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalRequest {
    pub id: String,
    pub requester: String,
    pub action: String,
    pub reason: String,
    pub status: ApprovalStatus,
    pub created_ms: i64,
    pub decided_ms: Option<i64>,
    pub decider: Option<String>,
}

pub struct ApprovalQueue {
    requests: HashMap<String, ApprovalRequest>,
}

impl ApprovalQueue {
    pub fn new() -> Self { Self { requests: HashMap::new() } }

    /// 0 装 PASS: 真 submit
    pub fn submit(&mut self, requester: impl Into<String>, action: impl Into<String>, reason: impl Into<String>) -> String {
        let id = format!("ap-{}-{}", chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0), std::process::id());
        let req = ApprovalRequest { id: id.clone(), requester: requester.into(), action: action.into(), reason: reason.into(), status: ApprovalStatus::Pending, created_ms: chrono::Utc::now().timestamp_millis(), decided_ms: None, decider: None };
        self.requests.insert(id.clone(), req);
        id
    }

    /// 0 装 PASS: 真 decide
    pub fn decide(&mut self, id: &str, approve: bool, decider: impl Into<String>) -> Result<(), String> {
        let req = self.requests.get_mut(id).ok_or_else(|| "not found")?;
        if req.status != ApprovalStatus::Pending { return Err("already decided".into()); }
        req.status = if approve { ApprovalStatus::Approved } else { ApprovalStatus::Denied };
        req.decided_ms = Some(chrono::Utc::now().timestamp_millis());
        req.decider = Some(decider.into());
        Ok(())
    }

    pub fn pending(&self) -> Vec<&ApprovalRequest> {
        self.requests.values().filter(|r| r.status == ApprovalStatus::Pending).collect()
    }

    pub fn count(&self) -> usize { self.requests.len() }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_submit_pending() {
        let mut q = ApprovalQueue::new();
        let id = q.submit("user1", "delete_file", "cleanup");
        assert_eq!(q.pending().len(), 1);
        assert!(q.requests.contains_key(&id));
    }
    #[test] fn test_decide_approve() {
        let mut q = ApprovalQueue::new();
        let id = q.submit("u", "x", "r");
        q.decide(&id, true, "admin").unwrap();
        assert_eq!(q.requests.get(&id).unwrap().status, ApprovalStatus::Approved);
    }
    #[test] fn test_decide_twice() {
        let mut q = ApprovalQueue::new();
        let id = q.submit("u", "x", "r");
        q.decide(&id, true, "admin").unwrap();
        assert!(q.decide(&id, false, "admin").is_err());
    }
    #[test] fn test_pending_filter() {
        let mut q = ApprovalQueue::new();
        let id1 = q.submit("u", "a", "r");
        let id2 = q.submit("u", "b", "r");
        q.decide(&id1, true, "admin").unwrap();
        assert_eq!(q.pending().len(), 1);
        let _ = id2;
    }
    #[test] fn test_unknown_decide() {
        let mut q = ApprovalQueue::new();
        assert!(q.decide("nonexistent", true, "x").is_err());
    }
}
