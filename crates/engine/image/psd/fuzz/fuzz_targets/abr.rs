// Copyright (c) 2026 ArtCraft Team and the PhotoCraft contributors.
// SPDX-License-Identifier: MIT OR Apache-2.0
//
// Modified 2026-10-18 for the Apeireth workspace: crate identifiers, tooling knob
// names and product references renamed to neutral names; generated metadata producer
// strings neutralised; scope adaptations recorded in
// docs/04-internal/license-port/gate1-license.md. Original license terms unchanged.
//! Fuzz the `.abr` brush parser: any input must return `Ok` or `Err`, never panic, and anything
//! that parses must keep its samples within the advertised bounds.
#![no_main]

use libfuzzer_sys::fuzz_target;
use apeireth_image_psd::abr;

fuzz_target!(|data: &[u8]| {
    if let Ok(f) = abr::parse(data) {
        for s in &f.samples {
            assert_eq!(s.data.len(), s.width as usize * s.height as usize * usize::from(s.depth / 8));
        }
    }
});
