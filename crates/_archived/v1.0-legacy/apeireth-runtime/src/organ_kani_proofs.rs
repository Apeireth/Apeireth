//! R177 runtime organ Kani proofs (W7)

#![allow(missing_docs)]

use crate::*;

#[test]
fn r177_rt_01_modules() {
    assert_eq!(MODULES_ORCHESTRATED, 7);
}

#[test]
fn r177_rt_02_tick_interval() {
    assert_eq!(DEFAULT_TICK_INTERVAL_SECS, 10);
}

#[test]
fn r177_rt_03_room_capacity() {
    assert_eq!(DEFAULT_ROOM_CAPACITY, 16);
}

#[test]
fn r177_rt_04_modules_positive() {
    assert!(MODULES_ORCHESTRATED > 0);
}

#[test]
fn r177_rt_05_tick_positive() {
    assert!(DEFAULT_TICK_INTERVAL_SECS > 0);
}

#[cfg(kani)]
#[kani::proof]
fn r177_rt_kani_01_capacity_positive() {
    assert!(DEFAULT_ROOM_CAPACITY >= 1);
}

#[cfg(kani)]
#[kani::proof]
fn r177_rt_kani_02_tick_positive() {
    assert!(DEFAULT_TICK_INTERVAL_SECS >= 1);
}
