// Copyright (c) 2026 ArtCraft Team and the PhotoCraft contributors.
// SPDX-License-Identifier: MIT OR Apache-2.0
//
// Modified 2026-10-18 for the Apeireth workspace: crate identifiers, tooling knob
// names and product references renamed to neutral names; generated metadata producer
// strings neutralised; scope adaptations recorded in
// docs/04-internal/license-port/gate1-license.md. Original license terms unchanged.
//! Fuzz the `.atn` actions parser: any input must return `Ok` or `Err`, never panic, and
//! anything that parses must survive a write and parse back equal.
#![no_main]

use libfuzzer_sys::fuzz_target;
use apeireth_image_psd::atn;

fuzz_target!(|data: &[u8]| {
    if let Ok(f) = atn::parse(data) {
        let bytes = atn::write(&f).expect("a parsed set is writable");
        let again = atn::parse(&bytes).expect("a written set parses");
        assert_eq!(again.actions, f.actions);
    }
});
