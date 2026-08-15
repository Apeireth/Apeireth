//! R177 team-lead organ Kani proofs (W8)

#![allow(missing_docs)]

use crate::*;

#[test]
fn r177_tl_01_tool_whitelist() {
    assert!(TOOL_WHITELIST.len() >= 1);
}

#[test]
fn r177_tl_02_orchestrator_count() {
    assert_eq!(ORCHESTRATOR_TOOL_COUNT, 14);
}

#[test]
fn r177_tl_03_scheduling_count() {
    assert_eq!(SCHEDULING_TOOL_COUNT, 8);
}

#[test]
fn r177_tl_04_worktree_count() {
    assert_eq!(WORKTREE_TOOL_COUNT, 3);
}

#[test]
fn r177_tl_05_awareness_count() {
    assert_eq!(AWARENESS_TOOL_COUNT, 3);
}

#[cfg(kani)]
#[kani::proof]
fn r177_tl_kani_01_orchestrator_invariant() {
    assert_eq!(ORCHESTRATOR_TOOL_COUNT, 14);
}

#[cfg(kani)]
#[kani::proof]
fn r177_tl_kani_02_scheduling_invariant() {
    assert_eq!(SCHEDULING_TOOL_COUNT, 8);
}

