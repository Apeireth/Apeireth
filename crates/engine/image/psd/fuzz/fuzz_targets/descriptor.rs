// Copyright (c) 2026 ArtCraft Team and the PhotoCraft contributors.
// SPDX-License-Identifier: MIT OR Apache-2.0
//
// Modified 2026-10-18 for the Apeireth workspace: crate identifiers, tooling knob
// names and product references renamed to neutral names; generated metadata producer
// strings neutralised; scope adaptations recorded in
// docs/04-internal/license-port/gate1-license.md. Original license terms unchanged.
//! Fuzz the ActionDescriptor parser; anything that parses must be byte
//! stable after one normalization pass (e.g. `bool` bytes other than 0/1).
#![no_main]

use libfuzzer_sys::fuzz_target;
use apeireth_image_psd::descriptor::Descriptor;

fuzz_target!(|data: &[u8]| {
    if let Ok(d) = Descriptor::from_bytes(data) {
        let b = d.to_bytes();
        assert_eq!(Descriptor::from_bytes(&b).expect("re-parse").to_bytes(), b);
    }
});
