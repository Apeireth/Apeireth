//! R177 workflow organ Kani proofs (W9)

#![allow(missing_docs)]

use crate::*;
use parking_lot::Mutex;
use std::sync::Arc;

#[test]
fn r177_wf_01_event_kind_workflow_started() {
    let e = EventKind::WorkflowStarted;
    let _: String = format!("{:?}", e);
}

#[test]
fn r177_wf_02_workflow_runner_new() {
    let r = WorkflowRunner::new();
    let n = r.workflows.len();
    assert_eq!(n, 0);
}

#[test]
fn r177_wf_03_workflow_context_new() {
    let history = Arc::new(Mutex::new(Vec::new()));
    let activities = Arc::new(std::collections::HashMap::new());
    let c = WorkflowContext::new("wf-1", history, activities);
    assert_eq!(c.workflow_id, "wf-1");
}

#[test]
fn r177_wf_04_event_struct() {
    let e = Event {
        event_id: 0,
        kind: EventKind::WorkflowStarted,
        activity_id: "".into(),
        input: None,
        output: None,
        timestamp_ms: 0,
        error: None,
    };
    assert_eq!(e.event_id, 0);
}

#[test]
fn r177_wf_05_worker() {
    let runner = Arc::new(WorkflowRunner::new());
    let w = WorkflowWorker::new(runner, "wf-1");
    assert_eq!(w.workflow_id(), "wf-1");
}

#[cfg(kani)]
#[kani::proof]
fn r177_wf_kani_01_event_kind_invariant() {
    let e = EventKind::WorkflowStarted;
    assert!(!format!("{:?}", e).is_empty());
}

#[cfg(kani)]
#[kani::proof]
fn r177_wf_kani_02_runner_invariant() {
    let r = WorkflowRunner::new();
    assert_eq!(r.workflows.len(), 0);
}
