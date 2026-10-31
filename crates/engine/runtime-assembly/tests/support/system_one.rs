use apeireth_plugin::system_one::*;
use apeireth_runtime::canonical::system_one::{SystemOneGate, SystemOneLimits};
use async_trait::async_trait;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};

#[derive(Default)]
pub struct Audit {
    records: Mutex<Vec<SystemOneAuditRecord>>,
    pub fail: AtomicBool,
}
impl Audit {
    pub fn records(&self) -> Vec<SystemOneAuditRecord> {
        self.records.lock().unwrap().clone()
    }
}
#[async_trait]
impl SystemOneAudit for Audit {
    async fn record(&self, record: &SystemOneAuditRecord) -> Result<(), SystemOneError> {
        if self.fail.load(Ordering::Acquire) {
            return Err(SystemOneError::AuditFailed);
        }
        let mut records = self.records.lock().unwrap();
        if records.len() >= 1024 {
            return Err(SystemOneError::AuditFailed);
        }
        records.push(record.clone());
        Ok(())
    }
}
#[derive(Clone)]
pub enum Mode {
    Decision(SystemOneDecision),
    Unavailable,
    InvalidVersion,
    InvalidPolicy,
    InvalidDigest,
    InvalidReview,
    Pending,
    Wait(Arc<tokio::sync::Notify>, Arc<tokio::sync::Notify>),
}
pub struct Supervisor {
    pub mode: Mode,
    requests: Mutex<Vec<SystemOneRequest>>,
}
impl Supervisor {
    pub fn new(mode: Mode) -> Arc<Self> {
        Arc::new(Self {
            mode,
            requests: Mutex::new(vec![]),
        })
    }
    pub fn requests(&self) -> Vec<SystemOneRequest> {
        self.requests.lock().unwrap().clone()
    }
}
#[async_trait]
impl SystemOneSupervisor for Supervisor {
    async fn review(&self, request: &SystemOneRequest) -> Result<SystemOneVerdict, SystemOneError> {
        self.requests.lock().unwrap().push(request.clone());
        let mut verdict = SystemOneVerdict::for_request(request, SystemOneDecision::Allow);
        match &self.mode {
            Mode::Decision(decision) => verdict.decision = *decision,
            Mode::Unavailable => return Err(SystemOneError::Unavailable),
            Mode::InvalidVersion => verdict.version += 1,
            Mode::InvalidPolicy => verdict.policy_version += 1,
            Mode::InvalidDigest => verdict.action_arguments_digest = "unrelated".into(),
            Mode::InvalidReview => verdict.review_id = apeireth_core::kernel::TaskId::new(),
            Mode::Pending => std::future::pending::<()>().await,
            Mode::Wait(entered, release) => {
                entered.notify_one();
                release.notified().await;
            }
        }
        Ok(verdict)
    }
}
pub fn gate(mode: Mode) -> (Arc<SystemOneGate>, Arc<Audit>, Arc<Supervisor>) {
    let audit = Arc::new(Audit::default());
    let gate = Arc::new(SystemOneGate::new(audit.clone(), SystemOneLimits::default()).unwrap());
    let supervisor = Supervisor::new(mode);
    gate.bind_supervisor(Some(supervisor.clone()));
    (gate, audit, supervisor)
}
pub fn allow_gate() -> (Arc<SystemOneGate>, Arc<Audit>) {
    let (gate, audit, _) = gate(Mode::Decision(SystemOneDecision::Allow));
    (gate, audit)
}
