//! R177 http-client organ Kani proofs (W7)

#![allow(missing_docs)]

use crate::*;

#[test]
fn r177_hc_01_module_compiles() {
    let _ = std::mem::size_of::<u64>();
}

#[test]
fn r177_hc_02_keep_alive_default() {
    let c = KeepAliveConfig::default(); assert!(c.keep_alive);
}

#[test]
fn r177_hc_03_scheduling_default() {
    let c = KeepAliveConfig::default(); assert_eq!(c.scheduling, SchedulingPolicy::Lifo);
}

#[test]
fn r177_hc_04_max_sockets_positive() {
    let c = KeepAliveConfig::default(); assert!(c.max_sockets > 0);
}

#[test]
fn r177_hc_05_keep_alive_msecs() {
    let c = KeepAliveConfig::default(); assert!(c.keep_alive_msecs > 0);
}

#[cfg(kani)]
#[kani::proof]
fn r177_hc_kani_01_max_sockets_bounded() {
    let c = KeepAliveConfig::default(); assert!(c.max_sockets <= 100_000);
}

#[cfg(kani)]
#[kani::proof]
fn r177_hc_kani_02_keep_alive_msecs_bounded() {
    let c = KeepAliveConfig::default(); assert!(c.keep_alive_msecs <= 60_000);
}

