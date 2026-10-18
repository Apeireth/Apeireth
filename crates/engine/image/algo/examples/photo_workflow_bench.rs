// Copyright (c) 2026 ArtCraft Team and the PhotoCraft contributors.
// SPDX-License-Identifier: MIT OR Apache-2.0
//
// Modified 2026-10-18 for the Apeireth workspace: crate identifiers, tooling knob
// names and product references renamed to neutral names; generated metadata producer
// strings neutralised; scope adaptations recorded in
// docs/04-internal/license-port/gate1-license.md. Original license terms unchanged.
//! Deterministic 24 MP blur/resize benchmark. Run with --release; no private photos needed.
use apeireth_image_algo::{
    FilterParams, apply_in,
    resample::{Resample, resize_surface_in_canvas},
};
use apeireth_image_color::{ColorMode, PixelFormat, SampleType};
use apeireth_image_geom::Rect;
use apeireth_image_raster::Surface;
use std::time::Instant;
fn main() {
    let bounds = Rect::new(0, 0, 6000, 4000);
    let mut src = Surface::new(PixelFormat::new(ColorMode::Rgb, SampleType::U8, true));
    for y in (0..4000).step_by(128) {
        let r = Rect::new(0, y, 6000, (y + 128).min(4000));
        let mut data = Vec::new();
        for yy in r.y0..r.y1 {
            for x in r.x0..r.x1 {
                data.extend_from_slice(&[(x % 251) as f32 / 250.0, (yy % 241) as f32 / 240.0, ((x + yy) % 239) as f32 / 238.0, 1.0]);
            }
        }
        src.write_region(r, &data);
    }
    for op in ["blur3", "bicubic1791"] {
        for rep in 0..4 {
            let t = Instant::now();
            let out = if op == "blur3" {
                apply_in(&src, &FilterParams::GaussianBlur { radius: 3.0 }, bounds, bounds, None, bounds)
            } else {
                resize_surface_in_canvas(&src, 1791.0 / 6000.0, 1194.0 / 4000.0, Resample::Bicubic, bounds)
            };
            let elapsed = t.elapsed().as_secs_f64() * 1000.0;
            let sample = out.read_region(Rect::new(100, 100, 140, 130));
            let checksum = sample.iter().fold(0u64, |s, v| s.wrapping_mul(31).wrapping_add(v.to_bits() as u64));
            println!("{op} rep={rep} ms={elapsed:.3} checksum={checksum}");
            std::hint::black_box(out);
        }
    }
}
