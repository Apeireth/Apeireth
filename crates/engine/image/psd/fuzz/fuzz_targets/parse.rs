// Copyright (c) 2026 ArtCraft Team and the PhotoCraft contributors.
// SPDX-License-Identifier: MIT OR Apache-2.0
//
// Modified 2026-10-18 for the Apeireth workspace: crate identifiers, tooling knob
// names and product references renamed to neutral names; generated metadata producer
// strings neutralised; scope adaptations recorded in
// docs/04-internal/license-port/gate1-license.md. Original license terms unchanged.
//! Fuzz the full parser, and exercise writer/decoders on anything that parses.
#![no_main]

use libfuzzer_sys::fuzz_target;
use apeireth_image_psd::PsdFile;

fuzz_target!(|data: &[u8]| {
    let Ok(f) = PsdFile::from_bytes(data) else { return };
    // Anything written must re-parse and be byte stable from then on.
    // (Byte comparison rather than model equality, since NaN != NaN.)
    if let Ok(out) = f.to_bytes() {
        let g = PsdFile::from_bytes(&out).expect("re-parse of written file");
        assert_eq!(g.to_bytes().expect("re-write"), out);
    }
    // Keep decoding bounded: only small images.
    if u64::from(f.header.width) * u64::from(f.header.height) <= 1 << 20 {
        let _ = f.decode_merged();
        let _ = f.composite_rgba8();
        for l in f.iter_layers() {
            if l.record.rect.width() * l.record.rect.height() <= 1 << 20 {
                let _ = l.rgba8();
                let _ = l.user_mask();
            }
        }
    }
    let _ = f.layer_tree();
});
