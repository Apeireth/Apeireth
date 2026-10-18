// Copyright (c) 2026 ArtCraft Team and the PhotoCraft contributors.
// SPDX-License-Identifier: MIT OR Apache-2.0
//
// Modified 2026-10-18 for the Apeireth workspace: crate identifiers, tooling knob
// names and product references renamed to neutral names; generated metadata producer
// strings neutralised; scope adaptations recorded in
// docs/04-internal/license-port/gate1-license.md. Original license terms unchanged.
//! Fuzz the layered-TIFF tag parsers (37724 image source data in either byte order, and 34377
//! image resources), and the byte-order transcoder on anything that parses.
#![no_main]

use libfuzzer_sys::fuzz_target;
use apeireth_image_psd::tiff::{ByteOrder, ImageSourceData, resources_from_bytes};

fuzz_target!(|data: &[u8]| {
    let _ = resources_from_bytes(data);
    let Ok((d, _)) = ImageSourceData::from_bytes(data) else { return };
    // Whatever parsed must write in both orders, and each output must parse again.
    for order in [ByteOrder::Big, ByteOrder::Little] {
        if let Ok((out, _)) = d.to_bytes(order) {
            let (again, _) = ImageSourceData::from_bytes(&out).expect("re-parse of written data");
            let _ = again.to_bytes(order);
        }
    }
});
