// Copyright (c) 2026 ArtCraft Team and the PhotoCraft contributors.
// SPDX-License-Identifier: MIT OR Apache-2.0
//
// Modified 2026-10-18 for the Apeireth workspace: crate identifiers, tooling knob
// names and product references renamed to neutral names; generated metadata producer
// strings neutralised; scope adaptations recorded in
// docs/04-internal/license-port/gate1-license.md. Original license terms unchanged.
//! Fuzz the `FEid` / `FXid` smart-filter cache parser: any input must return `Ok` or `Err`, never
//! panic. Decoding every cached plane and the filter mask must also stay panic-free (a size the
//! data cannot hold is refused before allocation, for RLE and zlib alike), and a parsed block must
//! re-parse to the same value and re-serialise byte for byte.
#![no_main]
use libfuzzer_sys::fuzz_target;
use apeireth_image_psd::filter_effects::FilterEffects;
fuzz_target!(|data: &[u8]| {
    let Ok(fx) = FilterEffects::parse(data) else { return };
    for it in &fx.items {
        let _ = it.decoded_mask();
        if let (Ok(depth), Ok((w, h))) = (u16::try_from(it.depth), it.rect.size()) {
            for p in it.slots.iter().flatten() {
                let _ = p.decode(w, h, depth);
            }
        }
    }
    let out = fx.to_bytes();
    let back = FilterEffects::parse(&out).expect("a written FEid block must re-parse");
    assert_eq!(back, fx, "a written FEid block does not re-parse to the same value");
    assert_eq!(back.to_bytes(), out, "a second round trip is not byte-stable");
});
