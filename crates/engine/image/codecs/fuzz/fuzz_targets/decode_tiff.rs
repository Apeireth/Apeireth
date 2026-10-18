// Copyright (c) 2026 ArtCraft Team and the PhotoCraft contributors.
// SPDX-License-Identifier: MIT OR Apache-2.0
//
// Modified 2026-10-18 for the Apeireth workspace: crate identifiers, tooling knob
// names and product references renamed to neutral names; generated metadata producer
// strings neutralised; scope adaptations recorded in
// docs/04-internal/license-port/gate1-license.md. Original license terms unchanged.
#![no_main]

use libfuzzer_sys::fuzz_target;
use apeireth_image_codecs::{decode_as_with, decode_tiff_page, exif_orientation, tiff_info, tiff_orientation, DecodeOptions, Format, Limits};

fuzz_target!(|data: &[u8]| {
    let opts = DecodeOptions {
        limits: Limits { max_width: 4096, max_height: 4096, max_pixels: 1 << 22, max_alloc: 256 << 20 },
        ..Default::default()
    };
    let _ = decode_as_with(Format::Tiff, data, &opts);
    let _ = exif_orientation(data);
    let _ = tiff_orientation(data);
    // Every directory of the chain and its SubIFDs (BigTIFF too), not only the default page.
    if let Ok(info) = tiff_info(data) {
        for page in 0..info.pages.len().min(16) {
            let _ = decode_tiff_page(data, page, &opts);
        }
    }
});
